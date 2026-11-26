use super::*;
use std::collections::VecDeque;
pub(crate) mod comparison;
pub(crate) mod fbx_write;

pub(crate) struct Workspace {
    root: String,
    caption: String,
    assets: Vec<Asset>,
    clip: usize,
    controller: Option<usize>,
    node: Option<usize>,
    track: Option<AnimationTrackSelection>,
    keyframe: Option<SelectedKeyframe>,
    time: f32,
    playing: bool,
    looping: bool,
    filter_keys: bool,
    last: Option<std::time::Instant>,
    editing: Option<(usize, editing::Shared)>,
    message: String,
    parsed_clip: std::cell::RefCell<Option<Arc<ParsedClip>>>,
    optimization_open: bool,
    optimization_tolerance: f64,
    optimization: Option<OptimizationDraft>,
}

struct OptimizationDraft {
    clip: usize,
    text: Arc<str>,
    before: usize,
    after: usize,
    max_error: f64,
    preview: bool,
    compare_open: bool,
    player: Option<comparison::Player>,
    confirm_replace: bool,
    fbx: Option<FbxDraft>,
    comparison: Option<MotionComparison>,
    pending: Option<std::sync::mpsc::Receiver<MotionComparison>>,
}

struct FbxDraft {
    source: Vec<u8>,
    meta: String,
    result: Vec<u8>,
    timeline: Arc<ParsedClip>,
}

#[derive(Default)]
struct MotionComparison {
    samples: usize,
    nodes: usize,
    attachments: usize,
    max_node: f64,
    max_attachment: f64,
}

struct ParsedClip {
    text: Arc<str>,
    tree: AnimationPathNode,
    curves: animation_curves::CurveData,
    duration: f32,
}

impl ParsedClip {
    fn new(text: Arc<str>) -> Self {
        let tree = animation_path_tree(&text);
        let mut bindings = Vec::new();
        collect_animation_bindings(&tree, &mut bindings);
        let duration = animation_stop_time(&text, &bindings);
        let curves = animation_curves::CurveData::parse(&text);
        Self { text, tree, curves, duration }
    }

    fn sample(&self, documents: &[Document], root: &str, time: f32) -> Option<Vec<Document>> {
        let mut bindings = Vec::new();
        collect_animation_bindings(&self.tree, &mut bindings);
        Workspace::sample_bindings(documents, root, &self.text, &self.curves, &bindings, time)
    }
}

struct Asset {
    path: PathBuf,
    guid: String,
    disk: Vec<u8>,
    raw: String,
    docs: Vec<Document>,
    timeline: Option<Arc<ParsedClip>>,
}

impl Asset {
    fn read(path: PathBuf, guid: String) -> Option<Self> {
        if fs::metadata(&path).ok()?.len() > 16 * 1024 * 1024 { return None; }
        let disk = fs::read(&path).ok()?;
        let raw = std::str::from_utf8(&disk).ok()?.to_owned();
        let docs = parse_unity_yaml(&raw);
        (!docs.is_empty()).then_some(Self { path, guid, disk, raw, docs, timeline: None })
    }

    fn imported(&self) -> bool {
        self.path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("fbx"))
    }

    fn imported_clip(path: PathBuf, guid: String, id: &str) -> Self {
        let raw = format!("--- !u!74 &{id}\nAnimationClip:\n");
        let docs = parse_unity_yaml(&raw);
        Self { path, guid, disk: Vec::new(), raw, docs, timeline: None }
    }

    fn load_imported(&mut self) -> Result<(), String> {
        if !self.imported() || self.raw.contains("m_SampleRate:") { return Ok(()); }
        let id = self.clip().ok_or("Missing FBX AnimationClip fileID")?.file_id.clone();
        let (raw, timeline) = imported_clip_data(&self.path, &id)?;
        self.raw = raw;
        self.timeline = Some(Arc::new(ParsedClip::new(timeline.into())));
        self.docs = parse_unity_yaml(&self.raw);
        Ok(())
    }

    fn clip(&self) -> Option<&Document> {
        self.docs.iter().find(|doc| doc.type_name == "AnimationClip")
    }

    fn optimized_file(&self, replacement: &str) -> Option<String> {
        let clip = self.clip()?;
        let range = clip.start_offset..clip.start_offset + clip.text.len();
        if self.raw.get(range.clone())? != clip.text.as_ref() { return None; }
        let mut raw = self.raw.clone();
        raw.replace_range(range, replacement);
        let docs = parse_unity_yaml(&raw);
        docs.iter().map(|doc| (&doc.file_id, &doc.type_name))
            .eq(self.docs.iter().map(|doc| (&doc.file_id, &doc.type_name)))
            .then_some(raw)
    }

    fn label(&self) -> String {
        self.path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
    }
}

fn optimization_destination(asset: &Asset, extension: &str) -> Option<PathBuf> {
    let mut path = rfd::FileDialog::new()
        .set_directory(asset.path.parent().unwrap_or(std::path::Path::new(".")))
        .set_file_name(format!("{}_optimized.{extension}", asset.label()))
        .add_filter(extension, &[extension]).save_file()?;
    path.set_extension(extension);
    Some(path)
}

fn replace_fbx(asset: &Asset, draft: &FbxDraft) -> Result<(), String> {
    replace_fbx_bytes(&asset.path, &draft.source, Some(draft.meta.as_bytes()), &draft.result)
}

pub(crate) fn replace_fbx_bytes(path: &std::path::Path, source: &[u8], meta: Option<&[u8]>, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let meta_path = path.with_added_extension("meta");
    let verify = || -> Result<(), String> {
        let actual_meta = read_optional_fbx_meta(&meta_path)?;
        if fs::read(path).map_err(|e| e.to_string())? != source || actual_meta.as_deref() != meta {
            return Err(crate::localization::tr("保存前检测到文件发生变化，已取消保存。").into());
        }
        Ok(())
    };
    verify()?;
    let temp = path.with_added_extension(format!("{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e| e.to_string())?;
    let write = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = write.map_err(|e| e.to_string()).and_then(|_| verify())
        .and_then(|_| fs::rename(&temp, path).map_err(|e| e.to_string()));
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

pub(crate) fn read_optional_fbx_meta(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn save_new_animation(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|error| error.to_string())?;
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(error) = result {
        let _ = fs::remove_file(path);
        return Err(error.to_string());
    }
    Ok(())
}

impl Workspace {
    pub fn open(ctx: &egui::Context, root: String, caption: String, component: &Document) -> Self {
        let catalog = ctx.data(|data| data.get_temp::<Arc<HashMap<String, PathBuf>>>(egui::Id::new("asset_catalog")));
        let references = regex::Regex::new(r"fileID:\s*(-?\d+),\s*guid:\s*([[:xdigit:]]{32})").unwrap();
        let mut pending: VecDeque<_> = references.captures_iter(&component.text).map(|entry| (entry[2].to_ascii_lowercase(), entry[1].to_owned())).collect();
        let mut visited = std::collections::HashSet::new();
        let mut assets = Vec::new();
        while let Some((guid, id)) = pending.pop_front() {
            let Some(path) = catalog.as_ref().and_then(|index| index.get(&guid)) else { continue; };
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase();
            if !matches!(extension.as_str(), "anim" | "controller" | "overridecontroller" | "fbx") { continue; }
            if !visited.insert((guid.clone(), if extension == "fbx" { id.clone() } else { String::new() })) { continue; }
            if extension == "fbx" {
                if id != "0" { assets.push(Asset::imported_clip(path.clone(), guid, &id)); }
                if assets.len() >= 128 { break; }
                continue;
            }
            let Some(asset) = Asset::read(path.clone(), guid) else { continue; };
            if extension != "anim" {
                pending.extend(references.captures_iter(&asset.raw).map(|entry| (entry[2].to_ascii_lowercase(), entry[1].to_owned())));
            }
            assets.push(asset);
            if assets.len() >= 128 { break; }
        }
        let controller = assets.iter().position(|asset| asset.docs.iter().any(|doc| doc.type_name == "AnimatorController"));
        let clip = assets.iter().position(|asset| asset.clip().is_some()).unwrap_or(0);
        let mut workspace = Self {
            root, caption, assets, clip, controller, node: None, track: None, keyframe: None,
            time: 0.0, playing: false, looping: false, filter_keys: false, last: None, editing: None, message: String::new(), parsed_clip: Default::default(),
            optimization_open: false, optimization_tolerance: 0.001, optimization: None,
        };
        workspace.select_clip(clip);
        workspace
    }

    pub fn has_unsaved_edits(&self) -> bool {
        self.editing.as_ref().is_some_and(|(_, session)| {
            let session = session.lock().unwrap();
            session.has_changes() || session.has_drafts()
        })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        crate::native_window::Window::new(&self.caption).id(egui::Id::new("prefab_animation_workspace"))
            .open(&mut open).always_on_top().resizable(true).scrollable(false).default_width(760.0)
            .show(ctx, |ui| self.controls(ui));
        if !open && self.has_unsaved_edits() {
            self.message = crate::localization::tr("请先保存或放弃编辑。").into();
            crate::native_window::wake(ctx, egui::Id::new("prefab_animation_workspace"));
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            return true;
        }
        open
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        use crate::localization::tr;
        let previous = (self.clip, self.time, self.playing, self.editing.as_ref().map(|(index, session)| (*index, session.lock().unwrap().revision_for_preview())));
        let controller_view = ui.ctx().data(|data| data.get_temp::<bool>(ui.id().with("controller_view"))).unwrap_or(false);
        let mut graph = controller_view;
        ui.horizontal_wrapped(|ui| {
            if self.controller.is_some() { ui.selectable_value(&mut graph, true, "Controller"); }
            ui.selectable_value(&mut graph, false, "AnimationClip");
        });
        ui.ctx().data_mut(|data| data.insert_temp(ui.id().with("controller_view"), graph));
        if self.assets.is_empty() {
            ui.label(tr("未找到可预览的动画资源"));
            return;
        }
        let now = std::time::Instant::now();
        let dt = self.last.replace(now).map_or(0.0, |last| (now-last).as_secs_f32().min(0.1));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().interact_size.y = 23.0;
            let mut clip = self.clip;
            egui::ComboBox::from_id_salt("clip").width(136.0).truncate().selected_text(self.assets[self.clip].label()).show_ui(ui, |ui| {
                let mut choices: Vec<_> = self.assets.iter().enumerate()
                    .filter(|(_, asset)| asset.clip().is_some())
                    .map(|(index, asset)| (index, asset.label())).collect();
                choices.sort_by(|(_, left), (_, right)| crate::playlist::natural_cmp(left, right));
                for (index, label) in choices {
                    ui.selectable_value(&mut clip, index, label);
                }
            });
            if clip != self.clip { self.select_clip(clip); }
            let parsed = self.fbx_timeline().cloned().or_else(|| self.clip_text().map(|text| self.parse_clip(text)));
            let mut bindings = Vec::new();
            if let Some(clip) = &parsed { collect_animation_bindings(&clip.tree, &mut bindings); }
            let duration = parsed.as_ref().map_or(0.0, |clip| clip.duration);
            let fixed_time = parsed.as_ref().and_then(|clip| single_frame_time(&clip.text, &bindings, duration));
            let can_play = duration > 0.0 && fixed_time.is_none();
            if let Some(time) = fixed_time {
                self.time = time;
                self.playing = false;
            }
            let key_times = bindings.iter().filter(|binding| {
                graph || !self.filter_keys || self.track.as_ref().is_some_and(|track| track_matches(track, binding))
            }).flat_map(|binding| binding.keyframes.iter().map(|key| key.time))
                .filter(|time| time.is_finite() && (0.0..=duration).contains(time));
            let previous_key = key_times.clone().filter(|time| *time < self.time - 0.00001).max_by(f32::total_cmp);
            let next_key = key_times.filter(|time| *time > self.time + 0.00001).min_by(f32::total_cmp);
            if self.playing && can_play && !self.comparing() {
                self.time += dt;
                if self.time >= duration {
                    if self.looping { self.time = self.time.rem_euclid(duration); }
                    else { self.time = duration; self.playing = false; }
                }
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
            }
            clip_transport(ui, &mut self.time, &mut self.playing, duration, can_play, previous_key, next_key);
            ui.checkbox(&mut self.looping, tr("循环"));
            let time_changed = ui.scope(|ui| {
                // Reserve space for the longest time in this clip before playback starts.
                let digits = format!("{duration:.5}").len().saturating_sub(6).max(1);
                let sample = format!("{}.88888 s", "8".repeat(digits));
                ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;
                let font = egui::TextStyle::Monospace.resolve(ui.style());
                let width = ui.painter().layout_no_wrap(sample, font, ui.visuals().text_color()).size().x;
                ui.spacing_mut().interact_size.x = width + 2.0 * ui.spacing().button_padding.x;
                ui.add_enabled(can_play, egui::Slider::new(&mut self.time, 0.0..=duration)
                    .custom_formatter(|time, _| format!("{time:.5}")).suffix(" s")).changed()
            }).inner;
            if time_changed { self.playing = false; }
            if !graph {
                let button = egui::Button::new("").selected(self.filter_keys);
                let response = ui.add_sized([23.0, 23.0], button);
                if response.clicked() { self.filter_keys = !self.filter_keys; }
                let icon = crate::icons::Icon::Locate;
                let ink = if self.filter_keys { Color32::WHITE } else { ui.visuals().text_color() };
                crate::icons::paint(ui, icon, response.rect.shrink(4.0), ink);
                response.on_hover_text(tr(if self.filter_keys { "仅显示选中属性的关键帧" } else { "显示所有属性的关键帧" }));
            }
        });
        let toolbar_id = ui.id().with("clip_views");
        let mut curves = ui.ctx().data(|data| data.get_temp::<bool>(toolbar_id)).unwrap_or(false);
        let show_optimization = !graph && self.editing.is_none() && self.assets[self.clip].clip().is_some();
        if graph && !controller_view {
            self.optimization_open = false;
            self.close_comparison();
        }
        ui.horizontal_wrapped(|ui| {
            if show_optimization {
                let visuals = ui.visuals();
                let border = egui::Stroke::new(1.0_f32, visuals.widgets.inactive.fg_stroke.color.gamma_multiply(0.55));
                let button = egui::Button::new(tr("关键帧优化"))
                    .frame(true).selected(self.optimization_open).stroke(border);
                if ui.add(button).clicked() {
                    self.optimization_open = !self.optimization_open;
                    if !self.optimization_open { self.close_comparison(); }
                }
            }
            let edit_target = if graph { self.controller.unwrap_or(self.clip) } else { self.clip };
            if let Some((index, session)) = &self.editing {
                let index = *index;
                let session = session.clone();
                ui.label(self.assets[index].label());
                if ui.button(tr("放弃编辑")).clicked() { self.editing = None; self.message.clear(); }
                if ui.button(tr("保存编辑")).clicked() {
                    ui.memory_mut(|memory| { if let Some(id) = memory.focused() { memory.surrender_focus(id); } });
                    let result = editing::save(&session.lock().unwrap());
                    match result {
                        Ok(raw) => {
                            let asset = &mut self.assets[index];
                            asset.disk = fs::read(&asset.path).unwrap_or_default();
                            asset.docs = parse_unity_yaml(&raw);
                            asset.raw = raw;
                            if self.optimization.as_ref().is_some_and(|draft| draft.clip == index) {
                                self.optimization = None;
                            }
                            self.editing = None;
                            self.message.clear();
                        }
                        Err(error) => self.message = error,
                    }
                }
            } else if show_optimization && self.optimization_open {
                self.show_optimization_controls(ui);
            } else if !self.optimization_open && !self.assets[edit_target].imported()
                && ui.button(tr("编辑模式")).clicked() {
                self.close_comparison();
                let asset = &self.assets[edit_target];
                self.editing = Some((edit_target, Arc::new(std::sync::Mutex::new(
                    editing::Session::new(asset.path.clone(), asset.raw.clone(), asset.disk.clone())))));
            }
        });
        if show_optimization && self.optimization_open {
            self.show_optimization_summary(ui);
        }
        if !self.message.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &self.message); }
        let index = if graph { self.controller.unwrap_or(self.clip) } else { self.clip };
        let asset = &self.assets[index];
        let session = self.editing.as_ref().filter(|(edit, _)| *edit == index).map(|(_, session)| session.clone());
        if graph {
            editing::publish(ui.ctx(), session, 0..asset.raw.len());
            let motion = self.node.and_then(|node| asset.docs.get(node)).filter(|node| node.type_name == "AnimatorState")
                .and_then(|node| static_preview::field(&node.text, "m_Motion"))
                .and_then(|reference| Some((static_preview::reference(reference, "guid")?,
                    static_preview::reference(reference, "fileID")?)));
            let clip = motion.and_then(|(guid, id)| self.assets.iter().position(|asset| asset.guid.eq_ignore_ascii_case(&guid)
                && asset.clip().is_some_and(|clip| !asset.imported() || clip.file_id == id)));
            let preview = ui.add_enabled(clip.is_some(), egui::Button::new(tr("预览选中动画"))).clicked();
            controller_graph::show(ui, &asset.docs, &mut self.node, format!("animation-window:{}", asset.path.display()), true);
            if preview && let Some(clip) = clip { self.select_clip(clip); }
        } else if let Some(clip) = asset.clip() {
            let start = asset.raw.find(clip.text.as_ref()).unwrap_or(0);
            editing::publish(ui.ctx(), session, start..start+clip.text.len());
            let data = editing::scope_value(ui).map(Arc::<str>::from)
                .or_else(|| self.fbx_timeline().map(|timeline| timeline.text.clone()))
                .or_else(|| self.optimized_text()).unwrap_or_else(|| clip.text.clone());
            let (toolbar_line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().hline(toolbar_line.x_range(), toolbar_line.center().y, ui.visuals().widgets.noninteractive.bg_stroke);
            let parsed = self.fbx_timeline().cloned().unwrap_or_else(|| self.parse_clip(data.clone()));
            let mut bindings = Vec::new();
            collect_animation_bindings(&parsed.tree, &mut bindings);
            let fixed_time = single_frame_time(&data, &bindings, parsed.duration);
            let has_values = !curves && self.track.is_some() && self.keyframe.is_some();
            let grid_height = (ui.available_height() - if has_values { 96.0 } else { 55.0 }).max(40.0);
            let before = self.time;
            clip_grid(ui, &data, &parsed.curves, &bindings, &mut self.track, &mut self.keyframe, &mut self.time,
                self.filter_keys, grid_height, &mut curves, toolbar_line.center().y);
            if let Some(time) = fixed_time { self.time = time; }
            if self.time != before { self.playing = false; }
            if has_values && !curves {
                if let Some(track) = &self.track {
                    let matching: Vec<_> = bindings.iter().copied().filter(|binding| track_matches(track, binding)).collect();
                    show_animation_property_editor(ui, &data, &bindings, track, &matching,
                        animation_stop_time(&data, &bindings), &mut self.keyframe, false);
                }
            }
        }
        ui.ctx().data_mut(|data| data.insert_temp(toolbar_id, curves));
        editing::publish(ui.ctx(), None, 0..0);
        let current = (self.clip, self.time, self.playing, self.editing.as_ref().map(|(index, session)| (*index, session.lock().unwrap().revision_for_preview())));
        if current != previous { ui.ctx().request_repaint_of(egui::ViewportId::ROOT); }
    }

    fn select_clip(&mut self, clip: usize) {
        if self.clip != clip { self.close_comparison(); }
        self.message.clear();
        if let Some(asset) = self.assets.get_mut(clip)
            && let Err(error) = asset.load_imported() { self.message = error; }
        self.clip = clip;
        self.time = 0.0;
        self.track = None;
        self.keyframe = None;
    }

    fn optimized_text(&self) -> Option<Arc<str>> {
        self.optimization.as_ref().filter(|draft| draft.clip == self.clip && draft.preview)
            .map(|draft| draft.text.clone())
    }

    fn fbx_timeline(&self) -> Option<&Arc<ParsedClip>> {
        self.optimization.as_ref().filter(|draft| draft.clip == self.clip && draft.preview)
            .and_then(|draft| draft.fbx.as_ref().map(|fbx| &fbx.timeline))
            .or_else(|| self.assets.get(self.clip)?.timeline.as_ref())
    }

    fn comparing(&self) -> bool {
        self.optimization.as_ref().is_some_and(|draft| draft.clip == self.clip && draft.compare_open)
    }

    pub(crate) fn preview_label(&self) -> Option<&'static str> {
        let draft = self.optimization.as_ref().filter(|draft| draft.clip == self.clip)?;
        if self.editing.as_ref().is_some_and(|(index, _)| *index == self.clip) { return None; }
        Some(if draft.preview { "优化后 · 草稿" } else { "优化前" })
    }

    fn close_comparison(&mut self) {
        if let Some(draft) = &mut self.optimization {
            draft.compare_open = false;
            draft.player = None;
            draft.confirm_replace = false;
        }
    }

    pub(crate) fn show_comparison(&mut self, ui: &mut egui::Ui, documents: &[Document],
        flags: &HashMap<String, crate::hierarchy_controls::Flags>, catalog: Option<&Arc<HashMap<String, PathBuf>>>,
        background: &str, rgb: [u8; 3], front_view: bool) -> bool {
        let Some(draft) = self.optimization.as_mut().filter(|draft| draft.clip == self.clip && draft.compare_open)
            else { return false; };
        let asset = &self.assets[self.clip];
        let title = format!("{} · {}", crate::localization::tr("左右同步对比"), asset.label());
        let Some(source) = asset.clip() else { return false; };
        let player = draft.player.get_or_insert_with(|| {
            let cached = self.parsed_clip.borrow();
            let parsed = |text: &Arc<str>| cached.as_ref().filter(|clip| Arc::ptr_eq(&clip.text, text))
                .cloned().unwrap_or_else(|| Arc::new(ParsedClip::new(text.clone())));
            comparison::Player::new(ui.ctx(), documents.to_vec(), self.root.clone(), parsed(&source.text),
                parsed(&draft.text), flags.clone(), catalog.cloned().unwrap_or_default(), front_view)
        });
        crate::native_window::Window::new(title).id(comparison::window_id()).open(&mut draft.compare_open)
            .always_on_top().scrollable(false).default_width(960.0).default_height(560.0).min_width(360.0).min_height(220.0)
            .show(ui.ctx(), |ui| player.show(ui, &mut self.time, &mut self.playing, &mut self.looping, background, rgb));
        if !draft.compare_open {
            draft.player = None;
            self.last = None;
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        draft.compare_open
    }

    fn show_optimization_controls(&mut self, ui: &mut egui::Ui) {
        use crate::localization::tr;
        let clip = self.clip;
        ui.label(tr("误差阈值"));
        ui.add(egui::DragValue::new(&mut self.optimization_tolerance)
            .range(0.0..=1_000.0).speed(0.0001).max_decimals(6));
        let mut changed = false;
        if ui.button(tr("生成优化草稿")).clicked() && let Some(source) = self.assets[clip].clip() {
            let asset = &self.assets[clip];
            let result = if asset.imported() {
                Self::fbx_draft(asset, &source.file_id, self.optimization_tolerance)
            } else {
                let (text, before, after, error) = animation_curves::optimize_linear_clip(&source.text, self.optimization_tolerance);
                Ok((text, before, after, error, None))
            };
            match result {
                Ok((text, before, after, max_error, fbx)) => {
                    self.optimization = Some(OptimizationDraft { clip, text: text.into(), before, after, max_error, preview: true,
                        compare_open: false, player: None, confirm_replace: false, fbx, comparison: None, pending: None });
                    self.message.clear();
                    changed = true;
                }
                Err(error) => self.message = error,
            }
        }
        if let Some(draft) = self.optimization.as_mut().filter(|draft| draft.clip == clip) {
            changed |= ui.checkbox(&mut draft.preview, tr("预览优化草稿")).changed();
            if ui.button(tr("左右同步对比")).clicked() {
                draft.compare_open = true;
                crate::native_window::wake(ui.ctx(), comparison::window_id());
                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
            }
            if ui.button(tr("放弃草稿")).clicked() {
                self.optimization = None;
                changed = true;
            } else {
                changed |= self.show_save_optimization(ui);
            }
        }
        if changed {
            self.track = None;
            self.keyframe = None;
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
    }

    fn fbx_draft(asset: &Asset, id: &str, tolerance: f64) -> Result<(String, usize, usize, f64, Option<FbxDraft>), String> {
        if fs::metadata(&asset.path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            return Err("FBX animation exceeds preview sampling budget".into());
        }
        let source = fs::read(&asset.path).map_err(|e| e.to_string())?;
        let meta = fs::read_to_string(asset.path.with_added_extension("meta")).map_err(|e| e.to_string())?;
        let load = |bytes| {
            let mut options = fbx_preview_options();
            options.retain_dom = true;
            ufbx::load_memory(bytes, options).map_err(|e| format!("FBX import failed: {e:?}"))
        };
        let original = load(&source)?;
        let (stack, frames) = resolve_fbx_clip(&original, &meta, id)
            .map_err(|error| format!("{error} in {}", asset.path.display()))?;
        let rate = original.settings.frames_per_second;
        let range = fbx_time_range(stack, frames, rate);
        let scale = crate::model_preview::importer_scale(&meta, original.settings.unit_meters)?;
        let fresh = sample_fbx_clip(&original, stack, id, range, scale)?;
        if parse_unity_yaml(&fresh).first().map(|doc| &doc.text) != asset.clip().map(|doc| &doc.text) {
            return Err(crate::localization::tr("保存前检测到文件发生变化，已取消保存。").into());
        }
        let result = fbx_write::optimize(&source, &original, stack, range, scale, tolerance)?;
        let candidate = load(&result.bytes)?;
        let candidate_stack = find_fbx_stack(&candidate, stack.element.name.as_ref())?;
        fbx_write::verify(&original, &candidate, candidate_stack, result.after)?;
        let text = sample_fbx_clip(&candidate, candidate_stack, id, range, scale)?;
        let timeline = fbx_key_timeline(&candidate, candidate_stack, id, range, scale);
        let draft = FbxDraft { source, meta, result: result.bytes, timeline: Arc::new(ParsedClip::new(timeline.into())) };
        Ok((text, result.before, result.after, result.error, Some(draft)))
    }

    fn show_save_optimization(&mut self, ui: &mut egui::Ui) -> bool {
        use crate::localization::tr;
        let asset = &mut self.assets[self.clip];
        let draft = self.optimization.as_mut().unwrap();
        let supported = (draft.fbx.is_some() || asset.path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("anim")))
            && draft.after < draft.before;
        let replace = ui.add_enabled(supported, crate::theme::green_outline_button(ui, tr("原位替换")));
        let save_as = ui.add_enabled(supported, crate::theme::preview_navigation_button(tr("另存为")));
        draft.confirm_replace |= replace.clicked();
        let mut confirmed = false;
        if draft.confirm_replace {
            let modal = egui::Modal::new(ui.id().with("replace_optimized_animation")).show(ui.ctx(), |ui| {
                ui.label(tr("确认原位替换"));
                ui.label(asset.path.display().to_string());
                ui.horizontal(|ui| {
                    if ui.button(tr("取消")).clicked() { draft.confirm_replace = false; }
                    if ui.add(crate::theme::green_outline_button(ui, tr("确定"))).clicked() {
                        confirmed = true;
                        draft.confirm_replace = false;
                    }
                });
            });
            if modal.should_close() { draft.confirm_replace = false; }
        }
        if !confirmed && !save_as.clicked() { return false; }
        if let Some(fbx) = &draft.fbx {
            if !confirmed {
                if let Some(path) = optimization_destination(asset, "fbx") {
                    self.message = save_new_animation(&path, &fbx.result).err().unwrap_or_default();
                }
                return false;
            }
            let result = replace_fbx(asset, fbx);
            match result {
                Ok(()) => {
                    let path = asset.path.clone();
                    asset.raw = draft.text.to_string();
                    asset.docs = parse_unity_yaml(&asset.raw);
                    asset.timeline = Some(fbx.timeline.clone());
                    for (index, sibling) in self.assets.iter_mut().enumerate() {
                        if index != self.clip && sibling.path == path && let Some(doc) = sibling.clip() {
                            *sibling = Asset::imported_clip(path.clone(), sibling.guid.clone(), &doc.file_id);
                        }
                    }
                    self.optimization = None;
                    self.message.clear();
                    return true;
                }
                Err(error) => self.message = error,
            }
            return false;
        }
        let Some(raw) = asset.optimized_file(&draft.text) else {
            self.message = tr("保存验证失败：对象索引或类型发生变化。").into();
            return false;
        };
        if confirmed {
            let session = editing::Session::new(asset.path.clone(), raw, asset.disk.clone());
            match editing::save(&session) {
                Ok(raw) => {
                    let mut disk = raw.as_bytes().to_vec();
                    if asset.disk.starts_with(&[0xef, 0xbb, 0xbf]) && !disk.starts_with(&[0xef, 0xbb, 0xbf]) {
                        disk.splice(..0, [0xef, 0xbb, 0xbf]);
                    }
                    asset.disk = disk;
                    asset.docs = parse_unity_yaml(&raw);
                    asset.raw = raw;
                    self.optimization = None;
                    self.message.clear();
                    return true;
                }
                Err(error) => self.message = error,
            }
        } else {
            if let Some(path) = optimization_destination(asset, "anim") {
                match save_new_animation(&path, raw.as_bytes()) {
                    Ok(()) => self.message.clear(),
                    Err(error) => self.message = error,
                }
            }
        }
        false
    }

    fn show_optimization_summary(&self, ui: &mut egui::Ui) {
        use crate::localization::tr;
        if let Some(draft) = self.optimization.as_ref().filter(|draft| draft.clip == self.clip) {
            ui.small(format!("{} {} → {}  ·  {} {:.6}", tr("关键帧"), draft.before, draft.after,
                tr("最大曲线误差"), draft.max_error));
            if let Some(result) = &draft.comparison {
                if result.nodes == 0 {
                    ui.small(tr("没有可对照的动画节点"));
                } else {
                ui.small(format!("{}: {:.6}  ·  {}: {}  ·  {}: {}",
                    tr("节点最大位置偏差"), result.max_node, tr("对照节点"), result.nodes,
                    tr("采样时间点"), result.samples));
                if result.attachments != 0 {
                    ui.small(format!("{}: {:.6}  ·  {}: {}", tr("挂点最大位置偏差"),
                        result.max_attachment, tr("挂点"), result.attachments));
                }
                }
            } else {
                ui.small(tr("正在比较动画姿态…"));
            }
        }
    }

    pub(crate) fn compare_optimization(&mut self, ctx: &egui::Context, documents: &[Document]) {
        let Some(draft) = self.optimization.as_mut().filter(|draft| draft.clip == self.clip) else { return; };
        if draft.comparison.is_some() { return; }
        if let Some(receiver) = &draft.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    draft.comparison = Some(result);
                    draft.pending = None;
                    ctx.request_repaint();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    draft.comparison = Some(MotionComparison::default());
                    draft.pending = None;
                }
            }
            return;
        }
        let Some(source) = self.assets[self.clip].clip() else { return; };
        let original = source.text.clone();
        let optimized = draft.text.clone();
        let root = self.root.clone();
        let documents = documents.to_vec();
        let repaint = ctx.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        draft.pending = Some(receiver);
        std::thread::spawn(move || {
            let result = compare_motion(&documents, &root, &original, &optimized);
            let _ = sender.send(result);
            repaint.request_repaint_of(egui::ViewportId::ROOT);
        });
    }

    fn clip_text(&self) -> Option<Arc<str>> {
        let asset = self.assets.get(self.clip)?;
        let clip = asset.clip()?;
        if let Some((index, session)) = &self.editing && *index == self.clip {
            return parse_unity_yaml(&session.lock().unwrap().rendered()).into_iter()
                .find(|doc| doc.file_id == clip.file_id).map(|doc| doc.text);
        }
        self.optimized_text().or_else(|| Some(clip.text.clone()))
    }

    fn parse_clip(&self, text: Arc<str>) -> Arc<ParsedClip> {
        let mut cached = self.parsed_clip.borrow_mut();
        if let Some(clip) = cached.as_ref()
            && (Arc::ptr_eq(&clip.text, &text) || clip.text == text) {
            return clip.clone();
        }
        let parsed = Arc::new(ParsedClip::new(text));
        *cached = Some(parsed.clone());
        parsed
    }

    pub fn sample(&self, documents: &[Document]) -> Option<Vec<Document>> {
        self.parse_clip(self.clip_text()?).sample(documents, &self.root, self.time)
    }

    #[cfg(test)]
    pub(crate) fn sample_pose(documents: &[Document], root: &str, text: &str, time: f32) -> Option<Vec<Document>> {
        let tree = animation_path_tree(text);
        let mut bindings = Vec::new();
        collect_animation_bindings(&tree, &mut bindings);
        Self::sample_bindings(documents, root, text, &animation_curves::CurveData::parse(text), &bindings, time)
    }

    fn sample_bindings(documents: &[Document], root: &str, text: &str, curves: &animation_curves::CurveData,
        bindings: &[&AnimationBinding], time: f32) -> Option<Vec<Document>> {
        let objects: HashMap<_, _> = documents.iter().filter(|doc| doc.type_name=="GameObject").map(|doc| (doc.file_id.as_str(),doc)).collect();
        let transforms: HashMap<_, _> = documents.iter().filter(|doc| matches!(doc.type_name.as_str(),"Transform"|"RectTransform"))
            .filter_map(|doc| Some((doc.game_object.as_deref()?, doc))).collect();
        let root = transforms.get(root)?;
        let mut pose = documents.to_vec();
        let mut indices = HashMap::with_capacity(documents.len());
        for (i, doc) in documents.iter().enumerate() { indices.entry(doc.file_id.as_str()).or_insert(i); }
        // Many FBX component tracks share the same hierarchy path on each sampled frame.
        let mut child_names = HashMap::new();
        for &doc in transforms.values() {
            if let Some(parent) = doc.parent_transform.as_deref()
                && let Some(object) = doc.game_object.as_deref().and_then(|id| objects.get(id))
                && let Some(name) = name_from_text(&object.text) {
                child_names.entry(parent).or_insert_with(HashMap::new)
                    .entry(name).and_modify(|target| *target = None).or_insert(Some(doc));
            }
        }
        let target = |path: &str| {
            let mut transform = *root;
            for name in path.trim_matches(['\'', '"']).split('/').filter(|segment| !segment.is_empty()) {
                transform = (*child_names.get(transform.file_id.as_str())?.get(name)?)?;
            }
            Some(transform)
        };
        for (path, attribute, value) in animation_curves::transform_values_from(curves, text, bindings, time) {
            let Some(target) = target(&path) else { continue; };
            let id = if attribute == "m_IsActive" {
                target.game_object.as_deref()
            } else if let Some((field, _)) = attribute.split_once('.') && matches!(field, "m_Color" | "m_fontColor") {
                let mut graphics = documents.iter().filter(|doc| {
                    doc.game_object == target.game_object && doc.type_name == "MonoBehaviour"
                        && crate::ui_preview::graphic(doc).is_some()
                        && static_preview::field(&doc.text, field).is_some()
                });
                let graphic = graphics.next();
                if graphics.next().is_some() { continue; }
                graphic.map(|doc| doc.file_id.as_str())
            } else {
                Some(target.file_id.as_str())
            };
            if let Some(&index) = id.and_then(|id| indices.get(id)) {
                crate::static_preview::prefab::patch(&mut pose[index], &attribute, &value);
            }
        }
        for doc in &mut pose {
            if doc.type_name == "SkinnedMeshRenderer" && doc.game_object.as_deref() == root.game_object.as_deref()
                && static_preview::number(&doc.text, "preview_FbxStaticPose", 0.0) == 1.0 {
                doc.text = format!("{}  preview_FbxAnimatedPose: 1\n", doc.text).into();
            }
        }
        Some(pose)
    }
}
fn compare_motion(documents: &[Document], root: &str, original: &str, optimized: &str) -> MotionComparison {
    let transforms: HashMap<_, _> = documents.iter()
        .filter(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform"))
        .map(|doc| (doc.file_id.as_str(), doc)).collect();
    let Some(root) = transforms.values().find(|doc| doc.game_object.as_deref() == Some(root)) else {
        return MotionComparison::default();
    };
    let root_object = root.game_object.as_deref().unwrap_or_default();
    let targets: Vec<_> = transforms.values().filter_map(|doc| {
        let mut parent = Some(*doc);
        let mut visited = std::collections::HashSet::new();
        while let Some(node) = parent.filter(|node| visited.insert(node.file_id.as_str())) {
            if node.file_id == root.file_id {
                let attachment = doc.parent_transform.as_deref().and_then(|id| transforms.get(id))
                    .is_some_and(|p| static_preview::number(&p.text, "preview_FbxNode", 0.0) == 1.0)
                    && static_preview::number(&doc.text, "preview_FbxNode", 0.0) == 0.0;
                return Some((doc.file_id.as_str(), attachment));
            }
            parent = node.parent_transform.as_deref().and_then(|id| transforms.get(id).copied());
        }
        None
    }).collect();
    let before = animation_path_tree(original);
    let after = animation_path_tree(optimized);
    let mut original_bindings = Vec::new();
    let mut optimized_bindings = Vec::new();
    collect_animation_bindings(&before, &mut original_bindings);
    collect_animation_bindings(&after, &mut optimized_bindings);
    let original_curves = animation_curves::CurveData::parse(original);
    let optimized_curves = animation_curves::CurveData::parse(optimized);
    let duration = animation_stop_time(original, &original_bindings);
    let steps = (duration * animation_sample_rate(original).unwrap_or(30.0)).ceil().clamp(1.0, 120.0) as usize;
    let positions = |pose: &[Document]| {
        let objects: HashMap<_, _> = pose.iter().filter(|doc| doc.type_name == "GameObject")
            .map(|doc| (doc.file_id.as_str(), doc)).collect();
        let transforms: HashMap<_, _> = pose.iter()
            .filter(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform"))
            .map(|doc| (doc.file_id.as_str(), doc)).collect();
        targets.iter().map(|(id, _)| static_preview::world(id, &transforms, &objects, &mut Default::default())
            .map(|matrix| [matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]])).collect::<Vec<_>>()
    };
    let mut result = MotionComparison::default();
    let mut measured = vec![false; targets.len()];
    for step in 0..=steps {
        let time = duration * step as f32 / steps as f32;
        let Some(a) = Workspace::sample_bindings(documents, root_object, original, &original_curves, &original_bindings, time) else { continue; };
        let Some(b) = Workspace::sample_bindings(documents, root_object, optimized, &optimized_curves, &optimized_bindings, time) else { continue; };
        result.samples += 1;
        for (index, (a, b)) in positions(&a).into_iter().zip(positions(&b)).enumerate() {
            if let (Some(a), Some(b)) = (a, b) {
                measured[index] = true;
                let error = a.into_iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
                result.max_node = result.max_node.max(error);
                if targets[index].1 { result.max_attachment = result.max_attachment.max(error); }
            }
        }
    }
    result.nodes = measured.iter().filter(|&&measured| measured).count();
    result.attachments = targets.iter().zip(measured).filter(|((_, attachment), measured)| *attachment && *measured).count();
    result
}

fn imported_clip_take<'a>(meta: &'a str, id: &str) -> Option<(&'a str, Option<(f64, f64)>)> {
    let mut clips = Vec::new();
    let mut in_clips = false;
    for line in meta.lines() {
        if line == "    clipAnimations:" {
            in_clips = true;
        } else if in_clips {
            if line.starts_with("    - ") {
                clips.push(Vec::new());
            } else if line.starts_with("      ") {
                if let Some(clip) = clips.last_mut() { clip.push(line.trim()); }
            } else if !line.trim().is_empty() {
                break;
            }
        }
    }
    let imported_name = meta.lines().map(str::trim).zip(meta.lines().skip(1).map(str::trim))
        .find_map(|(line, next)| {
            if line.strip_prefix("74:").map(str::trim) != Some(id) { return None; }
            Some(next.strip_prefix("second:")?.trim().trim_matches(['\'', '"']))
        });
    let matching_clip = clips.iter().find(|clip| clip_field(clip, "internalID") == Some(id))
        .or_else(|| {
            let imported_name = imported_name?;
            let mut named = clips.iter().filter(|clip| {
                clip_field(clip, "name").map(|name| name.trim_matches(['\'', '"'])) == Some(imported_name)
            });
            let clip = named.next()?;
            named.next().is_none().then_some(clip)
        });
    if let Some(clip) = matching_clip {
        let take = clip_field(clip, "takeName")?.trim_matches(['\'', '"']);
        let frame_range = clip_field(clip, "firstFrame").and_then(|first| {
            Some((first.parse::<f64>().ok()?, clip_field(clip, "lastFrame")?.parse::<f64>().ok()?))
        });
        return Some((take, frame_range));
    }
    imported_name.map(|name| (name, None))
}

fn clip_field<'a>(fields: &[&'a str], name: &str) -> Option<&'a str> {
    fields.iter().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key == name).then(|| value.trim())
    })
}

#[cfg(test)]
pub(crate) fn imported_clip_yaml(path: &std::path::Path, id: &str) -> Result<String, String> {
    imported_clip_data(path, id).map(|(text, _)| text)
}

fn imported_clip_data(path: &std::path::Path, id: &str) -> Result<(String, String), String> {
    let meta = fs::read_to_string(format!("{}.meta", path.display())).map_err(|e| e.to_string())?;
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("FBX animation exceeds 64 MiB".into());
    }
    let scene = ufbx::load_file(path.to_str().ok_or("Invalid FBX path")?, fbx_preview_options())
        .map_err(|error| format!("FBX import failed: {error:?}"))?;
    let (stack, frame_range) = resolve_fbx_clip(&scene, &meta, id)
        .map_err(|error| format!("{error} in {}", path.display()))?;
    let range = fbx_time_range(stack, frame_range, scene.settings.frames_per_second);
    let scale = crate::model_preview::importer_scale(&meta, scene.settings.unit_meters)?;
    let sampled = sample_fbx_clip(&scene, stack, id, range, scale)?;
    Ok((sampled, fbx_key_timeline(&scene, stack, id, range, scale)))
}

pub(crate) fn fbx_preview_options() -> ufbx::LoadOpts<'static> {
    let mut options = ufbx::LoadOpts::default();
    options.ignore_embedded = true;
    options.ignore_geometry = true;
    options.temp_allocator.memory_limit = 128 * 1024 * 1024;
    options.result_allocator.memory_limit = 128 * 1024 * 1024;
    options
}

fn resolve_fbx_clip<'a>(scene: &'a ufbx::Scene, meta: &str, id: &str)
    -> Result<(&'a ufbx::AnimStack, Option<(f64, f64)>), String> {
    if let Some((take, frames)) = imported_clip_take(meta, id) {
        return Ok((find_fbx_stack(scene, take)?, frames));
    }
    // Unity imports the only take as a default clip without a serialized split.
    if scene.anim_stacks.len() == 1
        && meta.lines().any(|line| line.trim() == "clipAnimations: []")
        && meta.lines().any(|line| line.trim() == "internalIDToNameTable: []")
        && meta.lines().any(|line| line.trim() == "importAnimation: 1")
    {
        return Ok((&scene.anim_stacks[0], None));
    }
    Err(format!("FBX AnimationClip fileID {id} not found in import metadata"))
}

fn find_fbx_stack<'a>(scene: &'a ufbx::Scene, take: &str) -> Result<&'a ufbx::AnimStack, String> {
    let mut stacks = scene.anim_stacks.iter().filter(|stack| stack.element.name.as_ref() == take);
    let stack = stacks.next().ok_or_else(|| format!("FBX animation take {take} not found"))?;
    if stacks.next().is_some() { return Err(format!("Ambiguous FBX animation take {take}")); }
    Ok(stack)
}

fn fbx_time_range(stack: &ufbx::AnimStack, frame_range: Option<(f64, f64)>, rate: f64) -> (f64, f64) {
    frame_range.map_or((stack.time_begin, stack.time_end), |(first, last)| {
        (stack.time_begin + first / rate, stack.time_begin + last / rate)
    })
}

pub(crate) fn fbx_binding_path(node: &ufbx::Node) -> String {
    let mut names = vec![node.element.name.as_ref()];
    let mut parent = node.parent.as_ref();
    while let Some(node) = parent.filter(|node| !node.is_root) {
        names.push(node.element.name.as_ref());
        parent = node.parent.as_ref();
    }
    names.reverse();
    names.join("/")
}

pub(crate) fn fbx_key_timeline(scene: &ufbx::Scene, stack: &ufbx::AnimStack, id: &str, (start, end): (f64, f64), scale: f64) -> String {
    use std::fmt::Write as _;
    let rate = scene.settings.frames_per_second;
    let mut yaml = format!("--- !u!74 &{id}\nAnimationClip:\n  m_SampleRate: {rate}\n  m_StopTime: {}\n  m_FloatCurves:\n", end - start);
    let mut tracks = BTreeMap::<(String, &str, &str), Vec<(f64, f64)>>::new();
    for layer in &stack.layers {
        for prop in &layer.anim_props {
            if prop.element.type_ != ufbx::ElementType::Node { continue; }
            let (attribute, factors) = match prop.prop_name.as_ref() {
                "Lcl Translation" => ("m_LocalPosition", [-scale, scale, scale]),
                "Lcl Rotation" => ("localEulerAnglesRaw", [1.0, -1.0, -1.0]),
                "Lcl Scaling" => ("m_LocalScale", [1.0; 3]),
                _ => continue,
            };
            let Some(node) = scene.nodes.get(prop.element.typed_id as usize) else { continue; };
            if node.is_root { continue; }
            let path = fbx_binding_path(node);
            if path.contains(['\r', '\n']) { continue; }
            for ((axis, factor), curve) in ["x", "y", "z"].into_iter().zip(factors).zip(&prop.anim_value.curves) {
                let Some(curve) = curve else { continue; };
                let keys = tracks.entry((path.clone(), attribute, axis)).or_default();
                keys.extend(curve.keyframes.iter().filter(|key| key.time.is_finite() && key.value.is_finite()
                    && key.time >= start - 1e-8 && key.time <= end + 1e-8)
                    .map(|key| ((key.time - start).clamp(0.0, end - start), key.value * factor)));
            }
        }
    }
    for ((path, attribute, axis), mut keys) in tracks {
        if keys.is_empty() { continue; }
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        keys.dedup_by(|a, b| a.0 == b.0);
        yaml.push_str("  - curve:\n      m_Curve:\n");
        for (time, value) in keys {
            writeln!(yaml, "      - serializedVersion: 3\n        time: {time}\n        value: {value}").unwrap();
        }
        writeln!(yaml, "    attribute: {attribute}.{axis}\n    path: {path}\n    classID: 4").unwrap();
    }
    yaml
}

fn sample_fbx_clip(scene: &ufbx::Scene, stack: &ufbx::AnimStack, id: &str, (start, end): (f64, f64), scale: f64) -> Result<String, String> {
    use std::fmt::Write as _;
    let rate = scene.settings.frames_per_second;
    let duration = end - start;
    if !duration.is_finite() || duration < 0.0 || !rate.is_finite() || rate <= 0.0
        || duration * rate > 1200.0 || scene.nodes.len() * (duration * rate + 2.0) as usize > 25_000 {
        return Err("FBX animation exceeds preview sampling budget".into());
    }
    let mut yaml = format!("--- !u!74 &{id}\nAnimationClip:\n  m_SampleRate: {rate}\n  m_StopTime: {duration}\n  m_FloatCurves:\n");
    const ATTRIBUTES: [&str; 10] = ["m_LocalPosition.x", "m_LocalPosition.y", "m_LocalPosition.z",
        "m_LocalRotation.x", "m_LocalRotation.y", "m_LocalRotation.z", "m_LocalRotation.w",
        "m_LocalScale.x", "m_LocalScale.y", "m_LocalScale.z"];
    for node in scene.nodes.iter().filter(|node| !node.is_root) {
        let binding = fbx_binding_path(node);
        if binding.contains(['\r', '\n']) { continue; }
        let mut frames = Vec::new();
        let mut previous = None;
        for frame in 0..=(duration * rate).ceil() as usize {
            let time = (frame as f64 / rate).min(duration);
            let t = ufbx::evaluate_transform(&stack.anim, node, start + time);
            let mut q = [t.rotation.x, -t.rotation.y, -t.rotation.z, t.rotation.w];
            if previous.is_some_and(|last: [f64; 4]| last.iter().zip(q).map(|(a,b)| a*b).sum::<f64>() < 0.0) {
                q.iter_mut().for_each(|x| *x = -*x);
            }
            previous = Some(q);
            frames.push((time, [-t.translation.x * scale, t.translation.y * scale, t.translation.z * scale,
                q[0], q[1], q[2], q[3], t.scale.x, t.scale.y, t.scale.z]));
        }
        for (channel, attribute) in ATTRIBUTES.iter().enumerate() {
            if frames.windows(2).all(|pair| (pair[0].1[channel] - pair[1].1[channel]).abs() < 1e-8) { continue; }
            yaml.push_str("  - curve:\n      m_Curve:\n");
            for (time, values) in &frames {
                writeln!(yaml, "      - serializedVersion: 3\n        time: {time}\n        value: {}", values[channel]).unwrap();
            }
            writeln!(yaml, "    attribute: {attribute}\n    path: {binding}\n    classID: 4").unwrap();
        }
    }
    Ok(yaml)
}

fn single_frame_time(text: &str, bindings: &[&AnimationBinding], duration: f32) -> Option<f32> {
    let rate = animation_sample_rate(text).filter(|rate| *rate > 0.0 && rate.is_finite()).unwrap_or(60.0);
    if duration > (1.0 / rate + 0.000_01).max(0.001) { return None; }
    let mut times = bindings.iter().flat_map(|binding| &binding.keyframes).map(|key| key.time);
    let first = times.next().unwrap_or(0.0);
    times.all(|time| (time - first).abs() <= 0.000_01).then_some(first)
}

fn segment_border(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().vline(rect.right(), rect.y_range(), ui.visuals().widgets.inactive.bg_stroke);
}

fn transport_button(ui: &mut egui::Ui, icon: crate::icons::Icon, endpoint: Option<bool>, hint: &str, enabled: bool) -> bool {
    let response = ui.add_enabled_ui(enabled, |ui| ui.add_sized([23.0, 21.0], egui::Button::new("").frame(false))).inner;
    if !matches!(icon, crate::icons::Icon::Stop) { segment_border(ui, response.rect); }
    let color = if enabled { ui.visuals().text_color() } else { ui.visuals().weak_text_color().gamma_multiply(0.5) };
    if let Some(forward) = endpoint {
        let center = response.rect.center();
        let direction = if forward { 1.0 } else { -1.0 };
        for offset in [-6.0, -1.0] {
            let x = center.x + offset * direction;
            ui.painter().add(egui::Shape::convex_polygon(
                vec![egui::pos2(x, center.y - 4.0), egui::pos2(x + 5.0 * direction, center.y), egui::pos2(x, center.y + 4.0)],
                color, egui::Stroke::NONE,
            ));
        }
        ui.painter().vline(center.x + 6.0 * direction, (center.y - 4.0)..=(center.y + 4.0), egui::Stroke::new(1.5, color));
    } else {
        crate::icons::paint(ui, icon, response.rect.shrink(4.0), color);
    }
    response.on_hover_text(hint).clicked()
}

fn track_matches(track: &AnimationTrackSelection, binding: &AnimationBinding) -> bool {
    binding.path == track.path && binding.class_name == track.class_name
        && curve_group_root(&track.attribute).map_or(binding.attribute == track.attribute || is_group_component(&binding.attribute, &track.attribute),
            |group| is_group_component(&binding.attribute, group))
}

fn track_selection(binding: &AnimationBinding) -> AnimationTrackSelection {
    AnimationTrackSelection {
        path: binding.path.clone(), class_name: binding.class_name.clone(), attribute: binding.attribute.clone(),
    }
}

const CLIP_DIVIDER_WIDTH: f32 = 8.0;

fn clip_label_width(ui: &egui::Ui) -> f32 {
    let id = ui.id().with("aligned_clip_grid");
    let maximum = (ui.available_width() - 125.0).max(90.0);
    ui.ctx().data(|data| data.get_temp::<f32>(id)).unwrap_or(225.0).clamp(90.0, maximum)
}

fn clip_transport(ui: &mut egui::Ui, time: &mut f32, playing: &mut bool, duration: f32,
    enabled: bool, previous_key: Option<f32>, next_key: Option<f32>) {
    use crate::{icons::Icon, localization::tr};
    egui::Frame::NONE.stroke(ui.visuals().widgets.inactive.bg_stroke).corner_radius(3).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            let start = transport_button(ui, Icon::Previous, Some(false), tr("跳到起点"), enabled);
            let previous = transport_button(ui, Icon::Previous, None, tr("上一关键帧"), enabled && previous_key.is_some());
            let play = ui.add_enabled_ui(enabled, |ui| ui.add_sized([23.0, 21.0], egui::Button::new("").frame(false))).inner;
            if *playing { ui.painter().rect_filled(play.rect, 0.0, ui.visuals().selection.bg_fill); }
            segment_border(ui, play.rect);
            let ink = if *playing { Color32::WHITE } else { ui.visuals().text_color() };
            crate::icons::paint(ui, Icon::Play, play.rect.shrink(4.0), ink);
            if play.on_hover_text(tr(if *playing { "暂停" } else { "播放" })).clicked() {
                if *time >= duration { *time = 0.0; }
                *playing = !*playing;
            }
            let next = transport_button(ui, Icon::Next, None, tr("下一关键帧"), enabled && next_key.is_some());
            let end = transport_button(ui, Icon::Next, Some(true), tr("跳到终点"), enabled);
            let stop = transport_button(ui, Icon::Stop, None, tr("停止"), enabled);
            let seek = if start || stop { Some(0.0) } else if previous { previous_key }
                else if next { next_key } else if end { Some(duration) } else { None };
            if let Some(target) = seek {
                *playing = false;
                *time = target;
            }
        });
    });
}

const CLIP_RULER_HEIGHT: f32 = 28.0;
const CLIP_RANGE_BAR_HEIGHT: f32 = 21.0;

pub(crate) struct ReadOnlyTimeline {
    clip: ParsedClip,
    key_times: Vec<f32>,
    track: Option<AnimationTrackSelection>,
    key: Option<SelectedKeyframe>,
    curves: bool,
}

impl ReadOnlyTimeline {
    pub(crate) fn new(text: String) -> Self {
        let clip = ParsedClip::new(text.into());
        let mut bindings = Vec::new();
        collect_animation_bindings(&clip.tree, &mut bindings);
        let mut key_times: Vec<_> = bindings.iter().flat_map(|binding| &binding.keyframes).map(|key| key.time)
            .filter(|time| (0.0..=clip.duration).contains(time)).collect();
        key_times.sort_by(f32::total_cmp);
        key_times.dedup();
        Self { clip, key_times, track: None, key: None, curves: false }
    }

    pub(crate) fn transport(timeline: Option<&Self>, ui: &mut egui::Ui, time: &mut f64,
        playing: &mut bool, looping: &mut bool, show_slider: bool) -> bool {
        let duration = timeline.map_or(0.0, |timeline| timeline.clip.duration);
        let mut position = *time as f32;
        let previous = (position, *playing, *looping);
        let keys = timeline.map_or(&[][..], |timeline| timeline.key_times.as_slice());
        let before = keys.partition_point(|key| *key < position - 0.00001).checked_sub(1).map(|index| keys[index]);
        let after = keys.get(keys.partition_point(|key| *key <= position + 0.00001)).copied();
        ui.horizontal_wrapped(|ui| {
            clip_transport(ui, &mut position, playing, duration, timeline.is_some(), before, after);
            ui.add_enabled_ui(timeline.is_some(), |ui| ui.checkbox(looping, crate::localization::tr("循环")));
            if show_slider && timeline.is_some() && ui.add(egui::Slider::new(&mut position, 0.0..=duration)
                .custom_formatter(|time, _| format!("{time:.5}")).suffix(" s")).changed() {
                *playing = false;
            }
        });
        if position != previous.0 { *time = f64::from(position); }
        previous != (position, *playing, *looping)
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, time: &mut f64, playing: &mut bool, looping: &mut bool) -> bool {
        let changed = Self::transport(Some(self), ui, time, playing, looping, true);
        let mut position = (*time as f32).clamp(0.0, self.clip.duration);
        let mut bindings = Vec::new();
        collect_animation_bindings(&self.clip.tree, &mut bindings);
        let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(line.x_range(), line.center().y, ui.visuals().widgets.noninteractive.bg_stroke);
        let before_grid = position;
        // clip_grid allocates the ruler and range bar separately from the track body.
        let controls_height = CLIP_RULER_HEIGHT + CLIP_RANGE_BAR_HEIGHT + 2.0 * ui.spacing().item_spacing.y;
        let height = (ui.available_height() - controls_height).max(1.0);
        clip_grid(ui, &self.clip.text, &self.clip.curves, &bindings, &mut self.track, &mut self.key,
            &mut position, false, height, &mut self.curves, line.center().y);
        if position != before_grid {
            *playing = false;
            *time = f64::from(position);
            return true;
        }
        changed
    }
}

fn clip_grid(
    ui: &mut egui::Ui, text: &str, curve_data: &animation_curves::CurveData, bindings: &[&AnimationBinding], track: &mut Option<AnimationTrackSelection>,
    keyframe: &mut Option<SelectedKeyframe>, time: &mut f32, filter: bool, height: f32, curves: &mut bool, divider_top: f32,
) {
    const ROW_HEIGHT: f32 = 24.0;
    const RULER_HEIGHT: f32 = CLIP_RULER_HEIGHT;
    let divider = CLIP_DIVIDER_WIDTH;
    let id = ui.id().with("aligned_clip_grid");
    let width = ui.available_width();
    let max_labels = (width - 125.0).max(90.0);
    let labels = clip_label_width(ui);
    let visible = bindings.iter().copied().filter(|binding| {
        !filter || track.as_ref().is_some_and(|selected| track_matches(selected, binding))
    });
    let mut groups: Vec<(&AnimationBinding, Vec<&AnimationBinding>)> = Vec::new();
    for binding in visible {
        if let Some(root) = curve_group_root(&binding.attribute)
            && let Some((_, members)) = groups.iter_mut().find(|(first, _)| {
                first.path == binding.path && first.class_name == binding.class_name
                    && curve_group_root(&first.attribute) == Some(root)
            })
        {
            members.push(binding);
        } else {
            groups.push((binding, vec![binding]));
        }
    }
    let mut summaries = Vec::new();
    let mut lanes = Vec::new();
    for (first, members) in &groups {
        if members.len() < 2 { continue; }
        let root = curve_group_root(&first.attribute).unwrap();
        let mut keys: Vec<AnimationKeyframe> = members.iter().flat_map(|binding| binding.keyframes.iter().cloned()).collect();
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        keys.dedup_by(|next, current| {
            if next.time != current.time { return false; }
            current.time_lines.extend(next.time_lines.iter().copied());
            true
        });
        summaries.push(AnimationBinding {
            path: first.path.clone(), class_name: first.class_name.clone(),
            attribute: root.to_owned(), keyframes: keys,
        });
    }
    let mut aggregate = summaries.iter();
    for (first, members) in &groups {
        if members.len() < 2 {
            lanes.push((*first, false, false));
            continue;
        }
        let summary = aggregate.next().unwrap();
        let key = id.with((&summary.path, &summary.class_name, &summary.attribute));
        lanes.push((summary, true, false));
        if ui.ctx().data(|data| data.get_temp::<bool>(key)).unwrap_or(false) {
            lanes.extend(members.iter().map(|binding| (*binding, false, true)));
        }
    }
    let rows: Vec<_> = lanes.iter().map(|(binding, _, _)| *binding).collect();
    let visible_paths: std::collections::HashSet<_> = rows.iter().map(|binding| binding.path.as_str()).collect();
    let summary_rows: Vec<_> = lanes.iter().map(|(_, summary, _)| *summary).collect();
    let duration = animation_stop_time(text, bindings);
    let display = animation_sample_rate(text).filter(|rate| rate.is_finite() && *rate > 0.0)
        .map_or(duration, |rate| duration + 10.0 / rate);
    let right_width = (width - labels - divider - ui.spacing().scroll.allocated_width()).max(60.0);
    let zoom_id = id.with("time_zoom");
    let zoom = ui.ctx().data(|data| data.get_temp::<f32>(zoom_id)).unwrap_or(1.0);
    let offset_id = id.with("time_scroll");
    let time_scroll = ui.ctx().data(|data| data.get_temp::<f32>(offset_id)).unwrap_or(0.0)
        .clamp(0.0, ((zoom - 1.0) * right_width).max(0.0));
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, RULER_HEIGHT), egui::Sense::hover());
    let tab_rect = egui::Rect::from_min_size(header.left_top() + egui::vec2(0.0, 3.0), egui::vec2(labels, RULER_HEIGHT - 3.0));
    let mut tab_ui = ui.new_child(egui::UiBuilder::new().id_salt("clip_view_tabs").max_rect(tab_rect)
        .layout(egui::Layout::top_down(egui::Align::Center)));
    tab_ui.set_width(labels);
    tab_ui.set_clip_rect(tab_rect.intersect(ui.clip_rect()));
    let mut tab = if *curves { "curves" } else { "keys" }.to_owned();
    crate::theme::small_selector(&mut tab_ui, &mut tab, [("keys", "关键帧"), ("curves", "曲线")]);
    *curves = tab == "curves";
    let curve_width = (right_width - 20.0).max(40.0);
    let view_width = if *curves { curve_width } else { right_width };
    let vertical_zoom_id = id.with("curve_zoom");
    let vertical_scroll_id = id.with("curve_scroll");
    let vertical = ui.ctx().data(|data| data.get_temp::<f32>(vertical_zoom_id)).unwrap_or(1.0);
    let curve_zoom = curves.then_some(vertical);
    let ruler_rect = egui::Rect::from_min_size(
        header.left_top() + egui::vec2(labels + divider, 0.0), egui::vec2(view_width, RULER_HEIGHT));
    let content_rect = egui::Rect::from_min_size(
        ruler_rect.min - egui::vec2(time_scroll, 0.0), egui::vec2(view_width * zoom, RULER_HEIGHT));
    let mut ruler = ui.new_child(egui::UiBuilder::new().id_salt("fixed_clip_ruler").max_rect(content_rect)
        .layout(egui::Layout::top_down(egui::Align::Min)));
    ruler.set_clip_rect(ruler_rect.intersect(ui.clip_rect()));
    ruler.set_width(view_width * zoom);
    let matching: Vec<_> = bindings.iter().copied().filter(|binding| {
        track.as_ref().is_some_and(|selected| track_matches(selected, binding))
    }).collect();
    if let Some(at) = show_animation_frame_ruler(&mut ruler, display, animation_sample_rate(text), duration, !matching.is_empty(), Some(time)) {
        animation_curves::insert(&mut ruler, &matching, at);
    }
    ui.painter().hline(header.x_range(), header.bottom(), ui.visuals().widgets.noninteractive.bg_stroke);
    let split_x = header.left() + labels + divider * 0.5;
    let split_end = header.bottom() + ui.spacing().item_spacing.y;
    ui.painter().vline(split_x, divider_top..=split_end, ui.visuals().widgets.noninteractive.bg_stroke);
    let scrollbar_width = ui.spacing().scroll.allocated_width().max(12.0);
    let mut draw_tracks = |ui: &mut egui::Ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.allocate_ui_with_layout(egui::vec2(labels, if curve_zoom.is_some() { height } else { 0.0 }),
                egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(labels);
                ui.spacing_mut().item_spacing.y = 0.0;

                let mut show_labels = |ui: &mut egui::Ui| {
                    let labels = ui.available_width().min(labels);
                    ui.set_width(labels);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (binding, summary, component) in &lanes {
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(labels, ROW_HEIGHT), egui::Sense::click());
                        if !ui.is_rect_visible(rect) { continue; }
                        let value = (!summary).then(|| curve_data.track_value(binding, *time)).flatten();
                        let selected = track_selection(binding);
                        let active = track.as_ref() == Some(&selected);
                        let full_label = if binding.path.is_empty() {
                            format!("{} · {}", binding.class_name, binding.attribute)
                        } else {
                            format!("{} · {} · {}", binding.path, binding.class_name, binding.attribute)
                        };
                        let node = binding.path.rsplit('/').next().filter(|name| !name.is_empty())
                            .unwrap_or_else(|| localization::tr("根对象（本对象）"));
                        let attribute = binding.attribute.trim_start_matches("m_");
                        let attribute = attribute.strip_prefix("Local").unwrap_or(attribute);
                        let attribute = if let Some(axes) = attribute.strip_prefix("localEulerAnglesRaw") {
                            format!("Rotation{axes}")
                        } else { attribute.to_owned() };
                        let label = if *component { attribute } else { format!("{node} : {attribute}") };
                        if active || response.hovered() {
                            ui.painter().rect_filled(rect, 0.0, if active { ui.visuals().selection.bg_fill } else { ui.visuals().widgets.hovered.weak_bg_fill });
                        }
                        let ancestors = binding.path.match_indices('/')
                            .filter(|(index, _)| visible_paths.contains(&binding.path[..*index])).count()
                            + usize::from(!binding.path.is_empty() && visible_paths.contains(""));
                        let indent = (ancestors as f32 * 12.0 + if *component { 24.0 } else { 0.0 }).min(rect.width() * 0.4);
                        let key = id.with((&binding.path, &binding.class_name, &binding.attribute));
                        if *summary {
                            let expanded = ui.ctx().data(|data| data.get_temp::<bool>(key)).unwrap_or(false);
                            let icon = if expanded { crate::icons::Icon::TriangleDown } else { crate::icons::Icon::TriangleRight };
                            let center = rect.left_center() + egui::vec2(8.0 + indent, 0.0);
                            crate::icons::paint(ui, icon, egui::Rect::from_center_size(center, egui::Vec2::splat(10.0)),
                                if active { ui.visuals().selection.stroke.color } else { ui.visuals().text_color() });
                        }
                        let right = rect.right() - 18.0;
                        let value_width = (labels * 0.27).clamp(30.0, 64.0);
                        let value_rect = egui::Rect::from_min_max(egui::pos2(right - value_width, rect.top() + 3.0), egui::pos2(right, rect.bottom() - 3.0));
                        let text_rect = egui::Rect::from_min_max(rect.min, egui::pos2(value_rect.left() - 5.0, rect.bottom()));
                        if !summary && let Some(value) = value {
                            let painter = ui.painter().with_clip_rect(value_rect.intersect(ui.clip_rect()));
                            if animation_curves::is_boolean_track(binding) {
                                let box_rect = egui::Rect::from_center_size(value_rect.right_center() - egui::vec2(8.0, 0.0), egui::Vec2::splat(13.0));
                                let style = ui.visuals().widgets.inactive;
                                painter.rect(box_rect, 2.0, ui.visuals().extreme_bg_color, style.bg_stroke, egui::StrokeKind::Inside);
                                if value == "true" || value.parse::<f32>().is_ok_and(|value| value >= 0.5) {
                                    painter.add(egui::Shape::line(
                                        vec![box_rect.left_center() + egui::vec2(3.0, 0.0), box_rect.center_bottom() - egui::vec2(1.0, 3.0),
                                            box_rect.right_top() + egui::vec2(-2.5, 3.0)],
                                        egui::Stroke::new(1.3_f32, ui.visuals().text_color()),
                                    ));
                                }
                            } else {
                                painter.rect_filled(value_rect, 3.0, ui.visuals().extreme_bg_color);
                                painter.text(value_rect.right_center() - egui::vec2(4.0, 0.0), egui::Align2::RIGHT_CENTER,
                                    value, egui::TextStyle::Small.resolve(ui.style()), ui.visuals().text_color());
                            }
                        }
                        ui.painter().with_clip_rect(text_rect.intersect(ui.clip_rect())).text(
                            rect.left_center() + egui::vec2(16.0 + indent, 0.0), egui::Align2::LEFT_CENTER, &label,
                            egui::TextStyle::Body.resolve(ui.style()), if active { ui.visuals().selection.stroke.color } else { ui.visuals().text_color() },
                        );
                        let color = if *summary { timeline_edit::summary_color(ui) }
                            else { animation_curves::color(&binding.attribute) };
                        ui.painter().circle_filled(rect.right_center() - egui::vec2(9.0, 0.0), 2.5, color);
                        if response.on_hover_text(full_label).clicked() {
                            if *summary {
                                ui.ctx().data_mut(|data| {
                                    let expanded = data.get_temp::<bool>(key).unwrap_or(false);
                                    data.insert_temp(key, !expanded);
                                });
                            }
                            *track = if active && !summary && !filter { None } else { Some(selected) };
                            if track.is_none() { *keyframe = None; }
                        }
                        ui.painter().hline(rect.x_range(), rect.bottom(), ui.visuals().widgets.noninteractive.bg_stroke);
                    }
                    let blank_height = (height - rows.len() as f32 * ROW_HEIGHT).max(0.0);
                    if curve_zoom.is_some() && !filter && blank_height > 0.0 {
                        let (_, blank) = ui.allocate_exact_size(egui::vec2(labels, blank_height), egui::Sense::click());
                        if blank.clicked() {
                            *track = None;
                            *keyframe = None;
                        }
                    }
                };
                if curve_zoom.is_some() {
                    ui.spacing_mut().scroll.floating = false;
                    egui::ScrollArea::vertical().id_salt(id.with("curve_track_list"))
                        .max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                            let row_width = ui.available_width().min(labels - scrollbar_width);
                            ui.allocate_ui_with_layout(egui::vec2(row_width, 0.0),
                                egui::Layout::top_down(egui::Align::Min), &mut show_labels);
                        });
                } else {
                    ui.allocate_ui_with_layout(egui::vec2(labels - scrollbar_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min), &mut show_labels);
                }
            });
            let (handle, response) = ui.allocate_exact_size(
                egui::vec2(divider, if curve_zoom.is_some() { height } else { (rows.len() as f32 * ROW_HEIGHT).max(ROW_HEIGHT) }),
                egui::Sense::drag(),
            );
            let response = response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            let stroke = if response.hovered() || response.dragged() { ui.visuals().widgets.hovered.fg_stroke }
                else { ui.visuals().widgets.noninteractive.bg_stroke };
            ui.painter().vline(handle.center().x, handle.y_range(), stroke);
            if response.dragged() {
                let next = (labels + ui.input(|input| input.pointer.delta().x)).clamp(90.0, max_labels);
                ui.ctx().data_mut(|data| data.insert_temp(id, next));
            }
            if let Some(vertical) = curve_zoom {
                ui.allocate_ui_with_layout(egui::vec2(curve_width, height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(curve_width);
                    let matching: Vec<_> = bindings.iter().copied().filter(|binding| {
                        track.as_ref().map_or(!filter, |selected| {
                            binding.path == selected.path && binding.class_name == selected.class_name
                                && (binding.attribute == selected.attribute || is_group_component(&binding.attribute, &selected.attribute))
                        })
                    }).collect();
                    if matching.is_empty() {
                        ui.small(localization::tr("从左侧路径树选择一个动画属性，可查看其独立时间轴。"));
                    } else {
                        let previous = keyframe.as_ref().map(|key| key.yaml_line);
                        animation_curves::show_parsed(ui, text, curve_data, &matching, display, keyframe, Some(animation_curves::CurveViewport {
                            zoom, vertical, height, scroll_id: offset_id, vertical_scroll_id, playhead: *time,
                        }));
                        if let Some(key) = keyframe.as_ref() && previous != Some(key.yaml_line) {
                            *time = key.time.clamp(0.0, duration);
                        }
                    }
                });
                let (rail, _) = ui.allocate_exact_size(egui::vec2(20.0, height), egui::Sense::hover());
                let offset = ui.ctx().data(|data| data.get_temp::<f32>(vertical_scroll_id)).unwrap_or(0.0);
                if let Some((zoom, offset)) = timeline_range_bar(ui, rail.shrink2(egui::vec2(2.0, 0.0)),
                    id.with("curve_range"), vertical, offset, true) {
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(vertical_zoom_id, zoom);
                        data.insert_temp(vertical_scroll_id, offset);
                    });
                    ui.ctx().request_repaint();
                }
                return;
            }
            let horizontal = egui::ScrollArea::horizontal().id_salt("aligned_clip_time").max_width(right_width)
                .horizontal_scroll_offset(time_scroll).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).auto_shrink([false, true]).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(right_width * zoom);
                        timeline_edit::publish_owner(ui);
                        let display = timeline_edit::display_duration(ui, display);
                        let top = ui.cursor().top();
                        let before = keyframe.as_ref().map(|key| (key.yaml_line, key.time));
                        let (_, lane) = timeline_edit::show(ui, &rows, display, animation_sample_rate(text), false, keyframe, &summary_rows);
                        if let Some(key) = keyframe.as_ref()
                            && before != Some((key.yaml_line, key.time))
                            && let Some(binding) = rows.iter().find(|binding| binding.keyframes.iter().any(|frame| frame.yaml_line == key.yaml_line))
                        {
                            *track = Some(track_selection(binding));
                            *time = key.time.clamp(0.0, duration);
                        }
                        if lane.is_some() {
                            let rect = egui::Rect::from_min_max(egui::pos2(ui.min_rect().left(), top), ui.min_rect().max);
                            let x = animation_time_x(rect, *time as f64, display as f64);
                            ui.painter().vline(x, rect.y_range(), egui::Stroke::new(1.0_f32, ui.visuals().selection.stroke.color));
                        }
                    });
                });
            ui.ctx().data_mut(|data| data.insert_temp(offset_id, horizontal.state.offset.x));
        });
    };
    // Reserve the body independently of its scrollable contents. In
    // particular a zoomed curve must not push the range bar out of the window.
    let (body, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let mut body_ui = ui.new_child(egui::UiBuilder::new().id_salt("clip_tracks_body").max_rect(body));
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    {
        let ui = &mut body_ui;
        if curve_zoom.is_some() {
            draw_tracks(ui);
        } else {
            let scroll = egui::ScrollArea::vertical().id_salt(id).max_height(height)
                .auto_shrink([false, false]).show(ui, draw_tracks);
            let rail = egui::Rect::from_min_max(
                egui::pos2(header.left() + labels - scrollbar_width, scroll.inner_rect.top()),
                egui::pos2(header.left() + labels, scroll.inner_rect.bottom()),
            ).shrink2(egui::vec2(2.0, 0.0));
            let content_height = scroll.content_size.y.max(rail.height());
            let max_offset = (content_height - rail.height()).max(0.0);
            if max_offset > 0.0 {
                let thumb_height = (rail.height() * rail.height() / content_height.max(1.0))
                    .clamp(0.0, rail.height()).max(12.0_f32.min(rail.height()));
                let travel = rail.height() - thumb_height;
                let thumb_top = rail.top() + travel * (scroll.state.offset.y / max_offset.max(1.0)).clamp(0.0, 1.0);
                let thumb = egui::Rect::from_min_size(egui::pos2(rail.left(), thumb_top),
                    egui::vec2(rail.width(), thumb_height));
                let response = ui.interact(rail, id.with("key_list_scrollbar"), egui::Sense::click_and_drag());
                ui.painter().rect_filled(rail, 3.0, ui.visuals().extreme_bg_color);
                ui.painter().rect_filled(thumb, 3.0, if response.hovered() || response.dragged() {
                    ui.visuals().widgets.hovered.bg_fill
                } else { ui.visuals().widgets.inactive.bg_fill });
                let grab_id = id.with("key_list_scrollbar_grab");
                if travel > 0.0 && let Some(pointer) = response.interact_pointer_pos() {
                    if response.drag_started() || response.clicked() {
                        let start = ui.input(|input| input.pointer.press_origin()).unwrap_or(pointer);
                        let grab = if thumb.contains(start) { start.y - thumb.top() } else { thumb_height * 0.5 };
                        ui.ctx().data_mut(|data| data.insert_temp(grab_id, grab));
                    }
                    if response.dragged() || response.clicked() {
                        let grab = ui.ctx().data(|data| data.get_temp::<f32>(grab_id)).unwrap_or(thumb_height * 0.5);
                        let mut state = scroll.state;
                        state.offset.y = ((pointer.y - rail.top() - grab) / travel).clamp(0.0, 1.0) * max_offset;
                        state.store(ui.ctx(), scroll.id);
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
    }
    let (rail, _) = ui.allocate_exact_size(egui::vec2(width, CLIP_RANGE_BAR_HEIGHT), egui::Sense::hover());
    let clear_area = egui::Rect::from_min_size(rail.min, egui::vec2(labels, rail.height()));
    if !filter && ui.interact(clear_area, id.with("clear_track"), egui::Sense::click()).clicked() {
        *track = None;
        *keyframe = None;
        ui.ctx().request_repaint();
    }
    let rail = egui::Rect::from_min_size(rail.left_top() + egui::vec2(labels + divider, 0.0),
        egui::vec2(view_width, rail.height())).shrink2(egui::vec2(0.0, 2.0));
    let offset = ui.ctx().data(|data| data.get_temp::<f32>(offset_id)).unwrap_or(time_scroll);
    if let Some((zoom, offset)) = timeline_range_bar(ui, rail, id.with("range"), zoom, offset, false) {
        ui.ctx().data_mut(|data| {
            data.insert_temp(zoom_id, zoom);
            data.insert_temp(offset_id, offset);
        });
        ui.ctx().request_repaint();
    }
}

fn timeline_range_bar(ui: &mut egui::Ui, rail: egui::Rect, id: egui::Id, zoom: f32, offset: f32, vertical: bool) -> Option<(f32, f32)> {
    let axis = |point: egui::Pos2| if vertical { point.y } else { point.x };
    let rotate = |point: egui::Pos2| if vertical { egui::pos2(point.y, point.x) } else { point };
    let to_rect = |rect: egui::Rect| egui::Rect::from_min_max(rotate(rect.min), rotate(rect.max));
    let physical = rail;
    let rail = to_rect(rail);
    let width = rail.width().max(1.0);
    let span = (1.0 / zoom).clamp(0.05, 1.0);
    let start = (offset / (width * zoom)).clamp(0.0, 1.0 - span);
    let end = start + span;
    let thumb = egui::Rect::from_min_max(egui::pos2(rail.left() + start * width, rail.top()),
        egui::pos2(rail.left() + end * width, rail.bottom()));
    let edge = 14.0_f32.min(thumb.width() * 0.4);
    let part = |x: f32| {
        if (x - thumb.left()).abs() < edge { -1_i8 }
        else if (x - thumb.right()).abs() < edge { 1 }
        else { 0 }
    };
    let response = ui.interact(physical, id, egui::Sense::click_and_drag());
    let handle_id = id.with("handle");
    if response.drag_started() && let Some(origin) = ui.input(|input| input.pointer.press_origin()) {
        ui.ctx().data_mut(|data| data.insert_temp(handle_id, part(axis(origin))));
    }
    let handle = ui.ctx().data(|data| data.get_temp::<i8>(handle_id)).unwrap_or(0);
    let hovered_part = response.hover_pos().map_or(0, |pos| part(axis(pos)));
    let response = response.on_hover_cursor(if hovered_part != 0 || handle != 0 {
        if vertical { egui::CursorIcon::ResizeVertical } else { egui::CursorIcon::ResizeHorizontal }
    } else { egui::CursorIcon::Default });
    let dark = ui.visuals().dark_mode;
    let track_fill = if dark { Color32::from_rgb(26, 31, 43) } else { Color32::from_rgb(233, 239, 247) };
    let track_border = if dark { Color32::from_rgb(112, 132, 157) } else { Color32::from_rgb(88, 110, 140) };
    ui.painter().rect(physical, 3.0, track_fill, egui::Stroke::new(1.0_f32, track_border), egui::StrokeKind::Inside);
    let body = if dark { Color32::from_rgb(71, 130, 193) } else { Color32::from_rgb(73, 130, 192) };
    let active = ui.visuals().selection.bg_fill;
    let center_drag = response.dragged() && handle == 0;
    ui.painter().rect_filled(to_rect(thumb.shrink(1.0)), 3.0, if center_drag { active } else { body });
    for (side, left) in [(-1, thumb.left()), (1, thumb.right() - edge)] {
        let rect = egui::Rect::from_min_size(egui::pos2(left, thumb.top()), egui::vec2(edge, thumb.height())).shrink(1.0);
        let fill = if hovered_part == side || response.dragged() && handle == side { active } else { body.gamma_multiply(0.72) };
        ui.painter().rect_filled(to_rect(rect), 2.0, fill);
        for offset in [-1.5, 1.5] {
            let x = rect.center().x + offset;
            ui.painter().line_segment([rotate(egui::pos2(x, thumb.top() + 4.0)),
                rotate(egui::pos2(x, thumb.bottom() - 4.0))], egui::Stroke::new(1.0_f32, Color32::WHITE));
        }
    }
    if response.drag_stopped() { ui.ctx().data_mut(|data| data.remove::<i8>(handle_id)); }
    if response.double_clicked() { return Some((1.0, 0.0)); }
    let (start, end) = if response.dragged() {
        let delta = ui.input(|input| { let delta = input.pointer.delta(); if vertical { delta.y } else { delta.x } }) / width;
        match handle {
            -1 => ((start + delta).clamp(0.0, (end - 0.05).max(0.0)), end),
            1 => (start, (end + delta).clamp((start + 0.05).min(1.0), 1.0)),
            _ => {
                let start = (start + delta).clamp(0.0, 1.0 - span);
                (start, start + span)
            }
        }
    } else if response.clicked() && let Some(pointer) = response.interact_pointer_pos() && !to_rect(thumb).contains(pointer) {
        let start = ((axis(pointer) - rail.left()) / width - span * 0.5).clamp(0.0, 1.0 - span);
        (start, start + span)
    } else { return None; };
    let zoom = 1.0 / (end - start).clamp(0.05, 1.0);
    Some((zoom, start * width * zoom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires the local g001 project"]
    fn children_listen_binary_draft_can_reopen() {
        let path = PathBuf::from("C:/Git/g001_client_new/Assets/Res/Role/AllNewAnim/AnimChildren/children@listen.FBX");
        let id = "1827226128182048838";
        let mut asset = Asset::imported_clip(path, "c5e067bbbf80b4a44870d649c632e469".into(), id);
        asset.load_imported().unwrap();
        let (_, before, after, error, draft) = Workspace::fbx_draft(&asset, id, 0.001).unwrap();
        let draft = draft.unwrap();
        let count = |timeline: &ParsedClip| {
            let mut bindings = Vec::new();
            collect_animation_bindings(&timeline.tree, &mut bindings);
            bindings.iter().map(|binding| binding.keyframes.len()).sum::<usize>()
        };
        assert!(count(&draft.timeline) < count(asset.timeline.as_ref().unwrap()));
        assert!(after < before);
        assert!(error <= 0.001);
        assert_ne!(draft.source, draft.result);
        assert_eq!(draft.source, fs::read(&asset.path).unwrap());
    }

    #[test]
    fn switching_clips_closes_comparison_without_reopening_the_old_draft() {
        let mut docs = parse_unity_yaml("--- !u!95 &1\nAnimator:\n  m_Enabled: 1\n");
        let mut workspace = Workspace::open(&egui::Context::default(), "1".into(), "Animator".into(), &docs.remove(0));
        for name in ["first.anim", "second.anim"] {
            let raw = "--- !u!74 &74\nAnimationClip:\n  m_SampleRate: 30\n";
            workspace.assets.push(Asset { path: name.into(), guid: String::new(), disk: Vec::new(),
                raw: raw.into(), docs: parse_unity_yaml(raw), timeline: None });
        }
        let original = workspace.assets[0].clip().unwrap().text.clone();
        workspace.optimization = Some(OptimizationDraft { clip: 0, text: "draft".into(), before: 3, after: 2,
            max_error: 0.0, preview: true, compare_open: true, player: None, confirm_replace: false, fbx: None, comparison: None, pending: None });
        assert!(workspace.comparing());
        assert_eq!(workspace.preview_label(), Some("优化后 · 草稿"));
        assert_eq!(workspace.clip_text().as_deref(), Some("draft"));
        workspace.optimization.as_mut().unwrap().preview = false;
        assert_eq!(workspace.preview_label(), Some("优化前"));
        assert_eq!(workspace.clip_text().as_deref(), Some(original.as_ref()));
        workspace.select_clip(1);
        assert!(!workspace.comparing());
        assert!(workspace.preview_label().is_none());
        workspace.select_clip(0);
        assert!(!workspace.comparing());
        assert_eq!(workspace.preview_label(), Some("优化前"));
    }

    #[test]
    fn game_object_active_track_changes_only_at_keyframes() {
        let clip = "AnimationClip:\n  m_FloatCurves:\n  - curve:\n      m_Curve:\n      - serializedVersion: 3\n        time: 0\n        value: 0\n      - serializedVersion: 3\n        time: 1\n        value: 1\n    attribute: m_IsActive\n    path: ParticleRoot\n    classID: 1\n";
        let tree = animation_path_tree(clip);
        let mut bindings = Vec::new();
        collect_animation_bindings(&tree, &mut bindings);
        let sample = |time| animation_curves::transform_values_at(clip, &bindings, time);
        assert_eq!(sample(0.99), vec![("ParticleRoot".into(), "m_IsActive".into(), "0".into())]);
        assert_eq!(sample(1.0), vec![("ParticleRoot".into(), "m_IsActive".into(), "1".into())]);
    }

    #[test]
    fn prefab_pose_samples_vector_curve_without_mutating_source() {
        let raw = "--- !u!74 &7400000\nAnimationClip:\n  m_Name: Move\n  m_PositionCurves:\n  - curve:\n      m_Curve:\n      - serializedVersion: 3\n        time: 0\n        value: {x: 0, y: 5, z: 0}\n      - serializedVersion: 3\n        time: 1\n        value: {x: 20, y: 5, z: 0}\n    path: Child\n";
        let prefab = "--- !u!1 &1\nGameObject:\n  m_Name: Root\n--- !u!1 &2\nGameObject:\n  m_Name: Child\n--- !u!4 &3\nTransform:\n  m_GameObject: {fileID: 1}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n--- !u!4 &4\nTransform:\n  m_GameObject: {fileID: 2}\n  m_Father: {fileID: 3}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n";
        let documents = parse_unity_yaml(prefab);
        let workspace = Workspace {
            root: "1".into(), caption: String::new(), assets: vec![Asset {
                path: "move.anim".into(), guid: String::new(), disk: raw.as_bytes().to_vec(),
                raw: raw.into(), docs: parse_unity_yaml(raw), timeline: None,
            }],
            clip: 0, controller: None, node: None, track: None, keyframe: None, time: 0.5,
            playing: false, looping: false, filter_keys: false, last: None, editing: None, message: String::new(), parsed_clip: Default::default(),
            optimization_open: false, optimization_tolerance: 0.001, optimization: None,
        };
        let pose = workspace.sample(&documents).unwrap();
        let position = |docs: &[Document]| {
            let transform = docs.iter().find(|doc| doc.file_id == "4").unwrap();
            let raw = static_preview::field(&transform.text, "m_LocalPosition").unwrap();
            inline_vector(raw, &["x", "y", "z"]).unwrap()
        };
        assert_eq!(position(&documents), vec![0.0, 0.0, 0.0]);
        assert_eq!(position(&pose), vec![10.0, 5.0, 0.0]);
    }
}
