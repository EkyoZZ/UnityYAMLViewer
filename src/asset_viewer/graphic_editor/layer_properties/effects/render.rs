//! Per-cell compositor following PhotoCraft's effect stack and global light convention.
//! Reference revision and upstream paths are recorded in docs/versions/1.3.7.md.
use super::super::{dilate, over};
use super::*;
use crate::asset_viewer::graphic_editor::layer_groups::mix_group;
use image::{GrayImage, Luma, Rgba, RgbaImage};

fn percent(n: u8) -> f32 {
    f32::from(n) / 100.
}
fn paint(pixel: &mut Rgba<u8>, color: [u8; 3], mode: Blend, weight: f32) {
    if weight > 0. {
        *pixel = over(
            *pixel,
            Rgba([color[0], color[1], color[2], 255]),
            mode,
            weight,
        );
    }
}
fn grain(x: u32, y: u32) -> f32 {
    // Stable across previews, exports and undo.
    let n = x.wrapping_mul(0x9e3779b9) ^ y.wrapping_mul(0x85ebca6b);
    let n = (n ^ (n >> 16)).wrapping_mul(0x7feb352d);
    ((n ^ (n >> 15)) & 0xffff) as f32 / 65535.
}

struct GradientMap {
    gradient: Gradient,
    center: [f32; 2],
    axis: [f32; 2],
    radius: f32,
}
impl GradientMap {
    fn new(gradient: Gradient, bounds: [u32; 4]) -> Self {
        Self::from_frame(gradient, bounds.map(|n| n as f32))
    }
    fn from_frame(gradient: Gradient, bounds: [f32; 4]) -> Self {
        let [left, top, right, bottom] = bounds;
        let (sin, cos) = f32::from(gradient.angle).to_radians().sin_cos();
        let w = (right - left + 1.).max(1.);
        let h = (bottom - top + 1.).max(1.);
        let span = if matches!(
            gradient.style,
            GradientStyle::Linear | GradientStyle::Reflected
        ) {
            (w / cos.abs().max(1e-6)).min(h / sin.abs().max(1e-6))
        } else {
            (w * cos).hypot(h * sin)
        };
        Self {
            gradient,
            center: [(left + right) / 2., (top + bottom) / 2.],
            axis: [cos, -sin],
            radius: (span * f32::from(gradient.scale) / 200.).max(0.5),
        }
    }
    fn color(&self, x: u32, y: u32) -> [u8; 3] {
        let dx = (x as f32 - self.center[0]) / self.radius;
        let dy = (y as f32 - self.center[1]) / self.radius;
        let u = dx * self.axis[0] + dy * self.axis[1];
        let v = dy * self.axis[0] - dx * self.axis[1];
        let mut t = match self.gradient.style {
            GradientStyle::Linear => (u + 1.) / 2.,
            GradientStyle::Reflected => u.abs(),
            GradientStyle::Radial => dx.hypot(dy),
            GradientStyle::Angle => (v.atan2(u) / std::f32::consts::TAU).rem_euclid(1.),
            GradientStyle::Diamond => u.abs() + v.abs(),
        };
        if self.gradient.reverse {
            t = 1. - t;
        }
        if self.gradient.dither {
            t += (grain(x, y) - 0.5) / 255.;
        }
        self.gradient.sample(t)
    }
}
impl Contour {
    fn sample(self, t: f32, antialias: bool) -> f32 {
        fn point(curve: Contour, t: f32) -> f32 {
            match curve {
                Contour::Linear => t,
                Contour::Smooth => t * t * (3. - 2. * t),
                Contour::Cone => 1. - (2. * t - 1.).abs(),
                Contour::Ring => (t * std::f32::consts::TAU).sin().powi(2),
            }
        }
        if antialias && t > 0. && t < 1. {
            (point(self, (t - 0.025).max(0.))
                + 2. * point(self, t)
                + point(self, (t + 0.025).min(1.)))
                / 4.
        } else {
            point(self, t)
        }
    }
}
impl Quality {
    fn coverage(self, t: f32, x: u32, y: u32) -> f32 {
        let level = self.contour.sample(t.clamp(0., 1.), self.antialias);
        if self.noise == 0 {
            return level;
        }
        (level + (2. * grain(x, y) - 1.) * percent(self.noise) * level.sqrt()).clamp(0., 1.)
    }
}

// Separable squared Euclidean distance transform. Each scanline is the lower
// envelope of parabolas centred on the source samples. Wide outlines stay round.
fn edt_line(line: &mut [f32], result: &mut [f32], sites: &mut [usize], cuts: &mut [f32]) {
    let mut last = 0;
    sites[0] = 0;
    cuts[0] = f32::NEG_INFINITY;
    for q in 1..line.len() {
        let mut crossing;
        loop {
            let p = sites[last];
            crossing = (line[q] - line[p]) / (2. * (q - p) as f32) + (q + p) as f32 / 2.;
            if crossing > cuts[last] {
                break;
            }
            last -= 1;
        }
        last += 1;
        sites[last] = q;
        cuts[last] = crossing;
    }
    let mut active = 0;
    for q in 0..line.len() {
        while active < last && cuts[active + 1] < q as f32 {
            active += 1;
        }
        let p = sites[active];
        result[q] = line[p] + (q as f32 - p as f32).powi(2);
    }
    line.copy_from_slice(&result[..line.len()]);
}
fn distances(alpha: &GrayImage, inside: bool, periodic: bool) -> Vec<f32> {
    let (w, h) = (alpha.width() as usize, alpha.height() as usize);
    let far = ((w + h + 1).max(1024) as f32).powi(2);
    let mut field: Vec<_> = alpha
        .as_raw()
        .iter()
        .map(|a| {
            let opacity = f32::from(*a) / 255.;
            let d = if inside { opacity } else { 1. - opacity };
            if d < 1. { d * d } else { far }
        })
        .collect();
    let n = w.max(h);
    let mut line = vec![0.; n];
    let span = if periodic { 3 * n } else { n };
    let mut result = vec![0.; span];
    let mut sites = vec![0; span];
    let mut cuts = vec![0.; span];
    let mut repeated = vec![0.; if periodic { span } else { 0 }];
    let mut transform = |samples: &mut [f32]| {
        if periodic {
            let k = samples.len();
            let window = &mut repeated[..3 * k];
            for copy in window.chunks_exact_mut(k) {
                copy.copy_from_slice(samples);
            }
            edt_line(window, &mut result, &mut sites, &mut cuts);
            samples.copy_from_slice(&window[k..2 * k]);
        } else {
            edt_line(samples, &mut result, &mut sites, &mut cuts);
        }
    };
    for row in field.chunks_exact_mut(w) {
        transform(row);
    }
    for x in 0..w {
        for y in 0..h {
            line[y] = field[y * w + x];
        }
        transform(&mut line[..h]);
        for y in 0..h {
            let d = line[y].sqrt();
            field[y * w + x] = if inside && !periodic {
                d.min((x + 1).min(w - x).min(y + 1).min(h - y) as f32)
            } else {
                d
            };
        }
    }
    field
}
impl Glow {
    fn sample(
        self,
        soft: Option<&ShadowMap>,
        distance: &[f32],
        i: usize,
        x: u32,
        y: u32,
        inner: bool,
    ) -> ([u8; 3], f32) {
        let v = if let Some(map) = soft {
            map.sample(x, y)
        } else {
            let size = f32::from(self.size);
            let solid = size * percent(self.spread);
            let d = distance[i];
            if d <= solid {
                1.
            } else {
                0.5 * (1. - (d - solid) / (size + 1. - solid)).clamp(0., 1.)
            }
        };
        let shaped = self
            .quality
            .contour
            .sample((v / percent(self.range)).min(1.), self.quality.antialias);
        let strength = if inner && self.center {
            1. - shaped
        } else {
            shaped
        };
        let color = if self.gradient {
            let stop = (1. - strength + (grain(y, x) - 0.5) * percent(self.jitter)).clamp(0., 1.);
            self.color_ramp().sample(stop)
        } else {
            self.ink.color
        };
        let noise = Quality {
            noise: self.quality.noise,
            ..Default::default()
        };
        (
            color,
            noise.coverage(strength, x, y) * percent(self.ink.opacity),
        )
    }
}

// A pair of sliding box filters produces the tent falloff used by soft styles.
fn tent_blur(mut image: GrayImage, size: u8) -> GrayImage {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut tmp = image.clone();
    for radius in [usize::from(size).div_ceil(2), usize::from(size) / 2] {
        if radius == 0 {
            continue;
        }
        let count = (radius * 2 + 1) as u32;
        for row in 0..h {
            let src = &image.as_raw()[row * w..][..w];
            let dst = &mut tmp.as_mut()[row * w..][..w];
            let mut sum = (radius as u32) * u32::from(src[0]);
            for k in 0..=radius {
                sum += u32::from(src[k.min(w - 1)]);
            }
            for x in 0..w {
                dst[x] = (sum / count) as u8;
                sum += u32::from(src[(x + radius + 1).min(w - 1)]);
                sum -= u32::from(src[x.saturating_sub(radius)]);
            }
        }
        for x in 0..w {
            let src = tmp.as_raw();
            let mut sum = radius as u32 * u32::from(src[x]);
            for k in 0..=radius {
                sum += u32::from(src[k.min(h - 1) * w + x]);
            }
            for y in 0..h {
                image.as_mut()[y * w + x] = (sum / count) as u8;
                sum += u32::from(src[(y + radius + 1).min(h - 1) * w + x]);
                sum -= u32::from(src[y.saturating_sub(radius) * w + x]);
            }
        }
    }
    image
}

struct ShadowMap {
    alpha: GrayImage,
    offset: [i32; 2],
    inner: bool,
    period: Option<[u32; 2]>,
    pad: i32,
}
impl ShadowMap {
    fn new(
        mask: &GrayImage,
        radius: u8,
        shape: ShadowShape,
        offset: [i16; 2],
        light: Light,
        inner: bool,
        periodic: bool,
    ) -> Self {
        let spread = (f32::from(radius) * percent(shape.spread)).round() as usize;
        // Inner shadows need the transparent area just outside the tile as a caster too.
        let pad = u32::from(radius) + 1;
        let mut alpha =
            GrayImage::from_fn(mask.width() + 2 * pad, mask.height() + 2 * pad, |x, y| {
                let a = if periodic {
                    mask.get_pixel(
                        (x as i32 - pad as i32).rem_euclid(mask.width() as i32) as u32,
                        (y as i32 - pad as i32).rem_euclid(mask.height() as i32) as u32,
                    )[0]
                } else {
                    x.checked_sub(pad)
                        .zip(y.checked_sub(pad))
                        .and_then(|(x, y)| mask.get_pixel_checked(x, y))
                        .map_or(0, |p| p[0])
                };
                Luma([if inner { 255 - a } else { a }])
            });
        if spread > 0 {
            alpha = dilate(&alpha, spread);
        }
        let blur = radius.saturating_sub(spread as u8);
        if blur > 0 {
            alpha = tent_blur(alpha, blur);
        }
        let offset = shape.offset(offset, light).map(|n| n - pad as i32);
        Self {
            alpha,
            offset,
            inner,
            period: periodic.then_some([mask.width(), mask.height()]),
            pad: pad as i32,
        }
    }
    fn sample(&self, x: u32, y: u32) -> f32 {
        let mut x = x as i32 - self.offset[0];
        let mut y = y as i32 - self.offset[1];
        if let Some([w, h]) = self.period {
            x = (x - self.pad).rem_euclid(w as i32) + self.pad;
            y = (y - self.pad).rem_euclid(h as i32) + self.pad;
        }
        self.alpha
            .get_pixel_checked(x as u32, y as u32)
            .map_or(if self.inner { 1. } else { 0. }, |p| f32::from(p[0]) / 255.)
    }
}

struct BevelMap {
    height: GrayImage,
    inset: u32,
    light: [f32; 3],
    bevel: Bevel,
}
impl BevelMap {
    fn new(
        bevel: Bevel,
        inside: &[f32],
        outside: &[f32],
        w: u32,
        h: u32,
        global: Light,
        periodic: bool,
    ) -> Self {
        let size = f32::from(bevel.size);
        let height_at = |d: f32| {
            let t = match bevel.style {
                Relief::Inner => d / size,
                Relief::Outer => 1. + d / size,
                Relief::Emboss => 0.5 + d / (2. * size),
                Relief::Pillow => (d / size).abs(),
            }
            .clamp(0., 1.);
            let t = match bevel.technique {
                Carve::Smooth => t * t * (3. - 2. * t),
                Carve::Hard => t,
                Carve::Soft => (t * std::f32::consts::FRAC_PI_2).sin(),
            };
            (255. * t).round() as u8
        };
        // Include a border to retain the slope on shapes which touch the canvas edge.
        let pad = if periodic {
            1 + 4 * u32::from(bevel.soften)
        } else {
            1
        };
        let mut height = GrayImage::from_fn(w + 2 * pad, h + 2 * pad, |x, y| {
            let coord = |n: u32, side: u32| {
                if periodic {
                    (n as i32 - pad as i32).rem_euclid(side as i32) as u32
                } else {
                    n.saturating_sub(1).min(side - 1)
                }
            };
            let sx = coord(x, w);
            let sy = coord(y, h);
            let i = (sy * w + sx) as usize;
            let d = if !periodic && (x == 0 || y == 0 || x == w + 1 || y == h + 1) {
                -outside[i] - 0.5
            } else {
                inside[i] - outside[i]
            };
            Luma([height_at(d)])
        });
        if bevel.soften > 0 {
            height = image::imageops::blur(&height, f32::from(bevel.soften));
        }
        let l = if bevel.global { global } else { bevel.light };
        let (sin, cos) = f32::from(l.angle).to_radians().sin_cos();
        let (up, flat) = f32::from(l.altitude).to_radians().sin_cos();
        Self {
            height,
            inset: pad - 1,
            light: [cos * flat, -sin * flat, up],
            bevel,
        }
    }
    fn sample(&self, x: u32, y: u32) -> (Ink, f32) {
        let z = |dx, dy| {
            f32::from(
                self.height
                    .get_pixel(x + self.inset + dx, y + self.inset + dy)[0],
            ) / 255.
        };
        let factor = f32::from(self.bevel.depth) / 100.
            * f32::from(self.bevel.size)
            * if self.bevel.down { -0.5 } else { 0.5 };
        let nx = (z(0, 1) - z(2, 1)) * factor;
        let ny = (z(1, 0) - z(1, 2)) * factor;
        let [lx, ly, lz] = self.light;
        let diff = (nx * lx + ny * ly + lz) / (1. + nx * nx + ny * ny).sqrt() - lz;
        let ink = if diff >= 0. {
            self.bevel.highlight
        } else {
            self.bevel.shade
        };
        (
            ink,
            self.bevel
                .contour
                .sample(diff.abs().min(1.), self.bevel.antialias)
                * percent(ink.opacity),
        )
    }
}

struct StrokeMap {
    style: Stroke,
    inner: f32,
    outer: f32,
    gradient: Option<GradientMap>,
}
impl StrokeMap {
    fn new(style: Stroke, bounds: [u32; 4], cell: [u32; 4]) -> Self {
        let width = f32::from(style.size);
        let (inner, outer) = match style.position {
            Position::Outside => (0., width),
            Position::Inside => (width, 0.),
            Position::Center => (width / 2., width / 2.),
        };
        let gradient = style.gradient.then(|| {
            let grow = (outer.ceil() as u32).saturating_sub(1);
            let frame = if style.ramp.align_layer {
                [
                    bounds[0].saturating_sub(grow),
                    bounds[1].saturating_sub(grow),
                    bounds[2].saturating_add(grow),
                    bounds[3].saturating_add(grow),
                ]
            } else {
                cell
            };
            GradientMap::new(style.ramp, frame)
        });
        Self {
            style,
            inner,
            outer,
            gradient,
        }
    }
    fn paint(&self, pixel: &mut Rgba<u8>, width: f32, distance: f32, x: u32, y: u32) {
        let weight = (width + 1. - distance).clamp(0., 1.) * percent(self.style.opacity);
        let color = self
            .gradient
            .as_ref()
            .map_or(self.style.color, |g| g.color(x, y));
        paint(pixel, color, self.style.mode, weight);
    }
}

impl Effects {
    pub(in super::super) fn has_gradient_overlay(&self) -> bool {
        rows(&self.gradient, &self.extra_gradients).any(|effect| effect.enabled)
    }
    pub(in super::super) fn has_aligned_gradient_overlay(&self) -> bool {
        rows(&self.gradient, &self.extra_gradients).any(|effect| effect.enabled && effect.ramp.align_layer)
    }
    pub(in super::super) fn composite(
        &self,
        p: &Properties,
        out: &mut RgbaImage,
        source: &RgbaImage,
        tile: usize,
        light: Light,
        gradient_frames: Option<[[f32; 4]; 2]>,
        text_gradient: Option<&crate::asset_viewer::graphic_editor::text_layer::TextGradient>,
    ) {
        if p.opacity == 0 {
            return;
        }
        let periodic = p.text.as_ref().is_some_and(|t| t.repeat);
        if self.legacy_appearance && !periodic && gradient_frames.is_none() {
            self.composite_legacy(p, out, source, tile);
            return;
        }
        let (w, h) = source.dimensions();
        let mut bounds = [w, h, 0, 0];
        let alpha = GrayImage::from_fn(w, h, |x, y| {
            let i = (y * w + x) as usize;
            let a = (f32::from(source.get_pixel(x, y)[3]) * p.mask_value(tile, i)).round() as u8;
            if a > 0 {
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x),
                    bounds[3].max(y),
                ];
            }
            Luma([a])
        });
        if bounds[0] == w {
            return;
        }
        let cell = [0, 0, w - 1, h - 1];
        // The first row is visually on top. All maps sample the same masked source.
        let gradients: Vec<_> = rows(&self.gradient, &self.extra_gradients)
            .rev()
            .filter(|s| s.enabled)
            .map(|s| {
                let frame = if s.ramp.align_layer { bounds } else { cell };
                let map = GradientMap::from_frame(s.ramp, gradient_frames.map_or(frame.map(|n| n as f32), |frames| frames[usize::from(!s.ramp.align_layer)]));
                let text_map = text_gradient.filter(|_| s.ramp.align_layer)
                    .map(|text| GradientMap::new(s.ramp, text.bounds));
                (s, map, text_map)
            })
            .collect();
        let overlays: Vec<_> = rows(&self.overlay, &self.extra_overlays)
            .rev()
            .filter(|s| s.enabled)
            .collect();
        let strokes: Vec<_> = rows(&self.stroke, &self.extra_strokes)
            .rev()
            .filter(|s| s.enabled)
            .map(|s| StrokeMap::new(*s, bounds, cell))
            .collect();
        let inside = (strokes.iter().any(|s| s.inner > 0.)
            || self.bevel.enabled
            || (self.inner_glow.enabled && self.inner_glow.precise))
            .then(|| distances(&alpha, true, periodic))
            .unwrap_or_default();
        let outside = (self.bevel.enabled
            || (self.outer_glow.enabled && self.outer_glow.precise)
            || strokes.iter().any(|s| s.outer > 0.))
        .then(|| distances(&alpha, false, periodic))
        .unwrap_or_default();
        let soft_glow = |g: Glow, inner| {
            (g.enabled && !g.precise).then(|| {
                ShadowMap::new(
                    &alpha,
                    g.size,
                    ShadowShape {
                        spread: g.spread,
                        ..Default::default()
                    },
                    [0, 0],
                    light,
                    inner,
                    periodic,
                )
            })
        };
        let inner_soft = soft_glow(self.inner_glow, true);
        let outer_soft = soft_glow(self.outer_glow, false);
        let bevel = self
            .bevel
            .enabled
            .then(|| BevelMap::new(self.bevel, &inside, &outside, w, h, light, periodic));
        let shadows: Vec<_> = rows(&self.shadow, &self.extra_shadows)
            .rev()
            .filter(|s| s.enabled)
            .map(|s| {
                (
                    s,
                    ShadowMap::new(&alpha, s.radius, s.shape, s.offset, light, false, periodic),
                )
            })
            .collect();
        let inner_shadows: Vec<_> = rows(&self.inner_shadow, &self.extra_inner_shadows)
            .rev()
            .filter(|s| s.enabled)
            .map(|s| {
                (
                    s,
                    ShadowMap::new(&alpha, s.size, s.shape, [0, 0], light, true, periodic),
                )
            })
            .collect();

        // Use the covered part of the pixel as its own little layer stack. Apply the
        // masked shape alpha once, then apply layer opacity to the complete result.
        for (x, y, dst) in out.enumerate_pixels_mut() {
            let i = (y * w + x) as usize;
            let a = alpha.as_raw()[i];
            let coverage = f32::from(a) / 255.;
            let mut layer = *dst;
            let mut inside_backdrop = *dst;
            for (s, map) in &shadows {
                let k = s.shape.quality.coverage(map.sample(x, y), x, y) * percent(s.opacity);
                if a < 255 {
                    paint(&mut layer, s.color, s.mode, k);
                }
                if a > 0 && !s.knockout {
                    paint(&mut inside_backdrop, s.color, s.mode, k);
                }
            }
            if self.outer_glow.enabled && a < 255 {
                let (color, k) =
                    self.outer_glow
                        .sample(outer_soft.as_ref(), &outside, i, x, y, false);
                paint(&mut layer, color, self.outer_glow.ink.mode, k);
            }
            if a < 255 {
                for s in &strokes {
                    if s.outer > 0. {
                        s.paint(&mut layer, s.outer, outside[i], x, y);
                    }
                }
            }
            let shading = bevel.as_ref().map(|b| b.sample(x, y));
            if self.bevel.style != Relief::Inner && a < 255 {
                if let Some((ink, k)) = shading {
                    paint(&mut layer, ink.color, ink.mode, k);
                }
            }
            if a > 0 {
                let mut src = *source.get_pixel(x, y);
                src[3] = 255;
                src = over(inside_backdrop, src, p.blend, percent(p.fill));
                for (s, gradient, text_map) in &gradients {
                    let color = text_map.as_ref().zip(text_gradient.and_then(|text| text.point(i)))
                        .map_or_else(|| gradient.color(x, y), |(map, [x, y])| map.color(x, y));
                    paint(&mut src, color, s.mode, percent(s.opacity));
                }
                for s in &overlays {
                    paint(&mut src, s.color, s.mode, percent(s.opacity));
                }
                if self.inner_glow.enabled {
                    let (color, k) =
                        self.inner_glow
                            .sample(inner_soft.as_ref(), &inside, i, x, y, true);
                    paint(&mut src, color, self.inner_glow.ink.mode, k);
                }
                for (s, map) in &inner_shadows {
                    let k =
                        s.shape.quality.coverage(map.sample(x, y), x, y) * percent(s.ink.opacity);
                    paint(&mut src, s.ink.color, s.ink.mode, k);
                }
                for s in &strokes {
                    if s.inner > 0. {
                        s.paint(&mut src, s.inner, inside[i], x, y);
                    }
                }
                if self.bevel.style != Relief::Outer {
                    if let Some((ink, k)) = shading {
                        paint(&mut src, ink.color, ink.mode, k);
                    }
                }
                layer = mix_group(layer, src, coverage);
            }
            *dst = mix_group(*dst, layer, percent(p.opacity));
        }
    }
}
