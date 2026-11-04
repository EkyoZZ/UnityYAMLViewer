use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use eframe::egui;
use reqwest::{Url, blocking::Client};

const MAX_ATTEMPTS: u8 = 21; // One initial request and up to twenty retries.
const SEGMENT_LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Cache(Arc<Owner>);

struct Owner(Arc<Shared>);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.stopped.store(true, Ordering::Relaxed);
        self.0.changed.notify_all();
    }
}

struct Segment {
    url: Url,
    end: f64,
}

#[derive(Default)]
struct Status {
    attempts: u8,
    busy: bool,
    ready: bool,
    retry_at: Option<Instant>,
}

struct Shared {
    client: Client,
    segments: Vec<Segment>,
    status: Mutex<Vec<Status>>,
    changed: Condvar,
    stopped: AtomicBool,
    root: PathBuf,
    ctx: egui::Context,
}

impl Drop for Shared {
    fn drop(&mut self) {
        for index in 0..self.segments.len() {
            let _ = fs::remove_file(self.segment_path(index));
            let _ = fs::remove_file(self.root.join(format!("{index}.part")));
        }
        let _ = fs::remove_file(self.root.join("index.m3u8"));
        let _ = fs::remove_dir(&self.root);
        crate::app_cache::forget_task_cache(&self.root);
    }
}

impl Cache {
    pub(crate) fn open(path: &Path, ctx: egui::Context) -> Option<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(45))
            .build()
            .ok()?;
        let url = Url::parse(path.to_str()?).ok()?;
        let (manifest, segments) = resolve_manifest(&client, url)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let root = crate::app_cache::network_root()
            .join(format!("hls-play-{}-{nonce}", std::process::id()));
        crate::app_cache::create_task_cache(&root).ok()?;
        let cache = Self(Arc::new(Owner(Arc::new(Shared {
            client,
            status: Mutex::new(segments.iter().map(|_| Status::default()).collect()),
            segments,
            changed: Condvar::new(),
            stopped: AtomicBool::new(false),
            root,
            ctx,
        }))));
        let shared = &cache.0.0;
        fs::write(cache.manifest_path(), manifest).ok()?;
        let worker = shared.clone();
        thread::spawn(move || {
            for index in 0..worker.segments.len() {
                if worker.stopped() {
                    break;
                }
                worker.fetch_first(index);
            }
        });
        let worker = shared.clone();
        thread::spawn(move || worker.retry());
        Some(cache)
    }

    pub(crate) fn playback_path(&self) -> Option<PathBuf> {
        self.0
            .0
            .status
            .lock()
            .unwrap()
            .iter()
            .all(|entry| entry.ready)
            .then(|| self.manifest_path())
    }

    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.0.0.root.join("index.m3u8")
    }

    pub(crate) fn buffered_end(&self) -> Option<f64> {
        let shared = &self.0.0;
        let index = shared
            .status
            .lock()
            .unwrap()
            .iter()
            .rposition(|entry| entry.ready)?;
        Some(shared.segments[index].end)
    }

    pub(crate) fn progress(&self) -> (u8, bool, bool) {
        let status = self.0.0.status.lock().unwrap();
        let ready = status.iter().filter(|entry| entry.ready).count();
        let exhausted = status
            .iter()
            .any(|entry| !entry.ready && !entry.busy && entry.attempts >= MAX_ATTEMPTS);
        (
            (ready * 100 / status.len()) as u8,
            ready == status.len(),
            exhausted,
        )
    }
}

fn resolve_manifest(client: &Client, mut url: Url) -> Option<(String, Vec<Segment>)> {
    for _ in 0..3 {
        let response = client
            .get(url)
            .timeout(Duration::from_secs(8))
            .send()
            .ok()?
            .error_for_status()
            .ok()?;
        let base = response.url().clone();
        let mut text = String::new();
        response
            .take(2 * 1024 * 1024 + 1)
            .read_to_string(&mut text)
            .ok()?;
        if text.len() > 2 * 1024 * 1024
            || !text.trim_start_matches('\u{feff}').starts_with("#EXTM3U")
        {
            return None;
        }
        let lines: Vec<_> = text
            .trim_start_matches('\u{feff}')
            .lines()
            .map(str::trim)
            .collect();
        if lines
            .iter()
            .any(|line| line.starts_with("#EXT-X-STREAM-INF:"))
        {
            // Preserve FFmpeg's selection for multivariant playlists and external renditions.
            if lines.iter().any(|line| line.starts_with("#EXT-X-MEDIA:")) {
                return None;
            }
            let mut variants = lines
                .windows(2)
                .filter(|pair| pair[0].starts_with("#EXT-X-STREAM-INF:"));
            let variant = variants.next()?;
            if variants.next().is_some() || variant[1].starts_with('#') || variant[1].is_empty() {
                return None;
            }
            url = http_join(&base, variant[1])?;
        } else {
            return parse_media(&base, &lines);
        }
    }
    None
}

fn http_join(base: &Url, path: &str) -> Option<Url> {
    let url = base.join(path).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn parse_media(base: &Url, lines: &[&str]) -> Option<(String, Vec<Segment>)> {
    if !lines.contains(&"#EXT-X-ENDLIST") {
        return None;
    }
    let mut manifest = String::new();
    let mut segments = Vec::new();
    let mut duration = None;
    let mut end = 0.0;
    for &line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("#EXTINF:") {
            let seconds = value.split(',').next()?.parse::<f64>().ok()?;
            if !seconds.is_finite() || seconds <= 0.0 || duration.replace(seconds).is_some() {
                return None;
            }
        } else if line.starts_with('#') {
            if [
                "#EXT-X-KEY",
                "#EXT-X-MAP",
                "#EXT-X-BYTERANGE",
                "#EXT-X-GAP",
                "#EXT-X-PART",
                "#EXT-X-DEFINE",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix))
                || line.contains("URI=")
            {
                return None;
            }
        } else {
            let url = http_join(base, line)?;
            if !url.path().to_ascii_lowercase().ends_with(".ts") {
                return None;
            }
            end += duration.take()?;
            if !end.is_finite() || segments.len() >= 20_000 {
                return None;
            }
            manifest.push_str(&format!("{}.ts\n", segments.len()));
            segments.push(Segment { url, end });
            continue;
        }
        manifest.push_str(line);
        manifest.push('\n');
    }
    (!segments.is_empty() && duration.is_none()).then_some((manifest, segments))
}

impl Shared {
    fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    fn segment_path(&self, index: usize) -> PathBuf {
        self.root.join(format!("{index}.ts"))
    }

    fn fetch_first(&self, index: usize) {
        let mut status = self.status.lock().unwrap();
        if status[index].attempts != 0 || self.stopped() {
            return;
        }
        status[index].attempts = 1;
        status[index].busy = true;
        drop(status);
        self.fetch(index);
    }

    fn fetch(&self, index: usize) {
        let partial = self.root.join(format!("{index}.part"));
        let ready = self.fetch_to(index, &partial).is_ok();
        if !ready {
            let _ = fs::remove_file(&partial);
        }
        let mut status = self.status.lock().unwrap();
        let entry = &mut status[index];
        entry.busy = false;
        entry.ready = ready;
        entry.retry_at = (!ready && entry.attempts < MAX_ATTEMPTS)
            .then(|| Instant::now() + Duration::from_secs(u64::from(entry.attempts).min(10)));
        drop(status);
        self.changed.notify_all();
        if ready {
            self.ctx.request_repaint();
        }
    }

    fn fetch_to(&self, index: usize, partial: &Path) -> io::Result<()> {
        let mut response = self
            .client
            .get(self.segments[index].url.clone())
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(io::Error::other)?;
        let length = response.content_length();
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "Invalid MPEG-TS segment");
        if response.status() != reqwest::StatusCode::OK
            || length.is_some_and(|length| length > SEGMENT_LIMIT || length % 188 != 0)
        {
            return Err(invalid());
        }
        let mut output = BufWriter::new(File::create(partial)?);
        let mut input = BufReader::new(&mut response);
        let mut packet = [0; 188];
        let mut total = 0;
        while !self.stopped() {
            if input.fill_buf()?.is_empty() {
                break;
            }
            input.read_exact(&mut packet)?;
            total += packet.len() as u64;
            if packet[0] != 0x47 || packet[1] & 0x80 != 0 || total > SEGMENT_LIMIT {
                return Err(invalid());
            }
            output.write_all(&packet)?;
        }
        if self.stopped() || total == 0 || length.is_some_and(|length| length != total) {
            return Err(invalid());
        }
        output.flush()?;
        drop(output);
        fs::rename(partial, self.segment_path(index))
    }

    fn retry(&self) {
        let mut status = self.status.lock().unwrap();
        while !self.stopped() {
            let next = status
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| entry.retry_at.map(|at| (at, index)))
                .min();
            if let Some((at, index)) = next {
                let delay = at.saturating_duration_since(Instant::now());
                if delay.is_zero() {
                    let entry = &mut status[index];
                    entry.retry_at = None;
                    entry.attempts += 1;
                    entry.busy = true;
                    drop(status);
                    self.fetch(index);
                    status = self.status.lock().unwrap();
                } else {
                    (status, _) = self.changed.wait_timeout(status, delay).unwrap();
                }
            } else {
                // A timed wait also covers shutdown occurring just before the wait.
                (status, _) = self
                    .changed
                    .wait_timeout(status, Duration::from_millis(250))
                    .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_signed_segments_and_keeps_their_timeline() {
        let base = Url::parse("https://example.com/video/index.m3u8?playlist=token").unwrap();
        let input = "#EXTM3U\n#EXTINF:3.5,\npart.ts?sign=abc\n#EXT-X-DISCONTINUITY\n#EXTINF:2,\n../last.ts\n#EXT-X-ENDLIST";
        let (manifest, segments) = parse_media(&base, &input.lines().collect::<Vec<_>>()).unwrap();
        assert!(manifest.contains("0.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:2,\n1.ts"));
        assert_eq!(
            segments[0].url.as_str(),
            "https://example.com/video/part.ts?sign=abc"
        );
        assert_eq!(segments[1].url.as_str(), "https://example.com/last.ts");
        assert_eq!(segments[1].end, 5.5);

        let encrypted = input.replace(
            "#EXTINF:3.5,",
            "#EXT-X-KEY:METHOD=AES-128,URI=\"key\"\n#EXTINF:3.5,",
        );
        assert!(parse_media(&base, &encrypted.lines().collect::<Vec<_>>()).is_none());
    }
}
