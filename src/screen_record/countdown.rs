use super::Area;
use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Countdown {
    created: bool,
    clip_cache: Option<(isize, Vec<[i32; 4]>)>,
}
impl Countdown {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        area: Area,
        deadline: Option<Instant>,
    ) -> Result<(), String> {
        let id = egui::ViewportId::from_hash_of("screen_record_countdown");
        let remaining = deadline.map(|end| end.saturating_duration_since(Instant::now()));
        let Some(remaining) = remaining.filter(|remaining| !remaining.is_zero()) else {
            if self.created {
                ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Close);
            }
            self.created = false;
            self.clip_cache = None;
            return Ok(());
        };
        let number = (remaining.as_secs_f64().ceil() as u8).clamp(1, 3);
        let title = format!("UnityYAMLViewer.Recording.{}.countdown", std::process::id());
        let side = 160.0_f32;
        let mut scale = ctx.pixels_per_point();
        let mut embedded = false;
        self.created = true;
        ctx.show_viewport_immediate(
            id,
            egui::ViewportBuilder::default()
                .with_title(&title)
                .with_inner_size([side, side])
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_taskbar(false)
                .with_active(false)
                .with_mouse_passthrough(true)
                .with_visible(false)
                .with_window_level(egui::WindowLevel::AlwaysOnTop),
            |child, class| {
                if class == egui::ViewportClass::Embedded {
                    embedded = true;
                    return;
                }
                scale = child.pixels_per_point();
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(child, |ui| {
                        let center = ui.max_rect().center();
                        ui.painter().circle_filled(
                            center,
                            side * 0.44,
                            egui::Color32::from_black_alpha(185),
                        );
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            number.to_string(),
                            egui::FontId::proportional(100.0),
                            egui::Color32::WHITE,
                        );
                    });
                child.request_repaint_after(Duration::from_millis(50));
            },
        );
        if embedded {
            return Err(crate::localization::tr("录屏悬浮工具条需要独立窗口支持。").into());
        }
        let physical = (side * scale).ceil() as i32;
        let position = (
            area.x + (area.width - physical) / 2,
            area.y + (area.height - physical) / 2,
        );
        // Capture exclusion is applied and checked before the numeral becomes visible.
        super::toolbar::native_place(
            &title,
            position,
            [physical; 2],
            true,
            &[[0, 0, physical, physical]],
            &mut self.clip_cache,
        )?;
        ctx.request_repaint_after(Duration::from_millis(50));
        Ok(())
    }
}
