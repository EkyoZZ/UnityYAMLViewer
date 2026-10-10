//! Static uGUI identification and RectTransform layout. Runtime layout scripts are not executed.
use std::collections::{HashMap, HashSet};
use crate::{Document, static_preview::{Matrix, local, field, number, reference, vector}};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Graphic { Image, RawImage, Text, Tmp }
pub fn graphic(doc: &Document) -> Option<Graphic> {
    if doc.type_name != "MonoBehaviour" { return None; }
    // Inspector classification must be bounded even for a one-line IsoMap array.
    let mut end=doc.text.len().min(64*1024);
    while !doc.text.is_char_boundary(end) {end-=1;}
    let prefix=&doc.text[..end];
    let header_end=prefix.match_indices('\n').nth(127).map_or(prefix.len(),|(offset,_)|offset+1);
    let text=&prefix[..header_end];
    let header_field=|key|field(text,key);
    let guid = header_field("m_Script").and_then(|value|reference(value,"guid"));
    match guid.as_deref() {
        Some("fe87c0e1cc204ed48ad3b37840f39efc") => return Some(Graphic::Image),
        Some("1344c3c82d62a2a41a3576d8abb8e3ea") => return Some(Graphic::RawImage),
        Some("5f7201a12d95ffc409449d95f23cf332") => return Some(Graphic::Text),
        Some("f4688fdb7df04437aeb418b961361dc5") => return Some(Graphic::Tmp),
        _ => {},
    }
    // Also recognize serialized Image/Text layouts in derived components.
    if header_field("m_Sprite").is_some() && header_field("m_FillCenter").is_some() && header_field("m_Type").is_some() {Some(Graphic::Image)}
    else if header_field("m_Texture").is_some() && header_field("m_UVRect").is_some() {Some(Graphic::RawImage)}
    else if header_field("m_Text").is_some() && header_field("m_FontData").is_some() {Some(Graphic::Text)}
    else if header_field("m_text").is_some() && header_field("m_fontSize").is_some() {Some(Graphic::Tmp)}
    else {None}
}
#[derive(Clone, Copy)]
pub struct Layout { pub matrix: Matrix, pub size: [f64;2], pub pivot: [f64;2] }
pub fn seed_screen_canvases(documents: &[Document], viewport: [f64;2], cache: &mut HashMap<String,Layout>) {
    for canvas in documents.iter().filter(|doc| doc.type_name == "Canvas"
        && number(&doc.text,"m_RenderMode",0.0) != 2.0 && number(&doc.text,"m_Enabled",1.0) != 0.0) {
        let Some(root) = documents.iter().find(|doc| doc.type_name == "RectTransform"
            && doc.game_object == canvas.game_object
            && doc.parent_transform.as_deref().is_none_or(|id| id == "0")) else { continue; };
        if documents.iter().any(|doc| doc.type_name == "GameObject" && Some(&doc.file_id) == root.game_object.as_ref()
            && number(&doc.text,"m_IsActive",1.0) == 0.0) { continue; }
        // Screen-space Canvas dimensions and scale are driven by Unity, not the serialized root values.
        let mut normalized = root.clone();
        normalized.text = root.text.lines().map(|line| {
            if line.starts_with("  m_LocalScale:") { "  m_LocalScale: {x: 1, y: 1, z: 1}" } else { line }
        }).collect::<Vec<_>>().join("\n").into();
        let pivot = vector(&root.text,"m_Pivot",&["x","y"],&[0.5,0.5]);
        cache.insert(root.file_id.clone(), Layout { matrix: local(&normalized,true), size: viewport, pivot: [pivot[0],pivot[1]] });
    }
}
pub fn layout(id: &str, transforms: &HashMap<&str,&Document>, objects: &HashMap<&str,&Document>, viewport: [f64;2], cache: &mut HashMap<String,Layout>, visiting: &mut HashSet<String>) -> Option<Layout> {
    if let Some(layout)=cache.get(id) {return Some(*layout)}
    if !visiting.insert(id.to_owned()) {return None}
    let doc=transforms.get(id)?;
    if doc.game_object.as_deref().and_then(|id|objects.get(id)).is_some_and(|doc|number(&doc.text,"m_IsActive",1.0)==0.0) {return None}
    let parent=doc.parent_transform.as_deref().filter(|id|*id!="0");
    let parent_layout=if let Some(parent)=parent {Some(layout(parent,transforms,objects,viewport,cache,visiting)?)} else {None};
    let mut matrix=local(doc,parent.is_none());
    let mut size=viewport; let mut pivot=[0.5,0.5];
    if doc.type_name=="RectTransform" {
        let delta=vector(&doc.text,"m_SizeDelta",&["x","y"],&[0.0,0.0]);
        let p=vector(&doc.text,"m_Pivot",&["x","y"],&[0.5,0.5]);pivot=[p[0],p[1]];
        if let Some(parent)=parent_layout {
            let min=vector(&doc.text,"m_AnchorMin",&["x","y"],&[0.5,0.5]);
            let max=vector(&doc.text,"m_AnchorMax",&["x","y"],&[0.5,0.5]);
            let position=vector(&doc.text,"m_AnchoredPosition",&["x","y"],&[0.0,0.0]);
            for axis in 0..2 {
                size[axis]=parent.size[axis]*(max[axis]-min[axis])+delta[axis];
                matrix.0[axis][3]=parent.size[axis]*(min[axis]+(max[axis]-min[axis])*pivot[axis]-parent.pivot[axis])+position[axis];
            }
        } else {
            for axis in 0..2 {if delta[axis]>0.0 {size[axis]=delta[axis];}}
        }
    }
    if let Some(parent)=parent_layout {matrix=parent.matrix.mul(matrix)}
    visiting.remove(id);
    let result=Layout{matrix,size,pivot};cache.insert(id.to_owned(),result);Some(result)
}
pub fn nested_number(text: &str, key: &str, default: f64) -> f64 {
    text.lines().find_map(|line|line.trim().strip_prefix(key)?.strip_prefix(':')?.trim().parse::<f64>().ok()).filter(|v|v.is_finite()).unwrap_or(default)
}
pub fn text_content(doc: &Document, kind: Graphic) -> String {
    let key=if kind==Graphic::Tmp {"m_text"} else {"m_Text"};
    let mut value=field(&doc.text,key).map(|v|crate::yaml_strings::display(v).into_owned()).unwrap_or_default();
    if let Some(raw)=field(&doc.text,key).filter(|v|matches!(*v,"|"|"|-"|"|+"|">"|">-")) {
        let marker=format!("  {key}: {raw}");
        let mut lines=doc.text.lines().skip_while(|line|*line!=marker).skip(1);
        value=lines.by_ref().take_while(|line|line.starts_with("    ")||line.trim().is_empty()).map(|line|line.strip_prefix("    ").unwrap_or("")).collect::<Vec<_>>().join("\n").into();
    }
    let rich=if kind==Graphic::Tmp {number(&doc.text,"m_isRichText",1.0)} else {nested_number(&doc.text,"m_RichText",1.0)};
    if rich!=0.0 {
        static TAGS: std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
        value=TAGS.get_or_init(||regex::Regex::new(r"</?(?:b|i|u|s|color|size|font|sprite|align|alpha|mark|voffset|space|cspace|link)(?:[ =][^>]*)?>").unwrap()).replace_all(&value,"").into_owned();
        value=value.replace("<br>","\n").replace("<br/>","\n");
    }
    value
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_large_arrays_do_not_trigger_ui_header_detection() {
        let text=format!("  m_Script: {{fileID: 11500000, guid: custom}}\n  data: {}\n  m_Sprite: {{fileID: 0}}\n  m_FillCenter: 1\n  m_Type: 0\n","0".repeat(70*1024));
        let mut doc=Document {file_id:"1".into(),type_name:"MonoBehaviour".into(),start_line:1,start_offset:0,text:text.into(),game_object:None,parent_transform:None};
        assert_eq!(graphic(&doc),None);
        doc.text="  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc}".into();
        assert_eq!(graphic(&doc),Some(Graphic::Image));
    }
    #[test]
    fn screen_canvas_zero_serialized_scale_uses_reference_layout() {
        let root=Document { file_id:"1".into(), type_name:"RectTransform".into(), start_line:1,start_offset:0,
            text:"  m_LocalScale: {x: 0, y: 0, z: 0}\n  m_SizeDelta: {x: 0, y: 0}\n  m_Pivot: {x: 0, y: 0}\n".into(),
            game_object:Some("10".into()), parent_transform:Some("0".into()) };
        let canvas=Document { file_id:"2".into(), type_name:"Canvas".into(), start_line:1,start_offset:0,
            text:"  m_RenderMode: 1\n  m_Enabled: 1\n".into(), game_object:Some("10".into()), parent_transform:None };
        let original=root.text.clone();
        let mut cache=HashMap::new();
        seed_screen_canvases(&[root.clone(),canvas.clone()],[864.0,1536.0],&mut cache);
        assert_eq!(cache["1"].size,[864.0,1536.0]);
        assert_eq!(cache["1"].matrix.point(432.0,768.0),[432.0,768.0]);
        assert_eq!(root.text,original);
        let mut world=canvas; world.text="  m_RenderMode: 2\n".into();
        cache.clear(); seed_screen_canvases(&[root,world],[864.0,1536.0],&mut cache);
        assert!(cache.is_empty());
    }
    #[test]
    fn text_uses_decoded_content_and_ignores_basic_rich_text_tags() {
        let doc=Document{file_id:"10".into(),type_name:"MonoBehaviour".into(),start_line:1,start_offset:0,text:"  m_Text: \"<b>hello</b>\\nworld\"\n  m_FontData:\n    m_RichText: 1\n".into(),game_object:None,parent_transform:None};
        assert_eq!(graphic(&doc),Some(Graphic::Text));
        assert_eq!(text_content(&doc,Graphic::Text),"hello\nworld");
    }
    #[test]
    fn stretched_child_uses_anchors_parent_pivot_and_size_delta() {
        let root=Document{file_id:"1".into(),type_name:"RectTransform".into(),start_line:1,start_offset:0,text:"  m_SizeDelta: {x: 800, y: 600}\n  m_Pivot: {x: 0.5, y: 0.5}\n".into(),game_object:None,parent_transform:None};
        let child=Document{file_id:"2".into(),type_name:"RectTransform".into(),start_line:1,start_offset:0,text:"  m_AnchorMin: {x: 0, y: 0}\n  m_AnchorMax: {x: 1, y: 1}\n  m_SizeDelta: {x: -40, y: -60}\n  m_AnchoredPosition: {x: 10, y: 20}\n  m_Pivot: {x: 0.5, y: 0.5}\n".into(),game_object:None,parent_transform:Some("1".into())};
        let result=layout("2",&HashMap::from([("1",&root),("2",&child)]),&HashMap::new(),[1280.0,720.0],&mut HashMap::new(),&mut HashSet::new()).unwrap();
        assert_eq!(result.size,[760.0,540.0]);assert_eq!(result.matrix.point(0.0,0.0),[10.0,20.0]);
    }
}
/// uGUI draws parent graphics before descendants, following m_Children sibling order.
pub fn hierarchy_order(documents: &[Document]) -> HashMap<String,i64> {
    let transforms: HashMap<_,_>=documents.iter().filter(|doc|matches!(doc.type_name.as_str(),"Transform"|"RectTransform")).map(|doc|(doc.file_id.as_str(),doc)).collect();
    let mut children: HashMap<&str,Vec<&str>>=HashMap::new();let mut roots=Vec::new();
    for doc in documents.iter().filter(|doc|transforms.contains_key(doc.file_id.as_str())) {
        if let Some(parent)=doc.parent_transform.as_deref().filter(|id|*id!="0") {children.entry(parent).or_default().push(&doc.file_id);} else {roots.push(doc.file_id.as_str());}
    }
    for (parent,items) in &mut children {
        let listed=transforms.get(parent).map(|doc| {
            doc.text.lines().skip_while(|line|!line.starts_with("  m_Children:")).skip(1)
                .take_while(|line|line.trim_start().starts_with('-')||line.starts_with("    "))
                .filter_map(|line|reference(line.trim().trim_start_matches("- "),"fileID")).collect::<Vec<_>>()
        }).unwrap_or_default();
        items.sort_by_key(|id|listed.iter().position(|listed|listed==id).unwrap_or(usize::MAX));
    }
    let mut result=HashMap::new();let mut visited=HashSet::new();let mut pending:Vec<_>=roots.into_iter().rev().collect();let mut rank=0;
    while let Some(id)=pending.pop() {
        if !visited.insert(id) {continue}
        if let Some(object)=transforms[id].game_object.as_deref() {result.insert(object.to_owned(),rank);rank+=1;}
        if let Some(items)=children.get(id) {pending.extend(items.iter().rev().copied());}
    }
    result
}
/// Canvas enable state and CanvasGroup alpha propagate down the hierarchy.
pub fn opacity(id: &str, documents: &[Document], transforms: &HashMap<&str,&Document>) -> f64 {
    let mut current=Some(id);let mut alpha=1.0;let mut visited=HashSet::new();let mut ignore_groups=false;
    while let Some(id)=current {
        if !visited.insert(id) {return 0.0}
        let Some(transform)=transforms.get(id) else {break};
        if let Some(object)=transform.game_object.as_deref() {
            for component in documents.iter().filter(|doc|doc.game_object.as_deref()==Some(object)) {
                if component.type_name=="Canvas" && number(&component.text,"m_Enabled",1.0)==0.0 {return 0.0}
                if component.type_name=="CanvasGroup" && number(&component.text,"m_Enabled",1.0)!=0.0 && !ignore_groups {
                    alpha*=number(&component.text,"m_Alpha",1.0).clamp(0.0,1.0);
                    ignore_groups=number(&component.text,"m_IgnoreParentGroups",0.0)!=0.0;
                }
            }
        }
        current=transform.parent_transform.as_deref().filter(|id|*id!="0");
    }
    alpha
}
