//! Bounded audio tap shared with playback; recognition never blocks audio output.
use crate::{auto_subtitle, localization::tr};
use eframe::egui;
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const WORKER: &str = include_str!("live_subtitle.py");
const HELPERS: &str = include_str!("auto_subtitle.py");

pub(crate) fn worker(engine: &str, language: &str, action: &str) -> Result<Command, String> {
    let mut command = auto_subtitle::python()?;
    command
        .args(["-I", "-u", "-c", WORKER, HELPERS])
        .arg(auto_subtitle::component()?)
        .args([engine, language, action]);
    Ok(command)
}

pub(crate) fn model_installed(engine: &str) -> bool {
    auto_subtitle::component().is_ok_and(|root| {
        if engine == "sensevoice" {
            ["model.int8.onnx", "tokens.txt", "silero_vad.onnx"]
                .iter()
                .all(|name| root.join("models").join(name).is_file())
        } else {
            [
                "encoder-epoch-99-avg-1.int8.onnx",
                "decoder-epoch-99-avg-1.int8.onnx",
                "joiner-epoch-99-avg-1.int8.onnx",
                "tokens.txt",
            ]
            .iter()
            .all(|name| root.join("zipformer-zh-en/models").join(name).is_file())
        }
    })
}

#[derive(Clone)]
pub(crate) struct Tap {
    sender: mpsc::SyncSender<Vec<u8>>,
    ready: Arc<AtomicBool>,
    overloaded: Arc<AtomicBool>,
    replay: Arc<Vec<crate::live_subtitle_cache::Cue>>,
}
impl Tap {
    pub(crate) fn send(&self, timestamp: f64, samples: &[f32]) {
        if !samples.is_empty() && !self.ready.load(Ordering::Relaxed) {
            return;
        }
        if !samples.is_empty()
            && self
                .replay
                .iter()
                .any(|c| c.contains(timestamp, timestamp + samples.len() as f64 / 16000.0))
        {
            return;
        }
        let mut packet = Vec::with_capacity(12 + samples.len() * 4);
        packet.extend(timestamp.to_le_bytes());
        packet.extend((samples.len() as u32).to_le_bytes());
        for sample in samples {
            packet.extend(sample.to_le_bytes());
        }
        if self.sender.try_send(packet).is_err() {
            self.ready.store(false, Ordering::Relaxed);
            self.overloaded.store(true, Ordering::Relaxed);
        }
    }
}

/// 48 kHz stereo -> 16 kHz mono, with a windowed-sinc low-pass before decimation.
pub(crate) struct Resampler {
    history: VecDeque<f32>,
    taps: Vec<f32>,
    phase: usize,
}
impl Default for Resampler {
    fn default() -> Self {
        let mut taps: Vec<f32> = (0..49)
            .map(|i| {
                let x = i as f32 - 24.0;
                let sinc = if x == 0.0 {
                    0.3
                } else {
                    (std::f32::consts::PI * 0.3 * x).sin() / (std::f32::consts::PI * x)
                };
                sinc * (0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / 48.0).cos())
            })
            .collect();
        let sum: f32 = taps.iter().sum();
        for tap in &mut taps {
            *tap /= sum;
        }
        Self {
            history: VecDeque::from(vec![0.0; 49]),
            taps,
            phase: 0,
        }
    }
}
impl Resampler {
    pub(crate) fn convert(&mut self, samples: &[i16]) -> Vec<f32> {
        let mut output = Vec::with_capacity(samples.len() / 6);
        for frame in samples.chunks_exact(2) {
            self.history.pop_front();
            self.history
                .push_back((frame[0] as f32 + frame[1] as f32) / 65536.0);
            self.phase += 1;
            if self.phase == 3 {
                self.phase = 0;
                output.push(
                    self.history
                        .iter()
                        .zip(&self.taps)
                        .map(|(x, t)| x * t)
                        .sum(),
                );
            }
        }
        output
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub(crate) struct Event {
    #[serde(default)]
    pub(crate) ready: bool,
    #[serde(default)]
    pub(crate) done: bool,
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) start: f64,
    #[serde(default)]
    pub(crate) end: f64,
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default, rename = "final")]
    pub(crate) final_result: bool,
}
struct Session {
    child: Arc<Mutex<Child>>,
    tap: Tap,
    events: mpsc::Receiver<Event>,
    paths: crate::live_subtitle_cache::Paths,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.tap.ready.store(false, Ordering::Relaxed);
        let child = self.child.clone();
        std::thread::spawn(move || {
            let mut child = child.lock().unwrap();
            let _ = child.kill();
            let _ = child.wait();
        });
    }
}
impl Session {
    fn start(
        engine: &str,
        language: &str,
        source: &Path,
        track: u64,
        identity: Option<&str>,
        replay: Vec<crate::live_subtitle_cache::Cue>,
        ctx: &egui::Context,
    ) -> Result<Self, String> {
        if !auto_subtitle::installed() {
            return Err(tr("请先安装实时字幕运行库。").into());
        }
        let mut cache = crate::live_subtitle_cache::Cache::create(
            source,
            track,
            engine,
            if engine == "zipformer" {
                "zh-en"
            } else {
                language
            },
            identity,
        )?;
        let paths = cache.paths.clone();
        let replay = Arc::new(replay);
        cache.reused = replay.clone();
        let mut child = worker(engine, language, "recognize")?
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(30);
        let tap = Tap {
            sender: tx,
            ready: Arc::new(AtomicBool::new(false)),
            overloaded: Arc::new(AtomicBool::new(false)),
            replay,
        };
        let mut input = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            for packet in rx {
                if input
                    .write_all(&packet)
                    .and_then(|_| input.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        // Drain diagnostics continuously, retaining only a bounded tail for failures.
        let mut stderr = child.stderr.take().unwrap();
        let diagnostics = Arc::new(Mutex::new(VecDeque::new()));
        let log = diagnostics.clone();
        std::thread::spawn(move || {
            let mut bytes = [0u8; 1024];
            while let Ok(n) = stderr.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                let mut log = log.lock().unwrap();
                log.extend(&bytes[..n]);
                while log.len() > 4096 {
                    log.pop_front();
                }
            }
        });
        let stdout = child.stdout.take().unwrap();
        let (sender, events) = mpsc::sync_channel(32);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut completed = false;
            let mut terminal = false;
            let mut deliver = true;
            loop {
                let mut line = String::new();
                match reader.by_ref().take(65537).read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() <= 65536 => {
                        if let Ok(event) = serde_json::from_str::<Event>(&line) {
                            completed |= event.done;
                            terminal |= event.error.is_some();
                            if let Err(error) = cache.record(&event) {
                                let _ = sender.send(Event {
                                    error: Some(error),
                                    ..Default::default()
                                });
                                ctx.request_repaint();
                                return;
                            }
                            if deliver && sender.send(event).is_err() {
                                deliver = false;
                            }
                            ctx.request_repaint();
                        }
                    }
                    _ => break,
                }
            }
            if let Err(error) = cache.finish(completed) {
                let _ = sender.send(Event {
                    error: Some(error),
                    ..Default::default()
                });
                ctx.request_repaint();
                return;
            }
            if completed || terminal {
                return;
            }
            let log: Vec<u8> = diagnostics.lock().unwrap().iter().copied().collect();
            let detail = String::from_utf8_lossy(&log).trim().to_owned();
            let _ = sender.send(Event {
                error: Some(if detail.is_empty() {
                    tr("实时字幕工作进程已结束。").into()
                } else {
                    detail
                }),
                ..Default::default()
            });
            ctx.request_repaint();
        });
        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            tap,
            events,
            paths,
        })
    }
}

#[derive(Default)]
pub(crate) struct Live {
    enabled: bool,
    engine: usize,
    language: usize,
    session: Option<Session>,
    error: Option<String>,
    caption: String,
    updated: Option<Instant>,
    last_cache: Option<crate::live_subtitle_cache::Paths>,
    cache_key: Option<String>,
    replay: Vec<crate::live_subtitle_cache::Cue>,
    loading: Option<mpsc::Receiver<crate::live_subtitle_cache::Replay>>,
    replay_caption: Option<String>,
    reusable: bool,
    identity_source: Option<PathBuf>,
    identity_checked: bool,
    identity_job: Option<mpsc::Receiver<Option<String>>>,
    verified_identity: Option<String>,
}
impl Live {
    pub(crate) fn reset(&mut self) {
        self.session = None;
        self.caption.clear();
        self.updated = None;
        self.replay_caption = None;
    }
    pub(crate) fn attach(
        &mut self,
        slot: &Mutex<Option<Tap>>,
        ctx: &egui::Context,
        finished: bool,
        source: &Path,
        input: Option<&crate::network_video::Input>,
        duration: f64,
        track: u64,
        identity: Option<&str>,
        position: f64,
    ) {
        if !self.enabled {
            *slot.lock().unwrap() = None;
            return;
        }
        if self.identity_source.as_deref() != Some(source) {
            self.identity_source = Some(source.to_owned());
            self.identity_checked = false;
            self.identity_job = None;
            self.verified_identity = None;
        }
        if identity.is_none() && !self.identity_checked {
            if self.identity_job.is_none() && input.is_some_and(|i| i.hls) && duration > 0.0 {
                let (sender, receiver) = mpsc::channel();
                self.identity_job = Some(receiver);
                let input = input.cloned();
                let source = source.to_owned();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::live_subtitle_cache::identity(
                        &source,
                        input.as_ref(),
                        duration,
                    ));
                    ctx.request_repaint();
                });
            }
            if let Some(receiver) = &self.identity_job {
                match receiver.try_recv() {
                    Ok(value) => {
                        self.verified_identity = value;
                        self.identity_job = None;
                        self.identity_checked = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        ctx.request_repaint_after(Duration::from_millis(100));
                        return;
                    }
                    Err(_) => {
                        self.identity_job = None;
                        self.identity_checked = true;
                    }
                }
            } else {
                self.identity_checked = true;
            }
        }
        let identity = identity
            .map(str::to_owned)
            .or_else(|| self.verified_identity.clone());
        let identity = identity.as_deref();
        let engine = if self.engine == 0 {
            "sensevoice"
        } else {
            "zipformer"
        };
        let language = if self.engine == 0 {
            auto_subtitle::LANGUAGES[self.language].0
        } else {
            "zh-en"
        };
        let key = crate::live_subtitle_cache::key(source, track, engine, language, identity);
        if self.cache_key.as_ref() != Some(&key) {
            self.reset();
            self.cache_key = Some(key.clone());
            self.replay.clear();
            self.last_cache = None;
            self.loading = None;
            if identity.is_some() {
                let (sender, receiver) = mpsc::channel();
                self.loading = Some(receiver);
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::live_subtitle_cache::load(&key));
                    ctx.request_repaint();
                });
            }
        }
        if let Some(receiver) = &self.loading {
            match receiver.try_recv() {
                Ok(replay) => {
                    self.replay = replay.cues;
                    self.last_cache = replay.paths;
                    self.loading = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                    return;
                }
                Err(_) => self.loading = None,
            }
        }
        self.reusable = identity.is_some();
        self.replay_caption = self
            .replay
            .iter()
            .find(|c| self.reusable && c.start <= position && position < c.end)
            .map(|c| c.text.clone());
        if self.session.is_none() && self.replay_caption.is_some() {
            return;
        }
        if self.session.is_none() {
            if finished {
                return;
            }
            let engine = if self.engine == 0 {
                "sensevoice"
            } else {
                "zipformer"
            };
            match Session::start(
                engine,
                auto_subtitle::LANGUAGES[self.language].0,
                source,
                track,
                identity,
                if identity.is_some() {
                    self.replay.clone()
                } else {
                    Vec::new()
                },
                ctx,
            ) {
                Ok(session) => {
                    self.last_cache = Some(session.paths.clone());
                    *slot.lock().unwrap() = Some(session.tap.clone());
                    self.session = Some(session);
                }
                Err(error) => {
                    self.error = Some(error);
                    self.enabled = false;
                }
            }
        }
        self.poll();
    }
    pub(crate) fn poll(&mut self) {
        let mut failure = None;
        if let Some(session) = &self.session {
            if session.tap.overloaded.load(Ordering::Relaxed) {
                failure = Some(tr("实时识别跟不上播放，请切换引擎或重新开启。").to_owned());
            }
            for event in session.events.try_iter().take(32) {
                if event.ready {
                    session.tap.ready.store(true, Ordering::Relaxed);
                }
                if let Some(error) = event.error {
                    failure = Some(error);
                }
                if !event.text.is_empty()
                    && event.start.is_finite()
                    && event.end.is_finite()
                    && event.end >= event.start
                {
                    // Partial Zipformer results replace the current utterance, never append duplicates.
                    self.caption = event.text;
                    self.updated = Some(Instant::now());
                    if self.reusable
                        && event.final_result
                        && self.replay.len() < 10000
                        && !self
                            .replay
                            .iter()
                            .any(|c| c.overlaps(event.start, event.end))
                    {
                        self.replay.push(crate::live_subtitle_cache::Cue {
                            start: event.start,
                            end: event.end,
                            text: self.caption.clone(),
                        });
                    }
                }
            }
        }
        if let Some(error) = failure {
            self.error = Some(error);
            self.enabled = false;
            self.reset();
        }
    }
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, has_audio: bool) {
        let before = (self.enabled, self.engine, self.language);
        ui.menu_button(tr("实时字幕"), |ui| {
            ui.add_enabled(
                has_audio || self.enabled,
                egui::Checkbox::new(&mut self.enabled, tr("启用实时字幕")),
            );
            egui::ComboBox::from_id_salt("live_subtitle_engine")
                .selected_text(if self.engine == 0 {
                    "SenseVoice"
                } else {
                    "Zipformer"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.engine, 0, "SenseVoice");
                    ui.selectable_value(&mut self.engine, 1, "Zipformer · zh / en");
                });
            if self.engine == 1 && self.language > 1 {
                self.language = 0;
            }
            if self.engine == 0 {
                egui::ComboBox::from_id_salt("live_subtitle_language")
                    .selected_text(tr(auto_subtitle::LANGUAGES[self.language].1))
                    .show_ui(ui, |ui| {
                        for (index, (_, label)) in auto_subtitle::LANGUAGES
                            .iter()
                            .enumerate()
                            .take(if self.engine == 0 { 5 } else { 2 })
                        {
                            ui.selectable_value(&mut self.language, index, tr(label));
                        }
                    });
            } else {
                ui.label(tr("中文 / 英语（双语识别）"));
            }
            ui.small(tr("SenseVoice 分句显示；Zipformer 连续更新中英识别结果。"));
            ui.small(tr("首次使用会下载模型；模型就绪后从当前播放位置开始识别。"));
            ui.small(tr("实时字幕自动缓存；点播回放优先复用已确认字幕。"));
            if let Some(paths) = self.last_cache.clone() {
                if ui
                    .button(tr("定位实时字幕缓存"))
                    .on_hover_text(paths.json.display().to_string())
                    .clicked()
                {
                    if let Err(error) = crate::asset_viewer::reveal_file(&paths.json) {
                        self.error = Some(error.to_string());
                    }
                }
                if ui.button(tr("导出实时字幕 SRT")).clicked() {
                    if let Some(destination) = rfd::FileDialog::new()
                        .add_filter("SRT", &["srt"])
                        .set_file_name(paths.srt.file_name().unwrap_or_default().to_string_lossy())
                        .save_file()
                    {
                        let result = crate::live_subtitle_cache::snapshot_srt(&paths.srt)
                            .map_err(|e| e.to_string())
                            .and_then(|text| {
                                if text.trim().is_empty() {
                                    Err(tr("尚无已确认的实时字幕。").into())
                                } else {
                                    auto_subtitle::save_new(destination, &text).map(|_| ())
                                }
                            });
                        if let Err(error) = result {
                            self.error = Some(error);
                        }
                    }
                }
            }
            if !auto_subtitle::installed() {
                if ui.button(tr("前往第三方支持")).clicked() {
                    crate::third_party::request_open(ui.ctx());
                }
            }
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
        if before != (self.enabled, self.engine, self.language) {
            self.reset();
            self.error = None;
        }
        if self.enabled && (self.identity_job.is_some() || self.loading.is_some()) {
            ui.spinner();
            ui.small(tr("正在检查字幕缓存…"));
        }
        if self.enabled
            && self
                .session
                .as_ref()
                .is_some_and(|s| !s.tap.ready.load(Ordering::Relaxed))
        {
            ui.spinner();
            ui.small(tr("正在准备语言模型（首次使用时下载）…"));
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
    pub(crate) fn paint(&self, ui: &egui::Ui, canvas: egui::Rect) {
        let caption = self.replay_caption.as_ref().unwrap_or(&self.caption);
        if caption.is_empty()
            || (self.replay_caption.is_none()
                && !self
                    .updated
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(8)))
        {
            return;
        }
        let painter = ui
            .painter()
            .with_clip_rect(canvas.intersect(ui.clip_rect()));
        let galley = painter.layout(
            caption.clone(),
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
            (canvas.width() - 32.0).max(16.0),
        );
        let position = egui::pos2(
            canvas.center().x - galley.size().x / 2.0,
            canvas.bottom() - galley.size().y - 28.0,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                position - egui::vec2(8.0, 4.0),
                galley.size() + egui::vec2(16.0, 8.0),
            ),
            4.0,
            egui::Color32::from_black_alpha(200),
        );
        painter.galley(position, galley, egui::Color32::WHITE);
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn covered_audio_skips_recognition_but_uncached_audio_and_eof_pass() {
        let (sender, receiver) = mpsc::sync_channel(4);
        let tap = Tap {
            sender,
            ready: Arc::new(AtomicBool::new(true)),
            overloaded: Arc::new(AtomicBool::new(false)),
            replay: Arc::new(vec![crate::live_subtitle_cache::Cue {
                start: 5.0,
                end: 7.0,
                text: "cached".into(),
            }]),
        };
        tap.send(5.0, &[0.0; 1600]);
        assert!(receiver.try_recv().is_err());
        tap.send(7.0, &[0.0; 1600]);
        assert!(receiver.try_recv().is_ok());
        tap.send(8.0, &[]);
        assert_eq!(receiver.try_recv().unwrap().len(), 12);
    }
    #[test]
    fn resampling_keeps_chunk_boundaries_and_rejects_aliasing() {
        let audio: Vec<i16> = (0..4800)
            .flat_map(|i| {
                let x = ((i as f32 * 2.0 * std::f32::consts::PI * 12000.0 / 48000.0).sin()
                    * 20000.0) as i16;
                [x, x]
            })
            .collect();
        let all = Resampler::default().convert(&audio);
        let mut resampler = Resampler::default();
        let chunked: Vec<_> = audio
            .chunks(200)
            .flat_map(|chunk| resampler.convert(chunk))
            .collect();
        assert_eq!(all, chunked);
        assert_eq!(all.len(), 1600);
        assert!(all[50..].iter().all(|v| v.abs() < 0.02));
    }
    #[test]
    fn overload_never_blocks_playback() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let tap = Tap {
            sender,
            ready: Arc::new(AtomicBool::new(true)),
            overloaded: Arc::new(AtomicBool::new(false)),
            replay: Arc::new(Vec::new()),
        };
        tap.send(2.0, &[0.0]);
        tap.send(2.1, &[0.0]);
        assert!(tap.overloaded.load(Ordering::Relaxed));
        assert!(!tap.ready.load(Ordering::Relaxed));
    }
}
