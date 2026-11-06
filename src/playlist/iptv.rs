use super::Entry;
use std::{collections::HashMap,io::{Read,Write},path::{Path,PathBuf},sync::mpsc,time::{Duration,Instant}};
use sha2::{Digest,Sha256};
use eframe::egui;

pub(super) const CARD_WIDTH:f32=76.0;
const CARD_HEIGHT:f32=70.0;
const CARD_GAP:f32=8.0;
pub(super) struct CardGrid {pub columns:usize,pub height:f32}
impl CardGrid {
    pub fn new(width:f32,count:usize)->Self{
        let columns=(((width.max(0.0)+CARD_GAP)/(CARD_WIDTH+CARD_GAP)).floor() as usize).max(1).min(count.max(1));
        let rows=count.div_ceil(columns);
        Self{columns,height:rows as f32*CARD_HEIGHT+rows.saturating_sub(1) as f32*CARD_GAP}
    }
    pub fn card(&self,origin:egui::Pos2,index:usize)->egui::Rect{
        egui::Rect::from_min_size(origin+egui::vec2((index%self.columns) as f32*(CARD_WIDTH+CARD_GAP),(index/self.columns) as f32*(CARD_HEIGHT+CARD_GAP)),egui::vec2(CARD_WIDTH,CARD_HEIGHT))
    }
}

#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub(super) struct Metadata(pub Vec<(String,String)>);
impl Metadata {
    pub fn get(&self,key:&str)->&str {self.0.iter().find(|(name,_)|name.eq_ignore_ascii_case(key)).map_or("",|(_,value)|value.trim())}
    pub fn parse(mut text:&str)->Self {
        let mut fields=Vec::new();
        while !text.is_empty(){
            text=text.trim_start_matches(|ch:char|ch.is_whitespace() || ch==',');
            let end=text.find(|ch:char|ch.is_whitespace() || matches!(ch,'='|',')).unwrap_or(text.len());
            let key=&text[..end];text=&text[end..];text=text.trim_start();
            let Some(rest)=text.strip_prefix('=') else {if end==0{break;}continue;};
            text=rest.trim_start();
            let value;
            if let Some(quote)=text.chars().next().filter(|ch|matches!(ch,'"'|'\'')){
                text=&text[1..];let Some(end)=text.find(quote) else{break;};
                value=&text[..end];text=&text[end+1..];
            }else{
                let end=text.find(|ch:char|ch.is_whitespace() || ch==',').unwrap_or(text.len());
                value=&text[..end];text=&text[end..];
            }
            if !key.is_empty(){fields.push((key.to_owned(),value.to_owned()));}
        }
        Self(fields)
    }
    pub fn render(&self)->Result<String,String>{
        let mut result=String::new();
        for (key,value) in &self.0 {
            if key.is_empty() || !key.bytes().all(|ch|ch.is_ascii_alphanumeric() || matches!(ch,b'-'|b'_')) || value.contains(['\r','\n']){
                return Err(super::error("播放列表属性包含无效字符"));
            }
            let quote=if !value.contains('"'){'"'}else if !value.contains('\''){'\''}else{return Err(super::error("播放列表属性包含无效字符"));};
            result.push_str(&format!(" {key}={quote}{value}{quote}"));
        }
        Ok(result)
    }
}

pub(super) fn extinf(value:&str)->(Metadata,Option<String>){
    let Some((head,tail))=super::extinf_separator(value) else{return (Metadata::parse(value),None);};
    let attributes=tail.split_once('=').is_some_and(|(key,_)|{
        let key=key.trim();!key.is_empty() && key.chars().all(|ch|ch.is_ascii_alphanumeric() || matches!(ch,'-'|'_'))
    });
    let (metadata,name)=if attributes {
        match super::extinf_separator(tail){Some((metadata,name))=>(Metadata::parse(metadata),name.trim()),None=>(Metadata::parse(tail),"")}
    }else{(Metadata::parse(head),tail.trim())};
    (metadata,(!name.is_empty()).then(||name.to_owned()))
}

pub(super) struct Channel {pub key:String,pub title:String,pub sources:Vec<usize>,pub groups:Vec<String>,pub logos:Vec<String>}
#[derive(Default)]
pub(super) struct View {pub instance:u64,pub detected:bool,pub channels:Vec<Channel>,pub groups:Vec<String>,pub notices:Vec<String>,pub entry_channel:HashMap<usize,usize>}
impl View {
    pub fn new(entries:&[Entry])->Self {
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(1);
        let mut view=Self::default();view.instance=NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed);let mut keys=HashMap::<String,usize>::new();
        view.detected=entries.iter().any(|entry|super::remote_reference(&entry.path) && ["tvg-id","tvg-name","tvg-logo","group-title","radio"].iter().any(|key|!entry.metadata.get(key).is_empty()));
        if !view.detected{return view;}
        for (index,entry) in entries.iter().enumerate(){
            let group=entry.metadata.get("group-title");
            let stamp=entry.title.as_bytes();
            if group.contains("更新时间") && stamp.len()>=19 && stamp[0..4].iter().all(u8::is_ascii_digit) && stamp[4]==b'-' && stamp[7]==b'-' && stamp[10]==b' ' && stamp[13]==b':' && stamp[16]==b':' {
                if !view.notices.contains(&entry.title){view.notices.push(entry.title.clone());}continue;
            }
            let id=entry.metadata.get("tvg-id");let name=entry.metadata.get("tvg-name");
            let key=if !super::remote_reference(&entry.path){format!("local:{index}")}
                else if !id.is_empty(){format!("id:{id}")}else if !name.is_empty(){format!("name:{name}")}else{format!("title:{}",entry.title)};
            let channel=*keys.entry(key.clone()).or_insert_with(||{
                let current=view.channels.len();view.channels.push(Channel{key,title:entry.title.clone(),sources:Vec::new(),groups:Vec::new(),logos:Vec::new()});current
            });
            view.entry_channel.insert(index,channel);
            let channel=&mut view.channels[channel];
            if !channel.sources.iter().any(|source|entries[*source].path==entry.path){channel.sources.push(index);}
            if !channel.groups.iter().any(|value|value==group){channel.groups.push(group.to_owned());}
            if !view.groups.iter().any(|value|value==group){view.groups.push(group.to_owned());}
            let logo=entry.metadata.get("tvg-logo");
            if http_url(logo) && !channel.logos.iter().any(|value|value==logo){channel.logos.push(logo.to_owned());}
        }
        if view.channels.is_empty(){view.detected=false;}
        view
    }
    pub fn visible(&self,group:Option<&str>)->Vec<usize>{
        self.channels.iter().enumerate().filter(|(_,channel)|group.is_none_or(|group|channel.groups.iter().any(|value|value==group))).map(|(index,_)|index).collect()
    }
}

fn http_url(value:&str)->bool {reqwest::Url::parse(value).is_ok_and(|url|matches!(url.scheme(),"http"|"https") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none())}
enum Logo {Pending(mpsc::Receiver<Option<egui::ColorImage>>),Ready(egui::TextureHandle),Failed}
#[derive(Default)]
pub(super) struct Logos {items:HashMap<String,(Instant,Logo)>}
impl Logos {
    fn poll(&mut self,ctx:&egui::Context){
        // Finish offscreen requests too, or their occupied slots can stall new cards.
        for (url,(_,state)) in &mut self.items{
            if let Logo::Pending(receiver)=state{
                match receiver.try_recv(){
                    Ok(Some(image))=>*state=Logo::Ready(ctx.load_texture(format!("iptv-logo:{url}"),image,egui::TextureOptions::LINEAR)),
                    Ok(None)|Err(mpsc::TryRecvError::Disconnected)=>*state=Logo::Failed,
                    Err(mpsc::TryRecvError::Empty)=>{}
                }
            }
        }
    }
    pub fn get(&mut self,ui:&egui::Ui,candidates:&[String])->Option<egui::TextureHandle>{
        self.poll(ui.ctx());
        for url in candidates.iter().take(3){
            if let Some((used,state))=self.items.get_mut(url){
                *used=Instant::now();
                if let Logo::Pending(receiver)=state{
                    match receiver.try_recv(){
                        Ok(Some(image))=>*state=Logo::Ready(ui.ctx().load_texture(format!("iptv-logo:{url}"),image,egui::TextureOptions::LINEAR)),
                        Ok(None)|Err(mpsc::TryRecvError::Disconnected)=>*state=Logo::Failed,
                        Err(mpsc::TryRecvError::Empty)=>{ui.ctx().request_repaint_after(Duration::from_millis(100));return None;}
                    }
                }
                match state {Logo::Ready(texture)=>return Some(texture.clone()),Logo::Failed=>continue,Logo::Pending(_)=>return None}
            }
            if self.items.values().filter(|(_,state)|matches!(state,Logo::Pending(_))).count()>=4{return None;}
            if self.items.len()>=256{
                let victim=self.items.iter().filter(|(_,(_,state))|!matches!(state,Logo::Pending(_))).min_by_key(|(_, (used,_))|*used).map(|(key,_)|key.clone());
                if let Some(victim)=victim{self.items.remove(&victim);}else{return None;}
            }
            let (sender,receiver)=mpsc::channel();let address=url.clone();let ctx=ui.ctx().clone();
            self.items.insert(url.clone(),(Instant::now(),Logo::Pending(receiver)));
            std::thread::spawn(move||{let _=sender.send(download_logo(&address));ctx.request_repaint();});
            return None;
        }
        None
    }
}
fn download_logo(url:&str)->Option<egui::ColorImage>{
    if !http_url(url){return None;}
    let cached=cached_logo(url);
    if let Some((pixels,true))=&cached{return Some(color_image(pixels));}
    let pixels=fetch_logo(url).or_else(||cached.map(|(pixels,_)|pixels))?;
    Some(color_image(&pixels))
}
fn fetch_logo(url:&str)->Option<image::RgbaImage>{
    static CLIENT:std::sync::OnceLock<Option<reqwest::blocking::Client>>=std::sync::OnceLock::new();
    let client=CLIENT.get_or_init(||reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(3)).timeout(Duration::from_secs(8)).redirect(reqwest::redirect::Policy::limited(3)).build().ok()).as_ref()?;
    let response=client.get(url).send().ok()?.error_for_status().ok()?;
    if !http_url(response.url().as_str()) || response.content_length().is_some_and(|size|size>1024*1024){return None;}
    let mut bytes=Vec::new();response.take(1024*1024+1).read_to_end(&mut bytes).ok()?;
    if bytes.len()>1024*1024{return None;}
    let pixels=decode_logo(bytes)?;save_logo(url,&pixels);Some(pixels)
}
fn decode_logo(bytes:Vec<u8>)->Option<image::RgbaImage>{
    let mut reader=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
    let mut limits=image::Limits::default();limits.max_image_width=Some(2048);limits.max_image_height=Some(2048);limits.max_alloc=Some(16*1024*1024);reader.limits(limits);
    Some(reader.decode().ok()?.thumbnail(96,64).into_rgba8())
}
fn color_image(pixels:&image::RgbaImage)->egui::ColorImage{
    egui::ColorImage::from_rgba_unmultiplied([pixels.width() as usize,pixels.height() as usize],pixels.as_raw())
}
fn ordinary(path:&Path,directory:bool)->bool{
    let Ok(metadata)=std::fs::symlink_metadata(path) else{return false;};
    if metadata.file_type().is_symlink(){return false;}
    #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if metadata.file_attributes()&0x400!=0{return false;}}
    if directory{metadata.is_dir()}else{metadata.is_file()}
}
fn logo_path(root:&Path,url:&str)->PathBuf{root.join(format!("{:x}.png",Sha256::digest(url.as_bytes())))}
fn cached_logo(url:&str)->Option<(image::RgbaImage,bool)>{
    let root=crate::app_cache::root()?.join("iptv_logos");
    let (bytes,fresh)={
        let _guard=crate::app_cache::mutation_guard();if !ordinary(&root,true){return None;}
        let path=logo_path(&root,url);if !ordinary(&path,false){return None;}
        let metadata=std::fs::metadata(&path).ok()?;if metadata.len()>128*1024{return None;}
        let bytes=std::fs::read(&path).ok()?;
        let fresh=metadata.modified().ok()?.elapsed().ok().is_some_and(|age|age<Duration::from_secs(7*24*60*60));
        (bytes,fresh)
    };
    Some((decode_logo(bytes)?,fresh))
}
fn save_logo(url:&str,pixels:&image::RgbaImage){
    let Some(root)=crate::app_cache::root().map(|path|path.join("iptv_logos")) else{return;};
    let mut output=std::io::Cursor::new(Vec::new());
    if image::DynamicImage::ImageRgba8(pixels.clone()).write_to(&mut output,image::ImageFormat::Png).is_err(){return;}
    let _guard=crate::app_cache::mutation_guard();
    if std::fs::symlink_metadata(&root).is_ok() && !ordinary(&root,true){return;}
    if std::fs::create_dir_all(&root).is_err() || !ordinary(&root,true){return;}
    static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
    let destination=logo_path(&root,url);
    let temporary=root.join(format!(".logo-{}-{}.tmp",std::process::id(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
    let result=(||->std::io::Result<()>{
        let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&temporary)?;
        file.write_all(output.get_ref())?;drop(file);
        super::replace(&temporary,&destination).map_err(std::io::Error::other)
    })();
    if result.is_err(){let _=std::fs::remove_file(&temporary);}
    trim_logos(&root);
}
fn trim_logos(root:&Path){
    let Ok(entries)=std::fs::read_dir(root) else{return;};
    let mut files:Vec<_>=entries.filter_map(Result::ok).filter_map(|entry|{
        let name=entry.file_name();let name=name.to_str()?;
        if name.len()!=68 || !name.ends_with(".png") || !name.as_bytes()[..64].iter().all(u8::is_ascii_hexdigit) || !ordinary(&entry.path(),false){return None;}
        let metadata=entry.metadata().ok()?;Some((metadata.modified().ok(),metadata.len(),entry.path()))
    }).collect();
    files.sort_by_key(|(modified,_,_)|*modified);let mut count=files.len();let mut bytes:u64=files.iter().map(|(_,size,_)|*size).sum();
    for (_,size,path) in files{if count<=1024 && bytes<=32*1024*1024{break;}if std::fs::remove_file(path).is_ok(){count-=1;bytes=bytes.saturating_sub(size);}}
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn fixed_card_grid_uses_full_width_and_aligns_rows(){
        let origin=egui::pos2(12.0,40.0);
        for (width,columns) in [(159.0,1),(160.0,2),(243.0,2),(244.0,3),(328.0,4)]{
            let grid=CardGrid::new(width,19);assert_eq!(grid.columns,columns);
            for index in 0..19{
                let rect=grid.card(origin,index);assert_eq!(rect.width(),CARD_WIDTH);assert_eq!(rect.height(),70.0);
                assert!(rect.right()<=origin.x+width);
                if index%columns!=0{assert_eq!(rect.top(),grid.card(origin,index-1).top());}
                if index>=columns{assert_eq!(rect.left(),grid.card(origin,index-columns).left());assert_eq!(rect.top()-grid.card(origin,index-columns).top(),78.0);}
                assert!(rect.bottom()<=origin.y+grid.height);
            }
        }
        assert_eq!(CardGrid::new(280.0,0).height,0.0);
    }
    #[test]
    fn logo_cache_keys_and_bounded_bitmap_decode(){
        let root=Path::new("cache");let first=logo_path(root,"https://example.org/logo.png?a=1");
        assert_eq!(first.parent(),Some(root));assert_eq!(first.file_name().unwrap().to_string_lossy().len(),68);
        assert_ne!(first,logo_path(root,"https://example.org/logo.png?a=2"));
        assert!(decode_logo(b"not a logo".to_vec()).is_none());
        let image=image::RgbaImage::from_pixel(200,100,image::Rgba([1,2,3,255]));let mut data=std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image).write_to(&mut data,image::ImageFormat::Png).unwrap();
        let decoded=decode_logo(data.into_inner()).unwrap();assert_eq!(decoded.dimensions(),(96,48));
        assert!(!http_url("file:///logo.png"));assert!(!http_url("https://user:password@example.org/logo.png"));
    }
    #[test]
    fn finished_offscreen_logos_release_concurrency_slots(){
        let mut logos=Logos::default();
        for index in 0..4{
            let(sender,receiver)=mpsc::channel();sender.send(None).unwrap();
            logos.items.insert(index.to_string(),(Instant::now(),Logo::Pending(receiver)));
        }
        logos.poll(&egui::Context::default());
        assert!(logos.items.values().all(|(_,state)|matches!(state,Logo::Failed)));
    }
}
