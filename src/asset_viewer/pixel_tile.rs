//! Single-layer Tile sets, lattice-mapped painting and live raised/flat repeats.
use crate::localization::tr;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use image::{Rgba, RgbaImage};
use std::path::Path;

const MAX_SIDE: u32 = 1024;
const HISTORY_BYTES: usize = 32 * 1024 * 1024;
const MAX_TILES: usize = 64;
const LAYER_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, PartialEq)]
struct Layer {
    name: String,
    visible: bool,
    locked: bool,
    tiles: Vec<RgbaImage>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredLayer {
    name: String,
    visible: bool,
    locked: bool,
    tiles: Vec<Vec<u8>>,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tool {
    #[default]
    Brush,
    Eraser,
    Eyedropper,
    Rectangle,
    Line,
    Fill,
}
#[derive(Clone, Copy)]
struct RectangleDrag {
    start: (i32, i32),
    end: (i32, i32),
    layout: bool,
    preview: bool,
    line: bool,
}
fn default_background() -> [u8; 3] {
    [85, 85, 85]
}
fn enabled() -> bool {
    true
}
fn one() -> u32 {
    1
}
fn hex_color(text: &str, alpha: u8) -> Option<[u8; 4]> {
    let text = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if !matches!(text.len(), 6 | 8) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut rgba = [0, 0, 0, alpha];
    for (i, channel) in rgba.iter_mut().take(text.len() / 2).enumerate() {
        *channel = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(rgba)
}

#[derive(Clone, PartialEq)]
struct Snapshot {
    layers: Vec<Layer>,
    active_layer: usize,
    tiles: Vec<RgbaImage>,
    selected: usize,
    layout: Vec<usize>,
    columns: usize,
    rows: usize,
    multiple: bool,
    diamond: bool,
    elevated: bool,
    footprint: [u32; 2],
    wrap: bool,
}
impl Snapshot {
    fn bytes(&self) -> usize {
        self.tiles.iter().map(|p| p.as_raw().len()).sum::<usize>()
            + self
                .layers
                .iter()
                .flat_map(|l| &l.tiles)
                .map(|p| p.as_raw().len())
                .sum::<usize>()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Project {
    #[serde(default)]
    background_custom: bool,
    #[serde(default = "default_background")]
    background_color: [u8; 3],
    #[serde(default)]
    layers: Vec<StoredLayer>,
    #[serde(default)]
    active_layer: usize,
    version: u8,
    width: u32,
    height: u32,
    tiles: Vec<Vec<u8>>,
    selected: usize,
    layout: Vec<usize>,
    columns: usize,
    rows: usize,
    multiple: bool,
    diamond: bool,
    elevated: bool,
    footprint: [u32; 2],
    #[serde(default = "enabled")]
    wrap: bool,
    #[serde(default = "enabled")]
    edit_unit: bool,
    #[serde(default = "one")]
    brush_size: u32,
}
fn same_file(a: &Path, b: &Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.to_owned());
    let b = b.canonicalize().unwrap_or_else(|_| b.to_owned());
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
impl Project {
    fn validate(&self) -> Result<(), String> {
        let valid = (1..=3).contains(&self.version)
            && (2..=MAX_SIDE).contains(&self.width)
            && (2..=MAX_SIDE).contains(&self.height)
            && self.width % 2 == 0
            && self.height % 2 == 0
            && (1..=MAX_TILES).contains(&self.tiles.len())
            && (1..=32).contains(&self.brush_size)
            && self.selected < self.tiles.len()
            && (1..=8).contains(&self.columns)
            && (1..=8).contains(&self.rows)
            && self.layout.len() == 64
            && self.layout.iter().all(|i| *i < self.tiles.len())
            && self
                .footprint
                .iter()
                .all(|v| (2..=MAX_SIDE).contains(v) && v % 2 == 0)
            && self
                .tiles
                .iter()
                .all(|t| t.len() == self.width as usize * self.height as usize * 4)
            && self.tiles.iter().map(Vec::len).sum::<usize>() <= 16 * 1024 * 1024;
        let layer_valid = self.layers.len() <= 32
            && (self.layers.is_empty() || self.active_layer < self.layers.len())
            && self.layers.iter().all(|l| {
                l.tiles.len() == self.tiles.len()
                    && l.name.len() <= 256
                    && l.tiles
                        .iter()
                        .all(|t| t.len() == self.width as usize * self.height as usize * 4)
            })
            && self
                .layers
                .iter()
                .flat_map(|l| &l.tiles)
                .map(Vec::len)
                .sum::<usize>()
                <= LAYER_BYTES;
        if valid && layer_valid {
            Ok(())
        } else {
            Err(tr("无效的 Tile 工程或超出尺寸限制。").into())
        }
    }
}

pub(super) struct Editor {
    background_custom: bool,
    background_color: [u8; 3],
    layers: Vec<Layer>,
    active_layer: usize,
    layer_thumbnails: Vec<egui::TextureHandle>,
    pub open: bool,
    pixels: RgbaImage,
    tiles: Vec<RgbaImage>,
    selected: usize,
    layout: Vec<usize>,
    selected_slot: usize,
    columns: usize,
    rows: usize,
    multiple: bool,
    textures: Vec<egui::TextureHandle>,
    preview_last: Option<(i32, i32)>,
    dirty_texture: bool,
    dirty_all: bool,
    dirty_tiles: std::collections::BTreeSet<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    before: Option<Snapshot>,
    last: Option<(i32, i32)>,
    color: [u8; 4],
    tool: Tool,
    rectangle: Option<RectangleDrag>,
    diamond: bool,
    elevated: bool,
    footprint: [u32; 2],
    pan: Vec2,
    pan_drag: bool,
    grid: bool,
    preview_grid: bool,
    width: u32,
    height: u32,
    zoom: f32,
    repeat_zoom: f32,
    changed: bool,
    confirm_close: bool,
    message: String,
    wake: bool,
    wrap: bool,
    edit_unit: bool,
    brush_size: u32,
    preview_detached: bool,
    revision: u64,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            background_custom: false,
            background_color: default_background(),
            layers: Vec::new(),
            active_layer: 0,
            layer_thumbnails: Vec::new(),
            open: false,
            pixels: RgbaImage::new(128, 64),
            tiles: vec![RgbaImage::new(128, 64)],
            selected: 0,
            layout: vec![0; 64],
            selected_slot: 0,
            columns: 1,
            rows: 1,
            multiple: false,
            textures: Vec::new(),
            preview_last: None,
            dirty_texture: true,
            dirty_all: true,
            dirty_tiles: Default::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            before: None,
            last: None,
            color: [80, 180, 100, 255],
            tool: Tool::Brush,
            rectangle: None,
            diamond: true,
            elevated: false,
            footprint: [128, 64],
            pan: Vec2::ZERO,
            pan_drag: false,
            grid: true,
            preview_grid: true,
            width: 128,
            height: 64,
            zoom: 4.,
            repeat_zoom: 1.,
            changed: false,
            confirm_close: false,
            message: String::new(),
            wake: false,
            wrap: true,
            edit_unit: true,
            brush_size: 1,
            preview_detached: false,
            revision: 0,
        }
    }
}

// Half-open lattice coordinates give each edge pixel exactly one owner.
fn inside(x: i32, y: i32, w: u32, h: u32, diamond: bool) -> bool {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return false;
    }
    if !diamond {
        return true;
    }
    let a = (2 * x as i64 + 1 - w as i64) * h as i64;
    let b = (2 * y as i64 + 1 - h as i64) * w as i64;
    let limit = w as i64 * h as i64;
    (-limit..limit).contains(&(a + b)) && (-limit..limit).contains(&(b - a))
}

fn line(from: (i32, i32), to: (i32, i32), mut paint: impl FnMut(i32, i32)) {
    let (mut x, mut y) = from;
    let dx = (to.0 - x).abs();
    let sx = if x < to.0 { 1 } else { -1 };
    let dy = -(to.1 - y).abs();
    let sy = if y < to.1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        paint(x, y);
        if (x, y) == to {
            break;
        }
        let e = 2 * error;
        if e >= dy {
            error += dy;
            x += sx;
        }
        if e <= dx {
            error += dx;
            y += sy;
        }
    }
}

fn cell_at(world: Vec2, footprint: [u32; 2], diamond: bool) -> (i32, i32) {
    let x = world.x / footprint[0] as f32;
    let y = world.y / footprint[1] as f32;
    if diamond {
        ((x + y + 0.5).floor() as i32, (y - x + 0.5).floor() as i32)
    } else {
        ((x + 0.5).floor() as i32, (y + 0.5).floor() as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellPixel {
    col: i32,
    row: i32,
    x: u32,
    y: u32,
}
// Integer floor division is essential on negative coordinates and shared diagonal edges.
fn cell_pixel(x: i32, y: i32, w: u32, h: u32, diamond: bool) -> CellPixel {
    let (x, y, w, h) = (x as i64, y as i64, w as i64, h as i64);
    let (col, row, cx, cy) = if diamond {
        let denominator = 2 * w * h;
        let col = ((2 * x + 1) * h + (2 * y + 1) * w + w * h).div_euclid(denominator);
        let row = ((2 * y + 1) * w - (2 * x + 1) * h + w * h).div_euclid(denominator);
        (col, row, (col - row) * w / 2, (col + row) * h / 2)
    } else {
        let col = (x + w / 2).div_euclid(w);
        let row = (y + h / 2).div_euclid(h);
        (col, row, col * w, row * h)
    };
    CellPixel {
        col: col as i32,
        row: row as i32,
        x: (x - cx + w / 2) as u32,
        y: (y - cy + h / 2) as u32,
    }
}
fn cell_center(col: f32, row: f32, w: u32, h: u32, diamond: bool) -> Vec2 {
    if diamond {
        egui::vec2((col - row) * w as f32 / 2., (col + row) * h as f32 / 2.)
    } else {
        egui::vec2(col * w as f32, row * h as f32)
    }
}

impl Editor {
    fn layer_editable(&self) -> bool {
        self.layers
            .get(self.active_layer)
            .is_none_or(|layer| !layer.locked && layer.visible)
    }
    fn sync_layer(&mut self) {
        self.tiles[self.selected] = self.pixels.clone();
        if let Some(layer) = self.layers.get_mut(self.active_layer) {
            layer.tiles = self.tiles.clone();
        }
    }
    fn ensure_layers(&mut self) {
        if !self.layers.is_empty() {
            return;
        }
        self.layers.push(Layer {
            name: "BaseLayer".into(),
            visible: true,
            locked: true,
            tiles: self.tiles.clone(),
        });
        let blank: Vec<_> = self
            .tiles
            .iter()
            .map(|p| RgbaImage::new(p.width(), p.height()))
            .collect();
        self.layers.push(Layer {
            name: format!("{} 1", tr("图层")),
            visible: true,
            locked: false,
            tiles: blank.clone(),
        });
        self.active_layer = 1;
        self.tiles = blank;
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }
    fn paint_background(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        origin: Pos2,
        pixel_size: Vec2,
    ) {
        super::seamless::checker_background(
            painter,
            rect,
            origin,
            pixel_size,
            if self.background_custom {
                &super::ImageBackground::Custom
            } else {
                &super::ImageBackground::Checkerboard
            },
            Some(self.background_color),
        );
    }
    fn background_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("背景色"));
            let before = self.background_custom;
            egui::ComboBox::from_id_salt("tile_background_mode")
                .selected_text(tr(if self.background_custom {
                    "自定义颜色"
                } else {
                    "透明（棋盘格）"
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.background_custom, false, tr("透明（棋盘格）"));
                    ui.selectable_value(&mut self.background_custom, true, tr("自定义颜色"));
                });
            self.changed |= before != self.background_custom;
        });
        if self.background_custom {
            ui.horizontal(|ui| {
                let old = self.background_color;
                let [r, g, b] = self.background_color;
                if let Some(color) = crate::color_property::show_editable(
                    ui,
                    tr("预览背景色"),
                    [r as f64 / 255., g as f64 / 255., b as f64 / 255., 1.],
                ) {
                    self.background_color = [color[0], color[1], color[2]]
                        .map(|c| (c.clamp(0., 1.) * 255.).round() as u8);
                }
                let id = ui.make_persistent_id("tile_background_hex");
                let formatted = format!(
                    "#{:02X}{:02X}{:02X}",
                    self.background_color[0], self.background_color[1], self.background_color[2]
                );
                let mut text = if ui.memory(|m| m.has_focus(id)) {
                    ui.data_mut(|data| data.get_temp::<String>(id))
                        .unwrap_or(formatted)
                } else {
                    formatted
                };
                let response = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .id(id)
                            .desired_width(90.)
                            .char_limit(7),
                    )
                    .on_hover_text("#RRGGBB");
                if response.changed() {
                    if let Some(color) = hex_color(&text, 255) {
                        self.background_color = [color[0], color[1], color[2]];
                    }
                }
                ui.data_mut(|data| data.insert_temp(id, text));
                self.changed |= old != self.background_color;
            });
        }
    }
    fn composite(&self, tile: usize) -> RgbaImage {
        if self.layers.is_empty() {
            return if tile == self.selected {
                self.pixels.clone()
            } else {
                self.tiles[tile].clone()
            };
        }
        let mut out = RgbaImage::new(self.pixels.width(), self.pixels.height());
        for (index, layer) in self.layers.iter().enumerate().filter(|(_, l)| l.visible) {
            let source = if index == self.active_layer {
                if tile == self.selected {
                    &self.pixels
                } else {
                    &self.tiles[tile]
                }
            } else {
                &layer.tiles[tile]
            };
            for (dst, src) in out.pixels_mut().zip(source.pixels()) {
                let sa = u32::from(src[3]);
                let da = u32::from(dst[3]);
                let alpha = sa * 255 + da * (255 - sa);
                if alpha == 0 {
                    continue;
                }
                for c in 0..3 {
                    dst[c] = ((u32::from(src[c]) * sa * 255
                        + u32::from(dst[c]) * da * (255 - sa)
                        + alpha / 2)
                        / alpha) as u8;
                }
                dst[3] = ((alpha + 127) / 255) as u8;
            }
        }
        out
    }
    fn export_cell(&self, tile: usize) -> RgbaImage {
        let mut image = self.composite(tile);
        let (w, h) = image.dimensions();
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            if !inside(x as i32, y as i32, w, h, self.diamond && !self.elevated) {
                *pixel = Rgba([0; 4]);
            }
        }
        image
    }
    fn export_all_cells(&mut self) {
        self.finish_stroke();
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let paths: Vec<_> = (0..self.rows)
            .flat_map(|row| {
                let folder = &folder;
                (0..self.columns).map(move |col| {
                    (
                        row,
                        col,
                        folder.join(format!("tile_r{:02}_c{:02}.png", row + 1, col + 1)),
                    )
                })
            })
            .collect();
        if paths.iter().any(|(_, _, path)| path.exists()) {
            self.message = tr("目标文件已存在，请选择其他文件夹。").into();
            return;
        }
        let mut count = 0;
        for (row, col, path) in &paths {
            if let Err(error) = self
                .export_cell(self.layout[row * 8 + col])
                .save_with_format(path, image::ImageFormat::Png)
            {
                self.message = format!("{count}/{}: {error}", paths.len());
                return;
            }
            count += 1;
        }
        self.message = format!("{}: {count} PNG · {}", tr("已保存"), folder.display());
    }
    fn select_layer(&mut self, index: usize) {
        self.finish_stroke();
        self.sync_layer();
        self.active_layer = index;
        self.tiles = self.layers[index].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }
    fn record_layer_change(&mut self) {
        self.finish_stroke();
        self.sync_layer();
        self.undo.push(self.snapshot());
        self.redo.clear();
        self.changed = true;
        self.revision = self.revision.wrapping_add(1);
        self.trim_history();
        self.invalidate_textures();
    }
    fn reset_document(&mut self, image: RgbaImage) {
        self.record_layer_change();
        self.layers.clear();
        self.active_layer = 0;
        self.tiles = vec![image.clone()];
        self.pixels = image;
        self.selected = 0;
        self.selected_slot = 0;
        self.layout.fill(0);
        self.columns = 1;
        self.rows = 1;
        self.width = self.pixels.width();
        self.height = self.pixels.height();
        self.ensure_layers();
    }
    fn layer_icon_button(
        ui: &mut egui::Ui,
        id: egui::Id,
        selected: bool,
        icon: crate::icons::Icon,
        tooltip: &str,
    ) -> egui::Response {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(28., 28.), egui::Sense::hover());
        let response = ui.interact(rect, id, egui::Sense::click());
        let visuals = ui.style().interact_selectable(&response, selected);
        ui.painter().rect(
            rect,
            3.,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        crate::icons::paint(ui, icon, rect.shrink(5.), visuals.fg_stroke.color);
        response.on_hover_text(tooltip)
    }
    fn layer_lock_button(ui: &mut egui::Ui, id: egui::Id, locked: bool) -> egui::Response {
        Self::layer_icon_button(
            ui,
            id,
            locked,
            if locked {
                crate::icons::Icon::Lock
            } else {
                crate::icons::Icon::Unlock
            },
            tr(if locked {
                "解锁图层"
            } else {
                "锁定图层"
            }),
        )
    }
    fn layer_controls(&mut self, ui: &mut egui::Ui) {
        if self.layers.is_empty() {
            return;
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(tr("图层"));
            let bytes: usize = self
                .layers
                .iter()
                .map(|l| l.tiles.len() * self.pixels.as_raw().len())
                .sum();
            if ui
                .add_enabled(
                    self.layers.len() < 32
                        && bytes + self.tiles.len() * self.pixels.as_raw().len() <= LAYER_BYTES,
                    egui::Button::new("+"),
                )
                .on_hover_text(tr("添加图层"))
                .clicked()
            {
                self.record_layer_change();
                let tiles = self
                    .tiles
                    .iter()
                    .map(|p| RgbaImage::new(p.width(), p.height()))
                    .collect();
                self.layers.push(Layer {
                    name: format!("{} {}", tr("图层"), self.layers.len()),
                    visible: true,
                    locked: false,
                    tiles,
                });
                self.select_layer(self.layers.len() - 1);
            }
        });
        let mut select = None;
        let mut delete = None;
        let mut reorder = None;
        let mut toggle_visible = None;
        let mut toggle_lock = None;
        let mut rename = None;
        let owner = ui.id().with("tile_layers_owner");
        for index in (0..self.layers.len()).rev() {
            let image = if index == self.active_layer {
                &self.pixels
            } else {
                &self.layers[index].tiles[self.selected]
            };
            let thumb = image::imageops::thumbnail(image, 36, 28);
            let data = egui::ColorImage::from_rgba_unmultiplied(
                [thumb.width() as usize, thumb.height() as usize],
                thumb.as_raw(),
            );
            while self.layer_thumbnails.len() <= index {
                self.layer_thumbnails.push(ui.ctx().load_texture(
                    format!("tile-layer-thumb-{}", self.layer_thumbnails.len()),
                    data.clone(),
                    egui::TextureOptions::NEAREST,
                ));
            }
            self.layer_thumbnails[index].set(data, egui::TextureOptions::NEAREST);
            let active = index == self.active_layer;
            let name = self.layers[index].name.clone();
            let visible = self.layers[index].visible;
            let locked = self.layers[index].locked;
            let frame = egui::Frame::new().inner_margin(3).stroke(egui::Stroke::new(
                1_f32,
                if active {
                    ui.visuals().selection.stroke.color
                } else {
                    Color32::GRAY
                },
            ));
            let (row, dropped) = ui.dnd_drop_zone::<(egui::Id, usize), _>(frame, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    if Self::layer_icon_button(
                        ui,
                        owner.with(("visible", index)),
                        visible,
                        if visible {
                            crate::icons::Icon::Eye
                        } else {
                            crate::icons::Icon::EyeOff
                        },
                        tr("显示或隐藏图层"),
                    )
                    .clicked()
                    {
                        toggle_visible = Some(index);
                    }
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(36., 28.), egui::Sense::click());
                    super::seamless::checker_background(
                        ui.painter(),
                        rect,
                        rect.min,
                        Vec2::splat(1.),
                        &super::ImageBackground::Checkerboard,
                        None,
                    );
                    let thumb_size = egui::vec2(thumb.width() as f32, thumb.height() as f32);
                    ui.painter().image(
                        self.layer_thumbnails[index].id(),
                        Rect::from_center_size(rect.center(), thumb_size),
                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                    if response.clicked() {
                        select = Some(index);
                    }
                    let width = (ui.available_width() - 28. - ui.spacing().item_spacing.x).max(1.);
                    let name_response = ui
                        .dnd_drag_source(owner.with(("name", index)), (owner, index), |ui| {
                            ui.add_sized(
                                [width, 28.],
                                egui::Button::new(&name).selected(active).truncate(),
                            )
                        })
                        .inner
                        .on_hover_text(tr("拖拽调整图层顺序"));
                    if name_response.clicked() {
                        select = Some(index);
                    }
                    name_response.context_menu(|ui| {
                        if ui
                            .add_enabled(
                                self.layers.len() > 1 && !locked,
                                egui::Button::new(tr("删除图层")),
                            )
                            .clicked()
                        {
                            delete = Some(index);
                            ui.close_menu();
                        }
                        let mut text = name.clone();
                        if ui
                            .add(egui::TextEdit::singleline(&mut text).char_limit(64))
                            .changed()
                        {
                            rename = Some((index, text));
                        }
                    });
                    let lock = Self::layer_lock_button(ui, owner.with(("lock", index)), locked);
                    if lock.clicked() {
                        toggle_lock = Some(index);
                    }
                });
            });
            row.response.context_menu(|ui| {
                if ui
                    .add_enabled(
                        self.layers.len() > 1 && !locked,
                        egui::Button::new(tr("删除图层")),
                    )
                    .clicked()
                {
                    delete = Some(index);
                    ui.close_menu();
                }
            });
            if let Some(payload) = dropped {
                if payload.0 == owner {
                    reorder = Some((payload.1, index));
                }
            }
        }
        if let Some((index, name)) = rename {
            self.record_layer_change();
            self.layers[index].name = name;
        }
        if let Some(index) = select {
            self.select_layer(index);
        }
        if let Some(index) = toggle_visible {
            self.record_layer_change();
            self.layers[index].visible = !self.layers[index].visible;
        }
        if let Some(index) = toggle_lock {
            self.record_layer_change();
            self.layers[index].locked = !self.layers[index].locked;
        }
        if let Some((from, to)) = reorder {
            if from != to && from < self.layers.len() {
                self.record_layer_change();
                let selected = if self.active_layer == from {
                    to
                } else if from < self.active_layer && self.active_layer <= to {
                    self.active_layer - 1
                } else if to <= self.active_layer && self.active_layer < from {
                    self.active_layer + 1
                } else {
                    self.active_layer
                };
                let layer = self.layers.remove(from);
                self.layers.insert(to, layer);
                self.active_layer = selected;
            }
        }
        if let Some(index) = delete {
            self.record_layer_change();
            self.layers.remove(index);
            self.active_layer = if self.active_layer > index {
                self.active_layer - 1
            } else {
                self.active_layer.min(self.layers.len() - 1)
            };
            self.tiles = self.layers[self.active_layer].tiles.clone();
            self.pixels = self.tiles[self.selected].clone();
        }
        if !self.layer_editable() {
            ui.small(tr("当前图层已锁定或隐藏，不能绘制。"));
        }
    }
    fn invalidate_textures(&mut self) {
        self.dirty_texture = true;
        self.dirty_all = true;
        self.dirty_tiles.clear();
    }
    fn whole_unit(&self) -> bool {
        self.multiple && self.edit_unit && !self.elevated
    }
    fn edit_size(&self) -> Vec2 {
        let (w, h) = self.pixels.dimensions();
        if self.whole_unit() {
            if self.diamond {
                egui::vec2(
                    (self.columns + self.rows) as f32 * w as f32 / 2.,
                    (self.columns + self.rows) as f32 * h as f32 / 2.,
                )
            } else {
                egui::vec2(self.columns as f32 * w as f32, self.rows as f32 * h as f32)
            }
        } else {
            egui::vec2(w as f32, h as f32)
        }
    }
    fn plane_hit(&self, x: i32, y: i32, layout: bool) -> Option<(usize, u32, u32)> {
        let (w, h) = self.pixels.dimensions();
        let cell = cell_pixel(x, y, w, h, self.diamond);
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        if !self.wrap
            && (cell.col < 0 || cell.row < 0 || cell.col >= cols as i32 || cell.row >= rows as i32)
        {
            return None;
        }
        let tile = if layout {
            self.tile_at(cell.col, cell.row)
        } else {
            self.selected
        };
        Some((tile, cell.x, cell.y))
    }
    fn finite_plane_hit(&self, x: i32, y: i32, layout: bool) -> Option<(usize, u32, u32)> {
        let (w, h) = self.pixels.dimensions();
        let cell = cell_pixel(x, y, w, h, self.diamond);
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        if cell.col < 0 || cell.row < 0 || cell.col >= cols as i32 || cell.row >= rows as i32 {
            return None;
        }
        Some((
            if layout {
                self.tile_at(cell.col, cell.row)
            } else {
                self.selected
            },
            cell.x,
            cell.y,
        ))
    }
    fn fill_closed(&mut self, mut point: (i32, i32), layout: bool) {
        self.finish_stroke();
        if !self.layer_editable() {
            return;
        }
        let (w, h) = self.pixels.dimensions();
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        if self.wrap {
            let cell = cell_pixel(point.0, point.1, w, h, self.diamond);
            let shift = cell_center(
                (cell.col.div_euclid(cols as i32) * cols as i32) as f32,
                (cell.row.div_euclid(rows as i32) * rows as i32) as f32,
                w,
                h,
                self.diamond,
            );
            point.0 -= shift.x as i32;
            point.1 -= shift.y as i32;
        }
        let Some((tile, x, y)) = self.finite_plane_hit(point.0, point.1, layout) else {
            return;
        };
        let target = *self.tiles[tile].get_pixel(x, y);
        let color = Rgba(self.color);
        if target == color {
            return;
        }
        let (left, top, width, height) = if self.diamond {
            (
                -(rows as i32 * w as i32) / 2,
                -(h as i32) / 2,
                (cols + rows) * w as usize / 2,
                (cols + rows) * h as usize / 2,
            )
        } else {
            (
                -(w as i32) / 2,
                -(h as i32) / 2,
                cols * w as usize,
                rows * h as usize,
            )
        };
        if width.saturating_mul(height) > 4 * 1024 * 1024 {
            self.message = tr("填充区域过大，请缩小循环范围。").into();
            return;
        }
        let state = self.snapshot();
        let mut visited = vec![false; width * height];
        let mut queue = std::collections::VecDeque::new();
        let mut region = Vec::new();
        queue.push_back(point);
        while let Some((px, py)) = queue.pop_front() {
            let Some((tile, x, y)) = self.finite_plane_hit(px, py, layout) else {
                self.message = tr("区域未闭合，未执行填充。").into();
                return;
            };
            if *state.tiles[tile].get_pixel(x, y) != target {
                continue;
            }
            let index = (py - top) as usize * width + (px - left) as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;
            region.push((tile, x, y));
            for next in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
                if let Some((nt, nx, ny)) = self.finite_plane_hit(next.0, next.1, layout) {
                    let ni = (next.1 - top) as usize * width + (next.0 - left) as usize;
                    if !visited[ni] && *state.tiles[nt].get_pixel(nx, ny) == target {
                        queue.push_back(next);
                    }
                } else {
                    self.message = tr("区域未闭合，未执行填充。").into();
                    return;
                }
            }
        }
        self.before = Some(state);
        for (tile, x, y) in region {
            self.write_pixel(tile, x, y, color);
        }
        self.finish_stroke();
        self.message.clear();
    }
    fn rectangle_input(
        &mut self,
        point: Option<(i32, i32)>,
        active: bool,
        down: bool,
        layout: bool,
        preview: bool,
    ) {
        if active {
            if let Some(point) = point {
                if self.rectangle.is_none() {
                    self.rectangle = Some(RectangleDrag {
                        start: point,
                        end: point,
                        layout,
                        preview,
                        line: self.tool == Tool::Line,
                    });
                }
                if let Some(drag) = self
                    .rectangle
                    .as_mut()
                    .filter(|drag| drag.preview == preview)
                {
                    drag.end = point;
                }
            }
        }
        if !down {
            if let (Some(point), Some(drag)) = (
                point,
                self.rectangle
                    .as_mut()
                    .filter(|drag| drag.preview == preview),
            ) {
                drag.end = point;
            }
        }
        if !down && self.rectangle.is_some_and(|drag| drag.preview == preview) {
            let drag = self.rectangle.take().unwrap();
            let a = drag.start;
            let b = drag.end;
            if drag.line {
                self.plane_stroke(a, b, drag.layout);
            } else {
                self.plane_stroke(a, (b.0, a.1), drag.layout);
                self.plane_stroke((b.0, a.1), b, drag.layout);
                self.plane_stroke(b, (a.0, b.1), drag.layout);
                self.plane_stroke((a.0, b.1), a, drag.layout);
            }
            self.finish_stroke();
        }
    }
    fn rectangle_outline(&self, painter: &egui::Painter, origin: Pos2, scale: f32, preview: bool) {
        if let Some(drag) = self.rectangle.filter(|drag| drag.preview == preview) {
            if drag.line {
                painter.line_segment(
                    [
                        origin
                            + egui::vec2(drag.start.0 as f32 + 0.5, drag.start.1 as f32 + 0.5)
                                * scale,
                        origin
                            + egui::vec2(drag.end.0 as f32 + 0.5, drag.end.1 as f32 + 0.5) * scale,
                    ],
                    egui::Stroke::new(
                        (self.brush_size as f32 * scale).max(1.0),
                        Color32::from_rgba_unmultiplied(
                            self.color[0],
                            self.color[1],
                            self.color[2],
                            self.color[3],
                        ),
                    ),
                );
                return;
            }
            let min = egui::vec2(
                drag.start.0.min(drag.end.0) as f32,
                drag.start.1.min(drag.end.1) as f32,
            );
            let max = egui::vec2(
                (drag.start.0.max(drag.end.0) + 1) as f32,
                (drag.start.1.max(drag.end.1) + 1) as f32,
            );
            painter.rect_stroke(
                Rect::from_min_max(origin + min * scale, origin + max * scale),
                0.,
                egui::Stroke::new(1_f32, Color32::YELLOW),
                egui::StrokeKind::Inside,
            );
        }
    }
    fn tool_shortcuts(&mut self, ctx: &egui::Context) {
        if Self::text_input_active(ctx) || ctx.input(|i| !i.focused || i.pointer.primary_down()) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
            self.finish_stroke();
            self.history(false);
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)) {
            self.finish_stroke();
            self.history(true);
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::OpenBracket)) {
            self.brush_size = self.brush_size.saturating_sub(1).max(1);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::CloseBracket)) {
            self.brush_size = (self.brush_size + 1).min(32);
        }
        for (key, tool) in [
            (egui::Key::B, Tool::Brush),
            (egui::Key::I, Tool::Eyedropper),
            (egui::Key::E, Tool::Eraser),
            (egui::Key::U, Tool::Rectangle),
            (egui::Key::L, Tool::Line),
            (egui::Key::G, Tool::Fill),
        ] {
            if self.elevated && matches!(tool, Tool::Rectangle | Tool::Line | Tool::Fill) {
                continue;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.finish_stroke();
                self.tool = tool;
            }
        }
    }
    fn text_input_active(ctx: &egui::Context) -> bool {
        ctx.memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some())
    }
    fn write_pixel(&mut self, tile: usize, x: u32, y: u32, color: Rgba<u8>) {
        if !self.layer_editable() || *self.tiles[tile].get_pixel(x, y) == color {
            return;
        }
        self.tiles[tile].put_pixel(x, y, color);
        if tile == self.selected {
            self.pixels.put_pixel(x, y, color);
        }
        self.dirty_texture = true;
        self.dirty_tiles.insert(tile);
        self.revision = self.revision.wrapping_add(1);
    }
    fn plane_stroke(&mut self, from: (i32, i32), to: (i32, i32), layout: bool) {
        if self.before.is_none() {
            self.before = Some(self.snapshot());
        }
        let mut points = Vec::new();
        line(from, to, |x, y| points.push((x, y)));
        let size = self.brush_size as i32;
        let start = -(size / 2);
        let color = if self.tool == Tool::Eraser {
            Rgba([0; 4])
        } else {
            Rgba(self.color)
        };
        for (x, y) in points {
            for dy in start..start + size {
                for dx in start..start + size {
                    if let Some((tile, px, py)) = self.plane_hit(x + dx, y + dy, layout) {
                        self.write_pixel(tile, px, py, color);
                    }
                }
            }
        }
    }
    pub fn active(&self) -> bool {
        self.open || self.changed
    }
    fn snapshot(&self) -> Snapshot {
        let mut tiles = self.tiles.clone();
        tiles[self.selected] = self.pixels.clone();
        let mut layers = self.layers.clone();
        if let Some(layer) = layers.get_mut(self.active_layer) {
            layer.tiles = tiles.clone();
        }
        Snapshot {
            layers,
            active_layer: self.active_layer,
            tiles,
            selected: self.selected,
            layout: self.layout.clone(),
            columns: self.columns,
            rows: self.rows,
            multiple: self.multiple,
            diamond: self.diamond,
            elevated: self.elevated,
            footprint: self.footprint,
            wrap: self.wrap,
        }
    }
    fn select(&mut self, index: usize) {
        self.tiles[self.selected] = self.pixels.clone();
        self.selected = index;
        self.pixels = self.tiles[index].clone();
        self.invalidate_textures();
    }
    fn finish_stroke(&mut self) {
        if let Some(before) = self.before.take() {
            if before != self.snapshot() {
                self.undo.push(before);
                self.redo.clear();
                self.changed = true;
                self.trim_history();
            }
        }
        self.last = None;
        self.preview_last = None;
        self.rectangle = None;
    }
    fn trim_history(&mut self) {
        while self.undo.len() + self.redo.len() > 1
            && self
                .undo
                .iter()
                .chain(&self.redo)
                .map(Snapshot::bytes)
                .sum::<usize>()
                > HISTORY_BYTES
        {
            if !self.undo.is_empty() {
                self.undo.remove(0);
            } else {
                self.redo.remove(0);
            }
        }
    }
    fn replace(&mut self, image: RgbaImage) {
        if !self.layer_editable() {
            self.message = tr("当前图层已锁定，请先解锁或选择其他图层。").into();
            return;
        }
        self.finish_stroke();
        self.undo.push(self.snapshot());
        if image.dimensions() != self.pixels.dimensions() {
            self.tiles = vec![image.clone()];
            self.selected = 0;
            self.layout.fill(0);
            self.layers.clear();
            self.active_layer = 0;
        }
        self.pixels = image;
        self.tiles[self.selected] = self.pixels.clone();
        self.sync_layer();
        self.redo.clear();
        self.trim_history();
        self.changed = true;
        self.invalidate_textures();
    }
    fn history(&mut self, redo: bool) {
        self.finish_stroke();
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        if let Some(state) = stack.pop() {
            let old = self.snapshot();
            self.pixels = state.tiles[state.selected].clone();
            self.layers = state.layers;
            self.active_layer = state.active_layer;
            self.tiles = state.tiles;
            self.selected = state.selected;
            self.layout = state.layout;
            self.columns = state.columns;
            self.rows = state.rows;
            self.multiple = state.multiple;
            self.diamond = state.diamond;
            self.elevated = state.elevated;
            self.footprint = state.footprint;
            self.wrap = state.wrap;
            if redo {
                self.undo.push(old);
            } else {
                self.redo.push(old);
            }
            self.invalidate_textures();
            self.changed = true;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    fn sync_texture(&mut self, ctx: &egui::Context) {
        if !self.dirty_texture {
            return;
        }
        self.tiles[self.selected] = self.pixels.clone();
        self.textures.truncate(self.tiles.len());
        for index in 0..self.tiles.len() {
            if !self.dirty_all && !self.dirty_tiles.contains(&index) && index < self.textures.len()
            {
                continue;
            }
            let mut display = self.composite(index);
            let (w, h) = display.dimensions();
            for (x, y, p) in display.enumerate_pixels_mut() {
                if !inside(x as i32, y as i32, w, h, self.diamond && !self.elevated) {
                    *p = Rgba([0; 4]);
                }
            }
            let data = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                display.as_raw(),
            );
            if let Some(texture) = self.textures.get_mut(index) {
                texture.set(data, egui::TextureOptions::NEAREST);
            } else {
                self.textures.push(ctx.load_texture(
                    format!("pixel-tile-{index}"),
                    data,
                    egui::TextureOptions::NEAREST,
                ));
            }
        }
        self.dirty_texture = false;
        self.dirty_all = false;
        self.dirty_tiles.clear();
    }
    fn outline(p: &egui::Painter, rect: Rect, diamond: bool, color: Color32) {
        if diamond {
            p.add(egui::Shape::closed_line(
                vec![
                    rect.center_top(),
                    rect.right_center(),
                    rect.center_bottom(),
                    rect.left_center(),
                ],
                egui::Stroke::new(1_f32, color),
            ));
        } else {
            p.rect_stroke(
                rect,
                0.,
                egui::Stroke::new(1_f32, color),
                egui::StrokeKind::Inside,
            );
        }
    }
    fn brush_cursor(&self, ui: &egui::Ui, response: &egui::Response, origin: Pos2, scale: f32) {
        if !response.hovered()
            || !matches!(self.tool, Tool::Brush | Tool::Eraser)
            || ui.input(|i| {
                i.modifiers.alt
                    || i.key_down(egui::Key::Space)
                    || i.pointer.secondary_down()
                    || i.pointer.middle_down()
            })
            || self.pan_drag
        {
            return;
        }
        if !self.layer_editable() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::NotAllowed);
            return;
        }
        let Some(pointer) = response.hover_pos() else {
            return;
        };
        let size = if self.elevated {
            1
        } else {
            self.brush_size as i32
        };
        let point = (pointer - origin) / scale;
        let min = origin
            + egui::vec2(
                (point.x.floor() as i32 - size / 2) as f32,
                (point.y.floor() as i32 - size / 2) as f32,
            ) * scale;
        let rect = Rect::from_min_size(min, Vec2::splat(size as f32 * scale));
        let painter = ui.painter_at(response.rect);
        ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);
        if self.tool == Tool::Brush {
            painter.rect_filled(
                rect,
                0.,
                Color32::from_rgba_unmultiplied(
                    self.color[0],
                    self.color[1],
                    self.color[2],
                    self.color[3],
                ),
            );
        }
        painter.rect_stroke(
            rect,
            0.,
            egui::Stroke::new(3_f32, Color32::BLACK),
            egui::StrokeKind::Outside,
        );
        painter.rect_stroke(
            rect,
            0.,
            egui::Stroke::new(1_f32, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (area, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ui.available_height().max(100.)),
            egui::Sense::click_and_drag(),
        );
        let tools = self.floating_tools(ui, area);
        let over_tools = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| tools.contains(p));
        let painter = ui.painter_at(area);
        painter.rect_filled(area, 0., Color32::from_gray(45));
        if response.hovered() && !over_tools {
            let scroll = ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta.y;
                input.smooth_scroll_delta = Vec2::ZERO;
                delta
            });
            if scroll != 0. {
                self.finish_stroke();
                let old = self.zoom;
                self.zoom = (old * (scroll * 0.003).exp()).clamp(0.25, 64.);
                if let Some(pointer) = response.hover_pos() {
                    let offset = pointer - area.center();
                    self.pan = offset - (offset - self.pan) * (self.zoom / old);
                }
                ui.ctx().request_repaint();
            }
        }
        let down = ui.input(|i| i.pointer.primary_down());
        let space =
            ui.input(|i| i.key_down(egui::Key::Space)) && !Self::text_input_active(ui.ctx());
        let moving = response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
            || (response.is_pointer_button_down_on() && down && (space || self.pan_drag));
        if moving {
            self.finish_stroke();
            self.pan_drag = down;
            self.pan += ui.input(|i| i.pointer.delta());
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Grabbing);
        } else if response.hovered() && space {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Grab);
        }
        let panning = moving || self.pan_drag || (space && response.hovered());
        let (w, h) = self.pixels.dimensions();
        let whole = self.whole_unit();
        let (cols, rows) = if whole {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        let first = egui::vec2(
            -if self.diamond {
                rows as f32 * w as f32 / 2.
            } else {
                w as f32 / 2.
            },
            -(h as f32) / 2.,
        );
        let bounds = Rect::from_min_size(first.to_pos2(), self.edit_size());
        let scale = self.zoom / ui.ctx().pixels_per_point();
        let origin = area.center() + self.pan - bounds.center().to_vec2() * scale;
        let visible = Rect::from_min_max(
            origin + bounds.min.to_vec2() * scale,
            origin + bounds.max.to_vec2() * scale,
        );
        self.paint_background(
            &painter,
            visible.intersect(area),
            visible.min,
            Vec2::splat(scale),
        );
        let point = response
            .interact_pointer_pos()
            .filter(|p| area.contains(*p) && !over_tools)
            .map(|p| {
                (
                    ((p.x - origin.x) / scale).floor() as i32,
                    ((p.y - origin.y) / scale).floor() as i32,
                )
            });
        let special = !self.elevated
            && !ui.input(|i| i.modifiers.alt)
            && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill);
        if panning {
            self.last = None;
        } else if special {
            if matches!(self.tool, Tool::Rectangle | Tool::Line) {
                self.rectangle_input(
                    point,
                    response.is_pointer_button_down_on() && down || response.clicked(),
                    down,
                    whole,
                    false,
                );
            } else if response.clicked() {
                if let Some(point) = point {
                    self.fill_closed(point, whole);
                }
            }
        } else if response.is_pointer_button_down_on() && down || response.clicked() {
            if let Some(point) = point {
                if self.tool == Tool::Eyedropper || ui.input(|i| i.modifiers.alt) {
                    let hit = if self.elevated {
                        let x = point.0 + w as i32 / 2;
                        let y = point.1 + h as i32 / 2;
                        if inside(x, y, w, h, false) {
                            Some((self.selected, x as u32, y as u32))
                        } else {
                            None
                        }
                    } else {
                        self.plane_hit(point.0, point.1, whole)
                    };
                    if let Some((tile, x, y)) = hit {
                        self.color = self.composite(tile).get_pixel(x, y).0;
                    }
                    self.last = None;
                } else {
                    if self.elevated {
                        if self.before.is_none() {
                            self.before = Some(self.snapshot());
                        }
                        let mut points = Vec::new();
                        line(self.last.unwrap_or(point), point, |x, y| {
                            points.push((x, y))
                        });
                        for (x, y) in points {
                            let x = x + w as i32 / 2;
                            let y = y + h as i32 / 2;
                            if inside(x, y, w, h, false) {
                                self.write_pixel(
                                    self.selected,
                                    x as u32,
                                    y as u32,
                                    if self.tool == Tool::Eraser {
                                        Rgba([0; 4])
                                    } else {
                                        Rgba(self.color)
                                    },
                                );
                            }
                        }
                    } else {
                        self.plane_stroke(self.last.unwrap_or(point), point, whole);
                    }
                    self.last = Some(point);
                }
            } else {
                self.last = None;
            }
        }
        if !down {
            if !self.rectangle.is_some_and(|drag| drag.preview) {
                self.finish_stroke();
            }
            self.pan_drag = false;
        }
        self.sync_texture(ui.ctx());
        for row in 0..rows {
            for col in 0..cols {
                let center =
                    origin + cell_center(col as f32, row as f32, w, h, self.diamond) * scale;
                let rect = Rect::from_center_size(center, egui::vec2(w as f32, h as f32) * scale);
                let index = if whole {
                    self.tile_at(col as i32, row as i32)
                } else {
                    self.selected
                };
                painter.image(
                    self.textures[index].id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                    Color32::WHITE,
                );
                if self.grid {
                    Self::outline(
                        &painter,
                        rect,
                        self.diamond && !self.elevated,
                        Color32::from_white_alpha(85),
                    );
                }
            }
        }
        self.rectangle_outline(&painter, origin, scale, false);
        if self.grid && scale >= 6. {
            let clip = visible.intersect(area);
            let stroke = egui::Stroke::new(0.5_f32, Color32::from_white_alpha(30));
            let x0 = ((clip.left() - origin.x) / scale).ceil() as i32;
            let x1 = ((clip.right() - origin.x) / scale).floor() as i32;
            let y0 = ((clip.top() - origin.y) / scale).ceil() as i32;
            let y1 = ((clip.bottom() - origin.y) / scale).floor() as i32;
            for x in x0..=x1 {
                let x = origin.x + x as f32 * scale;
                painter.line_segment(
                    [egui::pos2(x, clip.top()), egui::pos2(x, clip.bottom())],
                    stroke,
                );
            }
            for y in y0..=y1 {
                let y = origin.y + y as f32 * scale;
                painter.line_segment(
                    [egui::pos2(clip.left(), y), egui::pos2(clip.right(), y)],
                    stroke,
                );
            }
        }
        let corners = if self.diamond && !self.elevated {
            [
                cell_center(-0.5, -0.5, w, h, true),
                cell_center(cols as f32 - 0.5, -0.5, w, h, true),
                cell_center(cols as f32 - 0.5, rows as f32 - 0.5, w, h, true),
                cell_center(-0.5, rows as f32 - 0.5, w, h, true),
            ]
        } else {
            [
                bounds.left_top().to_vec2(),
                bounds.right_top().to_vec2(),
                bounds.right_bottom().to_vec2(),
                bounds.left_bottom().to_vec2(),
            ]
        };
        painter.add(egui::Shape::closed_line(
            corners.map(|p| origin + p * scale).to_vec(),
            egui::Stroke::new(1_f32, Color32::YELLOW),
        ));
        if self.elevated {
            let foot = egui::vec2(self.footprint[0] as f32, self.footprint[1] as f32) * scale;
            Self::outline(
                &painter,
                Rect::from_center_size(visible.center_bottom() - egui::vec2(0., foot.y / 2.), foot),
                true,
                Color32::LIGHT_BLUE,
            );
        }
        if !over_tools {
            self.brush_cursor(ui, &response, origin, scale);
        }
    }
    fn tile_at(&self, col: i32, row: i32) -> usize {
        if self.multiple {
            self.layout[row.rem_euclid(self.rows as i32) as usize * 8
                + col.rem_euclid(self.columns as i32) as usize]
        } else {
            self.selected
        }
    }
    pub fn load_project(&mut self, path: &Path) -> Result<(), String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(256 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 256 * 1024 * 1024 {
            return Err(tr("无效的 Tile 工程或超出尺寸限制。").into());
        }
        let p: Project = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        p.validate()?;
        let mut seen = std::collections::BTreeSet::new();
        let duplicates = if p.multiple {
            (0..p.rows)
                .flat_map(|r| (0..p.columns).map(move |c| r * 8 + c))
                .filter(|&slot| !seen.insert(p.layout[slot]))
                .count()
        } else {
            0
        };
        let count = p.tiles.len() + duplicates;
        if count > MAX_TILES || count * p.width as usize * p.height as usize * 4 > 16 * 1024 * 1024
        {
            return Err(tr("格子独立副本超出64张或16 MiB限制，请减少行列或缩小画布。").into());
        }
        if count * p.width as usize * p.height as usize * 4 * p.layers.len().max(2) > LAYER_BYTES {
            return Err(tr("图层像素总量超过32 MiB限制。").into());
        }
        self.finish_stroke();
        self.undo.push(self.snapshot());
        self.redo.clear();
        self.tiles = p
            .tiles
            .into_iter()
            .map(|bytes| {
                RgbaImage::from_raw(p.width, p.height, bytes).expect("validated dimensions")
            })
            .collect();
        self.layers = p
            .layers
            .into_iter()
            .map(|l| Layer {
                name: l.name,
                visible: l.visible,
                locked: l.locked,
                tiles: l
                    .tiles
                    .into_iter()
                    .map(|bytes| {
                        RgbaImage::from_raw(p.width, p.height, bytes).expect("validated layer")
                    })
                    .collect(),
            })
            .collect();
        self.active_layer = p.active_layer;
        self.background_custom = p.background_custom;
        self.background_color = p.background_color;
        self.selected = p.selected;
        if let Some(layer) = self.layers.get(self.active_layer) {
            self.tiles = layer.tiles.clone();
        }
        self.pixels = self.tiles[self.selected].clone();
        self.layout = p.layout;
        self.columns = p.columns;
        self.rows = p.rows;
        self.multiple = p.multiple;
        self.diamond = p.diamond || p.elevated;
        self.elevated = p.elevated;
        self.footprint = p.footprint;
        self.wrap = p.wrap;
        self.edit_unit = p.edit_unit;
        self.brush_size = p.brush_size;
        self.width = p.width;
        self.height = p.height;
        if self.multiple {
            self.independent_slots()
                .expect("validated independent slot budget");
        }
        self.selected_slot = (0..self.rows)
            .flat_map(|r| (0..self.columns).map(move |c| r * 8 + c))
            .find(|&slot| self.layout[slot] == self.selected)
            .unwrap_or(0);
        self.invalidate_textures();
        self.changed = duplicates != 0;
        self.trim_history();
        self.message = path.display().to_string();
        self.wake = true;
        self.confirm_close = false;
        Ok(())
    }
    fn project_controls(&mut self, ui: &mut egui::Ui, source_path: &Path) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("保存 Tile 工程")).clicked() {
                self.finish_stroke();
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("tiles.uytile")
                    .add_filter("Pixel Tile", &["uytile"])
                    .save_file()
                {
                    let state = self.snapshot();
                    let project = Project {
                        version: 3,
                        background_custom: self.background_custom,
                        background_color: self.background_color,
                        layers: state
                            .layers
                            .iter()
                            .map(|l| StoredLayer {
                                name: l.name.clone(),
                                visible: l.visible,
                                locked: l.locked,
                                tiles: l.tiles.iter().map(|p| p.as_raw().clone()).collect(),
                            })
                            .collect(),
                        active_layer: state.active_layer,
                        width: self.pixels.width(),
                        height: self.pixels.height(),
                        tiles: state.tiles.iter().map(|p| p.as_raw().clone()).collect(),
                        selected: self.selected,
                        layout: self.layout.clone(),
                        columns: self.columns,
                        rows: self.rows,
                        multiple: self.multiple,
                        diamond: self.diamond,
                        elevated: self.elevated,
                        footprint: self.footprint,
                        wrap: self.wrap,
                        edit_unit: self.edit_unit,
                        brush_size: self.brush_size,
                    };
                    let result = if same_file(&path, source_path) {
                        Err(tr("请另存到其他文件，保留源图片。").into())
                    } else {
                        serde_json::to_vec(&project)
                            .map_err(|e| e.to_string())
                            .and_then(|bytes| {
                                std::fs::write(&path, bytes).map_err(|e| e.to_string())
                            })
                    };
                    match result {
                        Ok(()) => {
                            self.changed = false;
                            self.message = format!("{}: {}", tr("已保存"), path.display());
                        }
                        Err(e) => self.message = e,
                    }
                }
            }
            if ui.button(tr("打开 Tile 工程")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Pixel Tile", &["uytile"])
                    .pick_file()
                {
                    if let Err(e) = self.load_project(&path) {
                        self.message = e;
                    }
                }
            }
        });
    }
    fn independent_slots(&mut self) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        let mut duplicates = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.columns {
                let slot = row * 8 + col;
                if !seen.insert(self.layout[slot]) {
                    duplicates.push(slot);
                }
            }
        }
        if duplicates.is_empty() {
            return Ok(());
        }
        let count = self.tiles.len() + duplicates.len();
        if !self.layers.is_empty()
            && count * self.layers.len() * self.pixels.as_raw().len() > LAYER_BYTES
        {
            return Err(tr("图层像素总量超过32 MiB限制。").into());
        }
        self.sync_layer();
        if count > MAX_TILES || count * self.pixels.as_raw().len() > 16 * 1024 * 1024 {
            return Err(tr("格子独立副本超出64张或16 MiB限制，请减少行列或缩小画布。").into());
        }
        for slot in duplicates {
            let source = self.layout[slot];
            for layer in &mut self.layers {
                let image = layer.tiles[source].clone();
                layer.tiles.push(image);
            }
            let image = self.tiles[source].clone();
            self.layout[slot] = self.tiles.len();
            self.tiles.push(image);
        }
        self.invalidate_textures();
        Ok(())
    }
    fn tile_controls(&mut self, ui: &mut egui::Ui, _source_path: &Path) {
        let old = (self.multiple, self.columns, self.rows);
        let mut multiple = self.multiple;
        let mut columns = self.columns;
        let mut rows = self.rows;
        ui.checkbox(&mut multiple, tr("多 Tile 循环"));
        if multiple {
            ui.horizontal(|ui| {
                ui.label(tr("循环列数"));
                ui.add(egui::DragValue::new(&mut columns).range(1..=8));
                ui.label(tr("循环行数"));
                ui.add(egui::DragValue::new(&mut rows).range(1..=8));
            });
        }
        if multiple {
            ui.horizontal(|ui| {
                for size in [2, 3, 4] {
                    if ui.button(format!("{size}×{size}")).clicked() {
                        columns = size;
                        rows = size;
                    }
                }
            });
        }
        if (multiple, columns, rows) != old {
            self.finish_stroke();
            let before = self.snapshot();
            self.multiple = multiple;
            self.columns = columns;
            self.rows = rows;
            if multiple {
                if let Err(error) = self.independent_slots() {
                    (self.multiple, self.columns, self.rows) = old;
                    self.message = error;
                } else {
                    self.undo.push(before);
                    self.redo.clear();
                    self.changed = true;
                }
            } else {
                self.undo.push(before);
                self.redo.clear();
                self.changed = true;
            }
            self.trim_history();
        }
        if self.multiple {
            let row = (self.selected_slot / 8).min(self.rows - 1);
            let col = (self.selected_slot % 8).min(self.columns - 1);
            self.selected_slot = row * 8 + col;
            if self.layout[self.selected_slot] != self.selected {
                if let Some(slot) = (0..self.rows)
                    .flat_map(|r| (0..self.columns).map(move |c| r * 8 + c))
                    .find(|&slot| self.layout[slot] == self.selected)
                {
                    self.selected_slot = slot;
                } else {
                    self.select(self.layout[self.selected_slot]);
                }
            }
            let cell_width = ((ui.available_width() - 4. * (self.columns - 1) as f32)
                / self.columns as f32)
                .max(24.);
            egui::Grid::new("tile_coordinates")
                .spacing(egui::vec2(4., 4.))
                .show(ui, |ui| {
                    ui.spacing_mut().button_padding.x = 2.;
                    for row in 0..self.rows {
                        for col in 0..self.columns {
                            let slot = row * 8 + col;
                            if ui
                                .add_sized(
                                    [cell_width, 26.],
                                    egui::Button::new(format!("{},{}", row + 1, col + 1))
                                        .selected(self.selected_slot == slot),
                                )
                                .clicked()
                            {
                                self.finish_stroke();
                                self.selected_slot = slot;
                                self.select(self.layout[slot]);
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.label(format!(
                "{} {},{}",
                tr("当前格"),
                self.selected_slot / 8 + 1,
                self.selected_slot % 8 + 1
            ));
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("导入图片")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp", "tga"])
                    .pick_file()
                {
                    let loaded = (|| -> Result<RgbaImage, String> {
                        let mut reader = image::ImageReader::open(path)
                            .map_err(|e| e.to_string())?
                            .with_guessed_format()
                            .map_err(|e| e.to_string())?;
                        let mut limits = image::Limits::default();
                        limits.max_image_width = Some(MAX_SIDE);
                        limits.max_image_height = Some(MAX_SIDE);
                        limits.max_alloc = Some(32 * 1024 * 1024);
                        reader.limits(limits);
                        reader
                            .decode()
                            .map(|i| i.to_rgba8())
                            .map_err(|e| e.to_string())
                    })();
                    match loaded {
                        Ok(image) if image.dimensions() == self.pixels.dimensions() => {
                            self.replace(image)
                        }
                        Ok(_) => self.message = tr("Tile 尺寸必须与当前画布一致。").into(),
                        Err(error) => self.message = error,
                    }
                }
            }
            if ui.button(tr("清空当前格")).clicked() {
                self.replace(RgbaImage::new(self.pixels.width(), self.pixels.height()));
            }
        });
        if self.multiple && ui.button(tr("拆分导出全部格 PNG")).clicked() {
            self.export_all_cells();
        }
    }
    fn preview(&mut self, ui: &mut egui::Ui, editable: bool) {
        let (area, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ui.available_height().max(80.)),
            egui::Sense::click_and_drag(),
        );
        if response.hovered() {
            let scroll = ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta.y;
                input.smooth_scroll_delta = Vec2::ZERO;
                delta
            });
            if scroll != 0. {
                if editable {
                    self.finish_stroke();
                }
                self.repeat_zoom = (self.repeat_zoom * (scroll * 0.003).exp()).clamp(0.25, 8.);
                ui.ctx().request_repaint();
            }
        }
        let p = ui.painter_at(area);
        self.paint_background(&p, area, area.min, Vec2::splat(1.));
        let (w, h) = self.pixels.dimensions();
        let footprint = if self.elevated {
            self.footprint
        } else {
            [w, h]
        };
        let mut scale = self.repeat_zoom / ui.ctx().pixels_per_point();
        scale = scale.max(12. / footprint[0].min(footprint[1]) as f32);
        // Keep pathological sprite/footprint ratios from creating an unbounded draw list.
        scale = scale.max(h as f32 / footprint[1] as f32 / 8.);
        let size = egui::vec2(footprint[0] as f32, footprint[1] as f32) * scale;
        let image_size = egui::vec2(w as f32, h as f32) * scale;
        let row_min = (-(area.height() / 2. + size.y)
            / if self.diamond { size.y / 2. } else { size.y })
        .floor() as i32;
        let row_max = ((area.height() / 2. + image_size.y)
            / if self.diamond { size.y / 2. } else { size.y })
        .ceil() as i32;
        let cols = ((area.width() + image_size.x) / size.x / 2.).ceil() as i32 + 1;
        let mut instances = Vec::new();
        for row in row_min.max(-32)..=row_max.min(32) {
            for col in -cols.min(32)..=cols.min(32) {
                let offset = egui::vec2(
                    (col as f32
                        + if self.diamond {
                            row.rem_euclid(2) as f32 / 2.
                        } else {
                            0.
                        })
                        * size.x,
                    row as f32 * size.y * if self.diamond { 0.5 } else { 1. },
                );
                let center = area.center() + offset;
                let rect = Rect::from_min_size(
                    center + egui::vec2(-image_size.x / 2., size.y / 2. - image_size.y),
                    image_size,
                );
                let (i, j) = if self.diamond {
                    (col + (row + 1).div_euclid(2), row.div_euclid(2) - col)
                } else {
                    (col, row)
                };
                if rect.intersects(area) {
                    instances.push((
                        rect,
                        Rect::from_center_size(center, size),
                        self.tile_at(i, j),
                    ));
                }
            }
        }
        let point = response
            .interact_pointer_pos()
            .filter(|v| area.contains(*v))
            .map(|v| {
                (
                    ((v.x - area.center().x) / scale).floor() as i32,
                    ((v.y - area.center().y) / scale).floor() as i32,
                )
            });
        let special = editable
            && !self.elevated
            && !ui.input(|i| i.modifiers.alt)
            && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill);
        if special {
            if matches!(self.tool, Tool::Rectangle | Tool::Line) {
                self.rectangle_input(
                    point,
                    response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down())
                        || response.clicked(),
                    ui.input(|i| i.pointer.primary_down()),
                    true,
                    true,
                );
            } else if response.clicked() {
                if let Some(point) = point {
                    self.fill_closed(point, true);
                }
            }
        } else if editable
            && ((response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()))
                || response.clicked())
        {
            if let Some(point) = point {
                let picking = self.tool == Tool::Eyedropper || ui.input(|i| i.modifiers.alt);
                if !picking && self.before.is_none() {
                    self.before = Some(self.snapshot());
                }
                if !self.elevated {
                    if picking {
                        if let Some((tile, x, y)) = self.plane_hit(point.0, point.1, true) {
                            self.color = self.tiles[tile].get_pixel(x, y).0;
                        }
                    } else {
                        self.plane_stroke(self.preview_last.unwrap_or(point), point, true);
                    }
                } else {
                    let mut points = Vec::new();
                    line(self.preview_last.unwrap_or(point), point, |x, y| {
                        points.push((x, y))
                    });
                    for (x, y) in points {
                        let world = egui::vec2(x as f32 + 0.5, y as f32 + 0.5);
                        let mut hit = None;
                        if self.elevated {
                            let screen = area.center() + world * scale;
                            for (rect, _, index) in instances.iter().rev() {
                                if !rect.contains(screen) {
                                    continue;
                                }
                                let local = (screen - rect.min) / scale;
                                let (px, py) = (local.x.floor() as u32, local.y.floor() as u32);
                                let hit_pixels = self
                                    .before
                                    .as_ref()
                                    .map_or(&self.tiles[*index], |s| &s.tiles[*index]);
                                if px < w && py < h && hit_pixels.get_pixel(px, py)[3] > 0 {
                                    hit = Some((*index, px, py));
                                    break;
                                }
                            }
                        }
                        if hit.is_none() {
                            let (col, row) = cell_at(world, footprint, self.diamond);
                            let center = if self.diamond {
                                egui::vec2(
                                    (col - row) as f32 * footprint[0] as f32 / 2.,
                                    (col + row) as f32 * footprint[1] as f32 / 2.,
                                )
                            } else {
                                egui::vec2(
                                    col as f32 * footprint[0] as f32,
                                    row as f32 * footprint[1] as f32,
                                )
                            };
                            let local = world - center
                                + egui::vec2(w as f32 / 2., h as f32 - footprint[1] as f32 / 2.);
                            let (px, py) = (local.x.floor() as i32, local.y.floor() as i32);
                            if inside(px, py, w, h, self.diamond && !self.elevated) {
                                hit = Some((self.tile_at(col, row), px as u32, py as u32));
                            }
                        }
                        if let Some((index, px, py)) = hit {
                            if picking {
                                self.color = self.composite(index).get_pixel(px, py).0;
                            } else {
                                let color = if self.tool == Tool::Eraser {
                                    Rgba([0; 4])
                                } else {
                                    Rgba(self.color)
                                };
                                self.write_pixel(index, px, py, color);
                            }
                        }
                    }
                }
                self.preview_last = if picking { None } else { Some(point) };
            } else {
                self.preview_last = None;
            }
        }
        if editable && !ui.input(|i| i.pointer.primary_down()) {
            self.finish_stroke();
        }
        self.sync_texture(ui.ctx());
        for (rect, ground, index) in instances {
            p.image(
                self.textures[index].id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                Color32::WHITE,
            );
            if self.preview_grid {
                Self::outline(&p, ground, self.diamond, Color32::from_white_alpha(65));
            }
        }
        self.rectangle_outline(&p, area.center(), scale, true);
        if editable {
            self.brush_cursor(ui, &response, area.center(), scale);
        }
    }
    fn drawing_tools(&mut self, ui: &mut egui::Ui) {
        if self.elevated && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill) {
            self.finish_stroke();
            self.tool = Tool::Brush;
        }
        ui.horizontal_wrapped(|ui| {
            if let Some(color) = crate::color_property::show_editable(
                ui,
                tr("绘制颜色"),
                self.color.map(|v| f64::from(v) / 255.0),
            ) {
                self.color = color.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = 100.0;
                let mut alpha = f32::from(self.color[3]) * 100.0 / 255.0;
                ui.label("Alpha");
                if ui
                    .add(egui::Slider::new(&mut alpha, 0.0..=100.0).max_decimals(0))
                    .changed()
                {
                    self.color[3] = (alpha * 255.0 / 100.0).round() as u8;
                }
                ui.label("%");
            });
            ui.add_enabled_ui(!self.elevated, |ui| {
                for size in [1, 4, 8, 16, 32] {
                    let response = ui
                        .add_sized(
                            [22., 22.],
                            egui::Button::new("").selected(self.brush_size == size),
                        )
                        .on_hover_text(format!("{}: {size} px", tr("方形画笔尺寸")));
                    let side = 2. + 16. * size as f32 / 32.;
                    ui.painter().rect_filled(
                        Rect::from_center_size(response.rect.center(), Vec2::splat(side)),
                        0.,
                        ui.visuals().text_color(),
                    );
                    if response.clicked() {
                        self.brush_size = size;
                    }
                }
                ui.spacing_mut().slider_width = 90.;
                ui.add(egui::Slider::new(&mut self.brush_size, 1..=32))
                    .on_hover_text(format!("{} ([ / ])", tr("方形画笔尺寸")));
            });
        });
    }
    fn floating_tools(&mut self, ui: &egui::Ui, area: Rect) -> Rect {
        let ctx = ui.ctx().clone();
        egui::Area::new(ui.id().with(("tile_floating_tools", ctx.viewport_id())))
            .order(egui::Order::Foreground)
            .fixed_pos(area.min + egui::vec2(4., 4.))
            .movable(false)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.;
                        for (tool, icon, label, key) in [
                            (Tool::Brush, crate::icons::Icon::Brush, "画笔", "B"),
                            (
                                Tool::Eyedropper,
                                crate::icons::Icon::Eyedropper,
                                "取色",
                                "I",
                            ),
                            (Tool::Eraser, crate::icons::Icon::Eraser, "橡皮擦", "E"),
                            (
                                Tool::Rectangle,
                                crate::icons::Icon::RectangleTool,
                                "框画",
                                "U",
                            ),
                            (Tool::Line, crate::icons::Icon::LineTool, "直线", "L"),
                            (Tool::Fill, crate::icons::Icon::PaintBucket, "填充", "G"),
                        ] {
                            let selected = self.tool == tool;
                            let visuals = ui.visuals().clone();
                            let enabled = !self.elevated
                                || !matches!(tool, Tool::Rectangle | Tool::Line | Tool::Fill);
                            let response = ui
                                .add_enabled_ui(enabled, |ui| {
                                    ui.add_sized(
                                        [26., 26.],
                                        egui::Button::new("")
                                            .selected(selected)
                                            .fill(if selected {
                                                visuals.selection.bg_fill
                                            } else {
                                                visuals.extreme_bg_color
                                            })
                                            .stroke(if selected {
                                                egui::Stroke::new(
                                                    2_f32,
                                                    visuals.selection.stroke.color,
                                                )
                                            } else {
                                                visuals.widgets.inactive.bg_stroke
                                            })
                                            .corner_radius(2.),
                                    )
                                })
                                .inner;
                            let color = if self.tool == tool {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().text_color()
                            };
                            crate::icons::paint(ui, icon, response.rect.shrink(5.), color);
                            if selected {
                                ui.painter().line_segment(
                                    [response.rect.left_bottom(), response.rect.right_bottom()],
                                    egui::Stroke::new(3_f32, ui.visuals().selection.stroke.color),
                                );
                            }
                            let tooltip = if tool == Tool::Fill {
                                format!("{} ({key}) · {}", tr(label), tr("仅填充闭合区域"))
                            } else {
                                format!("{} ({key})", tr(label))
                            };
                            if response.on_hover_text(tooltip).clicked() {
                                self.finish_stroke();
                                self.tool = tool;
                            }
                        }
                    });
                });
            })
            .response
            .rect
    }
    fn controls(&mut self, ui: &mut egui::Ui, path: &Path, source: Option<&RgbaImage>) {
        self.project_controls(ui, path);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("画布尺寸"));
            ui.add(egui::DragValue::new(&mut self.width).range(2..=MAX_SIDE));
            ui.label("×");
            ui.add(egui::DragValue::new(&mut self.height).range(2..=MAX_SIDE));
            if ui.button(tr("新建空白")).clicked() {
                self.reset_document(RgbaImage::new(
                    self.width.div_ceil(2) * 2,
                    self.height.div_ceil(2) * 2,
                ));
            }
            if ui
                .add_enabled(
                    source.is_some_and(|s| {
                        s.width() <= MAX_SIDE
                            && s.height() <= MAX_SIDE
                            && s.width() % 2 == 0
                            && s.height() % 2 == 0
                    }),
                    egui::Button::new(tr("导入当前图片")),
                )
                .clicked()
            {
                if let Some(s) = source {
                    self.reset_document(s.clone());
                    self.width = s.width();
                    self.height = s.height();
                }
            }
            if ui.button(tr("另存 PNG")).clicked() {
                self.finish_stroke();
                if let Some(dest) = rfd::FileDialog::new()
                    .set_file_name("tile.png")
                    .add_filter("PNG", &["png"])
                    .save_file()
                {
                    if same_file(&dest, path) {
                        self.message = tr("请另存到其他文件，保留源图片。").into();
                    } else {
                        let mut out = self.composite(self.selected);
                        let (w, h) = out.dimensions();
                        for (x, y, p) in out.enumerate_pixels_mut() {
                            if !inside(x as i32, y as i32, w, h, self.diamond && !self.elevated) {
                                *p = Rgba([0; 4]);
                            }
                        }
                        match out.save_with_format(&dest, image::ImageFormat::Png) {
                            Ok(()) => {
                                if self.tiles.len() == 1 && self.layers.is_empty() {
                                    self.changed = false;
                                }
                                self.message = format!(
                                    "{}: {}",
                                    tr("已保存当前 Tile；多 Tile 排列请保存工程。"),
                                    dest.display()
                                );
                            }
                            Err(e) => self.message = e.to_string(),
                        }
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.elevated,
                    egui::Checkbox::new(&mut self.diamond, tr("菱形平铺")),
                )
                .changed()
            {
                self.finish_stroke();
                self.invalidate_textures();
                self.changed = true;
            }
            if ui.checkbox(&mut self.elevated, tr("带高度 Tile")).changed() {
                self.finish_stroke();
                self.diamond = true;
                self.invalidate_textures();
                self.changed = true;
            }
        });
        if ui
            .add_enabled(
                !self.elevated,
                egui::Checkbox::new(&mut self.wrap, tr("边缘回绕")),
            )
            .changed()
        {
            self.finish_stroke();
            self.changed = true;
        }
        if ui
            .add_enabled(
                self.multiple && !self.elevated,
                egui::Checkbox::new(&mut self.edit_unit, tr("编辑整个循环单元")),
            )
            .changed()
        {
            self.finish_stroke();
            self.pan = Vec2::ZERO;
            self.changed = true;
        }
        ui.checkbox(&mut self.preview_detached, tr("独立预览窗口"));
        self.tile_controls(ui, path);
        if self.elevated {
            ui.horizontal(|ui| {
                ui.label(tr("地面菱形尺寸"));
                for v in &mut self.footprint {
                    if ui
                        .add(egui::DragValue::new(v).range(2..=MAX_SIDE))
                        .changed()
                    {
                        self.changed = true;
                    }
                    *v = (*v).div_ceil(2) * 2;
                }
            });
        }
        self.layer_controls(ui);
        self.background_controls(ui);
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, path: &Path, source: Option<&RgbaImage>) {
        if !self.open {
            return;
        }
        self.ensure_layers();
        if self.wake {
            crate::native_window::wake(ctx, egui::Id::new(("pixel_tile", path)));
            self.wake = false;
        }
        let revision = self.revision;
        let view_state = (self.preview_detached, self.repeat_zoom, self.preview_grid);
        let mut open = true;
        let title = if path.as_os_str().is_empty() {
            tr("像素 Tile").to_owned()
        } else {
            format!(
                "{} · {}",
                tr("像素 Tile"),
                path.file_name().unwrap_or_default().to_string_lossy()
            )
        };
        crate::native_window::Window::new(title.clone())
            .id(egui::Id::new(("pixel_tile", path)))
            .retain_on_close_request()
            .open(&mut open)
            .default_width(1400.)
            .default_height(850.)
            .min_width(960.)
            .min_height(500.)
            .scrollable(false)
            .show(ctx, |ui| {
                if self.changed && ui.input(|i| i.viewport().close_requested()) {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.confirm_close = true;
                }
                if !self.confirm_close {
                    self.tool_shortcuts(ui.ctx());
                }
                let area = ui.available_rect_before_wrap();
                let gap = 8.;
                let left_width = 270_f32.min((area.width() - 400.).max(200.));
                let left = Rect::from_min_size(area.min, egui::vec2(left_width, area.height()));
                let content_width = (area.width() - left_width - 2. * gap).max(200.);
                let center = Rect::from_min_size(
                    egui::pos2(left.right() + gap, area.top()),
                    egui::vec2(content_width / 2., area.height()),
                );
                let right =
                    Rect::from_min_max(egui::pos2(center.right() + gap, area.top()), area.max);
                let mut settings = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tile_settings")
                        .max_rect(left),
                );
                settings.set_clip_rect(left.intersect(ui.clip_rect()));
                egui::ScrollArea::vertical()
                    .id_salt("tile_settings_scroll")
                    .auto_shrink([false, false])
                    .show(&mut settings, |ui| self.controls(ui, path, source));
                self.sync_texture(ui.ctx());
                let mut paint = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tile_paint")
                        .max_rect(center),
                );
                paint.set_clip_rect(center.intersect(ui.clip_rect()));
                paint.horizontal_wrapped(|ui| {
                    ui.heading(tr("绘制区"));
                    ui.add(
                        egui::DragValue::new(&mut self.zoom)
                            .range(0.25..=64.)
                            .speed(0.25),
                    );
                    ui.checkbox(&mut self.grid, tr("网格"));
                    if ui.button(tr("适应画布")).clicked() {
                        self.pan = Vec2::ZERO;
                        let size = self.edit_size();
                        self.zoom = ((ui.ctx().screen_rect().width() - 320.) / 2. / size.x)
                            .min((ui.ctx().screen_rect().height() - 70.) / size.y)
                            .max(0.25)
                            * ui.ctx().pixels_per_point();
                    }

                    if ui
                        .add_enabled(!self.undo.is_empty(), egui::Button::new(tr("撤销")))
                        .on_hover_text(format!("{} (Ctrl+Z)", tr("撤销")))
                        .clicked()
                    {
                        self.history(false);
                    }
                    if ui
                        .add_enabled(!self.redo.is_empty(), egui::Button::new(tr("重做")))
                        .on_hover_text(format!("{} (Ctrl+Y)", tr("重做")))
                        .clicked()
                    {
                        self.history(true);
                    }
                });
                self.drawing_tools(&mut paint);
                self.canvas(&mut paint);
                let mut preview = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tile_preview")
                        .max_rect(right),
                );
                preview.set_clip_rect(right.intersect(ui.clip_rect()));
                preview.horizontal_wrapped(|ui| {
                    ui.heading(tr("循环预览"));
                    ui.add(
                        egui::DragValue::new(&mut self.repeat_zoom)
                            .range(0.25..=8.)
                            .speed(0.1),
                    );
                    ui.checkbox(&mut self.preview_grid, tr("网格"));
                });
                if self.preview_detached {
                    preview.label(tr("预览已移至独立窗口"));
                    if preview.button(tr("回到右侧预览")).clicked() {
                        self.preview_detached = false;
                    }
                } else {
                    self.preview(&mut preview, true);
                }
                if self.confirm_close {
                    let modal_id = egui::Id::new(("pixel_tile_close", path));
                    let response = egui::Modal::new(modal_id)
                        .area(egui::Modal::default_area(modal_id).anchor(
                            egui::Align2::CENTER_CENTER,
                            area.center() - ui.ctx().screen_rect().center(),
                        ))
                        .show(ui.ctx(), |ui| {
                            ui.set_width(400.);
                            ui.vertical_centered(|ui| {
                                ui.heading(tr("像素 Tile"));
                            });
                            ui.separator();
                            ui.add_space(8.);
                            ui.vertical_centered(|ui| {
                                ui.add(egui::Label::new(tr("有未保存的绘制，是否关闭？")).wrap());
                            });
                            ui.add_space(18.);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.;
                                let width = (ui.available_width() - 12.) / 2.;
                                if ui
                                    .add_sized([width, 40.], egui::Button::new(tr("继续编辑")))
                                    .clicked()
                                {
                                    self.confirm_close = false;
                                }
                                if ui
                                    .add_sized(
                                        [width, 40.],
                                        egui::Button::new(tr("放弃修改并关闭")),
                                    )
                                    .clicked()
                                {
                                    *self = Self::default();
                                }
                            });
                            ui.add_space(6.);
                        });
                    if response.should_close() {
                        self.confirm_close = false;
                    }
                }
            });
        if self.open && self.preview_detached {
            let mut preview_open = true;
            crate::native_window::Window::new(format!("{} · {}", tr("循环预览"), title))
                .id(egui::Id::new(("pixel_tile_preview", path)))
                .open(&mut preview_open)
                .default_width(1000.)
                .default_height(800.)
                .min_width(300.)
                .min_height(220.)
                .scrollable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("预览缩放"));
                        ui.add(
                            egui::DragValue::new(&mut self.repeat_zoom)
                                .range(0.25..=8.)
                                .speed(0.1),
                        );
                        ui.checkbox(&mut self.preview_grid, tr("网格"));
                    });
                    self.preview(ui, false);
                });
            self.preview_detached = preview_open;
        }
        if revision != self.revision
            || view_state != (self.preview_detached, self.repeat_zoom, self.preview_grid)
        {
            ctx.request_repaint();
            ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile", path))));
            ctx.request_repaint_of(egui::ViewportId(egui::Id::new((
                "pixel_tile_preview",
                path,
            ))));
        }
        if !open {
            self.finish_stroke();
            if self.changed {
                self.confirm_close = true;
                ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile", path))));
            } else {
                self.open = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layer_eye_click_changes_visible_composite_without_changing_pixels() {
        fn frame(ctx: &egui::Context, e: &mut Editor, events: Vec<egui::Event>) -> egui::Id {
            let mut id = egui::Id::NULL;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(270., 600.))),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        id = ui.id().with("tile_layers_owner").with(("visible", 1usize));
                        e.layer_controls(ui);
                    });
                },
            );
            id
        }
        let ctx = egui::Context::default();
        let mut e = Editor::default();
        e.ensure_layers();
        e.write_pixel(0, 64, 32, Rgba([20, 180, 50, 255]));
        frame(&ctx, &mut e, vec![]);
        let id = frame(&ctx, &mut e, vec![]);
        let pos = ctx.read_response(id).unwrap().rect.center();
        for visible in [false, true] {
            frame(
                &ctx,
                &mut e,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            frame(
                &ctx,
                &mut e,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(e.layers[1].visible, visible);
            assert_eq!(
                e.composite(0).get_pixel(64, 32)[3],
                if visible { 255 } else { 0 }
            );
            assert_eq!(e.pixels.get_pixel(64, 32).0, [20, 180, 50, 255]);
            assert!(!e.layers[1].locked);
        }
    }
    #[test]
    fn layer_lock_has_an_unclipped_click_target_and_toggles_both_ways() {
        fn frame(
            ctx: &egui::Context,
            e: &mut Editor,
            events: Vec<egui::Event>,
        ) -> (egui::Id, Rect) {
            let mut id = egui::Id::NULL;
            let mut clip = Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(270., 600.))),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        id = ui.id().with("tile_layers_owner").with(("lock", 0usize));
                        clip = ui.clip_rect();
                        e.layer_controls(ui);
                    });
                },
            );
            (id, clip)
        }
        let ctx = egui::Context::default();
        let mut e = Editor::default();
        e.ensure_layers();
        e.layers[0].name =
            "A deliberately long layer name must never push the lock outside the sidebar".into();
        frame(&ctx, &mut e, vec![]);
        let (id, clip) = frame(&ctx, &mut e, vec![]);
        let rect = ctx.read_response(id).unwrap().rect;
        assert!(
            clip.contains_rect(rect),
            "lock must fit inside the sidebar: {rect:?} / {clip:?}"
        );
        assert_eq!(rect.size(), egui::vec2(28., 28.));
        for locked in [false, true] {
            let pos = rect.center();
            frame(
                &ctx,
                &mut e,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            frame(
                &ctx,
                &mut e,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(e.layers[0].locked, locked);
        }
    }
    #[test]
    fn straight_line_uses_brush_width_wraps_and_undoes_as_one_stroke() {
        let mut e = Editor::default();
        e.diamond = false;
        e.tool = Tool::Line;
        e.rectangle_input(Some((60, 0)), true, true, false, false);
        e.rectangle_input(Some((68, 0)), false, false, false, false);
        assert_eq!(e.undo.len(), 1);
        for x in [124, 125, 126, 127, 0, 1, 2, 3, 4] {
            assert_eq!(e.pixels.get_pixel(x, 32).0, e.color);
        }
        assert_eq!(e.pixels.pixels().filter(|p| p[3] > 0).count(), 9);
        e.history(false);
        assert!(e.pixels.pixels().all(|p| p[3] == 0));
        e.history(true);
        assert_eq!(e.pixels.pixels().filter(|p| p[3] > 0).count(), 9);
    }
    #[test]
    fn layers_preserve_base_lock_alpha_and_erasure_with_undo() {
        let mut e = Editor::default();
        e.replace(RgbaImage::from_pixel(128, 64, Rgba([20, 40, 60, 255])));
        e.ensure_layers();
        assert!(e.layers[0].locked);
        assert_eq!(e.active_layer, 1);
        e.write_pixel(0, 64, 32, Rgba([220, 40, 60, 128]));
        assert_eq!(e.layers[0].tiles[0].get_pixel(64, 32).0, [20, 40, 60, 255]);
        let c = e.composite(0).get_pixel(64, 32).0;
        assert!((120..=121).contains(&c[0]));
        assert_eq!(&c[1..], &[40, 60, 255]);
        e.select_layer(0);
        e.write_pixel(0, 64, 32, Rgba([0; 4]));
        assert_eq!(e.pixels.get_pixel(64, 32).0, [20, 40, 60, 255]);
        e.select_layer(1);
        e.tool = Tool::Eraser;
        e.plane_stroke((0, 0), (0, 0), false);
        e.finish_stroke();
        assert_eq!(e.composite(0).get_pixel(64, 32).0, [20, 40, 60, 255]);
        e.history(false);
        assert_eq!(e.composite(0).get_pixel(64, 32).0, c);
        e.layers[1].visible = false;
        assert_eq!(e.composite(0).get_pixel(64, 32).0, [20, 40, 60, 255]);
    }
    #[test]
    fn layered_grid_exports_individual_composites_and_validates_storage() {
        let mut e = Editor::default();
        e.ensure_layers();
        e.columns = 3;
        e.rows = 3;
        e.multiple = true;
        e.independent_slots().unwrap();
        for row in 0..3 {
            for col in 0..3 {
                e.write_pixel(
                    e.layout[row * 8 + col],
                    64,
                    32,
                    Rgba([(row * 3 + col + 1) as u8, 30, 50, 255]),
                );
            }
        }
        let mut count = 0;
        for row in 0..3 {
            for col in 0..3 {
                let image = e.export_cell(e.layout[row * 8 + col]);
                assert_eq!(image.dimensions(), (128, 64));
                assert_eq!(image.get_pixel(64, 32)[0], (row * 3 + col + 1) as u8);
                assert_eq!(image.get_pixel(0, 0)[3], 0);
                count += 1;
            }
        }
        assert_eq!(count, 9);
        let state = e.snapshot();
        let project = Project {
            version: 3,
            background_custom: false,
            background_color: default_background(),
            width: 128,
            height: 64,
            layers: state
                .layers
                .iter()
                .map(|l| StoredLayer {
                    name: l.name.clone(),
                    visible: l.visible,
                    locked: l.locked,
                    tiles: l.tiles.iter().map(|p| p.as_raw().clone()).collect(),
                })
                .collect(),
            active_layer: state.active_layer,
            tiles: state.tiles.iter().map(|p| p.as_raw().clone()).collect(),
            selected: e.selected,
            layout: e.layout.clone(),
            columns: 3,
            rows: 3,
            multiple: true,
            diamond: true,
            elevated: false,
            footprint: [128, 64],
            wrap: true,
            edit_unit: true,
            brush_size: 1,
        };
        let bytes = serde_json::to_vec(&project).unwrap();
        let mut restored: Project = serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();
        assert!(restored.layers[0].locked);
        assert_eq!(restored.layers[1].tiles[8][(32 * 128 + 64) * 4], 9);
        restored.layers[1].tiles.pop();
        assert!(restored.validate().is_err());
    }
    #[test]
    fn coordinate_slots_edit_independently_and_reject_over_budget_without_changes() {
        let mut e = Editor::default();
        e.multiple = true;
        e.columns = 2;
        e.rows = 2;
        e.independent_slots().unwrap();
        let ids = [e.layout[0], e.layout[1], e.layout[8], e.layout[9]];
        assert_eq!(
            ids.into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        e.select(e.layout[9]);
        let image = RgbaImage::from_pixel(128, 64, Rgba([12, 34, 56, 78]));
        e.replace(image.clone());
        assert_eq!(e.tiles[e.layout[9]], image);
        for slot in [0, 1, 8] {
            assert!(e.tiles[e.layout[slot]].pixels().all(|p| p.0 == [0; 4]));
        }
        e.replace(RgbaImage::new(128, 64));
        e.history(false);
        assert_eq!(e.pixels, image);
        e.history(true);
        assert!(e.pixels.pixels().all(|p| p.0 == [0; 4]));
        let mut large = Editor::default();
        large.pixels = RgbaImage::new(1024, 1024);
        large.tiles = vec![large.pixels.clone()];
        large.columns = 3;
        large.rows = 2;
        let before = large.snapshot();
        assert!(large.independent_slots().is_err());
        assert!(large.snapshot() == before);
    }
    #[test]
    fn diamond_lattice_has_exactly_one_owner() {
        for (w, h) in [(128, 64), (8, 8), (6, 4), (2, 2)] {
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let owners = (-3i32..=3)
                        .flat_map(|r| {
                            (-3i32..=3).map(move |c| {
                                (
                                    c * w as i32 + r.rem_euclid(2) * w as i32 / 2,
                                    r * h as i32 / 2,
                                )
                            })
                        })
                        .filter(|(ox, oy)| inside(x - ox, y - oy, w, h, true))
                        .count();
                    assert_eq!(owners, 1, "{w}x{h} at {x},{y}");
                }
            }
        }
    }
    #[test]
    fn continuous_strokes_and_history() {
        let mut e = Editor::default();
        e.before = Some(e.snapshot());
        line((64, 24), (70, 28), |x, y| {
            e.pixels.put_pixel(x as u32, y as u32, Rgba([255; 4]))
        });
        let painted = e.pixels.clone();
        e.finish_stroke();
        e.history(false);
        assert!(e.pixels.pixels().all(|p| p.0 == [0; 4]));
        e.history(true);
        assert_eq!(e.pixels, painted);
    }
    #[test]
    fn cross_cell_mapping_includes_negative_coordinates() {
        for (w, h) in [(128, 64), (8, 8), (6, 4), (2, 2)] {
            for y in -(h as i32)..h as i32 {
                for x in -(w as i32)..w as i32 {
                    let world = egui::vec2(x as f32 + 0.5, y as f32 + 0.5);
                    let (col, row) = cell_at(world, [w, h], true);
                    let px = x - (col - row) * w as i32 / 2 + w as i32 / 2;
                    let py = y - (col + row) * h as i32 / 2 + h as i32 / 2;
                    assert!(
                        inside(px, py, w, h, true),
                        "{w}x{h}: {x},{y} -> {col},{row}: {px},{py}"
                    );
                }
            }
        }
    }
    #[test]
    fn one_stroke_undo_restores_all_tiles() {
        let mut e = Editor::default();
        e.tiles.push(e.pixels.clone());
        e.before = Some(e.snapshot());
        e.pixels.put_pixel(64, 32, Rgba([255; 4]));
        e.tiles[1].put_pixel(64, 32, Rgba([128; 4]));
        e.finish_stroke();
        e.history(false);
        assert_eq!(e.pixels.get_pixel(64, 32).0, [0; 4]);
        assert_eq!(e.tiles[1].get_pixel(64, 32).0, [0; 4]);
        e.history(true);
        assert_eq!(e.pixels.get_pixel(64, 32).0, [255; 4]);
        assert_eq!(e.tiles[1].get_pixel(64, 32).0, [128; 4]);
    }
    fn tiny(columns: usize, rows: usize) -> Editor {
        let mut e = Editor::default();
        e.replace(RgbaImage::new(8, 4));
        e.undo.clear();
        e.multiple = columns * rows > 1;
        e.columns = columns;
        e.rows = rows;
        e.tiles = vec![RgbaImage::new(8, 4); columns * rows];
        for r in 0..rows {
            for c in 0..columns {
                e.layout[r * 8 + c] = r * columns + c;
            }
        }
        e
    }
    #[test]
    fn integer_ownership_and_periodicity_are_exact() {
        for w in [2, 4, 6, 10, 128] {
            for h in [2, 4, 6, 64] {
                for y in -(h as i32)..h as i32 {
                    for x in -(w as i32)..w as i32 {
                        let p = cell_pixel(x, y, w, h, true);
                        assert!(inside(p.x as i32, p.y as i32, w, h, true));
                        for (dc, dr) in [(3, 0), (0, 5), (-3, -5)] {
                            let q = cell_pixel(
                                x + (dc - dr) * w as i32 / 2,
                                y + (dc + dr) * h as i32 / 2,
                                w,
                                h,
                                true,
                            );
                            assert_eq!((q.x, q.y), (p.x, p.y));
                            assert_eq!((q.col, q.row), (p.col + dc, p.row + dr));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn whole_brush_wraps_to_opposite_edge_and_eraser_undo_works() {
        let mut e = tiny(1, 1);
        e.brush_size = 3;
        e.plane_stroke((2, 0), (2, 0), false);
        e.finish_stroke();
        assert_eq!(e.pixels.get_pixel(6, 2).0, e.color);
        assert_eq!(e.pixels.get_pixel(3, 0).0, e.color);
        assert!(
            e.pixels
                .enumerate_pixels()
                .all(|(x, y, p)| inside(x as i32, y as i32, 8, 4, true) || p.0 == [0; 4])
        );
        let painted = e.snapshot();
        e.tool = Tool::Eraser;
        e.plane_stroke((2, 0), (2, 0), false);
        e.finish_stroke();
        assert!(e.pixels.pixels().all(|p| p.0 == [0; 4]));
        e.history(false);
        assert!(e.snapshot() == painted);
        let mut clipped = tiny(1, 1);
        clipped.wrap = false;
        clipped.brush_size = 3;
        clipped.plane_stroke((2, 0), (2, 0), false);
        clipped.finish_stroke();
        assert_eq!(clipped.pixels.get_pixel(3, 0).0, [0; 4]);
    }
    #[test]
    fn multi_cell_outer_edge_wraps_into_correct_cell_in_one_undo() {
        let mut e = tiny(3, 3);
        let blank = e.snapshot();
        e.brush_size = 3;
        e.plane_stroke((10, 4), (10, 4), true);
        e.finish_stroke();
        assert_eq!(e.undo.len(), 1);
        assert_eq!(e.tiles[2].get_pixel(6, 2).0, e.color);
        assert_eq!(e.tiles[0].get_pixel(3, 0).0, e.color);
        let painted = e.snapshot();
        e.history(false);
        assert!(e.snapshot() == blank);
        e.history(true);
        assert!(e.snapshot() == painted);
        for y in -12..12 {
            for x in -24..24 {
                assert_eq!(e.plane_hit(x, y, true), e.plane_hit(x + 12, y + 6, true));
                assert_eq!(e.plane_hit(x, y, true), e.plane_hit(x - 12, y + 6, true));
            }
        }
    }
    #[test]
    fn detached_preview_never_finishes_an_in_progress_stroke() {
        let mut e = tiny(1, 1);
        e.plane_stroke((2, 0), (2, 0), false);
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| e.preview(ui, false));
        });
        assert!(e.before.is_some());
        assert!(e.undo.is_empty());
    }
    #[test]
    fn rectangle_then_fill_is_one_undo_and_open_regions_are_rejected() {
        let mut e = Editor::default();
        e.diamond = false;
        e.tool = Tool::Rectangle;
        e.rectangle_input(Some((-4, -4)), true, true, false, false);
        e.rectangle_input(Some((4, 4)), true, true, false, false);
        e.rectangle_input(Some((4, 4)), false, false, false, false);
        assert_eq!(e.undo.len(), 1);
        let ring = e.snapshot();
        e.color = [200, 20, 30, 255];
        e.tool = Tool::Fill;
        e.fill_closed((0, 0), false);
        assert_eq!(e.pixels.get_pixel(64, 32).0, e.color);
        assert_eq!(e.undo.len(), 2);
        assert_eq!(e.pixels.get_pixel(70, 32).0, [0; 4]);
        e.history(false);
        assert!(e.snapshot() == ring);
        e.history(true);
        assert_eq!(e.pixels.get_pixel(64, 32).0, e.color);
        e.history(false);
        e.tool = Tool::Eraser;
        e.plane_stroke((-4, 0), (-4, 0), false);
        e.finish_stroke();
        let open = e.snapshot();
        let history = e.undo.len();
        e.tool = Tool::Fill;
        e.fill_closed((0, 0), false);
        assert!(e.snapshot() == open);
        assert_eq!(e.undo.len(), history);
        assert!(!e.message.is_empty());
    }
    #[test]
    fn tool_and_history_keys_do_not_override_text_editing() {
        let mut e = Editor::default();
        let ctx = egui::Context::default();
        let event = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        for (key, tool) in [
            (egui::Key::I, Tool::Eyedropper),
            (egui::Key::E, Tool::Eraser),
            (egui::Key::U, Tool::Rectangle),
            (egui::Key::L, Tool::Line),
            (egui::Key::G, Tool::Fill),
            (egui::Key::B, Tool::Brush),
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    focused: true,
                    events: vec![event(key, egui::Modifiers::NONE)],
                    ..Default::default()
                },
                |ctx| e.tool_shortcuts(ctx),
            );
            assert!(e.tool == tool);
        }
        e.plane_stroke((0, 0), (0, 0), false);
        e.finish_stroke();
        for (key, painted) in [(egui::Key::Z, false), (egui::Key::Y, true)] {
            let _ = ctx.run(
                egui::RawInput {
                    focused: true,
                    modifiers: egui::Modifiers::CTRL,
                    events: vec![event(key, egui::Modifiers::CTRL)],
                    ..Default::default()
                },
                |ctx| e.tool_shortcuts(ctx),
            );
            assert_eq!(!e.undo.is_empty(), painted);
        }
        let mut text = String::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text))
                    .request_focus();
            });
        });
        let _ = ctx.run(
            egui::RawInput {
                focused: true,
                events: vec![event(egui::Key::I, egui::Modifiers::NONE)],
                ..Default::default()
            },
            |ctx| e.tool_shortcuts(ctx),
        );
        assert!(e.tool == Tool::Brush);
    }
    #[test]
    fn legacy_projects_default_to_wrap_and_new_projects_keep_brush_settings() {
        let mut value = serde_json::json!({"version":1,"width":8,"height":4,"tiles":[vec![0u8;128]],"selected":0,"layout":vec![0usize;64],"columns":1,"rows":1,"multiple":false,"diamond":true,"elevated":false,"footprint":[8,4]});
        let legacy: Project = serde_json::from_value(value.clone()).unwrap();
        legacy.validate().unwrap();
        assert!(legacy.wrap);
        assert!(legacy.edit_unit);
        assert_eq!(legacy.brush_size, 1);
        value["version"] = 2.into();
        value["wrap"] = false.into();
        value["edit_unit"] = false.into();
        value["brush_size"] = 32.into();
        let project: Project = serde_json::from_value(value).unwrap();
        project.validate().unwrap();
        let encoded = serde_json::to_vec(&project).unwrap();
        let loaded: Project = serde_json::from_slice(&encoded).unwrap();
        assert!(!loaded.wrap);
        assert!(!loaded.edit_unit);
        assert_eq!(loaded.brush_size, 32);
    }
}
