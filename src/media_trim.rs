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
    video_settings: VideoSettings,
    video_tracks: crate::export_tracks::Tracks,
    remembered_volume: f32,
    volume_hover: Option<std::time::Instant>,
    session:Option<crate::audio_edit::Session>,
    session_error:Option<String>,
    fade_dialog:Option<bool>,
    gain_dialog:bool,
    dialog_before:Option<crate::audio_edit::State>,
    history_changed:bool,
}

#[derive(Clone)]
struct VideoSettings { codec: usize, container: usize, quality: u32, bitrate_mode:bool, two_pass:bool, video_bitrate:u32, fps:String, audio_bitrate: u32, audio_codec:usize, audio_rate:u32, gif_colors:u32, gif_dither:usize, gif_bayer:u32, crop:crate::video_crop::Crop }
impl Default for VideoSettings {fn default()->Self{Self{codec:0,container:0,quality:18,bitrate_mode:false,two_pass:false,video_bitrate:6000,fps:"source".into(),audio_bitrate:192,audio_codec:0,audio_rate:48000,gif_colors:256,gif_dither:4,gif_bayer:2,crop:Default::default()}}}
impl VideoSettings {
    const CONTAINERS:[(&'static str,&'static str,&'static str);10]=[
        ("MP4","mp4","mp4"),("WebM","webm","webm"),("Matroska (MKV)","mkv","matroska"),
        ("QuickTime (MOV)","mov","mov"),("AVI","avi","avi"),("FLV","flv","flv"),
        ("MPEG-TS","ts","mpegts"),("3GP","3gp","3gp"),("NUT","nut","nut"),("GIF","gif","gif")];
    fn extension(&self)->&'static str {Self::CONTAINERS[self.container].1}
    fn muxer(&self)->&'static str {Self::CONTAINERS[self.container].2}
    const CODECS:[&'static str;11]=["H.264","VP8","VP9","H.265 (HEVC)","未压缩 RGB 24位","MJPEG","MPEG-4 Part 2","ProRes 422","AV1","MPEG-2","GIF"];
    fn compatible(&self,codec:usize)->bool{match self.container{
        0=>matches!(codec,0|2|3|6|8|9),1=>matches!(codec,1|2|8),2=>codec<10&&codec!=4,
        3=>matches!(codec,0|3|4|5|6|7|9),4=>matches!(codec,0|1|2|3|4|5|6|9),
        5=>matches!(codec,0|2|3),6=>matches!(codec,0|3|9),7=>matches!(codec,0|6),8=>codec<10,9=>codec==10,_=>false
    }}
    fn encoder(&self)->&'static str {["libx264","libvpx","libvpx-vp9","libx265","rawvideo","mjpeg","mpeg4","prores_ks","libsvtav1","mpeg2video","gif"][self.codec]}
    fn h26x(&self)->bool{matches!(self.codec,0|3)}
    const AUDIO:[(&'static str,&'static str);8]=[("AAC","aac"),("Opus","libopus"),("PCM 16-bit","pcm_s16le"),("MP3","libmp3lame"),("AC-3","ac3"),("FLAC","flac"),("Vorbis","libvorbis"),("E-AC-3","eac3")];
    fn audio_compatible(&self,codec:usize)->bool{match self.container{
        0=>matches!(codec,0|1|3|4|5|7),1=>matches!(codec,1|6),2|8=>codec<8,
        3=>matches!(codec,0|2|3|4|7),4=>matches!(codec,0|2|3|4),5=>matches!(codec,0|3),6=>matches!(codec,0|3|4|7),7=>codec==0,_=>false
    }}
    fn audio_encoder(&self)->&'static str{Self::AUDIO[self.audio_codec].1}
    fn audio_rates(&self)->&'static [u32]{match self.audio_codec{
        1=>&[48000],3|4|7=>&[32000,44100,48000],2|5=>&[8000,16000,22050,32000,44100,48000,96000],_=>&[32000,44100,48000,96000]
    }}
    fn default_quality(&self)->u32{match self.codec{0=>18,3=>24,5|6|9=>3,8=>30,_=>32}}
    const DITHER:[(&'static str,&'static str);5]=[("无抖动","none"),("Bayer","bayer"),("Floyd–Steinberg","floyd_steinberg"),("Sierra2","sierra2"),("Sierra2 4A","sierra2_4a")];
    fn bitrate_available(&self)->bool{matches!(self.codec,0|1|2|3|6|8|9)}
    fn two_pass_available(&self)->bool{matches!(self.codec,0|1|2|6|9)}
    fn use_two_pass(&self)->bool{self.bitrate_mode&&self.two_pass&&self.two_pass_available()}
    const FPS:[(&'static str,&'static str);9]=[("source","跟随源视频"),("24000/1001","23.976 fps"),("24","24 fps"),("25","25 fps"),("30000/1001","29.97 fps"),("30","30 fps"),("50","50 fps"),("60000/1001","59.94 fps"),("60","60 fps")];
    fn configure(&self,cmd:&mut std::process::Command){
        if self.fps!="source"{cmd.args(["-r",&self.fps,"-fps_mode","cfr"]);}
        else if self.codec==9 {cmd.args(["-r","25","-fps_mode","cfr"]);}
        if self.codec==10 {
            cmd.args(["-c:v","gif","-vf",&format!("{},split[a][b];[a]palettegen=max_colors={}:reserve_transparent=0[p];[b][p]paletteuse=dither={}:bayer_scale={}",self.crop.filter(),self.gif_colors,Self::DITHER[self.gif_dither].1,self.gif_bayer),"-loop","0","-f","gif"]);
            return;
        }
        let pixel=match self.codec{4=>if self.container==4{"bgr24"}else{"rgb24"},5=>"yuvj422p",7=>"yuv422p10le",_=>"yuv420p"};
        cmd.args(["-c:v",self.encoder(),"-vf",&self.crop.filter(),"-pix_fmt",pixel]);
        if self.bitrate_mode&&self.bitrate_available(){
            cmd.args(["-b:v",&format!("{}k",self.video_bitrate)]);
            if self.h26x(){cmd.args(["-preset","medium"]);}
            if self.codec==8{cmd.args(["-preset","8"]);}
        }else{match self.codec{
            0|3=>{cmd.args(["-crf",&self.quality.to_string(),"-preset","medium"]);}
            1|2=>{cmd.args(["-crf",&self.quality.to_string(),"-b:v",if self.codec==1{"2M"}else{"0"}]);}
            5|6|9=>{cmd.args(["-q:v",&self.quality.to_string()]);}
            7=>{cmd.args(["-profile:v","2"]);}
            8=>{cmd.args(["-crf",&self.quality.to_string(),"-preset","8"]);}
            _=>{}
        }}
        if self.codec==3 && matches!(self.container,0|3){cmd.args(["-tag:v","hvc1"]);}
        if matches!(self.container,0|3|7){cmd.args(["-movflags","+faststart"]);}
        cmd.args(["-f",self.muxer()]);
    }
    fn controls(&mut self,ui:&mut egui::Ui){
        ui.label(tr("视频封装格式 & 编码格式"));
        ui.horizontal(|ui|{
        let width=((ui.available_width()-ui.spacing().item_spacing.x)*0.5).max(1.0);
        egui::ComboBox::from_id_salt("video_container").width(width).truncate().selected_text(Self::CONTAINERS[self.container].0).show_ui(ui,|ui|{
            for (index,(label,_,_)) in Self::CONTAINERS.into_iter().enumerate(){
                if ui.selectable_value(&mut self.container,index,label).changed(){
                    if !self.compatible(self.codec){self.codec=match index{1=>2,9=>10,_=>0};self.quality=self.default_quality();}
                    if !self.audio_compatible(self.audio_codec){self.audio_codec=if index==1{1}else{0};}
                }
            }
        }).response.on_hover_text(Self::CONTAINERS[self.container].0);
        egui::ComboBox::from_id_salt("video_codec").width(width).truncate().selected_text(tr(Self::CODECS[self.codec])).show_ui(ui,|ui|{
            for (index,label) in Self::CODECS.into_iter().enumerate(){
                if !self.compatible(index){continue;}
                if ui.selectable_value(&mut self.codec,index,tr(label)).changed(){self.quality=self.default_quality();}
            }
        }).response.on_hover_text(tr(Self::CODECS[self.codec]));
        });
        if self.bitrate_available(){
            let mut mode=if self.bitrate_mode{"bitrate"}else{"quality"}.to_owned();
            crate::theme::compact_selector(ui,&mut mode,[("quality","质量模式"),("bitrate","目标码率")]);
            self.bitrate_mode=mode=="bitrate";
        }
        if self.bitrate_mode&&self.bitrate_available(){
            ui.add(egui::DragValue::new(&mut self.video_bitrate).range(32..=200000).speed(100.0).suffix(" kb/s"));
            if self.two_pass_available(){
                egui::ComboBox::from_id_salt("video_passes").selected_text(if self.two_pass{"2 pass"}else{"1 pass"}).show_ui(ui,|ui|{
                    ui.selectable_value(&mut self.two_pass,false,"1 pass");ui.selectable_value(&mut self.two_pass,true,"2 pass");
                });
            }
        }else if !matches!(self.codec,4|7|10){
            let quantizer=matches!(self.codec,5|6|9);
            let (min,max)=if quantizer{(2,31)}else if self.h26x(){(0,51)}else{(0,63)};
            ui.horizontal(|ui|{ui.label(if quantizer{"q:v"}else{"CRF"});ui.add(egui::DragValue::new(&mut self.quality).range(min..=max));});
        }
        if self.codec==9&&self.fps=="source"{self.fps="25".into();}
        ui.horizontal(|ui|{
            ui.label(tr("帧率"));
            let label=Self::FPS.iter().find(|(id,_)|*id==self.fps).map_or("跟随源视频",|(_,label)|*label);
            egui::ComboBox::from_id_salt("video_fps").selected_text(tr(label)).show_ui(ui,|ui|{
                for (id,label) in Self::FPS{if self.codec==9&&id=="source"{continue;}ui.selectable_value(&mut self.fps,id.to_owned(),tr(label));}
            });
        });
        if self.container==9 {
            ui.label(tr("调色板颜色数"));
            egui::ComboBox::from_id_salt("gif_colors").selected_text(self.gif_colors.to_string()).show_ui(ui,|ui|{for colors in [256,128,64,32,16,8,4]{ui.selectable_value(&mut self.gif_colors,colors,colors.to_string());}});
            ui.label(tr("滤波 / 抖动"));
            egui::ComboBox::from_id_salt("gif_dither").selected_text(tr(Self::DITHER[self.gif_dither].0)).show_ui(ui,|ui|{for(index,(label,_))in Self::DITHER.into_iter().enumerate(){ui.selectable_value(&mut self.gif_dither,index,tr(label));}});
            if self.gif_dither==1{ui.horizontal(|ui|{ui.label(tr("Bayer 强度"));ui.add(egui::DragValue::new(&mut self.gif_bayer).range(0..=5));});}
            ui.label(tr("GIF 不导出音频，最多 256 色；默认循环播放。"));return;
        }
        ui.label(tr("音频编码"));
        egui::ComboBox::from_id_salt("video_audio_codec").selected_text(Self::AUDIO[self.audio_codec].0).show_ui(ui,|ui|{
            for (index,(label,_)) in Self::AUDIO.into_iter().enumerate(){if self.audio_compatible(index){ui.selectable_value(&mut self.audio_codec,index,label);}}
        });
        if !matches!(self.audio_codec,2|5){ui.add(egui::DragValue::new(&mut self.audio_bitrate).range(64..=320).suffix(" kb/s"));}
        let rates=self.audio_rates();
        if !rates.contains(&self.audio_rate){self.audio_rate=48000;}
        ui.horizontal(|ui|{
            ui.label(tr("采样率"));
            egui::ComboBox::from_id_salt("video_audio_rate").selected_text(format!("{} Hz",self.audio_rate)).show_ui(ui,|ui|{for rate in rates{ui.selectable_value(&mut self.audio_rate,*rate,format!("{rate} Hz"));}});
        });
        ui.small(tr("按所选采样率导出双声道音频；未选音轨时不生成音频。"));
    }
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
    pub video_texture: Option<&'a egui::TextureHandle>,
    pub video_size: Option<[usize;2]>,
    pub frame_times: Option<&'a [f64]>,
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
        if video {self.video_tracks.initialize(path,audio,ui.ctx());}
        if let Some(size)=view.video_size {self.video_settings.crop.prepare(size);}
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
                    let result=Err(tr("剪切任务意外中断").into());notify_export_result(&result);
                    self.result = Some(result); self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_some() { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        let title = if video { format!("{} - {}",tr("视频编辑"),path.file_name().unwrap_or_default().to_string_lossy()) } else {
            format!("{} - {}", tr("音频编辑"), path.file_name().unwrap_or_default().to_string_lossy())
        };
        let mut open = self.open;
        crate::native_window::Window::new(title).id(egui::Id::new(("media_trim", path)))
            .open(&mut open).resizable(true).default_width(if video { 1100.0 } else { 900.0 })
            .scrollable(false).min_width(if video {800.0}else{560.0}).default_height(640.0).show(ui.ctx(), |ui| {
                let mut export_requested=false;
                if video {
                    let minimum_width=crate::video_crop::Crop::minimum_width(ui).max(260.0);
                    let maximum_width=(ui.available_width()*0.55).max(minimum_width);
                    egui::SidePanel::right(egui::Id::new(("video_export_settings_fixed",path))).default_width(300.0_f32.max(minimum_width)).width_range(minimum_width..=maximum_width).resizable(true).show_inside(ui,|ui|{
                        ui.horizontal(|ui|{
                            let title=tr("导出设置");
                            let title_width=ui.painter().layout_no_wrap(title.to_owned(),egui::TextStyle::Heading.resolve(ui.style()),ui.visuals().text_color()).size().x;
                            ui.allocate_ui_with_layout(egui::vec2(title_width,40.0),egui::Layout::bottom_up(egui::Align::Min),|ui|{ui.heading(title);});
                            let width=(ui.painter().layout_no_wrap(tr("另存为").to_owned(),egui::FontId::proportional(20.0),egui::Color32::WHITE).size().x+24.0).max(100.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                let button=ui.add_enabled(self.pending.is_none()&&self.video_tracks.valid(self.video_settings.container)&&valid_range(self.start,self.end,duration),egui::Button::new("").fill(egui::Color32::from_rgb(30,140,65)).min_size(egui::vec2(width,40.0)));
                                button.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,button.enabled(),tr("另存为")));
                                ui.painter().text(button.rect.center(),egui::Align2::CENTER_CENTER,tr("另存为"),egui::FontId::proportional(20.0),if button.enabled(){egui::Color32::WHITE}else{ui.visuals().weak_text_color()});
                                export_requested=button.clicked();
                            });
                        });
                        ui.separator();
                        ui.style_mut().spacing.scroll.floating=false;
                        egui::ScrollArea::vertical().show(ui,|ui|{
                            ui.add_enabled_ui(self.pending.is_none(),|ui|{self.video_settings.controls(ui);});
                            ui.add_enabled_ui(self.pending.is_none(),|ui|{self.video_tracks.controls(ui,self.video_settings.container,path,audio);});
                            ui.add_enabled_ui(self.pending.is_none(),|ui|{self.video_settings.crop.controls(ui);});
                            ui.small(&view.description);
                            ui.add_space(8.0);
                            ui.label(tr("按列表导出已勾选的音轨与字幕；语言写入轨道元数据。"));
                            ui.small(tr("重新编码，边界受帧 / 采样精度限制；不覆盖原文件或已有文件。"));
                        });
                    });
                }
                {
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
                if video {
                    egui::TopBottomPanel::top("video_trim_range").resizable(false).show_inside(ui,|ui|{
                        ui.add_enabled_ui(self.pending.is_none(),|ui|{
                            ui.horizontal_wrapped(|ui|{self.range_values(ui,duration,position);});
                            ui.horizontal_wrapped(|ui|{self.range_actions(ui,duration,false,&mut action);ui.label(format!("{}: {:.3} s",tr("区间时长"),(self.end-self.start).max(0.0)));});
                        });
                    });
                    egui::TopBottomPanel::bottom("video_edit_timeline").exact_height(88.0).resizable(false).show_inside(ui,|ui|{
                        ui.add_enabled_ui(self.pending.is_none(),|ui|{
                            if let Some(seek)=crate::media_preview::paint_video_edit_timeline(ui,position,duration,&mut self.start,&mut self.end,view.playing,view.frame_times){action.seek=Some(seek);}
                        });
                    });
                }
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
                    if !video {ui.label(tr("仅导出当前音轨；音量控件仅影响试听。"));
                    ui.small(tr("重新编码，边界受帧 / 采样精度限制；不覆盖原文件或已有文件。"));
                    }
                    let valid = valid_range(self.start, self.end, duration);
                    if !valid && !self.session.as_ref().is_some_and(|session|session.state.interval()) { ui.colored_label(ui.visuals().warn_fg_color, tr("终点必须大于起点且不超过媒体时长")); }
                    if export_requested {
                        let extension = if video { self.video_settings.extension() } else { self.settings.format.extension() };
                        let name = format!("{}_trim.{extension}", path.file_stem().unwrap_or_default().to_string_lossy());
                        let dialog = rfd::FileDialog::new().set_file_name(&name);
                        let dialog = if video {dialog}else{dialog.add_filter(extension,&[extension])};
                        let dialog = if let Some(parent) = path.parent() { dialog.set_directory(parent) } else { dialog };
                        if let Some(target) = dialog.save_file() {
                            let target = if video && target.extension().is_some(){target}else{target.with_extension(extension)};
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
                                let video_settings=self.video_settings.clone();
                                let tracks=self.video_tracks.entries.clone();
                                std::thread::spawn(move || {
                                    let _cache=clip;
                                    let result = export(&ffmpeg, &source, &target, start, end, duration, video, audio, &settings,&video_settings,&tracks,filter.as_deref());
                                    notify_export_result(&result);
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

                } else {
                    let available=egui::vec2(ui.available_width().max(1.0),ui.available_height().max(1.0));
                    let (rect,response)=ui.allocate_exact_size(available,egui::Sense::click());
                    ui.painter().rect_filled(rect,0.0,egui::Color32::BLACK);
                    if let Some(texture)=view.video_texture {
                        let size=self.video_settings.crop.preview_size(texture.size_vec2());let scale=(rect.width()/size.x).min(rect.height()/size.y);
                        let image=egui::Rect::from_center_size(rect.center(),size*scale);
                        ui.painter().image(texture.id(),image,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),egui::Color32::WHITE);
                        self.video_settings.crop.overlay(ui,image,self.pending.is_none());
                    } else {ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,tr("播放"),egui::FontId::proportional(16.0),egui::Color32::WHITE);}
                    if response.clicked() && !self.video_settings.crop.enabled {action.toggle=true;}
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

fn notify_export_result(result:&Result<PathBuf,String>){
    let (title,description,level)=match result {
        Ok(path)=>(tr("导出完成").to_owned(),path.display().to_string(),rfd::MessageLevel::Info),
        Err(error)=>(tr("导出失败").to_owned(),error.clone(),rfd::MessageLevel::Error),
    };
    std::thread::spawn(move||{rfd::MessageDialog::new().set_title(title).set_description(description).set_level(level).show();});
}

struct PassLogs(PathBuf);
#[cfg(test)]
mod rate_control_tests {
    use super::*;
    #[test]
    fn default_is_single_pass_quality_and_source_fps(){
        let settings=VideoSettings::default();assert!(!settings.use_two_pass());assert!(!settings.bitrate_mode);assert_eq!(settings.fps,"source");
    }
    #[test]
    fn two_pass_requires_bitrate_and_supported_codec(){
        let mut settings=VideoSettings{two_pass:true,..Default::default()};assert!(!settings.use_two_pass());
        settings.bitrate_mode=true;assert!(settings.use_two_pass());
        settings.codec=3;assert!(!settings.use_two_pass());settings.codec=10;assert!(!settings.use_two_pass());
    }
    #[test]
    fn target_bitrate_omits_quality_parameter(){
        let settings=VideoSettings{bitrate_mode:true,fps:"30000/1001".into(),..Default::default()};
        let mut cmd=std::process::Command::new("ffmpeg");settings.configure(&mut cmd);
        let args:Vec<_>=cmd.get_args().map(|arg|arg.to_string_lossy().to_string()).collect();
        assert!(!args.iter().any(|arg|arg=="-crf"));assert!(args.windows(2).any(|pair|pair==["-b:v","6000k"]));
        assert!(args.windows(2).any(|pair|pair==["-r","30000/1001"]));
    }
}
impl Drop for PassLogs {
    fn drop(&mut self){
        for suffix in ["-0.log","-0.log.mbtree","-0.log.temp","-0.log.mbtree.temp"] {
            let mut path=self.0.as_os_str().to_os_string();path.push(suffix);let _=std::fs::remove_file(PathBuf::from(path));
        }
    }
}

fn export(ffmpeg: &Path, source: &Path, target: &Path, start: f64, end: f64, duration: f64, video: bool, audio: Option<u64>, settings: &AudioSettings, video_settings:&VideoSettings, tracks:&[crate::export_tracks::Track], audio_filter:Option<&str>) -> Result<PathBuf, String> {
    let audio=if video && video_settings.container==9{None}else{audio};
    if !valid_range(start, end, duration) { return Err("Invalid trim range".into()); }
    // Encode to a unique sibling, then exclusively create the destination. This
    // works on filesystems without hard links and never replaces an existing file.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let extension = if video { video_settings.extension() } else { settings.format.extension() };
    let temp = target.with_file_name(format!(".unity-trim-{}-{}.{extension}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let reservation = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e| e.to_string())?;
    drop(reservation);
    let result = (|| {
        let mut cmd = command(ffmpeg);
        cmd.args(["-nostdin", "-v", "error", "-y", "-ss", &start.to_string(),"-t",&(end-start).to_string()]).arg("-i").arg(source);
        let has_audio=video&&video_settings.container!=9&&tracks.iter().any(|t|t.enabled&&!t.subtitle);
        let _subtitle_inputs;
        if video {
            _subtitle_inputs=Some(crate::export_tracks::configure(&mut cmd,tracks,source,video_settings.container,start,end-start,ffmpeg,&temp)?);
            let encoders=command(ffmpeg).args(["-hide_banner","-encoders"]).output().map_err(|e|e.to_string())?;
            let available=String::from_utf8_lossy(&encoders.stdout);
            for encoder in std::iter::once(video_settings.encoder()).chain(has_audio.then_some(video_settings.audio_encoder())) {
                if !encoders.status.success() || !available.lines().any(|line|line.split_whitespace().nth(1)==Some(encoder)){return Err(format!("FFmpeg encoder unavailable: {encoder}"));}
            }
            video_settings.configure(&mut cmd);
        } else { _subtitle_inputs=None;cmd.arg("-vn"); }
        if video {
            if has_audio {
                cmd.args(["-c:a",video_settings.audio_encoder()]);
                if !matches!(video_settings.audio_codec,2|5){cmd.args(["-b:a",&format!("{}k",video_settings.audio_bitrate)]);}
                cmd.args(["-ar",&video_settings.audio_rate.to_string(),"-ac","2"]);
            }
        }else if let Some(index)=audio{
            cmd.args(["-map",&format!("0:{index}")]);
            if let Some(filter)=audio_filter{cmd.args(["-af",filter]);}settings.configure(&mut cmd);
        } else { cmd.arg("-an"); }
        if !video{cmd.arg("-sn");}
        cmd.args(["-t",&(end-start).to_string(),"-dn", "-map_metadata", "-1"]);
        let _pass_logs;
        if video&&video_settings.use_two_pass(){
            let prefix=temp.with_extension("pass");
            _pass_logs=Some(PassLogs(prefix.clone()));
            let args:Vec<_>=cmd.get_args().map(|a|a.to_os_string()).collect();
            let inputs_end=args.iter().position(|arg|arg=="-map").ok_or("Missing video map")?;
            let mut first=command(ffmpeg);first.args(&args[..inputs_end]);
            video_settings.configure(&mut first);
            first.args(["-map","0:V:0","-an","-sn","-t",&(end-start).to_string(),"-pass","1","-passlogfile"]).arg(&prefix).args(["-f","null","-"]);
            let result=first.output().map_err(|e|e.to_string())?;
            if !result.status.success(){return Err(format!("Pass 1: {}",String::from_utf8_lossy(&result.stderr)));}
            cmd.args(["-pass","2","-passlogfile"]).arg(&prefix);
        }else{_pass_logs=None;}
        let output = cmd.arg(&temp).output().map_err(|e| e.to_string())?;
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
