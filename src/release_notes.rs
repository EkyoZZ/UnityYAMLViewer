use std::{collections::HashMap, sync::mpsc};
use eframe::egui;

#[derive(Default)]
pub struct ReleaseNotes { images: HashMap<String, Picture> }
struct Picture {
    receiver: Option<mpsc::Receiver<Result<egui::ColorImage,String>>>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}
impl ReleaseNotes {
    pub fn show(&mut self, ui: &mut egui::Ui, body: &str) {
        static IMAGES: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let pattern=IMAGES.get_or_init(|| regex::Regex::new(r#"(?is)<img\b[^>]*\bsrc\s*=\s*["']([^"']+)["'][^>]*>|!\[[^\]]*\]\((https://[^\s)]+)(?:\s+"[^"]*")?\)"#).unwrap());
        let mut position=0;
        for capture in pattern.captures_iter(body) {
            let matched=capture.get(0).unwrap();
            let text=body[position..matched.start()].trim();
            if !text.is_empty() { ui.add(egui::Label::new(text).wrap()); }
            let url=capture.get(1).or_else(||capture.get(2)).unwrap().as_str().replace("&amp;","&");
            self.picture(ui,&url);
            position=matched.end();
        }
        let text=body[position..].trim();
        if !text.is_empty() { ui.add(egui::Label::new(text).wrap()); }
    }
    fn picture(&mut self, ui: &mut egui::Ui, url: &str) {
        let picture=self.images.entry(url.to_owned()).or_insert_with(|| {
            let (sender,receiver)=mpsc::channel();
            let url=url.to_owned(); let context=ui.ctx().clone();
            std::thread::spawn(move || { let _=sender.send(load(&url)); context.request_repaint(); });
            Picture { receiver:Some(receiver),texture:None,error:None }
        });
        if let Some(receiver)=&picture.receiver {
            if let Ok(result)=receiver.try_recv() {
                match result {
                    Ok(image)=>picture.texture=Some(ui.ctx().load_texture(url,image,egui::TextureOptions::LINEAR)),
                    Err(error)=>picture.error=Some(error),
                }
                picture.receiver=None;
            }
        }
        if let Some(texture)=&picture.texture {
            let size=texture.size_vec2();
            let ratio=(ui.available_width()/size.x).min(1.0);
            ui.add(egui::Image::new(texture).fit_to_exact_size(size*ratio));
        } else if let Some(error)=&picture.error {
            ui.label(crate::localization::tr("更新说明图片加载失败")).on_hover_text(error);
            ui.hyperlink_to(crate::localization::tr("打开图片"),url);
        } else { ui.spinner(); }
    }
}
fn load(url: &str) -> Result<egui::ColorImage,String> {
    use std::io::Read;
    let parsed=reqwest::Url::parse(url).map_err(|error|error.to_string())?;
    if parsed.scheme()!="https" { return Err("Only HTTPS images are supported".into()); }
    let client=reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(30))
        .build().map_err(|error|error.to_string())?;
    let response=client.get(parsed).send().and_then(|response|response.error_for_status()).map_err(|error|error.to_string())?;
    let mut bytes=Vec::new(); response.take(8*1024*1024+1).read_to_end(&mut bytes).map_err(|error|error.to_string())?;
    if bytes.len()>8*1024*1024 { return Err("Image exceeds 8 MiB".into()); }
    let mut reader=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|error|error.to_string())?;
    let mut limits=image::Limits::default(); limits.max_image_width=Some(8192); limits.max_image_height=Some(8192); limits.max_alloc=Some(64*1024*1024);
    reader.limits(limits);
    let decoded=reader.decode().map_err(|error|error.to_string())?.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied([decoded.width() as usize,decoded.height() as usize],decoded.as_raw()))
}
