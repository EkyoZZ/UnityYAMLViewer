//! Editor-only preview settings. Never read or write the normal playback preference.
use super::*;

fn config_path() -> Option<PathBuf> {
    crate::last_open_dir_config_path()
        .map(|p| p.with_file_name("subtitle_editor_preview_style.json"))
}
fn normalize(mut style: Style) -> Style {
    style.native_ass = false;
    style.size = style.size.clamp(8.0, 120.0);
    style.outline_size = style.outline_size.clamp(0.0, 12.0);
    style.shadow_x = style.shadow_x.clamp(-12.0, 12.0);
    style.shadow_y = style.shadow_y.clamp(-12.0, 12.0);
    style.margin_x = style.margin_x.min(300);
    style.margin_y = style.margin_y.min(300);
    style
}
fn preference() -> &'static std::sync::Mutex<Style> {
    static STYLE: std::sync::OnceLock<std::sync::Mutex<Style>> = std::sync::OnceLock::new();
    STYLE.get_or_init(|| {
        std::sync::Mutex::new(normalize(
            config_path()
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| decode_style(&b).ok())
                .unwrap_or_default(),
        ))
    })
}
fn write_style(path: &Path, style: &Style) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Missing style settings folder")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = parent.join(format!(
        ".editor-preview-style-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let bytes =
            serde_json::to_vec_pretty(&normalize(style.clone())).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        crate::playlist::replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub(crate) struct EditorPreviewStyle {
    saved: Style,
    draft: Style,
    open: bool,
    search: String,
    error: Option<String>,
}
impl Default for EditorPreviewStyle {
    fn default() -> Self {
        let saved = preference().lock().unwrap().clone();
        Self {
            draft: saved.clone(),
            saved,
            open: false,
            search: String::new(),
            error: None,
        }
    }
}
impl EditorPreviewStyle {
    pub(crate) fn sync(&mut self) -> bool {
        let saved = preference().lock().unwrap().clone();
        if saved == self.saved {
            return false;
        }
        let clean = self.draft == self.saved;
        if clean {
            self.draft = saved.clone();
        }
        self.saved = saved;
        clean
    }
    pub(crate) fn open(&mut self, ctx: &egui::Context, id: egui::Id) {
        self.sync();
        if !self.open {
            self.draft = self.saved.clone();
            self.error = None;
            self.search.clear();
        }
        self.open = true;
        crate::native_window::wake(ctx, id);
        ctx.request_repaint();
    }
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.draft = self.saved.clone();
        self.error = None;
    }
    pub(crate) fn window(&mut self, ctx: &egui::Context, id: egui::Id) -> bool {
        if !self.open {
            return false;
        }
        let before = self.draft.clone();
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        crate::native_window::Window::new(tr("字幕编辑预览样式")).id(id).open(&mut open).default_width(360.0).fit_height().show(ctx,|ui|{
            ui.small(tr("仅用于字幕编辑预览，不修改字幕文档，也不影响普通播放；保存、转换和烧录不自动带入此样式。"));
            ui.small(tr("修改后预览；应用保存，取消或关闭恢复原样。"));
            ui.push_id(id,|ui|uniform_style_controls(ui,&mut self.draft,&mut self.search));
            if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
            ui.separator();ui.horizontal(|ui|{
                let labels=[tr("应用"),tr("取消"),tr("重置")];
                let widths=labels.map(|label|ui.painter().layout_no_wrap(label.to_owned(),egui::FontId::proportional(16.0),ui.visuals().text_color()).size().x+24.0).map(|width|width.max(88.0));
                ui.add_space(((ui.available_width()-widths.iter().sum::<f32>()-ui.spacing().item_spacing.x*2.0)/2.0).max(0.0));
                apply=ui.add_sized([widths[0],32.0],egui::Button::new(egui::RichText::new(labels[0]).size(16.0))).clicked();
                cancel=ui.add_sized([widths[1],32.0],egui::Button::new(egui::RichText::new(labels[1]).size(16.0))).clicked();
                if ui.add_sized([widths[2],32.0],egui::Button::new(egui::RichText::new(labels[2]).size(16.0))).clicked(){self.draft=Style::default();self.search.clear();}
            });
        });
        if apply {
            let result = (|| {
                let mut value = preference().lock().unwrap();
                let path = config_path().ok_or("APPDATA is unavailable")?;
                let style = normalize(self.draft.clone());
                write_style(&path, &style)?;
                *value = style.clone();
                self.saved = style;
                Ok::<(), String>(())
            })();
            match result {
                Ok(()) => {
                    self.draft = self.saved.clone();
                    open = false;
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
        }
        if cancel {
            open = false;
        }
        self.open = open;
        if !open {
            self.draft = self.saved.clone();
        }
        before != self.draft
    }
    pub(crate) fn script(&self, ass: &str, size: Option<[usize; 2]>) -> String {
        // Match playback's script-unit scale without changing source cues or paging their times.
        let [width, height] = size
            .filter(|[w, h]| *w > 0 && *h > 0)
            .unwrap_or([1920, 1080]);
        let script_width = (288.0 * width as f64 / height as f64).round().max(1.0) as u32;
        let canvas = ass
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("PlayResX:") {
                    format!("PlayResX: {script_width}\n")
                } else if line.trim_start().starts_with("PlayResY:") {
                    "PlayResY: 288\n".into()
                } else {
                    format!("{line}\n")
                }
            })
            .collect::<String>();
        bake_ass_style(
            &apply_shadow_offsets_mode(&canvas, &self.draft, false),
            &self.draft,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_settings_roundtrip_use_their_own_file_and_force_uniform_mode() {
        let path = std::env::temp_dir().join(format!(
            "uyv-editor-preview-style-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut value = Style::default();
        value.native_ass = true;
        value.size = 43.0;
        value.shadow = true;
        value.shadow_x = -4.0;
        write_style(&path, &value).unwrap();
        let restored = decode_style(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(restored.size, 43.0);
        assert_eq!(restored.shadow_x, -4.0);
        assert!(!restored.native_ass);
        value.size = 31.0;
        write_style(&path, &value).unwrap();
        assert_eq!(
            decode_style(&std::fs::read(&path).unwrap()).unwrap().size,
            31.0
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn editor_style_is_separate_and_preview_does_not_change_source_or_cue_timing() {
        if let Some(path) = config_path() {
            assert_ne!(Some(path), style_path());
        }
        let normal = style_preference().lock().unwrap();
        let normal_before = normal.clone();
        let source=crate::subtitle_document::Document::parse("1\n00:00:01,000 --> 00:00:30,000\nA long subtitle with many repeated words for wrapping. A long subtitle with many repeated words for wrapping.\n",crate::subtitle_document::Format::Srt).unwrap();
        let before = source.serialize();
        let mut style = EditorPreviewStyle {
            saved: Style::default(),
            draft: Style::default(),
            open: false,
            search: String::new(),
            error: None,
        };
        style.draft.size = 39.0;
        style.draft.top = true;
        let preview = style.script(&source.to_ass(Some([1920, 1080])), Some([1920, 1080]));
        assert!(preview.contains("PlayResY: 288"));
        assert!(preview.contains(",39,"));
        assert!(preview.contains("\\an8"));
        assert_eq!(preview.matches("Dialogue:").count(), 1);
        assert!(preview.contains("0:00:01.00,0:00:30.00"));
        assert_eq!(source.serialize(), before);
        assert!(*normal == normal_before);
        style.close();
        assert!(style.draft == style.saved);
    }
}
