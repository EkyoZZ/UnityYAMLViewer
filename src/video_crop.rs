use eframe::egui;
use crate::localization::tr;

#[derive(Clone)]
pub struct Crop {
    pub enabled: bool,
    source: [usize; 2],
    rect: [usize; 4], // x, y, width, height in decoded source pixels
    drag: Option<([usize; 4], bool)>,
    percent: [f64;2],
    linked: bool,
    pixel_mode: bool,
    scale_enabled: bool,
}
impl Default for Crop {fn default()->Self{Self{enabled:false,source:[0;2],rect:[0;4],drag:None,percent:[100.0;2],linked:true,pixel_mode:false,scale_enabled:false}}}

impl Crop {
    pub fn preview_size(&self,fallback:egui::Vec2)->egui::Vec2{
        let source=if self.source.contains(&0){fallback}else{egui::vec2(self.source[0] as f32,self.source[1] as f32)};
        if self.scale_enabled{source*egui::vec2(self.percent[0] as f32/100.0,self.percent[1] as f32/100.0)}else{source}
    }
    pub fn minimum_width(ui:&egui::Ui)->f32{
        let font=egui::TextStyle::Button.resolve(ui.style());
        let measure=|label:&str|ui.painter().layout_no_wrap(tr(label).to_owned(),font.clone(),ui.visuals().text_color()).size().x.ceil();
        let tabs=measure("百分比")+measure("像素")+24.0;
        28.0+measure("宽度").max(measure("高度"))+88.0+tabs+ui.spacing().item_spacing.x*3.0+44.0
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
    fn reset(&mut self) {self.rect=[0,0,self.source[0],self.source[1]];self.drag=None;}
    fn clamp(&mut self) {
        let [w,h]=self.source;
        self.rect[2]=self.rect[2].clamp(1,w.max(1));
        self.rect[3]=self.rect[3].clamp(1,h.max(1));
        self.rect[0]=self.rect[0].min(w.saturating_sub(self.rect[2]));
        self.rect[1]=self.rect[1].min(h.saturating_sub(self.rect[3]));
    }
    pub fn controls(&mut self,ui:&mut egui::Ui) {
        ui.separator();
        ui.checkbox(&mut self.enabled,tr("画面裁剪"));
        ui.add_enabled_ui(self.enabled && self.source[0]>0 && self.source[1]>0,|ui|{
            ui.horizontal_wrapped(|ui|{
                if ui.button(tr("重置范围")).clicked(){self.reset();}
                for (label,rw,rh) in [("16:9",16,9),("9:16",9,16),("1:1",1,1)] {
                    if ui.button(label).clicked(){
                        let [w,h]=self.source;
                        let width=w.min(h*rw/rh).max(1);let height=(width*rh/rw).max(1).min(h);
                        self.rect=[(w-width)/2,(h-height)/2,width,height];
                    }
                }
            });
            egui::Grid::new("crop_pixels").num_columns(4).show(ui,|ui|{
                for (index,label) in ["X","Y","W","H"].into_iter().enumerate(){
                    ui.label(label);
                    let max=self.source[index%2].max(1);
                    ui.add(egui::DragValue::new(&mut self.rect[index]).range(if index<2{0}else{1}..=max).suffix(" px"));
                    if index%2==1 {ui.end_row();}
                }
            });
            self.clamp();
            ui.small(tr("拖动框内移动，拖动右下角调整大小；比例按钮设置初始范围。"));
        });
        let [w,h]=if self.enabled{[self.rect[2],self.rect[3]]}else{self.source};
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
                ui.add_space(12.0);
            });
        });
        let percent=if self.scale_enabled{self.percent}else{[100.0;2]};
        let output=|size:usize,axis:usize|((size as f64*percent[axis]/100.0).round().max(1.0) as usize).div_ceil(2)*2;
        ui.small(format!("{}: {} × {} px",tr("输出尺寸"),output(w,0),output(h,1)));
    }
    pub fn overlay(&mut self,ui:&mut egui::Ui,image:egui::Rect,editable:bool) {
        if !self.enabled || self.source.contains(&0) {return;}
        let scale=image.size()/egui::vec2(self.source[0] as f32,self.source[1] as f32);
        let [x,y,w,h]=self.rect;
        let crop=egui::Rect::from_min_size(image.min+egui::vec2(x as f32,y as f32)*scale,egui::vec2(w as f32,h as f32)*scale);
        let painter=ui.painter_at(image);
        for mask in [
            egui::Rect::from_min_max(image.min,egui::pos2(image.right(),crop.top())),
            egui::Rect::from_min_max(egui::pos2(image.left(),crop.bottom()),image.max),
            egui::Rect::from_min_max(egui::pos2(image.left(),crop.top()),crop.left_bottom()),
            egui::Rect::from_min_max(crop.right_top(),egui::pos2(image.right(),crop.bottom())),
        ] {painter.rect_filled(mask,0.0,egui::Color32::from_black_alpha(150));}
        painter.rect_stroke(crop,0.0,egui::Stroke::new(2.0,egui::Color32::LIGHT_GREEN),egui::StrokeKind::Inside);
        let handle=egui::Rect::from_center_size(crop.max,egui::vec2(14.0,14.0)).intersect(image);
        painter.rect_filled(handle,2.0,egui::Color32::LIGHT_GREEN);
        if !editable{return;}
        let response=ui.interact(image,ui.id().with("crop_drag"),egui::Sense::click_and_drag());
        if response.drag_started(){
            if let Some(pointer)=response.interact_pointer_pos(){
                let origin=pointer-response.drag_delta();
                if crop.contains(origin) || handle.contains(origin){self.drag=Some((self.rect,handle.contains(origin)));}
            }
        }
        if response.dragged(){if let Some((before,resize))=self.drag{
            let delta=response.drag_delta()/scale;
            self.rect=before;
            if resize {
                self.rect[2]=(before[2] as f32+delta.x).round().clamp(1.0,(self.source[0]-before[0]) as f32) as usize;
                self.rect[3]=(before[3] as f32+delta.y).round().clamp(1.0,(self.source[1]-before[1]) as f32) as usize;
            }else{
                self.rect[0]=(before[0] as f32+delta.x).round().clamp(0.0,(self.source[0]-before[2]) as f32) as usize;
                self.rect[1]=(before[1] as f32+delta.y).round().clamp(0.0,(self.source[1]-before[3]) as f32) as usize;
            }
        }}
        if response.drag_stopped(){self.drag=None;}
    }
    pub fn filter_with_scaler(&self,scaler:&str)->String {
        let pad="pad=ceil(iw/2)*2:ceil(ih/2)*2:color=black@0";
        let mut filters=Vec::new();
        if self.enabled && !self.source.contains(&0) {
            let [x,y,w,h]=self.rect;
            filters.push(format!("crop={w}:{h}:{x}:{y}:exact=1"));
        }
        if self.scale_enabled && self.percent!=[100.0;2] {
            let [x,y]=self.percent.map(|value|value/100.0);
            filters.push(format!("scale=w='max(1,round(iw*{x}))':h='max(1,round(ih*{y}))':flags={scaler},setsar=1"));
        }
        filters.push(pad.to_owned());
        filters.join(",")
    }
}
