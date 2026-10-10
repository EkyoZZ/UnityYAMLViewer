use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::{Arc, mpsc}, time::SystemTime};
use eframe::egui::{self, Color32, Pos2, Vec2};
use crate::{Document, localization::tr};

#[derive(Default)]
pub struct Preview {
    pub open: bool,
    textures: HashMap<PathBuf, Texture>,
    identity: String,
    zoom: f32,
    pan: Vec2,
    next_asset_check: Option<std::time::Instant>,
    ui_viewport: Option<[f64;2]>,
    focus_request: Option<(HashSet<String>, std::time::Instant)>,
}
struct Texture {
    stamp: (Option<SystemTime>, Option<SystemTime>),
    receiver: Option<mpsc::Receiver<Result<(egui::ColorImage, String), String>>>,
    handle: Option<egui::TextureHandle>,
    meta: String,
    alpha: Vec<u8>,
    error: Option<String>,
}
#[derive(Clone, Copy)]
pub(crate) struct Matrix(pub(crate) [[f64; 4]; 4]);
impl Matrix {
    fn identity() -> Self { Self(std::array::from_fn(|row| std::array::from_fn(|column| if row == column { 1.0 } else { 0.0 }))) }
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
fn world(id: &str, transforms: &HashMap<&str,&Document>, objects: &HashMap<&str,&Document>, seen: &mut HashSet<String>) -> Option<Matrix> {
    if !seen.insert(id.into()) {return None}
    let doc=transforms.get(id)?;
    if doc.game_object.as_deref().and_then(|id| objects.get(id)).is_some_and(|go| number(&go.text,"m_IsActive",1.0)==0.0) {return None}
    let parent=doc.parent_transform.as_deref().filter(|id| *id!="0");
    if let Some(parent)=parent {Some(world(parent,transforms,objects,seen)?.mul(local(doc,false)))} else {Some(local(doc,true))}
}
struct Sprite { ui: Option<UiImage>, component: String, selectable: bool, name: String, path: PathBuf, id: String, matrix: Matrix, tint: Color32, flip: [bool;2], order: (i64,i64), }
struct Region { rect: [f64;4], pivot: [f64;2], ppu: f64, border: [f64;4] }
struct Quad {
    texture: egui::TextureId, points: [[f64;2];4], outline: [[f64;2];4], uv: [Pos2;4], tint: Color32,
    path: Option<PathBuf>, component: String, selectable: bool, order: (i64,i64),
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
fn region(meta: &str, id: &str, size: [usize;2]) -> Option<Region> {
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
    let blocks: Vec<_>=meta.split("    - serializedVersion:").skip(1).collect();
    for block in blocks {
        let internal=meta_value(block,"internalID");
        let legacy_name = meta_value(block,"name").filter(|name| meta.lines().any(|line| line.trim().split_once(':').is_some_and(|(key,value)| key.trim()==id && value.trim()==*name)));
        if internal!=Some(id) && legacy_name.is_none() {continue}
        let rect=block.split_once("      rect:")?.1;
        let scalar=|key: &str| meta_value(rect,key)?.parse::<f64>().ok().filter(|v|v.is_finite());
        let rect=[scalar("x")?,scalar("y")?,scalar("width")?,scalar("height")?];
        if rect[0]<0.0 || rect[1]<0.0 || rect[2]<=0.0 || rect[3]<=0.0 || rect[0]+rect[2]>size[0] as f64 || rect[1]+rect[3]>size[1] as f64 {return None}
        let pivot=pivot_for(block,"alignment","pivot");
        return Some(Region{rect,pivot:[pivot[0],pivot[1]],ppu,border:meta_value(block,"border").and_then(|v|crate::inline_vector(v,&["x","y","z","w"])).map(|v|[v[0],v[1],v[2],v[3]]).unwrap_or([0.0;4])})
    }
    None
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
    selectable: bool, order: (i64,i64),
}
fn tint(text: &str, key: &str) -> Color32 {
    let c=vector(text,key,&["r","g","b","a"],&[1.0;4]);
    let b=|v:f64|(v.clamp(0.0,1.0)*255.0).round() as u8;
    Color32::from_rgba_unmultiplied(b(c[0]),b(c[1]),b(c[2]),b(c[3]))
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
        result.push(Quad{texture,points,outline:ui_rect_points(crate::ui_preview::Layout{matrix:sprite.matrix,size:image.size,pivot:image.pivot}),uv,tint:sprite.tint,path:Some(sprite.path.clone()),component:sprite.component.clone(),selectable:sprite.selectable,order:sprite.order});
    }}
    result
}
fn ui_rect_points(layout: crate::ui_preview::Layout) -> [[f64;2];4] {
    [[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]].map(|[x,y]|layout.matrix.point((x-layout.pivot[0])*layout.size[0],(y-layout.pivot[1])*layout.size[1]))
}
impl Preview {
    pub fn focus(&mut self, components: HashSet<String>) {
        self.open = true;
        self.focus_request = Some((components, std::time::Instant::now()));
    }
    pub fn available(documents: &[Document], path: Option<&std::path::Path>) -> bool {
        path.is_some_and(|p|p.extension().is_some_and(|e|e.eq_ignore_ascii_case("prefab"))) && documents.iter().any(|d|
            crate::ui_preview::graphic(d).is_some() || (d.type_name=="SpriteRenderer" && field(&d.text,"m_Sprite").and_then(|v|reference(v,"fileID")).is_some_and(|id|id!="0")))
    }
    pub fn show(&mut self, ui: &mut egui::Ui, documents: &[Document], identity: &str, flags: &HashMap<String, crate::hierarchy_controls::Flags>, selected: Option<&str>) -> Option<String> {
        if self.identity!=identity {self.identity=identity.into();self.textures.clear();self.zoom=1.0;self.pan=Vec2::ZERO;self.next_asset_check=None;self.ui_viewport=None;}
        ui.horizontal(|ui| {ui.label(tr("静态预览")); if ui.button(tr("适应窗口")).clicked(){self.zoom=1.0;self.pan=Vec2::ZERO;} });
        let catalog=ui.ctx().data_mut(|data|data.get_temp::<Arc<HashMap<String,PathBuf>>>(egui::Id::new("asset_catalog")));
        let objects: HashMap<_,_>=documents.iter().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
        let transforms: HashMap<_,_>=documents.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
        let by_object: HashMap<_,_>=transforms.values().filter_map(|d|Some((d.game_object.as_deref()?,d.file_id.as_str()))).collect();
        let mut missing=0;
        let mut issues=Vec::new();
        let mut builtin_ui_approximation=false;
        let mut sprites=Vec::new();
        let mut quads=Vec::new();
        let mut texts=Vec::new();
        let has_ui=documents.iter().any(|doc|crate::ui_preview::graphic(doc).is_some());
        let reference_viewport=documents.iter().find_map(|doc|field(&doc.text,"m_ReferenceResolution").and_then(|value|crate::inline_vector(value,&["x","y"]))).unwrap_or(vec![1280.0,720.0]);
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

        for doc in documents.iter().filter(|d|d.type_name=="SpriteRenderer") {
            if doc.game_object.as_deref().and_then(|id| flags.get(id)).is_some_and(|flags| !flags.visible) { continue; }
            if number(&doc.text,"m_Enabled",1.0)==0.0 {continue}
            let Some(reference_text)=field(&doc.text,"m_Sprite") else {continue};
            let Some(id)=reference(reference_text,"fileID").filter(|id|id!="0") else {continue};
            let matrix=doc.game_object.as_deref().and_then(|go|by_object.get(go)).and_then(|id|world(id,&transforms,&objects,&mut HashSet::new()));
            let Some(matrix)=matrix else {continue};
            let path=reference(reference_text,"guid").and_then(|guid|catalog.as_ref()?.get(&guid.to_lowercase()).cloned());
            let Some(path)=path else {missing+=1;issues.push(format!("{} · {}: {}\n{}", crate::name_from_text(doc.game_object.as_deref().and_then(|id|objects.get(id)).map(|d|d.text.as_str()).unwrap_or("")).unwrap_or_else(|| doc.file_id.clone()), tr("资源 GUID 未找到"), reference_text, tr("GUID 索引尚未完成，或资源不存在、属于内置资源。")));continue};
            if number(&doc.text,"m_DrawMode",0.0)!=0.0 {missing+=1;issues.push(format!("{} · fileID {} · {}: {}",path.display(),doc.file_id,tr("暂不支持此 Sprite 绘制模式"),field(&doc.text,"m_DrawMode").unwrap_or("")));continue}
            let c=vector(&doc.text,"m_Color",&["r","g","b","a"],&[1.0;4]);
            let byte=|v:f64|(v.clamp(0.0,1.0)*255.0).round() as u8;
            sprites.push(Sprite{ui:None,component: doc.file_id.clone(), selectable: doc.game_object.as_deref().and_then(|id|flags.get(id)).is_none_or(|flags|flags.selectable),name: crate::name_from_text(doc.game_object.as_deref().and_then(|id|objects.get(id)).map(|d|d.text.as_str()).unwrap_or("")).unwrap_or_else(|| doc.file_id.clone()),path,id,matrix,tint:Color32::from_rgba_unmultiplied(byte(c[0]),byte(c[1]),byte(c[2]),byte(c[3])),flip:[number(&doc.text,"m_FlipX",0.0)!=0.0,number(&doc.text,"m_FlipY",0.0)!=0.0],order:(number(&doc.text,"m_SortingLayer",0.0) as i64,number(&doc.text,"m_SortingOrder",0.0) as i64)});
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
            let order=(1_000_000_000,*ui_order.get(object).unwrap_or(&0));
            let color=tint(&doc.text,if kind==Graphic::Tmp {"m_fontColor"} else {"m_Color"});
            let alpha=crate::ui_preview::opacity(transform,documents,&transforms);
            let rgba=color.to_srgba_unmultiplied();
            let color=Color32::from_rgba_unmultiplied(rgba[0],rgba[1],rgba[2],(rgba[3] as f64*alpha).round() as u8);
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
                    font_size:crate::ui_preview::nested_number(&doc.text,if kind==Graphic::Tmp {"m_fontSize"}else{"m_FontSize"},14.0).clamp(1.0,512.0),
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
            sprites.push(Sprite{ui:Some(image),component:doc.file_id.clone(),selectable,name:crate::name_from_text(&objects.get(object).map(|doc|doc.text.as_str()).unwrap_or("")).unwrap_or_else(||doc.file_id.clone()),path,id,matrix:layout.matrix,tint:color,flip:[false;2],order});
        }
        sprites.sort_by_key(|s|s.order);
        let used: HashSet<_>=sprites.iter().map(|s|s.path.clone()).collect();
        self.textures.retain(|path,_|used.contains(path));
        let mut loading=false;
        let now = std::time::Instant::now();
        let check_assets = self.next_asset_check.is_none_or(|next| now >= next);
        if check_assets { self.next_asset_check = Some(now + std::time::Duration::from_secs(1)); }
        for sprite in sprites {
            let meta_path=PathBuf::from(format!("{}.meta",sprite.path.display()));
            let stamp = if check_assets || !self.textures.contains_key(&sprite.path) {
                (std::fs::metadata(&sprite.path).and_then(|m|m.modified()).ok(),
                 std::fs::metadata(&meta_path).and_then(|m|m.modified()).ok())
            } else { self.textures[&sprite.path].stamp };
            if self.textures.get(&sprite.path).is_none_or(|t|t.stamp!=stamp) {
                if self.textures.values().filter(|t|t.receiver.is_some()).count() >= 2 { loading=true; continue; }
                let (sender,receiver)=mpsc::channel(); let path=sprite.path.clone();let ctx=ui.ctx().clone();
                std::thread::spawn(move||{
                    let result=(||{
                        let mut reader=image::ImageReader::open(&path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
                        let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
                        let image=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
                        let meta=std::fs::read_to_string(meta_path).unwrap_or_default();
                        Ok((egui::ColorImage::from_rgba_unmultiplied([image.width() as usize,image.height() as usize],image.as_raw()),meta))
                    })();let _=sender.send(result);ctx.request_repaint();
                });
                self.textures.insert(sprite.path.clone(),Texture{stamp,receiver:Some(receiver),handle:None,meta:String::new(),alpha:Vec::new(),error:None});
            }
            let cache_bytes: usize=self.textures.values().filter_map(|t| t.handle.as_ref()).map(|h|h.size()[0]*h.size()[1]*5).sum();
            let texture=self.textures.get_mut(&sprite.path).unwrap();
            if let Some(receiver)=&texture.receiver {
                match receiver.try_recv() {
                    Ok(Ok((image,meta)))=>{if cache_bytes + image.size[0]*image.size[1]*5 > 256*1024*1024 {texture.error=Some("Preview texture cache exceeds 256 MiB".into());texture.receiver=None;missing+=1;continue;} texture.alpha=image.pixels.iter().map(|pixel|pixel.a()).collect(); texture.handle=Some(ui.ctx().load_texture(sprite.path.display().to_string(),image,egui::TextureOptions::LINEAR));texture.meta=meta;texture.receiver=None;},
                    Ok(Err(e))=>{texture.error=Some(e);texture.receiver=None;},
                    Err(mpsc::TryRecvError::Disconnected)=>{texture.error=Some("decode worker stopped".into());texture.receiver=None;},
                    Err(mpsc::TryRecvError::Empty)=>{loading=true;},
                }
            }
            let Some(handle)=&texture.handle else {if let Some(error)=&texture.error {missing+=1;issues.push(format!("{} · {} · {}",sprite.name,sprite.path.display(),error));} continue};
            let region_result=if sprite.ui.as_ref().is_some_and(|image|image.raw) {Some(Region{rect:[0.0,0.0,handle.size()[0] as f64,handle.size()[1] as f64],pivot:[0.5,0.5],ppu:100.0,border:[0.0;4]})} else {region(&texture.meta,&sprite.id,handle.size())};
            let Some(region)=region_result else {missing+=1;issues.push(format!("{} · {} · fileID {} · {}",sprite.name,sprite.path.display(),sprite.id,tr("无法定位 Sprite 切片，或切片数据无效")));continue};
            if let Some(image)=&sprite.ui {
                quads.extend(ui_quads(&sprite,image,&region,handle.id(),handle.size()));continue;
            }
            let [x,y,w,h]=region.rect;let [iw,ih]=handle.size().map(|v|v as f64);
            let corners=[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
            let points=corners.map(|[a,b]|sprite.matrix.point((a-region.pivot[0])*w/region.ppu*if sprite.flip[0]{-1.0}else{1.0},(b-region.pivot[1])*h/region.ppu*if sprite.flip[1]{-1.0}else{1.0}));
            let uv=corners.map(|[a,b]|egui::pos2(((x+a*w)/iw) as f32,(1.0-(y+b*h)/ih) as f32));
            quads.push(Quad{texture:handle.id(),points,outline:points,uv,tint:sprite.tint,path:Some(sprite.path),component:sprite.component,selectable:sprite.selectable,order:sprite.order});
        }
        if loading {ui.label(tr("正在加载预览资源…"));}
        if missing>0 {
            ui.colored_label(ui.visuals().warn_fg_color,format!("{}: {missing}",tr("部分资源缺失或暂不支持"))).on_hover_text(issues.join("\n"));
            ui.collapsing(tr("预览问题详情"), |ui| {
                egui::ScrollArea::vertical().id_salt("preview_issues").max_height(160.0).show(ui, |ui| {
                    for issue in &issues { ui.add(egui::Label::new(issue).wrap().selectable(true)); }
                });
            });
        }
        if builtin_ui_approximation {ui.small(tr("UI 内置 Sprite 暂用颜色矩形近似显示。"));}
        if has_ui && documents.iter().any(|doc|field(&doc.text,"m_HorizontalFit").is_some() || field(&doc.text,"m_ChildControlWidth").is_some() || field(&doc.text,"m_ShowMaskGraphic").is_some() || field(&doc.text,"m_Softness").is_some()) {
            ui.small(tr("UI 布局驱动与遮罩暂按静态属性近似显示。"));
        }
        if documents.iter().any(|d|d.type_name=="PrefabInstance") {ui.colored_label(ui.visuals().warn_fg_color,tr("嵌套 Prefab 暂未展开，预览可能不完整。"));}
        ui.small(tr("XY 正交视图 · 滚轮缩放 · 拖动平移"));
        let (rect,response)=ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.0)),egui::Sense::click_and_drag());
        let painter=ui.painter_at(rect);
        painter.rect_filled(rect,0.0,ui.visuals().extreme_bg_color);
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
        let origin=screen(reference_frame.as_ref().map_or([0.0,0.0], |points| points[0]));
        let axis=egui::Stroke::new(1.0_f32,Color32::from_rgba_unmultiplied(128,128,128,100));
        painter.line_segment([Pos2::new(rect.left(),origin.y),Pos2::new(rect.right(),origin.y)],axis);
        painter.line_segment([Pos2::new(origin.x,rect.top()),Pos2::new(origin.x,rect.bottom())],axis);
        if let Some(points) = reference_frame {
            let corner = screen(points[2]);
            painter.line_segment([Pos2::new(rect.left(),corner.y),Pos2::new(rect.right(),corner.y)],axis);
            painter.line_segment([Pos2::new(corner.x,rect.top()),Pos2::new(corner.x,rect.bottom())],axis);
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
        let highlighted=picked.as_deref().or(selected);
        for (_,item) in drawing {
            match item {
                DrawItem::Quad(index)=>{
                    let quad=&quads[index];let mut mesh=egui::Mesh::with_texture(quad.texture);
                    for i in 0..4 {mesh.vertices.push(egui::epaint::Vertex{pos:screen(quad.points[i]),uv:quad.uv[i],color:quad.tint});}
                    mesh.indices.extend([0,1,2,0,2,3]);painter.add(egui::Shape::mesh(mesh));
                }
                DrawItem::Text(index)=>{
                    let text=&texts[index];let points=ui_rect_points(text.layout).map(screen);
                    let right=points[1]-points[0];let down=points[0]-points[3];
                    let width=right.length().max(1.0);let height=down.length().max(1.0);
                    let font_scale=height/text.layout.size[1].max(0.001) as f32;
                    let mut job=egui::text::LayoutJob::simple(text.content.clone(),egui::FontId::proportional((text.font_size as f32*font_scale).clamp(1.0,512.0)),text.color,if text.wrap {width}else{f32::INFINITY});
                    job.halign=if text.horizontal==0.5 {egui::Align::Center}else if text.horizontal==1.0 {egui::Align::Max}else{egui::Align::Min};
                    job.wrap.max_rows=512;
                    let galley=ui.fonts(|fonts|fonts.layout_job(job));
                    let top_left=points[3]+right*(text.horizontal as f32)+down.normalized()*((height-galley.size().y).max(0.0)*text.vertical as f32);
                    let clip=egui::Rect::from_points(&points).intersect(rect);
                    painter.with_clip_rect(clip).add(egui::epaint::TextShape::new(top_left,galley,text.color).with_angle(right.y.atan2(right.x)));
                }
            }
        }
        let mut outlined=HashSet::new();
        for quad in quads.iter().filter(|quad|Some(quad.component.as_str())==highlighted) {
            if outlined.insert(&quad.component) {painter.add(egui::Shape::closed_line(quad.outline.map(screen).to_vec(),egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235))));}
        }
        for text in texts.iter().filter(|text|Some(text.component.as_str())==highlighted) {
            painter.add(egui::Shape::closed_line(ui_rect_points(text.layout).map(screen).to_vec(),egui::Stroke::new(1.0_f32,Color32::from_rgb(70,155,235))));
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn transform(id: &str, parent: Option<&str>, position: &str, rotation: &str, scale: &str) -> Document {
        Document { file_id: id.into(), type_name: "Transform".into(), start_line: 1, game_object: None, parent_transform: parent.map(str::to_owned), text: format!("Transform:\n  m_LocalPosition: {position}\n  m_LocalRotation: {rotation}\n  m_LocalScale: {scale}\n") }
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
        let sprite=Sprite{ui:None,component:"20".into(),selectable:true,name:"panel".into(),path:"panel.png".into(),id:"21300000".into(),matrix:local(&doc,true),tint:Color32::WHITE,flip:[false;2],order:(0,0)};
        let image=UiImage{size:[200.0,100.0],pivot:[0.5,0.5],mode:1,fill_center:false,preserve_aspect:false,ppu_multiplier:1.0,reference_ppu:100.0,raw:false,uv_rect:[0.0,0.0,1.0,1.0],fill:1.0,fill_method:0,fill_origin:0};
        let region=Region{rect:[0.0,0.0,100.0,100.0],pivot:[0.5,0.5],ppu:100.0,border:[10.0;4]};
        let quads=ui_quads(&sprite,&image,&region,egui::TextureId::default(),[100,100]);
        assert_eq!(quads.len(),8);
        assert_eq!(quads[0].points,[[-100.0,-50.0],[-90.0,-50.0],[-90.0,-40.0],[-100.0,-40.0]]);
        assert!(quads.iter().all(|quad|quad.component=="20"));
        assert_eq!(adjusted_slices(10.0,8.0,12.0),[0.0,4.0,4.0,10.0]);
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
