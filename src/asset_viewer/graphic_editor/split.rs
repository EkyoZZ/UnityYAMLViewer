use super::*;

#[derive(Clone, Copy)]
struct SplitGrid {
    columns: usize,
    rows: usize,
    diamond: bool,
}

impl SplitGrid {
    // The diamond's row and column axes run down-left and down-right respectively.
    // With equal axis counts and even cell sizes, all origins land on whole pixels.
    fn offsets(self, col: usize, row: usize) -> (usize, usize) {
        if self.diamond {
            (self.rows - 1 + col - row, col + row)
        } else {
            (2 * col, 2 * row)
        }
    }

    fn cell_size(self, w: u32, h: u32) -> Result<(u32, u32), &'static str> {
        if !(1..=8).contains(&self.columns)
            || !(1..=8).contains(&self.rows)
            || (self.diamond && self.rows != self.columns)
        {
            return Err("拆分行列无效。");
        }
        let (cols, rows) = (self.columns as u32, self.rows as u32);
        if w % (2 * cols) != 0 || h % (2 * rows) != 0 || w < 2 * cols || h < 2 * rows {
            return Err("请调整行列，使每格的宽和高均为偶数像素。");
        }
        Ok((w / cols, h / rows))
    }

    fn origins(self, cell: (u32, u32)) -> impl Iterator<Item = (u32, u32)> {
        (0..self.rows).flat_map(move |row| {
            (0..self.columns).map(move |col| {
                let (x, y) = self.offsets(col, row);
                (x as u32 * cell.0 / 2, y as u32 * cell.1 / 2)
            })
        })
    }

    fn images(self, source: &RgbaImage, cell: (u32, u32)) -> Vec<RgbaImage> {
        self.origins(cell)
            .map(|(left, top)| {
                RgbaImage::from_fn(cell.0, cell.1, |x, y| {
                    if inside(x as i32, y as i32, cell.0, cell.1, self.diamond) {
                        *source.get_pixel(left + x, top + y)
                    } else {
                        Rgba([0; 4])
                    }
                })
            })
            .collect()
    }

    fn masks(self, source: &[u8], width: u32, cell: (u32, u32)) -> Vec<Vec<u8>> {
        self.origins(cell)
            .map(|(left, top)| {
                let mut mask = Vec::with_capacity((cell.0 * cell.1) as usize);
                for y in 0..cell.1 {
                    for x in 0..cell.0 {
                        mask.push(
                            if inside(x as i32, y as i32, cell.0, cell.1, self.diamond) {
                                source[((top + y) * width + left + x) as usize]
                            } else {
                                255
                            },
                        );
                    }
                }
                mask
            })
            .collect()
    }
}

impl Editor {
    pub(super) fn showing_split_guides(&self) -> bool {
        self.split_preview
            && (self.split_grid != [1, 1] || self.has_multiple_slots() || self.infinite_view)
    }

    fn split_plan(&self) -> SplitGrid {
        SplitGrid {
            columns: self.split_grid[0],
            rows: self.split_grid[1],
            diamond: self.diamond,
        }
    }

    fn split_cell_size(&self) -> Result<(u32, u32), &'static str> {
        if self.hex != 0 || self.elevated {
            return Err("拆分支持矩形和菱形 Tile。");
        }
        self.split_plan()
            .cell_size(self.pixels.width(), self.pixels.height())
    }

    fn split_needs_merge(&self) -> bool {
        self.layers
            .iter()
            .any(|l| l.settings.text.is_some() || l.settings.has_effects())
    }

    fn apply_split(&mut self) -> Result<(), &'static str> {
        if self.split_grid == [1, 1] {
            return Ok(());
        }
        let cell = self.split_cell_size()?;
        let plan = self.split_plan();
        let count = plan.columns * plan.rows;
        if self.materials.len() * count > 64 {
            return Err("拆分后的素材超过 64 项。");
        }
        self.finish_stroke();
        self.ensure_layers();
        self.sync_layer();
        // Layout previews may have created identical copies. Include hidden cells and
        // hidden layers in this check so applying a split never discards a unique cell.
        if self.layers.iter().any(|l| {
            l.tiles.iter().any(|t| t != &l.tiles[self.selected])
                || l.settings
                    .mask
                    .as_ref()
                    .is_some_and(|m| m.iter().any(|v| v != &m[self.selected]))
        }) {
            return Err("已有不同内容的格子，请从单张图拆分。");
        }
        let materials = self
            .materials
            .iter()
            .flat_map(|m| {
                let name: String = m.name.chars().take(58).collect();
                plan.images(&m.pixels, cell)
                    .into_iter()
                    .enumerate()
                    .map(move |(i, pixels)| Material {
                        name: format!("{name} {}-{}", i / plan.columns + 1, i % plan.columns + 1),
                        pixels,
                    })
            })
            .collect();
        let merged = self
            .split_needs_merge()
            .then(|| self.composite(self.selected));
        self.record_layer_change();
        if let Some(image) = merged {
            self.set_single_tile(image);
        }
        for layer in &mut self.layers {
            if let Some(masks) = &mut layer.settings.mask {
                *masks = plan.masks(&masks[self.selected], self.pixels.width(), cell);
            }
            layer.tiles = plan.images(&layer.tiles[self.selected], cell);
        }
        self.materials = materials;
        self.material_thumbnails.clear();
        self.layout.fill(0);
        for row in 0..plan.rows {
            for col in 0..plan.columns {
                self.layout[row * 8 + col] = row * plan.columns + col;
            }
        }
        self.selected = 0;
        self.selected_slot = 0;
        self.columns = plan.columns;
        self.rows = plan.rows;
        self.multiple = true;
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[0].clone();
        (self.width, self.height) = cell;
        self.footprint = [cell.0, cell.1];
        self.pan = Vec2::ZERO;
        self.repeat_pan = Vec2::ZERO;
        self.selection.clear();
        self.layer_detail = None;
        self.mask_editing = false;
        self.text_dialog = None;
        self.split_preview = false;
        self.split_grid = [1, 1];
        self.invalidate_textures();
        self.message.clear();
        Ok(())
    }

    pub(super) fn tile_settings(&mut self, ui: &mut egui::Ui) {
        let mut tab = if self.split_preview { "split" } else { "multi" }.to_owned();
        if crate::theme::compact_selector(
            ui,
            &mut tab,
            [("split", "拆分 Tile"), ("multi", "多 Tile")],
        ) {
            self.finish_stroke();
            self.split_preview = tab == "split";
        }
        let previous = if self.split_preview {
            self.split_grid
        } else if self.multiple {
            [self.columns, self.rows]
        } else {
            [1, 1]
        };
        let mut grid = previous;
        ui.horizontal(|ui| {
            if self.split_preview && self.diamond {
                ui.label(tr("每轴格数"));
                ui.add(egui::DragValue::new(&mut grid[0]).range(1..=8));
                grid[1] = grid[0];
            } else {
                for (value, label) in grid.iter_mut().zip(["列", "行"]) {
                    ui.label(tr(label));
                    ui.add(egui::DragValue::new(value).range(1..=8));
                }
            }
        });
        ui.horizontal(|ui| {
            for n in 1..=4 {
                if ui
                    .selectable_label(grid == [n, n], format!("{n}×{n}"))
                    .clicked()
                {
                    grid = [n, n];
                }
            }
        });
        if self.split_preview {
            self.split_grid = grid;
            if grid == [1, 1] {
                return;
            }
            match self.split_cell_size() {
                Ok((w, h)) => {
                    let label = if self.split_needs_merge() {
                        "合并可见图层并拆分"
                    } else {
                        "应用拆分"
                    };
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr(label)).clicked()
                            && let Err(reason) = self.apply_split()
                        {
                            self.message = tr(reason).to_owned();
                        }
                        ui.label(format!("{w} × {h} px"));
                    });
                }
                Err(reason) => {
                    ui.small(tr(reason));
                }
            }
        } else if grid != previous {
            self.set_preview_layout(grid != [1, 1], grid[0], grid[1]);
        }
    }

    pub(super) fn split_canvas(&self, ui: &mut egui::Ui) {
        ui.heading(tr("拆分预览"));
        let area = ui.available_rect_before_wrap();
        let Some(texture) = self.textures.get(self.selected) else {
            return;
        };
        let size = self.pixels.dimensions();
        let source = egui::vec2(size.0 as f32, size.1 as f32);
        let scale = ((area.width() - 16.) / source.x).min((area.height() - 16.) / source.y);
        if scale <= 0. {
            return;
        }
        let rect = Rect::from_center_size(area.center(), source * scale);
        self.paint_background(ui.painter(), rect, rect.min, Vec2::splat(scale));
        ui.painter().image(
            texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
            Color32::WHITE,
        );
        if self.split_grid == [1, 1] || self.split_cell_size().is_err() {
            return;
        }
        let grid = self.split_plan();
        let cell = egui::vec2(
            rect.width() / grid.columns as f32,
            rect.height() / grid.rows as f32,
        );
        for row in 0..grid.rows {
            for col in 0..grid.columns {
                let (x, y) = grid.offsets(col, row);
                let origin = rect.min + egui::vec2(x as f32 * cell.x / 2., y as f32 * cell.y / 2.);
                Self::outline(
                    ui.painter(),
                    Rect::from_min_size(origin, cell),
                    grid.diamond,
                    Color32::YELLOW,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| Rgba([x as u8, y as u8, 47, 255]))
    }

    #[test]
    fn preview_and_identity_never_create_history() {
        let mut e = Editor::default();
        e.ensure_material_base();
        e.set_single_tile(source(64, 32));
        e.set_preview_layout(true, 2, 2);
        e.select_slot(9);
        e.set_preview_shape(false, 0, false);
        let initial = e.snapshot();
        e.split_preview = true;
        e.split_grid = [2, 2];
        assert_eq!(e.split_cell_size(), Ok((32, 16)));
        e.split_grid = [1, 1];
        e.apply_split().unwrap();
        assert!(e.snapshot() == initial && !e.changed && e.undo.is_empty());
        e.reset_document(source(32, 16));
        assert!(e.changed && e.undo.len() == 1);
    }

    #[test]
    fn diamond_covers_exactly_the_visible_source_once() {
        for n in 2..=8 {
            let grid = SplitGrid {
                columns: n,
                rows: n,
                diamond: true,
            };
            let (cw, ch) = (6, 4);
            let (w, h) = (cw * n as u32, ch * n as u32);
            let source = source(w, h);
            let mut hits = vec![0; (w * h) as usize];
            for (piece, (left, top)) in grid
                .images(&source, (cw, ch))
                .iter()
                .zip(grid.origins((cw, ch)))
            {
                for (x, y, p) in piece.enumerate_pixels().filter(|(_, _, p)| p[3] != 0) {
                    assert_eq!(p, source.get_pixel(left + x, top + y));
                    hits[((top + y) * w + left + x) as usize] += 1;
                }
            }
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        hits[(y * w + x) as usize],
                        u8::from(inside(x as i32, y as i32, w, h, true))
                    );
                }
            }
        }
    }

    #[test]
    fn rect_split_preserves_masks_layers_materials_and_history() {
        let mut e = Editor::default();
        let source = source(12, 8);
        e.diamond = false;
        e.set_single_tile(source.clone());
        e.layers[1].settings.mask = Some(vec![(0..96).collect()]);
        e.set_preview_layout(true, 2, 2);
        e.select_slot(9);
        let before = e.snapshot();
        e.split_grid = [3, 2];
        e.apply_split().unwrap();
        assert_eq!(e.undo.len(), 1);
        assert_eq!((e.width, e.height, e.columns, e.rows), (4, 4, 3, 2));
        for row in 0..2 {
            for col in 0..3 {
                let i = e.layout[row * 8 + col];
                for y in 0..4 {
                    for x in 0..4 {
                        let sx = col as u32 * 4 + x;
                        let sy = row as u32 * 4 + y;
                        assert_eq!(
                            e.layers[0].tiles[i].get_pixel(x, y),
                            source.get_pixel(sx, sy)
                        );
                        assert_eq!(
                            e.layers[1].settings.mask.as_ref().unwrap()[i][(y * 4 + x) as usize],
                            (sy * 12 + sx) as u8
                        );
                    }
                }
                assert_eq!(e.materials[i].pixels, e.layers[0].tiles[i]);
            }
        }
        let after = e.snapshot();
        e.history(false);
        assert!(e.snapshot() == before);
        assert_eq!((e.width, e.height), (12, 8));
        e.history(true);
        assert!(e.snapshot() == after);
        assert_eq!((e.width, e.height), (4, 4));
    }

    #[test]
    fn selecting_another_cell_commits_only_an_actual_stroke() {
        let mut e = Editor::default();
        e.set_single_tile(source(32, 16));
        e.diamond = false;
        e.set_preview_layout(true, 2, 2);
        e.select_slot(1);
        assert!(!e.changed && e.undo.is_empty());
        e.tool = Tool::Brush;
        e.plane_stroke((0, 0), (0, 0), false);
        e.select_slot(9);
        assert!(e.changed && e.undo.len() == 1);
    }

    #[test]
    fn splitting_effects_preserves_the_composite_and_undo_restores_layers() {
        let mut e = Editor::default();
        e.diamond = false;
        e.set_single_tile(source(8, 8));
        e.layers[0].settings.effects =
            serde_json::from_value(serde_json::json!({"overlay":true,"overlay_color":[160,22,13]}))
                .unwrap();
        let before = e.snapshot();
        let appearance = e.composite(0);
        e.split_grid = [2, 2];
        let expected = e.split_plan().images(&appearance, (4, 4));
        assert!(e.split_needs_merge());
        e.apply_split().unwrap();
        for (i, image) in expected.iter().enumerate() {
            assert_eq!(&e.composite(i), image);
        }
        e.history(false);
        assert!(e.snapshot() == before);
    }

    #[test]
    fn splitting_and_exporting_the_assembled_png_preserves_original_pixels() {
        for diamond in [false, true] {
            let mut e = Editor::default();
            e.diamond = diamond;
            e.set_single_tile(RgbaImage::from_fn(64, 32, |x, y| {
                Rgba([x as u8, y as u8, 90, (x + y + 1) as u8])
            }));
            let visible = e.export_cell(0);
            e.split_grid = [2, 2];
            e.apply_split().unwrap();
            assert_eq!(e.merged_tile_image(), visible);
        }
    }

    #[test]
    fn invalid_splits_leave_existing_cells_intact() {
        let mut e = Editor::default();
        e.set_single_tile(source(12, 8));
        e.split_grid = [3, 3];
        assert!(e.apply_split().is_err());
        e.set_preview_layout(true, 2, 2);
        e.layers[0].tiles[3].put_pixel(2, 2, Rgba([75; 4]));
        e.split_grid = [2, 2];
        let before = e.snapshot();
        assert!(e.apply_split().is_err());
        assert!(e.snapshot() == before && !e.changed && e.undo.is_empty());
    }
}
