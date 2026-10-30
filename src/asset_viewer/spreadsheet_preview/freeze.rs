//! Best-effort XLSX freeze metadata. Does not interpret ordinary split-pane positions as freezes.
use std::{io::Read,path::Path};
use quick_xml::{Reader,events::{Event,BytesStart}};
use crate::delimited_table::FreezeSettings;
const LIMIT:u64=4*1024*1024;
fn attr(e:&BytesStart<'_>,key:&str)->Option<String>{
    e.attributes().flatten().find(|a|a.key.local_name().as_ref()==key)?.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|v|v.into_owned())
}
fn part<R:Read+std::io::Seek>(zip:&mut zip::ZipArchive<R>,name:&str)->Option<Vec<u8>>{
    let mut bytes=Vec::new();zip.by_name(name).ok()?.take(LIMIT+1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64<=LIMIT).then_some(bytes)
}
fn relationship_path(target:&str)->Option<String>{
    if target.contains([':', '\\']){return None;}
    let mut parts=if target.starts_with('/') {Vec::new()}else{vec!["xl"]};
    for item in target.split('/') {match item{""|"."=>{},".."=>{parts.pop()?;},_=>parts.push(item)}}
    Some(parts.join("/"))
}
fn count(text:Option<String>,max:usize)->Option<usize>{
    let Some(text)=text else{return Some(0);};let n=text.trim().parse::<f64>().ok()?;
    (n.is_finite()&&n>=0.0&&n.fract()==0.0&&n<=max as f64).then_some(n as usize)
}
fn pane(e:&BytesStart<'_>)->Option<FreezeSettings>{
    if !matches!(attr(e,"state").as_deref(),Some("frozen"|"frozenSplit")){return None;}
    let rows=count(attr(e,"ySplit"),1_048_576)?;
    let columns=count(attr(e,"xSplit"),16_384)?;
    (rows>0||columns>0).then_some(FreezeSettings{rows,columns})
}
pub(super) fn read(path:&Path,sheet:&str)->Option<FreezeSettings>{
    if path.extension()?.to_str()?.to_ascii_lowercase()!="xlsx"{return None;}
    let mut zip=zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
    let workbook=part(&mut zip,"xl/workbook.xml")?;let mut reader=Reader::from_reader(workbook.as_slice());let mut id=None;
    loop{match reader.read_event().ok()?{
        Event::Start(e)|Event::Empty(e) if e.local_name().as_ref()=="sheet"=>{
            if attr(&e,"name").as_deref()==Some(sheet){id=attr(&e,"id");break;}
        },Event::Eof=>break,_=>{}
    }}
    let id=id?;let rels=part(&mut zip,"xl/_rels/workbook.xml.rels")?;let mut reader=Reader::from_reader(rels.as_slice());let mut target=None;
    loop{match reader.read_event().ok()?{
        Event::Start(e)|Event::Empty(e) if e.local_name().as_ref()=="Relationship"=>{
            if attr(&e,"Id").as_deref()==Some(&id){
                if attr(&e,"TargetMode").as_deref()==Some("External"){return None;}
                target=attr(&e,"Target").and_then(|t|relationship_path(&t));break;
            }
        },Event::Eof=>break,_=>{}
    }}
    // Views precede sheetData; stream only that prefix instead of copying the whole sheet.
    let file=zip.by_name(&target?).ok()?;let mut reader=Reader::from_reader(std::io::BufReader::new(file.take(LIMIT)));let mut buffer=Vec::new();let mut in_views=false;
    loop{match reader.read_event_into(&mut buffer).ok()?{
        Event::Start(e) if e.local_name().as_ref()=="sheetViews"=>in_views=true,
        Event::Start(e)|Event::Empty(e) if in_views&&e.local_name().as_ref()=="pane"=>{if let Some(settings)=pane(&e){return Some(settings);}},
        Event::End(e) if e.local_name().as_ref()=="sheetViews"=>return None,
        Event::Start(e) if e.local_name().as_ref()=="sheetData"=>return None,
        Event::Eof=>return None,_=>{}
    }buffer.clear();}
}
#[cfg(test)]
mod tests{
    use super::*;
    fn parse(text:&str)->Option<FreezeSettings>{let mut r=Reader::from_reader(text.as_bytes());match r.read_event().unwrap(){Event::Empty(e)|Event::Start(e)=>pane(&e),_=>None}}
    #[test]fn frozen_counts_ignore_scroll_position_and_reject_plain_splits(){
        assert_eq!(parse(r#"<pane state="frozen" xSplit="2" ySplit="3" topLeftCell="D200"/>"#),Some(FreezeSettings{rows:3,columns:2}));
        assert_eq!(parse(r#"<pane state="split" xSplit="1200" ySplit="900"/>"#),None);
        assert_eq!(parse(r#"<pane state="frozenSplit" ySplit="1"/>"#),Some(FreezeSettings{rows:1,columns:0}));
        for text in [r#"<pane state="frozen" ySplit="-1"/>"#,r#"<pane state="frozen" ySplit="NaN"/>"#,r#"<pane state="frozen" ySplit="1.5"/>"#]{assert_eq!(parse(text),None);}
    }
    #[test]fn relationship_paths_stay_inside_zip_and_allow_root_relative_targets(){
        assert_eq!(relationship_path("worksheets/sheet1.xml").as_deref(),Some("xl/worksheets/sheet1.xml"));
        assert_eq!(relationship_path("/xl/worksheets/sheet2.xml").as_deref(),Some("xl/worksheets/sheet2.xml"));
        assert!(relationship_path("../../outside.xml").is_none());assert!(relationship_path("https://example.test/sheet.xml").is_none());
    }
}
