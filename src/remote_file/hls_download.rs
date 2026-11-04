use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, atomic::{AtomicU8, AtomicU64, Ordering}, mpsc},
    time::Duration,
};
use eframe::egui;
use crate::{localization::tr, media_tools::command, media_preview::hls_input_options};

#[derive(Default)]
pub(crate) struct Download {
    job: Option<Job>,
    result: Option<Result<PathBuf, String>>,
}

struct Job {
    cancel: Option<mpsc::Sender<()>>,
    result: mpsc::Receiver<Result<Option<PathBuf>, String>>,
    worker: Option<std::thread::JoinHandle<()>>,
    percent: Arc<AtomicU8>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.take();
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}

impl Download {
    pub(crate) fn active(&self) -> bool { self.job.is_some() }

    pub(crate) fn poll(&mut self) {
        if let Some(job) = &self.job {
            match job.result.try_recv() {
                Ok(result) => {
                    self.result = result.transpose();
                    self.job = None;
                    if let Some(Ok(path)) = &self.result {
                        rfd::MessageDialog::new().set_title(tr("下载 MP4 完成"))
                            .set_description(path.display().to_string())
                            .set_level(rfd::MessageLevel::Info).show();
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err(tr("网络加载任务已中断").into()));
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    pub(crate) fn button(&mut self, ui: &mut egui::Ui, source: &Path, title: &str, tools: Option<(&Path, &Path, f64)>, cache: Option<&super::HlsCache>) {
        if let Some(job) = &mut self.job {
            let percent = job.percent.load(Ordering::Relaxed);
            let spinner = ui.add(egui::Spinner::new().size(44.0));
            if percent < 100 {
                ui.painter().text(spinner.rect.center(), egui::Align2::CENTER_CENTER,
                    format!("{percent}%"), egui::FontId::proportional(12.0), ui.visuals().text_color());
            }
            ui.label(tr(if job.cancel.is_none() { "正在取消…" }
                else if percent == 100 { "正在处理音视频…" }
                else { "正在下载 MP4…" }));
            if ui.add_enabled(job.cancel.is_some(), egui::Button::new(tr("取消"))).clicked() {
                job.cancel.take();
            }
            return;
        }
        if ui.add_enabled(tools.is_some(), egui::Button::new(tr("下载 MP4"))).clicked() {
            let stem = title.strip_suffix(".m3u8").unwrap_or(title);
            let safe: String = stem.chars().map(|c| {
                if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }
            }).take(100).collect();
            let safe = safe.trim_matches([' ', '.']);
            let safe = if safe.is_empty() { "video" } else { safe };
            if let (Some((ffmpeg, ffprobe, duration)), Some(mut target)) = (tools, rfd::FileDialog::new()
                .add_filter("MP4", &["mp4"]).set_file_name(format!("{safe}.mp4")).save_file()) {
                target.set_extension("mp4");
                if target.exists() {
                    self.result = Some(Err(tr("目标文件已存在，请使用新文件名。").into()));
                    return;
                }
                self.result = None;
                self.job = Some(start(source.to_owned(), ffmpeg.to_owned(), ffprobe.to_owned(), target, duration, cache.cloned()));
                ui.ctx().request_repaint();
            }
        }
    }

    pub(crate) fn status(&self, ui: &mut egui::Ui) {
        match &self.result {
            Some(Ok(path)) => { ui.label(format!("{} · {}", tr("已保存"), path.display())); }
            Some(Err(error)) => { ui.colored_label(ui.visuals().error_fg_color, error); }
            None => {}
        }
    }
}

fn start(source: PathBuf, ffmpeg: PathBuf, ffprobe: PathBuf, target: PathBuf, duration: f64, shared: Option<super::HlsCache>) -> Job {
    let (cancel, cancellation) = mpsc::channel();
    let (sender, result) = mpsc::channel();
    let percent = Arc::new(AtomicU8::new(0));
    let worker_percent = percent.clone();
    let worker = std::thread::spawn(move || {
        let _ = sender.send(download(&source, &ffmpeg, &ffprobe, &target, duration, &cancellation, &worker_percent, shared.as_ref()));
    });
    Job { cancel: Some(cancel), result, worker: Some(worker), percent }
}

fn download(source: &Path, ffmpeg: &Path, ffprobe: &Path, target: &Path, duration: f64, cancel: &mpsc::Receiver<()>, percent: &Arc<AtomicU8>, shared: Option<&super::HlsCache>) -> Result<Option<PathBuf>, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = format!("hls-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    if let Some(shared) = shared {
        loop {
            if !matches!(cancel.recv_timeout(Duration::from_millis(100)), Err(mpsc::RecvTimeoutError::Timeout)) {
                return Ok(None);
            }
            let (progress, complete, exhausted) = shared.progress();
            if exhausted { return Err(tr("下载结果不完整，未保存 MP4。").into()); }
            if complete { break; }
            percent.store(progress, Ordering::Relaxed);
        }
        percent.store(100, Ordering::Relaxed);
        return merge(ffmpeg, ffprobe, target, &shared.manifest_path(), &id, duration, cancel);
    }
    let cache = crate::app_cache::network_root().join(&id);
    crate::app_cache::create_task_cache(&cache).map_err(|error| error.to_string())?;
    let result = download_and_merge(source, ffmpeg, ffprobe, target, &cache, &id, duration, cancel, percent);
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("segment-") || matches!(name.as_ref(), "cached.m3u8" | "cached.m3u8.tmp") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    let _ = fs::remove_dir(&cache);
    crate::app_cache::forget_task_cache(&cache);
    result
}

fn download_and_merge(source: &Path, ffmpeg: &Path, ffprobe: &Path, target: &Path, cache: &Path, id: &str, duration: f64, cancel: &mpsc::Receiver<()>, percent: &Arc<AtomicU8>) -> Result<Option<PathBuf>, String> {
    let list = cache.join("cached.m3u8");
    let mut fetch = command(ffmpeg);
    // Drop corrupt source packets. Missing segments and short output are checked separately.
    fetch.args(["-nostdin", "-v", "warning", "-n", "-nostats", "-progress", "pipe:1", "-fflags", "+discardcorrupt"]);
    hls_input_options(&mut fetch, source);
    fetch.arg("-i").arg(source)
        .args(["-map", "0:V:0?", "-map", "0:a:0?", "-c", "copy", "-abort_on", "empty_output",
            "-f", "hls", "-hls_time", "60", "-hls_playlist_type", "vod", "-hls_segment_filename"])
        .arg(cache.join("segment-%06d.ts")).arg(&list);
    if !run(&mut fetch, cancel, Some((percent.clone(), duration)))? { return Ok(None); }
    percent.store(100, Ordering::Relaxed);

    merge(ffmpeg, ffprobe, target, &list, id, duration, cancel)
}

fn merge(ffmpeg: &Path, ffprobe: &Path, target: &Path, list: &Path, id: &str, duration: f64, cancel: &mpsc::Receiver<()>) -> Result<Option<PathBuf>, String> {
    let temporary = target.with_file_name(format!(".unity-{id}.mp4"));
    drop(OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|error| error.to_string())?);
    let result = (|| {
        let mut merge = command(ffmpeg);
        // Allow the MP4 muxer to correct non-monotonic DTS in copied streams.
        merge.args(["-nostdin", "-v", "warning", "-y", "-protocol_whitelist", "file", "-f", "hls", "-i"]).arg(list)
            .args(["-c", "copy", "-abort_on", "empty_output", "-movflags", "+faststart", "-f", "mp4"]).arg(&temporary);
        if !run(&mut merge, cancel, None)? { return Ok(None); }
        let probe = command(ffprobe).args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
            .arg(&temporary).output().map_err(|error| error.to_string())?;
        let length = String::from_utf8_lossy(&probe.stdout).trim().parse::<f64>().unwrap_or(0.0);
        if !probe.status.success() || !length.is_finite() || length <= 0.0 || length + 1.0 < duration {
            return Err(tr("下载结果不完整，未保存 MP4。").into());
        }
        OpenOptions::new().write(true).open(&temporary).and_then(|file| file.sync_all()).map_err(|error| error.to_string())?;
        if !matches!(cancel.try_recv(), Err(mpsc::TryRecvError::Empty)) { return Ok(None); }
        crate::playlist::move_new(&temporary, target)?;
        Ok(Some(target.to_owned()))
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn run(cmd: &mut std::process::Command, cancel: &mpsc::Receiver<()>, progress: Option<(Arc<AtomicU8>, f64)>) -> Result<bool, String> {
    if !matches!(cancel.try_recv(), Err(mpsc::TryRecvError::Empty)) { return Ok(false); }
    let stdout = if progress.is_some() { Stdio::piped() } else { Stdio::null() };
    let mut child = cmd.stdin(Stdio::null()).stdout(stdout).stderr(Stdio::piped())
        .spawn().map_err(|error| error.to_string())?;
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
    let reader = progress.zip(child.stdout.take()).map(|((percent, duration), stdout)| {
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Some(micros) = line.strip_prefix("out_time_us=").and_then(|value| value.parse::<i64>().ok()) else { continue; };
                let value = (micros as f64 / 1_000_000.0 / duration * 100.0).clamp(0.0, 99.0) as u8;
                percent.fetch_max(value, Ordering::Relaxed);
            }
        })
    });
    let status = loop {
        if !matches!(cancel.recv_timeout(Duration::from_millis(100)), Err(mpsc::RecvTimeoutError::Timeout)) {
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
    if let Some(reader) = reader { let _ = reader.join(); }
    let (missing, message) = diagnostics.join().unwrap_or_default();
    let Some(status) = status? else { return Ok(false); };
    if !status.success() || missing {
        return Err(format!("{} · {}", tr("无法保存网络文件"), message.trim()));
    }
    Ok(true)
}
