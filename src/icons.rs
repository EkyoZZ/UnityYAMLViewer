//! Reusable SVG assets embedded in the executable; white strokes accept theme tinting.
use eframe::egui;

#[derive(Clone, Copy)]
pub enum Icon { Gear, Eye, Hand, Play, Pause }

pub fn paint(ui: &egui::Ui, icon: Icon, rect: egui::Rect, color: egui::Color32) {
    let source = match icon {
        Icon::Gear => egui::include_image!("../assets/icons/gear.svg"),
        Icon::Eye => egui::include_image!("../assets/icons/eye.svg"),
        Icon::Hand => egui::include_image!("../assets/icons/hand.svg"),
        Icon::Play => egui::include_image!("../assets/icons/play.svg"),
        Icon::Pause => egui::include_image!("../assets/icons/pause.svg"),
    };
    egui::Image::new(source).fit_to_exact_size(rect.size()).tint(color).paint_at(ui, rect);
}
