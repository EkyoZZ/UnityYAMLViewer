//! Reuse a verified yt-dlp installation without launching its bundled runtime.
use std::{path::{Path,PathBuf},sync::{Mutex,OnceLock}};

#[derive(Clone,serde::Serialize,serde::Deserialize)]
struct Record { path:PathBuf, selected:Option<PathBuf>, length:u64, modified:(u64,u32), version:String }

fn cache_path()->Option<PathBuf> {
    crate::app_cache::root()?.parent().map(|path|path.join("UnityYAMLViewer-components/yt_dlp_detection.json"))
}
fn cache()->&'static Mutex<Option<Record>> {
    static CACHE:OnceLock<Mutex<Option<Record>>>=OnceLock::new();
    CACHE.get_or_init(||Mutex::new(cache_path().and_then(|path|std::fs::read(path).ok()).and_then(|bytes|serde_json::from_slice(&bytes).ok())))
}
fn fingerprint(path:&Path)->Option<(u64,(u64,u32))> {
    let metadata=std::fs::metadata(path).ok()?;
    if !metadata.is_file(){return None;}
    let modified=metadata.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some((metadata.len(),(modified.as_secs(),modified.subsec_nanos())))
}
fn valid(record:&Record)->bool {
    fingerprint(&record.path)==Some((record.length,record.modified))
        && crate::component_paths::yt_dlp_version_key(&record.version).is_some()
}
pub(crate) fn lookup(selected:Option<&Path>)->Option<PathBuf> {
    let record=cache().lock().unwrap().clone()?;
    (record.selected.as_deref()==selected && valid(&record)).then_some(record.path)
}
pub(crate) fn version(path:&Path)->Option<String> {
    let record=cache().lock().unwrap().clone()?;
    (record.path==path && valid(&record)).then_some(record.version)
}
pub(crate) fn remember(path:&Path,version:&str) {
    let Some((length,modified))=fingerprint(path)else{return};
    let record=Record {path:path.to_owned(),selected:crate::component_paths::selected("yt-dlp"),length,modified,version:version.to_owned()};
    *cache().lock().unwrap()=Some(record.clone());
    if let Some(path)=cache_path() {
        if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}
        if let Ok(bytes)=serde_json::to_vec(&record){let _=std::fs::write(path,bytes);}
    }
}
pub(crate) fn invalidate() {
    *cache().lock().unwrap()=None;
    if let Some(path)=cache_path(){let _=std::fs::remove_file(path);}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verified_record_expires_when_file_changes_or_disappears() {
        let path=std::env::temp_dir().join(format!("unity-yt-detection-{}-{}.tmp",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&path,b"fixture").unwrap();
        let (length,modified)=fingerprint(&path).unwrap();
        let record=Record {path:path.clone(),selected:None,length,modified,version:"2026.10.26".into()};
        assert!(valid(&record));
        std::fs::write(&path,b"changed fixture").unwrap();assert!(!valid(&record));
        std::fs::remove_file(&path).unwrap();assert!(!valid(&record));
    }
}
