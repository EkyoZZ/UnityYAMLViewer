use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::{Arc, mpsc}, time::SystemTime};
use eframe::egui::{self, Color32, Pos2, Vec2};
use crate::{Document, localization::tr};
mod world;
mod sdf;
pub(crate) mod prefab;
const AUTO_PREFAB_LIMIT:usize=20;
const PREVIEW_TEXTURE_BUDGET:usize=2 * 1024 * 1024 * 1024;

// A component selection remains precise; object/transform selections cover all
// renderable components on that object, without selecting unrelated descendants.
fn selection_components(documents:&[Document],selected:Option<&str>,expanded:Option<&prefab::Expanded>) -> HashSet<String> {
    let Some(selected)=selected else {return HashSet::new()};
    let selected_doc=documents.iter().find(|doc|doc.file_id==selected);
    let object=selected_doc.and_then(|doc|match doc.type_name.as_str() {
        "GameObject"=>Some(doc.file_id.as_str()),
        "Transform"|"RectTransform"=>doc.game_object.as_deref(),
        _=>None,
    });
    documents.iter().filter(|doc| {
        doc.file_id==selected || object.is_some_and(|object|doc.game_object.as_deref()==Some(object))
            || (selected_doc.is_none_or(|doc|doc.type_name=="PrefabInstance") && expanded.is_some_and(|data|data.picks.get(&doc.file_id).is_some_and(|proxy|proxy==selected)))
    }).map(|doc|doc.file_id.clone()).collect()
}

#[derive(Default)]
pub struct Preview {
    pub open: bool,
    three_dimensional: bool,
    scene3d: Option<(u64, crate::model_preview::Preview)>,
    textures: HashMap<PathBuf, Texture>,
    identity: String,
    zoom: f32,
    pan: Vec2,
    next_asset_check: Option<std::time::Instant>,
    ui_viewport: Option<[f64;2]>,
    focus_request: Option<(HashSet<String>, std::time::Instant)>,
    fonts: crate::preview_fonts::Fonts,
    background: String,
    background_rgb: Option<[u8;3]>,
    show_guides: bool,
    sdf: sdf::Atlas,
    prefabs: prefab::Cache,
    expand_prefabs: bool,
    prefab_count: Option<(usize,usize,usize,usize)>,
    preparation: Option<Preparation>,
    prepared: Arc<Prepared>,
    sort_axis: Option<usize>,
    preparation_pending: bool,
    paused: Arc<std::sync::atomic::AtomicBool>,
    world: world::World,
    ui_presence: Option<(usize,usize,usize,bool)>,
}

#[derive(Default)]
struct Prepared { sprites: Vec<Sprite>, missing: usize, issues: Vec<String> }
struct Preparation {
    key: u64, receiver: Option<mpsc::Receiver<Prepared>>, cancelled: Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for Preparation {fn drop(&mut self) {
    self.cancelled.store(true,std::sync::atomic::Ordering::Relaxed);
    if let Some(receiver)=self.receiver.take() {std::thread::spawn(move || drop(receiver));}
}}
fn coordinate_key(value:f64) -> i64 {
    let value=if value.is_finite() {-value} else {0.0};
    let bits=value.to_bits() as i64;
    if bits<0 {bits ^ i64::MAX} else {bits}
}
fn renderer_order(doc:&Document, matrix:Matrix, axis:usize) -> (i64,i64,i64) {
    (number(&doc.text,"m_SortingLayer",0.0) as i64 as i16 as i64,
     number(&doc.text,"m_SortingOrder",0.0) as i64 as i16 as i64,
     coordinate_key(matrix.0[axis.min(2)][3]))
}
fn prepare(documents: &[Document], flags: &HashMap<String,crate::hierarchy_controls::Flags>, catalog: Option<&Arc<HashMap<String,PathBuf>>>, cancelled: &std::sync::atomic::AtomicBool, axis:usize, paused:Option<&std::sync::atomic::AtomicBool>) -> Prepared {
    let objects:HashMap<_,_>=documents.iter().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
    let transforms:HashMap<_,_>=documents.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
    let by_object:HashMap<_,_>=transforms.values().filter_map(|d|Some((d.game_object.as_deref()?,d.file_id.as_str()))).collect();
    let mut sprites=Vec::new();let mut missing=0;let mut issues=Vec::new();
    let mut tile_assets:HashMap<(PathBuf,String),Option<String>>=HashMap::new();
        for doc in documents.iter().filter(|d|d.type_name=="SpriteRenderer") {
            if !checkpoint(cancelled,paused) {return Prepared::default();}
            if doc.game_object.as_deref().and_then(|id| flags.get(id)).is_some_and(|flags| !flags.visible) { continue; }
            if number(&doc.text,"m_Enabled",1.0)==0.0 {continue}
            let Some(reference_text)=field(&doc.text,"m_Sprite") else {continue};
            let Some(id)=reference(reference_text,"fileID").filter(|id|id!="0") else {continue};
            let matrix=doc.game_object.as_deref().and_then(|go|by_object.get(go)).and_then(|id|world(id,&transforms,&objects,&mut HashSet::new()));
            let Some(matrix)=matrix else {continue};
            let path=reference(reference_text,"guid").and_then(|guid|catalog.as_ref()?.get(&guid.to_lowercase()).cloned());
            let Some(path)=path else {missing+=1;issues.push(format!("{} · {}: {}\n{}", crate::name_from_text(doc.game_object.as_deref().and_then(|id|objects.get(id)).map(|d|d.text.as_ref()).unwrap_or("")).unwrap_or_else(|| doc.file_id.clone()), tr("资源 GUID 未找到"), reference_text, tr("GUID 索引尚未完成，或资源不存在、属于内置资源。")));continue};
            if number(&doc.text,"m_DrawMode",0.0)!=0.0 {missing+=1;issues.push(format!("{} · fileID {} · {}: {}",path.display(),doc.file_id,tr("暂不支持此 Sprite 绘制模式"),field(&doc.text,"m_DrawMode").unwrap_or("")));continue}
            let c=vector(&doc.text,"m_Color",&["r","g","b","a"],&[1.0;4]);
            let byte=|v:f64|(v.clamp(0.0,1.0)*255.0).round() as u8;

            sprites.push(Sprite{baked:None,ui:None,component: doc.file_id.clone(), selectable: doc.game_object.as_deref().and_then(|id|flags.get(id)).is_none_or(|flags|flags.selectable),name: crate::name_from_text(doc.game_object.as_deref().and_then(|id|objects.get(id)).map(|d|d.text.as_ref()).unwrap_or("")).unwrap_or_else(|| doc.file_id.clone()),path,id,spatial:matrix,matrix:matrix.into(),tint:crate::asset_viewer::preview_color([byte(c[0]),byte(c[1]),byte(c[2]),byte(c[3])]),flip:[number(&doc.text,"m_FlipX",0.0)!=0.0,number(&doc.text,"m_FlipY",0.0)!=0.0],order:renderer_order(doc,matrix,axis)});
        }
        for doc in documents.iter().filter(|d|d.type_name=="Tilemap") {
            let Some(object)=doc.game_object.as_deref() else {continue};
            if flags.get(object).is_some_and(|f|!f.visible) || number(&doc.text,"m_Enabled",1.0)==0.0 {continue;}
            let Some(transform)=by_object.get(object) else {continue};
            let Some(base)=world(transform,&transforms,&objects,&mut HashSet::new()) else {continue};
            let renderer=documents.iter().find(|d|d.type_name=="TilemapRenderer" && d.game_object==doc.game_object);
            if renderer.is_none_or(|r|number(&r.text,"m_Enabled",1.0)==0.0) {continue;}
            let renderer=renderer.unwrap();

            let mut ancestor=Some(*transform);
            let mut grid=None;
            let mut visited=HashSet::new();
            while let Some(id)=ancestor {
                if !visited.insert(id) {break;}
                let Some(t)=transforms.get(id) else {break};
                grid=documents.iter().find(|d|d.type_name=="Grid" && d.game_object==t.game_object);
                if grid.is_some() {break;}
                ancestor=t.parent_transform.as_deref().filter(|id|*id!="0");
            }
            let Some(grid)=grid else {missing+=1;issues.push(format!("{}: Grid not found",doc.file_id));continue};
            let size=vector(&grid.text,"m_CellSize",&["x","y","z"],&[1.0;3]);
            let gap=vector(&grid.text,"m_CellGap",&["x","y","z"],&[0.0;3]);
            let layout=number(&grid.text,"m_CellLayout",0.0) as i32;
            let swizzle=number(&grid.text,"m_CellSwizzle",0.0) as usize;
            if !matches!(layout,0|2|3) || swizzle>5 {
                missing+=1;issues.push(format!("{}: unsupported Grid CellLayout {layout} / CellSwizzle {swizzle}",doc.file_id));continue;
            }
            let Some(data)=crate::tilemap::preview_data(&doc.text,cancelled,paused) else {return Prepared::default()};

            let selectable=flags.get(object).is_none_or(|f|f.selectable);
            let mut references:HashMap<usize,Option<(PathBuf,String)>>=HashMap::new();
            let mut failed=0;
            for cell in &data.cells {
                if !checkpoint(cancelled,paused) {return Prepared::default();}
                let resolved=references.entry(cell.sprite).or_insert_with(|| {
                    data.sprites.get(cell.sprite).and_then(|value|crate::tilemap::sprite_reference(value))
                        .and_then(|(guid,id)|Some((catalog.as_ref()?.get(&guid)?.clone(),id)))
                }).clone();
                let resolved=resolved.or_else(|| {
                    let (guid,id)=crate::tilemap::sprite_reference(data.assets.get(cell.tile)?)?;
                    let path=catalog.as_ref()?.get(&guid)?.clone();
                    let sprite=tile_assets.entry((path.clone(),id.clone())).or_insert_with(|| {
                        if std::fs::metadata(&path).ok()?.len()>4*1024*1024 {return None;}
                        let text=std::fs::read_to_string(&path).ok()?;
                        let docs=crate::parse_unity_yaml(&text);
                        field(&docs.iter().find(|d|d.file_id==id)?.text,"m_Sprite").map(str::to_owned)
                    });
                    let (guid,id)=crate::tilemap::sprite_reference(sprite.as_deref()?)?;
                    Some((catalog.as_ref()?.get(&guid)?.clone(),id))
                });
                let Some((path,id))=resolved else {failed+=1;continue};
                let Some(position)=crate::tilemap::cell_position(cell.position,data.anchor,[size[0],size[1],size[2]],[gap[0],gap[1],gap[2]],layout,swizzle) else {continue};
                let mut translation=Matrix::identity();for axis in 0..3 {translation.0[axis][3]=position[axis];}
                let matrix=base.mul(translation).mul(data.orientation).mul(data.matrices.get(cell.matrix).copied().unwrap_or_else(Matrix::identity));
                let color=data.colors.get(cell.color).copied().unwrap_or([1.0;4]);
                let rgba=std::array::from_fn(|i|(color[i]*data.tint[i]).clamp(0.0,1.0).mul_add(255.0,0.0).round() as u8);
                sprites.push(Sprite{baked:None,ui:None,component:doc.file_id.clone(),selectable,name:doc.file_id.clone(),path,id,spatial:matrix,matrix:matrix.into(),tint:crate::asset_viewer::preview_color(rgba),flip:[false;2],order:renderer_order(renderer,base,axis)});
            }
            if failed>0 {missing+=failed;issues.push(format!("{}: {failed} unresolved Tile/Sprite references (indexing or loading may still be in progress)",doc.file_id));}
        }
    let mut spine_cache:HashMap<String,Result<(PathBuf,Arc<crate::spine_preview::Baked>),String>>=HashMap::new();
    let mut baked_bytes=0usize;
    for doc in documents.iter().filter(|d|d.type_name=="MonoBehaviour" && field(&d.text,"skeletonDataAsset").is_some()) {
        if !checkpoint(cancelled,paused) {return Prepared::default();}
        let Some(object)=doc.game_object.as_deref() else {continue};
        if flags.get(object).is_some_and(|f|!f.visible) || number(&doc.text,"m_Enabled",1.0)==0.0 {continue;}
        let Some(transform)=by_object.get(object) else {continue};
        let Some(matrix)=world(transform,&transforms,&objects,&mut HashSet::new()) else {continue};
        let Some(catalog)=catalog else {missing+=1;issues.push(format!("{}: Spine 资源索引尚未完成",doc.file_id));continue;};
        let key=format!("{}|{}",field(&doc.text,"skeletonDataAsset").unwrap_or(""),field(&doc.text,"initialSkinName").unwrap_or(""));
        let baked=spine_cache.entry(key).or_insert_with(|| {
            let (path,baked)=crate::spine_preview::bake(&doc.text,catalog,cancelled,paused)?;
            baked_bytes+=baked.image.pixels.len()*4;
            if baked_bytes>PREVIEW_TEXTURE_BUDGET {return Err("Spine 静态缓存超过 2 GiB".into());}
            Ok((path,Arc::new(baked)))
        });
        let (path,baked)=match baked {Ok(value)=>value,Err(error)=>{missing+=1;issues.push(format!("{}: {error}",doc.file_id));continue;}};
        if baked.approximate {missing+=1;issues.push(format!("{}: Spine 特殊混合 / 双色染色为近似显示",doc.file_id));}
        let renderer=documents.iter().find(|d|d.type_name=="MeshRenderer" && d.game_object==doc.game_object).unwrap_or(doc);
        if number(&renderer.text,"m_Enabled",1.0)==0.0 {continue;}
        sprites.push(Sprite{baked:Some(baked.clone()),ui:None,component:doc.file_id.clone(),selectable:flags.get(object).is_none_or(|f|f.selectable),name:crate::name_from_text(&objects.get(object).map_or("",|d|d.text.as_ref())).unwrap_or_else(||doc.file_id.clone()),path:path.clone(),id:"spine".into(),spatial:matrix,matrix:matrix.into(),tint:Color32::WHITE,flip:[number(&doc.text,"initialFlipX",0.0)!=0.0,number(&doc.text,"initialFlipY",0.0)!=0.0],order:renderer_order(renderer,matrix,axis)});
    }
    for doc in documents.iter().filter(|d|matches!(d.type_name.as_str(),"MeshFilter"|"SkinnedMeshRenderer")) {
        if !checkpoint(cancelled,paused){return Prepared::default();}
        let Some(object)=doc.game_object.as_deref() else{continue};
        if flags.get(object).is_some_and(|f|!f.visible){continue;}
        let renderer=if doc.type_name=="SkinnedMeshRenderer" {Some(doc)}else{documents.iter().find(|r|r.type_name=="MeshRenderer" && r.game_object==doc.game_object)};
        let Some(renderer)=renderer else{continue};if number(&renderer.text,"m_Enabled",1.0)==0.0{continue;}
        let Some(mesh)=field(&doc.text,"m_Mesh").filter(|v|reference(v,"fileID").is_some_and(|id|id!="0")) else{continue};
        let Some(matrix)=by_object.get(object).and_then(|id|world(id,&transforms,&objects,&mut HashSet::new())) else{continue};
        let Some(catalog)=catalog else{missing+=1;issues.push(format!("{}: 模型资源索引尚未完成",doc.file_id));continue;};
        let mut bones=HashMap::new();let mut duplicate_bones=HashSet::new();
        if doc.type_name=="SkinnedMeshRenderer" {
            if let Some((_,tail))=doc.text.split_once("  m_Bones:") {
                for line in tail.lines().skip(1).take_while(|l|l.trim_start().starts_with("- ")) {
                    let Some(id)=reference(line.trim().trim_start_matches('-').trim(),"fileID") else{continue};
                    let Some(bone)=transforms.get(id.as_str()) else{continue};
                    let Some(name)=bone.game_object.as_deref().and_then(|id|objects.get(id)).and_then(|o|crate::name_from_text(&o.text)) else{continue};
                    let Some(matrix)=world(&id,&transforms,&objects,&mut HashSet::new()) else{continue};
                    if bones.insert(name.clone(),matrix).is_some(){duplicate_bones.insert(name);}
                }
            }
            for name in duplicate_bones {bones.remove(&name);}
        }
        let local=if reference(mesh,"guid").is_none() {reference(mesh,"fileID").and_then(|id|documents.iter().find(|d|d.file_id==id && d.type_name=="Mesh")).map(|d|d.text.as_ref())}else{None};
        match crate::model_preview::bake(mesh,&renderer.text,catalog,matrix,&bones,local) {
            Ok((baked,warnings))=>{
                baked_bytes+=baked.image.pixels.len()*4;
                if baked_bytes>PREVIEW_TEXTURE_BUDGET{missing+=1;issues.push("静态模型 / Spine 缓存超过 2 GiB".into());continue;}
                for w in warnings{missing+=1;issues.push(format!("{}: {w}",doc.file_id));}
                use std::hash::{Hash,Hasher};
                let mut image_hash=std::collections::hash_map::DefaultHasher::new();baked.image.pixels.hash(&mut image_hash);
                let path=PathBuf::from(format!("__model_preview_{}_{:x}",doc.file_id,image_hash.finish()));
                sprites.push(Sprite{baked:Some(Arc::new(baked)),ui:None,component:renderer.file_id.clone(),selectable:flags.get(object).is_none_or(|f|f.selectable),name:crate::name_from_text(objects.get(object).map_or("",|d|d.text.as_ref())).unwrap_or_else(||doc.file_id.clone()),path,id:"model".into(),spatial:Matrix::identity(),matrix:Matrix::identity().into(),tint:Color32::WHITE,flip:[false;2],order:renderer_order(renderer,matrix,axis)});
            }
            Err(e)=>{missing+=1;issues.push(format!("{}: {e}",doc.file_id));}
        }
    }
    sprites.sort_by_key(|s|s.order);
    Prepared {sprites,missing,issues}
}

struct Texture {
    stamp: (Option<SystemTime>, Option<SystemTime>),
    receiver: Option<mpsc::Receiver<Result<(egui::ColorImage, String), String>>>,
    handle: Option<egui::TextureHandle>,
    meta: String,
    alpha: Vec<u8>,
    error: Option<String>,
    regions: HashMap<String,Option<Region>>,
}
#[derive(Clone, Copy)]
pub(crate) struct Matrix(pub(crate) [[f64; 4]; 4]);
impl Matrix {
    pub(crate) fn identity() -> Self { Self(std::array::from_fn(|row| std::array::from_fn(|column| if row == column { 1.0 } else { 0.0 }))) }
    pub(crate) fn mul(self, rhs: Self) -> Self { Self(std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| self.0[i][k]*rhs.0[k][j]).sum()))) }
    pub(crate) fn point(self, x: f64, y: f64) -> [f64; 2] { [self.0[0][0]*x+self.0[0][1]*y+self.0[0][3], self.0[1][0]*x+self.0[1][1]*y+self.0[1][3]] }
}
pub(crate) fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix("  ")?.strip_prefix(name)?.strip_prefix(':').map(str::trim))
}
pub(crate) fn number(text: &str, name: &str, default: f64) -> f64 { field(text,name).and_then(|v| v.parse().ok()).filter(|v: &f64| v.is_finite()).unwrap_or(default) }
pub(crate) fn reference(value: &str, name: &str) -> Option<String> {
    value.trim_matches(|c| c=='{' || c=='}').split(',').find_map(|part| {let (key,value)=part.trim().split_once(':')?; (key==name).then(|| value.trim().to_owned())})
}
pub(crate) fn vector(text: &str, name: &str, axes: &[&str], fallback: &[f64]) -> Vec<f64> {
    field(text,name).and_then(|v| crate::inline_vector(v,axes)).unwrap_or_else(|| fallback.to_vec())
}
pub(crate) fn local(doc: &Document, root: bool) -> Matrix {
    let p=vector(&doc.text,"m_LocalPosition", &["x","y","z"], &[0.0;3]);
    let mut s=vector(&doc.text,"m_LocalScale", &["x","y","z"], &[1.0;3]);
    if root && s.iter().all(|value| *value == 0.0) { s.fill(1.0); }
    let q=vector(&doc.text,"m_LocalRotation", &["x","y","z","w"], &[0.0,0.0,0.0,1.0]);
    let norm=q.iter().map(|v|v*v).sum::<f64>().sqrt();
    let (x,y,z,w)=if norm>0.0 {(q[0]/norm,q[1]/norm,q[2]/norm,q[3]/norm)} else {(0.0,0.0,0.0,1.0)};
    Matrix([
        [(1.0-2.0*(y*y+z*z))*s[0],2.0*(x*y-z*w)*s[1],2.0*(x*z+y*w)*s[2],if root {0.0}else{p[0]}],
        [2.0*(x*y+z*w)*s[0],(1.0-2.0*(x*x+z*z))*s[1],2.0*(y*z-x*w)*s[2],if root {0.0}else{p[1]}],
        [2.0*(x*z-y*w)*s[0],2.0*(y*z+x*w)*s[1],(1.0-2.0*(x*x+y*y))*s[2],if root {0.0}else{p[2]}],
        [0.0,0.0,0.0,1.0],
    ])
}
pub(crate) fn world(id: &str, transforms: &HashMap<&str,&Document>, objects: &HashMap<&str,&Document>, seen: &mut HashSet<String>) -> Option<Matrix> {
    if !seen.insert(id.into()) {return None}
    let doc=transforms.get(id)?;
    if doc.game_object.as_deref().and_then(|id| objects.get(id)).is_some_and(|go| number(&go.text,"m_IsActive",1.0)==0.0) {return None}
    let parent=doc.parent_transform.as_deref().filter(|id| *id!="0");
    if let Some(parent)=parent {Some(world(parent,transforms,objects,seen)?.mul(local(doc,false)))} else {Some(local(doc,true))}
}
// Only the six XY affine coefficients are needed by a prepared sprite.
#[derive(Clone,Copy)]
struct Projection([[f64;3];2]);
impl From<Matrix> for Projection {
    fn from(m:Matrix) -> Self {Self([[m.0[0][0],m.0[0][1],m.0[0][3]],[m.0[1][0],m.0[1][1],m.0[1][3]]])}
}
impl Projection {
    fn point(self,x:f64,y:f64) -> [f64;2] {self.0.map(|r|r[0]*x+r[1]*y+r[2])}
    fn full(self) -> Matrix {let mut m=Matrix::identity();for i in 0..2 {m.0[i][0]=self.0[i][0];m.0[i][1]=self.0[i][1];m.0[i][3]=self.0[i][2];}m}
}
struct Sprite { spatial: Matrix, baked: Option<Arc<crate::spine_preview::Baked>>, ui: Option<UiImage>, component: String, selectable: bool, name: String, path: PathBuf, id: String, matrix: Projection, tint: Color32, flip: [bool;2], order: (i64,i64,i64), }
#[derive(Clone, Copy)]
pub(crate) struct Region { pub(crate) rect: [f64;4], pub(crate) pivot: [f64;2], pub(crate) ppu: f64, pub(crate) border: [f64;4] }
struct Quad {
    texture: egui::TextureId, points: [[f64;2];4], outline: [[f64;2];4], uv: [Pos2;4], tint: Color32,
    path: Option<PathBuf>, component: String, selectable: bool, order: (i64,i64,i64),
}
// Invert the projected affine rectangle, including rotation and negative scale.
fn quad_coordinates(points: [Pos2;4], point: Pos2) -> Option<[f32;2]> {
    let u=points[1]-points[0]; let v=points[3]-points[0]; let p=point-points[0];
    let determinant=u.x*v.y-u.y*v.x;
    if determinant.abs()<1e-6 {return None}
    let a=(p.x*v.y-p.y*v.x)/determinant;
    let b=(u.x*p.y-u.y*p.x)/determinant;
    (a>=0.0 && a<=1.0 && b>=0.0 && b<=1.0).then_some([a,b])
}
fn meta_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {text.lines().find_map(|l| l.trim().strip_prefix(key)?.strip_prefix(':').map(str::trim))}
pub(crate) fn region(meta: &str, id: &str, size: [usize;2]) -> Option<Region> {
    let pivot_for = |block: &str, alignment_key: &str, pivot_key: &str| {
        let alignment = meta_value(block, alignment_key).unwrap_or("0");
        match alignment {
            "1" => [0.0, 1.0], "2" => [0.5, 1.0], "3" => [1.0, 1.0],
            "4" => [0.0, 0.5], "5" => [1.0, 0.5], "6" => [0.0, 0.0],
            "7" => [0.5, 0.0], "8" => [1.0, 0.0],
            "9" => meta_value(block, pivot_key).and_then(|v| crate::inline_vector(v, &["x", "y"])).map(|v| [v[0], v[1]]).unwrap_or([0.5, 0.5]),
            _ => [0.5, 0.5],
        }
    };
    let mode=meta_value(meta,"spriteMode")?;
    let ppu=meta_value(meta,"spritePixelsToUnits").and_then(|v|v.parse::<f64>().ok()).filter(|v|v.is_finite()&&*v>0.0)?;
    // Polygon is a single sprite too. Preview its source rectangle and alpha;
    // custom outline/physics vertices are deliberately not used to clip the quad.
    if matches!(mode, "1" | "3") && id=="21300000" {
        let pivot=pivot_for(meta,"spriteAlignment","spritePivot");
        return Some(Region {rect:[0.0,0.0,size[0] as f64,size[1] as f64],pivot:[pivot[0],pivot[1]],ppu,border:meta_value(meta,"spriteBorder").and_then(|v|crate::inline_vector(v,&["x","y","z","w"])).map(|v|[v[0],v[1],v[2],v[3]]).unwrap_or([0.0;4])})
    }
    if mode!="2" {return None}
    let legacy_name=meta.lines().find_map(|line|line.trim().split_once(':').filter(|(key,_)|key.trim()==id).map(|(_,value)|value.trim()));
    for block in meta.split("    - serializedVersion:").skip(1) {
        let internal=meta_value(block,"internalID");
        if internal!=Some(id) && !legacy_name.is_some_and(|name|meta_value(block,"name")==Some(name)) {continue}
        let rect=block.split_once("      rect:")?.1;
        let scalar=|key: &str| meta_value(rect,key)?.parse::<f64>().ok().filter(|v|v.is_finite());
        let rect=[scalar("x")?,scalar("y")?,scalar("width")?,scalar("height")?];
        if rect[0]<0.0 || rect[1]<0.0 || rect[2]<=0.0 || rect[3]<=0.0 || rect[0]+rect[2]>size[0] as f64 || rect[1]+rect[3]>size[1] as f64 {return None}
        let pivot=pivot_for(block,"alignment","pivot");
        return Some(Region{rect,pivot:[pivot[0],pivot[1]],ppu,border:meta_value(block,"border").and_then(|v|crate::inline_vector(v,&["x","y","z","w"])).map(|v|[v[0],v[1],v[2],v[3]]).unwrap_or([0.0;4])})
    }
    None
}
// Build all known slice IDs once instead of rescanning the whole metadata per sprite.
fn region_index(meta:&str,size:[usize;2]) -> HashMap<String,Option<Region>> {
    let mut result=HashMap::new();
    if meta_value(meta,"spriteMode")!=Some("2") {
        result.insert("21300000".into(),region(meta,"21300000",size));return result;
    }
    let header=format!("spriteMode: 2\nspritePixelsToUnits: {}\n",meta_value(meta,"spritePixelsToUnits").unwrap_or("0"));
    let mut legacy:HashMap<&str,Vec<&str>>=HashMap::new();
    for line in meta.lines() {if let Some((key,value))=line.trim().split_once(':') {if key.trim().parse::<i64>().is_ok() {legacy.entry(value.trim()).or_default().push(key.trim());}}}
    for block in meta.split("    - serializedVersion:").skip(1) {
        let mut ids=Vec::new();
        if let Some(id)=meta_value(block,"internalID") {ids.push(id);}
        if let Some(old)=meta_value(block,"name").and_then(|name|legacy.get(name)) {ids.extend(old.iter().copied());}
        if ids.is_empty() {continue;}
        let synthetic=format!("{header}    - serializedVersion:{block}");
        let resolved=region(&synthetic,meta_value(block,"internalID").unwrap_or(ids[0]),size).or_else(|| {
            // Legacy IDs map to names in older Unity metadata.
            let mapping=format!("{header}{}: {}\n    - serializedVersion:{block}",ids[0],meta_value(block,"name").unwrap_or(""));region(&mapping,ids[0],size)
        });
        for id in ids {result.insert(id.into(),resolved);}
    }
    result
}
#[derive(Clone)]
struct UiImage {
    size: [f64;2], pivot: [f64;2], mode: i64, fill_center: bool,
    preserve_aspect: bool, ppu_multiplier: f64, reference_ppu: f64,
    raw: bool, uv_rect: [f64;4], fill: f64, fill_method: i64, fill_origin: i64,
}
struct UiText {
    component: String, content: String, layout: crate::ui_preview::Layout, color: Color32,
    font_size: f64, horizontal: f64, vertical: f64, wrap: bool,
    selectable: bool, order: (i64,i64,i64),
    family: egui::FontFamily,
    best_fit: Option<[f64;2]>,
    visible_center: bool,
    align_by_geometry: bool,
    outlines: Vec<TextOutline>,
    canvas_alpha: f64,
}
#[derive(Clone)]
struct TextOutline { color:Color32, distance:[f64;2], use_graphic_alpha:bool }
fn text_outline(doc:&Document,catalog:Option<&Arc<HashMap<String,PathBuf>>>) -> Option<TextOutline> {
    if doc.text.len()>128*1024 || number(&doc.text,"m_Enabled",1.0)==0.0 {return None;}
    let script=field(&doc.text,"m_Script").and_then(|v|reference(v,"guid"))
        .and_then(|guid|catalog?.get(&guid.to_lowercase())).and_then(|path|path.file_stem()).and_then(|name|name.to_str()).unwrap_or("");
    let class=field(&doc.text,"m_EditorClassIdentifier").unwrap_or("");
    if ![doc.type_name.as_str(),script,class].iter().any(|name|name.to_ascii_lowercase().contains("outline")) {return None;}
    field(&doc.text,"m_EffectColor")?;
    let distance=vector(&doc.text,"m_EffectDistance",&["x","y"],&[1.0,1.0]);
    if distance.iter().any(|v|!v.is_finite()) {return None;}
    Some(TextOutline {color:tint(&doc.text,"m_EffectColor"),distance:[distance[0].clamp(-64.0,64.0),distance[1].clamp(-64.0,64.0)],use_graphic_alpha:number(&doc.text,"m_UseGraphicAlpha",1.0)!=0.0})
}
fn tint(text: &str, key: &str) -> Color32 {
    let c=vector(text,key,&["r","g","b","a"],&[1.0;4]);
    let b=|v:f64|(v.clamp(0.0,1.0)*255.0).round() as u8;
    crate::asset_viewer::preview_color([b(c[0]),b(c[1]),b(c[2]),b(c[3])])
}
fn adjusted_slices(size: f64, first: f64, last: f64) -> [f64;4] {
    let total=first+last;
    let ratio=if total>size && total>0.0 {size/total} else {1.0};
    [0.0,first*ratio,size-last*ratio,size]
}
fn ui_quads(sprite: &Sprite, image: &UiImage, region: &Region, texture: egui::TextureId, texture_size: [usize;2]) -> Vec<Quad> {
    let [x,y,w,h]=region.rect;
    let mut size=image.size;
    if image.preserve_aspect && image.mode!=1 && w>0.0 && h>0.0 {
        let factor=(size[0]/w).min(size[1]/h);size=[w*factor,h*factor];
    }
    if size[0]<=0.0 || size[1]<=0.0 {return Vec::new()}
    let mut dx=vec![0.0,size[0]];let mut dy=vec![0.0,size[1]];
    let mut sx=vec![x,x+w];let mut sy=vec![y,y+h];
    let sliced=image.mode==1 && !image.raw && region.border.iter().any(|v|*v>0.0);
    if sliced {
        let mut b=region.border.map(|v|v.max(0.0));
        if b[0]+b[2]>w {let r=w/(b[0]+b[2]);b[0]*=r;b[2]*=r;}
        if b[1]+b[3]>h {let r=h/(b[1]+b[3]);b[1]*=r;b[3]*=r;}
        let ppu=(region.ppu/image.reference_ppu*image.ppu_multiplier).max(0.001);
        dx=adjusted_slices(size[0],b[0]/ppu,b[2]/ppu).to_vec();
        dy=adjusted_slices(size[1],b[1]/ppu,b[3]/ppu).to_vec();
        sx=vec![x,x+b[0],x+w-b[2],x+w];sy=vec![y,y+b[1],y+h-b[3],y+h];
    } else if image.mode==3 && !image.raw {
        let axis=if image.fill_method==0 {0} else {1};
        let (destination,source)=if axis==0 {(&mut dx,&mut sx)} else {(&mut dy,&mut sy)};
        if image.fill_origin==0 {destination[1]*=image.fill;source[1]=source[0]+(source[1]-source[0])*image.fill;}
        else {destination[0]=destination[1]*(1.0-image.fill);source[0]=source[1]-(source[1]-source[0])*image.fill;}
    }
    let mut result=Vec::new();
    for j in 0..dy.len()-1 {for i in 0..dx.len()-1 {
        if sliced && i==1 && j==1 && !image.fill_center {continue}
        if dx[i+1]<=dx[i] || dy[j+1]<=dy[j] {continue}
        let corners=[[0usize,0usize],[1,0],[1,1],[0,1]];
        let points=corners.map(|[a,b]|sprite.matrix.point(dx[i+a]-image.pivot[0]*size[0],dy[j+b]-image.pivot[1]*size[1]));
        let uv=corners.map(|[a,b]|{
            if image.raw {egui::pos2((image.uv_rect[0]+image.uv_rect[2]*a as f64) as f32,(1.0-image.uv_rect[1]-image.uv_rect[3]*b as f64) as f32)}
            else {egui::pos2((sx[i+a]/texture_size[0] as f64) as f32,(1.0-sy[j+b]/texture_size[1] as f64) as f32)}
        });
        result.push(Quad{texture,points,outline:ui_rect_points(crate::ui_preview::Layout{matrix:sprite.matrix.full(),size:image.size,pivot:image.pivot}),uv,tint:sprite.tint,path:Some(sprite.path.clone()),component:sprite.component.clone(),selectable:sprite.selectable,order:sprite.order});
    }}
    result
}
fn ui_rect_points(layout: crate::ui_preview::Layout) -> [[f64;2];4] {
    [[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]].map(|[x,y]|layout.matrix.point((x-layout.pivot[0])*layout.size[0],(y-layout.pivot[1])*layout.size[1]))
}
fn text_origin(points:[Pos2;4],bounds:egui::Rect,alignment:[f32;2]) -> Pos2 {
    let right=points[1]-points[0];let down=points[0]-points[3];
    let x=(right.length()-bounds.width())*alignment[0]-bounds.min.x;
    let y=(down.length()-bounds.height())*alignment[1]-bounds.min.y;
    points[3]+right.normalized()*x+down.normalized()*y
}
fn text_alignment_bounds(layout:egui::Rect,glyphs:egui::Rect,visible_center:bool,geometry:bool) -> egui::Rect {
    if !glyphs.is_positive() || !glyphs.is_finite() {return layout;}
    if geometry {glyphs}
    else if visible_center {egui::Rect::from_min_max(Pos2::new(layout.min.x,glyphs.min.y),Pos2::new(layout.max.x,glyphs.max.y))}
    else {layout}
}
fn fit_text(min:f64,max:f64,size:Vec2,mut layout:impl FnMut(f32)->Arc<egui::Galley>) -> Arc<egui::Galley> {
    let mut low=min.min(max).clamp(1.0,512.0).ceil() as i32;
    let mut high=max.max(min).clamp(1.0,512.0).floor() as i32;
    let mut fitted=layout(low as f32);
    while low<=high {
        let font_size=low+(high-low)/2;let candidate=layout(font_size as f32);
        if candidate.size().x<=size.x+0.5 && candidate.size().y<=size.y+0.5 {fitted=candidate;low=font_size+1;} else {high=font_size-1;}
    }
    fitted
}
pub(crate) fn checkpoint(cancelled:&std::sync::atomic::AtomicBool,paused:Option<&std::sync::atomic::AtomicBool>) -> bool {
    while paused.is_some_and(|p|p.load(std::sync::atomic::Ordering::Relaxed)) {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {return false;}
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    !cancelled.load(std::sync::atomic::Ordering::Relaxed)
}
fn preview_path_row(ui:&mut egui::Ui,label:&str) {
    let height=ui.text_style_height(&egui::TextStyle::Small);
    let (rect,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),height),egui::Sense::hover());
    let mut row=ui.new_child(egui::UiBuilder::new().id_salt("preview_path_row").max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.set_clip_rect(ui.clip_rect().intersect(rect));
    row.add(egui::Label::new(egui::RichText::new(label).small()).wrap_mode(egui::TextWrapMode::Extend)).on_hover_text(label);
}
fn preview_status_row(ui:&mut egui::Ui,status:&str,complete:bool,paused:&std::sync::atomic::AtomicBool,details:Option<&str>) -> bool {
    let actions_ready=preview_actions_ready(ui,complete,paused.load(std::sync::atomic::Ordering::Relaxed));
    let height=ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Body));
    let (rect,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),height),egui::Sense::hover());
    let mut row=ui.new_child(egui::UiBuilder::new().id_salt("preview_status_row").max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.set_clip_rect(ui.clip_rect().intersect(rect));
    let is_paused=paused.load(std::sync::atomic::Ordering::Relaxed);
    let label=row.add(egui::Label::new(if !complete && is_paused {tr("已暂停合成预览")} else {status}).wrap_mode(egui::TextWrapMode::Extend));
    if let Some(details)=details {label.on_hover_text(details);}
    if !actions_ready {return false;}
    if row.button(tr(if is_paused {"继续"} else {"暂停"})).clicked() {paused.store(!is_paused,std::sync::atomic::Ordering::Relaxed);}
    row.button(tr("退出预览")).clicked()
}
fn preview_actions_ready(ui:&egui::Ui,complete:bool,paused:bool)->bool {
    let key=ui.make_persistent_id("preview_status_started");
    let now=ui.input(|input|input.time);
    let started=ui.ctx().data_mut(|data|{
        if complete {data.remove_temp::<f64>(key);return now;}
        if let Some(started)=data.get_temp::<f64>(key){started}else{data.insert_temp(key,now);now}
    });
    !complete && (paused || now-started>=5.0)
}
fn paint_zoom_badge(painter:&egui::Painter,rect:egui::Rect,zoom:f64) {
    let galley=painter.layout_no_wrap(format!("{} {:.0}%",tr("缩放"),zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE);
    let size=galley.size()+egui::vec2(24.0,12.0);
    let badge=egui::Rect::from_center_size(egui::pos2(rect.center().x,rect.bottom()-8.0-size.y*0.5),size);
    painter.rect_filled(badge,6.0,Color32::from_black_alpha(128));
    painter.galley(badge.min+egui::vec2(12.0,6.0),galley,Color32::WHITE);
}
fn paint_background(painter:&egui::Painter,rect:egui::Rect,background:&str,rgb:[u8;3]) {
    painter.add(background_shape(rect,background,rgb));
}
pub(crate) fn background_shape(rect:egui::Rect,background:&str,rgb:[u8;3]) -> egui::Shape {
    if background=="gray" || background=="custom" {
        let [r,g,b]=if background=="gray" {[128,128,128]} else {rgb};egui::Shape::rect_filled(rect,0.0,Color32::from_rgb(r,g,b))
    } else {
        let cell=12.0;
        let mut mesh=egui::Mesh::default();
        for y in 0..(rect.height()/cell).ceil() as usize {for x in 0..(rect.width()/cell).ceil() as usize {
            let tile=egui::Rect::from_min_size(rect.min+egui::vec2(x as f32*cell,y as f32*cell),Vec2::splat(cell)).intersect(rect);
            mesh.add_colored_rect(tile,Color32::from_gray(if (x+y)%2==0 {190} else {230}));
        }}
        egui::Shape::mesh(mesh)
    }
}
impl Preview {
    pub fn focus(&mut self, components: HashSet<String>) {
        self.open = true;
        self.focus_request = Some((components, std::time::Instant::now()));
    }
    pub fn available(documents: &[Document], path: Option<&std::path::Path>) -> bool {
        path.is_some_and(|p|p.extension().is_some_and(|e|e.eq_ignore_ascii_case("prefab") || e.eq_ignore_ascii_case("unity"))) && documents.iter().any(|d|
            d.type_name=="PrefabInstance" || d.type_name=="Tilemap" || matches!(d.type_name.as_str(),"MeshFilter"|"SkinnedMeshRenderer") || (d.type_name=="MonoBehaviour" && field(&d.text,"skeletonDataAsset").is_some()) || crate::ui_preview::graphic(d).is_some() || (d.type_name=="SpriteRenderer" && field(&d.text,"m_Sprite").and_then(|v|reference(v,"fileID")).is_some_and(|id|id!="0")))
    }
    pub fn release(&mut self) {
        self.scene3d=None;
        self.preparation=None;self.preparation_pending=false;self.world.release();
        if self.sdf.has_state() {let sdf=std::mem::take(&mut self.sdf);std::thread::spawn(move || drop(sdf));}
        let prepared=std::mem::take(&mut self.prepared);let textures=std::mem::take(&mut self.textures);
        if !prepared.sprites.is_empty() || !textures.is_empty() {std::thread::spawn(move || {drop(prepared);drop(textures);});}
        self.paused.store(false,std::sync::atomic::Ordering::Relaxed);self.focus_request=None;
    }
    fn prepare_background(&mut self, ctx:&egui::Context, documents:&[Document], flags:&HashMap<String,crate::hierarchy_controls::Flags>, catalog:Option<Arc<HashMap<String,PathBuf>>>) -> Arc<Prepared> {
        use std::hash::{Hash,Hasher};
        let mut hash=std::collections::hash_map::DefaultHasher::new();
        for doc in documents {doc.text.as_ptr().hash(&mut hash);doc.text.len().hash(&mut hash);}
        let mut keys:Vec<_>=flags.iter().collect();keys.sort_by_key(|(id,_)|*id);
        for (id,f) in keys {id.hash(&mut hash);f.visible.hash(&mut hash);f.selectable.hash(&mut hash);}
        catalog.as_ref().map(|c|Arc::as_ptr(c) as usize).hash(&mut hash);
        let axis=self.sort_axis.unwrap_or(1);axis.hash(&mut hash);
        let key=hash.finish();
        if self.preparation.as_ref().is_none_or(|job|job.key!=key) {
            self.preparation=None;self.prepared=Arc::default();self.preparation_pending=true;
            let (sender,receiver)=mpsc::channel();let cancelled=Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop=cancelled.clone();let paused=self.paused.clone();let docs=documents.to_vec();let flags=flags.clone();let repaint=ctx.clone();
            std::thread::spawn(move||{let result=prepare(&docs,&flags,catalog.as_ref(),&stop,axis,Some(&paused));if !stop.load(std::sync::atomic::Ordering::Relaxed) {let _=sender.send(result);repaint.request_repaint();}});
            self.preparation=Some(Preparation{key,receiver:Some(receiver),cancelled});
        }
        if let Some(job)=&self.preparation {
            if let Some(Ok(result))=job.receiver.as_ref().map(|receiver|receiver.try_recv()) {self.prepared=Arc::new(result);self.preparation_pending=false;}
        }
        if self.preparation_pending {ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        self.prepared.clone()
    }
    fn clear_prefabs(&mut self) {
        if self.prefabs.has_state() {self.prefabs.cancel();let prefabs=std::mem::take(&mut self.prefabs);std::thread::spawn(move || drop(prefabs));}
    }
    fn initialize_document(&mut self,documents:&[Document],identity:&str) -> usize {
        let first_document=documents.first().map_or(0,|doc|doc.text.as_ptr() as usize);
        let last_document=documents.last().map_or(0,|doc|doc.text.as_ptr() as usize);
        let changed_identity=self.identity!=identity;
        let prefab_count=match self.prefab_count {
            Some((first,last,len,count)) if !changed_identity && first==first_document && last==last_document && len==documents.len()=>count,
            _=>documents.iter().filter(|doc|doc.type_name=="PrefabInstance").count(),
        };
        if self.identity!=identity {self.clear_prefabs();self.identity=identity.into();self.release();self.three_dimensional=false;self.ui_presence=None;self.expand_prefabs=prefab_count<=AUTO_PREFAB_LIMIT;self.zoom=1.0;self.pan=Vec2::ZERO;self.next_asset_check=None;self.ui_viewport=None;}
        self.prefab_count=Some((first_document,last_document,documents.len(),prefab_count));
        prefab_count
    }
    pub(crate) fn hierarchy_header(&mut self,ui:&mut egui::Ui,documents:&[Document],identity:&str) {
        let count=self.initialize_document(documents,identity);
        ui.horizontal_wrapped(|ui| {
            ui.heading(tr("Hierarchy"));
            if count>0 {
                crate::theme::toggle_switch(ui,tr("展开嵌套Prefab"),&mut self.expand_prefabs)
                    .on_hover_text(format!("{}: {count}\n{}: {AUTO_PREFAB_LIMIT}",tr("当前文件中的 Prefab 实例"),tr("自动展开上限")));
            }
        });
        if self.expand_prefabs && count>0 {
            let catalog=ui.ctx().data_mut(|data|data.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
            let (_,pending)=self.prefabs.resolve(ui.ctx(),documents,catalog,self.paused.clone());
            if pending {ui.small(tr("正在展开 Prefab…"));}
        } else {self.clear_prefabs();}
    }
    pub(crate) fn hierarchy_documents(&self,identity:&str) -> Option<Arc<prefab::Expanded>> {
        if self.expand_prefabs && self.identity==identity {self.prefabs.snapshot()} else {None}
    }
    pub fn show(&mut self, ui: &mut egui::Ui, documents: &[Document], identity: &str, flags: &HashMap<String, crate::hierarchy_controls::Flags>, selected: Option<&str>) -> Option<String> {
        let prefab_count=self.initialize_document(documents,identity);
        if ui.ctx().data_mut(|data|data.get_temp::<bool>(egui::Id::new("asset_catalog_loading"))).unwrap_or(false) {
            // Indexing runs independently of preview workers. Offering their pause
            // control here can leave future preparation paused before it starts.
            let count=ui.ctx().data_mut(|data|data.get_temp::<usize>(egui::Id::new("asset_catalog_progress"))).unwrap_or(0);
            ui.horizontal(|ui| {ui.spinner();ui.label(format!("{} {}: {count}",tr("正在索引预览资源…"),tr("已索引资源")));});
            if ui.button(tr("退出预览")).clicked() {self.open=false;self.release();}
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            return None;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("静态预览"));
            crate::theme::preview_space_selector(ui,&mut self.three_dimensional);
            if ui.button(tr("适应窗口")).clicked(){self.zoom=1.0;self.pan=Vec2::ZERO;self.world.fit();if let Some((_,preview))=&mut self.scene3d {preview.fit();}}
            if self.three_dimensional && ui.button(tr("重置视角")).clicked(){if let Some((_,preview))=&mut self.scene3d {preview.reset();}}
            ui.label(tr("背景色"));
            ui.push_id("preview_background", |ui| crate::theme::background_selector(ui, &mut self.background));
            let rgb = self.background_rgb.get_or_insert([190,190,190]);
            ui.color_edit_button_srgb(rgb);
            if !self.three_dimensional {
            ui.label(tr("排序轴"));
            let axis=self.sort_axis.get_or_insert(1);
            ui.selectable_value(axis,0,"X");ui.selectable_value(axis,1,"Y");ui.selectable_value(axis,2,"Z");
            crate::theme::toggle_switch(ui,tr("辅助线"),&mut self.show_guides);
            }
        });
        let catalog=ui.ctx().data_mut(|data|data.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
        let (expanded,expanding)=if self.expand_prefabs {
            self.prefabs.resolve(ui.ctx(),documents,catalog.clone(),self.paused.clone())
        } else {
            if self.prefabs.has_state() {self.prefabs.cancel();let prefabs=std::mem::take(&mut self.prefabs);std::thread::spawn(move || drop(prefabs));}
            (None,false)
        };
        if prefab_count>0 && !self.expand_prefabs {
            ui.small(format!("{}: {prefab_count}",tr("未展开的 Prefab 实例")));
        }
        if expanding && expanded.is_none() {
            if preview_status_row(ui,tr("正在展开 Prefab…"),false,&self.paused,None) {self.open=false;self.release();}
            return None;
        }
        let resolved_flags=expanded.as_ref().map(|data|data.flags(flags));
        let flags=resolved_flags.as_deref().unwrap_or(flags);
        let documents=expanded.as_ref().map_or(documents,|data|data.documents.as_slice());
        // Hierarchy selects objects, while preview geometry is keyed by component.
        let selected_components=selection_components(documents,selected,expanded.as_deref());
        if let Some(data)=&expanded {
            crate::theme::preview_warning_details(ui,"prefab_expansion_warnings",tr("Prefab 展开存在限制"),&data.issues);
        }
        if self.three_dimensional {
            use std::hash::{Hash,Hasher};
            let mut hash=std::collections::hash_map::DefaultHasher::new();
            for doc in documents {doc.text.as_ptr().hash(&mut hash);doc.text.len().hash(&mut hash);}
            let mut keys:Vec<_>=flags.iter().collect();keys.sort_by_key(|(id,_)|*id);
            for (id,value) in keys {id.hash(&mut hash);value.visible.hash(&mut hash);}
            if let Some(catalog)=&catalog {catalog.len().hash(&mut hash);Arc::as_ptr(catalog).hash(&mut hash);}
            let key=hash.finish();
            if self.scene3d.as_ref().is_none_or(|(previous,_)|*previous!=key) {
                self.scene3d=Some((key,crate::model_preview::Preview::new_scene(ui.ctx(),documents.to_vec(),flags.clone(),catalog.unwrap_or_default())));
            }
            self.scene3d.as_mut().unwrap().1.show_scene(ui,&self.background,self.background_rgb.unwrap_or([190;3]));
            return None;
        }
        let first=documents.first().map_or(0,|d|d.text.as_ptr() as usize);
        let last=documents.last().map_or(0,|d|d.text.as_ptr() as usize);
        let has_ui=match self.ui_presence {
            Some((a,b,n,value)) if a==first && b==last && n==documents.len()=>value,
            _=>{let value=documents.iter().any(|doc|crate::ui_preview::graphic(doc).is_some() || (doc.type_name=="MonoBehaviour" && field(&doc.text,"skeletonDataAsset").is_some() && documents.iter().any(|t|t.type_name=="RectTransform" && t.game_object==doc.game_object)));self.ui_presence=Some((first,last,documents.len(),value));value},
        };
        if !has_ui {
            let picked=self.world.show(ui,documents,flags,catalog,self.sort_axis.unwrap_or(1),self.paused.clone(),&selected_components,&mut self.focus_request,&self.background,self.background_rgb.unwrap_or([190,190,190]),self.show_guides,expanding,&mut self.open);
            if !self.open {self.release();ui.ctx().request_repaint();}
            return picked.map(|id|expanded.as_ref().map_or_else(||id.clone(),|data|data.selection(id.clone())));
        }
        let objects: HashMap<_,_>=documents.iter().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
        let transforms: HashMap<_,_>=documents.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
        let by_object: HashMap<_,_>=transforms.values().filter_map(|d|Some((d.game_object.as_deref()?,d.file_id.as_str()))).collect();
        let mut missing=0;
        let mut issues=Vec::new();
        let mut builtin_ui_approximation=false;
        let mut sprites=Vec::new();
        let mut quads=Vec::new();
        let mut texts=Vec::new();
        let reference_viewport=documents.iter().filter(|doc|doc.type_name=="MonoBehaviour" && doc.text.len()<128*1024).find_map(|doc|field(&doc.text,"m_ReferenceResolution").and_then(|value|crate::inline_vector(value,&["x","y"]))).unwrap_or(vec![1280.0,720.0]);
        let viewport=self.ui_viewport.get_or_insert([reference_viewport[0].max(1.0),reference_viewport[1].max(1.0)]);
        if has_ui {
            ui.horizontal(|ui| {
                ui.small(tr("UI 参考尺寸"));
                ui.add(egui::DragValue::new(&mut viewport[0]).range(1.0..=8192.0).speed(1.0));
                ui.label("×");ui.add(egui::DragValue::new(&mut viewport[1]).range(1.0..=8192.0).speed(1.0));
            });
        }
        let viewport=*viewport;
        let ui_order=crate::ui_preview::hierarchy_order(documents);
        let mut layout_cache=HashMap::new();
        crate::ui_preview::seed_screen_canvases(documents, viewport, &mut layout_cache);

        let prepared=self.prepare_background(ui.ctx(),documents,flags,catalog.clone());
        missing+=prepared.missing;issues.extend(prepared.issues.iter().cloned());
        let mut outlines_by_object:HashMap<&str,Vec<TextOutline>>=HashMap::new();
        for component in documents {
            if let Some(object)=component.game_object.as_deref() {
                if let Some(outline)=text_outline(component,catalog.as_ref()) {outlines_by_object.entry(object).or_default().push(outline);}
            }
        }
        for doc in documents {
            use crate::ui_preview::Graphic;
            let Some(kind)=crate::ui_preview::graphic(doc) else {continue};
            let Some(object)=doc.game_object.as_deref() else {continue};
            if flags.get(object).is_some_and(|flags|!flags.visible) || number(&doc.text,"m_Enabled",1.0)==0.0 {continue}
            let Some(transform)=by_object.get(object) else {continue};
            let Some(layout)=crate::ui_preview::layout(transform,&transforms,&objects,viewport,&mut layout_cache,&mut HashSet::new()) else {
                missing+=1; issues.push(format!("{} · {}", doc.file_id, tr("节点或父节点未激活，或布局层级无效"))); continue;
            };
            if layout.size.iter().any(|v|*v<=0.0||!v.is_finite()) {
                missing+=1; issues.push(format!("{} · {}", doc.file_id, tr("UI 尺寸为零或无效"))); continue;
            }
            let corners=ui_rect_points(layout);
            let a=[corners[1][0]-corners[0][0],corners[1][1]-corners[0][1]];
            let b=[corners[3][0]-corners[0][0],corners[3][1]-corners[0][1]];
            if (a[0]*b[1]-a[1]*b[0]).abs()<1e-9 {
                missing+=1; issues.push(format!("{} · {}", doc.file_id, tr("节点或父节点缩放为零，UI 图形没有可见面积"))); continue;
            }
            let selectable=flags.get(object).is_none_or(|flags|flags.selectable);
            let order=(1_000_000_000,*ui_order.get(object).unwrap_or(&0),0);
            let color=tint(&doc.text,if kind==Graphic::Tmp {"m_fontColor"} else {"m_Color"});
            let alpha=crate::ui_preview::opacity(transform,documents,&transforms);
            // Asset colors are already premultiplied in gamma space; preserve that space.
            let color=color.gamma_multiply(alpha as f32);
            if color.a()==0 {
                missing+=1; issues.push(format!("{} · {}", doc.file_id, tr("UI 颜色或 CanvasGroup 透明度为零"))); continue;
            }
            if matches!(kind,Graphic::Text|Graphic::Tmp) {
                let content: String=crate::ui_preview::text_content(doc,kind).chars().take(65536).collect();
                if content.is_empty(){continue}
                let (horizontal,vertical)=if kind==Graphic::Tmp {
                    let h=number(&doc.text,"m_HorizontalAlignment",1.0) as i64;
                    let v=number(&doc.text,"m_VerticalAlignment",256.0) as i64;
                    let combined=number(&doc.text,"m_textAlignment",0.0) as i64;
                    let h=if combined!=0 {combined&255} else {h};let v=if combined!=0 {combined&65280} else {v};
                    (if h==2 {0.5}else if h==4 {1.0}else{0.0},if v==512 {0.5}else if v==1024 {1.0}else{0.0})
                } else {let align=crate::ui_preview::nested_number(&doc.text,"m_Alignment",0.0) as i64;((align%3) as f64*0.5,(align/3) as f64*0.5)};
                texts.push(UiText{component:doc.file_id.clone(),content,layout,color,
                    outlines:outlines_by_object.get(object).cloned().unwrap_or_default(),canvas_alpha:alpha,
                    family:self.fonts.family(ui.ctx(), crate::preview_fonts::reference(&doc.text, kind==Graphic::Tmp), catalog.as_ref()),
                    font_size:crate::ui_preview::nested_number(&doc.text,if kind==Graphic::Tmp {"m_fontSize"}else{"m_FontSize"},14.0).clamp(1.0,512.0),
                    best_fit:if kind==Graphic::Text && crate::ui_preview::nested_number(&doc.text,"m_BestFit",0.0)!=0.0 {Some([crate::ui_preview::nested_number(&doc.text,"m_MinSize",10.0).clamp(1.0,512.0),crate::ui_preview::nested_number(&doc.text,"m_MaxSize",40.0).clamp(1.0,512.0)])} else {None},
                    visible_center:kind==Graphic::Text && vertical==0.5,
                    align_by_geometry:kind==Graphic::Text && crate::ui_preview::nested_number(&doc.text,"m_AlignByGeometry",0.0)!=0.0,
                    horizontal,vertical,wrap:if kind==Graphic::Tmp {number(&doc.text,"m_enableWordWrapping",1.0)!=0.0} else {crate::ui_preview::nested_number(&doc.text,"m_HorizontalOverflow",0.0)==0.0},
                    selectable,order});continue;
            }
            let mode=if kind==Graphic::RawImage {0}else{number(&doc.text,"m_Type",0.0) as i64};
            let fill_method=number(&doc.text,"m_FillMethod",0.0) as i64;
            if mode==2 || (mode==3 && fill_method>1) {
                missing+=1;issues.push(format!("{} · {}",doc.file_id,tr("此 UI 绘制模式暂不支持")));continue;
            }
            let raw=kind==Graphic::RawImage;
            let image=UiImage{size:layout.size,pivot:layout.pivot,mode,raw,fill_center:number(&doc.text,"m_FillCenter",1.0)!=0.0,
                preserve_aspect:number(&doc.text,"m_PreserveAspect",0.0)!=0.0,ppu_multiplier:number(&doc.text,"m_PixelsPerUnitMultiplier",1.0).max(0.001),
                reference_ppu:documents.iter().find_map(|doc|field(&doc.text,"m_ReferencePixelsPerUnit").and_then(|v|v.parse::<f64>().ok())).filter(|v|v.is_finite()&&*v>0.0).unwrap_or(100.0),
                uv_rect:vector(&doc.text,"m_UVRect",&["x","y","width","height"],&[0.0,0.0,1.0,1.0]).try_into().unwrap(),
                fill:number(&doc.text,"m_FillAmount",1.0).clamp(0.0,1.0),fill_method,fill_origin:number(&doc.text,"m_FillOrigin",0.0) as i64};
            let reference_text=field(&doc.text,if raw {"m_Texture"}else{"m_Sprite"}).unwrap_or("{fileID: 0}");
            let id=reference(reference_text,"fileID").unwrap_or_else(||"0".into());
            if id=="0" {
                quads.push(Quad{texture:egui::TextureId::default(),points:ui_rect_points(layout),outline:ui_rect_points(layout),uv:[egui::epaint::WHITE_UV;4],tint:color,path:None,component:doc.file_id.clone(),selectable,order});continue;
            }
            if reference(reference_text,"guid").is_some_and(|guid|guid.starts_with("0000000000000000")) {
                builtin_ui_approximation=true;
                quads.push(Quad{texture:egui::TextureId::default(),points:ui_rect_points(layout),outline:ui_rect_points(layout),uv:[egui::epaint::WHITE_UV;4],tint:color,path:None,component:doc.file_id.clone(),selectable,order});continue;
            }
            let path=reference(reference_text,"guid").and_then(|guid|catalog.as_ref()?.get(&guid.to_lowercase()).cloned());
            let Some(path)=path else {missing+=1;issues.push(format!("{} · {} · {}",doc.file_id,tr("资源 GUID 未找到"),reference_text));continue};
            sprites.push(Sprite{baked:None,ui:Some(image),component:doc.file_id.clone(),selectable,name:crate::name_from_text(&objects.get(object).map(|doc|doc.text.as_ref()).unwrap_or("")).unwrap_or_else(||doc.file_id.clone()),path,id,spatial:layout.matrix,matrix:layout.matrix.into(),tint:color,flip:[false;2],order});
        }
        sprites.sort_by_key(|s|s.order);
        let used: HashSet<_>=sprites.iter().chain(prepared.sprites.iter()).map(|s|s.path.clone()).collect();
        self.textures.retain(|path,_|used.contains(path));
        let mut loading=false;
        let now = std::time::Instant::now();
        let check_assets = self.next_asset_check.is_none_or(|next| now >= next);
        if check_assets { self.next_asset_check = Some(now + std::time::Duration::from_secs(1)); }
        let mut checked_paths=HashSet::new();
        let mut reported_errors=HashSet::new();
        let resource_budget=std::time::Instant::now();let mut uploaded=false;
        for sprite in sprites.iter().chain(prepared.sprites.iter()) {
            if let Some(baked)=&sprite.baked {
                if !self.textures.contains_key(&sprite.path) {
                    if self.paused.load(std::sync::atomic::Ordering::Relaxed) || uploaded || resource_budget.elapsed().as_millis()>=4 {loading=true;continue;}
                    let cache_bytes:usize=self.textures.values().filter_map(|t|t.handle.as_ref()).map(|h|h.size()[0]*h.size()[1]*5).sum();
                    if cache_bytes+baked.image.pixels.len()*5>PREVIEW_TEXTURE_BUDGET {missing+=1;issues.push("Spine 贴图超过预览缓存预算".into());continue;}
                    let handle=ui.ctx().load_texture(sprite.path.display().to_string(),baked.image.clone(),egui::TextureOptions::LINEAR);
                    self.textures.insert(sprite.path.clone(),Texture{stamp:(None,None),receiver:None,handle:Some(handle),meta:String::new(),alpha:baked.image.pixels.iter().map(|p|p.a()).collect(),error:None,regions:HashMap::new()});
                    uploaded=true;
                }
                let handle=self.textures[&sprite.path].handle.as_ref().unwrap();let region=baked.region;
                let mut matrix=sprite.matrix;let mut order=sprite.order;let mut color=sprite.tint;
                if let Some(doc)=documents.iter().find(|d|d.file_id==sprite.component) {
                    if let Some(object)=doc.game_object.as_deref() {
                        if let Some(transform)=by_object.get(object).filter(|id|transforms.get(**id).is_some_and(|t|t.type_name=="RectTransform")) {
                            let Some(layout)=crate::ui_preview::layout(transform,&transforms,&objects,viewport,&mut layout_cache,&mut HashSet::new()) else {continue};
                            matrix=layout.matrix.into();order=(1_000_000_000,*ui_order.get(object).unwrap_or(&0),0);
                            color=tint(&doc.text,"m_Color").gamma_multiply(crate::ui_preview::opacity(transform,documents,&transforms) as f32);
                        }
                    }
                }
                let [_,_,w,h]=region.rect;
                let corners=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
                let points=corners.map(|[a,b]|matrix.point((a-region.pivot[0])*w/region.ppu*if sprite.flip[0]{-1.0}else{1.0},(b-region.pivot[1])*h/region.ppu*if sprite.flip[1]{-1.0}else{1.0}));
                let uv=corners.map(|[a,b]|egui::pos2(a as f32,(1.0-b) as f32));
                quads.push(Quad{texture:handle.id(),points,outline:points,uv,tint:color,path:Some(sprite.path.clone()),component:sprite.component.clone(),selectable:sprite.selectable,order});
                continue;
            }
            if self.paused.load(std::sync::atomic::Ordering::Relaxed) && self.textures.get(&sprite.path).is_none_or(|t|t.handle.is_none()) {continue;}
            if checked_paths.contains(&sprite.path) && !self.textures.contains_key(&sprite.path) {continue;}
            let meta_path=PathBuf::from(format!("{}.meta",sprite.path.display()));
            let stamp = if checked_paths.insert(sprite.path.clone()) && (check_assets || !self.textures.contains_key(&sprite.path)) {
                (std::fs::metadata(&sprite.path).and_then(|m|m.modified()).ok(),
                 std::fs::metadata(&meta_path).and_then(|m|m.modified()).ok())
            } else { self.textures[&sprite.path].stamp };
            if self.textures.get(&sprite.path).is_none_or(|t|t.stamp!=stamp) {
                if self.paused.load(std::sync::atomic::Ordering::Relaxed) || resource_budget.elapsed().as_millis()>=4 || self.textures.values().filter(|t|t.receiver.is_some()).count() >= 2 { loading=true; continue; }
                let (sender,receiver)=mpsc::channel(); let path=sprite.path.clone();let ctx=ui.ctx().clone();
                std::thread::spawn(move||{
                    let result=(||{
                        let mut reader=image::ImageReader::open(&path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
                        let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
                        let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
                        let meta=std::fs::read_to_string(meta_path).unwrap_or_default();
                        let display = egui::ColorImage {
                            size: [image.width() as usize,image.height() as usize],
                            pixels: image.pixels().map(|pixel|crate::asset_viewer::preview_color(pixel.0)).collect(),
                        };
                        Ok((display,meta))
                    })();let _=sender.send(result);ctx.request_repaint();
                });
                self.textures.insert(sprite.path.clone(),Texture{stamp,receiver:Some(receiver),handle:None,meta:String::new(),alpha:Vec::new(),error:None,regions:HashMap::new()});
            }
            let cache_bytes: usize=self.textures.values().filter_map(|t| t.handle.as_ref()).map(|h|h.size()[0]*h.size()[1]*5).sum();
            let texture=self.textures.get_mut(&sprite.path).unwrap();
            if let Some(receiver)=texture.receiver.as_ref().filter(|_|!uploaded && !self.paused.load(std::sync::atomic::Ordering::Relaxed)) {
                match receiver.try_recv() {
                    Ok(Ok((image,meta)))=>{uploaded=true;if cache_bytes + image.size[0]*image.size[1]*5 > PREVIEW_TEXTURE_BUDGET {texture.error=Some("Preview texture cache exceeds 2 GiB".into());texture.receiver=None;missing+=1;continue;} texture.alpha=image.pixels.iter().map(|pixel|pixel.a()).collect(); texture.handle=Some(ui.ctx().load_texture(sprite.path.display().to_string(),image,egui::TextureOptions::LINEAR));texture.meta=meta;texture.receiver=None;},
                    Ok(Err(e))=>{texture.error=Some(e);texture.receiver=None;},
                    Err(mpsc::TryRecvError::Disconnected)=>{texture.error=Some("decode worker stopped".into());texture.receiver=None;},
                    Err(mpsc::TryRecvError::Empty)=>{loading=true;},
                }
            }
            let Some(handle)=&texture.handle else {if let Some(error)=&texture.error {missing+=1;if reported_errors.insert((sprite.path.clone(),sprite.id.clone())) {issues.push(format!("{} · {} · {}",sprite.name,sprite.path.display(),error));}} continue};
            let region_result=if sprite.ui.as_ref().is_some_and(|image|image.raw) {Some(Region{rect:[0.0,0.0,handle.size()[0] as f64,handle.size()[1] as f64],pivot:[0.5,0.5],ppu:100.0,border:[0.0;4]})} else {*texture.regions.entry(sprite.id.clone()).or_insert_with(||region(&texture.meta,&sprite.id,handle.size()))};
            let Some(region)=region_result else {missing+=1;if reported_errors.insert((sprite.path.clone(),sprite.id.clone())) {issues.push(format!("{} · {} · fileID {} · {}",sprite.name,sprite.path.display(),sprite.id,tr("无法定位 Sprite 切片，或切片数据无效")));}continue};
            if let Some(image)=&sprite.ui {
                quads.extend(ui_quads(&sprite,image,&region,handle.id(),handle.size()));continue;
            }
            let [x,y,w,h]=region.rect;let [iw,ih]=handle.size().map(|v|v as f64);
            let corners=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
            let points=corners.map(|[a,b]|sprite.matrix.point((a-region.pivot[0])*w/region.ppu*if sprite.flip[0]{-1.0}else{1.0},(b-region.pivot[1])*h/region.ppu*if sprite.flip[1]{-1.0}else{1.0}));
            let uv=corners.map(|[a,b]|egui::pos2(((x+a*w)/iw) as f32,(1.0-(y+b*h)/ih) as f32));
            quads.push(Quad{texture:handle.id(),points,outline:points,uv,tint:sprite.tint,path:Some(sprite.path.clone()),component:sprite.component.clone(),selectable:sprite.selectable,order:sprite.order});
        }
        let pending=expanding || self.preparation_pending || loading || self.textures.values().any(|texture|texture.receiver.is_some());
        if pending {ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
        let status=tr(if expanding {"正在展开 Prefab…"} else if self.preparation_pending {"正在准备预览…"} else if pending {"正在加载预览资源…"} else {"合成预览已完成"});
        if preview_status_row(ui,status,!pending,&self.paused,None) {self.open=false;self.release();ui.ctx().request_repaint();return None;}
        let mut issue_target=None;
        if missing>0 {
            ui.colored_label(ui.visuals().warn_fg_color,format!("{}: {missing}",tr("部分资源缺失或暂不支持"))).on_hover_text(issues.join("\n"));
            ui.collapsing(tr("预览问题详情"), |ui| {
                egui::ScrollArea::vertical().id_salt("preview_issues").max_height(160.0).show(ui, |ui| {
                    for (row,issue) in issues.iter().enumerate() {
                        // Only IDs explicitly at the start of a diagnostic refer to
                        // scene documents. Asset GUIDs and Sprite fileIDs do not.
                        let target=documents.iter().find(|doc|issue.strip_prefix(&doc.file_id).is_some_and(|tail|tail.starts_with(" · ") || tail.starts_with(": ")));
                        if let Some(doc)=target {
                            ui.push_id(("preview_issue",row),|ui|ui.horizontal_wrapped(|ui| {
                                let label=format!("{} · {}",doc.type_name,doc.file_id);
                                if ui.button(label).clicked() {issue_target=Some(doc.file_id.clone());}
                                let detail=issue[doc.file_id.len()..].trim_start_matches([' ', '·', ':']);
                                ui.add(egui::Label::new(detail).wrap().selectable(true));
                            }));
                        } else {ui.add(egui::Label::new(issue).wrap().selectable(true));}
                    }
                });
            });
        }
        if let Some(id)=issue_target {
            ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("preview_issue_component"),id.clone()));
            ui.ctx().request_repaint();
            return Some(id);
        }
        if builtin_ui_approximation {ui.small(tr("UI 内置 Sprite 暂用颜色矩形近似显示。"));}
        if has_ui && documents.iter().filter(|doc|doc.type_name=="MonoBehaviour" && doc.text.len()<128*1024).any(|doc|field(&doc.text,"m_HorizontalFit").is_some() || field(&doc.text,"m_ChildControlWidth").is_some() || field(&doc.text,"m_ShowMaskGraphic").is_some() || field(&doc.text,"m_Softness").is_some()) {
            ui.small(tr("UI 布局驱动与遮罩暂按静态属性近似显示。"));
        }
        ui.small(tr("XY 正交视图 · 滚轮缩放 · 拖动平移"));
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.0)),egui::Sense::click_and_drag());
        let painter=ui.painter_at(rect);
        if self.background == "gray" || self.background == "custom" {
            let [r,g,b] = if self.background == "gray" {[128,128,128]} else {self.background_rgb.unwrap_or([190,190,190])};
            painter.rect_filled(rect, 0.0, Color32::from_rgb(r,g,b));
        } else {
        let cell = 12.0;
        for y in 0..(rect.height() / cell).ceil() as usize {
            for x in 0..(rect.width() / cell).ceil() as usize {
                let tile = egui::Rect::from_min_size(
                    rect.min + egui::vec2(x as f32 * cell, y as f32 * cell), Vec2::splat(cell),
                ).intersect(rect);
                painter.rect_filled(tile, 0.0, Color32::from_gray(if (x+y)%2==0 {190} else {230}));
            }
        }
        }
        if response.dragged(){self.pan+=response.drag_delta();}
        if response.hovered(){let scroll=ui.input(|i|i.smooth_scroll_delta.y);self.zoom=(self.zoom*(scroll*0.002).exp()).clamp(0.05,50.0);}
        let reference_frame = if has_ui {
            let root = documents.iter().find(|doc| doc.type_name == "RectTransform"
                && doc.parent_transform.as_deref().is_none_or(|id| id == "0" || !transforms.contains_key(id)))
                ;
            let pivot = root.map(|doc| vector(&doc.text,"m_Pivot",&["x","y"],&[0.5,0.5])).unwrap_or(vec![0.5,0.5]);
            Some(ui_rect_points(crate::ui_preview::Layout {
                matrix: Matrix::identity(), size: viewport, pivot: [pivot[0],pivot[1]],
            }))
        } else { None };
        let mut bounds=[0.0_f64;4];
        if let Some(points) = &reference_frame {
            bounds=[points[0][0],points[0][1],points[0][0],points[0][1]];
            for p in points { bounds[0]=bounds[0].min(p[0]); bounds[1]=bounds[1].min(p[1]); bounds[2]=bounds[2].max(p[0]); bounds[3]=bounds[3].max(p[1]); }
        } else {
        for quad in &quads {for p in &quad.points {bounds[0]=bounds[0].min(p[0]);bounds[1]=bounds[1].min(p[1]);bounds[2]=bounds[2].max(p[0]);bounds[3]=bounds[3].max(p[1]);}}
        for text in &texts {for p in ui_rect_points(text.layout) {bounds[0]=bounds[0].min(p[0]);bounds[1]=bounds[1].min(p[1]);bounds[2]=bounds[2].max(p[0]);bounds[3]=bounds[3].max(p[1]);}}
        }
        let scale=((rect.width()-40.0).max(1.0)/(bounds[2]-bounds[0]).max(0.01) as f32).min((rect.height()-40.0).max(1.0)/(bounds[3]-bounds[1]).max(0.01) as f32)*self.zoom;
        let center=[(bounds[0]+bounds[2])*0.5,(bounds[1]+bounds[3])*0.5];
        if let Some((components, started)) = &self.focus_request {
            let points = quads.iter().filter(|quad| components.contains(&quad.component))
                .flat_map(|quad| quad.points.iter().copied())
                .chain(texts.iter().filter(|text| components.contains(&text.component))
                    .flat_map(|text| ui_rect_points(text.layout)));
            let mut focused: Option<[f64;4]> = None;
            for point in points {
                let area = focused.get_or_insert([point[0], point[1], point[0], point[1]]);
                area[0] = area[0].min(point[0]); area[1] = area[1].min(point[1]);
                area[2] = area[2].max(point[0]); area[3] = area[3].max(point[1]);
            }
            if let Some(area) = focused {
                let target = [(area[0]+area[2])*0.5, (area[1]+area[3])*0.5];
                self.pan = egui::vec2((center[0]-target[0]) as f32*scale, (target[1]-center[1]) as f32*scale);
                self.focus_request = None;
            } else if started.elapsed().as_secs() >= 10 {
                self.focus_request = None;
            } else { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        }
        let screen=|p:[f64;2]|rect.center()+self.pan+egui::vec2((p[0]-center[0]) as f32*scale,-(p[1]-center[1]) as f32*scale);
        if self.show_guides {
        let origin=screen(reference_frame.as_ref().map_or([0.0,0.0], |points| points[0]));
        let axis_color=if self.background!="gray" && self.background!="custom" {
            Color32::from_rgba_unmultiplied(100,100,100,170)
        } else {Color32::from_rgba_unmultiplied(128,128,128,100)};
        let axis=egui::Stroke::new(1.0_f32,axis_color);
        painter.line_segment([Pos2::new(rect.left(),origin.y),Pos2::new(rect.right(),origin.y)],axis);
        painter.line_segment([Pos2::new(origin.x,rect.top()),Pos2::new(origin.x,rect.bottom())],axis);
        if let Some(points) = reference_frame {
            let corner = screen(points[2]);
            painter.line_segment([Pos2::new(rect.left(),corner.y),Pos2::new(rect.right(),corner.y)],axis);
            painter.line_segment([Pos2::new(corner.x,rect.top()),Pos2::new(corner.x,rect.bottom())],axis);
        }
        }
        enum DrawItem { Quad(usize), Text(usize) }
        let mut drawing:Vec<_>=quads.iter().enumerate().map(|(index,quad)|(quad.order,DrawItem::Quad(index)))
            .chain(texts.iter().enumerate().map(|(index,text)|(text.order,DrawItem::Text(index)))).collect();
        drawing.sort_by_key(|(order,_)|*order);
        let mut picked=None;
        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pointer)=response.interact_pointer_pos() {
                for (_,item) in drawing.iter().rev() {
                    match item {
                        DrawItem::Text(index)=>{
                            let text=&texts[*index];
                            if text.selectable && text.color.a()>0 && quad_coordinates(ui_rect_points(text.layout).map(screen),pointer).is_some() {picked=Some(text.component.clone());break;}
                        }
                        DrawItem::Quad(index)=>{
                            let quad=&quads[*index];
                            if !quad.selectable || quad.tint.a()==0 {continue}
                            let Some([a,b])=quad_coordinates(quad.points.map(screen),pointer) else {continue};
                            let hit=if let Some(path)=&quad.path {
                                let uv=quad.uv[0]+(quad.uv[1]-quad.uv[0])*a+(quad.uv[3]-quad.uv[0])*b;
                                let texture=&self.textures[path];let size=texture.handle.as_ref().unwrap().size();
                                let x=(uv.x*size[0] as f32).floor().clamp(0.0,(size[0]-1) as f32) as usize;
                                let y=(uv.y*size[1] as f32).floor().clamp(0.0,(size[1]-1) as f32) as usize;
                                texture.alpha.get(y*size[0]+x).is_some_and(|alpha|*alpha>8)
                            } else {true};
                            if hit {picked=Some(quad.component.clone());break;}
                        }
                    }
                }
            }
        }
        self.sdf.begin();
        let highlighted=|id:&str|picked.as_deref().map_or_else(||selected_components.contains(id),|picked|picked==id);
        let mut batch: Option<egui::Mesh> = None;
        for (_,item) in drawing {
            match item {
                DrawItem::Quad(index)=>{
                    let quad=&quads[index];
                    let points=quad.points.map(screen);
                    if !egui::Rect::from_points(&points).intersects(rect) {continue;}
                    if batch.as_ref().is_some_and(|mesh|mesh.texture_id!=quad.texture) {
                        painter.add(egui::Shape::mesh(batch.take().unwrap()));
                    }
                    let mesh=batch.get_or_insert_with(||egui::Mesh::with_texture(quad.texture));
                    let base=mesh.vertices.len() as u32;
                    for i in 0..4 {mesh.vertices.push(egui::epaint::Vertex{pos:points[i],uv:quad.uv[i],color:quad.tint});}
                    mesh.indices.extend([base,base+1,base+2,base,base+2,base+3]);
                }
                DrawItem::Text(index)=>{
                    if let Some(mesh)=batch.take() {painter.add(egui::Shape::mesh(mesh));}
                    let text=&texts[index];let points=ui_rect_points(text.layout).map(screen);
                    let right=points[1]-points[0];let down=points[0]-points[3];
                    let width=right.length().max(1.0);let height=down.length().max(1.0);
                    let font_scale=height/text.layout.size[1].max(0.001) as f32;
                    let layout=|size:f32| {
                        let mut job=egui::text::LayoutJob::simple(text.content.clone(),egui::FontId::new((size*font_scale).clamp(1.0,512.0),text.family.clone()),text.color,if text.wrap {width}else{f32::INFINITY});
                        job.halign=if text.horizontal==0.5 {egui::Align::Center}else if text.horizontal==1.0 {egui::Align::Max}else{egui::Align::Min};
                        job.wrap.max_rows=512;
                        ui.fonts(|fonts|fonts.layout_job(job))
                    };
                    let galley=if let Some([min,max])=text.best_fit {
                        fit_text(min,max,Vec2::new(width,height),layout)
                    } else {layout(text.font_size as f32)};
                    let alignment_bounds=text_alignment_bounds(galley.rect,galley.mesh_bounds,text.visible_center,text.align_by_geometry);
                    let top_left=text_origin(points,alignment_bounds,[text.horizontal as f32,text.vertical as f32]);
                    let clip=egui::Rect::from_points(&points).intersect(rect);
                    for outline in &text.outlines {
                        let opacity=if outline.use_graphic_alpha {text.color.a() as f64/255.0} else {text.canvas_alpha};
                        let color=outline.color.gamma_multiply(opacity as f32);
                        let width=outline.distance[0].abs().max(outline.distance[1].abs()) as f32*font_scale;
                        let offset=-down/text.layout.size[1].max(0.001) as f32*outline.distance[1] as f32;
                        self.sdf.paint(ui.ctx(),&painter.with_clip_rect(clip),&galley,top_left,right.y.atan2(right.x),width,offset,color);
                    }
                    painter.with_clip_rect(clip).add(egui::epaint::TextShape::new(top_left,galley,text.color).with_angle(right.y.atan2(right.x)));
                }
            }
        }
        if let Some(mesh)=batch {painter.add(egui::Shape::mesh(mesh));}
        let mut outlined=HashSet::new();
        let highlighted_quads: Vec<_>=quads.iter().filter(|quad|highlighted(&quad.component)).collect();
        if highlighted_quads.len()>64 {
            let bounds=highlighted_quads.iter().fold(egui::Rect::NOTHING,|bounds,quad|bounds.union(egui::Rect::from_points(&quad.outline.map(screen))));
            painter.rect_stroke(bounds,0.0,egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235)),egui::StrokeKind::Inside);
        } else {
            for quad in highlighted_quads {
                if outlined.insert(&quad.component) {painter.add(egui::Shape::closed_line(quad.outline.map(screen).to_vec(),egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235))));}
            }
        }
        for text in texts.iter().filter(|text|highlighted(&text.component)) {
            painter.add(egui::Shape::closed_line(ui_rect_points(text.layout).map(screen).to_vec(),egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235))));
        }
        paint_zoom_badge(&painter,rect,self.zoom as f64);
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        picked.map(|id|expanded.as_ref().map_or_else(||id.clone(),|data|data.selection(id.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_actions_wait_five_seconds_and_reset_when_complete() {
        let ctx=egui::Context::default();
        let sample=|time:f64,complete:bool,paused:bool|{
            let mut ready=false;
            let _=ctx.run(egui::RawInput{time:Some(time),..Default::default()},|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|{ready=preview_actions_ready(ui,complete,paused);});
            });ready
        };
        assert!(!sample(10.0,false,false));
        assert!(!sample(14.99,false,false));
        assert!(sample(15.0,false,false));
        assert!(!sample(16.0,true,false));
        assert!(!sample(16.1,false,false));
        assert!(sample(16.2,false,true));
    }
    #[test]
    fn preview_status_rows_keep_canvas_position_during_refresh() {
        let measure=|path:&str,complete:bool|{
            let ctx=egui::Context::default();let mut y=0.0;
            let _=ctx.run(egui::RawInput::default(),|ctx|{
                egui::CentralPanel::default().show(ctx,|ui|{
                    ui.set_max_width(120.0);
                    preview_path_row(ui,path);
                    preview_status_row(ui,if complete {"合成预览已完成"}else{"正在合成预览…"},complete,&std::sync::atomic::AtomicBool::new(false),None);
                    y=ui.cursor().min.y;
                });
            });y
        };
        let completed=measure("GPU Shader 预览",true);
        let refreshing=measure("GPU Shader 预览",false);
        let fallback=measure("Ordinary texture preview (GPU Shader inactive)",false);
        assert!((completed-refreshing).abs()<0.01);
        assert!((completed-fallback).abs()<0.01);
    }
    #[test]
    fn outline_detection_accepts_custom_names_and_respects_enabled_and_color() {
        let mut doc=Document {file_id:"1".into(),type_name:"MonoBehaviour".into(),start_line:0,start_offset:0,
            text:"  m_Enabled: 1\n  m_EditorClassIdentifier: Game::CustomTextOutline\n  m_EffectColor: {r: 0.2, g: 0.4, b: 0.6, a: 0.5}\n  m_EffectDistance: {x: 0, y: -1}\n  m_UseGraphicAlpha: 0\n".into(),game_object:Some("2".into()),parent_transform:None};
        let effect=text_outline(&doc,None).unwrap();
        assert_eq!(effect.distance,[0.0,-1.0]);assert!(!effect.use_graphic_alpha);
        assert_eq!(effect.color.a(),128);
        doc.text=doc.text.replace("m_Enabled: 1","m_Enabled: 0").into();assert!(text_outline(&doc,None).is_none());
        doc.text="  m_Script: {fileID: 11500000, guid: abc, type: 3}\n  m_EffectColor: {r: 0, g: 1, b: 0, a: 1}\n".into();
        let catalog=Arc::new(HashMap::from([("abc".into(),PathBuf::from("Assets/PFOutLine.cs"))]));
        assert!(text_outline(&doc,Some(&catalog)).is_some());
        let catalog=Arc::new(HashMap::from([("abc".into(),PathBuf::from("Assets/Shadow.cs"))]));
        assert!(text_outline(&doc,Some(&catalog)).is_none());
        assert!(text_outline(&doc,None).is_none());
    }
    #[test]
    fn middle_center_uses_visible_glyph_height_without_font_padding() {
        let points=[Pos2::new(0.0,60.0),Pos2::new(240.0,60.0),Pos2::new(240.0,0.0),Pos2::new(0.0,0.0)];
        let layout=egui::Rect::from_min_max(Pos2::new(-90.0,0.0),Pos2::new(90.0,50.0));
        let glyphs=egui::Rect::from_min_max(Pos2::new(-85.0,6.0),Pos2::new(88.0,26.0));
        let bounds=text_alignment_bounds(layout,glyphs,true,false);
        let origin=text_origin(points,bounds,[0.5,0.5]);
        assert_eq!(origin.y+glyphs.center().y,30.0);
        assert_eq!(bounds.min.x,layout.min.x);
        assert_eq!(text_alignment_bounds(layout,glyphs,false,false),layout);
        assert_eq!(text_alignment_bounds(layout,glyphs,true,true),glyphs);
    }
    #[test]
    fn best_fit_keeps_complete_long_title_inside_text_box() {
        let ctx=egui::Context::default();
        let _=ctx.run(egui::RawInput::default(),|ctx| {
            let galley=fit_text(28.0,45.0,Vec2::new(450.0,70.0),|size| {
                let mut job=egui::text::LayoutJob::simple("Grotte des Chauves-souris".into(),egui::FontId::proportional(size),Color32::WHITE,450.0);
                job.halign=egui::Align::Center;
                ctx.fonts(|fonts|fonts.layout_job(job))
            });
            assert_eq!(galley.job.text,"Grotte des Chauves-souris");
            assert!(galley.size().x<=450.5 && galley.size().y<=70.5);
            assert!(!galley.elided);
            let drawn:String=galley.rows.iter().flat_map(|row|row.glyphs.iter()).map(|glyph|glyph.chr).filter(|c|!c.is_whitespace()).collect();
            assert_eq!(drawn,"GrottedesChauves-souris");
        });
    }
    #[test]
    fn text_anchor_positions_center_right_and_oversized_text() {
        let points=[Pos2::new(0.0,60.0),Pos2::new(240.0,60.0),Pos2::new(240.0,0.0),Pos2::new(0.0,0.0)];
        let centered=egui::Rect::from_min_max(Pos2::new(-50.0,0.0),Pos2::new(50.0,20.0));
        assert_eq!(text_origin(points,centered,[0.5,0.5]),Pos2::new(120.0,20.0));
        let right=egui::Rect::from_min_max(Pos2::new(-100.0,0.0),Pos2::new(0.0,20.0));
        assert_eq!(text_origin(points,right,[1.0,1.0]),Pos2::new(240.0,40.0));
        let oversized=egui::Rect::from_min_max(Pos2::new(-150.0,0.0),Pos2::new(150.0,80.0));
        assert_eq!(text_origin(points,oversized,[0.5,0.5]),Pos2::new(120.0,-10.0));
    }
    fn transform(id: &str, parent: Option<&str>, position: &str, rotation: &str, scale: &str) -> Document {
        Document { file_id: id.into(), type_name: "Transform".into(), start_line: 1, start_offset: 0, game_object: None, parent_transform: parent.map(str::to_owned), text: format!("Transform:\n  m_LocalPosition: {position}\n  m_LocalRotation: {rotation}\n  m_LocalScale: {scale}\n").into() }
    }
    #[test]
    fn equal_orders_draw_smaller_y_last_and_order_has_priority() {
        let docs=crate::parse_unity_yaml("--- !u!212 &1\nSpriteRenderer:\n  m_SortingLayer: 0\n  m_SortingOrder: 20\n--- !u!212 &2\nSpriteRenderer:\n  m_SortingLayer: 0\n  m_SortingOrder: 21\n");
        let mut high=Matrix::identity();high.0[1][3]=10.0;
        let mut low=Matrix::identity();low.0[1][3]=-5.0;
        assert!(renderer_order(&docs[0],high,1)<renderer_order(&docs[0],low,1));
        assert!(renderer_order(&docs[0],low,1)<renderer_order(&docs[1],high,1));
        high.0[0][3]=-9.0;
        assert!(renderer_order(&docs[0],high,0)>renderer_order(&docs[0],low,0));
        assert_eq!(renderer_order(&docs[0],high,2),renderer_order(&docs[0],low,2));
    }
    #[test]
    fn compact_projection_preserves_world_points() {
        let mut matrix=Matrix::identity();matrix.0[0]=[2.0,-3.0,4.0,8.0];matrix.0[1]=[-1.0,5.0,9.0,-6.0];
        let compact=Projection::from(matrix);
        assert_eq!(compact.point(-7.0,11.0),matrix.point(-7.0,11.0));
        assert_eq!(compact.full().point(-7.0,11.0),matrix.point(-7.0,11.0));
        assert!(std::mem::size_of::<Projection>()<std::mem::size_of::<Matrix>());
    }
    #[test]
    fn zero_root_scale_is_visible_but_zero_child_scale_is_preserved() {
        let root=transform("1",None,"{x: 0, y: 0, z: 0}","{x: 0, y: 0, z: 0, w: 1}","{x: 0, y: 0, z: 0}");
        assert_eq!(local(&root,true).point(2.0,3.0),[2.0,3.0]);
        assert_eq!(local(&root,false).point(2.0,3.0),[0.0,0.0]);
        assert!(root.text.contains("m_LocalScale: {x: 0, y: 0, z: 0}"));
    }
    #[test]
    fn child_translation_inherits_root_rotation_and_scale_and_root_position_is_zero() {
        let root=transform("1",None,"{x: 100, y: 200, z: 0}","{x: 0, y: 0, z: 0.7071067811865476, w: 0.7071067811865476}","{x: 2, y: 3, z: 1}");
        let child=transform("2",Some("1"),"{x: 1, y: 2, z: 0}","{x: 0, y: 0, z: 0, w: 1}","{x: 1, y: 1, z: 1}");
        let transforms=HashMap::from([("1",&root),("2",&child)]);
        let point=world("2",&transforms,&HashMap::new(),&mut HashSet::new()).unwrap().point(0.0,0.0);
        assert!((point[0]+6.0).abs()<1e-8 && (point[1]-2.0).abs()<1e-8);
    }
    #[test]
    fn bone_world_accumulates_all_parent_x_rotations() {
        let make=|id,parent,degrees:f64| {let angle=degrees.to_radians()*0.5;transform(id,parent,"{x: 0, y: 0, z: 0}",&format!("{{x: {}, y: 0, z: 0, w: {}}}",angle.sin(),angle.cos()),"{x: 1, y: 1, z: 1}")};
        let root=make("1",None,-40.0);let body=make("2",Some("1"),-90.0);let bone=make("3",Some("2"),90.0);
        let transforms=HashMap::from([("1",&root),("2",&body),("3",&bone)]);
        let matrix=world("3",&transforms,&HashMap::new(),&mut HashSet::new()).unwrap();
        assert!((matrix.0[1][1]-(-40.0f64).to_radians().cos()).abs()<1e-8);
        assert!((matrix.0[2][1]-(-40.0f64).to_radians().sin()).abs()<1e-8);
    }
    #[test]
    fn picking_handles_rotated_mirrored_and_degenerate_rectangles() {
        let points=[egui::pos2(10.0,10.0),egui::pos2(0.0,20.0),egui::pos2(10.0,30.0),egui::pos2(20.0,20.0)];
        assert_eq!(quad_coordinates(points,egui::pos2(10.0,20.0)),Some([0.5,0.5]));
        assert!(quad_coordinates(points,egui::pos2(0.0,0.0)).is_none());
        assert!(quad_coordinates([Pos2::ZERO;4],Pos2::ZERO).is_none());
    }
    #[test]
    fn cyclic_transform_parent_is_rejected() {
        let d=transform("1",Some("1"),"{x: 0, y: 0, z: 0}","{x: 0, y: 0, z: 0, w: 1}","{x: 1, y: 1, z: 1}");
        assert!(world("1",&HashMap::from([("1",&d)]),&HashMap::new(),&mut HashSet::new()).is_none());
    }
    #[test]
    fn polygon_sprite_uses_full_texture_and_custom_pivot_despite_outline() {
        let meta="TextureImporter:\n  spriteMode: 3\n  spritePixelsToUnits: 100\n  spriteAlignment: 9\n  spritePivot: {x: 0.25, y: 0.1}\n  spriteSheet:\n    serializedVersion: 2\n    sprites: []\n    outline:\n    - - {x: -20, y: -10}\n      - {x: 50, y: 100}\n    physicsShape:\n    - - {x: 10000, y: 10000}\n";
        let r=region(meta,"21300000",[132,224]).unwrap();
        assert_eq!(r.rect,[0.0,0.0,132.0,224.0]);
        assert_eq!(r.pivot,[0.25,0.1]);
        assert_eq!(r.ppu,100.0);
        assert!(region(meta,"21300002",[132,224]).is_none());
    }
    #[test]
    fn tight_multiple_sprite_ignores_outline_coordinates() {
        let meta="spriteMode: 2\nspritePixelsToUnits: 100\nspriteMeshType: 1\n    - serializedVersion: 2\n      name: tree\n      rect:\n        serializedVersion: 2\n        x: 10\n        y: 20\n        width: 30\n        height: 40\n      alignment: 0\n      pivot: {x: 0.5, y: 0.5}\n      outline:\n      - - {x: -500, y: 900}\n        - {x: 900, y: 500}\n      internalID: 21300002\n";
        let r=region(meta,"21300002",[100,100]).unwrap();
        assert_eq!(r.rect,[10.0,20.0,30.0,40.0]);
    }
    #[test]
    fn sliced_image_preserves_corners_and_skips_center() {
        let doc=transform("1",None,"{x: 0, y: 0, z: 0}","{x: 0, y: 0, z: 0, w: 1}","{x: 1, y: 1, z: 1}");
        let sprite=Sprite{baked:None,ui:None,component:"20".into(),selectable:true,name:"panel".into(),path:"panel.png".into(),id:"21300000".into(),spatial:local(&doc,true),matrix:local(&doc,true).into(),tint:Color32::WHITE,flip:[false;2],order:(0,0,0)};
        let image=UiImage{size:[200.0,100.0],pivot:[0.5,0.5],mode:1,fill_center:false,preserve_aspect:false,ppu_multiplier:1.0,reference_ppu:100.0,raw:false,uv_rect:[0.0,0.0,1.0,1.0],fill:1.0,fill_method:0,fill_origin:0};
        let region=Region{rect:[0.0,0.0,100.0,100.0],pivot:[0.5,0.5],ppu:100.0,border:[10.0;4]};
        let quads=ui_quads(&sprite,&image,&region,egui::TextureId::default(),[100,100]);
        assert_eq!(quads.len(),8);
        assert_eq!(quads[0].points,[[-100.0,-50.0],[-90.0,-50.0],[-90.0,-40.0],[-100.0,-40.0]]);
        assert!(quads.iter().all(|quad|quad.component=="20"));
        assert_eq!(adjusted_slices(10.0,8.0,12.0),[0.0,4.0,4.0,10.0]);
    }
    #[test]
    fn slice_index_matches_current_and_legacy_ids() {
        let meta="spriteMode: 2\nspritePixelsToUnits: 100\nfileIDToRecycleName:\n  21300004: old_name\n    - serializedVersion: 2\n      name: old_name\n      rect:\n        x: 10\n        y: 20\n        width: 30\n        height: 40\n      alignment: 9\n      pivot: {x: 0.2, y: 0.8}\n      internalID: 21300002\n";
        let index=region_index(meta,[100,100]);
        for id in ["21300002","21300004"] {assert_eq!(index[id].unwrap().rect,region(meta,id,[100,100]).unwrap().rect);assert_eq!(index[id].unwrap().pivot,[0.2,0.8]);}
        assert!(region_index(meta,[20,20])["21300002"].is_none());
    }
    #[test]
    fn sprite_slice_matches_file_id_and_rejects_out_of_bounds() {
        let meta="spriteMode: 2\nspritePixelsToUnits: 100\n    - serializedVersion: 2\n      rect:\n        x: 10\n        y: 20\n        width: 30\n        height: 40\n      alignment: 9\n      pivot: {x: 0.2, y: 0.8}\n      internalID: 21300002\n";
        let r=region(meta,"21300002",[100,100]).unwrap();
        assert_eq!(r.rect,[10.0,20.0,30.0,40.0]);assert_eq!(r.pivot,[0.2,0.8]);
        assert!(region(meta,"21300000",[100,100]).is_none());
        assert!(region(meta,"21300002",[20,20]).is_none());
    }
}

/// Preserve complete world transforms for textured planes in the orbit preview.
pub(crate) fn scene_planes(documents:&[Document],flags:&HashMap<String,crate::hierarchy_controls::Flags>,catalog:&Arc<HashMap<String,PathBuf>>)->Result<(Vec<crate::model_preview::ScenePlane>,Vec<String>),String> {
    let docs:Vec<_>=documents.iter().filter(|d|!matches!(d.type_name.as_str(),"MeshFilter"|"SkinnedMeshRenderer")).cloned().collect();
    let prepared=prepare(&docs,flags,Some(catalog),&std::sync::atomic::AtomicBool::new(false),1,None);
    let mut warnings=prepared.issues;let mut planes=Vec::new();let mut bytes=0usize;
    if documents.iter().any(|d|crate::ui_preview::graphic(d).is_some()) {warnings.push("3D模式暂不显示UI Canvas图形，请切换2D查看UI布局。".into());}
    for sprite in prepared.sprites {
        let loaded=(|| -> Result<_,String> {
            if let Some(baked)=&sprite.baked {return Ok((baked.image.clone(),baked.region));}
            if std::fs::metadata(&sprite.path).map_err(|e|e.to_string())?.len()>64*1024*1024 {return Err("贴图超过64 MiB".into());}
            let mut reader=image::ImageReader::open(&sprite.path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
            let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(128*1024*1024);reader.limits(limits);
            let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();let size=[image.width() as usize,image.height() as usize];
            let meta=std::fs::read_to_string(format!("{}.meta",sprite.path.display())).unwrap_or_default();
            let region=region(&meta,&sprite.id,size).ok_or("Sprite区域未找到")?;
            Ok((egui::ColorImage::from_rgba_unmultiplied(size,image.as_raw()),region))
        })();
        let (image,region)=match loaded {Ok(value)=>value,Err(error)=>{warnings.push(format!("{}: {error}",sprite.path.display()));continue;}};
        bytes+=image.pixels.len()*4;if bytes>128*1024*1024 {return Err("3D精灵贴图超过128 MiB".into());}
        let corners=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
        let points=corners.map(|corner|{
            let x=(corner[0]-region.pivot[0])*region.rect[2]/region.ppu*if sprite.flip[0]{-1.0}else{1.0};
            let y=(corner[1]-region.pivot[1])*region.rect[3]/region.ppu*if sprite.flip[1]{-1.0}else{1.0};
            std::array::from_fn(|axis|(sprite.spatial.0[axis][0]*x+sprite.spatial.0[axis][1]*y+sprite.spatial.0[axis][3]) as f32)
        });
        let uv=corners.map(|c|[((region.rect[0]+c[0]*region.rect[2])/image.size[0] as f64) as f32,((region.rect[1]+c[1]*region.rect[3])/image.size[1] as f64) as f32]);
        planes.push(crate::model_preview::ScenePlane{image,points,uv,tint:sprite.tint});
    }
    Ok((planes,warnings))
}
