//! Read-only PDFium rendering. All native handles stay on the rendering thread.
use std::{ffi::{c_void,c_char,c_int}, io::Read, path::Path, sync::{Arc,Mutex,mpsc}};
use eframe::egui::{self,Color32};
use crate::localization::tr;

pub(crate) static NATIVE_LOCK: Mutex<()> = Mutex::new(());
const SOURCE_LIMIT:u64=128*1024*1024;
// PDF page dimensions are 1/72 inch; UI logical pixels use a 96 DPI baseline.
const POINT_TO_DIP:f32=96.0/72.0;
type Handle=*mut c_void;
pub(crate) fn is_fullscreen(ctx:&egui::Context)->bool{
    ctx.data(|data|data.get_temp::<std::path::PathBuf>(egui::Id::new("pdf_fullscreen_path"))).is_some()
}
fn set_fullscreen(ctx:&egui::Context,path:Option<&Path>){
    ctx.data_mut(|data|{let id=egui::Id::new("pdf_fullscreen_path");if let Some(path)=path{data.insert_temp(id,path.to_owned());}else{data.remove::<std::path::PathBuf>(id);}});
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(path.is_some()));ctx.request_repaint();
}
pub(crate) fn update_fullscreen(ctx:&egui::Context,path:Option<&Path>)->bool{
    let owner=ctx.data(|data|data.get_temp::<std::path::PathBuf>(egui::Id::new("pdf_fullscreen_path")));
    let pdf=path.filter(|path|super::is_pdf(path));
    if owner.is_some()&&owner.as_deref()!=pdf{set_fullscreen(ctx,None);return false;}
    let toggle=pdf.is_some()&&ctx.input_mut(|input|input.consume_key(egui::Modifiers::NONE,egui::Key::F11));
    let escape=owner.is_some()&&ctx.input_mut(|input|input.consume_key(egui::Modifiers::NONE,egui::Key::Escape));
    if escape || (toggle&&owner.is_some()){set_fullscreen(ctx,None);}
    else if toggle {set_fullscreen(ctx,pdf);}
    is_fullscreen(ctx)
}
struct Rendered { source:Arc<Vec<u8>>, page:usize, count:usize, size:egui::Vec2, image:egui::ColorImage }
// Function pointers are copied only while the owning Library remains alive.
unsafe fn symbol<T:Copy>(lib:&libloading::Library,name:&[u8])->Result<T,String>{
    unsafe {lib.get::<T>(name).map(|value|*value).map_err(|error|error.to_string())}
}
struct Engine {lib:libloading::Library, destroy:unsafe extern "system" fn()}
impl Drop for Engine {fn drop(&mut self){unsafe{(self.destroy)()}}}
struct Document {handle:Handle,close:unsafe extern "system" fn(Handle)}
impl Drop for Document {fn drop(&mut self){unsafe{(self.close)(self.handle)}}}
fn raster_size(size:egui::Vec2,scale:f32)->[usize;2]{
    let scale=scale.clamp(0.001,64.0).min(4096.0/size.x.max(size.y));
    [(size.x*scale).round().max(1.0) as usize,(size.y*scale).round().max(1.0) as usize]
}
fn render(source:Arc<Vec<u8>>,page:usize,scale:f32)->Result<Rendered,String>{
    let _lock=NATIVE_LOCK.lock().map_err(|_|tr("PDFium 任务中断").to_owned())?;
    let path=crate::third_party::verified_pdfium_path()?;
    unsafe {
        let lib=libloading::Library::new(path).map_err(|e|e.to_string())?;
        let init:unsafe extern "system" fn()=symbol(&lib,b"FPDF_InitLibrary\0")?;
        let destroy= symbol(&lib,b"FPDF_DestroyLibrary\0")?;
        init();
        let engine=Engine{lib,destroy};let lib=&engine.lib;
        let load:unsafe extern "system" fn(*const c_void,usize,*const c_char)->Handle=symbol(lib,b"FPDF_LoadMemDocument64\0")?;
        let close= symbol(lib,b"FPDF_CloseDocument\0")?;
        let get_error:unsafe extern "system" fn()->std::ffi::c_ulong=symbol(lib,b"FPDF_GetLastError\0")?;
        let handle=load(source.as_ptr().cast(),source.len(),std::ptr::null());
        if handle.is_null(){return Err(tr(match get_error(){4=>"PDF 需要密码，暂不支持加密文档。",_=>"PDF 无法打开，文件可能损坏或格式不受支持。"}).into());}
        let doc=Document{handle,close};
        let count:unsafe extern "system" fn(Handle)->c_int=symbol(lib,b"FPDF_GetPageCount\0")?;
        let count=count(doc.handle);if count<=0{return Err(tr("PDF 没有可预览页面").into());}
        let page=page.min(count as usize-1);
        let load_page:unsafe extern "system" fn(Handle,c_int)->Handle=symbol(lib,b"FPDF_LoadPage\0")?;
        let close_page=symbol(lib,b"FPDF_ClosePage\0")?;
        let width:unsafe extern "system" fn(Handle)->f64=symbol(lib,b"FPDF_GetPageWidth\0")?;
        let height:unsafe extern "system" fn(Handle)->f64=symbol(lib,b"FPDF_GetPageHeight\0")?;
        let handle=load_page(doc.handle,page as c_int);if handle.is_null(){return Err(tr("PDF 页面加载失败").into());}
        let loaded=Document{handle,close:close_page};
        let size=egui::vec2(width(handle) as f32,height(handle) as f32)*POINT_TO_DIP;
        if !size.is_finite()||size.x<=0.0||size.y<=0.0{return Err(tr("PDF 页面尺寸无效").into());}
        let [w,h]=raster_size(size,scale);
        let create:unsafe extern "system" fn(c_int,c_int,c_int,*mut c_void,c_int)->Handle=symbol(lib,b"FPDFBitmap_CreateEx\0")?;
        let bitmap_close=symbol(lib,b"FPDFBitmap_Destroy\0")?;
        let draw:unsafe extern "system" fn(Handle,Handle,c_int,c_int,c_int,c_int,c_int,c_int)=symbol(lib,b"FPDF_RenderPageBitmap\0")?;
        // FPDFBitmap_BGRA = 4; opaque white background. Annotations included, no JS/actions.
        let mut bytes=vec![255u8;w*h*4];
        let bitmap=create(w as c_int,h as c_int,4,bytes.as_mut_ptr().cast(),(w*4) as c_int);
        if bitmap.is_null(){return Err(tr("PDF 页面渲染失败").into());}
        let bitmap=Document{handle:bitmap,close:bitmap_close};
        // Annotations plus PDFium's LCD text antialiasing for screen viewing.
        draw(bitmap.handle,loaded.handle,0,0,w as c_int,h as c_int,0,1|2);
        drop(bitmap);
        let pixels=bytes.chunks_exact(4).map(|p|Color32::from_rgb(p[2],p[1],p[0])).collect();
        Ok(Rendered{source,page,count:count as usize,size,image:egui::ColorImage{size:[w,h],pixels}})
    }
}
#[derive(Default)]
pub(super) struct Preview {
    source:Option<Arc<Vec<u8>>>, pending:Option<mpsc::Receiver<Result<Rendered,String>>>,
    texture:Option<egui::TextureHandle>,size:Option<egui::Vec2>,page:usize,count:usize,
    zoom:Option<f32>,fit_width:bool,pan:egui::Vec2,error:Option<String>,retry_requested:bool,
    auto_page:bool,loop_pages:bool,interval:Option<f64>,elapsed:f64,last_tick:Option<f64>,reload_requested:bool,
    displayed_page:Option<usize>,pending_since:Option<std::time::Instant>,
    transition:usize,fade:Option<(egui::TextureHandle,f64,f32)>,
}
impl Preview {
    pub(super) fn retry_component(&mut self){
        if self.error.is_some() || self.pending.is_some(){self.retry_requested=true;}
    }
    fn request(&mut self,path:&Path,scale:f32,ctx:&egui::Context){
        let path=path.to_owned();let source=self.source.clone();let page=self.page;let ctx=ctx.clone();
        let (send,receive)=mpsc::channel();self.pending=Some(receive);self.pending_since=Some(std::time::Instant::now());
        std::thread::spawn(move||{
            let result=(||{
                let source=match source{Some(s)=>s,None=>{
                    let mut bytes=Vec::new();std::fs::File::open(path).map_err(|e|e.to_string())?.take(SOURCE_LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                    if bytes.len() as u64>SOURCE_LIMIT{return Err(tr("PDF 文件超过 128 MiB，无法预览").into());}Arc::new(bytes)
                }};render(source,page,scale)
            })();let _=send.send(result);ctx.request_repaint();
        });
    }
    pub(super) fn show(&mut self,ui:&mut egui::Ui,path:&Path){
        let now=ui.input(|input|input.time);
        let delta=self.last_tick.replace(now).map(|last|now-last).filter(|delta|*delta>=0.0&&*delta<1.0).unwrap_or(0.0);
        if let Some(receiver)=&self.pending {match receiver.try_recv(){
            Ok(result)=>{self.pending=None;match result{Ok(r)=>{
                if self.displayed_page!=Some(r.page){
                    let direction=if r.page<self.displayed_page.unwrap_or(r.page){-1.0}else{1.0};
                    self.fade=if self.transition!=0{self.texture.take().map(|texture|(texture,now,direction))}else{None};
                }
                if self.displayed_page!=Some(r.page){self.pan=egui::Vec2::ZERO;}
                self.displayed_page=Some(r.page);
                self.source=Some(r.source);self.page=r.page;self.count=r.count;self.size=Some(r.size);
                self.texture=Some(ui.ctx().load_texture(format!("pdf:{}",path.display()),r.image,egui::TextureOptions::LINEAR));self.error=None;
            },Err(e)=>self.error=Some(e)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.error=Some(tr("PDFium 任务中断").into());},_=>{}
        }}
        if self.reload_requested&&self.pending.is_none(){*self=Self::default();}
        // Keep the notification while an earlier render is still in flight, so a
        // late missing-component error cannot overwrite an installation event.
        if self.pending.is_none() && self.retry_requested {
            self.retry_requested=false;
            if self.error.take().is_some(){self.request(path,1.0,ui.ctx());}
        }
        let old=self.page;
        if self.count<=1{self.auto_page=false;self.elapsed=0.0;}
        let fullscreen=is_fullscreen(ui.ctx());
        if !fullscreen {ui.horizontal_wrapped(|ui|{
            super::file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);
            if self.count>1{
            let label=tr("自动翻页");
            let width=ui.painter().layout_no_wrap(label.into(),egui::TextStyle::Button.resolve(ui.style()),ui.visuals().text_color()).size().x+48.0;
            let changed=ui.allocate_ui_with_layout(egui::vec2(width,26.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                crate::theme::toggle_switch(ui,label,&mut self.auto_page).changed()
            }).inner;
            let interval=self.interval.get_or_insert(5.0);
            if ui.add(egui::DragValue::new(interval).range(1.0..=3600.0).speed(1.0).max_decimals(0)).changed()||changed{self.elapsed=0.0;}
            ui.label(tr("秒"));
            let label=tr("循环");
            let width=ui.painter().layout_no_wrap(label.into(),egui::TextStyle::Button.resolve(ui.style()),ui.visuals().text_color()).size().x+48.0;
            ui.allocate_ui_with_layout(egui::vec2(width,26.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                if crate::theme::toggle_switch(ui,label,&mut self.loop_pages).changed(){self.elapsed=0.0;}
            });
            ui.label(tr("过渡"));
            let styles=["无","淡入淡出","左右滑动","推页","擦除","缩放淡入"];
            let old_transition=self.transition;
            egui::ComboBox::from_id_salt("pdf_transition").selected_text(tr(styles[self.transition])).show_ui(ui,|ui|{
                for (index,label) in styles.iter().enumerate(){ui.selectable_value(&mut self.transition,index,tr(label));}
            });
            if self.transition!=old_transition{self.fade=None;}
            }
        });}
        if self.auto_page&&self.pending.is_none()&&self.error.is_none()&&self.texture.is_some(){
            self.elapsed+=delta;
            if self.elapsed>=self.interval.unwrap_or(5.0){
                self.elapsed=0.0;
                if self.page+1<self.count{self.page+=1;}else if self.loop_pages{self.page=0;}else{self.auto_page=false;}
            }
        }
        if self.auto_page{ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
        if self.pending.is_none() && self.count>0 && !ui.ctx().wants_keyboard_input(){
            let direction=ui.input_mut(|input|{
                let previous=input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowLeft)
                    | input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowUp);
                let next=input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowRight)
                    | input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowDown);
                if previous {-1} else if next {1} else {0}
            });
            self.page=if direction<0 {self.page.saturating_sub(1)} else if direction>0 {(self.page+1).min(self.count-1)} else {self.page};
        }
        if !fullscreen {ui.horizontal_wrapped(|ui|{
            if ui.button(tr("适应窗口")).clicked(){self.zoom=None;self.fit_width=false;self.pan=egui::Vec2::ZERO;}
            if ui.button(tr("适应宽度")).clicked(){self.zoom=None;self.fit_width=true;self.pan=egui::Vec2::ZERO;}
            for (label,value) in [("50%",0.5),("100%",1.0),("200%",2.0)]{if ui.button(label).clicked(){self.zoom=Some(value);self.pan=egui::Vec2::ZERO;}}
            // Rendering is routine work, not a reason to dim the reload control.
            // Coalesce reload clicks until the current native task has finished.
            if ui.button(tr("重载")).clicked(){
                if self.pending.is_some(){self.reload_requested=true;}else{*self=Self::default();}
            }
            if ui.button(tr("全屏")).on_hover_text("F11").clicked(){set_fullscreen(ui.ctx(),Some(path));}
        });
        ui.small(tr("PDF 仅支持只读预览，暂不支持编辑、文字选择或搜索。"));
        }
        if let Some(error)=&self.error{
            self.auto_page=false;self.elapsed=0.0;
            ui.colored_label(ui.visuals().error_fg_color,error);
            if ui.button(tr("第三方支持")).clicked(){crate::third_party::request_open(ui.ctx());}
            if ui.button(tr("重试")).clicked(){self.error=None;self.request(path,1.0,ui.ctx());}return;
        }
        if old!=self.page{self.elapsed=0.0;}
        if self.pending.is_none()&&self.texture.is_none(){self.request(path,1.0,ui.ctx());}
        let show_spinner=self.pending.is_some()&&self.pending_since.is_some_and(|start|start.elapsed()>=std::time::Duration::from_secs(1));
        let Some(size)=self.size else{
            if show_spinner{ui.spinner();}
            if self.pending.is_some(){ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
            return;
        };
        let (canvas,response)=ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)),egui::Sense::drag());
        let width=(canvas.width()-16.0).max(1.0)/size.x;
        let mut zoom=self.zoom.unwrap_or(if self.fit_width{width}else{width.min((canvas.height()-16.0).max(1.0)/size.y)}).clamp(0.01,64.0);
        if response.dragged(){self.pan+=response.drag_delta();}
        if response.hovered(){let scroll=ui.input(|i|i.smooth_scroll_delta.y);if scroll!=0.0{
            let next=(zoom*(scroll*0.002).exp()).clamp(0.01,64.0);
            if let Some(pointer)=response.hover_pos(){let relative=pointer-canvas.center();self.pan=relative-(relative-self.pan)*(next/zoom);}
            zoom=next;self.zoom=Some(zoom);
        }}
        let desired=raster_size(size,zoom*ui.ctx().pixels_per_point());
        if self.pending.is_none()&&(self.displayed_page!=Some(self.page)||self.texture.as_ref().is_some_and(|t|t.size()!=desired)){self.request(path,zoom*ui.ctx().pixels_per_point(),ui.ctx());}
        // Match render pixels to screen pixels and avoid half-pixel placement
        // introducing another linear-filter blur after PDFium rasterization.
        let dpi=ui.ctx().pixels_per_point();
        let display_size=(size*zoom*dpi).round()/dpi;
        let min=((canvas.center()+self.pan-display_size*0.5).to_vec2()*dpi).round()/dpi;
        let rect=egui::Rect::from_min_size(min.to_pos2(),display_size);
        let alpha=self.fade.as_ref().filter(|_|self.transition!=0).map(|(_,start,_)|((now-start)/0.25).clamp(0.0,1.0) as f32).unwrap_or(1.0);
        if alpha>=1.0{self.fade=None;}
        let page_painter=ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        let uv=egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0));
        if let Some(texture)=&self.texture{
            if let Some((previous,_,direction))=&self.fade{
                let progress=alpha*alpha*(3.0-2.0*alpha);
                let offset=egui::vec2(rect.width()*direction,0.0);
                let clipped=page_painter.with_clip_rect(rect.intersect(canvas).intersect(ui.clip_rect()));
                let old_rect=if self.transition==3{rect.translate(-offset*progress)}else{rect};
                clipped.image(previous.id(),old_rect,uv,Color32::WHITE);
                match self.transition{
                    2|3=>{clipped.image(texture.id(),rect.translate(offset*(1.0-progress)),uv,Color32::WHITE);},
                    4=>{
                        let mut reveal=rect;
                        if *direction>0.0{reveal.max.x=rect.left()+rect.width()*progress;}else{reveal.min.x=rect.right()-rect.width()*progress;}
                        clipped.with_clip_rect(reveal.intersect(clipped.clip_rect())).image(texture.id(),rect,uv,Color32::WHITE);
                    },
                    _=>{
                        let new_rect=if self.transition==5{egui::Rect::from_center_size(rect.center(),rect.size()*(0.96+0.04*progress))}else{rect};
                        // Source-over of the new page gives a continuous dissolve.
                        clipped.image(texture.id(),new_rect,uv,Color32::from_white_alpha((progress*255.0).round() as u8));
                    }
                }
            }else{page_painter.image(texture.id(),rect,uv,Color32::WHITE);}
        }
        if self.fade.is_some(){ui.ctx().request_repaint();}
        if show_spinner{ui.put(egui::Rect::from_min_size(canvas.min+egui::vec2(8.0,8.0),egui::Vec2::splat(20.0)),egui::Spinner::new());}
        // Match the existing image zoom badge: 13pt text with 12x6 padding,
        // eight points above the canvas bottom. Buttons share its exact height.
        let painter=ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        let galley=painter.layout_no_wrap(format!("{}/{} {:.0}%",self.displayed_page.unwrap_or(self.page)+1,self.count,zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE);
        let badge_size=galley.size()+egui::vec2(24.0,12.0);
        let badge=egui::Rect::from_center_size(egui::pos2(canvas.center().x,canvas.bottom()-8.0-badge_size.y*0.5),badge_size);
        painter.rect_filled(badge,6.0,Color32::from_black_alpha(128));
        painter.galley(badge.min+egui::vec2(12.0,6.0),galley,Color32::WHITE);
        if let Some(value)=crate::theme::zoom_preset_menu(ui,badge,zoom,badge.right()-12.0){self.zoom=Some(value);self.pan=egui::Vec2::ZERO;}
        let mut target=self.page;
        for (previous,x) in [(true,badge.left()-12.0-badge_size.y*0.5),(false,badge.right()+12.0+badge_size.y*0.5)]{
            let center=egui::pos2(x,badge.center().y);
            let button=egui::Rect::from_center_size(center,egui::Vec2::splat(badge_size.y));
            let enabled=self.pending.is_none() && if previous {self.page>0} else {self.page+1<self.count};
            let response=ui.interact(button,ui.id().with(("pdf_page",previous)),egui::Sense::click());
            painter.circle_filled(center,badge_size.y*0.5,Color32::from_black_alpha(if enabled&&response.hovered(){180}else{128}));
            // Center the triangle's centroid, rather than its bounding box.
            let direction=if previous {-1.0}else{1.0};
            let r=badge_size.y*0.18;
            let points=vec![center+egui::vec2(direction*r,0.0),center+egui::vec2(-direction*r*0.5,-r*0.8660254),center+egui::vec2(-direction*r*0.5,r*0.8660254)];
            painter.add(egui::Shape::convex_polygon(points,if enabled{Color32::WHITE}else{Color32::from_white_alpha(70)},egui::Stroke::NONE));
            if enabled&&response.clicked(){target=if previous{self.page-1}else{self.page+1};}
        }
        if target!=self.page{self.elapsed=0.0;self.page=target;ui.ctx().request_repaint();}
        if self.pending.is_some(){ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn raster_is_bounded(){assert_eq!(raster_size(egui::vec2(100000.0,50000.0),64.0),[4096,2048]);assert_eq!(raster_size(egui::vec2(1.0,1.0),0.001),[1,1]);}
}
