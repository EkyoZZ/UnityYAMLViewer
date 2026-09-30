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

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
}

#[derive(Default)]
struct ViewerApp {
    path: Option<PathBuf>,
    last_open_dir: Option<PathBuf>,
    raw: String,
    documents: Vec<Document>,
    selected: Option<usize>,
    query: String,
    status: String,
    encoding: String,
    selected_keyframe: Option<SelectedKeyframe>,
    selected_animation_track: Option<AnimationTrackSelection>,
    show_custom_table: bool,
    freeze_table_header: bool,
    tabs: Vec<ViewerTab>,
    active_tab: usize,
    show_about: bool,
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
    fn open_file(&mut self, path: PathBuf) {
        self.last_open_dir = path.parent().map(PathBuf::from);
        self.persist_last_open_dir();
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
        let tab = self.tabs[index].clone();
        self.active_tab = index;
        self.path = Some(tab.path);
        self.raw = tab.raw;
        self.documents = tab.documents;
        self.selected = tab.selected;
        self.status = tab.status;
        self.encoding = tab.encoding;
    }

    fn close_tab(&mut self, index: usize) {
        self.save_active_tab();
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.path = None;
            self.raw.clear();
            self.documents.clear();
            self.selected = None;
            self.status.clear();
            self.encoding.clear();
            self.active_tab = 0;
        } else {
            let next = self.active_tab.min(self.tabs.len() - 1);
            self.activate_tab(next);
        }
    }

    fn display_name(&self, doc: &Document) -> String {
        if doc.type_name == "GameObject" {
            name_from_text(&doc.text).unwrap_or_else(|| format!("GameObject #{}", doc.file_id))
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
        let response = egui::CollapsingHeader::new(RichText::new(title).strong())
            .id_salt(format!("go-{id}"))
            .default_open(true)
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
    }

    fn show_document(&mut self, ui: &mut egui::Ui, index: usize, is_problem: bool) {
        if !self.matches(&self.documents[index]) {
            return;
        }
        let is_match = !self.query.trim().is_empty();
        let label = if is_problem {
            RichText::new(format!("⚠ {}", self.display_name(&self.documents[index])))
                .color(warning_color(ui))
        } else {
            RichText::new(self.display_name(&self.documents[index]))
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
            ui.selectable_label(self.selected == Some(index), RichText::new(title).strong());
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
            .selectable_label(self.selected == Some(index), RichText::new(title).strong())
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
                RichText::new(title).strong(),
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
            egui::CollapsingHeader::new(machine_name)
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
                                .selectable_label(self.selected == Some(state_index), state_name)
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
                if ui.button("关于").clicked() {
                    self.show_about = true;
                }
                ui.menu_button("主题", |ui| {
                    if ui.button("跟随系统").clicked() {
                        ctx.set_theme(egui::ThemePreference::System);
                        ui.close_menu();
                    }
                    if ui.button("浅色").clicked() {
                        ctx.set_theme(egui::Theme::Light);
                        apply_light_background(ctx);
                        ui.close_menu();
                    }
                    if ui.button("深色").clicked() {
                        ctx.set_theme(egui::Theme::Dark);
                        ui.close_menu();
                    }
                });
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
                    self.selected = if query.is_empty() {
                        None
                    } else {
                        self.documents.iter().position(|document| {
                            self.display_name(document).to_lowercase().contains(&query)
                                || document.text.to_lowercase().contains(&query)
                        })
                    };
                }
                if ui.button("×").on_hover_text("清除查找").clicked() {
                    self.query.clear();
                    self.selected = None;
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
                        let (open_clicked, close_clicked) = unified_tab(ui, title, is_active);
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
			ui.heading("只读 Inspector");
			if let Some(index) = self.selected {
				let doc = &self.documents[index];
				let is_animation_clip = doc.type_name == "AnimationClip";
				let custom_table = yaml_uniform_table(&doc.text);
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
					if ui.button("复制 FileID").clicked() {
						ctx.copy_text(doc.file_id.clone());
					}
					if is_animation_clip {
						ui.add_space(8.0);
						let duration = animation_duration(&doc.text);
						let sample_rate = animation_sample_rate(&doc.text)
							.map(|rate| format!("　·　{rate:.0} FPS"))
							.unwrap_or_default();
						ui.small(format!("总时长：0.00s — {duration:.2}s{sample_rate}"));
					}
					if custom_table.is_some() {
						if ui.selectable_label(self.show_custom_table, "显示表格").clicked() {
							self.show_custom_table = true;
						}
						if ui.selectable_label(!self.show_custom_table, "显示 YAML").clicked() {
							self.show_custom_table = false;
						}
						ui.checkbox(&mut self.freeze_table_header, "冻结表头");
					}
				});
				ui.separator();
				if is_animation_clip {
					show_animation_timeline(ui, &doc.text, &self.selected_animation_track, &mut self.selected_keyframe);
					ui.separator();
					ui.small("原始 YAML（点击关键帧后自动定位并高亮对应的 time / value）");
					show_animation_yaml(ui, &doc.text, self.selected_keyframe.as_ref(), &self.query);
				} else if let Some(table) = custom_table.filter(|_| self.show_custom_table) {
					show_yaml_table(ui, &table, self.freeze_table_header);
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

        if self.show_about {
            egui::Window::new("关于 Unity YAML Viewer")
                .open(&mut self.show_about)
                .resizable(true)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.heading("Unity YAML Viewer");
                    ui.label(format!("版本 {APP_VERSION}"));
                    ui.small("用于安全查看 Unity 文本序列化资源的只读工具");
                    ui.separator();
                    ui.heading("更新历史");
                    ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        ui.label(RichText::new("1.0.0").strong());
                        ui.label("首个正式版本。");
                        ui.add_space(4.0);
                        ui.label("• 支持 Unity YAML 资源的层级与 Inspector 浏览");
                        ui.label("• 支持搜索、多标签页、GUID / FileID 复制和编码识别");
                        ui.label("• 提供动画、材质、动画控制器等常见资源的结构化展示");
                        ui.label("• 所有资源以只读方式打开，不会写回或修改原文件");
                    });
                });
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
            "221" => "AnimatorOverrideController",
            "1101" => "AnimatorStateTransition",
            "1102" => "AnimatorState",
            "1107" => "AnimatorStateMachine",
            "687078895" => "SpriteAtlas",
            "114" => "MonoBehaviour",
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
    let expression = format!(r"(?m)^\s*{}:\s*\{{fileID:\s*(-?\d+)", regex::escape(key));
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
                .map(|value| value.chars().count())
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
    widths: &[f32],
    row: Option<&BTreeMap<String, String>>,
) {
    ui.horizontal(|ui| {
        for (column, header) in table.headers.iter().enumerate() {
            let text = row
                .and_then(|values| values.get(header))
                .map_or(header.as_str(), String::as_str);
            let label = if row.is_some() {
                egui::Label::new(RichText::new(text).monospace()).truncate()
            } else {
                egui::Label::new(RichText::new(text).strong())
            };
            let alignment = if row.is_some() && text.parse::<f64>().is_err() {
                egui::Align::Min
            } else {
                egui::Align::Center
            };
            ui.add_sized([widths[column], 20.0], label.halign(alignment));
        }
    });
}

fn show_yaml_table(ui: &mut egui::Ui, table: &YamlTable, freeze_header: bool) {
    ui.label(RichText::new(format!("{}（{} 条）", table.title, table.rows.len())).strong());
    let widths = table_column_widths(table);
    if freeze_header {
        show_table_row(ui, table, &widths, None);
        ui.separator();
    }
    ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(520.0)
        .show(ui, |ui| {
            if !freeze_header {
                show_table_row(ui, table, &widths, None);
            }
            for row in &table.rows {
                show_table_row(ui, table, &widths, Some(row));
            }
        });
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
        eframe::NativeOptions::default(),
        Box::new(|creation_context| {
            configure_system_fonts(&creation_context.egui_ctx);
            Ok(Box::new(ViewerApp {
                last_open_dir: load_last_open_dir(),
                ..Default::default()
            }))
        }),
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
