//! Independent stage nodes, source-derived links and optional explicit range membership.
use super::{egui,Rect,Vec2,Color32,Stroke,Sense,HashMap,Input,stages,probe,mini,tr,re};
#[derive(Clone,Debug)]
pub(super) struct Edge { pub from:Option<usize>,pub value:String }
#[derive(Clone,Debug,Default)]
pub(super) struct Plan {pub reads:Vec<Vec<Edge>>,pub outputs:Vec<usize>,pub visual:Vec<bool>,pub layout:Vec<Vec2>,pub output:Vec2}
#[derive(Default)]
pub(super) struct Areas { regions:HashMap<usize,Region>, previews:std::collections::HashSet<(usize,usize)>, pub(super) tools:super::tools::Tools }
struct Region { position:Vec2,size:Vec2,collapsed:bool,interface:std::sync::Arc<super::interfaces::Interface>,connected:std::collections::HashSet<usize>,members:std::collections::HashSet<usize> }
impl Areas { pub fn clear(&mut self){self.regions.clear();self.previews.clear();self.tools.clear();} }
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
        let region=&self.regions[&index];Rect::from_min_size(r.min+region.position*zoom,if region.collapsed{region.interface.collapsed_size()*zoom}else{region.size*zoom})
    }
    pub(super) fn collapsed(&self,index:usize)->bool{self.regions.get(&index).is_some_and(|r|r.collapsed)}
    pub(super) fn toggle_rect(&self,index:usize,r:Rect,zoom:f32)->Rect{let rect=self.rect(index,r,zoom);Rect::from_min_size(rect.right_top()+egui::vec2(-29.,5.)*zoom,egui::vec2(24.,24.)*zoom)}
    pub(super) fn input_port(&self,index:usize,r:Rect,zoom:f32,row:Option<usize>)->egui::Pos2{
        let region=&self.regions[&index];let row=row.and_then(|row|region.interface.properties.get(row).copied()).unwrap_or(0);self.named_port(index,r,zoom,row)
    }
    pub(super) fn named_port(&self,index:usize,r:Rect,zoom:f32,row:usize)->egui::Pos2{let rect=self.rect(index,r,zoom);egui::pos2(rect.left(),rect.top()+(54.+row as f32*20.)*zoom)}
    pub(super) fn value_port(&self,index:usize,r:Rect,zoom:f32,value:&str)->egui::Pos2{self.named_port(index,r,zoom,self.regions[&index].interface.row(value))}
    pub(super) fn parameter_ports(&self,index:usize,r:Rect,zoom:f32,name:&str)->Vec<egui::Pos2>{self.regions[&index].interface.parameter_rows(name).into_iter().map(|row|self.named_port(index,r,zoom,row)).collect()}
    pub(super) fn output_port(&self,index:usize,r:Rect,zoom:f32)->egui::Pos2{let rect=self.rect(index,r,zoom);egui::pos2(rect.right(),rect.top()+54.*zoom)}
    pub(super) fn source_port(&self,index:usize,group:&stages::Group,r:Rect,zoom:f32,positions:&HashMap<usize,Vec2>,port:&super::operations::Port)->egui::Pos2{
        if self.collapsed(index){return self.output_port(index,r,zoom);}
        if let Some(source)=port.from{
            let row=port.channel.as_ref().and_then(|channel|output_channels(group.operations.nodes[source].ty).iter().position(|value|value==channel)).map_or(0,|row|row+1);
            let point=positions.get(&key(index,source)).copied().unwrap_or(default_point(group,source));let width=if self.previews.contains(&(index,source)){node_width(group,source).max(148.)}else{node_width(group,source)};
            r.min+(point+egui::vec2(width,port_top(group,source)+row as f32*18.))*zoom
        }else{self.value_port(index,r,zoom,&port.value)}
    }
    pub(super) fn finish_tools(&mut self,ui:&mut egui::Ui,canvas:Rect,response:&egui::Response,zoom:f32,positions:&mut HashMap<usize,Vec2>){
        use super::tools::Target;
        self.tools.finish(ui,canvas,response);
        let delta=self.tools.delta/zoom.max(0.01);
        if delta!=Vec2::ZERO{
            self.move_selection(delta,positions);
            ui.ctx().request_repaint();
        }
        if ui.input(|i|!i.pointer.primary_down())&&std::mem::take(&mut self.tools.moving_nodes){
            for target in &self.tools.selected{let Target::Position(key)=target else{continue;};if *key>=10000&&*key<10000+64*1024{
                let group=(*key-10000)/1024;let node=(*key-10000)%1024;
                if let (Some(region),Some(point),Some(rect))=(self.regions.get_mut(&group),positions.get(key),self.tools.rect(*target)){
                    let center=*point+rect.size()/zoom*0.5;let bounds=Rect::from_min_size(egui::Pos2::ZERO+region.position,region.size);
                    if can_join(region.connected.contains(&node),egui::Pos2::ZERO+center,bounds){region.members.insert(node);}else{region.members.remove(&node);}
                }
            }}
        }
    }
    fn move_selection(&mut self,delta:Vec2,positions:&mut HashMap<usize,Vec2>){
        use super::tools::Target;
        let groups=self.tools.selected.iter().filter_map(|target|if let Target::Group(index)=target{Some(*index)}else{None}).collect::<std::collections::HashSet<_>>();
        for &index in &groups {if let Some(region)=self.regions.get(&index){
            if let Some(origin)=positions.get_mut(&(1000+index)){*origin+=delta;}
            for (&id,point) in positions.iter_mut(){if id>=key(index,0)&&id<key(index,1024)&&!region.members.contains(&(id-key(index,0))){*point-=delta;}}
        }}
        for target in &self.tools.selected {if let Target::Position(id)=target {
            let inherited=if *id>=10000&&*id<10000+64*1024 {let group=(*id-10000)/1024;let node=(*id-10000)%1024;groups.contains(&group)&&self.regions.get(&group).is_some_and(|r|r.members.contains(&node))}else{false};
            if !inherited {if let Some(point)=positions.get_mut(id){*point+=delta;}}
        }}
    }
    pub(super) fn prepare(&mut self,ui:&mut egui::Ui,group:&stages::Group,index:usize,r:&mut Rect,canvas:Rect,zoom:f32,positions:&mut HashMap<usize,Vec2>){
        let connected=reachable(&group.flow);
        let initial=bounds(group,index,*r,zoom,positions);
        let region=self.regions.entry(index).or_insert_with(||Region{position:(initial.min-r.min)/zoom,size:initial.size()/zoom,collapsed:false,interface:group.interface.clone(),connected:Default::default(),members:connected.iter().enumerate().filter_map(|(step,yes)|yes.then_some(step)).collect()});
        region.interface=group.interface.clone();
        region.connected=connected.iter().enumerate().filter_map(|(node,yes)|yes.then_some(node)).collect();
        region.members.retain(|step|region.connected.contains(step));
        for step in 0..group.operations.nodes.len(){positions.entry(key(index,step)).or_insert(default_point(group,step));}
        let displayed=if region.collapsed{region.interface.collapsed_size()}else{region.size};
        let rect=Rect::from_min_size(r.min+region.position*zoom,displayed*zoom);
        let toggle=Rect::from_min_size(rect.right_top()+egui::vec2(-29.,5.)*zoom,egui::vec2(24.,24.)*zoom);
        let button=ui.interact(toggle.intersect(canvas),ui.id().with(("function_fold",index)),Sense::click());
        if button.clicked(){region.collapsed=!region.collapsed;self.tools.selected.retain(|target|match target{super::tools::Target::Position(id)=>!(*id>=key(index,0)&&*id<key(index,group.operations.nodes.len())),super::tools::Target::Group(_)=>true});ui.ctx().request_repaint();}
        button.on_hover_text(tr(if region.collapsed{"展开函数"}else{"折叠函数"}));
        let displayed=if region.collapsed{region.interface.collapsed_size()}else{region.size};
        let rect=Rect::from_min_size(r.min+region.position*zoom,displayed*zoom);
        // Collapsed functions behave like a compact node: their body is a
        // drag surface too. Reserve the right column for the fold button/ports.
        let header=group_drag_rect(rect,zoom,region.collapsed);
        self.tools.node(ui,super::tools::Target::Group(index),rect,header,canvas,zoom).on_hover_cursor(egui::CursorIcon::Grab);
        if !region.collapsed{
            let rect=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
            let handle=Rect::from_min_size(rect.right_bottom()-egui::vec2(18.,18.),egui::vec2(18.,18.));
            let resize=ui.interact(handle.intersect(canvas),ui.id().with(("scope_block_resize",index)),Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeNwSe);
            if resize.dragged_by(egui::PointerButton::Primary){region.size=(region.size+resize.drag_delta()/zoom).clamp(egui::vec2(360.,region.interface.collapsed_size().y.max(200.)),egui::vec2(24000.,24000.));}
        }

    }
}
fn group_drag_rect(rect:Rect,zoom:f32,collapsed:bool)->Rect{
    Rect::from_min_max(rect.min,egui::pos2((rect.right()-34.*zoom).max(rect.left()),if collapsed{rect.bottom()}else{rect.top()+34.*zoom}))
}
#[cfg(test)]
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
fn key(group:usize,step:usize)->usize{10000+group*1024+step}
fn output_channels(ty:super::operations::Ty)->Vec<String>{
    let mut channels=Vec::new();if ty.width==4{channels.push(if ty.color{"RGB"}else{"XYZ"}.into());}
    if ty.width>1{channels.extend(if ty.color{"RGBA"}else{"XYZW"}.chars().take(ty.width).map(|c|c.to_string()));}channels
}
fn output_rows(group:&stages::Group,node:usize)->usize{1+output_channels(group.operations.nodes[node].ty).len()}
fn port_top(group:&stages::Group,node:usize)->f32{if !group.operations.nodes[node].target.is_empty()&&group.operations.nodes[node].target!="return"{54.}else{40.}}
fn height(group:&stages::Group,node:usize)->f32{port_top(group,node)+group.operations.nodes[node].inputs.len().max(output_rows(group,node)).max(1) as f32*18.}
const NODE_WIDTH:f32=100.;
fn node_width(group:&stages::Group,node:usize)->f32{
    let title=group.operations.nodes[node].title.as_str();
    if ["Add","Subtract","Multiply","Divide","Remainder","Negate","Positive","Not","Bitwise Not","Equal","Not Equal","Less","Greater","Less Equal","Greater Equal","And","Or","Assign","Return","Constant","Convert","Lerp","Power","Saturate","Dot","Normalize","min","max","clamp","step","smoothstep","abs","sin","cos","frac","floor","ceil","sqrt","exp","log","cross","length","distance"].contains(&title)||title.starts_with("Swizzle ")||probe::width(title).is_some(){NODE_WIDTH}else{200.}
}
pub(super) fn arrange(group:&stages::Group)->(Vec<Vec2>,Vec2){
    let mut ranks=Vec::<usize>::new();let mut widths=Vec::<f32>::new();
    for node in 0..group.operations.nodes.len(){
        let rank=group.flow.reads[node].iter().filter_map(|edge|edge.from).filter(|from|*from<node).map(|from|ranks[from]+1).max().unwrap_or(0);
        widths.resize(widths.len().max(rank+1),0.);widths[rank]=widths[rank].max(node_width(group,node));ranks.push(rank);
    }
    let mut left=260.;let columns=widths.iter().map(|width|{let x=left;left+=width+60.;x}).collect::<Vec<_>>();
    let mut rows=HashMap::<usize,f32>::new();let points=ranks.into_iter().enumerate().map(|(node,rank)|{let row=rows.entry(rank).or_insert(80.);let p=egui::vec2(columns[rank],*row);*row+=height(group,node)+24.;p}).collect();
    (points,egui::vec2(left,80.))
}
fn default_point(group:&stages::Group,step:usize)->Vec2{group.flow.layout[step]}
pub(super) fn size(group:&stages::Group,index:usize,positions:&HashMap<usize,Vec2>)->Vec2{bounds(group,index,Rect::from_min_size(egui::Pos2::ZERO,Vec2::ZERO),1.,positions).size()}
pub(super) fn step_ports(group:&stages::Group,index:usize,r:Rect,zoom:f32,positions:&HashMap<usize,Vec2>,step:usize)->(egui::Pos2,egui::Pos2){
    let node=step;
    let point=positions.get(&key(index,node)).copied().unwrap_or(default_point(group,node));
    let left=r.min+(point+egui::vec2(0.,port_top(group,node)))*zoom;(left,left+egui::vec2(node_width(group,node)*zoom,0.))
}
/// Visual extent only; containment never changes explicit region membership.
pub(super) fn bounds(group:&stages::Group,index:usize,r:Rect,zoom:f32,positions:&HashMap<usize,Vec2>)->Rect{
    let mut bounds=Rect::from_min_size(r.min,egui::vec2(360.,group.interface.collapsed_size().y)*zoom);
    for node in 0..group.operations.nodes.len(){if group.interface.merged==Some(node){continue;}
        let p=positions.get(&key(index,node)).copied().unwrap_or(default_point(group,node));bounds=bounds.union(Rect::from_min_size(r.min+p*zoom,egui::vec2(node_width(group,node)+160.,height(group,node)+28.)*zoom));
    }bounds.expand(10.*zoom)
}
pub(super) fn draw(ui:&mut egui::Ui,group:&stages::Group,index:usize,r:Rect,canvas:Rect,zoom:f32,positions:&mut HashMap<usize,Vec2>,areas:&mut Areas,_inputs:&[Input],selected:Option<&probe::Probe>,mini:&mut mini::Previews,source:Option<&mini::Source>)->Option<probe::Probe>{
    let painter=ui.painter().with_clip_rect(canvas);let tint=if group.stage==stages::Stage::Vert{Color32::from_rgb(95,175,225)}else{Color32::from_rgb(185,140,205)};
    let interface=&group.interface;
    let text_color=ui.visuals().text_color();
    for (row,port) in interface.inputs.iter().enumerate(){
        let point=areas.named_port(index,r,zoom,row);painter.circle_filled(point,3.6*zoom,if port.ty.texture{Color32::from_rgb(95,175,225)}else if port.ty.color{Color32::from_rgb(225,190,90)}else{tint});
        let clip=Rect::from_min_size(point+egui::vec2(8.,-8.)*zoom,egui::vec2(145.,18.)*zoom).intersect(canvas);
        let label=format!("{} · {}",port.name,port.dtype);painter.with_clip_rect(clip).text(point+egui::vec2(8.*zoom,0.),egui::Align2::LEFT_CENTER,&label,egui::FontId::proportional(10.*zoom),text_color);
        ui.interact(clip,ui.id().with(("function_boundary_input",index,row)),Sense::hover()).on_hover_text(label);
    }
    let output=areas.output_port(index,r,zoom);
    let return_kind=if interface.return_ty.width<=1&&!interface.return_ty.texture{group.result_type.as_str()}else{interface.return_ty.label()};
    let label=format!("{} · {}",if interface.single_return{interface.return_name.as_str()}else{tr("多路径返回")},return_kind);
    let clip=Rect::from_min_size(output-egui::vec2(153.,8.)*zoom,egui::vec2(145.,18.)*zoom).intersect(canvas);
    painter.circle_filled(output,4.*zoom,tint);painter.with_clip_rect(clip).text(output-egui::vec2(8.*zoom,0.),egui::Align2::RIGHT_CENTER,&label,egui::FontId::proportional(10.*zoom),text_color);
    ui.interact(clip,ui.id().with(("function_boundary_return",index)),Sense::hover()).on_hover_text(format!("{}\n{}",group.signature,group.statements.last().map_or("",String::as_str)));
    for (row,label) in interface.channels().iter().enumerate(){let point=output+egui::vec2(0.,(row+1)as f32*20.*zoom);painter.circle_stroke(point,3.*zoom,Stroke::new(1_f32,tint));painter.text(point-egui::vec2(8.*zoom,0.),egui::Align2::RIGHT_CENTER,label,egui::FontId::proportional(10.*zoom),text_color);}
    if areas.collapsed(index){return None;}
    let sources=group.operations.nodes.iter().map(|operation|operation.inputs.iter().map(|port|areas.source_port(index,group,r,zoom,positions,port)).collect::<Vec<_>>()).collect::<Vec<_>>();
    let returned=if interface.single_return{group.flow.outputs.first().map(|&node|{if interface.merged==Some(node){sources[node][0]}else{let operation=&group.operations.nodes[node];areas.source_port(index,group,r,zoom,positions,&super::operations::Port{name:String::new(),value:operation.expression.clone(),from:Some(node),channel:None,external:false,ty:operation.ty})}})}else{None};
    let mut region=areas.regions.remove(&index);
    let members_reachable=reachable(&group.flow);
    let fill=if ui.visuals().dark_mode{Color32::from_rgb(38,40,46)}else{Color32::from_rgb(242,244,248)};
    let text_color=ui.visuals().text_color();
    let panel=|rect:Rect,title:&str,accent:Color32|{
        painter.rect_filled(rect,4.,fill);painter.rect_stroke(rect,4.,Stroke::new(1.0_f32,accent.gamma_multiply(0.7)),egui::StrokeKind::Inside);
        let header=Rect::from_min_size(rect.min,egui::vec2(rect.width(),28.*zoom));painter.rect_filled(header,4.,accent.gamma_multiply(0.3));
        painter.with_clip_rect(header.shrink2(egui::vec2(6.*zoom,0.)).intersect(canvas)).text(header.left_center()+egui::vec2(8.*zoom,0.),egui::Align2::LEFT_CENTER,title,egui::FontId::proportional(12.*zoom),text_color);
    };
    let nodes=(0..group.operations.nodes.len()).map(|node|{let point=*positions.entry(key(index,node)).or_insert(default_point(group,node));let extra=if areas.previews.contains(&(index,node)){144.}else{0.};Rect::from_min_size(r.min+point*zoom,egui::vec2(if extra>0.{node_width(group,node).max(148.)}else{node_width(group,node)},height(group,node)+extra)*zoom)}).collect::<Vec<_>>();
    let wire=|from:egui::Pos2,to:egui::Pos2,tint:Color32|{painter.add(egui::epaint::CubicBezierShape::from_points_stroke(super::wire_points(from,to,zoom),false,Color32::TRANSPARENT,Stroke::new(1.2_f32,tint)));};
    let port_color=|ty:super::operations::Ty|if ty.texture{Color32::from_rgb(115,165,225)}else if ty.color{Color32::from_rgb(215,192,126)}else{Color32::from_rgb(170,186,195)};
    for (node,operation) in group.operations.nodes.iter().enumerate(){if interface.merged==Some(node){continue;}for (row,port) in operation.inputs.iter().enumerate(){
        if port.from.is_none()&&!port.external{continue;}
        wire(sources[node][row],egui::pos2(nodes[node].left(),nodes[node].top()+(port_top(group,node)+row as f32*18.)*zoom),port_color(port.ty));
    }}
    if let Some(from)=returned{wire(from,output,tint);}
    let mut request=None;
    for (node,rect) in nodes.iter().copied().enumerate(){
        if interface.merged==Some(node)||!rect.intersects(canvas){continue;}
        let operation=&group.operations.nodes[node];let candidate=&operation.probe;
        let active=selected.is_some_and(|p|candidate.as_ref().is_ok_and(|candidate|p.function==candidate.function&&p.step==candidate.step&&p.vertex==candidate.vertex&&p.expression==candidate.expression));
        let accent=if operation.title=="Texture Sample"{Color32::from_rgb(75,135,210)}else if operation.title=="Constant"{Color32::from_rgb(100,155,85)}else if operation.fallback{Color32::from_rgb(180,145,90)}else{Color32::from_rgb(100,135,155)};
        panel(rect,if operation.fallback{tr("源码语句")}else{&operation.title},accent);
        if active{painter.rect_stroke(rect,4.,Stroke::new(2.0_f32,Color32::from_rgb(225,155,65)),egui::StrokeKind::Inside);}
        if !operation.target.is_empty()&&operation.target!="return"{let clip=Rect::from_min_size(rect.min+egui::vec2(6.,29.)*zoom,egui::vec2(rect.width()/zoom-12.,16.)*zoom).intersect(canvas);painter.with_clip_rect(clip).text(rect.min+egui::vec2(6.,36.)*zoom,egui::Align2::LEFT_CENTER,&operation.target,egui::FontId::monospace(9.*zoom),ui.visuals().weak_text_color());}
        let previewable=operation.ty.color&&candidate.is_ok();
        let toggle=Rect::from_min_size(rect.right_top()+egui::vec2(-23.,3.)*zoom,egui::vec2(20.,22.)*zoom);
        let header=Rect::from_min_max(rect.min,egui::pos2(if previewable{toggle.left()}else{rect.right()},rect.top()+28.*zoom));
        // Register the node background first; the later button owns overlapping clicks.
        let response=areas.tools.node(ui,super::tools::Target::Position(key(index,node)),rect,header,canvas,zoom);
        if previewable{
            let open=areas.previews.contains(&(index,node));
            let response=ui.interact(toggle.intersect(canvas),ui.id().with(("operation_preview",index,node)),Sense::click()).on_hover_text(tr("展开 / 收起节点预览"));
            let visuals=ui.style().interact(&response);
            painter.rect(toggle,3.*zoom,if open{ui.visuals().selection.bg_fill}else{visuals.bg_fill},visuals.bg_stroke,egui::StrokeKind::Inside);
            let icon=Rect::from_center_size(toggle.center(),egui::vec2(16.,16.)*zoom);
            paint_preview_eye(&painter,icon,if open{ui.visuals().selection.stroke.color}else{visuals.fg_stroke.color});
            if response.clicked(){ui.ctx().request_repaint();if open{areas.previews.remove(&(index,node));}else{areas.previews.insert((index,node));
                // Make room below the expanded node without rearranging unrelated scopes.
                let expanded=Rect::from_min_size(rect.min,egui::vec2(rect.width().max(148.*zoom),rect.height()+144.*zoom));let mut bottom=expanded.bottom();
                for (other,other_rect) in nodes.iter().enumerate(){if other!=node&&other_rect.left()<expanded.right()&&other_rect.right()>expanded.left()&&other_rect.top()>=rect.bottom()&&other_rect.top()<bottom+16.*zoom{
                    let delta=(bottom+16.*zoom-other_rect.top())/zoom;positions.get_mut(&key(index,other)).unwrap().y+=delta;bottom=other_rect.bottom()+delta*zoom;
                    if let Some(region)=&mut region{let moved=positions[&key(index,other)]+other_rect.size()/zoom;region.size=region.size.max(moved-region.position+egui::vec2(12.,12.));}
                }}
                if let Some(region)=&mut region{region.size=region.size.max((expanded.max-r.min)/zoom-region.position+egui::vec2(12.,12.));}
            }}
        }
        if response.drag_stopped(){if let Some(region)=&mut region{
            let center=r.min+(positions[&key(index,node)]+rect.size()/zoom*0.5)*zoom;let bounds=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
            if can_join(members_reachable[node],center,bounds){region.members.insert(node);}else{region.members.remove(&node);}
        }}
        if let Some(region)=&mut region{
            response.clone().context_menu(|ui|{let bounds=Rect::from_min_size(r.min+region.position*zoom,region.size*zoom);
                if ui.add_enabled(can_join(members_reachable[node],rect.center(),bounds),egui::Button::new(tr("加入范围框"))).clicked(){region.members.insert(node);ui.close_menu();}
                if ui.button(tr("移出范围框")).clicked(){region.members.remove(&node);ui.close_menu();}
            });
        }
        let detail=format!("{} · {} {}\n{}\n{}{}",group.name,tr("步骤"),operation.step+1,operation.target,group.statements[operation.step],format!("\n{}\n{}",operation.expression,candidate.as_ref().err().map_or("",String::as_str)));
        response.on_hover_text(detail);
        let body=Rect::from_min_max(rect.min+egui::vec2(4.,28.)*zoom,rect.max-egui::vec2(4.,4.)*zoom);let labels=painter.with_clip_rect(body.intersect(canvas));
        for (row,port) in operation.inputs.iter().enumerate(){
            let point=egui::pos2(rect.left(),rect.top()+(port_top(group,node)+row as f32*18.)*zoom);let tint=port_color(port.ty);painter.circle_filled(point,3.2*zoom,tint);
            let named=port.value.split('.').all(|part|{let mut chars=part.chars();chars.next().is_some_and(|c|c.is_ascii_alphabetic()||c=='_')&&chars.all(|c|c.is_ascii_alphanumeric()||c=='_')});
            let variable=if named||port.from.is_none(){port.value.as_str()}else{port.from.map(|source|group.operations.nodes[source].target.as_str()).filter(|name|!name.is_empty()&&*name!="return").unwrap_or("")};
            let text=if variable.is_empty(){port.name.clone()}else{format!("{} {}",port.name,variable)};
            let limit=((rect.width()/zoom-58.)/5.5).max(6.)as usize;let text=text.chars().take(limit).collect::<String>();let input_clip=Rect::from_min_max(body.min,egui::pos2(rect.right()-44.*zoom,body.bottom()));labels.with_clip_rect(input_clip.intersect(canvas)).text(point+egui::vec2(7.*zoom,0.),egui::Align2::LEFT_CENTER,text,egui::FontId::proportional(10.*zoom),text_color);
            ui.interact(Rect::from_min_size(point+egui::vec2(0.,-8.*zoom),egui::vec2((rect.width()/zoom-48.).max(40.),16.)*zoom).intersect(canvas),ui.id().with(("operation_port",index,node,row)),Sense::hover()).on_hover_text(format!("{} · {}",port.value,port.ty.label()));
        }
        let main=egui::pos2(rect.right(),rect.top()+port_top(group,node)*zoom);painter.circle_filled(main,3.5*zoom,port_color(operation.ty));
        labels.text(main-egui::vec2(7.*zoom,0.),egui::Align2::RIGHT_CENTER,operation.ty.label(),egui::FontId::proportional(10.*zoom),text_color);
        let channels=output_channels(operation.ty);
        for (row,label) in channels.iter().enumerate(){let point=main+egui::vec2(0.,(row+1)as f32*18.*zoom);let tint=match label.as_str(){"R"|"X"=>Color32::from_rgb(220,95,95),"G"|"Y"=>Color32::from_rgb(95,190,125),"B"|"Z"=>Color32::from_rgb(100,135,220),_=>Color32::from_gray(190)};painter.circle_stroke(point,3.*zoom,Stroke::new(1.*zoom,tint));labels.text(point-egui::vec2(7.*zoom,0.),egui::Align2::RIGHT_CENTER,label,egui::FontId::proportional(10.*zoom),tint);}
        if areas.previews.contains(&(index,node)){if let Ok(probe)=candidate{let preview=Rect::from_center_size(egui::pos2(rect.center().x,rect.top()+(height(group,node)+68.)*zoom),egui::vec2(128.,128.)*zoom);mini.show(ui,preview,canvas,probe,source);if ui.interact(preview.intersect(canvas),ui.id().with(("inspect_operation",index,node)),Sense::click()).clicked(){request=Some(probe.clone());}}}
    }

    if let Some(region)=region{areas.regions.insert(index,region);}
    request
}

// Use the canvas painter for the whole control, including clipping. No image loader needed.
fn paint_preview_eye(painter:&egui::Painter,rect:Rect,color:Color32){
    let point=|x:f32,y:f32|rect.min+egui::vec2(x*rect.width()/24.,y*rect.height()/24.);
    let stroke=Stroke::new((rect.width()*1.8/24.).max(1.),color);
    for y in [4.,20.] {
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [point(2.,12.),point(8.,y),point(16.,y),point(22.,12.)],false,Color32::TRANSPARENT,stroke));
    }
    painter.circle_stroke(rect.center(),rect.width()*3./24.,stroke);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eye_button_clicks_toggle_snapshot_without_selecting_node(){
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nsampler2D _MainTex;float4 f(float2 uv:TEXCOORD0):SV_Target{float4 color=tex2D(_MainTex,uv);return color;}\nENDCG",&[]);
        let group=&graph.groups[0];
        let node=group.operations.nodes.iter().position(|n|n.title=="Texture Sample").unwrap();
        let mut areas=Areas::default();
        areas.regions.insert(0,Region{position:Vec2::ZERO,size:egui::vec2(1800.,800.),collapsed:false,interface:group.interface.clone(),connected:Default::default(),members:Default::default()});
        let mut positions=(0..group.operations.nodes.len()).map(|n|(key(0,n),egui::vec2(200.+n as f32*300.,150.))).collect::<HashMap<_,_>>();
        let point=egui::pos2(positions[&key(0,node)].x+node_width(group,node)-13.,164.);
        let ctx=egui::Context::default();let mut mini=mini::Previews::default();
        let button=|pressed|egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE};
        let frames=vec![vec![],vec![egui::Event::PointerMoved(point),button(true)],vec![button(false)],vec![button(true)],vec![button(false)]];
        for (frame,events) in frames.into_iter().enumerate(){
            let screen=Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(2000.,1000.));
            let _=ctx.run(egui::RawInput{screen_rect:Some(screen),events,..Default::default()},|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|{
                    areas.tools.begin(ui,screen);
                    draw(ui,group,0,screen,screen,1.,&mut positions,&mut areas,&[],None,&mut mini,None);
                });
            });
            if frame==2 {assert!(areas.previews.contains(&(0,node)));assert!(areas.tools.selected.is_empty());}
            if frame==4 {assert!(!areas.previews.contains(&(0,node)));assert!(areas.tools.selected.is_empty());}
        }
    }
    #[test]
    fn multi_selection_moves_each_selected_position_once(){
        use crate::material_graph::tools::Target;
        let mut areas=Areas::default();areas.tools.selected.extend([Target::Position(1),Target::Position(2)]);
        let mut positions=HashMap::from([(1,egui::vec2(20.,30.)),(2,egui::vec2(100.,50.)),(3,egui::vec2(4.,5.))]);
        areas.move_selection(egui::vec2(7.,-2.),&mut positions);
        assert_eq!(positions[&1],egui::vec2(27.,28.));assert_eq!(positions[&2],egui::vec2(107.,48.));assert_eq!(positions[&3],egui::vec2(4.,5.));
    }
    #[test]
    fn collapsed_function_body_drags_but_fold_button_is_reserved(){
        let rect=Rect::from_min_size(egui::pos2(100.,100.),egui::vec2(340.,240.));
        assert!(group_drag_rect(rect,1.,true).contains(rect.center()));
        assert!(!group_drag_rect(rect,1.,false).contains(rect.center()));
        assert!(!group_drag_rect(rect,1.,true).contains(rect.right_top()+egui::vec2(-17.,17.)));
    }
    #[test]
    fn group_and_member_selection_moves_once_and_preserves_excluded_nodes(){
        use crate::material_graph::tools::Target;
        let mut areas=Areas::default();
        areas.regions.insert(0,Region{position:Vec2::ZERO,size:egui::vec2(600.,400.),collapsed:false,interface:Default::default(),members:std::collections::HashSet::from([0]),connected:Default::default()});
        areas.tools.selected.extend([Target::Group(0),Target::Position(key(0,0)),Target::Position(5)]);
        let mut positions=HashMap::from([(1000,egui::vec2(10.,10.)),(key(0,0),egui::vec2(20.,20.)),(key(0,1),egui::vec2(30.,30.)),(5,Vec2::ZERO)]);
        areas.move_selection(egui::vec2(7.,3.),&mut positions);
        assert_eq!(positions[&1000]+positions[&key(0,0)],egui::vec2(37.,33.));
        assert_eq!(positions[&1000]+positions[&key(0,1)],egui::vec2(40.,40.));
        assert_eq!(positions[&5],egui::vec2(7.,3.));
    }
    #[test]
    fn boundary_output_tracks_resize_and_folding_preserves_expanded_size(){
        let mut areas=Areas::default();let r=Rect::from_min_size(egui::Pos2::ZERO,Vec2::splat(1000.));
        areas.regions.insert(0,Region{position:Vec2::ZERO,size:egui::vec2(600.,400.),collapsed:false,interface:Default::default(),members:Default::default(),connected:Default::default()});
        assert_eq!(areas.output_port(0,r,1.).x,600.);
        areas.regions.get_mut(&0).unwrap().size.x=800.;
        assert_eq!(areas.output_port(0,r,1.).x,800.);
        areas.regions.get_mut(&0).unwrap().collapsed=true;
        assert_eq!(areas.rect(0,r,1.).width(),340.);
        areas.regions.get_mut(&0).unwrap().collapsed=false;
        assert_eq!(areas.rect(0,r,1.).size(),egui::vec2(800.,400.));
    }
    #[test]
    fn constants_supporting_connected_calculations_join_but_unused_constants_do_not(){
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float3 c=color.rgb;float3 avgColor=float3(0.5,0.5,0.5);float3 unused=float3(1,1,1);c=lerp(avgColor,c,0.5);return color;}\nENDCG",&[]);
        let logical=plan(&graph.groups[0],"CGPROGRAM\nfloat4 f(float4 color){float3 c=color.rgb;float3 avgColor=float3(0.5,0.5,0.5);float3 unused=float3(1,1,1);c=lerp(avgColor,c,0.5);return color;}\nENDCG",&[]);let connected=reachable(&logical);
        assert!(connected[0]);assert!(connected[1]);assert!(!connected[2]);assert!(connected[3]);
    }
    #[test]
    fn dependencies_move_forward_and_independent_steps_share_a_column(){
        let graph=stages::parse("CGPROGRAM\n#pragma fragment f\nfloat4 f(float4 color):SV_Target{float4 a=color;float4 b=color;float4 c=a+b;return c;}\nENDCG",&[]);
        let group=&graph.groups[0];let p=group.operations.roots.iter().map(|root|group.flow.layout[*root]).collect::<Vec<_>>();
        assert_eq!(p[0].x,p[1].x);assert_ne!(p[0].y,p[1].y);
        assert!(p[2].x>p[0].x);assert!(p[3].x>p[2].x);
        assert!(group.flow.output.x>p[3].x+NODE_WIDTH);
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
        let logical=plan(group,source,&[]);
        let writes=logical.reads[2].iter().filter_map(|edge|edge.from).collect::<std::collections::HashSet<_>>();
        assert_eq!(writes,std::collections::HashSet::from([0,1]));
        let returns=logical.reads[4].iter().filter_map(|edge|edge.from).collect::<std::collections::HashSet<_>>();
        assert_eq!(returns,std::collections::HashSet::from([1,2]));
        assert_eq!(reachable(&logical),vec![true,true,true,false,true]);
        assert_eq!(logical.outputs,vec![4]);
        assert_eq!(logical.visual,vec![true,false,true,false,true]);
    }
    #[test]
    fn spatial_overlap_never_joins_unconnected_nodes_or_changes_independent_output_position() {
        let bounds=Rect::from_min_size(egui::pos2(100.,100.),egui::vec2(400.,400.));
        assert!(!can_join(false,bounds.center(),bounds));
        assert!(!can_join(true,egui::pos2(0.,0.),bounds));
        assert!(can_join(true,bounds.center(),bounds));
        let region=Region{position:Vec2::ZERO,size:egui::vec2(400.,400.),collapsed:false,interface:Default::default(),connected:Default::default(),members:Default::default()};
        assert!(region.members.is_empty());
    }
}
