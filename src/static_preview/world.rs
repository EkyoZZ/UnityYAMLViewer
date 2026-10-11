//! CPU scene composition runs off the UI thread. Only a bounded canvas image is uploaded.
use super::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_CANVAS: f32 = 1024.0;
const VIEW_IDLE: Duration = Duration::from_millis(500);
const ZOOM_REFRESH_RATIO: f64 = 1.5;
const TEXTURE_BUDGET: usize = 256 * 1024 * 1024;

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
}
impl World {
    pub(super) fn release(&mut self) {
        self.job=None;
        if let Some(textures)=self.textures.take() {std::thread::spawn(move || drop(textures));}
        self.texture=None;self.frame_key=None;self.errors.clear();
        if let Some(frame)=self.frame.take() {std::thread::spawn(move || drop(frame));}
        self.view=View::default();self.last_view_change=None;
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn show(&mut self, ui:&mut egui::Ui, documents:&[Document], flags:&HashMap<String,crate::hierarchy_controls::Flags>, catalog:Option<Arc<HashMap<String,PathBuf>>>, axis:usize, paused:Arc<AtomicBool>, selected:Option<&str>, focus:&mut Option<(HashSet<String>,Instant)>, background:&str, rgb:[u8;3],show_guides:bool,expanding:bool,open:&mut bool) -> Option<String> {
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
            // Cancel the old worker while retaining the displayed frame and camera.
            self.job=None;self.status_stage=0;self.status_text=tr("正在准备预览…").into();
            let shared=Arc::new(Shared {cancelled:AtomicBool::new(false),paused,revision:AtomicU64::new(1),view:Mutex::new(self.view.clone()),status:Mutex::new(Status::default()),frame:Mutex::new(None)});
            let worker=shared.clone();let docs=documents.to_vec();let flags=flags.clone();let ctx=ui.ctx().clone();
            let textures=self.textures.get_or_insert_with(||Arc::new(Mutex::new(TextureCache::default()))).clone();
            std::thread::spawn(move || run(&docs,&flags,catalog.as_ref(),axis,&worker,&ctx,textures));
            self.job=Some(Job {key,shared});
        }
        let shared=self.job.as_ref().unwrap().shared.clone();
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
        let status=if expanding {tr("正在展开 Prefab…")} else if complete {tr("合成预览已完成")} else if self.status_stage==3 {tr("正在合成预览…")} else {&self.status_text};
        if super::preview_status_row(ui,status,complete,&shared.paused,Some(&self.resource_details)) {*open=false;return None;}
        if !self.errors.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color,format!("{}: {}",tr("部分资源缺失或暂不支持"),self.errors.len())).on_hover_text(self.errors.iter().take(32).cloned().collect::<Vec<_>>().join("\n"));
        }
        ui.small(tr("XY 正交视图 · 滚轮缩放 · 拖动平移"));
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.0)),egui::Sense::click_and_drag());
        let previous_view=self.view.clone();
        let factor=(MAX_CANVAS/rect.width().max(rect.height())).min(1.0);
        self.view.size=[(rect.width()*factor).round().max(1.0) as usize,(rect.height()*factor).round().max(1.0) as usize];
        if response.dragged() {let delta=response.drag_delta();self.view.pan[0]+=delta.x as f64/rect.width() as f64;self.view.pan[1]+=delta.y as f64/rect.height() as f64;}
        let scroll=if response.hovered() {ui.input(|i|i.smooth_scroll_delta.y)} else {0.0};
        if scroll!=0.0 {self.view.zoom=(self.view.zoom*(scroll as f64*0.002).exp()).clamp(0.05,50.0);}
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
        if !shared.paused.load(Ordering::Relaxed) {
            let incoming=shared.frame.try_lock().ok().and_then(|mut latest|latest.take());
            if let Some(mut frame)=incoming {
                if frame.revision==shared.revision.load(Ordering::Relaxed) {
                    let image=std::mem::replace(&mut frame.image,egui::ColorImage::new([0,0],Color32::TRANSPARENT));
                    if let Some(texture)=&mut self.texture {texture.set(image,egui::TextureOptions::LINEAR);} else {self.texture=Some(ui.ctx().load_texture("scene_composite",image,egui::TextureOptions::LINEAR));}
                    self.frame_key=Some(key);
                    if let Some(old)=self.frame.replace(frame) {std::thread::spawn(move || drop(old));}
                } else {std::thread::spawn(move || drop(frame));}
            }
        }
        let painter=ui.painter_at(rect);
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
            painter.image(texture.id(),image_rect,egui::Rect::from_min_max(Pos2::ZERO,Pos2::new(1.0,1.0)),Color32::WHITE);
            if let Some(area)=picked.as_deref().or(selected).and_then(|id|frame.boxes.get(id)) {

                painter.rect_stroke(egui::Rect::from_two_pos(screen(area[0],area[1]),screen(area[2],area[3])),0.0,egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235)),egui::StrokeKind::Inside);
            }
        }
        super::paint_zoom_badge(&painter,rect,self.view.zoom);
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        picked
    }
    pub(super) fn set_zoom(&mut self,zoom:f64) {
        self.view.zoom=zoom;self.view.pan=[0.0;2];self.last_view_change=Some(Instant::now());
    }
    pub(super) fn fit(&mut self) {self.view.zoom=1.0;self.view.pan=[0.0;2];self.view.focus.clear();}
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
        if !reserve_bytes(reserved,texture_bytes(&texture)) {return Err("Preview texture cache exceeds 256 MiB".into());}
        return Ok((texture,true));
    }
    let (width,height)=image::ImageReader::open(path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())?;
    if width>8192 || height>8192 || version.meta.1>8*1024*1024 {return Err("Preview texture or Sprite metadata exceeds size limit".into());}
    let pixels=width as usize*height as usize*4;let bytes=pixels+version.meta.1 as usize;
    if !reserve_bytes(reserved,bytes) {return Err("Preview texture cache exceeds 256 MiB".into());}
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
    if size[0]*size[1]*4>remaining {return Err("Preview texture cache exceeds 256 MiB".into());}
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
    let prepared=super::prepare(docs,flags,catalog,&shared.cancelled,axis,Some(&shared.paused));
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
                    if reserve_bytes(reserved,baked.image.pixels.len()*4) {Ok((Arc::new(CpuTexture{size:baked.image.size,pixels:baked.image.pixels.clone(),meta:String::new(),regions:Mutex::default()}),false))} else {Err("Preview texture cache exceeds 256 MiB".into())}
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
    let a=(source.a() as u32*tint.a() as u32+127)/255;
    let c=|s:u8,t:u8,d:u8|((s as u32*t as u32+127)/255+(d as u32*(255-a)+127)/255).min(255) as u8;
    Color32::from_rgba_premultiplied(c(source.r(),tint.r(),dest.r()),c(source.g(),tint.g(),dest.g()),c(source.b(),tint.b(),dest.b()),(a+(dest.a() as u32*(255-a)+127)/255).min(255) as u8)
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
                if instance.selectable && source.a() as u32*instance.tint.a() as u32>8*255 {hits[position]=instance.component;}
            }
        }
    }
    Some(Frame {image,hits,components,boxes,center,scale,pan:view.pan,revision,view:view.clone()})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shared() -> Shared {
        Shared {cancelled:AtomicBool::new(false),paused:Arc::new(AtomicBool::new(false)),revision:AtomicU64::new(1),view:Mutex::new(View::default()),status:Mutex::new(Status::default()),frame:Mutex::new(None)}
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
