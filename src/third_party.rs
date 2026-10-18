//! Optional components live outside disposable cache roots. Nothing is downloaded on startup.
use std::{fs, io::{Read, Write}, path::{Path,PathBuf}, sync::{Arc,mpsc,atomic::{AtomicBool,AtomicU64,Ordering}}, time::Duration};
use eframe::egui;
use serde::{Serialize,Deserialize};
use sha2::{Digest,Sha256};
use crate::localization::tr;
/// One entry point for missing-component prompts, including detached preview windows.
pub fn request_open(ctx:&egui::Context) {
    ctx.data_mut(|data|data.insert_temp(egui::Id::new("third_party_manager_request"),true));
    ctx.request_repaint();
}
pub fn take_open_request(ctx:&egui::Context)->bool {
    ctx.data_mut(|data|data.remove_temp::<bool>(egui::Id::new("third_party_manager_request"))).unwrap_or(false)
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
    read_installed()?.ok_or_else(||tr("请先在第三方支持中下载 PDFium。" ).to_owned())?.path()
}
fn validate_library(path:&Path)->Result<(),String>{
    // Only called after validating our managed file hash or the downloaded release archive.
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
    if result.is_err(){let _=remove_directory(&folder);}result
}
#[derive(Default)]
struct PdfiumSettings {
    checked:bool,installed:Option<Installed>,pending:Option<mpsc::Receiver<Result<Option<Installed>,String>>>,
    ready_event:bool,installing:bool,error:Option<String>,cancel:Arc<AtomicBool>,done:Arc<AtomicU64>,total:Arc<AtomicU64>,
}
impl PdfiumSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool){
        if self.pending.is_some(){return;}self.error=None;self.installing=install;self.cancel.store(false,Ordering::Relaxed);self.done.store(0,Ordering::Relaxed);self.total.store(0,Ordering::Relaxed);
        let cancel=self.cancel.clone();let done=self.done.clone();let total=self.total.clone();let ctx=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some(receive);
        std::thread::spawn(move||{let result=if install{download(&cancel,&done,&total).and_then(|_|read_installed())}else{read_installed()};let _=send.send(result);ctx.request_repaint();});
    }
    fn poll(&mut self){if let Some(receiver)=&self.pending{match receiver.try_recv(){
        Ok(result)=>{self.pending=None;self.checked=true;self.ready_event=true;match result{Ok(installed)=>{self.installed=installed;},Err(error)=>self.error=Some(error)}},
        Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.checked=true;self.error=Some(tr("组件任务中断").into());},Err(mpsc::TryRecvError::Empty)=>{}
    }}}
    fn show(&mut self,ui:&mut egui::Ui){
        if !self.checked&&self.pending.is_none(){self.start(ui.ctx(),false);}
        ui.small(tr("PDF 渲染引擎（不含 V8）；支持 PDF 只读预览，暂不支持编辑。"));
        ui.hyperlink_to(tr("组件来源与许可"),"https://github.com/bblanchon/pdfium-binaries");
        ui.small(tr("组件保存在独立目录，不会被缓存清理删除。"));
        if let Some(installed)=&self.installed {ui.label(format!("PDFium · {}",installed.version));if let Ok(path)=installed.path(){ui.add(egui::Label::new(path.display().to_string()).wrap().selectable(true));}}
        else{ui.label(tr("未安装"));}
        if self.pending.is_some(){
            ui.horizontal(|ui|{ui.spinner();ui.label(tr(if self.installing{"正在下载组件…"}else{"正在检测…"}));});
            if self.installing {let total=self.total.load(Ordering::Relaxed);if total>0{ui.add(egui::ProgressBar::new(self.done.load(Ordering::Relaxed) as f32/total as f32).show_percentage());}if ui.button(tr("取消下载")).clicked(){self.cancel.store(true,Ordering::Relaxed);}}
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }else{ui.horizontal_wrapped(|ui|{
            if self.installed.is_none() && manifest().is_ok_and(|path|!path.exists()) && ui.add_enabled(cfg!(all(windows,target_arch="x86_64")),egui::Button::new(tr("下载 PDFium"))).clicked(){self.start(ui.ctx(),true);}
            if ui.button(tr("重新检测")).clicked(){self.start(ui.ctx(),false);}
            if ui.add_enabled(self.checked,egui::Button::new(tr("移除 PDFium"))).clicked(){
                let result=(||{let _guard=crate::asset_viewer::pdf_preview::NATIVE_LOCK.try_lock().map_err(|_|tr("PDFium 正在使用，请稍后重试。").to_owned())?;let path=manifest()?;if path.exists(){let installed:Installed=serde_json::from_slice(&fs::read(&path).map_err(|error|error.to_string())?).map_err(|error|error.to_string())?;remove_directory(&installed.folder)?;fs::remove_file(path).map_err(|error|error.to_string())?;}Ok::<_,String>(())})();
                match result{Ok(())=>{self.installed=None;self.error=None;},Err(error)=>self.error=Some(error)}
            }
        });}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().warn_fg_color,error);}
    }
}
#[derive(Default)]
struct LibreOfficeSettings {
    checked:bool,path:Option<PathBuf>,version:Option<String>,error:Option<String>,installing:bool,
    pending:Option<mpsc::Receiver<Result<Option<(PathBuf,String)>,String>>>,
}
fn libreoffice_config()->Result<PathBuf,String>{Ok(component_root()?.join("libreoffice_path.txt"))}
fn find_libreoffice(selected:Option<PathBuf>)->Result<Option<(PathBuf,String)>,String>{
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
impl LibreOfficeSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool,selected:Option<PathBuf>){
        if self.pending.is_some(){return;}self.error=None;self.installing=install;
        let ctx=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some(receive);
        std::thread::spawn(move||{let result=(||{
            if install {
                let output=crate::media_tools::command("winget").args(["install","--id","TheDocumentFoundation.LibreOffice","--exact","--source","winget","--silent","--disable-interactivity","--accept-package-agreements","--accept-source-agreements"]).output().map_err(|error|format!("WinGet: {error}"))?;
                if !output.status.success(){return Err(format!("WinGet ({})\n{}\n{}",output.status,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr)));}
            }
            let custom=selected.is_some();let found=find_libreoffice(selected)?;
            if custom {let (path,_)=found.as_ref().ok_or_else(||tr("未检测到有效的 LibreOffice 程序").to_owned())?;let config=libreoffice_config()?;fs::create_dir_all(config.parent().unwrap()).map_err(|error|error.to_string())?;fs::write(config,path.to_string_lossy().as_bytes()).map_err(|error|error.to_string())?;}
            Ok(found)
        })();let _=send.send(result);ctx.request_repaint();});
    }
    fn poll(&mut self){if let Some(receiver)=&self.pending{match receiver.try_recv(){
        Ok(result)=>{self.pending=None;self.checked=true;match result{Ok(found)=>{self.path=found.as_ref().map(|(path,_)|path.clone());self.version=found.map(|(_,version)|version);if self.installing&&self.path.is_none(){self.error=Some(tr("安装后尚未检测到 LibreOffice，请重新检测或选择程序。").into());}},Err(error)=>self.error=Some(error)}},
        Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.checked=true;self.error=Some(tr("组件任务中断").into());},Err(mpsc::TryRecvError::Empty)=>{}
    }}}
    fn show(&mut self,ui:&mut egui::Ui){
        if !self.checked&&self.pending.is_none(){self.start(ui.ctx(),false,None);}
        ui.small(tr("办公文档转换引擎；当前仅提供组件管理，Office 文档预览与编辑尚未接入。"));
        ui.small(tr("通过 WinGet 安装，可能需要管理员确认；由系统管理，不会被软件缓存清理删除。"));
        if let Some(version)=&self.version{ui.label(version);}else{ui.label(tr("未安装"));}
        if let Some(path)=&self.path{ui.add(egui::Label::new(path.display().to_string()).wrap().selectable(true));}
        if self.pending.is_some(){ui.horizontal(|ui|{ui.spinner();ui.label(tr(if self.installing{"正在安装…"}else{"正在检测…"}));});ui.ctx().request_repaint_after(Duration::from_millis(100));}
        else {ui.horizontal_wrapped(|ui|{
            if self.path.is_none()&&ui.add_enabled(cfg!(windows),egui::Button::new(tr("安装 LibreOffice"))).clicked(){self.start(ui.ctx(),true,None);}
            if ui.button(tr("重新检测")).clicked(){self.start(ui.ctx(),false,None);}
            if ui.button(tr("选择程序…")).clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("LibreOffice",&["exe","com"]).pick_file(){self.start(ui.ctx(),false,Some(path));}}
        });}
        ui.hyperlink_to(tr("手动安装"),"https://www.libreoffice.org/download/download-libreoffice/");
        if let Some(error)=&self.error{ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap());}
    }
}
#[derive(Default)]
pub struct Manager {open:bool,ffmpeg:crate::media_tools::DecoderSettings,pdfium:PdfiumSettings,libreoffice:LibreOfficeSettings}
impl Manager {
    pub fn is_open(&self)->bool{self.open}
    pub fn open(&mut self,ctx:&egui::Context){if !self.is_open(){self.ffmpeg.refresh(ctx);}self.open=true;crate::native_window::wake(ctx,egui::Id::new("third_party_window"));}
    pub fn show(&mut self,ctx:&egui::Context){
        self.ffmpeg.poll(ctx);self.pdfium.poll();self.libreoffice.poll();
        if std::mem::take(&mut self.pdfium.ready_event){request_pdf_retry(ctx);}
        if !self.open{return;}
        crate::native_window::Window::new(tr("第三方支持")).id(egui::Id::new("third_party_window")).open(&mut self.open).default_width(560.0).default_height(500.0).show(ctx,|ui|{
            let title_color=if ui.visuals().dark_mode {egui::Color32::from_rgb(105,175,255)} else {egui::Color32::from_rgb(25,85,165)};
            ui.heading(egui::RichText::new("FFmpeg / FFprobe").color(title_color));self.ffmpeg.show(ui);
            ui.separator();ui.heading(egui::RichText::new("PDFium").color(title_color));self.pdfium.show(ui);
            ui.separator();ui.heading(egui::RichText::new("LibreOffice").color(title_color));self.libreoffice.show(ui);
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
