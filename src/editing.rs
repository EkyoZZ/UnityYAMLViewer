//! Transactional property editing. All offsets refer to the untouched original.
use std::{collections::{BTreeMap, VecDeque}, ops::Range, path::PathBuf, sync::{Arc, Mutex}};
pub const DEFAULT_UNDO_LIMIT: usize = 200;
pub const MIN_UNDO_LIMIT: usize = 50;
pub const MAX_UNDO_LIMIT: usize = 1000;
use eframe::egui::{self, Color32, TextEdit};

#[derive(Clone)]
pub struct Session {
    pub path: PathBuf,
    pub original: String,
    pub disk: Vec<u8>,
    patches: BTreeMap<usize, (usize, String)>,
    saved_patches: BTreeMap<usize, (usize, String)>,
    undo: VecDeque<(String, BTreeMap<usize, (usize, String)>)>,
    saved_original: String,
    drafts: BTreeMap<usize, String>,
    component_drafts: BTreeMap<(usize, usize), String>,
    revision: usize,
    undo_limit: usize,
    active_drag: Option<usize>,
    pub warning: Option<String>,
}
pub type Shared = Arc<Mutex<Session>>;
#[derive(Clone)]
struct Source { session: Shared, range: Range<usize> }
#[derive(Clone)]
pub struct Binding { source: Source, range: Range<usize> }
fn id() -> egui::Id { egui::Id::new("property_edit_source") }
impl Session {
    pub fn new(path: PathBuf, original: String, disk: Vec<u8>) -> Self {
        Self { path, saved_original: original.clone(), original, disk, patches: BTreeMap::new(), saved_patches: BTreeMap::new(), undo: VecDeque::new(), drafts: BTreeMap::new(), component_drafts: BTreeMap::new(), revision: 0, undo_limit: DEFAULT_UNDO_LIMIT, active_drag: None, warning: None }
    }
    pub fn rendered(&self) -> String {
        let mut text = self.original.clone();
        for (start, (end, value)) in self.patches.iter().rev() { text.replace_range(*start..*end, value); }
        text
    }
    pub fn undo(&mut self) {
        if !self.has_drafts() { if let Some((original, previous)) = self.undo.pop_back() { self.original = original; self.patches = previous; } }
        self.drafts.clear(); self.component_drafts.clear(); self.revision += 1; self.active_drag = None;
    }
    fn record_undo(&mut self, previous: BTreeMap<usize, (usize, String)>) {
        if self.undo.len() == self.undo_limit { self.undo.pop_front(); }
        self.undo.push_back((self.original.clone(), previous));
    }
    fn change(&mut self, range: Range<usize>, value: String) {
        self.active_drag = None;
        self.change_grouped(range, value, false);
        self.active_drag = None;
    }
    fn change_grouped(&mut self, range: Range<usize>, value: String, continuing: bool) {
        let current = self.patches.get(&range.start).map(|(_, value)| value.as_str()).unwrap_or(&self.original[range.clone()]);
        if current == value { return; }
        if self.patches.iter().any(|(start, (end, _))| *start != range.start && *start < range.end && *end > range.start) {
            self.warning = Some(crate::localization::tr("此属性与另一项修改重叠，已拒绝修改。").into()); return;
        }
        if !continuing || self.active_drag != Some(range.start) { self.record_undo(self.patches.clone()); }
        self.active_drag = Some(range.start);
        if self.original[range.clone()] == value { self.patches.remove(&range.start); }
        else { self.patches.insert(range.start, (range.end, value)); }
    }
    pub fn has_drafts(&self) -> bool { !self.drafts.is_empty() || !self.component_drafts.is_empty() }
    pub fn has_changes(&self) -> bool {
        self.rendered() != baseline_text(&self.disk) || self.has_drafts()
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() || self.has_drafts() }
    pub fn mark_saved(&mut self, text: &str) {
        self.saved_patches = self.patches.clone();
        self.saved_original = self.original.clone();
        let bom = self.disk.starts_with(&[0xef, 0xbb, 0xbf]);
        self.disk = text.as_bytes().to_vec();
        if !text.starts_with('\u{feff}') && bom {
            self.disk.splice(0..0, [0xef, 0xbb, 0xbf]);
        }
        self.finish_drag();
    }
    pub fn has_changes_in(&self, range: Range<usize>) -> bool {
        if self.original != self.saved_original {
            let changed_structure = self.original.get(range.clone()).and_then(|text| crate::parse_unity_yaml(text).first().map(|doc| (doc.file_id.clone(),doc.text.clone())))
                .is_some_and(|(file_id,text)| !crate::parse_unity_yaml(&self.saved_original).iter().any(|doc| doc.file_id==file_id && doc.text==text));
            return changed_structure || self.patches.keys().chain(self.drafts.keys()).chain(self.component_drafts.keys().map(|(start, _)| start))
                .any(|start| range.contains(start));
        }
        self.patches.keys().chain(self.saved_patches.keys())
            .filter(|start| self.patches.get(start) != self.saved_patches.get(start))
            .chain(self.drafts.keys())
            .chain(self.component_drafts.keys().map(|(start, _)| start))
            .any(|start| range.contains(start))
    }
    pub fn finish_drag(&mut self) { self.active_drag = None; }
    fn change_many(&mut self, changes: Vec<(Range<usize>, String)>, continuing: bool) -> bool {
        if self.has_drafts() { return false; }
        if changes.iter().enumerate().any(|(i, (range, _))| range.end > self.original.len()
            || changes[..i].iter().any(|(other, _)| other.start < range.end && other.end > range.start)
            || self.patches.iter().any(|(start, (end, _))| *start != range.start && *start < range.end && *end > range.start)) { return false; }
        let changed = changes.iter().any(|(range, value)| self.patches.get(&range.start).map(|(_, value)| value.as_str()).unwrap_or(&self.original[range.clone()]) != value);
        if !changed { return true; }
        let Some(anchor) = changes.first().map(|(range, _)| range.start) else { return true };
        if !continuing || self.active_drag != Some(anchor) { self.record_undo(self.patches.clone()); }
        self.active_drag = Some(anchor);
        for (range, value) in changes {
            if self.original[range.clone()] == value { self.patches.remove(&range.start); }
            else { self.patches.insert(range.start, (range.end, value)); }
        }
        true
    }
    pub fn set_undo_limit(&mut self, limit: usize) {
        self.undo_limit = limit.clamp(MIN_UNDO_LIMIT, MAX_UNDO_LIMIT);
        while self.undo.len() > self.undo_limit { self.undo.pop_front(); }
    }
}
fn baseline_text(disk: &[u8]) -> String {
    let disk = disk.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(disk);
    String::from_utf8_lossy(disk).into_owned()
}
pub fn create_empty_controller_state(ui: &egui::Ui, machine: &str, position: egui::Pos2) -> Option<String> {
    if !position.x.is_finite() || !position.y.is_finite() { return None; }
    let source = source(ui)?;
    let mut session = source.session.lock().unwrap();
    if session.has_drafts() {
        session.warning = Some(crate::localization::tr("请先确认正在输入的属性，再创建状态。").into()); return None;
    }
    let mut text = session.rendered();
    let documents = crate::parse_unity_yaml(&text);
    let owner = documents.iter().position(|doc| doc.file_id == machine && doc.type_name == "AnimatorStateMachine")?;
    let owner_start = text.find("--- !u!")? + documents[..owner].iter().map(|doc| doc.text.len()).sum::<usize>();
    let mut file_id = 1_102_000_000_i64;
    while documents.iter().any(|doc| doc.file_id == file_id.to_string()) { file_id = file_id.checked_add(1)?; }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut name = "New State".to_string();
    let mut suffix = 1;
    while documents.iter().any(|doc| crate::name_from_text(&doc.text).as_deref() == Some(&name)) {
        name = format!("New State {suffix}"); suffix += 1;
    }
    let entry = format!("  - serializedVersion: 1{newline}    m_State: {{fileID: {file_id}}}{newline}    m_Position: {{x: {}, y: {}, z: 0}}{newline}", position.x, position.y);
    let mut offset = owner_start;
    let mut insertion = None;
    let mut in_list = false;
    for line in documents[owner].text.split_inclusive('\n') {
        if line.trim().starts_with("m_ChildStates:") {
            let value = line.trim().strip_prefix("m_ChildStates:")?.trim();
            if value == "[]" {
                insertion = Some((offset..offset + line.len(), format!("  m_ChildStates:{newline}{entry}"))); break;
            }
            if !value.is_empty() { return None; }
            in_list = true;
        } else if in_list && !line.trim().is_empty() && line.starts_with("  ") && !line.starts_with("    ") && !line.trim_start().starts_with('-') {
            insertion = Some((offset..offset, entry.clone())); break;
        }
        offset += line.len();
    }
    let (range, replacement) = insertion.or_else(|| in_list.then(|| (offset..offset, entry)))?;
    text.replace_range(range, &replacement);
    if !text.ends_with('\n') { text.push_str(newline); }
    text.push_str(&format!("--- !u!1102 &{file_id}{newline}AnimatorState:{newline}  serializedVersion: 6{newline}  m_ObjectHideFlags: 1{newline}  m_CorrespondingSourceObject: {{fileID: 0}}{newline}  m_PrefabInstance: {{fileID: 0}}{newline}  m_PrefabAsset: {{fileID: 0}}{newline}  m_Name: {name}{newline}  m_Speed: 1{newline}  m_CycleOffset: 0{newline}  m_Transitions: []{newline}  m_StateMachineBehaviours: []{newline}  m_Position: {{x: 50, y: 50, z: 0}}{newline}  m_IKOnFeet: 0{newline}  m_WriteDefaultValues: 1{newline}  m_Mirror: 0{newline}  m_SpeedParameterActive: 0{newline}  m_MirrorParameterActive: 0{newline}  m_CycleOffsetParameterActive: 0{newline}  m_TimeParameterActive: 0{newline}  m_Motion: {{fileID: 0}}{newline}  m_Tag:{newline}  m_SpeedParameter:{newline}  m_MirrorParameter:{newline}  m_CycleOffsetParameter:{newline}  m_TimeParameter:{newline}"));
    let created = crate::parse_unity_yaml(&text);
    if created.len() != documents.len() + 1
        || created.iter().zip(&documents).any(|(after, before)| after.file_id != before.file_id || after.type_name != before.type_name)
        || created.iter().filter(|doc| doc.file_id == file_id.to_string()).count() != 1
        || crate::file_ids_for_key(&created[owner].text, "m_State").iter().filter(|id| **id == file_id.to_string()).count() != 1 {
        return None;
    }
    let previous = session.patches.clone();
    session.record_undo(previous);
    session.original = text;
    session.patches.clear();
    session.component_drafts.clear();
    session.revision += 1;
    session.finish_drag();
    Some(file_id.to_string())
}
pub fn publish(ctx: &egui::Context, session: Option<Shared>, range: Range<usize>) {
    ctx.data_mut(|data| { data.remove::<Source>(id()); if let Some(session) = session { data.insert_temp(id(), Source { session, range }); } });
}
pub fn active(ui: &egui::Ui) -> bool { ui.ctx().data(|data| data.get_temp::<Source>(id())).is_some() }
fn controller_source(ui: &egui::Ui, machine: &str) -> Option<Source> {
    let mut source = source(ui)?;
    let session = source.session.lock().unwrap();
    let documents = crate::parse_unity_yaml(&session.original);
    let index = documents.iter().position(|doc| doc.file_id == machine)?;
    let start = session.original.find("--- !u!")? + documents[..index].iter().map(|doc| doc.text.len()).sum::<usize>();
    source.range = start..start + documents[index].text.len();
    drop(session);
    Some(source)
}
pub fn controller_machine_text(ui: &egui::Ui, machine: &str) -> Option<String> {
    document_text(ui, machine)
}
pub fn document_text(ui: &egui::Ui, machine: &str) -> Option<String> {
    let source = controller_source(ui, machine)?;
    document_text_from_source(source)
}
pub fn document_text_in_range(ui: &egui::Ui, range: Range<usize>) -> Option<String> {
    let mut source = source(ui)?;
    source.range = range;
    document_text_from_source(source)
}
fn document_text_from_source(source: Source) -> Option<String> {
    let session = source.session.lock().unwrap();
    let mut text = session.original.get(source.range.clone())?.to_string();
    let mut preview = session.patches.clone();
    for (start, draft) in session.drafts.iter().filter(|(start, _)| source.range.contains(start)) {
        if session.original[..*start].lines().last().is_some_and(|line| line.trim() == "m_Name:") {
            let value = session.original[*start..].lines().next().unwrap_or("").trim_end();
            let state = session.original[..*start].rsplit("---").next().is_some_and(|document| document.starts_with(" !u!1102 "));
            preview.insert(*start, (*start + value.len(), quoted(draft, if state { "" } else { value })));
        }
    }
    for (start, (end, value)) in preview.iter().rev().filter(|(start, (end, _))| **start >= source.range.start && *end <= source.range.end) {
        text.replace_range(start - source.range.start..end - source.range.start, value);
    }
    Some(text)
}
fn controller_binding(ui: &egui::Ui, machine: &str, node: Option<(&str, &str)>) -> Option<Binding> {
    let source = controller_source(ui, machine)?;
    let session = source.session.lock().unwrap();
    let mut offset = source.range.start;
    let mut matched = false;
    let mut found = None;
    for line in session.original[source.range.clone()].split_inclusive('\n') {
        let trimmed = line.trim().trim_start_matches("- ");
        if line.trim_start().starts_with('-') { matched = false; }
        if let Some((key, value)) = trimmed.split_once(':') {
            if let Some((reference_key, file_id)) = node {
                if key == reference_key { matched = value.trim() == format!("{{fileID: {file_id}}}"); }
            }
            let special_position = node.is_some_and(|(reference_key, position_key)| reference_key.is_empty()
                && matches!(position_key, "m_EntryPosition" | "m_AnyStatePosition" | "m_ExitPosition") && key == position_key);
            if (node.is_none() && key == "m_DefaultState") || (matched && key == "m_Position") || special_position {
                let colon = line.find(':')? + 1;
                let start = offset + colon + line[colon..].len() - line[colon..].trim_start().len();
                if found.is_some() { return None; }
                found = Some(start..start + value.trim().len());
            }
        }
        offset += line.len();
    }
    drop(session);
    Some(Binding { source, range: found? })
}
pub fn drag_controller_node(ui: &egui::Ui, machine: &str, node: &str, key: &str, delta: egui::Vec2, continuing: bool) {
    let target = if node.is_empty() { ("", key) } else { (key, node) };
    let Some(bind) = controller_binding(ui, machine, Some(target)) else { return };
    let value = bind.value();
    let Some(mut xyz) = crate::inline_vector(&value, &["x", "y", "z"]) else { return };
    xyz[0] += delta.x as f64; xyz[1] += delta.y as f64;
    if xyz.iter().all(|value| value.is_finite()) {
        bind.source.session.lock().unwrap().change_grouped(bind.range.clone(), float_components(&value, &["x", "y", "z"], &xyz), continuing);
    }
}
pub fn set_controller_default(ui: &egui::Ui, machine: &str, node: &str) {
    if let Some(bind) = controller_binding(ui, machine, None) { bind.apply(format!("{{fileID: {node}}}")); }
}
pub fn scope_value(ui: &egui::Ui) -> Option<String> {
    let source = source(ui)?;
    let session = source.session.lock().unwrap();
    let mut value = session.original[source.range.clone()].to_string();
    for (start, (end, patch)) in session.patches.iter().rev().filter(|(start, (end, _))| **start >= source.range.start && *end <= source.range.end) {
        value.replace_range(start - source.range.start..end - source.range.start, patch);
    }
    Some(value)
}
pub fn edit_revision(ui: &egui::Ui) -> Option<usize> {
    source(ui).map(|source| source.session.lock().unwrap().revision)
}
pub fn named_binding(ui: &egui::Ui, name: &str) -> Option<Binding> {
    let source = source(ui)?;
    let session = source.session.lock().unwrap();
    let mut offset = source.range.start;
    let mut ranges = Vec::new();
    for line in session.original[source.range.clone()].split_inclusive('\n') {
        if let Some((key, value)) = line.trim().trim_start_matches("- ").split_once(':') {
            if key == name && !value.trim().is_empty() {
                let colon = line.find(':')? + 1;
                let start = offset + colon + line[colon..].len() - line[colon..].trim_start().len();
                ranges.push(start..start + value.trim().len());
            }
        }
        offset += line.len();
    }
    if ranges.len() != 1 { return None; }
    drop(session);
    Some(Binding { source, range: ranges.remove(0) })
}
/// Bind an exact document line so equal values in different animation keys
/// cannot cause an ambiguous match or modify another keyframe.
pub fn line_binding(ui: &egui::Ui, line_index: usize, name: &str) -> Option<Binding> {
    let source = source(ui)?;
    let session = source.session.lock().unwrap();
    let text = session.original.get(source.range.clone())?;
    let mut offset = source.range.start;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if index == line_index {
            let (key, value) = line.trim().split_once(':')?;
            if key != name || value.trim().is_empty() { return None; }
            let colon = line.find(':')? + 1;
            let start = offset + colon + line[colon..].len() - line[colon..].trim_start().len();
            let range = start..start + value.trim().len();
            drop(session);
            return Some(Binding { source, range });
        }
        offset += line.len();
    }
    None
}

pub fn set_animation_values(ui: &egui::Ui, values: &[(usize, f64)]) -> bool {
    let Some(source) = source(ui) else { return false };
    let mut changes = Vec::new();
    for (line, value) in values {
        if !value.is_finite() || value.abs() > f32::MAX as f64 { return false; }
        let Some(bind) = line_binding(ui, *line, "value") else { return false };
        changes.push((bind.range, (*value as f32).to_string()));
    }
    changes.sort_by_key(|(range, _)| range.start);
    changes.dedup_by_key(|(range, _)| range.start);
    let continuing = ui.input(|input| input.pointer.primary_down());
    source.session.lock().unwrap().change_many(changes, continuing)
}

pub fn move_animation_keys(ui: &egui::Ui, keys: &[(usize, f64)], continuing: bool) -> bool {
    let Some(source) = source(ui) else { return false };
    if source.session.lock().unwrap().has_drafts() {
        warning(ui, crate::localization::tr("请先确认正在输入的值，再拖动关键帧。"));
        return false;
    }
    let mut changes = Vec::new();
    for (line, time) in keys {
        if !time.is_finite() || *time < 0.0 || *time > f32::MAX as f64 { return false; }
        let Some(bind) = line_binding(ui, *line, "time") else { return false };
        changes.push((bind.range, (*time as f32).to_string()));
    }
    if let Some(bind) = named_binding(ui,"m_StopTime") {
        let Some(text) = scope_value(ui) else { return false };
        let end = animation_end_after_moves(&text, keys);
        changes.push((bind.range,(end as f32).to_string()));
    }
    source.session.lock().unwrap().change_many(changes, continuing)
}
fn animation_end_after_moves(text: &str, keys: &[(usize, f64)]) -> f64 {
    text.lines().enumerate().filter_map(|(index, line)| {
        let value = line.trim().trim_start_matches("- ").strip_prefix("time:")?;
        keys.iter().find(|(line, _)| *line == index).map(|(_, time)| *time)
            .or_else(|| value.trim().parse::<f64>().ok())
            .filter(|time| time.is_finite() && *time >= 0.0)
    }).fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod animation_duration_tests {
    use super::*;
    #[test]
    fn end_follows_moved_keys_and_keeps_other_tracks() {
        let text = "time: 1.4\nvalue: 1\ntime: 1.4\nvalue: 2\ntime: 1\nvalue: 3\nm_StopTime: 1.4\n";
        let time = 40.0 / 30.0;
        assert_eq!(animation_end_after_moves(text, &[(0,time),(2,time)]),time);
        assert_eq!(animation_end_after_moves(text, &[(0,time)]),1.4);
        assert_eq!(animation_end_after_moves(text, &[(0,1.5),(2,1.5)]),1.5);
    }
}
/// Commit a structural animation edit as one undo operation.
pub fn change_animation_document(ui: &egui::Ui, transform: impl FnOnce(&str) -> Option<String>) -> bool {
    let Some(source) = source(ui) else { return false };
    let mut session = source.session.lock().unwrap();
    if session.has_drafts() { session.warning = Some(crate::localization::tr("请先确认正在输入的值，再拖动关键帧。").into()); return false; }
    let Some(owner) = session.original.get(source.range.clone()).and_then(|text| crate::parse_unity_yaml(text).first().map(|doc| doc.file_id.clone())) else { return false };
    let mut text = session.rendered();
    let docs = crate::parse_unity_yaml(&text);
    let Some(index) = docs.iter().position(|doc| doc.file_id == owner) else { return false };
    let start = text.find("--- !u!").unwrap_or(0) + docs[..index].iter().map(|doc| doc.text.len()).sum::<usize>();
    let Some(replacement) = transform(&docs[index].text) else { return false };
    if replacement == docs[index].text.as_ref() { return false; }
    text.replace_range(start..start + docs[index].text.len(), &replacement);
    let after = crate::parse_unity_yaml(&text);
    if after.len() != docs.len() || after.iter().zip(&docs).any(|(a,b)| a.file_id != b.file_id || a.type_name != b.type_name) { return false; }
    let previous = session.patches.clone();
    let structural = replacement.lines().count() != docs[index].text.lines().count();
    let continuing = !structural && ui.input(|input| input.pointer.primary_down());
    if !continuing || session.active_drag != Some(source.range.start) { session.record_undo(previous); }
    session.original = text;
    session.patches.clear();
    if structural { session.revision += 1; }
    session.active_drag = if continuing { Some(source.range.start) } else { None };
    drop(session);
    ui.ctx().data_mut(|data| data.remove::<Source>(id()));
    ui.ctx().request_repaint();
    true
}
fn source(ui: &egui::Ui) -> Option<Source> { ui.ctx().data(|data| data.get_temp(id())) }
pub fn warning(ui: &egui::Ui, message: &str) { if let Some(source) = source(ui) { source.session.lock().unwrap().warning = Some(message.into()); } }
pub fn reference_warning(ui: &egui::Ui, rect: egui::Rect) {
    if active(ui) && ui.interact(rect, ui.id().with("locked_reference"), egui::Sense::click()).clicked() {
        warning(ui, crate::localization::tr("带 fileID 的属性暂不开放编辑。修改可能破坏本文件内部对象索引、Prefab 关联或外部资源引用。请在 Unity 编辑器中修改引用。"));
    }
}
fn normalized(name: &str) -> String { name.trim_start_matches("m_").chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect() }
fn name_matches(key: &str, name: &str) -> bool {
    normalized(key) == normalized(name) || matches!((key, name), ("m_Scale", "Tiling") | ("m_CustomRenderQueue", "Render Queue") | ("m_EnableInstancingVariants", "Enable GPU Instancing")
        | ("m_SpriteTileMode", "Tile Mode") | ("m_AdaptiveModeThreshold", "Adaptive Threshold") | ("m_SpriteSortPoint", "Sort Point")
        | ("m_SortingLayer", "Sorting Layer Index") | ("m_SortingOrder", "Order in Layer") | ("m_DynamicOccludee", "Dynamic Occlusion")
        | ("m_LightProbeVolumeOverride", "Light Probe Volume") | ("m_AllowConstantClipSamplingOptimization", "Constant Clip Sampling Optimization")
        | ("m_KeepAnimatorStateOnDisable" | "m_KeepAnimatorControllerStateOnDisable", "Keep State On Disable"))
}
pub fn binding(ui: &egui::Ui, name: &str, raw: &str) -> Option<Binding> {
    let source = source(ui)?;
    let session = source.session.lock().unwrap();
    let text = &session.original[source.range.clone()];
    let mut offset = source.range.start;
    let mut named = Vec::new();
    let mut fallback = Vec::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim().trim_start_matches("- ");
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim();
            if value == raw.trim() {
                let value_start = line.find(':')? + 1;
                let content = line.trim_end_matches(['\r', '\n']);
                let start = offset + value_start + content[value_start..].len() - content[value_start..].trim_start().len();
                let range = start..start + value.len();
                if !value.is_empty() { fallback.push(range.clone()); }
                if name_matches(key, name) { named.push(range); }
            }
        }
        if name == "值" && line.trim().strip_prefix("- ").is_some_and(|value| value == raw.trim()) {
            let start = offset + line.find("- ")? + 2;
            named.push(start..start + raw.trim().len());
        }
        if raw.trim().starts_with('{') && line.trim().trim_start_matches("- ").trim_start_matches("- ") == raw.trim() {
            let start = offset + line.find('{')?;
            named.push(start..start + raw.trim().len());
        }
        offset += line.len();
    }
    named.sort_by_key(|range| range.start); named.dedup();
    fallback.sort_by_key(|range| range.start); fallback.dedup();
    let range = if named.len() == 1 { named.pop()? } else if named.is_empty() && fallback.len() == 1 { fallback.pop()? } else { return None };
    drop(session);
    Some(Binding { source, range })
}
pub fn with_field<R>(ui: &mut egui::Ui, name: &str, callback: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let previous = source(ui);
    if let Some(source) = &previous {
        let session = source.session.lock().unwrap();
        let text = &session.original[source.range.clone()];
        let mut offset = source.range.start;
        let mut candidates = Vec::new();
        let lines: Vec<_> = text.split_inclusive('\n').collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim().trim_start_matches("- ");
            if trimmed.split_once(':').is_some_and(|(key, _)| name_matches(key, name)) {
                let indent = line.len() - line.trim_start().len();
                let sequence_item = line.trim_start().starts_with("- ");
                let mut end = offset + line.len();
                for next in &lines[index + 1..] {
                    let depth = next.len() - next.trim_start().len();
                    if !next.trim().is_empty() && depth <= indent && (sequence_item || !(depth == indent && next.trim_start().starts_with("- "))) { break; }
                    end += next.len();
                }
                candidates.push(offset..end);
            }
            offset += line.len();
        }
        if candidates.len() == 1 { ui.ctx().data_mut(|data| data.insert_temp(id(), Source { session: source.session.clone(), range: candidates.remove(0) })); }
    }
    let result = callback(ui);
    ui.ctx().data_mut(|data| { data.remove::<Source>(id()); if let Some(previous) = previous { data.insert_temp(id(), previous); } });
    result
}
pub fn with_row<R>(ui: &mut egui::Ui, index: usize, callback: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let previous = source(ui);
    if let Some(source) = &previous {
        let session = source.session.lock().unwrap();
        let mut offset = source.range.start;
        let mut starts = Vec::new();
        let mut depth = None;
        for line in session.original[source.range.clone()].split_inclusive('\n') {
            let indent = line.len() - line.trim_start().len();
            if line.trim_start().starts_with("- ") {
                let shallowest = *depth.get_or_insert(indent);
                if indent == shallowest { starts.push(offset); }
            }
            offset += line.len();
        }
        if let Some(start) = starts.get(index) {
            let end = starts.get(index + 1).copied().unwrap_or(source.range.end);
            ui.ctx().data_mut(|data| data.insert_temp(id(), Source { session: source.session.clone(), range: *start..end }));
        }
    }
    let result = callback(ui);
    ui.ctx().data_mut(|data| { data.remove::<Source>(id()); if let Some(previous) = previous { data.insert_temp(id(), previous); } });
    result
}
pub fn with_flow_value<R>(ui: &mut egui::Ui, index: usize, callback: impl FnOnce(&mut egui::Ui, &str) -> R) -> Option<R> {
    let previous = source(ui)?;
    let (range, raw) = {
        let session = previous.session.lock().unwrap();
        let mut offset = previous.range.start;
        let mut entries = Vec::new();
        for line in session.original[previous.range.clone()].split_inclusive('\n') {
            if line.trim_start().starts_with("- ") {
                if let (Some(start), Some(end)) = (line.find('{'), line.rfind('}')) {
                    entries.push((offset + start..offset + end + 1, line[start..=end].to_string()));
                }
            }
            offset += line.len();
        }
        entries.get(index)?.clone()
    };
    ui.ctx().data_mut(|data| data.insert_temp(id(), Source { session: previous.session.clone(), range }));
    let output = callback(ui, &raw);
    ui.ctx().data_mut(|data| data.insert_temp(id(), previous));
    Some(output)
}
impl Binding {
    pub fn value(&self) -> String {
        let session = self.source.session.lock().unwrap();
        session.patches.get(&self.range.start).map(|(_, value)| value.clone()).unwrap_or_else(|| session.original[self.range.clone()].into())
    }
    pub fn dirty(&self) -> bool { self.source.session.lock().unwrap().has_changes_in(self.range.clone()) }
    fn apply(&self, value: String) { self.source.session.lock().unwrap().change(self.range.clone(), value); }
    fn apply_color(&self, ui: &egui::Ui, color: [f64; 4]) {
        if !color.iter().all(|value| value.is_finite() && value.abs() <= f32::MAX as f64) { warning(ui, crate::localization::tr("颜色分量必须为有效浮点数，已恢复之前的值。")); return; }
        let continuing = ui.input(|input| input.pointer.any_down() && !input.pointer.any_pressed());
        let value = float_components(&self.value(), &["r", "g", "b", "a"], &color);
        self.source.session.lock().unwrap().change_grouped(self.range.clone(), value, continuing);
    }
    pub fn restore(&self) {
        let original = { let mut session = self.source.session.lock().unwrap(); session.drafts.remove(&self.range.start); session.component_drafts.retain(|(start, _), _| *start != self.range.start); (session.original == session.saved_original).then(|| session.saved_patches.get(&self.range.start).map(|(_, value)| value.clone())).flatten().unwrap_or_else(|| session.original[self.range.clone()].to_string()) };
        self.apply(original);
    }
}
pub fn dirty_scope(ui: &egui::Ui) -> bool {
    source(ui).is_some_and(|source| source.session.lock().unwrap().has_changes_in(source.range))
}
pub fn highlight(ui: &egui::Ui, shape: egui::layers::ShapeIdx, rect: egui::Rect, dirty: bool) {
    if dirty { ui.painter().set(shape, egui::Shape::rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(240, 170, 30, 55))); }
}
pub fn placeholder(ui: &egui::Ui) -> egui::layers::ShapeIdx { ui.painter().add(egui::Shape::Noop) }
pub fn restore_scope(ui: &egui::Ui) {
    if let Some(source) = source(ui) {
        let mut session = source.session.lock().unwrap();
        let previous = session.patches.clone();
        session.patches.retain(|start, _| !source.range.contains(start));
        let saved: Vec<_> = session.saved_patches.iter().filter(|(start, _)| session.original == session.saved_original && source.range.contains(start)).map(|(start, value)| (*start, value.clone())).collect();
        session.patches.extend(saved);
        if session.patches != previous { session.record_undo(previous); }
        session.drafts.retain(|start, _| !source.range.contains(start));
        session.component_drafts.retain(|(start, _), _| !source.range.contains(start));
    }
}

pub fn validate(kind: &str, value: &str) -> bool {
    match kind.rsplit('.').next().unwrap_or(kind) {
        "int" | "Int32" => value.parse::<i32>().is_ok(), "uint" | "UInt32" => value.parse::<u32>().is_ok(),
        "long" | "Int64" => value.parse::<i64>().is_ok(), "ulong" | "UInt64" => value.parse::<u64>().is_ok(),
        "short" | "Int16" => value.parse::<i16>().is_ok(), "ushort" | "UInt16" => value.parse::<u16>().is_ok(),
        "byte" | "Byte" => value.parse::<u8>().is_ok(), "sbyte" | "SByte" => value.parse::<i8>().is_ok(),
        "float" | "Single" => value.parse::<f32>().is_ok_and(f32::is_finite),
        "double" | "Double" => value.parse::<f64>().is_ok_and(f64::is_finite),
        "string" | "String" => true,
        "char" | "Char" => value.chars().count() == 1 && value.chars().all(|c| c as u32 <= 0xffff),
        _ => false,
    }
}

fn float_components(current: &str, axes: &[&str], values: &[f64]) -> String {
    let entries: BTreeMap<_, _> = current.trim().trim_start_matches('{').trim_end_matches('}')
        .split(',').filter_map(|part| part.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim())).collect();
    let components = axes.iter().zip(values).map(|(axis, value)| {
        let value = *value as f32;
        let text = entries.get(axis).filter(|old| old.parse::<f32>().ok()
            .is_some_and(|old| old.to_bits() == value.to_bits()))
            .map(|old| old.to_string()).unwrap_or_else(|| value.to_string());
        format!("{axis}: {text}")
    }).collect::<Vec<_>>().join(", ");
    format!("{{{components}}}")
}
fn quoted(value: &str, original: &str) -> String {
    // Preserve Unity's plain scalar style, including numeric-looking names.
    if !original.starts_with(['\'', '"']) && plain_string(value) { return value.to_string(); }
    if original.starts_with('\'') && !value.chars().any(char::is_control) { return format!("'{}'", value.replace('\'', "''")); }
    let unicode_style = original.contains("\\u") || original.contains("\\U");
    let mut result = String::from("\"");
    for c in value.chars() {
        match c { '"' => result.push_str("\\\""), '\\' => result.push_str("\\\\"), '\n' => result.push_str("\\n"), '\r' => result.push_str("\\r"), '\t' => result.push_str("\\t"),
            c if c.is_control() || (unicode_style && !c.is_ascii()) => {
                if c as u32 <= 0xffff { result.push_str(&format!("\\u{:04X}", c as u32)); }
                else { result.push_str(&format!("\\U{:08X}", c as u32)); }
            }, c => result.push(c) }
    }
    result.push('"'); result
}
fn plain_string(value: &str) -> bool {
    if value.is_empty() { return true; }
    value.trim() == value
        && !value.chars().any(char::is_control)
        && !value.starts_with(['-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`'])
        && !value.contains(": ") && !value.ends_with(':')
        && !value.contains(" #")
}
#[derive(Clone)]
struct NumberDrag { start: f64, pixels: f64 }
fn integer_kind(kind: &str) -> bool {
    matches!(kind.rsplit('.').next().unwrap_or(kind), "int" | "Int32" | "uint" | "UInt32" | "long" | "Int64" | "ulong" | "UInt64" | "short" | "Int16" | "ushort" | "UInt16" | "byte" | "Byte" | "sbyte" | "SByte")
}
/// Scrub from the component label; text boxes keep native text selection.
fn scrub_number(ui: &egui::Ui, response: &egui::Response, input: &mut String, kind: &str) -> bool {
    let drag_id = response.id.with("number_scrub");
    let mut drag = ui.ctx().data(|data| data.get_temp::<NumberDrag>(drag_id));
    if response.drag_started_by(egui::PointerButton::Primary) && validate(kind, input) {
        if let Ok(start) = input.parse::<f64>() {
            // Larger integral values cannot be represented exactly by f64.
            if !integer_kind(kind) || start.abs() <= 9_007_199_254_740_991.0 { drag = Some(NumberDrag { start, pixels: 0.0 }); }
        }
    }
    let Some(mut drag) = drag else { return false };
    if !ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().data_mut(|data| data.remove::<NumberDrag>(drag_id));
        return false;
    }
    let multiplier = ui.input(|i| if i.modifiers.shift { 0.1 } else if i.modifiers.ctrl { 10.0 } else { 1.0 });
    drag.pixels += ui.input(|i|i.pointer.delta().x) as f64 * multiplier;
    let value = drag.start + drag.pixels * if integer_kind(kind) { 1.0 } else { 0.1 };
    let candidate = if integer_kind(kind) { format!("{:.0}", value.round()) }
        else if matches!(kind, "double" | "Double") { ((value * 1e6).round() / 1e6).to_string() }
        else { (((value * 1e6).round() / 1e6) as f32).to_string() };
    if validate(kind, &candidate) { *input = candidate; }
    ui.ctx().data_mut(|data| data.insert_temp(drag_id, drag));
    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    true
}
pub fn text(ui: &mut egui::Ui, bind: &Binding, kind: &str, width: f32) {
    text_inner(ui, bind, kind, width, false);
}
fn text_inner(ui: &mut egui::Ui, bind: &Binding, kind: &str, width: f32, exact_width: bool) {
    let current = bind.value();
    let is_string = matches!(kind.rsplit('.').next().unwrap_or(kind), "string" | "String" | "char" | "Char");
    let displayed = if is_string { crate::yaml_strings::display(&current).into_owned() } else { current.clone() };
    let mut draft = bind.source.session.lock().unwrap().drafts.get(&bind.range.start).cloned().unwrap_or(displayed.clone());
    let editor_id = ui.id().with(("edit_value", bind.range.start));
    let editor = TextEdit::singleline(&mut draft).id(editor_id).desired_width(width);
    let response = if exact_width {
        ui.add_sized(egui::vec2(width, ui.spacing().interact_size.y), editor)
    } else { ui.add(editor) };
    let scrubbing = false;
    if scrubbing {
        let mut session = bind.source.session.lock().unwrap();
        session.drafts.remove(&bind.range.start);
        session.change_grouped(bind.range.clone(), draft.clone(), true);
    } else if response.changed() {
        let mut session = bind.source.session.lock().unwrap();
        if draft == displayed { session.drafts.remove(&bind.range.start); } else { session.drafts.insert(bind.range.start, draft.clone()); }
        ui.ctx().request_repaint();
    }
    if !scrubbing && (response.lost_focus() || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))) {
        bind.source.session.lock().unwrap().drafts.remove(&bind.range.start);
        if draft != displayed {
            if validate(kind, &draft) {
                let state_name = if is_string {
                    let session = bind.source.session.lock().unwrap();
                    let prefix = &session.original[..bind.range.start];
                    prefix.lines().last().is_some_and(|line| line.trim() == "m_Name:")
                        && prefix.rsplit("---").next().is_some_and(|document| document.starts_with(" !u!1102 "))
                } else { false };
                bind.apply(if is_string { quoted(&draft, if state_name { "" } else { &current }) } else { draft });
            }
            else { warning(ui, crate::localization::tr("输入值与属性类型不符或超出范围，已恢复为之前的有效值。")); }
        }
    }
}
pub fn range_control(ui: &mut egui::Ui, name: &str, raw: &str, kind: &str, (min, max): (f64, f64)) {
    let bind = active(ui).then(|| binding(ui, name, raw)).flatten();
    let current = bind.as_ref().map(Binding::value).unwrap_or_else(|| raw.to_string());
    let Ok(mut value) = current.parse::<f64>() else { ui.label(raw); return; };
    if !value.is_finite() { ui.label(raw); return; }
    let integer = matches!(kind.rsplit('.').next().unwrap_or(kind), "int" | "Int32");
    let (min, max) = if integer { (min.ceil(), max.floor()) } else { (min, max) };
    if min > max { ui.label(current); return; }
    let mut slider = egui::Slider::new(&mut value, min..=max)
        .clamping(egui::SliderClamping::Edits);
    if integer { slider = slider.integer(); }
    let response = crate::theme::slider(ui, bind.is_some(), slider).on_hover_text(format!("Range({min}, {max})"));
    if response.changed() {
        let text = if integer { format!("{value:.0}") } else { (value as f32).to_string() };
        if validate(kind, &text) {
            if let Some(bind) = &bind {
                let continuing = ui.input(|input| input.pointer.any_down() && !input.pointer.any_pressed());
                bind.source.session.lock().unwrap().change_grouped(bind.range.clone(), text, continuing);
            }
        } else { warning(ui, crate::localization::tr("滑条数值超出属性类型范围，已恢复之前的值。")); }
    }
    if let Some(bind) = bind.filter(Binding::dirty) {
        if ui.small_button(crate::localization::tr("还原")).clicked() { bind.restore(); }
    }
}

pub fn table_control(ui: &mut egui::Ui, name: &str, raw: &str, kind: &str, enums: Option<&[(String, i64)]>) -> bool {
    let scalar = kind.rsplit('.').next().unwrap_or(kind);
    if active(ui) && enums.is_none() && !raw.contains("fileID:") {
        if let Some(bind) = binding(ui, name, raw) {
            let value = bind.value();
            if validate(scalar, if matches!(scalar, "string" | "String" | "char" | "Char") { "x" } else { &value })
                && !matches!(scalar, "bool" | "Boolean") {
                let path = bind.source.session.lock().unwrap().path.clone();
                ui.push_id(path, |ui| ui.horizontal(|ui| {
                    let button = egui::Button::new(crate::localization::tr("还原")).small();
                    let button_width = ui.painter().layout_no_wrap(crate::localization::tr("还原").into(),
                        egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color()).size().x
                        + 2.0 * ui.spacing().button_padding.x;
                    let width = (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(0.0);
                    text_inner(ui, &bind, kind, width, true);
                    if bind.dirty() && ui.add_sized(egui::vec2(button_width, ui.spacing().interact_size.y), button).clicked() {
                        bind.restore();
                    }
                }));
                return true;
            }
        }
    }
    control(ui, name, raw, kind, enums)
}

pub fn control(ui: &mut egui::Ui, name: &str, raw: &str, kind: &str, enums: Option<&[(String, i64)]>) -> bool {
    if !active(ui) || raw.contains("fileID:") { return false; }
    let path = source(ui).unwrap().session.lock().unwrap().path.clone();
    ui.push_id(path, |ui| ui.horizontal(|ui| {
        let shown = control_inner(ui, name, raw, kind, enums);
        if shown { if let Some(bind) = binding(ui, name, raw).filter(Binding::dirty) {
            if ui.small_button(crate::localization::tr("还原")).clicked() { bind.restore(); }
        } }
        shown
    }).inner).inner
}
fn control_inner(ui: &mut egui::Ui, name: &str, raw: &str, kind: &str, enums: Option<&[(String, i64)]>) -> bool {
    if !active(ui) { return false; }
    if raw.contains("fileID:") { return false; }
    let Some(bind) = binding(ui, name, raw) else { return false };
    let value = bind.value();
    let kind = kind.rsplit('.').next().unwrap_or(kind);
    if matches!(kind, "bool" | "Boolean") {
        let mut checked = match value.as_str() { "1" | "true" => true, "0" | "false" => false, _ => return false };
        if ui.add(egui::Checkbox::without_text(&mut checked)).changed() { bind.apply(if matches!(value.as_str(), "true" | "false") { checked.to_string() } else { (checked as u8).to_string() }); }
        return true;
    }
    if let Some(enums) = enums {
        let mut number = value.parse::<i64>().ok();
        let original = number;
        egui::ComboBox::from_id_salt(("edit_enum", bind.range.start)).width(200.0)
            .selected_text(enums.iter().find(|(_, n)| Some(*n) == number).map(|(name, n)| format!("{name} ({n})")).unwrap_or(value))
            .show_ui(ui, |ui| { for (name, n) in enums { ui.selectable_value(&mut number, Some(*n), format!("{name} ({n})")); } });
        if number != original { if let Some(number) = number { bind.apply(number.to_string()); } }
        return true;
    }
    let axes: Option<&[&str]> = match kind { "Color" | "MaterialVector4" => Some(&["r", "g", "b", "a"]), "Vector2" | "Vector2Int" => Some(&["x", "y"]), "Vector3" | "Vector3Int" | "Euler" => Some(&["x", "y", "z"]), "Vector4" | "Quaternion" => Some(&["x", "y", "z", "w"]), _ => None };
    if let Some(axes) = axes {
        let Some(mut values) = (if kind == "Euler" { crate::inline_vector(&value, &["x", "y", "z", "w"]).and_then(|q| crate::rotation_euler_degrees(&q)) } else { crate::inline_vector(&value, axes) }) else { return false };
        if kind == "Color" {
            if let Some(color) = crate::color_property::show(ui, name, [values[0], values[1], values[2], values[3]]) {
                bind.apply_color(ui, color);
            }
            return true;
        }
        let mut changed = false;
        let mut scrubbing_vector = false;
        let width = ((ui.available_width() - axes.len() as f32 * 20.0 - 50.0) / axes.len() as f32).clamp(48.0, 96.0);
        for (component, (axis, v)) in axes.iter().zip(&mut values).enumerate() {
            let label=ui.add(egui::Label::new(if kind == "MaterialVector4" { ["x", "y", "z", "w"][component] } else { *axis }).sense(egui::Sense::drag()))
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                .on_hover_text("按住左键左右拖动调整数值；Shift 精调，Ctrl 加速");
            let mut input = v.to_string();
            let draft_id = ui.id().with(("vector_draft", bind.range.start, axis));
            if let Some(draft) = bind.source.session.lock().unwrap().component_drafts.get(&(bind.range.start, component)).cloned() { input = draft; }
            let scalar_kind = if kind.ends_with("Int") { "int" } else { "float" };
            let scrubbing = scrub_number(ui, &label, &mut input, scalar_kind);
            let response = ui.add(TextEdit::singleline(&mut input).id(draft_id).desired_width(width));
            if scrubbing {
                bind.source.session.lock().unwrap().component_drafts.remove(&(bind.range.start, component));
                if let Ok(next) = input.parse::<f64>() { changed |= next != *v; *v = next; }
                scrubbing_vector = true;
            } else if response.changed() { bind.source.session.lock().unwrap().component_drafts.insert((bind.range.start, component), input.clone()); }
            if !scrubbing && (response.lost_focus() || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))) {
                bind.source.session.lock().unwrap().component_drafts.remove(&(bind.range.start, component));
                if validate(if kind.ends_with("Int") { "int" } else { "float" }, &input) {
                    let next = input.parse::<f64>().unwrap(); changed |= next != *v; *v = next;
                } else { warning(ui, crate::localization::tr("向量分量必须为有效数值且不能越界，已恢复之前的值。")); }
            }
        }
        if changed {
            let (axes, values) = if kind == "Euler" { (&["x", "y", "z", "w"][..], euler_quaternion(&values).to_vec()) } else { (axes, values) };
            if kind == "Quaternion" && values.iter().all(|v| *v == 0.0) { warning(ui, crate::localization::tr("四元数不能全部为零，已恢复之前的值。")); }
            else {
                let next = if kind.ends_with("Int") { format!("{{{}}}", axes.iter().zip(values).map(|(axis, v)| format!("{axis}: {v}")).collect::<Vec<_>>().join(", ")) }
                    else { float_components(&value, axes, &values) };
                if scrubbing_vector { bind.source.session.lock().unwrap().change_grouped(bind.range.clone(), next, true); }
                else { bind.apply(next); }
            }
        }
        return true;
    }
    if validate(kind, if matches!(kind, "string" | "String" | "char" | "Char") { "x" } else { &value }) { text(ui, &bind, kind, ui.available_width().min(240.0)); return true; }
    false
}
fn euler_quaternion(angles: &[f64]) -> [f64; 4] {
    let (sx, cx) = (angles[0].to_radians() * 0.5).sin_cos();
    let (sy, cy) = (angles[1].to_radians() * 0.5).sin_cos();
    let (sz, cz) = (angles[2].to_radians() * 0.5).sin_cos();
    [cy * sx * cz + sy * cx * sz, sy * cx * cz - cy * sx * sz,
        cy * cx * sz - sy * sx * cz, cy * cx * cz + sy * sx * sz]
}
pub fn packed_int_list(ui: &mut egui::Ui, name: &str, raw: &str, element: &str) -> bool {
    let unsigned = matches!(element.rsplit('.').next().unwrap_or(element), "uint" | "UInt32");
    if !unsigned && !matches!(element.rsplit('.').next().unwrap_or(element), "int" | "Int32") { return false; }
    if raw.is_empty() || raw.len() % 8 != 0 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) { return false; }
    let count = raw.len() / 8;
    egui::CollapsingHeader::new(crate::lf!("{0} ({1}[]) · {2} 条", name, element, count)).default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| { ui.add_sized([60.0, 24.0], egui::Label::new(crate::localization::tr("索引"))); ui.label(crate::localization::tr("值")); });
        let root = binding(ui, name, raw);
        egui::ScrollArea::vertical().id_salt(("packed_int_list", name)).max_height(count.min(11) as f32 * 24.0 + if count > 11 { 12.0 } else { 0.0 }).auto_shrink([false, true]).show(ui, |ui| {
            for index in 0..count {
                ui.push_id(index, |ui| {
                    let bind = root.as_ref().map(|root| Binding { source: root.source.clone(), range: root.range.start + index * 8..root.range.start + (index + 1) * 8 });
                    let raw = bind.as_ref().map(Binding::value).unwrap_or_else(|| raw[index * 8..(index + 1) * 8].into());
                    let bytes: Vec<_> = (0..8).step_by(2).map(|position| u8::from_str_radix(&raw[position..position + 2], 16).unwrap()).collect();
                    let number = u32::from_le_bytes(bytes.try_into().unwrap());
                    let shown = if unsigned { number.to_string() } else { (number as i32).to_string() };
                    let shape = placeholder(ui);
                    let response = ui.horizontal(|ui| {
                        ui.add_sized([60.0, 24.0], egui::Label::new(index.to_string()));
                        if let Some(bind) = &bind {
                            let session = bind.source.session.lock().unwrap();
                            let revision = session.revision;
                            let mut draft = session.drafts.get(&bind.range.start).cloned().unwrap_or(shown.clone());
                            drop(session);
                            let response = ui.add_sized([200.0, 24.0], TextEdit::singleline(&mut draft).id(ui.id().with(("packed_edit", bind.range.start, revision))));
                            if response.changed() { bind.source.session.lock().unwrap().drafts.insert(bind.range.start, draft.clone()); }
                            if response.lost_focus() || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))) {
                                bind.source.session.lock().unwrap().drafts.remove(&bind.range.start);
                                if validate(if unsigned { "uint" } else { "int" }, &draft) {
                                    let number = if unsigned { draft.parse::<u32>().unwrap() } else { draft.parse::<i32>().unwrap() as u32 };
                                    if draft != shown { bind.apply(number.to_le_bytes().iter().map(|byte| if raw.chars().any(|c| c.is_ascii_uppercase()) { format!("{byte:02X}") } else { format!("{byte:02x}") }).collect()); }
                                } else { warning(ui, crate::localization::tr("列表值与整数类型不符或越界，已恢复之前的值。")); }
                            }
                            if bind.dirty() && ui.small_button(crate::localization::tr("还原")).clicked() { bind.restore(); }
                        } else { let mut shown = shown; ui.add_sized([200.0, 24.0], TextEdit::singleline(&mut shown).interactive(false)); }
                    });
                    let mut rect = response.response.rect; rect.max.x = ui.max_rect().right();
                    highlight(ui, shape, rect, bind.as_ref().is_some_and(Binding::dirty));
                });
            }
        });
    });
    true
}
pub fn gradient_number(ui: &mut egui::Ui, key: &str, alpha: bool) -> bool {
    gradient_value(ui, key, alpha.then_some(3))
}
pub fn gradient_channel(ui: &mut egui::Ui, key: &str, channel: usize) -> bool { gradient_value(ui, key, Some(channel)) }
fn gradient_value(ui: &mut egui::Ui, key: &str, component: Option<usize>) -> bool {
    let Some(bind) = named_binding(ui, key) else { return false };
    let value = bind.value();
    let mut color = if component.is_some() { crate::inline_vector(&value, &["r", "g", "b", "a"]) } else { None };
    let shown = if let Some(component) = component { color.as_ref().map(|color| color[component]) } else { value.parse::<u16>().ok().map(|v| v as f64 / 65535.0 * 100.0) };
    let Some(shown) = shown else { return false };
    let session = bind.source.session.lock().unwrap();
    let revision = session.revision;
    let draft_key = (bind.range.start, component.unwrap_or(4));
    let mut draft = session.component_drafts.get(&draft_key).cloned().unwrap_or_else(|| shown.to_string());
    drop(session);
    let response = ui.add(TextEdit::singleline(&mut draft).id(ui.id().with(("gradient_number", key, component, revision))).desired_width(120.0));
    if response.changed() { bind.source.session.lock().unwrap().component_drafts.insert(draft_key, draft.clone()); }
    if response.lost_focus() || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))) {
        bind.source.session.lock().unwrap().component_drafts.remove(&draft_key);
        if let Some(next) = draft.parse::<f64>().ok().filter(|next| next.is_finite() && match component { Some(3) => (0.0..=1.0).contains(next), Some(_) => next.abs() <= f32::MAX as f64, None => (0.0..=100.0).contains(next) }) {
            if next != shown {
                if let Some(color) = color.as_mut() {
                    color[component.unwrap()] = next;
                    bind.apply(float_components(&bind.value(), &["r", "g", "b", "a"], color));
                } else { bind.apply(((next / 100.0 * 65535.0).round() as u16).to_string()); }
            }
        } else { warning(ui, match component { Some(3) => crate::localization::tr("Alpha 必须为 0～1，已恢复之前的值。"), Some(_) => crate::localization::tr("颜色分量必须为有限浮点数，已恢复之前的值。"), None => crate::localization::tr("渐变位置必须为 0～100%，已恢复之前的值。") }); }
    }
    true
}
pub fn gradient_color(ui: &mut egui::Ui, key: &str) -> bool {
    let Some(bind) = named_binding(ui, key) else { return false };
    let value = bind.value();
    let Some(values) = crate::inline_vector(&value, &["r", "g", "b", "a"]) else { return false };
    if let Some(color) = crate::color_property::show(ui, key, [values[0], values[1], values[2], values[3]]) {
        bind.apply_color(ui, color);
    }
    true
}

pub fn save(session: &Session) -> Result<String, String> {
    use std::io::Write;
    if session.warning.is_some() { return Err(crate::localization::tr("存在输入验证警告，请确认已回退的值后再保存。").into()); }
    if session.has_drafts() { return Err(crate::localization::tr("仍有输入正在编辑，请按 Enter 或点击其他位置确认输入后再保存。").into()); }
    let current = std::fs::read(&session.path).map_err(|error| error.to_string())?;
    if current != session.disk { return Err(crate::localization::tr("文件已被其他程序修改，未覆盖磁盘文件。请放弃编辑并重新打开文件。").into()); }
    let text = session.rendered();
    if crate::parse_unity_yaml(&text).iter().map(|doc| (&doc.file_id, &doc.type_name)).collect::<Vec<_>>() != crate::parse_unity_yaml(&session.original).iter().map(|doc| (&doc.file_id, &doc.type_name)).collect::<Vec<_>>() {
        return Err(crate::localization::tr("保存验证失败：对象索引或类型发生变化。").into());
    }
    let temporary = session.path.with_file_name(format!(".{}.unity-yaml-edit-{}.tmp", session.path.file_name().unwrap().to_string_lossy(), std::process::id()));
    let mut created = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|error| error.to_string())?;
        created = true;
        if session.disk.starts_with(&[0xef, 0xbb, 0xbf]) && !text.starts_with('\u{feff}') { file.write_all(&[0xef, 0xbb, 0xbf]).map_err(|error| error.to_string())?; }
        file.write_all(text.as_bytes()).and_then(|_| file.sync_all()).map_err(|error| error.to_string())?;
        drop(file);
        if std::fs::read(&session.path).map_err(|error| error.to_string())? != session.disk { return Err(crate::localization::tr("保存前检测到文件发生变化，已取消保存。").into()); }
        std::fs::rename(&temporary, &session.path).map_err(|error| crate::lf!("无法写入文件：{0}", error))?;
        Ok(text)
    })();
    if result.is_err() && created { let _ = std::fs::remove_file(&temporary); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_batch_drag_uses_one_undo_and_preserves_values() {
        let raw = "time: 0\r\nvalue: 1\r\ntime: 0.5\r\nvalue: 1\r\n";
        let mut session = Session::new(PathBuf::from("clip.anim"), raw.into(), raw.as_bytes().to_vec());
        let second = raw.find("0.5").unwrap();
        assert!(session.change_many(vec![(6..7, "0.1".into()), (second..second+3, "0.6".into())], true));
        assert!(session.change_many(vec![(6..7, "0.2".into()), (second..second+3, "0.7".into())], true));
        assert_eq!(session.undo.len(), 1);
        assert_eq!(session.rendered(), "time: 0.2\r\nvalue: 1\r\ntime: 0.7\r\nvalue: 1\r\n");
        session.undo();
        assert_eq!(session.rendered(), raw);
        session.drafts.insert(6, "typing".into());
        assert!(!session.change_many(vec![(6..7, "0.3".into())], true));
        assert_eq!(session.rendered(), raw);
    }
    #[test]
    fn special_controller_nodes_move_only_their_own_position_and_group_undo() {
        let raw = "--- !u!1107 &10\nAnimatorStateMachine:\n  m_AnyStatePosition: {x: 10, y: 20, z: 3}\n  m_EntryPosition: {x: 10, y: 20, z: 4}\n  m_ExitPosition: {x: 10, y: 20, z: 5}\n  m_DefaultState: {fileID: 0}\n";
        for key in ["m_AnyStatePosition", "m_EntryPosition", "m_ExitPosition"] {
            in_ui(raw, |ui, edit| {
                drag_controller_node(ui, "10", "", key, egui::vec2(10.0, -5.0), false);
                drag_controller_node(ui, "10", "", key, egui::vec2(2.0, 1.0), true);
                let changed = edit.lock().unwrap().rendered();
                let expected = raw.lines().find(|line| line.trim_start().starts_with(key)).unwrap()
                    .replace("x: 10, y: 20", "x: 22, y: 16");
                assert_eq!(changed, raw.replace(raw.lines().find(|line| line.trim_start().starts_with(key)).unwrap(), &expected));
                edit.lock().unwrap().undo();
                assert_eq!(edit.lock().unwrap().rendered(), raw);
            });
        }
    }
    #[test]
    fn state_name_draft_previews_without_committing_or_changing_undo_history() {
        let raw = "--- !u!1102 &20\nAnimatorState:\n  m_Name: 3\n  m_Speed: 1\n";
        in_ui(raw, |ui, edit| {
            let start = raw.find("m_Name: 3").unwrap() + "m_Name: ".len();
            edit.lock().unwrap().drafts.insert(start, "2".into());
            assert!(document_text(ui, "20").unwrap().contains("m_Name: 2\n"));
            assert_eq!(edit.lock().unwrap().rendered(), raw);
            assert!(edit.lock().unwrap().undo.is_empty());
            edit.lock().unwrap().undo();
            assert!(document_text(ui, "20").unwrap().contains("m_Name: 3\n"));
        });
    }
    #[test]
    fn plain_unity_strings_keep_numeric_names_and_quote_yaml_delimiters() {
        assert_eq!(quoted("0", "New State"), "0");
        assert_eq!(quoted("2", "0"), "2");
        assert_eq!(quoted("Idle State", "New State"), "Idle State");
        assert_eq!(quoted("", "New State"), "");
        for value in ["a: b", "a # comment", "{value}", " leading", "line\nbreak"] {
            let serialized = quoted(value, "New State");
            assert!(serialized.starts_with('"'));
            assert_eq!(crate::yaml_strings::display(&serialized), value);
        }
        assert_eq!(quoted("2", "\"0\""), "\"2\"");
    }
    #[test]
    fn creates_empty_states_with_unique_ids_and_atomic_undo() {
        for children in ["  m_ChildStates: []\n", "  m_ChildStates:\n  - serializedVersion: 1\n    m_State: {fileID: 1102000000}\n    m_Position: {x: 1, y: 2, z: 0}\n"] {
            let raw = format!("%YAML 1.1\n--- !u!1107 &10\nAnimatorStateMachine:\n{children}  m_DefaultState: {{fileID: 0}}\n--- !u!1102 &1102000000\nAnimatorState:\n  m_Name: New State\n  m_Motion: {{fileID: 0}}\n");
            in_ui(&raw, |ui, edit| {
                let first = create_empty_controller_state(ui, "10", egui::pos2(123.0, 456.0)).unwrap();
                assert_eq!(first, "1102000001");
                let created = edit.lock().unwrap().rendered();
                let documents = crate::parse_unity_yaml(&created);
                assert_eq!(documents.len(), 3);
                assert!(documents[0].text.contains("m_Position: {x: 123, y: 456, z: 0}"));
                assert!(documents[2].text.contains("m_Name: New State 1"));
                assert!(documents[2].text.contains("m_Motion: {fileID: 0}"));
                set_controller_default(ui, "10", &first);
                edit.lock().unwrap().undo();
                assert_eq!(edit.lock().unwrap().rendered(), created);
                let second = create_empty_controller_state(ui, "10", egui::pos2(0.0, 0.0)).unwrap();
                assert_ne!(first, second);
                edit.lock().unwrap().undo();
                assert_eq!(edit.lock().unwrap().rendered(), created);
                edit.lock().unwrap().undo();
                assert_eq!(edit.lock().unwrap().rendered(), raw);
                assert!(!edit.lock().unwrap().has_changes());
            });
        }
    }
    #[test]
    fn controller_layout_and_default_changes_target_owner_and_undo_independently() {
        let raw = "%YAML 1.1\n--- !u!1107 &10\nAnimatorStateMachine:\n  m_ChildStates:\n  - serializedVersion: 1\n    m_State: {fileID: 20}\n    m_Position: {x: 100, y: 50, z: 7}\n  - serializedVersion: 1\n    m_State: {fileID: 21}\n    m_Position: {x: 100, y: 50, z: 7}\n  m_DefaultState: {fileID: 20}\n--- !u!1102 &20\nAnimatorState:\n  m_Position: {x: 0, y: 0, z: 0}\n";
        in_ui(raw, |ui, edit| {
            drag_controller_node(ui, "10", "21", "m_State", egui::vec2(10.0, -5.0), false);
            drag_controller_node(ui, "10", "21", "m_State", egui::vec2(2.0, -3.0), true);
            let moved = edit.lock().unwrap().rendered();
            assert!(moved.contains("m_Position: {x: 112, y: 42, z: 7}"));
            assert!(moved.contains("m_Position: {x: 100, y: 50, z: 7}"));
            assert!(moved.contains("m_Position: {x: 0, y: 0, z: 0}"));
            set_controller_default(ui, "10", "21");
            assert!(controller_machine_text(ui, "10").unwrap().contains("m_DefaultState: {fileID: 21}"));
            edit.lock().unwrap().undo();
            assert_eq!(edit.lock().unwrap().rendered(), moved);
            edit.lock().unwrap().undo();
            assert_eq!(edit.lock().unwrap().rendered(), raw);
        });
    }
    #[test]
    fn float_components_write_short_single_precision_and_preserve_other_channels() {
        let value = 0.10214877128601074_f64;
        let result = float_components("{r: 0, g: 0.93636364, b: 0.2500, a: 1}",
            &["r", "g", "b", "a"], &[value, 0.9363636374473572, 0.25, 1.0]);
        assert_eq!(result, "{r: 0.10214877, g: 0.93636364, b: 0.2500, a: 1}");
        assert_eq!("0.10214877".parse::<f32>().unwrap().to_bits(), (value as f32).to_bits());
    }
    #[test]
    fn saved_history_undo_and_restore_use_latest_saved_values() {
        let raw = "name: old\nvalue: 1\n";
        let mut edit = Session::new(PathBuf::from("test.asset"), raw.into(), raw.as_bytes().into());
        edit.change(6..9, "longer name".into());
        edit.change(17..18, "22".into());
        let saved = edit.rendered();
        edit.mark_saved(&saved);
        assert!(!edit.has_changes());
        assert!(!edit.has_changes_in(0..raw.len()));
        assert!(edit.can_undo());
        edit.undo();
        assert_eq!(edit.rendered(), "name: longer name\nvalue: 1\n");
        assert!(edit.has_changes_in(17..18));
        assert!(!edit.has_changes_in(6..9));
        let shared = Arc::new(Mutex::new(edit));
        Binding { source: Source { session: shared.clone(), range: 0..raw.len() }, range: 17..18 }.restore();
        assert_eq!(shared.lock().unwrap().rendered(), saved);
        assert!(!shared.lock().unwrap().has_changes());
        shared.lock().unwrap().undo();
        let next = shared.lock().unwrap().rendered();
        shared.lock().unwrap().mark_saved(&next);
        shared.lock().unwrap().undo();
        // Saving records a baseline, not an undo step. The restore was already
        // undone, so the next undo reverses the original name edit.
        assert_eq!(shared.lock().unwrap().rendered(), raw);
        assert!(shared.lock().unwrap().has_changes());
    }

    #[test]
    fn saved_history_keeps_bom_in_disk_baseline() {
        let mut edit = Session::new(PathBuf::from("test.asset"), "v: 1".into(), vec![0xef, 0xbb, 0xbf, b'v', b':', b' ', b'1']);
        edit.change(3..4, "2".into());
        edit.mark_saved("v: 2");
        assert_eq!(edit.disk, vec![0xef, 0xbb, 0xbf, b'v', b':', b' ', b'2']);
        edit.undo();
        assert!(edit.has_changes());
    }
    fn session(raw: &str) -> Shared { Arc::new(Mutex::new(Session::new(PathBuf::from("test.asset"), raw.into(), raw.as_bytes().into()))) }
    fn in_ui(raw: &str, callback: impl FnOnce(&mut egui::Ui, Shared)) {
        let ctx = egui::Context::default();
        let edit = session(raw);
        publish(&ctx, Some(edit.clone()), 0..raw.len());
        let mut callback = Some(callback);
        let _ = ctx.run(Default::default(), |ctx| { egui::CentralPanel::default().show(ctx, |ui| callback.take().unwrap()(ui, edit.clone())); });
    }
    #[test]
    fn validates_numeric_widths_and_string_escape_roundtrips() {
        for (kind, valid, invalid) in [("byte", "255", "256"), ("int", "-2147483648", "2147483648"), ("uint", "0", "-1"), ("float", "-0.25", "NaN"), ("double", "1e200", "inf"), ("char", "灰", "灰色")] {
            assert!(validate(kind, valid)); assert!(!validate(kind, invalid));
        }
        for original in ["\"\\u7070\"", "\"灰\"", "'灰'"] {
            let value = "灰色: \"引号\"\\路径\n下一行";
            assert_eq!(crate::yaml_strings::display(&quoted(value, original)), value);
        }
    }
    #[test]
    fn repeated_rows_bind_to_the_correct_original_bytes_and_restore_independently() {
        let raw = "--- !u!114 &1\r\nMonoBehaviour:\r\n  itemList:\r\n  - value1: 1\r\n    desc: \"\\u7070\"\r\n  - value1: 1\r\n    desc: \"\\u7070\"\r\n  enabled: 1\r\n  ref: {fileID: 8}\r\n";
        in_ui(raw, |ui, edit| {
            with_field(ui, "itemList", |ui| {
                with_row(ui, 1, |ui| {
                    let bind = binding(ui, "value1", "1").unwrap();
                    bind.apply("25".into());
                });
            });
            with_field(ui, "enabled", |ui| binding(ui, "enabled", "1").unwrap().apply("0".into()));
            let rendered = edit.lock().unwrap().rendered();
            assert!(rendered.contains("  - value1: 1\r\n"));
            assert!(rendered.contains("  - value1: 25\r\n"));
            assert!(rendered.contains("  enabled: 0\r\n  ref: {fileID: 8}"));
            with_field(ui, "itemList", |ui| with_row(ui, 1, |ui| restore_scope(ui)));
            assert!(edit.lock().unwrap().rendered().contains("  enabled: 0"));
            assert!(!edit.lock().unwrap().rendered().contains("value1: 25"));
            edit.lock().unwrap().undo();
            assert!(edit.lock().unwrap().rendered().contains("value1: 25"));
            assert!(edit.lock().unwrap().rendered().contains("  enabled: 0"));
            assert!(!control(ui, "ref", "{fileID: 8}", "string", None));
        });
    }
    #[test]
    fn undo_is_bounded_and_each_file_has_its_own_history() {
        let mut first = Session::new("a".into(), "0".into(), vec![b'0']);
        let second = Session::new("b".into(), "0".into(), vec![b'0']);
        for n in 1..=250 { first.change(0..1, n.to_string()); }
        assert_eq!(first.undo.len(), 200);
        for _ in 0..200 { first.undo(); }
        assert_eq!(first.rendered(), "50");
        assert_eq!(second.rendered(), "0");
        first.set_undo_limit(0); assert_eq!(first.undo_limit, 50);
        first.set_undo_limit(10000); assert_eq!(first.undo_limit, 1000);
        first.drafts.insert(0, "pending".into());
        first.change(0..1, "500".into()); first.undo();
        assert_eq!(first.rendered(), "500"); first.undo(); assert_eq!(first.rendered(), "50");
    }
    #[test]
    fn euler_edits_use_unity_zxy_order() {
        for angles in [[20.0, 30.0, -40.0], [0.0, 0.0, 0.0], [-50.0, 70.0, 10.0]] {
            let restored = crate::rotation_euler_degrees(&euler_quaternion(&angles)).unwrap();
            for (original, restored) in angles.iter().zip(restored) { assert!((original - restored).abs() < 1e-8); }
        }
    }
    #[test]
    fn save_preserves_utf8_bom_and_rejects_external_changes() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("unity-yaml-viewer-edit-test-{}.asset", std::process::id()));
        let raw = "--- !u!114 &1\r\nMonoBehaviour:\r\n  desc: \"灰\"\r\n  ref: {fileID: 8}\r\n";
        let disk = [vec![0xef, 0xbb, 0xbf], raw.as_bytes().to_vec()].concat();
        std::fs::write(&path, &disk).unwrap();
        let mut edit = Session::new(path.clone(), raw.into(), disk);
        let start = raw.find("\"灰\"").unwrap();
        edit.change(start..start + "\"灰\"".len(), "\"绿色\"".into());
        let rendered = save(&edit).unwrap();
        let saved = std::fs::read(&path).unwrap();
        assert!(saved.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(&saved[3..], rendered.as_bytes());
        assert!(rendered.contains("ref: {fileID: 8}\r\n"));
        std::fs::write(&path, b"external modification").unwrap();
        assert!(save(&edit).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"external modification");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn empty_strings_flow_points_and_texture_scopes_keep_exact_targets() {
        let raw = "--- !u!114 &1\r\nMonoBehaviour:\r\n  desc: \r\n  points:\r\n    - - {x: 1, y: 2}\r\n      - {x: 1, y: 2}\r\n  textures:\r\n    - _First:\r\n        m_Scale: {x: 1, y: 1}\r\n    - _Second:\r\n        m_Scale: {x: 1, y: 1}\r\n";
        in_ui(raw, |ui, edit| {
            with_field(ui, "desc", |ui| binding(ui, "desc", "").unwrap().apply("\"灰色\"".into()));
            with_field(ui, "points", |ui| with_flow_value(ui, 1, |ui, raw| binding(ui, "值", raw).unwrap().apply("{x: 5, y: 6}".into())));
            with_field(ui, "textures", |ui| with_field(ui, "_Second", |ui| binding(ui, "Tiling", "{x: 1, y: 1}").unwrap().apply("{x: 2, y: 2}".into())));
            let rendered = edit.lock().unwrap().rendered();
            assert!(rendered.contains("  desc: \"灰色\"\r\n  points:"));
            assert!(rendered.contains("- - {x: 1, y: 2}\r\n      - {x: 5, y: 6}"));
            assert!(rendered.contains("_First:\r\n        m_Scale: {x: 1, y: 1}"));
            assert!(rendered.contains("_Second:\r\n        m_Scale: {x: 2, y: 2}"));
        });
    }
    #[test]
    fn color_drag_coalesces_and_restore_keeps_unrelated_drafts() {
        let mut edit = Session::new("a".into(), "0 0".into(), b"0 0".to_vec());
        edit.change_grouped(0..1, "1".into(), false);
        for n in 2..100 { edit.change_grouped(0..1, n.to_string(), true); }
        assert_eq!(edit.undo.len(), 1);
        edit.undo(); assert_eq!(edit.rendered(), "0 0");
        in_ui("  a: 0\n  b: 0\n", |ui, session| {
            let a = binding(ui, "a", "0").unwrap(); let b = binding(ui, "b", "0").unwrap();
            a.apply("1".into()); b.apply("2".into());
            session.lock().unwrap().component_drafts.insert((b.range.start, 0), "3".into());
            a.restore();
            assert!(session.lock().unwrap().rendered().contains("b: 2"));
            assert!(session.lock().unwrap().component_drafts.contains_key(&(b.range.start, 0)));
        });
    }
    #[test]
    fn grouped_vector_edit_preview_uses_patches_and_keeps_original_offsets() {
        let raw = "--- !u!4 &10\nTransform:\n  m_LocalPosition: {x: 0, y: 2, z: 3}\n--- !u!1 &20\nGameObject:\n  m_Name: tree\n";
        in_ui(raw, |ui, shared| {
            let bind = binding(ui, "m_LocalPosition", "{x: 0, y: 2, z: 3}").unwrap();
            for x in [1, 10, 100] {
                shared.lock().unwrap().change_grouped(bind.range.clone(), format!("{{x: {x}, y: 2, z: 3}}"), true);
            }
            let boundary = raw.find("--- !u!1").unwrap();
            let preview = document_text_in_range(ui, 0..boundary).unwrap();
            assert!(preview.contains("{x: 100, y: 2, z: 3}"));
            assert!(!preview.contains("GameObject"));
            assert!(document_text_in_range(ui, boundary..raw.len()).unwrap().contains("m_Name: tree"));
            let mut session = shared.lock().unwrap();
            assert_eq!(session.undo.len(), 1);
            assert_eq!(session.original, raw);
            session.undo();
            assert_eq!(session.rendered(), raw);
        });
    }
    #[test]
    fn editor_controls_render_with_synthetic_property_and_table_data() {
        let raw = "--- !u!212 &1\nSpriteRenderer:\n  m_Enabled: 1\n  m_Color: {r: 1, g: 0.5, b: 0, a: 1}\n  m_Size: {x: 2, y: 3}\n  m_DrawMode: 0\n  m_SpriteTileMode: 0\n  m_SpriteSortPoint: 0\n  m_SortingOrder: 1\n  m_Sprite: {fileID: 0}\n";
        in_ui(raw, |ui, _| crate::script_properties::show_sprite_renderer_properties_content(ui, raw, ""));
        in_ui("  values: 180000001c000000\n", |ui, _| assert!(packed_int_list(ui, "values", "180000001c000000", "int")));
        let raw = "--- !u!4 &1\nTransform:\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n";
        in_ui(raw, |ui, _| crate::show_transform_properties(ui, raw));
    }
}
