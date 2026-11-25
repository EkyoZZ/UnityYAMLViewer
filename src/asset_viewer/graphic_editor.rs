//! Single-layer Tile sets, lattice-mapped painting and live raised/flat repeats.
use crate::localization::tr;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use image::{Rgba, RgbaImage};
use std::path::Path;
mod ps_plugin;
mod materials;
mod pressure;
mod selection;
mod subgroup;
mod brush_stroke;
mod brush_presets;
mod layer_properties;
mod curve_editor;
mod levels_editor;
mod layer_groups;
mod layer_move;
mod layer_drop;
mod split;
mod png_export;
mod image_size;
mod guides;
mod vector;
mod text_layer;
mod transform;
pub(super) mod bitmap_workspace;
pub(super) mod project_preview;
use materials::{Material, StoredMaterial};

// Pixel-center sampling; tiny hard pencil tips remain square.
fn pencil_tip_contains(size: i32, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= size || y >= size {
        return false;
    }
    let dx = 2 * x + 1 - size;
    let dy = 2 * y + 1 - size;
    size <= 3 || dx * dx + dy * dy <= size * size
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum BrushMode { #[default] Normal, Multiply, Screen, Overlay, Darken, Lighten, Difference }
impl BrushMode {
    const ALL: [Self; 7] = [Self::Normal, Self::Multiply, Self::Screen, Self::Overlay, Self::Darken, Self::Lighten, Self::Difference];
    fn label(self) -> &'static str {
        match self { Self::Normal=>"正常", Self::Multiply=>"正片叠底", Self::Screen=>"滤色", Self::Overlay=>"叠加", Self::Darken=>"变暗", Self::Lighten=>"变亮", Self::Difference=>"差值" }
    }
    fn blend(self, base: f32, paint: f32) -> f32 {
        match self {
            Self::Normal => paint,
            Self::Multiply => base * paint,
            Self::Screen => 1. - (1. - base) * (1. - paint),
            Self::Overlay => if base <= 0.5 { 2. * base * paint } else { 1. - 2. * (1. - base) * (1. - paint) },
            Self::Darken => base.min(paint),
            Self::Lighten => base.max(paint),
            Self::Difference => (base - paint).abs(),
        }
    }
}

fn round_brush_blend(old: Rgba<u8>, color: Rgba<u8>, coverage: f32, erase: bool, mode: BrushMode) -> Rgba<u8> {
    let amount = coverage * f32::from(color[3]) / 255.;
    let old_alpha = f32::from(old[3]) / 255.;
    let alpha = if erase { old_alpha * (1. - amount) } else { amount + old_alpha * (1. - amount) };
    if alpha <= 0. { return Rgba([0; 4]); }
    let mut result = old;
    if !erase {
        for channel in 0..3 {
            let base = f32::from(old[channel]) / 255.;
            let paint = f32::from(color[channel]) / 255.;
            // Blend only where both pixels have coverage; transparent backing keeps paint RGB.
            let mixed = (1. - old_alpha) * paint + old_alpha * mode.blend(base, paint);
            result[channel] = ((mixed * amount + base * old_alpha * (1. - amount)) / alpha * 255.).round() as u8;
        }
    }
    result[3] = (alpha * 255.).round() as u8;
    result
}

fn round_tip_coverage(radius: f32, hardness: u8) -> f32 {
    if radius > 1. { return 0.; }
    let hard = f32::from(hardness.min(100)) / 100.;
    if radius <= hard || hard >= 1. { return 1.; }
    let distance = ((radius - hard) / (1. - hard)).clamp(0., 1.);
    let edge = (-4.5_f32).exp();
    ((-4.5 * distance * distance).exp() - edge) / (1. - edge)
}

#[cfg(test)]
fn brush_tip_coverage(size: i32, x: i32, y: i32, hardness: u8) -> f32 {
    let mut coverage = 0.;
    for sy in 0..4 {
        for sx in 0..4 {
            let cx = (x as f32 + (sx as f32 + 0.5) / 4.) * 2. / size as f32 - 1.;
            let cy = (y as f32 + (sy as f32 + 0.5) / 4.) * 2. / size as f32 - 1.;
            coverage += round_tip_coverage((cx * cx + cy * cy).sqrt(), hardness);
        }
    }
    coverage / 16.
}

// Keep straight RGBA in project/export data; premultiply only for egui display.
fn tile_preview_image(image: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage {
        size: [image.width() as usize, image.height() as usize],
        pixels: image.pixels().map(|pixel| super::preview_color(pixel.0)).collect(),
    }
}

const MAX_SIDE: u32 = 1024;
const HISTORY_BYTES: usize = 32 * 1024 * 1024;
const MAX_TILES: usize = 64;
const LAYER_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, PartialEq)]
struct Layer {
    settings:layer_properties::Properties,
    material: bool,
    name: String,
    visible: bool,
    locked: bool,
    tiles: Vec<RgbaImage>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredLayer {
    #[serde(default)]
    settings:layer_properties::Properties,
    #[serde(default)]
    material: bool,
    name: String,
    visible: bool,
    locked: bool,
    tiles: Vec<Vec<u8>>,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tool {
    #[default]
    Move,
    Zoom,
    Brush,
    Eraser,
    Eyedropper,
    Select,
    Rectangle,
    Line,
    Fill,
    Text,
    Shape,
    Path,
    Pen,
}
#[derive(Clone, Copy)]
struct RectangleDrag {
    start: (i32, i32),
    end: (i32, i32),
    layout: bool,
    preview: bool,
    line: bool,
    filled:bool,
    ellipse: bool,
    soft: bool,
}
const FRAME_MODES: [(bool, &str, crate::icons::Icon); 2] = [
    (false, "矩形", crate::icons::Icon::RectangleTool),
    (true, "椭圆", crate::icons::Icon::SelectionEllipse),
];
fn diamond_size(size: [f64; 2], footprint: [u32; 2]) -> [f64; 2] {
    let [w, h] = footprint.map(|value| f64::from(value.max(1)));
    let factor = (size[0] * w + size[1] * h) / (w * w + h * h);
    let shorter = (factor * w.min(h)).round().max(1.);
    let scale = shorter / w.min(h);
    [(scale * w).round(), (scale * h).round()]
}
fn default_brush_flow()->u8 {100}
fn default_brush_spacing()->u8 {10}
fn default_subgroup_size()->[u32;2] {[1,1]}
fn default_background() -> [u8; 3] {
    [85, 85, 85]
}
fn enabled() -> bool {
    true
}
fn one() -> u32 {
    1
}

#[derive(Clone, PartialEq)]
struct Snapshot {
    selected_slot: usize,
    subgroup_size: [u32; 2],
    fx_light: layer_properties::effects::Light,
    selection:selection::HistorySelection,
    selected_layers:Vec<usize>,
    layer_anchor:usize,
    mask_editing:bool,
    materials: Vec<Material>,
    layers: Vec<Layer>,
    active_layer: usize,
    tiles: Vec<RgbaImage>,
    selected: usize,
    layout: Vec<usize>,
    columns: usize,
    rows: usize,
    multiple: bool,
    diamond: bool,
    hex: u8,
    elevated: bool,
    footprint: [u32; 2],
    wrap: bool,
}
impl Snapshot {
    fn bytes(&self) -> usize {
        self.layers.iter().map(|l|l.settings.bytes() + l.settings.effects.extra_bytes()).sum::<usize>() + self.selection.bytes() + self.materials.iter().map(|m| m.pixels.as_raw().len()).sum::<usize>()
            + self.tiles.iter().map(|p| p.as_raw().len()).sum::<usize>()
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
    fx_light: layer_properties::effects::Light,
    #[serde(default)]
    materials: Vec<StoredMaterial>,
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
    #[serde(default)]
    hex: u8,
    elevated: bool,
    footprint: [u32; 2],
    #[serde(default = "enabled")]
    wrap: bool,
    #[serde(default = "enabled")]
    edit_unit: bool,
    #[serde(default = "one")]
    brush_size: u32,
    #[serde(default)]
    soft_brush: bool,
    #[serde(default)]
    rectangle_fill:bool,
    #[serde(default)]
    brush_hardness: u8,
    #[serde(default)]
    brush_mode: BrushMode,
    #[serde(default)]
    pressure_size: bool,
    #[serde(default)]
    pressure_opacity: bool,
    #[serde(default)]
    pressure_min_size: u8,
    #[serde(default)]
    subgroup_enabled: bool,
    #[serde(default = "default_subgroup_size")]
    subgroup_size: [u32;2],
    #[serde(default="default_brush_flow")]
    brush_flow:u8,
    #[serde(default="default_brush_spacing")]
    brush_spacing:u8,
    #[serde(default)]
    pressure_flow:bool,
}
// Horizontal pen drag is measured from the initial contact, independent of frame rate.
fn drag_zoom(anchor: Pos2, initial: f32, pan: Vec2, dx: f32, center: Pos2, minimum:f32) -> (f32, Vec2) {
    let zoom = (initial * (dx * 0.01).clamp(-20., 20.).exp()).clamp(minimum, 64.);
    let offset = anchor - center;
    (zoom, offset - (offset - pan) * (zoom / initial))
}

fn click_zoom(current: f32, out: bool, maximum: f32) -> f32 {
    const BELOW_100: [f32; 7] = [2. / 3., 1. / 2., 1. / 3., 1. / 4., 1. / 6., 1. / 8., 1. / 12.];
    const EPSILON: f32 = 0.0001;
    if out {
        if current > 1. + EPSILON {
            return (current - EPSILON).ceil() - 1.;
        }
        return BELOW_100.into_iter()
            .find(|step| *step < current - EPSILON)
            .unwrap_or(current);
    }
    if current < 1. - EPSILON {
        return BELOW_100.into_iter().rev()
            .find(|step| *step > current + EPSILON)
            .unwrap_or(1.);
    }
    ((current + EPSILON).floor() + 1.).min(maximum)
}

fn paint_zoom_cursor(ui: &egui::Ui, pointer: Pos2, out: bool) {
    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.id().with("zoom_cursor")));
    for (width, color) in [(3_f32, Color32::BLACK), (1.4_f32, Color32::WHITE)] {
        let stroke = egui::Stroke::new(width, color);
        painter.circle_stroke(pointer, 6., stroke);
        painter.line_segment([pointer + egui::vec2(4., 4.), pointer + egui::vec2(9., 9.)], stroke);
        painter.line_segment([pointer - egui::vec2(3., 0.), pointer + egui::vec2(3., 0.)], stroke);
        if !out {
            painter.line_segment([pointer - egui::vec2(0., 3.), pointer + egui::vec2(0., 3.)], stroke);
        }
    }
}
// Use the same effective scale for rendering and anchored drag zoom, including
// the minimum tile size and raised-sprite draw-budget floor.
fn repeat_scale(zoom: f32, dpi: f32, footprint: [u32; 2], height: u32) -> f32 {
    (zoom / dpi).max(12. / footprint[0].min(footprint[1]) as f32)
        .max(height as f32 / footprint[1] as f32 / 8.)
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
        let valid = (1..=12).contains(&self.version) && self.fx_light.valid()
            && self.hex <= 2 && !(self.hex != 0 && (self.elevated || self.diamond))
            && (self.hex != 1 || self.height % 4 == 0)
            && (self.hex != 2 || self.width % 4 == 0)
            && (self.hex == 0 || self.version >= 4)
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
        let layer_valid = (self.version >= 5 || (self.materials.is_empty() && self.layers.iter().all(|l|!l.material)))
            && layer_groups::valid_tree(self.layers.iter().map(|l|(&l.settings,l.material)))
            && self.layers.iter().filter(|l| l.material).count() <= 1
            && self.layers.iter().enumerate().all(|(i,l)| !l.material || (i == 0 && l.locked))
            && self.materials.len() <= 64
            && self.materials.iter().all(|m| m.name.chars().count() <= 64 && m.pixels.len() == self.width as usize*self.height as usize*4)
            && self.materials.iter().map(|m| m.pixels.len()).sum::<usize>() <= 16*1024*1024
            && self.layers.len() <= 32
            && (self.layers.is_empty() || self.active_layer < self.layers.len())
            && self.layers.iter().all(|l| {
                l.tiles.len() == self.tiles.len()
                    && l.settings.valid(l.tiles.len(),self.width as usize*self.height as usize)
                    && (l.settings.text.is_none() || (self.version >= 12 && !l.material))
                    && (self.version >= 11 || !l.settings.effects.has_extra())
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
        let mask_bytes:usize=self.layers.iter().map(|l|l.settings.bytes()).sum();
        let image_bytes:usize=self.layers.iter().flat_map(|l|&l.tiles).map(Vec::len).sum();
        if valid && layer_valid && image_bytes.saturating_add(mask_bytes)<=LAYER_BYTES {
            Ok(())
        } else {
            Err(tr("无效的 Tile 工程或超出尺寸限制。").into())
        }
    }
}

fn toolbar_offset_inside(offset:Vec2,size:Vec2,area:Rect)->Vec2 {
    let max=(area.size()-size).max(Vec2::ZERO);
    egui::vec2(if offset.x.is_finite(){offset.x.clamp(0.,max.x)}else{4_f32.min(max.x)},
        if offset.y.is_finite(){offset.y.clamp(0.,max.y)}else{4_f32.min(max.y)})
}
#[derive(serde::Serialize,serde::Deserialize)]
struct ToolbarSettings { offset:[f32;2], #[serde(default)] horizontal:bool }
fn editor_confirmation(
    ui: &egui::Ui,
    modal: egui::Modal,
    title: &str,
    message: &str,
    cancel: &str,
    confirm: &str,
    details: impl FnOnce(&mut egui::Ui),
) -> Option<bool> {
    let (decision, dismissed) =
        editor_confirmation_response(ui, modal, title, message, cancel, confirm, true, details);
    if dismissed { Some(false) } else { decision }
}

fn editor_confirmation_response(
    ui: &egui::Ui,
    modal: egui::Modal,
    title: &str,
    message: &str,
    cancel: &str,
    confirm: &str,
    confirm_enabled: bool,
    details: impl FnOnce(&mut egui::Ui),
) -> (Option<bool>, bool) {
    let mut decision = None;
    let modal = modal.show(ui.ctx(), |ui| {
        ui.set_width(286.);
        ui.vertical_centered(|ui| {
            ui.heading(title);
            ui.add(egui::Label::new(message).wrap().halign(egui::Align::Center));
            details(ui);
            ui.separator();
            let width_for = |label| ui.painter().layout_no_wrap(
                label, egui::FontId::proportional(14.), ui.visuals().text_color(),
            ).size().x + 16.;
            let max_button = (ui.available_width() - ui.spacing().item_spacing.x) / 2.;
            let button_size = egui::vec2(width_for(cancel.to_owned()).max(width_for(confirm.to_owned())).max(84.).min(max_button), 30.);
            let width = 2. * button_size.x + ui.spacing().item_spacing.x;
            ui.allocate_ui_with_layout(
                egui::vec2(width, button_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    if ui.add_sized(button_size, egui::Button::new(cancel)).clicked() {
                        decision = Some(false);
                    }
                    if ui.add_enabled_ui(confirm_enabled, |ui| {
                        ui.add_sized(button_size, egui::Button::new(confirm))
                    }).inner.clicked() {
                        decision = Some(true);
                    }
                },
            );
        });
    });
    (decision, modal.should_close())
}

fn tool_mode_button(ui:&mut egui::Ui,icon:crate::icons::Icon,label:&str,selected:bool)->egui::Response {
    let font=egui::TextStyle::Button.resolve(ui.style());let galley=ui.painter().layout_no_wrap(label.to_owned(),font,ui.visuals().text_color());
    let width=(galley.size().x+36.).max(ui.available_width());
    let response=ui.add_sized([width,26.],egui::Button::new("").selected(selected));
    let rect=Rect::from_center_size(egui::pos2(response.rect.left()+14.,response.rect.center().y),Vec2::splat(20.));
    crate::icons::paint(ui,icon,rect,ui.visuals().text_color());
    ui.painter().galley(egui::pos2(response.rect.left()+28.,response.rect.center().y-galley.size().y/2.),galley,ui.visuals().text_color());
    response
}
fn tool_popup_should_close(ctx:&egui::Context,trigger_clicked:bool,menu_contains_pointer:bool)->bool {
    ctx.input(|i|i.key_pressed(egui::Key::Escape) || (i.pointer.any_click() && !trigger_clicked && !menu_contains_pointer))
}
fn tool_popup_position(button:Rect,area:Rect,size:Vec2,horizontal:bool)->Pos2 {
    let gap=4.;
    let preferred=if horizontal {
        let below=area.bottom()-button.bottom()-gap;let above=button.top()-area.top()-gap;
        egui::pos2(button.left(),if below>=size.y || below>=above {button.bottom()+gap}else{button.top()-size.y-gap})
    }else{
        let right=area.right()-button.right()-gap;let left=button.left()-area.left()-gap;
        egui::pos2(if right>=size.x || right>=left {button.right()+gap}else{button.left()-size.x-gap},button.top())
    };
    egui::pos2(preferred.x.clamp(area.left(),(area.right()-size.x).max(area.left())),preferred.y.clamp(area.top(),(area.bottom()-size.y).max(area.top())))
}
struct ToolbarPlacement { offset:Vec2,size:Vec2,loaded:bool,dragging:bool,horizontal:bool }
impl Default for ToolbarPlacement {
    fn default()->Self {Self{offset:Vec2::splat(4.),size:egui::vec2(44.,270.),loaded:false,dragging:false,horizontal:false}}
}
pub(super) struct Editor {
    image_alpha: bool,
    image_size_draft: Option<image_size::Draft>,
    split_preview: bool,
    split_grid: [usize; 2],
    plain_image:bool,
    guides: guides::Guides,
    image_target:Option<std::path::PathBuf>,
    image_version:Option<bitmap_workspace::FileVersion>,
    image_readonly:bool,
    image_source_protected:bool,
    image_saved_as_merged_psd:bool,
    psd_resources:Vec<u8>,
    psd_patterns: Vec<u8>,
    image_saved:Option<std::path::PathBuf>,
    rectangle_fill:bool,
    text_dialog:Option<text_layer::Dialog>,
    inline_text:Option<text_layer::Inline>,
    path_tool: vector::PathTool,
    font_samples:text_layer::font_samples::Cache,
    transform:Option<transform::Session>,
    transform_bounds_cache:Option<((usize,usize,bool,u64),Option<Rect>)>,
    show_transform:bool,
    confirm_transform_exit: bool,
    transform_interpolation:transform::Interpolation,
    transform_linked:bool,
    layer_detail:Option<(usize,bool,String,Pos2)>,
    style_clipboard:Option<layer_properties::effects::Effects>,
    fx_page: layer_properties::effects::Page,
    fx_light: layer_properties::effects::Light,
    fx_dragging: bool,
    detail_histogram_fresh:bool,
    selected_layers:Vec<usize>,
    layer_anchor:usize,
    materials: Vec<Material>,
    material_drop_status: Option<(bool,String)>,
    material_thumbnails: Vec<egui::TextureHandle>,
    material_thumbnail_zoom: f32,
    mask_editing:bool,
    layer_move_group:bool,
    move_auto_select:bool,
    layer_drag:Option<layer_move::Drag>,
    show_slots: bool,
    infinite_view: bool,
    rendering_merged: bool,
    merged_pan_drag: bool,
    merged_zoom_drag: Option<(Pos2,f32,Vec2,f32)>,
    toolbar: ToolbarPlacement,
    ps_plugin: ps_plugin::State,
    background_custom: bool,
    background_color: [u8; 3],
    layers: Vec<Layer>,
    active_layer: usize,
    layer_thumbnails: Vec<Option<(egui::TextureHandle, u64, usize)>>,
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
    preview_samples:Vec<RgbaImage>,
    preview_last: Option<(i32, i32)>,
    dirty_texture: bool,
    preview_inline_active: bool,
    dirty_all: bool,
    dirty_tiles: std::collections::BTreeSet<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    before: Option<Snapshot>,
    brush_coverage: std::collections::HashMap<(usize, u32, u32), f32>,
    last: Option<(i32, i32)>,
    color: [u8; 4],
    drawing_background: [u8; 4],
    tool: Tool,
    rectangle: Option<RectangleDrag>,
    rectangle_ellipse: bool,
    diamond: bool,
    hex: u8,
    elevated: bool,
    footprint: [u32; 2],
    pan: Vec2,
    pan_drag: bool,
    zoom_drag: Option<(Pos2, f32, Vec2)>,
    grid: bool,
    preview_grid: bool,
    width: u32,
    height: u32,
    zoom: f32,
    repeat_zoom: f32,
    repeat_pan: Vec2,
    repeat_pan_drag: bool,
    repeat_zoom_drag: Option<(Pos2, f32, Vec2, f32)>,
    changed: bool,
    confirm_close: bool,
    discard_requested:bool,
    pending_open:Option<std::path::PathBuf>,
    project_target: Option<(std::path::PathBuf, bitmap_workspace::FileVersion)>,
    message: String,
    wake: bool,
    wrap: bool,
    edit_unit: bool,
    brush_size: u32,
    soft_brush: bool,
    brush_hardness: u8,
    brush_mode: BrushMode,
    pressure_size: bool,
    pressure_opacity: bool,
    pressure_min_size: u8,
    pen: pressure::PenInput,
    selection: selection::Selection,
    subgroup_enabled: bool,
    subgroup_size: [u32;2],
    subgroup_touches: std::collections::HashSet<(i32,i32)>,
    brush_flow:u8,
    brush_spacing:u8,
    pressure_flow:bool,
    brush_sampler:brush_stroke::Sampler,
    input_pressure: Option<f32>,
    stroke_pressure: Option<f32>,
    preview_detached: bool,
    revision: u64,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            split_preview: false, split_grid: [1, 1],
            image_alpha: true, image_size_draft: None,
            plain_image:false,guides:Default::default(),image_target:None,image_version:None,image_readonly:false,image_source_protected:false,image_saved_as_merged_psd:false,image_saved:None,psd_resources:Vec::new(),psd_patterns:Vec::new(),
            materials: Vec::new(),
            material_drop_status: None,
            material_thumbnails: Vec::new(),
            material_thumbnail_zoom: 0.,
            text_dialog:None,
            inline_text:None,
            path_tool: Default::default(),
            font_samples:Default::default(),transform:None,transform_bounds_cache:None,show_transform:false,confirm_transform_exit:false,transform_interpolation:Default::default(),transform_linked:true,
            layer_detail:None,
            style_clipboard:None,
            fx_page: Default::default(), fx_light: Default::default(), fx_dragging: false,
            detail_histogram_fresh:false,
            selected_layers:Vec::new(),layer_anchor:0,
            mask_editing:false,
            layer_move_group:false,
            move_auto_select:false,
            layer_drag:None,
            show_slots: false,
            infinite_view: false,
            rendering_merged: false,
            merged_pan_drag: false,
            merged_zoom_drag: None,
            toolbar: Default::default(),
            ps_plugin: Default::default(),
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
            preview_samples:Vec::new(),
            preview_last: None,
            dirty_texture: true,
            preview_inline_active: false,
            dirty_all: true,
            dirty_tiles: Default::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            before: None,
            brush_coverage: Default::default(),
            last: None,
            color: [80, 180, 100, 255],
            drawing_background: [0, 0, 0, 255],
            tool: Tool::Move,
            rectangle: None,
            diamond: true,
            hex: 0,
            elevated: false,
            footprint: [128, 64],
            pan: Vec2::ZERO,
            pan_drag: false,
            zoom_drag: None,
            grid: true,
            preview_grid: true,
            width: 128,
            height: 64,
            zoom: 4.,
            repeat_zoom: 1.,
            repeat_pan: Vec2::ZERO,
            repeat_pan_drag: false,
            repeat_zoom_drag: None,
            changed: false,
            confirm_close: false,
            discard_requested:false,
            pending_open:None,
            project_target: None,
            message: String::new(),
            wake: false,
            wrap: true,
            edit_unit: true,
            brush_size: 1,
            soft_brush: false,
            rectangle_fill:false,
            rectangle_ellipse:false,
            brush_hardness: 0,
            brush_mode: BrushMode::Normal,
            pressure_size: false,
            pressure_opacity: false,
            pressure_min_size: 0,
            pen: Default::default(),
            selection: Default::default(),
            subgroup_enabled: false,
            subgroup_size: [1,1],
            subgroup_touches: Default::default(),
            brush_flow:100,
            brush_spacing:10,
            pressure_flow:false,
            brush_sampler:Default::default(),
            input_pressure: None,
            stroke_pressure: None,
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

// Pointy-top axial lattice; pixel ownership is shared by hit-testing and clipping.
fn hex_size(cols: usize, rows: usize, w: u32, h: u32, flat: bool) -> Vec2 {
    if flat { egui::vec2((1.+(cols-1) as f32*0.75)*w as f32,(rows as f32+(cols-1) as f32/2.)*h as f32) }
    else { egui::vec2((cols as f32+(rows-1) as f32/2.)*w as f32,(1.+(rows-1) as f32*0.75)*h as f32) }
}
fn hex_center(col: i32, row: i32, w: u32, h: u32, flat: bool) -> Vec2 {
    if flat { let p = hex_center(row, col, h, w, false); return egui::vec2(p.y, p.x); }
    egui::vec2((col as f32 + row as f32 / 2.) * w as f32, row as f32 * h as f32 * 0.75)
}
fn hex_pixel(x: i32, y: i32, w: u32, h: u32, flat: bool) -> CellPixel {
    if flat { let p = hex_pixel(y, x, h, w, false); return CellPixel { col: p.row, row: p.col, x: p.y, y: p.x }; }
    let px = x as f64 + 0.5; let py = y as f64 + 0.5;
    let r = (py / (h as f64 * 0.75)).floor() as i32;
    let mut best = (f64::INFINITY, 0, 0, 0, 0);
    for row in r-1..=r+1 {
        let q = (px / w as f64 - row as f64 / 2.).floor() as i32;
        for col in q-1..=q+1 {
            let cx = (col as i64 * w as i64 + row as i64 * (w/2) as i64) as i32;
            let cy = (row as i64 * (h/4*3) as i64) as i32;
            let dx = (px - cx as f64) / w as f64;
            let dy = (py - cy as f64) / h as f64;
            let distance = 3. * dx * dx + 4. * dy * dy;
            if distance < best.0 { best = (distance, col, row, cx, cy); }
        }
    }
    CellPixel { col: best.1, row: best.2, x: (x-best.3+w as i32/2) as u32, y: (y-best.4+h as i32/2) as u32 }
}
fn hex_outline(rect: Rect, flat: bool) -> Vec<Pos2> {
    let points = if flat { vec![(0.,0.5),(0.25,0.),(0.75,0.),(1.,0.5),(0.75,1.),(0.25,1.)] }
        else { vec![(0.5,0.),(1.,0.25),(1.,0.75),(0.5,1.),(0.,0.75),(0.,0.25)] };
    points.into_iter().map(|(x,y)| rect.min + egui::vec2(rect.width()*x, rect.height()*y)).collect()
}
impl Editor {
    fn mapped_cell(&self, x: i32, y: i32, w: u32, h: u32) -> CellPixel {
        if self.hex != 0 { hex_pixel(x,y,w,h,self.hex == 2) } else { cell_pixel(x,y,w,h,self.diamond) }
    }
    fn mapped_center(&self, col: i32, row: i32, w: u32, h: u32) -> Vec2 {
        if self.hex != 0 { hex_center(col,row,w,h,self.hex == 2) } else { cell_center(col as f32,row as f32,w,h,self.diamond) + if self.plain_image{egui::vec2((w%2)as f32,(h%2)as f32)*0.5}else{Vec2::ZERO} }
    }
    fn inside_tile(&self, x: i32, y: i32, w: u32, h: u32) -> bool {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 { return false; }
        if self.hex == 0 { return inside(x,y,w,h,self.diamond && !self.elevated); }
        let p = hex_pixel(x-w as i32/2,y-h as i32/2,w,h,self.hex == 2);
        p.col == 0 && p.row == 0
    }
    fn tile_outline(&self, painter: &egui::Painter, rect: Rect, color: Color32) {
        if self.hex != 0 { painter.add(egui::Shape::closed_line(hex_outline(rect,self.hex == 2),egui::Stroke::new(1_f32,color))); }
        else { Self::outline(painter,rect,self.diamond && !self.elevated,color); }
    }

    fn layer_editable(&self) -> bool {
        self.layers
            .get(self.active_layer)
            .is_none_or(|layer| !layer.locked && !self.ancestor_locked(self.active_layer) && self.ancestors_visible(self.active_layer) && layer.visible && if self.mask_editing{layer.settings.mask.is_some() && layer.settings.vector.as_ref().is_none_or(|v| v.fill.is_some())}else{layer.settings.group_id==0 && !layer.settings.lock_pixels && !self.ancestor_paint_locked(self.active_layer) && layer.settings.adjustment.is_none() && layer.settings.text.is_none() && layer.settings.vector.as_ref().is_none_or(|v| v.fill.is_none())})
    }
    fn prepare_pixel_paint(&mut self, hit: impl FnOnce(&Self) -> bool) -> bool {
        if self.layer_editable() {
            return true;
        }
        let index = self.active_layer;
        let settings = &self.layers[index].settings;
        if self.mask_editing
            || !matches!(self.tool, Tool::Brush | Tool::Rectangle | Tool::Line | Tool::Fill)
            || self.color[3] == 0
            || (settings.vector.is_none() && settings.text.is_none())
            || self.ancestor_locked(index)
            || self.ancestor_paint_locked(index)
            || !self.ancestors_visible(index)
            || !hit(self)
        {
            return false;
        }
        let before = self.snapshot();
        let pressure = self.input_pressure;
        if !self.insert_empty_layer(false) {
            return false;
        }
        self.input_pressure = pressure;
        self.before = Some(before);
        self.message.clear();
        true
    }
    fn paint_hit(&self, point: (i32, i32), layout: bool) -> bool {
        self.plane_hit(point.0, point.1, layout)
            .is_some_and(|(tile, x, y)| self.selection.allows(tile, x, y))
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
        self.layers.push(Layer { settings:Default::default(), material: false,
            name: format!("{} 1", tr("图层")), visible: true, locked: false,
            tiles: self.tiles.clone(),
        });
        self.active_layer = 0;
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
            if self.background_custom || (self.plain_image && !self.image_alpha) {
                &super::ImageBackground::Custom
            } else {
                &super::ImageBackground::Checkerboard
            },
            Some(self.background_color),
        );
    }
    fn background_controls(&mut self,ui:&mut egui::Ui){
        ui.separator();
        ui.horizontal(|ui|{
            ui.spacing_mut().item_spacing.x=3.;ui.label(tr("预览背景色"));
            ui.horizontal(|ui|{
                ui.spacing_mut().item_spacing.x=0.;
                let mut mode=if self.background_custom{1}else{0};
                for(value,label)in [(0,"棋盘格"),(1,"自定义")]{let rounding=egui::CornerRadius{nw:if value==0{4}else{0},sw:if value==0{4}else{0},ne:if value==1{4}else{0},se:if value==1{4}else{0}};if ui.add(egui::Button::new(tr(label)).selected(mode==value).corner_radius(rounding)).clicked(){mode=value;}}
                self.background_custom=mode==1;
            });
            let [r,g,b]=self.background_color;
            if let Some(color)=crate::color_property::show_editable(ui,tr("预览背景色"),[r as f64/255.,g as f64/255.,b as f64/255.,1.]){
                self.background_color=[color[0],color[1],color[2]].map(|c|(c.clamp(0.,1.)*255.).round()as u8);
                self.background_custom=true;
            }
        });
    }
    fn pick_visible_pixel(&mut self, tile: usize, x: u32, y: u32) {
        self.color = self.composite(tile).get_pixel(x, y).0;
    }
    fn drawing_colors(&mut self, ui: &mut egui::Ui, height: f32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(44., height), egui::Sense::hover());
        let top = rect.center().y - 24.;
        let reset = Rect::from_min_size(egui::pos2(rect.left() + 5., top), Vec2::splat(16.));
        let response = ui.interact(reset, ui.id().with("reset_drawing_colors"), egui::Sense::click());
        let outline = egui::Stroke::new(1_f32, ui.style().interact(&response).fg_stroke.color);
        for (offset, fill) in [(egui::vec2(2., 2.), Color32::BLACK), (egui::vec2(-2., -2.), Color32::WHITE)] {
            let square = Rect::from_center_size(reset.center() + offset, Vec2::splat(7.));
            ui.painter().rect_filled(square, 0., fill);
            ui.painter().rect_stroke(square, 0., outline, egui::StrokeKind::Inside);
        }
        if response.on_hover_text(tr("恢复默认绘制颜色")).clicked() {
            self.color = [80, 180, 100, 255];
            self.drawing_background = [0, 0, 0, 255];
        }
        for (label, value, x, y) in [
            ("背景色", &mut self.drawing_background, 19., 27.),
            ("前景色", &mut self.color, 9., 17.),
        ] {
            let swatch = Rect::from_min_size(egui::pos2(rect.left() + x, top + y), Vec2::splat(19.));
            let mut slot = ui.new_child(egui::UiBuilder::new().id_salt(label).max_rect(swatch));
            if let Some(rgba) = crate::color_property::show_editable_swatch(
                &mut slot, tr(label), value.map(|v| f64::from(v) / 255.), swatch.size(),
            ) {
                *value = rgba.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
            }
        }
        let swap = Rect::from_min_size(egui::pos2(rect.left() + 24., top), Vec2::splat(16.));
        let response = ui.interact(swap, ui.id().with("swap_drawing_colors"), egui::Sense::click());
        let color = ui.style().interact(&response).fg_stroke.color;
        let c = swap.center();
        let stroke = egui::Stroke::new(1.25_f32, color);
        let p = |x, y| c + egui::vec2(x, y);
        ui.painter().add(egui::epaint::CubicBezierShape::from_points_stroke(
            [p(-4.5, -3.), p(1.5, -3.), p(4.5, -2.), p(4.5, 4.)],
            false, Color32::TRANSPARENT, stroke,
        ));
        ui.painter().add(egui::Shape::line(vec![p(-2., -5.5), p(-4.5, -3.), p(-2., -0.5)], stroke));
        ui.painter().add(egui::Shape::line(vec![p(2., 1.5), p(4.5, 4.), p(7., 1.5)], stroke));
        if response.on_hover_text(tr("交换前景色和背景色")).clicked() {
            std::mem::swap(&mut self.color, &mut self.drawing_background);
        }
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], ui.visuals().widgets.noninteractive.bg_stroke);
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
        self.composite_group_stack(0,tile,&mut out,0);
        out
    }
    fn export_cell(&self, tile: usize) -> RgbaImage {
        let mut image = self.composite(tile);
        let (w, h) = image.dimensions();
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            if !self.inside_tile(x as i32, y as i32, w, h) {
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
        if index!=self.active_layer{self.layer_detail=None;}
        self.finish_stroke();
        self.mask_editing=false;self.layer_drag=None;
        self.sync_layer();
        self.active_layer = index;self.selected_layers=vec![index];self.layer_anchor=index;
        self.tiles = self.layers[index].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }
    fn cut_selection_to_layer(&mut self,index:usize) {
        if index>=self.layers.len() || self.layers[index].settings.text.is_some() || self.layers[index].material || self.layers[index].locked || self.layers[index].settings.lock_alpha || self.layers[index].settings.lock_pixels || self.layers[index].settings.adjustment.is_some() || self.layers[index].settings.group_id!=0 || self.ancestor_locked(index) || self.mask_editing || !self.layers[index].visible {return;}
        let Some(mask)=self.selection.mask.as_ref().filter(|m|!m.is_empty()).cloned() else{return;};
        let count=self.layers.len();
        self.duplicate_layer(index,true);
        if self.layers.len()==count {return;}
        for (tile,x,y) in mask {
            self.layers[index].tiles[tile].put_pixel(x,y,Rgba([0;4]));
        }
        self.revision=self.revision.wrapping_add(1);self.invalidate_textures();
    }
    fn duplicate_layer(&mut self, index: usize, all_cells: bool) {
        if index >= self.layers.len() { return; }
        if self.layers[index].settings.group_id!=0{self.duplicate_group(index);return;}
        let bytes: usize = self.layers.iter().map(|layer|layer.tiles.iter().map(|tile|tile.as_raw().len()).sum::<usize>()+layer.settings.bytes()).sum();
        let added: usize = self.tiles.iter().map(|tile| tile.as_raw().len()).sum::<usize>()+self.layers[index].settings.bytes()+if self.layers[index].settings.adjustment.is_some()&&self.layers[index].settings.mask.is_none()&&self.selection.mask.is_some(){self.tiles.iter().map(|t|t.as_raw().len()/4).sum::<usize>()}else{0};
        if self.layers.len() >= 32 || bytes.saturating_add(added) > self.layer_budget() {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        self.record_layer_change();
        let source = &self.layers[index];
        let tiles = source.tiles.iter().enumerate().map(|(tile, pixels)| {
            let mut image=if !source.material || all_cells || tile == self.selected {pixels.clone()}else{RgbaImage::new(pixels.width(),pixels.height())};
            if self.selection.mask.is_some() {for (x,y,pixel) in image.enumerate_pixels_mut(){if !self.selection.allows(tile,x,y){*pixel=Rgba([0;4]);}}}
            image
        }).collect();
        let mut settings=source.settings.clone();if self.selection.mask.is_some(){settings.text=None;}settings.lock_alpha=false;settings.lock_pixels=false;settings.lock_position=false;
        if settings.adjustment.is_some()&&self.selection.mask.is_some(){let masks=settings.mask.get_or_insert_with(||source.tiles.iter().map(|t|vec![255;(t.width()*t.height())as usize]).collect());for(tile,mask)in masks.iter_mut().enumerate(){for(i,value)in mask.iter_mut().enumerate(){if !self.selection.allows(tile,i as u32%self.pixels.width(),i as u32/self.pixels.width()){*value=0;}}}}
        let copy = Layer { settings, material: false, name: format!("{} {}", source.name, tr("副本")), visible: true, locked: false, tiles };
        self.layers.insert(index + 1, copy);
        // Insertion can shift the previously active layer; select the copy directly.
        self.active_layer = index + 1;self.selected_layers=vec![self.active_layer];
        self.tiles = self.layers[self.active_layer].tiles.clone();
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
        self.guides.clear();
        self.inline_text = None;
        self.record_layer_change();
        self.set_single_tile(image);
    }
    fn set_single_tile(&mut self, image: RgbaImage) {
        self.transform=None;
        self.text_dialog=None;
        self.selection.clear();
        self.mask_editing=false;
        self.fx_light = Default::default();
        self.layers.clear();
        self.selected_layers.clear();self.layer_anchor=0;
        self.materials.clear();
        self.material_drop_status=None;
        self.active_layer = 0;
        self.pixels = RgbaImage::new(image.width(),image.height());
        self.tiles = vec![self.pixels.clone()];
        self.selected = 0;
        self.selected_slot = 0;
        self.layout.fill(0);
        self.columns = 1;
        self.rows = 1;
        self.multiple = false;
        self.split_preview = false;
        self.split_grid = [1, 1];
        self.width = self.pixels.width();
        self.height = self.pixels.height();
        if !self.elevated { self.footprint = [self.width, self.height]; }
        self.ensure_layers();
        if let Some(base)=self.ensure_material_base() {
            self.layers[base].tiles[0]=image.clone();
            if image.pixels().any(|p|p[3]!=0) { self.materials.push(Material{name:format!("{} 1",tr("素材")),pixels:image}); }
        }
        self.selected_layers=vec![self.active_layer];
        self.layer_anchor=self.active_layer;
    }
    fn layer_icon_button(
        ui: &mut egui::Ui,
        id: egui::Id,
        selected: bool,
        icon: crate::icons::Icon,
        tooltip: &str,
    ) -> egui::Response {
        let (_, rect) = ui.allocate_space(egui::vec2(28., 28.));
        let response = ui.interact(rect, id, egui::Sense::click());
        let visuals = ui.style().interact_selectable(&response, selected);
        ui.painter().rect(
            rect,
            3.,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let icon_color = if matches!(icon, crate::icons::Icon::Unlock|crate::icons::Icon::Lock|crate::icons::Icon::Eye|crate::icons::Icon::EyeOff) && !selected {
            Color32::from_gray(128)
        } else {
            visuals.fg_stroke.color
        };
        crate::icons::paint(ui, icon, rect.shrink(5.), icon_color);
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
        let mut delete = None;
        ui.separator();
        self.layer_properties_ui(ui);
        let mut select = None;
        let mut reorder = None;
        let mut toggle_visible = None;
        let mut toggle_lock = None;
        let mut rename = None;
        let mut material_action = None;
        let mut duplicate = None;
        let mut cut_duplicate = None;
        let mut edit_text = None;
        let mut inline_text = None;
        let mut rasterize = None;
        let mut select_mask=None;
        let mut toggle_mask_link=None;
        let mut detail_toggle=None;
        let mut clip_toggle=None;
        let mut previous_layer_row:Option<usize>=None;
        let owner = ui.id().with("tile_layers_owner");
        self.layer_thumbnails.resize_with(self.layers.len(), || None);
        for index in self.group_display_order() {
            if self.layer_thumbnails[index].as_ref().is_none_or(|(_,revision,cell)| (*revision,*cell)!=(self.revision,self.selected)) {
                let source = if index == self.active_layer { &self.pixels } else { &self.layers[index].tiles[self.selected] };
                let ratio = (36.0 / source.width() as f32).min(28.0 / source.height() as f32);
                let thumb = image::imageops::thumbnail(source, (source.width() as f32*ratio).round().max(1.) as u32, (source.height() as f32*ratio).round().max(1.) as u32);
                let data = tile_preview_image(&thumb);
                let entry = self.layer_thumbnails[index].get_or_insert_with(|| (ui.ctx().load_texture(
                    format!("tile-layer-thumb-{index}"), data.clone(), egui::TextureOptions::NEAREST), self.revision, self.selected));
                entry.0.set(data, egui::TextureOptions::NEAREST);
                entry.1=self.revision;
                entry.2=self.selected;
            }
            let active = index == self.active_layer;
            let row_selected=active||self.selected_layers.contains(&index);
            let name = self.layers[index].name.clone();
            let visible = self.layers[index].visible;
            let effective_visible=visible&&self.ancestors_visible(index);
            let locked = self.layers[index].locked||self.ancestor_locked(index);
            let material_layer = self.layers[index].material;
            let group_layer=self.layers[index].settings.group_id!=0;
            let vector_label = self.layers[index].settings.vector.as_ref().map(vector::Vector::property_label);
            let has_detail = vector_label.is_some() || self.layers[index].settings.adjustment.is_some()
                || (self.layers[index].settings.text.is_some() && self.rectangle_text_allowed());
            let frame = egui::Frame::new().inner_margin(3).stroke(egui::Stroke::new(
                1_f32,
                if row_selected {
                    ui.visuals().selection.stroke.color
                } else {
                    Color32::GRAY
                },
            ));
            let (row_response, dropped) = ui.dnd_drop_zone::<(egui::Id, usize), _>(frame, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    if Self::layer_icon_button(
                        ui,
                        owner.with(("visible", index)),
                        effective_visible,
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
                    let depth=self.group_depth(index);if depth>0{ui.add_space(depth as f32*6.);}
                    if self.layers[index].settings.group_id!=0{let response=ui.add_sized([10.,28.],egui::Button::new("").frame(false));let center=response.rect.center();let points=if self.layers[index].settings.collapsed{vec![center+egui::vec2(-2.,-4.),center+egui::vec2(2.,0.),center+egui::vec2(-2.,4.)]}else{vec![center+egui::vec2(-4.,-2.),center+egui::vec2(0.,2.),center+egui::vec2(4.,-2.)]};ui.painter().add(egui::Shape::line(points,egui::Stroke::new(1.4_f32,ui.visuals().text_color())));if response.clicked(){self.layers[index].settings.collapsed=!self.layers[index].settings.collapsed;}}
                    if self.layers[index].settings.clipped{let(rect,_)=ui.allocate_exact_size(egui::vec2(12.,20.),egui::Sense::hover());let c=ui.visuals().text_color();let a=rect.center()-egui::vec2(3.,5.);let b=rect.center()+egui::vec2(3.,4.);ui.painter().add(egui::Shape::line(vec![a,egui::pos2(a.x,b.y),b],egui::Stroke::new(1.3_f32,c)));ui.painter().add(egui::Shape::convex_polygon(vec![b,b+egui::vec2(-3.,-3.),b+egui::vec2(-3.,3.)],c,egui::Stroke::NONE));}
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(if self.layers[index].settings.group_id!=0{28.}else{36.}, 28.), egui::Sense::click());
                    if self.layers[index].settings.group_id==0&&self.layers[index].settings.adjustment.is_none(){super::seamless::checker_background(
                        ui.painter(),
                        rect,
                        rect.min,
                        Vec2::splat(1.),
                        &super::ImageBackground::Checkerboard,
                        None,
                    );
                    let texture = &self.layer_thumbnails[index].as_ref().unwrap().0;
                    let thumb_size = texture.size_vec2();
                    ui.painter().image(
                        texture.id(),
                        Rect::from_center_size(rect.center(), thumb_size),
                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                    }
                    if self.layers[index].settings.group_id!=0{layer_properties::paint_group_icon(ui,rect,!self.layers[index].settings.collapsed);}
                    if let Some(adjust)=&self.layers[index].settings.adjustment{layer_properties::paint_adjustment_icon(ui,rect,adjust);}
                    if let Some(vector) = &self.layers[index].settings.vector {
                        let badge = Rect::from_min_size(rect.right_bottom() - Vec2::splat(14.), Vec2::splat(13.));
                        ui.painter().rect_filled(badge, 2., ui.visuals().extreme_bg_color);
                        crate::icons::paint(ui, vector.layer_icon(), badge.shrink(1.5), ui.visuals().text_color());
                        response.clone().on_hover_text(tr(vector.property_label()));
                    }
                    if active&&!self.mask_editing{ui.painter().rect_stroke(rect,1.,egui::Stroke::new(2_f32,ui.visuals().selection.stroke.color),egui::StrokeKind::Inside);}
                    if let Some(text)=&self.layers[index].settings.text {
                        self.font_samples.tick(ui.ctx());
                        ui.painter().rect_filled(rect, 2., ui.visuals().extreme_bg_color);
                        ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,"T",egui::FontId::proportional(20.),ui.visuals().text_color());
                        if self.font_samples.catalog.as_ref().is_some_and(|f|!f.contains(&text.family)) {
                            let c=rect.right_bottom()-egui::vec2(7.,7.);
                            ui.painter().add(egui::Shape::convex_polygon(vec![c+egui::vec2(0.,-7.),c+egui::vec2(-7.,6.),c+egui::vec2(7.,6.)],Color32::from_rgb(255,206,60),egui::Stroke::NONE));
                            ui.painter().text(c+egui::vec2(0.,1.),egui::Align2::CENTER_CENTER,"!",egui::FontId::proportional(12.),Color32::BLACK);
                            response.clone().on_hover_text(format!("{} {}",tr("缺少字体"),text.family));
                        }
                        if response.double_clicked() {edit_text=Some((index,rect));}
                    }
                    if response.clicked() {
                        select = Some(index);
                    }
                    if self.layers[index].settings.mask.is_some(){
                        if layer_properties::mask_link_button(ui,self.layers[index].settings.mask_linked).clicked(){toggle_mask_link=Some(index);}
                        let mask_response=layer_properties::mask_button(ui,&self.layers[index].settings.mask.as_ref().unwrap()[self.selected],self.pixels.width(),self.pixels.height(),active&&self.mask_editing,index);
                        if !self.layers[index].settings.mask_enabled{let rect=mask_response.rect.shrink(3.);ui.painter().line_segment([rect.left_bottom(),rect.right_top()],egui::Stroke::new(2_f32,Color32::from_rgb(235,45,45)));}
                        if mask_response.clicked(){select_mask=Some(index);}
                        mask_response.context_menu(|ui|{
                            let mut action:Option<u8>=None;
                            ui.add_enabled_ui(!locked&&!material_layer,|ui|{
                                if ui.button(tr(if self.layers[index].settings.mask_enabled{"停用图层蒙版"}else{"启用图层蒙版"})).clicked(){action=Some(0);}
                                if ui.button(tr("删除图层蒙版")).clicked(){action=Some(1);}
                                if ui.add_enabled(self.layers[index].settings.group_id==0&&self.layers[index].settings.adjustment.is_none()&&self.layers[index].settings.text.is_none(),egui::Button::new(tr("应用图层蒙版"))).clicked(){action=Some(2);}
                            });
                            ui.separator();
                            for(code,label)in[(3,"蒙版转为选区"),(4,"添加蒙版到选区"),(5,"从选区中减去蒙版"),(6,"蒙版与选区交叉")]{if ui.add_enabled(code<5||self.selection.mask.is_some(),egui::Button::new(tr(label))).clicked(){action=Some(code);}}
                            ui.separator();
                            ui.add_enabled_ui(!locked&&!material_layer,|ui|{if ui.button(tr("反相蒙版")).clicked(){action=Some(7);}if ui.button(tr(if self.layers[index].settings.mask_linked{"取消蒙版与内容链接"}else{"链接蒙版与内容"})).clicked(){action=Some(8);}});
                            if let Some(action)=action{ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("tile_mask_context_action"),(index,action)));ui.close_menu();}
                        });
                    }
                    let width = (ui.available_width() - if self.layers[index].settings.link_id!=0{48.}else{28.} - if self.layers[index].settings.has_effects()||self.layer_detail.as_ref().is_some_and(|(i,fx,_,_)|*i==index&&*fx){24.}else{0.} - if has_detail{20.}else{0.} - ui.spacing().item_spacing.x).max(1.);
                    let name_response = ui.push_id(owner.with(("name", index)), |ui| {
                        ui.add_sized([width, 28.], egui::Button::new(&name).selected(row_selected).truncate()
                            .corner_radius(0)
                            .sense(egui::Sense::click_and_drag()))
                    }).inner.on_hover_text(tr("拖拽调整图层顺序"));
                    // Only a primary-button drag can start layer reordering.
                    // A secondary click remains available to the context menu.
                    if !locked && !self.layers[index].settings.lock_position && !self.layers[index].material && name_response.drag_started_by(egui::PointerButton::Primary) {
                        name_response.dnd_set_drag_payload((owner, index));
                    }
                    if name_response.double_clicked() && self.layers[index].settings.text.is_some() {
                        inline_text = Some(index);
                    } else if name_response.clicked() {
                        select = Some(index);
                    }
                    name_response.context_menu(|ui| {
                        if !material_layer&&!locked&&ui.button(tr(if self.layers[index].settings.clipped{"释放剪贴蒙版"}else{"创建剪贴蒙版"})).clicked(){clip_toggle=Some(index);ui.close_menu();}
                        if self.layers[index].settings.text.is_some() {
                            if ui.add_enabled(self.rectangle_text_allowed() && self.text_unlocked(index), egui::Button::new(tr("编辑文字图层"))).clicked(){edit_text=Some((index,name_response.rect));ui.close_menu();}
                            if ui.add_enabled(self.text_unlocked(index),egui::Button::new(tr("栅格化文字"))).clicked(){rasterize=Some(index);ui.close_menu();}
                        }
                        if self.layer_editable()&&index==self.active_layer&&ui.button(tr("清空选中图层")).clicked(){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("tile_clear_layer"),true));ui.close_menu();}
                        if material_layer {
                            if ui.button(format!("{}  Ctrl+J", tr("复制当前选中格为图层"))).clicked() { duplicate = Some((index, false)); ui.close_menu(); }
                            if ui.button(tr("复制所有格为图层")).clicked() { duplicate = Some((index, true)); ui.close_menu(); }
                            ui.separator();
                            if ui.button(tr("清空当前格素材")).clicked() { material_action=Some(None); ui.close_menu(); }
                            ui.menu_button(tr("导入素材"), |ui| {
                                if self.materials.is_empty() { ui.label(tr("素材库为空，请先导入素材。")); }
                                for (i,material) in self.materials.iter().enumerate() {
                                    if ui.button(&material.name).clicked() { material_action=Some(Some(i)); ui.close_menu(); }
                                }
                            });
                            return;
                        }
                        self.style_context_menu(ui,index,false);
                        if ui.button(format!("{}  Ctrl+J", tr(if group_layer{"复制组"}else{"复制图层"}))).clicked() { duplicate = Some((index, true)); ui.close_menu(); }
                        if !group_layer&&self.layers[index].settings.text.is_none()&&self.layers[index].settings.adjustment.is_none()&&ui.add_enabled(!locked && self.layers[index].visible && self.selection.mask.as_ref().is_some_and(|m|!m.is_empty()),egui::Button::new(format!("{}  Ctrl+Shift+J",tr("通过剪切的图层")))).clicked(){cut_duplicate=Some(index);ui.close_menu();}
                        if ui
                            .add_enabled(
                                !locked,
                                egui::Button::new(format!("{}  Delete",tr(if group_layer{"删除组"}else{"删除图层"}))),
                            )
                            .clicked()
                        {
                            delete = Some(index);
                            ui.close_menu();
                        }
                        let mut text = name.clone();
                        if !locked && !self.layers[index].material {
                            ui.horizontal(|ui| {
                                ui.label(tr("图层名"));
                                if ui.add(egui::TextEdit::singleline(&mut text).char_limit(64)).changed() {
                                    rename = Some((index, text));
                                }
                            });
                        }
                    });
                    if has_detail{let expanded=self.layer_detail.as_ref().is_some_and(|(i,fx,_,_)|*i==index&&!*fx);let response=ui.add_sized([16.,28.],egui::Button::new("").frame(false).selected(expanded));let c=response.rect.center();let points=if expanded{vec![c+egui::vec2(-4.,-2.),c+egui::vec2(0.,2.),c+egui::vec2(4.,-2.)]}else{vec![c+egui::vec2(-2.,-4.),c+egui::vec2(2.,0.),c+egui::vec2(-2.,4.)]};ui.painter().add(egui::Shape::line(points,egui::Stroke::new(1.4_f32,ui.visuals().text_color())));let rect=response.rect;if response.on_hover_text(tr(vector_label.unwrap_or(if self.layers[index].settings.text.is_some(){"编辑文字图层"}else{"调整"}))).clicked(){detail_toggle=Some((index,false,rect));}}
                    if self.layers[index].settings.has_effects()||self.layer_detail.as_ref().is_some_and(|(i,fx,_,_)|*i==index&&*fx){let expanded=self.layer_detail.as_ref().is_some_and(|(i,fx,_,_)|*i==index&&*fx);let response=ui.add_sized([20.,28.],egui::Button::new(egui::RichText::new("fx").italics().size(14.)).frame(false).selected(expanded));let rect=response.rect;response.context_menu(|ui|{self.style_context_menu(ui,index,true);});if response.on_hover_text(tr("图层样式")).clicked(){detail_toggle=Some((index,true,rect));}}
                    if self.layers[index].settings.link_id!=0 {let(rect,_)=ui.allocate_exact_size(egui::vec2(16.,16.),egui::Sense::hover());let c=ui.visuals().text_color();let center=rect.center();for dx in [-3.,3.]{ui.painter().rect_stroke(Rect::from_center_size(center+egui::vec2(dx,0.),egui::vec2(9.,6.)),3.,egui::Stroke::new(1.3_f32,c),egui::StrokeKind::Inside);}ui.painter().line_segment([center-egui::vec2(3.,0.),center+egui::vec2(3.,0.)],egui::Stroke::new(1.3_f32,c));}
                    let inherited_lock=self.ancestor_locked(index);
                    let lock=ui.add_enabled_ui(!material_layer&&!inherited_lock,|ui|Self::layer_lock_button(ui,owner.with(("lock",index)),self.layers[index].locked||inherited_lock)).inner;
                    let lock=if inherited_lock{lock.on_disabled_hover_text(tr("由父组锁定"))}else{lock};
                    if lock.clicked() && !self.layers[index].material && !self.ancestor_locked(index) {
                        toggle_lock = Some(index);
                    }
                });
            });
            if let Some(upper)=previous_layer_row{if self.layers[upper].settings.parent==self.layers[index].settings.parent{
                let rect=row_response.response.rect;let boundary=Rect::from_min_max(egui::pos2(rect.left(),rect.top()-ui.spacing().item_spacing.y.max(5.)),egui::pos2(rect.right(),rect.top()));
                if ui.input(|i|i.modifiers.alt){let response=ui.interact(boundary,owner.with(("clip_boundary",upper)),egui::Sense::click());if response.hovered(){ui.output_mut(|o|o.cursor_icon=egui::CursorIcon::PointingHand);ui.painter().line_segment([boundary.left_center(),boundary.right_center()],egui::Stroke::new(1.5_f32,ui.visuals().selection.stroke.color));}if response.clicked(){clip_toggle=Some(upper);}}
            }}previous_layer_row=Some(index);
            // Context actions belong to the name button. Making the enclosing row
            // clickable here would overlap the eye and lock hit targets.
            let hovered=row_response.response.dnd_hover_payload::<(egui::Id,usize)>();
            if let Some(payload)=dropped.or(hovered){if payload.0==owner{if let Some(pointer)=ui.input(|i|i.pointer.hover_pos()){
                let rect=row_response.response.rect;let inner=self.layers[index].settings.group_id!=0&&pointer.y>rect.top()+8.&&pointer.y<rect.bottom()-8.;
                let parent=if inner{self.layers[index].settings.group_id}else{self.layers[index].settings.parent};let anchor=if inner{None}else{Some(index)};let above=pointer.y<rect.center().y;
                if self.valid_layer_drop(payload.1,parent,anchor){if inner{ui.painter().rect_stroke(rect,2.,egui::Stroke::new(2_f32,ui.visuals().selection.stroke.color),egui::StrokeKind::Inside);}else{let y=if above{rect.top()}else{rect.bottom()};ui.painter().line_segment([egui::pos2(rect.left()+34.,y),egui::pos2(rect.right(),y)],egui::Stroke::new(2_f32,ui.visuals().selection.stroke.color));}if ui.input(|i|i.pointer.button_released(egui::PointerButton::Primary)){reorder=Some((payload.1,parent,anchor,above));}}
            }}}

        }
        if egui::DragAndDrop::payload::<(egui::Id,usize)>(ui.ctx()).is_some_and(|payload|payload.0==owner){
            let frame=egui::Frame::new().inner_margin(5).stroke(egui::Stroke::new(1_f32,ui.visuals().widgets.inactive.bg_stroke.color));
            let(_,drop)=ui.dnd_drop_zone::<(egui::Id,usize),_>(frame,|ui|{ui.set_min_width((ui.available_width()-10.).max(1.));ui.label(tr("拖到此处移出组"));});
            if let Some(payload)=drop{if payload.0==owner{reorder=Some((payload.1,0,None,true));}}
        }
        if let Some((index,rect))=edit_text {self.toggle_layer_detail(index,false,rect,ui.ctx().screen_rect());}
        if let Some(index)=rasterize {self.rasterize_text(index);}
        if let Some(index)=clip_toggle{self.toggle_clipping(index);}
        if let Some(index)=toggle_mask_link{if !self.layers[index].locked&&!self.ancestor_locked(index){self.record_layer_change();self.layers[index].settings.mask_linked=!self.layers[index].settings.mask_linked;}}
        if let Some((index,action))=ui.ctx().data_mut(|d|d.remove_temp::<(usize,u8)>(egui::Id::new("tile_mask_context_action"))){self.mask_context_action(index,action);}
        if let Some(index)=select_mask{self.select_layer(index);self.mask_editing=true;}
        if let Some(index)=cut_duplicate {self.cut_selection_to_layer(index);}
        if let Some((index, all_cells)) = duplicate { self.duplicate_selected_layers(index, all_cells); }
        if let Some(material) = material_action { self.assign_material(material); }
        if let Some((index, name)) = rename {
            self.record_layer_change();
            self.layers[index].name = name;
        }
        if let Some(index) = select {
            self.select_layer_click(index,ui.input(|i|i.modifiers));
        }
        if let Some(index) = inline_text {
            self.begin_inline_text(index, None);
        }
        if let Some(index) = toggle_visible {
            self.record_layer_change();
            self.layers[index].visible = !self.layers[index].visible;
        }
        if let Some(index) = toggle_lock {
            self.record_layer_change();
            self.layers[index].locked = !self.layers[index].locked;
        }
        if let Some((from,parent,anchor,above))=reorder{self.drop_layer_objects(from,parent,anchor,above);}
        if let Some(index) = delete { self.request_layer_action(false,index); }
        if ui.ctx().data_mut(|d|d.remove_temp::<bool>(egui::Id::new("tile_clear_layer"))).unwrap_or(false){self.request_layer_action(true,self.active_layer);}
        if let Some((index,fx,rect))=detail_toggle{self.toggle_layer_detail(index,fx,rect,ui.ctx().screen_rect());}
        self.layer_detail_ui(ui);
        if !self.layer_editable() && self.layers[self.active_layer].settings.text.is_none() {
            ui.small(tr("当前图层已锁定或隐藏，不能绘制。"));
        }
    }
    fn invalidate_textures(&mut self) {
        self.transform_bounds_cache=None;
        self.layer_thumbnails.clear();
        self.dirty_texture = true;
        self.dirty_all = true;
        self.dirty_tiles.clear();
    }
    fn whole_unit(&self) -> bool {
        self.multiple && !self.elevated
    }
    fn edit_size(&self) -> Vec2 {
        let (w, h) = self.pixels.dimensions();
        if self.elevated && self.multiple {
            return egui::vec2((self.columns+self.rows) as f32*self.footprint[0] as f32/2.,(self.columns+self.rows-2) as f32*self.footprint[1] as f32/2.+h as f32);
        }
        if self.whole_unit() {
            if self.hex != 0 { return hex_size(self.columns,self.rows,w,h,self.hex == 2); }
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
        let cell = self.mapped_cell(x, y, w, h);
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        if !(self.wrap || self.rendering_merged || self.subgroup_active())
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
        let cell = self.mapped_cell(x, y, w, h);
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
    fn geometry_hit(&self, x: i32, y: i32, layout: bool, wrap: bool) -> Option<(usize, u32, u32)> {
        if !wrap {
            return self.finite_plane_hit(x, y, layout);
        }
        let (w, h) = self.pixels.dimensions();
        let cell = self.mapped_cell(x, y, w, h);
        let slot = if layout { self.tile_at(cell.col, cell.row) } else { self.selected };
        Some((slot, cell.x, cell.y))
    }
    fn fill_region(&mut self, mut point: (i32, i32), layout: bool) {
        self.finish_stroke();
        let (w, h) = self.pixels.dimensions();
        let (cols, rows) = if layout && self.multiple {
            (self.columns, self.rows)
        } else {
            (1, 1)
        };
        if self.wrap || self.rendering_merged {
            let cell = self.mapped_cell(point.0, point.1, w, h);
            let shift = self.mapped_center(cell.col.div_euclid(cols as i32)*cols as i32,
                cell.row.div_euclid(rows as i32)*rows as i32,w,h);
            point.0 -= shift.x as i32;
            point.1 -= shift.y as i32;
        }
        let Some((tile, x, y)) = self.finite_plane_hit(point.0, point.1, layout) else {
            return;
        };
        if !self.prepare_pixel_paint(|editor| editor.selection.allows(tile, x, y)) {
            return;
        }
        let state=self.snapshot();
        let target = self.snapshot_edit_pixel(&state,tile,x,y);
        let color = Rgba(self.color);
        // Transparent pixels may have different hidden RGB values. Treat them as
        // one empty region; the finite tile boundary contains background fills.
        let empty = target[3] == 0;
        let matches_target = |pixel: Rgba<u8>| if empty { pixel[3] == 0 } else { pixel == target };
        if target == color {
            return;
        }
        let (left, top, width, height) = if self.hex != 0 {
            let size = hex_size(cols,rows,w,h,self.hex == 2);
            (-(w as i32)/2,-(h as i32)/2,size.x as usize,size.y as usize)
        } else if self.diamond {
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
        let mut visited = vec![false; width * height];
        let mut queue = std::collections::VecDeque::new();
        let mut region = Vec::new();
        queue.push_back(point);
        while let Some((px, py)) = queue.pop_front() {
            let Some((tile, x, y)) = self.finite_plane_hit(px, py, layout) else {
                continue;
            };
            if !self.selection.allows(tile,x,y) || !matches_target(self.snapshot_edit_pixel(&state,tile,x,y)) {
                continue;
            }
            let index = (py - top) as usize * width + (px - left) as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;
            region.push((tile, x, y));
            if self.subgroup_active() && layout {self.subgroup_touches.insert((px,py));}
            for next in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
                if let Some((nt, nx, ny)) = self.finite_plane_hit(next.0, next.1, layout) {
                    let ni = (next.1 - top) as usize * width + (next.0 - left) as usize;
                    if !visited[ni] && matches_target(self.snapshot_edit_pixel(&state,nt,nx,ny)) {
                        queue.push_back(next);
                    }
                }
            }
        }
        self.before.get_or_insert(state);
        for (tile, x, y) in region {
            self.write_pixel(tile, x, y, color);
        }
        self.finish_stroke();
        self.message.clear();
    }
    fn plane_filled_rectangle(&mut self,a:(i32,i32),b:(i32,i32),layout:bool, ellipse: bool, soft: bool){
        let width=(i64::from(a.0)-i64::from(b.0)).abs()+1;let height=(i64::from(a.1)-i64::from(b.1)).abs()+1;if width.saturating_mul(height)>4*1024*1024{self.message=tr("填充区域过大，请缩小循环范围。").into();return;}
        let diamond = self.diamond && !self.plain_image;
        let contains = |x: f64, y: f64| {
            let dx = (2. * x - f64::from(a.0) - f64::from(b.0)).abs() / width as f64;
            let dy = (2. * y - f64::from(a.1) - f64::from(b.1)).abs() / height as f64;
            if ellipse {
                dx * dx + dy * dy <= 1.
            } else {
                !diamond || dx + dy <= 1.
            }
        };
        let coverage = |x: i32, y: i32| {
            let (x, y) = (f64::from(x), f64::from(y));
            if !soft || (!diamond && !ellipse) {
                return f32::from(u8::from(contains(x, y)));
            }
            let samples = [-0.375, -0.125, 0.125, 0.375];
            samples.into_iter().flat_map(|dy| samples.map(move |dx| contains(x + dx, y + dy)))
                .filter(|&inside| inside).count() as f32 / 16.
        };
        if !self.prepare_pixel_paint(|editor| (a.1.min(b.1)..=a.1.max(b.1))
            .any(|y| (a.0.min(b.0)..=a.0.max(b.0)).any(|x| coverage(x,y) > 0. && editor.paint_hit((x, y), layout)))) {
            return;
        }
        if self.before.is_none(){self.before=Some(self.snapshot());}let mut touched=std::collections::HashSet::new();
        for y in a.1.min(b.1)..=a.1.max(b.1){for x in a.0.min(b.0)..=a.0.max(b.0){
            let amount = coverage(x, y);
            if amount == 0. { continue; }
            if let Some((tile,px,py))=self.plane_hit(x,y,layout){if !self.selection.allows(tile,px,py){continue;}if self.subgroup_active()&&layout{self.subgroup_touches.insert((x,y));}if !touched.insert((tile,px,py)){continue;}let old=self.before_edit_pixel(tile,px,py);let color=self.blend_edit_pixel(old,Rgba(self.color),amount,false,BrushMode::Normal);self.write_pixel(tile,px,py,color);}}}
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
                        filled:self.tool==Tool::Rectangle&&self.rectangle_fill,
                        ellipse: self.tool == Tool::Rectangle && self.rectangle_ellipse,
                        soft: self.soft_brush,
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
            let b = self.frame_end(drag);
            let brush_mode = self.soft_brush;
            if !drag.line { self.soft_brush = drag.soft; }
            if drag.filled{self.plane_filled_rectangle(a,b,drag.layout,drag.ellipse,drag.soft);}
            else if drag.line {
                self.plane_stroke(a, b, drag.layout);
            } else if drag.ellipse {
                let points = selection::ellipse_points(Rect::from_two_pos(
                    egui::pos2(a.0 as f32, a.1 as f32),
                    egui::pos2(b.0 as f32, b.1 as f32),
                ));
                let pixel = |p: &Pos2| (p.x.round() as i32, p.y.round() as i32);
                for (from, to) in points.iter().zip(points.iter().cycle().skip(1)) {
                    self.plane_stroke(pixel(from), pixel(to), drag.layout);
                }
            } else if self.diamond && !self.plain_image {
                let x = a.0.midpoint(b.0);
                let y = a.1.midpoint(b.1);
                let points = [(x, a.1), (b.0, y), (x, b.1), (a.0, y)];
                for (from, to) in points.into_iter().zip(points.into_iter().cycle().skip(1)) {
                    self.plane_stroke(from, to, drag.layout);
                }
            } else {
                self.plane_stroke(a, (b.0, a.1), drag.layout);
                self.plane_stroke((b.0, a.1), b, drag.layout);
                self.plane_stroke(b, (a.0, b.1), drag.layout);
                self.plane_stroke((a.0, b.1), a, drag.layout);
            }
            self.finish_stroke();
            self.soft_brush = brush_mode;
        }
    }
    fn frame_end(&self, drag: RectangleDrag) -> (i32, i32) {
        if self.plain_image || !self.diamond || drag.line || drag.ellipse {
            return drag.end;
        }
        let delta = [
            f64::from(drag.end.0) - f64::from(drag.start.0),
            f64::from(drag.end.1) - f64::from(drag.start.1),
        ];
        if delta == [0., 0.] {
            return drag.end;
        }
        let [w, h] = diamond_size(delta.map(|value| value.abs() + 1.), self.footprint);
        let x = (w - 1.).copysign(delta[0]);
        let y = (h - 1.).copysign(delta[1]);
        (drag.start.0.saturating_add(x as i32), drag.start.1.saturating_add(y as i32))
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
                        super::preview_color(self.color),
                    ),
                );
                return;
            }
            let end = self.frame_end(drag);
            let min = egui::vec2(
                drag.start.0.min(end.0) as f32,
                drag.start.1.min(end.1) as f32,
            );
            let max = egui::vec2(
                drag.start.0.max(end.0) as f32 + 1.,
                drag.start.1.max(end.1) as f32 + 1.,
            );
            if drag.ellipse {
                let points = selection::ellipse_points(Rect::from_min_max(origin + min * scale, origin + max * scale));
                let fill = if drag.filled { super::preview_color(self.color).gamma_multiply(0.35) } else { Color32::TRANSPARENT };
                painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::new(1_f32, Color32::YELLOW)));
                return;
            }
            if self.diamond && !self.plain_image {
                let mid = (min + max) * 0.5;
                let points = [egui::vec2(mid.x, min.y), egui::vec2(max.x, mid.y),
                    egui::vec2(mid.x, max.y), egui::vec2(min.x, mid.y)]
                    .map(|p| origin + p * scale).to_vec();
                let fill = if drag.filled { super::preview_color(self.color).gamma_multiply(0.35) }
                    else { Color32::TRANSPARENT };
                painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::new(1_f32, Color32::YELLOW)));
                return;
            }
            if drag.filled{painter.rect_filled(Rect::from_min_max(origin+min*scale,origin+max*scale),0.,super::preview_color(self.color).gamma_multiply(0.35));}
            painter.rect_stroke(
                Rect::from_min_max(origin + min * scale, origin + max * scale),
                0.,
                egui::Stroke::new(1_f32, Color32::YELLOW),
                egui::StrokeKind::Inside,
            );
        }
    }
    fn set_infinite_view(&mut self,infinite:bool) {
        if self.plain_image || self.infinite_view==infinite { return; }
        self.finish_stroke(); self.selection.draft.clear(); self.infinite_view=infinite;
        self.last=None; self.preview_last=None; self.zoom_drag=None; self.merged_zoom_drag=None;
        self.pan_drag=false; self.merged_pan_drag=false;
    }
    fn center_view(&mut self) {
        self.finish_stroke(); self.pan=Vec2::ZERO;
        self.last=None; self.preview_last=None; self.zoom_drag=None; self.merged_zoom_drag=None;
        self.pan_drag=false; self.merged_pan_drag=false;
    }
    fn tool_shortcuts(&mut self, ctx: &egui::Context) {
        if self.confirm_transform_exit || self.pending_open.is_some()||self.text_dialog.as_ref().is_some_and(|d|d.target.is_none()){return;}
        if self.inline_text.is_some() || self.path_tool.confirming_conversion()
            || Self::text_input_active(ctx) || ctx.input(|i| !i.focused) {
            return;
        }
        if ctx.input_mut(|i| {
            i.events.iter().any(|event| matches!(event, egui::Event::Key {
                key: egui::Key::H, pressed: true, repeat: false, ..
            })) && i.consume_key(egui::Modifiers::CTRL, egui::Key::H)
        }) {
            self.toggle_transform_controls();
            return;
        }
        if ctx.input(|i| i.pointer.primary_down()) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::R)) {
            self.guides.rulers = !self.guides.rulers;
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Semicolon)) {
            self.guides.visible = !self.guides.visible;
            return;
        }
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::T)){if matches!(self.tool,Tool::Move|Tool::Select){self.begin_transform();}return;}
        if self.transform.is_some() {
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Escape)){self.finish_transform(false);return;}
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Enter)){self.finish_transform(true);return;}
            let switching=ctx.input(|i|i.modifiers.is_none() && [egui::Key::V,egui::Key::M,egui::Key::B,egui::Key::I,egui::Key::E,egui::Key::K,egui::Key::L,egui::Key::G,egui::Key::Z,egui::Key::T,egui::Key::U,egui::Key::A,egui::Key::P].iter().any(|k|i.key_pressed(*k)));
            if !switching || !self.finish_transform(true){return;}
        }
        self.nudge_layers(ctx);
        if self.vector_available() {
            for (key, tool) in [(egui::Key::U, Tool::Shape), (egui::Key::A, Tool::Path), (egui::Key::P, Tool::Pen)] {
                if ctx.input_mut(|i| {
                    i.events.iter().any(|event| matches!(event, egui::Event::Key {
                        key: pressed, pressed: true, repeat: false, ..
                    } if *pressed == key))
                        && i.consume_key(egui::Modifiers::NONE, key)
                }) {
                    self.finish_stroke();
                    if self.tool == tool {
                        let count = vector::PathTool::modes(tool).len();
                        self.path_tool.set_mode(tool, (self.path_tool.mode(tool) + 1) % count);
                    } else {
                        self.tool = tool;
                    }
                    return;
                }
            }
            if self.tool == Tool::Pen && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.path_tool.clear_pen();
                return;
            }
        }
        if self.rectangle_text_allowed() && ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::T)) {
            self.finish_stroke();
            self.tool = Tool::Text;
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Delete)) {
            self.fill_selection_with_color(self.drawing_background);
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::Delete)) {
            self.fill_selection_color();
            return;
        }
        if ctx.input_mut(|i| {
            i.events.iter().any(|event| matches!(event, egui::Event::Key {
                key: egui::Key::Delete, pressed: true, repeat: false, ..
            })) && i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
        }) {
            if self.selection.mask.is_some() {
                self.delete_selection_pixels();
            } else {
                self.request_layer_action(false, self.active_layer);
            }
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
            self.select_entire_canvas();
            return;
        }
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::D)) {self.selection.clear();return;}
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Escape)) {
            if self.selection.draft.is_empty(){self.selection.clear();}else{self.selection.draft.clear();}return;
        }
        if !self.elevated && ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::M)) {
            self.finish_stroke();self.selection.draft.clear();
            if self.tool==Tool::Select {self.selection.mode=self.selection.mode.next();}
            self.tool=Tool::Select;return;
        }
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL|egui::Modifiers::SHIFT,egui::Key::J)){self.cut_selection_to_layer(self.active_layer);return;}
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::J)) {
            self.duplicate_selected_layers(self.active_layer, false);
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
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Q)) { self.set_infinite_view(false); }
        if !self.plain_image && ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::W)) { self.set_infinite_view(true); }
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::H)) { self.center_view(); }
        for (key, tool) in [
            (egui::Key::V, Tool::Move),
            (egui::Key::Z, Tool::Zoom),
            (egui::Key::B, Tool::Brush),
            (egui::Key::I, Tool::Eyedropper),
            (egui::Key::E, Tool::Eraser),
            (egui::Key::K, Tool::Rectangle),
            (egui::Key::L, Tool::Line),
            (egui::Key::G, Tool::Fill),
        ] {
            if self.elevated && matches!(tool, Tool::Rectangle | Tool::Line | Tool::Fill) {
                continue;
            }
            if matches!(tool, Tool::Brush | Tool::Rectangle) && !ctx.input(|i| i.events.iter().any(|event| matches!(event,
                egui::Event::Key { key: pressed, pressed: true, repeat: false, .. } if *pressed == key
            ))) {
                continue;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                if tool == Tool::Brush && self.tool == Tool::Brush {
                    self.soft_brush = !self.soft_brush;
                }
                if tool == Tool::Rectangle && self.tool == Tool::Rectangle {
                    self.rectangle_ellipse = !self.rectangle_ellipse;
                }
                self.finish_stroke();
                self.selection.draft.clear();
                self.tool = tool;
            }
        }
    }
    fn text_input_active(ctx: &egui::Context) -> bool {
        ctx.memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some())
    }
    fn write_pixel(&mut self, tile: usize, x: u32, y: u32, color: Rgba<u8>) {
        if self.mask_editing {
            if !self.selection.allows(tile,x,y)||!self.layer_editable(){return;}
            let value=(((color[0] as u32*30+color[1] as u32*59+color[2] as u32*11)/100)*color[3] as u32/255) as u8;
            let pixel=&mut self.layers[self.active_layer].settings.mask.as_mut().unwrap()[tile][(y*self.pixels.width()+x) as usize];
            if *pixel!=value{*pixel=value;self.invalidate_textures();self.revision=self.revision.wrapping_add(1);}return;
        }
        let mut color=color;
        if self.layers.get(self.active_layer).is_some_and(|l|l.settings.lock_alpha){let old=*self.tiles[tile].get_pixel(x,y);if old[3]==0||color[3]==0{return;}color[3]=old[3];}
        if !self.selection.allows(tile,x,y) || !self.layer_editable() || *self.tiles[tile].get_pixel(x, y) == color {
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
        if self.soft_brush{self.brush_segment(Pos2::new(from.0 as f32,from.1 as f32),Pos2::new(to.0 as f32,to.1 as f32),layout);return;}
        let mut points = Vec::new();
        line(from, to, |x,y|points.push((x,y)));
        if !self.prepare_pixel_paint(|editor| points.iter().any(|&point| editor.paint_hit(point, layout))) {
            return;
        }
        if self.before.is_none() {
            self.before = Some(self.snapshot());
        }
        let size=self.brush_size as i32;let start=-(size/2);
        let color = Rgba(self.color);
        for (x,y) in points {for dy in start..start+size {for dx in start..start+size {
            if !pencil_tip_contains(size,dx-start,dy-start){continue;}
            if let Some((tile,px,py))=self.plane_hit(x+dx,y+dy,layout){
                if self.subgroup_active() && layout{self.subgroup_touches.insert((x+dx,y+dy));}
                let ink = if self.tool == Tool::Eraser {
                    self.hard_eraser_pixel(tile, px, py)
                } else if self.tool == Tool::Brush && !self.mask_editing {
                    self.blend_edit_pixel(self.before_edit_pixel(tile, px, py), color, 1., false, BrushMode::Normal)
                } else {
                    color
                };
                self.write_pixel(tile,px,py,ink);
            }
        }}}
    }
    fn hard_eraser_pixel(&self, tile: usize, x: u32, y: u32) -> Rgba<u8> {
        let original = self.before_edit_pixel(tile, x, y);
        self.blend_edit_pixel(original, Rgba(self.color), 1., true, BrushMode::Normal)
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
            selected_slot: self.selected_slot,
            subgroup_size: self.subgroup_size,
            fx_light: self.fx_light,
            selection:self.selection.history(),
            selected_layers:self.selected_layers.clone(),layer_anchor:self.layer_anchor,
            mask_editing:self.mask_editing,
            materials: self.materials.clone(),
            layers,
            active_layer: self.active_layer,
            tiles,
            selected: self.selected,
            layout: self.layout.clone(),
            columns: self.columns,
            rows: self.rows,
            multiple: self.multiple,
            diamond: self.diamond,
            hex: self.hex,
            elevated: self.elevated,
            footprint: self.footprint,
            wrap: self.wrap,
        }
    }
    fn select_slot(&mut self, slot: usize) {
        let index = self.layout[slot];
        if self.selected_slot == slot && self.selected == index { return; }
        self.finish_stroke();
        self.selected_slot = slot;
        if self.selected == index { return; }
        self.tiles[self.selected] = self.pixels.clone();
        self.selected = index;
        self.pixels = self.tiles[index].clone();
        self.invalidate_textures();
    }
    fn finish_stroke(&mut self) {
        self.subgroup_sync();
        self.subgroup_touches.clear();
        self.stroke_pressure = None;
        self.input_pressure = None;
        self.brush_coverage.clear();
        self.brush_sampler.reset();
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
    fn ensure_layer_after_removal(&mut self) {
        if self.layers.iter().all(|l| l.material) {
            self.layers.push(Layer { settings:Default::default(), material:false,
                name:format!("{} 1",tr("图层")), visible:true, locked:false,
                tiles:self.tiles.iter().map(|im|RgbaImage::new(im.width(),im.height())).collect(),
            });
        }
    }
    fn delete_layer(&mut self, index: usize) {
        if index >= self.layers.len() || self.layers[index].locked || self.layers[index].material || self.ancestor_locked(index) { return; }
        if self.layers[index].settings.group_id!=0{self.delete_group(index);return;}
            self.record_layer_change();
            self.layers.remove(index);
            self.cleanup_single_links();
            self.ensure_layer_after_removal();
            self.active_layer = if self.active_layer > index {
                self.active_layer - 1
            } else {
                self.active_layer.min(self.layers.len() - 1)
            };
            self.selected_layers=vec![self.active_layer];self.layer_anchor=self.active_layer;self.mask_editing=false;
            self.tiles = self.layers[self.active_layer].tiles.clone();
            self.pixels = self.tiles[self.selected].clone();
    }
    fn clear_selected_layer(&mut self) {
        if !self.layer_editable() { return; }
        self.record_layer_change();
        if self.mask_editing{let w=self.pixels.width();for(tile,mask)in self.layers[self.active_layer].settings.mask.as_mut().unwrap().iter_mut().enumerate(){for(i,value)in mask.iter_mut().enumerate(){if self.selection.allows(tile,i as u32%w,i as u32/w){*value=0;}}}self.invalidate_textures();return;}
        if self.layers[self.active_layer].settings.lock_alpha{return;}
        for tile in &mut self.tiles { *tile = RgbaImage::new(tile.width(), tile.height()); }
        self.pixels = self.tiles[self.selected].clone();
        self.sync_layer();
        self.message = tr("已清空选中图层。").into();
    }
    #[cfg(test)]
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
            self.materials.clear();
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
        self.finish_transform(false);
        self.finish_stroke();
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        if let Some(state) = stack.pop() {
            let old = self.snapshot();
            self.pixels = state.tiles[state.selected].clone();
            self.fx_light = state.fx_light;
            self.layers = state.layers;
            self.materials = state.materials;
            self.active_layer = state.active_layer;
            self.selected_layers=state.selected_layers;self.layer_anchor=state.layer_anchor;
            self.tiles = state.tiles;
            self.selected = state.selected;
            self.selected_slot = state.selected_slot;
            self.subgroup_size = state.subgroup_size;
            (self.width, self.height) = self.pixels.dimensions();
            self.layout = state.layout;
            self.columns = state.columns;
            self.rows = state.rows;
            self.multiple = state.multiple;
            self.diamond = state.diamond;
            self.hex = state.hex;
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
            self.selection.restore(state.selection,self.revision);
            self.mask_editing=state.mask_editing && self.layers[self.active_layer].settings.mask.is_some();
        }
    }
    fn sync_texture(&mut self, ctx: &egui::Context) {
        let editing = self.inline_text.is_some();
        if self.preview_inline_active != editing {
            self.invalidate_textures();
            self.preview_inline_active = editing;
        }
        if !self.dirty_texture {
            return;
        }
        self.tiles[self.selected] = self.pixels.clone();
        self.textures.truncate(self.tiles.len());
        self.preview_samples.truncate(self.tiles.len());
        for index in 0..self.tiles.len() {
            if !self.dirty_all && !self.dirty_tiles.contains(&index) && index < self.textures.len()
            {
                continue;
            }
            let preview = self.inline_text.as_ref()
                .and_then(|edit| edit.preview.as_ref()).and_then(|tiles| tiles.get(index));
            let target = self.inline_text.as_ref().and_then(|edit| edit.index);
            let old = preview.filter(|_| target == Some(self.active_layer)).map(|image| {
                let source = if index == self.selected { &mut self.pixels } else { &mut self.tiles[index] };
                std::mem::replace(source, image.clone())
            });
            let mut display = self.composite(index);
            if let Some(original) = old {
                let source = if index == self.selected { &mut self.pixels } else { &mut self.tiles[index] };
                *source = original;
            }
            if let Some(image) = preview.filter(|_| target.is_none()) {
                let mut settings = layer_properties::Properties::default();
                settings.opacity = if self.plain_image { 100 } else { 35 };
                settings.composite(&mut display, image, index, self.fx_light);
            }
            let (w, h) = display.dimensions();
            if self.diamond || self.hex!=0 {for (x, y, p) in display.enumerate_pixels_mut() {
                if !self.inside_tile(x as i32, y as i32, w, h) {
                    *p = Rgba([0; 4]);
                }
            }
            }
            while self.preview_samples.len()<=index{self.preview_samples.push(RgbaImage::new(0,0));}
            self.preview_samples[index]=display.clone();
            let data = tile_preview_image(&display);
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
        if self.tool == Tool::Select && response.hovered() {ui.output_mut(|o|o.cursor_icon=self.selection.cursor());}
        let picking = self.tool == Tool::Eyedropper || (self.tool != Tool::Select && ui.input(|i| i.modifiers.alt));
        let navigating = ui.input(|i| i.key_down(egui::Key::Space) || i.pointer.secondary_down() || i.pointer.middle_down());
        if response.hovered() && picking && !navigating && !self.pan_drag {
            if let Some(pointer) = response.hover_pos() {
                ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::None);
                let rect = Rect::from_min_size(pointer + egui::vec2(-3., -21.), Vec2::splat(24.));
                crate::icons::paint(ui, crate::icons::Icon::Eyedropper, rect.expand(1.), Color32::BLACK);
                crate::icons::paint(ui, crate::icons::Icon::Eyedropper, rect, Color32::WHITE);
            }
            return;
        }

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
            self.pressure_size(self.pen.current)
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
        if self.soft_brush {
            painter.circle_stroke(pointer, rect.width() / 2., egui::Stroke::new(3_f32, Color32::BLACK));
            painter.circle_stroke(rect.center(), rect.width() / 2., egui::Stroke::new(1_f32, Color32::WHITE));
            return;
        }
        if matches!(self.tool, Tool::Brush | Tool::Eraser) {
            let mut edges = Vec::new();
            for y in 0..size {
                for x in 0..size {
                    if !pencil_tip_contains(size, x, y) {
                        continue;
                    }
                    let cell = Rect::from_min_size(
                        min + egui::vec2(x as f32, y as f32) * scale,
                        Vec2::splat(scale),
                    );
                    if self.tool == Tool::Brush { painter.rect_filled(cell, 0., super::preview_color(self.color)); }
                    for (nx, ny, edge) in [
                        (x - 1, y, [cell.left_top(), cell.left_bottom()]),
                        (x + 1, y, [cell.right_top(), cell.right_bottom()]),
                        (x, y - 1, [cell.left_top(), cell.right_top()]),
                        (x, y + 1, [cell.left_bottom(), cell.right_bottom()]),
                    ] {
                        if !pencil_tip_contains(size, nx, ny) {
                            edges.push(edge);
                        }
                    }
                }
            }
            for (width, color) in [(3_f32, Color32::BLACK), (1_f32, Color32::WHITE)] {
                for edge in &edges {
                    painter.line_segment(*edge, egui::Stroke::new(width, color));
                }
            }
            return;
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
    fn minimum_zoom(&self)->f32 { if self.plain_image {0.01} else {0.25} }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (outer, allocation) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ui.available_height().max(100.)),
            egui::Sense::hover(),
        );
        let area = self.guides.area(outer);
        let response = ui.interact(area, allocation.id.with("canvas"), egui::Sense::click_and_drag());
        let tools = self.floating_tools(ui, area);
        let over_tools = self.toolbar.dragging || ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| tools.contains(p));
        let guide_scale = self.zoom / ui.ctx().pixels_per_point();
        let guide_origin = area.center() + self.pan - self.edit_size() * (guide_scale / 2.);
        let (w, h) = self.pixels.dimensions();
        let whole = self.whole_unit();
        let (cols, rows) = if whole { (self.columns, self.rows) } else { (1, 1) };
        let first = if self.plain_image {
            egui::vec2(-((w / 2) as f32), -((h / 2) as f32))
        } else {
            egui::vec2(-if self.diamond {rows as f32 * w as f32 / 2.} else {w as f32 / 2.}, -(h as f32) / 2.)
        };
        let guide_drag = self.guide_input(ui, outer, area, guide_origin, guide_scale, first, whole, over_tools);
        let over_tools = over_tools || guide_drag || self.inline_text.is_some();
        let painter = ui.painter_at(area);
        painter.rect_filled(area, 0., Color32::from_gray(45));
        if response.hovered() && !over_tools && self.zoom_drag.is_none() {
            let scroll = ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta.y;
                input.smooth_scroll_delta = Vec2::ZERO;
                delta
            });
            if scroll != 0. {
                self.finish_stroke();
                let old = self.zoom;
                self.zoom = (old * (scroll * 0.003).exp()).clamp(self.minimum_zoom(), 64.);
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
        let move_tool = false;
        let moving = !over_tools && (response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
            || (response.is_pointer_button_down_on() && down && (move_tool || space || self.pan_drag)));
        if moving {
            self.finish_stroke();
            self.pan_drag = down;
            self.pan += ui.input(|i| i.pointer.delta());
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Move);
        } else if response.hovered() && !over_tools && (move_tool || space) {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Move);
        }
        let zoom_tool = self.tool == Tool::Zoom;
        if zoom_tool && !moving && !space {
            let out = ui.input(|i| i.modifiers.alt);
            if (!over_tools || self.zoom_drag.is_some())
                && let Some(pointer) = response.hover_pos().or_else(|| self.zoom_drag.and_then(|_| response.interact_pointer_pos()))
            {
                paint_zoom_cursor(ui, pointer, out);
            }
            if !over_tools && response.clicked_by(egui::PointerButton::Primary)
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let (_, initial, pan) = self.zoom_drag.unwrap_or((pointer, self.zoom, self.pan));
                let next = click_zoom(initial, out, 64.);
                let offset = pointer - area.center();
                self.pan = offset - (offset - pan) * (next / initial);
                self.zoom = next;
                ui.ctx().request_repaint();
            }
            if response.is_pointer_button_down_on() && down {
                if let Some(pointer) = response.interact_pointer_pos() {
                    if self.zoom_drag.is_none() && !over_tools {
                        self.finish_stroke();
                        self.zoom_drag = Some((pointer, self.zoom, self.pan));
                    }
                    if let Some((anchor, initial, pan)) = self.zoom_drag {
                        (self.zoom, self.pan) = drag_zoom(anchor, initial, pan, pointer.x - anchor.x, area.center(), self.minimum_zoom().min(initial));
                    }
                }
            }
        } else { self.zoom_drag = None; }
        if !down { self.zoom_drag = None; }
        let panning = zoom_tool || move_tool || moving || self.pan_drag || (space && response.hovered());
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
        if self.transform.is_none() {
            if self.tool == Tool::Brush && self.soft_brush {
                if (!over_tools && !moving) || response.context_menu_opened() {
                    response.context_menu(|ui| self.brush_style_menu(ui));
                }
            } else {
                self.selection_context(ui, &response, origin, scale, whole, false);
            }
        }
        let special = !self.elevated
            && !ui.input(|i| i.modifiers.alt)
            && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill);
        let transforming=!panning && !over_tools && self.transform_input(ui,&response,origin,scale);
        let editing_text = !panning && !over_tools && !transforming && self.transform.is_none()
            && self.tool == Tool::Move && response.double_clicked()
            && point.is_some_and(|(x, y)| self.inline_text_hit(egui::pos2(x as f32, y as f32), whole, false));
        if panning || over_tools || transforming || editing_text {
            self.last = None;
        } else if matches!(self.tool, Tool::Shape | Tool::Path | Tool::Pen) {
            self.last = None;
        } else if self.tool == Tool::Text {
            if self.rectangle_text_allowed() && response.hovered() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Text);
            }
            if response.clicked_by(egui::PointerButton::Primary) && let Some((x, y)) = point {
                self.text_tool_click(egui::pos2(x as f32, y as f32), whole, false);
            }
        } else if self.tool == Tool::Move && !self.elevated {
            self.layer_move_input(ui,&response,origin,scale,whole,false);
        } else if self.tool == Tool::Select && !self.elevated {
            self.selection_input(ui,&response,origin,scale,whole,false);
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
                    self.fill_region(point, whole);
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
                        self.finite_plane_hit(point.0, point.1, whole)
                    };
                    if let Some((tile, x, y)) = hit {
                        self.pick_visible_pixel(tile, x, y);
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
                                        self.hard_eraser_pixel(self.selected, x as u32, y as u32)
                                    } else {
                                        Rgba(self.color)
                                    },
                                );
                            }
                        }
                    } else {
                        self.plane_input_stroke(self.last.unwrap_or(point), point, whole, origin, scale);
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
                    origin + self.mapped_center(col as i32, row as i32, w, h) * scale;
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
                if self.grid || self.hex != 0 {
                    self.tile_outline(&painter, rect, Color32::from_white_alpha(85));
                }
            }
        }
        self.rectangle_outline(&painter, origin, scale, false);
        if self.transform.is_none(){self.selection_outline(&painter,origin,scale);}
        self.subgroup_outline(&painter,origin,scale);
        if !self.plain_image && self.grid && scale >= 6. {
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
        if self.hex == 0 && !self.plain_image { painter.add(egui::Shape::closed_line(
            corners.map(|p| origin + p * scale).to_vec(),
            egui::Stroke::new(1_f32, Color32::YELLOW),
        )); }
        if self.elevated {
            let foot = egui::vec2(self.footprint[0] as f32, self.footprint[1] as f32) * scale;
            Self::outline(
                &painter,
                Rect::from_center_size(visible.center_bottom() - egui::vec2(0., foot.y / 2.), foot),
                true,
                Color32::LIGHT_BLUE,
            );
        }
        self.transform_outline(&painter,origin,scale);
        self.guides.paint(ui, outer, area, visible.min, scale);
        if self.show_slots && self.has_multiple_slots() {
            for row in 0..rows {
                for col in 0..cols {
                    let center = origin + self.mapped_center(col as i32, row as i32, w, h) * scale;
                    let rect = Rect::from_center_size(center, egui::vec2(w as f32, h as f32) * scale);
                    self.slot_label(&painter, rect, col as i32, row as i32);
                }
            }
        }
        if !over_tools {
            self.paint_move_alignment(&painter, origin, scale);
        }
        if !over_tools && !transforming && self.transform.is_none() {
            self.brush_cursor(ui, &response, origin, scale);
        }
        self.inline_text_canvas(ui, area, origin, scale);
        let vector_cell = if self.plain_image { visible } else { self.vector_cell(origin, scale, whole) };
        self.vector_canvas(ui, &response, &painter, vector_cell, scale, panning || over_tools || transforming);
        self.vector_confirmation(ui);
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
        let mut p: Project = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        p.validate()?;
        if p.version < 10 {
            for layer in &mut p.layers { layer.settings.effects.legacy_appearance = layer.settings.has_effects(); }
        }
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
        if count * p.width as usize * p.height as usize * 4 * p.layers.len().max(2)+count*p.width as usize*p.height as usize*p.layers.iter().filter(|l|l.settings.mask.is_some()).count() > LAYER_BYTES {
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
        self.material_drop_status=None;
        self.transform=None;
        self.text_dialog=None;
        self.guides.clear();
        self.inline_text = None;
        self.materials = p.materials.into_iter().map(|m| Material { name: m.name, pixels: RgbaImage::from_raw(p.width,p.height,m.pixels).expect("validated material") }).collect();
        self.layers = p
            .layers
            .into_iter()
            .map(|l| Layer { settings:l.settings, material: l.material,
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
        self.cleanup_single_links();
        self.active_layer = p.active_layer;self.selected_layers=vec![self.active_layer];self.layer_anchor=self.active_layer;
        self.fx_light = p.fx_light;
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
        self.split_preview = false;
        self.split_grid = [1, 1];
        self.diamond = p.diamond || p.elevated;
        self.hex = p.hex;
        self.elevated = p.elevated;
        self.footprint = p.footprint;
        self.wrap = p.wrap;
        self.edit_unit = p.edit_unit;
        self.brush_size = p.brush_size;
        self.soft_brush = p.soft_brush;
        self.rectangle_fill=p.rectangle_fill;
        self.brush_hardness = p.brush_hardness.min(100);
        self.selection.clear();
        self.mask_editing=false;self.layer_drag=None;
        self.brush_mode = p.brush_mode;
        self.pressure_size = p.pressure_size;
        self.pressure_opacity = p.pressure_opacity;
        self.pressure_min_size = p.pressure_min_size.min(100);
        self.pen = Default::default();
        self.brush_flow=p.brush_flow.min(100);
        self.brush_spacing=p.brush_spacing.clamp(1,100);
        self.pressure_flow=p.pressure_flow;
        self.subgroup_enabled=p.subgroup_enabled;
        self.subgroup_size=p.subgroup_size.map(|v|v.clamp(1,8));
        if self.columns%self.subgroup_size[0] as usize!=0 || self.rows%self.subgroup_size[1] as usize!=0 {self.subgroup_size=[1,1];}
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
        self.changed = false;
        self.trim_history();
        self.project_target = Some((path.to_owned(), bitmap_workspace::FileVersion::from_bytes(&bytes)));
        self.message = path.display().to_string();
        self.wake = true;
        self.confirm_close = false;
        Ok(())
    }
    fn save_tile_project(&mut self, source: &Path, choose_path: bool) {
        self.finish_stroke();
        let path = match &self.project_target {
            Some((path, _)) if !choose_path => {
                if !self.changed {
                    return;
                }
                path.clone()
            }
            target => {
                let mut dialog = rfd::FileDialog::new()
                    .set_file_name("tiles.uytile")
                    .add_filter(tr("绘Tile"), &["uytile"]);
                if let Some((path, _)) = target {
                    dialog = dialog.set_file_name(path.file_name().unwrap_or_default().to_string_lossy());
                    if let Some(parent) = path.parent() {
                        dialog = dialog.set_directory(parent);
                    }
                }
                let Some(path) = dialog.save_file() else { return };
                path
            }
        };
        if same_file(&path, source) && !super::is_tile_project(source) {
            self.message = tr("请另存到其他文件，保留源图片。").into();
            return;
        }
        let state = self.snapshot();
        let project = Project {
            version: if state.layers.iter().any(|l| l.settings.text.is_some()) {12} else if state.layers.iter().any(|l| l.settings.effects.has_extra()) {11} else {10},
            fx_light: self.fx_light,
            materials: self.materials.iter().map(|m| StoredMaterial { name:m.name.clone(),pixels:m.pixels.as_raw().clone() }).collect(),
            background_custom: self.background_custom,
            background_color: self.background_color,
            layers: state
                .layers
                .iter()
                .map(|l| StoredLayer { settings:l.settings.clone(), material: l.material,
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
            hex: self.hex,
            elevated: self.elevated,
            footprint: self.footprint,
            wrap: self.wrap,
            edit_unit: self.edit_unit,
            brush_size: self.brush_size,
            soft_brush: self.soft_brush,
            rectangle_fill:self.rectangle_fill,
            brush_hardness: self.brush_hardness,
            brush_mode: self.brush_mode,
            pressure_size: self.pressure_size,
            pressure_opacity: self.pressure_opacity,
            pressure_min_size: self.pressure_min_size,
            subgroup_enabled: self.subgroup_enabled,
            subgroup_size: self.subgroup_size,
            brush_flow:self.brush_flow,
            brush_spacing:self.brush_spacing,
            pressure_flow:self.pressure_flow,
        };
        let expected = self.project_target.as_ref()
            .filter(|(current, _)| same_file(current, &path))
            .map(|(_, version)| version);
        let result = serde_json::to_vec(&project)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                if bytes.len() > 256 * 1024 * 1024 {
                    return Err(tr("无效的 Tile 工程或超出尺寸限制。").into());
                }
                bitmap_workspace::write_image(&path, expected, |out| {
                    std::io::Write::write_all(out, &bytes).map_err(|error| error.to_string())
                })
            });
        match result {
            Ok(version) => {
                self.message = format!("{}: {}", tr("已保存"), path.display());
                self.project_target = Some((path, version));
                self.changed = false;
            }
            Err(error) => self.message = error,
        }
    }
    fn project_controls(&mut self, ui: &mut egui::Ui, source_path: &Path) {
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(self.changed || self.project_target.is_none(), egui::Button::new(tr("保存"))).clicked() {
                self.save_tile_project(source_path, false);
            }
            if ui.button(tr("另存为")).clicked() {
                self.save_tile_project(source_path, true);
            }
            if ui.button(tr("打开 Tile 工程")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("绘Tile"), &["uytile"])
                    .pick_file()
                {
                    if super::is_bitmap(source_path)||super::is_psd(source_path){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("image_open_tile_project"),path));}else{self.request_open_project(path);}
                }
            }
        });
        if let Some(path) = self.pending_open.clone() {
            let decision = editor_confirmation(
                ui,
                egui::Modal::new(ui.id().with("tile_confirm_open")),
                tr("打开 Tile 工程"),
                tr("当前有未保存的修改，打开工程将替换当前编辑内容。是否继续？"),
                tr("取消"),
                tr("放弃修改并打开"),
                |ui| { ui.add(egui::Label::new(path.display().to_string()).wrap().halign(egui::Align::Center)); },
            );
            if let Some(accept) = decision {
                self.pending_open = None;
                if accept && let Err(error) = self.load_project(&path) {
                    self.message = error;
                }
            }
        }
    }
    fn request_open_project(&mut self,path:std::path::PathBuf){self.finish_stroke();if self.changed{self.pending_open=Some(path);}else if let Err(error)=self.load_project(&path){self.message=error;}}
    fn independent_slots(&mut self) -> Result<(), String> {
        self.selection.clear();
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
            && count * self.layers.len() * self.pixels.as_raw().len()+self.layers.iter().map(|l|l.settings.bytes()).sum::<usize>()+duplicates.iter().map(|slot|self.layers.iter().filter_map(|l|l.settings.mask.as_ref()).map(|m|m[self.layout[*slot]].len()).sum::<usize>()).sum::<usize>() > LAYER_BYTES
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
                if let Some(mask)=layer.settings.mask.as_mut(){mask.push(mask[source].clone());}
            }
            let image = self.tiles[source].clone();
            self.layout[slot] = self.tiles.len();
            self.tiles.push(image);
        }
        self.invalidate_textures();
        Ok(())
    }
    fn set_preview_layout(&mut self, multiple: bool, columns: usize, rows: usize) {
        let previous = (self.multiple, self.columns, self.rows);
        if previous == (multiple, columns, rows) { return; }
        self.finish_stroke();
        (self.multiple, self.columns, self.rows) = (multiple, columns, rows);
        if multiple && let Err(error) = self.independent_slots() {
            (self.multiple, self.columns, self.rows) = previous;
            self.message = error;
            return;
        }
        // Refresh the project preview while leaving document edits and undo history alone.
        self.revision = self.revision.wrapping_add(1);
    }
    fn has_multiple_slots(&self)->bool {self.multiple && (self.columns>1 || self.rows>1)}
    fn tile_controls(&mut self, ui: &mut egui::Ui, source_path: &Path, source: Option<&RgbaImage>) {
        if self.multiple && !self.split_preview {
            let row = (self.selected_slot / 8).min(self.rows - 1);
            let col = (self.selected_slot % 8).min(self.columns - 1);
            let clamped = row * 8 + col;
            let slot = if self.layout[clamped] == self.selected { clamped } else {
                (0..self.rows)
                    .flat_map(|r| (0..self.columns).map(move |c| r * 8 + c))
                    .find(|&slot| self.layout[slot] == self.selected)
                    .unwrap_or(clamped)
            };
            self.select_slot(slot);
            if self.has_multiple_slots() {
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
                                    egui::Button::new((row*self.columns+col+1).to_string())
                                        .selected(self.selected_slot == slot),
                                )
                                .on_hover_text(format!("{},{}",row+1,col+1))
                                .clicked()
                            {
                                self.select_slot(slot);
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.label(format!(
                "{} {}",
                tr("当前 Slot"),
                (self.selected_slot / 8)*self.columns+self.selected_slot % 8+1
            ));
            }
        }
        if !self.split_preview || !self.has_multiple_slots() {
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(source.is_some(), egui::Button::new(tr("导入当前图片"))).clicked()
                    && let Some(image) = source {
                    self.import_current_cell(image);
                }
                let label = if self.has_multiple_slots() { "导出当前格 PNG" } else { "另存 PNG" };
                if ui.button(tr(label)).clicked() { self.export_tile_png(source_path, false); }
            });
        }
        if self.has_multiple_slots() && !self.split_preview {
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("拆分导出全部格 PNG")).clicked() { self.export_all_cells(); }
                if ui.button(tr("合并导出 PNG")).clicked() { self.export_tile_png(source_path, true); }
            });
        }
        self.material_controls(ui);
    }
    fn center_unit_focus(&self) -> Vec2 {
        let (w,h)=self.pixels.dimensions();
        if self.elevated {
            let (cols,rows)=if self.multiple {(self.columns,self.rows)} else {(1,1)};
            return egui::vec2((cols as f32-rows as f32)*self.footprint[0] as f32/4.,(cols+rows) as f32*self.footprint[1] as f32/4.-h as f32/2.);
        }
        let min=egui::vec2(-if self.diamond && self.whole_unit() { self.rows as f32*w as f32/2. } else { w as f32/2. },-(h as f32)/2.);
        min+self.edit_size()/2.
    }
    fn preview_display_scale(&self,dpi:f32,footprint:[u32;2],height:u32)->f32 {
        if self.rendering_merged { self.repeat_zoom/dpi } else { repeat_scale(self.repeat_zoom,dpi,footprint,height) }
    }
    fn merged_canvas(&mut self,ui:&mut egui::Ui) {
        if !self.infinite_view && !self.elevated { self.canvas(ui); return; }
        let saved=(self.repeat_zoom,self.repeat_pan,self.preview_grid,self.repeat_pan_drag,self.repeat_zoom_drag);
        self.repeat_zoom=self.zoom; self.repeat_pan=self.pan; self.preview_grid=self.grid;
        self.repeat_pan_drag=self.merged_pan_drag; self.repeat_zoom_drag=self.merged_zoom_drag;
        self.rendering_merged=true;
        self.preview(ui,true);
        self.rendering_merged=false;
        self.zoom=self.repeat_zoom; self.pan=self.repeat_pan;
        self.merged_pan_drag=self.repeat_pan_drag; self.merged_zoom_drag=self.repeat_zoom_drag;
        (self.repeat_zoom,self.repeat_pan,self.preview_grid,self.repeat_pan_drag,self.repeat_zoom_drag)=saved;
    }
    fn preview(&mut self, ui: &mut egui::Ui, editable: bool) {
        let (outer, allocation) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ui.available_height().max(80.)),
            egui::Sense::hover(),
        );
        let area = if self.rendering_merged { self.guides.area(outer) } else { outer };
        let response = ui.interact(area, allocation.id.with("preview"), egui::Sense::click_and_drag());
        let over_tools = if self.rendering_merged {
            let tools=self.floating_tools(ui,area);
            self.toolbar.dragging || ui.input(|i|i.pointer.hover_pos()).is_some_and(|p|tools.contains(p))
        } else { false };
        let (width, height) = self.pixels.dimensions();
        let footprint = if self.elevated { self.footprint } else { [width, height] };
        let dpi = ui.ctx().pixels_per_point();
        let guide_scale = repeat_scale(self.repeat_zoom, dpi, footprint, height);
        let guide_focus = if self.rendering_merged { self.center_unit_focus() } else { Vec2::ZERO };
        let guide_offset = guide_focus - self.edit_size() * 0.5;
        let guide_origin = area.center() + self.repeat_pan
            - guide_focus * self.preview_display_scale(dpi, footprint, height)
            + guide_offset * guide_scale;
        let guide_drag = self.rendering_merged && self.guide_input(
            ui, outer, area, guide_origin, guide_scale, guide_offset, true, over_tools,
        );
        let over_tools = over_tools || guide_drag || (self.rendering_merged && self.inline_text.is_some());
        let maximum = if self.rendering_merged { 64. } else { 8. };
        if response.hovered() && !over_tools && self.repeat_zoom_drag.is_none() {
            let scroll = ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta.y;
                input.smooth_scroll_delta = Vec2::ZERO;
                delta
            });
            if scroll != 0. {
                if editable {
                    self.finish_stroke();
                }
                self.repeat_zoom = (self.repeat_zoom * (scroll * 0.003).exp()).clamp(0.25, maximum);
                ui.ctx().request_repaint();
            }
        }
        let down = ui.input(|i| i.pointer.primary_down());
        let space = ui.input(|i| i.key_down(egui::Key::Space)) && !Self::text_input_active(ui.ctx());
        let moving = !over_tools && (response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
            || (response.is_pointer_button_down_on() && down
                && (space || self.repeat_pan_drag)));
        if moving {
            if editable { self.finish_stroke(); }
            self.preview_last = None;
            self.repeat_pan_drag = down;
            self.repeat_pan += ui.input(|i| i.pointer.delta());
            self.repeat_pan = self.repeat_pan.clamp(Vec2::splat(-1_000_000.), Vec2::splat(1_000_000.));
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Move);
        } else if response.hovered() && !over_tools && space {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Move);
        }
        let navigating = moving || self.repeat_pan_drag || space
            || self.tool == Tool::Zoom;
        if !down { self.repeat_pan_drag = false; }
        let (w, h) = (width, height);
        if self.tool == Tool::Zoom && !moving && !space {
            let out = ui.input(|i| i.modifiers.alt);
            if (!over_tools || self.repeat_zoom_drag.is_some())
                && let Some(pointer) = response.hover_pos().or_else(|| self.repeat_zoom_drag.and_then(|_| response.interact_pointer_pos()))
            {
                paint_zoom_cursor(ui, pointer, out);
            }
            if !over_tools && response.clicked_by(egui::PointerButton::Primary)
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let (_, initial, pan, before) = self.repeat_zoom_drag.unwrap_or((
                    pointer, self.repeat_zoom, self.repeat_pan, self.preview_display_scale(dpi, footprint, h),
                ));
                self.repeat_zoom = click_zoom(initial, out, maximum);
                let ratio = self.preview_display_scale(dpi, footprint, h) / before;
                let offset = pointer - area.center();
                self.repeat_pan = offset - (offset - pan) * ratio;
                ui.ctx().request_repaint();
            }
            if response.is_pointer_button_down_on() && down && (!over_tools || self.repeat_zoom_drag.is_some()) {
                if let Some(pointer) = response.interact_pointer_pos() {
                    if self.repeat_zoom_drag.is_none() {
                        if editable { self.finish_stroke(); }
                        self.repeat_zoom_drag = Some((pointer, self.repeat_zoom, self.repeat_pan,
                            self.preview_display_scale(dpi, footprint, h)));
                    }
                    if let Some((anchor, initial, pan, initial_scale)) = self.repeat_zoom_drag {
                        let dx = pointer.x - anchor.x;
                        self.repeat_zoom = (initial * (dx * 0.01).clamp(-20., 20.).exp()).clamp(0.25_f32.min(initial), maximum);
                        let ratio = repeat_scale(self.repeat_zoom, dpi, footprint, h) / initial_scale;
                        let offset = anchor - area.center();
                        self.repeat_pan = offset - (offset - pan) * ratio;
                        ui.ctx().request_repaint();
                    }
                }
            }
        } else { self.repeat_zoom_drag = None; }
        if !down { self.repeat_zoom_drag = None; }
        let scale = self.preview_display_scale(dpi, footprint, h);
        let origin = area.center() + self.repeat_pan - if self.rendering_merged { self.center_unit_focus()*scale } else { Vec2::ZERO };
        let p = ui.painter_at(area);
        self.paint_background(&p, area, area.min, Vec2::splat(1.));
        let scale = repeat_scale(self.repeat_zoom, dpi, footprint, h);
        let size = egui::vec2(footprint[0] as f32, footprint[1] as f32) * scale;
        let image_size = egui::vec2(w as f32, h as f32) * scale;
        let row_step = if self.diamond { size.y / 2. } else { size.y };
        let row_mid = ((area.center().y - origin.y) / row_step).floor() as i32;
        let row_min = ((area.top() - origin.y - size.y) / row_step).floor() as i32;
        let row_max = ((area.bottom() - origin.y + image_size.y) / row_step).ceil() as i32;
        let col_mid = ((area.center().x - origin.x) / size.x).floor() as i32;
        let cols = (((area.width() + image_size.x) / size.x / 2.).ceil() as i32 + 1).min(32);
        let mut instances = Vec::new();
        for row in row_min.max(row_mid - 32)..=row_max.min(row_mid + 32) {
            for col in col_mid - cols..=col_mid + cols {
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
                let center = origin + offset;
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
        if self.hex != 0 {
            instances.clear();
            let flat = self.hex == 2;
            let step = if flat { size.x * 0.75 } else { size.y * 0.75 };
            let mid = if flat { (area.center().x-origin.x)/step } else { (area.center().y-origin.y)/step };
            let count = ((if flat { area.width() } else { area.height() })/step/2.).ceil() as i32 + 2;
            for major in mid.floor() as i32-count.min(32)..=mid.floor() as i32+count.min(32) {
                let cross = (if flat { (area.center().y-origin.y)/size.y } else { (area.center().x-origin.x)/size.x }) - major as f32/2.;
                let extent = ((if flat { area.height()/size.y } else { area.width()/size.x })/2.).ceil() as i32 + 2;
                for minor in cross.floor() as i32-extent.min(32)..=cross.floor() as i32+extent.min(32) {
                    let (col,row) = if flat { (major,minor) } else { (minor,major) };
                    let center = origin + self.mapped_center(col,row,w,h)*scale;
                    let rect = Rect::from_center_size(center,image_size);
                    if rect.intersects(area) { instances.push((rect,rect,self.tile_at(col,row))); }
                }
            }
        }
        if self.rendering_merged && !self.infinite_view {
            let (cols,rows)=if self.multiple {(self.columns,self.rows)} else {(1,1)};
            instances.retain(|(_,ground,_)| {
                let world=(ground.center()-origin)/scale;
                let (col,row)=cell_at(world,footprint,self.diamond);
                col>=0 && row>=0 && col<cols as i32 && row<rows as i32
            });
        }
        let point = response
            .interact_pointer_pos()
            .filter(|v| area.contains(*v) && !over_tools)
            .map(|v| {
                (
                    ((v.x - origin.x) / scale).floor() as i32,
                    ((v.y - origin.y) / scale).floor() as i32,
                )
            });
        if editable && self.transform.is_none() {
            if self.tool == Tool::Brush && self.soft_brush {
                if (!over_tools && !moving) || response.context_menu_opened() {
                    response.context_menu(|ui| self.brush_style_menu(ui));
                }
            } else {
                self.selection_context(ui, &response, origin, scale, true, self.infinite_view);
            }
        }
        let special = editable
            && !self.elevated
            && !ui.input(|i| i.modifiers.alt)
            && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill);
        let transforming=editable && !navigating && !over_tools && self.transform_input(ui,&response,origin,scale);
        let editing_text = self.rendering_merged && !navigating && !over_tools && !transforming
            && self.transform.is_none() && self.tool == Tool::Move && response.double_clicked()
            && point.is_some_and(|(x, y)| self.inline_text_hit(egui::pos2(x as f32, y as f32), true, self.infinite_view));
        if navigating || over_tools || transforming || editing_text {
            self.preview_last = None;
        } else if matches!(self.tool, Tool::Shape | Tool::Path | Tool::Pen) {
            self.preview_last = None;
        } else if self.tool == Tool::Text {
            if self.rendering_merged && self.rectangle_text_allowed() && response.hovered() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Text);
            }
            if self.rendering_merged && response.clicked_by(egui::PointerButton::Primary)
                && let Some((x, y)) = point {
                self.text_tool_click(egui::pos2(x as f32, y as f32), true, self.infinite_view);
            }
        } else if editable && self.tool == Tool::Select && !self.elevated && !ui.input(|i|i.modifiers.alt) {
            self.selection_input(ui,&response,origin,scale,true,self.infinite_view);
        } else if special {
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
                    self.fill_region(point, true);
                }
            }
        } else if editable && !matches!(self.tool, Tool::Move | Tool::Zoom)
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
                            self.color = self.composite(tile).get_pixel(x, y).0;
                        }
                    } else {
                        self.plane_input_stroke(self.preview_last.unwrap_or(point), point, true, origin, scale);
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
                            let screen = origin + world * scale;
                            for (rect, _, index) in instances.iter().rev() {
                                if !rect.contains(screen) {
                                    continue;
                                }
                                let local = (screen - rect.min) / scale;
                                let (px, py) = (local.x.floor() as u32, local.y.floor() as u32);
                                let visible = picking.then(|| self.composite(*index));
                                let hit_pixels = visible.as_ref().unwrap_or_else(|| self
                                    .before
                                    .as_ref()
                                    .map_or(&self.tiles[*index], |s| &s.tiles[*index]));
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
                                self.pick_visible_pixel(index, px, py);
                            } else {
                                let color = if self.tool == Tool::Eraser {
                                    self.hard_eraser_pixel(index, px, py)
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
        for &(rect, ground, index) in &instances {
            p.image(
                self.textures[index].id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                Color32::WHITE,
            );
            if self.preview_grid {
                if self.elevated { Self::outline(&p,ground,self.diamond,Color32::from_white_alpha(65)); }
                else { self.tile_outline(&p, ground, Color32::from_white_alpha(65)); }
            }
        }
        if self.rendering_merged {
            let (cols,rows)=if self.multiple {(self.columns,self.rows)} else {(1,1)};
            for row in 0..rows { for col in 0..cols {
                let center=origin+self.mapped_center(col as i32,row as i32,footprint[0],footprint[1])*scale;
                let rect=Rect::from_center_size(center,size);
                if self.elevated { Self::outline(&p,rect,self.diamond,Color32::YELLOW); }
                else { self.tile_outline(&p,rect,Color32::YELLOW); }
            } }
        }
        self.rectangle_outline(&p, origin, scale, true);
        if self.transform.is_none(){self.selection_outline(&p,origin,scale);}
        self.subgroup_outline(&p,origin,scale);
        if editable {self.transform_outline(&p,origin,scale);}
        if self.rendering_merged {
            self.guides.paint(ui, outer, area, origin + guide_offset * scale, scale);
        }
        if self.show_slots && self.has_multiple_slots() {
            for &(_, ground, _) in &instances {
                let world = (ground.center() - origin) / scale;
                let (col, row) = if self.hex != 0 {
                    let cell = self.mapped_cell(world.x.floor() as i32, world.y.floor() as i32, w, h);
                    (cell.col, cell.row)
                } else {
                    cell_at(world, footprint, self.diamond)
                };
                self.slot_label(&p, ground, col, row);
            }
        }
        if editable && !over_tools && !navigating && !transforming && self.transform.is_none() {
            self.brush_cursor(ui, &response, origin, scale);
        }
        if self.rendering_merged {
            self.inline_text_canvas(ui, area, origin, scale);
            let vector_cell = self.vector_cell(origin, scale, self.multiple);
            self.vector_canvas(ui, &response, &p, vector_cell, scale, !editable || navigating || over_tools || transforming);
            self.vector_confirmation(ui);
        }
    }
    fn contextual_tool_options(&mut self,ui:&mut egui::Ui)->bool{
        let tool_icon=|ui:&mut egui::Ui,icon:crate::icons::Icon,label:&str|{let(rect,response)=ui.allocate_exact_size(egui::vec2(26.,24.),egui::Sense::hover());crate::icons::paint(ui,icon,rect.shrink(5.),ui.visuals().text_color());response.on_hover_text(tr(label));};
        match self.tool{
            Tool::Shape | Tool::Path | Tool::Pen => { ui.horizontal(|ui| self.vector_options(ui)); true },
            Tool::Move=>{ui.horizontal(|ui|{tool_icon(ui,crate::icons::Icon::Move,"移动");ui.checkbox(&mut self.move_auto_select,tr("自动选择"));ui.add_enabled_ui(self.move_auto_select,|ui|{ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=0.;for(group,label)in[(false,"图层"),(true,"组")]{let corners=egui::CornerRadius{nw:if group{0}else{4},sw:if group{0}else{4},ne:if group{4}else{0},se:if group{4}else{0}};if ui.add(egui::Button::new(tr(label)).selected(self.layer_move_group==group).corner_radius(corners)).clicked(){self.layer_move_group=group;}}});});self.transform_switch(ui);});true},
            Tool::Select=>{ui.horizontal(|ui|{for mode in selection::Mode::ALL{let response=ui.add_sized([26.,24.],egui::Button::new("").selected(self.selection.mode==mode));crate::icons::paint(ui,mode.icon(),response.rect.shrink(5.),ui.visuals().text_color());if response.on_hover_text(tr(mode.label())).clicked(){self.finish_stroke();self.selection.draft.clear();self.selection.mode=mode;}}if ui.add_enabled(self.selection.mask.is_some(),egui::Button::new(tr("取消选区"))).clicked(){self.selection.clear();}
                if let Some(color)=crate::color_property::show_editable(ui,tr("绘制颜色"),self.color.map(|v|v as f64/255.)){self.color=color.map(|v|(v.clamp(0.,1.)*255.).round()as u8);}
                ui.scope(|ui|{ui.spacing_mut().slider_width=100.;ui.label(tr("不透明度"));let mut opacity=self.color[3]as f32*100./255.;if ui.add(egui::Slider::new(&mut opacity,0. ..=100.).max_decimals(0)).changed(){self.color[3]=(opacity*255./100.).round()as u8;}ui.label("%");});
                let fill_label = if self.selection.mask.is_some() { "填充选区" } else { "填充图层" };
                if ui.add_enabled(self.layer_editable() && self.selection.draft.is_empty(), egui::Button::new(tr(fill_label))).on_hover_text("Alt+Delete").clicked(){self.fill_selection_color();}
                self.transform_switch(ui);
            });true},
            Tool::Eyedropper=>{ui.horizontal(|ui|{tool_icon(ui,crate::icons::Icon::Eyedropper,"吸管");ui.label(tr("取样所有可见图层"));let(rect,_)=ui.allocate_exact_size(egui::vec2(30.,20.),egui::Sense::hover());super::seamless::checker_background(ui.painter(),rect,rect.min,Vec2::splat(1.),&super::ImageBackground::Checkerboard,None);ui.painter().rect_filled(rect,0.,Color32::from_rgba_unmultiplied(self.color[0],self.color[1],self.color[2],self.color[3]));ui.label(format!("#{:02X}{:02X}{:02X} · {} {:.0}%",self.color[0],self.color[1],self.color[2],tr("不透明度"),self.color[3]as f32/255.*100.));});true},
            Tool::Zoom=>{ui.horizontal(|ui|{tool_icon(ui,crate::icons::Icon::Search,"缩放");ui.label(tr("左右拖拽缩放"));let value=format!("{:.2}",self.zoom*100.);egui::ComboBox::from_id_salt("tile_zoom_presets").width(90.).height(260.).selected_text(format!("{}%",value.trim_end_matches('0').trim_end_matches('.'))).show_ui(ui,|ui|{for percent in [25,50,75,100,150,200,300,400,500,800,1000,1600,3200,6400]{let next=percent as f32/100.;if ui.selectable_label((self.zoom-next).abs()<0.0001,format!("{percent}%")).clicked(){self.pan*=next/self.zoom.max(0.001);self.zoom=next;}}});});true},
            _=>false
        }
    }
    fn drawing_tools_height(ui: &egui::Ui) -> f32 {
        (ui.spacing().interact_size.y.max(26.) + 4.).ceil()
    }
    fn drawing_tools(&mut self,ui:&mut egui::Ui){
        let height=Self::drawing_tools_height(ui);
        let(rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),height),egui::Sense::hover());
        let mut options=ui.new_child(egui::UiBuilder::new().id_salt("tile_tool_options_row").max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        if self.transform.is_some(){self.transform_options(&mut options);}else{self.drawing_tool_contents(&mut options);}
    }
    fn brush_style_menu(&mut self, ui: &mut egui::Ui) {
        let mut hardness = self.brush_hardness;
        let mut pressure_size = self.pressure_size;
        let mut pressure_opacity = self.pressure_opacity;
        let mut pressure_min_size = self.pressure_min_size;
        let mut flow=self.brush_flow;
        let mut spacing=self.brush_spacing;
        let mut pressure_flow=self.pressure_flow;
        ui.set_min_width(270.);
        ui.label(tr("画笔样式"));
        if let Some(preset)=brush_presets::show(ui,hardness,pressure_size,pressure_opacity,pressure_flow){
            hardness=preset.hardness;pressure_size=preset.size;pressure_opacity=preset.opacity;pressure_flow=preset.flow;
        }
        ui.separator();
        ui.checkbox(&mut pressure_size, tr("压力控制大小"));
        ui.checkbox(&mut pressure_opacity, tr("压力控制不透明度"));
        ui.checkbox(&mut pressure_flow,tr("压力控制流量"));
        for (label,value,minimum) in [("流量",&mut flow,0),("笔触间距",&mut spacing,1)] {
            ui.label(tr(label));
            ui.scope(|ui|{ui.spacing_mut().slider_width=150.;crate::theme::slider(ui,true,egui::Slider::new(value,minimum..=100).suffix("%"));});
        }
        ui.add_enabled_ui(pressure_size, |ui| {
            ui.label(tr("最小直径"));
            crate::theme::slider(ui, true, egui::Slider::new(&mut pressure_min_size, 0..=100).suffix("%"));
        });
        ui.label(match self.pen.current {
            Some(value) => format!("{}: {:.0}%", tr("当前压力"), value * 100.),
            None => match self.pen.last_received {
                Some(value)=>format!("{}: {:.0}%",tr("当前无接触，上次压力"),value*100.),
                None=>tr("尚未收到压力，请用笔按压测试").to_owned(),
            },
        });
        ui.separator();
        ui.label(tr("硬度"));
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = 150.;
            ui.spacing_mut().interact_size.x = 48.;
            crate::theme::slider(ui, true, egui::Slider::new(&mut hardness, 0..=100).suffix("%"));
        });
        if (hardness, pressure_size, pressure_opacity, pressure_min_size,flow,spacing,pressure_flow) != (self.brush_hardness, self.pressure_size, self.pressure_opacity, self.pressure_min_size,self.brush_flow,self.brush_spacing,self.pressure_flow) {
            self.finish_stroke();
            self.brush_hardness = hardness;
            self.pressure_size = pressure_size;
            self.pressure_opacity = pressure_opacity;
            self.pressure_min_size = pressure_min_size;
            self.brush_flow=flow;self.brush_spacing=spacing;self.pressure_flow=pressure_flow;
        }
    }
    fn drawing_tool_contents(&mut self, ui: &mut egui::Ui) {
        self.pen.read(ui);
        if self.elevated && matches!(self.tool, Tool::Rectangle | Tool::Line | Tool::Fill | Tool::Select) {
            self.finish_stroke();
            self.tool = Tool::Brush;
        }
        if self.contextual_tool_options(ui){return;}
        ui.horizontal(|ui| {
            if matches!(self.tool,Tool::Brush|Tool::Eraser|Tool::Rectangle){ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                for (soft, label) in [(false, "铅笔"), (true, "画笔")] {
                    if ui.add(egui::Button::new(tr(label)).selected(self.soft_brush == soft).corner_radius(egui::CornerRadius { nw: if soft { 0 } else { 4 }, sw: if soft { 0 } else { 4 }, ne: if soft { 4 } else { 0 }, se: if soft { 4 } else { 0 } })).clicked() {
                        self.finish_stroke();
                        self.rectangle = None;
                        self.soft_brush = soft;
                    }
                }
            });}
            if self.tool == Tool::Rectangle {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    for (ellipse, label, _) in FRAME_MODES {
                        let label = if !ellipse && self.diamond && !self.plain_image { "菱形" } else { label };
                        let (left, right) = if ellipse { (0, 4) } else { (4, 0) };
                        if ui.add(egui::Button::new(tr(label)).selected(self.rectangle_ellipse == ellipse)
                            .corner_radius(egui::CornerRadius { nw: left, sw: left, ne: right, se: right })).clicked()
                        {
                            self.finish_stroke();
                            self.rectangle_ellipse = ellipse;
                        }
                    }
                });
            } else if matches!(self.tool, Tool::Line | Tool::Fill) {
                ui.label(tr(if self.tool == Tool::Line { "直线" } else { "填充" }));
            }
            if self.tool==Tool::Rectangle{ui.checkbox(&mut self.rectangle_fill,tr("填充"));}
            if self.tool!=Tool::Eraser{if let Some(color) = crate::color_property::show_editable(
                ui,
                tr("绘制颜色"),
                self.color.map(|v| f64::from(v) / 255.0),
            ) {
                self.color = color.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            }}
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = 100.0;
                let mut alpha = f32::from(self.color[3]) * 100.0 / 255.0;
                ui.label(tr("不透明度"));
                if ui
                    .add(egui::Slider::new(&mut alpha, 0.0..=100.0).max_decimals(0))
                    .changed()
                {
                    self.color[3] = (alpha * 255.0 / 100.0).round() as u8;
                }
                ui.label("%");
            });
            if self.tool==Tool::Fill||(self.tool==Tool::Rectangle&&self.rectangle_fill){return;}
            ui.add_enabled_ui(!self.elevated, |ui| {
                let (icon_rect, response) = ui.allocate_exact_size(Vec2::splat(22.), egui::Sense::hover());
                response.on_hover_text(format!("{}: {} px", tr("笔尖尺寸"), self.brush_size));
                if self.soft_brush {
                    let menu = ui.menu_button("  ", |ui| self.brush_style_menu(ui));
                    let arrow_rect = Rect::from_center_size(menu.response.rect.center(), Vec2::splat(12.));
                    crate::icons::paint(ui, crate::icons::Icon::TriangleDown, arrow_rect, ui.visuals().text_color());
                    menu.response.on_hover_text(tr("画笔样式"));
                }
                ui.spacing_mut().slider_width = 90.;
                ui.add(egui::Slider::new(&mut self.brush_size, 1..=32))
                    .on_hover_text(format!("{} ([ / ])", tr("笔尖尺寸")));
                if self.soft_brush && self.tool!=Tool::Eraser {
                    let mut mode = self.brush_mode;
                    ui.add_enabled_ui(self.tool != Tool::Eraser, |ui| {
                        egui::ComboBox::from_id_salt("tile_brush_mode")
                            .selected_text(tr(mode.label())).width(82.)
                            .show_ui(ui, |ui| {
                                for value in BrushMode::ALL { ui.selectable_value(&mut mode, value, tr(value.label())); }
                            }).response.on_hover_text(tr("画笔混合模式"));
                    });
                    if mode != self.brush_mode {
                        self.finish_stroke();
                        self.brush_mode = mode;
                    }
                }
                let painter = ui.painter();
                painter.rect_stroke(icon_rect.shrink(0.5), 3., ui.visuals().widgets.inactive.bg_stroke, egui::StrokeKind::Inside);
                let side = 2. + 16. * self.brush_size as f32 / 32.;
                if self.soft_brush{brush_presets::paint_current_tip(ui,icon_rect,side,self.brush_hardness);}
                else if self.brush_size <= 3 {
                    painter.rect_filled(Rect::from_center_size(icon_rect.center(), Vec2::splat(side)), 0., ui.visuals().text_color());
                } else {
                    painter.circle_filled(icon_rect.center(), side / 2., ui.visuals().text_color());
                }
            });
        });
    }
    fn slot_label(&self,painter:&egui::Painter,rect:Rect,col:i32,row:i32) {
        if !self.has_multiple_slots() || !self.show_slots || rect.width()<28. || rect.height()<16. || !painter.clip_rect().contains(rect.center()) { return; }
        let (cols,rows)=if self.multiple {(self.columns as i32,self.rows as i32)}else{(1,1)};
        let row=row.rem_euclid(rows);
        let col=col.rem_euclid(cols);
        let selected=!self.multiple || self.selected_slot == row as usize*8+col as usize;
        let text=(row*cols+col+1).to_string();
        let selection=painter.ctx().style().visuals.selection;
        let foreground=if selected {selection.stroke.color}else{Color32::WHITE};
        let galley=painter.layout_no_wrap(text,egui::FontId::proportional(if selected {14.}else{12.}),foreground);
        let label=Rect::from_center_size(rect.center(),galley.size()+egui::vec2(8.,4.));
        painter.rect_filled(label,3.,if selected {selection.bg_fill}else{Color32::from_black_alpha(180)});
        if selected { painter.rect_stroke(label,3.,egui::Stroke::new(2_f32,selection.stroke.color),egui::StrokeKind::Outside); }
        painter.galley(label.min+egui::vec2(4.,2.),galley,foreground);
    }
    fn floating_tools(&mut self, ui: &egui::Ui, area: Rect) -> Rect {
        let area=area.intersect(ui.clip_rect());
        if !area.is_positive() { return Rect::NOTHING; }
        let ctx = ui.ctx().clone();
        if !self.toolbar.loaded {
            self.toolbar.loaded=true;
            if let Some(path)=crate::last_open_dir_config_path().map(|p|p.with_file_name("pixel_tile_toolbar.json")) {
                if let Ok(bytes)=std::fs::read(path) {
                    if bytes.len()<=1024 {
                        if let Ok(saved)=serde_json::from_slice::<ToolbarSettings>(&bytes) {self.toolbar.offset=egui::vec2(saved.offset[0],saved.offset[1]);self.toolbar.horizontal=saved.horizontal;}
                        else if let Ok([x,y])=serde_json::from_slice::<[f32;2]>(&bytes) { self.toolbar.offset=egui::vec2(x,y); }
                    }
                }
            }
        }
        if self.toolbar.horizontal!=(self.toolbar.size.x>self.toolbar.size.y){self.toolbar.size=egui::vec2(self.toolbar.size.y,self.toolbar.size.x);}
        self.toolbar.offset=toolbar_offset_inside(self.toolbar.offset,self.toolbar.size,area);
        let was_dragging=self.toolbar.dragging;
        let horizontal=self.toolbar.horizontal;
        let layout=(if horizontal {egui::Layout::left_to_right(egui::Align::Center)}else{egui::Layout::top_down(egui::Align::Center)}).with_main_wrap(true);
        let tool_count = 9 + usize::from(self.rectangle_text_allowed()) + 3 * usize::from(self.vector_available());
        let length = 26. + tool_count as f32 * 29.;
        let available_length = if horizontal { area.width() } else { area.height() };
        let tools_per_line = ((available_length - 26.) / 29.).floor().max(1.) as usize;
        let lines = tool_count.div_ceil(tools_per_line);
        let breadth = 42. + (lines - 1) as f32 * 29.;
        let default_size=if horizontal{egui::vec2(length, breadth)}else{egui::vec2(breadth, length)}.min(area.size());
        let response=egui::Area::new(ui.id().with(("tile_floating_tools", ctx.viewport_id(),horizontal)))
            .default_size(default_size)
            .order(egui::Order::Foreground)
            .fixed_pos(area.min + self.toolbar.offset)
            .constrain_to(area)
            .movable(false)
            .show(&ctx, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(area));
                ui.set_min_size(Vec2::ZERO);ui.set_max_size(default_size);
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.with_layout(layout,|ui| {
                        ui.spacing_mut().item_spacing=egui::vec2(3.,3.);
                        let (handle,drag)=ui.allocate_exact_size(if horizontal {egui::vec2(12.,26.)} else {egui::vec2(26.,12.)},egui::Sense::click_and_drag());
                        let color=ui.visuals().text_color();
                        for offset in [-2.,2.] {
                            let delta=if horizontal {egui::vec2(offset,7.)}else{egui::vec2(7.,offset)};
                            let other=if horizontal {egui::vec2(offset,-7.)}else{egui::vec2(-7.,offset)};
                            ui.painter().line_segment([handle.center()+delta,handle.center()+other],egui::Stroke::new(1_f32,color));
                        }
                        if drag.dragged_by(egui::PointerButton::Primary) {
                            self.toolbar.dragging=true;
                            self.toolbar.offset+=ui.input(|i|i.pointer.delta());
                            ctx.request_repaint();
                        }
                        drag.context_menu(|ui| {
                            for (value,label) in [(false,"竖向排列"),(true,"横向排列")] {
                                if ui.selectable_label(self.toolbar.horizontal==value,tr(label)).clicked(){self.toolbar.horizontal=value;ui.close_menu();ctx.request_repaint();}
                            }
                        });
                        drag.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text(tr("拖动工具栏"));
                        let vector_mode = |tool| vector::PathTool::modes(tool)[self.path_tool.mode(tool)];
                        let (path_label, path_icon) = vector_mode(Tool::Path);
                        let (shape_label, shape_icon) = vector_mode(Tool::Shape);
                        let (pen_label, pen_icon) = vector_mode(Tool::Pen);
                        for (tool, icon, label, key) in [
                            (Tool::Move, crate::icons::Icon::Move, "移动", "V"),
                            (Tool::Select, self.selection.mode.icon(), self.selection.mode.label(), "M"),
                            (Tool::Brush, if self.soft_brush { crate::icons::Icon::Brush } else { crate::icons::Icon::Pencil }, if self.soft_brush { "画笔" } else { "铅笔" }, "B"),
                            (
                                Tool::Eyedropper,
                                crate::icons::Icon::Eyedropper,
                                "取色",
                                "I",
                            ),
                            (Tool::Eraser, crate::icons::Icon::Eraser, if self.soft_brush { "画笔橡皮擦" } else { "铅笔橡皮擦" }, "E"),
                            (
                                Tool::Rectangle,
                                FRAME_MODES[usize::from(self.rectangle_ellipse)].2,
                                "框画",
                                "K",
                            ),
                            (Tool::Line, crate::icons::Icon::LineTool, "直线", "L"),
                            (Tool::Fill, crate::icons::Icon::PaintBucket, "填充", "G"),
                            (Tool::Pen, pen_icon, pen_label, "P"),
                            (Tool::Shape, shape_icon, shape_label, "U"),
                            (Tool::Path, path_icon, path_label, "A"),
                            (Tool::Text, crate::icons::Icon::Type, "添加文字图层", "T"),
                            (Tool::Zoom, crate::icons::Icon::Search, "缩放", "Z"),
                        ] {
                            if (matches!(tool, Tool::Shape | Tool::Path | Tool::Pen) && !self.vector_available())
                                || (tool == Tool::Text && !self.rectangle_text_allowed()) {
                                continue;
                            }
                            let selected = self.transform.is_none() && self.tool == tool;
                            let visuals = ui.visuals().clone();
                            let enabled = !self.elevated
                                || !matches!(tool, Tool::Rectangle | Tool::Line | Tool::Fill | Tool::Select);
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
                            let color = if selected {
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
                                format!("{}({key})", tr(label))
                            } else {
                                format!("{} ({key})", tr(label))
                            };
                            let response=response.on_hover_text(tooltip);
                            let popup=ui.id().with(("tool_modes",tool as u8));
                            #[cfg(test)]
                            ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new(("tile_group_test",tool as u8)),(response.rect,popup)));
                            let vector_modes = vector::PathTool::modes(tool);
                            let grouped = matches!(tool, Tool::Select | Tool::Brush | Tool::Rectangle) || !vector_modes.is_empty();
                            if response.clicked() && self.finish_transform(true) {
                                self.finish_stroke();self.selection.draft.clear();
                                if selected && grouped {ui.memory_mut(|m|m.toggle_popup(popup));}
                                else {self.tool=tool;}
                            }
                            if grouped {
                                let mut menu_contents=|ui:&mut egui::Ui| {
                                    if tool==Tool::Select {
                                        for mode in selection::Mode::ALL {
                                            if tool_mode_button(ui,mode.icon(),tr(mode.label()),self.selection.mode==mode).clicked(){self.selection.mode=mode;self.selection.draft.clear();ui.memory_mut(|m|m.close_popup());}
                                        }
                                        ui.separator();if ui.button(tr("取消选区")).clicked(){self.selection.clear();ui.memory_mut(|m|m.close_popup());}
                                    }else if tool == Tool::Rectangle {
                                        for (ellipse, label, icon) in FRAME_MODES {
                                            let label = if !ellipse && self.diamond && !self.plain_image { "菱形" } else { label };
                                            if tool_mode_button(ui, icon, tr(label), self.rectangle_ellipse == ellipse).clicked() {
                                                self.rectangle_ellipse = ellipse;
                                                ui.memory_mut(|m| m.close_popup());
                                            }
                                        }
                                    }else if !vector_modes.is_empty() {
                                        for (index, &(label, icon)) in vector_modes.iter().enumerate() {
                                            if tool_mode_button(ui, icon, tr(label), self.path_tool.mode(tool) == index).clicked() {
                                                self.path_tool.set_mode(tool, index);
                                                ui.memory_mut(|m| m.close_popup());
                                            }
                                        }
                                    }else{
                                        for (soft,label,icon) in [(false,"铅笔",crate::icons::Icon::Pencil),(true,"画笔",crate::icons::Icon::Brush)] {
                                            if tool_mode_button(ui,icon,tr(label),self.soft_brush==soft).clicked(){self.soft_brush=soft;ui.memory_mut(|m|m.close_popup());}
                                        }
                                    }
                                };
                                if ui.memory(|m|m.is_popup_open(popup)) {
                                    let label_width = |label| ui.painter().layout_no_wrap(
                                        tr(label).to_owned(), egui::FontId::proportional(14.), ui.visuals().text_color()
                                    ).size().x;
                                    let text_width = match tool {
                                        Tool::Select => selection::Mode::ALL.map(|mode| label_width(mode.label())).into_iter().fold(0., f32::max),
                                        Tool::Rectangle => label_width("矩形").max(label_width("椭圆")),
                                        _ if !vector_modes.is_empty() => vector_modes.iter().map(|(label, _)| label_width(label)).fold(0., f32::max),
                                        _ => label_width("铅笔").max(label_width("画笔")),
                                    };
                                    let menu_width = (text_width + 48.).max(64.).min((area.width() - 20.).max(40.));
                                    let size_id=popup.with("last_size");
                                    let old_size=ui.ctx().data(|d|d.get_temp::<Vec2>(size_id)).unwrap_or(egui::vec2(menu_width+16.,180.));
                                    let size=egui::vec2(menu_width+16.,old_size.y).min(area.size());
                                    let pos=tool_popup_position(response.rect,area,size,horizontal);
                                    let menu=egui::Area::new(popup).kind(egui::UiKind::Popup).order(egui::Order::Foreground).fixed_pos(pos).constrain_to(area).show(ui.ctx(),|ui| {
                                        egui::Frame::popup(ui.style()).show(ui,|ui| {
                                            ui.set_width(menu_width);
                                            if vector_modes.is_empty() {
                                                egui::ScrollArea::both().auto_shrink([false,true]).max_width(menu_width).max_height((area.height()-16.).max(1.)).show(ui,menu_contents);
                                            } else {
                                                menu_contents(ui);
                                            }
                                        });
                                    });
                                    ui.ctx().data_mut(|d|d.insert_temp(size_id,menu.response.rect.size()));
                                    // Response::clicked() itself reads Context input. Never call
                                    // it while holding the input lock from Ui::input.
                                    let trigger_clicked=response.clicked();
                                    let menu_contains_pointer=menu.response.contains_pointer();
                                    let close=tool_popup_should_close(ui.ctx(),trigger_clicked,menu_contains_pointer);
                                    if close {ui.memory_mut(|m|m.close_popup());}
                                }
                            }
                        }
                    });
                });
            })
            .response
            .rect;
        self.toolbar.size=if horizontal==self.toolbar.horizontal{response.size()}else{egui::vec2(response.height(),response.width())};
        let clamped=toolbar_offset_inside(self.toolbar.offset,self.toolbar.size,area);
        if clamped!=self.toolbar.offset { self.toolbar.offset=clamped; ctx.request_repaint(); }
        if !ui.input(|i|i.pointer.primary_down()) { self.toolbar.dragging=false; }
        if (was_dragging && !self.toolbar.dragging) || horizontal!=self.toolbar.horizontal {
            if let Some(path)=crate::last_open_dir_config_path().map(|p|p.with_file_name("pixel_tile_toolbar.json")) {
                let result=(||->std::io::Result<()> {
                    if let Some(parent)=path.parent() {std::fs::create_dir_all(parent)?;}
                    let bytes=serde_json::to_vec(&ToolbarSettings{offset:[self.toolbar.offset.x,self.toolbar.offset.y],horizontal:self.toolbar.horizontal})?;
                    std::fs::write(path,bytes)
                })();
                if let Err(error)=result {self.message=format!("{}: {error}",tr("无法保存工具栏位置"));}
            }
        }
        response.intersect(area)
    }
    fn tile_sidebar(&mut self, ui: &mut egui::Ui, path: &Path, source: Option<&RgbaImage>) {
        self.ps_plugin.ui(ui);
        self.project_controls(ui, path);
        egui::CollapsingHeader::new(egui::RichText::new(tr("设置")).heading().strong()).id_salt("pixel_tile_settings_group").default_open(false).show(ui,|ui| {
        let (mut diamond,mut hex,mut elevated)=(self.diamond,self.hex,self.elevated);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("模式"));
            let mut mode = if hex != 0 { 2 } else if diamond { 1 } else { 0 };
            egui::ComboBox::from_id_salt("tile_shape_mode").selected_text(tr(["矩形","菱形","六边形"][mode])).show_ui(ui,|ui| {
                for (i,label) in ["矩形","菱形","六边形"].iter().enumerate() { ui.selectable_value(&mut mode,i,tr(label)); }
            });
            diamond = mode == 1;
            if mode == 2 { if hex == 0 { hex = 1; } elevated = false; }
            else { hex = 0; if mode == 0 { elevated = false; } }
            if hex != 0 {
                egui::ComboBox::from_id_salt("tile_hex_orientation").selected_text(tr(if hex == 1 {"尖顶"} else {"平顶"})).show_ui(ui,|ui| {
                    ui.selectable_value(&mut hex,1,tr("尖顶"));
                    ui.selectable_value(&mut hex,2,tr("平顶"));
                });
            }
        });
        if self.hex != 0 { ui.small(tr("六边形宽高为外接矩形尺寸，斜边自动计算。")); }
        self.set_preview_shape(diamond,hex,elevated);

        ui.horizontal_wrapped(|ui| {
            ui.label(tr("画布尺寸"));
            ui.add(egui::DragValue::new(&mut self.width).range(2..=MAX_SIDE));
            ui.label("×");
            ui.add(egui::DragValue::new(&mut self.height).range(2..=MAX_SIDE));
            if ui.button(tr("新建空白")).clicked() {
                self.reset_document(RgbaImage::new(
                    self.width.div_ceil(if self.hex == 2 {4} else {2}) * if self.hex == 2 {4} else {2},
                    self.height.div_ceil(if self.hex == 1 {4} else {2}) * if self.hex == 1 {4} else {2},
                ));
            }
        });
        let mut wrap = self.wrap;
        if ui
            .add_enabled(
                !self.elevated,
                egui::Checkbox::new(&mut wrap, tr("边缘回绕")),
            )
            .changed()
        {
            self.finish_stroke();
            self.wrap = wrap;
        }
        ui.checkbox(&mut self.preview_detached, tr("独立预览窗口"));
        self.tile_settings(ui);
        if self.elevated {
            ui.horizontal(|ui| {
                ui.label(tr("地面菱形尺寸"));
                for v in &mut self.footprint {
                    ui.add(egui::DragValue::new(v).range(2..=MAX_SIDE));
                    *v = (*v).div_ceil(2) * 2;
                }
            });
        }
        });
        self.edit_unit = true;
        self.tile_controls(ui,path,source);
        self.subgroup_controls(ui);
        self.layer_controls(ui);
        self.background_controls(ui);
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
    }
    fn set_preview_shape(&mut self, diamond: bool, hex: u8, elevated: bool) {
        if (self.diamond,self.hex,self.elevated) == (diamond,hex,elevated) { return; }
        if (hex == 1 && self.pixels.height()%4 != 0) || (hex == 2 && self.pixels.width()%4 != 0) {
            self.message = tr("尖顶六边形的高度、平顶六边形的宽度须为4的倍数；请先新建对应尺寸画布。").into();
            return;
        }
        self.finish_stroke();
        self.selection.clear();
        (self.diamond,self.hex,self.elevated) = (diamond,hex,elevated);
        self.pan = Vec2::ZERO;
        self.repeat_pan = Vec2::ZERO;
        self.invalidate_textures();
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn show(&mut self, ctx: &egui::Context, path: &Path, source: Option<&RgbaImage>) {
        self.show_window(ctx,path,source,None,false,false);
    }
    fn show_window(&mut self,ctx:&egui::Context,path:&Path,source:Option<&RgbaImage>,mut tab:Option<&mut bool>,other_dirty:bool,unavailable:bool){
        if !self.open {
            return;
        }
        if self.elevated {
            let mut open=true;
            crate::native_window::Window::new(tr("绘Tile"))
                .id(egui::Id::new(("pixel_tile_height_readonly",path))).open(&mut open)
                .default_width(1000.).default_height(700.).scrollable(false)
                .show(ctx,|ui|{ui.label(tr("高度Tile本版暂不支持编辑，仅提供只读预览。"));self.preview(ui,false);});
            if !open{self.open=false;}
            return;
        }
        if !self.plain_image {self.ensure_material_base();}
        if self.wake {
            crate::native_window::wake(ctx, egui::Id::new(("pixel_tile", path)));
            self.wake = false;
        }
        let revision = self.revision;
        let view_state = (self.preview_detached, self.repeat_zoom, self.preview_grid, self.repeat_pan);
        let mut open = true;
        let min_zoom=self.minimum_zoom();
        let display_path = if self.plain_image {
            self.image_target.as_deref().unwrap_or(path)
        } else {
            self.project_target.as_ref().map_or(path, |(target, _)| target.as_path())
        };
        let mut title = if tab.is_some(){format!("{} · {}",if self.plain_image {"Image"} else {"Tile"},display_path.file_name().unwrap_or_default().to_string_lossy())}else if display_path.as_os_str().is_empty() {
            tr("绘Tile").to_owned()
        } else {
            format!(
                "{} · {}",
                tr("绘Tile"),
                display_path.file_name().unwrap_or_default().to_string_lossy()
            )
        };
        if self.changed {
            title.push('*');
        }
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
                if ui.input(|i| i.viewport().close_requested()) { self.finish_transform(true); self.finish_stroke(); }
                if (self.changed || other_dirty) && ui.input(|i| i.viewport().close_requested()) {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.confirm_close = true;
                }
                if !unavailable && !self.confirm_close {
                    self.confirm_inline_text_outside(ui);
                    if self.transform.is_none() && self.text_dialog.is_none() && !Self::text_input_active(ui.ctx()) {
                        let save_as=ui.input_mut(|i|i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT,egui::Key::S));
                        let save=ui.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::S));
                        if self.plain_image {
                            if save_as || (save && !self.image_readonly) {
                                self.save_image(path, save_as);
                            }
                        } else if save_as || save {
                            self.save_tile_project(path, save_as);
                        }
                    }
                    if !self.showing_split_guides() { self.tool_shortcuts(ui.ctx()); }
                }
                let area = ui.available_rect_before_wrap();
                let gap = 8.;
                let left_width = 270_f32.min((area.width() - 400.).max(200.));
                let left = Rect::from_min_size(area.min, egui::vec2(left_width, area.height()));
                let center=Rect::from_min_max(egui::pos2(left.right()+gap,area.top()),area.max);
                let divider_x = left.right() + gap * 0.5;
                ui.painter().line_segment(
                    [egui::pos2(divider_x, area.top()), egui::pos2(divider_x, area.bottom())],
                    ui.visuals().widgets.inactive.bg_stroke,
                );
                let mut settings = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tile_settings")
                        .max_rect(left),
                );
                if let Some(mode)=tab.as_deref_mut(){
                    let mut selection=if *mode{"tile"}else{"image"}.to_owned();
                    let changed=settings.add_enabled_ui(!self.confirm_close && self.pending_open.is_none() && self.text_dialog.as_ref().is_none_or(|d|d.target.is_some()),|ui|crate::theme::compact_selector(ui,&mut selection,[("image","Image"),("tile","Tile")])).inner;
                    if changed && self.finish_transform(true){*mode=selection=="tile";return;}
                    settings.separator();
                }
                if self.transform.is_some(){settings.disable();}
                settings.set_clip_rect(left.intersect(ui.clip_rect()));
                egui::ScrollArea::vertical()
                    .id_salt("tile_settings_scroll")
                    .auto_shrink([false, false])
                    .show(&mut settings, |ui| {
                        if unavailable {ui.label(&self.message);}
                        else if self.plain_image {self.image_controls(ui,path);}
                        else {self.tile_sidebar(ui,path,source);}
                    });
                if !unavailable && !self.confirm_close
                    && self.pending_open.is_none() && self.transform.is_none() && self.text_dialog.as_ref().is_none_or(|d|d.target.is_some()) {
                    self.layer_file_drop(ui,area);
                }
                if unavailable && !self.confirm_close {return;}
                self.sync_texture(ui.ctx());
                let mut paint = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tile_paint")
                        .max_rect(center),
                );
                paint.set_clip_rect(center.intersect(ui.clip_rect()));
                if self.showing_split_guides() { self.split_canvas(&mut paint); } else {
                let header_height=24_f32.max(paint.text_style_height(&egui::TextStyle::Heading));
                let color_height = header_height + paint.spacing().item_spacing.y + Self::drawing_tools_height(&paint);
                paint.horizontal_top(|ui| {
                self.drawing_colors(ui, color_height);
                ui.vertical(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(),header_height),
                    egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),|ui| {
                    ui.spacing_mut().interact_size.y=header_height;
                    ui.heading(tr("绘制区"));
                    if !self.plain_image {ui.push_id("tile_boundary_tabs",|ui| {
                        let mut mode=if self.infinite_view {"infinite"}else{"bounded"}.to_owned();
                        if crate::theme::compact_selector(ui,&mut mode,[("bounded","有边界"),("infinite","无边界")]) {
                            self.set_infinite_view(mode=="infinite");
                        }
                    }).response.on_hover_text("Q / W");}
                    if self.infinite_view {
                        let locate=ui.add_sized([24.,24.],egui::Button::new(""));
                        crate::icons::paint(ui,crate::icons::Icon::Locate,locate.rect.shrink(5.),ui.visuals().text_color());
                        if locate.on_hover_text(format!("{} (H)",tr("回到中心循环单元"))).clicked() {self.center_view();}
                    }
                    if !self.plain_image {
                        ui.checkbox(&mut self.grid, tr("网格"));
                    }
                    ui.checkbox(&mut self.guides.rulers, tr("标尺")).on_hover_text("Ctrl+R");
                    ui.checkbox(&mut self.guides.visible, tr("辅助线")).on_hover_text("Ctrl+;");
                    if self.has_multiple_slots() { ui.checkbox(&mut self.show_slots,tr("显示 Slot")); }
                    if ui.button(tr("适应画布")).clicked() {
                        self.pan = Vec2::ZERO;
                        let size = self.edit_size();
                        self.zoom = ((center.width() - 20.) / size.x)
                            .min((ui.ctx().screen_rect().height() - 70.) / size.y)
                            .max(min_zoom)
                            * ui.ctx().pixels_per_point();
                    }

                    let mut zoom_percent = self.zoom * 100.;
                    if ui.add(egui::DragValue::new(&mut zoom_percent)
                        .range(min_zoom*100.0..=6400.0).speed(25.0).max_decimals(2).suffix("%"))
                        .changed() {
                        self.zoom = zoom_percent / 100.;
                    }

                    if ui
                        .add_enabled(self.transform.is_none() && !self.undo.is_empty(), egui::Button::new(tr("撤销")))
                        .on_hover_text(format!("{} (Ctrl+Z)", tr("撤销")))
                        .clicked()
                    {
                        self.history(false);
                    }
                    if ui
                        .add_enabled(self.transform.is_none() && !self.redo.is_empty(), egui::Button::new(tr("重做")))
                        .on_hover_text(format!("{} (Ctrl+Y)", tr("重做")))
                        .clicked()
                    {
                        self.history(true);
                    }
                });
                if !unavailable {self.drawing_tools(ui);}
                });
                });
                if !unavailable {self.merged_canvas(&mut paint);}
                }
                if !self.confirm_close {self.text_dialog_ui(ui);}
                if !self.confirm_close { self.transform_exit_confirmation(ui); }
                if self.confirm_close {
                    let modal_id = egui::Id::new(("pixel_tile_close", path));
                    let modal = egui::Modal::new(modal_id)
                        .area(egui::Modal::default_area(modal_id).anchor(
                            egui::Align2::CENTER_CENTER,
                            area.center() - ui.ctx().screen_rect().center(),
                        ));
                    let decision = editor_confirmation(
                        ui,
                        modal,
                        tr(if tab.is_some() { "编辑图像" } else { "绘Tile" }),
                        tr(if tab.is_some() && other_dirty {
                            "其他编辑页也有未保存的绘制，是否放弃并关闭整个窗口？"
                        } else {
                            "有未保存的绘制，是否关闭？"
                        }),
                        tr("继续编辑"),
                        tr("放弃修改并关闭"),
                        |_| {},
                    );
                    if let Some(discard) = decision {
                        self.confirm_close = false;
                        if discard {
                            *self = Self::default();
                            self.discard_requested = true;
                        }
                    }
                }
            });
        if self.open && open { self.fx_window(ctx,path); }
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
                        let mut zoom_percent = self.repeat_zoom * 100.;
                        if ui.add(egui::DragValue::new(&mut zoom_percent)
                            .range(25.0..=800.0).speed(10.0).max_decimals(2).suffix("%"))
                            .changed() {
                            self.repeat_zoom = zoom_percent / 100.;
                        }
                        ui.checkbox(&mut self.preview_grid, tr("网格"));
                    });
                    self.preview(ui, false);
                });
            self.preview_detached = preview_open;
        }
        if revision != self.revision
            || view_state != (self.preview_detached, self.repeat_zoom, self.preview_grid, self.repeat_pan)
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
            if self.changed || other_dirty {
                self.confirm_close = true;
                ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile", path))));
            } else {
                self.open = false;
                self.image_size_draft = None;
                self.transform=None;
                self.text_dialog=None;
                if self.layer_detail.as_ref().is_some_and(|d|d.1) { self.layer_detail = None; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rectangle_fill_respects_selection_opacity_and_one_undo(){let mut e=Editor::default();e.diamond=false;e.reset_document(RgbaImage::new(4,4));e.tool=Tool::Rectangle;e.rectangle_fill=true;e.color=[100,150,200,128];e.selection.mask=Some([(0,2,2)].into_iter().collect());let count=e.undo.len();e.rectangle_input(Some((-1,-1)),true,true,false,false);e.rectangle_input(Some((1,1)),false,false,false,false);assert_eq!(e.pixels.get_pixel(2,2).0,[100,150,200,128]);assert_eq!(e.pixels.pixels().filter(|p|p[3]>0).count(),1);assert_eq!(e.undo.len(),count+1);e.history(false);assert!(e.pixels.pixels().all(|p|p[3]==0));assert!(e.selection.mask.is_some());}
    #[test]
    fn rectangle_wrap_does_not_apply_opacity_repeatedly(){let mut e=Editor::default();e.diamond=false;e.reset_document(RgbaImage::new(4,4));e.wrap=true;e.color=[100,150,200,128];e.plane_filled_rectangle((-6,-2),(6,2),false,false,false);assert!(e.pixels.pixels().all(|p|p[3]==128));}
    #[test]
    fn layer_actions_apply_immediately_and_can_be_undone() {
        let mut editor = Editor::default();
        editor.diamond = false;
        editor.reset_document(RgbaImage::new(4, 4));
        editor.write_pixel(0, 1, 1, Rgba([100, 150, 200, 255]));
        let original = editor.pixels.clone();
        editor.request_layer_action(true, editor.active_layer);
        assert!(editor.pixels.pixels().all(|pixel| pixel[3] == 0));
        editor.history(false);
        assert_eq!(editor.pixels, original);
        assert!(editor.insert_empty_layer(true));
        let count = editor.layers.len();
        editor.request_layer_action(false, editor.active_layer);
        assert_eq!(editor.layers.len(), count - 1);
        editor.history(false);
        assert_eq!(editor.layers.len(), count);
    }
    #[test]
    fn opening_project_with_unsaved_changes_defers_loading(){let mut e=Editor::default();e.diamond=false;e.reset_document(RgbaImage::new(4,4));e.write_pixel(0,1,1,Rgba([80,100,120,255]));e.changed=true;let before=e.snapshot();let path=std::path::PathBuf::from("not-loaded-until-confirmed.uytile");e.request_open_project(path.clone());assert_eq!(e.pending_open,Some(path));assert!(e.snapshot()==before);e.pending_open=None;assert!(e.snapshot()==before);}
    #[test]
    fn copy_and_cut_layer_undo_redo_restore_selection_and_source_layer(){
        for cut in [false,true] {
            let mut e=tiny(1,1);e.ensure_material_base();e.write_pixel(0,0,0,image::Rgba([80,150,20,255]));
            e.selection.mask=Some([(0,0,0)].into_iter().collect());
            let source=e.active_layer;let count=e.layers.len();
            if cut{e.cut_selection_to_layer(source);}else{e.duplicate_layer(source,true);}
            e.history(false);assert_eq!(e.layers.len(),count);assert_eq!(e.active_layer,source);
            assert!(e.selection.mask.as_ref().unwrap().contains(&(0,0,0)));
            assert_eq!(e.pixels.get_pixel(0,0).0,[80,150,20,255]);
            e.history(true);assert_eq!(e.layers.len(),count+1);assert!(e.selection.mask.as_ref().unwrap().contains(&(0,0,0)));
            if cut{assert_eq!(e.layers[source].tiles[0].get_pixel(0,0)[3],0);}
        }
    }

    #[test]
    fn tile_flow_changes_build_up_without_exceeding_stroke_opacity() {
        let mut low=tiny(1,1);low.soft_brush=true;low.brush_size=9;low.brush_flow=10;low.color=[60,150,200,128];
        let mut high=tiny(1,1);high.soft_brush=true;high.brush_size=9;high.brush_flow=100;high.color=low.color;
        low.plane_stroke((-2,0),(2,0),false);high.plane_stroke((-2,0),(2,0),false);
        assert!(high.tiles.iter().flat_map(|t|t.pixels()).all(|p|p[3]<=128));
        let low_alpha:u32=low.tiles.iter().flat_map(|t|t.pixels()).map(|p|u32::from(p[3])).sum();
        let high_alpha:u32=high.tiles.iter().flat_map(|t|t.pixels()).map(|p|u32::from(p[3])).sum();
        assert!(low_alpha<high_alpha);
        let unchanged=low.tiles.clone();low.plane_stroke((2,0),(2,0),false);assert_eq!(low.tiles,unchanged);
        let mut zero=tiny(1,1);zero.soft_brush=true;zero.brush_flow=0;zero.plane_stroke((0,0),(3,0),false);
        assert!(zero.tiles.iter().all(|t|t.pixels().all(|p|p[3]==0)));
    }

    #[test]
    fn layer_via_cut_and_base_deletion_guards() {
        let mut e=tiny(1,1);let base=e.ensure_material_base().unwrap();
        let source=e.active_layer;e.write_pixel(0,0,0,image::Rgba([80,150,20,255]));
        e.write_pixel(0,1,0,image::Rgba([10,20,30,255]));
        e.selection.mask=Some([(0,0,0)].into_iter().collect());
        let count=e.layers.len();e.cut_selection_to_layer(source);
        assert_eq!(e.layers.len(),count+1);
        assert_eq!(e.pixels.get_pixel(0,0).0,[80,150,20,255]);
        assert_eq!(e.pixels.get_pixel(1,0)[3],0);
        assert_eq!(e.layers[source].tiles[0].get_pixel(0,0)[3],0);
        assert_eq!(e.layers[source].tiles[0].get_pixel(1,0).0,[10,20,30,255]);
        e.history(false);assert_eq!(e.layers.len(),count);
        assert_eq!(e.pixels.get_pixel(0,0).0,[80,150,20,255]);
        e.layers[base].locked=false;e.delete_layer(base);assert_eq!(e.layers.len(),count);
        e.selection.mask=Some([(0,0,0)].into_iter().collect());e.cut_selection_to_layer(base);assert_eq!(e.layers.len(),count);
    }

    #[test]
    fn active_group_click_opens_menu_without_reentering_input_lock() {
        use eframe::egui;
        for tool in [super::Tool::Select,super::Tool::Brush] {
            let ctx=egui::Context::default();let mut e=tiny(1,1);
            e.toolbar.loaded=true;e.tool=tool;
            let area=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(800.,600.));
            let mut draw=|events|{let _=ctx.run(egui::RawInput {screen_rect:Some(area),events,..Default::default()},|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|{e.floating_tools(ui,area);});
            });};
            draw(vec![]);draw(vec![]);
            let (rect,popup)=ctx.data(|d|d.get_temp::<(egui::Rect,egui::Id)>(egui::Id::new(("tile_group_test",tool as u8)))).unwrap();
            let pos=rect.center();
            draw(vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::NONE}]);
            draw(vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::NONE}]);
            assert!(ctx.memory(|m|m.is_popup_open(popup)));
            assert!(ctx.data(|d|d.get_temp::<egui::Vec2>(popup.with("last_size"))).unwrap().x<300.);
            draw(vec![]);
            assert!(ctx.memory(|m|m.is_popup_open(popup)));
        }
    }

    #[test]
    fn grouped_tool_popups_flip_and_stay_inside_canvas() {
        let area=eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO,eframe::egui::vec2(800.,600.));
        let button=eframe::egui::Rect::from_min_size(eframe::egui::pos2(760.,560.),eframe::egui::vec2(28.,28.));
        let size=eframe::egui::vec2(200.,160.);
        let side=super::tool_popup_position(button,area,size,false);
        assert!(side.x<button.left());
        let above=super::tool_popup_position(button,area,size,true);
        assert!(above.y<button.top());
        assert!(area.contains_rect(eframe::egui::Rect::from_min_size(side,size)));
        assert!(area.contains_rect(eframe::egui::Rect::from_min_size(above,size)));
    }
    #[test]
    fn m_cycles_only_when_selection_tool_is_active() {
        let ctx=eframe::egui::Context::default();let mut e=tiny(1,1);
        for expected in [super::selection::Mode::Rectangle,super::selection::Mode::Ellipse] {
            let _=ctx.run(eframe::egui::RawInput {events:vec![eframe::egui::Event::Key{key:eframe::egui::Key::M,physical_key:None,pressed:true,repeat:false,modifiers:eframe::egui::Modifiers::NONE}],..Default::default()},|ctx|e.tool_shortcuts(ctx));
            assert!(e.selection.mode==expected);
            let _=ctx.run(eframe::egui::RawInput {events:vec![eframe::egui::Event::Key{key:eframe::egui::Key::M,physical_key:None,pressed:false,repeat:false,modifiers:eframe::egui::Modifiers::NONE}],..Default::default()},|_|{});
        }
        e.tool=super::Tool::Brush;
        let _=ctx.run(eframe::egui::RawInput {events:vec![eframe::egui::Event::Key{key:eframe::egui::Key::M,physical_key:None,pressed:true,repeat:false,modifiers:eframe::egui::Modifiers::NONE}],..Default::default()},|ctx|e.tool_shortcuts(ctx));
        assert!(e.tool==super::Tool::Select && e.selection.mode==super::selection::Mode::Ellipse);
    }

    #[test]
    fn base_copy_selected_or_all_and_ordinary_copy_ignore_slot_scope() {
        let mut e = tiny(2, 1);
        let base = e.ensure_material_base().unwrap();
        e.layers[base].tiles[0].put_pixel(0,0, image::Rgba([10,20,30,255]));
        e.layers[base].tiles[1].put_pixel(0,0, image::Rgba([40,50,60,255]));
        e.selected = 1;
        e.pixels = e.tiles[1].clone();
        let count = e.layers.len();
        e.duplicate_layer(base, false);
        assert_eq!(e.layers.len(), count + 1);
        assert!(!e.layers[e.active_layer].locked && !e.layers[e.active_layer].material);
        assert_eq!(e.tiles[0].get_pixel(0,0)[3], 0);
        assert_eq!(e.tiles[1].get_pixel(0,0).0, [40,50,60,255]);
        e.history(false);
        assert_eq!(e.layers.len(), count);
        e.duplicate_layer(base, true);
        assert_eq!(e.tiles[0].get_pixel(0,0).0, [10,20,30,255]);
        let original = e.tiles.clone();
        e.duplicate_layer(e.active_layer, false);
        assert_eq!(e.tiles, original);
    }

    #[test]
    fn pressure_controls_do_not_change_pencil_or_mouse_parameters() {
        let mut e = tiny(1,1);
        e.brush_size = 20;
        e.pressure_size = true;
        e.pressure_opacity = true;
        assert_eq!(e.pressure_size(Some(0.25)), 20);
        assert_eq!(e.pressure_alpha(Some(0.25)), 1.);
        e.soft_brush = true;
        assert_eq!(e.pressure_size(None), 20);
        assert_eq!(e.pressure_alpha(None), 1.);
        assert_eq!(e.pressure_size(Some(0.25)), 5);
        assert_eq!(e.pressure_alpha(Some(0.25)), 0.25);
    }

    #[test]
    fn hard_brush_has_antialiased_edge_and_single_stroke_does_not_build_up() {
        assert!(super::brush_tip_coverage(10, 1, 1, 100) > 0.);
        assert!(super::brush_tip_coverage(10, 1, 1, 100) < 1.);
        let mut e = tiny(1, 1);
        e.soft_brush = true;
        e.brush_size = 5;
        e.color = [80, 180, 100, 128];
        e.plane_stroke((0,0), (0,0), false);
        let first = e.tiles.clone();
        e.plane_stroke((0,0), (0,0), false);
        assert_eq!(first, e.tiles);
        e.finish_stroke();
        e.plane_stroke((0,0), (0,0), false);
        assert_ne!(first, e.tiles);
    }

    #[test]
    fn picking_reads_visible_layers_beneath_an_empty_drawing_layer() {
        let mut e = tiny(1, 1);
        e.ensure_layers();
        let base = e.ensure_material_base().unwrap();
        e.layers[base].tiles[0].put_pixel(0, 0, image::Rgba([40, 150, 60, 255]));
        e.pick_visible_pixel(0, 0, 0);
        assert_eq!(e.color, [40, 150, 60, 255]);
        e.layers[base].visible = false;
        e.pick_visible_pixel(0, 0, 0);
        assert_eq!(e.color, [0; 4]);
    }

    #[test]
    fn brush_modes_handle_opaque_and_transparent_backdrops() {
        use super::{BrushMode, round_brush_blend};
        for mode in BrushMode::ALL {
            let paint = image::Rgba([200, 100, 50, 128]);
            assert_eq!(round_brush_blend(image::Rgba([0; 4]), paint, 1., false, mode), paint);
            assert_eq!(round_brush_blend(image::Rgba([80, 90, 100, 255]), paint, 0., false, mode).0, [80, 90, 100, 255]);
            assert_eq!(round_brush_blend(image::Rgba([80, 90, 100, 255]), paint, 1., true, mode).0, [80, 90, 100, 127]);
        }
        assert_eq!(round_brush_blend(image::Rgba([128, 128, 128, 255]), image::Rgba([128, 128, 128, 255]), 1., false, BrushMode::Multiply).0, [64, 64, 64, 255]);
        assert_eq!(round_brush_blend(image::Rgba([128, 128, 128, 255]), image::Rgba([128, 128, 128, 255]), 1., false, BrushMode::Screen).0, [192, 192, 192, 255]);
    }

    #[test]
    fn round_tip_hardness_controls_falloff_without_changing_bounds() {
        assert_eq!(super::round_tip_coverage(0., 0), 1.);
        assert!(super::round_tip_coverage(0.5, 0) < 0.4);
        assert_eq!(super::round_tip_coverage(0.5, 50), 1.);
        assert_eq!(super::round_tip_coverage(0.9, 100), 1.);
        assert_eq!(super::round_tip_coverage(1.1, 100), 0.);
        assert_eq!(super::round_tip_coverage(1., 0), 0.);
    }

    #[test]
    fn tile_preview_premultiplies_rgb_once_in_display_space() {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([80, 180, 100, 128]));
        let display = super::tile_preview_image(&source);
        assert_eq!(display.pixels[0].to_array(), [40, 90, 50, 128]);
        assert_eq!(source.get_pixel(0, 0).0, [80, 180, 100, 128]);
        let transparent = image::RgbaImage::from_pixel(1, 1, image::Rgba([80, 180, 100, 0]));
        assert_eq!(super::tile_preview_image(&transparent).pixels[0].to_array(), [0; 4]);
    }

    #[test]
    fn brush_alpha_preserves_color_on_transparency() {
        assert_eq!(super::round_brush_blend(image::Rgba([0; 4]), image::Rgba([200, 80, 20, 255]), 0.5, false, super::BrushMode::Normal).0, [200, 80, 20, 128]);
        assert_eq!(super::round_brush_blend(image::Rgba([200, 80, 20, 255]), image::Rgba([0, 0, 0, 255]), 0.5, true, super::BrushMode::Normal).0, [200, 80, 20, 128]);
        assert_eq!(super::round_brush_blend(image::Rgba([200, 80, 20, 255]), image::Rgba([0, 0, 0, 255]), 1., true, super::BrushMode::Normal).0, [0; 4]);
    }

    use super::*;
    #[test]
    fn fill_recolors_only_connected_matching_pixels_even_at_the_boundary() {
        let mut e=tiny(1,1); e.diamond=false;
        let original=Rgba([10,20,30,255]);
        for y in 0..4 {for x in [0,1,6,7] {e.tiles[0].put_pixel(x,y,original);}}
        e.pixels=e.tiles[0].clone();
        let before=e.snapshot(); e.color=[100,150,200,255];
        e.fill_region((-4,-2),false);
        for y in 0..4 { for x in 0..8 {
            let expected=if x<2 {Rgba(e.color)}else if x>=6 {original}else{Rgba([0;4])};
            assert_eq!(*e.pixels.get_pixel(x,y),expected);
        } }
        assert_eq!(e.undo.len(),1);
        e.history(false); assert!(e.snapshot()==before);
    }
    #[test]
    fn toolbar_position_stays_inside_resized_drawing_regions() {
        let size=egui::vec2(44.,270.);
        let area=Rect::from_min_size(egui::pos2(300.,80.),egui::vec2(800.,600.));
        let offset=toolbar_offset_inside(egui::vec2(700.,400.),size,area);
        assert!(area.contains_rect(Rect::from_min_size(area.min+offset,size)));
        let smaller=Rect::from_min_size(egui::pos2(260.,100.),egui::vec2(300.,300.));
        let offset=toolbar_offset_inside(offset,size,smaller);
        assert_eq!(offset,egui::vec2(256.,30.));
        assert!(smaller.contains_rect(Rect::from_min_size(smaller.min+offset,size)));
        assert_eq!(toolbar_offset_inside(Vec2::splat(-100.),size,smaller),Vec2::ZERO);
        assert_eq!(toolbar_offset_inside(Vec2::splat(f32::NAN),size,smaller),Vec2::splat(4.));
        let tiny=Rect::from_min_size(Pos2::ZERO,egui::vec2(20.,80.));
        assert_eq!(toolbar_offset_inside(Vec2::splat(100.),size,tiny),Vec2::ZERO);
    }
    #[test]
    fn boundary_and_center_keys_change_only_the_shared_view() {
        let mut e=Editor::default();
        let ctx=egui::Context::default();
        let state=e.snapshot();
        e.pan=egui::vec2(50.,30.); e.zoom=3.;
        for (key,infinite) in [(egui::Key::W,true),(egui::Key::Q,false),(egui::Key::W,true)] {
            let _=ctx.run(egui::RawInput { focused:true,events:vec![egui::Event::Key {
                key,physical_key:None,pressed:true,repeat:false,modifiers:egui::Modifiers::NONE,
            }],..Default::default() },|ctx|e.tool_shortcuts(ctx));
            assert_eq!(e.infinite_view,infinite);
            assert_eq!(e.pan,egui::vec2(50.,30.));
        }
        let _=ctx.run(egui::RawInput { focused:true,events:vec![egui::Event::Key {
            key:egui::Key::H,physical_key:None,pressed:true,repeat:false,modifiers:egui::Modifiers::NONE,
        }],..Default::default() },|ctx|e.tool_shortcuts(ctx));
        assert_eq!(e.pan,Vec2::ZERO);
        assert_eq!(e.zoom,3.);
        assert!(e.snapshot()==state && e.undo.is_empty());
    }
    #[test]
    fn unified_view_focus_matches_the_configured_unit_in_all_flat_modes() {
        for (diamond,hex) in [(false,0),(true,0),(false,1),(false,2)] {
            let mut e=tiny(3,2); e.diamond=diamond; e.hex=hex;
            let mut bounds=Rect::NOTHING;
            for row in 0..2 { for col in 0..3 {
                bounds=bounds.union(Rect::from_center_size(e.mapped_center(col,row,8,4).to_pos2(),egui::vec2(8.,4.)));
            } }
            assert!((e.center_unit_focus()-bounds.center().to_vec2()).length()<0.001);
            let pan=egui::vec2(30.,-20.); let scale=2.; let center=egui::pos2(320.,240.);
            let origin=center+pan-e.center_unit_focus()*scale;
            assert!((origin+e.center_unit_focus()*scale-(center+pan)).length()<0.001);
        }
    }
    #[test]
    fn both_hex_lattices_have_unique_pixels_and_periodic_mapping() {
        for flat in [false, true] {
            let (w,h) = (16,12);
            let mut owned = 0;
            for y in 0..h { for x in 0..w {
                let cell = hex_pixel(x as i32-w as i32/2,y as i32-h as i32/2,w,h,flat);
                if cell.col == 0 && cell.row == 0 { owned += 1; }
            } }
            assert_eq!(owned, w*h*3/4);
            for y in -24..24 { for x in -32..32 {
                let cell = hex_pixel(x,y,w,h,flat);
                assert!(cell.x < w && cell.y < h);
                for (dc,dr) in [(1,0),(0,1),(-2,3)] {
                    let shift = hex_center(dc,dr,w,h,flat);
                    let next = hex_pixel(x+shift.x as i32,y+shift.y as i32,w,h,flat);
                    assert_eq!((next.col,next.row,next.x,next.y),(cell.col+dc,cell.row+dr,cell.x,cell.y));
                }
            } }
        }
    }
    #[test]
    fn hex_background_fill_export_and_history_use_the_same_shape() {
        for hex in [1,2] {
            let mut e = tiny(2,2);
            e.hex = hex; e.diamond = false;
            e.color = [20,140,60,255];
            e.fill_region((0,0),true);
            assert_eq!(e.undo.len(),1);
            for tile in 0..4 {
                let image = e.export_cell(tile);
                for y in 0..4 { for x in 0..8 {
                    assert_eq!(image.get_pixel(x,y)[3] != 0,e.inside_tile(x as i32,y as i32,8,4));
                } }
            }
            e.history(false);
            assert!(e.tiles.iter().all(|tile| tile.pixels().all(|p| p[3] == 0)));
            e.history(true);
            assert_eq!(e.hex,hex);
        }
    }
    fn two_layer_fixture(e: &mut Editor) {
        e.ensure_layers();
        e.layers[0].locked = true;
        let blank: Vec<_> = e.tiles.iter().map(|p| RgbaImage::new(p.width(), p.height())).collect();
        e.layers.push(Layer { settings:Default::default(), material: false, name: "Drawing".into(), visible: true, locked: false, tiles: blank });
        e.select_layer(1);
    }
    #[test]
    fn ordinary_import_clear_and_last_delete_are_undoable() {
        let mut e = Editor::default();
        e.ensure_layers();
        assert_eq!(e.layers.len(), 1);
        assert!(!e.layers[0].locked);
        e.record_layer_change();
        e.insert_image_layer("Layer".into(),RgbaImage::from_pixel(128, 64, Rgba([10, 20, 30, 255])));
        assert_eq!(e.layers.len(), 2);
        assert!(!e.layers[1].locked);
        e.layers[1].locked=true;
        e.delete_layer(1);
        assert_eq!(e.layers.len(), 2);
        e.layers[1].locked = false;
        e.clear_selected_layer();
        assert!(e.tiles[0].pixels().all(|p| p[3] == 0));
        e.history(false);
        assert_eq!(e.pixels.get_pixel(0, 0).0, [10, 20, 30, 255]);
        e.delete_layer(1);
        e.delete_layer(0);
        assert_eq!(e.layers.len(), 1);
        assert!(!e.layers[0].locked);
        assert!(e.pixels.pixels().all(|p| p[3] == 0));
        e.history(false);
        assert_eq!(e.layers.len(), 1);
    }
    #[test]
    fn preview_move_drag_changes_only_its_own_view() {
        let ctx = egui::Context::default();
        let mut e = tiny(1, 1);
        let before = e.snapshot();
        let mut frame = |events| {
            let _ = ctx.run(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(640., 480.))),
                focused: true, events, ..Default::default()
            }, |ctx| { egui::CentralPanel::default().show(ctx, |ui| e.preview(ui, true)); });
        };
        frame(vec![]);
        let start = egui::pos2(200., 150.);
        let end = start + egui::vec2(40., 30.);
        frame(vec![egui::Event::PointerMoved(start), egui::Event::PointerButton {
            pos: start, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE,
        }]);
        frame(vec![egui::Event::PointerMoved(end)]);
        frame(vec![egui::Event::PointerButton {
            pos: end, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE,
        }]);
        assert!(e.repeat_pan.length() > 0.);
        assert_eq!(e.pan, Vec2::ZERO);
        assert!(e.snapshot() == before);
        assert!(e.undo.is_empty());
        assert!(e.preview_last.is_none());
    }
    #[test]
    fn pen_zoom_preserves_contact_point_and_reverses_at_limits() {
        let anchor = egui::pos2(230., 170.);
        let center = egui::pos2(150., 100.);
        let pan = egui::vec2(20., -10.);
        let world = (anchor - center - pan) / 4.;
        for dx in [-10000., -80., 0., 80., 10000.] {
            let (zoom, shifted) = drag_zoom(anchor, 4., pan, dx, center,0.25);
            assert!((0.25..=64.).contains(&zoom));
            assert!((center + shifted + world * zoom - anchor).length() < 0.001);
            if dx < 0. { assert!(zoom < 4.); }
            if dx > 0. { assert!(zoom > 4.); }
        }
        let (zoom, shifted) = drag_zoom(anchor, 4., pan, 0., center,0.25);
        assert_eq!(zoom, 4.);
        assert_eq!(shifted, pan);
    }
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
        two_layer_fixture(&mut e);
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
        two_layer_fixture(&mut e);
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
        two_layer_fixture(&mut e);
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
        two_layer_fixture(&mut e);
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
            fx_light: Default::default(),
            version: 5,
            materials: Vec::new(),
            background_custom: false,
            background_color: default_background(),
            width: 128,
            height: 64,
            layers: state
                .layers
                .iter()
                .map(|l| StoredLayer { settings:l.settings.clone(), material: l.material,
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
            hex: 0,
            elevated: false,
            footprint: [128, 64],
            wrap: true,
            edit_unit: true,
            brush_size: 1,
            soft_brush: true,
            rectangle_fill:false,
            brush_hardness: 65,
            brush_mode: BrushMode::Multiply,
            pressure_size: true,
            pressure_opacity: true,
            pressure_min_size: 10,
            subgroup_enabled: true,
            subgroup_size: [1,1],
            brush_flow:37,
            brush_spacing:17,
            pressure_flow:true,
        };
        let bytes = serde_json::to_vec(&project).unwrap();
        let mut restored: Project = serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();
        assert!(restored.soft_brush);
        assert_eq!(restored.brush_hardness, 65);
        assert_eq!(restored.brush_mode, BrushMode::Multiply);
        assert!(restored.pressure_size && restored.pressure_opacity);
        assert_eq!(restored.pressure_min_size, 10);
        assert!(restored.subgroup_enabled);
        assert_eq!(restored.subgroup_size,[1,1]);
        assert_eq!(restored.brush_flow,37);
        assert_eq!(restored.brush_spacing,17);
        assert!(restored.pressure_flow);
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
        e.select_slot(9);
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
    fn transparent_diamond_fill_stays_inside_and_preserves_colored_pixels() {
        let mut e = tiny(1, 1);
        let (_, x, y) = e.finite_plane_hit(0, 0, false).unwrap();
        e.tiles[0].put_pixel(x, y, Rgba([200, 100, 50, 0]));
        let (_, bx, by) = e.finite_plane_hit(1, 0, false).unwrap();
        e.tiles[0].put_pixel(bx, by, Rgba([1, 2, 3, 128]));
        e.pixels = e.tiles[0].clone();
        let before = e.snapshot();
        e.color = [80, 30, 200, 255];
        e.fill_region((0, 0), false);
        for y in 0..4 {
            for x in 0..8 {
                let expected = if (x, y) == (bx, by) { [1, 2, 3, 128] }
                    else if inside(x as i32, y as i32, 8, 4, true) { e.color }
                    else { [0; 4] };
                assert_eq!(e.pixels.get_pixel(x, y).0, expected);
            }
        }
        assert_eq!(e.undo.len(), 1);
        e.history(false);
        assert!(e.snapshot() == before);
        e.history(true);
        assert_eq!(e.pixels.get_pixel(x, y).0, e.color);
    }
    #[test]
    fn rectangle_fill_and_open_transparent_background_are_each_one_undo() {
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
        e.fill_region((0, 0), false);
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
        e.fill_region((0, 0), false);
        assert_eq!(e.pixels.get_pixel(64, 32).0, e.color);
        assert_eq!(e.pixels.get_pixel(70, 32).0, e.color);
        assert_eq!(e.undo.len(), history + 1);
        assert!(e.message.is_empty());
        e.history(false);
        assert!(e.snapshot() == open);
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
        assert!(e.tool == Tool::Move);
        for (key, tool) in [
            (egui::Key::V, Tool::Move),
            (egui::Key::I, Tool::Eyedropper),
            (egui::Key::E, Tool::Eraser),
            (egui::Key::K, Tool::Rectangle),
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
        assert_eq!(legacy.brush_flow,100);
        assert_eq!(legacy.brush_spacing,10);
        assert!(!legacy.pressure_flow);
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
