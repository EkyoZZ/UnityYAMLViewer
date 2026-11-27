use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct Trace {
    id: u128,
    started: std::time::Instant,
    write_error: Arc<Mutex<Option<String>>>,
}
impl Trace {
    pub fn new() -> Self {
        Self {
            id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            started: std::time::Instant::now(),
            write_error: Arc::new(Mutex::new(None)),
        }
    }
    pub fn note(&self, phase: &str, message: impl std::fmt::Display) {
        let module = format!(
            "screen_record/{} {phase} +{:.3}s",
            self.id,
            self.started.elapsed().as_secs_f64()
        );
        let level = if phase == "FAILED" || phase.ends_with(".failed") {
            "ERROR"
        } else {
            "INFO"
        };
        if let Err(error) = crate::error_log::write(level, &module, &message.to_string()) {
            *self.write_error.lock().unwrap_or_else(|p| p.into_inner()) = Some(error.to_string());
        }
    }
    pub fn finish(
        &self,
        result: Result<std::path::PathBuf, String>,
    ) -> Result<std::path::PathBuf, String> {
        match result {
            Ok(path) => {
                self.note("complete", path.display());
                Ok(path)
            }
            Err(error) => {
                self.note("FAILED", &error);
                if let Some(log_error) = self
                    .write_error
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .as_ref()
                {
                    Err(format!(
                        "{error}\n{}: {log_error}",
                        crate::localization::tr("写入错误日志失败")
                    ))
                } else if let Some(path) = crate::error_log::path() {
                    Err(format!("{error}\nerror.log: {}", path.display()))
                } else {
                    Err(error)
                }
            }
        }
    }
}

/// Local sidecars for one opt-in recording. The main muxer keeps its normal maps.
pub(super) struct Capture {
    folder: std::path::PathBuf,
    audio: Vec<(String, &'static str)>,
    pub timestamps: std::fs::File,
}

impl Capture {
    pub fn open(options: &super::Options, use_warm_audio: bool) -> std::io::Result<Self> {
        use std::io::Write;
        let folder = options.path.with_extension("diagnostics");
        std::fs::create_dir(&folder)?;
        let mut timestamps = std::fs::File::create(folder.join("timestamps.log"))?;
        let screen = options.camera.source != super::camera::Source::Camera;
        let camera = options.camera.source != super::camera::Source::Screen;
        let system_index = usize::from(screen);
        let device_index = system_index + usize::from(options.system_audio);
        let mut audio = Vec::new();
        // PiP's warm branches contain the untouched input samples with the same
        // clock as the video. Mapping the raw device again can stall screen reads
        // while FFmpeg tries to catch that output up from the device's zero point.
        let source = |index: usize, warm: &str| {
            if use_warm_audio { warm.to_owned() } else { format!("{index}:a:0") }
        };
        writeln!(timestamps, "# Seconds on the left: elapsed since recording request when FFmpeg log was read.")?;
        writeln!(timestamps, "# pkt_pts_time: incoming stream timestamps; see demuxer+ffmpeg for normalized timestamps.")?;
        writeln!(timestamps, "# WAV files contain pre-mix input samples, including warmup. They start at their first sample.")?;
        writeln!(timestamps, "# FFmpeg bench summary: utime + stime = process CPU seconds across all threads; rtime = elapsed seconds.")?;
        if screen {
            writeln!(timestamps, "# ist_index:0:0 desktop video")?;
        }
        if options.system_audio {
            audio.push((source(system_index, "[sys_warm]"), "system.wav"));
            writeln!(timestamps, "# ist_index:{system_index}:0 system audio -> system.wav")?;
        }
        if camera {
            writeln!(timestamps, "# ist_index:{device_index}:0 camera video")?;
        }
        if options.microphone.is_some() {
            audio.push((source(device_index, "[mic_warm]"), "microphone.wav"));
            let stream = usize::from(camera);
            writeln!(timestamps, "# ist_index:{device_index}:{stream} microphone -> microphone.wav")?;
        }
        Ok(Self { folder, audio, timestamps })
    }

    pub fn add_outputs(&self, encoder: &mut std::process::Command) {
        for (source, name) in &self.audio {
            encoder
                .args(["-map", source.as_str(), "-c:a", "pcm_f32le", "-f", "wav"])
                .arg(self.folder.join(name));
        }
    }
}

pub(super) fn read_timestamps(
    stderr: impl std::io::Read,
    file: std::fs::File,
    trace: &Trace,
) -> String {
    use std::io::{BufRead, BufReader, BufWriter, Write};

    let mut output = BufWriter::new(file);
    let mut write_failed = false;
    let mut tail = String::new();
    for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
        let line = String::from_utf8_lossy(&line);
        if (line.contains("demuxer ->") || line.contains("demuxer+ffmpeg ->")
            || line.contains("Command reply") || line.contains("muxer <-") || line.starts_with("bench: utime="))
            && !write_failed
            && let Err(error) = writeln!(output, "{:.6}\t{}", trace.started.elapsed().as_secs_f64(), line)
        {
            write_failed = true;
            trace.note("diagnostics.failed", error);
        }
        if !line.contains(" -> ") && !line.contains(" <- ") {
            tail.push_str(&line);
            tail.push('\n');
            if tail.len() > 16384 {
                let start = tail.ceil_char_boundary(tail.len() - 16384);
                tail.drain(..start);
            }
        }
    }
    if let Err(error) = output.flush() {
        trace.note("diagnostics.failed", error);
    }
    tail
}
