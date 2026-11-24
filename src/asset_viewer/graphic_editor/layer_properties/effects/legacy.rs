//! Compatibility renderer for unedited layer styles from Tile project schemas 1–9.
use super::super::{dilate, over};
use super::*;
use image::{Rgba, RgbaImage};
impl Effects {
    // Keep the old integer colour interpolation at full fill. At lower fill,
    // only the source contributes less coverage; the colour overlay retains its own opacity.
    fn legacy_content(&self, mut pixel: Rgba<u8>, fill: u8) -> (Rgba<u8>, f32) {
        let overlay = if self.overlay.enabled {
            u32::from(self.overlay.opacity)
        } else {
            0
        };
        let source_weight = u32::from(fill) * (100 - overlay);
        let overlay_weight = 100 * overlay;
        let weight = source_weight + overlay_weight;
        if overlay_weight > 0 {
            for c in 0..3 {
                pixel[c] = ((u32::from(pixel[c]) * source_weight
                    + u32::from(self.overlay.color[c]) * overlay_weight)
                    / weight) as u8;
            }
        }
        (pixel, weight as f32 / 10_000.)
    }

    pub(super) fn composite_legacy(
        &self,
        properties: &Properties,
        out: &mut RgbaImage,
        source: &RgbaImage,
        tile: usize,
    ) {
        if !self.shadow.enabled && !self.stroke.enabled {
            for (i, (dst, src)) in out.pixels_mut().zip(source.pixels()).enumerate() {
                let (p, weight) = self.legacy_content(*src, properties.fill);
                *dst = over(
                    *dst,
                    p,
                    properties.blend,
                    properties.opacity as f32 / 100. * weight * properties.mask_value(tile, i),
                );
            }
            return;
        }
        let (w, h) = source.dimensions();
        let mut masked = source.clone();
        for (i, p) in masked.pixels_mut().enumerate() {
            p[3] = (p[3] as f32 * properties.mask_value(tile, i)).round() as u8;
        }
        let mut layer = RgbaImage::new(w, h);
        if self.shadow.enabled {
            let alpha =
                image::GrayImage::from_fn(w, h, |x, y| image::Luma([masked.get_pixel(x, y)[3]]));
            let blurred = image::imageops::blur(&alpha, self.shadow.radius as f32);
            for (x, y, p) in layer.enumerate_pixels_mut() {
                let sx = x as i32 - self.shadow.offset[0] as i32;
                let sy = y as i32 - self.shadow.offset[1] as i32;
                if sx >= 0 && sy >= 0 && sx < w as i32 && sy < h as i32 {
                    let a = blurred.get_pixel(sx as u32, sy as u32)[0] as u16
                        * self.shadow.opacity as u16
                        / 100;
                    *p = Rgba([
                        self.shadow.color[0],
                        self.shadow.color[1],
                        self.shadow.color[2],
                        a as u8,
                    ]);
                }
            }
        }
        if self.stroke.enabled {
            let alpha =
                image::GrayImage::from_fn(w, h, |x, y| image::Luma([masked.get_pixel(x, y)[3]]));
            let expanded = dilate(&alpha, self.stroke.size as usize);
            for y in 0..h {
                for x in 0..w {
                    let alpha =
                        expanded.get_pixel(x, y)[0].saturating_sub(masked.get_pixel(x, y)[3]);
                    let p = Rgba([
                        self.stroke.color[0],
                        self.stroke.color[1],
                        self.stroke.color[2],
                        alpha,
                    ]);
                    let old = *layer.get_pixel(x, y);
                    layer.put_pixel(x, y, over(old, p, Blend::Normal, 1.));
                }
            }
        }
        for (dst, src) in layer.pixels_mut().zip(masked.pixels()) {
            let (p, weight) = self.legacy_content(*src, properties.fill);
            *dst = over(*dst, p, Blend::Normal, weight);
        }
        for (dst, src) in out.pixels_mut().zip(layer.pixels()) {
            *dst = over(
                *dst,
                *src,
                properties.blend,
                properties.opacity as f32 / 100.,
            );
        }
    }
}
