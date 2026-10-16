//! User-triggered cache accounting and cleanup. Settings and source files are preserved.
use std::{path::{Path, PathBuf}, sync::mpsc, time::{Duration, Instant}};
use eframe::egui;
use crate::localization::tr;

pub fn mutation_guard() -> std::sync::MutexGuard<'static, ()> {
    static DISK_CACHE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    DISK_CACHE.lock().unwrap()
}

pub fn root() -> Option<PathBuf> { crate::last_open_dir_config_path()?.parent().map(Path::to_owned) }
pub fn network_root() -> PathBuf {
    root().map_or_else(|| std::env::temp_dir().join("UnityYAMLViewer-network"), |path| path.join("network_cache"))
}

fn legacy_update(name: &str) -> bool {
    let Some(suffix) = name.strip_prefix("UnityYAMLViewer-update-") else { return false; };
    let parts: Vec<_> = suffix.split('-').collect();
    parts.len() == 2 && parts.iter().all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()))
}

fn roots() -> Vec<PathBuf> {
    let mut paths: Vec<_> = root().into_iter().collect();
    let temporary = std::env::temp_dir();
    paths.push(temporary.join("UnityYAMLViewer-network"));
    if let Ok(entries) = std::fs::read_dir(&temporary) {
        for entry in entries.flatten() {
            if legacy_update(&entry.file_name().to_string_lossy()) { paths.push(entry.path()); }
        }
    }
    paths.sort(); paths.dedup(); paths
}

fn reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))] { metadata.file_type().is_symlink() }
}

fn comparable(path: &Path) -> PathBuf {
    if cfg!(windows) { PathBuf::from(path.as_os_str().to_string_lossy().to_lowercase()) }
    else { path.to_owned() }
}
fn protected(path: &Path, in_use: &[PathBuf]) -> bool {
    let normalized = comparable(&std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned()));
    in_use.iter().any(|used| {
        let used = comparable(&std::fs::canonicalize(used).unwrap_or_else(|_| used.clone()));
        normalized == used || normalized.starts_with(used)
    })
}

fn size(path: &Path, errors: &mut Vec<String>) -> u64 {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(error) => { errors.push(format!("{}: {error}", path.display())); return 0; }
    };
    // Junctions and symbolic links may point outside the application directory.
    if reparse(&metadata) { errors.push(path.display().to_string()); return 0; }
    if metadata.is_file() { return metadata.len(); }
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => { errors.push(format!("{}: {error}", path.display())); return 0; }
    };
    entries.filter_map(|entry| match entry {
        Ok(entry) => Some(size(&entry.path(), errors)),
        Err(error) => { errors.push(error.to_string()); None }
    }).fold(0, u64::saturating_add)
}

// Validate every deletion against the resolved application-owned root; never follow reparse points.
fn clean_tree(path: &Path, boundary: &Path, in_use: &[PathBuf], errors: &mut Vec<String>) {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => { errors.push(format!("{}: {error}", path.display())); return; }
    };
    if reparse(&metadata) { errors.push(path.display().to_string()); return; }
    let resolved = match std::fs::canonicalize(path) {
        Ok(path) if comparable(&path).starts_with(comparable(boundary)) => path,
        _ => { errors.push(path.display().to_string()); return; }
    };
    if protected(&resolved, in_use) || (metadata.is_file() && crate::playlist::is_playlist(path)) { return; }
    // Leave a failed update's recovery files and a recently started installer intact.
    if metadata.is_dir() && legacy_update(&path.file_name().unwrap_or_default().to_string_lossy()) {
        let installer = path.join("install.ps1");
        let active = std::fs::metadata(&installer).and_then(|metadata| metadata.modified()).ok()
            .and_then(|modified| modified.elapsed().ok()).is_some_and(|age| age < Duration::from_secs(300));
        if path.join("error.txt").exists() || active { return; }
    }
    let result = if metadata.is_dir() {
        match std::fs::read_dir(path) {
            Ok(entries) => for entry in entries {
                match entry {
                    Ok(entry) => clean_tree(&entry.path(), boundary, in_use, errors),
                    Err(error) => errors.push(error.to_string()),
                }
            },
            Err(error) => { errors.push(error.to_string()); return; }
        }
        // Leave nonempty directories that contain protected/locked files.
        if std::fs::read_dir(path).ok().is_some_and(|mut entries| entries.next().is_none()) {
            std::fs::remove_dir(path)
        } else { Ok(()) }
    } else { std::fs::remove_file(path) };
    if let Err(error) = result { errors.push(format!("{}: {error}", path.display())); }
}

#[derive(Default)]
struct Snapshot { locations: Vec<(PathBuf, u64)>, errors: Vec<String>, released: Option<u64> }
impl Snapshot { fn bytes(&self) -> u64 { self.locations.iter().map(|(_, bytes)| *bytes).fold(0, u64::saturating_add) } }

fn measure() -> Snapshot {
    let mut snapshot = Snapshot::default();
    for path in roots() {
        let bytes = size(&path, &mut snapshot.errors);
        if bytes > 0 || root().as_ref() == Some(&path) { snapshot.locations.push((path, bytes)); }
    }
    snapshot
}

fn cleanup(in_use: &[PathBuf]) -> Snapshot {
    crate::playlist::flush_writes();
    let _cache_guard = mutation_guard();
    let before = measure().bytes();
    let mut errors = Vec::new();
    for path in roots() {
        if !path.is_absolute() { errors.push(path.display().to_string()); continue; }
        if std::fs::symlink_metadata(&path).ok().is_some_and(|metadata| reparse(&metadata)) {
            errors.push(path.display().to_string()); continue;
        }
        let Ok(boundary) = std::fs::canonicalize(&path) else { continue; };
        if root().as_ref() == Some(&path) {
            // Only known disposable locations; configuration, recent files, tabs and unknown files stay.
            for name in ["project_indexes", "network_cache", "updates", "playlist_positions", "image_edit_temp"] {
                clean_tree(&path.join(name), &boundary, in_use, &mut errors);
            }
            if let Err(error) = crate::media_history::clear() { errors.push(error); }
        } else {
            clean_tree(&path, &boundary, in_use, &mut errors);
        }
    }
    let mut after = measure();
    after.released = Some(before.saturating_sub(after.bytes()));
    after.errors.extend(errors);
    after
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

#[derive(Default)]
pub struct Panel {
    snapshot: Option<Snapshot>,
    pending: Option<mpsc::Receiver<Snapshot>>,
    cleaning: bool,
    last_refresh: Option<Instant>,
    open_error: Option<String>,
}
impl Panel {
    fn start(&mut self, clean: bool, in_use: Vec<PathBuf>, ctx: &egui::Context) {
        if self.pending.is_some() { return; }
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive); self.cleaning = clean;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = if clean { cleanup(&in_use) } else { measure() };
            let _ = send.send(result); ctx.request_repaint();
        });
    }
    pub fn show(&mut self, ui: &mut egui::Ui, in_use: Vec<PathBuf>, busy: bool) {
        if let Some(pending) = &self.pending {
            match pending.try_recv() {
                Ok(snapshot) => { self.snapshot = Some(snapshot); self.pending = None; self.cleaning = false; self.last_refresh = Some(Instant::now()); }
                Err(mpsc::TryRecvError::Disconnected) => { self.pending = None; self.cleaning = false; self.open_error = Some(tr("缓存操作中断，请重试。").into()); }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_none() && self.last_refresh.is_none_or(|time| time.elapsed() >= Duration::from_secs(10)) {
            self.start(false, Vec::new(), ui.ctx());
        }
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        if let Some(snapshot) = &self.snapshot { ui.label(crate::lf!("占用空间：{0}", format_size(snapshot.bytes()))); }
        else { ui.label(tr("正在统计缓存…")); }
        ui.horizontal(|ui| {
            if ui.button(tr("查看")).clicked() {
                self.open_error = root().ok_or_else(|| "APPDATA is unavailable".into()).and_then(|path| open_folder(&path)).err();
            }
            if ui.add_enabled(self.pending.is_none() && !busy, egui::Button::new(tr("清理"))).clicked() {
                self.start(true, in_use, ui.ctx());
            }
            if self.pending.is_some() { ui.spinner(); }
        });
        if self.cleaning { ui.label(tr("正在清理缓存…")); }
        if busy { ui.small(tr("下载或索引扫描期间暂不能清理缓存。")); }
        if let Some(snapshot) = &self.snapshot {
            if let Some(bytes) = snapshot.released { ui.label(crate::lf!("已释放：{0}", format_size(bytes))); }
            for (path, bytes) in &snapshot.locations {
                if root().as_ref() != Some(path) {
                    ui.label(crate::lf!("旧缓存：{0}", format_size(*bytes)));
                    if ui.button(tr("查看旧缓存")).on_hover_text(path.display().to_string()).clicked() {
                        self.open_error = open_folder(path).err();
                    }
                }
            }
            if !snapshot.errors.is_empty() {
                ui.colored_label(ui.visuals().warn_fg_color, tr("部分缓存无法统计或清理，可能被占用或为目录链接。"))
                    .on_hover_text(snapshot.errors.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
            }
        }
        if let Some(error) = &self.open_error {
            ui.colored_label(ui.visuals().error_fg_color, crate::lf!("缓存操作失败：{0}", tr(error)));
        }
    }
}

fn open_folder(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| error.to_string())?;
    #[cfg(windows)] { crate::media_tools::command("explorer.exe").arg(path).spawn().map(|_| ()).map_err(|error| error.to_string()) }
    #[cfg(not(windows))] { let _ = path; Err(tr("文件关联设置仅支持 Windows。").into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_cleanup_names_require_exact_numeric_application_prefix() {
        assert!(legacy_update("UnityYAMLViewer-update-123-456"));
        for name in ["UnityYAMLViewer-update-", "UnityYAMLViewer-update-123-", "UnityYAMLViewer-update-123-456-backup", "other-update-123-456", "UnityYAMLViewer-update-../user"] {
            assert!(!legacy_update(name));
        }
    }
    #[test]
    fn readable_sizes_keep_large_and_small_values_distinct() {
        assert_eq!(format_size(12), "12 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }
    #[test]
    fn cleanup_preserves_open_files_and_rejects_outside_paths() {
        let workspace = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        let parent = workspace.join("target").join(format!("cache-cleanup-test-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        assert!(parent.starts_with(&workspace));
        let owned = parent.join("owned");
        std::fs::create_dir_all(owned.join("nested")).unwrap();
        let used = owned.join("nested/playing.mp4");
        let old = owned.join("nested/unused.mp4");
        let outside = parent.join("user.mp4");
        let playlist = owned.join("nested/user.m3u8");
        std::fs::write(&playlist, b"#EXTM3U\nplaying.mp4\n").unwrap();
        for path in [&used, &old, &outside] { std::fs::write(path, b"test data").unwrap(); }
        let boundary = std::fs::canonicalize(&owned).unwrap();
        let mut errors = Vec::new();
        clean_tree(&outside, &boundary, std::slice::from_ref(&used), &mut errors);
        assert!(!errors.is_empty());
        assert!(outside.exists());
        errors.clear();
        clean_tree(&owned, &boundary, std::slice::from_ref(&used), &mut errors);
        assert!(errors.is_empty());
        assert!(used.exists());
        assert!(!old.exists());
        assert!(outside.exists());
        clean_tree(&owned, &boundary, &[], &mut errors);
        assert!(playlist.exists());
        std::fs::remove_file(playlist).unwrap();
        clean_tree(&owned, &boundary, &[], &mut errors);
        assert!(!owned.exists());
        // The test only removes files it created under the resolved workspace target directory.
        std::fs::remove_file(outside).unwrap();
        std::fs::remove_dir(parent).unwrap();
    }
}
