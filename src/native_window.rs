//! Shared native dialog host. Embedded backends retain the original egui window.
use eframe::egui;

#[derive(Clone, Copy)]
struct MainBounds { physical_rect: egui::Rect }

/// Save the root window's OS bounds before any nested dialog changes the current viewport.
pub fn track_main_window(ctx: &egui::Context) {
    if ctx.viewport_id() != egui::ViewportId::ROOT { return; }
    if let Some(rect) = ctx.input(|input| input.viewport().outer_rect.or(input.viewport().inner_rect)) {
        let scale = ctx.pixels_per_point();
        let physical_rect = egui::Rect::from_min_max((rect.min.to_vec2() * scale).to_pos2(), (rect.max.to_vec2() * scale).to_pos2());
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("native_dialog_main_bounds"), MainBounds { physical_rect }));
    }
}

fn centered_position(parent: egui::Rect, outer_size: egui::Vec2, scale: f32) -> Option<egui::Pos2> {
    if !parent.is_finite() || !outer_size.is_finite() || !scale.is_finite() || scale <= 0.0
        || parent.width() <= 0.0 || parent.height() <= 0.0 { return None; }
    // Physical monitor coordinates also handle negative monitor origins and different DPI scales.
    Some((parent.center().to_vec2() / scale - outer_size * 0.5).to_pos2())
}

#[derive(Clone, Default)]
struct Placement {
    last_pass: Option<u64>,
    pending: bool,
    attempts: u8,
    stable: u8,
    last_size: Option<egui::Vec2>,
    closed: bool,
}

/// An explicit reopen action also restores an existing dialog's placement and focus.
pub fn wake(ctx:&egui::Context,id:egui::Id){
    ctx.data_mut(|data|data.insert_temp(id.with("native_dialog_wake"),true));
    ctx.request_repaint();
}

pub struct Window<'a> {
    title: String,
    id: egui::Id,
    open: Option<&'a mut bool>,
    width: f32,
    height: f32,
    resizable: bool,
    minimum_width: Option<f32>,
    maximum_width: Option<f32>,
    modal: bool,
    fit_height: bool,
    solid_scroll: bool,
    scrollable: bool,
}

impl<'a> Window<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self { id: egui::Id::new(&title), title, open: None, width: 380.0,
            height: 480.0, resizable: true, minimum_width: None, maximum_width: None, modal: false, fit_height: false, solid_scroll: false, scrollable: true }
    }
    pub fn id(mut self, id: egui::Id) -> Self { self.id = id; self }
    pub fn open(mut self, open: &'a mut bool) -> Self { self.open = Some(open); self }
    pub fn default_width(mut self, width: f32) -> Self { self.width = width + 16.0; self }
    pub fn default_height(mut self, height: f32) -> Self { if height > 0.0 { self.height = height + 16.0; } self }
    pub fn min_width(mut self, width: f32) -> Self { self.minimum_width = Some(width + 16.0); self }
    pub fn max_width(mut self, width: f32) -> Self { self.maximum_width = Some(width + 16.0); self }
    pub fn resizable(mut self, value: bool) -> Self { self.resizable = value; self }
    pub fn solid_scroll(mut self) -> Self { self.solid_scroll = true; self }
    pub fn scrollable(mut self, value: bool) -> Self { self.scrollable = value; self }
    pub fn fit_height(mut self) -> Self { self.fit_height = true; self }
    pub fn modal(mut self) -> Self { self.modal = true; self }
    // Native title bars are managed by the OS; fresh dialogs are centered on the main window.
    pub fn collapsible(self, _: bool) -> Self { self }
    pub fn title_bar(self, _: bool) -> Self { self }
    pub fn movable(self, _: bool) -> Self { self }
    pub fn anchor(self, _: egui::Align2, _: egui::Vec2) -> Self { self }
    pub fn fixed_size(mut self, size: egui::Vec2) -> Self {
        self.width = size.x + 16.0; self.height = size.y + 16.0; self.resizable = false; self
    }

    pub fn show(mut self, ctx: &egui::Context, contents: impl FnOnce(&mut egui::Ui)) {
        if self.open.as_deref() == Some(&false) { return; }
        track_main_window(ctx);
        let viewport_id = egui::ViewportId(self.id);
        let state_id = self.id.with("native_dialog_placement");
        let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
        let main = ctx.data(|data| data.get_temp::<MainBounds>(egui::Id::new("native_dialog_main_bounds")));
        let mut placement = ctx.data_mut(|data| data.get_temp::<Placement>(state_id)).unwrap_or_default();
        let exists = ctx.input(|input| input.raw.viewports.contains_key(&viewport_id));
        let requested_wake=ctx.data_mut(|data|data.remove_temp::<bool>(self.id.with("native_dialog_wake"))).unwrap_or(false);
        let fresh = requested_wake || placement.closed || placement.last_pass.is_none_or(|last| pass.saturating_sub(last) > 1) || !exists;
        if fresh { placement = Placement { pending: true, last_pass: Some(pass), ..Default::default() }; }
        placement.last_pass = Some(pass);
        if self.modal && !ctx.embed_viewports() {
            // Keep the parent's original input protection while the native confirmation is open.
            egui::Modal::new(self.id.with("parent_blocker")).frame(egui::Frame::NONE).show(ctx, |_| {});
        }
        let mut open = true;
        let mut contents = Some(contents);
        let mut builder = egui::ViewportBuilder::default().with_title(&self.title)
            .with_inner_size([self.width, self.height]).with_resizable(self.resizable)
            .with_close_button(self.open.is_some()).with_icon(crate::application_icon());
        if let Some(width) = self.minimum_width { builder = builder.with_min_inner_size([width, 120.0]); }
        if let Some(width) = self.maximum_width { builder = builder.with_max_inner_size([width, f32::INFINITY]); }
        if fresh {
            if let Some(position) = main.and_then(|main| centered_position(main.physical_rect, egui::vec2(self.width, self.height), ctx.pixels_per_point())) {
                builder = builder.with_position(position);
            }
        }
        ctx.show_viewport_immediate(viewport_id, builder, |child_ctx, class| {
            let Some(contents) = contents.take() else { return; };
            if class == egui::ViewportClass::Embedded {
                let mut window = egui::Window::new(&self.title).id(self.id)
                    .default_width(self.width - 16.0).resizable(self.resizable);
                if fresh { window = window.current_pos(child_ctx.screen_rect().center() - egui::vec2(self.width, self.height) * 0.5); }
                if self.open.is_some() { window.open(&mut open).show(child_ctx, contents); }
                else { window.show(child_ctx, contents); }
            } else {
                if fresh {
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    child_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    open = false;
                    if self.open.is_none() { child_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose); }
                }
                let mut target_inner = child_ctx.input(|input| input.screen_rect().size());
                let mut resizing = false;
                egui::CentralPanel::default().show(child_ctx, |ui| {
                    if self.solid_scroll { ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid(); }
                    let content_size = if self.scrollable {
                        egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, contents).content_size
                    } else {
                        contents(ui);
                        ui.min_size()
                    };
                    if !self.resizable || self.fit_height {
                        let maximum_height = child_ctx.input(|input| input.viewport().monitor_size)
                            .map_or(800.0, |size| (size.y - 100.0).max(120.0));
                        let height = (content_size.y + 16.0).clamp(80.0, maximum_height);
                        let current_height = child_ctx.input(|input| input.screen_rect().height());
                        if (current_height - height).abs() > 1.0 {
                            target_inner = egui::vec2(if self.resizable { child_ctx.input(|input| input.screen_rect().width()) } else { self.width }, height);
                            resizing = true;
                            child_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_inner));
                        }
                    }
                });
                if placement.pending {
                    placement.attempts = placement.attempts.saturating_add(1);
                    let decoration = child_ctx.input(|input| input.viewport().outer_rect.zip(input.viewport().inner_rect)
                        .map(|(outer, inner)| (outer.size() - inner.size()).max(egui::Vec2::ZERO)));
                    if let Some((main, decoration)) = main.zip(decoration) {
                        let outer_size = target_inner + decoration;
                        if let Some(position) = centered_position(main.physical_rect, outer_size, child_ctx.pixels_per_point()) {
                            child_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
                        }
                        placement.stable = if !resizing && placement.last_size.is_some_and(|last| (last - outer_size).length() < 1.0) {
                            placement.stable.saturating_add(1)
                        } else { 0 };
                        placement.last_size = Some(outer_size);
                    }
                    // Stop positioning once the actual outer frame and fitted height have settled.
                    // Later dragging, resizing or changes to content do not lock the window in place.
                    if placement.stable >= 2 || placement.attempts >= 12 { placement.pending = false; }
                    else { child_ctx.request_repaint(); }
                }
            }
        });
        placement.closed = !open;
        ctx.data_mut(|data| data.insert_temp(state_id, placement));
        if let Some(value) = self.open.as_mut() { **value = open; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aligns_outer_centers_with_negative_monitor_coordinates_and_dpi() {
        let parent = egui::Rect::from_min_size(egui::pos2(-1600.0, 100.0), egui::vec2(1200.0, 900.0));
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let outer = egui::vec2(400.0, 600.0);
            let position = centered_position(parent, outer, scale).unwrap();
            assert!(((position.to_vec2() + outer * 0.5) * scale - parent.center().to_vec2()).length() < 0.01);
        }
        assert!(centered_position(parent, egui::vec2(400.0, 600.0), 0.0).is_none());
    }
    #[test]
    fn fitted_height_keeps_the_same_center() {
        let parent = egui::Rect::from_min_size(egui::pos2(100.0, 200.0), egui::vec2(800.0, 600.0));
        let initial = centered_position(parent, egui::vec2(380.0, 560.0), 1.0).unwrap();
        let fitted = centered_position(parent, egui::vec2(380.0, 700.0), 1.0).unwrap();
        assert_eq!(initial.x, fitted.x);
        assert_eq!(initial.y - fitted.y, 70.0);
    }
}

/// OS file dragging can suppress winit cursor events or send PointerGone.
/// Query client coordinates from the real root HWND instead of a stale egui pointer.
pub(crate) fn track_file_drop(ctx: &egui::Context, frame: &eframe::Frame) {
    let dragging = ctx.input(|input| !input.raw.hovered_files.is_empty() || !input.raw.dropped_files.is_empty());
    let id = egui::Id::new("native_file_drop_position");
    let position = if dragging { file_drop_cursor(ctx, frame) } else { None };
    ctx.data_mut(|data| {
        if let Some(position) = position { data.insert_temp(id, position); }
        else { data.remove::<egui::Pos2>(id); }
    });
    if dragging { ctx.request_repaint_after(std::time::Duration::from_millis(16)); }
}

pub(crate) fn file_drop_position(ctx: &egui::Context) -> Option<egui::Pos2> {
    ctx.data(|data| data.get_temp(egui::Id::new("native_file_drop_position")))
}

#[cfg(windows)]
fn file_drop_cursor(ctx: &egui::Context, frame: &eframe::Frame) -> Option<egui::Pos2> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    #[repr(C)] struct Point { x: i32, y: i32 }
    #[repr(C)] struct ClientRect { left: i32, top: i32, right: i32, bottom: i32 }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetCursorPos(point: *mut Point) -> i32;
        fn ScreenToClient(window: *mut c_void, point: *mut Point) -> i32;
        fn GetClientRect(window: *mut c_void, rect: *mut ClientRect) -> i32;
    }
    let RawWindowHandle::Win32(window) = frame.window_handle().ok()?.as_raw() else { return None; };
    let hwnd = window.hwnd.get() as *mut c_void;
    let mut point = Point { x: 0, y: 0 };
    let mut rect = ClientRect { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe {
        if GetCursorPos(&mut point) == 0 || ScreenToClient(hwnd, &mut point) == 0 || GetClientRect(hwnd, &mut rect) == 0 { return None; }
    }
    client_drop_point([point.x, point.y], [rect.right, rect.bottom], ctx.pixels_per_point())
}

#[cfg(any(windows, test))]
fn client_drop_point(point: [i32; 2], size: [i32; 2], scale: f32) -> Option<egui::Pos2> {
    if !scale.is_finite() || scale <= 0. || point[0] < 0 || point[1] < 0 || point[0] >= size[0] || point[1] >= size[1] { return None; }
    Some(egui::pos2(point[0] as f32 / scale, point[1] as f32 / scale))
}

#[cfg(not(windows))]
fn file_drop_cursor(ctx: &egui::Context, _frame: &eframe::Frame) -> Option<egui::Pos2> {
    ctx.input(|input| input.pointer.latest_pos())
}

#[cfg(test)]
mod file_drop_tests {
    #[test]
    fn client_pixels_convert_at_display_and_ui_scale_without_accepting_outside() {
        assert_eq!(super::client_drop_point([450, 300], [1200, 900], 1.5), Some(eframe::egui::pos2(300., 200.)));
        for point in [[-1, 30], [30, -1], [1200, 30], [30, 900]] {
            assert!(super::client_drop_point(point, [1200, 900], 1.5).is_none());
        }
        assert!(super::client_drop_point([20, 20], [100, 100], 0.).is_none());
    }
}
