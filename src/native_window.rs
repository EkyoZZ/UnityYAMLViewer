//! Shared native dialog host. Embedded backends retain the original egui window.
use eframe::egui;

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
}

impl<'a> Window<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self { id: egui::Id::new(&title), title, open: None, width: 380.0,
            height: 480.0, resizable: true, minimum_width: None, maximum_width: None, modal: false, fit_height: false }
    }
    pub fn id(mut self, id: egui::Id) -> Self { self.id = id; self }
    pub fn open(mut self, open: &'a mut bool) -> Self { self.open = Some(open); self }
    pub fn default_width(mut self, width: f32) -> Self { self.width = width + 16.0; self }
    pub fn default_height(mut self, height: f32) -> Self { if height > 0.0 { self.height = height + 16.0; } self }
    pub fn min_width(mut self, width: f32) -> Self { self.minimum_width = Some(width + 16.0); self }
    pub fn max_width(mut self, width: f32) -> Self { self.maximum_width = Some(width + 16.0); self }
    pub fn resizable(mut self, value: bool) -> Self { self.resizable = value; self }
    pub fn fit_height(mut self) -> Self { self.fit_height = true; self }
    pub fn modal(mut self) -> Self { self.modal = true; self }
    // Native title bars and window positioning are managed by the OS.
    pub fn collapsible(self, _: bool) -> Self { self }
    pub fn title_bar(self, _: bool) -> Self { self }
    pub fn movable(self, _: bool) -> Self { self }
    pub fn anchor(self, _: egui::Align2, _: egui::Vec2) -> Self { self }
    pub fn fixed_size(mut self, size: egui::Vec2) -> Self {
        self.width = size.x + 16.0; self.height = size.y + 16.0; self.resizable = false; self
    }

    pub fn show(mut self, ctx: &egui::Context, contents: impl FnOnce(&mut egui::Ui)) {
        if self.open.as_deref() == Some(&false) { return; }
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
        ctx.show_viewport_immediate(egui::ViewportId(self.id), builder, |child_ctx, class| {
            let Some(contents) = contents.take() else { return; };
            if class == egui::ViewportClass::Embedded {
                let window = egui::Window::new(&self.title).id(self.id)
                    .default_width(self.width - 16.0).resizable(self.resizable);
                if self.open.is_some() { window.open(&mut open).show(child_ctx, contents); }
                else { window.show(child_ctx, contents); }
            } else {
                if child_ctx.input(|input| input.viewport().close_requested()) {
                    open = false;
                    if self.open.is_none() { child_ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose); }
                }
                egui::CentralPanel::default().show(child_ctx, |ui| {
                    let scroll = egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, contents);
                    if !self.resizable || self.fit_height {
                        let maximum_height = child_ctx.input(|input| input.viewport().monitor_size)
                            .map_or(800.0, |size| (size.y - 100.0).max(120.0));
                        let height = (scroll.content_size.y + 16.0).clamp(80.0, maximum_height);
                        let current_height = child_ctx.input(|input| input.screen_rect().height());
                        if (current_height - height).abs() > 1.0 {
                            child_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(if self.resizable { child_ctx.input(|input| input.screen_rect().width()) } else { self.width }, height)));
                        }
                    }
                });
            }
        });
        if let Some(value) = self.open.as_mut() { **value = open; }
    }
}
