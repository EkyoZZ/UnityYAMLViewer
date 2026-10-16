//! Single-Sprite settings and staged in-place replacement. No persistent backups.
use std::{fs, io::{Read, Seek, Write}, path::{Path, PathBuf}, time::SystemTime};
use crate::localization::tr;

#[derive(Clone)]
pub(super) struct Meta {
    pub path: PathBuf,
    text: String,
}

fn companion(image: &Path) -> PathBuf {
    let mut name=image.as_os_str().to_owned(); name.push(".meta"); name.into()
}


fn field<'a>(text: &'a str, key: &str) -> Result<&'a str,String> {
    let prefix=format!("  {key}: ");
    let mut matches=text.lines().filter_map(|line|line.strip_prefix(&prefix));
    let value=matches.next().ok_or_else(||tr("不支持此 .meta 格式或 Sprite 模式").to_owned())?;
    if matches.next().is_some() { return Err(tr("不支持此 .meta 格式或 Sprite 模式").into()); }
    Ok(value.split('#').next().unwrap_or("").trim())
}

fn validate(text: &str) -> Result<(),String> {
    let plain=text.trim_start_matches('\u{feff}');
    let guid=regex::Regex::new(r"(?m)^guid: [0-9a-fA-F]{32}\r?$").unwrap();
    if plain.lines().filter(|line|*line=="TextureImporter:").count()!=1 || guid.find_iter(plain).count()!=1
        || field(plain,"textureType")? != "8" || field(plain,"spriteMode")? != "1" {
        return Err(tr("仅支持单 Sprite 图片的 .meta；不修改多 Sprite 图集或其他纹理类型。").into());
    }
    field(plain,"spriteBorder")?;
    field(plain,"spriteMeshType")?;
    Ok(())
}

fn border_text([l,t,r,b,_,_]: [u32;6]) -> String {format!("{{x: {l}, y: {b}, z: {r}, w: {t}}}")}

fn patch(text: &str, values: [u32;6]) -> Result<String,String> {
    validate(text)?;
    let mut output=String::with_capacity(text.len()+64);
    // Retain BOM, line endings and all unrelated import settings byte-for-byte.
    for line in text.split_inclusive('\n') {
        let content=line.trim_end_matches(['\r','\n']);
        let ending=&line[content.len()..];
        if content.starts_with("  spriteBorder: ") {
            output.push_str(&format!("  spriteBorder: {}{ending}",border_text(values)));
        } else if content.starts_with("  spriteMeshType: ") {
            output.push_str(&format!("  spriteMeshType: 0{ending}")); // FullRect = 0.
        } else { output.push_str(line); }
    }
    Ok(output)
}

fn unique_id() -> String {
    use sha2::{Digest,Sha256};
    static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
    let seed=format!("{:?}-{}-{}",SystemTime::now(),std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed));
    format!("{:x}",Sha256::digest(seed.as_bytes()))[..32].to_owned()
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(),String> {
    let mut file=fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e|e.to_string())?;
    if let Err(error)=file.write_all(bytes).and_then(|_|file.sync_all()) {
        drop(file); let _=fs::remove_file(path); return Err(error.to_string());
    }
    Ok(())
}

impl Meta {
    // Missing, unreadable, oversized or non-UTF-8 companions deliberately produce no UI hint.
    pub fn read(image: &Path) -> Option<Self> {
        let path=companion(image);
        let info=fs::symlink_metadata(&path).ok()?;
        if !info.is_file() || info.len()>4*1024*1024 {return None;}
        let text=fs::read_to_string(&path).ok()?;
        Some(Self {path,text})
    }
    pub fn supported(&self) -> Result<(),String> {validate(&self.text)}
    pub fn borders(&self, w: u32, h: u32) -> Option<[u32;4]> {
        self.supported().ok()?;
        let text=field(&self.text,"spriteBorder").ok()?;
        let regex=regex::Regex::new(r"^\{x: ([0-9.]+), y: ([0-9.]+), z: ([0-9.]+), w: ([0-9.]+)\}$").ok()?;
        let groups=regex.captures(text)?;
        let mut xyzw=[0u32;4];
        for i in 0..4 {
            let value=groups[i+1].parse::<f64>().ok()?;
            if !value.is_finite() || value.fract()!=0. || value>8192. {return None;}
            xyzw[i]=value as u32;
        }
        let [l,b,r,t]=xyzw;
        (l.checked_add(r)?<w && t.checked_add(b)?<h).then_some([l,t,r,b])
    }

}

pub(super) fn export(image: &image::RgbaImage, target: &Path) -> Result<(),String> {
    let meta=companion(target);
    // Refuse even an orphan companion to avoid binding a new PNG to an unrelated asset GUID.
    if target.exists() || meta.exists() { return Err(tr("目标文件已存在，请使用新文件名。").into()); }
    let mut bytes=std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image.clone()).write_to(&mut bytes,image::ImageFormat::Png).map_err(|e|e.to_string())?;
    write_new(target,bytes.get_ref())
}

struct Staging { root: PathBuf }
impl Staging {
    fn new(parent: &Path) -> Result<Self,String> {
        fs::create_dir_all(parent).map_err(|e|e.to_string())?;
        let root=parent.join(unique_id());
        fs::create_dir(&root).map_err(|e|e.to_string())?;
        Ok(Self {root})
    }
    fn cleanup(&self) -> Result<(),String> {
        for name in ["image.tmp","meta.tmp"] {
            match fs::remove_file(self.root.join(name)) {
                Ok(())=>{}, Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{}, Err(e)=>return Err(e.to_string()),
            }
        }
        match fs::remove_dir(&self.root) {
            Ok(())=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(e)=>Err(e.to_string()),
        }
    }
}
impl Drop for Staging {fn drop(&mut self) {let _=self.cleanup();}}

fn open_existing(path: &Path) -> Result<fs::File,String> {
    let info=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !info.is_file() {return Err(tr("资源路径不是普通文件，无法替换。").into());}
    let mut options=fs::OpenOptions::new(); options.read(true).write(true);
    #[cfg(windows)] {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(path).map_err(|e|e.to_string())
}
fn overwrite(file: &mut fs::File, bytes: &[u8]) -> Result<(),String> {
    file.rewind().and_then(|_|file.write_all(bytes)).and_then(|_|file.set_len(bytes.len() as u64)).and_then(|_|file.sync_all()).map_err(|e|e.to_string())
}

pub(super) struct Replacement { pub changed: bool, pub message: String }

pub(super) fn replace_plan(path: &Path, original: &image::RgbaImage, pixels: &image::RgbaImage, request: &super::Request, plan: &super::Plan, meta: Option<&Meta>, staging_root: &Path) -> Replacement {
    let _cache_guard=crate::app_cache::mutation_guard();
    let mut changed=false;
    let mut stage=None;
    let result=(|| -> Result<(),String> {
        let expected=super::Plan::new(original,request);
        if &expected!=plan || expected.render(original)!=*pixels {
            return Err(tr("区域并非完全重复的 RGBA 行列，未替换图片。").into());
        }
        let format=image::ImageFormat::from_path(path).map_err(|e|e.to_string())?;
        if !matches!(format,image::ImageFormat::Png|image::ImageFormat::Tga|image::ImageFormat::Bmp|image::ImageFormat::WebP) {
            return Err(tr("此格式不支持无损原位替换，请另存 PNG。").into());
        }
        stage=Some(Staging::new(staging_root)?);
        let temp=&stage.as_ref().unwrap().root;
        let mut bytes=std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(pixels.clone()).write_to(&mut bytes,format).map_err(|e|e.to_string())?;
        if image::load_from_memory_with_format(bytes.get_ref(),format).map_err(|e|e.to_string())?.to_rgba8()!=*pixels {
            return Err(tr("此格式不支持无损原位替换，请另存 PNG。").into());
        }
        let updated=meta.map(|meta|patch(&meta.text,plan.borders())).transpose()?;
        write_new(&temp.join("image.tmp"),bytes.get_ref())?;
        if let Some(updated)=&updated {write_new(&temp.join("meta.tmp"),updated.as_bytes())?;}
        // Open and lock both destinations before changing either file.
        let mut image_file=open_existing(path)?;
        let mut meta_file=meta.map(|meta|open_existing(&meta.path)).transpose()?;
        let mut old=Vec::new();
        image_file.read_to_end(&mut old).map_err(|e|e.to_string())?;
        let animated=match format {
            image::ImageFormat::Png=>image::codecs::png::PngDecoder::new(std::io::Cursor::new(&old)).and_then(|decoder|decoder.is_apng()).map_err(|e|e.to_string())?,
            image::ImageFormat::WebP=>image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(&old)).map_err(|e|e.to_string())?.has_animation(),
            _=>false,
        };
        if animated {return Err(tr("动画图片不支持原位精简，请另存静态 PNG。").into());}
        let mut reader=image::ImageReader::new(std::io::Cursor::new(&old)).with_guessed_format().map_err(|e|e.to_string())?;
        let mut limits=image::Limits::default(); limits.max_image_width=Some(8192); limits.max_image_height=Some(8192); limits.max_alloc=Some(256*1024*1024); reader.limits(limits);
        if reader.decode().map_err(|e|e.to_string())?.to_rgba8()!=*original {return Err(tr("资源已在外部改变，请重载后再操作。").into());}
        if let (Some(file),Some(expected))=(&mut meta_file,meta) {
            let mut actual=String::new(); file.read_to_string(&mut actual).map_err(|e|e.to_string())?;
            if actual!=expected.text {return Err(tr("资源已在外部改变，请重载后再操作。").into());}
        }
        let commit=overwrite(&mut image_file,bytes.get_ref()).and_then(|_| {
            if let (Some(file),Some(updated))=(&mut meta_file,&updated) {overwrite(file,updated.as_bytes())} else {Ok(())}
        });
        if let Err(error)=commit {
            // Roll back from memory only; no backup files are created.
            let restore_image=overwrite(&mut image_file,&old);
            let restore_meta=if let (Some(file),Some(meta))=(&mut meta_file,meta) {overwrite(file,meta.text.as_bytes())} else {Ok(())};
            if restore_image.is_err() || restore_meta.is_err() {
                changed=true;
                return Err(format!("{}: {error}; {:?}; {:?}",tr("替换失败且回滚未完成，请检查 Git 中的资源修改。"),restore_image.err(),restore_meta.err()));
            }
            return Err(error);
        }
        changed=true; Ok(())
    })();
    let mut message=match result {Ok(())=>tr("已原位替换，页签已重新加载。").to_owned(),Err(e)=>format!("{}: {e}",tr("原位替换失败"))};
    if let Some(stage)=stage {
        if let Err(error)=stage.cleanup() {message.push_str(&format!("\n{}: {} ({error})",tr("临时文件清理失败"),stage.root.display()));}
    }
    Replacement {changed,message}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replace(path:&Path,original:&image::RgbaImage,pixels:&image::RgbaImage,v:[u32;6],meta:Option<&Meta>,root:&Path)->Replacement {
        let request=super::super::Request {borders:Some([v[0],v[1],v[2],v[3]]),center:[v[4],v[5]],multiple_of_four:false,..Default::default()};
        let plan=super::super::Plan::new(original,&request);
        replace_plan(path,original,pixels,&request,&plan,meta,root)
    }
    const SAMPLE:&str="fileFormatVersion: 2\nguid: 0123456789abcdef0123456789abcdef\nTextureImporter:\n  spriteMode: 1\n  spriteMeshType: 1\n  spriteBorder: {x: 2, y: 4, z: 3, w: 1}\n  textureType: 8\n  spritePixelsToUnits: 100\n  spriteSheet:\n    border: {x: 0, y: 0, z: 0, w: 0}\n";
    #[test]
    fn changes_only_border_and_mesh_and_preserves_bom_crlf_guid() {
        let src=format!("\u{feff}{}",SAMPLE.replace('\n',"\r\n"));
        let out=patch(&src,[5,6,7,8,1,1]).unwrap();
        assert_eq!(out,src.replace("spriteMeshType: 1","spriteMeshType: 0").replace("spriteBorder: {x: 2, y: 4, z: 3, w: 1}","spriteBorder: {x: 5, y: 8, z: 7, w: 6}"));
        assert!(patch(&SAMPLE.replace("spriteMode: 1","spriteMode: 2"),[0;6]).is_err());
        assert!(patch(&SAMPLE.replace("textureType: 8","textureType: 0"),[0;6]).is_err());
        assert!(patch(&(SAMPLE.to_owned()+"  spriteBorder: {}\n"),[0;6]).is_err());
    }
    #[test]
    fn reads_unity_border_order() {
        let meta=Meta {path:PathBuf::new(),text:SAMPLE.into()};
        assert_eq!(meta.borders(40,30),Some([2,1,3,4]));
    }
    #[test]
    fn replacement_cleans_staging_keeps_guid_and_refuses_stale_pixels() {
        let root=std::env::temp_dir().join(format!("nine-slice-test-{}",unique_id()));
        fs::create_dir(&root).unwrap();
        let image_path=root.join("ui.png");
        let original=image::RgbaImage::from_pixel(20,20,image::Rgba([10,20,30,128]));
        original.save(&image_path).unwrap();fs::write(companion(&image_path),SAMPLE).unwrap();
        let meta=Meta::read(&image_path).unwrap();
        let pixels=super::super::compact(&original,[2,1,3,4,1,1]);let staging=root.join("records");
        let result=replace(&image_path,&original,&pixels,[2,1,3,4,1,1],Some(&meta),&staging);
        assert!(result.changed,"{}",result.message);
        assert_eq!(image::open(&image_path).unwrap().to_rgba8(),pixels);
        let expected=patch(SAMPLE,[2,1,3,4,1,1]).unwrap();
        assert_eq!(fs::read_to_string(companion(&image_path)).unwrap(),expected);
        assert_eq!(fs::read_dir(&staging).unwrap().count(),0);
        assert!(!replace(&image_path,&original,&pixels,[2,1,3,4,1,1],Some(&meta),&staging).changed);
        assert_eq!(fs::read_to_string(companion(&image_path)).unwrap(),expected);
        assert_eq!(fs::read_dir(&staging).unwrap().count(),0);
        for entry in fs::read_dir(&root).unwrap() {let path=entry.unwrap().path();if path.is_file(){fs::remove_file(path).unwrap();}}
        fs::remove_dir(staging).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn stale_meta_and_unwritable_staging_leave_the_image_untouched() {
        let root=std::env::temp_dir().join(format!("nine-slice-test-{}",unique_id()));fs::create_dir(&root).unwrap();
        let path=root.join("ui.png");
        let source=image::RgbaImage::from_pixel(12,10,image::Rgba([40,50,60,255]));source.save(&path).unwrap();
        fs::write(companion(&path),SAMPLE).unwrap();let meta=Meta::read(&path).unwrap();
        let bytes=fs::read(&path).unwrap();
        let pixels=super::super::compact(&source,[1,1,1,1,1,1]);
        fs::write(companion(&path),SAMPLE.replace("spritePixelsToUnits: 100","spritePixelsToUnits: 200")).unwrap();
        let staging=root.join("records");
        assert!(!replace(&path,&source,&pixels,[1,1,1,1,1,1],Some(&meta),&staging).changed);
        assert_eq!(fs::read(&path).unwrap(),bytes);assert_eq!(fs::read_dir(&staging).unwrap().count(),0);
        fs::remove_dir(&staging).unwrap();fs::write(&staging,b"not a directory").unwrap();
        assert!(!replace(&path,&source,&pixels,[1,1,1,1,1,1],None,&staging).changed);assert_eq!(fs::read(&path).unwrap(),bytes);
        for name in [&path,&companion(&path),&staging] {fs::remove_file(name).unwrap();}fs::remove_dir(root).unwrap();
    }
}
