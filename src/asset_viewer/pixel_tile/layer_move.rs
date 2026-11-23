//! Move layer pixels in the Tile lattice; navigation remains a separate gesture.
use super::*;
pub(super) struct Drag {
    start: Pos2,
    before: Snapshot,
    members: Vec<usize>,
    mask_only: bool,
    last_delta: Option<(i32, i32)>,
}
impl Editor {
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
            self.layer_drag = Some(Drag {
                start,
                before,
                members,
                mask_only,
                last_delta: None,
            });
        }
        if let Some(mut drag) = self.layer_drag.take() {
            let delta = point - drag.start;
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
        assert!(e.layers[0]
            .tiles
            .iter()
            .all(|t| t.pixels().all(|p| p[3] == 0)));
    }
}
