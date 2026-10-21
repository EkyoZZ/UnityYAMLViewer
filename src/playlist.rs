//! Portable M3U documents and independent playlist playback sessions.
use std::{collections::HashMap, path::{Path, PathBuf}, time::{Duration, Instant}};
use serde::{Deserialize, Serialize};
use eframe::egui;
use crate::{localization::tr, media_preview::Preview};

const MAX_DOCUMENT: u64 = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
pub fn is_playlist(path: &Path) -> bool {
    matches!(path.extension().and_then(|v| v.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "m3u" | "m3u8")
}
#[derive(Clone, Debug)]
pub struct Entry { pub path: PathBuf, pub title: String }
fn media_path(path:&Path)->PathBuf {
    let path=std::fs::canonicalize(path).unwrap_or_else(|_|path.to_owned());
    #[cfg(windows)] {
        let text=path.to_string_lossy();
        if let Some(unc)=text.strip_prefix(r"\\?\UNC\"){return PathBuf::from(format!(r"\\{unc}"));}
        if let Some(value)=text.strip_prefix(r"\\?\"){return PathBuf::from(value);}
    }
    path
}
fn title(path: &Path) -> String { path.file_stem().unwrap_or_default().to_string_lossy().into_owned() }
fn error(key: &str) -> String { tr(key).into() }

pub fn read(path: &Path) -> Result<Vec<Entry>, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_DOCUMENT { return Err(error("播放列表文件过大")); }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text.to_owned(),
        Err(_) if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u")) => {
            // Legacy Windows M3U: BOM wins, otherwise try the current ANSI code page.
            decode_legacy(&bytes)?
        }
        Err(_) => return Err(error("M3U8 必须使用 UTF-8 编码")),
    };
    let mut entries = Vec::new();
    let mut pending_title = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.starts_with("#EXT-X-") { return Err(error("暂不支持 HLS 流媒体播放列表")); }
        if let Some(value) = line.strip_prefix("#EXTINF:") {
            pending_title = value.split_once(',').map(|(_, title)| title.trim().to_owned());
        } else if !line.is_empty() && !line.starts_with('#') {
            if line.contains("://") && !line.starts_with("file://") { return Err(error("播放列表目前只支持本地媒体路径")); }
            let item = if line.starts_with("file://") {
                reqwest::Url::parse(line).ok().and_then(|url|url.to_file_path().ok()).ok_or_else(||error("播放列表路径包含无效字符"))?
            }else{PathBuf::from(line)};
            let item = if item.is_absolute() { item } else { path.parent().unwrap_or(Path::new(".")).join(item) };
            if entries.len() >= MAX_ENTRIES { return Err(error("播放列表条目过多")); }
            let display = pending_title.take().filter(|s| !s.is_empty()).unwrap_or_else(|| title(&item));
            entries.push(Entry { path:media_path(&item), title: display });
        }
    }
    if entries.is_empty() { return Err(error("播放列表为空")); }
    Ok(entries)
}
#[cfg(windows)]
fn decode_legacy(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xff,0xfe]) || bytes.starts_with(&[0xfe,0xff]) {
        let le=bytes[0]==0xff;
        if bytes.len()%2!=0{return Err(error("无法读取 M3U 编码"));}
        let words:Vec<_>=bytes[2..].chunks_exact(2).map(|v|if le{u16::from_le_bytes([v[0],v[1]])}else{u16::from_be_bytes([v[0],v[1]])}).collect();
        return String::from_utf16(&words).map_err(|_|error("无法读取 M3U 编码"));
    }
    unsafe extern "system" { fn MultiByteToWideChar(cp: u32, flags: u32, bytes: *const u8, count: i32, output: *mut u16, capacity: i32) -> i32; }
    #[link(name="kernel32")] unsafe extern "system" {}
    let count = unsafe { MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0) };
    if count <= 0 { return Err(error("无法读取 M3U 编码")); }
    let mut output = vec![0u16; count as usize];
    if unsafe { MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, output.as_mut_ptr(), count) } != count { return Err(error("无法读取 M3U 编码")); }
    String::from_utf16(&output).map_err(|_| error("无法读取 M3U 编码"))
}
#[cfg(not(windows))]
fn decode_legacy(bytes: &[u8]) -> Result<String, String> { Ok(encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()) }

fn relative(base: &Path, target: &Path) -> PathBuf {
    let base = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_owned());
    let target = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_owned());
    let a: Vec<_> = base.components().collect(); let b: Vec<_> = target.components().collect();
    let common = a.iter().zip(&b).take_while(|(x,y)|x==y).count();
    if common == 0 {
        #[cfg(windows)] {
            let value=target.to_string_lossy();
            if let Some(unc)=value.strip_prefix(r"\\?\UNC\"){return PathBuf::from(format!(r"\\{unc}"));}
            if let Some(value)=value.strip_prefix(r"\\?\"){return PathBuf::from(value);}
        }
        return target;
    }
    let mut result = PathBuf::new();
    for _ in common..a.len() { result.push(".."); }
    for component in &b[common..] { result.push(component.as_os_str()); }
    result
}
pub fn write(path: &Path, entries: &[Entry], overwrite: bool) -> Result<(), String> {
    if entries.is_empty() || entries.len() > MAX_ENTRIES { return Err(error("播放列表为空或条目过多")); }
    let mut text = String::from(if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("m3u")){"\u{feff}#EXTM3U\n"}else{"#EXTM3U\n"});
    for item in entries {
        let name = item.title.replace(['\r', '\n'], " ");
        let mut item_path = relative(path.parent().unwrap_or(Path::new(".")), &item.path);
        if item_path.to_string_lossy().starts_with('#'){item_path=Path::new(".").join(item_path);}
        let value = item_path.to_str().ok_or_else(|| error("无法保存非 Unicode 路径"))?;
        if value.contains(['\r', '\n']) || value.starts_with('#') { return Err(error("播放列表路径包含无效字符")); }
        text.push_str(&format!("#EXTINF:-1,{name}\n{value}\n"));
    }
    if text.len() as u64 > MAX_DOCUMENT { return Err(error("播放列表文件过大")); }
    // Write completely before replacement. A new destination is created exclusively.
    let temporary = path.with_file_name(format!(".playlist-{}-{}.tmp", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes()).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        drop(file);
        if overwrite { replace(&temporary,path)?; }
        else { move_new(&temporary,path)?; }
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}
#[cfg(windows)]
fn move_file(source: &Path, destination: &Path,overwrite:bool) -> Result<(),String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name="kernel32")] unsafe extern "system" { fn MoveFileExW(source:*const u16,destination:*const u16,flags:u32)->i32; }
    let source:Vec<_>=source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination:Vec<_>=destination.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {MoveFileExW(source.as_ptr(),destination.as_ptr(),8|u32::from(overwrite))}==0 {Err(std::io::Error::last_os_error().to_string())}else{Ok(())}
}
#[cfg(windows)]
pub(crate) fn replace(source:&Path,destination:&Path)->Result<(),String>{move_file(source,destination,true)}
#[cfg(windows)]
fn move_new(source:&Path,destination:&Path)->Result<(),String>{move_file(source,destination,false)}
#[cfg(not(windows))]
fn move_new(source:&Path,destination:&Path)->Result<(),String>{std::fs::hard_link(source,destination).map_err(|e|e.to_string())}
#[cfg(not(windows))]
pub(crate) fn replace(source:&Path,destination:&Path)->Result<(),String>{std::fs::rename(source,destination).map_err(|e|e.to_string())}

/// Numeric runs compare by magnitude, so episode 2 precedes episode 10.
fn natural_key(value: &str) -> Vec<(bool,String)> {
    let mut parts: Vec<(bool,String)> = Vec::new();
    for c in value.to_lowercase().chars() {
        let digit=c.is_ascii_digit();
        if parts.last().is_some_and(|p|p.0==digit) {parts.last_mut().unwrap().1.push(c);}else{parts.push((digit,c.to_string()));}
    }
    parts
}
fn natural_cmp(a:&str,b:&str)->std::cmp::Ordering {
    let a=natural_key(a);let b=natural_key(b);
    for (a,b) in a.iter().zip(&b) {
        let order=if a.0 && b.0 {let x=a.1.trim_start_matches('0');let y=b.1.trim_start_matches('0');x.len().cmp(&y.len()).then_with(||x.cmp(y))}else{a.1.cmp(&b.1)};
        if order!=std::cmp::Ordering::Equal{return order;}
    }
    a.len().cmp(&b.len())
}
fn audio(path:&Path)->bool {matches!(path.extension().and_then(|v|v.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),"mp3"|"wav"|"ogg"|"oga"|"flac"|"aac"|"m4a"|"wma"|"opus"|"aiff"|"aif")}
fn candidate_prefix(path:&Path)->String {
    let stem=title(path);
    if let Some((prefix,_))=stem.split_once('_') {return prefix.to_owned()+"_";}
    let regex=regex::Regex::new(r"(?i)^(.*?)(?:[ ._-]*S\d+E\d+|[ ._-]*E(?:P)?\d+|\s*第\s*\d+\s*[集话話]|[ ._-]+\d+)$").unwrap();
    regex.captures(&stem).map_or(String::new(),|c|c[1].trim().to_owned())
}
fn skip_time(seconds:f64)->String {
    if seconds.is_finite(){(seconds.max(0.0).floor() as u64).to_string()}else{"0".into()}
}
fn parse_time(value:&str)->Option<f64> {
    let value=value.trim();
    if value.is_empty()||!value.bytes().all(|c|c.is_ascii_digit()){return None;}
    value.parse::<u32>().ok().map(|v|v as f64)
}
fn migrate_skip(value:&str)->String {
    let Some((minutes,seconds))=value.trim().split_once(':') else{return value.to_owned();};
    let parsed=(||{
        if minutes.is_empty()||seconds.len()!=2||!minutes.bytes().all(|c|c.is_ascii_digit())||!seconds.bytes().all(|c|c.is_ascii_digit()){return None;}
        let minutes=minutes.parse::<u32>().ok()?;let seconds=seconds.parse::<u32>().ok()?;
        if seconds>=60{return None;}minutes.checked_mul(60)?.checked_add(seconds)
    })();
    parsed.map_or_else(||value.to_owned(),|v|v.to_string())
}

#[derive(Clone,Serialize,Deserialize)]
#[serde(default)]
struct State {
    item: PathBuf, index:usize, identity:Option<crate::media_history::Identity>, position:f64,
    mode:u8, numbers:bool, episodes:bool, intro:String, outro:String, width:f32, height_ratio:f32, open:bool, completed:bool,
}
impl Default for State {fn default()->Self {Self {item:PathBuf::new(),index:0,identity:None,position:0.0,mode:0,numbers:true,episodes:false,intro:"0".into(),outro:"0".into(),width:280.0,height_ratio:0.5,open:true,completed:false}}}
fn state_path(path:&Path)->Option<PathBuf> {
    use sha2::{Digest,Sha256};
    let path=std::fs::canonicalize(path).unwrap_or_else(|_|path.to_owned()).to_string_lossy().into_owned();
    let key=if cfg!(windows){path.to_lowercase()}else{path};
    Some(crate::app_cache::root()?.join("playlist_state").join(format!("{:x}.json",Sha256::digest(key.as_bytes()))))
}
fn progress_path(path:&Path)->Option<PathBuf>{let settings=state_path(path)?;Some(crate::app_cache::root()?.join("playlist_positions").join(settings.file_name()?))}
#[derive(Serialize,Deserialize)]
struct Progress {item:PathBuf,index:usize,identity:Option<crate::media_history::Identity>,position:f64,completed:bool}
fn store_state(path:&Path,state:&State)->Result<(),String>{
    let _guard=crate::app_cache::mutation_guard();
    let mut settings=state.clone();settings.item=PathBuf::new();settings.index=0;settings.identity=None;settings.position=0.0;settings.completed=false;
    let progress=Progress{item:state.item.clone(),index:state.index,identity:state.identity.clone(),position:state.position,completed:state.completed};
    for (destination,data) in [(state_path(path),serde_json::to_vec(&settings)),(progress_path(path),serde_json::to_vec(&progress))]{
        let destination=destination.ok_or_else(||"APPDATA is unavailable".to_owned())?;
        std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e|e.to_string())?;
        let temp=destination.with_extension("tmp");std::fs::write(&temp,data.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;replace(&temp,&destination)?;
    }
    trim_progress(path);
    Ok(())
}
fn trim_progress(path:&Path){
    let Some(directory)=progress_path(path).and_then(|p|p.parent().map(Path::to_owned)) else{return;};
    let Ok(meta)=std::fs::symlink_metadata(&directory) else{return;};
    if meta.file_type().is_symlink(){return;}
    #[cfg(windows)] {use std::os::windows::fs::MetadataExt;if meta.file_attributes()&0x400!=0{return;}}
    let Ok(files)=std::fs::read_dir(directory) else{return;};
    let mut files:Vec<_>=files.filter_map(Result::ok).filter_map(|entry|{let metadata=entry.metadata().ok()?;let name=entry.file_name();let name=name.to_str()?;if !metadata.is_file()||entry.file_type().ok()?.is_symlink()||name.len()!=69||!name.ends_with(".json")||!name.as_bytes()[..64].iter().all(|v|v.is_ascii_hexdigit()){return None;}Some((metadata.modified().ok(),metadata.len(),entry.path()))}).collect();
    files.sort_by_key(|v|v.0);let mut bytes:u64=files.iter().map(|v|v.1).sum();let mut count=files.len();
    for(_,size,file)in files {if count<=500&&bytes<=2*1024*1024{break;}if std::fs::remove_file(file).is_ok(){count-=1;bytes=bytes.saturating_sub(size);}}
}
enum StateJob { Save(PathBuf,State,std::sync::mpsc::Sender<Result<(),String>>,Option<egui::Context>), Flush(std::sync::mpsc::Sender<()>) }
fn state_writer()->&'static std::sync::mpsc::Sender<StateJob>{
    static WRITER:std::sync::OnceLock<std::sync::mpsc::Sender<StateJob>>=std::sync::OnceLock::new();
    WRITER.get_or_init(||{let(tx,rx)=std::sync::mpsc::channel();std::thread::spawn(move||{while let Ok(job)=rx.recv(){match job{StateJob::Save(path,state,ack,ctx)=>{let result=store_state(&path,&state);if let Err(e)=&result{crate::media_history::report_error(e);}let _=ack.send(result);if let Some(ctx)=ctx{ctx.request_repaint();}},StateJob::Flush(ack)=>{let _=ack.send(());}}}});tx})
}
pub fn flush_writes(){let(tx,rx)=std::sync::mpsc::channel();if state_writer().send(StateJob::Flush(tx)).is_ok(){let _=rx.recv();}}
fn load_state(path:&Path)->State {
    let mut state:State=state_path(path).filter(|p|std::fs::metadata(p).is_ok_and(|m|m.len()<65536)).and_then(|p|std::fs::read(p).ok()).and_then(|v|serde_json::from_slice(&v).ok()).unwrap_or_default();
    if let Some(progress)=progress_path(path).filter(|p|std::fs::metadata(p).is_ok_and(|m|m.len()<65536)).and_then(|p|std::fs::read(p).ok()).and_then(|v|serde_json::from_slice::<Progress>(&v).ok()) {
        state.item=progress.item;state.index=progress.index;state.identity=progress.identity;state.position=progress.position;state.completed=progress.completed;
    }
    state
}

fn restore_entry(entries:&[Entry],item:&Path,index:usize)->Option<usize>{
    if entries.get(index).is_some_and(|e|e.path==item){Some(index)}else{entries.iter().position(|e|e.path==item)}
}
fn shuffle_order(count:usize,current:usize,first:bool,random:&mut u64)->Vec<usize>{
    let mut order:Vec<_>=(0..count).filter(|i|!first||*i!=current).collect();
    *random=(*random).max(1);
    for i in (1..order.len()).rev(){*random^=*random<<13;*random^=*random>>7;*random^=*random<<17;let j=*random as usize%(i+1);order.swap(i,j);}
    if order.last()==Some(&current)&&order.len()>1{let last=order.len()-1;order.swap(0,last);}
    order
}

pub struct Session {
    entries:Vec<Entry>, state:State, current:usize, preview:Option<Preview>, identity:Option<crate::media_history::Identity>,
    saved:Instant, error:Option<String>, last_prompt:bool, finished:bool, dirty:bool, scroll_current:bool,
    arrow_last_motion:Option<f64>,ctx:Option<egui::Context>,writes:Vec<std::sync::mpsc::Receiver<Result<(),String>>>, document_identity:Option<crate::media_history::Identity>, initial_pending:bool, shuffle:Vec<usize>, shuffle_started:bool, random:u64, edit:bool, drag:Option<usize>,
}
impl Session {
    pub fn open(path:&Path)->Result<Self,String> {
        let entries=read(path)?;let mut state=load_state(path);
        state.intro=migrate_skip(&state.intro);state.outro=migrate_skip(&state.outro);
        state.width=if state.width.is_finite(){state.width.clamp(180.0,700.0)}else{280.0};state.mode=state.mode.min(2);
        let current=restore_entry(&entries,&state.item,state.index).unwrap_or(0);
        let changed=!state.item.as_os_str().is_empty() && entries[current].path!=state.item;
        let identity=crate::media_history::Identity::read(&entries[current].path);
        if identity!=state.identity || changed || !state.position.is_finite(){state.position=0.0;state.completed=false;}
        if changed {state.position=0.0;}
        let preview=identity.as_ref().map(|_|Preview::for_playlist(state.position,!state.completed));
        let error=if identity.is_none(){Some(error("列表中的媒体文件不存在"))}else if changed{Some(error("上次播放条目已移除，已选择第一项"))}else{None};
        Ok(Self {arrow_last_motion:None,ctx:None,writes:Vec::new(),document_identity:crate::media_history::Identity::read(path),entries,state,current,preview,identity,saved:Instant::now(),error,last_prompt:false,finished:false,dirty:false,scroll_current:true,initial_pending:true,shuffle:Vec::new(),shuffle_started:false,random:std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64,edit:false,drag:None})
    }
    pub fn has_video(&self)->bool {self.preview.as_ref().filter(|p|p.playlist_duration().is_some()).map_or(!audio(&self.entries[self.current].path),Preview::has_video)}
    pub fn request_playback(&mut self){if let Some(p)=&mut self.preview{p.request_playback();self.finished=false;self.initial_pending=true;self.state.completed=false;}}
    pub fn save(&mut self,path:&Path) {
        self.state.item=self.entries[self.current].path.clone();self.state.index=self.current;self.state.identity=self.identity.clone();
        self.state.position=self.preview.as_ref().map_or(0.0,Preview::playlist_position);
        let(tx,rx)=std::sync::mpsc::channel();
        if state_writer().send(StateJob::Save(path.to_owned(),self.state.clone(),tx,self.ctx.clone())).is_ok(){self.writes.push(rx);}else{self.error=Some(error("播放列表记录保存失败"));}
        self.poll_writes();
        self.saved=Instant::now();
    }
    fn poll_writes(&mut self){
        self.writes.retain(|rx|match rx.try_recv(){Ok(Ok(()))=>false,Ok(Err(e))=>{self.error=Some(e);false},Err(std::sync::mpsc::TryRecvError::Empty)=>true,Err(std::sync::mpsc::TryRecvError::Disconnected)=>{self.error=Some(error("播放列表记录保存失败"));false}});
    }
    fn save_document(&mut self,path:&Path)->bool {
        if crate::media_history::Identity::read(path)!=self.document_identity{self.error=Some(error("列表文件已被外部修改，请重新打开后再编辑"));return false;}
        match write(path,&self.entries,true){Ok(())=>{self.dirty=false;self.edit=false;self.document_identity=crate::media_history::Identity::read(path);true},Err(e)=>{self.error=Some(e);false}}
    }
    pub fn deactivate(&mut self,path:&Path){if let Some(p)=&mut self.preview{p.deactivate();}self.save(path);}
    fn switch(&mut self,index:usize,path:&Path)->bool{
        if index>=self.entries.len(){return false;}
        if !self.entries[index].path.is_file(){self.error=Some(format!("{}: {}",tr("列表中的媒体文件不存在"),self.entries[index].path.display()));return false;}
        self.deactivate(path);self.current=index;self.identity=crate::media_history::Identity::read(&self.entries[index].path);
        let start=if self.state.episodes&&!audio(&self.entries[index].path){parse_time(&self.state.intro).unwrap_or(0.0)}else{0.0};
        self.preview=Some(Preview::for_playlist(start,true));self.initial_pending=true;self.finished=false;self.state.completed=false;self.last_prompt=false;self.error=None;self.scroll_current=true;self.save(path);true
    }
    fn next(&mut self,path:&Path,is_audio:bool,manual:bool) {
        if is_audio && self.state.mode==2 && !manual {self.switch(self.current,path);return;}
        if is_audio && self.state.mode==1 {
            if self.shuffle.is_empty(){self.shuffle=shuffle_order(self.entries.len(),self.current,!self.shuffle_started,&mut self.random);self.shuffle_started=true;}
            let next=self.shuffle.pop().unwrap_or(self.current);self.switch(next,path);return;
        }
        if self.current+1<self.entries.len(){self.switch(self.current+1,path);}else{
            if let Some(p)=&mut self.preview{p.pause();}self.finished=true;self.state.completed=true;
            if !is_audio{self.last_prompt=true;}self.save(path);
        }
    }
    fn panel(&mut self,ui:&mut egui::Ui,path:&Path){
        ui.add_space(8.0);
        ui.horizontal(|ui|{ui.label(path.file_name().unwrap_or_default().to_string_lossy());ui.menu_image_button(egui::Image::new(egui::include_image!("../assets/icons/wrench.svg")).fit_to_exact_size(egui::vec2(16.0,16.0)).tint(ui.visuals().text_color()),|ui|{
            if ui.button(tr(if self.edit{"退出编辑"}else{"编辑列表"})).clicked(){self.edit=!self.edit;ui.close_menu();}
            if ui.button(tr("添加文件")).clicked(){if let Some(files)=rfd::FileDialog::new().pick_files(){for file in files {if crate::media_preview::is_media(&file)&&self.entries.len()<MAX_ENTRIES{self.entries.push(Entry{title:title(&file),path:media_path(&file)});self.dirty=true;}}}ui.close_menu();}
            if ui.button(tr("保存列表")).clicked(){self.save_document(path);ui.close_menu();}
        });});
        if self.dirty {ui.small(tr("列表修改尚未保存"));}
        if self.state.completed{ui.small(tr("列表已播放完毕，可点击条目重新播放"));}
        let is_audio=self.preview.as_ref().filter(|p|p.playlist_duration().is_some()).map_or(audio(&self.entries[self.current].path),|p|!p.has_video());
        if !is_audio {ui.horizontal(|ui|{ui.label(tr("显示方式"));ui.selectable_value(&mut self.state.numbers,true,tr("编号"));ui.selectable_value(&mut self.state.numbers,false,tr("名称"));});}
        let bottom=if is_audio{40.0}else{130.0};
        let mut select=None;let mut remove=None;let mut reorder=None;
        egui::ScrollArea::vertical().auto_shrink([false,true]).id_salt(("playlist_entries",path)).max_height((ui.available_height()-bottom).max(50.0)).show(ui,|ui|{
            if !is_audio && self.state.numbers && !self.edit {
                let columns=(ui.available_width()/54.0).floor().max(1.0) as usize;
                egui::Grid::new(("playlist_numbers",path)).spacing(egui::vec2(4.0,4.0)).show(ui,|ui|{
                    for (index,item) in self.entries.iter().enumerate(){let response=ui.add_sized([50.0,44.0],egui::Button::new((index+1).to_string()).selected(index==self.current));
                        if self.scroll_current&&index==self.current{response.scroll_to_me(Some(egui::Align::Center));}
                        if response.clicked(){select=Some(index);}
                        response.on_hover_text(format!("{}\n{}",item.title,item.path.display()));if (index+1)%columns==0{ui.end_row();}
                    }
                });
            }else{
                for (index,item) in self.entries.iter_mut().enumerate(){ui.horizontal(|ui|{
                    let text=if !is_audio&&self.state.numbers{(index+1).to_string()}else{item.title.clone()};
                    let width=(ui.available_width()-if self.edit{80.0}else{0.0}).max(30.0);
                    let response=ui.add_sized([width,30.0],egui::Button::new(egui::RichText::new(text)).selected(index==self.current).sense(egui::Sense::click_and_drag()).truncate());
                    if self.scroll_current&&index==self.current{response.scroll_to_me(Some(egui::Align::Center));}
                    if response.clicked(){select=Some(index);}
                    if self.edit && response.drag_started(){self.drag=Some(index);}
                    if self.edit && ui.input(|i|i.pointer.any_released()) && response.contains_pointer(){if let Some(from)=self.drag.take(){reorder=Some((from,index));}}
                    let response=response.on_hover_text(format!("{}\n{}",item.title,item.path.display()));
                    response.context_menu(|ui|{ui.label(tr("显示名称"));if ui.text_edit_singleline(&mut item.title).changed(){self.dirty=true;}});
                    if self.edit {if ui.small_button("↑").clicked()&&index>0{reorder=Some((index,index-1));}if ui.small_button("−").on_hover_text(tr("移除条目，不删除文件")).clicked(){remove=Some(index);}}
                });}
            }
        });if self.state.open{self.scroll_current=false;}
        if let Some(index)=select {self.shuffle.clear();self.shuffle_started=false;self.switch(index,path);}
        if let Some((from,to))=reorder{let item=self.entries.remove(from);self.entries.insert(to,item);self.current=if self.current==from{to}else if from<self.current&&to>=self.current{self.current-1}else if from>self.current&&to<=self.current{self.current+1}else{self.current};self.dirty=true;self.shuffle.clear();self.shuffle_started=false;}
        if let Some(index)=remove {if self.entries.len()>1{if index==self.current{let next=if index+1<self.entries.len(){index+1}else{index-1};if !self.switch(next,path){return;}}self.entries.remove(index);if self.current>index{self.current-=1;}self.dirty=true;self.shuffle.clear();self.shuffle_started=false;}}
        ui.add_space((ui.available_height()-bottom).max(0.0));
        ui.separator();
        if is_audio {ui.horizontal(|ui|{ui.label(tr("播放模式"));egui::ComboBox::from_id_salt(("playlist_mode",path)).selected_text(tr(["顺序播放","随机播放","单曲循环"][self.state.mode as usize])).show_ui(ui,|ui|{for (i,name) in ["顺序播放","随机播放","单曲循环"].iter().enumerate(){if ui.selectable_value(&mut self.state.mode,i as u8,tr(name)).changed(){self.shuffle.clear();self.shuffle_started=false;}}});});}
        else {
            let position=self.preview.as_ref().filter(|p|p.playlist_duration().is_some()).map(Preview::playlist_position).filter(|v|v.is_finite()&&*v>=0.0);
            let duration=self.preview.as_ref().and_then(Preview::playlist_duration).filter(|v|v.is_finite()&&*v>0.0);
            ui.horizontal(|ui|{ui.checkbox(&mut self.state.episodes,tr("跳过片头片尾"));if ui.button(tr("重置")).clicked(){self.state.intro="0".into();self.state.outro="0".into();}});ui.add_enabled_ui(self.state.episodes,|ui|{
            egui::Grid::new(("episode_seconds_compact",path)).spacing(egui::vec2(4.0,4.0)).show(ui,|ui|{
                ui.label(tr("跳过片头"));ui.horizontal(|ui|{
                    ui.spacing_mut().item_spacing.x=2.0;
                    ui.add_sized([60.0,24.0],egui::TextEdit::singleline(&mut self.state.intro).desired_width(60.0).min_size(egui::vec2(60.0,24.0)));ui.label("s");
                    if ui.add_enabled(position.is_some(),egui::Button::new(tr("片头"))).on_hover_text(tr("使用当前播放时间设置片头")).clicked(){self.state.intro=skip_time(position.unwrap());}
                });ui.end_row();
                ui.label(tr("跳过片尾"));ui.horizontal(|ui|{
                    ui.spacing_mut().item_spacing.x=2.0;
                    ui.add_sized([60.0,24.0],egui::TextEdit::singleline(&mut self.state.outro).desired_width(60.0).min_size(egui::vec2(60.0,24.0)));ui.label("s");
                    if ui.add_enabled(position.is_some()&&duration.is_some(),egui::Button::new(tr("片尾"))).on_hover_text(tr("使用总时长减去当前播放时间设置片尾")).clicked(){self.state.outro=skip_time((duration.unwrap()-position.unwrap()).max(0.0));}
                });ui.end_row();
            });
            ui.small(tr("仅对当前列表生效"));
        });}
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path,fullscreen:bool){
        self.ctx=Some(ui.ctx().clone());
        self.poll_writes();
        let old_settings=(self.state.mode,self.state.numbers,self.state.episodes,self.state.intro.clone(),self.state.outro.clone(),self.state.open,self.state.width,self.state.height_ratio);
        let controls_visible=!fullscreen||crate::media_preview::fullscreen_controls_visible(ui.ctx());
        let bounds=ui.available_rect_before_wrap();
        let max_width=(bounds.width()/3.0-6.0).max(1.0);
        let min_width=214.0_f32.min(max_width);
        self.state.width=self.state.width.clamp(min_width,max_width);
        let animated=ui.ctx().animate_bool(egui::Id::new(("playlist_open",path)),self.state.open);
        let panel_width=if animated<=0.001{0.0}else{(self.state.width+6.0)*animated};
        let media_path=self.entries[self.current].path.clone();
        let was_playing=self.preview.as_ref().is_some_and(Preview::playlist_playing);
        let (arrow_now,arrow_pointer,arrow_moved)=ui.input(|input|(input.time,input.pointer.hover_pos(),input.events.iter().any(|event|matches!(event,egui::Event::PointerMoved(_)))));
        if arrow_moved&&arrow_pointer.is_some_and(|pos|self.preview.as_ref().and_then(Preview::canvas_rect).is_some_and(|rect|rect.contains(pos))){self.arrow_last_motion=Some(arrow_now);}
        let mut preview=self.preview.take();
        if !fullscreen{ui.horizontal_wrapped(|ui|{crate::asset_viewer::file_context_menu(&ui.heading(media_path.file_name().unwrap_or_default().to_string_lossy()),&media_path);if let Some(p)=&mut preview{if let Some(bytes)=p.file_size(){ui.small(if bytes<1048576{format!("({:.1} KiB)",bytes as f64/1024.0)}else{format!("({:.2} MiB)",bytes as f64/1048576.0)});}p.header_controls(ui,&media_path);}if preview.as_ref().filter(|p|p.playlist_duration().is_some()).map_or(audio(&media_path),|p|!p.has_video()){ui.label(tr(["顺序播放","随机播放","单曲循环"][self.state.mode as usize]));}if let Some(p)=&mut preview{p.trim_entry_button(ui,&media_path);}});}
        let max_height=(bounds.height()-136.0).max(1.0);
        let min_height=240.0_f32.min(max_height);
        let ratio=if self.state.height_ratio.is_finite(){self.state.height_ratio}else{0.5};
        let floating_height=(bounds.height()*ratio).clamp(min_height,max_height);
        let floating_rect=(fullscreen&&panel_width>6.0).then(||egui::Rect::from_center_size(egui::pos2(bounds.right()-panel_width/2.0,bounds.center().y),egui::vec2(panel_width,floating_height)));
        if let Some(rect)=floating_rect.filter(|_|controls_visible){ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("media_overlay_exclusion"),rect.expand2(egui::vec2(24.0,0.0))));}
        if let Some(p)=&mut preview{p.set_overlay_interactive(self.arrow_last_motion.is_some_and(|last|arrow_now-last<1.0)&&controls_visible);p.set_sidebar_width(if fullscreen{0.0}else{panel_width});p.show(ui,&media_path);}else{ui.label(tr("列表中的媒体文件不存在"));}
        let player_rect=preview.as_ref().and_then(Preview::canvas_rect).unwrap_or_else(||{
            let available=ui.available_rect_before_wrap();egui::Rect::from_min_max(available.min,egui::pos2(available.right()-panel_width,available.bottom()-32.0))
        });
        ui.ctx().data_mut(|data|data.remove::<egui::Rect>(egui::Id::new("media_overlay_exclusion")));
        let panel_rect=if fullscreen{floating_rect}else{preview.as_ref().and_then(Preview::sidebar_rect).or_else(||(panel_width>6.0).then(||egui::Rect::from_min_max(egui::pos2(player_rect.right()+6.0,player_rect.top()),egui::pos2(bounds.right(),player_rect.bottom()))))};
        self.preview=preview;
        if controls_visible&&animated>0.01 {if let Some(panel_rect)=panel_rect{
            let mut render=|ui:&mut egui::Ui|{
                ui.set_clip_rect(panel_rect.intersect(ui.ctx().screen_rect()));
                let divider=if fullscreen{egui::Rect::from_min_max(panel_rect.left_top(),panel_rect.left_bottom()+egui::vec2(6.0,0.0))}else{egui::Rect::from_min_max(panel_rect.left_top()-egui::vec2(6.0,0.0),panel_rect.left_bottom())};
                let handle=ui.interact(divider,ui.id().with("playlist_resize"),egui::Sense::drag());
                if handle.hovered()||handle.dragged(){ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);}
                if handle.dragged(){self.state.width=(self.state.width-ui.input(|i|i.pointer.delta().x)).clamp(min_width,max_width);}
                
                if fullscreen{
                    for (edge,sign) in [(0,-1.0),(1,1.0)]{
                        let y=if edge==0{panel_rect.top()}else{panel_rect.bottom()-6.0};
                        let resize=egui::Rect::from_min_size(egui::pos2(panel_rect.left()+6.0,y),egui::vec2((panel_rect.width()-12.0).max(1.0),6.0));
                        let response=ui.interact(resize,ui.id().with(("playlist_height",edge)),egui::Sense::drag());
                        if response.hovered()||response.dragged(){ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);}
                        if response.dragged(){let height=(floating_height+sign*2.0*ui.input(|i|i.pointer.delta().y)).clamp(min_height,max_height);self.state.height_ratio=height/bounds.height().max(1.0);}
                    }
                }
                egui::Frame::NONE.inner_margin(egui::Margin {left:2,right:0,top:0,bottom:0}).show(ui,|ui|self.panel(ui,path));
            };
            if fullscreen{
                egui::Area::new(egui::Id::new(("fullscreen_playlist_panel",path))).order(egui::Order::Foreground).constrain(false).movable(false).fixed_pos(panel_rect.min).show(ui.ctx(),|ui|{
                    ui.set_width(panel_rect.width());
                    egui::Frame::NONE.fill(ui.visuals().window_fill().gamma_multiply(0.45_f32)).stroke(egui::Stroke::new(1.0_f32,ui.visuals().widgets.noninteractive.fg_stroke.color.gamma_multiply(0.5_f32))).inner_margin(egui::Margin {left:14,right:6,top:14,bottom:14}).corner_radius(egui::CornerRadius {nw:30,ne:0,sw:30,se:0}).show(ui,|ui|{
                        ui.set_min_size((panel_rect.size()-egui::vec2(20.0,28.0)).max(egui::vec2(1.0,1.0)));
                        ui.set_max_size((panel_rect.size()-egui::vec2(20.0,28.0)).max(egui::vec2(1.0,1.0)));
                        render(ui);
                    });
                });
            }else{ui.scope_builder(egui::UiBuilder::new().max_rect(panel_rect),render);}
        }}
        let arrow=if let Some(panel)=floating_rect{egui::Rect::from_center_size(egui::pos2(panel.left()-12.0,panel.center().y),egui::vec2(24.0,60.0))}else{self.preview.as_ref().and_then(Preview::overlay_rect).unwrap_or_else(||egui::Rect::from_center_size(egui::pos2(player_rect.right()-12.0,player_rect.center().y),egui::vec2(24.0,60.0)))};
        if arrow_moved&&arrow_pointer.is_some_and(|pos|player_rect.contains(pos)){self.arrow_last_motion=Some(arrow_now);}
        let arrow_visible=controls_visible&&arrow_pointer.is_some_and(|pos|player_rect.contains(pos)||arrow.contains(pos))&&self.arrow_last_motion.is_some_and(|last|arrow_now-last<1.0);
        if arrow_visible{ui.ctx().request_repaint_after(Duration::from_secs_f64((1.0-(arrow_now-self.arrow_last_motion.unwrap())).max(0.001)));}
        if arrow_visible{egui::Area::new(egui::Id::new(("playlist_toggle",path))).order(egui::Order::Foreground).fixed_pos(arrow.min).show(ui.ctx(),|ui|{
            let response=ui.add_sized(arrow.size(),egui::Button::new("").fill(egui::Color32::from_black_alpha(110)));
            let icon_rect=egui::Rect::from_center_size(response.rect.center(),egui::vec2(18.0,18.0));
            crate::icons::paint(ui,if self.state.open{crate::icons::Icon::ChevronRight}else{crate::icons::Icon::ChevronLeft},icon_rect,egui::Color32::WHITE);
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(if self.state.open{"收起播放列表"}else{"展开播放列表"})).clicked(){self.state.open=!self.state.open;self.scroll_current=self.state.open;}
        });
        }
        let mut advance=false;let is_audio=self.preview.as_ref().filter(|p|p.playlist_duration().is_some()).map_or(audio(&media_path),|p|!p.has_video());
        let manual_next=self.preview.as_mut().is_some_and(Preview::take_playlist_next);
        if let Some(p)=&mut self.preview {
            let ended=p.take_playlist_end();
            if !manual_next {
            let range=if self.state.episodes&&p.has_video(){parse_time(&self.state.intro).zip(parse_time(&self.state.outro))}else{Some((0.0,0.0))};
            if range.is_none(){p.pause();self.error=Some(error("片头片尾设置无效，请检查秒数与媒体时长"));}
            else if p.playlist_duration()==Some(0.0)&&range.is_some_and(|(_,outro)|outro>0.0){p.pause();self.error=Some(error("媒体时长未知，无法跳过片尾"));}
            else if let Some(duration)=p.playlist_duration().filter(|d|*d>0.0) {
                let valid=range.is_some_and(|(intro,outro)|intro+outro<duration);
                if !valid{if p.playlist_playing(){p.pause();}self.error=Some(error("片头片尾设置无效，请检查秒数与媒体时长"));}
                else if !self.finished && !p.playlist_failed(){
                    if self.error.as_deref()==Some(tr("片头片尾设置无效，请检查秒数与媒体时长")){self.error=None;}
                    let (intro,outro)=range.unwrap();
                    if self.initial_pending {self.initial_pending=false;if self.state.episodes && p.has_video() && p.playlist_position()<intro{p.playlist_seek(ui.ctx(),&media_path,intro);}}
                    let at_outro=outro>0.0&&p.playlist_playing()&&p.playlist_position()>=duration-outro;
                    if ended||at_outro {p.pause();self.finished=true;advance=true;}
                }
            }else if ended&&!self.finished {self.finished=true;advance=true;}
        }
        }
        if manual_next{self.next(path,is_audio,true);}else if advance{self.next(path,is_audio,false);}
        if self.finished&&self.preview.as_ref().is_some_and(Preview::playlist_playing){
            self.finished=false;self.state.completed=false;
            if self.state.episodes&&self.has_video(){if let Some(p)=&mut self.preview{let intro=parse_time(&self.state.intro).unwrap_or(0.0);if p.playlist_position()<intro{p.playlist_seek(ui.ctx(),&media_path,intro);}}}
        }
        let changed=old_settings!=(self.state.mode,self.state.numbers,self.state.episodes,self.state.intro.clone(),self.state.outro.clone(),self.state.open,self.state.width,self.state.height_ratio);
        if changed || (was_playing&&!self.preview.as_ref().is_some_and(Preview::playlist_playing)) || self.saved.elapsed()>Duration::from_secs(10){self.save(path);}
        if let Some(e)=&self.error {egui::Area::new(ui.id().with("playlist_error")).order(egui::Order::Foreground).fixed_pos(player_rect.left_top()+egui::vec2(12.0,70.0)).show(ui.ctx(),|ui|{egui::Frame::popup(ui.style()).show(ui,|ui|{ui.set_max_width((player_rect.width()-30.0).max(40.0));ui.colored_label(ui.visuals().warn_fg_color,e);});});}
        if self.last_prompt {egui::Area::new(ui.id().with("playlist_last_episode")).order(egui::Order::Foreground).pivot(egui::Align2::CENTER_CENTER).fixed_pos(player_rect.center()).show(ui.ctx(),|ui|{egui::Frame::popup(ui.style()).inner_margin(20.0).show(ui,|ui|{ui.label(tr("已是最后一集"));if ui.button(tr("确定")).clicked(){self.last_prompt=false;}});});}
        ui.allocate_rect(bounds,egui::Sense::hover());
    }
}

pub struct Creator {
    protected:Vec<PathBuf>,open:bool,width:f32,directory:PathBuf,audio:bool,prefix_mode:bool,prefix:String,name:String,
    entries:Vec<Entry>,error:Option<String>,pending:Option<PathBuf>,overwrite:bool,ready:Option<PathBuf>,
    job:Option<std::sync::mpsc::Receiver<Result<Vec<Entry>,String>>>,job_stop:Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,drag:Option<usize>,
}
impl Creator {
    pub fn new(media:&Path)->Self{let prefix=candidate_prefix(media);let name=if prefix.is_empty(){title(media)}else{prefix.trim_end_matches(['_',' ','-','.']).to_owned()};Self {protected:Vec::new(),open:false,width:300.0,directory:media.parent().unwrap_or(Path::new(".")).to_owned(),audio:audio(media),prefix_mode:false,prefix,name,entries:Vec::new(),error:None,pending:None,overwrite:false,ready:None,job:None,job_stop:None,drag:None}}
    fn scan(&mut self,ctx:&egui::Context){
        if let Some(cancel)=self.job_stop.take(){cancel.store(true,std::sync::atomic::Ordering::Relaxed);}self.job=None;
        if self.prefix_mode&&self.prefix.trim().is_empty(){self.error=Some(error("请填写文件名前缀"));return;}
        let directory=self.directory.clone();let is_audio=self.audio;let prefix=self.prefix_mode.then(||self.prefix.clone());let ctx=ctx.clone();let(tx,rx)=std::sync::mpsc::channel();self.job=Some(rx);self.error=None;
        let cancel=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));self.job_stop=Some(cancel.clone());
        std::thread::spawn(move||{let result=(||{let mut entries=Vec::new();for entry in std::fs::read_dir(directory).map_err(|e|e.to_string())?{if cancel.load(std::sync::atomic::Ordering::Relaxed){return Ok(Vec::new());}let entry=entry.map_err(|e|e.to_string())?;let path=entry.path();if !entry.file_type().map_err(|e|e.to_string())?.is_file()||audio(&path)!=is_audio||!crate::media_preview::is_media(&path){continue;}
            let stem=title(&path);if prefix.as_ref().is_some_and(|p|!stem.starts_with(p)){continue;}
            let display=prefix.as_ref().and_then(|p|stem.strip_prefix(p)).filter(|s|!s.is_empty()).unwrap_or(&stem).to_owned();entries.push(Entry{path:media_path(&path),title:display});if entries.len()>MAX_ENTRIES{return Err(error("播放列表条目过多"));}}
            entries.sort_by(|a,b|natural_cmp(&a.path.to_string_lossy(),&b.path.to_string_lossy()).then_with(||a.path.cmp(&b.path)));Ok(entries)})();let _=tx.send(result);ctx.request_repaint();});
    }
    fn commit(&mut self){if let Some(path)=self.pending.take(){
        if self.protected.iter().any(|p|media_path(p)==media_path(&path)){self.error=Some(error("该列表正在编辑，请先保存或放弃修改再覆盖"));return;}
        flush_writes();
        match write(&path,&self.entries,self.overwrite){Ok(())=>{self.ready=Some(path);self.open=false;},Err(e)=>self.error=Some(e)}self.overwrite=false;}}
    pub fn panel(&mut self,ui:&mut egui::Ui){
        if let Some(job)=&self.job {match job.try_recv(){Ok(result)=>{self.job=None;match result{Ok(entries)=>self.entries=entries,Err(e)=>self.error=Some(e)}},Err(std::sync::mpsc::TryRecvError::Disconnected)=>{self.job=None;self.error=Some(error("目录扫描失败"));},Err(std::sync::mpsc::TryRecvError::Empty)=>{ui.ctx().request_repaint_after(Duration::from_millis(100));}}}
        ui.horizontal(|ui|{ui.heading(tr("创建播放列表"));if ui.button(tr("打开列表")).clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("M3U / M3U8",&["m3u8","m3u"]).pick_file(){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("playlist_open"),path));}}});
        ui.horizontal(|ui|{ui.label(tr("目录"));if ui.button(tr("选择目录")).clicked(){if let Some(path)=rfd::FileDialog::new().set_directory(&self.directory).pick_folder(){self.directory=path;self.pending=None;self.entries.clear();self.scan(ui.ctx());}}});ui.label(self.directory.to_string_lossy());
        let mut refresh=false;ui.horizontal(|ui|{refresh|=ui.selectable_value(&mut self.audio,true,tr("音频")).changed();refresh|=ui.selectable_value(&mut self.audio,false,tr("视频")).changed();});
        refresh|=ui.radio_value(&mut self.prefix_mode,false,tr("目录内全部")).changed();refresh|=ui.radio_value(&mut self.prefix_mode,true,tr("文件名前缀匹配")).changed();
        ui.add_enabled_ui(self.prefix_mode,|ui|{ui.label(tr("前缀"));refresh|=ui.text_edit_singleline(&mut self.prefix).changed();});
        if refresh{self.entries.clear();self.pending=None;self.scan(ui.ctx());}
        ui.label(tr("列表名称"));if ui.text_edit_singleline(&mut self.name).changed(){self.pending=None;}
        if self.job.is_some(){ui.spinner();}else{ui.label(format!("{}: {}",tr("匹配条目"),self.entries.len()));}
        let mut remove=None;let mut reorder=None;
        egui::ScrollArea::vertical().auto_shrink([false,true]).id_salt("playlist_create_preview").max_height((ui.available_height()-125.0).max(40.0)).show(ui,|ui|{
            for (index,item) in self.entries.iter().enumerate(){ui.horizontal(|ui|{let response=ui.add_sized([(ui.available_width()-70.0).max(20.0),26.0],egui::Label::new(&item.title).truncate().sense(egui::Sense::drag()));if response.drag_started(){self.drag=Some(index);}if ui.input(|i|i.pointer.any_released())&&response.contains_pointer(){if let Some(from)=self.drag.take(){reorder=Some((from,index));}}response.on_hover_text(item.path.display().to_string());if ui.small_button("↑").clicked()&&index>0{reorder=Some((index,index-1));}if ui.small_button("−").clicked(){remove=Some(index);}});}
        });if let Some((from,to))=reorder{let entry=self.entries.remove(from);self.entries.insert(to,entry);}if let Some(index)=remove{self.entries.remove(index);}
        ui.add_space((ui.available_height()-if self.pending.is_some(){110.0}else if self.error.is_some(){76.0}else{40.0}).max(0.0));
        if let Some(path)=self.pending.clone(){ui.colored_label(ui.visuals().warn_fg_color,tr("同名文件已存在，覆盖将替换原列表内容"));ui.label(path.display().to_string());ui.horizontal(|ui|{if ui.button(tr("覆盖")).clicked(){self.overwrite=true;self.commit();}if ui.button(tr("修改名称")).clicked(){self.pending=None;}if ui.button(tr("取消")).clicked(){self.pending=None;}});}
        else if ui.with_layout(egui::Layout::top_down(egui::Align::Center),|ui|{
            ui.add_enabled_ui(!self.entries.is_empty()&&self.job.is_none(),|ui|ui.add_sized([128.0_f32.min(ui.available_width()),36.0],egui::Button::new(tr("保存并使用")))).inner.clicked()
        }).inner{
            if self.name.trim().is_empty(){self.error=Some(error("请填写列表名称"));}
            else if self.name.chars().any(|c|c.is_control()||matches!(c,'<'|'>'|':'|'\\'|'/'|'|'|'?'|'*'|'"')){self.error=Some(error("列表名称包含无效字符"));}
            else if let Some(mut path)=rfd::FileDialog::new().set_directory(&self.directory).set_file_name(format!("{}.m3u8",self.name.trim().trim_end_matches(".m3u8"))).add_filter("M3U8",&["m3u8"]).save_file(){path.set_extension("m3u8");self.pending=Some(path.clone());if !path.exists(){self.commit();}}
        }
        if let Some(e)=&self.error{ui.colored_label(ui.visuals().warn_fg_color,e);}
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path,preview:&mut Preview){
        let bounds=ui.available_rect_before_wrap();
        let top=ui.horizontal_wrapped(|ui|{crate::asset_viewer::file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);if let Some(bytes)=preview.file_size(){ui.small(if bytes<1048576{format!("({:.1} KiB)",bytes as f64/1024.0)}else{format!("({:.2} MiB)",bytes as f64/1048576.0)});}preview.header_controls(ui,path);preview.trim_entry_button(ui,path);if ui.button(tr("播放列表")).clicked(){self.open=!self.open;if self.open&&self.entries.is_empty(){self.scan(ui.ctx());}}}).response;
        let fraction=ui.ctx().animate_bool(ui.id().with("playlist_creator_open"),self.open);
        let max_width=(bounds.width()/3.0-6.0).max(1.0);let min_width=214.0_f32.min(max_width);
        self.width=self.width.clamp(min_width,max_width);
        let width=self.width;
        preview.set_sidebar_width((width+6.0)*fraction);
        preview.show(ui,path);
        if fraction>0.01 {if let (Some(player),Some(panel))=(preview.canvas_rect(),preview.sidebar_rect()){
            let divider=egui::Rect::from_min_max(egui::pos2(player.right(),player.top()),egui::pos2(player.right()+6.0,player.bottom()));
            let handle=ui.interact(divider,ui.id().with("creator_resize"),egui::Sense::drag());
            if handle.hovered()||handle.dragged(){ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);}
            if handle.dragged(){self.width=(self.width-ui.input(|i|i.pointer.delta().x)).clamp(min_width,max_width);}
            ui.scope_builder(egui::UiBuilder::new().max_rect(panel),|ui|{ui.set_clip_rect(panel.intersect(ui.clip_rect()));egui::Frame::NONE.inner_margin(egui::Margin {left:2,right:0,top:0,bottom:0}).show(ui,|ui|self.panel(ui));});
        }}
        if let Some(list)=self.ready.take(){let position=preview.playlist_position();let entry=self.entries.iter().position(|e|media_path(&e.path)==media_path(path)).unwrap_or(0);let mut state=State::default();state.index=entry;state.item=media_path(&self.entries[entry].path);state.identity=crate::media_history::Identity::read(&state.item);state.position=if state.item==media_path(path){position}else{0.0};state.numbers=!self.prefix_mode;if let Err(e)=store_state(&list,&state){crate::media_history::report_error(&e);}ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("playlist_created"),(path.to_owned(),list)));}
        ui.allocate_rect(bounds.union(top.rect),egui::Sense::hover());
    }
}

impl Drop for Creator {fn drop(&mut self){if let Some(cancel)=&self.job_stop{cancel.store(true,std::sync::atomic::Ordering::Relaxed);}}}
#[derive(Default)]
pub struct Lists { sessions:HashMap<PathBuf,Session>,creators:HashMap<PathBuf,Creator>,errors:HashMap<PathBuf,String> }
impl Lists {
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path,fullscreen:bool){
        if !self.sessions.contains_key(path)&&!self.errors.contains_key(path){match Session::open(path){Ok(s)=>{self.sessions.insert(path.to_owned(),s);},Err(e)=>{self.errors.insert(path.to_owned(),e);}}}
        if let Some(s)=self.sessions.get_mut(path){s.show(ui,path,fullscreen);}else if let Some(e)=self.errors.get(path){ui.colored_label(ui.visuals().warn_fg_color,e);if ui.button(tr("重试")).clicked(){self.errors.remove(path);}}
    }
    pub fn show_creator(&mut self,ui:&mut egui::Ui,path:&Path,preview:&mut Preview){
        let protected=self.dirty_paths();let creator=self.creators.entry(path.to_owned()).or_insert_with(||Creator::new(path));creator.protected=protected;creator.show(ui,path,preview);
    }
    pub fn pause(&mut self){for(path,s)in &mut self.sessions{s.deactivate(path);}}
    pub fn forget(&mut self,path:&Path){if let Some(mut s)=self.sessions.remove(path){s.deactivate(path);flush_writes();s.poll_writes();}self.errors.remove(path);self.creators.remove(path);}
    pub fn request(&mut self,path:&Path){if let Some(s)=self.sessions.get_mut(path){s.request_playback();}}
    pub fn has_video(&self,path:&Path)->bool{self.sessions.get(path).is_some_and(Session::has_video)}
    pub fn retry(&mut self,path:&Path){if let Some(s)=self.sessions.get_mut(path){if let Some(p)=&mut s.preview{p.retry_missing_decoder();}}else{self.errors.remove(path);}}
    pub fn invalidate(&mut self,path:&Path){self.sessions.remove(path);self.errors.remove(path);}
    pub fn error(&self,path:&Path)->Option<&str>{self.sessions.get(path).and_then(|s|s.error.as_deref())}
    pub fn dirty(&self,path:&Path)->bool{self.sessions.get(path).is_some_and(|s|s.dirty)}
    pub fn dirty_paths(&self)->Vec<PathBuf>{self.sessions.iter().filter(|(_,s)|s.dirty).map(|(p,_)|p.clone()).collect()}
    pub fn resolve_dirty(&mut self,path:&Path,save:bool)->bool{self.sessions.get_mut(path).is_none_or(|s|{if save{s.save_document(path)}else{s.dirty=false;true}})}
    pub fn in_use(&self)->Vec<PathBuf>{self.sessions.values().flat_map(|s|s.entries.iter().map(|e|e.path.clone())).collect()}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn times_and_natural_order(){assert_eq!(parse_time("300"),Some(300.0));assert_eq!(parse_time("0"),Some(0.0));assert_eq!(parse_time("2:00"),None);assert_eq!(migrate_skip("75:59"),"4559");assert_eq!(migrate_skip("2:00"),"120");assert_eq!(migrate_skip("2:60"),"2:60");assert_eq!(skip_time(300.9),"300");assert_eq!(parse_time("2:60"),None);assert_eq!(parse_time("-1:00"),None);assert!(natural_cmp("第2集","第10集").is_lt());assert_eq!(candidate_prefix(Path::new("宫崎骏_龙猫.mp4")),"宫崎骏_");}

    struct Workspace(PathBuf);
    impl Workspace {
        fn new()->Self {
            let path=std::env::current_dir().unwrap().join("target").join(format!("playlist-test-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
            std::fs::create_dir_all(&path).unwrap();Self(path)
        }
    }
    impl Drop for Workspace {fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}

    #[test]
    fn portable_roundtrip_and_overwrite_protection(){
        let workspace=Workspace::new();let media=workspace.0.join("宫崎骏_龙猫 1988.mp4");std::fs::write(&media,b"fixture").unwrap();
        let list=workspace.0.join("电影.m3u8");let entries=vec![Entry {path:media.clone(),title:"龙猫, 1988".into()}];
        write(&list,&entries,false).unwrap();
        let document=std::fs::read_to_string(&list).unwrap();assert!(document.contains("#EXTINF:-1,龙猫, 1988"));assert!(document.contains("宫崎骏_龙猫 1988.mp4"));assert!(!document.contains(workspace.0.to_str().unwrap()));
        let loaded=read(&list).unwrap();assert_eq!(loaded[0].path,media_path(&media));assert_eq!(loaded[0].title,"龙猫, 1988");
        assert!(write(&list,&entries,false).is_err());assert_eq!(std::fs::read_to_string(&list).unwrap(),document);
        write(&list,&[Entry {path:media,title:"新标题".into()}],true).unwrap();assert_eq!(read(&list).unwrap()[0].title,"新标题");
    }

    #[test]
    fn rejects_hls_urls_and_invalid_utf8_but_keeps_missing_entries(){
        let workspace=Workspace::new();let list=workspace.0.join("test.m3u8");
        for text in ["#EXTM3U\n#EXT-X-TARGETDURATION:10\nsegment.ts\n","#EXTM3U\nhttps://example.invalid/movie.mp4\n"] {std::fs::write(&list,text).unwrap();assert!(read(&list).is_err());}
        std::fs::write(&list,[0xff,0xfe]).unwrap();assert!(read(&list).is_err());
        std::fs::write(&list,"\u{feff}#EXTM3U\n#EXTINF:-1,缺失条目\nmissing.mp4\n").unwrap();let entries=read(&list).unwrap();assert_eq!(entries.len(),1);assert_eq!(entries[0].title,"缺失条目");assert!(!entries[0].path.exists());
    }

    #[test]
    fn hash_filename_and_relative_subfolders(){
        let workspace=Workspace::new();let folder=workspace.0.join("媒体");std::fs::create_dir(&folder).unwrap();let item=folder.join("#片名.mp4");std::fs::write(&item,b"x").unwrap();
        let list=workspace.0.join("test.m3u8");write(&list,&[Entry {path:item.clone(),title:"片名".into()}],false).unwrap();assert_eq!(read(&list).unwrap()[0].path,media_path(&item));
        let hash=workspace.0.join("#同目录.mp4");std::fs::write(&hash,b"x").unwrap();write(&list,&[Entry {path:hash.clone(),title:"带井号".into()}],true).unwrap();assert_eq!(read(&list).unwrap()[0].path,media_path(&hash));
    }

    #[test]
    fn shuffle_rounds_and_restore_after_reorder(){
        let mut random=0;
        for current in 0..8 {
            let first=shuffle_order(8,current,true,&mut random);assert_eq!(first.len(),7);assert!(!first.contains(&current));
            let round=shuffle_order(8,current,false,&mut random);let mut unique=round.clone();unique.sort_unstable();assert_eq!(unique,(0..8).collect::<Vec<_>>());assert_ne!(round.last(),Some(&current));
        }
        assert!(shuffle_order(1,0,true,&mut random).is_empty());assert_eq!(shuffle_order(1,0,false,&mut random),vec![0]);
        let items=vec![Entry{path:PathBuf::from("movie-b.mp4"),title:"B".into()},Entry{path:PathBuf::from("movie-a.mp4"),title:"A".into()},Entry{path:PathBuf::from("movie-a.mp4"),title:"A again".into()}];
        assert_eq!(restore_entry(&items,Path::new("movie-a.mp4"),0),Some(1));assert_eq!(restore_entry(&items,Path::new("movie-a.mp4"),2),Some(2));assert_eq!(restore_entry(&items,Path::new("removed.mp4"),0),None);
    }

    #[test]
    fn playlist_player_does_not_use_file_history(){
        let mut player=Preview::for_playlist(12.5,false);assert_eq!(player.playlist_position(),12.5);assert!(!player.playlist_playing());assert!(!player.take_playlist_end());
        player.pause();assert!(!player.take_playlist_end());player.deactivate();assert_eq!(player.playlist_position(),12.5);assert!(!player.take_playlist_end());
    }
}
