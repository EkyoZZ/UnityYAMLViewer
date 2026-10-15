//! Independent stage nodes, source-derived links and optional explicit range membership.
use super::{egui,Rect,Vec2,Color32,Stroke,Sense,HashMap,Input,stages,probe,mini,tr,re};
#[derive(Clone,Debug)]
pub(super) struct Edge { pub from:Option<usize>,pub value:String }
#[derive(Clone,Debug,Default)]
pub(super) struct Plan {pub reads:Vec<Vec<Edge>>,pub outputs:Vec<usize>,pub visual:Vec<bool>,pub layout:Vec<Vec2>,pub output:Vec2}
#[derive(Default)]
pub(super) struct Areas { regions:HashMap<usize,Region> }
struct Region { position:Vec2,size:Vec2,members:std::collections::HashSet<usize> }
impl Areas { pub fn clear(&mut self){self.regions.clear();} }
fn reachable(plan:&Plan)->Vec<bool>{
    // Include both consumers and supporting producers (e.g. local constants)
    // in the input-connected computation, never unrelated spatial occupants.
    let mut result=plan.reads.iter().map(|edges|edges.iter().any(|edge|edge.from.is_none())).collect::<Vec<_>>();
    let count=result.len();
    loop {
        let mut changed=false;
        for (node,edges) in plan.reads.iter().enumerate(){
            for from in edges.iter().filter_map(|edge|edge.from).filter(|from|*from<count) {
                if result[node]||result[from] {
                    if !result[node]{result[node]=true;changed=true;}
                    if !result[from]{result[from]=true;changed=true;}
                }
            }
        }
        if !changed{break;}
    }
    result
}

impl Areas {
    pub(super) fn rect(&self,index:usize,r:Rect,zoom:f32)->Rect{
        let region=&self.regions[&index];Rect::from_min_size(r.min+region.position*zoom,region.size*zoom)
    }
    pub(super) fn prepare(&mut self,ui:&mut egui::Ui,group:&stages::Group,index:usize,r:&mut Rect,canvas:Rect,zoom:f32,positions:&mut HashMap<usize,Vec2>){
        let connected=reachable(&group.flow);
        let initial=bounds(group,index,*r,zoom,positions);
        let region=self.regions.entry(index).or_insert_with(||Region{position:(initial.min-r.min)/zoom,size:initial.size()/zoom,members:connected.iter().enumerate().filter_map(|(step,yes)|yes.then_some(step)).collect()});
        region.members.retain(|step|connected.get(*step).copied().unwrap_or(false));
        for step in 0..group.statements.len(){positions.entry(key(index,step)).or_insert(default_point(group,step));}
        let rect=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
        let header=Rect::from_min_size(rect.min,egui::vec2(rect.width(),34.*zoom));
        let drag=ui.interact(header.intersect(canvas),ui.id().with(("scope_block_drag",index)),Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if drag.dragged(){
            let delta=drag.drag_delta()/zoom;
            translate_members(index,group.statements.len(),delta,&region.members,positions);
            *r=r.translate(delta*zoom);
        }
        let rect=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
        let handle=Rect::from_min_size(rect.right_bottom()-egui::vec2(18.,18.),egui::vec2(18.,18.));
        let resize=ui.interact(handle.intersect(canvas),ui.id().with(("scope_block_resize",index)),Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeNwSe);
        if resize.dragged(){region.size=(region.size+resize.drag_delta()/zoom).clamp(egui::vec2(300.,200.),egui::vec2(24000.,24000.));}
    }
}
fn translate_members(index:usize,count:usize,delta:Vec2,members:&std::collections::HashSet<usize>,positions:&mut HashMap<usize,Vec2>){
    *positions.get_mut(&(1000+index)).unwrap()+=delta;
    // In/Out and explicit members follow the block; all other nodes stay put.
    for step in 0..count{if !members.contains(&step){*positions.get_mut(&key(index,step)).unwrap()-=delta;}}
}
fn can_join(reachable:bool,center:egui::Pos2,region:Rect)->bool{reachable&&region.contains(center)}

fn references(text:&str,known:&HashMap<String,usize>)->Vec<String>{
    let pattern=re(r"\b[A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*");
    pattern.find_iter(text).filter_map(|m|{
        let value=m.as_str().split_whitespace().collect::<String>();
        let root=value.split('.').next().unwrap();
        if text[m.end()..].trim_start().starts_with('(') || !(known.contains_key(root)||root.starts_with('_')) {None}else{Some(value)}
    }).collect()
}
fn atoms(value:&str,types:&HashMap<String,usize>)->Vec<String>{
    if let Some(&width)=types.get(value){
        if width>1 {return "rgba".chars().take(width).map(|c|format!("{value}.{c}")).collect();}
        if width==0 {
            let prefix=format!("{value}.");
            let mut fields=types.keys().filter(|key|key.starts_with(&prefix)).cloned().collect::<Vec<_>>();fields.sort();
            if !fields.is_empty(){return fields.iter().flat_map(|key|atoms(key,types)).collect();}
        }
        return vec![value.to_owned()];
    }
    if let Some((base,field))=value.rsplit_once('.'){
        let alphabet=if field.chars().all(|c|"rgba".contains(c)){Some("rgba")}else if field.chars().all(|c|"xyzw".contains(c)){Some("xyzw")}else{None};
        if let Some(alphabet)=alphabet {return field.chars().map(|c|format!("{base}.{}","rgba".chars().nth(alphabet.find(c).unwrap()).unwrap())).collect();}
    }
    vec![value.to_owned()]
}
pub(super) fn plan(group:&stages::Group,source:&str,inputs:&[Input])->Plan{
    let mut types=HashMap::new();
    let mut colors=std::collections::HashSet::<String>::new();
    for input in inputs {if matches!(input.value,super::Value::Vector(_,true)){colors.insert(input.name.clone());}}

    let numeric=re(r"\b(?:fixed|half|float|int|uint|bool)([1-4]?)\s+(\w+)\b");
    for c in numeric.captures_iter(source).chain(numeric.captures_iter(&group.signature)).chain(numeric.captures_iter(&group.body)) {
        types.insert(c[2].to_owned(),if c[1].is_empty(){1}else{c[1].parse().unwrap()});
    }
    let schemas=re(r"(?s)\bstruct\s+(\w+)\s*\{([^}]*)\}");
    for schema in schemas.captures_iter(source){
        let mut owners=group.argument_types.iter().zip(&group.parameters).filter(|(ty,_)|**ty==schema[1]).map(|(_,name)|name.clone()).collect::<Vec<_>>();
        let declaration=re(&format!(r"\b{}\s+(\w+)\s*(?:;|=)",regex::escape(&schema[1])));
        owners.extend(declaration.captures_iter(&group.body).map(|c|c[1].to_owned()));
        for owner in owners {
            types.insert(owner.clone(),0);
            for field in re(r"\b(?:float|half|fixed)(?:3|4)\s+(\w+)\s*:\s*COLOR\d*\b").captures_iter(&schema[2]){colors.insert(format!("{owner}.{}",&field[1]));}
            for field in numeric.captures_iter(&schema[2]) {types.insert(format!("{owner}.{}",&field[2]),if field[1].is_empty(){1}else{field[1].parse().unwrap()});}
        }
    }
    let mut definitions=HashMap::<String,usize>::new();let mut plan=Plan::default();
    for (step,statement) in group.statements.iter().enumerate(){
        let returned=statement.trim_start().starts_with("return ");
        let target=probe::target(statement).ok();
        let mut reads=statement.split_once('=').map_or_else(||references(statement,&types),|(_,rhs)|references(rhs,&types));
        if re(r"[+*/-]\s*=").is_match(statement){if let Some(target)=&target{reads.push(target.clone());}}
        let result_width=target.as_ref().map(|target|atoms(target,&types).len()).unwrap_or(0);
        let sampled=re(r"\btex2D\s*\(").is_match(statement);
        let color_input=reads.iter().any(|value|{
            let base=value.rsplit_once('.').map_or(value.as_str(),|(base,_)|base);
            let explicit=value.rsplit_once('.').is_some_and(|(_,field)|field.len()>=3&&field.chars().all(|c|"rgba".contains(c)));
            atoms(value,&types).len()>=3&&(explicit||colors.contains(value)||colors.contains(base))
        });
        let visual=result_width>=3&&(sampled||color_input);
        plan.visual.push(visual);
        if visual {if let Some(target)=&target{let base=target.rsplit_once('.').filter(|(_,suffix)|suffix.chars().all(|c|"rgba".contains(c))).map_or(target.as_str(),|(base,_)|base);colors.insert(base.to_owned());}}
        let mut edges=Vec::new();
        for value in reads {for atom in atoms(&value,&types){
            let from=definitions.get(&atom).copied();
            if !edges.iter().any(|edge:&Edge|edge.from==from && edge.value==value){edges.push(Edge{from,value:value.clone()});}
        }}
        if returned {plan.outputs=vec![step];}
        if !returned {if let Some(target)=target{for atom in atoms(&target,&types){definitions.insert(atom,step);}}}
        plan.reads.push(edges);
    }
    plan
}
fn key(group:usize,step:usize)->usize{10000+group*64+step}
fn height(group:&stages::Group,step:usize)->f32{let base:f32=if group.probes[step].is_ok()&&group.flow.visual[step]{272.}else{116.};base.max(60.+group.flow.reads[step].len() as f32*12.)}
pub(super) fn arrange(group:&stages::Group)->(Vec<Vec2>,Vec2){
    let mut ranks=Vec::<usize>::new();let mut rows=HashMap::<usize,f32>::new();let mut points=Vec::new();
    for step in 0..group.statements.len(){
        let rank=group.flow.reads[step].iter().filter_map(|edge|edge.from).filter(|from|*from<step).map(|from|ranks[from]+1).max().unwrap_or(0);
        let row=rows.entry(rank).or_insert(80.);
        points.push(egui::vec2(320.+rank as f32*460.,*row));
        *row+=height(group,step)+40.;ranks.push(rank);
    }
    let last=ranks.iter().copied().max().map_or(0,|rank|rank+1);
    (points,egui::vec2(320.+last as f32*460.,80.))
}
fn default_point(group:&stages::Group,step:usize)->Vec2{group.flow.layout[step]}
pub(super) fn size(group:&stages::Group,index:usize,positions:&HashMap<usize,Vec2>)->Vec2{
    let mut size=egui::vec2(group.flow.output.x+260.,(220.+group.inputs.len() as f32*24.).max(340.));
    for step in 0..group.statements.len(){let p=positions.get(&key(index,step)).copied().unwrap_or(default_point(group,step));size.x=size.x.max(p.x+360.+340.);size.y=size.y.max(p.y+height(group,step)+28.);}
    size
}
pub(super) fn step_ports(group:&stages::Group,index:usize,r:Rect,zoom:f32,positions:&HashMap<usize,Vec2>,step:usize)->(egui::Pos2,egui::Pos2){
    let point=positions.get(&key(index,step)).copied().unwrap_or(default_point(group,step));
    let left=r.min+(point+egui::vec2(0.,14.))*zoom;(left,left+egui::vec2(360.*zoom,0.))
}
/// Visual extent only; containment never changes explicit region membership.
pub(super) fn bounds(group:&stages::Group,index:usize,r:Rect,zoom:f32,positions:&HashMap<usize,Vec2>)->Rect{
    let mut bounds=input_rect(group,r,zoom).union(output_rect(group,r,zoom)).union(Rect::from_min_size(r.min,egui::vec2(280.,34.)*zoom));
    for step in 0..group.statements.len(){
        let p=positions.get(&key(index,step)).copied().unwrap_or(default_point(group,step));
        bounds=bounds.union(Rect::from_min_size(r.min+p*zoom,egui::vec2(360.,height(group,step))*zoom));
    }
    bounds.expand(10.*zoom)
}
pub(super) fn input_rect(group:&stages::Group,r:Rect,zoom:f32)->Rect{Rect::from_min_size(r.min+egui::vec2(16.,80.)*zoom,egui::vec2(240.,(110.+group.inputs.len() as f32*24.).max(150.))*zoom)}
pub(super) fn output_rect(group:&stages::Group,r:Rect,zoom:f32)->Rect{Rect::from_min_size(r.min+group.flow.output*zoom,egui::vec2(240.,180.)*zoom)}
pub(super) fn input_port(group:&stages::Group,r:Rect,zoom:f32,row:Option<usize>)->egui::Pos2{let rect=input_rect(group,r,zoom);egui::pos2(rect.left(),rect.top()+(row.map_or(42.,|i|78.+i as f32*24.))*zoom)}
pub(super) fn output_port(group:&stages::Group,r:Rect,zoom:f32)->egui::Pos2{let rect=output_rect(group,r,zoom);egui::pos2(rect.right(),rect.top()+42.*zoom)}
pub(super) fn draw(ui:&mut egui::Ui,group:&stages::Group,index:usize,r:Rect,canvas:Rect,zoom:f32,positions:&mut HashMap<usize,Vec2>,areas:&mut Areas,inputs:&[Input],selected:Option<&probe::Probe>,mini:&mut mini::Previews,source:Option<&mini::Source>)->Option<probe::Probe>{
    let painter=ui.painter().with_clip_rect(canvas);let tint=if group.stage==stages::Stage::Vert{Color32::from_rgb(95,175,225)}else{Color32::from_rgb(185,140,205)};
    let input=input_rect(group,r,zoom);let output=output_rect(group,r,zoom);
    let mut region=areas.regions.remove(&index);
    let members_reachable=reachable(&group.flow);

    let name=match group.stage {stages::Stage::Vert=>"Vert",stages::Stage::Frag=>"Frag",stages::Stage::Function=>group.name.as_str()};
    let fill=if ui.visuals().dark_mode{Color32::from_rgb(38,40,46)}else{Color32::from_rgb(242,244,248)};
    let text_color=ui.visuals().text_color();
    let panel=|rect:Rect,title:&str|{
        painter.rect_filled(rect,4.,fill);painter.rect_stroke(rect,4.,Stroke::new(1.0_f32,tint),egui::StrokeKind::Inside);
        let header=Rect::from_min_size(rect.min,egui::vec2(rect.width(),28.*zoom));painter.rect_filled(header,4.,tint.gamma_multiply(0.3));
        painter.text(header.left_center()+egui::vec2(8.*zoom,0.),egui::Align2::LEFT_CENTER,title,egui::FontId::proportional(14.*zoom),text_color);
    };
    panel(input,&format!("{name} In"));panel(output,&format!("{name} Out"));

    painter.text(input.min+egui::vec2(8.,36.)*zoom,egui::Align2::LEFT_TOP,group.argument_types.iter().zip(&group.parameters).map(|(ty,name)|format!("{ty} {name}")).collect::<Vec<_>>().join(", "),egui::FontId::monospace(11.*zoom),ui.visuals().weak_text_color());
    for (row,(property,channel,_)) in group.inputs.iter().enumerate(){let port=input_port(group,r,zoom,Some(row));painter.circle_filled(port,3.5*zoom,super::input_color(&inputs[*property].value));painter.text(port+egui::vec2(9.*zoom,0.),egui::Align2::LEFT_CENTER,format!("{} · {channel}",inputs[*property].name),egui::FontId::proportional(11.*zoom),ui.visuals().text_color());}
    let output_text=format!("{}\n{}",group.result_type,group.statements.last().map_or("",String::as_str));
    let output_body=Rect::from_min_max(output.min+egui::vec2(8.,38.)*zoom,output.max-egui::vec2(8.,8.)*zoom);
    let galley=painter.layout(output_text.clone(),egui::FontId::monospace(11.*zoom),ui.visuals().text_color(),output_body.width());
    painter.with_clip_rect(output_body.intersect(canvas)).galley(output_body.min,galley,ui.visuals().text_color());
    ui.interact(output_body.intersect(canvas),ui.id().with(("output_source",index)),Sense::hover()).on_hover_text(output_text);

    let nodes=(0..group.statements.len()).map(|step|{let point=*positions.entry(key(index,step)).or_insert(default_point(group,step));Rect::from_min_size(r.min+point*zoom,egui::vec2(360.,height(group,step))*zoom)}).collect::<Vec<_>>();
    let wire=|from:egui::Pos2,to:egui::Pos2|{painter.add(egui::epaint::CubicBezierShape::from_points_stroke(super::wire_points(from,to,zoom),false,Color32::TRANSPARENT,Stroke::new(1.2_f32,tint)));};
    for (step,edges) in group.flow.reads.iter().enumerate(){for (row,edge) in edges.iter().enumerate(){
        let from=edge.from.map_or(egui::pos2(input.right(),input.top()+42.*zoom),|node|egui::pos2(nodes[node].right(),nodes[node].top()+14.*zoom));
        let to=egui::pos2(nodes[step].left(),nodes[step].top()+(36.+row as f32*12.)*zoom);
        wire(from,to);painter.circle_filled(to,2.5*zoom,tint);
    }}
    for &step in &group.flow.outputs{wire(egui::pos2(nodes[step].right(),nodes[step].top()+14.*zoom),egui::pos2(output.left(),output.top()+42.*zoom));}
    for port in [egui::pos2(input.right(),input.top()+42.*zoom),egui::pos2(output.left(),output.top()+42.*zoom),output_port(group,r,zoom)]{painter.circle_filled(port,4.*zoom,tint);}
    let mut request=None;
    for (step,rect) in nodes.into_iter().enumerate(){
        if !rect.intersects(canvas){continue;}
        let candidate=&group.probes[step];let active=selected.is_some_and(|p|candidate.as_ref().is_ok_and(|candidate|p.function==candidate.function&&p.step==candidate.step&&p.vertex==candidate.vertex));
        panel(rect,&format!("{} {} · {}",tr("步骤"),step+1,candidate.as_ref().map_or("—",|p|p.target.as_str())));
        if active{painter.rect_stroke(rect,4.,Stroke::new(2.0_f32,Color32::from_rgb(225,155,65)),egui::StrokeKind::Inside);}
        let header=Rect::from_min_size(rect.min,egui::vec2(rect.width(),28.*zoom));
        let response=ui.interact(header.intersect(canvas),ui.id().with(("stage_step",index,step)),Sense::click_and_drag());
        if response.dragged(){let point=positions.get_mut(&key(index,step)).unwrap();*point=(*point+response.drag_delta()/zoom).clamp(egui::vec2(-16000.,-16000.),egui::vec2(16000.,16000.));}
        if response.drag_stopped(){
            if let Some(region)=&mut region{
                let point=positions[&key(index,step)];let center=r.min+(point+egui::vec2(180.,height(group,step)*0.5))*zoom;
                let bounds=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
                if can_join(members_reachable[step],center,bounds){region.members.insert(step);}else{region.members.remove(&step);}
            }
        }
        if let Some(region)=&mut region {
            if region.members.contains(&step){painter.circle_filled(rect.right_top()+egui::vec2(-8.,8.)*zoom,3.*zoom,Color32::from_rgb(225,155,65));}
            response.clone().context_menu(|ui|{
                let center=rect.center();let bounds=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
                if ui.add_enabled(can_join(members_reachable[step],center,bounds),egui::Button::new(tr("加入范围框"))).clicked(){region.members.insert(step);ui.close_menu();}
                if ui.button(tr("移出范围框")).clicked(){region.members.remove(&step);ui.close_menu();}
            });
        }
        if response.clicked()&&group.flow.visual[step]{request=candidate.as_ref().ok().cloned();}
        let text=group.statements[step].chars().take(512).collect::<String>();
        let galley=painter.layout(text,egui::FontId::monospace(11.*zoom),ui.visuals().text_color(),rect.width()-16.*zoom);
        let code=Rect::from_min_size(rect.min+egui::vec2(8.,34.)*zoom,egui::vec2(344.,52.)*zoom);
        painter.with_clip_rect(code.intersect(canvas)).galley(code.min,galley,ui.visuals().text_color());
        for point in [egui::pos2(rect.left(),rect.top()+14.*zoom),egui::pos2(rect.right(),rect.top()+14.*zoom)]{painter.circle_filled(point,3.5*zoom,tint);}
        if !group.flow.visual[step] {
            painter.text(rect.center_bottom()-egui::vec2(0.,10.*zoom),egui::Align2::CENTER_BOTTOM,tr("纯计算"),egui::FontId::proportional(11.*zoom),ui.visuals().weak_text_color());
        }else if let Ok(probe)=candidate{
            let preview=Rect::from_center_size(egui::pos2(rect.center().x,rect.top()+178.*zoom),egui::vec2(166.,166.)*zoom);
            mini.show(ui,preview,canvas,probe,source);
            painter.text(egui::pos2(rect.right()-6.*zoom,rect.bottom()-6.*zoom),egui::Align2::RIGHT_BOTTOM,tr("预览快照"),egui::FontId::proportional(10.*zoom),ui.visuals().weak_text_color());
        }else if let Err(error)=candidate{painter.text(rect.center_bottom()-egui::vec2(0.,10.*zoom),egui::Align2::CENTER_BOTTOM,tr("不可预览"),egui::FontId::proportional(11.*zoom),ui.visuals().weak_text_color());response.on_hover_text(error);}
    }
    if let Some(region)=region{areas.regions.insert(index,region);}
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constants_supporting_connected_calculations_join_but_unused_constants_do_not(){
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float3 c=color.rgb;float3 avgColor=float3(0.5,0.5,0.5);float3 unused=float3(1,1,1);c=lerp(avgColor,c,0.5);return color;}\nENDCG",&[]);
        let connected=reachable(&graph.groups[0].flow);
        assert!(connected[0]);assert!(connected[1]);assert!(!connected[2]);assert!(connected[3]);
    }
    #[test]
    fn dependencies_move_forward_and_independent_steps_share_a_column(){
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float4 a=color;float4 b=color;float4 c=a+b;return c;}\nENDCG",&[]);
        let group=&graph.groups[0];let p=&group.flow.layout;
        assert_eq!(p[0].x,p[1].x);assert_ne!(p[0].y,p[1].y);
        assert!(p[2].x>p[0].x);assert!(p[3].x>p[2].x);
        assert!(group.flow.output.x>p[3].x+360.);
    }
    #[test]
    fn moving_one_block_preserves_excluded_and_unrelated_node_world_positions(){
        let mut positions=HashMap::from([(1000,egui::vec2(50.,50.)),(key(0,0),egui::vec2(320.,80.)),(key(0,1),egui::vec2(320.,224.)),(1001,egui::vec2(2000.,0.))]);
        let excluded=positions[&1000]+positions[&key(0,1)];
        let member=positions[&1000]+positions[&key(0,0)];
        let delta=egui::vec2(120.,-40.);
        translate_members(0,2,delta,&std::collections::HashSet::from([0]),&mut positions);
        assert_eq!(positions[&1000]+positions[&key(0,1)],excluded);
        assert_eq!(positions[&1000]+positions[&key(0,0)],member+delta);
        assert_eq!(positions[&1001],egui::vec2(2000.,0.));
    }
    #[test]
    fn component_versions_link_only_to_actual_latest_writes() {
        let source="CGPROGRAM\n#pragma fragment frag\nstruct F {float2 uv:TEXCOORD0;};fixed4 frag(F i):SV_Target {fixed4 c=tex2D(_MainTex,i.uv);c.a*=_Alpha;c.rgb*=c.a;float constantValue=1;return c;}\nENDCG";
        let graph=stages::parse(source,&[]);let group=graph.groups.iter().find(|group|group.stage==stages::Stage::Frag).unwrap();
        let writes=group.flow.reads[2].iter().filter_map(|edge|edge.from).collect::<std::collections::HashSet<_>>();
        assert_eq!(writes,std::collections::HashSet::from([0,1]));
        let returns=group.flow.reads[4].iter().filter_map(|edge|edge.from).collect::<std::collections::HashSet<_>>();
        assert_eq!(returns,std::collections::HashSet::from([1,2]));
        assert_eq!(reachable(&group.flow),vec![true,true,true,false,true]);
        assert_eq!(group.flow.outputs,vec![4]);
        assert_eq!(group.flow.visual,vec![true,false,true,false,true]);
    }
    #[test]
    fn spatial_overlap_never_joins_unconnected_nodes_or_changes_independent_output_position() {
        let bounds=Rect::from_min_size(egui::pos2(100.,100.),egui::vec2(400.,400.));
        assert!(!can_join(false,bounds.center(),bounds));
        assert!(!can_join(true,egui::pos2(0.,0.),bounds));
        assert!(can_join(true,bounds.center(),bounds));
        let region=Region{position:Vec2::ZERO,size:egui::vec2(400.,400.),members:Default::default()};
        assert!(region.members.is_empty());
        let small=Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1040.,500.));
        let large=Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(2000.,1000.));
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 c):SV_Target{return c;}\nENDCG",&[]);
        assert_eq!(output_rect(&graph.groups[0],small,1.),output_rect(&graph.groups[0],large,1.));
    }
}
