use super::{ParsedClip, animation_sample_rate, collect_animation_bindings, single_frame_time};
use crate::{Document, hierarchy_controls::Flags, localization::tr, model_preview::{Camera, ComparisonRenderer, ComparisonScene}};
use eframe::egui::{self, Color32};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex, mpsc}, time::{Duration, Instant}};

pub(super) fn window_id() -> egui::Id { egui::Id::new("animation_optimization_comparison") }

#[derive(Clone, Copy)]
struct Timeline {
    rate: f64,
    duration: f32,
    last: u32,
    fixed: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_keeps_a_partial_last_frame_and_round_trips_sample_times() {
        let clip = ParsedClip::new("AnimationClip:\n  m_SampleRate: 30\n  m_StopTime: 0.08\n".into());
        let timeline = Timeline::new(&clip);
        assert_eq!(timeline.last, 3);
        assert_eq!(timeline.time(3), 0.08);
        for frame in 0..=3 { assert_eq!(timeline.frame(timeline.time(frame)), frame); }
        assert_eq!(timeline.frame(0.079), 3);
        let exact = ParsedClip::new("AnimationClip:\n  m_SampleRate: 30\n  m_StopTime: 2.6666667\n".into());
        assert_eq!(Timeline::new(&exact).last, 80);
    }
}

impl Timeline {
    fn new(clip: &ParsedClip) -> Self {
        let rate = f64::from(animation_sample_rate(&clip.text).filter(|r| r.is_finite() && *r > 0.0).unwrap_or(60.0));
        let mut bindings = Vec::new();
        collect_animation_bindings(&clip.tree, &mut bindings);
        let fixed = single_frame_time(&clip.text, &bindings, clip.duration);
        Self::from_duration(clip.duration, rate, fixed)
    }

    fn from_duration(duration: f32, rate: f64, fixed: Option<f32>) -> Self {
        // Allow for rounding at an exact sample boundary.
        let samples = f64::from(duration) * rate;
        let last = if fixed.is_some() { 0 } else { (samples - samples * f64::from(f32::EPSILON)).ceil().max(1.0) as u32 };
        Self { rate, duration, last, fixed }
    }

    fn time(self, frame: u32) -> f32 {
        self.fixed.unwrap_or_else(|| if frame >= self.last { self.duration } else { (f64::from(frame) / self.rate) as f32 })
    }

    fn frame(self, time: f32) -> u32 {
        if time >= self.duration { return self.last; }
        let regular = ((f64::from(time) * self.rate).round() as u32).min(self.last);
        if self.duration - time < (self.time(regular) - time).abs() { self.last } else { regular }
    }

    fn next(self, frame: u32, looping: bool) -> Option<u32> {
        if frame < self.last { Some(frame + 1) } else { (looping && self.last > 0).then_some(0) }
    }
}

type Pair = Result<[ComparisonScene; 2], String>;

pub(crate) struct Player {
    timeline: Timeline,
    request: mpsc::Sender<u32>,
    results: mpsc::Receiver<(u32, Pair)>,
    pending: bool,
    ready: Option<(u32, Pair)>,
    displayed: Option<(u32, Pair)>,
    presented_at: Instant,
    was_playing: bool,
    camera: Camera,
    initial_camera: Camera,
    gpu_errors: [Arc<Mutex<Option<String>>>; 2],
}

impl Player {
    pub(super) fn new(ctx: &egui::Context, documents: Vec<Document>, root: String, source: Arc<ParsedClip>,
        draft: Arc<ParsedClip>, flags: HashMap<String, Flags>, catalog: Arc<HashMap<String, PathBuf>>, front: bool) -> Self {
        let timeline = Timeline::new(&source);
        let mut renderer = ComparisonRenderer::default();
        Self::spawn(ctx, timeline, front, move |time| {
            let mut prepare = |clip: &ParsedClip| {
                let pose = clip.sample(&documents, &root, time).ok_or_else(|| tr("没有可对照的动画节点").to_owned())?;
                renderer.prepare(&pose, &flags, &catalog)
            };
            Ok([prepare(&source)?, prepare(&draft)?])
        })
    }

    pub(crate) fn for_motion(ctx: &egui::Context, duration: f32, rate: f64,
        render: impl FnMut(f32) -> Pair + Send + 'static) -> Self {
        Self::spawn(ctx, Timeline::from_duration(duration, rate, None), false, render)
    }

    fn spawn(ctx: &egui::Context, timeline: Timeline, front: bool,
        mut render: impl FnMut(f32) -> Pair + Send + 'static) -> Self {
        let (request, frames) = mpsc::channel();
        let (send, results) = mpsc::channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(frame) = frames.recv() {
                if send.send((frame, render(timeline.time(frame)))).is_err() { break; }
                repaint.request_repaint_of(egui::ViewportId::ROOT);
            }
        });
        let camera = if front { Camera::front() } else { Camera::default() };
        Self { timeline, request, results, pending: false, ready: None, displayed: None,
            presented_at: Instant::now(), was_playing: false, camera, initial_camera: camera, gpu_errors: Default::default() }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, time: &mut f32, playing: &mut bool, looping: &mut bool,
        background: &str, rgb: [u8; 3]) {
        let mut frame = self.timeline.frame(*time);
        let last = self.timeline.last;
        if *playing && !self.was_playing { self.presented_at = Instant::now(); }
        self.advance(ui.ctx(), &mut frame, playing, *looping);
        let previous = (frame, *playing, *looping);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            self.show_warnings(ui, frame);
            ui.vertical(|ui| ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(frame > 0, egui::Button::new(tr("上一帧"))).clicked() { frame -= 1; *playing = false; }
                if ui.add_enabled(last > 0, egui::Button::new(tr(if *playing { "暂停" } else { "播放" }))).clicked() {
                    *playing = !*playing;
                    if *playing && frame == last { frame = 0; }
                }
                if ui.add_enabled(frame < last, egui::Button::new(tr("下一帧"))).clicked() { frame += 1; *playing = false; }
                ui.checkbox(looping, tr("循环"));
                if ui.add_enabled(last > 0, egui::Slider::new(&mut frame, 0..=last).text(tr("帧"))).changed() { *playing = false; }
                ui.label(format!("{:.3} / {:.3} s", self.timeline.time(frame), self.timeline.duration));
                if ui.button(tr("重置视角")).clicked() { self.camera = self.initial_camera; }
            }));
        });
        if previous != (frame, *playing, *looping) {
            if *playing && !previous.1 { self.presented_at = Instant::now(); }
            self.advance(ui.ctx(), &mut frame, playing, *looping);
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        self.was_playing = *playing;
        *time = self.timeline.time(frame);

        let pair = self.displayed.as_ref().filter(|(shown, _)| *shown == frame).map(|(_, pair)| pair);
        let (area, _) = ui.allocate_exact_size(ui.available_size().max(egui::Vec2::splat(1.0)), egui::Sense::hover());
        let gap = 2.0_f32.min(area.width() * 0.1);
        let size = egui::vec2((area.width() - gap) * 0.5, area.height());
        let panes = [area.min, area.min + egui::vec2(size.x + gap, 0.0)].map(|pos| egui::Rect::from_min_size(pos, size));
        // Apply input before painting either side so the camera also changes atomically.
        for (side, rect) in panes.iter().enumerate() {
            let response = ui.interact(*rect, ui.id().with(("comparison_camera", side)), egui::Sense::drag());
            self.camera.interact(ui, &response, false);
        }
        for (side, rect) in panes.into_iter().enumerate() {
            let painter = ui.painter_at(rect);
            painter.add(crate::static_preview::background_shape(rect, background, rgb));
            let message = match pair {
                Some(Ok(scenes)) => {
                    scenes[side].paint(&painter, rect, &self.camera, self.gpu_errors[side].clone());
                    self.gpu_errors[side].lock().unwrap().clone()
                }
                Some(Err(error)) => Some(error.clone()),
                None => Some(tr("正在准备预览…").to_owned()),
            };
            if let Some(text) = message {
                let galley = painter.layout(text, egui::FontId::proportional(14.0), Color32::WHITE, (rect.width() - 24.0).max(1.0));
                let bounds = egui::Rect::from_center_size(rect.center(), galley.size());
                painter.rect_filled(bounds.expand(5.0), 4.0, Color32::from_black_alpha(195));
                painter.galley(bounds.min, galley, Color32::WHITE);
            }
            crate::static_preview::paint_pose_badge(&painter, rect, if side == 0 { "优化前" } else { "优化后" });
        }
    }

    fn show_warnings(&self, ui: &mut egui::Ui, frame: u32) {
        let mut warnings = Vec::new();
        if let Some((shown, Ok(scenes))) = &self.displayed && *shown == frame {
            for warning in scenes.iter().flat_map(|scene| &scene.warnings) {
                if !warnings.contains(&warning) { warnings.push(warning); }
            }
        }
        let label = tr("预览问题详情");
        let text = if warnings.is_empty() { egui::RichText::new(label) }
            else { egui::RichText::new(label).color(ui.visuals().warn_fg_color) };
        ui.menu_button(text, |ui| {
            ui.set_max_width(360.0);
            if warnings.is_empty() {
                ui.label("—");
            }
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for warning in warnings { ui.add(egui::Label::new(warning.as_str()).wrap()); }
            });
        });
    }

    fn advance(&mut self, ctx: &egui::Context, frame: &mut u32, playing: &mut bool, looping: bool) {
        if self.pending {
            match self.results.try_recv() {
                Ok(result) => { self.ready = Some(result); self.pending = false; }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.displayed = Some((*frame, Err(tr("对比预览准备失败").to_owned())));
                    self.pending = false;
                    *playing = false;
                }
            }
        }
        let seeking = self.displayed.as_ref().is_none_or(|(shown, _)| shown != frame);
        let mut target = if seeking { *frame } else if !*playing { return; }
            else if let Some(next) = self.timeline.next(*frame, looping) { next }
            else { *playing = false; return; };

        if self.ready.as_ref().is_some_and(|(ready, _)| *ready == target) {
            let interval = if target > *frame { self.timeline.time(target) - self.timeline.time(*frame) }
                else { (1.0 / self.timeline.rate) as f32 };
            let remaining = interval - self.presented_at.elapsed().as_secs_f32();
            if !seeking && remaining > 0.0 {
                ctx.request_repaint_after_for(Duration::from_secs_f32(remaining.min(1.0)), egui::ViewportId::ROOT);
                return;
            }
            self.displayed = self.ready.take();
            *frame = target;
            self.presented_at = Instant::now();
            if self.displayed.as_ref().is_some_and(|(_, pair)| pair.is_err()) { *playing = false; }
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            if !*playing { return; }
            let Some(next) = self.timeline.next(*frame, looping) else { *playing = false; return; };
            target = next;
        }
        if !self.pending {
            self.ready = None;
            if self.request.send(target).is_ok() { self.pending = true; }
            else {
                self.displayed = Some((*frame, Err(tr("对比预览准备失败").to_owned())));
                *playing = false;
            }
        }
        if self.pending { ctx.request_repaint_after_for(Duration::from_millis(100), egui::ViewportId::ROOT); }
    }
}
