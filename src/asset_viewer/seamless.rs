//! Exact rectangular periods and wrapped motif composition with PNG export.
use crate::localization::tr;
use eframe::egui;
use egui::emath::GuiRounding as _;
use image::{Pixel, Rgba, RgbaImage};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_SIDE: u32 = 8192;

// A mesh has no feathered rectangle edges; shared pixel-aligned edges avoid grid seams.
fn checker_background(painter: &egui::Painter, rect: egui::Rect, image_origin: egui::Pos2, pixel_size: egui::Vec2, background: &super::ImageBackground, custom: Option<[u8;3]>) {
    if *background!=super::ImageBackground::Checkerboard {
        let [r,g,b]=if *background==super::ImageBackground::Gray {[128,128,128]}else{custom.unwrap_or([0;3])};
        let mut mesh=egui::Mesh::default();
        mesh.add_colored_rect(rect,egui::Color32::from_rgb(r,g,b));
        painter.add(egui::Shape::mesh(mesh));
        return;
    }
    let aligned = pixel_size.x >= 8. && pixel_size.y >= 8.;
    let cell = if aligned { pixel_size } else { egui::Vec2::splat(12.) };
    let origin = if aligned { image_origin } else { rect.min };
    let first = egui::vec2((rect.left()-origin.x)/cell.x, (rect.top()-origin.y)/cell.y).floor();
    let last = egui::vec2((rect.right()-origin.x)/cell.x, (rect.bottom()-origin.y)/cell.y).ceil();
    let mut mesh = egui::Mesh::default();
    for y in first.y as i32..last.y as i32 {
        for x in first.x as i32..last.x as i32 {
            let min = origin + egui::vec2(x as f32 * cell.x, y as f32 * cell.y);
            let tile = egui::Rect::from_min_size(min, cell).intersect(rect);
            let tile = egui::Rect::from_min_max(
                tile.min.round_to_pixels(painter.pixels_per_point()), tile.max.round_to_pixels(painter.pixels_per_point()));
            mesh.add_colored_rect(tile, egui::Color32::from_gray(
                if (x + y) % 2 == 0 { 110 } else { 160 }));
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

// Keep small assets exact; preview limits must never enlarge or stretch them.
fn preview_pixels(image: &RgbaImage, limit: u32) -> RgbaImage {
    let (w, h) = image.dimensions();
    if w <= limit && h <= limit { return image.clone(); }
    let longest = w.max(h) as u64;
    let width = ((w as u64 * limit as u64) / longest).max(1) as u32;
    let height = ((h as u64 * limit as u64) / longest).max(1) as u32;
    image::imageops::resize(image, width, height, image::imageops::FilterType::Nearest)
}

fn checked_size(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w as u64 * h as u64 > MAX_PIXELS {
        return Err(
            tr("循环单元最多 8192 像素每边、总计 16777216 像素；请减小图案或间距。").into(),
        );
    }
    Ok(())
}

// KMP on complete rows/columns: exact comparison, including RGB below zero alpha.
fn period(n: u32, equal: impl Fn(u32, u32) -> bool) -> u32 {
    let mut prefix = vec![0u32; n as usize];
    for i in 1..n {
        let mut j = prefix[(i - 1) as usize];
        while j > 0 && !equal(i, j) {
            j = prefix[(j - 1) as usize];
        }
        if equal(i, j) {
            j += 1;
        }
        prefix[i as usize] = j;
    }
    let p = n - prefix[(n - 1) as usize];
    if n % p == 0 { p } else { n }
}

fn minimum_size(source: &RgbaImage) -> (u32, u32) {
    let (w, h) = source.dimensions();
    let x = period(w, |a, b| {
        (0..h).all(|y| source.get_pixel(a, y) == source.get_pixel(b, y))
    });
    let y = period(h, |a, b| {
        source.as_raw()[a as usize * w as usize * 4..(a + 1) as usize * w as usize * 4]
            == source.as_raw()[b as usize * w as usize * 4..(b + 1) as usize * w as usize * 4]
    });
    (x, y)
}
#[cfg(test)]
fn minimum(source: &RgbaImage) -> RgbaImage {
    let (x, y) = minimum_size(source);
    image::imageops::crop_imm(source, 0, 0, x, y).to_image()
}

fn aligned_repeat(source: &RgbaImage) -> Result<RgbaImage, String> {
    let (pw, ph) = minimum_size(source);
    let (w, h) = (pw / gcd(pw, 4) * 4, ph / gcd(ph, 4) * 4);
    checked_size(w, h)?;
    Ok(RgbaImage::from_fn(w, h, |x, y| {
        *source.get_pixel(x % pw, y % ph)
    }))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stagger {
    None,
    Rows,
    Columns,
}

#[derive(Clone)]
struct Settings {
    compose: bool,
    stagger: Stagger,
    denominator: u32,
    gap: [u32; 2],
    scale: u32,
    trim: bool,
    layout: String,
    transparent: bool,
    background: [u8; 3],
    offsets: Vec<[i32; 2]>,
    custom_size: bool,
    repeat_size: [u32; 2],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            compose: true,
            stagger: Stagger::None,
            denominator: 2,
            gap: [0, 0],
            scale: 100,
            trim: true,
            layout: "1".into(),
            transparent: true,
            background: [255; 3],
            offsets: vec![[0, 0]],
            custom_size: false,
            repeat_size: [4, 4],
        }
    }
}

fn grid(text: &str, count: usize) -> Result<Vec<Vec<usize>>, String> {
    let invalid = || {
        tr("排列请输入图案编号：逗号分列，分号或换行分行；每行列数相同，最多 16 × 16 格。")
            .to_owned()
    };
    let rows: Vec<Vec<usize>> = text
        .split([';', '\n'])
        .map(|row| {
            row.split(',')
                .map(|v| {
                    v.trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|v| *v > 0 && *v <= count)
                        .map(|v| v - 1)
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<_, _>>()?;
    let width = rows.first().map_or(0, Vec::len);
    if width == 0 || width > 16 || rows.len() > 16 || rows.iter().any(|r| r.len() != width) {
        return Err(invalid());
    }
    Ok(rows)
}

fn format_grid(rows: &[Vec<usize>]) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|v| (v + 1).to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn motif(source: &RgbaImage, settings: &Settings) -> Result<RgbaImage, String> {
    let mut rect = (0, 0, source.width(), source.height());
    if settings.trim {
        let (mut left, mut top, mut right, mut bottom) = (source.width(), source.height(), 0, 0);
        for (x, y, p) in source.enumerate_pixels() {
            if p[3] != 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
        if right == 0 {
            return Err(tr("图案完全透明，无法确定有效边界。").into());
        }
        rect = (left, top, right - left, bottom - top);
    }
    let w = ((rect.2 as u64 * settings.scale as u64 + 50) / 100).max(1) as u32;
    let h = ((rect.3 as u64 * settings.scale as u64 + 50) / 100).max(1) as u32;
    checked_size(w, h)?;
    let cropped = image::imageops::crop_imm(source, rect.0, rect.1, rect.2, rect.3);
    let pixels = if (w, h) == (rect.2, rect.3) {
        cropped.to_image()
    } else {
        image::imageops::resize(
            &cropped.to_image(),
            w,
            h,
            image::imageops::FilterType::Lanczos3,
        )
    };
    let (aw, ah) = (w.div_ceil(4) * 4, h.div_ceil(4) * 4);
    checked_size(aw, ah)?;
    let mut padded = RgbaImage::new(aw, ah);
    image::imageops::replace(
        &mut padded,
        &pixels,
        ((aw - w) / 2) as i64,
        ((ah - h) / 2) as i64,
    );
    Ok(padded)
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}
#[derive(Clone)]
struct Placement {
    cell: usize,
    origin: [i32; 2],
    size: [u32; 2],
}
struct Scene {
    original_fallback: bool,
    image: RgbaImage,
    placements: Vec<Placement>,
    effective_gap: Option<[u32; 2]>,
    edit_placements: Vec<Placement>,
    motifs: Vec<RgbaImage>,
    offsets: Vec<[i32; 2]>,
}

#[cfg(test)]
fn pick_motif(placements: &[Placement], point: [f32; 2], size: [u32; 2]) -> Option<usize> {
    placements
        .iter()
        .rev()
        .find(|p| {
            (0..2).all(|axis| {
                (point[axis] - p.origin[axis] as f32).rem_euclid(size[axis] as f32)
                    < p.size[axis] as f32
            })
        })
        .map(|p| p.cell)
}

fn dragged_offset(
    start: [i32; 2],
    press: egui::Pos2,
    current: egui::Pos2,
    scale: [f32; 2],
) -> [i32; 2] {
    let delta = current - press;
    [
        (start[0] as f32 + delta.x * scale[0])
            .round()
            .clamp(-32768., 32768.) as i32,
        (start[1] as f32 + delta.y * scale[1])
            .round()
            .clamp(-32768., 32768.) as i32,
    ]
}

fn measured_spacing(placements: &[Placement], fallback: [u32; 2]) -> [u32; 2] {
    let mut result = fallback;
    for axis in 0..2 {
        let other = 1 - axis;
        let mut nearest = None;
        for a in placements {
            for b in placements {
                if a.cell == b.cell || b.origin[axis] < a.origin[axis] {
                    continue;
                }
                if a.origin[other] + a.size[other] as i32 <= b.origin[other]
                    || b.origin[other] + b.size[other] as i32 <= a.origin[other]
                {
                    continue;
                }
                let gap = (b.origin[axis] - a.origin[axis] - a.size[axis] as i32).max(0) as u32;
                nearest = Some(nearest.map_or(gap, |v: u32| v.min(gap)));
            }
        }
        if let Some(gap) = nearest {
            result[axis] = gap;
        }
    }
    result
}

fn compose_scene(images: &[RgbaImage], settings: &Settings) -> Result<Scene, String> {
    let grid = grid(&settings.layout, images.len())?;
    let (cols, rows) = (grid[0].len() as u32, grid.len() as u32);
    let (mut cw, mut ch) = (
        images.iter().map(|i| i.width()).max().unwrap() + settings.gap[0],
        images.iter().map(|i| i.height()).max().unwrap() + settings.gap[1],
    );
    let q = settings.denominator;
    // Make fractional offsets exact integer pixels without rounding seams.
    if settings.stagger == Stagger::Rows {
        let multiple = 4 / gcd(4, q) * q;
        cw = cw.div_ceil(multiple) * multiple;
    } else {
        cw = cw.div_ceil(4) * 4;
    }
    if settings.stagger == Stagger::Columns {
        let multiple = 4 / gcd(4, q) * q;
        ch = ch.div_ceil(multiple) * multiple;
    } else {
        ch = ch.div_ceil(4) * 4;
    }
    let nx = if settings.stagger == Stagger::Columns {
        cols / gcd(cols, q) * q
    } else {
        cols
    };
    let ny = if settings.stagger == Stagger::Rows {
        rows / gcd(rows, q) * q
    } else {
        rows
    };
    let (w, h) = (cw * nx, ch * ny);
    checked_size(w, h)?;
    let background = if settings.transparent {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([
            settings.background[0],
            settings.background[1],
            settings.background[2],
            255,
        ])
    };
    let mut placements = Vec::new();
    let mut edit_placements = Vec::new();
    let mut lower = [0i64, 0];
    let mut upper = [w as i64, h as i64];
    for row in 0..ny {
        for col in 0..nx {
            let index = grid[(row % rows) as usize][(col % cols) as usize];
            let cell = ((row % rows) * cols + col % cols) as usize;
            let image = &images[index];
            let ox = col * cw
                + (cw - image.width()) / 2
                + if settings.stagger == Stagger::Rows {
                    (row % q) * cw / q
                } else {
                    0
                };
            let oy = row * ch
                + (ch - image.height()) / 2
                + if settings.stagger == Stagger::Columns {
                    (col % q) * ch / q
                } else {
                    0
                };
            let offset = settings.offsets.get(cell).copied().unwrap_or([0, 0]);
            let origin = [
                (ox % w) as i64 + offset[0] as i64,
                (oy % h) as i64 + offset[1] as i64,
            ];
            if row < rows && col < cols {
                edit_placements.push(Placement {
                    cell,
                    origin: [ox as i32 + offset[0], oy as i32 + offset[1]],
                    size: [image.width(), image.height()],
                });
            }
            for axis in 0..2 {
                if offset[axis] != 0 {
                    lower[axis] = lower[axis].min(origin[axis]);
                    let extent = if axis == 0 {
                        image.width()
                    } else {
                        image.height()
                    };
                    upper[axis] = upper[axis].max(origin[axis] + extent as i64);
                }
            }
            placements.push(Placement {
                cell,
                origin: [origin[0] as i32, origin[1] as i32],
                size: [image.width(), image.height()],
            });
        }
    }
    let padding = [
        ((-lower[0]) as u32).div_ceil(4) * 4,
        ((-lower[1]) as u32).div_ceil(4) * 4,
    ];
    let width = (upper[0] as u32 + padding[0]).div_ceil(4) * 4;
    let height = (upper[1] as u32 + padding[1]).div_ceil(4) * 4;
    checked_size(width, height)?;
    let mut output = RgbaImage::from_pixel(width, height, background);
    for (n, placement) in placements.iter_mut().enumerate() {
        let row = n as u32 / nx;
        let col = n as u32 % nx;
        let image = &images[grid[(row % rows) as usize][(col % cols) as usize]];
        placement.origin[0] += padding[0] as i32;
        placement.origin[1] += padding[1] as i32;
        let [ox, oy] = placement.origin.map(|v| v as u32);
        for (x, y, p) in image.enumerate_pixels() {
            output
                .get_pixel_mut((ox + x) % width, (oy + y) % height)
                .blend(p);
        }
    }
    let effective_gap = Some(measured_spacing(
        &edit_placements,
        [
            width
                .div_ceil(nx)
                .saturating_sub(images.iter().map(|i| i.width()).max().unwrap()),
            height
                .div_ceil(ny)
                .saturating_sub(images.iter().map(|i| i.height()).max().unwrap()),
        ],
    ));
    Ok(Scene {
        original_fallback: false,
        image: output,
        placements,
        effective_gap,
        edit_placements,
        motifs: images
            .iter()
            .map(|image| preview_pixels(image, 1024))
            .collect(),
        offsets: settings.offsets.clone(),
    })
}

#[cfg(test)]
fn compose(images: &[RgbaImage], settings: &Settings) -> Result<RgbaImage, String> {
    Ok(minimum(&compose_scene(images, settings)?.image))
}

fn generate_scene(
    source: &RgbaImage,
    paths: &[PathBuf],
    settings: &Settings,
) -> Result<Scene, String> {
    if !settings.compose {
        // Source tabs already enforce decoding limits; extraction adds no full-size copies.
        let (w, h) = minimum_size(source);
        let original_fallback = (w, h) == source.dimensions();
        checked_size(w, h)?;
        return Ok(Scene {
            original_fallback,
            image: if original_fallback {
                source.clone()
            } else {
                aligned_repeat(source)?
            },
            placements: Vec::new(),
            effective_gap: None,
            edit_placements: vec![Placement {
                cell: 0,
                origin: [0, 0],
                size: [source.width(), source.height()],
            }],
            motifs: vec![preview_pixels(source, 1024)],
            offsets: vec![[0, 0]],
        });
    }
    let mut images = vec![motif(source, settings)?];
    let mut total = images[0].width() as u64 * images[0].height() as u64;
    for path in paths {
        let mut reader = image::ImageReader::open(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_SIDE);
        limits.max_image_height = Some(MAX_SIDE);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let image = motif(
            &reader
                .decode()
                .map_err(|e| format!("{}: {e}", path.display()))?
                .to_rgba8(),
            settings,
        )?;
        total += image.width() as u64 * image.height() as u64;
        if total > MAX_PIXELS {
            return Err(tr("图案总像素超出限制，请缩小图案。").into());
        }
        images.push(image);
    }
    compose_scene(&images, settings)
}

struct Preview {
    scene: Scene,
    repeat: RgbaImage,
}
fn motif_list_button(ui: &mut egui::Ui, label: &str, width: f32, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.), egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let padding = ui.spacing().button_padding.x;
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        egui::TextStyle::Button.resolve(ui.style()),
        visuals.text_color(),
        (width - padding * 2.).max(1.),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(
        egui::pos2(
            rect.left() + padding,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        visuals.text_color(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response
}
fn output_tile(image: &RgbaImage, settings: &Settings) -> Result<RgbaImage, String> {
    if !settings.compose || !settings.custom_size {
        return aligned_repeat(image);
    }
    let [w, h] = settings.repeat_size.map(|v| v.max(4).div_ceil(4) * 4);
    checked_size(w, h)?;
    let background = if settings.transparent {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([
            settings.background[0],
            settings.background[1],
            settings.background[2],
            255,
        ])
    };
    Ok(RgbaImage::from_fn(w, h, |x, y| {
        if x < image.width() && y < image.height() {
            *image.get_pixel(x, y)
        } else {
            background
        }
    }))
}
fn generate(source: &RgbaImage, paths: &[PathBuf], settings: &Settings) -> Result<Preview, String> {
    let scene = generate_scene(source, paths, settings)?;
    let repeat = if scene.original_fallback {
        scene.image.clone()
    } else {
        output_tile(&scene.image, settings)?
    };
    Ok(Preview { scene, repeat })
}
type Generated = Result<Preview, String>;
#[derive(Default)]
pub(super) struct Editor {
    pub open: bool,
    settings: Settings,
    preview_background: super::ImageBackground,
    preview_background_color: Option<[u8;3]>,
    paths: Vec<PathBuf>,
    pending: Option<(u64, mpsc::Receiver<Generated>)>,
    saving: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    output: Option<Arc<RgbaImage>>,
    texture: Option<egui::TextureHandle>,
    repeat_output: Option<Arc<RgbaImage>>,
    repeat_texture: Option<egui::TextureHandle>,
    source: Option<Arc<RgbaImage>>,
    message: String,
    borders: bool,
    revision: u64,
    displayed_revision: Option<u64>,
    dirty_since: Option<std::time::Instant>,
    placements: Vec<Placement>,
    selected: Option<usize>,
    drag_origin: Option<[i32; 2]>,
    drag_pointer_origin: Option<egui::Pos2>,
    drag_scale: Option<[f32; 2]>,
    effective_gap: Option<[u32; 2]>,
    edit_placements: Vec<Placement>,
    motif_textures: Vec<egui::TextureHandle>,
    canvas_zoom: f32,
    canvas_pan: egui::Vec2,
    measured_gap: Option<[u32; 2]>,
    snapshot_offsets: Vec<[i32; 2]>,
}
impl Editor {
    fn save_button(&mut self, ui: &mut egui::Ui, path: &Path) {
        let ctx = ui.ctx().clone();
        if ui
            .add_enabled(
                self.saving.is_none()
                    && self.output.is_some()
                    && self.repeat_output.is_some()
                    && self.displayed_revision == Some(self.revision)
                    && self.dirty_since.is_none()
                    && self.pending.is_none(),
                egui::Button::new(egui::RichText::new(tr("另存为")).size(16.0).color(egui::Color32::WHITE))
                    .fill(egui::Color32::from_rgb(30,150,85))
                    .stroke((1.0,egui::Color32::from_rgb(30,150,85)))
                    .corner_radius(4.0)
                    .min_size(egui::vec2(120.0,40.0)),
            )
            .clicked()
        {
            let name = format!(
                "{}_tile.png",
                path.file_stem().unwrap_or_default().to_string_lossy()
            );
            if let Some(destination) = rfd::FileDialog::new()
                .set_file_name(name)
                .add_filter("PNG", &["png"])
                .save_file()
            {
                // The native save dialog confirms replacement of existing files.
                let image = self.repeat_output.as_ref().unwrap().clone();
                let ctx = ctx.clone();
                let (tx, rx) = mpsc::channel();
                self.saving = Some(rx);
                std::thread::spawn(move || {
                    let result = (|| {
                        // Encode before opening the destination so encoding errors preserve it.
                        let mut encoded = std::io::Cursor::new(Vec::new());
                        image::DynamicImage::ImageRgba8((*image).clone())
                            .write_to(&mut encoded, image::ImageFormat::Png)
                            .map_err(|e| e.to_string())?;
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&destination)
                            .map_err(|e| e.to_string())?;
                        std::io::Write::write_all(&mut file, encoded.get_ref())
                            .and_then(|_| file.sync_all())
                            .map_err(|e| e.to_string())?;
                        Ok(destination)
                    })();
                    let _ = tx.send(result);
                    ctx.request_repaint();
                });
            }
        }
    }
    fn clear_editor_view(&mut self) {
        self.edit_placements.clear();
        self.motif_textures.clear();
        self.snapshot_offsets.clear();
        self.canvas_zoom = 0.;
        self.canvas_pan = egui::Vec2::ZERO;
        self.measured_gap = None;
        self.drag_origin = None;
        self.drag_pointer_origin = None;
        self.drag_scale = None;
    }
    fn live_placements(&self) -> Vec<Placement> {
        self.edit_placements
            .iter()
            .map(|p| {
                let mut p = p.clone();
                let old = self.snapshot_offsets.get(p.cell).copied().unwrap_or([0, 0]);
                let now = self.settings.offsets.get(p.cell).copied().unwrap_or(old);
                for axis in 0..2 {
                    p.origin[axis] += now[axis] - old[axis];
                }
                p
            })
            .collect()
    }
    fn preview_header(&mut self, ui: &mut egui::Ui, path: &Path, index: usize) {
        let height=44.0_f32.max(ui.text_style_height(&egui::TextStyle::Body)*2.0+ui.spacing().item_spacing.y);
        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(),height),egui::Layout::left_to_right(egui::Align::Center),|ui| {
            if index==0 {
                ui.vertical(|ui| {ui.label(tr("实时编辑"));self.editor_toolbar(ui);});
            }else{
                let title=tr(if index==1 {"最小循环单元预览"}else{"连续平铺预览"});
                let dimensions=self.repeat_output.as_ref().map(|image|format!("{} × {} px",image.width(),image.height())).unwrap_or_default();
                let font=egui::TextStyle::Body.resolve(ui.style());
                let text_width=[title,dimensions.as_str()].into_iter().map(|text|
                    ui.painter().layout_no_wrap(text.to_owned(),font.clone(),ui.visuals().text_color()).size().x
                ).fold(0.0_f32,f32::max);
                ui.allocate_ui_with_layout(egui::vec2(text_width,height),egui::Layout::top_down(egui::Align::Min),|ui| {
                    ui.label(title);
                    if !dimensions.is_empty() {ui.label(dimensions);}
                });
                if index==1 {self.save_button(ui,path);}
            }
        });
    }
    fn editor_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{:.0}%", self.canvas_zoom.max(0.01) * 100.));
            if ui.small_button(tr("适应窗口")).clicked() {
                self.canvas_zoom = 0.;
            }
            ui.checkbox(&mut self.borders, tr("显示单元边框"));
        });
    }
    fn show_editor_canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(egui::vec2(1., 1.)),
            egui::Sense::click_and_drag(),
        );
        let mut placements = self.live_placements();
        if self.canvas_zoom <= 0. {
            let bounds = placements.iter().fold(egui::Rect::NOTHING, |r, p| {
                r.union(egui::Rect::from_min_size(
                    egui::pos2(p.origin[0] as f32, p.origin[1] as f32),
                    egui::vec2(p.size[0] as f32, p.size[1] as f32),
                ))
            });
            if bounds.is_finite() {
                self.canvas_zoom = ((rect.width() - 32.).max(1.) / bounds.width().max(1.))
                    .min((rect.height() - 32.).max(1.) / bounds.height().max(1.))
                    .clamp(0.05, 32.);
                self.canvas_pan = -bounds.center().to_vec2() * self.canvas_zoom;
            } else {
                self.canvas_zoom = 1.;
                self.canvas_pan = egui::Vec2::ZERO;
            }
        }
        if response.hovered() && self.drag_origin.is_none() {
            let factor = ui.input(|input| {
                if input.modifiers.ctrl {
                    input.zoom_delta()
                } else {
                    (input.smooth_scroll_delta.y / 200.).exp()
                }
            });
            if (factor - 1.).abs() > 0.0001 {
                let anchor = response.hover_pos().unwrap_or(rect.center());
                let world = (anchor - rect.center() - self.canvas_pan) / self.canvas_zoom;
                self.canvas_zoom = (self.canvas_zoom * factor).clamp(0.05, 32.);
                self.canvas_pan = anchor - rect.center() - world * self.canvas_zoom;
                ui.input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
            }
        }
        let origin = rect.center() + self.canvas_pan;
        let can_edit = self.saving.is_none() && self.settings.compose;
        if can_edit && (response.clicked() || response.drag_started()) {
            self.drag_origin = None;
            if let Some(pos) = response.interact_pointer_pos() {
                let start = if response.drag_started() {
                    ui.input(|input| input.pointer.press_origin())
                        .unwrap_or(pos)
                } else {
                    pos
                };
                let point = (start - origin) / self.canvas_zoom;
                self.selected = placements
                    .iter()
                    .rev()
                    .find(|p| {
                        point.x >= p.origin[0] as f32
                            && point.y >= p.origin[1] as f32
                            && point.x < (p.origin[0] as f32 + p.size[0] as f32)
                            && point.y < (p.origin[1] as f32 + p.size[1] as f32)
                    })
                    .map(|p| p.cell);
                if response.drag_started()
                    && ui.input(|i| i.pointer.primary_down() && !i.key_down(egui::Key::Space))
                {
                    self.drag_origin = self
                        .selected
                        .and_then(|cell| self.settings.offsets.get(cell).copied());
                    self.drag_scale = Some([1. / self.canvas_zoom; 2]);
                    self.drag_pointer_origin = Some(start);
                }
            }
        }
        if response.dragged() {
            if let (true, Some(cell), Some(start)) = (can_edit, self.selected, self.drag_origin) {
                let scale = self.drag_scale.unwrap_or([1. / self.canvas_zoom; 2]);
                let next = match self
                    .drag_pointer_origin
                    .zip(response.interact_pointer_pos())
                {
                    Some((press, current)) => dragged_offset(start, press, current, scale),
                    None => self.settings.offsets.get(cell).copied().unwrap_or(start),
                };
                if let Some(offset) = self.settings.offsets.get_mut(cell) {
                    if *offset != next {
                        *offset = next;
                        self.invalidate();
                    }
                }
            } else {
                self.canvas_pan += ui.input(|i| i.pointer.delta());
            }
            placements = self.live_placements();
            self.measured_gap = Some(measured_spacing(
                &placements,
                self.effective_gap.unwrap_or(self.settings.gap),
            ));
        }
        if response.drag_stopped() {
            self.drag_origin = None;
            self.drag_scale = None;
            self.drag_pointer_origin = None;
        }
        let painter = ui.painter().with_clip_rect(rect);
        checker_background(&painter, rect, rect.center() + self.canvas_pan, egui::Vec2::splat(self.canvas_zoom), &self.preview_background, self.preview_background_color);
        let origin = rect.center() + self.canvas_pan;
        let layout = if self.settings.compose {
            grid(&self.settings.layout, self.paths.len() + 1)
                .ok()
                .map(|rows| rows.into_iter().flatten().collect::<Vec<_>>())
                .unwrap_or_default()
        } else {
            vec![0]
        };
        for p in &placements {
            let Some(texture) = layout
                .get(p.cell)
                .and_then(|index| self.motif_textures.get(*index))
            else {
                continue;
            };
            let bounds = egui::Rect::from_min_size(
                origin + egui::vec2(p.origin[0] as f32, p.origin[1] as f32) * self.canvas_zoom,
                egui::vec2(p.size[0] as f32, p.size[1] as f32) * self.canvas_zoom,
            );
            painter.image(
                texture.id(),
                bounds,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                egui::Color32::WHITE,
            );
            if self.selected == Some(p.cell) {
                painter.rect_stroke(
                    bounds,
                    0.,
                    egui::Stroke::new(2.0_f32, egui::Color32::YELLOW),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
    fn reset_positions(&mut self) {
        let cells = grid(&self.settings.layout, self.paths.len() + 1)
            .map_or(0, |rows| rows.len() * rows[0].len());
        self.settings.offsets = vec![[0, 0]; cells];
        self.selected = None;
        self.drag_origin = None;
    }
    fn select_motif(&mut self, motif: usize) {
        self.selected = grid(&self.settings.layout, self.paths.len() + 1)
            .ok()
            .and_then(|rows| rows.into_iter().flatten().position(|v| v == motif));
    }
    fn selected_motif(&self) -> Option<usize> {
        let cell = self.selected?;
        grid(&self.settings.layout, self.paths.len() + 1)
            .ok()?
            .into_iter()
            .flatten()
            .nth(cell)
    }
    fn add_patterns(&mut self, source: &Path, paths: Vec<PathBuf>) {
        let old_count = self.paths.len() + 1;
        let mut skipped = false;
        for path in paths {
            if !super::is_bitmap(&path) || !path.is_file() {
                skipped = true;
                continue;
            }
            let same_file = |other: &Path| {
                let a = path.canonicalize().unwrap_or_else(|_| path.clone());
                let b = other.canonicalize().unwrap_or_else(|_| other.to_owned());
                if cfg!(windows) {
                    a.to_string_lossy()
                        .eq_ignore_ascii_case(&b.to_string_lossy())
                } else {
                    a == b
                }
            };
            if same_file(source) || self.paths.iter().any(|other| same_file(other)) {
                continue;
            }
            if self.paths.len() >= 15 {
                skipped = true;
                continue;
            }
            self.paths.push(path);
        }
        let count = self.paths.len() + 1;
        if count != old_count {
            let mut rows = grid(&self.settings.layout, old_count)
                .unwrap_or_else(|_| vec![(0..old_count).collect()]);
            for row in &mut rows {
                row.extend(old_count..count);
            }
            // A full-width custom layout falls back to one row containing all motifs.
            if rows[0].len() > 16 {
                rows = vec![(0..count).collect()];
            }
            self.settings.layout = format_grid(&rows);
            self.reset_positions();
            self.settings.compose = true;
            self.invalidate();
        }
        if skipped {
            self.message = tr("仅添加 PNG / JPG / JPEG / WebP / BMP / TGA 文件，含原图最多 16 个；其他项目已跳过。").into();
        }
    }
    fn remove_pattern(&mut self, index: usize) {
        let old_count = self.paths.len() + 1;
        self.paths.remove(index);
        self.selected = None;
        self.drag_origin = None;
        if let Ok(mut rows) = grid(&self.settings.layout, old_count) {
            let removed = index + 1;
            for row in &mut rows {
                for value in row {
                    if *value == removed {
                        *value = 0;
                    } else if *value > removed {
                        *value -= 1;
                    }
                }
            }
            self.settings.layout = format_grid(&rows);
        } else {
            self.settings.layout = format_grid(&[(0..self.paths.len() + 1).collect()]);
        }
        self.reset_positions();
        self.invalidate();
    }
    pub fn active(&self) -> bool {
        self.open || self.pending.is_some() || self.saving.is_some()
    }
    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty_since.get_or_insert_with(std::time::Instant::now);
        self.message.clear();
    }
    fn start_preview(&mut self, ctx: &egui::Context, source: &Arc<RgbaImage>) {
        if self.pending.is_some() || self.saving.is_some() {
            return;
        }
        let Some(since) = self.dirty_since else {
            return;
        };
        if since.elapsed() < std::time::Duration::from_millis(60) {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }
        self.dirty_since = None;
        let (tx, rx) = mpsc::channel();
        self.pending = Some((self.revision, rx));
        let source = source.clone();
        let paths = self.paths.clone();
        let settings = self.settings.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(generate(&source, &paths, &settings));
            ctx.request_repaint();
        });
    }
    pub fn show(&mut self, ctx: &egui::Context, path: &Path, source: &Arc<RgbaImage>) {
        if !self.active() {
            self.clear_editor_view();
            self.output = None;
            self.texture = None;
            self.repeat_output = None;
            self.repeat_texture = None;
            self.placements.clear();
            self.dirty_since = None;
            self.source = None;
            return;
        }
        if self
            .source
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, source))
        {
            self.invalidate();
            self.pending = None;
            self.output = None;
            self.texture = None;
            self.repeat_output = None;
            self.repeat_texture = None;
            self.placements.clear();
            self.source = Some(source.clone());
            self.clear_editor_view();
        }
        if let Some((revision, result)) =
            self.pending
                .as_ref()
                .and_then(|(revision, r)| match r.try_recv() {
                    Ok(v) => Some((*revision, v)),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        Some((*revision, Err(tr("图片处理任务中断，请重试。").into())))
                    }
                    Err(_) => None,
                })
        {
            self.pending = None;
            match result {
                Ok(preview) => {
                    let scene = preview.scene;
                    if scene.original_fallback && revision == self.revision {
                        self.message = tr("未找到更小循环，使用原图预览和平铺。").into();
                    }
                    if self.edit_placements.is_empty() && !scene.edit_placements.is_empty() {
                        self.canvas_zoom = 0.;
                    }
                    self.edit_placements = scene.edit_placements;
                    self.snapshot_offsets = scene.offsets;
                    self.measured_gap = scene.effective_gap;
                    self.motif_textures = scene
                        .motifs
                        .iter()
                        .enumerate()
                        .map(|(index, image)| {
                            ctx.load_texture(
                                format!("repeat-motif-{}-{index}", path.display()),
                                egui::ColorImage {
                                    size: [image.width() as usize, image.height() as usize],
                                    pixels: image
                                        .pixels()
                                        .map(|p| super::preview_color(p.0))
                                        .collect(),
                                },
                                egui::TextureOptions::NEAREST,
                            )
                        })
                        .collect();
                    let repeat_preview = preview_pixels(&preview.repeat, 2048);
                    self.repeat_texture = Some(
                        ctx.load_texture(
                            format!("seamless-repeat:{}", path.display()),
                            egui::ColorImage {
                                size: [
                                    repeat_preview.width() as usize,
                                    repeat_preview.height() as usize,
                                ],
                                pixels: repeat_preview
                                    .pixels()
                                    .map(|p| super::preview_color(p.0))
                                    .collect(),
                            },
                            egui::TextureOptions {
                                wrap_mode: egui::TextureWrapMode::Repeat,
                                ..egui::TextureOptions::NEAREST
                            },
                        ),
                    );
                    self.repeat_output = Some(Arc::new(preview.repeat));
                    let image = scene.image;
                    self.placements = scene.placements;
                    self.effective_gap = scene.effective_gap;
                    self.displayed_revision = Some(revision);
                    let preview = preview_pixels(&image, 2048);
                    let display = egui::ColorImage {
                        size: [preview.width() as usize, preview.height() as usize],
                        pixels: preview
                            .pixels()
                            .map(|p| super::preview_color(p.0))
                            .collect(),
                    };
                    self.texture = Some(ctx.load_texture(
                        format!("seamless:{}", path.display()),
                        display,
                        egui::TextureOptions {
                            wrap_mode: egui::TextureWrapMode::Repeat,
                            ..egui::TextureOptions::NEAREST
                        },
                    ));
                    self.output = Some(Arc::new(image));
                }
                Err(e) => {
                    if revision == self.revision {
                        self.message = e;
                        self.output = None;
                        self.texture = None;
                        self.repeat_output = None;
                        self.repeat_texture = None;
                        self.placements.clear();
                    }
                }
            }
        }
        if let Some(result) = self.saving.as_ref().and_then(|r| match r.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(tr("图片处理任务中断，请重试。").into()))
            }
            Err(_) => None,
        }) {
            self.saving = None;
            self.message = match result {
                Ok(path) => format!("{}: {}", tr("已保存"), path.display()),
                Err(e) => e,
            };
        }
        let mut open = self.open;
        crate::native_window::Window::new(format!("{} · {}",tr("四方连续图"),path.file_name().unwrap_or_default().to_string_lossy()))
            .id(egui::Id::new(("seamless",path))).open(&mut open).default_width(1100.).default_height(850.).min_width(820.).scrollable(false).show(ctx,|ui| {
            let canvas_height=(ui.available_height()*0.60).clamp(200.,600.);
            let mut canvases=Vec::new();
            ui.columns(3,|panes| {for (index,pane) in panes.iter_mut().enumerate() {
                let (rect,_)=pane.allocate_exact_size(egui::vec2(pane.available_width(),canvas_height),egui::Sense::hover());
                let mut canvas=pane.new_child(egui::UiBuilder::new().id_salt(("repeat_canvas",index)).max_rect(rect));
                canvas.set_clip_rect(pane.clip_rect().intersect(rect));
                canvases.push(canvas);
            }});
            let busy=self.saving.is_some();
            egui::ScrollArea::vertical().id_salt("repeat_parameters").max_height(ui.available_height().max(80.)).auto_shrink([false,false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).show(ui,|ui| {
            ui.style_mut().spacing.scroll=egui::style::ScrollStyle::solid();
            // Read the editor viewport's input, not the main window's input.
            let owns_drop=ui.ctx().viewport_id()!=egui::ViewportId::ROOT || ui.input(|input|
                input.pointer.latest_pos().is_some_and(|p|ui.clip_rect().contains(p)));
            if owns_drop {
                let files=ui.input_mut(|input|std::mem::take(&mut input.raw.dropped_files));
                if !files.is_empty() {
                    if busy {self.message=tr("处理中，请完成后再拖入图案。").into();}
                    else {
                        let missing=files.iter().any(|file|file.path.is_none());
                        self.add_patterns(path,files.into_iter().filter_map(|file|file.path).collect());
                        if missing {self.message=tr("仅支持从文件管理器拖入本地图片文件。").into();}
                    }
                }
            }
            let hint=tr("可将图片拖入此窗口，自动加入图案列表和排列；列表中可逐项移除。");
            if owns_drop && ui.input(|input|!input.raw.hovered_files.is_empty()) {ui.colored_label(egui::Color32::from_rgb(80,160,240),hint);}
            else {ui.label(hint);}
            ui.add_enabled_ui(!busy,|ui| {
                let mut changed=false;
                ui.horizontal_wrapped(|ui| {
                    let mut mode=if self.settings.compose {"compose"} else {"extract"}.to_owned();
                    if crate::theme::compact_selector(ui,&mut mode,[("extract","提取最小循环"),("compose","制作连续图")]) {self.settings.compose=mode=="compose";changed=true;}
                ui.label(tr("背景色"));
                self.preview_background.controls(ui,&mut self.preview_background_color);

                });
                if self.settings.compose {
                    ui.horizontal_wrapped(|ui| {
                        if ui.checkbox(&mut self.settings.custom_size,tr("自定义循环尺寸")).changed() {
                            if self.settings.custom_size {if let Some(image)=&self.repeat_output {self.settings.repeat_size=[image.width(),image.height()];}}
                            changed=true;
                        }
                        if self.settings.custom_size {for (axis,label) in ["循环宽度","循环高度"].into_iter().enumerate() {
                            ui.label(tr(label));
                            changed|=ui.add(egui::DragValue::new(&mut self.settings.repeat_size[axis]).range(4..=8192).speed(4).suffix(" px")).changed();
                            self.settings.repeat_size[axis]=self.settings.repeat_size[axis].div_ceil(4)*4;
                        }}
                    });
                    if self.settings.custom_size {ui.small(tr("固定左上角裁剪循环区域，扩大时补背景；宽高向上对齐 4，不缩放图案，超出区域的部分不会导出。"));}
                    ui.horizontal_wrapped(|ui| {
                        let mut arrangement=match self.settings.stagger {Stagger::None=>"straight",Stagger::Rows=>"rows",Stagger::Columns=>"columns"}.to_owned();
                        if crate::theme::compact_selector(ui,&mut arrangement,[("straight","普通排列"),("rows","逐行横向错位"),("columns","逐列纵向错位")]) {self.settings.stagger=match arrangement.as_str() {"rows"=>Stagger::Rows,"columns"=>Stagger::Columns,_=>Stagger::None};changed=true;}
                        if self.settings.stagger!=Stagger::None {for q in [2,3,4] {changed|=ui.selectable_value(&mut self.settings.denominator,q,format!("1/{q}")).changed();}}
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr("图案缩放"));changed|=ui.add(egui::DragValue::new(&mut self.settings.scale).range(1..=400).suffix("%")).changed();
                        for (i,label) in ["横向间距","纵向间距"].into_iter().enumerate() {ui.label(tr(label));
                            let mut value=self.measured_gap.unwrap_or(self.settings.gap)[i];
                            if ui.add(egui::DragValue::new(&mut value).range(0..=8192).suffix(" px")).changed() {
                                self.settings.gap[i]=value;for offset in &mut self.settings.offsets {offset[i]=0;}
                                self.measured_gap=None;self.drag_origin=None;self.drag_scale=None;changed=true;
                            }
                        }
                        changed|=ui.checkbox(&mut self.settings.trim,tr("裁去透明边缘")).changed();
                    });
                    ui.horizontal_wrapped(|ui| {
                        changed|=ui.checkbox(&mut self.settings.transparent,tr("透明背景")).changed();
                        if !self.settings.transparent {changed|=ui.color_edit_button_srgb(&mut self.settings.background).changed();}
                        if ui.add_enabled(self.paths.len()<15,egui::Button::new(tr("添加图案"))).clicked() {
                            if let Some(paths)=rfd::FileDialog::new().add_filter(tr("图片"),&["png","jpg","jpeg","webp","bmp","tga"]).pick_files() {
                                self.add_patterns(path,paths);
                            }
                        }
                    });
                    let labels=std::iter::once(format!("1: {}",path.file_name().unwrap_or_default().to_string_lossy()))
                        .chain(self.paths.iter().enumerate().map(|(i,path)|format!("{}: {}",i+2,path.file_name().unwrap_or_default().to_string_lossy()))).collect::<Vec<_>>();
                    let font=egui::TextStyle::Button.resolve(ui.style());
                    let text_width=|text:&str|ui.painter().layout_no_wrap(text.to_owned(),font.clone(),ui.visuals().text_color()).size().x;
                    let longest=labels.iter().map(|text|text_width(text)).fold(0_f32,f32::max)+ui.spacing().button_padding.x*2.;
                    let remove_width=text_width(tr("移除"))+ui.spacing().button_padding.x*2.+ui.spacing().item_spacing.x;
                    let width=longest.min((ui.available_width()-remove_width).max(1.));
                    let row_button=|ui:&mut egui::Ui,label:&str,selected:bool|motif_list_button(ui,label,width,selected);
                    if row_button(ui,&labels[0],self.selected_motif()==Some(0)).on_hover_text(path.display().to_string()).clicked() {self.select_motif(0);}
                    let mut remove=None;
                    let mut select=None;
                    for (i,path) in self.paths.iter().enumerate() {ui.push_id(i,|ui| {ui.horizontal(|ui| {
                        if row_button(ui,&labels[i+1],self.selected_motif()==Some(i+1)).on_hover_text(path.display().to_string()).clicked() {select=Some(i+1);}
                        if ui.small_button(tr("移除")).on_hover_text(tr("移除后，对应位置改用原图，其余编号自动更新。")).clicked() {remove=Some(i);}
                    });});}
                    if let Some(i)=remove {self.remove_pattern(i);}
                    if let Some(index)=select {self.select_motif(index);}
                    if let Some(index)=self.selected.filter(|i|*i<self.settings.offsets.len()) {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{} {}",tr("选中位置"),index+1));
                            for axis in 0..2 {ui.label(if axis==0 {"X"} else {"Y"});changed|=ui.add(egui::DragValue::new(&mut self.settings.offsets[index][axis]).range(-32768..=32768).suffix(" px")).changed();}
                            if ui.button(tr("重置位置")).clicked() {self.settings.offsets[index]=[0,0];changed=true;}
                        });
                    }
                    ui.label(tr("排列：逗号分列，分号分行，例如 1,2;2,1。编号对应上方图案。"));
                    ui.horizontal_wrapped(|ui| {
                        for (label,layout) in [("单图案","1"),("交替排列","1,2"),("棋盘排列","1,2;2,1")] {
                            if ui.add_enabled(layout=="1"||!self.paths.is_empty(),egui::Button::new(tr(label))).clicked() {self.settings.layout=layout.into();self.reset_positions();changed=true;}
                        }
                        if ui.button(tr("单图双排错位")).clicked() {self.settings.layout="1;1".into();self.settings.stagger=Stagger::Rows;self.settings.denominator=2;self.reset_positions();changed=true;}
                    });
                    if ui.text_edit_singleline(&mut self.settings.layout).changed() {self.reset_positions();changed=true;}
                    ui.small(tr("图案宽高透明补齐至 4 的倍数；格距按 4 和错位分母的公倍数自动补齐，不拉伸图案。"));
                    if let Some(gap)=self.effective_gap {ui.label(format!("{}: {} × {} px",tr("实际格子间距"),gap[0],gap[1]));}
                    ui.small(tr("修改排列或增删图案会重置位置；可选中每个排列位置单独移动。"));
                } else {ui.small(tr("完整 RGBA 精确比较，提取能整除原图尺寸的最小矩形循环，保留错位和图案顺序；不修补接缝。"));}
                if changed {self.invalidate();}

            });
            if busy {ui.spinner();ctx.request_repaint_after(std::time::Duration::from_millis(100));}
            if self.pending.is_some()||self.dirty_since.is_some() {ui.label(tr("预览更新中，仍可继续编辑。"));ctx.request_repaint_after(std::time::Duration::from_millis(60));}
            ui.label(tr("左侧为固定编辑视口：滚轮缩放，拖动空白处或按住空格拖动可平移；仅显示编辑图案，不显示循环副本。"));
            ui.small(tr("间距显示相邻图案边界距离；该方向无相邻图案时显示循环留白，重叠显示 0。移动后自动更新。"));
            if !self.message.is_empty() {ui.label(&self.message);}
            ui.small(tr("导出 PNG，保存对话框确认后可覆盖已有文件，不修改 Unity meta。预览最多 2048 像素，导出保留实际尺寸。"));
            ui.separator();
            });
            if let (Some(image),Some(texture))=(self.output.clone(),self.texture.clone()) {
                    for (i,ui) in canvases.iter_mut().enumerate() {
                        self.preview_header(ui,path,i);
                        if i==0 {self.show_editor_canvas(ui);continue;}
                        let (image,texture)=if i==0 {(&image,&texture)} else {(self.repeat_output.as_ref().unwrap(),self.repeat_texture.as_ref().unwrap())};
                        let (w,h)=image.dimensions();
                        let size=egui::vec2(w as f32,h as f32);
                        let max_height=ui.available_height().clamp(1.,600.);
                        let size=size*(ui.available_width()/size.x).min(max_height/size.y);
                        let (rect,_)=ui.allocate_exact_size(size,egui::Sense::hover());
                        let painter=ui.painter().with_clip_rect(rect);
                        let repeat=if i==2 {3.} else {1.};
                        checker_background(&painter,rect,rect.min,egui::vec2(rect.width()/(w as f32*repeat),rect.height()/(h as f32*repeat)),&self.preview_background,self.preview_background_color);
                        painter.image(texture.id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(repeat,repeat)),egui::Color32::WHITE);
                        if self.borders {for n in 0..=repeat as u32 {
                            let t=n as f32/repeat;let stroke=egui::Stroke::new(1.0_f32,egui::Color32::from_rgb(80,180,255));
                            painter.line_segment([rect.min+egui::vec2(rect.width()*t,0.),rect.min+egui::vec2(rect.width()*t,rect.height())],stroke);
                            painter.line_segment([rect.min+egui::vec2(0.,rect.height()*t),rect.min+egui::vec2(rect.width(),rect.height()*t)],stroke);
                        }}
                    }
            } else {for (index,canvas) in canvases.iter_mut().enumerate() {self.preview_header(canvas,path,index);if index==0 {self.show_editor_canvas(canvas);}}}
        });
        self.open = open;
        if self.open {
            self.start_preview(ctx, source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_repeat_dimensions_crop_or_pad_without_scaling() {
        let image = RgbaImage::from_fn(12, 8, |x, y| Rgba([x as u8, y as u8, 20, 255]));
        let mut settings = Settings {
            custom_size: true,
            repeat_size: [5, 4],
            ..Default::default()
        };
        let tile = output_tile(&image, &settings).unwrap();
        assert_eq!(tile.dimensions(), (8, 4));
        assert_eq!(tile.get_pixel(7, 3), image.get_pixel(7, 3));
        settings.repeat_size = [16, 12];
        let padded = output_tile(&image, &settings).unwrap();
        assert_eq!(padded.dimensions(), (16, 12));
        assert_eq!(padded.get_pixel(11, 7), image.get_pixel(11, 7));
        assert_eq!(padded.get_pixel(12, 8), &Rgba([0, 0, 0, 0]));
        settings.transparent = false;
        settings.background = [11, 22, 33];
        assert_eq!(
            output_tile(&image, &settings).unwrap().get_pixel(15, 11),
            &Rgba([11, 22, 33, 255])
        );
        settings.repeat_size = [8192, 8192];
        assert!(output_tile(&image, &settings).is_err());
    }
    #[test]
    fn continuous_drag_uses_total_displacement_at_fixed_zoom() {
        let start = [10, -5];
        let press = egui::pos2(100., 200.);
        let scale = [0.5, 0.5];
        assert_eq!(
            dragged_offset(start, press, egui::pos2(104., 202.), scale),
            [12, -4]
        );
        assert_eq!(
            dragged_offset(start, press, egui::pos2(140., 220.), scale),
            [30, 5]
        );
        assert_eq!(
            dragged_offset(start, press, egui::pos2(180., 240.), scale),
            [50, 15]
        );
        assert_eq!(
            dragged_offset(start, press, egui::pos2(180., 240.), scale),
            [50, 15]
        );
        assert_eq!(dragged_offset(start, press, press, scale), start);
        assert_eq!(
            dragged_offset(start, press, egui::pos2(80., 160.), scale),
            [0, -25]
        );
    }
    #[test]
    fn editor_has_no_automatic_repeat_copies_and_keeps_world_coordinates() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]));
        let settings = Settings {
            stagger: Stagger::Rows,
            denominator: 3,
            offsets: vec![[-5, 6]],
            ..Default::default()
        };
        let scene = compose_scene(&[image], &settings).unwrap();
        assert_eq!(scene.placements.len(), 3);
        assert_eq!(scene.edit_placements.len(), 1);
        assert_eq!(scene.edit_placements[0].origin, [-1, 6]);
        let mut editor = Editor::default();
        editor.edit_placements = scene.edit_placements;
        editor.snapshot_offsets = settings.offsets.clone();
        editor.settings = settings;
        editor.settings.offsets[0] = [-9, 7];
        assert_eq!(editor.live_placements()[0].origin, [-5, 7]);
    }
    #[test]
    fn measured_spacing_follows_element_movement_without_grid_feedback() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]));
        let mut settings = Settings {
            layout: "1,1".into(),
            gap: [4, 4],
            offsets: vec![[0, 0], [0, 0]],
            ..Default::default()
        };
        let before = compose_scene(&[image.clone()], &settings).unwrap();
        assert_eq!(before.effective_gap.unwrap()[0], 4);
        settings.offsets[1] = [5, 0];
        let after = compose_scene(&[image], &settings).unwrap();
        assert_eq!(after.effective_gap.unwrap()[0], 9);
        assert_eq!(settings.gap, [4, 4]);
        assert_eq!(
            after.edit_placements[0].origin,
            before.edit_placements[0].origin
        );
        assert_eq!(measured_spacing(&after.edit_placements, [0, 0])[0], 9);
    }
    #[test]
    fn canvas_expands_in_all_directions_without_losing_moved_pixels() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([17, 33, 55, 255]));
        for (offset, expected) in [([5, 6], [5, 6]), ([-5, -6], [3, 2])] {
            let settings = Settings {
                offsets: vec![offset],
                ..Default::default()
            };
            let scene = compose_scene(&[image.clone()], &settings).unwrap();
            assert_eq!(scene.image.dimensions(), (12, 12));
            assert_eq!(scene.placements[0].origin, expected);
            assert_eq!(scene.image.pixels().filter(|p| p[3] != 0).count(), 16);
            assert_eq!(
                scene
                    .image
                    .get_pixel(expected[0] as u32, expected[1] as u32),
                image.get_pixel(0, 0)
            );
        }
        let settings = Settings {
            offsets: vec![[8192, 0]],
            ..Default::default()
        };
        assert!(compose_scene(&[image.clone()], &settings).is_err());
        assert_eq!(
            compose_scene(&[image], &Settings::default())
                .unwrap()
                .image
                .dimensions(),
            (4, 4)
        );
    }
    #[test]
    fn preview_keeps_edit_canvas_and_actual_export_tile_distinct() {
        let source = RgbaImage::from_pixel(1, 1, Rgba([25, 50, 75, 255]));
        let settings = Settings {
            layout: "1,1".into(),
            ..Default::default()
        };
        let preview = generate(&source, &[], &settings).unwrap();
        assert_eq!(preview.scene.image.dimensions(), (8, 4));
        assert_eq!(preview.repeat.dimensions(), (4, 4));
        assert_eq!(
            preview.repeat,
            aligned_repeat(&preview.scene.image).unwrap()
        );
        assert_eq!(preview.scene.placements.len(), 2);
    }
    #[test]
    fn four_alignment_repeats_whole_cycles_without_seam_padding() {
        let image = RgbaImage::from_fn(6, 10, |x, y| Rgba([(x % 3) as u8, (y % 5) as u8, 23, 0]));
        let aligned = aligned_repeat(&image).unwrap();
        assert_eq!(aligned.dimensions(), (12, 20));
        for (x, y, pixel) in aligned.enumerate_pixels() {
            assert_eq!(*pixel, *image.get_pixel(x % 3, y % 5));
        }
        assert_eq!(
            aligned_repeat(&RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255])))
                .unwrap()
                .dimensions(),
            (4, 4)
        );
        let source = RgbaImage::from_pixel(5, 7, Rgba([12, 34, 56, 255]));
        let padded = motif(&source, &Settings::default()).unwrap();
        assert_eq!(padded.dimensions(), (8, 8));
        assert_eq!(padded.get_pixel(1, 0), source.get_pixel(0, 0));
        assert_eq!(padded.get_pixel(0, 0)[3], 0);
        let settings = Settings {
            gap: [1, 2],
            stagger: Stagger::Rows,
            denominator: 3,
            ..Default::default()
        };
        let scene = compose_scene(&[padded], &settings).unwrap();
        assert_eq!(scene.image.dimensions(), (12, 36));
        assert_eq!(scene.effective_gap, Some([4, 4]));
    }
    #[test]
    fn single_motif_second_row_moves_independently_and_wraps() {
        let motif = RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 255]));
        let mut settings = Settings {
            layout: "1;1".into(),
            stagger: Stagger::Rows,
            gap: [2, 2],
            offsets: vec![[0, 0], [0, 0]],
            ..Default::default()
        };
        let before = compose_scene(&[motif.clone()], &settings).unwrap();
        assert_eq!(before.image.dimensions(), (4, 8));
        assert_eq!(before.placements[0].origin, [1, 1]);
        assert_eq!(before.placements[1].origin, [3, 5]);
        assert_eq!(pick_motif(&before.placements, [0., 5.], [4, 8]), Some(1));
        assert_eq!(pick_motif(&before.placements, [4., 13.], [4, 8]), Some(1));
        settings.offsets[1] = [-5, 2];
        let after = compose_scene(&[motif], &settings).unwrap();
        assert_eq!(after.image.dimensions(), (8, 12));
        assert_eq!(after.placements[0].origin, [5, 1]);
        assert_eq!(after.placements[1].origin, [2, 7]);
        assert_eq!(after.image.get_pixel(2, 8)[3], 255);
        assert_eq!(after.image.get_pixel(5, 1), before.image.get_pixel(1, 1));
        assert_eq!(minimum(&after.image).dimensions(), (8, 12));
    }
    #[test]
    fn live_changes_keep_preview_and_queue_latest_revision() {
        let mut editor = Editor::default();
        editor.output = Some(Arc::new(RgbaImage::new(2, 2)));
        editor.displayed_revision = Some(editor.revision);
        editor.invalidate();
        let since = editor.dirty_since.unwrap();
        let revision = editor.revision;
        editor.invalidate();
        assert_eq!(editor.dirty_since, Some(since));
        assert_eq!(editor.revision, revision + 1);
        assert_ne!(editor.displayed_revision, Some(editor.revision));
        assert!(editor.output.is_some());
        editor.settings.layout = "1;1".into();
        editor.reset_positions();
        assert_eq!(editor.settings.offsets, vec![[0, 0], [0, 0]]);
        editor.selected = Some(1);
        assert_eq!(editor.selected_motif(), Some(0));
    }
    #[test]
    fn adding_patterns_filters_duplicates_and_extends_layout() {
        let root = std::env::temp_dir().join(format!(
            "seamless-drop-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let files: Vec<_> = (0..18)
            .map(|i| {
                let path = root.join(format!("{i}.PNG"));
                RgbaImage::new(1, 1).save(&path).unwrap();
                path
            })
            .collect();
        let mut editor = Editor::default();
        editor.add_patterns(
            &files[0],
            vec![
                files[0].clone(),
                files[1].clone(),
                files[1].clone(),
                root.clone(),
                root.join("missing.png"),
            ],
        );
        assert_eq!(editor.paths, vec![files[1].clone()]);
        assert!(editor.settings.compose);
        assert_eq!(editor.settings.layout, "1,2");
        editor.settings.layout = "1,2;2,1".into();
        editor.add_patterns(&files[0], vec![files[2].clone()]);
        assert_eq!(editor.settings.layout, "1,2,3;2,1,3");
        editor.add_patterns(&files[0], files[3..].to_vec());
        assert_eq!(editor.paths.len(), 15);
        assert!(grid(&editor.settings.layout, 16).is_ok());
        assert!(!editor.message.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn removal_preserves_other_motif_references_and_invalidates_result() {
        let mut editor = Editor::default();
        editor.paths = vec!["a.png".into(), "b.png".into(), "c.png".into()];
        editor.settings.layout = "1,2;3,4".into();
        editor.output = Some(Arc::new(RgbaImage::new(1, 1)));
        editor.remove_pattern(0);
        assert_eq!(editor.settings.layout, "1,1;2,3");
        assert!(editor.output.is_some());
        assert_ne!(editor.displayed_revision, Some(editor.revision));
        assert!(editor.dirty_since.is_some());
        assert_eq!(
            editor.paths,
            vec![PathBuf::from("b.png"), PathBuf::from("c.png")]
        );
        editor.settings.layout = "invalid".into();
        editor.remove_pattern(0);
        assert_eq!(editor.settings.layout, "1,2");
        editor.remove_pattern(0);
        assert_eq!(editor.settings.layout, "1,1");
    }
    #[test]
    fn exact_period_includes_hidden_rgb_and_requires_full_cycles() {
        let input = RgbaImage::from_fn(12, 8, |x, y| Rgba([(x % 3) as u8, (y % 2) as u8, 44, 0]));
        assert_eq!(minimum(&input).dimensions(), (3, 2));
        let partial = RgbaImage::from_fn(7, 1, |x, _| Rgba([(x % 3) as u8, 0, 0, 255]));
        assert_eq!(minimum(&partial).dimensions(), (7, 1));
        let single = RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 4]));
        assert_eq!(minimum(&single), single);
    }
    #[test]
    fn stagger_wraps_edges_and_keeps_pattern_cycle() {
        let red = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
        let blue = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255]));
        let mut settings = Settings {
            compose: true,
            stagger: Stagger::Rows,
            gap: [2, 2],
            layout: "1,2;2,1;1,1".into(),
            ..Default::default()
        };
        let out = compose(&[red.clone(), blue.clone()], &settings).unwrap();
        assert_eq!(out.dimensions(), (8, 24));
        assert_eq!(out.get_pixel(0, 5), &Rgba([255, 0, 0, 255]));
        settings.stagger = Stagger::Columns;
        settings.layout = "1,2,1;2,1,1".into();
        assert_eq!(
            compose(&[red, blue], &settings).unwrap().dimensions(),
            (24, 8)
        );
    }
    #[test]
    fn layout_trimming_spacing_and_limits() {
        assert!(grid("1,2;1", 2).is_err());
        assert!(grid("0", 2).is_err());
        assert!(grid("3", 2).is_err());
        let image = RgbaImage::from_fn(5, 5, |x, y| {
            if x == 2 && y == 2 {
                Rgba([1, 2, 3, 255])
            } else {
                Rgba([0; 4])
            }
        });
        let settings = Settings {
            compose: true,
            gap: [3, 4],
            ..Default::default()
        };
        let trimmed = motif(&image, &settings).unwrap();
        assert_eq!(trimmed.dimensions(), (4, 4));
        assert_eq!(trimmed.get_pixel(1, 1), &Rgba([1, 2, 3, 255]));
        assert_eq!(compose(&[trimmed], &settings).unwrap().dimensions(), (8, 8));
        assert!(checked_size(8192, 8192).is_err());
        assert!(checked_size(0, 1).is_err());
    }
    #[test]
    fn thirds_and_quarters_close_in_both_axes() {
        let dot = RgbaImage::from_pixel(1, 1, Rgba([20, 30, 40, 255]));
        for q in [3, 4] {
            let mut settings = Settings {
                compose: true,
                stagger: Stagger::Rows,
                denominator: q,
                gap: [2, 2],
                ..Default::default()
            };
            let out = compose(&[dot.clone()], &settings).unwrap();
            let pitch = if q == 3 { 12 } else { 4 };
            assert_eq!(out.dimensions(), (pitch, 4 * q));
            for row in 0..q {
                assert_eq!(
                    out.get_pixel(((pitch - 1) / 2 + row * pitch / q) % pitch, row * 4 + 1)[3],
                    255
                );
            }
            settings.stagger = Stagger::Columns;
            assert_eq!(
                compose(&[dot.clone()], &settings).unwrap().dimensions(),
                (4 * q, pitch)
            );
        }
    }
    #[test]
    fn extraction_falls_back_to_exact_original_and_preserves_source() {
        let input = RgbaImage::from_fn(4, 3, |x, y| Rgba([x as u8, y as u8, 15, 255]));
        let original = input.clone();
        let preview = generate(
            &input,
            &[],
            &Settings {
                compose: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(preview.scene.original_fallback);
        assert_eq!(preview.scene.image, input);
        assert_eq!(preview.repeat, input);
        assert_eq!(preview.repeat.dimensions(), (4, 3));
        assert_eq!(input, original);
        let transparent = RgbaImage::new(2, 2);
        assert!(motif(&transparent, &Settings::default()).is_err());
    }
}

#[cfg(test)]
mod preview_pixel_tests {
    use super::*;
    #[test]
    fn small_non_square_preview_preserves_all_pixels() {
        let image = RgbaImage::from_fn(14, 20, |x,y| Rgba([x as u8,y as u8,255,(x*y) as u8]));
        assert_eq!(preview_pixels(&image, 1024), image);
    }
    #[test]
    fn oversized_preview_keeps_aspect_and_does_not_interpolate() {
        let image = RgbaImage::from_fn(32, 16, |x,y| if (x+y)%2==0 {Rgba([255,255,255,255])} else {Rgba([0,0,0,0])});
        let preview = preview_pixels(&image, 8);
        assert_eq!(preview.dimensions(), (8,4));
        assert!(preview.pixels().all(|p| *p==Rgba([255,255,255,255]) || *p==Rgba([0,0,0,0])));
    }
}
