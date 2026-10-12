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
    // Installers update registry PATH, not the environment of this running process.
    #[cfg(windows)] for paths in fresh_windows_paths() {directories.extend(std::env::split_paths(&paths));}
    if let Some(local)=std::env::var_os("LOCALAPPDATA") {directories.push(PathBuf::from(local).join("Microsoft/WinGet/Links"));}
    for variable in ["ProgramFiles","ProgramFiles(x86)"] {
        if let Some(root)=std::env::var_os(variable) {directories.push(PathBuf::from(root).join("ffmpeg/bin"));}
    }
    #[cfg(windows)] {
        let mut roots=Vec::new();
        if let Some(local)=std::env::var_os("LOCALAPPDATA") {roots.push(PathBuf::from(local).join("Microsoft/WinGet/Packages"));}
        if let Some(programs)=std::env::var_os("ProgramFiles") {roots.push(PathBuf::from(programs).join("WinGet/Packages"));}
        for root in roots {
            if let Ok(packages)=std::fs::read_dir(root) {
                for package in packages.flatten() {
                    if package.file_name().to_string_lossy().to_ascii_lowercase().starts_with("gyan.ffmpeg_") {
                        let mut budget=256;collect_package_directories(&package.path(),4,&mut budget,&mut directories);
                    }
                }
            }
        }
    }
    directories.into_iter().map(|dir|dir.join(&file)).find(|path| {
        path.is_file() && command(path).arg("-version").output().is_ok_and(|output|output.status.success())
    })
}

#[cfg(windows)]
fn collect_package_directories(root:&std::path::Path,depth:usize,budget:&mut usize,output:&mut Vec<PathBuf>) {
    if *budget==0{return;}*budget-=1;output.push(root.to_owned());
    if depth==0{return;}
    if let Ok(entries)=std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if *budget==0{break;}
            if entry.file_type().is_ok_and(|kind|kind.is_dir()) {collect_package_directories(&entry.path(),depth-1,budget,output);}
        }
    }
}

#[cfg(windows)]
fn fresh_windows_paths()->Vec<std::ffi::OsString> {
    use std::{ffi::c_void,os::windows::ffi::OsStringExt};
    #[link(name="advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(key:*mut c_void,subkey:*const u16,value:*const u16,flags:u32,kind:*mut u32,data:*mut c_void,size:*mut u32)->i32;
    }
    let value:Vec<u16>="Path\0".encode_utf16().collect();
    let mut paths=Vec::new();
    for (key,subkey) in [(0x80000001u32,"Environment"),(0x80000002u32,"SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")] {
        let subkey:Vec<u16>=subkey.encode_utf16().chain(Some(0)).collect();
        let mut buffer=vec![0u16;32768];let mut size=(buffer.len()*2) as u32;
        // REG_SZ / REG_EXPAND_SZ; RegGetValueW expands variables for the latter.
        let status=unsafe {RegGetValueW((key as i32 as isize) as *mut c_void,subkey.as_ptr(),value.as_ptr(),0x6,std::ptr::null_mut(),buffer.as_mut_ptr().cast(),&mut size)};
        if status==0 {
            let length=buffer.iter().position(|value|*value==0).unwrap_or(buffer.len());
            paths.push(std::ffi::OsString::from_wide(&buffer[..length]));
        }
    }
    paths
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
        let notify_ready=self.installed!=Some(true);
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
            if notify_ready && matches!(&result,Ok(true)) {
                ctx.data_mut(|data|data.insert_temp(egui::Id::new("media_decoder_ready"),true));
            }
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
