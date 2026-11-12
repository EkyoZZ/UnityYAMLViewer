//! Append-only recognition evidence; final cues are also written as standard SRT.
use crate::live_subtitle::Event;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LIMIT: usize = 64 * 1024 * 1024;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Cue {
    #[serde(default)]
    pub language:Option<String>,
    pub start: f64,
    pub end: f64,
    pub text: String,
}
impl Cue {
    pub fn overlaps(&self, start: f64, end: f64) -> bool {
        self.start < end && self.end > start
    }
    pub fn contains(&self, start: f64, end: f64) -> bool {
        self.start <= start && self.end >= end
    }
}
pub(crate) fn key(
    source: &Path,
    track: u64,
    engine: &str,
    language: &str,
    identity: Option<&str>,
) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                source.to_string_lossy(),
                track,
                engine,
                language,
                identity,
                "sherpa-onnx==2.0.1/segmentation-v2",
                if engine == "sensevoice" {
                    "2365baeacb507f821a0c8120fcee3d484dba7a07"
                } else {
                    "98590b7ed6443e77b714204da2757d75e1a642f4"
                }
            ))
            .unwrap()
        )
    )
}
/// Only local files with a stable identity, or finite HLS media playlists, are reusable.
pub(crate) fn identity(
    source: &Path,
    input: Option<&crate::network_video::Input>,
    duration: f64,
) -> Option<String> {
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    if let Some(local) = crate::media_history::Identity::read(source) {
        if crate::remote_file::is_hls(source) {
            if std::fs::metadata(source).ok()?.len() > 1024 * 1024 {
                return None;
            }
            let bytes = std::fs::read(source).ok()?;
            if !String::from_utf8_lossy(&bytes)
                .lines()
                .any(|l| l.trim() == "#EXT-X-ENDLIST")
            {
                return None;
            }
            return Some(format!("{:x}", Sha256::digest(bytes)));
        }
        return serde_json::to_string(&local).ok();
    }
    let input = input?;
    if !input.hls || !input.cookies.is_empty() {
        return None;
    }
    let url = crate::network_video::playback_url(source)?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    let mut request = client.get(url);
    for line in input.headers.lines().filter(|l| !l.trim().is_empty()) {
        let (name, value) = line.split_once(':')?;
        request = request.header(name.trim(), value.trim());
    }
    let response = request.send().ok()?.error_for_status().ok()?;
    let mut bytes = Vec::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    let text = std::str::from_utf8(&bytes).ok()?;
    if !text.trim_start().starts_with("#EXTM3U")
        || !text.lines().any(|l| l.trim() == "#EXT-X-ENDLIST")
    {
        return None;
    }
    Some(format!("{:x}", Sha256::digest(bytes)))
}
#[derive(Default)]
pub(crate) struct Replay {
    pub cues: Vec<Cue>,
    pub paths: Option<Paths>,
}
pub(crate) fn load(key: &str) -> Replay {
    let Some(root) = crate::app_cache::root().map(|r| r.join("live_subtitles").join(key)) else {
        return Replay::default();
    };
    let mut directories: Vec<_> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    directories.sort();
    directories.reverse();
    let mut cues = Vec::new();
    let mut paths = None;
    let mut budget = LIMIT as u64;
    for directory in directories.into_iter().take(128) {
        let Ok(file) = File::open(directory.join("recognition.jsonl")) else {
            continue;
        };
        if budget == 0 {
            break;
        }
        let size = file.metadata().map_or(0, |m| m.len());
        if size > budget {
            continue;
        }
        budget -= size;
        if paths.is_none() {
            let srt = std::fs::read_dir(&directory)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|x| x == "srt"));
            if let Some(srt) = srt {
                paths = Some(Paths {
                    json: directory.join("recognition.jsonl"),
                    srt,
                });
            }
        }
        let mut reader = BufReader::new(file);
        loop {
            let mut line = String::new();
            if !matches!(reader.by_ref().take(65537).read_line(&mut line),Ok(n) if n>0 && n<=65536)
            {
                break;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if value["kind"] != "recognition" {
                continue;
            }
            if value["eligible_for_reuse"] == false {
                continue;
            }
            let Ok(event) = serde_json::from_value::<Event>(value["event"].clone()) else {
                continue;
            };
            if cue(1, &event).is_none() {
                continue;
            }
            // Newer sessions win for overlapping intervals; drafts are never reused.
            if cues
                .iter()
                .any(|c: &Cue| c.start < event.end && c.end > event.start)
            {
                continue;
            }
            cues.push(Cue {
                language:event.language,
                start: event.start,
                end: event.end,
                text: event.text,
            });
            if cues.len() >= 10000 {
                return Replay { cues, paths };
            }
        }
    }
    cues.sort_by(|a, b| a.start.total_cmp(&b.start));
    Replay { cues, paths }
}
#[derive(Clone)]
pub(crate) struct Paths {
    pub json: PathBuf,
    pub srt: PathBuf,
}
struct Protection(PathBuf);
impl Drop for Protection {
    fn drop(&mut self) {
        crate::app_cache::forget_task_cache(&self.0);
    }
}
pub(crate) struct Cache {
    json: File,
    srt: File,
    pub paths: Paths,
    bytes: usize,
    count: usize,
    last: Option<(f64, f64, String)>,
    pub reused: Arc<Vec<Cue>>,
    _protection: Protection,
}
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn stamp(seconds: f64) -> String {
    let ms = (seconds.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3600000,
        ms / 60000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
fn complete_srt(bytes: &[u8]) -> Result<String, String> {
    let end = bytes
        .windows(2)
        .rposition(|w| w == b"\n\n")
        .map_or(0, |i| i + 2);
    String::from_utf8(bytes[..end].to_vec()).map_err(|e| e.to_string())
}
pub(crate) fn snapshot_srt(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err(crate::localization::tr("实时字幕缓存已达到单次会话上限。").into());
    }
    complete_srt(&bytes)
}
fn cue(number: usize, event: &Event) -> Option<String> {
    if !event.final_result
        || event.text.trim().is_empty()
        || !event.start.is_finite()
        || !event.end.is_finite()
        || event.start < 0.0
        || event.end < event.start
    {
        return None;
    }
    let start = (event.start * 1000.0).round() / 1000.0;
    let end = event.end.max(start + 0.001);
    Some(format!(
        "{number}\n{} --> {}\n{}\n\n",
        stamp(start),
        stamp(end),
        event.text.trim()
    ))
}
impl Cache {
    pub(crate) fn create(
        source: &Path,
        track: u64,
        engine: &str,
        language: &str,
        identity: Option<&str>,
        ahead: bool,
    ) -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = crate::app_cache::root()
            .ok_or("Subtitle cache directory unavailable")?
            .join("live_subtitles")
            .join(key(source, track, engine, language, identity));
        let directory = root.join(format!(
            "{}-{}-{}",
            now(),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::app_cache::create_task_cache(&directory).map_err(|e| e.to_string())?;
        let protection = Protection(directory.clone());
        let paths = Paths {
            json: directory.join("recognition.jsonl"),
            srt: directory.join(format!("live.{engine}.{language}.srt")),
        };
        let open = |path: &Path| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())
        };
        let json = open(&paths.json)?;
        let srt = open(&paths.srt)?;
        let canonical = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_owned());
        let source_key = format!(
            "{:x}",
            Sha256::digest(canonical.to_string_lossy().as_bytes())
        );
        let file_identity = crate::media_history::Identity::read(source);
        let metadata = serde_json::json!({"kind":"session","schema":1,"created_unix_ms":now(),"source_key":source_key,
            "local_path":file_identity.as_ref().map(|_|canonical.to_string_lossy()),"file_identity":file_identity,"audio_stream_index":track,
            "engine":engine,"language":language,"runtime":"sherpa-onnx==2.0.1/segmentation-v2","model":if engine=="sensevoice" {"sensevoice-int8-2024-07-17"}else{"zipformer-zh-en-98590b7ed6443e77b714204da2757d75e1a642f4"},
            "time_base":"media_seconds","coverage":if ahead {"lookahead_audio"}else{"played_audio_only"},"automatic_correction":false,"reuse_identity":identity});
        let mut cache = Self {
            json,
            srt,
            paths,
            bytes: 0,
            count: 0,
            last: None,
            reused: Arc::new(Vec::new()),
            _protection: protection,
        };
        cache.append(&metadata)?;
        cache.json.sync_data().map_err(|e| e.to_string())?;
        Ok(cache)
    }
    fn append(&mut self, value: &serde_json::Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if self.bytes + bytes.len() > LIMIT {
            return Err(crate::localization::tr("实时字幕缓存已达到单次会话上限。").into());
        }
        self.json.write_all(&bytes).map_err(|e| e.to_string())?;
        self.bytes += bytes.len();
        Ok(())
    }
    pub(crate) fn record(&mut self, event: &Event) -> Result<(), String> {
        let eligible = event.final_result
            && !self
                .reused
                .iter()
                .any(|c| c.overlaps(event.start, event.end));
        self.append(
            &serde_json::json!({"kind":"recognition","received_unix_ms":now(),"event":event,"eligible_for_reuse":eligible}),
        )?;
        if let Some(text) = cue(self.count + 1, event) {
            let key = (event.start, event.end, event.text.clone());
            if self.last.as_ref() == Some(&key) {
                return Ok(());
            }
            if self.bytes + text.len() > LIMIT {
                return Err(crate::localization::tr("实时字幕缓存已达到单次会话上限。").into());
            }
            self.srt
                .write_all(text.as_bytes())
                .and_then(|_| self.srt.sync_data())
                .map_err(|e| e.to_string())?;
            self.json.sync_data().map_err(|e| e.to_string())?;
            self.bytes += text.len();
            self.count += 1;
            self.last = Some(key);
        }
        Ok(())
    }
    pub(crate) fn finish(&mut self, completed: bool) -> Result<(), String> {
        self.append(&serde_json::json!({"kind":"closed","completed":completed,"final_cues":self.count,"closed_unix_ms":now()}))?;
        self.json.sync_data().map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_exclude_in_progress_cues_and_keys_separate_sources() {
        assert_eq!(
            complete_srt(b"1\n00:00:01,000 --> 00:00:02,000\none\n\n2\npartial\xe4").unwrap(),
            "1\n00:00:01,000 --> 00:00:02,000\none\n\n"
        );
        let p = Path::new("video.mp4");
        let a = key(p, 0, "sensevoice", "ja", Some("first"));
        assert_ne!(a, key(p, 1, "sensevoice", "ja", Some("first")));
        assert_ne!(a, key(p, 0, "sensevoice", "ko", Some("first")));
        assert_ne!(a, key(p, 0, "sensevoice", "ja", Some("changed")));
        assert_ne!(a, key(p, 0, "zipformer", "ja", Some("first")));
    }
    #[test]
    fn only_final_cues_use_media_time_and_positive_duration() {
        let mut event = Event {
            start: 62.345,
            end: 63.5,
            text: "日语字幕".into(),
            ..Default::default()
        };
        assert!(cue(1, &event).is_none());
        event.final_result = true;
        assert_eq!(
            cue(1, &event).unwrap(),
            "1\n00:01:02,345 --> 00:01:03,500\n日语字幕\n\n"
        );
        event.end = event.start;
        assert!(cue(2, &event).unwrap().contains("00:01:02,346"));
        event.start = f64::NAN;
        assert!(cue(2, &event).is_none());
    }
}
