//! Text and image reference tabs, with nine-slice editing and in-place replacement.
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, mpsc}};
use eframe::egui::{self, Color32};
use egui::emath::GuiRounding as _;
pub(crate) mod nine_slice;
mod seamless;
mod svg_preview;
pub(crate) mod pdf_preview;
pub fn is_pdf(path: &Path) -> bool { extension(path) == "pdf" }

/// Shared by tabs, grouped file rows, and preview file headings.
pub(crate) fn file_context_menu(response:&egui::Response,path:&Path){
    response.context_menu(|ui|{
        if ui.add_enabled(path.is_file(),egui::Button::new(crate::localization::tr("打开文件所在目录"))).clicked(){
            let result=crate::media_tools::command("explorer.exe").arg("/select,").arg(path).spawn();
            if let Err(error)=result{ui.ctx().data_mut(|data|data.insert_temp(response.id.with("file_location_error"),error.to_string()));}
            else{ui.close_menu();}
        }
        if ui.button(crate::localization::tr("复制文件地址")).clicked(){ui.ctx().copy_text(path.to_string_lossy().into_owned());ui.close_menu();}
        if let Some(error)=ui.ctx().data(|data|data.get_temp::<String>(response.id.with("file_location_error"))){ui.colored_label(ui.visuals().error_fg_color,error);}
    });
}

/// egui's painter blends in gamma space, so asset RGB must be premultiplied there.
/// Color32::from_rgba_unmultiplied instead premultiplies in linear space.
pub(crate) fn preview_color([r,g,b,a]: [u8;4]) -> Color32 {
    let multiply = |value: u8| ((value as u16 * a as u16 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(multiply(r),multiply(g),multiply(b),a)
}

pub fn is_bitmap(path: &Path) -> bool {
    matches!(extension(path).as_str(), "png" | "jpg" | "jpeg" | "tga" | "bmp" | "webp")
}
pub fn is_image(path: &Path) -> bool { is_bitmap(path) || extension(path) == "svg" }

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
        assert!(is_image(Path::new("vector.SVG")));
        assert!(!is_text(Path::new("program.exe")));
    }
}
pub fn is_text(path: &Path) -> bool {
    matches!(extension(path).as_str(), "cs" | "txt" | "md" | "json" | "xml" | "shader" | "hlsl" | "cginc" | "compute" | "glsl" | "uxml" | "uss" | "asmdef" | "asmref" | "meta" | "csv" | "tsv" | "psv" | "log" | "ini" | "toml" | "js" | "ts" | "lua" | "py" | "pyw" | "c" | "cpp" | "h" | "hpp" | "rs" | "java" | "html" | "css" | "sh" | "ps1" | "bat" | "cmd")
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
#[derive(Default, PartialEq, Eq)]
enum ImageBackground { #[default] Checkerboard, Gray, Custom }
impl ImageBackground {
    fn controls(&mut self, ui: &mut egui::Ui, color:&mut Option<[u8;3]>) {
        let mut mode=match self {Self::Checkerboard=>"",Self::Gray=>"gray",Self::Custom=>"custom"}.to_owned();
        let mut rgb=color.unwrap_or([0;3]);
        crate::theme::shared_preview_background(ui,&mut mode,&mut rgb);
        *self=match mode.as_str() {"gray"=>Self::Gray,"custom"=>Self::Custom,_=>Self::Checkerboard};
        *color=Some(rgb);
    }
}
#[derive(Default)]
struct ImageTab {
    editor: nine_slice::Editor,
    seamless: seamless::Editor,
    original: Option<Arc<image::RgbaImage>>,
    pending: Option<mpsc::Receiver<Loaded>>,
    texture: Option<egui::TextureHandle>,
    channel: Channel,
    background: ImageBackground,
    background_color: Option<[u8;3]>,
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
    table: crate::delimited_table::Preview,
    playlists: crate::playlist::Lists,
    material_graphs: HashMap<PathBuf, crate::material_graph::Preview>,
    media: HashMap<PathBuf, crate::media_preview::Preview>,
    models: HashMap<PathBuf, crate::model_preview::Preview>,
    images: HashMap<PathBuf, ImageTab>,
    pdfs: HashMap<PathBuf, pdf_preview::Preview>,
    svgs: HashMap<PathBuf, svg_preview::Preview>,
    spine: HashMap<PathBuf, Result<crate::spine_preview::Preview,String>>,

}
impl Assets {
    pub fn show_pdf_fullscreen(&mut self,ui:&mut egui::Ui,path:&Path){self.pdfs.entry(path.to_owned()).or_default().show(ui,path);}
    pub fn retry_pdf_components(&mut self){for preview in self.pdfs.values_mut(){preview.retry_component();}}
    pub fn request_media_playback(&mut self, path: &Path) {
        self.playlists.request(path);
        if let Some(preview) = self.media.get_mut(path) { preview.request_playback(); }
    }
    pub fn retry_media_decoder(&mut self,path:&Path) {
        self.playlists.retry(path);
        if let Some(preview)=self.media.get_mut(path){preview.retry_missing_decoder();}
    }
    pub fn media_fullscreen(&mut self,ctx:&egui::Context,path:Option<&Path>)->bool {
        let video=path.is_some_and(|path|self.playlists.has_video(path)||self.media.get(path).is_some_and(|preview|preview.has_video()));
        let fullscreen=crate::media_preview::is_fullscreen(ctx);
        let owner_id=egui::Id::new("media_fullscreen_path");
        let owner=ctx.data(|data|data.get_temp::<PathBuf>(owner_id));
        let changed=fullscreen && owner.as_deref().is_some_and(|owner|Some(owner)!=path);
        let toggle=video && ctx.input_mut(|input|input.consume_key(egui::Modifiers::NONE,egui::Key::F11));
        let escape=fullscreen && ctx.input_mut(|input|input.consume_key(egui::Modifiers::NONE,egui::Key::Escape));
        if toggle || escape || changed || (fullscreen && !video) {crate::media_preview::set_fullscreen(ctx,video && !fullscreen && !escape && !changed);}
        let fullscreen=crate::media_preview::is_fullscreen(ctx);
        ctx.data_mut(|data|{if fullscreen {if let Some(path)=path{data.insert_temp(owner_id,path.to_owned());}}else{data.remove::<PathBuf>(owner_id);}});
        fullscreen
    }
    pub fn show_media_fullscreen(&mut self,ui:&mut egui::Ui,path:&Path) {
        if crate::playlist::is_playlist(path){self.playlists.show(ui,path,true);return;}
        if let Some(preview)=self.media.get_mut(path){preview.show(ui,path);}
    }
    pub fn invalidate_playlist(&mut self,path:&Path){self.playlists.invalidate(path);}
    pub fn playlist_error(&self,path:&Path)->Option<&str>{self.playlists.error(path)}
    pub fn playlist_dirty(&self,path:&Path)->bool{self.playlists.dirty(path)}
    pub fn playlist_dirty_paths(&self)->Vec<PathBuf>{self.playlists.dirty_paths()}
    pub fn resolve_playlist_dirty(&mut self,path:&Path,save:bool)->bool{self.playlists.resolve_dirty(path,save)}
    pub fn playlist_in_use(&self)->Vec<PathBuf>{self.playlists.in_use()}
    pub fn pause_media(&mut self) {self.playlists.pause();for tab in self.media.values_mut(){tab.deactivate();}}
    pub fn forget(&mut self, path: &Path) { self.material_graphs.remove(path); self.playlists.forget(path); self.media.remove(path);self.images.remove(path); self.spine.remove(path); self.models.remove(path); self.svgs.remove(path); self.pdfs.remove(path); }
    pub fn pause_material_graphs(&mut self){for preview in self.material_graphs.values_mut(){preview.deactivate();}}
    pub fn show_material_graph(&mut self,ui:&mut egui::Ui,path:&Path,text:&str){
        for (other,preview) in &mut self.material_graphs {if other!=path{preview.deactivate();}}
        let catalog=ui.ctx().data_mut(|d|d.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
        if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("mat"))&&catalog.is_none(){ui.spinner();ui.label(crate::localization::tr("正在索引资源"));return;}
        let preview=self.material_graphs.entry(path.to_owned()).or_insert_with(||crate::material_graph::Preview::new(ui.ctx(),path,text,catalog.unwrap_or_default()));
        let reload=ui.push_id(("material_graph_file",path),|ui|preview.show(ui)).inner;
        if reload{self.material_graphs.remove(path);}
    }
    pub fn open_slice_preview(&mut self,path:&Path,request:nine_slice::processing::Request,ctx:&egui::Context)->bool {
        if !is_bitmap(path) { return false; }
        if self.images.get(path).is_some_and(|t|t.editor.open) {crate::native_window::wake(ctx,egui::Id::new(("nine_slice",path)));return true}
        if self.images.values().filter(|t|t.editor.open||t.editor.busy()).count()>=10 {return false}
        let tab=self.images.entry(path.to_owned()).or_default();tab.channel=Channel::Rgba;
        tab.editor.open_preview(request);if tab.original.is_none()&&tab.pending.is_none(){tab.load(path.to_owned(),ctx.clone());}
        crate::native_window::wake(ctx,egui::Id::new(("nine_slice",path)));true
    }
    pub fn show_slice_previews(&mut self,ctx:&egui::Context) {
        for (path,tab) in &mut self.images {
            if !tab.editor.open&&!tab.editor.busy()&&!tab.seamless.active(){
                if let Some(original)=&tab.original {tab.seamless.show(ctx,path,original);}
                continue
            }
            if let Some(message)=nine_slice::Editor::refresh_message(ctx,path) {tab.original=None;tab.texture=None;tab.pending=None;tab.editor.replaced(message);}
            if tab.original.is_none()&&tab.pending.is_none()&&tab.error.is_none(){tab.load(path.clone(),ctx.clone());}
            if let Some(result)=tab.pending.as_ref().and_then(|r|r.try_recv().ok()) {
                tab.pending=None;
                match result {Ok((original,display))=>{tab.original=Some(original);tab.texture=Some(ctx.load_texture(path.display().to_string(),display,egui::TextureOptions::NEAREST));},Err(error)=>{tab.error=Some(error);tab.editor.open=false;tab.seamless=Default::default();}}
            }
            if let (Some(original),Some(texture))=(&tab.original,&tab.texture) {
                tab.seamless.show(ctx,path,original);
                if tab.editor.show(ctx,path,original,texture){tab.original=None;tab.texture=None;tab.pending=None;}
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path, text: &str) {
        self.svgs.retain(|other, _| other == path);
        // Keep decoded pixels and GPU textures only for the active image tab.
        for (other, tab) in &mut self.images {
            if other != path&&!tab.editor.open&&!tab.editor.busy()&&!tab.seamless.active() { tab.original = None; tab.texture = None; tab.pending = None; tab.editor.release(); }
        }
        if crate::playlist::is_playlist(path){self.playlists.show(ui,path,false);return;}
        if crate::media_preview::is_media(path) {
            let preview=self.media.entry(path.to_owned()).or_insert_with(|| crate::media_preview::Preview::new(path));
            self.playlists.show_creator(ui,path,preview);
            return;
        }
        if !is_bitmap(path) && !is_pdf(path) && !matches!(extension(path).as_str(), "csv" | "tsv" | "psv"){file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);}
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
            let refresh=ui.ctx().data_mut(|d|d.remove_temp::<PathBuf>(egui::Id::new("spine_refresh")));
            if refresh.as_deref()==Some(path){self.spine.remove(path);}
            let reload=match self.spine.entry(path.to_owned()).or_insert_with(|| {
                if crate::spine_preview::is_data_asset(text) {crate::spine_preview::Preview::load_asset(ui.ctx(),path,text,catalog.as_ref().unwrap())}
                else {crate::spine_preview::Preview::load(ui.ctx(),path)}
            }) {
                Ok(preview)=>preview.show(ui),
                Err(error)=>{ui.colored_label(Color32::RED,error.as_str());ui.button(crate::localization::tr("重载")).clicked()},
            };
            if reload {self.spine.remove(path);ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("spine_reload_request"),true));}
            return;
        }
        self.spine.clear();
        self.pdfs.retain(|other, _| other == path);
        if is_pdf(path) { self.pdfs.entry(path.to_owned()).or_default().show(ui,path); return; }
        self.svgs.retain(|other, _| other == path);
        if extension(path) == "svg" {
            self.svgs.entry(path.to_owned()).or_default().show(ui, path);
            return;
        }
        if matches!(extension(path).as_str(), "csv" | "tsv" | "psv") {
            ui.push_id(path, |ui| self.table.show(ui, text, match extension(path).as_str() { "tsv" => '\t', "psv" => '|', _ => ',' }));
            return;
        }
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
        let can_open_preview=self.images.values().filter(|t|t.editor.open||t.editor.busy()).count()<10;
        let tab = self.images.entry(path.to_owned()).or_insert_with(|| ImageTab { zoom: Some(1.0), ..Default::default() });
        if let Some(message)=nine_slice::Editor::refresh_message(ui.ctx(),path) {
            tab.original=None;tab.pending=None;tab.texture=None;tab.error=None;
            tab.editor.replaced(message);tab.pan=egui::Vec2::ZERO;tab.zoom=None;
        }
        if tab.pending.is_none() && tab.original.is_none() && tab.error.is_none() { tab.load(path.to_owned(), ui.ctx().clone()); }
        if let Some(result) = tab.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            tab.pending = None;
            match result {
                Ok((original, display)) => {
                    tab.original = Some(original);
                    tab.texture = Some(ui.ctx().load_texture(path.display().to_string(), display, egui::TextureOptions::NEAREST));
                }
                Err(error) => tab.error = Some(error),
            }
        }
        ui.horizontal_wrapped(|ui| {
            file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);
            if ui.add_enabled(!tab.editor.busy(),egui::Button::new(crate::localization::tr("重载"))).clicked() { tab.original = None; tab.editor.reset(); tab.load(path.to_owned(), ui.ctx().clone()); }
            if ui.add_enabled(tab.original.is_some()&&(tab.editor.open||can_open_preview), egui::Button::new(crate::localization::tr("九宫格精简"))).clicked() {
                tab.editor.open = true;
                crate::native_window::wake(ui.ctx(),egui::Id::new(("nine_slice",path)));
                if tab.channel != Channel::Rgba {tab.channel=Channel::Rgba;tab.load(path.to_owned(),ui.ctx().clone());}
            }
            if ui.add_enabled(tab.original.is_some(),egui::Button::new(crate::localization::tr("四方连续图"))).clicked() {
                tab.seamless.open=true;
                crate::native_window::wake(ui.ctx(),egui::Id::new(("seamless",path)));
            }
        });
        ui.horizontal_wrapped(|ui| {
            let previous = tab.channel;
            ui.add_enabled_ui(tab.pending.is_none() && !tab.editor.open && !tab.editor.busy(), |ui| {
                let mut channel=tab.channel.label().to_owned();
                crate::theme::compact_selector(ui,&mut channel,[("R","R"),("G","G"),("B","B"),("A","A"),("RGB","RGB"),("RGBA","RGBA")]);
                tab.channel=match channel.as_str() {"R"=>Channel::R,"G"=>Channel::G,"B"=>Channel::B,"A"=>Channel::A,"RGB"=>Channel::Rgb,_=>Channel::Rgba};
            });
            if previous != tab.channel { tab.load(path.to_owned(), ui.ctx().clone()); }
            if ui.button(crate::localization::tr("适应窗口")).clicked() { tab.zoom = None; tab.pan = egui::Vec2::ZERO; }
            ui.label(crate::localization::tr("背景色"));
            tab.background.controls(ui,&mut tab.background_color);
        });
        let reload=false;
        if reload {tab.original=None;tab.pan=egui::Vec2::ZERO;tab.zoom=None;tab.load(path.to_owned(),ui.ctx().clone());}
        if let Some(error) = &tab.error { ui.label(error); return; }
        let Some(texture) = &tab.texture else { ui.spinner(); return; };
        let size = texture.size_vec2();
        let (canvas, response) = ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)), egui::Sense::drag());
        let mut zoom = tab.zoom.unwrap_or_else(|| {
            ((canvas.width() - 16.0).max(1.0) / size.x).min((canvas.height() - 16.0).max(1.0) / size.y).clamp(0.01,64.0)
        });
        if response.dragged() { tab.pan += ui.input(|input| input.pointer.delta()); }
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
            if tab.background==ImageBackground::Checkerboard {
                // At high zoom, one background square covers exactly one source pixel.
                let aligned = zoom >= 8.0;
                let cell = if aligned { zoom } else { 12.0 };
                let origin = if aligned { rect.min } else { visible.min };
                let first = ((visible.min - origin) / cell).floor();
                let last = ((visible.max - origin) / cell).ceil();
                let mut background = egui::Mesh::default();
                for y in first.y as i32..last.y as i32 {
                    for x in first.x as i32..last.x as i32 {
                        let tile = egui::Rect::from_min_size(origin + egui::vec2(x as f32 * cell, y as f32 * cell), egui::vec2(cell,cell)).intersect(visible);
                        let tile = egui::Rect::from_min_max(
                            tile.min.round_to_pixels(painter.pixels_per_point()),
                            tile.max.round_to_pixels(painter.pixels_per_point()));
                        background.add_colored_rect(tile, Color32::from_gray(if (x+y)%2==0 { 190 } else { 230 }));
                    }
                }
                painter.add(egui::Shape::mesh(background));
            } else {
                let color=if tab.background==ImageBackground::Gray {[128,128,128]}else{tab.background_color.unwrap_or([0;3])};
                let mut background=egui::Mesh::default();
                background.add_colored_rect(visible,Color32::from_rgb(color[0],color[1],color[2]));
                painter.add(egui::Shape::mesh(background));
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
