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
        .env("UNITY_ASR_RUNTIME",auto_subtitle::runtime()?)
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
    pub(crate) sender: mpsc::SyncSender<Vec<u8>>,
    pub(crate) ready: Arc<AtomicBool>,
    overloaded: Arc<AtomicBool>,
    pub(crate) replay: Arc<Vec<crate::live_subtitle_cache::Cue>>,
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
    pub(crate) language:Option<String>,
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
    ahead: Option<crate::subtitle_lookahead::Ahead>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.ahead = None;
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
        ahead: bool,
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
            ahead,
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
            ahead: None,
        })
    }
}

#[derive(Default)]
pub(crate) struct Live {
    enabled: bool,
    engine: usize,
    language: usize,
    manual_language:bool,
    finalized_language:Option<String>,
    replay_language:Option<String>,
    session: Option<Session>,
    error: Option<String>,
    caption: String,
    finalized:String,
    translation:crate::live_translation::Translation,
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
    disable_lookahead: bool,
    scheduled: bool,
    media_position:f64,
}
impl Live {
    fn recognition_language(&self)->&'static str{if self.engine==0&&!self.manual_language{"auto"}else{auto_subtitle::LANGUAGES[self.language].0}}
    pub(crate) fn reset(&mut self) {
        self.session = None;
        self.translation.reset();
        self.finalized.clear();self.finalized_language=None;self.replay_language=None;
        self.caption.clear();
        self.updated = None;
        self.replay_caption = None;
        self.scheduled = false;
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
        ffmpeg: &Path,
    ) {
        self.media_position=position;
        if !self.enabled {
            *slot.lock().unwrap() = None;
            return;
        }
        self.poll();
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
            self.recognition_language()
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
        let lookahead =
            self.engine == 0 && !self.disable_lookahead && self.reusable && duration > position;
        self.replay_language=self.replay.iter().find(|cue|self.reusable&&cue.start<=position&&position<cue.end).and_then(|cue|cue.language.clone());
        self.replay_caption = self
            .replay
            .iter()
            .find(|c| self.reusable && c.start <= position && position < c.end)
            .map(|c| c.text.clone());
        if self.session.is_none() && self.replay_caption.is_some() && !lookahead {
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
                self.recognition_language(),
                source,
                track,
                identity,
                if identity.is_some() {
                    self.replay.clone()
                } else {
                    Vec::new()
                },
                lookahead,
                ctx,
            ) {
                Ok(mut session) => {
                    self.last_cache = Some(session.paths.clone());
                    self.scheduled = lookahead;
                    if lookahead {
                        *slot.lock().unwrap() = None;
                        session.ahead = Some(crate::subtitle_lookahead::Ahead::start(
                            ffmpeg,
                            source,
                            input,
                            track,
                            position,
                            duration,
                            session.tap.clone(),
                        ));
                    } else {
                        *slot.lock().unwrap() = Some(session.tap.clone());
                    }
                    self.session = Some(session);
                }
                Err(error) => {
                    self.error = Some(error);
                    self.enabled = false;
                }
            }
        }
        if let Some(ahead) = self.session.as_ref().and_then(|s| s.ahead.as_ref()) {
            ahead.position(position);
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.poll();
    }
    pub(crate) fn poll(&mut self) {
        let mut failure = None;
        if let Some(session) = &self.session {
            if let Some(ahead) = &session.ahead {
                if let Ok(error) = ahead.errors.try_recv() {
                    failure = Some(error);
                }
            }
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
                    if event.final_result {self.finalized=event.text.clone();self.finalized_language=event.language.clone();}
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
                            language:event.language.clone(),
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
        let before = (
            self.enabled,
            self.engine,
            self.language,
            self.manual_language,
            self.disable_lookahead,
        );
        let translation_before=(self.translation.enabled,self.translation.target);
        ui.label(tr("实时字幕"));
        ui.menu_button("⚙", |ui| {
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
                    .selected_text(tr(if self.manual_language{auto_subtitle::LANGUAGES[self.language].1}else{"自动识别"}))
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(!self.manual_language,tr("自动识别")).clicked(){self.manual_language=false;}
                        for (index, (_, label)) in auto_subtitle::LANGUAGES
                            .iter()
                            .enumerate()
                            .take(if self.engine == 0 { 5 } else { 2 })
                        {
                            if ui.selectable_label(self.manual_language&&self.language==index,tr(label)).clicked(){self.language=index;self.manual_language=true;}
                        }
                    });
                if let Some((_,label))=self.replay_language.as_deref().or(self.finalized_language.as_deref()).and_then(|code|auto_subtitle::LANGUAGES.iter().find(|(language,_)|*language==code)){
                    ui.small(format!("{}: {}",tr("识别结果语言"),tr(label)));
                }
            } else {
                ui.label(tr("中文 / 英语（双语识别）"));
                ui.label(tr("翻译源语言"));
                egui::ComboBox::from_id_salt("zip_translation_source").selected_text(tr(auto_subtitle::LANGUAGES[self.language].1)).show_ui(ui,|ui|{
                    for (index,(_,label)) in auto_subtitle::LANGUAGES.iter().take(2).enumerate(){ui.selectable_value(&mut self.language,index,tr(label));}
                });
            }
            ui.small(tr("SenseVoice 分句显示；Zipformer 连续更新中英识别结果。"));
            ui.small(tr("首次使用会下载模型；模型就绪后从当前播放位置开始识别。"));
            ui.small(tr("实时字幕自动缓存；点播回放优先复用已确认字幕。"));
            if self.engine==0 {
                let mut enabled=!self.disable_lookahead;
                if ui.checkbox(&mut enabled,tr("点播提前识别字幕")).changed(){self.disable_lookahead=!enabled;}
                ui.small(tr("本地视频或已确认的 HLS 点播提前识别约 20 秒，按播放时间显示；首次准备和跳转后可能仍需等待。"));
            }
            ui.separator();
            ui.small(tr("翻译优先使用可用 GPU，失败时回退低占用 CPU。"));
            ui.small(tr("计算偏好统一在第三方支持中设置。"));
            if !self.translation.backend.is_empty(){ui.small(&self.translation.backend);}
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
        ui.add_enabled_ui(has_audio||self.enabled,|ui|{crate::theme::compact_toggle_switch_size(ui,"",&mut self.enabled,crate::theme::ToggleSwitchSize::Small).on_hover_text(tr("启用实时字幕"));});
        if self.enabled {
            ui.label(tr("翻译:"));
            let font=egui::TextStyle::Button.resolve(ui.style());
            let language_width=auto_subtitle::LANGUAGES.iter().take(4).map(|(_,label)|ui.painter().layout_no_wrap(tr(label).to_owned(),font.clone(),ui.visuals().text_color()).size().x).fold(0.0_f32,f32::max);
            let selector_width=(language_width+ui.spacing().icon_width+ui.spacing().button_padding.x*2.0+4.0).max(44.0);
            egui::ComboBox::from_id_salt("live_translation_target").width(selector_width).selected_text(tr(auto_subtitle::LANGUAGES[self.translation.target].1)).show_ui(ui,|ui|{
                for (index,(_,label)) in auto_subtitle::LANGUAGES.iter().take(4).enumerate(){ui.selectable_value(&mut self.translation.target,index,tr(label));}
            });
            crate::theme::compact_toggle_switch_size(ui,"",&mut self.translation.enabled,crate::theme::ToggleSwitchSize::Small).on_hover_text(tr("翻译:"));
            if self.translation.enabled&&!self.translation.backend.is_empty(){
                ui.small(if self.translation.backend.starts_with("GPU"){"GPU"}else{"CPU"}).on_hover_text(&self.translation.backend);
            }
            let mut show_source=!self.translation.hide_source;
            if crate::theme::compact_toggle_switch_size(ui,tr("显示源字幕"),&mut show_source,crate::theme::ToggleSwitchSize::Small).changed(){self.translation.hide_source=!show_source;}
        }else{self.translation.enabled=false;}
        if translation_before!=(self.translation.enabled,self.translation.target){self.translation.reset();self.translation.error=None;}
        if before
            != (
                self.enabled,
                self.engine,
                self.language,
                self.manual_language,
                self.disable_lookahead,
            )
        {
            self.reset();
            self.error = None;
        }
    }
    // Must run independently of toolbar visibility and context-menu painting.
    pub(crate) fn tick_translation(&mut self,ctx:&egui::Context){
        self.translation.poll();
        if self.enabled && self.translation.enabled {
            let source=if self.scheduled {self.replay_caption.as_deref().unwrap_or("")}else{self.replay_caption.as_deref().unwrap_or(&self.finalized)};
            let detected=if self.replay_caption.is_some(){self.replay_language.as_deref()}else{self.finalized_language.as_deref()};
            let language=if self.engine==0{translation_source_language(source,detected,self.recognition_language())}else{self.recognition_language()};
            if language!="auto"{self.translation.submit(source,language,ctx);}
            self.translation.mark_visible(source);
            if self.scheduled&&self.reusable {
                let mut ahead_language=if source.is_empty()||language=="auto"{None}else{Some(language)};
                for cue in self.replay.iter().filter(|cue|cue.end>self.media_position&&cue.start<=self.media_position+20.0){
                    let language=translation_source_language(&cue.text,cue.language.as_deref(),self.recognition_language());
                    if language=="auto"{continue;}
                    if let Some(active)=ahead_language{if active!=language{break;}}else{ahead_language=Some(language);}
                    self.translation.submit(&cue.text,language,ctx);
                }
            }
        }
    }
    pub(crate) fn status(&self,ui:&mut egui::Ui){
        if let Some(error)=&self.translation.error {
            ui.colored_label(ui.visuals().error_fg_color,error);
            if ui.small_button(tr("前往第三方支持")).clicked(){crate::third_party::request_open(ui.ctx());}
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
        let caption = self
            .replay_caption
            .as_ref()
            .map(String::as_str)
            .unwrap_or(if self.scheduled { "" } else { &self.caption });
        let source=(!caption.is_empty()&&(self.replay_caption.is_some()||self.updated.is_some_and(|time|time.elapsed()<Duration::from_secs(8)))).then_some(caption);
        let display=self.translation.display(source);
        if display.source.is_none()&&display.translated.is_none(){return;}
        let canvas = canvas.intersect(ui.clip_rect());
        if canvas.width() < 48.0 || canvas.height() < 40.0 {
            return;
        }
        let (margin_x, margin_y) = crate::subtitle_player::live_margins();
        let top = crate::subtitle_player::live_top();
        let safe = subtitle_box(canvas, margin_x, margin_y, top);
        if safe.width() < 24.0 || safe.height() < 24.0 {
            return;
        }
        let painter = ui.painter().with_clip_rect(safe);
        let width = (safe.width() - 16.0).max(1.0);
        let layout = |text: String| {
            let mut job = egui::text::LayoutJob::simple(
                text,
                egui::FontId::proportional(22.0),
                egui::Color32::WHITE,
                width,
            );
            job.wrap.break_anywhere = true;
            painter.layout_job(job)
        };
        // Each lane wraps independently; a delayed translation never replaces source text.
        let fit=|text:&str|{
            let mut galley=layout(text.to_owned());
            if galley.rows.len()>2 {
                let boundaries:Vec<_>=text.char_indices().map(|(index,_)|index).chain(std::iter::once(text.len())).collect();
                let (mut low,mut high)=(0,boundaries.len()-1);
                while low<high {let mid=(low+high)/2;if layout(text[boundaries[mid]..].to_owned()).rows.len()>2{low=mid+1;}else{high=mid;}}
                galley=layout(text[boundaries[low]..].to_owned());
            }
            galley
        };
        let source=display.source.map(fit);
        let translated=display.translated.map(|text|layout(text.to_owned()));
        let heading=(display.previous&&translated.is_some()).then(||painter.layout_no_wrap(tr("稍早译文").to_owned(),egui::FontId::proportional(12.0),egui::Color32::from_gray(190)));
        // Stack actual text heights with no reserved rows or inter-block gap.
        let source_height=source.as_ref().map_or(0.0,|g|g.size().y+8.0);
        let translated_height=translated.as_ref().map_or(0.0,|g|g.size().y+8.0+if heading.is_some(){16.0}else{0.0});
        let height=source_height+translated_height;
        let start_y=if top{safe.top()}else{safe.bottom()-height};
        if let Some(galley)=source {
            let position=egui::pos2(safe.center().x-galley.size().x/2.0,start_y+source_height-galley.size().y-4.0);
            painter.rect_filled(egui::Rect::from_min_size(position-egui::vec2(8.0,4.0),galley.size()+egui::vec2(16.0,8.0)),4.0,egui::Color32::from_black_alpha(128));
            painter.galley(position,galley,egui::Color32::WHITE);
        }
        if let Some(galley)=translated {
            let label_height=if heading.is_some(){16.0}else{0.0};
            let lane_y=start_y+source_height;
            let position=egui::pos2(safe.center().x-galley.size().x/2.0,lane_y+4.0+label_height);
            let block_width=galley.size().x.max(heading.as_ref().map_or(0.0,|g|g.size().x));
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(safe.center().x-block_width/2.0-8.0,lane_y),egui::vec2(block_width+16.0,galley.size().y+8.0+label_height)),4.0,egui::Color32::from_rgba_unmultiplied(10,20,35,128));
            if let Some(heading)=heading {let position=egui::pos2(safe.center().x-heading.size().x/2.0,lane_y+2.0);painter.galley(position,heading,egui::Color32::from_gray(190));}
            painter.galley(position,galley,egui::Color32::WHITE);
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

fn translation_source_language(text:&str,detected:Option<&str>,configured:&'static str)->&'static str{
    if configured=="yue"{return "yue";}
    match detected.map(|value|value.trim().trim_start_matches("<|").trim_end_matches("|>")){
        Some("ja"|"japanese")=>return "ja",Some("ko"|"korean")=>return "ko",Some("zh"|"chinese")=>return "zh",Some("en"|"english")=>return "en",Some("yue")=>return "yue",_=>{}
    }
    // Legacy cached cues have no language metadata. Kana/Hangul are stronger
    // evidence than a default Mandarin setting; Han-only text stays ambiguous.
    if text.chars().any(|c|matches!(c,'\u{3040}'..='\u{30ff}'|'\u{ff66}'..='\u{ff9f}')){return "ja";}
    if text.chars().any(|c|matches!(c,'\u{ac00}'..='\u{d7af}'|'\u{1100}'..='\u{11ff}')){return "ko";}
    if text.is_ascii()&&text.chars().any(|c|c.is_ascii_alphabetic()){return "en";}
    configured
}

fn subtitle_box(canvas: egui::Rect, margin_x: f32, margin_y: f32, top: bool) -> egui::Rect {
    let horizontal = margin_x
        .max(0.0)
        .min(((canvas.width() - 48.0) / 2.0).max(0.0));
    let bottom = margin_y.max(0.0).min((canvas.height() - 32.0).max(0.0));
    egui::Rect::from_min_max(
        egui::pos2(
            canvas.left() + horizontal,
            canvas.top() + if top { bottom } else { 0.0 },
        ),
        egui::pos2(
            canvas.right() - horizontal,
            canvas.bottom() - if top { 0.0 } else { bottom },
        ),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn vod_prefetch_translates_future_cues_without_displaying_them_early(){
        let mut live=Live::default();live.enabled=true;live.translation.enabled=true;live.translation.target=0;live.scheduled=true;live.reusable=true;live.media_position=1.0;
        live.replay=vec![crate::live_subtitle_cache::Cue{start:5.0,end:7.0,text:"未来字幕".into(),language:Some("zh".into())}];
        live.tick_translation(&egui::Context::default());
        assert!(live.translation.display(None).translated.is_none());
        assert_eq!(live.translation.display(Some("未来字幕")).translated,Some("未来字幕"));
    }
    #[test]
    fn translation_uses_detected_language_instead_of_default_mandarin(){
        assert_eq!(translation_source_language("星のあしあと",Some("ja"),"zh"),"ja");
        assert_eq!(translation_source_language("星のあしあと",None,"zh"),"ja");
        assert_eq!(translation_source_language("안녕하세요",None,"zh"),"ko");
        assert_eq!(translation_source_language("星光",Some("ja"),"auto"),"ja");
        assert_eq!(translation_source_language("星光",None,"auto"),"auto");
        let live=Live::default();assert_eq!(live.recognition_language(),"auto");
    }
    #[test]
    fn translation_progresses_without_toolbar_or_menu(){
        let mut live=Live::default();live.enabled=true;live.translation.enabled=true;live.translation.target=0;live.manual_language=true;
        live.finalized="第一句确认文本".into();
        let ctx=egui::Context::default();
        live.tick_translation(&ctx);
        assert_eq!(live.translation.display(Some(&live.finalized)).translated,Some("第一句确认文本"));
        live.finalized="第二句确认文本".into();
        live.tick_translation(&ctx);
        assert_eq!(live.translation.display(Some(&live.finalized)).translated,Some("第二句确认文本"));
    }
    use super::*;
    #[test]
    fn subtitle_width_excludes_both_margins_and_handles_narrow_windows() {
        let canvas = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 450.0));
        let rect = subtitle_box(canvas, 30.0, 20.0, false);
        assert_eq!(rect.left(), 130.0);
        assert_eq!(rect.right(), 870.0);
        assert_eq!(rect.width(), 740.0);
        assert_eq!(rect.bottom(), 480.0);
        let top = subtitle_box(canvas, 30.0, 40.0, true);
        assert_eq!(top.top(), 90.0);
        assert_eq!(top.bottom(), 500.0);
        assert_eq!(top.width(), 740.0);
        let narrow = subtitle_box(canvas, 1000.0, 1000.0, false);
        assert_eq!(narrow.width(), 48.0);
        assert_eq!(narrow.height(), 32.0);
        assert!(canvas.contains_rect(narrow));
    }
    #[test]
    fn covered_audio_skips_recognition_but_uncached_audio_and_eof_pass() {
        let (sender, receiver) = mpsc::sync_channel(4);
        let tap = Tap {
            sender,
            ready: Arc::new(AtomicBool::new(true)),
            overloaded: Arc::new(AtomicBool::new(false)),
            replay: Arc::new(vec![crate::live_subtitle_cache::Cue {
                language:None,
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
