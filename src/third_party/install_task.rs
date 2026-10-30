//! Process-level guards shared by all application instances. Never terminates external jobs.
use std::{path::Path,process::Stdio,time::{Duration,Instant}};
use crate::localization::tr;
#[cfg(windows)]
pub(super) struct Guard { _library:libloading::Library,handle:*mut std::ffi::c_void,release:unsafe extern "system" fn(*mut std::ffi::c_void)->i32,close:unsafe extern "system" fn(*mut std::ffi::c_void)->i32 }
#[cfg(windows)]
impl Guard {
    pub(super) fn acquire()->Result<Self,String>{unsafe{
        let library=libloading::Library::new("kernel32.dll").map_err(|e|e.to_string())?;
        let create:unsafe extern "system" fn(*mut std::ffi::c_void,i32,*const u16)->*mut std::ffi::c_void=*library.get(b"CreateMutexW\0").map_err(|e|e.to_string())?;
        let wait:unsafe extern "system" fn(*mut std::ffi::c_void,u32)->u32=*library.get(b"WaitForSingleObject\0").map_err(|e|e.to_string())?;
        let release=*library.get(b"ReleaseMutex\0").map_err(|e|e.to_string())?;let close:unsafe extern "system" fn(*mut std::ffi::c_void)->i32=*library.get(b"CloseHandle\0").map_err(|e|e.to_string())?;
        let name:Vec<u16>="Local\\UnityYAMLViewer-LibreOffice-install".encode_utf16().chain(Some(0)).collect();
        let handle=create(std::ptr::null_mut(),0,name.as_ptr());if handle.is_null(){return Err(std::io::Error::last_os_error().to_string());}
        match wait(handle,0){0|0x80=>Ok(Self{_library:library,handle,release,close}),_=>{close(handle);Err(tr("已有第三方组件安装任务，请等待结束后再试。").into())}}
    }}
}
#[cfg(windows)]
impl Drop for Guard {fn drop(&mut self){unsafe{(self.release)(self.handle);(self.close)(self.handle);}}}
#[cfg(not(windows))]
pub(super) struct Guard;
#[cfg(not(windows))]
impl Guard {pub(super) fn acquire()->Result<Self,String>{Err(tr("自动安装仅支持 Windows。").into())}}
#[derive(serde::Deserialize)]
#[serde(rename_all="PascalCase")]
struct Process {name:String,process_id:u32,command_line:Option<String>}
fn blocks(process:&Process,own:Option<u32>)->bool{
    if Some(process.process_id)==own{return false;}
    ((process.name.eq_ignore_ascii_case("powershell.exe")||process.name.eq_ignore_ascii_case("pwsh.exe"))&&process.command_line.as_ref().is_some_and(|s|{let s=s.to_ascii_lowercase();s.contains("scoop.ps1")||s.contains("scoop-bootstrap")||s.contains("uyv_scoop_script")||s.contains("scoop install ")||s.contains("scoop update ")})) ||
    process.name.eq_ignore_ascii_case("msiexec.exe") || (process.name.eq_ignore_ascii_case("winget.exe") && process.command_line.as_ref().is_none_or(|s|{let s=s.to_ascii_lowercase();s.contains("libreoffice")||s.contains("gyan.ffmpeg")}))
}
pub(super) fn ensure_no_other_installer(own:Option<u32>)->Result<(),String>{
    let mut child=crate::media_tools::command("powershell.exe").args(["-NoProfile","-NonInteractive","-Command", "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $items=@(Get-CimInstance Win32_Process -Filter \"Name='winget.exe' OR Name='msiexec.exe' OR Name='powershell.exe' OR Name='pwsh.exe'\" | Where-Object {$_.ProcessId -ne $PID} | Select-Object Name,ProcessId,CommandLine); ConvertTo-Json -InputObject $items -Compress"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|_|tr("无法确认已有安装任务，已阻止重复安装。请检查系统安装状态后重试。").to_owned())?;
    let started=Instant::now();loop{match child.try_wait(){
        Ok(Some(_))=>break,
        Ok(None) if started.elapsed()<Duration::from_secs(10)=>std::thread::sleep(Duration::from_millis(50)),
        _=>{let _=child.kill();let _=child.wait();return Err(tr("无法确认已有安装任务，已阻止重复安装。请检查系统安装状态后重试。").into());}
    }}
    let output=child.wait_with_output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(tr("无法确认已有安装任务，已阻止重复安装。请检查系统安装状态后重试。").into());}
    let processes:Vec<Process>=serde_json::from_slice(&output.stdout).map_err(|_|tr("无法确认已有安装任务，已阻止重复安装。请检查系统安装状态后重试。").to_owned())?;
    if processes.iter().any(|p|blocks(p,own)){return Err(tr("已有组件或系统安装任务正在运行，请等待结束后再试；不会自动终止它。").into());}Ok(())
}
pub(super) fn ensure_download_released()->Result<(),String>{
    let root=std::env::temp_dir().join("WinGet");if !root.exists(){return Ok(());}
    for entry in std::fs::read_dir(root).map_err(|e|e.to_string())?{
        let entry=entry.map_err(|e|e.to_string())?;
        if entry.file_name().to_string_lossy().starts_with("TheDocumentFoundation.LibreOffice."){
            for file in std::fs::read_dir(entry.path()).map_err(|e|e.to_string())?{
                let file=file.map_err(|e|e.to_string())?;if file.file_type().map_err(|e|e.to_string())?.is_file(){probe_file(&file.path())?;}
            }
        }
    }Ok(())
}
fn probe_file(path:&Path)->Result<(),String>{
    let mut options=std::fs::OpenOptions::new();options.read(true);
    #[cfg(windows)]
    {use std::os::windows::fs::OpenOptionsExt;options.share_mode(0);}
    match options.open(path){Ok(_)=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(_)=>Err(tr("上一次下载文件仍被占用，已停止重试。请等待占用解除后再安装。").into())}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test] fn named_mutex_excludes_another_worker_and_releases_on_drop(){
        let guard=Guard::acquire().unwrap();
        assert!(std::thread::spawn(||Guard::acquire().is_err()).join().unwrap());drop(guard);
        assert!(Guard::acquire().is_ok());
    }
    #[cfg(windows)]
    #[test] fn occupied_download_cannot_be_reused_until_released(){
        use std::os::windows::fs::OpenOptionsExt;
        let path=std::env::temp_dir().join(format!("office-lock-test-{}.tmp",std::process::id()));
        let handle=std::fs::OpenOptions::new().write(true).create_new(true).share_mode(0).open(&path).unwrap();
        assert!(probe_file(&path).is_err());drop(handle);assert!(probe_file(&path).is_ok());std::fs::remove_file(path).unwrap();
    }
    #[test] fn guard_blocks_external_installer_and_unknown_command_but_ignores_own(){
        let p=|name:&str,id,cmd:Option<&str>|Process{name:name.into(),process_id:id,command_line:cmd.map(str::to_owned)};
        assert!(blocks(&p("winget.exe",1,Some("winget install --id TheDocumentFoundation.LibreOffice")),None));
        assert!(!blocks(&p("winget.exe",1,Some("winget install --id TheDocumentFoundation.LibreOffice")),Some(1)));
        assert!(blocks(&p("winget.exe",2,None),None));assert!(blocks(&p("msiexec.exe",3,None),None));
        assert!(!blocks(&p("winget.exe",4,Some("winget install --id Git.Git")),None));
        assert!(blocks(&p("powershell.exe",5,Some("powershell -File D:/scoop/apps/scoop/current/bin/scoop.ps1 install ffmpeg")),None));
        assert!(blocks(&p("pwsh.exe",6,Some("pwsh -Command & $env:UYV_SCOOP_SCRIPT install main/ffmpeg")),None));
        assert!(!blocks(&p("powershell.exe",7,Some("powershell -Command Get-Date")),None));
    }
}
