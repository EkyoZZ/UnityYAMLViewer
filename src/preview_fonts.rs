//! Resolve uGUI fonts and TMP source-font references, keeping system fonts as fallbacks.
use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Arc, mpsc}};
use eframe::egui;

type Catalog = Arc<HashMap<String, PathBuf>>;
enum Entry { Loading(mpsc::Receiver<Result<Vec<u8>, String>>), Ready(String), Failed }
#[derive(Default)]
pub struct Fonts { entries: HashMap<PathBuf, Entry>, bytes: usize, registered: usize }

pub fn reference(text: &str, tmp: bool) -> Option<String> {
    let key = if tmp { "m_fontAsset" } else { "m_Font" };
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix(key)?.strip_prefix(':')?.trim();
        crate::static_preview::reference(value, "guid").filter(|guid| guid.chars().any(|c| c != '0'))
    })
}

fn source(path: &Path, catalog: &HashMap<String, PathBuf>, visited: &mut HashSet<PathBuf>) -> Option<PathBuf> {
    if visited.len() >= 4 || !visited.insert(path.to_owned()) { return None; }
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if matches!(extension.as_str(), "ttf" | "otf" | "ttc") { return Some(path.to_owned()); }
    if extension != "asset" || std::fs::metadata(path).ok()?.len() > 16 * 1024 * 1024 { return None; }
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once(':') else { continue; };
        let guid = match key {
            "m_SourceFontFile" | "m_sourceFontFile" => crate::static_preview::reference(value.trim(), "guid"),
            "m_SourceFontFileGUID" | "m_sourceFontFileGUID" => Some(value.trim().trim_matches('"').to_owned()),
            _ => None,
        };
        if let Some(found) = guid.and_then(|guid| catalog.get(&guid)).and_then(|path| source(path, catalog, visited)) { return Some(found); }
    }
    None
}

impl Fonts {
    pub fn family(&mut self, ctx: &egui::Context, guid: Option<String>, catalog: Option<&Catalog>) -> egui::FontFamily {
        let fallback = egui::FontFamily::Proportional;
        let Some(catalog) = catalog else { return fallback; };
        let Some(path) = guid.and_then(|guid| catalog.get(&guid)).cloned() else { return fallback; };
        if !self.entries.contains_key(&path) {
            if self.entries.len() >= 32 || self.bytes >= 64 * 1024 * 1024 { return fallback; }
            let (send, receive) = mpsc::channel();
            self.entries.insert(path.clone(), Entry::Loading(receive));
            let catalog = catalog.clone(); let ctx = ctx.clone(); let path = path.clone();
            std::thread::spawn(move || {
                let result = (|| {
                    let path = source(&path, &catalog, &mut HashSet::new()).ok_or("Font source missing")?;
                    if std::fs::metadata(&path).map_err(|e|e.to_string())?.len() > 16 * 1024 * 1024 { return Err("Font too large".into()); }
                    let bytes = std::fs::read(path).map_err(|e|e.to_string())?;
                    ab_glyph::FontArc::try_from_vec(bytes.clone()).map_err(|_| "Unsupported font".to_owned())?;
                    Ok(bytes)
                })();
                let _ = send.send(result); ctx.request_repaint();
            });
        }
        let result = match self.entries.get(&path) { Some(Entry::Loading(receive)) => receive.try_recv().ok(), _ => None };
        if let Some(result) = result {
            match result {
                Ok(bytes) if self.bytes + bytes.len() <= 64 * 1024 * 1024 => {
                    let name = format!("preview_font_{}", self.registered);
                    self.registered += 1;
                    let mut definitions = ctx.data(|data| data.get_temp::<egui::FontDefinitions>(egui::Id::new("viewer_font_definitions"))).unwrap_or_default();
                    self.bytes += bytes.len();
                    definitions.font_data.insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
                    let mut fonts = vec![name.clone()];
                    fonts.extend(definitions.families.get(&fallback).cloned().unwrap_or_default());
                    definitions.families.insert(egui::FontFamily::Name(name.clone().into()), fonts);
                    ctx.data_mut(|data| data.insert_temp(egui::Id::new("viewer_font_definitions"), definitions.clone()));
                    ctx.set_fonts(definitions); ctx.request_repaint();
                    self.entries.insert(path.clone(), Entry::Ready(name));
                }
                _ => { self.entries.insert(path.clone(), Entry::Failed); }
            }
        }
        if let Some(Entry::Ready(name)) = self.entries.get(&path) {
            let family = egui::FontFamily::Name(name.clone().into());
            if ctx.fonts(|fonts| fonts.families().contains(&family)) { return family; }
        }
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_and_tmp_resolve_their_own_reference_and_ignore_builtin_fonts() {
        let guid = "abcdef0123456789abcdef0123456789";
        let text = format!("  m_FontData:\n    m_Font: {{fileID: 12800000, guid: {guid}, type: 3}}\n");
        assert_eq!(reference(&text, false), Some(guid.into()));
        assert_eq!(reference(&text, true), None);
        let tmp = format!("  m_fontAsset: {{fileID: 11400000, guid: {guid}, type: 2}}\n");
        assert_eq!(reference(&tmp, true), Some(guid.into()));
        assert_eq!(reference("    m_Font: {fileID: 12800000, guid: 00000000000000000000000000000000, type: 0}", false), None);
    }
}
