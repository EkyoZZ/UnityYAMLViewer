#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    sync::{Arc, mpsc, atomic::{AtomicBool, AtomicUsize, Ordering}},
};

use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit};
use encoding_rs::{GBK, UTF_8};
use regex::Regex;

mod script_properties;
mod color_property;
mod controller_graph;
mod material_graph;
mod preview_layout;
mod gradient_property;
mod yaml_strings;
mod editing;
mod localization;
mod localized_format;
mod theme;
mod icons;
mod asset_viewer;
mod media_tools;
mod media_trim;
mod audio_edit;
mod media_preview;
mod spine_preview;
mod particle_preview;
mod model_preview;
mod render_system;
mod preview_fonts;
mod recent_files;
mod remote_file;
mod native_window;
mod timeline_edit;
mod animation_curves;
mod updater;
mod static_preview;
mod tilemap;
mod ui_preview;
mod hierarchy_controls;
mod command_line;
mod file_associations;
mod media_history;
mod playlist;
mod app_cache;
mod release_notes;
use script_properties::ScriptProperties;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct AppSettings { theme: String, language: String, undo_steps: usize, auto_check_updates: bool, auto_open_preview: bool, hierarchy_depth: String }
impl Default for AppSettings {
    fn default() -> Self { Self { theme: "system".into(), language: "zh-CN".into(), undo_steps: editing::DEFAULT_UNDO_LIMIT, auto_check_updates: true, auto_open_preview: true, hierarchy_depth: "3".into() } }
}
impl AppSettings {
    fn load() -> Self {
        let mut settings = Self::default();
        if let Some(path) = last_open_dir_config_path() {
            if let Ok(value) = fs::read_to_string(path.with_file_name("hierarchy_depth.txt")) {
                if matches!(value.trim(), "1" | "2" | "3" | "all") {settings.hierarchy_depth = value.trim().into();}
            }
            if let Ok(value) = fs::read_to_string(path.with_file_name("auto_open_preview.txt")) {
                settings.auto_open_preview = value.trim() != "false";
            }
            if let Ok(value) = fs::read_to_string(path.with_file_name("auto_check_updates.txt")) {
                settings.auto_check_updates = value.trim() != "false";
            }
            if let Ok(language) = fs::read_to_string(path.with_file_name("language.txt")) {
                if localization::LANGUAGES.iter().any(|(code, _)| *code == language.trim()) {
                    settings.language = language.trim().into();
                }
            }
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
        if let Some(path) = last_open_dir_config_path().map(|p| p.with_file_name("hierarchy_depth.txt")) {
            if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::write(path, self.hierarchy_depth.as_bytes());
        }
        if let Some(path) = last_open_dir_config_path().map(|p| p.with_file_name("auto_open_preview.txt")) {
            if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::write(path, self.auto_open_preview.to_string());
        }
        if let Some(path) = last_open_dir_config_path().map(|p| p.with_file_name("auto_check_updates.txt")) {
            if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::write(path, self.auto_check_updates.to_string());
        }
        if let Some(path) = last_open_dir_config_path().map(|path| path.with_file_name("language.txt")) {
            if let Some(parent) = path.parent() { if fs::create_dir_all(parent).is_err() { return; } }
            let _ = fs::write(path, self.language.as_bytes());
        }
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
    start_offset: usize,
    text: Arc<str>,
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
    time_lines: Vec<usize>,
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
    command_inbox: Option<command_line::Inbox>,
    path: Option<PathBuf>,
    last_open_dir: Option<PathBuf>,
    raw: Arc<str>,
    documents: Vec<Document>,
    selected: Option<usize>,
    locate_target: Option<usize>,
    last_selected: Option<usize>,
    query: String,
    show_search: bool,
    show_supported_formats: bool,
    show_command_help: bool,
    file_associations: file_associations::Dialog,
    cache_panel: app_cache::Panel,
    show_url: bool,
    url_input: String,
    url_error: Option<String>,
    url_job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    status: String,
    encoding: String,
    selected_keyframe: Option<SelectedKeyframe>,
    selected_animation_track: Option<AnimationTrackSelection>,
    show_custom_table: bool,
    show_transform_yaml: bool,
    show_game_object_yaml: bool,
    show_script_yaml: bool,
    show_sprite_renderer_yaml: bool,
    show_collider_yaml: bool,
    show_animator_yaml: bool,
    show_animator_state_yaml: bool,
    show_material_yaml: bool,
    show_sprite_atlas_yaml: bool,
    script_properties: ScriptProperties,
    freeze_table_header: bool,
    table_column_widths: HashMap<String, Vec<f32>>,
    table_horizontal_offsets: HashMap<String, f32>,
    tabs: Vec<ViewerTab>,
    active_tab: usize,
    tab_group_active: HashMap<String, PathBuf>,
    close_group_queue: std::collections::VecDeque<PathBuf>,
    closing_tab_group: Option<String>,
    show_about: bool,
    show_older_history: bool,
    restoring_session: bool,
    edits: HashMap<PathBuf, editing::Shared>,
    saved_histories: HashMap<PathBuf, editing::Shared>,
    pending_save: Option<PathBuf>,
    pending_close_tab: Option<PathBuf>,
    pending_playlist_close: Option<PathBuf>,
    playlist_exit_requested: bool,
    show_settings: bool,
    settings: AppSettings,
    decoder_settings: media_tools::DecoderSettings,
    updater: updater::Updater,
    preview: static_preview::Preview,
    preview_text: Arc<str>,
    preview_documents: Vec<Document>,
    nested_selected: Option<(PathBuf,String)>,
    node_controls: HashMap<PathBuf, hierarchy_controls::NodeControls>,
    node_flags: HashMap<String, hierarchy_controls::Flags>,
    assets: asset_viewer::Assets,
    recent_files: recent_files::RecentFiles,
    load_queue: std::collections::VecDeque<PathBuf>,
    loading: Option<FileLoad>,
    restore_active: Option<PathBuf>,
    load_selection: HashMap<PathBuf,String>,
    load_reload: std::collections::HashSet<PathBuf>,
}

struct FileLoad {
    path: PathBuf,
    receiver: mpsc::Receiver<Result<(Arc<str>,Vec<Document>,bool),String>>,
    cancelled: Arc<AtomicBool>,
    stage: Arc<AtomicUsize>,
    progress: Arc<AtomicUsize>,
    started: std::time::Instant,
}

#[derive(Clone)]
struct ViewerTab {
    path: PathBuf,
    raw: Arc<str>,
    documents: Vec<Document>,
    selected: Option<usize>,
    status: String,
    encoding: String,
}

impl ViewerApp {
    fn poll_file_load(&mut self, ctx: &egui::Context) {
        if let Some(job)=&self.loading {
            let result=match job.receiver.try_recv() {
                Ok(result)=>Some(result),
                Err(mpsc::TryRecvError::Disconnected)=>Some(Err("Background file loader stopped".into())),
                Err(mpsc::TryRecvError::Empty)=>None,
            };
            if let Some(result)=result {
                let job=self.loading.take().unwrap();
                if !job.cancelled.load(Ordering::Relaxed) {
                    match result {
                        Ok((text,documents,gbk))=> {
                            let selected=self.load_selection.remove(&job.path).and_then(|id|documents.iter().position(|doc|doc.file_id==id));
                            if self.load_reload.remove(&job.path) {self.saved_histories.remove(&job.path);self.assets.forget(&job.path);}
                            let tab=ViewerTab {path:job.path,raw:text,status:crate::lf!("已读取 {0} 个 Unity 序列化对象（只读模式）",documents.len()),documents,selected,encoding:if gbk {crate::localization::tr("GBK（检测到无效 UTF-8 字节）").into()} else {"UTF-8".into()}};
                            self.save_active_tab();
                            let index=if let Some(index)=self.tabs.iter().position(|existing|existing.path==tab.path) {
                                for doc in &self.tabs[index].documents {tilemap::forget(ctx,&doc.file_id,&doc.text);}
                                self.tabs[index]=tab;index
                            } else {self.tabs.push(tab);self.tabs.len()-1};
                            self.activate_tab(index);
                            self.show_custom_table=true;self.remember_recent_file();
                        },
                        Err(error)=>{self.load_selection.remove(&job.path);self.load_reload.remove(&job.path);self.status=crate::lf!("无法读取文件：{0}",error);},
                    }
                } else {
                    if let Ok((_,documents,_))=&result {for doc in documents {tilemap::forget(ctx,&doc.file_id,&doc.text);}}
                    self.load_selection.remove(&job.path);self.load_reload.remove(&job.path);
                }
            }
        }
        if self.loading.is_none() {
            if let Some(path)=self.load_queue.pop_front() {
                let (send,receiver)=mpsc::channel();
                let cancelled=Arc::new(AtomicBool::new(false));let stage=Arc::new(AtomicUsize::new(0));let progress=Arc::new(AtomicUsize::new(0));
                let worker_cancelled=cancelled.clone();let worker_stage=stage.clone();let worker_path=path.clone();let worker_ctx=ctx.clone();
                let worker_progress=progress.clone();
                std::thread::spawn(move|| {
                    let result=(|| {
                        use std::io::Read;
                        let mut file=fs::File::open(&worker_path).map_err(|error|error.to_string())?;
                        let length=file.metadata().map_err(|error|error.to_string())?.len();
                        let mut bytes=Vec::new();
                        bytes.try_reserve_exact(usize::try_from(length).map_err(|error|error.to_string())?).map_err(|error|error.to_string())?;
                        let mut chunk=vec![0u8;1024*1024];
                        loop {
                            if worker_cancelled.load(Ordering::Relaxed) {return Err("Cancelled".into());}
                            let count=file.read(&mut chunk).map_err(|error|error.to_string())?;
                            if count==0 {break;}
                            bytes.extend_from_slice(&chunk[..count]);
                            worker_progress.store(((bytes.len() as u128*1000)/length.max(1) as u128).min(1000) as usize,Ordering::Relaxed);
                        }
                        if worker_cancelled.load(Ordering::Relaxed) {return Err("Cancelled".into());}
                        let (text,gbk)=match String::from_utf8(bytes) {
                            Ok(text)=>(text,false),
                            Err(error)=>(GBK.decode(error.as_bytes()).0.into_owned(),true),
                        };
                        worker_progress.store(0,Ordering::Relaxed);worker_stage.store(1,Ordering::Relaxed);worker_ctx.request_repaint();
                        let documents=parse_unity_yaml_with_progress(&text,Some(&worker_cancelled),Some(&worker_progress));
                        worker_progress.store(1000,Ordering::Relaxed);
                        Ok((Arc::from(text),documents,gbk))
                    })();let _=send.send(result);worker_ctx.request_repaint();
                });
                self.loading=Some(FileLoad{path,receiver,cancelled,stage,progress,started:std::time::Instant::now()});
            } else if self.restoring_session {
                if let Some(index)=self.restore_active.take().and_then(|path|self.tabs.iter().position(|tab|tab.path==path)) {self.activate_tab(index);}
                self.restoring_session=false;self.persist_open_tabs();
            }
        }
        if let Some(job)=&self.loading {
            if job.started.elapsed()>=std::time::Duration::from_secs(1) {
            crate::native_window::Window::new(crate::localization::tr("正在加载文件")).id(egui::Id::new("file_loading"))
                .title_bar(false).collapsible(false).resizable(false).movable(false)
                .anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).fixed_size(egui::vec2(420.0,72.0)).show(ctx,|ui| {
                let cancelled=job.cancelled.load(Ordering::Relaxed);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(crate::localization::tr(if cancelled {"正在取消…"} else if job.stage.load(Ordering::Relaxed)==0 {"读取中…"} else {"解析中…"})).strong())
                        .on_hover_text(job.path.display().to_string());
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let width=(ui.available_width()-72.0-ui.spacing().item_spacing.x).max(1.0);
                    ui.add(egui::ProgressBar::new(job.progress.load(Ordering::Relaxed) as f32/1000.0).desired_width(width).desired_height(22.0));
                    if ui.add_enabled(!cancelled,egui::Button::new(crate::localization::tr("取消")).min_size(egui::vec2(72.0,28.0))).clicked() {
                        job.cancelled.store(true,Ordering::Relaxed);
                    }
                });
            });
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn refresh_node_flags(&mut self) {
        let expanded=self.preview.hierarchy_documents(&self.path.as_ref().map(|path|path.display().to_string()).unwrap_or_default());
        self.node_flags = self.path.as_ref().and_then(|path| self.node_controls.get(path))
            .map(|controls| controls.effective(expanded.as_ref().map_or(self.documents.as_slice(),|data|data.documents.as_slice()))).unwrap_or_default();
        if self.selected.is_some_and(|index| !self.document_flags(index).selectable) {
            self.selected = None;
            self.last_selected = None;
        }
    }

    fn document_flags(&self, index: usize) -> hierarchy_controls::Flags {
        self.documents.get(index).and_then(|doc| {
            let id = if doc.type_name == "GameObject" { Some(doc.file_id.as_str()) } else { doc.game_object.as_deref() };
            id.and_then(|id| self.node_flags.get(id)).copied()
        }).unwrap_or_default()
    }

    fn update_windows(&mut self, ctx: &egui::Context) {
        self.url_window(ctx);
        if self.show_command_help {
            native_window::Window::new(localization::tr("命令行用法")).id(egui::Id::new("command_help")).open(&mut self.show_command_help).default_width(760.).default_height(600.).solid_scroll().show(ctx,|ui| {
                ui.label(localization::tr("slice9 默认原位替换，不创建备份；建议先用 --dry-run 查看方案。"));
                ui.label(localization::tr("简单路径可不加引号；包含空格或命令行特殊字符时，请分别给每个路径加引号。"));
                if ui.button(localization::tr("复制指令说明")).clicked() {ui.ctx().copy_text(asset_viewer::nine_slice::cli::HELP.to_owned());}
                ui.label(egui::RichText::new(asset_viewer::nine_slice::cli::HELP).monospace());
            });
        }
        if self.show_supported_formats {
            native_window::Window::new(localization::tr("支持的文件格式"))
                .id(egui::Id::new("supported_formats")).open(&mut self.show_supported_formats)
                .default_width(560.0).fit_height().show(ctx, |ui| {
                    for &(label, extensions) in file_associations::FORMAT_GROUPS {
                        ui.strong(localization::tr(label));
                        ui.add(egui::Label::new(extensions).wrap());
                        ui.add_space(6.0);
                    }
                    ui.add(egui::Label::new(localization::tr("音视频需要 FFmpeg；Spine 文件需要有效骨骼数据和配套资源。.bytes 中的 MP4 可按文件头识别。后缀列表不代表所有内容均可完整预览；代码文件仅作文本查看。")).wrap());
                    ui.separator();
                    if ui.button(localization::tr("设置默认打开程序…")).clicked() {
                        self.file_associations.open(ui.ctx());
                    }
                });
        }
        self.file_associations.show(ctx);
        if self.updater.show {
            let mut open = true;
            crate::native_window::Window::new(localization::tr("软件更新")).id(egui::Id::new("update_release")).fit_height()
                .open(&mut open).collapsible(false).default_width(480.0).show(ctx, |ui| {
                    if let Some(release) = self.updater.release.clone() {
                        ui.label(format!("{} {} → {}", localization::tr("版本"), APP_VERSION, release.tag_name));
                        ui.separator();
                        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            self.updater.notes.show(ui, release.body.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or(localization::tr("此版本未提供更新说明。")));
                        });
                        ui.separator();
                        ui.hyperlink_to(localization::tr("打开发布页面，手动更新"), release.page_url());
                        if self.updater.downloading {
                            let (done, total) = self.updater.progress;
                            let fraction = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                            ui.add(egui::ProgressBar::new(fraction).show_percentage().text(format!("{:.1} / {:.1} MB", done as f64 / 1048576.0, total as f64 / 1048576.0)));
                            ui.label(localization::tr("正在下载并校验更新文件…"));
                            ui.horizontal(|ui| {
                                ui.add_space(((ui.available_width()-240.0-ui.spacing().item_spacing.x)*0.5).max(0.0));
                                ui.add_enabled_ui(false, |ui| ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("更新"))));
                                if ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("取消"))).clicked() { self.updater.cancel_download(); }
                            });
                        } else {
                            if release.asset().is_none() { ui.label(localization::tr("此版本没有兼容的 Windows 更新附件。")); }
                            ui.horizontal(|ui| {
                                ui.add_space(((ui.available_width()-240.0-ui.spacing().item_spacing.x)*0.5).max(0.0));
                                if ui.add_enabled_ui(release.asset().is_some(), |ui| ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("更新")))).inner.clicked() {
                                    if self.updater.staged.is_some() { self.updater.confirm = true; }
                                    else { self.updater.download(ctx); }
                                }
                                if ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("取消"))).clicked() { self.updater.show = false; }
                            });
                        }
                    }
                });
            if !open { self.updater.show = false; }
        }
        if self.updater.confirm {
            let mut open = true;
            let dirty: Vec<_> = self.edits.iter().filter(|(_, session)| session.lock().unwrap().has_changes()).map(|(path, _)| path.clone()).collect();
            crate::native_window::Window::new(localization::tr("确认更新并重启")).id(egui::Id::new("confirm_update_restart"))
                .open(&mut open).collapsible(false).resizable(false).default_width(320.0).min_width(320.0).max_width(320.0).show(ctx, |ui| {
                    ui.set_width(320.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(localization::tr("更新已下载并通过校验。请先保存未保存的内容，确认后将替换程序并重启。")).wrap());
                    if !dirty.is_empty() {
                        ui.add(egui::Label::new(RichText::new(localization::tr("以下文件尚未保存，请取消更新并保存后再确认：")).color(Color32::RED)).wrap());
                        ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            for path in &dirty { ui.add(egui::Label::new(path.display().to_string()).wrap()); }
                        });
                    }
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.add_space(((ui.available_width()-240.0-ui.spacing().item_spacing.x)*0.5).max(0.0));
                        if ui.add_enabled_ui(dirty.is_empty(), |ui| ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("确认")))).inner.clicked() {
                            if let Some(path) = self.updater.staged.as_ref() {
                                match updater::install(path) {
                                    Ok(()) => { self.persist_open_tabs(); std::process::exit(0); }
                                    Err(error) => { self.updater.message = Some(format!("{} {error}", localization::tr("更新失败："))); self.updater.confirm = false; }
                                }
                            }
                        }
                        if ui.add_sized([120.0,36.0],egui::Button::new(localization::tr("取消"))).clicked() { self.updater.confirm = false; }
                    });
                    ui.add_space(8.0);
                });
            if !open { self.updater.confirm = false; }
        }
        if let Some(message) = self.updater.message.clone() {
            let mut open = true;
            crate::native_window::Window::new(localization::tr("软件更新")).id(egui::Id::new("update_message"))
                .open(&mut open).collapsible(false).resizable(false).default_width(260.0).min_width(260.0).max_width(260.0).show(ctx, |ui| {
                    ui.add_space(16.0);
                    ui.set_width(260.0);
                    ui.add_sized(egui::vec2(ui.available_width(), 0.0), egui::Label::new(localization::tr(&message)).wrap().halign(egui::Align::Center));
                    if let Some(release) = &self.updater.release {
                        ui.add_space(8.0);
                        ui.hyperlink_to(localization::tr("打开发布页面，手动更新"), release.page_url());
                    } else {
                        ui.add_space(8.0);
                        ui.hyperlink_to(localization::tr("打开发布页面，手动更新"), "https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest");
                    }
                    ui.add_space(20.0);
                    let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::hover());
                    let button_rect = egui::Rect::from_center_size(row.center(), egui::vec2(112.0, 34.0));
                    if ui.put(button_rect, egui::Button::new(localization::tr("确定"))).clicked() { self.updater.message = None; }
                    ui.add_space(10.0);
                });
            if !open { self.updater.message = None; }
        }
    }

    fn persist_open_tabs(&self) {
        if self.restoring_session { return; }
        let Some(config_path) = last_open_dir_config_path().map(|path| path.with_file_name("open_tabs.txt")) else { return };
        let Some(parent) = config_path.parent() else { return };
        if fs::create_dir_all(parent).is_err() { return; }
        let session = encode_open_tabs(&self.tabs.iter().filter(|tab| !remote_file::is_cached(&tab.path)).map(|tab| tab.path.clone()).collect::<Vec<_>>(), self.path.as_deref());
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
        if let Some(index) = active.as_ref().and_then(|path| self.tabs.iter().position(|tab| &tab.path == path)) {
            self.activate_tab(index);
        }
        if self.load_queue.is_empty() {
            self.restoring_session = false;
            self.persist_open_tabs();
        } else {
            self.restore_active = active;
        }
    }

    fn open_external_reference(&mut self, path: &std::path::Path) {
        if !path.is_file() {
            self.status = crate::lf!("引用文件不存在：{0}", path.display());
            return;
        }
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
        if matches!(extension.as_str(), "exe" | "com" | "bat" | "cmd" | "ps1" | "msi" | "lnk" | "vbs" | "js") {
            self.status = crate::localization::tr("此引用为可执行文件或脚本，不支持直接打开。").to_string();
            return;
        }
        match std::process::Command::new("explorer.exe").arg(path).spawn() {
            Ok(_) => self.status = crate::lf!("已交给系统默认程序打开：{0}", path.display()),
            Err(error) => self.status = crate::lf!("无法打开引用：{0}", error),
        }
    }

    fn search_field(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 2.0;
        let width = (ui.available_width() - 28.0 - ui.spacing().item_spacing.x).clamp(32.0, 420.0);
        let response = ui.add_sized([width, 28.0], TextEdit::singleline(&mut self.query).vertical_align(egui::Align::Center)
            .hint_text(localization::tr("名称、组件、字段值、fileID 或 GUID")));
        let clear = icons::button(ui, icons::Icon::Close, localization::tr("清除查找")).clicked();
        if clear { self.query.clear(); }
        if response.changed() || clear {
            let query = self.query.trim().to_lowercase();
            if query.is_empty() {
                self.selected = self.selected.or(self.last_selected);
                self.locate_target = self.selected;
            } else {
                self.selected = self.documents.iter().position(|document|
                    self.display_name(document).to_lowercase().contains(&query) || document.text.to_lowercase().contains(&query));
            }
        }
    }

    fn url_window(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.url_job {
            match job.try_recv() {
                Ok(result) => {
                    self.url_job = None;
                    match result {
                        Ok(path) => { self.show_url = false; self.open_file_with_directory_tracking(path, false); }
                        Err(error) => { self.url_error = Some(error); self.show_url = true; }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.url_job = None; self.url_error = Some(localization::tr("网络加载任务已中断").into()); }
                Err(mpsc::TryRecvError::Empty) => { ctx.request_repaint_after(std::time::Duration::from_millis(100)); }
            }
        }
        if !self.show_url { return; }
        let mut open = true;
        let mut submit = false;
        native_window::Window::new(localization::tr("打开 URL")).id(egui::Id::new("open_url"))
            .open(&mut open).default_width(620.0).fit_height().show(ctx, |ui| {
                ui.label(localization::tr("输入 HTTP / HTTPS 文件直链，下载完成后打开（最大 1 GiB）。"));
                let pending = self.url_job.is_some();
                ui.horizontal(|ui| {
                    let input_width = (ui.available_width() - 104.0 - ui.spacing().item_spacing.x).max(40.0);
                    let response = ui.add_enabled_ui(!pending, |ui| ui.add_sized([input_width, 32.0],
                        TextEdit::singleline(&mut self.url_input).vertical_align(egui::Align::Center).hint_text("https://"))).inner;
                    submit = ui.add_enabled_ui(!pending, |ui| ui.add_sized([104.0, 32.0],
                        egui::Button::new(localization::tr("确定")))).inner.clicked()
                        || (!pending && response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                });
                if let Some(error) = &self.url_error { ui.colored_label(ui.visuals().error_fg_color, error); }
                if pending { ui.horizontal(|ui| { ui.spinner(); ui.label(localization::tr("正在网络加载…")); }); }
            });
        if !open { self.show_url = false; }
        if submit {
            self.url_error = None;
            self.url_job = Some(remote_file::start(self.url_input.trim().to_owned(), ctx.clone()));
        }
    }

    fn show_open_dialog(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter("All files", &["*"]).add_filter(
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
        ).add_filter("Spine 3.8", &["skel", "bytes", "json", "txt"])
        .add_filter(crate::localization::tr("图片"), &["png", "jpg", "jpeg", "tga", "bmp", "webp"])
        .add_filter(crate::localization::tr("模型"), &["fbx", "obj"])
        .add_filter(crate::localization::tr("音频"), &["mp3", "wav", "ogg", "oga", "flac", "aac", "m4a", "wma", "opus", "aiff", "aif"])
        .add_filter(crate::localization::tr("播放列表"), &["m3u8", "m3u"])
        .add_filter(crate::localization::tr("视频"), &["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpeg", "mpg", "ts", "mts", "m2ts", "3gp", "bytes"]);
        let dialog = if let Some(directory) = &self.last_open_dir {
            dialog.set_directory(directory)
        } else {
            dialog
        };
        if let Some(path) = dialog.pick_file() {
            self.open_file(path);
        }
    }

    fn accept_open_request(&mut self,request:command_line::OpenRequest,ctx:&egui::Context) {
        let mut omitted=0;
        for path in request.paths {
            if let Some(settings)=&request.preview {
                if !self.assets.open_slice_preview(&path,settings.clone(),ctx){omitted+=1;continue}
            }
            // Preview dialogs must survive opening another image tab.
            if request.preview.is_none(){self.open_file(path);}else{self.open_file_with_directory_tracking(path,false);}
        }
        let notice=format!("{}{}",request.notice,if omitted>0 {format!("\n{}: {omitted}",localization::tr("超出预览上限，未打开"))}else{String::new()});
        if !notice.trim().is_empty(){rfd::MessageDialog::new().set_title(localization::tr("九宫格预览" )).set_description(notice).show();}
    }
    fn open_file(&mut self, path: PathBuf) {
        self.open_file_with_directory_tracking(path, true);
    }

    fn open_file_with_directory_tracking(&mut self, path: PathBuf, remember_directory: bool) {
        if remote_file::is_cached(&path) { /* Cached network files do not change the local open directory. */ }
        else if remember_directory && !self.restoring_session {
            self.last_open_dir = path.parent().map(PathBuf::from);
            self.persist_last_open_dir();
        }
        if let Some(index) = self.tabs.iter().position(|tab| tab.path == path) {
            self.save_active_tab();
            self.activate_tab(index);
            self.assets.request_media_playback(&path);
            self.remember_recent_file();
            return;
        }
        if (playlist::is_playlist(&path) || media_preview::is_media(&path) || asset_viewer::is_image(&path) || (spine_preview::is_spine(&path) || model_preview::is_model(&path))) && path.is_file() {
            self.save_active_tab();
            self.tabs.push(ViewerTab { path, raw: "".into(), documents: Vec::new(), selected: None, status: String::new(), encoding: String::new() });
            self.activate_tab(self.tabs.len() - 1);
            self.remember_recent_file();
            return;
        }
        if !self.load_queue.contains(&path) && !self.loading.as_ref().is_some_and(|job|job.path==path) {
            self.load_queue.push_back(path);
        }
    }
    fn remember_recent_file(&mut self) {
        if !self.restoring_session {
            if let Some(path) = &self.path { if !remote_file::is_cached(path) { self.recent_files.record(path.clone()); self.recent_files.persist(); } }
        }
    }

    fn reload_current_file(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return; };
        if playlist::is_playlist(&path) || media_preview::is_media(&path) || asset_viewer::is_image(&path) || (spine_preview::is_spine(&path) || model_preview::is_model(&path)) { self.assets.forget(&path); ctx.request_repaint(); return; }
        if self.edits.contains_key(&path) { return; }
        if !self.load_queue.contains(&path) && !self.loading.as_ref().is_some_and(|job|job.path==path) {
            if let Some(id)=self.selected.and_then(|index|self.documents.get(index)).map(|doc|doc.file_id.clone()) {self.load_selection.insert(path.clone(),id);}
            self.load_reload.insert(path.clone());self.load_queue.push_back(path);
            ctx.request_repaint();
        }
    }
    fn playlist_close_dialog(&mut self,ctx:&egui::Context){
        if self.playlist_exit_requested && self.pending_playlist_close.is_none(){
            if let Some(path)=self.assets.playlist_dirty_paths().into_iter().next(){self.pending_playlist_close=Some(path);}
            else{self.playlist_exit_requested=false;ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
        }
        let Some(path)=self.pending_playlist_close.clone() else{return;};
        let mut save=false;let mut discard=false;let mut cancel=false;
        native_window::Window::new(localization::tr("播放列表有未保存的修改")).id(egui::Id::new("playlist_close_confirm")).modal().resizable(false).default_width(380.0).show(ctx,|ui|{
            ui.label(path.display().to_string());ui.label(localization::tr("保存列表修改后关闭，或放弃本次列表编辑？"));
            if let Some(error)=self.assets.playlist_error(&path){ui.colored_label(ui.visuals().warn_fg_color,error);}
            ui.horizontal(|ui|{save=ui.button(localization::tr("保存列表")).clicked();discard=ui.button(localization::tr("放弃修改")).clicked();cancel=ui.button(localization::tr("取消")).clicked();});
        });
        if cancel{self.pending_playlist_close=None;self.playlist_exit_requested=false;self.close_group_queue.clear();self.closing_tab_group=None;}
        else if (save||discard)&&self.assets.resolve_playlist_dirty(&path,save){
            self.pending_playlist_close=None;
            if !self.playlist_exit_requested{if let Some(index)=self.tabs.iter().position(|tab|tab.path==path){self.close_tab(index,ctx);}}
            ctx.request_repaint();
        }
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
        self.assets.pause_media();
        self.locate_target = None;
        let tab = self.tabs[index].clone();
        self.tab_group_active.insert(tab_group_key(&tab.path), tab.path.clone());
        self.active_tab = index;
        self.path = Some(tab.path);
        self.raw = tab.raw;
        self.documents = tab.documents;
        self.preview.open = self.settings.auto_open_preview
            && (static_preview::Preview::available(&self.documents, self.path.as_deref()) || material_graph::available(self.path.as_deref()));
        self.selected = tab.selected;
        self.last_selected = tab.selected;
        self.status = tab.status;
        self.encoding = tab.encoding;
        self.persist_open_tabs();
    }

    fn close_tab(&mut self, mut index: usize, ctx: &egui::Context) {
        if self.assets.playlist_dirty(&self.tabs[index].path){self.pending_playlist_close=Some(self.tabs[index].path.clone());return;}
        if let Some(session) = self.edits.get(&self.tabs[index].path) {
            if session.lock().unwrap().has_changes() {
                self.pending_close_tab = Some(self.tabs[index].path.clone());
                return;
            }
        }
        // Pack members at their visible group position before the 4 -> 3 transition.
        // This also keeps batch-close confirmations anchored while the group shrinks.
        let closing_path = self.tabs[index].path.clone();
        let key = tab_group_key(&closing_path);
        let members: Vec<_> = self.tabs.iter().enumerate().filter(|(_, tab)| tab_group_key(&tab.path) == key).map(|(i, _)| i).collect();
        if members.len() == 4 {
            let order = packed_tab_group_indices(self.tabs.len(), &members);
            self.save_active_tab();
            let active = self.path.clone();
            let mut previous: Vec<_> = std::mem::take(&mut self.tabs).into_iter().map(Some).collect();
            self.tabs = order.into_iter().map(|i| previous[i].take().unwrap()).collect();
            self.active_tab = self.tabs.iter().position(|tab| Some(&tab.path) == active.as_ref()).unwrap_or(0);
            index = self.tabs.iter().position(|tab| tab.path == closing_path).unwrap();
        }
        self.edits.remove(&self.tabs[index].path);
        self.saved_histories.remove(&self.tabs[index].path);
        self.node_controls.remove(&self.tabs[index].path);
        self.assets.forget(&self.tabs[index].path);
        self.save_active_tab();
        let closing_path=&self.tabs[index].path;
        if let Some(job)=&self.loading {if &job.path==closing_path {job.cancelled.store(true,Ordering::Relaxed);}}
        self.load_queue.retain(|path|path!=closing_path);
        self.load_reload.remove(closing_path);self.load_selection.remove(closing_path);
        for doc in &self.tabs[index].documents {tilemap::forget(ctx,&doc.file_id,&doc.text);}
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.path = None;
            self.raw = "".into();
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
        } else if let Some(kind)=ui_preview::graphic(doc) {
            match kind {ui_preview::Graphic::Image=>"Image",ui_preview::Graphic::RawImage=>"RawImage",ui_preview::Graphic::Text=>"Text",ui_preview::Graphic::Tmp=>"TextMeshProUGUI"}.to_owned()
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
        let start = self.documents[index].start_offset;
        let title = if self.documents[index].type_name == "GameObject" {
            editing::document_text_in_range(ui, start..start + self.documents[index].text.len())
                .and_then(|text| name_from_text(&text)).unwrap_or(title)
        } else { title };
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
        depth: usize,
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
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id((format!("go-{id}"), &self.settings.hierarchy_depth)), hierarchy_default_open(&self.settings.hierarchy_depth, depth));
        if force_open { state.set_open(true); }
        let has_visible_children = children.get(&id).is_some_and(|items| items.iter()
            .any(|child| self.game_object_has_match(*child, children, components)));
        let response = ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = 0.0;
            if has_visible_children {
                state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
            } else {
                ui.allocate_exact_size(egui::vec2(ui.spacing().indent, ui.spacing().icon_width), egui::Sense::hover());
            }
            ui.spacing_mut().item_spacing.x = spacing;
            let local = self.path.as_ref().and_then(|path| self.node_controls.get(path)).map(|controls| controls.local(&id)).unwrap_or_default();
            let flags = self.document_flags(game_object);
            let label = self.node_label(ui, game_object, title).strong();
            let label = if !flags.visible || !flags.selectable { label.weak() } else { label };
            let selected = self.selected.is_some_and(|index| index == game_object || self.documents[index].game_object.as_deref() == Some(&id));
            let response = ui.add_enabled(flags.selectable, egui::SelectableLabel::new(selected, label));
            let mut toggle = None;
            let controls_x = ui.ctx().data_mut(|data| data.get_temp::<f32>(egui::Id::new("hierarchy_controls_x"))).unwrap_or(ui.cursor().min.x);
            let controls_rect = egui::Rect::from_min_size(egui::pos2(controls_x, response.rect.center().y - 10.0), egui::vec2(30.0, 20.0));
            let controls_clip = ui.ctx().data_mut(|data| data.get_temp::<egui::Rect>(egui::Id::new("hierarchy_controls_clip"))).unwrap_or(ui.clip_rect());
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(controls_clip.left(), response.rect.top().min(controls_rect.top())),
                egui::pos2(controls_clip.right(), response.rect.bottom().max(controls_rect.bottom())),
            ).intersect(controls_clip);
            let hovered = ui.input(|input| input.pointer.hover_pos().is_some_and(|pointer| row_rect.contains(pointer)));
            if selected || response.has_focus() || hovered || !local.visible || !local.selectable || !flags.visible || !flags.selectable {
            let mut controls_ui = ui.new_child(egui::UiBuilder::new().id_salt(("node_switches", &id)).max_rect(controls_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
            if let Some(clip) = ui.ctx().data_mut(|data| data.get_temp::<egui::Rect>(egui::Id::new("hierarchy_controls_clip"))) {
                controls_ui.set_clip_rect(clip);
            }
            controls_ui.spacing_mut().item_spacing.x = 0.0;
            if hierarchy_controls::icon(&mut controls_ui, true, flags.visible, local.visible && !flags.visible).clicked() { toggle = Some(true); }
            if hierarchy_controls::icon(&mut controls_ui, false, flags.selectable, local.selectable && !flags.selectable).clicked() { toggle = Some(false); }
            }
            if let (Some(eye), Some(path)) = (toggle, self.path.clone()) {
                self.node_controls.entry(path).or_default().toggle(&id, eye);
                self.refresh_node_flags();
            }
            if response.clicked() { self.selected = Some(game_object); self.nested_selected=None; }
            if response.double_clicked() { self.focus_preview_node(game_object); }
            response
        });
        if has_visible_children {
            state.show_body_indented(&response.response, ui, |ui| {
                if let Some(ids) = children.get(&id).cloned() {
                    for child in ids { self.show_game_object(ui, child, children, components, depth+1); }
                }
            });
        }
        if self.locate_target.is_some_and(|target| target == game_object || components.get(&id).is_some_and(|items| items.contains(&target))) {
            response.response.scroll_to_me(Some(egui::Align::Center));
            self.locate_target = None;
            ui.ctx().data_mut(|d|d.remove::<String>(egui::Id::new("preview_hierarchy_locate")));
        }
    }

    fn show_linked_node(&mut self,ui:&mut egui::Ui,index:usize,docs:&[Document],children:&HashMap<String,Vec<usize>>,flags:&HashMap<String,hierarchy_controls::Flags>,depth:usize) {
        if depth>128 {return;}
        let doc=&docs[index];let id=&doc.file_id;
        let title=name_from_text(&doc.text).unwrap_or_else(||id.clone());
        if !self.query.trim().is_empty() {
            let query=self.query.to_lowercase();let mut pending=vec![index];let mut visited=std::collections::HashSet::new();let mut matched=false;
            while let Some(item)=pending.pop() {
                if !visited.insert(item) {continue;}
                let node=&docs[item];
                if docs.iter().filter(|component|component.file_id==node.file_id || component.game_object.as_deref()==Some(node.file_id.as_str())).any(|component|format!("{} {} {}",component.type_name,component.file_id,component.text).to_lowercase().contains(&query)) {matched=true;break;}
                if let Some(items)=children.get(&node.file_id) {pending.extend(items.iter().copied());}
            }
            if !matched {return;}
        }
        let own=flags.get(id).copied().unwrap_or_default();
        let original=self.documents.iter().position(|item|item.file_id==*id);
        let selected=self.selected.and_then(|index|self.documents.get(index)).is_some_and(|doc|doc.file_id==*id || doc.game_object.as_deref()==Some(id.as_str())) || self.nested_selected.as_ref().is_some_and(|(path,key)|Some(path)==self.path.as_ref() && key==id);
        let locate=ui.ctx().data_mut(|d|d.get_temp::<String>(egui::Id::new("preview_hierarchy_locate")));
        let mut state=egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(),ui.make_persistent_id(("linked_node",id,&self.settings.hierarchy_depth)),hierarchy_default_open(&self.settings.hierarchy_depth,depth));
        let force_open=ui.ctx().data_mut(|d|d.get_temp::<Arc<std::collections::HashSet<String>>>(egui::Id::new("preview_hierarchy_ancestors"))).is_some_and(|ancestors|ancestors.contains(id));
        if !self.query.trim().is_empty() || force_open {state.set_open(true);}
        let row=ui.horizontal(|ui| {
            if children.get(id).is_some_and(|items|!items.is_empty()) {state.show_toggle_button(ui,egui::collapsing_header::paint_default_icon);}
            else {ui.allocate_exact_size(egui::vec2(ui.spacing().indent,20.0),egui::Sense::hover());}
            let label=if own.visible && own.selectable {RichText::new(title).strong()} else {RichText::new(title).weak()};
            let response=ui.add_enabled(own.selectable,egui::SelectableLabel::new(selected,label));
            if locate.as_deref()==Some(id.as_str()) {
                response.scroll_to_me(Some(egui::Align::Center));
                ui.ctx().data_mut(|d|d.remove::<String>(egui::Id::new("preview_hierarchy_locate")));
                self.locate_target=None;
            }
            let clip=ui.ctx().data_mut(|data|data.get_temp::<egui::Rect>(egui::Id::new("hierarchy_controls_clip"))).unwrap_or(ui.clip_rect());
            let x=ui.ctx().data_mut(|data|data.get_temp::<f32>(egui::Id::new("hierarchy_controls_x"))).unwrap_or(clip.left());
            let hovered=ui.input(|input|input.pointer.hover_pos().is_some_and(|point|clip.contains(point) && point.y>=response.rect.top() && point.y<=response.rect.bottom()));
            let local=self.path.as_ref().and_then(|path|self.node_controls.get(path)).map(|controls|controls.local(id)).unwrap_or_default();
            if selected || hovered || response.has_focus() || !own.visible || !own.selectable || !local.visible || !local.selectable {
                let mut controls=ui.new_child(egui::UiBuilder::new().id_salt(("linked_switches",id)).max_rect(egui::Rect::from_min_size(egui::pos2(x,response.rect.center().y-10.0),egui::vec2(30.0,20.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                controls.set_clip_rect(clip);controls.spacing_mut().item_spacing.x=0.0;
                let eye=hierarchy_controls::icon(&mut controls,true,own.visible,local.visible && !own.visible).clicked();
                let hand=hierarchy_controls::icon(&mut controls,false,own.selectable,local.selectable && !own.selectable).clicked();
                if let Some(path)=self.path.clone() {if eye || hand {self.node_controls.entry(path).or_default().toggle(id,eye);self.refresh_node_flags();}}
            }
            if response.clicked() {
                self.selected=original;
                self.nested_selected=if original.is_none() {self.path.clone().map(|path|(path,id.clone()))} else {None};
            }
            if response.double_clicked() {self.preview.focus(docs.iter().filter(|component|component.game_object.as_deref()==Some(id.as_str())).map(|component|component.file_id.clone()).collect());}
        });
        state.show_body_indented(&row.response,ui,|ui| {
            if let Some(items)=children.get(id) {for child in items {self.show_linked_node(ui,*child,docs,children,flags,depth+1);}}
        });
    }

    fn show_game_object_inspector(&mut self, ui: &mut egui::Ui, object: usize) {
        let ctx = ui.ctx().clone();
        let object_id = self.documents[object].file_id.clone();
        let inspector_path = self.path.clone();
        let session = self.path.as_ref().and_then(|path| self.edits.get(path)).cloned();
        let prefix = self.raw.find("--- !u!").unwrap_or(0);
        let mut offset = prefix;
        let mut sections = Vec::new();
        for (index, doc) in self.documents.iter().enumerate() {
            if doc.game_object.as_deref() == Some(&object_id) {
                sections.push((index, offset..offset + doc.text.len()));
            }
            offset += doc.text.len();
        }
        // Preserve serialized component order, with the Transform first.
        let order: Vec<_> = file_id_entries_from_text(&self.documents[object].text).into_iter()
            .filter(|(field, _)| field == "component").map(|(_, id)| id).collect();
        sections.sort_by_key(|(index, _)| {
            let doc = &self.documents[*index];
            (if matches!(doc.type_name.as_str(), "Transform" | "RectTransform") { 0 } else { 1 },
             order.iter().position(|id| id == &doc.file_id).unwrap_or(usize::MAX))
        });
        let target_key = ui.make_persistent_id(("component_target", &self.path));
        let previous = ctx.data_mut(|data| {
            let previous = data.get_temp::<usize>(target_key);
            if let Some(index) = self.selected { data.insert_temp(target_key, index); }
            previous
        });
        let prefab_guid=self.path.as_ref().and_then(|path|self.preview.hierarchy_documents(&path.display().to_string())).and_then(|data|data.prefab_roots.get(&object_id).cloned())
            .or_else(||ctx.data(|d|d.get_temp::<std::collections::HashMap<String,String>>(egui::Id::new("inspector_prefab_roots"))).and_then(|roots|roots.get(&object_id).cloned()));
        ui.horizontal_wrapped(|ui| {
            ui.heading(self.display_name(&self.documents[object]));
            if let Some(guid)=prefab_guid {
                let catalog=ctx.data(|d|d.get_temp::<Arc<std::collections::HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
                let path=catalog.as_ref().and_then(|c|c.get(&guid)).filter(|p|p.is_file()).cloned();
                if ui.add_enabled(path.is_some(),egui::Button::new(crate::localization::tr("打开"))).on_hover_text(crate::localization::tr("打开源 Prefab，场景实例覆盖不会写入源资源")).clicked(){if let Some(path)=path{ctx.data_mut(|d|d.insert_temp(egui::Id::new("open_reference"),script_properties::ReferenceOpen::Asset(path,String::new())));}}
            }
        });
        ui.horizontal(|ui| {
            ui.small(format!("GameObject · fileID: {object_id}"));
            if ui.button(crate::localization::tr("复制 FileID")).clicked() { ctx.copy_text(object_id.clone()); }
        });
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("inspector_component_sections"), true));
        ScrollArea::both().id_salt(("game_object_inspector", &inspector_path, &object_id)).auto_shrink([false, false]).show(ui, |ui| {
            let object_start=prefix+self.documents[..object].iter().map(|d|d.text.len()).sum::<usize>();
            let object_doc=self.documents[object].clone();
            editing::publish(&ctx,session.clone(),object_start..object_start+object_doc.text.len());
            egui::CollapsingHeader::new("GameObject").id_salt("game_object_properties").default_open(true).show(ui,|ui|{
                ui.horizontal(|ui|{
                    ui.selectable_value(&mut self.show_game_object_yaml,false,localization::tr("显示属性"));
                    ui.selectable_value(&mut self.show_game_object_yaml,true,localization::tr("显示 YAML"));
                });
                if self.show_game_object_yaml {ui.add(egui::Label::new(RichText::new(object_doc.text.as_ref()).monospace()).wrap());}
                else {show_game_object_properties(ui,&object_doc.text);}
            });
            for (index, range) in sections {
                let doc = self.documents[index].clone();
                ui.push_id((&inspector_path, &doc.file_id), |ui| {
                    let focused = self.selected == Some(index) && previous != Some(index);
                    let mut state=egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(),ui.make_persistent_id("component"),matches!(doc.type_name.as_str(),"Transform"|"RectTransform"));
                    if focused {state.set_open(true);}
                    let header_background = ui.painter().add(egui::Shape::Noop);
                    let header_fill = if ui.visuals().dark_mode {
                        Color32::from_rgb(65, 76, 96)
                    } else {
                        Color32::from_rgb(210, 225, 244)
                    };
                    let mut title_clicked=false;
                    let mut title_rect=egui::Rect::NOTHING;
                    let mut header=state.show_header(ui,|ui| {
                        let title=ui.add(egui::Label::new(self.display_name(&doc)).sense(egui::Sense::click()));
                        title_clicked=title.clicked();
                        title_rect=title.rect;
                        ui.add_space(4.0);
                        model_preview::component_button(ui,&doc.text);
                        spine_preview::component_button(ui,&doc.text,&self.documents);
                    });
                    if title_clicked {header.set_open(!header.is_open());}
                    let (_,header_response,_) = header.body(|ui| {
                        editing::publish(&ctx, session.clone(), range.clone());
                        ui.small(crate::lf!("{0}  ·  fileID: {1}  ·  YAML 第 {2} 行", self.display_name(&doc), doc.file_id, doc.start_line));
                        let yaml_key = ui.make_persistent_id("show_yaml");
                        let mut yaml = ctx.data(|data| data.get_temp::<bool>(yaml_key).unwrap_or(false));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(crate::localization::tr("复制 FileID")).clicked() { ctx.copy_text(doc.file_id.clone()); }
                            ui.menu_button(crate::localization::tr("复制 GUID"), |ui| {
                                let entries = guid_entries_from_text(&doc.text);
                                if entries.is_empty() { ui.label(crate::localization::tr("当前对象没有 GUID")); }
                                for (field, guid) in entries {
                                    if ui.button(crate::lf!("复制 {0}", field)).clicked() { ctx.copy_text(guid); ui.close_menu(); }
                                }
                            });
                            ui.selectable_value(&mut yaml, false, crate::localization::tr("显示属性"));
                            ui.selectable_value(&mut yaml, true, crate::localization::tr("显示 YAML"));
                        });
                        ctx.data_mut(|data| data.insert_temp(yaml_key, yaml));
                        script_properties::show_prefab_reference(ui, &doc.text, &self.documents);
                        if doc.type_name == "Tilemap" {
                            let data = tilemap::cached(&ctx, &doc.file_id, &doc.text);
                            tilemap::show(ui, &data, yaml);
                        } else if matches!(doc.type_name.as_str(), "Grid" | "TilemapRenderer") && !yaml {
                            let data = tilemap::cached(&ctx, &doc.file_id, &doc.text);
                            tilemap::show(ui, &data, false);
                        } else if yaml {
                            ui.add(egui::Label::new(RichText::new(doc.text.as_ref()).monospace()).wrap());
                        } else if matches!(doc.type_name.as_str(), "Transform" | "RectTransform") {
                            show_transform_properties_content(ui, &doc.text);
                        } else if doc.type_name == "SpriteRenderer" {
                            script_properties::show_sprite_renderer_properties_content(ui, &doc.text, &self.query);
                        } else if matches!(doc.type_name.as_str(), "ParticleSystem" | "ParticleSystemRenderer") {
                            script_properties::show_particle_properties_content(ui, &doc.type_name, &doc.text, &self.query);
                        } else if matches!(doc.type_name.as_str(), "MeshRenderer" | "SkinnedMeshRenderer") {
                            script_properties::show_mesh_renderer_properties_content(ui, &doc.text, &self.query);
                        } else if ui_preview::graphic(&doc).is_some() {
                            script_properties::show_ui_graphic_properties_content(ui, &doc.text, &self.query);
                        } else if doc.type_name == "MonoBehaviour" {
                            self.script_properties.show(ui, &doc.text, &self.query);
                        } else if script_properties::is_collider_type(&doc.type_name) {
                            script_properties::show_collider_properties(ui, &doc.type_name, &doc.text, &self.query);
                        } else if doc.type_name == "Animator" {
                            script_properties::show_animator_properties(ui, &doc.text, &self.query);
                        } else {
                            ui.add(egui::Label::new(RichText::new(doc.text.as_ref()).monospace()).wrap());
                        }
                    });
                    let mut header_rect = header_response.response.rect;
                    header_rect.max.x = title_rect.right()+8.0;
                    ui.painter().set(header_background, egui::Shape::rect_filled(header_rect, 4.0, header_fill));
                    if focused { header_response.response.scroll_to_me(Some(egui::Align::Center)); }
                    ui.separator();
                });
            }
        });
        ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("inspector_component_sections")));
        editing::publish(&ctx, session, 0..0);
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
        let selectable = self.document_flags(index).selectable;
        let response = ui.add_enabled_ui(selectable, |ui| if is_match {
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
        }).inner;
        if response.clicked() {
            self.selected = Some(index);
        }
        if response.double_clicked() { self.focus_preview_node(index); }
        if self.locate_target == Some(index) {
            response.scroll_to_me(Some(egui::Align::Center));
            self.locate_target = None;
        }
    }

    fn focus_preview_node(&mut self, index: usize) {
        if !static_preview::Preview::available(&self.documents, self.path.as_deref()) { return; }
        let document = &self.documents[index];
        let Some(object) = (if document.type_name == "GameObject" {
            Some(document.file_id.clone())
        } else { document.game_object.clone() }) else { return; };
        let mut objects = std::collections::HashSet::from([object]);
        loop {
            let old_len = objects.len();
            let transforms: std::collections::HashSet<_> = self.documents.iter()
                .filter(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform")
                    && doc.game_object.as_ref().is_some_and(|id| objects.contains(id)))
                .map(|doc| doc.file_id.clone()).collect();
            for doc in &self.documents {
                if matches!(doc.type_name.as_str(), "Transform" | "RectTransform")
                    && doc.parent_transform.as_ref().is_some_and(|id| transforms.contains(id)) {
                    if let Some(id) = &doc.game_object { objects.insert(id.clone()); }
                }
            }
            if old_len == objects.len() { break; }
        }
        let components = self.documents.iter()
            .filter(|doc| doc.game_object.as_ref().is_some_and(|id| objects.contains(id)))
            .map(|doc| doc.file_id.clone()).collect();
        self.preview.focus(components);
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
        ui.small(crate::localization::tr("按 Animation path / Component / Property 重建"));
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
        let cache_key=egui::Id::new(("hierarchy_width",self.documents.as_ptr() as usize,self.documents.len(),self.raw.as_ptr() as usize,localization::language_index()));
        if let Some(width)=ctx.data(|data|data.get_temp::<f32>(cache_key)) {return width;}
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
        let mut widest = measure(crate::localization::tr("按 GameObject / Transform 层级重建")) + 16.0;
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
            let objects: std::collections::HashSet<_> = self.documents.iter()
                .filter(|doc| doc.type_name == "GameObject").map(|doc| doc.file_id.as_str()).collect();
            for document in &self.documents {
                if document.type_name != "GameObject" && document.game_object.as_deref().is_some_and(|id| objects.contains(id)) { continue; }
                widest = widest.max(measure(&self.display_name(document)) + 88.0);
            }
        }
        let width=widest.ceil().max(260.0);
        ctx.data_mut(|data|data.insert_temp(cache_key,width));
        width
    }
}

impl eframe::App for ViewerApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.assets.pause_media();
        playlist::flush_writes();
        if let Some(error) = media_history::take_error() {
            rfd::MessageDialog::new().set_title("Unity YAML Viewer")
                .set_description(crate::lf!("播放位置记录保存失败：{0}", error))
                .set_level(rfd::MessageLevel::Warning).show();
        }
        render_system::shutdown();
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        native_window::track_main_window(ctx);
        native_window::track_file_drop(ctx, _frame);
        audio_edit::set_undo_limit(self.settings.undo_steps);
        media_preview::show_trim_windows(ctx);
        self.playlist_close_dialog(ctx);
        if let Some(error) = media_history::take_error() { self.status = crate::lf!("播放位置记录保存失败：{0}", error); }
        if !self.show_settings && ctx.data_mut(|data|data.remove_temp::<bool>(egui::Id::new("media_decoder_ready"))).unwrap_or(false) {
            if let Some(path)=self.path.as_deref(){self.assets.retry_media_decoder(path);}
            ctx.request_repaint();
        }
        if ctx.data_mut(|data|data.remove_temp::<bool>(egui::Id::new("media_settings_request"))).unwrap_or(false) {native_window::wake(ctx,egui::Id::new("settings_window"));self.show_settings=true;}
        if let Some(path)=ctx.data_mut(|data|data.remove_temp::<PathBuf>(egui::Id::new("spine_window_request"))) {
            self.open_file_with_directory_tracking(path,false);
        }
        self.poll_file_load(ctx);
        if ctx.data_mut(|data|data.remove_temp::<bool>(egui::Id::new("spine_reload_request"))).unwrap_or(false) {self.reload_current_file(ctx);}
        if let Some(inbox) = &mut self.command_inbox {
            let requests = inbox.receive();
            for request in requests {
                self.accept_open_request(request,ctx);
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        self.assets.show_slice_previews(ctx);
        localization::set_language(&self.settings.language);
        self.updater.poll();
        if self.path.as_ref().is_some_and(|path| !self.edits.contains_key(path))
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
            if let Some(path) = self.path.clone() {
                if let Some(session) = self.saved_histories.get(&path).cloned() {
                    let mut history = session.lock().unwrap();
                    if history.can_undo() {
                        if fs::read(&path).ok().as_ref() == Some(&history.disk) {
                            history.undo();
                            self.raw = history.original.clone().into();
                            self.documents = parse_unity_yaml(&self.raw);
                            drop(history);
                            self.saved_histories.remove(&path);
                            self.edits.insert(path, session);
                        } else { self.status = crate::localization::tr("文件已被外部修改，请重新打开后再编辑，不能撤销旧记录。").into(); }
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
            let original = session.lock().unwrap().original.clone();
            if self.raw.as_ref() != original {
                let selected_id = self.selected.and_then(|index| self.documents.get(index)).map(|doc| doc.file_id.clone());
                self.raw = original.into();
                self.documents = parse_unity_yaml(&self.raw);
                self.selected = selected_id.and_then(|file_id| self.documents.iter().position(|doc| doc.file_id == file_id));
                editing::publish(ctx, current_edit.clone(), 0..self.raw.len());
            }
            if let Some(file_id) = ctx.data_mut(|data| data.remove_temp::<String>(egui::Id::new("created_controller_state"))) {
                self.selected = self.documents.iter().position(|doc| doc.file_id == file_id);
            }
        }
        if ctx.input(|input| input.viewport().close_requested()) && !self.edits.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if let Some(session) = self.edits.values().next() { session.lock().unwrap().warning = Some(crate::localization::tr("仍有文件处于编辑模式，请先保存或放弃编辑，再关闭程序。").into()); }
        }
        if ctx.input(|input|input.viewport().close_requested()) && self.edits.is_empty() && !self.assets.playlist_dirty_paths().is_empty(){
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);self.playlist_exit_requested=true;ctx.request_repaint();
        }
        if self.selected.is_some() { self.last_selected = self.selected; }
        // Let visible texture nodes claim a drop before the normal file opener.
        let defer_texture_drop = self.preview.open && material_graph::available(self.path.as_deref());
        if !defer_texture_drop {
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
        }

        if let Some(path)=ctx.data_mut(|d|d.remove_temp::<PathBuf>(egui::Id::new("playlist_open"))){self.open_file(path);}
        if let Some((media,list))=ctx.data_mut(|d|d.remove_temp::<(PathBuf,PathBuf)>(egui::Id::new("playlist_created"))) {
            // Replace only the current media tab; other media tabs keep their ownership.
            self.save_active_tab();
            self.assets.invalidate_playlist(&list);
            if let Some(index)=self.tabs.iter().position(|t|t.path==media) {
                self.assets.forget(&media);
                if let Some(existing)=self.tabs.iter().position(|t|t.path==list) {
                    self.tabs.remove(index);
                    self.activate_tab(if existing>index{existing-1}else{existing});
                    self.assets.request_media_playback(&list);
                } else {
                    self.tabs[index]=ViewerTab {path:list.clone(),raw:"".into(),documents:Vec::new(),selected:None,status:String::new(),encoding:String::new()};
                    self.activate_tab(index);
                }
                self.remember_recent_file();
            }else{self.open_file(list);}
        }
        if self.assets.media_fullscreen(ctx,self.path.as_deref()) {
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::BLACK)).show(ctx,|ui| {
                ui.visuals_mut().clone_from(&egui::Visuals::dark());
                if let Some(path)=self.path.clone(){self.assets.show_media_fullscreen(ui,&path);}
            });
            return;
        }
        let mut search_inline = false;
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::settings_button(ui).clicked() { native_window::wake(ctx,egui::Id::new("settings_window"));self.show_settings = true; }
                let previous_theme = self.settings.theme.clone();
                ui.push_id("toolbar_theme", |ui| theme::toolbar_selector(ui, &mut self.settings.theme));
                if previous_theme != self.settings.theme {
                    theme::set_preference(ctx, &self.settings.theme);
                    persist_theme(&self.settings.theme);
                }
                let left_width = ui.available_width().max(0.0);
                ui.allocate_ui_with_layout(egui::vec2(left_width, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
                    if self.updater.release.is_some() && ui.add(theme::toolbar_button(ui, "New", true)).clicked() {
                        self.updater.show = true;
                    }
                    let mut open_dialog = false;
                    let mut recent_path = None;
                    egui::menu::menu_custom_button(ui, theme::toolbar_button(ui, crate::localization::tr("文件"), false), |ui| {
                        if ui.button(crate::localization::tr("打开文件")).clicked() {
                            open_dialog = true;
                            ui.close_menu();
                        }
                        if ui.button(localization::tr("打开 URL")).clicked() {
                            self.show_url = true;
                            self.url_error = None;
                            ui.close_menu();
                        }
                        ui.menu_button(crate::localization::tr("最近打开"), |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            if self.recent_files.paths().is_empty() { ui.label(crate::localization::tr("暂无最近文件")); }
                            let mut remove = None;
                            let row_height = ui.text_style_height(&egui::TextStyle::Button) + 4.0;
                            let widest = self.recent_files.paths().iter().map(|path| {
                                let name = path.file_name().unwrap_or_default().to_string_lossy();
                                ui.painter().layout_no_wrap(name.into_owned(), egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color()).size().x
                            }).fold(0.0_f32, f32::max);
                            let row_width = (widest + row_height + 20.0).clamp(180.0, 420.0).min((ctx.screen_rect().width() - 32.0).max(80.0));
                            // Reset the popup's content width as well as its rows. Otherwise the
                            // separator preserves the previous Area width after deleting a long name.
                            ui.set_width(row_width);
                            for path in self.recent_files.paths() {
                                ui.push_id(path, |ui| {
                                    let (row, _) = ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::hover());
                                    let close_rect = egui::Rect::from_min_max(egui::pos2(row.right() - row_height, row.top()), row.max);
                                    let name_rect = egui::Rect::from_min_max(row.min, egui::pos2(close_rect.left() - 4.0, row.bottom()));
                                    ui.spacing_mut().interact_size.y = row_height;
                                    ui.spacing_mut().button_padding.y = 2.0;
                                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                                    let name_response = ui.put(name_rect, egui::Button::new(""));
                                    let text_rect = name_rect.shrink2(egui::vec2(6.0, 0.0));
                                    let galley = ui.painter().layout(name.into_owned(), egui::TextStyle::Button.resolve(ui.style()),
                                        ui.visuals().text_color(), text_rect.width());
                                    let mut job = galley.job.as_ref().clone();
                                    job.wrap.max_rows = 1;
                                    job.wrap.break_anywhere = true;
                                    let galley = ui.painter().layout_job(job);
                                    ui.painter().with_clip_rect(ui.clip_rect().intersect(text_rect)).galley(
                                        egui::pos2(text_rect.left(), text_rect.center().y - galley.size().y * 0.5), galley, ui.visuals().text_color());
                                    if name_response.on_hover_text(path.display().to_string()).clicked() {
                                        recent_path = Some(path.clone());
                                        ui.close_menu();
                                    }
                                    let close = ui.put(close_rect, egui::Button::new(""));
                                    icons::paint(ui, icons::Icon::Minus, close.rect.shrink(3.0), ui.visuals().text_color());
                                    if close.on_hover_text(localization::tr("删除记录")).clicked() { remove = Some(path.clone()); }
                                });
                            }
                            if let Some(path) = remove { self.recent_files.remove(&path); }
                            if !self.recent_files.paths().is_empty() {
                                ui.separator();
                                if ui.button(localization::tr("清空记录")).clicked() { self.recent_files.clear(); }
                            }
                        });
                    });
                    if open_dialog { self.show_open_dialog(); }
                    if let Some(path) = recent_path { self.open_file(path); }
                    let previous_spacing = ui.spacing().item_spacing.x;
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if icons::button(ui, icons::Icon::Search, localization::tr("查找")).clicked() {
                        self.show_search = !self.show_search;
                    }
                    if self.show_search && ui.available_width() >= 160.0 {
                        search_inline = true;
                        self.search_field(ui);
                    }
                    if !self.show_search {
                        let text = localization::tr("可以将任何本软件支持的格式文件拖入窗口打开");
                        let font = egui::FontId::proportional(13.0);
                        let text_width = ui.painter().layout_no_wrap(text.into(),font.clone(),ui.visuals().weak_text_color()).size().x;
                        let button_text = localization::tr("格式支持");
                        let button_width = ui.painter().layout_no_wrap(button_text.into(),egui::TextStyle::Button.resolve(ui.style()),ui.visuals().text_color()).size().x + 12.0;
                        let command_width=ui.painter().layout_no_wrap(localization::tr("指令查看").into(),egui::TextStyle::Button.resolve(ui.style()),ui.visuals().text_color()).size().x+ui.spacing().button_padding.x*2.+ui.spacing().item_spacing.x;
                        let width = (text_width + button_width + 10.0).min((ui.available_width()-command_width).max(0.0));
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::hover());
                        let button_rect = egui::Rect::from_min_max(egui::pos2((rect.right()-button_width).max(rect.left()),rect.center().y-10.0),egui::pos2(rect.right(),rect.center().y+10.0));
                        let text_rect = egui::Rect::from_min_max(rect.min,egui::pos2((button_rect.left()-4.0).max(rect.left()),rect.bottom()));
                        let galley = ui.painter().layout_no_wrap(text.into(),font,ui.visuals().weak_text_color());
                        ui.painter().with_clip_rect(ui.clip_rect().intersect(text_rect)).galley(
                            egui::pos2(text_rect.left()+4.0,text_rect.center().y-galley.size().y*0.5),galley,ui.visuals().weak_text_color());
                        ui.scope(|ui| {
                            ui.spacing_mut().interact_size.y = 20.0;
                            ui.spacing_mut().button_padding.y = 1.0;
                            if ui.put(button_rect,egui::Button::new(button_text)).clicked() { self.show_supported_formats = true; }
                        });
                    }
                    if ui.button(localization::tr("指令查看")).clicked() {self.show_command_help=true;native_window::wake(ui.ctx(),egui::Id::new("command_help"));}
                    ui.spacing_mut().item_spacing.x = previous_spacing;
                });
                });
            });
            if self.show_search && !search_inline {
                ui.horizontal(|ui| self.search_field(ui));
            }
            if !self.tabs.is_empty() {
                ui.separator();
                let mut activate = None;
                let mut close = None;
                let mut tab_rects=Vec::new();
                let tab_width=(ctx.screen_rect().right()-ui.min_rect().left()-8.0).max(1.0);
                let spacing=ui.spacing().item_spacing;
                let mut slots=Vec::new();let mut x=0.0;let mut y=0.0;
                let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                for (index, tab) in self.tabs.iter().enumerate() {
                    let key = tab_group_key(&tab.path);
                    if let Some((_, members)) = groups.iter_mut().find(|(kind, _)| *kind == key) { members.push(index); }
                    else { groups.push((key, vec![index])); }
                }
                let mut visible = Vec::new();
                for (key, members) in &groups {
                    if members.len() > 3 || self.closing_tab_group.as_ref() == Some(key) {
                        let index = members.iter().copied().find(|&i| i == self.active_tab)
                            .or_else(|| members.iter().copied().find(|&i| self.tab_group_active.get(key) == Some(&self.tabs[i].path))).unwrap_or(members[0]);
                        visible.push(TabGroup { selected: index, members: members.clone() });
                    } else { for &index in members { visible.push(TabGroup { selected: index, members: vec![index] }); } }
                }
                visible.sort_by_key(|group| group.members[0]);
                let mut close_group = None;
                for group in &visible {
                    let tab = &self.tabs[group.selected];

                    let title=tab.path.file_name().and_then(|name|name.to_str()).unwrap_or(crate::localization::tr("未命名"));
                    let dirty=self.assets.playlist_dirty(&tab.path) || self.edits.get(&tab.path).is_some_and(|session|session.lock().unwrap().has_changes());
                    let title=if dirty{format!("{title} *")}else{title.to_owned()};
                    let width=(ui.painter().layout_no_wrap(title,egui::FontId::proportional(14.0),ui.visuals().text_color()).size().x + 14.0 + tab_action_width(ui, group.members.len())).min(350.0).min(tab_width);
                    if x>0.0 && x+width>tab_width {x=0.0;y+=24.0+spacing.y;}
                    slots.push(egui::Rect::from_min_size(egui::pos2(x,y),egui::vec2(width,24.0)));x+=width+spacing.x;
                }
                let (strip_rect,strip_response)=ui.allocate_exact_size(egui::vec2(tab_width,y+24.0),egui::Sense::hover());
                    for (position, group) in visible.iter().enumerate() {
                        let index = group.selected;
                        let members = &group.members;
                        let tab = &self.tabs[index];
                        let grouped = group.is_collapsed();
                        let title = tab
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(crate::localization::tr("未命名"));
                        let is_active = self.active_tab == index;
                        let dirty = self.assets.playlist_dirty(&tab.path) || self.edits.get(&tab.path).is_some_and(|session| session.lock().unwrap().has_changes());
                        let title = if dirty { format!("{title} *") } else { title.to_string() };
                        let slot=slots[position].translate(strip_rect.min.to_vec2());
                        let (open_clicked, close_clicked, response, slot) = unified_tab(ui, &title, is_active, &tab.path,slot,members.len());
                        tab_rects.push(slot);
                        if grouped {
                            let popup_id = ui.id().with(("tab_group", tab_group_key(&tab.path)));
                            if close_clicked { ui.memory_mut(|memory| memory.toggle_popup(popup_id)); }
                            egui::popup::popup_below_widget(ui, popup_id, &response, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                                let row_height = ui.text_style_height(&egui::TextStyle::Button) + 4.0;
                                let widest = members.iter().map(|&member| {
                                    let path = &self.tabs[member].path;
                                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                                    ui.painter().layout_no_wrap(format!("{name} *"), egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color()).size().x
                                }).fold(0.0_f32, f32::max);
                                let row_width = (widest + row_height + 20.0).clamp(180.0, 420.0).min((ctx.screen_rect().width() - 32.0).max(80.0));
                                ui.set_width(row_width);
                                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                    for &member in members {
                                        let path = &self.tabs[member].path;
                                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(localization::tr("未命名"));
                                        let dirty = self.assets.playlist_dirty(path) || self.edits.get(path).is_some_and(|edit| edit.lock().unwrap().has_changes());
                                        let title = if dirty { format!("{name} *") } else { name.to_owned() };
                                        ui.push_id(path, |ui| {
                                            let (row, _) = ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::hover());
                                            let close_rect = egui::Rect::from_min_max(egui::pos2(row.right() - row_height, row.top()), row.max);
                                            let name_rect = egui::Rect::from_min_max(row.min, egui::pos2(close_rect.left() - 4.0, row.bottom()));
                                            ui.spacing_mut().interact_size.y = row_height;
                                            ui.spacing_mut().button_padding.y = 2.0;
                                            if member == index { ui.painter().rect_filled(name_rect, 3.0, ui.visuals().selection.bg_fill); }
                                            let name_response = ui.put(name_rect, egui::Button::new("").frame(false));
                                            let text_rect = name_rect.shrink2(egui::vec2(6.0, 0.0));
                                            let galley = ui.painter().layout(title, egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color(), text_rect.width());
                                            let mut job = galley.job.as_ref().clone();
                                            job.wrap.max_rows = 1;
                                            job.wrap.break_anywhere = true;
                                            let galley = ui.painter().layout_job(job);
                                            ui.painter().with_clip_rect(ui.clip_rect().intersect(text_rect)).galley(
                                                egui::pos2(text_rect.left(), text_rect.center().y - galley.size().y * 0.5), galley, ui.visuals().text_color());
                                            if name_response.on_hover_text(path.display().to_string()).clicked() {
                                                activate = Some(member); ui.memory_mut(|memory| memory.close_popup());
                                            }
                                            let close_response = ui.put(close_rect, egui::Button::new("").frame(false));
                                            icons::paint(ui, icons::Icon::Close, close_response.rect.shrink(5.0), ui.visuals().text_color());
                                            if close_response.clicked() { close = Some(member); ui.memory_mut(|memory| memory.close_popup()); }
                                        });
                                    }
                                });
                                ui.separator();
                                if ui.add(egui::Button::new(localization::tr("关闭全部")).frame(false)).clicked() {
                                    close_group = Some(members.iter().map(|&i| self.tabs[i].path.clone()).collect::<Vec<_>>());
                                    ui.memory_mut(|memory| memory.close_popup());
                                }
                            });
                        }
                        if response.drag_started_by(egui::PointerButton::Primary) {
                            let origin=ui.input(|i|i.pointer.press_origin()).unwrap_or(slot.center());
                            if origin.x<slot.right()-tab_action_width(ui, members.len()) {
                                ctx.data_mut(|d|{d.insert_temp(egui::Id::new("tab_drag_path"),tab.path.clone());d.insert_temp(egui::Id::new("tab_drag_offset"),origin-slot.min);});
                            }
                        }
                        if open_clicked {
                            activate = Some(index);
                        }
                        if close_clicked && !grouped {
                            close = Some(index);
                        }
                    }
                if let Some(paths) = close_group {
                    self.closing_tab_group = paths.first().map(|path| tab_group_key(path));
                    self.close_group_queue = paths.into();
                    ctx.request_repaint();
                }
                let strip=strip_response;
                let dragged=ctx.data_mut(|d|d.get_temp::<PathBuf>(egui::Id::new("tab_drag_path")));
                if let Some(path)=dragged {
                    if let Some(pointer)=ctx.input(|i|i.pointer.hover_pos()).filter(|p|strip.rect.contains(*p)) {
                        let slot=tab_drop_slot(&tab_rects,pointer);
                        if let Some(rect)=tab_rects.get(slot).or_else(||tab_rects.last()) {
                            let x=if slot<tab_rects.len(){rect.left()-2.0}else{rect.right()+2.0};
                            ui.painter().line_segment([egui::pos2(x,rect.top()),egui::pos2(x,rect.bottom())],egui::Stroke::new(2.0_f32,egui::Color32::from_rgb(0,122,255)));
                        }
                        if let Some(from) = visible.iter().position(|group| {
                            group.members.iter().any(|&i| self.tabs[i].path == path)
                        }) {
                            let blocks: Vec<Vec<usize>> = visible.iter().map(|group| group.members.clone()).collect();
                            if let Some(order) = reordered_tab_indices(&blocks, from, slot) {
                                self.save_active_tab();
                                let active = self.path.clone();
                                // Move each visible tab as a block, including noncontiguous group members.
                                let mut previous: Vec<_> = std::mem::take(&mut self.tabs).into_iter().map(Some).collect();
                                self.tabs = order.into_iter().map(|index| previous[index].take().unwrap()).collect();
                                if let Some(index) = self.tabs.iter().position(|tab| Some(&tab.path) == active.as_ref()) { self.active_tab = index; }
                                ctx.request_repaint();
                            }
                        }
                    }
                    if !ctx.input(|i|i.pointer.primary_down()) {
                        ctx.data_mut(|d|{d.remove::<PathBuf>(egui::Id::new("tab_drag_path"));d.remove::<egui::Vec2>(egui::Id::new("tab_drag_offset"));});
                        self.persist_open_tabs();
                    }
                } else if let Some(index) = close {
                    self.close_tab(index,ctx);
                } else if let Some(index) = activate {
                    self.save_active_tab();
                    self.activate_tab(index);
                }
            }
        });

        let hierarchy_width = self.hierarchy_preferred_width(ctx);
        self.refresh_node_flags();
        self.script_properties.update(self.path.as_deref(), ctx);
        let reference_tab = spine_preview::is_data_asset(&self.raw) || self.path.as_ref().is_some_and(|path| playlist::is_playlist(path) || media_preview::is_media(path) || asset_viewer::is_image(path) || asset_viewer::is_text(path) || (spine_preview::is_spine(path) || model_preview::is_model(path)));
        if !reference_tab {
        egui::SidePanel::left("hierarchy")
            .resizable(false)
            .exact_width(hierarchy_width)
            .show(ctx, |ui| {
                let identity=self.path.as_ref().map(|path|path.display().to_string()).unwrap_or_default();
                self.preview.hierarchy_header(ui,&self.documents,&identity);
                ui.small(crate::localization::tr("按 GameObject / Transform 层级重建"));
                // A vertical-only scroll area expands its reported width for long
                // indented rows. SidePanel clips to its configured width, leaving
                // an unpainted gap before the next panel. Scroll in both axes so
                // the viewport stays bounded by the hierarchy panel instead.
                ScrollArea::both()
                    .id_salt("hierarchy_scroll")
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
                            let expanded=self.preview.hierarchy_documents(&self.path.as_ref().map(|path|path.display().to_string()).unwrap_or_default());
                            let tree_docs=expanded.as_ref().map_or(self.documents.as_slice(),|data|data.documents.as_slice());
                            let locate=ctx.data_mut(|data|data.get_temp::<String>(egui::Id::new("preview_hierarchy_locate")));
                            let ancestors=expanded.as_ref().zip(locate.as_deref()).map(|(data,id)|data.hierarchy_ancestors(id)).unwrap_or_default();
                            ctx.data_mut(|data|data.insert_temp(egui::Id::new("preview_hierarchy_ancestors"),Arc::new(ancestors)));
                            let (roots, children, components, unlinked) = build_tree(tree_docs);
                            let linked_flags=expanded.as_ref().map(|data|self.path.as_ref().and_then(|path|self.node_controls.get(path)).map(|controls|controls.effective(&data.documents)).unwrap_or_default());
                            let has_roots = !roots.is_empty();
                            // Pin node switches to the visible viewport, independent of horizontal scrolling.
                            let controls_clip = ui.clip_rect();
                            let controls_x = controls_clip.left().max(ui.cursor().min.x);
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(egui::Id::new("hierarchy_controls_x"), controls_x);
                                data.insert_temp(egui::Id::new("hierarchy_controls_clip"), controls_clip);
                            });
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.add_space(30.0);
                                ui.vertical(|ui| {
                                    let mut tree_clip = ui.clip_rect();
                                    tree_clip.min.x = tree_clip.min.x.max(controls_x + 30.0);
                                    ui.set_clip_rect(tree_clip);
                                    ui.spacing_mut().indent = 14.0;
                                    for root in roots {
                                        if let Some(data)=&expanded {
                                            self.show_linked_node(ui,root,&data.documents,&children,linked_flags.as_ref().unwrap(),0);
                                        } else {self.show_game_object(ui, root, &children, &components,0);}
                                    }
                                });
                            });
                            let unlinked:Vec<_>=if let Some(data)=&expanded {
                                unlinked.into_iter().filter_map(|index|self.documents.iter().position(|doc|doc.file_id==data.documents[index].file_id)).collect()
                            } else {unlinked};
                            if !has_roots && !unlinked.is_empty() {
                                for item in unlinked {
                                    self.show_custom_asset_tree(ui, item);
                                }
                            } else if !unlinked.is_empty() {
                                egui::CollapsingHeader::new(
                                    RichText::new(crate::localization::tr("未关联对象 / 解析异常")).color(warning_color(ui)),
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

        }
        let material_preview=material_graph::available(self.path.as_deref());
        let preview_available = material_preview || static_preview::Preview::available(&self.documents, self.path.as_deref());
        if !preview_available { self.preview.open = false; }
        if !self.preview.open {self.preview.release();self.preview_documents.clear();self.preview_text=Arc::from("");}
        if !self.preview.open || !material_preview {self.assets.pause_material_graphs();}
        if self.preview.open && material_preview {
            self.preview.release();self.preview_documents.clear();self.preview_text=Arc::from("");
            preview_layout::panel(ctx, "material_shader_preview").show(ctx,|ui|{
                let bounds=ui.available_rect_before_wrap();
                let mut child=ui.new_child(egui::UiBuilder::new().id_salt("bounded_material_preview").max_rect(bounds));
                child.set_clip_rect(ui.clip_rect().intersect(bounds));
                if let Some(path)=&self.path{self.assets.show_material_graph(&mut child,path,&self.raw);}
                ui.allocate_rect(bounds,egui::Sense::hover());
            });
        }
        if defer_texture_drop {
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
        }
        if self.preview.open && !material_preview {
            let preview_edit = self.path.as_ref().and_then(|path| self.edits.get(path));
            let rendered: Arc<str> = preview_edit
                .map(|session| Arc::from(session.lock().unwrap().rendered())).unwrap_or_else(|| self.raw.clone());
            if !Arc::ptr_eq(&self.preview_text, &rendered) && self.preview_text != rendered {
                self.preview_documents = if preview_edit.is_none() {self.documents.clone()} else {parse_unity_yaml(&rendered)};
                self.preview_text = rendered;
            }
            preview_layout::panel(ctx, "static_prefab_preview").show(ctx, |ui| {
                // Keep overflowing toolbar content from expanding the panel's reported width.
                let preview_rect=ui.available_rect_before_wrap();
                let mut preview_ui=ui.new_child(egui::UiBuilder::new().id_salt("bounded_preview").max_rect(preview_rect));
                preview_ui.set_clip_rect(ui.clip_rect().intersect(preview_rect));
                let selected_id=self.nested_selected.as_ref().filter(|(path,_)|Some(path)==self.path.as_ref()).map(|(_,id)|id.clone())
                    .or_else(||self.selected.and_then(|index|self.documents.get(index)).map(|doc|doc.file_id.clone()));
                if let Some(component_id)=self.preview.show(&mut preview_ui, &self.preview_documents, &self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(), &self.node_flags, selected_id.as_deref()) {
                    if let Some(index)=self.documents.iter().position(|doc|doc.file_id==component_id) {
                        ctx.data_mut(|data|data.remove::<String>(egui::Id::new("preview_issue_component")));
                        self.selected=Some(index);self.nested_selected=None;
                        self.locate_target=Some(index);
                        let doc=&self.documents[index];
                        let linked=self.preview.hierarchy_documents(&self.path.as_ref().map(|p|p.display().to_string()).unwrap_or_default()).and_then(|data|data.hierarchy_object(&component_id));
                        let object=linked.clone().unwrap_or_else(||doc.game_object.clone().unwrap_or_else(||doc.file_id.clone()));
                        if linked.is_some() && (doc.type_name=="PrefabInstance" || doc.text.lines().next().is_some_and(|line|line.ends_with(" stripped"))) {self.nested_selected=self.path.clone().map(|path|(path,object.clone()));self.locate_target=None;}
                        ctx.data_mut(|d|d.insert_temp(egui::Id::new("preview_hierarchy_locate"),object));
                        match self.documents[index].type_name.as_str() {
                            "SpriteRenderer" => self.show_sprite_renderer_yaml=false,
                            "MonoBehaviour" => self.show_script_yaml=false,
                            _ => {},
                        }
                        self.query.clear();
                        ctx.request_repaint();
                    } else if let Some(expanded)=self.preview.hierarchy_documents(&self.path.as_ref().map(|p|p.display().to_string()).unwrap_or_default()) {
                        if let Some(doc)=expanded.documents.iter().find(|d|d.file_id==component_id) {
                            let object=doc.game_object.clone().unwrap_or_else(||doc.file_id.clone());
                            self.selected=None;self.locate_target=None;
                            self.nested_selected=self.path.clone().map(|path|(path,object.clone()));
                            ctx.data_mut(|d|d.insert_temp(egui::Id::new("preview_hierarchy_locate"),object));
                            self.query.clear();ctx.request_repaint();
                        }
                    }
                }
                ui.allocate_rect(preview_rect,egui::Sense::hover());
            });
            if !self.preview.open {self.preview_documents.clear();self.preview_text=Arc::from("");}
        }
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("inspector_hide_property_types"), self.preview.open));
        let mut content_frame=egui::Frame::central_panel(&ctx.style()).fill(ctx.style().visuals.window_fill);
        if self.path.as_ref().is_some_and(|path|playlist::is_playlist(path)||media_preview::is_media(path)) {content_frame=content_frame.inner_margin(egui::Margin::same(8));}
        egui::CentralPanel::default().frame(content_frame).show(ctx, |ui| {
            if reference_tab {
                editing::publish(ctx, None, 0..0);
                if material_preview && theme::preview_toggle(ui,self.preview.open).clicked(){self.preview.open=!self.preview.open;ctx.request_repaint();}
                if !spine_preview::is_data_asset(&self.raw) && self.path.as_ref().is_some_and(|path| asset_viewer::is_text(path) && !spine_preview::is_spine(path)) && ui.button(crate::localization::tr("重载")).clicked() {
                    if let Some(path)=&self.path {self.assets.forget(path);}
                    self.reload_current_file(ctx);
                }
                if let Some(path) = self.path.clone() { self.assets.show(ui, &path, &self.raw); }
                return;
            }
            let current_edit = self.path.as_ref().and_then(|path| self.edits.get(path)).cloned();
            let header_fill = if current_edit.is_some() {
                if ui.visuals().dark_mode { Color32::from_rgb(35, 65, 90) }
                else { Color32::from_rgb(215, 235, 252) }
            } else { Color32::TRANSPARENT };
            let header_background = ui.painter().add(egui::Shape::Noop);
            let header_top_left = ui.cursor().min;
            egui::Frame::new().fill(header_fill).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if preview_available && theme::preview_toggle(ui, self.preview.open).clicked() {
                    self.preview.open = !self.preview.open;
                    ctx.data_mut(|data|data.insert_temp(egui::Id::new("inspector_hide_property_types"),self.preview.open));
                    ctx.request_repaint();
                }
                ui.heading(if current_edit.is_some() { crate::localization::tr("Inspector · 编辑模式") } else { crate::localization::tr("只读 Inspector") });
                if current_edit.is_some() {
                    if ui.add(egui::Button::new(RichText::new(crate::localization::tr("放弃编辑")).color(Color32::WHITE))
                        .fill(Color32::from_rgb(211, 55, 65)).stroke(egui::Stroke::NONE)).clicked() {
                        if let Some(path) = &self.path {
                            if let Some(session) = self.edits.remove(path) {
                                let disk = session.lock().unwrap().disk.clone();
                                let (decoded, _, errors) = UTF_8.decode(&disk);
                                self.raw = (if errors { GBK.decode(&disk).0.into_owned() } else { decoded.into_owned() }).into();
                                self.documents = parse_unity_yaml(&self.raw);
                            }
                            self.saved_histories.remove(path);
                        }
                        self.save_active_tab();
                        editing::publish(ctx, None, 0..0);
                    }
                    if ui.add(egui::Button::new(RichText::new(crate::localization::tr("保存编辑")).color(Color32::WHITE))
                        .fill(Color32::from_rgb(28, 140, 76)).stroke(egui::Stroke::NONE)).clicked() {
                        self.pending_save = self.path.clone();
                        ctx.memory_mut(|memory| { if let Some(id) = memory.focused() { memory.surrender_focus(id); } });
                    }
                } else if self.path.is_some() && ui.add_enabled(!self.documents.is_empty(), egui::Button::new(crate::localization::tr("编辑模式"))).clicked() {
                    let path = self.path.clone().unwrap();
                    if let Some(history) = self.saved_histories.get(&path).cloned().filter(|history| fs::read(&path).ok().as_ref() == Some(&history.lock().unwrap().disk)) {
                        self.raw = history.lock().unwrap().original.clone().into();
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
                                if errors { self.status = crate::localization::tr("文件包含无法正确解码的字节，暂不开放编辑。").into(); return; }
                                decoded.into_owned()
                            } else { decoded.into_owned() };
                            if decoded != self.raw.as_ref() {
                                self.status = crate::localization::tr("文件已被外部修改，请重新打开后再进入编辑模式。").into();
                                return;
                            }
                            let session = Arc::new(std::sync::Mutex::new(editing::Session::new(path.clone(), self.raw.to_string(), disk)));
                            session.lock().unwrap().set_undo_limit(self.settings.undo_steps);
                            self.edits.insert(path, session.clone());
                            editing::publish(ctx, Some(session), 0..self.raw.len());
                        }
                        Err(error) => self.status = crate::lf!("无法进入编辑模式：{0}", error),
                    }
                    }
                }
                if self.path.as_ref().is_some_and(|path| !self.edits.contains_key(path)) {
                    if ui.button(crate::localization::tr("重载"))
                        .on_hover_text(crate::localization::tr("从磁盘重新读取当前文件并清空旧撤销记录"))
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
            let selected_object = self.selected.and_then(|index| {
                let doc = self.documents.get(index)?;
                if doc.type_name == "GameObject" { Some(index) }
                else { self.documents.iter().position(|object| object.type_name == "GameObject" && Some(&object.file_id) == doc.game_object.as_ref()) }
            });
            let expanded=self.preview.hierarchy_documents(&self.path.as_ref().map(|path|path.display().to_string()).unwrap_or_default());
            let nested=self.nested_selected.as_ref().filter(|(path,_)|Some(path)==self.path.as_ref()).and_then(|(_,id)|expanded.as_ref()?.documents.iter().position(|doc|doc.file_id==*id));
            if let (Some(index),Some(data))=(nested,expanded) {
                ctx.data_mut(|d|d.insert_temp(egui::Id::new("inspector_prefab_roots"),data.prefab_roots.clone()));
                ui.small(format!("Prefab · {}",crate::localization::tr("只读 Inspector")));
                let documents=std::mem::replace(&mut self.documents,data.documents.clone());
                let path=self.path.take();
                editing::publish(ctx,None,0..0);
                let selected=self.selected;
                let issue_component=ctx.data_mut(|data|data.remove_temp::<String>(egui::Id::new("preview_issue_component")));
                if let Some(id)=issue_component {self.selected=self.documents.iter().position(|doc|doc.file_id==id);}
                self.show_game_object_inspector(ui,index);
                self.selected=selected;
                self.path=path;self.documents=documents;
                ctx.data_mut(|d|d.remove::<std::collections::HashMap<String,String>>(egui::Id::new("inspector_prefab_roots")));
                editing::publish(ctx,None,0..0);
            } else if let Some(object) = selected_object {
                self.show_game_object_inspector(ui, object);
            } else if let Some(index) = self.selected {
				let doc = &self.documents[index];
                let offset = self.documents[..index].iter().map(|doc| doc.text.len()).sum::<usize>();
                let prefix = self.raw.find("--- !u!").unwrap_or(0);
                editing::publish(ctx, self.path.as_ref().and_then(|path| self.edits.get(path)).cloned(), prefix + offset..prefix + offset + doc.text.len());
				let is_animation_clip = doc.type_name == "AnimationClip";
				let is_transform = matches!(doc.type_name.as_str(), "Transform" | "RectTransform");
				let is_script = doc.type_name == "MonoBehaviour";
				let is_sprite_renderer = doc.type_name == "SpriteRenderer";
                let is_mesh_renderer = matches!(doc.type_name.as_str(), "MeshRenderer" | "SkinnedMeshRenderer");
                let is_particle = matches!(doc.type_name.as_str(), "ParticleSystem" | "ParticleSystemRenderer");
                let is_tilemap = matches!(doc.type_name.as_str(), "Tilemap" | "TilemapRenderer" | "Grid");
				let is_collider = script_properties::is_collider_type(&doc.type_name);
				let is_animator = doc.type_name == "Animator";
				let is_animator_state = doc.type_name == "AnimatorState";
				let is_material = doc.type_name == "Material";
                let is_sprite_atlas = doc.type_name == "SpriteAtlas";
				// Animation clips use their dedicated inspector, even when their
				// keyframe lists also match the generic table format.
				let custom_table = if is_script || is_animation_clip || is_transform || is_sprite_renderer || is_mesh_renderer || is_particle || is_tilemap || is_collider || is_animator || is_animator_state || is_material || is_sprite_atlas {
					None
				} else {
					yaml_uniform_table(&doc.text).map(|mut table| {
                        self.script_properties.resolve_table_types(&doc.text, &mut table);
                        table
                    })
				};
                let metadata_top_left = ui.cursor().min;
				ui.horizontal_wrapped(|ui| {ui.label(crate::lf!("{0}  ·  fileID: {1}  ·  YAML 第 {2} 行", self.display_name(doc), doc.file_id, doc.start_line)); model_preview::component_button(ui,&doc.text); spine_preview::component_button(ui,&doc.text,&self.documents); });
				let guid_entries = guid_entries_from_text(&doc.text);
				ui.horizontal(|ui| {
					ui.menu_button(crate::localization::tr("复制 GUID"), |ui| {
						ui.set_min_width(320.0);
						if guid_entries.is_empty() {
							ui.label(crate::localization::tr("当前对象没有 GUID"));
						}
						for (field_name, guid) in &guid_entries {
							if ui.button(crate::lf!("复制 {0}", field_name)).clicked() {
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
					ui.menu_button(crate::localization::tr("复制 FileID"), |ui| {
						ui.set_min_width(320.0);
						if ui.button(crate::localization::tr("自身 FileID")).clicked() {
							ctx.copy_text(doc.file_id.clone());
							ui.close_menu();
						}
						ui.monospace(&doc.file_id);
						ui.separator();
						for (field_name, file_id) in file_id_entries_from_text(&doc.text) {
							if ui.button(crate::lf!("复制 {0}", field_name)).clicked() {
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
						ui.small(crate::lf!("总时长：0.00s — {0:.2}s{1}", duration, sample_rate));
					}
					if is_transform {
						ui.selectable_value(&mut self.show_transform_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_transform_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_script {
						ui.selectable_value(&mut self.show_script_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_script_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_sprite_renderer || is_mesh_renderer || is_particle || is_tilemap {
						ui.selectable_value(&mut self.show_sprite_renderer_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_sprite_renderer_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_collider {
						ui.selectable_value(&mut self.show_collider_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_collider_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_animator {
						ui.selectable_value(&mut self.show_animator_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_animator_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_animator_state {
						ui.selectable_value(&mut self.show_animator_state_yaml, false, crate::localization::tr("显示属性"));
						ui.selectable_value(&mut self.show_animator_state_yaml, true, crate::localization::tr("显示 YAML"));
                    } else if is_sprite_atlas {
                        ui.selectable_value(&mut self.show_sprite_atlas_yaml, false, crate::localization::tr("显示属性"));
                        ui.selectable_value(&mut self.show_sprite_atlas_yaml, true, crate::localization::tr("显示 YAML"));
					} else if is_material {
                        ui.selectable_value(&mut self.show_material_yaml,false,localization::tr("显示属性"));
                        ui.selectable_value(&mut self.show_material_yaml,true,localization::tr("显示 YAML"));
					} else if custom_table.is_some() {
						if ui.selectable_label(self.show_custom_table, crate::localization::tr("显示表格")).clicked() {
							self.show_custom_table = true;
						}
						if ui.selectable_label(!self.show_custom_table, crate::localization::tr("显示 YAML")).clicked() {
							self.show_custom_table = false;
						}
						if self.show_custom_table {
							ui.checkbox(&mut self.freeze_table_header, crate::localization::tr("冻结表头"));
						}
					}
				});
                if current_edit.is_some() {
                    // Reserve the background before drawing widgets so the fill
                    // covers the full header, including the gaps between rows.
                    // Keep the controller canvas outside the metadata highlight.
                    let top_left = if self.documents.iter().any(|doc| doc.type_name == "AnimatorController") {
                        metadata_top_left
                    } else { header_top_left };
                    let rect = egui::Rect::from_min_max(top_left, egui::pos2(ui.max_rect().right(), ui.cursor().min.y));
                    ui.painter().set(header_background, egui::Shape::rect_filled(rect, 0.0, header_fill));
                }
				ui.separator();
				script_properties::show_prefab_reference(ui, &doc.text, &self.documents);
				if is_animation_clip {
                    ui.push_id((self.path.as_ref(), &doc.file_id), |ui| {
                        show_animation_timeline(ui, &doc.text, &self.selected_animation_track, &mut self.selected_keyframe);
                    });
					ui.separator();
					ui.small(crate::localization::tr("原始 YAML（点击关键帧后自动定位并高亮对应的 time / value）"));
					show_animation_yaml(ui, &doc.text, self.selected_keyframe.as_ref(), &self.query);
				} else if is_transform && !self.show_transform_yaml {
					show_transform_properties(ui, &doc.text);
				} else if is_script && !self.show_script_yaml && ui_preview::graphic(doc).is_none() {
					self.script_properties.show(ui, &doc.text, &self.query);
                } else if is_tilemap {
                    let data = tilemap::cached(ctx, &doc.file_id, &doc.text);
                    tilemap::show(ui, &data, self.show_sprite_renderer_yaml);
                } else if is_particle && !self.show_sprite_renderer_yaml {
                    ScrollArea::both().id_salt("particle_properties").auto_shrink([false,false]).show(ui, |ui| {
                        script_properties::show_particle_properties_content(ui, &doc.type_name, &doc.text, &self.query);
                    });
                } else if is_mesh_renderer && !self.show_sprite_renderer_yaml {
                    ScrollArea::both().id_salt("mesh_renderer_properties").auto_shrink([false,false]).show(ui, |ui| {
                        script_properties::show_mesh_renderer_properties_content(ui, &doc.text, &self.query);
                    });
				} else if (is_sprite_renderer && !self.show_sprite_renderer_yaml) || (ui_preview::graphic(doc).is_some() && !self.show_script_yaml) {
                    ScrollArea::both().id_salt("sprite_and_transform").auto_shrink([false, false]).show(ui, |ui| {
                    if let Some(transform) = self.documents.iter().find(|candidate| matches!(candidate.type_name.as_str(), "Transform" | "RectTransform") && candidate.game_object == doc.game_object && doc.game_object.is_some()) {
                        let transform_index = self.documents.iter().position(|candidate| candidate.file_id == transform.file_id).unwrap();
                        let transform_offset = self.documents[..transform_index].iter().map(|d| d.text.len()).sum::<usize>();
                        editing::publish(ctx, self.path.as_ref().and_then(|p| self.edits.get(p)).cloned(), prefix + transform_offset..prefix + transform_offset + transform.text.len());
                        egui::CollapsingHeader::new(&transform.type_name).id_salt(("sprite_transform", &doc.file_id)).default_open(true).show(ui, |ui| {
                            show_transform_properties_content(ui, &transform.text);
                        });
                        editing::publish(ctx, self.path.as_ref().and_then(|p| self.edits.get(p)).cloned(), prefix + offset..prefix + offset + doc.text.len());
                        ui.separator();
                    }
                    if is_sprite_renderer {script_properties::show_sprite_renderer_properties_content(ui, &doc.text, &self.query);}
                    else {script_properties::show_ui_graphic_properties_content(ui, &doc.text, &self.query);}
                    });
				} else if is_collider && !self.show_collider_yaml {
					script_properties::show_collider_properties(ui, &doc.type_name, &doc.text, &self.query);
				} else if is_animator && !self.show_animator_yaml {
					script_properties::show_animator_properties(ui, &doc.text, &self.query);
				} else if is_animator_state && !self.show_animator_state_yaml {
					script_properties::show_animator_state_properties(ui, &doc.text, &self.query);
                } else if is_sprite_atlas && !self.show_sprite_atlas_yaml {
                    script_properties::show_sprite_atlas_properties(ui, &doc.text, &self.query);
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
					let mut view = doc.text.as_ref();
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
					.on_hover_text(crate::localization::tr("可框选后右键复制；此处的输入不会写回或保存到原文件"));
					});
				}
			} else { ui.label(crate::localization::tr("从左侧选择 GameObject 或 Component，即可查看对应原始 YAML。应用不会写回或修改文件。")); }
			ui.separator();
			ui.small(crate::lf!("{0}　·　文件编码：{1}", self.status, if self.encoding.is_empty() { "尚未读取" } else { &self.encoding }));
		});

        if let Some(request) = script_properties::take_reference_request(ctx) {
            match request {
                script_properties::ReferenceOpen::Local(file_id) => {
                    if let Some(index) = self.documents.iter().position(|doc| doc.file_id == file_id) {
                        self.selected = Some(index);
                        self.locate_target = Some(index);
                        self.query.clear();
                        ctx.request_repaint();
                    } else { self.status = crate::lf!("本文件中未找到 fileID: {0}", file_id); }
                }
                script_properties::ReferenceOpen::Asset(path, file_id) => {
                    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
                    if playlist::is_playlist(&path) || media_preview::is_media(&path) || asset_viewer::is_image(&path) || asset_viewer::is_text(&path) || (spine_preview::is_spine(&path) || model_preview::is_model(&path)) {
                        self.open_file_with_directory_tracking(path, false);
                        self.query.clear();
                        ctx.request_repaint();
                    } else if matches!(extension.as_str(), "prefab" | "unity" | "asset" | "mat" | "controller" | "overridecontroller" | "anim" | "spriteatlas" | "guiskin" | "yaml" | "yml" | "physicsmaterial2d" | "physicmaterial") {
                        // Inspect the header so binary Unity assets go to their default app.
                        let is_yaml = fs::File::open(&path).ok().is_some_and(|mut file| {
                            use std::io::Read;
                            let mut prefix = [0u8; 256];
                            let count = file.read(&mut prefix).unwrap_or(0);
                            String::from_utf8_lossy(&prefix[..count]).contains("%YAML")
                                || String::from_utf8_lossy(&prefix[..count]).contains("--- !u!")
                        });
                        if is_yaml {
                            self.open_file_with_directory_tracking(path.clone(), false);
                            if self.load_queue.contains(&path) || self.loading.as_ref().is_some_and(|job|job.path==path) {
                                self.load_selection.insert(path,file_id.clone());
                            } else {
                                self.selected = self.documents.iter().position(|doc| doc.file_id == file_id)
                                    .or_else(|| (!self.documents.is_empty()).then_some(0));
                            }
                            self.query.clear();
                        } else { self.open_external_reference(&path); }
                    } else { self.open_external_reference(&path); }
                }
            }
        }

        if self.show_settings {
            crate::native_window::Window::new(crate::localization::tr("设置")).id(egui::Id::new("settings_window")).open(&mut self.show_settings).collapsible(false).resizable(false).default_width(348.0).min_width(348.0).max_width(348.0).default_height(560.0).show(ctx, |ui| {
                egui::Grid::new("application_settings").num_columns(2).spacing(egui::vec2(12.0, 16.0)).show(ui, |ui| {
                    settings_label(ui, crate::localization::tr("关于"));
                    if settings_control(ui, |ui| ui.add_sized([208.0, 30.0], egui::Button::new(crate::localization::tr("关于 Unity YAML Viewer")).wrap())).inner.clicked() {
                        native_window::wake(ctx,egui::Id::new("about_window"));
                        self.show_about = true;
                        self.show_older_history = false;
                    }
                    ui.end_row();
                    settings_label(ui, localization::tr("语言"));
                    let previous_language = self.settings.language.clone();
                    let selected_language = localization::LANGUAGES.iter().find(|(code, _)| *code == self.settings.language).map(|(_, name)| *name).unwrap_or("简体中文");
                    settings_control(ui, |ui| egui::ComboBox::from_id_salt("settings_language").width(208.0).selected_text(selected_language).show_ui(ui, |ui| {
                        for (code, name) in localization::LANGUAGES {
                            ui.selectable_value(&mut self.settings.language, code.into(), name);
                        }
                    }));
                    if previous_language != self.settings.language {

        localization::set_language(&self.settings.language);
                        self.status = localization::retranslate(&self.status);
                        self.encoding = localization::retranslate(&self.encoding);
                        for tab in &mut self.tabs {
                            tab.status = localization::retranslate(&tab.status);
                            tab.encoding = localization::retranslate(&tab.encoding);
                        }
                        for session in self.edits.values().chain(self.saved_histories.values()) {
                            let mut session = session.lock().unwrap();
                            if let Some(message) = &mut session.warning { *message = localization::retranslate(message); }
                        }
                        self.settings.persist();
                        ctx.request_repaint();
                    }
                    ui.end_row();
                    settings_label(ui, crate::localization::tr("主题"));
                    let previous_theme = self.settings.theme.clone();
                    settings_control(ui, |ui| theme::selector(ui, &mut self.settings.theme));
                    if previous_theme != self.settings.theme {
                        theme::set_preference(ctx, &self.settings.theme);
                        persist_theme(&self.settings.theme);
                    }
                    ui.end_row();
                    settings_label(ui, localization::tr("Hierarchy 默认展开层级"));
                    let previous_depth = self.settings.hierarchy_depth.clone();
                    settings_control(ui, |ui| theme::hierarchy_depth_selector(ui, &mut self.settings.hierarchy_depth));
                    if previous_depth != self.settings.hierarchy_depth {self.settings.persist();}
                    ui.end_row();
                    settings_label(ui, localization::tr("预览"));
                    settings_group(ui, "preview_toggle_settings_group", |ui| {
                        ui.set_width(208.0);
                        if theme::toggle_switch(ui, localization::tr("可预览默认展开"), &mut self.settings.auto_open_preview).changed() {
                            self.settings.persist();
                        }
                    });
                    ui.end_row();
                    settings_label(ui, localization::tr("视频解码"));
                    settings_group(ui, "decoder_settings_group", |ui| {
                        ui.set_width(208.0);
                        ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
                        self.decoder_settings.show(ui);
                    });
                    ui.end_row();
                    settings_label(ui, localization::tr("文件关联"));
                    settings_group(ui, "file_associations_settings_group", |ui| {
                        ui.set_width(208.0);
                        if ui.add_sized([208.0, 30.0], egui::Button::new(localization::tr("设置默认打开程序…")).wrap()).clicked() {
                            self.file_associations.open(ui.ctx());
                        }
                    });
                    ui.end_row();
                    settings_label(ui, localization::tr("软件缓存"));
                    settings_group(ui, "app_cache_settings_group", |ui| {
                        ui.set_width(208.0);
                        let mut in_use: Vec<_> = self.tabs.iter().map(|tab| tab.path.clone()).collect();
                        in_use.extend(self.assets.playlist_in_use());
                        if let Ok(executable) = std::env::current_exe() { in_use.push(executable); }
                        if let Some(directory) = self.updater.staged.as_ref().and_then(|path| path.parent()) { in_use.push(directory.to_owned()); }
                        let busy = self.url_job.is_some() || self.updater.downloading
                            || ui.ctx().data(|data| data.get_temp::<bool>(egui::Id::new("asset_catalog_loading"))).unwrap_or(false);
                        self.cache_panel.show(ui, in_use, busy);
                    });
                    ui.end_row();
                    settings_label(ui, localization::tr("最近打开记录"));
                    let mut recent_limit = self.recent_files.limit();
                    settings_group(ui, "recent_files_settings_group", |ui| {
                        ui.set_width(208.0);
                        ui.allocate_ui_with_layout(egui::vec2(208.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add_sized([40.0, ui.spacing().interact_size.y], egui::DragValue::new(&mut recent_limit)
                                .range(recent_files::MIN_LIMIT..=recent_files::MAX_LIMIT).speed(1.0));
                            // Lay out the localized button from the right first, then fill the remaining width.
                            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), ui.spacing().interact_size.y), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new(localization::tr("清空记录")).wrap_mode(egui::TextWrapMode::Extend)).clicked() { self.recent_files.clear(); }
                                ui.spacing_mut().slider_width = ui.available_width().max(1.0);
                                theme::slider(ui, true, egui::Slider::new(&mut recent_limit, recent_files::MIN_LIMIT..=recent_files::MAX_LIMIT).integer().show_value(false));
                            });
                        });
                    });
                    if recent_limit != self.recent_files.limit() { self.recent_files.set_limit(recent_limit); }
                    ui.end_row();
                    // Keep these final two rows last; insert new settings above this block (AGENTS.md).
                    settings_label(ui, crate::localization::tr("单个文件缓存撤销步骤"));
                    let previous = self.settings.undo_steps;
                    settings_group(ui, "undo_settings_group", |ui| {
                        ui.set_width(208.0);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.allocate_ui_with_layout(egui::vec2(208.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            // Anchor the number field to the column's left edge and give
                            // the rail all remaining width, independently of the value's digits.
                            ui.add_sized([40.0, ui.spacing().interact_size.y], egui::DragValue::new(&mut self.settings.undo_steps)
                                .range(editing::MIN_UNDO_LIMIT..=editing::MAX_UNDO_LIMIT).speed(1.0));
                            ui.spacing_mut().slider_width = ui.available_width();
                            theme::slider(ui, true, egui::Slider::new(&mut self.settings.undo_steps, editing::MIN_UNDO_LIMIT..=editing::MAX_UNDO_LIMIT).integer().show_value(false));
                        });
                        ui.add(egui::Label::new(RichText::new(crate::localization::tr("撤销步骤范围：50～1000，默认 200。超过上限后移除最早的记录。增加上限会增加内存占用。")).small().weak()).wrap());
                    });
                    if previous != self.settings.undo_steps {
                        self.settings.persist();
                        for session in self.edits.values().chain(self.saved_histories.values()) { session.lock().unwrap().set_undo_limit(self.settings.undo_steps); }
                    }
                    ui.end_row();
                    settings_label(ui, localization::tr("检查更新"));
                    settings_group(ui, "update_settings_group", |ui| {
                        ui.set_width(208.0);
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        if ui.add(egui::Checkbox::new(&mut self.settings.auto_check_updates, localization::tr("启动时自动检查更新"))).changed() {
                            self.settings.persist();
                        }
                        if ui.add_enabled_ui(!self.updater.checking && !self.updater.downloading, |ui| {
                            ui.add_sized([208.0, 30.0], egui::Button::new(localization::tr("检查更新")).wrap())
                        }).inner.clicked() {
                            self.updater.check(ctx, true);
                        }
                        if self.updater.checking { ui.horizontal(|ui| { ui.spinner(); ui.label(localization::tr("正在检查更新…")); }); }
                        if let Some(message) = &self.updater.check_message {
                            ui.add(egui::Label::new(localization::retranslate(message)).wrap());
                            if self.updater.check_failed {
                                ui.hyperlink_to(localization::tr("打开发布页面，手动更新"), "https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest");
                            }
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(8.0);
            });
        }
        if !self.show_settings && ctx.data(|data|data.get_temp::<bool>(egui::Id::new("media_decoder_ready"))).unwrap_or(false) {ctx.request_repaint();}
        self.update_windows(ctx);
        if self.show_about {
            crate::native_window::Window::new(crate::localization::tr("关于 Unity YAML Viewer")).id(egui::Id::new("about_window")).fit_height()
                .open(&mut self.show_about)
                .resizable(true)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.heading("Unity YAML Viewer");
                    ui.label(crate::lf!("版本 {0}", APP_VERSION));
                    ui.small(crate::localization::tr("Unity 文本序列化资源查看与属性编辑工具"));
                    ui.separator();
                    ui.heading(crate::localization::tr("更新历史"));
                    ScrollArea::vertical().auto_shrink([false, true]).max_height(280.0).show(ui, |ui| {
                        ui.label(RichText::new("1.2.3").strong());
                        ui.label(localization::tr("• 逐项关闭至 3 个时在原位展开；关闭全部期间保持成组，取消后恢复自动展开"));
                        ui.label(localization::tr("• 页签支持整组拖拽与文件堆叠数量，修复选中或悬停时页签栏高度变化"));
                        ui.label(localization::tr("• 相同后缀超过 3 个文件自动折叠为下拉页签，支持逐项关闭、关闭全部与未保存提醒"));
                        ui.label(localization::tr("• 淡入淡出支持预设及贝塞尔手柄，淡入起点 / 淡出终点幅度可调，参数窗置顶"));
                        ui.label(localization::tr("• 选择模式旁显示区间时长，区间模式另显示剪切后时长；停止播放回到零点"));
                        ui.label(localization::tr("• 主窗口与音频编辑支持波形缩放、水平浏览与播放居中跟随"));
                        ui.label(localization::tr("• 剪切窗独立播放，打开即停止来源页签；支持多文件弹窗，切换或关闭页签仍保留"));
                        ui.label(localization::tr("• 音频统一左右键选区，剪切 / 淡入淡出 / dB 可按区间处理；未选范围的 dB 作用于当前整段"));
                        ui.label(localization::tr("• 音视频剪切：视频另存 MP4，音频可选 MP3 / WAV / FLAC / M4A 及适用参数；需 FFmpeg"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.2.2").strong());
                        ui.label(localization::tr("• 九宫格无损精简：完整 RGBA 判重、拉伸 / 重复自动识别，可编辑预览与原位替换"));
                        ui.label(localization::tr("• slice9 支持批量 / 目录 / dry-run，--preview 最多 10 个可编辑窗口；顶部指令查看提供示例"));
                        ui.label(localization::tr("• 对齐补白不计入 Unity Border；仅写回已读取且受支持的单 Sprite meta，保留 GUID"));
                        ui.label(localization::tr("• 普通弹窗恢复焦点保留位置，首次打开或按钮唤醒时才居中"));
                        ui.add_space(8.0);
                        if ui.button(crate::localization::tr(if self.show_older_history { "收起历史版本" } else { "查看更多历史" })).clicked() {
                            self.show_older_history = !self.show_older_history;
                        }
                        if self.show_older_history {
                        ui.label(RichText::new("1.2.1").strong());
                        ui.label(localization::tr("• 替换 AppIcon"));
                        ui.label(localization::tr("• 优化 Material 和 Shader 预览操作体验"));
                        ui.label(localization::tr("• 更新检查识别 GitHub 限流并等待重试，支持手动打开发布页"));
                        ui.add_space(8.0);

                        ui.label(RichText::new("1.2.0").strong());
                        ui.label(localization::tr("• 材质与 Shader 共用节点预览，分别载入材质输入与 Shader 默认值"));
                        ui.label(localization::tr("• 材质与 Shader 通过统一预览开关打开独立节点面板"));
                        ui.label(localization::tr("• Property 节点内直接编辑，RGBA / XYZW 逐行显示并按上下文展示分量连接"));
                        ui.label(localization::tr("• 交互参考 Amplify Shader Editor，模块内输入与重置，输出模块可切换 2D / 3D 并拉伸"));
                        ui.label(localization::tr("• 预览输入可临时调整与重置，不写回文件；渲染限受支持的 Built-in Shader"));
                        ui.add_space(8.0);

                        ui.label(RichText::new("1.1.9").strong());
                        ui.label(localization::tr("• 查找栏可收起并自适应换行，最近记录可删除或清空，按钮靠右对齐"));
                        ui.label(localization::tr("• 支持 HTTP / HTTPS 文件直链下载打开，浅色音频电平线更清晰"));
                        ui.label(localization::tr("• 设置集中管理文件关联与缓存清理，保留设置和最近文件"));
                        ui.label(localization::tr("• 音视频打开后自动播放并恢复上次位置，播放记录可清理；音量不显示数字"));
                        ui.label(localization::tr("• 弹窗新开或再次唤醒时居中并前置，设置长文案在右列换行"));
                        ui.label(localization::tr("• 支持本地 M3U / M3U8 播放列表、独立续播与可调宽侧栏；HLS 暂不支持"));
                        ui.label(localization::tr("• 音频列表支持顺序、随机及单曲循环；各列表片头片尾可按当前播放时间设置"));
                        ui.label(localization::tr("• 列表播放以矢量下一项按钮替代停止，窗口与全屏一致"));
                        ui.label(localization::tr("• 列表侧栏仅占媒体画面区域，信息与时间轴保持全宽；保存按钮居中放大"));
                        ui.label(localization::tr("• 全屏播放列表采用半透明浮层，不挤占视频画面"));
                        ui.label(localization::tr("• 音视频音频信息显示码率，列表箭头仅在媒体区域鼠标活动时显示"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.8").strong());
                        ui.label(localization::tr("• 场景接入有限 Built-in Sprite Shader GPU 预览；URP 与复杂 Shader 暂不支持"));
                        ui.label(localization::tr("• GPU 网格按材质槽绘制，支持同一子网格多材质叠加"));
                        ui.label(localization::tr("• 支持单蒙皮 FBX 静态导入姿态，骨骼覆盖与动画暂未还原"));
                        ui.label(localization::tr("• 修正静态 FBX 轴向及可唯一确认的根骨骼缩放"));
                        ui.label(localization::tr("• 静态模型兼容 PF3D2 材质颜色与透明度，保留多材质槽"));
                        ui.label(localization::tr("• Tilemap 材质接入 GPU，空网格不再误报缺失"));
                        ui.label(localization::tr("• 场景与独立 Spine 接入受限 GPU Shader，特殊效果保留回退"));
                        ui.label(localization::tr("• 场景与 Prefab 支持基础粒子播放、暂停和重播，复杂模块仍有限制"));
                        ui.label(localization::tr("• Prefab 根节点可打开源资源，基础粒子支持有限预热"));
                        ui.label(localization::tr("• 项目索引磁盘缓存优先加载，每次启动后台刷新一次"));
                        ui.label(localization::tr("• 场景点选嵌套 Prefab 自动定位 Hierarchy 节点"));
                        ui.label(localization::tr("• 提高 Prefab 展开预算；GPU 首次渲染后离焦冻结，返回恢复"));
                        ui.label(localization::tr("• 修复夜色参数绑定；预览增加悬浮 FPS 与鼠标位置缩放"));
                        ui.label(localization::tr("• TEXCOORD1/2 输入暂填零，粒子顶点流数据待接入"));
                        ui.label(localization::tr("• 对象名称的 Unicode 转义显示为实际文字"));
                        ui.label(localization::tr("• Prefab 展开限制详情支持一键复制"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.7").strong());
                        ui.label(localization::tr("• 最近记录支持 10～50 条与清空；联网更新跟随 Windows 系统代理"));
                        ui.label(localization::tr("• 弹窗使用独立系统窗口，优化多语言设置布局与关于窗口高度"));
                        ui.label(localization::tr("• 检查更新结果显示在设置底部，有新版本时打开统一更新窗口"));
                        ui.label(localization::tr("• 音视频支持空格播放、视频全屏与音频峰值拖尾"));
                        ui.label(localization::tr("• 改善 FFmpeg 安装识别，关闭设置自动刷新失败媒体页"));
                        ui.add_space(8.0);

                        ui.label(RichText::new("1.1.6").strong());
                        ui.label(localization::tr("• 音视频预览与播放、视频逐帧与硬解选项，改善资源索引及界面响应"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.5").strong());
                        ui.label(localization::tr("• Spine 3.8、FBX / OBJ 模型及场景 2D / 3D 预览；页签拖拽排序"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.4").strong());
                        ui.label(localization::tr("• 场景 / Tilemap 与嵌套 Prefab 预览、后台加载及 Inspector 优化"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.3").strong());
                        ui.label(localization::tr("• Prefab 静态 2D 与部分 UI 预览，支持选择定位、平移与缩放"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.2").strong());
                        ui.label(localization::tr("• 新增在线更新：启动自动检查、New 提示、更新说明与下载进度"));
                        ui.label(localization::tr("• 下载文件校验，保存提醒与二次确认，替换程序后自动重启"));
                        ui.add_space(8.0);
                        ui.label(RichText::new("1.1.1").strong());
                        ui.label(crate::localization::tr("• 更新深浅主题配色，优化界面与设置体验"));
                        ui.label(crate::localization::tr("• 新增简中、繁中、英语、日语、韩语、德语、法语，支持即时切换并记住选择"));
                        ui.label(crate::localization::tr("• 优化编辑模式视觉提示和更新历史展示"));
                        ui.label(crate::localization::tr("• 新增 AnimationClip 时间轴编辑功能"));
                        ui.add_space(8.0);
                        ui.add_space(6.0);
                        ui.label(RichText::new("1.1.0").strong());
                        ui.label(crate::localization::tr("• 属性与表格编辑，显式保存与放弃，修改整行高亮和逐项还原"));
                        ui.label(crate::localization::tr("• 支持已识别的数值、字符串、布尔值、枚举、向量、颜色和渐变关键点编辑"));
                        ui.label(crate::localization::tr("• 支持 Transform / RectTransform、材质字段和列表 / 数组元素编辑"));
                        ui.label(crate::localization::tr("• 每个文件独立撤销记录，默认 200 步，可在设置中调整到 50～1000"));
                        ui.label(crate::localization::tr("• 保存后保留撤销历史，Ctrl+Z 自动进入编辑模式；关闭文件或放弃编辑后清空"));
                        ui.label(crate::localization::tr("• 切换标签保留暂存修改，保存或放弃前阻止关闭编辑中的文件和应用"));
                        ui.label(crate::localization::tr("• 属性类型验证与引用保护；原始 YAML 保持只读"));
                        ui.label(crate::localization::tr("• 保存前检查外部改动，精确写回值片段并保留未修改文本与换行"));
                        ui.label(crate::localization::tr("• 设置弹窗整合关于、主题和撤销步数，记住用户设置"));
                        ui.add_space(6.0);
                        ui.label(RichText::new("1.0.0").strong());
                        ui.label(crate::localization::tr("首个正式版本。"));
                        ui.add_space(4.0);
                        ui.label(crate::localization::tr("• 支持 Unity YAML 资源的层级与 Inspector 浏览"));
                        ui.label(crate::localization::tr("• 支持搜索、多标签页、GUID / FileID 复制和编码识别"));
                        ui.label(crate::localization::tr("• 提供动画、材质、动画控制器等常见资源的结构化展示"));
                        ui.label(crate::localization::tr("• 只读浏览资源，不写入原始文件"));
                        }
                    });
                });
        }
        if self.pending_close_tab.is_none() && self.pending_playlist_close.is_none() && self.pending_save.is_none() {
            while let Some(path) = self.close_group_queue.pop_front() {
                if let Some(index) = self.tabs.iter().position(|tab| tab.path == path) {
                    self.close_tab(index, ctx);
                    if self.pending_close_tab.is_some() || self.pending_playlist_close.is_some() { break; }
                }
            }
        }
        if self.close_group_queue.is_empty() && self.pending_close_tab.is_none() && self.pending_playlist_close.is_none() {
            self.closing_tab_group = None;
        }
        if let Some(path) = self.pending_save.take() {
            if let Some(session) = self.edits.get(&path).cloned() {
                let result = editing::save(&session.lock().unwrap());
                match result {
                    Ok(text) => {
                        session.lock().unwrap().mark_saved(&text);
                        self.saved_histories.insert(path.clone(), session.clone());
                        self.edits.remove(&path);
                        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == path) {
                            tab.raw = text.clone().into();
                            tab.documents = parse_unity_yaml(&tab.raw);
                            tab.encoding = "UTF-8".into();
                        }
                        if self.path.as_ref() == Some(&path) {
                            self.raw = text.into();
                            self.documents = parse_unity_yaml(&self.raw);
                            self.encoding = "UTF-8".into();
                            self.status = crate::localization::tr("编辑已保存到文件。").into();
                            self.save_active_tab();
                            editing::publish(ctx, None, 0..0);
                        }
                    }
                    Err(message) => session.lock().unwrap().warning = Some(message),
                }
            }
        }
        if let Some(path) = self.pending_close_tab.clone() {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or(crate::localization::tr("未命名"));
            let mut discard = false;
            let mut save = false;
            let mut cancel = false;
            crate::native_window::Window::new(crate::localization::tr("未保存的修改")).id(egui::Id::new("discard_edit_on_close")).modal().resizable(false).default_width(420.0).show(ctx, |ui| {
                ui.heading(crate::localization::tr("未保存的修改"));
                ui.label(crate::lf!("“{0}”有未保存的修改。是否放弃编辑并关闭文件？", name));
                ui.label(crate::localization::tr("可保存后关闭、放弃修改，或取消关闭。"));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    save = ui.button(localization::tr("保存并关闭")).clicked();
                    discard = ui.add_sized([110.0, 30.0], egui::Button::new(crate::localization::tr("是，放弃并关闭"))).clicked();
                    cancel = ui.add_sized([88.0, 30.0], egui::Button::new(crate::localization::tr("否，继续编辑"))).clicked();
                });
            });
            if save { self.pending_save = Some(path.clone()); }
            if !self.edits.contains_key(&path) || discard {
                self.pending_close_tab = None;
                self.edits.remove(&path);
                if let Some(index) = self.tabs.iter().position(|tab| tab.path == path) {
                    self.close_tab(index,ctx);
                }
                editing::publish(ctx, None, 0..0);
            } else if cancel {
                self.pending_close_tab = None;
                self.close_group_queue.clear();self.closing_tab_group=None;
            }
            if save || discard || cancel || self.pending_close_tab.is_none() { ctx.request_repaint(); }
        }
        for session in self.edits.values() {
            let message = session.lock().unwrap().warning.clone();
            if let Some(message) = message {
                crate::native_window::Window::new(crate::localization::tr("⚠ 编辑提示")).id(egui::Id::new("edit_warning")).collapsible(false).resizable(false).show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        let visuals = ui.visuals();
                        let button = egui::Button::new(crate::localization::tr("确定"))
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
    parse_unity_yaml_cancellable(text, None)
}

fn parse_unity_yaml_cancellable(text: &str, cancelled: Option<&AtomicBool>) -> Vec<Document> {
    parse_unity_yaml_with_progress(text,cancelled,None)
}

fn parse_unity_yaml_with_progress(text: &str, cancelled: Option<&AtomicBool>, progress: Option<&AtomicUsize>) -> Vec<Document> {
    static PATTERNS: std::sync::OnceLock<(Regex, Regex, Regex)> = std::sync::OnceLock::new();
    let (header, go, father) = PATTERNS.get_or_init(|| (
        Regex::new(r"(?m)^--- !u!(\d+) &(-?\d+).*$").unwrap(),
        Regex::new(r"(?m)^\s*m_GameObject:\s*\{fileID:\s*(-?\d+)").unwrap(),
        Regex::new(r"(?m)^\s*m_Father:\s*\{fileID:\s*(-?\d+)").unwrap(),
    ));
    let mut starts = Vec::new();
    for start in header.find_iter(text) {
        if cancelled.is_some_and(|flag|flag.load(Ordering::Relaxed)) {return Vec::new();}
        starts.push(start);
        if let Some(progress)=progress {progress.store(100*start.end()/text.len().max(1),Ordering::Relaxed);}
    }
    let mut docs = Vec::new();
    let mut scanned = 0;
    let mut line = 1;
    for i in 0..starts.len() {
        if cancelled.is_some_and(|flag|flag.load(Ordering::Relaxed)) {return Vec::new();}
        let mat = starts[i];
        line += text[scanned..mat.start()].bytes().filter(|&b|b==b'\n').count();
        scanned = mat.start();
        let end = starts.get(i + 1).map_or(text.len(), |next| next.start());
        if let Some(progress)=progress {progress.store(100+800*end/text.len().max(1),Ordering::Relaxed);}
        let captures = header.captures(mat.as_str()).unwrap();
        let section: Arc<str> = Arc::from(&text[mat.start()..end]);
        let type_id = captures[1].to_string();
        let fallback_type_name = match type_id.as_str() {
            "1" => "GameObject",
            "4" => "Transform",
            "224" => "RectTransform",
            "20" => "Camera",
            "23" => "MeshRenderer",
            "198" => "ParticleSystem",
            "199" => "ParticleSystemRenderer",
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
            start_line: line,
            start_offset: mat.start(),
            game_object: go.captures(&section).map(|c| c[1].to_string()),
            parent_transform: father.captures(&section).map(|c| c[1].to_string()),
            text: section,
        });
    }
    docs
}

fn name_from_text(text: &str) -> Option<String> {
    text.lines().find_map(|line| line.trim_start().strip_prefix("m_Name:"))
        .map(|value| {let value=value.trim(); if value.starts_with('"') && value.contains("\\u") {serde_json::from_str::<String>(value).unwrap_or_else(|_|value.to_owned())}else{value.to_owned()}})
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod name_tests {
    #[test]
    fn shows_serialized_name_quotes_without_reading_the_next_field() {
        assert_eq!(super::name_from_text("AnimatorState:\n  m_Name: \"0\"\n"), Some("\"0\"".into()));
        assert_eq!(super::name_from_text("AnimatorState:\n  m_Name: '0'\n"), Some("'0'".into()));
        assert_eq!(super::name_from_text("AnimatorState:\n  m_Name:\n  m_Speed: 1\n"), None);
        assert_eq!(super::name_from_text("AnimatorState:\n  m_Name: 0\n"), Some("0".into()));
    }
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
                existing.keyframes.dedup_by(|left, right| {
                    if (left.time-right.time).abs() < f32::EPSILON {
                        right.time_lines.append(&mut left.time_lines);
                        true
                    } else { false }
                });
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
                    time_lines: vec![line_index.saturating_sub(1)],
                });
            }
        }
    }
    if active_curve {
        add_binding(&mut path, &mut attribute, &mut class_id, &mut keyframes);
    }
    drop(add_binding);
    attach_animation_runtime_times(&mut root, text);
    root
}

// Transform curves have one runtime curve with a vector value and separate
// editor curves per component. Move their time fields together.
fn attach_animation_runtime_times(tree: &mut AnimationPathNode, text: &str) {
    let mut section = "";
    let mut path = String::new();
    let mut times = Vec::new();
    let mut curves = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if (line.starts_with("  m_") || trimmed == "- curve:") && !times.is_empty() {
            curves.push((section.to_owned(), path.clone(), std::mem::take(&mut times)));
            path.clear();
        }
        if line.starts_with("  m_") { section = trimmed.split(':').next().unwrap_or(""); }
        if !matches!(section, "m_PositionCurves" | "m_ScaleCurves" | "m_RotationCurves" | "m_EulerCurves") { continue; }
        if let Some(value) = trimmed.strip_prefix("path:") { path = yaml_strings::display(value.trim()).into_owned(); }
        if let Some(time) = trimmed.strip_prefix("time:").and_then(|value| value.trim().parse::<f32>().ok()).filter(|time| time.is_finite()) { times.push((time, line_index)); }
    }
    if !times.is_empty() { curves.push((section.to_owned(), path, times)); }
    fn attach(node: &mut AnimationPathNode, curves: &[(String, String, Vec<(f32, usize)>)]) {
        for binding in &mut node.bindings {
            for (section, path, times) in curves {
                let root = match section.as_str() { "m_PositionCurves" => "m_LocalPosition", "m_ScaleCurves" => "m_LocalScale", "m_RotationCurves" => "m_LocalRotation", "m_EulerCurves" => "localEulerAnglesRaw", _ => continue };
                if binding.path != *path || !matches!(binding.class_name.as_str(), "Transform" | "RectTransform") || !is_group_component(&binding.attribute, root) { continue; }
                for key in &mut binding.keyframes {
                    key.time_lines.extend(times.iter().filter(|(time, _)| (key.time-time).abs() < f32::EPSILON).map(|(_, line)| *line));
                    key.time_lines.sort_unstable(); key.time_lines.dedup();
                }
            }
        }
        for child in node.children.values_mut() { attach(child, curves); }
    }
    attach(tree, &curves);
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

#[cfg(test)]
mod animation_key_drag_tests {
    use super::*;
    #[test]
    fn key_times_link_runtime_and_both_editor_copies() {
        let text = "AnimationClip:\n  m_ScaleCurves:\n  - curve:\n      m_Curve:\n      - serializedVersion: 3\n        time: 0.1\n        value: {x: 1, y: 1, z: 1}\n    path: Child\n  m_FloatCurves:\n  - curve:\n      m_Curve:\n      - serializedVersion: 3\n        time: 0.1\n        value: 1\n    attribute: m_LocalScale.x\n    path: Child\n    classID: 4\n  m_EditorCurves:\n  - curve:\n      m_Curve:\n      - serializedVersion: 3\n        time: 0.1\n        value: 1\n    attribute: m_LocalScale.x\n    path: Child\n    classID: 4\n  m_StopTime: 1\n";
        let tree = animation_path_tree(text);
        let mut bindings = Vec::new();
        collect_animation_bindings(&tree, &mut bindings);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].keyframes.len(), 1);
        let key = &bindings[0].keyframes[0];
        assert_eq!(key.time_lines.len(), 3);
        let lines: Vec<_> = text.lines().collect();
        for line in &key.time_lines { assert_eq!(lines[*line].trim_start_matches([' ', '-']), "time: 0.1"); }
    }
}

fn show_animation_path_node(
    ui: &mut egui::Ui,
    node: &AnimationPathNode,
    clip_index: usize,
    selected: &mut Option<usize>,
    selected_track: &mut Option<AnimationTrackSelection>,
) {
    for (name, child) in &node.children {
        egui::CollapsingHeader::new(if name == "根对象（本对象）" { localization::tr(name) } else { name })
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
        .all(|suffix| matches!(*suffix, "x" | "y" | "z" | "w"))
    {
        &["x", "y", "z", "w"]
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
    if matches!(component, "r" | "g" | "b" | "a" | "x" | "y" | "z" | "w") {
        Some(root)
    } else {
        None
    }
}

fn is_group_component(attribute: &str, root: &str) -> bool {
    attribute
        .strip_prefix(root)
        .is_some_and(|suffix| matches!(suffix, ".r" | ".g" | ".b" | ".a" | ".x" | ".y" | ".z" | ".w"))
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
    text: &str,
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
        &["x", "y", "z", "w"]
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
            .and_then(|binding| animation_curves::value_at(text,binding,time).or_else(||curve_value_at(binding,time)))
        {
            values.push(((*component).to_string(), value));
        }
    }
    (!values.is_empty()).then_some(values)
}

/// Shared time-to-position mapping for the ruler and keyframes. Future dragging
/// can invert this mapping and snap the resulting time to the sample rate.
fn animation_time_rect(rect: egui::Rect) -> egui::Rect {
    rect.shrink2(egui::vec2(12.0_f32.min(rect.width().max(0.0) * 0.25), 0.0))
}

fn animation_time_x(rect: egui::Rect, time: f64, duration: f64) -> f32 {
    let rect = animation_time_rect(rect);
    let fraction = if duration.is_finite() && duration > 0.0 { (time / duration).clamp(0.0, 1.0) } else { 0.0 };
    egui::lerp(rect.left()..=rect.right(), fraction as f32)
}

fn show_animation_frame_ruler(ui: &mut egui::Ui, duration: f32, sample_rate: Option<f32>, clip_duration: f32, can_insert: bool) -> Option<f64> {
    let Some(rate) = sample_rate.filter(|rate| rate.is_finite() && *rate > 0.0) else { return None };
    if !duration.is_finite() || duration <= 0.0 { return None; }
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
    let frames = duration as f64 * rate as f64;
    let last_frame = frames.floor().min(u32::MAX as f64) as u32;
    let time_rect = animation_time_rect(rect);
    let pixels_per_frame = time_rect.width() as f64 / frames;
    if !pixels_per_frame.is_finite() || pixels_per_frame <= 0.0 { return None; }
    // Keep long clips responsive and avoid overlapping labels at low density.
    let minor_step = (2.0 / pixels_per_frame).ceil().max(1.0).min(u32::MAX as f64) as u32;
    let major_step = ((2.0 / (pixels_per_frame * 5.0)).ceil().max(1.0) * 5.0).min(u32::MAX as f64) as u32;
    let label_step = ((36.0 / (pixels_per_frame * 5.0)).ceil().max(1.0) * 5.0).min(u32::MAX as f64) as u32;
    let minor_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let major_color = ui.visuals().weak_text_color();
    for frame in (0..=last_frame).step_by(minor_step as usize) {
        let x = animation_time_x(rect, frame as f64 / rate as f64, duration as f64);
        ui.painter().line_segment([egui::pos2(x, rect.bottom()-4.0), egui::pos2(x, rect.bottom())], egui::Stroke::new(1.0_f32, minor_color));
    }
    for frame in (0..=last_frame).step_by(major_step as usize) {
        let x = animation_time_x(rect, frame as f64 / rate as f64, duration as f64);
        ui.painter().line_segment([egui::pos2(x, rect.bottom()-10.0), egui::pos2(x, rect.bottom())], egui::Stroke::new(1.0_f32, major_color));
    }
    let end_x = animation_time_x(rect, clip_duration as f64, duration as f64);
    let end_frame = (clip_duration as f64 * rate as f64).round();
    let end_text = format!("End {end_frame:.0}");
    let end_galley = ui.painter().layout_no_wrap(end_text, egui::FontId::proportional(10.0), Color32::WHITE);
    let badge_width = end_galley.size().x + 8.0;
    let badge_left = (end_x-badge_width*0.5).clamp(rect.left(), (rect.right()-badge_width).max(rect.left()));
    // The badge padding surrounds text at the same top as ordinary frame labels.
    let badge = egui::Rect::from_min_size(egui::pos2(badge_left,rect.top()-3.0),egui::vec2(badge_width,end_galley.size().y+6.0));
    for frame in (0..=last_frame).step_by(label_step as usize) {
        let x = animation_time_x(rect, frame as f64 / rate as f64, duration as f64);
        if x >= badge.left()-12.0 && x <= badge.right()+12.0 { continue; }
        let alignment = if frame == 0 { egui::Align2::LEFT_TOP } else if x > rect.right()-14.0 { egui::Align2::RIGHT_TOP } else { egui::Align2::CENTER_TOP };
        ui.painter().text(egui::pos2(x, rect.top()), alignment, frame.to_string(), egui::FontId::proportional(10.0), major_color);
    }
    let end_color = Color32::from_rgb(210, 92, 45);
    ui.painter().line_segment([egui::pos2(end_x,badge.bottom()),egui::pos2(end_x,rect.bottom())],egui::Stroke::new(2.0_f32,end_color));
    ui.painter().rect_filled(badge,3.0,end_color);
    let text_position = badge.center() - end_galley.size() * 0.5;
    ui.painter().galley(text_position,end_galley,Color32::WHITE);
    let position_id = response.id.with("insert_frame");
    if response.secondary_clicked() && let Some(position) = response.interact_pointer_pos() {
        let frame = (((position.x-time_rect.left())/time_rect.width()) as f64 * frames).round().clamp(0.0,last_frame as f64);
        ui.ctx().data_mut(|data|data.insert_temp(position_id,frame / rate as f64));
    }
    let mut insert = None;
    response.context_menu(|ui| {
        if let Some(time)=ui.ctx().data_mut(|data|data.get_temp::<f64>(position_id)) {
            ui.label(format!("{}: {}",localization::tr("帧"),(time*rate as f64).round()));
            if ui.add_enabled(editing::active(ui) && can_insert,egui::Button::new(localization::tr("新增关键帧"))).clicked() { insert=Some(time);ui.close_menu(); }
        }
    });
    insert
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
            let x = animation_time_x(rect, keyframe.time as f64, stop_time as f64);
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
    timeline_edit::publish_owner(ui);
    let edited_text = editing::scope_value(ui);
    let text = edited_text.as_deref().unwrap_or(text);
    let tree = animation_path_tree(text);
    let mut bindings = Vec::new();
    collect_animation_bindings(&tree, &mut bindings);
    let clip_duration = animation_stop_time(text, &bindings);
    let preview_duration = animation_sample_rate(text).filter(|rate|rate.is_finite() && *rate>0.0)
        .map_or(clip_duration, |rate| clip_duration + 10.0 / rate);
    let stop_time = timeline_edit::display_duration(ui, preview_duration);
    let Some(track) = selected_track else {
        show_animation_frame_ruler(ui, stop_time, animation_sample_rate(text),clip_duration,false);
        show_animation_overview(ui, &bindings, stop_time);
        ui.small(crate::localization::tr("从左侧路径树选择一个动画属性，可查看其独立时间轴。"));
        return;
    };

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
    ui.label(crate::lf!("当前属性：{0} · {1}",
        track.class_name,
        group_root.unwrap_or(&track.attribute)));
    let original_item_spacing_y = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    let zoom_id=ui.id().with("timeline_zoom");
    let mut zoom=ui.ctx().data_mut(|data|data.get_temp::<f32>(zoom_id)).unwrap_or(1.0);
    ui.horizontal(|ui| {
        ui.label(localization::tr("缩放"));
        theme::zoom_slider(ui, &mut zoom, 20.0);
        if ui.small_button(localization::tr("适应窗口")).clicked(){zoom=1.0;}
    });
    ui.ctx().data_mut(|data|data.insert_temp(zoom_id,zoom));
    let width=ui.available_width(); let mut inserted=false;
    let scroll_id=ui.id().with("timeline_scroll_x");
    let scroll_x=ui.ctx().data_mut(|d|d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
    let timeline_output=egui::ScrollArea::horizontal().id_salt("animation_timeline_zoom").horizontal_scroll_offset(scroll_x).show(ui,|ui| {
        ui.set_min_width(width*zoom);
        if let Some(time)=show_animation_frame_ruler(ui,stop_time,animation_sample_rate(text),clip_duration,!matching.is_empty()) {
            inserted=animation_curves::insert(ui,&matching,time);
        }
        let overview_rect=show_animation_overview(ui,&bindings,stop_time);
        let result=if inserted {(None,None)} else {timeline_edit::show(ui,&matching,stop_time,animation_sample_rate(text),group_root.is_some(),selected_keyframe)};
        (result,overview_rect)
    });
    ui.ctx().data_mut(|d|d.insert_temp(scroll_id,timeline_output.state.offset.x));
    let ((selected_marker_x,current_track_rect),overview_rect)=timeline_output.inner;
    if inserted || editing::edit_revision(ui).is_none() && edited_text.is_some() {
        ui.spacing_mut().item_spacing.y=original_item_spacing_y;*selected_keyframe=None;return;
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
    if editing::active(ui) {
        ui.small(localization::tr("点击选择 · Ctrl / Shift 多选 · 空白处拖动框选 · 拖动选中关键帧按帧移动"));
    }
    let tab_id=ui.id().with("animation_editor_tab");
    let mut curves=ui.ctx().data_mut(|data|data.get_temp::<bool>(tab_id)).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut curves,false,localization::tr("属性编辑"));
        ui.selectable_value(&mut curves,true,localization::tr("曲线编辑"));
    });
    ui.ctx().data_mut(|data|data.insert_temp(tab_id,curves));
    if curves {animation_curves::show(ui,text,&matching,stop_time,selected_keyframe);return;}
    if let Some(keyframe) = selected_keyframe {
        ui.separator();
        ui.label(crate::lf!("关键帧：{0}　·　时间：{1:.3}s",
            keyframe.track_name, keyframe.time));
        if let Some(values) = grouped_keyframe_values(text,&bindings, track, keyframe.time) {
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
                let can_show_swatch = ["r", "g", "b"]
                    .iter()
                    .all(|component| values.iter().any(|(present, _)| present == component));
                ui.horizontal(|ui| {
                    show_animation_components(ui, &values, &bindings, track, keyframe.time);
                    if can_show_swatch {
                        let rgba = [channel("r",0.0) as f64,channel("g",0.0) as f64,channel("b",0.0) as f64,channel("a",1.0) as f64];
                        let channels: Option<Vec<_>> = ["r","g","b","a"].iter().map(|component| {
                            matching.iter().find(|binding| binding.attribute.rsplit('.').next()==Some(*component))
                                .and_then(|binding| binding.keyframes.iter().find(|key| (key.time-keyframe.time).abs()<1e-6))
                        }).collect();
                        if let Some(channels) = channels {
                            if let Some(color) = ui.push_id(("animation_color",keyframe.yaml_line), |ui| color_property::show(ui,group_root.unwrap_or("Color"),rgba)).inner {
                                if editing::active(ui) {
                                    let values: Vec<_> = channels.iter().zip(color).flat_map(|(key,value)| key.time_lines.iter().map(move |line| (*line+1,value))).collect();
                                    editing::set_animation_values(ui,&values);
                                }
                            }
                        } else {
                            let (swatch, _) = ui.allocate_exact_size(egui::vec2(120.0,19.0),egui::Sense::hover());
                            color_property::paint_swatch(ui,swatch,rgba);
                        }
                    }
                });
            } else {
                ui.horizontal_wrapped(|ui| show_animation_components(ui, &values, &bindings, track, keyframe.time));
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(localization::tr("值"));
                let value = bindings.iter().flat_map(|binding| &binding.keyframes)
                    .find(|key| key.yaml_line == keyframe.yaml_line)
                    .map_or(keyframe.value.as_str(), |key| key.value.as_str());
                show_animation_value(ui, value, Some(keyframe.yaml_line + 1));
            });
        }
    }
}

fn show_animation_value(ui: &mut egui::Ui, value: &str, value_line: Option<usize>) {
    if let Some(bind) = value_line.filter(|_| editing::active(ui)).and_then(|line| editing::line_binding(ui, line, "value")) {
        ui.push_id(value_line, |ui| {
            editing::text(ui, &bind, "float", 90.0);
            if bind.dirty() && ui.small_button(localization::tr("还原")).clicked() { bind.restore(); }
        });
    } else {
        let mut display = value.to_owned();
        ui.add(TextEdit::singleline(&mut display).desired_width(90.0).interactive(false));
    }
}

fn show_animation_components(ui: &mut egui::Ui, values: &[(String, String)], bindings: &[&AnimationBinding], track: &AnimationTrackSelection, time: f32) {
    let root = curve_group_root(&track.attribute);
    for (component, value) in values {
        ui.colored_label(animation_curves::color(&format!("{root:?}.{component}")), component);
        let value_line = bindings.iter().find(|binding| {
            binding.path == track.path && binding.class_name == track.class_name
                && curve_group_root(&binding.attribute) == root
                && binding.attribute.rsplit('.').next() == Some(component.as_str())
        }).and_then(|binding| binding.keyframes.iter().find(|key| key.time == time)).map(|key| key.yaml_line + 1);
        show_animation_value(ui, value, value_line);
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
    ScrollArea::both().id_salt("transform_properties").auto_shrink([false, false]).show(ui, |ui| show_transform_properties_content(ui, text));
}
fn show_transform_properties_content(ui: &mut egui::Ui, text: &str) {
    let viewport_width = ui.available_width();
        ui.small(crate::localization::tr("旋转以欧拉角（度）显示；编辑模式中的改动需要保存才会写入文件。"));
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
                            ui.colored_label(Color32::RED, crate::localization::tr("无法识别有效数值"));
                            ui.monospace(raw_value.trim());
                        }
                    });
                    let mut rect = response.response.rect;
                    rect.max.x = ui.max_rect().right();
                    editing::highlight(ui, highlight, rect, editing::binding(ui, name, raw_value.trim()).is_some_and(|binding| binding.dirty()));
                }
            });
        ui.separator();
        ui.collapsing(crate::localization::tr("其他字段（原始 YAML）"), |ui| {
            let mut remaining = remaining.join("\n");
            ui.add(TextEdit::multiline(&mut remaining)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY).interactive(false));
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
                    else { std::borrow::Cow::Borrowed(if matches!(header.as_str(), "索引" | "值") { localization::tr(raw_text) } else { raw_text }) };
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
                        if child.small_button(crate::localization::tr("还原")).clicked() { editing::restore_scope(&child); }
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
                    else { crate::lf!("{0}\n原始 YAML：{1}", text, raw_text) });
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
    ui.label(RichText::new(crate::lf!("{0}（{1} 条）", table.title, table.rows.len())).strong());
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
                    .unwrap_or(crate::localization::tr("条目"))
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
    let transforms:HashMap<_,_>=docs.iter().filter(|doc|matches!(doc.type_name.as_str(),"Transform"|"RectTransform")).map(|doc|(doc.file_id.as_str(),doc)).collect();
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
            let mut parent=doc.parent_transform.as_deref();
            let mut visited=std::collections::HashSet::new();
            let mut parent_go=None;
            while let Some(id)=parent.filter(|id|*id!="0") {
                if !visited.insert(id) {break;}
                if let Some(go)=transform_to_go.get(id) {parent_go=Some(go);break;}
                parent=transforms.get(id).and_then(|doc|doc.parent_transform.as_deref());
            }
            match parent_go {
                Some(parent_go) if go_indices.contains_key(own) => children
                    .entry(parent_go.clone())
                    .or_default()
                    .push(*go_indices.get(own).unwrap()),
                _ => {
                    if let Some(&index)=go_indices.get(own) {roots.push(index);} else {unlinked.push(i);continue;}
                },
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

fn hierarchy_default_open(preference: &str, depth: usize) -> bool {
    preference == "all" || depth < preference.parse::<usize>().unwrap_or(3)
}

fn settings_label(ui: &mut egui::Ui, label: &str) {
    // Allocate a cell-sized label directly, so the grid centers it against the whole right group.
    let height = 30.0;
    ui.allocate_ui_with_layout(egui::vec2(128.0, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(128.0);
        ui.set_max_width(128.0);
        ui.add(egui::Label::new(label).wrap());
    });
}

fn settings_group<R>(ui: &mut egui::Ui, key: &str, contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    // Grid alignment happens before child contents are laid out. Remember the group's
    // measured height so it starts at the top of its centered cell rather than its middle.
    let id = ui.make_persistent_id(key);
    let height = ui.ctx().data_mut(|data| data.get_temp::<f32>(id)).unwrap_or(30.0);
    let response = ui.allocate_ui_with_layout(egui::vec2(208.0, height), egui::Layout::top_down(egui::Align::Min), contents);
    let measured = response.response.rect.height();
    if (measured - height).abs() > 0.5 {
        ui.ctx().data_mut(|data| data.insert_temp(id, measured));
        ui.ctx().request_discard("settings group height changed");
    }
    response
}

fn settings_control<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(egui::vec2(208.0, 30.0), egui::Layout::right_to_left(egui::Align::Center), contents)
}

/// Place scattered members at their first member's slot, retaining all file order within the group.
fn packed_tab_group_indices(count: usize, members: &[usize]) -> Vec<usize> {
    let Some(&first) = members.first() else { return (0..count).collect(); };
    (0..first).chain(members.iter().copied()).chain((first..count).filter(|index| !members.contains(index))).collect()
}

/// Translate a visible insertion boundary into a complete file permutation.
/// A collapsed group is one block; member order is preserved when it moves or expands.
fn reordered_tab_indices(blocks: &[Vec<usize>], from: usize, slot: usize) -> Option<Vec<usize>> {
    if from >= blocks.len() || slot > blocks.len() { return None; }
    let to = if slot > from { slot - 1 } else { slot };
    if from == to { return None; }
    let mut reordered: Vec<_> = blocks.iter().collect();
    let moving = reordered.remove(from);
    reordered.insert(to, moving);
    Some(reordered.into_iter().flatten().copied().collect())
}

fn tab_drop_slot(rects:&[egui::Rect],pointer:egui::Pos2)->usize {
    let Some(nearest)=rects.iter().min_by(|a,b|(a.center().y-pointer.y).abs().total_cmp(&(b.center().y-pointer.y).abs())) else{return 0};
    let row_y=nearest.center().y;
    let mut end=0;
    for (index,rect) in rects.iter().enumerate().filter(|(_,r)|(r.center().y-row_y).abs()<1.0) {
        if pointer.x<rect.center().x{return index;}end=index+1;
    }end
}
#[cfg(test)]
mod tab_reorder_tests {
    use super::*;
    #[test]
    fn tab_icon_states_do_not_change_strip_height() {
        for active in [0, 1, 2] {
            for pointer in [egui::pos2(40.0, 12.0), egui::pos2(160.0, 12.0), egui::pos2(280.0, 12.0), egui::pos2(480.0, 200.0)] {
                let ctx = egui::Context::default();
                let input = egui::RawInput { events: vec![egui::Event::PointerMoved(pointer)], ..Default::default() };
                let _ = ctx.run(input, |ctx| {
                    let mut ui = egui::Ui::new(ctx.clone(), egui::Id::new("tab_height_test"),
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 300.0))));
                    let (strip, _) = ui.allocate_exact_size(egui::vec2(500.0, 24.0), egui::Sense::hover());
                    let before = ui.min_rect();
                    for index in 0..3 {
                        let slot = egui::Rect::from_min_size(strip.min + egui::vec2(index as f32 * 120.0, 0.0), egui::vec2(112.0, 24.0));
                        let path = PathBuf::from(format!("file{index}.png"));
                        unified_tab(&mut ui, "file.png", active == index, &path, slot, if index == 1 { 12 } else { 1 });
                    }
                    assert_eq!(ui.min_rect(), before, "active={active}, pointer={pointer:?}");
                });
            }
        }
    }
    #[test]
    fn dissolving_group_stays_at_its_slot_and_preserves_member_order() {
        assert_eq!(packed_tab_group_indices(8, &[1, 3, 5, 7]), vec![0, 1, 3, 5, 7, 2, 4, 6]);
        assert_eq!(packed_tab_group_indices(5, &[0, 1, 2, 3]), vec![0, 1, 2, 3, 4]);
        assert_eq!(packed_tab_group_indices(3, &[]), vec![0, 1, 2]);
        assert!(!TabGroup { members: vec![0], selected: 0 }.is_collapsed());
        assert!(TabGroup { members: vec![0, 1], selected: 0 }.is_collapsed());
    }
    #[test]
    fn collapsed_groups_move_as_blocks_and_preserve_every_member() {
        // Group members can be interleaved in the stored file order before dragging.
        let blocks = vec![vec![0, 2, 4, 6], vec![1], vec![3, 5, 7, 8], vec![9]];
        assert_eq!(reordered_tab_indices(&blocks, 0, 4), Some(vec![1, 3, 5, 7, 8, 9, 0, 2, 4, 6]));
        assert_eq!(reordered_tab_indices(&blocks, 2, 0), Some(vec![3, 5, 7, 8, 0, 2, 4, 6, 1, 9]));
        assert_eq!(reordered_tab_indices(&blocks, 1, 3), Some(vec![0, 2, 4, 6, 3, 5, 7, 8, 1, 9]));
        assert_eq!(reordered_tab_indices(&blocks, 2, 2), None);
        assert_eq!(reordered_tab_indices(&blocks, 2, 3), None);
        assert_eq!(reordered_tab_indices(&blocks, 4, 0), None);
        assert_eq!(reordered_tab_indices(&blocks, 0, 5), None);
        let singles = vec![vec![0], vec![1], vec![2]];
        assert_eq!(reordered_tab_indices(&singles, 0, 3), Some(vec![1, 2, 0]));
    }
    #[test]
    fn insertion_handles_gaps_end_and_wrapped_rows() {
        let rect=|x,y|egui::Rect::from_min_size(egui::pos2(x,y),egui::vec2(100.0,24.0));
        let rects=[rect(0.0,0.0),rect(108.0,0.0),rect(0.0,28.0)];
        assert_eq!(tab_drop_slot(&rects,egui::pos2(5.0,12.0)),0);
        assert_eq!(tab_drop_slot(&rects,egui::pos2(104.0,12.0)),1);
        assert_eq!(tab_drop_slot(&rects,egui::pos2(300.0,12.0)),2);
        assert_eq!(tab_drop_slot(&rects,egui::pos2(300.0,40.0)),3);
    }
}
/// Keep the start, last two stem characters and extension when a tab is too wide.
fn shortened_tab_title(ui: &egui::Ui, title: &str, width: f32, font: &egui::FontId) -> String {
    let measure = |value: &str| ui.painter().layout_no_wrap(value.to_owned(), font.clone(), ui.visuals().text_color()).size().x;
    if measure(title) <= width { return title.to_owned(); }
    let (name, dirty) = title.strip_suffix(" *").map_or((title, ""), |name| (name, " *"));
    let (stem, extension) = name.rsplit_once('.').filter(|(stem, _)| !stem.is_empty())
        .map_or((name, String::new()), |(stem, extension)| (stem, format!(".{extension}")));
    let chars: Vec<char> = stem.chars().collect();
    let suffix_start = chars.len().saturating_sub(2);
    let suffix: String = chars[suffix_start..].iter().collect();
    let tail = format!("…{suffix}{extension}{dirty}");
    let mut low = 0;
    let mut high = suffix_start;
    while low < high {
        let count = (low + high).div_ceil(2);
        let prefix: String = chars[..count].iter().collect();
        if measure(&format!("{prefix}{tail}")) <= width { low = count; } else { high = count - 1; }
    }
    let prefix: String = chars[..low].iter().collect();
    format!("{prefix}{tail}")
}

/// Every visible tab is a nonempty group. Ordinary tabs have one member;
/// collapsed tabs keep all members and the selected representative together.
struct TabGroup {
    members: Vec<usize>,
    selected: usize,
}

impl TabGroup {
    fn is_collapsed(&self) -> bool { self.members.len() > 1 }
}

fn tab_group_key(path: &std::path::Path) -> String {
    path.extension().and_then(|extension| extension.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn tab_action_width(ui: &egui::Ui, member_count: usize) -> f32 {
    if member_count <= 1 { return 24.0; }
    26.0 + ui.painter().layout_no_wrap(member_count.to_string(), egui::FontId::proportional(12.0), ui.visuals().text_color()).size().x
}

fn unified_tab(ui: &mut egui::Ui, title: &str, is_active: bool,path:&std::path::Path,slot:egui::Rect, member_count: usize) -> (bool, bool,egui::Response,egui::Rect) {
    let grouped = member_count > 1;
    let tab_id=ui.id().with(path);
    let font = egui::FontId::proportional(14.0);
    let text_color = ui.visuals().text_color();
    let mut min=egui::pos2(ui.ctx().animate_value_with_time(tab_id.with("tab_x"),slot.left(),0.12),ui.ctx().animate_value_with_time(tab_id.with("tab_y"),slot.top(),0.12));
    let dragged=ui.ctx().data_mut(|d|d.get_temp::<PathBuf>(egui::Id::new("tab_drag_path"))).is_some_and(|p|p==path);
    if dragged {
        if let Some(pointer)=ui.input(|i|i.pointer.hover_pos()) {
            let offset=ui.ctx().data_mut(|d|d.get_temp::<egui::Vec2>(egui::Id::new("tab_drag_offset"))).unwrap_or(slot.size()*0.5);
            min=pointer-offset;
        }
    }
    let rect=egui::Rect::from_min_size(min,slot.size());
    let response=ui.interact(rect,tab_id.with("tab_button"),egui::Sense::click_and_drag());
    let painter=if dragged {ui.painter().clone().with_layer_id(egui::LayerId::new(egui::Order::Foreground,tab_id.with("dragged_tab")))}else{ui.painter().clone()};
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - tab_action_width(ui, member_count), rect.top()),
        egui::vec2(tab_action_width(ui, member_count), rect.height()),
    );
    let tab_hovered = response.hovered();
    let close_hovered = ui.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|pointer| close_rect.contains(pointer))
    });
    let response = if !close_hovered && !dragged { response.on_hover_text(path.display().to_string()) } else { response };
    let show_close = grouped || is_active || tab_hovered;
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
    painter.rect_filled(rect, 4.0, fill);
    if is_active || tab_hovered {
        painter
            .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
    }
    let text_rect=egui::Rect::from_min_max(egui::pos2(rect.left()+8.0,rect.top()),egui::pos2(close_rect.left(),rect.bottom()));
    let title = shortened_tab_title(ui, title, text_rect.width(), &font);
    painter.with_clip_rect(painter.clip_rect().intersect(text_rect)).text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        font,
        text_color,
    );
    if show_close {
        if close_hovered {
            painter.rect_filled(
                close_rect.shrink(3.0),
                3.0,
                ui.visuals().widgets.active.bg_fill,
            );
        }
        let color = if close_hovered { ui.visuals().widgets.active.fg_stroke.color } else { ui.visuals().weak_text_color() };
        // Use the same embedded vector assets as other menus, including on the drag layer.
        // Painting icons must not allocate another row in the parent tab strip.
        let icon_ui = ui.new_child(egui::UiBuilder::new().id_salt(tab_id.with("icon"))
            .max_rect(close_rect).layer_id(painter.layer_id()));
        let icon_rect = egui::Rect::from_center_size(egui::pos2(close_rect.left() + 12.0, close_rect.center().y), egui::vec2(12.0, 12.0));
        icons::paint(&icon_ui, if grouped { icons::Icon::TabGroup } else { icons::Icon::Close }, icon_rect, color);
        if grouped {
            painter.text(egui::pos2(close_rect.left() + 22.0, close_rect.center().y), egui::Align2::LEFT_CENTER,
                member_count.to_string(), egui::FontId::proportional(12.0), color);
        }
    }
    (
        response.clicked() && !close_hovered,
        show_close && response.clicked() && close_hovered,
        response,
        slot,
    )
}

fn show_game_object_properties(ui:&mut egui::Ui,text:&str) {
    egui::Grid::new("game_object_fields").num_columns(2).show(ui,|ui|{
        for (key,label,kind) in [("m_Name","名称","string"),("m_IsActive","激活（Active）","bool"),("m_Layer","Layer","int"),("m_TagString","Tag","string")] {
            let Some(raw)=static_preview::field(text,key) else {continue;};
            ui.label(localization::tr(label));
            if key=="m_IsActive" {
                if !editing::control(ui,key,raw,kind,None) {
                    let mut active=raw=="1" || raw=="true";ui.add_enabled(false,egui::Checkbox::without_text(&mut active));
                }
            } else {ui.label(raw);}
            ui.end_row();
        }
    });
}

fn main() -> eframe::Result {
    if let Some(code)=asset_viewer::nine_slice::cli::dispatch() {std::process::exit(code);}
    let (command_inbox, startup_paths) = match command_line::start() {
        Ok(command_line::Startup::Primary(inbox, paths)) => (inbox, paths),
        Ok(command_line::Startup::Forwarded) => return Ok(()),
        Err(error) => {
            rfd::MessageDialog::new().set_title("Unity YAML Viewer")
                .set_description(format!("无法打开命令行请求：{error}"))
                .set_level(rfd::MessageLevel::Error).show();
            return Ok(());
        }
    };
    eframe::run_native(
        "Unity YAML Viewer",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default().with_icon(application_icon()),
            ..Default::default()
        },
        Box::new(move |creation_context| {
            render_system::initialize(creation_context.gl.as_ref());
            configure_system_fonts(&creation_context.egui_ctx);
            restore_theme(&creation_context.egui_ctx);
            let mut app = ViewerApp {
                command_inbox,
                last_open_dir: load_last_open_dir(),
                settings: AppSettings::load(),
                decoder_settings: media_tools::DecoderSettings::default(),
                recent_files: recent_files::RecentFiles::load(),
                ..Default::default()
            };
            localization::set_language(&app.settings.language);
            app.restore_open_tabs();
            app.accept_open_request(startup_paths,&creation_context.egui_ctx);
            if app.settings.auto_check_updates { app.updater.check(&creation_context.egui_ctx, false); }
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
    theme::install(ctx);
    egui_extras::install_image_loaders(ctx);
    theme::set_preference(ctx, theme.as_deref().map(str::trim).unwrap_or("system"));
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
    fn yaml_positions_preserve_unicode_crlf_and_preamble() {
        let text="%YAML 1.1\r\n# 中文\r\n--- !u!1 &1\r\nGameObject:\r\n  m_Name: 云\r\n--- !u!4 &2\r\nTransform:\r\n  m_GameObject: {fileID: 1}\r\n";
        let docs=parse_unity_yaml(text);
        assert_eq!(docs.len(),2);
        assert_eq!(docs[0].start_line,3);assert_eq!(docs[1].start_line,6);
        for doc in &docs {
            assert_eq!(&text[doc.start_offset..doc.start_offset+doc.text.len()],doc.text.as_ref());
        }
        let cancelled=AtomicBool::new(true);
        assert!(parse_unity_yaml_cancellable(text,Some(&cancelled)).is_empty());
    }

    #[test]
    fn cloned_documents_share_large_text() {
        let docs=parse_unity_yaml("--- !u!1 &1\nGameObject:\n  m_Name: sample\n");
        let clone=docs[0].clone();
        assert!(Arc::ptr_eq(&clone.text,&docs[0].text));
    }

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
    let candidates: &[&[&str]] = &[
        &[r"C:\Windows\Fonts\segoeui.ttf"],
        &[r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\msyh.ttf", r"C:\Windows\Fonts\simhei.ttf"],
        &[r"C:\Windows\Fonts\YuGothR.ttc", r"C:\Windows\Fonts\meiryo.ttc"],
        &[r"C:\Windows\Fonts\malgun.ttf"],
    ];
    #[cfg(target_os = "macos")]
    let candidates: &[&[&str]] = &[
        &["/System/Library/Fonts/PingFang.ttc", "/System/Library/Fonts/STHeiti Light.ttc"],
        &["/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"],
        &["/System/Library/Fonts/AppleSDGothicNeo.ttc"],
    ];
    #[cfg(target_os = "linux")]
    let candidates: &[&[&str]] = &[
        &["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc"],
        &["/usr/share/fonts/truetype/nanum/NanumGothic.ttf"],
    ];

    let mut fonts = egui::FontDefinitions::default();
    for paths in candidates {
        if let Some(data) = paths.iter().find_map(|path| fs::read(path).ok()) {
            let index = fonts.font_data.keys().filter(|name| name.starts_with("system_cjk_")).count();
            let name = format!("system_cjk_{index}");
            fonts.font_data.insert(
                name.clone(),
                Arc::new(egui::FontData::from_owned(data)),
            );
            let proportional = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();
            proportional.insert(index.min(proportional.len()), name.clone());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push(name);
        }
    }
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("viewer_font_definitions"), fonts.clone()));
    ctx.set_fonts(fonts);
}
