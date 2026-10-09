use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use crate::{Document, file_ids_for_key, name_from_text};

#[derive(Clone)]
struct View { machine: String, root: String, zoom: f32, pan: Vec2, fit: bool, yaml: bool }
impl Default for View {
    fn default() -> Self { Self { machine: String::new(), root: String::new(), zoom: 1.0, pan: Vec2::ZERO, fit: true, yaml: false } }
}
struct Node { id: String, label: String, index: Option<usize>, position: Pos2, kind: u8 }
struct Edge { from: usize, to: usize, index: Option<usize> }
struct Graph { nodes: Vec<Node>, edges: Vec<Edge> }

fn scalar<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.trim().strip_prefix(&format!("{key}:"))).map(str::trim)
}
fn section<'a>(text: &'a str, key: &str) -> &'a str {
    let mut offset = 0;
    let mut start = None;
    let mut indent = 0;
    for line in text.split_inclusive('\n') {
        let depth = line.len() - line.trim_start().len();
        if start.is_none() && line.trim().starts_with(&format!("{key}:")) {
            start = Some(offset); indent = depth;
        } else if let Some(start) = start {
            if !line.trim().is_empty() && depth <= indent && !line.trim_start().starts_with('-') {
                return &text[start..offset];
            }
        }
        offset += line.len();
    }
    start.map(|start| &text[start..]).unwrap_or("")
}
fn references(text: &str) -> Vec<String> {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN.get_or_init(|| regex::Regex::new(r"fileID:\s*(-?\d+)").unwrap())
        .captures_iter(text).map(|capture| capture[1].to_string()).filter(|id| id != "0").collect()
}
fn stored_position(text: &str, id: &str, key: &str) -> Option<Pos2> {
    let mut matched = false;
    for line in text.lines() {
        if line.contains(&format!("{key}:")) { matched = references(line).iter().any(|candidate| candidate == id); }
        if matched {
            if let Some(value) = line.trim().strip_prefix("m_Position:") {
                let xyz = crate::inline_vector(value, &["x", "y", "z"])?;
                return Some(egui::pos2(xyz[0] as f32, xyz[1] as f32));
            }
        }
    }
    None
}
fn special_position(text: &str, key: &str, fallback: Pos2) -> Pos2 {
    scalar(text, key).and_then(|value| crate::inline_vector(value, &["x", "y", "z"]))
        .map(|values| egui::pos2(values[0] as f32, values[1] as f32)).unwrap_or(fallback)
}

fn graph(documents: &[Document], machine: &Document) -> Graph {
    let mut nodes = vec![
        Node { id: "entry".into(), label: "Entry".into(), index: None, position: special_position(&machine.text, "m_EntryPosition", egui::pos2(20.0, 140.0)), kind: 1 },
        Node { id: "any".into(), label: "Any State".into(), index: None, position: special_position(&machine.text, "m_AnyStatePosition", egui::pos2(20.0, 20.0)), kind: 2 },
        Node { id: "exit".into(), label: "Exit".into(), index: None, position: special_position(&machine.text, "m_ExitPosition", egui::pos2(700.0, 140.0)), kind: 3 },
    ];
    let default = file_ids_for_key(&machine.text, "m_DefaultState").first().cloned();
    for (key, kind) in [("m_State", 0), ("m_StateMachine", 4)] {
        for id in file_ids_for_key(&machine.text, key) {
            if nodes.iter().any(|node| node.id == id) { continue; }
            let Some(index) = documents.iter().position(|doc| doc.file_id == id) else { continue };
            let document = &documents[index];
            let ordinal = nodes.len() - 3;
            let position = stored_position(&machine.text, &id, key)
                .unwrap_or(egui::pos2(240.0 + (ordinal % 3) as f32 * 220.0, 40.0 + (ordinal / 3) as f32 * 100.0));
            nodes.push(Node { id: id.clone(), label: name_from_text(&document.text).unwrap_or_else(|| document.type_name.clone()),
                index: Some(index), position, kind: if default.as_ref() == Some(&id) { 5 } else { kind } });
        }
    }
    let mut edges = Vec::new();
    let mut add = |source: usize, ids: Vec<String>| {
        for id in ids {
            let Some(index) = documents.iter().position(|doc| doc.file_id == id) else { continue };
            let transition = &documents[index];
            let target = file_ids_for_key(&transition.text, "m_DstState").into_iter()
                .chain(file_ids_for_key(&transition.text, "m_DstStateMachine")).find(|id| id != "0");
            let destination = if scalar(&transition.text, "m_IsExit") == Some("1") { Some(2) }
                else { target.and_then(|id| nodes.iter().position(|node| node.id == id)) };
            if let Some(to) = destination { edges.push(Edge { from: source, to, index: Some(index) }); }
        }
    };
    for (i, node) in nodes.iter().enumerate().skip(3) {
        if let Some(index) = node.index { add(i, references(section(&documents[index].text, "m_Transitions"))); }
    }
    add(1, references(section(&machine.text, "m_AnyStateTransitions")));
    add(0, references(section(&machine.text, "m_EntryTransitions")));
    for outgoing in section(&machine.text, "m_StateMachineTransitions").split("- first:").skip(1) {
        let Some((first, rest)) = outgoing.split_once('\n') else { continue };
        if let Some(source) = references(first).first().and_then(|id| nodes.iter().position(|node| &node.id == id)) {
            add(source, references(rest));
        }
    }
    if let Some(to) = default.and_then(|id| nodes.iter().position(|node| node.id == id)) {
        edges.push(Edge { from: 0, to, index: None });
    }
    Graph { nodes, edges }
}

pub fn show(ui: &mut egui::Ui, documents: &[Document], selected: &mut Option<usize>, path: String) {
    let Some(controller) = documents.iter().find(|doc| doc.type_name == "AnimatorController") else { return };
    let id = egui::Id::new(("controller_graph", path));
    let mut view = ui.ctx().data_mut(|data| data.get_temp::<View>(id)).unwrap_or_default();
    let roots = file_ids_for_key(&controller.text, "m_StateMachine");
    if !roots.contains(&view.root) { view.root = roots.first().cloned().unwrap_or_default(); view.machine = view.root.clone(); view.fit = true; }
    ui.horizontal_wrapped(|ui| {
        ui.strong("Animator 关系图");
        ui.selectable_value(&mut view.yaml, false, "显示关系图");
        ui.selectable_value(&mut view.yaml, true, "显示 YAML");
    });
    if view.yaml {
        ui.ctx().data_mut(|data| data.insert_temp(id, view));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("graph_layer").selected_text(documents.iter().find(|doc| doc.file_id == view.root)
            .and_then(|doc| name_from_text(&doc.text)).unwrap_or_else(|| "Layer".into())).show_ui(ui, |ui| {
                for root in &roots {
                    let label = documents.iter().find(|doc| doc.file_id == *root).and_then(|doc| name_from_text(&doc.text)).unwrap_or_else(|| root.clone());
                    if ui.selectable_value(&mut view.root, root.clone(), label).changed() { view.machine = root.clone(); view.fit = true; }
                }
            });
        if ui.button("返回 Layer").clicked() { view.machine = view.root.clone(); view.fit = true; }
        if ui.button("适应画布").clicked() { view.fit = true; }
        ui.small("拖动画布 · 滚轮缩放 · 点击连线 · 双击子状态机进入");
    });
    ui.collapsing("Parameters（参数）", |ui| {
        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| parameters(ui, &controller.text));
    });
    let Some(machine) = documents.iter().find(|doc| doc.file_id == view.machine) else { ui.label("未找到状态机"); return };
    let data = graph(documents, machine);
    let height = (ui.available_height() * 0.48).clamp(180.0, 360.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, if ui.visuals().dark_mode { Color32::from_gray(28) } else { Color32::from_gray(225) });
    if view.fit {
        let bounds = data.nodes.iter().fold(Rect::NOTHING, |bounds, node| bounds.union(Rect::from_min_size(node.position, egui::vec2(160.0, 48.0))));
        view.zoom = ((rect.width() - 40.0) / bounds.width()).min((rect.height() - 40.0) / bounds.height()).clamp(0.15, 1.5);
        view.pan = rect.size() * 0.5 - bounds.center().to_vec2() * view.zoom;
        view.fit = false;
    }
    if response.dragged() { view.pan += ui.input(|input| input.pointer.delta()); }
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let old = view.zoom;
            view.zoom = (old * (scroll * 0.002).exp()).clamp(0.15, 3.0);
            let anchor = ui.input(|input| input.pointer.hover_pos()).unwrap_or(rect.center()) - rect.min;
            view.pan = anchor - (anchor - view.pan) * (view.zoom / old);
        }
    }
    let boxes: Vec<_> = data.nodes.iter().map(|node| Rect::from_min_size(rect.min + view.pan + node.position.to_vec2() * view.zoom, egui::vec2(160.0, 48.0) * view.zoom)).collect();
    let mut edge_hit = None;
    for edge in &data.edges {
        let a = boxes[edge.from].center(); let b = boxes[edge.to].center();
        let color = if edge.index.is_some() && edge.index == *selected { Color32::YELLOW }
            else if edge.index.is_none() { Color32::from_rgb(180, 115, 25) } else { Color32::from_gray(110) };
        let points = if edge.from == edge.to {
            let bounds = boxes[edge.from];
            vec![bounds.right_center(), egui::pos2(bounds.right() + 25.0, a.y),
                egui::pos2(bounds.right() + 25.0, bounds.top() - 25.0),
                egui::pos2(a.x, bounds.top() - 25.0), bounds.center_top()]
        } else {
            let direction = (b - a).normalized();
            let boundary = |bounds: Rect, direction: Vec2| {
                let distance = (bounds.width() * 0.5 / direction.x.abs().max(0.001))
                    .min(bounds.height() * 0.5 / direction.y.abs().max(0.001));
                bounds.center() + direction * distance
            };
            let offset = egui::vec2(-direction.y, direction.x) * 5.0;
            vec![boundary(boxes[edge.from], direction) + offset, boundary(boxes[edge.to], -direction) + offset]
        };
        painter.add(egui::Shape::line(points.clone(), Stroke::new(2.0_f32, color)));
        let end = points[points.len() - 1];
        let start = points[points.len() - 2];
        let direction = (end - start).normalized(); let normal = egui::vec2(-direction.y, direction.x);
        painter.add(egui::Shape::convex_polygon(vec![end, end - direction * 10.0 + normal * 4.0, end - direction * 10.0 - normal * 4.0], color, Stroke::NONE));
        if response.clicked() {
            if let Some(point) = response.interact_pointer_pos() {
                for segment in points.windows(2) {
                    let line = segment[1] - segment[0];
                    let nearest = segment[0] + line * ((point - segment[0]).dot(line) / line.length_sq().max(1.0)).clamp(0.0, 1.0);
                    if point.distance(nearest) < 8.0 { edge_hit = edge.index; }
                }
            }
        }
    }
    if let Some(index) = edge_hit { *selected = Some(index); }
    for (i, node) in data.nodes.iter().enumerate() {
        let bounds = boxes[i];
        let color = match node.kind { 1 => Color32::from_rgb(30, 130, 65), 2 => Color32::from_rgb(45, 135, 140), 3 => Color32::from_rgb(160, 40, 45), 5 => Color32::from_rgb(175, 100, 20), _ => Color32::from_gray(75) };
        painter.rect_filled(bounds, 4.0, color);
        painter.rect_stroke(bounds, 4.0, Stroke::new(2.0_f32, if node.index.is_some() && node.index == *selected { Color32::YELLOW } else { Color32::from_gray(125) }), egui::StrokeKind::Inside);
        let label = painter.layout(node.label.clone(), egui::FontId::proportional((13.0 * view.zoom).clamp(8.0, 20.0)), Color32::WHITE, bounds.width() - 8.0);
        painter.galley(bounds.center() - label.size() * 0.5, label, Color32::WHITE);
        let interaction = ui.interact(bounds.intersect(rect), id.with(&node.id), Sense::click());
        if interaction.clicked() { if let Some(index) = node.index { *selected = Some(index); } }
        if interaction.double_clicked() && node.kind == 4 { view.machine = node.id.clone(); view.fit = true; }
    }
    if let Some(document) = selected.and_then(|index| documents.get(index)) {
        if matches!(document.type_name.as_str(), "AnimatorStateTransition" | "AnimatorTransition") { transition_details(ui, document); }
        else if document.type_name == "AnimatorState" {
            ui.label(format!("状态：{}", name_from_text(&document.text).unwrap_or_default()));
            crate::script_properties::show_graph_reference(ui, "Motion", scalar(&document.text, "m_Motion").unwrap_or("{fileID: 0}"));
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, view));
}

fn parameters(ui: &mut egui::Ui, text: &str) {
    let entries = parse_parameters(text);
    if entries.is_empty() { ui.label("没有参数"); return; }
    egui::Grid::new("controller_parameters").striped(true).show(ui, |ui| {
        for title in ["名称", "类型", "默认值"] { ui.strong(title); } ui.end_row();
        for (name, kind, value) in entries {
            ui.label(name); ui.label(kind); ui.label(value); ui.end_row();
        }
    });
}

fn parse_parameters(text: &str) -> Vec<(String, String, String)> {
    let raw = section(text, "m_AnimatorParameters");
    let entries: Vec<_> = raw.split("- m_Name:").skip(1).collect();
    entries.into_iter().map(|entry| {
            let name = entry.lines().next().unwrap_or("").trim();
            let (kind, value) = match scalar(entry, "m_Type") {
                Some("1") => ("Float", scalar(entry, "m_DefaultFloat").unwrap_or("0").to_string()),
                Some("3") => ("Int", scalar(entry, "m_DefaultInt").unwrap_or("0").to_string()),
                Some("4") => ("Bool", if scalar(entry, "m_DefaultBool") == Some("1") { "true" } else { "false" }.into()),
                Some("9") => ("Trigger", "—".into()),
                _ => ("未知", scalar(entry, "m_Type").unwrap_or("").to_string()),
            };
            (name.to_string(), kind.to_string(), value)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_parameter_types_and_defaults_without_layer_fields() {
        let text = "AnimatorController:\n  m_AnimatorParameters:\n  - m_Name: speed\n    m_Type: 1\n    m_DefaultFloat: 0.25\n  - m_Name: count\n    m_Type: 3\n    m_DefaultInt: 2\n  - m_Name: active\n    m_Type: 4\n    m_DefaultBool: 1\n  - m_Name: fire\n    m_Type: 9\n  m_AnimatorLayers:\n  - m_Name: Base Layer\n";
        let parameters = parse_parameters(text);
        assert_eq!(parameters.len(), 4);
        assert_eq!(parameters[0], ("speed".into(), "Float".into(), "0.25".into()));
        assert_eq!(parameters[1].2, "2");
        assert_eq!(parameters[2].2, "true");
        assert_eq!(parameters[3].1, "Trigger");
    }

    #[test]
    fn builds_default_any_state_entry_exit_and_submachine_edges() {
        let documents = crate::parse_unity_yaml("--- !u!1107 &10\nAnimatorStateMachine:\n  m_Name: Base Layer\n  m_ChildStates:\n  - serializedVersion: 1\n    m_State: {fileID: 20}\n    m_Position: {x: 300, y: 100, z: 0}\n  m_ChildStateMachines:\n  - m_StateMachine: {fileID: 30}\n    m_Position: {x: 600, y: 100, z: 0}\n  m_AnyStateTransitions:\n  - {fileID: 40}\n  m_EntryTransitions:\n  - {fileID: 42}\n  m_DefaultState: {fileID: 20}\n--- !u!1102 &20\nAnimatorState:\n  m_Name: Idle\n  m_Transitions:\n  - {fileID: 41}\n--- !u!1107 &30\nAnimatorStateMachine:\n  m_Name: Nested\n--- !u!1101 &40\nAnimatorStateTransition:\n  m_DstState: {fileID: 20}\n  m_IsExit: 0\n--- !u!1101 &41\nAnimatorStateTransition:\n  m_DstState: {fileID: 0}\n  m_IsExit: 1\n--- !u!1109 &42\nAnimatorTransition:\n  m_DstStateMachine: {fileID: 30}\n");
        let graph = graph(&documents, &documents[0]);
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.nodes[3].kind, 5);
        assert_eq!(graph.nodes[3].position, egui::pos2(300.0, 100.0));
        assert_eq!(graph.nodes[4].kind, 4);
        assert_eq!(graph.edges.len(), 4);
        assert!(graph.edges.iter().any(|edge| edge.from == 3 && edge.to == 2));
        assert!(graph.edges.iter().any(|edge| edge.from == 1 && edge.to == 3));
        assert!(graph.edges.iter().any(|edge| edge.from == 0 && edge.to == 4));
        assert!(graph.edges.iter().any(|edge| edge.index.is_none() && edge.from == 0 && edge.to == 3));
    }
}

fn transition_details(ui: &mut egui::Ui, document: &Document) {
    egui::CollapsingHeader::new("过渡属性与 Conditions").default_open(true).show(ui, |ui| {
        for key in ["m_HasExitTime", "m_ExitTime", "m_TransitionDuration", "m_TransitionOffset", "m_HasFixedDuration", "m_InterruptionSource", "m_OrderedInterruption", "m_CanTransitionToSelf"] {
            if let Some(value) = scalar(&document.text, key) { ui.label(format!("{}: {value}", key.trim_start_matches("m_"))); }
        }
        let conditions = section(&document.text, "m_Conditions");
        for condition in conditions.split("- m_ConditionMode:").skip(1) {
            let mode = condition.lines().next().unwrap_or("").trim();
            let operator = match mode { "1" => "If", "2" => "IfNot", "3" => ">", "4" => "<", "6" => "==", "7" => "!=", _ => mode };
            ui.label(format!("{} {operator} {}", scalar(condition, "m_ConditionEvent").unwrap_or("?"), scalar(condition, "m_EventTreshold").or_else(|| scalar(condition, "m_EventThreshold")).unwrap_or("")));
        }
    });
}
