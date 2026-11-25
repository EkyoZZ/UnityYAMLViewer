//! Tile-scoped non-destructive layer appearance and editing permissions.
use super::*;
pub(super) mod effects;
pub(super) use effects::{
    read_live_rectangle, read_solid_fill, solid_shape_stroke_supported, write_live_rectangle,
    write_solid_fill,
};
pub(super) const DETAIL_WIDTH: f32 = 240.;
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Blend {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    Difference,
    Exclusion,
    Add,
    Subtract,
    SoftLight,
    HardLight,
    Dodge,
    Burn,
    Divide,
    LinearBurn,
    Hue,
    Saturation,
    Color,
    Luminosity,
}
impl Blend {
    pub(super) const ALL: [Self; 20] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::Difference,
        Self::Exclusion,
        Self::Add,
        Self::Subtract,
        Self::SoftLight,
        Self::HardLight,
        Self::Dodge,
        Self::Burn,
        Self::Divide,
        Self::LinearBurn,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Multiply => "正片叠底",
            Self::Screen => "滤色",
            Self::Overlay => "叠加",
            Self::Darken => "变暗",
            Self::Lighten => "变亮",
            Self::Difference => "差值",
            Self::Exclusion => "排除",
            Self::Add => "线性减淡",
            Self::Subtract => "减去",
            Self::SoftLight => "柔光",
            Self::HardLight => "强光",
            Self::Dodge => "颜色减淡",
            Self::Burn => "颜色加深",
            Self::Divide => "划分",
            Self::LinearBurn => "线性加深",
            Self::Hue => "色相",
            Self::Saturation => "饱和度",
            Self::Color => "颜色",
            Self::Luminosity => "明度",
        }
    }
    fn rgb(self, b: [f32; 3], s: [f32; 3]) -> [f32; 3] {
        fn lum(c: [f32; 3]) -> f32 {
            c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11
        }
        fn sat(c: [f32; 3]) -> f32 {
            c.into_iter().fold(0_f32, f32::max) - c.into_iter().fold(1_f32, f32::min)
        }
        fn set_lum(mut c: [f32; 3], l: f32) -> [f32; 3] {
            let d = l - lum(c);
            for v in &mut c {
                *v += d;
            }
            let n = c.into_iter().fold(f32::INFINITY, f32::min);
            let x = c.into_iter().fold(f32::NEG_INFINITY, f32::max);
            if n < 0. {
                for v in &mut c {
                    *v = l + (*v - l) * l / (l - n).max(1e-6);
                }
            }
            if x > 1. {
                for v in &mut c {
                    *v = l + (*v - l) * (1. - l) / (x - l).max(1e-6);
                }
            }
            c.map(|v| v.clamp(0., 1.))
        }
        fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
            let mut order = [0, 1, 2];
            order.sort_by(|a, b| c[*a].total_cmp(&c[*b]));
            let mut out = [0.; 3];
            let range = c[order[2]] - c[order[0]];
            if range > 0. {
                out[order[1]] = (c[order[1]] - c[order[0]]) / range * s;
                out[order[2]] = s;
            }
            out
        }
        match self {
            Self::Hue => set_lum(set_sat(s, sat(b)), lum(b)),
            Self::Saturation => set_lum(set_sat(b, sat(s)), lum(b)),
            Self::Color => set_lum(s, lum(b)),
            Self::Luminosity => set_lum(b, lum(s)),
            _ => std::array::from_fn(|c| self.channel(b[c], s[c])),
        }
    }
    fn channel(self, b: f32, s: f32) -> f32 {
        match self {
            Self::Normal => s,
            Self::Multiply => b * s,
            Self::Screen => 1. - (1. - b) * (1. - s),
            Self::Overlay => {
                if b <= 0.5 {
                    2. * b * s
                } else {
                    1. - 2. * (1. - b) * (1. - s)
                }
            }
            Self::Darken => b.min(s),
            Self::Lighten => b.max(s),
            Self::Difference => (b - s).abs(),
            Self::Exclusion => b + s - 2. * b * s,
            Self::Add => (b + s).min(1.),
            Self::Subtract => (b - s).max(0.),
            Self::HardLight => {
                if s <= 0.5 {
                    2. * b * s
                } else {
                    1. - 2. * (1. - b) * (1. - s)
                }
            }
            Self::SoftLight => {
                if s <= 0.5 {
                    b - (1. - 2. * s) * b * (1. - b)
                } else {
                    let d = if b <= 0.25 {
                        ((16. * b - 12.) * b + 4.) * b
                    } else {
                        b.sqrt()
                    };
                    b + (2. * s - 1.) * (d - b)
                }
            }
            Self::Dodge => {
                if s >= 1. {
                    1.
                } else {
                    (b / (1. - s)).min(1.)
                }
            }
            Self::Burn => {
                if s <= 0. {
                    0.
                } else {
                    1. - ((1. - b) / s).min(1.)
                }
            }
            Self::Divide => {
                if s <= 0. {
                    1.
                } else {
                    (b / s).min(1.)
                }
            }
            Self::LinearBurn => (b + s - 1.).max(0.),
            _ => s,
        }
    }
}
pub(super) fn over(dst: Rgba<u8>, src: Rgba<u8>, mode: Blend, weight: f32) -> Rgba<u8> {
    let sa = src[3] as f32 / 255. * weight;
    let da = dst[3] as f32 / 255.;
    let a = sa + da * (1. - sa);
    if a <= 0. {
        return Rgba([0; 4]);
    }
    let mut p = [0; 4];
    let mixed = mode.rgb(
        [
            dst[0] as f32 / 255.,
            dst[1] as f32 / 255.,
            dst[2] as f32 / 255.,
        ],
        [
            src[0] as f32 / 255.,
            src[1] as f32 / 255.,
            src[2] as f32 / 255.,
        ],
    );
    for c in 0..3 {
        let b = dst[c] as f32 / 255.;
        let s = src[c] as f32 / 255.;
        p[c] =
            (((s * (1. - da) + mixed[c] * da) * sa + b * da * (1. - sa)) / a * 255.).round() as u8;
    }
    p[3] = (a * 255.).round() as u8;
    Rgba(p)
}
fn default_output_white() -> u8 {
    255
}
fn identity_curves() -> [[u8; 5]; 4] {
    [[0, 64, 128, 192, 255]; 4]
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) enum Adjustment {
    Brightness {
        brightness: i16,
        contrast: i16,
    },
    Levels {
        black: u8,
        gamma: f32,
        white: u8,
        #[serde(default)]
        output_black: u8,
        #[serde(default = "default_output_white")]
        output_white: u8,
    },
    Curves {
        values: [[u8; 5]; 4],
    },
    CurvesBezier {
        channels: [Vec<curve_editor::Node>; 4],
    },
    Hue {
        saturation: i16,
        hue: i16,
        lightness: i16,
    },
}
impl Default for Adjustment {
    fn default() -> Self {
        Self::Brightness {
            brightness: 0,
            contrast: 0,
        }
    }
}
impl Adjustment {
    fn label(&self) -> &'static str {
        match self {
            Self::Brightness { .. } => "亮度/对比度",
            Self::Levels { .. } => "色阶",
            Self::Curves { .. } | Self::CurvesBezier { .. } => "曲线",
            Self::Hue { .. } => "色相/饱和度",
        }
    }
    fn apply(&self, p: Rgba<u8>) -> Rgba<u8> {
        let mut q = p;
        match self {
            Self::Brightness {
                brightness,
                contrast,
            } => {
                let b = (*brightness as f32).clamp(-150., 150.) / 100.;
                let c = (*contrast as f32).clamp(-100., 100.) / 100.;
                for i in 0..3 {
                    let v = p[i] as f32 / 255.;
                    q[i] = (((v - 0.5) * (1. + c * 0.99) / (1. - c * 0.99) + 0.5 + b).clamp(0., 1.)
                        * 255.)
                        .round() as u8;
                }
            }
            Self::Levels {
                black,
                gamma,
                white,
                output_black,
                output_white,
            } => {
                let range = white.saturating_sub(*black).max(1) as f32;
                let g = if gamma.is_finite() {
                    gamma.clamp(0.1, 10.)
                } else {
                    1.
                };
                for i in 0..3 {
                    q[i] = (((p[i] as f32 - *black as f32) / range)
                        .clamp(0., 1.)
                        .powf(1. / g)
                        * (*output_white as f32 - *output_black as f32)
                        + *output_black as f32)
                        .clamp(0., 255.)
                        .round() as u8;
                }
            }
            Self::Curves { values } => {
                fn curve(v: u8, a: &[u8; 5]) -> u8 {
                    let stops = [0_f32, 64., 128., 192., 255.];
                    let i = ((v as usize) / 64).min(3);
                    let t = (v as f32 - stops[i]) / (stops[i + 1] - stops[i]);
                    (a[i] as f32 + (a[i + 1] as f32 - a[i] as f32) * t).round() as u8
                }
                for i in 0..3 {
                    q[i] = curve(curve(p[i], &values[0]), &values[i + 1]);
                }
            }
            Self::CurvesBezier { channels } => {
                for i in 0..3 {
                    q[i] = (curve_editor::evaluate(
                        &channels[i + 1],
                        curve_editor::evaluate(&channels[0], p[i] as f32 / 255.),
                    ) * 255.)
                        .round() as u8;
                }
            }
            Self::Hue {
                saturation,
                hue,
                lightness,
            } => {
                let mut h = egui::ecolor::Hsva::from_rgba_unmultiplied(
                    p[0] as f32 / 255.,
                    p[1] as f32 / 255.,
                    p[2] as f32 / 255.,
                    1.,
                );
                h.h = (h.h + *hue as f32 / 360.).rem_euclid(1.);
                h.s = (h.s * (1. + *saturation as f32 / 100.)).clamp(0., 1.);
                let v = h.to_rgba_unmultiplied();
                let l = (*lightness as f32 / 100.).clamp(-1., 1.);
                for i in 0..3 {
                    q[i] = ((if l >= 0. {
                        v[i] + (1. - v[i]) * l
                    } else {
                        v[i] * (1. + l)
                    })
                    .clamp(0., 1.)
                        * 255.)
                        .round() as u8;
                }
            }
        }
        q
    }
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct Properties {
    #[serde(skip)]
    pub(super) psd_layer_id:Option<u32>,
    #[serde(skip)]
    pub(super) psd_metadata:Vec<u8>,
    #[serde(skip)]
    pub(super) psd_native_text: bool,
    #[serde(skip)]
    pub(super) psd_tysh: Option<std::sync::Arc<[u8]>>,
    #[serde(skip)]
    pub(super) psd_text_count: usize,
    pub clipped: bool,
    pub link_id: u64,
    pub group_id: u64,
    pub parent: u64,
    pub pass_through: bool,
    pub collapsed: bool,
    pub blend: Blend,
    pub opacity: u8,
    pub fill: u8,
    pub lock_alpha: bool,
    pub lock_pixels: bool,
    pub lock_position: bool,
    pub group: String,
    pub mask: Option<Vec<Vec<u8>>>,
    pub mask_enabled: bool,
    pub mask_linked: bool,
    pub adjustment: Option<Adjustment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) text: Option<super::text_layer::Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) vector: Option<super::vector::Vector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) vector_origin: Option<[i32; 2]>,
    #[serde(flatten)]
    pub(super) effects: effects::Effects,
}
impl Default for Properties {
    fn default() -> Self {
        Self {
            psd_layer_id:None, psd_metadata:Vec::new(), psd_native_text: false,
            psd_tysh: None, psd_text_count: 0,
            clipped: false,
            link_id: 0,
            group_id: 0,
            parent: 0,
            pass_through: true,
            collapsed: false,
            blend: Blend::Normal,
            opacity: 100,
            fill: 100,
            lock_alpha: false,
            lock_pixels: false,
            lock_position: false,
            group: String::new(),
            mask: None,
            mask_enabled: true,
            mask_linked: true,
            adjustment: None,
            text: None,
            vector: None,
            vector_origin: None,
            effects: Default::default(),
        }
    }
}
impl Properties {
    pub(super) fn has_effects(&self) -> bool {
        self.effects.active()
    }
    pub fn bytes(&self) -> usize {
        self.mask
            .as_ref()
            .map_or(0, |m| m.iter().map(Vec::len).sum())
    }
    pub fn valid(&self, count: usize, pixels: usize) -> bool {
        self.opacity <= 100
            && self.fill <= 100
            && self.group.chars().count() <= 64
            && self.effects.valid()
            && self.vector.as_ref().is_none_or(|shape| self.group_id == 0 && self.adjustment.is_none() && self.text.is_none() && shape.valid())
            && self.text.as_ref().is_none_or(|text| self.group_id == 0 && self.adjustment.is_none() && text.valid())
            && self
                .mask
                .as_ref()
                .is_none_or(|m| m.len() == count && m.iter().all(|v| v.len() == pixels))
            && match &self.adjustment {
                Some(Adjustment::Levels {
                    black,
                    gamma,
                    white,
                    ..
                }) => white > black && gamma.is_finite() && (0.1..=10.).contains(gamma),
                Some(Adjustment::CurvesBezier { channels }) => {
                    channels.iter().all(|nodes| curve_editor::valid(nodes))
                }
                _ => true,
            }
    }
    pub(super) fn mask_value(&self, tile: usize, index: usize) -> f32 {
        if self.mask_enabled {
            self.mask
                .as_ref()
                .and_then(|m| m.get(tile))
                .and_then(|m| m.get(index))
                .copied()
                .unwrap_or(255) as f32
                / 255.
        } else {
            1.
        }
    }
    pub fn composite(&self, out: &mut RgbaImage, source: &RgbaImage, tile: usize, light: effects::Light) {
        self.composite_with_gradient_frames(out, source, tile, light, None, None);
    }

    pub(super) fn has_gradient_overlay(&self) -> bool {
        self.has_effects() && self.effects.has_gradient_overlay()
    }

    pub(super) fn has_aligned_gradient_overlay(&self) -> bool {
        self.has_effects() && self.effects.has_aligned_gradient_overlay()
    }

    pub(super) fn composite_with_gradient_frames(
        &self, out: &mut RgbaImage, source: &RgbaImage, tile: usize, light: effects::Light,
        gradient_frames: Option<[[f32; 4]; 2]>,
        text_gradient: Option<&super::text_layer::TextGradient>,
    ) {
        if let Some(adjust) = &self.adjustment {
            let lut = if let Adjustment::CurvesBezier { channels } = adjust {
                Some(curve_editor::lookup(channels))
            } else {
                None
            };
            for (i, p) in out.pixels_mut().enumerate() {
                let q = if let Some(lut) = &lut {
                    let mut q = *p;
                    for c in 0..3 {
                        q[c] = lut[c][p[c] as usize];
                    }
                    q
                } else {
                    adjust.apply(*p)
                };
                let k =
                    self.opacity as f32 / 100. * self.fill as f32 / 100. * self.mask_value(tile, i);
                let mixed = self.blend.rgb(
                    [p[0] as f32 / 255., p[1] as f32 / 255., p[2] as f32 / 255.],
                    [q[0] as f32 / 255., q[1] as f32 / 255., q[2] as f32 / 255.],
                );
                for c in 0..3 {
                    let b = p[c] as f32 / 255.;
                    p[c] = ((b * (1. - k) + mixed[c] * k) * 255.).round() as u8;
                }
            }
            return;
        }
        if self.has_effects() {
            self.effects.composite(self, out, source, tile, light, gradient_frames, text_gradient);
        } else {
            for (i, (dst, src)) in out.pixels_mut().zip(source.pixels()).enumerate() {
                *dst = over(*dst, *src, self.blend,
                    self.opacity as f32 / 100. * self.fill as f32 / 100. * self.mask_value(tile, i));
            }
        }
    }
}
impl Editor {
    pub(super) fn style_context_menu(&mut self, ui: &mut egui::Ui, index: usize, fx: bool) {
        let eligible=!self.layers[index].material && self.layers[index].settings.adjustment.is_none();
        let editable=eligible && !self.layers[index].locked && !self.ancestor_locked(index);
        if fx && ui.button(tr("复制图层样式")).clicked() { self.style_clipboard=Some(self.layers[index].settings.effects.clone()); ui.close_menu(); }
        if ui.add_enabled(editable && self.style_clipboard.is_some(),egui::Button::new(tr("粘贴图层样式"))).clicked() {
            let source=self.style_clipboard.clone().unwrap();
            if source != self.layers[index].settings.effects { self.record_layer_change(); self.layers[index].settings.effects=source; }
            ui.close_menu();
        }
        if fx && ui.add_enabled(editable && self.layers[index].settings.has_effects(),egui::Button::new(tr("清除图层样式"))).clicked() {
            self.record_layer_change(); self.layers[index].settings.effects = Default::default(); ui.close_menu();
        }
    }

    pub(super) fn move_allowed(&self, index: usize) -> bool {
        self.position_editable(index) && self.layers[index].settings.vector.is_none()
    }

    pub(super) fn position_editable(&self, index: usize) -> bool {
        self.layers.get(index).is_some_and(|l| {
            !l.material
                && !l.locked
                && !self.ancestor_locked(index)
                && !self.ancestor_move_locked(index)
                && !l.settings.lock_position
                && !l.settings.lock_alpha
        })
    }
    pub(super) fn blend_edit_pixel(
        &self,
        old: Rgba<u8>,
        color: Rgba<u8>,
        coverage: f32,
        erase: bool,
        mode: BrushMode,
    ) -> Rgba<u8> {
        if !self.mask_editing
            && self
                .layers
                .get(self.active_layer)
                .is_some_and(|l| l.settings.lock_alpha)
        {
            if old[3] == 0 || erase {
                return old;
            }
            let mut opaque = old;
            opaque[3] = 255;
            let mut result = round_brush_blend(opaque, color, coverage, false, mode);
            result[3] = old[3];
            result
        } else {
            round_brush_blend(old, color, coverage, erase, mode)
        }
    }
    pub(super) fn before_edit_pixel(&self, tile: usize, x: u32, y: u32) -> Rgba<u8> {
        let before = self.before.as_ref().unwrap();
        if before.active_layer != self.active_layer {
            return Rgba([0; 4]);
        }
        self.snapshot_edit_pixel(before, tile, x, y)
    }
    pub(super) fn snapshot_edit_pixel(
        &self,
        s: &Snapshot,
        tile: usize,
        x: u32,
        y: u32,
    ) -> Rgba<u8> {
        if self.mask_editing {
            let value = s.layers[s.active_layer].settings.mask.as_ref().unwrap()[tile]
                [(y * self.pixels.width() + x) as usize];
            Rgba([value, value, value, 255])
        } else {
            *s.tiles[tile].get_pixel(x, y)
        }
    }
    fn add_adjustment(&mut self, kind: Adjustment) {
        let bytes: usize = self
            .layers
            .iter()
            .map(|l| l.tiles.iter().map(|p| p.as_raw().len()).sum::<usize>() + l.settings.bytes())
            .sum();
        let added = self.tiles.iter().map(|p| p.as_raw().len()).sum::<usize>();
        if self.layers.len() >= 32 || bytes + added > self.layer_budget() {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        self.record_layer_change();
        let mut settings = Properties::default();
        let name = tr(kind.label()).to_owned();
        settings.adjustment = Some(kind);
        let (parent, index) = self.layer_insert_position();
        settings.parent = parent;
        if let Some(group) = self
            .layers
            .iter_mut()
            .find(|l| parent != 0 && l.settings.group_id == parent)
        {
            group.settings.collapsed = false;
        }
        let tiles = self
            .tiles
            .iter()
            .map(|p| RgbaImage::new(p.width(), p.height()))
            .collect();
        self.layers.insert(
            index,
            Layer {
                material: false,
                name,
                visible: true,
                locked: false,
                settings,
                tiles,
            },
        );
        self.active_layer = index;
        self.selected_layers = vec![index];
        self.layer_anchor = index;
        self.tiles = self.layers[index].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.mask_editing = false;
        self.invalidate_textures();
    }
    pub(super) fn layer_properties_ui(&mut self, ui: &mut egui::Ui) {
        if self.layers.is_empty() {
            return;
        }
        let index = self.active_layer;
        let original = self.layers[index].settings.clone();
        let mut p = original.clone();
        let material = self.layers[index].material;
        let locked = self.layers[index].locked || self.ancestor_locked(index);
        let mut full = locked;
        let mut added_mask = false;
        let mut open_fx = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            ui.label(egui::RichText::new(tr("图层")).heading().strong());
            let linked = self.layers[index].settings.link_id != 0;
            if ui
                .add_enabled_ui(
                    !locked && !material && (linked || self.selected_roots().len() > 1),
                    |ui| {
                        panel_button(
                            ui,
                            PanelIcon::Link,
                            linked,
                            if linked {
                                "取消链接"
                            } else {
                                "链接图层"
                            },
                        )
                    },
                )
                .inner
                .clicked()
            {
                queue_panel(ui, if linked { 2 } else { 1 }, 0);
            }
            ui.add_enabled_ui(!locked && !material, |ui| {
                ui.add_enabled_ui(p.adjustment.is_none(), |ui| {
                    let expanded = self
                        .layer_detail
                        .as_ref()
                        .is_some_and(|(i, fx, _, _)| *i == index && *fx);
                    let response = panel_button(ui, PanelIcon::Effects, expanded, "图层样式");
                    if response.clicked() {
                        open_fx = Some(response.rect);
                    }
                });
                if ui
                    .add_enabled_ui(p.mask.is_none(), |ui| {
                        panel_button(ui, PanelIcon::Mask, false, "添加白色蒙版")
                    })
                    .inner
                    .clicked()
                {
                    let bytes = self
                        .tiles
                        .iter()
                        .map(|t| (t.width() * t.height()) as usize)
                        .sum::<usize>();
                    let total = self
                        .layers
                        .iter()
                        .map(|l| {
                            l.tiles.iter().map(|t| t.as_raw().len()).sum::<usize>()
                                + l.settings.bytes()
                        })
                        .sum::<usize>();
                    if total + bytes <= self.layer_budget() {
                        p.mask = Some(
                            self.tiles
                                .iter()
                                .map(|t| vec![255; (t.width() * t.height()) as usize])
                                .collect(),
                        );
                        p.mask_enabled = true;
                        p.mask_linked = true;
                        added_mask = true;
                    } else {
                        self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
                    }
                }
                panel_menu(ui, PanelIcon::Adjust, "调整", |ui| {
                    for kind in [
                        Adjustment::Brightness {
                            brightness: 0,
                            contrast: 0,
                        },
                        Adjustment::Levels {
                            black: 0,
                            gamma: 1.,
                            white: 255,
                            output_black: 0,
                            output_white: 255,
                        },
                        Adjustment::CurvesBezier {
                            channels: curve_editor::identity(),
                        },
                        Adjustment::Hue {
                            hue: 0,
                            saturation: 0,
                            lightness: 0,
                        },
                    ] {
                        if ui.button(tr(kind.label())).clicked() {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("tile_pending_adjustment"), kind)
                            });
                            ui.memory_mut(|m| m.close_popup());
                            ui.close_menu();
                        }
                    }
                });

                panel_menu(ui, PanelIcon::Folder, "组", |ui| {
                    if ui.button(tr("编组")).clicked() {
                        queue_panel(ui, 3, 0);
                        ui.close_menu();
                    }
                    if p.group_id != 0
                        && ui
                            .button(tr("解散组"))
                            .on_hover_text(tr("解散后移除组蒙版和组样式，保留成员图层。"))
                            .clicked()
                    {
                        queue_panel(ui, 4, 0);
                        ui.close_menu();
                    }
                    ui.menu_button(tr("移动到组"), |ui| {
                        if ui.button(tr("无分组")).clicked() {
                            queue_panel(ui, 5, 0);
                            ui.close_menu();
                        }
                        for (i, l) in self.layers.iter().enumerate() {
                            if l.settings.group_id != 0
                                && i != index
                                && !self
                                    .selected_roots()
                                    .iter()
                                    .any(|root| self.group_contains(*root, i))
                                && !l.locked
                                && !self.ancestor_locked(i)
                            {
                                if ui.button(&l.name).clicked() {
                                    queue_panel(ui, 5, l.settings.group_id);
                                    ui.close_menu();
                                }
                            }
                        }
                    });
                });
            });
            if ui
                .add_enabled_ui(self.layers.len() < 32, |ui| {
                    panel_button(ui, PanelIcon::New, false, "添加图层")
                })
                .inner
                .clicked()
            {
                queue_panel(ui, 6, 0);
            }
            if ui
                .add_enabled_ui(!locked && !material, |ui| {
                    panel_button(ui, PanelIcon::Delete, false, "删除图层")
                })
                .inner
                .clicked()
            {
                queue_panel(ui, 7, 0);
            }
            if ui
                .add_enabled_ui(
                    self.layer_editable() && !self.mask_editing && !p.lock_alpha,
                    |ui| panel_button(ui, PanelIcon::Clear, false, "清空选中图层"),
                )
                .inner
                .clicked()
            {
                queue_panel(ui, 8, 0);
            }
        });
        ui.add_enabled_ui(!locked && !material, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.;
                egui::ComboBox::from_id_salt("layer_blend")
                    .width((ui.available_width() - 130.).max(65.))
                    .selected_text(tr(if p.group_id != 0 && p.pass_through {
                        "穿透"
                    } else {
                        p.blend.label()
                    }))
                    .show_ui(ui, |ui| {
                        if p.group_id != 0
                            && ui.selectable_label(p.pass_through, tr("穿透")).clicked()
                        {
                            p.pass_through = true;
                        }
                        for mode in Blend::ALL {
                            if ui
                                .selectable_label(
                                    (p.group_id == 0 || !p.pass_through) && p.blend == mode,
                                    tr(mode.label()),
                                )
                                .clicked()
                            {
                                p.blend = mode;
                                p.pass_through = false;
                            }
                        }
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_sized([42., ui.spacing().interact_size.y],
                    egui::DragValue::new(&mut p.opacity)
                        .range(0..=100)
                        .suffix("%")
                        .speed(1.),
                );
                ui.label(tr("不透明度"));
                });
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            ui.label(tr("锁定"));
            ui.add_enabled_ui(!locked && !material, |ui| {
                let alpha_available =
                    p.group_id == 0 && !p.lock_pixels && !self.ancestor_paint_locked(index);
                for (value, icon, label) in [
                    (&mut p.lock_alpha, PanelIcon::Alpha, "透明度"),
                    (&mut p.lock_pixels, PanelIcon::Paint, "绘制"),
                    (&mut p.lock_position, PanelIcon::Move, "移动"),
                ] {
                    if ui
                        .add_enabled_ui(label != "透明度" || alpha_available, |ui| {
                            panel_button(ui, icon, *value, label)
                        })
                        .inner
                        .clicked()
                    {
                        *value = !*value;
                    }
                }
                if !original.lock_pixels && p.lock_pixels {
                    p.lock_alpha = false;
                }
            });
            if ui
                .add_enabled_ui(!material && !self.ancestor_locked(index), |ui| {
                    panel_button(ui, PanelIcon::Lock, full, "全部")
                })
                .inner
                .clicked()
            {
                full = !full;
            }
            ui.add_enabled_ui(!locked && !material, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_sized([42., ui.spacing().interact_size.y],
                    egui::DragValue::new(&mut p.fill)
                        .range(0..=100)
                        .suffix("%")
                        .speed(1.),
                );
                ui.label(tr("填充"));
                });
            });
        });
        if p != original || full != locked {
            self.record_layer_change();
            self.layers[index].settings = p;
            self.layers[index].locked = full;
            if self.layers[index].settings.mask.is_none() {
                self.mask_editing = false;
            }
            self.invalidate_textures();
        }
        if added_mask {
            self.mask_editing = true;
        }
        if let Some(kind) = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<Adjustment>(egui::Id::new("tile_pending_adjustment")))
        {
            self.add_adjustment(kind);
        }
        if let Some(rect) = open_fx {
            self.toggle_layer_detail(index, true, rect, ui.ctx().screen_rect());
        }
        self.group_controls(ui);
    }
}
fn adjustment_ui(ui: &mut egui::Ui, adjust: &mut Adjustment, histogram: Option<&[f32; 256]>) {
    match adjust {
        Adjustment::Brightness {
            brightness,
            contrast,
        } => {
            for (value, label, range) in [
                (brightness, "亮度", -150..=150),
                (contrast, "对比度", -50..=100),
            ] {
                ui.horizontal(|ui| {
                    ui.label(tr(label));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::DragValue::new(&mut *value)
                                .range(range.clone())
                                .clamp_existing_to_range(false)
                                .speed(1.),
                        );
                    });
                });
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = ui.available_width();
                    crate::theme::slider(
                        ui,
                        true,
                        egui::Slider::new(value, range)
                            .show_value(false)
                            .clamping(egui::SliderClamping::Edits),
                    );
                });
            }
        }
        Adjustment::Levels {
            black,
            gamma,
            white,
            output_black,
            output_white,
        } => super::levels_editor::show(
            ui,
            black,
            gamma,
            white,
            output_black,
            output_white,
            histogram,
        ),
        Adjustment::Hue {
            hue,
            saturation,
            lightness,
        } => {
            for (value, label, range, colors) in [
                (
                    hue,
                    "色相",
                    -180..=180,
                    vec![
                        Color32::RED,
                        Color32::YELLOW,
                        Color32::GREEN,
                        Color32::from_rgb(0, 255, 255),
                        Color32::BLUE,
                        Color32::from_rgb(255, 0, 255),
                        Color32::RED,
                    ],
                ),
                (
                    saturation,
                    "饱和度",
                    -100..=100,
                    vec![Color32::GRAY, Color32::RED],
                ),
                (
                    lightness,
                    "明度",
                    -100..=100,
                    vec![Color32::BLACK, Color32::GRAY, Color32::WHITE],
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(tr(label));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::DragValue::new(&mut *value).range(range.clone()));
                    });
                });
                super::levels_editor::colored_slider(ui, value, range, &colors);
            }
        }
        Adjustment::CurvesBezier { channels } => curve_editor::show(ui, channels),
        Adjustment::Curves { values } => {
            let original=curve_editor::from_legacy(*values);
            let mut channels=original.clone();
            curve_editor::show(ui,&mut channels);
            if channels!=original {*adjust=Adjustment::CurvesBezier{channels};}
        }
    }
}

pub(super) fn mask_button(
    ui: &mut egui::Ui,
    values: &[u8],
    width: u32,
    height: u32,
    selected: bool,
    index: usize,
) -> egui::Response {
    let response = ui.add_sized([26., 26.], egui::Button::new("").selected(selected));
    let thumb = egui::ColorImage {
        size: [20, 20],
        pixels: (0..400)
            .map(|i| {
                let x = (i % 20) as u32 * width / 20;
                let y = (i / 20) as u32 * height / 20;
                Color32::from_gray(values[(y * width + x) as usize])
            })
            .collect(),
    };
    let id = ui.id().with(("mask_thumbnail", index));
    let mut texture = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            ui.ctx().load_texture(
                format!("mask-{index}"),
                thumb.clone(),
                egui::TextureOptions::NEAREST,
            )
        });
    texture.set(thumb, egui::TextureOptions::NEAREST);
    ui.painter().image(
        texture.id(),
        response.rect.shrink(3.),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
        Color32::WHITE,
    );
    ui.ctx().data_mut(|d| d.insert_temp(id, texture));
    response.on_hover_text(tr("编辑蒙版"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opacity_mask_and_fill_are_non_destructive() {
        let source = RgbaImage::from_pixel(2, 2, Rgba([200, 100, 50, 255]));
        let original = source.clone();
        let mut p = Properties::default();
        p.opacity = 50;
        let mut out = RgbaImage::new(2, 2);
        p.composite(&mut out, &source, 0, effects::Light::default());
        assert_eq!(out.get_pixel(0, 0)[3], 128);
        assert_eq!(source, original);
        p.mask = Some(vec![vec![0; 4]]);
        let mut out = RgbaImage::new(2, 2);
        p.composite(&mut out, &source, 0, effects::Light::default());
        assert!(out.pixels().all(|p| p[3] == 0));
        p.mask_enabled = false;
        p.fill = 0;
        let mut out = RgbaImage::new(2, 2);
        p.composite(&mut out, &source, 0, effects::Light::default());
        assert!(out.pixels().all(|p| p[3] == 0));
    }
    #[test]
    fn adjustments_have_identity_defaults_and_keep_alpha() {
        let pixel = Rgba([79, 123, 190, 128]);
        for a in [
            Adjustment::default(),
            Adjustment::Levels {
                black: 0,
                gamma: 1.,
                white: 255,
                output_black: 0,
                output_white: 255,
            },
            Adjustment::Curves {
                values: identity_curves(),
            },
            Adjustment::Hue {
                hue: 0,
                saturation: 0,
                lightness: 0,
            },
        ] {
            assert_eq!(a.apply(pixel), pixel);
        }
        let inverted = Adjustment::Curves {
            values: [
                [255, 191, 127, 63, 0],
                identity_curves()[1],
                identity_curves()[2],
                identity_curves()[3],
            ],
        };
        assert_eq!(inverted.apply(Rgba([0, 0, 0, 120])).0, [255, 255, 255, 120]);
    }
    #[test]
    fn styles_and_masks_roundtrip_and_reject_bad_geometry() {
        let mut p = Properties::default();
        p.blend = Blend::Multiply;
        p.lock_position = true;
        p.mask = Some(vec![vec![128; 4]]);
        p.effects = serde_json::from_value(serde_json::json!({"shadow":true,"overlay":true})).unwrap();
        let q: Properties = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(p == q);
        assert!(q.valid(1, 4));
        assert!(!q.valid(2, 4));
        assert!(!q.valid(1, 5));
    }
    #[test]
    fn multiply_and_color_modes_are_bounded() {
        assert_eq!(
            over(
                Rgba([128, 128, 128, 255]),
                Rgba([128, 128, 128, 255]),
                Blend::Multiply,
                1.
            )
            .0,
            [64, 64, 64, 255]
        );
        for mode in Blend::ALL {
            let result = mode.rgb([0.2, 0.4, 0.8], [0.9, 0.3, 0.1]);
            assert!(result.iter().all(|v| v.is_finite() && *v >= 0. && *v <= 1.));
        }
    }
    #[test]
    fn mask_paint_does_not_overwrite_pixels_and_locks_are_independent() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(8, 4));
        let i = e.active_layer;
        e.write_pixel(0, 0, 0, Rgba([200, 100, 50, 128]));
        let original = e.pixels.clone();
        e.layers[i].settings.lock_pixels = true;
        assert!(!e.layer_editable());
        assert!(e.move_allowed(i));
        e.layers[i].settings.lock_position = true;
        assert!(!e.move_allowed(i));
        e.layers[i].settings.lock_pixels = false;
        e.layers[i].settings.lock_position = false;
        e.layers[i].settings.lock_alpha = true;
        let q = e.blend_edit_pixel(
            Rgba([200, 100, 50, 128]),
            Rgba([0, 0, 0, 255]),
            0.5,
            false,
            BrushMode::Normal,
        );
        assert_eq!(q.0, [100, 50, 25, 128]);
        e.layers[i].settings.mask = Some(vec![vec![255; 32]]);
        e.mask_editing = true;
        e.before = Some(e.snapshot());
        e.write_pixel(0, 0, 0, Rgba([0, 0, 0, 255]));
        e.finish_stroke();
        assert_eq!(e.pixels, original);
        assert_eq!(e.layers[i].settings.mask.as_ref().unwrap()[0][0], 0);
        e.history(false);
        assert_eq!(e.layers[i].settings.mask.as_ref().unwrap()[0][0], 255);
    }
}

fn dilate(source: &image::GrayImage, radius: usize) -> image::GrayImage {
    let w = source.width() as usize;
    let h = source.height() as usize;
    let mut temp = vec![0; w * h];
    let mut out = temp.clone();
    for row in 0..h {
        let mut queue = std::collections::VecDeque::new();
        let mut next = 0;
        for x in 0..w {
            while next <= (x + radius).min(w - 1) {
                let value = source.get_pixel(next as u32, row as u32)[0];
                while queue
                    .back()
                    .is_some_and(|i| source.get_pixel(*i as u32, row as u32)[0] <= value)
                {
                    queue.pop_back();
                }
                queue.push_back(next);
                next += 1;
            }
            while queue.front().is_some_and(|i| *i < x.saturating_sub(radius)) {
                queue.pop_front();
            }
            temp[row * w + x] = source.get_pixel(*queue.front().unwrap() as u32, row as u32)[0];
        }
    }
    for col in 0..w {
        let mut queue = std::collections::VecDeque::new();
        let mut next = 0;
        for y in 0..h {
            while next <= (y + radius).min(h - 1) {
                let value = temp[next * w + col];
                while queue.back().is_some_and(|i| temp[*i * w + col] <= value) {
                    queue.pop_back();
                }
                queue.push_back(next);
                next += 1;
            }
            while queue.front().is_some_and(|i| *i < y.saturating_sub(radius)) {
                queue.pop_front();
            }
            out[y * w + col] = temp[*queue.front().unwrap() * w + col];
        }
    }
    image::GrayImage::from_raw(source.width(), source.height(), out).expect("matched mask dimensions")
}

pub(super) fn mask_link_button(ui: &mut egui::Ui, linked: bool) -> egui::Response {
    let response = ui.add_sized([16., 24.], egui::Button::new(""));
    if linked {
        let center = response.rect.center();
        let stroke = egui::Stroke::new(1.3_f32, ui.visuals().text_color());
        for dy in [-3., 3.] {
            ui.painter().rect_stroke(
                Rect::from_center_size(center + egui::vec2(0., dy), egui::vec2(6., 9.)),
                3.,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().line_segment(
            [center - egui::vec2(0., 3.), center + egui::vec2(0., 3.)],
            stroke,
        );
    }
    response.on_hover_text(tr("链接蒙版与内容"))
}

#[derive(Clone, Copy, Debug)]
enum PanelIcon {
    Link,
    Effects,
    Mask,
    Adjust,
    Folder,
    New,
    Delete,
    Clear,
    Alpha,
    Paint,
    Move,
    Lock,
}
fn panel_button(ui: &mut egui::Ui, icon: PanelIcon, selected: bool, label: &str) -> egui::Response {
    let response = ui.add_sized(
        [22., 24.],
        egui::Button::new("").frame(selected).selected(selected),
    );
    let rect = Rect::from_center_size(response.rect.center(), egui::Vec2::splat(16.));
    let color = ui
        .style()
        .interact_selectable(&response, selected)
        .fg_stroke
        .color;
    let dpi = ui.ctx().pixels_per_point();
    let physical_width = (1.25 * dpi).round().max(1.);
    let phase = if physical_width as i32 % 2 == 0 {
        0.
    } else {
        0.5
    };
    let stroke = egui::Stroke::new(physical_width / dpi, color);
    let p = |x: f32, y: f32| {
        let q = rect.min + egui::vec2(x, y);
        egui::pos2(
            ((q.x * dpi - phase).round() + phase) / dpi,
            ((q.y * dpi - phase).round() + phase) / dpi,
        )
    };
    let painter = ui.painter();
    match icon {
        PanelIcon::Clear => crate::icons::paint(ui, crate::icons::Icon::Trash, rect, color),
        PanelIcon::New => crate::icons::paint(ui, crate::icons::Icon::AddLayer, rect, color),
        PanelIcon::Delete => crate::icons::paint_layer_button(ui, rect, color, false),
        PanelIcon::Folder => {
            painter.add(egui::Shape::closed_line(
                vec![
                    p(1., 4.),
                    p(1., 2.),
                    p(6., 2.),
                    p(8., 5.),
                    p(15., 5.),
                    p(15., 14.),
                    p(1., 14.),
                ],
                stroke,
            ));
        }
        PanelIcon::Link => {
            for dx in [-3., 3.] {
                painter.rect_stroke(
                    Rect::from_center_size(rect.center() + egui::vec2(dx, 0.), egui::vec2(9., 6.)),
                    3.,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            painter.line_segment([p(5., 8.), p(11., 8.)], stroke);
        }
        PanelIcon::Mask => {
            painter.rect_filled(Rect::from_min_max(p(1., 3.), p(15., 13.)), 1., color);
            painter.circle_filled(p(8., 8.), 3., ui.visuals().panel_fill);
        }
        PanelIcon::Adjust => {
            painter.circle_stroke(p(8., 8.), 6., stroke);
            let mut pts = vec![p(8., 8.)];
            pts.extend((0..=16).map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 16.;
                p(8. + 6. * a.cos(), 8. + 6. * a.sin())
            }));
            painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
        }
        PanelIcon::Alpha => {
            for y in 0..3 {
                for x in 0..3 {
                    if (x + y) % 2 == 0 {
                        let a = rect.min + egui::vec2(x as f32 * 5., y as f32 * 5.);
                        let b = a + egui::Vec2::splat(5.);
                        let snap = |v: f32| (v * dpi).round() / dpi;
                        painter.rect_filled(
                            Rect::from_min_max(
                                egui::pos2(snap(a.x), snap(a.y)),
                                egui::pos2(snap(b.x), snap(b.y)),
                            ),
                            0.,
                            color,
                        );
                    }
                }
            }
        }
        PanelIcon::Effects => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "fx",
                egui::FontId::proportional(15.),
                color,
            );
        }
        PanelIcon::Paint => crate::icons::paint(ui, crate::icons::Icon::Brush, rect, color),
        PanelIcon::Move => crate::icons::paint(ui, crate::icons::Icon::Move, rect, color),
        PanelIcon::Lock => crate::icons::paint(ui, crate::icons::Icon::Lock, rect, color),
    }
    response.on_hover_text(tr(label))
}
fn panel_menu(
    ui: &mut egui::Ui,
    icon: PanelIcon,
    label: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    let response = panel_button(ui, icon, false, label);
    let id = ui.make_persistent_id(format!("layer-panel-{icon:?}"));
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(id));
    }
    egui::popup::popup_below_widget(
        ui,
        id,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(150.);
            content(ui);
        },
    );
}
fn queue_panel(ui: &egui::Ui, action: u8, target: u64) {
    ui.memory_mut(|m| m.close_popup());
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("tile_layer_panel_action"), (action, target)));
}

impl Editor {
    pub(super) fn mask_context_action(&mut self, index: usize, action: u8) {
        if index >= self.layers.len() || self.layers[index].settings.mask.is_none() {
            return;
        }
        if !(3..=6).contains(&action)
            && (self.layers[index].locked
                || self.layers[index].material
                || self.ancestor_locked(index))
        {
            return;
        }
        if action == 2
            && (self.layers[index].settings.group_id != 0
                || self.layers[index].settings.adjustment.is_some() || self.layers[index].settings.text.is_some()
                || self.layers[index].settings.vector.as_ref().is_some_and(|v| v.fill.is_some()))
        {
            return;
        }
        self.select_layer(index);
        self.record_layer_change();
        match action {
            0 => {
                self.layers[index].settings.mask_enabled = !self.layers[index].settings.mask_enabled
            }
            1 => {
                self.layers[index].settings.mask = None;
                if self.layers[index].settings.vector.as_ref().is_some_and(|v| v.fill.is_none()) {
                    self.layers[index].settings.vector = None;
                }
                self.mask_editing = false;
            }
            2 => {
                if let Some(mask) = self.layers[index].settings.mask.take() {
                    self.layers[index].settings.vector = None;
                    for (tile, values) in self.tiles.iter_mut().zip(mask) {
                        for (pixel, value) in tile.pixels_mut().zip(values) {
                            pixel[3] = (pixel[3] as u16 * value as u16 / 255) as u8;
                        }
                    }
                    self.pixels = self.tiles[self.selected].clone();
                    self.sync_layer();
                    self.mask_editing = false;
                }
            }
            3..=6 => {
                let mut pixels = std::collections::HashSet::new();
                let w = self.pixels.width();
                for (tile, mask) in self.layers[index]
                    .settings
                    .mask
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    for (i, value) in mask.iter().enumerate() {
                        let x = i as u32 % w;
                        let y = i as u32 / w;
                        if *value >= 128
                            && self.inside_tile(x as i32, y as i32, w, self.pixels.height())
                        {
                            pixels.insert((tile, x, y));
                        }
                    }
                }
                match action {
                    4 => {
                        if let Some(old) = &self.selection.mask {
                            pixels.extend(old.iter().copied());
                        }
                    }
                    5 => {
                        pixels = self
                            .selection
                            .mask
                            .as_ref()
                            .map(|old| old.difference(&pixels).copied().collect())
                            .unwrap_or_default();
                    }
                    6 => {
                        if let Some(old) = &self.selection.mask {
                            pixels.retain(|p| old.contains(p));
                        }
                    }
                    _ => {}
                }
                self.selection_from_mask_pixels(pixels);
            }
            7 => {
                for mask in self.layers[index].settings.mask.as_mut().unwrap() {
                    for value in mask {
                        *value = 255 - *value;
                    }
                }
            }
            8 => self.layers[index].settings.mask_linked = !self.layers[index].settings.mask_linked,
            _ => {}
        }
        self.invalidate_textures();
    }
}

#[cfg(test)]
mod mask_context_tests {
    use super::*;
    #[test]
    fn mask_menu_apply_and_selection_roundtrip() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(2, 2));
        let i = e.active_layer;
        e.write_pixel(0, 0, 0, Rgba([200, 100, 50, 255]));
        e.layers[i].settings.mask = Some(vec![vec![128, 0, 255, 255]]);
        e.mask_context_action(i, 3);
        assert!(e.selection.allows(0, 0, 0));
        assert!(!e.selection.allows(0, 1, 0));
        e.mask_context_action(i, 0);
        assert!(!e.layers[i].settings.mask_enabled);
        e.mask_context_action(i, 2);
        assert!(e.layers[i].settings.mask.is_none());
        assert_eq!(e.pixels.get_pixel(0, 0).0, [200, 100, 50, 128]);
        e.history(false);
        assert!(e.layers[i].settings.mask.is_some());
        assert_eq!(e.pixels.get_pixel(0, 0)[3], 255);
    }
}

pub(super) fn paint_adjustment_icon(ui: &egui::Ui, rect: Rect, adjust: &Adjustment) {
    let painter = ui.painter();
    painter.rect_filled(rect, 1., ui.visuals().panel_fill);
    let icon = Rect::from_center_size(rect.center(), egui::vec2(24., 20.));
    let p = |x: f32, y: f32| icon.min + egui::vec2(x / 24. * icon.width(), y / 20. * icon.height());
    let c = ui.visuals().text_color();
    let stroke = egui::Stroke::new(1.2_f32, c);
    painter.rect_stroke(icon, 1., stroke, egui::StrokeKind::Inside);
    match adjust {
        Adjustment::Curves { .. } | Adjustment::CurvesBezier { .. } => {
            for x in [8., 16.] {
                painter.line_segment(
                    [p(x, 1.), p(x, 19.)],
                    egui::Stroke::new(0.5_f32, c.gamma_multiply(0.4)),
                );
            }
            let points = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.;
                    p(2. + 20. * t, 18. - 16. * (t * t * (3. - 2. * t)))
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
        }
        Adjustment::Levels { .. } => {
            for (i, h) in [4., 7., 12., 6., 15., 10., 17., 7., 11.].iter().enumerate() {
                painter.line_segment(
                    [p(3. + i as f32 * 2., 17.), p(3. + i as f32 * 2., 17. - h)],
                    stroke,
                );
            }
        }
        Adjustment::Brightness { .. } => {
            painter.circle_stroke(p(12., 10.), 4., stroke);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.;
                let v = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([p(12., 10.) + v * 6., p(12., 10.) + v * 8.], stroke);
            }
        }
        Adjustment::Hue { .. } => {
            for (y, x) in [(5., 8.), (10., 16.), (15., 11.)] {
                painter.line_segment([p(3., y), p(21., y)], stroke);
                painter.circle_filled(p(x, y), 2., c);
            }
        }
    }
}

pub(super) fn paint_group_icon(ui: &egui::Ui, rect: Rect, expanded: bool) {
    let rect = Rect::from_center_size(rect.center(), egui::vec2(22., 18.));
    let dpi = ui.ctx().pixels_per_point();
    let p = |x: f32, y: f32| {
        egui::pos2(
            ((rect.left() + x) * dpi).round() / dpi,
            ((rect.top() + y) * dpi).round() / dpi,
        )
    };
    let c = ui.visuals().text_color();
    let stroke = egui::Stroke::new((1.25 * dpi).round().max(1.) / dpi, c);
    if expanded {
        ui.painter().add(egui::Shape::line(
            vec![
                p(1., 14.),
                p(1., 2.),
                p(8., 2.),
                p(11., 5.),
                p(19., 5.),
                p(19., 8.),
            ],
            stroke,
        ));
        ui.painter().add(egui::Shape::closed_line(
            vec![p(1., 16.), p(4., 8.), p(22., 8.), p(19., 16.)],
            stroke,
        ));
    } else {
        ui.painter()
            .rect_filled(Rect::from_min_max(p(1., 5.), p(21., 16.)), 1., c);
        ui.painter()
            .rect_filled(Rect::from_min_max(p(1., 2.), p(9., 6.)), 1., c);
    }
}

#[cfg(test)]
mod insertion_tests {
    use super::*;
    #[test]
    fn selected_group_adds_adjustment_above_all_children() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(4, 4));
        let child = e.active_layer;
        e.layers[child].settings.parent = 11;
        let mut group = e.layers[child].clone();
        group.settings = Properties::default();
        group.settings.group_id = 11;
        group.settings.collapsed = true;
        group.name = "group".into();
        e.layers.push(group);
        e.select_layer(e.layers.len() - 1);
        e.add_adjustment(Adjustment::default());
        let new = e.active_layer;
        assert_eq!(e.layers[new].settings.parent, 11);
        let order = e.group_display_order();
        let group = e
            .layers
            .iter()
            .position(|l| l.settings.group_id == 11)
            .unwrap();
        let group_pos = order.iter().position(|i| *i == group).unwrap();
        assert_eq!(order[group_pos + 1], new);
        assert!(!e.layers[group].settings.collapsed);
    }
}

impl Editor {
    pub(super) fn toggle_layer_detail(
        &mut self,
        index: usize,
        fx: bool,
        anchor: Rect,
        screen: Rect,
    ) {
        let same = self.active_layer == index
            && self
                .layer_detail
                .as_ref()
                .is_some_and(|(i, kind, _, _)| *i == index && *kind == fx);
        self.select_layer(index);

        self.text_dialog=None;
        if same {
            self.layer_detail = None;
        } else {
            self.detail_histogram_fresh = false;
            let position = if fx {
                self.fx_page = effects::Page::initial(&self.layers[index].settings.effects);
                self.fx_dragging = false;
                Pos2::ZERO
            } else {
                let height = match self.layers[index].settings.adjustment {
                    Some(Adjustment::Brightness { .. }) => 145.,
                    Some(Adjustment::Hue { .. }) => 220.,
                    _ => 330.,
                };
                tool_popup_position(anchor, screen, egui::vec2(DETAIL_WIDTH + 16., height), false)
            };
            self.layer_detail = Some((index, fx, self.layers[index].name.clone(), position));
            if !fx && self.layers[index].settings.text.is_some(){self.begin_text(Some(index));}
        }
    }
    pub(super) fn fx_window(&mut self, ctx: &egui::Context, path: &Path) {
        let Some((index,true,name,_)) = self.layer_detail.clone() else { return; };
        if index != self.active_layer || self.layers.get(index).is_none_or(|layer|
            layer.name != name || layer.material || layer.settings.adjustment.is_some()) {
            self.layer_detail = None;
            return;
        }
        if self.confirm_close || self.pending_open.is_some() || self.text_dialog.is_some() || self.transform.is_some() {
            self.fx_dragging=false;
            return;
        }
        let editable = !self.layers[index].locked && !self.ancestor_locked(index);
        let mut effects = self.layers[index].settings.effects.clone();
        let mut light = self.fx_light;
        let mut pointer_down = false;
        let mut history = None;
        let mut open = true;
        crate::native_window::Window::new(format!("{} · {}",name,tr("图层样式")))
            .id(egui::Id::new(("pixel_tile_fx",path))).open(&mut open)
            .default_width(effects::PANEL_WIDTH).default_height(520.).min_width(310.).min_height(225.)
            .always_on_top().scrollable(false)
            .show(ctx, |ui| {
                effects::detail_ui(ui,&mut effects,&mut self.fx_page,&mut light,editable,self.plain_image || self.layers[index].settings.text.is_some());
                pointer_down = ui.input(|i| i.pointer.primary_down());
                if !ui.ctx().wants_keyboard_input() && !pointer_down {
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL,egui::Key::Z)) {history=Some(false);}
                    else if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL,egui::Key::Y)) {history=Some(true);}
                }
            });
        if let Some(redo)=history {
            self.fx_dragging=false;
            self.finish_stroke();
            self.history(redo);
        } else if editable && (effects != self.layers[index].settings.effects || light != self.fx_light) {
            if self.fx_dragging {
                self.revision=self.revision.wrapping_add(1);
                self.changed=true;
            } else { self.record_layer_change(); }
            self.fx_dragging=pointer_down;
            effects.legacy_appearance=false;
            self.layers[index].settings.effects=effects;
            self.fx_light=light;
            self.invalidate_textures();
        }
        if !pointer_down {self.fx_dragging=false;}
        if !open {
            self.layer_detail=None;
            ctx.request_repaint_of(egui::ViewportId(egui::Id::new(("pixel_tile",path))));
        }
    }
    pub(super) fn layer_detail_ui(&mut self, ui: &mut egui::Ui) {
        let Some((index, false, name, position)) = self.layer_detail.clone() else {
            return;
        };
        if self.layers.get(index).is_some_and(|l|l.settings.text.is_some()) {return;}
        if index == self.active_layer
            && let Some(vector) = self.layers.get(index)
                .filter(|layer| layer.name == name)
                .and_then(|layer| layer.settings.vector.clone())
        {
            self.vector_detail_ui(ui, index, name, position, vector);
            return;
        }
        if index != self.active_layer
            || self.layers.get(index).is_none_or(|l| l.name != name || l.settings.adjustment.is_none())
        {
            self.layer_detail = None;
            return;
        }
        let mut adjustment = self.layers[index].settings.adjustment.clone();
        let histogram_id = ui.id().with(("tile_levels_input_histogram", index));
        let histogram = if matches!(adjustment, Some(Adjustment::Levels { .. })) {
            let cached = ui
                .ctx()
                .data(|d| d.get_temp::<(String, [f32; 256])>(histogram_id));
            if let Some((_name, bins)) = cached
                .filter(|(name, _)| self.detail_histogram_fresh && *name == self.layers[index].name)
            {
                Some(bins)
            } else {
                let mut bins = [0_f32; 256];
                let mut tiles = std::collections::HashSet::new();
                if self.multiple {
                    for row in 0..self.rows {
                        for col in 0..self.columns {
                            tiles.insert(self.tile_at(col as i32, row as i32));
                        }
                    }
                } else {
                    tiles.insert(self.selected);
                }
                for tile in tiles {
                    for (x, y, p) in self.adjustment_input(index, tile).enumerate_pixels() {
                        if p[3] > 0
                            && self.inside_tile(
                                x as i32,
                                y as i32,
                                self.pixels.width(),
                                self.pixels.height(),
                            )
                        {
                            for c in 0..3 {
                                bins[p[c] as usize] += p[3] as f32 / 255.;
                            }
                        }
                    }
                }
                ui.ctx().data_mut(|d| {
                    d.insert_temp(histogram_id, (self.layers[index].name.clone(), bins))
                });
                self.detail_histogram_fresh = true;
                Some(bins)
            }
        } else {
            None
        };
        let mut open = true;
        let enabled = !self.layers[index].locked
            && !self.ancestor_locked(index)
            && !self.layers[index].material
            && self.pending_open.is_none();
        egui::Window::new(name)
            .id(ui.id().with(("tile_layer_detail_window", false)))
            .open(&mut open)
            .collapsible(false)
            .auto_sized()
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
                ui.set_width(DETAIL_WIDTH.min((ui.ctx().screen_rect().width() - 28.).max(120.)));
                if let Some(adjust) = adjustment.as_mut() {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .max_height((ui.ctx().screen_rect().height() - 80.).max(100.))
                        .show(ui, |ui| {
                            ui.add_enabled_ui(enabled, |ui| adjustment_ui(ui, adjust, histogram.as_ref()));
                        });
                }
            });
        if enabled && adjustment != self.layers[index].settings.adjustment {
            self.record_layer_change();
            self.layers[index].settings.adjustment = adjustment;
            self.invalidate_textures();
        }
        if !open {
            self.layer_detail = None;
        }
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;
    #[test]
    fn detail_toggle_selects_target_and_other_layer_collapses() {
        let mut e = Editor::default();
        e.reset_document(RgbaImage::new(4, 4));
        let first = e.active_layer;
        e.layers[first].settings.effects = serde_json::from_value(serde_json::json!({"overlay":true})).unwrap();
        e.duplicate_layer(first, true);
        let second = e.active_layer;
        e.layers[second].settings.adjustment = Some(Adjustment::default());
        let anchor = Rect::from_min_size(Pos2::new(100., 100.), egui::vec2(20., 28.));
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800., 600.));
        e.toggle_layer_detail(first, true, anchor, screen);
        assert_eq!(e.active_layer, first);
        assert!(e
            .layer_detail
            .as_ref()
            .is_some_and(|(i, fx, _, _)| *i == first && *fx));
        e.toggle_layer_detail(first, true, anchor, screen);
        assert!(e.layer_detail.is_none());
        e.toggle_layer_detail(second, false, anchor, screen);
        assert_eq!(e.active_layer, second);
        e.select_layer(first);
        assert!(e.layer_detail.is_none());
    }
}

#[cfg(test)]
mod levels_tests {
    use super::*;
    #[test]
    fn output_levels_and_legacy_defaults_preserve_alpha() {
        let levels = Adjustment::Levels {
            black: 0,
            gamma: 1.,
            white: 255,
            output_black: 10,
            output_white: 200,
        };
        assert_eq!(levels.apply(Rgba([0, 255, 0, 128])).0, [10, 200, 10, 128]);
        let old: Adjustment = serde_json::from_value(
            serde_json::json!({"Levels":{"black":0,"gamma":1.,"white":255}}),
        )
        .unwrap();
        let p = Rgba([20, 100, 200, 128]);
        assert_eq!(old.apply(p), p);
    }
    #[test]
    fn levels_histogram_input_excludes_own_adjustment() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::from_pixel(4, 4, Rgba([20, 30, 40, 255])));
        let before = e.composite(0);
        e.add_adjustment(Adjustment::Levels {
            black: 20,
            gamma: 1.,
            white: 200,
            output_black: 10,
            output_white: 220,
        });
        assert_eq!(e.adjustment_input(e.active_layer, 0), before);
        assert_ne!(e.composite(0), before);
    }
}
