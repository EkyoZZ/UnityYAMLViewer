//! Office documents are converted in an isolated, disposable LibreOffice profile.
use std::{fs, io::Read, path::{Path,PathBuf}, sync::atomic::{AtomicBool,Ordering}, time::{Duration,Instant}};
use crate::localization::tr;
const LIMIT:u64=128*1024*1024;
pub(super) fn supports(path:&Path)->bool {
    matches!(super::extension(path).as_str(),"doc"|"docx"|"xls"|"xlsx"|"ppt"|"pptx"|"odt"|"ods"|"odp"|"rtf")
}
/// Launch the original file using the normal desktop profile, independently of PDF preview.
fn edit(path:&Path)->Result<(),String>{
    let source=fs::canonicalize(path).map_err(|e|e.to_string())?;
    if !source.is_file(){return Err(tr("办公文档不存在或不是文件").into());}
    let (mut program,_)=crate::third_party::find_libreoffice(None)?.ok_or_else(||tr("请先在第三方支持中安装或选择 LibreOffice。").to_owned())?;
    #[cfg(windows)]
    if program.file_name().is_some_and(|name|name.eq_ignore_ascii_case("soffice.com")){
        let gui=program.with_file_name("soffice.exe");if gui.is_file(){program=gui;}
    }
    let url=reqwest::Url::from_file_path(&source).map_err(|_|tr("办公文档路径无效").to_owned())?;
    // Pass a file URL as one argument, never through a shell or the conversion workspace.
    let mut child=crate::media_tools::command(program).arg("-o").arg(url.as_str())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e|format!("{}: {e}",tr("无法启动 LibreOffice")))?;
    // Reap independently: an existing desktop session may close the launcher immediately,
    // while a newly created session may keep it alive until the user closes LibreOffice.
    std::thread::spawn(move||{let _=child.wait();});Ok(())
}
#[derive(Default)]
pub(super) struct Editor {pending:Option<std::sync::mpsc::Receiver<Result<(),String>>>,error:Option<String>,started:bool}
impl Editor {
    pub(super) fn button(&mut self,ui:&mut eframe::egui::Ui,path:&Path){
        if let Some(receiver)=&self.pending{match receiver.try_recv(){
            Ok(result)=>{self.pending=None;match result{Ok(())=>{self.started=true;self.error=None;},Err(error)=>{self.started=false;self.error=Some(error);}}},
            Err(std::sync::mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.error=Some(tr("无法启动 LibreOffice").into());},_=>{}
        }}
        if ui.add_enabled(self.pending.is_none(),eframe::egui::Button::new(tr("在LibreOffice编辑")))
            .on_hover_text(tr("直接打开原文档；在 LibreOffice 保存后，点击重载更新预览。")).clicked(){
            self.error=None;self.started=false;let path=path.to_owned();let ctx=ui.ctx().clone();
            let (send,receive)=std::sync::mpsc::channel();self.pending=Some(receive);
            std::thread::spawn(move||{let result=edit(&path);let _=send.send(result);ctx.request_repaint();});
        }
        if self.pending.is_some(){ui.spinner();ui.label(tr("正在打开 LibreOffice…"));ui.ctx().request_repaint_after(Duration::from_millis(100));}
    }
    pub(super) fn status(&self,ui:&mut eframe::egui::Ui,path:&Path){
        if let Some(error)=&self.error{
            ui.colored_label(ui.visuals().error_fg_color,error);
            if ui.button(tr("第三方支持")).clicked(){crate::third_party::request_open(ui.ctx());}
        }else if self.started{
            ui.small(tr("已启动 LibreOffice；保存后请重载预览。"));
        }
        if crate::remote_file::is_cached(path){ui.small(tr("网络文档编辑的是本地下载副本，不会同步回原网址；请在 LibreOffice 中另存到自己的目录。"));}
    }
}
struct Workspace(PathBuf);
impl Drop for Workspace {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn stop(child:&mut std::process::Child){
    // Terminate only this conversion's process tree, never an existing desktop session.
    #[cfg(windows)]
    {let _=crate::media_tools::command("taskkill").args(["/PID",&child.id().to_string(),"/T","/F"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();}
    let _=child.kill();let _=child.wait();
}
pub(super) fn convert(path:&Path,cancel:&AtomicBool)->Result<Vec<u8>,String>{
    if cancel.load(Ordering::Relaxed){return Err(tr("文档转换已取消").into());}
    // Check PDFium before starting an expensive conversion.
    crate::third_party::verified_pdfium_path()?;
    let (program,_)=crate::third_party::find_libreoffice(None)?.ok_or_else(||tr("请先在第三方支持中安装或选择 LibreOffice。").to_owned())?;
    let root=std::env::temp_dir().join(format!("UnityYAMLViewer-office-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    fs::create_dir(&root).map_err(|e|e.to_string())?;let workspace=Workspace(root);
    let source=fs::File::open(path).map_err(|e|e.to_string())?;
    if source.metadata().map_err(|e|e.to_string())?.len()>LIMIT{return Err(tr("办公文档超过 128 MiB，无法预览").into());}
    // A neutral name prevents filename switches, wildcard expansion and output collisions.
    let input=workspace.0.join(format!("document.{}",super::extension(path)));
    let mut output=fs::File::create(&input).map_err(|e|e.to_string())?;
    let copied=std::io::copy(&mut source.take(LIMIT+1),&mut output).map_err(|e|e.to_string())?;drop(output);
    if copied>LIMIT{return Err(tr("办公文档超过 128 MiB，无法预览").into());}
    let profile=workspace.0.join("profile");fs::create_dir_all(profile.join("user")).map_err(|e|e.to_string())?;
    // Disable all document macros and automatic external-link updates in this profile.
    fs::write(profile.join("user/registrymodifications.xcu"),r#"<?xml version="1.0" encoding="UTF-8"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item><item oor:path="/org.openoffice.Office.Calc/Content/Update"><prop oor:name="Link" oor:op="fuse"><value>1</value></prop></item><item oor:path="/org.openoffice.Office.Writer/Content/Update"><prop oor:name="Link" oor:op="fuse"><value>2</value></prop></item></oor:items>"#).map_err(|e|e.to_string())?;
    let profile=reqwest::Url::from_directory_path(&profile).map_err(|_|tr("文档转换路径无效").to_owned())?;
    if cancel.load(Ordering::Relaxed){return Err(tr("文档转换已取消").into());}
    let mut child=crate::media_tools::command(program)
        .arg(format!("-env:UserInstallation={profile}"))
        .args(["--headless","--nologo","--nodefault","--nofirststartwizard","--convert-to","pdf","--outdir"])
        .arg(&workspace.0).arg(&input).stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e|e.to_string())?;
    let started=Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed)||started.elapsed()>Duration::from_secs(120){
            stop(&mut child);return Err(tr(if cancel.load(Ordering::Relaxed){"文档转换已取消"}else{"文档转换超时（120 秒），请检查文档或重试。"}).into());
        }
        match child.try_wait(){
            Ok(Some(status))=>{if !status.success(){return Err(tr("文档转换失败，文件可能损坏、加密或格式不受支持。").into());}break;},
            Ok(None)=>std::thread::sleep(Duration::from_millis(100)),
            Err(e)=>{stop(&mut child);return Err(e.to_string());}
        }
    }
    let mut bytes=Vec::new();
    fs::File::open(workspace.0.join("document.pdf")).map_err(|_|tr("文档转换失败，文件可能损坏、加密或格式不受支持。").to_owned())?
        .take(LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len() as u64>LIMIT{return Err(tr("转换后的 PDF 超过 128 MiB，无法预览").into());}
    if !bytes.starts_with(b"%PDF-"){return Err(tr("文档转换未生成有效 PDF").into());}
    Ok(bytes)
}
