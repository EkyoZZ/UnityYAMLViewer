use super::*;
use crate::localization::tr;
use std::time::{Duration, Instant};

pub(super) struct Action {
    path: PathBuf,
    revision: u64,
    scene: ufbx::SceneRoot,
    mapping: Vec<(u32, u32, bool)>,
}

impl Action {
    fn load(
        path: PathBuf,
        model: &ufbx::Scene,
        draft: Option<&[u8]>,
        revision: u64,
    ) -> Result<Self, String> {
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            return Err("FBX animation exceeds 64 MiB".into());
        }
        let mut options = ufbx::LoadOpts::default();
        options.ignore_geometry = true;
        options.ignore_embedded = true;
        options.temp_allocator.memory_limit = 128 * 1024 * 1024;
        options.result_allocator.memory_limit = 128 * 1024 * 1024;
        let scene = if let Some(draft) = draft {
            ufbx::load_memory(draft, options)
        } else {
            ufbx::load_file(path.to_str().ok_or("Invalid FBX path")?, options)
        }
        .map_err(|e| format!("FBX animation: {e:?}"))?;
        let stack = scene
            .anim_stacks
            .first()
            .ok_or_else(|| tr("FBX 中没有可播放的动作").to_owned())?;
        if !stack.time_begin.is_finite()
            || !stack.time_end.is_finite()
            || stack.time_end <= stack.time_begin
        {
            return Err(tr("FBX 中没有可播放的动作").into());
        }
        let axes = |s: &ufbx::Scene| {
            (
                s.settings.axes.right,
                s.settings.axes.up,
                s.settings.axes.front,
            )
        };
        if axes(model) != axes(&scene) {
            return Err(tr("动作与模型的坐标轴不匹配").into());
        }
        let mut paths = HashMap::new();
        let mut names = HashMap::new();
        for node in scene.nodes.iter().filter(|n| !n.is_root) {
            paths
                .entry(crate::animation_workspace::fbx_binding_path(node))
                .and_modify(|slot| *slot = None)
                .or_insert(Some(node.element.typed_id));
            names
                .entry(node.element.name.as_ref())
                .and_modify(|slot| *slot = None)
                .or_insert(Some(node.element.typed_id));
        }
        let mut model_names = HashMap::new();
        for node in model.nodes.iter().filter(|n| !n.is_root) {
            *model_names
                .entry(node.element.name.as_ref())
                .or_insert(0_usize) += 1;
        }
        let mapping: Vec<_> = model
            .nodes
            .iter()
            .filter(|n| !n.is_root)
            .filter_map(|node| {
                if let Some(source) = paths
                    .get(&crate::animation_workspace::fbx_binding_path(node))
                    .copied()
                    .flatten()
                {
                    return Some((node.element.typed_id, source, false));
                }
                // Some skin exports flatten bones directly below the scene root.
                // A unique name can bind these bones to the action's world pose.
                let name = node.element.name.as_ref();
                if !node.parent.as_ref().is_some_and(|parent| parent.is_root)
                    || model_names.get(name) != Some(&1)
                {
                    return None;
                }
                Some((
                    node.element.typed_id,
                    names.get(name).copied().flatten()?,
                    true,
                ))
            })
            .collect();
        let mapped: std::collections::HashSet<_> = mapping.iter().map(|&(id, _, _)| id).collect();
        let missing = model
            .meshes
            .iter()
            .flat_map(|mesh| mesh.skin_deformers.iter())
            .flat_map(|skin| skin.clusters.iter())
            .filter_map(|cluster| cluster.bone_node.as_ref())
            .find(|bone| !mapped.contains(&bone.element.typed_id));
        if let Some(bone) = missing {
            return Err(format!(
                "{} · {}",
                tr("动作骨骼与模型不匹配"),
                bone.element.name.as_ref()
            ));
        }
        if mapping.is_empty() {
            return Err(tr("动作骨骼与模型不匹配").into());
        }
        Ok(Self {
            path,
            revision,
            scene,
            mapping,
        })
    }

    fn pose(&self, model: &ufbx::Scene, time: f64) -> Result<ufbx::SceneRoot, String> {
        let stack = &self.scene.anim_stacks[0];
        let time = (stack.time_begin + time).clamp(stack.time_begin, stack.time_end);
        let ratio = self.scene.settings.unit_meters / model.settings.unit_meters;
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err("Invalid FBX unit scale".into());
        }
        let mut options = ufbx::EvaluateOpts::default();
        options.temp_allocator.memory_limit = 128 * 1024 * 1024;
        options.result_allocator.memory_limit = 128 * 1024 * 1024;
        let action_pose = if self.mapping.iter().any(|entry| entry.2) {
            Some(
                ufbx::evaluate_scene(&self.scene, &stack.anim, time, options)
                    .map_err(|e| format!("FBX animation: {e:?}"))?,
            )
        } else {
            None
        };
        let overrides: Result<Vec<_>, String> = self
            .mapping
            .iter()
            .map(|&(node_id, source, world)| {
                let mut transform = if world {
                    ufbx::matrix_to_transform(
                        &action_pose.as_ref().unwrap().nodes[source as usize].node_to_world,
                    )
                } else {
                    ufbx::evaluate_transform(&stack.anim, &self.scene.nodes[source as usize], time)
                };
                transform.translation.x *= ratio;
                transform.translation.y *= ratio;
                transform.translation.z *= ratio;
                let p = transform.translation;
                let r = transform.rotation;
                let s = transform.scale;
                if [p.x, p.y, p.z, r.x, r.y, r.z, r.w, s.x, s.y, s.z]
                    .iter()
                    .any(|value| !value.is_finite())
                {
                    return Err(tr("动作包含无效的骨骼变换").into());
                }
                Ok(ufbx::TransformOverride { node_id, transform })
            })
            .collect();
        let overrides = overrides?;
        let mut options = ufbx::AnimOpts::default();
        options.transform_overrides = overrides.as_slice().into();
        options.result_allocator.memory_limit = 128 * 1024 * 1024;
        let anim =
            ufbx::create_anim(model, options).map_err(|e| format!("FBX animation: {e:?}"))?;
        let mut options = ufbx::EvaluateOpts::default();
        options.evaluate_skinning = true;
        options.temp_allocator.memory_limit = 128 * 1024 * 1024;
        options.result_allocator.memory_limit = 128 * 1024 * 1024;
        ufbx::evaluate_scene(model, &anim, time, options).map_err(|e| format!("FBX pose: {e:?}"))
    }
}

pub(super) struct Info {
    slots: Vec<String>,
    action: Option<(f64, f64)>,
    timeline: Option<String>,
    error: Option<String>,
}

#[derive(Clone)]
pub(super) struct Request {
    path: PathBuf,
    materials: Vec<Option<PathBuf>>,
    textures: HashMap<usize, PathBuf>,
    rendering: HashMap<usize, SlotRendering>,
    action: Option<PathBuf>,
    time: f64,
    timeline: bool,
    optimized: Option<Arc<[u8]>>,
    revision: u64,
}

#[derive(Clone, Copy, Default)]
struct SlotRendering {
    opaque: bool,
    single_sided: bool,
}

pub(super) fn render(
    request: Request,
    catalog: &HashMap<String, PathBuf>,
    resources: &mut Resources,
    framing: Option<([f32; 3], f32)>,
) -> Result<PreparedModel, String> {
    if request.action.is_none() {
        return render_pose(request, catalog, resources, framing);
    }
    match render_pose(request.clone(), catalog, resources, framing) {
        Ok(model) => Ok(model),
        Err(error) => {
            let mut rest = request;
            rest.action = None;
            let mut model = render_pose(rest, catalog, resources, framing)?;
            if let Some(info) = &mut model.standalone {
                info.error = Some(error);
            }
            Ok(model)
        }
    }
}

fn render_pose(
    request: Request,
    catalog: &HashMap<String, PathBuf>,
    resources: &mut Resources,
    framing: Option<([f32; 3], f32)>,
) -> Result<PreparedModel, String> {
    let (base, scale) = resources.fbx(&request.path, false)?;
    let mut info = Info {
        slots: base
            .materials
            .iter()
            .map(|m| m.element.name.as_ref().to_owned())
            .collect(),
        action: None,
        timeline: None,
        error: None,
    };
    if info.slots.is_empty() {
        info.slots.push(String::new());
    }
    let posed = if let Some(path) = request.action {
        if resources
            .action
            .as_ref()
            .is_none_or(|action| action.path != path || action.revision != request.revision)
        {
            resources.action = Some(Action::load(
                path,
                &base,
                request.optimized.as_deref(),
                request.revision,
            )?);
        }
        let action = resources.action.as_ref().unwrap();
        let stack = &action.scene.anim_stacks[0];
        let duration = stack.time_end - stack.time_begin;
        let rate = action.scene.settings.frames_per_second;
        info.action = Some((
            duration,
            if rate.is_finite() && rate > 0.0 {
                rate
            } else {
                30.0
            },
        ));
        if request.timeline {
            info.timeline = Some(crate::animation_workspace::fbx_key_timeline(
                &action.scene,
                stack,
                "74",
                (stack.time_begin, stack.time_end),
                scale,
            ));
        }
        Some(action.pose(&base, request.time)?)
    } else {
        None
    };
    let mut model = model_from_scene(
        posed.as_deref().unwrap_or(&base),
        scale,
        &request.materials,
        catalog,
        None,
        posed.is_some(),
        false,
        resources,
        Some(&request.textures),
    )?;
    for (slot, settings) in request.rendering {
        if let Some(material) = model.materials.get_mut(slot) {
            material.opaque = settings.opaque;
            material.cull_back = settings.single_sided;
        }
    }
    // Mirroring X already flips the FBX winding to the renderer's CW front face.
    for point in model
        .triangles
        .iter_mut()
        .flat_map(|triangle| &mut triangle.points)
    {
        point[0] = -point[0];
    }
    let (center, scale) =
        framing.unwrap_or_else(|| world_bounds(model.triangles.iter().flat_map(|tri| tri.points)));
    for point in model.triangles.iter_mut().flat_map(|tri| &mut tri.points) {
        for axis in 0..3 {
            point[axis] = (point[axis] - center[axis]) / scale;
        }
    }
    let mut prepared = prepare_gpu_model(model, Vec::new(), center, scale);
    prepared.standalone = Some(info);
    Ok(prepared)
}

pub(super) struct Setup {
    pub(super) expanded: bool,
    request: Request,
    slots: Vec<String>,
    actions: Vec<PathBuf>,
    pub(super) dirty: bool,
    playing: bool,
    looping: bool,
    last: Instant,
    duration: Option<(f64, f64)>,
    timeline: Option<crate::animation_workspace::ReadOnlyTimeline>,
    timeline_open: bool,
    error: Option<String>,
    selection_revision: u64,
    requested_revision: u64,
    drop_targets: Vec<(egui::Rect, DropTarget)>,
    optimization_open: bool,
    tolerance: f64,
    draft: Option<OptimizationDraft>,
    optimizing: Option<mpsc::Receiver<Result<OptimizationDraft, String>>>,
    optimization_error: Option<String>,
    confirm_replace: bool,
    compare: Option<crate::animation_workspace::comparison::Player>,
    compare_requested: bool,
}

#[derive(Clone)]
enum DropTarget {
    Panel,
    Texture(usize),
    Actions,
    Action(PathBuf),
}

fn extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.iter().any(|x| ext.eq_ignore_ascii_case(x)))
}

const TEXTURES: &[&str] = &["tga", "png", "jpg", "jpeg", "bmp", "webp"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires the local gingerbread FBX and TGA files"]
    fn gingerbread_actions_bind_to_flattened_skin() {
        let directory =
            PathBuf::from(std::env::var_os("USERPROFILE").unwrap()).join("Downloads/姜饼人");
        let model = directory.join("christmas2026_gingerbread_skin.FBX");
        let mut setup = Setup::new(&model, Vec::new());
        assert_eq!(setup.actions.len(), 4);
        let mut resources = Resources::default();
        let catalog = HashMap::new();
        let rest = render(setup.request(), &catalog, &mut resources, None).unwrap();
        assert!(rest.model.materials[0].texture.is_some());
        for action in setup.actions.clone() {
            setup.select(Some(action));
            let first = render(
                setup.request(),
                &catalog,
                &mut resources,
                Some((rest.center, rest.scale)),
            )
            .unwrap();
            let info = first.standalone.unwrap();
            assert!(info.error.is_none(), "{:?}", info.error);
            let (duration, _) = info.action.unwrap();
            setup.request.time = duration * 0.5;
            let middle = render(
                setup.request(),
                &catalog,
                &mut resources,
                Some((rest.center, rest.scale)),
            )
            .unwrap();
            assert!(middle.standalone.unwrap().error.is_none());
            assert!(
                first
                    .model
                    .triangles
                    .iter()
                    .zip(&middle.model.triangles)
                    .any(|(a, b)| a.points != b.points)
            );
        }
    }
}

struct OptimizationDraft {
    source: Arc<[u8]>,
    meta: Option<Vec<u8>>,
    bytes: Arc<[u8]>,
    before: usize,
    after: usize,
    error: f64,
}

fn optimize_action(path: &Path, tolerance: f64) -> Result<OptimizationDraft, String> {
    use crate::animation_workspace::{fbx_preview_options, fbx_write, read_optional_fbx_meta};
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("FBX animation exceeds 64 MiB".into());
    }
    let source = std::fs::read(path).map_err(|e| e.to_string())?;
    let meta = read_optional_fbx_meta(&path.with_added_extension("meta"))?;
    let load = |bytes| {
        let mut options = fbx_preview_options();
        options.retain_dom = true;
        ufbx::load_memory(bytes, options).map_err(|e| format!("FBX animation: {e:?}"))
    };
    let original = load(&source)?;
    let stack = original
        .anim_stacks
        .first()
        .ok_or_else(|| tr("FBX 中没有可播放的动作").to_owned())?;
    let range = (stack.time_begin, stack.time_end);
    let result = fbx_write::optimize(
        &source,
        &original,
        stack,
        range,
        original.settings.unit_meters,
        tolerance,
    )?;
    let candidate = load(&result.bytes)?;
    let updated = candidate
        .anim_stacks
        .first()
        .filter(|updated| {
            updated.element.name.as_ref() == stack.element.name.as_ref()
                && (updated.time_begin, updated.time_end) == range
        })
        .ok_or("FBX validation failed: take or duration changed")?;
    fbx_write::verify(&original, &candidate, updated, result.after)?;
    Ok(OptimizationDraft {
        source: source.into(),
        meta,
        bytes: result.bytes.into(),
        before: result.before,
        after: result.after,
        error: result.error,
    })
}

impl Setup {
    pub(super) fn new(path: &Path, materials: Vec<Option<PathBuf>>) -> Self {
        let mut setup = Self {
            request: Request {
                path: path.to_owned(),
                materials,
                textures: HashMap::new(),
                rendering: HashMap::new(),
                action: None,
                time: 0.0,
                timeline: false,
                optimized: None,
                revision: 0,
            },
            expanded: false,
            slots: vec![String::new()],
            actions: Vec::new(),
            dirty: true,
            playing: false,
            looping: true,
            last: Instant::now(),
            duration: None,
            timeline: None,
            timeline_open: false,
            error: None,
            selection_revision: 0,
            requested_revision: 0,
            drop_targets: Vec::new(),
            optimization_open: false,
            tolerance: 0.001,
            draft: None,
            optimizing: None,
            optimization_error: None,
            confirm_replace: false,
            compare: None,
            compare_requested: false,
        };
        setup.detect();
        setup
    }

    fn detect(&mut self) {
        let Some(parent) = self.request.path.parent() else {
            return;
        };
        let Some(stem) = self.request.path.file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        let stem = stem.to_ascii_lowercase();
        let base = stem.strip_suffix("_skin").unwrap_or(&stem);
        let prefix = format!("{base}@");
        let mut paths = std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .take(4096)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            if extension(&path, &["fbx"]) && name.starts_with(&prefix) && self.actions.len() < 128 {
                if !self.actions.contains(&path) {
                    self.actions.push(path);
                }
            } else if extension(&path, TEXTURES)
                && (name == base || name == stem || name.strip_suffix("_t") == Some(base))
                && self.request.materials.first().is_none_or(Option::is_none)
                && !self.request.textures.contains_key(&0)
            {
                self.request.textures.insert(0, path);
                self.dirty = true;
            }
        }
    }

    fn select(&mut self, action: Option<PathBuf>) {
        self.close_comparison();
        if self.request.action != action {
            self.draft = None;
            self.optimizing = None;
            self.optimization_error = None;
            self.confirm_replace = false;
            self.request.optimized = None;
        }
        self.request.action = action;
        self.request.time = 0.0;
        self.duration = None;
        self.timeline = None;
        self.error = None;
        self.selection_revision = self.selection_revision.wrapping_add(1);
        self.playing = self.request.action.is_some();
        self.last = Instant::now();
        self.dirty = true;
    }

    fn add_action(&mut self, path: PathBuf, replace: Option<&Path>) {
        if !extension(&path, &["fbx"]) || path == self.request.path {
            return;
        }
        if let Some(old) = replace.filter(|old| *old != path) {
            self.actions.retain(|p| p != old);
        }
        if !self.actions.contains(&path) {
            if self.actions.len() >= 128 {
                return;
            }
            self.actions.push(path.clone());
        }
        self.select(Some(path));
    }

    pub(super) fn drop_file(&mut self, point: egui::Pos2, path: &Path, batch: bool) -> bool {
        if !self.expanded {
            return false;
        }
        let Some(target) = self
            .drop_targets
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(point))
            .map(|(_, target)| target.clone())
        else {
            return false;
        };
        match target {
            DropTarget::Panel | DropTarget::Texture(_) if extension(path, TEXTURES) => {
                let slot = match target {
                    DropTarget::Texture(slot) => slot,
                    _ => 0,
                };
                self.request.textures.insert(slot, path.to_owned());
                self.dirty = true;
            }
            DropTarget::Panel | DropTarget::Actions | DropTarget::Action(_)
                if extension(path, &["fbx"]) =>
            {
                let replace = match &target {
                    DropTarget::Action(old) if !batch => Some(old.as_path()),
                    _ => None,
                };
                self.add_action(path.to_owned(), replace);
            }
            _ => return false,
        }
        true
    }

    fn dialog(&self, filter: &str, extensions: &[&str]) -> rfd::FileDialog {
        rfd::FileDialog::new()
            .set_directory(self.request.path.parent().unwrap_or(Path::new(".")))
            .add_filter(filter, extensions)
    }

    pub(super) fn panel(&mut self, ui: &mut egui::Ui, bounds: egui::Rect) {
        self.drop_targets.clear();
        self.drop_targets.push((bounds, DropTarget::Panel));
        egui::ScrollArea::vertical().show(ui, |ui| {
            if ui.button(tr("识别同目录素材")).clicked() {
                self.detect();
            }
            ui.separator();
            ui.heading(tr("贴图"));
            for slot in 0..self.slots.len() {
                let path = self.request.textures.get(&slot).cloned();
                let response = ui
                    .horizontal(|ui| {
                        ui.spacing_mut().interact_size.y = 22.0;
                        ui.menu_button(format!("mat{slot}"), |ui| {
                            let settings = self.request.rendering.entry(slot).or_default();
                            ui.horizontal(|ui| {
                                self.dirty |= ui
                                    .selectable_value(&mut settings.opaque, false, tr("透明"))
                                    .changed();
                                self.dirty |= ui
                                    .selectable_value(&mut settings.opaque, true, tr("不透明"))
                                    .changed();
                            });
                            ui.horizontal(|ui| {
                                self.dirty |= ui
                                    .selectable_value(&mut settings.single_sided, false, tr("双面"))
                                    .changed();
                                self.dirty |= ui
                                    .selectable_value(&mut settings.single_sided, true, tr("单面"))
                                    .changed();
                            });
                        })
                        .response
                        .on_hover_text(&self.slots[slot]);
                        let label = path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map_or_else(|| tr("指定贴图").into(), |name| name.to_string_lossy());
                        let width = (ui.available_width()
                            - if path.is_some() { 27.0 } else { 0.0 })
                        .max(20.0);
                        let button = ui
                            .add_sized([width, 22.0], egui::Button::new(label).truncate())
                            .on_hover_text(path.as_ref().map_or_else(
                                || tr("指定贴图").to_owned(),
                                |p| p.display().to_string(),
                            ));
                        if button.clicked()
                            && let Some(path) = self.dialog(tr("贴图"), TEXTURES).pick_file()
                        {
                            self.request.textures.insert(slot, path);
                            self.dirty = true;
                        }
                        if path.is_some() && ui.button("−").on_hover_text(tr("移除")).clicked()
                        {
                            self.request.textures.remove(&slot);
                            self.dirty = true;
                        }
                    })
                    .response;
                self.drop_targets
                    .push((response.rect.intersect(bounds), DropTarget::Texture(slot)));
            }
            ui.separator();
            let actions = ui
                .horizontal_wrapped(|ui| {
                    ui.heading(tr("动作"));
                    if ui.button(tr("加载动作")).clicked()
                        && let Some(paths) = self.dialog("FBX", &["fbx"]).pick_files()
                    {
                        for path in paths {
                            self.add_action(path, None);
                        }
                    }
                    if ui
                        .add_enabled(
                            !self.actions.is_empty(),
                            egui::Button::new(tr("查看时间轴")),
                        )
                        .clicked()
                    {
                        if self.request.action.is_none() {
                            let initial = self
                                .actions
                                .iter()
                                .find(|path| {
                                    path.file_stem()
                                        .and_then(|stem| stem.to_str())
                                        .and_then(|stem| stem.rsplit_once('@'))
                                        .is_some_and(|(_, action)| {
                                            action.eq_ignore_ascii_case("idle")
                                        })
                                })
                                .or_else(|| self.actions.first())
                                .cloned();
                            self.select(initial);
                        }
                        self.timeline_open = true;
                        if self.timeline.is_none() {
                            self.dirty = true;
                        }
                        crate::native_window::wake(ui.ctx(), self.timeline_id());
                    }
                })
                .response;
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            self.drop_targets
                .push((actions.rect.intersect(bounds), DropTarget::Actions));
            if crate::animation_workspace::ReadOnlyTimeline::transport(
                self.timeline.as_ref(),
                ui,
                &mut self.request.time,
                &mut self.playing,
                &mut self.looping,
                false,
            ) {
                self.last = Instant::now();
                self.dirty = true;
            }
            for path in self.actions.clone() {
                let row = ui
                    .horizontal(|ui| {
                        let selected = self.request.action.as_ref() == Some(&path);
                        let name = path.file_stem().unwrap_or_default().to_string_lossy();
                        let response = ui
                            .add_sized(
                                [(ui.available_width() - 27.0).max(20.0), 24.0],
                                egui::Button::new(name).selected(selected).truncate(),
                            )
                            .on_hover_text(path.display().to_string());
                        if response.clicked() {
                            self.select(Some(path.clone()));
                        }
                        response.context_menu(|ui| {
                            if ui.button(tr("替换")).clicked() {
                                ui.close_menu();
                                if let Some(next) = self.dialog("FBX", &["fbx"]).pick_file() {
                                    self.add_action(next, Some(&path));
                                }
                            }
                        });
                        if ui.button("−").on_hover_text(tr("移除")).clicked() {
                            self.actions.retain(|item| item != &path);
                            if selected {
                                self.select(None);
                            }
                        }
                    })
                    .response;
                self.drop_targets
                    .push((row.rect.intersect(bounds), DropTarget::Action(path)));
            }
        });
    }

    fn timeline_id(&self) -> egui::Id {
        egui::Id::new(("standalone_fbx_timeline", &self.request.path))
    }

    fn comparison_id(&self) -> egui::Id {
        egui::Id::new(("standalone_fbx_comparison", &self.request.path))
    }

    pub(super) fn comparing(&self) -> bool {
        self.compare_requested
    }

    fn close_comparison(&mut self) {
        if self.compare_requested {
            self.compare = None;
            self.compare_requested = false;
            self.last = Instant::now();
            self.dirty = true;
        }
    }

    pub(super) fn show_comparison(
        &mut self,
        ctx: &egui::Context,
        background: &str,
        rgb: [u8; 3],
        framing: Option<([f32; 3], f32)>,
    ) {
        if !self.compare_requested {
            return;
        }
        let id = self.comparison_id();
        let Some((duration, rate)) = self.duration else {
            self.close_comparison();
            return;
        };
        let Some(draft) = &self.draft else {
            self.close_comparison();
            return;
        };
        let player = self.compare.get_or_insert_with(|| {
            let mut original = self.request.clone();
            original.optimized = Some(draft.source.clone());
            original.timeline = false;
            let mut optimized = original.clone();
            optimized.optimized = Some(draft.bytes.clone());
            let catalog = catalog_for(ctx);
            let mut original_resources = Resources::default();
            let mut optimized_resources = Resources::default();
            let mut framing = framing;
            crate::animation_workspace::comparison::Player::for_motion(
                ctx,
                duration as f32,
                rate,
                move |time| {
                    original.time = f64::from(time);
                    optimized.time = f64::from(time);
                    let left =
                        render_pose(original.clone(), &catalog, &mut original_resources, framing)?;
                    framing.get_or_insert((left.center, left.scale));
                    let right = render_pose(
                        optimized.clone(),
                        &catalog,
                        &mut optimized_resources,
                        framing,
                    )?;
                    Ok([left, right].map(|frame| ComparisonScene {
                        gpu: frame.gpu,
                        warnings: frame.model.warnings,
                        direction: 1.0,
                    }))
                },
            )
        });
        let mut open = true;
        let mut time = self.request.time as f32;
        crate::native_window::Window::new(tr("左右同步对比"))
            .id(id)
            .open(&mut open)
            .always_on_top()
            .scrollable(false)
            .default_width(960.0)
            .default_height(560.0)
            .min_width(360.0)
            .min_height(220.0)
            .show(ctx, |ui| {
                player.show(
                    ui,
                    &mut time,
                    &mut self.playing,
                    &mut self.looping,
                    background,
                    rgb,
                )
            });
        self.request.time = f64::from(time);
        if !open {
            self.close_comparison();
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
    }

    pub(super) fn show_timeline(&mut self, ctx: &egui::Context) {
        if !self.timeline_open {
            return;
        }
        let id = self.timeline_id();
        let mut open = self.timeline_open;
        crate::native_window::Window::new(tr("动画预览"))
            .id(id)
            .open(&mut open)
            .always_on_top()
            .scrollable(false)
            .solid_scroll()
            .min_height(220.0)
            .default_width(760.0)
            .show(ctx, |ui| {
                self.optimization_controls(ui);
                let Some(timeline) = &mut self.timeline else {
                    if let Some(error) = &self.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    } else if self.request.action.is_some() {
                        ui.label(tr("正在准备预览…"));
                    }
                    return;
                };
                if timeline.show(
                    ui,
                    &mut self.request.time,
                    &mut self.playing,
                    &mut self.looping,
                ) {
                    self.last = Instant::now();
                    self.dirty = true;
                    ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                }
            });
        self.timeline_open = open;
    }

    fn preview_optimization(&mut self, preview: bool) {
        self.close_comparison();
        self.request.optimized = if preview {
            self.draft.as_ref().map(|draft| draft.bytes.clone())
        } else {
            None
        };
        self.timeline = None;
        self.error = None;
        self.selection_revision = self.selection_revision.wrapping_add(1);
        self.dirty = true;
    }

    fn optimization_controls(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.request.action.clone() else {
            return;
        };
        if let Some(job) = &self.optimizing {
            match job.try_recv() {
                Ok(Ok(draft)) => {
                    self.optimizing = None;
                    self.draft = Some(draft);
                    self.preview_optimization(true);
                }
                Ok(Err(error)) => {
                    self.optimizing = None;
                    self.optimization_error = Some(error);
                }
                Err(mpsc::TryRecvError::Disconnected) => self.optimizing = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        ui.horizontal_wrapped(|ui| {
            let label = path.file_stem().unwrap_or_default().to_string_lossy();
            ui.label(label);
            if ui
                .add(
                    egui::Button::new(tr("关键帧优化"))
                        .frame(true)
                        .selected(self.optimization_open),
                )
                .clicked()
            {
                self.optimization_open = !self.optimization_open;
                if !self.optimization_open {
                    self.confirm_replace = false;
                    self.close_comparison();
                }
            }
            if !self.optimization_open {
                return;
            }
            ui.label(tr("误差阈值"));
            ui.add(
                egui::DragValue::new(&mut self.tolerance)
                    .range(0.0..=1000.0)
                    .speed(0.0001)
                    .max_decimals(6),
            );
            if ui
                .add_enabled(
                    self.optimizing.is_none(),
                    egui::Button::new(tr("生成优化草稿")),
                )
                .clicked()
            {
                let (send, receive) = mpsc::channel();
                let path = path.clone();
                let tolerance = self.tolerance;
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let _ = send.send(optimize_action(&path, tolerance));
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                });
                self.optimizing = Some(receive);
                self.optimization_error = None;
            }
            if self.optimizing.is_some() {
                ui.spinner();
            }
            if self.draft.is_none() {
                return;
            }
            let mut preview = self.request.optimized.is_some();
            if ui.checkbox(&mut preview, tr("预览优化草稿")).changed() {
                self.preview_optimization(preview);
            }
            if ui
                .add_enabled(
                    self.duration.is_some(),
                    egui::Button::new(tr("左右同步对比")),
                )
                .clicked()
            {
                self.compare_requested = true;
                crate::native_window::wake(ui.ctx(), self.comparison_id());
                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
            }
            if ui.button(tr("放弃草稿")).clicked() {
                self.draft = None;
                self.optimizing = None;
                self.confirm_replace = false;
                self.preview_optimization(false);
                return;
            }
            let draft = self.draft.as_ref().unwrap();
            let can_save = self.optimizing.is_none() && draft.after < draft.before;
            self.confirm_replace |= ui
                .add_enabled(
                    can_save,
                    crate::theme::green_outline_button(ui, tr("原位替换")),
                )
                .clicked();
            if ui
                .add_enabled(
                    can_save,
                    crate::theme::preview_navigation_button(tr("另存为")),
                )
                .clicked()
            {
                let file = format!(
                    "{}_optimized.fbx",
                    path.file_stem().unwrap_or_default().to_string_lossy()
                );
                if let Some(mut target) = rfd::FileDialog::new()
                    .add_filter("FBX", &["fbx"])
                    .set_directory(path.parent().unwrap_or(Path::new(".")))
                    .set_file_name(file)
                    .save_file()
                {
                    target.set_extension("fbx");
                    self.optimization_error =
                        crate::animation_workspace::save_new_animation(&target, &draft.bytes).err();
                }
            }
        });
        if !self.optimization_open {
            return;
        }
        if let Some(error) = &self.optimization_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let Some(draft) = &self.draft else {
            return;
        };
        ui.small(format!(
            "{} {} → {}  ·  {} {:.6}",
            tr("关键帧"),
            draft.before,
            draft.after,
            tr("最大曲线误差"),
            draft.error
        ));
        if !self.confirm_replace {
            return;
        }
        let mut confirmed = false;
        let modal = egui::Modal::new(ui.id().with("standalone_fbx_replace")).show(ui.ctx(), |ui| {
            ui.label(tr("确认原位替换"));
            ui.label(path.display().to_string());
            ui.horizontal(|ui| {
                if ui.button(tr("取消")).clicked() {
                    self.confirm_replace = false;
                }
                if ui
                    .add(crate::theme::green_outline_button(ui, tr("确定")))
                    .clicked()
                {
                    self.confirm_replace = false;
                    confirmed = true;
                }
            });
        });
        if modal.should_close() {
            self.confirm_replace = false;
        }
        if confirmed {
            match crate::animation_workspace::replace_fbx_bytes(
                &path,
                &draft.source,
                draft.meta.as_deref(),
                &draft.bytes,
            ) {
                Ok(()) => {
                    self.draft = None;
                    self.optimization_error = None;
                    self.preview_optimization(false);
                }
                Err(error) => self.optimization_error = Some(error),
            }
        }
    }

    pub(super) fn request(&mut self) -> Request {
        self.dirty = false;
        self.requested_revision = self.selection_revision;
        let mut request = self.request.clone();
        request.revision = self.selection_revision;
        request.timeline = request.action.is_some() && self.timeline.is_none();
        request
    }

    pub(super) fn loaded(&mut self, info: Info) {
        self.slots = info.slots;
        if self.duration.is_none() {
            self.last = Instant::now();
        }
        self.duration = info.action;
        self.error = info.error;
        if self.error.is_some() {
            self.playing = false;
        }
        if let Some(text) = info.timeline {
            self.timeline = Some(crate::animation_workspace::ReadOnlyTimeline::new(text));
        }
    }

    pub(super) fn failed(&mut self) {
        self.playing = false;
    }

    pub(super) fn stale_result(&self) -> bool {
        self.requested_revision != self.selection_revision
    }

    pub(super) fn tick(&mut self, ctx: &egui::Context) {
        if self.comparing() {
            return;
        }
        let Some((duration, rate)) = self.duration.filter(|_| self.playing) else {
            self.last = Instant::now();
            return;
        };
        let elapsed = self.last.elapsed().as_secs_f64();
        let step = 1.0 / rate.clamp(1.0, 60.0);
        if elapsed >= step {
            self.last = Instant::now();
            self.request.time += elapsed.min(0.25);
            if self.request.time >= duration {
                if self.looping {
                    self.request.time = self.request.time.rem_euclid(duration);
                } else {
                    self.request.time = duration;
                    self.playing = false;
                }
            }
            self.dirty = true;
        }
        if self.playing {
            ctx.request_repaint_after(Duration::from_secs_f64(
                (step - self.last.elapsed().as_secs_f64()).clamp(0.001, step),
            ));
        }
    }
}
