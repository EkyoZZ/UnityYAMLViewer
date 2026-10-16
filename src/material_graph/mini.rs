//! Bounded, visible-node live snapshots sharing decoded material inputs.
use std::time::{Duration, Instant};
const REFRESH_INTERVAL:Duration=Duration::from_nanos(16_666_667);
use super::{egui,rs,Arc,HashMap,Mutex,mpsc,Input,Value,PathBuf,Rect,Color32,tr,probe,render_system};
#[derive(Clone)]
pub(super) struct Source { pub shader:PathBuf,pub material:String,pub inputs:Vec<Input>,pub scene:Arc<rs::Scene>,pub revision:u64 }
type Key=(String,usize,bool,String);
struct Entry {
    receive:Option<mpsc::Receiver<Result<Arc<rs::Scene>,String>>>,
    scene:Option<Arc<rs::Scene>>,capture:render_system::SnapshotResult,
    image:Option<egui::TextureHandle>,error:Option<String>,
    pending:bool,last_capture:Option<Instant>,
}
#[derive(Default)]
pub(super) struct Previews { entries:HashMap<Key,Entry> }
impl Previews {
    pub fn clear(&mut self){self.entries.clear();}
    pub fn poll(&mut self,ctx:&egui::Context){
        for (key,entry) in &mut self.entries {
            let received=entry.receive.as_ref().and_then(|rx|match rx.try_recv(){
                Ok(result)=>Some(result),Err(mpsc::TryRecvError::Empty)=>None,
                Err(mpsc::TryRecvError::Disconnected)=>Some(Err("Node preparation stopped".into())),
            });
            if let Some(result)=received{
                entry.receive=None;match result {Ok(scene)=>entry.scene=Some(scene),Err(error)=>entry.error=Some(error)}
            }
            let result=entry.capture.lock().unwrap().take();
            if let Some(result)=result {
                entry.pending=false;
                match result {
                    Ok(image)=>{
                        if let Some(texture)=&mut entry.image {texture.set(image,egui::TextureOptions::LINEAR);}
                        else {entry.image=Some(ctx.load_texture(format!("step-{}-{}-{}-{}",key.0,key.1,key.2,key.3),image,egui::TextureOptions::LINEAR));}
                    },
                    Err(error)=>entry.error=Some(error),
                }
            }
        }
    }
    pub fn show(&mut self,ui:&mut egui::Ui,rect:Rect,canvas:Rect,probe:&probe::Probe,source:Option<&Source>){
        if !rect.intersects(canvas){return;}
        let key=(probe.function.clone(),probe.step,probe.vertex,probe.expression.as_ref().map_or(String::new(),|(expr,_)|expr.clone()));
        if !self.entries.contains_key(&key) && self.entries.len()<64 && self.entries.values().all(|entry|entry.receive.is_none()) {
            if let Some(source)=source {
                let source=source.clone();let probe=probe.clone();let ctx=ui.ctx().clone();let (send,receive)=mpsc::channel();
                self.entries.insert(key.clone(),Entry{receive:Some(receive),scene:None,capture:Arc::new(Mutex::new(None)),image:None,error:None,pending:false,last_capture:None});
                std::thread::spawn(move ||{let _=send.send(prepare(source,probe));ctx.request_repaint();});
            }
        }
        let painter=ui.painter().with_clip_rect(rect.intersect(canvas));
        painter.rect_filled(rect,2.,Color32::from_gray(75));
        if let Some(entry)=self.entries.get_mut(&key){
            if let Some(error)=&entry.error {
                painter.text(rect.center(),egui::Align2::CENTER_CENTER,tr("预览不可用"),egui::FontId::proportional(11.),ui.visuals().warn_fg_color);
                ui.interact(rect.intersect(canvas),ui.id().with((&key,"snapshot_error")),egui::Sense::hover()).on_hover_text(error);
                return;
            }
            if let Some(image)=&entry.image {
                painter.image(image.id(),rect,Rect::from_min_max(egui::pos2(0.,0.),egui::pos2(1.,1.)),Color32::WHITE);
            } else {
                painter.text(rect.center(),egui::Align2::CENTER_CENTER,tr("准备预览快照"),egui::FontId::proportional(11.),Color32::WHITE);
            }
            // Keep the compiled scene and reuse the texture; at most one GPU capture
            // may be in flight for this entry. Hidden/collapsed nodes never call show.
            if let Some(scene)=&entry.scene {
                let now=Instant::now();
                if refresh_due(entry.pending,entry.last_capture,now) {
                    entry.pending=true;entry.last_capture=Some(now);
                    painter.add(egui::Shape::Callback(render_system::snapshot_callback(rect.intersect(canvas),scene.clone(),render_system::elapsed_seconds(),entry.capture.clone(),ui.ctx().clone())));
                }
                let delay=entry.last_capture.map_or(REFRESH_INTERVAL,|last|REFRESH_INTERVAL.saturating_sub(now.saturating_duration_since(last)));
                ui.ctx().request_repaint_after(if entry.pending{REFRESH_INTERVAL}else{delay});
            }
        }else{
            painter.text(rect.center(),egui::Align2::CENTER_CENTER,tr("等待预览快照"),egui::FontId::proportional(11.),Color32::WHITE);
        }
    }
}
fn refresh_due(pending:bool,last:Option<Instant>,now:Instant)->bool {
    !pending && last.is_none_or(|last|now.saturating_duration_since(last)>=REFRESH_INTERVAL)
}
fn prepare(source:Source,probe:probe::Probe)->Result<Arc<rs::Scene>,String>{
    let mut material=source.material;
    for input in &source.inputs {if let Value::Float(value,_)=input.value {material.push_str(&format!("\n- {}: {}\n",input.name,value));}}
    let shader=render_system::shader::expanded_source_file(&source.shader)?;
    let shader=probe::instrument(&shader,&probe)?;
    let (program,mut state)=render_system::shader::translate_preview_source(&shader,&material)?;
    state.source=rs::BlendFactor::One;state.destination=rs::BlendFactor::Zero;state.color_mask=[true;4];
    let program=Arc::new(program);
    let draws=source.scene.draws.iter().map(|draw|{
        let mut draw=draw.clone();let mut material=(*draw.material).clone();material.program=program.clone();material.state=state;
        draw.material=Arc::new(material);draw
    }).collect::<Vec<_>>();
    Ok(Arc::new(rs::Scene{draws:Arc::from(draws),revision:source.revision}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refresh_is_throttled_and_never_duplicates_inflight_capture(){
        let now=Instant::now();
        assert!(refresh_due(false,None,now));
        assert!(!refresh_due(true,None,now));
        assert!(!refresh_due(false,Some(now),now+REFRESH_INTERVAL-Duration::from_nanos(1)));
        assert!(refresh_due(false,Some(now),now+REFRESH_INTERVAL));
        assert!(!refresh_due(true,Some(now),now+Duration::from_secs(1)));
        assert!(refresh_due(false,Some(now),now+Duration::from_secs(10)));
    }
}
