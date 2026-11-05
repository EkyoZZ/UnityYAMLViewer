use eframe::egui;
use crate::localization::tr;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Crop {
    pub enabled: bool,
    source: [usize; 2],
    rect: [i64; 4], // x, y, width, height in decoded source pixels
    #[serde(skip)]
    drag: Option<CropDrag>,
    percent: [f64;2],
    linked: bool,
    pixel_mode: bool,
    scale_enabled: bool,
}

#[cfg(test)]
mod canvas_tests {
    use super::*;
    #[test]
    fn canvas_can_expand_and_reset_without_scaling_source() {
        let mut canvas=Crop::default();canvas.prepare([1920,1080]);canvas.enabled=true;
        canvas.rect=[-120,-60,2160,1200];canvas.validate_project().unwrap();
        let filter=canvas.filter_with_scaler("lanczos",[255,0,128]);
        assert!(filter.contains("crop=2160:1200:0:0:exact=1"));
        assert!(filter.contains("color=0xFF0080"));assert!(!filter.contains("scale="));
        let restored:Crop=serde_json::from_slice(&serde_json::to_vec(&canvas).unwrap()).unwrap();
        assert_eq!(restored.rect,canvas.rect);
        canvas.reset();assert_eq!(canvas.rect,[0,0,1920,1080]);
    }
    #[test]
    fn edges_and_corners_can_extend_beyond_original_source() {
        assert_eq!(drag_axis(0,100,100,-20.0,-1,false),(-20,120));
        assert_eq!(drag_axis(0,100,100,20.0,1,false),(0,120));
        let drag=CropDrag{rect:[0,0,100,50],pointer:egui::Pos2::ZERO,edges:[1,1]};
        assert_eq!(resize_corner(&drag,[100,50],egui::vec2(20.0,10.0)),[0,0,120,60]);
    }
}
#[derive(Clone)]
struct CropDrag {
    rect: [i64; 4],
    pointer: egui::Pos2,
    edges: [i8; 2],
}

const CANVAS_LIMIT:i64=8192;
fn drag_axis(start:i64,size:i64,_limit:usize,delta:f32,edge:i8,moving:bool)->(i64,i64) {
    let delta=delta.round() as i64;
    match edge {
        -1=>{let next=(start+delta).clamp((start+size-CANVAS_LIMIT).max(-CANVAS_LIMIT),start+size-1);(next,start+size-next)},
        1=>(start,(size+delta).clamp(1,CANVAS_LIMIT)),
        _ if moving=>((start+delta).clamp(-CANVAS_LIMIT,CANVAS_LIMIT),size),
        _=>(start,size),
    }
}
fn resize_corner(drag:&CropDrag,_source:[usize;2],delta:egui::Vec2)->[i64;4] {
    let [x,y,w,h]=drag.rect;
    let [horizontal,vertical]=drag.edges;
    let diagonal=egui::vec2(w as f32*f32::from(horizontal),h as f32*f32::from(vertical));
    let scale=(1.0+delta.dot(diagonal)/diagonal.length_sq()).clamp((1.0/w as f32).max(1.0/h as f32),(CANVAS_LIMIT as f32/w as f32).min(CANVAS_LIMIT as f32/h as f32));
    let width=(w as f32*scale).round().max(1.0) as i64;
    let height=(h as f32*scale).round().max(1.0) as i64;
    [if horizontal<0{x+w-width}else{x},if vertical<0{y+h-height}else{y},width,height]
}

impl Default for Crop {fn default()->Self{Self{enabled:false,source:[0;2],rect:[0;4],drag:None,percent:[100.0;2],linked:true,pixel_mode:false,scale_enabled:false}}}

impl Crop {
    pub(crate) fn validate_project(&self) -> Result<(), String> {
        if self.percent.iter().any(|n| !n.is_finite() || *n <= 0.0 || *n > 10000.0)
            || self.source.iter().any(|n| *n > 65536)
            || self.enabled && (self.rect[2]<1 || self.rect[3]<1
                || self.rect[2]>CANVAS_LIMIT || self.rect[3]>CANVAS_LIMIT
                || self.rect[0].unsigned_abs()>CANVAS_LIMIT as u64 || self.rect[1].unsigned_abs()>CANVAS_LIMIT as u64)
        { return Err(crate::localization::tr("工程设置无效或格式版本不受支持。").into()); }
        Ok(())
    }
    pub fn preview_rects(&self,area:egui::Rect,fallback:egui::Vec2)->(egui::Rect,egui::Rect) {
        let source=if self.source.contains(&0){fallback}else{egui::vec2(self.source[0] as f32,self.source[1] as f32)};
        let canvas=if self.enabled {egui::Rect::from_min_size(egui::pos2(self.rect[0] as f32,self.rect[1] as f32),egui::vec2(self.rect[2] as f32,self.rect[3] as f32))}
            else{egui::Rect::from_min_size(egui::Pos2::ZERO,source)};
        let bounds=canvas.union(egui::Rect::from_min_size(egui::Pos2::ZERO,source));
        let stretch=if self.scale_enabled{egui::vec2(self.percent[0] as f32/100.0,self.percent[1] as f32/100.0)}else{egui::Vec2::splat(1.0)};
        let size=bounds.size()*stretch;
        let zoom=(area.width()/size.x).min(area.height()/size.y)*if self.enabled{0.92}else{1.0};
        let origin=area.center()-size*zoom*0.5-bounds.min.to_vec2()*stretch*zoom;
        let map=|rect:egui::Rect|egui::Rect::from_min_max(origin+rect.min.to_vec2()*stretch*zoom,origin+rect.max.to_vec2()*stretch*zoom);
        (map(egui::Rect::from_min_size(egui::Pos2::ZERO,source)),map(canvas))
    }
    pub fn minimum_width(ui:&egui::Ui)->f32{
        let font=egui::TextStyle::Button.resolve(ui.style());
        let measure=|label:&str|ui.painter().layout_no_wrap(tr(label).to_owned(),font.clone(),ui.visuals().text_color()).size().x.ceil();
        let tabs=measure("百分比")+measure("像素")+24.0;
        let link_width=ui.spacing().icon_width+ui.spacing().icon_spacing;
        let scroll=&ui.spacing().scroll;
        let scrollbar=scroll.bar_width+scroll.bar_inner_margin+scroll.bar_outer_margin;
        // Match the actual row below, plus the panel's left padding and scrollbar.
        link_width+measure("宽度").max(measure("高度"))+88.0+tabs
            +ui.spacing().item_spacing.x*2.0+8.0+scrollbar+2.0
    }
    fn scale_input(&mut self,ui:&mut egui::Ui,axis:usize,size:usize,width:f32){
        let changed=if self.pixel_mode {
            let mut value=(size as f64*self.percent[axis]/100.0).round().max(1.0) as usize;
            let minimum=(size as f64*0.01).round().max(1.0) as usize;
            let changed=ui.add_sized([width,24.0],egui::DragValue::new(&mut value).range(minimum..=(size*4).max(1)).suffix(" px")).changed();
            if changed{self.percent[axis]=(value as f64/size.max(1) as f64*100.0).clamp(1.0,400.0);}
            changed
        }else{ui.add_sized([width,24.0],egui::DragValue::new(&mut self.percent[axis]).range(1.0..=400.0).speed(1.0).max_decimals(1).suffix(" %")).changed()};
        if changed && self.linked {self.percent[1-axis]=self.percent[axis];}
    }
    pub fn prepare(&mut self, source: [usize; 2]) {
        if self.source != source { self.source=source; self.reset(); }
    }
    fn reset(&mut self) {self.rect=[0,0,self.source[0] as i64,self.source[1] as i64];self.drag=None;}
    pub fn controls(&mut self,ui:&mut egui::Ui) {
        ui.separator();
        ui.checkbox(&mut self.enabled,tr("画布大小"));
        ui.add_enabled_ui(self.enabled && self.source[0]>0 && self.source[1]>0,|ui|{
            ui.horizontal_wrapped(|ui|{
                if ui.button(tr("重置范围")).clicked(){self.reset();}
                for (label,rw,rh) in [("16:9",16,9),("9:16",9,16),("1:1",1,1)] {
                    if ui.button(label).clicked(){
                        let [w,h]=self.source;
                        let width=w.min(h*rw/rh).max(1);let height=(width*rh/rw).max(1).min(h);
                        self.rect=[((w-width)/2) as i64,((h-height)/2) as i64,width as i64,height as i64];
                        self.drag = None;
                    }
                }
            });
            egui::Grid::new("crop_pixels").num_columns(4).show(ui,|ui|{
                for (index,label) in ["X","Y","W","H"].into_iter().enumerate(){
                    ui.label(label);
                    let range=if index<2 {-CANVAS_LIMIT..=CANVAS_LIMIT}else{1..=CANVAS_LIMIT};
                    if ui.add(egui::DragValue::new(&mut self.rect[index]).range(range).suffix(" px")).changed() {self.drag=None;}
                    if index%2==1 {ui.end_row();}
                }
            });
            ui.small(tr("拖动调整画布位置和大小；向外扩展使用填充色，重置恢复原尺寸。"));
        });
        let [w,h]=if self.enabled{[self.rect[2] as usize,self.rect[3] as usize]}else{self.source};
        ui.separator();
        ui.checkbox(&mut self.scale_enabled,tr("画面大小"));
        ui.add_enabled_ui(self.scale_enabled && w>0 && h>0,|ui|{
            let font=egui::TextStyle::Button.resolve(ui.style());
            let label_width=["宽度","高度"].iter().map(|label|ui.painter().layout_no_wrap(tr(label).to_owned(),font.clone(),ui.visuals().text_color()).size().x).fold(0.0_f32,f32::max);
            let spacing=ui.spacing().item_spacing.x;
            let input_width=88.0;
            let link_width=ui.spacing().icon_width+ui.spacing().icon_spacing;
            ui.horizontal(|ui|{
                ui.spacing_mut().item_spacing.x=0.0;
                let button=ui.add_sized([link_width,52.0],egui::Button::new("").frame(false));
                if button.clicked(){self.linked=!self.linked;if self.linked{self.percent[1]=self.percent[0];}}
                let tint=if self.linked{crate::theme::BLUE}else{egui::Color32::from_gray(128)};
                egui::Image::new(egui::include_image!("../assets/icons/size-link.svg")).tint(tint).paint_at(ui,button.rect);
                button.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Checkbox,button.enabled(),tr("锁定宽高比例")));
                button.on_hover_text(tr("锁定宽高比例；重新锁定时以宽度为准恢复比例。"));
                ui.allocate_ui_with_layout(egui::vec2(label_width+spacing+input_width,52.0),egui::Layout::top_down(egui::Align::Min),|ui|{
                    ui.spacing_mut().item_spacing.x=spacing;
                    egui::Grid::new("picture_size_inputs_fixed").num_columns(2).min_col_width(0.0).max_col_width(input_width).show(ui,|ui|{
                        ui.label(tr("宽度"));self.scale_input(ui,0,w,input_width);ui.end_row();
                        ui.label(tr("高度"));self.scale_input(ui,1,h,input_width);ui.end_row();
                    });
                });
                ui.add_space(spacing);
                let mut mode=if self.pixel_mode{"pixels"}else{"percent"}.to_owned();
                crate::theme::compact_selector(ui,&mut mode,[("percent","百分比"),("pixels","像素")]);
                self.pixel_mode=mode=="pixels";
            });
        });
        let percent=if self.scale_enabled{self.percent}else{[100.0;2]};
        let output=|size:usize,axis:usize|((size as f64*percent[axis]/100.0).round().max(1.0) as usize).div_ceil(2)*2;
        ui.small(format!("{}: {} × {} px",tr("输出尺寸"),output(w,0),output(h,1)));
    }
    pub fn overlay(&mut self,ui:&mut egui::Ui,image:egui::Rect,editable:bool) {
        if !self.enabled || self.source.contains(&0) || !image.is_positive() {
            self.drag = None;
            return;
        }
        let scale = image.size() / egui::vec2(self.source[0] as f32, self.source[1] as f32);
        let bounds = |[x, y, w, h]: [i64; 4]| {
            egui::Rect::from_min_size(image.min + egui::vec2(x as f32, y as f32) * scale,
                egui::vec2(w as f32, h as f32) * scale)
        };
        let crop = bounds(self.rect);
        if editable {
            let response = ui.interact(crop.expand(7.0).intersect(ui.clip_rect()), ui.id().with("crop_drag"), egui::Sense::drag());
            let hit_edges = |pointer: egui::Pos2| {
                let edge = |value: f32, low: f32, high: f32| {
                    let left = (value - low).abs();
                    let right = (value - high).abs();
                    if left.min(right) > 7.0 { 0 } else if left <= right { -1 } else { 1 }
                };
                [edge(pointer.x, crop.left(), crop.right()), edge(pointer.y, crop.top(), crop.bottom())]
            };
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pointer) = ui.input(|input| input.pointer.press_origin()).or_else(|| response.interact_pointer_pos()) {
                    self.drag = Some(CropDrag { rect: self.rect, pointer, edges: hit_edges(pointer) });
                }
            }
            if let Some((drag, pointer)) = self.drag.as_ref().zip(response.interact_pointer_pos()) {
                if response.dragged_by(egui::PointerButton::Primary) || response.drag_stopped_by(egui::PointerButton::Primary) {
                    let delta = (pointer - drag.pointer) / scale;
                    let [x, y, w, h] = drag.rect;
                    let mut next = if !drag.edges.contains(&0) {
                        resize_corner(drag, self.source, delta)
                    } else {
                        let moving = drag.edges == [0, 0];
                        let (x, w) = drag_axis(x, w, self.source[0], delta.x, drag.edges[0], moving);
                        let (y, h) = drag_axis(y, h, self.source[1], delta.y, drag.edges[1], moving);
                        [x, y, w, h]
                    };
                    next[0]=next[0].clamp(-CANVAS_LIMIT,CANVAS_LIMIT);
                    next[1]=next[1].clamp(-CANVAS_LIMIT,CANVAS_LIMIT);
                    if next != self.rect {
                        self.rect = next;
                        ui.ctx().request_repaint();
                    }
                }
            }
            if response.hovered() || response.dragged() {
                if let Some(edges) = self.drag.as_ref().map(|drag| drag.edges)
                    .or_else(|| ui.input(|input| input.pointer.hover_pos()).map(hit_edges)) {
                    ui.ctx().set_cursor_icon(match edges {
                        [0, 0] => egui::CursorIcon::Move,
                        [0, _] => egui::CursorIcon::ResizeVertical,
                        [_, 0] => egui::CursorIcon::ResizeHorizontal,
                        [-1, -1] | [1, 1] => egui::CursorIcon::ResizeNwSe,
                        _ => egui::CursorIcon::ResizeNeSw,
                    });
                }
            }
            if !ui.input(|input| input.pointer.primary_down()) { self.drag = None; }
        } else {
            self.drag = None;
        }
        let crop = bounds(self.rect);
        let painter=ui.painter_at(ui.clip_rect());
        for mask in [
            egui::Rect::from_min_max(image.min,egui::pos2(image.right(),crop.top())),
            egui::Rect::from_min_max(egui::pos2(image.left(),crop.bottom()),image.max),
            egui::Rect::from_min_max(egui::pos2(image.left(),crop.top()),crop.left_bottom()),
            egui::Rect::from_min_max(crop.right_top(),egui::pos2(image.right(),crop.bottom())),
        ] {if mask.is_positive(){painter.rect_filled(mask,0.0,egui::Color32::from_black_alpha(150));}}
        painter.rect_stroke(crop,0.0,egui::Stroke::new(2.0_f32,egui::Color32::LIGHT_GREEN),egui::StrokeKind::Inside);
        let size = egui::vec2(7.0, 7.0).min(image.size());
        for corner in [crop.left_top(), crop.right_top(), crop.left_bottom(), crop.right_bottom()] {
            let min = corner - size * 0.5;
            painter.rect_filled(egui::Rect::from_min_size(min, size), 1.0, egui::Color32::LIGHT_GREEN);
        }
    }
    pub(crate) fn without_scaling(&self) -> Self {
        let mut crop = self.clone();
        crop.scale_enabled = false;
        crop.percent = [100.0; 2];
        crop
    }
    pub fn filter_with_scaler(&self,scaler:&str,fill:[u8;3])->String {
        let pad="pad=ceil(iw/2)*2:ceil(ih/2)*2:color=black@0";
        let mut filters=Vec::new();
        if self.enabled && !self.source.contains(&0) {
            let [x,y,w,h]=self.rect;
            let left=(-x).max(0);let top=(-y).max(0);
            let [r,g,b]=fill;
            filters.push(format!("pad=w='max(iw+{left},{w}+{x}+{left})':h='max(ih+{top},{h}+{y}+{top})':x={left}:y={top}:color=0x{r:02X}{g:02X}{b:02X},crop={w}:{h}:{}:{}:exact=1",x+left,y+top));
        }
        if self.scale_enabled && self.percent!=[100.0;2] {
            let [x,y]=self.percent.map(|value|value/100.0);
            filters.push(format!("scale=w='max(1,round(iw*{x}))':h='max(1,round(ih*{y}))':flags={scaler},setsar=1"));
        }
        filters.push(pad.to_owned());
        filters.join(",")
    }
}
