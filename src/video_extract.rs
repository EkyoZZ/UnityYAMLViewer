use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc},
    time::Duration,
};
use eframe::egui;
use serde_json::Value;
use crate::{localization::tr, media_tools, native_window, network_video::Track};

const WINDOW_ID: &str = "video_extract";
const ACTION_GREEN: egui::Color32 = egui::Color32::from_rgb(20, 125, 55);
const MIN_CONTENT_WIDTH: f32 = 504.0;
const MIN_CONTENT_HEIGHT: f32 = 194.0;
const SESSION_BROWSERS: [(&str, &str); 8] = [
    ("Chrome", "chrome"), ("Edge", "edge"), ("Firefox", "firefox"), ("Brave", "brave"),
    ("Chromium", "chromium"), ("Opera", "opera"), ("Vivaldi", "vivaldi"), ("Whale", "whale"),
];
const COOKIE_FILE: usize = SESSION_BROWSERS.len();
const COOKIE_HELP: &str = "https://github.com/yt-dlp/yt-dlp/wiki/FAQ#how-do-i-pass-cookies-to-yt-dlp";

fn cookie_export_page(prog_id: &str) -> Option<&'static str> {
    let id = prog_id.to_ascii_lowercase();
    if id == "chromehtml" || id.starts_with("chromehtml.") || id.starts_with("msedgehtm") {
        Some("https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc")
    } else if id.starts_with("firefoxurl") || id.starts_with("firefoxhtml") {
        Some("https://addons.mozilla.org/firefox/addon/cookies-txt/")
    } else {
        None
    }
}

fn available_session_browsers(names: &[String]) -> impl Iterator<Item = usize> + '_ {
    SESSION_BROWSERS.iter().enumerate().filter_map(|(index, (label, _))| {
        let registered_name = match *label {
            "Chrome" => "Google Chrome",
            "Edge" => "Microsoft Edge",
            "Firefox" => "Mozilla Firefox",
            "Brave" => "Brave Browser",
            "Opera" => "Opera Stable",
            "Whale" => "Naver Whale",
            _ => label,
        };
        names.iter().any(|name| name.eq_ignore_ascii_case(label) || name.eq_ignore_ascii_case(registered_name))
            .then_some(index)
    })
}

fn action_width(ui: &egui::Ui, label: &str, minimum: f32) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = ui.painter().layout_no_wrap(label.to_owned(), font, ui.visuals().text_color()).size().x.ceil();
    (text_width + 2.0 * ui.spacing().button_padding.x).max(minimum)
}

fn action_button(label: &str, color: egui::Color32) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE))
        .fill(color).stroke(egui::Stroke::new(1.0_f32, color))
}

#[derive(Default)]
pub struct Window {
    open: bool,
    autoplay: bool,
    components_ready: Option<bool>,
    component_probe: Option<mpsc::Receiver<bool>>,
    component_window_open: bool,
    input: String,
    browser_session: bool,
    browser: usize,
    cookie_file: Option<PathBuf>,
    detected_browsers: Vec<String>,
    browser_probe: Option<mpsc::Receiver<(Vec<String>, Option<&'static str>)>>,
    cookie_export_page: Option<&'static str>,
    browsers_checked: bool,
    result: Option<Media>,
    video: usize,
    audio: usize,
    job: Option<mpsc::Receiver<Result<Outcome, String>>>,
    transfer_progress: Option<Arc<Mutex<TransferProgress>>>,
    transfer_cancel: Option<Arc<AtomicBool>>,
    error: Option<String>,
}

struct Format {
    input: Option<crate::network_video::Input>,
    language: String,
    id: String,
    label: String,
    extension: String,
    size: Option<u64>,
    video_only: bool,
    has_audio: bool,
}

struct Media {
    tool: PathBuf,
    url: String,
    browser_session: Option<Session>,
    title: String,
    video: Vec<Format>,
    audio: Vec<Format>,
}

enum Outcome { Analyzed(Media), Saved { path: PathBuf, record_error: Option<String> } }

#[derive(Clone, Copy, Default)]
enum TransferProgress {
    #[default]
    Preparing,
    Downloading { bytes: u64, total: Option<u64>, track: Option<(usize, usize)> },
    Processing,
}

impl TransferProgress {
    fn show(self, ui: &mut egui::Ui, cancel: &AtomicBool) {
        match self {
            Self::Preparing | Self::Processing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr(if matches!(self, Self::Preparing) { "准备下载…" } else { "正在处理音视频…" }));
                    cancel_download_button(ui, cancel, matches!(self, Self::Preparing));
                });
            }
            Self::Downloading { bytes, total, track } => {
                let label = match track {
                    Some((1, 2)) => format!("{} · 1/2", tr("正在下载视频…")),
                    Some((2, 2)) => format!("{} · 2/2", tr("正在下载音频…")),
                    _ => tr("正在下载视频…").to_owned(),
                };
                if let Some(total) = total {
                    ui.label(format!("{label} · {} / {}", crate::app_cache::format_size(bytes), crate::app_cache::format_size(total)));
                    ui.horizontal(|ui| {
                        let cancel_label = tr(if cancel.load(Ordering::Relaxed) { "正在取消…" } else { "取消" });
                        let button_width = action_width(ui, cancel_label, 80.0);
                        let bar_width = (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(1.0);
                        ui.add(egui::ProgressBar::new((bytes as f64 / total as f64).clamp(0.0, 1.0) as f32)
                            .desired_width(bar_width).show_percentage());
                        cancel_download_button(ui, cancel, true);
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("{label} · {}", crate::app_cache::format_size(bytes)));
                        cancel_download_button(ui, cancel, true);
                    });
                }
            }
        }
    }
}

fn cancel_download_button(ui: &mut egui::Ui, cancel: &AtomicBool, enabled: bool) {
    let cancelling = cancel.load(Ordering::Relaxed);
    let label = tr(if cancelling { "正在取消…" } else { "取消" });
    let button = egui::Button::new(label).min_size(egui::vec2(action_width(ui, label, 80.0), 0.0));
    if ui.add_enabled(enabled && !cancelling, button).clicked() {
        cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone)]
enum Session { Browser(&'static str), CookieFile(PathBuf) }

impl Window {
    pub fn open(&mut self, ctx: &egui::Context) {
        self.autoplay = false;
        native_window::wake(ctx, egui::Id::new(WINDOW_ID));
        self.open = true;
    }

    pub fn busy(&self) -> bool { self.job.is_some() }

    pub fn analyze_and_play(&mut self, input: String, ctx: &egui::Context) -> bool {
        if self.busy() { return false; }
        self.open(ctx);
        self.input = input;
        self.analyze();
        self.autoplay = self.job.is_some();
        true
    }

    pub fn show(&mut self, ctx: &egui::Context, component_window_open: bool) -> Option<crate::network_video::Source> {
        if let Some(job) = &self.job {
            match job.try_recv() {
                Ok(result) => {
                    let transfer = self.transfer_progress.is_some();
                    self.job = None;
                    self.transfer_progress = None;
                    self.transfer_cancel = None;
                    match result {
                        Ok(Outcome::Analyzed(media)) => {
                            self.result = Some(media);
                            self.video = 0;
                            self.audio = 0;
                            if std::mem::take(&mut self.autoplay) && self.open {
                                self.open = false;
                                return self.result.as_ref().map(|media| media.source(0, 0));
                            }
                        }
                        Ok(Outcome::Saved { record_error, .. }) => {
                            if let Some(error) = record_error {
                                self.error = Some(format!("{} · {error}", tr("视频下载记录保存失败")));
                            }
                        }
                        Err(error) => { self.autoplay = false; self.error = Some(error); if !transfer { self.open = true; } }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.job = None;
                    self.autoplay = false;
                    self.transfer_progress = None;
                    self.transfer_cancel = None;
                    self.error = Some(tr("网络加载任务已中断").into());
                }
            }
        }
        let components_changed = self.component_window_open && !component_window_open;
        self.component_window_open = component_window_open;
        if !self.open || components_changed {
            self.components_ready = None;
            self.component_probe = None;
        }
        if !self.open { return None; }
        if let Some(probe) = &self.component_probe {
            match probe.try_recv() {
                Ok(ready) => {
                    self.components_ready = Some(ready);
                    self.component_probe = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.components_ready = Some(false);
                    self.component_probe = None;
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            }
        }
        if self.components_ready.is_none() && self.component_probe.is_none() {
            let (sender, receiver) = mpsc::channel();
            self.component_probe = Some(receiver);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let ready = ["yt-dlp", "ffmpeg", "ffprobe"].into_iter().all(|tool| media_tools::find_tool(tool).is_some());
                let _ = sender.send(ready);
                ctx.request_repaint();
            });
        }
        if let Some(probe) = &self.browser_probe {
            match probe.try_recv() {
                Ok((browsers, cookie_export_page)) => {
                    self.detected_browsers = browsers;
                    self.cookie_export_page = cookie_export_page;
                    let mut available = available_session_browsers(&self.detected_browsers);
                    let first = available.next();
                    if first != Some(self.browser) && !available.any(|index| index == self.browser) {
                        self.browser = first.unwrap_or(COOKIE_FILE);
                    }
                    self.browser_probe = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.browser = COOKIE_FILE;
                    self.browser_probe = None;
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            }
        }
        let mut open = self.open;
        let mut saved_playback = None;
        native_window::Window::new(tr("视频提取")).id(egui::Id::new(WINDOW_ID))
            .open(&mut open)
            .default_width(MIN_CONTENT_WIDTH).min_width(MIN_CONTENT_WIDTH)
            .default_height(MIN_CONTENT_HEIGHT).min_height(MIN_CONTENT_HEIGHT)
            .fit_height().show(ctx, |ui| {
                let busy = self.busy();
                let mut has_download = false;
                ui.add_enabled_ui(!busy, |ui| {
                    ui.label(tr("粘贴视频链接或完整分享文字。需要 yt-dlp，下载与合并需要 FFmpeg。"));
                    ui.horizontal_wrapped(|ui| {
                        if self.components_ready != Some(true) && ui.button(tr("第三方支持")).clicked() {
                            crate::third_party::request_open(ui.ctx());
                        }
                        let mut session_changed = ui.checkbox(&mut self.browser_session, tr("使用浏览器会话"))
                            .on_hover_text(tr("启用后，分析、播放和下载会读取所选浏览器或文件中的 Cookie。"))
                            .changed();
                        if self.browser_session {
                            if !self.browsers_checked || self.browser_probe.is_some() {
                                ui.small(tr("正在检测…"));
                            } else {
                                let label = SESSION_BROWSERS.get(self.browser).map_or_else(|| tr("Cookie 文件"), |(label, _)| *label);
                                let previous = self.browser;
                                egui::ComboBox::from_id_salt("extract_session_browser")
                                    .selected_text(label).height(120.0).show_ui(ui, |ui| {
                                        for index in available_session_browsers(&self.detected_browsers) {
                                            ui.selectable_value(&mut self.browser, index, SESSION_BROWSERS[index].0);
                                        }
                                        ui.selectable_value(&mut self.browser, COOKIE_FILE, tr("Cookie 文件"));
                                    });
                                session_changed |= self.browser != previous;
                            }
                        }
                        if session_changed {
                            self.result = None;
                            self.error = None;
                        }
                    });
                    if self.browser_session {
                        if !self.browsers_checked {
                            self.browsers_checked = true;
                            let (sender, receiver) = mpsc::channel();
                            self.browser_probe = Some(receiver);
                            std::thread::spawn(move || { let _ = sender.send(registered_browsers()); });
                            ui.ctx().request_repaint_after(Duration::from_millis(100));
                        }
                        if !self.detected_browsers.is_empty() {
                            ui.small(format!("{} · {}", tr("Windows 登记的浏览器"), self.detected_browsers.join(", ")));
                        }
                        if self.browser == COOKIE_FILE {
                            ui.horizontal_wrapped(|ui| {
                                if let Some(url) = self.cookie_export_page {
                                    ui.hyperlink_to(tr("安装 Cookie 导出扩展"), url);
                                } else {
                                    ui.hyperlink_to(tr("Cookie 文件获取说明"), COOKIE_HELP);
                                }
                                if ui.button(tr("选择 Cookie 文件")).clicked() {
                                    let mut dialog = rfd::FileDialog::new().add_filter("Cookie", &["txt"]);
                                    let directory = self.cookie_file.as_deref().and_then(Path::parent).map(Path::to_owned)
                                        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Downloads")));
                                    if let Some(directory) = directory.filter(|path| path.is_dir()) {
                                        dialog = dialog.set_directory(directory);
                                    }
                                    if let Some(path) = dialog.pick_file() {
                                        self.cookie_file = Some(path);
                                        self.result = None;
                                        self.error = None;
                                    }
                                }
                                if let Some(path) = &self.cookie_file {
                                    ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                                }
                            });
                            ui.small(tr("先从浏览器导出目标网站的 Cookie 为 Netscape 格式的 .txt 文件，再在这里选择导出的文件。"));
                            if self.cookie_export_page.is_some() {
                                ui.hyperlink_to(tr("Cookie 文件获取说明"), COOKIE_HELP);
                            }
                        }
                    }
                    ui.horizontal(|ui| {
                        const ROW_HEIGHT: f32 = 32.0;
                        let label = tr("分析");
                        let button_width = action_width(ui, label, 76.0);
                        let input_width = (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(32.0);
                        let input_id = ui.make_persistent_id("extract_url_input");
                        crate::theme::prepare_url_paste(ui, input_id);
                        let input = egui::TextEdit::singleline(&mut self.input)
                            .id(input_id)
                            .vertical_align(egui::Align::Center)
                            .min_size(egui::vec2(0.0, ROW_HEIGHT));
                        let response = ui.add_sized([input_width, ROW_HEIGHT], input);
                        let pasted = crate::theme::paste_replace_menu(&response, &mut self.input);
                        if response.changed() || pasted {
                            self.result = None;
                            self.error = None;
                        }
                        let detecting = self.browser_session && self.browser_probe.is_some();
                        ui.add_enabled_ui(!detecting, |ui| {
                            if ui.add_sized([button_width, ROW_HEIGHT], action_button(label, ACTION_GREEN)).clicked() { self.analyze(); }
                        });
                    });
                    if let Some(media) = &self.result {
                        ui.separator();
                        ui.add(egui::Label::new(&media.title).wrap());
                        if media.video.len() > 1 { ui.horizontal_wrapped(|ui| {
                            ui.label(tr("视频"));
                            egui::ComboBox::from_id_salt("extract_video").selected_text(&media.video[self.video].label)
                                .show_ui(ui, |ui| for (index, format) in media.video.iter().enumerate() {
                                    ui.selectable_value(&mut self.video, index, &format.label);
                                });
                        }); }
                        if media.video[self.video].video_only {
                            if let Some(audio) = media.audio.get(self.audio) {
                                if media.audio.len() > 1 { ui.horizontal_wrapped(|ui| {
                                    ui.label(tr("音轨"));
                                    egui::ComboBox::from_id_salt("extract_audio").selected_text(&audio.label)
                                        .show_ui(ui, |ui| for (index, format) in media.audio.iter().enumerate() {
                                            ui.selectable_value(&mut self.audio, index, &format.label);
                                        });
                                }); }
                            } else {
                                ui.label(tr("没有可用音轨，结果可能无声。"));
                            }
                        }
                        let video = &media.video[self.video];
                        let audio = if video.video_only { media.audio.get(self.audio) } else { None };
                        let total = combined_size(video.size, audio.map(|track| track.size));
                        let total = total.map(crate::app_cache::format_size).unwrap_or_else(|| tr("大小未知").to_owned());
                        let selection = media.source(self.video, self.audio).selection();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{} · {total}", tr("预计总大小")));
                            if selection.fully_cached() {
                                ui.label(format!("[{}]", tr("本地已缓存")));
                            }
                        });
                        let recorded = selection.download_record();
                        has_download = recorded.as_ref().is_some_and(|path| path.is_file());
                        if let Some(path) = recorded {
                            if has_download {
                                ui.label(tr("已下载"));
                                if ui.link(path.display().to_string()).on_hover_text(tr("打开文件所在目录")).clicked() {
                                    if let Err(error) = crate::asset_viewer::reveal_file(&path) {
                                        self.error = Some(error.to_string());
                                    }
                                }
                            } else {
                                ui.add(egui::Label::new(egui::RichText::new(path.display().to_string())
                                    .color(ui.visuals().error_fg_color)).wrap())
                                    .on_hover_text(tr("原路径已失效"));
                            }
                        }
                        if !has_download && ui.small_button(tr("关联已有视频…")).clicked() {
                            let mut dialog = rfd::FileDialog::new().add_filter(tr("视频"), &["mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpeg", "mpg", "ts", "mts", "m2ts", "3gp"]);
                            if let Some(home) = std::env::var_os("USERPROFILE") {
                                dialog = dialog.set_directory(PathBuf::from(home).join("Downloads"));
                            }
                            if let Some(path) = dialog.pick_file().filter(|path| path.is_file()) {
                                let _guard = crate::app_cache::mutation_guard();
                                match selection.remember(Some(path)) {
                                    Ok(()) => { has_download = true; self.error = None; }
                                    Err(error) => self.error = Some(format!("{} · {error}", tr("视频下载记录保存失败"))),
                                }
                            }
                        }
                    }
                });
                if self.result.is_none() && (busy || self.error.is_some()) { ui.separator(); }
                if let Some((progress, cancel)) = self.transfer_progress.as_ref().zip(self.transfer_cancel.as_ref()) {
                    let progress = *progress.lock().unwrap();
                    progress.show(ui, cancel);
                } else if busy {
                    ui.horizontal(|ui| { ui.spinner(); ui.label(tr("正在分析链接…")); });
                }
                if let Some(error) = &self.error {
                    ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().error_fg_color))
                        .wrap().selectable(true));
                }
                if self.result.is_some() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let download = tr(if has_download { "已下载" } else { "下载视频" });
                        let play = tr("播放");
                        let download_width = action_width(ui, download, 112.0);
                        let play_width = action_width(ui, play, 112.0);
                        let width = download_width + play_width + ui.spacing().item_spacing.x;
                        ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
                        if ui.add_enabled_ui(!self.busy() && !has_download, |ui| ui.add_sized([download_width, 40.0], action_button(download, crate::theme::BLUE))).inner.clicked() {
                            self.transfer(ui.ctx());
                        }
                        if ui.add_sized([play_width, 40.0], action_button(play, ACTION_GREEN)).clicked() {
                            saved_playback = self.result.as_ref().map(|media| media.source(self.video, self.audio));
                        }
                    });
                }
            });
        self.open = open && saved_playback.is_none();
        if !self.open { self.autoplay = false; }
        saved_playback
    }


    fn analyze(&mut self) {
        let Some((url, link_title)) = extraction_source(&self.input) else {
            self.error = Some(tr("URL 格式无效").into());
            return;
        };
        self.result = None;
        self.error = None;
        let browser_session = if !self.browser_session {
            None
        } else if let Some((_, browser)) = SESSION_BROWSERS.get(self.browser) {
            Some(Session::Browser(browser))
        } else if let Some(path) = self.cookie_file.clone() {
            Some(Session::CookieFile(path))
        } else {
            self.error = Some(tr("请选择 Cookie 文件。").into());
            return;
        };
        self.job = Some(background(move || {
            let tool = media_tools::find_tool("yt-dlp")
                .ok_or_else(|| tr("找不到 yt-dlp，请在第三方支持中安装或设置路径。").to_owned())?;
            let (mut command, _cookies) = extractor(&tool, browser_session.as_ref())?;
            let output = run(command.args(["--skip-download", "--dump-single-json", "--", &url]))?;
            let json: Value = serde_json::from_slice(&output).map_err(|error| error.to_string())?;
            let entry = json["entries"].as_array().and_then(|entries| entries.iter().find(|entry| !entry.is_null())).unwrap_or(&json);
            if entry["is_live"] == true { return Err(tr("视频提取暂不支持直播。").into()); }
            let title = link_title.or_else(|| entry["title"].as_str().and_then(crate::remote_file::clean_title))
                .unwrap_or_else(|| tr("未命名").to_owned());
            let (video, audio) = formats(entry);
            if video.is_empty() { return Err(tr("没有可用的视频格式。").into()); }
            Ok(Outcome::Analyzed(Media { tool, url, browser_session, title, video, audio }))
        }));
    }

    fn transfer(&mut self, ctx: &egui::Context) {
        if self.busy() { return; }
        let Some(media) = &self.result else { return; };
        let video = &media.video[self.video];
        let audio = video.video_only.then(|| media.audio.get(self.audio)).flatten();
        let selector = match audio {
            Some(audio) => format!("{}+{}", video.id, audio.id),
            None => video.id.clone(),
        };
        let extension = output_extension(&video.extension, audio.map(|track| track.extension.as_str()));
        let selection = media.source(self.video, self.audio).selection();
        if selection.downloaded().is_some() { return; }
        let title = filename(&media.title);
        let Some(mut destination) = rfd::FileDialog::new().add_filter(extension.to_ascii_uppercase(), &[extension])
            .set_file_name(format!("{title}.{extension}")).save_file() else { return; };
        destination.set_extension(extension);
        if destination.exists() {
            self.error = Some(tr("目标文件已存在，请使用新文件名。").into());
            return;
        }
        let tool = media.tool.clone();
        let url = media.url.clone();
        let verify_audio = audio.is_some() || video.has_audio;
        self.error = None;
        let browser_session = media.browser_session.clone();
        let progress = Arc::new(Mutex::new(TransferProgress::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        self.transfer_progress = Some(progress.clone());
        self.transfer_cancel = Some(cancel.clone());
        use crate::background_tasks::{Outcome as TaskOutcome, Progress, Target, Task};
        let task_progress = progress.clone();
        let task_cancel = cancel.clone();
        let cancel_button = cancel.clone();
        let cancel_progress = progress.clone();
        let task = Task::start(ctx, "下载视频", destination.file_name().unwrap_or_default().to_string_lossy().into_owned(), move || {
            let state = *task_progress.lock().unwrap();
            let cancelling = task_cancel.load(Ordering::Relaxed);
            let (stage, fraction) = match state {
                TransferProgress::Preparing => (tr("准备下载…"), None),
                TransferProgress::Processing => (tr("正在处理音视频…"), None),
                TransferProgress::Downloading { bytes, total, track } => (
                    tr(if matches!(track, Some((2, 2))) { "正在下载音频…" } else { "正在下载视频…" }),
                    total.filter(|total| *total > 0).map(|total| (bytes as f64 / total as f64) as f32),
                ),
            };
            Progress {
                stage: if cancelling { tr("正在取消…") } else { stage }.into(),
                fraction,
                cancellable: !cancelling && !matches!(state, TransferProgress::Processing),
            }
        }, Some(Box::new(move || {
            if !matches!(*cancel_progress.lock().unwrap(), TransferProgress::Processing) {
                cancel_button.store(true, Ordering::Relaxed);
            }
        })));
        self.job = Some(background(move || {
            let result = transfer(&tool, &url, browser_session.as_ref(), &selector, extension, destination, selection, verify_audio, &progress, &cancel);
            let outcome = match &result {
                Ok(Outcome::Saved { record_error: Some(error), .. }) => TaskOutcome::Failed(format!("{} · {error}", tr("视频下载记录保存失败"))),
                Ok(Outcome::Saved { path, .. }) => TaskOutcome::Complete(Some(Target::File(path.clone()))),
                Err(_) if cancel.load(Ordering::Relaxed) => TaskOutcome::Cancelled,
                Err(error) => TaskOutcome::Failed(error.clone()),
                Ok(_) => TaskOutcome::Complete(None),
            };
            task.finish(outcome);
            result
        }));
    }
}

pub(crate) fn extraction_source(input: &str) -> Option<(String, Option<String>)> {
    let stream = crate::remote_file::stream_source(input.trim());
    let (path, title) = match stream {
        Some(stream) => stream,
        None => {
            let url = share_url(input)?;
            match crate::remote_file::stream_source(&url) {
                Some(stream) => stream,
                None => return Some((url, None)),
            }
        }
    };
    let url = path.to_str()?;
    let Some((base, query)) = url.split_once('?') else {
        return Some((url.to_owned(), title));
    };
    // Keep the original encoding of the signed query parameters.
    let mut parameter = reqwest::Url::parse(url).ok()?;
    let query = query.split('&').filter(|pair| {
        parameter.set_query(Some(pair));
        !parameter.query_pairs().any(|(key, _)| key == "name")
    }).collect::<Vec<_>>().join("&");
    let url = if query.is_empty() { base.to_owned() } else { format!("{base}?{query}") };
    Some((url, title))
}

fn share_url(input: &str) -> Option<String> {
    let input = input.replace("https\\://", "https://").replace("http\\://", "http://");
    input.split_whitespace().find_map(|word| {
        let start = [word.find("https://"), word.find("http://")].into_iter().flatten().min()?;
        let candidate = word[start..].split(['<', '>', '"', '“', '”', '。', '，', '、']).next()?;
        let url = reqwest::Url::parse(candidate.trim_end_matches(')')).ok()?;
        let host = url.host_str()?;
        if matches!(host, "www.douyin.com" | "douyin.com") && url.path().trim_end_matches('/') == "/jingxuan" {
            if let Some((_, id)) = url.query_pairs().find(|(key, value)| {
                key == "modal_id" && !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
            }) {
                return Some(format!("https://www.douyin.com/video/{id}"));
            }
        }
        Some(url.to_string())
    })
}

fn background(work: impl FnOnce() -> Result<Outcome, String> + Send + 'static) -> mpsc::Receiver<Result<Outcome, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || { let _ = sender.send(work()); });
    receiver
}

#[cfg(windows)]
fn registered_browsers() -> (Vec<String>, Option<&'static str>) {
    let script = r#"
        [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
        $choice = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice' -ErrorAction SilentlyContinue
        if ($choice.ProgId) { 'UYV-DEFAULT:' + $choice.ProgId }
        foreach ($root in @('HKCU:\Software\Clients\StartMenuInternet',
                            'HKLM:\Software\Clients\StartMenuInternet',
                            'HKLM:\Software\WOW6432Node\Clients\StartMenuInternet')) {
            foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
                $name = $key.GetValue('')
                if ($name -is [string] -and $name.Trim()) { $name.Trim() }
                else { $key.PSChildName }
            }
        }
    "#;
    let Ok(output) = media_tools::command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script]).output() else { return (Vec::new(), None); };
    let output = String::from_utf8_lossy(&output.stdout);
    let export_page = output.lines().find_map(|line| line.strip_prefix("UYV-DEFAULT:")).and_then(cookie_export_page);
    let mut names: Vec<_> = output.lines().filter(|line| !line.starts_with("UYV-DEFAULT:"))
        .filter_map(crate::remote_file::clean_title).collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    (names, export_page)
}

#[cfg(not(windows))]
fn registered_browsers() -> (Vec<String>, Option<&'static str>) { (Vec::new(), None) }

struct CookieCopy(PathBuf);

impl CookieCopy {
    fn open(source: &Path) -> Result<Self, String> {
        const LIMIT: u64 = 8 * 1024 * 1024;
        let mut contents = Vec::new();
        fs::File::open(source).and_then(|file| file.take(LIMIT + 1).read_to_end(&mut contents))
            .map_err(|error| error.to_string())?;
        if contents.len() as u64 > LIMIT || !(contents.starts_with(b"# Netscape HTTP Cookie File")
            || contents.starts_with(b"# HTTP Cookie File")) {
            return Err(tr("Cookie 文件格式无效或超过 8 MiB。").into());
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("unityyaml-cookie-{}-{}.txt", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).map_err(|error| error.to_string())?;
        let copy = Self(path);
        let written = file.write_all(&contents);
        drop(file);
        written.map_err(|error| error.to_string())?;
        Ok(copy)
    }
}

impl Drop for CookieCopy {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

fn extractor(tool: &Path, browser_session: Option<&Session>) -> Result<(Command, Option<CookieCopy>), String> {
    let mut command = media_tools::command(tool);
    command.args(["--ignore-config", "--no-update", "--no-plugin-dirs", "--no-cache-dir", "--no-playlist",
        "--playlist-items", "1", "--socket-timeout", "20", "--retries", "1", "--fragment-retries", "1",
        "--abort-on-unavailable-fragments", "--encoding", "utf-8", "--no-progress", "--js-runtimes", "node"]);
    let cookies = match browser_session {
        Some(Session::Browser(browser)) => {
            command.args(["--cookies-from-browser", browser]);
            None
        }
        Some(Session::CookieFile(path)) => {
            let copy = CookieCopy::open(path)?;
            command.arg("--cookies").arg(&copy.0);
            Some(copy)
        }
        None => None,
    };
    Ok((command, cookies))
}

fn run(command: &mut Command) -> Result<Vec<u8>, String> {
    let output = command.stdin(Stdio::null()).output().map_err(|error| error.to_string())?;
    if output.status.success() { return Ok(output.stdout); }
    Err(command_error(&output.stderr))
}

fn command_error(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let summary = if stderr.contains("Could not copy Chrome cookie database") {
        tr("无法读取浏览器的 Cookie 数据库。公开视频可关闭浏览器会话后重试。需要会话时，请完全退出所选浏览器后重试，或使用 Cookie 文件。")
    } else if stderr.contains("Failed to decrypt with DPAPI") {
        tr("无法解密所选浏览器的 Cookie。可改用能播放该视频的 Firefox 会话，或选择仅含目标站点 Cookie 的文件。")
    } else if stderr.contains("[Douyin]") && stderr.contains("Fresh cookies") {
        tr("抖音要求有效的浏览器会话，暂时无法分析此视频。")
    } else {
        tr("网络加载失败")
    };
    let mut lines: Vec<_> = stderr.trim().lines().collect();
    lines.dedup();
    format!("{summary}\n{}", lines.join("\n").chars().take(2000).collect::<String>())
}

fn run_transfer(command: &mut Command, selector: &str, progress: &Arc<Mutex<TransferProgress>>, cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) { return Err(tr("下载已取消。").into()); }
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let diagnostics = std::thread::spawn(move || {
        let mut limited = stderr.take(8192);
        let mut bytes = Vec::new();
        let _ = limited.read_to_end(&mut bytes);
        let _ = io::copy(&mut limited.into_inner(), &mut io::sink());
        bytes
    });
    let reader = {
        let progress = progress.clone();
        let selector = selector.to_owned();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(state) = parse_transfer_progress(&line, &selector) {
                    *progress.lock().unwrap() = state;
                }
            }
        })
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if !cancel.load(Ordering::Relaxed) => std::thread::sleep(Duration::from_millis(100)),
            result => {
                #[cfg(windows)]
                let _ = media_tools::command("taskkill.exe").args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
                let _ = child.kill();
                let _ = child.wait();
                break Err(result.err().map_or_else(|| tr("下载已取消。").to_owned(), |error| error.to_string()));
            }
        }
    };
    let _ = reader.join();
    let diagnostics = diagnostics.join().unwrap_or_default();
    let status = status?;
    if cancel.load(Ordering::Relaxed) { return Err(tr("下载已取消。").into()); }
    if !status.success() { return Err(command_error(&diagnostics)); }
    *progress.lock().unwrap() = TransferProgress::Processing;
    Ok(())
}

fn parse_transfer_progress(line: &str, selector: &str) -> Option<TransferProgress> {
    if line == "uyv-processing" { return Some(TransferProgress::Processing); }
    let mut fields = line.strip_prefix("uyv-download\t")?.split('\t');
    let format = fields.next()?;
    let bytes = fields.next()?.parse().ok()?;
    let total = fields.next()?.parse::<u64>().ok().filter(|size| *size > 0)
        .or_else(|| fields.next()?.parse::<u64>().ok().filter(|size| *size > 0));
    let track = selector.split('+').position(|id| id == format)
        .map(|index| (index + 1, selector.split('+').count()));
    Some(TransferProgress::Downloading { bytes, total, track })
}

impl Media {
    fn source(&self, selected_video: usize, selected_audio: usize) -> crate::network_video::Source {
        use crate::network_video::{Format, Source};
        Source {
            url: self.url.clone(), title: self.title.clone(), selected_video, selected_audio,
            video: self.video.iter().map(|format| Format {
                id: format.id.clone(), label: format.label.clone(), input: format.input.clone(), language: format.language.clone(),
                separate_audio: format.video_only,
            }).collect(),
            audio: self.audio.iter().map(|format| Format {
                id: format.id.clone(), label: format.label.clone(), input: format.input.clone(), language: format.language.clone(),
                separate_audio: false,
            }).collect(),
        }
    }
}

fn playback_input(entry: &Value, format: &Value) -> Option<crate::network_video::Input> {
    let url = reqwest::Url::parse(format["url"].as_str()?).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() { return None; }
    let protocol = format["protocol"].as_str().unwrap_or(url.scheme());
    if !matches!(protocol, "http" | "https" | "m3u8" | "m3u8_native") { return None; }
    let mut input = crate::network_video::Input::new(PathBuf::from(url.as_str()));
    input.hls |= protocol.starts_with("m3u8");
    // yt-dlp deliberately keeps scoped cookies separate from http_headers.
    input.cookies = playback_cookies(format["cookies"].as_str()
        .or_else(|| entry["cookies"].as_str()).unwrap_or_default(), &url);
    let mut headers = std::collections::BTreeMap::new();
    for values in [&entry["http_headers"], &format["http_headers"]] {
        if let Some(values) = values.as_object() {
            for (name, value) in values {
                if reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err() { continue; }
                if let Some(value) = value.as_str().filter(|value| reqwest::header::HeaderValue::from_str(value).is_ok()) {
                    headers.insert(name.to_ascii_lowercase(), value);
                }
            }
        }
    }
    for (name, value) in headers {
        input.headers.push_str(&format!("{name}: {value}\r\n"));
    }
    Some(input)
}

/// Convert yt-dlp's serialized cookies into FFmpeg's scoped Set-Cookie lines.
/// Never flatten these into a Cookie header, which would lose redirect scoping.
fn playback_cookies(serialized: &str, url: &reqwest::Url) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_secs();
    let host = url.host_str().unwrap_or_default();
    let mut result = String::new();
    let mut cookie = Vec::<(&str, &str)>::new();
    let mut secure = false;
    let mut valid = true;
    let flush = |cookie: &[(&str, &str)], secure: bool, valid: bool, result: &mut String| {
        let Some(&(name, encoded_value)) = cookie.first() else { return; };
        let Some(value) = playback_cookie_value(encoded_value) else { return; };
        let attribute = |key: &str| cookie.iter().skip(1)
            .find(|(name, _)| name.eq_ignore_ascii_case(key)).map(|(_, value)| *value);
        let Some(domain) = attribute("domain") else { return; };
        let domain = domain.trim_start_matches('.');
        let Some(path) = attribute("path").filter(|path| path.starts_with('/')) else { return; };
        let domain_matches = host.eq_ignore_ascii_case(domain) || host.to_ascii_lowercase()
            .strip_suffix(&domain.to_ascii_lowercase()).is_some_and(|prefix| prefix.ends_with('.'));
        let path_matches = url.path() == path || url.path().strip_prefix(path)
            .is_some_and(|suffix| path.ends_with('/') || suffix.starts_with('/'));
        let expired = attribute("expires").is_some_and(|value| value.parse::<u64>().map_or(true, |expires| expires <= now));
        if !valid || domain.is_empty() || !domain_matches || !path_matches || expired
            || (secure && url.scheme() != "https") { return; }
        result.push_str(&format!("{name}={value}; domain={domain}; path={path};{}\r\n",
            if secure { " secure;" } else { "" }));
    };
    // Python's SimpleCookie serialization escapes semicolons in quoted values.
    for part in serialized.split(';').map(str::trim).filter(|part| !part.is_empty()) {
        let (name, value) = part.split_once('=').unwrap_or((part, ""));
        let attribute = matches!(name.to_ascii_lowercase().as_str(),
            "domain" | "path" | "expires" | "version" | "secure" | "httponly" | "samesite" | "max-age");
        if !attribute {
            flush(&cookie, secure, valid, &mut result);
            cookie.clear();
            secure = false;
            valid = part.contains('=') && reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_ok();
        }
        valid &= part.bytes().all(|byte| (0x20..=0x7e).contains(&byte));
        if name.eq_ignore_ascii_case("secure") { secure = true; }
        cookie.push((name, value));
    }
    flush(&cookie, secure, valid, &mut result);
    result
}

// Python SimpleCookie quotes values such as base64 tokens containing '='.
// FFmpeg expects the decoded value, including decoded backslash/octal escapes.
fn playback_cookie_value(encoded: &str) -> Option<String> {
    let mut decoded = Vec::new();
    if encoded.starts_with('"') {
        let inner = encoded.strip_prefix('"')?.strip_suffix('"')?;
        let bytes = inner.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            let mut byte = bytes[index];
            if byte == b'\\' {
                index += 1;
                byte = *bytes.get(index)?;
                if (b'0'..=b'3').contains(&byte) && bytes.get(index + 1..index + 3)
                    .is_some_and(|digits| digits.iter().all(|digit| (b'0'..=b'7').contains(digit))) {
                    byte = (byte - b'0') * 64 + (bytes[index + 1] - b'0') * 8 + (bytes[index + 2] - b'0');
                    index += 2;
                }
            }
            decoded.push(byte);
            index += 1;
        }
    } else {
        decoded.extend_from_slice(encoded.as_bytes());
    }
    // Do not allow escaped delimiters to create new Set-Cookie attributes/lines.
    decoded.iter().all(|byte| (0x20..=0x7e).contains(byte) && *byte != b';')
        .then(|| String::from_utf8(decoded).ok()).flatten()
}

fn formats(entry: &Value) -> (Vec<Format>, Vec<Format>) {
    let Some(formats) = entry["formats"].as_array() else { return Default::default(); };
    let mut video = Vec::new();
    let mut audio = Vec::new();
    for format in formats.iter().rev() {
        if format["has_drm"].as_bool() == Some(true) { continue; }
        let Some(id) = format["format_id"].as_str().filter(|id| !id.is_empty() &&
            id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))) else { continue; };
        let has_video = format["vcodec"].as_str().is_some_and(|codec| codec != "none");
        let has_audio = format["acodec"].as_str().is_some_and(|codec| codec != "none");
        let width = format["width"].as_u64();
        let height = format["height"].as_u64();
        let exact = format["filesize"].as_u64().filter(|size| *size > 0);
        let size = exact.or_else(|| format["filesize_approx"].as_u64().filter(|size| *size > 0));
        let size_label = size.map(|size| {
            let prefix = if exact.is_some() { "" } else { "≈ " };
            format!("{prefix}{}", crate::app_cache::format_size(size))
        }).unwrap_or_else(|| tr("大小未知").to_owned());
        if has_video || width.is_some() || height.is_some() {
            let resolution = match (width, height) {
                (Some(w), Some(h)) => format!("{w} × {h}"),
                (_, Some(h)) => format!("{h}p"),
                _ => tr("未知").to_owned(),
            };
            let ext = format["ext"].as_str().unwrap_or("—");
            let note = format["format_note"].as_str().and_then(crate::remote_file::clean_title);
            let video_only = format["acodec"] == "none";
            let kind = tr(if video_only { "仅视频" } else if has_audio { "含音频" } else { "视频" });
            let detail = note.map_or_else(|| kind.to_owned(), |note| format!("{kind} · {note}"));
            let label = format!("{resolution} · {ext} · {detail} · {size_label} · {id}");
            video.push(Format { input: playback_input(entry, format), language: String::new(), id: id.into(), label, extension: ext.into(), size, video_only, has_audio });
        } else if format["vcodec"] == "none" && format["acodec"] != "none" {
            let lang = format["language"].as_str().unwrap_or("—");
            let codec = format["acodec"].as_str().unwrap_or("—");
            let rate = format["abr"].as_f64().map_or_else(|| "—".to_owned(), |rate| format!("{rate:.0} kb/s"));
            let extension = format["ext"].as_str().unwrap_or_default().to_owned();
            audio.push(Format { input: playback_input(entry, format), language: crate::network_video::language_key(lang), id: id.into(), label: format!("{lang} · {codec} · {rate} · {size_label} · {id}"), extension, size, video_only: false, has_audio: true });
        }
    }
    (video, audio)
}

fn combined_size(video: Option<u64>, audio: Option<Option<u64>>) -> Option<u64> {
    video?.checked_add(audio.unwrap_or(Some(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_autoplay_selects_first_video_and_audio_without_downloading() {
        let metadata = serde_json::json!({"formats": [
            {"format_id":"first-video", "url":"https://example.org/video.mp4", "vcodec":"h264", "acodec":"none", "ext":"mp4"},
            {"format_id":"second-video", "url":"https://example.org/other.mp4", "vcodec":"h264", "acodec":"none", "ext":"mp4"},
            {"format_id":"first-audio", "url":"https://example.org/audio.m4a", "vcodec":"none", "acodec":"aac", "ext":"m4a"}
        ]});
        let (video, audio) = formats(&metadata);
        let first_video_id = video[0].id.clone();
        let first_audio_id = audio[0].id.clone();
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(Outcome::Analyzed(Media { tool: PathBuf::new(), url:"https://example.org/watch".into(),
            browser_session:None, title:"Test".into(), video, audio }))).unwrap_or_else(|_| panic!("receiver dropped"));
        let mut window = Window { open:true, autoplay:true, job:Some(receiver), ..Default::default() };
        let source = window.show(&egui::Context::default(), false).expect("automatic playback source");
        assert_eq!(source.video[source.selected_video].id, first_video_id);
        assert_eq!(source.audio[source.selected_audio].id, first_audio_id);
        assert!(source.video[0].separate_audio);
        assert!(window.job.is_none());
        assert!(!window.open && !window.autoplay);
    }

    #[test]
    fn automatic_extraction_does_not_replace_an_active_job() {
        let (_sender, receiver) = mpsc::channel();
        let mut window = Window { input:"existing task".into(), job:Some(receiver), ..Default::default() };
        assert!(!window.analyze_and_play("https://example.org/new".into(), &egui::Context::default()));
        assert_eq!(window.input, "existing task");
        assert!(!window.autoplay);
    }

    #[test]
    fn playback_keeps_format_headers_and_scoped_cookies() {
        let entry = serde_json::json!({"http_headers": {"User-Agent": "base", "Referer": "https://example.org/"},
            "cookies": "wrong=entry; Domain=elsewhere.invalid; Path=/"});
        let format = serde_json::json!({"url": "https://cdn.example.org/video/test.mp4", "protocol": "https",
            "http_headers": {"User-Agent": "format"},
            "cookies": "token=abc; Domain=.example.org; Path=/video; Secure; other=def; Domain=cdn.example.org; Path=/"});
        let input = playback_input(&entry, &format).unwrap();
        assert!(input.headers.contains("user-agent: format\r\n"));
        assert!(input.headers.contains("referer: https://example.org/\r\n"));
        assert!(!input.headers.to_ascii_lowercase().contains("cookie:"));
        assert_eq!(input.cookies, "token=abc; domain=example.org; path=/video; secure;\r\nother=def; domain=cdn.example.org; path=/;\r\n");
        let mut command = std::process::Command::new("ffprobe");
        input.options(&mut command);
        let args: Vec<_> = command.get_args().map(|value| value.to_string_lossy().into_owned()).collect();
        assert!(args.windows(2).any(|pair| pair[0] == "-cookies" && pair[1] == input.cookies));
    }

    #[test]
    fn playback_decodes_python_cookie_values_before_passing_to_ffmpeg() {
        let url = reqwest::Url::parse("https://cdn.example.org/video.mp4").unwrap();
        assert_eq!(playback_cookies(r#"tt_chain_token="abc=="; Domain=.example.org; Path=/; Secure"#, &url),
            "tt_chain_token=abc==; domain=example.org; path=/; secure;\r\n");
        assert_eq!(playback_cookie_value(r#""abc\075\075""#).as_deref(), Some("abc=="));
        assert_eq!(playback_cookie_value(r#""abc\\def""#).as_deref(), Some(r"abc\def"));
        assert!(playback_cookie_value(r#""abc\015\012injected""#).is_none());
        assert!(playback_cookie_value(r#""abc\073path=/""#).is_none());
        assert!(playback_cookie_value(r#""abc\377""#).is_none());
        assert!(playback_cookie_value("\"unterminated").is_none());
    }

    #[test]
    fn playback_rejects_unscoped_expired_and_unrelated_cookies() {
        let url = reqwest::Url::parse("https://cdn.example.org/video/test.mp4").unwrap();
        for cookies in ["token=abc", "token=abc; Domain=example.org", "token=abc; Domain=evil.example.org; Path=/",
            "token=abc; Domain=ample.org; Path=/", "token=abc; Domain=example.org; Path=/videos",
            "token=abc; Domain=example.org; Path=/; Expires=1",
            "token=abc\r\nx-injected: yes; Domain=example.org; Path=/"] {
            assert!(playback_cookies(cookies, &url).is_empty(), "{cookies:?}");
        }
        let entry = serde_json::json!({"cookies": "token=abc; Domain=.example.org; Path=/"});
        let input = playback_input(&entry, &serde_json::json!({"url": url.as_str()})).unwrap();
        assert!(input.cookies.starts_with("token=abc;"));
        let http = reqwest::Url::parse("http://cdn.example.org/video/test.mp4").unwrap();
        assert!(playback_cookies("token=abc; Domain=example.org; Path=/; Secure", &http).is_empty());
    }

    #[test]
    fn transfer_progress_reads_estimated_bytes_and_separate_track_index() {
        let state = parse_transfer_progress("uyv-download\t251\t4096\tNA\t8192", "616+251");
        assert!(matches!(state, Some(TransferProgress::Downloading {
            bytes: 4096, total: Some(8192), track: Some((2, 2))
        })));
        assert!(matches!(parse_transfer_progress("uyv-processing", "616+251"), Some(TransferProgress::Processing)));
    }

    #[test]
    fn selected_sizes_require_every_separate_track() {
        assert_eq!(combined_size(Some(100), None), Some(100));
        assert_eq!(combined_size(Some(100), Some(Some(20))), Some(120));
        assert_eq!(combined_size(Some(100), Some(None)), None);
    }

    #[test]
    fn browser_choices_only_include_registered_supported_browsers() {
        let names = ["Google Chrome", "Microsoft Edge", "Internet Explorer", "QQ Browser"]
            .map(str::to_owned);
        assert_eq!(available_session_browsers(&names).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(available_session_browsers(&[]).count(), 0);
    }

    #[test]
    fn keeps_mp4_and_webm_for_matching_tracks() {
        assert_eq!(output_extension("mp4", None), "mp4");
        assert_eq!(output_extension("mp4", Some("m4a")), "mp4");
        assert_eq!(output_extension("webm", Some("webm")), "webm");
        assert_eq!(output_extension("mp4", Some("webm")), "mkv");
    }

    #[test]
    fn extracts_hls_without_sending_the_display_name() {
        let source = "https://example.org/index.m3u8?sign=x%2By&token=a%20b";
        let expected = Some((source.to_owned(), Some("吞噬星空 第241集".to_owned())));
        assert_eq!(extraction_source(&format!("{source}&name=吞噬星空 第241集")), expected);
        let signed = "https://example.org/index.m3u8?sign=x%2By";
        assert_eq!(
            extraction_source(&format!("https://web.zyplayer.fun/?url={signed}&name=吞噬星空 第241集")),
            Some((signed.to_owned(), Some("吞噬星空 第241集".to_owned()))),
        );
    }

    #[test]
    fn share_text_keeps_the_video_link_and_query() {
        assert_eq!(share_url("复制链接：https://v.douyin.com/abc/?x=1。打开抖音").as_deref(),
            Some("https://v.douyin.com/abc/?x=1"));
        assert_eq!(share_url("file:///video.mp4"), None);
        assert_eq!(share_url(r"复制 https\://v.douyin.com/JrC3Ilg36b0/ 打开抖音").as_deref(),
            Some("https://v.douyin.com/JrC3Ilg36b0/"));
    }

    #[test]
    fn douyin_featured_video_resolves_from_modal_id() {
        assert_eq!(share_url("https://www.douyin.com/jingxuan?modal_id=7671972624104197391").as_deref(),
            Some("https://www.douyin.com/video/7671972624104197391"));
        assert_eq!(share_url("https://www.douyin.com/jingxuan?modal_id=invalid").as_deref(),
            Some("https://www.douyin.com/jingxuan?modal_id=invalid"));
        assert_eq!(share_url("https://example.org/jingxuan?modal_id=123").as_deref(),
            Some("https://example.org/jingxuan?modal_id=123"));
    }

    #[test]
    fn lists_separate_hls_audio_without_a_declared_codec() {
        let metadata = serde_json::json!({"formats": [
            {"format_id": "sound-Test_audio", "vcodec": "none", "language": "en"},
            {"format_id": "500", "vcodec": "avc1", "acodec": "none", "width": 64, "height": 64},
            {"format_id": "22", "vcodec": "avc1", "acodec": "mp4a", "height": 720},
            {"format_id": "drm", "vcodec": "avc1", "has_drm": true}
        ]});
        let (video, audio) = formats(&metadata);
        assert_eq!(video.len(), 2);
        assert!(video[0].has_audio);
        assert!(video[1].video_only);
        assert_eq!(audio[0].id, "sound-Test_audio");
    }
}

fn filename(title: &str) -> String {
    let safe: String = title.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).take(80).collect();
    let safe = safe.trim_matches([' ', '.']);
    if safe.is_empty() { "video".into() } else { safe.to_owned() }
}

fn output_extension(video: &str, audio: Option<&str>) -> &'static str {
    match (video, audio) {
        ("mp4", None | Some("m4a" | "mp4")) => "mp4",
        ("webm", None | Some("webm")) => "webm",
        _ => "mkv",
    }
}

fn transfer(tool: &Path, url: &str, browser_session: Option<&Session>, selector: &str, extension: &str, destination: PathBuf, selection: crate::network_video::Selection, verify_audio: bool, progress: &Arc<Mutex<TransferProgress>>, cancel: &AtomicBool) -> Result<Outcome, String> {
    let ffmpeg = media_tools::find_tool("ffmpeg").ok_or_else(|| tr("未安装 FFmpeg，请前往第三方支持安装。").to_owned())?;
    let ffprobe = media_tools::find_tool("ffprobe").ok_or_else(|| tr("未安装 ffprobe，请前往第三方支持安装。").to_owned())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = format!("extract-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    let cache = crate::app_cache::network_root().join(&id);
    crate::app_cache::create_task_cache(&cache).map_err(|error| error.to_string())?;
    let result = (|| {
        let payload = cache.join(format!("payload.{extension}"));
        let mut parts = selector.split('+');
        let video_id = parts.next().unwrap_or(selector);
        let audio_id = parts.next();
        let cached_video = selection.track_cache(Track::Video);
        let cached_audio = selection.track_cache(Track::Audio);
        if let Some(downloaded) = selection.downloaded() {
            if downloaded.extension().is_some_and(|ext| ext.eq_ignore_ascii_case(extension)) {
                save_copy(&downloaded, &payload)?;
            } else {
                combine(&ffmpeg, &downloaded, None, &payload, progress, cancel)?;
            }
        } else if cached_video.is_some() || cached_audio.is_some() {
            let fetch = |id: &str, prefix: &str| {
                fetch_format(tool, url, browser_session, &ffmpeg, &cache, id, prefix, None, progress, cancel)
            };
            let video = match cached_video {
                Some(video) => video,
                None => fetch(video_id, "picture")?,
            };
            let audio = match (audio_id, cached_audio) {
                (_, Some(audio)) => Some(audio),
                (Some(id), None) => Some(fetch(id, "sound")?),
                (None, None) => None,
            };
            combine(&ffmpeg, &video, audio.as_deref(), &payload, progress, cancel)?;
        } else {
            fetch_format(tool, url, browser_session, &ffmpeg, &cache, selector, "payload", Some(extension), progress, cancel)?;
        }
        let types = run(media_tools::command(ffprobe).args(["-v", "error", "-show_entries", "stream=codec_type",
            "-of", "csv=p=0"]).arg(&payload))?;
        let types = String::from_utf8_lossy(&types);
        if !types.lines().any(|line| line.trim() == "video") ||
            (verify_audio && !types.lines().any(|line| line.trim() == "audio")) {
            return Err(tr("下载结果缺少所选的音频或视频。").into());
        }
        save_copy(&payload, &destination)?;
        let _guard = crate::app_cache::mutation_guard();
        let record_error = selection.remember(Some(destination.clone())).err();
        Ok(Outcome::Saved { path: destination, record_error })
    })();
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() { let _ = fs::remove_file(entry.path()); }
    }
    let _ = fs::remove_dir(&cache);
    crate::app_cache::forget_task_cache(&cache);
    result
}

fn fetch_format(tool: &Path, url: &str, session: Option<&Session>, ffmpeg: &Path, cache: &Path,
    selector: &str, prefix: &str, extension: Option<&str>, progress: &Arc<Mutex<TransferProgress>>,
    cancel: &AtomicBool) -> Result<PathBuf, String> {
    let (mut command, _cookies) = extractor(tool, session)?;
    command.arg("--ffmpeg-location").arg(ffmpeg).arg("--paths").arg(cache)
        .args(["--progress", "--newline", "--progress-delta", "0.2",
            "--progress-template", "download:uyv-download\t%(info.format_id)s\t%(progress.downloaded_bytes)s\t%(progress.total_bytes)s\t%(progress.total_bytes_estimate)s",
            "--progress-template", "postprocess:uyv-processing"]);
    if let Some(extension) = extension {
        command.args(["--merge-output-format", extension, "--remux-video", extension]);
    }
    run_transfer(command.args(["--format", selector, "--output", &format!("{prefix}.%(ext)s"), "--", url]),
        selector, progress, cancel)?;
    if let Some(extension) = extension { return Ok(cache.join(format!("{prefix}.{extension}"))); }
    fs::read_dir(cache).map_err(|error| error.to_string())?.flatten().map(|entry| entry.path())
        .find(|path| path.is_file() && path.file_stem().is_some_and(|stem| stem == prefix))
        .ok_or_else(|| tr("下载结果缺少所选的音频或视频。").to_owned())
}

fn combine(ffmpeg: &Path, video: &Path, audio: Option<&Path>, target: &Path,
    progress: &Mutex<TransferProgress>, cancel: &AtomicBool) -> Result<(), String> {
    {
        let mut state = progress.lock().unwrap();
        if cancel.load(Ordering::Relaxed) { return Err(tr("下载已取消。").into()); }
        *state = TransferProgress::Processing;
    }
    let mut command = media_tools::command(ffmpeg);
    command.args(["-nostdin", "-v", "error", "-n", "-i"]).arg(video);
    if let Some(audio) = audio {
        command.arg("-i").arg(audio).args(["-map", "0:V:0", "-map", "1:a:0"]);
    } else {
        command.args(["-map", "0:V:0", "-map", "0:a?"]);
    }
    run(command.args(["-c", "copy", "-abort_on", "empty_output"]).arg(target)).map(|_| ())
}

fn save_copy(source: &Path, target: &Path) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = target.with_file_name(format!(".unity-video-{}-{}.part",
        std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let mut output = OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|error| error.to_string())?;
    let copied = fs::File::open(source).and_then(|mut input| {
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()
    });
    drop(output);
    let saved = copied.map_err(|error| error.to_string()).and_then(|()| crate::playlist::move_new(&temporary, target));
    let _ = fs::remove_file(&temporary);
    saved
}
