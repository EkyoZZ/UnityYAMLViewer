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
    if let Some(path)=crate::component_paths::selected(name){
        return crate::component_paths::validate_tool(&path,name).ok().map(|_|path);
    }
    let file=if cfg!(windows) {format!("{name}.exe")} else {name.to_owned()};
    let mut directories=Vec::new();
    if let Some(paths)=std::env::var_os("PATH") {directories.extend(std::env::split_paths(&paths));}
    // Installers update registry PATH, not the environment of this running process.
    #[cfg(windows)] for paths in fresh_windows_paths() {directories.extend(std::env::split_paths(&paths));}
    if let Some(parent)=std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.to_owned())) {directories.push(parent);}
    for variable in ["ProgramFiles","ProgramFiles(x86)"] {
        if let Some(root)=std::env::var_os(variable) {directories.push(PathBuf::from(root).join("ffmpeg/bin"));}
    }
    directories.extend(crate::third_party::installer::tool_directories(name));
    directories.into_iter().map(|dir|dir.join(&file)).find(|path| {
        path.is_file() && command(path).arg("-version").output().is_ok_and(|output|output.status.success())
    })
}

#[cfg(windows)]
pub(crate) fn fresh_windows_paths()->Vec<std::ffi::OsString> {
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

#[cfg(windows)]
fn add_user_path(directory:&std::path::Path)->Result<(),String>{
    use std::{ffi::c_void,os::windows::ffi::{OsStrExt,OsStringExt}};
    #[link(name="advapi32")] unsafe extern "system"{
        fn RegCreateKeyExW(key:*mut c_void,name:*const u16,reserved:u32,class:*mut u16,options:u32,access:u32,security:*const c_void,result:*mut *mut c_void,disposition:*mut u32)->i32;
        fn RegGetValueW(key:*mut c_void,subkey:*const u16,name:*const u16,flags:u32,kind:*mut u32,data:*mut c_void,size:*mut u32)->i32;
        fn RegSetValueExW(key:*mut c_void,name:*const u16,reserved:u32,kind:u32,data:*const u8,size:u32)->i32;
        fn RegCloseKey(key:*mut c_void)->i32;
    }
    #[link(name="kernel32")] unsafe extern "system"{fn ExpandEnvironmentStringsW(source:*const u16,target:*mut u16,size:u32)->u32;}
    #[link(name="user32")] unsafe extern "system"{fn SendMessageTimeoutW(window:*mut c_void,message:u32,wparam:usize,lparam:isize,flags:u32,timeout:u32,result:*mut usize)->isize;}
    for tool in ["ffmpeg","ffprobe"]{crate::component_paths::validate_tool(&directory.join(format!("{tool}.exe")),tool)?;}
    if !directory.is_absolute()||directory.as_os_str().encode_wide().any(|c|c==0||c==b';' as u16){return Err("Invalid PATH directory".into());}
    let subkey:Vec<u16>="Environment\0".encode_utf16().collect();let name:Vec<u16>="Path\0".encode_utf16().collect();
    let mut key=std::ptr::null_mut();let status=unsafe{RegCreateKeyExW((0x80000001u32 as i32 as isize) as *mut c_void,subkey.as_ptr(),0,std::ptr::null_mut(),0,3,std::ptr::null_mut(),&mut key,std::ptr::null_mut())};
    if status!=0{return Err(format!("Registry: {status}"));}
    struct Key(*mut c_void);impl Drop for Key{fn drop(&mut self){unsafe{RegCloseKey(self.0);}}}let key=Key(key);
    let mut buffer=vec![0u16;32768];let mut bytes=(buffer.len()*2) as u32;let mut kind=2;
    let status=unsafe{RegGetValueW(key.0,std::ptr::null(),name.as_ptr(),0x10000006,&mut kind,buffer.as_mut_ptr().cast(),&mut bytes)};
    let mut value=if status==2{kind=2;std::ffi::OsString::new()}else if status==0{
        let end=buffer.iter().position(|c|*c==0).unwrap_or(buffer.len());std::ffi::OsString::from_wide(&buffer[..end])
    }else{return Err(format!("Registry: {status}"));};
    let expected=directory.to_string_lossy().trim_end_matches(['\\','/']).to_lowercase();
    for part in std::env::split_paths(&value){
        let input:Vec<u16>=part.as_os_str().encode_wide().chain(Some(0)).collect();let mut expanded=vec![0u16;32768];
        let size=unsafe{ExpandEnvironmentStringsW(input.as_ptr(),expanded.as_mut_ptr(),expanded.len() as u32)};
        if size>0&&size<=expanded.len() as u32{
            let existing=std::ffi::OsString::from_wide(&expanded[..size as usize-1]);
            if existing.to_string_lossy().trim_end_matches(['\\','/']).to_lowercase()==expected{return Ok(());}
        }
    }
    if !value.is_empty()&&!value.to_string_lossy().ends_with(';'){value.push(";");}value.push(directory);
    let data:Vec<u16>=value.encode_wide().chain(Some(0)).collect();if data.len()>32767{return Err("PATH exceeds Windows length limit".into());}
    let status=unsafe{RegSetValueExW(key.0,name.as_ptr(),0,kind,data.as_ptr().cast(),(data.len()*2) as u32)};
    if status!=0{return Err(format!("Registry: {status}"));}
    unsafe{SendMessageTimeoutW(0xffffusize as *mut c_void,0x1a,0,subkey.as_ptr() as isize,2,2000,std::ptr::null_mut());}Ok(())
}
#[cfg(not(windows))]
fn add_user_path(_directory:&std::path::Path)->Result<(),String>{Err("Windows only".into())}
fn path_candidate()->Option<PathBuf>{
    let mut paths=Vec::new();if let Some(value)=std::env::var_os("PATH"){paths.extend(std::env::split_paths(&value));}
    #[cfg(windows)] for value in fresh_windows_paths(){paths.extend(std::env::split_paths(&value));}
    if ["ffmpeg","ffprobe"].iter().all(|tool|paths.iter().any(|directory|crate::component_paths::validate_tool(&directory.join(format!("{tool}.exe")),tool).is_ok())){return None;}
    crate::third_party::installer::tool_directories("ffmpeg").into_iter().find(|directory|["ffmpeg","ffprobe"].iter().all(|tool|crate::component_paths::validate_tool(&directory.join(format!("{tool}.exe")),tool).is_ok()))
}

#[derive(Default)]
pub struct DecoderSettings {
    path_candidate:Option<PathBuf>,path_pending:Option<mpsc::Receiver<Result<(),String>>>,path_added:bool,
    install_complete:bool,progress:crate::third_party::install_progress::Shared,
    ffmpeg_picker:crate::component_paths::Picker,
    ffprobe_picker:crate::component_paths::Picker,
    installed:Option<bool>,
    pending:Option<mpsc::Receiver<Result<(Option<PathBuf>,Option<PathBuf>,Option<PathBuf>),String>>>,
    paths:(Option<PathBuf>,Option<PathBuf>),
    installing:bool,
    error:Option<String>,
}
impl DecoderSettings {
    fn start(&mut self,ctx:&egui::Context,install:bool) {
        if self.pending.is_some()||self.path_pending.is_some() {return;}
        self.error=None;self.path_added=false;
        self.install_complete=false;self.installing=install;self.progress=Default::default();
        let progress=self.progress.clone();

        let (sender,receiver)=mpsc::channel();self.pending=Some(receiver);
        let ctx=ctx.clone();
        std::thread::spawn(move|| {
            let result=(|| {
                if install{crate::third_party::installer::install("ffmpeg",&progress)?;}
                Ok((find_tool("ffmpeg"),find_tool("ffprobe"),path_candidate()))
            })();
            let _=sender.send(result);ctx.request_repaint();
        });
    }
    pub fn refresh(&mut self,ctx:&egui::Context){self.start(ctx,false);}
    pub fn poll(&mut self,ctx:&egui::Context) {
        use crate::localization::tr;
        if let Some(receiver)=&self.path_pending{match receiver.try_recv(){
            Ok(result)=>{self.path_pending=None;match result{Ok(())=>{self.path_added=true;self.path_candidate=None;},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.path_pending=None;self.error=Some(tr("组件任务中断").into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if let Some(receiver)=&self.pending {
            match receiver.try_recv() {
                Ok(result)=>{ctx.data_mut(|data|data.insert_temp(egui::Id::new("media_decoder_ready"),true));ctx.request_repaint();self.pending=None;match result {Ok(paths)=>{let installed=paths.0.is_some()&&paths.1.is_some();self.path_candidate=paths.2;self.paths=(paths.0,paths.1);self.installed=Some(installed);self.install_complete=self.installing&&installed;if self.installing && !installed {self.error=Some(tr("安装后尚未检测到解码组件，请重新检测或重启程序。").into());}},Err(error)=>self.error=Some(error)}},
                Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.error=Some(tr("解码组件检测失败").into());},
                Err(mpsc::TryRecvError::Empty)=>{},
            }
        }
    }
    pub fn show(&mut self,ui:&mut egui::Ui) {
        use crate::localization::tr;
        if self.installed.is_none() && self.pending.is_none() && self.error.is_none(){self.start(ui.ctx(),false);}
        self.poll(ui.ctx());
        let (recheck,install)=crate::third_party::component_header_actions(ui,"FFmpeg / FFprobe",self.installed,self.pending.is_some()||self.path_pending.is_some(),(self.installed==Some(false)).then_some(("安装 FFmpeg",cfg!(all(windows,target_arch="x86_64")))),|_|{});
        if recheck{self.start(ui.ctx(),false);}
        if install{self.start(ui.ctx(),true);}
        if self.pending.is_some() {
            if self.installing{self.progress.show_for(ui,"internal");if self.progress.can_cancel()&&ui.button(tr("取消下载")).clicked(){self.progress.request_cancel();}}else{ui.horizontal(|ui| {ui.spinner();ui.label(tr(if self.installing {"正在安装…"}else{"正在检测…"}));});}
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        let mut changed=false;
        ui.add_enabled_ui(self.pending.is_none()&&self.path_pending.is_none(),|ui|{
            changed|=self.ffmpeg_picker.show(ui,"ffmpeg",tr("FFmpeg 程序路径"));
            changed|=self.ffprobe_picker.show(ui,"ffprobe",tr("FFprobe 程序路径"));
        });
        ui.small(tr("手动路径优先，留空或恢复自动后使用自动检测；设置会保存。"));
        if changed{self.start(ui.ctx(),false);}
        for (label,path) in [("FFmpeg",&self.paths.0),("FFprobe",&self.paths.1)] {
            if let Some(path)=path {ui.add(egui::Label::new(format!("{label}: {}",path.display())).wrap().selectable(true));}
        }
        if self.install_complete{ui.colored_label(crate::theme::BLUE,tr("FFmpeg Shared 安装完成，已检测到 FFmpeg 和 FFprobe。"));}
        if self.path_pending.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
        if self.pending.is_none()&&self.path_pending.is_none(){if let Some(directory)=self.path_candidate.clone(){
            if ui.button(tr("加入用户 PATH")).on_hover_text(directory.display().to_string()).clicked(){
                let (sender,receiver)=mpsc::channel();self.path_pending=Some(receiver);self.error=None;let ctx=ui.ctx().clone();
                std::thread::spawn(move||{let _=sender.send(add_user_path(&directory));ctx.request_repaint();});
            }
        }}
        if self.path_added{ui.label(tr("已加入用户 PATH，请重新打开终端或相关程序后使用。"));}
        ui.hyperlink_to(tr("组件来源与许可"),"https://github.com/BtbN/FFmpeg-Builds");
        if let Some(error)=&self.error {
            ui.colored_label(ui.visuals().warn_fg_color,tr("解码组件检测或安装失败"));
            ui.collapsing(tr("详情"),|ui| {egui::ScrollArea::vertical().max_height(100.0).show(ui,|ui| {ui.add(egui::Label::new(error).wrap().selectable(true));});});
            ui.hyperlink_to(tr("手动安装"),"https://www.ffmpeg.org/download.html");
        }
    }
}
