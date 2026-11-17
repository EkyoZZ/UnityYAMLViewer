//! Smooth asset previews interpolate gamma-premultiplied RGBA directly.
//! egui's sRGB texture decoding/encoding otherwise brightens alpha transitions.
use super::video::Frame;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub(crate) fn callback(
    image: egui::Rect,
    visible: egui::Rect,
    frame: Arc<Frame>,
    error: Arc<Mutex<Option<String>>>,
) -> egui::PaintCallback {
    let crop = [
        (visible.left() - image.left()) / image.width(),
        (visible.top() - image.top()) / image.height(),
        (visible.right() - image.left()) / image.width(),
        (visible.bottom() - image.top()) / image.height(),
    ];
    egui::PaintCallback {
        rect: visible,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, _painter| {
            let viewport = info.viewport_in_pixels();
            if let Some(gpu) = super::GPU.lock().unwrap().as_mut() {
                *error.lock().unwrap() = unsafe {
                    gpu.draw_image_preview(
                        &frame,
                        [
                            viewport.left_px,
                            viewport.from_bottom_px,
                            viewport.width_px,
                            viewport.height_px,
                        ],
                        crop,
                    )
                }
                .err();
            }
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_hidden_white_never_creates_a_bright_fringe() {
        let opaque = crate::asset_viewer::preview_color([60, 160, 20, 255]);
        let transparent = crate::asset_viewer::preview_color([255, 255, 255, 0]);
        assert_eq!(transparent.to_array(), [0; 4]);
        for weight in [0.0_f32, 0.1, 0.25, 0.5, 0.75, 1.0] {
            let sampled = std::array::from_fn::<_, 4, _>(|i| {
                opaque.to_array()[i] as f32 * weight
                    + transparent.to_array()[i] as f32 * (1.0 - weight)
            });
            for c in 0..3 {
                assert!(sampled[c] <= sampled[3]);
                assert!((sampled[c] - opaque.to_array()[c] as f32 * weight).abs() < 0.001);
            }
        }
        // Contrast the old sRGB filtering path at a 50% opaque-white edge.
        let old_gamma = 1.055_f32 * 0.5_f32.powf(1.0 / 2.4) - 0.055;
        assert!(old_gamma > 0.7);
        assert!(old_gamma > 0.5);
    }
    #[test]
    fn partial_alpha_is_not_multiplied_twice() {
        let color = crate::asset_viewer::preview_color([200, 100, 50, 128]);
        let frame = Frame::from_image(egui::ColorImage {
            size: [1, 1],
            pixels: vec![color],
        });
        assert_eq!(frame.bytes, vec![100, 50, 25, 128]);
        assert!(frame.format == super::super::video::Format::RgbaPremultiplied);
    }
}
