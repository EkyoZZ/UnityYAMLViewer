//! Canvas tools and transient multi-selection; no shader-file edits.
use super::{Color32, HashMap, Rect, Sense, Stroke, Vec2, egui};
use std::collections::HashSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Target {
    Position(usize),
    Group(usize),
}
#[derive(Default)]
pub(super) struct Tools {
    pub selected: HashSet<Target>,
    nodes: HashMap<Target, Rect>,
    marquee: Option<(egui::Pos2, HashSet<Target>, bool)>,
    pub delta: Vec2,
    pub moving_nodes: bool,
    right_pan: bool,
    active_drag: Option<Target>,
    scope: Option<egui::Id>,
}
impl Tools {
    pub fn rect(&self, target: Target) -> Option<Rect> {
        self.nodes.get(&target).copied()
    }
    pub fn clear(&mut self) {
        self.selected.clear();
        self.nodes.clear();
        self.marquee = None;
        self.delta = Vec2::ZERO;
        self.moving_nodes = false;
        self.right_pan = false;
        self.active_drag = None;
    }
    pub fn pan_delta(&mut self,ui:&egui::Ui,canvas:Rect,_response:&egui::Response)->Vec2 {
        let visible=canvas.intersect(ui.clip_rect());
        if ui.memory(|m|m.any_popup_open()){self.right_pan=false;return Vec2::ZERO;}
        ui.input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary) {
                self.right_pan=i.pointer.press_origin().is_some_and(|p|visible.contains(p));
            }
            if !i.pointer.button_down(egui::PointerButton::Secondary){self.right_pan=false;}
            if self.right_pan {i.pointer.delta()}else{Vec2::ZERO}
        })
    }
    pub fn begin(&mut self,ui:&egui::Ui,_canvas:Rect){
        self.scope=Some(ui.id().with("graph_pointer_owner"));
        self.nodes.clear();self.delta=Vec2::ZERO;
        let (pressed,down,focused)=ui.input(|i|(i.pointer.button_pressed(egui::PointerButton::Primary),i.pointer.primary_down(),i.focused));
        if pressed {self.active_drag=None;self.marquee=None;self.moving_nodes=false;}
        if !down || !focused {self.active_drag=None;}
        if !focused {self.marquee=None;self.right_pan=false;self.moving_nodes=false;}
    }
    pub fn node(
        &mut self,
        ui: &egui::Ui,
        target: Target,
        rect: Rect,
        header: Rect,
        canvas: Rect,
        _zoom: f32,
    ) -> egui::Response {
        self.nodes.insert(target, rect);
        // One interaction owns selection and dragging. Later parameter widgets
        // still take precedence over the node's background.
        let hit_rect=if matches!(target,Target::Group(_)){header}else{rect};
        let response = ui.interact(
            hit_rect.intersect(canvas),
            self.scope.unwrap_or(ui.id()).with(("graph_node_move", target)),
            Sense::click_and_drag(),
        );
        {
            let shift = ui.input(|i| i.modifiers.shift);
            if response.drag_started_by(egui::PointerButton::Primary) && self.active_drag.is_none() {
                self.active_drag=Some(target);
                self.marquee=None;
                if !self.selected.contains(&target) {
                    if !shift {self.selected.clear();}
                    self.selected.insert(target);
                }
                ui.ctx().request_repaint();
            }
            if response.clicked() {
                ui.ctx().request_repaint();
                if shift {
                    if !self.selected.remove(&target) {
                        self.selected.insert(target);
                    }
                } else {
                    self.selected.clear();
                    self.selected.insert(target);
                }
            }
            if self.selected.contains(&target) {
                ui.painter().with_clip_rect(canvas).rect_stroke(
                    rect.expand(2.),
                    4.,
                    Stroke::new(1.6_f32, Color32::from_rgb(95, 175, 245)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        response
    }
    pub fn finish(&mut self, ui: &mut egui::Ui, canvas: Rect, response: &egui::Response) {
        {
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(start) = ui.input(|i| i.pointer.press_origin()).filter(|p| {
                    canvas.contains(*p)
                        && !self.nodes.iter().any(|(target,rect)| matches!(target,Target::Position(_)) && rect.contains(*p))
                }) {
                    self.marquee = Some((
                        start,
                        self.selected.clone(),
                        ui.input(|i| i.modifiers.shift),
                    ));
                }
            }
            if let Some((start, baseline, shift)) = &self.marquee {
                if let Some(end) = ui.input(|i| i.pointer.latest_pos()) {
                    let rect = Rect::from_two_pos(*start, end).intersect(canvas);
                    ui.painter().rect_filled(
                        rect,
                        0.,
                        Color32::from_rgba_unmultiplied(80, 150, 235, 30),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        0.,
                        Stroke::new(1_f32, Color32::from_rgb(95, 175, 245)),
                        egui::StrokeKind::Inside,
                    );
                    if ui.input(|i| i.pointer.primary_released()) {
                        let hits = self
                            .nodes
                            .iter()
                            .filter_map(|(key, r)| r.intersects(rect).then_some(*key))
                            .collect::<HashSet<_>>();
                        self.selected = if *shift {
                            baseline.symmetric_difference(&hits).copied().collect()
                        } else {
                            hits
                        };
                        self.marquee = None;
                        ui.ctx().request_repaint();
                    }
                }
            } else if response.clicked()
                && !ui.input(|i| i.modifiers.shift)
                && !ui.input(|i| {
                    i.pointer
                        .latest_pos()
                        .is_some_and(|p| self.nodes.values().any(|rect| rect.contains(p)))
                })
            {
                self.selected.clear();
                ui.ctx().request_repaint();
            }
        }
        if self.active_drag.is_some() && ui.input(|i|i.pointer.primary_down()) {
            self.moving_nodes=true;
            self.delta=ui.input(|i|i.pointer.delta());
            ui.ctx().request_repaint();
        }
        if !ui.input(|i|i.pointer.primary_down()) {
            self.active_drag=None;
            // A lost release / focus change must not leave a permanent marquee.
            self.marquee=None;
        }
        for target in &self.selected {
            if matches!(target,Target::Group(_)) {if let Some(rect)=self.nodes.get(target){
                ui.painter().with_clip_rect(canvas).rect_stroke(rect.expand(2.),4.,Stroke::new(1.6,Color32::from_rgb(95,175,245)),egui::StrokeKind::Outside);
            }}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_consecutive_drags_release_capture_and_apply_one_delta_per_frame(){
        let ctx=egui::Context::default();let mut tools=Tools::default();
        let point=egui::pos2(60.,60.);let moved=egui::pos2(90.,60.);
        let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};
        let frames=vec![vec![],vec![egui::Event::PointerMoved(point),button(point,true)],vec![egui::Event::PointerMoved(moved)],vec![button(moved,false)],vec![egui::Event::PointerMoved(point),button(point,true)],vec![egui::Event::PointerMoved(moved)],vec![button(moved,false)]];
        let mut drags=0;
        for (frame,events) in frames.into_iter().enumerate(){
            let input=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(500.,400.))),events,..Default::default()};
            let _=ctx.run(input,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
                let (canvas,response)=ui.allocate_exact_size(egui::vec2(450.,350.),Sense::click_and_drag());
                tools.begin(ui,canvas);
                let rect=Rect::from_min_size(egui::pos2(30.,30.),egui::vec2(150.,100.));
                tools.node(ui,Target::Position(0),rect,rect,canvas,1.);
                tools.finish(ui,canvas,&response);
                if frame==2||frame==5 {assert_eq!(tools.delta,egui::vec2(30.,0.));drags+=1;}
                if frame==3||frame==6 {assert!(tools.active_drag.is_none());assert!(tools.marquee.is_none());assert_eq!(tools.delta,Vec2::ZERO);}
            });});
        }
        assert_eq!(drags,2);
    }
    #[test]
    fn shift_marquee_toggles_hits_without_losing_other_nodes() {
        let a = Target::Position(1);
        let b = Target::Position(2);
        let c = Target::Position(10000);
        let baseline = HashSet::from([a, b]);
        let hits = HashSet::from([b, c]);
        assert_eq!(
            baseline
                .symmetric_difference(&hits)
                .copied()
                .collect::<HashSet<_>>(),
            HashSet::from([a, c])
        );
    }
}
