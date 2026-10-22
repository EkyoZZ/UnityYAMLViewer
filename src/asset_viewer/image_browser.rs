//! Docked non-recursive directory filmstrip with on-demand thumbnail decoding.
use super::*;
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
}
impl Browser {
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
    pub fn position(&self, path: &Path) -> Option<(usize, usize)> {
        self.files
            .iter()
            .position(|file| file == path)
            .map(|index| (index + 1, self.files.len()))
    }
    fn scan(&mut self, ctx: &egui::Context) {
        let directory = self.owner.parent().unwrap_or(Path::new(".")).to_owned();
        let (tx, rx) = mpsc::channel();
        self.scan = Some(rx);
        self.error = None;
        self.thumbs.clear();
        self.pending = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let mut files = Vec::new();
                for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_type().map_err(|e| e.to_string())?.is_file()
                        && is_bitmap(&entry.path())
                    {
                        files.push(entry.path());
                    }
                }
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
                self.thumbs.clear();
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
                        let response = response.on_hover_text(full_path.display().to_string());
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
                        let mut reader = image::ImageReader::open(&path)
                            .map_err(|e| e.to_string())?
                            .with_guessed_format()
                            .map_err(|e| e.to_string())?;
                        let mut limits = image::Limits::default();
                        limits.max_image_width = Some(8192);
                        limits.max_image_height = Some(8192);
                        limits.max_alloc = Some(256 * 1024 * 1024);
                        reader.limits(limits);
                        let image = reader
                            .decode()
                            .map_err(|e| e.to_string())?
                            .thumbnail(44, 44)
                            .to_rgba8();
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
