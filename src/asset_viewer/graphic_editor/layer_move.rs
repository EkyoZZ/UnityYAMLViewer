//! Move layer pixels in the Tile lattice; navigation remains a separate gesture.
use super::*;
pub(super) struct Drag {
    start: Pos2,
    before: Snapshot,
    members: Vec<usize>,
    mask_only: bool,
    last_delta: Option<(i32, i32)>,
    alignment: Option<Alignment>,
}

struct Alignment {
    moving: Rect,
    targets: Vec<Rect>,
    lines: [Option<f32>; 2],
}

impl Alignment {
    fn snap(&mut self, delta: Vec2, scale: f32) -> Vec2 {
        let mut result = delta;
        self.lines = [None; 2];
        for axis in 0..2 {
            let edges = |rect: Rect| [rect.min[axis], rect.center()[axis], rect.max[axis]];
            let mut closest = 6. / scale;
            for target in &self.targets {
                for at in edges(*target) {
                    for source in edges(self.moving) {
                        let correction = at - source - delta[axis];
                        if correction.abs() < closest && correction.fract() == 0. {
                            closest = correction.abs();
                            result[axis] = delta[axis] + correction;
                            self.lines[axis] = Some(at);
                        }
                    }
                }
            }
        }
        result
    }
}
impl Editor {
    pub(super) fn nudge_layers(&mut self, ctx: &egui::Context) {
        let mut moves = Vec::new();
        ctx.input_mut(|input| {
            input.events.retain(|event| {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                else {
                    return true;
                };
                let step = match *modifiers {
                    egui::Modifiers::NONE => 1.,
                    egui::Modifiers::SHIFT => 10.,
                    _ => return true,
                };
                let direction = match key {
                    egui::Key::ArrowLeft => -Vec2::X,
                    egui::Key::ArrowRight => Vec2::X,
                    egui::Key::ArrowUp => -Vec2::Y,
                    egui::Key::ArrowDown => Vec2::Y,
                    _ => return true,
                };
                moves.push(direction * step);
                false
            })
        });
        if moves.is_empty() {
            return;
        }
        let members = self.move_members(self.active_layer, false);
        if members.is_empty() || members.iter().any(|&index| !self.move_allowed(index)) {
            self.message = tr("图层已锁定移动或不可编辑。").into();
            return;
        }
        self.finish_stroke();
        self.sync_layer();
        let layout = self.whole_unit() || self.infinite_view;
        for delta in moves {
            let before = self.snapshot();
            self.apply_layer_translation_target(
                &before,
                &members,
                delta,
                layout,
                self.infinite_view,
                false,
            );
            self.before = Some(before);
            self.finish_stroke();
        }
    }

    pub(super) fn guide_layer_bounds(&self, layout: bool) -> Option<Rect> {
        let members = self.move_members(self.active_layer, false);
        self.move_alignment(&members, layout, false)
            .map(|alignment| alignment.moving)
    }

    fn move_alignment(
        &self,
        members: &[usize],
        layout: bool,
        include_targets: bool,
    ) -> Option<Alignment> {
        let (w, h) = self.pixels.dimensions();
        let half = egui::vec2(w as f32, h as f32) * 0.5;
        let mut cells = Vec::new();
        if layout {
            for row in 0..self.rows {
                for col in 0..self.columns {
                    cells.push((
                        self.tile_at(col as i32, row as i32),
                        self.mapped_center(col as i32, row as i32, w, h) - half,
                    ));
                }
            }
        } else {
            cells.push((self.selected, self.mapped_center(0, 0, w, h) - half));
        }
        let mut canvas = Rect::NOTHING;
        for &(_, offset) in &cells {
            canvas = canvas.union(Rect::from_min_size(offset.to_pos2(), half * 2.));
        }
        let mut moving = Rect::NOTHING;
        let mut targets = vec![canvas];
        for (index, layer) in self.layers.iter().enumerate() {
            let selected = members.contains(&index);
            if layer.settings.group_id != 0
                || layer.settings.adjustment.is_some()
                || (!selected
                    && (!include_targets || !layer.visible || !self.ancestors_visible(index)))
            {
                continue;
            }
            let mut bounds = Rect::NOTHING;
            let mut local = std::collections::HashMap::new();
            for &(tile, offset) in &cells {
                let rect = local.entry(tile).or_insert_with(|| {
                    let mut rect = Rect::NOTHING;
                    for (x, y, pixel) in layer.tiles[tile].enumerate_pixels() {
                        if pixel[3] != 0 {
                            let p = egui::pos2(x as f32, y as f32);
                            rect.extend_with(p);
                            rect.extend_with(p + Vec2::splat(1.));
                        }
                    }
                    rect
                });
                if rect.is_positive() {
                    bounds = bounds.union(rect.translate(offset));
                }
            }
            if bounds.is_positive() {
                if selected {
                    moving = moving.union(bounds);
                } else {
                    targets.push(bounds);
                }
            }
        }
        moving.is_positive().then_some(Alignment {
            moving,
            targets,
            lines: [None; 2],
        })
    }

    pub(super) fn paint_move_alignment(&self, painter: &egui::Painter, origin: Pos2, scale: f32) {
        let Some(alignment) = self
            .layer_drag
            .as_ref()
            .and_then(|drag| drag.alignment.as_ref())
        else {
            return;
        };
        let clip = painter.clip_rect();
        let stroke = egui::Stroke::new(1_f32, Color32::from_rgb(199, 116, 255));
        for (axis, at) in alignment.lines.into_iter().enumerate() {
            if let Some(at) = at {
                let mut start = clip.min;
                let mut end = clip.max;
                start[axis] = origin[axis] + at * scale;
                end[axis] = start[axis];
                painter.line_segment([start, end], stroke);
            }
        }
    }
    #[cfg(test)]
    fn apply_layer_translation(
        &mut self,
        before: &Snapshot,
        members: &[usize],
        delta: Vec2,
        layout: bool,
        infinite: bool,
    ) {
        self.apply_layer_translation_target(before, members, delta, layout, infinite, false);
    }
    fn apply_layer_translation_target(
        &mut self,
        before: &Snapshot,
        members: &[usize],
        delta: Vec2,
        layout: bool,
        infinite: bool,
        mask_only: bool,
    ) {
        let (w, h) = self.pixels.dimensions();
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        for &index in members {
            let source = &before.layers[index];
            if let Some(text) = source
                .settings
                .text
                .as_ref()
                .filter(|t| !mask_only && !t.repeat)
            {
                let mut next = text.clone();
                next.offset = [
                    (text.offset[0] + delta.x.round() as i32).clamp(-8192, 8192),
                    (text.offset[1] + delta.y.round() as i32).clamp(-8192, 8192),
                ];
                self.layers[index].settings.text = Some(next);
            }
            if let Some(text) = source
                .settings
                .text
                .as_ref()
                .filter(|t| !mask_only && t.repeat)
            {
                let dx = (delta.x.round() as i32).rem_euclid(w as i32);
                let dy = (delta.y.round() as i32).rem_euclid(h as i32);
                let layer = &mut self.layers[index];
                let previous = |x: u32, y: u32| {
                    (
                        ((x as i32 - dx).rem_euclid(w as i32)) as u32,
                        ((y as i32 - dy).rem_euclid(h as i32)) as u32,
                    )
                };
                layer.tiles = source
                    .tiles
                    .iter()
                    .map(|im| {
                        RgbaImage::from_fn(w, h, |x, y| {
                            let (px, py) = previous(x, y);
                            *im.get_pixel(px, py)
                        })
                    })
                    .collect();
                if source.settings.mask_linked {
                    layer.settings.mask = source.settings.mask.as_ref().map(|masks| {
                        masks
                            .iter()
                            .map(|m| {
                                (0..w * h)
                                    .map(|i| {
                                        let (px, py) = previous(i % w, i / w);
                                        m[(py * w + px) as usize]
                                    })
                                    .collect()
                            })
                            .collect()
                    });
                }
                let shifted = |coord: i32, d: i32, side: u32| {
                    if d == 0 {
                        coord
                    } else {
                        let s = side as i32;
                        (coord + d + s / 2).rem_euclid(s) - s / 2
                    }
                };
                layer.settings.text.as_mut().unwrap().offset = [
                    shifted(text.offset[0], dx, w),
                    shifted(text.offset[1], dy, h),
                ];
                continue;
            }
            let move_content =
                !mask_only && source.settings.group_id == 0 && source.settings.adjustment.is_none();
            let move_mask = mask_only || source.settings.mask_linked;
            let mut tiles = source.tiles.clone();
            let mut mask = source.settings.mask.clone();
            for row in 0..rows {
                for col in 0..cols {
                    let tile = if layout {
                        self.tile_at(col as i32, row as i32)
                    } else {
                        self.selected
                    };
                    for y in 0..h {
                        for x in 0..w {
                            if self.inside_tile(x as i32, y as i32, w, h) {
                                if move_content {
                                    tiles[tile].put_pixel(x, y, Rgba([0; 4]));
                                }
                                if let Some(m) = mask.as_mut().filter(|_| move_mask) {
                                    m[tile][(y * w + x) as usize] = 255;
                                }
                            }
                        }
                    }
                }
            }
            for row in 0..rows {
                for col in 0..cols {
                    let tile = if layout {
                        self.tile_at(col as i32, row as i32)
                    } else {
                        self.selected
                    };
                    let center = self.mapped_center(col as i32, row as i32, w, h);
                    for y in 0..h {
                        for x in 0..w {
                            if !self.inside_tile(x as i32, y as i32, w, h) {
                                continue;
                            }
                            let world = center
                                + egui::vec2(x as f32 - w as f32 / 2., y as f32 - h as f32 / 2.)
                                + delta;
                            let hit = if self.wrap || infinite {
                                self.plane_hit(world.x as i32, world.y as i32, layout)
                            } else {
                                self.finite_plane_hit(world.x as i32, world.y as i32, layout)
                            };
                            if let Some((to, px, py)) = hit {
                                if move_content {
                                    tiles[to].put_pixel(
                                        px,
                                        py,
                                        *source.tiles[tile].get_pixel(x, y),
                                    );
                                }
                                if let (Some(dst), Some(src)) = (
                                    mask.as_mut().filter(|_| move_mask),
                                    source.settings.mask.as_ref(),
                                ) {
                                    dst[to][(py * w + px) as usize] =
                                        src[tile][(y * w + x) as usize];
                                }
                            }
                        }
                    }
                }
            }
            if !mask_only
                && let Some(text) = self.layers[index]
                    .settings
                    .text
                    .as_ref()
                    .filter(|t| !t.repeat)
                && let Some(fonts) = &self.font_samples.catalog
                && let Ok(im) = text.raster(w, h, fonts)
            {
                tiles = vec![im];
            }
            self.layers[index].tiles = tiles;
            self.layers[index].settings.mask = mask;
        }
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
        self.revision = self.revision.wrapping_add(1);
    }
    pub(super) fn layer_move_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        origin: Pos2,
        scale: f32,
        layout: bool,
        infinite: bool,
    ) {
        if self.tool != Tool::Move {
            return;
        }
        if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
            if let Some(pointer) = response.hover_pos() {
                let cursor_painter = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    ui.id().with("tile_move_cursor"),
                ));
                cursor_painter.add(egui::Shape::convex_polygon(
                    vec![
                        pointer,
                        pointer + egui::vec2(1., 16.),
                        pointer + egui::vec2(12., 10.),
                    ],
                    Color32::BLACK,
                    egui::Stroke::new(1_f32, Color32::WHITE),
                ));
            }
        }
        let Some(pointer) = response.interact_pointer_pos() else {
            return;
        };
        let local = (pointer - origin) / scale;
        let point = Pos2::new(local.x.floor(), local.y.floor());
        if self.move_auto_select && response.clicked_by(egui::PointerButton::Primary) {
            if let Some(index) = self.move_hit_target(point, layout, infinite) {
                self.select_layer_click(index, ui.input(|i| i.modifiers));
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            if self.move_auto_select {
                let pressed = ui.input(|i| i.pointer.press_origin()).unwrap_or(pointer);
                let local = (pressed - origin) / scale;
                let Some(index) =
                    self.move_hit_target(Pos2::new(local.x, local.y), layout, infinite)
                else {
                    return;
                };
                if self.mask_editing {
                    self.mask_editing = false;
                }
                if !self.selected_layers.contains(&index) {
                    self.select_layer_click(index, ui.input(|i| i.modifiers));
                }
            }
            let mask_only = self.mask_editing
                && self.layers[self.active_layer].settings.mask.is_some()
                && !self.layers[self.active_layer].settings.mask_linked;
            let members = if mask_only {
                vec![self.active_layer]
            } else {
                self.move_members(self.active_layer, false)
            };
            if members.iter().any(|i| {
                if mask_only {
                    self.layers[*i].locked
                        || self.ancestor_locked(*i)
                        || self.layers[*i].settings.lock_position
                        || self.ancestor_move_locked(*i)
                } else {
                    !self.move_allowed(*i)
                }
            }) {
                self.message = tr("图层已锁定移动或不可编辑。").into();
                return;
            }
            self.finish_stroke();
            let before = self.snapshot();
            self.before = Some(before.clone());
            let start = ui
                .input(|i| i.pointer.press_origin())
                .map(|p| {
                    let p = (p - origin) / scale;
                    Pos2::new(p.x.floor(), p.y.floor())
                })
                .unwrap_or(point);
            let alignment = (!mask_only && !infinite)
                .then(|| self.move_alignment(&members, layout, true))
                .flatten();
            self.layer_drag = Some(Drag {
                start,
                before,
                members,
                mask_only,
                last_delta: None,
                alignment,
            });
        }
        if let Some(mut drag) = self.layer_drag.take() {
            let mut delta = point - drag.start;
            if let Some(alignment) = &mut drag.alignment {
                delta = alignment.snap(delta, scale);
            }
            let key = (delta.x as i32, delta.y as i32);
            if drag.last_delta == Some(key) {
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    self.finish_stroke();
                } else {
                    self.layer_drag = Some(drag);
                }
                return;
            }
            drag.last_delta = Some(key);
            self.apply_layer_translation_target(
                &drag.before,
                &drag.members,
                delta,
                layout,
                infinite,
                drag.mask_only,
            );
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                self.finish_stroke();
            } else {
                self.layer_drag = Some(drag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unlinked_mask_and_content_move_independently() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(8, 4));
        let i = e.active_layer;
        e.write_pixel(0, 1, 1, Rgba([200, 100, 50, 255]));
        e.sync_layer();
        let mut mask = vec![255; 32];
        mask[9] = 0;
        e.layers[i].settings.mask = Some(vec![mask]);
        e.layers[i].settings.mask_linked = false;
        let before = e.snapshot();
        e.apply_layer_translation_target(&before, &[i], egui::vec2(2., 0.), false, true, true);
        assert_eq!(e.layers[i].tiles[0], before.layers[i].tiles[0]);
        assert_eq!(e.layers[i].settings.mask.as_ref().unwrap()[0][11], 0);
        assert_eq!(e.layers[i].settings.mask.as_ref().unwrap()[0][9], 255);
        e.apply_layer_translation_target(&before, &[i], egui::vec2(2., 0.), false, true, false);
        assert_eq!(e.layers[i].settings.mask, before.layers[i].settings.mask);
        assert_eq!(e.layers[i].tiles[0].get_pixel(3, 1)[3], 255);
        assert_eq!(e.layers[i].tiles[0].get_pixel(1, 1)[3], 0);
    }
    #[test]
    fn layer_move_wraps_across_cells_and_moves_linked_mask() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(8, 4));
        e.multiple = true;
        e.columns = 2;
        e.rows = 2;
        e.independent_slots().unwrap();
        let index = e.active_layer;
        e.write_pixel(0, 7, 2, Rgba([200, 100, 50, 255]));
        let mut masks = vec![vec![255; 32]; 4];
        masks[0][23] = 0;
        e.layers[index].settings.mask = Some(masks);
        let before = e.snapshot();
        e.apply_layer_translation(&before, &[index], egui::vec2(2., 0.), true, true);
        let to = e.tile_at(1, 0);
        assert_eq!(e.tiles[to].get_pixel(1, 2).0, [200, 100, 50, 255]);
        assert_eq!(e.tiles[0].get_pixel(7, 2)[3], 0);
        assert_eq!(e.layers[index].settings.mask.as_ref().unwrap()[to][17], 0);
        assert!(
            e.layers[0]
                .tiles
                .iter()
                .all(|t| t.pixels().all(|p| p[3] == 0))
        );
    }
}
