//! Conservative C# source inspection for a read-only Unity inspector.
//! This is not a C# compiler: unsupported or ambiguous declarations stay unknown.
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::{AtomicUsize, Ordering}, mpsc::{self, Receiver}},
};

use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit};
use regex::Regex;

#[derive(Default)]
pub struct ScriptProperties {
    projects: HashMap<PathBuf, ProjectState>,
    current_root: Option<PathBuf>,
}

enum ProjectState {
    Loading(ScanJob),
    Ready(ProjectIndex),
}
impl ProjectState {
    fn index(&self)->Option<&ProjectIndex> {match self {Self::Ready(index)=>Some(index),Self::Loading(job)=>job.cached.as_ref()}}
}

enum ScanResult { Cached(ProjectIndex), Assets(Arc<HashMap<String, PathBuf>>), Complete(ProjectIndex) }
struct ScanJob {
    receiver: Receiver<ScanResult>,
    assets: Option<Arc<HashMap<String, PathBuf>>>,
    cached: Option<ProjectIndex>,
    progress: Arc<ScanProgress>,
}
#[derive(Default)]
struct ScanProgress { resources: AtomicUsize, parsed: AtomicUsize, total: AtomicUsize }

#[derive(Default,serde::Serialize,serde::Deserialize)]
struct ProjectIndex {
    #[serde(skip)]
    cache_error:Option<String>,
    shader_properties: Arc<HashMap<String, HashMap<String, String>>>,
    assets: Arc<HashMap<String, PathBuf>>,
    scripts: HashMap<String, PathBuf>,
    classes: HashMap<String, Vec<ClassInfo>>,
    enums: HashMap<String, Vec<Vec<(String, i64)>>>,
    issues: usize,
}

#[derive(Clone, Debug,serde::Serialize,serde::Deserialize)]
struct ClassInfo {
    path: PathBuf,
    name: String,
    base: Option<String>,
    fields: BTreeMap<String, String>,
    ranges: BTreeMap<String, (f64, f64)>,
}

impl ScriptProperties {
    pub fn resolve_table_types(&self, text: &str, table: &mut crate::YamlTable) {
        let Some(index) = self.current_root.as_ref().and_then(|root| self.projects.get(root)).and_then(ProjectState::index) else { return };
        let Some(guid) = text.lines().find_map(|line| line.strip_prefix("  m_Script:")).and_then(guid_from_meta) else { return };
        let Some(class) = index.scripts.get(&guid).and_then(|path| index.script_class(path)) else { return };
        let fields = index.inherited_fields(class, &mut HashSet::new());
        if let Some(element) = fields.get(&table.title).and_then(|kind| collection_element_type(kind)) {
            assign_table_types(table, element, index);
        }
    }
    pub fn script_name(&self, text: &str) -> Option<&str> {
        let root = self.current_root.as_ref()?;
        let index = self.projects.get(root)?.index()?;
        let reference = text.lines().find_map(|line| line.strip_prefix("  m_Script:"))?;
        let guid = guid_from_meta(reference)?;
        let path = index.scripts.get(&guid)?;
        index.script_class(path).map(|class| class.name.as_str())
    }

    pub fn update(&mut self, path: Option<&Path>, ctx: &egui::Context) {
        self.current_root = path.and_then(project_root);
        ctx.data_mut(|data| data.remove::<Arc<HashMap<String, PathBuf>>>(egui::Id::new("asset_catalog")));
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("asset_catalog_loading"), false));
        ctx.data_mut(|data| data.remove::<Arc<HashMap<String, HashMap<String, String>>>>(egui::Id::new("shader_properties")));
        let Some(root) = self.current_root.clone() else { return };
        if !self.projects.contains_key(&root) {
            self.start_scan(root.clone(), ctx.clone());
        }
        let mut completed = None;
        if let Some(ProjectState::Loading(job)) = self.projects.get_mut(&root) {
            loop {
                match job.receiver.try_recv() {
                    Ok(ScanResult::Cached(index)) => {job.assets=Some(index.assets.clone());job.cached=Some(index);ctx.request_repaint();}
                    Ok(ScanResult::Assets(assets)) => { if job.cached.is_none() {job.assets = Some(assets);} ctx.request_repaint(); }
                    Ok(ScanResult::Complete(index)) => { completed = Some(index); break; }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        completed = Some(job.cached.take().unwrap_or_else(||ProjectIndex { assets: job.assets.clone().unwrap_or_default(), issues: 1, ..Default::default() }));
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
            if let Some(assets) = &job.assets {
                ctx.data_mut(|data| data.insert_temp(egui::Id::new("asset_catalog"), assets.clone()));
                if let Some(index)=&job.cached {ctx.data_mut(|data|data.insert_temp(egui::Id::new("shader_properties"),index.shader_properties.clone()));}
            } else {
                ctx.data_mut(|data| {
                    data.insert_temp(egui::Id::new("asset_catalog_loading"), true);
                    data.insert_temp(egui::Id::new("asset_catalog_progress"), job.progress.resources.load(Ordering::Relaxed));
                });
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(index) = completed {
            self.projects.insert(root.clone(), ProjectState::Ready(index));
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("asset_catalog_loading"), false));
            ctx.request_repaint();
        }
        if let Some(ProjectState::Ready(index)) = self.projects.get(&root) {
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("asset_catalog"), index.assets.clone()));
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("shader_properties"), index.shader_properties.clone()));
        }
    }

    fn start_scan(&mut self, root: PathBuf, ctx: egui::Context) {
        let (sender, receiver) = mpsc::channel();
        let progress = Arc::new(ScanProgress::default());
        let cached=self.projects.remove(&root).and_then(|state|match state {ProjectState::Ready(index)=>Some(index),ProjectState::Loading(job)=>job.cached});
        let assets=cached.as_ref().map(|index|index.assets.clone());
        let load_disk=cached.is_none();
        self.projects.insert(root.clone(), ProjectState::Loading(ScanJob { receiver, assets, cached, progress: progress.clone() }));
        std::thread::spawn(move || {
            let cache_path=index_cache_path(&root);
            if load_disk {if let Some(index)=cache_path.as_ref().and_then(|path|read_index_cache(path,&root)) {let _=sender.send(ScanResult::Cached(index));ctx.request_repaint();}}
            let mut index = scan_project(&root, &progress, |assets| {
                let _ = sender.send(ScanResult::Assets(assets));
                ctx.request_repaint();
            });
            if let Some(path)=cache_path {if let Err(error)=write_index_cache(&path,&root,&index) {index.cache_error=Some(error);}}
            let _ = sender.send(ScanResult::Complete(index));
            ctx.request_repaint();
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, text: &str, query: &str) {
        let root = self.current_root.clone();
        if let Some(root) = &root {
            ui.horizontal(|ui| {
                ui.small(crate::lf!("Unity 工程：{0}", root.display()));
                if ui.add_enabled(!matches!(self.projects.get(root),Some(ProjectState::Loading(_))),egui::Button::new(crate::localization::tr("重新扫描项目索引"))).clicked() {
                    self.start_scan(root.clone(), ui.ctx().clone());
                }
            });
        }
        let fields = yaml_fields(text);
        let script_guid = fields.iter().find(|field| field.name == "m_Script")
            .and_then(|field| guid_from_meta(&field.value));
        let index = root.as_ref().and_then(|root| self.projects.get(root));
        if let Some(error)=index.and_then(ProjectState::index).and_then(|index|index.cache_error.as_ref()) {
            ui.small(crate::localization::tr("索引缓存无法写入，下次启动将重新扫描。")).on_hover_text(error);
        }
        if let Some(ProjectState::Loading(job)) = index {
            if job.assets.is_some() {
                let done = job.progress.parsed.load(Ordering::Relaxed);
                let total = job.progress.total.load(Ordering::Relaxed);
                ui.small(format!("{} {done}/{total}", crate::localization::tr("正在解析脚本与 Shader…")));
                if total > 0 {ui.add(egui::ProgressBar::new(done as f32 / total as f32).desired_width(ui.available_width()));}
            }
        }
        let resolved = match index.and_then(ProjectState::index) {
            Some(index) => script_guid.as_ref()
                .and_then(|guid| index.scripts.get(guid))
                .and_then(|path| index.script_class(path))
                .map(|class| (index, class)),
            _ => None,
        };
        if let Some((index, class)) = resolved {
            ui.label(RichText::new(&class.name).strong());
            ui.small(crate::lf!("脚本：{0}", class.path.display()));
            if index.issues > 0 {
                ui.small(crate::lf!("扫描中有 {0} 项无法读取或存在重复 GUID；相关字段可能无法确定类型。", index.issues));
            }
        } else {
            ui.label(match index {
                Some(ProjectState::Loading(_)) => crate::localization::tr("正在建立脚本 GUID 索引…暂时显示原始属性。"),
                Some(ProjectState::Ready(_)) => crate::localization::tr("未找到对应脚本或无法确定脚本类；字段类型未知。"),
                None => crate::localization::tr("未找到 Unity 工程根目录（Assets / ProjectSettings）；字段类型未知。"),
            });
        }
        ui.small(crate::localization::tr("类型来自脚本声明；编辑模式支持已确认类型的属性。复杂原始结构保持只读。"));
        let known_fields = resolved.map(|(index, class)| index.inherited_fields(class, &mut HashSet::new()));
        let ranges = resolved.map(|(index, class)| index.inherited_ranges(class, &mut HashSet::new()));
        let search = query.trim().to_lowercase();
        property_scroll(ui, "script_properties", |ui| {
            publish_property_columns(ui);
            ui.collapsing(crate::localization::tr("组件信息"), |ui| {
                for field in fields.iter().filter(|field| is_component_field(&field.name)) {
                    raw_field(ui, field, crate::localization::tr("组件信息"));
                }
            });
            ui.separator();
            for field in fields.iter().filter(|field| !is_component_field(&field.name)) {
                let field_type = known_fields.as_ref().and_then(|fields| fields.get(&field.name));
                if !search.is_empty() && !format!("{} {} {}", field.name, field.value, field_type.map(String::as_str).unwrap_or(""))
                    .to_lowercase().contains(&search) {
                    continue;
                }
                ui.push_id(&field.name, |ui| {
                    if let (Some(field_type), Some((index, _))) = (field_type, resolved) {
                        typed_field_range(ui, field, field_type, index, ranges.as_ref().and_then(|ranges| ranges.get(&field.name)).copied());
                    } else {
                        raw_field(ui, field, crate::localization::tr("类型未知"));
                    }
                });
            }
        });
    }
}

// Component sections share the enclosing Inspector scroll area.
fn property_scroll(ui: &mut egui::Ui, id: &str, contents: impl FnOnce(&mut egui::Ui)) {
    if ui.ctx().data(|data| data.get_temp::<bool>(egui::Id::new("inspector_component_sections")).unwrap_or(false)) {
        contents(ui);
    } else {
        ScrollArea::both().id_salt(id).auto_shrink([false, false]).show(ui, contents);
    }
}

fn project_root(path: &Path) -> Option<PathBuf> {
    path.parent()?.ancestors()
        .find(|dir| dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir())
        .map(Path::to_path_buf)
}

const INDEX_CACHE_SCHEMA:u32=1;
#[derive(serde::Serialize,serde::Deserialize)]
struct CachedProjectIndex {schema:u32,app_version:String,root:String,index:ProjectIndex}
#[derive(serde::Serialize)]
struct CachedProjectIndexRef<'a> {schema:u32,app_version:&'a str,root:String,index:&'a ProjectIndex}
fn root_identity(root:&Path)->String {
    let text=root.to_string_lossy().replace('\\',"/");
    if cfg!(windows) {text.to_lowercase()}else{text}
}
fn index_cache_path(root:&Path)->Option<PathBuf> {
    use sha2::{Digest,Sha256};
    let directory=crate::last_open_dir_config_path()?.parent()?.join("project_indexes");
    Some(directory.join(format!("{:x}.json",Sha256::digest(root_identity(root).as_bytes()))))
}
fn read_index_cache(path:&Path,root:&Path)->Option<ProjectIndex> {
    if fs::metadata(path).ok()?.len()>256*1024*1024 {return None;}
    let cache:CachedProjectIndex=serde_json::from_reader(std::io::BufReader::new(fs::File::open(path).ok()?)).ok()?;
    if cache.schema!=INDEX_CACHE_SCHEMA || cache.app_version!=env!("CARGO_PKG_VERSION") || cache.root!=root_identity(root) {return None;}
    let prefix=format!("{}/",root_identity(root).trim_end_matches('/'));
    let valid=|path:&Path|root_identity(path).starts_with(&prefix) && !path.components().any(|p|matches!(p,std::path::Component::ParentDir));
    if !cache.index.assets.values().chain(cache.index.scripts.values()).all(|path|valid(path)) || !cache.index.classes.values().flatten().all(|class|valid(&class.path)) {return None;}
    Some(cache.index)
}
fn write_index_cache(path:&Path,root:&Path,index:&ProjectIndex)->Result<(),String> {
    let _cache_guard = crate::app_cache::mutation_guard();
    // An incomplete scan must not replace a usable cache.
    if index.issues>0 && path.exists() {return Ok(());}
    fs::create_dir_all(path.parent().ok_or("Missing cache directory")?).map_err(|e|e.to_string())?;
    let temporary=path.with_extension(format!("{}.tmp",std::process::id()));
    let result=(|| {
        use std::io::Write;
        let mut writer=std::io::BufWriter::new(fs::File::create(&temporary).map_err(|e|e.to_string())?);
        serde_json::to_writer(&mut writer,&CachedProjectIndexRef{schema:INDEX_CACHE_SCHEMA,app_version:env!("CARGO_PKG_VERSION"),root:root_identity(root),index}).map_err(|e|e.to_string())?;
        writer.flush().map_err(|e|e.to_string())?;
        writer.get_ref().sync_all().map_err(|e|e.to_string())?;
        drop(writer);
        fs::rename(&temporary,path).map_err(|e|e.to_string())
    })();
    if result.is_err() {let _=fs::remove_file(&temporary);}
    result
}

fn guid_from_meta(text: &str) -> Option<String> {
    static GUID_PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    GUID_PATTERN.get_or_init(|| Regex::new(r"\bguid:\s*([0-9a-fA-F]{32})\b").unwrap())
        .captures(text).map(|capture| capture[1].to_ascii_lowercase())
}

fn scan_project(root: &Path, progress: &ScanProgress, publish_assets: impl FnOnce(Arc<HashMap<String, PathBuf>>)) -> ProjectIndex {
    let mut index = ProjectIndex::default();
    let mut stack = vec![root.join("Assets"), root.join("Packages"), root.join("Library/PackageCache")];
    let mut visited = HashSet::new();
    let mut duplicate_guids = HashSet::new();
    while let Some(directory) = stack.pop() {
        if !directory.exists() { continue; }
        let Ok(canonical) = directory.canonicalize() else { index.issues += 1; continue };
        if !visited.insert(canonical) { continue; }
        let Ok(entries) = fs::read_dir(&directory) else { index.issues += 1; continue };
        for entry in entries {
            let Ok(entry) = entry else { index.issues += 1; continue };
            let path = entry.path();
            // Do not traverse junctions or symbolic links into unrelated trees.
            let Ok(kind) = entry.file_type() else { index.issues += 1; continue };
            if kind.is_dir() { stack.push(path); continue; }
            if !kind.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("meta") { continue; }
            let Ok(meta) = fs::read_to_string(&path) else { index.issues += 1; continue };
            let Some(guid) = guid_from_meta(&meta) else { continue };
            let script_path = path.with_extension("");
            if !script_path.is_file() { continue; }
            if duplicate_guids.contains(&guid) { continue; }
            if index.assets.contains_key(&guid) {
                Arc::make_mut(&mut index.assets).remove(&guid);
                index.scripts.remove(&guid);
                Arc::make_mut(&mut index.shader_properties).remove(&guid);
                duplicate_guids.insert(guid);
                index.issues += 1;
                continue;
            }
            Arc::make_mut(&mut index.assets).insert(guid.clone(), script_path.clone());
            progress.resources.store(index.assets.len(), Ordering::Relaxed);
        }
    }
    // Freeze the complete GUID catalog before parsing source files. Both phases
    // share this exact Arc so preview cache keys stay stable when scripts finish.
    let sources: Vec<_> = index.assets.iter().filter(|(_, path)|
        matches!(path.extension().and_then(|ext| ext.to_str()), Some("cs" | "shader")))
        .map(|(guid,path)|(guid.clone(),path.clone())).collect();
    progress.total.store(sources.len(), Ordering::Relaxed);
    publish_assets(index.assets.clone());
    for (position, (guid, script_path)) in sources.into_iter().enumerate() {
        if let Ok(source) = fs::read_to_string(&script_path) {
            if script_path.extension().and_then(|ext| ext.to_str()) == Some("shader") {
                Arc::make_mut(&mut index.shader_properties).insert(guid, parse_shader_properties(&source));
            } else {
                index.scripts.insert(guid, script_path.clone());
                let (classes, enums) = parse_source(&source, &script_path);
                for class in classes { index.classes.entry(class.name.clone()).or_default().push(class); }
                for (name, values) in enums { index.enums.entry(name).or_default().push(values); }
            }
        } else { index.issues += 1; }
        progress.parsed.store(position + 1, Ordering::Relaxed);
    }
    index
}

impl ProjectIndex {
    fn inherited_ranges(&self, class: &ClassInfo, visited: &mut HashSet<(PathBuf, String)>) -> BTreeMap<String, (f64, f64)> {
        if !visited.insert((class.path.clone(), class.name.clone())) { return BTreeMap::new(); }
        let mut ranges = class.base.as_ref().and_then(|base| self.classes.get(short_type(base)))
            .filter(|classes| classes.len() == 1)
            .map(|classes| self.inherited_ranges(&classes[0], visited)).unwrap_or_default();
        for name in class.fields.keys() { ranges.remove(name); }
        ranges.extend(class.ranges.clone());
        ranges
    }
    fn script_class(&self, path: &Path) -> Option<&ClassInfo> {
        let stem = path.file_stem()?.to_str()?;
        let candidates: Vec<_> = self.classes.get(stem)?.iter().filter(|class| class.path == path).collect();
        if candidates.len() == 1 { Some(candidates[0]) } else { None }
    }

    fn inherited_fields(&self, class: &ClassInfo, visited: &mut HashSet<(PathBuf, String)>) -> BTreeMap<String, String> {
        if !visited.insert((class.path.clone(), class.name.clone())) { return BTreeMap::new(); }
        let mut fields = class.base.as_ref()
            .and_then(|base| self.classes.get(short_type(base)))
            .filter(|classes| classes.len() == 1)
            .map(|classes| self.inherited_fields(&classes[0], visited))
            .unwrap_or_default();
        fields.extend(class.fields.clone());
        fields
    }
}

// Strings and comments are tokenized separately so braces in them cannot be
// mistaken for class/method scopes. UNITY_EDITOR is evaluated in editor context;
// other conditional symbols remain unknown and their branches are omitted.
fn parse_source(source: &str, path: &Path) -> (Vec<ClassInfo>, Vec<(String, Vec<(String, i64)>)>) {
    let token_re = Regex::new(r#"(?s)//[^\r\n]*|/\*.*?\*/|@"(?:""|[^"])*"|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|[A-Za-z_][A-Za-z_0-9]*|-?\d+|[^\s]"#).unwrap();
    let mut conditions: Vec<(bool, Option<bool>)> = Vec::new();
    let mut active = true;
    let source: String = source.lines().filter_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let directive = trimmed.trim_start_matches('#').trim_start().split_whitespace().next().unwrap_or("");
            match directive {
                "if" => {
                    let expression=trimmed.trim_start_matches('#').trim_start().strip_prefix("if").unwrap_or("").trim();
                    let known=match expression {"UNITY_EDITOR"=>Some(true),"!UNITY_EDITOR"=>Some(false),_=>None};
                    conditions.push((active,known));active=active && known==Some(true);
                },
                "else" => {if let Some((parent,known))=conditions.last() {active=*parent && *known==Some(false);}},
                "elif" => {if let Some((_,known))=conditions.last_mut() {*known=None;} active=false;},
                "endif" => {if let Some((parent,_))=conditions.pop() {active=parent;}},
                _ => {}
            }
            None
        } else if active {
            Some(format!("{line}\n"))
        } else { None }
    }).collect();
    let tokens: Vec<&str> = token_re.find_iter(&source).map(|token| token.as_str())
        .filter(|token| !token.starts_with("//") && !token.starts_with("/*")).collect();
    let mut classes = Vec::new();
    let mut enums = Vec::new();
    let mut scopes: Vec<(usize, usize, String)> = Vec::new();
    for i in 0..tokens.len() {
        if !matches!(tokens[i], "class" | "struct" | "enum") { continue; }
        let Some(name) = tokens.get(i + 1) else { continue };
        let Some(relative_open) = tokens[i + 2..].iter().position(|token| matches!(*token, "{" | ";")) else { continue };
        let open = i + 2 + relative_open;
        if tokens[open] != "{" { continue; }
        let Some(close) = matching_brace(&tokens, open) else { continue };
        if tokens[i] == "enum" {
            if let Some(values) = enum_values(&tokens[open + 1..close]) {
                let owners: Vec<_> = scopes.iter().filter(|(start, end, _)| *start < i && i < *end)
                    .map(|(_, _, name)| name.as_str()).collect();
                let qualified = if owners.is_empty() { name.to_string() } else { format!("{}.{}", owners.join("."), name) };
                enums.push((qualified, values));
            }
        } else {
            scopes.push((open, close, name.to_string()));
            // Generic classes and partial classes need compiler context.
            if tokens[i + 2..open].contains(&"<") || (i > 0 && tokens[i - 1] == "partial") { continue; }
            let base = tokens[i + 2..open].iter().position(|token| *token == ":")
                .map(|colon| tokens[i + 3 + colon..open].iter().take_while(|token| **token != ",").copied().collect::<String>());
            let (fields, ranges) = class_fields(&tokens[open + 1..close]);
            classes.push(ClassInfo {
                path: path.to_path_buf(), name: name.to_string(), base,
                fields, ranges,
            });
        }
    }
    for class in &mut classes {
        for field_type in class.fields.values_mut() {
            let qualified = format!("{}.{}", class.name, field_type);
            if enums.iter().any(|(name, _)| name == &qualified) { *field_type = qualified; }
        }
    }
    (classes, enums)
}

fn matching_brace(tokens: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate().skip(open) {
        match *token {
            "{" => depth += 1,
            "}" => { depth = depth.checked_sub(1)?; if depth == 0 { return Some(i); } }
            _ => {}
        }
    }
    None
}

fn class_fields(tokens: &[&str]) -> (BTreeMap<String, String>, BTreeMap<String, (f64, f64)>) {
    let mut fields = BTreeMap::new();
    let mut ranges = BTreeMap::new();
    let mut start = 0;
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "{" => {
                let Some(end) = matching_brace(tokens, i) else { break };
                // Field initializers may contain object/array initializer braces.
                // Method bodies, properties and nested classes are not fields.
                if !tokens[start..i].contains(&"=") { start = end + 1; }
                i = end;
            }
            ";" => {
                if let Some((name, field_type, aliases)) = field_declaration(&tokens[start..i]) {
                    if matches!(short_type(&field_type), "float" | "Single" | "int" | "Int32") {
                        if let Some(range) = range_attribute(&tokens[start..i]) {
                            ranges.insert(name.clone(), range);
                            for alias in &aliases { ranges.insert(alias.clone(), range); }
                        }
                    }
                    fields.insert(name, field_type.clone());
                    for alias in aliases { fields.insert(alias, field_type.clone()); }
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    (fields, ranges)
}

fn range_attribute(tokens: &[&str]) -> Option<(f64, f64)> {
    let mut offset = 0;
    let mut found = None;
    while tokens.get(offset) == Some(&"[") {
        let end = offset + tokens[offset..].iter().position(|token| *token == "]")?;
        let attr = &tokens[offset + 1..end];
        for (i, token) in attr.iter().enumerate() {
            if !matches!(*token, "Range" | "RangeAttribute") || attr.get(i + 1) != Some(&"(") { continue; }
            if found.is_some() { return None; }
            let close = i + 2 + attr[i + 2..].iter().position(|token| *token == ")")?;
            let args = &attr[i + 2..close];
            let comma = args.iter().position(|token| *token == ",")?;
            let number = |parts: &[&str]| {
                let text = parts.concat();
                let text = text.trim_end_matches(['f', 'F', 'd', 'D']);
                text.parse::<f64>().ok().filter(|value| value.is_finite())
            };
            let (min, max) = (number(&args[..comma])?, number(&args[comma + 1..])?);
            if min > max || min.abs() > f32::MAX as f64 || max.abs() > f32::MAX as f64 { return None; }
            found = Some((min, max));
        }
        offset = end + 1;
    }
    found
}

fn field_declaration(tokens: &[&str]) -> Option<(String, String, Vec<String>)> {
    let mut offset = 0;
    let mut serialized = false;
    let mut aliases = Vec::new();
    while tokens.get(offset) == Some(&"[") {
        let end = offset + tokens[offset..].iter().position(|token| *token == "]")?;
        let attr = &tokens[offset + 1..end];
        if attr.iter().any(|token| matches!(*token, "NonSerialized" | "NonSerializedAttribute")) { return None; }
        serialized |= attr.iter().any(|token| matches!(*token, "SerializeField" | "SerializeFieldAttribute" | "SerializeReference"));
        if let Some(i) = attr.iter().position(|token| *token == "FormerlySerializedAs") {
            if let Some(value) = attr.get(i + 2).and_then(|value| value.strip_prefix('"')).and_then(|value| value.strip_suffix('"')) {
                aliases.push(value.to_string());
            }
        }
        offset = end + 1;
    }
    let declaration = &tokens[offset..];
    let end = declaration.iter().position(|token| *token == "=").unwrap_or(declaration.len());
    let declaration = &declaration[..end];
    if declaration.iter().any(|token| matches!(*token, "static" | "const" | "readonly" | "(" | ")" | "=>")) { return None; }
    if !serialized && !declaration.contains(&"public") { return None; }
    let type_start = declaration.iter().position(|token| !matches!(*token, "public" | "private" | "protected" | "internal" | "new" | "volatile"))?;
    let name = declaration.last()?;
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || declaration.len() <= type_start + 1 { return None; }
    let field_type = declaration[type_start..declaration.len() - 1].concat();
    if field_type.contains(',') { return None; }
    Some((name.to_string(), field_type, aliases))
}

fn enum_values(tokens: &[&str]) -> Option<Vec<(String, i64)>> {
    let mut values = Vec::new();
    let mut next = 0i64;
    for entry in tokens.split(|token| *token == ",").filter(|entry| !entry.is_empty()) {
        let name = entry[0];
        let value = match entry {
            [_] => next,
            [_, "=", number] => number.parse::<i64>().ok()?,
            _ => return None,
        };
        values.push((name.to_string(), value));
        next = value.checked_add(1)?;
    }
    Some(values)
}

fn short_type(field_type: &str) -> &str { field_type.rsplit('.').next().unwrap_or(field_type) }

struct YamlField { name: String, value: String }

fn yaml_fields(text: &str) -> Vec<YamlField> {
    let mut fields: Vec<YamlField> = Vec::new();
    for line in text.lines() {
        if let Some(direct) = line.strip_prefix("  ").filter(|value| !value.starts_with([' ', '-'])) {
            if let Some((name, value)) = direct.split_once(':') {
                fields.push(YamlField { name: name.to_string(), value: value.trim().to_string() });
                continue;
            }
        }
        if let Some(field) = fields.last_mut() {
            if !field.value.is_empty() { field.value.push('\n'); }
            field.value.push_str(line);
        }
    }
    fields
}

fn is_component_field(name: &str) -> bool {
    matches!(name, "m_ObjectHideFlags" | "m_CorrespondingSourceObject" | "m_PrefabInstance" | "m_PrefabAsset" | "m_GameObject" | "m_Enabled" | "m_EditorHideFlags" | "m_Script" | "m_Name" | "m_EditorClassIdentifier")
}

pub fn show_sprite_renderer_properties_content(ui: &mut egui::Ui, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let search = query.trim().to_lowercase();
    let mut index = ProjectIndex::default();
    for (name, entries) in [
        ("SpriteDrawMode", vec![("Simple", 0), ("Sliced", 1), ("Tiled", 2)]),
        ("SpriteTileMode", vec![("Continuous", 0), ("Adaptive", 1)]),
        ("SpriteMaskInteraction", vec![("None", 0), ("VisibleInsideMask", 1), ("VisibleOutsideMask", 2)]),
        ("SpriteSortPoint", vec![("Center", 0), ("Pivot", 1)]),
        ("ShadowCastingMode", vec![("Off", 0), ("On", 1), ("TwoSided", 2), ("ShadowsOnly", 3)]),
    ] {
        index.enums.insert(name.to_string(), vec![entries.into_iter().map(|(name, value)| (name.to_string(), value)).collect()]);
    }
    ui.small(crate::localization::tr("SpriteRenderer 内置字段，无需查找脚本。"));
            publish_property_columns(ui);
        ui.collapsing(crate::localization::tr("组件信息"), |ui| {
            for field in fields.iter().filter(|field| sprite_metadata(&field.name)) {
                raw_field(ui, field, crate::localization::tr("组件信息"));
            }
        });
        ui.separator();
        // Put the properties commonly used for sprite appearance first,
        // regardless of the order of renderer internals in serialized YAML.
        let primary = ["m_Enabled", "m_Sprite", "m_Color", "m_FlipX", "m_FlipY", "m_DrawMode",
            "m_Size", "m_SpriteTileMode", "m_AdaptiveModeThreshold", "m_MaskInteraction",
            "m_SpriteSortPoint", "m_SortingLayerID", "m_SortingLayer", "m_SortingOrder", "m_Materials"];
        let show_field = |ui: &mut egui::Ui, field: &YamlField| {
            let (label, kind) = sprite_field_info(&field.name);
            if !search.is_empty() && !format!("{} {label} {kind} {}", field.name, field.value).to_lowercase().contains(&search) { return; }
            ui.push_id(&field.name, |ui| {
                let display = YamlField { name: label.to_string(), value: field.value.clone() };
                if kind == crate::localization::tr("原始值") { raw_field(ui, &display, kind); }
                else { typed_field(ui, &display, kind, &index); }
            });
        };
        for name in &primary {
            if let Some(field) = fields.iter().find(|field| field.name == *name) { show_field(ui, field); }
        }
        egui::CollapsingHeader::new(crate::localization::tr("其他渲染设置与字段")).default_open(!search.is_empty()).show(ui, |ui| {
            for field in fields.iter().filter(|field| !sprite_metadata(&field.name) && !primary.contains(&field.name.as_str())) {
                show_field(ui, field);
            }
        });
}

fn sprite_metadata(name: &str) -> bool {
    is_component_field(name) && name != "m_Enabled"
}

pub fn show_sprite_atlas_properties(ui: &mut egui::Ui, text: &str, query: &str) {
    let index = ProjectIndex::default();
    ScrollArea::both().id_salt("sprite_atlas_properties").auto_shrink([false,false]).show(ui, |ui| {
        publish_property_columns(ui);
        atlas_fields(ui, &yaml_fields(text), &index, &query.trim().to_lowercase(), 0);
    });
}

fn atlas_fields(ui: &mut egui::Ui, fields: &[YamlField], index: &ProjectIndex, search: &str, depth: usize) {
    for field in fields {
        if !search.is_empty() && !format!("{} {}", field.name, field.value).to_lowercase().contains(search) { continue; }
        ui.push_id(&field.name, |ui| {
            if let Some(kind) = atlas_field_type(&field.name) {
                typed_field(ui, field, kind, index);
            } else if field.value.contains('\n') && depth < 12 && !field.value.lines().find(|line| !line.trim().is_empty()).is_some_and(|line| line.trim_start().starts_with('-')) {
                let nested = field.value.lines().map(|line| line.strip_prefix("  ").unwrap_or(line)).collect::<Vec<_>>().join("\n");
                let children = yaml_fields(&nested);
                if children.is_empty() { raw_field(ui, field, crate::localization::tr("原始值")); }
                else {
                    crate::editing::with_field(ui, &field.name, |ui| {
                        egui::CollapsingHeader::new(&field.name).default_open(true).show(ui, |ui| atlas_fields(ui, &children, index, search, depth + 1));
                    });
                }
            } else { raw_field(ui, field, crate::localization::tr("原始值")); }
        });
    }
}

fn atlas_field_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "m_Name" | "buildTarget" => "string",
        "packables" => "UnityEngine.Object[]",
        "m_PackedSprites" => "Sprite[]",
        "m_PackedSpriteNamesToIndex" => "string[]",
        "platformSettings" | "m_PlatformSettings" => "TextureImporterPlatformSettings[]",
        "m_MasterAtlas" => "SpriteAtlas",
        "enableRotation" | "enableTightPacking" | "bindAsDefault" | "isReadable" | "generateMipMaps" | "sRGB" | "enableAlphaDilation" |
        "overridden" | "crunchedCompression" | "allowsAlphaSplitting" | "isVariant" | "m_IsVariant" | "m_IncludeInBuild" => "bool",
        "variantMultiplier" => "float",
        "padding" | "blockOffset" | "maxTextureSize" | "textureCompression" | "compressionQuality" | "filterMode" | "anisoLevel" | "textureFormat" |
        "format" | "serializedVersion" | "m_ObjectHideFlags" | "resizeAlgorithm" | "androidETC2FallbackOverride" => "int",
        _ => return None,
    })
}

/// Serialized particle modules are inspected without running a simulation.
pub fn show_particle_properties_content(ui: &mut egui::Ui, type_name: &str, text: &str, query: &str) {
    let fields=yaml_fields(text);
    let mut index=ProjectIndex::default();
    for (name,values) in [
        ("ShadowCastingMode",vec![("Off",0),("On",1),("Two Sided",2),("Shadows Only",3)]),
        ("MotionVectorGenerationMode",vec![("Camera",0),("Object",1),("Force No Motion",2)]),
        ("LightProbeUsage",vec![("Off",0),("Blend Probes",1),("Use Proxy Volume",2),("Custom Provided",4)]),
        ("ReflectionProbeUsage",vec![("Off",0),("Blend Probes",1),("Blend Probes And Skybox",2),("Simple",3)]),
        ("RayTracingMode",vec![("Off",0),("Static",1),("Dynamic Transform",2),("Dynamic Geometry",3)]),
        ("ReceiveGI",vec![("Lightmaps",1),("Light Probes",2)]),
    ] {index.enums.insert(name.into(),vec![values.into_iter().map(|(label,value)|(label.into(),value)).collect()]);}
    publish_property_columns(ui);
    ui.collapsing(crate::localization::tr("组件信息"),|ui| {
        for field in fields.iter().filter(|field|sprite_metadata(&field.name)) {raw_field(ui,field,crate::localization::tr("组件信息"));}
    });
    let properties:Vec<_>=fields.into_iter().filter(|field|!sprite_metadata(&field.name)).collect();
    particle_fields(ui,&properties,type_name=="ParticleSystemRenderer",&index,&query.trim().to_lowercase(),0);
}

fn particle_field_kind(name:&str,renderer:bool,depth:usize) -> &str {
    if depth==0 && renderer {
        let (_,kind)=mesh_renderer_field_info(name);
        if kind!=crate::localization::tr("原始值") {return kind;}
        return match name {
            "m_RenderMode"|"m_SortMode"|"m_MeshDistribution"=>"int",
            "m_MinParticleSize"|"m_MaxParticleSize"|"m_CameraVelocityScale"|"m_VelocityScale"|"m_LengthScale"|"m_SortingFudge"|"m_NormalDirection"|"m_ShadowBias"=>"float",
            "m_AllowRoll"|"m_EnableGPUInstancing"|"m_FreeformStretching"|"m_RotateWithStretchDirection"=>"bool",
            "m_Pivot"|"m_Flip"=>"Vector3",
            "m_Mesh"|"m_Mesh1"|"m_Mesh2"|"m_Mesh3"=>"Mesh",
            "m_TrailMaterial"=>"Material",
            _=>crate::localization::tr("原始值"),
        };
    }
    match name {
        "looping"|"prewarm"|"playOnAwake"|"useUnscaledTime"|"autoRandomSeed"|"useRigidbodyForVelocity" if depth==0=>"bool",
        "enabled"=>"bool",
        "lengthInSec" if depth==0=>"float",
        "simulationSpeed" if depth==0=>"float",
        "stopAction"|"cullingMode"|"ringBufferMode"|"scalingMode"|"moveWithTransform" if depth==0=>"int",
        "randomSeed" if depth==0=>"uint",
        "ringBufferLoopRange" if depth==0=>"Vector2",
        _=>crate::localization::tr("原始值"),
    }
}

fn particle_fields(ui:&mut egui::Ui,fields:&[YamlField],renderer:bool,index:&ProjectIndex,search:&str,depth:usize) {
    for field in fields {
        if field.name=="serializedVersion" {continue;}
        if !search.is_empty() && !format!("{} {}",field.name,field.value).to_lowercase().contains(search) {continue;}
        ui.push_id(&field.name,|ui| {
            let kind=particle_field_kind(&field.name,renderer,depth);
            if kind!=crate::localization::tr("原始值") {typed_field(ui,field,kind,index);}
            else if depth<12 && field.value.lines().find(|line|!line.trim().is_empty()).is_some_and(|line|line.starts_with("    ") && !line.trim_start().starts_with('-')) {
                let nested=field.value.lines().map(|line|line.strip_prefix("  ").unwrap_or(line)).collect::<Vec<_>>().join("\n");
                let children=yaml_fields(&nested);
                if children.is_empty() {raw_field(ui,field,kind);} else {
                    crate::editing::with_field(ui,&field.name,|ui| {
                        egui::CollapsingHeader::new(&field.name).default_open(!search.is_empty()).show(ui,|ui|particle_fields(ui,&children,renderer,index,search,depth+1));
                    });
                }
            } else {raw_field(ui,field,kind);}
        });
    }
}

pub fn show_mesh_renderer_properties_content(ui: &mut egui::Ui, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let search = query.trim().to_lowercase();
    let mut index = ProjectIndex::default();
    for (name, values) in [
        ("ShadowCastingMode", vec![("Off",0),("On",1),("Two Sided",2),("Shadows Only",3)]),
        ("MotionVectorGenerationMode", vec![("Camera",0),("Object",1),("Force No Motion",2)]),
        ("LightProbeUsage", vec![("Off",0),("Blend Probes",1),("Use Proxy Volume",2),("Custom Provided",4)]),
        ("ReflectionProbeUsage", vec![("Off",0),("Blend Probes",1),("Blend Probes And Skybox",2),("Simple",3)]),
        ("RayTracingMode", vec![("Off",0),("Static",1),("Dynamic Transform",2),("Dynamic Geometry",3)]),
        ("ReceiveGI", vec![("Lightmaps",1),("Light Probes",2)]),
        ("SkinQuality", vec![("Auto",0),("Bone 1",1),("Bone 2",2),("Bone 4",4)]),
    ] {
        index.enums.insert(name.into(), vec![values.into_iter().map(|(label,value)| (label.into(),value)).collect()]);
    }
    publish_property_columns(ui);
    ui.collapsing(crate::localization::tr("组件信息"), |ui| {
        for field in fields.iter().filter(|field| sprite_metadata(&field.name)) {
            raw_field(ui, field, crate::localization::tr("组件信息"));
        }
    });
    for field in fields.iter().filter(|field| !sprite_metadata(&field.name)) {
        let (label, kind) = mesh_renderer_field_info(&field.name);
        if !search.is_empty() && !format!("{} {label} {kind} {}",field.name,field.value).to_lowercase().contains(&search) { continue; }
        ui.push_id(&field.name, |ui| {
            let display = YamlField { name: label.into(), value: field.value.clone() };
            if kind == crate::localization::tr("原始值") { raw_field(ui, &display, kind); }
            else { typed_field(ui, &display, kind, &index); }
        });
    }
}

fn mesh_renderer_field_info(name: &str) -> (&str, &str) {
    match name {
        "m_Quality" => ("Skin Quality", "SkinQuality"),
        "m_UpdateWhenOffscreen" => ("Update When Offscreen", "bool"),
        "m_SkinnedMotionVectors" => ("Skinned Motion Vectors", "bool"),
        "m_Mesh" => ("Mesh", "Mesh"),
        "m_Bones" => ("Bones", "Transform[]"),
        "m_RootBone" => ("Root Bone", "Transform"),
        "m_BlendShapeWeights" => ("Blend Shape Weights", "float[]"),
        "m_AABB" => ("Local Bounds", "SkinnedMeshBounds"),
        "m_DirtyAABB" => ("Dirty Bounds", "bool"),
        "m_MotionVectors" => ("Motion Vectors", "MotionVectorGenerationMode"),
        "m_LightProbeUsage" => ("Light Probes", "LightProbeUsage"),
        "m_ReflectionProbeUsage" => ("Reflection Probes", "ReflectionProbeUsage"),
        "m_RayTracingMode" => ("Ray Tracing Mode", "RayTracingMode"),
        "m_ReceiveGI" => ("Receive GI", "ReceiveGI"),
        "m_StaticBatchRoot" => ("Static Batch Root", "Transform"),
        "m_LightmapParameters" => ("Lightmap Parameters", "LightmapParameters"),
        "m_PreserveUVs" => ("Preserve UVs", "bool"),
        "m_IgnoreNormalsForChartDetection" => ("Ignore Normals For Chart Detection", "bool"),
        "m_StitchLightmapSeams" => ("Stitch Lightmap Seams", "bool"),
        "m_ImportantGI" => ("Important GI", "bool"),
        "m_MinimumChartSize" => ("Minimum Chart Size", "int"),
        "m_SelectedEditorRenderState" => ("Selected Editor Render State", "int"),
        "m_AutoUVMaxDistance" => ("Auto UV Max Distance", "float"),
        "m_AutoUVMaxAngle" => ("Auto UV Max Angle", "float"),
        _ => sprite_field_info(name),
    }
}

fn material_groups(text: &str) -> Vec<(String, Vec<YamlField>)> {
    let mut groups: Vec<(String, Vec<YamlField>)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if matches!(trimmed.split_once(':').map(|(name, _)| name), Some("m_TexEnvs" | "m_Ints" | "m_Floats" | "m_Colors")) {
            groups.push((trimmed.split_once(':').unwrap().0.into(), Vec::new()));
            continue;
        }
        let Some((_, entries)) = groups.last_mut() else { continue };
        if let Some(entry) = trimmed.strip_prefix("- ") {
            if let Some((name, value)) = entry.split_once(':') {
                entries.push(YamlField { name: name.to_string(), value: value.trim().to_string() });
            } else if let Some(entry) = entries.last_mut() {
                entry.value.push('\n'); entry.value.push_str(line);
            }
        } else if let Some(entry) = entries.last_mut() {
            if !entry.value.is_empty() { entry.value.push('\n'); }
            entry.value.push_str(line);
        }
    }
    groups
}

pub fn show_material_properties(ui: &mut egui::Ui, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let index = ProjectIndex::default();
    let search = query.trim().to_lowercase();
    let matches = |field: &YamlField| search.is_empty() || format!("{} {}", field.name, field.value).to_lowercase().contains(&search);
    let shader_types = ui.ctx().data_mut(|data| data.get_temp::<Arc<HashMap<String, HashMap<String, String>>>>(egui::Id::new("shader_properties")));
    let shader_guid = fields.iter().find(|field| field.name == "m_Shader").and_then(|field| guid_from_meta(&field.value));
    let property_types = shader_guid.as_ref().and_then(|guid| shader_types.as_ref()?.get(guid));
    ui.small(crate::localization::tr("按 Shader 声明区分 Color 和 Vector4；无法确认类型时以只读四分量显示。"));
    ScrollArea::both().id_salt("material_properties").auto_shrink([false, false]).show(ui, |ui| {
            publish_property_columns(ui);
        ui.collapsing(crate::localization::tr("资源信息"), |ui| {
            for field in fields.iter().filter(|field| is_component_field(&field.name)) { raw_field(ui, field, crate::localization::tr("资源信息")); }
        });
        for field in fields.iter().filter(|field| matches!(field.name.as_str(), "m_Shader" | "m_Parent" | "m_CustomRenderQueue" | "m_EnableInstancingVariants" | "m_DoubleSidedGI") && matches(field)) {
            let (label, kind) = match field.name.as_str() {
                "m_Shader" => ("Shader", crate::localization::tr("Shader 引用")),
                "m_Parent" => ("Parent", crate::localization::tr("Material 引用")),
                "m_CustomRenderQueue" => ("Render Queue", "int"),
                "m_EnableInstancingVariants" => ("Enable GPU Instancing", "bool"),
                _ => ("Double Sided GI", "bool"),
            };
            ui.push_id(&field.name, |ui| typed_field(ui, &YamlField { name: label.into(), value: field.value.clone() }, kind, &index));
        }
        if let Some(saved) = fields.iter().find(|field| field.name == "m_SavedProperties") {
            for (group, entries) in material_groups(&saved.value) {
                let label = match group.as_str() { "m_TexEnvs" => crate::localization::tr("纹理"), "m_Ints" => crate::localization::tr("整数"), "m_Floats" => crate::localization::tr("浮点数"), _ => crate::localization::tr("颜色 / Vector4") };
                egui::CollapsingHeader::new(format!("{label} ({})", entries.len())).id_salt(&group).default_open(true).show(ui, |ui| {
                    for entry in entries.iter().filter(|entry| matches(entry)) {
                        ui.push_id(&entry.name, |ui| {
                            if group == "m_TexEnvs" {
                                egui::CollapsingHeader::new(&entry.name).default_open(true).show(ui, |ui| {
                                    let mut texture_fields = Vec::new();
                                    let mut recognized = true;
                                    for line in entry.value.lines().filter(|line| !line.trim().is_empty()) {
                                        let Some((name, value)) = line.trim().split_once(':') else { recognized = false; break };
                                        let (label, kind) = match name {
                                            "m_Texture" => ("Texture", crate::localization::tr("Texture 引用")),
                                            "m_Scale" => ("Tiling", "Vector2"),
                                            "m_Offset" => ("Offset", "Vector2"),
                                            _ => { recognized = false; break; }
                                        };
                                        texture_fields.push((label, kind, value.trim()));
                                    }
                                    if recognized && !texture_fields.is_empty() {
                                        for (label, kind, value) in texture_fields {
                                            ui.push_id(label, |ui| crate::editing::with_field(ui, &entry.name, |ui| typed_field(ui, &YamlField { name: label.into(), value: value.into() }, kind, &index)));
                                        }
                                    } else { raw_field(ui, entry, crate::localization::tr("未识别的纹理结构")); }
                                });
                            } else {
                                if group == "m_Colors" {
                                    match property_types.and_then(|types| types.get(&entry.name)).map(String::as_str) {
                                        Some("Color") => typed_field(ui, entry, "Color", &index),
                                        Some("Vector4") => material_vector_field(ui, entry, true),
                                        _ => material_vector_field(ui, entry, false),
                                    }
                                } else {
                                    let kind = if group == "m_Ints" { "int" } else { "float" };
                                    typed_field(ui, entry, kind, &index);
                                }
                            }
                        });
                    }
                });
            }
            ui.collapsing(crate::localization::tr("原始材质属性 YAML"), |ui| readonly_text(ui, &saved.value, true));
        }
        ui.collapsing(crate::localization::tr("其他渲染设置"), |ui| {
            for field in fields.iter().filter(|field| !is_component_field(&field.name) && !matches!(field.name.as_str(), "serializedVersion" | "m_Shader" | "m_Parent" | "m_CustomRenderQueue" | "m_EnableInstancingVariants" | "m_DoubleSidedGI" | "m_SavedProperties") && matches(field)) {
                raw_field(ui, field, crate::localization::tr("原始值"));
            }
        });
    });
}

fn parse_shader_properties(source: &str) -> HashMap<String, String> {
    let pattern = Regex::new(r#"(?s)//[^\r\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|[^\s]"#).unwrap();
    let tokens: Vec<_> = pattern.find_iter(source).map(|item| item.as_str()).filter(|item| !item.starts_with("//") && !item.starts_with("/*")).collect();
    let mut properties = HashMap::new();
    let Some(start) = tokens.windows(2).position(|pair| pair == ["Properties", "{"]) else { return properties };
    let mut depth = 1;
    let mut position = start + 2;
    while position < tokens.len() && depth > 0 {
        match tokens[position] { "{" => depth += 1, "}" => depth -= 1, _ => {} }
        if depth == 1 && position + 5 < tokens.len() && tokens[position+1] == "(" && tokens[position+2].starts_with('"') && tokens[position+3] == "," && tokens[position+5] == ")" {
            let kind = match tokens[position+4] { "Color" => Some("Color"), "Vector" => Some("Vector4"), _ => None };
            if let Some(kind) = kind { properties.insert(tokens[position].into(), kind.into()); }
        }
        position += 1;
    }
    properties
}

fn material_vector_field(ui: &mut egui::Ui, field: &YamlField, confirmed: bool) {
    ui.horizontal(|ui| {
        property_label(ui, &field.name, if confirmed { "Vector4" } else { "Color / Vector4" });
        if confirmed && crate::editing::control(ui, &field.name, &field.value, "MaterialVector4", None) { return; }
        if let Some(values) = crate::inline_vector(&field.value, &["r", "g", "b", "a"]) {
            let available = ui.clip_rect().width() - (ui.cursor().min.x - ui.min_rect().left());
            let values: Vec<_> = values.iter().map(ToString::to_string).collect();
            crate::show_vector_inputs(ui, &["x", "y", "z", "w"], &values, available);
        } else { invalid_value(ui, &field.value); }
    });
}

pub fn show_animator_properties(ui: &mut egui::Ui, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let search = query.trim().to_lowercase();
    let mut index = ProjectIndex::default();
    for (name, entries) in [
        ("AnimatorCullingMode", vec![("AlwaysAnimate", 0), ("CullUpdateTransforms", 1), ("CullCompletely", 2)]),
        ("AnimatorUpdateMode", vec![("Normal", 0), ("Fixed / AnimatePhysics", 1), ("UnscaledTime", 2)]),
    ] {
        index.enums.insert(name.to_string(), vec![entries.into_iter().map(|(name, value)| (name.to_string(), value)).collect()]);
    }
    ui.small(crate::localization::tr("Animator 属性；引用资源可打开，本文件对象可定位。"));
    property_scroll(ui, "animator_properties", |ui| {
            publish_property_columns(ui);
        ui.collapsing(crate::localization::tr("组件信息"), |ui| {
            for field in fields.iter().filter(|field| sprite_metadata(&field.name)) {
                raw_field(ui, field, crate::localization::tr("组件信息"));
            }
        });
        ui.separator();
        for field in fields.iter().filter(|field| !sprite_metadata(&field.name)) {
            let (label, kind) = match field.name.as_str() {
                "serializedVersion" => continue,
                "m_Enabled" => ("Enabled", "bool"),
                "m_Avatar" => ("Avatar", crate::localization::tr("Avatar 引用")),
                "m_Controller" => ("Controller", crate::localization::tr("RuntimeAnimatorController 引用")),
                "m_CullingMode" => ("Culling Mode", "AnimatorCullingMode"),
                "m_UpdateMode" => ("Update Mode", "AnimatorUpdateMode"),
                "m_ApplyRootMotion" => ("Apply Root Motion", "bool"),
                "m_LinearVelocityBlending" => ("Linear Velocity Blending", "bool"),
                "m_StabilizeFeet" => ("Stabilize Feet", "bool"),
                "m_HasTransformHierarchy" => ("Has Transform Hierarchy", "bool"),
                "m_AllowConstantClipSamplingOptimization" => ("Constant Clip Sampling Optimization", "bool"),
                "m_KeepAnimatorStateOnDisable" | "m_KeepAnimatorControllerStateOnDisable" => ("Keep State On Disable", "bool"),
                "m_WriteDefaultValuesOnDisable" => ("Write Default Values On Disable", "bool"),
                "m_AnimatePhysics" => ("Animate Physics", "bool"),
                "m_WarningMessage" => ("Warning Message", "string"),
                _ => (field.name.as_str(), crate::localization::tr("原始值")),
            };
            if !search.is_empty() && !format!("{} {label} {kind} {}", field.name, field.value).to_lowercase().contains(&search) { continue; }
            ui.push_id(&field.name, |ui| {
                let display = YamlField { name: label.to_string(), value: field.value.clone() };
                if kind == crate::localization::tr("原始值") { raw_field(ui, &display, kind); }
                else { typed_field(ui, &display, kind, &index); }
            });
        }
    });
}

pub fn show_animator_state_properties(ui: &mut egui::Ui, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let index = ProjectIndex::default();
    let search = query.trim().to_lowercase();
    let properties = [
        ("m_Name", "string"), ("m_Tag", "string"),
        ("m_Motion", crate::localization::tr("Motion 引用")), ("m_Speed", "float"),
        ("m_SpeedParameter", "string"), ("m_SpeedParameterActive", "bool"),
        ("m_TimeParameter", "string"), ("m_TimeParameterActive", "bool"),
        ("m_Mirror", "bool"), ("m_MirrorParameter", "string"), ("m_MirrorParameterActive", "bool"),
        ("m_CycleOffset", "float"), ("m_CycleOffsetParameter", "string"), ("m_CycleOffsetParameterActive", "bool"),
        ("m_IKOnFeet", "bool"), ("m_WriteDefaultValues", "bool"),
        ("m_Transitions", "List<AnimatorStateTransition>"),
        ("m_StateMachineBehaviours", "List<StateMachineBehaviour>"),
    ];
    ScrollArea::both().id_salt("animator_state_properties").auto_shrink([false, false]).show(ui, |ui| {
            publish_property_columns(ui);
        for (name, kind) in properties {
            let Some(field) = fields.iter().find(|field| field.name == name) else { continue };
            if !search.is_empty() && !format!("{name} {kind} {}", field.value).to_lowercase().contains(&search) { continue; }
            ui.push_id(name, |ui| typed_field(ui, field, kind, &index));
        }
        ui.collapsing(crate::localization::tr("其他属性"), |ui| {
            for field in fields.iter().filter(|field| !properties.iter().any(|(name, _)| *name == field.name)) {
                if !search.is_empty() && !format!("{} {}", field.name, field.value).to_lowercase().contains(&search) { continue; }
                if field.name == "m_Position" { typed_field(ui, field, "Vector3", &index); }
                else { raw_field(ui, field, crate::localization::tr("原始值")); }
            }
        });
    });
}

pub fn is_collider_type(name: &str) -> bool {
    matches!(name, "PolygonCollider2D" | "BoxCollider2D" | "CircleCollider2D" | "CapsuleCollider2D"
        | "EdgeCollider2D" | "CompositeCollider2D" | "BoxCollider" | "SphereCollider"
        | "CapsuleCollider" | "MeshCollider" | "TerrainCollider" | "CharacterController")
}

pub fn show_collider_properties(ui: &mut egui::Ui, type_name: &str, text: &str, query: &str) {
    let fields = yaml_fields(text);
    let index = ProjectIndex::default();
    let search = query.trim().to_lowercase();
    let is_2d = type_name.ends_with("2D");
    ui.small(crate::localization::tr("碰撞体属性；进入编辑模式后可修改已确认类型的属性。"));
    property_scroll(ui, "collider_properties", |ui| {
            publish_property_columns(ui);
        ui.collapsing(crate::localization::tr("组件信息"), |ui| {
            for field in fields.iter().filter(|field| sprite_metadata(&field.name)) {
                raw_field(ui, field, crate::localization::tr("组件信息"));
            }
        });
        ui.separator();
        for field in fields.iter().filter(|field| !sprite_metadata(&field.name)) {
            if !search.is_empty() && !format!("{} {}", field.name, field.value).to_lowercase().contains(&search) { continue; }
            ui.push_id(&field.name, |ui| {
                if matches!(field.name.as_str(), "m_Points" | "m_Paths") {
                    show_collider_points(ui, field);
                    return;
                }
                let (label, kind) = match field.name.as_str() {
                    "m_Enabled" => ("Enabled", "bool"),
                    "m_IsTrigger" => ("Is Trigger", "bool"),
                    "m_UsedByEffector" => ("Used By Effector", "bool"),
                    "m_UsedByComposite" => ("Used By Composite", "bool"),
                    "m_AutoTiling" => ("Auto Tiling", "bool"),
                    "m_UseDelaunayMesh" => ("Use Delaunay Mesh", "bool"),
                    "m_Convex" => ("Convex", "bool"),
                    "m_ProvidesContacts" => ("Provides Contacts", "bool"),
                    "m_IsComposite" => ("Is Composite", "bool"),
                    "m_Offset" => ("Offset", "Vector2"),
                    "m_Center" => ("Center", "Vector3"),
                    "m_Size" => ("Size", if is_2d { "Vector2" } else { "Vector3" }),
                    "m_Radius" => ("Radius", "float"),
                    "m_Height" => ("Height", "float"),
                    "m_EdgeRadius" => ("Edge Radius", "float"),
                    "m_Density" => ("Density", "float"),
                    "m_ContactOffset" => ("Contact Offset", "float"),
                    "m_VertexDistance" => ("Vertex Distance", "float"),
                    "m_OffsetDistance" => ("Offset Distance", "float"),
                    "m_SkinWidth" => ("Skin Width", "float"),
                    "m_SlopeLimit" => ("Slope Limit", "float"),
                    "m_StepOffset" => ("Step Offset", "float"),
                    "m_MinMoveDistance" => ("Min Move Distance", "float"),
                    "m_Direction" => ("Direction", "int"),
                    "m_GeometryType" => ("Geometry Type", "int"),
                    "m_GenerationType" => ("Generation Type", "int"),
                    "m_CompositeOperation" => ("Composite Operation", "int"),
                    "m_CompositeOrder" => ("Composite Order", "int"),
                    "m_LayerOverridePriority" => ("Layer Override Priority", "int"),
                    "m_CookingOptions" => ("Cooking Options", "int"),
                    "m_Material" => ("Material", if is_2d { crate::localization::tr("PhysicsMaterial2D 引用") } else { crate::localization::tr("PhysicMaterial 引用") }),
                    "m_Mesh" => ("Mesh", crate::localization::tr("Mesh 引用")),
                    "m_TerrainData" => ("Terrain Data", crate::localization::tr("TerrainData 引用")),
                    "m_IncludeLayers" => ("Include Layers", "LayerMask"),
                    "m_ExcludeLayers" => ("Exclude Layers", "LayerMask"),
                    "m_ForceSendLayers" => ("Force Send Layers", "LayerMask"),
                    "m_ForceReceiveLayers" => ("Force Receive Layers", "LayerMask"),
                    "m_ContactCaptureLayers" => ("Contact Capture Layers", "LayerMask"),
                    "m_CallbackLayers" => ("Callback Layers", "LayerMask"),
                    "m_SpriteTilingProperty" => ("Sprite Tiling", crate::localization::tr("原始值")),
                    "serializedVersion" => return,
                    _ => (field.name.as_str(), crate::localization::tr("原始值")),
                };
                let mut display = YamlField { name: label.to_string(), value: field.value.clone() };
                if kind == "LayerMask" {
                    // Unity stores layer bits inside a versioned mapping.
                    if let Some(bits) = field.value.lines().find_map(|line| line.trim().strip_prefix("m_Bits:")) {
                        display.value = bits.trim().to_string();
                        let kind = if display.value.starts_with('-') { "int" } else { "uint" };
                        typed_field(ui, &display, kind, &index);
                    } else { raw_field(ui, &display, kind); }
                } else if kind == crate::localization::tr("原始值") { raw_field(ui, &display, kind); }
                else { typed_field(ui, &display, kind, &index); }
            });
        }
    });
}

fn collider_point_paths(value: &str) -> Option<Vec<Vec<Vec<f64>>>> {
    let mut paths: Vec<Vec<Vec<f64>>> = Vec::new();
    for line in value.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "m_Paths:" || trimmed == "[]" || trimmed == "m_Paths: []" { continue; }
        let tail = trimmed.strip_prefix('-')?.trim_start();
        let (new_path, vector) = if let Some(vector) = tail.strip_prefix('-') {
            (true, vector.trim_start())
        } else { (false, tail) };
        if new_path || paths.is_empty() { paths.push(Vec::new()); }
        paths.last_mut()?.push(super::inline_vector(vector, &["x", "y"])?);
    }
    Some(paths)
}

fn show_collider_points(ui: &mut egui::Ui, field: &YamlField) {
    crate::editing::with_field(ui, &field.name, |ui| show_collider_points_inner(ui, field));
}
fn show_collider_points_inner(ui: &mut egui::Ui, field: &YamlField) {
    let Some(paths) = collider_point_paths(&field.value) else {
        raw_field(ui, field, crate::localization::tr("未识别的顶点结构"));
        return;
    };
    let count: usize = paths.iter().map(Vec::len).sum();
    ui.collapsing(crate::lf!("Points · {0} 条路径，{1} 个顶点", paths.len(), count), |ui| {
        for (path_index, points) in paths.iter().enumerate() {
            ui.push_id(path_index, |ui| {
                ui.collapsing(crate::lf!("Path {1} · {0} 个顶点", points.len(), path_index), |ui| {
                    for (point_index, point) in points.iter().enumerate() {
                        ui.push_id(point_index, |ui| {
                            let shape = crate::editing::placeholder(ui);
                            let response = ui.horizontal(|ui| {
                                ui.label(format!("[{point_index}]"));
                                let flat_index = paths[..path_index].iter().map(Vec::len).sum::<usize>() + point_index;
                                if crate::editing::with_flow_value(ui, flat_index, |ui, raw| crate::editing::control(ui, "值", raw, "Vector2", None)).unwrap_or(false) { return; }
                                let available = ui.clip_rect().width() - (ui.cursor().min.x - ui.min_rect().left());
                                let values: Vec<String> = point.iter().map(ToString::to_string).collect();
                                super::show_vector_inputs(ui, &["x", "y"], &values, available);
                            });
                            let flat_index = paths[..path_index].iter().map(Vec::len).sum::<usize>() + point_index;
                            let dirty = crate::editing::with_flow_value(ui, flat_index, |ui, _| crate::editing::dirty_scope(ui)).unwrap_or(false);
                            let mut rect = response.response.rect; rect.max.x = ui.max_rect().right();
                            crate::editing::highlight(ui, shape, rect, dirty);
                        });
                    }
                });
            });
        }
    });
}

fn sprite_field_info(name: &str) -> (&str, &str) {
    match name {
        "m_Enabled" => ("Enabled", "bool"),
        "m_Sprite" => ("Sprite", crate::localization::tr("Sprite 引用")),
        "m_Color" => ("Color", "Color"),
        "m_FlipX" => ("Flip X", "bool"),
        "m_FlipY" => ("Flip Y", "bool"),
        "m_DrawMode" => ("Draw Mode", "SpriteDrawMode"),
        "m_Size" => ("Size", "Vector2"),
        "m_SpriteTileMode" => ("Tile Mode", "SpriteTileMode"),
        "m_AdaptiveModeThreshold" => ("Adaptive Threshold", "float"),
        "m_MaskInteraction" => ("Mask Interaction", "SpriteMaskInteraction"),
        "m_SpriteSortPoint" => ("Sort Point", "SpriteSortPoint"),
        "m_SortingLayerID" => ("Sorting Layer ID", "int"),
        "m_SortingLayer" => ("Sorting Layer Index", "int"),
        "m_SortingOrder" => ("Order in Layer", "int"),
        "m_Materials" => ("Materials", "Material[]"),
        "m_CastShadows" => ("Cast Shadows", "ShadowCastingMode"),
        "m_ReceiveShadows" => ("Receive Shadows", "bool"),
        "m_DynamicOccludee" => ("Dynamic Occlusion", "bool"),
        "m_StaticShadowCaster" => ("Static Shadow Caster", "bool"),
        "m_RendererPriority" => ("Renderer Priority", "int"),
        "m_RenderingLayerMask" => ("Rendering Layer Mask", "uint"),
        "m_ScaleInLightmap" => ("Scale in Lightmap", "float"),
        "m_ProbeAnchor" => ("Probe Anchor", crate::localization::tr("Transform 引用")),
        "m_LightProbeVolumeOverride" => ("Light Probe Volume", crate::localization::tr("GameObject 引用")),
        _ => (name, crate::localization::tr("原始值")),
    }
}

fn raw_field(ui: &mut egui::Ui, field: &YamlField, field_type: &str) {
    if field.value.contains('\n') {
        ui.collapsing(format!("{}  ({field_type})", field.name), |ui| {
            readonly_text(ui, &field.value, true);
        });
    } else {
        ui.horizontal(|ui| {
            property_label(ui, &field.name, field_type);
            if crate::editing::active(ui) && !field.value.contains("fileID:") {
                ui.add(egui::Label::new(RichText::new(crate::localization::tr("类型未确认，暂不开放编辑")).small())).on_hover_text(crate::localization::tr("无法验证声明类型的属性保持只读，避免写入不符合 Unity 类型的数据。"));
            }
            readonly_text(ui, &field.value, false);
        });
    }
}

fn readonly_text(ui: &mut egui::Ui, value: &str, multiline: bool) {
    let reference_text = value;
    let mut value = if multiline { value.to_string() } else { crate::yaml_strings::display(value).into_owned() };
    let edit = if multiline { TextEdit::multiline(&mut value) } else { TextEdit::singleline(&mut value) };
    let response = ui.add(edit.desired_width(if multiline { 500.0 } else { 240.0 }).interactive(false));
    if !multiline && reference_text.contains("fileID:") { crate::editing::reference_warning(ui, response.rect); }
    reference_buttons(ui, reference_text);
}

pub fn show_graph_reference(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| { ui.label(label); readonly_text(ui, value, false); });
}

#[derive(Clone)]
pub enum ReferenceOpen {
    Asset(PathBuf, String),
    Local(String),
}

pub fn take_reference_request(ctx: &egui::Context) -> Option<ReferenceOpen> {
    ctx.data_mut(|data| {
        let id = egui::Id::new("open_reference");
        let request = data.get_temp::<ReferenceOpen>(id);
        data.remove::<ReferenceOpen>(id);
        request
    })
}

fn reference_buttons(ui: &mut egui::Ui, value: &str) {
    if !value.contains("fileID:") || !value.contains('{') { return; }
    let pattern = Regex::new(r"\{\s*fileID:\s*(-?\d+)([^{}]*)\}").unwrap();
    let catalog = ui.ctx().data_mut(|data| data.get_temp::<Arc<HashMap<String, PathBuf>>>(egui::Id::new("asset_catalog")));
    for (i, reference) in pattern.captures_iter(value).enumerate() {
        let file_id = reference[1].to_string();
        let guid = guid_from_meta(&reference[2]);
        let (request, explanation) = if let Some(guid) = guid {
            if let Some(path) = catalog.as_ref().and_then(|catalog| catalog.get(&guid)) {
                (Some(ReferenceOpen::Asset(path.clone(), file_id)), path.display().to_string())
            } else {
                (None, crate::localization::tr("GUID 索引尚未完成，或资源不存在、属于内置资源。").to_string())
            }
        } else if file_id != "0" {
            (Some(ReferenceOpen::Local(file_id)), crate::localization::tr("跳转到本文件中对应的对象").to_string())
        } else { continue; };
        let button_label = if matches!(request, Some(ReferenceOpen::Local(_))) { crate::localization::tr("定位") } else { crate::localization::tr("打开") };
        ui.push_id(i, |ui| {
            if ui.add_enabled(request.is_some(), egui::Button::new(button_label)).on_hover_text(explanation).clicked() {
                ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("open_reference"), request.unwrap()));
            }
        });
    }
}

pub fn show_prefab_reference(ui: &mut egui::Ui, text: &str, documents: &[crate::Document]) {
    if let Some(reference) = prefab_source_reference(text, documents) {
        ui.horizontal(|ui| {
            ui.label(crate::localization::tr("源 Prefab / 对象"));
            readonly_text(ui, &reference, false);
        });
    }
}

fn prefab_source_reference(text: &str, documents: &[crate::Document]) -> Option<String> {
    let fields: Vec<_> = text.lines().filter_map(|line| {
        let top = line.strip_prefix("  ")?;
        if top.starts_with([' ', '-']) { return None; }
        let (name, value) = top.split_once(':')?;
        matches!(name, "m_CorrespondingSourceObject" | "m_SourcePrefab" | "m_PrefabInstance")
            .then(|| (name, value.trim()))
    }).collect();
    // Stripped components can point straight to their corresponding source
    // object. Its fileID lets the viewer select that object after opening.
    for name in ["m_CorrespondingSourceObject", "m_SourcePrefab"] {
        if let Some((_, value)) = fields.iter().find(|(field, value)| *field == name && guid_from_meta(value).is_some()) {
            return Some((*value).to_owned());
        }
    }
    let (_, instance) = fields.iter().find(|(field, _)| *field == "m_PrefabInstance")?;
    let pattern = Regex::new(r"fileID:\s*(-?\d+)").unwrap();
    let id = pattern.captures(instance)?.get(1)?.as_str();
    if id == "0" { return None; }
    let instance = documents.iter().find(|document| document.file_id == id)?;
    yaml_fields(&instance.text).into_iter()
        .find(|field| field.name == "m_SourcePrefab" && guid_from_meta(&field.value).is_some())
        .map(|field| field.value)
}

fn typed_field(ui: &mut egui::Ui, field: &YamlField, field_type: &str, index: &ProjectIndex) {
    typed_field_range(ui, field, field_type, index, None);
}

fn typed_field_range(ui: &mut egui::Ui, field: &YamlField, field_type: &str, index: &ProjectIndex, range: Option<(f64, f64)>) {
    crate::editing::with_field(ui, &field.name, |ui| {
        let shape = crate::editing::placeholder(ui);
        let output = ui.scope(|ui| {
            if let Some(range) = range {
                ui.horizontal(|ui| {
                    property_label(ui, &field.name, field_type);
                    crate::editing::range_control(ui, &field.name, &field.value, field_type, range);
                });
            } else { typed_field_inner(ui, field, field_type, index); }
        });
        let mut rect = output.response.rect;
        rect.max.x = ui.max_rect().right();
        crate::editing::highlight(ui, shape, rect, collection_element_type(field_type).is_none() && (!field.value.contains('\n') || short_type(field_type) == "Gradient") && crate::editing::dirty_scope(ui));
    });
}

fn typed_field_inner(ui: &mut egui::Ui, field: &YamlField, field_type: &str, index: &ProjectIndex) {
    if let Some(element) = collection_element_type(field_type) {
        if crate::editing::packed_int_list(ui, &field.name, &field.value, element) { return; }
    }
    if collection_element_type(field_type).is_some() {
        if let Some(mut table) = collection_table(&field.name, &field.value) {
            assign_table_types(&mut table, collection_element_type(field_type).unwrap(), index);
            egui::CollapsingHeader::new(crate::lf!("{0} ({2}) · {1} 条", field.name, table.rows.len(), field_type))
                .default_open(true).show(ui, |ui| {
                    let mode_id = ui.id().with("collection_property_mode");
                    let mut properties = ui.ctx().data_mut(|data| data.get_temp::<bool>(mode_id)).unwrap_or(false);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut properties, true, crate::localization::tr("属性"));
                        ui.selectable_value(&mut properties, false, crate::localization::tr("表格"));
                    });
                    ui.ctx().data_mut(|data| data.insert_temp(mode_id, properties));
                    if table.rows.is_empty() { ui.label(crate::localization::tr("空列表")); }
                    else if properties {
                        for (row_index, row) in table.rows.iter().enumerate() {
                            ui.push_id(row_index, |ui| {
                                crate::editing::with_row(ui, row_index, |ui| {
                                    egui::CollapsingHeader::new(format!("[{row_index}]"))
                                        .default_open(true).show(ui, |ui| {
                                            for header in table.headers.iter().skip(1) {
                                                let Some(value) = row.get(header) else { continue; };
                                                let nested = YamlField { name: header.clone(), value: value.clone() };
                                                ui.push_id(header, |ui| {
                                                    if let Some(kind) = table.column_types.get(header) {
                                                        typed_field(ui, &nested, kind, index);
                                                    } else { raw_field(ui, &nested, crate::localization::tr("类型未知")); }
                                                });
                                            }
                                        });
                                });
                            });
                        }
                    }
                    else {
                        let state_id = ui.id().with("collection_table_layout");
                        let (mut widths, mut offset, mut freeze) = ui.ctx().data_mut(|data| data.get_temp::<(Vec<f32>, f32, bool)>(state_id))
                            .filter(|(widths, _, _)| widths.len() == table.headers.len())
                            .unwrap_or_else(|| (crate::table_column_widths(&table), 0.0, true));
                        ui.checkbox(&mut freeze, crate::localization::tr("冻结表头"));
                        ui.push_id(&field.name, |ui| {
                            ui.scope(|ui| {
                                ui.set_max_height(320.0);
                                offset = crate::show_yaml_table(ui, &table, freeze, &mut widths, offset);
                            });
                        });
                        ui.ctx().data_mut(|data| data.insert_temp(state_id, (widths, offset, freeze)));
                    }
                    ui.collapsing(crate::localization::tr("原始列表 YAML"), |ui| readonly_text(ui, &field.value, true));
                });
            return;
        }
    }
    let kind = short_type(field_type);
    if matches!(field_type, "RectOffset" | "UnityEngine.RectOffset") {
        if let Some(offsets) = rect_offset_fields(&field.value) {
            egui::CollapsingHeader::new(format!("{} (RectOffset)", field.name)).default_open(true).show(ui, |ui| {
                for (label, value) in offsets {
                    ui.push_id(label, |ui| {
                        typed_field(ui, &YamlField { name: label.into(), value }, "int", index);
                    });
                }
            });
        } else { raw_field(ui, field, crate::localization::tr("RectOffset（未识别的结构）")); }
        return;
    }
    if matches!(field_type, "Gradient" | "UnityEngine.Gradient") {
        if crate::gradient_property::show(ui, &field.name, &field.value) { return; }
        raw_field(ui, field, crate::localization::tr("Gradient（未识别的结构或模式）"));
        return;
    }
    if field.value.contains('\n') {
        let class = index.classes.get(kind).filter(|classes| classes.len() == 1).map(|classes| &classes[0]);
        let builtin = builtin_struct_fields(field_type);
        if !field_type.contains(['[', '<']) && (class.is_some() || builtin.is_some()) {
            let nested_text = field.value.lines().map(|line| line.strip_prefix("  ").unwrap_or(line))
                .collect::<Vec<_>>().join("\n");
            let nested = yaml_fields(&nested_text);
            if !nested.is_empty() {
                let mut known = builtin.unwrap_or_default();
                let ranges = class.map(|class| index.inherited_ranges(class, &mut HashSet::new())).unwrap_or_default();
                if let Some(class) = class {
                    known.extend(index.inherited_fields(class, &mut HashSet::new()));
                }
                ui.collapsing(format!("{}  ({field_type})", field.name), |ui| {
                    for field in &nested {
                        ui.push_id(&field.name, |ui| {
                            if let Some(kind) = known.get(&field.name) {
                                typed_field_range(ui, field, kind, index, ranges.get(&field.name).copied());
                            } else { raw_field(ui, field, crate::localization::tr("类型未知")); }
                        });
                    }
                });
                return;
            }
        }
        raw_field(ui, field, field_type);
        return;
    }
    ui.horizontal(|ui| {
        property_label(ui, &field.name, field_type);
        let builtin = builtin_enum_values(field_type);
        let enums = index.enums.get(field_type).or_else(|| index.enums.get(kind)).filter(|candidates| candidates.len() == 1)
            .map(|candidates| candidates[0].as_slice()).or(builtin.as_deref());
        if crate::editing::control(ui, &field.name, &field.value, field_type, enums) { return; }
        match kind {
            "bool" | "Boolean" => {
                match field.value.as_str() {
                    "0" | "1" | "true" | "false" => {
                        let mut value = matches!(field.value.as_str(), "1" | "true");
                        ui.add_enabled(false, egui::Checkbox::without_text(&mut value));
                    }
                    _ => invalid_value(ui, &field.value),
                }
            }
            "int" | "Int32" | "uint" | "UInt32" | "long" | "Int64" | "ulong" | "UInt64" | "short" | "Int16" | "ushort" | "UInt16" | "byte" | "Byte" | "sbyte" | "SByte" => {
                let valid = match kind {
                    "int" | "Int32" => field.value.parse::<i32>().is_ok(),
                    "uint" | "UInt32" => field.value.parse::<u32>().is_ok(),
                    "long" | "Int64" => field.value.parse::<i64>().is_ok(),
                    "ulong" | "UInt64" => field.value.parse::<u64>().is_ok(),
                    "short" | "Int16" => field.value.parse::<i16>().is_ok(),
                    "ushort" | "UInt16" => field.value.parse::<u16>().is_ok(),
                    "byte" | "Byte" => field.value.parse::<u8>().is_ok(),
                    _ => field.value.parse::<i8>().is_ok(),
                };
                if valid { readonly_text(ui, &field.value, false); } else { invalid_value(ui, &field.value); }
            }
            "float" | "Single" | "double" | "Double" => {
                let valid = if matches!(kind, "float" | "Single") {
                    field.value.parse::<f32>().is_ok_and(f32::is_finite)
                } else { field.value.parse::<f64>().is_ok_and(f64::is_finite) };
                if valid { readonly_text(ui, &field.value, false); } else { invalid_value(ui, &field.value); }
            }
            "Vector2" | "Vector3" | "Vector4" | "Quaternion" | "Color" | "Vector2Int" | "Vector3Int" => {
                let axes: &[&str] = match kind {
                    "Vector2" | "Vector2Int" => &["x", "y"],
                    "Vector3" | "Vector3Int" => &["x", "y", "z"],
                    "Color" => &["r", "g", "b", "a"],
                    _ => &["x", "y", "z", "w"],
                };
                if let Some(values) = super::inline_vector(&field.value, axes) {
                    if kind.ends_with("Int") && values.iter().any(|value| value.fract() != 0.0 || *value < i32::MIN as f64 || *value > i32::MAX as f64) {
                        invalid_value(ui, &field.value); return;
                    }
                    if kind == "Color" {
                        super::color_property::show(ui, &field.name, [values[0], values[1], values[2], values[3]]);
                        return;
                    }
                    let available = ui.clip_rect().width() - (ui.cursor().min.x - ui.min_rect().left());
                    let values: Vec<String> = values.iter().map(ToString::to_string).collect();
                    super::show_vector_inputs(ui, axes, &values, available);
                } else { invalid_value(ui, &field.value); }
            }
            _ => {
                let builtin = builtin_enum_values(field_type);
                if let Some(values) = index.enums.get(field_type).or_else(|| index.enums.get(kind)).filter(|candidates| candidates.len() == 1)
                    .map(|candidates| candidates[0].as_slice()).or(builtin.as_deref()) {
                    let value = field.value.parse::<i64>().ok();
                    let selected = values.iter().find(|(_, number)| Some(*number) == value)
                        .map(|(name, number)| format!("{name} ({number})"))
                        .unwrap_or_else(|| crate::lf!("未匹配值 ({0})", field.value));
                    egui::ComboBox::from_id_salt(("enum_options", &field.name, field_type))
                        .width(240.0)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            ui.small(crate::localization::tr("枚举列表（只读）"));
                            for (name, number) in values {
                                // Allow opening and inspecting the list without
                                // storing a replacement value in the asset.
                                ui.selectable_label(Some(*number) == value, format!("{name} ({number})"))
                                    .on_hover_text(crate::localization::tr("仅查看枚举定义，不修改当前值"));
                            }
                        });
                } else { readonly_text(ui, &field.value, false); }
            }
        }
    });
}

fn collection_element_type(field_type: &str) -> Option<&str> {
    if let Some(element) = field_type.strip_suffix("[]") { return (!element.is_empty()).then_some(element); }
    let (container, argument) = field_type.split_once('<')?;
    if short_type(container) != "List" { return None; }
    argument.strip_suffix('>').filter(|element| !element.is_empty())
}

fn collection_table(name: &str, text: &str) -> Option<crate::YamlTable> {
    if text.trim() == "[]" {
        return Some(crate::YamlTable { title: name.into(), headers: Vec::new(), rows: Vec::new(), column_types: BTreeMap::new(), column_enums: BTreeMap::new() });
    }
    let lines: Vec<_> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let first = *lines.first()?;
    let indentation = first.len() - first.trim_start().len();
    if !first.trim_start().starts_with("- ") { return None; }
    let mut headers = vec!["索引".to_string()];
    let mut rows = Vec::new();
    let mut row: Option<BTreeMap<String, String>> = None;
    let mut object_items = None;
    for line in lines {
        let depth = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        let new_item = depth == indentation && trimmed.starts_with("- ");
        if new_item {
            if let Some(previous) = row.take() { rows.push(previous); }
            let mut next = BTreeMap::new();
            next.insert("索引".into(), rows.len().to_string());
            row = Some(next);
        } else if depth != indentation + 2 || row.is_none() || trimmed.starts_with('-') {
            // Nested lists/maps and block scalars retain their original view.
            return None;
        }
        let entry = if new_item { &trimmed[2..] } else { trimmed };
        let is_scalar = entry.starts_with(['{', '"', '\'']) || !entry.contains(':');
        if new_item {
            if object_items.is_some_and(|object| object == is_scalar) { return None; }
            object_items = Some(!is_scalar);
        }
        let (key, value) = if is_scalar && new_item { ("值", entry) }
            else if let Some((key, value)) = entry.split_once(':') {
                // Only mapping separators followed by whitespace count as fields.
                if !value.is_empty() && !value.starts_with(char::is_whitespace) {
                    if new_item { ("值", entry) } else { return None; }
                } else { (key.trim(), value.trim()) }
            } else { return None; };
        if key.is_empty() || value.is_empty() || matches!(value, "|" | ">") { return None; }
        if !headers.iter().any(|header| header == key) { headers.push(key.into()); }
        if row.as_mut()?.insert(key.into(), value.into()).is_some() { return None; }
    }
    if let Some(row) = row { rows.push(row); }
    Some(crate::YamlTable { title: name.into(), headers, rows, column_types: BTreeMap::new(), column_enums: BTreeMap::new() })
}

fn assign_table_types(table: &mut crate::YamlTable, element: &str, index: &ProjectIndex) {
    if table.headers.iter().any(|header| header == "值") {
        table.column_types.insert("值".into(), element.into());
    } else if let Some(classes) = index.classes.get(short_type(element)).filter(|classes| classes.len() == 1) {
        table.column_types = index.inherited_fields(&classes[0], &mut HashSet::new());
    }
    for (name, kind) in &table.column_types {
        if let Some(enums) = index.enums.get(kind).or_else(|| index.enums.get(short_type(kind))).filter(|enums| enums.len() == 1) {
            table.column_enums.insert(name.clone(), enums[0].clone());
        } else if let Some(enums) = builtin_enum_values(kind) { table.column_enums.insert(name.clone(), enums); }
    }
}

pub fn show_table_cell(ui: &mut egui::Ui, name: &str, raw: &str, field_type: Option<&str>, enums: Option<&[(String, i64)]>) -> bool {
    if raw.contains("fileID:") && crate::editing::active(ui) {
        let mut value = raw.to_string();
        let response = ui.add(TextEdit::singleline(&mut value).desired_width(ui.available_width()).interactive(false));
        crate::editing::reference_warning(ui, response.rect);
        return true;
    }
    if let Some(kind) = field_type {
        if crate::editing::table_control(ui, name, raw, kind, enums) { return true; }
    }
    match field_type.map(short_type) {
        Some("Color") => {
            if let Some(values) = crate::inline_vector(raw, &["r", "g", "b", "a"]) {
                crate::color_property::show(ui, name, [values[0], values[1], values[2], values[3]]);
                return true;
            }
        }
        Some("bool" | "Boolean") => {
            let mut value = match raw.trim() { "1" | "true" => true, "0" | "false" => false, _ => return false };
            ui.add_enabled(false, egui::Checkbox::without_text(&mut value)).on_hover_text(raw);
            return true;
        }
        _ => {}
    }
    false
}

fn rect_offset_fields(text: &str) -> Option<Vec<(&'static str, String)>> {
    let mut fields = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (name, value) = line.trim().split_once(':')?;
        if !matches!(name, "m_Left" | "m_Right" | "m_Top" | "m_Bottom") || fields.insert(name, value.trim()).is_some() { return None; }
    }
    [("m_Left", "Left"), ("m_Right", "Right"), ("m_Top", "Top"), ("m_Bottom", "Bottom")]
        .into_iter().map(|(name, label)| {
            let value = *fields.get(name)?;
            value.parse::<i32>().ok()?;
            Some((label, value.to_string()))
        }).collect()
}

// Unity UI types may be supplied by an assembly or have unavailable source.
// Use serialized field names, rather than guessing types from YAML values.
fn builtin_struct_fields(field_type: &str) -> Option<BTreeMap<String, String>> {
    let fields = match field_type {
        "SkinnedMeshBounds" => &[("m_Center", "Vector3"), ("m_Extent", "Vector3")][..],
        "ColorBlock" | "UnityEngine.UI.ColorBlock" => &[
            ("m_NormalColor", "Color"),
            ("m_HighlightedColor", "Color"),
            ("m_PressedColor", "Color"),
            ("m_SelectedColor", "Color"),
            ("m_DisabledColor", "Color"),
            ("m_ColorMultiplier", "float"),
            ("m_FadeDuration", "float"),
        ][..],
        _ => return None,
    };
    Some(fields.iter().map(|(name, kind)| (name.to_string(), kind.to_string())).collect())
}

fn builtin_enum_values(field_type: &str) -> Option<Vec<(String, i64)>> {
    match field_type {
        "TextAnchor" | "UnityEngine.TextAnchor" => Some(["UpperLeft", "UpperCenter", "UpperRight", "MiddleLeft", "MiddleCenter", "MiddleRight", "LowerLeft", "LowerCenter", "LowerRight"]
            .into_iter().enumerate().map(|(value, label)| (label.to_string(), value as i64)).collect()),
        "FontStyle" | "UnityEngine.FontStyle" => Some(["Normal","Bold","Italic","BoldAndItalic"].into_iter().enumerate().map(|(value,label)|(label.to_owned(),value as i64)).collect()),
        "HorizontalWrapMode" | "UnityEngine.HorizontalWrapMode" => Some(vec![("Wrap".into(),0),("Overflow".into(),1)]),
        "VerticalWrapMode" | "UnityEngine.VerticalWrapMode" => Some(vec![("Truncate".into(),0),("Overflow".into(),1)]),
        _ => None,
    }
}

fn invalid_value(ui: &mut egui::Ui, value: &str) {
    readonly_text(ui, value, false);
    ui.colored_label(Color32::RED, crate::localization::tr("值与声明类型不符或越界"));
}

fn publish_property_columns(ui: &egui::Ui) {
    let origin = ui.cursor().min.x;
    ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("inspector_property_column_origin"), origin));
}

fn property_label(ui: &mut egui::Ui, name: &str, field_type: &str) {
    let origin = ui.ctx().data_mut(|data| data.get_temp::<f32>(egui::Id::new("inspector_property_column_origin"))).unwrap_or(ui.cursor().min.x);
    let name_width = (210.0 - (ui.cursor().min.x-origin)).max(40.0);
    ui.allocate_ui_with_layout(egui::vec2(name_width, 20.0), egui::Layout::left_to_right(egui::Align::Center),
        |ui| { ui.set_min_width(name_width); ui.add(egui::Label::new(name).truncate()) }).inner.on_hover_text(name);
    if ui.ctx().data_mut(|data| data.get_temp::<bool>(egui::Id::new("inspector_hide_property_types"))).unwrap_or(false) { return; }
    ui.allocate_ui_with_layout(egui::vec2(100.0, 20.0), egui::Layout::left_to_right(egui::Align::Center),
        |ui| { ui.set_min_width(100.0); ui.add(egui::Label::new(RichText::new(field_type).small().weak()).truncate()) }).inner
        .on_hover_text(field_type);
}

#[cfg(test)]
mod tests {
    #[test]
    fn disk_index_roundtrip_replacement_and_invalid_cache() {
        let folder=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("unity-yaml-index-test-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&folder).unwrap();
        let root=folder.join("project");let path=folder.join("index.json");
        let mut index=ProjectIndex::default();
        Arc::make_mut(&mut index.assets).insert("guid".into(),root.join("Assets/a.prefab"));
        index.scripts.insert("script".into(),root.join("Assets/a.cs"));
        Arc::make_mut(&mut index.shader_properties).insert("shader".into(),HashMap::from([("_Color".into(),"Color".into())]));
        index.classes.insert("Thing".into(),vec![ClassInfo{path:root.join("Assets/a.cs"),name:"Thing".into(),base:None,fields:BTreeMap::from([("active".into(),"bool".into())]),ranges:BTreeMap::new()}]);
        write_index_cache(&path,&root,&index).unwrap();
        let loaded=read_index_cache(&path,&root).unwrap();
        assert_eq!(loaded.assets,index.assets);
        assert_eq!(loaded.shader_properties,index.shader_properties);
        assert_eq!(loaded.classes["Thing"][0].fields["active"],"bool");
        assert!(read_index_cache(&path,&folder.join("other")).is_none());
        Arc::make_mut(&mut index.assets).insert("new".into(),root.join("Assets/b.prefab"));
        write_index_cache(&path,&root,&index).unwrap();
        assert_eq!(read_index_cache(&path,&root).unwrap().assets.len(),2);
        index.issues=1;Arc::make_mut(&mut index.assets).clear();
        write_index_cache(&path,&root,&index).unwrap();
        assert_eq!(read_index_cache(&path,&root).unwrap().assets.len(),2);
        let state=ProjectState::Loading(ScanJob{receiver:mpsc::channel().1,assets:Some(loaded.assets.clone()),cached:Some(loaded),progress:Arc::new(ScanProgress::default())});
        assert!(state.index().unwrap().classes.contains_key("Thing"));
        fs::write(&path,"invalid").unwrap();assert!(read_index_cache(&path,&root).is_none());
        fs::remove_file(&path).unwrap();fs::remove_dir(&folder).unwrap();
    }
    #[test]
    fn particle_modules_split_nested_fields_and_keep_unconfirmed_types_readonly() {
        let fields=yaml_fields("ParticleSystem:\n  looping: 1\n  InitialModule:\n    enabled: 1\n    startLifetime:\n      scalar: 5\n");
        assert_eq!(fields.len(),2);
        let nested=fields[1].value.lines().map(|line|line.strip_prefix("  ").unwrap_or(line)).collect::<Vec<_>>().join("\n");
        let children=yaml_fields(&nested);
        assert_eq!(children[0].name,"enabled");assert_eq!(children[1].name,"startLifetime");
        assert_eq!(particle_field_kind("looping",false,0),"bool");
        assert_eq!(particle_field_kind("enabled",false,1),"bool");
        assert_eq!(particle_field_kind("m_MinParticleSize",true,0),"float");
        assert_eq!(particle_field_kind("randomSeed",false,0),"uint");
        assert_eq!(particle_field_kind("scalar",false,2),crate::localization::tr("原始值"));
    }
    #[test]
    fn editor_only_classes_and_fields_are_recognized() {
        let source="\n#if UNITY_EDITOR\npublic class TiledMapGrid : MonoBehaviour { public int MAP_WIDTH; public Color Color;\n#if CUSTOM_SYMBOL\npublic int unknown;\n#endif\n}\n#else\npublic class RuntimeGrid {}\n#endif\n";
        let (classes,_)=super::parse_source(source,std::path::Path::new("TiledMapGrid.cs"));
        assert_eq!(classes.len(),1);
        assert_eq!(classes[0].name,"TiledMapGrid");
        assert_eq!(classes[0].fields.get("MAP_WIDTH").map(String::as_str),Some("int"));
        assert_eq!(classes[0].fields.get("Color").map(String::as_str),Some("Color"));
        assert!(!classes[0].fields.contains_key("unknown"));
    }

    #[test]
    fn shader_properties_distinguish_colors_and_vectors_without_name_guesses() {
        let source = r#"Shader "Test" {
            Properties {
                // _Fake ("Fake", Color) = (1,1,1,1)
                [HDR] _ArbitraryName ("Tint", Color) = (1,1,1,1)
                _LooksLikeColor ("Direction", Vector) = (0,0,1,0)
                _Speed ("Speed", Float) = 1
            }
            SubShader { _Outside ("Outside", Color) = (1,1,1,1) }
        }"#;
        let properties = parse_shader_properties(source);
        assert_eq!(properties.len(), 2);
        assert_eq!(properties["_ArbitraryName"], "Color");
        assert_eq!(properties["_LooksLikeColor"], "Vector4");
        assert!(!properties.contains_key("_Fake"));
        assert!(!properties.contains_key("_Outside"));
    }
    #[test]
    fn color_block_resolves_serialized_properties_without_source() {
        let known = builtin_struct_fields("UnityEngine.UI.ColorBlock").unwrap();
        let nested = yaml_fields("  m_NormalColor: {r: 1, g: 1, b: 1, a: 1}\n  m_SelectedColor: {r: 0.5, g: 0.5, b: 0.5, a: 1}\n  m_ColorMultiplier: 1\n  m_FadeDuration: 0.1");
        assert_eq!(nested.len(), 4);
        assert_eq!(known[&nested[0].name], "Color");
        assert_eq!(known[&nested[1].name], "Color");
        assert_eq!(known[&nested[2].name], "float");
        assert_eq!(known[&nested[3].name], "float");
        assert_eq!(builtin_struct_fields("ColorBlock"), Some(known));
        assert!(builtin_struct_fields("Custom.ColorBlock").is_none());
    }

    #[test]
    fn range_metadata_preserves_literals_aliases_and_inheritance() {
        let source = r#"
            public class Base {
                [SerializeField, Range(-1, 1)] float offset;
                [Range(0, 5)] public int replaced;
            }
            public class Derived : Base {
                [SerializeField] [UnityEngine.RangeAttribute(0.1f, 10)]
                [FormerlySerializedAs("oldZoom")] float zoom;
                public int replaced;
                [Range(Limits.Min, 10)] public float unknown;
                [Range(10, 1)] public float reversed;
                [Range(0, 1)] public string text;
            }
        "#;
        let (classes, _) = parse_source(source, Path::new("Derived.cs"));
        let mut index = ProjectIndex::default();
        for class in classes { index.classes.entry(class.name.clone()).or_default().push(class); }
        let ranges = index.inherited_ranges(&index.classes["Derived"][0], &mut HashSet::new());
        assert_eq!(ranges["offset"], (-1.0, 1.0));
        assert_eq!(ranges["zoom"], (0.1, 10.0));
        assert_eq!(ranges["oldZoom"], (0.1, 10.0));
        for name in ["replaced", "unknown", "reversed", "text"] { assert!(!ranges.contains_key(name)); }
    }
    use super::*;

    #[test]
    fn lists_and_arrays_become_tables_without_flattening_complex_structures() {
        assert_eq!(collection_element_type("List<SpriteAtlasConfigItem>"), Some("SpriteAtlasConfigItem"));
        assert_eq!(collection_element_type("System.Collections.Generic.List<int>"), Some("int"));
        assert_eq!(collection_element_type("float[]"), Some("float"));
        assert!(collection_element_type("Dictionary<string,int>").is_none());
        let table = collection_table("items", "  - path: Assets/Map/Cloud\n    name: map_cloud\n    allowRotate: 1\n  - path: Assets/Map/Pier\n    name: map_pier\n    allowRotate: 0").unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1]["name"], "map_pier");
        assert_eq!(table.rows[1]["索引"], "1");
        let scalars = collection_table("values", "  - 1\n  - 2\n  - 3").unwrap();
        assert_eq!(scalars.rows[2]["值"], "3");
        assert_eq!(collection_table("empty", "[]").unwrap().rows.len(), 0);
        assert!(collection_table("nested", "  - child:\n      value: 1").is_none());
    }

    #[test]
    fn material_saved_properties_separate_textures_numbers_and_colors() {
        let groups = material_groups("    serializedVersion: 3\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {fileID: 0}\n        m_Scale: {x: 1, y: 1}\n        m_Offset: {x: 0, y: 0}\n    m_Ints: []\n    m_Floats:\n    - _AlphaScale: 1\n    - _Brightness: 1\n    m_Colors:\n    - _Color: {r: 1, g: 1, b: 1, a: 1}\n    - _OverlayColor: {r: 0.5, g: 0.5, b: 0.5, a: 0}");
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0].1[0].name, "_MainTex");
        assert!(groups[0].1[0].value.contains("m_Scale: {x: 1, y: 1}"));
        assert!(groups[1].1.is_empty());
        assert_eq!(groups[2].1[0].value, "1");
        assert_eq!(groups[2].1[1].name, "_Brightness");
        assert_eq!(groups[3].1.len(), 2);
        assert_eq!(crate::inline_vector(&groups[3].1[1].value, &["r", "g", "b", "a"]), Some(vec![0.5, 0.5, 0.5, 0.0]));
    }

    #[test]
    fn rect_offsets_preserve_signed_integers_and_text_anchor_values() {
        let fields = rect_offset_fields("    m_Left: 0\n    m_Right: 10\n    m_Top: -2\n    m_Bottom: 3").unwrap();
        assert_eq!(fields, vec![("Left", "0".into()), ("Right", "10".into()), ("Top", "-2".into()), ("Bottom", "3".into())]);
        assert!(rect_offset_fields("m_Left: 0\nm_Right: 0\nm_Top: 1.5\nm_Bottom: 0").is_none());
        assert!(rect_offset_fields("m_Left: 0").is_none());
        assert_eq!(builtin_enum_values("TextAnchor").unwrap()[4], ("MiddleCenter".into(), 4));
    }

    #[test]
    fn resolves_nested_enums_by_owner_instead_of_ambiguous_short_name() {
        let (classes, enums) = parse_source("public class Gradient2 { [SerializeField] Type _gradientType; [SerializeField] Blend _blendMode; public enum Type { Horizontal, Vertical, Radial, Diamond } public enum Blend { Override, Add, Multiply } } public class Other { public Type value; public enum Type { First, Second } }", Path::new("Gradient2.cs"));
        assert_eq!(classes[0].fields.get("_gradientType").map(String::as_str), Some("Gradient2.Type"));
        assert_eq!(classes[0].fields.get("_blendMode").map(String::as_str), Some("Gradient2.Blend"));
        assert_eq!(classes[1].fields.get("value").map(String::as_str), Some("Other.Type"));
        assert_eq!(enums.iter().find(|(name, _)| name == "Gradient2.Type").unwrap().1[1], ("Vertical".into(), 1));
        assert_eq!(enums.iter().find(|(name, _)| name == "Gradient2.Blend").unwrap().1[0], ("Override".into(), 0));
    }

    #[test]
    fn stripped_component_follows_prefab_instance_to_source() {
        let documents = crate::parse_unity_yaml("--- !u!1001 &10\nPrefabInstance:\n  m_SourcePrefab: {fileID: 100100000, guid: 4e809f962d9a410dbbe6f2f0242e9303, type: 3}\n--- !u!224 &20 stripped\nRectTransform:\n  m_PrefabInstance: {fileID: 10}\n");
        let source = prefab_source_reference(&documents[1].text, &documents).unwrap();
        assert!(source.contains("fileID: 100100000"));
        assert!(source.contains("guid: 4e809f962d9a410dbbe6f2f0242e9303"));
        assert_eq!(prefab_source_reference(&documents[0].text, &documents), Some(source));
        assert!(prefab_source_reference("RectTransform:\n  m_PrefabInstance: {fileID: 0}\n", &documents).is_none());
    }

    #[test]
    fn collider_paths_preserve_groups_and_reject_unrecognized_data() {
        let paths = collider_point_paths("    m_Paths:\n    - - {x: 232.001, y: 39.001}\n      - {x: -74, y: 280}\n    - - {x: 1, y: 2}").unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].len(), 2);
        assert_eq!(paths[0][1], vec![-74.0, 280.0]);
        assert_eq!(paths[1][0], vec![1.0, 2.0]);
        assert!(collider_point_paths("    - {x: 1, y: NaN}").is_none());
        assert!(collider_point_paths("    unknown: 1").is_none());
        assert!(collider_point_paths("[]").unwrap().is_empty());
        assert!(is_collider_type("PolygonCollider2D"));
        assert!(!is_collider_type("CustomCollider"));
    }

    #[test]
    fn script_guid_is_read_from_the_script_reference() {
        let fields = yaml_fields("MonoBehaviour:\n  m_GameObject: {fileID: 1}\n  m_Script: {fileID: 11500000, guid: 4E809F962D9A410DBBE6F2F0242E9303, type: 3}\n  speed: 0\n");
        let script = fields.iter().find(|field| field.name == "m_Script").unwrap();
        assert_eq!(guid_from_meta(&script.value).as_deref(), Some("4e809f962d9a410dbbe6f2f0242e9303"));
    }

    #[test]
    fn reads_declarations_without_guessing_from_values() {
        let source = r#"
            public class Water : MonoBehaviour {
                public float windSpeed = 0;
                public int waveCount;
                [SerializeField, Range(0, 1)] private float amplitude;
                [FormerlySerializedAs("oldTint")] public UnityEngine.Color tint;
                [SerializeField] private Vector2 direction = new Vector2(0, 1);
                [NonSerialized] public int ignored;
                public static int staticField;
                private int hidden;
                public float Property { get; set; }
                public void Update() { float local = 0; string braces = "{;}"; }
                // public int fake;
                /* public bool anotherFake; */
            }
        "#;
        let (classes, _) = parse_source(source, Path::new("Water.cs"));
        assert_eq!(classes.len(), 1);
        let fields = &classes[0].fields;
        assert_eq!(fields.get("windSpeed").map(String::as_str), Some("float"));
        assert_eq!(fields.get("waveCount").map(String::as_str), Some("int"));
        assert_eq!(fields.get("amplitude").map(String::as_str), Some("float"));
        assert_eq!(fields.get("oldTint").map(String::as_str), Some("UnityEngine.Color"));
        assert_eq!(fields.get("direction").map(String::as_str), Some("Vector2"));
        for name in ["ignored", "staticField", "hidden", "Property", "local", "fake", "anotherFake"] {
            assert!(!fields.contains_key(name), "must not expose {name}");
        }
    }

    #[test]
    fn conditional_fields_are_unknown_but_regions_are_supported() {
        let source = "public class Water {\n#region Fields\npublic float speed;\n#if CUSTOM_SYMBOL\npublic int editorField;\n#else\npublic bool editorField;\n#endif\n#endregion\n}";
        let (classes, _) = parse_source(source, Path::new("Water.cs"));
        assert_eq!(classes[0].fields.get("speed").map(String::as_str), Some("float"));
        assert!(!classes[0].fields.contains_key("editorField"));
    }

    #[test]
    fn resolves_inherited_fields_and_explicit_enum_values() {
        let source = "public class Base { public float speed; } public class Water : Base { public int count; } enum Mode { Calm, Rough = 4, Storm }";
        let (classes, enums) = parse_source(source, Path::new("Water.cs"));
        let mut index = ProjectIndex::default();
        for class in classes { index.classes.entry(class.name.clone()).or_default().push(class); }
        let fields = index.inherited_fields(&index.classes["Water"][0], &mut HashSet::new());
        assert_eq!(fields.get("speed").map(String::as_str), Some("float"));
        assert_eq!(fields.get("count").map(String::as_str), Some("int"));
        assert_eq!(enums[0].1, vec![("Calm".into(), 0), ("Rough".into(), 4), ("Storm".into(), 5)]);
    }

    #[test]
    fn retains_nested_values_lists_and_component_metadata() {
        let text = "MonoBehaviour:\n  m_Enabled: 1\n  settings:\n    speed: 0\n    tint: {r: 1, g: 0, b: 0, a: 1}\n  items:\n  - 1\n  - 2\n  count: 0\n";
        let fields = yaml_fields(text);
        assert_eq!(fields.len(), 4);
        assert!(is_component_field(&fields[0].name));
        assert!(fields[1].value.contains("    speed: 0"));
        assert!(fields[2].value.contains("  - 2"));
        assert_eq!(fields[3].value, "0");
    }
}
/// Known uGUI scalar fields stay editable even when Unity's package scripts are unavailable.
pub fn show_ui_graphic_properties_content(ui: &mut egui::Ui, text: &str, query: &str) {
    let mut index=ProjectIndex::default();
    index.enums.insert("ImageType".into(),vec![vec![("Simple".into(),0),("Sliced".into(),1),("Tiled".into(),2),("Filled".into(),3)]]);
    index.enums.insert("ImageFillMethod".into(),vec![vec![("Horizontal".into(),0),("Vertical".into(),1),("Radial90".into(),2),("Radial180".into(),3),("Radial360".into(),4)]]);
    publish_property_columns(ui);
    let fields=yaml_fields(text);
    let kind=|name:&str| match name {
        "m_Enabled"|"m_RaycastTarget"|"m_Maskable"|"m_FillCenter"|"m_PreserveAspect"|"m_FillClockwise"|"m_UseSpriteMesh"|"m_enableWordWrapping"|"m_isRichText" => Some("bool"),
        "m_Color"|"m_fontColor" => Some("Color"),
        "m_Type" => Some("ImageType"), "m_FillMethod" => Some("ImageFillMethod"),
        "m_FillAmount"|"m_PixelsPerUnitMultiplier"|"m_fontSize"|"m_fontSizeBase" => Some("float"),
        "m_Text"|"m_text" => Some("string"),
        "m_HorizontalAlignment"|"m_VerticalAlignment"|"m_textAlignment"|"m_FillOrigin" => Some("int"),
        _ => None,
    };
    ui.collapsing(crate::localization::tr("组件信息"),|ui| {
        for field in fields.iter().filter(|field|is_component_field(&field.name)&&field.name!="m_Enabled") {raw_field(ui,field,crate::localization::tr("组件信息"));}
    });
    for field in fields.iter().filter(|field|!is_component_field(&field.name)||field.name=="m_Enabled") {
        if !query.trim().is_empty() && !format!("{} {}",field.name,field.value).to_lowercase().contains(&query.trim().to_lowercase()) {continue}
        if let Some(kind)=kind(&field.name) {typed_field(ui,field,kind,&index);}
        else if field.name=="m_FontData" {
            ui.collapsing("Font",|ui|crate::editing::with_field(ui,"m_FontData",|ui| {
                let nested=field.value.lines().map(|line|line.strip_prefix("  ").unwrap_or(line)).collect::<Vec<_>>().join("\n");
                for child in yaml_fields(&nested) {
                    let kind=match child.name.as_str() {"m_FontSize"|"m_MinSize"|"m_MaxSize"=>Some("int"),"m_FontStyle"=>Some("FontStyle"),"m_Alignment"=>Some("TextAnchor"),"m_HorizontalOverflow"=>Some("HorizontalWrapMode"),"m_VerticalOverflow"=>Some("VerticalWrapMode"),"m_RichText"|"m_BestFit"|"m_AlignByGeometry"=>Some("bool"),"m_LineSpacing"=>Some("float"),_=>None};
                    if let Some(kind)=kind {typed_field(ui,&child,kind,&index);} else {raw_field(ui,&child,crate::localization::tr("原始值"));}
                }
            }));
        }
        else {raw_field(ui,field,crate::localization::tr("原始值"));}
    }
}
