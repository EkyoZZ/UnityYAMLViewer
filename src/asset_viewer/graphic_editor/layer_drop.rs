use super::*;
use std::path::PathBuf;

fn layer_image(path: &Path) -> bool {
    super::super::is_browsable_image(path)
}

fn layer_name(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
        .take(64)
        .collect()
}

impl Editor {
    pub(super) fn layer_file_drop(&mut self, ui: &mut egui::Ui, area: Rect) {
        if !ui.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .any(|f| f.path.as_deref().is_some_and(layer_image))
        }) {
            return;
        }
        let ctx = ui.ctx();
        let native = ctx.viewport_id() != egui::ViewportId::ROOT;
        let position = crate::native_window::file_drop_position(ctx)
            .or_else(|| ui.input(|i| i.pointer.latest_pos()));
        if !position.map_or(native, |p| {
            area.contains(p) && (native || ctx.layer_id_at(p) == Some(ui.layer_id()))
        }) {
            return;
        }
        // The Tile material library has already claimed files released inside its own border.
        let paths = ui.input_mut(|input| {
            let mut paths = Vec::new();
            input.raw.dropped_files.retain(|file| {
                if let Some(path) = file.path.as_ref().filter(|p| layer_image(p)) {
                    paths.push(path.clone());
                    false
                } else {
                    true
                }
            });
            paths
        });
        self.import_layer_files(&paths);
        ctx.request_repaint();
    }

    fn image_layer_capacity(&self) -> Result<(), &'static str> {
        if self.ancestor_locked(self.active_layer)
            || (self.layers[self.active_layer].settings.group_id != 0
                && self.layers[self.active_layer].locked)
        {
            return Err("当前图层已锁定，请先解锁或选择其他图层。");
        }
        let bytes = self.tiles.iter().map(|im| im.as_raw().len()).sum();
        if !self.can_add_layers(&[], bytes) {
            return Err("图层数量或内存已达上限，无法导入新图层。");
        }
        Ok(())
    }

    fn import_layer_files(&mut self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }
        let mut added = 0;
        let mut errors = Vec::new();
        for path in paths {
            if let Err(reason) = self.image_layer_capacity() {
                errors.push(tr(reason).to_owned());
                break;
            }
            let image = match super::super::load_preview_pixels(path) {
                Ok(im) => im,
                Err(reason) => {
                    errors.push(format!("{} · {reason}", path.display()));
                    continue;
                }
            };
            if added == 0 {
                self.record_layer_change();
            }
            self.insert_image_layer(layer_name(path), image);
            added += 1;
        }
        if added > 0 {
            self.material_drop_status = None;
            errors.insert(0, format!("{} · {added}", tr("已导入普通图层")));
        }
        self.message = errors.join("\n");
    }

    pub(super) fn centered_image(&self, image: RgbaImage) -> RgbaImage {
        let (w, h) = self.pixels.dimensions();
        let image = if image.width() > w || image.height() > h {
            let filter = if self.plain_image {
                image::imageops::FilterType::Triangle
            } else {
                image::imageops::FilterType::Nearest
            };
            image::DynamicImage::ImageRgba8(image)
                .resize(w, h, filter)
                .into_rgba8()
        } else {
            image
        };
        if image.dimensions() == (w, h) {
            return image;
        }
        let mut canvas = RgbaImage::new(w, h);
        image::imageops::replace(
            &mut canvas,
            &image,
            i64::from((w - image.width()) / 2),
            i64::from((h - image.height()) / 2),
        );
        canvas
    }

    pub(super) fn insert_image_layer(&mut self, name: String, image: RgbaImage) {
        let mut tiles: Vec<_> = self
            .tiles
            .iter()
            .map(|im| RgbaImage::new(im.width(), im.height()))
            .collect();
        tiles[self.selected] = self.centered_image(image);
        let (parent, index) = self.layer_insert_position();
        if self.layers[self.active_layer].settings.group_id != 0 {
            self.layers[self.active_layer].settings.collapsed = false;
        }
        self.layers.insert(
            index,
            Layer {
                settings: layer_properties::Properties {
                    parent,
                    ..Default::default()
                },
                name,
                visible: true,
                locked: false,
                material: false,
                tiles,
            },
        );
        if index <= self.active_layer {
            self.active_layer += 1;
        }
        self.select_layer(index);
        self.text_dialog = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importing_current_image_only_changes_selected_cell_content() {
        let mut e = Editor::default();
        e.diamond = false;
        e.set_single_tile(RgbaImage::from_pixel(8, 8, Rgba([17, 77, 91, 255])));
        e.set_preview_layout(true, 2, 2);
        e.select_slot(9);
        let before = e.snapshot();
        let untouched = e.export_cell(0);
        let source = RgbaImage::from_pixel(4, 4, Rgba([240, 63, 42, 255]));
        e.import_current_cell(&source);
        e.import_current_cell(&source);
        assert_eq!(e.undo.len(), 1);
        assert_eq!(e.layers.len(), before.layers.len());
        assert_eq!(e.export_cell(0), untouched);
        assert_eq!(e.export_cell(3).get_pixel(3, 3), source.get_pixel(0, 0));
        assert_eq!((e.columns, e.rows, e.selected_slot), (2, 2, 9));
        assert_eq!(e.pixels.dimensions(), (8, 8));
        e.history(false);
        assert!(e.snapshot() == before);
    }

    #[test]
    fn dropping_files_inserts_centered_layers_as_one_edit() {
        let prefix = format!(
            "uy-drop-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let dir = std::env::temp_dir().join(prefix);
        std::fs::create_dir_all(&dir).unwrap();
        let small = dir.join("small.png");
        let wide = dir.join("wide.png");
        let missing = dir.join("missing.png");
        let red = Rgba([220, 12, 33, 255]);
        RgbaImage::from_pixel(2, 2, red).save(&small).unwrap();
        RgbaImage::from_pixel(16, 4, red).save(&wide).unwrap();
        let mut e = Editor::default();
        e.set_single_tile(RgbaImage::new(8, 8));
        e.set_preview_layout(true, 2, 2);
        e.select_slot(9);
        e.import_layer_files(std::slice::from_ref(&missing));
        assert!(!e.changed && e.undo.is_empty());
        let before = e.snapshot();
        e.import_layer_files(&[small.clone(), missing, wide.clone()]);
        assert!(e.changed && e.undo.len() == 1);
        assert_eq!(e.layers.len(), 4);
        for (index, name, pos, count) in [(2, "small", (3, 3), 4), (3, "wide", (0, 3), 16)] {
            let layer = &e.layers[index];
            assert_eq!(layer.name, name);
            assert!(!layer.locked && !layer.material);
            assert_eq!(*layer.tiles[e.selected].get_pixel(pos.0, pos.1), red);
            assert_eq!(
                layer.tiles[e.selected]
                    .pixels()
                    .filter(|p| p[3] != 0)
                    .count(),
                count
            );
            assert!(layer.tiles[0].pixels().all(|p| p[3] == 0));
        }
        e.history(false);
        assert!(e.snapshot() == before);
        std::fs::remove_file(small).unwrap();
        std::fs::remove_file(wide).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
