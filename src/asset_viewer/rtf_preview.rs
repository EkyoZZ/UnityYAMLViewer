//! Direct Windows RichEdit RTF rendering; no Office conversion or PDF backend.
use std::{path::{Path,PathBuf},sync::mpsc};
use eframe::egui;
use crate::localization::tr;
#[cfg(windows)]
#[path = "rtf_editor.rs"]
mod editor;
struct Document{pages:Vec<egui::ColorImage>,text:String}
#[derive(Default)]
pub(super) struct Preview{path:PathBuf,pending:Option<mpsc::Receiver<Result<Document,String>>>,pages:Vec<egui::TextureHandle>,text:String,error:Option<String>,zoom:f32,text_view:bool,
    #[cfg(windows)]
    editors: std::collections::HashMap<PathBuf, editor::Session>,
    page: usize,
}
impl Preview{
    pub(super) fn update_editor_theme(&self, _ctx: &egui::Context) {
        #[cfg(windows)]
        if !self.editors.is_empty() {
            let dark = _ctx.style().visuals.dark_mode;
            for editor in self.editors.values() {
                editor.update_theme(dark);
            }
        }
    }

    pub(super) fn invalidate(&mut self, path: &Path) {
        if self.path == path {
            self.path.clear();
            self.pending = None;
            self.pages.clear();
            self.text.clear();
        }
    }

    pub(super) fn request_editor_close(&self) -> bool {
        #[cfg(windows)]
        {
            if let Some(session) = self.editors.values().find(|session| session.is_open()) {
                session.request_close();
                return true;
            }
        }
        false
    }

    fn load(&mut self,ctx:&egui::Context,path:&Path){
        self.path=path.to_owned();self.pages.clear();self.text.clear();self.error=None;self.zoom=1.0;self.page=0;
        let (tx,rx)=mpsc::channel();self.pending=Some(rx);let path=path.to_owned();let ctx=ctx.clone();
        std::thread::spawn(move||{let _=tx.send(render(&path));ctx.request_repaint();});
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path){
        if self.path!=path{self.text_view=false;self.load(ui.ctx(),path);}
        #[cfg(windows)]
        {
            let mut reload = false;
            self.editors.retain(|_, session| {
                let open = session.is_open();
                reload |= session.take_saved(path);
                open
            });
            if reload {
                let zoom = self.zoom;
                self.load(ui.ctx(), path);
                self.zoom = zoom;
            }
        }
        if let Some(result)=self.pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.pending=None;match result{Ok(doc)=>{self.text=doc.text;self.pages=doc.pages.into_iter().enumerate().map(|(i,page)|ui.ctx().load_texture(format!("rtf:{}:{i}",path.display()),page,egui::TextureOptions::LINEAR)).collect();},Err(error)=>self.error=Some(error)}
        }
        let mut jump = false;
        ui.horizontal_wrapped(|ui|{
            super::file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);
            #[cfg(windows)]
            if ui.add_enabled(self.pending.is_none() && self.error.is_none(), egui::Button::new(tr("编辑 RTF"))).clicked() {
                match path.canonicalize() {
                    Ok(key) => {
                        if let Some(session) = self.editors.get(&key) {
                            session.activate();
                        } else {
                            self.editors.insert(key.clone(), editor::Session::open(key, ui.ctx().clone()));
                        }
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            if ui.add_enabled(self.pending.is_none(),egui::Button::new(tr("重载"))).clicked(){self.load(ui.ctx(),path);}
            if !self.text_view {
                ui.label(tr("缩放"));ui.add(egui::DragValue::new(&mut self.zoom).range(0.25..=3.0).speed(0.05).suffix("×"));
                if !self.pages.is_empty() {
                    if ui.add_enabled(self.page > 0, egui::Button::new("‹")).clicked() {
                        self.page -= 1;
                        jump = true;
                    }
                    let mut page = self.page + 1;
                    if ui.add(egui::DragValue::new(&mut page).range(1..=self.pages.len())).changed() {
                        self.page = page - 1;
                        jump = true;
                    }
                    ui.label(format!("/ {}", self.pages.len()));
                    if ui.add_enabled(self.page + 1 < self.pages.len(), egui::Button::new("›")).clicked() {
                        self.page += 1;
                        jump = true;
                    }
                }
            }
            let mode = if self.text_view { "排版预览" } else { "选择文本" };
            if ui.add_enabled(!self.text.is_empty(), egui::Button::new(tr(mode))).clicked() {
                self.text_view = !self.text_view;
            }
            if ui.add_enabled(!self.text.is_empty(),egui::Button::new(tr("复制全文"))).clicked(){ui.ctx().copy_text(self.text.clone());}
        });
        if self.pending.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().error_fg_color,error);}
        if self.text_view {
            egui::ScrollArea::vertical().id_salt(("rtf_text", path)).auto_shrink([false, false]).show(ui, |ui| {
                let mut text = self.text.as_str();
                ui.add(egui::TextEdit::multiline(&mut text).id_salt(path).desired_width(f32::INFINITY));
            });
            return;
        }
        let width=ui.available_width().min(794.0).max(100.0)*self.zoom;
        let stride=width*1123.0/794.0+12.0;
        let mut scroll=egui::ScrollArea::both().id_salt(("rtf_pages",path)).auto_shrink([false,false]);
        if jump { scroll=scroll.vertical_scroll_offset(self.page as f32 * stride); }
        scroll.show_viewport(ui,|ui,viewport|{
            ui.spacing_mut().item_spacing.y=0.0;
            if !self.pages.is_empty() && !jump {
                self.page=((viewport.min.y+1.0)/stride).floor().max(0.0) as usize;
                self.page=self.page.min(self.pages.len()-1);
            }
            for texture in &self.pages{let size=texture.size_vec2();ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(width,width*size.y/size.x)));ui.add_space(12.0);}
            if self.pages.len() > 1 { ui.add_space((viewport.height()-stride).max(0.0)); }
        });
    }
}
#[cfg(not(windows))]
fn render(_path:&Path)->Result<Document,String>{Err(tr("RTF 直读目前仅支持 Windows。").into())}
#[cfg(windows)]
fn render(path:&Path)->Result<Document,String>{native::render(path)}
#[cfg(windows)]
mod native{
    use super::*;
    use std::{ffi::c_void,io::{Read,Cursor}};
    pub(super) const MAX_RTF_BYTES: usize = 16 * 1024 * 1024;
    pub(super) type Handle=*mut c_void;
    #[repr(C)]#[derive(Clone,Copy)]struct Rect{left:i32,top:i32,right:i32,bottom:i32}
    #[repr(C)]struct TextLength{flags:u32,codepage:u32}
    #[repr(C)]struct Range{min:i32,max:i32}
    #[repr(C)]struct FormatRange{dc:Handle,target:Handle,rect:Rect,page:Rect,range:Range}
    const PAGE: Rect = Rect{left:0,top:0,right:11910,bottom:16845};
    const CONTENT: Rect = Rect{left:720,top:720,right:11190,bottom:16125};
    #[repr(C,packed(4))]struct Stream{cookie:usize,error:u32,callback:unsafe extern "system" fn(usize,*mut u8,i32,*mut i32)->u32}
    #[repr(C)]struct BitmapHeader{size:u32,width:i32,height:i32,planes:u16,bits:u16,compression:u32,image_size:u32,xppm:i32,yppm:i32,used:u32,important:u32}
    #[link(name="user32")]unsafe extern "system"{
        fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,x:i32,y:i32,width:i32,height:i32,parent:Handle,menu:Handle,instance:Handle,param:Handle)->Handle;
        fn DestroyWindow(window:Handle)->i32;
        pub(super) fn SendMessageW(window:Handle,message:u32,wparam:usize,lparam:isize)->isize;
        fn GetDC(window:Handle)->Handle;
        fn ReleaseDC(window:Handle,dc:Handle)->i32;
    }
    #[link(name="gdi32")]unsafe extern "system"{
        fn CreateCompatibleDC(dc:Handle)->Handle;
        fn DeleteDC(dc:Handle)->i32;
        fn CreateDIBSection(dc:Handle,header:*const BitmapHeader,usage:u32,bits:*mut *mut c_void,section:Handle,offset:u32)->Handle;
        fn SelectObject(dc:Handle,object:Handle)->Handle;
        fn DeleteObject(object:Handle)->i32;
    }
    struct Window(Handle);impl Drop for Window{fn drop(&mut self){unsafe{SendMessageW(self.0,0x439,0,0);DestroyWindow(self.0);}}}
    struct Surface{dc:Handle,bitmap:Handle,old:Handle,bits:*mut u8}
    impl Drop for Surface{fn drop(&mut self){unsafe{SelectObject(self.dc,self.old);DeleteObject(self.bitmap);DeleteDC(self.dc);}}}
    unsafe extern "system" fn read(cookie:usize,buffer:*mut u8,count:i32,read:*mut i32)->u32{
        if count<=0{unsafe{*read=0;}return u32::from(count<0);}
        let cursor=unsafe{&mut *(cookie as *mut Cursor<&[u8]>)};
        let output=unsafe{std::slice::from_raw_parts_mut(buffer,count as usize)};
        match cursor.read(output){Ok(n)=>{unsafe{*read=n as i32;}0},Err(_)=>1}
    }
    pub(super) fn read_rtf(path: &Path) -> Result<Vec<u8>, String> {
        let file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut bytes=Vec::new();file.take(MAX_RTF_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len()>MAX_RTF_BYTES{return Err(tr("RTF 文件超过 16 MiB，无法预览。").into());}
        if !bytes.starts_with(b"{\\rtf"){return Err(tr("RTF 文件头无效。").into());}
        Ok(bytes)
    }

    pub(super) fn stream_in(window: Handle, bytes: &[u8], selection: bool) -> Result<(), String> {
        let mut cursor = Cursor::new(bytes);
        let mut stream = Stream { cookie: &mut cursor as *mut _ as usize, error: 0, callback: read };
        unsafe { SendMessageW(window, 0x449, 2 | if selection { 0x8000 } else { 0 }, &mut stream as *mut _ as isize); }
        let error = stream.error;
        if error != 0 { return Err(format!("RichEdit: {error}")); }
        Ok(())
    }

    pub(super) fn stream_out(window: Handle) -> Result<Vec<u8>, String> {
        unsafe extern "system" fn write(cookie: usize, buffer: *mut u8, count: i32, written: *mut i32) -> u32 {
            let bytes = unsafe { &mut *(cookie as *mut Vec<u8>) };
            unsafe { *written = 0; }
            if count < 0 || bytes.len() + count as usize > MAX_RTF_BYTES { return 1; }
            if count == 0 { return 0; }
            bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer, count as usize) });
            unsafe { *written = count; }
            0
        }
        let mut bytes = Vec::<u8>::new();
        let mut stream = Stream { cookie: &mut bytes as *mut _ as usize, error: 0, callback: write };
        unsafe { SendMessageW(window, 0x44a, 2, &mut stream as *mut _ as isize); }
        if stream.error != 0 || !bytes.starts_with(b"{\\rtf") {
            return Err(tr("RTF 保存失败").into());
        }
        Ok(bytes)
    }

    pub(super) fn page_starts(window: Handle) -> Option<Vec<i32>> {
        unsafe {
            let options = TextLength { flags: 8, codepage: 1200 };
            let length = SendMessageW(window, 0x45f, &options as *const _ as usize, 0) as i32;
            let mut starts = vec![0];
            if length <= 0 { return Some(starts); }
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() { return None; }
            let dc = CreateCompatibleDC(screen);
            ReleaseDC(std::ptr::null_mut(), screen);
            if dc.is_null() { return None; }
            let result = loop {
                let start = *starts.last().unwrap();
                let mut format = FormatRange { dc, target: dc, rect: CONTENT, page: PAGE, range: Range { min: start, max: -1 } };
                let next = SendMessageW(window, 0x439, 0, &mut format as *mut _ as isize) as i32;
                if next >= length { break Some(starts); }
                if next <= start || starts.len() >= 64 { break None; }
                starts.push(next);
            };
            SendMessageW(window, 0x439, 0, 0);
            DeleteDC(dc);
            result
        }
    }

    pub fn render(path:&Path)->Result<Document,String>{unsafe{
        let bytes = read_rtf(path)?;
        let _library=libloading::Library::new("Msftedit.dll").map_err(|e|e.to_string())?;
        let class:Vec<u16>="RICHEDIT50W\0".encode_utf16().collect();
        let window=Window(CreateWindowExW(0,class.as_ptr(),std::ptr::null(),0x80000000|4|0x800,0,0,794,1123,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut()));
        if window.0.is_null(){return Err(std::io::Error::last_os_error().to_string());}
        // Keep embedded objects inactive; RichEdit receives no OLE activation callback.
        SendMessageW(window.0,0x435,0,8*1024*1024);
        stream_in(window.0, &bytes, false)?;
        let length=SendMessageW(window.0,0x000e,0,0).max(0) as usize;
        if length>8*1024*1024{return Err(tr("RTF 内容超过预览限制。").into());}
        let length_options=TextLength{flags:8,codepage:1200};
        let rich_length=SendMessageW(window.0,0x45f,&length_options as *const _ as usize,0).max(0) as usize;
        let mut text=vec![0u16;length+1];let count=SendMessageW(window.0,0x000d,text.len(),text.as_mut_ptr() as isize).max(0) as usize;
        let text=String::from_utf16_lossy(&text[..count.min(length)]);
        let screen=GetDC(std::ptr::null_mut());if screen.is_null(){return Err("GetDC failed".into());}
        let dc=CreateCompatibleDC(screen);ReleaseDC(std::ptr::null_mut(),screen);if dc.is_null(){return Err("CreateCompatibleDC failed".into());}
        let header=BitmapHeader{size:40,width:794,height:-1123,planes:1,bits:32,compression:0,image_size:0,xppm:3780,yppm:3780,used:0,important:0};let mut bits=std::ptr::null_mut();
        let bitmap=CreateDIBSection(dc,&header,0,&mut bits,std::ptr::null_mut(),0);
        if bitmap.is_null(){DeleteDC(dc);return Err("CreateDIBSection failed".into());}
        let old=SelectObject(dc,bitmap);let surface=Surface{dc,bitmap,old,bits:bits.cast()};
        // EM_FORMATRANGE takes twips and converts them to the target DC pixels.
        let mut pages=Vec::new();let mut start=0i32;
        loop{
            if pages.len()>=64{return Err(tr("RTF 页数超过 64 页预览限制。").into());}
            std::ptr::write_bytes(surface.bits,255,794*1123*4);
            let mut format=FormatRange{dc,target:dc,rect:CONTENT,page:PAGE,range:Range{min:start,max:-1}};
            let next=SendMessageW(window.0,0x439,1,&mut format as *mut _ as isize) as i32;
            let data=std::slice::from_raw_parts(surface.bits,794*1123*4);
            let pixels=data.chunks_exact(4).map(|p|egui::Color32::from_rgb(p[2],p[1],p[0])).collect();pages.push(egui::ColorImage{size:[794,1123],pixels});
            if next as usize>=rich_length||rich_length==0{break;}if next<=start{return Err(tr("RTF 排版未能继续，文件可能包含不支持的内容。").into());}
            start=next;
        }
        SendMessageW(window.0,0x439,0,0);Ok(Document{pages,text})
    }}
}
