//! Recording history and its thumbnail cache. Removing entries leaves video files intact.
use crate::localization::tr;
use eframe::egui::{self, Color32, RichText};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
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
    thumbnails: HashMap<PathBuf, Option<egui::TextureHandle>>,
    thumbnail_job: Option<ThumbnailJob>,
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
            thumbnails: HashMap::new(),
            thumbnail_job: None,
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
        self.trim_entries();
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
            self.trim_entries();
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
        self.clear_thumbnail(path);
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
    fn trim_entries(&mut self) {
        if self.entries.len() > self.limit {
            for path in self.entries.split_off(self.limit) {
                self.clear_thumbnail(&path);
            }
        }
    }

    fn clear_thumbnail(&mut self, path: &Path) {
        self.thumbnails.remove(path);
        if let Some(cache) = thumbnail_path(path) {
            let _ = std::fs::remove_file(cache);
        }
    }

    fn load_thumbnail(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.thumbnail_job {
            let image = match job.result.try_recv() {
                Ok(image) => image,
                Err(mpsc::TryRecvError::Disconnected) => None,
                Err(mpsc::TryRecvError::Empty) => return,
            };
            let path = self.thumbnail_job.take().unwrap().path;
            if self.entries.contains(&path) && path.is_file() {
                let texture = image.map(|image| {
                    if let Some(png) = image.new_png
                        && let Some(cache) = thumbnail_path(&path)
                        && let Some(parent) = cache.parent()
                        && std::fs::create_dir_all(parent).is_ok()
                    {
                        let _ = std::fs::write(cache, png);
                    }
                    ctx.load_texture(
                        "recording_history_thumbnail",
                        image.pixels,
                        egui::TextureOptions::LINEAR,
                    )
                });
                self.thumbnails.insert(path, texture);
            }
        }
        let Some(path) = self
            .entries
            .iter()
            .find(|path| !self.thumbnails.contains_key(*path) && path.is_file())
            .cloned()
        else {
            return;
        };
        let (send, result) = mpsc::channel();
        let video = path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(load_thumbnail(&video));
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        });
        self.thumbnail_job = Some(ThumbnailJob { path, result });
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
        self.load_thumbnail(ctx);
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
                            let selected = self
                                .selected
                                .as_ref()
                                .is_some_and(|selected| same_path(selected, path));
                            let thumbnail = if exists {
                                self.thumbnails.get(path).and_then(Option::as_ref)
                            } else {
                                None
                            };
                            if history_row(ui, &name, path, exists, selected, thumbnail).clicked() {
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
        if !open {
            self.thumbnails.retain(|_, texture| texture.is_some());
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(2));
        edit
    }
}
struct ThumbnailJob {
    path: PathBuf,
    result: mpsc::Receiver<Option<ThumbnailImage>>,
}

struct ThumbnailImage {
    pixels: egui::ColorImage,
    new_png: Option<Vec<u8>>,
}

fn thumbnail_path(video: &Path) -> Option<PathBuf> {
    use sha2::{Digest, Sha256};
    let identity = video.to_string_lossy();
    let identity = if cfg!(windows) {
        identity.to_ascii_lowercase()
    } else {
        identity.into_owned()
    };
    let name = format!("{:x}.png", Sha256::digest(identity.as_bytes()));
    Some(
        config()
            .ok()?
            .with_file_name("screen_record_thumbnails")
            .join(name),
    )
}

fn load_thumbnail(video: &Path) -> Option<ThumbnailImage> {
    fn decode(png: &[u8]) -> Option<egui::ColorImage> {
        let pixels = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .ok()?
            .to_rgba8();
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [pixels.width() as usize, pixels.height() as usize],
            pixels.as_raw(),
        ))
    }
    if let Some(cache) = thumbnail_path(video)
        && let Ok(video_modified) = std::fs::metadata(video).and_then(|meta| meta.modified())
        && std::fs::metadata(&cache)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified >= video_modified)
        && let Ok(png) = std::fs::read(cache)
        && let Some(pixels) = decode(&png)
    {
        return Some(ThumbnailImage {
            pixels,
            new_png: None,
        });
    }
    let ffmpeg = super::find_tool("ffmpeg")?;
    let png = crate::media_tools::video_thumbnail_png(&ffmpeg, video, 224, 224)?;
    Some(ThumbnailImage {
        pixels: decode(&png)?,
        new_png: Some(png),
    })
}

fn history_row(
    ui: &mut egui::Ui,
    name: &str,
    path: &Path,
    exists: bool,
    selected: bool,
    texture: Option<&egui::TextureHandle>,
) -> egui::Response {
    let response = ui.add_sized(
        [ui.available_width(), 62.0],
        egui::Button::new("").selected(selected),
    );
    let row = response.rect.shrink(5.0);
    let preview = egui::Rect::from_min_size(row.min, egui::vec2(92.0, row.height()));
    ui.painter()
        .rect_filled(preview, 2.0, ui.visuals().extreme_bg_color);
    if let Some(texture) = texture {
        let size = texture.size_vec2();
        let scale = (preview.width() / size.x).min(preview.height() / size.y);
        let frame = egui::Rect::from_center_size(preview.center(), size * scale);
        ui.painter().image(
            texture.id(),
            frame,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let date = recording_date(path);
    let mut text_right = row.right();
    if let Some(date) = date {
        let font = egui::TextStyle::Small.resolve(ui.style());
        let color = ui.visuals().weak_text_color();
        let galley = ui.painter().layout_no_wrap(date, font, color);
        text_right -= galley.size().x + 10.0;
        ui.painter().with_clip_rect(row.intersect(ui.clip_rect())).galley(
            egui::pos2(row.right() - galley.size().x, row.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }
    let text_left = preview.right() + 8.0;
    let text_rect = egui::Rect::from_min_max(
        egui::pos2(text_left, row.top()),
        egui::pos2(text_right.max(text_left), row.bottom()),
    );
    let mut text = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    text.set_clip_rect(text_rect.intersect(ui.clip_rect()));
    let color = if exists {
        ui.visuals().text_color()
    } else {
        ui.visuals().error_fg_color
    };
    text.add_space(7.0);
    text.add(egui::Label::new(RichText::new(name).color(color)).truncate());
    text.add(
        egui::Label::new(
            RichText::new(path.display().to_string())
                .small()
                .color(color),
        )
        .truncate(),
    );
    response.on_hover_text(path.display().to_string())
}

#[cfg(windows)]
fn recording_date(path: &Path) -> Option<String> {
    use std::time::{Duration, UNIX_EPOCH};

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[derive(Default)]
    #[repr(C)]
    struct LocalTime {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millisecond: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FileTimeToSystemTime(file: *const FileTime, utc: *mut LocalTime) -> i32;
        fn SystemTimeToTzSpecificLocalTimeEx(
            zone: *const std::ffi::c_void,
            utc: *const LocalTime,
            local: *mut LocalTime,
        ) -> i32;
    }

    let created = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("recording-"))
        .and_then(|stamp| stamp.parse().ok())
        .and_then(|ns| UNIX_EPOCH.checked_add(Duration::from_nanos(ns)))
        .or_else(|| path.metadata().ok()?.created().ok())?;
    let elapsed = created.duration_since(UNIX_EPOCH).ok()?;
    let ticks = u64::try_from(elapsed.as_nanos() / 100).ok()?
        .checked_add(116_444_736_000_000_000)?;
    let file_time = FileTime { low: ticks as u32, high: (ticks >> 32) as u32 };
    let mut utc = LocalTime::default();
    let mut local = LocalTime::default();
    unsafe {
        if FileTimeToSystemTime(&file_time, &mut utc) == 0
            || SystemTimeToTzSpecificLocalTimeEx(std::ptr::null(), &utc, &mut local) == 0
        {
            return None;
        }
    }
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        local.year, local.month, local.day, local.hour, local.minute, local.second,
    ))
}

#[cfg(not(windows))]
fn recording_date(_: &Path) -> Option<String> {
    None
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
            thumbnails: HashMap::new(),
            thumbnail_job: None,
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
