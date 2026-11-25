//! Read-only static FBX/OBJ preview with bounded loading and depth-tested rendering.
use std::{path::{Path,PathBuf},sync::{Arc,mpsc},collections::HashMap};
use eframe::egui::{self,Color32};
pub(crate) fn is_model(path:&Path)->bool {path.extension().and_then(|s|s.to_str()).is_some_and(|s|matches!(s.to_ascii_lowercase().as_str(),"fbx"|"obj"))}
pub(crate) fn component_button(ui:&mut egui::Ui,text:&str) {
    let Some(reference)=crate::static_preview::field(text,"m_Mesh") else{return};
    let Some(guid)=crate::static_preview::reference(reference,"guid") else{return};
    let catalog=ui.ctx().data_mut(|d|d.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
    let Some(path)=catalog.and_then(|c|c.get(&guid.to_ascii_lowercase()).cloned()) else{return};
    if is_model(&path) && ui.add(crate::theme::preview_navigation_button(">前往模型预览")).clicked() {let materials=material_paths(text,&catalog_for(ui.ctx()));ui.ctx().data_mut(|d|{d.insert_temp(egui::Id::new(("model_materials",&path)),materials);d.insert_temp(egui::Id::new("model_refresh"),path.clone());d.insert_temp(egui::Id::new("spine_window_request"),path);});}
}
fn catalog_for(ctx:&egui::Context)->Arc<HashMap<String,PathBuf>> {ctx.data_mut(|d|d.get_temp(egui::Id::new("asset_catalog"))).unwrap_or_default()}
pub(crate) fn material_paths(text:&str,catalog:&HashMap<String,PathBuf>)->Vec<Option<PathBuf>> {
    text.split_once("  m_Materials:").map(|(_,s)|s.lines().skip(1).take_while(|l|l.trim_start().starts_with("- ")).map(|l|crate::static_preview::reference(l.trim().trim_start_matches('-').trim(),"guid").and_then(|g|catalog.get(&g.to_ascii_lowercase()).cloned())).collect()).unwrap_or_default()
}
#[derive(Clone)]
struct Triangle {points:[[f32;3];3],uv:[[f32;2];3],material:usize}
#[derive(Clone)]
struct Material {texture:Option<Arc<image::RgbaImage>>,scale:[f32;2],offset:[f32;2],color:[u8;4],pf:Option<PfColor>,sorting:(i64,i64),gpu_texture:Arc<std::sync::OnceLock<Arc<crate::render_system::scene::Texture>>>}
#[derive(Clone)]
struct PfColor {main:[f32;4],tint:[f32;4],ambient:[f32;4],overlay:[f32;4],alpha:f32,brightness:f32,saturation:f32,contrast:f32}
impl Default for Material {fn default()->Self{Self{texture:None,scale:[1.0;2],offset:[0.0;2],color:[200,200,200,255],pf:None,sorting:(0,0),gpu_texture:Default::default()}}}
struct Model {vertex_count:usize,triangles:Vec<Triangle>,materials:Vec<Material>,warnings:Vec<String>}
pub(crate) struct ScenePlane {pub texture:Arc<crate::render_system::scene::Texture>,pub points:[[f32;3];4],pub uv:[[f32;2];4],pub tint:Color32,pub sorting:(i64,i64)}
pub(crate) struct SceneParticle {
    pub emitter: crate::particle_preview::Emitter,
    pub template: crate::render_system::scene::Draw,
    pub sorting: (i64, i64),
    pub origin: [f32; 3],
}

fn append_model(target:&mut Model,mut other:Model)->Result<(),String> {
    if target.triangles.len()+other.triangles.len()>500_000{return Err("3D 合成三角面超过 500,000".into());}
    let bytes=target.materials.iter().chain(&other.materials).map(|m|m.texture.as_ref().map_or(0,|t|t.as_raw().len())).sum::<usize>();
    if bytes>256*1024*1024{return Err("3D 合成贴图超过 256 MiB".into());}
    let offset=target.materials.len();if other.materials.is_empty(){other.materials.push(Material::default());}
    for t in &mut other.triangles{t.material=offset+t.material.min(other.materials.len()-1);}
    target.vertex_count+=other.vertex_count;target.triangles.extend(other.triangles);target.materials.extend(other.materials);target.warnings.extend(other.warnings);Ok(())
}
#[derive(Default)]
struct Resources {
    fbx: HashMap<(PathBuf, bool), (Arc<ufbx::SceneRoot>, f64)>,
    materials: HashMap<PathBuf, Material>,
    planes: crate::static_preview::ScenePlaneCache,
    mesh_bytes: u64,
    texture_bytes: usize,
}

impl Resources {
    fn fbx(&mut self, path: &Path, skin: bool) -> Result<(Arc<ufbx::SceneRoot>, f64), String> {
        let key = (path.to_path_buf(), skin);
        if let Some(asset) = self.fbx.get(&key) { return Ok(asset.clone()); }
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > 128 * 1024 * 1024 { return Err("模型文件超过 128 MiB".into()); }
        let mut options = ufbx::LoadOpts::default();
        options.ignore_animation = true;
        options.ignore_embedded = true;
        options.evaluate_skinning = skin;
        options.temp_allocator.memory_limit = 256 * 1024 * 1024;
        options.result_allocator.memory_limit = 256 * 1024 * 1024;
        let scene = Arc::new(ufbx::load_file(path.to_str().ok_or("模型路径无法转换为 UTF-8")?, options)
            .map_err(|error| format!("模型读取失败：{error:?}"))?);
        let meta = std::fs::read_to_string(format!("{}.meta", path.display())).unwrap_or_default();
        let scale = importer_scale(&meta, scene.settings.unit_meters)?;
        if self.fbx.len() < 16 && self.mesh_bytes + size <= 128 * 1024 * 1024 {
            self.fbx.insert(key, (scene.clone(), scale));
            self.mesh_bytes += size;
        }
        Ok((scene, scale))
    }

    fn material(&mut self, path: &Path, catalog: &HashMap<String, PathBuf>) -> Result<Material, String> {
        if let Some(material) = self.materials.get(path) { return Ok(material.clone()); }
        let material = material(path, catalog)?;
        let size = material.texture.as_ref().map_or(0, |image| image.as_raw().len());
        if self.materials.len() < 256 && self.texture_bytes + size <= 128 * 1024 * 1024 {
            self.materials.insert(path.to_path_buf(), material.clone());
            self.texture_bytes += size;
        }
        Ok(material)
    }
}

fn scene_model(documents:&[crate::Document],flags:&HashMap<String,crate::hierarchy_controls::Flags>,catalog:&Arc<HashMap<String,PathBuf>>, resources: &mut Resources)->Result<PreparedModel,String> {
    use crate::static_preview::{field,number,reference,world};
    let objects:HashMap<_,_>=documents.iter().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
    let transforms:HashMap<_,_>=documents.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
    let mut model=Model{vertex_count:0,triangles:Vec::new(),materials:Vec::new(),warnings:Vec::new()};
    for doc in documents.iter().filter(|d|matches!(d.type_name.as_str(),"MeshFilter"|"SkinnedMeshRenderer")) {
        let Some(object)=doc.game_object.as_deref() else{continue};if flags.get(object).is_some_and(|f|!f.visible){continue;}
        let renderer=if doc.type_name=="SkinnedMeshRenderer"{Some(doc)}else{documents.iter().find(|d|d.type_name=="MeshRenderer" && d.game_object==doc.game_object)};
        let Some(renderer)=renderer else{continue};if number(&renderer.text,"m_Enabled",1.0)==0.0{continue;}
        let Some(mesh)=field(&doc.text,"m_Mesh").filter(|v|reference(v,"fileID").is_some_and(|id|id!="0")) else{continue};
        let Some(transform)=transforms.values().find(|t|t.game_object==doc.game_object) else{continue};
        let Some(matrix)=world(&transform.file_id,&transforms,&objects,&mut Default::default()) else{continue};
        let mut bones=HashMap::new();let mut duplicate=std::collections::HashSet::new();
        if doc.type_name=="SkinnedMeshRenderer" {if let Some((_,tail))=doc.text.split_once("  m_Bones:"){for line in tail.lines().skip(1).take_while(|l|l.trim_start().starts_with("- ")) {
            let Some(id)=reference(line.trim().trim_start_matches('-').trim(),"fileID") else{continue};
            let Some(t)=transforms.get(id.as_str()) else{continue};let Some(name)=t.game_object.as_deref().and_then(|id|objects.get(id)).and_then(|d|crate::name_from_text(&d.text)) else{continue};
            if let Some(m)=world(&id,&transforms,&objects,&mut Default::default()){if bones.insert(name.clone(),m).is_some(){duplicate.insert(name);}}
        }}}
        for name in duplicate{bones.remove(&name);}
        let local=if reference(mesh,"guid").is_none() {reference(mesh,"fileID").and_then(|id|documents.iter().find(|d|d.file_id==id && d.type_name=="Mesh")).map(|d|d.text.as_ref())}else{None};
        match component_model_cached(mesh,&renderer.text,catalog,matrix,&bones,local,resources){Ok(mut mesh_model) => {
            let sorting = crate::static_preview::renderer_sorting(renderer);
            if mesh_model.materials.is_empty() { mesh_model.materials.push(Material::default()); }
            for material in &mut mesh_model.materials { material.sorting = sorting; }
            append_model(&mut model, mesh_model)?;
        }, Err(e)=>model.warnings.push(format!("{}: {e}",doc.file_id))}
    }
    let (planes,particles,warnings)=crate::static_preview::scene_planes(documents,flags,catalog,&mut resources.planes)?;model.warnings.extend(warnings);
    for plane in planes {
        let triangles=[[0,1,2],[0,2,3]].map(|i|Triangle{points:i.map(|j|plane.points[j]),uv:i.map(|j|plane.uv[j]),material:0}).to_vec();
        append_model(&mut model,Model{vertex_count:4,triangles,materials:vec![Material{gpu_texture:Arc::new(std::sync::OnceLock::from(plane.texture)),color:plane.tint.to_srgba_unmultiplied(),sorting:plane.sorting,..Default::default()}],warnings:Vec::new()})?;
    }
    if model.triangles.is_empty() && particles.is_empty(){return Err("没有可显示的网格、粒子或精灵平面；UI Canvas 请使用 2D 预览".into());}
    if model.triangles.iter().flat_map(|t|t.points).flatten().any(|v|!v.is_finite()){return Err("3D 场景包含无效顶点".into());}
    model.triangles.sort_by_key(|triangle| model.materials[triangle.material].sorting);
    let bounds = model.triangles.iter().flat_map(|tri| tri.points).chain(particles.iter().flat_map(|particle| {
        let radius = if model.triangles.is_empty() { particle.emitter.bounds_radius() as f32 } else { 0.0 };
        [particle.origin.map(|value| value - radius), particle.origin.map(|value| value + radius)]
    }));
    let (center, scale) = world_bounds(bounds);
    for point in model.triangles.iter_mut().flat_map(|tri| &mut tri.points) {
        for axis in 0..3 { point[axis] = (point[axis] - center[axis]) / scale; }
    }
    Ok(prepare_gpu_model(model, particles, center, scale))
}
type SortingKey = (i64, i64);
type GpuModel = (Arc<crate::render_system::scene::Scene>, Arc<[usize]>, Vec<SortingKey>);

struct PreparedModel {
    model: Model,
    gpu: GpuModel,
    particles: Vec<SceneParticle>,
    center: [f32; 3],
    scale: f32,
}

fn prepare_gpu_model(model: Model, particles: Vec<SceneParticle>, center: [f32; 3], scale: f32) -> PreparedModel {
    let gpu = gpu_model(&model);
    PreparedModel { model, gpu, particles, center, scale }
}

fn sorted_scene(mut draws: Vec<(SortingKey, crate::render_system::scene::Draw)>)
    -> (Arc<crate::render_system::scene::Scene>, Arc<[usize]>) {
    draws.sort_by_key(|(sorting, _)| *sorting);
    let depth_groups = (0..draws.len()).filter(|&index| index == 0 || draws[index].0 != draws[index - 1].0)
        .collect::<Vec<_>>();
    let draws = draws.into_iter().map(|(_, draw)| draw).collect::<Vec<_>>();
    (Arc::new(crate::render_system::scene::Scene { draws: draws.into(), revision: 1 }), depth_groups.into())
}

fn gpu_model(model: &Model) -> GpuModel {
    use crate::render_system::scene as rs;
    static PROGRAM: std::sync::OnceLock<Arc<rs::Program>> = std::sync::OnceLock::new();
    let program = PROGRAM.get_or_init(|| Arc::new(rs::Program {
        revision: 1, pipeline: rs::UnityPipeline::BuiltIn,
        vertex: Arc::from(r#"#version 330 core
layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_uv;
layout(location=3) in vec3 a_barycentric;
uniform mat4 unity_MatrixVP;
out vec2 uv;
noperspective out vec3 barycentric;
void main() {
    gl_Position = unity_MatrixVP * vec4(a_position, 1.0);
    uv = a_uv;
    barycentric = a_barycentric;
}"#),
        fragment: Arc::from(r#"#version 330 core
in vec2 uv;
noperspective in vec3 barycentric;
uniform sampler2D _MainTex;
uniform float preview_mode;
uniform vec4 tint, pf_main, pf_tint, pf_ambient, pf_overlay, pf_adjust;
out vec4 color;
void main() {
    vec4 pixel = texture(_MainTex, uv);
    vec4 c = pixel * tint;
    if (pf_adjust.w >= 0.0) {
        c = pixel * pf_main;
        float gray = dot(c.rgb, vec3(0.2125,0.7154,0.0721));
        vec3 rgb = (0.5 + (mix(vec3(gray), c.rgb * pf_adjust.x, pf_adjust.y)-0.5)*pf_adjust.z)
            * pf_tint.rgb * clamp(pf_ambient.a,0.0,1.0) * pf_ambient.rgb;
        rgb = dot(rgb,vec3(0.22,0.707,0.071)) >= 0.5
            ? 1.0-2.0*(1.0-rgb)*(1.0-pf_overlay.rgb) : 2.0*rgb*pf_overlay.rgb;
        c = vec4(clamp(rgb,0.0,1.0), clamp(c.a*pf_tint.a*pf_adjust.w,0.0,1.0));
    }
    vec3 distance_px = barycentric / max(fwidth(barycentric), vec3(0.00001));
    float edge = 1.0 - smoothstep(0.25, 1.05, min(min(distance_px.x,distance_px.y),distance_px.z));
    if (preview_mode < 1.5 && c.a <= 0.0) discard;
    color = preview_mode < 1.5 ? vec4(c.rgb*c.a,c.a) : vec4(0.0);
    if (preview_mode > 0.5) {
        if (preview_mode > 1.5 && edge <= 0.0) discard;
        color = vec4(vec3(0.118,0.667,1.0)*edge,edge) + color*(1.0-edge);
    }
}"#),
    }));
    let fallback = Material::default();
    let mut ordered: Vec<_> = model.triangles.iter().collect();
    ordered.sort_by_key(|tri| (model.materials.get(tri.material).map_or((0, 0), |m| m.sorting), tri.material));
    let mut depth_groups = Vec::new();
    let mut current_group = None;
    let mut draws = Vec::new();
    let mut sorting = Vec::new();
    for chunk in ordered.chunk_by(|a, b| a.material == b.material) {
        let source = model.materials.get(chunk[0].material).unwrap_or(&fallback);
        if current_group != Some(source.sorting) {
            depth_groups.push(draws.len());
            current_group = Some(source.sorting);
        }
        let texture = source.gpu_texture.get_or_init(|| {
            let (size, rgba) = source.texture.as_ref().map_or_else(
                || ([1, 1], Arc::from([255_u8; 4])),
                |image| ([image.width(), image.height()], Arc::from(image.as_raw().as_slice())),
            );
            Arc::new(rs::Texture { size, rgba, revision: 1, repeat: [true; 2] })
        }).clone();
        let color = source.color.map(|value| value as f32 / 255.0);
        let mut uniforms = std::collections::BTreeMap::from([
            ("tint".into(), rs::Uniform::Vector(color)),
            ("pf_adjust".into(), rs::Uniform::Vector([0.0, 0.0, 0.0, -1.0])),
        ]);
        if let Some(pf) = &source.pf {
            for (name, values) in [("pf_main", pf.main), ("pf_tint", pf.tint), ("pf_ambient", pf.ambient),
                ("pf_overlay", pf.overlay), ("pf_adjust", [pf.brightness, pf.saturation, pf.contrast, pf.alpha])] {
                uniforms.insert(name.into(), rs::Uniform::Vector(values));
            }
        }
        let material = Arc::new(rs::Material {
            program: program.clone(), uniforms, textures: Default::default(),
            state: rs::RenderState {
                source: rs::BlendFactor::One, destination: rs::BlendFactor::OneMinusSourceAlpha,
                color_mask: [true; 4], depth_write: true, cull_back: false,
            },
        });
        let vertices: Vec<_> = chunk.iter().flat_map(|tri| (0..3).map(move |i| rs::Vertex {
            position: tri.points[i],
            uv: [tri.uv[i][0] * source.scale[0] + source.offset[0],
                1.0 - (tri.uv[i][1] * source.scale[1] + source.offset[1])],
            color: [1.0; 4],
            normal: std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 }),
            tangent: [0.0; 4],
        })).collect();
        let indices: Vec<_> = (0..vertices.len() as u32).collect();
        sorting.push(source.sorting);
        draws.push(rs::Draw {
            strict_shader: true, component: "model preview".into(), vertices: vertices.into(), indices: indices.into(),
            object_to_world: std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 })),
            material, main_texture: texture,
        });
    }
    (Arc::new(rs::Scene { draws: draws.into(), revision: 1 }), depth_groups.into(), sorting)
}

#[derive(Clone,Copy,PartialEq,Eq,Default)]
enum RenderMode {#[default] Solid,Overlay,Wire}
impl RenderMode {
    fn label(self)->&'static str {match self {Self::Solid=>"实体",Self::Overlay=>"实体+线框",Self::Wire=>"线框"}}
    fn key(self)->&'static str {match self {Self::Solid=>"solid",Self::Overlay=>"overlay",Self::Wire=>"wire"}}
}
fn material(path:&Path,catalog:&HashMap<String,PathBuf>)->Result<Material,String> {
    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>1024*1024{return Err("材质超过 1 MiB".into());}
    let text=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
    let blocks:Vec<_>=["_BaseMap","_MainTex","_BaseColorMap"].into_iter().filter_map(|key|text.split_once(&format!("- {key}:" )).map(|(_,s)|s.split("\n    - ").next().unwrap_or(s))).collect();
    let block=blocks.iter().copied().find(|s|s.lines().any(|l|l.trim().strip_prefix("m_Texture:").and_then(|v|crate::static_preview::reference(v.trim(),"guid")).is_some())).or_else(||blocks.first().copied());
    let mut m=Material{color:[255;4],..Default::default()};
    // These verified shader families share this fragment formula. Do not infer
    // arbitrary custom shader semantics merely from a _MainColor property.
    let shader=crate::static_preview::field(&text,"m_Shader").and_then(|v|crate::static_preview::reference(v,"guid")).and_then(|g|catalog.get(&g.to_ascii_lowercase())).and_then(|p|std::fs::metadata(p).ok().filter(|m|m.len()<=1024*1024).and_then(|_|std::fs::read_to_string(p).ok()));
    if shader.as_deref().is_some_and(|s|s.contains("Shader \"PFCore/PF3D2_Default\"") || s.contains("Shader \"PFCore/PF3D2_Transparent\"")) {
        let color=|key:&str,default:[f32;4]|text.lines().find_map(|l|l.trim().strip_prefix(&format!("- {key}:"))).and_then(|v|crate::inline_vector(v.trim(),&["r","g","b","a"])).map(|v|std::array::from_fn(|i|v[i] as f32)).unwrap_or(default);
        let scalar=|key:&str|text.lines().find_map(|l|l.trim().strip_prefix(&format!("- {key}:"))).and_then(|v|v.trim().parse::<f32>().ok()).filter(|v|v.is_finite()).unwrap_or(1.0);
        m.pf=Some(PfColor{main:color("_MainColor",[1.0;4]),tint:color("_Color",[1.0;4]),ambient:color("_AmbientColor",[1.0;4]),overlay:color("_OverlayColor",[0.5,0.5,0.5,0.0]),alpha:scalar("_AlphaScale"),brightness:scalar("_Brightness"),saturation:scalar("_Saturation"),contrast:scalar("_Contrast")});
    }
    for key in ["_BaseColor","_Color"] {if let Some(value)=text.lines().find_map(|l|l.trim().strip_prefix(&format!("- {key}:"))){if let Some(c)=crate::inline_vector(value.trim(),&["r","g","b","a"]){m.color=std::array::from_fn(|i|(c[i].clamp(0.0,1.0)*255.0).round() as u8);}break;}}
    if let Some(block)=block {
        let resolve=|key:&str|block.lines().find_map(|l|l.trim().strip_prefix(&format!("{key}:"))).map(str::trim);
        if let Some(g)=resolve("m_Texture").and_then(|v|crate::static_preview::reference(v,"guid")) {
            let p=catalog.get(&g.to_ascii_lowercase()).ok_or_else(||format!("贴图 GUID 未找到：{g}"))?;
            let mut r=image::ImageReader::open(p).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
            let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(64*1024*1024);r.limits(limits);
            m.texture=Some(Arc::new(r.decode().map_err(|e|e.to_string())?.to_rgba8()));
        }
        for(key,target) in [("m_Scale",&mut m.scale),("m_Offset",&mut m.offset)] {if let Some(v)=resolve(key).and_then(|v|crate::inline_vector(v,&["x","y"])){*target=[v[0] as f32,v[1] as f32];}}
    }Ok(m)
}
fn fbx_axes_point(p:[f32;3],axes:ufbx::CoordinateAxes)->[f64;3] {
    let component=|axis|{use ufbx::CoordinateAxis::*;match axis{PositiveX=>p[0] as f64,NegativeX=>-p[0] as f64,PositiveY=>p[1] as f64,NegativeY=>-p[1] as f64,PositiveZ=>p[2] as f64,NegativeZ=>-p[2] as f64,Unknown=>0.0}};
    [component(axes.right),component(axes.up),component(axes.front)]
}
fn unity_point(matrix:crate::static_preview::Matrix,p:ufbx::Vec3)->[f32;3] {
    let v=[-p.x,p.y,p.z,1.0];std::array::from_fn(|a|(0..4).map(|i|matrix.0[a][i]*v[i]).sum::<f64>() as f32)
}
pub(crate) fn importer_scale(meta:&str,unit_meters:f64)->Result<f64,String> {
    let value=|key:&str|meta.lines().find_map(|l|l.trim().strip_prefix(&format!("{key}:"))).map(str::trim);
    let global=value("globalScale").and_then(|v|v.parse::<f64>().ok()).unwrap_or(1.0);
    let scale=global*if value("useFileScale")==Some("0") {1.0}else{unit_meters};
    if !scale.is_finite() || scale<=0.0 {return Err("FBX 导入缩放无效".into());}Ok(scale)
}
fn load(path:&Path,paths:&[Option<PathBuf>],catalog:&HashMap<String,PathBuf>,pose:Option<(&HashMap<String,crate::static_preview::Matrix>,crate::static_preview::Matrix)>)->Result<Model,String> {
    load_evaluated(path,paths,catalog,pose,false,false,&mut Resources::default())
}
fn load_evaluated(path:&Path,paths:&[Option<PathBuf>],catalog:&HashMap<String,PathBuf>,pose:Option<(&HashMap<String,crate::static_preview::Matrix>,crate::static_preview::Matrix)>,static_skin:bool,imported_pose:bool,resources:&mut Resources)->Result<Model,String> {
    let imported_paths=if paths.is_empty(){std::fs::read_to_string(path.with_extension(format!("{}.meta",path.extension().and_then(|s|s.to_str()).unwrap_or("")))).ok().map(|s|s.lines().filter_map(|l|l.trim().strip_prefix("second:").and_then(|v|crate::static_preview::reference(v.trim(),"guid")).map(|g|catalog.get(&g.to_ascii_lowercase()).cloned())).collect::<Vec<_>>()).unwrap_or_default()}else{Vec::new()};
    let paths=if paths.is_empty(){imported_paths.as_slice()}else{paths};
    let (scene, import_scale) = resources.fbx(path, static_skin)?;
    let mut triangles=Vec::new();let mut vertex_count=0;
    for node in &scene.nodes {
        let Some(mesh)=node.mesh.as_ref() else{continue};
        vertex_count+=mesh.num_vertices;
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
                            let mut local=ufbx::transform_position(&cluster.geometry_to_bone,*p);
                            if imported_pose { local.x *= import_scale; local.y *= import_scale; local.z *= import_scale; }
                            let world=unity_point(*matrix,local);
                            for a in 0..3 {result[a]+=world[a]*weight.weight as f32;}total+=weight.weight as f32;
                        }
                    }
                }
                if total>0.0 {for v in &mut result{*v/=total;}}else{
                    let mut local=ufbx::transform_position(&node.geometry_to_node,*p);
                    if skin.is_none() || imported_pose {local.x*=import_scale;local.y*=import_scale;local.z*=import_scale;}
                    result=unity_point(renderer_matrix,local);
                }
                vertices.push(result);
            }Some(vertices)
        }else{None};
        if mesh.max_face_triangles>1_000_000 {return Err("模型面数超过限制".into());}
        let mut indices=vec![0;mesh.max_face_triangles*3];
        for (face_index,face) in mesh.faces.iter().enumerate() {
            let n=mesh.triangulate_face(&mut indices,*face) as usize;
            if triangles.len()+n>500_000 {return Err("模型三角形超过 500,000".into());}
            for t in indices[..n*3].chunks_exact(3) {
                let p=[t[0],t[1],t[2]].map(|i|{if let Some(vertices)=&posed{vertices[mesh.vertex_indices[i as usize] as usize]}else{let p=if static_skin && mesh.skinned_position.exists {let p=mesh.skinned_position[i as usize];if mesh.skinned_is_local{ufbx::transform_position(&node.geometry_to_world,p)}else{p}}else{ufbx::transform_position(&node.geometry_to_world,mesh.vertex_position[i as usize])};[p.x as f32,p.y as f32,p.z as f32]}});
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
    let materials=paths.iter().map(|p|{let result=p.as_ref().ok_or("材质引用未找到".to_owned()).and_then(|p|resources.material(p,catalog));match result {Ok(m)=>{bytes+=m.texture.as_ref().map_or(0,|t|t.as_raw().len());if bytes>128*1024*1024 {warnings.push("模型贴图总量超过 128 MiB".into());Material::default()}else{m}},Err(e)=>{warnings.push(e);Material::default()}}}).collect();
    Ok(Model{vertex_count,triangles,materials,warnings})
}
pub(crate) struct Preview {
    resources: Arc<std::sync::Mutex<Resources>>,
    particles: Vec<SceneParticle>,
    particle_clock: Option<std::time::Instant>,
    particles_paused: bool,
    particle_center: [f32; 3],
    particle_scale: f32,
    camera_direction:f32,
    mode:RenderMode,
    pending:Option<mpsc::Receiver<Result<PreparedModel,String>>>,model:Option<Model>,error:Option<String>,
    gpu:Option<GpuModel>,gpu_error:Arc<std::sync::Mutex<Option<String>>>,yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,background:String,rgb:[u8;3],
}
impl Preview {
    pub(crate) fn new_scene(ctx:&egui::Context,documents:Vec<crate::Document>,flags:HashMap<String,crate::hierarchy_controls::Flags>,catalog:Arc<HashMap<String,PathBuf>>)->Self {
        let (send,recv)=mpsc::channel();let ctx=ctx.clone();
        let resources = Arc::new(std::sync::Mutex::new(Resources::default()));
        let worker_resources = resources.clone();
        std::thread::spawn(move||{let _=send.send(scene_model(&documents,&flags,&catalog,&mut worker_resources.lock().unwrap()));ctx.request_repaint();});
        Self{resources,particles:Vec::new(),particle_clock:None,particles_paused:false,particle_center:[0.0;3],particle_scale:1.0,camera_direction:-1.0,mode:RenderMode::Solid,pending:Some(recv),model:None,error:None,gpu:None,gpu_error:Default::default(),yaw:0.3,pitch:0.15,zoom:1.0,pan:egui::Vec2::ZERO,background:"checker".into(),rgb:[0;3]}
    }
    pub(crate) fn update_scene(&mut self, ctx: &egui::Context, documents: Vec<crate::Document>,
        flags: HashMap<String,crate::hierarchy_controls::Flags>, catalog: Arc<HashMap<String,PathBuf>>) -> bool {
        if self.pending.is_some() { return false; }
        let (send, recv) = mpsc::channel();
        let ctx = ctx.clone();
        let resources = self.resources.clone();
        std::thread::spawn(move || {
            let _ = send.send(scene_model(&documents, &flags, &catalog, &mut resources.lock().unwrap()));
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        });
        self.pending = Some(recv);
        true
    }
    pub(crate) fn new(ctx:&egui::Context,path:&Path)->Self {
        let catalog=catalog_for(ctx);let paths=ctx.data_mut(|d|d.get_temp::<Vec<Option<PathBuf>>>(egui::Id::new(("model_materials",path)))).unwrap_or_default();
        let (send,recv)=mpsc::channel();let path=path.to_owned();let ctx=ctx.clone();
        std::thread::spawn(move||{let result=load(&path,&paths,&catalog,None).map(|mut m|{normalize(&mut m.triangles);prepare_gpu_model(m,Vec::new(),[0.0;3],1.0)});let _=send.send(result);ctx.request_repaint();});
        Self{resources:Default::default(),particles:Vec::new(),particle_clock:None,particles_paused:false,particle_center:[0.0;3],particle_scale:1.0,camera_direction:1.0,mode:RenderMode::Solid,pending:Some(recv),model:None,error:None,gpu:None,gpu_error:Default::default(),yaw:0.3,pitch:0.15,zoom:1.0,pan:egui::Vec2::ZERO,background:"checker".into(),rgb:[0;3]}
    }
    pub(crate) fn scene_pending(&self) -> bool { self.pending.is_some() }
    pub(crate) fn fit(&mut self){self.zoom=1.0;self.pan=egui::Vec2::ZERO;}
    pub(crate) fn reset(&mut self){self.fit();self.yaw=0.3;self.pitch=0.15;}
    pub(crate) fn show_scene(&mut self,ui:&mut egui::Ui,background:&str,rgb:[u8;3],continuous:bool,refresh_enabled:bool,fixed_view:bool)->bool {
        
        self.background=background.into();self.rgb=rgb;self.show_inner(ui,false,continuous,refresh_enabled,fixed_view)
    }
    pub(crate) fn show(&mut self,ui:&mut egui::Ui)->bool {
        self.show_inner(ui,true,false,false,false)
    }
    fn show_inner(&mut self,ui:&mut egui::Ui,controls:bool,continuous:bool,refresh_enabled:bool,fixed_view:bool)->bool {
        let mut reload=false;
        if controls {ui.horizontal_wrapped(|ui|{
            egui::ComboBox::from_id_salt("model_render_mode").width(96.0).selected_text(self.mode.label()).show_ui(ui,|ui| {
                for mode in [RenderMode::Solid,RenderMode::Overlay,RenderMode::Wire] {ui.selectable_value(&mut self.mode,mode,mode.label());}
            });
            ui.label("静态模型");if ui.button("适应窗口").clicked(){self.zoom=1.0;self.pan=egui::Vec2::ZERO;}
            if ui.button("重置视角").clicked(){self.yaw=0.3;self.pitch=0.15;self.pan=egui::Vec2::ZERO;self.zoom=1.0;}
            crate::theme::shared_preview_background(ui,&mut self.background,&mut self.rgb);
            reload=ui.button("重载").clicked();
        });}
        ui.small(if fixed_view {
            crate::localization::tr("XY 正交视图 · 滚轮缩放 · 拖动平移")
        } else if controls {"左键旋转 · 右键平移 · 滚轮围绕鼠标缩放；无光照主贴图预览，暂不还原 Shader、骨骼动画及 Prefab 骨骼姿势。FBX 多网格显示整个模型。"} else {"左键旋转 · 右键平移 · 滚轮围绕鼠标缩放；网格保留父级变换与静态骨骼姿势，精灵和Spine使用静态平面。"});
        if let Some(result) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.pending = None;
            match result {
                Ok(prepared) => {
                    let mut old: HashMap<_, _> = std::mem::take(&mut self.particles).into_iter()
                        .map(|particle| (particle.emitter.id.clone(), particle.emitter)).collect();
                    self.particles = prepared.particles;
                    for particle in &mut self.particles {
                        if let Some(mut emitter) = old.remove(&particle.emitter.id) {
                            emitter.update_transform(&particle.emitter);
                            particle.emitter = emitter;
                        }
                    }
                    self.particle_center = prepared.center;
                    self.particle_scale = prepared.scale;
                    self.gpu = Some(prepared.gpu);
                    self.model = Some(prepared.model);
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
        }
        if self.pending.is_some() && self.model.is_none(){ui.spinner();return reload;}if let Some(e)=&self.error{ui.colored_label(Color32::RED,e);return reload;}
        let Some(model)=&self.model else{return reload};
        crate::theme::preview_warning_details(ui,"model_preview_warnings",crate::localization::tr("预览问题详情"),&model.warnings);
        if !self.particles.is_empty() {
            ui.horizontal(|ui| {
                use crate::localization::tr;
                if ui.button(tr(if self.particles_paused { "播放" } else { "暂停" })).clicked() {
                    self.particles_paused = !self.particles_paused;
                    self.particle_clock = None;
                }
                if ui.button(tr("重播")).clicked() {
                    for particle in &mut self.particles { particle.emitter.restart(); }
                    self.particles_paused = false;
                    self.particle_clock = None;
                }
            });
        }
        let running = refresh_enabled && !self.particles_paused
            && self.particles.iter().any(|particle| particle.emitter.is_playing());
        if running {
            let now = std::time::Instant::now();
            let dt = self.particle_clock.replace(now).map_or(1.0 / 60.0, |last| (now - last).as_secs_f64());
            for particle in &mut self.particles { particle.emitter.advance(dt); }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            self.particle_clock = None;
        }
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.0,1.0)),egui::Sense::drag());
        let primary_drag = response.dragged_by(egui::PointerButton::Primary);
        if primary_drag && !fixed_view {
            let delta = ui.input(|input| input.pointer.delta());
            self.yaw += delta.x * 0.01;
            self.pitch = (self.pitch + delta.y * 0.01).clamp(-1.5, 1.5);
        }
        if response.dragged_by(egui::PointerButton::Secondary) || (primary_drag && fixed_view) {
            self.pan += ui.input(|input| input.pointer.delta());
        }
        if response.hovered(){let scroll=ui.input(|i|i.smooth_scroll_delta.y);if scroll!=0.0{
            let next=(self.zoom*(scroll*0.002).exp()).clamp(0.05,20.0);
            if let Some(pointer)=ui.input(|i|i.pointer.hover_pos()){self.pan=zoom_pan(self.pan,pointer-rect.center(),next/self.zoom);}
            self.zoom=next;
        }}
        ui.painter().add(crate::static_preview::background_shape(rect,&self.background,self.rgb));
        if continuous {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        let painter=ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        if let Some((scene, depth_groups, sorting)) = &self.gpu {
            let (sy, cy) = if fixed_view { (0.0, 1.0) } else { self.yaw.sin_cos() };
            let (sp, cp) = if fixed_view { (0.0, 1.0) } else { self.pitch.sin_cos() };
            let scale = rect.width().min(rect.height()) * 1.5 * self.zoom;
            let x = scale / rect.width().max(1.0);
            let y = scale / rect.height().max(1.0);
            let z = -self.camera_direction * 0.5;
            let projection = [
                [cy*x, 0.0, sy*x, 2.0*self.pan.x/rect.width().max(1.0)],
                [sy*sp*y, cp*y, -cy*sp*y, -2.0*self.pan.y/rect.height().max(1.0)],
                [-sy*cp*z, sp*z, cy*cp*z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
            let mode = match self.mode { RenderMode::Solid => 0.0, RenderMode::Overlay => 1.0, RenderMode::Wire => 2.0 };
            let (scene, depth_groups) = if self.particles.is_empty() {
                (scene.clone(), depth_groups.clone())
            } else {
                let axes = [[cy, 0.0, sy], [sy*sp, cp, -cy*sp]];
                let mut draws: Vec<_> = sorting.iter().copied().zip(scene.draws.iter().cloned()).collect();
                let mut count = 0;
                for particle in &self.particles {
                    count += particle.emitter.particle_count();
                    if count > 65_536 { break; }
                    let mut draw = particle.emitter.draw_billboard(&particle.template, axes);
                    if draw.indices.is_empty() { continue; }
                    let scale = self.particle_scale;
                    for axis in 0..3 {
                        draw.object_to_world[axis][axis] = 1.0 / scale;
                        draw.object_to_world[axis][3] = -self.particle_center[axis] / scale;
                    }
                    draws.push((particle.sorting, draw));
                }
                sorted_scene(draws)
            };
            painter.add(crate::render_system::model_callback(rect, scene, depth_groups, projection, mode, self.gpu_error.clone()));
        }
        if let Some(error) = self.gpu_error.lock().unwrap().as_ref() {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, error, egui::FontId::proportional(13.0), Color32::RED);
        }
        let stats=painter.layout_no_wrap(format!("顶点：{}\n三角面：{}",model.vertex_count,model.triangles.len()),egui::FontId::proportional(13.0),Color32::WHITE);
        let stats_rect=egui::Rect::from_min_size(rect.min+egui::vec2(8.0,8.0),stats.size()+egui::vec2(24.0,12.0));
        painter.rect_filled(stats_rect,6.0,Color32::from_black_alpha(128));
        painter.galley(stats_rect.min+egui::vec2(12.0,6.0),stats,Color32::WHITE);
        let galley=painter.layout_no_wrap(format!("{:.0}%",self.zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE);
        let badge_size=galley.size()+egui::vec2(24.0,12.0);
        let badge=egui::Rect::from_center_size(egui::pos2(rect.center().x,rect.bottom()-8.0-badge_size.y*0.5),badge_size);
        painter.rect_filled(badge,6.0,Color32::from_black_alpha(128));
        painter.galley(badge.min+egui::vec2(12.0,6.0),galley,Color32::WHITE);
        if let Some(value)=crate::theme::zoom_preset_menu(ui,badge,self.zoom,badge.right()-12.0){self.zoom=value;self.pan=egui::Vec2::ZERO;}
        reload
    }
}
fn viewport_size(viewport:egui::Vec2)->[usize;2] {
    let factor=(1024.0/viewport.x.max(viewport.y).max(1.0)).min(1.0);
    [(viewport.x*factor).round().max(1.0) as usize,(viewport.y*factor).round().max(1.0) as usize]
}
fn zoom_pan(pan:egui::Vec2,anchor:egui::Vec2,ratio:f32)->egui::Vec2 {
    anchor-(anchor-pan)*ratio
}
fn normalize(triangles:&mut [Triangle]) {
    for p in triangles.iter_mut().flat_map(|t|&mut t.points){p[0]=-p[0];}
    normalize_world(triangles);
}
fn normalize_world(triangles: &mut [Triangle]) {
    let (center, scale) = world_bounds(triangles.iter().flat_map(|tri| tri.points));
    for point in triangles.iter_mut().flat_map(|tri| &mut tri.points) {
        for axis in 0..3 { point[axis] = (point[axis] - center[axis]) / scale; }
    }
}

fn world_bounds(points: impl Iterator<Item = [f32; 3]>) -> ([f32; 3], f32) {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for point in points {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(point[axis]);
            hi[axis] = hi[axis].max(point[axis]);
        }
    }
    let scale = (0..3).map(|axis| hi[axis] - lo[axis]).fold(0.0, f32::max).max(1e-6);
    (std::array::from_fn(|axis| (lo[axis] + hi[axis]) * 0.5), scale)
}
fn sample(material:Option<&Material>,uv:[f32;2])->Color32 {
    let Some(m)=material else{return Color32::from_gray(200)};
    let mut pixel=[255;4];
    if let Some(t)=&m.texture {let u=(uv[0]*m.scale[0]+m.offset[0]).rem_euclid(1.0);let v=(uv[1]*m.scale[1]+m.offset[1]).rem_euclid(1.0);let x=(u*t.width() as f32).floor() as u32;let y=((1.0-v)*t.height() as f32).floor() as u32;pixel=t.get_pixel(x.min(t.width()-1),y.min(t.height()-1)).0;}
    if let Some(p)=&m.pf {
        let c:[f32;4]=std::array::from_fn(|i|pixel[i] as f32/255.0*p.main[i]);
        let gray=0.2125*c[0]+0.7154*c[1]+0.0721*c[2];
        let rgb:[f32;3]=std::array::from_fn(|i|(0.5+((gray+(c[i]*p.brightness-gray)*p.saturation)-0.5)*p.contrast)*p.tint[i]*p.ambient[3].clamp(0.0,1.0)*p.ambient[i]);
        let high=0.22*rgb[0]+0.707*rgb[1]+0.071*rgb[2]>=0.5;
        let mut result=std::array::from_fn(|i|if i==3 {0} else {let v=if high {1.0-2.0*(1.0-rgb[i])*(1.0-p.overlay[i])}else{2.0*rgb[i]*p.overlay[i]};(v.clamp(0.0,1.0)*255.0).round() as u8});
        result[3]=(c[3]*p.tint[3]*p.alpha).clamp(0.0,1.0).mul_add(255.0,0.0).round() as u8;
        return crate::asset_viewer::preview_color(result);
    }
    crate::asset_viewer::preview_color(std::array::from_fn(|i|((pixel[i] as u16*m.color[i] as u16+127)/255) as u8))
}
fn component_model(mesh:&str,renderer:&str,catalog:&HashMap<String,PathBuf>,matrix:crate::static_preview::Matrix,bones:&HashMap<String,crate::static_preview::Matrix>,local_mesh:Option<&str>)->Result<Model,String> {
    component_model_cached(mesh, renderer, catalog, matrix, bones, local_mesh, &mut Resources::default())
}
fn component_model_cached(mesh:&str,renderer:&str,catalog:&HashMap<String,PathBuf>,matrix:crate::static_preview::Matrix,bones:&HashMap<String,crate::static_preview::Matrix>,local_mesh:Option<&str>, resources: &mut Resources)->Result<Model,String> {
    let builtin_quad=crate::static_preview::reference(mesh,"fileID").as_deref()==Some("10210") && crate::static_preview::reference(mesh,"guid").as_deref()==Some("0000000000000000e000000000000000");
    let model=if builtin_quad {
        let points=[[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]].map(|p|std::array::from_fn(|a|(matrix.0[a][0]*p[0]+matrix.0[a][1]*p[1]+matrix.0[a][3]) as f32));
        let uv=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
        let triangles=[[0,1,2],[0,2,3]].map(|t|Triangle{points:t.map(|i|points[i]),uv:t.map(|i|uv[i]),material:0}).to_vec();
        let mut warnings=Vec::new();let materials=material_paths(renderer,catalog).iter().map(|p|match p.as_ref().ok_or("Quad 材质引用未找到".into()).and_then(|p|resources.material(p,catalog)) {Ok(m)=>m,Err(e)=>{warnings.push(e);Material::default()}}).collect();
        Model{vertex_count:4,triangles,materials,warnings}
    }else{
        let path=crate::static_preview::reference(mesh,"guid").and_then(|g|catalog.get(&g.to_ascii_lowercase()));
        let local;
        let text=if let Some(text)=local_mesh {Some(text)}else if let Some(path)=path.filter(|path|path.extension().is_some_and(|e|e.eq_ignore_ascii_case("asset"))) {
            if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>64*1024*1024 {return Err("Unity Mesh asset exceeds 64 MiB".into());}
            local=std::fs::read_to_string(path).map_err(|e|e.to_string())?;Some(local.as_str())
        }else{None};
        if let Some(text)=text {
            let mesh=if local_mesh.is_some() {crate::render_system::mesh::decode(text).map_err(|e|format!("Scene-local Mesh: {e}"))?}else{crate::render_system::mesh::decode_reference(text,&crate::static_preview::reference(mesh,"fileID").ok_or("Missing Mesh fileID")?)?};
            let points:Vec<[f32;3]>=mesh.vertices.iter().map(|v|std::array::from_fn(|a|(matrix.0[a][0]*v.position[0] as f64+matrix.0[a][1]*v.position[1] as f64+matrix.0[a][2]*v.position[2] as f64+matrix.0[a][3]) as f32)).collect();
            let triangles=mesh.submeshes.iter().enumerate().flat_map(|(slot,indices)|indices.chunks_exact(3).map(move |t|(slot,[t[0] as usize,t[1] as usize,t[2] as usize]))).map(|(slot,t)|Triangle{points:t.map(|i|points[i]),uv:t.map(|i|mesh.vertices[i].uv),material:slot}).collect();
            let mut warnings=Vec::new();let materials=material_paths(renderer,catalog).iter().map(|p|match p.as_ref().ok_or("Mesh 材质引用未找到".into()).and_then(|p|resources.material(p,catalog)){Ok(m)=>m,Err(e)=>{warnings.push(e);Material::default()}}).collect();
            return Ok(Model{vertex_count:mesh.vertices.len(),triangles,materials,warnings});
        }
        let path=path.ok_or("Mesh reference not resolved (external GUID or scene-local fileID)")?;
        if !is_model(path){return Err("暂不支持此 Mesh 资源格式".into());}
        // Prefab supplies node transforms: never apply the FBX hierarchy a second time.
        if crate::static_preview::number(renderer,"preview_FbxAnimatedPose",0.0)==1.0 {
            load_evaluated(path,&material_paths(renderer,catalog),catalog,Some((bones,matrix)),false,true,resources)?
        } else if crate::static_preview::number(renderer,"preview_FbxStaticPose",0.0)==1.0 {
            let mut model=load_evaluated(path,&material_paths(renderer,catalog),catalog,None,true,false,resources)?;
            let (scene, scale) = resources.fbx(path, true)?;
            let bone_ratio=crate::static_preview::number(renderer,"preview_FbxBoneScaleRatio",1.0);
            let pivot=scene.nodes.iter().find(|n|n.bone.is_some() && (n.local_transform.scale.x-1.0).abs()>0.001).map(|n|ufbx::transform_position(&n.node_to_world,ufbx::Vec3::default())).unwrap_or_default();
            for triangle in &mut model.triangles {for point in &mut triangle.points {
                for (a,p) in [pivot.x,pivot.y,pivot.z].iter().enumerate(){point[a]=(*p+(point[a] as f64-*p)*bone_ratio) as f32;}
                let p=fbx_axes_point(*point,scene.settings.axes);
                *point=unity_point(matrix,ufbx::Vec3{x:p[0]*scale,y:p[1]*scale,z:p[2]*scale});
            }}
            model.warnings.push("FBX static imported pose; animation and bone overrides are not reproduced".into());
            model
        }else{load_evaluated(path,&material_paths(renderer,catalog),catalog,Some((bones,matrix)),false,false,resources)?}
    };
    Ok(model)
}
pub(crate) fn bake(mesh:&str,renderer:&str,catalog:&HashMap<String,PathBuf>,matrix:crate::static_preview::Matrix,bones:&HashMap<String,crate::static_preview::Matrix>,local_mesh:Option<&str>)->Result<(crate::spine_preview::Baked,Vec<String>),String> {
    let mut model=component_model(mesh,renderer,catalog,matrix,bones,local_mesh)?;
    if model.triangles.is_empty() {return Ok((crate::spine_preview::Baked{image:egui::ColorImage::new([1,1],Color32::TRANSPARENT),region:crate::static_preview::Region{rect:[0.0,0.0,1.0,1.0],pivot:[0.5;2],ppu:1.0,border:[0.0;4]},approximate:false,gpu:None},model.warnings));}
    if model.triangles.iter().flat_map(|t|&t.points).flatten().any(|v|!v.is_finite()){return Err("模型变换包含无效坐标".into());}
    let mut lo=[f32::INFINITY;2];let mut hi=[f32::NEG_INFINITY;2];for p in model.triangles.iter().flat_map(|t|&t.points){for a in 0..2{lo[a]=lo[a].min(p[a]);hi[a]=hi[a].max(p[a]);}}
    let extent=(hi[0]-lo[0]).max(hi[1]-lo[1]).max(0.001);let ppu=1024.0/extent;
    let size=[((hi[0]-lo[0])*ppu).ceil().max(1.0) as usize,((hi[1]-lo[1])*ppu).ceil().max(1.0) as usize];
    for p in model.triangles.iter_mut().flat_map(|t|&mut t.points){p[0]=(p[0]-(lo[0]+hi[0])*0.5)/extent;p[1]=(p[1]-(lo[1]+hi[1])*0.5)/extent;}
    let zoom=1024.0/(size[0].min(size[1]) as f32*0.75);
    let image=render_direction(&model,size,0.0,0.0,zoom,egui::Vec2::ZERO,-1.0);
    let region=crate::static_preview::Region{rect:[0.0,0.0,size[0] as f64,size[1] as f64],pivot:[-(lo[0] as f64)*ppu as f64/size[0] as f64,-(lo[1] as f64)*ppu as f64/size[1] as f64],ppu:ppu as f64,border:[0.0;4]};
    Ok((crate::spine_preview::Baked{image,region,approximate:true,gpu:None},model.warnings))
}
#[cfg(test)]
fn render(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2)->egui::ColorImage {
    render_direction(model,size,yaw,pitch,zoom,pan,1.0)
}
fn render_direction(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,depth_direction:f32)->egui::ColorImage {
    render_mode(model,size,yaw,pitch,zoom,pan,depth_direction,"solid")
}
#[allow(clippy::too_many_arguments)]
fn render_mode(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,depth_direction:f32,mode:&str)->egui::ColorImage {
    render_raw(model,size,yaw,pitch,zoom,pan,depth_direction,mode,1.0)
}
#[allow(clippy::too_many_arguments)]
fn render_raw(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,depth_direction:f32,mode:&str,line_scale:f32)->egui::ColorImage {
    let mut image=egui::ColorImage{size,pixels:vec![Color32::TRANSPARENT;size[0]*size[1]]};let mut depth=vec![f32::NEG_INFINITY;image.pixels.len()];
    let scale=size[0].min(size[1]) as f32*0.75*zoom;
    let sorted_layers = model.materials.iter().any(|material| material.sorting != (0, 0));
    let mut layer_pixels = sorted_layers.then(|| vec![(usize::MAX, Color32::TRANSPARENT); image.pixels.len()]);
    let mut group = 0;
    let mut sorting = None;
    for t in &model.triangles {
        let material = model.materials.get(t.material);
        let next_sorting = material.map_or((0, 0), |material| material.sorting);
        if sorted_layers && sorting != Some(next_sorting) {
            sorting = Some(next_sorting);
            group += 1;
        }
        let p=t.points.map(|p|{let x=p[0]*yaw.cos()+p[2]*yaw.sin();let z=-p[0]*yaw.sin()+p[2]*yaw.cos();let y=p[1]*pitch.cos()-z*pitch.sin();let z=p[1]*pitch.sin()+z*pitch.cos();[size[0] as f32*0.5+x*scale+pan.x,size[1] as f32*0.5-y*scale+pan.y,z*depth_direction]});
        let edge=|a:[f32;3],b:[f32;3],x:f32,y:f32|(b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]);let area=edge(p[0],p[1],p[2][0],p[2][1]);if area.abs()<1e-6{continue;}
        let heights=[(1,2),(2,0),(0,1)].map(|(a,b)|area.abs()/((p[a][0]-p[b][0]).hypot(p[a][1]-p[b][1])).max(1e-6));
        let bounds=|axis:usize,max:usize|{let lo=p.iter().map(|p|p[axis]).fold(f32::INFINITY,f32::min).floor().clamp(0.0,max as f32) as usize;let hi=p.iter().map(|p|p[axis]).fold(f32::NEG_INFINITY,f32::max).ceil().clamp(0.0,max as f32) as usize;(lo,hi)};let (x0,x1)=bounds(0,size[0]);let(y0,y1)=bounds(1,size[1]);
        for y in y0..y1 {for x in x0..x1 {let w=[edge(p[1],p[2],x as f32+0.5,y as f32+0.5)/area,edge(p[2],p[0],x as f32+0.5,y as f32+0.5)/area,edge(p[0],p[1],x as f32+0.5,y as f32+0.5)/area];if w.iter().any(|v|*v<0.0){continue;}let z=(0..3).map(|i|w[i]*p[i][2]).sum::<f32>();let i=y*size[0]+x;
            if let Some(pixels) = &mut layer_pixels && pixels[i].0 != group {
                pixels[i] = (group, image.pixels[i]);
                depth[i] = f32::NEG_INFINITY;
            }
            if z>depth[i]{let uv=std::array::from_fn(|a|(0..3).map(|j|w[j]*t.uv[j][a]).sum());let c=sample(material,uv);if mode!="wire" && c.a()==0{continue;}let distance=(0..3).map(|j|w[j]*heights[j]).fold(f32::INFINITY,f32::min);depth[i]=z;let color = wire_pixel(c,distance/line_scale,zoom,mode);
                image.pixels[i] = layer_pixels.as_ref().map_or(color, |pixels| {
                    crate::render_system::source_over(color, Color32::WHITE, pixels[i].1)
                });}}}
    }image
}
fn wire_pixel(base:Color32,distance:f32,zoom:f32,mode:&str)->Color32 {
    if mode=="solid" {return base;}
    let half_width=(0.35*zoom.min(1.0).max(0.0).sqrt()).max(0.12);
    let coverage=(half_width+0.5-distance).clamp(0.0,1.0);
    let line=crate::asset_viewer::preview_color([30,170,255,(coverage*255.0).round() as u8]);
    if mode=="wire" {return line;}
    crate::render_system::source_over(line,Color32::WHITE,base)
}
#[allow(clippy::too_many_arguments)]
fn render_antialiased(model:&Model,size:[usize;2],yaw:f32,pitch:f32,zoom:f32,pan:egui::Vec2,mode:&str,direction:f32)->egui::ColorImage {
    let high=render_raw(model,[size[0]*2,size[1]*2],yaw,pitch,zoom,pan*2.0,direction,mode,2.0);
    downsample_twice(&high)
}
fn downsample_twice(high:&egui::ColorImage)->egui::ColorImage {
    let size=[high.size[0]/2,high.size[1]/2];let mut pixels=Vec::with_capacity(size[0]*size[1]);
    for y in 0..size[1] {for x in 0..size[0] {
        let samples=[high.pixels[y*2*high.size[0]+x*2].to_array(),high.pixels[y*2*high.size[0]+x*2+1].to_array(),high.pixels[(y*2+1)*high.size[0]+x*2].to_array(),high.pixels[(y*2+1)*high.size[0]+x*2+1].to_array()];
        let p:[u8;4]=std::array::from_fn(|a|((samples.iter().map(|p|p[a] as u16).sum::<u16>()+2)/4) as u8);
        pixels.push(Color32::from_rgba_premultiplied(p[0],p[1],p[2],p[3]));
    }}egui::ColorImage{size,pixels}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pf_fragment_alpha_preserves_texture_main_tint_and_script_scale() {
        let pf=PfColor{main:[1.0,0.5,0.25,0.4],tint:[1.0,1.0,1.0,0.5],ambient:[1.0;4],overlay:[0.5,0.5,0.5,0.0],alpha:0.5,brightness:1.0,saturation:1.0,contrast:1.0};
        let material=Material{pf:Some(pf),texture:Some(Arc::new(image::RgbaImage::from_pixel(1,1,image::Rgba([255,255,255,128])))),..Default::default()};
        let color=sample(Some(&material),[0.5,0.5]);
        assert_eq!(color.a(),13);
        assert!(color.r()>color.g() && color.g()>color.b());
        let background=Color32::BLUE;
        let composed=crate::render_system::source_over(color,Color32::WHITE,background);
        assert_eq!(composed.a(),255);
        assert!(composed.b()>200 && composed.r()<30);
    }
    #[test]
    fn scene_local_triangle_mesh_needs_no_guid() {
        let source=include_str!("../tests/fixtures/l9584_river1.asset");
        let model=component_model("{fileID: 1}","",&HashMap::new(),crate::static_preview::Matrix::identity(),&HashMap::new(),Some(source)).unwrap();
        assert!(model.vertex_count>0 && !model.triangles.is_empty());
        let line_mesh=source.replace("topology: 0","topology: 3");
        let error=component_model("{fileID: 1}","",&HashMap::new(),crate::static_preview::Matrix::identity(),&HashMap::new(),Some(&line_mesh)).err().unwrap();
        assert!(error.contains("Scene-local Mesh") && error.contains("triangle"));
    }
    #[test]
    fn zoom_preserves_point_under_mouse_after_pan() {
        let pan=egui::vec2(20.0,-30.0);let anchor=egui::vec2(-60.0,-120.0);
        let next=zoom_pan(pan,anchor,2.0);
        assert_eq!((anchor-pan)*2.0+next,anchor);
        assert_eq!(zoom_pan(pan,anchor,1.0),pan);
    }
    #[test]
    fn supersample_resolve_preserves_transparent_edge_color() {
        let high=egui::ColorImage{size:[2,2],pixels:vec![Color32::RED,Color32::TRANSPARENT,Color32::TRANSPARENT,Color32::TRANSPARENT]};
        let result=downsample_twice(&high);
        assert_eq!(result.size,[1,1]);
        assert_eq!(result.pixels[0].to_array(),[64,0,0,64]);
    }
    #[test]
    fn wire_edges_are_antialiased_and_thinner_when_zoomed_out() {
        let near=wire_pixel(Color32::WHITE,0.3,1.0,"wire");
        let small=wire_pixel(Color32::WHITE,0.3,0.25,"wire");
        assert!(near.a()>0 && near.a()<255);
        assert!(small.a()<near.a());
        assert_eq!(wire_pixel(Color32::WHITE,2.0,1.0,"wire"),Color32::TRANSPARENT);
        assert_eq!(wire_pixel(Color32::WHITE,2.0,1.0,"overlay"),Color32::WHITE);
    }
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
    fn unity_import_scale_converts_inches_and_respects_override() {
        assert_eq!(importer_scale("    globalScale: 1\n    useFileScale: 1",0.0254).unwrap(),0.0254);
        assert_eq!(importer_scale("    globalScale: 2\n    useFileScale: 0",0.0254).unwrap(),2.0);
        assert!(importer_scale("    globalScale: -1",0.0254).is_err());
    }
    #[test]
    fn lower_sorting_order_shadow_cannot_occlude_model() {
        let triangle = |z, material| Triangle {
            points: [[-0.5, -0.5, z], [0.5, -0.5, z], [0.0, 0.5, z]],
            uv: [[0.0; 2]; 3],
            material,
        };
        let model = Model {
            vertex_count: 6,
            triangles: vec![triangle(0.4, 0), triangle(0.0, 1)],
            materials: vec![
                Material { color: [0, 0, 0, 128], sorting: (0, -1), ..Default::default() },
                Material { color: [255, 0, 0, 255], ..Default::default() },
            ],
            warnings: Vec::new(),
        };
        let frame = render_direction(&model, [32, 32], 0.0, 0.0, 1.0, egui::Vec2::ZERO, 1.0);
        assert_eq!(frame.pixels[16 * 32 + 16], Color32::RED);
    }

    #[test]
    fn depth_buffer_is_independent_of_triangle_order() {
        let back=[[-0.5,-0.5,-0.2],[0.5,-0.5,-0.2],[0.0,0.5,-0.2]];
        let front=[[-0.5,-0.5,0.0],[0.5,-0.5,0.4],[0.0,0.5,0.2]];
        let triangle=|points|Triangle{points,uv:[[0.0;2];3],material:0};
        let a=render(&Model{vertex_count:0,triangles:vec![triangle(back),triangle(front)],materials:vec![],warnings:vec![]},[32,32],0.0,0.0,1.0,egui::Vec2::ZERO);
        let b=render(&Model{vertex_count:0,triangles:vec![triangle(front),triangle(back)],materials:vec![],warnings:vec![]},[32,32],0.0,0.0,1.0,egui::Vec2::ZERO);
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
        let m=Material{texture:Some(Arc::new(texture)),color:[128,255,255,255],..Default::default()};
        assert_eq!(sample(Some(&m),[0.5,0.75]),Color32::from_rgb(128,0,0));
        assert_eq!(sample(Some(&m),[0.5,0.25]),Color32::from_rgb(0,255,0));
        let triangle=|z,material|Triangle{points:[[-0.5,-0.5,z],[0.5,-0.5,z],[0.0,0.5,z]],uv:[[0.5;2];3],material};
        let model=Model{vertex_count:0,triangles:vec![triangle(0.4,0),triangle(0.0,1)],materials:vec![Material{color:[0;4],..Default::default()},Material{color:[255,0,0,255],..Default::default()}],warnings:vec![]};
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
        let model=Model{vertex_count:0,triangles:vec![triangle(-0.2,0),triangle(0.2,1)],materials:vec![Material{color:[255,0,0,255],..Default::default()},Material{color:[0,255,0,255],..Default::default()}],warnings:vec![]};
        assert_eq!(render_direction(&model,[32,32],0.0,0.0,1.0,egui::Vec2::ZERO,-1.0).pixels[16*32+16],Color32::RED);
    }
    #[test]
    fn wire_mode_keeps_edges_and_clears_face_interior() {
        let model=Model{vertex_count:3,triangles:vec![Triangle{points:[[-0.5,-0.5,0.0],[0.5,-0.5,0.0],[0.0,0.5,0.0]],uv:[[0.0;2];3],material:0}],materials:vec![],warnings:vec![]};
        let wire=render_mode(&model,[64,64],0.0,0.0,1.0,egui::Vec2::ZERO,1.0,"wire");
        let overlay=render_mode(&model,[64,64],0.0,0.0,1.0,egui::Vec2::ZERO,1.0,"overlay");
        assert_eq!(wire.pixels[32*64+32],Color32::TRANSPARENT);
        assert_ne!(overlay.pixels[32*64+32],Color32::TRANSPARENT);
        assert!(wire.pixels.iter().any(|p|p.a()>0 && p.a()<255));
    }
}
