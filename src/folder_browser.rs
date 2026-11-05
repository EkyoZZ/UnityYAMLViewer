use std::{path::{Path, PathBuf}, sync::mpsc};
use eframe::egui;
use crate::localization::tr;

#[derive(Default)]
pub struct FolderBrowser {
    roots: Vec<Node>,
    saved: Vec<SavedRoot>,
    save_error: Option<String>,
    locate_root: Option<PathBuf>,
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct SavedRoot {
    path: PathBuf,
    #[serde(default)]
    ignored: String,
}

fn config_path() -> Option<PathBuf> {
    crate::last_open_dir_config_path().map(|path| path.with_file_name("folder_browser.json"))
}

struct Node {
    path: PathBuf,
    directory: bool,
    children: Option<Vec<Node>>,
    pending: Option<mpsc::Receiver<Result<Vec<Node>, String>>>,
    error: Option<String>,
    ignored: String,
}

impl Node {
    fn new(path: PathBuf, directory: bool) -> Self {
        Self { path, directory, children: None, pending: None, error: None, ignored: String::new() }
    }

    fn show(&mut self, ui: &mut egui::Ui, active: Option<&Path>, open: &mut Option<PathBuf>, root: bool, ignored: &[String], locate: bool) -> bool {
        let name = self.path.file_name().unwrap_or(self.path.as_os_str()).to_string_lossy().into_owned();
        let icon_size = ui.text_style_height(&egui::TextStyle::Button);
        let settings_width = if root { icon_size + ui.spacing().item_spacing.x } else { 0.0 };
        if !self.directory {
            if ui.selectable_label(active == Some(self.path.as_path()), name)
                .on_hover_text(self.path.display().to_string()).clicked() {
                *open = Some(self.path.clone());
            }
            return false;
        }
        let title: egui::WidgetText = name.into();
        let title = title.into_galley(ui, Some(egui::TextWrapMode::Truncate),
            (ui.available_width() - ui.spacing().indent - ui.spacing().button_padding.x * 2.0 - settings_width).max(0.0),
            egui::TextStyle::Button);
        let root_background = root.then(|| (ui.painter().add(egui::Shape::Noop), ui.available_rect_before_wrap()));
        let response = egui::CollapsingHeader::new(title).id_salt(&self.path).default_open(root).open(locate.then_some(true)).show(ui, |ui| {
            let line_top = ui.cursor().top();
            if let Some(receiver) = &self.pending {
                match receiver.try_recv() {
                    Ok(result) => {
                        self.pending = None;
                        match result {
                            Ok(children) => self.children = Some(children),
                            Err(error) => self.error = Some(error),
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.pending = None;
                        self.error = Some(tr("目录扫描失败").into());
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            if self.children.is_none() && self.pending.is_none() && self.error.is_none() {
                let path = self.path.clone();
                let context = ui.ctx().clone();
                let (sender, receiver) = mpsc::channel();
                self.pending = Some(receiver);
                std::thread::spawn(move || {
                    let _ = sender.send(read_directory(&path));
                    context.request_repaint();
                });
            }
            if let Some(error) = &self.error {
                ui.label(tr("目录扫描失败")).on_hover_text(error);
            } else if let Some(children) = &mut self.children {
                if children.is_empty() { ui.label(tr("空目录")); }
                for child in children {
                    let hidden = !child.directory && child.path.file_name().is_some_and(|name| {
                        let name = name.to_string_lossy().to_lowercase();
                        ignored.iter().any(|pattern| wildcard_matches(pattern, &name))
                    });
                    if !hidden { child.show(ui, active, open, false, ignored, false); }
                }
            } else { ui.spinner(); }
            paint_hierarchy_line(ui, line_top);
        }).header_response.on_hover_text(self.path.display().to_string());
        if locate { response.scroll_to_me(Some(egui::Align::Center)); }
        if let Some((shape, bounds)) = root_background {
            let row = egui::Rect::from_min_max(
                egui::pos2(bounds.left(), response.rect.top()),
                egui::pos2(bounds.right(), response.rect.bottom()));
            let color = if ui.visuals().dark_mode {
                egui::Color32::from_white_alpha(14)
            } else { egui::Color32::from_black_alpha(12) };
            ui.painter().set(shape, egui::Shape::rect_filled(row, 3.0, color));
        }
        let mut remove = false;
        if root {
            response.context_menu(|ui| {crate::theme::menu_no_wrap(ui); remove |= self.settings_ui(ui); });
            let rect = egui::Rect::from_center_size(
                egui::pos2(response.rect.right() + ui.spacing().item_spacing.x + icon_size / 2.0, response.rect.center().y),
                egui::Vec2::splat(icon_size));
            let id = ui.make_persistent_id(("root_folder_settings", &self.path));
            let button = ui.interact(rect, id, egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr("忽略显示的文件"));
            button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tr("忽略显示的文件")));
            let visuals = ui.style().interact(&button);
            if button.hovered() || button.is_pointer_button_down_on() {
                ui.painter().rect_filled(rect, 2.0, visuals.bg_fill);
            }
            let glyph_rect = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(icon_size * 0.72));
            crate::icons::paint(ui, crate::icons::Icon::Gear, glyph_rect, visuals.fg_stroke.color);
            ui.expand_to_include_rect(rect);
            let popup_id = id.with("popup");
            if button.clicked() { ui.memory_mut(|memory| memory.toggle_popup(popup_id)); }
            egui::popup_below_widget(ui, popup_id, &button, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                if self.settings_ui(ui) {
                    remove = true;
                    ui.memory_mut(|memory| memory.close_popup());
                }
            });
        }
        remove
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
                ui.set_max_width(360.0);
                ui.label(tr("忽略显示的文件"));
                ui.add(egui::TextEdit::singleline(&mut self.ignored)
                    .hint_text(egui::RichText::new(tr("示例: *.meta;*.tmp")).color(egui::Color32::GRAY))
                    .text_color(egui::Color32::WHITE)
                    .desired_width(240.0));
                ui.add(egui::Label::new(egui::RichText::new(tr("支持 * 和 ?，多个规则用分号分隔。应用于该根目录及其所有子目录，不影响其他根目录；仅隐藏文件，不删除文件。自动保存，重启后恢复。")).small()).wrap());
                ui.separator();
                let remove = ui.button(tr("从目录浏览中移除该目录")).clicked();
                if remove { ui.close_menu(); }
                remove
    }
}

fn paint_hierarchy_line(ui: &egui::Ui, top: f32) {
    let clip = ui.clip_rect();
    let scale = ui.ctx().pixels_per_point();
    let x = ((ui.min_rect().left() - ui.spacing().indent * 0.5) * scale).round() / scale;
    let top = (top.max(clip.top()) * scale).ceil() / scale;
    let bottom = ((ui.min_rect().bottom() - 2.0).min(clip.bottom()) * scale).floor() / scale;
    if bottom <= top || x + 1.0 / scale < clip.left() || x > clip.right() { return; }
    // Bound geometry before tessellation: a large directory can be hundreds of thousands
    // of logical pixels tall. A one-physical-pixel filled strip avoids thin-line AA jitter.
    ui.painter().rect_filled(
        egui::Rect::from_min_max(egui::pos2(x, top), egui::pos2(x + 1.0 / scale, bottom)),
        0.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
}

// Wildcard matching with backtracking to the most recent star.
fn wildcard_matches(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n) = (0, 0);
    let mut star = None;
    let mut retry = 0;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            retry = n;
        } else if let Some(position) = star {
            retry += 1;
            n = retry;
            p = position + 1;
        } else { return false; }
    }
    while p < pattern.len() && pattern[p] == '*' { p += 1; }
    p == pattern.len()
}

fn read_directory(path: &Path) -> Result<Vec<Node>, String> {
    let mut children = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        // Do not follow links or junctions into cycles or outside the chosen tree.
        if kind.is_symlink() { continue; }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if entry.metadata().map_err(|error| error.to_string())?.file_attributes() & 0x400 != 0 { continue; }
        }
        if kind.is_dir() || kind.is_file() {
            children.push(Node::new(entry.path(), kind.is_dir()));
        }
    }
    children.sort_by_cached_key(|node| (!node.directory, node.path.file_name().unwrap_or_default().to_string_lossy().to_lowercase()));
    Ok(children)
}

impl FolderBrowser {
    pub fn load() -> Self {
        let saved = config_path().and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Vec<SavedRoot>>(&bytes).ok()).unwrap_or_default();
        let mut browser = Self::default();
        for entry in &saved {
            if browser.roots.iter().any(|root| root.path == entry.path) { continue; }
            let mut node = Node::new(entry.path.clone(), true);
            node.ignored = entry.ignored.clone();
            browser.roots.push(node);
        }
        browser.saved = saved;
        browser
    }

    fn persist_if_changed(&mut self) {
        let current: Vec<SavedRoot> = self.roots.iter().map(|root| SavedRoot {
            path: root.path.clone(), ignored: root.ignored.clone(),
        }).collect();
        if current == self.saved { return; }
        let result = (|| -> Result<(), String> {
            let path = config_path().ok_or_else(|| "Application settings directory unavailable".to_owned())?;
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
            let bytes = serde_json::to_vec_pretty(&current).map_err(|error| error.to_string())?;
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
            std::fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
            Ok(())
        })();
        self.saved = current;
        self.save_error = result.err();
    }

    pub fn take_save_error(&mut self) -> Option<String> { self.save_error.take() }

    pub fn add(&mut self, path: PathBuf) -> PathBuf {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if !self.roots.iter().any(|root| root.path == path) {
            self.roots.push(Node::new(path.clone(), true));
            self.persist_if_changed();
        }
        self.locate_root = Some(path.clone());
        path
    }

    pub fn show(&mut self, ctx: &egui::Context, active: Option<&Path>) -> Option<(PathBuf, PathBuf)> {
        if self.roots.is_empty() { return None; }
        let locate_root = self.locate_root.take();
        let mut open = None;
        let mut remove = None;
        let width_limit = (ctx.screen_rect().width() * 0.8).max(140.0);
        // Keep the scrollbar outside the panel divider's resize hit area.
        let divider_gap = (ctx.style().interaction.resize_grab_radius_side.ceil() + 1.0).clamp(1.0, 16.0) as i8;
        let frame = egui::Frame::side_top_panel(&ctx.style())
            .inner_margin(egui::Margin { left: 8, right: divider_gap, top: 2, bottom: 0 });
        egui::SidePanel::left("folder_browser_manual_width").default_width(230.0)
            .frame(frame)
            .min_width(140.0).max_width(width_limit).resizable(true).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.strong(tr("目录"));
                if ui.small_button(tr("刷新目录")).clicked() {
                    for root in &mut self.roots {
                        root.children = None;
                        root.pending = None;
                        root.error = None;
                    }
                }
            });
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical().id_salt("folder_tree_scroll").auto_shrink([false, false])
                .max_height(ui.available_height()).show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.visuals_mut().indent_has_left_vline = false;
                for (index, root) in self.roots.iter_mut().enumerate() {
                    let ignored: Vec<String> = root.ignored.split([';', ',', '\n']).map(str::trim)
                        .filter(|rule| !rule.is_empty()).map(str::to_lowercase).collect();
                    let mut selected_file = None;
                    let locate = locate_root.as_ref() == Some(&root.path);
                    if root.show(ui, active, &mut selected_file, true, &ignored, locate) { remove = Some(index); }
                    if let Some(path) = selected_file { open = Some((path, root.path.clone())); }
                }
            });
        });
        if let Some(index) = remove {
            self.roots.remove(index);
            ctx.request_repaint();
        }
        self.persist_if_changed();
        open
    }
}
