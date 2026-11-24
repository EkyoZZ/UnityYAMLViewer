use super::*;

impl Editor {
    pub(super) fn export_tile_png(&mut self, source: &Path, merged: bool) {
        self.finish_stroke();
        let name = if merged {
            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("tile");
            format!("{stem}-{}x{}.png", self.columns, self.rows)
        } else {
            String::from("tile.png")
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        if same_file(source, &path) {
            self.message = tr("请另存到其他文件，保留源图片。").into();
            return;
        }
        let image = if merged {
            self.merged_tile_image()
        } else {
            self.export_cell(self.selected)
        };
        let result = bitmap_workspace::write_image(&path, None, |out| {
            image
                .write_to(out, image::ImageFormat::Png)
                .map_err(|e| e.to_string())
        });
        self.message = match result {
            Ok(_) => format!("{} · {}", tr("已保存"), path.display()),
            Err(error) => error,
        };
    }

    pub(super) fn merged_tile_image(&self) -> RgbaImage {
        if !self.has_multiple_slots() {
            return self.export_cell(self.selected);
        }
        let (w, h) = self.pixels.dimensions();
        let size = egui::vec2(w as f32, h as f32);
        let mut cells = Vec::with_capacity(self.rows * self.columns);
        let mut bounds = Rect::NOTHING;
        for row in 0..self.rows {
            for col in 0..self.columns {
                let center = self.mapped_center(col as i32, row as i32, w, h).to_pos2();
                let rect = Rect::from_center_size(center, size);
                bounds = bounds.union(rect);
                cells.push((self.tile_at(col as i32, row as i32), rect.min));
            }
        }
        // Rectangles, diamonds and hexagons each occupy one contiguous span per scanline.
        // Copy owned pixels verbatim, including partial alpha along a shared edge.
        let spans: Vec<_> = (0..h)
            .map(|y| {
                if !self.diamond && self.hex == 0 {
                    return 0..w as usize;
                }
                let mut owned = (0..w).filter(|&x| self.inside_tile(x as i32, y as i32, w, h));
                let Some(first) = owned.next() else {
                    return 0..0;
                };
                first as usize..(owned.next_back().unwrap_or(first) as usize + 1)
            })
            .collect();
        let mut out = RgbaImage::new(bounds.width() as u32, bounds.height() as u32);
        let stride = out.width() as usize;
        for (tile, origin) in cells {
            let offset = origin - bounds.min;
            let image = self.composite(tile);
            for (y, span) in spans.iter().enumerate() {
                let src = 4 * (y * w as usize + span.start);
                let dst = 4 * ((offset.y as usize + y) * stride + offset.x as usize + span.start);
                let len = 4 * span.len();
                out.as_mut()[dst..dst + len].copy_from_slice(&image.as_raw()[src..src + len]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_png_matches_canvas_pixel_ownership_in_each_layout() {
        for (diamond, hex) in [(false, 0), (true, 0), (false, 1), (false, 2)] {
            let mut e = Editor::default();
            let (w, h) = (12, 8);
            e.set_single_tile(RgbaImage::new(w, h));
            e.set_preview_shape(diamond, hex, false);
            e.set_preview_layout(true, 3, 2);
            e.layout.swap(0, 10);
            for (i, tile) in e.layers[0].tiles.iter_mut().enumerate() {
                *tile = RgbaImage::from_fn(w, h, |x, y| Rgba([i as u8 + 1, x as u8, y as u8, 117]));
            }
            let cells: Vec<_> = (0..6).map(|i| e.export_cell(i)).collect();
            let image = e.merged_tile_image();
            let size = e.edit_size();
            assert_eq!(image.dimensions(), (size.x as u32, size.y as u32));
            let left = if diamond {
                -(e.rows as i32) * w as i32 / 2
            } else {
                -(w as i32) / 2
            };
            for (x, y, pixel) in image.enumerate_pixels() {
                let p = e.mapped_cell(x as i32 + left, y as i32 - h as i32 / 2, w, h);
                if (0..3).contains(&p.col) && (0..2).contains(&p.row) {
                    let i = e.tile_at(p.col, p.row);
                    assert_eq!(pixel, cells[i].get_pixel(p.x, p.y));
                } else {
                    assert_eq!(pixel.0, [0; 4]);
                }
            }
            e.set_preview_layout(false, 1, 1);
            assert_eq!(e.merged_tile_image(), e.export_cell(e.selected));
            assert!(!e.changed && e.undo.is_empty());
        }
    }
}
