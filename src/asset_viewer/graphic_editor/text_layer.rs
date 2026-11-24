//! Editable rectangular Tile text. Store the rendered cell alongside the type settings
//! so projects keep their appearance when opened on a machine with different fonts.
use super::*;
pub(super) mod font_samples;
mod paragraph;
use crate::asset_viewer::svg_preview::tree_with_fonts;
use resvg::tiny_skia;
use std::fmt::Write;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Text {
    pub content: String,
    pub(super) family: String,
    size: u16,
    angle: i16,
    tracking: i16,
    line_height: u16,
    bold: bool,
    italic: bool,
    color: [u8; 3],
    pub offset: [i32; 2],
    #[serde(default="repeat_by_default")]
    pub(super) repeat:bool,
    #[serde(default = "identity")]
    linear: [f32; 4],
    #[serde(default)]
    paragraph: paragraph::Paragraph,
}
fn repeat_by_default()->bool{true}
fn identity() -> [f32; 4] {
    [1., 0., 0., 1.]
}

impl Text {
    fn new(size: u32, color: [u8; 4]) -> Self {
        Self {
            content: String::new(),
            family: "Microsoft YaHei".into(),
            size: (size / 5).clamp(2, 96) as u16,
            angle: -30,
            tracking: 0,
            line_height: 120,
            bold: false,
            italic: false,
            color: [color[0], color[1], color[2]],
            offset: [0; 2],repeat:true,
            linear: identity(),
            paragraph: Default::default(),
        }
    }
    pub(super) fn valid(&self) -> bool {
        !self.content.trim().is_empty()
            && self.content.chars().count() <= 512
            && self.content.split('\n').count() <= 16
            && self
                .content
                .chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            && !self.family.is_empty()
            && self.family.len() <= 512
            && (1..=256).contains(&self.size)
            && (-180..=180).contains(&self.angle)
            && (-32..=128).contains(&self.tracking)
            && (25..=300).contains(&self.line_height)
            && self.offset.iter().all(|v| (-8192..=8192).contains(v))
            && self
                .linear
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 4096.)
            && self.paragraph.valid()
    }
    pub(super) fn transform(
        &mut self,
        linear: [Vec2; 2],
        origin: Vec2,
        delta: Vec2,
        period: [u32; 2],
    ) {
        let apply = |v: Vec2| linear[0] * v.x + linear[1] * v.y;
        let [a, b, c, d] = self.linear;
        let x = apply(egui::vec2(a, b));
        let y = apply(egui::vec2(c, d));
        self.linear = [x.x, x.y, y.x, y.y];
        let position = origin
            + delta
            + apply(egui::vec2(self.offset[0] as f32, self.offset[1] as f32) - origin);
        let wrap = |v: f32, size: u32| {
            (v.round() as i32 + size as i32 / 2).rem_euclid(size as i32) - size as i32 / 2
        };
        self.offset = if self.repeat{[wrap(position.x, period[0]), wrap(position.y, period[1])]}else{[position.x.round().clamp(-8192.,8192.)as i32,position.y.round().clamp(-8192.,8192.)as i32]};
    }
    pub(super) fn raster(
        &self,
        w: u32,
        h: u32,
        fonts: &font_samples::Catalog,
    ) -> Result<RgbaImage, String> {
        if !self.valid() {
            return Err(tr("请输入 1 至 16 行文字，最多 512 个字符。").into());
        }
        if !fonts.contains(&self.family) {
            return Err(format!("{} {}", tr("缺少字体"), self.family));
        }
        let family = quick_xml::escape::escape(&self.family);
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><g transform="rotate({})" font-family="{family}" font-size="{}" font-weight="{}" font-style="{}" letter-spacing="{}" text-anchor="middle" xml:space="preserve">"#,
            self.angle,
            self.size,
            if self.bold { "bold" } else { "normal" },
            if self.italic { "italic" } else { "normal" },
            self.tracking
        );
        self.paragraph.write_lines(self, &fonts.db, &mut svg);
        svg.push_str("</g></svg>");
        let tree = resvg::Tree::from_usvg(&tree_with_fonts(svg.as_bytes(), &fonts.db)?);
        let bounds = tree
            .content_area
            .ok_or_else(|| tr("所选字体无法绘制这些文字。").to_owned())?;
        // Layout coordinates are relative to the watermark's paragraph origin.
        let [a, b, c, d] = self.linear;
        let matrix = tiny_skia::Transform::from_row(a, b, c, d, 0., 0.);
        let bounds = bounds
            .transform(matrix)
            .ok_or_else(|| tr("文字图像过大，请缩小字号或缩短文字。").to_owned())?;
        let left = bounds.left().floor() - 1.;
        let top = bounds.top().floor() - 1.;
        let sw = (bounds.right().ceil() + 1. - left) as u32;
        let sh = (bounds.bottom().ceil() + 1. - top) as u32;
        if u64::from(sw) * u64::from(sh) > 4 * 1024 * 1024 {
            return Err(tr("文字图像过大，请缩小字号或缩短文字。").into());
        }
        let mut stamp = tiny_skia::Pixmap::new(sw, sh)
            .ok_or_else(|| tr("所选字体无法绘制这些文字。").to_owned())?;
        tree.render(
            tiny_skia::Transform::from_translate(-left, -top).pre_concat(matrix),
            &mut stamp.as_mut(),
        );
        let start = [
            w as i32 / 2 + left as i32 + self.offset[0],
            h as i32 / 2 + top as i32 + self.offset[1],
        ];
        Ok(place_stamp(&stamp, start, w, h, self.color,self.repeat))
    }
}

// Fold every covered pixel onto the repeat cell, including glyphs wider than a cell.
fn place_stamp(
    stamp: &tiny_skia::Pixmap,
    start: [i32; 2],
    w: u32,
    h: u32,
    color: [u8; 3],repeat:bool,
) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for (y, row) in stamp
        .pixels()
        .chunks_exact(stamp.width() as usize)
        .enumerate()
    {
        let cy=start[1]+y as i32;
        if !repeat && !(0..h as i32).contains(&cy){continue;}
        let py=cy.rem_euclid(h as i32) as u32;
        for (x, pixel) in row.iter().enumerate() {
            let a = u32::from(pixel.alpha());
            if a == 0 {
                continue;
            }
            let cx=start[0]+x as i32;
            if !repeat && !(0..w as i32).contains(&cx){continue;}
            let px=cx.rem_euclid(w as i32)as u32;
            let old = out.get_pixel_mut(px, py);
            let alpha = a + ((255 - a) * u32::from(old[3]) + 127) / 255;
            *old = Rgba([color[0], color[1], color[2], alpha as u8]);
        }
    }
    out
}

pub(super) struct Dialog {
    pub(super) target: Option<usize>,
    paragraph_tab: bool,
    last_applied: Option<(Text, u8)>,
    draft: Text,
    opacity: u8,
    cached: Option<(Text, Result<RgbaImage, String>)>,
    texture: Option<egui::TextureHandle>,
    font_generation: u64,
}

pub(super) struct Inline {
    index: Option<usize>,
    original: Text,
    content: String,
    bounds: Rect,
    focus: bool,
    input_rect: Option<Rect>,
}

impl Editor {
    pub(super) fn text_tool_click(&mut self, point: Pos2, layout: bool, infinite: bool) {
        if !self.rectangle_text_allowed() || self.inline_text_hit(point, layout, infinite) {
            return;
        }
        let hit = if infinite {
            self.plane_hit(point.x as i32, point.y as i32, layout)
        } else {
            self.finite_plane_hit(point.x as i32, point.y as i32, layout)
        };
        let Some((_, x, y)) = hit else { return; };
        let (w, h) = self.pixels.dimensions();
        let bytes = self.tiles.iter().map(|tile| tile.as_raw().len()).sum();
        if !self.can_add_layers(&[], bytes) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        self.finish_stroke();
        self.layer_detail = None;
        self.text_dialog = None;
        let mut text = Text::new(w.min(h), self.color);
        text.angle = 0;
        text.repeat = !self.plain_image;
        text.offset = [x as i32 - (w / 2) as i32, y as i32 - (h / 2) as i32];
        self.inline_text = Some(Inline {
            index: None, original: text, content: String::new(),
            bounds: Rect::from_min_size(point, Vec2::splat(1.)), focus: true, input_rect: None,
        });
    }
    pub(super) fn inline_text_hit(&mut self, point: Pos2, layout: bool, infinite: bool) -> bool {
        if !self.rectangle_text_allowed() {
            return false;
        }
        let hit = if infinite {
            self.plane_hit(point.x as i32, point.y as i32, layout)
        } else {
            self.finite_plane_hit(point.x as i32, point.y as i32, layout)
        };
        let Some((tile, x, y)) = hit else { return false; };
        let index = self.layers.iter().enumerate().rev().find_map(|(i, layer)| {
            (layer.settings.text.is_some() && layer.visible && self.ancestors_visible(i)
                && layer.settings.opacity > 0 && layer.tiles[tile].get_pixel(x, y)[3] != 0)
                .then_some(i)
        });
        let Some(index) = index else { return false; };
        let cell = (tile, point.to_vec2() - egui::vec2(x as f32, y as f32));
        self.begin_inline_text(index, Some(cell));
        true
    }

    pub(super) fn begin_inline_text(&mut self, index: usize, cell: Option<(usize, Vec2)>) {
        if !self.rectangle_text_allowed() || !self.text_unlocked(index) || !self.finish_transform(true) {
            return;
        }
        let Some(text) = self.layers[index].settings.text.clone() else { return; };
        let (w, h) = self.pixels.dimensions();
        let size = egui::vec2(w as f32, h as f32);
        let (tile, offset) = cell.unwrap_or_else(|| {
            let anchor = (0..self.rows).flat_map(|row| (0..self.columns).map(move |col| (col, row)))
                .find(|&(col, row)| self.whole_unit() && self.tile_at(col as i32, row as i32) == self.selected)
                .unwrap_or((0, 0));
            (self.selected, self.mapped_center(anchor.0 as i32, anchor.1 as i32, w, h) - size * 0.5)
        });
        let mut bounds = Rect::NOTHING;
        for (x, y, pixel) in self.layers[index].tiles[tile].enumerate_pixels() {
            if pixel[3] != 0 {
                let p = (offset + egui::vec2(x as f32, y as f32)).to_pos2();
                bounds.extend_with(p);
                bounds.extend_with(p + Vec2::splat(1.));
            }
        }
        if !bounds.is_positive() {
            let center = offset + size * 0.5 + egui::vec2(text.offset[0] as f32, text.offset[1] as f32);
            bounds = Rect::from_center_size(center.to_pos2(), egui::vec2(48., text.size as f32));
        }
        self.select_layer(index);
        self.layer_detail = None;
        self.text_dialog = None;
        self.inline_text = Some(Inline {
            index: Some(index), content: text.content.clone(), original: text, bounds, focus: true, input_rect: None,
        });
    }

    pub(super) fn confirm_inline_text_outside(&mut self, ui: &egui::Ui) {
        let outside = self.inline_text.as_ref().and_then(|edit| edit.input_rect).is_some_and(|rect| {
            ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p)))
        });
        if outside && let Some(edit) = self.inline_text.take() {
            self.commit_inline_text(edit, ui.ctx());
        }
    }

    pub(super) fn inline_text_canvas(&mut self, ui: &egui::Ui, view: Rect, origin: Pos2, scale: f32) {
        let Some(mut edit) = self.inline_text.take() else { return; };
        if edit.index.is_some_and(|index| self.active_layer != index || !self.text_unlocked(index)
            || self.layers[index].settings.text.as_ref() != Some(&edit.original)) {
            return;
        }
        let position = origin + edit.bounds.min.to_vec2() * scale - Vec2::splat(3.);
        let width = (edit.bounds.width() * scale + 24.).max(100.).min((view.width() - 8.).max(1.));
        let first = edit.focus;
        let response = egui::Area::new(ui.id().with("canvas_text_edit"))
            .order(egui::Order::Foreground).movable(false).fixed_pos(position)
            .constrain_to(view)
            .show(ui.ctx(), |ui| {
                egui::Frame::new().fill(ui.visuals().extreme_bg_color)
                    .stroke(egui::Stroke::new(1_f32, ui.visuals().selection.stroke.color))
                    .inner_margin(3).show(ui, |ui| {
                        let input = ui.add(egui::TextEdit::multiline(&mut edit.content)
                            .font(egui::FontId::proportional((f32::from(edit.original.size) * scale).clamp(12., 96.)))
                            .desired_rows(1).desired_width(width).char_limit(512).frame(false));
                        if first { input.request_focus(); }
                    });
            }).response;
        edit.focus = false;
        edit.input_rect = Some(response.rect);
        if ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            return;
        }
        let confirmed = ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
        if !confirmed && (first || !response.clicked_elsewhere()) {
            self.inline_text = Some(edit);
            return;
        }
        self.commit_inline_text(edit, ui.ctx());
    }

    fn commit_inline_text(&mut self, mut edit: Inline, ctx: &egui::Context) {
        if edit.content != edit.original.content
            && !(edit.index.is_none() && edit.content.trim().is_empty())
            && edit.index.is_none_or(|index| self.text_unlocked(index)
                && self.layers[index].settings.text.as_ref() == Some(&edit.original)) {
            let (w, h) = self.pixels.dimensions();
            edit.original.content = edit.content;
            self.font_samples.tick(ctx);
            let result = self.font_samples.catalog.as_ref()
                .ok_or_else(|| tr("缺少字体").to_owned())
                .and_then(|fonts| edit.original.raster(w, h, fonts));
            match result {
                Ok(image) => {
                    let opacity = edit.index.map_or(100, |index| self.layers[index].settings.opacity);
                    self.apply_text_at(edit.index, edit.original, opacity, image, true);
                }
                Err(reason) => self.message = reason,
            }
        }
    }

    pub(super) fn rectangle_text_allowed(&self) -> bool {
        !self.diamond && !self.elevated && self.hex == 0
    }
    pub(super) fn text_unlocked(&self, i: usize) -> bool {
        self.layers
            .get(i)
            .is_some_and(|l| !l.locked && !l.settings.lock_pixels && !l.settings.lock_alpha)
            && !self.ancestor_locked(i)
            && !self.ancestor_paint_locked(i)
    }
    pub(super) fn begin_text(&mut self, target: Option<usize>) {
        if !self.rectangle_text_allowed() {
            return;
        }
        if target.is_none() {
            self.layer_detail = None;
        }
        self.inline_text = None;
        self.finish_stroke();
        let (draft, opacity, cached) = if let Some(i) = target {
            let Some(text) = self.layers[i].settings.text.clone() else {
                return;
            };
            self.select_layer(i);
            (
                text.clone(),
                self.layers[i].settings.opacity,
                Some((text, Ok(self.pixels.clone()))),
            )
        } else {
            let bytes = self.tiles.iter().map(|im| im.as_raw().len()).sum();
            if !self.can_add_layers(&[], bytes) {
                self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
                return;
            }
            let mut text=Text::new(self.pixels.width().min(self.pixels.height()),self.color);
            text.repeat=!self.plain_image;
            if self.plain_image{text.angle=0;}
            (text,if self.plain_image{100}else{35},None)
        };
        let last_applied = target.map(|_| (draft.clone(), opacity));
        self.text_dialog = Some(Dialog {
            target,
            draft,
            opacity,
            cached,
            texture: None,
            font_generation: 0,
            paragraph_tab: false,
            last_applied,
        });
    }
    fn apply_text(&mut self, target: Option<usize>, text: Text, opacity: u8, image: RgbaImage) {
        self.apply_text_at(target, text, opacity, image, false);
    }
    fn apply_text_at(&mut self, target: Option<usize>, text: Text, opacity: u8, image: RgbaImage, top: bool) {
        let index = if let Some(i) = target {
            if !self.text_unlocked(i) || self.layers[i].settings.text.is_none() {
                return;
            }
            if self.layers[i].settings.text.as_ref() == Some(&text)
                && self.layers[i].settings.opacity == opacity
            {
                return;
            }
            self.record_layer_change();
            if self.layers[i].settings.text.as_ref() != Some(&text) {
                self.layers[i].tiles.fill(image);
            }
            i
        } else {
            if !self.can_add_layers(&[], image.as_raw().len() * self.tiles.len()) {
                return;
            }
            self.record_layer_change();
            let (parent, index) = if top { (0, self.layers.len()) } else { self.layer_insert_position() };
            if let Some(group) = self
                .layers
                .iter_mut()
                .find(|l| parent != 0 && l.settings.group_id == parent)
            {
                group.settings.collapsed = false;
            }
            let name = text
                .content
                .lines()
                .next()
                .unwrap_or_default()
                .chars()
                .take(40)
                .collect();
            let settings = layer_properties::Properties {
                parent,
                ..Default::default()
            };
            self.layers.insert(
                index,
                Layer {
                    settings,
                    material: false,
                    visible: true,
                    locked: false,
                    name,
                    tiles: vec![image; self.tiles.len()],
                },
            );
            index
        };
        self.layers[index].settings.text = Some(text);
        self.layers[index].settings.opacity = opacity;
        self.active_layer = index;
        self.selected_layers = vec![index];
        self.layer_anchor = index;
        if target.is_none() {
            self.layer_detail = None;
        }
        self.mask_editing = false;
        self.tiles = self.layers[index].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }
    pub(super) fn rasterize_text(&mut self, index: usize) {
        if self.text_unlocked(index) && self.layers[index].settings.text.is_some() {
            self.record_layer_change();
            self.layers[index].settings.text = None;
        }
    }
    pub(super) fn text_dialog_ui(&mut self, ui: &egui::Ui) {
        let Some(mut dialog) = self.text_dialog.take() else {
            return;
        };
        let detail = dialog.target.and_then(|i| {
            self.layer_detail
                .as_ref()
                .filter(|(idx, fx, name, _)| {
                    *idx == i
                        && !*fx
                        && self.active_layer == i
                        && self
                            .layers
                            .get(i)
                            .is_some_and(|l| l.name == *name && l.settings.text.is_some())
                })
                .cloned()
        });
        if dialog.target.is_some() && detail.is_none() {
            return;
        }
        if let Some(i) = dialog.target {
            let p = &self.layers[i].settings;
            if let Some((last, opacity)) = &dialog.last_applied {
                if Some(last) != p.text.as_ref() || *opacity != p.opacity {
                    dialog.draft = p.text.clone().unwrap();
                    dialog.opacity = p.opacity;
                    dialog.texture = None;
                    dialog.cached = Some((dialog.draft.clone(), Ok(self.pixels.clone())));
                    dialog.last_applied = Some((dialog.draft.clone(), p.opacity));
                }
            }
        }
        let (w, h) = self.pixels.dimensions();
        self.font_samples.tick(ui.ctx());
        let fonts = self.font_samples.catalog.clone();
        let mut accept = false;
        let mut cancel = false;
        let existing = dialog.target.is_some();
        let editable =
            dialog.target.is_none_or(|i| self.text_unlocked(i)) && self.rectangle_text_allowed();
        let mut contents = |ui: &mut egui::Ui| {
            let preferred_width: f32 = if existing { 300. } else { 390. };
            ui.set_width(preferred_width.min((ui.ctx().screen_rect().width() - 32.).max(150.)));
            if !editable {
                ui.disable();
            }
            if !existing {
                ui.heading(tr("添加文字图层"));
            }
            ui.horizontal(|ui| {
                ui.selectable_value(&mut dialog.paragraph_tab, false, tr("字符"));
                ui.selectable_value(&mut dialog.paragraph_tab, true, tr("段落"));
            });
            egui::ScrollArea::vertical()
                .max_height((ui.ctx().screen_rect().height() - 114.).max(160.))
                .show(ui, |ui| {
                    let t = &mut dialog.draft;
                    if !existing {
                        let input = ui.add(
                        egui::TextEdit::multiline(&mut t.content)
                            .char_limit(512)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .hint_text(tr(if self.plain_image{"输入文字"}else{"输入水印文字"})),
                        );
                        if dialog.cached.is_none() && t.content.is_empty() {
                            input.request_focus();
                        }
                    }
                    if fonts.as_ref().is_some_and(|f| !f.contains(&t.family)) {
                        ui.colored_label(
                            Color32::YELLOW,
                            format!(
                                "{} {}\n{}",
                                tr("缺少字体"),
                                t.family,
                                tr("请选择替代字体，或安装原字体后刷新。")
                            ),
                        );
                    }
                    if dialog.paragraph_tab {
                        t.paragraph.controls(ui);
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("字体"));
                            self.font_samples.picker(ui, &mut t.family);
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.toggle_value(&mut t.bold, egui::RichText::new("B").strong())
                                .on_hover_text(tr("粗体"));
                            ui.toggle_value(&mut t.italic, egui::RichText::new("I").italics())
                                .on_hover_text(tr("斜体"));
                            ui.color_edit_button_srgb(&mut t.color);
                        });
                        egui::Grid::new("tile_text_metrics")
                            .num_columns(4)
                            .show(ui, |ui| {
                                ui.label(tr("字号"));
                                ui.add(
                                    egui::DragValue::new(&mut t.size)
                                        .range(1..=256)
                                        .suffix(" px"),
                                );
                                ui.label(tr("旋转"));
                                ui.add(
                                    egui::DragValue::new(&mut t.angle)
                                        .range(-180..=180)
                                        .suffix("°"),
                                );
                                ui.end_row();
                                ui.label(tr("字距"));
                                ui.add(
                                    egui::DragValue::new(&mut t.tracking)
                                        .range(-32..=128)
                                        .suffix(" px"),
                                );
                                ui.label(tr("行距"));
                                ui.add(
                                    egui::DragValue::new(&mut t.line_height)
                                        .range(25..=300)
                                        .suffix("%"),
                                );
                                ui.end_row();
                                for ((label, limit), value) in [("X", w as i32), ("Y", h as i32)]
                                    .into_iter()
                                    .zip(t.offset.iter_mut())
                                {
                                    ui.label(label);
                                    ui.add(
                                        egui::DragValue::new(value)
                                            .range(-limit..=limit)
                                            .clamp_existing_to_range(false)
                                            .suffix(" px"),
                                    );
                                }
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            ui.label(tr("不透明度"));
                            ui.add(egui::Slider::new(&mut dialog.opacity, 0..=100).suffix("%"));
                        });
                    }
                    if let Some(fonts) = &fonts {
                        if dialog
                            .cached
                            .as_ref()
                            .is_none_or(|(previous, _)| previous != t)
                            || (dialog.font_generation != self.font_samples.generation
                                && dialog.cached.as_ref().is_some_and(|(_, v)| v.is_err()))
                        {
                            dialog.cached = Some((t.clone(), t.raster(w, h, fonts)));
                            dialog.texture = None;
                        }
                    }
                    dialog.font_generation = self.font_samples.generation;
                    if let Some((_, Ok(image))) = &dialog.cached {
                        if !existing {
                            let texture = dialog.texture.get_or_insert_with(|| {
                                ui.ctx().load_texture(
                                    "tile-text-repeat",
                                    tile_preview_image(image),
                                    egui::TextureOptions::LINEAR,
                                )
                            });
                            let copies=if self.plain_image{1}else{3};
                            let cell = egui::vec2(w as f32, h as f32) * ((288./copies as f32) / w.max(h) as f32);
                            let (rect, _) = ui.allocate_exact_size(cell * copies as f32, egui::Sense::hover());
                            super::super::seamless::checker_background(
                                ui.painter(),
                                rect,
                                rect.min,
                                Vec2::splat(1.),
                                &super::super::ImageBackground::Checkerboard,
                                None,
                            );
                            let tint = Color32::from_white_alpha(
                                (u16::from(dialog.opacity) * 255 / 100) as u8,
                            );
                            for row in 0..copies {
                                for col in 0..copies {
                                    let min = rect.min
                                        + egui::vec2(cell.x * col as f32, cell.y * row as f32);
                                    ui.painter().image(
                                        texture.id(),
                                        Rect::from_min_size(min, cell),
                                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                                        tint,
                                    );
                                }
                            }
                        }
                    } else if !t.content.is_empty() {
                        if let Some((_, Err(message))) = &dialog.cached {
                            ui.label(message);
                        }
                    }
                });
            if !existing {
                ui.horizontal(|ui| {
                    accept = ui
                        .add_enabled(
                            fonts
                                .as_ref()
                                .is_some_and(|f| f.contains(&dialog.draft.family))
                                && matches!(&dialog.cached, Some((_, Ok(_)))),
                            egui::Button::new(tr("确定")),
                        )
                        .clicked();
                    cancel = ui.button(tr("取消")).clicked();
                });
            }
        };
        let closed = if let Some((i, _, name, position)) = detail {
            let mut open = true;
            egui::Window::new(name)
                .id(ui.id().with("tile_text_properties"))
                .open(&mut open)
                .collapsible(false)
                .auto_sized()
                .fixed_pos(position)
                .show(ui.ctx(), &mut contents);
            if editable
                && fonts
                    .as_ref()
                    .is_some_and(|f| f.contains(&dialog.draft.family))
                && dialog
                    .last_applied
                    .as_ref()
                    .is_some_and(|(style, opacity)| {
                        style != &dialog.draft || *opacity != dialog.opacity
                    })
            {
                if let Some((style, Ok(image))) = &dialog.cached {
                    self.apply_text(Some(i), style.clone(), dialog.opacity, image.clone());
                    dialog.last_applied = Some((style.clone(), dialog.opacity));
                }
            }
            if !open {
                self.layer_detail = None;
            }
            !open
        } else {
            egui::Modal::new(ui.id().with("tile_text"))
                .show(ui.ctx(), contents)
                .should_close()
        };
        if accept {
            if let Some((text, Ok(image))) = dialog.cached {
                self.apply_text(dialog.target, text, dialog.opacity, image);
            }
        } else if !cancel && !closed {
            self.text_dialog = Some(dialog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_wraps_both_axes_and_accumulates_overlapping_glyphs() {
        let mut stamp = tiny_skia::Pixmap::new(4, 2).unwrap();
        stamp.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 128));
        let im = place_stamp(&stamp, [-1, -1], 3, 3, [12, 34, 56],true);
        assert_eq!(im.get_pixel(2, 2).0, [12, 34, 56, 192]);
        assert_eq!(im.get_pixel(1, 0).0, [12, 34, 56, 128]);
        assert_eq!(im.get_pixel(1, 1)[3], 0);
        assert_eq!(im, place_stamp(&stamp, [5, -4], 3, 3, [12, 34, 56],true));
    }

    #[test]
    fn text_layer_is_editable_after_save_and_after_undoing_rasterization() {
        let mut editor = Editor::default();
        editor.diamond = false;
        editor.reset_document(RgbaImage::new(8, 8));
        let mut text = Text::new(8, [255; 4]);
        text.content = "A<&字".into();
        let mark = RgbaImage::from_fn(8, 8, |x, y| {
            if x == 0 && y == 0 {
                Rgba([255; 4])
            } else {
                Rgba([0; 4])
            }
        });
        editor.apply_text(None, text.clone(), 35, mark.clone());
        let i = editor.active_layer;
        let p: &layer_properties::Properties = &editor.layers[i].settings;
        let saved: layer_properties::Properties =
            serde_json::from_slice(&serde_json::to_vec(p).unwrap()).unwrap();
        assert!(saved == *p && saved.valid(1, 64));
        assert!(!editor.layer_editable());
        editor.rasterize_text(i);
        assert!(editor.layer_editable());
        editor.history(false);
        assert!(editor.layers[i].settings.text == Some(text));
        assert_eq!(editor.pixels, mark);
        let mut p = saved;
        // A stroke touching the left/top edge also appears on the other edges.
        p.effects =
            serde_json::from_value(serde_json::json!({"stroke":true,"stroke_size":1})).unwrap();
        p.fill = 0;
        p.opacity = 100;
        let mut out = RgbaImage::new(8, 8);
        p.composite(&mut out, &mark, 0, Default::default());
        assert_eq!(out.get_pixel(7, 0)[3], 255);
        assert_eq!(out.get_pixel(0, 7)[3], 255);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
    }
}
