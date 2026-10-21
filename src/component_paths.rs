//! Explicit external component overrides, kept outside disposable cache directories.
use std::{collections::BTreeMap,path::{Path,PathBuf}};
use eframe::egui;
use crate::localization::tr;
fn config()->Result<PathBuf,String>{
    crate::app_cache::root().and_then(|p|p.parent().map(|p|p.join("UnityYAMLViewer-components/manual_paths.json"))).ok_or_else(||tr("无法确定组件目录").to_owned())
}
fn read()->Result<BTreeMap<String,PathBuf>,String>{
    let path=config()?;
    if !path.exists(){return Ok(BTreeMap::new())}
    serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
}
pub(crate) fn selected(key:&str)->Option<PathBuf>{read().ok()?.remove(key)}
fn save(key:&str,path:Option<PathBuf>)->Result<(),String>{
    let mut values=read()?;
    if let Some(path)=path {values.insert(key.into(),path);}else{values.remove(key);}
    let file=config()?;std::fs::create_dir_all(file.parent().unwrap()).map_err(|e|e.to_string())?;
    let temporary=file.with_extension("json.tmp");
    std::fs::write(&temporary,serde_json::to_vec_pretty(&values).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    std::fs::rename(temporary,file).map_err(|e|e.to_string())
}
pub(crate) fn validate_tool(path:&Path,name:&str)->Result<(),String>{
    if !path.is_absolute()||!path.is_file(){return Err(tr("请选择存在的程序或 DLL 的绝对路径。").into())}
    if name=="pdfium" {return crate::third_party::validate_library(path)}
    let output=crate::media_tools::command(path).arg("-version").output().map_err(|e|e.to_string())?;
    if output.status.success()&&String::from_utf8_lossy(&output.stdout).trim_start().starts_with(&format!("{name} version")){Ok(())}
    else{Err(tr("指定程序与组件不匹配或无法运行。").into())}
}
#[derive(Default)]
pub(crate) struct Picker {value:Option<String>,error:Option<String>}
impl Picker {
    pub(crate) fn show(&mut self,ui:&mut egui::Ui,key:&str,label:&str)->bool {
        if self.value.is_none(){match read(){Ok(values)=>self.value=Some(values.get(key).map(|p|p.to_string_lossy().into_owned()).unwrap_or_default()),Err(e)=>{self.value=Some(String::new());self.error=Some(e);}}}
        let mut changed=false;let mut commit=false;let mut clear=false;
        ui.push_id(key,|ui|{
            ui.label(label);
            ui.horizontal(|ui|{
                ui.add(egui::TextEdit::singleline(self.value.as_mut().unwrap()).desired_width((ui.available_width()-180.).max(80.)));
                if ui.button(tr("选择文件")).clicked(){
                    let extension=if key=="pdfium" {"dll"}else{"exe"};
                    if let Some(path)=rfd::FileDialog::new().add_filter(label,&[extension]).pick_file(){self.value=Some(path.to_string_lossy().into_owned());commit=true;}
                }
                commit|=ui.button(tr("保存")).clicked();clear=ui.button(tr("恢复自动")).clicked();
            });
        });
        if commit||clear {
            let path=if clear||self.value.as_deref()==Some(""){None}else{Some(PathBuf::from(self.value.as_deref().unwrap()))};
            let result=(||{if let Some(path)=&path {validate_tool(path,key)?;}save(key,path)})();
            match result {Ok(())=>{if clear{self.value=Some(String::new());}self.error=None;changed=true},Err(e)=>self.error=Some(e)}
        }
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
        changed
    }
}
