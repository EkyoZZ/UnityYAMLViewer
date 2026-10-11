//! Read-only static FBX/OBJ preview with bounded loading and depth-tested rendering.
use std::{path::{Path,PathBuf},sync::{Arc,mpsc},collections::HashMap};
use eframe::egui::{self,Color32};
pub(crate) fn is_model(path:&Path)->bool {path.extension().and_then(|s|s.to_str()).is_some_and(|s|matches!(s.to_ascii_lowercase().as_str(),"fbx"|"obj"))}
pub(crate) fn component_button(ui:&mut egui::Ui,text:&str) {
    let Some(reference)=crate::static_preview::field(text,"m_Mesh") else{return};
    let Some(guid)=crate::static_preview::reference(reference,"guid") else{return};
    let catalog=ui.ctx().data_mut(|d|d.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
    let Some(path)=catalog.and_then(|c|c.get(&guid.to_ascii_lowercase()).cloned()) else{return};
    if is_model(&path) && ui.add(crate::theme::preview_navigation_button("前往模型预览>")).clicked() {let materials=material_paths(text,&catalog_for(ui.ctx()));ui.ctx().data_mut(|d|{d.insert_temp(egui::Id::new(("model_materials",&path)),materials);d.insert_temp(egui::Id::new("model_refresh"),path.clone());d.insert_temp(egui::Id::new("spine_window_request"),path);});}
}
fn catalog_for(ctx:&egui::Context)->Arc<HashMap<String,PathBuf>> {ctx.data_mut(|d|d.get_temp(egui::Id::new("asset_catalog"))).unwrap_or_default()}
pub(crate) fn material_paths(text:&str,catalog:&HashMap<String,PathBuf>)->Vec<Option<PathBuf>> {
    text.split_once("  m_Materials:").map(|(_,s)|s.lines().skip(1).take_while(|l|l.trim_start().starts_with("- ")).map(|l|crate::static_preview::reference(l.trim().trim_start_matches('-').trim(),"guid").and_then(|g|catalog.get(&g.to_ascii_lowercase()).cloned())).collect()).unwrap_or_default()
}
#[derive(Clone)]
struct Triangle {points:[[f32;3];3],uv:[[f32;2];3],material:usize}
struct Material {texture:Option<image::RgbaImage>,scale:[f32;2],offset:[f32;2],color:[u8;4]}
impl Default for Material {fn default()->Self{Self{texture:None,scale:[1.0;2],offset:[0.0;2],color:[200,200,200,255]}}}
struct Model {triangles:Vec<Triangle>,materials:Vec<Material>,warnings:Vec<String>}
fn material(path:&Path,catalog:&HashMap<String,PathBuf>)->Result<Material,String> {
    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>1024*1024{return Err("材质超过 1 MiB".into());}
    let text=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
    let blocks:Vec<_>=["_BaseMap","_MainTex","_BaseColorMap"].into_iter().filter_map(|key|text.split_once(&format!("- {key}:" )).map(|(_,s)|s.split("\n    - ").next().unwrap_or(s))).collect();
    let block=blocks.iter().copied().find(|s|s.lines().any(|l|l.trim().strip_prefix("m_Texture:").and_then(|v|crate::static_preview::reference(v.trim(),"guid")).is_some())).or_else(||blocks.first().copied());
    let mut m=Material{color:[255;4],..Default::default()};
    for key in ["_BaseColor","_Color"] {if let Some(value)=text.lines().find_map(|l|l.trim().strip_prefix(&format!("- {key}:"))){if let Some(c)=crate::inline_vector(value.trim(),&["r","g","b","a"]){m.color=std::array::from_fn(|i|(c[i].clamp(0.0,1.0)*255.0).round() as u8);}break;}}
    if let Some(block)=block {
        let resolve=|key:&str|block.lines().find_map(|l|l.trim().strip_prefix(&format!("{key}:"))).map(str::trim);
        if let Some(g)=resolve("m_Texture").and_then(|v|crate::static_preview::reference(v,"guid")) {
            let p=catalog.get(&g.to_ascii_lowercase()).ok_or_else(||format!("贴图 GUID 未找到：{g}"))?;
            let mut r=image::ImageReader::open(p).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
            let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(64*1024*1024);r.limits(limits);
            m.texture=Some(r.decode().map_err(|e|e.to_string())?.to_rgba8());
        }
        for(key,target) in [("m_Scale",&mut m.scale),("m_Offset",&mut m.offset)] {if let Some(v)=resolve(key).and_then(|v|crate::inline_vector(v,&["x","y"])){*target=[v[0] as f32,v[1] as f32];}}
    }Ok(m)
}
fn unity_point(matrix:crate::static_preview::Matrix,p:ufbx::Vec3)->[f32;3] {
    let v=[-p.x,p.y,p.z,1.0];std::array::from_fn(|a|(0..4).map(|i|matrix.0[a][i]*v[i]).sum::<f64>() as f32)
}
fn load(path:&Path,paths:&[Option<PathBuf>],catalog:&HashMap<String,PathBuf>,pose:Option<(&HashMap<String,crate::static_preview::Matrix>,crate::static_preview::Matrix)>)->Result<Model,String> {
    let imported_paths=if paths.is_empty(){std::fs::read_to_string(path.with_extension(format!("{}.meta",path.extension().and_then(|s|s.to_str()).unwrap_or("")))).ok().map(|s|s.lines().filter_map(|l|l.trim().strip_prefix("second:").and_then(|v|crate::static_preview::reference(v.trim(),"guid")).map(|g|catalog.get(&g.to_ascii_lowercase()).cloned())).collect::<Vec<_>>()).unwrap_or_default()}else{Vec::new()};
    let paths=if paths.is_empty(){imported_paths.as_slice()}else{paths};
    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>128*1024*1024 {return Err("模型文件超过 128 MiB".into());}
    let mut opts=ufbx::LoadOpts::default();opts.ignore_animation=true;opts.ignore_embedded=true;
    opts.temp_allocator.memory_limit=256*1024*1024;opts.result_allocator.memory_limit=256*1024*1024;
    let scene=ufbx::load_file(path.to_str().ok_or("模型路径无法转换为 UTF-8")?,opts).map_err(|e|format!("模型读取失败：{e:?}"))?;
    let mut triangles=Vec::new();
    for node in &scene.nodes {
        let Some(mesh)=node.mesh.as_ref() else{continue};
        let posed=if let Some((bones,renderer_matrix))=pose {
            let mut vertices=Vec::with_capacity(mesh.vertices.len());
            let skin=mesh.skin_deformers.first();
            for (index,p) in mesh.vertices.iter().enumerate() {
                let mut result=[0.0f32;3];let mut total=0.0;
                if let Some(skin)=skin {
                    if let Some(vertex)=skin.vertices.get(index) {
                        for weight_index in vertex.weight_begin..vertex.weight_begin+vertex.num_weights {
                            let weight=&skin.weights[weight_index as usize];
                            if weight.weight<=0.0 {continue;}
                            let cluster=&skin.clusters[weight.cluster_index as usize];
                            let bone=cluster.bone_node.as_ref().ok_or("蒙皮绑定缺少骨骼节点")?;
                            let name=bone.element.name.as_ref();
                            let matrix=bones.get(name).ok_or_else(||format!("Prefab 骨骼未找到或名称重复：{name}"))?;
                            let local=ufbx::transform_position(&cluster.geometry_to_bone,*p);
                            let world=unity_point(*matrix,local);
                            for a in 0..3 {result[a]+=world[a]*weight.weight as f32;}total+=weight.weight as f32;
                        }
                    }
                }
                if total>0.0 {for v in &mut result{*v/=total;}}else{result=unity_point(renderer_matrix,ufbx::transform_position(&node.geometry_to_node,*p));}
                vertices.push(result);
            }Some(vertices)
        }else{None};
        if mesh.max_face_triangles>1_000_000 {return Err("模型面数超过限制".into());}
        let mut indices=vec![0;mesh.max_face_triangles*3];
        for (face_index,face) in mesh.faces.iter().enumerate() {
            let n=mesh.triangulate_face(&mut indices,*face) as usize;
            if triangles.len()+n>500_000 {return Err("模型三角形超过 500,000".into());}
            for t in indices[..n*3].chunks_exact(3) {
                let p=[t[0],t[1],t[2]].map(|i|{if let Some(vertices)=&posed{vertices[mesh.vertex_indices[i as usize] as usize]}else{let p=ufbx::transform_position(&node.geometry_to_world,mesh.vertex_position[i as usize]);[p.x as f32,p.y as f32,p.z as f32]}});
                if p.iter().flatten().any(|v|!v.is_finite()) {return Err("模型包含无效顶点".into());}
                let uv=[t[0],t[1],t[2]].map(|i|if mesh.vertex_uv.exists {let v=mesh.vertex_uv[i as usize];[v.x as f32,v.y as f32]}else{[0.0;2]});
                if uv.iter().flatten().any(|v|!v.is_finite()){return Err("模型包含无效 UV".into());}
                triangles.push(Triangle{points:p,uv,material:mesh.face_material.get(face_index).copied().unwrap_or(0) as usize});
            }
        }
    }
    if triangles.is_empty(){return Err("模型没有可绘制网格".into());}
    let mut warnings=Vec::new();let mut bytes=0usize;
    if paths.is_empty(){warnings.push("没有解析到 Unity 材质引用，使用无光照灰色显示".into());}
    let materials=paths.iter().map(|p|{let result=p.as_ref().ok_or("材质引用未找到".to_owned()).and_then(|p|material(p,catalog));match result {Ok(m)=>{bytes+=m.texture.as_ref().map_or(0,|t|t.as_raw().len());if bytes>128*1024*1024 {warnings.push("模型贴图总量超过 128 MiB".into());Material::default()}else{m}},Err(e)=>{warnings.push(e);Material::default()}}}).collect();
    Ok(Model{triangles,materials,warnings})
}
pub(crate) struct Preview {
    pending:Option<mpsc::Receiver<Result<Model,String>>>,model:Option<Model>,error:Option<String>,
    texture:Option<egui::TextureHandle>,yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,background:String,rgb:[u8;3],dirty:bool,size:[usize;2],
}
impl Preview {
    pub(crate) fn new(ctx:&egui::Context,path:&Path)->Self {
        let catalog=catalog_for(ctx);let paths=ctx.data_mut(|d|d.get_temp::<Vec<Option<PathBuf>>>(egui::Id::new(("model_materials",path)))).unwrap_or_default();
        let (send,recv)=mpsc::channel();let path=path.to_owned();let ctx=ctx.clone();
        std::thread::spawn(move||{let result=load(&path,&paths,&catalog,None).map(|mut m|{normalize(&mut m.triangles);m});let _=send.send(result);ctx.request_repaint();});
        Self{pending:Some(recv),model:None,error:None,texture:None,yaw:0.3,pitch:0.15,zoom:1.0,pan:egui::Vec2::ZERO,background:"checker".into(),rgb:[60;3],dirty:true,size:[0;2]}
    }
    pub(crate) fn show(&mut self,ui:&mut egui::Ui)->bool {
        let mut reload=false;
        ui.horizontal_wrapped(|ui|{
            ui.label("静态模型");if ui.button("适应窗口").clicked(){self.zoom=1.0;self.pan=egui::Vec2::ZERO;self.dirty=true;}
            if ui.button("重置视角").clicked(){self.yaw=0.3;self.pitch=0.15;self.dirty=true;}
            crate::theme::model_background_selector(ui,&mut self.background);
            reload=ui.button("重载").clicked();
        });
        ui.small("左键旋转 · 右键平移 · 滚轮缩放；无光照主贴图预览，暂不还原 Shader、骨骼动画及 Prefab 骨骼姿势。FBX 多网格显示整个模型。");
        if let Some(result)=self.pending.as_ref().and_then(|r|r.try_recv().ok()){self.pending=None;match result {Ok(m)=>self.model=Some(m),Err(e)=>self.error=Some(e)}}
        if self.pending.is_some(){ui.spinner();return reload;}if let Some(e)=&self.error{ui.colored_label(Color32::RED,e);return reload;}
        let Some(model)=&self.model else{return reload};for w in &model.warnings{ui.colored_label(ui.visuals().warn_fg_color,w);}
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)),egui::Sense::drag());
        if response.dragged_by(egui::PointerButton::Primary){let d=ui.input(|i|i.pointer.delta());self.yaw+=d.x*0.01;self.pitch=(self.pitch+d.y*0.01).clamp(-1.5,1.5);self.dirty=true;}
        if response.dragged_by(egui::PointerButton::Secondary){self.pan+=ui.input(|i|i.pointer.delta());self.dirty=true;}
        if response.hovered(){let scroll=ui.input(|i|i.smooth_scroll_delta.y);if scroll!=0.0{self.zoom=(self.zoom*(scroll*0.002).exp()).clamp(0.05,20.0);self.dirty=true;}}
        let size=viewport_size(rect.size());
        ui.painter().add(crate::static_preview::background_shape(rect,&self.background,self.rgb));
        if self.dirty||size!=self.size {
            let pan=egui::vec2(self.pan.x*size[0] as f32/rect.width(),self.pan.y*size[1] as f32/rect.height());
            let image=render(model,size,self.yaw,self.pitch,self.zoom,pan);
            if let Some(t)=&mut self.texture{t.set(image,egui::TextureOptions::LINEAR);}else{self.texture=Some(ui.ctx().load_texture("model-static",image,egui::TextureOptions::LINEAR));}
            self.size=size;self.dirty=false;
        }
        if let Some(t)=&self.texture{ui.painter().image(t.id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);}
        let painter=ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let galley=painter.layout_no_wrap(format!("{:.0}%",self.zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE);
        let badge_size=galley.size()+egui::vec2(24.0,12.0);
        let badge=egui::Rect::from_center_size(egui::pos2(rect.center().x,rect.bottom()-8.0-badge_size.y*0.5),badge_size);
        painter.rect_filled(badge,6.0,Color32::from_black_alpha(128));
        painter.galley(badge.min+egui::vec2(12.0,6.0),galley,Color32::WHITE);
        reload
    }
}
fn viewport_size(viewport:egui::Vec2)->[usize;2] {
    let factor=(1024.0/viewport.x.max(viewport.y).max(1.0)).min(1.0);
    [(viewport.x*factor).round().max(1.0) as usize,(viewport.y*factor).round().max(1.0) as usize]
}
fn normalize(triangles:&mut [Triangle]) {
    for p in triangles.iter_mut().flat_map(|t|&mut t.points){p[0]=-p[0];}
    let mut lo=[f32::INFINITY;3];let mut hi=[f32::NEG_INFINITY;3];for p in triangles.iter().flat_map(|t|&t.points){for a in 0..3 {lo[a]=lo[a].min(p[a]);hi[a]=hi[a].max(p[a]);}}
    let scale=(0..3).map(|a|hi[a]-lo[a]).fold(0.0,f32::max).max(1e-6);for p in triangles.iter_mut().flat_map(|t|&mut t.points){for a in 0..3 {p[a]=(p[a]-(lo[a]+hi[a])*0.5)/scale;}}
}
fn sample(material:Option<&Material>,uv:[f32;2])->Color32 {
    let Some(m)=material else{return Color32::from_gray(200)};
    let mut pixel=[255;4];
    if let Some(t)=&m.texture {let u=(uv[0]*m.scale[0]+m.offset[0]).rem_euclid(1.0);let v=(uv[1]*m.scale[1]+m.offset[1]).rem_euclid(1.0);let x=(u*t.width() as f32).floor() as u32;let y=((1.0-v)*t.height() as f32).floor() as u32;pixel=t.get_pixel(x.min(t.width()-1),y.min(t.height()-1)).0;}
    crate::asset_viewer::preview_color(std::array::from_fn(|i|((pixel[i] as u16*m.color[i] as u16+127)/255) as u8))
}
pub(crate) fn bake(mesh:&str,renderer:&str,catalog:&HashMap<String,PathBuf>,matrix:crate::static_preview::Matrix,bones:&HashMap<String,crate::static_preview::Matrix>)->Result<(crate::spine_preview::Baked,Vec<String>),String> {
    let path=crate::static_preview::reference(mesh,"guid").and_then(|g|catalog.get(&g.to_ascii_lowercase())).ok_or("模型 GUID 未找到")?;
    if !is_model(path){return Err("暂不支持此 Mesh 资源格式".into());}
    // Prefab supplies node transforms: never apply the FBX hierarchy a second time.
    let mut model=load(path,&material_paths(renderer,catalog),catalog,Some((bones,matrix)))?;
    if model.triangles.iter().flat_map(|t|&t.points).flatten().any(|v|!v.is_finite()){return Err("模型变换包含无效坐标".into());}
    let mut lo=[f32::INFINITY;2];let mut hi=[f32::NEG_INFINITY;2];for p in model.triangles.iter().flat_map(|t|&t.points){for a in 0..2{lo[a]=lo[a].min(p[a]);hi[a]=hi[a].max(p[a]);}}
    let extent=(hi[0]-lo[0]).max(hi[1]-lo[1]).max(0.001);let ppu=1024.0/extent;
    let size=[((hi[0]-lo[0])*ppu).ceil().max(1.0) as usize,((hi[1]-lo[1])*ppu).ceil().max(1.0) as usize];
    for p in model.triangles.iter_mut().flat_map(|t|&mut t.points){p[0]=(p[0]-(lo[0]+hi[0])*0.5)/extent;p[1]=(p[1]-(lo[1]+hi[1])*0.5)/extent;}
    let zoom=1024.0/(size[0].min(size[1]) as f32*0.75);
    let image=render_direction(&model,size,0.0,0.0,zoom,egui::Vec2::ZERO,-1.0);
    let region=crate::static_preview::Region{rect:[0.0,0.0,size[0] as f64,size[1] as f64],pivot:[-(lo[0] as f64)*ppu as f64/size[0] as f64,-(lo[1] as f64)*ppu as f64/size[1] as f64],ppu:ppu as f64,border:[0.0;4]};
    Ok((crate::spine_preview::Baked{image,region,approximate:true},model.warnings))
}
fn render(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2)->egui::ColorImage {
    render_direction(model,size,yaw,pitch,zoom,pan,1.0)
}
fn render_direction(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,depth_direction:f32)->egui::ColorImage {
    let mut image=egui::ColorImage{size,pixels:vec![Color32::TRANSPARENT;size[0]*size[1]]};let mut depth=vec![f32::NEG_INFINITY;image.pixels.len()];
    let scale=size[0].min(size[1]) as f32*0.75*zoom;
    for t in &model.triangles {
        let p=t.points.map(|p|{let x=p[0]*yaw.cos()+p[2]*yaw.sin();let z=-p[0]*yaw.sin()+p[2]*yaw.cos();let y=p[1]*pitch.cos()-z*pitch.sin();let z=p[1]*pitch.sin()+z*pitch.cos();[size[0] as f32*0.5+x*scale+pan.x,size[1] as f32*0.5-y*scale+pan.y,z*depth_direction]});
        let edge=|a:[f32;3],b:[f32;3],x:f32,y:f32|(b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]);let area=edge(p[0],p[1],p[2][0],p[2][1]);if area.abs()<1e-6{continue;}
        let bounds=|axis:usize,max:usize|{let lo=p.iter().map(|p|p[axis]).fold(f32::INFINITY,f32::min).floor().clamp(0.0,max as f32) as usize;let hi=p.iter().map(|p|p[axis]).fold(f32::NEG_INFINITY,f32::max).ceil().clamp(0.0,max as f32) as usize;(lo,hi)};let (x0,x1)=bounds(0,size[0]);let(y0,y1)=bounds(1,size[1]);
        for y in y0..y1 {for x in x0..x1 {let w=[edge(p[1],p[2],x as f32+0.5,y as f32+0.5)/area,edge(p[2],p[0],x as f32+0.5,y as f32+0.5)/area,edge(p[0],p[1],x as f32+0.5,y as f32+0.5)/area];if w.iter().any(|v|*v<0.0){continue;}let z=(0..3).map(|i|w[i]*p[i][2]).sum::<f32>();let i=y*size[0]+x;if z>depth[i]{let uv=std::array::from_fn(|a|(0..3).map(|j|w[j]*t.uv[j][a]).sum());let c=sample(model.materials.get(t.material),uv);if c.a()==0{continue;}depth[i]=z;image.pixels[i]=c;}}}
    }image
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_limit_preserves_wide_and_tall_aspect_ratios() {
        for viewport in [egui::vec2(1360.0,600.0),egui::vec2(600.0,1360.0),egui::vec2(2000.0,1800.0),egui::vec2(750.0,600.0)] {
            let size=viewport_size(viewport);
            assert!(size[0]<=1024 && size[1]<=1024);
            let scale_x=size[0] as f32/viewport.x;let scale_y=size[1] as f32/viewport.y;
            assert!((scale_x-scale_y).abs()<=0.5/viewport.x+0.5/viewport.y);
        }
        assert_eq!(viewport_size(egui::vec2(750.0,600.0)),[750,600]);
    }
    #[test]
    fn depth_buffer_is_independent_of_triangle_order() {
        let back=[[-0.5,-0.5,-0.2],[0.5,-0.5,-0.2],[0.0,0.5,-0.2]];
        let front=[[-0.5,-0.5,0.0],[0.5,-0.5,0.4],[0.0,0.5,0.2]];
        let triangle=|points|Triangle{points,uv:[[0.0;2];3],material:0};
        let a=render(&Model{triangles:vec![triangle(back),triangle(front)],materials:vec![],warnings:vec![]},[32,32],0.0,0.0,1.0,egui::Vec2::ZERO);
        let b=render(&Model{triangles:vec![triangle(front),triangle(back)],materials:vec![],warnings:vec![]},[32,32],0.0,0.0,1.0,egui::Vec2::ZERO);
        assert_eq!(a.pixels,b.pixels);
        assert_ne!(a.pixels[16*32+16],Color32::TRANSPARENT);
    }
    #[test]
    fn recognizes_case_insensitive_model_extensions() {
        assert!(is_model(Path::new("arlo.FBX")));
        assert!(is_model(Path::new("mesh.obj")));
        assert!(!is_model(Path::new("mesh.asset")));
    }
    #[test]
    fn unlit_sampling_uses_uv_tint_and_ignores_transparent_depth() {
        let texture=image::RgbaImage::from_raw(1,2,vec![255,0,0,255,0,255,0,255]).unwrap();
        let m=Material{texture:Some(texture),color:[128,255,255,255],..Default::default()};
        assert_eq!(sample(Some(&m),[0.5,0.75]),Color32::from_rgb(128,0,0));
        assert_eq!(sample(Some(&m),[0.5,0.25]),Color32::from_rgb(0,255,0));
        let triangle=|z,material|Triangle{points:[[-0.5,-0.5,z],[0.5,-0.5,z],[0.0,0.5,z]],uv:[[0.5;2];3],material};
        let model=Model{triangles:vec![triangle(0.4,0),triangle(0.0,1)],materials:vec![Material{color:[0;4],..Default::default()},Material{color:[255,0,0,255],..Default::default()}],warnings:vec![]};
        assert_eq!(render(&model,[32,32],0.0,0.0,1.0,egui::Vec2::ZERO).pixels[16*32+16],Color32::RED);
    }
    #[test]
    fn renderer_material_list_preserves_empty_slots() {
        let catalog=HashMap::from([("abc".into(),PathBuf::from("sample.mat"))]);
        assert_eq!(material_paths("  m_Materials:\n  - {fileID: 0}\n  - {fileID: 2100000, guid: abc, type: 2}\n  m_Mesh: {fileID: 0}",&catalog),vec![None,Some(PathBuf::from("sample.mat"))]);
    }
    #[test]
    fn bone_point_applies_handedness_rotation_and_translation_once() {
        let matrix=crate::static_preview::Matrix([[1.0,0.0,0.0,2.0],[0.0,0.0,1.0,3.0],[0.0,-1.0,0.0,4.0],[0.0,0.0,0.0,1.0]]);
        assert_eq!(unity_point(matrix,ufbx::Vec3{x:1.0,y:2.0,z:5.0}),[1.0,8.0,2.0]);
    }
    #[test]
    fn scene_view_prefers_negative_z_surface() {
        let triangle=|z,material|Triangle{points:[[-0.5,-0.5,z],[0.5,-0.5,z],[0.0,0.5,z]],uv:[[0.0;2];3],material};
        let model=Model{triangles:vec![triangle(-0.2,0),triangle(0.2,1)],materials:vec![Material{color:[255,0,0,255],..Default::default()},Material{color:[0,255,0,255],..Default::default()}],warnings:vec![]};
        assert_eq!(render_direction(&model,[32,32],0.0,0.0,1.0,egui::Vec2::ZERO,-1.0).pixels[16*32+16],Color32::RED);
    }
}
