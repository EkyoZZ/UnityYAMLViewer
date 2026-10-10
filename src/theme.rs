use eframe::egui::{self, Color32, RichText, Stroke};

const BLUE: Color32 = Color32::from_rgb(0, 127, 255);

// Separate styles let System preference retain egui's live OS theme detection.
pub fn install(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            let dark = theme == egui::Theme::Dark;
            let (panel, surface, input, border, text, selected, hover) = if dark {
                (rgb(46,48,61), rgb(38,40,51), rgb(35,37,47), rgb(69,71,83), rgb(245,246,250), rgb(33,78,129), rgb(52,56,70))
            } else {
                (rgb(245,245,245), Color32::WHITE, Color32::WHITE, rgb(211,214,220), rgb(24,26,31), rgb(207,227,249), rgb(231,240,251))
            };
            let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
            visuals.panel_fill = panel;
            visuals.window_fill = surface;
            visuals.extreme_bg_color = input;
            visuals.faint_bg_color = if dark { rgb(42,44,56) } else { rgb(237,237,237) };
            visuals.override_text_color = Some(text);
            visuals.hyperlink_color = BLUE;
            visuals.selection.bg_fill = selected;
            visuals.selection.stroke = Stroke::new(1.0_f32, if dark { Color32::WHITE } else { text });
            visuals.window_stroke = Stroke::new(1.0_f32, border);
            visuals.window_corner_radius = egui::CornerRadius::same(8);
            for widget in [&mut visuals.widgets.noninteractive, &mut visuals.widgets.inactive, &mut visuals.widgets.hovered, &mut visuals.widgets.active, &mut visuals.widgets.open] {
                widget.bg_fill = surface;
                widget.weak_bg_fill = surface;
                widget.bg_stroke = Stroke::new(1.0_f32, border);
                widget.fg_stroke = Stroke::new(1.0_f32, text);
                widget.corner_radius = egui::CornerRadius::same(4);
            }
            visuals.widgets.noninteractive.bg_fill = panel;
            visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text);
            visuals.widgets.hovered.bg_fill = hover;
            visuals.widgets.hovered.weak_bg_fill = hover;
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BLUE);
            visuals.widgets.active.bg_fill = selected;
            visuals.widgets.active.weak_bg_fill = selected;
            visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, BLUE);
            visuals.widgets.open.bg_fill = selected;
            visuals.widgets.open.weak_bg_fill = selected;
            style.visuals = visuals;
        });
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 { Color32::from_rgb(r,g,b) }

/// Keep zoom slider rails distinct from the surrounding theme surface.
pub fn zoom_slider(ui: &mut egui::Ui, value: &mut f32, maximum: f32) -> egui::Response {
    ui.scope(|ui| {
        let dark = ui.visuals().dark_mode;
        ui.spacing_mut().slider_rail_height = 5.0;
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.bg_fill = if dark { rgb(82,90,108) } else { rgb(205,215,229) };
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.5_f32, BLUE);
        visuals.widgets.hovered.bg_fill = BLUE;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.widgets.active.bg_fill = BLUE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.selection.bg_fill = BLUE;
        ui.add(egui::Slider::new(value,1.0..=maximum).logarithmic(true).suffix("×").trailing_fill(true))
    }).inner
}

pub fn set_preference(ctx: &egui::Context, preference: &str) {
    ctx.set_theme(match preference {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    });
}

/// Joined tabs with a solid blue active segment, matching the reference.
pub fn selector(ui: &mut egui::Ui, preference: &mut String) {
    ui.allocate_ui_with_layout(egui::vec2(192.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (index, (value, label)) in [("light", "浅色"), ("dark", "深色"), ("system", "系统")].into_iter().enumerate() {
            let selected = preference == value;
            let rounding = egui::CornerRadius {
                nw: if index == 0 { 4 } else { 0 }, sw: if index == 0 { 4 } else { 0 },
                ne: if index == 2 { 4 } else { 0 }, se: if index == 2 { 4 } else { 0 },
            };
            let button = egui::Button::new(RichText::new(crate::localization::tr(label)).color(if selected { Color32::WHITE } else { BLUE }))
                .fill(if selected { BLUE } else { ui.visuals().window_fill })
                .stroke(Stroke::new(1.0_f32, BLUE)).corner_radius(rounding);
            let response = ui.add_sized([64.0, 30.0], button);
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, crate::localization::tr(label)));
            if response.clicked() { *preference = value.into(); }
        }
    });
}
