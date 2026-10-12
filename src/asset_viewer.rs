//! Read-only text and image reference tabs. Image decoding and channel conversion run off-thread.
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, mpsc}};
use eframe::egui::{self, Color32};

/// egui's painter blends in gamma space, so asset RGB must be premultiplied there.
/// Color32::from_rgba_unmultiplied instead premultiplies in linear space.
pub(crate) fn preview_color([r,g,b,a]: [u8;4]) -> Color32 {
    let multiply = |value: u8| ((value as u16 * a as u16 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(multiply(r),multiply(g),multiply(b),a)
}

pub fn is_image(path: &Path) -> bool {
    matches!(extension(path).as_str(), "png" | "jpg" | "jpeg" | "tga" | "bmp" | "webp")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_views_keep_color_under_zero_alpha() {
        let pixel = [123, 45, 67, 0];
        assert_eq!(Channel::R.pixel(pixel), [123,123,123,255]);
        assert_eq!(Channel::G.pixel(pixel), [45,45,45,255]);
        assert_eq!(Channel::B.pixel(pixel), [67,67,67,255]);
        assert_eq!(Channel::A.pixel(pixel), [0,0,0,255]);
        assert_eq!(Channel::Rgb.pixel(pixel), [123,45,67,255]);
        assert_eq!(Channel::Rgba.pixel(pixel), pixel);
        assert_eq!(Channel::A.pixel([1,2,3,255]), [255,255,255,255]);
    }
    #[test]
    fn reference_types_and_syntaxes_include_common_scripts() {
        for extension in ["cs", "lua", "py"] {
            assert!(is_text(Path::new(&format!("example.{extension}"))));
        }
        assert!(is_image(Path::new("texture.JPG")));
        assert!(!is_text(Path::new("program.exe")));
    }
}
pub fn is_text(path: &Path) -> bool {
    matches!(extension(path).as_str(), "cs" | "txt" | "md" | "json" | "xml" | "shader" | "hlsl" | "cginc" | "compute" | "glsl" | "uxml" | "uss" | "asmdef" | "asmref" | "meta" | "csv" | "tsv" | "log" | "ini" | "toml" | "js" | "ts" | "lua" | "py" | "pyw" | "c" | "cpp" | "h" | "hpp" | "rs" | "java" | "html" | "css" | "sh" | "ps1" | "bat" | "cmd")
}
fn extension(path: &Path) -> String { path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase() }

fn syntax_language(path: &Path) -> String {
    if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.to_ascii_lowercase().ends_with(".lua.txt")) {
        "lua".into()
    } else { extension(path) }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Channel { R, G, B, A, Rgb, #[default] Rgba }
impl Channel {
    fn label(self) -> &'static str { match self { Self::R=>"R",Self::G=>"G",Self::B=>"B",Self::A=>"A",Self::Rgb=>"RGB",Self::Rgba=>"RGBA" } }
    fn pixel(self, pixel: [u8; 4]) -> [u8; 4] {
        let [r,g,b,a] = pixel;
        match self {
            Self::R => [r,r,r,255], Self::G => [g,g,g,255],
            Self::B => [b,b,b,255], Self::A => [a,a,a,255],
            Self::Rgb => [r,g,b,255], Self::Rgba => pixel,
        }
    }
}
type Loaded = Result<(Arc<image::RgbaImage>, egui::ColorImage), String>;
#[derive(Default)]
struct ImageTab {
    original: Option<Arc<image::RgbaImage>>,
    pending: Option<mpsc::Receiver<Loaded>>,
    texture: Option<egui::TextureHandle>,
    channel: Channel,
    error: Option<String>,
    zoom: Option<f32>,
    pan: egui::Vec2,
}
impl ImageTab {
    fn load(&mut self, path: PathBuf, ctx: egui::Context) {
        let original = self.original.clone();
        let channel = self.channel;
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive);
        self.error = None;
        self.texture = None;
        std::thread::spawn(move || {
            let result = (|| {
                let original = if let Some(original) = original { original } else {
                    let mut reader = image::ImageReader::open(&path).map_err(|e| e.to_string())?
                        .with_guessed_format().map_err(|e| e.to_string())?;
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(8192); limits.max_image_height = Some(8192);
                    limits.max_alloc = Some(256 * 1024 * 1024); reader.limits(limits);
                    let image = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
                    Arc::new(image)
                };
                let pixels = original.pixels().map(|pixel| {
                    let [r,g,b,a] = channel.pixel(pixel.0);
                    preview_color([r,g,b,a])
                }).collect();
                let display = egui::ColorImage { size: [original.width() as usize, original.height() as usize], pixels };
                Ok((original, display))
            })();
            let _ = send.send(result);
            ctx.request_repaint();
        });
    }
}
#[derive(Default)]
pub struct Assets {
    media: HashMap<PathBuf, crate::media_preview::Preview>,
    models: HashMap<PathBuf, crate::model_preview::Preview>,
    images: HashMap<PathBuf, ImageTab>,
    spine: HashMap<PathBuf, Result<crate::spine_preview::Preview,String>>,

}
impl Assets {
    pub fn pause_media(&mut self) {for tab in self.media.values_mut(){tab.deactivate();}}
    pub fn forget(&mut self, path: &Path) { self.media.remove(path);self.images.remove(path); self.spine.remove(path); self.models.remove(path); }
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path, text: &str) {
        // Keep decoded pixels and GPU textures only for the active image tab.
        for (other, tab) in &mut self.images {
            if other != path { tab.original = None; tab.texture = None; tab.pending = None; }
        }
        if crate::media_preview::is_media(path) {
            let preview=self.media.entry(path.to_owned()).or_default();
            ui.horizontal_wrapped(|ui| {
                ui.heading(path.file_name().unwrap_or_default().to_string_lossy());
                if let Some(bytes)=preview.file_size() {ui.small(if bytes<1048576{format!("({:.1} KiB)",bytes as f64/1024.0)}else{format!("({:.2} MiB)",bytes as f64/1048576.0)});}
                preview.header_controls(ui,path);
            });
            preview.show(ui,path);
            return;
        }
        ui.heading(path.file_name().unwrap_or_default().to_string_lossy());
        self.models.retain(|other,_|other==path);
        if crate::model_preview::is_model(path) {
            let refresh=ui.ctx().data_mut(|d|d.remove_temp::<PathBuf>(egui::Id::new("model_refresh")));
            if refresh.as_deref()==Some(path){self.models.remove(path);}
            let reload=self.models.entry(path.to_owned()).or_insert_with(||crate::model_preview::Preview::new(ui.ctx(),path)).show(ui);
            if reload {self.models.remove(path);}
            return;
        }
        if crate::spine_preview::is_spine(path) || crate::spine_preview::is_data_asset(text) {
            self.spine.retain(|other,_|other==path);
            let catalog=ui.ctx().data_mut(|data|data.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
            if crate::spine_preview::is_data_asset(text) && catalog.is_none() {ui.spinner();ui.label("正在索引 Spine 资源…");return;}
            let reload=match self.spine.entry(path.to_owned()).or_insert_with(|| {
                if crate::spine_preview::is_data_asset(text) {crate::spine_preview::Preview::load_asset(ui.ctx(),text,catalog.as_ref().unwrap())}
                else {crate::spine_preview::Preview::load(ui.ctx(),path)}
            }) {
                Ok(preview)=>preview.show(ui),
                Err(error)=>{ui.colored_label(Color32::RED,error.as_str());ui.button(crate::localization::tr("重载")).clicked()},
            };
            if reload {self.spine.remove(path);ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("spine_reload_request"),true));}
            return;
        }
        self.spine.clear();
        if !is_image(path) {
            egui::ScrollArea::both().id_salt(("reference_text", path)).auto_shrink([false, false]).show(ui, |ui| {
                let language = syntax_language(path);
                if matches!(language.as_str(), "shader" | "hlsl" | "cginc" | "compute" | "glsl") {
                    ui.add(egui::Label::new(shader_layout(ui, text)).selectable(true));
                } else {
                    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
                    egui_extras::syntax_highlighting::code_view_ui(ui, &theme, text, &language);
                }
            });
            return;
        }
        let tab = self.images.entry(path.to_owned()).or_insert_with(|| ImageTab { zoom: Some(1.0), ..Default::default() });
        if tab.pending.is_none() && tab.original.is_none() && tab.error.is_none() { tab.load(path.to_owned(), ui.ctx().clone()); }
        if let Some(result) = tab.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            tab.pending = None;
            match result {
                Ok((original, display)) => {
                    tab.original = Some(original);
                    tab.texture = Some(ui.ctx().load_texture(path.display().to_string(), display, egui::TextureOptions::LINEAR));
                }
                Err(error) => tab.error = Some(error),
            }
        }
        ui.horizontal_wrapped(|ui| {
            let previous = tab.channel;
            ui.add_enabled_ui(tab.pending.is_none(), |ui| {
                for channel in [Channel::R,Channel::G,Channel::B,Channel::A,Channel::Rgb,Channel::Rgba] {
                    ui.selectable_value(&mut tab.channel, channel, channel.label());
                }
            });
            if previous != tab.channel { tab.load(path.to_owned(), ui.ctx().clone()); }
            if ui.button(crate::localization::tr("适应窗口")).clicked() { tab.zoom = None; tab.pan = egui::Vec2::ZERO; }
            for (label, zoom) in [("50%", 0.5), ("100%", 1.0), ("200%", 2.0)] {
                if ui.button(label).clicked() { tab.zoom = Some(zoom); tab.pan = egui::Vec2::ZERO; }
            }
            if ui.button(crate::localization::tr("重载")).clicked() { tab.original = None; tab.load(path.to_owned(), ui.ctx().clone()); }
        });
        if let Some(error) = &tab.error { ui.label(error); return; }
        let Some(texture) = &tab.texture else { ui.spinner(); return; };
        let size = texture.size_vec2();
        let (canvas, response) = ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)), egui::Sense::drag());
        let mut zoom = tab.zoom.unwrap_or_else(|| {
            ((canvas.width() - 16.0).max(1.0) / size.x).min((canvas.height() - 16.0).max(1.0) / size.y).clamp(0.01,64.0)
        });
        if response.dragged() { tab.pan += response.drag_delta(); }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let next = (zoom * (scroll * 0.002).exp()).clamp(0.01,64.0);
                if let Some(pointer) = response.hover_pos() {
                    let relative = pointer - canvas.center();
                    tab.pan = relative - (relative - tab.pan) * (next / zoom);
                }
                zoom = next;
                tab.zoom = Some(zoom);
                ui.ctx().request_repaint();
            }
        }
        let painter = ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        let rect = egui::Rect::from_center_size(canvas.center() + tab.pan, size * zoom);
        let visible = rect.intersect(canvas).intersect(ui.clip_rect());
        if visible.is_positive() {
            let cell = 12.0;
            for y in 0..(visible.height() / cell).ceil() as i32 {
                for x in 0..(visible.width() / cell).ceil() as i32 {
                    let tile = egui::Rect::from_min_size(visible.min + egui::vec2(x as f32 * cell, y as f32 * cell), egui::vec2(cell,cell)).intersect(visible);
                    painter.rect_filled(tile, 0.0, Color32::from_gray(if (x+y)%2==0 { 190 } else { 230 }));
                }
            }
            painter.image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0,1.0)), Color32::WHITE);
        }
        let info = format!("{} × {} · {} {:.0}%", size.x as usize, size.y as usize, tab.channel.label(), zoom * 100.0);
        let galley = painter.layout_no_wrap(info, egui::FontId::proportional(13.0), Color32::WHITE);
        let badge_size = galley.size() + egui::vec2(24.0,12.0);
        let badge = egui::Rect::from_center_size(egui::pos2(canvas.center().x, canvas.bottom() - 8.0 - badge_size.y * 0.5), badge_size);
        painter.rect_filled(badge, 6.0, Color32::from_black_alpha(128));
        painter.galley(badge.min + egui::vec2(12.0,6.0), galley, Color32::WHITE);
    }
}

fn shader_layout(ui: &egui::Ui, text: &str) -> egui::text::LayoutJob {
    static TOKENS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tokens = TOKENS.get_or_init(|| regex::Regex::new(r#"(?s:/\*.*?\*/)|//[^\n]*|"(?:\\.|[^"\\])*"|#[ \t]*[A-Za-z_]+|\b[A-Za-z_][A-Za-z_0-9]*\b|\b(?:0[xX][0-9a-fA-F]+|\d+(?:\.\d*)?(?:[eE][+-]?\d+)?[fFhHuU]?)\b"#).unwrap());
    let dark = ui.visuals().dark_mode;
    let keyword = if dark { Color32::from_rgb(197,134,192) } else { Color32::from_rgb(128,32,145) };
    let string = if dark { Color32::from_rgb(206,145,120) } else { Color32::from_rgb(160,65,25) };
    let comment = if dark { Color32::from_rgb(106,153,85) } else { Color32::from_rgb(65,120,50) };
    let number = if dark { Color32::from_rgb(181,206,168) } else { Color32::from_rgb(65,115,60) };
    let kind = if dark { Color32::from_rgb(78,201,176) } else { Color32::from_rgb(0,115,125) };
    let mut job = egui::text::LayoutJob::default();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let mut append = |value: &str, color| job.append(value, 0.0, egui::TextFormat { font_id: font.clone(), color, ..Default::default() });
    let mut end = 0;
    for token in tokens.find_iter(text) {
        append(&text[end..token.start()], ui.visuals().text_color());
        let value = token.as_str();
        let color = if value.starts_with("//") || value.starts_with("/*") { comment }
            else if value.starts_with('"') { string }
            else if value.starts_with('#') || matches!(value,
                "Shader"|"Properties"|"SubShader"|"Pass"|"Tags"|"LOD"|"Fallback"|"FallBack"|"CustomEditor"|"UsePass"|"GrabPass"|
                "CGPROGRAM"|"ENDCG"|"CGINCLUDE"|"HLSLPROGRAM"|"ENDHLSL"|"HLSLINCLUDE"|
                "Cull"|"ZWrite"|"ZTest"|"Blend"|"BlendOp"|"ColorMask"|"Offset"|"Stencil"|"Ref"|"Comp"|"Fail"|"ZFail"|"ReadMask"|"WriteMask"|
                "Lighting"|"Fog"|"Name"|"On"|"Off"|"Front"|"Back"|"Always"|"Equal"|"NotEqual"|"Less"|"LEqual"|"Greater"|"GEqual"|"Keep"|"Replace"|
                "struct"|"return"|"if"|"else"|"for"|"while"|"do"|"break"|"continue"|"discard"|"const"|"static"|"uniform"|"in"|"out"|"inout"|"true"|"false") { keyword }
            else if value.as_bytes().first().is_some_and(u8::is_ascii_digit) { number }
            else if matches!(value, "Float"|"Range"|"Int"|"Integer"|"Color"|"Vector"|"void"|"sampler2D"|"samplerCUBE"|"Texture2D"|"TextureCube"|"SamplerState")
                || ["float","half","fixed","int","uint","bool","vec","mat"].iter().any(|prefix| value.strip_prefix(prefix).is_some_and(|suffix| suffix.chars().all(|c| c.is_ascii_digit() || c=='x'))) { kind }
            else { ui.visuals().text_color() };
        append(value, color);
        end = token.end();
    }
    append(&text[end..], ui.visuals().text_color());
    job
}
