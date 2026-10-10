use eframe::egui::{self, Color32, Rect, Sense, Stroke, TextEdit};

#[derive(Clone)]
struct Preview {
    original: [f64; 4],
    rgba: [f64; 4],
    mode: u8,
    open: bool,
}

/// Shared RGB band and alpha bar for material and animation previews.
pub fn paint_swatch(ui: &egui::Ui, rect: Rect, rgba: [f64; 4]) {
    // The opaque upper band keeps RGB visible even when alpha is zero.
    let rgb = rgb_color(rgba);
    let top = Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - 4.0));
    ui.painter().rect_filled(top, 0.0, rgb);
    let bottom = Rect::from_min_max(egui::pos2(rect.left(), top.bottom()), rect.max);
    ui.painter().rect_filled(bottom, 0.0, Color32::BLACK);
    ui.painter().rect_filled(Rect::from_min_max(bottom.min,
        egui::pos2(bottom.left() + bottom.width() * rgba[3].clamp(0.0, 1.0) as f32, bottom.bottom())),
        0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.inactive.bg_stroke, egui::StrokeKind::Inside);
}

pub fn show(ui: &mut egui::Ui, name: &str, rgba: [f64; 4]) -> Option<[f64; 4]> {
    let id = ui.id().with(("color_preview", name));
    let mut state = ui.ctx().data_mut(|data| data.get_temp::<Preview>(id))
        .unwrap_or(Preview { original: rgba, rgba, mode: 0, open: false });
    if state.original != rgba { state.original = rgba; state.rgba = rgba; }
    let before = state.rgba;
    let available = ui.clip_rect().width() - (ui.cursor().min.x - ui.min_rect().left());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(available.clamp(80.0, 120.0), 19.0), Sense::click());
    paint_swatch(ui, rect, rgba);
    if response.clicked() {
        state.rgba = rgba;
        state.open = true;
    }
    response.on_hover_text(crate::lf!("RGB: {0}, {1}, {2}\nAlpha: {3}\n点击打开调色板（仅预览）", rgba[0], rgba[1], rgba[2], rgba[3]));
    if state.open {
        let mut open = true;
        egui::Window::new(name)
            .id(id.with("window")).open(&mut open).resizable(false)
            .default_width(220.0).min_width(220.0).max_width(220.0)
            .show(ui.ctx(), |ui| {
                ui.set_width(220.0);
                ui.small(if crate::editing::active(ui) { crate::localization::tr("编辑颜色，保存编辑后写入文件。") } else { crate::localization::tr("仅预览，不修改属性或资源文件。") });
                palette(ui, &mut state.rgba);
                egui::ComboBox::from_id_salt("color_units")
                    .selected_text(["RGB 0–1", "RGB 0–255", "HSV"][state.mode as usize])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.mode, 0, "RGB 0–1");
                        ui.selectable_value(&mut state.mode, 1, "RGB 0–255");
                        ui.selectable_value(&mut state.mode, 2, "HSV");
                    });
                if state.mode == 2 {
                    let mut hsv = to_hsv(state.rgba);
                    let mut changed = false;
                    for (label, value) in [("H", &mut hsv.h), ("S", &mut hsv.s), ("V", &mut hsv.v)] {
                        let mut display = *value as f64;
                        if channel_slider(ui, label, &mut display, 1.0, false) {
                            *value = display as f32;
                            changed = true;
                        }
                    }
                    if changed { assign_hsv(&mut state.rgba, hsv); }
                } else {
                    let scale = if state.mode == 1 { 255.0 } else { 1.0 };
                    for (axis, value) in ["R", "G", "B"].iter().zip(&mut state.rgba[..3]) {
                        let mut display = *value * scale;
                        if channel_slider(ui, axis, &mut display, scale, state.mode == 1) { *value = display / scale; }
                    }
                }
                let scale = if state.mode == 1 { 255.0 } else { 1.0 };
                let mut alpha = state.rgba[3] * scale;
                if channel_slider(ui, "A", &mut alpha, scale, state.mode == 1) { state.rgba[3] = alpha / scale; }
                let [r, g, b, _] = rgb_color(state.rgba).to_array();
                ui.horizontal(|ui| {
                    ui.label("Hexadecimal");
                    let mut hex = format!("{r:02X}{g:02X}{b:02X}");
                    let editable = crate::editing::active(ui);
                    let hex_id = ui.id().with("hex_draft");
                    if editable { if let Some(draft) = ui.ctx().data(|data| data.get_temp::<String>(hex_id)) { hex = draft; } }
                    let response = ui.add(TextEdit::singleline(&mut hex).desired_width(90.0).interactive(editable));
                    if editable && response.changed() { ui.ctx().data_mut(|data| data.insert_temp(hex_id, hex.clone())); }
                    if editable && (response.lost_focus() || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))) {
                        ui.ctx().data_mut(|data| data.remove::<String>(hex_id));
                        if let Some(value) = hex.strip_prefix('#').unwrap_or(&hex).get(..).filter(|value| value.len() == 6).and_then(|value| u32::from_str_radix(value, 16).ok()) {
                            state.rgba[0] = ((value >> 16) & 255) as f64 / 255.0;
                            state.rgba[1] = ((value >> 8) & 255) as f64 / 255.0;
                            state.rgba[2] = (value & 255) as f64 / 255.0;
                        } else { crate::editing::warning(ui, crate::localization::tr("颜色十六进制值需要 6 位（例如 FFFFFF），已恢复之前的颜色。")); }
                    }
                });
                if state.original.iter().any(|value| *value < 0.0 || *value > 1.0) {
                    ui.small(crate::localization::tr("原始颜色包含 HDR 或超出 0–1 的值；色块和调色区域仅显示 0–1 范围。"));
                }
                if ui.button(crate::localization::tr("恢复原始颜色")).clicked() { state.rgba = state.original; }
            });
        state.open = open;
    }
    if !state.rgba.iter().all(|value| value.is_finite() && value.abs() <= f32::MAX as f64) {
        state.rgba = before;
        crate::editing::warning(ui, crate::localization::tr("颜色分量必须为有效浮点数，已恢复之前的颜色。"));
    }
    let changed = (before != state.rgba).then_some(state.rgba);
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    changed
}

fn channel_slider(ui: &mut egui::Ui, label: &str, value: &mut f64, maximum: f64, integer: bool) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([12.0, 20.0], egui::Label::new(label));
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = 140.0;
            ui.spacing_mut().slider_rail_height = 6.0;
            let dark = ui.visuals().dark_mode;
            let visuals = ui.visuals_mut();
            visuals.widgets.inactive.bg_fill = if dark { Color32::from_gray(90) } else { Color32::from_gray(150) };
            for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.hovered, &mut visuals.widgets.active] {
                widget.fg_stroke = Stroke::new(1.5_f32, if dark { Color32::WHITE } else { Color32::BLACK });
            }
            let slider = egui::Slider::new(value, 0.0..=maximum)
                .clamping(egui::SliderClamping::Edits).trailing_fill(true);
            ui.add(if integer { slider.integer() } else { slider }).changed()
        }).inner
    }).inner
}

fn rgb_color(rgba: [f64; 4]) -> Color32 {
    Color32::from_rgb((rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn to_hsv(rgba: [f64; 4]) -> egui::ecolor::Hsva {
    // Use the same RGB coordinates for display, numeric values and the palette;
    // do not gamma-convert the serialized channel values.
    egui::ecolor::Hsva::from_rgba_unmultiplied(rgba[0].clamp(0.0, 1.0) as f32,
        rgba[1].clamp(0.0, 1.0) as f32, rgba[2].clamp(0.0, 1.0) as f32, 1.0)
}

fn assign_hsv(rgba: &mut [f64; 4], hsv: egui::ecolor::Hsva) {
    let converted = hsv.to_rgba_unmultiplied();
    for i in 0..3 { rgba[i] = converted[i] as f64; }
}

fn gradient(ui: &egui::Ui, rect: Rect, colors: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    for (position, color) in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].into_iter().zip(colors) {
        mesh.colored_vertex(position, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}

fn palette(ui: &mut egui::Ui, rgba: &mut [f64; 4]) {
    let mut hsv = to_hsv(*rgba);
    let hue_rgb = egui::ecolor::Hsva { h: hsv.h, s: 1.0, v: 1.0, a: 1.0 }.to_rgba_unmultiplied();
    let hue = rgb_color(hue_rgb.map(f64::from));
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 220.0), Sense::click_and_drag());
    // Layer saturation then transparent-to-black value, for a bilinear SV field.
    gradient(ui, rect, [Color32::WHITE, hue, hue, Color32::WHITE]);
    gradient(ui, rect, [Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK]);
    if let Some(pointer) = response.interact_pointer_pos() {
        hsv.s = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsv.v = (1.0 - (pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        assign_hsv(rgba, hsv);
    }
    let marker = egui::pos2(rect.left() + hsv.s * rect.width(), rect.top() + (1.0 - hsv.v) * rect.height());
    ui.painter().circle_stroke(marker, 5.0, Stroke::new(2.0_f32, Color32::BLACK));
    ui.painter().circle_stroke(marker, 3.0, Stroke::new(1.0_f32, Color32::WHITE));
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 16.0), Sense::click_and_drag());
    let colors = [Color32::RED, Color32::YELLOW, Color32::GREEN, Color32::CYAN, Color32::BLUE, Color32::MAGENTA, Color32::RED];
    for i in 0..6 {
        let segment = Rect::from_min_max(egui::pos2(rect.left() + rect.width() * i as f32 / 6.0, rect.top()),
            egui::pos2(rect.left() + rect.width() * (i + 1) as f32 / 6.0, rect.bottom()));
        gradient(ui, segment, [colors[i], colors[i + 1], colors[i + 1], colors[i]]);
    }
    if let Some(pointer) = response.interact_pointer_pos() {
        hsv.h = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        assign_hsv(rgba, hsv);
    }
    let x = rect.left() + hsv.h * rect.width();
    ui.painter().line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
}
