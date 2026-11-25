//! On-demand system font discovery and reusable per-face thumbnail masks.
use super::*;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use resvg::usvg::fontdb;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{Read, Write},
    sync::{Arc, mpsc},
};

const WIDTH: usize = 160;
const HEIGHT: usize = 36;
struct Entry {
    id: fontdb::ID,
    key: String,
    language_coverage: u8,
}

const LANGUAGE_SAMPLES: [&str; 7] = [
    "简体汉字",
    "繁體漢字",
    "English",
    "日本語かな",
    "한글서체",
    "ÄÖÜäöüß",
    "Ééèêçœ",
];

fn localized_family<'a>(face: &'a fontdb::FaceInfo, fallback: &'a str, language: &str) -> &'a str {
    use fontdb::Language::*;
    let preferred = match language {
        "zh-CN" => [
            Chinese_PeoplesRepublicOfChina,
            Chinese_Singapore,
            Chinese_Taiwan,
            Chinese_HongKongSAR,
            Chinese_MacaoSAR,
        ],
        "zh-TW" => [
            Chinese_Taiwan,
            Chinese_HongKongSAR,
            Chinese_MacaoSAR,
            Chinese_PeoplesRepublicOfChina,
            Chinese_Singapore,
        ],
        _ => return fallback,
    };
    preferred
        .iter()
        .find_map(|language| {
            face.families
                .iter()
                .find(|(name, locale)| locale == language && !name.is_empty())
                .map(|(name, _)| name.as_str())
        })
        .unwrap_or(fallback)
}
pub(in super::super) struct Catalog {
    pub(super) db: fontdb::Database,
    rows: BTreeMap<String, Entry>,
}
impl Catalog {
    fn display_family<'a>(&'a self, family: &'a str, language: &str) -> &'a str {
        self.rows
            .get(family)
            .and_then(|entry| self.db.face(entry.id))
            .or_else(|| self.db.faces().find(|face| face.post_script_name == family))
            .map_or(family, |face| localized_family(face, family, language))
    }

    pub(in super::super) fn contains(&self, family: &str) -> bool {
        self.db.faces().any(|face| face.post_script_name == family)
            || self.rows.contains_key(family)
            || self
                .db
                .query(&fontdb::Query {
                    families: &[fontdb::Family::Name(family)],
                    ..Default::default()
                })
                .is_some()
    }
    pub(in crate::asset_viewer::graphic_editor) fn read() -> Self {
        let db = crate::asset_viewer::svg_preview::load_system_fonts();
        let mut rows = BTreeMap::new();
        for face in db.faces() {
            let Some((family, _)) = face.families.first() else {
                continue;
            };
            if rows.contains_key(family) {
                continue;
            }
            let Some(id) = db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(family)],
                ..Default::default()
            }) else {
                continue;
            };
            let selected = db.face(id).unwrap();
            let mut hash = Sha256::new();
            hash.update(b"tile-font-sample-v1");
            hash.update(selected.post_script_name.as_bytes());
            hash.update(selected.index.to_le_bytes());
            match &selected.source {
                fontdb::Source::File(path) => {
                    hash.update(path.as_os_str().as_encoded_bytes());
                    if let Ok(m) = fs::metadata(path) {
                        hash.update(m.len().to_le_bytes());
                        if let Ok(t) = m.modified().and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .map_err(std::io::Error::other)
                        }) {
                            hash.update(t.as_nanos().to_le_bytes());
                        }
                    }
                }
                fontdb::Source::Binary(bytes) => hash.update(bytes.as_ref().as_ref()),
            }
            let language_coverage = db
                .with_face_data(id, |bytes, index| {
                    let Ok(font) = FontRef::try_from_slice_and_index(bytes, index) else {
                        return 0;
                    };
                    LANGUAGE_SAMPLES
                        .iter()
                        .enumerate()
                        .fold(0, |coverage, (locale, sample)| {
                            if sample
                                .chars()
                                .all(|character| font.glyph_id(character).0 != 0)
                            {
                                coverage | (1 << locale)
                            } else {
                                coverage
                            }
                        })
                })
                .unwrap_or(0);
            rows.insert(
                family.clone(),
                Entry {
                    id,
                    language_coverage,
                    key: format!("{:x}", hash.finalize()),
                },
            );
        }
        Self { db, rows }
    }
    fn sample(&self, entry: &Entry) -> Option<Vec<u8>> {
        let path = crate::app_cache::root().map(|p| p.join("tile_font_samples").join(&entry.key));
        if let Some(mut file) = path.as_ref().and_then(|p| fs::File::open(p).ok()) {
            let mut alpha = vec![0; WIDTH * HEIGHT];
            if file.metadata().is_ok_and(|m| m.len() == alpha.len() as u64)
                && file.read_exact(&mut alpha).is_ok()
            {
                return Some(alpha);
            }
        }
        let alpha = self.db.with_face_data(entry.id, |bytes, index| {
            let face = FontRef::try_from_slice_and_index(bytes, index).ok()?;
            let font = face.as_scaled(PxScale::from(29.));
            let mut sample = [
                "字体样式",
                "字型樣式",
                "文字もじ",
                "서체견본",
                "Sample",
                "مثال",
                "Образец",
            ]
            .into_iter()
            .find(|s| s.chars().all(|c| font.glyph_id(c).0 != 0))
            .unwrap_or("")
            .to_owned();
            if sample.is_empty() {
                sample.extend(
                    face.codepoint_ids()
                        .map(|(_, c)| c)
                        .filter(|c| !c.is_control() && !c.is_whitespace())
                        .take(5),
                );
            }
            let mut alpha = vec![0u8; WIDTH * HEIGHT];
            let mut x = 2.;
            let mut previous = None;
            for c in sample.chars() {
                let mut glyph = font.scaled_glyph(c);
                if let Some(prev) = previous {
                    x += font.kern(prev, glyph.id);
                }
                glyph.position = ab_glyph::point(x, 3. + font.ascent());
                previous = Some(glyph.id);
                x += font.h_advance(glyph.id);
                if let Some(outline) = font.outline_glyph(glyph) {
                    let bounds = outline.px_bounds();
                    outline.draw(|dx, dy, a| {
                        let px = bounds.min.x as i32 + dx as i32;
                        let py = bounds.min.y as i32 + dy as i32;
                        if (0..WIDTH as i32).contains(&px) && (0..HEIGHT as i32).contains(&py) {
                            let v = &mut alpha[py as usize * WIDTH + px as usize];
                            *v = (*v).max((a * 255.).round() as u8);
                        }
                    });
                }
            }
            alpha.iter().any(|v| *v != 0).then_some(alpha)
        })??;
        if let Some(path) = path {
            let _guard = crate::app_cache::mutation_guard();
            if fs::create_dir_all(path.parent()?).is_ok() {
                let staging = path.with_extension(format!("{}.tmp", std::process::id()));
                if fs::File::create(&staging)
                    .and_then(|mut f| f.write_all(&alpha))
                    .is_ok()
                {
                    let _ = fs::rename(&staging, &path);
                }
                if staging.exists() {
                    let _ = fs::remove_file(staging);
                }
            }
        }
        Some(alpha)
    }
}

enum Job {
    Scan,
    Sample(Arc<Catalog>, String),
}
enum Event {
    Scanned(Arc<Catalog>),
    Sample(String, Option<Vec<u8>>),
}
struct Worker {
    send: mpsc::SyncSender<(Job, egui::Context, egui::ViewportId)>,
    receive: mpsc::Receiver<Event>,
}
impl Worker {
    fn new() -> Self {
        let (send, jobs) = mpsc::sync_channel::<(Job, egui::Context, egui::ViewportId)>(20);
        let (done, receive) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok((job, ctx, viewport)) = jobs.recv() {
                let response = match job {
                    Job::Scan => Event::Scanned(Arc::new(Catalog::read())),
                    Job::Sample(fonts, name) => {
                        let entry = &fonts.rows[&name];
                        Event::Sample(entry.key.clone(), fonts.sample(entry))
                    }
                };
                if done.send(response).is_err() {
                    break;
                }
                ctx.request_repaint_of(viewport);
                ctx.request_repaint();
            }
        });
        Self { send, receive }
    }
}
#[derive(Default)]
pub(in super::super) struct Cache {
    worker: Option<Worker>,
    pub(in super::super) catalog: Option<Arc<Catalog>>,
    pub(super) generation: u64,
    scanning: bool,
    refresh_requested: bool,
    pending: HashSet<String>,
    samples: HashMap<String, Option<egui::TextureHandle>>,
}
impl Cache {
    pub(in super::super) fn tick(&mut self, ctx: &egui::Context) {
        let worker = self.worker.get_or_insert_with(Worker::new);
        for event in worker.receive.try_iter() {
            match event {
                Event::Scanned(fonts) => {
                    let keys: HashSet<_> = fonts.rows.values().map(|e| e.key.as_str()).collect();
                    self.samples.retain(|k, _| keys.contains(k.as_str()));
                    self.catalog = Some(fonts);
                    self.scanning = false;
                    self.generation = self.generation.wrapping_add(1);
                }
                Event::Sample(key, alpha) => {
                    self.pending.remove(&key);
                    let texture = alpha.map(|a| {
                        let image = egui::ColorImage {
                            size: [WIDTH, HEIGHT],
                            pixels: a.into_iter().map(Color32::from_white_alpha).collect(),
                        };
                        ctx.load_texture(format!("font-{key}"), image, egui::TextureOptions::LINEAR)
                    });
                    self.samples.insert(key, texture);
                }
            }
        }
        if (self.catalog.is_none() || self.refresh_requested)
            && !self.scanning
            && worker
                .send
                .try_send((Job::Scan, ctx.clone(), ctx.viewport_id()))
                .is_ok()
        {
            self.scanning = true;
            self.refresh_requested = false;
        }
    }
    pub(super) fn picker(&mut self, ui: &mut egui::Ui, family: &mut String) {
        let locale = crate::localization::language_index();
        let language = crate::localization::LANGUAGES[locale].0;
        let selected = self
            .catalog
            .as_deref()
            .map_or(family.as_str(), |fonts| {
                fonts.display_family(family, language)
            })
            .to_owned();
        egui::ComboBox::from_id_salt("tile_font_family")
            .width(145.)
            .height(260.)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                let Some(fonts) = self.catalog.clone() else {
                    ui.label(tr("正在读取系统字体"));
                    return;
                };
                let supports_language =
                    |entry: &Entry| entry.language_coverage & (1 << locale) != 0;
                let rows = fonts
                    .rows
                    .iter()
                    .filter(|(_, entry)| supports_language(entry))
                    .chain(
                        fonts
                            .rows
                            .iter()
                            .filter(|(_, entry)| !supports_language(entry)),
                    );
                for (name, entry) in rows {
                    let label = fonts
                        .db
                        .face(entry.id)
                        .map_or(name.as_str(), |face| localized_family(face, name, language));
                    let selected = name == family
                        || fonts
                            .db
                            .face(entry.id)
                            .is_some_and(|face| face.post_script_name == *family);
                    let response =
                        ui.add_sized([300., 25.], egui::Button::new("").selected(selected));
                    if !ui.is_rect_visible(response.rect) {
                        continue;
                    }
                    let r = response.rect;
                    let color = ui.visuals().text_color();
                    let label_rect =
                        Rect::from_min_max(r.min, r.right_bottom() - egui::vec2(114., 0.));
                    ui.painter()
                        .with_clip_rect(label_rect.intersect(ui.clip_rect()))
                        .text(
                            r.left_center() + egui::vec2(5., 0.),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(14.),
                            color,
                        );
                    if let Some(Some(texture)) = self.samples.get(&entry.key) {
                        let dest = Rect::from_center_size(
                            r.right_center() - egui::vec2(56., 0.),
                            egui::vec2(104., 23.4),
                        );
                        ui.painter().image(
                            texture.id(),
                            dest,
                            Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                            color,
                        );
                    } else if !self.samples.contains_key(&entry.key)
                        && !self.pending.contains(&entry.key)
                        && self.worker.as_ref().is_some_and(|w| {
                            w.send
                                .try_send((
                                    Job::Sample(fonts.clone(), name.clone()),
                                    ui.ctx().clone(),
                                    ui.ctx().viewport_id(),
                                ))
                                .is_ok()
                        })
                    {
                        self.pending.insert(entry.key.clone());
                    }
                    let tooltip = if label == name {
                        name.clone()
                    } else {
                        format!("{label} ({name})")
                    };
                    if response.on_hover_text(tooltip).clicked() {
                        if !selected {
                            family.clone_from(name);
                        }
                        ui.close_menu();
                    }
                }
            });
        if ui
            .add_enabled(!self.scanning, egui::Button::new(tr("刷新字体")))
            .clicked()
        {
            self.refresh_requested = true;
            self.tick(ui.ctx());
        }
    }
}
