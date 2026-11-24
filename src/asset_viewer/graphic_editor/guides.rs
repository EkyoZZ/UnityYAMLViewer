use super::*;

const RULER: f32 = 20.;
const GUIDE_COLOR: Color32 = Color32::CYAN;

#[derive(Clone, Copy)]
struct Guide {
    // An X coordinate makes a vertical guide. A Y coordinate makes a horizontal guide.
    axis: usize,
    pixel: f32,
}

struct Drag {
    guide: Guide,
    index: Option<usize>,
    snap_edges: Vec<f32>,
}

pub(super) struct Guides {
    pub(super) rulers: bool,
    pub(super) visible: bool,
    items: Vec<Guide>,
    drag: Option<Drag>,
}

impl Default for Guides {
    fn default() -> Self {
        Self {
            rulers: false,
            visible: true,
            items: Vec::new(),
            drag: None,
        }
    }
}

impl Editor {
    pub(super) fn guide_input(
        &mut self,
        ui: &egui::Ui,
        outer: Rect,
        area: Rect,
        origin: Pos2,
        scale: f32,
        world_offset: Vec2,
        layout: bool,
        blocked: bool,
    ) -> bool {
        let mut guides = std::mem::take(&mut self.guides);
        let dragging = guides.input(
            ui,
            outer,
            area,
            origin,
            scale,
            blocked,
            self.tool == Tool::Move,
            || {
                [
                    self.selection.guide_bounds(),
                    self.guide_layer_bounds(layout),
                ]
                .map(|bounds| bounds.map(|rect| rect.translate(-world_offset)))
            },
        );
        self.guides = guides;
        dragging
    }
}

impl Guides {
    pub(super) fn clear(&mut self) {
        self.items.clear();
        self.drag = None;
    }

    pub(super) fn area(&self, outer: Rect) -> Rect {
        if self.rulers {
            Rect::from_min_max((outer.min + Vec2::splat(RULER)).min(outer.max), outer.max)
        } else {
            outer
        }
    }

    fn ruler_rects(outer: Rect, area: Rect) -> [Rect; 2] {
        [
            Rect::from_min_max(
                egui::pos2(outer.left(), area.top()),
                egui::pos2(area.left(), area.bottom()),
            ),
            Rect::from_min_max(
                egui::pos2(area.left(), outer.top()),
                egui::pos2(area.right(), area.top()),
            ),
        ]
    }

    pub(super) fn input(
        &mut self,
        ui: &egui::Ui,
        outer: Rect,
        area: Rect,
        origin: egui::Pos2,
        scale: f32,
        blocked: bool,
        can_move: bool,
        bounds: impl FnOnce() -> [Option<Rect>; 2],
    ) -> bool {
        let (pointer, pressed, down, released) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.primary_down(),
                i.pointer.button_released(egui::PointerButton::Primary),
            )
        });
        let bars = Self::ruler_rects(outer, area);
        if let Some(pos) =
            pointer.filter(|p| !blocked && outer.contains(*p) && ui.rect_contains_pointer(outer))
        {
            let ruler = self
                .rulers
                .then(|| bars.iter().position(|r| r.contains(pos)))
                .flatten();
            let nearest = if self.visible && can_move && area.contains(pos) {
                self.items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, g)| {
                        let distance = (pos[g.axis] - origin[g.axis] - g.pixel * scale).abs();
                        (distance <= 4.).then_some((index, distance))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(index, _)| index)
            } else {
                None
            };
            let candidate = ruler
                .map(|axis| Drag {
                    guide: Guide { axis, pixel: 0. },
                    index: None,
                    snap_edges: Vec::new(),
                })
                .or_else(|| {
                    nearest.map(|index| Drag {
                        guide: self.items[index],
                        index: Some(index),
                        snap_edges: Vec::new(),
                    })
                });
            if self.drag.is_none()
                && let Some(mut candidate) = candidate
            {
                ui.output_mut(|o| o.cursor_icon = Self::cursor(candidate.guide.axis));
                if pressed {
                    let axis = candidate.guide.axis;
                    for rect in bounds().into_iter().flatten() {
                        candidate
                            .snap_edges
                            .extend([rect.min[axis], rect.max[axis]]);
                    }
                    self.drag = Some(candidate);
                }
            }
        }
        let Some(drag) = self.drag.as_mut() else {
            return false;
        };
        let axis = drag.guide.axis;
        if let Some(pos) = pointer {
            let pixel = (pos[axis] - origin[axis]) / scale;
            drag.guide.pixel = drag
                .snap_edges
                .iter()
                .copied()
                .filter(|edge| (edge - pixel).abs() * scale <= 6.)
                .min_by(|a, b| (a - pixel).abs().total_cmp(&(b - pixel).abs()))
                .unwrap_or_else(|| pixel.round());
        }
        ui.output_mut(|o| o.cursor_icon = Self::cursor(axis));
        if !down {
            let drag = self.drag.take().unwrap();
            if released && let Some(pos) = pointer {
                if self.rulers && bars.iter().any(|r| r.contains(pos)) {
                    if let Some(index) = drag.index {
                        self.items.remove(index);
                    }
                } else if area.contains(pos) && !blocked {
                    if let Some(index) = drag.index {
                        self.items[index] = drag.guide;
                    } else {
                        self.items.push(drag.guide);
                        self.visible = true;
                    }
                }
            }
        }
        true
    }

    fn cursor(axis: usize) -> egui::CursorIcon {
        if axis == 0 {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        }
    }

    pub(super) fn paint(
        &self,
        ui: &egui::Ui,
        outer: Rect,
        area: Rect,
        origin: egui::Pos2,
        scale: f32,
    ) {
        let canvas = ui.painter_at(area);
        let dpi = ui.ctx().pixels_per_point();
        let draw = |g: Guide| {
            let axis = g.axis;
            let at = origin[axis] + g.pixel * scale;
            if at < area.min[axis] || at > area.max[axis] {
                return;
            }
            let mut first = area.min;
            let mut last = area.max;
            let pixel_center = ((at * dpi).floor() + 0.5) / dpi;
            first[axis] = pixel_center;
            last[axis] = pixel_center;
            canvas.line_segment([first, last], egui::Stroke::new(1.0 / dpi, GUIDE_COLOR));
        };
        if self.visible {
            for (i, &guide) in self.items.iter().enumerate() {
                if !self.drag.as_ref().is_some_and(|d| d.index == Some(i)) {
                    draw(guide);
                }
            }
        }
        if let Some(drag) = &self.drag {
            draw(drag.guide);
            let g = drag.guide;
            if let Some(pointer) = ui
                .input(|i| i.pointer.interact_pos())
                .filter(|p| area.contains(*p))
            {
                let pos = pointer + egui::vec2(10., 12.);
                let axis = if g.axis == 0 { "X" } else { "Y" };
                let label = canvas.layout_no_wrap(
                    format!("{axis}  {} px", g.pixel),
                    egui::FontId::monospace(11.),
                    Color32::WHITE,
                );
                let bg = Rect::from_min_size(pos, label.size()).expand(3.);
                canvas.rect_filled(bg, 3., Color32::from_gray(35));
                canvas.galley(pos, label, Color32::WHITE);
            }
        }
        if !self.rulers {
            return;
        }
        let panel = ui.painter_at(outer);
        let background = ui.visuals().panel_fill;
        let foreground = ui.visuals().text_color();
        panel.rect_filled(Rect::from_min_max(outer.min, area.min), 0., background);
        let pitch = (40. / scale).max(1.);
        let decade = 10_f32.powf(pitch.log10().floor());
        let major = [1., 2., 5., 10.]
            .into_iter()
            .map(|m| m * decade)
            .find(|&step| step >= pitch)
            .unwrap_or(10. * decade);
        let divisions = if major <= 2. { major as i32 } else { 5 };
        let minor = major / divisions as f32;
        for (axis, bar) in Self::ruler_rects(outer, area).into_iter().rev().enumerate() {
            let ruler = ui.painter_at(bar);
            ruler.rect_filled(bar, 0., background);
            let edge = 1 - axis;
            let start = ((bar.min[axis] - origin[axis]) / (minor * scale)).ceil() as i32;
            let end = ((bar.max[axis] - origin[axis]) / (minor * scale)).floor() as i32;
            for tick in start..=end.min(start.saturating_add(4096)) {
                let is_major = tick.rem_euclid(divisions) == 0;
                let mut to = bar.max;
                to[axis] = origin[axis] + tick as f32 * minor * scale;
                let mut from = to;
                from[edge] -= if is_major { 7. } else { 4. };
                ruler.line_segment(
                    [from, to],
                    egui::Stroke::new(1.0_f32, foreground.gamma_multiply(0.72)),
                );
                if is_major {
                    let text = format!("{:.0}", tick as f32 * minor);
                    if axis == 0 {
                        ruler.text(
                            from + egui::vec2(2., -1.),
                            egui::Align2::LEFT_BOTTOM,
                            text,
                            egui::FontId::monospace(9.),
                            foreground,
                        );
                    } else {
                        let galley =
                            ruler.layout_no_wrap(text, egui::FontId::monospace(9.), foreground);
                        let pos = egui::pos2(bar.left() + 2., to.y - 2.);
                        ruler.add(
                            egui::epaint::TextShape::new(pos, galley, foreground)
                                .with_angle(-std::f32::consts::FRAC_PI_2),
                        );
                    }
                }
            }
        }
    }
}
