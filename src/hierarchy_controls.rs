//! Viewer-only visibility and selection switches. No Unity fields are changed.
use std::collections::{HashMap, HashSet};
use eframe::egui::{self, Color32};
use crate::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags {
    pub visible: bool,
    pub selectable: bool,
}
impl Default for Flags {
    fn default() -> Self { Self { visible: true, selectable: true } }
}
#[derive(Default)]
pub struct NodeControls {
    local: HashMap<String, Flags>,
}
impl NodeControls {
    pub fn local(&self, id: &str) -> Flags { self.local.get(id).copied().unwrap_or_default() }
    pub fn toggle(&mut self, id: &str, eye: bool) {
        let flags = self.local.entry(id.to_owned()).or_default();
        if eye { flags.visible = !flags.visible; } else { flags.selectable = !flags.selectable; }
        if *flags == Flags::default() { self.local.remove(id); }
    }
    pub fn effective(&self, documents: &[Document]) -> HashMap<String, Flags> {
        let transforms: HashMap<_, _> = documents.iter()
            .filter(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform"))
            .map(|doc| (doc.file_id.as_str(), doc)).collect();
        let parents: HashMap<_, _> = transforms.values().filter_map(|doc| {
            let object = doc.game_object.as_deref()?;
            let mut current=doc.parent_transform.as_deref();let mut visited=HashSet::new();let mut parent=None;
            while let Some(id)=current.filter(|id|*id!="0") {
                if !visited.insert(id) {parent=Some(object);break;}
                let Some(transform)=transforms.get(id) else {break;};
                if let Some(go)=transform.game_object.as_deref() {parent=Some(go);break;}
                current=transform.parent_transform.as_deref();
            }
            Some((object, parent))
        }).collect();
        documents.iter().filter(|doc| doc.type_name == "GameObject").map(|doc| {
            let mut flags = Flags::default();
            let mut current = Some(doc.file_id.as_str());
            let mut visited = HashSet::new();
            while let Some(id) = current {
                if !visited.insert(id) { flags = Flags { visible: false, selectable: false }; break; }
                let local = self.local(id);
                flags.visible &= local.visible;
                flags.selectable &= local.selectable;
                current = parents.get(id).copied().flatten();
            }
            (doc.file_id.clone(), flags)
        }).collect()
    }
}

/// Shared SVG icons retain local and inherited switch appearance.
pub fn icon(ui: &mut egui::Ui, eye: bool, enabled: bool, inherited: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(15.0, 20.0), egui::Sense::click());
    let painter = ui.painter_at(rect);
    if response.hovered() { painter.rect_filled(rect, 3.0, ui.visuals().widgets.hovered.bg_fill); }
    let color = if inherited { ui.visuals().weak_text_color() }
        else if enabled { ui.visuals().text_color() } else { ui.visuals().weak_text_color() };
    let p = |x: f32, y: f32| rect.min + egui::vec2(x * 0.75 + 0.4, y * 0.75 + 2.5);
    crate::icons::paint(ui, if eye { crate::icons::Icon::Eye } else { crate::icons::Icon::Hand },
        egui::Rect::from_center_size(rect.center(), egui::vec2(15.0, 15.0)), color);
    if !enabled { painter.line_segment([p(2.0,17.0),p(17.0,3.0)],egui::Stroke::new(1.6_f32,if inherited {color} else {Color32::from_rgb(220,100,100)})); }
    let label = if inherited {
        if eye { "父节点已关闭渲染；本节点开关在父节点恢复后生效。" } else { "父节点已禁止选中；本节点开关在父节点恢复后生效。" }
    } else if eye {
        if enabled { "关闭此节点及子节点的预览渲染" } else { "开启此节点及子节点的预览渲染" }
    } else if enabled { "禁止选中此节点及子节点" } else { "允许选中此节点及子节点" };
    response.on_hover_text(crate::localization::tr(label))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn documents() -> Vec<Document> {
        [("1","GameObject",None,None),("2","GameObject",None,None),
         ("10","Transform",Some("1"),Some("0")),("20","RectTransform",Some("2"),Some("10"))]
        .into_iter().map(|(id,kind,object,parent)| Document { file_id:id.into(), type_name:kind.into(), start_line:1,start_offset:0,text:"".into(),game_object:object.map(str::to_owned),parent_transform:parent.map(str::to_owned) }).collect()
    }
    #[test]
    fn parent_switches_propagate_independently_and_child_preferences_survive() {
        let mut controls=NodeControls::default(); let docs=documents();
        controls.toggle("1",true);
        assert!(!controls.effective(&docs)["2"].visible);
        assert!(controls.effective(&docs)["2"].selectable);
        controls.toggle("2",true); controls.toggle("1",true);
        assert!(controls.effective(&docs)["1"].visible);
        assert!(!controls.effective(&docs)["2"].visible);
        controls.toggle("1",false);
        assert!(!controls.effective(&docs)["2"].selectable);
        controls.toggle("1",false);
        assert!(controls.effective(&docs)["2"].selectable);
    }
    #[test]
    fn cycles_are_disabled_and_empty_state_is_enabled() {
        let controls=NodeControls::default();let mut docs=documents();
        assert_eq!(controls.effective(&docs)["2"],Flags::default());
        docs[2].parent_transform=Some("20".into());
        assert_eq!(controls.effective(&docs)["1"],Flags{visible:false,selectable:false});
    }
}
