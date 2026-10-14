//! Reusable SVG assets embedded in the executable; white strokes accept theme tinting.
use eframe::egui;

#[derive(Clone, Copy)]
pub enum Icon { Gear, Eye, Hand, Play, Pause, Stop, TriangleRight, TriangleDown, Search, Close, Minus }

pub fn paint(ui: &egui::Ui, icon: Icon, rect: egui::Rect, color: egui::Color32) {
    let source = match icon {
        Icon::Minus => egui::include_image!("../assets/icons/minus.svg"),
        Icon::Search => egui::include_image!("../assets/icons/search.svg"),
        Icon::Close => egui::include_image!("../assets/icons/close.svg"),
        Icon::Gear => egui::include_image!("../assets/icons/gear.svg"),
        Icon::Eye => egui::include_image!("../assets/icons/eye.svg"),
        Icon::Hand => egui::include_image!("../assets/icons/hand.svg"),
        Icon::Play => egui::include_image!("../assets/icons/play.svg"),
        Icon::Stop => egui::include_image!("../assets/icons/stop.svg"),
        Icon::Pause => egui::include_image!("../assets/icons/pause.svg"),
        Icon::TriangleRight => egui::include_image!("../assets/icons/triangle-right.svg"),
        Icon::TriangleDown => egui::include_image!("../assets/icons/triangle-down.svg"),
    };
    egui::Image::new(source).fit_to_exact_size(rect.size()).tint(color).paint_at(ui, rect);
}

/// Square vector-icon button shared by compact toolbar controls.
pub fn button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    let response = ui.add_sized([28.0, 28.0], egui::Button::new(""));
    paint(ui, icon, response.rect.shrink(7.0), ui.visuals().text_color());
    response.on_hover_text(tooltip)
}
