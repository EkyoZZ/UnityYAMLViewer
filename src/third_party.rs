//! Optional components live outside disposable cache roots. Nothing is downloaded on startup.
use std::{fs, io::{Read, Write}, path::{Path,PathBuf}, sync::{Arc,mpsc,atomic::{AtomicBool,AtomicU64,Ordering}}, time::Duration};
use eframe::egui;
use serde::{Serialize,Deserialize};
use sha2::{Digest,Sha256};
use crate::localization::tr;
pub(crate) mod install_progress;
pub(crate) mod installer;
mod install_task;
mod yt_dlp;
/// One entry point for missing-component prompts, including detached preview windows.
pub fn request_open(ctx:&egui::Context) {
    ctx.data_mut(|data|data.insert_temp(egui::Id::new("third_party_manager_request"),true));
    ctx.request_repaint();
}
pub fn take_open_request(ctx:&egui::Context)->bool {
    ctx.data_mut(|data|data.remove_temp::<bool>(egui::Id::new("third_party_manager_request"))).unwrap_or(false)
}
pub(crate) fn component_header_actions(ui:&mut egui::Ui,name:&str,installed:Option<bool>,busy:bool,install:Option<(&str,bool)>,extra:impl FnOnce(&mut egui::Ui))->(bool,bool){
    ui.horizontal_wrapped(|ui|{
        crate::theme::section_heading(ui, name);
        if let Some(installed)=installed{
            let color=if installed{if ui.visuals().dark_mode{egui::Color32::from_rgb(80,210,120)}else{egui::Color32::from_rgb(20,125,55)}}else{if ui.visuals().dark_mode{egui::Color32::from_rgb(245,100,100)}else{egui::Color32::from_rgb(185,35,35)}};
            egui::Frame::new().stroke(egui::Stroke::new(1.0_f32,color)).corner_radius(3).inner_margin(egui::Margin::symmetric(6,2)).show(ui,|ui|{
                ui.label(egui::RichText::new(tr(if installed{"已安装"}else{"未安装"})).color(color));
            });
        }
        let mut install_clicked=false;
        if let Some((label,enabled))=install{
            install_clicked=ui.add_enabled(!busy&&enabled,crate::theme::green_outline_button(ui,tr(label))).clicked();
        }
        let recheck=ui.add_enabled(!busy,egui::Button::new(tr("重新检测"))).clicked();
        extra(ui);(recheck,install_clicked)
    }).inner
}

const RELEASE_API:&str="https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest";
const DOWNLOAD_PREFIX:&str="https://github.com/bblanchon/pdfium-binaries/releases/download/";
const DOWNLOAD_LIMIT:u64=64*1024*1024;
fn component_root()->Result<PathBuf,String> {
    crate::app_cache::root().and_then(|path|path.parent().map(|parent|parent.join("UnityYAMLViewer-components"))).ok_or_else(||tr("无法确定组件目录").to_owned())
}
fn valid_folder(folder:&str)->bool {folder.starts_with("pdfium-") && folder.len()<160 && folder.bytes().all(|value|value.is_ascii_alphanumeric()||value==b'-')}
#[derive(Clone,Serialize,Deserialize)]
struct Installed {folder:String,version:String,dll_sha256:String}
impl Installed {
    fn path(&self)->Result<PathBuf,String>{if !valid_folder(&self.folder){return Err(tr("组件记录无效").into());}Ok(component_root()?.join(&self.folder).join("pdfium.dll"))}
}
fn manifest()->Result<PathBuf,String>{Ok(component_root()?.join("pdfium.json"))}
fn read_installed()->Result<Option<Installed>,String>{
    let path=manifest()?;
    if !path.exists(){return Ok(None);}
    let installed:Installed=serde_json::from_slice(&fs::read(path).map_err(|error|error.to_string())?).map_err(|error|error.to_string())?;
    let path=installed.path()?;
    let mut bytes=Vec::new();fs::File::open(&path).map_err(|error|error.to_string())?.take(64*1024*1024+1).read_to_end(&mut bytes).map_err(|error|error.to_string())?;
    if bytes.len()>64*1024*1024 || format!("{:x}",Sha256::digest(&bytes))!=installed.dll_sha256{return Err(tr("PDFium 文件校验失败，请移除后重新下载。").into());}
    validate_library(&path)?;Ok(Some(installed))
}
pub(crate) fn verified_pdfium_path()->Result<PathBuf,String>{
    if let Some(path)=crate::component_paths::selected("pdfium") {validate_library(&path)?;return Ok(path)}
    read_installed()?.ok_or_else(||tr("请先在第三方支持中下载 PDFium。" ).to_owned())?.path()
}
pub(crate) fn validate_library(path:&Path)->Result<(),String>{
    // Managed files are hash-checked; manual files are explicitly selected by the user.
    let library=unsafe{libloading::Library::new(path)}.map_err(|error|error.to_string())?;
    for name in [b"FPDF_InitLibrary\0".as_slice(),b"FPDF_DestroyLibrary\0",b"FPDF_LoadDocument\0",b"FPDF_RenderPageBitmap\0",b"FPDF_SaveAsCopy\0"] {
        unsafe{library.get::<unsafe extern "C" fn()>(name)}.map_err(|error|error.to_string())?;
    }Ok(())
}
fn http_client()->Result<reqwest::blocking::Client,String>{
    reqwest::blocking::Client::builder().user_agent("UnityYAMLViewer/third-party")
        .connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(300)).build().map_err(|error|error.to_string())
}
fn extract(package:&Path,entry:&str,limit:usize)->Result<Vec<u8>,String>{
    let output=crate::media_tools::command("tar").args(["-xOf"]).arg(package).arg("--").arg(entry).output().map_err(|error|format!("tar: {error}"))?;
    if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).into_owned());}
    if output.stdout.len()>limit{return Err(tr("组件解压大小超限").into());}Ok(output.stdout)
}
fn remove_directory(folder:&str)->Result<(),String>{
    if !valid_folder(folder){return Err(tr("组件记录无效").into());}
    let root=component_root()?;let path=root.join(folder);
    if !path.exists(){return Ok(());}
    let boundary=fs::canonicalize(&root).map_err(|error|error.to_string())?;
    let resolved=fs::canonicalize(&path).map_err(|error|error.to_string())?;
    if resolved.parent()!=Some(boundary.as_path()){return Err(tr("组件目录不在管理范围内").into());}
    fs::remove_dir_all(path).map_err(|error|error.to_string())
}
fn download(cancel:&AtomicBool,done:&AtomicU64,total:&AtomicU64)->Result<Installed,String>{
    if !cfg!(all(windows,target_arch="x86_64")){return Err(tr("PDFium 自动下载目前仅支持 Windows x64。").into());}
    let client=http_client()?;
    let mut response=client.get(RELEASE_API).send().and_then(|response|response.error_for_status()).map_err(|error|error.to_string())?;
    let mut metadata=Vec::new();response.by_ref().take(4*1024*1024+1).read_to_end(&mut metadata).map_err(|error|error.to_string())?;
    if metadata.len()>4*1024*1024{return Err(tr("组件发行信息过大").into());}
    let release:serde_json::Value=serde_json::from_slice(&metadata).map_err(|error|error.to_string())?;
    let asset=release["assets"].as_array().and_then(|assets|assets.iter().find(|asset|asset["name"]=="pdfium-win-x64.tgz")).ok_or_else(||tr("未找到匹配的 PDFium 下载包").to_owned())?;
    let url=asset["browser_download_url"].as_str().filter(|url|url.starts_with(DOWNLOAD_PREFIX)).ok_or_else(||tr("组件下载地址无效").to_owned())?;
    let expected=asset["digest"].as_str().and_then(|digest|digest.strip_prefix("sha256:")).filter(|digest|digest.len()==64&&digest.bytes().all(|value|value.is_ascii_hexdigit())).ok_or_else(||tr("组件缺少 SHA-256 校验值").to_owned())?;
    let size=asset["size"].as_u64().filter(|size|*size>0&&*size<=DOWNLOAD_LIMIT).ok_or_else(||tr("组件下载大小无效").to_owned())?;
    total.store(size,Ordering::Relaxed);
    let root=component_root()?;fs::create_dir_all(&root).map_err(|error|error.to_string())?;
    let folder=format!("pdfium-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    let directory=root.join(&folder);fs::create_dir(&directory).map_err(|error|error.to_string())?;
    let result=(|| {
        let package=directory.join("download.tgz");let mut file=fs::File::create(&package).map_err(|error|error.to_string())?;
        let mut response=client.get(url).send().and_then(|response|response.error_for_status()).map_err(|error|error.to_string())?;
        let mut hash=Sha256::new();let mut received=0;let mut buffer=[0;65536];
        loop {
            if cancel.load(Ordering::Relaxed){return Err(tr("下载已取消。").into());}
            let count=response.read(&mut buffer).map_err(|error|error.to_string())?;if count==0{break;}
            received+=count as u64;if received>size{return Err(tr("组件下载大小无效").into());}
            file.write_all(&buffer[..count]).map_err(|error|error.to_string())?;hash.update(&buffer[..count]);done.store(received,Ordering::Relaxed);
        }
        file.sync_all().map_err(|error|error.to_string())?;drop(file);
        if received!=size || !format!("{:x}",hash.finalize()).eq_ignore_ascii_case(expected){return Err(tr("PDFium 文件校验失败，请移除后重新下载。").into());}
        let listing=crate::media_tools::command("tar").arg("-tf").arg(&package).output().map_err(|error|format!("tar: {error}"))?;
        if !listing.status.success(){return Err(String::from_utf8_lossy(&listing.stderr).into_owned());}
        let text=String::from_utf8(listing.stdout).map_err(|error|error.to_string())?;
        let entries:Vec<_>=text.lines().collect();
        let dlls:Vec<_>=entries.iter().copied().filter(|entry|entry.trim_start_matches("./")=="bin/pdfium.dll").collect();
        if dlls.len()!=1{return Err(tr("组件压缩包内容无效").into());}
        let dll=extract(&package,dlls[0],64*1024*1024)?;
        let dll_path=directory.join("pdfium.dll");fs::write(&dll_path,&dll).map_err(|error|error.to_string())?;
        // Copy license texts as flat files; never extract archive paths onto the filesystem.
        let license_dir=directory.join("licenses");fs::create_dir(&license_dir).map_err(|error|error.to_string())?;
        let mut licenses=0;
        for (index,entry) in entries.iter().filter(|entry|{let name=entry.to_ascii_lowercase();!entry.ends_with('/')&&(name.contains("license")||name.contains("notice"))}).take(128).enumerate(){
            let bytes=extract(&package,entry,2*1024*1024)?;fs::write(license_dir.join(format!("{index}.txt")),bytes).map_err(|error|error.to_string())?;licenses+=1;
        }
        if licenses==0{return Err(tr("组件压缩包缺少许可文件").into());}
        if cancel.load(Ordering::Relaxed){return Err(tr("下载已取消。").into());}
        validate_library(&dll_path)?;
        let installed=Installed{folder:folder.clone(),version:release["tag_name"].as_str().unwrap_or("PDFium").to_owned(),dll_sha256:format!("{:x}",Sha256::digest(&dll))};
        fs::remove_file(package).map_err(|error|error.to_string())?;
        let temporary=root.join("pdfium.json.tmp");fs::write(&temporary,serde_json::to_vec(&installed).map_err(|error|error.to_string())?).map_err(|error|error.to_string())?;
        fs::rename(temporary,manifest()?).map_err(|error|error.to_string())?;Ok(installed)
    })();
    if result.is_err(){let _=remove_directory(&folder);}result.map_err(|error|format!("{error}\n{RELEASE_API}\n{url}"))
}
#[derive(Default)]
struct PdfiumSettings {
    picker:crate::component_paths::Picker,
    available:Option<bool>,checked:bool,installed:Option<Installed>,pending:Option<mpsc::Receiver<Result<Option<Installed>,String>>>,
    ready_event:bool,installing:bool,error:Option<String>,cancel:Arc<AtomicBool>,done:Arc<AtomicU64>,total:Arc<AtomicU64>,
}
impl PdfiumSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool){
        if self.pending.is_some(){return;}self.error=None;self.installing=install;self.cancel.store(false,Ordering::Relaxed);self.done.store(0,Ordering::Relaxed);self.total.store(0,Ordering::Relaxed);
        let cancel=self.cancel.clone();let done=self.done.clone();let total=self.total.clone();let ctx=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some(receive);
        std::thread::spawn(move||{let result=if install{download(&cancel,&done,&total).map_err(|error|format!("{error}\n{RELEASE_API}")).and_then(|_|read_installed())}else if let Some(path)=crate::component_paths::selected("pdfium"){validate_library(&path).map(|_|None)}else{read_installed()};let _=send.send(result);ctx.request_repaint();});
    }
    fn poll(&mut self){if let Some(receiver)=&self.pending{match receiver.try_recv(){
        Ok(result)=>{self.pending=None;self.checked=true;self.ready_event=true;match result{Ok(installed)=>{self.available=Some(installed.is_some()||crate::component_paths::selected("pdfium").is_some());self.installed=installed;},Err(error)=>{self.available=Some(false);self.error=Some(error)}}},
        Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.checked=true;self.error=Some(tr("组件任务中断").into());},Err(mpsc::TryRecvError::Empty)=>{}
    }}}
    fn show(&mut self,ui:&mut egui::Ui){
        if !self.checked&&self.pending.is_none(){self.start(ui.ctx(),false);}
        let mut remove=false;
        let (recheck,install)=component_header_actions(ui,"PDFium",self.available,self.pending.is_some(),(self.available==Some(false)).then_some(("下载 PDFium",cfg!(all(windows,target_arch="x86_64"))&&manifest().is_ok_and(|path|!path.exists()))),|ui|{
            remove=ui.add_enabled(self.checked&&self.pending.is_none(),egui::Button::new(tr("移除"))).clicked();
        });
        if recheck{self.start(ui.ctx(),false);}
        if install{self.start(ui.ctx(),true);}
        if remove{
                let result=(||{let _guard=crate::asset_viewer::pdf_preview::NATIVE_LOCK.try_lock().map_err(|_|tr("PDFium 正在使用，请稍后重试。").to_owned())?;let path=manifest()?;if path.exists(){let installed:Installed=serde_json::from_slice(&fs::read(&path).map_err(|error|error.to_string())?).map_err(|error|error.to_string())?;remove_directory(&installed.folder)?;fs::remove_file(path).map_err(|error|error.to_string())?;}Ok::<_,String>(())})();
                match result{Ok(())=>{self.installed=None;self.available=None;self.checked=false;self.error=None;},Err(error)=>self.error=Some(error)}
            }
        ui.small(tr("PDF 渲染引擎（不含 V8）；支持 PDF 只读预览，暂不支持编辑。"));
        ui.hyperlink_to(tr("组件来源与许可"),"https://github.com/bblanchon/pdfium-binaries");
        ui.small(tr("组件保存在独立目录，不会被缓存清理删除。"));
        if let Some(installed)=&self.installed {ui.label(format!("PDFium · {}",installed.version));if let Ok(path)=installed.path(){ui.add(egui::Label::new(path.display().to_string()).wrap().selectable(true));}}
        if self.pending.is_some(){
            ui.horizontal(|ui|{ui.spinner();ui.label(tr(if self.installing{"正在下载组件…"}else{"正在检测…"}));});
            if self.installing {let total=self.total.load(Ordering::Relaxed);if total>0{ui.add(egui::ProgressBar::new(self.done.load(Ordering::Relaxed) as f32/total as f32).show_percentage());}if ui.button(tr("取消下载")).clicked(){self.cancel.store(true,Ordering::Relaxed);}}
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        let mut changed=false;
        ui.add_enabled_ui(self.pending.is_none(),|ui|{changed=self.picker.show(ui,"pdfium",tr("PDFium DLL 路径"));});
        ui.small(tr("手动路径优先，留空或恢复自动后使用自动检测；设置会保存。"));
        if let Some(path)=crate::component_paths::selected("pdfium"){ui.label(format!("PDFium: {}",path.display()));}
        if changed {self.error=None;self.start(ui.ctx(),false);request_pdf_retry(ui.ctx());}
        if let Some(error)=&self.error{ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap().selectable(true));}
    }
}
#[derive(Default)]
struct LibreOfficeSettings {
    install_complete:bool,
    progress:install_progress::Shared,ready_event:bool,
    checked:bool,path:Option<PathBuf>,version:Option<String>,error:Option<String>,installing:bool,
    pending:Option<mpsc::Receiver<Result<Option<(PathBuf,String)>,String>>>,
}
fn libreoffice_config()->Result<PathBuf,String>{Ok(component_root()?.join("libreoffice_path.txt"))}
pub(crate) fn find_libreoffice(selected:Option<PathBuf>)->Result<Option<(PathBuf,String)>,String>{
    let selected_only=selected.is_some();
    let mut paths=Vec::new();
    if let Some(selected)=selected {
        let name=selected.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
        if !matches!(name.as_str(),"soffice.exe"|"soffice.com"|"soffice"|"libreoffice"){return Err(tr("未检测到有效的 LibreOffice 程序").into());}
        paths.push(selected);
    }
    if let Ok(path)=libreoffice_config(){if let Ok(saved)=fs::read_to_string(path){paths.push(PathBuf::from(saved.trim()));}}
    for variable in ["ProgramFiles","ProgramFiles(x86)"] {if let Some(root)=std::env::var_os(variable){for name in ["soffice.com","soffice.exe"]{paths.push(PathBuf::from(&root).join("LibreOffice/program").join(name));}}}
    if let Some(search)=std::env::var_os("PATH") {for directory in std::env::split_paths(&search){for name in if cfg!(windows){["soffice.com","soffice.exe"]}else{["libreoffice","soffice"]}{paths.push(directory.join(name));}}}
    paths.dedup();
    if selected_only {paths.truncate(1);}
    for path in paths.into_iter().filter(|path|path.is_file()) {
        // --version does not open or convert documents. Bound detection time for custom installations.
        let mut child=crate::media_tools::command(&path).args(["--headless","--version"]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|error|error.to_string())?;
        let started=std::time::Instant::now();
        loop {match child.try_wait().map_err(|error|error.to_string())? {
            Some(status)=>{let output=child.wait_with_output().map_err(|error|error.to_string())?;let version=String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if status.success()&&version.starts_with("LibreOffice"){return Ok(Some((path,version)));}break;},
            None if started.elapsed()>Duration::from_secs(15)=>{let _=child.kill();let _=child.wait();break;},
            None=>std::thread::sleep(Duration::from_millis(50)),
        }}
    }Ok(None)
}
fn install_libreoffice(progress:&install_progress::Shared)->Result<Option<(PathBuf,String)>,String>{
    if let Some(found)=find_libreoffice(None)?{return Ok(Some(found));}
    installer::install("libreoffice",progress)?;find_libreoffice(None)
}
impl LibreOfficeSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool,selected:Option<PathBuf>){
        if self.pending.is_some(){return;}self.install_complete=false;self.error=None;self.installing=install;
        self.progress=install_progress::Shared::default();let progress=self.progress.clone();
        let ctx=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some(receive);
        std::thread::spawn(move||{let result=(||{
            let custom=selected.is_some();let found=if install{install_libreoffice(&progress)?}else{find_libreoffice(selected)?};
            if custom {let (path,_)=found.as_ref().ok_or_else(||tr("未检测到有效的 LibreOffice 程序").to_owned())?;let config=libreoffice_config()?;fs::create_dir_all(config.parent().unwrap()).map_err(|error|error.to_string())?;fs::write(config,path.to_string_lossy().as_bytes()).map_err(|error|error.to_string())?;}
            Ok(found)
        })();let _=send.send(result);ctx.request_repaint();});
    }
    fn poll(&mut self,_ctx:&egui::Context){if let Some(receiver)=&self.pending{match receiver.try_recv(){
        Ok(result)=>{self.pending=None;self.checked=true;self.ready_event=true;let was_installing=std::mem::replace(&mut self.installing,false);match result{Ok(found)=>{self.path=found.as_ref().map(|(path,_)|path.clone());self.version=found.map(|(_,version)|version);self.install_complete=was_installing&&self.path.is_some();if was_installing&&self.path.is_none(){self.error=Some(tr("安装后尚未检测到 LibreOffice，请重新检测或选择程序。").into());}},Err(error)=>self.error=Some(error)}},
        Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.checked=true;self.installing=false;self.error=Some(tr("组件任务中断").into());},Err(mpsc::TryRecvError::Empty)=>{}
    }}}
    fn show(&mut self,ui:&mut egui::Ui){
        if !self.checked&&self.pending.is_none(){self.start(ui.ctx(),false,None);}
        let mut choose=false;
        let (recheck,install)=component_header_actions(ui,"LibreOffice",self.checked.then_some(self.path.is_some()),self.pending.is_some(),(self.checked&&self.path.is_none()).then_some(("安装 LibreOffice",cfg!(all(windows,target_arch="x86_64")))),|ui|{
            choose=ui.add_enabled(self.pending.is_none(),egui::Button::new(tr("选择程序…"))).clicked();
        });
        if recheck{self.start(ui.ctx(),false,None);}
        if install{self.start(ui.ctx(),true,None);}
        if choose{if let Some(path)=rfd::FileDialog::new().add_filter("LibreOffice",&["exe","com"]).pick_file(){self.start(ui.ctx(),false,Some(path));}}
        ui.small(tr("办公文档只读预览需要 LibreOffice 和 PDFium；暂不支持编辑。"));
        ui.small(tr("内置下载器直接下载并校验组件包；LibreOffice 使用 Windows Installer 安装，可能需要系统权限确认。"));
        if self.install_complete{ui.colored_label(crate::theme::BLUE,tr("LibreOffice 安装完成，已检测到可用程序。"));}
        if self.progress.needs_reboot(){ui.small(tr("安装程序要求重启系统，请保存工作后自行重启。"));}
        if let Some(version)=&self.version{ui.label(version);}
        if let Some(path)=&self.path{ui.add(egui::Label::new(path.display().to_string()).wrap().selectable(true));}
        if self.pending.is_some(){if self.installing{
            self.progress.show_for(ui,"internal");
            if self.progress.can_cancel(){if ui.button(tr("取消下载")).clicked(){self.progress.request_cancel();}}
            else{ui.small(tr("已有任务正在进行；当前阶段不允许取消或重复安装。"));}
        }else{ui.horizontal(|ui|{ui.spinner();ui.label(tr("正在检测…"));});}ui.ctx().request_repaint_after(Duration::from_millis(100));}
        ui.hyperlink_to(tr("手动安装"),"https://www.libreoffice.org/download/download-libreoffice/");
        if let Some(error)=&self.error{ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap().selectable(true));}
    }
}
#[derive(Default)]
pub struct Manager {open:bool,ffmpeg:crate::media_tools::DecoderSettings,pdfium:PdfiumSettings,libreoffice:LibreOfficeSettings,yt_dlp:yt_dlp::Settings}
impl Manager {
    pub fn is_open(&self)->bool{self.open}
    pub fn open(&mut self,ctx:&egui::Context){if !self.is_open(){self.ffmpeg.refresh(ctx);}self.open=true;crate::native_window::wake(ctx,egui::Id::new("third_party_window"));}
    pub fn show(&mut self,ctx:&egui::Context){
        self.ffmpeg.poll(ctx);self.pdfium.poll();self.libreoffice.poll(ctx);self.yt_dlp.poll();
        if std::mem::take(&mut self.libreoffice.ready_event){request_pdf_retry(ctx);}
        if std::mem::take(&mut self.pdfium.ready_event){request_pdf_retry(ctx);}
        if !self.open{return;}
        crate::native_window::Window::new(tr("第三方支持")).id(egui::Id::new("third_party_window")).open(&mut self.open).default_width(560.0).default_height(500.0).show(ctx,|ui|{
            self.ffmpeg.show(ui);
            ui.separator();self.yt_dlp.show(ui);
            ui.separator();
            crate::theme::section_heading(ui, tr("网页浏览器HLS嗅探助手"));
            ui.label(tr("需要先安装油猴插件,再安装此脚本"));
            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to(tr("安装油猴插件"), "https://www.tampermonkey.net/");
                ui.add_space(12.0);
                ui.hyperlink_to(tr("安装脚本"), "https://greasyfork.org/zh-CN/scripts/604041-unityyamlviewer-hls-%E5%97%85%E6%8E%A2%E5%8A%A9%E6%89%8B");
            });
            ui.separator();self.pdfium.show(ui);
            ui.separator();self.libreoffice.show(ui);
        });

    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_folder_rejects_traversal_and_cache_roots_do_not_contain_it(){
        assert!(valid_folder("pdfium-123-456"));for value in ["../pdfium-1","pdfium-../a","pdfium-/a","pdfium-1\\a",""]{assert!(!valid_folder(value));}
        if let (Some(cache),Ok(components))=(crate::app_cache::root(),component_root()){assert!(!components.starts_with(&cache));assert_ne!(cache,components);}
    }
}

fn request_pdf_retry(ctx:&egui::Context){
    ctx.data_mut(|data|data.insert_temp(egui::Id::new("pdf_component_retry"),true));
    ctx.request_repaint();
}
