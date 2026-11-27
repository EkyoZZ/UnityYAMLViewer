use super::{Area, camera::Settings, command, find_tool};
use crate::localization::tr;
use eframe::egui;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const PREVIEW_FPS: u32 = 24;

fn preview_size(area: Area) -> [usize; 2] {
    [area.width, area.height].map(|edge| (edge.max(1) as usize).next_multiple_of(2))
}

pub(super) fn preview_filter() -> String {
    super::camera_frame_limit(PREVIEW_FPS)
}

const WINDOW_TITLE: &str = "画中画实时预览";

pub(super) type Position = (u32, u32, bool);

#[derive(Clone, Copy)]
pub(super) enum RecordControl {
    Unavailable,
    Start,
    Stop,
    Stopping,
}

impl RecordControl {
    pub fn audio_enabled(self) -> bool {
        matches!(self, Self::Start | Self::Stop)
    }

    pub fn button(self, ui: &mut egui::Ui, ready: bool) -> bool {
        super::toolbar::record_button(
            ui,
            matches!(self, Self::Stop | Self::Stopping),
            matches!(self, Self::Stop) || (matches!(self, Self::Start) && ready),
        )
        .clicked()
    }
}

pub(super) struct Footer<'a> {
    pub stop_label: &'a str,
    pub status: &'a str,
    pub result: Option<&'a std::path::Path>,
    pub history_requested: bool,
}

pub(super) fn preview_toolbar(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 28.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            left(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
        },
    );
}

pub(super) fn audio_controls(
    ui: &mut egui::Ui,
    control: RecordControl,
    audio: [Option<bool>; 2],
    toggles: &mut [bool; 2],
) {
    for (index, enabled) in audio.into_iter().enumerate().rev() {
        if let Some(on) = enabled {
            toggles[index] |=
                super::toolbar::audio_button(ui, on, index == 1, control.audio_enabled()).clicked();
        }
    }
}

pub(super) fn control_group(
    ui: &mut egui::Ui,
    name: &'static str,
    initial_width: f32,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.id().with(name);
    let width = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(id))
        .unwrap_or(initial_width);
    let response = ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            contents(ui);
        },
    );
    let measured = response.response.rect.width();
    if (width - measured).abs() > 0.5 {
        ui.ctx().data_mut(|data| data.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
}

impl Footer<'_> {
    pub fn show_controls(&mut self, ui: &mut egui::Ui) {
        control_group(ui, "preview_history", 175.0, |ui| {
            ui.label(format!("{}: {}", tr("停止"), self.stop_label));
            if ui.button(tr("历史记录")).clicked() {
                self.history_requested = true;
            }
        });
        if !self.status.is_empty() {
            ui.label(self.status);
        }
    }

    pub fn show_result(&self, ui: &mut egui::Ui) {
        if let Some(path) = self.result {
            ui.horizontal(|ui| {
                if ui.button(tr("打开所在文件夹")).clicked() {
                    let _ = command("explorer.exe").arg("/select,").arg(path).spawn();
                }
                let path_text = path.display().to_string();
                ui.add(egui::Label::new(&path_text).truncate())
                    .on_hover_text(path_text);
            });
        }
    }
}

fn camera_button(ui: &mut egui::Ui, hidden: bool, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .min_size(egui::vec2(28.0, 28.0))
            .selected(!hidden),
    );
    let center = response.rect.center();
    let ink = ui.style().interact(&response).text_color();
    let painter = ui.painter();
    let body = egui::Rect::from_center_size(center - egui::vec2(2.5, 0.0), egui::vec2(11.0, 9.0));
    painter.rect_filled(body, 1.5, ink);
    let lens = [(4.0, -2.0), (8.0, -4.5), (8.0, 4.5), (4.0, 2.0)];
    painter.add(egui::Shape::convex_polygon(
        lens.into_iter()
            .map(|(x, y)| center + egui::vec2(x, y))
            .collect(),
        ink,
        egui::Stroke::NONE,
    ));
    if hidden {
        let rect = response.rect.shrink(4.0);
        let stroke = egui::Stroke::new(1.6_f32, egui::Color32::from_rgb(230, 65, 75));
        painter.circle_stroke(center, rect.width() * 0.5, stroke);
        painter.line_segment([rect.left_bottom(), rect.right_top()], stroke);
    }
    response.on_hover_text(tr(if hidden {
        "显示摄像头"
    } else {
        "隐藏摄像头"
    }))
}

fn position(settings: &Settings) -> Position {
    (settings.x, settings.y, true)
}

pub(super) fn send_position(
    input: &mut impl Write,
    (x, y, visible): Position,
) -> std::io::Result<()> {
    let x = if visible {
        format!("(W-w)*{x}/100")
    } else {
        "W".to_owned()
    };
    writeln!(
        input,
        "coverlay@record_pip -1 x {x}\ncoverlay@record_pip -1 y (H-h)*{y}/100"
    )?;
    input.flush()
}

fn output_args(gpu: bool) -> [&'static str; 7] {
    [
        "-an",
        "-c:v",
        "rawvideo",
        "-threads:v",
        "1",
        "-pix_fmt",
        if gpu { "yuv420p" } else { "rgb24" },
    ]
}

pub(super) struct Picture {
    frames: Arc<Mutex<LatestFrame>>,
    size: [usize; 2],
    texture: Option<egui::TextureHandle>,
    gpu: bool,
    video: Option<Arc<crate::render_system::video::Frame>>,
    gpu_error: Arc<Mutex<Option<String>>>,
}

#[derive(Default)]
struct LatestFrame {
    bytes: Option<Vec<u8>>,
    finished: bool,
    received: bool,
}

impl Picture {
    fn new(frames: Arc<Mutex<LatestFrame>>, size: [usize; 2], gpu: bool) -> Self {
        Self {
            frames,
            size,
            gpu,
            texture: None,
            video: None,
            gpu_error: Default::default(),
        }
    }

    fn read(
        mut source: impl Read,
        frames: Arc<Mutex<LatestFrame>>,
        ctx: egui::Context,
        size: [usize; 2],
        gpu: bool,
    ) {
        let length = if gpu {
            crate::render_system::video::Frame::byte_len(size)
        } else {
            size[0] * size[1] * 3
        };
        let mut bytes = vec![0; length];
        while source.read_exact(&mut bytes).is_ok() {
            let replaced = {
                let mut frame = frames.lock().unwrap();
                frame.received = true;
                frame.bytes.replace(bytes)
            };
            if replaced.is_none() {
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
            bytes = replaced.unwrap_or_else(|| vec![0; length]);
        }
        frames.lock().unwrap().finished = true;
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    pub fn draw_frame(&mut self, ui: &mut egui::Ui) -> Option<egui::Rect> {
        let (bytes, finished) = {
            let mut frames = self.frames.lock().unwrap();
            (frames.bytes.take(), frames.finished)
        };
        match bytes {
            Some(bytes) => {
                if self.gpu {
                    self.video = Some(Arc::new(crate::render_system::video::Frame {
                        size: self.size,
                        bytes,
                        format: crate::render_system::video::Format::Yuv420,
                    }));
                } else {
                    let image = egui::ColorImage::from_rgb(self.size, &bytes);
                    if let Some(texture) = &mut self.texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        self.texture = Some(ui.ctx().load_texture(
                            "record_pip",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
            }
            None if finished => {
                ui.label(tr("摄像头预览已中断"));
                return None;
            }
            None => {}
        }
        if self.video.is_none() && self.texture.is_none() {
            return None;
        }
        let (available, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let aspect = self.size[0] as f32 / self.size[1] as f32;
        let width = available.width().min(available.height() * aspect);
        let frame =
            egui::Rect::from_center_size(available.center(), egui::vec2(width, width / aspect));
        if let Some(texture) = &self.texture {
            ui.painter().image(
                texture.id(),
                frame,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        if let Some(video) = &self.video {
            ui.painter().add(crate::render_system::video::callback(
                frame,
                frame,
                video.clone(),
                [false; 2],
                self.gpu_error.clone(),
            ));
        }
        if let Some(error) = self.gpu_error.lock().unwrap().as_deref() {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        Some(frame)
    }

    fn draw(&mut self, ui: &mut egui::Ui, settings: &mut Settings, resizable: bool) {
        let Some(frame) = self.draw_frame(ui) else {
            return;
        };
        settings.placement(ui, frame, resizable, None);
    }
}

pub(super) struct Output {
    address: String,
    _lifetime: Arc<()>,
    format: PreviewFormat,
    combined_frames: Option<Arc<Mutex<LatestFrame>>>,
}

enum PreviewFormat {
    Screen { ready: Arc<std::sync::atomic::AtomicBool> },
    Camera,
    Combined {
        gpu: bool,
        position: Arc<Mutex<Position>>,
    },
}

pub(super) struct LiveView {
    pub picture: Picture,
    pub position: Arc<Mutex<Position>>,
    pub settings: Settings,
}

impl Output {
    pub fn bind(
        ctx: &egui::Context,
        area: Area,
        settings: &Settings,
    ) -> std::io::Result<(Self, Picture, Arc<Mutex<Position>>)> {
        let frames = Arc::new(Mutex::new(LatestFrame::default()));
        let producer = frames.clone();
        let gpu = crate::render_system::available();
        let ctx = ctx.clone();
        let size = preview_size(area);
        let position = Arc::new(Mutex::new(position(settings)));
        let mut output = Self::listen(
            PreviewFormat::Combined {
                gpu,
                position: position.clone(),
            },
            move |stream| Picture::read(stream, producer, ctx, size, gpu),
        )?;
        output.combined_frames = Some(frames.clone());
        Ok((output, Picture::new(frames, size, gpu), position))
    }

    pub fn bind_screen_ready() -> std::io::Result<Self> {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ready = Arc::new(AtomicBool::new(false));
        let received = ready.clone();
        Self::listen(PreviewFormat::Screen { ready }, move |mut stream| {
            if stream.read_exact(&mut [0; 1]).is_ok() {
                received.store(true, Ordering::Release);
                let _ = std::io::copy(&mut stream, &mut std::io::sink());
            }
        })
    }

    pub fn bind_camera(
        ctx: &egui::Context,
    ) -> std::io::Result<(Self, std::sync::mpsc::Receiver<egui::ColorImage>)> {
        let (send, frames) = std::sync::mpsc::sync_channel(1);
        let ctx = ctx.clone();
        let output = Self::listen(PreviewFormat::Camera, move |stream| {
            super::camera::read_preview_frames(stream, send, ctx);
        })?;
        Ok((output, frames))
    }

    fn listen(
        format: PreviewFormat,
        read: impl FnOnce(std::net::TcpStream) + Send + 'static,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let address = format!("tcp://{}", listener.local_addr()?);
        let lifetime = Arc::new(());
        let guard = Arc::downgrade(&lifetime);
        std::thread::spawn(move || {
            while guard.strong_count() != 0 {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if stream.set_nonblocking(false).is_ok() {
                            read(stream);
                        }
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(15))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address,
            _lifetime: lifetime,
            format,
            combined_frames: None,
        })
    }

    pub fn has_frame(&self) -> bool {
        if let PreviewFormat::Screen { ready } = &self.format {
            ready.load(std::sync::atomic::Ordering::Acquire)
        } else {
            self.combined_frames.as_ref().is_some_and(|frames| frames.lock().unwrap().received)
        }
    }

    pub fn args(&self) -> Vec<String> {
        let mut args = vec!["-map", "[preview]", "-fps_mode", "passthrough"];
        let muxer = match self.format {
            PreviewFormat::Screen { .. } => {
                args.extend(["-an", "-flush_packets", "1", "-c:v", "rawvideo", "-threads:v", "1",
                    "-pix_fmt", "gray", "-f", "rawvideo", self.address.as_str()]);
                return args.into_iter().map(str::to_owned).collect();
            }
            PreviewFormat::Camera => {
                args.extend(super::camera::PREVIEW_OUTPUT);
                "image2pipe"
            }
            PreviewFormat::Combined { gpu, .. } => {
                args.extend(output_args(gpu));
                "rawvideo"
            }
        };
        // The preview socket can fall behind without slowing the recording output.
        args.extend([
            "-f",
            "fifo",
            "-fifo_format",
            muxer,
            "-queue_size",
            "3",
            "-drop_pkts_on_overflow",
            "1",
            self.address.as_str(),
        ]);
        args.into_iter().map(str::to_owned).collect()
    }

    pub fn position(&self) -> Option<Position> {
        match &self.format {
            PreviewFormat::Screen { .. } | PreviewFormat::Camera => None,
            PreviewFormat::Combined { position, .. } => Some(*position.lock().unwrap()),
        }
    }
}

struct PreviewProcess {
    child: Child,
    camera: Option<super::camera::Preview>,
    camera_failed: bool,
    picture: Picture,
    area: Area,
    device: String,
    last_update: Instant,
    position: Position,
    size: u32,
    ratio: super::camera::Ratio,
    portrait: bool,
    mirrored: bool,
}

impl PreviewProcess {
    fn open(ctx: &egui::Context, area: Area, settings: &Settings) -> Result<Self, String> {
        let ffmpeg = find_tool("ffmpeg").ok_or("FFmpeg")?;
        let gpu = crate::render_system::available();
        let size = preview_size(area);
        let camera = if gpu {
            let mut settings = settings.clone();
            settings.resolution = super::camera::Resolution::Device;
            Some(super::camera::Preview::open(ctx, &settings)?)
        } else {
            None
        };
        let mut capture = command(ffmpeg);
        capture
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-thread_queue_size",
                "4",
                "-f",
                "gdigrab",
                "-framerate",
                &PREVIEW_FPS.to_string(),
                "-draw_mouse",
                "0",
            ])
            .arg("-offset_x")
            .arg(area.x.to_string())
            .arg("-offset_y")
            .arg(area.y.to_string())
            .arg("-video_size")
            .arg(format!("{}x{}", area.width, area.height))
            .args(["-i", "desktop"]);
        if camera.is_some() {
            capture.args(["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2"]);
        } else {
            capture
                .args(["-thread_queue_size", "4", "-f", "dshow", "-i"])
                .arg(format!("video={}", settings.device))
                .arg("-filter_complex")
                .arg(settings.filter(area))
                .args(["-map", "[out]"]);
        }
        let mut child = capture
            .args(output_args(gpu))
            .args(["-f", "rawvideo", "pipe:1"])
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdout.take().unwrap();
        let frames = Arc::new(Mutex::new(LatestFrame::default()));
        let producer = frames.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || Picture::read(input, producer, ctx, size, gpu));
        Ok(Self {
            child,
            camera,
            camera_failed: false,
            picture: Picture::new(frames, size, gpu),
            area,
            device: settings.device.clone(),
            last_update: Instant::now(),
            position: position(settings),
            size: settings.size,
            ratio: settings.ratio,
            portrait: settings.portrait,
            mirrored: settings.mirrored,
        })
    }
}

impl Drop for PreviewProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn display_selector(
    ui: &mut egui::Ui,
    monitors: &[super::displays::Display],
    selected: &mut String,
    enabled: bool,
) {
    let display_label = |display: &super::displays::Display| {
        tr("屏幕{0}").replace("{0}", &display.number.to_string())
    };
    let label = if selected == "all" {
        tr("全部屏幕").to_owned()
    } else {
        monitors
            .iter()
            .find(|display| display.key == *selected)
            .map_or_else(|| tr("主屏全屏").to_owned(), display_label)
    };
    control_group(ui, "pip_display", 190.0, |ui| {
        if !enabled {
            ui.disable();
        }
        ui.label(tr("画中画录制屏幕"));
        egui::ComboBox::from_id_salt("pip_display")
            .selected_text(label)
            .show_ui(ui, |ui| {
                ui.selectable_value(selected, String::new(), tr("主屏全屏"));
                for display in monitors {
                    ui.selectable_value(selected, display.key.clone(), display_label(display));
                }
                if monitors.len() > 1 || selected == "all" {
                    ui.selectable_value(selected, "all".to_owned(), tr("全部屏幕"));
                }
            });
    });
}

#[derive(Default)]
pub(super) struct View {
    pub open: bool,
    retained: Option<(Picture, Option<egui::TextureHandle>)>,
    process: Option<PreviewProcess>,
    error: Option<String>,
    camera_hidden: bool,
}

impl View {
    pub fn id() -> egui::Id {
        egui::Id::new("record_pip_preview")
    }

    pub fn release(&mut self) {
        self.process = None;
        self.retained = None;
    }

    pub fn handoff(&mut self) {
        if let Some(mut process) = self.process.take() {
            let placeholder = Picture::new(
                Default::default(),
                process.picture.size,
                process.picture.gpu,
            );
            let mut picture = std::mem::replace(&mut process.picture, placeholder);
            picture.frames = Default::default();
            let camera = process
                .camera
                .as_ref()
                .and_then(|camera| camera.texture())
                .cloned();
            self.retained = Some((picture, camera));
        }
    }

    pub fn verify_exclusion(&self) -> Result<(), String> {
        if exclude_window(tr(WINDOW_TITLE))? {
            Ok(())
        } else {
            Err(tr("无法从屏幕捕获排除画中画窗口").into())
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        area: Area,
        monitors: &[super::displays::Display],
        settings: &mut Settings,
        live: Option<(&mut Picture, &Mutex<Position>)>,
        recording: bool,
        control: RecordControl,
        switch: bool,
        selection: &mut Option<super::camera::Source>,
        settings_requested: &mut bool,
        audio: [Option<bool>; 2],
        audio_toggles: &mut [bool; 2],
        footer: &mut Footer<'_>,
        countdown: Option<&mut super::countdown::Request>,
    ) -> bool {
        if !self.open {
            self.release();
            return false;
        }
        let title = tr(WINDOW_TITLE);
        let excluded = exclude_window(title);
        let mut open = self.open;
        let mut toggle = false;
        if !recording {
            self.camera_hidden = false;
        }
        if self.process.as_ref().is_some_and(|process| {
            process.area != area
                || process.device != settings.device
                || process.ratio != settings.ratio
                || process.portrait != settings.portrait
        }) {
            self.release();
        }
        if matches!(excluded, Ok(true))
            && !recording
            && self.process.is_none()
            && self.error.is_none()
        {
            match PreviewProcess::open(ctx, area, settings) {
                Ok(process) => self.process = Some(process),
                Err(error) => self.error = Some(error),
            }
        }
        if let Some(process) = &mut self.process
            && let Some(camera) = &mut process.camera
        {
            process.camera_failed = camera.update_texture(ctx);
        }
        let window = crate::native_window::Window::new(title).id(Self::id());
        let window = window.open(&mut open).block_close_request(matches!(
            control,
            RecordControl::Stop | RecordControl::Stopping
        ));
        window
            .always_on_top().scrollable(false).default_width(660.0).min_width(420.0).default_height(400.0).min_height(320.0).show(ctx, |ui| {
                egui::TopBottomPanel::bottom("pip_preview_controls")
                    .default_height(140.0)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        ui.separator();
                        preview_toolbar(ui, |ui| {
                            super::camera::source_menu(ui, super::camera::Source::Combined, switch, selection);
                            *settings_requested |= ui.add_enabled_ui(switch, crate::theme::settings_button).inner.clicked();
                        }, |ui| {
                            toggle = control.button(ui, matches!(excluded, Ok(true)) && self.process.as_ref().is_some_and(|process| !process.camera_failed && process.camera.as_ref().is_none_or(|camera| camera.texture().is_some())));
                            if recording && live.is_some()
                                && camera_button(ui, self.camera_hidden, matches!(control, RecordControl::Stop)).clicked()
                            {
                                self.camera_hidden = !self.camera_hidden;
                            }
                            audio_controls(ui, control, audio, audio_toggles);
                        });
                        let previous_display = settings.display.clone();
                        ui.horizontal_wrapped(|ui| {
                            display_selector(ui, monitors, &mut settings.display, !recording && switch);
                            settings.placement_controls(ui, !recording && switch);
                            settings.framing_controls(ui, !recording && switch);
                        });
                        ui.horizontal_wrapped(|ui| footer.show_controls(ui));
                    footer.show_result(ui);
                        if previous_display != settings.display {
                            self.error = None;
                            toggle = false;
                            ctx.request_repaint_of(egui::ViewportId::ROOT);
                        }
                    });
                if let Err(error) = &excluded {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                } else if let Some((picture, position)) = live {
                    if let Some(frame) = picture.draw_frame(ui) {
                        self.retained = None;
                        settings.placement(ui, frame, false, None);
                        if let Some(countdown) = countdown {
                            countdown.paint(ui, frame);
                        }
                    } else if let Some((picture, camera)) = &mut self.retained
                        && let Some(frame) = picture.draw_frame(ui)
                        && let Some(texture) = camera
                    {
                        let size = texture.size_vec2();
                        let overlay = settings.camera_rect(frame, Some(size.x / size.y));
                        ui.painter_at(frame).image(
                            texture.id(), overlay,
                            super::camera::preview_uv(settings.mirrored),
                            egui::Color32::WHITE,
                        );
                    }
                    *position.lock().unwrap() = (settings.x, settings.y, !self.camera_hidden);
                } else if let Some(process) = &mut self.process {
                    self.retained = None;
                    if let Some(camera) = &process.camera {
                        if process.camera_failed {
                            ui.colored_label(ui.visuals().error_fg_color, tr("摄像头预览已中断"));
                            if ui.button(tr("重试")).clicked() {
                                self.release();
                                ctx.request_repaint_of(egui::ViewportId::ROOT);
                            }
                        } else if let Some(frame) = process.picture.draw_frame(ui)
                            && let Some(texture) = camera.texture()
                        {
                            let size = texture.size_vec2();
                            let aspect = Some(size.x / size.y);
                            let camera_rect = settings.camera_rect(frame, aspect);
                            ui.painter_at(frame).image(
                                texture.id(), camera_rect,
                                super::camera::preview_uv(settings.mirrored),
                                egui::Color32::WHITE,
                            );
                            settings.placement(ui, frame, true, aspect);
                        }
                        return;
                    }
                    if process.mirrored != settings.mirrored
                        && let Some(input) = &mut process.child.stdin
                        && writeln!(input, "chflip@preview_camera_mirror -1 enable {}", u8::from(settings.mirrored))
                            .and_then(|()| input.flush()).is_ok()
                    {
                        process.mirrored = settings.mirrored;
                    }
                    process.picture.draw(ui, settings, true);
                    if process.last_update.elapsed() >= Duration::from_millis(420) && let Some(input) = &mut process.child.stdin {
                        if position(settings) != process.position {
                            let _ = send_position(input, position(settings));
                            process.position = position(settings);
                            process.last_update = Instant::now();
                        }
                        if settings.size != process.size {
                            let (width, height) = settings.dimensions(area);
                            let _ = writeln!(input, "cscale@record_camera -1 w {width}\ncscale@record_camera -1 h {height}");
                            let _ = input.flush();
                            process.size = settings.size;
                            process.last_update = Instant::now();
                        }
                    }
                } else if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    if ui.button(tr("重试")).clicked() { self.error = None; }
                } else {
                    ui.spinner();
                    ctx.request_repaint_after_for(Duration::from_millis(50), egui::ViewportId::ROOT);
                }
            });
        self.open = open;
        if !open {
            self.release();
            self.error = None;
        }
        toggle
    }
}

#[cfg(windows)]
fn exclude_window(title: &str) -> Result<bool, String> {
    use std::{ffi::c_void, ptr::null};
    type Handle = *mut c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> Handle;
        fn GetWindowThreadProcessId(window: Handle, process: *mut u32) -> u32;
        fn GetWindowDisplayAffinity(window: Handle, affinity: *mut u32) -> i32;
        fn SetWindowDisplayAffinity(window: Handle, affinity: u32) -> i32;
    }
    let title: Vec<_> = title.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let window = FindWindowW(null(), title.as_ptr());
        let mut process = 0;
        if window.is_null() {
            return Ok(false);
        }
        GetWindowThreadProcessId(window, &mut process);
        if process != std::process::id() {
            return Ok(false);
        }
        let mut affinity = 0;
        if GetWindowDisplayAffinity(window, &mut affinity) != 0 && affinity == 0x11 {
            return Ok(true);
        }
        if SetWindowDisplayAffinity(window, 0x11) != 0
            && GetWindowDisplayAffinity(window, &mut affinity) != 0
            && affinity == 0x11
        {
            return Ok(true);
        }
        Err(format!(
            "{} · {}",
            tr("无法从屏幕捕获排除画中画窗口"),
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(not(windows))]
fn exclude_window(_: &str) -> Result<bool, String> {
    Err(tr("无法从屏幕捕获排除画中画窗口").into())
}
