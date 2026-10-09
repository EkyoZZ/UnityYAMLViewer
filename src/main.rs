#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    sync::Arc,
};

use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit};
use encoding_rs::{GBK, UTF_8};
use regex::Regex;

mod script_properties;
mod color_property;
mod controller_graph;
mod gradient_property;
mod yaml_strings;
mod editing;
use script_properties::ScriptProperties;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct AppSettings { theme: String, undo_steps: usize }
impl Default for AppSettings {
    fn default() -> Self { Self { theme: "system".into(), undo_steps: editing::DEFAULT_UNDO_LIMIT } }
}
impl AppSettings {
    fn load() -> Self {
        let mut settings = Self::default();
        if let Some(path) = last_open_dir_config_path() {
            if let Ok(theme) = fs::read_to_string(path.with_file_name("theme.txt")) {
                if matches!(theme.trim(), "system" | "light" | "dark") { settings.theme = theme.trim().into(); }
            }
            if let Ok(steps) = fs::read_to_string(path.with_file_name("undo_steps.txt")) {
                if let Ok(steps) = steps.trim().parse::<usize>() { settings.undo_steps = steps.clamp(editing::MIN_UNDO_LIMIT, editing::MAX_UNDO_LIMIT); }
            }
        }
        settings
    }
    fn persist(&self) {
        if let Some(path) = last_open_dir_config_path().map(|path| path.with_file_name("undo_steps.txt")) {
            if let Some(parent) = path.parent() { if fs::create_dir_all(parent).is_err() { return; } }
            let _ = fs::write(path, self.undo_steps.to_string().as_bytes());
        }
    }
}

#[derive(Clone)]
struct Document {
    file_id: String,
    type_name: String,
    start_line: usize,
    text: String,
    game_object: Option<String>,
    parent_transform: Option<String>,
}

#[derive(Default)]
struct AnimationPathNode {
    children: BTreeMap<String, AnimationPathNode>,
    bindings: Vec<AnimationBinding>,
}

struct AnimationBinding {
    path: String,
    class_name: String,
    attribute: String,
    keyframes: Vec<AnimationKeyframe>,
}

struct AnimationKeyframe {
    time: f32,
    value: String,
    yaml_line: usize,
}

struct SelectedKeyframe {
    track_name: String,
    time: f32,
    value: String,
    yaml_line: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct AnimationTrackSelection {
    path: String,
    class_name: String,
    attribute: String,
}

struct YamlTable {
    title: String,
    headers: Vec<String>,
    rows: Vec<BTreeMap<String, String>>,
    column_types: BTreeMap<String, String>,
    column_enums: BTreeMap<String, Vec<(String, i64)>>,
}

#[derive(Default)]
struct ViewerApp {
    path: Option<PathBuf>,
    last_open_dir: Option<PathBuf>,
    raw: String,
    documents: Vec<Document>,
    selected: Option<usize>,
    locate_target: Option<usize>,
    last_selected: Option<usize>,
    query: String,
    status: String,
    encoding: String,
    selected_keyframe: Option<SelectedKeyframe>,
    selected_animation_track: Option<AnimationTrackSelection>,
    show_custom_table: bool,
    show_transform_yaml: bool,
    show_script_yaml: bool,
    show_sprite_renderer_yaml: bool,
    show_collider_yaml: bool,
    show_animator_yaml: bool,
    show_material_yaml: bool,
    script_properties: ScriptProperties,
    freeze_table_header: bool,
    table_column_widths: HashMap<String, Vec<f32>>,
    table_horizontal_offsets: HashMap<String, f32>,
    tabs: Vec<ViewerTab>,
    active_tab: usize,
    show_about: bool,
    restoring_session: bool,
    edits: HashMap<PathBuf, editing::Shared>,
    saved_histories: HashMap<PathBuf, editing::Shared>,
    pending_save: Option<PathBuf>,
    pending_close_tab: Option<PathBuf>,
    show_settings: bool,
    settings: AppSettings,
}

#[derive(Clone)]
struct ViewerTab {
    path: PathBuf,
    raw: String,
    documents: Vec<Document>,
    selected: Option<usize>,
    status: String,
    encoding: String,
}

impl ViewerApp {
    fn persist_open_tabs(&self) {
        if self.restoring_session { return; }
        let Some(config_path) = last_open_dir_config_path().map(|path| path.with_file_name("open_tabs.txt")) else { return };
        let Some(parent) = config_path.parent() else { return };
        if fs::create_dir_all(parent).is_err() { return; }
        let session = encode_open_tabs(&self.tabs.iter().map(|tab| tab.path.clone()).collect::<Vec<_>>(), self.path.as_deref());
        let temporary = config_path.with_extension("tmp");
        if fs::write(&temporary, session.as_bytes()).is_ok() {
            let _ = fs::rename(temporary, config_path);
        }
    }

    fn restore_open_tabs(&mut self) {
        let Some(config_path) = last_open_dir_config_path().map(|path| path.with_file_name("open_tabs.txt")) else { return };
        let Ok(session) = fs::read_to_string(config_path) else { return };
        let (paths, active) = decode_open_tabs(&session);
        self.restoring_session = true;
        for path in paths {
            if path.is_file() { self.open_file(path); }
        }
        if let Some(index) = active.and_then(|path| self.tabs.iter().position(|tab| tab.path == path)) {
            self.activate_tab(index);
        }
        self.restoring_session = false;
        self.persist_open_tabs();
    }

    fn open_external_reference(&mut self, path: &std::path::Path) {
        if !path.is_file() {
            self.status = format!("引用文件不存在：{}", path.display());
            return;
        }
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
        if matches!(extension.as_str(), "exe" | "com" | "bat" | "cmd" | "ps1" | "msi" | "lnk" | "vbs" | "js") {
            self.status = "此引用为可执行文件或脚本，不支持直接打开。".to_string();
            return;
        }
        match std::process::Command::new("explorer.exe").arg(path).spawn() {
            Ok(_) => self.status = format!("已交给系统默认程序打开：{}", path.display()),
            Err(error) => self.status = format!("无法打开引用：{error}"),
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        self.open_file_with_directory_tracking(path, true);
    }

    fn open_file_with_directory_tracking(&mut self, path: PathBuf, remember_directory: bool) {
        if remember_directory && !self.restoring_session {
            self.last_open_dir = path.parent().map(PathBuf::from);
            self.persist_last_open_dir();
        }
        if let Some(index) = self.tabs.iter().position(|tab| tab.path == path) {
            self.save_active_tab();
            self.activate_tab(index);
            return;
        }
        match fs::read(&path) {
            Ok(bytes) => {
                let (text, _, had_errors) = UTF_8.decode(&bytes);
                let (text, encoding) = if had_errors {
                    (
                        GBK.decode(&bytes).0.into_owned(),
                        "GBK（检测到无效 UTF-8 字节）",
                    )
                } else {
                    (text.into_owned(), "UTF-8")
                };
                let documents = parse_unity_yaml(&text);
                self.show_custom_table = documents
                    .iter()
                    .any(|document| yaml_uniform_table(&document.text).is_some());
                let tab = ViewerTab {
                    path,
                    raw: text,
                    status: format!("已读取 {} 个 Unity 序列化对象（只读模式）", documents.len()),
                    documents,
                    selected: None,
                    encoding: encoding.to_string(),
                };
                self.save_active_tab();
                self.tabs.push(tab);
                self.activate_tab(self.tabs.len() - 1);
            }
            Err(error) => self.status = format!("无法读取文件：{error}"),
        }
    }

    fn reload_current_file(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return; };
        if self.edits.contains_key(&path) { return; }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => { self.status = format!("重载失败：{error}"); return; }
        };
        let selected_id = self.selected.and_then(|index| self.documents.get(index))
            .map(|doc| doc.file_id.clone());
        let (text, _, errors) = UTF_8.decode(&bytes);
        let (text, encoding) = if errors {
            (GBK.decode(&bytes).0.into_owned(), "GBK（检测到无效 UTF-8 字节）")
        } else { (text.into_owned(), "UTF-8") };
        let documents = parse_unity_yaml(&text);
        self.selected = selected_id.and_then(|id| documents.iter().position(|doc| doc.file_id == id));
        self.last_selected = self.selected;
        self.locate_target = self.selected;
        self.show_custom_table = documents.iter().any(|doc| yaml_uniform_table(&doc.text).is_some());
        self.raw = text;
        self.documents = documents;
        self.encoding = encoding.into();
        self.selected_keyframe = None;
        self.selected_animation_track = None;
        self.saved_histories.remove(&path);
        self.pending_save = None;
        self.status = format!("已从磁盘重载 {} 个 Unity 序列化对象（只读模式），旧撤销记录已清空", self.documents.len());
        self.save_active_tab();
        editing::publish(ctx, None, 0..0);
    }

    fn persist_last_open_dir(&self) {
        let (Some(directory), Some(config_path)) =
            (&self.last_open_dir, last_open_dir_config_path())
        else {
            return;
        };
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(config_path, directory.to_string_lossy().as_bytes());
    }

    fn save_active_tab(&mut self) {
        if self.path.is_some() && self.active_tab < self.tabs.len() {
            self.tabs[self.active_tab] = ViewerTab {
                path: self.path.clone().unwrap(),
                raw: self.raw.clone(),
                documents: self.documents.clone(),
                selected: self.selected,
                status: self.status.clone(),
                encoding: self.encoding.clone(),
            };
        }
    }

    fn activate_tab(&mut self, index: usize) {
        self.locate_target = None;
        let tab = self.tabs[index].clone();
        self.active_tab = index;
        self.path = Some(tab.path);
        self.raw = tab.raw;
        self.documents = tab.documents;
        self.selected = tab.selected;
        self.last_selected = tab.selected;
        self.status = tab.status;
        self.encoding = tab.encoding;
        self.persist_open_tabs();
    }

    fn close_tab(&mut self, index: usize) {
        if let Some(session) = self.edits.get(&self.tabs[index].path) {
            if session.lock().unwrap().has_changes() {
                self.pending_close_tab = Some(self.tabs[index].path.clone());
                return;
            }
        }
        self.edits.remove(&self.tabs[index].path);
        self.saved_histories.remove(&self.tabs[index].path);
        self.save_active_tab();
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.path = None;
            self.raw.clear();
            self.documents.clear();
            self.selected = None;
            self.last_selected = None;
            self.status.clear();
            self.encoding.clear();
            self.active_tab = 0;
            self.locate_target = None;
            self.persist_open_tabs();
        } else {
            let next = if index < self.active_tab { self.active_tab - 1 }
                else { self.active_tab.min(self.tabs.len() - 1) };
            self.activate_tab(next);
        }
    }

    fn display_name(&self, doc: &Document) -> String {
        if doc.type_name == "GameObject" {
            name_from_text(&doc.text).unwrap_or_else(|| format!("GameObject #{}", doc.file_id))
        } else if doc.type_name == "MonoBehaviour" {
            self.script_properties.script_name(&doc.text)
                .unwrap_or("MonoBehaviour").to_string()
        } else {
            doc.type_name.clone()
        }
    }

    fn matches(&self, doc: &Document) -> bool {
        let query = self.query.trim().to_lowercase();
        query.is_empty()
            || self.display_name(doc).to_lowercase().contains(&query)
            || doc.text.to_lowercase().contains(&query)
    }

    fn node_label(&self, ui: &egui::Ui, index: usize, title: String) -> RichText {
        let start = self.raw.find("--- !u!").unwrap_or(0)
            + self.documents[..index].iter().map(|doc| doc.text.len()).sum::<usize>();
        let dirty = self.path.as_ref().and_then(|path| self.edits.get(path))
            .is_some_and(|session| session.lock().unwrap()
                .has_changes_in(start..start + self.documents[index].text.len()));
        if dirty {
            let (background, foreground) = if ui.visuals().dark_mode {
                (Color32::from_rgb(90, 75, 25), Color32::from_rgb(255, 235, 150))
            } else {
                (Color32::from_rgb(255, 250, 185), Color32::from_rgb(90, 65, 0))
            };
            RichText::new(format!("{title} *")).background_color(background).color(foreground)
        } else {
            RichText::new(title)
        }
    }

    fn show_game_object(
        &mut self,
        ui: &mut egui::Ui,
        game_object: usize,
        children: &HashMap<String, Vec<usize>>,
        components: &HashMap<String, Vec<usize>>,
    ) {
        let id = self.documents[game_object].file_id.clone();
        let title = self.display_name(&self.documents[game_object]);
        if !self.game_object_has_match(game_object, children, components) {
            return;
        }
        let force_open = self.locate_target.is_some_and(|target| {
            let mut pending = vec![game_object];
            let mut visited = std::collections::HashSet::new();
            while let Some(item) = pending.pop() {
                if !visited.insert(item) { continue; }
                let id = &self.documents[item].file_id;
                if item == target || components.get(id).is_some_and(|items| items.contains(&target)) { return true; }
                if let Some(items) = children.get(id) { pending.extend(items.iter().copied()); }
            }
            false
        });
        let response = egui::CollapsingHeader::new(self.node_label(ui, game_object, title).strong())
            .id_salt(format!("go-{id}"))
            .default_open(true)
            .open(force_open.then_some(true))
            .show(ui, |ui| {
                if let Some(ids) = components.get(&id) {
                    for &component in ids {
                        self.show_document(ui, component, false);
                    }
                }
                if let Some(ids) = children.get(&id).cloned() {
                    for child in ids {
                        self.show_game_object(ui, child, children, components);
                    }
                }
            });
        if response.header_response.clicked() {
            self.selected = Some(game_object);
        }
        if self.locate_target == Some(game_object) {
            response.header_response.scroll_to_me(Some(egui::Align::Center));
            self.locate_target = None;
        }
    }

    fn show_document(&mut self, ui: &mut egui::Ui, index: usize, is_problem: bool) {
        if !self.matches(&self.documents[index]) {
            return;
        }
        let is_match = !self.query.trim().is_empty();
        let label = if is_problem {
            self.node_label(ui, index, format!("⚠ {}", self.display_name(&self.documents[index])))
                .color(warning_color(ui))
        } else {
            self.node_label(ui, index, self.display_name(&self.documents[index]))
        };
        let response = if is_match {
            let highlight_color = if ui.visuals().dark_mode {
                Color32::YELLOW
            } else {
                Color32::from_rgb(190, 95, 0)
            };
            egui::Frame::new()
                .stroke(egui::Stroke::new(2.0_f32, highlight_color))
                .inner_margin(egui::Margin::symmetric(2, 1))
                .show(ui, |ui| {
                    ui.selectable_label(self.selected == Some(index), label)
                })
                .inner
        } else {
            ui.selectable_label(self.selected == Some(index), label)
        };
        if response.clicked() {
            self.selected = Some(index);
        }
        if self.locate_target == Some(index) {
            response.scroll_to_me(Some(egui::Align::Center));
            self.locate_target = None;
        }
    }

    fn game_object_has_match(
        &self,
        game_object: usize,
        children: &HashMap<String, Vec<usize>>,
        components: &HashMap<String, Vec<usize>>,
    ) -> bool {
        let id = &self.documents[game_object].file_id;
        self.matches(&self.documents[game_object])
            || components.get(id).is_some_and(|items| {
                items
                    .iter()
                    .any(|&item| self.matches(&self.documents[item]))
            })
            || children.get(id).is_some_and(|items| {
                items
                    .iter()
                    .any(|&item| self.game_object_has_match(item, children, components))
            })
    }

    fn show_animation_tree(&mut self, ui: &mut egui::Ui, clip_index: usize) {
        let clip_name = name_from_text(&self.documents[clip_index].text)
            .unwrap_or_else(|| "AnimationClip".to_string());
        ui.label(RichText::new(clip_name).strong());
        ui.small("按 Animation path / Component / Property 重建");
        let tree = animation_path_tree(&self.documents[clip_index].text);
        show_animation_path_node(
            ui,
            &tree,
            clip_index,
            &mut self.selected,
            &mut self.selected_animation_track,
        );
    }

    fn show_custom_asset_tree(&mut self, ui: &mut egui::Ui, index: usize) {
        let title = name_from_text(&self.documents[index].text)
            .unwrap_or_else(|| self.display_name(&self.documents[index]));
        let fields = custom_list_fields(&self.documents[index].text);
        let response =
            ui.selectable_label(self.selected == Some(index), self.node_label(ui, index, title).strong());
        if response.clicked() {
            self.selected = Some(index);
        }
        for (field, entries) in fields {
            egui::CollapsingHeader::new(format!("{field} ({})", entries.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for entry in entries {
                        if ui.selectable_label(false, entry).clicked() {
                            self.selected = Some(index);
                        }
                    }
                });
        }
    }

    fn show_sectioned_asset_tree(&mut self, ui: &mut egui::Ui, index: usize, sections: &[&str]) {
        let title = name_from_text(&self.documents[index].text)
            .unwrap_or_else(|| self.display_name(&self.documents[index]));
        if ui
            .selectable_label(self.selected == Some(index), self.node_label(ui, index, title).strong())
            .clicked()
        {
            self.selected = Some(index);
        }
        for (section, count) in yaml_section_counts(&self.documents[index].text, sections) {
            let label = if count == 0 {
                section.to_string()
            } else {
                format!("{section} ({count})")
            };
            if ui.selectable_label(false, label).clicked() {
                self.selected = Some(index);
            }
        }
    }

    fn show_animator_controller_tree(&mut self, ui: &mut egui::Ui, controller_index: usize) {
        let controller = &self.documents[controller_index];
        let title =
            name_from_text(&controller.text).unwrap_or_else(|| "Animator Controller".to_string());
        if ui
            .selectable_label(
                self.selected == Some(controller_index),
                self.node_label(ui, controller_index, title).strong(),
            )
            .clicked()
        {
            self.selected = Some(controller_index);
        }
        for machine_id in file_ids_for_key(&controller.text, "m_StateMachine") {
            let Some(machine_index) = self
                .documents
                .iter()
                .position(|document| document.file_id == machine_id)
            else {
                continue;
            };
            let machine_name = name_from_text(&self.documents[machine_index].text)
                .unwrap_or_else(|| "State Machine".to_string());
            egui::CollapsingHeader::new(self.node_label(ui, machine_index, machine_name))
                .default_open(true)
                .show(ui, |ui| {
                    for state_id in file_ids_for_key(&self.documents[machine_index].text, "m_State")
                    {
                        if let Some(state_index) = self
                            .documents
                            .iter()
                            .position(|document| document.file_id == state_id)
                        {
                            let state_name = name_from_text(&self.documents[state_index].text)
                                .unwrap_or_else(|| "State".to_string());
                            if ui
                                .selectable_label(self.selected == Some(state_index), self.node_label(ui, state_index, state_name))
                                .clicked()
                            {
                                self.selected = Some(state_index);
                            }
                        }
                    }
                });
        }
    }

    fn hierarchy_preferred_width(&self, ctx: &egui::Context) -> f32 {
        let measure = |text: &str| {
            ctx.fonts(|fonts| {
                fonts
                    .layout_no_wrap(
                        text.to_owned(),
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    )
                    .rect
                    .width()
            })
        };
        let mut widest = measure("按 GameObject / Transform 层级重建") + 16.0;
        if let Some(clip) = self
            .documents
            .iter()
            .find(|document| document.type_name == "AnimationClip")
        {
            widest = widest.max(measure(&name_from_text(&clip.text).unwrap_or_default()) + 16.0);
            let tree = animation_path_tree(&clip.text);
            let mut bindings = Vec::new();
            collect_animation_bindings(&tree, &mut bindings);
            for binding in bindings {
                let depth = binding
                    .path
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .count() as f32;
                widest = widest.max(measure(&binding.class_name) + (depth + 1.0) * 18.0 + 32.0);
                widest = widest.max(measure(&binding.attribute) + (depth + 2.0) * 18.0 + 32.0);
            }
        } else {
            for document in &self.documents {
                widest = widest.max(measure(&self.display_name(document)) + 88.0);
            }
        }
        widest.ceil().max(260.0)
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.path.as_ref().is_some_and(|path| !self.edits.contains_key(path))
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
            if let Some(path) = self.path.clone() {
                if let Some(session) = self.saved_histories.get(&path).cloned() {
                    let mut history = session.lock().unwrap();
                    if history.can_undo() {
                        if fs::read(&path).ok().as_ref() == Some(&history.disk) {
                            history.undo();
                            self.raw = history.original.clone();
                            self.documents = parse_unity_yaml(&self.raw);
                            drop(history);
                            self.saved_histories.remove(&path);
                            self.edits.insert(path, session);
                        } else { self.status = "文件已被外部修改，请重新打开后再编辑，不能撤销旧记录。".into(); }
                    }
                }
            }
        }
        let current_edit = self.path.as_ref().and_then(|path| self.edits.get(path)).cloned();
        editing::publish(ctx, current_edit.clone(), 0..self.raw.len());
        if let Some(session) = &current_edit {
            if ctx.input(|input| input.pointer.any_pressed() || !input.pointer.any_down()) { session.lock().unwrap().finish_drag(); }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
                session.lock().unwrap().undo();
                ctx.memory_mut(|memory| { if let Some(id) = memory.focused() { memory.surrender_focus(id); } });
            }
        }
        if ctx.input(|input| input.viewport().close_requested()) && !self.edits.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if let Some(session) = self.edits.values().next() { session.lock().unwrap().warning = Some("仍有文件处于编辑模式，请先保存或放弃编辑，再关闭程序。".into()); }
        }
        if self.selected.is_some() { self.last_selected = self.selected; }
        let dropped_paths = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        for path in dropped_paths {
            self.open_file(path);
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("设置").clicked() { self.show_settings = true; }
                if ui.button("打开").clicked() {
                    let dialog = rfd::FileDialog::new().add_filter(
                        "Unity YAML",
                        &[
                            "prefab",
                            "unity",
                            "asset",
                            "mat",
                            "controller",
                            "overrideController",
                            "anim",
                            "spriteatlas",
                            "guiskin",
                            "yaml",
                            "yml",
                        ],
                    );
                    let dialog = if let Some(directory) = &self.last_open_dir {
                        dialog.set_directory(directory)
                    } else {
                        dialog
                    };
                    if let Some(path) = dialog.pick_file() {
                        self.open_file(path);
                    }
                }
                ui.label(
                    self.path
                        .as_ref()
                        .map_or("尚未打开文件".into(), |p| p.display().to_string()),
                );
            });
            ui.horizontal(|ui| {
                ui.label("查找：");
                let search_border = if ui.visuals().dark_mode {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                };
                let search_response = egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0_f32, search_border))
                    .inner_margin(egui::Margin::symmetric(1, 1))
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.query)
                                .hint_text("名称、组件、字段值、fileID 或 GUID"),
                        )
                    })
                    .inner;
                if search_response.changed() {
                    let query = self.query.trim().to_lowercase();
                    if query.is_empty() {
                        self.selected = self.selected.or(self.last_selected);
                        self.locate_target = self.selected;
                    } else {
                        self.selected = self.documents.iter().position(|document| {
                            self.display_name(document).to_lowercase().contains(&query)
                                || document.text.to_lowercase().contains(&query)
                        });
                    }
                }
                if ui.button("×").on_hover_text("清除查找").clicked() {
                    self.query.clear();
                    self.selected = self.selected.or(self.last_selected);
                    self.locate_target = self.selected;
                }
            });
            ui.small("也可以将 .prefab、.unity、.asset 等 Unity YAML 文件拖入窗口打开");
            if !self.tabs.is_empty() {
                ui.separator();
                let mut activate = None;
                let mut close = None;
                ui.horizontal_wrapped(|ui| {
                    for (index, tab) in self.tabs.iter().enumerate() {
                        let title = tab
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("未命名");
                        let is_active = self.active_tab == index;
                        let dirty = self.edits.get(&tab.path).is_some_and(|session| session.lock().unwrap().has_changes());
                        let title = if dirty { format!("{title} *") } else { title.to_string() };
                        let (open_clicked, close_clicked) = unified_tab(ui, &title, is_active);
                        if open_clicked {
                            activate = Some(index);
                        }
                        if close_clicked {
                            close = Some(index);
                        }
                    }
                });
                if let Some(index) = close {
                    self.close_tab(index);
                } else if let Some(index) = activate {
                    self.save_active_tab();
                    self.activate_tab(index);
                }
            }
        });

        let hierarchy_width = self.hierarchy_preferred_width(ctx);
        self.script_properties.update(self.path.as_deref(), ctx);
        egui::SidePanel::left("hierarchy")
            .resizable(false)
            .exact_width(hierarchy_width)
            .show(ctx, |ui| {
                ui.heading("Hierarchy");
                ui.small("按 GameObject / Transform 层级重建");
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(clip_index) = self
                            .documents
                            .iter()
                            .position(|document| document.type_name == "AnimationClip")
                        {
                            self.show_animation_tree(ui, clip_index);
                        } else if let Some(controller_index) = self
                            .documents
                            .iter()
                            .position(|document| document.type_name == "AnimatorController")
                        {
                            self.show_animator_controller_tree(ui, controller_index);
                        } else if let Some(material_index) = self
                            .documents
                            .iter()
                            .position(|document| document.type_name == "Material")
                        {
                            self.show_sectioned_asset_tree(
                                ui,
                                material_index,
                                &["m_Shader", "m_TexEnvs", "m_Floats", "m_Colors"],
                            );
                        } else if let Some(atlas_index) = self
                            .documents
                            .iter()
                            .position(|document| document.type_name == "SpriteAtlas")
                        {
                            self.show_sectioned_asset_tree(
                                ui,
                                atlas_index,
                                &["m_ObjectsForPacking", "m_PackedSprites", "platformSettings"],
                            );
                        } else if let Some(override_index) = self
                            .documents
                            .iter()
                            .position(|document| document.type_name == "AnimatorOverrideController")
                        {
                            self.show_sectioned_asset_tree(
                                ui,
                                override_index,
                                &["m_Controller", "m_Clips", "m_Overrides"],
                            );
                        } else {
                            let (roots, children, components, unlinked) =
                                build_tree(&self.documents);
                            let has_roots = !roots.is_empty();
                            for root in roots {
                                self.show_game_object(ui, root, &children, &components);
                            }
                            if !has_roots && !unlinked.is_empty() {
                                for item in unlinked {
                                    self.show_custom_asset_tree(ui, item);
                                }
                            } else if !unlinked.is_empty() {
                                egui::CollapsingHeader::new(
                                    RichText::new("未关联对象 / 解析异常").color(warning_color(ui)),
                                )
                                .open(self.locate_target.map(|_| true))
                                .show(ui, |ui| {
                                    for item in unlinked {
                                        self.show_document(ui, item, true);
                                    }
                                });
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let current_edit = self.path.as_ref().and_then(|path| self.edits.get(path)).cloned();
            let header_fill = if current_edit.is_some() {
                if ui.visuals().dark_mode { Color32::from_rgb(35, 65, 90) }
                else { Color32::from_rgb(215, 235, 252) }
            } else { Color32::TRANSPARENT };
            egui::Frame::new().fill(header_fill).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.heading(if current_edit.is_some() { "Inspector · 编辑模式" } else { "只读 Inspector" });
                if current_edit.is_some() {
                    if ui.button("放弃编辑").clicked() {
                        if let Some(path) = &self.path {
                            if let Some(session) = self.edits.remove(path) {
                                let disk = session.lock().unwrap().disk.clone();
                                let (decoded, _, errors) = UTF_8.decode(&disk);
                                self.raw = if errors { GBK.decode(&disk).0.into_owned() } else { decoded.into_owned() };
                                self.documents = parse_unity_yaml(&self.raw);
                            }
                            self.saved_histories.remove(path);
                        }
                        self.save_active_tab();
                        editing::publish(ctx, None, 0..0);
                    }
                    if ui.button("保存编辑").clicked() {
                        self.pending_save = self.path.clone();
                        ctx.memory_mut(|memory| { if let Some(id) = memory.focused() { memory.surrender_focus(id); } });
                    }
                } else if self.path.is_some() && ui.add_enabled(!self.documents.is_empty(), egui::Button::new("编辑模式")).clicked() {
                    let path = self.path.clone().unwrap();
                    if let Some(history) = self.saved_histories.get(&path).cloned().filter(|history| fs::read(&path).ok().as_ref() == Some(&history.lock().unwrap().disk)) {
                        self.raw = history.lock().unwrap().original.clone();
                        self.documents = parse_unity_yaml(&self.raw);
                        self.saved_histories.remove(&path);
                        self.edits.insert(path, history.clone());
                        editing::publish(ctx, Some(history), 0..self.raw.len());
                    } else {
                    self.saved_histories.remove(&path);
                    match fs::read(&path) {
                        Ok(disk) => {
                            let (decoded, _, had_errors) = UTF_8.decode(&disk);
                            let decoded = if had_errors {
                                let (decoded, _, errors) = GBK.decode(&disk);
                                if errors { self.status = "文件包含无法正确解码的字节，暂不开放编辑。".into(); return; }
                                decoded.into_owned()
                            } else { decoded.into_owned() };
                            if decoded != self.raw {
                                self.status = "文件已被外部修改，请重新打开后再进入编辑模式。".into();
                                return;
                            }
                            let session = Arc::new(std::sync::Mutex::new(editing::Session::new(path.clone(), self.raw.clone(), disk)));
                            session.lock().unwrap().set_undo_limit(self.settings.undo_steps);
                            self.edits.insert(path, session.clone());
                            editing::publish(ctx, Some(session), 0..self.raw.len());
                        }
                        Err(error) => self.status = format!("无法进入编辑模式：{error}"),
                    }
                    }
                }
                if self.path.as_ref().is_some_and(|path| !self.edits.contains_key(path)) {
                    if ui.button("重载")
                        .on_hover_text("从磁盘重新读取当前文件并清空旧撤销记录")
                        .clicked() {
                        self.reload_current_file(ctx);
                    }
                }
            });
            });
			if self.documents.iter().any(|doc| doc.type_name == "AnimatorController") {
				controller_graph::show(ui, &self.documents, &mut self.selected,
					self.path.as_ref().map(|path| path.to_string_lossy().into_owned()).unwrap_or_default());
				ui.separator();
			}
			if let Some(index) = self.selected {
				let doc = &self.documents[index];
                let offset = self.documents[..index].iter().map(|doc| doc.text.len()).sum::<usize>();
                let prefix = self.raw.find("--- !u!").unwrap_or(0);
                editing::publish(ctx, self.path.as_ref().and_then(|path| self.edits.get(path)).cloned(), prefix + offset..prefix + offset + doc.text.len());
				let is_animation_clip = doc.type_name == "AnimationClip";
				let is_transform = matches!(doc.type_name.as_str(), "Transform" | "RectTransform");
				let is_script = doc.type_name == "MonoBehaviour";
				let is_sprite_renderer = doc.type_name == "SpriteRenderer";
				let is_collider = script_properties::is_collider_type(&doc.type_name);
				let is_animator = doc.type_name == "Animator";
				let is_material = doc.type_name == "Material";
				// Animation clips use their dedicated inspector, even when their
				// keyframe lists also match the generic table format.
				let custom_table = if is_script || is_animation_clip || is_transform || is_sprite_renderer || is_collider || is_animator || is_material {
					None
				} else {
					yaml_uniform_table(&doc.text).map(|mut table| {
                        self.script_properties.resolve_table_types(&doc.text, &mut table);
                        table
                    })
				};
				ui.label(format!("{}  ·  fileID: {}  ·  YAML 第 {} 行", self.display_name(doc), doc.file_id, doc.start_line));
				let guid_entries = guid_entries_from_text(&doc.text);
				ui.horizontal(|ui| {
					ui.menu_button("复制 GUID", |ui| {
						ui.set_min_width(320.0);
						if guid_entries.is_empty() {
							ui.label("当前对象没有 GUID");
						}
						for (field_name, guid) in &guid_entries {
							if ui.button(format!("复制 {field_name}")).clicked() {
								ctx.copy_text(guid.clone());
								ui.close_menu();
							}
							ui.add(
								egui::Label::new(RichText::new(guid).monospace())
									.wrap_mode(egui::TextWrapMode::Extend),
							);
							ui.separator();
						}
					});
					ui.menu_button("复制 FileID", |ui| {
						ui.set_min_width(320.0);
						if ui.button("自身 FileID").clicked() {
							ctx.copy_text(doc.file_id.clone());
							ui.close_menu();
						}
						ui.monospace(&doc.file_id);
						ui.separator();
						for (field_name, file_id) in file_id_entries_from_text(&doc.text) {
							if ui.button(format!("复制 {field_name}")).clicked() {
								ctx.copy_text(file_id.clone());
								ui.close_menu();
							}
							ui.monospace(file_id);
							ui.separator();
						}
					});
					if is_animation_clip {
						ui.add_space(8.0);
						let duration = animation_duration(&doc.text);
						let sample_rate = animation_sample_rate(&doc.text)
							.map(|rate| format!("　·　{rate:.0} FPS"))
							.unwrap_or_default();
						ui.small(format!("总时长：0.00s — {duration:.2}s{sample_rate}"));
					}
					if is_transform {
						ui.selectable_value(&mut self.show_transform_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_transform_yaml, true, "显示 YAML");
					} else if is_script {
						ui.selectable_value(&mut self.show_script_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_script_yaml, true, "显示 YAML");
					} else if is_sprite_renderer {
						ui.selectable_value(&mut self.show_sprite_renderer_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_sprite_renderer_yaml, true, "显示 YAML");
					} else if is_collider {
						ui.selectable_value(&mut self.show_collider_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_collider_yaml, true, "显示 YAML");
					} else if is_animator {
						ui.selectable_value(&mut self.show_animator_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_animator_yaml, true, "显示 YAML");
					} else if is_material {
						ui.selectable_value(&mut self.show_material_yaml, false, "显示属性");
						ui.selectable_value(&mut self.show_material_yaml, true, "显示 YAML");
					} else if custom_table.is_some() {
						if ui.selectable_label(self.show_custom_table, "显示表格").clicked() {
							self.show_custom_table = true;
						}
						if ui.selectable_label(!self.show_custom_table, "显示 YAML").clicked() {
							self.show_custom_table = false;
						}
						if self.show_custom_table {
							ui.checkbox(&mut self.freeze_table_header, "冻结表头");
						}
					}
				});
				ui.separator();
				script_properties::show_prefab_reference(ui, &doc.text, &self.documents);
				if is_animation_clip {
					show_animation_timeline(ui, &doc.text, &self.selected_animation_track, &mut self.selected_keyframe);
					ui.separator();
					ui.small("原始 YAML（点击关键帧后自动定位并高亮对应的 time / value）");
					show_animation_yaml(ui, &doc.text, self.selected_keyframe.as_ref(), &self.query);
				} else if is_transform && !self.show_transform_yaml {
					show_transform_properties(ui, &doc.text);
				} else if is_script && !self.show_script_yaml {
					self.script_properties.show(ui, &doc.text, &self.query);
				} else if is_sprite_renderer && !self.show_sprite_renderer_yaml {
					script_properties::show_sprite_renderer_properties(ui, &doc.text, &self.query);
				} else if is_collider && !self.show_collider_yaml {
					script_properties::show_collider_properties(ui, &doc.type_name, &doc.text, &self.query);
				} else if is_animator && !self.show_animator_yaml {
					script_properties::show_animator_properties(ui, &doc.text, &self.query);
				} else if is_material && !self.show_material_yaml {
					script_properties::show_material_properties(ui, &doc.text, &self.query);
				} else if let Some(table) = custom_table.filter(|_| self.show_custom_table) {
					let table_key = format!("{}:{}", doc.file_id, table.title);
					let horizontal_offset = self
						.table_horizontal_offsets
						.get(&table_key)
						.copied()
						.unwrap_or_default();
					let new_horizontal_offset = {
						let widths = self
							.table_column_widths
							.entry(table_key.clone())
							.or_insert_with(|| table_column_widths(&table));
						show_yaml_table(
							ui,
							&table,
							self.freeze_table_header,
							widths,
							horizontal_offset,
						)
					};
					self.table_horizontal_offsets
						.insert(table_key, new_horizontal_offset);
				} else {
				ScrollArea::vertical().show(ui, |ui| {
					let mut view = doc.text.clone();
					let search_text = self.query.trim().to_string();
					let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
						ui.fonts(|fonts| fonts.layout_job(yaml_layout_job(text, &search_text, ui, wrap_width)))
					};
					ui.add(
						TextEdit::multiline(&mut view)
							.font(egui::TextStyle::Monospace)
							.desired_width(f32::INFINITY)
							.layouter(&mut layouter),
					)
					.on_hover_text("可框选后右键复制；此处的输入不会写回或保存到原文件");
					});
				}
			} else { ui.label("从左侧选择 GameObject 或 Component，即可查看对应原始 YAML。应用不会写回或修改文件。"); }
			ui.separator();
			ui.small(format!("{}　·　文件编码：{}", self.status, if self.encoding.is_empty() { "尚未读取" } else { &self.encoding }));
		});

        if let Some(request) = script_properties::take_reference_request(ctx) {
            match request {
                script_properties::ReferenceOpen::Local(file_id) => {
                    if let Some(index) = self.documents.iter().position(|doc| doc.file_id == file_id) {
                        self.selected = Some(index);
                        self.locate_target = Some(index);
                        self.query.clear();
                        ctx.request_repaint();
                    } else { self.status = format!("本文件中未找到 fileID: {file_id}"); }
                }
                script_properties::ReferenceOpen::Asset(path, file_id) => {
                    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
                    if matches!(extension.as_str(), "prefab" | "unity" | "asset" | "mat" | "controller" | "overridecontroller" | "anim" | "spriteatlas" | "guiskin" | "yaml" | "yml" | "physicsmaterial2d" | "physicmaterial") {
                        // Inspect the header so binary Unity assets go to their default app.
                        let is_yaml = fs::File::open(&path).ok().is_some_and(|mut file| {
                            use std::io::Read;
                            let mut prefix = [0u8; 256];
                            let count = file.read(&mut prefix).unwrap_or(0);
                            String::from_utf8_lossy(&prefix[..count]).contains("%YAML")
                                || String::from_utf8_lossy(&prefix[..count]).contains("--- !u!")
                        });
                        if is_yaml {
                            self.open_file_with_directory_tracking(path, false);
                            self.selected = self.documents.iter().position(|doc| doc.file_id == file_id)
                                .or_else(|| (!self.documents.is_empty()).then_some(0));
                            self.query.clear();
                        } else { self.open_external_reference(&path); }
                    } else { self.open_external_reference(&path); }
                }
            }
        }

        if self.show_settings {
            egui::Window::new("设置").open(&mut self.show_settings).collapsible(false).resizable(false).default_width(400.0).show(ctx, |ui| {
                egui::Grid::new("application_settings").spacing(egui::vec2(20.0, 16.0)).show(ui, |ui| {
                    ui.label("关于");
                    if ui.button("关于 Unity YAML Viewer").clicked() { self.show_about = true; }
                    ui.end_row();
                    ui.label("主题");
                    let previous_theme = self.settings.theme.clone();
                    egui::ComboBox::from_id_salt("settings_theme").selected_text(match self.settings.theme.as_str() { "light" => "浅色", "dark" => "深色", _ => "跟随系统" }).show_ui(ui, |ui| {
                        for (value, label) in [("system", "跟随系统"), ("light", "浅色"), ("dark", "深色")] { ui.selectable_value(&mut self.settings.theme, value.into(), label); }
                    });
                    if previous_theme != self.settings.theme {
                        match self.settings.theme.as_str() { "light" => { ctx.set_theme(egui::Theme::Light); apply_light_background(ctx); }, "dark" => ctx.set_theme(egui::Theme::Dark), _ => ctx.set_theme(egui::ThemePreference::System) }
                        persist_theme(&self.settings.theme);
                    }
                    ui.end_row();
                    ui.label("单个文件缓存撤销步骤");
                    let previous = self.settings.undo_steps;
                    ui.add(egui::DragValue::new(&mut self.settings.undo_steps).range(editing::MIN_UNDO_LIMIT..=editing::MAX_UNDO_LIMIT).speed(1));
                    if previous != self.settings.undo_steps {
                        self.settings.persist();
                        for session in self.edits.values().chain(self.saved_histories.values()) { session.lock().unwrap().set_undo_limit(self.settings.undo_steps); }
                    }
                    ui.end_row();
                });
                ui.separator();
                ui.small("撤销步骤范围：50～1000，默认 200。超过上限后移除最早的记录。增加上限会增加内存占用。");
            });
        }
        if self.show_about {
            egui::Window::new("关于 Unity YAML Viewer")
                .open(&mut self.show_about)
                .resizable(true)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.heading("Unity YAML Viewer");
                    ui.label(format!("版本 {APP_VERSION}"));
                    ui.small("Unity 文本序列化资源查看与属性编辑工具");
                    ui.separator();
                    ui.heading("更新历史");
                    ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        ui.label(RichText::new(APP_VERSION).strong());
                        ui.label("• 属性与表格编辑，显式保存与放弃，修改整行高亮和逐项还原");
                        ui.label("• 支持已识别的数值、字符串、布尔值、枚举、向量、颜色和渐变关键点编辑");
                        ui.label("• 支持 Transform / RectTransform、材质字段和列表 / 数组元素编辑");
                        ui.label("• 每个文件独立撤销记录，默认 200 步，可在设置中调整到 50～1000");
                        ui.label("• 保存后保留撤销历史，Ctrl+Z 自动进入编辑模式；关闭文件或放弃编辑后清空");
                        ui.label("• 切换标签保留暂存修改，保存或放弃前阻止关闭编辑中的文件和应用");
                        ui.label("• 属性类型验证与引用保护；原始 YAML 保持只读");
                        ui.label("• 保存前检查外部改动，精确写回值片段并保留未修改文本与换行");
                        ui.label("• 设置弹窗整合关于、主题和撤销步数，记住用户设置");
                        ui.separator();
                        ui.label(RichText::new("1.0.0").strong());
                        ui.label("首个正式版本。");
                        ui.add_space(4.0);
                        ui.label("• 支持 Unity YAML 资源的层级与 Inspector 浏览");
                        ui.label("• 支持搜索、多标签页、GUID / FileID 复制和编码识别");
                        ui.label("• 提供动画、材质、动画控制器等常见资源的结构化展示");
                        ui.label("• 只读浏览资源，不写入原始文件");
                    });
                });
        }
        if let Some(path) = self.pending_save.take() {
            if let Some(session) = self.edits.get(&path).cloned() {
                let result = editing::save(&session.lock().unwrap());
                match result {
                    Ok(text) => {
                        session.lock().unwrap().mark_saved(&text);
                        self.saved_histories.insert(path.clone(), session.clone());
                        self.edits.remove(&path);
                        if self.path.as_ref() == Some(&path) {
                            self.raw = text;
                            self.documents = parse_unity_yaml(&self.raw);
                            self.encoding = "UTF-8".into();
                            self.status = "编辑已保存到文件。".into();
                            self.save_active_tab();
                            editing::publish(ctx, None, 0..0);
                        }
                    }
                    Err(message) => session.lock().unwrap().warning = Some(message),
                }
            }
        }
        if let Some(path) = self.pending_close_tab.clone() {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("未命名");
            let mut discard = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("discard_edit_on_close")).show(ctx, |ui| {
                ui.heading("⚠ 放弃编辑？");
                ui.label(format!("“{name}”有未保存的修改。是否放弃编辑并关闭文件？"));
                ui.label("选择“是”将直接关闭，不写入文件；选择“否”继续编辑。");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    discard = ui.add_sized([110.0, 30.0], egui::Button::new("是，放弃并关闭")).clicked();
                    cancel = ui.add_sized([88.0, 30.0], egui::Button::new("否，继续编辑")).clicked();
                });
            });
            if discard {
                self.pending_close_tab = None;
                self.edits.remove(&path);
                if let Some(index) = self.tabs.iter().position(|tab| tab.path == path) {
                    self.close_tab(index);
                }
                editing::publish(ctx, None, 0..0);
            } else if cancel {
                self.pending_close_tab = None;
            }
        }
        for session in self.edits.values() {
            let message = session.lock().unwrap().warning.clone();
            if let Some(message) = message {
                egui::Window::new("⚠ 编辑提示").collapsible(false).resizable(false).show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        let visuals = ui.visuals();
                        let button = egui::Button::new("确定")
                            .fill(visuals.widgets.inactive.bg_fill)
                            .stroke(egui::Stroke::new(1.0_f32, visuals.widgets.noninteractive.fg_stroke.color));
                        if ui.add_sized([88.0, 30.0], button).clicked() {
                            session.lock().unwrap().warning = None;
                        }
                    });
                });
                break;
            }
        }
    }
}

fn parse_unity_yaml(text: &str) -> Vec<Document> {
    let header = Regex::new(r"(?m)^--- !u!(\d+) &(-?\d+).*$").unwrap();
    let go = Regex::new(r"(?m)^\s*m_GameObject:\s*\{fileID:\s*(-?\d+)").unwrap();
    let father = Regex::new(r"(?m)^\s*m_Father:\s*\{fileID:\s*(-?\d+)").unwrap();
    let starts: Vec<_> = header.find_iter(text).collect();
    let mut docs = Vec::new();
    for i in 0..starts.len() {
        let mat = starts[i];
        let end = starts.get(i + 1).map_or(text.len(), |next| next.start());
        let captures = header.captures(mat.as_str()).unwrap();
        let section = text[mat.start()..end].to_string();
        let type_id = captures[1].to_string();
        let fallback_type_name = match type_id.as_str() {
            "1" => "GameObject",
            "4" => "Transform",
            "224" => "RectTransform",
            "20" => "Camera",
            "23" => "MeshRenderer",
            "33" => "MeshFilter",
            "21" => "Material",
            "74" => "AnimationClip",
            "91" => "AnimatorController",
            "95" => "Animator",
            "221" => "AnimatorOverrideController",
            "1101" => "AnimatorStateTransition",
            "1102" => "AnimatorState",
            "1107" => "AnimatorStateMachine",
            "1109" => "AnimatorTransition",
            "687078895" => "SpriteAtlas",
            "114" => "MonoBehaviour",
            "1001" => "PrefabInstance",
            _ => "Unity Object",
        }
        .to_string();
        let type_name = section
            .lines()
            .skip(1)
            .find_map(|line| line.trim().strip_suffix(':'))
            .filter(|name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
            })
            .map(str::to_owned)
            .unwrap_or(fallback_type_name);
        docs.push(Document {
            file_id: captures[2].to_string(),
            type_name,
            start_line: text[..mat.start()].bytes().filter(|&b| b == b'\n').count() + 1,
            game_object: go.captures(&section).map(|c| c[1].to_string()),
            parent_transform: father.captures(&section).map(|c| c[1].to_string()),
            text: section,
        });
    }
    docs
}

fn name_from_text(text: &str) -> Option<String> {
    Regex::new(r"(?m)^\s*m_Name:\s*(.*)$")
        .unwrap()
        .captures(text)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty())
}

fn animation_path_tree(text: &str) -> AnimationPathNode {
    let mut root = AnimationPathNode::default();
    let mut path = None;
    let mut attribute = None;
    let mut class_id = None;
    let mut keyframes = Vec::new();
    let mut pending_time = None;
    let mut active_curve = false;
    let mut in_property_curves = false;
    let mut add_binding = |path: &mut Option<String>,
                           attribute: &mut Option<String>,
                           class_id: &mut Option<String>,
                           keyframes: &mut Vec<AnimationKeyframe>| {
        if let (Some(path_value), Some(attribute_value), Some(class_id_value)) =
            (path.take(), attribute.take(), class_id.take())
        {
            let mut node = &mut root;
            if path_value.trim().is_empty() {
                node = node
                    .children
                    .entry("根对象（本对象）".to_string())
                    .or_default();
            } else {
                for segment in path_value.split('/').filter(|segment| !segment.is_empty()) {
                    node = node.children.entry(segment.to_string()).or_default();
                }
            }
            let class_name = animation_class_name(&class_id_value);
            let mut incoming_keyframes = std::mem::take(keyframes);
            if let Some(existing) = node.bindings.iter_mut().find(|binding| {
                binding.path == path_value
                    && binding.class_name == class_name
                    && binding.attribute == attribute_value
            }) {
                existing.keyframes.append(&mut incoming_keyframes);
                existing
                    .keyframes
                    .sort_by(|left, right| left.time.total_cmp(&right.time));
                existing
                    .keyframes
                    .dedup_by(|left, right| (left.time - right.time).abs() < f32::EPSILON);
            } else {
                node.bindings.push(AnimationBinding {
                    path: path_value,
                    class_name,
                    attribute: attribute_value,
                    keyframes: incoming_keyframes,
                });
            }
        }
    };
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "m_FloatCurves:" || trimmed == "m_EditorCurves:" {
            if active_curve {
                add_binding(&mut path, &mut attribute, &mut class_id, &mut keyframes);
                active_curve = false;
            }
            in_property_curves = true;
            pending_time = None;
            continue;
        }
        if in_property_curves && line.starts_with("  m_") {
            if active_curve {
                add_binding(&mut path, &mut attribute, &mut class_id, &mut keyframes);
                active_curve = false;
            }
            in_property_curves = false;
            pending_time = None;
        }
        if !in_property_curves {
            continue;
        }
        if trimmed == "- curve:" || trimmed == "curve:" {
            if active_curve {
                add_binding(&mut path, &mut attribute, &mut class_id, &mut keyframes);
            }
            active_curve = true;
            pending_time = None;
            continue;
        }
        if !active_curve {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("path:") {
            path = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("attribute:") {
            attribute = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("classID:") {
            class_id = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("time:") {
            if let Ok(time) = value.trim().parse::<f32>() {
                pending_time = Some(time);
            }
        } else if let Some(value) = trimmed.strip_prefix("value:") {
            if let Some(time) = pending_time.take() {
                keyframes.push(AnimationKeyframe {
                    time,
                    value: value.trim().to_string(),
                    yaml_line: line_index.saturating_sub(1),
                });
            }
        }
    }
    if active_curve {
        add_binding(&mut path, &mut attribute, &mut class_id, &mut keyframes);
    }
    drop(add_binding);
    root
}

fn animation_class_name(class_id: &str) -> String {
    match class_id {
        "4" => "Transform",
        "224" => "RectTransform",
        "20" => "Camera",
        "23" => "MeshRenderer",
        "95" => "Animator",
        "114" => "MonoBehaviour",
        "137" => "SkinnedMeshRenderer",
        "212" => "SpriteRenderer",
        "198" => "ParticleSystem",
        "199" => "ParticleSystemRenderer",
        other => return format!("Class ID {other}"),
    }
    .to_string()
}

fn show_animation_path_node(
    ui: &mut egui::Ui,
    node: &AnimationPathNode,
    clip_index: usize,
    selected: &mut Option<usize>,
    selected_track: &mut Option<AnimationTrackSelection>,
) {
    for (name, child) in &node.children {
        egui::CollapsingHeader::new(name)
            .default_open(true)
            .show(ui, |ui| {
                show_animation_path_node(ui, child, clip_index, selected, selected_track);
            });
    }
    let mut by_component: BTreeMap<&str, Vec<&AnimationBinding>> = BTreeMap::new();
    for binding in &node.bindings {
        by_component
            .entry(&binding.class_name)
            .or_default()
            .push(binding);
    }
    for (class_name, bindings) in by_component {
        egui::CollapsingHeader::new(class_name)
            .default_open(true)
            .show(ui, |ui| {
                let mut by_property: BTreeMap<String, Vec<&AnimationBinding>> = BTreeMap::new();
                for binding in bindings {
                    let property_root = binding
                        .attribute
                        .rsplit_once('.')
                        .map_or_else(|| binding.attribute.clone(), |(root, _)| root.to_string());
                    by_property.entry(property_root).or_default().push(binding);
                }
                for (property_root, property_bindings) in by_property {
                    if let Some(components) = compact_animation_components(&property_bindings) {
                        let label = format!("{property_root}.{components}");
                        show_animation_property(
                            ui,
                            property_bindings[0],
                            clip_index,
                            selected,
                            selected_track,
                            &label,
                        );
                    } else if property_bindings.len() > 1 {
                        egui::CollapsingHeader::new(property_root)
                            .default_open(true)
                            .show(ui, |ui| {
                                for binding in property_bindings {
                                    show_animation_property(
                                        ui,
                                        binding,
                                        clip_index,
                                        selected,
                                        selected_track,
                                        binding
                                            .attribute
                                            .rsplit_once('.')
                                            .map_or(&binding.attribute, |(_, suffix)| suffix),
                                    );
                                }
                            });
                    } else if let Some(binding) = property_bindings.first() {
                        show_animation_property(
                            ui,
                            binding,
                            clip_index,
                            selected,
                            selected_track,
                            &binding.attribute,
                        );
                    }
                }
            });
    }
}

fn compact_animation_components(bindings: &[&AnimationBinding]) -> Option<String> {
    let suffixes: Vec<_> = bindings
        .iter()
        .map(|binding| binding.attribute.rsplit_once('.').map(|(_, suffix)| suffix))
        .collect::<Option<_>>()?;
    let order: &[&str] = if suffixes
        .iter()
        .all(|suffix| matches!(*suffix, "r" | "g" | "b" | "a"))
    {
        &["r", "g", "b", "a"]
    } else if suffixes
        .iter()
        .all(|suffix| matches!(*suffix, "x" | "y" | "z"))
    {
        &["x", "y", "z"]
    } else {
        return None;
    };
    Some(
        order
            .iter()
            .filter(|component| suffixes.contains(component))
            .copied()
            .collect::<String>(),
    )
}

fn show_animation_property(
    ui: &mut egui::Ui,
    binding: &AnimationBinding,
    clip_index: usize,
    selected: &mut Option<usize>,
    selected_track: &mut Option<AnimationTrackSelection>,
    label: &str,
) {
    let track = AnimationTrackSelection {
        path: binding.path.clone(),
        class_name: binding.class_name.clone(),
        attribute: binding.attribute.clone(),
    };
    if ui
        .selectable_label(selected_track.as_ref() == Some(&track), label)
        .clicked()
    {
        *selected = Some(clip_index);
        *selected_track = Some(track);
    }
}

fn collect_animation_bindings<'a>(
    node: &'a AnimationPathNode,
    bindings: &mut Vec<&'a AnimationBinding>,
) {
    for child in node.children.values() {
        collect_animation_bindings(child, bindings);
    }
    bindings.extend(&node.bindings);
}

fn animation_stop_time(text: &str, bindings: &[&AnimationBinding]) -> f32 {
    let configured_stop = Regex::new(r"(?m)^\s*m_StopTime:\s*([0-9.]+)")
        .unwrap()
        .captures(text)
        .and_then(|capture| capture[1].parse::<f32>().ok())
        .unwrap_or(0.0);
    configured_stop
        .max(
            bindings
                .iter()
                .flat_map(|binding| binding.keyframes.iter().map(|keyframe| keyframe.time))
                .fold(0.0, f32::max),
        )
        .max(0.001)
}

fn animation_sample_rate(text: &str) -> Option<f32> {
    Regex::new(r"(?m)^\s*m_SampleRate:\s*([0-9.]+)")
        .unwrap()
        .captures(text)
        .and_then(|capture| capture[1].parse::<f32>().ok())
}

fn animation_duration(text: &str) -> f32 {
    let tree = animation_path_tree(text);
    let mut bindings = Vec::new();
    collect_animation_bindings(&tree, &mut bindings);
    animation_stop_time(text, &bindings)
}

fn curve_group_root(attribute: &str) -> Option<&str> {
    let (root, component) = attribute.rsplit_once('.')?;
    if matches!(component, "r" | "g" | "b" | "a" | "x" | "y" | "z") {
        Some(root)
    } else {
        None
    }
}

fn is_group_component(attribute: &str, root: &str) -> bool {
    attribute
        .strip_prefix(root)
        .is_some_and(|suffix| matches!(suffix, ".r" | ".g" | ".b" | ".a" | ".x" | ".y" | ".z"))
}

fn curve_value_at(binding: &AnimationBinding, time: f32) -> Option<String> {
    let first = binding.keyframes.first()?;
    if time <= first.time {
        return Some(first.value.clone());
    }
    let last = binding.keyframes.last()?;
    if time >= last.time {
        return Some(last.value.clone());
    }
    let previous = binding
        .keyframes
        .iter()
        .rev()
        .find(|keyframe| keyframe.time <= time)?;
    let next = binding
        .keyframes
        .iter()
        .find(|keyframe| keyframe.time >= time)?;
    if (next.time - previous.time).abs() < f32::EPSILON {
        return Some(previous.value.clone());
    }
    match (previous.value.parse::<f32>(), next.value.parse::<f32>()) {
        (Ok(start), Ok(end)) => {
            let progress = (time - previous.time) / (next.time - previous.time);
            Some(format!("{:.6}", start + (end - start) * progress))
        }
        _ => Some(previous.value.clone()),
    }
}

fn grouped_keyframe_values(
    bindings: &[&AnimationBinding],
    track: &AnimationTrackSelection,
    time: f32,
) -> Option<Vec<(String, String)>> {
    let root = curve_group_root(&track.attribute)?;
    let components: &[&str] = if track.attribute.ends_with(".r")
        || track.attribute.ends_with(".g")
        || track.attribute.ends_with(".b")
        || track.attribute.ends_with(".a")
    {
        &["r", "g", "b", "a"]
    } else {
        &["x", "y", "z"]
    };
    let mut values = Vec::new();
    for component in components {
        let attribute = format!("{root}.{component}");
        if let Some(value) = bindings
            .iter()
            .find(|binding| {
                binding.path == track.path
                    && binding.class_name == track.class_name
                    && binding.attribute == attribute
            })
            .and_then(|binding| curve_value_at(binding, time))
        {
            values.push(((*component).to_string(), value));
        }
    }
    (!values.is_empty()).then_some(values)
}

fn show_animation_overview(
    ui: &mut egui::Ui,
    bindings: &[&AnimationBinding],
    stop_time: f32,
) -> egui::Rect {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    let gray = if ui.visuals().dark_mode {
        Color32::from_gray(110)
    } else {
        Color32::from_gray(150)
    };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.center().y),
            egui::pos2(rect.right(), rect.center().y),
        ],
        egui::Stroke::new(1.0_f32, gray),
    );
    for binding in bindings {
        for keyframe in &binding.keyframes {
            let x = egui::lerp(
                rect.left()..=rect.right(),
                (keyframe.time / stop_time).clamp(0.0, 1.0),
            );
            ui.painter()
                .circle_filled(egui::pos2(x, rect.center().y), 2.5, gray);
        }
    }
    rect
}

fn show_animation_timeline(
    ui: &mut egui::Ui,
    text: &str,
    selected_track: &Option<AnimationTrackSelection>,
    selected_keyframe: &mut Option<SelectedKeyframe>,
) {
    let tree = animation_path_tree(text);
    let mut bindings = Vec::new();
    collect_animation_bindings(&tree, &mut bindings);
    let stop_time = animation_stop_time(text, &bindings);
    let Some(track) = selected_track else {
        show_animation_overview(ui, &bindings, stop_time);
        ui.small("从左侧路径树选择一个动画属性，可查看其独立时间轴。");
        return;
    };
    let mut selected_marker_x = None;
    let group_root = curve_group_root(&track.attribute);
    let matching: Vec<_> = bindings
        .iter()
        .copied()
        .filter(|binding| {
            binding.path == track.path
                && binding.class_name == track.class_name
                && group_root.map_or(binding.attribute == track.attribute, |root| {
                    is_group_component(&binding.attribute, root)
                })
        })
        .collect();
    ui.label(format!(
        "当前属性：{} · {}",
        track.class_name,
        group_root.unwrap_or(&track.attribute)
    ));
    let original_item_spacing_y = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    let overview_rect = show_animation_overview(ui, &bindings, stop_time);
    let mut current_track_rect = None;
    for binding in matching {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
        current_track_rect = Some(rect);
        let track_color = if ui.visuals().dark_mode {
            Color32::DARK_GRAY
        } else {
            Color32::LIGHT_GRAY
        };
        let key_color = if ui.visuals().dark_mode {
            Color32::LIGHT_BLUE
        } else {
            Color32::from_rgb(0, 100, 180)
        };
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(1.0_f32, track_color),
        );
        for (key_index, keyframe) in binding.keyframes.iter().enumerate() {
            let x = egui::lerp(
                rect.left()..=rect.right(),
                (keyframe.time / stop_time).clamp(0.0, 1.0),
            );
            let center = egui::pos2(x, rect.center().y);
            let key_rect = egui::Rect::from_center_size(center, egui::vec2(16.0, 16.0));
            let response = ui.interact(
                key_rect,
                ui.id().with((
                    binding.path.as_str(),
                    binding.class_name.as_str(),
                    binding.attribute.as_str(),
                    key_index,
                )),
                egui::Sense::click(),
            );
            if response.clicked() {
                *selected_keyframe = Some(SelectedKeyframe {
                    track_name: format!(
                        "{} · {} · {}",
                        if binding.path.is_empty() {
                            "根对象（本对象）"
                        } else {
                            &binding.path
                        },
                        binding.class_name,
                        binding.attribute
                    ),
                    time: keyframe.time,
                    value: keyframe.value.clone(),
                    yaml_line: keyframe.yaml_line,
                });
            }
            let track_name = format!(
                "{} · {} · {}",
                if binding.path.is_empty() {
                    "根对象（本对象）"
                } else {
                    &binding.path
                },
                binding.class_name,
                binding.attribute
            );
            let is_selected = selected_keyframe.as_ref().is_some_and(|selected| {
                selected.track_name == track_name
                    && (selected.time - keyframe.time).abs() < f32::EPSILON
            });
            if is_selected {
                selected_marker_x = Some(x);
            }
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x, center.y - 6.0),
                    egui::pos2(x + 6.0, center.y),
                    egui::pos2(x, center.y + 6.0),
                    egui::pos2(x - 6.0, center.y),
                ],
                if is_selected {
                    if ui.visuals().dark_mode {
                        Color32::YELLOW
                    } else {
                        Color32::from_rgb(190, 95, 0)
                    }
                } else {
                    key_color
                },
                egui::Stroke::NONE,
            ));
        }
    }
    if let (Some(x), Some(current_rect)) = (selected_marker_x, current_track_rect) {
        let marker_color = if ui.visuals().dark_mode {
            Color32::YELLOW
        } else {
            Color32::from_rgb(190, 95, 0)
        };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, overview_rect.center().y - 5.0),
                egui::pos2(x - 4.0, overview_rect.center().y - 10.0),
                egui::pos2(x + 4.0, overview_rect.center().y - 10.0),
            ],
            marker_color,
            egui::Stroke::NONE,
        ));
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, current_rect.center().y + 5.0),
                egui::pos2(x - 4.0, current_rect.center().y + 10.0),
                egui::pos2(x + 4.0, current_rect.center().y + 10.0),
            ],
            marker_color,
            egui::Stroke::NONE,
        ));
    }
    ui.spacing_mut().item_spacing.y = original_item_spacing_y;
    if let Some(keyframe) = selected_keyframe {
        ui.separator();
        ui.label(format!(
            "关键帧：{}　·　时间：{:.3}s",
            keyframe.track_name, keyframe.time
        ));
        if let Some(values) = grouped_keyframe_values(&bindings, track, keyframe.time) {
            if track.attribute.ends_with(".r")
                || track.attribute.ends_with(".g")
                || track.attribute.ends_with(".b")
                || track.attribute.ends_with(".a")
            {
                let channel = |name: &str, fallback: f32| {
                    values
                        .iter()
                        .find(|(component, _)| component == name)
                        .and_then(|(_, value)| value.parse::<f32>().ok())
                        .unwrap_or(fallback)
                };
                let values_text = values
                    .iter()
                    .map(|(component, value)| format!("{component}: {value}"))
                    .collect::<Vec<_>>()
                    .join("　");
                let can_show_swatch = ["r", "g", "b"]
                    .iter()
                    .all(|component| values.iter().any(|(present, _)| present == component));
                ui.horizontal(|ui| {
                    ui.monospace(values_text);
                    if can_show_swatch {
                        let color = Color32::from_rgba_unmultiplied(
                            (channel("r", 0.0).clamp(0.0, 1.0) * 255.0) as u8,
                            (channel("g", 0.0).clamp(0.0, 1.0) * 255.0) as u8,
                            (channel("b", 0.0).clamp(0.0, 1.0) * 255.0) as u8,
                            (channel("a", 1.0).clamp(0.0, 1.0) * 255.0) as u8,
                        );
                        let (swatch, _) =
                            ui.allocate_exact_size(egui::vec2(56.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(swatch, 2.0, color);
                        ui.painter().rect_stroke(
                            swatch,
                            2.0,
                            egui::Stroke::new(
                                1.0_f32,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ),
                            egui::StrokeKind::Outside,
                        );
                    }
                });
            } else {
                ui.monospace(
                    values
                        .iter()
                        .map(|(component, value)| format!("{component}: {value}"))
                        .collect::<Vec<_>>()
                        .join("　"),
                );
            }
        } else {
            ui.monospace(format!("值：{}", keyframe.value));
        }
    }
}

fn show_animation_yaml(
    ui: &mut egui::Ui,
    text: &str,
    selected_keyframe: Option<&SelectedKeyframe>,
    query: &str,
) {
    let selected_line = selected_keyframe.map(|keyframe| keyframe.yaml_line);
    ScrollArea::vertical()
        .id_salt("animation_original_yaml")
        .auto_shrink([false, false])
        .max_height(320.0)
        .show(ui, |ui| {
            for (line_index, line) in text.lines().enumerate() {
                let is_keyframe_line = selected_line.is_some_and(|target| {
                    line_index == target || line_index == target.saturating_add(1)
                });
                let background = if is_keyframe_line {
                    if ui.visuals().dark_mode {
                        Color32::from_rgb(85, 68, 0)
                    } else {
                        Color32::from_rgb(255, 230, 120)
                    }
                } else {
                    Color32::TRANSPARENT
                };
                let mut job = yaml_layout_job(line, query.trim(), ui, f32::INFINITY);
                job.wrap.max_width = f32::INFINITY;
                let response = egui::Frame::new()
                    .fill(background)
                    .inner_margin(egui::Margin::symmetric(3, 1))
                    .show(ui, |ui| ui.add(egui::Label::new(job).selectable(true)))
                    .response;
                if is_keyframe_line {
                    ui.scroll_to_rect(response.rect, Some(egui::Align::Center));
                }
            }
        });
}

fn guid_entries_from_text(text: &str) -> Vec<(String, String)> {
    let field_pattern = Regex::new(r"^\s*([A-Za-z0-9_]+):").unwrap();
    let guid_pattern = Regex::new(r"guid:\s*([0-9a-fA-F]{32})").unwrap();
    let mut entries = Vec::new();
    let mut current_field = None;
    for line in text.lines() {
        if let Some(capture) = field_pattern.captures(line) {
            current_field = Some(capture[1].trim_start_matches("m_").to_string());
        }
        for capture in guid_pattern.captures_iter(line) {
            if let Some(field_name) = &current_field {
                entries.push((field_name.clone(), capture[1].to_lowercase()));
            }
        }
    }
    entries
}

fn file_id_entries_from_text(text: &str) -> Vec<(String, String)> {
    let inline_reference = Regex::new(r"([A-Za-z0-9_]+):\s*\{fileID:\s*(-?\d+)").unwrap();
    let mut entries = Vec::new();
    for line in text.lines() {
        for capture in inline_reference.captures_iter(line) {
            let file_id = capture[2].to_string();
            if file_id != "0" {
                entries.push((capture[1].trim_start_matches("m_").to_string(), file_id));
            }
        }
    }
    entries
}

fn warning_color(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::YELLOW
    } else {
        Color32::from_rgb(180, 90, 0)
    }
}

fn yaml_layout_job(
    text: &str,
    query: &str,
    ui: &egui::Ui,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    let normal = egui::TextFormat {
        font_id: egui::FontId::monospace(14.0),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    if query.is_empty() {
        job.append(text, 0.0, normal);
        return job;
    }
    let highlight = egui::TextFormat {
        font_id: egui::FontId::monospace(14.0),
        color: Color32::BLACK,
        background: Color32::YELLOW,
        ..Default::default()
    };
    let mut cursor = 0;
    for (start, matched) in text.match_indices(query) {
        job.append(&text[cursor..start], 0.0, normal.clone());
        job.append(matched, 0.0, highlight.clone());
        cursor = start + matched.len();
    }
    job.append(&text[cursor..], 0.0, normal);
    job
}

fn file_ids_for_key(text: &str, key: &str) -> Vec<String> {
    let expression = format!(r"(?m)^\s*(?:-\s*)?{}:\s*\{{fileID:\s*(-?\d+)", regex::escape(key));
    Regex::new(&expression)
        .unwrap()
        .captures_iter(text)
        .map(|capture| capture[1].to_string())
        .collect()
}

fn yaml_section_counts(text: &str, sections: &[&str]) -> Vec<(String, usize)> {
    let lines: Vec<_> = text.lines().collect();
    let mut result = Vec::new();
    for section in sections {
        let Some(index) = lines
            .iter()
            .position(|line| line.trim() == format!("{section}:"))
        else {
            continue;
        };
        let indent = lines[index]
            .len()
            .saturating_sub(lines[index].trim_start().len());
        let mut count = 0;
        for line in &lines[index + 1..] {
            let next_indent = line.len().saturating_sub(line.trim_start().len());
            let trimmed = line.trim();
            if !trimmed.is_empty() && next_indent <= indent && !trimmed.starts_with("- ") {
                break;
            }
            if next_indent == indent + 2 && trimmed.starts_with("- ") {
                count += 1;
            }
        }
        result.push(((*section).to_string(), count));
    }
    result
}

fn inline_vector(value: &str, axes: &[&str]) -> Option<Vec<f64>> {
    let body = value.trim().strip_prefix('{')?.strip_suffix('}')?;
    let mut fields = BTreeMap::new();
    for entry in body.split(',') {
        let (axis, value) = entry.split_once(':')?;
        let number = value.trim().parse::<f64>().ok()?;
        if !number.is_finite() || fields.insert(axis.trim(), number).is_some() {
            return None;
        }
    }
    if fields.len() != axes.len() {
        return None;
    }
    axes.iter().map(|axis| fields.get(axis).copied()).collect()
}

// Unity applies Euler rotation in Z-X-Y order. Normalize before conversion
// and choose Z = 0 at the gimbal singularity, where angles are not unique.
fn rotation_euler_degrees(q: &[f64]) -> Option<Vec<f64>> {
    let [x, y, z, w] = q else { return None };
    let norm = x.hypot(*y).hypot(*z).hypot(*w);
    if norm == 0.0 || !norm.is_finite() {
        return None;
    }
    let (x, y, z, w) = (x / norm, y / norm, z / norm, w / norm);
    let sin_x = (2.0 * (w * x - y * z)).clamp(-1.0, 1.0);
    let angle_x = sin_x.asin();
    let (angle_y, angle_z) = if sin_x.abs() < 1.0 - 1e-10 {
        (
            (2.0 * (x * z + w * y)).atan2(1.0 - 2.0 * (x * x + y * y)),
            (2.0 * (x * y + w * z)).atan2(1.0 - 2.0 * (x * x + z * z)),
        )
    } else {
        ((2.0 * (w * y - x * z)).atan2(1.0 - 2.0 * (y * y + z * z)), 0.0)
    };
    Some(vec![angle_x.to_degrees(), angle_y.to_degrees(), angle_z.to_degrees()])
}

fn show_transform_properties(ui: &mut egui::Ui, text: &str) {
    let viewport_width = ui.available_width();
    ScrollArea::both().id_salt("transform_properties").show(ui, |ui| {
        ui.small("旋转以欧拉角（度）显示；编辑模式中的改动需要保存才会写入文件。");
        let mut remaining = Vec::new();
        ui.vertical(|ui| {
                for line in text.lines() {
                    // Only direct fields of the selected Transform are properties.
                    let Some(field) = line.strip_prefix("  ").filter(|field| !field.starts_with(' ')) else {
                        remaining.push(line);
                        continue;
                    };
                    let Some((name, raw_value)) = field.split_once(':') else {
                        remaining.push(line);
                        continue;
                    };
                    let (label, axes): (&str, &[&str]) = match name {
                        "m_LocalPosition" => ("LocalPosition", &["x", "y", "z"]),
                        "m_LocalRotation" => ("LocalRotation (°)", &["x", "y", "z", "w"]),
                        "m_LocalScale" => ("LocalScale", &["x", "y", "z"]),
                        "m_LocalEulerAnglesHint" => ("LocalEulerAnglesHint (°)", &["x", "y", "z"]),
                        "m_AnchorMin" => ("AnchorMin", &["x", "y"]),
                        "m_AnchorMax" => ("AnchorMax", &["x", "y"]),
                        "m_AnchoredPosition" => ("AnchoredPosition", &["x", "y"]),
                        "m_SizeDelta" => ("SizeDelta", &["x", "y"]),
                        "m_Pivot" => ("Pivot", &["x", "y"]),
                        _ => {
                            remaining.push(line);
                            continue;
                        }
                    };
                    let values = inline_vector(raw_value, axes).and_then(|values| {
                        if name == "m_LocalRotation" {
                            rotation_euler_degrees(&values)
                        } else {
                            Some(values)
                        }
                    });
                    let highlight = editing::placeholder(ui);
                    let response = ui.horizontal(|ui| {
                        let label_width = 152.0;
                        ui.allocate_ui_with_layout(egui::vec2(label_width, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_width(label_width);
                            ui.add(egui::Label::new(label).truncate())
                        }).inner
                            .on_hover_text(format!("{name}:{}", raw_value));
                        let kind = if name == "m_LocalRotation" { "Euler" } else if axes.len() == 2 { "Vector2" } else { "Vector3" };
                        if editing::control(ui, name, raw_value.trim(), kind, None) { return; }
                        if let Some(values) = values {
                            let display: Vec<String> = values.into_iter().map(|value| {
                                if name == "m_LocalRotation" {
                                    format!("{:.6}", if value.abs() < 0.0000005 { 0.0 } else { value })
                                        .trim_end_matches('0').trim_end_matches('.').to_string()
                                } else {
                                    value.to_string()
                                }
                            }).collect();
                            show_vector_inputs(ui, &["x", "y", "z"][..display.len()], &display,
                                viewport_width - label_width - ui.spacing().item_spacing.x);
                        } else {
                            ui.colored_label(Color32::RED, "无法识别有效数值");
                            ui.monospace(raw_value.trim());
                        }
                    });
                    let mut rect = response.response.rect;
                    rect.max.x = ui.max_rect().right();
                    editing::highlight(ui, highlight, rect, editing::binding(ui, name, raw_value.trim()).is_some_and(|binding| binding.dirty()));
                }
            });
        ui.separator();
        ui.collapsing("其他字段（原始 YAML）", |ui| {
            let mut remaining = remaining.join("\n");
            ui.add(TextEdit::multiline(&mut remaining)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY).interactive(false));
        });
    });
}

// Size every component before placing any widget so the final axis cannot
// absorb the remaining space. Dimensions are egui logical pixels.
fn show_vector_inputs(ui: &mut egui::Ui, axes: &[&str], values: &[String], available_width: f32) {
    const MIN_WIDTH: f32 = 48.0;
    const MAX_WIDTH: f32 = 96.0;
    const AXIS_WIDTH: f32 = 12.0;
    let count = axes.len();
    if count == 0 { return; }
    let spacing = ui.spacing().item_spacing.x;
    let overhead = AXIS_WIDTH * count as f32 + spacing * (count * 2 - 1) as f32;
    let width = ((available_width - overhead) / count as f32).clamp(MIN_WIDTH, MAX_WIDTH);
    let height = ui.spacing().interact_size.y;
    for (axis, value) in axes.iter().zip(values) {
        ui.add_sized([AXIS_WIDTH, height], egui::Label::new(*axis));
        let mut display = value.clone();
        ui.add_sized([width, height], TextEdit::singleline(&mut display)
            .desired_width(f32::INFINITY)
            .min_size(egui::vec2(width, height)).interactive(false))
            .on_hover_text(value);
    }
}

#[cfg(test)]
mod transform_property_tests {
    use super::{inline_vector, rotation_euler_degrees};

    #[test]
    fn vectors_require_complete_finite_unique_components() {
        assert_eq!(inline_vector("{z: 3, x: 1, y: -2}", &["x", "y", "z"]), Some(vec![1.0, -2.0, 3.0]));
        for invalid in ["{x: 1, y: 2}", "{x: 1, x: 2, z: 3}", "{x: NaN, y: 2, z: 3}", "{x: inf, y: 2, z: 3}"] {
            assert!(inline_vector(invalid, &["x", "y", "z"]).is_none());
        }
    }

    #[test]
    fn rotation_handles_identity_normalization_and_gimbal_lock() {
        assert_eq!(rotation_euler_degrees(&[0.0, 0.0, 0.0, 2.0]), Some(vec![0.0, 0.0, 0.0]));
        assert!(rotation_euler_degrees(&[0.0; 4]).is_none());
        let half = std::f64::consts::FRAC_1_SQRT_2;
        for (quaternion, expected) in [
            ([half, 0.0, 0.0, half], [90.0, 0.0, 0.0]),
            ([0.0, half, 0.0, half], [0.0, 90.0, 0.0]),
            ([0.0, 0.0, half, half], [0.0, 0.0, 90.0]),
        ] {
            let actual = rotation_euler_degrees(&quaternion).unwrap();
            for (actual, expected) in actual.iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-6);
            }
        }
    }
}

fn yaml_uniform_table(text: &str) -> Option<YamlTable> {
    let lines: Vec<_> = text.lines().collect();
    for (start, line) in lines.iter().enumerate() {
        let indentation = line.len().saturating_sub(line.trim_start().len());
        let trimmed = line.trim();
        if indentation != 2 || !trimmed.ends_with(':') || trimmed.starts_with("m_") {
            continue;
        }
        let title = trimmed.trim_end_matches(':').to_string();
        let mut headers = Vec::new();
        let mut rows = Vec::new();
        let mut current: Option<BTreeMap<String, String>> = None;
        for item_line in &lines[start + 1..] {
            let item_indent = item_line.len().saturating_sub(item_line.trim_start().len());
            let item = item_line.trim();
            if !item.is_empty() && item_indent <= indentation && !item.starts_with("- ") {
                break;
            }
            let field = if item_indent == indentation && item.starts_with("- ") {
                if let Some(row) = current.take() {
                    rows.push(row);
                }
                item.trim_start_matches("- ")
            } else if item_indent > indentation {
                item
            } else {
                continue;
            };
            if let Some((key, value)) = field.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    if !headers.contains(&key) {
                        headers.push(key.clone());
                    }
                    current.get_or_insert_with(BTreeMap::new).insert(key, value);
                }
            }
        }
        if let Some(row) = current {
            rows.push(row);
        }
        if rows.len() >= 2 && headers.len() >= 2 {
            return Some(YamlTable {
                title,
                headers,
                rows,
                column_types: BTreeMap::new(),
                column_enums: BTreeMap::new(),
            });
        }
    }
    None
}

fn table_column_widths(table: &YamlTable) -> Vec<f32> {
    table
        .headers
        .iter()
        .map(|header| {
            let widest = table
                .rows
                .iter()
                .filter_map(|row| row.get(header))
                .map(|value| yaml_strings::display(value).chars().count())
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or(8);
            (widest as f32 * 7.5 + 18.0).clamp(92.0, 360.0)
        })
        .collect()
}

fn show_table_row(
    ui: &mut egui::Ui,
    table: &YamlTable,
    widths: &mut [f32],
    row: Option<&BTreeMap<String, String>>,
    allow_resize: bool,
) {
    const CELL_HEIGHT: f32 = 24.0;
    let highlight = editing::placeholder(ui);
    let row_response = ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.horizontal(|ui| {
            for (column, header) in table.headers.iter().enumerate() {
                let raw_text = row
                    .and_then(|values| values.get(header))
                    .map_or(header.as_str(), String::as_str);
                let display_text = if row.is_some() { yaml_strings::display(raw_text) }
                    else { std::borrow::Cow::Borrowed(raw_text) };
                let text = display_text.as_ref();
                let is_header = row.is_none();
                let alignment = if is_header || text.parse::<f64>().is_ok() {
                    egui::Align::Center
                } else {
                    egui::Align::Min
                };
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(widths[column], CELL_HEIGHT),
                    egui::Sense::hover(),
                );
                let clipped_painter = ui
                    .painter()
                    .with_clip_rect(rect.shrink2(egui::vec2(6.0, 0.0)));
                let text_position = if alignment == egui::Align::Min {
                    egui::pos2(rect.left() + 6.0, rect.center().y)
                } else {
                    rect.center()
                };
                let typed = !is_header && ui.push_id(header, |ui| {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(6.0, 2.5)))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)));
                    child.set_clip_rect(ui.clip_rect().intersect(rect));
                    if header == "索引" && editing::dirty_scope(&child) {
                        child.label(raw_text);
                        if child.small_button("还原").clicked() { editing::restore_scope(&child); }
                        true
                    } else { script_properties::show_table_cell(&mut child, header, raw_text,
                        table.column_types.get(header).map(String::as_str), table.column_enums.get(header).map(Vec::as_slice)) }
                }).inner;
                if !typed { clipped_painter.text(
                    text_position,
                    if alignment == egui::Align::Min {
                        egui::Align2::LEFT_CENTER
                    } else {
                        egui::Align2::CENTER_CENTER
                    },
                    text,
                    if is_header {
                        egui::FontId::proportional(14.0)
                    } else {
                        egui::FontId::monospace(14.0)
                    },
                    ui.visuals().text_color(),
                ); }
                response.on_hover_text(if text == raw_text { text.to_string() }
                    else { format!("{text}\n原始 YAML：{raw_text}") });
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                    egui::StrokeKind::Inside,
                );

                if allow_resize {
                    let handle_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.right(), rect.center().y),
                        egui::vec2(8.0, CELL_HEIGHT),
                    );
                    let handle = ui.interact(
                        handle_rect,
                        ui.id().with(("table-column-resize", header)),
                        egui::Sense::drag(),
                    );
                    if handle.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if handle.dragged() {
                        let delta = ui.input(|input| input.pointer.delta().x);
                        widths[column] = (widths[column] + delta).clamp(72.0, 640.0);
                        ui.ctx().request_repaint();
                    }
                }
            }
        });
    });
    if row.is_some() { editing::highlight(ui, highlight, row_response.response.rect, editing::dirty_scope(ui)); }
}

fn show_yaml_table(
    ui: &mut egui::Ui,
    table: &YamlTable,
    freeze_header: bool,
    widths: &mut [f32],
    horizontal_offset: f32,
) -> f32 {
    if editing::active(ui) {
        if let Some(index) = table.headers.iter().position(|header| header == "索引") { widths[index] = widths[index].max(100.0); }
    }
    ui.label(RichText::new(format!("{}（{} 条）", table.title, table.rows.len())).strong());
    let table_width = widths.iter().sum::<f32>();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if freeze_header {
            ScrollArea::horizontal()
                .id_salt(("yaml-table-header", &table.title))
                .horizontal_scroll_offset(horizontal_offset)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.set_min_width(table_width);
                    show_table_row(ui, table, widths, None, true);
                });
        }
        let output = ScrollArea::both()
            .id_salt(("yaml-table-body", &table.title))
            .auto_shrink([false, true])
            // Keep the header outside the 11.5-row data allowance. Account
            // for a horizontal scrollbar so it cannot hide the partial row.
            .max_height(
                table.rows.len().min(11) as f32 * 24.0
                    + if table.rows.len() > 11 { 12.0 } else { 0.0 }
                    + if freeze_header { 0.0 } else { 24.0 }
                    + if table_width > ui.available_width() {
                        ui.spacing().scroll.bar_width
                            + ui.spacing().scroll.bar_inner_margin
                            + ui.spacing().scroll.bar_outer_margin
                    } else { 0.0 }
            )
            .show(ui, |ui| {
                ui.set_min_width(table_width);
                if !freeze_header {
                    show_table_row(ui, table, widths, None, true);
                }
                for (row_index, row) in table.rows.iter().enumerate() {
                    ui.push_id(row_index, |ui| editing::with_row(ui, row_index, |ui| show_table_row(ui, table, widths, Some(row), false)));
                }
            });
        if freeze_header && output.state.offset.x != horizontal_offset {
            ui.ctx().request_repaint();
        }
        output.state.offset.x
    })
    .inner
}

fn custom_list_fields(text: &str) -> Vec<(String, Vec<String>)> {
    let mut fields: Vec<(String, Vec<String>)> = Vec::new();
    let mut active_field = None;
    for line in text.lines() {
        let indentation = line.len().saturating_sub(line.trim_start().len());
        let trimmed = line.trim();
        if indentation == 2 && trimmed.ends_with(':') && !trimmed.starts_with("- ") {
            fields.push((trimmed.trim_end_matches(':').to_string(), Vec::new()));
            active_field = Some(fields.len() - 1);
        } else if indentation == 2 && trimmed.starts_with("- ") {
            if let Some(field_index) = active_field {
                let fallback = trimmed
                    .trim_start_matches("- ")
                    .strip_prefix("path:")
                    .map(str::trim)
                    .unwrap_or("条目")
                    .to_string();
                fields[field_index].1.push(fallback);
            }
        } else if indentation >= 4 && trimmed.starts_with("name:") {
            if let Some(field_index) = active_field {
                if let Some(last) = fields[field_index].1.last_mut() {
                    let name = trimmed.trim_start_matches("name:").trim();
                    if !name.is_empty() {
                        *last = name.to_string();
                    }
                }
            }
        }
    }
    fields.retain(|(_, entries)| !entries.is_empty());
    fields
}

fn build_tree(
    docs: &[Document],
) -> (
    Vec<usize>,
    HashMap<String, Vec<usize>>,
    HashMap<String, Vec<usize>>,
    Vec<usize>,
) {
    let mut transform_to_go = HashMap::new();
    let mut go_indices = HashMap::new();
    for (i, doc) in docs.iter().enumerate() {
        if doc.type_name == "GameObject" {
            go_indices.insert(doc.file_id.clone(), i);
        }
        if doc.type_name == "Transform" || doc.type_name == "RectTransform" {
            if let Some(go) = &doc.game_object {
                transform_to_go.insert(doc.file_id.clone(), go.clone());
            }
        }
    }
    let mut roots = Vec::new();
    let mut children: HashMap<String, Vec<usize>> = HashMap::new();
    let mut components: HashMap<String, Vec<usize>> = HashMap::new();
    let mut unlinked = Vec::new();
    for (i, doc) in docs.iter().enumerate() {
        if doc.type_name == "GameObject" {
            continue;
        }
        if (doc.type_name == "Transform" || doc.type_name == "RectTransform")
            && doc.game_object.is_some()
        {
            let own = doc.game_object.as_ref().unwrap();
            match doc
                .parent_transform
                .as_ref()
                .and_then(|parent| transform_to_go.get(parent))
            {
                Some(parent_go) => children
                    .entry(parent_go.clone())
                    .or_default()
                    .push(*go_indices.get(own).unwrap()),
                None => roots.push(*go_indices.get(own).unwrap()),
            }
            components.entry(own.clone()).or_default().push(i);
        } else if let Some(go) = doc.game_object.as_ref().filter(|go| go.as_str() != "0") {
            components.entry(go.clone()).or_default().push(i);
        } else {
            unlinked.push(i);
        }
    }
    for component_list in components.values_mut() {
        component_list.sort_by_key(|&index| {
            if docs[index].type_name == "Transform" || docs[index].type_name == "RectTransform" {
                0
            } else {
                1
            }
        });
    }
    roots.sort_unstable();
    (roots, children, components, unlinked)
}

fn unified_tab(ui: &mut egui::Ui, title: &str, is_active: bool) -> (bool, bool) {
    let font = egui::FontId::proportional(14.0);
    let text_color = ui.visuals().text_color();
    let text_width = ui
        .painter()
        .layout_no_wrap(title.to_owned(), font.clone(), text_color)
        .size()
        .x;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(text_width + 38.0, 24.0), egui::Sense::click());
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 24.0, rect.top()),
        egui::vec2(24.0, rect.height()),
    );
    let tab_hovered = response.hovered();
    let close_hovered = ui.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|pointer| close_rect.contains(pointer))
    });
    let show_close = is_active || tab_hovered;
    let (fill, stroke) = if is_active {
        (
            ui.visuals().selection.bg_fill,
            ui.visuals().selection.stroke,
        )
    } else if tab_hovered {
        (
            ui.visuals().widgets.hovered.bg_fill,
            ui.visuals().widgets.hovered.bg_stroke,
        )
    } else {
        (Color32::TRANSPARENT, egui::Stroke::NONE)
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    if is_active || tab_hovered {
        ui.painter()
            .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        font,
        text_color,
    );
    if show_close {
        if close_hovered {
            ui.painter().rect_filled(
                close_rect.shrink(3.0),
                3.0,
                ui.visuals().widgets.active.bg_fill,
            );
        }
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(14.0),
            if close_hovered {
                ui.visuals().widgets.active.fg_stroke.color
            } else {
                ui.visuals().weak_text_color()
            },
        );
    }
    (
        response.clicked() && !close_hovered,
        show_close && response.clicked() && close_hovered,
    )
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Unity YAML Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_icon(application_icon()),
            ..Default::default()
        },
        Box::new(|creation_context| {
            configure_system_fonts(&creation_context.egui_ctx);
            restore_theme(&creation_context.egui_ctx);
            let mut app = ViewerApp {
                last_open_dir: load_last_open_dir(),
                settings: AppSettings::load(),
                ..Default::default()
            };
            app.restore_open_tabs();
            Ok(Box::new(app))
        }),
    )
}

fn application_icon() -> Arc<egui::IconData> {
    Arc::new(
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/app_icon_256.png"))
            .expect("无法读取窗口图标"),
    )
}

fn last_open_dir_config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|directory| directory.join("UnityYAMLViewer").join("last_open_dir.txt"))
}

fn load_last_open_dir() -> Option<PathBuf> {
    let path = last_open_dir_config_path()?;
    let value = fs::read_to_string(path).ok()?;
    let directory = PathBuf::from(value.trim());
    directory.is_dir().then_some(directory)
}

fn persist_theme(theme: &str) {
    let Some(path) = last_open_dir_config_path().map(|path| path.with_file_name("theme.txt")) else { return };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() { return; }
    }
    let _ = fs::write(path, theme.as_bytes());
}

fn restore_theme(ctx: &egui::Context) {
    let theme = last_open_dir_config_path()
        .and_then(|path| fs::read_to_string(path.with_file_name("theme.txt")).ok());
    match theme.as_deref().map(str::trim) {
        Some("light") => {
            ctx.set_theme(egui::Theme::Light);
            apply_light_background(ctx);
        }
        Some("dark") => ctx.set_theme(egui::Theme::Dark),
        _ => ctx.set_theme(egui::ThemePreference::System),
    }
}

fn encode_open_tabs(paths: &[PathBuf], active: Option<&std::path::Path>) -> String {
    let mut text = "UnityYAMLViewer open tabs v1\n".to_string();
    if let Some(active) = active { text.push_str(&format!("active\t{}\n", active.display())); }
    for path in paths { text.push_str(&format!("file\t{}\n", path.display())); }
    text
}

fn decode_open_tabs(text: &str) -> (Vec<PathBuf>, Option<PathBuf>) {
    let mut lines = text.lines();
    if lines.next() != Some("UnityYAMLViewer open tabs v1") { return (Vec::new(), None); }
    let mut paths = Vec::new();
    let mut active = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("active\t").filter(|value| !value.is_empty()) { active = Some(PathBuf::from(value)); }
        if let Some(value) = line.strip_prefix("file\t").filter(|value| !value.is_empty()) {
            let path = PathBuf::from(value);
            if !paths.contains(&path) { paths.push(path); }
        }
    }
    (paths, active)
}

#[cfg(test)]
mod open_tab_session_tests {
    use super::*;

    #[test]
    fn session_preserves_unicode_paths_order_and_active_tab() {
        let paths = vec![PathBuf::from("C:/工程/资源/ui card.prefab"), PathBuf::from("C:/工程/材质.mat")];
        let session = encode_open_tabs(&paths, Some(&paths[0]));
        let (restored, active) = decode_open_tabs(&session);
        assert_eq!(restored, paths);
        assert_eq!(active, Some(paths[0].clone()));
        assert_eq!(decode_open_tabs(&encode_open_tabs(&[], None)), (Vec::new(), None));
        assert_eq!(decode_open_tabs("invalid session"), (Vec::new(), None));
    }
}

fn configure_system_fonts(ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];

    for path in candidates {
        if let Ok(data) = fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "system_cjk".to_owned(),
                Arc::new(egui::FontData::from_owned(data)),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "system_cjk".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "system_cjk".to_owned());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

fn apply_light_background(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    let background = Color32::from_rgb(0xe2, 0xe2, 0xe2);
    visuals.panel_fill = background;
    visuals.window_fill = background;
    visuals.faint_bg_color = Color32::from_rgb(0xe8, 0xe8, 0xe8);
    visuals.widgets.noninteractive.bg_fill = background;
    visuals.extreme_bg_color = Color32::from_rgb(0xf2, 0xf2, 0xf2);
    ctx.set_visuals(visuals);
}
