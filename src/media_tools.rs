//! Optional external FFmpeg tools; installation is only started by a user click.
use std::{path::PathBuf, process::Command, sync::mpsc};
use eframe::egui;

pub(crate) fn command(program:impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command=Command::new(program);
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

pub fn find_tool(name:&str) -> Option<PathBuf> {
    let file=if cfg!(windows) {format!("{name}.exe")} else {name.to_owned()};
    let mut directories=Vec::new();
    if let Some(parent)=std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.to_owned())) {directories.push(parent);}
    if let Some(paths)=std::env::var_os("PATH") {directories.extend(std::env::split_paths(&paths));}
    if let Some(local)=std::env::var_os("LOCALAPPDATA") {directories.push(PathBuf::from(local).join("Microsoft/WinGet/Links"));}
    for variable in ["ProgramFiles","ProgramFiles(x86)"] {
        if let Some(root)=std::env::var_os(variable) {directories.push(PathBuf::from(root).join("ffmpeg/bin"));}
    }
    directories.into_iter().map(|dir|dir.join(&file)).find(|path| {
        path.is_file() && command(path).arg("-version").output().is_ok_and(|output|output.status.success())
    })
}

#[derive(Default)]
pub struct DecoderSettings {
    installed:Option<bool>,
    pending:Option<mpsc::Receiver<Result<bool,String>>>,
    installing:bool,
    error:Option<String>,
}
impl DecoderSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool) {
        if self.pending.is_some() {return;}
        self.error=None;
        self.installing=install;
        let (sender,receiver)=mpsc::channel();self.pending=Some(receiver);
        let ctx=ctx.clone();
        std::thread::spawn(move|| {
            let result=(|| {
                if install {
                    let output=command("winget").args(["install","--id","Gyan.FFmpeg","--exact","--source","winget","--scope","user","--silent","--disable-interactivity","--accept-package-agreements","--accept-source-agreements"])
                        .output().map_err(|e|format!("WinGet: {e}"))?;
                    if !output.status.success() {
                        return Err(format!("WinGet ({})\n{}\n{}",output.status,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr)));
                    }
                }
                Ok(find_tool("ffmpeg").is_some() && find_tool("ffprobe").is_some())
            })();
            let _=sender.send(result);ctx.request_repaint();
        });
    }
    pub fn show(&mut self,ui:&mut egui::Ui) {
        use crate::localization::tr;
        if self.installed.is_none() && self.pending.is_none() && self.error.is_none() {self.start(ui.ctx(),false);}
        if let Some(receiver)=&self.pending {
            match receiver.try_recv() {
                Ok(result)=>{self.pending=None;match result {Ok(installed)=>{self.installed=Some(installed);if self.installing && !installed {self.error=Some(tr("安装后尚未检测到解码组件，请重新检测或重启程序。").into());}},Err(error)=>self.error=Some(error)}},
                Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.error=Some(tr("解码组件检测失败").into());},
                Err(mpsc::TryRecvError::Empty)=>{},
            }
        }
        if self.pending.is_some() {
            ui.horizontal(|ui| {ui.spinner();ui.label(tr(if self.installing {"正在安装…"}else{"正在检测…"}));});
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        } else {
            ui.horizontal(|ui| {
                ui.label(tr(if self.installed==Some(true) {"已安装"}else{"未安装"}));
                if self.installed!=Some(true) && ui.button(tr("安装 FFmpeg")).clicked() {self.start(ui.ctx(),true);}
                if ui.button(tr("重新检测")).clicked() {self.start(ui.ctx(),false);}
            });
        }
        if self.installed!=Some(true) {ui.small(tr("通过 Windows 包管理器安装 FFmpeg，需要联网；点击安装即同意第三方软件许可。"));}
        if let Some(error)=&self.error {
            ui.colored_label(ui.visuals().warn_fg_color,tr("解码组件检测或安装失败"));
            ui.collapsing(tr("详情"),|ui| {egui::ScrollArea::vertical().max_height(100.0).show(ui,|ui| {ui.add(egui::Label::new(error).wrap().selectable(true));});});
            ui.hyperlink_to(tr("手动安装"),"https://www.ffmpeg.org/download.html");
            ui.hyperlink_to("Windows App Installer","https://apps.microsoft.com/detail/9nblggh4nns1");
        }
    }
}
