//! Shared AI execution preferences and optional NVIDIA runtime installer.
use std::{path::PathBuf,sync::mpsc};
use eframe::egui;
use crate::{localization::tr,third_party::install_progress::Shared};
#[derive(Clone,serde::Serialize,serde::Deserialize)]
#[serde(default)]
pub(crate) struct Settings {pub prefer_gpu:bool,pub cpu_threads:usize}
impl Default for Settings {fn default()->Self{Self{prefer_gpu:true,cpu_threads:2}}}
fn preferences()->&'static std::sync::Mutex<Settings>{
 static VALUE:std::sync::OnceLock<std::sync::Mutex<Settings>>=std::sync::OnceLock::new();
 VALUE.get_or_init(||{let mut value=crate::third_party::component_root().ok().and_then(|root|std::fs::read(root.join("ai-runtime.json")).ok()).and_then(|bytes|serde_json::from_slice::<Settings>(&bytes).ok()).unwrap_or_default();value.cpu_threads=value.cpu_threads.clamp(1,4);std::sync::Mutex::new(value)})
}
pub(crate) fn settings()->Settings{preferences().lock().unwrap().clone()}

#[derive(Default)]
pub(crate) struct Controls {job:Option<mpsc::Receiver<Result<PathBuf,String>>>,progress:Shared,error:Option<String>}

impl Controls {
 pub fn show(&mut self,ui:&mut egui::Ui){
  crate::theme::section_heading(ui,tr("字幕 AI 计算设置"));
  let mut settings=settings();let old=(settings.prefer_gpu,settings.cpu_threads);
  crate::theme::toggle_switch(ui,tr("优先使用 GPU"),&mut settings.prefer_gpu);
  ui.horizontal(|ui|{ui.label(tr("CPU 回退线程"));ui.add(egui::DragValue::new(&mut settings.cpu_threads).range(1..=4));});
  ui.small(tr("统一用于字幕生成、实时识别和翻译；新任务生效，GPU 不可用时回退 CPU。"));
  if old!=(settings.prefer_gpu,settings.cpu_threads){let result=(||{let root=crate::third_party::component_root()?;std::fs::create_dir_all(&root).map_err(|e|e.to_string())?;std::fs::write(root.join("ai-runtime.json"),serde_json::to_vec_pretty(&settings).map_err(|e|e.to_string())?).map_err(|e|e.to_string())})();if result.is_ok(){*preferences().lock().unwrap()=settings;}self.error=result.err();}
  crate::third_party::component_status_badge(ui,crate::third_party::component_root().is_ok_and(|root|root.join("ai-gpu-runtime/ready").is_file()));
  ui.small(tr("NVIDIA CUDA 运行库下载约 1.3 GB，建议预留 4 GiB；不包含显卡驱动或 CUDA 版识别引擎。"));
  if ui.add_enabled(self.job.is_none(),egui::Button::new(tr("安装 / 补齐 GPU 运行库"))).clicked(){
   self.progress=Shared::default();self.error=None;let progress=self.progress.clone();
   crate::auto_subtitle::run(&mut self.job,ui.ctx(),move||{
    let root=crate::third_party::component_root()?.join("ai-gpu-runtime");
    let mut worker=crate::auto_subtitle::python()?;
    worker.args(["-I","-u","-c",include_str!("ai_gpu_install.py")]).arg(&root);
    crate::auto_subtitle::run_install_command(worker,&progress).map_err(|error|{
        if error.contains("GPU_RUNTIME_IN_USE"){tr("GPU 运行库正在使用中，请先关闭实时字幕及翻译，并等待字幕任务结束后重试。").to_owned()}else{error}
    })?;
    Ok(root)
   });
  }
  if let Some(result)=crate::auto_subtitle::poll(&mut self.job){self.error=result.err();}
  if self.job.is_some(){self.progress.show_for(ui,"CUDA");ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));}
  if let Some(error)=&self.error{self.progress.show_log(ui);ui.colored_label(ui.visuals().error_fg_color,error);}
 }
}
