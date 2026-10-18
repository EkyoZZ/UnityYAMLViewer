use std::{path::{Path,PathBuf},sync::mpsc};
use eframe::egui;
use crate::{localization::tr,media_tools::command};

const LANGUAGES:&[(&str,&str)]=&[("und","未标注语言"),("zho","中文"),("cmn","国语"),("yue","粤语"),("eng","英语"),("jpn","日语"),("kor","韩语"),("deu","德语"),("fra","法语"),("spa","西班牙语"),("ita","意大利语"),("por","葡萄牙语"),("rus","俄语"),("ara","阿拉伯语"),("hin","印地语"),("tha","泰语"),("vie","越南语"),("ind","印度尼西亚语"),("msa","马来语"),("mul","多种语言"),("zxx","无语言内容")];

#[derive(Clone)]
pub struct Track {pub path:PathBuf,pub index:u64,pub subtitle:bool,pub codec:String,pub language:String,pub title:String,pub enabled:bool,pub default:bool,pub forced:bool}
impl Track {
    pub fn subtitle_encoder(&self,container:usize)->Option<&'static str>{
        let text=matches!(self.codec.as_str(),"subrip"|"ass"|"ssa"|"webvtt"|"mov_text"|"text");
        match container {
            0|3|7 if text=>Some("mov_text"),1 if text=>Some("webvtt"),2 if text=>Some("srt"),
            2 if matches!(self.codec.as_str(),"dvd_subtitle"|"hdmv_pgs_subtitle")=>Some("copy"),
            6 if self.codec=="dvb_subtitle"=>Some("copy"),_=>None
        }
    }
    pub fn supported(&self,container:usize)->bool {if self.subtitle{self.subtitle_encoder(container).is_some()}else{container!=9}}
}

#[derive(Default)]
pub struct Tracks {pub entries:Vec<Track>,job:Option<mpsc::Receiver<Result<Vec<Track>,String>>>,initialized:bool,source_failed:bool,loading_source:bool,error:Option<String>}
impl Tracks {
    pub fn ready(&self)->bool{self.initialized&&self.job.is_none()&&!self.source_failed}
    pub fn initialize(&mut self,path:&Path,selected:Option<u64>,ctx:&egui::Context){
        if !self.initialized {self.initialized=true;self.start(vec![path.to_owned()],None,selected,ctx);}
        if let Some(job)=&self.job {match job.try_recv(){
            Ok(result)=>{self.job=None;match result{Ok(tracks)=>{self.entries.extend(tracks);if self.loading_source{self.source_failed=false;}},Err(error)=>{self.error=Some(error);if self.loading_source{self.source_failed=true;}}}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.job=None;self.error=Some(tr("轨道读取失败").to_owned());if self.loading_source{self.source_failed=true;}},
            Err(mpsc::TryRecvError::Empty)=>{ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        }}
    }
    fn start(&mut self,paths:Vec<PathBuf>,kind:Option<bool>,_selected:Option<u64>,ctx:&egui::Context){
        let (tx,rx)=mpsc::channel();self.job=Some(rx);self.loading_source=kind.is_none();self.error=None;let ctx=ctx.clone();
        std::thread::spawn(move||{
            let result=(||{
                let probe=crate::media_tools::find_tool("ffprobe").ok_or("ffprobe unavailable")?;
                let mut tracks=Vec::new();
                for path in paths {
                    let output=command(&probe).args(["-v","error","-show_streams","-of","json"]).arg(&path).output().map_err(|e|e.to_string())?;
                    if !output.status.success(){return Err(format!("{}: {}",path.display(),String::from_utf8_lossy(&output.stderr)));}
                    let data:serde_json::Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
                    for stream in data["streams"].as_array().ok_or("No streams")? {
                        let subtitle=match stream["codec_type"].as_str(){Some("audio")=>false,Some("subtitle")=>true,_=>continue};
                        if kind.is_some_and(|kind|kind!=subtitle){continue;}
                        let index=stream["index"].as_u64().ok_or("Invalid stream index")?;
                        let title=stream["tags"]["title"].as_str().unwrap_or("").to_owned();
                        let language=stream["tags"]["language"].as_str().unwrap_or("und");
                        let name=crate::media_preview::audio_language_name(&title,language);
                        let language=LANGUAGES.iter().find(|(_,label)|Some(*label)==name).map_or(language,|(code,_)|*code).to_owned();
                        tracks.push(Track{path:path.clone(),index,subtitle,codec:stream["codec_name"].as_str().unwrap_or("").to_owned(),language,title,enabled:true,default:stream["disposition"]["default"]==1,forced:stream["disposition"]["forced"]==1});
                    }
                }
                if kind.is_some()&&tracks.is_empty(){return Err("No matching audio / subtitle streams".to_owned());}
                Ok(tracks)
            })();let _=tx.send(result);ctx.request_repaint();
        });
    }
    pub fn valid(&self,container:usize)->bool{
        self.ready() && (container==9 || self.entries.iter().filter(|t|t.enabled).all(|t|t.supported(container))) && (container!=5 || self.selected(container).filter(|t|!t.subtitle).count()<=1)
    }
    pub fn selected(&self,container:usize)->impl Iterator<Item=&Track>{self.entries.iter().filter(move|t|t.enabled&&t.supported(container))}
    pub fn controls(&mut self,ui:&mut egui::Ui,container:usize,path:&Path,selected:Option<u64>){
        ui.separator();ui.heading(tr("音轨与字幕"));
        if self.source_failed&&ui.button(tr("重试")).clicked(){self.initialized=false;self.source_failed=false;self.entries.clear();self.initialize(path,selected,ui.ctx());}
        if container==9 {ui.label(tr("此容器不导出音轨或字幕。"));return;}
        let subtitle_allowed=matches!(container,0|1|2|3|6|7);
        ui.add_enabled_ui(self.job.is_none(),|ui|{ui.horizontal_wrapped(|ui|{
            for (subtitle,label) in [(false,"添加音轨"),(true,"添加字幕")] {
                if ui.add_enabled(!subtitle||subtitle_allowed,egui::Button::new(tr(label))).clicked(){
                    let dialog=rfd::FileDialog::new();
                    let dialog=if subtitle{dialog.add_filter(tr("字幕"),&["srt","ass","ssa","vtt","mkv","mp4","mov","ts"])}else{dialog.add_filter(tr("音轨"),&["wav","mp3","aac","m4a","flac","ogg","opus","mp4","mkv","mov","avi","ts"])};
                    if let Some(paths)=dialog.pick_files(){self.start(paths,Some(subtitle),None,ui.ctx());}
                }
            }
        });});
        if !subtitle_allowed{ui.small(tr("此容器不支持当前字幕格式。"));}
        if container==5 {ui.small(tr("此容器最多导出一条音轨。"));}
        if matches!(container,3|4|5){ui.small(tr("部分语言标签可能无法被此容器保留；需要完整语言标记时建议使用 MKV 或 MP4。"));}
        let mut remove=None;
        for (index,track) in self.entries.iter_mut().enumerate(){ui.push_id(index,|ui|{
            let supported=track.supported(container);
            ui.horizontal_wrapped(|ui|{
                ui.checkbox(&mut track.enabled,tr(if track.subtitle{"字幕"}else{"音轨"}));
                ui.label(format!("{} · #{} · {}",track.path.file_name().unwrap_or_default().to_string_lossy(),track.index,track.codec));
                if ui.small_button("×").on_hover_text(tr("从导出列表移除，不修改原文件。")).clicked(){remove=Some(index);}
            });
            ui.add_enabled_ui(supported,|ui|{
                let label=LANGUAGES.iter().find(|(code,_)|*code==track.language).map_or(track.language.as_str(),|(_,label)|tr(label));
                egui::ComboBox::from_id_salt("track_language").selected_text(label).show_ui(ui,|ui|{
                    if !LANGUAGES.iter().any(|(code,_)|*code==track.language){let original=track.language.clone();ui.selectable_value(&mut track.language,original.clone(),original);}
                    for (code,label) in LANGUAGES{ui.selectable_value(&mut track.language,(*code).to_owned(),tr(label));}
                });
            });
            if !supported&&track.enabled{ui.small(tr("当前容器不支持此轨道，请取消选择或切换容器后导出。"));}
        });}
        if let Some(index)=remove{self.entries.remove(index);}
        if self.job.is_some(){ui.spinner();}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
        if container==5&&self.selected(container).filter(|t|!t.subtitle).count()>1{ui.colored_label(ui.visuals().warn_fg_color,tr("请选择不超过容器上限的音轨数量。"));}
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn clips_overlapping_cues_and_rebases_times(){
        let text="1\r\n00:00:00,100 --> 00:00:01,600\r\nfirst\r\n\r\n2\r\n00:00:02,000 --> 00:00:03,000\r\noutside\r\n";
        let clipped=clip_subtitle(text,0.4,1.1);
        assert!(clipped.contains("00:00:00,000 --> 00:00:01,100"));
        assert!(clipped.contains("first"));assert!(!clipped.contains("outside"));
    }
    #[test]
    fn unsupported_selected_subtitles_block_export(){
        let track=Track{path:PathBuf::from("source.mkv"),index:2,subtitle:true,codec:"hdmv_pgs_subtitle".into(),language:"eng".into(),title:String::new(),enabled:true,default:false,forced:false};
        assert_eq!(track.subtitle_encoder(2),Some("copy"));assert_eq!(track.subtitle_encoder(0),None);
        let manager=Tracks{entries:vec![track],initialized:true,..Default::default()};
        assert!(!manager.valid(0));assert!(manager.valid(2));assert!(manager.valid(9));
    }
    #[test]
    fn flv_rejects_multiple_audio_tracks(){
        let track=Track{path:PathBuf::from("source.mkv"),index:1,subtitle:false,codec:"aac".into(),language:"eng".into(),title:String::new(),enabled:true,default:false,forced:false};
        let manager=Tracks{entries:vec![track.clone(),track],initialized:true,..Default::default()};
        assert!(!manager.valid(5));assert!(manager.valid(0));
    }
}

pub struct TemporaryInputs(Vec<PathBuf>);
impl Drop for TemporaryInputs{fn drop(&mut self){for path in &self.0{let _=std::fs::remove_file(path);}}}

fn subtitle_time(text:&str)->Option<f64>{
    let parts:Vec<_>=text.trim().split([':',',','.']).collect();
    if parts.len()!=4{return None;}
    Some(parts[0].parse::<f64>().ok()?*3600.0+parts[1].parse::<f64>().ok()?*60.0+parts[2].parse::<f64>().ok()?+parts[3].parse::<f64>().ok()?/1000.0)
}
fn srt_time(time:f64)->String{
    let ms=(time.max(0.0)*1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}",ms/3600000,ms/60000%60,ms/1000%60,ms%1000)
}
fn clip_subtitle(text:&str,start:f64,duration:f64)->String{
    let normalized=text.replace("\r\n","\n");let mut result=String::new();let mut count=0;
    for block in normalized.trim().split("\n\n") {
        let lines:Vec<_>=block.lines().collect();
        let Some(index)=lines.iter().position(|line|line.contains(" --> "))else{continue};
        let Some((a,b))=lines[index].split_once(" --> ")else{continue};
        let (Some(a),Some(b))=(subtitle_time(a),subtitle_time(b.split_whitespace().next().unwrap_or("")))else{continue};
        let a=(a-start).max(0.0);let b=(b-start).min(duration);
        if b<=a||a>=duration{continue;}
        count+=1;result+=&format!("{count}\n{} --> {}\n{}\n\n",srt_time(a),srt_time(b),lines[index+1..].join("\n"));
    }
    result
}

pub fn configure(cmd:&mut std::process::Command,tracks:&[Track],source:&Path,container:usize,start:f64,duration:f64,ffmpeg:&Path,temp:&Path)->Result<TemporaryInputs,String>{
    if container!=9&&tracks.iter().any(|t|t.enabled&&!t.supported(container)){return Err("Container cannot retain selected tracks".to_owned());}
    let selected:Vec<_>=tracks.iter().filter(|t|t.enabled&&t.supported(container)).collect();
    if container==5&&selected.iter().filter(|t|!t.subtitle).count()>1{return Err("FLV supports one audio track".to_owned());}
    let mut inputs=vec![source.to_owned()];
    let mut temporary=TemporaryInputs(Vec::new());
    let mut subtitle_inputs=std::collections::HashMap::new();
    for (index,track) in selected.iter().enumerate().filter(|(_,t)|t.subtitle&&t.subtitle_encoder(container)!=Some("copy")) {
        let output=command(ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&track.path).args(["-map",&format!("0:{}",track.index),"-c:s","srt","-f","srt","pipe:1"]).output().map_err(|e|e.to_string())?;
        if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).to_string());}
        let text=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
        let text=clip_subtitle(&text,start,duration);
        if text.is_empty(){return Err(format!("{}: {} #{}",tr("所选片段没有此字幕的有效内容，请取消该轨道后导出。"),track.path.display(),track.index));}
        let path=temp.with_extension(format!("subtitle-{index}.srt"));
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
        temporary.0.push(path.clone());
        use std::io::Write;
        file.write_all(text.as_bytes()).map_err(|e|e.to_string())?;
        cmd.arg("-i").arg(&path);inputs.push(path);subtitle_inputs.insert(index,inputs.len()-1);
    }
    for (index,track) in selected.iter().enumerate(){
        if subtitle_inputs.contains_key(&index){continue;}
        if !inputs.contains(&track.path){
            cmd.args(["-ss",&start.to_string(),"-t",&duration.to_string(),"-i"]).arg(&track.path);
            inputs.push(track.path.clone());
        }
    }
    cmd.args(["-map","0:V:0"]);
    let mut audio=0;let mut subtitle=0;
    let default_audio=selected.iter().filter(|t|!t.subtitle).position(|t|t.default).unwrap_or(0);
    let default_subtitle=selected.iter().filter(|t|t.subtitle).position(|t|t.default);
    for (ordinal,track) in selected.into_iter().enumerate() {
        let input=subtitle_inputs.get(&ordinal).copied().or_else(||inputs.iter().position(|p|p==&track.path)).ok_or("Track input missing")?;
        let stream=if subtitle_inputs.contains_key(&ordinal){0}else{track.index};
        cmd.args(["-map",&format!("{input}:{stream}")]);
        let (kind,index)=if track.subtitle{let i=subtitle;subtitle+=1;("s",i)}else{let i=audio;audio+=1;("a",i)};
        cmd.args([&format!("-metadata:s:{kind}:{index}"),&format!("language={}",track.language),&format!("-metadata:s:{kind}:{index}"),&format!("title={}",track.title)]);
        let is_default=if track.subtitle{Some(index)==default_subtitle}else{index==default_audio};
        let disposition=match (is_default,track.forced){(true,true)=>"default+forced",(true,false)=>"default",(false,true)=>"forced",_=>"0"};
        cmd.arg(format!("-disposition:{kind}:{index}")).arg(disposition);
        if track.subtitle{cmd.arg(format!("-c:s:{index}")).arg(track.subtitle_encoder(container).ok_or("Unsupported subtitle")?);}
    }
    if audio==0{cmd.arg("-an");}if subtitle==0{cmd.arg("-sn");}
    Ok(temporary)
}
