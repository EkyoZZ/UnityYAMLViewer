//! Shared status for component downloads and installation.
use std::{sync::{Arc,Mutex},time::Instant};
use eframe::egui;
use crate::localization::tr;
struct State {phase:usize,done:u64,total:u64,cancel:bool,started:Instant,lines:std::collections::VecDeque<String>,reboot:bool}
impl Default for State{fn default()->Self{Self{phase:0,done:0,total:0,cancel:false,started:Instant::now(),lines:Default::default(),reboot:false}}}
#[derive(Clone,Default)]
pub(crate) struct Shared(Arc<Mutex<State>>);
impl Shared{
    pub(crate) fn phase(&self,phase:usize){if let Ok(mut s)=self.0.lock(){s.phase=phase;}}
    pub(crate) fn can_cancel(&self)->bool{self.0.lock().is_ok_and(|s|s.phase<=2&&!s.cancel)}
    pub(crate) fn request_cancel(&self){if let Ok(mut s)=self.0.lock(){if s.phase<=2{s.cancel=true;}}}
    pub(crate) fn cancel_requested(&self)->bool{self.0.lock().is_ok_and(|s|s.cancel)}
    pub(crate) fn bytes(&self,done:u64,total:u64){if let Ok(mut s)=self.0.lock(){s.done=done;s.total=total;}}
    pub(crate) fn log(&self,line:impl Into<String>){if let Ok(mut s)=self.0.lock(){let line=line.into();if s.lines.back()==Some(&line){return;}s.lines.push_back(line);while s.lines.len()>24{s.lines.pop_front();}}}
    pub(crate) fn details(&self)->String{self.0.lock().map(|s|s.lines.iter().cloned().collect::<Vec<_>>().join("\n")).unwrap_or_default()}
    pub(crate) fn reboot(&self){if let Ok(mut s)=self.0.lock(){s.reboot=true;}}
    pub(crate) fn needs_reboot(&self)->bool{self.0.lock().is_ok_and(|s|s.reboot)}
    pub(crate) fn show_for(&self,ui:&mut egui::Ui,_source:&str){
        let Ok(s)=self.0.lock()else{return;};
        let labels=["正在检测…","正在获取组件发行信息…","正在下载组件包…","正在校验组件包…","正在解压或安装组件…","正在检测安装结果…"];
        ui.horizontal(|ui|{ui.spinner();ui.label(tr(labels[s.phase.min(5)]));});
        if s.phase==2{
            let label=format!("{:.1} MiB / {}",s.done as f64/1048576.0,if s.total>0{format!("{:.1} MiB",s.total as f64/1048576.0)}else{tr("大小未知").into()});
            if s.total>0{ui.add(egui::ProgressBar::new((s.done as f64/s.total as f64).clamp(0.0,1.0) as f32).show_percentage());}
            ui.small(label);
        }
        ui.small(format!("{}: {} s",tr("已等待"),s.started.elapsed().as_secs()));
        if s.cancel{ui.small(tr("正在取消下载…"));}
        if let Some(line)=s.lines.back(){ui.add(egui::Label::new(line).wrap());}
        drop(s);
        self.show_log(ui);
    }
    pub(crate) fn show_log(&self,ui:&mut egui::Ui){
        let Ok(s)=self.0.lock()else{return;};
        if !s.lines.is_empty(){egui::CollapsingHeader::new(tr("查看安装日志")).show(ui,|ui|{egui::ScrollArea::vertical().max_height(160.0).stick_to_bottom(true).show(ui,|ui|{for line in &s.lines{ui.add(egui::Label::new(line).wrap().selectable(true));}});});}
    }
}
#[cfg(test)] mod tests{
    use super::*;
    #[test] fn cancellation_does_not_cover_installation(){let s=Shared::default();s.phase(2);assert!(s.can_cancel());s.request_cancel();assert!(s.cancel_requested());s.phase(4);assert!(!s.can_cancel());}
}
