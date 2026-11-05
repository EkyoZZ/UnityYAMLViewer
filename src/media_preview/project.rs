//! Versioned media projects. Originals remain references; edited audio is embedded.
use super::*;
use crate::{localization::tr, media_trim::{EditState, MergeSettings}};
use serde::{Deserialize, Serialize};
use std::{fs, io::{Read, Write}, sync::atomic::AtomicU64};
mod bundle;

static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Assets(PathBuf);
impl Drop for Assets { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag="storage", rename_all="snake_case")]
enum Reference { External { absolute:PathBuf, relative:Option<PathBuf> }, Embedded { entry:String } }
#[derive(Serialize, Deserialize)]
struct Item {
    #[serde(default="default_enabled")]
    enabled:bool,
    source: Reference,
    state: EditState,
    tracks: Vec<Reference>,
    clip: Option<Reference>,
    position: f64,
    selected_audio: Option<u64>,
}
#[derive(Serialize, Deserialize)]
struct Document {
    format: String,
    version: u32,
    kind: String,
    active: usize,
    expanded: bool,
    merge: bool,
    merge_settings: MergeSettings,
    items: Vec<Item>,
}
fn default_enabled()->bool {true}

struct Loaded { path:PathBuf, document:Document, media:Vec<(PathBuf,Info,Option<(PathBuf,f64)>,Option<egui::ColorImage>)>, assets:Arc<Assets> }
#[derive(Default)]
struct Opens { requests:Vec<PathBuf>, jobs:Vec<mpsc::Receiver<Result<Loaded,String>>>, error:Option<String> }
thread_local! { static OPENS:std::cell::RefCell<Opens> = std::cell::RefCell::new(Opens::default()); }

pub(crate) fn request_open(path:PathBuf) { OPENS.with(|opens| opens.borrow_mut().requests.push(path)); }

pub(crate) fn is_project(path:&Path)->bool {
    path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case("uyap")||ext.eq_ignore_ascii_case("uyvp"))
}

struct Tab {
    pending:Option<mpsc::Receiver<Result<(String,usize,usize,bool),String>>>,
    result:Option<Result<(String,usize,usize,bool),String>>,
    checked:Instant,
    modified:Option<std::time::SystemTime>,
    packing:Option<mpsc::Receiver<Result<PathBuf,String>>>,
    pack_result:Option<Result<PathBuf,String>>,
}
thread_local! {static TABS:std::cell::RefCell<std::collections::HashMap<PathBuf,Tab>>=std::cell::RefCell::new(std::collections::HashMap::new());}

pub(crate) fn show_tab(ui:&mut egui::Ui,path:&Path) {
    TABS.with(|tabs| {
        let mut tabs=tabs.borrow_mut();
        let modified=fs::metadata(path).and_then(|metadata|metadata.modified()).ok();
        let tab=tabs.entry(path.to_owned()).or_insert(Tab{pending:None,result:None,checked:Instant::now(),modified,packing:None,pack_result:None});
        if let Some(receiver)=&tab.packing {
            match receiver.try_recv() {
                Ok(result)=>{tab.pack_result=Some(result);tab.packing=None;},
                Err(mpsc::TryRecvError::Disconnected)=>{tab.pack_result=Some(Err(tr("工程打包失败。").into()));tab.packing=None;},
                Err(mpsc::TryRecvError::Empty)=>ui.ctx().request_repaint_after(Duration::from_millis(100)),
            }
        }
        if tab.pending.is_none() && (tab.modified!=modified || matches!(tab.result,Some(Ok((_,_,_,false)))) && tab.checked.elapsed()>Duration::from_secs(3)) {
            tab.result=None;tab.modified=modified;
        }
        if tab.pending.is_none() && tab.result.is_none() {
            let (tx,rx)=mpsc::channel();tab.pending=Some(rx);
            let path=path.to_owned();let ctx=ui.ctx().clone();
            std::thread::spawn(move|| {
                let result=(|| {
                    let mut archive=zip::ZipArchive::new(fs::File::open(&path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                    let entry=archive.by_name("project.json").map_err(|e|e.to_string())?;
                    if entry.size()>8*1024*1024 {return Err(tr("工程设置无效或格式版本不受支持。").into());}
                    let doc:Document=serde_json::from_reader(entry).map_err(|e|e.to_string())?;
                    let video=doc.kind=="video";
                    if doc.format!="UnityYAMLViewer.MediaProject" || doc.version!=1 || !matches!(doc.kind.as_str(),"audio"|"video")
                        || doc.items.is_empty() || doc.active>=doc.items.len()
                        || !path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case(if video{"uyvp"}else{"uyap"}))
                    {return Err(tr("工程设置无效或格式版本不受支持。").into());}
                    let ready=crate::media_tools::find_tool("ffmpeg").is_some() && crate::media_tools::find_tool("ffprobe").is_some();
                    Ok((doc.kind,doc.items.len(),doc.items.iter().filter(|item|item.enabled).count(),ready))
                })();
                let _=tx.send(result);ctx.request_repaint();
            });
        }
        if let Some(pending)=&tab.pending {
            match pending.try_recv() {
                Ok(result)=>{tab.result=Some(result);tab.pending=None;tab.checked=Instant::now();},
                Err(mpsc::TryRecvError::Disconnected)=>{tab.pending=None;tab.result=Some(Err(tr("打开工程失败").into()));},
                Err(mpsc::TryRecvError::Empty)=>{ui.ctx().request_repaint_after(Duration::from_millis(100));},
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.heading(path.file_name().unwrap_or_default().to_string_lossy());
            if let Ok(metadata)=fs::metadata(path) {
                let bytes=metadata.len() as f64;
                ui.small(if bytes>=1048576.0 {format!("({:.2} MiB)",bytes/1048576.0)}else{format!("({:.1} KiB)",bytes/1024.0)});
            }
            if tab.result.as_ref().is_some_and(|result|result.is_ok()) {
                if ui.add_enabled(tab.packing.is_none(),egui::Button::new(tr("打包工程"))).clicked() {
                    let name=path.with_extension("zip");
                    if let Some(target)=rfd::FileDialog::new().add_filter("ZIP",&["zip"]).set_file_name(name.file_name().unwrap_or_default().to_string_lossy()).save_file() {
                        tab.packing=Some(bundle::start(ui.ctx(),path.to_owned(),target.with_extension("zip")));
                        tab.pack_result=None;
                    }
                }
            }
        });
        ui.add(egui::Label::new(egui::RichText::new(tr("分享工程时，建议将工程与素材放在同一文件夹内，保存后整体打包发送。使用“打包工程”可自动收集素材；解压后保持内部目录结构不变即可。")).small()).wrap());
        ui.small(tr("打包使用已保存的工程，请先保存编辑窗口中的修改。"));
        if tab.packing.is_some(){ui.horizontal(|ui|{ui.spinner();ui.label(tr("正在打包工程…"));});}
        if let Some(result)=&tab.pack_result {match result {
            Ok(path)=>{ui.label(format!("{}: {}",tr("任务完成"),path.display()));},
            Err(error)=>{ui.colored_label(ui.visuals().error_fg_color,error);},
        }}
        match &tab.result {
            Some(Ok((_,_,_,ready)))=> {
                if !*ready {
                    ui.ctx().request_repaint_after(Duration::from_secs(3));
                    ui.label(tr("未安装 FFmpeg，请前往第三方支持安装。"));
                    if ui.button(tr("第三方支持")).clicked(){crate::third_party::request_open(ui.ctx());}
                    if ui.button(tr("重新检测")).clicked(){tab.result=None;}
                }
            }
            Some(Err(error))=>{ui.colored_label(ui.visuals().error_fg_color,error);},
            None=>{ui.horizontal(|ui| {ui.spinner();ui.label(tr("正在检测…"));});},
        }
        if ui.ctx().data(|data|data.get_temp::<bool>(egui::Id::new("media_decoder_ready"))).unwrap_or(false) {tab.result=None;}
        let area=ui.available_rect_before_wrap();
        let video=path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case("uyvp"));
        let title=tr(if video {"Unity YAML Viewer 视频编辑工程"}else{"Unity YAML Viewer 音频编辑工程"});
        let size=(area.width()/28.0).clamp(14.0,26.0);
        let title=ui.painter().layout(title.to_owned(),egui::FontId::proportional(size),ui.visuals().text_color(),area.width());
        let description=ui.painter().layout(tr("保存素材列表与编辑设置，可继续未完成的编辑。").to_owned(),egui::FontId::proportional(15.0),ui.visuals().weak_text_color(),area.width());
        let ready=matches!(&tab.result,Some(Ok((_,_,_,true))));
        let height=title.size().y+12.0+description.size().y+if ready {56.0}else{0.0};
        let top=(area.center().y-height*0.5-if ready {36.0}else{20.0}).max(area.top()+8.0);
        ui.painter().galley(egui::pos2(area.center().x-title.size().x*0.5,top),title.clone(),ui.visuals().text_color());
        let description_height=description.size().y;
        ui.painter().galley(egui::pos2(area.center().x-description.size().x*0.5,top+title.size().y+12.0),description,ui.visuals().weak_text_color());
        if ready {
            let label=tr("进入编辑窗口");
            let width=(ui.painter().layout_no_wrap(label.to_owned(),egui::FontId::proportional(18.0),egui::Color32::WHITE).size().x+40.0).max(180.0).min(area.width());
            let rect=egui::Rect::from_min_size(egui::pos2(area.center().x-width*0.5,top+title.size().y+12.0+description_height+18.0),egui::vec2(width,38.0));
            if ui.put(rect,egui::Button::new(egui::RichText::new(label).size(18.0).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(30,140,65))).clicked() {
                request_open(path.to_owned());
            }
        }
    });
}

fn external(path:&Path, parent:&Path) -> Reference {
    let absolute=fs::canonicalize(path).unwrap_or_else(|_|path.to_owned());
    let parent=fs::canonicalize(parent).unwrap_or_else(|_|parent.to_owned());
    let relative=absolute.strip_prefix(parent).ok().map(Path::to_owned);
    Reference::External { absolute, relative }
}

fn write_archive(path:&Path, document:&Document, files:&[(String,PathBuf)]) -> Result<(),String> {
    let file=fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|e|e.to_string())?;
    let mut archive=zip::ZipWriter::new(file);
    let options=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    archive.start_file("project.json",options).map_err(|e|e.to_string())?;
    archive.write_all(&serde_json::to_vec_pretty(document).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    for (name,path) in files {
        archive.start_file(name,options).map_err(|e|e.to_string())?;
        std::io::copy(&mut fs::File::open(path).map_err(|e|e.to_string())?,&mut archive).map_err(|e|e.to_string())?;
    }
    archive.finish().map_err(|e|e.to_string())?.sync_all().map_err(|e|e.to_string())
}

fn resolve(reference:&Reference, project:&Path, archive:&mut zip::ZipArchive<fs::File>, assets:&Assets, next:&mut usize) -> Result<PathBuf,String> {
    match reference {
        Reference::External {absolute,relative} => {
            let local=relative.as_ref().filter(|p|!p.is_absolute() && p.components().all(|c|matches!(c,std::path::Component::Normal(_))))
                .map(|p|project.parent().unwrap_or(Path::new(".")).join(p));
            let path=local.filter(|p|p.is_file()).unwrap_or_else(||absolute.clone());
            if !path.is_file() { return Err(format!("{}: {}",tr("工程素材不存在，请恢复原路径后重新打开。"),path.display())); }
            Ok(path)
        }
        Reference::Embedded {entry} => {
            if !entry.starts_with("assets/") { return Err(tr("工程设置无效或格式版本不受支持。").into()); }
            let mut input=archive.by_name(entry).map_err(|e|e.to_string())?;
            if input.size()>64*1024*1024*1024 {return Err(tr("工程设置无效或格式版本不受支持。").into());}
            let extension=Path::new(entry).extension().and_then(|s|s.to_str()).filter(|s|s.chars().all(|c|c.is_ascii_alphanumeric())).unwrap_or("bin");
            let path=assets.0.join(format!("{}.{extension}",*next)); *next+=1;
            let mut output=fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
            std::io::copy(&mut input,&mut output).map_err(|e|e.to_string())?;
            Ok(path)
        }
    }
}

fn load(path:PathBuf) -> Result<Loaded,String> {
    let mut archive=zip::ZipArchive::new(fs::File::open(&path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let document:Document={
        let mut entry=archive.by_name("project.json").map_err(|e|e.to_string())?;
        if entry.size()>8*1024*1024 {return Err(tr("工程设置无效或格式版本不受支持。").into());}
        let mut bytes=Vec::new();entry.read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e|e.to_string())?
    };
    let video=match document.kind.as_str() {"video"=>true,"audio"=>false,_=>return Err(tr("工程设置无效或格式版本不受支持。").into())};
    if document.format!="UnityYAMLViewer.MediaProject" || document.version!=1 || document.items.is_empty()
        || document.items.len()>10000 || document.active>=document.items.len()
        || !path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case(if video {"uyvp"}else{"uyap"}))
    {return Err(tr("工程设置无效或格式版本不受支持。").into());}
    document.merge_settings.validate_project()?;
    let root=std::env::temp_dir().join(format!("unity-media-project-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
    fs::create_dir(&root).map_err(|e|e.to_string())?;
    let assets=Arc::new(Assets(root));let mut next=0;let mut media=Vec::new();
    let mut document=document;
    for item in &mut document.items {
        item.state.validate()?;
        if !item.position.is_finite() || item.position<0.0 || item.tracks.len()!=item.state.tracks.len() {return Err(tr("工程设置无效或格式版本不受支持。").into());}
        let source=resolve(&item.source,&path,&mut archive,&assets,&mut next)?;
        let info=probe(&source,None,None,false)?;
        if info.video.is_some()!=video {return Err(tr("工程素材类型不匹配。").into());}
        for (track,reference) in item.state.tracks.iter_mut().zip(&item.tracks) {track.path=resolve(reference,&path,&mut archive,&assets,&mut next)?;}
        let clip=if let Some(reference)=&item.clip {
            let clip=resolve(reference,&path,&mut archive,&assets,&mut next)?;
            let duration=probe(&clip,None,None,false)?.duration;
            Some((clip,duration))
        }else{None};
        if !video && (item.state.audio_state.is_none() || clip.is_none()) {return Err(tr("工程中的音频编辑数据缺失。").into());}
        let thumbnail=if video {
            command(&info.ffmpeg).args(["-nostdin","-v","error","-ss",&item.position.min(info.duration).to_string(),"-i"]).arg(&source)
                .args(["-map","0:V:0","-frames:v","1","-an","-vf","scale=240:240:force_original_aspect_ratio=decrease","-c:v","png","-f","image2pipe","pipe:1"])
                .output().ok().filter(|output|output.status.success()).and_then(|output|image::load_from_memory(&output.stdout).ok())
                .map(|image|{let image=image.to_rgba8();egui::ColorImage::from_rgba_unmultiplied([image.width() as usize,image.height() as usize],image.as_raw())})
        }else{None};
        media.push((source,info,clip,thumbnail));
    }
    Ok(Loaded {path,document,media,assets})
}

impl Loaded {
    fn editor(self,ctx:&egui::Context) -> Result<(PathBuf,TrimEditor),String> {
        let mut players=Vec::new();let mut queue=crate::media_edit_queue::Panel::default();
        queue.active=self.document.active;queue.expanded=self.document.expanded;queue.merge=self.document.merge;
        queue.merge_settings=self.document.merge_settings;queue.project_path=Some(self.path.clone());
        for (item,(path,info,clip,thumbnail)) in self.document.items.into_iter().zip(self.media) {
            let mut player=Preview::default();player.trim.open(info.duration);
            player.trim.restore_project(item.state,clip.as_ref().map(|(p,d)|(p.as_path(),*d)),info.ffmpeg.clone(),info.source_channels as usize)?;
            player.position=item.position.min(clip.as_ref().map_or(info.duration,|(_,d)|*d));
            player.selected_audio=item.selected_audio.filter(|index|info.audio_tracks.iter().any(|track|track.index==*index));
            player.info=Some(info);
            let thumbnail=thumbnail.map(|image|ctx.load_texture(format!("project-{}",path.display()),image,egui::TextureOptions::LINEAR));
            queue.items.push(crate::media_edit_queue::Item{path:path.clone(),thumbnail,enabled:item.enabled});players.push((path,player));
        }
        let mut editor=TrimEditor {players,queue,pending:None,project_assets:Some(self.assets),project_save:None,saved_edit_state:None,saving_edit_state:None,confirm_discard:false,removed_items:vec![]};
        editor.saved_edit_state=editor.edit_fingerprint();
        Ok((self.path,editor))
    }
}

pub(super) fn pump(ctx:&egui::Context) {
    OPENS.with(|opens| {
        let mut opens=opens.borrow_mut();
        for path in std::mem::take(&mut opens.requests) {
            let existing=TRIM_EDITORS.with(|editors|editors.borrow().iter().find(|(root,editor)|**root==path || editor.queue.project_path.as_ref()==Some(&path)).map(|(root,_)|root.clone()));
            if let Some(root)=existing {crate::native_window::wake(ctx,egui::Id::new(("media_trim",root)));continue;}
            let (tx,rx)=mpsc::channel();opens.jobs.push(rx);let ctx=ctx.clone();
            std::thread::spawn(move||{let _=tx.send(load(path));ctx.request_repaint_of(egui::ViewportId::ROOT);});
        }
        let mut ready=Vec::new();
        opens.jobs.retain(|job|match job.try_recv(){Ok(result)=>{ready.push(result);false},Err(mpsc::TryRecvError::Empty)=>true,Err(_)=>false});
        for result in ready {
            match result.and_then(|loaded|loaded.editor(ctx)) {
                Ok((path,editor))=>{
                    let root=TRIM_EDITORS.with(|editors|{
                        let mut editors=editors.borrow_mut();
                        if let Some((root,_))=editors.iter().find(|(root,current)|**root==path || current.queue.project_path.as_ref()==Some(&path)) { return root.clone(); }
                        editors.insert(path.clone(),editor);path
                    });
                    crate::native_window::wake(ctx,egui::Id::new(("media_trim",root)));
                }
                Err(error)=>opens.error=Some(error),
            }
        }
        if !opens.jobs.is_empty(){ctx.request_repaint_after(Duration::from_millis(100));}
        if let Some(error)=opens.error.clone() {
            let mut open=true;
            crate::native_window::Window::new(tr("打开工程失败")).id(egui::Id::new("media_project_error")).open(&mut open).show(ctx,|ui|{ui.colored_label(ui.visuals().error_fg_color,error);});
            if !open {opens.error=None;}
        }
    });
}

impl TrimEditor {
    pub(super) fn edit_fingerprint(&self) -> Option<Vec<u8>> {
        let items=self.players.iter().zip(&self.queue.items).map(|((path,player),item)| {
            let state=player.trim.project_state(player.has_video()).ok()?;
            Some(serde_json::json!({"path":path,"enabled":item.enabled,"state":state,
                "clip":player.trim.clip().map(|clip|clip.path.clone())}))
        }).collect::<Option<Vec<_>>>()?;
        serde_json::to_vec(&serde_json::json!({"items":items,"settings":self.queue.merge_settings})).ok()
    }
    pub(super) fn poll_project_save(&mut self,ctx:&egui::Context) {
        if let Some(receiver)=&self.project_save {
            match receiver.try_recv() {
                Ok(result)=>{self.project_save=None;match result {Ok(path)=>{self.queue.project_path=Some(path);self.queue.error=None;self.saved_edit_state=self.saving_edit_state.take();},Err(error)=>{self.saving_edit_state=None;self.queue.error=Some(error);}}},
                Err(mpsc::TryRecvError::Disconnected)=>{self.project_save=None;self.queue.error=Some(tr("工程保存失败。").into());},
                Err(mpsc::TryRecvError::Empty)=>ctx.request_repaint_after(Duration::from_millis(100)),
            }
        }
        self.queue.project_saving=self.project_save.is_some();
    }
    pub(super) fn save_project(&mut self,ctx:&egui::Context) {
        if self.project_save.is_some() || self.queue.loading || self.pending.is_some() || !self.queue.add.is_empty() {self.queue.error=Some(tr("请等待编辑处理完成并关闭参数弹窗后保存工程。").into());return;}
        let video=self.players[0].1.has_video();let extension=if video {"uyvp"}else{"uyap"};
        let states=self.players.iter().map(|(_,player)|player.trim.project_state(video)).collect::<Result<Vec<_>,_>>();
        let states=match states {Ok(states)=>states,Err(error)=>{self.queue.error=Some(error);return;}};
        let target=if !std::mem::take(&mut self.queue.project_save_as) {self.queue.project_path.clone()}else{None};
        let target=target.or_else(||{
            let name=self.queue.project_path.as_ref().unwrap_or(&self.players[0].0).with_extension(extension);
            rfd::FileDialog::new().add_filter(tr("编辑工程"),&[extension]).set_file_name(name.file_name().unwrap_or_default().to_string_lossy()).set_directory(name.parent().unwrap_or(Path::new("."))).save_file()
        });
        let Some(target)=target else{return};let target=target.with_extension(extension);
        let mut files=Vec::new();let mut items=Vec::new();let mut clips=Vec::new();let mut sequences=Vec::new();
        let parent=target.parent().unwrap_or(Path::new("."));
        for (((path,player),state),queue_item) in self.players.iter().zip(states).zip(&self.queue.items) {
            let source=if let Some(sequence)=player.trim.sequence_source() {
                let entry=format!("assets/source-{}.mkv",items.len());files.push((entry.clone(),sequence.path.clone()));sequences.push(sequence);Reference::Embedded{entry}
            }else if self.project_assets.as_ref().is_some_and(|assets|path.starts_with(&assets.0)) {
                let entry=format!("assets/source-{}.mkv",items.len());files.push((entry.clone(),path.clone()));Reference::Embedded{entry}
            }else{external(path,parent)};
            let tracks=state.tracks.iter().enumerate().map(|(index,track)| {
                if track.path==*path {source.clone()}
                else if self.project_assets.as_ref().is_some_and(|assets|track.path.starts_with(&assets.0)) {
                    let extension=track.path.extension().and_then(|s|s.to_str()).unwrap_or("mkv");
                    let entry=format!("assets/track-{}-{index}.{extension}",items.len());
                    files.push((entry.clone(),track.path.clone()));Reference::Embedded{entry}
                }else{external(&track.path,parent)}
            }).collect();
            let clip=player.trim.clip().map(|clip|{let entry=format!("assets/audio-{}.wav",items.len());files.push((entry.clone(),clip.path.clone()));clips.push(clip);Reference::Embedded{entry}});
            items.push(Item{enabled:queue_item.enabled,source,state,tracks,clip,position:player.position_now(),selected_audio:player.selected_audio});
        }
        if self.players.iter().any(|(path,_)|*path==target) || files.iter().any(|(_,path)|*path==target) {self.queue.error=Some(tr("工程不能覆盖素材文件。").into());return;}
        let document=Document {format:"UnityYAMLViewer.MediaProject".into(),version:1,kind:if video{"video"}else{"audio"}.into(),active:self.queue.active,expanded:self.queue.expanded,merge:self.queue.merge,merge_settings:self.queue.merge_settings.clone(),items};
        self.saving_edit_state=self.edit_fingerprint();
        let (tx,rx)=mpsc::channel();self.project_save=Some(rx);self.queue.project_saving=true;
        use crate::background_tasks::{Task,Progress,Outcome,Target};
        let task=Task::start(ctx,"保存工程",target.file_name().unwrap_or_default().to_string_lossy().into_owned(),||Progress{stage:tr("正在保存工程…").into(),fraction:None,cancellable:false},None);
        let assets=self.project_assets.clone();
        std::thread::spawn(move||{
            let _keep=(clips,sequences,assets);
            let temporary=target.with_file_name(format!(".unity-project-{}-{}.tmp",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
            let result=write_archive(&temporary,&document,&files).and_then(|()|crate::playlist::replace(&temporary,&target)).map(|()|target);
            if result.is_err(){let _=fs::remove_file(&temporary);}
            task.finish(match &result {Ok(path)=>Outcome::Complete(Some(Target::File(path.clone()))),Err(error)=>Outcome::Failed(error.clone())});
            let _=tx.send(result);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        Document{format:"UnityYAMLViewer.MediaProject".into(),version:1,kind:"audio".into(),active:0,expanded:true,merge:false,merge_settings:MergeSettings::default(),items:vec![Item{enabled:true,
            source:Reference::External{absolute:PathBuf::from("missing.wav"),relative:None},state:EditState::fixture(),tracks:vec![],clip:None,position:12.0,selected_audio:Some(0)}]}
    }
    #[test]
    fn archive_preserves_manifest_and_embedded_bytes() {
        let root=std::env::temp_dir().join(format!("unity-project-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();let _cleanup=Assets(root.clone());
        let source=root.join("source.wav");fs::write(&source,b"edited audio fixture").unwrap();
        let target=root.join("test.uyap");
        write_archive(&target,&document(),&[("assets/audio.wav".into(),source)]).unwrap();
        let mut archive=zip::ZipArchive::new(fs::File::open(&target).unwrap()).unwrap();
        let restored:Document=serde_json::from_reader(archive.by_name("project.json").unwrap()).unwrap();
        assert_eq!(restored.kind,"audio");assert_eq!(restored.items[0].position,12.0);
        let mut bytes=Vec::new();archive.by_name("assets/audio.wav").unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes,b"edited audio fixture");
    }
    #[test]
    fn unknown_version_and_missing_reference_are_explicit_errors() {
        let root=std::env::temp_dir().join(format!("unity-project-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();let _cleanup=Assets(root.clone());let target=root.join("test.uyap");
        let mut doc=document();doc.version=999;write_archive(&target,&doc,&[]).unwrap();
        assert!(load(target).is_err());
        let target=root.join("missing.uyap");doc.version=1;write_archive(&target,&doc,&[]).unwrap();
        assert!(load(target).err().unwrap().contains("missing.wav"));
    }
}
