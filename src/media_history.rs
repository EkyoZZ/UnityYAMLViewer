//! Bounded playback-position records, keyed by canonical file path and file identity.
use std::{collections::BTreeMap, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::{SystemTime, UNIX_EPOCH}};
use serde::{Deserialize, Serialize};

const LIMIT: usize = 500;
const MAX_BYTES: u64 = 2 * 1024 * 1024;

fn error_slot() -> &'static Mutex<Option<String>> {
    static ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    ERROR.get_or_init(|| Mutex::new(None))
}
pub fn report_error(error: &str) { *error_slot().lock().unwrap() = Some(error.to_owned()); }
pub fn take_error() -> Option<String> { error_slot().lock().unwrap().take() }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity { length: u64, modified: Option<(u64, u32)> }
impl Identity {
    pub fn read(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() { return None; }
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()
            .map(|time| (time.as_secs(), time.subsec_nanos()));
        Some(Self { length: metadata.len(), modified })
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Record { identity: Identity, position: f64, updated: u64 }

#[derive(Default)]
struct History { records: BTreeMap<String, Record>, dirty: bool }

fn config_path() -> Option<PathBuf> {
    crate::last_open_dir_config_path().map(|path| path.with_file_name("media_positions.json"))
}

fn history() -> &'static Mutex<History> {
    static HISTORY: OnceLock<Mutex<History>> = OnceLock::new();
    HISTORY.get_or_init(|| {
        let mut records: BTreeMap<String, Record> = config_path().and_then(|path| {
            if std::fs::metadata(&path).ok()?.len() > MAX_BYTES { return None; }
            std::fs::read(path).ok()
        })
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        records.retain(|_, record| record.position.is_finite() && record.position >= 0.0);
        trim(&mut records);
        Mutex::new(History { records, dirty: false })
    })
}

fn trim(records: &mut BTreeMap<String, Record>) {
    while records.len() > LIMIT {
        let oldest = records.iter().min_by_key(|(_, record)| record.updated).map(|(key, _)| key.clone());
        if let Some(key) = oldest { records.remove(&key); }
    }
}

fn key(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let value = canonical.to_string_lossy().into_owned();
    if cfg!(windows) { value.to_lowercase() } else { value }
}

pub fn load(path: &Path, identity: &Identity) -> f64 {
    if crate::remote_file::is_cached(path) { return 0.0; }
    history().lock().unwrap().records.get(&key(path))
        .filter(|record| &record.identity == identity).map_or(0.0, |record| record.position)
}

/// Unknown durations may still be seekable. Completed or invalid records restart at zero.
pub fn restore_position(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || position < 0.0 { return 0.0; }
    if duration.is_finite() && duration > 0.0 && position >= duration { return 0.0; }
    position
}

pub fn save(path: &Path, identity: &Identity, position: f64) -> Result<(), String> {
    if crate::remote_file::is_cached(path) || !position.is_finite() || position < 0.0 { return Ok(()); }
    // Do not apply an old playback clock to a file that was replaced while it was open.
    if Identity::read(path).as_ref() != Some(identity) { return Ok(()); }
    let Some(config) = config_path() else { return Err("APPDATA is unavailable".into()); };
    let mut history = history().lock().unwrap();
    if !history.dirty && history.records.get(&key(path)).is_some_and(|record|
        &record.identity == identity && (record.position - position).abs() < 0.01) { return Ok(()); }
    history.records.insert(key(path), Record {
        identity: identity.clone(), position,
        updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
    });
    history.dirty = true;
    trim(&mut history.records);
    let bytes = loop {
        let bytes = serde_json::to_vec_pretty(&history.records).map_err(|error| error.to_string())?;
        if bytes.len() as u64 <= MAX_BYTES { break bytes; }
        let oldest = history.records.iter().min_by_key(|(_, record)| record.updated).map(|(key, _)| key.clone());
        if let Some(key) = oldest { history.records.remove(&key); } else { break bytes; }
    };
    if let Some(parent) = config.parent() { std::fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
    // Same-directory replacement preserves the previous record file on write failures.
    let temporary = config.with_extension("json.tmp");
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, &config)
    })();
    if result.is_err() { let _ = std::fs::remove_file(&temporary); }
    if result.is_ok() { history.dirty = false; }
    result.map_err(|error| error.to_string())
}

pub fn clear() -> Result<(), String> {
    let mut history = history().lock().unwrap();
    if let Some(path) = config_path() {
        for path in [path.with_extension("json.tmp"), path] {
            match std::fs::remove_file(path) {
                Ok(()) => {},
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Err(error) => return Err(error.to_string()),
            }
        }
    }
    history.records.clear();
    history.dirty = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_completed_and_unknown_duration_positions() {
        assert_eq!(restore_position(42.5, 100.0), 42.5);
        assert_eq!(restore_position(100.0, 100.0), 0.0);
        assert_eq!(restore_position(101.0, 100.0), 0.0);
        assert_eq!(restore_position(-1.0, 100.0), 0.0);
        assert_eq!(restore_position(f64::NAN, 100.0), 0.0);
        assert_eq!(restore_position(f64::INFINITY, 100.0), 0.0);
        assert_eq!(restore_position(42.5, 0.0), 42.5);
    }
    #[test]
    fn records_round_trip_unicode_paths_and_evict_oldest() {
        let identity = Identity { length: 100, modified: Some((123, 456)) };
        let mut records = BTreeMap::new();
        for index in 0..=LIMIT {
            records.insert(format!("C:/影片/剧集 {index}.mkv"), Record { identity: identity.clone(), position: 12.75, updated: index as u64 });
        }
        trim(&mut records);
        assert_eq!(records.len(), LIMIT);
        assert!(!records.contains_key("C:/影片/剧集 0.mkv"));
        let bytes = serde_json::to_vec(&records).unwrap();
        let restored: BTreeMap<String, Record> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored["C:/影片/剧集 1.mkv"].position, 12.75);
        assert_eq!(restored["C:/影片/剧集 1.mkv"].identity, identity);
        assert_ne!(restored["C:/影片/剧集 1.mkv"].identity, Identity { length: 101, modified: Some((123, 456)) });
    }
}
