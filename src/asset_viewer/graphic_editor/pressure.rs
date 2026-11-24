//! Pressure delivered by the existing winit -> egui Windows Ink path.
use super::*;
#[derive(Default)]
pub(super) struct PenInput {
    pub current: Option<f32>,
    pub last_received: Option<f32>,
    position:Option<Pos2>,
    pub samples: Vec<(Pos2, f32)>,
    contact: Option<(egui::TouchDeviceId, egui::TouchId)>,
}
impl PenInput {
    pub fn read(&mut self, ui: &egui::Ui) {
        self.samples.clear();
        self.position=ui.input(|i|i.pointer.interact_pos());
        ui.input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Touch { device_id, id, phase, pos, force } => {
                        let key = (*device_id, *id);
                        match phase {
                            egui::TouchPhase::Start | egui::TouchPhase::Move => {
                                if self.contact.is_some_and(|active| active != key) { continue; }
                                if let Some(value) = force.filter(|v| v.is_finite()) {
                                    let value = value.clamp(0., 1.);
                                    self.contact = Some(key);
                                    self.current = Some(value);
                                    self.last_received = Some(value);
                                    self.samples.push((*pos, value));
                                }
                            }
                            egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                                if self.contact == Some(key) {
                                    self.current = None;
                                    self.contact = None;
                                }
                            }
                        }
                    }
                    egui::Event::WindowFocused(false) | egui::Event::PointerGone => {
                        self.current = None;
                        self.contact = None;
                    }
                    _ => {}
                }
            }
        });
    }
}
fn pressure_factor(pressure: Option<f32>, minimum: u8) -> f32 {
    let Some(value) = pressure.filter(|v| v.is_finite()) else { return 1.; };
    let min = f32::from(minimum.min(100)) / 100.;
    min + (1. - min) * value.clamp(0., 1.)
}
impl Editor {
    pub(super) fn plane_input_stroke(&mut self, from: (i32,i32), to: (i32,i32), layout: bool, origin: Pos2, scale: f32) {
        if self.soft_brush && !self.pen.samples.is_empty() {
            let samples=self.pen.samples.clone();
            let mut previous=self.pen.samples.first().map(|s|((s.0-origin)/scale).to_pos2()-egui::vec2(0.5,0.5)).unwrap();
            if self.stroke_pressure.is_some(){previous=Pos2::new(from.0 as f32,from.1 as f32);}
            for (pos,pressure) in samples {
                let local=((pos-origin)/scale).to_pos2()-egui::vec2(0.5,0.5);self.input_pressure=Some(pressure);
                self.brush_segment(previous,local,layout);previous=local;
            }
        } else {
            self.input_pressure=self.pen.current;
            if self.soft_brush {
                let local=self.pen.position.map(|p|((p-origin)/scale).to_pos2()-egui::vec2(0.5,0.5)).unwrap_or(Pos2::new(to.0 as f32,to.1 as f32));
                self.brush_segment(local,local,layout);
            }else{self.plane_stroke(from,to,layout);}
        }
    }
    pub(super) fn pressure_size(&self, pressure: Option<f32>) -> i32 {
        let factor = if self.soft_brush && self.pressure_size { pressure_factor(pressure, self.pressure_min_size) } else { 1. };
        (self.brush_size as f32 * factor).round().max(1.) as i32
    }
    pub(super) fn pressure_alpha(&self, pressure: Option<f32>) -> f32 {
        if self.soft_brush && self.pressure_opacity { pressure_factor(pressure, 0) } else { 1. }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_events_update_contact_and_clear_on_lift() {
        let ctx = egui::Context::default();
        let mut pen = PenInput::default();
        for (phase, force, expected) in [
            (egui::TouchPhase::Start, Some(0.25), Some(0.25)),
            (egui::TouchPhase::Move, Some(0.75), Some(0.75)),
            (egui::TouchPhase::End, None, None),
        ] {
            let _ = ctx.run(egui::RawInput { events: vec![egui::Event::Touch {
                device_id: egui::TouchDeviceId(1), id: egui::TouchId(1), phase,
                pos: Pos2::new(20., 20.), force,
            }], ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| pen.read(ui));
            });
            assert_eq!(pen.current, expected);
        }
        assert!(pen.contact.is_none());
        assert_eq!(pen.last_received,Some(0.75));
    }
    #[test]
    fn missing_pressure_keeps_mouse_at_full_strength() {
        assert_eq!(pressure_factor(None, 0), 1.);
        assert_eq!(pressure_factor(Some(f32::NAN), 0), 1.);
        assert_eq!(pressure_factor(Some(0.), 20), 0.2);
        assert_eq!(pressure_factor(Some(1.), 20), 1.);
    }
}
