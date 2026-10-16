use std::{path::Path, sync::{Arc,mpsc}};
use eframe::egui::{self,Color32};
use crate::localization::tr;
mod meta;
pub(crate) mod processing;
pub(crate) mod cli;
use processing::{Direction as Mode,Plan,Request,Treatment,Trial,Reason};

impl Mode {
    fn label(self) -> &'static str {match self {Self::Both=>"同时",Self::Horizontal=>"横向",Self::Vertical=>"纵向"}}
}
impl Treatment {
    fn label(self) -> &'static str {match self {Self::Stretch=>"拉伸",Self::Repeat=>"重复"}}
}
struct Preview {request:Request,plan:Plan,pixels:Arc<image::RgbaImage>,texture:egui::TextureHandle,mask_color:Color32,trials:Vec<Trial>}
struct Generated {submitted:Request,request:Request,plan:Plan,pixels:Arc<image::RgbaImage>,display:egui::ColorImage,mask_color:Color32,trials:Vec<Trial>}
fn replace_button(label:&str)->egui::Button<'_> {
    let green=Color32::from_rgb(30,150,85);
    egui::Button::new(egui::RichText::new(label).color(Color32::WHITE)).fill(green).stroke((1.,green)).corner_radius(4.).min_size(egui::vec2(144.,44.))
}
#[derive(Default)]
enum MaskColor {#[default] Auto,Red}
fn color_image(pixels:&image::RgbaImage) -> egui::ColorImage {
    egui::ColorImage {size:[pixels.width() as usize,pixels.height() as usize],pixels:pixels.pixels().map(|p|super::preview_color(p.0)).collect()}
}
#[derive(Default)]
pub(super) struct Editor {
    pub open:bool,
    request:Request,
    result:Option<Arc<Preview>>,
    pending:Option<(Request,mpsc::Receiver<Generated>)>,
    failed:Option<Request>,
    operation:Option<mpsc::Receiver<meta::Replacement>>,
    metadata:Option<meta::Meta>,
    original_borders:Option<[u32;4]>,
    show_original_border:bool,
    mask_color:MaskColor,
    manual:bool,
    autodetected:bool,
    show_details:bool,
    footer_height:f32,
    initialized:bool,
    confirm:bool,
    status:String,
}
// Both rows share the same fixed half-widths; overflowing content cannot resize a column.
fn equal_columns(ui:&mut egui::Ui,contents:impl FnOnce(&mut [egui::Ui;2])) {
    let origin=ui.cursor().min;
    let width=ui.available_width().max(0.);
    let gap=ui.spacing().item_spacing.x;
    let half=((width-gap)*0.5).max(0.);
    let id=ui.next_auto_id();
    let mut columns=std::array::from_fn(|index| {
        let min=origin+egui::vec2(index as f32*(half+gap),0.);
        let rect=egui::Rect::from_min_size(min,egui::vec2(half,ui.available_height()));
        let mut child=ui.new_child(egui::UiBuilder::new().id_salt(id.with(index)).max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
        child.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
        child.set_clip_rect(ui.clip_rect().intersect(egui::Rect::from_min_max(egui::pos2(min.x,ui.clip_rect().top()),egui::pos2(min.x+half,ui.clip_rect().bottom()))));
        child
    });
    contents(&mut columns);
    let height=columns.iter().map(|column|column.min_rect().height()).fold(0.,f32::max);
    ui.allocate_space(egui::vec2(width,height));
}
fn fitted(w: u32,h: u32,width: f32,height: f32) -> egui::Vec2 {
    egui::vec2(w as f32,h as f32)*(width.max(1.)/w as f32).min(height/h as f32).min(8.)
}

fn checker(ui: &egui::Ui,rect: egui::Rect) {
    let painter=ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    for y in 0..(rect.height()/12.).ceil() as i32 {for x in 0..(rect.width()/12.).ceil() as i32 {
        let tile=egui::Rect::from_min_size(rect.min+egui::vec2(x as f32*12.,y as f32*12.),egui::vec2(12.,12.)).intersect(rect);
        painter.rect_filled(tile,0.,Color32::from_gray(if (x+y)%2==0 {100}else{140}));
    }}
}

fn badge(painter: &egui::Painter,point: egui::Pos2,anchor: egui::Align2,text: String,color: Color32) {
    let galley=painter.layout_no_wrap(text,egui::FontId::proportional(12.),color);
    let rect=anchor.anchor_rect(egui::Rect::from_min_size(point,galley.size()+egui::vec2(8.,4.)));
    painter.rect_filled(rect,2.,Color32::from_black_alpha(210));
    painter.galley(rect.min+egui::vec2(4.,2.),galley,color);
}

/// Both previews carry distances in source pixels, never scaled screen coordinates.
fn guides(ui: &egui::Ui,rect: egui::Rect,w: u32,h: u32,v: [u32;6],mode: Mode) {
    let painter=ui.painter().with_clip_rect(ui.clip_rect());
    let [l,t,r,b,_,_]=v;
    if mode!=Mode::Vertical {
        for (value,pos,label,anchor,y) in [
            (l,l,"左边界",egui::Align2::LEFT_BOTTOM,rect.top()-4.),
            (r,w-r,"右边界",egui::Align2::RIGHT_TOP,rect.bottom()+4.),
        ] {
            let x=rect.left()+rect.width()*pos as f32/w as f32;
            painter.line_segment([egui::pos2(x,rect.top()),egui::pos2(x,rect.bottom())],(1.5,Color32::YELLOW));
            badge(&painter,egui::pos2(x,y),anchor,format!("{} {value} px",tr(label)),Color32::YELLOW);
        }
    }
    if mode!=Mode::Horizontal {
        for (value,pos,label,anchor,x) in [
            (t,t,"上边界",egui::Align2::LEFT_TOP,rect.left()),
            (b,h-b,"下边界",egui::Align2::RIGHT_BOTTOM,rect.right()),
        ] {
            let y=rect.top()+rect.height()*pos as f32/h as f32;
            painter.line_segment([egui::pos2(rect.left(),y),egui::pos2(rect.right(),y)],(1.5,Color32::LIGHT_BLUE));
            badge(&painter,egui::pos2(x,y),anchor,format!("{} {value} px",tr(label)),Color32::LIGHT_BLUE);
        }
    }
}

fn removed_regions(plan:&Plan)->Vec<[u32;4]> {
    let [w,h]=plan.original;let [x,y,cw,ch]=plan.crop;
    let [l,t,r,b,keep_w,keep_h]=plan.values;
    let (x0,x1,y0,y1)=(x+l+keep_w,x+cw-r,y+t+keep_h,y+ch-b);
    // Disjoint rectangles avoid applying translucency twice in the cross intersection.
    vec![[0,0,w,y],[0,y+ch,w,h],[0,y,x,y+ch],[x+cw,y,w,y+ch],
        [x,y0,x+cw,y1],[x0,y,x1,y0],[x0,y1,x1,y+ch]]
        .into_iter().filter(|[l,t,r,b]|l<r&&t<b).collect()
}
fn choose_mask_color(source:&image::RgbaImage,plan:&Plan)->Color32 {
    let candidates=[[240,35,45],[20,220,245],[245,30,220],[245,235,20],[40,65,245],[20,235,65],[255,255,255],[0,0,0]];
    let mut scores=[0f64;8];let mut samples=0;
    for [l,t,r,b] in removed_regions(plan) {
        let sx=(r-l).div_ceil(32) as usize;let sy=(b-t).div_ceil(32) as usize;
        for y in (t..b).step_by(sy) {for x in (l..r).step_by(sx) {
            let [red,green,blue,alpha]=source.get_pixel(x,y).0;let a=alpha as f64/255.;
            let base=[red as f64*a+120.*(1.-a),green as f64*a+120.*(1.-a),blue as f64*a+120.*(1.-a)];
            for (i,color) in candidates.iter().enumerate() {
                let dr=color[0] as f64-base[0];let dg=color[1] as f64-base[1];let db=color[2] as f64-base[2];
                let luminance=0.2126*dr+0.7152*dg+0.0722*db;
                scores[i]+=luminance*luminance+0.15*(dr*dr+dg*dg+db*db);
            }
            samples+=1;
        }}
    }
    let best=if samples==0 {0}else{scores.iter().enumerate().max_by(|(_,a),(_,b)|a.total_cmp(b)).map_or(0,|(i,_)|i)};
    let [r,g,b]=candidates[best];Color32::from_rgba_unmultiplied(r,g,b,135)
}
fn removed_mask(ui:&egui::Ui,rect:egui::Rect,plan:&Plan,color:Color32) {
    let [w,h]=plan.original;
    for [l,t,r,b] in removed_regions(plan) {
        let target=egui::Rect::from_min_max(rect.min+rect.size()*egui::vec2(l as f32/w as f32,t as f32/h as f32),rect.min+rect.size()*egui::vec2(r as f32/w as f32,b as f32/h as f32));
        ui.painter().rect_filled(target,0.,color);
        let painter=ui.painter().with_clip_rect(target.intersect(ui.clip_rect()));
        let stripe=Color32::from_rgba_unmultiplied(color.r(),color.g(),color.b(),230);
        // One image-wide phase keeps diagonal hatching continuous across the cross.
        let first=((target.left()+target.top())/14.).floor() as i32;
        let last=((target.right()+target.bottom())/14.).ceil() as i32;
        for index in first..=last {
            let sum=index as f32*14.;
            painter.line_segment([egui::pos2(target.left(),sum-target.left()),egui::pos2(target.right(),sum-target.right())],(1.25,stripe));
        }
    }
}
fn original_border_overlay(ui:&egui::Ui,rect:egui::Rect,w:u32,h:u32,[l,t,r,b]:[u32;4]) {
    let color=Color32::from_rgb(85,235,130);
    for x in [l,w-r] {let x=rect.left()+rect.width()*x as f32/w as f32;ui.painter().line_segment([egui::pos2(x,rect.top()),egui::pos2(x,rect.bottom())],(2.,color));}
    for y in [t,h-b] {let y=rect.top()+rect.height()*y as f32/h as f32;ui.painter().line_segment([egui::pos2(rect.left(),y),egui::pos2(rect.right(),y)],(2.,color));}
}
fn repeat_outline(ui:&egui::Ui,rect:egui::Rect,plan:&Plan) {
    if plan.treatment!=Treatment::Repeat||plan.periods==[0,0] {return;}
    let [w,h]=plan.original;let [l,t,_,_,cw,ch]=plan.values;
    let to_point=|x:u32,y:u32|rect.min+rect.size()*egui::vec2(x as f32/w as f32,y as f32/h as f32);
    let (x,y)=(plan.crop[0]+l,plan.crop[1]+t);
    let region=egui::Rect::from_min_max(to_point(x,y),to_point(x+cw,y+ch));
    ui.painter().rect_stroke(region,0.,(3.,Color32::BLACK),egui::StrokeKind::Inside);
    ui.painter().rect_stroke(region,0.,(1.,Color32::WHITE),egui::StrokeKind::Inside);
    for axis in 0..2 {
        let period=plan.periods[axis];if period==0 {continue;}
        let span=if axis==0 {cw}else{ch};let copies=span/period;
        // Bound preview geometry even if the chosen region contains thousands of cycles.
        let stride=copies.div_ceil(64).max(1);
        for i in (1..copies).step_by(stride as usize) {
            let (a,b)=if axis==0 {(to_point(x+i*period,y),to_point(x+i*period,y+ch))}else{(to_point(x,y+i*period),to_point(x+cw,y+i*period))};
            let length=a.distance(b);let direction=(b-a).normalized();
            for offset in (0..length.ceil() as usize).step_by(6) {
                let first=a+direction*offset as f32;let last=a+direction*(offset as f32+3.).min(length);
                ui.painter().line_segment([first,last],(2.5,Color32::BLACK));ui.painter().line_segment([first,last],(1.,Color32::WHITE));
            }
        }
    }
}

impl Editor {
    pub fn busy(&self)->bool {self.operation.is_some()}
    pub fn open_preview(&mut self,request:Request) {self.release();self.request=request;self.request.automatic=true;self.request.borders=None;self.open=true;}
    pub fn reset(&mut self) {self.release();self.request.borders=None;self.manual=false;self.status.clear();}
    pub fn refresh_message(ctx:&egui::Context,path:&Path)->Option<String> {
        ctx.data_mut(|data|data.remove_temp::<String>(egui::Id::new(("nine_slice_refresh",path))))
    }
    pub fn replaced(&mut self,message:String) {self.operation=None;self.release();self.request.borders=None;self.manual=false;self.status=message;}
    pub fn release(&mut self) {
        self.result=None;self.pending=None;self.failed=None;self.initialized=false;self.metadata=None;self.original_borders=None;self.confirm=false;self.autodetected=false;
    }
    pub fn show(&mut self,ctx:&egui::Context,path:&Path,source:&Arc<image::RgbaImage>,original_texture:&egui::TextureHandle)->bool {
        if let Some(receiver)=&self.operation {
            match receiver.try_recv() {
                Ok(result)=>{self.status=result.message;self.operation=None;self.confirm=false;if result.changed {self.release();self.request.borders=None;return true;}}
                Err(mpsc::TryRecvError::Disconnected)=>{self.status=tr("图片处理任务中断，请重载资源。").into();self.operation=None;self.release();return true;}
                Err(mpsc::TryRecvError::Empty)=>ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            }
        }
        if !self.open&&!self.busy() {return false;}
        let (w,h)=source.dimensions();
        if !self.initialized {
            self.metadata=meta::Meta::read(path);
            self.original_borders=self.metadata.as_ref().and_then(|m|m.borders(w,h));
            if self.request.borders.is_none()&&!self.request.automatic {
                if let Some(b)=self.original_borders.filter(|b|b.iter().any(|v|*v>0)) {self.request.borders=Some(b);}
            }
            self.initialized=true;
        }
        if let Some((key,receiver))=&self.pending {
            match receiver.try_recv() {
                Ok(generated)=>{
                    if generated.submitted==self.request {
                        if generated.submitted.automatic {self.request=generated.request.clone();self.autodetected=true;self.manual=false;}
                        self.result=Some(Arc::new(Preview {request:generated.request,plan:generated.plan,pixels:generated.pixels,texture:ctx.load_texture("nine_slice_result",generated.display,egui::TextureOptions::NEAREST),mask_color:generated.mask_color,trials:generated.trials}));
                    }
                    self.pending=None;
                }
                Err(mpsc::TryRecvError::Disconnected)=>{self.failed=Some(key.clone());self.pending=None;self.status=tr("图片处理任务中断，请重载资源。").into();}
                Err(mpsc::TryRecvError::Empty)=>{},
            }
        }
        let mut open=self.open;
        crate::native_window::Window::new(format!("{} · {}",tr("九宫格精简"),path.file_name().unwrap_or_default().to_string_lossy()))
            .id(egui::Id::new(("nine_slice",path))).open(&mut open).default_width(1000.).default_height(760.).min_width(600.).resizable(true).solid_scroll().show(ctx,|ui| {
            let busy=self.busy();
            let mut automatic=matches!(self.mask_color,MaskColor::Auto);
            equal_columns(ui,|columns| {
                columns[0].vertical(|ui| {
            let old=self.request.clone();let mut auto_clicked=false;
            ui.add_enabled_ui(!busy,|ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut direction=match self.request.direction {Mode::Both=>"both",Mode::Horizontal=>"horizontal",Mode::Vertical=>"vertical"}.to_owned();
                    if crate::theme::compact_selector(ui,&mut direction,[("both","同时"),("horizontal","横向"),("vertical","纵向")]) {
                        self.request.direction=match direction.as_str() {"horizontal"=>Mode::Horizontal,"vertical"=>Mode::Vertical,_=>Mode::Both};self.request.borders=None;self.manual=false;
                    }
                    ui.add_space(6.);
                    let mut treatment=if self.request.treatment==Treatment::Stretch {"stretch"}else{"repeat"}.to_owned();
                    if crate::theme::compact_selector(ui,&mut treatment,[("stretch","拉伸"),("repeat","重复")]) {
                        self.request.treatment=if treatment=="repeat" {Treatment::Repeat}else{Treatment::Stretch};self.request.borders=None;self.manual=false;
                    }
                    ui.add_space(6.);
                    if ui.add(crate::theme::toolbar_button(ui,tr("自动识别"),false)).clicked() {
                        self.request.automatic=true;self.request.borders=None;self.manual=false;self.autodetected=false;self.confirm=false;self.failed=None;auto_clicked=true;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let changed=ui.checkbox(&mut self.request.preserve_transparent,tr("保留边缘透明像素")).changed();
                    ui.checkbox(&mut self.request.multiple_of_four,tr("边长为 4 的倍数"));
                    if changed {self.request.borders=None;self.manual=false;}
                });
            });
            ui.add_enabled_ui(!busy,|ui| {
                ui.small(tr("完整比较 RGBA；拉伸仅精简一致行列，重复寻找最小循环，均不缩放像素。"));
                let fallback=[0,0,0,0,w,h];
                let current=self.result.as_ref().map_or(fallback,|r|r.plan.source_borders());
                let mut borders=self.request.borders.unwrap_or([current[0],current[1],current[2],current[3]]);
                let previous=borders;
                ui.horizontal(|ui| {
                    let label=ui.painter().layout_no_wrap(format!("{}:",tr("边界")),egui::TextStyle::Body.resolve(ui.style()),ui.visuals().text_color());
                    let (label_rect,_)=ui.allocate_exact_size(egui::vec2(label.size().x,50.),egui::Sense::hover());
                    ui.painter().galley(egui::pos2(label_rect.left(),label_rect.center().y-label.size().y*0.5),label,ui.visuals().text_color());
                    egui::Grid::new("nine_slice_border_inputs").min_col_width(0.).min_row_height(24.).spacing([2.,2.]).show(ui,|ui| {
                        for row in [[(1,"上"),(0,"左")],[(3,"下"),(2,"右")]] {
                            for (i,label) in row {
                                let active=if i%2==0 {self.request.direction!=Mode::Vertical}else{self.request.direction!=Mode::Horizontal};
                                let mut value=if active {borders[i]}else{0};
                                ui.add_enabled(active,egui::Label::new(tr(label)).wrap_mode(egui::TextWrapMode::Extend));
                                ui.add_enabled_ui(active,|ui| {ui.add_sized([64.,24.],egui::DragValue::new(&mut value).range(0..=if i%2==0 {w-1}else{h-1}));});
                                ui.add_enabled(active,egui::Label::new("px").wrap_mode(egui::TextWrapMode::Extend));
                                if i==1||i==3 {ui.allocate_exact_size(egui::vec2(8.,0.),egui::Sense::hover());}
                                if active {borders[i]=value;}
                            }
                            ui.end_row();
                        }
                    });
                });
                borders[0]=borders[0].min(w-1);borders[1]=borders[1].min(h-1);borders[2]=borders[2].min(w-1-borders[0]);borders[3]=borders[3].min(h-1-borders[1]);
                if borders!=previous {self.request.borders=Some(borders);self.manual=true;}
                ui.horizontal_wrapped(|ui| {for axis in 0..2 {
                    let active=if axis==0 {self.request.direction!=Mode::Vertical}else{self.request.direction!=Mode::Horizontal};
                    let repeat=self.request.treatment==Treatment::Repeat;
                    let label=if repeat {if axis==0 {"横向循环倍数"}else{"纵向循环倍数"}}else if axis==0 {"中心保留宽度"}else{"中心保留高度"};
                    ui.add_enabled_ui(active,|ui| {
                        ui.label(tr(label));
                        let value=if repeat {&mut self.request.copies[axis]}else{&mut self.request.center[axis]};
                        ui.add_sized([64.,24.],egui::DragValue::new(value).range(1..=if axis==0 {w}else{h}).suffix(if repeat {" ×"}else{" px"}));
                    });
                }});
            });
            if old!=self.request {self.confirm=false;self.failed=None;if !auto_clicked {self.request.automatic=false;self.autodetected=false;}}
            if ui.checkbox(&mut automatic,tr("自动选择遮罩对比色")).changed() {self.mask_color=if automatic {MaskColor::Auto}else{MaskColor::Red};}
                if self.original_borders.is_some() {ui.checkbox(&mut self.show_original_border,tr("裁剪图叠加原有 Unity Border"));}
                });
                let information_height=columns[0].min_rect().height().max(1.);
                egui::ScrollArea::vertical().id_salt("nine_slice_information").max_height(information_height).auto_shrink([false,false]).show(&mut columns[1],|ui| {
                    let preview=self.result.clone();
                    let plan=preview.as_ref().map(|r|&r.plan);
                    let [nw,nh]=plan.map_or([w,h],|p|p.size());
            ui.allocate_ui_with_layout(egui::vec2(ui.available_width(),18.),egui::Layout::left_to_right(egui::Align::Center),|ui| {
                if self.request.automatic {ui.small(tr("正在比较六种安全方案…"));}
                else if preview.as_ref().is_none_or(|r|r.request!=self.request) {ui.small(tr("正在更新预览，暂时显示上次结果。"));}
                else if self.autodetected {ui.small(format!("{}: {} · {}",tr("自动选择"),tr(self.request.direction.label()),tr(self.request.treatment.label())));}
            });
            let before=i64::from(w)*i64::from(h);let after=i64::from(nw)*i64::from(nh);let saving=before-after;
            let highlight=if saving>0 {Color32::from_rgb(25,110,65)}else if saving<0 {Color32::from_rgb(145,65,20)}else{Color32::from_rgb(75,80,90)};
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{w} × {h} →"));
                ui.label(egui::RichText::new(format!(" {nw} × {nh} ")).strong().color(Color32::WHITE).background_color(highlight));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{}: {before} → {after} px² · {}:",tr("像素面积"),tr(if saving>=0 {"节省面积"}else{"增加面积"})));
                ui.label(egui::RichText::new(format!(" {} px² ({:.2}%) ",saving.abs(),100.*saving.abs() as f64/before as f64)).strong().color(Color32::WHITE).background_color(highlight));
            });
            ui.small(tr("面积节省不等于 PNG 文件大小或 Unity 压缩后显存的节省比例。"));
            let shrink=if nw<w&&nh<h {"横向和纵向均已缩小"}else if nw<w&&nh==h {"仅横向缩小，高度不变"}else if nh<h&&nw==w {"仅纵向缩小，宽度不变"}else if nw<w {"横向缩小，纵向因对齐增加"}else if nh<h {"纵向缩小，横向因对齐增加"}else if nw>w||nh>h {"尺寸因 4 倍数对齐增加"}else{"当前未缩小图片"};
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(Color32::from_rgb(220,155,50),tr(shrink));
                if ui.small_button(tr(if self.show_details {"收起详情"}else{"查看详情"})).clicked() {self.show_details=!self.show_details;}
            });
            if self.show_details {
            if let Some(plan)=plan {
                ui.small(format!("{}: {} × {} px",tr("实际保留中心"),plan.values[4],plan.values[5]));
                for axis in 0..2 {
                    let report=&plan.reports[axis];let color=if report.reason==Reason::Reduced {ui.visuals().weak_text_color()}else{ui.visuals().warn_fg_color};
                    ui.colored_label(color,format!("{}: {}",tr(if axis==0 {"横向"}else{"纵向"}),tr(report.reason.label())));
                    if report.span>0 {
                        let matched=matches!(report.reason,Reason::Reduced|Reason::CenterKeepsAll|Reason::CopiesKeepAll|Reason::AlignmentKeepsAll);
                        if matched {
                            // Repeat's incomplete tail belongs to the fixed border, not the removable cycles.
                            let period=plan.periods[axis].max(1);
                            let span=if plan.treatment==Treatment::Repeat {report.span/period*period}else{report.span};
                            ui.small(format!("{} {} px · {} {} px · {} {} px",tr("可精简区"),span,tr("保留"),report.kept,tr("移除"),span.saturating_sub(report.kept)));
                        }else {ui.small(format!("{} {} px",tr("检查范围"),report.span));}
                        if report.protected>0 {ui.small(format!("{} {} px",tr("受保护透明外圈"),report.protected));}
                    }
                }
                if plan.reports.iter().any(|r|r.difference.is_some()) {
                    ui.collapsing(tr("RGBA 差异示例"),|ui| {for axis in 0..2 {if let Some(diff)=&plan.reports[axis].difference {
                        ui.small(format!("{}: ({}, {}) {:?} ≠ ({}, {}) {:?}",tr(if axis==0 {"横向"}else{"纵向"}),diff.points[0][0],diff.points[0][1],diff.rgba[0],diff.points[1][0],diff.points[1][1],diff.rgba[1]));
                        if diff.rgba[0][3]==0&&diff.rgba[1][3]==0 {ui.small(tr("这些像素虽然透明，隐藏 RGB 不同也不能视为一致。"));}
                    }}});
                }
                if let Some(result)=&preview {if !result.trials.is_empty() {
                    ui.collapsing(tr("六种方案比较详情"),|ui| {for trial in &result.trials {
                        ui.label(format!("{} · {}: {} × {}",tr(trial.direction.label()),tr(trial.treatment.label()),trial.size[0],trial.size[1]));
                        if !trial.eligible {ui.small(tr("未发现需要 Repeat 的非单像素周期，优先拉伸。"));}
                        for axis in 0..2 {ui.small(format!("{}: {}",tr(if axis==0 {"横向"}else{"纵向"}),tr(trial.reports[axis].reason.label())));}
                    }});
                }}
                if plan.crop!=[0,0,w,h] {ui.small(format!("{}: ({}, {}) · {} × {}",tr("去除外圈透明像素"),plan.crop[0],plan.crop[1],plan.crop[2],plan.crop[3]));}
                if plan.padding!=[0,0] {let [l,t,r,b]=plan.padding_sides();ui.small(format!("{} (L, T, R, B): {l}, {t}, {r}, {b} px",tr("对齐补白，不计入 Border")));}
                if self.request.treatment==Treatment::Repeat {
                    let period=|axis:usize|if plan.periods[axis]==0 {tr("无可用循环").to_owned()}else{format!("{} px × {}",plan.periods[axis],plan.values[axis+4]/plan.periods[axis])};
                    ui.label(format!("{}: {} · {}: {}",tr("横向最小循环与实际倍数"),period(0),tr("纵向最小循环与实际倍数"),period(1)));
                }
            }else {ui.label(tr("正在逐像素检查 RGBA…"));}
            }
                });
            });
            let preview=self.result.clone();
            let plan=preview.as_ref().map(|r|&r.plan);
            let [nw,nh]=plan.map_or([w,h],|p|p.size());
            let preview_current=preview.as_ref().is_some_and(|r|r.request==self.request);

            let preview_height=(ui.clip_rect().bottom()-ui.cursor().top()-if self.footer_height>0. {self.footer_height}else{120.}-48.).max(180.);
            equal_columns(ui,|columns| {
                columns[0].strong(tr("裁剪展示"));
                columns[1].strong(tr("结果展示"));
                columns[0].small(tr(if self.autodetected {"来源：六种方案自动比较"}else if self.manual {"来源：手动设置"}else if self.request.borders.is_some() {"来源：沿用原有边界"}else{"来源：像素自动识别"}));
                columns[1].small(tr("本次输出切分线"));
                for (index,column) in columns.iter_mut().enumerate() {
                    let (area,_)=column.allocate_exact_size(egui::vec2(column.available_width().max(1.),preview_height),egui::Sense::hover());
                    let (pw,ph)=if index==0 {(w,h)}else{(nw,nh)};
                    let rect=egui::Rect::from_center_size(area.center(),fitted(pw,ph,(area.width()-16.).max(1.),area.height()-76.));checker(column,rect);
                    let texture=if index==0 {Some(original_texture)}else{preview.as_ref().map(|r|&r.texture)};
                    if let Some(texture)=texture {
                        column.painter().image(texture.id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.,1.)),Color32::WHITE);
                        if let Some(plan)=plan {
                            if index==0 {let color=if automatic {preview.as_ref().unwrap().mask_color}else{Color32::from_rgba_unmultiplied(240,35,45,135)};removed_mask(column,rect,plan,color);repeat_outline(column,rect,plan);}
                            guides(column,rect,pw,ph,if index==1 {plan.borders()}else{plan.source_borders()},preview.as_ref().unwrap().request.direction);
                            if index==0&&self.show_original_border {if let Some(border)=self.original_borders {original_border_overlay(column,rect,w,h,border);}}
                        }
                    }else {column.painter().text(area.center(),egui::Align2::CENTER_CENTER,tr("正在生成精简预览…"),egui::FontId::proportional(14.),column.visuals().text_color());}                    let gap=if preview.as_ref().is_some_and(|r|r.request.direction!=Mode::Vertical) {28.}else{6.};
                    let painter=column.painter();
                    let text=painter.layout_no_wrap(format!("{pw} × {ph} px"),egui::FontId::proportional(12.),Color32::WHITE);
                    let backing=egui::Align2::CENTER_TOP.anchor_rect(egui::Rect::from_min_size(egui::pos2(rect.center().x,rect.bottom()+gap),text.size()+egui::vec2(16.,8.)));
                    painter.rect_filled(backing,6.,Color32::from_black_alpha(205));
                    painter.galley(backing.min+egui::vec2(8.,4.),text,Color32::WHITE);

                }
            });
            let footer_top=ui.cursor().top();
                ui.small(tr("半透明斜纹遮罩：准备移除的像素；黄 / 蓝线：本次切分线。"));
                if plan.is_some_and(|p|p.treatment==Treatment::Repeat) {ui.small(tr("白框内保留 N 个完整循环，白色虚线标最小循环边界；遮罩区移除，固定边缘保持原样。"));}
                if self.show_original_border {if let Some([l,t,r,b])=self.original_borders {
                    ui.small(format!("{} · L {l}, B {b}, R {r}, T {t} px",tr("绿色：原 .meta 的 Unity Border")));
                }}
            ui.small(tr(if self.request.treatment==Treatment::Repeat {"Unity UI 使用 Tiled；增加循环倍数可减少平铺次数，实际网格数量由 Unity 决定。"}else{"Unity UI 使用 Sliced。"}));
            let meta_error=self.metadata.as_ref().and_then(|m|m.supported().err());
            let lossless=matches!(image::ImageFormat::from_path(path),Ok(image::ImageFormat::Png|image::ImageFormat::Tga|image::ImageFormat::Bmp|image::ImageFormat::WebP));
            ui.separator();
            ui.scope(|ui| {
                // Keep footer actions clear of the native window's overlay scrollbar.
                ui.set_max_width((ui.available_width()-18.).max(0.));
                let actions_width=288.+ui.spacing().item_spacing.x;
                let information_width=(ui.available_width()-actions_width-ui.spacing().item_spacing.x).max(0.);
                if self.metadata.is_none()&&!lossless {ui.small(tr("此格式不支持无损原位替换，请另存 PNG。"));}
                ui.horizontal(|ui| {
                    if self.metadata.is_some() {
                    ui.allocate_ui_with_layout(egui::vec2(information_width,44.),egui::Layout::top_down(egui::Align::Min),|ui| {
                        ui.set_min_width(information_width);
                        if let Some(metadata)=&self.metadata {
                            ui.add(egui::Label::new(metadata.path.display().to_string()).wrap());
                            if let Some(error)=&meta_error {ui.colored_label(Color32::from_rgb(220,155,50),error);}
                            else if let Some(plan)=plan {let v=plan.borders();ui.label(format!("{} (L, B, R, T): {}, {}, {}, {} px",tr("待写入 Border"),v[0],v[3],v[2],v[1]));ui.small(tr("同步 Border 和 Full Rect，保留 GUID；不更改场景或 Prefab 的 Image Type。"));}
                        }
                        if !lossless {ui.small(tr("此格式不支持无损原位替换，请另存 PNG。"));}
                    });
                    }else {ui.add_space(((ui.available_width()-actions_width)*0.5-ui.spacing().item_spacing.x).max(0.));}
            ui.allocate_ui_with_layout(egui::vec2(actions_width,44.),egui::Layout::right_to_left(egui::Align::Center),|ui| {
                if ui.add_enabled(!busy&&preview_current,crate::theme::toolbar_button(ui,tr("另存为"),false).min_size(egui::vec2(144.,44.))).clicked() {
                    if let Some(target)=rfd::FileDialog::new().add_filter("PNG",&["png"]).set_file_name(format!("{}_9slice.png",path.file_stem().unwrap_or_default().to_string_lossy())).save_file() {
                        let pixels=preview.as_ref().unwrap().pixels.clone();let repaint=ctx.clone();let (send,receive)=mpsc::channel();self.operation=Some(receive);
                        std::thread::spawn(move||{let message=match meta::export(&pixels,&target) {Ok(())=>format!("{}: {}",tr("已保存"),target.display()),Err(e)=>e};let _=send.send(meta::Replacement {changed:false,message});repaint.request_repaint();});
                    }
                }
                if ui.add_enabled(!busy&&preview_current&&lossless&&meta_error.is_none(),replace_button(tr("原位替换"))).clicked() {self.confirm=true;}
            });
                });
            });
            if self.confirm&&!busy&&preview_current {
                ui.group(|ui| {
                    ui.strong(tr("确认后直接覆盖此图片，不创建备份。"));ui.label(path.display().to_string());ui.label(format!("{w} × {h} → {nw} × {nh}"));
                    if self.metadata.is_some() {ui.label(tr("同时写入已读取的同名 .meta。"));}
                    ui.horizontal(|ui| {
                        if ui.add(replace_button(tr("确认原位替换"))).clicked() {
                            if let (Some(result),Some(root))=(&preview,crate::app_cache::root()) {
                                let source=source.clone();let result=result.clone();let metadata=self.metadata.clone();let path=path.to_owned();let repaint=ctx.clone();let (send,receive)=mpsc::channel();self.operation=Some(receive);self.confirm=false;
                                std::thread::spawn(move|| {
                                    let outcome=meta::replace_plan(&path,&source,&result.pixels,&result.request,&result.plan,metadata.as_ref(),&root.join("image_edit_temp"));
                                    if outcome.changed {repaint.data_mut(|data|data.insert_temp(egui::Id::new(("nine_slice_refresh",path.as_path())),outcome.message.clone()));}
                                    let _=send.send(outcome);repaint.request_repaint();
                                });
                            }else {self.status=tr("无法定位软件记录目录，未替换图片。").into();}
                        }
                        if ui.button(tr("取消")).clicked() {self.confirm=false;}
                    });
                });
            }
            if busy {ui.spinner();ui.label(tr("正在处理图片，请稍候…"));}
            if !self.status.is_empty() {ui.label(&self.status);}
            let footer_height=ui.cursor().top()-footer_top;
            if (self.footer_height-footer_height).abs()>0.5 {self.footer_height=footer_height;ctx.request_repaint();}
        });
        if !open {self.show_details=false;}
        self.open=open||self.busy();
        if self.pending.is_none()&&!self.busy()&&self.failed.as_ref()!=Some(&self.request)&&self.result.as_ref().is_none_or(|r|r.request!=self.request) {
            let source=source.clone();let request=self.request.clone();let repaint=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some((request.clone(),receive));
            std::thread::spawn(move||{
                let submitted=request.clone();
                let (request,plan,trials)=if request.automatic {processing::automatic(&source,&request)}else{let plan=Plan::new(&source,&request);(request,plan,Vec::new())};
                let pixels=Arc::new(plan.render(&source));let display=color_image(&pixels);let mask_color=choose_mask_color(&source,&plan);
                let _=send.send(Generated {submitted,request,plan,pixels,display,mask_color,trials});repaint.request_repaint();
            });
        }
        false
    }
}
#[cfg(test)]
fn compact(source:&image::RgbaImage,v:[u32;6])->image::RgbaImage {
    let request=Request {borders:Some([v[0],v[1],v[2],v[3]]),center:[v[4],v[5]],multiple_of_four:false,..Default::default()};
    Plan::new(source,&request).render(source)
}

#[cfg(test)]
mod mask_tests {
    use super::*;
    #[test]
    fn default_center_is_four_and_masks_cover_removed_pixels_once() {
        let source=image::RgbaImage::from_pixel(20,16,image::Rgba([255,255,255,255]));
        let plan=Plan::new(&source,&Request::default());assert_eq!(plan.size(),[4,4]);
        let mut visits=vec![0u8;320];
        for [l,t,r,b] in removed_regions(&plan) {for y in t..b {for x in l..r {visits[(y*20+x) as usize]+=1;}}}
        assert!(visits.iter().all(|n|*n<=1));assert_eq!(visits.iter().filter(|n|**n==1).count(),320-16);
    }
    #[test]
    fn automatic_mask_uses_translucent_contrasting_colors_for_bright_and_dark_sources() {
        let white=image::RgbaImage::from_pixel(20,16,image::Rgba([255,255,255,255]));
        let black=image::RgbaImage::from_pixel(20,16,image::Rgba([0,0,0,255]));
        assert_eq!(choose_mask_color(&white,&Plan::new(&white,&Request::default())),Color32::from_rgba_unmultiplied(0,0,0,135));
        assert_eq!(choose_mask_color(&black,&Plan::new(&black,&Request::default())),Color32::from_rgba_unmultiplied(255,255,255,135));
    }
}
