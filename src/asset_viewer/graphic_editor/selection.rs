//! Session-local pixel selections shared by the painting tools.
use super::*;
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Mode { #[default] Rectangle, Ellipse, Lasso, Polygon }
impl Mode {
    pub const ALL: [Self;4] = [Self::Rectangle,Self::Ellipse,Self::Lasso,Self::Polygon];
    pub fn next(self)->Self { Self::ALL[(Self::ALL.iter().position(|m|*m==self).unwrap()+1)%4] }
    pub fn label(self)-> &'static str { match self { Self::Rectangle=>"矩形选取",Self::Ellipse=>"椭圆选取",Self::Lasso=>"套索选取",Self::Polygon=>"多边形选取" } }
    pub fn icon(self)->crate::icons::Icon { match self {Self::Rectangle=>crate::icons::Icon::SelectionRect,Self::Ellipse=>crate::icons::Icon::SelectionEllipse,Self::Lasso=>crate::icons::Icon::SelectionLasso,Self::Polygon=>crate::icons::Icon::SelectionPolygon} }
}
type Pixel = (usize,u32,u32);
type World = std::collections::HashSet<(i32,i32)>;
#[derive(Clone,Copy,Default)]
enum Combine { #[default] Replace, Add, Subtract }
struct MoveDrag { start:Pos2, source:World, cut:bool, offset:(i32,i32) }
#[derive(Clone)]
struct Floating { layer:usize, base:std::sync::Arc<Vec<RgbaImage>>, pixels:std::sync::Arc<Vec<(i32,i32,Rgba<u8>)>>, offset:(i32,i32), revision:u64 }
impl PartialEq for Floating {fn eq(&self,other:&Self)->bool{self.layer==other.layer && self.offset==other.offset && self.base==other.base && self.pixels==other.pixels}}
#[derive(Clone,PartialEq)]
pub(super) struct HistorySelection {mode:Mode,mask:Option<std::collections::HashSet<Pixel>>,world:World,cut_layer:Option<usize>,floating:Option<Floating>}
impl HistorySelection {
    pub(super) fn bytes(&self)->usize {
        self.mask.as_ref().map_or(0,|m|m.len()*std::mem::size_of::<Pixel>()*2)+self.world.len()*std::mem::size_of::<(i32,i32)>()*2+
        self.floating.as_ref().map_or(0,|f|f.base.iter().map(|i|i.as_raw().len()).sum::<usize>()+f.pixels.len()*std::mem::size_of::<(i32,i32,Rgba<u8>)>())
    }
}
#[derive(Default)]
pub(super) struct Selection {
    pub(super) generation:u64,
    pub mode: Mode,
    pub mask: Option<std::collections::HashSet<Pixel>>,
    pub draft: Vec<Pos2>,
    world:World,
    edges:Vec<[Pos2;2]>,
    contours:Vec<Vec<Pos2>>,
    combine:Combine,
    proportional:bool,
    moving:Option<MoveDrag>,
    cut_layer:Option<usize>,
    floating:Option<Floating>,
}
impl Selection {
    pub(super) fn guide_bounds(&self) -> Option<Rect> {
        let mut bounds = Rect::NOTHING;
        for &(x, y) in &self.world {
            let corner = Pos2::new(x as f32, y as f32);
            bounds.extend_with(corner);
            bounds.extend_with(corner + Vec2::splat(1.));
        }
        bounds.is_positive().then_some(bounds)
    }
    pub(super) fn history(&self)->HistorySelection{HistorySelection{mode:self.mode,mask:self.mask.clone(),world:self.world.clone(),cut_layer:self.cut_layer,floating:self.floating.clone()}}
    pub(super) fn restore(&mut self,state:HistorySelection,revision:u64){self.mode=state.mode;self.mask=state.mask;self.world=state.world;self.cut_layer=state.cut_layer;self.floating=state.floating;if let Some(f)=self.floating.as_mut(){f.revision=revision;}self.draft.clear();self.moving=None;self.rebuild_edges();}
    pub(super) fn cursor(&self)->egui::CursorIcon{if self.moving.is_some(){egui::CursorIcon::Grabbing}else{egui::CursorIcon::Crosshair}}
    pub fn allows(&self,tile:usize,x:u32,y:u32)->bool {self.mask.as_ref().is_none_or(|m|m.contains(&(tile,x,y)))}
    pub fn clear(&mut self) {self.generation=self.generation.wrapping_add(1);self.mask=None;self.draft.clear();self.world.clear();self.edges.clear();self.contours.clear();self.moving=None;self.cut_layer=None;self.floating=None;}
    fn polygon_close_at(&self,pointer:Pos2,origin:Pos2,scale:f32,dpi:f32)->bool{self.draft.len()>=3&&self.draft.first().is_some_and(|first|(origin+first.to_vec2()*scale).distance(pointer)*dpi<=8.)}
    fn draft_preview(&self,origin:Pos2,scale:f32,pointer:Option<Pos2>,dpi:f32,diamond:Option<[u32;2]>)->(Vec<Pos2>,Option<(Pos2,bool)>){let mut points:Vec<_>=self.path(diamond).iter().map(|p|origin+p.to_vec2()*scale).collect();if self.mode==Mode::Polygon&&!self.draft.is_empty(){let first=points[0];let close=pointer.is_some_and(|p|self.polygon_close_at(p,origin,scale,dpi));if let Some(pointer)=pointer{points.push(if close{first}else{pointer});}(points,Some((first,close)))}else{if points.len()>1{points.push(points[0]);}(points,None)}}
    fn path(&self, diamond: Option<[u32; 2]>)->Vec<Pos2> {
        if self.draft.len()<2 {return self.draft.clone();}
        let a=self.draft[0];let b=*self.draft.last().unwrap();
        if self.proportional && matches!(self.mode, Mode::Rectangle | Mode::Ellipse) {
            let delta = proportional_drag_delta(b - a, 1.0);
            let bounds = Rect::from_two_pos(a, a + delta);
            let bounds = Rect::from_min_max(bounds.min, bounds.max + Vec2::splat(1.0));
            if self.mode == Mode::Ellipse {
                return ellipse_points(bounds);
            }
            if diamond.is_some() {
                let center = bounds.center();
                return vec![Pos2::new(center.x, bounds.top()), Pos2::new(bounds.right(), center.y),
                    Pos2::new(center.x, bounds.bottom()), Pos2::new(bounds.left(), center.y)];
            }
            return vec![bounds.left_top(), bounds.right_top(), bounds.right_bottom(), bounds.left_bottom()];
        }
        match self.mode {
            Mode::Rectangle if let Some(footprint) = diamond => {
                let delta = b - a;
                let [w, h] = super::diamond_size(
                    [f64::from(delta.x.abs()) + 1., f64::from(delta.y.abs()) + 1.],
                    footprint,
                );
                let anchor = a + egui::vec2(
                    if delta.x < 0. { 1. } else { 0. },
                    if delta.y < 0. { 1. } else { 0. },
                );
                let end = anchor + egui::vec2((w as f32).copysign(delta.x), (h as f32).copysign(delta.y));
                let r = Rect::from_two_pos(anchor, end);
                let center = r.center();
                vec![Pos2::new(center.x, r.top()), Pos2::new(r.right(), center.y),
                    Pos2::new(center.x, r.bottom()), Pos2::new(r.left(), center.y)]
            }
            Mode::Rectangle=>{let r=Rect::from_two_pos(a,b);vec![r.left_top(),r.right_top(),r.right_bottom(),r.left_bottom()]},
            Mode::Ellipse=>ellipse_points(Rect::from_two_pos(a,b)),
            _=>self.draft.clone(),
        }
    }
    fn rebuild_edges(&mut self) {
        self.generation=self.generation.wrapping_add(1);
        self.edges.clear();
        for &(x,y) in &self.world {
            let a=Pos2::new(x as f32,y as f32);let b=a+egui::vec2(1.,1.);
            for (neighbor,edge) in [((x-1,y),[Pos2::new(a.x,b.y),a]),((x+1,y),[Pos2::new(b.x,a.y),b]),((x,y-1),[a,Pos2::new(b.x,a.y)]),((x,y+1),[b,Pos2::new(a.x,b.y)])] {
                if !self.world.contains(&neighbor) {self.edges.push(edge);}
            }
        }
        self.contours.clear();
        let mut next=std::collections::BTreeMap::<(i32,i32),Vec<(i32,i32)>>::new();
        for edge in &self.edges{next.entry((edge[0].x as i32,edge[0].y as i32)).or_default().push((edge[1].x as i32,edge[1].y as i32));}
        while let Some(start)=next.keys().next().copied(){
            let mut current=start;let mut path=vec![Pos2::new(start.0 as f32,start.1 as f32)];
            loop{
                let to=next.get_mut(&current).and_then(|p|p.pop());
                if next.get(&current).is_some_and(|p|p.is_empty()){next.remove(&current);}
                let Some(to)=to else{break;};path.push(Pos2::new(to.0 as f32,to.1 as f32));current=to;if current==start{break;}
            }
            if path.len()>1{self.contours.push(path);}
        }
    }
}
fn ants(painter:&egui::Painter,points:&[Pos2]){
    let dpi=painter.ctx().pixels_per_point().max(0.1);let dash=4./dpi;
    let phase=painter.ctx().input(|i|i.time) as f32*12./dpi;let mut distance=0_f32;
    let align=|p:Pos2|Pos2::new((p.x*dpi).floor()/dpi+0.5/dpi,(p.y*dpi).floor()/dpi+0.5/dpi);
    for edge in points.windows(2){let length=(edge[1]-edge[0]).length();if length<0.0001{continue;}
        let dir=(edge[1]-edge[0])/length;let mut pos=0_f32;
        while pos<length {
            let offset=(distance+pos+phase).rem_euclid(dash*2.);let white=offset<dash;
            let step=if white{dash-offset}else{dash*2.-offset};let end=(pos+step.max(0.0001)).min(length);
            let a=align(edge[0]+dir*pos);let b=align(edge[0]+dir*end);
            if Rect::from_two_pos(a,b).expand(1.).intersects(painter.clip_rect()){painter.line_segment([a,b],egui::Stroke::new(1./dpi,if white{Color32::WHITE}else{Color32::BLACK}));}
            pos=end;
        }
        distance+=length;
    }
}
fn contains(path:&[Pos2],p:Pos2)->bool {
    let mut inside=false;let mut j=path.len()-1;
    for i in 0..path.len() {let a=path[i];let b=path[j];if (a.y>p.y)!=(b.y>p.y) && p.x<(b.x-a.x)*(p.y-a.y)/(b.y-a.y)+a.x {inside=!inside;}j=i;}
    inside
}

pub(super) fn ellipse_points(rect: Rect) -> Vec<Pos2> {
    (0..64).map(|i| {
        let angle = i as f32 * std::f32::consts::TAU / 64.;
        rect.center() + egui::vec2(angle.cos() * rect.width() / 2., angle.sin() * rect.height() / 2.)
    }).collect()
}
impl Editor {
    fn selection_hit(&self,point:Pos2,layout:bool,infinite:bool)->Option<Pixel> {
        if infinite {self.plane_hit(point.x as i32,point.y as i32,layout)}else{self.finite_plane_hit(point.x as i32,point.y as i32,layout)}
    }
    fn selection_contains(&self,point:Pos2,layout:bool,infinite:bool)->bool {
        self.selection.mask.as_ref().is_some_and(|mask|self.selection_hit(point,layout,infinite).is_some_and(|pixel|mask.contains(&pixel)))
    }
    fn rebuild_selection(&mut self,layout:bool,infinite:bool) {
        let mask=self.selection.world.iter().filter_map(|&(x,y)|self.selection_hit(Pos2::new(x as f32,y as f32),layout,infinite)).collect();
        self.selection.mask=Some(mask);self.selection.rebuild_edges();
    }
    pub(super) fn selection_context(&mut self,ui:&egui::Ui,response:&egui::Response,origin:Pos2,scale:f32,layout:bool,infinite:bool) {
        let inside=response.hover_pos().is_some_and(|p|{let p=(p-origin)/scale;self.selection_contains(Pos2::new(p.x.floor(),p.y.floor()),layout,infinite)});
        if inside || response.context_menu_opened() {
            response.context_menu(|ui| {if ui.button(tr("取消选区")).clicked(){self.selection.clear();ui.close_menu();}});
        }
        let _=ui;
    }
    pub(super) fn selection_input(&mut self,ui:&egui::Ui,response:&egui::Response,origin:Pos2,scale:f32,layout:bool,infinite:bool) {
        let Some(pointer)=response.interact_pointer_pos().or_else(||response.hover_pos()) else{return;};
        let local=(pointer-origin)/scale;let point=Pos2::new(local.x.floor(),local.y.floor());
        let modifiers=ui.input(|i|i.modifiers);
        if self.selection.cut_layer.is_some_and(|layer|layer!=self.active_layer){self.selection.cut_layer=None;self.selection.floating=None;}
        if self.selection.floating.as_ref().is_some_and(|f|f.revision!=self.revision){self.selection.floating=None;}
        let start=ui.input(|i|i.pointer.press_origin()).map(|pos|{let p=(pos-origin)/scale;Pos2::new(p.x.floor(),p.y.floor())}).unwrap_or(point);
        let dragging=response.drag_started_by(egui::PointerButton::Primary);
        let stopped=response.drag_stopped_by(egui::PointerButton::Primary);
        let clicked=response.clicked_by(egui::PointerButton::Primary);
        if dragging && self.selection.draft.is_empty() && !modifiers.shift && !modifiers.alt && self.selection_contains(start,layout,infinite) {
            let cut=modifiers.ctrl || self.selection.cut_layer==Some(self.active_layer);
            if cut && !self.move_allowed(self.active_layer){self.message=tr("当前图层已锁定或隐藏，不能绘制。").into();return;}
            self.finish_stroke();let offset=self.selection.floating.as_ref().map_or((0,0),|f|f.offset);
            self.selection.moving=Some(MoveDrag{start,source:self.selection.world.clone(),cut,offset});
        }
        if self.selection.moving.is_some() {
            let drag=self.selection.moving.as_mut().unwrap();
            let delta=point-drag.start;
            if modifiers.ctrl && delta!=Vec2::ZERO {drag.cut=true;}
            let cut=drag.cut;let source=drag.source.clone();let offset=drag.offset;
            if cut && !self.layer_editable(){self.selection.moving=None;return;}
            if cut && delta!=Vec2::ZERO && self.before.is_none(){
                self.before=Some(self.snapshot());
                if self.selection.floating.is_none(){self.prepare_floating(&source,layout,infinite);}
                self.selection.cut_layer=Some(self.active_layer);
            }
            self.selection.world=source.iter().map(|&(x,y)|(x+delta.x as i32,y+delta.y as i32)).collect();
            self.rebuild_selection(layout,infinite);
            if cut && self.selection.floating.is_some(){self.apply_floating((offset.0+delta.x as i32,offset.1+delta.y as i32),layout,infinite);}
            ui.output_mut(|o|o.cursor_icon=egui::CursorIcon::Grabbing);
            if stopped {self.selection.moving=None;if cut{self.finish_stroke();}}
            return;
        }
        if clicked && !modifiers.shift && !modifiers.alt && !modifiers.ctrl && self.selection.mask.is_some() && self.selection.draft.is_empty() {
            if !self.selection_contains(point,layout,infinite){self.selection.clear();}
            return;
        }
        if modifiers.ctrl && self.selection.draft.is_empty() && !self.selection_contains(start,layout,infinite) {return;}
        if dragging || (clicked && self.selection.draft.is_empty()) {
            self.selection.combine=if modifiers.shift{Combine::Add}else if modifiers.alt{Combine::Subtract}else{Combine::Replace};
        }
        let mut commit=false;
        match self.selection.mode {
            Mode::Polygon=>{
                if response.double_clicked_by(egui::PointerButton::Primary)&&self.selection.draft.len()>=3 {commit=true;}
                else if clicked {
                    if self.selection.polygon_close_at(pointer,origin,scale,ui.ctx().pixels_per_point()){commit=true;}
                    else if self.selection.draft.last()!=Some(&point){self.selection.draft.push(point);}
                }
                if self.selection.draft.len()>=3&&Shortcut::Confirm.take(ui.ctx(), false) {commit=true;}
            }
            mode=>{
                if dragging {self.selection.draft=vec![start,point]; self.selection.proportional=false;}
                if !self.selection.draft.is_empty() && (response.dragged_by(egui::PointerButton::Primary) || stopped) {
                    self.selection.proportional = modifiers.shift && matches!(mode, Mode::Rectangle | Mode::Ellipse);
                    if mode==Mode::Lasso {if self.selection.draft.last()!=Some(&point){self.selection.draft.push(point);}}
                    else if self.selection.proportional || (mode == Mode::Rectangle && self.diamond && !self.plain_image) {
                        // The pointer no longer reports a press origin on release.
                        // Keep the anchor captured when the drag started.
                        *self.selection.draft.last_mut().unwrap() = point;
                    }
                    else {*self.selection.draft.last_mut().unwrap()=point+egui::vec2(1.,1.);}
                }
                commit=stopped;
            }
        }
        if commit {self.commit_selection(layout,infinite);}
    }
    fn prepare_floating(&mut self,source:&World,layout:bool,infinite:bool){
        let mut base=self.tiles.clone();let mut points:Vec<_>=source.iter().copied().collect();points.sort_unstable();
        let mut unique=std::collections::BTreeMap::new();
        for (x,y) in points {if let Some((tile,px,py))=self.selection_hit(Pos2::new(x as f32,y as f32),layout,infinite){
            unique.entry((tile,px,py)).or_insert((x,y,*self.tiles[tile].get_pixel(px,py)));
            base[tile].put_pixel(px,py,Rgba([0;4]));
        }}
        self.selection.floating=Some(Floating{layer:self.active_layer,base:std::sync::Arc::new(base),pixels:std::sync::Arc::new(unique.into_values().collect()),offset:(0,0),revision:self.revision});
    }
    fn apply_floating(&mut self,offset:(i32,i32),layout:bool,infinite:bool){
        let Some(floating)=self.selection.floating.as_ref() else{return;};
        if floating.layer!=self.active_layer || !self.move_allowed(self.active_layer){return;}
        let mut tiles=floating.base.as_ref().clone();let mut targets=std::collections::BTreeMap::new();
        for &(x,y,color) in floating.pixels.iter() {
            if let Some(pixel)=self.selection_hit(Pos2::new((x+offset.0) as f32,(y+offset.1) as f32),layout,infinite){targets.insert(pixel,color);}
        }
        for ((tile,x,y),color) in targets {
            let old=*tiles[tile].get_pixel(x,y);tiles[tile].put_pixel(x,y,round_brush_blend(old,color,1.,false,BrushMode::Normal));
        }
        if tiles!=self.tiles {self.tiles=tiles;self.pixels=self.tiles[self.selected].clone();self.invalidate_textures();self.revision=self.revision.wrapping_add(1);}
        if let Some(floating)=self.selection.floating.as_mut(){floating.offset=offset;floating.revision=self.revision;}
    }
    #[cfg(test)]
    fn cut_move_selection(&mut self,source:&World,delta:Vec2,layout:bool,infinite:bool){
        if !self.move_allowed(self.active_layer){return;}
        self.finish_stroke();self.before=Some(self.snapshot());self.prepare_floating(source,layout,infinite);
        self.apply_floating((delta.x as i32,delta.y as i32),layout,infinite);self.selection.cut_layer=Some(self.active_layer);self.finish_stroke();
    }
    fn commit_selection(&mut self,layout:bool,infinite:bool) {
        let diamond = (self.diamond && !self.plain_image).then_some(self.footprint);
        let path=self.selection.path(diamond);
        self.selection.proportional = false;
        if path.len()<3 {self.selection.draft.clear();return;}
        let bounds=Rect::from_points(&path);
        let width=bounds.width().ceil() as usize;let height=bounds.height().ceil() as usize;
        if width.saturating_mul(height)>4*1024*1024 {self.message=tr("选取区域过大，请缩小范围。").into();self.selection.draft.clear();return;}
        let mut world=World::new();
        for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            if contains(&path,Pos2::new(x as f32+0.5,y as f32+0.5)) && self.selection_hit(Pos2::new(x as f32,y as f32),layout,infinite).is_some() {world.insert((x,y));}
        }}
        self.finish_stroke();
        self.selection.cut_layer=None;self.selection.floating=None;
        match self.selection.combine {
            Combine::Replace=>self.selection.world=world,
            Combine::Add=>{
                if self.selection.world.len().saturating_add(world.len())>4*1024*1024 {self.message=tr("选取区域过大，请缩小范围。").into();self.selection.draft.clear();return;}
                self.selection.world.extend(world);
            },
            Combine::Subtract=>{
                let remove:std::collections::HashSet<_>=world.iter().filter_map(|&(x,y)|self.selection_hit(Pos2::new(x as f32,y as f32),layout,infinite)).collect();
                self.selection.world=self.selection.world.iter().copied().filter(|&(x,y)|self.selection_hit(Pos2::new(x as f32,y as f32),layout,infinite).is_none_or(|p|!remove.contains(&p))).collect();
            },
        }
        self.rebuild_selection(layout,infinite);self.selection.draft.clear();
    }
    pub(super) fn selection_outline(&self,painter:&egui::Painter,origin:Pos2,scale:f32) {
        if !self.selection.contours.is_empty(){painter.ctx().request_repaint_after(std::time::Duration::from_millis(50));}
        for contour in &self.selection.contours {let points:Vec<_>=contour.iter().map(|p|origin+p.to_vec2()*scale).collect();ants(painter,&points);}
        let dpi=painter.ctx().pixels_per_point();
        let pointer=painter.ctx().pointer_hover_pos().filter(|p|painter.clip_rect().contains(*p));
        let diamond = (self.diamond && !self.plain_image).then_some(self.footprint);
        let(points,marker)=self.selection.draft_preview(origin,scale,pointer,dpi,diamond);
        if let Some((start,near))=marker{painter.circle_stroke(start,4./dpi,egui::Stroke::new(3./dpi,Color32::BLACK));painter.circle_stroke(start,4./dpi,egui::Stroke::new(1./dpi,Color32::WHITE));if near{painter.circle_filled(start,2./dpi,painter.ctx().style().visuals.selection.stroke.color);}}
        if points.len()<2{return;}
        for edge in points.windows(2){let length=(edge[1]-edge[0]).length();let count=(length*dpi/4.).ceil().max(1.) as usize;
            for step in 0..count{let a=edge[0]+(edge[1]-edge[0])*step as f32/count as f32;let b=edge[0]+(edge[1]-edge[0])*(step+1) as f32/count as f32;let middle=(a+(b-a)*0.5-origin)/scale;
                let layout=self.whole_unit()||self.rendering_merged;
                let hit=if self.infinite_view&&self.rendering_merged{self.plane_hit(middle.x.floor() as i32,middle.y.floor() as i32,layout)}else{self.finite_plane_hit(middle.x.floor() as i32,middle.y.floor() as i32,layout)};
                let pixel=hit.and_then(|(tile,x,y)|self.preview_samples.get(tile).filter(|image|x<image.width()&&y<image.height()).map(|image|*image.get_pixel(x,y)));
                let background=if self.background_custom{self.background_color}else{[210;3]};
                let rgb=if let Some(p)=pixel{let alpha=p[3] as f32/255.;std::array::from_fn::<f32,3,_>(|c|p[c] as f32*alpha+background[c] as f32*(1.-alpha))}else{background.map(|v|v as f32)};
                let color=if rgb[0]*0.299+rgb[1]*0.587+rgb[2]*0.114>128.{Color32::BLACK}else{Color32::WHITE};
                painter.line_segment([a,b],egui::Stroke::new(1./dpi,color));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn editor()->Editor {let mut e=Editor::default();e.diamond=false;e.wrap=false;e.reset_document(RgbaImage::new(8,4));e}
    fn select(e:&mut Editor,a:Pos2,b:Pos2,combine:Combine){e.selection.mode=Mode::Rectangle;e.selection.combine=combine;e.selection.draft=vec![a,b];e.commit_selection(false,false);}
    #[test]
    fn polygon_first_point_previews_open_segment_and_start_marker(){let mut selection=Selection::default();selection.mode=Mode::Polygon;selection.draft=vec![Pos2::new(1.,2.)];let(pointer,marker)=selection.draft_preview(Pos2::ZERO,1.,Some(Pos2::new(30.,40.)),1.,None);assert_eq!(pointer,vec![Pos2::new(1.,2.),Pos2::new(30.,40.)]);assert_eq!(marker,Some((Pos2::new(1.,2.),false)));selection.draft.extend([Pos2::new(10.,2.),Pos2::new(10.,10.)]);let(points,_)=selection.draft_preview(Pos2::ZERO,1.,Some(Pos2::new(30.,40.)),1.,None);assert_eq!(points.len(),4);assert_ne!(points.first(),points.last());assert!(selection.polygon_close_at(Pos2::new(4.9,2.),Pos2::ZERO,1.,2.));assert!(!selection.polygon_close_at(Pos2::new(5.1,2.),Pos2::ZERO,1.,2.));}
    #[test]
    fn selection_add_subtract_and_copy_clips_pixels() {
        let mut e=editor();
        select(&mut e,Pos2::new(-4.,-2.),Pos2::new(-2.,0.),Combine::Replace);
        assert_eq!(e.selection.mask.as_ref().unwrap().len(),4);
        select(&mut e,Pos2::new(-2.,-2.),Pos2::new(0.,0.),Combine::Add);
        assert_eq!(e.selection.mask.as_ref().unwrap().len(),8);
        select(&mut e,Pos2::new(-4.,-2.),Pos2::new(-3.,0.),Combine::Subtract);
        assert_eq!(e.selection.mask.as_ref().unwrap().len(),6);
        e.write_pixel(0,1,0,Rgba([200,100,50,255]));
        e.write_pixel(0,0,0,Rgba([255;4]));
        assert_eq!(e.pixels.get_pixel(0,0)[3],0);
        let current=e.active_layer;e.duplicate_layer(current,true);
        assert_eq!(e.pixels.get_pixel(1,0).0,[200,100,50,255]);
        assert_eq!(e.pixels.get_pixel(0,0)[3],0);
        assert!(e.selection.mask.is_some());
    }
    #[test]
    fn cut_move_changes_only_active_layer_and_undo_restores_overlap() {
        let mut e=editor();
        e.write_pixel(0,0,0,Rgba([200,100,50,255]));
        e.write_pixel(0,1,0,Rgba([20,30,40,255]));
        let original=e.pixels.clone();
        select(&mut e,Pos2::new(-4.,-2.),Pos2::new(-2.,-1.),Combine::Replace);
        let source=e.selection.world.clone();
        e.selection.world=source.iter().map(|&(x,y)|(x+1,y)).collect();e.rebuild_selection(false,false);
        e.cut_move_selection(&source,egui::vec2(1.,0.),false,false);
        assert_eq!(e.pixels.get_pixel(0,0)[3],0);
        assert_eq!(e.pixels.get_pixel(1,0).0,[200,100,50,255]);
        assert_eq!(e.pixels.get_pixel(2,0).0,[20,30,40,255]);
        assert!(e.layers[0].tiles[0].pixels().all(|p|p[3]==0));
        e.history(false);assert_eq!(e.pixels,original);
    }
    #[test]
    fn repeated_floating_moves_restore_background_and_keep_only_cut_payload(){
        let mut e=editor();e.write_pixel(0,0,0,Rgba([200,100,50,128]));e.write_pixel(0,2,0,Rgba([20,40,80,255]));
        select(&mut e,Pos2::new(-4.,-2.),Pos2::new(-3.,-1.),Combine::Replace);
        let source=e.selection.world.clone();e.cut_move_selection(&source,egui::vec2(2.,0.),false,false);
        assert_eq!(e.selection.cut_layer,Some(e.active_layer));
        e.before=Some(e.snapshot());e.apply_floating((3,0),false,false);e.finish_stroke();
        assert_eq!(e.pixels.get_pixel(2,0).0,[20,40,80,255]);
        assert_eq!(e.pixels.get_pixel(3,0).0,[200,100,50,128]);
        assert_eq!(e.pixels.get_pixel(0,0)[3],0);
        e.selection.clear();assert!(e.selection.floating.is_none()&&e.selection.cut_layer.is_none());
    }
    #[test]
    fn mode_cycles_and_polygon_membership(){assert!(Mode::Polygon.next()==Mode::Rectangle);let p=[Pos2::new(0.,0.),Pos2::new(4.,0.),Pos2::new(4.,4.),Pos2::new(0.,4.)];assert!(contains(&p,Pos2::new(2.,2.)));assert!(!contains(&p,Pos2::new(5.,2.)));}
}

impl Editor{
 pub(super) fn selection_from_mask_pixels(&mut self,pixels:std::collections::HashSet<(usize,u32,u32)>){
    self.selection.clear();let(w,h)=self.pixels.dimensions();let layout=self.whole_unit();let(cols,rows)=if layout{(self.columns,self.rows)}else{(1,1)};let mut mask=std::collections::HashSet::new();
    for row in 0..rows{for col in 0..cols{let tile=if layout{self.tile_at(col as i32,row as i32)}else{self.selected};let center=self.mapped_center(col as i32,row as i32,w,h);for &(t,x,y)in &pixels{if t==tile{let world=center+egui::vec2(x as f32-w as f32/2.,y as f32-h as f32/2.);self.selection.world.insert((world.x.floor()as i32,world.y.floor()as i32));mask.insert((t,x,y));}}}}
    self.selection.mask=Some(mask);self.selection.rebuild_edges();
 }
}

impl Editor {
    pub(super) fn select_entire_canvas(&mut self) {
        let (w, h) = self.pixels.dimensions();
        let layout = self.whole_unit();
        let (cols, rows) = if layout { (self.columns, self.rows) } else { (1, 1) };
        if (w as usize).saturating_mul(h as usize).saturating_mul(cols).saturating_mul(rows) > 4 * 1024 * 1024 {
            self.message = tr("选取区域过大，请缩小范围。").into();
            return;
        }
        self.finish_stroke();
        self.selection.clear();
        for row in 0..rows {
            for col in 0..cols {
                let offset = self.mapped_center(col as i32, row as i32, w, h)
                    - egui::vec2(w as f32, h as f32) * 0.5;
                for y in 0..h {
                    for x in 0..w {
                        if self.inside_tile(x as i32, y as i32, w, h) {
                            self.selection.world.insert((offset.x.floor() as i32 + x as i32, offset.y.floor() as i32 + y as i32));
                        }
                    }
                }
            }
        }
        self.rebuild_selection(layout, false);
    }

    pub(super) fn delete_selection_pixels(&mut self) {
        let revision = self.revision;
        self.fill_selection_pixels([0; 4], false);
        if self.revision != revision {
            self.message = tr("已清空选区。").into();
        }
    }

    pub(super) fn fill_selection_color(&mut self) {
        self.fill_selection_with_color(self.color);
    }

    pub(super) fn fill_selection_with_color(&mut self, ink: [u8; 4]) {
        self.fill_selection_pixels(ink, self.soft_brush);
    }

    fn fill_selection_pixels(&mut self, ink: [u8; 4], soften_edges: bool) {
        if !self.layer_editable() || !self.selection.draft.is_empty() {
            return;
        }
        let selection = self.selection.mask.clone();
        self.finish_stroke();
        self.before = Some(self.snapshot());
        let revision = self.revision;
        let (w, h) = self.pixels.dimensions();
        let tile_count = self.tiles.len();
        let mut fill = |tile, x, y| {
            if !self.inside_tile(x as i32, y as i32, w, h) {
                return;
            }
            let mut color = Rgba(ink);
            if soften_edges && let Some(mask) = &selection {
                // A one-pixel weighted edge transition derived from the final selection mask.
                let mut weight = 0;
                for (dy, wy) in [(-1, 1), (0, 2), (1, 1)] {
                    for (dx, wx) in [(-1, 1), (0, 2), (1, 1)] {
                        let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                        let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        if mask.contains(&(tile, nx, ny)) {
                            weight += wx * wy;
                        }
                    }
                }
                if weight < 16 {
                    color = layer_groups::mix_group(self.before_edit_pixel(tile, x, y), color, weight as f32 / 16.0);
                }
            }
            self.write_pixel(tile, x, y, color);
        };
        if let Some(mask) = &selection {
            for &(tile, x, y) in mask {
                if tile < tile_count {
                    fill(tile, x, y);
                }
            }
        } else {
            for tile in 0..tile_count {
                for y in 0..h {
                    for x in 0..w {
                        fill(tile, x, y);
                    }
                }
            }
        }
        if self.revision != revision {
            self.selection.floating = None;
            self.selection.cut_layer = None;
            self.message = tr(if selection.is_some() { "已填充选区。" } else { "已填充图层。" }).into();
        }
        self.finish_stroke();
    }
}
#[cfg(test)]
#[test]
fn brush_fill_softens_selected_edges_without_affecting_the_outside() {
    let mut editor = Editor::default();
    editor.init_image(&RgbaImage::new(5, 5));
    editor.selection.mask = Some((1..4).flat_map(|y| (1..4).map(move |x| (0, x, y))).collect());
    editor.color = [40, 80, 120, 255];
    editor.soft_brush = true;
    editor.fill_selection_color();
    assert_eq!(editor.pixels.get_pixel(2, 2)[3], 255);
    assert!((1..255).contains(&editor.pixels.get_pixel(1, 1)[3]));
    assert_eq!(editor.pixels.get_pixel(0, 0)[3], 0);
    editor.history(false);
    assert!(editor.pixels.pixels().all(|pixel| pixel[3] == 0));
}

#[cfg(test)]mod fill_tests{use super::*;#[test]fn fill_preserves_selection_opacity_and_undo(){let mut e=Editor::default();e.diamond=false;e.reset_document(RgbaImage::new(4,4));let mask:std::collections::HashSet<_>=[(0,1,1),(0,2,1)].into_iter().collect();e.selection.mask=Some(mask.clone());e.color=[80,120,160,128];e.fill_selection_color();assert_eq!(e.pixels.get_pixel(1,1).0,[80,120,160,128]);assert_eq!(e.pixels.get_pixel(0,0)[3],0);assert_eq!(e.selection.mask.as_ref(),Some(&mask));e.history(false);assert!(e.pixels.pixels().all(|p|p[3]==0));assert_eq!(e.selection.mask.as_ref(),Some(&mask));e.layers[e.active_layer].settings.lock_pixels=true;e.fill_selection_color();assert!(e.pixels.pixels().all(|p|p[3]==0));}}

#[cfg(test)]
#[test]
fn fill_without_selection_colors_the_entire_layer_and_can_undo() {
    let mut editor = Editor::default();
    editor.init_image(&RgbaImage::new(3, 2));
    editor.color = [25, 80, 120, 160];
    editor.fill_selection_color();
    assert!(editor.pixels.pixels().all(|pixel| pixel.0 == editor.color));
    editor.history(false);
    assert!(editor.pixels.pixels().all(|pixel| pixel[3] == 0));
}
