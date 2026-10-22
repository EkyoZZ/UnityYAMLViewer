//! Static SVG viewer. This intentionally has no access to the bitmap editor.
use std::{io::Read, path::Path, sync::{Arc, mpsc}};
use eframe::egui::{self, Color32};
use resvg::{tiny_skia, usvg::{self, TreeParsing}};
use crate::localization::tr;

const SOURCE_LIMIT: u64 = 16 * 1024 * 1024;
const RASTER_SIDE: f32 = 4096.0;
struct Rendered {
    source: Arc<Vec<u8>>,
    size: egui::Vec2,
    image: egui::ColorImage,
}
fn raster_size(size: egui::Vec2, scale: f32) -> [u32; 2] {
    let scale = scale.max(0.001).min(RASTER_SIDE / size.x.max(size.y));
    [(size.x * scale).round().max(1.0) as u32, (size.y * scale).round().max(1.0) as u32]
}
fn render(source: Arc<Vec<u8>>, scale: f32) -> Result<Rendered, String> {
    let mut options = usvg::Options::default();
    // Static, self-contained documents only: never resolve arbitrary file or URL references.
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let mut tree = usvg::Tree::from_data(&source, &options).map_err(|error| error.to_string())?;
    let size = egui::vec2(tree.size.width(), tree.size.height());
    let [width, height] = raster_size(size, scale);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| tr("SVG 预览分配失败").to_owned())?;
    tree.size = usvg::Size::from_wh(width as f32, height as f32).ok_or_else(|| tr("SVG 尺寸无效").to_owned())?;
    resvg::Tree::from_usvg(&tree).render(Default::default(), &mut pixmap.as_mut());
    let pixels = pixmap.data().chunks_exact(4).map(|pixel| Color32::from_rgba_premultiplied(pixel[0], pixel[1], pixel[2], pixel[3])).collect();
    Ok(Rendered { source, size, image: egui::ColorImage { size: [width as usize, height as usize], pixels } })
}
#[derive(Default)]
pub(super) struct Preview {
    background: String,
    background_rgb: Option<[u8; 3]>,
    source: Option<Arc<Vec<u8>>>,
    size: Option<egui::Vec2>,
    texture: Option<egui::TextureHandle>,
    pending: Option<mpsc::Receiver<Result<Rendered, String>>>,
    error: Option<String>,
    zoom: Option<f32>,
    pan: egui::Vec2,
}
impl Preview {
    fn request(&mut self, path: &Path, scale: f32, ctx: &egui::Context) {
        let path = path.to_owned();
        let source = self.source.clone();
        let ctx = ctx.clone();
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive);
        std::thread::spawn(move || {
            let result = (|| {
                let source = match source {
                    Some(source) => source,
                    None => {
                        let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
                        let mut bytes = Vec::new();
                        file.take(SOURCE_LIMIT + 1).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
                        if bytes.len() as u64 > SOURCE_LIMIT { return Err(tr("SVG 文件超过 16 MiB，无法预览").into()); }
                        Arc::new(bytes)
                    }
                };
                render(source, scale)
            })();
            let _ = send.send(result);
            ctx.request_repaint();
        });
    }
    pub(super) fn show(&mut self, ui: &mut egui::Ui, path: &Path) {
        if let Some(result) = self.pending.as_ref().and_then(|receive| receive.try_recv().ok()) {
            self.pending = None;
            match result {
                Ok(rendered) => {
                    self.source = Some(rendered.source);
                    self.size = Some(rendered.size);
                    self.texture = Some(ui.ctx().load_texture(format!("svg:{}", path.display()), rendered.image, egui::TextureOptions::LINEAR));
                }
                Err(error) => self.error = Some(error),
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("适应窗口")).clicked() { self.zoom = None; self.pan = egui::Vec2::ZERO; }
            for (label, scale) in [("50%", 0.5), ("100%", 1.0), ("200%", 2.0)] {
                if ui.button(label).clicked() { self.zoom = Some(scale); self.pan = egui::Vec2::ZERO; }
            }
            if ui.add_enabled(self.pending.is_none(), egui::Button::new(tr("重载"))).clicked() { let background=self.background.clone(); let background_rgb=self.background_rgb; *self = Self {background, background_rgb, ..Self::default()}; }
            ui.label(tr("背景色"));
            crate::theme::shared_preview_background(ui,&mut self.background,self.background_rgb.get_or_insert([0;3]));
        });
        ui.small(tr("SVG 仅支持只读预览，暂不支持编辑。"))
            .on_hover_text(tr("SVG：文字需转轮廓；暂不加载外部资源或内嵌位图，不支持动画。"));
        if let Some(error) = &self.error { ui.colored_label(ui.visuals().error_fg_color, error); return; }
        if self.pending.is_none() && self.source.is_none() { self.request(path, 1.0, ui.ctx()); }
        let Some(size) = self.size else { ui.spinner(); return; };
        let (canvas, response) = ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0, 1.0)), egui::Sense::drag());
        let mut zoom = self.zoom.unwrap_or_else(|| ((canvas.width() - 16.0).max(1.0) / size.x).min((canvas.height() - 16.0).max(1.0) / size.y).clamp(0.01, 64.0));
        if response.dragged() { self.pan += response.drag_delta(); }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let next = (zoom * (scroll * 0.002).exp()).clamp(0.01, 64.0);
                if let Some(pointer) = response.hover_pos() {
                    let relative = pointer - canvas.center();
                    self.pan = relative - (relative - self.pan) * (next / zoom);
                }
                zoom = next;
                self.zoom = Some(zoom);
            }
        }
        let desired = raster_size(size, zoom * ui.ctx().pixels_per_point());
        if self.pending.is_none() && self.texture.as_ref().is_some_and(|texture| texture.size() != [desired[0] as usize, desired[1] as usize]) {
            self.request(path, zoom * ui.ctx().pixels_per_point(), ui.ctx());
        }
        let painter = ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        let rect = egui::Rect::from_center_size(canvas.center() + self.pan, size * zoom);
        let visible = rect.intersect(canvas).intersect(ui.clip_rect());
        if visible.is_positive() {
            if self.background == "gray" || self.background == "custom" {
                let rgb=if self.background == "gray" {[128;3]} else {self.background_rgb.unwrap_or([0;3])};
                painter.rect_filled(visible,0.0,Color32::from_rgb(rgb[0],rgb[1],rgb[2]));
            } else {
            for y in 0..(visible.height() / 12.0).ceil() as i32 {
                for x in 0..(visible.width() / 12.0).ceil() as i32 {
                    let tile = egui::Rect::from_min_size(visible.min + egui::vec2(x as f32 * 12.0, y as f32 * 12.0), egui::vec2(12.0, 12.0)).intersect(visible);
                    painter.rect_filled(tile, 0.0, Color32::from_gray(if (x+y)%2 == 0 { 190 } else { 230 }));
                }
            }
            }
            if let Some(texture) = &self.texture { painter.image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE); }
        }
        let info = format!("{} × {} · SVG {:.0}%", size.x, size.y, zoom * 100.0);
        let galley = painter.layout_no_wrap(info, egui::FontId::proportional(13.0), Color32::WHITE);
        let badge = egui::Rect::from_center_size(egui::pos2(canvas.center().x, canvas.bottom() - 20.0), galley.size() + egui::vec2(24.0, 12.0));
        painter.rect_filled(badge, 6.0, Color32::from_black_alpha(128));
        painter.galley(badge.min + egui::vec2(12.0, 6.0), galley, Color32::WHITE);
        if let Some(value)=crate::theme::zoom_preset_menu(ui,badge,zoom,badge.right()-12.0){self.zoom=Some(value);self.pan=egui::Vec2::ZERO;}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_viewbox_transparency_and_invalid_input() {
        let source = Arc::new(br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect width="10" height="10" fill="#ff0000" fill-opacity="0.5"/></svg>"##.to_vec());
        let rendered = render(source.clone(), 2.0).unwrap();
        assert_eq!(rendered.size, egui::vec2(20.0, 10.0));
        assert_eq!(rendered.image.size, [40, 20]);
        let pixel = rendered.image.pixels[5 * 40 + 5];
        assert!((127..=128).contains(&pixel.a()));
        assert_eq!(pixel.r(), pixel.a());
        assert_eq!(rendered.image.pixels[5 * 40 + 35].a(), 0);
        assert!(render(Arc::new(b"invalid SVG".to_vec()), 1.0).is_err());
        assert_eq!(raster_size(egui::vec2(100000.0, 50000.0), 64.0), [4096, 2048]);
    }
}
