//! CPU scene composition runs off the UI thread. Only a bounded canvas image is uploaded.
use super::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_CANVAS: f32 = 1024.0;
const VIEW_IDLE: Duration = Duration::from_millis(500);
const ZOOM_REFRESH_RATIO: f64 = 1.5;
const TEXTURE_BUDGET: usize = super::PREVIEW_TEXTURE_BUDGET;

#[derive(Clone, PartialEq)]
struct View {
    size: [usize; 2],
    zoom: f64,
    pan: [f64; 2],
    focus: Vec<String>,
}
impl Default for View {
    fn default() -> Self { Self { size: [1, 1], zoom: 1.0, pan: [0.0; 2], focus: Vec::new() } }
}
#[derive(Default)]
struct Status { loaded: usize, total: usize, stage: usize, errors: Vec<String>, cached:usize, load_ms:u128, build_ms:u128, slowest:Vec<(PathBuf,u128)> }
struct Shared {
    particles:Mutex<Vec<crate::particle_preview::Emitter>>,
    gpu_scene: Mutex<Option<Arc<crate::render_system::scene::Scene>>>,
    cancelled: AtomicBool,
    paused: Arc<AtomicBool>,
    revision: AtomicU64,
    view: Mutex<View>,
    status: Mutex<Status>,
    frame: Mutex<Option<Frame>>,
}
impl Shared {
    fn checkpoint(&self) -> bool { super::checkpoint(&self.cancelled, Some(&self.paused)) }
}
struct Frame {
    image: egui::ColorImage,
    hits: Vec<u32>,
    components: Arc<Vec<String>>,
    boxes: Arc<HashMap<String, [f64; 4]>>,
    center: [f64; 2],
    scale: f64,
    pan: [f64; 2],
    revision: u64,
    view: View,
}
struct Job { key: u64, shared: Arc<Shared> }
impl Drop for Job {
    fn drop(&mut self) {
        self.shared.cancelled.store(true, Ordering::Relaxed);
        // Keep large pixel buffers and parsed instances off the UI-thread drop path.
        let shared=self.shared.clone();
        std::thread::spawn(move || {
            shared.frame.lock().unwrap().take();
        });
    }
}
#[derive(Default)]
pub(super) struct World {
    particle_playing:bool,
    particle_last:Option<Instant>,
    particle_elapsed:f32,
    particle_scene:Option<(usize,Arc<crate::render_system::scene::Scene>)>,
    gpu_error: Arc<Mutex<Option<String>>>,
    job: Option<Job>,
    texture: Option<egui::TextureHandle>,
    frame: Option<Frame>,
    view: View,
    frame_key: Option<u64>,
    errors: Vec<String>,
    status_text: String,
    status_stage: usize,
    resource_details: String,
    last_view_change: Option<Instant>,
    textures: Option<Arc<Mutex<TextureCache>>>,
    fps: PreviewFps,
}
impl World {
    pub(super) fn release(&mut self) {
        self.particle_scene=None;self.particle_last=None;self.particle_elapsed=0.0;
        self.job=None;
        if let Some(textures)=self.textures.take() {std::thread::spawn(move || drop(textures));}
        self.texture=None;self.frame_key=None;self.errors.clear();
        if let Some(frame)=self.frame.take() {std::thread::spawn(move || drop(frame));}
        self.view=View::default();self.last_view_change=None;
        self.fps=PreviewFps::default();
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn show(&mut self, ui:&mut egui::Ui, documents:&[Document], flags:&HashMap<String,crate::hierarchy_controls::Flags>, catalog:Option<Arc<HashMap<String,PathBuf>>>, axis:usize, paused:Arc<AtomicBool>, selected:&HashSet<String>, focus:&mut Option<(HashSet<String>,Instant)>, background:&str, rgb:[u8;3],show_guides:bool,expanding:bool,open:&mut bool) -> Option<String> {
        use std::hash::{Hash,Hasher};
        let mut hash=std::collections::hash_map::DefaultHasher::new();
        for doc in documents {doc.text.as_ptr().hash(&mut hash);doc.text.len().hash(&mut hash);}
        // Order-independent flags fingerprint; no per-frame sort of thousands of node IDs.
        let mut flag_hash=0_u64;
        for (id,f) in flags {let mut h=std::collections::hash_map::DefaultHasher::new();id.hash(&mut h);f.visible.hash(&mut h);f.selectable.hash(&mut h);flag_hash^=h.finish();}
        flag_hash.hash(&mut hash);axis.hash(&mut hash);
        catalog.as_ref().map(|c|Arc::as_ptr(c) as usize).hash(&mut hash);
        let key=hash.finish();
        if self.job.as_ref().is_none_or(|job|job.key!=key) {
            self.particle_playing=true;self.particle_elapsed=0.0;self.particle_last=Some(Instant::now());self.particle_scene=None;
            // Cancel the old worker while retaining the displayed frame and camera.
            self.job=None;self.status_stage=0;self.status_text=tr("正在准备预览…").into();
            *self.gpu_error.lock().unwrap()=None;
            let shared=Arc::new(Shared {particles:Mutex::default(),gpu_scene:Mutex::new(None),cancelled:AtomicBool::new(false),paused,revision:AtomicU64::new(1),view:Mutex::new(self.view.clone()),status:Mutex::new(Status::default()),frame:Mutex::new(None)});
            let worker=shared.clone();let docs=documents.to_vec();let flags=flags.clone();let ctx=ui.ctx().clone();
            let textures=self.textures.get_or_insert_with(||Arc::new(Mutex::new(TextureCache::default()))).clone();
            std::thread::spawn(move || run(&docs,&flags,catalog.as_ref(),axis,&worker,&ctx,textures));
            self.job=Some(Job {key,shared});
        }
        let shared=self.job.as_ref().unwrap().shared.clone();
        let has_particles=!shared.particles.lock().unwrap().is_empty();
        let mut particle_controls_changed=false;
        if has_particles {ui.horizontal(|ui|{
            ui.label(tr("粒子播放"));
            if ui.button(tr(if self.particle_playing {"暂停"}else{"播放"})).clicked(){self.particle_playing=!self.particle_playing;particle_controls_changed=true;}
            if ui.button(tr("重播")).clicked(){for emitter in shared.particles.lock().unwrap().iter_mut(){emitter.restart();}self.particle_elapsed=0.0;self.particle_playing=true;particle_controls_changed=true;}
            ui.label(format!("{:.2} s",self.particle_elapsed));
        });}
        if let Ok(status)=shared.status.try_lock() {
            self.status_stage=status.stage;
            self.status_text=match status.stage {
                0=>tr("正在准备预览…").into(),
                1=>format!("{} {}/{}",tr("正在加载预览资源…"),status.loaded,status.total),
                2=>tr("正在合成预览…").into(),
                4=>tr("正在构建预览实例…").into(),
                _=>tr("合成预览已完成").into(),
            };
            self.resource_details=format!("{}: {}/{}\n{}: {}\n{}: {} ms\n{}: {} ms",tr("预览资源统计"),status.loaded,status.total,tr("缓存命中"),status.cached,tr("加载耗时"),status.load_ms,tr("实例构建耗时"),status.build_ms);
            for (path,elapsed) in &status.slowest {self.resource_details.push_str(&format!("\n{}: {} ms",path.file_name().unwrap_or_default().to_string_lossy(),elapsed));}
            if self.frame.is_none() || status.stage==3 { self.errors.clone_from(&status.errors); }
        }
        let complete=!expanding && self.status_stage==3 && self.frame_key==Some(key) && self.frame.as_ref().is_some_and(|frame|frame.revision==shared.revision.load(Ordering::Relaxed));
        let ready=shared.gpu_scene.lock().unwrap().is_some() && crate::render_system::available() && self.gpu_error.lock().unwrap().is_none();
        super::preview_path_row(ui,tr(if ready {"GPU Shader 预览"} else {"普通贴图预览（GPU Shader 未启用）"}));
        let status=if expanding {tr("正在展开 Prefab…")} else if complete {tr("合成预览已完成")} else if self.status_stage==3 {tr("正在合成预览…")} else {&self.status_text};
        if super::preview_status_row(ui,status,complete,&shared.paused,Some(&self.resource_details)) {*open=false;return None;}
        let gpu_warnings=crate::render_system::warnings();
        if !self.errors.is_empty() || !gpu_warnings.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color,format!("{}: {}",tr("部分资源缺失或暂不支持"),self.errors.len()+gpu_warnings.len()));
            ui.collapsing(tr("预览错误详情"),|ui|{
                let mut seen=HashSet::new();
                let details=gpu_warnings.iter().chain(self.errors.iter()).filter(|e|seen.insert(e.as_str())).cloned().collect::<Vec<_>>().join("\n\n");
                if ui.button(tr("复制错误详情")).clicked(){ui.ctx().copy_text(details.clone());}
                egui::ScrollArea::vertical().max_height(160.0).show(ui,|ui|{ui.label(details);});
            });
        }
        if let Some(error)=self.gpu_error.lock().unwrap().as_ref(){ui.colored_label(ui.visuals().warn_fg_color,format!("Shader GPU: {error}"));}
        if let Some(error)=self.errors.iter().find(|e|e.starts_with("Shader GPU:")){ui.colored_label(ui.visuals().warn_fg_color,error);}
        ui.small(tr("XY 正交视图 · 滚轮缩放 · 拖动平移"));
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.0)),egui::Sense::click_and_drag());
        let previous_view=self.view.clone();
        let factor=(MAX_CANVAS/rect.width().max(rect.height())).min(1.0);
        self.view.size=[(rect.width()*factor).round().max(1.0) as usize,(rect.height()*factor).round().max(1.0) as usize];
        if response.dragged() {let delta=response.drag_delta();self.view.pan[0]+=delta.x as f64/rect.width() as f64;self.view.pan[1]+=delta.y as f64/rect.height() as f64;}
        let scroll=if response.hovered() {ui.input(|i|i.smooth_scroll_delta.y)} else {0.0};
        if scroll!=0.0 {
            let pointer=ui.input(|i|i.pointer.hover_pos()).unwrap_or(rect.center());
            zoom_at_pointer(&mut self.view,rect,pointer,scroll);
        }
        if let Some((ids,_))=focus.take() {self.view.focus=ids.into_iter().collect();self.view.pan=[0.0;2];}
        // Every input restarts the full quiet period, even at the zoom limit.
        if response.dragged() || scroll!=0.0 || self.view!=previous_view {self.last_view_change=Some(Instant::now());}
        let idle=!response.dragged() && self.last_view_change.is_none_or(|last|last.elapsed()>=VIEW_IDLE);
        let needs_refresh=self.frame.as_ref().is_none_or(|frame|needs_view_refresh(rect,&self.view,&frame.view));
        {
            let mut view=shared.view.lock().unwrap();
            // Initialize the real canvas promptly; later view changes wait for input to settle.
            let initialize=view.size==[1,1];
            if *view!=self.view && (initialize || (idle && needs_refresh)) {
                *view=self.view.clone();shared.revision.fetch_add(1,Ordering::Relaxed);
            }
        }
        let mut composed_frame_changed=false;
        if !shared.paused.load(Ordering::Relaxed) {
            let incoming=shared.frame.try_lock().ok().and_then(|mut latest|latest.take());
            if let Some(mut frame)=incoming {
                if frame.revision==shared.revision.load(Ordering::Relaxed) {
                    let image=std::mem::replace(&mut frame.image,egui::ColorImage::new([0,0],Color32::TRANSPARENT));
                    if let Some(texture)=&mut self.texture {texture.set(image,egui::TextureOptions::LINEAR);} else {self.texture=Some(ui.ctx().load_texture("scene_composite",image,egui::TextureOptions::LINEAR));}
                    self.frame_key=Some(key);
                    composed_frame_changed=true;
                    if let Some(old)=self.frame.replace(frame) {std::thread::spawn(move || drop(old));}
                } else {std::thread::spawn(move || drop(frame));}
            }
        }
        let painter=ui.painter_at(rect);
        let preview_active=response.hovered() && ui.input(|input|input.focused) && !shared.paused.load(Ordering::Relaxed);
        let particle_now=Instant::now();let particle_dt=self.particle_last.replace(particle_now).map_or(0.0,|last|particle_now.duration_since(last).as_secs_f64().min(0.25));
        let particle_running=has_particles && self.particle_playing && preview_active && self.status_stage==3;
        if particle_running {self.particle_elapsed+=particle_dt as f32;for emitter in shared.particles.lock().unwrap().iter_mut(){emitter.advance(particle_dt);}}
        let mut live_frame=preview_active;
        super::paint_background(&painter,rect,background,rgb);
        let mut picked=None;
        if let (Some(texture),Some(frame))=(&self.texture,&self.frame) {
            let image_rect=cached_image_rect(rect,&self.view,&frame.view);
            let size=texture.size();
            if response.clicked() && self.frame_key==Some(key) && frame.revision==shared.revision.load(Ordering::Relaxed) {
                if let Some(pointer)=response.interact_pointer_pos().filter(|point|image_rect.contains(*point)) {
                    let uv=(pointer-image_rect.min)/image_rect.size();let x=(uv.x*size[0] as f32) as usize;let y=(uv.y*size[1] as f32) as usize;
                    if x<size[0] && y<size[1] {picked=frame.hits.get(y*size[0]+x).and_then(|id|frame.components.get(*id as usize)).cloned();}
                }
            }
            let screen=|x:f64,y:f64|image_rect.min+egui::vec2(((size[0] as f64*0.5+frame.pan[0]*size[0] as f64+(x-frame.center[0])*frame.scale)/size[0] as f64*image_rect.width() as f64) as f32,((size[1] as f64*0.5+frame.pan[1]*size[1] as f64-(y-frame.center[1])*frame.scale)/size[1] as f64*image_rect.height() as f64) as f32);
            if show_guides {
            let origin=screen(0.0,0.0);
            let axis_color=if background!="gray" && background!="custom" {
                Color32::from_rgba_unmultiplied(100,100,100,170)
            } else {Color32::from_rgba_unmultiplied(128,128,128,100)};
            let axis=egui::Stroke::new(1.0_f32,axis_color);
            painter.line_segment([Pos2::new(rect.left(),origin.y),Pos2::new(rect.right(),origin.y)],axis);
            painter.line_segment([Pos2::new(origin.x,rect.top()),Pos2::new(origin.x,rect.bottom())],axis);
            }
            // Guides sit above the background and beneath scene content, like UI previews.
            let gpu_scene=shared.gpu_scene.lock().unwrap().clone();
            if let Some(scene)=gpu_scene.filter(|_|self.frame_key==Some(key) && crate::render_system::available() && self.gpu_error.lock().unwrap().is_none()) {
                let scene=if has_particles {
                    let source_key=Arc::as_ptr(&scene) as usize;
                    if self.particle_scene.as_ref().is_none_or(|(key,_)|*key!=source_key) || particle_running || particle_controls_changed {
                    let particles=shared.particles.lock().unwrap();
                    let emitters:HashMap<_,_>=particles.iter().map(|p|(p.id.as_str(),p)).collect();
                    let mut draws=Vec::new();let mut particle_vertices=0usize;
                    for draw in scene.draws.iter(){if let Some(emitter)=emitters.get(draw.component.as_ref()){let draw=emitter.draw(draw);particle_vertices+=draw.vertices.len();if particle_vertices<=262144{draws.push(draw);}}else{draws.push(draw.clone());}}
                    self.particle_scene=Some((source_key,Arc::new(crate::render_system::scene::Scene{draws:Arc::from(draws),revision:scene.revision})));
                    }
                    self.particle_scene.as_ref().unwrap().1.clone()
                }else{scene};
                let w=size[0] as f64;let h=size[1] as f64;
                let projection=[[(2.0*frame.scale/w) as f32,0.0,0.0,(2.0*frame.pan[0]-2.0*frame.center[0]*frame.scale/w) as f32],
                    [0.0,(2.0*frame.scale/h) as f32,0.0,(-2.0*frame.pan[1]-2.0*frame.center[1]*frame.scale/h) as f32],
                    [0.0,0.0,0.0,0.0],[0.0,0.0,0.0,1.0]];
                let projection=crate::render_system::fixed_viewport_projection(projection,image_rect,rect);
                let first_frame=!crate::render_system::snapshot_ready(&scene);
                live_frame=preview_active || first_frame || composed_frame_changed || particle_running;
                let mut style=std::collections::hash_map::DefaultHasher::new();background.hash(&mut style);rgb.hash(&mut style);show_guides.hash(&mut style);
                let camera_input=self.view.pan!=previous_view.pan || self.view.zoom!=previous_view.zoom || self.view.focus!=previous_view.focus;
                painter.add(crate::render_system::callback_timed(rect,scene,projection,self.gpu_error.clone(),preview_active || camera_input || composed_frame_changed || particle_controls_changed,style.finish(),has_particles.then_some(self.particle_elapsed)));
                if live_frame {ui.ctx().request_repaint_after(Duration::from_millis(16));}
            } else {
                painter.image(texture.id(),image_rect,egui::Rect::from_min_max(Pos2::ZERO,Pos2::new(1.0,1.0)),Color32::WHITE);
            }
            for (_,area) in frame.boxes.iter().filter(|(id,_)|picked.as_ref().map_or_else(||selected.contains(*id),|picked|picked==*id)) {

                painter.rect_stroke(egui::Rect::from_two_pos(screen(area[0],area[1]),screen(area[2],area[3])),0.0,egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235)),egui::StrokeKind::Inside);
            }
        }
        super::paint_zoom_badge(&painter,rect,self.view.zoom);
        if self.frame.is_some() {
            if live_frame {self.fps.sample(ui.input(|i|i.time));}else{self.fps.start=None;self.fps.frames=0;}
            let text=if live_frame {format!("FPS: {:.0}",self.fps.value)}else{"FPS: —".into()};
            let pos=rect.left_top()+egui::vec2(8.0,8.0);
            let font=egui::FontId::proportional(16.0);
            let galley=painter.layout_no_wrap(text,font,Color32::WHITE);
            let badge=egui::Rect::from_min_size(pos,galley.size()+egui::vec2(12.0,6.0));
            painter.rect_filled(badge,4.0,Color32::from_black_alpha(150));
            painter.galley(pos+egui::vec2(6.0,3.0),galley,Color32::WHITE);
        }
        if self.status_stage!=3 || !idle {ui.ctx().request_repaint_after(Duration::from_millis(100));}
        picked
    }
    pub(super) fn fit(&mut self) {self.view.zoom=1.0;self.view.pan=[0.0;2];self.view.focus.clear();}
}

#[derive(Default)]
struct PreviewFps { start: Option<f64>, frames: u32, value: f64 }
impl PreviewFps {
    fn sample(&mut self, now:f64) {
        let Some(start)=self.start else {self.start=Some(now);return;};
        self.frames+=1;
        let elapsed=now-start;
        if elapsed>=0.5 {self.value=self.frames as f64/elapsed;self.frames=0;self.start=Some(now);}
    }
}

fn zoom_at_pointer(view:&mut View,rect:egui::Rect,pointer:Pos2,scroll:f32) {
    let next=(view.zoom*(scroll as f64*0.002).exp()).clamp(0.05,50.0);
    let ratio=next/view.zoom;
    let anchor=[(pointer.x-rect.center().x) as f64/rect.width() as f64,
        (pointer.y-rect.center().y) as f64/rect.height() as f64];
    for (pan,anchor) in view.pan.iter_mut().zip(anchor) {*pan=anchor+(*pan-anchor)*ratio;}
    view.zoom=next;
}

// Apply the desired camera to the cached viewport without composing pixels again.
fn cached_image_rect(rect:egui::Rect,current:&View,cached:&View) -> egui::Rect {
    let size=egui::vec2(cached.size[0] as f32,cached.size[1] as f32);
    let fit=(rect.width()/size.x).min(rect.height()/size.y);
    let ratio=(current.zoom/cached.zoom) as f32;
    let offset=egui::vec2(
        (current.pan[0]-cached.pan[0]*ratio as f64) as f32*rect.width(),
        (current.pan[1]-cached.pan[1]*ratio as f64) as f32*rect.height());
    egui::Rect::from_center_size(rect.center()+offset,size*fit*ratio)
}
fn needs_view_refresh(rect:egui::Rect,current:&View,cached:&View) -> bool {
    if current.size!=cached.size || current.focus!=cached.focus {return true;}
    let ratio=current.zoom/cached.zoom;
    let image_rect=cached_image_rect(rect,current,cached);
    ratio>=ZOOM_REFRESH_RATIO || ratio<=1.0/ZOOM_REFRESH_RATIO ||
        !image_rect.expand(1.0).contains_rect(rect)
}

struct CpuTexture {size:[usize;2], pixels:Vec<Color32>, meta:String, regions:Mutex<HashMap<String,Option<Region>>>}
#[derive(Clone,PartialEq)]
struct Stamp { image:(Option<SystemTime>,u64),meta:(Option<SystemTime>,u64) }
fn stamp(path:&std::path::Path) -> Result<Stamp,String> {
    let image=std::fs::metadata(path).map_err(|e|e.to_string())?;
    let meta=std::fs::metadata(format!("{}.meta",path.display())).ok();
    Ok(Stamp{image:(image.modified().ok(),image.len()),meta:meta.map_or((None,0),|m|(m.modified().ok(),m.len()))})
}
struct CachedTexture {stamp:Stamp,texture:Arc<CpuTexture>,last:u64}
#[derive(Default)]
struct TextureCache {entries:HashMap<PathBuf,CachedTexture>,bytes:usize,clock:u64}
impl TextureCache {
    fn get(&mut self,path:&std::path::Path,stamp:&Stamp) -> Option<Arc<CpuTexture>> {
        self.clock+=1;
        if self.entries.get(path).is_some_and(|entry|entry.stamp!=*stamp) {if let Some(old)=self.entries.remove(path) {self.bytes-=texture_bytes(&old.texture);}}
        let entry=self.entries.get_mut(path)?;entry.last=self.clock;Some(entry.texture.clone())
    }
    fn insert(&mut self,path:PathBuf,stamp:Stamp,texture:Arc<CpuTexture>) -> Arc<CpuTexture> {
        if let Some(existing)=self.get(&path,&stamp) {return existing;}
        let size=texture_bytes(&texture);
        while self.bytes+size>TEXTURE_BUDGET {
            let oldest=self.entries.iter().filter(|(_,entry)|Arc::strong_count(&entry.texture)==1).min_by_key(|(_,entry)|entry.last).map(|(path,_)|path.clone());
            let Some(oldest)=oldest else {return texture;};
            self.bytes-=texture_bytes(&self.entries.remove(&oldest).unwrap().texture);
        }
        self.bytes+=size;self.entries.insert(path,CachedTexture{stamp,texture:texture.clone(),last:self.clock});texture
    }
}
fn texture_bytes(texture:&CpuTexture) -> usize {texture.pixels.len()*4+texture.meta.len()}
fn reserve_bytes(reserved:&std::sync::atomic::AtomicUsize,size:usize) -> bool {
    reserved.try_update(Ordering::Relaxed,Ordering::Relaxed,|used|used.checked_add(size).filter(|total|*total<=TEXTURE_BUDGET)).is_ok()
}
fn load_cached(path:&std::path::Path,reserved:&std::sync::atomic::AtomicUsize,cache:&Mutex<TextureCache>,shared:&Shared) -> Result<(Arc<CpuTexture>,bool),String> {
    let version=stamp(path)?;
    if let Some(texture)=cache.lock().unwrap().get(path,&version) {
        if !reserve_bytes(reserved,texture_bytes(&texture)) {return Err("Preview texture cache exceeds 2 GiB".into());}
        return Ok((texture,true));
    }
    let (width,height)=image::ImageReader::open(path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())?;
    if width>8192 || height>8192 || version.meta.1>8*1024*1024 {return Err("Preview texture or Sprite metadata exceeds size limit".into());}
    let pixels=width as usize*height as usize*4;let bytes=pixels+version.meta.1 as usize;
    if !reserve_bytes(reserved,bytes) {return Err("Preview texture cache exceeds 2 GiB".into());}
    match load(path,pixels,shared) {
        Ok(texture)=> {
            let texture=Arc::new(texture);
            match stamp(path) {
                Ok(current) if current==version=>{},
                _=>{reserved.fetch_sub(bytes,Ordering::Relaxed);return Err("Preview resource changed during loading; retry preview".into());},
            }
            Ok((cache.lock().unwrap().insert(path.to_owned(),version,texture),false))
        },
        Err(error)=>{reserved.fetch_sub(bytes,Ordering::Relaxed);Err(error)},
    }
}
struct Instance {points:[[f64;2];4],uv:[[f64;2];4],tint:Color32,texture:usize,component:u32,selectable:bool}
fn load(path:&std::path::Path,remaining:usize,shared:&Shared) -> Result<CpuTexture,String> {
    let mut reader=image::ImageReader::open(path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
    let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);// Decoder scratch buffers (especially PNG inflation) can exceed a small tile's pixels.
    // Keep the decoded-output reservation separate from the decoder allocation limit.
    limits.max_alloc=Some(remaining.saturating_add(8*1024*1024).min(TEXTURE_BUDGET) as u64);reader.limits(limits);
    let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
    let size=[image.width() as usize,image.height() as usize];
    if size[0]*size[1]*4>remaining {return Err("Preview texture cache exceeds 2 GiB".into());}
    let meta_path=PathBuf::from(format!("{}.meta",path.display()));
    if std::fs::metadata(&meta_path).is_ok_and(|m|m.len()>8*1024*1024) {return Err("Sprite metadata exceeds 8 MiB".into());}
    let meta=std::fs::read_to_string(meta_path).unwrap_or_default();
    let mut pixels=Vec::with_capacity(size[0]*size[1]);
    for (index,pixel) in image.pixels().enumerate() {
        if index%16384==0 && !shared.checkpoint() {return Err("Preview cancelled".into());}
        pixels.push(crate::asset_viewer::preview_color(pixel.0));
    }
    let regions=Mutex::new(super::region_index(&meta,size));
    Ok(CpuTexture {size,pixels,meta,regions})
}
fn include(area:&mut [f64;4],points:[[f64;2];4]) {
    for p in points {area[0]=area[0].min(p[0]);area[1]=area[1].min(p[1]);area[2]=area[2].max(p[0]);area[3]=area[3].max(p[1]);}
}
fn run(docs:&[Document],flags:&HashMap<String,crate::hierarchy_controls::Flags>,catalog:Option<&Arc<HashMap<String,PathBuf>>>,axis:usize,shared:&Shared,ctx:&egui::Context,cache:Arc<Mutex<TextureCache>>) {
    if !shared.checkpoint() {return;}
    let mut prepared=super::prepare(docs,flags,catalog,&shared.cancelled,axis,Some(&shared.paused));
    if prepared.particles.len()*crate::particle_preview::MAX_PARTICLES>65536 {prepared.issues.push("Particle preview limits total visible particles to 65,536 per frame".into());}
    *shared.particles.lock().unwrap()=std::mem::take(&mut prepared.particles);
    if !shared.checkpoint() {return;}
    let mut paths=Vec::new();let mut indices=HashMap::new();
    for (index,sprite) in prepared.sprites.iter().enumerate() {
        if index%256==0 && !shared.checkpoint() {return;}
        if !indices.contains_key(&sprite.path) {indices.insert(sprite.path.clone(),paths.len());paths.push(sprite.path.clone());}
    }
    {let mut status=shared.status.lock().unwrap();status.total=paths.len();status.stage=1;status.errors=prepared.issues.clone();}
    let loading_started=Instant::now();
    let baked:HashMap<_,_>=prepared.sprites.iter().filter_map(|s|Some((s.path.clone(),s.baked.clone()?))).collect();
    let mut textures=vec![None;paths.len()];
    let next=std::sync::atomic::AtomicUsize::new(0);let reserved=std::sync::atomic::AtomicUsize::new(0);
    let (sender,receiver)=std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..2.min(paths.len()) {
            let sender=sender.clone();let paths=&paths;let next=&next;let reserved=&reserved;let cache=&cache;let baked=&baked;
            scope.spawn(move || loop {
                if !shared.checkpoint() {return;}
                let index=next.fetch_add(1,Ordering::Relaxed);let Some(path)=paths.get(index) else {return;};
                let started=Instant::now();let result=if let Some(baked)=baked.get(path) {
                    if reserve_bytes(reserved,baked.image.pixels.len()*4) {Ok((Arc::new(CpuTexture{size:baked.image.size,pixels:baked.image.pixels.clone(),meta:String::new(),regions:Mutex::default()}),false))} else {Err("Preview texture cache exceeds 2 GiB".into())}
                } else {load_cached(path,reserved,cache,shared)};
                let elapsed=started.elapsed();
                if sender.send((index,result,elapsed)).is_err() {return;}
            });
        }
        drop(sender);
        for (index,result,elapsed) in receiver {
            if shared.cancelled.load(Ordering::Relaxed) {break;}
            let mut status=shared.status.lock().unwrap();
            match result {
                Ok((texture,hit))=>{textures[index]=Some(texture);if hit {status.cached+=1;}},
                Err(error)=>status.errors.push(format!("{}: {error}",paths[index].display())),
            }
            status.loaded+=1;status.load_ms=loading_started.elapsed().as_millis();
            status.slowest.push((paths[index].clone(),elapsed.as_millis()));
            status.slowest.sort_by_key(|(_,ms)|std::cmp::Reverse(*ms));status.slowest.truncate(5);
            ctx.request_repaint();
        }
    });
    if !shared.checkpoint() {return;}
    {let mut status=shared.status.lock().unwrap();status.load_ms=loading_started.elapsed().as_millis();status.stage=4;}
    ctx.request_repaint();
    let building_started=Instant::now();
    let mut instances=Vec::with_capacity(prepared.sprites.len());
    let mut regions:HashMap<(usize,String),Option<Region>>=HashMap::new();
    let mut components=Vec::new();let mut component_ids=HashMap::new();let mut boxes=HashMap::new();let mut bounds=[0.0_f64;4];
    for (index,sprite) in prepared.sprites.iter().enumerate() {
        if index%256==0 && !shared.checkpoint() {return;}
        let texture_index=indices[&sprite.path];let Some(texture)=&textures[texture_index] else {continue;};
        let r=regions.entry((texture_index,sprite.id.clone())).or_insert_with(||sprite.baked.as_ref().map(|b|b.region).or_else(||*texture.regions.lock().unwrap().entry(sprite.id.clone()).or_insert_with(||super::region(&texture.meta,&sprite.id,texture.size))));
        let Some(r)=r else {continue;};
        let [x,y,w,h]=r.rect;let corners=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
        let points=corners.map(|[a,b]|sprite.matrix.point((a-r.pivot[0])*w/r.ppu*if sprite.flip[0]{-1.0}else{1.0},(b-r.pivot[1])*h/r.ppu*if sprite.flip[1]{-1.0}else{1.0}));
        if points.iter().flatten().any(|v|!v.is_finite()) {continue;}
        include(&mut bounds,points);
        let area=boxes.entry(sprite.component.clone()).or_insert([f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY]);include(area,points);
        let component=*component_ids.entry(sprite.component.clone()).or_insert_with(||{let id=components.len() as u32;components.push(sprite.component.clone());id});
        let uv=corners.map(|[a,b]|[(x+a*w)/texture.size[0] as f64,1.0-(y+b*h)/texture.size[1] as f64]);
        instances.push(Instance {points,uv,tint:sprite.tint,texture:texture_index,component,selectable:sprite.selectable});
    }
    for ((texture,id),region) in regions {if region.is_none() {shared.status.lock().unwrap().errors.push(format!("Sprite fileID {id}, texture {texture}: {}",tr("无法定位 Sprite 切片，或切片数据无效")));}}
    shared.status.lock().unwrap().build_ms=building_started.elapsed().as_millis();
    match gpu_scene(&prepared.sprites,&instances,&textures,&paths,&components,docs,catalog,shared) {
        Ok(scene)=>*shared.gpu_scene.lock().unwrap()=scene,
        Err(error)=>shared.status.lock().unwrap().errors.push(format!("Shader GPU: {error}")),
    }
    drop(prepared);drop(indices);drop(component_ids);
    let components=Arc::new(components);let boxes=Arc::new(boxes);let mut rendered_revision=0;
    loop {
        if !shared.checkpoint() {return;}
        let revision=shared.revision.load(Ordering::Relaxed);
        if revision==rendered_revision {std::thread::sleep(Duration::from_millis(50));continue;}
        shared.status.lock().unwrap().stage=2;ctx.request_repaint();
        let view=shared.view.lock().unwrap().clone();
        if let Some(frame)=compose(&instances,&textures,&view,bounds,components.clone(),boxes.clone(),shared,revision) {
            if revision==shared.revision.load(Ordering::Relaxed) && !shared.cancelled.load(Ordering::Relaxed) {
                *shared.frame.lock().unwrap()=Some(frame);rendered_revision=revision;shared.status.lock().unwrap().stage=3;ctx.request_repaint();
            }
        }
    }
}
fn over(source:Color32,tint:Color32,dest:Color32) -> Color32 {
    crate::render_system::source_over(source,tint,dest)
}

#[allow(clippy::too_many_arguments)]
fn gpu_scene(sprites:&[Sprite],instances:&[Instance],textures:&[Option<Arc<CpuTexture>>],paths:&[PathBuf],components:&[String],docs:&[Document],catalog:Option<&Arc<HashMap<String,PathBuf>>>,shared:&Shared)->Result<Option<Arc<crate::render_system::scene::Scene>>,String> {
    use crate::render_system::scene::{self as rs, BlendFactor as B, Uniform as U};
    use std::collections::BTreeMap;
    fn nested_field<'a>(text:&'a str,key:&str)->Option<&'a str>{let prefix=format!("{key}:");text.lines().find_map(|line|line.trim_start().strip_prefix(&prefix).map(str::trim))}
    let Some(catalog)=catalog else {return Ok(None)};
    let docs:HashMap<_,_>=docs.iter().map(|d|(d.file_id.as_str(),d)).collect();
    let identity=[[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]];
    let ordinary=Arc::new(rs::Material { program:Arc::new(rs::Program { pipeline:rs::UnityPipeline::BuiltIn,revision:1,
        vertex:Arc::from("#version 330 core\nlayout(location=0) in vec3 a_position;layout(location=1) in vec2 a_uv;layout(location=2) in vec4 a_color;uniform mat4 unity_MatrixVP;uniform mat4 unity_ObjectToWorld;out vec2 v_uv;out vec4 v_color;void main(){gl_Position=unity_MatrixVP*unity_ObjectToWorld*vec4(a_position,1);v_uv=a_uv;v_color=a_color;}"),
        fragment:Arc::from("#version 330 core\nin vec2 v_uv;in vec4 v_color;uniform sampler2D _MainTex;out vec4 output_color;void main(){vec4 c=texture(_MainTex,v_uv)*v_color;output_color=vec4(c.rgb*c.a,c.a);}") }),
        state:rs::RenderState{source:B::One,destination:B::OneMinusSourceAlpha,color_mask:[true;4],depth_write:false,cull_back:false},uniforms:BTreeMap::new(),textures:BTreeMap::new() });
    let read=|path:&PathBuf|->Result<String,String>{
        if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>2*1024*1024 {return Err(format!("Shader/material exceeds 2 MiB: {}",path.display()));}
        std::fs::read_to_string(path).map_err(|e|e.to_string())
    };
    let guid=regex::Regex::new(r"guid:\s*([0-9a-fA-F]{32})").unwrap();
    let values=regex::Regex::new(r"(?m)^\s*-\s*(_\w+):\s*([-+0-9.eE]+)\s*$").unwrap();
    let defaults=regex::Regex::new(r#"(?m)^\s*(?:\[\w+\]\s*)?(_\w+)\s*\([^\n]*\)\s*=\s*([-+0-9.eE]+)"#).unwrap();
    let mut materials:HashMap<(PathBuf,bool,bool,bool),Result<Arc<rs::Material>,String>>=HashMap::new();
    let mut material_sources:HashMap<PathBuf,Arc<rs::Texture>>=HashMap::new();
    let mut material_source_bytes=0usize;
    let mut custom=HashMap::new();
    let tile_renderers:HashMap<_,_>=docs.values().filter(|d|d.type_name=="TilemapRenderer").filter_map(|d|Some((d.game_object.as_deref()?,*d))).collect();
    let mesh_renderers:HashMap<_,_>=docs.values().filter(|d|d.type_name=="MeshRenderer").filter_map(|d|Some((d.game_object.as_deref()?,*d))).collect();
    let mut visited_components=HashSet::new();
    for sprite in sprites {
        if !shared.checkpoint() {return Err("Preview cancelled".into());}
        if !visited_components.insert(sprite.component.as_str()) {continue;}
        let Some(component)=docs.get(sprite.component.as_str()) else {continue;};
        let spine=component.type_name=="MonoBehaviour" && field(&component.text,"skeletonDataAsset").is_some();
        if spine && sprite.baked.as_ref().is_none_or(|b|b.gpu.is_none()) {shared.status.lock().unwrap().errors.push(format!("Spine {}: GPU geometry unavailable (atlas material mapping, special blend or two-color tint); baked fallback",component.file_id));continue;}
        let doc=if component.type_name=="Tilemap" {let Some(renderer)=component.game_object.as_deref().and_then(|go|tile_renderers.get(go)) else {continue;};*renderer}else if spine {let Some(renderer)=component.game_object.as_deref().and_then(|go|mesh_renderers.get(go)) else {continue;};*renderer}else{*component};
        if !matches!(doc.type_name.as_str(),"SpriteRenderer"|"MeshRenderer"|"TilemapRenderer"|"ParticleSystemRenderer") {continue;}
        let sprite_texture=spine || matches!(doc.type_name.as_str(),"SpriteRenderer"|"TilemapRenderer");
        let Some(tail)=doc.text.split_once("  m_Materials:").map(|(_,v)|v) else {continue;};
        let mut slots=Vec::new();
        let limit=if doc.type_name=="MeshRenderer" {64}else{1};
        let lines:Vec<_>=tail.lines().skip_while(|line|line.trim().is_empty()).take_while(|line|line.trim_start().starts_with('-')).take(65).collect();
        if doc.type_name=="MeshRenderer" && lines.len()>64 {shared.status.lock().unwrap().errors.push(format!("Mesh {}: material slot limit 64 exceeded",doc.file_id));}
        for line in lines.into_iter().take(limit) {
        let Some(path)=guid.captures(line).and_then(|c|catalog.get(&c[1].to_lowercase())) else {
            if matches!(doc.type_name.as_str(),"MeshRenderer"|"ParticleSystemRenderer") {shared.status.lock().unwrap().errors.push(format!("Renderer {}: material slot {} reference unavailable",doc.file_id,slots.len()));}
            slots.push(None);continue;
        };
        let material=materials.entry((path.clone(),sprite_texture,spine,doc.type_name=="ParticleSystemRenderer")).or_insert_with(||{
            let text=read(path)?;
            let reference=field(&text,"m_Shader").ok_or("Missing material shader")?;
            let shader_path=guid.captures(reference).and_then(|c|catalog.get(&c[1].to_lowercase())).ok_or("Shader GUID not found")?;
            let shader=read(shader_path)?;
            let (program,state)=if spine {crate::render_system::shader::translate_spine_material_file(shader_path,&text)?}else if doc.type_name=="ParticleSystemRenderer" {crate::render_system::shader::translate_particle_material_file(shader_path,&text)?}else{crate::render_system::shader::translate_material_file(shader_path,&text)?};
            let mut uniforms=BTreeMap::new();
            uniforms.insert("_MainTex_ST".into(),U::Vector([1.0,1.0,0.0,0.0]));
            uniforms.insert("_LightIntensity".into(),U::Float(0.0));
            uniforms.insert("_ToneMappingMin".into(),U::Float(0.0));uniforms.insert("_ToneMappingMax".into(),U::Float(1.0));
            let vector_defaults=regex::Regex::new(r"(?m)^\s*(?:\[\w+\]\s*)?(_\w+)\s*\([^\n]*\)\s*=\s*\(([^)]+)\)").unwrap();
            for c in vector_defaults.captures_iter(&shader) {
                let parsed=c[2].split(',').map(|v|v.trim().parse::<f32>().map_err(|_|"Invalid Shader vector default")).collect::<Result<Vec<_>,_>>()?;
                if !(2..=4).contains(&parsed.len()){return Err("Invalid Shader vector component count".into());}
                uniforms.insert(c[1].to_owned(),U::Vector(std::array::from_fn(|i|parsed.get(i).copied().unwrap_or(0.0))));
            }
            for c in defaults.captures_iter(&shader).chain(values.captures_iter(&text)) {
                if let Ok(value)=c[2].parse::<f32>() {uniforms.insert(c[1].to_owned(),U::Float(value));}
            }
            let vectors=regex::Regex::new(r"(?m)^\s*-\s*(_\w+):\s*\{r:\s*([^,]+),\s*g:\s*([^,]+),\s*b:\s*([^,]+),\s*a:\s*([^}]+)\}").unwrap();
            for c in vectors.captures_iter(&text) {let values=(1..=4).map(|i|c[i+1].trim().parse::<f32>().map_err(|_|"Invalid material vector")).collect::<Result<Vec<_>,_>>()?;uniforms.insert(c[1].to_owned(),U::Vector(std::array::from_fn(|i|values[i])));}
            let mut bound_textures=BTreeMap::new();
            let texture_blocks=regex::Regex::new(r"(?m)^\s*-\s*(_\w+):\s*$").unwrap();
            let starts:Vec<_>=texture_blocks.captures_iter(&text).map(|c|(c[1].to_owned(),c.get(0).unwrap().end())).collect();
            for (i,(name,start)) in starts.iter().enumerate() {
                let block=&text[*start..starts.get(i+1).map_or(text.len(),|(_,p)|*p)];
                let scale=nested_field(block,"m_Scale").and_then(|v|crate::inline_vector(v,&["x","y"])).unwrap_or(vec![1.0;2]);
                let offset=nested_field(block,"m_Offset").and_then(|v|crate::inline_vector(v,&["x","y"])).unwrap_or(vec![0.0;2]);
                uniforms.insert(format!("{name}_ST"),U::Vector([scale[0] as f32,scale[1] as f32,offset[0] as f32,offset[1] as f32]));
                if name=="_MainTex" && sprite_texture {continue;}
                let Some(reference)=nested_field(block,"m_Texture") else{continue;};
                let Some(texture_path)=guid.captures(reference).and_then(|c|catalog.get(&c[1].to_lowercase())) else{continue;};
                if let Some(texture)=material_sources.get(texture_path) {bound_textures.insert(name.clone(),texture.clone());continue;}
                let mut reader=image::ImageReader::open(texture_path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
                let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
                let mut image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
                material_source_bytes+=image.len();if material_source_bytes>64*1024*1024 {return Err("Shared material textures exceed 64 MiB".into());}
                image::imageops::flip_vertical_in_place(&mut image);
                let meta=std::fs::read_to_string(format!("{}.meta",texture_path.display())).unwrap_or_default();
                let fallback_wrap=nested_field(&meta,"wrapMode").and_then(|v|v.parse::<i32>().ok()).unwrap_or(0);
                let wraps=["wrapU","wrapV"].map(|axis|nested_field(&meta,axis).and_then(|v|v.parse::<i32>().ok()).filter(|v|*v>=0).unwrap_or(fallback_wrap));
                if wraps.iter().any(|v|!matches!(v,0|1)){return Err("Mirror texture wrapping is not implemented".into());}
                let repeat=wraps.map(|v|v==0);
                let texture=rs::Texture{size:[image.width(),image.height()],rgba:Arc::from(image.into_raw()),revision:1,repeat};
                let texture=Arc::new(texture);material_sources.insert(texture_path.clone(),texture.clone());bound_textures.insert(name.clone(),texture);
            }
            for c in regex::Regex::new(r"uniform\s+float\s+(_\w+)\s*;").unwrap().captures_iter(&program.fragment) {
                if !uniforms.contains_key(&c[1]) && (crate::render_system::shader::scalar_is_referenced(&program.fragment,&c[1]) || crate::render_system::shader::scalar_is_referenced(&program.vertex,&c[1])) {
                    uniforms.insert(c[1].to_owned(),U::Float(1.0));
                    shared.status.lock().unwrap().errors.push(format!("{}: unresolved scalar {} defaults to 1 in preview",path.display(),&c[1]));
                }
            }
            if let Some(tail)=text.split_once("- _MainTex:").map(|(_,v)|v) {
                let scale=nested_field(tail,"m_Scale").and_then(|v|crate::inline_vector(v,&["x","y"])).unwrap_or(vec![1.0;2]);let offset=nested_field(tail,"m_Offset").and_then(|v|crate::inline_vector(v,&["x","y"])).unwrap_or(vec![0.0;2]);
                uniforms.insert("_MainTex_ST".into(),U::Vector([scale[0] as f32,scale[1] as f32,offset[0] as f32,offset[1] as f32]));
            }
            for c in regex::Regex::new(r"uniform\s+sampler2D\s+(_\w+)\s*;").unwrap().captures_iter(&program.fragment) {
                if &c[1]=="_MainTex" && sprite_texture {continue;}
                if bound_textures.contains_key(&c[1]) {continue;}
                let texture_defaults=regex::Regex::new(r#"(_\w+)\s*\("[^"\n]*",\s*2D\s*\)\s*=\s*"(\w+)""#).unwrap();
                let default=texture_defaults.captures_iter(&shader).find(|v|v[1]==c[1]).map(|v|v[2].to_owned()).ok_or_else(||format!("Missing material texture: {}",&c[1]))?;
                let pixel=match default.as_str(){"white"=>[255;4],"black"=>[0,0,0,255],"gray"=>[128,128,128,255],"bump"=>[128,128,255,255],_=>return Err(format!("Unsupported Shader default texture: {default}"))};
                bound_textures.insert(c[1].to_owned(),Arc::new(rs::Texture{size:[1,1],rgba:Arc::from(pixel),revision:1,repeat:[false;2]}));
            }
            // Unity sets these through the scene controller, not saved material
            // properties. Without that controller, use neutral white last.
            neutral_day_night_colors(&mut uniforms);
            if regex::Regex::new(r"(?i)\bStencil\s*\{").unwrap().is_match(&shader) {
                shared.status.lock().unwrap().errors.push(format!("{}: Stencil ignored in preview (mask not reproduced)",path.display()));
            }
            Ok(Arc::new(rs::Material { program:Arc::new(program),state,uniforms,textures:bound_textures }))
        });
        match material {
            Ok(material)=>{slots.push(Some(material.clone()));}
            Err(error)=>{shared.status.lock().unwrap().errors.push(format!("{}: {error}; material slot {} unavailable",path.display(),slots.len()));slots.push(None);},
        }
        }
        if slots.iter().any(Option::is_some) {custom.insert(sprite.component.clone(),slots);}
    }
    if custom.is_empty() {return Ok(None);}
    let mut gpu_textures=HashMap::new();let mut bytes=0usize;
    for instance in instances {
        if gpu_textures.contains_key(&instance.texture) {continue;}
        if !shared.checkpoint() {return Err("Preview cancelled".into());}
        let cpu=textures[instance.texture].as_ref().unwrap();
        let length=cpu.size[0]*cpu.size[1]*4;
        bytes=bytes.checked_add(length).ok_or("GPU source texture budget overflow")?;
        if bytes>TEXTURE_BUDGET {return Err("GPU source textures exceed 2 GiB; CPU fallback".into());}
        let rgba=if sprites.iter().any(|s|s.path==paths[instance.texture] && s.baked.is_some()) {
            cpu.pixels.iter().flat_map(|p|straight_gamma(*p)).collect::<Vec<u8>>()
        } else {
            let mut reader=image::ImageReader::open(&paths[instance.texture]).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
            let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
            let decoded=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
            if [decoded.width() as usize,decoded.height() as usize]!=cpu.size {return Err("Texture changed during preparation".into());}
            decoded.into_raw()
        };
        gpu_textures.insert(instance.texture,Arc::new(rs::Texture{size:cpu.size.map(|v|v as u32),rgba:Arc::from(rgba),revision:1,repeat:[false;2]}));
    }
    let mut draws=Vec::new();
    let sprite_lookup:HashMap<_,_>=sprites.iter().map(|s|(s.component.as_str(),s)).collect();
    let mut spine_textures:HashMap<PathBuf,Arc<rs::Texture>>=HashMap::new();
    for instance in instances {
        let id=components[instance.component as usize].as_str();
        if let Some(sprite)=sprite_lookup.get(id).filter(|s|s.id=="spine") {
            if let (Some(parts),Some(slots))=(sprite.baked.as_ref().and_then(|b|b.gpu.as_ref()),custom.get(id)) {
                let prepared=(||->Result<Vec<rs::Draw>,String>{
                    if parts.iter().any(|p|slots.get(p.slot).and_then(Option::as_ref).is_none()) {return Err("Atlas material slot unavailable".into());}
                    let mut matrix=sprite.spatial.0.map(|row|row.map(|v|v as f32));
                    for row in &mut matrix {for axis in 0..2 {if sprite.flip[axis]{row[axis]*=-1.0;}}}
                    let mut result=Vec::new();
                    for part in parts.iter() {
                        if !shared.checkpoint(){return Err("Preview cancelled".into());}
                        let texture=if let Some(texture)=spine_textures.get(&part.path){texture.clone()}else{
                            let mut reader=image::ImageReader::open(&part.path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
                            let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
                            let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
                            bytes=bytes.checked_add(image.len()).ok_or("Spine texture budget overflow")?;if bytes>TEXTURE_BUDGET {return Err("GPU source textures exceed 2 GiB".into());}
                            // Spine atlas UVs and pixel rows share the top-down convention.
                            // Preserve source pixels, including premultiplied-alpha atlases.
                            let texture=Arc::new(rs::Texture{size:[image.width(),image.height()],rgba:Arc::from(image.into_raw()),revision:1,repeat:[false;2]});
                            spine_textures.insert(part.path.clone(),texture.clone());texture
                        };
                        let material=slots[part.slot].as_ref().unwrap();
                        let vertices=if material.state.source==B::One && part.vertices.iter().any(|v|v.color[3]!=1.0) {Arc::from(part.vertices.iter().copied().map(|mut v|{for i in 0..3{v.color[i]*=v.color[3];}v}).collect::<Vec<_>>())}else{part.vertices.clone()};
                        result.push(rs::Draw{strict_shader:false,component:Arc::from(id),vertices,indices:part.indices.clone(),object_to_world:matrix,material:material.clone(),main_texture:texture});
                    }Ok(result)
                })();
                match prepared {Ok(parts)=>{draws.extend(parts);continue;},Err(error)=>{shared.status.lock().unwrap().errors.push(format!("Spine {id}: {error}; baked fallback"));}}
            }
            custom.remove(id);
        }
        let tint=docs.get(id).filter(|d|d.type_name=="SpriteRenderer").map(|d|{let c=vector(&d.text,"m_Color",&["r","g","b","a"],&[1.0;4]);std::array::from_fn(|i|c[i] as f32)}).unwrap_or_else(||straight_gamma(instance.tint).map(|v|v as f32/255.0));
        let mut vertices=instance.points.iter().zip(instance.uv).map(|(p,uv)|rs::Vertex{position:[p[0] as f32,p[1] as f32,0.0],uv:[uv[0] as f32,uv[1] as f32],color:tint,normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]}).collect::<Vec<_>>();
        let mut submeshes=vec![vec![0,1,2,0,2,3]];let mut object_to_world=identity;
        if custom.contains_key(id) && docs.get(id).is_some_and(|d|d.type_name=="MeshRenderer") {
            let renderer=docs[id];
            let geometry=(||->Result<_,String>{
                let mesh_filter=docs.values().find(|d|d.type_name=="MeshFilter" && d.game_object==renderer.game_object).ok_or("MeshRenderer has no MeshFilter")?;
                let reference_text=field(&mesh_filter.text,"m_Mesh").ok_or("Missing Mesh reference")?;
                let mesh=if reference(reference_text,"fileID").as_deref()==Some("10210") && reference(reference_text,"guid").as_deref()==Some("0000000000000000e000000000000000") {
                    crate::render_system::mesh::quad()
                }else if reference(reference_text,"guid").is_none() {
                    let id=reference(reference_text,"fileID").ok_or("Missing local Mesh fileID")?;
                    let source=docs.get(id.as_str()).filter(|doc|doc.type_name=="Mesh").ok_or("Scene-local Mesh fileID not found")?;
                    crate::render_system::mesh::decode(&source.text)?
                }else{
                    let path=reference(reference_text,"guid").and_then(|g|catalog.get(&g.to_lowercase())).ok_or("Native Mesh GUID missing")?;
                    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>64*1024*1024 {return Err("Unity Mesh asset exceeds 64 MiB".into());}
                    if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("fbx")) {
                        crate::render_system::mesh::fbx(path)?
                    }else if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("asset")) {
                        crate::render_system::mesh::decode_reference(&std::fs::read_to_string(path).map_err(|e|e.to_string())?,&reference(reference_text,"fileID").ok_or("Missing Mesh fileID")?)?
                    }else{return Err("Unsupported GPU mesh format".into());}
                };
                if mesh.submeshes.is_empty() {return Err("Mesh has no submeshes".into());}
                let transforms:HashMap<_,_>=docs.values().copied().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
                let objects:HashMap<_,_>=docs.values().copied().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
                let transform=transforms.values().find(|d|d.game_object==renderer.game_object).ok_or("Mesh Transform missing")?;
                let matrix=world(&transform.file_id,&transforms,&objects,&mut HashSet::new()).ok_or("Mesh world transform missing")?;
                Ok((mesh,matrix))
            })();
            match geometry {
                Ok((mesh,matrix))=>{vertices=mesh.vertices;submeshes=mesh.submeshes;object_to_world=matrix.0.map(|row|row.map(|v|v as f32));}
                Err(error)=>{shared.status.lock().unwrap().errors.push(format!("Mesh {id}: {error}; baked fallback"));custom.remove(id);}
            }
        }
        let vertices:Arc<[rs::Vertex]>=Arc::from(vertices);
        let submeshes:Vec<Arc<[u32]>>=submeshes.into_iter().map(Arc::from).collect();
        if let Some(slots)=custom.get(id) {
            if slots.len()<submeshes.len() {shared.status.lock().unwrap().errors.push(format!("Mesh {id}: {} submeshes but {} material slots; unmatched submeshes not rendered",submeshes.len(),slots.len()));}
            for (slot,material) in slots.iter().enumerate() {
                let Some(material)=material else {continue;};
                let main=material.textures.get("_MainTex").cloned().unwrap_or_else(||gpu_textures[&instance.texture].clone());
                draws.push(rs::Draw{strict_shader:false,component:Arc::from(id),vertices:vertices.clone(),indices:submeshes[slot.min(submeshes.len()-1)].clone(),object_to_world,material:material.clone(),main_texture:main});
            }
        }else{
            draws.push(rs::Draw{strict_shader:false,component:Arc::from(id),vertices,indices:submeshes[0].clone(),object_to_world,material:ordinary.clone(),main_texture:gpu_textures[&instance.texture].clone()});
        }
    }
    Ok(Some(Arc::new(rs::Scene{draws:Arc::from(draws),revision:1})))
}
pub(crate) fn spine_preview_resources(parts:Vec<crate::spine_preview::GpuPart>,materials:&str,catalog:&HashMap<String,PathBuf>)->Result<Arc<crate::render_system::scene::Scene>,String> {
    let docs=crate::parse_unity_yaml(&format!("--- !u!114 &1\nMonoBehaviour:\n  m_GameObject: {{fileID: 3}}\n  skeletonDataAsset: {{fileID: 1}}\n--- !u!23 &2\nMeshRenderer:\n  m_GameObject: {{fileID: 3}}\n  m_Materials:\n{materials}\n"));
    let baked=Arc::new(crate::spine_preview::Baked{image:egui::ColorImage::new([1,1],Color32::TRANSPARENT),region:Region{rect:[0.0,0.0,1.0,1.0],pivot:[0.5;2],ppu:1.0,border:[0.0;4]},approximate:false,gpu:Some(Arc::new(parts))});
    let path=PathBuf::from("__spine_gpu_preview");
    let sprite=Sprite{baked:Some(baked),ui:None,component:"1".into(),selectable:true,name:String::new(),path:path.clone(),id:"spine".into(),spatial:Matrix::identity(),matrix:Matrix::identity().into(),tint:Color32::WHITE,flip:[false;2],order:(0,0,0)};
    let instance=Instance{points:[[0.0,0.0];4],uv:[[0.0,0.0];4],tint:Color32::WHITE,texture:0,component:0,selectable:true};
    let texture=Arc::new(CpuTexture{size:[1,1],pixels:vec![Color32::TRANSPARENT],meta:String::new(),regions:Mutex::default()});
    let shared=Shared{particles:Mutex::default(),gpu_scene:Mutex::default(),cancelled:AtomicBool::new(false),paused:Arc::new(AtomicBool::new(false)),revision:AtomicU64::new(1),view:Mutex::default(),status:Mutex::default(),frame:Mutex::default()};
    let scene=gpu_scene(&[sprite],&[instance],&[Some(texture)],&[path],&["1".into()],&docs,Some(&Arc::new(catalog.clone())),&shared)?;
    let errors=shared.status.lock().unwrap().errors.clone();
    let scene=scene.ok_or_else(||errors.join("\n"))?;
    if scene.draws.iter().any(|d|d.main_texture.size==[1,1] && d.main_texture.rgba.iter().all(|v|*v==0)){return Err(errors.join("\n"));}
    Ok(scene)
}
fn neutral_day_night_colors(uniforms:&mut BTreeMap<String,crate::render_system::scene::Uniform>) {
    for name in ["_GlobalDayNightColor","_GlobalDayNightFogColor","_GlobalDayNightWaterColor"] {
        uniforms.insert(name.into(),crate::render_system::scene::Uniform::Vector([1.0;4]));
    }
}
fn straight_gamma(color:Color32)->[u8;4] {
    let a=color.a() as u32;
    let channel=|v:u8|if a==0 {0}else{((v as u32*255+a/2)/a).min(255) as u8};
    [channel(color.r()),channel(color.g()),channel(color.b()),color.a()]
}
fn sample(texture:&CpuTexture,uv:[f64;2]) -> Color32 {
    let x=(uv[0]*texture.size[0] as f64-0.5).clamp(0.0,(texture.size[0]-1) as f64);
    let y=(uv[1]*texture.size[1] as f64-0.5).clamp(0.0,(texture.size[1]-1) as f64);
    let ix=x.floor() as usize;let iy=y.floor() as usize;let dx=x-ix as f64;let dy=y-iy as f64;
    let get=|x:usize,y:usize|texture.pixels[y.min(texture.size[1]-1)*texture.size[0]+x.min(texture.size[0]-1)];
    let pixels=[get(ix,iy),get(ix+1,iy),get(ix,iy+1),get(ix+1,iy+1)];let weights=[(1.0-dx)*(1.0-dy),dx*(1.0-dy),(1.0-dx)*dy,dx*dy];
    let channel=|i:usize|(0..4).map(|j|pixels[j].to_array()[i] as f64*weights[j]).sum::<f64>().round() as u8;
    Color32::from_rgba_premultiplied(channel(0),channel(1),channel(2),channel(3))
}
#[allow(clippy::too_many_arguments)]
fn compose(instances:&[Instance],textures:&[Option<Arc<CpuTexture>>],view:&View,bounds:[f64;4],components:Arc<Vec<String>>,boxes:Arc<HashMap<String,[f64;4]>>,shared:&Shared,revision:u64) -> Option<Frame> {
    let [width,height]=view.size;
    let scale=((width as f64-40.0).max(1.0)/(bounds[2]-bounds[0]).max(0.01)).min((height as f64-40.0).max(1.0)/(bounds[3]-bounds[1]).max(0.01))*view.zoom;
    let mut center=[(bounds[0]+bounds[2])*0.5,(bounds[1]+bounds[3])*0.5];
    let mut focused=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];
    for id in &view.focus {if let Some(b)=boxes.get(id) {include(&mut focused,[[b[0],b[1]],[b[2],b[3]],[b[0],b[3]],[b[2],b[1]]]);}}
    if focused.iter().all(|v|v.is_finite()) {center=[(focused[0]+focused[2])*0.5,(focused[1]+focused[3])*0.5];}
    let screen=|p:[f64;2]|[width as f64*0.5+view.pan[0]*width as f64+(p[0]-center[0])*scale,height as f64*0.5+view.pan[1]*height as f64-(p[1]-center[1])*scale];
    let mut image=egui::ColorImage::new([width,height],Color32::TRANSPARENT);let mut hits=vec![u32::MAX;width*height];
    for (index,instance) in instances.iter().enumerate() {
        if index%128==0 && (!shared.checkpoint() || revision!=shared.revision.load(Ordering::Relaxed)) {return None;}
        if instance.tint.a()==0 {continue;}
        let points=instance.points.map(screen);let mut area=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];include(&mut area,points);
        let min_x=area[0].floor().clamp(0.0,width as f64) as usize;let max_x=area[2].ceil().clamp(0.0,width as f64) as usize;
        let min_y=area[1].floor().clamp(0.0,height as f64) as usize;let max_y=area[3].ceil().clamp(0.0,height as f64) as usize;
        let u=[points[1][0]-points[0][0],points[1][1]-points[0][1]];let v=[points[3][0]-points[0][0],points[3][1]-points[0][1]];let determinant=u[0]*v[1]-u[1]*v[0];
        if determinant.abs()<1e-12 {continue;}
        let texture=textures[instance.texture].as_ref().unwrap();
        for y in min_y..max_y {
            if (y-min_y)%32==0 && (!shared.checkpoint() || revision!=shared.revision.load(Ordering::Relaxed)) {return None;}
            for x in min_x..max_x {
                let p=[x as f64+0.5-points[0][0],y as f64+0.5-points[0][1]];let a=(p[0]*v[1]-p[1]*v[0])/determinant;let b=(u[0]*p[1]-u[1]*p[0])/determinant;
                if !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b) {continue;}
                let uv=std::array::from_fn(|i|instance.uv[0][i]+(instance.uv[1][i]-instance.uv[0][i])*a+(instance.uv[3][i]-instance.uv[0][i])*b);
                let source=sample(texture,uv);let position=y*width+x;
                image.pixels[position]=over(source,instance.tint,image.pixels[position]);
                if instance.selectable && crate::render_system::pickable(source,instance.tint) {hits[position]=instance.component;}
            }
        }
    }
    Some(Frame {image,hits,components,boxes,center,scale,pan:view.pan,revision,view:view.clone()})
}

#[cfg(test)]
mod tests {
    #[test]
    fn spine_gpu_resources_keep_original_geometry_and_material_tint() {
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("spine-resources-test-{}",std::process::id()));
        let assets=root.join("Assets");std::fs::create_dir_all(&assets).unwrap();
        let shader=assets.join("test.shader");std::fs::write(&shader,include_str!("../../tests/fixtures/pf_sprite_overlay.shader")).unwrap();
        let mat=assets.join("night.mat");std::fs::write(&mat,"Material:\n  m_Shader: {fileID: 4800000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}\n  m_Colors:\n  - _Color: {r: 0.5, g: 0.6, b: 0.7, a: 1}\n").unwrap();
        let page=assets.join("page.png");image::RgbaImage::from_pixel(2,2,image::Rgba([64,128,192,128])).save(&page).unwrap();
        let vertex=crate::render_system::scene::Vertex{position:[3.0,4.0,0.0],uv:[0.25,0.75],color:[1.0;4],normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]};
        let part=crate::spine_preview::GpuPart{path:page.clone(),slot:0,vertices:Arc::from(vec![vertex;3]),indices:Arc::from(vec![0,1,2])};
        let catalog=HashMap::from([("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),shader.clone()),("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),mat.clone())]);
        let scene=spine_preview_resources(vec![part],"  - {fileID: 2100000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb}",&catalog).unwrap();
        assert_eq!(scene.draws.len(),1);
        assert_eq!(scene.draws[0].vertices[0].position,[3.0,4.0,0.0]);
        assert_eq!(scene.draws[0].vertices[0].uv,[0.25,0.75]);
        assert_eq!(&scene.draws[0].main_texture.rgba[..4],&[64,128,192,128]);
        assert_eq!(scene.draws[0].material.uniforms["_Color"],crate::render_system::scene::Uniform::Vector([0.5,0.6,0.7,1.0]));
        assert!(!scene.draws[0].material.textures.contains_key("_MainTex"));
        for path in [&shader,&mat,&page]{std::fs::remove_file(path).unwrap();}std::fs::remove_dir(assets).unwrap();std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn tilemap_material_preserves_each_cell_texture_tint_and_uv() {
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("tilemap-material-test-{}",std::process::id()));
        let assets=root.join("Assets");std::fs::create_dir_all(&assets).unwrap();
        let shader=assets.join("test.shader");std::fs::write(&shader,include_str!("../../tests/fixtures/pf_sprite_overlay.shader")).unwrap();
        let mat=assets.join("night.mat");
        std::fs::write(&mat,"Material:\n  m_Shader: {fileID: 4800000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}\n  m_Colors:\n  - _Color: {r: 0.5, g: 0.6, b: 0.7, a: 1}\n").unwrap();
        let paths=[assets.join("red.png"),assets.join("green.png")];
        let colors=[Color32::RED,Color32::GREEN];
        for (path,pixel) in paths.iter().zip([[255,0,0,255],[0,255,0,255]]) {image::RgbaImage::from_pixel(1,1,image::Rgba(pixel)).save(path).unwrap();}
        let docs=crate::parse_unity_yaml("--- !u!1839735485 &10\nTilemap:\n  m_GameObject: {fileID: 1}\n--- !u!483693784 &11\nTilemapRenderer:\n  m_GameObject: {fileID: 1}\n  m_Materials:\n  - {fileID: 2100000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb}\n");
        let sprites:Vec<_>=paths.iter().map(|path|Sprite{spatial:Matrix::identity(),baked:None,ui:None,component:"10".into(),selectable:true,name:String::new(),path:path.clone(),id:String::new(),matrix:Matrix::identity().into(),tint:Color32::WHITE,flip:[false;2],order:(0,0,0)}).collect();
        let instances:Vec<_>=(0..2).map(|texture|Instance{points:[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]],uv:[[0.25,0.75];4],tint:crate::asset_viewer::preview_color([128,255,255,128]),texture,component:0,selectable:true}).collect();
        let textures:Vec<_>=colors.iter().map(|color|Some(Arc::new(CpuTexture{size:[1,1],pixels:vec![*color],meta:String::new(),regions:Mutex::default()}))).collect();
        let catalog=Arc::new(HashMap::from([("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),shader.clone()),("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),mat.clone())]));
        let state=shared();
        let scene=gpu_scene(&sprites,&instances,&textures,&paths,&["10".into()],&docs,Some(&catalog),&state).unwrap().unwrap();
        assert_eq!(scene.draws.len(),2,"{:?}",state.status.lock().unwrap().errors);
        assert_eq!(scene.draws[0].material.uniforms["_Color"],crate::render_system::scene::Uniform::Vector([0.5,0.6,0.7,1.0]));
        assert!(Arc::ptr_eq(&scene.draws[0].material,&scene.draws[1].material));
        assert_eq!(&*scene.draws[0].main_texture.rgba,&[255,0,0,255]);
        assert_eq!(&*scene.draws[1].main_texture.rgba,&[0,255,0,255]);
        assert_eq!(scene.draws[0].vertices[0].uv,[0.25,0.75]);
        assert_eq!(scene.draws[0].vertices[0].color[3],128.0/255.0);
        assert_eq!(&*scene.draws[0].component,"10");
        assert!(!scene.draws[0].material.textures.contains_key("_MainTex"));
        for path in paths.iter().chain([&shader,&mat]) {std::fs::remove_file(path).unwrap();}
        std::fs::remove_dir(assets).unwrap();std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn one_submesh_draws_both_material_slots_in_order() {
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("material-slots-test-{}",std::process::id()));
        let assets=root.join("Assets");std::fs::create_dir_all(&assets).unwrap();
        let shader=assets.join("test.shader");std::fs::write(&shader,include_str!("../../tests/fixtures/pf_sprite_overlay.shader")).unwrap();
        let shader_guid="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let red_guid="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";let blue_guid="cccccccccccccccccccccccccccccccc";
        let red=assets.join("red.mat");let blue=assets.join("blue.mat");
        for (path,color) in [(&red,"{r: 1, g: 0, b: 0, a: 1}"),(&blue,"{r: 0, g: 0, b: 1, a: 1}")] {
            std::fs::write(path,format!("Material:\n  m_Shader: {{fileID: 4800000, guid: {shader_guid}, type: 3}}\n  m_Colors:\n  - _Color: {color}\n")).unwrap();
        }
        let image_path=assets.join("white.png");image::RgbaImage::from_pixel(1,1,image::Rgba([255;4])).save(&image_path).unwrap();
        let docs=crate::parse_unity_yaml(&format!("--- !u!1 &1\nGameObject:\n  m_Name: mesh\n  m_IsActive: 1\n--- !u!4 &2\nTransform:\n  m_GameObject: {{fileID: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Father: {{fileID: 0}}\n--- !u!33 &3\nMeshFilter:\n  m_GameObject: {{fileID: 1}}\n  m_Mesh: {{fileID: 10210, guid: 0000000000000000e000000000000000, type: 0}}\n--- !u!23 &4\nMeshRenderer:\n  m_GameObject: {{fileID: 1}}\n  m_Materials:\n  - {{fileID: 2100000, guid: {red_guid}, type: 2}}\n  - {{fileID: 2100000, guid: {blue_guid}, type: 2}}\n"));
        let sprite=Sprite{spatial:Matrix::identity(),baked:None,ui:None,component:"4".into(),selectable:true,name:"mesh".into(),path:image_path.clone(),id:String::new(),matrix:Matrix::identity().into(),tint:Color32::WHITE,flip:[false;2],order:(0,0,0)};
        let instance=Instance{points:[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]],uv:[[0.0,0.0];4],tint:Color32::WHITE,texture:0,component:0,selectable:true};
        let texture=Arc::new(CpuTexture{size:[1,1],pixels:vec![Color32::WHITE],meta:String::new(),regions:Mutex::default()});
        let catalog=Arc::new(HashMap::from([(shader_guid.into(),shader.clone()),(red_guid.into(),red.clone()),(blue_guid.into(),blue.clone())]));
        let state=shared();
        let scene=gpu_scene(&[sprite],&[instance],&[Some(texture)],&[image_path.clone()],&["4".into()],&docs,Some(&catalog),&state).unwrap().unwrap();
        assert_eq!(scene.draws.len(),2,"{:?}",state.status.lock().unwrap().errors);
        assert_eq!(scene.draws[0].material.uniforms["_Color"],crate::render_system::scene::Uniform::Vector([1.0,0.0,0.0,1.0]));
        assert_eq!(scene.draws[1].material.uniforms["_Color"],crate::render_system::scene::Uniform::Vector([0.0,0.0,1.0,1.0]));
        assert!(Arc::ptr_eq(&scene.draws[0].vertices,&scene.draws[1].vertices));
        assert!(Arc::ptr_eq(&scene.draws[0].indices,&scene.draws[1].indices));
        for path in [&shader,&red,&blue,&image_path] {std::fs::remove_file(path).unwrap();}
        std::fs::remove_dir(assets).unwrap();std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn preview_neutral_colors_and_pointer_zoom() {
        let mut uniforms=BTreeMap::new();
        uniforms.insert("_GlobalDayNightColor".into(),crate::render_system::scene::Uniform::Vector([0.0;4]));
        neutral_day_night_colors(&mut uniforms);
        for name in ["_GlobalDayNightColor","_GlobalDayNightFogColor","_GlobalDayNightWaterColor"] {
            assert!(matches!(uniforms[name],crate::render_system::scene::Uniform::Vector(v) if v==[1.0;4]));
        }
        let rect=egui::Rect::from_min_size(Pos2::new(20.0,40.0),egui::vec2(800.0,600.0));
        let mut view=View {size:[800,600],pan:[0.2,-0.1],..View::default()};
        let cached=view.clone();
        let pointer=Pos2::new(650.0,190.0);
        zoom_at_pointer(&mut view,rect,pointer,150.0);
        let image=cached_image_rect(rect,&view,&cached);
        let anchored=image.min+(pointer-rect.min)/rect.size()*image.size();
        assert!((anchored-pointer).length()<0.001);
        let mut fps=PreviewFps::default();
        for i in 0..=30 {fps.sample(i as f64/60.0);}
        assert!((fps.value-60.0).abs()<0.001);
    }
    use super::*;
    #[test]
    #[ignore = "Requires UNITY_SHADER_TEST_PROJECT pointing to a local Unity project"]
    fn real_project_material_preparation() {
        let root=PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        let mut pending=vec![root.join("Assets")];let mut catalog=HashMap::new();
        while let Some(directory)=pending.pop() {
            for entry in std::fs::read_dir(directory).unwrap().flatten() {
                let path=entry.path();if path.is_dir(){pending.push(path);continue;}
                if path.extension().is_some_and(|e|e=="meta") {
                    if let Ok(text)=std::fs::read_to_string(&path) {
                        if let Some(guid)=text.lines().find_map(|l|l.strip_prefix("guid: ")) {
                            let asset=path.with_extension("");catalog.insert(guid.trim().to_lowercase(),asset);
                        }
                    }
                }
            }
        }
        let docs=crate::parse_unity_yaml(&std::fs::read_to_string(root.join("Assets/Res/Prefabs/Map/Level101/map_land.prefab")).unwrap());
        let sprites:Vec<_>=docs.iter().filter(|d|matches!(d.type_name.as_str(),"SpriteRenderer"|"MeshRenderer")).map(|d|Sprite{spatial:Matrix::identity(),baked:None,ui:None,component:d.file_id.clone(),selectable:true,name:String::new(),path:PathBuf::new(),id:String::new(),matrix:Matrix::identity().into(),tint:Color32::WHITE,flip:[false;2],order:(0,0,0)}).collect();
        let state=shared();
        let scene=gpu_scene(&sprites,&[],&[],&[],&[],&docs,Some(&Arc::new(catalog)),&state).unwrap();
        let errors=&state.status.lock().unwrap().errors;
        let unique:HashSet<_>=errors.iter().collect();for error in unique {println!("{error}");}
        assert!(scene.is_some(),"All custom shaders were rejected");
    }
    fn shared() -> Shared {
        Shared {particles:Mutex::default(),gpu_scene:Mutex::new(None),cancelled:AtomicBool::new(false),paused:Arc::new(AtomicBool::new(false)),revision:AtomicU64::new(1),view:Mutex::new(View::default()),status:Mutex::new(Status::default()),frame:Mutex::new(None)}
    }
    #[test]
    fn small_tile_decodes_with_exact_pixel_reservation() {
        let path=std::env::temp_dir().join(format!("unity_yaml_tile_decode_{}_{}.png",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        image::RgbaImage::from_pixel(64,33,image::Rgba([120,180,60,255])).save(&path).unwrap();
        let state=shared();let reserved=std::sync::atomic::AtomicUsize::new(0);let cache=Mutex::new(TextureCache::default());
        let result=load_cached(&path,&reserved,&cache,&state);
        std::fs::remove_file(&path).unwrap();
        let (texture,hit)=result.unwrap();
        assert!(!hit);assert_eq!(texture.size,[64,33]);assert_eq!(texture.pixels.len(),64*33);
        assert_eq!(reserved.load(Ordering::Relaxed),64*33*4);
    }
    #[test]
    fn texture_cache_reuses_arc_and_invalidates_image_or_meta_changes() {
        let mut cache=TextureCache::default();let path=PathBuf::from("sprite.png");
        let version=Stamp{image:(None,4),meta:(None,0)};
        let texture=Arc::new(CpuTexture{size:[1,1],pixels:vec![Color32::WHITE],meta:String::new(),regions:Mutex::default()});
        let saved=cache.insert(path.clone(),version.clone(),texture.clone());
        assert!(Arc::ptr_eq(&saved,&cache.get(&path,&version).unwrap()));assert_eq!(cache.bytes,4);
        let changed=Stamp{meta:(None,1),..version.clone()};assert!(cache.get(&path,&changed).is_none());assert_eq!(cache.bytes,0);
        cache.insert(path.clone(),version.clone(),texture);
        let changed=Stamp{image:(None,5),..version};assert!(cache.get(&path,&changed).is_none());
    }
    #[test]
    fn concurrent_loading_reservations_never_exceed_budget() {
        let reserved=std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {for _ in 0..8 {let reserved=&reserved;scope.spawn(move || {reserve_bytes(reserved,TEXTURE_BUDGET/4);});}});
        assert_eq!(reserved.load(Ordering::Relaxed),TEXTURE_BUDGET);
        assert!(!reserve_bytes(&reserved,1));
    }
    #[test]
    fn composite_respects_draw_order_and_alpha_picking_with_mirrored_quad() {
        let textures=vec![Some(Arc::new(CpuTexture {size:[1,1],pixels:vec![Color32::RED],meta:String::new(),regions:Mutex::default()})),Some(Arc::new(CpuTexture {size:[1,1],pixels:vec![Color32::BLUE],meta:String::new(),regions:Mutex::default()}))];
        let points=[[-1.0,-1.0],[1.0,-1.0],[1.0,1.0],[-1.0,1.0]];
        let uv=[[0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0,0.0]];
        let instances=vec![Instance {points,uv,tint:Color32::WHITE,texture:0,component:0,selectable:true},Instance {points:[points[1],points[0],points[3],points[2]],uv,tint:Color32::WHITE,texture:1,component:1,selectable:true}];
        let view=View {size:[64,64],..View::default()};let state=shared();
        let frame=compose(&instances,&textures,&view,[-1.0,-1.0,1.0,1.0],Arc::new(vec!["back".into(),"front".into()]),Arc::new(HashMap::new()),&state,1).unwrap();
        assert_eq!(frame.image.pixels[32*64+32],Color32::BLUE);
        assert_eq!(frame.hits[32*64+32],1);
        assert_eq!(frame.hits[0],u32::MAX);
    }
    #[test]
    fn cached_view_reuses_small_zoom_and_refreshes_relative_to_last_frame() {
        let rect=egui::Rect::from_min_size(Pos2::ZERO,egui::vec2(800.0,600.0));
        let cached=View {size:[800,600],zoom:2.0,pan:[0.0;2],focus:Vec::new()};
        let mut current=cached.clone();current.zoom=2.8;
        assert!(!needs_view_refresh(rect,&current,&cached));
        current.zoom=3.0;assert!(needs_view_refresh(rect,&current,&cached));
        current=cached.clone();current.pan[0]=0.1;
        assert!(needs_view_refresh(rect,&current,&cached));
        current=cached.clone();current.zoom=1.9;
        assert!(needs_view_refresh(rect,&current,&cached)); // Newly exposed pixels need composition.
        current=cached.clone();current.size=[801,600];assert!(needs_view_refresh(rect,&current,&cached));
        current=cached.clone();current.focus.push("node".into());assert!(needs_view_refresh(rect,&current,&cached));
    }
    #[test]
    fn cached_transform_keeps_pan_and_inverse_picking_aligned() {
        let rect=egui::Rect::from_min_size(Pos2::ZERO,egui::vec2(800.0,600.0));
        let cached=View {size:[800,600],zoom:2.0,pan:[0.1,-0.1],focus:Vec::new()};
        let current=View {zoom:2.5,pan:[0.2,0.0],..cached.clone()};
        let image_rect=cached_image_rect(rect,&current,&cached);
        let cached_origin=egui::vec2(0.6,0.4);
        let displayed=image_rect.min+cached_origin*image_rect.size();
        assert!((displayed-(rect.center()+egui::vec2(160.0,0.0))).length()<0.001);
        let picked_uv=(displayed-image_rect.min)/image_rect.size();
        assert!((picked_uv-cached_origin).length()<0.001);
    }
    #[test]
    fn cancelled_paused_task_can_exit_without_resume() {
        let state=shared();state.paused.store(true,Ordering::Relaxed);state.cancelled.store(true,Ordering::Relaxed);
        assert!(!state.checkpoint());
    }
    #[test]
    fn changed_view_discards_old_composition() {
        let state=shared();state.revision.store(2,Ordering::Relaxed);
        let instance=Instance {points:[[-1.0,-1.0],[1.0,-1.0],[1.0,1.0],[-1.0,1.0]],uv:[[0.0;2];4],tint:Color32::WHITE,texture:0,component:0,selectable:true};
        let textures=vec![Some(Arc::new(CpuTexture {size:[1,1],pixels:vec![Color32::WHITE],meta:String::new(),regions:Mutex::default()}))];
        assert!(compose(&[instance],&textures,&View::default(),[-1.0,-1.0,1.0,1.0],Arc::default(),Arc::default(),&state,1).is_none());
    }
    #[test]
    fn alpha_composition_multiplies_texture_and_tint() {
        let source=crate::asset_viewer::preview_color([255,255,255,128]);let tint=crate::asset_viewer::preview_color([255,255,255,128]);
        let combined=over(source,tint,Color32::TRANSPARENT);
        assert_eq!(combined.a(),64);assert_eq!(combined.r(),64);
        assert_eq!(over(Color32::TRANSPARENT,tint,Color32::RED),Color32::RED);
    }
    #[test]
    fn bilinear_sampling_keeps_premultiplied_alpha() {
        let t=CpuTexture {size:[2,1],pixels:vec![Color32::WHITE,Color32::TRANSPARENT],meta:String::new(),regions:Mutex::default()};
        let c=sample(&t,[0.5,0.5]);assert_eq!(c.a(),128);assert_eq!(c.r(),128);
    }
}
