use super::bitmap_workspace::{MAX_IMAGE_PIXELS, MAX_IMAGE_SIDE};
use super::*;
use image::imageops::{self, FilterType};
use image::{GrayImage, ImageBuffer, Luma, Pixel};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Canvas,
    Image,
}

pub(super) struct Draft {
    mode: Mode,
    original: [u32; 2],
    percent: [f64; 2],
    pixels: bool,
    linked: bool,
    relative: bool,
    anchor: [i32; 2],
    filter: FilterType,
}

impl Draft {
    fn new(mode: Mode, original: [u32; 2]) -> Self {
        Self {
            mode,
            original,
            percent: [100.; 2],
            pixels: mode == Mode::Canvas,
            linked: mode == Mode::Image,
            relative: false,
            anchor: [1, 1],
            filter: FilterType::CatmullRom,
        }
    }

    fn output(&self) -> [u32; 2] {
        std::array::from_fn(|axis| {
            (self.original[axis] as f64 * self.percent[axis] / 100.)
                .round()
                .max(1.) as u32
        })
    }

    fn offset(&self) -> [i64; 2] {
        let target = self.output();
        std::array::from_fn(|axis| {
            (i64::from(target[axis]) - i64::from(self.original[axis]))
                * i64::from(self.anchor[axis])
                / 2
        })
    }

    fn display_value(&self, axis: usize, percent: f64) -> f64 {
        let source = f64::from(self.original[axis]);
        if self.pixels {
            (source * percent / 100.).round().max(1.) - if self.relative { source } else { 0. }
        } else {
            percent - if self.relative { 100. } else { 0. }
        }
    }

    fn set_display_value(&mut self, axis: usize, value: f64) {
        let source = f64::from(self.original[axis]);
        self.percent[axis] = if self.pixels {
            let pixels = value.round() + if self.relative { source } else { 0. };
            100. * pixels / source
        } else {
            value + if self.relative { 100. } else { 0. }
        };
        if self.linked {
            self.percent[1 - axis] = self.percent[axis];
        }
    }

    fn set_relative(&mut self, relative: bool) {
        self.relative = relative;
        if relative {
            self.linked = false;
        }
    }

    fn input(&mut self, ui: &mut egui::Ui, axis: usize, size: Vec2) {
        let source = f64::from(self.original[axis]);
        let range = self.display_value(axis, 100. / source)
            ..=self.display_value(axis, 100. * f64::from(MAX_IMAGE_SIDE) / source);
        let mut value = self.display_value(axis, self.percent[axis]);
        let prefix = if self.relative && value > 0. { "+" } else { "" };
        let drag = egui::DragValue::new(&mut value)
            .range(range)
            .clamp_existing_to_range(false)
            .speed(1.)
            .max_decimals(if self.pixels { 0 } else { 4 })
            .prefix(prefix)
            .suffix(if self.pixels { " px" } else { " %" });
        // DragValue can grow to its text width while typing. Keep both hit areas
        // and their painted bounds in the same fixed column even for long edits.
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let mut field = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        ));
        field.shrink_clip_rect(rect);
        field.spacing_mut().interact_size = size;
        let visuals = field.visuals_mut();
        visuals.selection.stroke = visuals.widgets.hovered.bg_stroke;
        for state in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            state.expansion = 0.;
            state.corner_radius = egui::CornerRadius::ZERO;
        }
        if field.add(drag).changed() {
            self.set_display_value(axis, value);
        }
    }

    fn fields(&mut self, ui: &mut egui::Ui) {
        ui.small(format!(
            "{}  {} × {} px",
            tr("原始尺寸"),
            self.original[0],
            self.original[1]
        ));
        let labels = [tr("宽度"), tr("高度")];
        let font = egui::TextStyle::Body.resolve(ui.style());
        let label_width = labels
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap((*label).to_owned(), font.clone(), ui.visuals().text_color())
                    .size()
                    .x
                    .ceil()
            })
            .fold(0., f32::max);
        let row_height = 24.;
        let input_size = egui::vec2(80., row_height);
        let field_width = label_width + ui.spacing().item_spacing.x + input_size.x;
        // Reserve both rows before placing the link and unit tabs on their shared centerline.
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 52.),
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
            |ui| {
                let link = ui.add_enabled_ui(!self.relative, |ui| {
                    crate::theme::size_link(ui, &mut self.linked)
                });
                if link.inner && self.linked {
                    self.percent[1] = self.percent[0];
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(field_width, 52.),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 4.;
                        for (axis, label) in labels.into_iter().enumerate() {
                            ui.allocate_ui_with_layout(
                                egui::vec2(field_width, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add_sized(
                                        [label_width, row_height],
                                        egui::Label::new(label).extend().halign(egui::Align::Min),
                                    );
                                    self.input(ui, axis, input_size);
                                },
                            );
                        }
                    },
                );
                let mut unit = if self.pixels { "pixels" } else { "percent" }.to_owned();
                crate::theme::compact_selector(
                    ui,
                    &mut unit,
                    [("percent", "百分比"), ("pixels", "像素")],
                );
                self.pixels = unit == "pixels";
            },
        );
        if self.mode == Mode::Canvas {
            let mut relative = self.relative;
            if ui
                .checkbox(&mut relative, tr("相对"))
                .on_hover_text(tr("正值扩展画布，负值裁切，零保持原尺寸。宽高分别调整。"))
                .changed()
            {
                self.set_relative(relative);
            }
            ui.horizontal(|ui| {
                ui.label(tr("锚点"));
                egui::Grid::new("canvas_anchor")
                    .spacing(Vec2::splat(2.))
                    .show(ui, |ui| {
                        for (row, labels) in [["↖", "↑", "↗"], ["←", "●", "→"], ["↙", "↓", "↘"]]
                            .iter()
                            .enumerate()
                        {
                            for (col, &label) in labels.iter().enumerate() {
                                let pos = [col as i32, row as i32];
                                if ui
                                    .add_sized(
                                        [22., 22.],
                                        egui::Button::new(label).selected(self.anchor == pos),
                                    )
                                    .clicked()
                                {
                                    self.anchor = pos;
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        } else {
            const FILTERS: [(FilterType, &str); 4] = [
                (FilterType::Nearest, "邻近（硬边缘）"),
                (FilterType::Triangle, "两次线性"),
                (FilterType::CatmullRom, "两次立方"),
                (FilterType::Lanczos3, "兰佐斯（Lanczos3）"),
            ];
            ui.horizontal(|ui| {
                ui.label(tr("插值方式"));
                let label = FILTERS.iter().find(|(f, _)| *f == self.filter).unwrap().1;
                egui::ComboBox::from_id_salt("image_resize_filter")
                    .selected_text(tr(label))
                    .show_ui(ui, |ui| {
                        for (filter, label) in FILTERS {
                            ui.selectable_value(&mut self.filter, filter, tr(label));
                        }
                    });
            });
        }
    }
}

// imageops filters vertically into a float buffer before sampling horizontally.
// Shrink width first for extreme aspect changes that would exceed the pixel budget.
fn filtered<P: Pixel + 'static>(
    source: &ImageBuffer<P, Vec<P::Subpixel>>,
    size: [u32; 2],
    filter: FilterType,
) -> ImageBuffer<P, Vec<P::Subpixel>> {
    let [w, h] = size;
    if filter == FilterType::Nearest {
        let index = |v: u32, from: u32, to: u32| ((2 * v + 1) * from) / (2 * to);
        return ImageBuffer::from_fn(w, h, |x, y| {
            *source.get_pixel(index(x, source.width(), w), index(y, source.height(), h))
        });
    }
    if u64::from(source.width()) * u64::from(h) > MAX_IMAGE_PIXELS {
        let narrow = imageops::resize(source, w, source.height(), filter);
        imageops::resize(&narrow, w, h, filter)
    } else {
        imageops::resize(source, w, h, filter)
    }
}

fn resize_layer(source: &RgbaImage, size: [u32; 2], filter: FilterType) -> RgbaImage {
    if filter == FilterType::Nearest || source.pixels().all(|p| p[3] == 255) {
        return filtered(source, size, filter);
    }
    // RGB*alpha fits in u16. This keeps low-alpha precision and prevents invisible
    // RGB in a transparent pixel leaking into neighbouring pixels during filtering.
    let premult = ImageBuffer::from_fn(source.width(), source.height(), |x, y| {
        let [r, g, b, a] = source.get_pixel(x, y).0.map(u16::from);
        Rgba([r * a, g * a, b * a, 255 * a])
    });
    let scaled = filtered(&premult, size, filter);
    RgbaImage::from_fn(size[0], size[1], |x, y| {
        let [r, g, b, a] = scaled.get_pixel(x, y).0.map(u32::from);
        let alpha = ((a + 127) / 255).min(255) as u8;
        if alpha == 0 {
            return Rgba([0; 4]);
        }
        let rgb = [r, g, b].map(|v| ((v * 255 + a / 2) / a).min(255) as u8);
        Rgba([rgb[0], rgb[1], rgb[2], alpha])
    })
}

impl Editor {
    pub(super) fn image_size_controls(&mut self, ui: &mut egui::Ui) {
        let original = [self.pixels.width(), self.pixels.height()];
        if self
            .image_size_draft
            .as_ref()
            .is_some_and(|d| d.original != original)
        {
            self.image_size_draft = None;
        }
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [(Mode::Canvas, "画布尺寸"), (Mode::Image, "图像大小")] {
                let selected = self
                    .image_size_draft
                    .as_ref()
                    .is_some_and(|d| d.mode == mode);
                if ui
                    .add(egui::Button::new(tr(label)).selected(selected))
                    .clicked()
                {
                    self.image_size_draft = (!selected).then(|| Draft::new(mode, original));
                }
            }
        });
        let Some(mut draft) = self.image_size_draft.take() else {
            return;
        };
        draft.fields(ui);
        let size = draft.output();
        let side_limit = MAX_IMAGE_SIDE.min(ui.input(|i| i.max_texture_side) as u32);
        let valid = self.validate_image_size(size, side_limit);
        ui.small(format!("{}  {} × {} px", tr("输出尺寸"), size[0], size[1]));
        if let Err(reason) = valid {
            ui.colored_label(ui.visuals().error_fg_color, tr(reason));
        }
        let mut finished = false;
        let button_size = egui::vec2(76., 24.);
        let row_width = 2. * button_size.x + ui.spacing().item_spacing.x;
        let inset = ((ui.available_width() - row_width) / 2.).max(0.);
        ui.horizontal(|ui| {
            ui.add_space(inset);
            if ui
                .add_enabled(
                    valid.is_ok() && size != original,
                    egui::Button::new(tr("确定")).min_size(button_size),
                )
                .clicked()
            {
                match self.apply_image_size(&draft, side_limit) {
                    Ok(()) => finished = true,
                    Err(reason) => self.message = tr(reason).into(),
                }
            }
            if ui
                .add(egui::Button::new(tr("取消")).min_size(button_size))
                .clicked()
            {
                finished = true;
            }
        });
        if !finished {
            self.image_size_draft = Some(draft);
        }
    }

    fn validate_image_size(&self, size: [u32; 2], side_limit: u32) -> Result<(), &'static str> {
        if self.layers.iter().any(|layer| layer.settings.vector.is_some()) {
            return Err("当前文档含矢量图层，暂不能调整尺寸。");
        }
        let pixels = u64::from(size[0]) * u64::from(size[1]);
        if size.contains(&0)
            || size.iter().any(|&n| n > side_limit.min(MAX_IMAGE_SIDE))
            || pixels > MAX_IMAGE_PIXELS
        {
            return Err("当前图像超出编辑画布限制。");
        }
        let bytes_per_pixel: usize = self
            .layers
            .iter()
            .map(|l| 4 + usize::from(l.settings.mask.is_some()))
            .sum();
        if pixels * bytes_per_pixel as u64 > self.layer_budget() as u64 {
            return Err("调整后的图层与蒙版超过内存上限。");
        }
        Ok(())
    }

    fn apply_image_size(&mut self, draft: &Draft, side_limit: u32) -> Result<(), &'static str> {
        let target = draft.output();
        let old = [self.pixels.width(), self.pixels.height()];
        if target == old {
            return Ok(());
        }
        self.validate_image_size(target, side_limit)?;
        let [w, h] = target;
        let offset = draft.offset();
        let scale = if draft.mode == Mode::Image {
            egui::vec2(w as f32 / old[0] as f32, h as f32 / old[1] as f32)
        } else {
            Vec2::splat(1.)
        };
        let shift = if draft.mode == Mode::Canvas {
            egui::vec2(offset[0] as f32, offset[1] as f32)
        } else {
            Vec2::ZERO
        };
        // Text positions are offsets from the integer centre of the document.
        let centre = |size: [u32; 2]| egui::vec2((size[0] / 2) as f32, (size[1] / 2) as f32);
        let delta = centre(old) * scale + shift - centre(target);
        let mut texts = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let mut style = layer.settings.text.clone();
            if let Some(t) = &mut style {
                let offset = egui::vec2(t.offset[0] as f32, t.offset[1] as f32) * scale + delta;
                t.transform(
                    [Vec2::X * scale.x, Vec2::Y * scale.y],
                    Vec2::ZERO,
                    delta,
                    target,
                );
                if !t.valid() || offset.round().abs().max_elem() > 8192. {
                    return Err("调整后的文字超出可编辑范围。");
                }
            }
            texts.push(style);
        }
        self.record_layer_change();
        for (layer, text) in self.layers.iter_mut().zip(texts) {
            layer.tiles[0] = match draft.mode {
                Mode::Canvas => {
                    let mut im = RgbaImage::new(w, h);
                    imageops::replace(&mut im, &layer.tiles[0], offset[0], offset[1]);
                    im
                }
                Mode::Image => resize_layer(&layer.tiles[0], target, draft.filter),
            };
            if let Some(masks) = &mut layer.settings.mask {
                let source = GrayImage::from_raw(old[0], old[1], std::mem::take(&mut masks[0]))
                    .expect("validated layer mask");
                masks[0] = match draft.mode {
                    Mode::Canvas => {
                        let mut im = GrayImage::from_pixel(w, h, Luma([255]));
                        imageops::replace(&mut im, &source, offset[0], offset[1]);
                        im
                    }
                    Mode::Image => filtered(&source, target, draft.filter),
                }
                .into_raw();
            }
            layer.settings.text = text;
        }
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[0].clone();
        (self.width, self.height) = (w, h);
        self.footprint = target;
        self.selection.clear();
        self.text_dialog = None;
        self.layer_detail = None;
        self.layer_drag = None;
        self.pan = Vec2::ZERO;
        self.invalidate_textures();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_canvas_edits_unlock_the_axes_and_preserve_the_target_when_toggled() {
        let mut draft = Draft::new(Mode::Canvas, [128, 64]);
        draft.linked = true;
        draft.set_relative(true);
        assert!(!draft.linked);
        assert_eq!(draft.display_value(0, draft.percent[0]), 0.);
        draft.set_display_value(0, 32.);
        assert_eq!(draft.output(), [160, 64]);
        draft.set_display_value(1, -16.);
        assert_eq!(draft.output(), [160, 48]);
        assert_eq!(draft.offset(), [16, -8]);

        draft.pixels = false;
        assert_eq!(draft.display_value(0, draft.percent[0]), 25.);
        assert_eq!(draft.display_value(1, draft.percent[1]), -25.);
        draft.set_display_value(0, -50.);
        assert_eq!(draft.output(), [64, 48]);
        draft.set_relative(false);
        assert!(!draft.linked);
        assert_eq!(draft.display_value(0, draft.percent[0]), 50.);
        draft.pixels = true;
        assert_eq!(draft.display_value(1, draft.percent[1]), 48.);
        draft.linked = true;
        draft.set_display_value(1, 96.);
        assert_eq!(draft.output(), [192, 96]);
        draft.set_relative(true);
        assert_eq!(draft.display_value(0, draft.percent[0]), 64.);
        assert_eq!(draft.display_value(1, draft.percent[1]), 32.);
        assert_eq!(draft.output(), [192, 96]);
    }

    #[test]
    fn canvas_changes_extent_and_preserves_hidden_layers_and_masks_through_undo() {
        let mut editor = Editor::default();
        let original = RgbaImage::from_fn(3, 2, |x, y| Rgba([x as u8, y as u8, 8, 133]));
        editor.init_image(&original);
        let mut hidden = editor.layers[0].clone();
        hidden.visible = false;
        hidden.locked = true;
        hidden.settings.mask = Some(vec![vec![0, 1, 2, 3, 4, 5]]);
        editor.layers.push(hidden);
        let before = editor.snapshot();
        let mut draft = Draft::new(Mode::Canvas, [3, 2]);
        draft.percent = [200., 50.];
        draft.anchor = [2, 2];
        editor.apply_image_size(&draft, 8192).unwrap();
        assert_eq!((editor.width, editor.height), (6, 1));
        assert_eq!(editor.pixels.get_pixel(0, 0).0, [0; 4]);
        assert_eq!(editor.pixels.get_pixel(3, 0), original.get_pixel(0, 1));
        assert_eq!(editor.layers[1].tiles[0], editor.pixels);
        assert_eq!(
            editor.layers[1].settings.mask.as_ref().unwrap()[0],
            [255, 255, 255, 3, 4, 5]
        );
        assert!(editor.layers[1].locked && !editor.layers[1].visible);
        assert!(editor.changed && editor.undo.len() == 1);
        let after = editor.snapshot();
        editor.history(false);
        assert!(editor.snapshot() == before);
        editor.history(true);
        assert!(editor.snapshot() == after);
    }

    #[test]
    fn image_resampling_preserves_nearest_pixels_and_avoids_hidden_rgb_in_smooth_modes() {
        let source = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 0])
            }
        });
        let mut editor = Editor::default();
        editor.init_image(&source);
        editor.layers[0].settings.mask = Some(vec![vec![41, 113]]);
        let mut draft = Draft::new(Mode::Image, [2, 1]);
        draft.percent = [200., 300.];
        draft.filter = FilterType::Nearest;
        editor.apply_image_size(&draft, 8192).unwrap();
        assert_eq!(editor.pixels.dimensions(), (4, 3));
        for (x, _, pixel) in editor.pixels.enumerate_pixels() {
            assert_eq!(pixel, source.get_pixel(x / 2, 0));
        }
        assert_eq!(
            editor.layers[0].settings.mask.as_ref().unwrap()[0],
            [41, 41, 113, 113].repeat(3)
        );
        for filter in [
            FilterType::Triangle,
            FilterType::CatmullRom,
            FilterType::Lanczos3,
        ] {
            let output = resize_layer(&source, [9, 1], filter);
            assert!(output.pixels().any(|p| p[3] > 0 && p[3] < 255));
            for p in output.pixels().filter(|p| p[3] > 0) {
                assert_eq!(&p.0[..3], &[255, 0, 0]);
            }
        }
    }

    #[test]
    fn draft_and_invalid_sizes_leave_pixels_and_history_alone() {
        let mut e = Editor::default();
        e.init_image(&RgbaImage::new(5, 3));
        let mut draft = Draft::new(Mode::Image, [5, 3]);
        e.apply_image_size(&draft, 8192).unwrap();
        draft.percent = [50.; 2];
        assert_eq!(draft.output(), [3, 2]);
        draft.pixels = true;
        assert_eq!(draft.output(), [3, 2]);
        assert!(e.validate_image_size([8192, 8192], 8192).is_err());
        assert!(e.apply_image_size(&draft, 2).is_err());
        assert_eq!(e.pixels.dimensions(), (5, 3));
        assert!(e.undo.is_empty() && !e.changed);
    }
}
