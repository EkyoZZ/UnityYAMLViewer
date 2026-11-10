use std::{fs, io::Write, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

use sha2::{Digest, Sha256};

pub(crate) const MAX_FRAMES: usize = 1_000_000;
const CHECKSUM_BYTES: usize = 32;
const CACHE_LIMIT: u64 = 128 * 1024 * 1024;

pub(crate) fn key(video: &Path, start_time: f64) -> Option<PathBuf> {
    if crate::remote_file::is_cached(video) {
        return None;
    }
    let path = video.canonicalize().ok()?;
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    let mut hash = Sha256::new();
    hash.update(b"video-frame-index-v1");
    hash.update(metadata.len().to_le_bytes());
    hash.update(modified.as_nanos().to_le_bytes());
    hash.update(start_time.to_le_bytes());
    hash.update(path.as_os_str().as_encoded_bytes());
    Some(crate::app_cache::root()?.join("video_frame_indexes").join(format!("{:x}.idx", hash.finalize())))
}

pub(crate) fn load(path: &Path) -> Option<Vec<f64>> {
    use std::io::Read;

    let maximum = MAX_FRAMES * 8 + CHECKSUM_BYTES;
    let mut bytes = Vec::new();
    fs::File::open(path).ok()?.take((maximum + 1) as u64).read_to_end(&mut bytes).ok()?;
    if bytes.len() > maximum || bytes.len() <= CHECKSUM_BYTES {
        return None;
    }
    let (checksum, data) = bytes.split_at(CHECKSUM_BYTES);
    if data.len() % 8 != 0 || &Sha256::digest(data)[..] != checksum {
        return None;
    }
    let times: Vec<_> = data.chunks_exact(8).map(|time| f64::from_le_bytes(time.try_into().unwrap())).collect();
    (times.iter().all(|time| time.is_finite()) && times.is_sorted()).then_some(times)
}

pub(crate) fn save(path: &Path, times: &[f64]) {
    let _guard = crate::app_cache::mutation_guard();
    let Some(directory) = path.parent() else {return;};
    if fs::create_dir_all(directory).is_err() {
        return;
    }
    let bytes: Vec<_> = times.iter().flat_map(|time| time.to_le_bytes()).collect();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let Ok(mut file) = fs::OpenOptions::new().write(true).create_new(true).open(&temporary) else {return;};
    let written = file.write_all(&Sha256::digest(&bytes)).and_then(|_| file.write_all(&bytes));
    drop(file);
    if written.is_ok() && crate::playlist::replace(&temporary, path).is_ok() {
        trim(directory, path);
    }
    let _ = fs::remove_file(temporary);
}

fn trim(directory: &Path, current: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {return;};
    let mut indexes: Vec<_> = entries.flatten().filter_map(|entry| {
        let path = entry.path();
        if path.extension()?.to_str()? != "idx" || !entry.file_type().ok()?.is_file() {
            return None;
        }
        let metadata = entry.metadata().ok()?;
        Some((metadata.modified().ok()?, path, metadata.len()))
    }).collect();
    let mut size = indexes.iter().map(|entry| entry.2).sum::<u64>();
    indexes.sort_unstable();
    for (_, path, length) in indexes {
        if size <= CACHE_LIMIT { break; }
        if path != current && fs::remove_file(path).is_ok() { size -= length; }
    }
}
