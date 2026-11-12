//! Recording metadata only. Removing or trimming entries never touches video files.
use crate::localization::tr;
use eframe::egui::{self, Color32, RichText};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub(super) const MIN_LIMIT: usize = 20;
pub(super) const MAX_LIMIT: usize = 50;
pub(super) const DEFAULT_LIMIT: usize = 20;
pub(super) struct History {
    entries: Vec<PathBuf>,
    limit: usize,
    selected: Option<PathBuf>,
    pub open: bool,
    error: Option<String>,
}
impl History {
    pub fn load(limit: usize) -> Self {
        let result = (|| {
            let path = config()?;
            match std::fs::read(&path) {
                Ok(bytes) => serde_json::from_slice::<Vec<PathBuf>>(&bytes)
                    .map_err(|error| format!("{}: {error}", path.display())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
                Err(error) => Err(format!("{}: {error}", path.display())),
            }
        })();
        let mut history = Self {
            entries: vec![],
            limit: limit.clamp(MIN_LIMIT, MAX_LIMIT),
            selected: None,
            open: false,
            error: None,
        };
        match result {
            Ok(paths) => {
                for path in paths.into_iter().rev() {
                    history.insert(path);
                }
            }
            Err(error) => history.error = Some(error),
        }
        history
    }
    fn insert(&mut self, path: PathBuf) {
        self.entries.retain(|entry| !same_path(entry, &path));
        self.entries.insert(0, path);
        self.entries.truncate(self.limit);
    }
    pub fn record(&mut self, path: PathBuf) {
        self.insert(path.clone());
        self.selected = Some(path);
        self.persist();
    }
    pub fn set_limit(&mut self, limit: usize) {
        let limit = limit.clamp(MIN_LIMIT, MAX_LIMIT);
        if self.limit != limit {
            self.limit = limit;
            self.entries.truncate(limit);
            if self
                .selected
                .as_ref()
                .is_some_and(|path| !self.entries.iter().any(|entry| same_path(path, entry)))
            {
                self.selected = None;
            }
            self.persist();
        }
    }
    fn remove_record(&mut self, path: &Path) {
        self.entries.retain(|entry| !same_path(entry, path));
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| same_path(selected, path))
        {
            self.selected = None;
        }
        self.persist();
    }
    fn persist(&mut self) {
        let result = (|| {
            let path = config()?;
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
            let temporary = path.with_extension(format!(
                "{}-{}.tmp",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            let result = (|| {
                let mut file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temporary)
                    .map_err(|error| error.to_string())?;
                file.write_all(
                    &serde_json::to_vec_pretty(&self.entries).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                file.sync_all().map_err(|error| error.to_string())?;
                drop(file);
                std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(&temporary);
            }
            result
        })();
        self.error = result.err();
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        if !self.open {
            return None;
        }
        let mut open = true;
        let mut edit = None;
        let mut remove = None;
        crate::native_window::Window::new(tr("录屏历史"))
            .id(egui::Id::new("screen_record_history"))
            .open(&mut open)
            .default_width(620.0)
            .default_height(460.0)
            .show(ctx, |ui| {
                ui.small(tr("移除记录不会删除视频文件。"));
                if self.entries.is_empty() {
                    ui.label(tr("暂无录屏记录"));
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for path in &self.entries {
                            let exists = path.is_file();
                            let name = path
                                .file_name()
                                .unwrap_or(path.as_os_str())
                                .to_string_lossy();
                            let text = format!("{name}\n{}", path.display());
                            let text = if exists {
                                RichText::new(text)
                            } else {
                                RichText::new(text).color(ui.visuals().error_fg_color)
                            };
                            if ui
                                .add_sized(
                                    [ui.available_width(), 42.0],
                                    egui::Button::new(text)
                                        .selected(
                                            self.selected
                                                .as_ref()
                                                .is_some_and(|selected| same_path(selected, path)),
                                        )
                                        .wrap(),
                                )
                                .on_hover_text(path.display().to_string())
                                .clicked()
                            {
                                self.selected = Some(path.clone());
                            }
                        }
                    });
                ui.separator();
                if let Some(path) = self.selected.clone() {
                    let exists = path.is_file();
                    if !exists {
                        ui.colored_label(ui.visuals().error_fg_color, tr("文件不存在"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(exists, egui::Button::new(tr("视频编辑")))
                            .clicked()
                        {
                            edit = Some(path.clone());
                        }
                        if ui
                            .add_enabled(exists, egui::Button::new(tr("打开所在文件夹")))
                            .clicked()
                        {
                            let _ = crate::media_tools::command("explorer.exe")
                                .arg("/select,")
                                .arg(&path)
                                .spawn();
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(tr("移除记录")).color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(170, 45, 57)),
                            )
                            .clicked()
                        {
                            remove = Some(path);
                        }
                    });
                }
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            });
        if let Some(path) = remove {
            self.remove_record(&path);
        }
        self.open = open;
        ctx.request_repaint_after(std::time::Duration::from_secs(2));
        edit
    }
}
fn config() -> Result<PathBuf, String> {
    crate::last_open_dir_config_path()
        .map(|path| path.with_file_name("screen_record_history.json"))
        .ok_or_else(|| tr("无法确定录屏设置目录。").to_owned())
}
fn same_path(first: &Path, second: &Path) -> bool {
    if cfg!(windows) {
        first
            .to_string_lossy()
            .eq_ignore_ascii_case(&second.to_string_lossy())
    } else {
        first == second
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_history_has_independent_bounds_and_deduplicates() {
        let mut history = History {
            entries: vec![],
            limit: 20,
            selected: None,
            open: false,
            error: None,
        };
        for n in 0..25 {
            history.insert(PathBuf::from(format!("recording-{n}.mkv")));
        }
        assert_eq!(history.entries.len(), 20);
        assert_eq!(history.entries[0], PathBuf::from("recording-24.mkv"));
        history.insert(PathBuf::from("recording-10.mkv"));
        assert_eq!(history.entries.len(), 20);
        assert_eq!(history.entries[0], PathBuf::from("recording-10.mkv"));
        assert_eq!((MIN_LIMIT, MAX_LIMIT, DEFAULT_LIMIT), (20, 50, 20));
    }
}
