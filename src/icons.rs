//! Reusable SVG assets embedded in the executable; white strokes accept theme tinting.
use eframe::egui;

#[derive(Clone, Copy)]
pub enum Icon { Gear, BackgroundTasks, EditList, Eye, Hand, Play, Pause, Stop, Next, Previous, TrimPlay, MarkStart, MarkEnd, ChevronLeft, ChevronRight, TriangleRight, TriangleDown, Search, Close, Minus, TabGroup }

pub fn paint(ui: &egui::Ui, icon: Icon, rect: egui::Rect, color: egui::Color32) {
    // Keep Next's triangle and bar at their original scale; TrimPlay adds a bar.
    let rect = if matches!(icon, Icon::TrimPlay) {
        egui::Rect::from_center_size(rect.center(), egui::vec2(rect.height()*28.0/24.0, rect.height()))
    } else { rect };
    let source = match icon {
        Icon::TabGroup => egui::include_image!("../assets/icons/tab-group.svg"),
        Icon::Minus => egui::include_image!("../assets/icons/minus.svg"),
        Icon::Search => egui::include_image!("../assets/icons/search.svg"),
        Icon::Close => egui::include_image!("../assets/icons/close.svg"),
        Icon::EditList => egui::include_image!("../assets/icons/edit-list.svg"),
        Icon::BackgroundTasks => egui::include_image!("../assets/icons/background-tasks.svg"),
        Icon::Gear => egui::include_image!("../assets/icons/gear.svg"),
        Icon::Eye => egui::include_image!("../assets/icons/eye.svg"),
        Icon::Hand => egui::include_image!("../assets/icons/hand.svg"),
        Icon::Play => egui::include_image!("../assets/icons/play.svg"),
        Icon::ChevronLeft => egui::include_image!("../assets/icons/chevron-left.svg"),
        Icon::ChevronRight => egui::include_image!("../assets/icons/chevron-right.svg"),
        Icon::Next | Icon::Previous => egui::include_image!("../assets/icons/next.svg"),
        Icon::TrimPlay => egui::include_image!("../assets/icons/trim-play.svg"),
        Icon::MarkStart => egui::include_image!("../assets/icons/mark-start.svg"),
        Icon::MarkEnd => egui::include_image!("../assets/icons/mark-end.svg"),
        Icon::Stop => egui::include_image!("../assets/icons/stop.svg"),
        Icon::Pause => egui::include_image!("../assets/icons/pause.svg"),
        Icon::TriangleRight => egui::include_image!("../assets/icons/triangle-right.svg"),
        Icon::TriangleDown => egui::include_image!("../assets/icons/triangle-down.svg"),
    };
    let mut image=egui::Image::new(source).fit_to_exact_size(rect.size()).tint(color);
    if matches!(icon,Icon::Previous) {image=image.uv(egui::Rect::from_min_max(egui::pos2(1.0,0.0),egui::pos2(0.0,1.0)));}
    image.paint_at(ui,rect);
}

/// Square vector-icon button shared by compact toolbar controls.
pub fn button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    let response = ui.add_sized([28.0, 28.0], egui::Button::new(""));
    paint(ui, icon, response.rect.shrink(7.0), ui.visuals().text_color());
    response.on_hover_text(tooltip)
}
