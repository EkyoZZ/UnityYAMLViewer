use std::collections::HashMap;
use eframe::egui::{self, Color32, Rect, Sense, Stroke, TextEdit};

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
    let painter = ui.painter();
    let tile = 6.0;
    for row in 0..(height / tile).ceil() as usize {
        for column in 0..(width / tile).ceil() as usize {
            let cell = Rect::from_min_size(rect.min + egui::vec2(column as f32 * tile, row as f32 * tile), egui::vec2(tile, tile)).intersect(rect);
            painter.rect_filled(cell, 0.0, Color32::from_gray(if (row + column) % 2 == 0 { 195 } else { 115 }));
        }
    }
    painter.add(egui::Shape::mesh(gradient_mesh(gradient, rect)));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, ui.visuals().text_color()), egui::StrokeKind::Inside);
    response
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
    let edited = crate::editing::scope_value(ui);
    let value = edited.as_deref().unwrap_or(value);
    let Some(gradient) = parse(value) else { return false; };
    let id = ui.id().with(("gradient", name));
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
        ui.allocate_ui_with_layout(egui::vec2(100.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(100.0); ui.weak("Gradient");
        });
        if swatch(ui, &gradient, 180.0, 22.0).on_hover_text(crate::localization::tr("点击查看渐变关键点（只读）")).clicked() { open = true; }
        if crate::editing::dirty_scope(ui) && ui.small_button(crate::localization::tr("还原")).clicked() { crate::editing::restore_scope(ui); }
    });
    if open {
        crate::native_window::Window::new(name).id(id.with("window")).open(&mut open).resizable(false)
            .default_width(300.0).min_width(300.0).max_width(300.0).show(ui.ctx(), |ui| {
            ui.set_width(300.0);
            ui.small(if crate::editing::active(ui) { crate::localization::tr("编辑渐变关键点，保存编辑后写入文件。") } else { crate::localization::tr("只读渐变，不修改资源文件。") });
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
                let x = preview.rect.left() + preview.rect.width() * *time as f32;
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
                let x = preview.rect.left() + preview.rect.width() * *time as f32;
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
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, open));
    ui.ctx().data_mut(|data| data.insert_temp(id.with("selected"), selected));
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
