//! Spine 3.8 resource preview. Runtime objects stay on the UI thread.
use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc, time::Instant};
use eframe::egui::{self, Color32};
use spine38::{Atlas, SkeletonBinary, SkeletonJson, AnimationStateData, controller::SkeletonController};

pub(crate) fn is_data_asset(text:&str) -> bool {
    crate::static_preview::field(text,"skeletonJSON").is_some() && text.lines().any(|line|line=="  atlasAssets:")
}

fn callbacks() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        spine38::extension::set_create_texture_cb(|page,path|page.renderer_object().set(PathBuf::from(path)));
        spine38::extension::set_dispose_texture_cb(|page|unsafe {page.renderer_object().dispose::<PathBuf>()});
    });
}

pub(crate) struct Baked {
    pub image: egui::ColorImage,
    pub region: crate::static_preview::Region,
    pub approximate: bool,
    pub gpu: Option<Arc<Vec<GpuPart>>>,
}
pub(crate) struct GpuPart {pub path:PathBuf,pub slot:usize,pub vertices:Arc<[crate::render_system::scene::Vertex]>,pub indices:Arc<[u32]>}

/// Bake setup pose off-thread, retaining transparent margins and skeleton-space origin.
pub(crate) fn bake(component: &str, catalog: &HashMap<String,PathBuf>, cancelled: &std::sync::atomic::AtomicBool, paused:Option<&std::sync::atomic::AtomicBool>) -> Result<(PathBuf,Baked),String> {
    use crate::static_preview::{field,reference,number};
    let resolve=|value:&str|reference(value,"guid").and_then(|g|catalog.get(&g.to_ascii_lowercase()).cloned()).ok_or("Spine 资源 GUID 未找到".to_owned());
    let data_path=resolve(field(component,"skeletonDataAsset").ok_or("缺少 skeletonDataAsset")?)?;
    let read_text=|path:&Path| -> Result<String,String> {
        if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>128*1024 {return Err("Spine 引用文件超过读取限制".into());}
        std::fs::read_to_string(path).map_err(|e|e.to_string())
    };
    let source=read_text(&data_path)?;
    let skeleton=resolve(field(&source,"skeletonJSON").ok_or("缺少 skeletonJSON")?)?;
    let atlas_reference=source.split_once("  atlasAssets:").and_then(|(_,tail)|tail.lines().find(|line|line.trim_start().starts_with("- {"))).ok_or("缺少 atlasAssets")?;
    let atlas_asset=read_text(&resolve(atlas_reference.trim().trim_start_matches('-').trim())?)?;
    let atlas_file=resolve(field(&atlas_asset,"atlasFile").ok_or("缺少 atlasFile")?)?;
    callbacks();
    if std::fs::metadata(&skeleton).map_err(|e|e.to_string())?.len()>16*1024*1024 {return Err("骨骼文件超过 16 MiB".into());}
    let bytes=std::fs::read(&skeleton).map_err(|e|e.to_string())?;
    let json=bytes.iter().copied().find(|b|!b.is_ascii_whitespace())==Some(b'{');
    let version=version(&bytes,json)?;
    if !version.starts_with("3.8.") {return Err(format!("仅支持 Spine 3.8，资源为 {version}"));}
    let atlas=Arc::new(Atlas::new_from_file(&atlas_file).map_err(|e|e.to_string())?);
    let mut images=HashMap::new();let mut total=0usize;
    let atlas_materials:Vec<_>=atlas_asset.split_once("  materials:").map(|(_,s)|s.lines().skip(1).take_while(|l|l.trim_start().starts_with('-')).filter_map(|l|resolve(l.trim().trim_start_matches('-').trim()).ok()).collect()).unwrap_or_default();
    let mut page_slots=HashMap::new();
    for (slot,material) in atlas_materials.iter().enumerate() {let text=read_text(material)?;if let Some(tail)=text.split_once("- _MainTex:").map(|(_,s)|s){if let Some(reference)=tail.lines().find_map(|l|l.trim().strip_prefix("m_Texture:")){if let Ok(path)=resolve(reference.trim()){if page_slots.insert(path.clone(),slot).is_some(){page_slots.insert(path,usize::MAX);}}}}}
    for page in atlas.pages() {
        if !crate::static_preview::checkpoint(cancelled,paused) {return Err("预览已取消".into());}
        let path=unsafe {page.renderer_object().get::<PathBuf>().cloned()}.ok_or("缺少贴图路径")?;
        let mut reader=image::ImageReader::open(&path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
        let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
        let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
        total+=image.as_raw().len();if total>256*1024*1024 {return Err("Spine 贴图超过 256 MiB".into());}
        images.insert(path,image);
    }
    let scale=number(&source,"scale",0.01) as f32;
    if !scale.is_finite() || scale<=0.0 {return Err("SkeletonDataAsset scale 无效".into());}
    let data=if json {let mut loader=SkeletonJson::new(atlas);loader.set_scale(scale);loader.read_skeleton_data(&bytes)} else {let mut loader=SkeletonBinary::new(atlas);loader.set_scale(scale);loader.read_skeleton_data(&bytes)}.map_err(|e|e.to_string())?;
    let data=Arc::new(data);let state=Arc::new(AnimationStateData::new(data.clone()));
    let mut controller=SkeletonController::new(data,state);
    let skin=field(component,"initialSkinName").unwrap_or("").trim_matches('"').trim_matches('\'');
    if !skin.is_empty() {controller.skeleton.set_skin_by_name(skin).map_err(|e|e.to_string())?;}
    controller.skeleton.set_to_setup_pose();controller.update(0.0);
    let meshes=controller.renderables();
    let points:Vec<_>=meshes.iter().flat_map(|m|m.vertices.iter().copied()).collect();
    if points.is_empty() || points.iter().flatten().any(|v|!v.is_finite()) {return Err("Spine 初始姿势没有可绘制网格".into());}
    let mut bounds=[f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY];
    for p in &points {bounds[0]=bounds[0].min(p[0]);bounds[1]=bounds[1].min(p[1]);bounds[2]=bounds[2].max(p[0]);bounds[3]=bounds[3].max(p[1]);}
    let width=(bounds[2]-bounds[0]).max(0.001);let height=(bounds[3]-bounds[1]).max(0.001);
    let ppu=(1024.0/width.max(height)).min(100.0/scale);
    let size=[(width*ppu).ceil().max(1.0) as usize,(height*ppu).ceil().max(1.0) as usize];
    let mut image=egui::ColorImage{size,pixels:vec![Color32::TRANSPARENT;size[0]*size[1]]};
    let mut approximate=false;let mut gpu=Vec::new();let mut gpu_valid=true;let mut gpu_vertices=0usize;
    for mesh in meshes {
        approximate |= mesh.blend_mode!=spine38::BlendMode::Normal || mesh.dark_color.r!=0.0 || mesh.dark_color.g!=0.0 || mesh.dark_color.b!=0.0;
        let Some(pointer)=mesh.attachment_renderer_object else {continue};
        let path=unsafe {&*pointer.cast::<PathBuf>()};
        let Some(texture)=images.get(path) else {continue};
        let c=mesh.color;
        gpu_vertices+=mesh.vertices.len();
        if gpu_vertices>1_000_000 {return Err("Spine GPU geometry exceeds 1,000,000 vertices".into());}
        if let Some(&slot)=page_slots.get(path).filter(|slot|**slot!=usize::MAX) {
            if mesh.vertices.len()!=mesh.uvs.len() || mesh.indices.iter().any(|i|*i as usize>=mesh.vertices.len()) {return Err("Invalid Spine GPU geometry".into());}
            let vertices=mesh.vertices.iter().zip(&mesh.uvs).map(|(p,uv)|crate::render_system::scene::Vertex{position:[p[0],p[1],0.0],uv:*uv,color:[c.r,c.g,c.b,c.a],normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]}).collect::<Vec<_>>();
            gpu.push(GpuPart{path:path.clone(),slot,vertices:Arc::from(vertices),indices:Arc::from(mesh.indices.iter().map(|i|*i as u32).collect::<Vec<_>>())});
        }else{gpu_valid=false;}
        let byte=|v:f32|(v.clamp(0.0,1.0)*255.0).round() as u8;
        let tint=crate::asset_viewer::preview_color([byte(c.r),byte(c.g),byte(c.b),byte(c.a)]);
        for triangle in mesh.indices.chunks_exact(3) {
            let indices=[triangle[0] as usize,triangle[1] as usize,triangle[2] as usize];
            let p=indices.map(|i|[(mesh.vertices[i][0]-bounds[0])*ppu,(bounds[1]+size[1] as f32/ppu-mesh.vertices[i][1])*ppu]);
            let uv=indices.map(|i|mesh.uvs[i]);
            let edge=|a:[f32;2],b:[f32;2],q:[f32;2]|(b[0]-a[0])*(q[1]-a[1])-(b[1]-a[1])*(q[0]-a[0]);
            let area=edge(p[0],p[1],p[2]);if area.abs()<1e-6 {continue;}
            let minx=p.iter().map(|p|p[0]).fold(f32::INFINITY,f32::min).floor().max(0.0) as usize;
            let maxx=p.iter().map(|p|p[0]).fold(f32::NEG_INFINITY,f32::max).ceil().max(0.0).min(size[0] as f32) as usize;
            let miny=p.iter().map(|p|p[1]).fold(f32::INFINITY,f32::min).floor().max(0.0) as usize;
            let maxy=p.iter().map(|p|p[1]).fold(f32::NEG_INFINITY,f32::max).ceil().max(0.0).min(size[1] as f32) as usize;
            for y in miny..maxy {
                if !crate::static_preview::checkpoint(cancelled,paused) {return Err("预览已取消".into());}
                for x in minx..maxx {
                    let q=[x as f32+0.5,y as f32+0.5];
                    let e=[edge(p[1],p[2],q),edge(p[2],p[0],q),edge(p[0],p[1],q)];
                    let sign=area.signum();
                    // Half-open edges prevent alpha seams where two triangles share an edge.
                    let edges=[(p[1],p[2]),(p[2],p[0]),(p[0],p[1])];
                    if (0..3).any(|i|{let v=e[i]*sign;let (mut a,mut b)=edges[i];if sign<0.0 {std::mem::swap(&mut a,&mut b);} v<0.0 || (v==0.0 && !(b[1]<a[1] || (b[1]==a[1] && b[0]>a[0])))}) {continue;}
                    let w=e.map(|v|v/area);
                    let u=(0..3).map(|i|uv[i][0]*w[i]).sum::<f32>();let v=(0..3).map(|i|uv[i][1]*w[i]).sum::<f32>();
                    let pixel=texture.get_pixel((u*texture.width() as f32).floor().clamp(0.0,(texture.width()-1) as f32) as u32,(v*texture.height() as f32).floor().clamp(0.0,(texture.height()-1) as f32) as u32);
                    let source=crate::asset_viewer::preview_color(pixel.0);let dest=image.pixels[y*size[0]+x];
                    image.pixels[y*size[0]+x]=crate::render_system::source_over(source,tint,dest);
                }
            }
        }
    }
    let region=crate::static_preview::Region{rect:[0.0,0.0,size[0] as f64,size[1] as f64],pivot:[-bounds[0] as f64*ppu as f64/size[0] as f64,-bounds[1] as f64*ppu as f64/size[1] as f64],ppu:ppu as f64,border:[0.0;4]};
    Ok((PathBuf::from(format!("{}.{}.spine-preview",data_path.display(),skin)),Baked{image,region,approximate,gpu:if gpu_valid && !approximate && !gpu.is_empty(){Some(Arc::new(gpu))}else{None}}))
}

/// Follow Unity's serialized SkeletonDataAsset reference using the existing project index.
pub fn component_button(ui: &mut egui::Ui, text: &str,documents:&[crate::Document]) {
    let data_reference = crate::static_preview::field(text,"skeletonDataAsset");
    let direct_reference = crate::static_preview::field(text,"skeletonJSON");
    if data_reference.is_none() && direct_reference.is_none() {return;}
    let catalog = ui.ctx().data_mut(|data|data.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
    let clicked=ui.add_enabled(catalog.is_some(),crate::theme::preview_navigation_button(">前往Spine预览")).on_hover_text("在独立只读页签预览骨骼与动画；不包含节点父级变换或 UI 遮罩").clicked();
    if clicked {
        let result = (|| -> Result<PathBuf,String> {
            let catalog = catalog.as_ref().ok_or("资源索引尚未完成")?;
            let resolve = |reference:&str| -> Result<PathBuf,String> {
                let guid = crate::static_preview::reference(reference,"guid").ok_or("骨骼引用缺少 GUID")?.to_ascii_lowercase();
                catalog.get(&guid).cloned().ok_or(format!("未找到骨骼引用 {guid}"))
            };
            if let Some(reference)=direct_reference {return resolve(reference);}
            resolve(data_reference.unwrap())
        })();
        match result {
            Ok(path)=>{
                let object=crate::static_preview::field(text,"m_GameObject").and_then(|v|crate::static_preview::reference(v,"fileID"));
                let materials=documents.iter().find(|d|d.type_name=="MeshRenderer" && d.game_object==object).and_then(|d|d.text.split_once("  m_Materials:").map(|(_,s)|s.lines().skip(1).take_while(|l|l.trim_start().starts_with('-')).collect::<Vec<_>>().join("\n")));
                let skin=crate::static_preview::field(text,"initialSkinName").unwrap_or("").trim_matches('"').trim_matches('\'').to_owned();
                ui.ctx().data_mut(|data|{data.insert_temp(egui::Id::new(("spine_skin",&path)),skin);data.insert_temp(egui::Id::new(("spine_materials",&path)),materials);data.insert_temp(egui::Id::new("spine_refresh"),path.clone());data.insert_temp(egui::Id::new("spine_window_request"),path);});
            },
            Err(error)=>ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new(("spine_reference_error",text)),error)),
        }
        ui.ctx().request_repaint();
    }
    if let Some(error)=ui.ctx().data_mut(|data|data.get_temp::<String>(egui::Id::new(("spine_reference_error",text)))) {ui.colored_label(Color32::RED,error);}
}

pub fn is_spine(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".skel") || name.ends_with(".skel.bytes") { return true; }
    if name.ends_with(".json") || name.ends_with(".json.txt") {
        use std::io::Read;
        let mut prefix = [0;4096];
        if let Ok(mut file) = std::fs::File::open(path) {
            let count = file.read(&mut prefix).unwrap_or(0);
            let header = String::from_utf8_lossy(&prefix[..count]);
            return header.contains("\"skeleton\"") && header.contains("\"spine\"");
        }
    }
    false
}

fn binary_string<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a str,String> {
    let mut length = 0usize;
    for shift in (0..35).step_by(7) {
        let byte = *bytes.get(*cursor).ok_or("骨骼文件头不完整")?;
        *cursor += 1;
        length |= usize::from(byte & 127) << shift;
        if byte & 128 == 0 {
            let count = length.saturating_sub(1);
            let end = cursor.checked_add(count).ok_or("骨骼文件头长度错误")?;
            let value = std::str::from_utf8(bytes.get(*cursor..end).ok_or("骨骼文件头不完整")?).map_err(|e|e.to_string())?;
            *cursor = end;
            return Ok(value);
        }
    }
    Err("骨骼文件头长度错误".into())
}

fn version(bytes: &[u8], json: bool) -> Result<String,String> {
    if json {
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
        return value["skeleton"]["spine"].as_str().map(str::to_owned).ok_or("JSON 缺少 skeleton.spine".into());
    }
    let mut cursor = 0;
    binary_string(bytes, &mut cursor)?;
    Ok(binary_string(bytes, &mut cursor)?.to_owned())
}

fn atlas_path(path: &Path) -> Result<PathBuf,String> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let base = [".skel.bytes", ".skel", ".json.txt", ".json"].iter().find_map(|suffix|name.strip_suffix(suffix)).ok_or("无法识别骨骼文件名")?;
    let directory = path.parent().ok_or("缺少资源目录")?;
    for suffix in [".atlas.txt", ".atlas"] {
        let candidate = directory.join(format!("{base}{suffix}"));
        if candidate.is_file() { return Ok(candidate); }
    }
    Err(format!("未找到同名图集：{base}.atlas.txt / {base}.atlas"))
}

pub struct Preview {
    gpu_resources:HashMap<PathBuf,(Arc<crate::render_system::scene::Material>,Arc<crate::render_system::scene::Texture>)>,
    gpu_error:Arc<std::sync::Mutex<Option<String>>>,
    gpu_status:Option<String>,
    controller: SkeletonController,
    textures: HashMap<PathBuf, egui::TextureHandle>,
    animations: Vec<(String,f32)>,
    skins: Vec<String>,
    animation: usize,
    base_track: usize,
    tracks: Vec<(usize,usize)>,
    skin: usize,
    dynamic: bool,
    playing: bool,
    looping: bool,
    speed: f32,
    time: f32,
    last: Instant,
    bounds: egui::Rect,
    zoom: f32,
    pan: egui::Vec2,
    background: String,
    background_rgb: [u8;3],
    show_guides: bool,
    error: Option<String>,
}

impl Preview {
    pub fn load_asset(ctx:&egui::Context,path:&Path,text:&str,catalog:&HashMap<String,PathBuf>) -> Result<Self,String> {
        use crate::static_preview::{field,reference};
        let resolve=|value:&str|reference(value,"guid").and_then(|guid|catalog.get(&guid.to_ascii_lowercase()).cloned()).ok_or("Spine 资源 GUID 未找到".to_owned());
        let skeleton=resolve(field(text,"skeletonJSON").ok_or("缺少 skeletonJSON")?)?;
        let atlas_reference=text.split_once("  atlasAssets:").and_then(|(_,tail)|tail.lines().find(|line|line.trim_start().starts_with("- {"))).ok_or("缺少 atlasAssets")?;
        let atlas_asset=resolve(atlas_reference.trim().trim_start_matches('-').trim())?;
        if std::fs::metadata(&atlas_asset).map_err(|e|e.to_string())?.len()>128*1024 {return Err("AtlasAsset 超过读取限制".into());}
        let atlas_source=std::fs::read_to_string(atlas_asset).map_err(|e|e.to_string())?;
        let atlas=resolve(field(&atlas_source,"atlasFile").ok_or("缺少 atlasFile")?)?;
        let mut preview=Self::load_with_atlas(ctx,&skeleton,Some(atlas))?;
        if let Some(skin)=ctx.data(|d|d.get_temp::<String>(egui::Id::new(("spine_skin",path)))).filter(|s|!s.is_empty()) {if let Some(index)=preview.skins.iter().position(|s|s==&skin){preview.skin=index;preview.controller.skeleton.set_skin_by_name(&skin).map_err(|e|e.to_string())?;preview.pose();preview.fit_bounds();}}
        let material_refs:Vec<_>=atlas_source.split_once("  materials:").map(|(_,s)|s.lines().skip(1).take_while(|l|l.trim_start().starts_with('-')).collect()).unwrap_or_default();
        let mut page_slots=HashMap::new();
        for (slot,reference) in material_refs.iter().enumerate() {if let Ok(material)=resolve(reference.trim().trim_start_matches('-').trim()) {let content=std::fs::read_to_string(material).unwrap_or_default();if let Some(tail)=content.split_once("- _MainTex:").map(|(_,s)|s){if let Some(reference)=tail.lines().find_map(|l|l.trim().strip_prefix("m_Texture:")){if let Ok(texture)=resolve(reference.trim()){if page_slots.insert(texture.clone(),slot).is_some(){page_slots.insert(texture,usize::MAX);}}}}}}
        let materials=ctx.data(|d|d.get_temp::<Option<String>>(egui::Id::new(("spine_materials",path)))).flatten().unwrap_or_else(||material_refs.join("\n"));
        let result=(||->Result<_,String>{
            let renderables=preview.controller.renderables();let mut parts=Vec::new();
            for mesh in renderables {
                if mesh.blend_mode!=spine38::BlendMode::Normal || mesh.dark_color.r!=0.0 || mesh.dark_color.g!=0.0 || mesh.dark_color.b!=0.0 {return Err("Spine special blend / two-color tint uses ordinary preview".into());}
                let pointer=mesh.attachment_renderer_object.ok_or("Missing Spine atlas page")?;let path=unsafe {&*pointer.cast::<PathBuf>()};
                let slot=*page_slots.get(path).filter(|slot|**slot!=usize::MAX).ok_or("Spine atlas material mapping unavailable or ambiguous")?;let c=mesh.color;
                let vertices=mesh.vertices.iter().zip(&mesh.uvs).map(|(p,uv)|crate::render_system::scene::Vertex{position:[p[0],p[1],0.0],uv:*uv,color:[c.r,c.g,c.b,c.a],normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]}).collect::<Vec<_>>();
                parts.push(GpuPart{path:path.clone(),slot,vertices:Arc::from(vertices),indices:Arc::from(mesh.indices.iter().map(|i|*i as u32).collect::<Vec<_>>())});
            }
            let paths=parts.iter().map(|p|p.path.clone()).collect::<Vec<_>>();
            let scene=crate::static_preview::world::spine_preview_resources(parts,&materials,catalog)?;
            if scene.draws.len()!=paths.len(){return Err("Spine GPU draw count mismatch".into());}
            Ok((scene,paths))
        })();
        match result {Ok((scene,paths))=>{for (draw,path) in scene.draws.iter().zip(paths){preview.gpu_resources.insert(path,(draw.material.clone(),draw.main_texture.clone()));}preview.gpu_status=None;},Err(error)=>preview.gpu_status=Some(error)}
        Ok(preview)
    }
    pub fn load(ctx: &egui::Context, path: &Path) -> Result<Self,String> {
        if let Some(catalog)=ctx.data(|d|d.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog"))) {
            let mut matching=Vec::new();
            for candidate in catalog.values().filter(|p|p.parent()==path.parent() && p.extension().is_some_and(|v|v=="asset")) {
                if std::fs::metadata(candidate).is_ok_and(|m|m.len()<=128*1024) {
                    if let Ok(text)=std::fs::read_to_string(candidate) {
                        let skeleton=crate::static_preview::field(&text,"skeletonJSON").and_then(|v|crate::static_preview::reference(v,"guid")).and_then(|g|catalog.get(&g.to_ascii_lowercase()));
                        if skeleton.is_some_and(|p|p==path){matching.push(text);}
                    }
                }
            }
            if matching.len()==1 {return Self::load_asset(ctx,path,&matching[0],&catalog);}
        }
        Self::load_with_atlas(ctx,path,None)
    }
    fn load_with_atlas(ctx: &egui::Context,path:&Path,atlas_file:Option<PathBuf>) -> Result<Self,String> {
        callbacks();
        if std::fs::metadata(path).map_err(|e|e.to_string())?.len() > 16*1024*1024 { return Err("骨骼文件超过 16 MiB 预览限制".into()); }
        let bytes = std::fs::read(path).map_err(|e|e.to_string())?;
        let json = bytes.iter().copied().find(|b| !b.is_ascii_whitespace()) == Some(b'{');
        let version = version(&bytes,json)?;
        if !version.starts_with("3.8.") { return Err(format!("仅支持 Spine 3.8，当前资源：{version}")); }
        let atlas_file = match atlas_file {Some(path)=>path,None=>atlas_path(path)?};
        let atlas = Arc::new(Atlas::new_from_file(&atlas_file).map_err(|e|e.to_string())?);
        let mut textures = HashMap::new();
        let mut total = 0usize;
        for page in atlas.pages() {
            // All page objects are allocated by the callbacks above and owned by this atlas.
            let texture_path = unsafe { page.renderer_object().get::<PathBuf>().cloned() }.ok_or("图集贴图路径缺失")?;
            let reader = image::ImageReader::open(&texture_path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
            let (w,h) = reader.into_dimensions().map_err(|e|e.to_string())?;
            if w > 8192 || h > 8192 { return Err("贴图尺寸超过 8192".into()); }
            total += w as usize*h as usize*4;
            if total > 256*1024*1024 { return Err("Spine 贴图超过 256 MiB 缓存限制".into()); }
            let image = image::open(&texture_path).map_err(|e|e.to_string())?.into_rgba8();
            let pixels = image.pixels().map(|p|crate::asset_viewer::preview_color(p.0)).collect();
            textures.insert(texture_path.clone(),ctx.load_texture(texture_path.display().to_string(),egui::ColorImage{size:[w as usize,h as usize],pixels},egui::TextureOptions::LINEAR));
        }
        let data = Arc::new(if json { SkeletonJson::new(atlas).read_skeleton_data(&bytes) } else { SkeletonBinary::new(atlas).read_skeleton_data(&bytes) }.map_err(|e|e.to_string())?);
        let animations = data.animations().map(|a|(a.name().to_owned(),a.duration())).collect();
        let skins: Vec<String> = data.skins().map(|s|s.name().to_owned()).collect();
        let state = Arc::new(AnimationStateData::new(data.clone()));
        let mut controller = SkeletonController::new(data,state);
        if let Some(name)=skins.first() {
            controller.skeleton.set_skin_by_name(name).map_err(|e|e.to_string())?;
            controller.skeleton.set_to_setup_pose();
            controller.update(0.0);
        }
        let mut preview = Self {gpu_resources:HashMap::new(),gpu_error:Arc::new(std::sync::Mutex::new(None)),gpu_status:Some("Spine material context unavailable; ordinary preview".into()),controller,textures,animations,skins,animation:0,base_track:0,tracks:Vec::new(),skin:0,dynamic:false,playing:false,looping:true,speed:1.0,time:0.0,last:Instant::now(),bounds:egui::Rect::NOTHING,zoom:1.0,pan:egui::Vec2::ZERO,background:String::new(),background_rgb:[190;3],show_guides:false,error:None };
        preview.fit_bounds();
        Ok(preview)
    }

    fn pose(&mut self) {
        self.controller.animation_state.clear_tracks();
        self.controller.skeleton.set_to_setup_pose();
        if self.dynamic {
            if let Some((name,_)) = self.animations.get(self.animation) {
                match self.controller.animation_state.set_animation_by_name(self.base_track,name,self.looping) {
                    Ok(mut entry) => { entry.set_mix_duration(0.0); entry.set_track_time(self.time); },
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            for (track,animation) in &self.tracks {
                if let Some((name,_))=self.animations.get(*animation) {
                    match self.controller.animation_state.set_animation_by_name(*track,name,self.looping) {
                        Ok(mut entry)=>{entry.set_mix_duration(0.0);entry.set_track_time(self.time);},
                        Err(e)=>self.error=Some(e.to_string()),
                    }
                }
            }
        }
        self.controller.update(0.0);
    }

    fn fit_bounds(&mut self) {
        let points: Vec<_> = self.controller.renderables().iter().flat_map(|r|r.vertices.iter().map(|v|egui::pos2(v[0],-v[1]))).collect();
        self.bounds = if points.is_empty() { egui::Rect::from_center_size(egui::Pos2::ZERO,egui::vec2(100.0,100.0)) } else { egui::Rect::from_points(&points) };
        self.zoom = 1.0; self.pan = egui::Vec2::ZERO;
    }

    fn duration(&self) -> f32 {
        std::iter::once(self.animation).chain(self.tracks.iter().map(|(_,animation)|*animation)).filter_map(|i|self.animations.get(i).map(|a|a.1)).fold(0.0,f32::max)
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let now = Instant::now();
        let delta = now.duration_since(self.last).as_secs_f32().min(0.1)*self.speed;
        self.last = now;
        let mut changed = false;
        let mut reload=false;
        ui.horizontal_wrapped(|ui| {
            changed|=crate::theme::preview_mode_selector(ui,&mut self.dynamic);
            if ui.button("适应窗口").clicked() {self.fit_bounds();}
            for (label,zoom) in [("50%",0.5),("100%",1.0),("200%",2.0)] {
                if ui.button(label).clicked() {self.zoom=zoom;self.pan=egui::Vec2::ZERO;}
            }
            ui.label("背景色");
            crate::theme::background_selector(ui,&mut self.background);
            ui.color_edit_button_srgb(&mut self.background_rgb);
            crate::theme::toggle_switch(ui,crate::localization::tr("辅助线"),&mut self.show_guides);
            ui.menu_button("设置",|ui| {
                changed|=ui.checkbox(&mut self.looping,"循环").changed();
                ui.add(egui::Slider::new(&mut self.speed,0.1..=3.0).text("速度"));
                egui::ComboBox::from_id_salt("spine_skin").selected_text(self.skins.get(self.skin).map_or("无皮肤",String::as_str)).show_ui(ui,|ui| {
                    for (index,name) in self.skins.iter().enumerate() {
                        if ui.selectable_value(&mut self.skin,index,name).changed() {
                            if let Err(e)=self.controller.skeleton.set_skin_by_name(name) {self.error=Some(e.to_string());}
                            changed=true;
                        }
                    }
                });
                if self.dynamic {
                    let duration=self.duration();
                    let mut displayed=if self.looping && duration>0.0 {self.time%duration} else {self.time};
                    if ui.add(egui::Slider::new(&mut displayed,0.0..=duration.max(0.001)).text("秒")).changed() {self.time=displayed;changed=true;}
                }
            });
            reload=ui.button(crate::localization::tr("重载")).clicked();
        });
        let (canvas,response)=ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)),egui::Sense::drag());
        let painter=ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        let scene_shape=painter.add(egui::Shape::Noop);
        let zoom_galley=painter.layout_no_wrap(format!("{:.0}%",self.zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE);
        let badge_size=egui::vec2(zoom_galley.size().x+50.0,zoom_galley.size().y+12.0);
        let badge=egui::Rect::from_center_size(egui::pos2(canvas.center().x,canvas.bottom()-8.0-badge_size.y*0.5),badge_size);
        let track_height=((self.tracks.len()+1) as f32*28.0+12.0).min(208.0).min((badge.top()-canvas.top()-56.0).max(1.0));
        let panel=egui::Rect::from_min_size(egui::pos2(canvas.left()+8.0,badge.top()-4.0-track_height),egui::vec2((canvas.width()-16.0).clamp(1.0,264.0),track_height));
        let frame=||egui::Frame::NONE.fill(Color32::from_black_alpha(128)).corner_radius(6.0).inner_margin(6.0);
        let mut track_ui=ui.new_child(egui::UiBuilder::new().id_salt("spine_tracks_overlay").max_rect(panel).layout(egui::Layout::top_down(egui::Align::Min)));
        // This overlay has a fixed dark backdrop; its inputs and popups must
        // use the same palette even when the surrounding app uses light mode.
        *track_ui.visuals_mut()=egui::Visuals::dark();
        track_ui.visuals_mut().override_text_color=Some(Color32::WHITE);
        track_ui.set_clip_rect(panel.intersect(canvas));
        frame().show(&mut track_ui,|ui| {
            ui.spacing_mut().item_spacing=egui::vec2(4.0,4.0);
            ui.set_min_width((panel.width()-12.0).max(1.0));
            ui.visuals_mut().override_text_color=Some(Color32::WHITE);
            egui::ScrollArea::vertical().id_salt("spine_tracks").max_height((track_height-12.0).max(1.0)).show(ui,|ui| {
                ui.add_enabled_ui(!self.animations.is_empty(),|ui| {
                    ui.visuals_mut().override_text_color=Some(Color32::WHITE);
                    let dropdown_width=(panel.width()-110.0).max(40.0);
                    let occupied:Vec<_>=self.tracks.iter().map(|t|t.0).collect();
                    ui.horizontal(|ui| {
                        ui.label("轨道");
                        let previous=self.base_track;
                        if ui.add_sized([28.0,24.0],egui::DragValue::new(&mut self.base_track).range(0..=31).speed(0.1)).on_hover_text("轨道编号（0～31，不可重复）").changed() {
                            if occupied.contains(&self.base_track) {self.base_track=previous;} else {changed=true;}
                        }
                        egui::ComboBox::from_id_salt("track_zero").width(dropdown_width).height(180.0).truncate().selected_text(self.animations.get(self.animation).map_or("无动画",|a|a.0.as_str())).show_ui(ui,|ui| {
                            for (index,(name,_)) in self.animations.iter().enumerate() {changed|=ui.selectable_value(&mut self.animation,index,name).changed();}
                        });
                        let next=self.tracks.iter().map(|t|t.0).max().unwrap_or(self.base_track).max(self.base_track)+1;
                        if ui.add_enabled(next<32,egui::Button::new("＋").min_size(egui::vec2(24.0,24.0))).on_hover_text("添加轨道").clicked() {self.tracks.push((next,0));changed=true;}
                    });
                    let mut remove=None;
                    for (track,animation) in &mut self.tracks {
                        ui.push_id(*track,|ui|ui.horizontal(|ui| {
                            ui.label("轨道");
                            let previous=*track;
                            if ui.add_sized([28.0,24.0],egui::DragValue::new(track).range(0..=31).speed(0.1)).on_hover_text("轨道编号（0～31，不可重复）").changed() {
                                if *track==self.base_track || occupied.iter().any(|i|*i==*track && *i!=previous) {*track=previous;} else {changed=true;}
                            }
                            egui::ComboBox::from_id_salt("track_animation").width(dropdown_width).height(180.0).truncate().selected_text(self.animations.get(*animation).map_or("无动画",|a|a.0.as_str())).show_ui(ui,|ui| {
                                for (index,(name,_)) in self.animations.iter().enumerate() {changed|=ui.selectable_value(animation,index,name).changed();}
                            });
                            if ui.add(egui::Button::new("－").min_size(egui::vec2(24.0,24.0))).on_hover_text("删除这条轨道").clicked() {remove=Some(*track);}
                        }));
                    }
                    if let Some(track)=remove {self.tracks.retain(|t|t.0!=track);changed=true;}
                });
            });
        });
        painter.rect_filled(badge,6.0,Color32::from_black_alpha(128));
        painter.galley(badge.min+egui::vec2(12.0,6.0),zoom_galley,Color32::WHITE);
        let enabled=self.dynamic && !self.animations.is_empty();
        let play_rect=egui::Rect::from_center_size(egui::pos2(badge.right()-20.0,badge.center().y),egui::vec2(24.0,badge.height()));
        let button=ui.interact(play_rect,ui.make_persistent_id("spine_playback_button"),if enabled {egui::Sense::click()} else {egui::Sense::hover()}).on_hover_text(if self.playing {"暂停"} else {"播放"});
        button.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,enabled,if self.playing {"暂停"} else {"播放"}));
        crate::icons::paint(ui,if self.playing {crate::icons::Icon::Pause} else {crate::icons::Icon::Play},egui::Rect::from_center_size(play_rect.center(),egui::vec2(22.0,22.0)),if enabled {Color32::WHITE} else {Color32::from_white_alpha(80)});
        if enabled && button.clicked() {self.playing=!self.playing;}
        if changed { self.pose(); }
        else if self.dynamic && self.playing {
            let duration = self.duration();
            let step = if self.looping {delta} else {delta.min((duration-self.time).max(0.0))};
            self.time += step;
            if !self.looping && self.time >= duration { self.playing = false; }
            self.controller.update(step);
        }
        if self.dynamic && self.playing { ui.ctx().request_repaint_after(std::time::Duration::from_millis(16)); }
        if let Some(error)=&self.error { painter.text(canvas.left_top()+egui::vec2(8.0,8.0),egui::Align2::LEFT_TOP,error,egui::FontId::proportional(13.0),Color32::RED); }
        let renderables = self.controller.renderables();
        if renderables.iter().any(|r|r.blend_mode != spine38::BlendMode::Normal || r.dark_color.r != 0.0 || r.dark_color.g != 0.0 || r.dark_color.b != 0.0) { painter.text(canvas.left_top()+egui::vec2(8.0,28.0),egui::Align2::LEFT_TOP,"特殊混合和双色染色为近似显示",egui::FontId::proportional(13.0),Color32::YELLOW); }
        let over_controls=ui.input(|i|i.pointer.hover_pos()).is_some_and(|p|panel.contains(p) || badge.contains(p));
        if response.dragged() && !over_controls { self.pan += response.drag_delta(); }
        if response.hovered() && !over_controls { self.zoom=(self.zoom*(ui.input(|i|i.smooth_scroll_delta.y)*0.002).exp()).clamp(0.01,100.0); }
        let scale=((canvas.width()-24.0).max(1.0)/self.bounds.width().max(1.0)).min((canvas.height()-24.0).max(1.0)/self.bounds.height().max(1.0))*self.zoom;
        let mut shapes=vec![crate::static_preview::background_shape(canvas,&self.background,self.background_rgb)];
        if self.show_guides {
            let origin=canvas.center()+self.pan+(egui::Pos2::ZERO-self.bounds.center())*scale;
            let color=if self.background=="gray" || self.background=="custom" {Color32::from_rgba_unmultiplied(128,128,128,100)} else {Color32::from_rgba_unmultiplied(100,100,100,170)};
            let stroke=egui::Stroke::new(1.0_f32,color);
            shapes.push(egui::Shape::line_segment([egui::pos2(canvas.left(),origin.y),egui::pos2(canvas.right(),origin.y)],stroke));
            shapes.push(egui::Shape::line_segment([egui::pos2(origin.x,canvas.top()),egui::pos2(origin.x,canvas.bottom())],stroke));
        }
        let gpu_background=shapes.clone();
        for renderable in renderables {
            let Some(pointer)=renderable.attachment_renderer_object else {continue};
            // Runtime returns the page renderer object allocated above; atlas outlives meshes.
            let path=unsafe {&*pointer.cast::<PathBuf>()};
            let Some(texture)=self.textures.get(path) else {continue};
            let mut mesh=egui::Mesh::with_texture(texture.id());
            let color=renderable.color;
            let byte=|v:f32|(v.clamp(0.0,1.0)*255.0).round() as u8;
            let tint=crate::asset_viewer::preview_color([byte(color.r),byte(color.g),byte(color.b),byte(color.a)]);
            for (vertex,uv) in renderable.vertices.iter().zip(&renderable.uvs) {
                mesh.vertices.push(egui::epaint::Vertex {pos:canvas.center()+self.pan+(egui::pos2(vertex[0],-vertex[1])-self.bounds.center())*scale,uv:egui::pos2(uv[0],uv[1]),color:tint});
            }
            mesh.indices=renderable.indices.iter().map(|i|u32::from(*i)).collect();
            shapes.push(egui::Shape::mesh(mesh));
        }
        painter.set(scene_shape,egui::Shape::Vec(shapes));
        let gpu_error=self.gpu_error.lock().unwrap().clone();
        if let Some(error)=gpu_error.as_ref().or(self.gpu_status.as_ref()){painter.text(canvas.left_top()+egui::vec2(8.0,46.0),egui::Align2::LEFT_TOP,error,egui::FontId::proportional(13.0),Color32::YELLOW);}
        if !self.gpu_resources.is_empty() && crate::render_system::available() && gpu_error.is_none() {
            let renderables=self.controller.renderables();let mut draws=Vec::new();let mut supported=true;let mut count=0usize;
            for mesh in renderables {
                count+=mesh.vertices.len();
                if count>1_000_000 || mesh.blend_mode!=spine38::BlendMode::Normal || mesh.dark_color.r!=0.0 || mesh.dark_color.g!=0.0 || mesh.dark_color.b!=0.0 {supported=false;break;}
                let Some(pointer)=mesh.attachment_renderer_object else {supported=false;break;};let path=unsafe {&*pointer.cast::<PathBuf>()};
                let Some((material,texture))=self.gpu_resources.get(path) else {supported=false;break;};
                if mesh.vertices.len()!=mesh.uvs.len() || mesh.indices.iter().any(|i|*i as usize>=mesh.vertices.len()) {supported=false;break;}
                let c=mesh.color;let color=if material.state.source==crate::render_system::scene::BlendFactor::One {[c.r*c.a,c.g*c.a,c.b*c.a,c.a]}else{[c.r,c.g,c.b,c.a]};
                let vertices=mesh.vertices.iter().zip(&mesh.uvs).map(|(p,uv)|crate::render_system::scene::Vertex{position:[p[0],p[1],0.0],uv:*uv,color,normal:[0.0,0.0,-1.0],tangent:[1.0,0.0,0.0,1.0]}).collect::<Vec<_>>();
                draws.push(crate::render_system::scene::Draw{strict_shader:false,component:Arc::from("spine-preview"),vertices:Arc::from(vertices),indices:Arc::from(mesh.indices.iter().map(|i|*i as u32).collect::<Vec<_>>()),object_to_world:[[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]],material:material.clone(),main_texture:texture.clone()});
            }
            if supported && !draws.is_empty() {
                let projection=[[2.0*scale/canvas.width(),0.0,0.0,2.0*(self.pan.x-self.bounds.center().x*scale)/canvas.width()],
                    [0.0,2.0*scale/canvas.height(),0.0,2.0*(-self.pan.y+self.bounds.center().y*scale)/canvas.height()],
                    [0.0,0.0,0.0,0.0],[0.0,0.0,0.0,1.0]];
                // Replace ordinary geometry, preserving the same background and guides.
                let mut shapes=gpu_background;
                shapes.push(egui::Shape::Callback(crate::render_system::callback(canvas,Arc::new(crate::render_system::scene::Scene{draws:Arc::from(draws),revision:1}),projection,self.gpu_error.clone(),true,0)));
                painter.set(scene_shape,egui::Shape::Vec(shapes));
            }else{painter.text(canvas.left_top()+egui::vec2(8.0,46.0),egui::Align2::LEFT_TOP,"Spine pose uses ordinary fallback (blend, tint, atlas page or geometry)",egui::FontId::proportional(13.0),Color32::YELLOW);}
        }
        reload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_binary_header_with_long_hash_and_rejects_truncation() {
        let mut bytes=vec![131,1]; // 130 bytes of hash, length stored as a varint plus one.
        bytes.extend(std::iter::repeat_n(b'a',130));
        bytes.push(7);
        bytes.extend_from_slice(b"3.8.95");
        assert_eq!(version(&bytes,false).unwrap(),"3.8.95");
        assert!(version(&bytes[..bytes.len()-1],false).is_err());
        assert!(version(&[255,255,255,255,255],false).is_err());
    }

    #[test]
    fn reads_json_version_and_rejects_unrelated_json() {
        assert_eq!(version(br#"{"skeleton":{"spine":"3.8.95"}}"#,true).unwrap(),"3.8.95");
        assert!(version(br#"{"skeleton":{}}"#,true).is_err());
        assert!(version(br#"{"settings":true}"#,true).is_err());
    }
}
