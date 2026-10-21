//! Same-size masked template matching, coarse search plus original-pixel refinement.
use super::{MAX_MASK, MAX_PIXELS, mark, selected};
use crate::localization::tr;
use image::RgbaImage;
struct Template {
    points: Vec<(usize, usize, f64)>,
    energy: f64,
}
impl Template {
    fn new(points: Vec<(usize, usize, f64)>) -> Option<Self> {
        if points.len() < 12 {
            return None;
        }
        let mean = points.iter().map(|p| p.2).sum::<f64>() / points.len() as f64;
        let points: Vec<_> = points
            .into_iter()
            .map(|(x, y, v)| (x, y, v - mean))
            .collect();
        let energy = points.iter().map(|p| p.2 * p.2).sum::<f64>();
        if energy < points.len() as f64 * 4. {
            None
        } else {
            Some(Self { points, energy })
        }
    }
    fn score(&self, gray: &[f32], width: usize, x: usize, y: usize) -> f64 {
        let mut sum = 0.;
        let mut squares = 0.;
        let mut dot = 0.;
        for &(dx, dy, t) in &self.points {
            let c = gray[(y + dy) * width + x + dx] as f64;
            sum += c;
            squares += c * c;
            dot += t * c;
        }
        let variance = (squares - sum * sum / self.points.len() as f64).max(0.);
        if variance < 1. {
            return -1.;
        }
        dot / (variance * self.energy).sqrt()
    }
}
fn gray(image: &RgbaImage) -> Vec<f32> {
    image
        .pixels()
        .map(|p| {
            (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) * p[3] as f32 / 255.
        })
        .collect()
}
fn overlaps(a: (usize, usize), b: (usize, usize), w: usize, h: usize) -> bool {
    a.0.abs_diff(b.0) < w * 3 / 4 + 1 && a.1.abs_diff(b.1) < h * 3 / 4 + 1
}
pub(super) fn find(
    image: &RgbaImage,
    mask: &[u8],
    threshold: u32,
) -> Result<(Vec<u8>, usize), String> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    if w * h > MAX_PIXELS || mask.len() != (w * h).div_ceil(8) {
        return Err(tr("图片过大或蒙版无效。").into());
    }
    if mask
        .iter()
        .map(|byte| byte.count_ones() as usize)
        .sum::<usize>()
        > 384 * 256
    {
        return Err(tr("样本范围过大，请清除蒙版后只标记一个水印。").into());
    }
    let pixels: Vec<_> = (0..w * h).filter(|&i| selected(mask, i)).collect();
    if pixels.len() < 12 {
        return Err(tr("请先完整涂抹一个水印样本，至少标记 12 个像素。").into());
    }
    let x0 = pixels.iter().map(|i| i % w).min().unwrap();
    let y0 = pixels.iter().map(|i| i / w).min().unwrap();
    let x1 = pixels.iter().map(|i| i % w).max().unwrap();
    let y1 = pixels.iter().map(|i| i / w).max().unwrap();
    let (tw, th) = (x1 - x0 + 1, y1 - y0 + 1);
    if tw > 384 || th > 256 {
        return Err(tr("样本范围过大，请清除蒙版后只标记一个水印。").into());
    }
    let full = gray(image);
    let stride = pixels.len().div_ceil(96).max(1);
    let template = Template::new(
        pixels
            .iter()
            .step_by(stride)
            .map(|&i| (i % w - x0, i / w - y0, full[i] as f64))
            .collect(),
    )
    .ok_or_else(|| tr("样本缺少可匹配纹理，请包含水印文字或图案及少量周边。").to_owned())?;
    let scale = (768.0 / w.max(h) as f64).min(1.);
    let (sw, sh) = (
        ((w as f64 * scale).round() as usize).max(1),
        ((h as f64 * scale).round() as usize).max(1),
    );
    let reduced = image::imageops::resize(
        image,
        sw as u32,
        sh as u32,
        image::imageops::FilterType::Triangle,
    );
    let reduced_gray = gray(&reduced);
    let (sx, sy) = (
        (x0 as f64 * scale).round() as usize,
        (y0 as f64 * scale).round() as usize,
    );
    let mut seen = std::collections::HashSet::new();
    let mut points = Vec::new();
    for &(dx, dy, _) in &template.points {
        let px = ((dx as f64 * scale).round() as usize).min(sw - 1 - sx.min(sw - 1));
        let py = ((dy as f64 * scale).round() as usize).min(sh - 1 - sy.min(sh - 1));
        if seen.insert((px, py)) {
            points.push((
                px,
                py,
                reduced_gray[(sy + py).min(sh - 1) * sw + (sx + px).min(sw - 1)] as f64,
            ));
        }
    }
    let coarse = Template::new(points)
        .ok_or_else(|| tr("缩小后样本细节不足，请选更完整的水印或使用手动蒙版。").to_owned())?;
    let cw = coarse.points.iter().map(|p| p.0).max().unwrap() + 1;
    let ch = coarse.points.iter().map(|p| p.1).max().unwrap() + 1;
    let cutoff = (threshold.clamp(70, 99) as f64 / 100. - 0.15).max(0.55);
    let mut candidates = Vec::new();
    for y in 0..=sh - ch {
        for x in 0..=sw - cw {
            let score = coarse.score(&reduced_gray, sw, x, y);
            if score >= cutoff {
                candidates.push((score, x, y));
                if candidates.len() > 20_000 {
                    candidates.select_nth_unstable_by(512, |a, b| b.0.total_cmp(&a.0));
                    candidates.truncate(512);
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut kept = Vec::new();
    for &(_, x, y) in &candidates {
        if kept.iter().any(|&p| overlaps(p, (x, y), cw, ch)) {
            continue;
        }
        kept.push((x, y));
        if kept.len() >= 256 {
            break;
        }
    }
    let radius = (1. / scale).ceil() as usize + 1;
    let mut locations = vec![(x0, y0)];
    let required = threshold.clamp(70, 99) as f64 / 100.;
    for (cx, cy) in kept {
        let ox = (cx as f64 / scale).round() as usize;
        let oy = (cy as f64 / scale).round() as usize;
        let mut best = (-1., 0, 0);
        for y in oy.saturating_sub(radius)..=oy.saturating_add(radius).min(h - th) {
            for x in ox.saturating_sub(radius)..=ox.saturating_add(radius).min(w - tw) {
                let score = template.score(&full, w, x, y);
                if score > best.0 {
                    best = (score, x, y);
                }
            }
        }
        if best.0 >= required
            && !locations
                .iter()
                .any(|&p| overlaps(p, (best.1, best.2), tw, th))
        {
            locations.push((best.1, best.2));
            if locations.len() >= 128 {
                break;
            }
        }
    }
    let mut output = mask.to_vec();
    for &(x, y) in &locations {
        for &i in &pixels {
            mark(&mut output, (y + i / w - y0) * w + x + i % w - x0, true);
        }
    }
    if output
        .iter()
        .map(|byte| byte.count_ones() as usize)
        .sum::<usize>()
        > MAX_MASK
    {
        return Err(tr("匹配区域过多，请提高相似度或分批处理。").into());
    }
    Ok((output, locations.len()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_repeated_pattern_and_translates_only_the_painted_mask() {
        let mut image = RgbaImage::from_pixel(50, 20, image::Rgba([0, 0, 0, 255]));
        let mut mask = vec![0; 1000_usize.div_ceil(8)];
        for start in [2, 25] {
            for y in 0..6 {
                for x in 0..8 {
                    let v = if (x * 3 + y * 5) % 7 < 3 { 90 } else { 15 };
                    image.put_pixel(start + x, 3 + y, image::Rgba([v, v, v, 255]));
                    if start == 2 {
                        mark(&mut mask, ((3 + y) * 50 + start + x) as usize, true);
                    }
                }
            }
        }
        let (found, count) = find(&image, &mask, 99).unwrap();
        assert_eq!(count, 2);
        for y in 3..9 {
            for x in 25..33 {
                assert!(selected(&found, y * 50 + x));
            }
        }
        assert!(!selected(&found, 0));
    }
    #[test]
    fn rejects_flat_or_multiple_large_samples() {
        let image = RgbaImage::from_pixel(40, 20, image::Rgba([20, 20, 20, 255]));
        let mut mask = vec![0; 800_usize.div_ceil(8)];
        for i in 0..24 {
            mark(&mut mask, i, true);
        }
        assert!(find(&image, &mask, 90).is_err());
    }
}
