use super::*;
use crate::{particle_preview::Emitter, render_system::scene::{Draw, Scene}};
use std::{sync::Mutex, time::Instant};

#[derive(Default)]
pub(super) struct Playback {
    source: Option<Arc<Prepared>>,
    emitters: Vec<Emitter>,
    last: Option<Instant>,
    controlled: HashSet<String>,
    paused: HashSet<String>,
    active_emitters: HashSet<String>,
    pub error: Arc<Mutex<Option<String>>>,
}

pub(super) struct Item {
    pub order: (i64, i64, i64),
    draw: Draw,
    masks: Vec<[[f64; 2]; 4]>,
    time: f32,
    selectable: bool,
}

impl Item {
    pub fn hit(&self, pointer: Pos2, screen: impl Fn([f64; 2]) -> Pos2) -> Option<String> {
        if !self.selectable || self.masks.iter().any(|mask| !egui::Rect::from_points(&mask.map(&screen)).contains(pointer)) { return None; }
        self.draw.vertices.chunks_exact(4).any(|quad| {
            let points = std::array::from_fn(|i| screen([quad[i].position[0] as f64, quad[i].position[1] as f64]));
            quad_coordinates(points, pointer).is_some()
        }).then(|| self.draw.component.to_string())
    }

    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, scale: f32,
                 screen: impl Fn([f64; 2]) -> Pos2, error: Arc<Mutex<Option<String>>>) {
        if self.draw.indices.is_empty() || !rect.is_positive() { return; }
        let clip = self.masks.iter().fold(rect, |clip, mask| clip.intersect(egui::Rect::from_points(&mask.map(&screen))));
        if !clip.is_positive() { return; }
        let origin = screen([0.0; 2]) - rect.center();
        let projection = [
            [2.0*scale/rect.width(), 0.0, 0.0, 2.0*origin.x/rect.width()],
            [0.0, 2.0*scale/rect.height(), 0.0, -2.0*origin.y/rect.height()],
            [0.0; 4], [0.0, 0.0, 0.0, 1.0],
        ];
        let scene = Scene { draws: Arc::from([self.draw.clone()]), revision: 1 };
        let time = self.time;
        painter.with_clip_rect(clip).add(crate::render_system::canvas_callback(rect, scene, projection, time, error));
    }
}

impl Playback {
    pub fn is_advancing(&self) -> bool {
        self.emitters.iter().any(|emitter| emitter.is_playing()
            && self.active_emitters.contains(&emitter.id) && !self.paused.contains(&emitter.id))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self, prepared: &Arc<Prepared>, documents: &[Document], catalog: Option<&HashMap<String, PathBuf>>,
        transforms: &HashMap<&str, &Document>, objects: &HashMap<&str, &Document>, by_object: &HashMap<&str, &str>,
        order: &HashMap<String, i64>, layouts: &mut HashMap<String, crate::ui_preview::Layout>, viewport: [f64; 2],
        continuous: bool, selected: Option<&str>, selected_components: &HashSet<String>,
    ) -> Vec<Item> {
        if self.source.as_ref().is_none_or(|old| !Arc::ptr_eq(old, prepared)) {
            *self = Self { source: Some(prepared.clone()), emitters: prepared.particles.clone(), ..Self::default() };
        }
        self.controlled.clear();
        let explicit_object = selected.and_then(|id| documents.iter().find(|doc| doc.file_id == id))
            .and_then(|doc| doc.game_object.as_deref()).or(selected);
        let selected_object = documents.iter().find(|doc| doc.type_name == "ParticleSystem"
            && (doc.game_object.as_deref() == explicit_object || selected_components.contains(&doc.file_id)))
            .and_then(|doc| doc.game_object.as_deref());
        let selected_system = selected_object.is_some();
        let now = Instant::now();
        let dt = self.last.replace(now).map_or(0.0, |last| (now-last).as_secs_f64().min(0.1));
        let mut items = Vec::new();
        if self.emitters.is_empty() { return items; }
        let mut components: HashMap<&str, Vec<&Document>> = HashMap::new();
        let mut renderers = HashMap::new();
        for doc in documents {
            if let Some(object) = doc.game_object.as_deref() { components.entry(object).or_default().push(doc); }
            if doc.type_name == "ParticleSystemRenderer" { renderers.insert(doc.file_id.as_str(), doc); }
        }
        let templates: HashMap<_, _> = prepared.particle_materials.iter().flat_map(|scene| scene.draws.iter())
            .map(|draw| (draw.component.as_ref(), draw)).collect();
        let mut vertices = 0;
        for (index, emitter) in self.emitters.iter_mut().enumerate() {
            let Some(renderer) = renderers.get(emitter.id.as_str()) else { continue; };
            let Some(object) = renderer.game_object.as_deref() else { continue; };
            let Some(&transform) = by_object.get(object) else { continue; };
            let mut current = Some(transform);
            let mut visited = HashSet::new();
            let mut owner = None;
            let mut masks = Vec::new();
            let mut controlled = selected_components.contains(&emitter.id) && selected_system;
            let mut maskable = true;
            while let Some(id) = current.filter(|id| visited.insert(*id)) {
                let Some(doc) = transforms.get(id) else { break; };
                if selected_system && doc.game_object.as_deref() == selected_object { controlled = true; }
                if let Some(parts) = doc.game_object.as_deref().and_then(|object| components.get(object)) {
                    for component in parts.iter().filter(|doc| number(&doc.text,"m_Enabled",1.0) != 0.0) {
                        let script = field(&component.text, "m_Script").and_then(|value| reference(value,"guid"));
                        let name = script.as_ref().and_then(|guid| catalog?.get(&guid.to_ascii_lowercase())?.file_stem()?.to_str());
                        if owner.is_none() && matches!(name, Some("UIParticle" | "UIParticleSystem" | "UIParticleOrder")) {
                            owner = doc.game_object.as_deref();
                            maskable = number(&component.text, "m_Maskable", 1.0) != 0.0;
                        }
                        if matches!(name, Some("RectMask2D" | "Mask"))
                            || field(&component.text,"m_ShowMaskGraphic").is_some()
                            || (field(&component.text,"m_Padding").is_some() && field(&component.text,"m_Softness").is_some()) {
                            if let Some(layout) = crate::ui_preview::layout(id,transforms,objects,viewport,layouts,&mut HashSet::new()) {
                                masks.push(ui_rect_points(layout));
                            }
                        }
                    }
                }
                current = doc.parent_transform.as_deref().filter(|id| *id != "0");
            }
            let Some(layout) = crate::ui_preview::layout(transform,transforms,objects,viewport,layouts,&mut HashSet::new()) else {
                self.active_emitters.remove(&emitter.id);
                self.paused.remove(&emitter.id);
                continue;
            };
            if self.active_emitters.insert(emitter.id.clone()) {
                *emitter = prepared.particles[index].clone();
            }
            if controlled { self.controlled.insert(emitter.id.clone()); }
            let Some(template) = templates.get(emitter.id.as_str()) else { continue; };
            if continuous && !self.paused.contains(&emitter.id) { emitter.advance(dt); }
            let alpha = crate::ui_preview::opacity(transform,documents,transforms) as f32;
            if alpha <= 0.0 { continue; }
            emitter.canvas_transform(layout.matrix);
            let mut draw = emitter.draw(template);
            if alpha < 1.0 { for vertex in Arc::make_mut(&mut draw.vertices) { vertex.color[3] *= alpha; } }
            vertices += draw.vertices.len();
            if vertices > 262144 { break; }
            items.push(Item {
                order: (1_000_000_000, *order.get(owner.unwrap_or(object)).unwrap_or(&0),
                    number(&renderer.text,"m_SortingOrder",0.0) as i64),
                draw, masks: if maskable {masks} else {Vec::new()},
                time: emitter.elapsed(),
                selectable: prepared.sprites.iter().any(|s| s.component == emitter.id && s.selectable),
            });
        }
        items
    }

    pub fn controls(&mut self, ui: &mut egui::Ui, rect: egui::Rect, continuous: bool) {
        if self.controlled.is_empty() { return; }
        let count: usize = self.emitters.iter().filter(|emitter| self.controlled.contains(&emitter.id)).map(Emitter::particle_count).sum();
        let playing = self.controlled.iter().any(|id| !self.paused.contains(id));
        let pos = rect.right_bottom() - egui::vec2(12.0, 12.0);
        egui::Area::new(ui.id().with("canvas_particle_controls")).fixed_pos(pos)
            .pivot(egui::Align2::RIGHT_BOTTOM).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(tr("粒子播放"));
                    ui.horizontal(|ui| {
                        if ui.add_enabled(continuous, egui::Button::new(tr(if playing {"暂停"} else {"播放"}))).clicked() {
                            for emitter in self.emitters.iter_mut().filter(|emitter| self.controlled.contains(&emitter.id)) {
                                if playing {
                                    self.paused.insert(emitter.id.clone());
                                } else {
                                    if !emitter.is_playing() { emitter.restart(); }
                                    self.paused.remove(&emitter.id);
                                }
                            }
                        }
                        if ui.button(tr("重播")).clicked() {
                            for emitter in self.emitters.iter_mut().filter(|emitter| self.controlled.contains(&emitter.id)) {
                                emitter.restart();
                                self.paused.remove(&emitter.id);
                            }
                            ui.ctx().request_repaint();
                        }
                        if ui.button(tr("停止")).clicked() {
                            for emitter in self.emitters.iter_mut().filter(|emitter| self.controlled.contains(&emitter.id)) {
                                emitter.stop();
                                self.paused.insert(emitter.id.clone());
                            }
                            ui.ctx().request_repaint();
                        }
                    });
                    let time = self.emitters.iter().filter(|emitter| self.controlled.contains(&emitter.id)).map(Emitter::elapsed).fold(0.0, f32::max);
                    ui.label(format!("{time:.2} s  ·  {} {count}", tr("粒子")));
                });
            });
    }
}
#[cfg(test)]
pub(super) fn verify_fixture(docs: &[Document], catalog: &HashMap<String, PathBuf>, prepared: Arc<Prepared>) {
    let selected = docs.iter().find(|doc| doc.type_name == "GameObject"
        && field(&doc.text,"m_Name")==Some("ui_normal_lucky_drop_effect")).unwrap();
    let order = crate::ui_preview::hierarchy_order(docs);
    let viewport = [1280.0,720.0];
    let mut playback = Playback::default();
    let frame = |playback: &mut Playback, docs: &[Document], continuous| {
        let objects = docs.iter().filter(|doc| doc.type_name == "GameObject").map(|doc| (doc.file_id.as_str(), doc)).collect();
        let transforms: HashMap<_,_> = docs.iter().filter(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform")).map(|doc| (doc.file_id.as_str(), doc)).collect();
        let by_object = transforms.values().filter_map(|doc| Some((doc.game_object.as_deref()?, doc.file_id.as_str()))).collect();
        let mut layouts = HashMap::new();
        crate::ui_preview::seed_screen_canvases(docs, viewport, &mut layouts);
        playback.frame(&prepared, docs, Some(catalog), &transforms, &objects, &by_object,
            &order, &mut layouts, viewport, continuous, Some(&selected.file_id), &HashSet::new())
    };
    assert!(frame(&mut playback, docs, true).is_empty());
    assert!(playback.controlled.is_empty());
    let raw = std::fs::read_to_string(
        "C:/Git/g001_client_new/Assets/Res/Animations/UI/LuckyDrop/double_production_drop_show.anim",
    ).unwrap();
    let clip = crate::parse_unity_yaml(&raw).into_iter().find(|doc| doc.type_name == "AnimationClip").unwrap();
    let transform = docs.iter().find(|doc| matches!(doc.type_name.as_str(), "Transform" | "RectTransform")
        && doc.game_object.as_deref() == Some(selected.file_id.as_str())).unwrap();
    let root = docs.iter().find(|doc| Some(doc.file_id.as_str()) == transform.parent_transform.as_deref())
        .and_then(|doc| doc.game_object.as_deref()).unwrap();
    let sample = |time| crate::animation_workspace::Workspace::sample_pose(docs, root, &clip.text, time).unwrap();
    let before = sample(3.0 / 30.0);
    assert!(frame(&mut playback, &before, false).is_empty());
    let active_docs = sample(4.0 / 30.0);
    assert_eq!(field(&active_docs.iter().find(|doc| doc.file_id == selected.file_id).unwrap().text, "m_IsActive"), Some("1"));
    let items = frame(&mut playback, &active_docs, false);
    assert_eq!(playback.controlled.len(), 3);
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|item| item.order.1 == order[&selected.file_id]));
    assert!(items.iter().any(|item| !item.draw.indices.is_empty()));
    playback.last = Some(Instant::now() - std::time::Duration::from_millis(100));
    let paused = frame(&mut playback, &active_docs, false);
    assert_eq!(items[0].time, paused[0].time);
    playback.last = Some(Instant::now() - std::time::Duration::from_millis(100));
    let playing = frame(&mut playback, &active_docs, true);
    assert!(playing[0].time > paused[0].time);
    assert!(frame(&mut playback, &before, false).is_empty());
    assert!(playback.active_emitters.is_empty());
    let restarted = frame(&mut playback, &active_docs, false);
    assert_eq!(restarted[0].time, items[0].time);
}