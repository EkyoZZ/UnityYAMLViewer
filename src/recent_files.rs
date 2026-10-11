use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct RecentFiles { paths: Vec<PathBuf> }
impl RecentFiles {
    pub fn load() -> Self {
        let paths = config().and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Vec<PathBuf>>(&bytes).ok()).unwrap_or_default();
        let mut recent = Self::default();
        for path in paths.into_iter().rev() { recent.record(path); }
        recent
    }
    pub fn paths(&self) -> &[PathBuf] { &self.paths }
    pub fn record(&mut self, path: PathBuf) {
        self.paths.retain(|entry| !same_path(entry, &path));
        self.paths.insert(0, path);
        self.paths.truncate(30);
    }
    pub fn persist(&self) {
        let Some(path) = config() else { return; };
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.paths) { let _ = std::fs::write(path, bytes); }
    }
}
fn same_path(first: &Path, second: &Path) -> bool {
    if cfg!(windows) { first.to_string_lossy().eq_ignore_ascii_case(&second.to_string_lossy()) } else { first == second }
}
fn config() -> Option<PathBuf> { crate::last_open_dir_config_path().map(|path| path.with_file_name("recent_files.json")) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_files_deduplicate_move_to_front_and_keep_thirty() {
        let mut recent = RecentFiles::default();
        for index in 0..32 { recent.record(PathBuf::from(format!("file{index}.prefab"))); }
        assert_eq!(recent.paths().len(), 30);
        recent.record(PathBuf::from("file5.prefab"));
        assert_eq!(recent.paths()[0], PathBuf::from("file5.prefab"));
        assert_eq!(recent.paths().len(), 30);
    }
}
