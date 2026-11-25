//! Editable rectangular Tile text. Store the rendered cell alongside the type settings
//! so projects keep their appearance when opened on a machine with different fonts.
use super::*;
pub(super) mod font_samples;
mod paragraph;
mod character;
pub(super) mod psd;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) tile_origin: Option<[i32; 2]>,
    #[serde(default="repeat_by_default")]
    pub(super) repeat:bool,
    #[serde(default)]
    point_text: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    character_runs: Vec<character::Run>,
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
    fn without_color(&self) -> Self {
        let mut text = self.clone();
        character::apply_layer_style(&mut text, character::Style { color: Some([255; 3]), ..Default::default() });
        text
    }

    pub(super) fn new(size: u32, color: [u8; 4]) -> Self {
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
            offset: [0; 2],repeat:true,tile_origin:None,
            point_text: false,
            character_runs: Vec::new(),
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
            && character::valid(&self.character_runs, self.content.chars().count())
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
        self.offset = if self.repeat && self.tile_origin.is_none(){[wrap(position.x, period[0]), wrap(position.y, period[1])]}else{[position.x.round().clamp(-8192.,8192.)as i32,position.y.round().clamp(-8192.,8192.)as i32]};
    }
    pub(super) fn raster(
        &self,
        w: u32,
        h: u32,
        fonts: &font_samples::Catalog,
    ) -> Result<RgbaImage, String> {
        self.raster_with_layout(w, h, fonts).map(|(image, _)| image)
    }

    fn svg_layout(&self, w: u32, h: u32, fonts: &font_samples::Catalog) -> (String, TextLayout) {
        let family = quick_xml::escape::escape(&self.family);
        use resvg::usvg::fontdb::{Family, Query, Style, Weight};
        let native_italic = self.italic && fonts.db.query(&Query {
            families: &[Family::Name(&self.family)],
            weight: if self.bold { Weight::BOLD } else { Weight::NORMAL },
            style: Style::Italic,
            ..Default::default()
        }).and_then(|id| fonts.db.face(id)).is_some_and(|face| face.style != Style::Normal);
        let synthetic_slant = if self.italic && !native_italic { " skewX(-12)" } else { "" };
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><g transform="rotate({}){synthetic_slant}" font-family="{family}" font-size="{}" font-weight="{}" font-style="{}" letter-spacing="{}" text-anchor="middle" xml:space="preserve">"#,
            self.angle,
            self.size,
            if self.bold { "bold" } else { "normal" },
            if native_italic { "italic" } else { "normal" },
            self.tracking
        );
        let lines = self.paragraph.write_lines(self, &fonts.db, &mut svg);
        svg.push_str("</g></svg>");
        let [a, b, c, d] = self.linear;
        let matrix = tiny_skia::Transform::from_row(a, b, c, d, 0., 0.);
        let skew = if synthetic_slant.is_empty() { 0. } else { (-12_f32).to_radians().tan() };
        let layout = TextLayout {
            lines,
            transform: matrix.pre_concat(tiny_skia::Transform::from_rotate(f32::from(self.angle)))
                .pre_concat(tiny_skia::Transform::from_skew(skew, 0.)),
            origin: egui::vec2((w / 2) as f32 + self.offset[0] as f32, (h / 2) as f32 + self.offset[1] as f32),
        };
        (svg, layout)
    }

    fn raster_with_layout(
        &self,
        w: u32,
        h: u32,
        fonts: &font_samples::Catalog,
    ) -> Result<(RgbaImage, TextLayout), String> {
        let (stamp, start, layout) = self.stamp_with_layout(w, h, fonts)?;
        Ok((place_stamp(&stamp, start, w, h, self.repeat), layout))
    }

    fn stamp_with_layout(
        &self, w: u32, h: u32, fonts: &font_samples::Catalog,
    ) -> Result<(tiny_skia::Pixmap, [i32; 2], TextLayout), String> {
        if !self.valid() {
            return Err(tr("请输入 1 至 16 行文字，最多 512 个字符。").into());
        }
        let (svg, layout) = self.svg_layout(w, h, fonts);
        let tree = resvg::Tree::from_usvg(&tree_with_fonts(svg.as_bytes(), &fonts.db)?);
        let bounds = tree
            .content_area
            .ok_or_else(|| tr("所选字体无法绘制这些文字。").to_owned())?;
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
        Ok((stamp, start, layout))
    }
}

struct TextLayout {
    lines: Vec<paragraph::CaretLine>,
    transform: tiny_skia::Transform,
    origin: Vec2,
}

impl TextLayout {
    fn to_cell(&self, x: f32, y: f32) -> Pos2 {
        let mut point = tiny_skia::Point::from_xy(x, y);
        self.transform.map_point(&mut point);
        egui::pos2(point.x, point.y) + self.origin
    }

    fn index_at(&self, cell: Pos2) -> Option<usize> {
        let mut point = tiny_skia::Point::from_xy(cell.x - self.origin.x, cell.y - self.origin.y);
        self.transform.invert()?.map_point(&mut point);
        let line = self.lines.iter().min_by(|a, b| {
            let distance = |line: &paragraph::CaretLine| {
                (point.y - point.y.clamp(line.baseline - line.ascent, line.baseline + line.descent)).abs()
            };
            distance(a).total_cmp(&distance(b))
        })?;
        line.columns.iter().enumerate()
            .min_by(|(_, a), (_, b)| (point.x - **a).abs().total_cmp(&(point.x - **b).abs()))
            .map(|(column, _)| line.start + column)
    }
}

fn selected_characters(range: egui::text::CCursorRange) -> std::ops::Range<usize> {
    let [start, end] = range.sorted();
    start.index..end.index
}

fn paint_text_caret(
    ui: &egui::Ui,
    layout: &TextLayout,
    cursor: Option<egui::text::CCursorRange>,
    style: egui::style::TextCursorStyle,
    origin: Pos2,
    scale: f32,
) {
    let screen = |x, y| origin + layout.to_cell(x, y).to_vec2() * scale;
    let baseline = egui::Stroke::new(1_f32, style.stroke.color);
    let selected = cursor.map(selected_characters);
    for line in &layout.lines {
        let first = line.columns[0];
        let last = *line.columns.last().unwrap();
        let end = line.start + line.columns.len() - 1;
        ui.painter().line_segment(
            [screen(first, line.baseline), screen(last.max(first + 4. / scale), line.baseline)],
            baseline,
        );
        if let Some(range) = &selected {
            let start = range.start.max(line.start);
            let stop = range.end.min(end);
            if start < stop {
                let left = line.columns[start - line.start];
                let right = line.columns[stop - line.start];
                let top = line.baseline - line.ascent;
                let bottom = line.baseline + line.descent;
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![screen(left, top), screen(right, top), screen(right, bottom), screen(left, bottom)],
                    style.stroke.color.gamma_multiply(0.3),
                    egui::Stroke::NONE,
                ));
            }
        }
    }
    let Some(cursor) = cursor.map(|range| range.primary.index) else { return; };
    if style.blink {
        let period = f64::from(style.on_duration + style.off_duration).max(0.01);
        let phase = ui.input(|input| input.time).rem_euclid(period);
        let visible = phase < f64::from(style.on_duration);
        let next = if visible { f64::from(style.on_duration) - phase } else { period - phase };
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(next.max(0.01)));
        if !visible {
            return;
        }
    }
    if let Some((line, x)) = layout.lines.iter().rev().find_map(|line| {
        cursor.checked_sub(line.start).and_then(|index| line.columns.get(index)).map(|x| (line, *x))
    }) {
        ui.painter().line_segment(
            [screen(x, line.baseline - line.ascent), screen(x, line.baseline + line.descent)],
            style.stroke,
        );
    }
}

// Fold every covered pixel onto the repeat cell, including glyphs wider than a cell.
fn place_stamp(
    stamp: &tiny_skia::Pixmap,
    start: [i32; 2],
    w: u32,
    h: u32,
    repeat:bool,
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
            composite_stamp_pixel(out.get_pixel_mut(px, py), *pixel);
        }
    }
    out
}
fn composite_stamp_pixel(old: &mut Rgba<u8>, pixel: tiny_skia::PremultipliedColorU8) {
    let a = u32::from(pixel.alpha());
    let background_alpha = ((255 - a) * u32::from(old[3]) + 127) / 255;
    let alpha = a + background_alpha;
    let source = [pixel.red(), pixel.green(), pixel.blue()];
    let color: [u8; 3] = std::array::from_fn(|channel| {
        ((u32::from(source[channel]) * 255 + u32::from(old[channel]) * background_alpha + alpha / 2) / alpha).min(255) as u8
    });
    *old = Rgba([color[0], color[1], color[2], alpha as u8]);
}

pub(super) struct Dialog {
    pub(super) target: Option<usize>,
    paragraph_tab: bool,
    last_applied: Option<(Text, u8)>,
    last_preview_update: Option<std::time::Instant>,
    opacity_drag_recorded: bool,
    color_preview_recorded: bool,
    draft: Text,
    opacity: u8,
    cached: Option<(Text, Result<RgbaImage, String>)>,
    texture: Option<egui::TextureHandle>,
    font_generation: u64,
}

pub(super) struct Inline {
    pub(super) index: Option<usize>,
    original: Text,
    content: String,
    runs: Vec<character::Run>,
    bounds: Rect,
    cell_offset: Vec2,
    pub(super) preview: Option<Vec<RgbaImage>>,
    preview_style: Option<(Text, u64)>,
    layout: Option<TextLayout>,
    mouse_anchor: Option<usize>,
    focus: bool,
    input_rect: Option<Rect>,
}

#[derive(PartialEq, Eq)]
struct GradientGrid {
    diamond: bool,
    hex: u8,
    rows: usize,
    columns: usize,
    layout: Vec<usize>,
}

pub(super) struct GradientStamp {
    text: Text,
    fonts: std::sync::Arc<font_samples::Catalog>,
    cell: [u32; 2],
    stamp: tiny_skia::Pixmap,
    start: [i32; 2],
    bounds: [u32; 4],
    grid: GradientGrid,
    coordinates: std::collections::HashMap<usize, std::sync::Arc<[u32]>>,
}

pub(super) struct TextGradient {
    pub bounds: [u32; 4],
    width: u32,
    coordinates: std::sync::Arc<[u32]>,
}

impl TextGradient {
    pub fn point(&self, index: usize) -> Option<[u32; 2]> {
        let offset = self.coordinates[index];
        (offset != u32::MAX).then(|| [offset % self.width, offset / self.width])
    }
}

fn text_pixel_bounds(image: &RgbaImage) -> Rect {
    let mut bounds = Rect::NOTHING;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] != 0 {
            let point = egui::pos2(x as f32, y as f32);
            bounds.extend_with(point);
            bounds.extend_with(point + Vec2::splat(1.));
        }
    }
    bounds
}

impl Editor {
    fn anchor_tile_text(&self, text: &mut Text, origin: Vec2) {
        if text.tile_origin.is_none() && !self.plain_image && (self.multiple || self.diamond || self.hex != 0) {
            text.tile_origin = Some([origin.x.round() as i32, origin.y.round() as i32]);
        }
    }

    fn text_tile_images(
        &self, text: &Text, fonts: &font_samples::Catalog,
    ) -> Result<(Vec<RgbaImage>, TextLayout), String> {
        let (w, h) = self.pixels.dimensions();
        let (stamp, start, layout) = text.stamp_with_layout(w, h, fonts)?;
        let Some(origin) = text.tile_origin.filter(|_| !self.plain_image) else {
            return Ok((vec![place_stamp(&stamp, start, w, h, text.repeat); self.tiles.len()], layout));
        };
        let mut tiles = vec![RgbaImage::new(w, h); self.tiles.len()];
        let layout_mode = self.multiple;
        for (i, &pixel) in stamp.pixels().iter().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let x = origin[0].clamp(-32768, 32768) + start[0] + (i % stamp.width() as usize) as i32;
            let y = origin[1].clamp(-32768, 32768) + start[1] + (i / stamp.width() as usize) as i32;
            let hit = self.geometry_hit(x, y, layout_mode, self.wrap && text.repeat);
            if let Some((tile, px, py)) = hit {
                composite_stamp_pixel(tiles[tile].get_pixel_mut(px, py), pixel);
            }
        }
        Ok((tiles, layout))
    }

    pub(super) fn wrapped_text_gradient(&self, text: &Text, tile: usize) -> Option<TextGradient> {
        if self.plain_image || !self.wrap || !text.repeat || !self.multiple {
            return None;
        }
        let origin = text.tile_origin?;
        let fonts = self.font_samples.catalog.as_ref()?;
        let (w, h) = self.pixels.dimensions();
        let count = w as usize * h as usize;
        if count > 4 * 1024 * 1024 { return None; }
        let geometry = text.without_color();
        let mut cache = self.text_gradient_stamp.borrow_mut();
        if cache.as_ref().is_none_or(|cached| {
            cached.cell != [w, h] || cached.text != geometry || !std::sync::Arc::ptr_eq(fonts, &cached.fonts)
        }) {
            let (stamp, start, _) = geometry.stamp_with_layout(w, h, fonts).ok()?;
            let sw = stamp.width();
            let mut bounds = [sw, stamp.height(), 0, 0];
            for (i, _) in stamp.pixels().iter().enumerate().filter(|(_, pixel)| pixel.alpha() != 0) {
                let x = i as u32 % sw;
                let y = i as u32 / sw;
                bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x), bounds[3].max(y)];
            }
            *cache = Some(GradientStamp { text: geometry, fonts: fonts.clone(), cell: [w, h], stamp, start, bounds,
                grid: GradientGrid { diamond: self.diamond, hex: self.hex, rows: self.rows, columns: self.columns, layout: self.layout.clone() },
                coordinates: Default::default(),
            });
        }
        let cached = cache.as_mut()?;
        let sw = cached.stamp.width();
        if cached.bounds[0] == sw { return None; }
        if cached.grid.diamond != self.diamond || cached.grid.hex != self.hex
            || cached.grid.rows != self.rows || cached.grid.columns != self.columns || cached.grid.layout != self.layout {
            cached.grid = GradientGrid { diamond: self.diamond, hex: self.hex, rows: self.rows,
                columns: self.columns, layout: self.layout.clone() };
            cached.coordinates.clear();
        }
        if let Some(coordinates) = cached.coordinates.get(&tile) {
            return Some(TextGradient { bounds: cached.bounds, width: sw, coordinates: coordinates.clone() });
        }
        let mut coordinates = vec![u32::MAX; count];
        let base = [origin[0].clamp(-32768, 32768) + cached.start[0],
            origin[1].clamp(-32768, 32768) + cached.start[1]];
        for (i, pixel) in cached.stamp.pixels().iter().enumerate() {
            if pixel.alpha() == 0 { continue; }
            let x = base[0] + (i as u32 % sw) as i32;
            let y = base[1] + (i as u32 / sw) as i32;
            if let Some((at, px, py)) = self.geometry_hit(x, y, true, true) && at == tile {
                coordinates[(py * w + px) as usize] = i as u32;
            }
        }
        let coordinates: std::sync::Arc<[u32]> = coordinates.into();
        if (cached.coordinates.len() + 1) * count <= 8 * 1024 * 1024 {
            cached.coordinates.insert(tile, coordinates.clone());
        }
        Some(TextGradient { bounds: cached.bounds, width: sw, coordinates })
    }

    pub(super) fn render_text_tiles(&self, text: &Text, fonts: &font_samples::Catalog) -> Result<Vec<RgbaImage>, String> {
        self.text_tile_images(text, fonts).map(|(tiles, _)| tiles)
    }

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
        text.point_text = true;
        text.repeat = !self.plain_image;
        text.offset = [x as i32 - (w / 2) as i32, y as i32 - (h / 2) as i32];
        self.anchor_tile_text(&mut text, point.to_vec2() - egui::vec2(x as f32, y as f32));
        self.inline_text = Some(Inline {
            index: None, original: text, content: String::new(), runs: Vec::new(),
            bounds: Rect::from_min_size(point, Vec2::splat(1.)),
            cell_offset: point.to_vec2() - egui::vec2(x as f32, y as f32),
            preview: None, preview_style: None, layout: None, mouse_anchor: None, focus: true, input_rect: None,
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
                && layer.settings.opacity > 0 && text_pixel_bounds(&layer.tiles[tile])
                    .contains(egui::pos2(x as f32 + 0.5, y as f32 + 0.5))).then_some(i)
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
        if self.layers[index].settings.psd_native_text {
            let fonts = self.font_samples.catalog.as_deref();
            let missing = std::iter::once(text.family.as_str())
                .chain(text.content.chars().enumerate().filter_map(|(i, _)| {
                    character::get(&text.character_runs, i)?.family.as_deref()
                }))
                .find(|family| fonts.is_some_and(|fonts| !fonts.contains(family)));
            let differs = fonts
                .and_then(|fonts| text.raster(w, h, fonts).ok())
                .is_none_or(|image| image != self.layers[index].tiles[self.selected]);
            if let Some(family) = missing {
                self.message = format!("{} {family} · {}", tr("缺少字体"), tr("编辑后文字外观可能与 PSD 原预览不同。"));
            } else if differs {
                self.message = tr("编辑后文字外观可能与 PSD 原预览不同。").into();
            }
        }
        let size = egui::vec2(w as f32, h as f32);
        let (tile, offset) = cell.unwrap_or_else(|| {
            let anchor = (0..self.rows).flat_map(|row| (0..self.columns).map(move |col| (col, row)))
                .find(|&(col, row)| self.whole_unit() && self.tile_at(col as i32, row as i32) == self.selected)
                .unwrap_or((0, 0));
            (self.selected, self.mapped_center(anchor.0 as i32, anchor.1 as i32, w, h) - size * 0.5)
        });
        let mut bounds = text_pixel_bounds(&self.layers[index].tiles[tile]).translate(offset);
        if !bounds.is_positive() {
            let center = offset + size * 0.5 + egui::vec2(text.offset[0] as f32, text.offset[1] as f32);
            bounds = Rect::from_center_size(center.to_pos2(), egui::vec2(48., text.size as f32));
        }
        self.select_layer(index);
        if !self.text_dialog.as_ref().is_some_and(|dialog| dialog.target == Some(index)) {
            self.layer_detail = None;
            self.text_dialog = None;
        }
        let offset = text.tile_origin.map_or(offset, |origin| egui::vec2(origin[0] as f32, origin[1] as f32));
        self.inline_text = Some(Inline {
            index: Some(index), content: text.content.clone(), runs: text.character_runs.clone(),
            original: text, bounds, cell_offset: offset, preview: None, preview_style: None,
            layout: None, mouse_anchor: None,
            focus: true, input_rect: None,
        });
    }

    pub(super) fn confirm_inline_text_outside(&mut self, ui: &egui::Ui) {
        if ui.memory(|memory| memory.any_popup_open()) { return; }
        let outside = self.inline_text.as_ref().and_then(|edit| edit.input_rect).is_some_and(|rect| {
            ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p)))
        });
        if outside && let Some(edit) = self.inline_text.take() {
            if edit.preview.is_some() {
                self.invalidate_textures();
            }
            self.commit_inline_text(edit, ui.ctx());
        }
    }

    pub(super) fn inline_text_canvas(&mut self, ui: &egui::Ui, view: Rect, origin: Pos2, scale: f32) {
        let Some(mut edit) = self.inline_text.take() else { return; };
        if edit.index.is_some_and(|index| self.active_layer != index || !self.text_unlocked(index)
            || self.layers[index].settings.text.as_ref() != Some(&edit.original)) {
            return;
        }
        let position = origin + edit.bounds.min.to_vec2() * scale;
        let width = (edit.bounds.width() * scale + 12.).max(16.);
        let font = egui::FontId::proportional((f32::from(edit.original.size) * scale).clamp(6., 192.));
        let [r, g, b] = edit.original.color;
        let normal = Color32::from_rgb(r, g, b);
        let alignment = edit.original.paragraph.editor_alignment();
        let first = edit.focus;
        let id = ui.id().with("canvas_text_edit");
        let input_id = id.with("input");
        self.font_samples.tick(ui.ctx());
        let popup_was_open = ui.memory(|memory| memory.any_popup_open());
        let response = egui::Area::new(id)
            .order(egui::Order::Foreground).movable(false).fixed_pos(position)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.shrink_clip_rect(view);
                let cursor_style = ui.visuals().text_cursor.clone();
                if edit.layout.is_some() {
                    ui.visuals_mut().text_cursor.stroke.color = Color32::TRANSPARENT;
                    ui.visuals_mut().selection.bg_fill = Color32::TRANSPARENT;
                } else {
                    ui.visuals_mut().selection.bg_fill = normal.gamma_multiply(0.35);
                }
                let selection = (!first).then(|| egui::TextEdit::load_state(ui.ctx(), input_id))
                    .flatten().and_then(|state| state.cursor.char_range())
                    .map(|range| {
                        let a = range.primary.index;
                        let b = range.secondary.index;
                        a.min(b)..a.max(b)
                    });
                let previous = edit.content.clone();
                let mut layouter = |ui: &egui::Ui, content: &str, _width: f32| {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = f32::INFINITY;
                    let lines: Vec<_> = content.split_inclusive('\n').collect();
                    let widths = ui.fonts(|fonts| lines.iter().map(|line|
                        fonts.layout_no_wrap(line.trim_end_matches('\n').to_owned(), font.clone(), normal).size().x
                    ).collect::<Vec<_>>());
                    let longest = widths.iter().copied().fold(0_f32, f32::max);
                    let mut index = 0;
                    for (line, line_width) in lines.into_iter().zip(widths) {
                        let mut leading = match alignment {
                            egui::Align::Min => 0.,
                            egui::Align::Center => (longest - line_width) / 2.,
                            egui::Align::Max => longest - line_width,
                        };
                        for character in line.chars() {
                            let mut bytes = [0; 4];
                            let style = character::get(&edit.runs, index);
                            let size = style.and_then(|s| s.size).unwrap_or(edit.original.size);
                            job.append(character.encode_utf8(&mut bytes), leading, egui::TextFormat::simple(
                                egui::FontId::proportional((f32::from(size) * scale).clamp(6.,192.)), Color32::TRANSPARENT,
                            ));
                            leading = 0.;
                            index += 1;
                        }
                    }
                    if content.is_empty() {
                        job.append("", 0., egui::TextFormat::simple(font.clone(), normal));
                    }
                    ui.fonts(|fonts| fonts.layout_job(job))
                };
                let mut output = egui::TextEdit::multiline(&mut edit.content)
                    .id(input_id).font(font.clone()).desired_rows(1).desired_width(width)
                    .char_limit(512).margin(0).frame(false).clip_text(false).layouter(&mut layouter)
                    .show(ui);
                if first { output.response.request_focus(); }
                if output.response.changed() {
                    character::edit(&mut edit.runs, &previous, &edit.content);
                }
                let mut cursor_range = output.state.cursor.char_range();
                if let Some(layout) = &edit.layout {
                    let (pointer, pressed, held, shift) = ui.input(|i| (
                        i.pointer.interact_pos(), i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers.shift,
                    ));
                    if let Some(pointer) = pointer
                        && (pressed && output.response.hovered() || held && edit.mouse_anchor.is_some())
                        && let Some(index) = layout.index_at(((pointer - origin) / scale - edit.cell_offset).to_pos2())
                    {
                        let anchor = if pressed && shift {
                            cursor_range.map_or(index, |r| r.secondary.index)
                        } else {
                            edit.mouse_anchor.unwrap_or(index)
                        };
                        edit.mouse_anchor = Some(anchor);
                        let range = egui::text::CCursorRange {
                            primary: egui::text::CCursor::new(index),
                            secondary: egui::text::CCursor::new(anchor),
                        };
                        output.state.cursor.set_char_range(Some(range));
                        output.state.store(ui.ctx(), input_id);
                        cursor_range = Some(range);
                    }
                    if !held {
                        edit.mouse_anchor = None;
                    }
                    if !output.response.changed() {
                        paint_text_caret(ui, layout, cursor_range, cursor_style, origin + edit.cell_offset * scale, scale);
                    }
                }
                let range = cursor_range.map(selected_characters).or(selection);
                if let Some(range) = range.filter(|r| !r.is_empty() && r.end <= edit.content.chars().count()) {
                    character::controls(ui, &mut self.font_samples, &mut edit, range);
                }
            }).response;
        edit.focus = false;
        edit.input_rect = Some(response.rect);
        if ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if edit.preview.is_some() {
                self.invalidate_textures();
            }
            return;
        }
        let confirmed = ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
        if !confirmed && (first || popup_was_open || ui.memory(|memory| memory.any_popup_open()) || !response.clicked_elsewhere()) {
            self.refresh_inline_preview(ui.ctx(), &mut edit);
            self.inline_text = Some(edit);
            return;
        }
        if edit.preview.is_some() {
            self.invalidate_textures();
        }
        self.commit_inline_text(edit, ui.ctx());
    }

    fn refresh_inline_preview(&mut self, ctx: &egui::Context, edit: &mut Inline) {
        let mut style = edit.original.clone();
        style.content.clone_from(&edit.content);
        style.character_runs.clone_from(&edit.runs);
        if style != edit.original {
            self.anchor_tile_text(&mut style, edit.cell_offset);
        }
        let generation = self.font_samples.generation;
        if edit.preview_style.as_ref().is_some_and(|(old, fonts)| *old == style && *fonts == generation) {
            return;
        }
        let (w, h) = self.pixels.dimensions();
        let fonts = self.font_samples.catalog.as_deref();
        if style == edit.original && edit.index.is_some() {
            edit.preview = None;
            edit.layout = fonts.map(|fonts| style.svg_layout(w, h, fonts).1);
            if let Some(image) = edit.index.and_then(|index| self.layers.get(index))
                .and_then(|layer| layer.tiles.get(self.selected))
            {
                let bounds = text_pixel_bounds(image);
                if bounds.is_positive() && self.plain_image {
                    edit.bounds = bounds.translate(edit.cell_offset);
                }
            }
        } else if style.content.is_empty() {
            edit.preview = Some(vec![RgbaImage::new(w, h); self.tiles.len()]);
            edit.layout = fonts.map(|fonts| style.svg_layout(w, h, fonts).1);
        } else if let Some(fonts) = fonts {
            if let Ok((image, layout)) = self.text_tile_images(&style, fonts) {
                edit.preview = Some(image);
                edit.layout = Some(layout);
            } else {
                edit.preview = None;
                edit.layout = None;
            }
        }
        if let Some(preview) = edit.preview.as_ref().and_then(|tiles| tiles.get(self.selected)) {
            let bounds = text_pixel_bounds(preview);
            if bounds.is_positive() && self.plain_image {
                edit.bounds = bounds.translate(edit.cell_offset);
            }
        }
        edit.preview_style = Some((style, generation));
        self.invalidate_textures();
        ctx.request_repaint();
    }

    fn commit_inline_text(&mut self, mut edit: Inline, ctx: &egui::Context) {
        if (edit.content != edit.original.content || edit.runs != edit.original.character_runs)
            && !(edit.index.is_none() && edit.content.trim().is_empty())
            && edit.index.is_none_or(|index| self.text_unlocked(index)
                && self.layers[index].settings.text.as_ref() == Some(&edit.original)) {
            edit.original.content = edit.content;
            edit.original.character_runs = edit.runs;
            self.anchor_tile_text(&mut edit.original, edit.cell_offset);
            self.font_samples.tick(ctx);
            let valid_preview = edit.preview_style.as_ref().is_some_and(|(style, generation)| {
                *style == edit.original && *generation == self.font_samples.generation
            }) && edit.original.valid();
            let result = if let Some(preview) = edit.preview.take().filter(|_| valid_preview) {
                Ok(preview)
            } else {
                self.font_samples.catalog.as_ref()
                    .ok_or_else(|| tr("缺少字体").to_owned())
                    .and_then(|fonts| self.text_tile_images(&edit.original, fonts).map(|(tiles, _)| tiles))
            };
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
        !self.elevated
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
        if !self.inline_text.as_ref().is_some_and(|edit| edit.index == target && target.is_some()) {
            self.inline_text = None;
        }
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
            last_preview_update: None,
            opacity_drag_recorded: false,
            color_preview_recorded: false,
        });
    }
    fn apply_text(&mut self, target: Option<usize>, text: Text, opacity: u8, image: RgbaImage) {
        let tiles = if text.tile_origin.is_some() && !self.plain_image {
            match self.font_samples.catalog.as_ref().ok_or_else(|| tr("缺少字体").to_owned())
                .and_then(|fonts| self.text_tile_images(&text, fonts))
            {
                Ok((tiles, _)) => tiles,
                Err(error) => { self.message = error; return; }
            }
        } else {
            vec![image; self.tiles.len()]
        };
        self.apply_text_at(target, text, opacity, tiles, false);
    }
    fn apply_text_at(&mut self, target: Option<usize>, text: Text, opacity: u8, tiles: Vec<RgbaImage>, top: bool) {
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
                self.layers[i].tiles = tiles;
            }
            i
        } else {
            if !self.can_add_layers(&[], tiles.iter().map(|image| image.as_raw().len()).sum()) {
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
                    tiles,
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
        let mut color_picker_open = false;
        let mut opacity_dragging = false;
        let mut opacity_released = false;
        let existing = dialog.target.is_some();
        let inline_style = self.inline_text.as_ref().filter(|edit| existing && edit.index == dialog.target)
            .map(|edit| character::layer_style(&edit.original, &edit.content, &edit.runs));
        let mut inline_change = None;
        let editable =
            dialog.target.is_none_or(|i| self.text_unlocked(i)) && self.rectangle_text_allowed();
        let mut contents = |ui: &mut egui::Ui| {
            let preferred_width: f32 = if existing { 252. } else { 390. };
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
                        let effective = inline_style.clone()
                            .unwrap_or_else(|| character::layer_style(t, &t.content, &t.character_runs));
                        let mut family = effective.family.clone().unwrap_or_default();
                        let mut size = effective.size.unwrap_or(0);
                        let mut color = None;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("字体"));
                            self.font_samples.picker(ui, &mut family);
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("颜色"));
                            color = character::color_control(ui, effective.color, t.color);
                            color_picker_open = crate::color_property::rgb_picker_open(ui, tr("颜色"));
                            ui.label(tr("样式"));
                            ui.toggle_value(&mut t.bold, egui::RichText::new("B").strong())
                                .on_hover_text(tr("粗体"));
                            ui.toggle_value(&mut t.italic, egui::RichText::new("I").italics())
                                .on_hover_text(tr("斜体"));
                        });
                        egui::Grid::new("tile_text_metrics")
                            .num_columns(4)
                            .show(ui, |ui| {
                                ui.label(tr("字号"));
                                ui.add(
                                    egui::DragValue::new(&mut size)
                                        .range(1..=256)
                                        .clamp_existing_to_range(false)
                                        .custom_formatter(|value, _| {
                                            if value == 0. { String::new() } else { format!("{value:.0} px") }
                                        }),
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
                        let change = character::Style {
                            family: (!family.is_empty() && effective.family.as_deref() != Some(&family)).then_some(family),
                            size: (size > 0 && effective.size != Some(size)).then_some(size),
                            color,
                        };
                        if change != character::Style::default() {
                            if inline_style.is_some() {
                                inline_change = Some(change);
                            } else {
                                character::apply_layer_style(t, change);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(tr("不透明度"));
                            let response = crate::theme::slider(ui, true, egui::Slider::new(&mut dialog.opacity, 0..=100).suffix("%"));
                            opacity_dragging = response.dragged();
                            opacity_released = response.drag_stopped();
                        });
                    }
                    if !existing && let Some(fonts) = &fonts {
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
                        if !existing && let Some((_, Err(message))) = &dialog.cached {
                            ui.label(message);
                        }
                    }
                });
            if !existing {
                ui.horizontal(|ui| {
                    cancel = ui.button(tr("取消")).clicked();
                    accept = ui
                        .add_enabled(
                            fonts
                                .as_ref()
                                .is_some_and(|f| f.contains(&dialog.draft.family))
                                && matches!(&dialog.cached, Some((_, Ok(_)))),
                            egui::Button::new(tr("确定")),
                        )
                        .clicked();
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
                && dialog
                    .last_applied
                    .as_ref()
                    .is_some_and(|(style, opacity)| {
                        style != &dialog.draft || *opacity != dialog.opacity
                    })
            {
                let interval = std::time::Duration::from_millis(33);
                let now = std::time::Instant::now();
                let remaining = dialog.last_preview_update
                    .map_or(std::time::Duration::ZERO, |last| interval.saturating_sub(now - last));
                if open && !opacity_released && !remaining.is_zero() {
                    ui.ctx().request_repaint_after(remaining);
                } else {
                    dialog.last_preview_update = Some(now);
                    let style = &dialog.draft;
                    if self.layers[i].settings.text.as_ref() == Some(style) {
                        if self.text_unlocked(i) {
                            if !dialog.opacity_drag_recorded {
                                self.record_layer_change();
                                dialog.opacity_drag_recorded = opacity_dragging;
                            }
                            self.layers[i].settings.opacity = dialog.opacity;
                            self.dirty_texture = true;
                            self.dirty_all = true;
                            dialog.last_applied = Some((style.clone(), dialog.opacity));
                        }
                    } else if let Some(rgb) = self.layers[i].settings.text.as_ref().and_then(|previous| {
                        let rgb = character::layer_style(style, &style.content, &style.character_runs).color?;
                        (previous.without_color() == style.without_color()).then_some(rgb)
                    }) {
                        if self.text_unlocked(i) {
                            if !dialog.color_preview_recorded {
                                self.record_layer_change();
                                dialog.color_preview_recorded = color_picker_open;
                            }
                            let recolor = |image: &mut RgbaImage| {
                                for pixel in image.pixels_mut().filter(|pixel| pixel[3] != 0) {
                                    pixel.0[..3].copy_from_slice(&rgb);
                                }
                            };
                            for image in &mut self.layers[i].tiles { recolor(image); }
                            if i == self.active_layer {
                                for image in &mut self.tiles { recolor(image); }
                                recolor(&mut self.pixels);
                            }
                            self.layers[i].settings.text = Some(style.clone());
                            self.layers[i].settings.opacity = dialog.opacity;
                            self.dirty_texture = true;
                            self.dirty_all = true;
                            if let Some(thumbnail) = self.layer_thumbnails.get_mut(i) { *thumbnail = None; }
                            dialog.last_applied = Some((style.clone(), dialog.opacity));
                        }
                    } else {
                        let result = if let Some(fonts) = &fonts {
                            if style.tile_origin.is_some() && !self.plain_image {
                                self.render_text_tiles(style, fonts)
                            } else {
                                style.raster(w, h, fonts).map(|image| vec![image; self.tiles.len()])
                            }
                        } else {
                            Err(tr("缺少字体").to_owned())
                        };
                        match result {
                            Ok(tiles) => {
                                self.apply_text_at(Some(i), style.clone(), dialog.opacity, tiles, false);
                                dialog.last_applied = Some((style.clone(), dialog.opacity));
                            }
                            Err(error) => self.message = error,
                        }
                    }
                }
            }
            if !opacity_dragging {
                dialog.opacity_drag_recorded = false;
            }
            if !color_picker_open && dialog.last_applied.as_ref().is_some_and(|(style, _)| style == &dialog.draft) {
                dialog.color_preview_recorded = false;
            }
            if let Some(change) = inline_change
                && let Some(edit) = self.inline_text.as_mut().filter(|edit| edit.index == Some(i))
            {
                let count = edit.content.chars().count();
                character::apply(&mut edit.runs, count, 0..count, change);
                ui.ctx().request_repaint();
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
        stamp.fill(tiny_skia::Color::from_rgba8(12, 34, 56, 128));
        let im = place_stamp(&stamp, [-1, -1], 3, 3,true);
        assert_eq!(im.get_pixel(2, 2).0, [12, 34, 56, 192]);
        assert_eq!(im.get_pixel(1, 0).0, [12, 34, 56, 128]);
        assert_eq!(im.get_pixel(1, 1)[3], 0);
        assert_eq!(im, place_stamp(&stamp, [5, -4], 3, 3,true));
    }

    #[test]
    fn text_layer_is_editable_after_save_and_after_undoing_rasterization() {
        let mut editor = Editor::default();
        editor.diamond = false;
        editor.reset_document(RgbaImage::new(8, 8));
        let mut text = Text::new(8, [255; 4]);
        text.content = "A<&字".into();
        character::apply(&mut text.character_runs, 4, 1..3, character::Style {
            size: Some(18), ..Default::default()
        });
        character::apply(&mut text.character_runs, 4, 2..4, character::Style {
            family: Some("Segoe UI".into()), color: Some([12, 72, 154]), ..Default::default()
        });
        assert_eq!(character::get(&text.character_runs, 2).unwrap().size, Some(18));
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
