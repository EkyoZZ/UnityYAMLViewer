//! External FFmpeg decoding with bounded frame/audio queues. No codec is bundled.
use std::{io::{Read,BufRead}, path::{Path,PathBuf}, process::{Child,Stdio}, sync::{Arc,Mutex,mpsc,atomic::{AtomicBool,AtomicU32,Ordering}}, time::{Duration,Instant}};
use eframe::egui;
use crate::media_tools::{command,find_tool};

pub fn fullscreen_controls_visible(ctx:&egui::Context)->bool {
    let (now,active)=ctx.input(|input|(input.time,input.pointer.any_down()||input.events.iter().any(|event|matches!(event,egui::Event::PointerMoved(_)|egui::Event::MouseWheel {..}|egui::Event::Key {pressed:true,..}|egui::Event::Text(_)))));
    let editing=ctx.wants_keyboard_input();
    let last=ctx.data_mut(|data|{let id=egui::Id::new("media_last_motion");let last=if active||editing{now}else{data.get_temp::<f64>(id).unwrap_or(now)};data.insert_temp(id,last);last});
    let visible=now-last<1.0;
    if visible{ctx.request_repaint_after(Duration::from_secs_f64((1.0-(now-last)).max(0.001)));}
    visible
}
pub fn is_fullscreen(ctx:&egui::Context)->bool {ctx.data(|data|data.get_temp::<bool>(egui::Id::new("media_fullscreen"))).unwrap_or(false)}
pub fn set_fullscreen(ctx:&egui::Context,fullscreen:bool) {
    let now=ctx.input(|input|input.time);
    ctx.data_mut(|data|{data.insert_temp(egui::Id::new("media_fullscreen"),fullscreen);data.insert_temp(egui::Id::new("media_last_motion"),now);});
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));ctx.request_repaint();
}
fn fullscreen_button(ui:&mut egui::Ui,fullscreen:bool)->bool {
    let response=ui.add_sized([28.0,28.0],egui::Button::new(""));
    let rect=response.rect;
    let color=ui.visuals().text_color();let bounds=rect.shrink(6.0);let length=4.0;
    for (corner,dx,dy) in [(bounds.left_top(),1.0,1.0),(bounds.right_top(),-1.0,1.0),(bounds.left_bottom(),1.0,-1.0),(bounds.right_bottom(),-1.0,-1.0)] {
        let corner=if fullscreen{corner+egui::vec2(dx,dy)*length}else{corner};
        let direction=if fullscreen{-1.0}else{1.0};
        ui.painter().line_segment([corner+egui::vec2(dx*length*direction,0.0),corner],egui::Stroke::new(1.5_f32,color));
        ui.painter().line_segment([corner,corner+egui::vec2(0.0,dy*length*direction)],egui::Stroke::new(1.5_f32,color));
    }
    response.on_hover_text(crate::localization::tr(if fullscreen{"退出全屏 (F11 / Esc)"}else{"全屏播放 (F11)"})).clicked()
}

pub fn is_media(path:&Path)->bool {
    let known=matches!(path.extension().and_then(|e|e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),
        "mp3"|"wav"|"ogg"|"oga"|"flac"|"aac"|"m4a"|"wma"|"opus"|"aiff"|"aif"|"mp4"|"m4v"|"mkv"|"webm"|"mov"|"avi"|"wmv"|"flv"|"mpeg"|"mpg"|"ts"|"mts"|"m2ts"|"3gp");
    known || has_media_header(path)
}
fn has_media_header(path:&Path)->bool {
    use std::{collections::HashMap,sync::OnceLock,time::SystemTime};
    type Entry=(u64,Option<SystemTime>,bool);
    static CACHE:OnceLock<Mutex<HashMap<PathBuf,Entry>>>=OnceLock::new();
    let Ok(metadata)=std::fs::metadata(path) else{return false};
    if !metadata.is_file(){return false;}
    let stamp=(metadata.len(),metadata.modified().ok());
    let cache=CACHE.get_or_init(||Mutex::new(HashMap::new()));
    if let Some(entry)=cache.lock().unwrap().get(path) {if (entry.0,entry.1)==stamp{return entry.2;}}
    let result=std::fs::File::open(path).ok().and_then(|mut file| {
        let mut header=[0u8;4096];let count=file.read(&mut header).ok()?;
        Some(is_mp4_header(&header[..count]))
    }).unwrap_or(false);
    let mut cache=cache.lock().unwrap();if cache.len()>=256{cache.clear();}cache.insert(path.to_owned(),(stamp.0,stamp.1,result));result
}
fn is_mp4_header(bytes:&[u8])->bool {
    let mut offset=0;
    while offset+8<=bytes.len() {
        let size=u32::from_be_bytes(bytes[offset..offset+4].try_into().unwrap()) as usize;
        let kind=&bytes[offset+4..offset+8];
        if size<8 {return false;}
        if kind==b"ftyp" {
            if size<16 || offset+size>bytes.len(){return false;}
            let brands=std::iter::once(&bytes[offset+8..offset+12]).chain(bytes[offset+16..offset+size].chunks_exact(4));
            return brands.into_iter().any(|brand|matches!(brand,b"isom"|b"iso2"|b"iso3"|b"iso4"|b"iso5"|b"iso6"|b"mp41"|b"mp42"|b"avc1"|b"M4V "|b"M4A "|b"qt  "|b"dash"|b"3gp4"|b"3gp5"|b"3gp6"));
        }
        if !matches!(kind,b"free"|b"skip"|b"wide") || size>bytes.len()-offset{return false;}
        offset+=size;
    }
    false
}
#[derive(Clone)]
struct AudioTrack {bit_rate:u64,index:u64,label:String,default:bool,ordinal:usize,language_name:Option<&'static str>,details:String}
impl AudioTrack {
    fn display_label(&self)->String {
        match self.language_name {
            Some(name)=>format!("{}.{} · {}",self.ordinal,crate::localization::tr(name),self.details),
            None=>self.label.clone(),
        }
    }
}
fn audio_language_name(title:&str,language:&str)->Option<&'static str> {
    fn identify(value:&str)->Option<&'static str> {
        match value.trim().to_lowercase().replace('_', "-").as_str() {
            "en" | "eng" | "english" | "英语" | "英語"=>Some("英语"),
            "cmn" | "zh-cmn" | "mandarin" | "mandarin chinese" | "国语" | "國語" | "普通话" | "普通話"=>Some("国语"),
            "yue" | "zh-yue" | "cantonese" | "cantonese chinese" | "粤语" | "粵語" | "广东话" | "廣東話"=>Some("粤语"),
            "zh" | "zho" | "chi" | "chinese" | "中文"=>Some("中文"),
            "ja" | "jpn" | "japanese" | "日本語" | "日语" | "日語"=>Some("日语"),
            "ko" | "kor" | "korean" | "한국어" | "韩语" | "韓語"=>Some("韩语"),
            "de" | "deu" | "ger" | "german" | "deutsch"=>Some("德语"),
            "fr" | "fra" | "fre" | "french" | "français"=>Some("法语"),
            "es" | "spa" | "spanish" | "español"=>Some("西班牙语"),
            "it" | "ita" | "italian" | "italiano"=>Some("意大利语"),
            "pt" | "por" | "portuguese" | "português"=>Some("葡萄牙语"),
            "ru" | "rus" | "russian" | "русский"=>Some("俄语"),
            "ar" | "ara" | "arabic"=>Some("阿拉伯语"),
            "hi" | "hin" | "hindi"=>Some("印地语"),
            "th" | "tha" | "thai"=>Some("泰语"),
            "vi" | "vie" | "vietnamese"=>Some("越南语"),
            "id" | "ind" | "indonesian"=>Some("印度尼西亚语"),
            "ms" | "msa" | "may" | "malay"=>Some("马来语"),
            "und"=>Some("未标注语言"), "mul"=>Some("多种语言"),
            "zxx"=>Some("无语言内容"), _=>None,
        }
    }
    // A recognized track name may correct an inconsistent container language tag.
    identify(title).or_else(|| {
        let normalized=language.trim().to_lowercase().replace('_', "-");
        identify(&normalized).or_else(|| {
            if normalized.starts_with("zh-yue-") { Some("粤语") }
            else if normalized.starts_with("zh-cmn-") { Some("国语") }
            else { normalized.split_once('-').and_then(|(base,_)|identify(base)) }
        })
    })
}
#[derive(Clone)]
struct Info { ffmpeg:PathBuf,ffprobe:PathBuf,duration:f64,start_time:f64,video:Option<[usize;2]>,audio:bool,channels:usize,sample_rate:u64,bits:u64,source_channels:u64,file_size:u64,fps:f64,audio_tracks:Vec<AudioTrack>,video_codec:String,pixel_format:String,container:String,bit_rate:u64,audio_bit_rate:u64,subtitle_count:usize,sample_aspect:String,video_bits:Option<u64>,dynamic_range:String,color_transfer:String }
fn probe(path:&Path)->Result<Info,String> {
    let ffmpeg=find_tool("ffmpeg").ok_or("未安装 FFmpeg，请前往设置中的视频解码安装。")?;
    let ffprobe=find_tool("ffprobe").ok_or("未安装 ffprobe，请前往设置中的视频解码安装。")?;
    let output=command(&ffprobe).args(["-v","error","-show_streams","-show_format","-of","json"]).arg(path).output().map_err(|e|e.to_string())?;
    if !output.status.success() {return Err(String::from_utf8_lossy(&output.stderr).into());}
    let data:serde_json::Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
    let streams=data["streams"].as_array().ok_or("无法读取媒体流")?;
    let audio_tracks=streams.iter().filter(|s|s["codec_type"]=="audio").enumerate().map(|(ordinal,s)| {
        let index=s["index"].as_u64().unwrap_or(ordinal as u64);
        let language=s["tags"]["language"].as_str().filter(|v|!v.is_empty()).unwrap_or("und");
        let title=s["tags"]["title"].as_str().map(str::trim).filter(|v|!v.is_empty())
            .or_else(||s["tags"]["handler_name"].as_str().map(str::trim).filter(|v|!v.is_empty())).unwrap_or("");
        let codec=s["codec_name"].as_str().unwrap_or("—").to_owned();
        let rate=s["sample_rate"].as_str().and_then(|s|s.parse().ok()).unwrap_or(0);
        let channels=s["channels"].as_u64().unwrap_or(0);let default=s["disposition"]["default"]==1;
        let description=if title.is_empty(){language.to_owned()}else{format!("{title} ({language})")};
        AudioTrack {bit_rate:s["bit_rate"].as_str().and_then(|v|v.parse::<u64>().ok()).or_else(||s["bit_rate"].as_u64()).unwrap_or(0),index,label:format!("{} · {description} · {codec} · {rate} Hz · {channels} ch",ordinal+1),default,
            ordinal:ordinal+1,language_name:audio_language_name(title,language),details:format!("{codec} · {rate} Hz · {channels} ch")}
    }).collect();
    let video_stream=streams.iter().find(|s|s["codec_type"]=="video" && s["disposition"]["attached_pic"]!=1);
    let video_codec=video_stream.and_then(|s|s["codec_name"].as_str()).unwrap_or("—").to_owned();
    let pixel_format=video_stream.and_then(|s|s["pix_fmt"].as_str()).unwrap_or("—").to_owned();
    let sample_aspect=video_stream.and_then(|s|s["sample_aspect_ratio"].as_str()).filter(|v|*v!="N/A" && *v!="0:1").unwrap_or("未知").to_owned();
    let color_transfer=video_stream.and_then(|s|s["color_transfer"].as_str()).unwrap_or("unknown").to_owned();
    let dynamic_range=match color_transfer.as_str() {"smpte2084"=>"HDR (PQ)","arib-std-b67"=>"HDR (HLG)","bt709"|"gamma22"|"gamma28"|"smpte170m"|"smpte240m"|"iec61966-2-1"|"bt2020-10"|"bt2020-12"=>"SDR",_=>"未知"}.to_owned();
    // Pixel-format descriptors report per-component precision, unlike container
    // or decoder output sample sizes. Missing descriptors remain unknown.
    let video_bits=video_stream.and_then(|s|s["bits_per_raw_sample"].as_str()).and_then(|s|s.parse::<u64>().ok()).filter(|v|*v>0).or_else(|| {
        if video_stream.is_none(){return None;}
        let output=command(&ffprobe).args(["-v","error","-show_pixel_formats","-of","json"]).output().ok()?;
        if !output.status.success(){return None;}
        let formats:serde_json::Value=serde_json::from_slice(&output.stdout).ok()?;
        formats["pixel_formats"].as_array()?.iter().find(|format|format["name"].as_str()==Some(pixel_format.as_str()))?["components"].as_array()?.iter().filter_map(|c|c["bit_depth"].as_u64()).max()
    });
    let container=data["format"]["format_name"].as_str().unwrap_or("—").to_owned();
    let bit_rate=video_stream.and_then(|s|s["bit_rate"].as_str()).and_then(|s|s.parse().ok()).unwrap_or(0);
    let subtitle_count=streams.iter().filter(|s|s["codec_type"]=="subtitle").count();
    let audio=streams.iter().any(|s|s["codec_type"]=="audio");
    let channels=streams.iter().find(|s|s["codec_type"]=="audio").and_then(|s|s["channels"].as_u64()).unwrap_or(1).clamp(1,2) as usize;
    let audio_stream=streams.iter().find(|s|s["codec_type"]=="audio");
    let audio_bit_rate=audio_stream.and_then(|s|s["bit_rate"].as_str().and_then(|v|v.parse::<u64>().ok()).or_else(||s["bit_rate"].as_u64())).unwrap_or(0);
    let sample_rate=audio_stream.and_then(|s|s["sample_rate"].as_str()).and_then(|s|s.parse().ok()).unwrap_or(0);
    let bits=audio_stream.and_then(|s|s["bits_per_raw_sample"].as_str().and_then(|v|v.parse::<u64>().ok()).filter(|v|*v>0).or_else(||s["bits_per_sample"].as_u64())).unwrap_or(0);
    let source_channels=audio_stream.and_then(|s|s["channels"].as_u64()).unwrap_or(0);
    let file_size=std::fs::metadata(path).map_or(0,|m|m.len());
    let fps=streams.iter().find(|s|s["codec_type"]=="video" && s["disposition"]["attached_pic"]!=1)
        .and_then(|s|s["avg_frame_rate"].as_str()).and_then(|s|s.split_once('/'))
        .and_then(|(a,b)|Some(a.parse::<f64>().ok()?/b.parse::<f64>().ok()?)).filter(|v|v.is_finite() && *v>0.0).unwrap_or(30.0);
    let video=streams.iter().find(|s|s["codec_type"]=="video" && s["disposition"]["attached_pic"]!=1).map(|s| {
        let w=s["width"].as_u64().unwrap_or(1).max(1) as f64;
        let h=s["height"].as_u64().unwrap_or(1).max(1) as f64;
        let rotation=s["side_data_list"].as_array().and_then(|list|list.iter().find_map(|entry|entry["rotation"].as_i64())).unwrap_or(0);
        if rotation.rem_euclid(180)==90 {[h as usize,w as usize]}else{[w as usize,h as usize]}
    });
    if video.is_some_and(|[w,h]|w>8192 || h>8192 || w.saturating_mul(h)>32*1024*1024) {return Err("视频分辨率超过预览内存预算".into());}
    if !audio && video.is_none() {return Err("没有可播放的音视频流".into());}
    let duration=data["format"]["duration"].as_str().and_then(|s|s.parse::<f64>().ok()).or_else(||streams.iter().filter_map(|s|s["duration"].as_str()?.parse::<f64>().ok()).max_by(f64::total_cmp)).unwrap_or(0.0);
    let start_time=data["format"]["start_time"].as_str().and_then(|s|s.parse::<f64>().ok()).filter(|t|t.is_finite()).unwrap_or(0.0);
    Ok(Info {ffmpeg,ffprobe,start_time,duration:if duration.is_finite(){duration.max(0.0)}else{0.0},video,audio,channels,sample_rate,bits,source_channels,file_size,fps,audio_tracks,video_codec,pixel_format,container,bit_rate,audio_bit_rate,subtitle_count,sample_aspect,video_bits,dynamic_range,color_transfer})
}
struct Control { stop:AtomicBool, volume:AtomicU32, children:Mutex<Vec<Child>>, clock:Mutex<Option<Instant>>,video_ready:AtomicBool,paused:AtomicBool,pause_at:Mutex<Option<Instant>> }
struct Playback { control:Arc<Control>, frames:mpsc::Receiver<(usize,egui::ColorImage)>, errors:mpsc::Receiver<String>, done:Arc<AtomicBool> }
impl Drop for Playback {
    fn drop(&mut self) {
        self.control.stop.store(true,Ordering::Relaxed);
        let children=std::mem::take(&mut *self.control.children.lock().unwrap());
        std::thread::spawn(move|| {for mut child in children {let _=child.kill();let _=child.wait();}});
    }
}
fn spawn_decoder(control:&Control,ffmpeg:&Path,path:&Path,position:f64,realtime:bool,hardware:Option<&str>,args:&[String])->Result<std::process::ChildStdout,String> {
    let mut cmd=command(ffmpeg);
    cmd.args(["-nostdin","-v","error"]);
    if realtime {cmd.arg("-re");}
    if let Some(mode)=hardware {cmd.args(["-hwaccel",mode]);}
    if position>0.0 {cmd.args(["-ss",&position.to_string()]);}
    let mut child=cmd.arg("-i").arg(path)
        .args(args).args(["-threads","2","pipe:1"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;
    let stdout=child.stdout.take().ok_or("无法读取解码输出")?;
    let mut children=control.children.lock().unwrap();
    if control.stop.load(Ordering::Relaxed) {let _=child.kill();let _=child.wait();return Err("播放已停止".into());}
    children.push(child);Ok(stdout)
}
fn start(info:&Info,path:&Path,position:f64,first_frame:usize,volume:f32,mode:&str,audio_track:Option<u64>,info_frame_times:Option<Arc<Vec<f64>>>,cache:Arc<Mutex<FrameCache>>,ctx:&egui::Context)->Playback {
    let control=Arc::new(Control {stop:AtomicBool::new(false),volume:AtomicU32::new(volume.to_bits()),children:Mutex::new(Vec::new()),clock:Mutex::new(None),video_ready:AtomicBool::new(info.video.is_none()),paused:AtomicBool::new(false),pause_at:Mutex::new(None)});
    let (frame_tx,frames)=mpsc::sync_channel(2);let (error_tx,errors)=mpsc::channel();
    let done=Arc::new(AtomicBool::new(false));
    let remaining=Arc::new(AtomicU32::new(u32::from(info.video.is_some())+u32::from(audio_track.is_some())));
    if let Some(size)=info.video {
        let control=control.clone();let path=path.to_owned();let ffmpeg=info.ffmpeg.clone();let ctx=ctx.clone();let errors=error_tx.clone();let done=done.clone();let mode=mode.to_owned();
        let remaining=remaining.clone();
        let only_video=audio_track.is_none();let fps=info.fps;let frame_times=info_frame_times.clone();let cache=cache.clone();
        std::thread::spawn(move|| {
            let result=(|| {
                let filter=format!("scale={}:{}:flags=bilinear",size[0],size[1]);
                let mut reader=spawn_decoder(&control,&ffmpeg,&path,position,false,Some(&mode),&["-map".into(),"0:V:0".into(),"-an".into(),"-vf".into(),filter,"-fps_mode".into(),"passthrough".into(),"-pix_fmt".into(),"rgba".into(),"-f".into(),"rawvideo".into()])?;
                let mut bytes=vec![0u8;size[0]*size[1]*4];let mut any=false;let mut index=first_frame;
                while !control.stop.load(Ordering::Relaxed) {
                    match reader.read_exact(&mut bytes) {Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::UnexpectedEof=>break,Err(e)=>return Err(e.to_string())}
                    any=true;
                    control.video_ready.store(true,Ordering::Relaxed);
                    if only_video {control.clock.lock().unwrap().get_or_insert_with(Instant::now);}
                    let image=egui::ColorImage::from_rgba_unmultiplied(size,&bytes);
                    cache.lock().unwrap().insert(index,image.clone());
                    let target=frame_times.as_ref().and_then(|times|times.get(index)).copied().unwrap_or(index as f64/fps)-position;
                    loop {
                        if control.stop.load(Ordering::Relaxed){return Ok(());}
                        if control.paused.load(Ordering::Relaxed){std::thread::sleep(Duration::from_millis(5));continue;}
                        let clock=*control.clock.lock().unwrap();
                        if clock.is_some_and(|clock|clock.elapsed().as_secs_f64()>=target.max(0.0)){break;}
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    let clock=*control.clock.lock().unwrap();
                    if clock.is_some_and(|clock|clock.elapsed().as_secs_f64()-target>0.15){index+=1;continue;}
                    if frame_tx.send((index,image)).is_err() {break;}index+=1;ctx.request_repaint();
                }
                if !any && !control.stop.load(Ordering::Relaxed) {return Err("视频解码没有输出，请检查格式或解码器。".into());}Ok(())
            })();
            if let Err(error)=result {if !control.stop.load(Ordering::Relaxed) {let _=errors.send(error);}}
            if remaining.fetch_sub(1,Ordering::Relaxed)==1 {done.store(true,Ordering::Relaxed);}ctx.request_repaint();
        });
    }
    if let Some(audio_track)=audio_track {
        let control=control.clone();let path=path.to_owned();let ffmpeg=info.ffmpeg.clone();let ctx=ctx.clone();let done=done.clone();let remaining=remaining.clone();
        std::thread::spawn(move|| {
            let result=(|| {
                let (_stream,handle)=rodio::OutputStream::try_default().map_err(|e|e.to_string())?;
                let sink=rodio::Sink::try_new(&handle).map_err(|e|e.to_string())?;
                sink.pause();
                let mut reader=spawn_decoder(&control,&ffmpeg,&path,position,false,None,&["-map".into(),format!("0:{audio_track}"),"-vn".into(),"-ac".into(),"2".into(),"-ar".into(),"48000".into(),"-f".into(),"s16le".into()])?;
                let mut bytes=vec![0u8;19200];let mut any=false;
                loop {
                    if control.stop.load(Ordering::Relaxed) {sink.stop();return Ok(());}
                    sink.set_volume(f32::from_bits(control.volume.load(Ordering::Relaxed)));
                    while (sink.len()>=3 || control.paused.load(Ordering::Relaxed)) && !control.stop.load(Ordering::Relaxed) {if control.paused.load(Ordering::Relaxed){sink.pause();}else{sink.play();}sink.set_volume(f32::from_bits(control.volume.load(Ordering::Relaxed)));std::thread::sleep(Duration::from_millis(5));}
                    // Accumulate an aligned PCM block; pipe reads can split samples.
                    let mut count=0;
                    while count<bytes.len() {let n=reader.read(&mut bytes[count..]).map_err(|e|e.to_string())?;if n==0 {break;}count+=n;}
                    if control.stop.load(Ordering::Relaxed){sink.stop();return Ok(());}
                    if count==0 {break;}any=true;
                    let samples:Vec<i16>=bytes[..count-count%4].chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]])).collect();
                    sink.append(rodio::buffer::SamplesBuffer::new(2,48000,samples));
                    while !control.video_ready.load(Ordering::Relaxed) && !control.stop.load(Ordering::Relaxed){std::thread::sleep(Duration::from_millis(2));}
                    if control.stop.load(Ordering::Relaxed){sink.stop();return Ok(());}
                    control.clock.lock().unwrap().get_or_insert_with(Instant::now);
                    if control.paused.load(Ordering::Relaxed){sink.pause();}else{sink.play();}
                }
                if !any {return Err("音频解码没有输出，请检查格式或解码器。".into());}
                while !sink.empty() && !control.stop.load(Ordering::Relaxed) {if control.paused.load(Ordering::Relaxed){sink.pause();}else{sink.play();}sink.set_volume(f32::from_bits(control.volume.load(Ordering::Relaxed)));std::thread::sleep(Duration::from_millis(20));}
                sink.stop();Ok(())
            })();
            if let Err(error)=result {if !control.stop.load(Ordering::Relaxed) {let _=error_tx.send(error);}}
            if remaining.fetch_sub(1,Ordering::Relaxed)==1 {done.store(true,Ordering::Relaxed);}ctx.request_repaint();
        });
    }
    Playback {control,frames,errors,done}
}

fn preview_poster(frame:&egui::ColorImage)->egui::ColorImage {
    let [width,height]=frame.size;
    let scale=(1024.0/width.max(height) as f32).min(1.0);
    let size=[(width as f32*scale).round().max(1.0) as usize,(height as f32*scale).round().max(1.0) as usize];
    if size==frame.size{return frame.clone();}
    let mut pixels=Vec::with_capacity(size[0]*size[1]);
    for y in 0..size[1]{for x in 0..size[0]{pixels.push(frame.pixels[(y*height/size[1])*width+x*width/size[0]]);}}
    egui::ColorImage {size,pixels}
}
const FRAME_CACHE_LIMIT:usize=128*1024*1024;
#[derive(Default)]
struct FrameCache {frames:std::collections::VecDeque<(usize,egui::ColorImage)>,bytes:usize}
impl FrameCache {
    fn insert(&mut self,index:usize,frame:egui::ColorImage){self.insert_bounded(index,frame,FRAME_CACHE_LIMIT);}
    fn insert_bounded(&mut self,index:usize,frame:egui::ColorImage,limit:usize){
        let bytes=frame.pixels.len()*std::mem::size_of::<egui::Color32>();
        if bytes>limit{return;}
        if let Some(i)=self.frames.iter().position(|(id,_)|*id==index){let (_,old)=self.frames.remove(i).unwrap();self.bytes-=old.pixels.len()*4;}
        while self.bytes+bytes>limit {if let Some((_,old))=self.frames.pop_front(){self.bytes-=old.pixels.len()*4;}else{break;}}
        self.bytes+=bytes;self.frames.push_back((index,frame));
    }
    fn get(&mut self,index:usize)->Option<egui::ColorImage>{let i=self.frames.iter().position(|(id,_)|*id==index)?;let entry=self.frames.remove(i)?;let frame=entry.1.clone();self.frames.push_back(entry);Some(frame)}
}

#[derive(Clone,Copy)]
struct VolumePreference {volume:f32,remembered:f32,dirty:bool}
fn volume_preference()->&'static Mutex<VolumePreference> {
    static VALUE:std::sync::OnceLock<Mutex<VolumePreference>>=std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        let mut value=VolumePreference {volume:0.8,remembered:0.8,dirty:false};
        if let Some(path)=crate::last_open_dir_config_path().map(|path|path.with_file_name("media_volume.txt")) {
            if let Ok(text)=std::fs::read_to_string(path) {let numbers=text.split_whitespace().filter_map(|part|part.parse::<f32>().ok()).collect::<Vec<_>>();
                if numbers.len()==2 && numbers.iter().all(|n|n.is_finite() && (0.0..=1.0).contains(n)) && numbers[1]>0.0 {value.volume=numbers[0];value.remembered=numbers[1];}
            }
        }
        Mutex::new(value)
    })
}
fn persist_volume() {
    let mut preference=volume_preference().lock().unwrap();
    if !preference.dirty{return;}
    if let Some(path)=crate::last_open_dir_config_path().map(|path|path.with_file_name("media_volume.txt")) {
        if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}
        if std::fs::write(path,format!("{} {}",preference.volume,preference.remembered)).is_ok(){preference.dirty=false;}
    }
}
struct VideoGesture {origin:egui::Pos2,last:egui::Pos2,dragging:bool,on_image:bool}
impl VideoGesture {
    fn move_to(&mut self,point:egui::Pos2)->egui::Vec2 {
        // Latch dragging so returning to the press position never becomes a click.
        let was_dragging=self.dragging;
        self.dragging|=point.distance(self.origin)>=5.0;
        let delta=if was_dragging{point-self.last}else if self.dragging{point-self.origin}else{egui::Vec2::ZERO};
        self.last=point;delta
    }
}
thread_local! {
    static TRIM_EDITORS: std::cell::RefCell<std::collections::HashMap<PathBuf,Preview>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Pump independent editors from the application, rather than the selected tab.
pub fn show_trim_windows(ctx:&egui::Context) {
    TRIM_EDITORS.with(|editors| {
        editors.borrow_mut().retain(|path,player| {
            let mut host=egui::Ui::new(ctx.clone(),egui::Id::new(("trim_host",path)),egui::UiBuilder::new().max_rect(ctx.screen_rect()));
            player.show_trim_editor(&mut host,path);
            player.trim.is_open()
        });
    });
}

pub struct Preview { edit_clip:Option<Arc<crate::audio_edit::Clip>>,trim:crate::media_trim::Trim,trim_preview_end:Option<f64>,trim_preview_next:Option<(f64,f64)>,overlay_interactive:bool,sidebar_width:f32,overlay_rect:Option<egui::Rect>,canvas_rect:Option<egui::Rect>,sidebar_rect:Option<egui::Rect>,playlist_mode:bool,playlist_next:bool,playlist_ended:bool,initial_seek:Option<f64>,history_file:Option<(PathBuf,crate::media_history::Identity)>,history_error:Option<String>,autoplay_pending:bool,meter_trails:[f32;2],video_gesture:Option<VideoGesture>,info:Option<Info>,probe:Option<mpsc::Receiver<Result<Info,String>>>,playback:Option<Playback>,texture:Option<egui::TextureHandle>,poster:Option<egui::ColorImage>,error:Option<String>,position:f64,started:Option<Instant>,volume:f32,remembered_volume:f32,volume_hover:Option<Instant>,
    waveform:Option<Vec<Vec<f32>>>,wave_job:Option<WaveJob>,wave_error:Option<String>,scale_mode:String,zoom:Option<f32>,pan:egui::Vec2,flip_x:bool,flip_y:bool,decoder:String,
    frame_times:Option<Arc<Vec<f64>>>,index_job:Option<IndexJob>,index_error:Option<String>,frame_job:Option<FrameJob>,current_frame:Option<usize>,selected_audio:Option<u64>,seek_drag:Option<(f64,bool)>,cache:Arc<Mutex<FrameCache>>,playback_origin:f64,resume_position:Option<f64> }
impl Default for Preview {fn default()->Self {let preference=*volume_preference().lock().unwrap();Self {edit_clip:None,trim:crate::media_trim::Trim::default(),trim_preview_end:None,trim_preview_next:None,overlay_interactive:false,sidebar_width:0.0,overlay_rect:None,canvas_rect:None,sidebar_rect:None,playlist_mode:false,playlist_next:false,playlist_ended:false,initial_seek:None,history_file:None,history_error:None,autoplay_pending:false,meter_trails:[0.0;2],video_gesture:None,info:None,probe:None,playback:None,texture:None,poster:None,error:None,position:0.0,started:None,volume:preference.volume,remembered_volume:preference.remembered,volume_hover:None,waveform:None,wave_job:None,wave_error:None,scale_mode:"fit".into(),zoom:None,pan:egui::Vec2::ZERO,flip_x:false,flip_y:false,decoder:"auto".into(),frame_times:None,index_job:None,index_error:None,frame_job:None,current_frame:None,selected_audio:None,seek_drag:None,cache:Arc::new(Mutex::new(FrameCache::default())),playback_origin:0.0,resume_position:None}}}
impl Preview {
    pub fn new(path: &Path) -> Self {
        let identity = crate::media_history::Identity::read(path);
        let position = identity.as_ref().map_or(0.0, |identity| crate::media_history::load(path, identity));
        let mut preview = Self::default();
        preview.autoplay_pending = true;
        preview.history_file = identity.map(|identity| (path.to_owned(), identity));
        preview.initial_seek = (position > 0.0).then_some(position);
        preview.position = position;
        preview
    }
    /// Playlist sessions never read or write the ordinary file history.
    pub fn for_playlist(position:f64,autoplay:bool)->Self {
        let mut preview=Self::default();
        preview.playlist_mode=true;
        preview.position=if position.is_finite(){position.max(0.0)}else{0.0};
        preview.initial_seek=(preview.position>0.0).then_some(preview.position);
        preview.autoplay_pending=autoplay;
        preview
    }
    pub fn set_overlay_interactive(&mut self,visible:bool){self.overlay_interactive=visible;}
    pub fn set_sidebar_width(&mut self,width:f32){self.sidebar_width=width.max(0.0);}
    pub fn overlay_rect(&self)->Option<egui::Rect>{self.overlay_rect}
    pub fn canvas_rect(&self)->Option<egui::Rect>{self.canvas_rect}
    pub fn sidebar_rect(&self)->Option<egui::Rect>{self.sidebar_rect}
    pub fn playlist_position(&self)->f64 {self.position_now()}
    pub fn playlist_duration(&self)->Option<f64> {self.info.as_ref().map(|i|i.duration)}
    pub fn playlist_playing(&self)->bool {self.is_playing()}
    pub fn playlist_failed(&self)->bool {self.error.is_some()}
    pub fn take_playlist_next(&mut self)->bool {std::mem::take(&mut self.playlist_next)}
    pub fn take_playlist_end(&mut self)->bool {std::mem::take(&mut self.playlist_ended)}
    pub fn playlist_seek(&mut self,ctx:&egui::Context,path:&Path,position:f64){
        let playing=self.is_playing();self.stop_playback();self.frame_job=None;
        self.position=position;self.initial_seek=Some(position);self.current_frame=None;
        if playing{self.play(ctx,path);}else{self.seek_frame(ctx,path);}
    }
    pub fn request_playback(&mut self) { self.autoplay_pending = true; }
    fn save_position(&mut self) {
        if self.info.is_none() { return; }
        if let Some((path, identity)) = &self.history_file {
            let position = self.position_now();
            let position = self.info.as_ref().filter(|info| info.duration.is_finite() && info.duration > 0.0)
                .map_or(position, |info| position.min(info.duration));
            self.history_error = crate::media_history::save(path, identity, position).err();
            if let Some(error) = &self.history_error { crate::media_history::report_error(error); }
        }
    }
    pub fn retry_missing_decoder(&mut self) {
        if self.info.is_none() && self.error.is_some(){self.error=None;self.probe=None;}
    }
    pub fn has_video(&self)->bool {self.info.as_ref().is_some_and(|info|info.video.is_some())}
    pub fn header_controls(&mut self,ui:&mut egui::Ui,path:&Path) {
        if !self.info.as_ref().is_some_and(|info|info.video.is_some()){return;}
        let previous=self.decoder.clone();
        crate::theme::video_decoder_selector(ui,&mut self.decoder);
        let mirror=match (self.flip_x,self.flip_y){(false,false)=>"无镜像",(true,false)=>"左右镜像",(false,true)=>"上下镜像",(true,true)=>"上下左右镜像"};
        ui.small(crate::localization::tr(mirror));
        self.scale_controls(ui);
        if self.index_job.is_some(){ui.spinner();ui.label(crate::localization::tr("正在索引视频帧…"));}
        if previous!=self.decoder {let playing=self.is_playing();self.stop_playback();self.cache=Arc::new(Mutex::new(FrameCache::default()));if playing{self.play(ui.ctx(),path);}else{self.error=None;}}
    }
    fn scale_controls(&mut self,ui:&mut egui::Ui) {
        let previous_scale=self.scale_mode.clone();
        crate::theme::video_scale_selector(ui,&mut self.scale_mode);
        if previous_scale!=self.scale_mode {
            self.video_gesture=None;
            self.pan=egui::Vec2::ZERO;
            if self.scale_mode=="native"{self.zoom=Some(1.0);}
            if self.scale_mode=="fit"{self.zoom=None;}
        }
        if let Some(zoom)=self.zoom {ui.small(format!("{:.1}%",zoom*100.0));}
    }
    pub fn file_size(&self)->Option<u64> {self.info.as_ref().map(|info|info.file_size)}
    pub fn trim_entry_button(&mut self,ui:&mut egui::Ui,path:&Path) {
        let duration=self.info.as_ref().map_or(0.0,|info|info.duration);
        let label=if self.info.as_ref().is_some_and(|info|info.video.is_some()){"视频编辑"}else{"音频编辑"};
        if ui.add_enabled(duration>0.0,egui::Button::new(crate::localization::tr(label))).clicked() {
            self.autoplay_pending=false;self.initial_seek=None;self.stop_playback();
            self.playlist_ended=false;self.playlist_next=false;
            let key=std::fs::canonicalize(path).unwrap_or_else(|_|path.to_owned());
            TRIM_EDITORS.with(|editors| {
                let mut editors=editors.borrow_mut();
                editors.entry(key.clone()).or_insert_with(|| {
                    self.detached_trim_player(duration)
                });
            });
            crate::native_window::wake(ui.ctx(),egui::Id::new(("media_trim",&key)));
        }
    }
    fn detached_trim_player(&self,duration:f64)->Self {
        let mut player=Self::default();
        player.info=self.info.clone();player.selected_audio=self.selected_audio;
        player.waveform=self.waveform.clone();player.volume=self.volume;player.remembered_volume=self.remembered_volume;
        player.trim.open(duration);
        player
    }
    fn is_playing(&self)->bool {self.playback.as_ref().is_some_and(|job|!job.control.paused.load(Ordering::Relaxed))}
    fn position_now(&self)->f64 {if self.is_playing(){self.playback_origin+self.started.map_or(0.0,|start|start.elapsed().as_secs_f64())}else{self.position}}
    pub fn pause(&mut self) {
        if !self.is_playing(){return;}
        self.position=self.position_now();self.started=None;
        if let Some(job)=&self.playback {job.control.paused.store(true,Ordering::Relaxed);*job.control.pause_at.lock().unwrap()=Some(Instant::now());}
        if let Some(time)=self.current_frame.and_then(|i|self.frame_times.as_ref()?.get(i).copied()){self.position=time;}
        self.resume_position=Some(self.position);
    }
    fn stop_playback(&mut self){self.pause();self.playback=None;self.started=None;self.resume_position=None;}
    fn stop_and_rewind(&mut self){
        self.stop_playback();self.position=0.0;self.initial_seek=None;
        self.trim_preview_next=None;self.trim_preview_end=None;
    }
    pub fn deactivate(&mut self) {
        self.autoplay_pending = false;
        self.meter_trails=[0.0;2];
        self.video_gesture=None;
        self.stop_playback();
        self.save_position();
        if let Some(frame)=self.current_frame.and_then(|index|self.cache.lock().unwrap().get(index)){self.poster=Some(preview_poster(&frame));}
        self.texture=None;self.wave_job=None;self.index_job=None;self.frame_job=None;self.cache=Arc::new(Mutex::new(FrameCache::default()));}
    fn show_trim_editor(&mut self,ui:&mut egui::Ui,path:&Path) {
        if let Some(info)=&self.info {
            if info.video.is_none() {
                self.trim.prepare_audio(&info.ffmpeg,path,info.duration,self.selected_audio.unwrap_or(0),info.source_channels as usize,ui.ctx());
            }
        }
        if let Some(clip)=self.trim.clip() {
            if self.edit_clip.as_ref().is_none_or(|old|old.path!=clip.path){
                self.stop_playback();self.trim_preview_end=None;self.frame_job=None;self.wave_job=None;
                self.waveform=None;self.wave_error=None;self.error=None;
                self.position=self.position.min(clip.duration);self.selected_audio=Some(0);
                if let Some(info)=&mut self.info{info.duration=clip.duration;}
                self.edit_clip=Some(clip);
            }
        }
        let playback_path=self.edit_clip.as_ref().map_or_else(||path.to_owned(),|clip|clip.path.clone());
        let mut failed=false;
        if let Some(job)=&self.playback {
            self.started=if job.control.paused.load(Ordering::Relaxed){None}else{*job.control.clock.lock().unwrap()};
            while let Ok((index,_))=job.frames.try_recv(){self.current_frame=Some(index);}
            if let Ok(error)=job.errors.try_recv(){self.error=Some(error);failed=true;}
            job.control.volume.store(self.volume.to_bits(),Ordering::Relaxed);
            if job.done.load(Ordering::Relaxed) && self.error.is_none() {
                self.stop_playback();self.position=self.info.as_ref().map_or(0.0,|info|info.duration);
            }
        }
        if failed {self.stop_playback();}
        if self.playback.is_some(){ui.ctx().request_repaint_after(Duration::from_millis(16));}
        if let Some(info)=&self.info {
            if info.video.is_none() && self.waveform.is_none() && self.wave_job.is_none() && self.wave_error.is_none() {
                self.wave_job=Some(waveform(info,&playback_path,ui.ctx()));
            }
        }
        if let Some(job)=&self.wave_job {match job.receiver.try_recv(){
            Ok(result)=>{self.wave_job=None;match result{Ok(wave)=>self.waveform=Some(wave),Err(error)=>self.wave_error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.wave_job=None;self.wave_error=Some("波形生成失败".into());},
            Err(mpsc::TryRecvError::Empty)=>{ui.ctx().request_repaint_after(Duration::from_millis(50));}
        }}
        if let Some(end)=self.trim_preview_end {
            if self.position_now()>=end {
                self.stop_playback();
                if let Some((start,next_end))=self.trim_preview_next.take(){self.position=start;self.trim_preview_end=Some(next_end);self.play(ui.ctx(),&playback_path);}
                else{self.position=end;self.trim_preview_end=None;}
            }
        }
        if let Some(info)=&self.info {
            let position=self.position_now();
            let description=format!("{} Hz · {} bit · {} ch",info.sample_rate,info.bits,info.source_channels);
            let view=crate::media_trim::AudioView {wave:self.waveform.as_deref(),wave_error:self.wave_error.as_deref(),playing:self.is_playing(),volume:self.volume,playback_error:self.error.as_deref(),description};
            let action=self.trim.show(ui,path,&info.ffmpeg,info.duration,position,info.video.is_some(),self.selected_audio,view);
            if let Some(volume)=action.volume {
                self.volume=volume;if volume>0.0{self.remembered_volume=volume;}

            }
            if let Some(position)=action.seek {self.trim_preview_next=None;self.trim_preview_end=None;self.playlist_seek(ui.ctx(),&playback_path,position);}
            if action.toggle {self.trim_preview_next=None;self.trim_preview_end=None;if self.is_playing(){self.pause();}else{if self.info.as_ref().is_some_and(|info|self.position>=info.duration){self.position=0.0;}self.play(ui.ctx(),&playback_path);}}
            if action.stop {self.trim_preview_next=None;self.trim_preview_end=None;if action.rewind{self.stop_and_rewind();}else{self.stop_playback();}}
            if let Some((start,end))=action.audition {
                self.stop_playback();self.position=start;self.trim_preview_next=action.audition_tail;self.trim_preview_end=Some(end);self.play(ui.ctx(),&playback_path);
            }
        }
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path) {
        self.canvas_rect=None;self.sidebar_rect=None;self.overlay_rect=None;
        let fullscreen=is_fullscreen(ui.ctx());
        let external_exclusion=ui.ctx().data(|data|data.get_temp::<egui::Rect>(egui::Id::new("media_overlay_exclusion")));
        let mut overlay_exclusion=external_exclusion;
        let controls_visible=!fullscreen||fullscreen_controls_visible(ui.ctx());
        if fullscreen&&!controls_visible{ui.ctx().set_cursor_icon(egui::CursorIcon::None);}
        let preference=*volume_preference().lock().unwrap();self.volume=preference.volume;self.remembered_volume=preference.remembered;
        if let Some(error) = &self.history_error {
            ui.colored_label(ui.visuals().warn_fg_color, crate::lf!("播放位置记录保存失败：{0}", error));
        }
        if self.info.is_none() && self.probe.is_none() && self.error.is_none() {
            let (tx,rx)=mpsc::channel();self.probe=Some(rx);let path=path.to_owned();let ctx=ui.ctx().clone();
            std::thread::spawn(move|| {let _=tx.send(probe(&path));ctx.request_repaint();});
        }
        if let Some(receiver)=&self.probe {match receiver.try_recv() {
            Ok(result)=>{self.probe=None;match result {Ok(info)=>{self.position=crate::media_history::restore_position(self.position,info.duration);self.selected_audio=info.audio_tracks.iter().find(|t|t.default).or_else(||info.audio_tracks.first()).map(|t|t.index);self.info=Some(info);if self.autoplay_pending {self.autoplay_pending=false;self.play(ui.ctx(),path);}},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.probe=None;self.error=Some("媒体信息读取失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if self.probe.is_some() {ui.spinner();ui.label(crate::localization::tr("正在读取媒体信息…"));ui.ctx().request_repaint_after(Duration::from_millis(100));return;}
        if self.autoplay_pending && self.info.is_some() && self.error.is_none() {
            self.autoplay_pending = false;
            if self.info.as_ref().is_some_and(|info| info.duration > 0.0 && self.position >= info.duration) { self.position = 0.0; }
            self.play(ui.ctx(), path);
        }
        let mut failed=false;
        if let Some(job)=&self.playback {
            self.started=if job.control.paused.load(Ordering::Relaxed){None}else{*job.control.clock.lock().unwrap()};
            while !job.control.paused.load(Ordering::Relaxed) {let Ok((index,frame))=job.frames.try_recv() else{break};self.current_frame=Some(index);if let Some(texture)=&mut self.texture {texture.set(frame,egui::TextureOptions::LINEAR);}else{self.texture=Some(ui.ctx().load_texture("media_frame",frame,egui::TextureOptions::LINEAR));}}
            if let Ok(error)=job.errors.try_recv() {self.error=Some(error);failed=true;}
            if job.done.load(Ordering::Relaxed) && self.error.is_none() {self.stop_playback();self.playlist_ended=true;if let Some(info)=&self.info {self.position=info.duration;}}
        }
        if failed {self.stop_playback();}
        if let Some(error)=&self.error {ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap());
            if ui.button(crate::localization::tr("前往视频解码设置")).clicked(){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("media_settings_request"),true));}
            if ui.button(crate::localization::tr("重试")).clicked(){self.stop_playback();self.cache=Arc::new(Mutex::new(FrameCache::default()));self.error=None;self.poster=None;self.texture=None;self.info=None;self.wave_job=None;self.waveform=None;self.wave_error=None;self.frame_times=None;self.index_job=None;self.index_error=None;self.frame_job=None;self.current_frame=None;}}
        let Some(info)=&self.info else{return};let duration=info.duration;
        if info.video.is_some() && self.texture.is_none() && !self.is_playing() {
            if let Some(poster)=&self.poster {self.texture=Some(ui.ctx().load_texture("media_frame",poster.clone(),egui::TextureOptions::LINEAR));}
            else if self.frame_job.is_none() && self.error.is_none() {
                let target = (self.position * info.fps).round() as usize;
                self.frame_job=Some(decode_frame(info,path,target,self.frame_times.clone(),self.cache.clone(),&self.decoder,ui.ctx()));
            }
        }
        if info.video.is_some() && self.frame_times.is_none() && self.index_job.is_none() && self.index_error.is_none() {self.index_job=Some(index_frames(info,path,ui.ctx()));}
        let mut indexed_now=false;
        if let Some(job)=&self.index_job {match job.receiver.try_recv(){
            Ok(result)=>{self.index_job=None;match result {Ok(times)=>{self.frame_times=Some(Arc::new(times));indexed_now=true;},Err(error)=>self.index_error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.index_job=None;self.index_error=Some("视频帧索引失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if let Some(job)=&self.frame_job {match job.receiver.try_recv(){
            Ok(result)=>{self.frame_job=None;match result {Ok((index,frame))=>{self.poster=Some(preview_poster(&frame));self.current_frame=Some(index);self.position=self.initial_seek.unwrap_or_else(||self.frame_times.as_ref().and_then(|times|times.get(index)).copied().unwrap_or(self.position));if let Some(texture)=&mut self.texture {texture.set(frame,egui::TextureOptions::LINEAR);}else{self.texture=Some(ui.ctx().load_texture("media_frame",frame,egui::TextureOptions::LINEAR));}},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.frame_job=None;self.error=Some("逐帧解码失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if info.video.is_none() && self.waveform.is_none() && self.wave_job.is_none() && self.wave_error.is_none() {
            if duration>0.0 {self.wave_job=Some(waveform(info,path,ui.ctx()));}else{self.wave_error=Some("媒体时长未知，无法生成完整波形。".into());}
        }
        if let Some(job)=&self.wave_job {match job.receiver.try_recv(){
            Ok(result)=>{self.wave_job=None;match result{Ok(wave)=>self.waveform=Some(wave),Err(error)=>self.wave_error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.wave_job=None;self.wave_error=Some("波形生成失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if indexed_now && self.is_playing() {let position=self.position_now();self.stop_playback();self.cache=Arc::new(Mutex::new(FrameCache::default()));self.position=position;self.current_frame=None;self.play(ui.ctx(),path);}
        if indexed_now && !self.is_playing() {
            if let Some(position) = self.initial_seek.take() {
                self.frame_job = None;
                self.position = crate::media_history::restore_position(position, duration);
                self.seek_frame(ui.ctx(), path);
            }
        }
        let mut position=self.seek_drag.map_or(self.position_now(),|seek|seek.0);if duration>0.0 {position=position.min(duration);}
        if !fullscreen {if let Some(info)=&self.info {if let Some([w,h])=info.video {
            ui.horizontal_wrapped(|ui| {
                let bitrate=if info.bit_rate>0{format!("{:.0} kb/s",info.bit_rate as f64/1000.0)}else{crate::localization::tr("未知").to_owned()};
                let bits=info.video_bits.map_or_else(||crate::localization::tr("未知").to_owned(),|bits|format!("{bits} bit"));
                ui.small(format!("{}: {} · {:.2} fps · {} · {w} × {h} · {}: {} · {} · {}",crate::localization::tr("视频信息"),info.video_codec,info.fps,bitrate,crate::localization::tr("像素宽高比"),crate::localization::tr(&info.sample_aspect),bits,crate::localization::tr(&info.dynamic_range)));
            });
            ui.horizontal_wrapped(|ui| {
                ui.small(format!("{}:",crate::localization::tr("音频信息")));
                if let Some(track)=info.audio_tracks.iter().find(|t|Some(t.index)==self.selected_audio) {
                    ui.small(format!("{}: {}",crate::localization::tr("当前音轨"),track.display_label())).on_hover_text(&track.label);
                    let bitrate=if track.bit_rate>0{format!("{:.0} kb/s",track.bit_rate as f64/1000.0)}else{crate::localization::tr("未知").into()};
                    ui.small(format!("{}: {bitrate}",crate::localization::tr("码率")));
                }else {ui.small(crate::localization::tr(if info.audio_tracks.is_empty(){"没有音轨"}else{"关闭音频"}));}
                ui.small(format!("{}: {}",crate::localization::tr("音轨"),info.audio_tracks.len()));
            });
        }else if info.audio {
            let depth=if info.bits>0{format!("{} bit",info.bits)}else{"bit: —".into()};
            let channels=match info.source_channels {1=>crate::localization::tr("单声道").to_owned(),2=>crate::localization::tr("双声道").to_owned(),n=>format!("{n} ch")};
            let bitrate=if info.audio_bit_rate>0{format!("{:.0} kb/s",info.audio_bit_rate as f64/1000.0)}else{crate::localization::tr("未知").into()};
            ui.small(format!("{} Hz · {depth} · {channels} · {}: {bitrate}",info.sample_rate,crate::localization::tr("码率")));
        }}}
        if let Some(job)=&self.playback {job.control.volume.store(self.volume.to_bits(),Ordering::Relaxed);let fps=self.info.as_ref().map_or(30.0,|i|i.fps);ui.ctx().request_repaint_after(Duration::from_secs_f64((0.5/fps).clamp(0.001,0.03)));}
        if self.info.as_ref().is_some_and(|i|i.video.is_none()) {
            paint_level_meter(ui,self.waveform.as_deref(),position,duration,&mut self.meter_trails);
        }
        if self.info.as_ref().is_some_and(|i|i.video.is_some()) {
            let mut step=0i32;
            let enabled=!self.is_playing() && self.frame_times.is_some() && self.frame_job.is_none();
            if enabled && !ui.ctx().wants_keyboard_input() {ui.input_mut(|input|{
                    if input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowLeft){step=-1;}
                    if input.consume_key(egui::Modifiers::NONE,egui::Key::ArrowRight){step=1;}
            });}
            if step!=0 {if let Some(times)=&self.frame_times {
                let base=self.current_frame.unwrap_or_else(||times.partition_point(|time|*time<self.position).min(times.len()-1));
                let target=(base as i64+step as i64).clamp(0,times.len() as i64-1) as usize;
                self.display_frame(ui.ctx(),path,target);
            }}
            if self.index_job.is_some() || self.frame_job.is_some(){ui.ctx().request_repaint_after(Duration::from_millis(50));}
            if !fullscreen {if let Some(error)=&self.index_error {ui.small(error);}}

        }
        let available_body=ui.available_rect_before_wrap();
        let footer=if fullscreen{0.0}else{32.0};
        let body=egui::Rect::from_min_max(available_body.min,egui::pos2(available_body.right(),(available_body.bottom()-footer).max(available_body.top()+1.0)));
        let width=self.sidebar_width.min((body.width()-100.0).max(0.0));
        let canvas=egui::Rect::from_min_max(body.min,egui::pos2(body.right()-width,body.bottom()));
        self.canvas_rect=Some(canvas);
        self.sidebar_rect=(width>6.0).then(||egui::Rect::from_min_max(egui::pos2(canvas.right()+6.0,body.top()),body.max));
        if self.playlist_mode {let arrow=Some(egui::Rect::from_center_size(egui::pos2(canvas.right()-12.0,canvas.center().y),egui::vec2(24.0,60.0)));self.overlay_rect=arrow;if self.overlay_interactive{overlay_exclusion=arrow;}}
        let (video_clicked,previous_audio,wave_seek)=ui.scope_builder(egui::UiBuilder::new().max_rect(canvas),|ui|{
            ui.set_clip_rect(canvas.intersect(ui.clip_rect()));
        let wave_seek=self.waveform.as_ref().and_then(|wave|ui.push_id(path,|ui|paint_waveform(ui,wave,position,duration,self.is_playing())).inner);
        if let Some(gesture)=&wave_seek {
            if gesture.started {self.pause();self.frame_job=None;self.seek_drag=Some((gesture.position,true));}
            position=gesture.position;
            if let Some(drag)=&mut self.seek_drag {drag.0=position;drag.1=true;}
        }
        if self.wave_job.is_some() {ui.spinner();ui.label(crate::localization::tr("正在生成音频波形…"));ui.ctx().request_repaint_after(Duration::from_millis(100));}
        if let Some(error)=&self.wave_error {ui.small(error);}
        let previous_audio=self.selected_audio;
        let mut video_clicked=false;
        if let Some(texture)=&self.texture {
            let uv=egui::Rect::from_min_max(egui::pos2(if self.flip_x{1.0}else{0.0},if self.flip_y{1.0}else{0.0}),egui::pos2(if self.flip_x{0.0}else{1.0},if self.flip_y{0.0}else{1.0}));
            let available=egui::vec2(ui.available_width(),ui.available_height().max(1.0));let native=self.info.as_ref().and_then(|info|info.video).map_or(texture.size_vec2(),|[w,h]|egui::vec2(w as f32,h as f32));
            let free=self.scale_mode=="free";
            let (canvas,response)=ui.allocate_exact_size(available,if free{egui::Sense::drag()}else{egui::Sense::click()});
            let mut zoom=match self.scale_mode.as_str(){
                "native"=>1.0,
                "free"=>self.zoom.unwrap_or_else(||fit_scale(native,canvas.size()).clamp(0.01,64.0)),
                _=>fit_scale(native,canvas.size()),
            };
            if self.zoom!=Some(zoom){self.zoom=Some(zoom);ui.ctx().request_repaint();}
            if !free{self.pan=egui::Vec2::ZERO;}
            if free {
                let press_image=egui::Rect::from_center_size(canvas.center()+self.pan,native*zoom);
                for event in ui.input(|input|input.events.clone()) {
                    match event {
                        egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:true,..}=>{
                            if response.contains_pointer() && canvas.intersect(ui.clip_rect()).contains(pos) && !overlay_exclusion.is_some_and(|rect|rect.contains(pos)) && !external_exclusion.is_some_and(|rect|rect.contains(pos)) {
                                self.video_gesture=Some(VideoGesture {origin:pos,last:pos,dragging:false,on_image:press_image.contains(pos)});
                            }
                        },
                        egui::Event::PointerMoved(pos)=>if let Some(gesture)=&mut self.video_gesture {self.pan+=gesture.move_to(pos);},
                        egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:false,..}=>{
                            if let Some(mut gesture)=self.video_gesture.take() {
                                self.pan+=gesture.move_to(pos);
                                video_clicked=!gesture.dragging && gesture.on_image && canvas.intersect(ui.clip_rect()).contains(pos) && press_image.contains(pos) && !overlay_exclusion.is_some_and(|rect|rect.contains(pos)) && !external_exclusion.is_some_and(|rect|rect.contains(pos));
                            }
                        },
                        egui::Event::PointerGone|egui::Event::WindowFocused(false)=>self.video_gesture=None,
                        _=>{},
                    }
                }
            } else {self.video_gesture=None;}
            if free && response.hovered() {
                let scroll=ui.input_mut(|input|{let scroll=input.smooth_scroll_delta.y;input.smooth_scroll_delta=egui::Vec2::ZERO;input.raw_scroll_delta=egui::Vec2::ZERO;scroll});
                if scroll!=0.0 {
                    let next=(zoom*(scroll*0.002).exp()).clamp(0.01,64.0);
                    if let Some(pointer)=response.hover_pos(){let relative=pointer-canvas.center();self.pan=relative-(relative-self.pan)*(next/zoom);}
                    zoom=next;self.zoom=Some(zoom);ui.ctx().request_repaint();
                }
            }
            let mut image_rect=egui::Rect::from_center_size(canvas.center()+self.pan,native*zoom);
            // Avoid blending adjacent texels just because centering lands between screen pixels.
            if fullscreen || (zoom-1.0).abs()<f32::EPSILON {
                let pixels_per_point=ui.ctx().pixels_per_point();
                let aligned=egui::pos2((image_rect.min.x*pixels_per_point).round()/pixels_per_point,(image_rect.min.y*pixels_per_point).round()/pixels_per_point);
                image_rect=egui::Rect::from_min_size(aligned,image_rect.size());
            }
            if !free {video_clicked=response.clicked_by(egui::PointerButton::Primary) && response.interact_pointer_pos().is_some_and(|point|image_rect.contains(point)&&!overlay_exclusion.is_some_and(|rect|rect.contains(point)) && !external_exclusion.is_some_and(|rect|rect.contains(point)));}
            ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect())).image(texture.id(),image_rect,uv,egui::Color32::WHITE);
            if free && response.dragged_by(egui::PointerButton::Primary){ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);}
            if let Some(info)=&self.info {response.context_menu(|ui|{display_menu(ui,&mut self.flip_x,&mut self.flip_y);audio_menu(ui,info,&mut self.selected_audio);});}
        }
        else if self.info.as_ref().is_some_and(|info|info.video.is_some()) {
            let size=egui::vec2(ui.available_width(),ui.available_height().max(1.0));
            let (canvas,response)=ui.allocate_exact_size(size,egui::Sense::hover());
            if self.is_playing() {
                // Only show loading on an empty canvas when no preview is available.
                let spinner=egui::Rect::from_center_size(canvas.center(),egui::vec2(24.0,24.0));
                ui.put(spinner,egui::Spinner::new());
            } else if self.scale_mode!="free" {
            let diameter=128.0_f32.min(size.x.min(size.y)).max(1.0);
            let button_rect=egui::Rect::from_center_size(canvas.center(),egui::vec2(diameter,diameter));
            let button=ui.interact(button_rect,ui.make_persistent_id("video_initial_play"),egui::Sense::click());
            let color=if button.hovered(){ui.visuals().selection.bg_fill}else{ui.visuals().text_color()};
            ui.painter().circle_stroke(button_rect.center(),diameter*0.48,egui::Stroke::new(2.0_f32,color));
            let triangle_size=egui::vec2(diameter*0.44,diameter*0.52);
            let triangle=egui::Rect::from_center_size(button_rect.center()+egui::vec2(triangle_size.x/6.0,0.0),triangle_size);
            paint_rounded_play(ui,triangle,color,diameter*0.025);
            video_clicked=button.clicked();
            }
            if let Some(info)=&self.info {response.context_menu(|ui|{display_menu(ui,&mut self.flip_x,&mut self.flip_y);audio_menu(ui,info,&mut self.selected_audio);});}
        }
        else if self.waveform.is_none() {ui.label(crate::localization::tr("音频播放"));}
            (video_clicked,previous_audio,wave_seek)
        }).inner;
        ui.allocate_rect(body,egui::Sense::hover());
        if previous_audio!=self.selected_audio {let playing=self.is_playing();let position=self.position_now();self.stop_playback();self.position=position;if playing{self.play(ui.ctx(),path);}}
        // Keep transport controls below the preview, with a full-width timeline.
        if !fullscreen {let spare=(ui.available_height()-28.0-ui.spacing().item_spacing.y).max(0.0);ui.add_space(spare);}
        let space_toggle=!ui.ctx().wants_keyboard_input() && ui.input_mut(|input|{
            let pressed=input.events.iter().any(|event|matches!(event,egui::Event::Key {key:egui::Key::Space,pressed:true,repeat:false,modifiers,..} if *modifiers==egui::Modifiers::NONE));
            let consumed=input.consume_key(egui::Modifiers::NONE,egui::Key::Space);
            pressed && consumed
        });
        let mut toggle=video_clicked || space_toggle;let mut stop=false;let mut next=false;let mut seek=wave_seek.as_ref().is_some_and(|gesture|gesture.finished);
        let screen=ui.clip_rect();
        if fullscreen && controls_visible {
            egui::Area::new(egui::Id::new("fullscreen_video_scale")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_TOP,egui::vec2(screen.center().x-ui.ctx().screen_rect().center().x,screen.top()+12.0)).show(ui.ctx(),|ui|{
                egui::Frame::NONE.fill(egui::Color32::from_black_alpha(128)).inner_margin(8.0).corner_radius(6.0).show(ui,|ui|{ui.horizontal(|ui|{self.scale_controls(ui);});});
            });
        }
        let mut transport=|ui:&mut egui::Ui| {ui.horizontal(|ui| {
            let playing=self.is_playing();
            toggle|=transport_button(ui,if playing{crate::icons::Icon::Pause}else{crate::icons::Icon::Play},crate::localization::tr(if playing{"暂停"}else{"播放"}));
            if self.playlist_mode{next=transport_button(ui,crate::icons::Icon::Next,crate::localization::tr("下一项"));}
            else{stop=transport_button(ui,crate::icons::Icon::Stop,crate::localization::tr("停止"));}
            volume_control(ui,path,&mut self.volume,&mut self.remembered_volume,&mut self.volume_hover);
            if let Some(job)=&self.playback {job.control.volume.store(self.volume.to_bits(),Ordering::Relaxed);}
            let video=self.has_video();
            let (rect,response)=ui.allocate_exact_size(egui::vec2((ui.available_width()-if video{28.0+ui.spacing().item_spacing.x}else{0.0}).max(1.0),28.0),egui::Sense::click_and_drag());
            ui.painter().rect_filled(rect,3.0,ui.visuals().extreme_bg_color);
            if duration>0.0 {let mut fill=rect;fill.max.x=rect.left()+rect.width()*(position/duration) as f32;ui.painter().rect_filled(fill,3.0,ui.visuals().selection.bg_fill);
                if response.drag_started(){let playing=self.is_playing();self.pause();self.frame_job=None;self.seek_drag=Some((self.position,playing));}
                if response.clicked() || response.dragged() || response.drag_stopped(){if let Some(pointer)=response.interact_pointer_pos(){position=((pointer.x-rect.left())/rect.width()).clamp(0.0,1.0) as f64*duration;if let Some(drag)=&mut self.seek_drag {drag.0=position;}if !response.dragged() || response.drag_stopped(){seek=true;}}}
                if response.drag_stopped(){seek=true;}
            }
            let time_label=if self.info.as_ref().is_some_and(|info|info.video.is_some()) {
                let current=self.current_frame.map_or(0,|frame|frame+1);
                let total=self.frame_times.as_ref().map_or_else(||"—".to_owned(),|times|times.len().to_string());
                format!("{}(f{current}) / {}(f{total})",time(position),time(duration))
            }else{format!("{} / {}",time(position),time(duration))};
            ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,time_label,egui::FontId::proportional(13.0),ui.visuals().text_color());
            response.on_hover_text(crate::localization::tr("点击或拖动跳转"));
            if video && fullscreen_button(ui,fullscreen){set_fullscreen(ui.ctx(),!fullscreen);}
        });};
        if fullscreen {
            if controls_visible {egui::Area::new(egui::Id::new("fullscreen_video_transport")).order(egui::Order::Foreground).fixed_pos(egui::pos2(screen.left()+12.0,screen.bottom()-56.0)).show(ui.ctx(),|ui|{
                egui::Frame::NONE.fill(egui::Color32::from_black_alpha(128)).inner_margin(8.0).corner_radius(6.0).show(ui,|ui|{ui.set_width((screen.width()-40.0).max(100.0));transport(ui);});
            });}
        }else{transport(ui);}
        if seek {self.initial_seek=None;let drag=self.seek_drag.take();let playing=drag.map_or(self.is_playing(),|seek|seek.1);if let Some(drag)=drag{position=drag.0;}self.pause();self.position=position;self.current_frame=None;if playing {
            let cached=self.frame_times.as_ref().map(|times|times.partition_point(|t|*t<=self.position).saturating_sub(1).min(times.len()-1)).filter(|target|self.cache.lock().unwrap().frames.iter().any(|(index,_)|index==target));
            if let Some(target)=cached{self.display_frame(ui.ctx(),path,target);}
            self.play(ui.ctx(),path);
        }else{self.seek_frame(ui.ctx(),path);}}
        if toggle {if self.is_playing(){self.pause();}else{if duration>0.0 && self.position>=duration{self.position=0.0;}self.play(ui.ctx(),path);}}
        if next {self.playlist_next=true;}
        if stop {self.initial_seek=None;self.stop_playback();self.position=0.0;self.current_frame=None;self.frame_job=None;if self.frame_times.is_some(){self.seek_frame(ui.ctx(),path);}else if let Some(info)=self.info.as_ref().filter(|info|info.video.is_some()){self.frame_job=Some(decode_frame(info,path,0,None,self.cache.clone(),&self.decoder,ui.ctx()));}}
        {
            let mut preference=volume_preference().lock().unwrap();
            if preference.volume!=self.volume || preference.remembered!=self.remembered_volume {preference.volume=self.volume;preference.remembered=self.remembered_volume;preference.dirty=true;}
        }
        if !ui.input(|input|input.pointer.any_down()){persist_volume();}
    }
    fn display_frame(&mut self,ctx:&egui::Context,path:&Path,target:usize) {
        self.frame_job=None;
        if let Some(frame)=self.cache.lock().unwrap().get(target) {
            self.current_frame=Some(target);self.position=self.frame_times.as_ref().and_then(|times|times.get(target)).copied().unwrap_or(self.position);
            if let Some(texture)=&mut self.texture {texture.set(frame,egui::TextureOptions::LINEAR);}else{self.texture=Some(ctx.load_texture("media_frame",frame,egui::TextureOptions::LINEAR));}
        }else if let Some(info)=&self.info {self.frame_job=Some(decode_frame(info,path,target,self.frame_times.clone(),self.cache.clone(),&self.decoder,ctx));}
    }
    fn seek_frame(&mut self,ctx:&egui::Context,path:&Path) {if let Some(times)=&self.frame_times {let target=times.partition_point(|t|*t<=self.position).saturating_sub(1).min(times.len()-1);self.display_frame(ctx,path,target);}}
    fn play(&mut self,ctx:&egui::Context,path:&Path) {
        self.playlist_ended=false;
        self.initial_seek = None;
        self.frame_job=None;self.error=None;
        if self.resume_position.is_some_and(|position|(position-self.position).abs()<0.000001) {
            if let Some(job)=&self.playback {if !job.done.load(Ordering::Relaxed) {
                if let Some(paused)=job.control.pause_at.lock().unwrap().take(){if let Some(clock)=job.control.clock.lock().unwrap().as_mut(){*clock+=Instant::now().saturating_duration_since(paused.max(*clock));}}
                job.control.paused.store(false,Ordering::Relaxed);self.started=*job.control.clock.lock().unwrap();self.resume_position=None;ctx.request_repaint();return;
            }}
        }
        self.stop_playback();
        // Keep the visible preview until the first decoded frame replaces it.
        if let Some(info)=&self.info {let first=self.frame_times.as_ref().map_or_else(||(self.position*info.fps).round() as usize,|times|times.partition_point(|time|*time<self.position));self.playback_origin=self.position;self.playback=Some(start(info,path,self.position,first,self.volume,&self.decoder,self.selected_audio,self.frame_times.clone(),self.cache.clone(),ctx));self.started=None;}
    }

}
impl Drop for Preview {fn drop(&mut self){self.save_position();self.stop_playback();persist_volume();}}
fn display_menu(ui:&mut egui::Ui,flip_x:&mut bool,flip_y:&mut bool) {
    ui.menu_button(crate::localization::tr("镜像"),|ui| {
        ui.checkbox(flip_x,crate::localization::tr("左右镜像"));
        ui.checkbox(flip_y,crate::localization::tr("上下镜像"));
        if ui.button(crate::localization::tr("无镜像")).clicked(){*flip_x=false;*flip_y=false;ui.close_menu();}
    });
}
fn audio_menu(ui:&mut egui::Ui,info:&Info,selected:&mut Option<u64>) {
    ui.menu_button(crate::localization::tr("详细信息"),|ui| {
        ui.label(format!("{}: {}",crate::localization::tr("容器格式"),info.container));
        ui.label(format!("{}: {}",crate::localization::tr("像素格式"),info.pixel_format));
        ui.label(format!("{}: {}",crate::localization::tr("传递特性"),info.color_transfer));
        ui.label(format!("{}: {}",crate::localization::tr("内嵌字幕"),info.subtitle_count));
    });
    ui.menu_button(crate::localization::tr("音轨 / 语言"),|ui| {
        for track in &info.audio_tracks {if ui.selectable_label(*selected==Some(track.index),track.display_label()).on_hover_text(&track.label).clicked(){*selected=Some(track.index);ui.close_menu();}}
        if info.audio_tracks.is_empty(){ui.label(crate::localization::tr("没有音轨"));}
    });
}
fn paint_rounded_play(ui:&egui::Ui,rect:egui::Rect,color:egui::Color32,radius:f32) {
    let vertices=[rect.left_top(),rect.left_bottom(),rect.right_center()];
    let mut outline=Vec::with_capacity(27);
    for index in 0..3 {
        let vertex=vertices[index];let previous=vertices[(index+2)%3];let next=vertices[(index+1)%3];
        let entry=vertex+(previous-vertex).normalized()*radius;
        let exit=vertex+(next-vertex).normalized()*radius;
        for step in 0..=8 {
            let t=step as f32/8.0;
            let point=entry.to_vec2()*(1.0-t).powi(2)+vertex.to_vec2()*2.0*(1.0-t)*t+exit.to_vec2()*t*t;
            outline.push(point.to_pos2());
        }
    }
    ui.painter().add(egui::Shape::convex_polygon(outline,color,egui::Stroke::NONE));
}
pub(crate) fn volume_control(ui:&mut egui::Ui,path:&Path,volume:&mut f32,remembered:&mut f32,hover:&mut Option<Instant>) {
            let volume_response=ui.add_sized([28.0,28.0],egui::Button::new(""));
            paint_volume_icon(ui,volume_response.rect.shrink(5.0),*volume);
            let popup_id=ui.make_persistent_id(("media_volume",path));
            if volume_response.clicked(){if (*volume)>0.0 {(*remembered)=*volume;(*volume)=0.0;}else{(*volume)=(*remembered).max(0.01);}}
            if volume_response.hovered() || volume_response.has_focus(){(*hover)=Some(Instant::now());ui.memory_mut(|memory|memory.open_popup(popup_id));}
            let mut popup_active=false;
            let volume_response=volume_response.on_hover_text(crate::localization::tr("音量"));
            egui::popup::popup_above_or_below_widget(ui,popup_id,&volume_response,egui::AboveOrBelow::Above,egui::popup::PopupCloseBehavior::CloseOnClickOutside,|ui| {
                ui.horizontal(|ui| {
                    let (rect,response)=ui.allocate_exact_size(egui::vec2(18.0,120.0),egui::Sense::click_and_drag());
                    if response.clicked() || response.dragged(){if let Some(pointer)=response.interact_pointer_pos(){(*volume)=((rect.bottom()-pointer.y)/rect.height()).clamp(0.0,1.0);}}
                    if response.has_focus(){ui.input(|input| {if input.key_pressed(egui::Key::ArrowUp){(*volume)=((*volume)+0.01).min(1.0);}if input.key_pressed(egui::Key::ArrowDown){(*volume)=((*volume)-0.01).max(0.0);}});}
                    if response.clicked(){response.request_focus();}
                    popup_active=response.hovered() || response.dragged() || response.has_focus();
                    if (*volume)>0.0 {(*remembered)=*volume;}
                    let inner=rect.shrink(1.0);let mut fill=inner;fill.min.y=inner.bottom()-inner.height()*(*volume);
                    ui.painter().rect_filled(fill,0.0,ui.visuals().selection.bg_fill);
                    ui.painter().rect_stroke(rect,0.0,egui::Stroke::new(1.0_f32,ui.visuals().widgets.noninteractive.fg_stroke.color),egui::StrokeKind::Inside);
                });
                popup_active|=ui.rect_contains_pointer(ui.min_rect());
            });
            if popup_active {(*hover)=Some(Instant::now());}
            if ui.memory(|memory|memory.is_popup_open(popup_id)) {
                if (*hover).is_some_and(|last|last.elapsed()>Duration::from_millis(250)) {ui.memory_mut(|memory|memory.close_popup());(*hover)=None;}
                else {ui.ctx().request_repaint_after(Duration::from_millis(50));}
            }
}

pub(crate) fn paint_volume_icon(ui:&egui::Ui,rect:egui::Rect,volume:f32) {
    let percent=(volume.clamp(0.0,1.0)*100.0).round() as u32;
    let active=percent.div_ceil(20).min(5);
    let width=rect.width()/9.0;
    for index in 0..5 {
        let height=rect.height()*(0.3+index as f32*0.175);
        let center=egui::pos2(rect.left()+width*(0.5+index as f32*2.0),rect.center().y);
        let color=if index<active {ui.visuals().text_color()}else{ui.visuals().weak_text_color().gamma_multiply(0.45)};
        ui.painter().rect_filled(egui::Rect::from_center_size(center,egui::vec2(width,height)),width*0.5,color);
    }
    if percent==0 {
        let radius=rect.width().min(rect.height())*0.48;
        let stroke=egui::Stroke::new(1.5_f32,ui.visuals().text_color());
        ui.painter().circle_stroke(rect.center(),radius,stroke);
        let offset=egui::vec2(radius*0.7,-radius*0.7);
        ui.painter().line_segment([rect.center()-offset,rect.center()+offset],stroke);
    }
}
pub(crate) fn transport_button(ui:&mut egui::Ui,icon:crate::icons::Icon,label:&str)->bool {
    let response=ui.add_sized([28.0,28.0],egui::Button::new(""));
    crate::icons::paint(ui,icon,response.rect.shrink(5.0),ui.visuals().text_color());
    let clicked=response.clicked();response.on_hover_text(label);clicked
}
pub(crate) fn time(seconds:f64)->String {let ms=(seconds.max(0.0)*1000.0).round() as u64;let seconds=ms/1000;format!("{:02}:{:02}:{:02}.{:03}",seconds/3600,seconds/60%60,seconds%60,ms%1000)}

struct WaveJob {control:Arc<Control>,receiver:mpsc::Receiver<Result<Vec<Vec<f32>>,String>>}
struct IndexJob {control:Arc<Control>,receiver:mpsc::Receiver<Result<Vec<f64>,String>>}
struct FrameJob {control:Arc<Control>,receiver:mpsc::Receiver<Result<(usize,egui::ColorImage),String>>}
fn frame_control()->Arc<Control> {Arc::new(Control {stop:AtomicBool::new(false),volume:AtomicU32::new(0),children:Mutex::new(Vec::new()),clock:Mutex::new(None),video_ready:AtomicBool::new(true),paused:AtomicBool::new(false),pause_at:Mutex::new(None)})}
fn cancel_frame_task(control:&Control) {
    control.stop.store(true,Ordering::Relaxed);
    let children=std::mem::take(&mut *control.children.lock().unwrap());
    std::thread::spawn(move||{for mut child in children {let _=child.kill();let _=child.wait();}});
}
impl Drop for IndexJob {fn drop(&mut self){cancel_frame_task(&self.control);}}
impl Drop for FrameJob {fn drop(&mut self){cancel_frame_task(&self.control);}}
fn index_frames(info:&Info,path:&Path,ctx:&egui::Context)->IndexJob {
    let control=frame_control();let worker=control.clone();let ffprobe=info.ffprobe.clone();let start_time=info.start_time;let path=path.to_owned();let ctx=ctx.clone();let (tx,receiver)=mpsc::channel();
    std::thread::spawn(move|| {
        let result=(|| {
            let mut child=command(ffprobe).args(["-v","error","-select_streams","V:0","-show_frames","-show_entries","frame=best_effort_timestamp_time","-of","csv=p=0"]).arg(path).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;
            let stdout=child.stdout.take().ok_or("无法读取视频帧索引")?;let mut children=worker.children.lock().unwrap();if worker.stop.load(Ordering::Relaxed){let _=child.kill();let _=child.wait();return Err("Cancelled".into());}children.push(child);drop(children);
            let mut times=Vec::new();
            for line in std::io::BufReader::new(stdout).lines() {
                if worker.stop.load(Ordering::Relaxed){return Err("Cancelled".into());}
                let line=line.map_err(|e|e.to_string())?;
                if line.trim().is_empty(){continue;}
                let time=line.split(',').next().and_then(|s|s.parse::<f64>().ok()).filter(|t|t.is_finite()).ok_or("视频帧缺少有效时间戳，无法精确逐帧定位")?;
                if times.last().is_some_and(|previous|time-start_time<*previous){return Err("视频时间戳不连续，无法精确逐帧定位".into());}
                times.push(time-start_time);
                if times.len()>1_000_000 {return Err("视频帧索引超过 1,000,000 帧预算".into());}
            }
            if times.is_empty(){return Err("没有可索引的视频帧".into());}Ok(times)
        })();let _=tx.send(result);ctx.request_repaint();
    });IndexJob {control,receiver}
}
fn decode_frame(info:&Info,path:&Path,index:usize,times:Option<Arc<Vec<f64>>>,cache:Arc<Mutex<FrameCache>>,mode:&str,ctx:&egui::Context)->FrameJob {
    let fps = info.fps;
    let control=frame_control();let worker=control.clone();let ffmpeg=info.ffmpeg.clone();let path=path.to_owned();let size=info.video.unwrap();let mode=mode.to_owned();let ctx=ctx.clone();let (tx,receiver)=mpsc::channel();
    std::thread::spawn(move|| {
        let result=(|| {
            // Accurate input seeking decodes only the prefix from a nearby keyframe.
            // Prefetch neighbouring frames within the shared bounded cache.
            let first=if times.is_some() { index.saturating_sub(2) } else { index };
            let timestamp=times.as_ref().and_then(|times|times.get(first)).copied().unwrap_or_else(|| first as f64 / fps);
            let count=times.as_ref().map_or(1,|times|8.min(times.len().saturating_sub(first)));
            let filter=format!("scale={}:{}:flags=bilinear",size[0],size[1]);
            let mut reader=spawn_decoder(&worker,&ffmpeg,&path,(timestamp-0.000001).max(0.0),false,Some(&mode),&["-map".into(),"0:V:0".into(),"-an".into(),"-vf".into(),filter,"-frames:v".into(),count.to_string(),"-fps_mode".into(),"passthrough".into(),"-pix_fmt".into(),"rgba".into(),"-f".into(),"rawvideo".into()])?;
            let mut bytes=vec![0u8;size[0]*size[1]*4];let mut target=None;
            for offset in 0..count {
                if worker.stop.load(Ordering::Relaxed){return Err("Cancelled".into());}
                if let Err(error)=reader.read_exact(&mut bytes){if target.is_some() && error.kind()==std::io::ErrorKind::UnexpectedEof{break;}return Err(format!("逐帧解码失败: {error}"));}
                let frame=egui::ColorImage::from_rgba_unmultiplied(size,&bytes);
                if first+offset==index{target=Some(frame.clone());}
                cache.lock().unwrap().insert(first+offset,frame);
            }
            Ok((index,target.ok_or("未找到目标视频帧")?))
        })();let _=tx.send(result);ctx.request_repaint();
    });FrameJob {control,receiver}
}
impl Drop for WaveJob {fn drop(&mut self){
    self.control.stop.store(true,Ordering::Relaxed);
    let children=std::mem::take(&mut *self.control.children.lock().unwrap());
    std::thread::spawn(move||{for mut child in children {let _=child.kill();let _=child.wait();}});
}}
fn waveform(info:&Info,path:&Path,ctx:&egui::Context)->WaveJob {
    let control=Arc::new(Control{stop:AtomicBool::new(false),volume:AtomicU32::new(0),children:Mutex::new(Vec::new()),clock:Mutex::new(None),video_ready:AtomicBool::new(true),paused:AtomicBool::new(false),pause_at:Mutex::new(None)});
    let worker=control.clone();let channels=info.channels;let duration=info.duration;let ffmpeg=info.ffmpeg.clone();let path=path.to_owned();let ctx=ctx.clone();
    let (tx,receiver)=mpsc::channel();
    std::thread::spawn(move|| {
        let result=(|| {
            let mut reader=spawn_decoder(&worker,&ffmpeg,&path,0.0,false,None,&["-map".into(),"0:a:0".into(),"-vn".into(),"-ac".into(),channels.to_string(),"-ar".into(),"8000".into(),"-f".into(),"s16le".into()])?;
            let bins=(duration*8000.0).ceil().clamp(2048.0,4_000_000.0) as usize;
            let mut peaks=vec![vec![0.0f32;bins];channels];let mut bytes=vec![0u8;16000*channels];let mut sample=0usize;
            loop {
                if worker.stop.load(Ordering::Relaxed){return Err("Cancelled".into());}
                let mut count=0;while count<bytes.len(){let n=reader.read(&mut bytes[count..]).map_err(|e|e.to_string())?;if n==0{break;}count+=n;}
                if count==0{break;}
                for frame in bytes[..count-count%(channels*2)].chunks_exact(channels*2) {
                    let column=((sample as f64/(duration*8000.0)*bins as f64) as usize).min(bins-1);
                    for channel in 0..channels {let offset=channel*2;let value=i16::from_le_bytes([frame[offset],frame[offset+1]]) as f32/32768.0;peaks[channel][column]=peaks[channel][column].max(value.abs());}
                    sample+=1;
                }
            }
            if sample==0 {return Err("音频波形解码没有输出".into());}Ok(peaks)
        })();
        let _=tx.send(result);ctx.request_repaint();
    });
    WaveJob{control,receiver}
}
// The meter uses the source waveform envelope; it is not an output-device meter.
pub(crate) fn paint_level_meter(ui:&mut egui::Ui,wave:Option<&[Vec<f32>]>,position:f64,duration:f64,trails:&mut [f32;2]) {
    let decay=ui.input(|input|input.stable_dt.clamp(0.0,0.1))*(5.0/60.0);
    let channels=wave.map_or(1,|wave|wave.len());
    for channel in 0..channels {
        ui.horizontal(|ui| {
            let (label_rect,_)=ui.allocate_exact_size(egui::vec2(36.0,18.0),egui::Sense::hover());
            ui.painter().text(label_rect.left_center(),egui::Align2::LEFT_CENTER,if channels==1{"Mono"}else if channel==0{"L"}else{"R"},egui::FontId::proportional(13.0),ui.visuals().text_color());
            let peak=wave.and_then(|wave|wave.get(channel)).and_then(|peaks| {
                let index=if duration>0.0{((position/duration).clamp(0.0,1.0)*peaks.len() as f64) as usize}else{0};
                peaks.get(index.min(peaks.len().saturating_sub(1))).copied()
            });
            let db=peak.filter(|p|*p>0.0).map(|p|20.0*p.log10());
            let label=if wave.is_none(){"— dBFS".to_owned()}else{db.map_or_else(||"−∞ dBFS".into(),|db|format!("{db:.1} dBFS"))};
            let (rect,response)=ui.allocate_exact_size(egui::vec2((ui.available_width()-86.0).max(20.0),18.0),egui::Sense::hover());
            let painter=ui.painter_at(rect);painter.rect_filled(rect,2.0,ui.visuals().extreme_bg_color);
            let level=db.map_or(0.0,|db|((db+60.0)/60.0).clamp(0.0,1.0));
            for segment in 0..100 {
                let fraction=segment as f32/100.0;
                if fraction>=level {break;}
                let color=if fraction<0.75{egui::Color32::from_rgb(35,200,35)}else if fraction<0.9{egui::Color32::from_rgb(230,195,25)}else{egui::Color32::from_rgb(235,50,35)};
                let bar=egui::Rect::from_min_max(egui::pos2(rect.left()+rect.width()*fraction,rect.top()),egui::pos2(rect.left()+rect.width()*((segment+1) as f32/100.0).min(level),rect.bottom()));
                painter.rect_filled(bar,0.0,color);
            }
            for db in [-60.0_f32,-30.0,-12.0,-6.0,0.0] {
                let x=rect.left()+rect.width()*(db+60.0)/60.0;
                painter.line_segment([egui::pos2(x,rect.bottom()-4.0),egui::pos2(x,rect.bottom())],egui::Stroke::new(1.0_f32,ui.visuals().text_color()));
            }
            if wave.is_some() {
                trails[channel]=level.max(trails[channel]-decay);
                if trails[channel]>level {ui.ctx().request_repaint_after(Duration::from_millis(16));}
                let pixels_per_point=ui.ctx().pixels_per_point();let width=2.0/pixels_per_point;
                let x=((rect.left()+rect.width()*trails[channel])*pixels_per_point).round()/pixels_per_point;
                let x=x.clamp(rect.left(),(rect.right()-width).max(rect.left()));
                painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x,rect.top()),egui::vec2(width,rect.height())),0.0,if ui.visuals().dark_mode {egui::Color32::WHITE} else {egui::Color32::from_rgb(64,64,64)});
            } else {trails[channel]=0.0;}
            response.on_hover_text(crate::localization::tr("dBFS 为源音频近似峰值，不受播放音量影响"));
            ui.small(label);
        });
    }
}

#[derive(Clone,Copy,Default)]
struct WaveViewport { offset:f64, span:f64, total:f64 }
impl WaveViewport {
    fn follow(&mut self,position:f64){self.offset=(position-self.span*0.5).clamp(0.0,(self.total-self.span).max(0.0));}
    fn zoom(&mut self,scroll:f64,anchor:f64,minimum:f64){
        let span=(self.span*(-scroll*0.005).exp()).clamp(minimum,self.total);
        self.offset=(self.offset+anchor.clamp(0.0,1.0)*(self.span-span)).clamp(0.0,self.total-span);
        self.span=span;
    }
}

fn wave_view<T>(ui:&mut egui::Ui,wave:&[Vec<f32>],position:f64,duration:f64,follow_playback:bool,paint:impl FnOnce(&mut egui::Ui,&[Vec<f32>],f64,f64,f64)->Option<T>)->Option<(T,f64)> {
    if duration<=0.0{return None;}
    let id=ui.make_persistent_id(("audio_wave_viewport",ui.ctx().viewport_id()));
    let navigation_id=id.with("last_navigation");
    let now=ui.input(|input|input.time);
    let mut last_navigation=ui.ctx().data_mut(|data|data.get_temp::<f64>(navigation_id));
    let mut view=ui.ctx().data_mut(|data|data.get_temp::<WaveViewport>(id)).unwrap_or_default();
    if view.total!=duration||view.span<=0.0 {view=WaveViewport{offset:0.0,span:duration,total:duration};last_navigation=None;}
    let rect=ui.available_rect_before_wrap();
    let left=rect.left()+62.0_f32.min(rect.width()*0.3);
    let width=(rect.right()-left).max(1.0);
    let minimum=(width as f64/90.0*0.01).min(duration);
    if ui.is_enabled(){ui.input_mut(|input|{
        if let Some(pointer)=input.pointer.hover_pos().filter(|p|rect.contains(*p)){
            let scroll=input.smooth_scroll_delta.y;
            if scroll!=0.0 {
                let anchor=((pointer.x-left)/width).clamp(0.0,1.0) as f64;
                view.zoom(scroll as f64,anchor,minimum);
                last_navigation=Some(now);
                input.smooth_scroll_delta=egui::Vec2::ZERO;
            }
        }
    });}
    let zoomed=view.span<duration-0.000001;
    if follow_playback&&zoomed {
        if let Some(last)=last_navigation {
            if now-last>=0.1&&!ui.input(|input|input.pointer.any_down()) {view.follow(position);}
            else {ui.ctx().request_repaint_after(Duration::from_secs_f64((0.1-(now-last)).max(0.001)));}
        }
    }
    let plot=egui::Rect::from_min_max(rect.min,egui::pos2(rect.right(),(rect.bottom()-if zoomed{18.0}else{0.0}).max(rect.top()+1.0)));
    let columns=(width*ui.ctx().pixels_per_point()).ceil() as usize;
    // Aggregate original fine-grained peaks into screen pixels, never stretch the overview bins.
    let visible:Vec<Vec<f32>>=wave.iter().map(|peaks|(0..columns).map(|column|{
        let first=(((view.offset+view.span*column as f64/columns as f64)/duration)*peaks.len() as f64).floor() as usize;
        let last=(((view.offset+view.span*(column+1) as f64/columns as f64)/duration)*peaks.len() as f64).ceil() as usize;
        peaks.get(first..last.max(first+1).min(peaks.len())).unwrap_or(&[]).iter().copied().fold(0.0_f32,f32::max)
    }).collect()).collect();
    let result=ui.scope_builder(egui::UiBuilder::new().max_rect(plot),|ui|paint(ui,&visible,position-view.offset,view.span,view.offset)).inner.map(|value|(value,view.offset));
    if zoomed {
        let track=egui::Rect::from_min_max(egui::pos2(left,rect.bottom()-14.0),rect.max);
        let thumb_width=(track.width()*(view.span/duration) as f32).max(20.0).min(track.width());
        let travel=(track.width()-thumb_width).max(1.0);
        let x=track.left()+travel*(view.offset/(duration-view.span)) as f32;
        let thumb=egui::Rect::from_min_size(egui::pos2(x,track.top()),egui::vec2(thumb_width,track.height()));
        let response=ui.interact(track,id.with("scroll"),egui::Sense::click_and_drag());
        if response.clicked()||response.dragged(){if let Some(pointer)=response.interact_pointer_pos(){
            let grab_id=id.with("grab");
            let grab=if response.drag_started(){let grab=if thumb.contains(pointer){pointer.x-thumb.left()}else{thumb_width*0.5};ui.ctx().data_mut(|d|d.insert_temp(grab_id,grab));grab}else{ui.ctx().data_mut(|d|d.get_temp::<f32>(grab_id)).unwrap_or(thumb_width*0.5)};
            view.offset=(((pointer.x-track.left()-grab)/travel).clamp(0.0,1.0) as f64)*(duration-view.span);
            last_navigation=Some(now);
            ui.ctx().request_repaint();
        }}
        ui.painter().rect_filled(track,4.0,ui.visuals().extreme_bg_color);
        ui.painter().rect_filled(thumb,4.0,ui.visuals().widgets.inactive.fg_stroke.color);
    }
    ui.ctx().data_mut(|data|data.insert_temp(id,view));
    ui.ctx().data_mut(|data|{if let Some(last)=last_navigation{data.insert_temp(navigation_id,last);}else{data.remove::<f64>(navigation_id);}});
    ui.allocate_rect(rect,egui::Sense::hover());
    result
}

pub(crate) fn paint_trim_waveform(ui:&mut egui::Ui,wave:&[Vec<f32>],position:f64,duration:f64,start:&mut f64,end:&mut f64,interval:bool,playing:bool)->Option<f64> {
    wave_view(ui,wave,position,duration,playing,|ui,wave,position,span,origin|{
        let mut a=*start-origin;let mut b=*end-origin;
        let before=(a,b);
        let seek=paint_trim_waveform_local(ui,wave,position,span,&mut a,&mut b,interval,origin);
        if a!=before.0{*start=(a+origin).clamp(0.0,duration);}
        if b!=before.1{*end=(b+origin).clamp(0.0,duration);}
        seek
    }).map(|(seek,origin)|seek+origin)
}

fn paint_trim_waveform_local(ui:&mut egui::Ui,wave:&[Vec<f32>],position:f64,duration:f64,start:&mut f64,end:&mut f64,interval:bool,origin:f64)->Option<f64> {
    let rect=ui.available_rect_before_wrap();
    let left=rect.left()+62.0_f32.min(rect.width()*0.3);
    let width=(rect.right()-left).max(1.0);
    let bottom=(rect.bottom()-26.0).max(rect.top());
    let center=(rect.top()+bottom)*0.5;
    let background=ui.interact(egui::Rect::from_min_max(egui::pos2(left,rect.top()),egui::pos2(rect.right(),bottom)),ui.make_persistent_id("trim_range_click"),egui::Sense::click());
    let mut boundary_active=false;
    if duration>0.0 {
        for first in [true,false] {
            let time=if first {*start}else{*end};
            if time<0.0||time>duration {continue;}
            let x=left+width*(time/duration) as f32;
            // The triangle sits outside the kept range and points inward.
            let hit=egui::Rect::from_min_max(egui::pos2(x-10.0,rect.top()),egui::pos2(x+10.0,bottom));
            let grip=egui::Rect::from_center_size(egui::pos2(x+if first {-18.0}else{18.0},center),egui::vec2(32.0,36.0));
            let response=ui.interact(hit,ui.make_persistent_id(("trim_boundary",first)),egui::Sense::click_and_drag()).on_hover_text(crate::localization::tr(if interval {if first {"区间起点"}else{"区间终点"}}else if first {"保留起点"}else{"保留终点"})).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            let grip_response=ui.interact(grip,ui.make_persistent_id(("trim_grip",first)),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            boundary_active|=ui.input(|input|input.pointer.hover_pos().is_some_and(|pos|hit.contains(pos)||grip.contains(pos)));
            boundary_active|=response.hovered() || response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped() || grip_response.hovered() || grip_response.is_pointer_button_down_on() || grip_response.dragged() || grip_response.drag_stopped();
            if response.dragged() || response.clicked() || grip_response.dragged() {
                if let Some(pointer)=response.interact_pointer_pos().or_else(||grip_response.interact_pointer_pos()) {
                    let offset=if grip_response.dragged(){if first {-18.0}else{18.0}}else{0.0};
                    let time=((pointer.x-offset-left)/width).clamp(0.0,1.0) as f64*duration;
                    let gap=0.001_f64.min(duration);
                    if first {*start=if interval{time}else{time.clamp(0.0,(*end-gap).max(0.0))};}
                    else {*end=if interval{time}else{time.clamp((*start+gap).min(duration),duration)};}
                }
            }
        }
    }
    if duration>0.0 && !boundary_active {
        if let Some(pointer)=background.interact_pointer_pos() {
            let time=((pointer.x-left)/width).clamp(0.0,1.0) as f64*duration;
            let gap=0.001_f64.min(duration);
            let playhead=left+width*(position/duration) as f32;
            if background.clicked_by(egui::PointerButton::Primary) && !(playhead-14.0..=playhead+30.0).contains(&pointer.x) {*start=if interval{time}else{time.clamp(0.0,(*end-gap).max(0.0))};}
            if background.clicked_by(egui::PointerButton::Secondary) {*end=if interval{time}else{time.clamp((*start+gap).min(duration),duration)};}
        }
    }
    let seek=paint_waveform_interactive(ui,wave,position,duration,!boundary_active,false,origin);
    if duration>0.0 {
        let x1=left+width*(*start/duration) as f32;
        let x2=left+width*(*end/duration) as f32;
        let painter=ui.painter_at(egui::Rect::from_min_max(egui::pos2(left,rect.top()),rect.max));
        let masks=if interval{vec![(x1.min(x2),x1.max(x2))]}else{vec![(left,x1),(x2,rect.right())]};
        for (a,b) in masks {
            if b>a {painter.rect_filled(egui::Rect::from_min_max(egui::pos2(a,rect.top()),egui::pos2(b,bottom)),0.0,egui::Color32::from_black_alpha(77));}
        }
        let color=egui::Color32::from_rgb(45,255,95);
        for (x,first) in [(x1,true),(x2,false)] {
            painter.line_segment([egui::pos2(x,rect.top()),egui::pos2(x,bottom+5.0)],egui::Stroke::new(1.5_f32,color));
            let tip=x+if first {-5.0}else{5.0};
            let base=x+if first {-26.333334}else{26.333334};
            painter.add(egui::Shape::convex_polygon(vec![egui::pos2(base,center-10.666667),egui::pos2(tip,center),egui::pos2(base,center+10.666667)],color,egui::Stroke::NONE));
        }
    }
    seek.map(|seek|seek.position)
}

struct WaveSeek {position:f64,started:bool,finished:bool}
fn paint_waveform(ui:&mut egui::Ui,wave:&[Vec<f32>],position:f64,duration:f64,playing:bool)->Option<WaveSeek> {
    wave_view(ui,wave,position,duration,playing,|ui,wave,position,span,origin|paint_waveform_interactive(ui,wave,position,span,true,true,origin)).map(|(mut seek,origin)|{seek.position+=origin;seek})
}
fn paint_waveform_interactive(ui:&mut egui::Ui,wave:&[Vec<f32>],mut position:f64,duration:f64,allow_seek:bool,allow_background:bool,origin:f64)->Option<WaveSeek> {
    let height=ui.available_height().max(1.0);
    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),height),egui::Sense::hover());
    response.on_hover_text(crate::localization::tr("dBFS 为源音频近似峰值，不受播放音量影响"));
    let painter=ui.painter_at(rect);painter.rect_filled(rect,4.0,ui.visuals().extreme_bg_color);
    let plot_left=rect.left()+62.0_f32.min(rect.width()*0.3);let plot_width=(rect.right()-plot_left).max(1.0);
    let wave_height=(height-26.0).max(1.0);
    let mut gesture=None;
    if duration>0.0 && allow_seek {
        let x=plot_left+(position/duration) as f32*plot_width;
        let handle_rect=egui::Rect::from_min_max(egui::pos2(x-14.0,rect.top()),egui::pos2(x+30.0,rect.top()+wave_height+5.0)).intersect(egui::Rect::from_min_max(egui::pos2(plot_left,rect.top()),rect.max));
        let handle=ui.interact(handle_rect,ui.make_persistent_id("waveform_playhead"),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        // Disjoint hit regions: a press on the playhead cannot also become a
        // background seek, even when the drag leaves the handle's original area.
        for (side,left,right) in [(0,plot_left,handle_rect.left().clamp(plot_left,rect.right())),(1,handle_rect.right().clamp(plot_left,rect.right()),rect.right())] {
            if !allow_background {continue;}
            if right<=left{continue;}
            let area=egui::Rect::from_min_max(egui::pos2(left,rect.top()),egui::pos2(right,rect.bottom()));
            let click=ui.interact(area,ui.make_persistent_id(("waveform_seek_area",side)),egui::Sense::click());
            if allow_background && !handle.is_pointer_button_down_on() && !handle.dragged() && !handle.drag_stopped() && !handle.clicked() && click.clicked_by(egui::PointerButton::Primary) {
                if let Some(pointer)=click.interact_pointer_pos(){position=((pointer.x-plot_left)/plot_width).clamp(0.0,1.0) as f64*duration;gesture=Some(WaveSeek {position,started:true,finished:true});}
            }
        }
        if handle.drag_started() || handle.dragged() || handle.drag_stopped() || handle.clicked() {
            if let Some(pointer)=handle.interact_pointer_pos() {
                position=((pointer.x-plot_left)/plot_width).clamp(0.0,1.0) as f64*duration;
                gesture=Some(WaveSeek {position,started:handle.drag_started() || handle.clicked(),finished:handle.drag_stopped() || handle.clicked()});
            }
        }
    }
    for (channel,peaks) in wave.iter().enumerate() {
        let row_height=wave_height/wave.len() as f32;let top=rect.top()+row_height*channel as f32;let center=top+row_height*0.5;let full_height=row_height*0.4;
        let grid=ui.visuals().widgets.noninteractive.bg_stroke.color;
        painter.line_segment([egui::pos2(plot_left,center),egui::pos2(rect.right(),center)],egui::Stroke::new(1.0_f32,grid));
        // The waveform remains linear in amplitude: dB ticks are placed at
        // 10^(dB/20), rather than spacing dB values evenly on a linear plot.
        for db in [0.0_f32,-6.0,-12.0] {
            if db== -12.0 && row_height<120.0 {continue;}
            let offset=10.0_f32.powf(db/20.0)*full_height;
            for sign in [-1.0_f32,1.0] {
                let y=center+sign*offset;
                painter.line_segment([egui::pos2(plot_left,y),egui::pos2(rect.right(),y)],egui::Stroke::new(0.5_f32,grid));
                if row_height>=70.0 {painter.text(egui::pos2(plot_left-5.0,y),egui::Align2::RIGHT_CENTER,format!("{db:.0} dB"),egui::FontId::proportional(10.0),ui.visuals().weak_text_color());}
            }
        }
        if row_height>=100.0 {painter.text(egui::pos2(plot_left-5.0,center),egui::Align2::RIGHT_CENTER,"−∞",egui::FontId::proportional(10.0),ui.visuals().weak_text_color());}
        // A shared-edge triangle mesh avoids antialiased line fringes between columns.
        // Use physical pixel boundaries so fractional DPI does not introduce periodic gaps.
        let mut mesh=egui::Mesh::default();
        let scale=ui.ctx().pixels_per_point();
        for (index,peak) in peaks.iter().enumerate(){
            let left=wave_column_edge(plot_left,plot_width,index,peaks.len(),scale);
            let right=wave_column_edge(plot_left,plot_width,index+1,peaks.len(),scale);
            let amplitude=peak.clamp(0.0,1.0)*full_height;
            mesh.add_colored_rect(egui::Rect::from_min_max(egui::pos2(left,center-amplitude),egui::pos2(right,center+amplitude)),egui::Color32::from_rgb(45,125,250));
        }
        painter.add(egui::Shape::mesh(mesh));
        let channel_label=if wave.len()==1{"Mono"}else if channel==0{"L"}else{"R"};
        painter.text(egui::pos2(plot_left+8.0,top+3.0),egui::Align2::LEFT_TOP,channel_label,egui::FontId::proportional(12.0),ui.visuals().text_color());
    }
    // Disabling playhead input while a trim boundary is dragged must not hide
    // the ruler, grid or current playback cursor.
    if duration>0.0 {
        let ruler_top=rect.top()+wave_height;
        let color=ui.visuals().weak_text_color();
        painter.line_segment([egui::pos2(plot_left,ruler_top),egui::pos2(rect.right(),ruler_top)],egui::Stroke::new(1.0_f32,color));
        // Choose readable 1/2/5 intervals according to the available width.
        let target=duration/(plot_width as f64/90.0).max(1.0);
        let magnitude=10.0_f64.powf(target.log10().floor());
        let ratio=target/magnitude;
        let step=magnitude*if ratio<=1.0{1.0}else if ratio<=2.0{2.0}else if ratio<=5.0{5.0}else{10.0};
        let count=(duration/step).floor() as usize;
        for tick in 0..=count.min(100) {
            let seconds=tick as f64*step;let x=plot_left+(seconds/duration) as f32*plot_width;
            painter.line_segment([egui::pos2(x,rect.top()),egui::pos2(x,ruler_top)],egui::Stroke::new(0.5_f32,ui.visuals().widgets.noninteractive.bg_stroke.color));
            painter.line_segment([egui::pos2(x,ruler_top),egui::pos2(x,ruler_top+5.0)],egui::Stroke::new(1.0_f32,color));
            if tick==0 || rect.right()-x>75.0 {
                painter.text(egui::pos2(x,ruler_top+8.0),if tick==0{egui::Align2::LEFT_TOP}else{egui::Align2::CENTER_TOP},ruler_time(seconds+origin,step),egui::FontId::proportional(11.0),color);
            }
            for minor in 1..5 {let t=seconds+step*minor as f64/5.0;if t>=duration{break;}let x=plot_left+(t/duration) as f32*plot_width;
                painter.line_segment([egui::pos2(x,ruler_top),egui::pos2(x,ruler_top+3.0)],egui::Stroke::new(0.5_f32,color));}
        }
        if plot_width>=150.0 {painter.text(egui::pos2(rect.right(),ruler_top+8.0),egui::Align2::RIGHT_TOP,ruler_time(duration+origin,step),egui::FontId::proportional(11.0),color);}
        let painter=ui.painter_at(egui::Rect::from_min_max(egui::pos2(plot_left,rect.top()),rect.max));
        let x=plot_left+(position/duration) as f32*plot_width;
        let center_y=rect.top()+wave_height*0.5;
        let handle=egui::Rect::from_center_size(egui::pos2(x+2.0,center_y),egui::vec2(25.6,25.6));
        let stroke=egui::Stroke::new(1.5_f32,egui::Color32::from_rgb(255,175,40));
        // Match the triangle edges where the vertical cursor intersects the
        // triangle, rather than its rectangular bounds. Allow for stroke thickness.
        let triangle_left=handle.left()+handle.width()*2.0/24.0;
        let triangle_right=handle.left()+handle.width()*22.0/24.0;
        let fraction=((x-triangle_left)/(triangle_right-triangle_left)).clamp(0.0,1.0);
        let half_height=handle.height()*10.0/24.0*(1.0-fraction);
        let gap=2.0+stroke.width*0.5;
        let upper=center_y-half_height-gap;let lower=center_y+half_height+gap;
        if upper>rect.top(){paint_smooth_cursor(&painter,x,rect.top(),upper,stroke);}
        if lower<ruler_top+5.0{paint_smooth_cursor(&painter,x,lower,ruler_top+5.0,stroke);}
        let triangle_top=handle.top()+handle.height()*2.0/24.0;
        let triangle_bottom=handle.top()+handle.height()*22.0/24.0;
        painter.add(egui::Shape::convex_polygon(vec![egui::pos2(triangle_left,triangle_top),egui::pos2(triangle_left,triangle_bottom),egui::pos2(triangle_right,center_y)],stroke.color,egui::Stroke::NONE));
    }
    gesture
}

fn wave_column_edge(left:f32,width:f32,index:usize,count:usize,scale:f32)->f32 {
    ((left+width*index as f32/count.max(1) as f32)*scale).round()/scale
}

fn paint_smooth_cursor(painter:&egui::Painter,x:f32,top:f32,bottom:f32,stroke:egui::Stroke) {
    let half=stroke.width*0.5;
    // A filled polygon retains fractional positions; thin line primitives may
    // snap to physical pixels, making continuous motion look stepped.
    painter.add(egui::Shape::convex_polygon(vec![egui::pos2(x-half,top),egui::pos2(x-half,bottom),egui::pos2(x+half,bottom),egui::pos2(x+half,top)],stroke.color,egui::Stroke::NONE));
}
fn ruler_time(seconds:f64,step:f64)->String {
    let full=time(seconds);
    let clock=if seconds>=3600.0{&full[..8]}else{&full[3..8]};
    if step<1.0 {format!("{clock}.{}",&full[9..12])}else{clock.to_owned()}
}
fn fit_scale(native:egui::Vec2,available:egui::Vec2)->f32 {(available.x/native.x).min(available.y/native.y)}
#[cfg(test)]
mod tests {
    #[test]
    fn waveform_follow_centers_playhead_and_clamps_at_both_ends(){
        let mut view=super::WaveViewport{offset:0.0,span:10.0,total:80.0};
        view.follow(23.0);assert_eq!(view.offset,18.0);
        view.follow(2.0);assert_eq!(view.offset,0.0);
        view.follow(79.0);assert_eq!(view.offset,70.0);
        view.span=80.0;view.follow(40.0);assert_eq!(view.offset,0.0);
    }
    #[test]
    fn stop_rewinds_and_clears_pending_trim_playback(){
        let mut player=super::Preview::default();
        player.position=13.457;player.initial_seek=Some(8.0);player.resume_position=Some(13.457);
        player.trim_preview_end=Some(20.0);player.trim_preview_next=Some((25.0,30.0));
        player.stop_and_rewind();
        assert_eq!(player.position_now(),0.0);
        assert!(!player.is_playing());
        assert!(player.initial_seek.is_none()&&player.resume_position.is_none());
        assert!(player.trim_preview_end.is_none()&&player.trim_preview_next.is_none());
    }
    #[test]
    fn waveform_columns_cover_fractional_dpi_without_gaps(){
        for scale in [1.0,1.25,1.5,2.0] {
            let count=(721.37_f32*scale).ceil() as usize;
            let edges:Vec<_>=(0..=count).map(|i|super::wave_column_edge(71.23,721.37,i,count,scale)).collect();
            for pair in edges.windows(2){
                assert!(pair[1]>=pair[0]);
                let pixels=(pair[1]-pair[0])*scale;
                assert!(pixels<=1.001 && (pixels-pixels.round()).abs()<0.001);
            }
        }
    }
    #[test]
    fn waveform_zoom_preserves_anchor_and_limits_visible_range(){
        let mut view=super::WaveViewport{offset:0.0,span:80.0,total:80.0};
        let anchor=0.37;
        let time=view.offset+view.span*anchor;
        view.zoom(120.0,anchor,0.1);
        assert!((view.offset+view.span*anchor-time).abs()<1e-10);
        view.zoom(100000.0,anchor,0.1);
        assert_eq!(view.span,0.1);
        assert!(view.offset>=0.0&&view.offset+view.span<=view.total);
        view.zoom(-100000.0,anchor,0.1);
        assert_eq!(view.span,80.0);assert_eq!(view.offset,0.0);
    }
    #[test]
    fn detached_trim_has_independent_transport_and_waveform() {
        let mut source=super::Preview::default();
        source.position=23.0;source.autoplay_pending=true;source.playlist_mode=true;
        source.volume=0.35;source.waveform=Some(vec![vec![0.2,0.7]]);
        let mut editor=source.detached_trim_player(60.0);
        assert_eq!(editor.position,0.0);
        assert!(!editor.autoplay_pending && !editor.playlist_mode);
        assert!(editor.history_file.is_none() && editor.playback.is_none());
        assert!(editor.trim.is_open());
        editor.position=7.0;editor.volume=0.8;
        editor.waveform.as_mut().unwrap()[0][0]=0.9;
        assert_eq!(source.position,23.0);
        assert_eq!(source.volume,0.35);
        assert_eq!(source.waveform.as_ref().unwrap()[0][0],0.2);
    }
    #[test]
    fn multiple_trim_sessions_outlive_the_source_player() {
        let source=super::Preview::default();
        let a=std::path::PathBuf::from("trim-session-a.wav");
        let b=std::path::PathBuf::from("trim-session-b.wav");
        super::TRIM_EDITORS.with(|editors| {
            let mut editors=editors.borrow_mut();
            editors.insert(a.clone(),source.detached_trim_player(10.0));
            editors.insert(b.clone(),source.detached_trim_player(20.0));
        });
        drop(source);
        super::TRIM_EDITORS.with(|editors| {
            let mut editors=editors.borrow_mut();
            assert!(editors.get(&a).unwrap().trim.is_open());
            assert!(editors.get(&b).unwrap().trim.is_open());
            editors.remove(&a);
            assert!(editors.contains_key(&b));
            editors.remove(&b);
        });
    }

    use super::*;
    #[test]
    fn frame_cache_respects_budget_and_recent_access() {
        let mut cache=FrameCache::default();
        let frame=egui::ColorImage::new([2,2],egui::Color32::BLACK);
        cache.insert_bounded(1,frame.clone(),32);cache.insert_bounded(2,frame.clone(),32);
        assert!(cache.get(1).is_some());cache.insert_bounded(3,frame.clone(),32);
        assert!(cache.get(2).is_none());assert!(cache.get(1).is_some());assert_eq!(cache.bytes,32);
        cache.insert_bounded(1,frame,32);assert_eq!(cache.bytes,32);
        cache.insert_bounded(9,egui::ColorImage::new([4,4],egui::Color32::BLACK),32);
        assert!(cache.get(9).is_none());assert_eq!(cache.bytes,32);
    }
    #[test]
    fn mp4_header_recognizes_brands_and_rejects_text_and_images() {
        assert!(is_mp4_header(b"\0\0\0\x18ftypisom\0\0\0\0isommp42"));
        assert!(!is_mp4_header(b"ordinary text ftypisom"));
        assert!(!is_mp4_header(b"\0\0\0\x14ftypavif\0\0\0\0avif"));
        assert!(!is_mp4_header(b"\0\0\0\x18ftypisom"));
        assert!(is_mp4_header(b"\0\0\0\x08free\0\0\0\x18ftypisom\0\0\0\0isommp42"));
    }
    #[test]
    fn media_extensions_are_case_insensitive_and_do_not_capture_yaml() {
        for name in ["clip.MP4","audio.FLAC","sound.wav","movie.webm"] {assert!(is_media(Path::new(name)));}
        assert!(!is_media(Path::new("scene.prefab")));
    }
    #[test]
    fn fit_fills_one_dimension_without_cropping_or_stretching() {
        let wide=egui::vec2(1920.0,1080.0);let bounds=egui::vec2(800.0,600.0);
        let size=wide*fit_scale(wide,bounds);assert_eq!(size,egui::vec2(800.0,450.0));
        let tall=egui::vec2(1080.0,1920.0);let size=tall*fit_scale(tall,bounds);
        assert!((size.y-600.0).abs()<0.001);assert!(size.x<800.0);
        assert_eq!(time(3661.123),"01:01:01.123");
    }
}
