//! App-rendered floating controls. Native APIs only place/exclude the top-level windows.
use super::{Area, displays, screen};
use crate::localization::tr;
use eframe::egui::{self, Color32, Stroke};
use std::path::PathBuf;

#[derive(Clone, Default, PartialEq)]
pub(super) struct Toolbar {
    pub items: Vec<String>,
    pub selected: usize,
    pub stop_label: String,
    pub can_start: bool,
    pub system_audio: bool,
    pub status: String,
    pub result: Option<PathBuf>,
    pub error: Option<String>,
}
#[derive(Clone, Copy)]
pub(super) enum Action {
    Toggle,
    ToggleSystemAudio,
    Settings,
    Close,
    Choose(usize),
    SetArea(Area),
    Edit,
    OpenFolder,
    History,
    ErrorDetails,
}
#[derive(Default)]
pub(super) struct View {
    fields: [String; 4],
    last_area: Option<Area>,
    dirty: bool,
    input_error: bool,
    size: egui::Vec2,
    body_size: egui::Vec2,
    row_group_widths: [f32; 5],
    combo_open: bool,
    pub ready: bool,
    pub error: Option<String>,
    pub handles: [isize; 2],
    pixels_per_point: f32,
    created: bool,
    was_selecting: bool,
    clip_cache: Option<(isize, Vec<[i32; 4]>)>,
}
impl View {
    pub fn close(&mut self, ctx: &egui::Context) {
        if self.created {
            ctx.send_viewport_cmd_to(id("toolbar"), egui::ViewportCommand::Close);
        }
        self.created = false;
        self.was_selecting = false;
        self.last_area = None;
        self.dirty = false;
        self.input_error = false;
        self.error = None;
        self.ready = false;
        self.combo_open = false;
        self.handles = [0; 2];
        self.clip_cache = None;
        self.row_group_widths = [0.0; 5];
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        area: Area,
        controls: &Toolbar,
        editable: bool,
        selecting: bool,
        recording: bool,
        visible: bool,
    ) -> Vec<Action> {
        let mut actions = vec![];
        if selecting && !self.was_selecting {
            self.last_area = None;
            self.dirty = false;
            self.input_error = false;
        }
        self.was_selecting = selecting;
        if self.last_area != Some(area) || (!editable && !selecting) {
            self.fields = [area.x, area.y, area.width, area.height].map(|n| n.to_string());
            self.last_area = Some(area);
            self.dirty = false;
            self.input_error = false;
        }
        let title = title("toolbar");
        let mut scale = if self.pixels_per_point > 0.0 {
            self.pixels_per_point
        } else {
            ctx.pixels_per_point()
        };
        let work: Vec<Area> = displays::list()
            .into_iter()
            .map(|display| display.work)
            .collect();
        let max_width = work.iter().map(|area| area.width).max().unwrap_or(800) as f32 / scale;
        let previous_groups = self.row_group_widths;
        let group_widths = if self.row_group_widths.iter().all(|width| *width > 0.0) {
            self.row_group_widths
        } else {
            [156.0, 64.0, 64.0, 132.0, 94.0]
        };
        // Four inter-group gaps plus the frame's inner margin and stroke.
        // Measure the controls themselves, so result text cannot widen the toolbar.
        let preferred_width = group_widths.iter().sum::<f32>() + 4.0 * 4.0 + 14.0;
        let width = preferred_width.min((max_width - 40.0).max(260.0));
        const PAD: f32 = 12.0;
        let previous_height = if self.body_size.y > 0.0 {
            self.body_size.y
        } else {
            66.0
        };
        let anchor = placement(
            area,
            ((width + PAD * 2.0) * scale).ceil() as i32,
            ((previous_height + PAD * 2.0) * scale).ceil() as i32,
            &work,
        );
        let menu_height = (controls.items.len() as f32 * 24.0 + 16.0).min(180.0);
        let (above, reserve) = popup_space(
            self.combo_open,
            anchor,
            [
                ((width + PAD * 2.0) * scale).ceil() as i32,
                ((previous_height + PAD * 2.0) * scale).ceil() as i32,
            ],
            menu_height,
            scale,
            &work,
        );
        let body_offset = if above { reserve } else { 0.0 };
        let mut logical_size = egui::vec2(width + PAD * 2.0, previous_height + reserve + PAD * 2.0);
        let mut body_rect = egui::Rect::NOTHING;
        let mut popup_rect = None;
        let builder = egui::ViewportBuilder::default()
            .with_title(&title)
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false)
            .with_visible(false)
            .with_inner_size(logical_size)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        self.created = true;
        ctx.show_viewport_immediate(id("toolbar"), builder, |child, class| {
            if class == egui::ViewportClass::Embedded {
                self.error = Some(tr("录屏悬浮工具条需要独立窗口支持。").into());
                return;
            }
            scale = child.pixels_per_point();
            if visible && !recording && child.input(|input| input.key_pressed(egui::Key::Escape)) {
                actions.push(Action::Close);
            }
            if child.input(|input| input.viewport().close_requested()) {
                child.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                actions.push(Action::Close);
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(PAD))
                .show(child, |outer| {
                    if body_offset > 0.0 {
                        outer.add_space(body_offset);
                    }
                    let frame = egui::Frame::new()
                        .fill(outer.visuals().window_fill)
                        .stroke(Stroke::new(1.0_f32, outer.visuals().window_stroke.color))
                        .corner_radius(8)
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 2],
                            blur: 10,
                            spread: 0,
                            color: Color32::from_black_alpha(90),
                        })
                        .inner_margin(6);
                    let response = frame.show(outer, |ui| {
                        ui.set_min_width((width - 14.0).max(0.0));
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
                        ui.spacing_mut().interact_size.y = 20.0;
                        let mut commit = false;
                        ui.horizontal_wrapped(|ui| {
                            let group = ui.allocate_ui_with_layout(
                                egui::vec2(group_widths[0], 30.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let label = controls
                                        .items
                                        .get(controls.selected)
                                        .map_or(tr("指定区域"), String::as_str);
                                    let combo = ui
                                        .add_enabled_ui(!recording, |ui| {
                                            egui::ComboBox::from_id_salt("recording_capture_mode")
                                                .width(88.0)
                                                .height((menu_height - 12.0).max(24.0))
                                                .selected_text(label)
                                                .show_ui(ui, |ui| {
                                                    for (index, label) in
                                                        controls.items.iter().enumerate()
                                                    {
                                                        if ui
                                                            .selectable_label(
                                                                index == controls.selected,
                                                                label,
                                                            )
                                                            .clicked()
                                                        {
                                                            actions.push(Action::Choose(index));
                                                        }
                                                    }
                                                    ui.min_rect()
                                                })
                                        })
                                        .inner;
                                    if let Some(rect) = combo.inner {
                                        popup_rect = Some(
                                            rect + egui::Frame::popup(ui.style()).total_margin(),
                                        );
                                    }
                                    self.combo_open =
                                        egui::ComboBox::is_open(child, combo.response.id);
                                    if child.input(|input| input.viewport().focused == Some(false))
                                        && self.combo_open
                                    {
                                        child.memory_mut(|memory| memory.close_popup());
                                        self.combo_open = false;
                                    }
                                    if cross_button(ui, !recording)
                                        .on_hover_text(tr("重新选择区域"))
                                        .clicked()
                                    {
                                        actions.push(Action::Choose(0));
                                        child.memory_mut(|memory| memory.close_popup());
                                        self.combo_open = false;
                                    }
                                    if ui
                                        .add_enabled_ui(!recording, crate::theme::settings_button)
                                        .inner
                                        .clicked()
                                    {
                                        actions.push(Action::Settings);
                                        child.memory_mut(|memory| memory.close_popup());
                                        self.combo_open = false;
                                    }
                                },
                            );
                            self.row_group_widths[0] =
                                (group.response.rect.width() * scale).ceil() / scale;
                            for (index, label) in ["X", "Y"].into_iter().enumerate() {
                                let group = ui.allocate_ui_with_layout(
                                    egui::vec2(group_widths[index + 1], 30.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.label(egui::RichText::new(label).strong());
                                        let (changed, apply) = coordinate_field(
                                            ui,
                                            &mut self.fields[index],
                                            editable && !selecting && !recording,
                                        );
                                        self.dirty |= changed;
                                        commit |= apply;
                                    },
                                );
                                self.row_group_widths[index + 1] =
                                    (group.response.rect.width() * scale).ceil() / scale;
                            }
                            let group = ui.allocate_ui_with_layout(
                                egui::vec2(group_widths[3], 30.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    for index in [2, 3] {
                                        if index == 3 {
                                            ui.label("×");
                                        }
                                        let (changed, apply) = coordinate_field(
                                            ui,
                                            &mut self.fields[index],
                                            editable && !selecting && !recording,
                                        );
                                        self.dirty |= changed;
                                        commit |= apply;
                                    }
                                    ui.label("px");
                                },
                            );
                            self.row_group_widths[3] =
                                (group.response.rect.width() * scale).ceil() / scale;
                            let group = ui.allocate_ui_with_layout(
                                egui::vec2(group_widths[4], 30.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let response = ui
                                        .add_enabled_ui(!recording && controls.can_start, |ui| {
                                            let response = ui.add_sized(
                                                [28.0, 28.0],
                                                egui::Button::new("")
                                                    .selected(controls.system_audio),
                                            );
                                            crate::icons::paint(
                                                ui,
                                                crate::icons::Icon::Microphone,
                                                response.rect.shrink(6.0),
                                                ui.visuals().text_color(),
                                            );
                                            if !controls.system_audio {
                                                let rect = response.rect.shrink(4.0);
                                                let stroke = Stroke::new(
                                                    1.6_f32,
                                                    Color32::from_rgb(230, 65, 75),
                                                );
                                                ui.painter().circle_stroke(
                                                    rect.center(),
                                                    rect.width() * 0.5,
                                                    stroke,
                                                );
                                                ui.painter().line_segment(
                                                    [rect.left_bottom(), rect.right_top()],
                                                    stroke,
                                                );
                                            }
                                            response.on_hover_text(if controls.system_audio {
                                                tr("系统声音：开启（不采集麦克风）")
                                            } else {
                                                tr("系统声音：关闭")
                                            })
                                        })
                                        .inner;
                                    if response.clicked() {
                                        actions.push(Action::ToggleSystemAudio);
                                    }
                                    if record_button(
                                        ui,
                                        recording,
                                        recording
                                            || (self.ready
                                                && controls.can_start
                                                && !selecting
                                                && !self.input_error),
                                    )
                                    .clicked()
                                    {
                                        actions.push(Action::Toggle);
                                        commit = true;
                                        child.memory_mut(|memory| memory.close_popup());
                                        self.combo_open = false;
                                    }
                                    if close_button(ui).clicked() {
                                        actions.push(Action::Close);
                                    }
                                },
                            );
                            self.row_group_widths[4] =
                                (group.response.rect.width() * scale).ceil() / scale;
                        });
                        if commit && self.dirty && editable && !selecting {
                            let parsed = self
                                .fields
                                .iter()
                                .map(|text| text.trim().parse::<i32>())
                                .collect::<Result<Vec<_>, _>>();
                            let area = parsed
                                .ok()
                                .map(|n| Area {
                                    x: n[0],
                                    y: n[1],
                                    width: n[2],
                                    height: n[3],
                                })
                                .filter(|area| {
                                    area.width >= 16
                                        && area.height >= 16
                                        && area.valid(screen(true))
                                });
                            self.input_error = area.is_none();
                            if let Some(area) = area {
                                actions.insert(0, Action::SetArea(area));
                                self.dirty = false;
                            }
                        }
                        if self.input_error {
                            actions.retain(|action| !matches!(action, Action::Toggle));
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                tr("坐标或尺寸无效，请检查输入。"),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{}: {}", tr("停止"), controls.stop_label));
                            if !controls.status.is_empty() {
                                ui.label(&controls.status);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button(tr("历史记录")).clicked() {
                                        actions.push(Action::History);
                                        child.memory_mut(|memory| memory.close_popup());
                                        self.combo_open = false;
                                    }
                                },
                            );
                        });
                        if let Some(error) = &controls.error {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(error).color(ui.visuals().error_fg_color),
                                )
                                .wrap(),
                            );
                            if ui.button(tr("查看错误详情")).clicked() {
                                actions.push(Action::ErrorDetails);
                            }
                        }
                        if let Some(path) = &controls.result {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(tr("录屏已保存")).strong());
                                if ui.button(tr("视频编辑")).clicked() {
                                    actions.push(Action::Edit);
                                }
                                if ui.button(tr("打开所在文件夹")).clicked() {
                                    actions.push(Action::OpenFolder);
                                }
                            });
                            let text = path.display().to_string();
                            let mut read_only = text.as_str();
                            ui.add(
                                egui::TextEdit::singleline(&mut read_only)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Body),
                            )
                            .on_hover_text(text);
                        }
                    });
                    body_rect = response.response.rect;
                });
            logical_size = egui::vec2(width + PAD * 2.0, body_rect.height() + reserve + PAD * 2.0);
            if self.size != logical_size {
                child.send_viewport_cmd(egui::ViewportCommand::InnerSize(logical_size));
            }
        });
        self.body_size = body_rect.size();
        self.size = logical_size;
        if self.pixels_per_point != scale {
            self.pixels_per_point = scale;
            ctx.request_repaint();
        }
        let body_position = placement(
            area,
            ((width + PAD * 2.0) * scale).ceil() as i32,
            ((body_rect.height() + PAD * 2.0) * scale).ceil() as i32,
            &work,
        );
        let position = (
            body_position.0,
            body_position.1 - (body_offset * scale).round() as i32,
        );
        let physical_size = [
            (logical_size.x * scale).ceil() as i32,
            (logical_size.y * scale).ceil() as i32,
        ];
        let mut clips = vec![physical_rect(body_rect.expand(PAD), scale)];
        if let Some(rect) = popup_rect {
            clips.push(physical_rect(rect.expand(PAD), scale));
        }
        let was_ready = self.ready;
        match native_place(
            &title,
            position,
            physical_size,
            visible,
            &clips,
            &mut self.clip_cache,
        ) {
            Ok(handle) => {
                self.handles = [handle.unwrap_or(0), 0];
                self.ready = handle.is_some();
                if self.ready {
                    self.error = None;
                }
            }
            Err(error) => {
                self.error = Some(error);
                self.ready = false;
            }
        }
        if self.ready != was_ready {
            ctx.request_repaint();
        }
        if !self.ready {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        if self.combo_open
            || reserve > 0.0
            || !actions.is_empty()
            || self.row_group_widths != previous_groups
        {
            ctx.request_repaint();
        }
        actions
    }
}
fn coordinate_field(ui: &mut egui::Ui, text: &mut String, enabled: bool) -> (bool, bool) {
    let response = ui.add_enabled(
        enabled,
        egui::TextEdit::singleline(text)
            .desired_width(40.0)
            .margin(egui::vec2(4.0, 2.0)),
    );
    (
        response.changed(),
        response.lost_focus()
            || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))),
    )
}
fn physical_rect(rect: egui::Rect, scale: f32) -> [i32; 4] {
    [
        (rect.left() * scale).floor() as i32,
        (rect.top() * scale).floor() as i32,
        (rect.right() * scale).ceil() as i32,
        (rect.bottom() * scale).ceil() as i32,
    ]
}
fn popup_space(
    open: bool,
    position: (i32, i32),
    size: [i32; 2],
    menu_height: f32,
    scale: f32,
    work: &[Area],
) -> (bool, f32) {
    if !open {
        return (false, 0.0);
    }
    let work = work
        .iter()
        .find(|work| {
            position.0 >= work.x
                && position.0 + size[0] <= work.x + work.width
                && position.1 >= work.y
                && position.1 + size[1] <= work.y + work.height
        })
        .copied()
        .unwrap_or_else(|| screen(true));
    let below = (work.y + work.height - position.1 - size[1]).max(0) as f32 / scale;
    let above = (position.1 - work.y).max(0) as f32 / scale;
    let place_above = below < menu_height && above > below;
    (
        place_above,
        menu_height
            .min(if place_above { above } else { below })
            .max(0.0),
    )
}
fn id(suffix: &str) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("screen_record", suffix))
}
fn title(suffix: &str) -> String {
    format!("UnityYAMLViewer.Recording.{}.{suffix}", std::process::id())
}
fn cross_button(ui: &mut egui::Ui, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("").min_size(egui::vec2(28.0, 28.0)),
    );
    let center = response.rect.center();
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tr("重新选择区域"))
    });
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let stroke = Stroke::new(1.4_f32, color);
    ui.painter().circle_stroke(center, 8.0, stroke);
    ui.painter().line_segment(
        [
            center - egui::vec2(11.0, 0.0),
            center + egui::vec2(11.0, 0.0),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            center - egui::vec2(0.0, 11.0),
            center + egui::vec2(0.0, 11.0),
        ],
        stroke,
    );
    response
}
fn close_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr("关闭")));
    let visuals = ui.style().interact(&response);
    if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);
    }
    // Draw at the viewport's current physical scale instead of resampling an SVG
    // texture. Pixel-aligned endpoints and round caps keep the small X crisp.
    let scale = ui.ctx().pixels_per_point();
    let center = rect.center();
    let half = (7.0 * scale).round() / scale;
    let snap = |point: egui::Pos2| {
        egui::pos2(
            (point.x * scale).round() / scale,
            (point.y * scale).round() / scale,
        )
    };
    let stroke = Stroke::new(
        (2.0 * scale).round().max(1.0) / scale,
        visuals.fg_stroke.color,
    );
    for ends in [
        [
            snap(center - egui::vec2(half, half)),
            snap(center + egui::vec2(half, half)),
        ],
        [
            snap(center + egui::vec2(-half, half)),
            snap(center + egui::vec2(half, -half)),
        ],
    ] {
        ui.painter().line_segment(ends, stroke);
        for point in ends {
            ui.painter()
                .circle_filled(point, stroke.width * 0.5, stroke.color);
        }
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr("关闭"))
}
fn record_button(ui: &mut egui::Ui, recording: bool, enabled: bool) -> egui::Response {
    let response = ui
        .add_enabled(
            enabled,
            egui::Button::new("").min_size(egui::vec2(28.0, 28.0)),
        )
        .on_hover_text(tr(if recording {
            "停止并保存"
        } else {
            "开始录屏"
        }));
    let color = if enabled {
        Color32::from_rgb(240, 62, 80)
    } else {
        ui.visuals().weak_text_color()
    };
    let center = response.rect.center();
    if recording {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(center, egui::vec2(14.0, 14.0)),
            2,
            color,
        );
    } else {
        ui.painter().circle_filled(center, 8.0, color);
    }
    response
}
pub(super) fn placement(area: Area, width: i32, height: i32, work: &[Area]) -> (i32, i32) {
    let gap = 6;
    let mut monitors: Vec<Area> = work
        .iter()
        .copied()
        .filter(|work| work.width >= width && work.height >= height)
        .collect();
    monitors.sort_by_key(|work| {
        let dx = i64::from(work.x) + i64::from(work.width) / 2
            - i64::from(area.x)
            - i64::from(area.width) / 2;
        let dy = i64::from(work.y) + i64::from(work.height) / 2
            - i64::from(area.y)
            - i64::from(area.height) / 2;
        dx * dx + dy * dy
    });
    for side in 0..4 {
        for work in &monitors {
            let x = (area.x + area.width - width).clamp(work.x, work.x + work.width - width);
            let y = area.y.clamp(work.y, work.y + work.height - height);
            let (x, y) = match side {
                0 => (x, area.y + area.height + gap),
                1 => (x, area.y - height - gap),
                2 => (area.x + area.width + gap, y),
                _ => (area.x - width - gap, y),
            };
            if side < 2 && (x >= area.x + area.width || x + width <= area.x) {
                continue;
            }
            if side >= 2 && (y >= area.y + area.height || y + height <= area.y) {
                continue;
            }
            if x >= work.x
                && y >= work.y
                && x + width <= work.x + work.width
                && y + height <= work.y + work.height
            {
                return (x, y);
            }
        }
    }
    let work = monitors.first().copied().unwrap_or_else(|| screen(true));
    (
        (area.x + area.width - width).clamp(work.x, (work.x + work.width - width).max(work.x)),
        (area.y + area.height - height - gap)
            .clamp(work.y, (work.y + work.height - height).max(work.y)),
    )
}
#[cfg(windows)]
pub(super) fn native_place(
    title: &str,
    position: (i32, i32),
    size: [i32; 2],
    visible: bool,
    clips: &[[i32; 4]],
    clip_cache: &mut Option<(isize, Vec<[i32; 4]>)>,
) -> Result<Option<isize>, String> {
    use std::{ffi::c_void, ptr::null};
    type Handle = *mut c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> Handle;
        fn GetWindowThreadProcessId(hwnd: Handle, process: *mut u32) -> u32;
        fn SetWindowDisplayAffinity(hwnd: Handle, affinity: u32) -> i32;
        fn GetWindowDisplayAffinity(hwnd: Handle, affinity: *mut u32) -> i32;
        fn SetWindowPos(
            hwnd: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn ShowWindow(hwnd: Handle, command: i32) -> i32;
        fn SetWindowRgn(hwnd: Handle, region: Handle, redraw: i32) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> Handle;
        fn CombineRgn(dest: Handle, first: Handle, second: Handle, mode: i32) -> i32;
        fn DeleteObject(object: Handle) -> i32;
    }
    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let hwnd = FindWindowW(null(), title.as_ptr());
        if hwnd.is_null() {
            return Ok(None);
        }
        let mut process = 0;
        GetWindowThreadProcessId(hwnd, &mut process);
        if process != std::process::id() {
            return Err(tr("无法创建可排除捕获的录屏线框。").into());
        }
        let mut affinity = 0;
        if SetWindowDisplayAffinity(hwnd, 0x11) == 0
            || GetWindowDisplayAffinity(hwnd, &mut affinity) == 0
            || affinity != 0x11
        {
            ShowWindow(hwnd, 0);
            return Err(format!(
                "{}: {}",
                tr("无法创建可排除捕获的录屏线框。"),
                std::io::Error::last_os_error()
            ));
        }
        if visible {
            if clip_cache.as_ref().is_none_or(|(previous, rects)| {
                *previous != hwnd as isize || rects.as_slice() != clips
            }) {
                let region = CreateRectRgn(0, 0, 0, 0);
                if region.is_null() {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                for rect in clips {
                    if rect[2] > rect[0] && rect[3] > rect[1] {
                        let part = CreateRectRgn(rect[0], rect[1], rect[2], rect[3]);
                        if part.is_null() {
                            DeleteObject(region);
                            return Err(std::io::Error::last_os_error().to_string());
                        }
                        let combined = CombineRgn(region, region, part, 2);
                        DeleteObject(part);
                        if combined == 0 {
                            DeleteObject(region);
                            return Err(std::io::Error::last_os_error().to_string());
                        }
                    }
                }
                if SetWindowRgn(hwnd, region, 1) == 0 {
                    DeleteObject(region);
                    return Err(std::io::Error::last_os_error().to_string());
                }
                *clip_cache = Some((hwnd as isize, clips.to_vec()));
            }
            if SetWindowPos(
                hwnd,
                (-1isize) as Handle,
                position.0,
                position.1,
                size[0],
                size[1],
                0x10 | 0x40,
            ) == 0
            {
                ShowWindow(hwnd, 0);
                return Err(std::io::Error::last_os_error().to_string());
            }
        } else {
            ShowWindow(hwnd, 0);
        }
        Ok(Some(hwnd as isize))
    }
}
#[cfg(not(windows))]
pub(super) fn native_place(
    _: &str,
    _: (i32, i32),
    _: [i32; 2],
    _: bool,
    _: &[[i32; 4]],
    _: &mut Option<(isize, Vec<[i32; 4]>)>,
) -> Result<Option<isize>, String> {
    Err(tr("录屏当前仅支持 Windows。").into())
}
