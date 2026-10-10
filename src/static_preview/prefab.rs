//! Read-only expansion of linked prefab instances for preview documents.
use super::*;
use std::sync::atomic::AtomicBool;
use std::time::{Duration,Instant};
type Source=(String,String);
#[derive(Default)]
pub(crate) struct Expanded {pub documents:Vec<Document>,pub picks:HashMap<String,String>,pub issues:Vec<String>,flags_cache:std::sync::Mutex<Option<(u64,Arc<HashMap<String,crate::hierarchy_controls::Flags>>)>>,deps:Vec<(PathBuf,Option<SystemTime>,u64)>}
impl Expanded {
    pub(super) fn selection(&self,id:String) -> String {self.picks.get(&id).unwrap_or(&id).clone()}
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
                let result=Expanded{documents:level.docs,picks:level.picks,issues:resolver.issues,flags_cache:Default::default(),deps:resolver.deps};
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
struct Level {docs:Vec<Document>,sources:HashMap<Source,String>,ambiguous:HashSet<Source>,picks:HashMap<String,String>}
struct Resolver<'a> {
    catalog:Option<&'a HashMap<String,PathBuf>>,stop:&'a AtomicBool,paused:&'a AtomicBool,
    templates:HashMap<String,Vec<Document>>,stack:HashSet<String>,used:HashSet<String>,next:i64,
    issues:Vec<String>,deps:Vec<(PathBuf,Option<SystemTime>,u64)>,bytes:usize,generated:usize,
}
impl<'a> Resolver<'a> {
    fn new(catalog:Option<&'a HashMap<String,PathBuf>>,stop:&'a AtomicBool,paused:&'a AtomicBool,docs:&[Document]) -> Self {
        Self{catalog,stop,paused,templates:HashMap::new(),stack:HashSet::new(),used:docs.iter().map(|d|d.file_id.clone()).collect(),next:i64::MIN,issues:Vec::new(),deps:Vec::new(),bytes:0,generated:0}
    }
    fn id(&mut self) -> String {loop {let id=self.next.to_string();self.next+=1;if self.used.insert(id.clone()) {return id;}}}
    fn template(&mut self,guid:&str) -> Option<Vec<Document>> {
        if let Some(docs)=self.templates.get(guid) {return Some(docs.clone());}
        let path=self.catalog?.get(guid)?.clone();
        if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("prefab")) {return None;}
        let metadata=match std::fs::metadata(&path) {Ok(metadata)=>metadata,Err(_)=>{self.deps.push((path,None,0));return None;}};
        if metadata.len()>64*1024*1024 || self.bytes+metadata.len() as usize>256*1024*1024 {return None;}
        self.deps.push((path.clone(),metadata.modified().ok(),metadata.len()));
        let text=std::fs::read_to_string(path).ok()?;self.bytes+=text.len();
        let docs=crate::parse_unity_yaml_cancellable(&text,Some(self.stop));self.templates.insert(guid.into(),docs.clone());Some(docs)
    }
    fn level(&mut self,docs:&[Document],guid:Option<&str>,pick:Option<&str>,depth:usize) -> Level {
        let mut result=Level::default();let mut ids=HashMap::new();
        for doc in docs {ids.insert(doc.file_id.clone(),if guid.is_some() {self.id()}else {doc.file_id.clone()});}
        let mut aliases=HashMap::new();
        for doc in docs.iter().filter(|d|d.type_name=="PrefabInstance") {
            if !checkpoint(self.stop,Some(self.paused)) {return result;}
            let source=reference_field(&doc.text,"m_SourcePrefab").and_then(|v|reference(&v,"guid")).map(|s|s.to_lowercase());
            let Some(source)=source else {self.issues.push(format!("PrefabInstance {}: missing source GUID",doc.file_id));continue;};
            if depth>=16 || self.stack.contains(&source) {self.issues.push(format!("PrefabInstance {}: recursive reference or depth limit",doc.file_id));continue;}
            let Some(template)=self.template(&source) else {self.issues.push(format!("PrefabInstance {}: source {} unavailable or exceeds preview budget",doc.file_id,source));continue;};
            self.stack.insert(source.clone());
            let proxy=pick.unwrap_or(&doc.file_id);
            let mut child=self.level(&template,Some(&source),Some(proxy),depth+1);
            self.stack.remove(&source);
            let mut renamed=HashMap::new();
            for stripped in docs.iter().filter(|d|reference_field(&d.text,"m_PrefabInstance").and_then(|v|reference(&v,"fileID")).as_deref()==Some(doc.file_id.as_str())) {
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
            }
            let lookup:HashMap<_,_>=child.docs.iter().enumerate().map(|(i,d)|(d.file_id.clone(),i)).collect();
            for modification in modifications(&doc.text) {
                let Some(target)=child.sources.get(&modification.target) else {self.issues.push(format!("PrefabInstance {}: override target not resolved",doc.file_id));continue;};
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
            let holder=self.id();
            let roots:Vec<_>=child.docs.iter().filter(|d|matches!(d.type_name.as_str(),"Transform"|"RectTransform") && d.parent_transform.as_deref().is_none_or(|id|id=="0")).map(|d|d.file_id.clone()).collect();
            for item in &mut child.docs {if roots.contains(&item.file_id) {
                let father=if item.type_name=="RectTransform" {ids.get(&parent).unwrap_or(&parent)} else {&holder};
                patch(item,"m_Father",&format!("{{fileID: {father}}}"));
            }}
            let holder_doc=Document{file_id:holder.clone(),type_name:"Transform".into(),start_line:0,start_offset:0,
                text:format!("Transform:\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Father: {{fileID: {}}}\n",ids.get(&parent).unwrap_or(&parent)).into(),game_object:None,parent_transform:Some(ids.get(&parent).unwrap_or(&parent).clone())};
            child.docs.push(holder_doc);
            for field in ["m_AddedComponents","m_AddedGameObjects"] {if doc.text.lines().any(|line|line.trim().starts_with(&format!("{field}:")) && !line.trim().ends_with("[]")) {self.issues.push(format!("PrefabInstance {}: {} uses serialized local documents; insertion order may be approximate",doc.file_id,field));}}
            self.issues.truncate(256);
            result.docs.extend(child.docs);
            result.ambiguous.extend(child.ambiguous);
            for source in &result.ambiguous {result.sources.remove(source);}
            for (source,id) in child.sources {
                if result.ambiguous.contains(&source) {continue;}
                if result.sources.get(&source).is_some_and(|old|*old!=id) {result.sources.remove(&source);result.ambiguous.insert(source);} else {result.sources.insert(source,id);}
            }
            result.picks.extend(child.picks);
        }
        for doc in docs {
            if !checkpoint(self.stop,Some(self.paused)) {return result;}
            if doc.type_name=="PrefabInstance" || doc.text.lines().next().is_some_and(|line|line.ends_with(" stripped")) {continue;}
            let mut copy=doc.clone();
            if guid.is_some() {remap(&mut copy,&ids);self.generated+=copy.text.len();if self.generated>256*1024*1024 || result.docs.len()>250_000 {self.issues.push("Prefab expansion exceeds preview budget".into());break;}}
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
fn reference_field(text:&str,key:&str) -> Option<String> {
    let mut lines=text.lines();
    while let Some(line)=lines.next() {
        let trimmed=line.trim();let value=trimmed.strip_prefix(key).and_then(|line|line.strip_prefix(':'));
        if let Some(value)=value {let mut value=value.trim().to_owned();while value.starts_with('{') && !value.contains('}') {value.push(' ');value.push_str(lines.next()?.trim());}return Some(value);}
    }
    None
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
    use super::*;
    const TEMPLATE:&str="--- !u!1 &10\nGameObject:\n  m_Name: source\n  m_IsActive: 1\n--- !u!4 &11\nTransform:\n  m_GameObject: {fileID: 10}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Father: {fileID: 0}\n--- !u!212 &12\nSpriteRenderer:\n  m_GameObject: {fileID: 10}\n  m_Enabled: 1\n  m_Sprite: {fileID: 21300000, guid: texture, type: 3}\n";
    fn instance(id:i64,guid:&str,x:i64) -> String {format!("--- !u!1001 &{id}\nPrefabInstance:\n  m_Modification:\n    m_TransformParent: {{fileID: 2}}\n    m_Modifications:\n    - target: {{fileID: 11, guid: {guid},\n        type: 3}}\n      propertyPath: m_LocalPosition.x\n      value: {x}\n      objectReference: {{fileID: 0}}\n  m_SourcePrefab: {{fileID: 100100000, guid: {guid}, type: 3}}\n")}
    fn expand(scene:&str,templates:&[(&str,&str)]) -> Level {
        let docs=crate::parse_unity_yaml(scene);let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);let mut resolver=Resolver::new(None,&stop,&paused,&docs);
        for (guid,text) in templates {resolver.templates.insert(guid.to_string(),crate::parse_unity_yaml(text));}
        resolver.level(&docs,None,None,0)
    }
    #[test]
    fn instances_isolate_ids_apply_transform_overrides_and_resolve_stripped_alias() {
        let scene=format!("--- !u!1 &1\nGameObject:\n  m_Name: parent\n--- !u!4 &2\nTransform:\n  m_GameObject: {{fileID: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Father: {{fileID: 0}}\n{}{}--- !u!4 &101 stripped\nTransform:\n  m_CorrespondingSourceObject: {{fileID: 11, guid: a, type: 3}}\n  m_PrefabInstance: {{fileID: 100}}\n",instance(100,"a",5),instance(200,"a",-5));
        let level=expand(&scene,&[("a",TEMPLATE)]);
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
        assert_eq!(rewrite_refs("{fileID: 12, guid: asset, type: 3}",&HashMap::from([("12".into(),"99".into())])),"{fileID: 12, guid: asset, type: 3}");
    }
    #[test]
    fn recursive_sources_are_bounded_and_cancelled_expansion_stops() {
        let docs=crate::parse_unity_yaml(&instance(1,"a",0));let stop=AtomicBool::new(false);let paused=AtomicBool::new(false);let mut resolver=Resolver::new(None,&stop,&paused,&docs);
        resolver.templates.insert("a".into(),docs.clone());resolver.level(&docs,None,None,0);assert!(resolver.issues.iter().any(|issue|issue.contains("recursive")));
        stop.store(true,std::sync::atomic::Ordering::Relaxed);assert!(resolver.level(&docs,None,None,0).docs.is_empty());
    }
}
