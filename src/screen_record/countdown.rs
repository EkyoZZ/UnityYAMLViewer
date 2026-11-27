use super::Area;
use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

const DURATION: Duration = Duration::from_secs(3);

type Curve = [[f32; 2]; 4];
const DIGIT_STROKE: f32 = 12.0;
const ONE: &[Curve] = &[
    [[18.0, 18.0], [24.0, 14.0], [34.0, 3.0], [40.0, 3.0]],
    [[40.0, 3.0], [40.0, 30.0], [40.0, 57.0], [40.0, 84.0]],
    [[18.0, 84.0], [32.0, 84.0], [46.0, 84.0], [60.0, 84.0]],
];
const TWO: &[Curve] = &[
    [[12.0, 20.0], [12.0, -7.0], [68.0, -7.0], [68.0, 22.0]],
    [[68.0, 22.0], [68.0, 42.0], [12.0, 58.0], [12.0, 84.0]],
    [[12.0, 84.0], [30.0, 84.0], [50.0, 84.0], [68.0, 84.0]],
];
const THREE: &[Curve] = &[
    [[12.0, 10.0], [26.0, -2.0], [66.0, -2.0], [66.0, 18.0]],
    [[66.0, 18.0], [66.0, 34.0], [52.0, 42.0], [34.0, 42.0]],
    [[34.0, 42.0], [52.0, 42.0], [68.0, 50.0], [68.0, 66.0]],
    [[68.0, 66.0], [68.0, 88.0], [28.0, 92.0], [12.0, 78.0]],
];

fn digit_shapes(number: u8, center: egui::Pos2, height: f32) -> Vec<egui::Shape> {
    let curves = match number {
        1 => ONE,
        2 => TWO,
        _ => THREE,
    };
    let paths: Vec<Vec<egui::Pos2>> = curves
        .iter()
        .map(|curve| {
            (0..=32)
                .map(|step| {
                    let t = step as f32 / 32.0;
                    let u = 1.0 - t;
                    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    let mut point = egui::Pos2::ZERO;
                    for (control, weight) in curve.iter().zip(weights) {
                        point.x += control[0] * weight;
                        point.y += control[1] * weight;
                    }
                    point
                })
                .collect()
        })
        .collect();
    // Center the visible ink, including rounded caps, rather than a font layout box.
    let mut bounds = egui::Rect::NOTHING;
    for point in paths.iter().flatten() {
        bounds.extend_with(*point);
    }
    let bounds = bounds.expand(DIGIT_STROKE * 0.5);
    let scale = height / bounds.height();
    let stroke = egui::Stroke::new(DIGIT_STROKE * scale, egui::Color32::WHITE);
    let mut shapes = Vec::with_capacity(paths.len() * 3);
    for path in paths {
        let points: Vec<_> = path
            .into_iter()
            .map(|point| center + (point - bounds.center()) * scale)
            .collect();
        for point in [points[0], *points.last().unwrap()] {
            shapes.push(egui::Shape::circle_filled(
                point,
                stroke.width * 0.5,
                stroke.color,
            ));
        }
        shapes.push(egui::Shape::line(points, stroke));
    }
    shapes
}

// The worker must not consume the countdown while Windows is still creating the window.
pub(super) struct Request {
    ready: Option<mpsc::Sender<Instant>>,
    deadline: Option<Instant>,
}
impl Request {
    pub fn new(ready: mpsc::Sender<Instant>) -> Self {
        Self {
            ready: Some(ready),
            deadline: None,
        }
    }
    fn remaining(&self, now: Instant) -> Duration {
        self.deadline
            .map_or(DURATION, |end| end.saturating_duration_since(now))
    }
    pub fn paint(&mut self, ui: &egui::Ui, area: egui::Rect) {
        let now = Instant::now();
        let remaining = self.remaining(now);
        if remaining.is_zero() {
            return;
        }
        let number = (remaining.as_secs_f64().ceil() as u8).clamp(1, 3);
        let center = area.center();
        let side = 140.0_f32.min(area.width()).min(area.height()).max(1.0);
        let painter = ui.painter_at(area);
        painter.circle_filled(center, side * 0.44, egui::Color32::from_black_alpha(185));
        painter.extend(digit_shapes(number, center, side * 0.6));
        self.presented(now);
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }

    fn presented(&mut self, now: Instant) {
        if let Some(ready) = self.ready.take() {
            let deadline = now + DURATION;
            self.deadline = Some(deadline);
            let _ = ready.send(deadline);
        }
    }
}

pub(super) fn wait_until_visible(
    ready: &mpsc::Receiver<Instant>,
    stop: &mpsc::Receiver<()>,
    ctx: &egui::Context,
) -> Result<Instant, String> {
    let timeout = Instant::now() + Duration::from_secs(5);
    loop {
        if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            return Err(crate::localization::tr("录屏已取消").into());
        }
        match ready.recv_timeout(Duration::from_millis(50)) {
            Ok(deadline) => return Ok(deadline),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() >= timeout => break,
            Err(mpsc::RecvTimeoutError::Timeout) => ctx.request_repaint(),
        }
    }
    Err(crate::localization::tr("无法显示录屏倒计时，请重试。").into())
}

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
        request: Option<&mut Request>,
    ) -> Result<(), String> {
        let id = egui::ViewportId::from_hash_of("screen_record_countdown");
        let request = request.filter(|request| !request.remaining(Instant::now()).is_zero());
        let Some(request) = request else {
            if self.created {
                ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Close);
            }
            self.created = false;
            self.clip_cache = None;
            return Ok(());
        };
        let remaining = request.remaining(Instant::now());
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
                        ui.painter()
                            .extend(digit_shapes(number, center, side * 0.6));
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
        let handle = super::toolbar::native_place(
            &title,
            position,
            [physical; 2],
            true,
            &[[0, 0, physical, physical]],
            &mut self.clip_cache,
        )?;
        if handle.is_some() {
            // show_viewport_immediate has rendered a frame, and native_place has
            // initialized transparency, capture exclusion, size and visibility.
            request.presented(Instant::now());
        }
        ctx.request_repaint_after(Duration::from_millis(50));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_starts_after_presentation_and_does_not_restart_on_repaint() {
        let (ready, events) = mpsc::channel();
        let mut request = Request::new(ready);
        let now = Instant::now();
        let shown = now + Duration::from_secs(10);
        assert_eq!(request.remaining(shown), DURATION);
        assert!(matches!(events.try_recv(), Err(mpsc::TryRecvError::Empty)));
        request.presented(shown);
        assert_eq!(events.recv().unwrap(), shown + DURATION);
        request.presented(shown + Duration::from_secs(1));
        assert_eq!(
            request.remaining(shown + Duration::from_secs(1)),
            Duration::from_secs(2)
        );
        assert_eq!(
            request.remaining(shown + Duration::from_secs(2)),
            Duration::from_secs(1)
        );
        assert!(request.remaining(shown + DURATION).is_zero());
        assert!(events.try_recv().is_err());
    }

    #[test]
    fn waiting_for_window_is_cancellable_and_rejects_closed_ui() {
        let (ready, events) = mpsc::channel();
        let (stop, canceled) = mpsc::channel();
        stop.send(()).unwrap();
        assert_eq!(
            wait_until_visible(&events, &canceled, &egui::Context::default()).unwrap_err(),
            crate::localization::tr("录屏已取消")
        );
        drop(ready);
        assert_eq!(
            wait_until_visible(&events, &canceled, &egui::Context::default()).unwrap_err(),
            crate::localization::tr("无法显示录屏倒计时，请重试。")
        );
    }
}
