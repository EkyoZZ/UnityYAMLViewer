use std::collections::HashMap;
use crate::localization::tr;
use eframe::egui::{self, Color32, Rect, Sense, Stroke, TextEdit};

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct GradientStop {
    pub(crate) position: u16,
    pub(crate) color: [u8; 3],
}

pub(crate) const MAX_GRADIENT_STOPS: usize = 16;
pub(crate) const GRADIENT_END: u16 = 4096;
/// Open the native RGB ramp editor from a compact gradient swatch.
pub(crate) fn edit_rgb_stops(
    ui: &mut egui::Ui,
    stops: &mut Vec<GradientStop>,
    reverse: bool,
    sample: impl Fn(f32) -> [u8; 3],
) {
    let title = tr("渐变");
    let id = ui.id().with("rgb_gradient_editor");
    let mut open = ui.ctx().data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    let original_id = id.with("opening_stops");
    let gradient = rgb_gradient(stops, reverse);
    ui.horizontal(|ui| {
        ui.label(title);
        let width = ui.available_width().clamp(32., 112.);
        if swatch(ui, &gradient, width, 20.).on_hover_text(title).clicked() {
            if !open { ui.ctx().data_mut(|data| data.insert_temp(original_id, stops.clone())); }
            open_gradient(ui, id, &mut open);
        }
    });
    let original = ui.ctx().data(|data| data.get_temp::<Vec<GradientStop>>(original_id));
    let was_open = open;
    let confirmed = gradient_window(ui, id, title, &mut open, 280., true, false, |ui| {
        rgb_stop_controls(ui, stops, &gradient, reverse, sample);
        if let Some(original) = &original {
            let preview = rgb_gradient(original, reverse);
            if restore_gradient_row(ui, &preview, stops != original) { stops.clone_from(original); }
        }
    });
    if was_open && !open && !confirmed && let Some(original) = original { *stops = original; }
    ui.ctx().data_mut(|data| {
        if !open { data.remove::<Vec<GradientStop>>(original_id); }
        data.insert_temp(id, open);
    });
}

fn rgb_gradient(stops: &[GradientStop], reverse: bool) -> Gradient {
    let colors = stops.iter().map(|stop| {
        let time = f64::from(stop.position) / f64::from(GRADIENT_END);
        let [r, g, b] = stop.color.map(|channel| f64::from(channel) / 255.);
        (if reverse { 1. - time } else { time }, [r, g, b, 1.])
    });
    let mut colors: Vec<_> = colors.collect();
    if reverse {
        colors.reverse();
    }
    Gradient { colors, alphas: vec![(0., 1.)], mode: 0, color_indices: vec![], alpha_indices: vec![] }
}

fn restore_gradient_row(ui: &mut egui::Ui, original: &Gradient, changed: bool) -> bool {
    ui.horizontal(|ui| {
        swatch(ui, original, 85., 16.).on_hover_text(tr("恢复原始渐变"));
        ui.add_enabled(changed, egui::Button::new(tr("恢复原始渐变"))).clicked()
    }).inner
}

fn open_gradient(ui: &egui::Ui, id: egui::Id, open: &mut bool) {
    *open = true;
    crate::native_window::wake(ui.ctx(), id.with("window"));
}

fn gradient_window(
    ui: &egui::Ui, id: egui::Id, title: &str, open: &mut bool, width: f32, topmost: bool, read_only: bool,
    contents: impl FnOnce(&mut egui::Ui),
) -> bool {
    if !*open {
        return false;
    }
    let mut confirmed = false;
    let mut cancelled = false;
    let window = crate::native_window::Window::new(title).id(id.with("window")).open(open)
        .resizable(false).default_width(width).min_width(width).max_width(width);
    let window = if topmost { window.always_on_top() } else { window };
    window.show(ui.ctx(), |ui| {
        ui.set_width(width);
        if read_only {
            ui.small(tr("只读渐变，不修改资源文件。"));
        }
        contents(ui);
        if !read_only {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let gap = 8.0;
                let button_width = ((ui.available_width() - gap) * 0.5).clamp(0.0, 84.0);
                ui.spacing_mut().item_spacing.x = 0.0;
                let inset = (ui.available_width() - 2.0 * button_width - gap).max(0.0) * 0.5;
                ui.add_space(inset);
                cancelled = ui.add_sized([button_width, 30.0], egui::Button::new(tr("取消"))).clicked();
                ui.add_space(gap);
                confirmed = ui.add_sized([button_width, 30.0], egui::Button::new(tr("确定"))).clicked();
            });
        }
    });
    *open &= !confirmed && !cancelled;
    confirmed
}
fn rgb_stop_controls(
    ui: &mut egui::Ui, stops: &mut Vec<GradientStop>, gradient: &Gradient, reverse: bool, sample: impl Fn(f32) -> [u8; 3],
) {
    let selected_id = ui.id().with("selected_gradient_stop");
    let mut selected = ui
        .ctx()
        .data(|data| data.get_temp::<usize>(selected_id))
        .unwrap_or(0)
        .min(stops.len() - 1);
    let (control, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(260.), 28.),
        egui::Sense::click_and_drag(),
    );
    let rect = egui::Rect::from_min_max(control.min, control.right_bottom() - egui::vec2(0., 12.));
    let pos_x = |position: u16| {
        egui::lerp(
            rect.x_range(),
            f32::from(position) / f32::from(GRADIENT_END),
        )
    };
    let from_x = |x: f32| {
        (((x - rect.left()) / rect.width().max(1.)).clamp(0., 1.) * f32::from(GRADIENT_END)).round()
            as u16
    };
    let display = |position| {
        if reverse {
            GRADIENT_END - position
        } else {
            position
        }
    };
    let mut moved = None;
    if (response.clicked() || response.drag_started())
        && let Some(pointer) = ui
            .input(|i| i.pointer.press_origin())
            .or(response.interact_pointer_pos())
        && let Some((index, _)) = stops.iter().enumerate().min_by(|(_, a), (_, b)| {
            (pos_x(display(a.position)) - pointer.x)
                .abs()
                .total_cmp(&(pos_x(display(b.position)) - pointer.x).abs())
        })
    {
        selected = index;
    }
    if response.dragged()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        moved = Some(display(from_x(pointer.x)));
    }
    paint_swatch(ui, gradient, rect);
    for (index, stop) in stops.iter().enumerate() {
        let [r, g, b] = stop.color;
        let marker = egui::Rect::from_center_size(
            egui::pos2(pos_x(display(stop.position)), rect.bottom() + 6.),
            egui::Vec2::splat(8.),
        );
        ui.painter()
            .rect_filled(marker, 1., egui::Color32::from_rgb(r, g, b));
        let border = if index == selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().widgets.inactive.fg_stroke.color
        };
        ui.painter().rect_stroke(
            marker,
            1.,
            egui::Stroke::new(if index == selected { 2_f32 } else { 1_f32 }, border),
            egui::StrokeKind::Inside,
        );
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(stops.len() < MAX_GRADIENT_STOPS, egui::Button::new("+"))
            .on_hover_text(tr("添加色标"))
            .clicked()
        {
            let mut points = vec![0];
            points.extend(stops.iter().map(|stop| stop.position));
            points.push(GRADIENT_END);
            let gap = points
                .windows(2)
                .max_by_key(|pair| pair[1] - pair[0])
                .unwrap();
            let position = gap[0].midpoint(gap[1]);
            let color = sample(f32::from(position) / f32::from(GRADIENT_END));
            selected = stops.partition_point(|stop| stop.position <= position);
            stops.insert(selected, GradientStop { position, color });
            moved = None;
        }
        if ui
            .add_enabled(stops.len() > 2, egui::Button::new("−"))
            .on_hover_text(tr("删除色标"))
            .clicked()
        {
            stops.remove(selected);
            selected = selected.min(stops.len() - 1);
            moved = None;
        }
        let mut position = f32::from(stops[selected].position) * 100. / f32::from(GRADIENT_END);
        if ui
            .add(
                egui::DragValue::new(&mut position)
                    .range(0. ..=100.)
                    .max_decimals(1)
                    .suffix("%"),
            )
            .changed()
        {
            moved = Some((position * f32::from(GRADIENT_END) / 100.).round() as u16);
        }
        ui.push_id(("gradient_stop_color", selected), |ui| {
            ui.label(tr("颜色"));
            if let Some(color) = crate::color_property::show_editable_rgb(ui, tr("颜色"), stops[selected].color, false) {
                stops[selected].color = color;
            }
        });
    });
    if let Some(position) = moved.filter(|position| *position != stops[selected].position) {
        let mut stop = stops.remove(selected);
        stop.position = position;
        selected = stops.partition_point(|other| other.position <= position);
        stops.insert(selected, stop);
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(selected_id, selected));
}

struct Gradient {
    colors: Vec<(f64, [f64; 4])>,
    alphas: Vec<(f64, f64)>,
    mode: u8,
    color_indices: Vec<usize>,
    alpha_indices: Vec<usize>,
}
pub(crate) struct GradientSampler(Gradient);
impl GradientSampler {
    pub(crate) fn parse(text:&str)->Option<Self>{parse(text).map(Self)}
    pub(crate) fn at(&self,time:f64)->[f64;4]{self.0.sample(time)}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedKey { Color(usize), Alpha(usize) }

fn parse(text: &str) -> Option<Gradient> {
    let fields: HashMap<_, _> = text.lines().filter_map(|line| line.trim().split_once(':')).collect();
    let count = |key: &str| -> Option<usize> {
        let count = fields.get(key)?.trim().parse().ok()?;
        (1..=8).contains(&count).then_some(count)
    };
    let time = |prefix: &str, index: usize| -> Option<f64> {
        let time = fields.get(format!("{prefix}{index}").as_str())?.trim().parse::<u16>().ok()?;
        Some(time as f64 / 65535.0)
    };
    let color = |index: usize| -> Option<[f64; 4]> {
        crate::inline_vector(fields.get(format!("key{index}").as_str())?, &["r", "g", "b", "a"])?
            .try_into().ok()
    };
    let mut colors = Vec::new();
    let mut alphas = Vec::new();
    for index in 0..count("m_NumColorKeys")? { colors.push((time("ctime", index)?, color(index)?)); }
    for index in 0..count("m_NumAlphaKeys")? { alphas.push((time("atime", index)?, color(index)?[3])); }
    let mut color_indices: Vec<_> = (0..colors.len()).collect();
    let mut alpha_indices: Vec<_> = (0..alphas.len()).collect();
    color_indices.sort_by(|a, b| colors[*a].0.total_cmp(&colors[*b].0));
    alpha_indices.sort_by(|a, b| alphas[*a].0.total_cmp(&alphas[*b].0));
    colors = color_indices.iter().map(|index| colors[*index]).collect();
    alphas = alpha_indices.iter().map(|index| alphas[*index]).collect();
    let mode = fields.get("m_Mode")?.trim().parse::<u8>().ok()?;
    // Unknown interpolation modes remain raw rather than showing a false preview.
    if mode > 1 { return None; }
    Some(Gradient { colors, alphas, mode, color_indices, alpha_indices })
}

fn interval<T>(keys: &[(f64, T)], time: f64, fixed: bool) -> (usize, usize, f64) {
    if time <= keys[0].0 { return (0, 0, 0.0); }
    for index in 1..keys.len() {
        if time <= keys[index].0 {
            if fixed { return (index, index, 0.0); }
            let width = keys[index].0 - keys[index - 1].0;
            return (index - 1, index, if width > 0.0 { (time - keys[index - 1].0) / width } else { 1.0 });
        }
    }
    let last = keys.len() - 1;
    (last, last, 0.0)
}

impl Gradient {
    fn sample(&self, time: f64) -> [f64; 4] {
        let (a, b, t) = interval(&self.colors, time, self.mode == 1);
        let mut color = [0.0; 4];
        for (index, channel) in color.iter_mut().enumerate().take(3) {
            *channel = self.colors[a].1[index] * (1.0 - t) + self.colors[b].1[index] * t;
        }
        let (a, b, t) = interval(&self.alphas, time, self.mode == 1);
        color[3] = self.alphas[a].1 * (1.0 - t) + self.alphas[b].1 * t;
        color
    }
}

fn swatch(ui: &mut egui::Ui, gradient: &Gradient, width: f32, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    paint_swatch(ui, gradient, rect);
    response
}

fn paint_swatch(ui: &egui::Ui, gradient: &Gradient, rect: Rect) {
    let painter = ui.painter();
    let tile = 6.0;
    for row in 0..(rect.height() / tile).ceil() as usize {
        for column in 0..(rect.width() / tile).ceil() as usize {
            let cell = Rect::from_min_size(rect.min + egui::vec2(column as f32 * tile, row as f32 * tile), egui::vec2(tile, tile)).intersect(rect);
            painter.rect_filled(cell, 0.0, Color32::from_gray(if (row + column) % 2 == 0 { 195 } else { 115 }));
        }
    }
    painter.add(egui::Shape::mesh(gradient_mesh(gradient, rect)));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, ui.visuals().text_color()), egui::StrokeKind::Inside);
}

fn gradient_mesh(gradient: &Gradient, rect: Rect) -> egui::Mesh {
    // A single mesh has no antialiased internal rectangle borders, which
    // otherwise expose checkerboard seams at fractional display coordinates.
    let mut stops: Vec<f64> = (0..=128).map(|i| i as f64 / 128.0).collect();
    stops.extend(gradient.colors.iter().map(|key| key.0));
    stops.extend(gradient.alphas.iter().map(|key| key.0));
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    let mut mesh = egui::Mesh::default();
    let color = |time| {
        let rgba = gradient.sample(time).map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
        Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
    };
    if gradient.mode == 0 {
        for time in &stops {
            let x = rect.left() + rect.width() * *time as f32;
            let color = color(*time);
            mesh.colored_vertex(egui::pos2(x, rect.top()), color);
            mesh.colored_vertex(egui::pos2(x, rect.bottom()), color);
        }
        for i in 0..stops.len() - 1 {
            let base = i as u32 * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 1, base + 3);
        }
    } else {
        for pair in stops.windows(2) {
            let left = rect.left() + rect.width() * pair[0] as f32;
            let right = rect.left() + rect.width() * pair[1] as f32;
            let color = color((pair[0] + pair[1]) * 0.5);
            let base = mesh.vertices.len() as u32;
            for point in [egui::pos2(left, rect.top()), egui::pos2(left, rect.bottom()), egui::pos2(right, rect.top()), egui::pos2(right, rect.bottom())] {
                mesh.colored_vertex(point, color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 1, base + 3);
        }
    }
    mesh
}

fn key_column(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(egui::vec2(80.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(80.0);
        ui.label(text);
    });
}

pub fn show(ui: &mut egui::Ui, name: &str, value: &str) -> bool {
    let read_only = !crate::editing::active(ui);
    let edited = crate::editing::scope_value(ui);
    let value = edited.as_deref().unwrap_or(value);
    let Some(gradient) = parse(value) else { return false; };
    let id = ui.id().with(("gradient", name));
    let original_id = id.with("opening_gradient");
    let mut open = ui.ctx().data_mut(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    let mut selected = ui.ctx().data_mut(|data| data.get_temp::<SelectedKey>(id.with("selected")))
        .unwrap_or(SelectedKey::Color(0));
    if matches!(selected, SelectedKey::Color(index) if index >= gradient.colors.len())
        || matches!(selected, SelectedKey::Alpha(index) if index >= gradient.alphas.len()) {
        selected = SelectedKey::Color(0);
    }
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(210.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(210.0); ui.add(egui::Label::new(name).truncate());
        });
        if swatch(ui, &gradient, 180.0, 22.0).on_hover_text(if read_only { tr("点击查看渐变关键点（只读）") } else { tr("渐变") }).clicked() {
            if !open && !read_only {
                if let Some(snapshot) = crate::editing::snapshot_scope(ui) {
                    ui.ctx().data_mut(|data| data.insert_temp(original_id, (value.to_owned(), snapshot)));
                }
            }
            open_gradient(ui, id, &mut open);
        }
        if crate::editing::dirty_scope(ui) && ui.small_button(crate::localization::tr("还原")).clicked() { crate::editing::restore_scope(ui); }
    });
    let original = ui.ctx().data(|data| data.get_temp::<(String, crate::editing::ScopeSnapshot)>(original_id));
    let was_open = open;
    let confirmed = gradient_window(ui, id, name, &mut open, 300., false, read_only, |ui| {
            if !read_only && let Some((text, snapshot)) = &original && let Some(parsed) = parse(text) {
                if restore_gradient_row(ui, &parsed, text != value) {
                    crate::editing::restore_scope_snapshot(ui, snapshot);
                    return;
                }
            }
            if !read_only { ui.small(tr("编辑渐变关键点，保存编辑后写入文件。")); }
            ui.label(if gradient.mode == 0 { "Mode: Blend (Classic)" } else { "Mode: Fixed" });
            let content_width = ui.available_width();
            ui.add_space(12.0);
            let preview = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(6.0);
                let preview = swatch(ui, &gradient, content_width - 12.0, 44.0);
                ui.add_space(6.0);
                preview
            }).inner;
            for (index, (time, alpha)) in gradient.alphas.iter().enumerate() {
                let x = egui::lerp(preview.rect.x_range(), *time as f32);
                let hit = Rect::from_center_size(egui::pos2(x, preview.rect.top() - 4.0), egui::vec2(14.0, 12.0));
                if ui.interact(hit, id.with(("alpha_marker", index)), Sense::click())
                    .on_hover_text(format!("Alpha {index} · {:.2}% · {alpha}", time * 100.0)).clicked() {
                    selected = SelectedKey::Alpha(index);
                }
                let gray = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
                ui.painter().add(egui::Shape::convex_polygon(vec![egui::pos2(x - 4.0, preview.rect.top() - 7.0),
                    egui::pos2(x + 4.0, preview.rect.top() - 7.0), egui::pos2(x, preview.rect.top())],
                    Color32::from_gray(gray), Stroke::new(2.0_f32, if selected == SelectedKey::Alpha(index) { Color32::from_rgb(30, 150, 255) } else { ui.visuals().text_color() })));
            }
            for (index, (time, color)) in gradient.colors.iter().enumerate() {
                let x = egui::lerp(preview.rect.x_range(), *time as f32);
                let hit = Rect::from_center_size(egui::pos2(x, preview.rect.bottom() + 4.0), egui::vec2(14.0, 12.0));
                if ui.interact(hit, id.with(("color_marker", index)), Sense::click())
                    .on_hover_text(format!("Color {index} · {:.2}%", time * 100.0)).clicked() {
                    selected = SelectedKey::Color(index);
                }
                let rgb = color.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
                ui.painter().add(egui::Shape::convex_polygon(vec![egui::pos2(x - 4.0, preview.rect.bottom() + 7.0),
                    egui::pos2(x, preview.rect.bottom()), egui::pos2(x + 4.0, preview.rect.bottom() + 7.0)],
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]), Stroke::new(2.0_f32, if selected == SelectedKey::Color(index) { Color32::from_rgb(30, 150, 255) } else { ui.visuals().text_color() })));
            }
            ui.add_space(12.0);
            ui.separator();
            let (label, time) = match selected {
                SelectedKey::Color(index) => (crate::lf!("颜色关键点 {0}", index), gradient.colors[index].0),
                SelectedKey::Alpha(index) => (crate::lf!("Alpha 关键点 {0}", index), gradient.alphas[index].0),
            };
            ui.strong(label);
            let time_key = match selected { SelectedKey::Color(index) => format!("ctime{}", gradient.color_indices[index]), SelectedKey::Alpha(index) => format!("atime{}", gradient.alpha_indices[index]) };
            ui.horizontal(|ui| { key_column(ui, crate::localization::tr("位置（%）")); if !crate::editing::gradient_number(ui, &time_key, false) { readonly_key_value(ui, format!("{:.4}", time * 100.0)); } });
            match selected {
                SelectedKey::Color(index) => {
                    let color = gradient.colors[index].1;
                    ui.horizontal(|ui| {
                        key_column(ui, crate::localization::tr("颜色"));
                        if !crate::editing::gradient_color(ui, &format!("key{}", gradient.color_indices[index])) {
                            crate::color_property::show(ui, &format!("Color {index}"), [color[0], color[1], color[2], 1.0]);
                        }
                    });
                    for (component, (channel, number)) in ["R", "G", "B"].iter().zip(color).enumerate() {
                        ui.horizontal(|ui| { key_column(ui, channel); if !crate::editing::gradient_channel(ui, &format!("key{}", gradient.color_indices[index]), component) { readonly_key_value(ui, number.to_string()); } });
                    }
                }
                SelectedKey::Alpha(index) => {
                    ui.horizontal(|ui| { key_column(ui, crate::localization::tr("Alpha（0–1）")); if !crate::editing::gradient_number(ui, &format!("key{}", gradient.alpha_indices[index]), true) { readonly_key_value(ui, gradient.alphas[index].1.to_string()); } });
                }
            }
            ui.collapsing(crate::localization::tr("全部关键点"), |ui| {
            ui.add_space(8.0);
            ui.strong(crate::localization::tr("颜色关键点"));
            ui.horizontal(|ui| { key_column(ui, crate::localization::tr("位置")); ui.label(crate::localization::tr("颜色")); });
            for (index, (time, color)) in gradient.colors.iter().enumerate() {
                ui.push_id(("color", index), |ui| {
                    ui.horizontal(|ui| {
                        key_column(ui, &format!("{:.2}%", time * 100.0));
                        ui.allocate_ui_with_layout(egui::vec2(120.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_width(120.0);
                            crate::color_property::show(ui, &format!("Color {index}"), *color);
                        });
                    });
                });
            }
            ui.add_space(8.0);
            ui.strong(crate::localization::tr("Alpha 关键点"));
            ui.horizontal(|ui| { key_column(ui, crate::localization::tr("位置")); ui.label(crate::localization::tr("Alpha（0–1）")); });
            for (index, (time, alpha)) in gradient.alphas.iter().enumerate() {
                ui.push_id(("alpha", index), |ui| {
                    ui.horizontal(|ui| {
                        key_column(ui, &format!("{:.2}%", time * 100.0));
                        let mut alpha = alpha.to_string();
                        ui.add_sized([120.0, 22.0], TextEdit::singleline(&mut alpha)
                            .desired_width(f32::INFINITY).min_size(egui::vec2(120.0, 22.0)).interactive(false));
                    });
                });
            }
            });
        });
    if was_open && !open && !confirmed && !read_only && let Some((_, snapshot)) = &original {
        crate::editing::restore_scope_snapshot(ui, snapshot);
    }
    ui.ctx().data_mut(|data| {
        if !open || read_only { data.remove::<(String, crate::editing::ScopeSnapshot)>(original_id); }
        data.insert_temp(id, open);
        data.insert_temp(id.with("selected"), selected);
    });
    true
}

fn readonly_key_value(ui: &mut egui::Ui, mut value: String) {
    ui.add_sized([120.0, 22.0], TextEdit::singleline(&mut value).desired_width(f32::INFINITY)
        .min_size(egui::vec2(120.0, 22.0)).interactive(false));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gradient_uses_independent_color_and_alpha_times() {
        let raw = "key0: {r: 1, g: 0, b: 0, a: 0}\nkey1: {r: 0, g: 0, b: 1, a: 1}\nctime0: 0\nctime1: 65535\natime0: 0\natime1: 32768\nm_Mode: 0\nm_NumColorKeys: 2\nm_NumAlphaKeys: 2";
        let gradient = parse(raw).unwrap();
        let middle = gradient.sample(0.5);
        assert!((middle[0] - 0.5).abs() < 1e-9);
        assert!((middle[2] - 0.5).abs() < 1e-9);
        assert!(middle[3] > 0.999);
        assert_eq!(gradient.sample(1.0), [0.0, 0.0, 1.0, 1.0]);
        assert!(parse(&raw.replace("m_NumColorKeys: 2", "m_NumColorKeys: 9")).is_none());
    }
}
