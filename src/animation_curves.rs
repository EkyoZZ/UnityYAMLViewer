//! Unity animation key blocks and curve evaluation. Preserve unrelated YAML verbatim.
use std::collections::BTreeSet;
use eframe::egui::{self, Color32, Stroke};
use crate::{AnimationBinding, SelectedKeyframe, editing, localization::tr};
#[derive(Clone)]
struct Block { start: usize, end: usize, time_line: usize, parent: usize, time: f64, fields: Vec<(String,String)> }
pub(crate) struct ScalarSampler(Vec<Block>);
impl ScalarSampler {
    pub(crate) fn parse(text:&str)->Option<Self>{let mut keys=blocks(text);keys.sort_by(|a,b|a.time.total_cmp(&b.time));(!keys.is_empty()).then_some(Self(keys))}
    pub(crate) fn at(&self,time:f64)->f64{let first=&self.0[0];let last=self.0.last().unwrap();if time<=first.time{return number(first,"value",None).unwrap_or(0.0);}if time>=last.time{return number(last,"value",None).unwrap_or(0.0);}let a=self.0.iter().rev().find(|k|k.time<=time).unwrap();let b=self.0.iter().find(|k|k.time>=time).unwrap();evaluate(a,b,time,None).map_or(0.0,|v|v.0)}
}
fn blocks(text: &str) -> Vec<Block> {
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let mut result = Vec::new(); let mut parent = 0;
    for (i,line) in lines.iter().enumerate() {
        if line.trim() == "m_Curve:" { parent = i; }
        if !line.trim().starts_with("time:") || i == 0 || !lines[i-1].trim().starts_with("- serializedVersion:") { continue; }
        let indent = lines[i-1].len()-lines[i-1].trim_start().len();
        let end = (i+1..lines.len()).find(|j| !lines[*j].trim().is_empty() && lines[*j].len()-lines[*j].trim_start().len() <= indent).unwrap_or(lines.len());
        let fields = lines[i..end].iter().filter_map(|line| line.trim().split_once(':').map(|(k,v)| (k.to_string(),v.trim().to_string()))).collect();
        if let Some(time) = line.trim().strip_prefix("time:").and_then(|s| s.trim().parse::<f64>().ok()).filter(|t| t.is_finite()) {
            result.push(Block {start:i-1,end,time_line:i,parent,time,fields});
        }
    } result
}
fn field<'a>(block: &'a Block, name: &str) -> &'a str { block.fields.iter().find(|(k,_)| k == name).map_or("",|(_,v)| v) }
fn number(block: &Block, name: &str, component: Option<&str>) -> Option<f64> {
    let raw = field(block,name);
    if let Some(c) = component { crate::inline_vector(raw,&[c]).and_then(|v| v.first().copied()) } else { raw.parse().ok() }
}
fn components(raw: &str) -> Vec<&str> { if raw.starts_with('{') { raw.trim_matches(['{','}']).split(',').filter_map(|s| s.trim().split_once(':').map(|(k,_)|k)).collect() } else { Vec::new() } }
fn evaluate(a: &Block,b: &Block,time: f64,c: Option<&str>) -> Option<(f64,f64)> {
    let v0=number(a,"value",c)?; let v1=number(b,"value",c)?;
    let dt=b.time-a.time;
    if dt <= 0.0 {return Some((v0,0.0))}
    let m0=number(a,"outSlope",c).unwrap_or((v1-v0)/dt);
    let m1=number(b,"inSlope",c).unwrap_or((v1-v0)/dt);
    if !m0.is_finite() || !m1.is_finite() { return Some((if time < b.time {v0} else {v1},f64::INFINITY)); }
    let mode0=field(a,"weightedMode").parse::<u32>().unwrap_or(0);
    let mode1=field(b,"weightedMode").parse::<u32>().unwrap_or(0);
    let w0=if mode0 & 2 != 0 {number(a,"outWeight",c).unwrap_or(1.0/3.0).clamp(0.0,1.0)} else {1.0/3.0};
    let w1=if mode1 & 1 != 0 {number(b,"inWeight",c).unwrap_or(1.0/3.0).clamp(0.0,1.0)} else {1.0/3.0};
    let target=((time-a.time)/dt).clamp(0.0,1.0);
    let bezier=|u:f64,p:f64,q:f64,r:f64,s:f64| {let t=1.0-u;t*t*t*p+3.0*t*t*u*q+3.0*t*u*u*r+u*u*u*s};
    let derivative=|u:f64,p:f64,q:f64,r:f64,s:f64| 3.0*((1.0-u).powi(2)*(q-p)+2.0*(1.0-u)*u*(r-q)+u*u*(s-r));
    let (mut lo,mut hi)=(0.0,1.0);
    for _ in 0..48 { let mid=(lo+hi)*0.5;if bezier(mid,0.0,w0,1.0-w1,1.0)<target {lo=mid}else{hi=mid} }
    let u=(lo+hi)*0.5; let p1=v0+m0*dt*w0;let p2=v1-m1*dt*w1;
    let dx=derivative(u,0.0,w0,1.0-w1,1.0)*dt;
    Some((bezier(u,v0,p1,p2,v1), if dx.abs()>1e-12 {derivative(u,v0,p1,p2,v1)/dx} else {0.0}))
}
fn sample(keys: &[&Block],time:f64,c:Option<&str>) -> Option<(f64,f64)> {
    let first=*keys.first()?;let last=*keys.last()?;
    if time<=first.time {return Some((number(first,"value",c)?,0.0))}
    if time>=last.time {return Some((number(last,"value",c)?,0.0))}
    let a=*keys.iter().rev().find(|k|k.time<=time)?;let b=*keys.iter().find(|k|k.time>=time)?;
    evaluate(a,b,time,c)
}
fn replace_fields(lines:&[&str],block:&Block,changes:&[(String,String)]) -> String {
    lines[block.start..block.end].iter().map(|line| {
        if let Some((key,_))=line.trim().split_once(':') && let Some((_,value))=changes.iter().find(|(k,_)|k==key) {
            let colon=line.find(':').unwrap();let newline=if line.ends_with("\r\n"){"\r\n"}else if line.ends_with('\n'){"\n"}else{""};
            format!("{}: {}{}",&line[..colon],value,newline)
        } else {line.to_string()}
    }).collect()
}
fn apply(text:&str,mut changes:Vec<(usize,usize,String)>) -> String {
    let lines:Vec<_>=text.split_inclusive('\n').collect();let mut offsets=vec![0];for line in &lines {offsets.push(offsets.last().unwrap()+line.len())}
    changes.sort_by_key(|c|(c.0,c.1));let mut result=text.to_owned();for (start,end,value) in changes.into_iter().rev(){result.replace_range(offsets[start]..offsets[end],&value)}result
}
pub fn value_at(text:&str,binding:&AnimationBinding,time:f32)->Option<String> {
    let all=blocks(text);let keys:Vec<_>=binding.keyframes.iter().filter_map(|k|all.iter().find(|b|b.time_line==k.yaml_line)).collect();
    sample(&keys,time as f64,None).filter(|(v,_)|v.is_finite()).map(|(v,_)|(v as f32).to_string())
}
fn deletion_lines(bindings:&[&AnimationBinding],selected:&BTreeSet<usize>)->BTreeSet<usize> {
    let aliases:BTreeSet<_>=bindings.iter().flat_map(|b|b.keyframes.iter()).filter(|k|selected.contains(&k.yaml_line)).flat_map(|k|k.time_lines.iter().copied()).collect();
    bindings.iter().flat_map(|b|b.keyframes.iter()).filter(|k|k.time_lines.iter().any(|line|aliases.contains(line))).flat_map(|k|k.time_lines.iter().copied()).collect()
}
pub fn delete(ui:&egui::Ui,lines:&BTreeSet<usize>) -> bool {
    editing::change_animation_document(ui,|text|{
        let all=blocks(text);let chosen:Vec<_>=all.iter().filter(|b|lines.contains(&b.time_line)).collect();
        if chosen.is_empty(){return None}
        let mut changes=Vec::new();
        for parent in chosen.iter().map(|b|b.parent).collect::<BTreeSet<_>>() {
            let keys:Vec<_>=all.iter().filter(|b|b.parent==parent).collect();
            if keys.iter().all(|b|lines.contains(&b.time_line)) {
                let raw:Vec<_>=text.split_inclusive('\n').collect();let newline=if text.contains("\r\n"){"\r\n"}else{"\n"};
                changes.push((parent,keys.last()?.end,format!("{} []{}",raw[parent].trim_end(),newline)));
            } else {for b in keys.iter().filter(|b|lines.contains(&b.time_line)){changes.push((b.start,b.end,String::new()))}}
        }Some(apply(text,changes))
    })
}
fn split_weights(a:&Block,b:&Block,time:f64,c:Option<&str>) -> Option<[f64;4]> {
    let dt=b.time-a.time;let target=(time-a.time)/dt;
    if target<=0.0 || target>=1.0 {return None}
    let w0=if field(a,"weightedMode").parse::<u32>().unwrap_or(0)&2!=0{number(a,"outWeight",c).unwrap_or(1.0/3.0).clamp(0.0,1.0)}else{1.0/3.0};
    let w1=if field(b,"weightedMode").parse::<u32>().unwrap_or(0)&1!=0{number(b,"inWeight",c).unwrap_or(1.0/3.0).clamp(0.0,1.0)}else{1.0/3.0};
    let (mut lo,mut hi)=(0.0_f64,1.0_f64);
    for _ in 0..48 {let u=(lo+hi)*0.5;let x=3.0*(1.0-u).powi(2)*u*w0+3.0*(1.0-u)*u*u*(1.0-w1)+u*u*u;if x<target{lo=u}else{hi=u}}
    let u=(lo+hi)*0.5;let q0=u*w0;let q1=w0+(1.0-w1-w0)*u;let q2=1.0-w1+w1*u;let r0=q0+(q1-q0)*u;let r1=q1+(q2-q1)*u;
    Some([q0/target,(target-r0)/target,(r1-target)/(1.0-target),(1.0-q2)/(1.0-target)])
}
fn extend_stop_time(text:&str,time:f64)->String {
    let mut offset=0;
    for line in text.split_inclusive('\n') {
        if let Some(value)=line.trim().strip_prefix("m_StopTime:") && value.trim().parse::<f64>().is_ok_and(|stop|time>stop) {
            let colon=line.find(':').unwrap()+1;
            let start=offset+colon+line[colon..].len()-line[colon..].trim_start().len();
            let mut result=text.to_owned();result.replace_range(start..start+value.trim().len(),&(time as f32).to_string());return result;
        }
        offset+=line.len();
    }
    text.to_owned()
}
pub fn insert(ui:&egui::Ui,bindings:&[&AnimationBinding],time:f64) -> bool {
    let aliases:BTreeSet<_>=bindings.iter().flat_map(|b|b.keyframes.iter().flat_map(|k|k.time_lines.iter().copied())).collect();
    editing::change_animation_document(ui,|text|{
        let all=blocks(text);let raw:Vec<_>=text.split_inclusive('\n').collect();let mut changes=Vec::new();
        for parent in all.iter().filter(|b|aliases.contains(&b.time_line)).map(|b|b.parent).collect::<BTreeSet<_>>() {
            let keys:Vec<_>=all.iter().filter(|b|b.parent==parent).collect();
            if keys.iter().any(|b|(b.time-time).abs()<1e-6){continue}
            let template=*keys.iter().rev().find(|b|b.time<time).unwrap_or(&keys[0]);
            let cs=components(field(template,"value"));
            let format_sample=|slope:bool| -> Option<String> {
                let format=|v:f64|if v.is_infinite(){"Infinity".to_owned()}else{(v as f32).to_string()};
                if cs.is_empty(){let (v,m)=sample(&keys,time,None)?;Some(format(if slope{m}else{v}))}
                else {let mut values=Vec::new();for c in &cs {let(v,m)=sample(&keys,time,Some(c))?;values.push(format!("{c}: {}",format(if slope{m}else{v})))}Some(format!("{{{}}}",values.join(", ")))}
            };
            let value=format_sample(false)?;let slope=format_sample(true)?;
            let mut replacement=replace_fields(&raw,template,&[("time".into(),(time as f32).to_string()),("value".into(),value),("inSlope".into(),slope.clone()),("outSlope".into(),slope),("tangentMode".into(),"1".into()),("weightedMode".into(),"0".into())]);
            if let (Some(a),Some(b))=(keys.iter().rev().find(|b|b.time<time),keys.iter().find(|b|b.time>time)) {
                // Split the Bezier segment so insertion preserves weighted curve shape.
                let weight_value=|index:usize| -> Option<String> {
                    if cs.is_empty(){Some((split_weights(a,b,time,None)?[index] as f32).to_string())}
                    else {let mut values=Vec::new();for c in &cs {values.push(format!("{c}: {}",split_weights(a,b,time,Some(c))?[index] as f32))}Some(format!("{{{}}}",values.join(", ")))}
                };
                let new_lines:Vec<_>=replacement.split_inclusive('\n').collect();let new_blocks=blocks(&replacement);let new_block=new_blocks.first()?;
                replacement=replace_fields(&new_lines,new_block,&[("inWeight".into(),weight_value(1)?),("outWeight".into(),weight_value(2)?),("weightedMode".into(),"3".into())]);
                for (old,name,index,bit) in [(*a,"outWeight",0,2),(*b,"inWeight",3,1)] {
                    let mode=field(old,"weightedMode").parse::<u32>().unwrap_or(0)|bit;
                    let mut fields=vec![(name.into(),weight_value(index)?),("weightedMode".into(),mode.to_string())];
                    if cs.is_empty(){let shift=if name=="outWeight"{5}else{1};let flags=field(old,"tangentMode").parse::<u32>().unwrap_or(0);fields.push(("tangentMode".into(),((flags & !(15<<shift))|1).to_string()));}
                    changes.push((old.start,old.end,replace_fields(&raw,old,&fields)));
                }
            }
            let at=keys.iter().find(|b|b.time>time).map_or(keys.last()?.end,|b|b.start);
            changes.push((at,at,replacement));
        }
        for (index,line) in raw.iter().enumerate().filter(|(_,line)|line.trim()=="m_Curve: []") {
            let indent=line.len()-line.trim_start().len();
            let end=(index+1..raw.len()).find(|j|raw[*j].trim()=="- curve:" || raw[*j].starts_with("  m_")).unwrap_or(raw.len());
            let metadata=&raw[index+1..end];
            let get=|name:&str|metadata.iter().find_map(|line|line.trim().strip_prefix(&format!("{name}:")).map(str::trim));
            let section=raw[..index].iter().rev().find(|line|line.starts_with("  m_")).and_then(|line|line.trim().split_once(':').map(|(k,_)|k)).unwrap_or("");
            let runtime_root=match section {"m_PositionCurves"=>Some("m_LocalPosition"),"m_ScaleCurves"=>Some("m_LocalScale"),"m_RotationCurves"=>Some("m_LocalRotation"),"m_EulerCurves"=>Some("localEulerAnglesRaw"),_=>None};
            let matches=bindings.iter().any(|b|get("path").unwrap_or("")==b.path && if let Some(root)=runtime_root{crate::is_group_component(&b.attribute,root)}else{get("attribute")==Some(b.attribute.as_str()) && get("classID").is_some_and(|id|crate::animation_class_name(id)==b.class_name)});
            if !matches{continue}
            let newline=if text.contains("\r\n"){"\r\n"}else{"\n"};let space=" ".repeat(indent);let field_space=" ".repeat(indent+2);
            let cs=if runtime_root.is_some(){if section=="m_RotationCurves"{vec!["x","y","z","w"]}else{vec!["x","y","z"]}}else{vec![]};
            let vector=|value:bool|if cs.is_empty(){"0".into()}else{format!("{{{}}}",cs.iter().map(|c|format!("{c}: {}",if value && (section=="m_ScaleCurves" || section=="m_RotationCurves" && *c=="w"){1}else{0})).collect::<Vec<_>>().join(", "))};
            let mut block=format!("{space}m_Curve:{newline}{space}- serializedVersion: 3{newline}");
            for (key,value) in [("time",(time as f32).to_string()),("value",vector(true)),("inSlope",vector(false)),("outSlope",vector(false)),("tangentMode","1".into()),("weightedMode","0".into()),("inWeight",if cs.is_empty(){"0.33333334".into()}else{format!("{{{}}}",cs.iter().map(|c|format!("{c}: 0.33333334")).collect::<Vec<_>>().join(", "))}),("outWeight",if cs.is_empty(){"0.33333334".into()}else{format!("{{{}}}",cs.iter().map(|c|format!("{c}: 0.33333334")).collect::<Vec<_>>().join(", "))})] {block.push_str(&format!("{field_space}{key}: {value}{newline}"));}
            changes.push((index,index+1,block));
        }
        if changes.is_empty(){None}else{Some(extend_stop_time(&apply(text,changes),time))}
    })
}
/// Apply tangent operations to editor and runtime copies together.
fn tangent(ui:&egui::Ui,bindings:&[&AnimationBinding],selected:&BTreeSet<usize>,side:usize,mode:&str,manual:Option<f64>) -> bool {
    editing::change_animation_document(ui,|text|{
        let all=blocks(text);let raw:Vec<_>=text.split_inclusive('\n').collect();
        let mut updates=std::collections::BTreeMap::<usize,Vec<(String,String)>>::new();
        for binding in bindings { for key in binding.keyframes.iter().filter(|k|selected.contains(&k.yaml_line)) {
            for line in &key.time_lines {
                let block=all.iter().find(|b|b.time_line==*line)?;
                let component=if field(block,"value").starts_with('{'){binding.attribute.rsplit('.').next()}else{None};
                let keys:Vec<_>=all.iter().filter(|b|b.parent==block.parent).collect();let index=keys.iter().position(|b|b.time_line==*line)?;
                let value=number(block,"value",component)?;
                let prev=index.checked_sub(1).map(|i|keys[i]);let next=keys.get(index+1).copied();
                let left=prev.and_then(|b|number(b,"value",component).map(|v|(value-v)/(block.time-b.time))).unwrap_or(0.0);
                let right=next.and_then(|b|number(b,"value",component).map(|v|(v-value)/(b.time-block.time))).unwrap_or(0.0);
                let auto=match(prev,next){(Some(a),Some(b))=>(number(b,"value",component)?-number(a,"value",component)?)/(b.time-a.time),_=>if prev.is_some(){left}else{right}};
                let slope=match mode {"Free Smooth"=>(number(block,"inSlope",component).unwrap_or(0.0)+number(block,"outSlope",component).unwrap_or(0.0))*0.5,"Flat"=>0.0,"Constant"=>f64::INFINITY,"Auto"=>auto,"Clamped Auto"=>if left*right<=0.0{0.0}else{auto.signum()*auto.abs().min(3.0*left.abs().min(right.abs()))},_=>manual.unwrap_or(auto)};
                let changes=updates.entry(block.start).or_default();
                let current=|name:&str,changes:&Vec<(String,String)>| changes.iter().find(|(k,_)|k==name).map_or_else(||field(block,name).to_owned(),|(_,v)|v.clone());
                for which in 0..2 { if side!=2 && side!=which {continue}
                    let name=if which==0{"inSlope"}else{"outSlope"};
                    let m=if mode=="Linear" {if which==0{left}else{right}}else{slope};
                    if mode!="Broken" && mode!="Weighted" && mode!="Weight" && (mode!="Free" || manual.is_some()) {
                        let old=current(name,changes);let value=if let Some(c)=component {
                            let cs=components(&old);let mut values=Vec::new();for name in cs {let v=if name==c{m}else{crate::inline_vector(&old,&[name])?.first().copied()?};values.push(format!("{name}: {}",if v.is_infinite(){"Infinity".into()}else{(v as f32).to_string()}))}format!("{{{}}}",values.join(", "))
                        }else{if m.is_infinite(){"Infinity".into()}else{(m as f32).to_string()}};
                        if let Some(item)=changes.iter_mut().find(|(k,_)|k==name){item.1=value}else{changes.push((name.into(),value))}
                    }
                    if component.is_none() && mode!="Weight" && mode!="Weighted" {
                        let mut bits=current("tangentMode",changes).parse::<u32>().unwrap_or(0);let shift=if which==0{1}else{5};
                        if mode=="Broken" {bits|=1}else{let kind=match mode{"Auto"=>1,"Linear"=>2,"Constant"=>3,"Clamped Auto"=>4,_=>0};bits=(bits & !(15<<shift))|(kind<<shift);if side!=2{bits|=1}else{bits&=!1}}
                        let value=bits.to_string();if let Some(item)=changes.iter_mut().find(|(k,_)|k=="tangentMode"){item.1=value}else{changes.push(("tangentMode".into(),value))}
                    }
                    if mode=="Weighted" || mode=="Weight" {
                        if mode=="Weight" {
                            let name=if which==0{"inWeight"}else{"outWeight"};let old=current(name,changes);let weight=manual?.clamp(0.0,1.0);
                            let value=if let Some(c)=component {let mut values=Vec::new();for name in components(&old){let v=if name==c{weight}else{crate::inline_vector(&old,&[name])?.first().copied()?};values.push(format!("{name}: {}",v as f32))}format!("{{{}}}",values.join(", "))}else{(weight as f32).to_string()};
                            if let Some(item)=changes.iter_mut().find(|(k,_)|k==name){item.1=value}else{changes.push((name.into(),value))}
                        }
                        let mut bits=current("weightedMode",changes).parse::<u32>().unwrap_or(0);bits|=1<<which;
                        let value=bits.to_string();if let Some(item)=changes.iter_mut().find(|(k,_)|k=="weightedMode"){item.1=value}else{changes.push(("weightedMode".into(),value))}
                    }
                }
            }
        }}
        let changes=updates.into_iter().map(|(start,updates)|{let block=all.iter().find(|b|b.start==start).unwrap();(start,block.end,replace_fields(&raw,block,&updates))}).collect();Some(apply(text,changes))
    })
}
fn menu_label(mode:&str)->&str {tr(match mode {"Clamped Auto"=>"自动钳制","Auto"=>"自动","Free Smooth"=>"自由平滑","Flat"=>"平坦","Broken"=>"断开切线","Left Tangent"=>"左切线","Right Tangent"=>"右切线","Both Tangents"=>"双侧切线","Free"=>"自由","Linear"=>"线性","Constant"=>"常量","Weighted"=>"加权",_=>mode})}
pub fn color(attribute:&str) -> Color32 {
    match attribute.rsplit('.').next().unwrap_or("") {"x"|"r"=>Color32::from_rgb(235,85,85),"y"|"g"=>Color32::from_rgb(90,190,105),"z"|"b"=>Color32::from_rgb(85,145,245),"w"|"a"=>Color32::from_rgb(210,160,65),_=>Color32::from_rgb(180,120,220)}
}
pub fn show(ui:&mut egui::Ui,text:&str,bindings:&[&AnimationBinding],duration:f32,primary:&mut Option<SelectedKeyframe>) {
    let all=blocks(text);
    let mut selected=crate::timeline_edit::selected_lines(ui);
    if let Some(key)=primary.as_ref(){selected.insert(key.yaml_line);}
    ui.horizontal(|ui|{for binding in bindings{ui.colored_label(color(&binding.attribute),&binding.attribute);}});
    let zoom=ui.ctx().data_mut(|d|d.get_temp::<f32>(ui.id().with("timeline_zoom"))).unwrap_or(1.0);
    let vertical_id=ui.id().with("curve_vertical_zoom");
    let mut vertical=ui.ctx().data_mut(|d|d.get_temp::<f32>(vertical_id)).unwrap_or(1.0);
    ui.horizontal(|ui|{ui.label(tr("纵向缩放"));crate::theme::zoom_slider(ui, &mut vertical, 10.0);});
    ui.ctx().data_mut(|d|d.insert_temp(vertical_id,vertical));
    let width=ui.available_width();let mut changed=false;
    let scroll_id=ui.id().with("timeline_scroll_x");let scroll_x=ui.ctx().data_mut(|d|d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
    let range_id=ui.id().with("curve_drag_range");
    let output=egui::ScrollArea::both().id_salt("animation_curve_view").horizontal_scroll_offset(scroll_x).max_height(260.0).show(ui,|ui|{
        let (rect,response)=ui.allocate_exact_size(egui::vec2(width*zoom,220.0*vertical),egui::Sense::click_and_drag());
        let mut series=Vec::new();let mut low=f64::INFINITY;let mut high=f64::NEG_INFINITY;
        for binding in bindings {
            let keys:Vec<_>=binding.keyframes.iter().filter_map(|k|all.iter().find(|b|b.time_line==k.yaml_line)).collect();
            let values:Vec<_>=(0..=400).filter_map(|i|{let t=i as f64/400.0*duration as f64;sample(&keys,t,None).filter(|(v,_)|v.is_finite()).map(|(v,_)|(t,v))}).collect();
            for (_,v) in &values{low=low.min(*v);high=high.max(*v)}series.push(values);
        }
        if !low.is_finite(){low=0.0;high=1.0}let pad=((high-low)*0.12).max(0.1);low-=pad;high+=pad;
        if ui.input(|i|i.pointer.primary_down()) {
            if let Some((saved_low,saved_high))=ui.ctx().data_mut(|d|d.get_temp::<(f64,f64)>(range_id)){low=saved_low;high=saved_high}else{ui.ctx().data_mut(|d|d.insert_temp(range_id,(low,high)));}
        }else{ui.ctx().data_mut(|d|d.remove::<(f64,f64)>(range_id));}
        let plot=rect.shrink2(egui::vec2(0.0,16.0));
        let point=|t:f64,v:f64|egui::pos2(crate::animation_time_x(plot,t,duration as f64),egui::lerp(plot.bottom()..=plot.top(),((v-low)/(high-low)) as f32));
        ui.painter().rect_filled(rect,2.0,ui.visuals().extreme_bg_color);
        for i in 0..=4 {let v=low+(high-low)*i as f64/4.0;let y=point(0.0,v).y;ui.painter().line_segment([egui::pos2(plot.left(),y),egui::pos2(plot.right(),y)],Stroke::new(0.5_f32,ui.visuals().widgets.noninteractive.bg_stroke.color));ui.painter().text(egui::pos2(plot.left()+3.0,y),egui::Align2::LEFT_BOTTOM,format!("{v:.2}"),egui::FontId::proportional(10.0),ui.visuals().weak_text_color());}
        for (binding,values) in bindings.iter().zip(series){
            ui.painter().add(egui::Shape::line(values.iter().map(|(t,v)|point(*t,*v)).collect(),Stroke::new(1.5_f32,color(&binding.attribute))));
            for key in &binding.keyframes {if let Ok(value)=key.value.parse::<f64>() {
                let p=point(key.time as f64,value);let hit=egui::Rect::from_center_size(p,egui::vec2(14.0,14.0));
                ui.painter().circle_filled(p,if selected.contains(&key.yaml_line){4.5}else{3.0},if selected.contains(&key.yaml_line){Color32::YELLOW}else{color(&binding.attribute)});
                if (response.clicked()||response.secondary_clicked()) && response.interact_pointer_pos().is_some_and(|pos|hit.contains(pos)) {
                    if !ui.input(|i|i.modifiers.ctrl||i.modifiers.shift){selected.clear()}selected.insert(key.yaml_line);
                    crate::timeline_edit::set_selection(ui,&selected,key.yaml_line);
                    *primary=Some(SelectedKeyframe{track_name:format!("{} · {}",binding.path,binding.attribute),time:key.time,value:key.value.clone(),yaml_line:key.yaml_line});
                }
            }}
        }
        if let Some(key)=primary.as_ref() && let Some(block)=all.iter().find(|b|b.time_line==key.yaml_line) && let Some(value)=number(block,"value",None) {
            let origin=point(key.time as f64,value);
            for (name,side,direction) in [("inSlope",0,-1.0),("outSlope",1,1.0)] {
                let slope=number(block,name,None).unwrap_or(0.0);
                if !slope.is_finite(){continue}
                let dt=(duration as f64*0.06/zoom as f64).max(1e-6)*direction;
                let handle=point(key.time as f64+dt,value+slope*dt);
                ui.painter().line_segment([origin,handle],Stroke::new(1.0_f32,Color32::YELLOW));
                ui.painter().circle_filled(handle,3.5,Color32::YELLOW);
                let response=ui.interact(egui::Rect::from_center_size(handle,egui::vec2(12.0,12.0)),ui.id().with(("curve_handle",key.yaml_line,side)),egui::Sense::drag());
                if response.dragged() && editing::active(ui) && let Some(position)=response.interact_pointer_pos() {
                    let v=low+(plot.bottom()-position.y) as f64/plot.height() as f64*(high-low);
                    let slope=(v-value)/dt;
                    if slope.is_finite(){changed=tangent(ui,bindings,&selected,side,"Free",Some(slope));}
                }
            }
        }
        response.context_menu(|ui|{
            ui.add_enabled_ui(editing::active(ui)&&!selected.is_empty(),|ui|{
                for mode in ["Clamped Auto","Auto","Free Smooth","Flat","Broken"]{if ui.button(menu_label(mode)).clicked(){changed=tangent(ui,bindings,&selected,2,mode,None);ui.close_menu();}}
                ui.separator();
                for (label,side) in [("Left Tangent",0),("Right Tangent",1),("Both Tangents",2)]{ui.menu_button(menu_label(label),|ui|{for mode in ["Free","Linear","Constant","Weighted"]{if ui.button(menu_label(mode)).clicked(){changed=tangent(ui,bindings,&selected,side,mode,None);ui.close_menu();}}});}
                ui.separator();if ui.button(tr("删除选中关键帧")).clicked(){let lines=deletion_lines(bindings,&selected);changed=delete(ui,&lines);ui.close_menu();}
            });
        });
    });
    ui.ctx().data_mut(|d|d.insert_temp(scroll_id,output.state.offset.x));
    if changed {return}
    if let Some(key)=primary.as_ref() && let Some(block)=all.iter().find(|b|b.time_line==key.yaml_line) {
        ui.horizontal(|ui|{for (label,name,side) in [("Left Tangent","inSlope",0),("Right Tangent","outSlope",1)] {
            ui.label(menu_label(label));let mut value=number(block,name,None).unwrap_or(0.0);
            let response=ui.add_enabled(editing::active(ui),egui::DragValue::new(&mut value).speed(0.05));
            if response.changed(){changed=tangent(ui,bindings,&selected,side,"Free",Some(value));}
        }});
        ui.horizontal(|ui|{for (label,name,side) in [("左权重","inWeight",0),("右权重","outWeight",1)] {
            ui.label(menu_label(label));let mut weight=number(block,name,None).unwrap_or(1.0/3.0);
            if ui.add_enabled(editing::active(ui),egui::DragValue::new(&mut weight).range(0.0..=1.0).speed(0.01)).changed(){
                changed=tangent(ui,bindings,&selected,side,"Weight",Some(weight));
            }
        }});
    }
    if changed {ui.ctx().request_repaint();}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn hermite_and_step_sampling(){
        let text="    m_Curve:\n    - serializedVersion: 3\n      time: 0\n      value: 0\n      inSlope: 0\n      outSlope: 0\n    - serializedVersion: 3\n      time: 1\n      value: 1\n      inSlope: 0\n      outSlope: 0\n    m_PreInfinity: 2\n";
        let mut keys=blocks(text);assert_eq!(keys.len(),2);assert!((evaluate(&keys[0],&keys[1],0.25,None).unwrap().0-0.15625).abs()<1e-8);
        keys[0].fields.iter_mut().find(|(k,_)|k=="outSlope").unwrap().1="Infinity".into();assert_eq!(evaluate(&keys[0],&keys[1],0.5,None).unwrap().0,0.0);
    }
    #[test]
    fn weighted_split_preserves_original_segment() {
        let text="    m_Curve:\n    - serializedVersion: 3\n      time: 0\n      value: 0\n      inSlope: 0\n      outSlope: 3\n      weightedMode: 2\n      outWeight: 0.15\n    - serializedVersion: 3\n      time: 1\n      value: 1\n      inSlope: -2\n      outSlope: 0\n      weightedMode: 1\n      inWeight: 0.6\n    m_PreInfinity: 2\n";
        let keys=blocks(text);let (a,b)=(&keys[0],&keys[1]);let time=0.4;
        let (value,slope)=evaluate(a,b,time,None).unwrap();let weights=split_weights(a,b,time,None).unwrap();
        let mut left=a.clone();let mut right=b.clone();let mut middle=a.clone();middle.time=time;
        let set=|key:&mut Block,name:&str,value:f64|{if let Some((_,v))=key.fields.iter_mut().find(|(k,_)|k==name){*v=value.to_string()}else{key.fields.push((name.into(),value.to_string()))}};
        set(&mut left,"outWeight",weights[0]);set(&mut left,"weightedMode",2.0);
        set(&mut middle,"value",value);set(&mut middle,"inSlope",slope);set(&mut middle,"outSlope",slope);set(&mut middle,"inWeight",weights[1]);set(&mut middle,"outWeight",weights[2]);set(&mut middle,"weightedMode",3.0);
        set(&mut right,"inWeight",weights[3]);set(&mut right,"weightedMode",1.0);
        for i in 1..100 {let t=i as f64/100.0;let expected=evaluate(a,b,t,None).unwrap().0;let actual=if t<time{evaluate(&left,&middle,t,None)}else{evaluate(&middle,&right,t,None)}.unwrap().0;assert!((expected-actual).abs()<1e-8);}
    }
    #[test]
    fn insertion_deletion_and_undo_sync_curve_copies() {
        use std::sync::{Arc,Mutex};
        let curve="  - curve:\n      serializedVersion: 2\n      m_Curve:\n      - serializedVersion: 3\n        time: 0\n        value: 0\n        inSlope: 0\n        outSlope: 0\n        tangentMode: 1\n        weightedMode: 0\n        inWeight: 0.33333334\n        outWeight: 0.33333334\n      - serializedVersion: 3\n        time: 1\n        value: 1\n        inSlope: 0\n        outSlope: 0\n        tangentMode: 1\n        weightedMode: 0\n        inWeight: 0.33333334\n        outWeight: 0.33333334\n      m_PreInfinity: 2\n      m_PostInfinity: 2\n    attribute: m_Alpha\n    path: Child\n    classID: 225\n    script: {fileID: 0}\n";
        let original=format!("--- !u!74 &7400000\nAnimationClip:\n  m_FloatCurves:\n{curve}  m_EditorCurves:\n{curve}  m_StopTime: 1\n").replace("\n","\r\n");
        let session=Arc::new(Mutex::new(editing::Session::new("test.anim".into(),original.clone(),original.as_bytes().to_vec())));
        let ctx=egui::Context::default();
        let _=ctx.run(Default::default(),|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
            editing::publish(ctx,Some(session.clone()),0..original.len());
            let tree=crate::animation_path_tree(&original);let mut bindings=Vec::new();crate::collect_animation_bindings(&tree,&mut bindings);
            assert!(insert(ui,&bindings,0.25));
        });});
        let inserted=session.lock().unwrap().rendered();let keys=blocks(&inserted);assert_eq!(keys.len(),6);
        let added:Vec<_>=keys.iter().filter(|k|(k.time-0.25).abs()<1e-6).collect();assert_eq!(added.len(),2);assert!(added.iter().all(|k|(number(k,"value",None).unwrap()-0.15625).abs()<1e-6));
        let _=ctx.run(Default::default(),|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
            editing::publish(ctx,Some(session.clone()),0..inserted.len());
            assert!(delete(ui,&added.iter().map(|k|k.time_line).collect()));
        });});
        assert_eq!(blocks(&session.lock().unwrap().rendered()).len(),4);
        session.lock().unwrap().undo();assert_eq!(session.lock().unwrap().rendered(),inserted);
        session.lock().unwrap().undo();assert_eq!(session.lock().unwrap().rendered(),original);
    }

}
