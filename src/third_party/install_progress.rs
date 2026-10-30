//! Best-effort CLI stages. Unknown/localized output remains visible without guessing progress.
use std::{io::Read, sync::{Arc,Mutex,LazyLock}, time::Instant};
use eframe::egui;
use crate::localization::tr;
struct State {phase:usize,lines:std::collections::VecDeque<String>,started:Instant,updated:Instant,cancel:bool,slow:bool,cancel_blocked:bool}
impl Default for State {fn default()->Self{let now=Instant::now();Self{phase:0,lines:Default::default(),started:now,updated:now,cancel:false,slow:false,cancel_blocked:false}}}
#[derive(Clone,Default)]
pub(crate) struct Shared(Arc<Mutex<State>>);
static ANSI:LazyLock<regex::Regex>=LazyLock::new(||regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").unwrap());
impl Shared {
    pub(crate) fn can_cancel(&self)->bool{self.0.lock().is_ok_and(|state|state.phase==2&&!state.cancel&&!state.cancel_blocked)}
    pub(crate) fn refuse_cancel(&self){if let Ok(mut state)=self.0.lock(){state.cancel=false;state.cancel_blocked=true;}}
    pub(crate) fn request_cancel(&self){if let Ok(mut state)=self.0.lock(){if state.phase==2&&!state.cancel_blocked{state.cancel=true;}}}
    pub(crate) fn cancel_requested(&self)->bool{self.0.lock().is_ok_and(|state|state.cancel)}
    pub(crate) fn cancel_download(&self,child:&mut std::process::Child)->Result<bool,String>{
        let mut state=self.0.lock().map_err(|_|tr("组件任务中断").to_owned())?;
        if state.phase!=2{state.cancel=false;return Ok(false);}
        // Kill only our WinGet process. Never kill a system installer or a process tree.
        if let Err(error)=child.kill(){state.cancel=false;state.cancel_blocked=true;return Err(error.to_string());}state.cancel=false;Ok(true)
    }
    pub(crate) fn slow(&self){if let Ok(mut state)=self.0.lock(){state.slow=true;}}

    pub(crate) fn phase(&self,phase:usize){if let Ok(mut state)=self.0.lock(){if phase>state.phase{state.phase=phase;if phase!=2{state.cancel=false;}state.updated=Instant::now();}}}
    fn line(&self,bytes:&[u8]){
        let decoded=String::from_utf8_lossy(bytes);let stripped=ANSI.replace_all(&decoded,"");
        let line:String=stripped.chars().filter(|c|!c.is_control()).take(1024).collect();let line=line.trim();
        if line.is_empty()||line.chars().all(|c|"-\\|/ ░▒▓█▏▎▍▌▋▊▉".contains(c)){return;}
        let phase=classify(line);
        if let Ok(mut state)=self.0.lock(){
            state.phase=state.phase.max(phase);
            // A new unrecognized message may indicate an installer transition in another locale.
            if state.phase==2&&phase==0&&!download_numbers(line){state.cancel_blocked=true;state.cancel=false;}
            else if state.phase==2&&phase==2{state.cancel_blocked=false;}
            if state.phase!=2{state.cancel=false;}
            if state.lines.back().is_none_or(|last|last!=line){
                state.lines.push_back(line.into());while state.lines.len()>24{state.lines.pop_front();}state.updated=Instant::now();
            }
        }
    }
    pub(crate) fn details(&self)->String{self.0.lock().map(|state|state.lines.iter().cloned().collect::<Vec<_>>().join("\n")).unwrap_or_default()}
    pub(crate) fn show_for(&self,ui:&mut egui::Ui,source:&str){
        let Ok(state)=self.0.lock()else{return;};
        let labels=["正在检测已有 LibreOffice…","正在查询安装包（WinGet）…","正在下载安装包…","安装包校验完成，等待启动安装…","正在安装，请留意系统权限提示…","安装程序已结束，正在检测 LibreOffice…"];
        ui.horizontal(|ui|{ui.spinner();ui.label(tr(if source=="Scoop"&&state.phase==1{"正在处理 Scoop 安装任务…"}else if source=="Scoop"&&state.phase==5{"正在检测安装结果…"}else{labels[state.phase.min(5)]}));});
        ui.small(format!("{}: {} s",tr("已等待"),state.started.elapsed().as_secs()));
        if let Some(line)=state.lines.back(){ui.add(egui::Label::new(format!("{source}: {line}")).wrap());}
        if state.phase==2{ui.small(tr("下载百分比仅在安装器返回进度时可见。"));}
        if state.cancel_blocked{ui.small(tr("无法确认可以安全取消，已保留原任务并禁止重复安装。"));}
        if state.cancel{ui.small(tr("正在取消下载，等待旧任务退出并释放文件…"));}
        if state.slow{ui.small(tr("安装任务已等待超过 15 分钟，仍在监视；禁止重复安装，请检查系统提示。"));}
        if state.updated.elapsed().as_secs()>=15{ui.small(tr("等待安装器返回新状态；暂时无法判断是否仍在处理，请检查网络和系统提示。"));}
    }
}
fn download_numbers(line:&str)->bool{
    static NUMBERS:LazyLock<regex::Regex>=LazyLock::new(||regex::Regex::new(r"(?i)\d+(?:[.,]\d+)?\s*(?:[kmgt]?i?b)\s*/\s*\d+(?:[.,]\d+)?\s*(?:[kmgt]?i?b)|\d+(?:[.,]\d+)?\s*%").unwrap());
    NUMBERS.is_match(line)
}
fn classify(line:&str)->usize{
    let lower=line.to_lowercase();
    // Only recognize explicit messages. Never infer a phase from elapsed time.
    if ["successfully installed","已成功安装","已成功安裝","インストールが完了","설치 성공","erfolgreich installiert","installation réussie"].iter().any(|s|lower.contains(s)){5}
    else if ["starting package install","正在启动程序包安装","正在啟動套件安裝","正在安装程序包","正在安裝套件","paketinstallation wird gestartet","démarrage de l’installation du paquet"].iter().any(|s|lower.contains(s)){4}
    else if ["successfully verified installer hash","已成功验证安装程序哈希","已成功驗證安裝程式雜湊","installerhash erfolgreich überprüft"].iter().any(|s|lower.contains(s)){3}
    else if ["downloading ","正在下载 ","正在下載 ","ダウンロード中 ","다운로드 중 ","téléchargement ","wird heruntergeladen "].iter().any(|s|lower.contains(s)){2}
    else {0}
}
pub(crate) fn capture(reader:impl Read+Send+'static,progress:Shared){
    std::thread::spawn(move||{
        let mut reader=reader;let mut buffer=[0;2048];let mut line=Vec::new();
        loop{match reader.read(&mut buffer){
            Ok(0)=>break,
            Ok(count)=>for &byte in &buffer[..count]{
                if byte==b'\r'||byte==b'\n'{progress.line(&line);line.clear();}
                else if line.len()<4096{line.push(byte);}
            },
            Err(error) if error.kind()==std::io::ErrorKind::Interrupted=>continue,
            Err(_)=>break,
        }}
        if !line.is_empty(){progress.line(&line);}
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn explicit_messages_advance_but_unknown_output_does_not_invent_stages(){
        let shared=Shared::default();shared.phase(1);
        shared.line(b"\x1b[32mDownloading https://example.test/installer.msi\x1b[0m");assert_eq!(shared.0.lock().unwrap().phase,2);
        shared.line(b"  64.0 MB / 256 MB");assert!(shared.details().contains("64.0 MB / 256 MB"));
        shared.line(b"Successfully verified installer hash");assert_eq!(shared.0.lock().unwrap().phase,3);
        shared.line(b"Starting package install...");assert_eq!(shared.0.lock().unwrap().phase,4);
        shared.line(b"Downloading an older buffered line");assert_eq!(shared.0.lock().unwrap().phase,4);
        shared.line(b"Successfully installed");assert_eq!(shared.0.lock().unwrap().phase,5);
        assert!(!shared.details().contains('\u{1b}'));assert_eq!(classify("Unknown localized status"),0);
        let unknown=Shared::default();unknown.phase(2);unknown.line(b"Unrecognized installer transition");assert!(!unknown.can_cancel());
    }
    #[test] fn cancellation_requires_download_and_is_rechecked_before_termination(){
        let shared=Shared::default();assert!(!shared.can_cancel());shared.phase(1);shared.request_cancel();assert!(!shared.cancel_requested());
        shared.phase(2);assert!(shared.can_cancel());shared.request_cancel();assert!(shared.cancel_requested());assert!(!shared.can_cancel());
        shared.phase(4);assert!(!shared.can_cancel());
    }
    #[test] fn chinese_status_and_bounded_logs(){
        let shared=Shared::default();shared.line("正在下载 https://example.test/a.msi".as_bytes());assert_eq!(shared.0.lock().unwrap().phase,2);
        shared.line("已成功验证安装程序哈希".as_bytes());assert_eq!(shared.0.lock().unwrap().phase,3);
        shared.line("正在启动程序包安装...".as_bytes());assert_eq!(shared.0.lock().unwrap().phase,4);
        for n in 0..100{shared.line(format!("status {n}").as_bytes());}assert_eq!(shared.0.lock().unwrap().lines.len(),24);
        shared.line(b" / ");assert!(!shared.details().ends_with(" / "));
    }
}
