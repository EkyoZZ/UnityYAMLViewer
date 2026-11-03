//! Source image facts and reversible PNG-only cleanup. Never infer source depth from RGBA preview pixels.
use std::{path::{Path,PathBuf},sync::{Arc,mpsc},io::{Cursor,Write},time::SystemTime};
use eframe::egui;
use image::ImageDecoder;
use crate::localization::tr;
const LIMIT:usize=128*1024*1024;
const SIGNATURE:&[u8]=b"\x89PNG\r\n\x1a\n";
#[derive(Clone)]struct Info{bytes:Arc<Vec<u8>>,format:String,width:u32,height:u32,bits:u16,alpha:bool,opaque:bool,dpi:Option<(f64,f64)>,system_dpi:bool,itxt:usize,indexed:bool,animated:bool}
#[derive(Clone,Copy)]enum Action{Load,Alpha,Text,Undo,Redo}
struct ResultData{info:Info,before:Option<Arc<Vec<u8>>>,action:Action}
#[derive(Default)]pub(super) struct State{path:PathBuf,stamp:Option<(u64,SystemTime)>,info:Option<Info>,pending:Option<mpsc::Receiver<Result<ResultData,String>>>,history:super::image_history::History<Arc<Vec<u8>>>,error:Option<String>}
fn crc(bytes:&[u8])->u32{let mut value=0xffffffffu32;for byte in bytes{value^=*byte as u32;for _ in 0..8{value=(value>>1)^if value&1!=0{0xedb88320}else{0};}}!value}
fn chunks(bytes:&[u8])->Result<Vec<(usize,usize,[u8;4])>,String>{
    if !bytes.starts_with(SIGNATURE){return Err("Not PNG".into());}let mut result=Vec::new();let mut offset=8;let mut ended=false;
    while offset+12<=bytes.len(){let length=u32::from_be_bytes(bytes[offset..offset+4].try_into().unwrap()) as usize;
        let end=offset.checked_add(length).and_then(|n|n.checked_add(12)).filter(|n|*n<=bytes.len()).ok_or("Invalid PNG chunk length")?;
        let kind=bytes[offset+4..offset+8].try_into().unwrap();
        if crc(&bytes[offset+4..end-4])!=u32::from_be_bytes(bytes[end-4..end].try_into().unwrap()){return Err("PNG CRC mismatch".into());}
        result.push((offset,end,kind));offset=end;if &kind==b"IEND"{ended=true;break;}
    }
    if !ended||offset!=bytes.len(){return Err("Invalid PNG end".into());}Ok(result)
}
fn inspect(bytes:Arc<Vec<u8>>)->Result<Info,String>{
    if bytes.len()>=26&&bytes.starts_with(b"8BPS"){
        let channels=u16::from_be_bytes(bytes[12..14].try_into().unwrap());let depth=u16::from_be_bytes(bytes[22..24].try_into().unwrap());
        return Ok(Info{bytes:bytes.clone(),format:"PSD".into(),width:u32::from_be_bytes(bytes[18..22].try_into().unwrap()),height:u32::from_be_bytes(bytes[14..18].try_into().unwrap()),bits:channels.saturating_mul(depth),alpha:false,opaque:false,dpi:None,system_dpi:false,itxt:0,indexed:false,animated:false});
    }
    let reader=image::ImageReader::new(Cursor::new(bytes.as_slice())).with_guessed_format().map_err(|e|e.to_string())?;
    let format=reader.format().map(|f|format!("{f:?}")).unwrap_or_default();let decoder=reader.into_decoder().map_err(|e|e.to_string())?;
    let (width,height)=decoder.dimensions();let color=decoder.original_color_type();
    let mut info=Info{bytes:bytes.clone(),format,width,height,bits:color.bits_per_pixel(),alpha:decoder.color_type().has_alpha(),opaque:false,dpi:None,system_dpi:false,itxt:0,indexed:false,animated:false};
    if bytes.starts_with(SIGNATURE){
        for (start,end,kind) in chunks(&bytes)?{let data=&bytes[start+8..end-4];match &kind{
            b"IHDR" if data.len()==13=>{let channels=match data[9]{0|3=>1,2=>3,4=>2,6=>4,_=>return Err("Invalid PNG color type".into())};info.bits=data[8] as u16*channels;info.indexed=data[9]==3;info.alpha=matches!(data[9],4|6);},
            b"tRNS"=>info.alpha=true,
            b"pHYs" if data.len()==9&&data[8]==1=>{info.dpi=Some((u32::from_be_bytes(data[..4].try_into().unwrap()) as f64*0.0254,u32::from_be_bytes(data[4..8].try_into().unwrap()) as f64*0.0254));},
            b"iTXt"=>info.itxt+=data.len(),b"acTL"=>info.animated=true,_=>{}
        }}
    }else if bytes.starts_with(&[0xff,0xd8]){
        let mut position=2;
        while position+4<=bytes.len(){if bytes[position]!=0xff{break;}let marker=bytes[position+1];if marker==0xda||marker==0xd9{break;}let length=u16::from_be_bytes([bytes[position+2],bytes[position+3]]) as usize;
            if length<2||position+2+length>bytes.len(){break;}let data=&bytes[position+4..position+2+length];
            if marker==0xe0&&data.len()>=12&&data.starts_with(b"JFIF\0"){let factor=match data[7]{1=>1.0,2=>2.54,_=>0.0};if factor>0.0{info.dpi=Some((u16::from_be_bytes([data[8],data[9]]) as f64*factor,u16::from_be_bytes([data[10],data[11]]) as f64*factor));}}
            position+=2+length;
        }
    }
    if info.dpi.is_some_and(|(x,y)|x<=0.0||y<=0.0){info.dpi=None;}
    if info.dpi.is_none(){info.dpi=super::image_dpi::windows_resolution(&bytes);info.system_dpi=info.dpi.is_some();}
    if info.alpha&&info.bits!=24&&!info.animated&&bytes.starts_with(SIGNATURE){
        let color=decoder.color_type();let size=decoder.total_bytes();
        if size<=256*1024*1024{
            let mut pixels=vec![0;size as usize];
            if decoder.read_image(&mut pixels).is_ok(){info.opaque=opaque_pixels(color,&pixels);}
        }
    }
    Ok(info)
}
fn opaque_pixels(color:image::ColorType,pixels:&[u8])->bool{
    let (stride,alpha_bytes)=match color{image::ColorType::Rgba8=>(4,1),image::ColorType::La8=>(2,1),image::ColorType::Rgba16=>(8,2),image::ColorType::La16=>(4,2),_=>return false};
    !pixels.is_empty()&&pixels.len()%stride==0&&pixels.chunks_exact(stride).all(|pixel|pixel[stride-alpha_bytes..].iter().all(|value|*value==255))
}
fn transform(info:&Info,action:Action)->Result<Vec<u8>,String>{
    let original=&info.bytes;let source=chunks(original)?;
    if matches!(action,Action::Text){let mut output=SIGNATURE.to_vec();for (start,end,kind) in source{if &kind!=b"iTXt"{output.extend_from_slice(&original[start..end]);}}return Ok(output);}
    if !info.opaque{return Err(tr("图片存在透明像素或未能确认完全不透明，不能去除 Alpha。").into());}
    if info.animated{return Err(tr("动画 PNG 暂不支持去除 Alpha。").into());}
    let header=source.iter().find(|(_,_,kind)|kind==b"IHDR").ok_or("Missing IHDR")?;let color=original[header.0+17];
    if matches!(color,0|2|3){let mut output=SIGNATURE.to_vec();for (start,end,kind) in source{if &kind!=b"tRNS"{output.extend_from_slice(&original[start..end]);}}return Ok(output);}
    let mut reader=png::Decoder::new(Cursor::new(original.as_slice())).read_info().map_err(|e|e.to_string())?;
    let size=reader.output_buffer_size().filter(|n|*n<=256*1024*1024).ok_or("PNG pixel buffer too large")?;let mut pixels=vec![0;size];let frame=reader.next_frame(&mut pixels).map_err(|e|e.to_string())?;
    let channels=if color==6{4}else if color==4{2}else{return Err("Unsupported PNG color".into())};let sample=if frame.bit_depth==png::BitDepth::Sixteen{2}else{1};
    let mut opaque=Vec::with_capacity(frame.buffer_size());for pixel in pixels[..frame.buffer_size()].chunks_exact(channels*sample){opaque.extend_from_slice(&pixel[..(channels-1)*sample]);}
    let mut encoded=Vec::new();{let mut encoder=png::Encoder::new(&mut encoded,frame.width,frame.height);encoder.set_depth(frame.bit_depth);encoder.set_color(if channels==4{png::ColorType::Rgb}else{png::ColorType::Grayscale});let mut writer=encoder.write_header().map_err(|e|e.to_string())?;writer.write_image_data(&opaque).map_err(|e|e.to_string())?;}
    let mut output=SIGNATURE.to_vec();let mut ancillary_written=false;for (start,end,kind) in chunks(&encoded)?{
        if &kind==b"IDAT"&&!ancillary_written{
            ancillary_written=true;
            // Preserve ancillary chunks whose semantics remain valid after alpha removal.
            for (old_start,old_end,old_kind) in &source{if matches!(old_kind,b"pHYs"|b"iCCP"|b"sRGB"|b"gAMA"|b"cHRM"|b"tEXt"|b"zTXt"|b"iTXt"|b"eXIf"|b"bKGD"){output.extend_from_slice(&original[*old_start..*old_end]);}}
        }output.extend_from_slice(&encoded[start..end]);
    }Ok(output)
}
fn replace(path:&Path,before:&[u8],after:&[u8])->Result<(),String>{
    if std::fs::read(path).map_err(|e|e.to_string())?!=before{return Err(tr("图片已被其他程序修改，请重载后重试。").into());}
    let temporary=path.with_file_name(format!(".uyv-image-{}-{}.tmp",std::process::id(),SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    let result=(||{let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e|e.to_string())?;file.write_all(after).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;drop(file);
        if std::fs::read(path).map_err(|e|e.to_string())?!=before{return Err(tr("图片已被其他程序修改，请重载后重试。").into());}
        std::fs::rename(&temporary,path).map_err(|e|e.to_string())})();if result.is_err(){let _=std::fs::remove_file(temporary);}result
}
impl State{
    fn start(&mut self,ctx:&egui::Context,path:&Path,action:Action){
        if self.pending.is_some(){return;}self.error=None;let previous=self.info.clone();let restore=match action{Action::Undo=>self.history.undo.back().cloned(),Action::Redo=>self.history.redo.last().cloned(),_=>None};
        let path=path.to_owned();let ctx=ctx.clone();let (tx,rx)=mpsc::channel();self.pending=Some(rx);
        std::thread::spawn(move||{let result=(||{
            if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>LIMIT as u64{return Err(tr("图片信息读取超过 128 MiB 限制。").into());}
            if matches!(action,Action::Load){return Ok(ResultData{info:inspect(Arc::new(std::fs::read(&path).map_err(|e|e.to_string())?))?,before:None,action});}
            let previous=previous.ok_or("Image not loaded")?;let after=if let Some(restore)=restore{restore}else{Arc::new(transform(&previous,action)?)};
            if after.len()>LIMIT{return Err(tr("图片信息读取超过 128 MiB 限制。").into());}
            let info=inspect(after.clone())?;replace(&path,&previous.bytes,&after)?;Ok(ResultData{info,before:Some(previous.bytes),action})
        })();let _=tx.send(result);ctx.request_repaint();});
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path,enabled:bool)->bool{
        let stamp=std::fs::metadata(path).ok().and_then(|m|m.modified().ok().map(|time|(m.len(),time)));let mut changed=false;
        if self.path!=path{*self=Self::default();self.path=path.to_owned();}
        if let Some(result)=self.pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.pending=None;match result{Ok(result)=>{
                if let Some(before)=result.before{match result.action{Action::Undo=>{self.history.undo(&before);},Action::Redo=>{self.history.redo(&before);},_=>self.history.record(before,&result.info.bytes)}changed=true;}
                self.info=Some(result.info);self.stamp=std::fs::metadata(path).ok().and_then(|m|m.modified().ok().map(|time|(m.len(),time)));
                while self.history.undo.iter().chain(self.history.redo.iter()).map(|b|b.len()).sum::<usize>()>128*1024*1024{if self.history.undo.pop_front().is_none(){self.history.redo.clear();break;}}
            },Err(error)=>{self.error=Some(error);self.stamp=stamp;}}
        }
        if self.pending.is_none()&&self.stamp!=stamp{self.history=Default::default();self.info=None;self.start(ui.ctx(),path,Action::Load);}
        if self.pending.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
        let mut action=None;
        if let Some(info)=&self.info{ui.horizontal_wrapped(|ui|{
            let number=|value:f64|{let value=format!("{value:.1}");value.trim_end_matches('0').trim_end_matches('.').to_owned()};
            let dpi=info.dpi.map(|(x,y)|if (x-y).abs()<0.1{format!("dpi: {}",number(x))}else{format!("dpi: {} × {}",number(x),number(y))}).unwrap_or_else(||format!("dpi: {}",tr("未记录或未解析")));
            ui.label(format!("{} · {} × {} · {} bit{} · {:.1} KiB · {}{}",info.format,info.width,info.height,info.bits,if info.indexed{format!(" ({})",tr("索引色"))}else{String::new()},info.bytes.len() as f64/1024.0,dpi,if info.alpha{" · Alpha"}else{""})).on_hover_text(if info.system_dpi{format!("{}\n{}",tr("Windows 读取值"),tr("文件密度记录优先；Windows 读取值可能来自系统默认值，不会写入图片。"))}else{tr("文件记录值").to_owned()});
            if info.bytes.starts_with(SIGNATURE){ui.label(format!("iTXt: {:.1} KiB",info.itxt as f64/1024.0));
                if info.alpha&&info.opaque&&info.bits!=24&&!info.animated&&ui.add_enabled(enabled&&self.pending.is_none(),egui::Button::new(tr("去除 Alpha"))).on_hover_text(tr("已确认所有像素完全不透明，移除多余 Alpha 通道；可撤销。")).clicked(){action=Some(Action::Alpha);}
                if info.itxt>0&&ui.add_enabled(enabled&&self.pending.is_none(),egui::Button::new(tr("去除 iTXt 元信息"))).clicked(){action=Some(Action::Text);}
            }
            if !self.history.undo.is_empty()&&ui.add_enabled(enabled&&self.pending.is_none(),egui::Button::new(tr("撤销"))).clicked(){action=Some(Action::Undo);}
            if !self.history.redo.is_empty()&&ui.add_enabled(enabled&&self.pending.is_none(),egui::Button::new(tr("重做"))).clicked(){action=Some(Action::Redo);}
        });}
        if let Some(action)=action{self.start(ui.ctx(),path,action);}
        if let Some(error)=&self.error{ui.colored_label(ui.visuals().warn_fg_color,error);}changed
    }
}

#[cfg(test)]mod tests{
    use super::*;
    fn chunk(kind:&[u8;4],data:&[u8])->Vec<u8>{let mut out=(data.len() as u32).to_be_bytes().to_vec();out.extend_from_slice(kind);out.extend_from_slice(data);out.extend_from_slice(&crc(&out[4..]).to_be_bytes());out}
    fn sample(depth:png::BitDepth)->Info{
        let mut out=Vec::new();{let mut encoder=png::Encoder::new(&mut out,1,1);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(depth);let mut writer=encoder.write_header().unwrap();writer.write_image_data(if depth==png::BitDepth::Sixteen{&[1,2,3,4,5,6,255,255]}else{&[12,34,56,255]}).unwrap();}
        let mut tagged=SIGNATURE.to_vec();for(start,end,kind)in chunks(&out).unwrap(){if &kind==b"IDAT"{let mut text=b"Comment\0\0\0\0\0".to_vec();text.extend(vec![b'x';11000]);tagged.extend(chunk(b"iTXt",&text));tagged.extend(chunk(b"pHYs",&[0,0,11,19,0,0,11,19,1]));}tagged.extend_from_slice(&out[start..end]);}inspect(Arc::new(tagged)).unwrap()
    }
    #[test]fn png_info_uses_source_depth_density_and_stored_text_size(){let info=sample(png::BitDepth::Eight);assert_eq!(info.bits,32);assert!(info.alpha);assert!(info.itxt>10*1024);assert!((info.dpi.unwrap().0-72.0).abs()<0.1);}
    #[test]fn text_removal_preserves_every_other_chunk(){let info=sample(png::BitDepth::Eight);let result=transform(&info,Action::Text).unwrap();let cleaned=inspect(Arc::new(result.clone())).unwrap();assert_eq!(cleaned.itxt,0);let expected=chunks(&info.bytes).unwrap().into_iter().filter(|(_,_,kind)|kind!=b"iTXt").map(|(s,e,_)|info.bytes[s..e].to_vec()).collect::<Vec<_>>();let actual=chunks(&result).unwrap().into_iter().map(|(s,e,_)|result[s..e].to_vec()).collect::<Vec<_>>();assert_eq!(expected,actual);}
    #[test]fn alpha_removal_keeps_rgb_samples_and_metadata_at_both_depths(){for depth in [png::BitDepth::Eight,png::BitDepth::Sixteen]{let info=sample(depth);let result=transform(&info,Action::Alpha).unwrap();let cleaned=inspect(Arc::new(result.clone())).unwrap();assert!(!cleaned.alpha);assert_eq!(cleaned.bits,if depth==png::BitDepth::Eight{24}else{48});assert_eq!(cleaned.itxt,info.itxt);assert_eq!(cleaned.dpi,info.dpi);let mut reader=png::Decoder::new(Cursor::new(result)).read_info().unwrap();let mut data=vec![0;reader.output_buffer_size().unwrap()];reader.next_frame(&mut data).unwrap();assert_eq!(data,if depth==png::BitDepth::Eight{vec![12,34,56]}else{vec![1,2,3,4,5,6]});}}
    #[test]fn opacity_requires_every_alpha_sample_to_be_maximum(){
        assert!(opaque_pixels(image::ColorType::Rgba8,&[1,2,3,255]));
        for alpha in [0,128,254]{assert!(!opaque_pixels(image::ColorType::Rgba8,&[1,2,3,255,4,5,6,alpha]));}
        assert!(opaque_pixels(image::ColorType::Rgba16,&[1,2,3,4,5,6,255,255]));
        assert!(!opaque_pixels(image::ColorType::Rgba16,&[1,2,3,4,5,6,255,254]));
        assert!(!opaque_pixels(image::ColorType::Rgb8,&[1,2,3]));
    }
    #[test]fn corrupted_png_is_rejected(){let info=sample(png::BitDepth::Eight);let mut bad=info.bytes.as_ref().clone();bad[16]^=1;assert!(chunks(&bad).is_err());}
}
