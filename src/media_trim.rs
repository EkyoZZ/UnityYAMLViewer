//! Small, non-destructive FFmpeg trim exporter.
use std::{path::{Path, PathBuf}, sync::mpsc};
use eframe::egui;
use crate::{localization::tr, media_tools::command};

#[derive(Default)]
pub struct Trim {
    open: bool,
    start: f64,
    end: f64,
    pending: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    result: Option<Result<PathBuf, String>>,
    meter_trails: [f32; 2],
    settings: AudioSettings,
    remembered_volume: f32,
    volume_hover: Option<std::time::Instant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AudioFormat { Mp3, Wav, Flac, M4a }
impl AudioFormat {
    fn extension(self) -> &'static str { match self { Self::Mp3=>"mp3", Self::Wav=>"wav", Self::Flac=>"flac", Self::M4a=>"m4a" } }
    fn lossy(self) -> bool { matches!(self, Self::Mp3 | Self::M4a) }
}
#[derive(Clone)]
struct AudioSettings { format: AudioFormat, rate: u32, bits: u32, channels: u32, bitrate: u32 }
impl Default for AudioSettings {
    fn default() -> Self { Self { format:AudioFormat::Mp3, rate:44100, bits:16, channels:2, bitrate:192 } }
}
impl AudioSettings {
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.label(tr("导出设置"));
        egui::ComboBox::from_id_salt("trim_format").width(58.0).selected_text(self.format.extension()).show_ui(ui, |ui| {
            for format in [AudioFormat::Mp3, AudioFormat::Wav, AudioFormat::Flac, AudioFormat::M4a] {
                ui.selectable_value(&mut self.format, format, format.extension());
            }
        }).response.on_hover_text(tr("导出格式"));
        // Restrict MP3 to MPEG-1 sample rates so all offered bitrates are valid.
        let rates: &[u32] = if self.format==AudioFormat::Mp3 { &[32000,44100,48000] } else { &[8000,16000,22050,32000,44100,48000,96000] };
        if !rates.contains(&self.rate) { self.rate=44100; }
        egui::ComboBox::from_id_salt("trim_rate").width(78.0).selected_text(format!("{} Hz",self.rate)).show_ui(ui, |ui| {
            for rate in rates { ui.selectable_value(&mut self.rate, *rate, format!("{rate} Hz")); }
        }).response.on_hover_text(tr("采样率"));
        let depths: &[u32] = if self.format==AudioFormat::Wav { &[8,16,24,32] } else { &[16,24] };
        if !self.format.lossy() && !depths.contains(&self.bits) { self.bits=16; }
        if !self.format.lossy() {
            egui::ComboBox::from_id_salt("trim_bits").width(55.0).selected_text(format!("{} bit",self.bits)).show_ui(ui, |ui| {
                for bits in depths { ui.selectable_value(&mut self.bits, *bits, format!("{bits} bit")); }
            }).response.on_hover_text(tr("位深"));
        }
        egui::ComboBox::from_id_salt("trim_channels").width(65.0).selected_text(tr(if self.channels==1 {"单声道"}else{"双声道"})).show_ui(ui, |ui| {
            ui.selectable_value(&mut self.channels, 1, tr("单声道"));
            ui.selectable_value(&mut self.channels, 2, tr("双声道"));
        });
        if self.format.lossy() {
            egui::ComboBox::from_id_salt("trim_bitrate").width(75.0).selected_text(format!("{} kb/s",self.bitrate)).show_ui(ui, |ui| {
                for rate in [64,96,128,160,192,256,320] { ui.selectable_value(&mut self.bitrate, rate, format!("{rate} kb/s")); }
            }).response.on_hover_text(tr("码率"));
        }
    }
    fn configure(&self, cmd: &mut std::process::Command) {
        cmd.args(["-ar",&self.rate.to_string(),"-ac",&self.channels.to_string()]);
        match self.format {
            AudioFormat::Mp3 => { cmd.args(["-c:a","libmp3lame","-b:a",&format!("{}k",self.bitrate)]); }
            AudioFormat::M4a => { cmd.args(["-c:a","aac","-b:a",&format!("{}k",self.bitrate),"-movflags","+faststart"]); }
            AudioFormat::Wav => { cmd.args(["-c:a",match self.bits { 8=>"pcm_u8",24=>"pcm_s24le",32=>"pcm_s32le",_=>"pcm_s16le" }]); }
            AudioFormat::Flac => { cmd.args(["-c:a","flac","-sample_fmt",if self.bits==24 {"s32"} else {"s16"},"-bits_per_raw_sample",&self.bits.to_string()]); }
        }
    }
}

#[derive(Default)]
pub struct Action {
    pub seek: Option<f64>,
    pub toggle: bool,
    pub stop: bool,
    pub audition: Option<(f64, f64)>,
    pub volume: Option<f32>,
}

pub struct AudioView<'a> {
    pub wave: Option<&'a [Vec<f32>]>,
    pub wave_error: Option<&'a str>,
    pub playing: bool,
    pub volume: f32,
    pub description: String,
    pub playback_error: Option<&'a str>,
}

impl Trim {
    fn range_values(&mut self,ui:&mut egui::Ui,duration:f64,position:f64) {
        ui.label(tr("保留起点"));
        ui.add(egui::DragValue::new(&mut self.start).speed(0.1).range(0.0..=duration).max_decimals(3).suffix(" s"));
        if ui.button(tr("以当前位置为起点")).clicked(){self.start=position.clamp(0.0,duration);}
        ui.label(tr("保留终点"));
        ui.add(egui::DragValue::new(&mut self.end).speed(0.1).range(0.0..=duration).max_decimals(3).suffix(" s"));
        if ui.button(tr("以当前位置为终点")).clicked(){self.end=position.clamp(0.0,duration);}
    }
    fn range_actions(&mut self,ui:&mut egui::Ui,duration:f64,video:bool,action:&mut Action) {
        if ui.button(tr("重置范围")).clicked(){self.start=0.0;self.end=duration;}
        if video && ui.add_enabled(valid_range(self.start,self.end,duration),egui::Button::new(tr("试听保留片段"))).clicked(){action.audition=Some((self.start,self.end));}
        if ui.button(tr("跳到起点")).clicked(){action.seek=Some(self.start);}
        if ui.button(tr("跳到终点")).clicked(){action.seek=Some(self.end);}
    }
    pub fn open(&mut self, duration: f64) {
        if !self.open && self.pending.is_none() { self.start=0.0;self.end=duration;self.result=None; }
        self.open=true;
    }
    pub fn is_open(&self)->bool {self.open}
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path, ffmpeg: &Path, duration: f64, position: f64, video: bool, audio: Option<u64>, view: AudioView<'_>) -> Action {
        let mut action = Action::default();
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => { self.result = Some(result); self.pending = None; }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err(tr("剪切任务意外中断").into())); self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_some() { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        let title = if video { tr("剪切视频").to_owned() } else {
            format!("{} - {}", tr("剪切/转换音频"), path.file_name().unwrap_or_default().to_string_lossy())
        };
        let mut open = self.open;
        crate::native_window::Window::new(title).id(egui::Id::new(("media_trim", path)))
            .open(&mut open).resizable(true).default_width(if video { 560.0 } else { 900.0 })
            .scrollable(video).min_width(560.0).default_height(if video { 360.0 } else { 640.0 }).show(ui.ctx(), |ui| {
                if !video {
                    egui::TopBottomPanel::bottom("trim_transport").exact_height(28.0+ui.spacing().item_spacing.y).resizable(false).show_separator_line(false).frame(egui::Frame::NONE).show_inside(ui, |ui| {
                        ui.add_space(ui.spacing().item_spacing.y);
                    ui.horizontal(|ui| {
                        use crate::icons::Icon;
                        use crate::media_preview::transport_button;
                        if transport_button(ui, if view.playing { Icon::Pause } else { Icon::Play }, tr(if view.playing { "暂停" } else { "播放" })) { action.toggle = true; }
                        ui.add_enabled_ui(valid_range(self.start, self.end, duration), |ui| {
                            if transport_button(ui, Icon::TrimPlay, tr("剪切播放")) { action.audition = Some((self.start, self.end)); }
                        });
                        if transport_button(ui, Icon::Stop, tr("停止")) { action.stop = true; }
                        let mut volume=view.volume;
                        crate::media_preview::volume_control(ui,path,&mut volume,&mut self.remembered_volume,&mut self.volume_hover);
                        if volume!=view.volume {action.volume=Some(volume);}
                        let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width().max(1.0),28.0),egui::Sense::click_and_drag());
                        ui.painter().rect_filled(rect,3.0,ui.visuals().extreme_bg_color);
                        let mut cursor=position.clamp(0.0,duration);
                        if duration>0.0 && (response.clicked() || response.dragged() || response.drag_stopped()) {
                            if let Some(pointer)=response.interact_pointer_pos() {
                                cursor=((pointer.x-rect.left())/rect.width()).clamp(0.0,1.0) as f64*duration;
                                action.seek=Some(cursor);
                            }
                        }
                        if duration>0.0 {
                            let mut fill=rect;fill.max.x=rect.left()+rect.width()*(cursor/duration).clamp(0.0,1.0) as f32;
                            ui.painter().rect_filled(fill,3.0,ui.visuals().selection.bg_fill);
                        }
                        ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,format!("{} / {}",crate::media_preview::time(cursor),crate::media_preview::time(duration)),egui::FontId::proportional(13.0),ui.visuals().text_color());
                        response.on_hover_text(tr("点击或拖动跳转"));
                    });
                    });
                }
                let mut export_requested=false;
                if !video {
                    ui.add_enabled_ui(self.pending.is_none(), |ui| {
                        ui.horizontal_top(|ui| {
                            let button_width=(ui.painter().layout_no_wrap(tr("另存为").to_owned(),egui::FontId::proportional(20.0),egui::Color32::WHITE).size().x+24.0).max(120.0);
                            let left_width=(ui.available_width()-button_width-ui.spacing().item_spacing.x).max(1.0);
                            ui.allocate_ui_with_layout(egui::vec2(left_width,0.0),egui::Layout::top_down(egui::Align::Min),|ui| {
                                ui.horizontal_wrapped(|ui| {self.settings.controls(ui);});
                                egui::ScrollArea::horizontal().id_salt("trim_range_controls").auto_shrink([false,true]).show(ui,|ui| {
                                    ui.horizontal(|ui| {self.range_values(ui,duration,position);});
                                });
                                ui.horizontal_wrapped(|ui| {self.range_actions(ui,duration,false,&mut action);});
                            });
                            let export_button=ui.add_enabled(valid_range(self.start,self.end,duration),
                                egui::Button::new("").fill(egui::Color32::from_rgb(30,140,65)).min_size(egui::vec2(button_width,40.0)));
                            export_button.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,export_button.enabled(),tr("另存为")));
                            ui.painter().text(export_button.rect.center(),egui::Align2::CENTER_CENTER,tr("另存为"),egui::FontId::proportional(20.0),
                                if export_button.enabled(){egui::Color32::WHITE}else{ui.visuals().weak_text_color()});
                            export_requested=export_button.clicked();
                        });
                    });
                }
                ui.add_enabled_ui(self.pending.is_none(), |ui| {
                    if video {
                    egui::ScrollArea::horizontal().id_salt("trim_range_controls").auto_shrink([false,true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.range_values(ui,duration,position);
                        self.range_actions(ui,duration,true,&mut action);
                    });
                    });
                    }
                    ui.label(tr(if video { "另存 MP4（H.264 / AAC），仅保留主视频和当前音轨；不保留字幕。" } else { "仅导出当前音轨；音量控件仅影响试听。" }));
                    ui.small(tr("重新编码，边界受帧 / 采样精度限制；不覆盖原文件或已有文件。"));
                    let valid = valid_range(self.start, self.end, duration);
                    if !valid { ui.colored_label(ui.visuals().warn_fg_color, tr("终点必须大于起点且不超过媒体时长")); }
                    if export_requested || (video && ui.add_enabled(valid, egui::Button::new(tr("导出"))).clicked()) {
                        let extension = if video { "mp4" } else { self.settings.format.extension() };
                        let name = format!("{}_trim.{extension}", path.file_stem().unwrap_or_default().to_string_lossy());
                        let dialog = rfd::FileDialog::new().set_file_name(&name).add_filter(extension, &[extension]);
                        let dialog = if let Some(parent) = path.parent() { dialog.set_directory(parent) } else { dialog };
                        if let Some(target) = dialog.save_file() {
                            let target = target.with_extension(extension);
                            if target.exists() {
                                self.result = Some(Err(tr("目标已存在，请选择新的文件名").into()));
                            } else {
                                self.result = None;
                                let (tx, rx) = mpsc::channel(); self.pending = Some(rx);
                                let source = path.to_owned(); let ffmpeg = ffmpeg.to_owned();
                                let start = self.start; let end = self.end; let ctx = ui.ctx().clone();
                                let settings=self.settings.clone();
                                std::thread::spawn(move || {
                                    let result = export(&ffmpeg, &source, &target, start, end, duration, video, audio, &settings);
                                    let _ = tx.send(result); ctx.request_repaint();
                                });
                            }
                        }
                    }
                });
                if let Some(error)=view.playback_error {ui.colored_label(ui.visuals().warn_fg_color,error);}
                if self.pending.is_some() { ui.horizontal(|ui| { ui.spinner(); ui.label(tr("正在导出剪切片段…")); }); }
                if let Some(result) = &self.result {
                    match result {
                        Ok(path) => { ui.label(tr("剪切导出完成")); ui.label(path.display().to_string()); }
                        Err(error) => { ui.colored_label(ui.visuals().error_fg_color, tr("剪切导出失败")); ui.label(error); }
                    }
                }
                if !video {
                    ui.separator();
                    ui.small(&view.description);
                    ui.small(tr("波形左键设置起点，右键设置终点；拖动绿色边界调整范围。"));
                    crate::media_preview::paint_level_meter(ui, view.wave, position, duration, &mut self.meter_trails);
                    if let Some(wave) = view.wave {
                        ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height().max(1.0)), |ui| {
                            if let Some(seek) = crate::media_preview::paint_trim_waveform(ui, wave, position, duration, &mut self.start, &mut self.end) {
                                action.seek = Some(seek);
                            }
                        });
                    } else if let Some(error) = view.wave_error {
                        ui.colored_label(ui.visuals().warn_fg_color, error);
                    } else { ui.label(tr("正在生成波形…")); }

                }
            });
        self.open = open;
        action
    }
}

fn valid_range(start: f64, end: f64, duration: f64) -> bool {
    start.is_finite() && end.is_finite() && duration.is_finite() && start >= 0.0 && end > start && end <= duration
}

fn export(ffmpeg: &Path, source: &Path, target: &Path, start: f64, end: f64, duration: f64, video: bool, audio: Option<u64>, settings: &AudioSettings) -> Result<PathBuf, String> {
    if !valid_range(start, end, duration) { return Err("Invalid trim range".into()); }
    // Encode to a unique sibling, then exclusively create the destination. This
    // works on filesystems without hard links and never replaces an existing file.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let extension = if video { "mp4" } else { settings.format.extension() };
    let temp = target.with_file_name(format!(".unity-trim-{}-{}.{extension}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let reservation = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e| e.to_string())?;
    drop(reservation);
    let result = (|| {
        let mut cmd = command(ffmpeg);
        cmd.args(["-nostdin", "-v", "error", "-y", "-ss", &start.to_string()]).arg("-i").arg(source)
            .args(["-t", &(end-start).to_string()]);
        if video {
            cmd.args(["-map", "0:V:0", "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2", "-pix_fmt", "yuv420p", "-movflags", "+faststart"]);
        } else { cmd.arg("-vn"); }
        if let Some(index) = audio {
            cmd.args(["-map", &format!("0:{index}")]);
            if video {cmd.args(["-c:a","aac"]);}else{settings.configure(&mut cmd);}
        } else { cmd.arg("-an"); }
        let output = cmd.args(["-sn", "-dn", "-map_metadata", "-1"]).arg(&temp).output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).into_owned()); }
        if std::fs::metadata(&temp).map_err(|e| e.to_string())?.len() == 0 { return Err("FFmpeg produced an empty file".into()); }
        let mut input = std::fs::File::open(&temp).map_err(|e| e.to_string())?;
        let mut destination = std::fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(|e| e.to_string())?;
        let copied = std::io::copy(&mut input, &mut destination).and_then(|_| destination.sync_all());
        drop(destination);
        if let Err(error) = copied {
            let _ = std::fs::remove_file(target);
            return Err(error.to_string());
        }
        Ok(target.to_owned())
    })();
    let _ = std::fs::remove_file(&temp);
    result
}
