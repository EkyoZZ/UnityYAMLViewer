//! Optional package managers: changing selection never downloads or installs anything.
use std::{fs,path::PathBuf,sync::{mpsc,atomic::{AtomicBool,Ordering}},process::Stdio,time::{Duration,Instant}};
use eframe::egui;
use crate::localization::tr;
use super::install_progress::Shared;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub(crate) enum Backend {#[default] WinGet,Scoop}
impl Backend {pub(crate) fn name(self)->&'static str{match self{Self::WinGet=>"WinGet",Self::Scoop=>"Scoop"}}}
static SELECTED:std::sync::LazyLock<std::sync::Mutex<Backend>>=std::sync::LazyLock::new(||std::sync::Mutex::new(read_backend()));
static SCOOP_READY:AtomicBool=AtomicBool::new(false);
fn config()->Result<PathBuf,String>{Ok(super::component_root()?.join("installer.json"))}
fn read_backend()->Backend{config().ok().and_then(|p|fs::read(p).ok()).and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default()}
pub(crate) fn selected()->Backend{SELECTED.lock().map(|b|*b).unwrap_or_default()}
pub(crate) fn ready()->bool{selected()==Backend::WinGet||SCOOP_READY.load(Ordering::Relaxed)}
pub(crate) fn roots()->Vec<PathBuf>{
    let mut roots=Vec::new();
    for name in ["SCOOP","SCOOP_GLOBAL"]{if let Some(root)=std::env::var_os(name){roots.push(root.into());}}
    if let Some(home)=std::env::var_os("USERPROFILE"){roots.push(PathBuf::from(home).join("scoop"));}
    if let Some(data)=std::env::var_os("ProgramData"){roots.push(PathBuf::from(data).join("scoop"));}
    let mut paths:Vec<std::ffi::OsString>=std::env::var_os("PATH").into_iter().collect();
    #[cfg(windows)] paths.extend(crate::media_tools::fresh_windows_paths());
    for path in paths{for dir in std::env::split_paths(&path){if dir.file_name().is_some_and(|n|n.eq_ignore_ascii_case("shims")){if let Some(root)=dir.parent(){roots.push(root.to_owned());}}}}
    roots.dedup();roots
}
pub(crate) fn tool_directories(name:&str)->Vec<PathBuf>{
    roots().into_iter().map(|root|match name{"libreoffice"=>root.join("apps/libreoffice/current/LibreOffice/program"),_=>root.join("apps/ffmpeg/current/bin")}).collect()
}
fn script()->Option<PathBuf>{roots().into_iter().map(|r|r.join("apps/scoop/current/bin/scoop.ps1")).find(|p|p.is_file())}
fn powershell()->std::process::Command{
    let mut command=crate::media_tools::command("powershell.exe");
    command.args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass"]);command
}
fn run(mut command:std::process::Command,progress:&Shared,limit:Option<Duration>)->Result<(),String>{
    let mut child=command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
    if let Some(stdout)=child.stdout.take(){super::install_progress::capture(stdout,progress.clone());}
    if let Some(stderr)=child.stderr.take(){super::install_progress::capture(stderr,progress.clone());}
    let started=Instant::now();loop{match child.try_wait(){
        Ok(Some(status))=>{if status.success(){return Ok(());}return Err(format!("Scoop ({status})\n{}",progress.details()));},
        Ok(None)=>{},Err(e)=>{
            // Preserve a potentially running installer; retain the lock by continuing to monitor.
            if limit.is_some(){let _=child.kill();let _=child.wait();return Err(e.to_string());}
            progress.slow();std::thread::sleep(Duration::from_millis(500));continue;
        }
    }
    if limit.is_some_and(|d|started.elapsed()>d){let _=child.kill();let _=child.wait();return Err(tr("Scoop 检测超时，请重新检测或检查安装目录。").into());}
    if started.elapsed()>Duration::from_secs(900){progress.slow();}
    std::thread::sleep(Duration::from_millis(100));}
}
fn detect(progress:&Shared)->Result<Option<PathBuf>,String>{
    let Some(script)=script()else{return Ok(None);};let mut command=powershell();command.arg("-File").arg(&script).arg("--version");
    run(command,progress,Some(Duration::from_secs(15)))?;Ok(Some(script))
}
struct Staging(PathBuf);
impl Drop for Staging{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn bootstrap(progress:&Shared)->Result<Option<PathBuf>,String>{
    let _guard=super::install_task::Guard::acquire()?;super::install_task::ensure_no_other_installer(None)?;
    if let Some(found)=detect(progress)?{return Ok(Some(found));}
    progress.phase(2);
    let mut response=super::http_client()?.get("https://get.scoop.sh").send().and_then(|r|r.error_for_status()).map_err(|e|e.to_string())?;
    let url=response.url();if !matches!(url.host_str(),Some("get.scoop.sh"|"raw.githubusercontent.com")){return Err(tr("Scoop 官方安装脚本地址无效").into());}
    use std::io::Read;
    let mut bytes=Vec::new();response.by_ref().take(1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.is_empty()||bytes.len()>1024*1024{return Err(tr("Scoop 官方安装脚本大小无效").into());}
    let directory=super::component_root()?.join(format!("scoop-bootstrap-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    fs::create_dir_all(&directory).map_err(|e|e.to_string())?;let staging=Staging(directory);
    let path=staging.0.join("scoop-bootstrap.ps1");fs::write(&path,bytes).map_err(|e|e.to_string())?;
    progress.phase(4);let mut command=powershell();command.arg("-File").arg(path);run(command,progress,None)?;
    progress.phase(5);detect(progress)?.map(Some).ok_or_else(||tr("Scoop 安装后尚未检测到，请检查安装信息后重新检测。").to_owned())
}
pub(crate) fn install(component:&str,progress:&Shared)->Result<(),String>{
    let _guard=super::install_task::Guard::acquire()?;super::install_task::ensure_no_other_installer(None)?;
    let script=script().ok_or_else(||tr("请先安装并检测 Scoop，再安装第三方组件。").to_owned())?;
    // All script text is fixed. Paths and component names are data in environment variables.
    let body=match component{
        "ffmpeg"=>"& $env:UYV_SCOOP_SCRIPT install main/ffmpeg; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}",
        "libreoffice"=>"if(-not (Test-Path -LiteralPath $env:UYV_SCOOP_EXTRAS)){if(-not (Get-Command git -ErrorAction SilentlyContinue)){& $env:UYV_SCOOP_SCRIPT install main/git; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}}; & $env:UYV_SCOOP_SCRIPT bucket add extras; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}}; & $env:UYV_SCOOP_SCRIPT install extras/libreoffice; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}",
        _=>return Err(tr("不支持的第三方组件").into()),
    };
    let root=script.ancestors().nth(5).ok_or_else(||tr("Scoop 安装路径无效").to_owned())?;
    let mut command=powershell();command.arg("-Command").arg(format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $env:PATH=$env:UYV_SCOOP_SHIMS+';'+$env:PATH; {body}"));
    command.env("UYV_SCOOP_SCRIPT",&script).env("UYV_SCOOP_EXTRAS",root.join("buckets/extras")).env("UYV_SCOOP_SHIMS",root.join("shims"));
    progress.phase(1);run(command,progress,None)
}
pub(crate) fn install_ffmpeg_winget()->Result<(),String>{
    let _guard=super::install_task::Guard::acquire()?;super::install_task::ensure_no_other_installer(None)?;
    let output=crate::media_tools::command("winget").args(["install","--id","Gyan.FFmpeg","--exact","--source","winget","--scope","user","--silent","--disable-interactivity","--accept-package-agreements","--accept-source-agreements"]).output().map_err(|e|format!("WinGet: {e}"))?;
    if !output.status.success(){return Err(format!("WinGet ({})\n{}\n{}",output.status,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr)));}Ok(())
}
#[derive(Default)]
pub(crate) struct Settings {checked:bool,path:Option<PathBuf>,pending:Option<mpsc::Receiver<Result<Option<PathBuf>,String>>>,installing:bool,error:Option<String>,progress:Shared}
impl Settings {
    pub(crate) fn busy(&self)->bool{self.pending.is_some()}
    fn start(&mut self,ctx:&egui::Context,install:bool){
        if self.busy(){return;}self.error=None;self.installing=install;self.progress=Shared::default();let progress=self.progress.clone();
        let (send,receive)=mpsc::channel();self.pending=Some(receive);let ctx=ctx.clone();
        std::thread::spawn(move||{let result=if install{bootstrap(&progress)}else{detect(&progress)};let _=send.send(result);ctx.request_repaint();});
    }
    pub(crate) fn show(&mut self,ui:&mut egui::Ui,busy:bool){
        if let Some(receiver)=&self.pending{match receiver.try_recv(){
            Ok(result)=>{self.pending=None;self.checked=true;self.installing=false;match result{Ok(path)=>{SCOOP_READY.store(path.is_some(),Ordering::Relaxed);self.path=path;},Err(e)=>{SCOOP_READY.store(false,Ordering::Relaxed);self.error=Some(e);}}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.checked=true;self.installing=false;SCOOP_READY.store(false,Ordering::Relaxed);self.error=Some(tr("组件任务中断").into());},_=>{}
        }}
        let old=selected();let mut choice=old;
        ui.horizontal(|ui|{
            ui.add(egui::Label::new(tr("第三方支持软件安装器:")).wrap_mode(egui::TextWrapMode::Extend));
            ui.add_enabled_ui(!busy&&!self.busy(),|ui|{
                let mut tab=choice.name().to_owned();
                crate::theme::compact_selector(ui,&mut tab,[("WinGet","WinGet"),("Scoop","Scoop")]);
                choice=if tab=="Scoop"{Backend::Scoop}else{Backend::WinGet};
            });
            if choice!=old{
                let saved=(||{let path=config()?;fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;fs::write(path,serde_json::to_vec(&choice).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;*SELECTED.lock().map_err(|e|e.to_string())?=choice;Ok::<_,String>(())})();
                if let Err(e)=saved{self.error=Some(e);}else{self.error=None;if choice==Backend::Scoop{self.checked=false;SCOOP_READY.store(false,Ordering::Relaxed);}}
            }
            if selected()==Backend::Scoop{
                if !self.checked&&!self.busy(){self.start(ui.ctx(),false);}
                if self.busy(){ui.spinner();ui.label(tr(if self.installing{"正在安装 Scoop…"}else{"正在检测…"}));}
                else{
                    ui.label(tr(if SCOOP_READY.load(Ordering::Relaxed){"Scoop 已安装"}else if self.error.is_some(){"Scoop 检测失败"}else{"Scoop 未安装"}));
                    if !SCOOP_READY.load(Ordering::Relaxed)&&ui.add_enabled(!busy&&cfg!(windows),egui::Button::new(tr("安装 Scoop"))).clicked(){self.start(ui.ctx(),true);}
                    if ui.add_enabled(!busy,egui::Button::new(tr("重新检测"))).clicked(){self.start(ui.ctx(),false);}
                }
            }
        });
        if selected()==Backend::Scoop{
            ui.small(tr("Scoop 独立安装，不随程序附带；仅点击安装才下载官方脚本。组件由 Scoop 管理，不属于软件缓存。"));
            if self.busy(){if self.installing{self.progress.show_for(ui,"Scoop");}ui.ctx().request_repaint_after(Duration::from_millis(100));}
            if let Some(path)=&self.path{ui.add(egui::Label::new(path.display().to_string()).wrap().selectable(true));}
            ui.small(tr("Scoop 按当前用户安装；安装可能受系统策略或管理员运行方式限制。"));
            ui.hyperlink_to(tr("组件来源与许可"),"https://scoop.sh/");
        }
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().warn_fg_color,error);}
        ui.separator();
    }
}
