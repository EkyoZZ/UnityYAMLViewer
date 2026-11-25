//! Saved Tile layer styles. The original overlay, stroke and shadow keys stay flat in project JSON.
use super::{Blend, Properties};
use serde::{Deserialize, Serialize};

mod legacy;
mod psd;
pub(in crate::asset_viewer::graphic_editor) use psd::{
    read_live_rectangle, read_solid_fill, solid_shape_stroke_supported, write_live_rectangle,
    write_solid_fill,
};
mod render;
#[cfg(test)]
mod tests;
mod ui;
pub(in super::super) use ui::detail_ui;
pub(in super::super) const PANEL_WIDTH: f32 = 506.;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Light {
    pub(crate) angle: i16,
    pub(crate) altitude: u8,
}
impl Default for Light {
    fn default() -> Self {
        Self {
            angle: 120,
            altitude: 30,
        }
    }
}
impl Light {
    pub(crate) fn valid(self) -> bool {
        self.angle.unsigned_abs() <= 180 && self.altitude <= 90
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum Position {
    #[default]
    Outside,
    Inside,
    Center,
}
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum GradientStyle {
    #[default]
    Linear,
    Radial,
    Angle,
    Reflected,
    Diamond,
}
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum Contour {
    #[default]
    Linear,
    Smooth,
    Cone,
    Ring,
}
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum Relief {
    #[default]
    Inner,
    Outer,
    Emboss,
    Pillow,
}
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum Carve {
    #[default]
    Smooth,
    Hard,
    Soft,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
struct GradientStop {
    position: u16,
    color: [u8; 3],
}

const MAX_GRADIENT_STOPS: usize = 16;
const GRADIENT_END: u16 = 4096;
type GradientStops = [Option<GradientStop>; MAX_GRADIENT_STOPS];

fn no_gradient_stops(stops: &GradientStops) -> bool {
    stops.iter().all(Option::is_none)
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Gradient {
    colors: [[u8; 3]; 2],
    #[serde(skip_serializing_if = "no_gradient_stops")]
    stops: GradientStops,
    style: GradientStyle,
    angle: i16,
    scale: u16,
    reverse: bool,
    align_layer: bool,
    dither: bool,
}
impl Default for Gradient {
    fn default() -> Self {
        Self {
            colors: [[0, 0, 0], [255; 3]],
            stops: [None; MAX_GRADIENT_STOPS],
            style: GradientStyle::Linear,
            angle: 90,
            scale: 100,
            reverse: false,
            align_layer: true,
            dither: false,
        }
    }
}
impl Gradient {
    fn color_stops(&self) -> impl Iterator<Item = GradientStop> + '_ {
        let legacy = [0, GRADIENT_END]
            .into_iter()
            .zip(self.colors)
            .map(|(position, color)| GradientStop { position, color });
        let use_legacy = self.stops[0].is_none();
        self.stops
            .iter()
            .map_while(|stop| *stop)
            .chain(legacy.filter(move |_| use_legacy))
    }

    fn set_color_stops(&mut self, stops: &[GradientStop]) {
        self.colors = [stops[0].color, stops[stops.len() - 1].color];
        self.stops = [None; MAX_GRADIENT_STOPS];
        if stops.len() != 2 || stops[0].position != 0 || stops[1].position != GRADIENT_END {
            for (slot, stop) in self.stops.iter_mut().zip(stops) {
                *slot = Some(*stop);
            }
        }
    }

    fn sample(&self, t: f32) -> [u8; 3] {
        let position = t.clamp(0., 1.) * f32::from(GRADIENT_END);
        let mut stops = self.color_stops();
        let mut left = stops.next().unwrap();
        if position < f32::from(left.position) {
            return left.color;
        }
        for right in stops {
            if position < f32::from(right.position) {
                let k = (position - f32::from(left.position))
                    / f32::from(right.position - left.position);
                return std::array::from_fn(|c| {
                    (f32::from(left.color[c]) * (1. - k) + f32::from(right.color[c]) * k).round()
                        as u8
                });
            }
            left = right;
        }
        left.color
    }

    fn valid(self) -> bool {
        let count = self.stops.iter().flatten().count();
        self.angle.unsigned_abs() <= 180
            && (1..=400).contains(&self.scale)
            && (count == 0 || count >= 2)
            && self.stops[..count].iter().all(Option::is_some)
            && self
                .stops
                .iter()
                .flatten()
                .all(|stop| stop.position <= GRADIENT_END)
            && self.stops[..count]
                .windows(2)
                .all(|pair| pair[0].unwrap().position <= pair[1].unwrap().position)
    }
}

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
struct Quality {
    contour: Contour,
    antialias: bool,
    noise: u8,
}

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
struct ShadowShape {
    // Missing direction preserves both Cartesian components of a legacy shadow exactly.
    direction: Option<(i16, u8)>,
    global: bool,
    spread: u8,
    quality: Quality,
}
impl ShadowShape {
    fn polar(self, offset: [i16; 2]) -> (i16, u8) {
        self.direction.unwrap_or_else(|| {
            let [x, y] = offset.map(f32::from);
            (
                y.atan2(-x).to_degrees().round() as i16,
                x.hypot(y).round() as u8,
            )
        })
    }
    fn offset(self, legacy: [i16; 2], light: Light) -> [i32; 2] {
        if !self.global && self.direction.is_none() {
            return legacy.map(i32::from);
        }
        let (angle, distance) = self.polar(legacy);
        let (sin, cos) = f32::from(if self.global { light.angle } else { angle })
            .to_radians()
            .sin_cos();
        [
            (-cos * f32::from(distance)).round() as i32,
            (sin * f32::from(distance)).round() as i32,
        ]
    }
    fn valid(self) -> bool {
        self.direction
            .is_none_or(|(a, d)| a.unsigned_abs() <= 180 && d <= 128)
            && self.spread <= 100
            && self.quality.noise <= 100
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Ink {
    mode: Blend,
    color: [u8; 3],
    opacity: u8,
}
impl Default for Ink {
    fn default() -> Self {
        Self {
            mode: Blend::Normal,
            color: [0; 3],
            opacity: 100,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct InnerShadow {
    enabled: bool,
    ink: Ink,
    size: u8,
    shape: ShadowShape,
}
impl Default for InnerShadow {
    fn default() -> Self {
        Self {
            enabled: false,
            ink: Ink {
                mode: Blend::Multiply,
                opacity: 70,
                ..Ink::default()
            },
            size: 5,
            shape: ShadowShape {
                direction: Some((120, 5)),
                ..ShadowShape::default()
            },
        }
    }
}
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Glow {
    enabled: bool,
    ink: Ink,
    size: u8,
    spread: u8,
    range: u8,
    center: bool,
    precise: bool,
    gradient: bool,
    end_color: [u8; 3],
    #[serde(skip_serializing_if = "no_gradient_stops")]
    stops: GradientStops,
    jitter: u8,
    quality: Quality,
}
impl Default for Glow {
    fn default() -> Self {
        Self {
            enabled: false,
            ink: Ink {
                mode: Blend::Screen,
                color: [255, 246, 198],
                opacity: 75,
            },
            size: 7,
            spread: 0,
            range: 50,
            center: false,
            precise: false,
            gradient: false,
            end_color: [255; 3],
            stops: [None; MAX_GRADIENT_STOPS],
            jitter: 0,
            quality: Quality::default(),
        }
    }
}
impl Glow {
    fn color_ramp(&self) -> Gradient {
        Gradient {
            colors: [self.ink.color, self.end_color],
            stops: self.stops,
            ..Gradient::default()
        }
    }

    fn valid(self) -> bool {
        self.ink.opacity <= 100
            && (1..=64).contains(&self.size)
            && self.spread <= 100
            && (1..=100).contains(&self.range)
            && self.jitter <= 100
            && self.quality.noise <= 100
            && self.color_ramp().valid()
    }
}
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Bevel {
    enabled: bool,
    style: Relief,
    technique: Carve,
    depth: u16,
    down: bool,
    size: u8,
    soften: u8,
    light: Light,
    global: bool,
    contour: Contour,
    antialias: bool,
    highlight: Ink,
    shade: Ink,
}
impl Default for Bevel {
    fn default() -> Self {
        Self {
            enabled: false,
            style: Relief::Inner,
            technique: Carve::Smooth,
            depth: 100,
            down: false,
            size: 3,
            soften: 0,
            light: Light::default(),
            global: false,
            contour: Contour::Linear,
            antialias: false,
            highlight: Ink {
                mode: Blend::Screen,
                color: [255; 3],
                opacity: 100,
            },
            shade: Ink {
                mode: Blend::Multiply,
                opacity: 50,
                ..Ink::default()
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Stroke {
    #[serde(rename = "stroke")]
    enabled: bool,
    #[serde(rename = "stroke_size")]
    size: u8,
    #[serde(rename = "stroke_color")]
    color: [u8; 3],
    #[serde(rename = "stroke_position")]
    position: Position,
    #[serde(rename = "stroke_opacity")]
    opacity: u8,
    #[serde(rename = "stroke_mode")]
    mode: Blend,
    #[serde(rename = "stroke_gradient")]
    gradient: bool,
    #[serde(rename = "stroke_ramp")]
    ramp: Gradient,
}
impl Default for Stroke {
    fn default() -> Self {
        Self {
            enabled: false,
            size: 1,
            color: [0; 3],
            position: Position::Outside,
            opacity: 100,
            mode: Blend::Normal,
            gradient: false,
            ramp: Gradient::default(),
        }
    }
}
impl Stroke {
    fn valid(self) -> bool {
        self.size <= 64 && self.opacity <= 100 && self.ramp.valid()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct DropShadow {
    #[serde(rename = "shadow")]
    enabled: bool,
    #[serde(rename = "shadow_radius")]
    radius: u8,
    #[serde(rename = "shadow_offset")]
    offset: [i16; 2],
    #[serde(rename = "shadow_opacity")]
    opacity: u8,
    #[serde(rename = "shadow_color")]
    color: [u8; 3],
    #[serde(rename = "shadow_mode")]
    mode: Blend,
    #[serde(rename = "shadow_shape")]
    shape: ShadowShape,
    #[serde(rename = "shadow_knockout")]
    knockout: bool,
}
impl Default for DropShadow {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 3,
            offset: [3, 3],
            opacity: 50,
            color: [0; 3],
            mode: Blend::Normal,
            shape: ShadowShape::default(),
            knockout: false,
        }
    }
}
impl DropShadow {
    fn valid(self) -> bool {
        self.radius <= 64
            && self.opacity <= 100
            && self.offset.iter().all(|v| v.unsigned_abs() <= 32)
            && self.shape.valid()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct ColorOverlay {
    #[serde(rename = "overlay")]
    enabled: bool,
    #[serde(rename = "overlay_color")]
    color: [u8; 3],
    #[serde(rename = "overlay_opacity")]
    opacity: u8,
    #[serde(rename = "overlay_mode")]
    mode: Blend,
}
impl Default for ColorOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [255; 3],
            opacity: 100,
            mode: Blend::Normal,
        }
    }
}
impl ColorOverlay {
    fn valid(self) -> bool {
        self.opacity <= 100
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct GradientOverlay {
    #[serde(rename = "gradient")]
    enabled: bool,
    #[serde(rename = "gradient_ramp")]
    ramp: Gradient,
    #[serde(rename = "gradient_opacity")]
    opacity: u8,
    #[serde(rename = "gradient_mode")]
    mode: Blend,
}
impl Default for GradientOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            ramp: Gradient::default(),
            opacity: 100,
            mode: Blend::Normal,
        }
    }
}
impl GradientOverlay {
    fn valid(self) -> bool {
        self.opacity <= 100 && self.ramp.valid()
    }
}
impl InnerShadow {
    fn valid(self) -> bool {
        self.shape.valid() && self.size <= 64 && self.ink.opacity <= 100
    }
}

const MAX_INSTANCES: usize = 10;
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Effects {
    pub(crate) legacy_appearance: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    master_disabled: bool,
    #[serde(flatten)]
    stroke: Stroke,
    #[serde(flatten)]
    shadow: DropShadow,
    #[serde(flatten)]
    overlay: ColorOverlay,
    #[serde(flatten)]
    gradient: GradientOverlay,
    inner_shadow: InnerShadow,
    inner_glow: Glow,
    outer_glow: Glow,
    bevel: Bevel,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_strokes: Vec<Stroke>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_shadows: Vec<DropShadow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_overlays: Vec<ColorOverlay>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_gradients: Vec<GradientOverlay>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_inner_shadows: Vec<InnerShadow>,
}

fn rows<'a, T>(first: &'a T, extra: &'a [T]) -> impl DoubleEndedIterator<Item = &'a T> {
    std::iter::once(first).chain(extra.iter())
}
fn row_at<'a, T>(first: &'a T, extra: &'a [T], index: usize) -> &'a T {
    if index == 0 { first } else { &extra[index - 1] }
}
fn row_mut<'a, T>(first: &'a mut T, extra: &'a mut [T], index: usize) -> &'a mut T {
    if index == 0 {
        first
    } else {
        &mut extra[index - 1]
    }
}

impl Effects {
    fn count(&self, kind: Kind) -> usize {
        1 + match kind {
            Kind::Stroke => self.extra_strokes.len(),
            Kind::Shadow => self.extra_shadows.len(),
            Kind::Overlay => self.extra_overlays.len(),
            Kind::Gradient => self.extra_gradients.len(),
            Kind::InnerShadow => self.extra_inner_shadows.len(),
            _ => 0,
        }
    }
    pub(crate) fn has_extra(&self) -> bool {
        Kind::ALL.into_iter().any(|kind| self.count(kind) > 1)
    }
    pub(in super::super) fn extra_bytes(&self) -> usize {
        std::mem::size_of_val(self.extra_strokes.as_slice())
            + std::mem::size_of_val(self.extra_shadows.as_slice())
            + std::mem::size_of_val(self.extra_overlays.as_slice())
            + std::mem::size_of_val(self.extra_gradients.as_slice())
            + std::mem::size_of_val(self.extra_inner_shadows.as_slice())
    }
    fn add(&mut self, kind: Kind) -> Option<Page> {
        let index = self.count(kind);
        if index >= MAX_INSTANCES {
            return None;
        }
        match kind {
            Kind::Stroke => self.extra_strokes.push(Stroke {
                enabled: true,
                ..self.stroke
            }),
            Kind::Shadow => self.extra_shadows.push(DropShadow {
                enabled: true,
                ..self.shadow
            }),
            Kind::Overlay => self.extra_overlays.push(ColorOverlay {
                enabled: true,
                ..self.overlay
            }),
            Kind::Gradient => self.extra_gradients.push(GradientOverlay {
                enabled: true,
                ..self.gradient
            }),
            Kind::InnerShadow => self.extra_inner_shadows.push(InnerShadow {
                enabled: true,
                ..self.inner_shadow
            }),
            _ => return None,
        }
        self.legacy_appearance = false;
        Some(Page { kind, index })
    }
    fn remove(&mut self, row: Page, selected: &mut Page) {
        if row.index == 0 || row.index >= self.count(row.kind) {
            return;
        }
        match row.kind {
            Kind::Stroke => {
                self.extra_strokes.remove(row.index - 1);
            }
            Kind::Shadow => {
                self.extra_shadows.remove(row.index - 1);
            }
            Kind::Overlay => {
                self.extra_overlays.remove(row.index - 1);
            }
            Kind::Gradient => {
                self.extra_gradients.remove(row.index - 1);
            }
            Kind::InnerShadow => {
                self.extra_inner_shadows.remove(row.index - 1);
            }
            _ => return,
        }
        if selected.kind == row.kind && selected.index >= row.index {
            selected.index -= 1;
        }
    }
    fn edit(&mut self, page: Page) -> EntryMut<'_> {
        match page.kind {
            Kind::Stroke => EntryMut::Stroke(row_mut(
                &mut self.stroke,
                &mut self.extra_strokes,
                page.index,
            )),
            Kind::Shadow => EntryMut::Shadow(row_mut(
                &mut self.shadow,
                &mut self.extra_shadows,
                page.index,
            )),
            Kind::Overlay => EntryMut::Overlay(row_mut(
                &mut self.overlay,
                &mut self.extra_overlays,
                page.index,
            )),
            Kind::Gradient => EntryMut::Gradient(row_mut(
                &mut self.gradient,
                &mut self.extra_gradients,
                page.index,
            )),
            Kind::InnerShadow => EntryMut::InnerShadow(row_mut(
                &mut self.inner_shadow,
                &mut self.extra_inner_shadows,
                page.index,
            )),
            Kind::Bevel => EntryMut::Bevel(&mut self.bevel),
            Kind::InnerGlow => EntryMut::InnerGlow(&mut self.inner_glow),
            Kind::OuterGlow => EntryMut::OuterGlow(&mut self.outer_glow),
        }
    }
    fn enabled(&self, page: Page) -> bool {
        match page.kind {
            Kind::Stroke => row_at(&self.stroke, &self.extra_strokes, page.index).enabled,
            Kind::Shadow => row_at(&self.shadow, &self.extra_shadows, page.index).enabled,
            Kind::Overlay => row_at(&self.overlay, &self.extra_overlays, page.index).enabled,
            Kind::Gradient => row_at(&self.gradient, &self.extra_gradients, page.index).enabled,
            Kind::InnerShadow => {
                row_at(&self.inner_shadow, &self.extra_inner_shadows, page.index).enabled
            }
            Kind::Bevel => self.bevel.enabled,
            Kind::InnerGlow => self.inner_glow.enabled,
            Kind::OuterGlow => self.outer_glow.enabled,
        }
    }
    pub(super) fn valid(&self) -> bool {
        (!self.legacy_appearance || !self.has_extra())
            && (self.extra_strokes.len() < MAX_INSTANCES
                && rows(&self.stroke, &self.extra_strokes).all(|r| r.valid()))
            && (self.extra_shadows.len() < MAX_INSTANCES
                && rows(&self.shadow, &self.extra_shadows).all(|r| r.valid()))
            && (self.extra_overlays.len() < MAX_INSTANCES
                && rows(&self.overlay, &self.extra_overlays).all(|r| r.valid()))
            && (self.extra_gradients.len() < MAX_INSTANCES
                && rows(&self.gradient, &self.extra_gradients).all(|r| r.valid()))
            && (self.extra_inner_shadows.len() < MAX_INSTANCES
                && rows(&self.inner_shadow, &self.extra_inner_shadows).all(|r| r.valid()))
            && self.inner_glow.valid()
            && self.outer_glow.valid()
            && self.bevel.light.valid()
            && (1..=64).contains(&self.bevel.size)
            && (1..=1000).contains(&self.bevel.depth)
            && self.bevel.soften <= 16
            && self.bevel.highlight.opacity <= 100
            && self.bevel.shade.opacity <= 100
    }
    fn pages(&self) -> impl Iterator<Item = Page> + '_ {
        Kind::ALL
            .into_iter()
            .flat_map(|kind| (0..self.count(kind)).map(move |index| Page { kind, index }))
    }
    pub(super) fn active(&self) -> bool {
        !self.master_disabled && self.pages().any(|page| self.enabled(page))
    }
}

enum EntryMut<'a> {
    Stroke(&'a mut Stroke),
    Shadow(&'a mut DropShadow),
    Overlay(&'a mut ColorOverlay),
    Gradient(&'a mut GradientOverlay),
    InnerShadow(&'a mut InnerShadow),
    Bevel(&'a mut Bevel),
    InnerGlow(&'a mut Glow),
    OuterGlow(&'a mut Glow),
}
impl EntryMut<'_> {
    fn switch(&mut self) -> &mut bool {
        match self {
            Self::Stroke(s) => &mut s.enabled,
            Self::Shadow(s) => &mut s.enabled,
            Self::Overlay(s) => &mut s.enabled,
            Self::Gradient(s) => &mut s.enabled,
            Self::InnerShadow(s) => &mut s.enabled,
            Self::Bevel(s) => &mut s.enabled,
            Self::InnerGlow(s) => &mut s.enabled,
            Self::OuterGlow(s) => &mut s.enabled,
        }
    }
    fn reset(mut self) {
        let enabled = *self.switch();
        match &mut self {
            Self::Stroke(s) => **s = Default::default(),
            Self::Shadow(s) => **s = Default::default(),
            Self::Overlay(s) => **s = Default::default(),
            Self::Gradient(s) => **s = Default::default(),
            Self::InnerShadow(s) => **s = Default::default(),
            Self::Bevel(s) => **s = Default::default(),
            Self::InnerGlow(s) => **s = Default::default(),
            Self::OuterGlow(s) => **s = Default::default(),
        }
        *self.switch() = enabled;
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    Bevel,
    #[default]
    Stroke,
    InnerShadow,
    InnerGlow,
    Overlay,
    Gradient,
    OuterGlow,
    Shadow,
}
impl Kind {
    const ALL: [Self; 8] = [
        Self::Bevel,
        Self::Stroke,
        Self::InnerShadow,
        Self::InnerGlow,
        Self::Overlay,
        Self::Gradient,
        Self::OuterGlow,
        Self::Shadow,
    ];
    fn repeatable(self) -> bool {
        matches!(
            self,
            Self::Stroke | Self::InnerShadow | Self::Overlay | Self::Gradient | Self::Shadow
        )
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct Page {
    kind: Kind,
    index: usize,
}
impl Page {
    pub(super) fn initial(fx: &Effects) -> Self {
        fx.pages().find(|p| fx.enabled(*p)).unwrap_or_default()
    }
}
