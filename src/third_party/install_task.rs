//! Process-level guards shared by all application instances. Never terminates external jobs.
use std::{process::Stdio,time::{Duration,Instant}};
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
struct Process {name:String,process_id:u32}
fn blocks(process:&Process,own:Option<u32>)->bool{
    if Some(process.process_id)==own{return false;}
    process.name.eq_ignore_ascii_case("msiexec.exe")
}
pub(super) fn ensure_no_other_installer(own:Option<u32>)->Result<(),String>{
    let mut child=crate::media_tools::command("powershell.exe").args(["-NoProfile","-NonInteractive","-Command", "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $items=@(Get-CimInstance Win32_Process -Filter \"Name='msiexec.exe'\" | Where-Object {$_.ProcessId -ne $PID} | Select-Object Name,ProcessId); ConvertTo-Json -InputObject $items -Compress"])
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
#[cfg(test)]
mod tests{
    use super::*;
    #[cfg(windows)]
    #[test]fn named_mutex_excludes_another_worker_and_releases_on_drop(){
        let guard=Guard::acquire().unwrap();assert!(std::thread::spawn(||Guard::acquire().is_err()).join().unwrap());drop(guard);assert!(Guard::acquire().is_ok());
    }
    #[test]fn system_installer_is_blocked_but_own_process_is_ignored(){
        let process=Process{name:"msiexec.exe".into(),process_id:42};assert!(blocks(&process,None));assert!(!blocks(&process,Some(42)));
    }
}
