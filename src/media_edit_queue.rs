use crate::{icons, localization::tr};
use eframe::egui;
use std::path::PathBuf;

pub(crate) const HEADER_HEIGHT: f32 = 32.0;

pub(crate) struct Item {
    pub path: PathBuf,
    pub thumbnail: Option<egui::TextureHandle>,
    pub enabled: bool,
}

struct QueueDrag {
    viewport: egui::ViewportId,
    index: usize,
}

fn insertion_target(from: usize, boundary: usize) -> usize {
    boundary.saturating_sub(usize::from(from < boundary))
}

#[derive(Default)]
pub(crate) struct Panel {
    pub items: Vec<Item>,
    pub active: usize,
    pub expanded: bool,
    pub merge: bool,
    pub merge_settings: crate::media_trim::MergeSettings,
    pub select: Option<usize>,
    pub reorder: Option<(usize, usize)>,
    pub remove: Option<usize>,
    pub undo_remove: bool,
    pub can_undo_remove: bool,
    pub add: Vec<PathBuf>,
    pub export: bool,
    pub save_project: bool,
    pub project_path: Option<PathBuf>,
    pub project_save_as: bool,
    pub project_saving: bool,
    pub confirm_discard: bool,
    pub discard_confirmed: bool,
    pub loading: bool,
    pub error: Option<String>,
}

impl Panel {
    pub fn toolbar(&mut self, ui: &mut egui::Ui) {
        let response = crate::theme::toolbar_icon(ui, icons::Icon::EditList, tr("显示编辑列表"));
        if response.clicked() {
            self.expanded = !self.expanded;
        }
        if let Some(url) = crate::background_tasks::button(ui) {
            ui.ctx()
                .data_mut(|data| data.insert_temp(egui::Id::new("background_task_open_url"), url));
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
    }

    fn add_button(&mut self, ui: &mut egui::Ui, video: bool) {
        let add = ui.add_sized([24.0, 24.0], egui::Button::new(""));
        let center = add.rect.center();
        for offset in [egui::vec2(6.0, 0.0), egui::vec2(0.0, 6.0)] {
            ui.painter().line_segment([center-offset, center+offset], egui::Stroke::new(1.5_f32, ui.visuals().text_color()));
        }
        if add.on_hover_text(tr("添加文件…")).clicked() {
            let extensions: &[&str] = if video {
                &[
                    "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpeg", "mpg", "ts",
                    "mts", "m2ts", "3gp", "gif",
                ]
            } else {
                &[
                    "mp3", "wav", "ogg", "flac", "aac", "m4a", "opus", "wma", "aiff",
                ]
            };
            if let Some(paths) = rfd::FileDialog::new()
                .add_filter(tr(if video { "视频" } else { "音频" }), extensions)
                .pick_files()
            {
                self.add.extend(paths);
            }
        }
    }

    pub fn sidebar(&mut self, ui: &mut egui::Ui, video: bool) {
        if self.items.len()==1 { self.items[0].enabled=true; }
        if !self.items.iter().any(|item|item.enabled) {
            if let Some(item)=self.items.get_mut(self.active){item.enabled=true;}
        }
        let dropped = ui.ctx().input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.add.extend(dropped);
            self.expanded = true;
        }
        if !self.expanded {
            return;
        }
        let panel_id = ui.id().with(("media_edit_queue", ui.ctx().viewport_id(), video));
        let mut frame = egui::Frame::side_top_panel(ui.style());
        // The native editor already supplies the outer left margin.
        frame.inner_margin.left = 0;
        frame.inner_margin.top = 0;
        egui::SidePanel::left(panel_id)
            .default_width(190.0)
            .width_range(150.0..=300.0)
            .resizable(true)
            .frame(frame)
            .show_inside(ui, |ui| {
                let mut mode = if self.merge { "merge" } else { "queue" }.to_owned();
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), HEADER_HEIGHT), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    crate::theme::compact_selector(
                        ui,
                        &mut mode,
                        [("queue", "队列"), ("merge", "合并")],
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.add_button(ui, video);
                        if self.loading { ui.spinner(); }
                    });
                });
                self.merge = mode == "merge";
                ui.separator();
                if self.merge {
                    if video {
                        egui::CollapsingHeader::new(tr("合并导出编码设置"))
                            .id_salt("merge_export_settings")
                            .default_open(true)
                            .show(ui, |ui| {
                                self.merge_settings.controls(ui, true);
                            });
                    } else {
                        ui.strong(tr("合并导出编码设置"));
                        self.merge_settings.controls(ui, false);
                    }
                    ui.separator();
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let removable = self.items.len() > 1;
                        let mut checked_count=self.items.iter().filter(|item|item.enabled).count();
                        for (index, item) in self.items.iter_mut().enumerate() {
                            let mut frame =
                                egui::Frame::group(ui.style()).fill(if index == self.active {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().faint_bg_color
                                });
                            if item.thumbnail.is_none() {
                                // 22px controls plus the same 4px inset on every side.
                                // Include the frame stroke in the 30px outer row height.
                                frame.inner_margin.top = 2;
                                frame.inner_margin.bottom = 2;
                            }
                            let card = frame
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let name =
                                        item.path.file_name().unwrap_or_default().to_string_lossy();
                                    let color = if item.path.is_file() {
                                        ui.visuals().text_color()
                                    } else {
                                        ui.visuals().error_fg_color
                                    };
                                    let label = egui::Label::new(
                                        egui::RichText::new(name.as_ref()).color(color),
                                    )
                                    .truncate().halign(egui::Align::Min).selectable(false);
                                    if let Some(texture) = &item.thumbnail {
                                        let width = ui.available_width();
                                        let image = ui.add(
                                            egui::Image::new(texture)
                                                .fit_to_exact_size(egui::vec2(width, width * 0.56))
                                                .maintain_aspect_ratio(true),
                                        );
                                        let mut caption = image.rect;
                                        caption.min.y = (caption.max.y
                                            - ui.text_style_height(&egui::TextStyle::Body)
                                            - 8.0)
                                            .max(caption.min.y);
                                        ui.painter().rect_filled(
                                            caption,
                                            0.0,
                                            egui::Color32::from_black_alpha(170),
                                        );
                                        let caption_color = if item.path.is_file() {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::from_rgb(255, 110, 110)
                                        };
                                        let text_rect=caption.shrink2(egui::vec2(4.0,2.0));
                                        let mut text_ui=ui.new_child(egui::UiBuilder::new().max_rect(text_rect)
                                            .layout(egui::Layout::left_to_right(egui::Align::Center)));
                                        text_ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(name.as_ref())
                                                    .color(caption_color),
                                            )
                                            .truncate().halign(egui::Align::Min).selectable(false),
                                        );
                                    } else {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 24.0),
                                            egui::Sense::hover(),
                                        );
                                        let mut text_rect=rect;
                                        text_rect.min.x+=32.0;
                                        text_rect.max.x-=24.0;
                                        let mut text_ui=ui.new_child(egui::UiBuilder::new().max_rect(text_rect)
                                            .layout(egui::Layout::left_to_right(egui::Align::Center)));
                                        text_ui.add(label);
                                    }
                                })
                                .response;
                            let row = ui.interact(
                                card.rect,
                                ui.id().with(("edit_queue_card", &item.path)),
                                egui::Sense::click(),
                            );
                            let handle_rect = egui::Rect::from_min_size(
                                egui::pos2(card.rect.left()+4.0, if item.thumbnail.is_some() { card.rect.top()+4.0 } else { card.rect.center().y-11.0 }),
                                egui::vec2(30.0, 22.0),
                            );
                            let remove_rect = egui::Rect::from_min_size(
                                egui::pos2(card.rect.right() - 26.0, if item.thumbnail.is_some() { card.rect.top()+4.0 } else { card.rect.center().y-11.0 }),
                                egui::vec2(22.0, 22.0),
                            );
                            let handle = ui.put(
                                handle_rect,
                                egui::Button::new("").min_size(egui::vec2(30.0,22.0)).sense(egui::Sense::drag()),
                            );
                            let number=(index+1).to_string();
                            let font_size=match number.len() {0..=2=>13.0,3=>11.0,_=>9.0};
                            ui.painter().text(
                                handle.rect.center(),egui::Align2::CENTER_CENTER,
                                number,egui::FontId::monospace(font_size),
                                ui.visuals().text_color(),
                            );
                            handle.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    tr("拖动排序"),
                                )
                            });
                            if handle.drag_started_by(egui::PointerButton::Primary) {
                                handle.dnd_set_drag_payload(QueueDrag {
                                    viewport: ui.ctx().viewport_id(),
                                    index,
                                });
                            }
                            let dragging = handle.dragged_by(egui::PointerButton::Primary);
                            let control_size=handle.rect.width().min(handle.rect.height());
                            handle
                                .on_hover_text(tr("拖动排序"))
                                .on_hover_cursor(if dragging {
                                    egui::CursorIcon::Grabbing
                                } else {
                                    egui::CursorIcon::Grab
                                });
                            if removable { ui.scope(|ui| {
                                ui.spacing_mut().icon_width=control_size;
                                ui.spacing_mut().icon_width_inner=(control_size-9.0).max(7.0);
                                let widgets=&mut ui.visuals_mut().widgets;
                                for visuals in [&mut widgets.inactive,&mut widgets.hovered,&mut widgets.active,&mut widgets.noninteractive] {
                                    visuals.expansion=0.0;
                                }
                                let green = if ui.visuals().dark_mode {
                                    egui::Color32::from_rgb(65,220,110)
                                } else { egui::Color32::from_rgb(15,135,55) };
                                if item.enabled {
                                    let widgets=&mut ui.visuals_mut().widgets;
                                    for visuals in [&mut widgets.inactive,&mut widgets.hovered,&mut widgets.active,&mut widgets.noninteractive] {
                                        visuals.bg_fill=green;
                                        visuals.weak_bg_fill=green;
                                        visuals.bg_stroke.color=green;
                                        visuals.fg_stroke.color=egui::Color32::WHITE;
                                        visuals.fg_stroke.width=2.5;
                                    }
                                }
                                let was_enabled=item.enabled;
                                ui.put(remove_rect,egui::Checkbox::without_text(&mut item.enabled))
                                    .on_hover_text(tr("参与导出"));
                                if was_enabled && !item.enabled {
                                    if checked_count==1 {item.enabled=true;} else {checked_count-=1;}
                                }else if !was_enabled && item.enabled {checked_count+=1;}
                            }); }
                            row.context_menu(|ui| {
                                if ui.add_enabled(removable,egui::Button::new(tr("移除"))).clicked() {
                                    self.remove=Some(index);ui.close_menu();
                                }
                            });
                            if row.clicked()
                                && row.interact_pointer_pos().is_some_and(|pointer| {
                                    !handle_rect.contains(pointer)
                                        && !remove_rect.contains(pointer)
                                })
                            {
                                self.select = Some(index);
                            }
                            if row.hovered() && ui.input(|input|input.pointer.hover_pos()).is_some_and(|pointer|
                                !handle_rect.contains(pointer) && !remove_rect.contains(pointer)) {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                            }
                            row.clone().on_hover_text(item.path.display().to_string());
                            let after = ui.input(|input|input.pointer.hover_pos())
                                .is_some_and(|pointer|pointer.y >= row.rect.center().y);
                            let boundary = index + usize::from(after);
                            if let Some(from) = row.dnd_release_payload::<QueueDrag>() {
                                let target = insertion_target(from.index, boundary);
                                if from.viewport == ui.ctx().viewport_id() && from.index != target {
                                    self.reorder = Some((from.index, target));
                                }
                            }
                            if row
                                .dnd_hover_payload::<QueueDrag>()
                                .is_some_and(|from| from.viewport == ui.ctx().viewport_id())
                            {
                                ui.painter().line_segment(
                                    if after { [row.rect.left_bottom(), row.rect.right_bottom()] }
                                    else { [row.rect.left_top(), row.rect.right_top()] },
                                    egui::Stroke::new(2.0_f32, crate::theme::BLUE),
                                );
                            }
                            // Overlay widgets use ui.put, which moves the layout cursor
                            // back up to the corner controls. Resume below the full card.
                            ui.advance_cursor_after_rect(card.rect);
                        }
                        if !self.items.is_empty() {
                            // The tail is only a drop target, not scrollable content.
                            // Allocating the remaining viewport height here lets the
                            // scroll offset itself grow the content on every pass.
                            let top=ui.cursor().top();
                            let rect=egui::Rect::from_min_max(
                                egui::pos2(ui.clip_rect().left(),top),
                                egui::pos2(ui.clip_rect().right(),(top+8.0).min(ui.clip_rect().bottom()).max(top)),
                            );
                            let tail=ui.interact(rect,ui.id().with("queue_tail_drop"),egui::Sense::hover());
                            if tail.dnd_hover_payload::<QueueDrag>().is_some_and(|from|from.viewport==ui.ctx().viewport_id()) {
                                ui.painter().line_segment([rect.left_top(),rect.right_top()],egui::Stroke::new(2.0_f32,crate::theme::BLUE));
                            }
                            if let Some(from) = tail.dnd_release_payload::<QueueDrag>() {
                                let target = self.items.len()-1;
                                if from.viewport==ui.ctx().viewport_id() && from.index!=target {
                                    self.reorder=Some((from.index,target));
                                }
                            }
                        }
                    });
                if let Some(error) = &self.error {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(error).color(ui.visuals().error_fg_color),
                        )
                        .wrap(),
                    );
                }
            });
    }
}
