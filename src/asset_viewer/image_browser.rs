//! Docked non-recursive directory filmstrip with on-demand thumbnail decoding.
use super::*;

#[cfg(test)]
mod folder_selection_tests {
    use super::*;

    #[test]
    fn same_folder_selection_keeps_scan_and_thumbnail_cache() {
        let first = PathBuf::from("images/a.png");
        let next = PathBuf::from("images/b.png");
        let (_sender, receiver) = mpsc::channel();
        let mut browser = Browser::default();
        browser.owner = first.clone();
        browser.selected = first.clone();
        browser.multiple = true;
        browser.files = vec![first.clone(), next.clone()];
        browser.scan = Some(receiver);
        browser.thumbs.insert(first.clone(), Err("cached thumbnail error".into()));
        browser.select_from_folder(&next);
        browser.forget(&first);
        assert_eq!(browser.owner, next);
        assert_eq!(browser.selected, next);
        assert_eq!(browser.files.len(), 2);
        assert!(browser.multiple && browser.reveal);
        assert!(browser.scan.is_some());
        assert!(browser.thumbs.contains_key(&first));
    }

    #[test]
    fn prefetch_neighbours_are_bounded_and_do_not_wrap() {
        let mut browser=Browser::default();
        browser.multiple=true;
        browser.files=(0..12).map(|i|PathBuf::from(format!("frames/{i}.png"))).collect();
        let expected=[6,4,7,3,8,2,9,1].map(|i|browser.files[i].clone());
        assert_eq!(browser.nearby(&browser.files[5]),expected);
        assert_eq!(browser.nearby(&browser.files[0]),browser.files[1..5]);
        assert_eq!(browser.nearby(&browser.files[11]),[10,9,8,7].map(|i|browser.files[i].clone()));
        assert!(browser.nearby(Path::new("elsewhere.png")).is_empty());
        browser.multiple=false;
        assert!(browser.nearby(&browser.files[5]).is_empty());
    }

    #[test]
    fn another_folder_does_not_reuse_the_old_selection() {
        let mut browser = Browser::default();
        browser.owner = PathBuf::from("first/a.png");
        browser.selected = browser.owner.clone();
        browser.select_from_folder(Path::new("second/b.png"));
        assert_eq!(browser.owner, PathBuf::from("first/a.png"));
        assert_eq!(browser.selected, browser.owner);
    }
}
fn folder_images(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_file()
            && is_browsable_image(&entry.path())
        {
            files.push(entry.path());
        }
    }
    Ok(files)
}

#[derive(Default)]
struct BatchResult {
    directory: PathBuf,
    alpha: Vec<PathBuf>,
    text: Vec<PathBuf>,
    both: Vec<PathBuf>,
    unchanged: usize,
    errors: Vec<String>,
}

#[derive(Default)]
pub(super) struct Browser {
    owner: PathBuf,
    selected: PathBuf,
    multiple: bool,
    default_multiple: bool,
    files: Vec<PathBuf>,
    scan: Option<mpsc::Receiver<Result<Vec<PathBuf>, String>>>,
    pending: Option<mpsc::Receiver<(PathBuf, Result<egui::ColorImage, String>)>>,
    thumbs: HashMap<PathBuf, Result<egui::TextureHandle, String>>,
    error: Option<String>,
    reveal: bool,
    confirm_optimization: bool,
    optimization: Option<mpsc::Receiver<BatchResult>>,
    optimization_result: Option<BatchResult>,
    optimization_report_open: bool,
}
impl Browser {
    pub fn select_from_folder(&mut self, path: &Path) {
        if !self.owner.as_os_str().is_empty() && self.owner.parent() == path.parent() {
            self.owner = path.to_owned();
            self.selected = path.to_owned();
            self.reveal = true;
        }
    }
    pub fn set_default_multiple(&mut self, enabled: bool) {
        self.default_multiple = enabled;
    }
    pub fn forget(&mut self, path: &Path) {
        if self.owner == path {
            self.owner = PathBuf::new();
        }
    }

    pub fn is_multiple(&self) -> bool {
        self.multiple
    }
    /// Nearest frames first, favour forward browsing without assuming numeric names.
    pub fn nearby(&self, path: &Path) -> Vec<PathBuf> {
        if !self.multiple { return Vec::new(); }
        let Some(index)=self.files.iter().position(|file|file==path) else {return Vec::new();};
        [1isize,-1,2,-2,3,-3,4,-4].into_iter()
            .filter_map(|offset| index.checked_add_signed(offset).and_then(|i|self.files.get(i)).cloned())
            .collect()
    }
    pub fn position(&self, path: &Path) -> Option<(usize, usize)> {
        self.files
            .iter()
            .position(|file| file == path)
            .map(|index| (index + 1, self.files.len()))
    }
    fn directory(&self) -> &Path {
        self.owner.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."))
    }

    fn scan(&mut self, ctx: &egui::Context) {
        let directory = self.directory().to_owned();
        let (tx, rx) = mpsc::channel();
        self.scan = Some(rx);
        self.error = None;
        self.confirm_optimization = false;
        self.thumbs.clear();
        self.pending = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let mut files = folder_images(&directory)?;
                files.sort_by_cached_key(|p| {
                    p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                });
                Ok(files)
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path) -> PathBuf {
        if self.owner != path {
            self.multiple = self.default_multiple;
            self.owner = path.to_owned();
            self.selected = path.to_owned();
            self.files.clear();
            self.scan = None;
            self.pending = None;
            self.thumbs.clear();
            self.error = None;
            self.reveal = true;
            if self.multiple {
                self.scan(ui.ctx());
            }
        }
        if let Some(result) = self.scan.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.scan = None;
            match result {
                Ok(files) => {
                    self.files = files;
                    self.reveal = true;
                }
                Err(e) => self.error = Some(e),
            }
        }
        if let Some((path, result)) = self.pending.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.pending = None;
            if self.thumbs.len() >= 192 {
                if let Some(evicted) = self.thumbs.keys()
                    .find(|cached| *cached != &self.selected && *cached != &path).cloned() {
                    self.thumbs.remove(&evicted);
                }
            }
            self.thumbs.insert(
                path.clone(),
                result.map(|image| {
                    ui.ctx().load_texture(
                        format!("thumb:{}", path.display()),
                        image,
                        egui::TextureOptions::LINEAR,
                    )
                }),
            );
        }
        self.selected.clone()
    }
    pub fn controls(&mut self, ui: &mut egui::Ui) {
        let previous = self.multiple;
        let mut mode = if self.multiple { "multi" } else { "single" }.to_owned();
        crate::theme::compact_selector(
            ui,
            &mut mode,
            [("single", "单图查看"), ("multi", "多图浏览")],
        );
        self.multiple = mode == "multi";
        if previous != self.multiple {
            self.reveal = true;
            if self.multiple {
                self.scan(ui.ctx());
            }
            ui.ctx().request_repaint();
        }
    }
    pub fn optimizing(&self) -> bool {
        self.optimization.is_some()
    }

    pub fn poll_optimization(&mut self) -> Vec<PathBuf> {
        if let Some(mut result) = self.optimization.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.optimization = None;
            for files in [&mut result.alpha, &mut result.text, &mut result.both] {
                files.sort();
            }
            let changed = result.alpha.iter().chain(&result.text).chain(&result.both).cloned().collect();
            self.optimization_result = Some(result);
            self.optimization_report_open = true;
            changed
        } else {
            Vec::new()
        }
    }

    pub fn optimization_button(&mut self, ui: &mut egui::Ui, enabled: bool) {
        use crate::localization::tr;
        if !self.multiple {
            return;
        }
        let can_start = enabled && self.scan.is_none() && !self.optimizing()
            && self.files.iter().any(|path| extension(path) == "png");
        if ui.add_enabled(can_start, crate::theme::green_outline_button(ui, tr("批量优化PNG")))
            .on_hover_text(tr("去除Alpha通道和iTXt元信息")).clicked()
        {
            self.confirm_optimization = true;
        }
        if self.optimizing() {
            ui.spinner();
        }
        if self.confirm_optimization {
            let response = egui::Modal::new(ui.id().with("confirm_png_optimization")).show(ui.ctx(), |ui| {
                ui.set_width(380.0_f32.min(ui.ctx().screen_rect().width() * 0.9));
                ui.vertical_centered(|ui| ui.heading(tr("批量优化PNG")));
                ui.add_space(12.0);
                ui.label(tr("将原位优化当前目录中的 PNG（不含子目录）。批量修改无法撤销。"));
                ui.label(tr("移除 PNG 中的 iTXt 元信息。"));
                ui.label(tr("仅当静态 PNG 为 32 位 RGBA 且所有像素的 Alpha 都是 255（完全不透明）时，才移除 Alpha 通道。"));
                ui.add_space(4.0);
                ui.label(std::path::absolute(self.directory()).unwrap_or_else(|_| self.directory().to_owned())
                    .display().to_string());
                ui.add_space(14.0);
                let button_size = egui::vec2(80.0, 30.0);
                let gap = 10.0;
                ui.vertical_centered(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(button_size.x * 2.0 + gap, button_size.y),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            let confirm = ui.add_enabled(
                                can_start, egui::Button::new(tr("确认")).min_size(button_size),
                            ).clicked();
                            let cancel = ui.add(egui::Button::new(tr("取消")).min_size(button_size)).clicked();
                            (confirm, cancel)
                        },
                    ).inner
                }).inner
            });
            let (confirm, cancel) = response.inner;
            if confirm || cancel || response.should_close() {
                self.confirm_optimization = false;
            }
            if confirm {
                let directory = self.directory().to_owned();
                let (tx, rx) = mpsc::channel();
                self.optimization = Some(rx);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let scan = folder_images(&directory);
                    let mut result = BatchResult { directory, ..Default::default() };
                    let paths = match scan {
                        Ok(paths) => paths,
                        Err(error) => {
                            result.errors.push(error);
                            Vec::new()
                        }
                    };
                    use super::image_info::Optimization;
                    for path in paths.into_iter().filter(|path| extension(path) == "png") {
                        match super::image_info::optimize_png(&path) {
                            Ok(Optimization::Alpha) => result.alpha.push(path),
                            Ok(Optimization::Text) => result.text.push(path),
                            Ok(Optimization::Both) => result.both.push(path),
                            Ok(Optimization::Unchanged) => result.unchanged += 1,
                            Err(error) => result.errors.push(format!("{}: {error}", path.display())),
                        }
                    }
                    let _ = tx.send(result);
                    ctx.request_repaint();
                });
            }
        }
    }

    pub fn show_optimization_report(&mut self, ctx: &egui::Context) {
        use crate::localization::tr;
        let Some(result) = &self.optimization_result else { return; };
        crate::native_window::Window::new(tr("PNG 批量优化报告"))
            .id(egui::Id::new("png_optimization_report"))
            .open(&mut self.optimization_report_open)
            .resizable(true)
            .default_width(540.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.label(result.directory.display().to_string());
                let changed = result.alpha.len() + result.text.len() + result.both.len();
                ui.label(crate::lf!("PNG 优化完成：修改 {0}，无需修改 {1}，失败 {2}",
                    changed, result.unchanged, result.errors.len()));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (label, files) in [
                        ("仅去除 Alpha 通道", &result.alpha),
                        ("仅清理 iTXt 元信息", &result.text),
                        ("同时去除 Alpha 和 iTXt", &result.both),
                    ] {
                        ui.heading(format!("{} ({})", tr(label), files.len()));
                        for path in files {
                            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                        }
                        ui.add_space(8.0);
                    }
                    if !result.errors.is_empty() {
                        ui.heading(tr("失败"));
                        for error in &result.errors {
                            ui.label(error);
                        }
                    }
                });
            });
    }

    pub fn navigate(&mut self, ui: &mut egui::Ui, left: egui::Rect, right: egui::Rect) {
        let index = self.files.iter().position(|p| p == &self.selected);
        let mut delta = 0isize;
        for (previous, button) in [(true, left), (false, right)] {
            let enabled = index.is_some_and(|i| {
                if previous {
                    i > 0
                } else {
                    i + 1 < self.files.len()
                }
            });
            let response = ui.interact(
                button,
                ui.id().with(("image_navigation", previous)),
                egui::Sense::click(),
            );
            let painter = ui.painter().with_clip_rect(ui.clip_rect());
            let center = button.center();
            painter.circle_filled(
                center,
                button.height() * 0.5,
                Color32::from_black_alpha(if enabled && response.hovered() {
                    180
                } else {
                    128
                }),
            );
            let direction = if previous { -1.0 } else { 1.0 };
            let r = button.height() * 0.18;
            let points = vec![
                center + egui::vec2(direction * r, 0.0),
                center + egui::vec2(-direction * r * 0.5, -r * 0.8660254),
                center + egui::vec2(-direction * r * 0.5, r * 0.8660254),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                if enabled {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(70)
                },
                egui::Stroke::NONE,
            ));
            if enabled && response.clicked() {
                delta = if previous { -1 } else { 1 };
            }
        }
        if !ui.ctx().wants_keyboard_input() {
            if ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
            }) {
                delta = -1;
            }
            if ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            }) {
                delta = 1;
            }
        }
        if let Some(i) = index {
            if delta != 0 {
                let next = (i as isize + delta)
                    .clamp(0, self.files.len().saturating_sub(1) as isize)
                    as usize;
                self.selected = self.files[next].clone();
                self.reveal = true;
                ui.ctx().request_repaint();
            }
        }
    }
    pub fn strip(&mut self, ui: &mut egui::Ui) {
        let active = self.selected.clone();
        let mut request = None;
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.add_space(4.0);
        egui::ScrollArea::horizontal()
            .id_salt("filmstrip_scroll")
            .max_height(48.0)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for file in &self.files {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::click());
                        let selected = file == &self.selected;
                        if selected && self.reveal {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                        if ui.is_rect_visible(rect) {
                            if let Some(Ok(texture)) = self.thumbs.get(file) {
                                let size = texture.size_vec2();
                                let scale = (44.0 / size.x).min(44.0 / size.y);
                                let image_rect =
                                    egui::Rect::from_center_size(rect.center(), size * scale);
                                let painter = ui.painter();
                                let image_rect = image_rect.translate(
                                    image_rect.min.round_to_pixels(painter.pixels_per_point())
                                        - image_rect.min,
                                );
                                // Background is visible only through the thumbnail's alpha.
                                let cell = 4.0;
                                let mut checker = egui::Mesh::default();
                                for y in 0..(image_rect.height() / cell).ceil() as usize {
                                    for x in 0..(image_rect.width() / cell).ceil() as usize {
                                        let tile = egui::Rect::from_min_size(
                                            image_rect.min
                                                + egui::vec2(x as f32 * cell, y as f32 * cell),
                                            egui::Vec2::splat(cell),
                                        );
                                        let tile = egui::Rect::from_min_max(
                                            tile.min.round_to_pixels(painter.pixels_per_point()),
                                            tile.max.round_to_pixels(painter.pixels_per_point()),
                                        )
                                        .intersect(image_rect);
                                        if tile.is_positive() {
                                            checker.add_colored_rect(
                                                tile,
                                                Color32::from_gray(if (x + y) % 2 == 0 {
                                                    190
                                                } else {
                                                    230
                                                }),
                                            );
                                        }
                                    }
                                }
                                painter.add(egui::Shape::mesh(checker));
                                ui.painter().image(
                                    texture.id(),
                                    image_rect,
                                    egui::Rect::from_min_max(
                                        egui::Pos2::ZERO,
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                            } else if let Some(Err(_)) = self.thumbs.get(file) {
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "!",
                                    egui::FontId::proportional(20.0), ui.visuals().error_fg_color);
                            } else if !self.thumbs.contains_key(file) && request.is_none() {
                                request = Some(file.clone());
                            }
                        }
                        if selected && ui.is_rect_visible(rect) {
                            ui.painter().rect_stroke(
                                rect.shrink(1.0),
                                0.0,
                                egui::Stroke::new(3.0_f32, crate::theme::BLUE),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let full_path = std::path::absolute(file).unwrap_or_else(|_| file.clone());
                        let tooltip = match self.thumbs.get(file) {
                            Some(Err(error)) => format!("{}\n{error}", full_path.display()),
                            _ => full_path.display().to_string(),
                        };
                        let response = response.on_hover_text(tooltip);
                        if response.clicked() {
                            self.selected = file.clone();
                            self.reveal = true;
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });
        self.reveal = self.selected != active;
        if self.reveal {
            ui.ctx().request_repaint();
        }
        if self.pending.is_none() {
            if let Some(path) = request {
                let (tx, rx) = mpsc::channel();
                self.pending = Some(rx);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let result = (|| {
                        let original = load_preview_pixels(&path)?;
                        let image = image::imageops::thumbnail(&original, 44, 44);
                        Ok(egui::ColorImage {
                            size: [image.width() as usize, image.height() as usize],
                            pixels: image.pixels().map(|p| preview_color(p.0)).collect(),
                        })
                    })();
                    let _ = tx.send((path, result));
                    ctx.request_repaint();
                });
            }
        }
    }
}
