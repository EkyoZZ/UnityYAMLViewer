use super::*;
use sha2::{Digest, Sha256};
use std::io::{BufWriter, Cursor, Read, Seek, Write};
use std::sync::mpsc;
mod psd;

#[derive(PartialEq)]
pub(super) struct FileVersion([u8; 32]);
impl FileVersion {
    pub(super) fn from_bytes(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    fn read(path: &Path) -> Result<Self, String> {
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut hash).map_err(|e| e.to_string())?;
        Ok(Self(hash.finalize().into()))
    }
}

// A bitmap owns both drafts. Project previews use a standalone Tile editor.
#[derive(Default)]
pub(in super::super) struct Workspace {
    pub open: bool,
    tile_mode: bool,
    image: Editor,
    tile: Editor,
    loaded_image: bool,
    loading: Option<mpsc::Receiver<Result<OpenedImage, String>>>,
    load_failed: bool,
    seeded_tile: bool,
}
pub(super) const MAX_IMAGE_PIXELS: u64 = 16 * 1024 * 1024;
pub(super) const MAX_IMAGE_SIDE: u32 = 8192;

struct OpenedImage {
    pixels: RgbaImage,
    version: FileVersion,
    layers: Vec<Layer>,
    resources: Vec<u8>,
    patterns: Vec<u8>,
    restriction: Option<String>,
}

fn open_image(path: &Path, max_side: u32) -> Result<OpenedImage, String> {
    const MAX_FILE: u64 = 256 * 1024 * 1024;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE {
        return Err(tr("当前图像超出编辑画布限制。").into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(tr("当前图像超出编辑画布限制。").into());
    }
    let version = FileVersion(Sha256::digest(&bytes).into());
    let cursor = Cursor::new(&bytes);
    let is_psd = super::super::is_psd(path);
    let check_dimensions = |w: u32, h: u32| {
        if w == 0 || h == 0 || w.max(h) > max_side || u64::from(w) * u64::from(h) > MAX_IMAGE_PIXELS
        {
            Err(tr("当前图像超出编辑画布限制。").to_owned())
        } else {
            Ok(())
        }
    };
    if is_psd {
        let header = bytes.get(14..22).ok_or("Truncated PSD header")?;
        let height = u32::from_be_bytes(header[..4].try_into().unwrap());
        let width = u32::from_be_bytes(header[4..].try_into().unwrap());
        check_dimensions(width, height)?;
        let (layers, resources, patterns, restriction) = match psd::decode(&bytes, (width, height)) {
            Ok((layers, resources, patterns, has_unwritten_metadata)) => {
                let mut notices = Vec::new();
                if layers.iter().any(|layer| layer.settings.psd_native_text && layer.settings.text.is_none()) {
                    notices.push(tr("PSD 原生文字以独立像素图层读取，请另存为新文件。"));
                }
                if has_unwritten_metadata {
                    notices.push(tr("已读取 PSD 图层。源文件含额外的 PSD 元数据，请另存为新文件。"));
                }
                let restriction = (!notices.is_empty()).then(|| notices.join("\n"));
                (layers, resources, patterns, restriction)
            }
            Err(reason) => (
                vec![],
                vec![],
                vec![],
                Some(format!(
                    "{}\n{reason}",
                    tr("此 PSD 以合成图打开，请另存为新文件。")
                )),
            ),
        };
        let pixels = if let Some(layer) = layers.last() {
            layer.tiles[0].clone()
        } else {
            if !psd::has_merged_preview(&bytes)? {
                return Err(tr("此 PSD 缺少兼容合成图。").into());
            }
            super::super::psd::decode(cursor)?
        };
        return Ok(OpenedImage {
            pixels,
            version,
            layers,
            resources,
            patterns,
            restriction,
        });
    }
    let pixels = {
        let format = image::ImageFormat::from_path(path).map_err(|e| e.to_string())?;
        let (width, height) = image::ImageReader::with_format(cursor.clone(), format)
            .into_dimensions()
            .map_err(|e| e.to_string())?;
        check_dimensions(width, height)?;
        let mut reader = image::ImageReader::with_format(cursor, format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(max_side);
        limits.max_image_height = Some(max_side);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        reader.decode().map_err(|e| e.to_string())?.into_rgba8()
    };
    let restriction = (!flat_source_editable(path, &bytes))
        .then(|| tr("此图含动画或高位深数据，当前画面可另存为新文件。").to_owned());
    let layers = vec![];
    let resources = vec![];
    Ok(OpenedImage {
        pixels,
        version,
        layers,
        resources,
        patterns: vec![],
        restriction,
    })
}

impl Workspace {
    pub(in super::super) fn open_image_tab(&mut self, path: &Path) {
        if !self.open
            && self.image.image_target.as_deref().is_some_and(|target| !same_file(path, target))
        {
            self.image = Editor::default();
            self.loaded_image = false;
            self.load_failed = false;
        }
        self.open = true;
    }

    pub(in super::super) fn active(&self) -> bool {
        self.open || self.image.changed || self.tile.changed
    }
    pub(in super::super) fn take_saved(&mut self) -> Option<std::path::PathBuf> {
        self.image.image_saved.take()
    }
    pub(in super::super) fn show(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        source: Option<&RgbaImage>,
    ) {
        if !self.open {
            return;
        }
        let max_side = MAX_IMAGE_SIDE.min(ctx.input(|i| i.max_texture_side) as u32);
        let eligible = (source.is_none() && super::super::is_psd(path))
            || source.is_some_and(|im| {
                im.width().max(im.height()) <= max_side
                    && u64::from(im.width()) * u64::from(im.height()) <= MAX_IMAGE_PIXELS
            });
        if !self.tile_mode && !self.loaded_image && !self.load_failed {
            self.image.plain_image = true;
            if !eligible {
                self.image.message = tr("当前图像超出编辑画布限制。").into();
            } else if self.loading.is_none() {
                let (tx, rx) = mpsc::sync_channel(1);
                self.loading = Some(rx);
                let file = path.to_owned();
                let ctx = ctx.clone();
                self.image.message = tr("正在读取图像").into();
                std::thread::spawn(move || {
                    let result = open_image(&file, max_side);
                    let _ = tx.send(result);
                    ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile", &file))));
                    ctx.request_repaint();
                });
            }
            if let Some(result) = self.loading.as_ref().and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(tr("读取失败").into())),
            }) {
                self.loading = None;
                match result {
                    Ok(doc) => {
                        self.image.init_image(&doc.pixels);
                        self.image.image_target = Some(path.to_owned());
                        self.image.image_version = Some(doc.version);
                        self.image.fx_light = psd::global_light(&doc.resources);
                        self.image.psd_resources = doc.resources;
                        self.image.psd_patterns = doc.patterns;
                        self.image.image_readonly = doc.restriction.is_some();
                        self.image.image_source_protected = self.image.image_readonly;
                        self.image.message = doc.restriction.unwrap_or_default();
                        if let Some(i) = doc.layers.len().checked_sub(1) {
                            self.image.tiles = doc.layers[i].tiles.clone();
                            self.image.pixels = self.image.tiles[0].clone();
                            self.image.layers = doc.layers;
                            self.image.active_layer = i;
                            self.image.selected_layers = vec![i];
                            self.image.layer_anchor = i;
                        }
                        self.loaded_image = true;
                    }
                    Err(error) => {
                        self.load_failed = true;
                        self.image.message = error;
                    }
                }
            }
        }
        if self.tile_mode && !self.seeded_tile {
            if let Some(im) = source {
                if im.width().max(im.height()) <= MAX_SIDE
                    && im.width() % 2 == 0
                    && im.height() % 2 == 0
                {
                    self.tile.diamond = false;
                    self.tile.set_single_tile(im.clone());
                } else {
                    self.tile.message =
                        tr("原图不符合 Tile 尺寸限制，已打开独立的空白 Tile 画布。").into();
                }
            }
            self.seeded_tile = true;
        }
        let was_tile = self.tile_mode;
        let (active, other) = if was_tile {
            (&mut self.tile, &self.image)
        } else {
            (&mut self.image, &self.tile)
        };
        active.open = true;
        active.show_window(
            ctx,
            path,
            source,
            Some(&mut self.tile_mode),
            other.changed,
            !was_tile && !self.loaded_image,
        );
        if active.discard_requested {
            *self = Self::default();
            return;
        }
        if !active.open {
            // A saved PNG is flat. Keep editable layers in memory while this bitmap tab is open.
            self.open = false;
            self.tile_mode = false;
        } else if was_tile != self.tile_mode {
            active.finish_stroke();
            active.layer_detail = None;
            active.text_dialog = None;
            active.open = false;
            ctx.request_repaint();
            ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile", path))));
        }
    }
}

impl Editor {
    pub(super) fn init_image(&mut self, im: &RgbaImage) {
        self.path_tool.clear_selection();
        self.psd_patterns.clear();
        self.guides.clear();
        self.inline_text = None;
        self.plain_image = true;
        self.image_alpha = true;
        self.image_size_draft = None;
        self.diamond = false;
        self.wrap = false;
        self.grid = false;
        self.width = im.width();
        self.height = im.height();
        self.footprint = [im.width(), im.height()];
        self.pixels = im.clone();
        self.tiles = vec![im.clone()];
        self.layers = vec![Layer {
            settings: Default::default(),
            name: tr("图像").into(),
            visible: true,
            locked: false,
            material: false,
            tiles: vec![im.clone()],
        }];
        self.active_layer = 0;
        self.selected_layers = vec![0];
        self.layer_anchor = 0;
        self.zoom = (760. / im.width().max(im.height()) as f32).clamp(0.05, 1.);
        self.invalidate_textures();
    }
    pub(super) fn layer_budget(&self) -> usize {
        if self.plain_image {
            128 * 1024 * 1024
        } else {
            LAYER_BYTES
        }
    }
    pub(super) fn image_controls(&mut self, ui: &mut egui::Ui, path: &Path) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.changed && !self.image_readonly && self.image_version.is_some(),
                    egui::Button::new(tr("保存")),
                )
                .clicked()
            {
                self.save_image(path, false);
            }
            if ui.button(tr("另存为")).clicked() {
                self.save_image(path, true);
            }
            ui.checkbox(&mut self.image_alpha, tr("透明度"));
        });
        if !self.image_alpha {
            let [r, g, b, _] = self.drawing_background;
            ui.small(format!(
                "{}  #{r:02X}{g:02X}{b:02X}",
                tr("普通图片使用背景色填充透明区域，PSD 保留透明度。")
            ));
        }
        self.image_size_controls(ui);
        self.layer_controls(ui);
        self.background_controls(ui);
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
    }
    pub(super) fn save_image(&mut self, source: &Path, choose_path: bool) {
        self.finish_stroke();
        if !choose_path && !self.changed {
            return;
        }
        self.sync_layer();
        let current = self.image_target.as_deref().unwrap_or(source);
        let dest = if choose_path {
            let name = current.file_stem().unwrap_or_default().to_string_lossy();
            let psd_label = if psd::needs_merged(self) {
                tr("PSD（合成图）")
            } else {
                tr("PSD（像素图层）")
            };
            let extension = super::super::extension(current);
            let jpeg_extensions = if extension == "jpeg" { ["jpeg", "jpg"] } else { ["jpg", "jpeg"] };
            let mut formats = [
                ("PNG", &["png"][..]),
                (psd_label, &["psd"][..]),
                ("JPEG", &jpeg_extensions[..]),
                ("WebP", &["webp"][..]),
                ("BMP", &["bmp"][..]),
                ("TGA", &["tga"][..]),
            ];
            if let Some(index) = formats.iter().position(|(_, extensions)| extensions.contains(&extension.as_str())) {
                formats[..=index].rotate_right(1);
            }
            let mut dialog = rfd::FileDialog::new()
                .set_directory(current.parent().unwrap_or(Path::new(".")))
                .set_file_name(format!("{name}-edited"));
            for (label, extensions) in formats {
                dialog = dialog.add_filter(label, extensions);
            }
            let Some(p) = dialog.save_file() else {
                return;
            };
            p
        } else {
            current.to_owned()
        };
        let same = same_file(current, &dest);
        if (same && self.image_readonly)
            || (self.image_source_protected && same_file(source, &dest))
        {
            self.message = tr("请另存到其他文件，保留源图片。").into();
            return;
        }
        let ext = super::super::extension(&dest);
        if ext == "psd" && psd::needs_merged(self) && same && !self.image_saved_as_merged_psd {
            self.message = tr("当前效果需合并后导出 PSD，请另存为新文件。").into();
            return;
        }
        let expected = if same {
            self.image_version.as_ref()
        } else {
            None
        };
        if same && expected.is_none() {
            self.message = tr("图片已被其他程序修改，请重载后重试。").into();
            return;
        }
        match write_image(&dest, expected, |f| self.encode_image(f, &ext)) {
            Ok(version) => {
                self.image_version = Some(version);
                self.image_target = Some(dest.clone());
                self.image_readonly = false;
                self.image_saved_as_merged_psd = ext == "psd" && psd::needs_merged(self);
                self.image_saved = Some(dest.clone());
                self.changed = false;
                self.message = format!("{} · {}", tr("已保存"), dest.display());
            }
            Err(err) => self.message = err.to_string(),
        }
    }
    fn encode_image(&self, writer: &mut (impl Write + Seek), ext: &str) -> Result<(), String> {
        let mut out = self.composite(0);
        if ext == "psd" {
            return psd::write(writer, self, &out);
        }
        if !self.image_alpha {
            let [r, g, b, _] = self.drawing_background;
            fill_transparency(&mut out, [r, g, b]);
        }
        write_flat(writer, ext, &out, self.image_alpha)
    }
}
fn flat_source_editable(path: &Path, bytes: &[u8]) -> bool {
    use image::ImageDecoder;
    let r = Cursor::new(bytes);
    match super::super::extension(path).as_str() {
        "png" => image::codecs::png::PngDecoder::new(r).is_ok_and(|d| {
            d.is_apng().is_ok_and(|a| !a)
                && d.original_color_type().bits_per_pixel()
                    <= 8 * d.original_color_type().channel_count() as u16
        }),
        "webp" => image::codecs::webp::WebPDecoder::new(r).is_ok_and(|d| !d.has_animation()),
        _ => true,
    }
}
fn fill_transparency(image: &mut RgbaImage, background: [u8; 3]) {
    for pixel in image.pixels_mut() {
        let a = u32::from(pixel[3]);
        for c in 0..3 {
            pixel[c] = ((u32::from(pixel[c]) * a + u32::from(background[c]) * (255 - a) + 127)
                / 255) as u8;
        }
        pixel[3] = 255;
    }
}

fn write_flat(
    out: &mut (impl Write + Seek),
    ext: &str,
    image: &RgbaImage,
    alpha: bool,
) -> Result<(), String> {
    let format = image::ImageFormat::from_extension(ext)
        .filter(|f| {
            matches!(
                f,
                image::ImageFormat::Png
                    | image::ImageFormat::Jpeg
                    | image::ImageFormat::WebP
                    | image::ImageFormat::Bmp
                    | image::ImageFormat::Tga
            )
        })
        .ok_or_else(|| tr("不支持的图像保存格式").to_owned())?;
    if format == image::ImageFormat::Jpeg {
        let rgb = image::RgbImage::from_fn(image.width(), image.height(), |x, y| {
            let p = image.get_pixel(x, y);
            let a = u32::from(p[3]);
            image::Rgb(std::array::from_fn(|i| {
                ((u32::from(p[i]) * a + 255 * (255 - a) + 127) / 255) as u8
            }))
        });
        image::codecs::jpeg::JpegEncoder::new_with_quality(out, 95)
            .encode_image(&rgb)
            .map_err(|e| e.to_string())
    } else if !alpha {
        let rgb = image::RgbImage::from_fn(image.width(), image.height(), |x, y| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            image::Rgb([r, g, b])
        });
        rgb.write_to(out, format).map_err(|e| e.to_string())
    } else {
        image.write_to(out, format).map_err(|e| e.to_string())
    }
}
pub(super) fn write_image(
    path: &Path,
    expected: Option<&FileVersion>,
    encode: impl FnOnce(&mut BufWriter<std::fs::File>) -> Result<(), String>,
) -> Result<FileVersion, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = path.with_file_name(format!(".uyimage-{}-{id}.tmp", std::process::id()));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut out = BufWriter::new(file);
        encode(&mut out)?;
        out.flush()
            .and_then(|_| out.get_ref().sync_all())
            .map_err(|e| e.to_string())?;
        drop(out);
        let version = FileVersion::read(&staging)?;
        if let Some(original) = expected {
            if &FileVersion::read(path)? != original {
                return Err(tr("图片已被其他程序修改，请重载后重试。").into());
            }
        }
        std::fs::rename(&staging, path).map_err(|e| e.to_string())?;
        Ok(version)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alpha_output_fills_with_the_chosen_color_without_changing_editable_pixels() {
        let pixels = RgbaImage::from_fn(3, 1, |x, _| match x {
            0 => Rgba([0; 4]),
            1 => Rgba([240, 31, 63, 128]),
            _ => Rgba([20, 50, 81, 255]),
        });
        let mut e = Editor::default();
        e.init_image(&pixels);
        let original = e.composite(0);
        assert!(e.image_alpha);
        e.drawing_background = [16, 32, 64, 255];
        e.background_color = [240, 240, 240];
        for alpha in [false, true] {
            e.image_alpha = alpha;
            let mut expected = original.clone();
            if !alpha {
                *expected.get_pixel_mut(0, 0) = Rgba([16, 32, 64, 255]);
                *expected.get_pixel_mut(1, 0) = Rgba([128, 31, 63, 255]);
            }
            for ext in ["png", "psd"] {
                let mut bytes = Cursor::new(Vec::new());
                e.encode_image(&mut bytes, ext).unwrap();
                let data = bytes.into_inner();
                let decoded = if ext == "png" {
                    assert_eq!(data[25], if alpha { 6 } else { 2 });
                    image::load_from_memory(&data).unwrap().to_rgba8()
                } else {
                    assert_eq!(data[13], 4);
                    psd::decode(&data, (3, 1)).unwrap().0[0].tiles[0].clone()
                };
                assert_eq!(&decoded, if ext == "psd" { &original } else { &expected });
            }
        }
        assert_eq!(e.pixels, pixels);
        assert!(!e.changed && e.undo.is_empty());
    }

    #[test]
    fn both_layered_and_composite_psd_ignore_the_flat_image_alpha_setting() {
        let mut e = Editor::default();
        e.init_image(&RgbaImage::new(2, 1));
        let mut top = e.layers[0].clone();
        top.tiles[0].put_pixel(1, 0, Rgba([190, 51, 82, 107]));
        e.layers.push(top);
        e.background_color = [250, 250, 0];
        for composite_only in [false, true] {
            if composite_only {
                let mut adjustment = e.layers[0].clone();
                adjustment.settings.adjustment = Some(Default::default());
                e.layers.push(adjustment);
            }
            let mut encoded = Vec::new();
            for alpha in [true, false] {
                e.image_alpha = alpha;
                assert_eq!(psd::needs_merged(&e), composite_only);
                let mut out = Cursor::new(Vec::new());
                e.encode_image(&mut out, "psd").unwrap();
                let bytes = out.into_inner();
                let (layers, _, _, _) = psd::decode(&bytes, (2, 1)).unwrap();
                assert_eq!(layers.len(), if composite_only { 1 } else { 2 });
                let merged = super::super::super::psd::decode(Cursor::new(&bytes)).unwrap();
                assert_eq!(merged.get_pixel(0, 0)[3], 0);
                assert_eq!(merged.get_pixel(1, 0)[3], 107);
                encoded.push(bytes);
            }
            assert_eq!(encoded[0], encoded[1]);
        }
    }
    #[test]
    fn image_uses_odd_dimensions_and_has_independent_tile_history() {
        let mut w = Workspace::default();
        w.image
            .init_image(&RgbaImage::from_pixel(5, 3, Rgba([1, 2, 3, 255])));
        assert_eq!(w.image.finite_plane_hit(-2, -1, false), Some((0, 0, 0)));
        assert_eq!(w.image.finite_plane_hit(2, 1, false), Some((0, 4, 2)));
        assert_eq!(w.image.finite_plane_hit(3, 1, false), None);
        w.image.record_layer_change();
        w.image.pixels.put_pixel(0, 0, Rgba([7, 8, 9, 255]));
        assert!(w.image.changed && !w.tile.changed && w.tile.undo.is_empty());
        assert!(!w.image.layers[0].material && !w.image.wrap);
        w.image.history(false);
        assert_eq!(w.image.pixels.get_pixel(0, 0).0, [1, 2, 3, 255]);
    }

    #[test]
    fn failed_encode_or_external_edit_preserves_destination() {
        let dir = std::env::temp_dir().join(format!(
            "uy-image-conflict-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layer.png");
        std::fs::write(&path, b"first").unwrap();
        let version = FileVersion::read(&path).unwrap();
        std::fs::write(&path, b"other").unwrap();
        assert!(
            write_image(&path, Some(&version), |w| w
                .write_all(b"changed")
                .map_err(|e| e.to_string()))
            .is_err()
        );
        assert!(write_image(&path, None, |_| Err("encode".into())).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"other");
        let next = FileVersion::read(&path).unwrap();
        write_image(&path, Some(&next), |w| {
            w.write_all(b"saved").map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"saved");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
