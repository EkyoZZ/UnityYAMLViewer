use crate::{
    background_tasks::{Outcome, Progress, Target, Task},
    localization::tr,
    media_tools::command,
    network_video::{Input, Selection, Track},
};
use eframe::egui;
use std::{
    fs,
    io::{BufRead, BufReader},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

pub(crate) struct Download {
    cancel: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    result: mpsc::Receiver<Result<(), String>>,
    percent: Arc<AtomicU8>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Download {
    pub fn start(
        ctx: &egui::Context,
        name: String,
        url: String,
        selection: Selection,
        inputs: (Input, Option<Input>),
        duration: f64,
    ) -> Self {
        let (cancel, cancellation) = mpsc::channel();
        let cancel = Arc::new(Mutex::new(Some(cancel)));
        let (sender, result) = mpsc::channel();
        let percent = Arc::new(AtomicU8::new(0));
        let worker_percent = percent.clone();
        let task_percent = percent.clone();
        let task_cancel = cancel.clone();
        let cancel_button = cancel.clone();
        let task = Task::start(
            ctx,
            "视频缓存",
            name,
            move || {
                let active = task_cancel.lock().unwrap().is_some();
                let percent = task_percent.load(Ordering::Relaxed);
                Progress {
                    stage: tr(if !active {
                        "正在取消…"
                    } else if percent == 100 {
                        "正在处理音视频…"
                    } else {
                        "缓存中…"
                    })
                    .into(),
                    fraction: (percent > 0 && percent < 100).then(|| f32::from(percent) / 100.0),
                    cancellable: active,
                }
            },
            Some(Box::new(move || {
                cancel_button.lock().unwrap().take();
            })),
        );
        let worker = std::thread::spawn(move || {
            let result = cache(
                selection.clone(),
                inputs,
                duration,
                &cancellation,
                worker_percent,
            );
            let outcome = match &result {
                Ok(()) if selection.fully_cached() => {
                    Outcome::Complete(selection.track_cache(Track::Video)
                        .map(|path| Target::Playback { url, path }))
                }
                Ok(()) => Outcome::Cancelled,
                Err(error) => Outcome::Failed(error.clone()),
            };
            let _ = sender.send(result);
            task.finish(outcome);
        });
        Self {
            cancel,
            result,
            percent,
            worker: Some(worker),
        }
    }

    pub fn poll(&self) -> Option<Result<(), String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(tr("网络加载任务已中断").into())),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let percent = self.percent.load(Ordering::Relaxed);
        let active = self.cancel.lock().unwrap().is_some();
        let status = if !active {
            tr("正在取消…").to_owned()
        } else if percent == 100 {
            tr("正在处理音视频…").to_owned()
        } else {
            format!("{}{percent}%", tr("缓存中…"))
        };
        let font = egui::TextStyle::Button.resolve(ui.style());
        let measure = |text: &str| ui.painter().layout_no_wrap(
            text.to_owned(), font.clone(), ui.visuals().text_color()).size().x.ceil();
        let status_width = measure(&status) + 12.0;
        let cancel_width = measure(tr("取消")) + 2.0 * ui.spacing().button_padding.x;
        let width = status_width + ui.spacing().item_spacing.x + cancel_width;
        ui.allocate_ui_with_layout(egui::vec2(width, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.add_sized([status_width, 24.0], egui::Button::new(status)
                .fill(egui::Color32::TRANSPARENT)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(3.0).sense(egui::Sense::hover()));
            if ui.add_enabled_ui(active, |ui| ui.add_sized([cancel_width, 24.0], egui::Button::new(tr("取消")))).inner.clicked() {
                self.cancel.lock().unwrap().take();
            }
        });
    }
}

impl Drop for Download {
    fn drop(&mut self) {
        self.cancel.lock().unwrap().take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn cache(
    selection: Selection,
    (video, audio): (Input, Option<Input>),
    duration: f64,
    cancel: &mpsc::Receiver<()>,
    percent: Arc<AtomicU8>,
) -> Result<(), String> {
    let ffmpeg = crate::media_tools::find_tool("ffmpeg")
        .ok_or_else(|| tr("未安装 FFmpeg，请前往第三方支持安装。").to_owned())?;
    let ffprobe = crate::media_tools::find_tool("ffprobe")
        .ok_or_else(|| tr("未安装 ffprobe，请前往第三方支持安装。").to_owned())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let scratch = crate::app_cache::network_root().join(format!(
        "video-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    crate::app_cache::create_task_cache(&scratch).map_err(|error| error.to_string())?;
    let result = (|| {
        for (track, input) in
            std::iter::once((Track::Video, video)).chain(audio.map(|audio| (Track::Audio, audio)))
        {
            if selection.track_cache(track).is_some() {
                continue;
            }
            percent.store(0, Ordering::Relaxed);
            let (kind, staging_extension, preferred_extension, default_map) = match track {
                Track::Video => ("video", "mkv", "mp4", "0:V:0"),
                Track::Audio => ("audio", "mka", "m4a", "0:a:0"),
            };
            let staged = scratch.join(format!("track.{staging_extension}"));
            let preferred = scratch.join(format!("track.{preferred_extension}"));
            let stream = input
                .stream
                .map_or_else(|| default_map.to_owned(), |index| format!("0:{index}"));
            let mut fetch = command(&ffmpeg);
            fetch.args([
                "-nostdin",
                "-v",
                "warning",
                "-n",
                "-nostats",
                "-progress",
                "pipe:1",
                "-fflags",
                "+discardcorrupt",
            ]);
            input.options(&mut fetch);
            fetch.arg("-i").arg(&input.path).args(["-map", &stream]);
            if matches!(track, Track::Video) && !selection.split() {
                fetch.args(["-map", "0:a?"]);
            }
            fetch
                .args(["-c", "copy", "-abort_on", "empty_output"])
                .arg(&staged);
            if !run(&mut fetch, cancel, Some((percent.clone(), duration)))? {
                return Ok(());
            }
            percent.store(100, Ordering::Relaxed);
            let probe = command(&ffprobe)
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "format=duration:stream=codec_type",
                    "-of",
                    "json",
                ])
                .arg(&staged)
                .output()
                .map_err(|error| error.to_string())?;
            let metadata: serde_json::Value =
                serde_json::from_slice(&probe.stdout).unwrap_or_default();
            let length = metadata["format"]["duration"]
                .as_str()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0);
            let has_track = metadata["streams"]
                .as_array()
                .is_some_and(|streams| streams.iter().any(|stream| stream["codec_type"] == kind));
            if !probe.status.success()
                || !has_track
                || !length.is_finite()
                || length <= 0.0
                || length + 1.0 < duration
            {
                return Err(tr("缓存结果不完整。").into());
            }
            let mut remux = command(&ffmpeg);
            remux
                .args(["-nostdin", "-v", "error", "-n", "-i"])
                .arg(&staged)
                .args(["-map", "0", "-c", "copy", "-movflags", "+faststart"])
                .arg(&preferred);
            let (ready, extension) = match run(&mut remux, cancel, None) {
                Ok(true) => (&preferred, preferred_extension),
                Ok(false) => return Ok(()),
                Err(_) => (&staged, staging_extension),
            };
            fs::OpenOptions::new()
                .write(true)
                .open(ready)
                .and_then(|file| file.sync_all())
                .map_err(|error| error.to_string())?;
            if !matches!(cancel.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                return Ok(());
            }
            let _guard = crate::app_cache::mutation_guard();
            if selection.track_cache(track).is_none() {
                let target = selection
                    .track_file(track, extension)
                    .ok_or_else(|| tr("缓存结果不完整。").to_owned())?;
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                crate::playlist::move_new(ready, &target)?;
            }
        }
        let _guard = crate::app_cache::mutation_guard();
        selection.remember(None)
    })();
    if let Ok(entries) = fs::read_dir(&scratch) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
    let _ = fs::remove_dir(&scratch);
    crate::app_cache::forget_task_cache(&scratch);
    result
}

fn run(
    cmd: &mut std::process::Command,
    cancel: &mpsc::Receiver<()>,
    progress: Option<(Arc<AtomicU8>, f64)>,
) -> Result<bool, String> {
    if !matches!(cancel.try_recv(), Err(mpsc::TryRecvError::Empty)) {
        return Ok(false);
    }
    let stdout = if progress.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let stderr = child.stderr.take().expect("piped stderr");
    let diagnostics = std::thread::spawn(move || {
        let mut message = String::new();
        let mut missing = false;
        for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line);
            missing |= line.contains("Failed to open segment");
            if message.len() < 4096 {
                message.extend(line.chars().take(512));
                message.push('\n');
            }
        }
        (missing, message)
    });
    let reader = progress
        .zip(child.stdout.take())
        .map(|((percent, duration), stdout)| {
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    let Some(micros) = line
                        .strip_prefix("out_time_us=")
                        .and_then(|value| value.parse::<i64>().ok())
                    else {
                        continue;
                    };
                    if !duration.is_finite() || duration <= 0.0 {
                        continue;
                    }
                    let value =
                        (micros as f64 / 1_000_000.0 / duration * 100.0).clamp(0.0, 99.0) as u8;
                    percent.fetch_max(value, Ordering::Relaxed);
                }
            })
        });
    let status = loop {
        if !matches!(
            cancel.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            let _ = child.kill();
            let _ = child.wait();
            break Ok(None);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(Some(status)),
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error.to_string());
            }
        }
    };
    if let Some(reader) = reader {
        let _ = reader.join();
    }
    let (missing, message) = diagnostics.join().unwrap_or_default();
    let Some(status) = status? else {
        return Ok(false);
    };
    if !status.success() || missing {
        return Err(format!("{} · {}", tr("无法保存网络文件"), message.trim()));
    }
    Ok(true)
}
