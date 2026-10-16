//! Headless entry, deliberately executed before GUI single-instance forwarding.
use super::{meta,processing::{self,Direction,Request,Treatment}};
use std::{path::{Path,PathBuf},collections::BTreeSet};
use serde_json::{json,Value};
pub const HELP:&str=r#"Unity YAML Viewer — slice9

Open files: unity_yaml_viewer.exe <file> [file ...]
Help:       unity_yaml_viewer.exe --help

Open one file:    unity_yaml_viewer.exe "a.png"
Open many files:  unity_yaml_viewer.exe "a.png" "b.prefab" "c.mat"
Simple paths without spaces or shell-special characters do not require quotes.
Quote each path containing spaces or shell-special characters separately.
Examples use double quotes for easy copying; escaping depends on your shell.
In PowerShell, use single quotes for literal paths containing $ or backticks.

slice9 <file-or-directory> [...] [options]
  -r, --recursive       Include subdirectories (no symbolic links)
  --preview             Open automatic GUI previews (max 10 unique images); no automatic writes
  --dry-run             Report plans without writing
  --center WxH          Retained Stretch center, default 4x4
  --repeat-count XxY    Retained Repeat cycles, default 1x1
  --direction MODE      auto (default), both, horizontal, vertical
  --mode MODE           auto (default), stretch, repeat
  --no-preserve-edges   Allow trimming transparent margins
  --no-align4           Disable multiple-of-four alignment
  --report FILE         Also save JSON report (must be a new file)
  -h, --help            Show this help
  --                    Treat remaining arguments as paths

Examples (PowerShell):
  .\unity_yaml_viewer.exe slice9 "button.png"
  .\unity_yaml_viewer.exe slice9 "a.png" "b.png"
  .\unity_yaml_viewer.exe slice9 "Assets\UI" -r
  .\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
  .\unity_yaml_viewer.exe slice9 "button.png" --center 8x4
  .\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
Preview: reuse existing dialogs; at most 10 open at once; extra images are reported, not opened.
Preview supports center/cycle/edge/alignment options, but uses automatic axes and treatment.
--preview cannot combine with --dry-run, --report or explicit direction/mode.

DEFAULT: exact RGBA auto detection; IN-PLACE overwrite, NO BACKUP.
Readable supported single-Sprite .meta: update Border / Full Rect, keep GUID.
Absent/unreadable .meta: image only, never create .meta.
Readable unsupported .meta: fail that image without modifying it.
PNG/TGA/BMP/static WebP only; other directory entries are skipped.
No area reduction: leave image and meta unchanged. One failure does not stop a batch.
Padding is balanced, odd extra on right/bottom, excluded from Border.
Output: JSON on stdout. Exit 0: no failures; 1: file failures; 2: arguments/report error.
Windows PowerShell: use Start-Process -Wait -PassThru to reliably wait for this GUI exe.
"#;
#[derive(Default)]
struct Options {paths:Vec<PathBuf>,request:Request,recursive:bool,dry:bool,preview:bool,report:Option<PathBuf>,direction:Option<Direction>,mode:Option<Treatment>}
fn pair(value:&str)->Result<[u32;2],String> {
    let (a,b)=value.split_once('x').ok_or("Expected positive WxH, e.g. 4x4")?;
    let pair=[a.parse::<u32>().map_err(|_|"Invalid width")?,b.parse::<u32>().map_err(|_|"Invalid height")?];
    if pair.iter().any(|v|*v==0||*v>8192) {return Err("Values must be in 1..8192".into())} Ok(pair)
}
fn parse(args:Vec<std::ffi::OsString>)->Result<Options,String> {
    let mut o=Options::default();let mut args=args.into_iter();let mut literal=false;
    while let Some(arg)=args.next() {
        if literal {o.paths.push(arg.into());continue}
        let text=arg.to_string_lossy();
        let mut value=||args.next().ok_or_else(||format!("Missing value for {text}"));
        match text.as_ref() {
            "--"=>literal=true,"-r"|"--recursive"=>o.recursive=true,"--dry-run"=>o.dry=true,"--preview"=>o.preview=true,
            "--no-preserve-edges"=>o.request.preserve_transparent=false,"--no-align4"=>o.request.multiple_of_four=false,
            "--center"=>o.request.center=pair(&value()?.to_string_lossy())?,
            "--repeat-count"=>o.request.copies=pair(&value()?.to_string_lossy())?,
            "--report"=>o.report=Some(value()?.into()),
            "--direction"=>o.direction=match value()?.to_string_lossy().as_ref() {"auto"=>None,"both"=>Some(Direction::Both),"horizontal"=>Some(Direction::Horizontal),"vertical"=>Some(Direction::Vertical),_=>return Err("Invalid direction".into())},
            "--mode"=>o.mode=match value()?.to_string_lossy().as_ref() {"auto"=>None,"stretch"=>Some(Treatment::Stretch),"repeat"=>Some(Treatment::Repeat),_=>return Err("Invalid mode".into())},
            _ if text.starts_with('-')=>return Err(format!("Unknown option: {text}")),_=>o.paths.push(arg.into()),
        }
    }
    if o.preview&&(o.dry||o.report.is_some()||o.direction.is_some()||o.mode.is_some()){return Err("--preview uses automatic detection; cannot combine with --dry-run, --report, explicit --direction or --mode".into())}
    if o.paths.is_empty(){return Err("Provide at least one file or directory".into())} Ok(o)
}
fn supported(path:&Path)->bool {matches!(image::ImageFormat::from_path(path),Ok(image::ImageFormat::Png|image::ImageFormat::Tga|image::ImageFormat::Bmp|image::ImageFormat::WebP))}
fn collect(path:&Path,recursive:bool,files:&mut BTreeSet<PathBuf>,records:&mut Vec<Value>) {
    let result=(||->Result<(),String>{
        let metadata=std::fs::symlink_metadata(path).map_err(|e|e.to_string())?;
        if metadata.file_type().is_symlink() {records.push(json!({"path":path,"status":"skipped","reason":"symbolic link"}));return Ok(())}
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).map_err(|e|e.to_string())? {
                match entry {Ok(entry)=>{let p=entry.path();if p.is_dir()&&!recursive {continue} collect(&p,recursive,files,records)},Err(e)=>records.push(json!({"path":path,"status":"failed","reason":e.to_string()}))}
            }
        }else if metadata.is_file()&&supported(path) {let absolute=if path.is_absolute(){path.to_owned()}else{std::env::current_dir().map_err(|e|e.to_string())?.join(path)};files.insert(std::fs::canonicalize(&absolute).unwrap_or(absolute));}
        else {records.push(json!({"path":path,"status":"skipped","reason":"unsupported file"}));} Ok(())
    })();if let Err(e)=result {records.push(json!({"path":path,"status":"failed","reason":e}));}
}
fn process(path:&Path,o:&Options)->Result<Value,String> {
    let format=image::ImageFormat::from_path(path).map_err(|e|e.to_string())?;
    let file=std::io::BufReader::new(std::fs::File::open(path).map_err(|e|e.to_string())?);
    let animated=match format {
        image::ImageFormat::Png=>image::codecs::png::PngDecoder::new(file).and_then(|d|d.is_apng()).map_err(|e|e.to_string())?,
        image::ImageFormat::WebP=>image::codecs::webp::WebPDecoder::new(file).map_err(|e|e.to_string())?.has_animation(),
        _=>false,
    };
    if animated {return Err("Animated images cannot be replaced in place".into())}
    let mut reader=image::ImageReader::open(path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
    let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
    let source=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
    let metadata=meta::Meta::read(path);if let Some(meta)=&metadata {meta.supported()?;}
    let mut template=o.request.clone();if let Some(direction)=o.direction {template.direction=direction} if let Some(mode)=o.mode {template.treatment=mode}
    let (mut request,mut plan,trials)=processing::automatic(&source,&template);
    if o.direction.is_some()||o.mode.is_some() {
        let best=trials.iter().filter(|t|(t.eligible||o.mode==Some(Treatment::Repeat))&&o.direction.is_none_or(|d|d==t.direction)&&o.mode.is_none_or(|m|m==t.treatment)).min_by_key(|t|(u64::from(t.size[0])*u64::from(t.size[1]),t.treatment==Treatment::Repeat));
        if let Some(best)=best {request=template.clone();request.direction=best.direction;request.treatment=best.treatment;plan=processing::Plan::new(&source,&request);}
    }
    let before=u64::from(source.width())*u64::from(source.height());let size=plan.size();let after=u64::from(size[0])*u64::from(size[1]);
    let mut status=if after<before {"planned"}else{"unchanged"};
    if after<before&&!o.dry {
        let root=crate::app_cache::root().ok_or("Cannot locate application records directory")?;
        let result=meta::replace_plan(path,&source,&plan.render(&source),&request,&plan,metadata.as_ref(),&root.join("image_edit_temp"));
        if !result.changed {return Err(result.message)} status="replaced";
    }
    let b=plan.borders();
    Ok(json!({"path":path,"status":status,"source_size":[source.width(),source.height()],"output_size":size,"saved_area":before as i64-after as i64,"direction":format!("{:?}",request.direction),"mode":format!("{:?}",request.treatment),"border_lbrt":[b[0],b[3],b[2],b[1]],"padding_ltrb":plan.padding_sides(),"meta":metadata.as_ref().map(|m|&m.path),"meta_written":status=="replaced"&&metadata.is_some(),"reasons":plan.reports.iter().map(|r|r.reason.label()).collect::<Vec<_>>()}))
}
pub fn dispatch()->Option<i32> {
    let mut args=std::env::args_os().skip(1);let first=args.next()?;
    if first!="slice9"&&first!="--help"&&first!="-h" {return None}
    attach_console();
    let rest:Vec<_>=args.collect();
    if first!="slice9"||rest.iter().take_while(|a|a.as_os_str()!=std::ffi::OsStr::new("--")).any(|a|a=="--help"||a=="-h") {println!("{HELP}");return Some(0)}
    if rest.iter().take_while(|a|a.as_os_str()!=std::ffi::OsStr::new("--")).any(|a|a=="--preview") {return match parse(rest.clone()) {Ok(_)=>None,Err(e)=>{eprintln!("{e}");Some(2)}}}
    let options=match parse(rest) {Ok(o)=>o,Err(e)=>{eprintln!("{e}\nUse slice9 --help");return Some(2)}};
    // Reserve the report before changing any source files; never overwrite an existing file.
    let mut report=match &options.report {Some(p)=>match std::fs::OpenOptions::new().write(true).create_new(true).open(p){Ok(f)=>Some(f),Err(e)=>{eprintln!("Report: {e}");return Some(2)}},None=>None};
    let mut files=BTreeSet::new();let mut records=Vec::new();for p in &options.paths {collect(p,options.recursive,&mut files,&mut records)}
    for path in files {records.push(process(&path,&options).unwrap_or_else(|e|json!({"path":path,"status":"failed","reason":e})));}
    let count=|status:&str|records.iter().filter(|r|r["status"]==status).count();
    let failed=count("failed");let output=json!({"dry_run":options.dry,"summary":{"replaced":count("replaced"),"planned":count("planned"),"unchanged":count("unchanged"),"skipped":count("skipped"),"failed":failed},"files":records});
    let text=serde_json::to_string_pretty(&output).unwrap();println!("{text}");
    if let Some(file)=&mut report {use std::io::Write;if let Err(e)=file.write_all(text.as_bytes()).and_then(|_|file.sync_all()){eprintln!("Report: {e}");return Some(2)}}
    Some(if failed>0 {1}else{0})
}
#[cfg(windows)]
fn attach_console(){#[link(name="kernel32")] unsafe extern "system" {fn AttachConsole(process:u32)->i32;} unsafe {AttachConsole(u32::MAX);}}
#[cfg(not(windows))] fn attach_console(){}
#[cfg(test)] mod tests {use super::*;#[test]fn arguments_defaults_and_overrides(){let o=parse(vec!["a.png".into(),"-r".into(),"--dry-run".into(),"--center".into(),"8x4".into()]).unwrap();assert!(o.recursive&&o.dry);assert_eq!(o.request.center,[8,4]);assert!(parse(vec!["--center".into(),"0x4".into()]).is_err());assert!(parse(vec!["--bad".into()]).is_err());}}

#[cfg(test)]
mod batch_tests {
    use super::*;
    #[test]
    fn recursive_collection_deduplicates_and_dry_run_keeps_source() {
        let root=std::env::temp_dir().join(format!("slice9-cli-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let image=image::RgbaImage::from_pixel(40,20,image::Rgba([1,2,3,255]));
        let path=root.join("a.png");image.save(&path).unwrap();image.save(root.join("sub/b.png")).unwrap();
        std::fs::write(root.join("notes.txt"),"skip").unwrap();
        let before=std::fs::read(&path).unwrap();
        let mut files=BTreeSet::new();let mut records=Vec::new();collect(&root,false,&mut files,&mut records);assert_eq!(files.len(),1,"root={root:?}; records={records:?}");
        collect(&root,true,&mut files,&mut records);collect(&path,true,&mut files,&mut records);assert_eq!(files.len(),2);
        let o=Options {dry:true,..Default::default()};let result=process(&path,&o).unwrap();assert_eq!(result["status"],"planned");assert_eq!(std::fs::read(&path).unwrap(),before);assert!(!path.with_extension("png.meta").exists());
        let o=Options {dry:true,direction:Some(Direction::Horizontal),mode:Some(Treatment::Stretch),..Default::default()};let result=process(&path,&o).unwrap();assert_eq!(result["direction"],"Horizontal");assert_eq!(result["output_size"][1],20);
        std::fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn preview_request(args:Vec<std::ffi::OsString>)->Result<crate::command_line::OpenRequest,String> {
    let o=parse(args)?;if !o.preview {return Err("Expected --preview".into())}
    let mut files=BTreeSet::new();let mut records=Vec::new();for p in &o.paths {collect(p,o.recursive,&mut files,&mut records)}
    let omitted=files.len().saturating_sub(10);let mut request=o.request;request.automatic=true;
    let failures=records.iter().filter(|r|r["status"]=="failed").count();
    let notice=if omitted>0||failures>0 {format!("{}: {omitted}; {}: {failures}",crate::localization::tr("超出预览上限，未打开"),crate::localization::tr("无法访问的路径"))}else{String::new()};
    if files.is_empty(){return Err("No supported images found for preview".into())}
    Ok(crate::command_line::OpenRequest {paths:files.into_iter().take(10).collect(),preview:Some(request),notice})
}
#[cfg(test)] mod preview_tests {
    use super::*;
    #[test] fn preview_limits_deduplicates_and_never_changes_input() {
        assert!(parse(vec!["a.png".into(),"--preview".into(),"--dry-run".into()]).is_err());
        let root=std::env::temp_dir().join(format!("slice9-preview-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&root).unwrap();
        for i in 0..12 {std::fs::write(root.join(format!("{i:02}.png")),b"fixture").unwrap();}
        let request=preview_request(vec![root.clone().into_os_string(),root.clone().into_os_string(),"--preview".into()]).unwrap();
        assert_eq!(request.paths.len(),10);assert!(request.preview.as_ref().unwrap().automatic);assert!(!request.notice.is_empty());
        let bytes=serde_json::to_vec(&request).unwrap();let decoded:crate::command_line::OpenRequest=serde_json::from_slice(&bytes).unwrap();assert_eq!(decoded.paths,request.paths);
        for i in 0..12 {assert_eq!(std::fs::read(root.join(format!("{i:02}.png"))).unwrap(),b"fixture");}
        std::fs::remove_dir_all(root).unwrap();
    }
}
