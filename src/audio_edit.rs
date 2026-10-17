//! Immutable lossless cache revisions; edits never overwrite the source or an undo revision.
use std::{path::{Path,PathBuf},sync::{Arc,mpsc,atomic::{AtomicU64,Ordering}}};
use eframe::egui;
use crate::media_tools::command;
static NEXT:AtomicU64=AtomicU64::new(0);
static UNDO_LIMIT:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(crate::editing::DEFAULT_UNDO_LIMIT);
pub fn set_undo_limit(limit:usize){UNDO_LIMIT.store(limit.clamp(crate::editing::MIN_UNDO_LIMIT,crate::editing::MAX_UNDO_LIMIT),Ordering::Relaxed);}
struct Cache(PathBuf);
impl Drop for Cache {fn drop(&mut self){
    if std::fs::remove_dir_all(&self.0).is_err(){
        // Playback children stop asynchronously; Windows can briefly retain handles.
        let path=self.0.clone();std::thread::spawn(move||{
            for _ in 0..20 {
                match std::fs::remove_dir_all(&path){Ok(())=>return,Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return,Err(_)=>std::thread::sleep(std::time::Duration::from_millis(100))}
            }
        });
    }
}}
pub struct Clip {pub path:PathBuf,pub duration:f64,cache:Arc<Cache>}
impl Drop for Clip {fn drop(&mut self){let _=std::fs::remove_file(&self.path);}}
#[derive(Clone)]
pub struct State {
    pub clip:Option<Arc<Clip>>,pub mode:String,pub start:f64,pub end:f64,
    pub db:f64,pub fade_in:f64,pub fade_out:f64,
    pub in_curve:String,pub out_curve:String,
    pub in_points:Vec<(f64,f64)>,pub out_points:Vec<(f64,f64)>,
    pub in_handles:Vec<[(f64,f64);2]>,pub out_handles:Vec<[(f64,f64);2]>,
}
impl State {
    pub fn duration(&self)->f64{self.clip.as_ref().map_or(self.end,|clip|clip.duration)}
    pub fn interval(&self)->bool{self.mode=="interval"}
    pub fn bounds(&self)->(f64,f64){if self.interval(){(self.start.min(self.end),self.start.max(self.end))}else{(self.start,self.end)}}
    pub fn gain_bounds(&self)->(f64,f64){
        let (start,end)=self.bounds();
        if start.is_finite()&&end.is_finite()&&end>start{(start.max(0.0),end.min(self.duration()))}else{(0.0,self.duration())}
    }
    pub fn effect_factor(&self,t:f64)->f64 {
        let (start,end)=self.bounds();
        let (gain_start,gain_end)=self.gain_bounds();
        let mut factor=if t>=gain_start&&t<gain_end{10.0_f64.powf(self.db/20.0)}else{1.0};
        if t>=start && t<=end {
            if self.fade_in>0.0 {factor*=fade_value(((t-start)/self.fade_in).clamp(0.0,1.0),&self.in_curve,&self.in_points,&self.in_handles);}
            if self.fade_out>0.0 {factor*=fade_value(((end-t)/self.fade_out).clamp(0.0,1.0),&self.out_curve,&self.out_points,&self.out_handles);}
        }
        factor
    }
    pub fn filter(&self,channels:usize)->String {
        let (start,end)=self.bounds();
        let gain=10.0_f64.powf(self.db/20.0);
        let (gain_start,gain_end)=self.gain_bounds();
        let mut envelope="1".to_owned();
        if self.fade_in>0.0 {envelope+=&format!("*({})",fade_expression(&format!("min(1,max(0,(t-{start})/{}))",self.fade_in),&self.in_curve,&self.in_points,&self.in_handles));}
        if self.fade_out>0.0 {envelope+=&format!("*({})",fade_expression(&format!("min(1,max(0,({end}-t)/{}))",self.fade_out),&self.out_curve,&self.out_points,&self.out_handles));}
        let factor=format!("if(gte(n,ceil({gain_start}*s))*lt(n,ceil({gain_end}*s)),{gain},1)*if(between(t,{start},{end}),{envelope},1)");
        format!("aeval=exprs='{}':c=same",(0..channels).map(|ch|format!("val({ch})*({factor})")).collect::<Vec<_>>().join("|"))
    }
    pub fn kept_ranges(&self)->Vec<(f64,f64)> {
        let (start,end)=self.bounds();
        if self.interval(){[(0.0,start),(end,self.duration())].into_iter().filter(|(a,b)|b-a>0.000001).collect()}
        else{vec![(self.start,self.end)]}
    }
    pub fn same_controls(&self,other:&Self)->bool {
        self.mode==other.mode && self.start==other.start && self.end==other.end && self.db==other.db && self.fade_in==other.fade_in && self.fade_out==other.fade_out && self.in_curve==other.in_curve && self.out_curve==other.out_curve && self.in_points==other.in_points && self.out_points==other.out_points && self.in_handles==other.in_handles && self.out_handles==other.out_handles
    }
}
struct Job {receiver:mpsc::Receiver<Result<Arc<Clip>,String>>,before:Option<State>,cut:bool}
pub struct Session {
    pub state:State,pub error:Option<String>,undo:Vec<State>,redo:Vec<State>,job:Option<Job>,
    ffmpeg:PathBuf,channels:usize,gesture:bool,
}
impl Session {
    pub fn new(ffmpeg:&Path,source:&Path,duration:f64,audio:u64,channels:usize,ctx:&egui::Context)->Result<Self,String> {
        let root=std::env::temp_dir().join(format!("unity-audio-edit-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&root).map_err(|e|e.to_string())?;
        let cache=Arc::new(Cache(root));let output=cache.0.join("base.wav");
        let (tx,rx)=mpsc::channel();let ffmpeg=ffmpeg.to_owned();let tool=ffmpeg.clone();let source=source.to_owned();let ctx=ctx.clone();
        std::thread::spawn(move|| {
            let result=(|| {
                let status=command(&tool).args(["-nostdin","-v","error","-n","-i"]).arg(source).args(["-map",&format!("0:{audio}"),"-vn","-sn","-dn","-c:a","pcm_f32le","-rf64","auto"]).arg(&output).output().map_err(|e|e.to_string())?;
                if !status.status.success(){return Err(String::from_utf8_lossy(&status.stderr).into_owned());}
                let duration=duration_of(&output).unwrap_or(duration);
                Ok(Arc::new(Clip{path:output,duration,cache}))
            })();let _=tx.send(result);ctx.request_repaint();
        });
        Ok(Self{state:State{clip:None,mode:"ends".into(),start:0.0,end:duration,db:0.0,fade_in:0.0,fade_out:0.0,in_curve:"linear".into(),out_curve:"linear".into(),in_points:Vec::new(),out_points:Vec::new(),in_handles:Vec::new(),out_handles:Vec::new()},error:None,undo:vec![],redo:vec![],job:Some(Job{receiver:rx,before:None,cut:false}),ffmpeg,channels:channels.max(1),gesture:false})
    }
    pub fn busy(&self)->bool{self.job.is_some()}
    pub fn ready(&self)->bool{self.state.clip.is_some() && !self.busy()}
    pub fn can_undo(&self)->bool{!self.busy()&&!self.undo.is_empty()}
    pub fn can_redo(&self)->bool{!self.busy()&&!self.redo.is_empty()}
    fn push_undo(&mut self,state:State){self.undo.push(state);while self.undo.len()>UNDO_LIMIT.load(Ordering::Relaxed){self.undo.remove(0);}}
    pub fn record_controls(&mut self,before:State,pointer_down:bool){
        if !self.state.same_controls(&before) {
            if !self.gesture {self.push_undo(before);self.redo.clear();}
            self.gesture=pointer_down;
        }else if !pointer_down {self.gesture=false;}
    }
    pub fn undo(&mut self){if self.can_undo(){self.redo.push(self.state.clone());self.state=self.undo.pop().unwrap();self.gesture=false;}}
    pub fn redo(&mut self){if self.can_redo(){self.push_undo(self.state.clone());self.state=self.redo.pop().unwrap();self.gesture=false;}}
    pub fn poll(&mut self,ctx:&egui::Context){
        let limit=UNDO_LIMIT.load(Ordering::Relaxed);
        if self.undo.len()>limit{self.undo.drain(..self.undo.len()-limit);}
        if self.redo.len()>limit{self.redo.drain(..self.redo.len()-limit);}
        let Some(job)=&self.job else{return};
        let result=match job.receiver.try_recv(){Ok(result)=>result,Err(mpsc::TryRecvError::Empty)=>{ctx.request_repaint_after(std::time::Duration::from_millis(40));return},Err(_)=>Err("音频编辑任务意外中断".into())};
        let job=self.job.take().unwrap();
        let initial=job.before.is_none();
        match result {
            Ok(clip)=>{
                if let Some(before)=job.before{self.push_undo(before);self.redo.clear();}
                self.state.clip=Some(clip);self.state.db=0.0;self.state.fade_in=0.0;self.state.fade_out=0.0;
                if initial||job.cut {self.state.start=0.0;self.state.end=self.state.duration();}
                if job.cut {self.state.mode="ends".into();}
                self.error=None;
            }
            Err(error)=>{if let Some(before)=job.before{self.state=before;}self.error=Some(error);},
        }
    }
    pub fn apply(&mut self,cut:bool,ctx:&egui::Context,undo_state:Option<State>){
        if !self.ready(){return;}
        let before=self.state.clone();let clip=before.clip.as_ref().unwrap().clone();
        let ranges=if cut{before.kept_ranges()}else{vec![(0.0,clip.duration)]};
        let duration:f64=ranges.iter().map(|(a,b)|b-a).sum();
        if duration<=0.000001{self.error=Some("剪切后没有保留的音频".into());return;}
        let output=clip.cache.0.join(format!("edit-{}.wav",NEXT.fetch_add(1,Ordering::Relaxed)));
        let filter=before.filter(self.channels);let tool=self.ffmpeg.clone();let ctx=ctx.clone();let (tx,rx)=mpsc::channel();
        self.job=Some(Job{receiver:rx,before:Some(undo_state.unwrap_or(before)),cut});self.error=None;
        std::thread::spawn(move||{
            let result=(||{
                let mut graph=format!("[0:a]{filter}[fx];");
                if ranges.len()>1 {graph+=&format!("[fx]asplit={}",ranges.len());for i in 0..ranges.len(){graph+=&format!("[part{i}]");}graph.push(';');}
                for (i,(a,b)) in ranges.iter().enumerate(){graph+=&format!("[{}]atrim=start={a}:end={b},asetpts=PTS-STARTPTS[s{i}];",if ranges.len()>1{format!("part{i}")}else{"fx".into()});}
                for i in 0..ranges.len(){graph+=&format!("[s{i}]");}
                graph+=&format!("concat=n={}:v=0:a=1[out]",ranges.len());
                // Bezier expressions can exceed Windows command-line length for multichannel audio.
                let script=output.with_extension("filter.txt");
                std::fs::write(&script,&graph).map_err(|e|e.to_string())?;
                let run=|option:&str|command(&tool).args(["-nostdin","-v","error","-n","-i"]).arg(&clip.path).arg(option).arg(&script).args(["-map","[out]","-c:a","pcm_f32le","-rf64","auto"]).arg(&output).output();
                let mut result=run("-/filter_complex");
                if result.as_ref().is_ok_and(|r|!r.status.success()&&String::from_utf8_lossy(&r.stderr).contains("Unrecognized option")){
                    result=run("-filter_complex_script");
                }
                let _=std::fs::remove_file(script);
                let result=result.map_err(|e|e.to_string())?;
                if !result.status.success(){let _=std::fs::remove_file(&output);return Err(String::from_utf8_lossy(&result.stderr).into_owned());}
                let duration=duration_of(&output).unwrap_or(duration);
                Ok(Arc::new(Clip{path:output,duration,cache:clip.cache.clone()}))
            })();let _=tx.send(result);ctx.request_repaint();
        });
    }
    pub fn filter_preview(&self,wave:&[Vec<f32>])->Vec<Vec<f32>> {
        let duration=self.state.duration();
        wave.iter().map(|channel|channel.iter().enumerate().map(|(i,value)|(*value as f64*self.state.effect_factor((i as f64+0.5)/channel.len().max(1) as f64*duration)) as f32).collect()).collect()
    }
    pub fn filter_string(&self)->String{self.state.filter(self.channels)}
}
pub const FADE_CURVES:[(&str,&str);7]=[("linear","线性"),("smooth","平滑"),("equal","等功率"),("quadratic","二次曲线"),("cubic","三次曲线"),("sqrt","平方根曲线"),("smoother","更平滑")];
pub fn preset_points(kind:&str)->Vec<(f64,f64)>{(0..=2).map(|i|{let x=i as f64/2.0;(x,curve(x,kind))}).collect()}
pub fn preset_handles(kind:&str,points:&[(f64,f64)])->Vec<[(f64,f64);2]>{
    points.windows(2).map(|p|{
        let ((a,y),(b,z))=(p[0],p[1]);let dx=(b-a)/3.0;
        let slope=|x:f64|{let lo=(x-0.0001).max(0.0);let hi=(x+0.0001).min(1.0);(curve(hi,kind)-curve(lo,kind))/(hi-lo)};
        [(a+dx,(y+slope(a)*dx).clamp(0.0,1.0)),(b-dx,(z-slope(b)*dx).clamp(0.0,1.0))]
    }).collect()
}
fn bezier(t:f64,a:f64,b:f64,c:f64,d:f64)->f64{let u=1.0-t;u*u*u*a+3.0*u*u*t*b+3.0*u*t*t*c+t*t*t*d}
pub fn fade_value(x:f64,kind:&str,points:&[(f64,f64)],handles:&[[(f64,f64);2]])->f64{
    if kind!="custom"||points.len()<2{return curve(x,kind);}
    let x=x.clamp(0.0,1.0);
    for (index,pair) in points.windows(2).enumerate(){
        let ((a,y),(b,z))=(pair[0],pair[1]);
        if x<=b{
            let Some(h)=handles.get(index) else{return (y+(z-y)*((x-a)/(b-a)).clamp(0.0,1.0)).clamp(0.0,1.0)};
            if x<=a{return y;}if x>=b{return z;}
            let (mut lo,mut hi)=(0.0,1.0);
            for _ in 0..18{let t=(lo+hi)*0.5;if bezier(t,a,h[0].0,h[1].0,b)<x{lo=t;}else{hi=t;}}
            return bezier((lo+hi)*0.5,y,h[0].1,h[1].1,z).clamp(0.0,1.0);
        }
    }
    points.last().unwrap().1
}
fn bezier_expression(t:&str,a:f64,b:f64,c:f64,d:f64)->String{
    format!("pow(1-({t}),3)*({a})+3*pow(1-({t}),2)*({t})*({b})+3*(1-({t}))*pow(({t}),2)*({c})+pow(({t}),3)*({d})")
}
fn fade_expression(x:&str,kind:&str,points:&[(f64,f64)],handles:&[[(f64,f64);2]])->String{
    if kind!="custom"||points.len()<2{return curve_expression(x,kind);}
    let mut result=points.last().unwrap().1.to_string();
    for (index,pair) in points.windows(2).enumerate().rev(){
        let ((a,y),(b,z))=(pair[0],pair[1]);
        let segment=if let Some(h)=handles.get(index){
            let bx=bezier_expression("ld(3)",a,h[0].0,h[1].0,b);
            let by=bezier_expression("(ld(1)+ld(2))/2",y,h[0].1,h[1].1,z);
            format!("if(lte(({x}),{a}),{y},if(gte(({x}),{b}),{z},(st(0,({x}));st(1,0);st(2,1);st(4,0);while(lt(ld(4),18),st(3,(ld(1)+ld(2))/2);if(lt(({bx}),ld(0)),st(1,ld(3)),st(2,ld(3)));st(4,ld(4)+1));{by})))")
        }else{format!("({y})+(({z})-({y}))*(({x})-({a}))/(({b})-({a}))")};
        result=format!("if(lte(({x}),{b}),({segment}),{result})");
    }
    result
}
fn curve(x:f64,kind:&str)->f64{match kind{"smooth"=>x*x*(3.0-2.0*x),"equal"=>(x*std::f64::consts::FRAC_PI_2).sin(),"quadratic"=>x*x,"cubic"=>x*x*x,"sqrt"=>x.sqrt(),"smoother"=>x*x*x*(x*(x*6.0-15.0)+10.0),_=>x}}
fn duration_of(path:&Path)->Option<f64>{
    let tool=crate::media_tools::find_tool("ffprobe")?;
    let output=command(tool).args(["-v","error","-show_entries","format=duration","-of","csv=p=0"]).arg(path).output().ok()?;
    let duration=String::from_utf8_lossy(&output.stdout).trim().parse::<f64>().ok()?;
    (output.status.success()&&duration.is_finite()&&duration>0.0).then_some(duration)
}
fn curve_expression(x:&str,kind:&str)->String{match kind{"smooth"=>format!("({x})*({x})*(3-2*({x}))"),"equal"=>format!("sin(({x})*PI/2)"),"quadratic"=>format!("pow(({x}),2)"),"cubic"=>format!("pow(({x}),3)"),"sqrt"=>format!("sqrt({x})"),"smoother"=>format!("pow(({x}),3)*(({x})*(({x})*6-15)+10)"),_=>x.into()}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn bezier_handles_change_curvature_and_survive_undo(){
        let points=vec![(0.0,0.0),(1.0,1.0)];
        let handles=vec![[(0.15,0.8),(0.7,0.2)]];
        let middle=fade_value(0.5,"custom",&points,&handles);
        assert!((middle-0.5147104).abs()<1e-5);
        assert!((fade_value(0.25,"custom",&points,&handles)-0.25).abs()>0.05);
        assert_eq!(fade_value(0.0,"custom",&points,&handles),0.0);
        assert_eq!(fade_value(1.0,"custom",&points,&handles),1.0);
        let mut editor=session();let before=editor.state.clone();
        editor.state.in_curve="custom".into();editor.state.in_points=points;editor.state.in_handles=handles.clone();
        editor.undo.push(before);editor.undo();assert!(editor.state.in_handles.is_empty());
        editor.redo();assert_eq!(editor.state.in_handles,handles);
        editor.state.fade_in=1.0;assert!(editor.state.filter(1).contains("while("));
    }
    #[test] fn custom_curve_interpolates_and_survives_history(){
        let points=vec![(0.0,0.0),(0.25,0.8),(0.75,0.2),(1.0,1.0)];
        assert!((fade_value(0.125,"custom",&points,&[])-0.4).abs()<1e-12);
        assert!((fade_value(0.5,"custom",&points,&[])-0.5).abs()<1e-12);
        assert_eq!(fade_value(0.0,"custom",&points,&[]),0.0);
        assert_eq!(fade_value(1.0,"custom",&points,&[]),1.0);
        let mut editor=session();
        let before=editor.state.clone();
        editor.state.in_curve="custom".into();editor.state.in_points=points.clone();
        assert!(!editor.state.same_controls(&before));
        editor.undo.push(before);editor.undo();
        assert_eq!(editor.state.in_curve,"linear");
        editor.redo();assert_eq!(editor.state.in_points,points);
        assert!(fade_expression("t","custom",&points,&[]).contains("if(lte"));
    }
    #[test] fn all_fade_presets_are_bounded_monotonic_and_have_exact_endpoints(){
        for (kind,_) in FADE_CURVES {
            assert!(curve(0.0,kind).abs()<1e-12);
            assert!((curve(1.0,kind)-1.0).abs()<1e-12);
            let mut previous=0.0;
            for sample in 0..=1000 {
                let value=curve(sample as f64/1000.0,kind);
                assert!(value.is_finite()&&value>=previous-1e-12&&(-1e-12..=1.0+1e-12).contains(&value));
                previous=value;
            }
            assert!(!curve_expression("0.5",kind).is_empty());
        }
    }
    fn session()->Session {
        Session{state:State{clip:None,mode:"ends".into(),start:2.0,end:8.0,db:0.0,fade_in:0.0,fade_out:0.0,in_curve:"linear".into(),out_curve:"linear".into(),in_points:Vec::new(),out_points:Vec::new(),in_handles:Vec::new(),out_handles:Vec::new()},error:None,undo:vec![],redo:vec![],job:None,ffmpeg:PathBuf::new(),channels:1,gesture:false}
    }
    #[test] fn gain_and_fades_match_the_waveform_envelope(){
        let mut editor=session();editor.state.db=20.0*2.0_f64.log10();editor.state.fade_in=2.0;editor.state.fade_out=2.0;
        assert!((editor.state.effect_factor(2.0)-0.0).abs()<1e-8);
        assert!((editor.state.effect_factor(3.0)-1.0).abs()<1e-8);
        assert!((editor.state.effect_factor(5.0)-2.0).abs()<1e-8);
        assert!((editor.state.effect_factor(7.0)-1.0).abs()<1e-8);
        assert!((editor.state.effect_factor(8.0)-0.0).abs()<1e-8);
        assert!((editor.state.effect_factor(1.0)-1.0).abs()<1e-8);
    }
    #[test] fn gain_targets_selection_or_whole_audio_when_no_range_is_set(){
        let mut editor=session();editor.state.db=20.0*2.0_f64.log10();
        let root=std::env::temp_dir().join(format!("unity-gain-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&root).unwrap();let cache=Arc::new(Cache(root));
        editor.state.clip=Some(Arc::new(Clip{path:cache.0.join("unused.wav"),duration:8.0,cache}));
        assert!((editor.state.effect_factor(1.0)-1.0).abs()<1e-8);
        assert!((editor.state.effect_factor(5.0)-2.0).abs()<1e-8);
        assert!((editor.state.effect_factor(8.0)-1.0).abs()<1e-8);
        editor.state.mode="interval".into();editor.state.start=6.0;editor.state.end=2.0;
        assert!((editor.state.effect_factor(3.0)-2.0).abs()<1e-8);
        assert!((editor.state.effect_factor(1.0)-1.0).abs()<1e-8);
        editor.state.start=4.0;editor.state.end=4.0;
        assert_eq!(editor.state.gain_bounds(),(0.0,8.0));
        assert!((editor.state.effect_factor(1.0)-2.0).abs()<1e-8);
    }
    #[test] fn history_coalesces_a_drag_and_invalidates_redo_on_new_edit(){
        let mut editor=session();let initial=editor.state.clone();editor.state.start=3.0;editor.record_controls(initial,true);
        let previous=editor.state.clone();editor.state.start=4.0;editor.record_controls(previous,true);
        let previous=editor.state.clone();editor.record_controls(previous,false);
        assert_eq!(editor.undo.len(),1);editor.undo();assert_eq!(editor.state.start,2.0);
        editor.redo();assert_eq!(editor.state.start,4.0);editor.undo();
        let previous=editor.state.clone();editor.state.db=-3.0;editor.record_controls(previous,false);
        assert!(!editor.can_redo());editor.undo();assert_eq!(editor.state.db,0.0);
    }
    #[test] fn curve_shapes_have_matching_endpoints(){
        for name in ["linear","smooth","equal"]{assert!(curve(0.0,name).abs()<1e-8);assert!((curve(1.0,name)-1.0).abs()<1e-8);}
        assert!(curve(0.25,"smooth")<curve(0.25,"linear"));
        assert!(curve(0.5,"equal")>curve(0.5,"linear"));
    }
    #[test] fn interval_uses_the_same_span_regardless_of_click_order(){
        let mut editor=session();editor.state.mode="interval".into();
        editor.state.start=6.0;editor.state.end=3.0;
        let root=std::env::temp_dir().join(format!("unity-unused-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&root).unwrap();let cache=Arc::new(Cache(root));
        editor.state.clip=Some(Arc::new(Clip{path:cache.0.join("unused.wav"),duration:8.0,cache}));
        assert_eq!(editor.state.kept_ranges(),vec![(0.0,3.0),(6.0,8.0)]);
        editor.state.fade_in=1.0;
        assert!((editor.state.effect_factor(3.5)-0.5).abs()<1e-8);
        let filter=editor.state.filter(1);
        editor.state.start=3.0;editor.state.end=6.0;
        assert_eq!(editor.state.filter(1),filter);
        editor.state.start=2.0;editor.state.end=6.0;
        assert_eq!(editor.state.kept_ranges(),vec![(0.0,2.0),(6.0,8.0)]);
    }
    #[test] fn lossless_cut_effects_undo_and_redo_preserve_revisions(){
        let Some(ffmpeg)=crate::media_tools::find_tool("ffmpeg") else{return};
        let fixture=std::env::temp_dir().join(format!("unity-audio-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&fixture).unwrap();let fixture=Cache(fixture);let source=fixture.0.join("source.wav");
        assert!(command(&ffmpeg).args(["-nostdin","-v","error","-f","lavfi","-i","aevalsrc=0.25:s=48000:d=4","-c:a","pcm_f32le"]).arg(&source).status().unwrap().success());
        let source_bytes=std::fs::read(&source).unwrap();let ctx=egui::Context::default();
        let mut editor=Session::new(&ffmpeg,&source,4.0,0,1,&ctx).unwrap();
        fn finish(editor:&mut Session,ctx:&egui::Context){
            let started=std::time::Instant::now();
            while editor.busy(){assert!(started.elapsed().as_secs()<30,"audio worker timeout");editor.poll(ctx);std::thread::sleep(std::time::Duration::from_millis(10));}
            assert!(editor.error.is_none(),"{:?}",editor.error);
        }
        finish(&mut editor,&ctx);let base=editor.state.clip.clone().unwrap();
        editor.state.mode="interval".into();editor.state.start=1.0;editor.state.end=3.0;
        editor.apply(true,&ctx,None);finish(&mut editor,&ctx);
        let cut=editor.state.clip.clone().unwrap();assert!((cut.duration-2.0).abs()<0.001);
        editor.undo();assert_eq!(editor.state.clip.as_ref().unwrap().path,base.path);assert!(cut.path.exists());
        editor.redo();assert_eq!(editor.state.clip.as_ref().unwrap().path,cut.path);
        let before=editor.state.clone();editor.state.db=20.0*2.0_f64.log10();editor.state.fade_in=1.0;editor.state.fade_out=1.0;
        editor.apply(false,&ctx,Some(before));finish(&mut editor,&ctx);
        let fx=editor.state.clip.clone().unwrap();
        let output=command(&ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&fx.path).args(["-f","f32le","pipe:1"]).output().unwrap();
        assert!(output.status.success());
        for (index,expected) in [(0,0.0),(24000,0.25),(48000,0.5),(72000,0.25)] {
            let actual=f32::from_le_bytes(output.stdout[index*4..index*4+4].try_into().unwrap());assert!((actual-expected).abs()<0.00001);
        }
        editor.undo();assert_eq!(editor.state.clip.as_ref().unwrap().path,cut.path);
        editor.redo();assert_eq!(editor.state.clip.as_ref().unwrap().path,fx.path);
        let before=editor.state.clone();editor.state.start=0.5;editor.state.end=1.5;editor.state.db=-20.0*2.0_f64.log10();
        editor.apply(false,&ctx,Some(before));finish(&mut editor,&ctx);
        let selected=editor.state.clip.clone().unwrap();assert!((selected.duration-2.0).abs()<0.001);
        let output=command(&ffmpeg).args(["-nostdin","-v","error","-i"]).arg(&selected.path).args(["-f","f32le","pipe:1"]).output().unwrap();
        assert!(output.status.success());
        for (index,expected) in [(12000,0.125),(24000,0.125),(48000,0.25),(72000,0.25)] {
            let actual=f32::from_le_bytes(output.stdout[index*4..index*4+4].try_into().unwrap());assert!((actual-expected).abs()<0.00001,"sample {index}: {actual}");
        }
        editor.undo();assert_eq!(editor.state.clip.as_ref().unwrap().path,fx.path);
        editor.redo();assert_eq!(editor.state.clip.as_ref().unwrap().path,selected.path);
        assert_eq!(std::fs::read(source).unwrap(),source_bytes);
    }
}
