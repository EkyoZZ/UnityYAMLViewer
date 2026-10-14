//! Read-only expansion of linked prefab instances for preview documents.
use super::*;
use std::sync::atomic::AtomicBool;
use std::time::{Duration,Instant};
type Source=(String,String);
const SOURCE_BUDGET:usize=512*1024*1024;
const EXPANSION_BUDGET:usize=512*1024*1024;
const EXPANSION_DOCUMENT_LIMIT:usize=500_000;
#[derive(Default)]
pub(crate) struct Expanded {pub documents:Vec<Document>,pub picks:HashMap<String,String>,pub prefab_roots:HashMap<String,String>,pub issues:Vec<String>,flags_cache:std::sync::Mutex<Option<(u64,Arc<HashMap<String,crate::hierarchy_controls::Flags>>)>>,deps:Vec<(PathBuf,Option<SystemTime>,u64)>}
impl Expanded {
    pub(super) fn selection(&self,id:String) -> String {self.picks.get(&id).unwrap_or(&id).clone()}
    pub(crate) fn hierarchy_object(&self,picked:&str)->Option<String> {
        if let Some(doc)=self.documents.iter().find(|doc|doc.file_id==picked) {
            return doc.game_object.clone().or_else(||(doc.type_name=="GameObject").then(||doc.file_id.clone()));
        }
        let candidates:HashSet<_>=self.documents.iter().filter(|doc|doc.type_name=="GameObject" && self.picks.get(&doc.file_id).is_some_and(|proxy|proxy==picked)).map(|doc|doc.file_id.as_str()).collect();
        let transforms:HashMap<_,_>=self.documents.iter().filter(|doc|matches!(doc.type_name.as_str(),"Transform"|"RectTransform")).map(|doc|(doc.file_id.as_str(),doc)).collect();
        let by_object:HashMap<_,_>=transforms.values().filter_map(|doc|Some((doc.game_object.as_deref()?,*doc))).collect();
        self.documents.iter().filter(|doc|doc.type_name=="GameObject" && candidates.contains(doc.file_id.as_str())).find(|doc|{
            let parent=by_object.get(doc.file_id.as_str()).and_then(|t|t.parent_transform.as_deref()).and_then(|id|transforms.get(id)).and_then(|t|t.game_object.as_deref());
            parent.is_none_or(|id|!candidates.contains(id))
        }).map(|doc|doc.file_id.clone())
    }
    pub(crate) fn hierarchy_ancestors(&self,object:&str)->HashSet<String> {
        let transforms:HashMap<_,_>=self.documents.iter().filter(|doc|matches!(doc.type_name.as_str(),"Transform"|"RectTransform")).map(|doc|(doc.file_id.as_str(),doc)).collect();
        let by_object:HashMap<_,_>=transforms.values().filter_map(|doc|Some((doc.game_object.as_deref()?,*doc))).collect();
        let mut result=HashSet::new();let mut current=Some(object);
        while let Some(id)=current {if !result.insert(id.to_owned()){break;} current=by_object.get(id).and_then(|t|t.parent_transform.as_deref()).and_then(|id|transforms.get(id)).and_then(|t|t.game_object.as_deref());}
        result
    }
    pub(super) fn flags(&self,base:&HashMap<String,crate::hierarchy_controls::Flags>) -> Arc<HashMap<String,crate::hierarchy_controls::Flags>> {
        use std::hash::{Hash,Hasher};
        let mut key=base.len() as u64;
        for (id,flags) in base {let mut hash=std::collections::hash_map::DefaultHasher::new();id.hash(&mut hash);flags.visible.hash(&mut hash);flags.selectable.hash(&mut hash);key^=hash.finish();}
        if let Some((cached,result))=&*self.flags_cache.lock().unwrap() {if *cached==key {return result.clone();}}
        let transforms:HashMap<_,_>=self.documents.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform")).map(|d|(d.file_id.as_str(),d)).collect();
        let by_object:HashMap<_,_>=transforms.values().filter_map(|d|Some((d.game_object.as_deref()?,d.file_id.as_str()))).collect();
        let mut result=base.clone();
        for doc in self.documents.iter().filter(|d|d.type_name=="GameObject") {
            let mut flags=base.get(&doc.file_id).copied().unwrap_or_default();
            if let Some(proxy)=self.picks.get(&doc.file_id) {if let Some(inherited)=base.get(proxy) {flags.visible&=inherited.visible;flags.selectable&=inherited.selectable;}}
            let mut current=by_object.get(doc.file_id.as_str()).copied();let mut visited=HashSet::new();
            while let Some(id)=current {
                if !visited.insert(id) {flags.visible=false;flags.selectable=false;break;}
                let Some(transform)=transforms.get(id) else {break;};
                if let Some(parent)=transform.game_object.as_ref().and_then(|object|base.get(object)) {flags.visible&=parent.visible;flags.selectable&=parent.selectable;}
                current=transform.parent_transform.as_deref().filter(|id|*id!="0");
            }
            result.insert(doc.file_id.clone(),flags);
        }
        let result=Arc::new(result);*self.flags_cache.lock().unwrap()=Some((key,result.clone()));result
    }
}
struct Task {key:u64,cancelled:Arc<AtomicBool>,receiver:Option<mpsc::Receiver<Expanded>>}
impl Drop for Task {fn drop(&mut self) {self.cancelled.store(true,std::sync::atomic::Ordering::Relaxed);}}
#[derive(Default)]
pub(super) struct Cache {task:Option<Task>,result:Option<Arc<Expanded>>,next_check:Option<Instant>}
impl Cache {
    pub(super) fn snapshot(&self) -> Option<Arc<Expanded>> {self.result.clone()}
    pub(super) fn has_state(&self) -> bool {self.task.is_some() || self.result.is_some()}
    pub(super) fn cancel(&self) {if let Some(task)=&self.task {task.cancelled.store(true,std::sync::atomic::Ordering::Relaxed);}}
    pub(super) fn resolve(&mut self,ctx:&egui::Context,docs:&[Document],catalog:Option<Arc<HashMap<String,PathBuf>>>,paused:Arc<AtomicBool>) -> (Option<Arc<Expanded>>,bool) {
        use std::hash::{Hash,Hasher};
        if !docs.iter().any(|doc|doc.type_name=="PrefabInstance") {return (None,false);}
        let mut hash=std::collections::hash_map::DefaultHasher::new();
        for doc in docs {doc.text.as_ptr().hash(&mut hash);doc.text.len().hash(&mut hash);}
        catalog.as_ref().map(|c|Arc::as_ptr(c) as usize).hash(&mut hash);let key=hash.finish();
        let busy=self.task.as_ref().is_some_and(|task|task.receiver.is_some());
        let mut changed=false;
        if self.next_check.is_none_or(|time|Instant::now()>=time) {
            self.next_check=Some(Instant::now()+Duration::from_secs(1));
            if !busy {if let Some(result)=&self.result {changed=result.deps.iter().any(|(path,time,len)|std::fs::metadata(path).map_or(time.is_some()||*len!=0,|m|m.modified().ok()!=*time||m.len()!=*len));}}
        }
        if changed || self.task.as_ref().is_none_or(|job|job.key!=key) {
            self.task=None;
            if !changed {self.result=None;}
            let cancelled=Arc::new(AtomicBool::new(false));let stop=cancelled.clone();let docs=docs.to_vec();let repaint=ctx.clone();let (sender,receiver)=mpsc::channel();
            std::thread::spawn(move|| {
                let mut resolver=Resolver::new(catalog.as_deref(),&stop,&paused,&docs);
                let level=resolver.level(&docs,None,None,0);
                let result=Expanded{documents:level.docs,picks:level.picks,prefab_roots:level.prefab_roots,issues:resolver.issues,flags_cache:Default::default(),deps:resolver.deps};
                if !stop.load(std::sync::atomic::Ordering::Relaxed) {let _=sender.send(result);repaint.request_repaint();}
            });
            self.task=Some(Task{key,cancelled,receiver:Some(receiver)});
        }
        let incoming=self.task.as_ref().and_then(|task|task.receiver.as_ref()?.try_recv().ok());
        if let Some(result)=incoming {self.result=Some(Arc::new(result));self.task.as_mut().unwrap().receiver=None;}
        let pending=self.task.as_ref().is_some_and(|task|task.receiver.is_some());
        if pending {ctx.request_repaint_after(Duration::from_millis(100));}
        (self.result.clone(),pending)
    }
}
#[derive(Default)]
struct Level {docs:Vec<Document>,sources:HashMap<Source,String>,ambiguous:HashSet<Source>,picks:HashMap<String,String>,prefab_roots:HashMap<String,String>}
struct Resolver<'a> {
    catalog:Option<&'a HashMap<String,PathBuf>>,stop:&'a AtomicBool,paused:&'a AtomicBool,
    templates:HashMap<String,Arc<Vec<Document>>>,stack:HashSet<String>,used:HashSet<String>,next:i64,
    issues:Vec<String>,deps:Vec<(PathBuf,Option<SystemTime>,u64)>,bytes:usize,generated:usize,
    generated_limit:usize,
}
impl<'a> Resolver<'a> {
    fn new(catalog:Option<&'a HashMap<String,PathBuf>>,stop:&'a AtomicBool,paused:&'a AtomicBool,docs:&[Document]) -> Self {
        Self{catalog,stop,paused,templates:HashMap::new(),stack:HashSet::new(),used:docs.iter().map(|d|d.file_id.clone()).collect(),next:i64::MIN,issues:Vec::new(),deps:Vec::new(),bytes:0,generated:0,generated_limit:EXPANSION_BUDGET}
    }
    fn id(&mut self) -> String {loop {let id=self.next.to_string();self.next+=1;if self.used.insert(id.clone()) {return id;}}}
    fn template(&mut self,guid:&str,instance:&Document,level_docs:&[Document]) -> Option<Arc<Vec<Document>>> {
        if let Some(docs)=self.templates.get(guid) {return Some(docs.clone());}
        let path=self.catalog?.get(guid)?.clone();
        if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("fbx")) {
            let metadata=std::fs::metadata(&path).ok()?;
            self.deps.push((path.clone(),metadata.modified().ok(),metadata.len()));
            let meta=PathBuf::from(format!("{}.meta",path.display()));
            if let Ok(m)=std::fs::metadata(&meta){self.deps.push((meta,m.modified().ok(),m.len()));}
            match model_template(&path,guid,instance,level_docs) {
                Ok(docs)=>{let docs=Arc::new(docs);self.templates.insert(guid.into(),docs.clone());return Some(docs);}
                Err(error)=>{self.issues.push(format!("Model Prefab {}: {error}",path.display()));return None;}
            }
        }
        if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("prefab")) {return None;}
        let metadata=match std::fs::metadata(&path) {Ok(metadata)=>metadata,Err(_)=>{self.deps.push((path,None,0));return None;}};
        if metadata.len()>64*1024*1024 || self.bytes+metadata.len() as usize>SOURCE_BUDGET {self.issues.push(format!("Prefab template budget exceeded: {} (single file limit 64 MiB, total 512 MiB)",path.display()));return None;}
        self.deps.push((path.clone(),metadata.modified().ok(),metadata.len()));
        let text=std::fs::read_to_string(path).ok()?;self.bytes+=text.len();
        let docs=Arc::new(crate::parse_unity_yaml_cancellable(&text,Some(self.stop)));self.templates.insert(guid.into(),docs.clone());Some(docs)
    }
    fn level(&mut self,docs:&[Document],guid:Option<&str>,pick:Option<&str>,depth:usize) -> Level {
        let mut result=Level::default();let mut ids=HashMap::new();
        for doc in docs {ids.insert(doc.file_id.clone(),if guid.is_some() {self.id()}else {doc.file_id.clone()});}
        let mut aliases=HashMap::new();
        // Index once per level rather than scanning every document for each instance.
        let mut instance_refs:HashMap<String,Vec<&Document>>=HashMap::new();
        for item in docs {
            if let Some(instance)=reference_field(&item.text,"m_PrefabInstance").and_then(|v|reference(&v,"fileID")).filter(|id|id!="0") {
                instance_refs.entry(instance).or_default().push(item);
            }
        }
        for doc in docs.iter().filter(|d|d.type_name=="PrefabInstance") {
            if !checkpoint(self.stop,Some(self.paused)) {return result;}
            let source=reference_field(&doc.text,"m_SourcePrefab").and_then(|v|reference(&v,"guid")).map(|s|s.to_lowercase());
            let Some(source)=source else {self.issues.push(format!("PrefabInstance {}: missing source GUID",doc.file_id));continue;};
            if depth>=16 || self.stack.contains(&source) {self.issues.push(format!("PrefabInstance {}: recursive reference or depth limit",doc.file_id));continue;}
            let Some(template)=self.template(&source,doc,docs) else {self.issues.push(format!("PrefabInstance {}: source {} unavailable or exceeds preview budget",doc.file_id,source));continue;};
            self.stack.insert(source.clone());
            let proxy=pick.unwrap_or(&doc.file_id);
            let mut child=self.level(&template,Some(&source),Some(proxy),depth+1);
            self.stack.remove(&source);
            let mut renamed=HashMap::new();
            for stripped in instance_refs.get(&doc.file_id).into_iter().flatten() {
                if let Some((g,id))=reference_field(&stripped.text,"m_CorrespondingSourceObject").and_then(|v|source_ref(&v)) {
                    if let Some(actual)=child.sources.get(&(g,id)) {
                        let alias=ids[&stripped.file_id].clone();renamed.insert(actual.clone(),alias.clone());aliases.insert(stripped.file_id.clone(),alias);
                    }
                }
            }
            if !renamed.is_empty() {
                for item in &mut child.docs {remap(item,&renamed);}
                for id in child.sources.values_mut() {if let Some(new)=renamed.get(id) {*id=new.clone();}}
                let old=std::mem::take(&mut child.picks);for (id,proxy) in old {let mapped=renamed.get(&id).unwrap_or(&id).clone();let proxy=if guid.is_none() && renamed.contains_key(&id) {mapped.clone()} else {proxy};child.picks.insert(mapped,proxy);}
                let old=std::mem::take(&mut child.prefab_roots);for (id,source) in old {child.prefab_roots.insert(renamed.get(&id).unwrap_or(&id).clone(),source);}
            }
            let lookup:HashMap<_,_>=child.docs.iter().enumerate().map(|(i,d)|(d.file_id.clone(),i)).collect();
            for modification in modifications(&doc.text) {
                let Some(target)=child.sources.get(&modification.target) else {
                    let absent=modification.target.0==source && !template.iter().any(|d|d.file_id==modification.target.1);
                    let reason=if absent {"absent from source asset (stale override)"}else{"not resolved"};
                    self.issues.push(format!("PrefabInstance {}: override target {}/{} {reason} ({})",doc.file_id,modification.target.0,modification.target.1,modification.path));continue;
                };
                let Some(index)=lookup.get(target) else {continue;};
                let resolved_object=source_ref(&modification.object).and_then(|source|child.sources.get(&source)).map(|id|format!("{{fileID: {id}}}"));
                let value=if let Some(reference)=resolved_object {reference} else if modification.object.starts_with('{') && (reference(&modification.object,"fileID").is_some_and(|id|id!="0") || field(&child.docs[*index].text,&modification.path).is_some_and(|value|value.starts_with("{fileID:"))) {rewrite_refs(&modification.object,&ids)} else {modification.value};
                if !patch(&mut child.docs[*index],&modification.path,&value) {self.issues.push(format!("PrefabInstance {}: unsupported override {}",doc.file_id,modification.path));}
            }
            let removed:HashSet<_>=removed_references(&doc.text).iter().filter_map(|reference|child.sources.get(reference).cloned()).collect();
            let mut removed=removed;
            loop {
                let before=removed.len();
                for item in &child.docs {if item.game_object.as_ref().is_some_and(|id|removed.contains(id)) || item.parent_transform.as_ref().is_some_and(|id|removed.contains(id)) {removed.insert(item.file_id.clone());}}
                if removed.len()==before {break;}
            }
            child.docs.retain(|d|!removed.contains(&d.file_id));
            let parent=reference_field(&doc.text,"m_TransformParent").and_then(|v|reference(&v,"fileID")).unwrap_or_else(||"0".into());
            let roots:Vec<_>=child.docs.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform") && d.parent_transform.as_deref().is_none_or(|id|id=="0")).map(|d|d.file_id.clone()).collect();
            for root in child.docs.iter().filter(|d|roots.contains(&d.file_id)) {if let Some(object)=&root.game_object {child.prefab_roots.insert(object.clone(),source.clone());}}
            child.prefab_roots.retain(|id,_|!removed.contains(id));
            for item in &mut child.docs {if roots.contains(&item.file_id) {
                let father=ids.get(&parent).unwrap_or(&parent);
                patch(item,"m_Father",&format!("{{fileID: {father}}}"));
            }}
            for field in ["m_AddedComponents","m_AddedGameObjects"] {if doc.text.lines().any(|line|line.trim().starts_with(&format!("{field}:")) && !line.trim().ends_with("[]")) {self.issues.push(format!("PrefabInstance {}: {} uses serialized local documents; insertion order may be approximate",doc.file_id,field));}}
            limit_issues(&mut self.issues);
            result.docs.extend(child.docs);
            result.ambiguous.extend(child.ambiguous);
            for source in &result.ambiguous {result.sources.remove(source);}
            for (source,id) in child.sources {
                if result.ambiguous.contains(&source) {continue;}
                if result.sources.get(&source).is_some_and(|old|*old!=id) {result.sources.remove(&source);result.ambiguous.insert(source);} else {result.sources.insert(source,id);}
            }
            result.picks.extend(child.picks);
            result.prefab_roots.extend(child.prefab_roots);
        }
        for doc in docs {
            if !checkpoint(self.stop,Some(self.paused)) {return result;}
            if doc.type_name=="PrefabInstance" || doc.text.lines().next().is_some_and(|line|line.ends_with(" stripped")) {continue;}
            let mut copy=doc.clone();
            if guid.is_some() {remap(&mut copy,&ids);self.generated+=copy.text.len();if self.generated>self.generated_limit || result.docs.len()>EXPANSION_DOCUMENT_LIMIT {let issue=format!("Prefab expansion budget exceeded ({} MiB text, {} documents)",self.generated_limit/(1024*1024),EXPANSION_DOCUMENT_LIMIT);if !self.issues.contains(&issue){self.issues.push(issue);}break;}}
            else {let changed=copy.game_object.as_ref().is_some_and(|id|aliases.contains_key(id))||copy.parent_transform.as_ref().is_some_and(|id|aliases.contains_key(id));if changed {remap(&mut copy,&ids);}}
            if let Some(guid)=guid {result.sources.insert((guid.into(),doc.file_id.clone()),copy.file_id.clone());}
            if let Some(proxy)=pick {result.picks.insert(copy.file_id.clone(),proxy.into());}
            result.docs.push(copy);
        }
        // Expose this prefab's stripped IDs to overrides in its parent instance.
        if let Some(guid)=guid {for (original,actual) in aliases {result.sources.insert((guid.into(),original),actual);}}
        result
    }
}
fn limit_issues(issues:&mut Vec<String>) {
    if issues.len()>256 {issues.sort_by_key(|issue|!issue.contains("budget exceeded"));issues.truncate(256);}
}
fn reference_field(text:&str,key:&str) -> Option<String> {
    let mut lines=text.lines();
    while let Some(line)=lines.next() {
        let trimmed=line.trim();let value=trimmed.strip_prefix(key).and_then(|line|line.strip_prefix(':'));
        if let Some(value)=value {let mut value=value.trim().to_owned();while value.starts_with('{') && !value.contains('}') {value.push(' ');value.push_str(lines.next()?.trim());}return Some(value);}
    }
    None
}
/// A single imported mesh node has unique GO/Transform/Renderer roles. Source
/// IDs are taken from serialized typed references, never computed from names.
fn model_template(path:&std::path::Path,guid:&str,instance:&Document,docs:&[Document])->Result<Vec<Document>,String> {
    if std::fs::metadata(path).map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("FBX exceeds 64 MiB".into());}
    let mut opts=ufbx::LoadOpts::default();opts.ignore_animation=true;opts.ignore_embedded=true;
    opts.temp_allocator.memory_limit=128*1024*1024;opts.result_allocator.memory_limit=128*1024*1024;
    let scene=ufbx::load_file(path.to_str().ok_or("Invalid FBX path")?,opts).map_err(|e|format!("FBX import failed: {e:?}"))?;
    let nodes:Vec<_>=scene.nodes.iter().filter(|n|!n.is_root).collect();
    if nodes.len()!=1 || nodes[0].mesh.is_none(){return static_skin_template(path,guid,instance,docs,&scene);}
    let local=nodes[0].local_transform;
    if [local.translation.x,local.translation.y,local.translation.z,local.rotation.x,local.rotation.y,local.rotation.z,local.scale.x,local.scale.y,local.scale.z,local.rotation.w].iter().any(|v|!v.is_finite()){return Err("Non-finite FBX root transform".into());}
    let mut roles:HashMap<&str,HashSet<String>>=HashMap::new();
    for modification in modifications(&instance.text) {
        if modification.target.0!=guid {continue;}
        let role=if modification.path.starts_with("m_Local") {Some("Transform")}else if modification.path.starts_with("m_Materials") || modification.path.starts_with("m_Sorting") || modification.path=="m_Enabled" {Some("MeshRenderer")}else if matches!(modification.path.as_str(),"m_Name"|"m_IsActive"|"m_Layer"|"m_TagString") {Some("GameObject")}else{None};
        if let Some(role)=role {roles.entry(role).or_default().insert(modification.target.1);}
    }
    for doc in docs.iter().filter(|d|reference_field(&d.text,"m_PrefabInstance").and_then(|v|reference(&v,"fileID")).as_deref()==Some(instance.file_id.as_str())) {
        if let Some((g,id))=reference_field(&doc.text,"m_CorrespondingSourceObject").and_then(|v|source_ref(&v)) {
            if g==guid && matches!(doc.type_name.as_str(),"GameObject"|"Transform"|"MeshRenderer") {let role=match doc.type_name.as_str(){"GameObject"=>"GameObject","Transform"=>"Transform",_=>"MeshRenderer"};roles.entry(role).or_default().insert(id);}
        }
    }
    let unique=|role:&str|->Result<String,String>{let ids=roles.get(role).ok_or_else(||format!("Missing typed source ID for {role}"))?;if ids.len()!=1{return Err(format!("Ambiguous source IDs for {role}"));}Ok(ids.iter().next().unwrap().clone())};
    let go=unique("GameObject")?;let transform=unique("Transform")?;let renderer=unique("MeshRenderer")?;
    if [&go,&transform,&renderer].into_iter().collect::<HashSet<_>>().len()!=3{return Err("Conflicting Model Prefab source IDs".into());}
    if [&go,&transform,&renderer].iter().any(|id|id.as_str()=="3300000"){return Err("Model Prefab virtual MeshFilter ID conflicts with a source object".into());}
    let filter="__model_mesh_filter";
    // Virtual filter IDs are local to the synthesized template and remapped
    // before being exposed as scene documents.
    let name=path.file_stem().unwrap_or_default().to_string_lossy();
    let text=format!("--- !u!1 &{go}\nGameObject:\n  m_Name: {name}\n  m_IsActive: 1\n  m_Component:\n  - component: {{fileID: {transform}}}\n  - component: {{fileID: {filter}}}\n  - component: {{fileID: {renderer}}}\n--- !u!4 &{transform}\nTransform:\n  m_GameObject: {{fileID: {go}}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_LocalEulerAnglesHint: {{x: 0, y: 0, z: 0}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!33 &3300000\nMeshFilter:\n  m_GameObject: {{fileID: {go}}}\n  m_Mesh: {{fileID: 4300000, guid: {guid}, type: 3}}\n--- !u!23 &{renderer}\nMeshRenderer:\n  m_GameObject: {{fileID: {go}}}\n  m_Enabled: 1\n  m_Materials:\n  - {{fileID: 0}}\n  m_SortingLayerID: 0\n  m_SortingLayer: 0\n  m_SortingOrder: 0\n");
    let meta=std::fs::read_to_string(format!("{}.meta",path.display())).unwrap_or_default();
    let scale=crate::model_preview::importer_scale(&meta,scene.settings.unit_meters)?;
    let text=text.replace(filter,"3300000")
        .replace("m_LocalPosition: {x: 0, y: 0, z: 0}",&format!("m_LocalPosition: {{x: {}, y: {}, z: {}}}",-local.translation.x*scale,local.translation.y*scale,local.translation.z*scale))
        .replace("m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",&format!("m_LocalRotation: {{x: {}, y: {}, z: {}, w: {}}}",local.rotation.x,-local.rotation.y,-local.rotation.z,local.rotation.w))
        .replace("m_LocalScale: {x: 1, y: 1, z: 1}",&format!("m_LocalScale: {{x: {}, y: {}, z: {}}}",local.scale.x,local.scale.y,local.scale.z));
    Ok(crate::parse_unity_yaml(&text))
}
fn static_skin_template(path:&std::path::Path,guid:&str,instance:&Document,docs:&[Document],scene:&ufbx::Scene)->Result<Vec<Document>,String> {
    let meshes:Vec<_>=scene.nodes.iter().filter(|n|n.mesh.is_some()).collect();
    if meshes.len()!=1 || meshes[0].mesh.as_ref().unwrap().skin_deformers.is_empty(){return Err("Static Model Prefab requires one skinned mesh".into());}
    let source_id=|kind:&str|->Result<String,String>{
        let ids:HashSet<_>=docs.iter().filter(|d|d.type_name==kind && reference_field(&d.text,"m_PrefabInstance").and_then(|v|reference(&v,"fileID")).as_deref()==Some(instance.file_id.as_str())).filter_map(|d|reference_field(&d.text,"m_CorrespondingSourceObject").and_then(|v|source_ref(&v))).filter(|(g,_)|g==guid).map(|(_,id)|id).collect();
        if ids.len()!=1{return Err(format!("Ambiguous or missing static skinned {kind} source ID"));}Ok(ids.into_iter().next().unwrap())
    };
    let transform=source_id("Transform")?;let renderer=source_id("SkinnedMeshRenderer")?;
    let go="-1";if transform==go || renderer==go || renderer==transform{return Err("Static skinned source ID conflict".into());}
    let meta=std::fs::read_to_string(format!("{}.meta",path.display())).unwrap_or_default();
    let materials=static_material_refs(&meta,meshes[0].materials.iter().map(|m|m.element.name.as_ref()));
    let material=materials.first().copied().unwrap_or("{fileID: 0}");
    let name=path.file_stem().unwrap_or_default().to_string_lossy();
    let text=format!("--- !u!1 &{go}\nGameObject:\n  m_Name: {name}\n  m_IsActive: 1\n  m_Component:\n  - component: {{fileID: {transform}}}\n  - component: {{fileID: {renderer}}}\n--- !u!4 &{transform}\nTransform:\n  m_GameObject: {{fileID: {go}}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Father: {{fileID: 0}}\n  m_Children: []\n--- !u!137 &{renderer}\nSkinnedMeshRenderer:\n  m_GameObject: {{fileID: {go}}}\n  m_Enabled: 1\n  m_Mesh: {{fileID: 4300000, guid: {guid}, type: 3}}\n  m_Materials:\n  - {material}\n  m_SortingOrder: 0\n  m_Bones: []\n  preview_FbxStaticPose: 1\n");
    let mut scales:HashMap<String,[Option<f64>;3]>=HashMap::new();
    for m in modifications(&instance.text).into_iter().filter(|m|m.target.0==guid && m.target.1!=transform) {
        if let Some(axis)=["m_LocalScale.x","m_LocalScale.y","m_LocalScale.z"].iter().position(|p|*p==m.path) {scales.entry(m.target.1).or_insert([None;3])[axis]=m.value.parse().ok();}
    }
    let candidates:Vec<_>=scales.values().filter_map(|v|Some([v[0]?,v[1]?,v[2]?])).collect();
    let bones:Vec<_>=scene.nodes.iter().filter(|n|n.bone.is_some() && (n.local_transform.scale.x-1.0).abs()>0.001 && (n.local_transform.scale.x-n.local_transform.scale.y).abs()<0.001 && (n.local_transform.scale.x-n.local_transform.scale.z).abs()<0.001).collect();
    let mut text=text;
    if materials.len()>1 {text=text.replace(&format!("  m_Materials:\n  - {material}\n"),&format!("  m_Materials:\n{}",materials.iter().map(|m|format!("  - {m}\n")).collect::<String>()));}
    if candidates.len()==1 && bones.len()==1 && candidates[0].iter().all(|v|v.is_finite() && (*v-candidates[0][0]).abs()<0.0001) {
        let root=bones[0];let skin=meshes[0].mesh.as_ref().unwrap().skin_deformers.first().unwrap();
        let all_descend=skin.clusters.iter().all(|c|{let mut node=c.bone_node.as_ref();while let Some(n)=node {if n.element.element_id==root.element.element_id{return true;}node=n.parent.as_ref();}false});
        if all_descend {text.push_str(&format!("  preview_FbxBoneScaleRatio: {}\n",candidates[0][0]/root.local_transform.scale.x));}
    }
    Ok(crate::parse_unity_yaml(&text))
}
fn static_material_refs<'a>(meta:&'a str,names:impl Iterator<Item=&'a str>)->Vec<&'a str> {
    let mut mapped=HashMap::new();let mut name=None;
    if let Some((_,tail))=meta.split_once("  externalObjects:") {
        for line in tail.lines().skip(1) {
            if !line.starts_with("    ") && !line.starts_with("  - ") {break;}
            if line.trim()=="- first:" {name=None;}
            if let Some(value)=line.trim().strip_prefix("name:") {name=Some(value.trim());}
            if let Some(value)=line.trim().strip_prefix("second:") {if let Some(name)=name.take(){mapped.insert(name,value.trim());}}
        }
    }
    names.map(|name|mapped.get(name).copied().unwrap_or("{fileID: 0}")).collect()
}
fn source_ref(value:&str) -> Option<Source> {Some((reference(value,"guid")?.to_lowercase(),reference(value,"fileID")?))}
fn rewrite_refs(text:&str,ids:&HashMap<String,String>) -> String {
    static REFERENCES:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
    REFERENCES.get_or_init(||regex::Regex::new(r"\{fileID:\s*(-?\d+)[^}]*\}").unwrap()).replace_all(text,|captures:&regex::Captures| {
        let value=&captures[0];if value.contains("guid:") {value.to_owned()} else {ids.get(&captures[1]).map_or_else(||value.to_owned(),|id|format!("{{fileID: {id}}}"))}
    }).into_owned()
}
fn remap(doc:&mut Document,ids:&HashMap<String,String>) {
    doc.file_id=ids.get(&doc.file_id).unwrap_or(&doc.file_id).clone();
    doc.game_object=doc.game_object.as_ref().map(|id|ids.get(id).unwrap_or(id).clone());
    doc.parent_transform=doc.parent_transform.as_ref().map(|id|ids.get(id).unwrap_or(id).clone());
    doc.text=rewrite_refs(&doc.text,ids).into();
}
struct Modification {target:Source,path:String,value:String,object:String}
fn modifications(text:&str) -> Vec<Modification> {
    let mut result=Vec::new();let mut current:Option<Modification>=None;
    let mut lines=text.lines().peekable();
    while let Some(line)=lines.next() {
        let line=line.trim();
        if let Some(value)=line.strip_prefix("- target:") {
            if let Some(item)=current.take() {result.push(item);}
            let mut value=value.trim().to_owned();while !value.contains('}') {let Some(next)=lines.next() else {break;};value.push(' ');value.push_str(next.trim());}
            current=source_ref(&value).map(|target|Modification{target,path:String::new(),value:String::new(),object:String::new()});
        } else if let Some(item)=&mut current {
            if let Some(value)=line.strip_prefix("propertyPath:") {item.path=value.trim().into();}
            if let Some(value)=line.strip_prefix("value:") {item.value=value.trim().into();}
            if let Some(value)=line.strip_prefix("objectReference:") {let mut value=value.trim().to_owned();while value.starts_with('{') && !value.contains('}') {let Some(next)=lines.next() else {break;};value.push(' ');value.push_str(next.trim());}item.object=value;}
        }
    }
    if let Some(item)=current {result.push(item);}result
}
fn removed_references(text:&str) -> Vec<Source> {
    let mut result=Vec::new();let mut active=false;let mut value=String::new();
    for line in text.lines() {
        let trimmed=line.trim();
        if trimmed.starts_with("m_RemovedComponents:") || trimmed.starts_with("m_RemovedGameObjects:") {active=!trimmed.ends_with("[]");continue;}
        if active && trimmed.starts_with("- {") {value=trimmed.trim_start_matches("- ").into();}
        else if active && !value.is_empty() && !value.contains('}') {value.push(' ');value.push_str(trimmed);}
        else if active {active=false;}
        if value.contains('}') {if let Some(source)=source_ref(&value) {result.push(source);}value.clear();}
    }
    result
}
fn patch(doc:&mut Document,path:&str,value:&str) -> bool {
    if let Some(index)=path.strip_prefix("m_Materials.Array.data[").and_then(|p|p.strip_suffix(']')).and_then(|p|p.parse::<usize>().ok()) {
        if index>=64{return false;}
        let mut lines:Vec<String>=doc.text.lines().map(str::to_owned).collect();
        let Some(start)=lines.iter().position(|l|l.trim()=="m_Materials:" || l.trim()=="m_Materials: []") else{return false;};
        let end=(start+1..lines.len()).find(|i|!lines[*i].trim_start().starts_with('-')).unwrap_or(lines.len());
        let mut entries=lines[start+1..end].to_vec();
        while entries.len()<=index {entries.push("  - {fileID: 0}".into());}
        entries[index]=format!("  - {value}");lines[start]="  m_Materials:".into();lines.splice(start+1..end,entries);doc.text=lines.join("\n").into();return true;
    }
    if path.is_empty() || path.contains("Array.") {return false;}
    let parts:Vec<_>=path.split('.').collect();let mut lines:Vec<String>=doc.text.lines().map(str::to_owned).collect();
    let mut range=0..lines.len();let mut indent=2;
    for (part_index,part) in parts.iter().enumerate() {
        let Some(index)=range.clone().find(|i|lines[*i].starts_with(&format!("{}{}:"," ".repeat(indent),part))) else {return false;};
        let prefix=format!("{}{}:"," ".repeat(indent),part);
        if part_index+1==parts.len() {lines[index]=format!("{prefix} {value}");doc.text=lines.join("\n").into();if path=="m_Father" {doc.parent_transform=reference(value,"fileID");}if path=="m_GameObject" {doc.game_object=reference(value,"fileID");}return true;}
        let existing=lines[index][prefix.len()..].trim();
        if existing.starts_with('{') && part_index+2==parts.len() {
            let axis=parts[part_index+1];let mut found=false;
            let values:Vec<_>=existing.trim_matches(|c|c=='{'||c=='}').split(',').map(|entry| {
                if entry.trim().split_once(':').is_some_and(|(key,_)|key.trim()==axis) {found=true;format!("{axis}: {value}")}else {entry.trim().into()}
            }).collect();
            if !found {return false;}lines[index]=format!("{prefix} {{{}}}",values.join(", "));doc.text=lines.join("\n").into();return true;
        }
        let end=(index+1..range.end).find(|i|!lines[*i].trim().is_empty() && lines[*i].len()-lines[*i].trim_start().len()<=indent).unwrap_or(range.end);
        range=index+1..end;indent+=2;
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn static_material_slots_follow_fbx_names_instead_of_meta_order() {
        let meta="  externalObjects:\n  - first:\n      name: wings\n    second: {fileID: 2}\n  - first:\n      name: body\n    second: {fileID: 1}\n  materials:\n    name: ignored\n    second: {fileID: 3}";
        assert_eq!(static_material_refs(meta,["body","wings","missing"].into_iter()),vec!["{fileID: 1}","{fileID: 2}","{fileID: 0}"]);
    }
    #[test]
    #[ignore = "Requires g003 FBX assets"]
    fn inspect_static_fbx_axes() {
        for name in ["Seagull/seagull_skin","Dragonfly/dragonfly_skin","Dolphin/dolphin_skin"] {
            let path=format!("C:/Git/g003_client/Assets/Res/Role/{name}.fbx");
            let Ok(scene)=ufbx::load_file(&path,ufbx::LoadOpts::default()) else {continue};
            println!("{name} axes={:?} unit={}",scene.settings.axes,scene.settings.unit_meters);
            for n in scene.nodes.iter().filter(|n|!n.is_root && (n.mesh.is_some() || n.parent.as_ref().is_some_and(|p|p.is_root))) {println!("{} local={:?} world={:?}",n.element.name.as_ref(),n.local_transform,n.node_to_world);}
            let mut opts=ufbx::LoadOpts::default();opts.evaluate_skinning=true;opts.ignore_animation=true;
            let evaluated=ufbx::load_file(&path,opts).unwrap();
            let raw=scene.meshes.first().unwrap();let posed=evaluated.meshes.first().unwrap();
            let extent=|v:&ufbx::VertexVec3|{let mut min=[f64::INFINITY;3];let mut max=[f64::NEG_INFINITY;3];for p in v.values.iter(){for (a,value) in [p.x,p.y,p.z].iter().enumerate(){min[a]=min[a].min(*value);max[a]=max[a].max(*value);}} std::array::from_fn::<_,3,_>(|a|max[a]-min[a])};
            assert!(posed.skinned_position.exists);
            println!("{name} raw extent {:?}, evaluated extent {:?}",extent(&raw.vertex_position),extent(&posed.skinned_position));
            for n in &scene.nodes {if n.local_transform.scale.x!=1.0 || n.local_transform.scale.y!=1.0 || n.local_transform.scale.z!=1.0 {println!("scaled {} {:?}",n.element.name.as_ref(),n.local_transform.scale);}}
        }
    }
    #[test]
    #[ignore = "Requires the user's g003 project"]
    fn real_seagull_static_pose() {
        let root=PathBuf::from("C:/Git/g003_client/Assets/Res");
        let path=root.join("Role/Seagull/seagull_skin.fbx");
        let source=std::fs::read_to_string(root.join("Prefabs/Map/Npc/seagull.prefab")).unwrap();
        let docs=crate::parse_unity_yaml(&source);
        let instance=docs.iter().find(|d|d.type_name=="PrefabInstance").unwrap();
        let template=model_template(&path,"fd3513352b6517145b14a132e4f4d679",instance,&docs).unwrap();
        let renderer=template.iter().find(|d|d.type_name=="SkinnedMeshRenderer").unwrap();
        assert!(renderer.text.contains("preview_FbxStaticPose: 1"));
        let mut catalog=HashMap::new();catalog.insert("fd3513352b6517145b14a132e4f4d679".into(),path);
        catalog.insert("13cefc8f233d2544fb5b6675f01ec432".into(),root.join("Role/Seagull/seagull_m.mat"));
        catalog.insert("1587872b19ad62e4ea60b8928838410d".into(),root.join("Role/Seagull/seagull_t.tga"));
        let mesh=crate::static_preview::field(&renderer.text,"m_Mesh").unwrap();
        let (image,warnings)=crate::model_preview::bake(mesh,&renderer.text,&catalog,crate::static_preview::Matrix::identity(),&HashMap::new(),None).unwrap();
        assert!(image.image.pixels.iter().any(|p|p.a()>0));
        assert!(!warnings.iter().any(|w|w.contains("贴图")));
        assert!(warnings.iter().any(|w|w.contains("static imported pose")));
    }
    #[test]
    #[ignore = "Requires g003 dragonfly assets"]
    fn real_dragonfly_root_scale_override() {
        let path=PathBuf::from("C:/Git/g003_client/Assets/Res/Role/Dragonfly/dragonfly_skin.fbx");
        let source=std::fs::read_to_string("C:/Git/g003_client/Assets/Res/Prefabs/Map/Npc/dragonfly_red.prefab").unwrap();
        let docs=crate::parse_unity_yaml(&source);let instance=docs.iter().find(|d|d.type_name=="PrefabInstance").unwrap();
        let template=model_template(&path,"c991400fe4fd292408e9407ab78847dc",instance,&docs).unwrap();
        let renderer=template.iter().find(|d|d.type_name=="SkinnedMeshRenderer").unwrap();
        assert!((crate::static_preview::number(&renderer.text,"preview_FbxBoneScaleRatio",1.0)-0.01).abs()<0.000001);
    }
    use super::*;
    #[test]
    fn budget_warning_remains_visible_when_other_warnings_are_capped() {
        let mut issues=(0..256).map(|i|format!("ordinary warning {i}")).collect::<Vec<_>>();
        issues.push("Prefab expansion budget exceeded".into());limit_issues(&mut issues);
        assert_eq!(issues.len(),256);assert!(issues[0].contains("budget exceeded"));
    }
    #[test]
    #[ignore="Requires UNITY_SHADER_TEST_PROJECT and its existing on-disk index"]
    fn real_floor_expansion_budget() {
        use sha2::{Digest,Sha256};
        #[derive(serde::Deserialize)] struct Cached {index:Index}
        #[derive(serde::Deserialize)] struct Index {assets:HashMap<String,PathBuf>}
        let root=PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        let identity=root.to_string_lossy().replace('\\',"/").to_lowercase();
        let cache=crate::last_open_dir_config_path().unwrap().parent().unwrap().join("project_indexes").join(format!("{:x}.json",Sha256::digest(identity.as_bytes())));
        let cached:Cached=serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(cache).unwrap())).unwrap();
        let docs=crate::parse_unity_yaml(&std::fs::read_to_string(root.join("Assets/Scenes/Map/Level_101_Scene.unity")).unwrap());
        let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);
        for budget in [256*1024*1024,EXPANSION_BUDGET] {
            let mut resolver=Resolver::new(Some(&cached.index.assets),&stop,&paused,&docs);resolver.generated_limit=budget;
            let result=resolver.level(&docs,None,None,0);
            let (_,children,_,_)=crate::build_tree(&result.docs);
            let group=result.docs.iter().find(|doc|doc.type_name=="GameObject" && crate::name_from_text(&doc.text).as_deref()==Some("new_home_decoration")).unwrap();
            let mut pending=children.get(&group.file_id).cloned().unwrap_or_default();
            println!("new_home_decoration children={}",pending.len());
            let mut visited=HashSet::new();let mut names=HashSet::new();
            while let Some(index)=pending.pop(){if !visited.insert(index){continue;}let doc=&result.docs[index];if let Some(name)=crate::name_from_text(&doc.text){names.insert(name);}if let Some(child)=children.get(&doc.file_id){pending.extend(child.iter().copied());}}
            let missing:Vec<_>=["new_initial_floor_3_design","new_initial_floor_4_design","new_initial_wallpaper_4_design"].into_iter().filter(|name|!names.contains(*name)).collect();
            println!("budget={} MiB generated={} MiB docs={} missing={:?}",budget/(1024*1024),resolver.generated/(1024*1024),result.docs.len(),missing);
            for issue in resolver.issues.iter().filter(|issue|issue.contains("budget")) {println!("{issue}");}
            if budget==EXPANSION_BUDGET {assert!(missing.is_empty(),"{missing:?}");}
        }
    }
    #[test]
    fn picked_instances_locate_their_own_expanded_roots() {
        let scene=format!("--- !u!1 &1\nGameObject:\n  m_Name: parent\n--- !u!4 &2\nTransform:\n  m_GameObject: {{fileID: 1}}\n  m_Father: {{fileID: 0}}\n{}{}",instance(100,"a",5),instance(200,"a",-5));
        let level=expand(&scene,&[("a",TEMPLATE)]);
        let data=Expanded{documents:level.docs,picks:level.picks,..Default::default()};
        let first=data.hierarchy_object("100").unwrap();let second=data.hierarchy_object("200").unwrap();
        assert_ne!(first,second);
        let ancestors=data.hierarchy_ancestors(&first);
        assert!(ancestors.contains("1") && ancestors.contains(&first));
        assert!(!ancestors.contains(&second));
        let renderer=data.documents.iter().find(|doc|doc.type_name=="SpriteRenderer" && doc.game_object.as_ref()==Some(&first)).unwrap();
        assert_eq!(data.hierarchy_object(&renderer.file_id),Some(first));
        assert!(data.hierarchy_object("unknown").is_none());
    }
    #[test]
    #[ignore="Requires UNITY_SHADER_TEST_PROJECT"]
    fn real_garden_scene_renderer_overrides() {
        let root=PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        let scene=std::fs::read_to_string(root.join("Assets/Scenes/Map/Level_101_Scene.unity")).unwrap();
        let start=scene.find("--- !u!1001 &1640878127").unwrap();
        let end=start+scene[start+1..].find("--- !u!").unwrap()+1;
        let template=std::fs::read_to_string(root.join("Assets/Res/Prefabs/MapDesign/BuildingDesign/garden_bed_design.prefab")).unwrap();
        let level=expand(&scene[start..end],&[("6b155e57ad4ca804198a5b49d547739f",&template)]);
        for renderer in level.docs.iter().filter(|d|d.type_name=="SpriteRenderer") {
            println!("{} {:?} order={} sprite={}",renderer.file_id,renderer.game_object,field(&renderer.text,"m_SortingOrder").unwrap(),field(&renderer.text,"m_Sprite").unwrap());
            let expected=if renderer.text.contains("3a2fa0aeef75d4e78ab93bcdcfcc2a43") {"2000"} else {"7464"};
            assert_eq!(field(&renderer.text,"m_SortingOrder"),Some(expected));
        }
        assert_eq!(level.docs.iter().filter(|d|d.type_name=="SpriteRenderer").count(),3);
    }
    #[test]
    #[ignore="Requires UNITY_SHADER_TEST_PROJECT"]
    fn real_beach_model_prefab_hierarchy() {
        let root=PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        let docs=crate::parse_unity_yaml(&std::fs::read_to_string(root.join("Assets/Res/Prefabs/Map/Level101/map_land.prefab")).unwrap());
        let mut catalog=HashMap::new();
        for name in ["map_101_beach","map_101_beach2","map_101_beach3","map_101_beach4"] {
            let path=root.join(format!("Assets/Res/Mesh/Map/{name}.FBX"));
            let meta=std::fs::read_to_string(format!("{}.meta",path.display())).unwrap();
            let guid=meta.lines().find_map(|l|l.strip_prefix("guid: ")).unwrap();catalog.insert(guid.to_owned(),path);
        }
        let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);
        let mut resolver=Resolver::new(Some(&catalog),&stop,&paused,&docs);let result=resolver.level(&docs,None,None,0);
        for guid in catalog.keys(){assert!(resolver.templates.contains_key(guid),"{}: {:?}",guid,resolver.issues);}
        let beach=result.docs.iter().find(|d|d.file_id=="4130157995768155313").unwrap();
        assert_eq!(beach.parent_transform.as_deref(),Some("768023218"));
        assert!(beach.text.contains("x: -1680"));
        assert!(result.docs.iter().any(|d|d.type_name=="MeshRenderer" && d.text.contains("bfa5a729023b88d4e9c3113e110721a5")));
    }
    #[test]
    #[ignore="Requires UNITY_SHADER_TEST_PROJECT"]
    fn real_effect_override_targets() {
        let root=PathBuf::from(std::env::var("UNITY_SHADER_TEST_PROJECT").unwrap());
        let source=root.join("Assets/Res/Prefabs/Effect/dns_night_home_door_effect.prefab");
        let catalog=HashMap::from([("6a67288b572d5374ca999087ec3355b5".into(),source)]);
        let docs=crate::parse_unity_yaml(&std::fs::read_to_string(root.join("Assets/Res/Prefabs/Map/Level101/map_land.prefab")).unwrap());
        let instance=docs.iter().find(|d|d.file_id=="1395338311").unwrap().clone();
        let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);let mut resolver=Resolver::new(Some(&catalog),&stop,&paused,&docs);
        let result=resolver.level(&[instance],None,None,0);
        assert_eq!(resolver.issues.len(),3,"{:?}",resolver.issues);
        for id in ["6720468148848614069","6720468149440723645","6720468149850780655"] {
            assert!(resolver.issues.iter().any(|e|e.contains(id) && e.contains("absent from source asset")));
        }
        for id in ["6720468148054464119","6720468148055709344"] {
            let mapped=&result.sources[&("6a67288b572d5374ca999087ec3355b5".into(),id.into())];
            let object=result.docs.iter().find(|d|d.file_id==*mapped).unwrap();
            assert_eq!(field(&object.text,"m_IsActive"),Some("1"));
        }
    }
    const TEMPLATE:&str="--- !u!1 &10\nGameObject:\n  m_Name: source\n  m_IsActive: 1\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Father: {fileID: 0}\n--- !u!212 &12\nSpriteRenderer:\n  m_GameObject: {fileID: 10}\n  m_Enabled: 1\n  m_Sprite: {fileID: 21300000, guid: texture, type: 3}\n";
    fn instance(id:i64,guid:&str,x:i64) -> String {format!("--- !u!1001 &{id}\nPrefabInstance:\n  m_Modification:\n    m_TransformParent: {{fileID: 2}}\n    m_Modifications:\n    - target: {{fileID: 11, guid: {guid},\n        type: 3}}\n      propertyPath: m_LocalPosition.x\n      value: {x}\n      objectReference: {{fileID: 0}}\n  m_SourcePrefab: {{fileID: 100100000, guid: {guid}, type: 3}}\n")}
    fn expand(scene:&str,templates:&[(&str,&str)]) -> Level {
        let docs=crate::parse_unity_yaml(scene);let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);let mut resolver=Resolver::new(None,&stop,&paused,&docs);
        for (guid,text) in templates {resolver.templates.insert(guid.to_string(),Arc::new(crate::parse_unity_yaml(text)));}
        resolver.level(&docs,None,None,0)
    }
    #[test]
    fn instances_isolate_ids_apply_transform_overrides_and_resolve_stripped_alias() {
        let scene=format!("--- !u!1 &1\nGameObject:\n  m_Name: parent\n--- !u!4 &2\nTransform:\n  m_GameObject: {{fileID: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Father: {{fileID: 0}}\n{}{}--- !u!4 &101 stripped\nTransform:\n  m_CorrespondingSourceObject: {{fileID: 11, guid: a, type: 3}}\n  m_PrefabInstance: {{fileID: 100}}\n",instance(100,"a",5),instance(200,"a",-5));
        let level=expand(&scene,&[("a",TEMPLATE)]);
        assert_eq!(level.prefab_roots.len(),2);
        assert!(level.prefab_roots.values().all(|guid|guid=="a"));
        assert_eq!(level.docs.iter().filter(|d|d.type_name=="SpriteRenderer").count(),2);
        assert_eq!(level.docs.iter().map(|d|&d.file_id).collect::<HashSet<_>>().len(),level.docs.len());
        let transforms:HashMap<_,_>=level.docs.iter().filter(|d|d.type_name=="Transform").map(|d|(d.file_id.as_str(),d)).collect();
        let objects:HashMap<_,_>=level.docs.iter().filter(|d|d.type_name=="GameObject").map(|d|(d.file_id.as_str(),d)).collect();
        assert_eq!(world("101",&transforms,&objects,&mut HashSet::new()).unwrap().point(0.0,0.0),[5.0,0.0]);
        let other=level.docs.iter().find(|d|d.type_name=="Transform" && field(&d.text,"m_LocalPosition").is_some_and(|v|v.contains("x: -5"))).unwrap();
        assert_eq!(world(&other.file_id,&transforms,&objects,&mut HashSet::new()).unwrap().point(0.0,0.0),[-5.0,0.0]);
        for sprite in level.docs.iter().filter(|d|d.type_name=="SpriteRenderer") {assert!(field(&sprite.text,"m_Sprite").unwrap().contains("guid: texture"));assert!(level.picks.contains_key(&sprite.file_id));}
    }
    #[test]
    fn variant_override_reaches_inherited_transform_and_removed_component_is_excluded() {
        let variant="--- !u!1001 &20\nPrefabInstance:\n  m_Modification:\n    m_TransformParent: {fileID: 0}\n    m_Modifications: []\n  m_SourcePrefab: {fileID: 100100000, guid: a, type: 3}\n--- !u!4 &21 stripped\nTransform:\n  m_CorrespondingSourceObject: {fileID: 11, guid: a, type: 3}\n  m_PrefabInstance: {fileID: 20}\n";
        let scene=instance(100,"v",7).replace("fileID: 11, guid: v","fileID: 21, guid: v");
        let level=expand(&scene,&[("v",variant),("a",TEMPLATE)]);
        assert!(level.docs.iter().any(|d|field(&d.text,"m_LocalPosition").is_some_and(|v|v.contains("x: 7"))));
        let scene=instance(100,"a",0).replace("  m_SourcePrefab:","    m_RemovedComponents:\n    - {fileID: 12, guid: a, type: 3}\n  m_SourcePrefab:");
        let level=expand(&scene,&[("a",TEMPLATE)]);assert!(!level.docs.iter().any(|d|d.type_name=="SpriteRenderer"));
    }
    #[test]
    fn patches_nested_fields_and_keeps_external_refs_and_source_unchanged() {
        let mut doc=crate::parse_unity_yaml("--- !u!114 &1\nMonoBehaviour:\n  m_FontData:\n    m_FontSize: 14\n  m_Color: {r: 1, g: 1, b: 1, a: 1}\n").remove(0);
        let original=doc.text.clone();assert!(patch(&mut doc,"m_FontData.m_FontSize","28"));assert!(patch(&mut doc,"m_Color.a","0.5"));
        assert!(doc.text.contains("m_FontSize: 28"));assert!(doc.text.contains("a: 0.5"));assert!(original.contains("m_FontSize: 14"));
        assert!(!patch(&mut doc,"m_Children.Array.size","2"));
        let mut renderer=crate::parse_unity_yaml("--- !u!23 &2\nMeshRenderer:\n  m_Materials:\n  - {fileID: 0}\n  m_SortingOrder: 0\n").remove(0);
        assert!(patch(&mut renderer,"m_Materials.Array.data[0]","{fileID: 2100000, guid: beach, type: 2}"));
        assert!(renderer.text.contains("guid: beach"));
        assert_eq!(rewrite_refs("{fileID: 12, guid: asset, type: 3}",&HashMap::from([("12".into(),"99".into())])),"{fileID: 12, guid: asset, type: 3}");
    }
    #[test]
    fn recursive_sources_are_bounded_and_cancelled_expansion_stops() {
        let docs=crate::parse_unity_yaml(&instance(1,"a",0));let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);let mut resolver=Resolver::new(None,&stop,&paused,&docs);
        resolver.templates.insert("a".into(),Arc::new(docs.clone()));resolver.level(&docs,None,None,0);assert!(resolver.issues.iter().any(|issue|issue.contains("recursive")));
        stop.store(true,std::sync::atomic::Ordering::Relaxed);assert!(resolver.level(&docs,None,None,0).docs.is_empty());
    }
}
