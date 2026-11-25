use eframe::egui::{self, Color32, Rect, Sense, Stroke, TextEdit};

#[derive(Clone)]
struct Preview {
    original: [f64; 4],
    rgba: [f64; 4],
    hue: f32,
    original_hue: f32,
    mode: u8,
    open: bool,
}

/// Shared RGB band and alpha bar for material and animation previews.
pub fn paint_swatch(ui: &egui::Ui, rect: Rect, rgba: [f64; 4]) {
    paint_alpha_swatch(ui, rect, rgba, 4.0);
}

fn paint_alpha_swatch(ui: &egui::Ui, rect: Rect, rgba: [f64; 4], alpha_height: f32) {
    // The opaque upper band keeps RGB visible even when alpha is zero.
    let rgb = rgb_color(rgba);
    let top = Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - alpha_height));
    ui.painter().rect_filled(top, 0.0, rgb);
    let bottom = Rect::from_min_max(egui::pos2(rect.left(), top.bottom()), rect.max);
    ui.painter().rect_filled(bottom, 0.0, Color32::BLACK);
    ui.painter().rect_filled(Rect::from_min_max(bottom.min,
        egui::pos2(bottom.left() + bottom.width() * rgba[3].clamp(0.0, 1.0) as f32, bottom.bottom())),
        0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.inactive.bg_stroke, egui::StrokeKind::Inside);
}

pub fn show(ui: &mut egui::Ui, name: &str, rgba: [f64; 4]) -> Option<[f64; 4]> {
    show_impl(ui, name, rgba, false, None, true, false)
}

/// Editable drawing color using the shared native palette.
pub fn show_editable(ui: &mut egui::Ui, name: &str, rgba: [f64; 4]) -> Option<[f64; 4]> {
    show_impl(ui, name, rgba, true, None, true, false)
}

pub(crate) fn show_editable_swatch(ui: &mut egui::Ui, name: &str, rgba: [f64; 4], size: egui::Vec2) -> Option<[f64; 4]> {
    show_impl(ui, name, rgba, true, Some(size), true, false)
}

/// Editable RGB palette. A mixed value displays a question mark and requires confirmation.
pub(crate) fn rgb_picker_open(ui: &egui::Ui, name: &str) -> bool {
    let id = ui.id().with(("color_preview", name, false));
    ui.ctx().data(|data| data.get_temp::<Preview>(id)).is_some_and(|state| state.open)
}

pub fn show_editable_rgb(
    ui: &mut egui::Ui,
    name: &str,
    rgb: [u8; 3],
    mixed: bool,
) -> Option<[u8; 3]> {
    let [r, g, b] = rgb.map(|value| f64::from(value) / 255.);
    let color = show_impl(ui, name, [r, g, b, 1.], true, None, false, mixed)?;
    Some([color[0], color[1], color[2]].map(|value| (value.clamp(0., 1.) * 255.).round() as u8))
}
pub fn edit_rgb(ui: &mut egui::Ui, name: &str, color: &mut [u8; 3]) -> bool {
    if let Some(value) = show_editable_rgb(ui, name, *color, false) {
        *color = value;
        true
    } else {
        false
    }
}

pub fn edit_rgba(ui: &mut egui::Ui, name: &str, color: &mut [u8; 4]) -> bool {
    if let Some(value) = show_editable(ui, name, color.map(|v| f64::from(v) / 255.)) {
        *color = value.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
        true
    } else {
        false
    }
}

pub fn edit_color32(ui: &mut egui::Ui, name: &str, color: &mut Color32) {
    let mut rgba = color.to_srgba_unmultiplied();
    if edit_rgba(ui, name, &mut rgba) {
        *color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    }
}

pub fn edit_rgb_float(ui: &mut egui::Ui, name: &str, color: &mut [f32; 3]) -> bool {
    let [r, g, b] = color.map(f64::from);
    if let Some(value) = show_impl(ui, name, [r, g, b, 1.], true, None, false, false) {
        *color = [value[0] as f32, value[1] as f32, value[2] as f32];
        true
    } else {
        false
    }
}

fn show_impl(ui: &mut egui::Ui, name: &str, rgba: [f64; 4], drawing: bool, size: Option<egui::Vec2>, alpha_enabled: bool, mixed: bool) -> Option<[f64; 4]> {
    let id = ui.id().with(("color_preview", name, alpha_enabled));
    let mut state = ui.ctx().data_mut(|data| data.get_temp::<Preview>(id))
        .unwrap_or(Preview { original: rgba, rgba, hue: 0.0, original_hue: 0.0, mode: 0, open: false });
    if drawing && (!mixed || !state.open) {
        if !state.open { state.original = rgba; }
        state.rgba = rgba;
    } else if !state.open && state.original != rgba { state.original = rgba; state.rgba = rgba; }
    let before = state.rgba;
    let available = ui.clip_rect().width() - (ui.cursor().min.x - ui.min_rect().left());
    let size = size.unwrap_or_else(|| egui::vec2(if drawing { 48.0 } else { available.clamp(80.0, 120.0) }, 19.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if alpha_enabled {
        paint_swatch(ui, rect, rgba);
    } else {
        ui.painter().rect_filled(rect, 0., if mixed { Color32::WHITE } else { rgb_color(rgba) });
        ui.painter().rect_stroke(rect, 0., ui.visuals().widgets.inactive.bg_stroke, egui::StrokeKind::Inside);
        if mixed {
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?",
                egui::FontId::proportional(14.), Color32::RED);
        }
    }
    if response.clicked() {
        state.rgba = rgba;
        state.original = rgba;
        picker_hsv(rgba, &mut state.hue);
        state.original_hue = state.hue;
        state.open = true;
        crate::native_window::wake(ui.ctx(), id.with("window"));
    }
    response.on_hover_text(if drawing { name.to_owned() } else { crate::lf!("RGB: {0}, {1}, {2}\nAlpha: {3}\n点击打开调色板（仅预览）", rgba[0], rgba[1], rgba[2], rgba[3]) });
    if !ui.is_enabled() { state.open = false; }
    let editable = drawing || crate::editing::active(ui);
    let mut apply = false;
    if state.open {
        let mut open = true;
        let window = crate::native_window::Window::new(name)
            .id(id.with("window")).open(&mut open).resizable(false)
            .default_width(220.0).min_width(220.0).max_width(220.0);
        let window = if drawing { window.always_on_top() } else { window };
        let mut cancel = false;
        window.show(ui.ctx(), |ui| {
                ui.set_width(220.0);
                if !drawing { ui.small(if crate::editing::active(ui) { crate::localization::tr("编辑颜色，保存编辑后写入文件。") } else { crate::localization::tr("仅预览，不修改属性或资源文件。") }); }
                palette(ui, &mut state.rgba, &mut state.hue);
                egui::ComboBox::from_id_salt("color_units")
                    .selected_text(["RGB 0–1", "RGB 0–255", "HSV"][state.mode as usize])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.mode, 0, "RGB 0–1");
                        ui.selectable_value(&mut state.mode, 1, "RGB 0–255");
                        ui.selectable_value(&mut state.mode, 2, "HSV");
                    });
                if state.mode == 2 {
                    let mut hsv = picker_hsv(state.rgba, &mut state.hue);
                    let mut changed = false;
                    for (label, value) in [("H", &mut hsv.h), ("S", &mut hsv.s), ("V", &mut hsv.v)] {
                        let mut display = *value as f64;
                        if channel_slider(ui, label, &mut display, 1.0, false) {
                            *value = display as f32;
                            changed = true;
                        }
                    }
                    if changed { state.hue = hsv.h; assign_hsv(&mut state.rgba, hsv); }
                } else {
                    let scale = if state.mode == 1 { 255.0 } else { 1.0 };
                    for (axis, value) in ["R", "G", "B"].iter().zip(&mut state.rgba[..3]) {
                        let mut display = *value * scale;
                        if channel_slider(ui, axis, &mut display, scale, state.mode == 1) { *value = display / scale; }
                    }
                }
                if alpha_enabled {
                    let scale = if state.mode == 1 { 255.0 } else { 1.0 };
                    let mut alpha = state.rgba[3] * scale;
                    if channel_slider(ui, "A", &mut alpha, scale, state.mode == 1) { state.rgba[3] = alpha / scale; }
                }
                let [r, g, b, _] = rgb_color(state.rgba).to_array();
                ui.horizontal(|ui| {
                    ui.label("Hexadecimal");
                    let mut hex = format!("{r:02X}{g:02X}{b:02X}");
                    let editable = drawing || crate::editing::active(ui);
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
                let swatch = |ui: &mut egui::Ui, color| {
                    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(18.0), Sense::hover());
                    if alpha_enabled {
                        paint_alpha_swatch(ui, rect, color, 6.0);
                        response.on_hover_text(format!("Alpha {:.1}%", color[3] * 100.0));
                    } else {
                        ui.painter().rect_filled(rect, 0.0, rgb_color(color));
                    }
                };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.small(crate::localization::tr("当前"));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        swatch(ui, state.rgba);
                        swatch(ui, state.original);
                    });
                    ui.small(crate::localization::tr("之前"));
                    if ui.add(egui::Button::new(egui::RichText::new(crate::localization::tr("恢复原始颜色")).small())).clicked() { state.rgba = state.original; state.hue = state.original_hue; }
                });
                if editable {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let width = 86.0;
                        let gap = ui.spacing().item_spacing.x;
                        ui.add_space(((ui.available_width() - 2.0 * width - gap) * 0.5).max(0.0));
                        cancel = ui.add_sized([width, 28.0], egui::Button::new(crate::localization::tr("取消"))).clicked();
                        apply = ui.add_sized([width, 28.0], egui::Button::new(crate::localization::tr("确定"))).clicked();
                    });
                }
            });
        state.open = open && !apply && !cancel;
        if editable && !mixed && (cancel || !open) && !apply {
            state.rgba = state.original;
            state.hue = state.original_hue;
        }
    }
    if !state.rgba.iter().all(|value| value.is_finite() && value.abs() <= f32::MAX as f64) {
        state.rgba = before;
        crate::editing::warning(ui, crate::localization::tr("颜色分量必须为有效浮点数，已恢复之前的颜色。"));
    }
    let changed = (apply || (!mixed && before != state.rgba)).then_some(state.rgba);
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
    changed
}

fn channel_slider(ui: &mut egui::Ui, label: &str, value: &mut f64, maximum: f64, integer: bool) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([12.0, 20.0], egui::Label::new(label));
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = 140.0;
            let slider = egui::Slider::new(value, 0.0..=maximum)
                .clamping(egui::SliderClamping::Edits).trailing_fill(true);
            crate::theme::slider(ui, true, if integer { slider.integer() } else { slider }).changed()
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

fn picker_hsv(rgba: [f64; 4], hue: &mut f32) -> egui::ecolor::Hsva {
    let mut hsv = to_hsv(rgba);
    if hsv.s == 0.0 || hsv.v == 0.0 || (hsv.h == 0.0 && *hue == 1.0) {
        hsv.h = *hue;
    } else {
        *hue = hsv.h;
    }
    hsv
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

fn palette(ui: &mut egui::Ui, rgba: &mut [f64; 4], hue: &mut f32) {
    let mut hsv = picker_hsv(*rgba, hue);
    let hue_rgb = egui::ecolor::Hsva { h: hsv.h, s: 1.0, v: 1.0, a: 1.0 }.to_rgba_unmultiplied();
    let hue_color = rgb_color(hue_rgb.map(f64::from));
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 220.0), Sense::click_and_drag());
    // Layer saturation then transparent-to-black value, for a bilinear SV field.
    gradient(ui, rect, [Color32::WHITE, hue_color, hue_color, Color32::WHITE]);
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
        *hue = hsv.h;
        assign_hsv(rgba, hsv);
    }
    let x = rect.left() + hsv.h * rect.width();
    ui.painter().line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE));
}
