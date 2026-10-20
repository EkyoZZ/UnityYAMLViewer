use std::{path::{Path,PathBuf},sync::{Arc,mpsc}};
use eframe::egui;
use crate::localization::tr;

// Enumerate installed family names, not font filenames, for libass FontName.
#[cfg(windows)]
fn installed_font_families()->Vec<String> {
    use std::ffi::c_void;
    #[repr(C)]
    struct LogFont {height:i32,width:i32,escapement:i32,orientation:i32,weight:i32,
        italic:u8,underline:u8,strike_out:u8,char_set:u8,out_precision:u8,clip_precision:u8,quality:u8,pitch_and_family:u8,face_name:[u16;32]}
    #[link(name="user32")]
    unsafe extern "system" {fn GetDC(window:*mut c_void)->*mut c_void;fn ReleaseDC(window:*mut c_void,dc:*mut c_void)->i32;}
    #[link(name="gdi32")]
    unsafe extern "system" {fn EnumFontFamiliesExW(dc:*mut c_void,font:*const LogFont,callback:unsafe extern "system" fn(*const LogFont,*const c_void,u32,isize)->i32,param:isize,flags:u32)->i32;}
    unsafe extern "system" fn collect(font:*const LogFont,_metrics:*const c_void,_kind:u32,param:isize)->i32 {
        if font.is_null() {return 1;}
        // EnumFontFamiliesExW invokes this synchronously while the vector is alive.
        let name=unsafe {&(*font).face_name};
        let length=name.iter().position(|c|*c==0).unwrap_or(name.len());
        let name=String::from_utf16_lossy(&name[..length]);
        if !name.is_empty() && !name.starts_with('@') {unsafe {&mut *(param as *mut Vec<String>)}.push(name);}
        1
    }
    let mut names=Vec::new();
    unsafe {
        let dc=GetDC(std::ptr::null_mut());
        if dc.is_null() {return names;}
        let mut font:LogFont=std::mem::zeroed();font.char_set=1; // DEFAULT_CHARSET
        EnumFontFamiliesExW(dc,&font,collect,&mut names as *mut Vec<String> as isize,0);
        ReleaseDC(std::ptr::null_mut(),dc);
    }
    names.sort_by_key(|name|name.to_lowercase());names.dedup_by(|a,b|a.eq_ignore_ascii_case(b));names
}
#[cfg(not(windows))]
fn installed_font_families()->Vec<String> {Vec::new()}
fn system_font_families()->&'static [String] {
    static FONTS:std::sync::OnceLock<Vec<String>>=std::sync::OnceLock::new();
    FONTS.get_or_init(installed_font_families)
}

#[derive(Clone)]
pub struct Source {pub path:PathBuf,pub ordinal:usize,pub label:String,pub default:bool,pub text:bool,pub language:Option<&'static str>}
pub struct Asset(PathBuf);
impl Drop for Asset{fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}
#[derive(Clone)]
pub struct Style {font:String,size:f32,color:egui::Color32,outline:bool,outline_size:f32,outline_color:egui::Color32,shadow:bool,shadow_size:f32,shadow_color:egui::Color32,margin_x:u32,margin_y:u32}
impl Default for Style{fn default()->Self{Self{font:String::new(),size:24.0,color:egui::Color32::WHITE,outline:true,outline_size:2.0,outline_color:egui::Color32::BLACK,shadow:false,shadow_size:2.0,shadow_color:egui::Color32::BLACK,margin_x:20,margin_y:20}}}
#[derive(Clone)]
pub struct Render {asset:Arc<Asset>,style:Style}
impl Render{
    pub fn filter(&self,position:f64,base:&str)->String{
        // Escape the filename for both the filter-option and filtergraph parsers.
        let path=self.asset.0.to_string_lossy().replace('\\',"/").replace(':',"\\:").replace('\'',"'\\''");
        let color=|c:egui::Color32|format!("&H{:02X}{:02X}{:02X}{:02X}",255-c.a(),c.b(),c.g(),c.r());
        let s=&self.style;
        let font=if s.font.trim().is_empty(){"Segoe UI"}else{s.font.trim()};
        let font:String=font.chars().filter(|c|!matches!(c,','|'\''|':'|'['|']'|'\\'|'\n'|'\r')).collect();
        let style=format!("FontName={font},FontSize={},PrimaryColour={},OutlineColour={},BackColour={},BorderStyle=1,Outline={},Shadow={},MarginL={},MarginR={},MarginV={},Alignment=2",s.size,color(s.color),color(s.outline_color),color(s.shadow_color),if s.outline{s.outline_size}else{0.0},if s.shadow{s.shadow_size}else{0.0},s.margin_x,s.margin_x,s.margin_y);
        format!("setpts=PTS+{position}/TB,subtitles=filename='{path}':force_style='{style}',setpts=PTS-STARTPTS,{base}")
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn source(language:Option<&'static str>)->Source{Source{path:PathBuf::from("source.mkv"),ordinal:0,label:String::new(),default:false,text:true,language}}
    #[test]
    fn unknown_audio_requests_manual_selection(){
        let mut subtitles=Subtitles::default();
        subtitles.follow_audio(None,&[source(Some("英语"))],&egui::Context::default());
        assert!(subtitles.manual_required);assert!(subtitles.selected.is_none());
    }
    #[test]
    fn ambiguous_language_does_not_guess(){
        let mut subtitles=Subtitles::default();let a=source(Some("英语"));let mut b=a.clone();b.ordinal=1;
        subtitles.follow_audio(Some("英语"),&[a,b],&egui::Context::default());
        assert!(subtitles.manual_required);assert!(subtitles.job.is_none());
    }
    #[test]
    fn manual_override_survives_audio_changes(){
        let mut subtitles=Subtitles{manual_override:true,selected:Some(source(Some("中文"))),..Default::default()};
        subtitles.follow_audio(Some("英语"),&[],&egui::Context::default());
        assert_eq!(subtitles.selected.unwrap().language,Some("中文"));
    }
}
#[derive(Default)]
pub struct Subtitles {pub revision:u64,asset:Option<Arc<Asset>>,selected:Option<Source>,external:Vec<Source>,job:Option<mpsc::Receiver<Result<Arc<Asset>,String>>>,initialized:bool,style:Style,draft:Style,settings_open:bool,font_search:String,error:Option<String>,manual_required:bool,manual_override:bool,needs_follow:bool}
impl Subtitles {
    pub fn session_copy(&self)->Self{Self{revision:1,asset:self.asset.clone(),selected:self.selected.clone(),external:self.external.clone(),initialized:true,style:self.style.clone(),manual_override:self.manual_override,manual_required:self.manual_required,..Default::default()}}
    pub fn render(&self)->Option<Render>{Some(Render{asset:self.asset.clone()?,style:self.style.clone()})}
    pub fn follow_audio(&mut self,language:Option<&str>,sources:&[Source],ctx:&egui::Context){
        if self.manual_override{return;}
        let matches:Vec<_>=sources.iter().chain(self.external.iter()).filter(|s|s.text&&s.language.is_some()&&s.language!=Some("未标注语言")&&s.language.as_deref()==language&&language.is_some()).cloned().collect();
        let chosen=if matches.len()==1{matches.first()}else{
            let defaults:Vec<_>=matches.iter().filter(|s|s.default).collect();if defaults.len()==1{defaults.first().copied()}else{None}
        };
        if let Some(source)=chosen{
            self.manual_required=false;
            if !self.selected.as_ref().is_some_and(|s|s.path==source.path&&s.ordinal==source.ordinal){self.load(source.clone(),ctx);}
        }else{
            self.manual_required=!sources.is_empty()||!self.external.is_empty();
            if self.manual_required{self.asset=None;self.selected=None;self.job=None;self.revision+=1;}
        }
    }
    pub fn status(&self,ui:&mut egui::Ui){
        if self.manual_required {ui.colored_label(ui.visuals().warn_fg_color,tr("未能匹配音轨语言字幕，请右键打开字幕菜单手动选择或加载外挂字幕。"));}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
    }
    pub fn poll(&mut self,sources:&[Source],language:Option<&str>,ctx:&egui::Context){
        if !self.initialized {self.initialized=true;self.follow_audio(language,sources,ctx);}
        if self.needs_follow{self.needs_follow=false;self.follow_audio(language,sources,ctx);}
        if let Some(job)=&self.job{match job.try_recv(){
            Ok(result)=>{self.job=None;match result{Ok(asset)=>{self.asset=Some(asset);self.revision+=1;},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.job=None;self.error=Some(tr("字幕加载失败").to_owned());},
            Err(mpsc::TryRecvError::Empty)=>ctx.request_repaint_after(std::time::Duration::from_millis(100))
        }}
    }
    fn load(&mut self,source:Source,ctx:&egui::Context){
        self.asset=None;self.selected=Some(source.clone());self.error=None;self.revision+=1;
        let (tx,rx)=mpsc::channel();self.job=Some(rx);let ctx=ctx.clone();
        std::thread::spawn(move||{
            let result=(||{
                let ffmpeg=crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
                let filters=crate::media_tools::command(&ffmpeg).args(["-hide_banner","-filters"]).output().map_err(|e|e.to_string())?;
                if !String::from_utf8_lossy(&filters.stdout).lines().any(|line|line.split_whitespace().nth(1)==Some("subtitles")){return Err("FFmpeg subtitles / libass filter unavailable".to_owned());}
                static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
                let path=std::env::temp_dir().join(format!("unity-subtitle-{}-{}.ass",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
                let file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;drop(file);
                let asset=Arc::new(Asset(path));
                let output=crate::media_tools::command(&ffmpeg).args(["-nostdin","-v","error","-y","-i"]).arg(&source.path).args(["-map",&format!("0:s:{}",source.ordinal),"-c:s","ass","-f","ass"]).arg(&asset.0).output().map_err(|e|e.to_string())?;
                if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).to_string());}
                Ok(asset)
            })();let _=tx.send(result);ctx.request_repaint();
        });
    }
    pub fn menu(&mut self,ui:&mut egui::Ui,sources:&[Source]){
        ui.menu_button(tr("字幕"),|ui|{
            if ui.checkbox(&mut self.manual_override,tr("手动选择字幕（关闭语言跟随）")).changed(){self.manual_required=false;self.needs_follow=!self.manual_override;}
            if ui.selectable_label(self.selected.is_none(),tr("关闭字幕")).clicked(){self.manual_override=true;self.manual_required=false;self.selected=None;self.asset=None;self.job=None;self.revision+=1;ui.close_menu();}
            let choices:Vec<_>=sources.iter().chain(self.external.iter()).cloned().collect();
            for source in choices{
                let selected=self.selected.as_ref().is_some_and(|s|s.path==source.path&&s.ordinal==source.ordinal);
                if ui.add_enabled(source.text,egui::SelectableLabel::new(selected,&source.label)).clicked(){self.manual_override=true;self.manual_required=false;self.load(source,ui.ctx());ui.close_menu();}
            }
            if sources.iter().any(|s|!s.text){ui.small(tr("图像字幕暂不支持播放器显示；可加载文字外挂字幕。"));}
            if ui.button(tr("加载外挂字幕")).clicked(){
                if let Some(path)=rfd::FileDialog::new().add_filter(tr("字幕"),&["srt","ass","ssa","vtt"]).pick_file(){
                    let stem=path.file_stem().unwrap_or_default().to_string_lossy();let language=stem.split(['.','_','-',' ']).rev().find_map(|part|crate::media_preview::audio_language_name(part,"und").filter(|name|*name!="未标注语言"));
                    let source=Source{label:path.file_name().unwrap_or_default().to_string_lossy().to_string(),path,ordinal:0,default:false,text:true,language};
                    if !self.external.iter().any(|s|s.path==source.path){self.external.push(source.clone());}
                    self.manual_override=true;self.manual_required=false;self.load(source,ui.ctx());
                }ui.close_menu();
            }
            if ui.button(tr("字幕样式")).clicked(){self.draft=self.style.clone();self.font_search.clear();self.settings_open=true;ui.close_menu();}
            if self.job.is_some(){ui.spinner();}
            if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
        });
    }
    pub fn style_window(&mut self,ctx:&egui::Context,path:&Path){
        if !self.settings_open{return;}
        let mut open=true;let mut apply=false;let mut cancel=false;
        crate::native_window::Window::new(tr("字幕样式")).id(egui::Id::new(("subtitle_style",path))).open(&mut open).default_width(320.0).fit_height().show(ctx,|ui|{
            let s=&mut self.draft;
            ui.label(tr("字体"));
            let fonts=system_font_families();
            let selected=if s.font.is_empty(){tr("系统默认字体")}else{s.font.as_str()};
            egui::ComboBox::from_id_salt("subtitle_system_font").width(ui.available_width()).height(220.0)
                .selected_text(selected).show_ui(ui,|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.font_search).hint_text(tr("搜索字体")));
                    ui.selectable_value(&mut s.font,String::new(),tr("系统默认字体"));
                    let search=self.font_search.to_lowercase();
                    for family in fonts.iter().filter(|name|name.to_lowercase().contains(&search)) {
                        ui.selectable_value(&mut s.font,family.clone(),family);
                    }
                    if fonts.is_empty() {ui.label(tr("未能获取系统字体列表"));}
                });
            ui.horizontal(|ui|{ui.label(tr("字号"));ui.add(egui::DragValue::new(&mut s.size).range(8.0..=120.0));ui.color_edit_button_srgba(&mut s.color);});
            ui.horizontal(|ui|{ui.checkbox(&mut s.outline,tr("描边"));ui.add_enabled_ui(s.outline,|ui|{ui.add(egui::DragValue::new(&mut s.outline_size).range(0.0..=12.0));ui.color_edit_button_srgba(&mut s.outline_color);});});
            ui.horizontal(|ui|{ui.checkbox(&mut s.shadow,tr("投影"));ui.add_enabled_ui(s.shadow,|ui|{ui.add(egui::DragValue::new(&mut s.shadow_size).range(0.0..=12.0));ui.color_edit_button_srgba(&mut s.shadow_color);});});
            ui.horizontal(|ui|{ui.label(tr("水平边距"));ui.add(egui::DragValue::new(&mut s.margin_x).range(0..=300));});
            ui.horizontal(|ui|{ui.label(tr("底部边距"));ui.add(egui::DragValue::new(&mut s.margin_y).range(0..=300));});
            ui.separator();
            ui.horizontal(|ui| {
                let font=egui::FontId::proportional(16.0);
                let labels=[tr("应用"),tr("取消"),tr("重置")];
                let widths=labels.map(|label|ui.painter().layout_no_wrap(label.to_owned(),font.clone(),ui.visuals().text_color()).size().x+24.0);
                let widths=widths.map(|width|width.max(88.0));
                let total=widths.iter().sum::<f32>()+ui.spacing().item_spacing.x*2.0;
                ui.add_space(((ui.available_width()-total)/2.0).max(0.0));
                apply=ui.add_sized([widths[0],32.0],egui::Button::new(egui::RichText::new(labels[0]).size(16.0))).clicked();
                cancel=ui.add_sized([widths[1],32.0],egui::Button::new(egui::RichText::new(labels[1]).size(16.0))).clicked();
                if ui.add_sized([widths[2],32.0],egui::Button::new(egui::RichText::new(labels[2]).size(16.0))).clicked() {self.draft=Style::default();self.font_search.clear();}
            });
        });
        if apply{self.style=self.draft.clone();self.revision+=1;open=false;}
        if cancel{open=false;}
        self.settings_open=open;
    }
}
