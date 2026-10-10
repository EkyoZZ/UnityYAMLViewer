use std::collections::BTreeSet;
use eframe::egui::{self, Color32, Stroke};
use crate::{AnimationBinding, SelectedKeyframe, animation_time_x, editing, localization};

#[derive(Clone)]
struct Key { line: usize, time_lines: Vec<usize>, lane: usize, time: f64, value: String, name: String, rect: egui::Rect }
#[derive(Clone)]
enum Drag {
    Keys { keys: Vec<Key>, origin: egui::Pos2, anchor: f64, duration: f64, width: f64, minimum: f64, maximum: f64 },
    Box { origin: egui::Pos2, previous: BTreeSet<usize> },
}
#[derive(Clone, Default)]
struct State { selected: BTreeSet<usize>, primary: Option<usize>, drag: Option<Drag>, revision: Option<usize> }

pub fn publish_owner(ui: &egui::Ui) { ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("timeline_owner"),ui.id())); }
fn selection_id(ui: &egui::Ui) -> egui::Id { ui.ctx().data_mut(|data|data.get_temp::<egui::Id>(egui::Id::new("timeline_owner"))).unwrap_or(ui.id()).with("animation_timeline_selection") }
pub fn selected_lines(ui: &egui::Ui) -> BTreeSet<usize> {
    let id = selection_id(ui);
    ui.ctx().data_mut(|data| data.get_temp::<State>(id)).unwrap_or_default().selected
}
pub fn set_selection(ui: &egui::Ui, selected: &BTreeSet<usize>, primary: usize) {
    let id=selection_id(ui);
    let mut state=ui.ctx().data_mut(|data|data.get_temp::<State>(id)).unwrap_or_default();
    state.selected=selected.clone();state.primary=Some(primary);
    ui.ctx().data_mut(|data|data.insert_temp(id,state));
}

fn select(state: &mut State, key: &Key, items: &[Key], _grouped: bool, additive: bool, toggle: bool) {
    let group: Vec<_> = items.iter().filter(|item| item.line == key.line || item.time_lines.iter().any(|line| key.time_lines.contains(line))).map(|item| item.line).collect();
    let remove = toggle && group.iter().all(|line| state.selected.contains(line));
    if !additive { state.selected.clear(); }
    for line in group { if remove { state.selected.remove(&line); } else { state.selected.insert(line); } }
    state.primary = if remove { state.selected.first().copied() } else { Some(key.line) };
}

fn movement_limits(items: &[Key], selected: &BTreeSet<usize>, duration: f64) -> (f64, f64) {
    let mut minimum = f64::NEG_INFINITY;
    let mut maximum = f64::INFINITY;
    for key in items.iter().filter(|key| selected.contains(&key.line)) {
        minimum = minimum.max(-key.time);
        maximum = maximum.min(duration-key.time);
        for other in items.iter().filter(|other| other.lane == key.lane && !selected.contains(&other.line)) {
            if other.time < key.time { minimum = minimum.max(other.time-key.time+1e-6); }
            else { maximum = maximum.min(other.time-key.time-1e-6); }
        }
    }
    (minimum.min(0.0), maximum.max(0.0))
}

fn snapped_delta(anchor: f64, pixels: f64, width: f64, duration: f64, rate: f64, minimum: f64, maximum: f64) -> f64 {
    let time = anchor + pixels / width.max(1.0) * duration;
    ((time*rate).round()/rate-anchor).clamp(minimum, maximum)
}

pub fn display_duration(ui: &egui::Ui, fallback: f32) -> f32 {
    let id = selection_id(ui);
    let state = ui.ctx().data_mut(|data| data.get_temp::<State>(id));
    match state.and_then(|state| state.drag) {
        Some(Drag::Keys { duration, .. }) => duration as f32,
        _ => fallback,
    }
}

pub fn show(ui: &mut egui::Ui, bindings: &[&AnimationBinding], duration: f32, rate: Option<f32>, grouped: bool,
    primary: &mut Option<SelectedKeyframe>) -> (Option<f32>, Option<egui::Rect>) {
    let id = selection_id(ui);
    let mut state = ui.ctx().data_mut(|data| data.get_temp::<State>(id)).unwrap_or_default();
    let revision = editing::edit_revision(ui);
    if state.revision != revision { state.drag = None; state.selected.clear(); state.primary = None; state.revision = revision; }
    let editable = editing::active(ui);
    let rate = rate.filter(|rate| rate.is_finite() && *rate > 0.0);
    let mut items = Vec::new();
    let mut rows = Vec::new();
    for (lane, binding) in bindings.iter().enumerate() {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
        rows.push(rect);
        for key in &binding.keyframes {
            let x = animation_time_x(rect, key.time as f64, duration as f64);
            items.push(Key { line: key.yaml_line, time_lines: key.time_lines.clone(), lane, time: key.time as f64, value: key.value.clone(),
                name: format!("{} · {} · {}", if binding.path.is_empty() { localization::tr("根对象（本对象）") } else { &binding.path }, binding.class_name, binding.attribute),
                rect: egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(16.0, 16.0)) });
        }
    }
    let Some(first) = rows.first() else { return (None, None) };
    let canvas = first.union(*rows.last().unwrap());
    state.selected.retain(|line| items.iter().any(|key| key.line == *line));
    if state.primary.is_none() && let Some(key) = primary.as_ref().and_then(|selected| items.iter().find(|key| key.line == selected.yaml_line)) {
        select(&mut state, key, &items, grouped, false, false);
    }
    let response = ui.interact(canvas, id, egui::Sense::click_and_drag());
    let additive = ui.input(|input| input.modifiers.ctrl || input.modifiers.command || input.modifiers.shift);
    let pointer = response.interact_pointer_pos();
    if response.secondary_clicked() && let Some(position)=pointer && let Some(key)=items.iter().find(|key|key.rect.contains(position)) {
        if !state.selected.contains(&key.line) { select(&mut state,key,&items,grouped,false,false); }
    }
    let mut deleted=false;
    response.context_menu(|ui| {
        if ui.add_enabled(editable && !state.selected.is_empty(),egui::Button::new(localization::tr("删除选中关键帧"))).clicked() {
            let lines=items.iter().filter(|key|state.selected.contains(&key.line)).flat_map(|key|key.time_lines.iter().copied()).collect();
            deleted=crate::animation_curves::delete(ui,&lines); ui.close_menu();
        }
    });
    if deleted { state=State::default(); *primary=None;ui.ctx().data_mut(|data|data.insert_temp(id,state));return (None,None); }
    if response.clicked() && let Some(position) = pointer {
        if let Some(key) = items.iter().find(|key| key.rect.contains(position)) { select(&mut state, key, &items, grouped, additive, additive); }
        else if !additive { state.selected.clear(); state.primary = None; *primary = None; }
    }
    if response.drag_started() && let Some(origin) = ui.input(|input| input.pointer.press_origin()) {
        if let Some(key) = items.iter().find(|key| key.rect.contains(origin)) {
            if !state.selected.contains(&key.line) { select(&mut state, key, &items, grouped, additive, false); }
            state.primary = Some(key.line);
            if editable && rate.is_some() && duration.is_finite() && duration > 0.0 {
                let (minimum, maximum) = movement_limits(&items, &state.selected, duration as f64);
                state.drag = Some(Drag::Keys { keys: items.iter().filter(|key| state.selected.contains(&key.line)).cloned().collect(),
                    origin, anchor: key.time, duration: duration as f64, width: crate::animation_time_rect(canvas).width() as f64, minimum, maximum });
            }
        } else {
            state.drag = Some(Drag::Box { origin, previous: if additive { state.selected.clone() } else { BTreeSet::new() } });
        }
    }
    if response.dragged() || response.drag_stopped() {
        match state.drag.clone() {
            Some(Drag::Keys { keys, origin, anchor, duration, width, minimum, maximum }) => {
                let pixels = pointer.map_or(0.0, |position| position.x-origin.x);
                let delta = snapped_delta(anchor, pixels as f64, width, duration, rate.unwrap() as f64, minimum, maximum);
                let mut changes: Vec<_> = keys.iter().flat_map(|key| key.time_lines.iter().map(move |line| (*line, key.time+delta))).collect();
                changes.sort_by_key(|(line, _)| *line);
                changes.dedup_by(|left, right| left.0 == right.0);
                if editing::move_animation_keys(ui, &changes, true) {
                    for key in &mut items {
                        if let Some((_, time)) = changes.iter().find(|(line, _)| *line == key.line) {
                            key.time = *time;
                            key.rect = egui::Rect::from_center_size(egui::pos2(animation_time_x(canvas, *time, duration), key.rect.center().y), key.rect.size());
                        }
                    }
                    ui.ctx().request_repaint();
                }
            }
            Some(Drag::Box { origin, previous }) => if let Some(position) = pointer {
                let rect = egui::Rect::from_two_pos(origin, position).intersect(canvas);
                state.selected = previous;
                state.selected.extend(items.iter().filter(|key| rect.contains(key.rect.center())).map(|key| key.line));
                if grouped {
                    let shared_lines: BTreeSet<_> = items.iter().filter(|key| state.selected.contains(&key.line)).flat_map(|key| key.time_lines.iter().copied()).collect();
                    state.selected.extend(items.iter().filter(|key| key.time_lines.iter().any(|line| shared_lines.contains(line))).map(|key| key.line));
                }
                state.primary = state.selected.first().copied();
                ui.painter().rect_filled(rect, 0.0, ui.visuals().hyperlink_color.gamma_multiply(0.15));
                ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0_f32, ui.visuals().hyperlink_color), egui::StrokeKind::Inside);
            },
            None => {}
        }
    }
    if response.drag_stopped() || !ui.input(|input| input.pointer.primary_down()) { state.drag = None; }
    for row in &rows {
        ui.painter().line_segment([egui::pos2(row.left(), row.center().y), egui::pos2(row.right(), row.center().y)], Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color));
    }
    for key in &items {
        let center = key.rect.center();
        let color = if state.selected.contains(&key.line) { if ui.visuals().dark_mode { Color32::YELLOW } else { Color32::from_rgb(190,95,0) } }
            else { crate::animation_curves::color(&bindings[key.lane].attribute) };
        ui.painter().add(egui::Shape::convex_polygon(vec![center+egui::vec2(0.0,-6.0), center+egui::vec2(6.0,0.0), center+egui::vec2(0.0,6.0), center+egui::vec2(-6.0,0.0)], color, Stroke::NONE));
    }
    let active = items.iter().find(|key| Some(key.line) == state.primary && state.selected.contains(&key.line));
    let result = active.map_or((None, rows.last().copied()), |key| {
        *primary = Some(SelectedKeyframe { track_name: key.name.clone(), time: key.time as f32, value: key.value.clone(), yaml_line: key.line });
        (Some(key.rect.center().x), Some(rows[key.lane]))
    });
    if state.selected.is_empty() { *primary = None; }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeline_state_reads_do_not_reenter_context_lock() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                publish_owner(ui);
                assert_eq!(display_duration(ui, 1.0), 1.0);
                assert!(selected_lines(ui).is_empty());
                set_selection(ui, &BTreeSet::from([7]), 7);
                assert_eq!(selected_lines(ui), BTreeSet::from([7]));
            });
        });
    }
    #[test]
    fn simultaneous_independent_components_are_selected_separately() {
        let key = |line,lane| Key {line,time_lines:vec![line],lane,time:0.5,value:"1".into(),name:"component".into(),rect:egui::Rect::ZERO};
        let items=vec![key(10,0),key(20,1)];let mut state=State::default();
        select(&mut state,&items[1],&items,true,false,false);
        assert_eq!(state.selected,BTreeSet::from([20]));
        select(&mut state,&items[0],&items,true,true,true);
        assert_eq!(state.selected,BTreeSet::from([10,20]));
    }
    #[test]
    fn drag_snaps_anchor_and_clamps_group_delta() {
        assert!((snapped_delta(0.1, 25.0, 100.0, 1.0, 30.0, -0.1, 0.5)-0.2666666667).abs() < 1e-8);
        assert_eq!(snapped_delta(0.1, -100.0, 100.0, 1.0, 30.0, -0.1, 0.5), -0.1);
        assert_eq!(snapped_delta(0.1, 100.0, 100.0, 1.0, 30.0, -0.1, 0.5), 0.5);
    }
    #[test]
    fn group_movement_keeps_order_and_avoids_unselected_keys() {
        let key = |line, time| Key { line, time_lines: vec![line], lane: 0, time, value: "1".into(), name: "x".into(), rect: egui::Rect::ZERO };
        let items = vec![key(0,0.0), key(1,0.2), key(2,0.4), key(3,0.6)];
        let (minimum, maximum) = movement_limits(&items, &BTreeSet::from([1,2]), 1.0);
        assert!(minimum > -0.2 && maximum < 0.2);
        let delta = snapped_delta(0.2, 100.0, 100.0, 1.0, 30.0, minimum, maximum);
        assert!(items[2].time+delta < items[3].time);
        assert!(((items[2].time+delta)-(items[1].time+delta)-0.2).abs() < 1e-8);
    }
}
