use crate::localization::tr;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Source {
    #[default]
    Screen,
    Camera,
    Combined,
}

pub(super) fn source_menu(
    ui: &mut egui::Ui,
    source: Source,
    enabled: bool,
    selection: &mut Option<Source>,
) {
    let label = |source| {
        tr(match source {
            Source::Screen => "选取屏幕",
            Source::Camera => "摄像头",
            Source::Combined => "摄像头画中画",
        })
    };
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt("preview_recording_source")
            .selected_text(label(source))
            .show_ui(ui, |ui| {
                for choice in [Source::Screen, Source::Camera, Source::Combined] {
                    if ui
                        .selectable_label(choice == source, label(choice))
                        .clicked()
                        && choice != source
                    {
                        *selection = Some(choice);
                    }
                }
            });
    });
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Ratio {
    #[default]
    Original,
    Wide,
    Standard,
    Photo,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Resolution {
    #[default]
    Device,
    P480,
    P720,
    P1080,
}

impl Resolution {
    fn label(self) -> &'static str {
        match self {
            Self::Device => tr("设备默认"),
            Self::P480 => "480p",
            Self::P720 => "720p",
            Self::P1080 => "1080p",
        }
    }

    fn short_edge(self) -> Option<u32> {
        match self {
            Self::Device => None,
            Self::P480 => Some(480),
            Self::P720 => Some(720),
            Self::P1080 => Some(1080),
        }
    }
}

impl Ratio {
    fn label(self, portrait: bool) -> &'static str {
        match (self, portrait) {
            (Self::Original, _) => tr("原始比例"),
            (Self::Square, _) => "1:1",
            (Self::Wide, false) => "16:9",
            (Self::Wide, true) => "9:16",
            (Self::Standard, false) => "4:3",
            (Self::Standard, true) => "3:4",
            (Self::Photo, false) => "3:2",
            (Self::Photo, true) => "2:3",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Settings {
    pub source: Source,
    pub device: String,
    pub display: String,
    pub x: u32,
    pub y: u32,
    pub size: u32,
    pub ratio: Ratio,
    pub portrait: bool,
    pub mirrored: bool,
    pub resolution: Resolution,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source: Source::Screen,
            device: String::new(),
            display: String::new(),
            x: 98,
            y: 98,
            size: 25,
            ratio: Ratio::Original,
            portrait: false,
            mirrored: false,
            resolution: Resolution::Device,
        }
    }
}

impl Settings {
    pub fn valid(&self) -> bool {
        self.x <= 100
            && self.y <= 100
            && (5..=75).contains(&self.size)
            && (self.source == Source::Screen
                || (cfg!(windows)
                    && !self.device.is_empty()
                    && !self.device.contains(['\0', '\r', '\n'])))
    }

    pub fn filter(&self, area: super::Area) -> String {
        format!(
            "[0:v]setpts=PTS-STARTPTS[screen];[1:v]setpts=PTS-STARTPTS[camera_src];{}",
            self.overlay_filter(area, true)
        )
    }

    pub(super) fn overlay_filter(&self, area: super::Area, preview: bool) -> String {
        let (width, height) = self.dimensions(area);
        let framing = self.camera_input_filter(!preview);
        let mirror = if preview {
            format!(",hflip@preview_camera_mirror=enable={}", u8::from(self.mirrored))
        } else {
            String::new()
        };
        format!(
            "[camera_src]{framing}scale@record_camera=w={width}:h={height}:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1{mirror}[camera];[screen][camera]overlay@record_pip=x=(W-w)*{}/100:y=(H-h)*{}/100:shortest=1,pad=ceil(iw/2)*2:ceil(ih/2)*2[out]",
            self.x, self.y
        )
    }

    fn camera_input_filter(&self, mirror: bool) -> String {
        let mut filter = self.aspect().map_or_else(String::new, |ratio| format!(
            "crop=w='max(2,trunc(min(iw,ih*{ratio})/2)*2)':h='max(2,trunc(min(ih,iw/{ratio})/2)*2)',"
        ));
        if mirror && self.mirrored {
            filter.push_str("hflip,");
        }
        filter
    }

    pub fn camera_only_filter(&self) -> String {
        self.camera_filter(true)
    }

    fn camera_filter(&self, mirror: bool) -> String {
        let framing = self.camera_input_filter(mirror);
        let scale = self
            .resolution
            .short_edge()
            .map_or_else(String::new, |edge| {
                format!("scale=w='if(gte(iw,ih),-2,{edge})':h='if(gte(iw,ih),{edge},-2)',")
            });
        format!("{framing}{scale}setsar=1,crop=trunc(iw/2)*2:trunc(ih/2)*2")
    }

    pub fn resolution_controls(&mut self, ui: &mut egui::Ui, enabled: bool) {
        super::pip::control_group(ui, "preview_resolution", 185.0, |ui| {
            if !enabled {
                ui.disable();
            }
            ui.label(tr("摄像头录制分辨率"));
            egui::ComboBox::from_id_salt("camera_resolution")
                .selected_text(self.resolution.label())
                .show_ui(ui, |ui| {
                    for resolution in [
                        Resolution::Device,
                        Resolution::P480,
                        Resolution::P720,
                        Resolution::P1080,
                    ] {
                        ui.selectable_value(&mut self.resolution, resolution, resolution.label());
                    }
                });
        });
    }

    fn aspect(&self) -> Option<f64> {
        let ratio = match self.ratio {
            Ratio::Original => return None,
            Ratio::Wide => 16.0 / 9.0,
            Ratio::Standard => 4.0 / 3.0,
            Ratio::Photo => 3.0 / 2.0,
            Ratio::Square => 1.0,
        };
        Some(if self.portrait { 1.0 / ratio } else { ratio })
    }

    pub fn framing_controls(&mut self, ui: &mut egui::Ui, enabled: bool) {
        super::pip::control_group(ui, "preview_ratio", 250.0, |ui| {
            if !enabled {
                ui.disable();
            }
            ui.label(tr("摄像头画面比例"));
            egui::ComboBox::from_id_salt("record_camera_ratio")
                .selected_text(self.ratio.label(self.portrait))
                .show_ui(ui, |ui| {
                    for ratio in [
                        Ratio::Original,
                        Ratio::Wide,
                        Ratio::Standard,
                        Ratio::Photo,
                        Ratio::Square,
                    ] {
                        ui.selectable_value(&mut self.ratio, ratio, ratio.label(self.portrait));
                    }
                });
            ui.add_enabled_ui(
                !matches!(self.ratio, Ratio::Original | Ratio::Square),
                |ui| {
                    let mut orientation = if self.portrait {
                        "portrait"
                    } else {
                        "landscape"
                    }
                    .to_owned();
                    if crate::theme::small_selector(
                        ui,
                        &mut orientation,
                        [("landscape", "横向"), ("portrait", "竖向")],
                    ) {
                        self.portrait = orientation == "portrait";
                    }
                },
            );
            crate::theme::small_toggle_switch(ui, tr("镜像"), &mut self.mirrored);
        });
    }

    pub fn dimensions(&self, area: super::Area) -> (i64, i64) {
        let dimension = |length: i32| (i64::from(length) * i64::from(self.size) / 200 * 2).max(2);
        (dimension(area.width), dimension(area.height))
    }

    pub fn placement_controls(&mut self, ui: &mut egui::Ui, resizable: bool) {
        super::pip::control_group(ui, "pip_placement", 210.0, |ui| {
            for (label, value, low, high) in [
                ("X", &mut self.x, 0, 100),
                ("Y", &mut self.y, 0, 100),
                (tr("大小"), &mut self.size, 5, 75),
            ] {
                ui.label(label);
                ui.add_enabled(
                    label != tr("大小") || resizable,
                    egui::DragValue::new(value).range(low..=high).suffix("%"),
                );
            }
        });
    }

    pub fn camera_rect(&self, rect: egui::Rect, source_aspect: Option<f32>) -> egui::Rect {
        let mut size = rect.size() * self.size as f32 / 100.0;
        if let Some(ratio) = source_aspect.or_else(|| self.aspect().map(|aspect| aspect as f32)) {
            size.x = size.x.min(size.y * ratio);
            size.y = size.y.min(size.x / ratio);
        }
        let free = rect.size() - size;
        let origin = rect.min + free * egui::vec2(self.x as f32, self.y as f32) / 100.0;
        egui::Rect::from_min_size(origin, size)
    }

    pub fn placement(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        resizable: bool,
        source_aspect: Option<f32>,
    ) {
        let camera = self.camera_rect(rect, source_aspect);
        let free = rect.size() - camera.size();
        let handle = egui::Rect::from_center_size(camera.right_bottom(), egui::Vec2::splat(12.0))
            .intersect(rect);
        let move_camera = ui.interact(camera, ui.id().with("camera_move"), egui::Sense::drag());
        let resize = ui.interact(
            handle,
            ui.id().with("camera_resize"),
            if resizable {
                egui::Sense::drag()
            } else {
                egui::Sense::hover()
            },
        );
        let drag_id = ui.id().with("camera_placement_start");
        if resize.drag_started() || move_camera.drag_started() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(drag_id, (self.x, self.y, self.size)));
        }
        let (x, y, size_percent) = ui
            .ctx()
            .data(|data| data.get_temp::<(u32, u32, u32)>(drag_id))
            .unwrap_or((self.x, self.y, self.size));
        let delta = ui.input(|input| {
            input
                .pointer
                .press_origin()
                .zip(input.pointer.latest_pos())
                .map_or(egui::Vec2::ZERO, |(origin, current)| current - origin)
        });
        if resize.dragged() {
            let resize_width = (camera.width() * 100.0 / self.size as f32).max(1.0);
            self.size = (size_percent as f32 + delta.x / resize_width * 100.0)
                .round()
                .clamp(5.0, 75.0) as u32;
        } else if move_camera.dragged() {
            self.x = (x as f32 + delta.x / free.x.max(1.0) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u32;
            self.y = (y as f32 + delta.y / free.y.max(1.0) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u32;
        }
        let painter = ui.painter_at(rect);
        painter.rect_stroke(
            camera,
            2.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
        if resizable {
            painter.rect_filled(handle.shrink(3.0), 1.0, ui.visuals().selection.stroke.color);
        }
    }
}

#[derive(Default)]
pub(super) struct Devices {
    pub video: Vec<(String, String)>,
    pub audio: Vec<(String, String)>,
}

pub(super) fn devices() -> Result<Devices, String> {
    let ffmpeg = super::find_tool("ffmpeg").ok_or("FFmpeg")?;
    let output = super::command(ffmpeg)
        .args([
            "-hide_banner",
            "-f",
            "dshow",
            "-list_devices",
            "true",
            "-i",
            "dummy",
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut devices = Devices::default();
    let mut kind = None;
    for line in stderr.lines() {
        let Some((_, quoted)) = line.split_once('"') else {
            continue;
        };
        let Some((name, suffix)) = quoted.rsplit_once('"') else {
            continue;
        };
        if line.contains("Alternative name") {
            if let Some(is_video) = kind.take() {
                let list = if is_video {
                    &mut devices.video
                } else {
                    &mut devices.audio
                };
                if let Some((id, _)) = list.last_mut() {
                    *id = name.to_owned();
                }
            }
        } else {
            kind = match suffix.trim() {
                "(video)" | "(video, audio)" => Some(true),
                "(audio)" => Some(false),
                _ => None,
            };
            if let Some(is_video) = kind {
                let list = if is_video {
                    &mut devices.video
                } else {
                    &mut devices.audio
                };
                list.push((name.to_owned(), name.to_owned()));
            }
        }
    }
    if devices.video.is_empty() && devices.audio.is_empty() {
        return Err(format!("{}\n{}", tr("未找到摄像头或麦克风"), stderr.trim()));
    }
    Ok(devices)
}

const PREVIEW_EDGE: usize = 640;
pub(super) const PREVIEW_FPS: u32 = 15;
pub(super) const PREVIEW_SCALE_FILTER: &str =
    "scale=640:640:force_original_aspect_ratio=decrease,setsar=1";
pub(super) const PREVIEW_OUTPUT: [&str; 7] =
    ["-an", "-c:v", "ppm", "-threads:v", "1", "-pix_fmt", "rgb24"];

pub(super) fn read_preview_frames(
    source: impl std::io::Read,
    send: std::sync::mpsc::SyncSender<egui::ColorImage>,
    ctx: egui::Context,
) {
    use std::{io::BufReader, sync::mpsc::TrySendError};
    let mut source = BufReader::new(source);
    while let Some(image) = read_preview_frame(&mut source) {
        match send.try_send(image) {
            Ok(()) => ctx.request_repaint_of(egui::ViewportId::ROOT),
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => return,
        }
    }
    drop(send);
    ctx.request_repaint_of(egui::ViewportId::ROOT);
}

fn read_preview_frame(source: &mut impl std::io::BufRead) -> Option<egui::ColorImage> {
    use std::io::{BufRead, Read};
    // FFmpeg's PPM encoder writes three header lines before each packed RGB frame.
    let mut header = String::new();
    let mut header_reader = source.by_ref().take(64);
    header_reader.read_line(&mut header).ok()?;
    if header.trim_end() != "P6" {
        return None;
    }
    header.clear();
    header_reader.read_line(&mut header).ok()?;
    let mut dimensions = header.split_ascii_whitespace();
    let width: usize = dimensions.next()?.parse().ok()?;
    let height: usize = dimensions.next()?.parse().ok()?;
    if !(1..=PREVIEW_EDGE).contains(&width) || !(1..=PREVIEW_EDGE).contains(&height) {
        return None;
    }
    header.clear();
    header_reader.read_line(&mut header).ok()?;
    if header.trim_end() != "255" {
        return None;
    }
    let mut pixels = vec![0; width * height * 3];
    source.read_exact(&mut pixels).ok()?;
    Some(egui::ColorImage::from_rgb([width, height], &pixels))
}

pub(super) fn preview_uv(mirrored: bool) -> egui::Rect {
    if mirrored {
        egui::Rect { min: egui::pos2(1., 0.), max: egui::pos2(0., 1.) }
    } else {
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.))
    }
}

pub(super) struct Preview {
    child: Option<std::process::Child>,
    frames: std::sync::mpsc::Receiver<egui::ColorImage>,
    texture: Option<egui::TextureHandle>,
    source: (String, Ratio, bool, Resolution),
    texture_from_recording: bool,
}

impl Preview {
    pub fn open(ctx: &egui::Context, settings: &Settings) -> Result<Self, String> {
        use std::{process::Stdio, sync::mpsc};
        let ffmpeg = super::find_tool("ffmpeg").ok_or("FFmpeg")?;
        let mut child = super::command(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "dshow",
                "-i",
            ])
            .arg(format!("video={}", settings.device))
            .args([
                "-vf",
                &format!(
                    "{},fps={PREVIEW_FPS},{PREVIEW_SCALE_FILTER}",
                    settings.camera_filter(false)
                ),
            ])
            .args(PREVIEW_OUTPUT)
            .args(["-f", "image2pipe", "pipe:1"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;
        let stdout = child.stdout.take().unwrap();
        let (send, frames) = mpsc::sync_channel(1);
        let ctx = ctx.clone();
        std::thread::spawn(move || read_preview_frames(stdout, send, ctx));
        Ok(Self {
            child: Some(child),
            frames,
            texture: None,
            source: (
                settings.device.clone(),
                settings.ratio,
                settings.portrait,
                settings.resolution,
            ),
            texture_from_recording: false,
        })
    }

    pub fn start_live(&mut self, frames: std::sync::mpsc::Receiver<egui::ColorImage>) {
        self.release_device();
        self.frames = frames;
    }

    fn release_device(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn matches(&self, settings: &Settings) -> bool {
        self.source.0 == settings.device
            && self.source.1 == settings.ratio
            && self.source.2 == settings.portrait
            && self.source.3 == settings.resolution
    }

    pub fn update_texture(&mut self, ctx: &egui::Context) -> bool {
        match self.frames.try_recv() {
            Ok(image) => {
                self.texture_from_recording = self.child.is_none();
                if let Some(texture) = &mut self.texture {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "record_camera_preview",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                false
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => true,
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
        }
    }

    pub fn texture(&self) -> Option<&egui::TextureHandle> {
        self.texture.as_ref()
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        switch: bool,
        selection: &mut Option<Source>,
        control: super::pip::RecordControl,
        recording: bool,
        toggle: &mut bool,
        settings_requested: &mut bool,
        audio: [Option<bool>; 2],
        audio_toggles: &mut [bool; 2],
        footer: &mut super::pip::Footer<'_>,
        countdown: Option<&mut super::countdown::Request>,
    ) -> bool {
        let closed = self.update_texture(ctx);
        let mut open = true;
        let title = tr(if recording {
            "摄像头录制中"
        } else {
            "摄像头预览"
        });
        let window = crate::native_window::Window::new(title)
            .id(egui::Id::new("record_camera_preview"))
            .always_on_top()
            .scrollable(false)
            .default_width(400.0)
            .min_width(350.0)
            .default_height(380.0)
            .min_height(300.0);
        let window = window.open(&mut open).block_close_request(matches!(
            control,
            super::pip::RecordControl::Stop | super::pip::RecordControl::Stopping
        ));
        window.show(ctx, |ui| {
            egui::TopBottomPanel::bottom("camera_preview_controls")
                .default_height(115.0)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.separator();
                    super::pip::preview_toolbar(
                        ui,
                        |ui| {
                            source_menu(ui, Source::Camera, switch, selection);
                            *settings_requested |= ui
                                .add_enabled_ui(switch, crate::theme::settings_button)
                                .inner
                                .clicked();
                        },
                        |ui| {
                            *toggle |= control.button(ui, !closed && self.texture.is_some());
                            super::pip::audio_controls(ui, control, audio, audio_toggles);
                        },
                    );
                    let previous = (
                        settings.ratio,
                        settings.portrait,
                        settings.resolution,
                        settings.mirrored,
                    );
                    ui.horizontal_wrapped(|ui| {
                        settings.framing_controls(ui, switch);
                        settings
                            .resolution_controls(ui, switch && settings.source == Source::Camera);
                    });
                    if previous
                        != (
                            settings.ratio,
                            settings.portrait,
                            settings.resolution,
                            settings.mirrored,
                        )
                    {
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                    ui.horizontal_wrapped(|ui| footer.show_controls(ui));
                    footer.show_result(ui);
                });
            let mut countdown_area = ui.available_rect_before_wrap();
            if closed {
                ui.colored_label(ui.visuals().error_fg_color, tr("摄像头预览已中断"));
            } else if let Some(texture) = &self.texture {
                let (available, _) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                let frame_size = texture.size_vec2();
                let scale =
                    (available.width() / frame_size.x).min(available.height() / frame_size.y);
                let frame = egui::Rect::from_center_size(available.center(), frame_size * scale);
                ui.painter().image(
                    texture.id(),
                    frame,
                    preview_uv(settings.mirrored && !self.texture_from_recording),
                    egui::Color32::WHITE,
                );
                countdown_area = frame;
                ui.painter().rect_stroke(
                    frame,
                    0.0,
                    ui.visuals().window_stroke,
                    egui::StrokeKind::Outside,
                );
            } else {
                ui.spinner();
            }
            if let Some(countdown) = countdown {
                countdown.paint(ui, countdown_area);
            }
        });
        open
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.release_device();
    }
}
