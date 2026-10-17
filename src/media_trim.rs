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
    session:Option<crate::audio_edit::Session>,
    session_error:Option<String>,
    fade_dialog:Option<bool>,
    gain_dialog:bool,
    dialog_before:Option<crate::audio_edit::State>,
    history_changed:bool,
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
    pub rewind: bool,
    pub audition: Option<(f64, f64)>,
    pub audition_tail:Option<(f64,f64)>,
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
    pub fn prepare_audio(&mut self,ffmpeg:&Path,path:&Path,duration:f64,audio:u64,channels:usize,ctx:&egui::Context){
        if self.session.is_none() && self.session_error.is_none(){
            match crate::audio_edit::Session::new(ffmpeg,path,duration,audio,channels,ctx){Ok(session)=>self.session=Some(session),Err(error)=>self.session_error=Some(error)}
        }
        if let Some(session)=&mut self.session{session.poll(ctx);}
    }
    pub fn clip(&self)->Option<std::sync::Arc<crate::audio_edit::Clip>>{self.session.as_ref()?.state.clip.clone()}
    fn range_values(&mut self,ui:&mut egui::Ui,duration:f64,position:f64) {
        let interval=self.session.as_ref().is_some_and(|session|session.state.interval());
        ui.label(tr(if interval{"区间起点"}else{"保留起点"}));
        ui.add(egui::DragValue::new(&mut self.start).speed(0.1).range(0.0..=duration).max_decimals(3).suffix(" s"));
        if ui.button(tr("以当前位置为起点")).clicked(){self.start=position.clamp(0.0,duration);}
        ui.label(tr(if interval{"区间终点"}else{"保留终点"}));
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
    fn edit_buttons(&mut self,ui:&mut egui::Ui,duration:f64,action:&mut Action){
        let Some(session)=&mut self.session else{return};
        if ui.add_enabled(session.ready()&&(session.state.interval()||valid_range(self.start,self.end,duration)),egui::Button::new(tr("剪切"))).clicked(){
            session.state.start=self.start;session.state.end=self.end;
            session.apply(true,ui.ctx(),None);action.stop=true;
        }
        for (label,incoming) in [("淡入",true),("淡出",false)] {
            if ui.add_enabled(session.ready()&&valid_range(if session.state.interval(){self.start.min(self.end)}else{self.start},if session.state.interval(){self.start.max(self.end)}else{self.end},duration),egui::Button::new(tr(label))).clicked(){
                session.state.start=self.start;session.state.end=self.end;
                self.dialog_before=Some(session.state.clone());self.fade_dialog=Some(incoming);
                let (start,end)=session.state.bounds();
                if incoming {session.state.fade_in=(end-start-session.state.fade_out).max(0.0);}
                else {session.state.fade_out=(end-start-session.state.fade_in).max(0.0);}
            }
        }
        if ui.add_enabled(session.ready(),egui::Button::new(tr("音量(dB)"))).clicked(){self.dialog_before=Some(session.state.clone());self.gain_dialog=true;}
        let keyboard=ui.is_enabled()&&!ui.ctx().wants_keyboard_input();
        let undo=ui.add_enabled(session.can_undo(),egui::Button::new(tr("撤销"))).clicked() || (keyboard&&ui.input_mut(|input|input.consume_key(egui::Modifiers::CTRL,egui::Key::Z)));
        let redo=ui.add_enabled(session.can_redo(),egui::Button::new(tr("重做"))).clicked() || (keyboard&&ui.input_mut(|input|input.consume_key(egui::Modifiers::CTRL,egui::Key::Y)));
        if undo||redo {if undo{session.undo();}else{session.redo();}self.start=session.state.start;self.end=session.state.end;self.history_changed=true;action.stop=true;}
    }
    fn effect_dialogs(&mut self,ctx:&egui::Context,path:&Path){
        let Some(session)=&mut self.session else{return};
        let incoming=self.fade_dialog;
        if incoming.is_none()&&!self.gain_dialog{return;}
        let title=tr(match incoming{Some(true)=>"淡入",Some(false)=>"淡出",None=>"音量(dB)"});
        let font=egui::TextStyle::Button.resolve(&ctx.style());
        let curve_width=crate::audio_edit::FADE_CURVES.iter().map(|(_,label)|ctx.debug_painter().layout_no_wrap(tr(label).to_owned(),font.clone(),egui::Color32::WHITE).size().x.ceil()).fold(0.0_f32,f32::max)+40.0;
        let (start,end)=session.state.gain_bounds();
        let scope=if start==0.0&&end==session.state.duration(){tr("作用范围：整段音频").to_owned()}
            else{format!("{}: {:.3} s – {:.3} s",tr("作用范围"),start,end)};
        let style=ctx.style();
        let measure=|text:&str,font:egui::FontId|ctx.debug_painter().layout_no_wrap(text.to_owned(),font,egui::Color32::WHITE).size().x.ceil();
        let gain_value_width=(measure("-60.0 dB",egui::TextStyle::Monospace.resolve(&style))+style.spacing.button_padding.x*2.0).max(style.spacing.interact_size.x);
        let gain_controls_width=["−","+",tr("重置")].iter().map(|text|(measure(text,font.clone())+style.spacing.button_padding.x*2.0).max(style.spacing.interact_size.x)).sum::<f32>()
            +gain_value_width+style.spacing.item_spacing.x*3.0;
        let action_width=["应用","取消"].iter().map(|label|(measure(tr(label),egui::FontId::proportional(16.0))+24.0).max(80.0)).sum::<f32>()+style.spacing.item_spacing.x;
        // NativeWindow adds the window content margins to this logical width.
        let width=if incoming.is_some(){240.0_f32.max(curve_width).max(action_width)}
            else{240.0_f32.max(measure(&scope,egui::TextStyle::Body.resolve(&style))).max(gain_controls_width).max(action_width)};
        let mut open=true;let mut apply=false;let mut cancel=false;
        crate::native_window::Window::new(title).id(egui::Id::new(("audio_effect",path))).open(&mut open).modal().always_on_top().default_width(width).min_width(width).fit_height().show(ctx,|ui|{
            if let Some(incoming)=incoming {
                let (start,end)=session.state.bounds();let span=(end-start).max(0.0);
                let (seconds,curve,points,handles,other)=if incoming{(&mut session.state.fade_in,&mut session.state.in_curve,&mut session.state.in_points,&mut session.state.in_handles,session.state.fade_out)}else{(&mut session.state.fade_out,&mut session.state.out_curve,&mut session.state.out_points,&mut session.state.out_handles,session.state.fade_in)};
                ui.horizontal(|ui|{ui.label(tr("时长"));ui.add(egui::DragValue::new(seconds).speed(0.1).range(0.0..=(span-other).max(0.0)).suffix(" s"));});
                let selected=if curve=="custom"{"自定义"}else{crate::audio_edit::FADE_CURVES.iter().find(|(id,_)|*id==curve.as_str()).map_or("线性",|(_,label)|*label)};
                egui::ComboBox::from_id_salt("fade_curve").width(curve_width).selected_text(tr(selected)).show_ui(ui,|ui|{
                    for (id,label) in crate::audio_edit::FADE_CURVES {if ui.selectable_value(curve,id.to_owned(),tr(label)).clicked(){*points=crate::audio_edit::preset_points(id);*handles=crate::audio_edit::preset_handles(id,points);}}
                });
                edit_fade_curve(ui,curve,points,handles,incoming,*seconds);
                ui.add(egui::Label::new(egui::RichText::new(tr("拖动圆点调整位置，拖动方形手柄调整曲率；点击添加，右键删除，最多8点。")).small()).wrap());
            }else{
                ui.add(egui::Label::new(&scope).extend());
                ui.horizontal(|ui|{
                    if ui.button("−").clicked(){session.state.db=(session.state.db-1.0).max(-60.0);}
                    ui.add_sized([gain_value_width,ui.spacing().interact_size.y],egui::DragValue::new(&mut session.state.db).speed(0.1).range(-60.0..=24.0).suffix(" dB"));
                    if ui.button("+").clicked(){session.state.db=(session.state.db+1.0).min(24.0);}
                    if ui.button(tr("重置")).clicked(){session.state.db=0.0;}
                });
                if session.state.db>0.0{ui.add(egui::Label::new(egui::RichText::new(tr("增益可能导致削波失真")).small()).wrap());}
            }
            ui.add(egui::Label::new(egui::RichText::new(tr("确认后刷新音频与波形，可撤销；取消恢复预览前状态。")).small()).wrap());
            let font=egui::FontId::proportional(16.0);
            let widths=["应用","取消"].map(|label|(ui.painter().layout_no_wrap(tr(label).to_owned(),font.clone(),ui.visuals().text_color()).size().x+24.0).max(80.0));
            ui.add_space(4.0);
            ui.horizontal(|ui|{
                ui.add_space(((ui.available_width()-widths.iter().sum::<f32>()-ui.spacing().item_spacing.x)*0.5).max(0.0));
                for (index,label) in ["应用","取消"].into_iter().enumerate(){
                    let response=ui.add_sized([widths[index],32.0],egui::Button::new(""));
                    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,response.enabled(),tr(label)));
                    ui.painter().text(response.rect.center(),egui::Align2::CENTER_CENTER,tr(label),font.clone(),ui.visuals().text_color());
                    if index==0{apply=response.clicked();}else{cancel=response.clicked();}
                }
            });
        });
        if apply {session.apply(false,ctx,self.dialog_before.take());self.fade_dialog=None;self.gain_dialog=false;}
        else if cancel||!open {if let Some(before)=self.dialog_before.take(){session.state=before;}self.fade_dialog=None;self.gain_dialog=false;}
    }
    pub fn show(&mut self, ui: &mut egui::Ui, path: &Path, ffmpeg: &Path, duration: f64, position: f64, video: bool, audio: Option<u64>, view: AudioView<'_>) -> Action {
        self.history_changed=false;
        let before=self.session.as_ref().map(|session|session.state.clone());
        if let Some(session)=&self.session{self.start=session.state.start;self.end=session.state.end;}
        let settings_open=self.fade_dialog.is_some()||self.gain_dialog;
        let edit_busy=self.session.as_ref().is_some_and(|session|!session.ready())||self.session_error.is_some();
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
        let title = if video { tr("视频编辑").to_owned() } else {
            format!("{} - {}", tr("音频编辑"), path.file_name().unwrap_or_default().to_string_lossy())
        };
        let mut open = self.open;
        crate::native_window::Window::new(title).id(egui::Id::new(("media_trim", path)))
            .open(&mut open).resizable(true).default_width(if video { 560.0 } else { 900.0 })
            .scrollable(video).min_width(560.0).default_height(if video { 360.0 } else { 640.0 }).show(ui.ctx(), |ui| {
                if !video {
                    egui::TopBottomPanel::bottom("trim_transport").exact_height(28.0+ui.spacing().item_spacing.y).resizable(false).show_separator_line(false).frame(egui::Frame::NONE).show_inside(ui, |ui| {
                        ui.add_space(ui.spacing().item_spacing.y);
                    ui.add_enabled_ui(!edit_busy,|ui| {ui.horizontal(|ui| {
                        use crate::icons::Icon;
                        use crate::media_preview::transport_button;
                        if transport_button(ui, if view.playing { Icon::Pause } else { Icon::Play }, tr(if view.playing { "暂停" } else { "播放" })) { action.toggle = true; }
                        ui.add_enabled_ui(valid_range(self.start, self.end, duration)||self.session.as_ref().is_some_and(|session|session.state.interval()), |ui| {
                            if transport_button(ui, Icon::TrimPlay, tr("剪切播放")) {
                                if let Some(session)=&self.session {
                                    let ranges=session.state.kept_ranges();action.audition=ranges.first().copied();action.audition_tail=ranges.get(1).copied();
                                }else{action.audition=Some((self.start,self.end));}
                            }
                        });
                        if transport_button(ui, Icon::Stop, tr("停止")) { action.stop = true; action.rewind = true; }
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
                    });});
                    });
                }
                let mut export_requested=false;
                if !video {
                    ui.add_enabled_ui(self.pending.is_none() && !edit_busy && !settings_open, |ui| {
                        ui.horizontal_top(|ui| {
                            let button_width=(ui.painter().layout_no_wrap(tr("另存为").to_owned(),egui::FontId::proportional(20.0),egui::Color32::WHITE).size().x+24.0).max(120.0);
                            let left_width=(ui.available_width()-button_width-ui.spacing().item_spacing.x).max(1.0);
                            ui.allocate_ui_with_layout(egui::vec2(left_width,0.0),egui::Layout::top_down(egui::Align::Min),|ui| {
                                ui.horizontal_wrapped(|ui| {self.settings.controls(ui);});
                                ui.horizontal_wrapped(|ui| {
                                    if let Some(session)=&mut self.session {
                                        if crate::theme::compact_selector(ui,&mut session.state.mode,[("ends","头尾"),("interval","区间")]) {
                                            if session.state.interval(){
                                                if self.start==0.0&&self.end==duration{self.end=0.0;}
                                            }else if self.start==self.end{self.start=0.0;self.end=duration;}
                                            else{let low=self.start.min(self.end);self.end=self.start.max(self.end);self.start=low;}
                                        }
                                        let selected=(self.end.clamp(0.0,duration)-self.start.clamp(0.0,duration)).abs();
                                        ui.label(format!("{}: {:.3} s",tr("区间时长"),selected));
                                        if session.state.interval(){
                                            ui.label(format!("{}: {:.3} s",tr("剪切后时长"),(duration-selected).max(0.0)));
                                        }
                                    }
                                    self.edit_buttons(ui,duration,&mut action);
                                });
                                egui::ScrollArea::horizontal().id_salt("trim_range_controls").auto_shrink([false,true]).show(ui,|ui| {
                                    ui.horizontal(|ui| {self.range_values(ui,duration,position);});
                                });
                                ui.horizontal_wrapped(|ui| {self.range_actions(ui,duration,false,&mut action);});
                            });
                            let export_button=ui.add_enabled(self.session.as_ref().map_or(valid_range(self.start,self.end,duration),|session|session.ready()),
                                egui::Button::new("").fill(egui::Color32::from_rgb(30,140,65)).min_size(egui::vec2(button_width,40.0)));
                            export_button.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,export_button.enabled(),tr("另存为")));
                            ui.painter().text(export_button.rect.center(),egui::Align2::CENTER_CENTER,tr("另存为"),egui::FontId::proportional(20.0),
                                if export_button.enabled(){egui::Color32::WHITE}else{ui.visuals().weak_text_color()});
                            export_requested=export_button.clicked();
                        });
                    });
                }
                ui.add_enabled_ui(self.pending.is_none() && !edit_busy && !settings_open, |ui| {
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
                    if !valid && !self.session.as_ref().is_some_and(|session|session.state.interval()) { ui.colored_label(ui.visuals().warn_fg_color, tr("终点必须大于起点且不超过媒体时长")); }
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
                                let clip=self.clip();
                                let source=clip.as_ref().map_or_else(||path.to_owned(),|clip|clip.path.clone()); let ffmpeg = ffmpeg.to_owned();
                                let (start,end,duration,audio)=if let Some(clip)=&clip{(0.0,clip.duration,clip.duration,Some(0))}else{(self.start,self.end,duration,audio)};let ctx = ui.ctx().clone();
                                let filter=self.session.as_ref().map(|session|session.filter_string());
                                let settings=self.settings.clone();
                                std::thread::spawn(move || {
                                    let _cache=clip;
                                    let result = export(&ffmpeg, &source, &target, start, end, duration, video, audio, &settings,filter.as_deref());
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
                    ui.small(tr(if self.session.as_ref().is_some_and(|session|session.state.interval()) {
                        "区间：左键设起点，右键设终点；删除两条边界之间的范围并拼接两端。"
                    }else{"波形左键设置起点，右键设置终点；拖动绿色边界调整范围。"}));
                    ui.small(tr("选择仅作预览；点击剪切提交，另存为输出当前完整时间轴。"));
                    crate::media_preview::paint_level_meter(ui, view.wave, position, duration, &mut self.meter_trails);
                    if self.session.as_ref().is_some_and(|session|session.busy()) {ui.horizontal(|ui|{ui.spinner();ui.label(tr("正在处理无损音频…"));});}
                    if let Some(error)=self.session_error.as_ref().or_else(||self.session.as_ref().and_then(|session|session.error.as_ref())){ui.colored_label(ui.visuals().error_fg_color,tr(error));}
                    let processed_wave=self.session.as_ref().filter(|session|session.state.db!=0.0||session.state.fade_in>0.0||session.state.fade_out>0.0).and_then(|session|view.wave.map(|wave|session.filter_preview(wave)));
                    if let Some(wave) = processed_wave.as_deref().or(view.wave) {
                        ui.add_enabled_ui(!edit_busy && !settings_open,|ui| {ui.allocate_ui(egui::vec2(ui.available_width(), ui.available_height().max(1.0)), |ui| {
                            if let Some(seek) = crate::media_preview::paint_trim_waveform(ui, wave, position, duration, &mut self.start, &mut self.end,self.session.as_ref().is_some_and(|session|session.state.interval()),view.playing) {
                                action.seek = Some(seek);
                            }
                        });});
                    } else if let Some(error) = view.wave_error {
                        ui.colored_label(ui.visuals().warn_fg_color, error);
                    } else { ui.label(tr("正在生成波形…")); }

                }
            });
        self.open = open;
        if let Some(session)=&mut self.session {
            session.state.start=self.start;session.state.end=self.end;
            let (start,end)=session.state.bounds();let span=(end-start).max(0.0);session.state.fade_in=session.state.fade_in.min(span);session.state.fade_out=session.state.fade_out.min((span-session.state.fade_in).max(0.0));
            if !self.history_changed && self.dialog_before.is_none() && !session.busy(){if let Some(before)=before{session.record_controls(before,ui.input(|input|input.pointer.any_down()));}}
        }
        self.effect_dialogs(ui.ctx(),path);
        action
    }
}

fn valid_range(start: f64, end: f64, duration: f64) -> bool {
    start.is_finite() && end.is_finite() && duration.is_finite() && start >= 0.0 && end > start && end <= duration
}

fn export(ffmpeg: &Path, source: &Path, target: &Path, start: f64, end: f64, duration: f64, video: bool, audio: Option<u64>, settings: &AudioSettings, audio_filter:Option<&str>) -> Result<PathBuf, String> {
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
            if video {cmd.args(["-c:a","aac"]);}else{if let Some(filter)=audio_filter{cmd.args(["-af",filter]);}settings.configure(&mut cmd);}
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

fn edit_fade_curve(ui:&mut egui::Ui,kind:&mut String,points:&mut Vec<(f64,f64)>,handles:&mut Vec<[(f64,f64);2]>,incoming:bool,seconds:f64){
    if kind!="custom" {*points=crate::audio_edit::preset_points(kind);*handles=crate::audio_edit::preset_handles(kind,points);}
    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),170.0),egui::Sense::click());
    let plot=rect.shrink2(egui::vec2(12.0,18.0));
    let painter=ui.painter_at(rect);
    painter.rect_filled(plot,3.0,ui.visuals().extreme_bg_color);
    let screen=|x:f64,y:f64|egui::pos2(plot.left()+plot.width()*if incoming{x as f32}else{1.0-x as f32},plot.bottom()-plot.height()*y as f32);
    let value=|p:egui::Pos2|{let u=((p.x-plot.left())/plot.width()).clamp(0.0,1.0) as f64;(if incoming{u}else{1.0-u},((plot.bottom()-p.y)/plot.height()).clamp(0.0,1.0) as f64)};
    for i in 0..=4 {
        let fraction=i as f32/4.0;
        painter.line_segment([egui::pos2(plot.left()+plot.width()*fraction,plot.top()),egui::pos2(plot.left()+plot.width()*fraction,plot.bottom())],ui.visuals().widgets.noninteractive.bg_stroke);
        painter.line_segment([egui::pos2(plot.left(),plot.top()+plot.height()*fraction),egui::pos2(plot.right(),plot.top()+plot.height()*fraction)],ui.visuals().widgets.noninteractive.bg_stroke);
    }
    let hovered=ui.input(|i|i.pointer.hover_pos());
    let over_control=hovered.is_some_and(|p|points.iter().chain(handles.iter().flatten()).any(|&(x,y)|screen(x,y).distance(p)<10.0));
    // Handle x coordinates stay ordered within the segment: time cannot fold backwards.
    for segment in 0..handles.len(){for side in 0..2 {
        let p=screen(handles[segment][side].0,handles[segment][side].1);
        let grip=ui.interact(egui::Rect::from_center_size(p,egui::vec2(12.0,12.0)),response.id.with(("handle",segment,side)),egui::Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if grip.dragged(){if let Some(p)=grip.interact_pointer_pos(){
            let (x,y)=value(p);
            let low=if side==0{points[segment].0}else{handles[segment][0].0};
            let high=if side==0{handles[segment][1].0}else{points[segment+1].0};
            handles[segment][side]=(x.clamp(low,high),y);*kind="custom".into();
        }}
    }}
    // Normalized x=0 is the fade-in start and the fade-out end.
    // Its time stays fixed; only amplitude and its attached tangent move vertically.
    if let Some(&(x,y))=points.first(){
        let grip=ui.interact(egui::Rect::from_center_size(screen(x,y),egui::vec2(16.0,16.0)),response.id.with("fade_endpoint"),egui::Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical);
        if grip.dragged(){if let Some(p)=grip.interact_pointer_pos(){
            let (_,amplitude)=value(p);let delta=amplitude-y;
            points[0]=(0.0,amplitude);
            if let Some(h)=handles.first_mut(){h[0].1=(h[0].1+delta).clamp(0.0,1.0);}
            *kind="custom".into();
        }}
    }
    let mut remove=None;
    for index in 1..points.len().saturating_sub(1){
        let grip=ui.interact(egui::Rect::from_center_size(screen(points[index].0,points[index].1),egui::vec2(16.0,16.0)),response.id.with(("anchor",index)),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if grip.clicked_by(egui::PointerButton::Secondary){remove=Some(index);}
        if grip.dragged(){if let Some(p)=grip.interact_pointer_pos(){
            let (x,y)=value(p);let old=points[index];
            points[index]=(x.clamp(points[index-1].0+0.001,points[index+1].0-0.001),y);
            let delta=(points[index].0-old.0,points[index].1-old.1);
            for (seg,side) in [(index-1,1),(index,0)]{
                handles[seg][side].0+=delta.0;handles[seg][side].1=(handles[seg][side].1+delta.1).clamp(0.0,1.0);
            }
            // Clamp both neighboring segments after moving an anchor.
            for seg in [index-1,index]{
                handles[seg][0].0=handles[seg][0].0.clamp(points[seg].0,points[seg+1].0);
                handles[seg][1].0=handles[seg][1].0.clamp(handles[seg][0].0,points[seg+1].0);
            }
            *kind="custom".into();
        }}
    }
    if let Some(index)=remove{
        handles[index-1]=[handles[index-1][0],handles[index][1]];
        handles.remove(index);points.remove(index);*kind="custom".into();
    }
    if response.clicked()&&!over_control&&points.len()<8 {
        if let Some(p)=response.interact_pointer_pos().filter(|p|plot.contains(*p)){
            let (x,y)=value(p);
            if points.iter().all(|&(a,_)|(a-x).abs()>0.002){
                let index=points.partition_point(|p|p.0<x);
                points.insert(index,(x,y));
                let a=points[index-1];let b=points[index+1];
                let linear=|p:(f64,f64),q:(f64,f64)|[(p.0+(q.0-p.0)/3.0,p.1+(q.1-p.1)/3.0),(p.0+(q.0-p.0)*2.0/3.0,p.1+(q.1-p.1)*2.0/3.0)];
                handles[index-1]=linear(a,(x,y));handles.insert(index,linear((x,y),b));*kind="custom".into();
            }
        }
    }
    let color=ui.visuals().selection.bg_fill;
    for (index,h) in handles.iter().enumerate(){for side in 0..2{
        let anchor=points[index+side];let p=screen(h[side].0,h[side].1);
        painter.line_segment([screen(anchor.0,anchor.1),p],egui::Stroke::new(1.0,ui.visuals().weak_text_color()));
        painter.rect_filled(egui::Rect::from_center_size(p,egui::vec2(6.0,6.0)),0.0,color);
    }}
    let line:Vec<_>=(0..=160).map(|i|{let x=i as f64/160.0;screen(x,crate::audio_edit::fade_value(x,kind,points,handles))}).collect();
    painter.add(egui::Shape::line(line,egui::Stroke::new(2.0,color)));
    for &(x,y) in points.iter(){painter.circle_filled(screen(x,y),3.5,ui.visuals().text_color());}
    painter.text(egui::pos2(plot.left(),rect.bottom()),egui::Align2::LEFT_BOTTOM,"0 s",egui::FontId::proportional(10.0),ui.visuals().weak_text_color());
    painter.text(egui::pos2(plot.right(),rect.bottom()),egui::Align2::RIGHT_BOTTOM,format!("{seconds:.3} s"),egui::FontId::proportional(10.0),ui.visuals().weak_text_color());
    painter.text(plot.left_top(),egui::Align2::LEFT_BOTTOM,"100%",egui::FontId::proportional(10.0),ui.visuals().weak_text_color());
}
