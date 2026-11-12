//! Stream the desktop straight into FFmpeg, without a frame-image cache.
mod countdown;
mod diagnostics;
mod displays;
mod history;
mod outline;
mod pcm_stream;
mod preferences;
mod system_audio;
mod toolbar;
mod ui;
use crate::{
    localization::tr,
    media_tools::{command, find_tool},
};

#[cfg(windows)]
fn downloads_directory() -> Result<PathBuf, String> {
    use std::{ffi::c_void, os::windows::ffi::OsStringExt};
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(memory: *mut c_void);
    }
    let downloads = Guid {
        a: 0x374de290,
        b: 0x123f,
        c: 0x4565,
        d: [0x91, 0x64, 0x39, 0xc4, 0x92, 0x5e, 0x46, 0x7b],
    };
    let mut raw = std::ptr::null_mut();
    let status = unsafe { SHGetKnownFolderPath(&downloads, 0, std::ptr::null_mut(), &mut raw) };
    let result = if status >= 0 && !raw.is_null() {
        let mut length = 0;
        unsafe {
            while *raw.add(length) != 0 {
                length += 1;
            }
            let path = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
                raw, length,
            )));
            if path.is_absolute() {
                Ok(path)
            } else {
                Err(tr("无法确定用户下载目录。").into())
            }
        }
    } else {
        Err(format!(
            "{} (0x{:08X})",
            tr("无法确定用户下载目录。"),
            status as u32
        ))
    };
    unsafe {
        CoTaskMemFree(raw.cast());
    }
    result
}

#[cfg(not(windows))]
fn downloads_directory() -> Result<PathBuf, String> {
    Err(tr("录屏当前仅支持 Windows。").into())
}
use eframe::egui;
use std::{
    io::{BufRead, Read, Write},
    path::PathBuf,
    process::Stdio,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}
impl Area {
    fn valid(self, desktop: Self) -> bool {
        self.width > 0
            && self.height > 0
            && self.x >= desktop.x
            && self.y >= desktop.y
            && i64::from(self.x) + i64::from(self.width)
                <= i64::from(desktop.x) + i64::from(desktop.width)
            && i64::from(self.y) + i64::from(self.height)
                <= i64::from(desktop.y) + i64::from(desktop.height)
    }
}

#[cfg(windows)]
fn screen(all: bool) -> Area {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetSystemMetrics(index: i32) -> i32;
    }
    unsafe {
        if all {
            Area {
                x: GetSystemMetrics(76),
                y: GetSystemMetrics(77),
                width: GetSystemMetrics(78),
                height: GetSystemMetrics(79),
            }
        } else {
            Area {
                x: 0,
                y: 0,
                width: GetSystemMetrics(0),
                height: GetSystemMetrics(1),
            }
        }
    }
}
#[cfg(not(windows))]
fn screen(_: bool) -> Area {
    Area {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    }
}

#[derive(Clone, Debug)]
struct Options {
    area: Area,
    fps: u32,
    crf: u32,
    mp4: bool,
    mouse: bool,
    system_audio: bool,
    path: PathBuf,
}
impl Options {
    #[cfg(test)]
    fn args(&self) -> Vec<String> {
        self.args_with_audio(None)
    }
    fn args_with_audio(&self, audio: Option<&system_audio::Session>) -> Vec<String> {
        let mut args: Vec<String> = [
            "-hide_banner",
            "-loglevel",
            "warning",
            "-nostats",
            "-n",
            "-f",
            "gdigrab",
            "-framerate",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        args.extend([
            self.fps.to_string(),
            "-draw_mouse".into(),
            u8::from(self.mouse).to_string(),
            "-offset_x".into(),
            self.area.x.to_string(),
            "-offset_y".into(),
            self.area.y.to_string(),
            "-video_size".into(),
            format!("{}x{}", self.area.width, self.area.height),
            "-i".into(),
            "desktop".into(),
        ]);
        if let Some(audio) = audio {
            args.extend([
                "-thread_queue_size".into(),
                "512".into(),
                "-probesize".into(),
                "32".into(),
                "-analyzeduration".into(),
                "0".into(),
                "-f".into(),
                "f32le".into(),
                "-ar".into(),
                audio.rate.to_string(),
                "-ac".into(),
                audio.channels.to_string(),
                "-i".into(),
                audio.address.clone(),
                "-map".into(),
                "0:v:0".into(),
                "-map".into(),
                "1:a:0".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "128k".into(),
                "-ac".into(),
                "2".into(),
            ]);
        } else {
            args.push("-an".into());
        }
        args.extend(
            [
                "-progress",
                "pipe:1",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        args.extend([
            self.crf.to_string(),
            "-vf".into(),
            "pad=ceil(iw/2)*2:ceil(ih/2)*2".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-g".into(),
            (self.fps * 2).to_string(),
        ]);
        if self.mp4 {
            args.extend(
                ["-movflags", "+frag_keyframe+empty_moov+default_base_moof"]
                    .into_iter()
                    .map(str::to_owned),
            );
        }
        args.extend([
            "-f".into(),
            if self.mp4 {
                "mp4".into()
            } else {
                "matroska".into()
            },
        ]);
        args
    }
}

enum Event {
    Countdown(Instant),
    Starting,
    Recording,
    Stopping,
    Finished(Result<PathBuf, String>),
}
struct Job {
    area: Area,
    stop: mpsc::Sender<()>,
    events: mpsc::Receiver<Event>,
    path: PathBuf,
    started: Option<Instant>,
    countdown: Option<Instant>,
    stopping: bool,
}
impl Drop for Job {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn record(
    options: Options,
    ffmpeg: PathBuf,
    stop: mpsc::Receiver<()>,
    events: &mpsc::Sender<Event>,
    ctx: &egui::Context,
) -> Result<PathBuf, String> {
    let trace = diagnostics::Trace::new();
    trace.note(
        "requested",
        format!("ffmpeg={} options={options:?}", ffmpeg.display()),
    );
    let result = record_inner(options, ffmpeg, stop, events, ctx, &trace);
    trace.finish(result)
}

fn record_inner(
    options: Options,
    ffmpeg: PathBuf,
    stop: mpsc::Receiver<()>,
    events: &mpsc::Sender<Event>,
    ctx: &egui::Context,
    trace: &diagnostics::Trace,
) -> Result<PathBuf, String> {
    trace.note("countdown", "3 seconds");
    // Give the root window time to minimize. Cancellation also works during preparation.
    let deadline = Instant::now() + Duration::from_secs(3);
    let _ = events.send(Event::Countdown(deadline));
    ctx.request_repaint();
    match stop.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        _ => return Err(tr("录屏已取消").into()),
    }
    let _ = events.send(Event::Starting);
    ctx.request_repaint();
    if !options.area.valid(screen(true)) {
        return Err(tr("录制区域超出桌面范围，请重新设置。").into());
    }
    trace.note("audio.init", format!("enabled={}", options.system_audio));
    let mut audio =
        if options.system_audio {
            Some(system_audio::Session::start(trace).map_err(|error| {
                format!("{}\n{error}", tr("无法录制系统声音，可关闭声音后重试。"))
            })?)
        } else {
            None
        };
    if let Some(audio) = &audio {
        trace.note(
            "audio.ready",
            format!("sample_rate={} channels={}", audio.rate, audio.channels),
        );
    }
    let args = options.args_with_audio(audio.as_ref());
    trace.note(
        "ffmpeg.spawn",
        format!(
            "args={} output={}",
            serde_json::to_string(&args).unwrap_or_default(),
            options.path.display()
        ),
    );
    let mut child = command(ffmpeg)
        .args(&args)
        .arg(&options.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("FFmpeg spawn: {e}"))?;
    trace.note("ffmpeg.pid", child.id());
    let output = child.stdout.take().unwrap();
    let progress_events = events.clone();
    let context = ctx.clone();
    let progress_trace = trace.clone();
    let progress = std::thread::spawn(move || {
        let mut captured = false;
        for line in std::io::BufReader::new(output)
            .lines()
            .map_while(Result::ok)
        {
            if !captured
                && line
                    .strip_prefix("frame=")
                    .and_then(|n| n.trim().parse::<u64>().ok())
                    .is_some_and(|n| n > 0)
            {
                captured = true;
                progress_trace.note("recording", "first encoded video frame reported");
                let _ = progress_events.send(Event::Recording);
                context.request_repaint();
            }
        }
        captured
    });
    let mut stderr = child.stderr.take().unwrap();
    let stderr_trace = trace.clone();
    // Drain continuously, keeping only a bounded diagnostic tail for long recordings.
    let errors = std::thread::spawn(move || {
        let mut tail = Vec::new();
        let mut block = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut block) {
            if n == 0 {
                break;
            }
            stderr_trace.note("ffmpeg.stderr", String::from_utf8_lossy(&block[..n]));
            tail.extend_from_slice(&block[..n]);
            if tail.len() > 16384 {
                tail.drain(..tail.len() - 16384);
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let mut stopping = None;
    let mut audio_error = None;
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                trace.note(
                    "ffmpeg.exit",
                    format!(
                        "status={status} code={:?} stop_requested={}",
                        status.code(),
                        stopping.is_some()
                    ),
                );
                break Ok(status);
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error.to_string());
            }
            Ok(None) => {}
        }
        if audio_error.is_none() {
            audio_error = audio.as_ref().and_then(|audio| audio.error());
        }
        let requested = audio_error.is_some()
            || match stop.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => true,
                Err(mpsc::TryRecvError::Empty) => false,
            };
        if stopping.is_none() && requested {
            trace.note(
                "stop",
                audio_error
                    .as_deref()
                    .unwrap_or("user request or control channel disconnected"),
            );
            if let Some(audio) = &mut audio {
                audio.finish();
            }
            if let Some(mut input) = child.stdin.take() {
                let _ = input.write_all(b"q\n");
                let _ = input.flush();
            }
            stopping = Some(Instant::now());
            let _ = events.send(Event::Stopping);
            ctx.request_repaint();
        }
        if stopping.is_some_and(|time| time.elapsed() > Duration::from_secs(20)) {
            trace.note("stop.timeout", "force terminate after 20 seconds");
            let _ = child.kill();
            let _ = child.wait();
            break Err(tr("录屏收尾超时，文件可能不完整。").into());
        }
        std::thread::sleep(Duration::from_millis(40));
    };
    if let Some(audio) = &audio {
        if audio_error.is_none() {
            audio_error = audio.error();
        }
        audio.stop();
    }
    drop(audio);
    let error = errors.join().unwrap_or_default();
    let captured = progress.join().unwrap_or(false);
    trace.note(
        "output",
        format!(
            "captured={captured} bytes={:?}",
            options.path.metadata().map(|meta| meta.len())
        ),
    );
    if let Some(error) = audio_error {
        return Err(format!(
            "{}\n{error}\n{}",
            tr("系统声音录制中断，请检查录屏文件。"),
            options.path.display()
        ));
    }
    match outcome {
        Ok(status)
            if status.success()
                && captured
                && options.path.metadata().is_ok_and(|m| m.len() > 0) =>
        {
            Ok(options.path)
        }
        Ok(_) => Err(format!(
            "{}\n{}\n{}",
            tr("录屏失败；若已生成文件，请检查其完整性。"),
            options.path.display(),
            error
        )),
        Err(message) => Err(format!("{message}\n{}\n{error}", options.path.display())),
    }
}

pub use ui::Recorder;

fn recording_path(extension: &str) -> Result<PathBuf, String> {
    let directory = downloads_directory()?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    loop {
        let path = directory.join(format!("recording-{stamp}.{extension}"));
        if !path
            .try_exists()
            .map_err(|error| format!("{}: {error}", path.display()))?
        {
            return Ok(path);
        }
        stamp += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_validation_handles_negative_origins_and_overflow() {
        let desktop = Area {
            x: -1920,
            y: 0,
            width: 3840,
            height: 1080,
        };
        assert!(
            Area {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080
            }
            .valid(desktop)
        );
        assert!(
            !Area {
                x: 0,
                y: 0,
                width: i32::MAX,
                height: 1080
            }
            .valid(desktop)
        );
        assert!(
            !Area {
                x: -1921,
                y: 0,
                width: 10,
                height: 10
            }
            .valid(desktop)
        );
        assert!(
            !Area {
                width: 0,
                ..desktop
            }
            .valid(desktop)
        );
    }
    #[test]
    fn streaming_arguments_keep_container_and_quality_and_pad_odd_sizes() {
        let mut options = Options {
            area: Area {
                x: -10,
                y: 0,
                width: 527,
                height: 295,
            },
            fps: 30,
            crf: 23,
            mp4: false,
            mouse: true,
            system_audio: false,
            path: PathBuf::from("test.mkv"),
        };
        let args = options.args();
        assert!(args.windows(2).any(|a| a == ["-offset_x", "-10"]));
        assert!(args.windows(2).any(|a| a == ["-video_size", "527x295"]));
        assert!(args.windows(2).any(|a| a == ["-crf", "23"]));
        assert!(args.windows(2).any(|a| a == ["-f", "matroska"]));
        assert!(args.contains(&"pad=ceil(iw/2)*2:ceil(ih/2)*2".into()));
        assert!(args.contains(&"-n".into()));
        options.mp4 = true;
        assert!(
            options
                .args()
                .windows(2)
                .any(|a| a == ["-movflags", "+frag_keyframe+empty_moov+default_base_moof"])
        );
    }
}
