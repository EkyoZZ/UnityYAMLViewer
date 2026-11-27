//! Stream the desktop straight into FFmpeg, without a frame-image cache.
mod camera;
mod countdown;
mod diagnostics;
mod displays;
mod history;
mod outline;
mod pcm_stream;
mod pip;
mod preferences;
mod system_audio;
mod toolbar;
mod ui;
use crate::{
    localization::tr,
    media_tools::{command, find_tool},
};

// Warmup consumes every frame for timing but only needs a one-byte readiness signal.
const WARMUP_PIXEL: &str = "crop=1:1:0:0:exact=1,format=gray";

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
    camera: camera::Settings,
    area: Area,
    fps: u32,
    crf: u32,
    mp4: bool,
    mouse: bool,
    system_audio: bool,
    capture_diagnostics: bool,
    microphone: Option<String>,
    muted: [bool; 2],
    path: PathBuf,
}
impl Options {
    #[cfg(test)]
    fn args(&self) -> Vec<String> {
        self.args_with_audio(None, false, VideoEncoder::Software, None)
    }
    fn args_with_audio(&self, audio: Option<&system_audio::Session>, preview: bool, video_encoder: VideoEncoder, clock_us: Option<u128>) -> Vec<String> {
        let mut args: Vec<String> = [
            "-hide_banner",
            "-loglevel",
            if self.capture_diagnostics { "info" } else { "warning" },
            "-nostats",
            "-n",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        if self.capture_diagnostics {
            args.extend(["-debug_ts".into(), "-benchmark".into()]);
        }
        let screen = self.camera.source != camera::Source::Camera;
        let camera = self.camera.source != camera::Source::Screen;
        if screen { args.extend([
            "-thread_queue_size".into(), if camera { "8" } else { "64" }.into(),
            "-f".into(), "gdigrab".into(), "-framerate".into(),
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
        ]); }
        let combined = screen && camera;
        let gated = screen && preview;
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
            ]);
        }
        let system_index = usize::from(screen);
        let microphone_index = system_index + usize::from(audio.is_some());
        let camera_index = microphone_index;
        if camera {
            let input = match &self.microphone {
                Some(device) => format!("video={}:audio={device}", self.camera.device),
                None => format!("video={}", self.camera.device),
            };
            if clock_us.is_some() && self.microphone.is_some() {
                args.extend(["-audio_buffer_size".into(), "50".into()]);
            }
            args.extend([
                "-thread_queue_size".into(), "32".into(), "-rtbufsize".into(), "64M".into(),
                "-probesize".into(), "32".into(), "-analyzeduration".into(), "0".into(),
                "-f".into(), "dshow".into(), "-i".into(), input,
            ]);
        } else if let Some(device) = &self.microphone {
            args.extend(["-thread_queue_size".into(), "256".into(), "-probesize".into(), "32".into(), "-analyzeduration".into(), "0".into(), "-f".into(), "dshow".into(),
                "-i".into(), format!("audio={device}")]);
        }
        let video_stream = if camera && !screen { format!("{camera_index}:v:0") } else { "0:v:0".to_owned() };
        let mut filters = Vec::new();
        if combined {
            if let Some(clock_us) = clock_us {
                // Give each capture input its own FFmpeg scheduling point. A mic or
                // preview request in the shared graph could otherwise starve gdigrab.
                for (index, source, warm) in [
                    (0, "screen", "desktop_warm"),
                    (camera_index, "camera_src", "camera_warm"),
                ] {
                    args.extend(["-filter_complex".into(), format!(
                        "[{index}:v]settb=AVTB,setpts=RTCTIME-{clock_us},split[{source}][warm_src];[warm_src]{WARMUP_PIXEL}[{warm}]"
                    )]);
                }
            } else {
                filters.push(format!("[0:v]setpts=N/(FRAME_RATE*TB)[screen];[{camera_index}:v]setpts=PTS-STARTPTS[camera_src]"));
            }
            filters.push(self.camera.overlay_filter(self.area, false));
        }
        if preview && !camera {
            filters.push(format!("[0:v]setpts=N/(FRAME_RATE*TB),pad=ceil(iw/2)*2:ceil(ih/2)*2,split[record_src][ready_src];[ready_src]{WARMUP_PIXEL}[preview]"));
            filters.push(format!("{}setpts=PTS-STARTPTS[record]",
                record_gate("[record_src]", "metadata", "record_start", ",")));
        } else if preview {
            let input = if combined { "[out]".to_owned() } else {
                format!("[{video_stream}]{}", self.camera.camera_only_filter())
            };
            let split = if combined { "split" } else { ",split" };
            let filter = if combined {
                pip::preview_filter()
            } else {
                format!("{},{}", camera_frame_limit(camera::PREVIEW_FPS), camera::PREVIEW_SCALE_FILTER)
            };
            let record = "[record_src]";
            filters.push(format!("{input}{split}{record}[preview_src];[preview_src]{filter}[preview]"));
            if clock_us.is_some() {
                filters.push(record_gate("[record_src]", "metadata", "record_start", "[record]"));
            } else {
                filters.push(format!("[record_src]{}[record]", camera_frame_limit(self.fps)));
            }
        }
        if clock_us.is_some() {
            args.extend(["-filter_complex".into(), filters.join(";")]);
            filters.clear();
        }
        for (enabled, index, name, muted) in [
            (audio.is_some(), system_index, "system", self.muted[0]),
            (self.microphone.is_some(), microphone_index, "mic", self.muted[1]),
        ] {
            if !enabled {
                continue;
            }
            let label = if name == "system" { "sys" } else { "mic" };
            let volume = u8::from(!muted);
            if let Some(clock_us) = clock_us {
                filters.push(format!(
                    "[{index}:a:0]asettb=1/1000000,asetpts=RTCTIME-{clock_us}-NB_SAMPLES*1000000/SR,asplit[{label}_live][{label}_warm]"
                ));
                filters.push(format!(
                    "{}aresample=48000:async=1000,volume@record_{name}={volume}[{label}]",
                    record_gate(&format!("[{label}_live]"), "ametadata", &format!("record_start_{label}"), ",")
                ));
            } else {
                filters.push(format!("[{index}:a:0]volume@record_{name}={volume}[{label}]"));
            }
        }
        let sound = match (audio.is_some(), self.microphone.is_some()) {
            (true, true) => {
                if clock_us.is_some() {
                    filters.push("[sys][mic]amix=inputs=2:duration=first:weights='1 2':normalize=1[mix]".into());
                } else {
                    filters.push("[sys]aresample=48000,asetpts=N/SR/TB[sys_sync];[mic]aresample=48000,asetpts=N/SR/TB[mic_sync];[sys_sync][mic_sync]amix=inputs=2:duration=first:weights='1 2':normalize=1[mix]".into());
                }
                Some("[mix]".to_owned())
            }
            (true, false) => Some("[sys]".into()),
            (false, true) => Some("[mic]".into()),
            (false, false) => None,
        };
        let sound = sound.map(|source| {
            if gated && clock_us.is_none() {
                filters.push(format!("{source}asplit[record_audio_src][warm_audio]"));
                filters.push(format!("{}asetpts=PTS-STARTPTS[record_audio]",
                    record_gate("[record_audio_src]", "ametadata", "record_start", ",")));
                "[record_audio]".to_owned()
            } else {
                source
            }
        });
        if combined && video_encoder != VideoEncoder::Software && self.fps <= 30
            && i64::from(self.area.width) * i64::from(self.area.height) <= 1920 * 1080
            && std::thread::available_parallelism().is_ok_and(|n| n.get() > 4)
        {
            // For HD PiP the default pool can spend more CPU dispatching tiny filter jobs than processing pixels.
            args.extend(["-filter_complex_threads".into(), "4".into()]);
        }
        if !filters.is_empty() { args.extend(["-filter_complex".into(), filters.join(";")]); }
        args.extend(["-map".into(), if preview { "[record]".into() } else if combined { "[out]".into() } else { video_stream }]);
        if let Some(sound) = sound {
            args.extend([
                "-map".into(),
                sound,
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "192k".into(),
                "-ac".into(),
                "2".into(),
            ]);
        } else {
            args.push("-an".into());
        }
        args.extend(
            [
                "-stats_period",
                "0.2",
                "-progress",
                "pipe:1",
            ].into_iter().map(str::to_owned),
        );
        if video_encoder != VideoEncoder::Software {
            args.extend(video_encoder.hardware_args(self.crf, combined));
        } else {
            args.extend(["-c:v".into(), "libx264".into(), "-preset".into(),
                if camera { "superfast" } else { "veryfast" }.into(),
                "-crf".into(), self.crf.to_string()]);
            if camera { args.extend(["-tune".into(), "zerolatency".into()]); }
        }
        if !combined && !preview {
            let filter = if screen { "pad=ceil(iw/2)*2:ceil(ih/2)*2".into() } else { format!("{},{}", self.camera.camera_only_filter(), camera_frame_limit(self.fps)) };
            args.extend(["-vf".into(), filter]);
        }
        if camera { args.extend(["-fps_mode".into(), "vfr".into()]); }
        if gated && (audio.is_some() || self.microphone.is_some()) {
            args.push("-shortest".into());
        }
        args.extend([
            "-pix_fmt".into(),
            video_encoder.pixel_format().into(),
            "-g".into(),
            (self.fps * 2).to_string(),
        ]);
        if clock_us.is_some() {
            args.extend(["-avoid_negative_ts".into(), "make_zero".into()]);
        }
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

fn record_gate(source: &str, metadata: &str, name: &str, destination: &str) -> String {
    format!("{source}{metadata}@{name}=mode=add:key=unity.record:value=1:enable=0,{metadata}=mode=select:key=unity.record:value=1{destination}")
}

fn schedule_record_gates(
    input: &mut impl Write,
    deadline: Instant,
    audio: [bool; 2],
    clock_us: Option<u128>,
) -> std::io::Result<()> {
    // FFmpeg services interactive commands on separate iterations. Install
    // both expressions early so they can use the same wall-clock boundary.
    let wall_deadline = std::time::SystemTime::now()
        + deadline.saturating_duration_since(Instant::now());
    let seconds = wall_deadline.duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?.as_secs_f64();
    if let Some(clock_us) = clock_us {
        let cutoff = wall_deadline.duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?.as_micros().saturating_sub(clock_us) as f64 / 1_000_000.0;
        let gate = format!("gte(time(0),{seconds:.6})*gte(t,{cutoff:.6})");
        for (enabled, name) in audio.into_iter().zip(["sys", "mic"]) {
            if enabled {
                writeln!(input, "cametadata@record_start_{name} -1 enable {gate}")?;
            }
        }
        writeln!(input, "cmetadata@record_start -1 enable {gate}")?;
    } else {
        if audio.into_iter().any(|enabled| enabled) {
            writeln!(input, "cametadata@record_start -1 enable gte(time(0),{seconds:.6})")?;
        }
        writeln!(input, "cmetadata@record_start -1 enable gte(time(0),{seconds:.6})")?;
    }
    input.flush()
}

fn camera_frame_limit(fps: u32) -> String {
    format!("select='isnan(prev_selected_t)+gt(floor(t*{fps}+0.001),floor(prev_selected_t*{fps}+0.001))'")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VideoEncoder {
    NvencPlanar,
    Nvenc,
    QuickSync,
    Software,
}

impl VideoEncoder {
    fn name(self) -> &'static str {
        match self {
            Self::NvencPlanar | Self::Nvenc => "h264_nvenc",
            Self::QuickSync => "h264_qsv",
            Self::Software => "libx264",
        }
    }

    fn pixel_format(self) -> &'static str {
        match self {
            Self::Nvenc | Self::QuickSync => "nv12",
            Self::Software | Self::NvencPlanar => "yuv420p",
        }
    }

    fn hardware_args(self, quality: u32, picture_in_picture: bool) -> Vec<String> {
        let (settings, latency): (&[&str], &[&str]) = match self {
            Self::NvencPlanar | Self::Nvenc => (
                &["-preset", "p3", "-tune", "ull", "-rc", "constqp", "-qp"],
                &["-bf", "0", "-zerolatency", "1"],
            ),
            Self::QuickSync => (
                &["-preset", "veryfast", "-global_quality"],
                &["-bf", "0", "-async_depth", "1", "-look_ahead", "0"],
            ),
            Self::Software => return Vec::new(),
        };
        let mut args: Vec<_> = ["-c:v", self.name()].into_iter().map(str::to_owned).collect();
        args.extend(settings.iter().copied().map(str::to_owned));
        args.push(quality.to_string());
        args.extend(latency.iter().copied().map(str::to_owned));
        if picture_in_picture && matches!(self, Self::NvencPlanar | Self::Nvenc) {
            // The first muxed video packet must be available as soon as the gate opens.
            args.extend(["-delay".into(), "0".into()]);
        }
        args
    }
}

fn choose_video_encoder(ffmpeg: &std::path::Path, options: &Options) -> VideoEncoder {
    if options.camera.source == camera::Source::Screen {
        return VideoEncoder::Software;
    }
    let (width, height) = if options.camera.source == camera::Source::Combined {
        ((options.area.width + 1) & !1, (options.area.height + 1) & !1)
    } else {
        (1920, 1080)
    };
    let input = format!("color=size={width}x{height}:rate={}", options.fps);
    // The overlay already produces yuv420p. Probe that input before repacking the whole frame to NV12.
    [VideoEncoder::NvencPlanar, VideoEncoder::Nvenc, VideoEncoder::QuickSync]
        .into_iter()
        .filter(|encoder| *encoder != VideoEncoder::NvencPlanar || options.camera.source == camera::Source::Combined)
        .find(|encoder| {
            command(ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-f", "lavfi", "-i"])
                .arg(&input)
                .args(["-frames:v", "1", "-pix_fmt", encoder.pixel_format()])
                .args(encoder.hardware_args(options.crf, options.camera.source == camera::Source::Combined))
                .args(["-f", "null", "-"])
                .output()
                .is_ok_and(|output| output.status.success())
        })
        .unwrap_or(VideoEncoder::Software)
}

enum Event {
    Countdown(mpsc::Sender<Instant>),
    Starting(mpsc::Sender<()>),
    CountdownComplete,
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
    countdown: Option<countdown::Request>,
    stopping: bool,
    pip: Option<pip::LiveView>,
    camera_frames: Option<mpsc::Receiver<egui::ColorImage>>,
    capture_started: bool,
    mute: std::sync::Arc<std::sync::Mutex<[bool; 2]>>,
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
    pip: Option<pip::Output>,
    mute: std::sync::Arc<std::sync::Mutex<[bool; 2]>>,
) -> Result<PathBuf, String> {
    let trace = diagnostics::Trace::new();
    trace.note(
        "requested",
        format!("ffmpeg={} options={options:?}", ffmpeg.display()),
    );
    let result = record_inner(options, ffmpeg, stop, events, ctx, &trace, pip, mute);
    trace.finish(result)
}

fn record_inner(
    options: Options,
    ffmpeg: PathBuf,
    stop: mpsc::Receiver<()>,
    events: &mpsc::Sender<Event>,
    ctx: &egui::Context,
    trace: &diagnostics::Trace,
    pip: Option<pip::Output>,
    mute: std::sync::Arc<std::sync::Mutex<[bool; 2]>>,
) -> Result<PathBuf, String> {
    if options.camera.source != camera::Source::Camera && !options.area.valid(screen(true)) {
        return Err(tr("录制区域超出桌面范围，请重新设置。").into());
    }
    trace.note("audio.init", format!("enabled={}", options.system_audio));
    let audio =
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
    let video_encoder = choose_video_encoder(&ffmpeg, &options);
    trace.note("video.encoder", format!("{} {}", video_encoder.name(), video_encoder.pixel_format()));
    let pip = if options.camera.source == camera::Source::Screen {
        Some(pip::Output::bind_screen_ready().map_err(|error| format!("Screen warmup socket: {error}"))?)
    } else {
        pip
    };
    let gated = options.camera.source != camera::Source::Camera && pip.is_some();
    if !gated {
        trace.note("countdown.waiting", "waiting for the numeral window to be displayed");
        let (ready, visible) = mpsc::channel();
        let _ = events.send(Event::Countdown(ready));
        ctx.request_repaint();
        let deadline = countdown::wait_until_visible(&visible, &stop, ctx)?;
        trace.note("countdown.shown", "3 seconds after first presentation");
        match stop.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            _ => return Err(tr("录屏已取消").into()),
        }
    }
    let (ready, released) = mpsc::channel();
    let _ = events.send(Event::Starting(ready));
    ctx.request_repaint();
    released.recv_timeout(Duration::from_secs(3))
        .map_err(|_| tr("录屏任务意外中断").to_owned())?;
    if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
        return Err(tr("录屏已取消").into());
    }
    let clock_us = (gated && options.camera.source == camera::Source::Combined)
        .then(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH))
        .transpose().map_err(|error| error.to_string())?.map(|duration| duration.as_micros());
    let mut capture_diagnostics = options.capture_diagnostics
        .then(|| diagnostics::Capture::open(&options, clock_us.is_some()))
        .transpose().map_err(|error| format!("Recording diagnostics: {error}"))?;
    let args = options.args_with_audio(audio.as_ref(), pip.is_some(), video_encoder, clock_us);
    trace.note(
        "ffmpeg.spawn",
        format!(
            "args={} output={}",
            serde_json::to_string(&args).unwrap_or_default(),
            options.path.display()
        ),
    );
    let mut encoder = command(ffmpeg);
    encoder.args(&args).arg(&options.path);
    if let Some(pip) = &pip { encoder.args(pip.args()); }
    if clock_us.is_some() {
        for stream in ["[desktop_warm]", "[camera_warm]"] {
            encoder.args(["-map", stream, "-an", "-c:v", "wrapped_avframe", "-fps_mode", "passthrough", "-f", "null", "-"]);
        }
    }
    if clock_us.is_some() && capture_diagnostics.is_none() {
        for (enabled, stream) in [(audio.is_some(), "[sys_warm]"), (options.microphone.is_some(), "[mic_warm]")] {
            if enabled {
                encoder.args(["-map", stream, "-vn", "-c:a", "pcm_s16le", "-f", "null", "-"]);
            }
        }
    } else if gated && clock_us.is_none() && (audio.is_some() || options.microphone.is_some()) {
        // Consume the mixed stream even while the recording gate drops its frames.
        encoder.args(["-map", "[warm_audio]", "-vn", "-c:a", "pcm_s16le", "-f", "null", "-"]);
    }
    if let Some(diagnostics) = &capture_diagnostics { diagnostics.add_outputs(&mut encoder); }
    let mut child = encoder
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
        let mut statistics = Vec::new();
        for line in std::io::BufReader::new(output)
            .lines()
            .map_while(Result::ok)
        {
            if line == "progress=end" {
                progress_trace.note("video.progress", statistics.join(" "));
            } else if line.starts_with("progress=") {
                statistics.clear();
            } else if ["frame=", "dup_frames=", "drop_frames=", "speed="].iter().any(|prefix| line.starts_with(prefix)) {
                statistics.push(line.clone());
            }
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
    let diagnostic_log = capture_diagnostics.take().map(|diagnostics| diagnostics.timestamps);
    let stderr_trace = trace.clone();
    // Drain continuously, keeping only a bounded diagnostic tail for long recordings.
    let errors = std::thread::spawn(move || {
        if let Some(log) = diagnostic_log {
            return diagnostics::read_timestamps(stderr, log, &stderr_trace);
        }
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
    let mut last_position = (options.camera.x, options.camera.y, true);
    let mut last_position_update = Instant::now();
    let mut applied_mute = options.muted;
    let mut warmup_deadline = Instant::now() + Duration::from_secs(15);
    let mut countdown_presented: Option<mpsc::Receiver<Instant>> = None;
    let mut countdown_deadline = None;
    let mut gate_open = !gated;
    let mut cancelled = false;
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
            cancelled = !gate_open;
            trace.note(
                "stop",
                audio_error
                    .as_deref()
                    .unwrap_or("user request or control channel disconnected"),
            );
            if let Some(audio) = &audio {
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
        if stopping.is_none() && !gate_open {
            let now = Instant::now();
            if let Some(deadline) = countdown_deadline {
                if now >= deadline {
                    gate_open = true;
                    trace.note("countdown.complete", "opened audio and video recording gates");
                    let _ = events.send(Event::CountdownComplete);
                    ctx.request_repaint();
                }
            } else if let Some(presented) = &countdown_presented {
                match presented.try_recv() {
                    Ok(deadline) => {
                        let Some(input) = child.stdin.as_mut() else {
                            break Err("FFmpeg recording control pipe closed".into());
                        };
                        if let Err(error) = schedule_record_gates(
                            input,
                            deadline,
                            [options.system_audio, options.microphone.is_some()],
                            clock_us,
                        ) {
                            break Err(format!("FFmpeg recording gate: {error}"));
                        }
                        countdown_deadline = Some(deadline);
                        trace.note("countdown.shown", "live preview ready before countdown");
                    }
                    Err(mpsc::TryRecvError::Empty) if now < warmup_deadline => {}
                    Err(_) => break Err(tr("无法显示录屏倒计时，请重试。").into()),
                }
            } else if pip.as_ref().is_some_and(pip::Output::has_frame) {
                let (ready, presented) = mpsc::channel();
                let _ = events.send(Event::Countdown(ready));
                ctx.request_repaint();
                countdown_presented = Some(presented);
                warmup_deadline = now + Duration::from_secs(5);
                trace.note("video.warmup", "first captured video frame received");
            } else if now >= warmup_deadline {
                break Err(tr(if options.camera.source == camera::Source::Screen {
                    "录屏任务意外中断"
                } else {
                    "摄像头预览已中断"
                }).into());
            }
        }
        if stopping.is_none() && last_position_update.elapsed() >= Duration::from_millis(220)
            && let Some(position) = pip.as_ref().and_then(pip::Output::position)
        {
            if position != last_position && let Some(input) = &mut child.stdin {
                last_position_update = Instant::now();
                if pip::send_position(input, position).is_ok() { last_position = position; }
            }
        }
        if stopping.is_none() {
            let desired = *mute.lock().unwrap();
            if let Some(input) = &mut child.stdin {
                for (index, name) in ["record_system", "record_mic"].into_iter().enumerate() {
                    let captured = if index == 0 { options.system_audio } else { options.microphone.is_some() };
                    if captured && desired[index] != applied_mute[index]
                        && writeln!(input, "cvolume@{name} -1 volume {}", u8::from(!desired[index])).and_then(|_| input.flush()).is_ok() {
                        applied_mute[index] = desired[index];
                    }
                }
            }
        }
        if stopping.is_some_and(|time| time.elapsed() > Duration::from_secs(20)) {
            trace.note("stop.timeout", "force terminate after 20 seconds");
            let _ = child.kill();
            let _ = child.wait();
            break Err(tr("录屏收尾超时，文件可能不完整。").into());
        }
        std::thread::sleep(Duration::from_millis(40));
    };
    if outcome.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
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
    if cancelled {
        return Err(tr("录屏已取消").into());
    }
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
            camera: Default::default(),
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
            capture_diagnostics: false,
            microphone: None,
            muted: [false; 2],
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

    #[test]
    fn pip_capture_and_mix_are_scheduled_independently() {
        let options = Options {
            camera: camera::Settings { source: camera::Source::Combined, ..Default::default() },
            area: Area { x: 0, y: 0, width: 1920, height: 1080 },
            fps: 30,
            crf: 18,
            mp4: true,
            mouse: true,
            system_audio: false,
            capture_diagnostics: true,
            microphone: Some("mic".into()),
            muted: [false; 2],
            path: PathBuf::from("test.mp4"),
        };
        let args = options.args_with_audio(None, true, VideoEncoder::NvencPlanar, Some(123));
        let graphs: Vec<_> = args.windows(2).filter(|pair| pair[0] == "-filter_complex")
            .map(|pair| pair[1].as_str()).collect();
        assert_eq!(graphs.len(), 4);
        assert!(graphs[0].starts_with("[0:v]") && graphs[0].ends_with("[desktop_warm]"));
        assert!(graphs[1].starts_with("[1:v]") && graphs[1].ends_with("[camera_warm]"));
        assert!(graphs[2].contains("overlay@record_pip") && graphs[2].ends_with("[record]"));
        assert!(graphs[3].starts_with("[1:a:0]") && graphs[3].contains("[mic_warm]"));
        assert!(args.windows(2).any(|pair| pair == ["-delay", "0"]));
    }
}
