//! Independent subtitle sessions with source documents and isolated preview players.
use super::*;
use crate::{
    localization::tr,
    subtitle_document::{self as document, Document, Format, Line, Record},
};
use std::collections::BTreeSet;

thread_local! {
    static EDITORS:std::cell::RefCell<Vec<Editor>>=const {std::cell::RefCell::new(Vec::new())};
}
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        path.parent()
            .and_then(|p| std::fs::canonicalize(p).ok())
            .zip(path.file_name())
            .map_or_else(|| path.to_owned(), |(parent, name)| parent.join(name))
    })
}

pub(super) fn open(
    ctx: &egui::Context,
    source: &Path,
    video: &Path,
    preview: &Preview,
) -> Result<(), String> {
    let path = canonical(source);
    EDITORS.with(|editors| {
        let mut editors = editors.borrow_mut();
        if let Some(editor) = editors.iter().find(|editor| {
            canonical(&editor.path) == path || editor.save_target.as_ref() == Some(&path) || matches!(editor.pending.as_ref(),Some(Pending::Load(target)) if canonical(target)==path)
        }) {
            crate::native_window::wake(ctx, editor.id);
            return Ok(());
        }
        let (doc, bytes) = document::read(&path)?;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = egui::Id::new(("subtitle_editor", NEXT.fetch_add(1, Ordering::Relaxed)));
        let mut player = Preview::default();
        player.info = preview.info.clone();
        player.selected_audio = preview.selected_audio;
        player.frame_times = preview.frame_times.clone();
        // The normal player may have a waveform for a different audio track.
        player.waveform = None;
        player.position = preview.position_now();
        player.volume = preview.volume;
        player.remembered_volume=preview.remembered_volume;
        player.decoder = preview.decoder.clone();
        let mut editor = Editor {
            path: path.clone(),
            video: video.to_owned(),
            id,
            player,
            saved: doc.clone(),
            doc,
            source_bytes: bytes,
            selected: BTreeSet::new(),
            active: None,
            list_reveal:None,
            timeline_reveal:None,
            followed_cue:None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: None,
            history_navigation: false,
            preview_due: Some(Instant::now()),
            preview_text: String::new(),
            editor_style:crate::subtitle_player::EditorPreviewStyle::default(),
            save_job: None,
            save_target: None,
            error: None,
            status: String::new(),
            pending: None,
            confirm: false,
            conversion: None,
            span: 20.0,
            offset: 0.0,
            follow: true,
            snap: true,
            shift_ms: 100,
            search: String::new(),
            replacement: String::new(),
            style_index: None,
            show_styles: false,
            drag: None,
            layout:if preview.info.as_ref().and_then(|i|i.video).is_some_and(|[w,h]|h>w){"portrait"}else{"landscape"}.into(),
        };
        let position=(editor.player.position_now()*1000.0) as i64;
        let cues=editor.doc.cues();
        if let Some(i) = cues.iter().copied().find(|i|editor.doc.cue(*i).is_some_and(|r|!r.comment()&&r.start()<=position&&r.end()>position))
            .or_else(||cues.iter().copied().find(|i|editor.doc.cue(*i).is_some_and(|r|r.start()>=position)))
            .or_else(||cues.last().copied()) {
            editor.select(i, false);
            editor.reveal_cue(i);
        }
        editors.push(editor);
        crate::native_window::wake(ctx, id);
        Ok(())
    })
}
pub(super) fn show(ctx: &egui::Context) {
    EDITORS.with(|editors| {
        let mut editors = editors.borrow_mut();
        let mut reserved = editors
            .iter()
            .flat_map(|e| {
                std::iter::once(canonical(&e.path))
                    .chain(e.save_target.iter().cloned())
                    .chain(e.conversion.iter().map(|p| canonical(p)))
                    .chain(e.pending.iter().filter_map(|p| {
                        if let Pending::Load(path) = p {
                            Some(canonical(path))
                        } else {
                            None
                        }
                    }))
            })
            .collect::<Vec<_>>();
        editors.retain_mut(|editor| editor.show(ctx, &mut reserved));
    });
}
pub(super) fn files_in_use() -> Vec<PathBuf> {
    EDITORS.with(|e| {
        e.borrow()
            .iter()
            .flat_map(|e| [e.path.clone(), e.video.clone()])
            .collect()
    })
}

enum Pending {
    Close,
    Load(PathBuf),
}
struct Saved {
    path: PathBuf,
    doc: Document,
    bytes: Vec<u8>,
}
struct Drag {
    index: usize,
    start: i64,
    end: i64,
    origin: f32,
    mode: u8,
    selected: Vec<(usize, i64, i64)>,
}
struct Editor {
    path: PathBuf,
    video: PathBuf,
    id: egui::Id,
    player: Preview,
    doc: Document,
    saved: Document,
    source_bytes: Vec<u8>,
    selected: BTreeSet<usize>,
    active: Option<usize>,
    list_reveal: Option<usize>,
    timeline_reveal: Option<usize>,
    followed_cue: Option<usize>,
    undo: Vec<Document>,
    redo: Vec<Document>,
    last_edit: Option<Instant>,
    history_navigation: bool,
    preview_due: Option<Instant>,
    preview_text: String,
    editor_style: crate::subtitle_player::EditorPreviewStyle,
    save_job: Option<mpsc::Receiver<Result<Saved, String>>>,
    save_target: Option<PathBuf>,
    error: Option<String>,
    status: String,
    pending: Option<Pending>,
    confirm: bool,
    conversion: Option<PathBuf>,
    span: f64,
    offset: f64,
    follow: bool,
    snap: bool,
    shift_ms: i64,
    search: String,
    replacement: String,
    style_index: Option<usize>,
    show_styles: bool,
    drag: Option<Drag>,
    layout: String,
}
impl Editor {
    fn dirty(&self) -> bool {
        self.doc != self.saved
    }
    fn clear_selection(&mut self) {
        self.selected.clear();
        self.active = None;
        self.list_reveal = None;
        self.timeline_reveal = None;
        self.followed_cue = None;
        self.drag = None;
        self.last_edit = None;
    }
    fn select(&mut self, i: usize, multiple: bool) {
        if !multiple {
            self.selected.clear();
        }
        if multiple && self.selected.contains(&i) {
            self.selected.remove(&i);
            if !self
                .active
                .is_some_and(|active| self.selected.contains(&active))
            {
                self.active = self.selected.last().copied();
            }
        } else {
            self.selected.insert(i);
            self.active = Some(i);
        }
        self.list_reveal = self.active;
        if !self.search.is_empty()
            && self
                .active
                .and_then(|i| self.doc.cue(i))
                .is_some_and(|r| !r.get("Text").contains(&self.search))
        {
            self.search.clear();
        }
        self.last_edit = None;
    }
    fn reveal_time(&mut self, position: f64) {
        let duration = self
            .player
            .info
            .as_ref()
            .map_or(0.0, |i| i.duration.max(0.0));
        let span = self.span.min(duration);
        self.offset = (position - span * 0.5).clamp(0.0, (duration - span).max(0.0));
    }
    fn reveal_cue(&mut self, index: usize) {
        let Some(record) = self.doc.cue(index) else {
            return;
        };
        let start = record.start() as f64 / 1000.0;
        let end = record.end() as f64 / 1000.0;
        if start < self.offset || end > self.offset + self.span {
            self.reveal_time(if end - start <= self.span {
                (start + end) * 0.5
            } else {
                start + self.span * 0.4
            });
        }
        self.timeline_reveal = Some(index);
    }
    fn remember(&mut self, before: Document) {
        let now = Instant::now();
        if self
            .last_edit
            .is_none_or(|last| now.duration_since(last) > Duration::from_millis(500))
        {
            // Bound history by both steps and document size (roughly 32 MiB of source text).
            let limit = (32 * 1024 * 1024 / before.serialize().len().max(1)).clamp(1, 64);
            while self.undo.len() >= limit {
                self.undo.remove(0);
            }
            self.undo.push(before);
        }
        self.redo.clear();
        self.last_edit = Some(now);
        self.preview_due = Some(now + Duration::from_millis(250));
        self.status.clear();
    }
    fn undo(&mut self, redo: bool) {
        let next = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(doc) = next {
            let old = std::mem::replace(&mut self.doc, doc);
            if redo {
                self.undo.push(old);
            } else {
                self.redo.push(old);
            }
            self.clear_selection();
            self.last_edit = None;
            self.history_navigation = true;
            self.preview_due = Some(Instant::now());
        }
    }
    fn request(&mut self, pending: Pending) {
        self.player.pause();
        self.drag = None;
        self.pending = Some(pending);
        self.confirm = self.dirty();
    }
    fn save(&mut self, path: PathBuf, reserved: &mut Vec<PathBuf>) {
        if self.save_job.is_some() {
            return;
        }
        let path = if canonical(&path) == canonical(&self.path) {
            path
        } else {
            path.with_extension(match Format::from_path(&path) {
                Ok(f) => f.extension(),
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            })
        };
        if canonical(&path) != canonical(&self.path) && reserved.contains(&canonical(&path)) {
            self.error = Some(tr("该字幕已在其他编辑窗口中打开。").into());
            return;
        }
        if let Err(error) = self.doc.validate() {
            self.error = Some(error);
            self.confirm = self.pending.is_some();
            return;
        }
        let target = if canonical(&path) == canonical(&self.path) {
            self.doc.format
        } else {
            Format::from_path(&path).unwrap()
        };
        if target == Format::Ssa && self.doc.format != Format::Ssa {
            self.error = Some(tr("暂不支持将其他格式转换成旧版 SSA，请另存为 ASS。").into());
            return;
        }
        if self.doc.conversion_warning(target) {
            reserved.push(canonical(&path));
            self.conversion = Some(path);
            return;
        }
        reserved.push(canonical(&path));
        self.start_save(path);
    }
    fn start_save(&mut self, path: PathBuf) {
        self.drag = None;
        let doc = self.doc.clone();
        let size = self.player.info.as_ref().and_then(|i| i.video);
        let expected =
            (canonical(&path) == canonical(&self.path)).then(|| self.source_bytes.clone());
        let target = if expected.is_some() {
            doc.format
        } else {
            match Format::from_path(&path) {
                Ok(format) => format,
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            }
        };
        let (tx, rx) = mpsc::channel();
        self.save_job = Some(rx);
        self.save_target = Some(canonical(&path));
        self.error = None;
        std::thread::spawn(move || {
            let result = (|| {
                let format = target;
                let text = doc.convert(format, size)?;
                let doc = Document::parse(&text, format)?;
                doc.validate()?;
                let bytes = text.into_bytes();
                if bytes.len() > 32 * 1024 * 1024 {
                    return Err("Subtitle exceeds 32 MiB".into());
                }
                document::save(&path, &bytes, expected.as_deref())?;
                document::notify_saved(&path);
                Ok(Saved { path, doc, bytes })
            })();
            let _ = tx.send(result);
        });
    }
    fn save_as(&mut self, reserved: &mut Vec<PathBuf>) {
        let dialog = rfd::FileDialog::new()
            .add_filter("ASS", &["ass"])
            .add_filter("SRT", &["srt"])
            .add_filter("WebVTT", &["vtt"]);
        let dialog = if self.doc.format == Format::Ssa {
            dialog.add_filter("SSA", &["ssa"])
        } else {
            dialog
        };
        if let Some(path) = dialog
            .set_directory(self.path.parent().unwrap_or(Path::new(".")))
            .set_file_name(
                self.path
                    .with_extension(Format::Ass.extension())
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .save_file()
        {
            self.save(path, reserved);
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        if self.editor_style.sync() && !self.doc.format.styled() {
            self.preview_due = Some(Instant::now());
        }
        if let Some(job) = &self.save_job {
            match job.try_recv() {
                Ok(result) => {
                    self.save_job = None;
                    self.save_target = None;
                    match result {
                        Ok(saved) => {
                            self.path = saved.path;
                            self.doc = saved.doc.clone();
                            self.saved = saved.doc;
                            self.source_bytes = saved.bytes;
                            self.undo.clear();
                            self.redo.clear();
                            self.clear_selection();
                            self.show_styles = false;
                            self.style_index = None;
                            self.preview_due = Some(Instant::now());
                            self.status = tr("字幕已保存").into();
                        }
                        Err(error) => {
                            self.error = Some(error);
                            self.pending = None;
                            self.confirm = false;
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.save_job = None;
                    self.save_target = None;
                    self.error = Some(tr("保存失败").into());
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.save_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
        if self.preview_due.is_some_and(|due| Instant::now() >= due) && self.drag.is_none() {
            let raw = self
                .doc
                .to_ass(self.player.info.as_ref().and_then(|i| i.video));
            let text = if self.doc.format.styled() {
                raw
            } else {
                self.editor_style
                    .script(&raw, self.player.info.as_ref().and_then(|i| i.video))
            };
            if text != self.preview_text {
                match crate::subtitle_player::Render::draft(&text) {
                    Ok(render) => {
                        let playing = self.player.is_playing();
                        let position = self.player.position_now();
                        self.player.stop_playback();
                        self.player.cache = Arc::new(Mutex::new(FrameCache::default()));
                        self.player.current_frame = None;
                        self.player.subtitles.editor_render(render);
                        self.player.position = position;
                        self.player.frame_job = None;
                        if playing {
                            self.player.play(ctx, &self.video);
                        } else {
                            self.player
                                .preview_trim_position(ctx, &self.video, position, false);
                        }
                        self.preview_text = text;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            self.preview_due = None;
        } else if self.preview_due.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    fn show(&mut self, ctx: &egui::Context, reserved: &mut Vec<PathBuf>) -> bool {
        self.poll(ctx);
        let mut open = true;
        let title = format!(
            "{} — {}{}",
            tr("字幕编辑"),
            self.path.file_name().unwrap_or_default().to_string_lossy(),
            if self.dirty() { " *" } else { "" }
        );
        crate::native_window::Window::new(&title).id(self.id).open(&mut open).default_width(1120.0).default_height(850.0).min_width(640.0).min_height(520.0).show(ctx,|ui|{
            if ui.input(|i|i.viewport().close_requested()){ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);self.request(Pending::Close);}
            let busy=self.save_job.is_some()||self.confirm||self.conversion.is_some();
            let before=(!busy&&ui.input(|input|input.pointer.primary_down()||input.events.iter().any(|event|matches!(event,egui::Event::Key{pressed:true,..}|egui::Event::Text(_)|egui::Event::Paste(_)|egui::Event::Cut|egui::Event::PointerButton{..}|egui::Event::MouseWheel{..})))).then(||self.doc.clone());
            self.history_navigation=false;
            ui.add_enabled_ui(!busy,|ui|{
                self.toolbar(ui,reserved);
                if !busy&&(self.save_job.is_some()||self.confirm||self.conversion.is_some()){return;}
                if !busy&&ui.input(|i|i.focused)&&!ui.memory(|m|m.any_popup_open()){
                    if ui.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::S)){self.save(self.path.clone(),reserved);}
                    if !super::text_input_active(ui.ctx()){
                        if ui.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Z)){self.undo(false);}
                        if ui.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Y)){self.undo(true);}
                    }
                }
                if !busy&&(self.save_job.is_some()||self.confirm||self.conversion.is_some()){return;}
                if !self.status.is_empty(){ui.small(&self.status);}
                self.content(ui);
            });
            if let Some(before)=before{if before!=self.doc&&!self.history_navigation {self.remember(before);}}
            if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
            if self.save_job.is_some(){ui.spinner();ui.label(tr("正在保存字幕…"));}
            if self.confirm{
                egui::Modal::new(self.id.with("unsaved")).show(ui.ctx(),|ui|{
                    ui.label(tr("字幕有未保存的修改。"));ui.horizontal(|ui|{
                        if ui.add_enabled(self.save_job.is_none(),egui::Button::new(tr("保存"))).clicked(){self.confirm=false;self.save(self.path.clone(),reserved);}
                        if ui.button(tr("放弃修改")).clicked(){self.doc=self.saved.clone();self.confirm=false;}
                        if ui.button(tr("取消")).clicked(){self.pending=None;self.confirm=false;}
                    });
                });
            }
            if let Some(path)=self.conversion.clone(){
                egui::Modal::new(self.id.with("conversion")).show(ui.ctx(),|ui|{
                    ui.set_max_width(440.0);ui.label(tr("转换格式可能丢失样式、定位、动画、图层、注释，以及 WebVTT 的标识、CSS、区域和时间设置。原文件保持不变；仅另存目标使用转换结果。"));
                    ui.horizontal(|ui|{
                        if ui.button(tr("继续转换")).clicked(){self.conversion=None;self.start_save(path);}
                        if ui.button(tr("取消")).clicked(){self.conversion=None;self.pending=None;}
                    });
                });
            }
        });
        if self.doc.format.styled() {
            self.editor_style.close();
        } else if self
            .editor_style
            .window(ctx, self.id.with("editor_preview_style"))
        {
            self.preview_due = Some(Instant::now() + Duration::from_millis(180));
            ctx.request_repaint_after(Duration::from_millis(180));
        }
        if !open && self.pending.is_none() {
            self.request(Pending::Close);
        }
        if !self.confirm && !self.dirty() && self.save_job.is_none() && self.conversion.is_none() {
            match self.pending.take() {
                Some(Pending::Close) => return false,
                Some(Pending::Load(path)) => match document::read(&path) {
                    Ok((doc, bytes)) => {
                        self.path = path;
                        self.doc = doc.clone();
                        self.saved = doc;
                        self.source_bytes = bytes;
                        self.undo.clear();
                        self.redo.clear();
                        self.clear_selection();
                        self.show_styles = false;
                        self.style_index = None;
                        self.preview_due = Some(Instant::now());
                    }
                    Err(error) => self.error = Some(error),
                },
                None => {}
            }
        }
        true
    }
    fn content(&mut self, ui: &mut egui::Ui) {
        if self.layout == "portrait" {
            let width = ui.available_width().max(1.0);
            let minimum = (width * 0.20).clamp(120.0, 240.0).min(width * 0.45);
            let maximum = (width - 300.0).max(minimum).min(width * 0.55);
            let [w, h] = self
                .player
                .info
                .as_ref()
                .and_then(|i| i.video)
                .unwrap_or([9, 16]);
            let preferred = ((ui.available_height() - 35.0).max(1.0) * w as f32 / h.max(1) as f32
                + 2.0)
                .clamp(minimum, maximum);
            egui::SidePanel::left("subtitle_portrait_video")
                .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                    left: 0,
                    right: 2,
                    top: 0,
                    bottom: 0,
                }))
                .resizable(true)
                .default_width(preferred)
                .width_range(minimum..=maximum)
                .show_inside(ui, |ui| {
                    self.player.subtitle_preview(ui, &self.video);
                });
            self.editing_area(ui, true);
        } else {
            self.editing_area(ui, false);
        }
    }
    fn editing_area(&mut self, ui: &mut egui::Ui, portrait: bool) {
        ui.push_id(
            if portrait {
                "portrait_edit"
            } else {
                "landscape_edit"
            },
            |ui| {
                let lower = (ui.available_height() * 0.40).clamp(160.0, 320.0);
                egui::TopBottomPanel::bottom("subtitle_rows")
                    .resizable(true)
                    .default_height(lower)
                    .height_range(120.0..=500.0)
                    .show_inside(ui, |ui| self.list(ui));
                egui::TopBottomPanel::bottom("subtitle_timeline")
                    .resizable(true)
                    .default_height(165.0)
                    .height_range(110.0..=320.0)
                    .show_inside(ui, |ui| self.timeline(ui));
                if portrait {
                    // Clip properties to the space left by the bottom panels;
                    // a ScrollArea alone permits painting beyond its viewport margin.
                    egui::CentralPanel::default()
                        .frame(egui::Frame::side_top_panel(ui.style()))
                        .show_inside(ui, |ui| {
                            self.properties_panel(ui);
                        });
                } else {
                    egui::SidePanel::right("subtitle_properties")
                        .frame(egui::Frame::side_top_panel(ui.style()))
                        .resizable(true)
                        .default_width(340.0)
                        .width_range(220.0..=560.0)
                        .show_inside(ui, |ui| {
                            self.properties_panel(ui);
                        });
                    self.player.subtitle_preview(ui, &self.video);
                }
            },
        );
    }
    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        let cue = self
            .active
            .and_then(|i| self.doc.cue(i))
            .map(|record| (record.start(), record.end()));
        egui::TopBottomPanel::bottom("subtitle_playback_controls")
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 2)))
            .exact_height(35.0)
            .show_inside(ui, |ui| {
                self.player.subtitle_playback_controls(ui, &self.video, cue);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("subtitle_property_scroll")
                    .min_scrolled_height(0.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.show_styles {
                            self.styles(ui);
                        } else {
                            self.properties(ui);
                        }
                    });
            });
    }
    fn toolbar(&mut self, ui: &mut egui::Ui, reserved: &mut Vec<PathBuf>) {
        ui.horizontal_wrapped(|ui| {
            crate::theme::compact_selector(
                ui,
                &mut self.layout,
                [("landscape", "横屏布局"), ("portrait", "竖屏布局")],
            );
            ui.small(format!(
                "{} · {}",
                self.doc.format.extension().to_uppercase(),
                tr("原格式编辑")
            ));
            ui.strong(tr("字幕来源"));
            ui.add(egui::Label::new(self.path.display().to_string()).truncate())
                .on_hover_text(self.path.display().to_string());
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("加载字幕")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "ASS / SSA / SRT / WebVTT",
                        &["ass", "ssa", "srt", "vtt", "webvtt"],
                    )
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    if canonical(&path) != canonical(&self.path)
                        && reserved.contains(&canonical(&path))
                    {
                        self.error = Some(tr("该字幕已在其他编辑窗口中打开。").into());
                    } else {
                        reserved.push(canonical(&path));
                        self.request(Pending::Load(path));
                    }
                }
            }
            if ui.button(tr("保存")).clicked() {
                self.save(self.path.clone(), reserved);
            }
            if ui.button(tr("另存为…")).clicked() {
                self.save_as(reserved);
            }
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new(tr("撤销")))
                .clicked()
            {
                self.undo(false);
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new(tr("重做")))
                .clicked()
            {
                self.undo(true);
            }
            ui.selectable_value(&mut self.show_styles, false, tr("字幕文本"));
            ui.add_enabled_ui(self.doc.format.styled(), |ui| {
                ui.selectable_value(&mut self.show_styles, true, tr("ASS／SSA 样式"));
            });
            if !self.doc.format.styled() && ui.button(tr("统一预览样式")).clicked() {
                self.editor_style
                    .open(ui.ctx(), self.id.with("editor_preview_style"));
            }
            if ui.button(tr("使用草稿烧录…")).clicked() {
                self.send_to_burn(ui);
            }
            ui.small(if self.dirty() {
                tr("未保存")
            } else {
                tr("已保存")
            });
        });
        if self.doc.format == Format::Vtt {
            ui.small(tr(
                "WebVTT 文件信息会保留；预览暂不完整还原 CSS、区域、定位、说话人和逐词时间效果。",
            ));
        }
    }
    fn send_to_burn(&mut self, ui: &egui::Ui) {
        if let Err(error) = self.doc.validate() {
            self.error = Some(error);
            return;
        }
        match crate::subtitle_player::Render::draft(
            &self
                .doc
                .to_ass(self.player.info.as_ref().and_then(|i| i.video)),
        ) {
            Ok(render) => {
                let duration = self.player.info.as_ref().map_or(0.0, |i| i.duration);
                self.player.open_trim_editor(ui, &self.video, duration);
                let key = canonical(&self.video);
                TRIM_EDITORS.with(|editors| {
                    if let Some(editor) = editors.borrow_mut().get_mut(&key) {
                        if let Some((_, player)) = editor
                            .players
                            .iter_mut()
                            .find(|(path, _)| canonical(path) == key)
                        {
                            player.trim.set_burn_snapshot(render);
                        }
                    }
                });
                self.status = tr("草稿快照已送入视频编辑，请在视频编辑窗口导出。").into();
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("查找"));
            ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(120.0));
            ui.label(tr("替换"));
            ui.add(egui::TextEdit::singleline(&mut self.replacement).desired_width(120.0));
            if ui
                .add_enabled(!self.search.is_empty(), egui::Button::new(tr("全部替换")))
                .clicked()
            {
                for i in self.doc.cues() {
                    if let Some(r) = self.doc.cue_mut(i) {
                        let value = r.get("Text").replace(&self.search, &self.replacement);
                        r.set("Text", value);
                    }
                }
                self.last_edit = None;
            }
            if ui.button(tr("新增")).clicked() {
                self.insert();
            }
            if ui
                .add_enabled(self.active.is_some(), egui::Button::new(tr("复制")))
                .clicked()
            {
                if let Some(i) = self.active {
                    if let Some(mut record) = self.doc.cue(i).cloned() {
                        if self.doc.format == Format::Vtt {
                            record.set("Identifier", String::new());
                        }
                        self.doc.lines.insert(i + 1, Line::Cue(record));
                        self.select(i + 1, false);
                    }
                }
            }
            if ui
                .add_enabled(!self.selected.is_empty(), egui::Button::new(tr("删除")))
                .clicked()
            {
                for &i in self.selected.iter().rev() {
                    self.doc.lines.remove(i);
                }
                self.clear_selection();
                self.last_edit = None;
            }
            if ui
                .add_enabled(self.active.is_some(), egui::Button::new(tr("在指针处拆分")))
                .clicked()
            {
                self.split();
            }
            if ui
                .add_enabled(
                    self.selected.len() > 1,
                    egui::Button::new(tr("合并选中字幕")),
                )
                .clicked()
            {
                self.merge();
            }
            if ui.checkbox(&mut self.follow, tr("跟随播放")).changed() && self.follow {
                self.followed_cue = None;
                self.list_reveal = None;
            }
            self.shift_controls(ui);
        });
        let position = (self.player.position_now() * 1000.0) as i64;
        let indices = self
            .doc
            .cues()
            .into_iter()
            .enumerate()
            .filter(|(_, i)| {
                self.search.is_empty()
                    || self.doc.cue(*i).unwrap().get("Text").contains(&self.search)
            })
            .collect::<Vec<_>>();
        let playing = self.player.is_playing();
        let mut reveal = self.list_reveal.take();
        let current = indices
            .iter()
            .find(|(_, i)| {
                let r = self.doc.cue(*i).unwrap();
                !r.comment() && r.start() <= position && r.end() > position
            })
            .map(|(_, i)| *i);
        if self.follow
            && playing
            && !super::text_input_active(ui.ctx())
            && self.search.is_empty()
            && self.selected.len() <= 1
            && reveal.is_none()
            && current != self.followed_cue
        {
            if let Some(i) = current {
                self.active = Some(i);
                self.selected.clear();
                self.selected.insert(i);
                reveal = Some(i);
            }
        }
        self.followed_cue = if playing { current } else { None };
        let active = self.active;
        let selected_range = active
            .and_then(|i| self.doc.cue(i))
            .map(|r| (r.start(), r.end()));
        let row_height = 24.0;
        let mut area = egui::ScrollArea::vertical()
            .id_salt("subtitle_grid")
            .min_scrolled_height(0.0)
            .auto_shrink([false, false]);
        if let Some(index) = reveal {
            if let Some(row) = indices.iter().position(|(_, i)| *i == index) {
                area = area.vertical_scroll_offset(
                    (row as f32 * row_height - ui.available_height() * 0.4).max(0.0),
                );
            }
        }
        area.show_rows(ui, row_height, indices.len(), |ui, range| {
            for row in range {
                let (number, i) = indices[row];
                let r = self.doc.cue(i).unwrap();
                let live = !r.comment() && r.start() <= position && r.end() > position;
                let invalid = r.end() <= r.start();
                let overlapping = Some(i) != active
                    && !r.comment()
                    && selected_range
                        .is_some_and(|(start, end)| r.start() < end && r.end() > start);
                let label = format!(
                    "{:>4}  {} → {}  {:>5}ms  [{}] {}",
                    number + 1,
                    document::display_time(r.start()),
                    document::display_time(r.end()),
                    r.end() - r.start(),
                    r.get("Style"),
                    r.get("Text").replace('\n', " ↵ ")
                );
                let color = if invalid {
                    ui.visuals().error_fg_color
                } else if live {
                    crate::theme::BLUE
                } else if overlapping {
                    ui.visuals().warn_fg_color
                } else if r.comment() {
                    ui.visuals().weak_text_color()
                } else {
                    ui.visuals().text_color()
                };
                let response = ui.add(
                    egui::Button::new(egui::RichText::new(label).color(color))
                        .selected(self.selected.contains(&i))
                        .truncate()
                        .min_size(egui::vec2(ui.available_width(), row_height)),
                );
                if response.clicked() || response.double_clicked() {
                    self.select(i, ui.input(|input| input.modifiers.ctrl));
                    self.list_reveal = None;
                    if let Some(active) = self.active {
                        self.reveal_cue(active);
                    }
                }
                if response.double_clicked() {
                    let target = self.doc.cue(i).unwrap().start() as f64 / 1000.0;
                    self.player
                        .preview_trim_position(ui.ctx(), &self.video, target, false);
                }
                if overlapping {
                    response.on_hover_text(tr(
                        "与当前字幕时间重叠；ASS 可正常叠加显示，不会自动修改时间。",
                    ));
                }
            }
        });
    }
    fn insert(&mut self) {
        let mut record = self
            .active
            .and_then(|i| self.doc.cue(i))
            .or_else(|| self.doc.cues().last().and_then(|i| self.doc.cue(*i)))
            .cloned()
            .unwrap_or_else(|| self.doc.new_record());
        if self.doc.format.styled() && self.doc.cues().is_empty() {
            if let Some(columns) = self.doc.lines.iter().rev().find_map(|line| match line {
                Line::Raw(s)
                    if s.trim_start().starts_with("Format:")
                        && s.to_ascii_lowercase().contains("text") =>
                {
                    Some(
                        s.split_once(':')
                            .unwrap()
                            .1
                            .split(',')
                            .map(|s| s.trim().to_owned())
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            }) {
                let default = record;
                record = Record {
                    prefix: default.prefix.clone(),
                    values: columns
                        .iter()
                        .map(|name| default.get(name).to_owned())
                        .collect(),
                    columns,
                };
            }
        }
        record.set("Text", String::new());
        if self.doc.format == Format::Vtt {
            record.set("Identifier", String::new());
        }
        record.set("Effect", String::new());
        let start = (self.player.position_now() * 1000.0) as i64;
        set_times(&mut record, start, start + 2000, self.doc.format);
        if self.doc.format == Format::Vtt
            && self.doc.cues().is_empty()
            && !matches!(self.doc.lines.last(),Some(Line::Raw(s)) if s.is_empty())
        {
            self.doc.lines.push(Line::Raw(String::new()));
        }
        let index = if let Some(i) = self.active {
            i + 1
        } else {
            self.doc.cues().last().map_or_else(||self.doc.lines.iter().rposition(|l|matches!(l,Line::Raw(s) if s.trim().to_ascii_lowercase().starts_with("format:")&&s.to_ascii_lowercase().contains("text"))).map_or(self.doc.lines.len(),|i|i+1),|i|i+1)
        };
        self.doc.lines.insert(index, Line::Cue(record));
        self.select(index, false);
    }
    fn split(&mut self) {
        let Some(i) = self.active else {
            return;
        };
        let Some(mut right) = self.doc.cue(i).cloned() else {
            return;
        };
        let time = (self.player.position_now() * 1000.0).round() as i64;
        let (start, end) = (right.start(), right.end());
        if time <= start || time >= end {
            self.error = Some(tr("播放指针需要位于当前字幕内部。").into());
            return;
        }
        let format = self.doc.format;
        if format == Format::Vtt {
            right.set("Identifier", String::new());
        }
        set_times(self.doc.cue_mut(i).unwrap(), start, time, format);
        set_times(&mut right, time, end, self.doc.format);
        self.doc.lines.insert(i + 1, Line::Cue(right));
        self.select(i + 1, false);
    }
    fn merge(&mut self) {
        let indices = self.selected.iter().copied().collect::<Vec<_>>();
        let cues = self.doc.cues();
        let positions = indices
            .iter()
            .filter_map(|i| cues.iter().position(|x| x == i))
            .collect::<Vec<_>>();
        if positions.windows(2).any(|p| p[1] != p[0] + 1) {
            self.error = Some(tr("只能合并相邻字幕。").into());
            return;
        }
        let Some(&first) = indices.first() else {
            return;
        };
        let first_record = self.doc.cue(first).unwrap();
        if indices.iter().any(|i| {
            let r = self.doc.cue(*i).unwrap();
            r.comment() != first_record.comment()
                || [
                    "Style", "Layer", "Name", "MarginL", "MarginR", "MarginV", "Effect", "Settings",
                ]
                .iter()
                .any(|field| r.get(field) != first_record.get(field))
        }) {
            self.error = Some(tr("合并字幕需要相同的样式、图层和注释类型。").into());
            return;
        }
        let start = indices
            .iter()
            .map(|i| self.doc.cue(*i).unwrap().start())
            .min()
            .unwrap();
        let end = indices
            .iter()
            .map(|i| self.doc.cue(*i).unwrap().end())
            .max()
            .unwrap();
        let text = indices
            .iter()
            .map(|i| self.doc.cue(*i).unwrap().get("Text"))
            .collect::<Vec<_>>()
            .join(if !self.doc.format.styled() {
                "\n"
            } else {
                "\\N"
            });
        let format = self.doc.format;
        let record = self.doc.cue_mut(first).unwrap();
        record.set("Text", text);
        set_times(record, start, end, format);
        for &i in indices.iter().skip(1).rev() {
            self.doc.lines.remove(i);
        }
        self.select(first, false);
    }
    fn properties(&mut self, ui: &mut egui::Ui) {
        let Some(i) = self.active else {
            ui.label(tr("选择字幕以编辑文字和时间。"));
            return;
        };
        let Some(record) = self.doc.cue(i).cloned() else {
            return;
        };
        let format = self.doc.format;
        let position = (self.player.position_now() * 1000.0).round() as i64;
        let mut start = record.start();
        let mut end = record.end();
        ui.strong(tr("当前字幕"));
        time_field(ui, tr("起始"), &mut start);
        time_field(ui, tr("结束"), &mut end);
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("指针设为起点")).clicked() {
                start = position;
            }
            if ui.button(tr("指针设为终点")).clicked() {
                end = position;
            }
        });
        ui.small(format!("{} ms", end - start));
        if end <= start {
            ui.colored_label(ui.visuals().error_fg_color, tr("终点必须晚于起点。"));
        }
        let mut text = if !format.styled() {
            record.get("Text").to_owned()
        } else {
            record.get("Text").replace("\\N", "\n")
        };
        ui.label(tr("字幕文本"));
        if ui
            .add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.doc.cue_mut(i).unwrap().set(
                "Text",
                if !format.styled() {
                    text.clone()
                } else {
                    text.replace('\n', "\\N")
                },
            );
        }
        set_times(self.doc.cue_mut(i).unwrap(), start, end, format);
        let plain = regex::Regex::new(r"\{[^}]*\}")
            .unwrap()
            .replace_all(&text, "")
            .to_string();
        let chars = plain.chars().filter(|c| !c.is_whitespace()).count();
        let cps = chars as f64 / ((end - start).max(1) as f64 / 1000.0);
        ui.small(format!("{}: {chars} · CPS: {cps:.1}", tr("字符数")));
        ui.separator();
        if format == Format::Vtt {
            for (field, label) in [("Identifier", "字幕标识"), ("Settings", "WebVTT 时间设置")]
            {
                let mut value = record.get(field).to_owned();
                ui.label(tr(label));
                if ui.text_edit_singleline(&mut value).changed() {
                    self.doc.cue_mut(i).unwrap().set(field, value);
                }
            }
        }
        ui.add_enabled_ui(format.styled(), |ui| {
            if !format.styled() {
                ui.small(tr(
                    "文档样式需另存为 ASS 后编辑；预览样式可在顶部单独设置。",
                ));
            }
            let mut style = record.get("Style").to_owned();
            let styles = self
                .doc
                .lines
                .iter()
                .filter_map(|l| {
                    if let Line::Style(s) = l {
                        Some(s.get("Name").to_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            egui::ComboBox::from_id_salt("cue_style")
                .width(180.0)
                .truncate()
                .selected_text(&style)
                .show_ui(ui, |ui| {
                    for name in styles {
                        ui.selectable_value(&mut style, name.clone(), name);
                    }
                });
            if style != record.get("Style") {
                self.doc.cue_mut(i).unwrap().set("Style", style);
            }
            for (field, label) in [
                ("Layer", "图层"),
                ("Marked", "旧版标记"),
                ("Name", "角色名"),
                ("MarginL", "左边距"),
                ("MarginR", "右边距"),
                ("MarginV", "垂直边距"),
                ("Effect", "效果"),
            ] {
                if format.styled()
                    && !record
                        .columns
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(field))
                {
                    continue;
                }
                let mut value = record.get(field).to_owned();
                ui.horizontal(|ui| {
                    ui.label(tr(label));
                    if ui.text_edit_singleline(&mut value).changed() {
                        self.doc.cue_mut(i).unwrap().set(field, value);
                    }
                });
            }
            let mut comment = record.comment();
            if ui.checkbox(&mut comment, tr("注释（不显示）")).changed() {
                self.doc.cue_mut(i).unwrap().prefix =
                    if comment { "Comment: " } else { "Dialogue: " }.into();
            }
            ui.small(tr(
                "可直接编辑 ASS 标签；复杂定位、动画、绘图和卡拉 OK 暂无可视化工具。",
            ));
        });
    }
    fn shift_controls(&mut self, ui: &mut egui::Ui) {
        let text_width = |text: &str, style: egui::TextStyle| {
            ui.painter()
                .layout_no_wrap(
                    text.to_owned(),
                    style.resolve(ui.style()),
                    ui.visuals().text_color(),
                )
                .size()
                .x
        };
        let padding = ui.spacing().button_padding.x * 2.0;
        let drag_width = (text_width("3600000 ms", egui::TextStyle::Monospace) + padding)
            .max(ui.spacing().interact_size.x);
        let width = text_width(tr("选中字幕批量平移"), egui::TextStyle::Body)
            + drag_width
            + ["提前", "延后"]
                .into_iter()
                .map(|label| {
                    (text_width(tr(label), egui::TextStyle::Button) + padding)
                        .max(ui.spacing().interact_size.x)
                })
                .sum::<f32>()
            + 3.0 * ui.spacing().item_spacing.x;
        ui.allocate_ui_with_layout(
            egui::vec2(width.ceil(), ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
                    ui.label(tr("选中字幕批量平移"));
                    ui.add_sized(
                        [drag_width, ui.spacing().interact_size.y],
                        egui::DragValue::new(&mut self.shift_ms)
                            .range(1..=3_600_000)
                            .suffix(" ms"),
                    );
                    for (label, sign) in [("提前", -1), ("延后", 1)] {
                        if ui.button(tr(label)).clicked() {
                            let indices = self.selected.iter().copied().collect::<Vec<_>>();
                            self.doc.shift(&indices, self.shift_ms * sign);
                            if let Some(index) = self.active {
                                self.reveal_cue(index);
                            }
                            self.last_edit = None;
                        }
                    }
                });
            },
        );
    }
    fn styles(&mut self, ui: &mut egui::Ui) {
        let styles = self
            .doc
            .lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                if let Line::Style(s) = l {
                    Some((i, s.get("Name").to_owned()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut selected = self.style_index.or_else(|| styles.first().map(|(i, _)| *i));
        egui::ComboBox::from_id_salt("ass_styles")
            .selected_text(
                selected
                    .and_then(|i| styles.iter().find(|(j, _)| *j == i))
                    .map_or("", |(_, s)| s.as_str()),
            )
            .show_ui(ui, |ui| {
                for (i, name) in &styles {
                    ui.selectable_value(&mut selected, Some(*i), name);
                }
            });
        self.style_index = selected;
        if ui.button(tr("复制样式")).clicked() {
            if let Some(i) = selected {
                if let Line::Style(mut record) = self.doc.lines[i].clone() {
                    let name = format!("{}_copy", record.get("Name"));
                    let mut unique = name.clone();
                    let mut n = 2;
                    while styles.iter().any(|(_, s)| s == &unique) {
                        unique = format!("{name}{n}");
                        n += 1;
                    }
                    record.set("Name", unique);
                    self.doc.lines.insert(i + 1, Line::Style(record));
                    self.style_index = Some(i + 1);
                    self.clear_selection();
                    self.last_edit = None;
                }
            }
        }
        let Some(i) = self.style_index else {
            ui.label(tr("当前字幕没有可编辑的样式。"));
            return;
        };
        let Some(Line::Style(record)) = self.doc.lines.get(i).cloned() else {
            return;
        };
        ui.small(tr("修改样式会影响使用该样式的全部字幕。"));
        for (field, label) in [
            ("Name", "样式名称"),
            ("Fontname", "字体"),
            ("Fontsize", "字号"),
            ("Outline", "描边"),
            ("Shadow", "投影"),
            ("MarginL", "左边距"),
            ("MarginR", "右边距"),
            ("MarginV", "垂直边距"),
        ] {
            let mut value = record.get(field).to_owned();
            ui.horizontal(|ui| {
                ui.label(tr(label));
                if ui.text_edit_singleline(&mut value).changed() {
                    if field == "Name"
                        && (!value.is_empty()
                            && !styles.iter().any(|(j, name)| *j != i && name == &value))
                    {
                        let old = record.get("Name");
                        for line in &mut self.doc.lines {
                            if let Line::Cue(cue) = line {
                                if cue.get("Style") == old {
                                    cue.set("Style", value.clone());
                                }
                            }
                        }
                    }
                    if field != "Name"
                        || (!value.is_empty()
                            && !styles.iter().any(|(j, name)| *j != i && name == &value))
                    {
                        if let Line::Style(style) = &mut self.doc.lines[i] {
                            style.set(field, value);
                        }
                    }
                }
            });
        }
        for (field, label) in [
            ("PrimaryColour", "文字颜色"),
            ("SecondaryColour", "次要颜色"),
            ("OutlineColour", "描边颜色"),
            ("BackColour", "阴影颜色"),
        ] {
            if self.doc.format == Format::Ssa && field == "BackColour" {
                continue;
            }
            let field = if self.doc.format == Format::Ssa && field == "OutlineColour" {
                "BackColour"
            } else {
                field
            };
            if !record
                .columns
                .iter()
                .any(|name| name.eq_ignore_ascii_case(field))
            {
                continue;
            }
            let mut color = parse_color(record.get(field));
            if self.doc.format == Format::Ssa {
                color[3] = 255 - record.get("AlphaLevel").parse::<u8>().unwrap_or(0);
            }
            ui.horizontal(|ui| {
                ui.label(tr(label));
                if ui
                    .color_edit_button_srgba_unmultiplied(&mut color)
                    .changed()
                {
                    if let Line::Style(style) = &mut self.doc.lines[i] {
                        if self.doc.format == Format::Ssa {
                            style.set(
                                field,
                                ((color[2] as u32) << 16
                                    | (color[1] as u32) << 8
                                    | color[0] as u32)
                                    .to_string(),
                            );
                            style.set("AlphaLevel", (255 - color[3]).to_string());
                        } else {
                            style.set(
                                field,
                                format!(
                                    "&H{:02X}{:02X}{:02X}{:02X}",
                                    255 - color[3],
                                    color[2],
                                    color[1],
                                    color[0]
                                ),
                            );
                        }
                    }
                }
            });
        }
        for (field, label) in [
            ("Bold", "粗体"),
            ("Italic", "斜体"),
            ("Underline", "下划线"),
            ("StrikeOut", "删除线"),
        ] {
            if !record
                .columns
                .iter()
                .any(|name| name.eq_ignore_ascii_case(field))
            {
                continue;
            }
            let mut enabled = record.get(field).parse::<i32>().unwrap_or(0) != 0;
            if ui.checkbox(&mut enabled, tr(label)).changed() {
                if let Line::Style(style) = &mut self.doc.lines[i] {
                    style.set(field, if enabled { "-1" } else { "0" }.into());
                }
            }
        }
        let mut align = record.get("Alignment").parse::<usize>().unwrap_or(2);
        if self.doc.format == Format::Ssa {
            align = document::ssa_alignment_to_ass(align);
        }
        let original_alignment = align;
        egui::ComboBox::from_id_salt("ass_alignment")
            .selected_text(format!("{}: {align}", tr("对齐")))
            .show_ui(ui, |ui| {
                for n in 1..=9 {
                    ui.selectable_value(&mut align, n, n.to_string());
                }
            });
        if align != original_alignment {
            if let Line::Style(style) = &mut self.doc.lines[i] {
                style.set(
                    "Alignment",
                    if self.doc.format == Format::Ssa {
                        document::ass_alignment_to_ssa(align)
                    } else {
                        align
                    }
                    .to_string(),
                );
            }
        }
        ui.small(tr(
            "对齐采用 ASS 数字键盘方位：1–3 底部，4–6 中部，7–9 顶部。边距为脚本单位。",
        ));
        if self.doc.format == Format::Ssa {
            ui.small(tr(
                "SSA 的描边与阴影共用背景颜色；透明度使用原格式 AlphaLevel，预览按 SSA 规则处理。",
            ));
        }
    }
    fn timeline(&mut self, ui: &mut egui::Ui) {
        let duration = self
            .player
            .info
            .as_ref()
            .map_or(1.0, |i| i.duration.max(0.001));
        self.span = self
            .span
            .clamp((duration / 2000.0).max(0.1).min(duration), duration);
        self.offset = self.offset.clamp(0.0, (duration - self.span).max(0.0));
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("时间轴"));
            ui.checkbox(&mut self.snap, tr("吸附视频帧"));
            if ui.button(tr("定位到指针")).clicked() {
                self.reveal_time(self.player.position_now());
            }
            if ui.button("−").clicked() {
                self.span = (self.span * 1.5).min(duration);
            }
            if ui.button("+").clicked() {
                self.span = (self.span / 1.5).max(0.1).min(duration);
            }
        });
        if ui.is_enabled()
            && ui.input(|i| i.focused)
            && !super::text_input_active(ui.ctx())
            && !ui.memory(|m| m.any_popup_open())
        {
            let steps = ui.input_mut(|i| {
                i.count_and_consume_key(egui::Modifiers::CTRL, egui::Key::ArrowRight) as isize
                    - i.count_and_consume_key(egui::Modifiers::CTRL, egui::Key::ArrowLeft) as isize
            });
            if steps != 0 {
                let delta = if let Some(times) =
                    self.player.frame_times.as_ref().filter(|t| !t.is_empty())
                {
                    let base = times
                        .partition_point(|t| *t <= self.offset)
                        .saturating_sub(1);
                    let target = base.saturating_add_signed(5 * steps).min(times.len() - 1);
                    times[target] - times[base]
                } else {
                    steps as f64 * 5.0 / self.player.info.as_ref().map_or(25.0, |i| i.fps.max(1.0))
                };
                self.offset = (self.offset + delta).clamp(0.0, (duration - self.span).max(0.0));
            }
        }
        timeline_navigation(ui, &mut self.offset, self.span, duration);
        let mut entries = self
            .doc
            .cues()
            .into_iter()
            .map(|i| {
                let r = self.doc.cue(i).unwrap();
                (i, r.start(), r.end())
            })
            .filter(|(_, s, e)| {
                *e as f64 / 1000.0 >= self.offset && (*s as f64 / 1000.0) <= self.offset + self.span
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|(i, s, _)| (*s, *i));
        let mut ends = Vec::<i64>::new();
        let mut blocks = Vec::new();
        for (i, start, end) in entries {
            let lane = ends
                .iter()
                .position(|last| *last <= start)
                .unwrap_or(ends.len());
            if lane == ends.len() {
                ends.push(end);
            } else {
                ends[lane] = end;
            }
            blocks.push((i, start, end, lane));
        }
        let mut neighbors = vec![(None, None); blocks.len()];
        let mut previous = vec![None; ends.len()];
        for (at, (_, _, end, lane)) in blocks.iter().enumerate() {
            neighbors[at].0 = previous[*lane];
            previous[*lane] = Some(*end);
        }
        let mut next = vec![None; ends.len()];
        for (at, (_, start, _, lane)) in blocks.iter().enumerate().rev() {
            neighbors[at].1 = next[*lane];
            next[*lane] = Some(*start);
        }
        let available = ui.available_height().max(1.0);
        let mut area = timeline_scroll_area(available);
        if let Some(index) = self.timeline_reveal.take() {
            if let Some((_, _, _, lane)) = blocks.iter().find(|(i, _, _, _)| *i == index) {
                area = area.vertical_scroll_offset(
                    (45.0 + *lane as f32 * 25.0 - available * 0.4).max(0.0),
                );
            }
        }
        area.show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(
                    ui.available_width().max(1.0),
                    (45.0 + ends.len().max(1) as f32 * 25.0).max(available),
                ),
                egui::Sense::click(),
            );
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
            let offset = self.offset;
            let span = self.span;
            let x = |ms: i64| {
                rect.left() + ((ms as f64 / 1000.0 - offset) / span) as f32 * rect.width()
            };
            for n in 0..=8 {
                let at = self.offset + self.span * n as f64 / 8.0;
                let xx = rect.left() + rect.width() * n as f32 / 8.0;
                painter.line_segment(
                    [egui::pos2(xx, rect.top()), egui::pos2(xx, rect.bottom())],
                    egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
                painter.text(
                    egui::pos2(xx + 2.0, rect.top()),
                    egui::Align2::LEFT_TOP,
                    document::display_time((at * 1000.0) as i64),
                    egui::FontId::monospace(10.0),
                    ui.visuals().weak_text_color(),
                );
            }
            if let Some(wave) = &self.player.waveform {
                if let Some(peaks) = wave.first() {
                    for column in 0..rect.width().min(2400.0) as usize {
                        let n = rect.width().min(2400.0).max(1.0);
                        let t0 = self.offset + self.span * column as f64 / n as f64;
                        let t1 = self.offset + self.span * (column + 1) as f64 / n as f64;
                        let first = (t0 / duration * peaks.len() as f64) as usize;
                        let last = ((t1 / duration * peaks.len() as f64).ceil() as usize)
                            .max(first + 1)
                            .min(peaks.len());
                        let peak = peaks
                            .get(first..last)
                            .unwrap_or(&[])
                            .iter()
                            .copied()
                            .fold(0.0_f32, f32::max);
                        let xx = rect.left() + column as f32 * rect.width() / n;
                        painter.line_segment(
                            [
                                egui::pos2(xx, rect.top() + 31.0 - peak * 12.0),
                                egui::pos2(xx, rect.top() + 31.0 + peak * 12.0),
                            ],
                            egui::Stroke::new(1.0_f32, crate::theme::BLUE),
                        );
                    }
                }
            }
            let mut clicked = None;
            let mut started = None;
            let mut handles = Vec::new();
            for (at, (i, start, end, lane)) in blocks.into_iter().enumerate() {
                let block = egui::Rect::from_min_max(
                    egui::pos2(
                        x(start).max(rect.left()),
                        rect.top() + 45.0 + lane as f32 * 25.0,
                    ),
                    egui::pos2(
                        x(end)
                            .min(rect.right())
                            .max(x(start).max(rect.left()) + 3.0),
                        rect.top() + 68.0 + lane as f32 * 25.0,
                    ),
                )
                .intersect(rect);
                if !block.is_positive() {
                    continue;
                }
                let item = ui
                    .interact(
                        block,
                        self.id.with(("subtitle_block", i)),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::Grab);
                let record = self.doc.cue(i).unwrap();
                painter.rect_filled(
                    block,
                    2.0,
                    if self.selected.contains(&i) {
                        crate::theme::BLUE
                    } else if record.comment() {
                        ui.visuals().widgets.inactive.bg_fill
                    } else {
                        egui::Color32::from_rgb(48, 105, 95)
                    },
                );
                let galley = painter.layout(
                    record.get("Text").to_owned(),
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                    block.width().max(1.0),
                );
                ui.painter()
                    .with_clip_rect(block.intersect(ui.clip_rect()))
                    .galley(
                        block.left_top() + egui::vec2(3.0, 3.0),
                        galley,
                        egui::Color32::WHITE,
                    );
                for (mode, edge, neighbor) in [
                    (1, x(start), neighbors[at].0.map(x)),
                    (2, x(end), neighbors[at].1.map(x)),
                ] {
                    if edge >= rect.left() && edge <= rect.right() {
                        let hit = edge_hit_rect(
                            block,
                            edge,
                            mode,
                            neighbor,
                            rect.intersect(ui.clip_rect()),
                        );
                        if hit.is_positive() {
                            handles.push((i, start, end, mode, edge, block, hit));
                        }
                    }
                }
                if item.clicked() {
                    clicked = Some((i, item.double_clicked()));
                }
                if item.drag_started() {
                    if let Some(origin) = ui.input(|input| input.pointer.press_origin()) {
                        started = Some((i, start, end, origin.x, 0));
                    }
                }
                item.on_hover_text(format!(
                    "{} → {}\n{}",
                    document::display_time(start),
                    document::display_time(end),
                    record.get("Text")
                ));
            }
            // Register handles after all bodies so their wider targets win over body dragging.
            for (i, start, end, mode, edge, block, hit) in handles {
                let handle = ui
                    .interact(
                        hit,
                        self.id.with(("subtitle_edge", i, mode)),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                let active = self
                    .drag
                    .as_ref()
                    .is_some_and(|drag| drag.index == i && drag.mode == mode);
                let highlighted = handle.hovered() || handle.dragged() || active;
                if highlighted {
                    painter.rect_filled(hit, 2.0, egui::Color32::LIGHT_GREEN.gamma_multiply(0.18));
                }
                painter.line_segment(
                    [
                        egui::pos2(edge, block.top()),
                        egui::pos2(edge, block.bottom()),
                    ],
                    egui::Stroke::new(
                        if highlighted { 5.0_f32 } else { 2.0_f32 },
                        egui::Color32::LIGHT_GREEN,
                    ),
                );
                if handle.clicked() {
                    clicked = Some((i, handle.double_clicked()));
                }
                if handle.drag_started() {
                    if let Some(origin) = ui.input(|input| input.pointer.press_origin()) {
                        started = Some((i, start, end, origin.x, mode));
                    }
                }
                handle.on_hover_text(format!(
                    "{}\n{}",
                    tr(if mode == 1 {
                        "拖动调整字幕起点"
                    } else {
                        "拖动调整字幕终点"
                    }),
                    document::display_time(if mode == 1 { start } else { end })
                ));
            }
            if let Some((i, jump)) = clicked {
                self.select(i, ui.input(|i| i.modifiers.ctrl));
                ui.ctx().request_repaint();
                if jump {
                    let at = self.doc.cue(i).unwrap().start() as f64 / 1000.0;
                    self.player
                        .preview_trim_position(ui.ctx(), &self.video, at, false);
                }
            }
            if let Some((index, start, end, origin, mode)) = started {
                self.player.pause();
                if !self.selected.contains(&index) {
                    self.select(index, false);
                }
                self.last_edit = None;
                self.drag = Some(Drag {
                    index,
                    start,
                    end,
                    origin,
                    mode,
                    selected: self
                        .selected
                        .iter()
                        .filter_map(|i| self.doc.cue(*i).map(|r| (*i, r.start(), r.end())))
                        .collect(),
                });
            }
            if let Some(drag) = &self.drag {
                ui.ctx().set_cursor_icon(if drag.mode == 0 {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::ResizeHorizontal
                });
                if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
                    let delta =
                        ((pointer.x - drag.origin) / rect.width()) as f64 * self.span * 1000.0;
                    let start = self.snap_time((drag.start as f64 + delta).round() as i64);
                    let end = self.snap_time((drag.end as f64 + delta).round() as i64);
                    let format = self.doc.format;
                    if drag.mode == 0 {
                        let minimum = drag.selected.iter().map(|(_, s, _)| *s).min().unwrap_or(0);
                        let delta = (start - drag.start).max(-minimum);
                        for &(i, s, e) in &drag.selected {
                            if let Some(r) = self.doc.cue_mut(i) {
                                set_times(r, s + delta, e + delta, format);
                            }
                        }
                    } else if let Some(r) = self.doc.cue_mut(drag.index) {
                        if drag.mode == 1 {
                            set_times(r, start.min(drag.end - 10).max(0), drag.end, format);
                        } else {
                            set_times(r, drag.start, end.max(drag.start + 10), format);
                        }
                    }
                }
                if !ui.input(|i| i.pointer.primary_down()) {
                    self.drag = None;
                }
            }
            let cursor = x((self.player.position_now() * 1000.0) as i64);
            painter.line_segment(
                [
                    egui::pos2(cursor, rect.top()),
                    egui::pos2(cursor, rect.bottom()),
                ],
                egui::Stroke::new(2.0_f32, egui::Color32::YELLOW),
            );
            if response.clicked() && clicked.is_none() {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let at =
                        self.offset + ((pointer.x - rect.left()) / rect.width()) as f64 * self.span;
                    self.player.preview_trim_position(
                        ui.ctx(),
                        &self.video,
                        at.clamp(0.0, duration),
                        false,
                    );
                }
            }
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 && self.drag.is_none() {
                    let anchor = ui.input(|i| i.pointer.hover_pos()).map_or(0.5, |p| {
                        ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0)
                    }) as f64;
                    let target = (self.span * (-scroll as f64 * 0.003).exp())
                        .clamp(0.1_f64.min(duration), duration);
                    self.offset = (self.offset + (self.span - target) * anchor)
                        .clamp(0.0, (duration - target).max(0.0));
                    self.span = target;
                    ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                }
            }
        });
    }
    fn snap_time(&self, ms: i64) -> i64 {
        if !self.snap {
            return ms.max(0);
        }
        if let Some(times) = self.player.frame_times.as_ref().filter(|t| !t.is_empty()) {
            let at = ms as f64 / 1000.0;
            let right = times.partition_point(|t| *t < at).min(times.len() - 1);
            let left = right.saturating_sub(1);
            return (if (times[right] - at).abs() < (times[left] - at).abs() {
                times[right]
            } else {
                times[left]
            } * 1000.0)
                .round() as i64;
        }
        ms.max(0)
    }
}
fn edge_hit_rect(
    block: egui::Rect,
    edge: f32,
    mode: u8,
    neighbor: Option<f32>,
    clip: egui::Rect,
) -> egui::Rect {
    const RADIUS: f32 = 10.0;
    let inward = RADIUS.min(block.width() * 0.25);
    let (left, right) = if mode == 1 {
        (
            (edge - RADIUS).max(neighbor.map_or(f32::NEG_INFINITY, |other| (edge + other) * 0.5)),
            edge + inward,
        )
    } else {
        (
            edge - inward,
            (edge + RADIUS).min(neighbor.map_or(f32::INFINITY, |other| (edge + other) * 0.5)),
        )
    };
    egui::Rect::from_min_max(
        egui::pos2(left, block.top()),
        egui::pos2(right, block.bottom()),
    )
    .intersect(clip)
}
fn timeline_scroll_area(height: f32) -> egui::ScrollArea {
    egui::ScrollArea::vertical()
        .id_salt("subtitle_timeline_lanes")
        .min_scrolled_height(0.0)
        .max_height(height.max(1.0))
        .auto_shrink([false, false])
}
fn timeline_navigation(
    ui: &mut egui::Ui,
    offset: &mut f64,
    span: f64,
    duration: f64,
) -> Option<egui::Rect> {
    if duration <= span {
        return None;
    }
    let maximum = duration - span;
    *offset = offset.clamp(0.0, maximum);
    Some(
        egui::TopBottomPanel::bottom("subtitle_timeline_navigation")
            .exact_height(26.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().max(1.0), 14.0),
                    egui::Sense::hover(),
                );
                super::timeline_pan_bar(ui, rect, response.id.with("pan"), offset, span, duration);
                rect
            })
            .inner,
    )
}
fn set_times(record: &mut Record, start: i64, end: i64, format: Format) {
    if !format.styled() {
        if record.start() != start {
            record.set("Start", document::display_time(start));
        }
        if record.end() != end {
            record.set("End", document::display_time(end));
        }
    } else {
        record.times(start, end);
    }
}
fn time_field(ui: &mut egui::Ui, label: &str, time: &mut i64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(time)
                .range(0..=359_999_999)
                .speed(10)
                .custom_formatter(|n, _| document::display_time(n as i64))
                .custom_parser(|s| document::parse_time(s).map(|n| n as f64)),
        );
    });
}
fn parse_color(value: &str) -> [u8; 4] {
    let value = value.trim();
    let n = if value.starts_with("&H")
        || value.starts_with("&h")
        || value.starts_with("0x")
        || value.starts_with("0X")
    {
        u32::from_str_radix(value[2..].trim_end_matches('&'), 16).unwrap_or(0x00ffffff)
    } else {
        value.parse::<i64>().unwrap_or(0x00ffffff) as u32
    };
    [
        (n & 255) as u8,
        ((n >> 8) & 255) as u8,
        ((n >> 16) & 255) as u8,
        255 - (n >> 24) as u8,
    ]
}

fn centered_row(ui: &mut egui::Ui, width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    let (row, _) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width().max(1.0),
            ui.available_height().max(28.0),
        ),
        egui::Sense::hover(),
    );
    let rect = egui::Rect::from_center_size(row.center(), egui::vec2(width.min(row.width()), 28.0));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        contents,
    );
}

impl Preview {
    fn subtitle_playback_controls(
        &mut self,
        ui: &mut egui::Ui,
        path: &Path,
        cue: Option<(i64, i64)>,
    ) {
        let mut toggle = false;
        let mut step = 0;
        let mut play_cue = false;
        let old_volume = self.volume;
        let width = 5.0 * 28.0 + 4.0 * ui.spacing().item_spacing.x;
        centered_row(ui, width, |ui| {
            ui.add_enabled_ui(
                cue.is_some_and(|(start, end)| end > start) && self.info.is_some(),
                |ui| {
                    play_cue = transport_button(ui, crate::icons::Icon::TrimPlay, tr("播放当前句"));
                },
            );
            toggle = transport_button(
                ui,
                if self.is_playing() {
                    crate::icons::Icon::Pause
                } else {
                    crate::icons::Icon::Play
                },
                tr(if self.is_playing() {
                    "暂停"
                } else {
                    "播放"
                }),
            );
            ui.add_enabled_ui(
                !self.is_playing() && self.frame_job.is_none() && self.frame_times.is_some(),
                |ui| {
                    if transport_button(ui, crate::icons::Icon::Previous, tr("上一帧")) {
                        step = -1;
                    }
                    if transport_button(ui, crate::icons::Icon::Next, tr("下一帧")) {
                        step = 1;
                    }
                },
            );
            volume_control(
                ui,
                path,
                &mut self.volume,
                &mut self.remembered_volume,
                &mut self.volume_hover,
                None,
            );
        });
        if self.volume != old_volume {
            if let Some(playback) = &self.playback {
                playback
                    .control
                    .volume
                    .store(self.volume.to_bits(), Ordering::Relaxed);
            }
        }
        if play_cue {
            if let Some((start, end)) = cue {
                self.stop_playback();
                self.position = start as f64 / 1000.0;
                self.trim_preview_end = Some(end as f64 / 1000.0);
                self.play(ui.ctx(), path);
            }
        } else {
            self.subtitle_apply_transport(ui, path, toggle, step);
        }
    }
    fn subtitle_apply_transport(
        &mut self,
        ui: &mut egui::Ui,
        path: &Path,
        toggle: bool,
        step: isize,
    ) {
        let Some(duration) = self.info.as_ref().map(|info| info.duration) else {
            return;
        };
        let mut seek = None;
        if step != 0 && !self.is_playing() && self.frame_job.is_none() {
            if let Some(times) = self.frame_times.as_ref().filter(|t| !t.is_empty()) {
                let base = self.current_frame.unwrap_or_else(|| {
                    times
                        .partition_point(|t| *t <= self.position)
                        .saturating_sub(1)
                });
                let index = base.saturating_add_signed(step).min(times.len() - 1);
                seek = Some(times[index]);
            }
        }
        if let Some(at) = seek {
            self.preview_trim_position(ui.ctx(), path, at, false);
        }
        if toggle {
            if self.is_playing() {
                self.pause();
            } else {
                if self.position >= duration {
                    self.position = 0.0;
                }
                self.trim_preview_end = None;
                self.play(ui.ctx(), path);
            }
        }
    }
    fn subtitle_preview(&mut self, ui: &mut egui::Ui, path: &Path) {
        let Some(info) = self.info.clone() else {
            ui.label(tr("正在读取媒体信息…"));
            return;
        };
        if self.frame_times.is_none() && self.index_job.is_none() && self.index_error.is_none() {
            self.index_job = Some(index_frames(&info, path, ui.ctx()));
        }
        if let Some(job) = &self.index_job {
            match job.receiver.try_recv() {
                Ok(result) => {
                    self.index_job = None;
                    match result {
                        Ok(times) => {
                            self.frame_times = Some(Arc::new(times));
                            if !self.is_playing() {
                                self.current_frame = None;
                                self.preview_trim_position(ui.ctx(), path, self.position, false);
                            }
                        }
                        Err(error) => self.index_error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.index_job = None;
                    self.index_error = Some("Frame index worker disconnected".into());
                }
                _ => {}
            }
        }
        if info.audio
            && self.waveform.is_none()
            && self.wave_job.is_none()
            && self.wave_error.is_none()
        {
            self.wave_job = Some(waveform_track(&info, path, self.selected_audio, ui.ctx()));
        }
        if let Some(job) = &self.wave_job {
            match job.receiver.try_recv() {
                Ok(result) => {
                    self.wave_job = None;
                    match result {
                        Ok(wave) => self.waveform = Some(wave),
                        Err(error) => self.wave_error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.wave_job = None;
                    self.wave_error = Some("Waveform worker disconnected".into());
                }
                _ => {}
            }
        }
        if let Some(job) = &self.frame_job {
            match job.receiver.try_recv() {
                Ok(result) => {
                    self.frame_job = None;
                    match result {
                        Ok((index, frame)) => {
                            self.current_frame = Some(index);
                            self.position = self
                                .frame_times
                                .as_ref()
                                .and_then(|t| t.get(index))
                                .copied()
                                .unwrap_or(self.position);
                            present_playback_frame(
                                &mut self.texture,
                                &mut self.video_frame,
                                PlaybackFrame::Rgba(frame),
                                self.gpu_video,
                                ui.ctx(),
                            );
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.frame_job = None;
                    self.error = Some("Frame worker disconnected".into());
                }
                _ => {}
            }
        }
        let mut done = false;
        if let Some(job) = &self.playback {
            self.started = *job.control.clock.lock().unwrap();
            if !job.control.paused.load(Ordering::Relaxed) {
                if let Some((index, frame)) = job.frames.try_iter().last() {
                    self.current_frame = Some(index);
                    present_playback_frame(
                        &mut self.texture,
                        &mut self.video_frame,
                        frame,
                        self.gpu_video,
                        ui.ctx(),
                    );
                }
            }
            if let Ok(error) = job.errors.try_recv() {
                self.error = Some(error);
                done = true;
            }
            done |= job.done.load(Ordering::Relaxed)
                || self
                    .trim_preview_end
                    .is_some_and(|end| self.position_now() >= end);
            job.control
                .volume
                .store(self.volume.to_bits(), Ordering::Relaxed);
        }
        if done {
            let position = self
                .trim_preview_end
                .unwrap_or(info.duration)
                .min(info.duration);
            self.stop_playback();
            self.position = position;
            self.trim_preview_end = None;
        }
        let render_error = self.yuv_error.lock().unwrap().take();
        if render_error.is_some() {
            let playing = self.is_playing();
            self.position = self.position_now();
            self.stop_playback();
            self.gpu_video = false;
            self.video_frame = None;
            self.current_frame = None;
            if playing {
                self.play(ui.ctx(), path);
            } else {
                self.seek_frame(ui.ctx(), path);
            }
        }
        if self.texture.is_none()
            && self.video_frame.is_none()
            && self.frame_job.is_none()
            && self.error.is_none()
        {
            self.preview_trim_position(ui.ctx(), path, self.position, false);
        }
        let mut toggle = false;
        let mut step = 0isize;
        egui::TopBottomPanel::bottom("subtitle_transport")
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 2)))
            .exact_height(35.0)
            .show_inside(ui, |ui| {
                let label = format!("{} / {}", time(self.position_now()), time(info.duration));
                let menu_width = ui
                    .painter()
                    .layout_no_wrap(
                        tr("音轨 / 语言").to_owned(),
                        egui::TextStyle::Button.resolve(ui.style()),
                        ui.visuals().text_color(),
                    )
                    .size()
                    .x
                    + ui.spacing().button_padding.x * 2.0;
                let time_width = ui
                    .painter()
                    .layout_no_wrap(
                        label.clone(),
                        egui::TextStyle::Body.resolve(ui.style()),
                        ui.visuals().text_color(),
                    )
                    .size()
                    .x;
                let width = menu_width + ui.spacing().item_spacing.x + time_width;
                centered_row(ui, width, |ui| {
                    let old_audio = self.selected_audio;
                    ui.menu_button(tr("音轨 / 语言"), |ui| {
                        ui.set_width(240.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        for track in &info.audio_tracks {
                            if ui
                                .selectable_label(
                                    self.selected_audio == Some(track.index),
                                    track.display_label(),
                                )
                                .on_hover_text(&track.label)
                                .clicked()
                            {
                                self.selected_audio = Some(track.index);
                                ui.close_menu();
                            }
                        }
                    });
                    if self.selected_audio != old_audio {
                        let playing = self.is_playing();
                        self.position = self.position_now();
                        self.stop_playback();
                        self.wave_job = None;
                        self.waveform = None;
                        self.wave_error = None;
                        if playing {
                            self.play(ui.ctx(), path);
                        }
                    }
                    ui.add(egui::Label::new(&label).truncate())
                        .on_hover_text(label);
                });
            });
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let Some(error) = &self.index_error {
            ui.small(error);
        }
        let available = ui.available_size().max(egui::vec2(1.0, 1.0));
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click());
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        let size = info.video.map_or(egui::vec2(1.0, 1.0), |[w, h]| {
            egui::vec2(w as f32, h as f32)
        });
        let factor = (rect.width() / size.x).min(rect.height() / size.y);
        let image = egui::Rect::from_center_size(rect.center(), size * factor);
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        if let Some(frame) = &self.video_frame {
            ui.painter().add(crate::render_system::video::callback(
                image,
                image.intersect(ui.clip_rect()),
                frame.clone(),
                [false, false],
                self.yuv_error.clone(),
            ));
        } else if let Some(texture) = &self.texture {
            ui.painter()
                .image(texture.id(), image, uv, egui::Color32::WHITE);
        }
        if response.clicked() {
            toggle = true;
        }
        if ui.is_enabled()
            && !super::text_input_active(ui.ctx())
            && !ui.memory(|m| m.any_popup_open())
            && ui.input(|i| i.focused)
        {
            toggle |= ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space));
            if !self.is_playing() {
                step += ui.input_mut(|i| {
                    i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) as isize
                        - i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                            as isize
                });
            }
        }
        self.subtitle_apply_transport(ui, path, toggle, step);
        if self.playback.is_some()
            || self.index_job.is_some()
            || self.frame_job.is_some()
            || self.wave_job.is_some()
        {
            ui.ctx().request_repaint_after(Duration::from_millis(30));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_targets_cover_both_sides_without_stealing_adjacent_edges_or_the_body() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 50.0));
        let block = egui::Rect::from_min_max(egui::pos2(100.0, 10.0), egui::pos2(200.0, 33.0));
        let start = edge_hit_rect(block, 100.0, 1, None, clip);
        let end = edge_hit_rect(block, 200.0, 2, None, clip);
        assert!(start.contains(egui::pos2(95.0, 20.0)));
        assert!(start.contains(egui::pos2(105.0, 20.0)));
        assert!(end.contains(egui::pos2(195.0, 20.0)));
        assert!(end.contains(egui::pos2(205.0, 20.0)));
        assert!(!start.contains(block.center()));
        assert!(!end.contains(block.center()));
        let next = egui::Rect::from_min_max(egui::pos2(204.0, 10.0), egui::pos2(304.0, 33.0));
        let end = edge_hit_rect(block, 200.0, 2, Some(204.0), clip);
        let next_start = edge_hit_rect(next, 204.0, 1, Some(200.0), clip);
        assert!(end.right() <= next_start.left());
        let short = egui::Rect::from_min_max(egui::pos2(100.0, 10.0), egui::pos2(108.0, 33.0));
        assert!(
            edge_hit_rect(short, 100.0, 1, None, clip).right()
                < edge_hit_rect(short, 108.0, 2, None, clip).left()
        );
        let tight = egui::Rect::from_min_max(egui::pos2(99.0, 15.0), egui::pos2(201.0, 30.0));
        assert!(tight.contains_rect(edge_hit_rect(block, 100.0, 1, None, tight)));
    }
    #[test]
    fn navigation_gap_survives_short_viewports_and_overflowing_timeline_content() {
        for height in [85.0, 110.0, 165.0] {
            let ctx = egui::Context::default();
            let mut geometry = None;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(320.0, height),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            ui.label("Timeline");
                            let mut offset = 4.0;
                            let bar = timeline_navigation(ui, &mut offset, 1.0, 10.0).unwrap();
                            let area = timeline_scroll_area(ui.available_height()).show(ui, |ui| {
                                ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 400.0),
                                    egui::Sense::hover(),
                                );
                            });
                            geometry = Some((bar, area.inner_rect));
                        });
                },
            );
            let (bar, graph) = geometry.unwrap();
            assert!(bar.top() - graph.bottom() >= 7.9);
            assert!(bar.bottom() <= height + 0.1);
            assert!((bar.height() - 14.0).abs() < 0.1);
        }
    }
    #[test]
    fn reopening_original_after_save_as_keeps_both_sessions() {
        let root = std::env::temp_dir().join(format!(
            "uyv-subtitle-sessions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let first = root.join("first.srt");
        let second = root.join("second.srt");
        let content = "1\n00:00:00,000 --> 00:00:01,000\noriginal\n\n2\n00:00:02,000 --> 00:00:03,000\nsecond\n";
        std::fs::write(&first, content).unwrap();
        std::fs::write(&second, content).unwrap();
        let ctx = egui::Context::default();
        let preview = Preview::default();
        open(&ctx, &first, &root.join("video.mp4"), &preview).unwrap();
        let original_id = EDITORS.with(|editors| {
            let mut editors = editors.borrow_mut();
            let editor = &mut editors[0];
            let baseline = editor.doc.clone();
            let cues = editor.doc.cues();
            editor.player.position = 1.5;
            editor.search = "original".into();
            editor.select(cues[1], false);
            editor.reveal_cue(cues[1]);
            assert_eq!(editor.active, Some(cues[1]));
            assert_eq!(editor.list_reveal, Some(cues[1]));
            assert_eq!(editor.timeline_reveal, Some(cues[1]));
            assert!(editor.search.is_empty());
            assert_eq!(editor.player.position, 1.5);
            assert_eq!(editor.doc, baseline);
            editor.select(cues[0], true);
            assert_eq!(editor.selected.len(), 2);
            editor.select(cues[0], true);
            assert_eq!(editor.active, Some(cues[1]));
            assert_eq!(editor.selected.len(), 1);
            editor
                .doc
                .shift(&editor.selected.iter().copied().collect::<Vec<_>>(), 100);
            assert_eq!(editor.doc.cue(cues[0]).unwrap().start(), 0);
            assert_eq!(editor.doc.cue(cues[1]).unwrap().start(), 2100);
            editor.doc = baseline;
            editor.select(cues[1], true);
            assert!(editor.selected.is_empty());
            assert_eq!(editor.active, None);
            editor.path = canonical(&second);
            let cue = editor.doc.cues()[0];
            editor
                .doc
                .cue_mut(cue)
                .unwrap()
                .set("Text", "unsaved second".into());
            editor.id
        });
        open(&ctx, &first, &root.join("video.mp4"), &preview).unwrap();
        open(&ctx, &second, &root.join("video.mp4"), &preview).unwrap();
        EDITORS.with(|editors| {
            let mut editors = editors.borrow_mut();
            assert_eq!(editors.len(), 2);
            assert_eq!(editors[0].id, original_id);
            assert_ne!(editors[0].id, editors[1].id);
            let cue = editors[0].doc.cues()[0];
            assert_eq!(
                editors[0].doc.cue(cue).unwrap().get("Text"),
                "unsaved second"
            );
            editors.clear();
        });
        std::fs::remove_file(first).unwrap();
        std::fs::remove_file(second).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
