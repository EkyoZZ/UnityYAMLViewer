//! Bounded, deterministic scene/Prefab particle preview; no Unity scripts run.
use std::sync::Arc;
use crate::{animation_curves::ScalarSampler,gradient_property::GradientSampler,render_system::scene::{Draw,Vertex},static_preview::Matrix};
pub(crate) const MAX_PARTICLES:usize=4096;
pub(crate) fn section<'a>(text:&'a str,key:&str)->Option<&'a str>{
    let mut offset=0;let mut start=None;let mut indent=0;
    for line in text.split_inclusive('\n') {
        let trimmed=line.trim();let depth=line.len()-line.trim_start().len();
        if let Some(start)=start {if !trimmed.is_empty() && depth<=indent && !trimmed.starts_with('-'){return Some(&text[start..offset]);}}
        else if trimmed.strip_prefix(key).is_some_and(|v|v.starts_with(':')){start=Some(offset);indent=depth;}
        offset+=line.len();
    }start.map(|start|&text[start..])
}
fn value<'a>(text:&'a str,key:&str)->Option<&'a str>{section(text,key)?.lines().next()?.trim().split_once(':').map(|(_,v)|v.trim())}
fn num(text:&str,key:&str,default:f64)->Result<f64,String>{let v=value(text,key).map_or(Ok(default),|v|v.parse::<f64>().map_err(|_|format!("Invalid particle {key}")))?;if !v.is_finite(){return Err(format!("Non-finite particle {key}"));}Ok(v)}
fn enabled(text:&str)->bool{value(text,"enabled")==Some("1")}
fn list_items(text:&str)->Vec<&str>{let mut entries=Vec::new();let mut offset=0;for line in text.split_inclusive('\n'){if line.trim_start().starts_with("- serializedVersion:"){entries.push((offset,line.len()-line.trim_start().len()));}offset+=line.len();}let Some(indent)=entries.iter().map(|e|e.1).min() else{return Vec::new();};let starts=entries.iter().filter(|e|e.1==indent).map(|e|e.0).collect::<Vec<_>>();starts.iter().enumerate().map(|(i,start)|&text[*start..starts.get(i+1).copied().unwrap_or(text.len())]).collect()}
struct Scalar {mode:u8,max:f64,min:f64,upper:Option<ScalarSampler>,lower:Option<ScalarSampler>}
impl Scalar {
    fn parse(text:Option<&str>,default:f64)->Result<Self,String>{let text=text.unwrap_or("");let mode=num(text,"minMaxState",0.0)? as u8;if mode>3{return Err("Unsupported particle MinMaxCurve mode".into());}let curve=|key|section(text,key).and_then(ScalarSampler::parse);let result=Self{mode,max:num(text,"scalar",default)?,min:num(text,"minScalar",default)?,upper:curve("maxCurve"),lower:curve("minCurve")};if (mode==1 || mode==2) && (result.upper.is_none() || (mode==2 && result.lower.is_none())){return Err("Particle curve data unavailable".into());}Ok(result)}
    fn at(&self,t:f64,r:f64)->f64{match self.mode{1=>self.max*self.upper.as_ref().unwrap().at(t),2=>{let a=self.lower.as_ref().unwrap().at(t)*self.max;let b=self.upper.as_ref().unwrap().at(t)*self.max;a+(b-a)*r},3=>self.min+(self.max-self.min)*r,_=>self.max}}
}
struct Color {mode:u8,max:[f64;4],min:[f64;4],upper:Option<GradientSampler>,lower:Option<GradientSampler>}
impl Color {
    fn parse(text:Option<&str>)->Result<Self,String>{let text=text.unwrap_or("");let mode=num(text,"minMaxState",0.0)? as u8;if mode>3{return Err("Particle random-color gradient is not implemented".into());}let rgba=|key|value(text,key).and_then(|v|crate::inline_vector(v,&["r","g","b","a"])).map(|v|std::array::from_fn(|i|v[i])).unwrap_or([1.0;4]);let result=Self{mode,max:rgba("maxColor"),min:rgba("minColor"),upper:section(text,"maxGradient").and_then(GradientSampler::parse),lower:section(text,"minGradient").and_then(GradientSampler::parse)};if (mode==1||mode==3)&&(result.upper.is_none()||(mode==3&&result.lower.is_none())){return Err("Particle color gradient unavailable".into());}Ok(result)}
    fn at(&self,t:f64,r:f64)->[f64;4]{match self.mode{1=>self.upper.as_ref().unwrap().at(t),2=>std::array::from_fn(|i|self.min[i]+(self.max[i]-self.min[i])*r),3=>{let a=self.lower.as_ref().unwrap().at(t);let b=self.upper.as_ref().unwrap().at(t);std::array::from_fn(|i|a[i]+(b[i]-a[i])*r)},_=>self.max}}
}
struct Burst {time:f64,count:Scalar,cycles:usize,interval:f64,probability:f64}
#[derive(Clone)] struct Particle {birth:f64,life:f64,position:[f64;3],velocity:[f64;3],size:[f64;2],rotation:f64,color:[f64;4],random:f64}
struct Shape {kind:i32,radius:f64,length:f64,angle:f64,position:[f64;3],rotation:[f64;3],scale:[f64;3],thickness:f64}
struct WarmState {time:f64,remaining:f64,fraction:f64,seed:u64,started:bool,particles:Vec<Particle>}
pub(crate) struct Emitter {
    pub id:String,pub warnings:Vec<String>,matrix:Matrix,duration:f64,delay:f64,speed:f64,looping:bool,max:usize,seed:u64,initial_seed:u64,
    lifetime:Scalar,start_speed:Scalar,size:Scalar,size_y:Option<Scalar>,rotation:Scalar,color:Color,gravity:f64,rate:Scalar,bursts:Vec<Burst>,shape:Option<Shape>,size_life:Option<Scalar>,color_life:Option<Color>,rotation_life:Option<Scalar>,
    time:f64,remaining:f64,emission_fraction:f64,particles:Vec<Particle>,started:bool,scale_size:bool,pivot:[f64;3],active:bool,warm:Option<WarmState>,
}
impl Emitter {
    pub fn bounds_radius(&self)->f64{let maximum=|s:&Scalar|(0..=8).flat_map(|i|[s.at(i as f64/8.0,0.0).abs(),s.at(i as f64/8.0,1.0).abs()]).fold(0.0,f64::max);let shape=self.shape.as_ref().map_or(0.0,|s|s.radius+s.position.iter().map(|v|v.abs()).sum::<f64>()+s.scale.iter().map(|v|v.abs()).fold(0.0,f64::max));(maximum(&self.size)+maximum(&self.start_speed)*maximum(&self.lifetime).min(120.0)+shape).clamp(1.0,10000.0)}
    pub fn parse(id:String,text:&str,renderer:&str,matrix:Matrix)->Result<Self,String>{
        if num(renderer,"m_RenderMode",0.0)?!=0.0 || num(renderer,"m_RenderAlignment",0.0)?!=0.0{return Err("Particle preview currently requires View Billboard rendering".into());}
        if num(renderer,"m_UseCustomVertexStreams",0.0)?!=0.0{return Err("Particle Custom Vertex Streams are not implemented".into());}
        if num(text,"ringBufferMode",0.0)?!=0.0{return Err("Particle ring buffer is not implemented".into());}
        if num(text,"moveWithTransform",0.0)?==2.0{return Err("Custom particle simulation transform is not implemented".into());}
        let scaling=num(text,"scalingMode",0.0)?;if scaling==1.0{return Err("Local-only particle scaling mode is not implemented".into());}
        let initial=section(text,"InitialModule").ok_or("Missing particle InitialModule")?;
        if num(initial,"rotation3D",0.0)?!=0.0{return Err("3D particle start rotation is not implemented".into());}
        let duration=num(text,"lengthInSec",5.0)?;if duration<0.001 || duration>3600.0{return Err("Particle duration outside preview range 0.001..3600 seconds".into());}
        let mut warnings=Vec::new();
        for line in text.lines().filter(|l|l.starts_with("  ") && !l.starts_with("   ") && l.trim().ends_with("Module:")) {let key=line.trim().trim_end_matches(':');if !matches!(key,"InitialModule"|"EmissionModule"|"ShapeModule"|"SizeModule"|"ColorModule"|"RotationModule") && section(text,key).is_some_and(enabled){warnings.push(format!("Particle {id}: {key} is not reproduced"));}}
        let emission=section(text,"EmissionModule").filter(|v|enabled(v)).unwrap_or("");
        if Scalar::parse(section(emission,"rateOverDistance"),0.0)?.at(0.0,0.5)!=0.0{warnings.push(format!("Particle {id}: rate over distance requires emitter motion and is not reproduced"));}
        let mut bursts=Vec::new();
        if let Some(list)=section(emission,"m_Bursts") {for block in list_items(list){if bursts.len()>=64{return Err("Particle burst count exceeds 64".into());}let time=num(block,"time",0.0)?;let cycles=num(block,"cycleCount",1.0)? as usize;let interval=num(block,"repeatInterval",0.01)?;if time<0.0 || cycles>256 || interval<=0.0{return Err("Particle burst schedule exceeds preview limits".into());}bursts.push(Burst{time,count:Scalar::parse(section(block,"countCurve"),1.0)?,cycles,interval,probability:num(block,"probability",1.0)?.clamp(0.0,1.0)});}}
        let vector=|text:&str,key:&str,default:[f64;3]|->Result<[f64;3],String>{let result=value(text,key).and_then(|v|crate::inline_vector(v,&["x","y","z"])).map(|v|[v[0],v[1],v[2]]).unwrap_or(default);if result.iter().any(|v|!v.is_finite()){return Err(format!("Invalid particle vector {key}"));}Ok(result)};
        let shape=if let Some(shape)=section(text,"ShapeModule").filter(|v|enabled(v)){let kind=num(shape,"type",0.0)? as i32;if !matches!(kind,0|4|5|8|10|18){return Err(format!("Particle shape {kind} is not implemented"));}if num(shape,"alignToDirection",0.0)?!=0.0{return Err("Particle shape direction alignment is not implemented".into());}Some(Shape{kind,length:num(shape,"length",1.0)?.max(0.0),radius:if value(shape,"radius")==Some(""){let radius=section(shape,"radius").unwrap();if num(radius,"mode",0.0)?!=0.0{return Err("Animated particle shape radius is not implemented".into());}num(radius,"value",1.0)?.max(0.0)}else{num(shape,"radius",1.0)?.max(0.0)},angle:num(shape,"angle",25.0)?.to_radians(),position:vector(shape,"m_Position",[0.0;3])?,rotation:vector(shape,"m_Rotation",[0.0;3])?.map(f64::to_radians),scale:vector(shape,"m_Scale",[1.0;3])?,thickness:num(shape,"radiusThickness",1.0)?.clamp(0.0,1.0)})}else{None};
        if let Some(shape)=section(text,"ShapeModule").filter(|v|enabled(v)){if let Some(arc)=section(shape,"arc"){if num(arc,"mode",0.0)?!=0.0 || num(arc,"value",360.0)?!=360.0{return Err("Partial / animated particle shape arc is not implemented".into());}}for key in ["randomDirectionAmount","sphericalDirectionAmount","randomPositionAmount"]{if num(shape,key,0.0)?!=0.0{return Err(format!("Particle shape {key} is not implemented"));}}}
        if section(text,"RotationModule").filter(|v|enabled(v)).is_some_and(|v|value(v,"separateAxes")==Some("1")){return Err("Separate-axis particle rotation over lifetime is not implemented".into());}
        let module_scalar=|module,key|section(text,module).filter(|v|enabled(v)).map(|v|Scalar::parse(section(v,key),1.0)).transpose();
        if section(text,"SizeModule").filter(|v|enabled(v)).is_some_and(|v|value(v,"separateAxes")==Some("1")){return Err("Separate-axis particle size over lifetime is not implemented".into());}
        let gravity=Scalar::parse(section(initial,"gravityModifier"),0.0)?;if gravity.mode!=0{return Err("Variable particle gravity is not implemented".into());}
        let delay=Scalar::parse(section(text,"startDelay"),0.0)?;if delay.mode!=0{return Err("Variable particle start delay is not implemented".into());}
        let seed=num(text,"randomSeed",1.0)?.max(1.0) as u64;
        let max=num(initial,"maxNumParticles",1000.0)?;if max<0.0{return Err("Invalid particle maximum count".into());}if max>MAX_PARTICLES as f64{warnings.push(format!("Particle {id}: max particles capped at {MAX_PARTICLES}"));}
        if num(text,"autoRandomSeed",0.0)?!=0.0{warnings.push(format!("Particle {id}: preview uses a fixed seed for repeatable replay"));}
        let speed=num(text,"simulationSpeed",1.0)?;if !(0.0..=2.0).contains(&speed){return Err("Particle simulation speed outside preview range 0..2".into());}
        let mut emitter=Self{id,warnings,matrix,duration,delay:delay.max.max(0.0),speed,looping:num(text,"looping",1.0)?!=0.0,max:(max as usize).min(MAX_PARTICLES),seed,initial_seed:seed,lifetime:Scalar::parse(section(initial,"startLifetime"),5.0)?,start_speed:Scalar::parse(section(initial,"startSpeed"),5.0)?,size:Scalar::parse(section(initial,"startSize"),1.0)?,size_y:if num(initial,"size3D",0.0)?!=0.0 {Some(Scalar::parse(section(initial,"startSizeY"),1.0)?)}else{None},rotation:Scalar::parse(section(initial,"startRotation"),0.0)?,color:Color::parse(section(initial,"startColor"))?,gravity:gravity.max* -9.81,rate:Scalar::parse(section(emission,"rateOverTime"),0.0)?,bursts,shape,size_life:module_scalar("SizeModule","curve")?,color_life:section(text,"ColorModule").filter(|v|enabled(v)).map(|v|Color::parse(section(v,"gradient"))).transpose()?,rotation_life:module_scalar("RotationModule","curve")?,time:0.0,remaining:0.0,emission_fraction:0.0,particles:Vec::new(),started:false,scale_size:scaling==0.0,pivot:vector(renderer,"m_Pivot",[0.0;3])?,active:num(text,"playOnAwake",1.0)?!=0.0,warm:None};
        if num(text,"prewarm",0.0)?!=0.0 && emitter.looping {
            if emitter.speed<=0.0 || emitter.duration>60.0 || emitter.duration*60.0*emitter.max as f64>2_000_000.0{return Err("Particle prewarm exceeds 60-second / 2,000,000 particle-step budget".into());}
            let active=emitter.active;emitter.active=true;
            for _ in 0..=(emitter.duration*60.0).ceil() as usize {if emitter.time+1e-8>=emitter.duration{break;}emitter.advance((1.0/60.0)/emitter.speed);}
            emitter.warm=Some(WarmState{time:emitter.time,remaining:emitter.remaining,fraction:emitter.emission_fraction,seed:emitter.seed,started:emitter.started,particles:emitter.particles.clone()});
            emitter.active=active;if !active{emitter.particles.clear();emitter.time=0.0;emitter.remaining=0.0;emitter.emission_fraction=0.0;emitter.started=false;emitter.seed=emitter.initial_seed;}
        }
        Ok(emitter)
    }
    pub fn restart(&mut self){if let Some(warm)=&self.warm{self.time=warm.time;self.remaining=warm.remaining;self.emission_fraction=warm.fraction;self.seed=warm.seed;self.started=warm.started;self.particles=warm.particles.clone();self.active=true;return;}self.time=0.0;self.remaining=0.0;self.emission_fraction=0.0;self.started=false;self.particles.clear();self.seed=self.initial_seed;self.active=true;}
    fn random(&mut self)->f64{self.seed^=self.seed<<13;self.seed^=self.seed>>7;self.seed^=self.seed<<17;(self.seed>>11) as f64/(1u64<<53) as f64}
    fn spawn(&mut self,birth:f64){if self.particles.len()>=self.max{return;}let r=self.random();let t=((birth-self.delay)/self.duration).rem_euclid(1.0);let life=self.lifetime.at(t,r).clamp(0.0,120.0);if life<=0.0{return;}let mut position=[0.0;3];let mut direction=[0.0,0.0,1.0];let a=self.random()*std::f64::consts::TAU;let v=self.random();let w=self.random();
        if let Some(shape)=&self.shape {let radius=shape.radius*((1.0-shape.thickness).powi(2)+(1.0-(1.0-shape.thickness).powi(2))*v).sqrt();match shape.kind{0=>{let z=2.0*v-1.0;let xy=(1.0-z*z).sqrt();direction=[a.cos()*xy,a.sin()*xy,z];position=direction.map(|d|d*shape.radius*((1.0-shape.thickness).powi(3)+(1.0-(1.0-shape.thickness).powi(3))*w).cbrt());},4|8=>{let z=if shape.kind==8 {let tan=shape.angle.tan();if tan.abs()<1e-9{shape.length*w}else{let top=shape.radius+shape.length*tan;((shape.radius.powi(3)+w*(top.powi(3)-shape.radius.powi(3))).cbrt()-shape.radius)/tan}}else{0.0};let outer=shape.radius+z*shape.angle.tan();let radius=outer*((1.0-shape.thickness).powi(2)+(1.0-(1.0-shape.thickness).powi(2))*v).sqrt();position=[a.cos()*radius,a.sin()*radius,z];let angle=shape.angle*(radius/outer.max(1e-9));direction=[a.cos()*angle.sin(),a.sin()*angle.sin(),angle.cos()];},5=>position=[r-0.5,v-0.5,w-0.5],10=>{position=[a.cos()*radius,a.sin()*radius,0.0];direction=[a.cos(),a.sin(),0.0];},18=>position=[r-0.5,v-0.5,0.0],_=>{}}position=rotate(std::array::from_fn(|i|position[i]*shape.scale[i]),shape.rotation);for i in 0..3{position[i]+=shape.position[i];}direction=rotate(direction,shape.rotation);}
        let speed=self.start_speed.at(t,r);let velocity=direction.map(|v|v*speed);self.particles.push(Particle{birth,life,position,velocity,size:[self.size.at(t,r).max(0.0),self.size_y.as_ref().map_or_else(||self.size.at(t,r),|s|s.at(t,r)).max(0.0)],rotation:self.rotation.at(t,r),color:self.color.at(t,r),random:r});
    }
    pub fn advance(&mut self,dt:f64){if !self.active{return;}self.remaining+=(dt.clamp(0.0,0.25)*self.speed).min(0.5);let step=1.0/60.0;for _ in 0..30{if self.remaining+1e-9<step{break;}self.remaining-=step;let previous=self.time;self.time+=step;self.particles.retain(|p|self.time-p.birth<p.life);let mut events=Vec::new();let first=((previous-self.delay).max(0.0)/self.duration).floor() as usize;let last=((self.time-self.delay).max(0.0)/self.duration).floor() as usize;for cycle in first..=last{if cycle>0&&!self.looping{continue;}let base=self.delay+cycle as f64*self.duration;let active_start=previous.max(base);let active_end=self.time.min(base+self.duration);if active_end>active_start{let rate=self.rate.at(((active_start+active_end)*0.5-base)/self.duration,0.5).clamp(0.0,10000.0);self.emission_fraction+=rate*(active_end-active_start);let count=(self.emission_fraction+1e-9).floor().min(MAX_PARTICLES as f64) as usize;self.emission_fraction=(self.emission_fraction-count as f64).max(0.0);for _ in 0..count{events.push((active_end,1,1.0,None));}}
            for (burst_index,burst) in self.bursts.iter().enumerate(){for repeat in 0..burst.cycles{let birth=base+burst.time+repeat as f64*burst.interval;if birth>=base+self.duration{break;}if birth<=self.time && (birth>previous || (!self.started&&birth==0.0)){events.push((birth,0,burst.probability,Some((burst_index,(birth-base)/self.duration))));}}}}
            self.started=true;for (birth,count,probability,burst) in events{let chance=self.random();if probability>0.0 && chance<probability{let random=self.random();let count=burst.map_or(count,|(index,time)|self.bursts[index].count.at(time,random).clamp(0.0,MAX_PARTICLES as f64).round() as usize);for _ in 0..count{self.spawn(birth);}}}
        }}
    pub fn draw(&self,template:&Draw)->Draw{
        let mut vertices=Vec::with_capacity(self.particles.len()*4);let mut indices=Vec::with_capacity(self.particles.len()*6);
        for p in &self.particles{let age=(self.time-p.birth).max(0.0);let t=(age/p.life).clamp(0.0,1.0);let local=std::array::from_fn::<_,3,_>(|i|p.position[i]+p.velocity[i]*age);let center: [f64;3]=std::array::from_fn(|i|(0..3).map(|j|self.matrix.0[i][j]*local[j]).sum::<f64>()+self.matrix.0[i][3]+if i==1{0.5*self.gravity*age*age}else{0.0});let size=p.size.map(|v|v*self.size_life.as_ref().map_or(1.0,|c|c.at(t,p.random)).max(0.0));let color_life=self.color_life.as_ref().map_or([1.0;4],|c|c.at(t,p.random));let color=std::array::from_fn::<_,4,_>(|i|(p.color[i]*color_life[i]).clamp(0.0,1.0) as f32);
            let rotation=p.rotation+self.rotation_life.as_ref().map_or(0.0,|c|{let step=t/16.0;(0..16).map(|i|c.at((i as f64+0.5)*step,p.random)*step*p.life).sum::<f64>()});let (sin,cos)=(-rotation).sin_cos();let scale=if self.scale_size{[0,1].map(|a|(0..3).map(|i|self.matrix.0[i][a].powi(2)).sum::<f64>().sqrt())}else{[1.0;2]};let start=vertices.len() as u32;for (corner,uv) in [[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]].into_iter().zip([[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]]){let x=(corner[0]-self.pivot[0])*size[0]*scale[0];let y=(corner[1]-self.pivot[1])*size[1]*scale[1];vertices.push(Vertex{position:[(center[0]+x*cos-y*sin) as f32,(center[1]+x*sin+y*cos) as f32,center[2] as f32],uv,color,normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]});}indices.extend([start,start+1,start+2,start,start+2,start+3]);}
        Draw{component:template.component.clone(),vertices:Arc::from(vertices),indices:Arc::from(indices),object_to_world:[[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]],material:template.material.clone(),main_texture:template.main_texture.clone()}
    }
}
fn rotate(mut p:[f64;3],r:[f64;3])->[f64;3]{for axis in [2,0,1]{let (a,b)=match axis{0=>(1,2),1=>(2,0),_=>(0,1)};let (s,c)=r[axis].sin_cos();let x=p[a]*c-p[b]*s;p[b]=p[a]*s+p[b]*c;p[a]=x;}p}

#[cfg(test)]
mod tests {
    #[test]
    fn prewarm_cache_replays_without_repeating_simulation_work(){
        let system=SYSTEM.replace("  looping: 0","  looping: 1\n  prewarm: 1").replace("scalar: 0.2","scalar: 2");
        let mut emitter=Emitter::parse("renderer".into(),&system,"",Matrix::identity()).unwrap();
        assert!(emitter.time>=0.99 && !emitter.particles.is_empty());
        let seed=emitter.seed;let count=emitter.particles.len();let random=emitter.particles[0].random;
        emitter.advance(0.25);emitter.restart();assert_eq!(emitter.seed,seed);assert_eq!(emitter.particles.len(),count);assert_eq!(emitter.particles[0].random,random);
    }
    #[test]
    #[ignore="Requires the user's g003 project"]
    fn real_lighthouse_prewarmed_systems_parse_and_emit(){
        let text=std::fs::read_to_string("C:/Git/g003_client/Assets/Res/Prefabs/Effect/building_night_lighthouse_01_effect.prefab").unwrap();let docs=crate::parse_unity_yaml(&text);let mut emitted=0;
        for system in docs.iter().filter(|d|d.type_name=="ParticleSystem") {
            let renderer=docs.iter().find(|d|d.type_name=="ParticleSystemRenderer" && d.game_object==system.game_object).unwrap();
            let mut emitter=Emitter::parse(renderer.file_id.clone(),&system.text,&renderer.text,Matrix::identity()).unwrap_or_else(|e|panic!("{}: {e}",system.file_id));
            assert!(emitter.warm.is_some());emitter.advance(0.25);println!("{} time={} alive={} rate={} bursts={} max={} active={}",system.file_id,emitter.time,emitter.particles.len(),emitter.rate.at(0.5,0.5),emitter.bursts.len(),emitter.max,emitter.active);if !emitter.particles.is_empty(){emitted+=1;}
        }assert!(emitted>=4);
    }
    use super::*;
    const SYSTEM:&str="ParticleSystem:\n  lengthInSec: 1\n  looping: 0\n  randomSeed: 42\n  InitialModule:\n    enabled: 1\n    maxNumParticles: 10\n    startLifetime:\n      scalar: 0.2\n    startSpeed:\n      scalar: 0\n    startSize:\n      scalar: 2\n  EmissionModule:\n    enabled: 1\n    rateOverTime:\n      scalar: 0\n    m_Bursts:\n    - serializedVersion: 2\n      time: 0\n      countCurve:\n        minMaxState: 0\n        scalar: 2\n        maxCurve:\n          m_Curve:\n          - serializedVersion: 3\n            time: 0\n            value: 1\n          - serializedVersion: 3\n            time: 1\n            value: 1\n      cycleCount: 1\n      repeatInterval: 0.01\n      probability: 1\n";
    #[test]
    fn nested_curve_keys_do_not_become_bursts_and_replay_is_deterministic(){
        let mut emitter=Emitter::parse("renderer".into(),SYSTEM,"",Matrix::identity()).unwrap();
        assert_eq!(emitter.bursts.len(),1);
        emitter.advance(1.0/60.0);assert_eq!(emitter.particles.len(),2);
        let first=emitter.particles[0].random;
        emitter.advance(0.25);assert!(emitter.particles.is_empty());
        emitter.restart();emitter.advance(1.0/60.0);assert_eq!(emitter.particles.len(),2);assert_eq!(emitter.particles[0].random,first);
    }
    #[test]
    fn emission_rate_duration_and_disabled_modules_are_respected(){
        let system=SYSTEM.replace("scalar: 0\n    m_Bursts:","scalar: 60\n    m_Bursts:").replace("scalar: 0.2","scalar: 2").replace("maxNumParticles: 10","maxNumParticles: 100");
        let mut emitter=Emitter::parse("renderer".into(),&system,"",Matrix::identity()).unwrap();
        emitter.advance(0.25);assert_eq!(emitter.particles.len(),17);
        for _ in 0..4{emitter.advance(0.25);}let count=emitter.particles.len();assert!(count>=60 && count<=63);
        emitter.advance(0.25);assert_eq!(emitter.particles.len(),count);
        assert!(Emitter::parse("renderer".into(),SYSTEM,"  m_RenderMode: 4",Matrix::identity()).is_err());
        let mut disabled=Emitter::parse("renderer".into(),&SYSTEM.replace("  randomSeed: 42","  randomSeed: 42\n  playOnAwake: 0"),"",Matrix::identity()).unwrap();disabled.advance(0.1);assert!(disabled.particles.is_empty());disabled.restart();disabled.advance(1.0/60.0);assert_eq!(disabled.particles.len(),2);
    }
    #[test]
    fn particle_draw_preserves_straight_vertex_color_for_original_shader(){
        use crate::render_system::scene::{Material,Program,RenderState,BlendFactor,UnityPipeline,Texture};
        let program=Arc::new(Program{pipeline:UnityPipeline::BuiltIn,vertex:Arc::from(""),fragment:Arc::from(""),revision:1});
        let material=Arc::new(Material{program,state:RenderState{source:BlendFactor::One,destination:BlendFactor::OneMinusSourceAlpha,color_mask:[true;4],depth_write:false,cull_back:false},uniforms:Default::default(),textures:Default::default()});
        let texture=Arc::new(Texture{size:[1,1],rgba:Arc::from([255;4]),repeat:[false;2],revision:1});
        let template=Draw{component:Arc::from("renderer"),vertices:Arc::from([]),indices:Arc::from([]),object_to_world:[[0.0;4];4],material,main_texture:texture};
        let mut emitter=Emitter::parse("renderer".into(),SYSTEM,"",Matrix::identity()).unwrap();emitter.advance(1.0/60.0);
        emitter.particles[0].color=[1.0,0.5,0.25,0.5];let draw=emitter.draw(&template);
        assert_eq!(draw.vertices.len(),8);assert_eq!(draw.indices.len(),12);
        assert_eq!(draw.vertices[0].color,[1.0,0.5,0.25,0.5]);assert_eq!(draw.vertices[0].uv,[0.0,0.0]);
        assert_eq!(draw.vertices[0].position,[-1.0,-1.0,0.0]);
    }
    #[test]
    #[ignore="Requires the user's g003 project"]
    fn real_newitem_prefab_initial_burst_and_lifetime(){
        let text=std::fs::read_to_string("C:/Git/g003_client/Assets/Res/Prefabs/Effect/board_item_newitem_effect.prefab").unwrap();let docs=crate::parse_unity_yaml(&text);
        let system=docs.iter().find(|d|d.type_name=="ParticleSystem").unwrap();let renderer=docs.iter().find(|d|d.type_name=="ParticleSystemRenderer" && d.game_object==system.game_object).unwrap();
        let mut emitter=Emitter::parse(renderer.file_id.clone(),&system.text,&renderer.text,Matrix::identity()).unwrap();assert_eq!(emitter.bursts.len(),1);emitter.advance(1.0/60.0);assert_eq!(emitter.particles.len(),1);
        assert!(emitter.particles[0].life>0.59 && emitter.particles[0].life<0.61);for _ in 0..4{emitter.advance(0.25);}assert!(emitter.particles.is_empty());
    }
}
