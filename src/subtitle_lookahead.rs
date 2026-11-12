//! Independently decoded VOD audio, bounded by the viewer's media clock.
use crate::{live_subtitle::Tap, network_video::Input};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

const HORIZON: f64 = 20.0;
pub(crate) fn may_feed(timestamp: f64, position: f64) -> bool {
    timestamp < position + HORIZON
}
struct Control {
    stop: AtomicBool,
    position: AtomicU64,
    child: Mutex<Option<Child>>,
}
pub(crate) struct Ahead {
    control: Arc<Control>,
    pub errors: mpsc::Receiver<String>,
}
impl Ahead {
    pub fn position(&self, position: f64) {
        self.control
            .position
            .store(position.to_bits(), Ordering::Relaxed);
    }
    pub fn start(
        ffmpeg: &Path,
        source: &Path,
        input: Option<&Input>,
        track: u64,
        position: f64,
        duration: f64,
        tap: Tap,
    ) -> Self {
        let control = Arc::new(Control {
            stop: AtomicBool::new(false),
            position: AtomicU64::new(position.to_bits()),
            child: Mutex::new(None),
        });
        let (sender, errors) = mpsc::channel();
        let worker = control.clone();
        let ffmpeg = ffmpeg.to_owned();
        let source = source.to_owned();
        let input = input.cloned();
        std::thread::spawn(move || {
            let result = decode(
                &worker,
                &ffmpeg,
                &source,
                input.as_ref(),
                track,
                position,
                duration,
                &tap,
            );
            if let Err(error) = result {
                if !worker.stop.load(Ordering::Relaxed) {
                    let _ = sender.send(error);
                }
            }
            if let Some(mut child) = worker.child.lock().unwrap().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        });
        Self { control, errors }
    }
}
impl Drop for Ahead {
    fn drop(&mut self) {
        self.control.stop.store(true, Ordering::Relaxed);
        let control = self.control.clone();
        std::thread::spawn(move || {
            if let Some(mut child) = control.child.lock().unwrap().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        });
    }
}
fn send(control: &Control, tap: &Tap, packet: Vec<u8>) -> bool {
    let mut packet = packet;
    loop {
        if control.stop.load(Ordering::Relaxed) {
            return false;
        }
        match tap.sender.try_send(packet) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Full(value)) => {
                packet = value;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
        }
    }
}
fn packet(timestamp: f64, samples: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(samples.len() + 12);
    packet.extend(timestamp.to_le_bytes());
    packet.extend((samples.len() as u32 / 4).to_le_bytes());
    packet.extend(samples);
    packet
}
fn decode(
    control: &Control,
    ffmpeg: &PathBuf,
    source: &Path,
    input: Option<&Input>,
    track: u64,
    origin: f64,
    duration: f64,
    tap: &Tap,
) -> Result<(), String> {
    while !tap.ready.load(Ordering::Relaxed) {
        if control.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut command = crate::media_tools::command(ffmpeg);
    command.args(["-nostdin", "-v", "error"]);
    if let Some(input) = input {
        input.options(&mut command);
    }
    command
        .args(["-ss", &format!("{origin:.6}"), "-i"])
        .arg(source)
        .args([
            "-map",
            &format!("0:{track}"),
            "-vn",
            "-t",
            &format!("{:.6}", (duration - origin).max(0.001)),
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut slot = control.child.lock().unwrap();
    if control.stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    *slot = Some(child);
    drop(slot);
    let diagnostics = Arc::new(Mutex::new(Vec::new()));
    let log = diagnostics.clone();
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        while let Ok(n) = stderr.read(&mut buffer) {
            if n == 0 {
                break;
            }
            let mut log = log.lock().unwrap();
            log.extend_from_slice(&buffer[..n]);
            if log.len() > 4096 {
                let count = log.len() - 4096;
                log.drain(..count);
            }
        }
    });
    let mut frames = 0u64;
    let mut buffer = [0u8; 6400];
    loop {
        let timestamp = origin + frames as f64 / 16000.0;
        while !may_feed(
            timestamp,
            f64::from_bits(control.position.load(Ordering::Relaxed)),
        ) {
            if control.stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if control.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut count = 0;
        while count < buffer.len() {
            let n = stdout
                .read(&mut buffer[count..])
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            count += n;
        }
        if count == 0 {
            break;
        }
        if count % 4 != 0 {
            return Err("Truncated lookahead audio sample".into());
        }
        let end = timestamp + count as f64 / 64000.0;
        if !tap.replay.iter().any(|cue| cue.contains(timestamp, end))
            && !send(control, tap, packet(timestamp, &buffer[..count]))
        {
            return Ok(());
        }
        frames += (count / 4) as u64;
    }
    if control.stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    let mut child = control
        .child
        .lock()
        .unwrap()
        .take()
        .ok_or("Lookahead decoder stopped")?;
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "{}: {}",
            crate::localization::tr("提前识别音频解码失败。"),
            String::from_utf8_lossy(&diagnostics.lock().unwrap())
        ));
    }
    send(control, tap, packet(origin + frames as f64 / 16000.0, &[]));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn horizon_tracks_media_clock_and_packet_keeps_source_time() {
        assert!(may_feed(29.9, 10.0));
        assert!(!may_feed(30.0, 10.0));
        assert!(may_feed(30.0, 10.1));
        let value = packet(42.125, &[0; 8]);
        assert_eq!(f64::from_le_bytes(value[..8].try_into().unwrap()), 42.125);
        assert_eq!(u32::from_le_bytes(value[8..12].try_into().unwrap()), 2);
    }
}
