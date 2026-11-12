use std::{path::{Path,PathBuf},sync::{Arc,mpsc}};
use eframe::egui;
use crate::localization::tr;
mod editor_style;
pub(crate) use editor_style::EditorPreviewStyle;

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

#[derive(Clone, PartialEq, Eq)]
pub struct Source {pub path:PathBuf,pub ordinal:usize,pub codec:String,pub label:String,pub default:bool,pub text:bool,pub language:Option<&'static str>}
pub struct Asset(PathBuf);
impl Drop for Asset{fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Style {
    font:String,
    size:f32,
    #[serde(with="style_color")]
    color:egui::Color32,
    outline:bool,
    outline_size:f32,
    #[serde(with="style_color")]
    outline_color:egui::Color32,
    shadow:bool,
    shadow_x:f32,
    shadow_y:f32,
    #[serde(with="style_color")]
    shadow_color:egui::Color32,
    margin_x:u32,
    margin_y:u32,
    top:bool,
    native_ass:bool,
}
impl Default for Style{fn default()->Self{Self{font:String::new(),size:24.0,color:egui::Color32::WHITE,outline:true,outline_size:2.0,outline_color:egui::Color32::BLACK,shadow:false,shadow_x:2.0,shadow_y:2.0,shadow_color:egui::Color32::BLACK,margin_x:20,margin_y:20,top:false,native_ass:false}}}
mod style_color {
    use eframe::egui::Color32;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(color:&Color32, serializer:S)->Result<S::Ok,S::Error>{
        color.to_array().serialize(serializer)
    }

    pub fn deserialize<'de,D: Deserializer<'de>>(deserializer:D)->Result<Color32,D::Error>{
        let [r,g,b,a]=<[u8;4]>::deserialize(deserializer)?;
        Ok(Color32::from_rgba_premultiplied(r,g,b,a))
    }
}

fn style_path()->Option<PathBuf>{
    crate::last_open_dir_config_path().map(|path|path.with_file_name("subtitle_style.json"))
}

fn decode_style(bytes:&[u8])->Result<Style,serde_json::Error>{
    let mut value:serde_json::Value=serde_json::from_slice(bytes)?;
    if let Some(object)=value.as_object_mut(){
        if let Some(size)=object.get("shadow_size").cloned(){
            object.entry("shadow_x").or_insert(size.clone());
            object.entry("shadow_y").or_insert(size);
        }
    }
    serde_json::from_value(value)
}

// Split plain long cues by actual two-line capacity, preferring clause boundaries.
// Explicit multiline/bilingual text and ASS effects retain their original timing.
fn cue_pages(text:&str,measure:&mut crate::subtitle_wrap::Measure,width:f32)->Vec<String>{
    let mut remaining=text.to_owned();let mut pages=Vec::new();
    loop {
        let wrapped=measure.wrap(&remaining,width);
        let rows:Vec<_>=wrapped.split("\\N").collect();
        if rows.len()<=2 {pages.push(wrapped);break;}
        let capacity=rows[..2].iter().map(|row|row.chars().count()).sum::<usize>();
        let chars:Vec<_>=remaining.chars().collect();
        let end=capacity.min(chars.len());
        let cut=(end/2..end).rev().find(|&i|matches!(chars[i],'。'|'！'|'？'|'；'|'，'|'、'|'.'|'!'|'?'|';'|','|' '))
            .map_or(end,|i|i+1).max(1);
        let page:String=chars[..cut].iter().collect();
        pages.push(measure.wrap(&page,width));
        remaining=chars[cut..].iter().collect();
        if remaining.is_empty(){break;}
    }
    pages
}
fn ass_centiseconds(value:&str)->Option<u64>{
    let parts:Vec<_>=value.trim().split(':').collect();if parts.len()!=3{return None;}
    let (seconds,fraction)=parts[2].split_once('.')?;
    if fraction.len()!=2{return None;}
    Some(parts[0].parse::<u64>().ok()?*360000+parts[1].parse::<u64>().ok()?*6000+seconds.parse::<u64>().ok()?*100+fraction.parse::<u64>().ok()?)
}
fn ass_time(value:u64)->String{format!("{}:{:02}:{:02}.{:02}",value/360000,value/6000%60,value/100%60,value%100)}

// ASS styles expose only one Shadow value. Override tags provide independent signed axes.
fn apply_shadow_offsets(text:&str,style:&Style)->String{
    apply_shadow_offsets_mode(text,style,true)
}
fn apply_shadow_offsets_mode(text:&str,style:&Style,paginate:bool)->String{
    if style.native_ass {return text.to_owned();}
    let (x,y)=if style.shadow{(style.shadow_x,style.shadow_y)}else{(0.0,0.0)};
    let alignment=if style.top{8}else{2};
    let tags=format!("{{\\q2\\an{alignment}\\xshad{x}\\yshad{y}}}");
    let script_width=text.lines().find_map(|line|line.trim().strip_prefix("PlayResX:")?.trim().parse::<f32>().ok()).filter(|w|w.is_finite()&&*w>=64.0).unwrap_or(384.0);
    let safe_margin=(style.margin_x as f32).min(((script_width-style.size*2.5)/2.0).max(0.0));
    let available=(script_width-safe_margin*2.0-style.outline_size*2.0-style.shadow_x.abs()-4.0).max(style.size*1.5);
    let mut measure=crate::subtitle_wrap::Measure::new(if style.font.is_empty(){"Segoe UI"}else{&style.font},style.size);

    let positioning=regex::Regex::new(r"\\(?:q[0-3]|an[1-9]|(?:pos|move|org)\([^)]*\)|(?:fs[cy]?[xy]?|fsp)[-+0-9.]+|fn[^\\}]+)").unwrap();
    let overrides=regex::Regex::new(r"\{[^}]*\}").unwrap();
    text.lines().map(|line|{
        if line.starts_with("Dialogue:") {
            let mut fields:Vec<_>=line.splitn(10,',').collect();
            if fields.len()==10 {
                // Per-dialogue margins and absolute positioning otherwise bypass force_style.
                let margin_x=(safe_margin.floor() as u32).to_string();let margin_y=style.margin_y.to_string();
                // Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text.
                fields[5]=&margin_x;fields[6]=&margin_x;fields[7]=&margin_y;
                let cleaned=overrides.replace_all(fields[9],|caps:&regex::Captures|positioning.replace_all(&caps[0],"").into_owned());
                // Only plain single-language cues are safe to retime without word timestamps.
                if paginate && !fields[9].contains(['{','}']) && !fields[9].contains("\\N") && !fields[9].contains("\\n") && fields[8].trim().is_empty() {
                    let pages=cue_pages(&cleaned,&mut measure,available);
                    if pages.len()>1 {
                        if let (Some(start),Some(end))=(ass_centiseconds(fields[1]),ass_centiseconds(fields[2])) {
                            // Avoid unreadably short flashes; retain normal wrapping for very fast speech.
                            if end.saturating_sub(start)>=pages.len() as u64*100 {
                                let weights:Vec<u64>=pages.iter().map(|page|page.replace("\\N","").chars().count().max(1) as u64).collect();
                                let total:u64=weights.iter().sum();let spare=end-start-pages.len() as u64*100;
                                let mut output=String::new();let mut used=0;let mut previous=start;
                                for (index,page) in pages.iter().enumerate(){
                                    used+=weights[index];let boundary=if index+1==pages.len(){end}else{start+(index as u64+1)*100+spare*used/total};
                                    let mut timed:Vec<String>=fields[..9].iter().map(|field|field.to_string()).collect();
                                    timed[1]=ass_time(previous);timed[2]=ass_time(boundary);
                                    output.push_str(&format!("{},{tags}{page}\n",timed.join(",")));
                                    previous=boundary;
                                }
                                return output;
                            }
                        }
                    }
                }
                let wrapped=measure.wrap(&cleaned,available);
                let body=format!("{tags}{}",wrapped.replace('}',&format!("}}{tags}")));
                fields.pop();return format!("{},{body}\n",fields.join(","));
            }
        }
        format!("{line}\n")
    }).collect()
}

fn prepare_ass(text:&str,style:&Style,video_size:Option<[usize;2]>)->String{
    if style.native_ass {return text.to_owned();}
    let Some([width,height])=video_size.filter(|size|size[0]>0 && size[1]>0) else{return apply_shadow_offsets(text,style);};
    let script_height=text.lines().find_map(|line|line.trim().strip_prefix("PlayResY:")?.trim().parse::<u32>().ok()).filter(|h|*h>0).unwrap_or(288);
    let script_width=(script_height as f64*width as f64/height as f64).round().max(1.0) as u32;
    // libass fonts scale with script height. Match the uniform canvas aspect to the video
    // so measuring against PlayResX does not shrink widescreen text into a 4:3 column.
    let mut found=false;
    let mut normalized:String=text.lines().map(|line|{
        if line.trim().starts_with("PlayResX:"){found=true;format!("PlayResX: {script_width}\n")}else{format!("{line}\n")}
    }).collect();
    if !found {normalized=normalized.replacen("[Script Info]",&format!("[Script Info]\nPlayResX: {script_width}"),1);}
    apply_shadow_offsets(&normalized,style)
}

pub(crate) fn live_top()->bool{style_preference().lock().unwrap().top}

pub(crate) fn live_margins()->(f32,f32){let style=style_preference().lock().unwrap();(style.margin_x as f32,style.margin_y as f32)}

fn bake_ass_style(text:&str,style:&Style)->String{
    if style.native_ass {return text.to_owned();}
    let color=|c:egui::Color32|format!("&H{:02X}{:02X}{:02X}{:02X}",255-c.a(),c.b(),c.g(),c.r());
    let font=if style.font.trim().is_empty(){"Segoe UI"}else{style.font.trim()};
    let font:String=font.chars().filter(|c|!matches!(c,','|'\n'|'\r')).collect();
    text.lines().map(|line|{
        if let Some(value)=line.strip_prefix("Style:"){
            let name=value.split(',').next().unwrap_or("Default").trim();
            format!("Style: {name},{font},{},{},{},{},{},0,0,0,0,100,100,0,0,1,{},0,{},{},{},{},1\n",style.size,color(style.color),color(style.color),color(style.outline_color),color(style.shadow_color),if style.outline{style.outline_size}else{0.0},if style.top{8}else{2},style.margin_x,style.margin_x,style.margin_y)
        }else{format!("{line}\n")}
    }).collect()
}

fn style_preference()->&'static std::sync::Mutex<Style>{
    static STYLE:std::sync::OnceLock<std::sync::Mutex<Style>>=std::sync::OnceLock::new();
    STYLE.get_or_init(||{
        let mut style:Style=style_path().and_then(|path|std::fs::read(path).ok())
            .and_then(|bytes|decode_style(&bytes).ok()).unwrap_or_default();
        style.size=style.size.clamp(8.0,120.0);
        style.outline_size=style.outline_size.clamp(0.0,12.0);
        style.shadow_x=style.shadow_x.clamp(-12.0,12.0);
        style.shadow_y=style.shadow_y.clamp(-12.0,12.0);
        style.margin_x=style.margin_x.min(300);
        style.margin_y=style.margin_y.min(300);
        std::sync::Mutex::new(style)
    })
}

fn save_style(style:&Style)->Result<(),String>{
    let mut preference=style_preference().lock().unwrap();
    let path=style_path().ok_or_else(||"APPDATA is unavailable".to_owned())?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|error|error.to_string())?;
    let bytes=serde_json::to_vec_pretty(style).map_err(|error|error.to_string())?;
    let temporary=path.with_extension(format!("{}.tmp",std::process::id()));
    let result=(||{
        use std::io::Write;
        let mut file=std::fs::File::create(&temporary).map_err(|error|error.to_string())?;
        file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|error|error.to_string())?;
        drop(file);
        crate::playlist::replace(&temporary,&path)
    })();
    if result.is_err(){let _=std::fs::remove_file(&temporary);}
    result?;
    *preference=style.clone();
    Ok(())
}

#[derive(Clone)]
pub struct Render {asset:Arc<Asset>,style:Style}
impl Render{
    pub(crate) fn path(&self)->&Path{&self.asset.0}
    pub(crate) fn draft(text:&str)->Result<Self,String>{
        use std::io::Write;
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        let path=std::env::temp_dir().join(format!("uyv-subtitle-draft-{}-{}.ass",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
        let asset=Arc::new(Asset(path));file.write_all(text.as_bytes()).map_err(|e|e.to_string())?;
        Ok(Self{asset,style:Style{native_ass:true,..Default::default()}})
    }
    pub fn filter(&self,position:f64,base:&str)->String{
        // Escape the filename for both the filter-option and filtergraph parsers.
        let path=self.asset.0.to_string_lossy().replace('\\',"/").replace(':',"\\:").replace('\'',"'\\''");
        if self.style.native_ass {return format!("setpts=PTS+{position}/TB,subtitles=filename='{path}',setpts=PTS-STARTPTS,{base}");}
        let color=|c:egui::Color32|format!("&H{:02X}{:02X}{:02X}{:02X}",255-c.a(),c.b(),c.g(),c.r());
        let s=&self.style;
        let font=if s.font.trim().is_empty(){"Segoe UI"}else{s.font.trim()};
        let font:String=font.chars().filter(|c|!matches!(c,','|'\''|':'|'['|']'|'\\'|'\n'|'\r')).collect();
        let style=format!("FontName={font},FontSize={},PrimaryColour={},OutlineColour={},BackColour={},BorderStyle=1,Outline={},Shadow={},MarginL={},MarginR={},MarginV={},Alignment={},WrapStyle=2",s.size,color(s.color),color(s.outline_color),color(s.shadow_color),if s.outline{s.outline_size}else{0.0},0.0,s.margin_x,s.margin_x,s.margin_y,if s.top{8}else{2});
        format!("setpts=PTS+{position}/TB,subtitles=filename='{path}':force_style='{style}',setpts=PTS-STARTPTS,{base}")
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn original_extraction_requires_a_supported_standalone_codec(){
        assert_eq!(original_track_format("ass"),Some(("ass","ass")));
        assert_eq!(original_track_format("subrip"),Some(("srt","srt")));
        assert_eq!(original_track_format("webvtt"),Some(("vtt","webvtt")));
        assert_eq!(original_track_format("mov_text"),None);
        assert_eq!(original_track_format("hdmv_pgs_subtitle"),None);
    }
    #[test]
    fn multiple_exact_sidecar_formats_require_manual_selection(){
        let mut subtitles=Subtitles{initialized:true,sidecars:vec![Source{path:"film.srt".into(),default:true,..source(None)},Source{path:"film.ass".into(),default:true,..source(None)}],..Default::default()};
        subtitles.follow_audio(None,&[],&egui::Context::default());assert!(subtitles.selected.is_none());assert!(subtitles.manual_required);
    }
    #[test]
    fn pending_subtitle_swap_keeps_active_asset_and_style() {
        let ctx=egui::Context::default();
        let saved=style_preference().lock().unwrap().clone();
        let old=Arc::new(Asset(PathBuf::new()));
        let mut next=saved.clone();next.margin_y=123;
        let (tx,rx)=mpsc::channel();
        let mut subtitles=Subtitles{initialized:true,manual_override:true,style:saved.clone(),
            asset:Some(old.clone()),asset_style:Some(saved.clone()),preview_style:Some(next.clone()),
            job:Some(rx),revision:7,..Default::default()};
        subtitles.poll(&[],None,None,&ctx);
        assert_eq!(subtitles.revision,7);
        let render=subtitles.render().unwrap();
        assert!(Arc::ptr_eq(&render.asset,&old));assert!(render.style==saved);
        let replacement=Arc::new(Asset(PathBuf::new()));
        tx.send(Ok((replacement.clone(),next.clone()))).unwrap();
        subtitles.poll(&[],None,None,&ctx);
        assert_eq!(subtitles.revision,8);
        let render=subtitles.render().unwrap();
        assert!(Arc::ptr_eq(&render.asset,&replacement));assert!(render.style==next);
        let (tx,rx)=mpsc::channel();subtitles.job=Some(rx);
        tx.send(Err("load failed".to_owned())).unwrap();
        subtitles.poll(&[],None,None,&ctx);
        assert_eq!(subtitles.revision,8);
        assert!(Arc::ptr_eq(&subtitles.render().unwrap().asset,&replacement));
        assert_eq!(subtitles.error.as_deref(),Some("load failed"));
    }
    #[test]
    fn long_plain_cues_are_paged_without_losing_text_or_time(){
        let text="朝九成宇宙最强者都是从法则出发的，突破神国瓶颈，演化小型宇宙，法则之路就是从某一基础法的不断攀升。其终极目标就是将十大基础法则全部学会并融为一体。";
        let mut style=Style::default();style.size=24.0;
        let input=format!("PlayResX: 384\nDialogue: 0,0:00:01.00,0:00:16.00,Default,,0,0,0,,{text}\n");
        let output=apply_shadow_offsets(&input,&style);
        let lines:Vec<_>=output.lines().filter(|line|line.starts_with("Dialogue:")).collect();
        assert!(lines.len()>1);
        let tags=regex::Regex::new(r"\{[^}]*\}").unwrap();
        let mut joined=String::new();let mut previous=100;
        for line in lines {
            let fields:Vec<_>=line.splitn(10,',').collect();
            assert_eq!(ass_centiseconds(fields[1]),Some(previous));
            previous=ass_centiseconds(fields[2]).unwrap();
            assert!(fields[9].split("\\N").count()<=2);
            joined.push_str(&tags.replace_all(fields[9],"").replace("\\N",""));
        }
        assert_eq!(previous,1600);assert_eq!(joined,text);
        let mut native=style.clone();native.native_ass=true;
        assert_eq!(apply_shadow_offsets(&input,&native),input);
        let bilingual=input.replace(text,&format!("{text}\\NEnglish translation"));
        assert_eq!(apply_shadow_offsets(&bilingual,&style).matches("Dialogue:").count(),1);
    }
    #[test]
    fn margins_use_ass_columns_five_six_seven_and_preserve_effect(){
        let mut style=Style::default();style.margin_x=10;style.margin_y=100;
        let ass="Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,Banner,test\n";
        let output=apply_shadow_offsets(ass,&style);
        let fields:Vec<_>=output.trim().splitn(10,',').collect();
        assert_eq!(fields.len(),10);
        assert_eq!((fields[5],fields[6],fields[7],fields[8]),("10","10","100","Banner"));
        assert!(fields[9].ends_with("test"));
    }
    #[test]
    fn preview_is_debounced_and_reverting_does_not_change_saved_style(){
        let mut subtitles=Subtitles::default();let ctx=egui::Context::default();
        let saved=subtitles.style.clone();subtitles.draft=saved.clone();subtitles.draft.margin_y=100;
        let start=std::time::Instant::now();
        assert!(!subtitles.queue_preview(start));
        assert!(!subtitles.queue_preview(start+std::time::Duration::from_millis(100)));
        assert!(subtitles.queue_preview(start+std::time::Duration::from_millis(200)));
        assert_eq!(subtitles.render_style().margin_y,100);assert!(subtitles.style==saved);
        subtitles.revert_preview(&ctx);assert!(subtitles.render_style()==&saved);
        assert!(subtitles.preview_pending.is_none());
    }
    #[test]
    fn uniform_canvas_matches_video_aspect_without_changing_native_ass(){
        let ass="[Script Info]\nPlayResX: 384\nPlayResY: 288\n[Events]\nDialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,test\n";
        let mut style=Style::default();
        assert!(prepare_ass(ass,&style,Some([1920,1080])).contains("PlayResX: 512\n"));
        assert!(prepare_ass(ass,&style,Some([1440,1080])).contains("PlayResX: 384\n"));
        assert!(prepare_ass(ass,&style,Some([1080,1920])).contains("PlayResX: 162\n"));
        style.native_ass=true;assert_eq!(prepare_ass(ass,&style,Some([1920,1080])),ass);
    }
    #[test]
    fn native_mode_preserves_effects_and_does_not_force_player_style(){
        let mut style=Style::default();style.native_ass=true;
        let ass="[Script Info]\nPlayResX: 1280\n[Events]\nDialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\\move(0,10,100,20)\\fad(100,200)\\k30\\fs60}test\n";
        assert_eq!(apply_shadow_offsets(ass,&style),ass);assert_eq!(bake_ass_style(ass,&style),ass);
        let render=Render{asset:Arc::new(Asset(PathBuf::new())),style};
        let filter=render.filter(2.0,"scale=1280:720");
        assert!(!filter.contains("force_style"));assert!(filter.contains("setpts=PTS+2/TB"));
        assert!(!decode_style(b"{}").unwrap().native_ass);
    }
    #[test]
    fn exported_uniform_ass_contains_the_selected_style(){
        let mut style=Style::default();style.size=32.0;style.top=true;style.margin_y=40;
        let ass="[V4+ Styles]\nStyle: Default,Arial,16,white,white,black,black,0,0,0,0,100,100,0,0,1,0,0,2,10,10,10,1\n";
        let exported=bake_ass_style(ass,&style);
        assert!(exported.contains("Style: Default,Segoe UI,32,"));assert!(exported.contains(",8,20,20,40,1"));
    }
    #[test]
    fn reopening_style_preserves_pending_edits_and_requests_existing_window_focus(){
        let ctx=egui::Context::default();let id=egui::Id::new("subtitle-test");
        let mut subtitles=Subtitles::default();subtitles.settings_id=Some(id);
        subtitles.open_style(&ctx);subtitles.draft.size=53.0;
        subtitles.open_style(&ctx);
        assert_eq!(subtitles.draft.size,53.0);assert!(subtitles.settings_open);
        assert_eq!(ctx.data_mut(|data|data.remove_temp::<bool>(id.with("native_dialog_wake"))),Some(true));
    }
    #[test]
    fn top_position_and_new_vertical_margin_reach_dialogue_tags(){
        let mut style=Style::default();style.top=true;style.margin_y=40;
        let ass="[Script Info]\nPlayResX: 384\nPlayResY: 288\n[Events]\nDialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,test\n";
        let output=apply_shadow_offsets(ass,&style);
        assert!(output.contains(",20,20,40,,"));assert!(output.contains("\\an8"));
        assert!(!decode_style(b"{}").unwrap().top);
        style.margin_y=20;assert!(apply_shadow_offsets(ass,&style).contains(",20,20,20,,"));
    }
    #[test]
    fn dialogue_cannot_bypass_player_margins_or_wrapping(){
        let mut style=Style::default();style.margin_x=30;style.margin_y=18;
        let source="Dialogue: 0,0:00:00.00,0:00:02.00,Default,,1,999,7,,{\\q2\\pos(0,40)\\an7\\b1}long text, intact\n";
        let normalized=apply_shadow_offsets(source,&style);
        assert!(normalized.contains(",30,30,18,,"));
        assert!(!normalized.contains("{\\q2\\pos"));assert!(!normalized.contains("\\pos("));
        assert!(!normalized.contains("\\an7"));assert!(normalized.contains("\\b1"));
        assert!(normalized.contains("long text, intact"));assert!(normalized.contains("\\q2\\an2"));
    }
    #[test]
    fn legacy_shadow_and_signed_axes_survive_loading(){
        let legacy=decode_style(br#"{"shadow":true,"shadow_size":4.5}"#).unwrap();
        assert_eq!((legacy.shadow_x,legacy.shadow_y),(4.5,4.5));
        let axes=decode_style(br#"{"shadow":true,"shadow_size":4.5,"shadow_x":-3,"shadow_y":0}"#).unwrap();
        assert_eq!((axes.shadow_x,axes.shadow_y),(-3.0,0.0));
        assert!(!serde_json::to_string(&axes).unwrap().contains("shadow_size"));
    }
    #[test]
    fn shadow_tags_override_existing_tags_without_losing_text(){
        let mut style=Style::default();style.shadow=true;style.shadow_x=-3.0;style.shadow_y=5.0;
        let ass="[Events]\nDialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\\shad8}hello, world\n";
        let output=apply_shadow_offsets(ass,&style);
        assert!(output.contains("{\\q2\\an2\\xshad-3\\yshad5}{\\shad8}{\\q2\\an2\\xshad-3\\yshad5}hello, world"));
        style.shadow=false;
        assert!(apply_shadow_offsets(ass,&style).contains("{\\q2\\an2\\xshad0\\yshad0}"));
    }
    fn source(language:Option<&'static str>)->Source{Source{path:PathBuf::from("source.mkv"),ordinal:0,codec:String::new(),label:String::new(),default:false,text:true,language}}
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
pub struct Subtitles {pub revision:u64,video_size:Option<[usize;2]>,asset:Option<Arc<Asset>>,asset_style:Option<Style>,selected:Option<Source>,external:Vec<Source>,sidecars:Vec<Source>,next_scan:Option<std::time::Instant>,job:Option<mpsc::Receiver<Result<(Arc<Asset>,Style),String>>>,initialized:bool,style:Style,preview_style:Option<Style>,preview_pending:Option<(Style,std::time::Instant)>,draft:Style,style_error:Option<String>,settings_open:bool,settings_id:Option<egui::Id>,export_job:Option<mpsc::Receiver<Result<PathBuf,String>>>,export_result:Option<PathBuf>,editor_request:Option<PathBuf>,saved_revision:u64,extracting:bool,font_search:String,error:Option<String>,manual_required:bool,manual_override:bool,needs_follow:bool}
impl Subtitles {
    pub(crate) fn take_editor_request(&mut self)->Option<PathBuf>{self.editor_request.take()}
    pub(crate) fn editor_render(&mut self,render:Render){
        self.asset=Some(render.asset);self.asset_style=Some(render.style);self.job=None;
        self.initialized=true;self.manual_override=true;self.revision+=1;
    }
    fn local_editable(&self)->Option<PathBuf>{
        let source=self.selected.as_ref()?;
        if crate::subtitle_document::Format::from_path(&source.path).is_err(){
            if !self.external.iter().chain(&self.sidecars).any(|s|s.path==source.path)||crate::subtitle_document::Format::from_path(Path::new(&format!("source.{}",source.codec))).is_err(){return None;}
        }
        Some(source.path.clone())
    }
    pub(crate) fn video_size(&mut self,size:Option<[usize;2]>,ctx:&egui::Context){
        if self.video_size!=size {self.video_size=size;if let Some(source)=self.selected.clone(){self.load(source,ctx);}}
    }
    pub fn session_copy(&self)->Self{Self{revision:1,video_size:self.video_size,asset:self.asset.clone(),asset_style:self.asset_style.clone(),selected:self.selected.clone(),external:self.external.clone(),sidecars:self.sidecars.clone(),initialized:true,style:self.style.clone(),manual_override:self.manual_override,manual_required:self.manual_required,..Default::default()}}
    fn render_style(&self)->&Style{self.preview_style.as_ref().unwrap_or(&self.style)}
    pub fn render(&self)->Option<Render>{Some(Render{asset:self.asset.clone()?,style:self.asset_style.as_ref().unwrap_or_else(||self.render_style()).clone()})}
    pub(crate) fn translation_source(&self)->Option<Source>{
        self.selected.as_ref().filter(|source|source.text&&!crate::subtitle_translation::is_translation(&source.path)).cloned()
    }
    pub fn follow_audio(&mut self,language:Option<&str>,sources:&[Source],ctx:&egui::Context){
        if self.manual_override{return;}
        let matches:Vec<_>=sources.iter().chain(self.external.iter()).filter(|s|s.text&&s.language.is_some()&&s.language!=Some("未标注语言")&&s.language.as_deref()==language&&language.is_some()).cloned().collect();
        let chosen=if matches.len()==1{matches.first()}else{
            let defaults:Vec<_>=matches.iter().filter(|s|s.default).collect();if defaults.len()==1{defaults.first().copied()}else{None}
        };
        let originals=self.sidecars.iter().filter(|source|!crate::subtitle_translation::is_translation(&source.path));
        let sidecar={let mut defaults=originals.clone().filter(|source|source.default);let first=defaults.next();if defaults.next().is_none(){first}else{None}}
            .or_else(|| {
                let mut candidates=originals.clone();
                let first=candidates.next()?;
                candidates.next().is_none().then_some(first)
            })
            .or_else(|| {
                let mut matches=originals.filter(|source|language.is_some_and(|language|
                    source.language==Some(language) ||
                    crate::auto_subtitle::language_code(Some(language)).is_some_and(|code|
                        crate::auto_subtitle::language_code(source.language)==Some(code))));
                let first=matches.next()?;
                matches.next().is_none().then_some(first)
            });
        if let Some(source)=sidecar.or(chosen){
            self.manual_required=false;
            if !self.selected.as_ref().is_some_and(|s|s.path==source.path&&s.ordinal==source.ordinal){self.load(source.clone(),ctx);}
        }else{
            self.manual_required=!sources.is_empty()||!self.external.is_empty()||!self.sidecars.is_empty();
            if self.selected.take().is_some(){self.asset=None;self.job=None;self.revision+=1;}
        }
    }
    pub fn header(&mut self,ui:&mut egui::Ui,sources:&[Source]){
        if self.selected.is_some() && self.preview_style.is_some(){ui.small(tr("预览（尚未保存）"));}
        if !self.sidecars.is_empty() || !self.external.is_empty() || !sources.is_empty() {
            let current=self.selected.as_ref().map_or_else(||tr("关闭字幕"),|source|source.label.as_str()).to_owned();
            let width=ui.available_width().min(180.0);
            ui.scope(|ui|{
                // ComboBox::width is a minimum; constrain its layout as well.
                ui.set_max_width(width);
                egui::ComboBox::from_id_salt("video_header_subtitle")
                    .width(width)
                    .truncate()
                    .selected_text(&current)
                    .show_ui(ui,|ui|self.menu_contents(ui,sources))
                    .response.on_hover_text(current);
            });
            if let Some(path)=self.local_editable(){if ui.button(tr("字幕编辑")).clicked(){self.editor_request=Some(path);}}
        }
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
    }
    pub fn status(&self,ui:&mut egui::Ui){
        if self.manual_required {
            ui.colored_label(ui.visuals().warn_fg_color,tr("未能匹配音轨语言字幕，请右键打开字幕菜单手动选择或加载外挂字幕。"));
        }
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
    }
    pub fn poll(&mut self,sources:&[Source],language:Option<&str>,local_video:Option<&Path>,ctx:&egui::Context){
        self.poll_export(ctx);
        if let Some(source)=self.selected.clone(){
            let revision=crate::subtitle_document::saved_revision(&source.path);
            if revision!=self.saved_revision{self.saved_revision=revision;self.load(source,ctx);}
        }
        let preference=style_preference().lock().unwrap().clone();
        if self.style!=preference {
            self.style=preference;
            if let Some(source)=self.selected.clone(){self.load(source,ctx);}else{self.revision+=1;}
        }
        let mut changed=false;
        if let Some(path)=local_video {
            let now=std::time::Instant::now();
            let interval=std::time::Duration::from_secs(2);
            if self.next_scan.is_none_or(|next|now>=next){
                self.next_scan=Some(now+interval);
                changed=self.discover_sidecars(path);
            }
            ctx.request_repaint_after(interval);
        }
        if !self.initialized || self.needs_follow || changed {
            self.initialized=true;
            self.needs_follow=false;
            if changed && self.selected.as_ref().is_some_and(|source|!source.path.is_file()) {
                self.manual_override=false;
            }
            self.follow_audio(language,sources,ctx);
        }
        if let Some(job)=&self.job{match job.try_recv(){
            Ok(result)=>{self.job=None;match result{Ok((asset,style))=>{self.asset=Some(asset);self.asset_style=Some(style);self.revision+=1;},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.job=None;self.error=Some(tr("字幕加载失败").to_owned());},
            Err(mpsc::TryRecvError::Empty)=>ctx.request_repaint_after(std::time::Duration::from_millis(100))
        }}
    }
    fn discover_sidecars(&mut self,path:&Path)->bool{
        let Some(stem)=path.file_stem().and_then(|stem|stem.to_str()) else{return false;};
        let directory=path.parent().filter(|parent|!parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let mut sidecars:Vec<_>=std::fs::read_dir(directory).into_iter().flatten().filter_map(Result::ok)
            .filter_map(|entry|{
                let label=entry.file_name().into_string().ok()?;
                let (name,extension)=label.rsplit_once('.')?;
                if !matches!(extension.to_ascii_lowercase().as_str(),"ass"|"ssa"|"srt"|"vtt"|"webvtt") || !name.get(..stem.len())?.eq_ignore_ascii_case(stem) {
                    return None;
                }
                let suffix=&name[stem.len()..];
                let default=suffix.is_empty();
                let language=if default {None} else {
                    let suffix=suffix.strip_prefix('.').filter(|suffix|!suffix.is_empty())?;
                    let tag=suffix.rsplit('.').next()?;
                    crate::media_preview::audio_language_name(tag,"und").filter(|name|*name!="未标注语言")
                };
                let path=path.with_file_name(&label);
                path.is_file().then_some(Source{path,ordinal:0,codec:extension.to_ascii_lowercase(),label,default,text:true,language})
            }).collect();
        sidecars.sort_by(|a,b|b.default.cmp(&a.default).then_with(||a.label.cmp(&b.label)));
        if sidecars==self.sidecars{return false;}
        self.external.retain(|source|!sidecars.iter().any(|sidecar|sidecar.path==source.path));
        self.sidecars=sidecars;
        true
    }
    fn source_choice(&mut self,ui:&mut egui::Ui,source:Source){
        let selected=self.selected.as_ref().is_some_and(|s|s.path==source.path&&s.ordinal==source.ordinal);
        if ui.add_enabled(source.text,egui::SelectableLabel::new(selected,&source.label)).on_hover_text(&source.label).clicked(){
            self.manual_override=true;self.manual_required=false;self.load(source,ui.ctx());ui.close_menu();
        }
    }
    pub(crate) fn generated(&mut self,video:&Path,path:PathBuf,language:Option<&str>,current_track:bool,ctx:&egui::Context){
        self.discover_sidecars(video);
        if !current_track {return;}
        if let Some(source)=self.sidecars.iter().find(|source|source.path==path).cloned(){
            self.manual_override=crate::auto_subtitle::language_code(language)!=crate::auto_subtitle::language_code(source.language);
            self.manual_required=false;
            self.load(source,ctx);
        }
    }
    fn load(&mut self,source:Source,ctx:&egui::Context){
        self.saved_revision=crate::subtitle_document::saved_revision(&source.path);
        // Keep the active asset until its replacement is ready; loading must not restart playback.
        self.selected=Some(source.clone());self.error=None;
        let (tx,rx)=mpsc::channel();self.job=Some(rx);let ctx=ctx.clone();let style=self.render_style().clone();let video_size=self.video_size;
        std::thread::spawn(move||{
            let result=(||{
                let ffmpeg=crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
                let filters=crate::media_tools::command(&ffmpeg).args(["-hide_banner","-filters"]).output().map_err(|e|e.to_string())?;
                if !String::from_utf8_lossy(&filters.stdout).lines().any(|line|line.split_whitespace().nth(1)==Some("subtitles")){return Err("FFmpeg subtitles / libass filter unavailable".to_owned());}
                static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
                let path=std::env::temp_dir().join(format!("unity-subtitle-{}-{}.ass",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
                let file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;drop(file);
                let asset=Arc::new(Asset(path));
                if style.native_ass && source.path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case("ass")||ext.eq_ignore_ascii_case("ssa")) {
                    std::fs::copy(&source.path,&asset.0).map_err(|e|e.to_string())?;
                }else{
                    let output=crate::media_tools::command(&ffmpeg).args(["-nostdin","-v","error","-y","-i"]).arg(&source.path).args(["-map",&format!("0:s:{}",source.ordinal),"-c:s","ass","-f","ass"]).arg(&asset.0).output().map_err(|e|e.to_string())?;
                    if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).to_string());}
                }
                let text=std::fs::read_to_string(&asset.0).map_err(|e|e.to_string())?;
                std::fs::write(&asset.0,prepare_ass(&text,&style,video_size)).map_err(|e|e.to_string())?;
                Ok((asset,style))
            })();let _=tx.send(result);ctx.request_repaint();
        });
    }
    pub(crate) fn load_external(&mut self,path:PathBuf,ctx:&egui::Context){
        let stem=path.file_stem().unwrap_or_default().to_string_lossy();
        let language=stem.split(['.','_','-',' ']).rev().find_map(|part|crate::media_preview::audio_language_name(part,"und").filter(|name|*name!="未标注语言"));
        let codec=crate::subtitle_document::read(&path).ok().map_or_else(||path.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase(),|(doc,_)|doc.format.extension().into());
        let source=Source{label:path.file_name().unwrap_or_default().to_string_lossy().to_string(),codec,path,ordinal:0,default:false,text:true,language};
        if !self.external.iter().chain(&self.sidecars).any(|s|s.path==source.path){self.external.push(source.clone());}
        self.manual_override=true;self.manual_required=false;self.load(source,ctx);
    }
    pub fn menu(&mut self,ui:&mut egui::Ui,sources:&[Source]){
        ui.menu_button(tr("字幕"),|ui|self.menu_contents(ui,sources));
    }
    fn menu_contents(&mut self,ui:&mut egui::Ui,sources:&[Source]){
        self.poll_export(ui.ctx());
        ui.set_width((ui.ctx().screen_rect().width()-32.0).clamp(0.0,280.0));
        ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Truncate);
        if ui.checkbox(&mut self.manual_override,tr("手动选择字幕（关闭语言跟随）")).changed(){self.manual_required=false;self.needs_follow=!self.manual_override;}
        if ui.selectable_label(self.selected.is_none(),tr("关闭字幕")).clicked(){self.manual_override=true;self.manual_required=false;self.selected=None;self.asset=None;self.job=None;self.revision+=1;ui.close_menu();}
        if ui.button(tr("字幕样式")).clicked(){self.open_style(ui.ctx());ui.close_menu();}
        if let Some(path)=self.local_editable(){if ui.button(tr("字幕编辑")).clicked(){self.editor_request=Some(path);ui.close_menu();}}
        if let Some(source)=self.selected.clone().filter(|source|source.text&&sources.contains(source)){
            if ui.add_enabled(self.export_job.is_none(),egui::Button::new(tr("提取字幕（ASS）"))).clicked(){
                let name=format!("{}.track{}.ass",source.path.file_stem().unwrap_or_default().to_string_lossy(),source.ordinal+1);
                if let Some(path)=rfd::FileDialog::new().add_filter("ASS",&["ass"]).set_directory(source.path.parent().unwrap_or(Path::new("."))).set_file_name(name).save_file(){
                    self.extracting=true;self.error=None;self.export_result=None;
                    let source=source.clone();
                    crate::auto_subtitle::run(&mut self.export_job,ui.ctx(),move||extract_original_ass(&source,&path));
                }ui.close_menu();
            }
            let native=original_track_format(&source.codec);
            if ui.add_enabled(self.export_job.is_none()&&native.is_some(),egui::Button::new(tr("按原格式提取字幕"))).on_hover_text(tr("按原字幕编码复制；当前不支持无独立文字格式的字幕轨。字体附件不随字幕提取。" )).clicked(){
                if let Some((extension,_))=native{
                    let name=format!("{}.track{}.{}",source.path.file_stem().unwrap_or_default().to_string_lossy(),source.ordinal+1,extension);
                    if let Some(path)=rfd::FileDialog::new().add_filter(extension,&[extension]).set_directory(source.path.parent().unwrap_or(Path::new("."))).set_file_name(name).save_file(){
                        self.extracting=true;self.error=None;self.export_result=None;
                        crate::auto_subtitle::run(&mut self.export_job,ui.ctx(),move||extract_original_track(&source,&path));
                    }ui.close_menu();
                }
            }
        }
        if let (Some(asset),Some(source))=(self.asset.clone(),self.selected.clone()) {
            ui.menu_button(tr("导出当前字幕"),|ui|{
                ui.small(tr("SRT 保存文字与时间；ASS 可保留当前模式的样式和特效。"));
                for (extension,label) in [("srt","SRT"),("ass","ASS")] {
                    if ui.add_enabled(self.export_job.is_none()&&self.job.is_none(),egui::Button::new(label)).clicked(){
                        let name=format!("{}.export.{extension}",source.path.file_stem().unwrap_or_default().to_string_lossy());
                        if let Some(destination)=rfd::FileDialog::new().add_filter(label,&[extension]).set_file_name(name).save_file(){
                            self.error=None;self.export_result=None;let asset=asset.clone();let style=self.asset_style.as_ref().unwrap_or_else(||self.render_style()).clone();
                            crate::auto_subtitle::run(&mut self.export_job,ui.ctx(),move||{
                                let text=if extension=="ass" {
                                    let text=std::fs::read_to_string(&asset.0).map_err(|e|e.to_string())?;
                                    bake_ass_style(&text,&style)
                                }else{
                                    let ffmpeg=crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
                                    let output=crate::media_tools::command(ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&asset.0).args(["-map","0:s:0","-c:s","srt","-f","srt","pipe:1"]).output().map_err(|e|e.to_string())?;
                                    if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).to_string());}
                                    String::from_utf8(output.stdout).map_err(|e|e.to_string())?
                                };
                                crate::auto_subtitle::save_new(destination,&text)
                            });
                        }
                        ui.close_menu();
                    }
                }
            });
        }
        if self.export_job.is_some(){ui.spinner();ui.label(tr("正在导出字幕…"));}
        if let Some(path)=self.export_result.clone(){if ui.link(tr("字幕已保存")).clicked(){if let Err(error)=crate::asset_viewer::reveal_file(&path){self.error=Some(error.to_string());}}}

        let choices:Vec<_>=sources.iter().chain(self.sidecars.iter()).chain(self.external.iter()).cloned().collect();
        for source in choices{
            self.source_choice(ui,source);
        }
        if sources.iter().any(|s|!s.text){ui.small(tr("图像字幕暂不支持播放器显示；可加载文字外挂字幕。"));}
        if ui.button(tr("加载外挂字幕")).clicked(){
            if let Some(path)=rfd::FileDialog::new().add_filter(tr("字幕"),&["srt","ass","ssa","vtt","webvtt"]).add_filter("All files",&["*"]).pick_file(){
                self.load_external(path,ui.ctx());
            }ui.close_menu();
        }
        if self.job.is_some(){ui.spinner();}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
    }
    fn poll_export(&mut self,ctx:&egui::Context){
        if let Some(result)=crate::auto_subtitle::poll(&mut self.export_job){let extracted=std::mem::take(&mut self.extracting);match result{Ok(path)=>{if extracted{self.load_external(path.clone(),ctx);self.editor_request=Some(path.clone());}self.export_result=Some(path);},Err(error)=>self.error=Some(error)}}
        if self.export_job.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(100));}
    }
    fn open_style(&mut self,ctx:&egui::Context){
        if !self.settings_open {self.draft=self.style.clone();self.preview_style=None;self.preview_pending=None;self.style_error=None;self.font_search.clear();}
        self.settings_open=true;
        if let Some(id)=self.settings_id {crate::native_window::wake(ctx,id);}
        ctx.request_repaint();
    }
    pub fn style_window(&mut self,ctx:&egui::Context,path:&Path){
        let id=egui::Id::new(("subtitle_style",path));self.settings_id=Some(id);
        if !self.settings_open{return;}
        let mut open=true;let mut apply=false;let mut cancel=false;
        crate::native_window::Window::new(tr("字幕样式")).id(id).open(&mut open).default_width(320.0).fit_height().show(ctx,|ui|{
            ui.small(tr("修改后预览；应用保存，取消或关闭恢复原样。"));
            let s=&mut self.draft;
            ui.horizontal(|ui|{ui.selectable_value(&mut s.native_ass,false,tr("统一样式"));ui.selectable_value(&mut s.native_ass,true,tr("原生 ASS"));});
            if s.native_ass {ui.small(tr("保留字幕自身的字体、定位、换行和特效；统一样式参数不参与渲染。"));}
            ui.add_enabled_ui(!s.native_ass,|ui|{
            uniform_style_controls(ui,s,&mut self.font_search);
            });
            if let Some(error)=&self.style_error{ui.colored_label(ui.visuals().error_fg_color,error);}
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
        if apply {
            match save_style(&self.draft) {
                Ok(())=>{self.style=self.draft.clone();self.preview_style=None;self.preview_pending=None;if let Some(source)=self.selected.clone(){self.load(source,ctx);}else{self.revision+=1;}open=false;}
                Err(error)=>self.style_error=Some(error),
            }
        }
        if cancel{open=false;}
        self.settings_open=open;
        if open {
            let now=std::time::Instant::now();
            if self.queue_preview(now) {
                if let Some(source)=self.selected.clone(){self.load(source,ctx);}else{self.revision+=1;}
            }
            if self.preview_pending.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(50));}
        }else{self.revert_preview(ctx);}
    }
    fn queue_preview(&mut self,now:std::time::Instant)->bool{
        if self.render_style()==&self.draft {self.preview_pending=None;return false;}
        if self.preview_pending.as_ref().is_none_or(|(style,_)|style!=&self.draft){self.preview_pending=Some((self.draft.clone(),now));return false;}
        if self.preview_pending.as_ref().is_some_and(|(_,start)|now.saturating_duration_since(*start)>=std::time::Duration::from_millis(180)){
            self.preview_style=Some(self.draft.clone());self.preview_pending=None;return true;
        }
        false
    }
    fn revert_preview(&mut self,ctx:&egui::Context){
        self.preview_pending=None;
        if self.preview_style.take().is_some(){if let Some(source)=self.selected.clone(){self.load(source,ctx);}else{self.revision+=1;}}
    }
}

fn original_track_format(codec:&str)->Option<(&'static str,&'static str)>{match codec{
    "ass"|"ssa"=>Some(("ass","ass")),"subrip"|"srt"=>Some(("srt","srt")),"webvtt"=>Some(("vtt","webvtt")),_=>None
}}
fn extract_original_track(source:&Source,destination:&Path)->Result<PathBuf,String>{
    let (extension,muxer)=original_track_format(&source.codec).ok_or("No supported standalone original subtitle format")?;
    let ffmpeg=crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
    let output=crate::media_tools::command(&ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&source.path)
        .args(["-map",&format!("0:s:{}",source.ordinal),"-c:s","copy","-f",muxer,"pipe:1"]).output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).into());}
    let text=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
    let target=destination.with_extension(extension);
    let format=crate::subtitle_document::Format::detect(&text,&target)?;
    crate::subtitle_document::Document::parse(&text,format)?;
    crate::auto_subtitle::save_new(target,&text)
}

fn extract_original_ass(source:&Source,destination:&Path)->Result<PathBuf,String>{
    let ffprobe=crate::media_tools::find_tool("ffprobe").ok_or("FFprobe unavailable")?;
    let probe=crate::media_tools::command(&ffprobe).args(["-v","error","-select_streams",&format!("s:{}",source.ordinal),"-show_entries","stream=codec_name","-of","default=nw=1:nk=1"]).arg(&source.path).output().map_err(|e|e.to_string())?;
    if !probe.status.success(){return Err(String::from_utf8_lossy(&probe.stderr).into());}
    let native=String::from_utf8_lossy(&probe.stdout).lines().any(|s|matches!(s.trim(),"ass"|"ssa"));
    let ffmpeg=crate::media_tools::find_tool("ffmpeg").ok_or("FFmpeg unavailable")?;
    let output=crate::media_tools::command(&ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&source.path)
        .args(["-map",&format!("0:s:{}",source.ordinal),"-c:s",if native{"copy"}else{"ass"},"-f","ass","pipe:1"]).output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).into());}
    let text=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
    crate::subtitle_document::Document::parse(&text,crate::subtitle_document::Format::Ass)?;
    crate::auto_subtitle::save_new(destination.with_extension("ass"),&text)
}

fn uniform_style_controls(ui:&mut egui::Ui,s:&mut Style,font_search:&mut String){
            ui.label(tr("字体"));
            let fonts=system_font_families();
            let selected=if s.font.is_empty(){tr("系统默认字体")}else{s.font.as_str()};
            egui::ComboBox::from_id_salt("subtitle_system_font").width(ui.available_width()).height(220.0)
                .selected_text(selected).show_ui(ui,|ui| {
                    ui.add(egui::TextEdit::singleline(font_search).hint_text(tr("搜索字体")));
                    ui.selectable_value(&mut s.font,String::new(),tr("系统默认字体"));
                    let search=font_search.to_lowercase();
                    for family in fonts.iter().filter(|name|name.to_lowercase().contains(&search)) {
                        ui.selectable_value(&mut s.font,family.clone(),family);
                    }
                    if fonts.is_empty() {ui.label(tr("未能获取系统字体列表"));}
                });
            ui.horizontal(|ui|{ui.label(tr("字号"));ui.add(egui::DragValue::new(&mut s.size).range(8.0..=120.0));ui.color_edit_button_srgba(&mut s.color);});
            ui.horizontal(|ui|{ui.checkbox(&mut s.outline,tr("描边"));ui.add_enabled_ui(s.outline,|ui|{ui.add(egui::DragValue::new(&mut s.outline_size).range(0.0..=12.0));ui.color_edit_button_srgba(&mut s.outline_color);});});
            ui.horizontal(|ui|{ui.checkbox(&mut s.shadow,tr("投影"));ui.add_enabled_ui(s.shadow,|ui|{ui.label("X");ui.add(egui::DragValue::new(&mut s.shadow_x).range(-12.0..=12.0)).on_hover_text(tr("水平投影偏移"));ui.label("Y");ui.add(egui::DragValue::new(&mut s.shadow_y).range(-12.0..=12.0)).on_hover_text(tr("垂直投影偏移"));ui.color_edit_button_srgba(&mut s.shadow_color);});});
            ui.horizontal(|ui|{ui.label(tr("水平边距（脚本单位）"));ui.add(egui::DragValue::new(&mut s.margin_x).range(0..=300));});
            ui.small(tr("边距不是百分比或屏幕像素，会随视频缩放；20 表示 20 个字幕脚本单位。"));
            ui.horizontal(|ui|{ui.label(tr("字幕位置"));ui.selectable_value(&mut s.top,false,tr("底部"));ui.selectable_value(&mut s.top,true,tr("顶部"));});
            ui.horizontal(|ui|{ui.label(tr(if s.top{"顶部边距（脚本单位）"}else{"底部边距（脚本单位）"}));ui.add(egui::DragValue::new(&mut s.margin_y).range(0..=300));});
}
