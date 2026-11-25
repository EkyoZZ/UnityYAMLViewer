//! A free transform keeps a draft separate from the document until confirmation.
use super::*;
use std::collections::HashSet;
type Pixel = (usize, u32, u32);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Interpolation {
    Nearest,
    Bilinear,
    #[default]
    Bicubic,
    Smoother,
    Sharper,
    Automatic,
}

impl Interpolation {
    const CHOICES: [(Self, &'static str); 6] = [
        (Self::Nearest, "邻近"),
        (Self::Bilinear, "两次线性"),
        (Self::Bicubic, "两次立方"),
        (Self::Smoother, "两次立方（较平滑）"),
        (Self::Sharper, "两次立方（较锐利）"),
        (Self::Automatic, "两次立方（自动）"),
    ];

    fn cubic_parameter(self, scale: f32) -> f32 {
        match self {
            Self::Smoother => -0.25,
            Self::Sharper => -0.75,
            Self::Automatic if scale < 1. => -0.75,
            Self::Automatic if scale > 1. => -0.25,
            _ => -0.5,
        }
    }
}

fn cubic_weight(distance: f32, a: f32) -> f32 {
    let x = distance.abs();
    if x <= 1. {
        ((a + 2.) * x - (a + 3.)) * x * x + 1.
    } else if x < 2. {
        ((a * x - 5. * a) * x + 8. * a) * x - 4. * a
    } else {
        0.
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Pose {
    center: Pos2,
    scale: Vec2,
    angle: f32,
}
fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.to_radians().sin_cos();
    egui::vec2(c * v.x - s * v.y, s * v.x + c * v.y)
}
impl Pose {
    fn initial(r: Rect) -> Self {
        Self {
            center: r.center(),
            scale: Vec2::splat(1.),
            angle: 0.,
        }
    }
    fn forward(self, r: Rect, p: Pos2) -> Pos2 {
        self.center + rotate((p - r.center()) * self.scale, self.angle)
    }
    fn inverse(self, r: Rect, p: Pos2) -> Pos2 {
        r.center() + rotate(p - self.center, -self.angle) / self.scale
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Handle {
    Scale(Vec2),
    Rotate,
    Move,
}
#[derive(Clone, Copy)]
struct Drag {
    handle: Handle,
    start: Pos2,
    pose: Pose,
}
pub(super) struct Session {
    bounds: Rect,
    pose: Pose,
    drag: Option<Drag>,
    linked: bool,
    interpolation: Interpolation,
    pub index: usize,
    pub preview: Option<Layer>,
    selection: Option<HashSet<Pixel>>,
    error: Option<String>,
}

const HANDLES: [Vec2; 8] = [
    egui::vec2(-1., -1.),
    egui::vec2(1., -1.),
    egui::vec2(1., 1.),
    egui::vec2(-1., 1.),
    egui::vec2(0., -1.),
    egui::vec2(1., 0.),
    egui::vec2(0., 1.),
    egui::vec2(-1., 0.),
];
fn handle_position(r: Rect, pose: Pose, handle: Vec2) -> Pos2 {
    pose.forward(r, r.center() + r.size() * handle * 0.5)
}

fn corner_controls(bounds: Rect, radii: [f32; 4], angle: f32, zoom: f32) -> Option<[Pos2; 4]> {
    let side = bounds.width().min(bounds.height());
    if side * zoom < 30. {
        return None;
    }
    let min_inset = (12. / zoom).min(side * 0.36);
    let max_inset = side * 0.38;
    Some(std::array::from_fn(|index| {
        let outward = HANDLES[index];
        let inset = (radii[index] * 0.65).clamp(min_inset, max_inset);
        bounds.center() + rotate((bounds.size() * 0.5 - Vec2::splat(inset)) * outward, angle)
    }))
}
fn hit_test(r: Rect, pose: Pose, point: Pos2, zoom: f32, active: bool) -> Option<Handle> {
    let nearest = HANDLES
        .into_iter()
        .map(|h| (h, point.distance(handle_position(r, pose, h)) * zoom))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    if nearest.1 <= 6. {
        return Some(Handle::Scale(nearest.0));
    }
    if r.contains(pose.inverse(r, point)) {
        return active.then_some(Handle::Move);
    }
    let near_corner = HANDLES[..4]
        .iter()
        .any(|h| point.distance(handle_position(r, pose, *h)) * zoom <= 35.);
    (active || near_corner).then_some(Handle::Rotate)
}
fn drag_pose(d: Drag, p: Pos2, bounds: Rect, linked: bool) -> Pose {
    let mut next = d.pose;
    match d.handle {
        Handle::Move => next.center += p - d.start,
        Handle::Rotate => {
            let offset = (p - next.center).angle() - (d.start - next.center).angle();
            next.angle = (next.angle + offset.to_degrees() + 180.).rem_euclid(360.) - 180.;
        }
        Handle::Scale(h) => {
            let fixed = handle_position(bounds, d.pose, -h);
            let delta = rotate(p - d.start, -d.pose.angle);
            if h.x != 0. {
                next.scale.x = (next.scale.x + delta.x * h.x / bounds.width()).clamp(0.01, 32.);
            }
            if h.y != 0. {
                next.scale.y = (next.scale.y + delta.y * h.y / bounds.height()).clamp(0.01, 32.);
            }
            if linked {
                let ratios = next.scale / d.pose.scale;
                let ratio = if (ratios.x - 1.).abs() > (ratios.y - 1.).abs() {
                    ratios.x
                } else {
                    ratios.y
                };
                next.scale = (d.pose.scale * ratio).clamp(Vec2::splat(0.01), Vec2::splat(32.));
            }
            next.center = fixed + rotate(bounds.size() * next.scale * h * 0.5, d.pose.angle);
        }
    }
    next
}
impl Editor {
    fn transform_allowed(&self) -> bool {
        let i = self.active_layer;
        matches!(self.tool, Tool::Move | Tool::Select)
            && !self.elevated
            && self.position_editable(i)
            && self.ancestors_visible(i)
            && self.layers[i].visible
            && !self.ancestor_paint_locked(i)
            && !self.layers[i].settings.lock_pixels
            && self.selection.draft.is_empty()
            && if self.mask_editing {
                let p = &self.layers[i].settings;
                p.mask.is_some() && p.vector.as_ref().is_none_or(|vector| vector.fill.is_some())
            } else {
                let p = &self.layers[i].settings;
                p.group_id == 0
                    && p.adjustment.is_none()
                    && (p.vector.is_none() || self.selection.mask.is_none())
                    && (p.text.is_none()
                        || (self.rectangle_text_allowed() && self.selection.mask.is_none()))
            }
    }
    fn transform_cells(&self, text: bool) -> Vec<(usize, Vec2)> {
        let (w, h) = self.pixels.dimensions();
        let layout = self.whole_unit();
        let (cols, rows) = if layout {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        let mut out = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let tile = if layout {
                    self.tile_at(col as i32, row as i32)
                } else {
                    self.selected
                };
                if text && tile != self.selected {
                    continue;
                }
                out.push((
                    tile,
                    self.mapped_center(col as i32, row as i32, w, h)
                        - egui::vec2(w as f32, h as f32) * 0.5,
                ));
            }
        }
        out
    }
    fn transform_bounds(&mut self) -> Option<Rect> {
        if !self.transform_allowed() {
            return None;
        }
        let key = (
            self.active_layer,
            self.selected,
            self.mask_editing,
            self.selection.generation,
        );
        if let Some((old, r)) = self.transform_bounds_cache {
            if old == key {
                return r;
            }
        }
        let layer = &self.layers[self.active_layer];
        let (w, h) = self.pixels.dimensions();
        if let Some(vector) = layer
            .settings
            .vector
            .as_ref()
            .filter(|_| !self.mask_editing)
        {
            let offset = self
                .vector_origin(layer.settings.vector_origin)
                .map_or_else(
                    || {
                        self.vector_cell(Pos2::ZERO, 1., self.whole_unit())
                            .min
                            .to_vec2()
                    },
                    |[x, y]| egui::vec2(x as f32, y as f32),
                );
            let result = vector
                .bounds()
                .filter(Rect::is_positive)
                .map(|bounds| bounds.translate(offset));
            self.transform_bounds_cache = Some((key, result));
            return result;
        }
        let mut bounds = Rect::NOTHING;
        for (tile, offset) in
            self.transform_cells(layer.settings.text.is_some() && !self.mask_editing)
        {
            for y in 0..h {
                for x in 0..w {
                    let selected = if let Some(mask) = &self.selection.mask {
                        mask.contains(&(tile, x, y))
                    } else if self.mask_editing {
                        layer.settings.mask.as_ref().unwrap()[tile][(y * w + x) as usize] != 255
                    } else {
                        self.tiles[tile].get_pixel(x, y)[3] != 0
                    };
                    if selected && self.inside_tile(x as i32, y as i32, w, h) {
                        let p = (offset + egui::vec2(x as f32, y as f32)).to_pos2();
                        bounds.extend_with(p);
                        bounds.extend_with(p + Vec2::splat(1.));
                    }
                }
            }
        }
        let result = bounds.is_positive().then_some(bounds);
        self.transform_bounds_cache = Some((key, result));
        result
    }
    pub(super) fn begin_transform(&mut self) {
        if self.transform.is_some() {
            return;
        }
        self.finish_stroke();
        self.sync_layer();
        let Some(bounds) = self.transform_bounds() else {
            self.message = tr("当前目标没有可变换的内容。").into();
            return;
        };
        self.layer_detail = None;
        self.text_dialog = None;
        self.show_transform = true;
        self.transform = Some(Session {
            bounds,
            pose: Pose::initial(bounds),
            drag: None,
            linked: self.transform_linked,
            interpolation: self.transform_interpolation,
            index: self.active_layer,
            preview: None,
            selection: None,
            error: None,
        });
    }
    pub(super) fn finish_transform(&mut self, commit: bool) -> bool {
        let Some(mut t) = self.transform.take() else {
            self.confirm_transform_exit = false;
            return true;
        };
        if commit && t.error.is_some() {
            self.transform = Some(t);
            return false;
        }
        self.confirm_transform_exit = false;
        self.transform_linked = t.linked;
        self.transform_interpolation = t.interpolation;
        if let Some(layer) = t
            .preview
            .take()
            .filter(|_| commit && t.index == self.active_layer)
        {
            if layer != self.layers[t.index]
                || t.selection
                    .as_ref()
                    .is_some_and(|m| Some(m) != self.selection.mask.as_ref())
            {
                self.record_layer_change();
                self.layers[t.index] = layer;
                self.tiles = self.layers[t.index].tiles.clone();
                self.pixels = self.tiles[self.selected].clone();
                if let Some(mask) = t.selection {
                    self.selection_from_mask_pixels(mask);
                }
            }
        }
        self.invalidate_textures();
        true
    }
    fn transform_update(&mut self, t: &mut Session) {
        t.preview = None;
        t.error = None;
        t.selection = None;
        if t.pose == Pose::initial(t.bounds) {
            self.invalidate_textures();
            return;
        }
        let layer = &self.layers[t.index];
        let mut draft = layer.clone();
        let (w, h) = self.pixels.dimensions();
        let text = layer.settings.text.is_some() && !self.mask_editing;
        let vector = layer
            .settings
            .vector
            .as_ref()
            .filter(|_| !self.mask_editing);
        if let Some(vector) = vector {
            let origin = self.vector_origin(layer.settings.vector_origin);
            let offset = origin.map_or_else(
                || {
                    self.vector_cell(Pos2::ZERO, 1., self.whole_unit())
                        .min
                        .to_vec2()
                },
                |[x, y]| egui::vec2(x as f32, y as f32),
            );
            let transformed = vector.transformed(
                |point| t.pose.forward(t.bounds, point + offset) - offset,
                t.pose.scale,
                t.pose.angle,
            );
            let Some(images) = transformed
                .valid()
                .then(|| self.render_vector_tiles(&transformed, origin))
                .flatten()
            else {
                t.error = Some(tr("当前目标没有可变换的内容。").to_owned());
                self.invalidate_textures();
                return;
            };
            if transformed.fill.is_some() {
                draft.tiles = images;
            } else {
                draft.settings.mask = Some(
                    images
                        .iter()
                        .map(|image| image.pixels().map(|p| p[3]).collect())
                        .collect(),
                );
            }
            draft.settings.vector = Some(transformed);
            draft.settings.vector_origin = origin;
        }
        if text {
            let Some((_, min)) = self.transform_cells(true).into_iter().next() else {
                return;
            };
            let style = draft.settings.text.as_mut().unwrap();
            let min = style
                .tile_origin
                .map_or(min, |[x, y]| egui::vec2(x as f32, y as f32));
            let pivot = t.bounds.center().to_vec2() - min - egui::vec2(w as f32, h as f32) * 0.5;
            style.transform(
                [
                    rotate(Vec2::X * t.pose.scale.x, t.pose.angle),
                    rotate(Vec2::Y * t.pose.scale.y, t.pose.angle),
                ],
                pivot,
                t.pose.center - t.bounds.center(),
                [w, h],
            );
            match self
                .font_samples
                .catalog
                .as_ref()
                .ok_or_else(|| tr("正在读取系统字体").to_owned())
                .and_then(|fonts| self.render_text_tiles(style, fonts))
            {
                Ok(tiles) => draft.tiles = tiles,
                Err(e) => {
                    self.message = e.clone();
                    t.error = Some(e);
                    self.invalidate_textures();
                    return;
                }
            }
        }
        let masks = if self.mask_editing
            || (layer.settings.mask_linked && vector.is_none_or(|vector| vector.fill.is_some()))
        {
            layer.settings.mask.as_ref()
        } else {
            None
        };
        if (!text && vector.is_none()) || masks.is_some() {
            let mut selected = HashSet::new();
            let layout = self.whole_unit();
            let text_cell = text.then(|| self.transform_cells(true)[0].1);
            for (tile, cell) in self.transform_cells(false) {
                let offset = text_cell.unwrap_or(cell);
                let sample = |x: i32, y: i32| {
                    if text && !self.plain_image {
                        Some((
                            tile,
                            (x - offset.x as i32).rem_euclid(w as i32) as u32,
                            (y - offset.y as i32).rem_euclid(h as i32) as u32,
                        ))
                    } else {
                        self.finite_plane_hit(x, y, layout)
                            .filter(|&(i, x, y)| self.selection.allows(i, x, y))
                    }
                };
                for y in 0..h {
                    for x in 0..w {
                        if !self.inside_tile(x as i32, y as i32, w, h) {
                            continue;
                        }
                        let dest = (offset + egui::vec2(x as f32 + 0.5, y as f32 + 0.5)).to_pos2();
                        let local = t.pose.inverse(t.bounds, dest);
                        let from = sample(local.x.floor() as i32, local.y.floor() as i32);
                        if from.is_some() && self.selection.mask.is_some() {
                            selected.insert((tile, x, y));
                        }
                        let clear = self.selection.allows(tile, x, y);
                        let mut rgba = [0.; 4];
                        let mut mask_drop = 0.;
                        let mut add = |from: Option<Pixel>, weight: f32| {
                            if let Some((i, sx, sy)) = from {
                                let pixel = layer.tiles[i].get_pixel(sx, sy);
                                let alpha = f32::from(pixel[3]) * weight;
                                for c in 0..3 {
                                    rgba[c] += f32::from(pixel[c]) * alpha;
                                }
                                rgba[3] += alpha;
                                if let Some(m) = masks {
                                    mask_drop +=
                                        f32::from(255 - m[i][(sy * w + sx) as usize]) * weight;
                                }
                            }
                        };
                        if t.interpolation != Interpolation::Nearest {
                            let p = local - Vec2::splat(0.5);
                            let sx = p.x.floor() as i32;
                            let sy = p.y.floor() as i32;
                            let fx = p.x - sx as f32;
                            let fy = p.y - sy as f32;
                            if t.interpolation == Interpolation::Bilinear {
                                for (dx, wx) in [(0, 1. - fx), (1, fx)] {
                                    for (dy, wy) in [(0, 1. - fy), (1, fy)] {
                                        add(sample(sx + dx, sy + dy), wx * wy);
                                    }
                                }
                            } else {
                                let ax = t.interpolation.cubic_parameter(t.pose.scale.x);
                                let ay = t.interpolation.cubic_parameter(t.pose.scale.y);
                                let x = [-1, 0, 1, 2].map(|i| (i, cubic_weight(fx - i as f32, ax)));
                                let y = [-1, 0, 1, 2].map(|i| (i, cubic_weight(fy - i as f32, ay)));
                                for (dx, wx) in x {
                                    for (dy, wy) in y {
                                        let weight = wx * wy;
                                        if weight != 0. {
                                            add(sample(sx + dx, sy + dy), weight);
                                        }
                                    }
                                }
                            }
                        } else {
                            add(from, 1.);
                        }
                        if !self.mask_editing && !text && vector.is_none() {
                            let base = if clear {
                                Rgba([0; 4])
                            } else {
                                *layer.tiles[tile].get_pixel(x, y)
                            };
                            let paint = if rgba[3] > 0. {
                                Rgba([
                                    (rgba[0] / rgba[3]).round() as u8,
                                    (rgba[1] / rgba[3]).round() as u8,
                                    (rgba[2] / rgba[3]).round() as u8,
                                    rgba[3].round() as u8,
                                ])
                            } else {
                                Rgba([0; 4])
                            };
                            draft.tiles[tile].put_pixel(
                                x,
                                y,
                                if paint[3] == 0 {
                                    base
                                } else {
                                    layer_properties::over(
                                        base,
                                        paint,
                                        layer_properties::Blend::Normal,
                                        1.,
                                    )
                                },
                            );
                        }
                        if let (Some(m), Some(dst)) = (masks, draft.settings.mask.as_mut()) {
                            let at = (y * w + x) as usize;
                            let base = if clear { 255 } else { m[tile][at] };
                            dst[tile][at] =
                                (f32::from(base) * (1. - mask_drop / 255.)).round() as u8;
                        }
                    }
                }
            }
            if self.selection.mask.is_some() {
                t.selection = Some(selected);
            }
        }
        t.preview = Some(draft);
        self.invalidate_textures();
    }
    pub(super) fn transform_options(&mut self, ui: &mut egui::Ui) {
        let Some(mut t) = self.transform.take() else {
            return;
        };
        let mut commit = false;
        let mut cancel = false;
        let old = t.pose;
        let interpolation = t.interpolation;
        egui::ScrollArea::horizontal()
            .id_salt("transform_options")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.;
                    ui.strong(tr("自由变换"));
                    for (label, position, side) in [
                        ("X", &mut t.pose.center.x, self.pixels.width()),
                        ("Y", &mut t.pose.center.y, self.pixels.height()),
                    ] {
                        let mut value = *position + side as f32 / 2.;
                        ui.label(label);
                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .speed(0.5)
                                    .range(-32768. ..=32768.),
                            )
                            .changed()
                        {
                            *position = value - side as f32 / 2.;
                        }
                    }
                    let scale = t.pose.scale;
                    for axis in 0..2 {
                        if axis == 1 {
                            ui.toggle_value(&mut t.linked, "↔")
                                .on_hover_text(tr("等比缩放"));
                        }
                        ui.label(if axis == 0 { "W" } else { "H" });
                        let mut value = scale[axis] * 100.;
                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .range(1. ..=3200.)
                                    .suffix("%"),
                            )
                            .changed()
                        {
                            if t.linked {
                                t.pose.scale = (scale * (value / (100. * scale[axis])))
                                    .clamp(Vec2::splat(0.01), Vec2::splat(32.));
                            } else {
                                t.pose.scale[axis] = value / 100.;
                            }
                        }
                    }
                    ui.label("∠");
                    ui.add(
                        egui::DragValue::new(&mut t.pose.angle)
                            .range(-180. ..=180.)
                            .suffix("°"),
                    );
                    let settings = &self.layers[t.index].settings;
                    let samples_pixels = (settings.text.is_none() && settings.vector.is_none())
                        || (settings.mask.is_some()
                            && (self.mask_editing || settings.mask_linked)
                            && settings
                                .vector
                                .as_ref()
                                .is_none_or(|vector| vector.fill.is_some()));
                    ui.add_enabled_ui(samples_pixels, |ui| {
                        ui.label(tr("插值方式"));
                        let label = Interpolation::CHOICES
                            .iter()
                            .find(|(value, _)| *value == t.interpolation)
                            .unwrap()
                            .1;
                        egui::ComboBox::from_id_salt("transform_interpolation")
                            .selected_text(tr(label))
                            .show_ui(ui, |ui| {
                                for (value, label) in Interpolation::CHOICES {
                                    ui.selectable_value(&mut t.interpolation, value, tr(label));
                                }
                            });
                    });
                    commit = ui
                        .add_enabled(t.error.is_none(), egui::Button::new(tr("确定")))
                        .clicked();
                    cancel = ui.button(tr("取消")).clicked();
                })
            });
        if old != t.pose || interpolation != t.interpolation {
            self.transform_update(&mut t);
        }
        self.transform = Some(t);
        if cancel || commit {
            self.finish_transform(commit);
        }
    }
    pub(super) fn transform_switch(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.show_transform, tr("显示变换控件"))
            .on_hover_text("Ctrl+H");
        if ui
            .small_button(tr("自由变换"))
            .on_hover_text("Ctrl+T")
            .clicked()
        {
            self.begin_transform();
        }
    }

    pub(super) fn toggle_transform_controls(&mut self) {
        if let Some(session) = self.transform.as_mut() {
            session.drag = None;
            self.confirm_transform_exit = true;
        } else {
            self.show_transform = !self.show_transform;
        }
    }

    pub(super) fn transform_exit_confirmation(&mut self, ui: &egui::Ui) {
        if !self.confirm_transform_exit {
            return;
        }
        let Some(session) = &self.transform else {
            self.confirm_transform_exit = false;
            return;
        };
        let (decision, dismissed) = editor_confirmation_response(
            ui,
            egui::Modal::new(ui.id().with("transform_exit_confirmation")),
            tr("退出变换"),
            tr("是否应用当前变换？"),
            tr("放弃变换"),
            tr("应用变换"),
            session.error.is_none(),
            |_| {},
        );
        if dismissed {
            self.confirm_transform_exit = false;
        } else if let Some(commit) = decision
            && self.finish_transform(commit)
        {
            self.show_transform = false;
        }
    }
    pub(super) fn transform_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        origin: Pos2,
        zoom: f32,
    ) -> bool {
        if self.confirm_transform_exit {
            return true;
        }
        if (!self.show_transform || !matches!(self.tool, Tool::Move | Tool::Select))
            && self.transform.is_none()
        {
            self.path_tool.corner_drag = None;
            return false;
        }
        let active = self.transform.is_some();
        let frame = self
            .transform
            .as_ref()
            .map(|t| (t.bounds, t.pose))
            .or_else(|| self.transform_bounds().map(|r| (r, Pose::initial(r))));
        let Some((r, pose)) = frame else {
            self.path_tool.corner_drag = None;
            return active;
        };
        let Some(pointer) = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos())
        else {
            return active;
        };
        let local = |p: Pos2| ((p - origin) / zoom).to_pos2();
        let start = ui.input(|i| i.pointer.press_origin()).unwrap_or(pointer);
        if !active
            && self.tool == Tool::Move
            && self.corner_input(ui, response, local(start), local(pointer), zoom)
        {
            return true;
        }
        let hit = hit_test(
            r,
            pose,
            local(if response.drag_started() {
                start
            } else {
                pointer
            }),
            zoom,
            active,
        );
        let handle = self
            .transform
            .as_ref()
            .and_then(|t| t.drag)
            .map(|d| d.handle)
            .or(hit);
        if let Some(handle) = handle {
            let icon = match handle {
                Handle::Move => egui::CursorIcon::Move,
                Handle::Scale(h) => {
                    let v = rotate(h * pose.scale, pose.angle);
                    match (v.angle().to_degrees() / 45.).round() as i32 % 4 {
                        0 => egui::CursorIcon::ResizeHorizontal,
                        1 | -3 => egui::CursorIcon::ResizeNwSe,
                        2 | -2 => egui::CursorIcon::ResizeVertical,
                        _ => egui::CursorIcon::ResizeNeSw,
                    }
                }
                Handle::Rotate => {
                    let painter = ui.ctx().layer_painter(egui::LayerId::new(
                        egui::Order::Tooltip,
                        ui.id().with("rotate_cursor"),
                    ));
                    let path: Vec<_> = (0..=12)
                        .map(|i| pointer + rotate(egui::vec2(7., 0.), -175. + i as f32 * 13.))
                        .collect();
                    let tip = *path.last().unwrap();
                    for (color, width) in [(Color32::BLACK, 3.5_f32), (Color32::WHITE, 1.5)] {
                        let stroke = egui::Stroke::new(width, color);
                        painter.add(egui::Shape::line(path.clone(), stroke));
                        painter.add(egui::Shape::line(
                            vec![tip + egui::vec2(-4., -2.), tip, tip + egui::vec2(2., -4.)],
                            stroke,
                        ));
                    }
                    egui::CursorIcon::None
                }
            };
            ui.output_mut(|o| o.cursor_icon = icon);
        }
        if active && response.double_clicked_by(egui::PointerButton::Primary) {
            self.finish_transform(true);
            return true;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(h) = hit {
                if !active {
                    self.begin_transform();
                }
                if let Some(t) = self.transform.as_mut() {
                    t.drag = Some(Drag {
                        handle: h,
                        start: local(start),
                        pose,
                    });
                }
            }
        }
        if let Some(mut t) = self.transform.take() {
            if let Some(d) = t.drag {
                let next = drag_pose(d, local(pointer), r, t.linked);
                if next != t.pose {
                    t.pose = next;
                    self.transform_update(&mut t);
                }
                if !ui.input(|i| i.pointer.primary_down()) {
                    t.drag = None;
                }
            }
            self.transform = Some(t);
            return true;
        }
        handle.is_some()
    }

    fn corner_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        start: Pos2,
        pointer: Pos2,
        zoom: f32,
    ) -> bool {
        let layer = self.active_layer;
        let Some(mut vector) = self.layers[layer]
            .settings
            .vector
            .clone()
            .filter(|_| !self.mask_editing)
        else {
            self.path_tool.corner_drag = None;
            return false;
        };
        let vector::Geometry::Rectangle {
            min,
            max,
            radii,
            angle,
        } = &mut vector.geometry
        else {
            self.path_tool.corner_drag = None;
            return false;
        };
        let bounds = self.rectangle_corner_bounds(*min, *max);
        let hit = corner_controls(bounds, *radii, *angle, zoom).and_then(|handles| {
            let point = if response.drag_started() {
                start
            } else {
                pointer
            };
            handles
                .iter()
                .position(|handle| handle.distance(point) * zoom <= 7.)
        });
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(corner) = hit
        {
            self.path_tool.corner_drag = Some(vector::CornerDrag {
                layer,
                corner,
                pointer: start,
                radius: radii[corner],
                changed: false,
            });
        }
        if let Some(mut drag) = self
            .path_tool
            .corner_drag
            .take()
            .filter(|drag| drag.layer == layer)
        {
            let down = ui.input(|i| i.pointer.primary_down());
            if !down && !response.drag_stopped_by(egui::PointerButton::Primary) {
                return true;
            }
            let inward = rotate(-HANDLES[drag.corner], *angle);
            let targeted =
                (drag.radius + (pointer - drag.pointer).dot(inward) * 0.5).clamp(0., 8192.);
            if targeted != radii[drag.corner] {
                if self.path_tool.corner_unlinked {
                    radii[drag.corner] = targeted;
                } else {
                    radii.fill(targeted);
                }
                self.update_vector(layer, vector, !drag.changed);
                drag.changed = true;
            }
            if down {
                self.path_tool.corner_drag = Some(drag);
            }
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            return true;
        }
        if hit.is_some() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
        hit.is_some()
    }
    pub(super) fn transform_outline(&mut self, painter: &egui::Painter, origin: Pos2, zoom: f32) {
        if !self.show_transform || !matches!(self.tool, Tool::Move | Tool::Select) {
            return;
        }
        let frame = self
            .transform
            .as_ref()
            .map(|t| (t.bounds, t.pose))
            .or_else(|| self.transform_bounds().map(|r| (r, Pose::initial(r))));
        let Some((bounds, pose)) = frame else {
            return;
        };
        let handles = HANDLES.map(|h| origin + handle_position(bounds, pose, h).to_vec2() * zoom);
        let blue = Color32::from_rgb(65, 143, 255);
        painter.add(egui::Shape::closed_line(
            handles[..4].to_vec(),
            egui::Stroke::new(1_f32, blue),
        ));
        for p in handles {
            let r = Rect::from_center_size(p, Vec2::splat(7.));
            painter.rect_filled(r, 0., Color32::WHITE);
            painter.rect_stroke(
                r,
                0.,
                egui::Stroke::new(1_f32, blue),
                egui::StrokeKind::Inside,
            );
        }
        if self.tool == Tool::Move
            && self.transform.is_none()
            && !self.mask_editing
            && let Some(vector::Geometry::Rectangle {
                min,
                max,
                radii,
                angle,
            }) = self.layers[self.active_layer]
                .settings
                .vector
                .as_ref()
                .map(|vector| &vector.geometry)
            && let Some(points) = corner_controls(
                self.rectangle_corner_bounds(*min, *max),
                *radii,
                *angle,
                zoom,
            )
        {
            for point in points {
                let center = origin + point.to_vec2() * zoom;
                painter.circle_filled(center, 4., Color32::WHITE);
                painter.circle_stroke(center, 4., egui::Stroke::new(1_f32, blue));
                painter.circle_filled(center, 1.3, blue);
            }
        }
    }

    fn rectangle_corner_bounds(&self, min: [f32; 2], max: [f32; 2]) -> Rect {
        let offset = self.vector_origin(self.layers[self.active_layer].settings.vector_origin).map_or_else(
            || self.vector_cell(Pos2::ZERO, 1., self.whole_unit()).min.to_vec2(),
            |[x, y]| egui::vec2(x as f32, y as f32),
        );
        Rect::from_min_max(
            egui::pos2(min[0], min[1]) + offset,
            egui::pos2(max[0], max[1]) + offset,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_resize_keeps_opposite_corner_fixed_after_rotation() {
        let r = Rect::from_min_max(egui::pos2(-2., -1.), egui::pos2(2., 1.));
        let initial = Pose {
            angle: 33.,
            ..Pose::initial(r)
        };
        let h = HANDLES[0];
        let start = handle_position(r, initial, h);
        let end = start + rotate(egui::vec2(-2., -1.), 33.);
        let result = drag_pose(
            Drag {
                handle: Handle::Scale(h),
                start,
                pose: initial,
            },
            end,
            r,
            true,
        );
        assert!(handle_position(r, result, -h).distance(handle_position(r, initial, -h)) < 0.001);
        assert!(handle_position(r, result, h).distance(end) < 0.001);
        assert!(hit_test(r, result, end, 10., true) == Some(Handle::Scale(h)));
        assert!(
            hit_test(
                r,
                result,
                end + rotate(egui::vec2(-2., -1.), 33.),
                10.,
                true
            ) == Some(Handle::Rotate)
        );
    }
    #[test]
    fn transformed_selection_is_draft_until_commit_and_bounds_follow_undo() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(8, 8));
        e.tool = Tool::Select;
        e.write_pixel(0, 2, 2, Rgba([240, 20, 30, 255]));
        e.write_pixel(0, 6, 6, Rgba([20, 40, 60, 255]));
        e.selection_from_mask_pixels([(0, 2, 2)].into_iter().collect());
        let state = e.snapshot();
        e.begin_transform();
        let mut t = e.transform.take().unwrap();
        t.pose.center.x += 2.;
        e.transform_update(&mut t);
        e.transform = Some(t);
        assert!(state == e.snapshot());
        assert_eq!(e.composite(0).get_pixel(4, 2).0, [240, 20, 30, 255]);
        e.finish_transform(false);
        assert!(state == e.snapshot());
        e.begin_transform();
        let mut t = e.transform.take().unwrap();
        t.pose.center.x += 2.;
        e.transform_update(&mut t);
        e.transform = Some(t);
        e.finish_transform(true);
        assert_eq!(e.transform_bounds().unwrap().min, egui::pos2(0., -2.));
        assert_eq!(e.pixels.get_pixel(6, 6)[3], 255);
        assert_eq!(e.pixels.get_pixel(2, 2)[3], 0);
        e.history(false);
        assert!(state == e.snapshot());
    }
}
