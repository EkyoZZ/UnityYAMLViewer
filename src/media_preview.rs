//! External FFmpeg decoding with bounded frame/audio queues. No codec is bundled.
use std::{io::{Read,BufRead}, path::{Path,PathBuf}, process::{Child,Stdio}, sync::{Arc,Mutex,mpsc,atomic::{AtomicBool,AtomicU32,Ordering}}, time::{Duration,Instant}};
use eframe::egui;
use crate::media_tools::{command,find_tool};

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
struct AudioTrack {index:u64,label:String,default:bool}
struct Info { ffmpeg:PathBuf,ffprobe:PathBuf,duration:f64,start_time:f64,video:Option<[usize;2]>,audio:bool,channels:usize,sample_rate:u64,bits:u64,source_channels:u64,file_size:u64,fps:f64,audio_tracks:Vec<AudioTrack>,video_codec:String,pixel_format:String,container:String,bit_rate:u64,subtitle_count:usize,sample_aspect:String,video_bits:Option<u64>,dynamic_range:String,color_transfer:String }
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
        let title=s["tags"]["title"].as_str().unwrap_or("");
        let codec=s["codec_name"].as_str().unwrap_or("—").to_owned();
        let rate=s["sample_rate"].as_str().and_then(|s|s.parse().ok()).unwrap_or(0);
        let channels=s["channels"].as_u64().unwrap_or(0);let default=s["disposition"]["default"]==1;
        AudioTrack {index,label:format!("{} · {language} {} · {codec} · {rate} Hz · {channels} ch",ordinal+1,title),default}
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
    Ok(Info {ffmpeg,ffprobe,start_time,duration:if duration.is_finite(){duration.max(0.0)}else{0.0},video,audio,channels,sample_rate,bits,source_channels,file_size,fps,audio_tracks,video_codec,pixel_format,container,bit_rate,subtitle_count,sample_aspect,video_bits,dynamic_range,color_transfer})
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
pub struct Preview { info:Option<Info>,probe:Option<mpsc::Receiver<Result<Info,String>>>,playback:Option<Playback>,texture:Option<egui::TextureHandle>,poster:Option<egui::ColorImage>,error:Option<String>,position:f64,started:Option<Instant>,volume:f32,remembered_volume:f32,volume_hover:Option<Instant>,
    waveform:Option<Vec<Vec<f32>>>,wave_job:Option<WaveJob>,wave_error:Option<String>,scale_mode:String,zoom:Option<f32>,pan:egui::Vec2,flip_x:bool,flip_y:bool,decoder:String,
    frame_times:Option<Arc<Vec<f64>>>,index_job:Option<IndexJob>,index_error:Option<String>,frame_job:Option<FrameJob>,current_frame:Option<usize>,selected_audio:Option<u64>,seek_drag:Option<(f64,bool)>,cache:Arc<Mutex<FrameCache>>,playback_origin:f64,resume_position:Option<f64> }
impl Default for Preview {fn default()->Self {let preference=*volume_preference().lock().unwrap();Self {info:None,probe:None,playback:None,texture:None,poster:None,error:None,position:0.0,started:None,volume:preference.volume,remembered_volume:preference.remembered,volume_hover:None,waveform:None,wave_job:None,wave_error:None,scale_mode:"fit".into(),zoom:None,pan:egui::Vec2::ZERO,flip_x:false,flip_y:false,decoder:"auto".into(),frame_times:None,index_job:None,index_error:None,frame_job:None,current_frame:None,selected_audio:None,seek_drag:None,cache:Arc::new(Mutex::new(FrameCache::default())),playback_origin:0.0,resume_position:None}}}
impl Preview {
    pub fn header_controls(&mut self,ui:&mut egui::Ui,path:&Path) {
        if !self.info.as_ref().is_some_and(|info|info.video.is_some()){return;}
        let previous=self.decoder.clone();
        crate::theme::video_decoder_selector(ui,&mut self.decoder);
        let mirror=match (self.flip_x,self.flip_y){(false,false)=>"无镜像",(true,false)=>"左右镜像",(false,true)=>"上下镜像",(true,true)=>"上下左右镜像"};
        ui.small(crate::localization::tr(mirror));
        let previous_scale=self.scale_mode.clone();
        crate::theme::video_scale_selector(ui,&mut self.scale_mode);
        if previous_scale!=self.scale_mode {
            self.pan=egui::Vec2::ZERO;
            if self.scale_mode=="native"{self.zoom=Some(1.0);}
            if self.scale_mode=="fit"{self.zoom=None;}
        }
        if let Some(zoom)=self.zoom {ui.small(format!("{:.1}%",zoom*100.0));}
        if self.index_job.is_some(){ui.spinner();ui.label(crate::localization::tr("正在索引视频帧…"));}
        if previous!=self.decoder {let playing=self.is_playing();self.stop_playback();self.cache=Arc::new(Mutex::new(FrameCache::default()));if playing{self.play(ui.ctx(),path);}else{self.error=None;}}
    }
    pub fn file_size(&self)->Option<u64> {self.info.as_ref().map(|info|info.file_size)}
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
    pub fn deactivate(&mut self) {
        self.stop_playback();
        if let Some(frame)=self.current_frame.and_then(|index|self.cache.lock().unwrap().get(index)){self.poster=Some(preview_poster(&frame));}
        self.texture=None;self.wave_job=None;self.index_job=None;self.frame_job=None;self.cache=Arc::new(Mutex::new(FrameCache::default()));}
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path) {
        let preference=*volume_preference().lock().unwrap();self.volume=preference.volume;self.remembered_volume=preference.remembered;
        if self.info.is_none() && self.probe.is_none() && self.error.is_none() {
            let (tx,rx)=mpsc::channel();self.probe=Some(rx);let path=path.to_owned();let ctx=ui.ctx().clone();
            std::thread::spawn(move|| {let _=tx.send(probe(&path));ctx.request_repaint();});
        }
        if let Some(receiver)=&self.probe {match receiver.try_recv() {
            Ok(result)=>{self.probe=None;match result {Ok(info)=>{self.selected_audio=info.audio_tracks.iter().find(|t|t.default).or_else(||info.audio_tracks.first()).map(|t|t.index);self.info=Some(info);},Err(error)=>self.error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.probe=None;self.error=Some("媒体信息读取失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if self.probe.is_some() {ui.spinner();ui.label(crate::localization::tr("正在读取媒体信息…"));ui.ctx().request_repaint_after(Duration::from_millis(100));return;}
        let mut failed=false;
        if let Some(job)=&self.playback {
            self.started=if job.control.paused.load(Ordering::Relaxed){None}else{*job.control.clock.lock().unwrap()};
            while !job.control.paused.load(Ordering::Relaxed) {let Ok((index,frame))=job.frames.try_recv() else{break};self.current_frame=Some(index);if let Some(texture)=&mut self.texture {texture.set(frame,egui::TextureOptions::LINEAR);}else{self.texture=Some(ui.ctx().load_texture("media_frame",frame,egui::TextureOptions::LINEAR));}}
            if let Ok(error)=job.errors.try_recv() {self.error=Some(error);failed=true;}
            if job.done.load(Ordering::Relaxed) && self.error.is_none() {self.stop_playback();if let Some(info)=&self.info {self.position=info.duration;}}
        }
        if failed {self.stop_playback();}
        if let Some(error)=&self.error {ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap());
            if ui.button(crate::localization::tr("前往视频解码设置")).clicked(){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("media_settings_request"),true));}
            if ui.button(crate::localization::tr("重试")).clicked(){self.stop_playback();self.cache=Arc::new(Mutex::new(FrameCache::default()));self.error=None;self.poster=None;self.texture=None;self.info=None;self.wave_job=None;self.waveform=None;self.wave_error=None;self.frame_times=None;self.index_job=None;self.index_error=None;self.frame_job=None;self.current_frame=None;}}
        let Some(info)=&self.info else{return};let duration=info.duration;
        if info.video.is_some() && self.texture.is_none() && !self.is_playing() {
            if let Some(poster)=&self.poster {self.texture=Some(ui.ctx().load_texture("media_frame",poster.clone(),egui::TextureOptions::LINEAR));}
            else if self.frame_job.is_none() && self.error.is_none() {
                self.frame_job=Some(decode_frame(info,path,0,None,self.cache.clone(),&self.decoder,ui.ctx()));
            }
        }
        if info.video.is_some() && self.frame_times.is_none() && self.index_job.is_none() && self.index_error.is_none() {self.index_job=Some(index_frames(info,path,ui.ctx()));}
        let mut indexed_now=false;
        if let Some(job)=&self.index_job {match job.receiver.try_recv(){
            Ok(result)=>{self.index_job=None;match result {Ok(times)=>{self.frame_times=Some(Arc::new(times));indexed_now=true;},Err(error)=>self.index_error=Some(error)}},
            Err(mpsc::TryRecvError::Disconnected)=>{self.index_job=None;self.index_error=Some("视频帧索引失败".into());},Err(mpsc::TryRecvError::Empty)=>{}
        }}
        if let Some(job)=&self.frame_job {match job.receiver.try_recv(){
            Ok(result)=>{self.frame_job=None;match result {Ok((index,frame))=>{self.poster=Some(preview_poster(&frame));self.current_frame=Some(index);self.position=self.frame_times.as_ref().and_then(|times|times.get(index)).copied().unwrap_or(self.position);if let Some(texture)=&mut self.texture {texture.set(frame,egui::TextureOptions::LINEAR);}else{self.texture=Some(ui.ctx().load_texture("media_frame",frame,egui::TextureOptions::LINEAR));}},Err(error)=>self.error=Some(error)}},
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
        let mut position=self.seek_drag.map_or(self.position_now(),|seek|seek.0);if duration>0.0 {position=position.min(duration);}
        if let Some(info)=&self.info {if let Some([w,h])=info.video {
            ui.horizontal_wrapped(|ui| {
                let bitrate=if info.bit_rate>0{format!("{:.0} kb/s",info.bit_rate as f64/1000.0)}else{crate::localization::tr("未知").to_owned()};
                let bits=info.video_bits.map_or_else(||crate::localization::tr("未知").to_owned(),|bits|format!("{bits} bit"));
                ui.small(format!("{}: {} · {:.2} fps · {} · {w} × {h} · {}: {} · {} · {}",crate::localization::tr("视频信息"),info.video_codec,info.fps,bitrate,crate::localization::tr("像素宽高比"),crate::localization::tr(&info.sample_aspect),bits,crate::localization::tr(&info.dynamic_range)));
            });
            ui.horizontal_wrapped(|ui| {
                ui.small(format!("{}:",crate::localization::tr("音频信息")));
                if let Some(track)=info.audio_tracks.iter().find(|t|Some(t.index)==self.selected_audio) {
                    ui.small(format!("{}: {}",crate::localization::tr("当前音轨"),track.label));
                }else {ui.small(crate::localization::tr(if info.audio_tracks.is_empty(){"没有音轨"}else{"关闭音频"}));}
                ui.small(format!("{}: {}",crate::localization::tr("音轨"),info.audio_tracks.len()));
            });
        }else if info.audio {
            let depth=if info.bits>0{format!("{} bit",info.bits)}else{"bit: —".into()};
            let channels=match info.source_channels {1=>crate::localization::tr("单声道").to_owned(),2=>crate::localization::tr("双声道").to_owned(),n=>format!("{n} ch")};
            ui.small(format!("{} Hz · {depth} · {channels}",info.sample_rate));
        }}
        if let Some(job)=&self.playback {job.control.volume.store(self.volume.to_bits(),Ordering::Relaxed);let fps=self.info.as_ref().map_or(30.0,|i|i.fps);ui.ctx().request_repaint_after(Duration::from_secs_f64((0.5/fps).clamp(0.001,0.03)));}
        if self.info.as_ref().is_some_and(|i|i.video.is_none()) {
            paint_level_meter(ui,self.waveform.as_deref(),position,duration);
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
            if let Some(error)=&self.index_error {ui.small(error);}

        }
        let wave_seek=self.waveform.as_ref().and_then(|wave|paint_waveform(ui,wave,position,duration));
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
            let available=egui::vec2(ui.available_width(),(ui.available_height()-32.0).max(1.0));let native=self.info.as_ref().and_then(|info|info.video).map_or(texture.size_vec2(),|[w,h]|egui::vec2(w as f32,h as f32));
            let free=self.scale_mode=="free";
            let (canvas,response)=ui.allocate_exact_size(available,if free{egui::Sense::drag()}else{egui::Sense::click()});
            let mut zoom=match self.scale_mode.as_str(){
                "native"=>1.0,
                "free"=>self.zoom.unwrap_or_else(||fit_scale(native,canvas.size()).clamp(0.01,64.0)),
                _=>fit_scale(native,canvas.size()),
            };
            if self.zoom!=Some(zoom){self.zoom=Some(zoom);ui.ctx().request_repaint();}
            if !free{self.pan=egui::Vec2::ZERO;}
            if free && response.dragged_by(egui::PointerButton::Primary){self.pan+=response.drag_delta();}
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
            if (zoom-1.0).abs()<f32::EPSILON {
                let pixels_per_point=ui.ctx().pixels_per_point();
                let aligned=egui::pos2((image_rect.min.x*pixels_per_point).round()/pixels_per_point,(image_rect.min.y*pixels_per_point).round()/pixels_per_point);
                image_rect=egui::Rect::from_min_size(aligned,image_rect.size());
            }
            video_clicked=!free && response.clicked_by(egui::PointerButton::Primary) && response.interact_pointer_pos().is_some_and(|point|image_rect.contains(point));
            ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect())).image(texture.id(),image_rect,uv,egui::Color32::WHITE);
            if free && response.dragged_by(egui::PointerButton::Primary){ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);}
            if let Some(info)=&self.info {response.context_menu(|ui|{display_menu(ui,&mut self.flip_x,&mut self.flip_y);audio_menu(ui,info,&mut self.selected_audio);});}
        }
        else if self.info.as_ref().is_some_and(|info|info.video.is_some()) {
            let size=egui::vec2(ui.available_width(),(ui.available_height()-32.0).max(1.0));
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
        if previous_audio!=self.selected_audio {let playing=self.is_playing();let position=self.position_now();self.stop_playback();self.position=position;if playing{self.play(ui.ctx(),path);}}
        // Keep transport controls below the preview, with a full-width timeline.
        let spare=(ui.available_height()-28.0-ui.spacing().item_spacing.y).max(0.0);ui.add_space(spare);
        let mut toggle=video_clicked;let mut stop=false;let mut seek=wave_seek.as_ref().is_some_and(|gesture|gesture.finished);
        ui.horizontal(|ui| {
            let playing=self.is_playing();
            toggle|=transport_button(ui,if playing{crate::icons::Icon::Pause}else{crate::icons::Icon::Play},crate::localization::tr(if playing{"暂停"}else{"播放"}));
            stop=transport_button(ui,crate::icons::Icon::Stop,crate::localization::tr("停止"));
            let volume_response=ui.add_sized([28.0,28.0],egui::Button::new(""));
            paint_volume_icon(ui,volume_response.rect.shrink(5.0),self.volume);
            let popup_id=ui.make_persistent_id(("media_volume",path));
            if volume_response.clicked(){if self.volume>0.0 {self.remembered_volume=self.volume;self.volume=0.0;}else{self.volume=self.remembered_volume.max(0.01);}}
            if volume_response.hovered() || volume_response.has_focus(){self.volume_hover=Some(Instant::now());ui.memory_mut(|memory|memory.open_popup(popup_id));}
            let mut popup_active=false;
            let volume_response=volume_response.on_hover_text(format!("{}: {}%",crate::localization::tr("音量"),(self.volume*100.0).round() as u32));
            egui::popup::popup_above_or_below_widget(ui,popup_id,&volume_response,egui::AboveOrBelow::Above,egui::popup::PopupCloseBehavior::CloseOnClickOutside,|ui| {
                ui.horizontal(|ui| {
                    let (rect,response)=ui.allocate_exact_size(egui::vec2(18.0,120.0),egui::Sense::click_and_drag());
                    if response.clicked() || response.dragged(){if let Some(pointer)=response.interact_pointer_pos(){self.volume=((rect.bottom()-pointer.y)/rect.height()).clamp(0.0,1.0);}}
                    if response.has_focus(){ui.input(|input| {if input.key_pressed(egui::Key::ArrowUp){self.volume=(self.volume+0.01).min(1.0);}if input.key_pressed(egui::Key::ArrowDown){self.volume=(self.volume-0.01).max(0.0);}});}
                    if response.clicked(){response.request_focus();}
                    popup_active=response.hovered() || response.dragged() || response.has_focus();
                    if self.volume>0.0 {self.remembered_volume=self.volume;}
                    let inner=rect.shrink(1.0);let mut fill=inner;fill.min.y=inner.bottom()-inner.height()*self.volume;
                    ui.painter().rect_filled(fill,0.0,ui.visuals().selection.bg_fill);
                    ui.painter().rect_stroke(rect,0.0,egui::Stroke::new(1.0_f32,ui.visuals().widgets.noninteractive.fg_stroke.color),egui::StrokeKind::Inside);
                    ui.allocate_ui(egui::vec2(42.0,120.0),|ui| {ui.with_layout(egui::Layout::left_to_right(egui::Align::Center),|ui| {ui.label(format!("{}%",(self.volume*100.0).round() as u32));});});
                });
                popup_active|=ui.rect_contains_pointer(ui.min_rect());
            });
            if popup_active {self.volume_hover=Some(Instant::now());}
            if ui.memory(|memory|memory.is_popup_open(popup_id)) {
                if self.volume_hover.is_some_and(|last|last.elapsed()>Duration::from_millis(250)) {ui.memory_mut(|memory|memory.close_popup());self.volume_hover=None;}
                else {ui.ctx().request_repaint_after(Duration::from_millis(50));}
            }
            if let Some(job)=&self.playback {job.control.volume.store(self.volume.to_bits(),Ordering::Relaxed);}
            let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width().max(30.0),28.0),egui::Sense::click_and_drag());
            ui.painter().rect_filled(rect,3.0,ui.visuals().extreme_bg_color);
            if duration>0.0 {let mut fill=rect;fill.max.x=rect.left()+rect.width()*(position/duration).clamp(0.0,1.0) as f32;ui.painter().rect_filled(fill,3.0,ui.visuals().selection.bg_fill);
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
        });
        if seek {let drag=self.seek_drag.take();let playing=drag.map_or(self.is_playing(),|seek|seek.1);if let Some(drag)=drag{position=drag.0;}self.pause();self.position=position;self.current_frame=None;if playing {
            let cached=self.frame_times.as_ref().map(|times|times.partition_point(|t|*t<=self.position).saturating_sub(1).min(times.len()-1)).filter(|target|self.cache.lock().unwrap().frames.iter().any(|(index,_)|index==target));
            if let Some(target)=cached{self.display_frame(ui.ctx(),path,target);}
            self.play(ui.ctx(),path);
        }else{self.seek_frame(ui.ctx(),path);}}
        if toggle {if self.is_playing(){self.pause();}else{if duration>0.0 && self.position>=duration{self.position=0.0;}self.play(ui.ctx(),path);}}
        if stop {self.stop_playback();self.position=0.0;self.current_frame=None;self.seek_frame(ui.ctx(),path);}
        {
            let mut preference=volume_preference().lock().unwrap();
            if preference.volume!=self.volume || preference.remembered!=self.remembered_volume {preference.volume=self.volume;preference.remembered=self.remembered_volume;preference.dirty=true;}
        }
        if !ui.input(|input|input.pointer.primary_down()){persist_volume();}
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
impl Drop for Preview {fn drop(&mut self){persist_volume();}}
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
        for track in &info.audio_tracks {if ui.selectable_label(*selected==Some(track.index),&track.label).clicked(){*selected=Some(track.index);ui.close_menu();}}
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
fn paint_volume_icon(ui:&egui::Ui,rect:egui::Rect,volume:f32) {
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
fn transport_button(ui:&mut egui::Ui,icon:crate::icons::Icon,label:&str)->bool {
    let response=ui.add_sized([28.0,28.0],egui::Button::new(""));
    crate::icons::paint(ui,icon,response.rect.shrink(5.0),ui.visuals().text_color());
    let clicked=response.clicked();response.on_hover_text(label);clicked
}
fn time(seconds:f64)->String {let ms=(seconds.max(0.0)*1000.0).round() as u64;let seconds=ms/1000;format!("{:02}:{:02}:{:02}.{:03}",seconds/3600,seconds/60%60,seconds%60,ms%1000)}

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
    let control=frame_control();let worker=control.clone();let ffmpeg=info.ffmpeg.clone();let path=path.to_owned();let size=info.video.unwrap();let mode=mode.to_owned();let ctx=ctx.clone();let (tx,receiver)=mpsc::channel();
    std::thread::spawn(move|| {
        let result=(|| {
            // Accurate input seeking decodes only the prefix from a nearby keyframe.
            // Prefetch neighbouring frames within the shared bounded cache.
            let first=index.saturating_sub(2);
            let timestamp=times.as_ref().and_then(|times|times.get(first)).copied().unwrap_or(0.0);
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
            let mut peaks=vec![vec![0.0f32;2048];channels];let mut bytes=vec![0u8;16000*channels];let mut sample=0usize;
            loop {
                if worker.stop.load(Ordering::Relaxed){return Err("Cancelled".into());}
                let mut count=0;while count<bytes.len(){let n=reader.read(&mut bytes[count..]).map_err(|e|e.to_string())?;if n==0{break;}count+=n;}
                if count==0{break;}
                for frame in bytes[..count-count%(channels*2)].chunks_exact(channels*2) {
                    let column=((sample as f64/(duration*8000.0)*2048.0) as usize).min(2047);
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
fn paint_level_meter(ui:&mut egui::Ui,wave:Option<&[Vec<f32>]>,position:f64,duration:f64) {
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
                let bar=egui::Rect::from_min_max(egui::pos2(rect.left()+rect.width()*fraction,rect.top()),egui::pos2(rect.left()+rect.width()*((segment+1) as f32/100.0),rect.bottom()));
                painter.rect_filled(bar,0.0,color);
            }
            for db in [-60.0_f32,-30.0,-12.0,-6.0,0.0] {
                let x=rect.left()+rect.width()*(db+60.0)/60.0;
                painter.line_segment([egui::pos2(x,rect.bottom()-4.0),egui::pos2(x,rect.bottom())],egui::Stroke::new(1.0_f32,ui.visuals().text_color()));
            }
            response.on_hover_text(crate::localization::tr("dBFS 为源音频近似峰值，不受播放音量影响"));
            ui.small(label);
        });
    }
}

struct WaveSeek {position:f64,started:bool,finished:bool}
fn paint_waveform(ui:&mut egui::Ui,wave:&[Vec<f32>],mut position:f64,duration:f64)->Option<WaveSeek> {
    let height=(ui.available_height()-40.0).max(40.0);
    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),height),egui::Sense::hover());
    response.on_hover_text(crate::localization::tr("dBFS 为源音频近似峰值，不受播放音量影响"));
    let painter=ui.painter_at(rect);painter.rect_filled(rect,4.0,ui.visuals().extreme_bg_color);
    let plot_left=rect.left()+62.0_f32.min(rect.width()*0.3);let plot_width=(rect.right()-plot_left).max(1.0);
    let wave_height=(height-26.0).max(1.0);
    let mut gesture=None;
    if duration>0.0 {
        let x=plot_left+(position/duration).clamp(0.0,1.0) as f32*plot_width;
        let handle_rect=egui::Rect::from_min_max(egui::pos2(x-14.0,rect.top()),egui::pos2(x+30.0,rect.top()+wave_height+5.0));
        let handle=ui.interact(handle_rect,ui.make_persistent_id("waveform_playhead"),egui::Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        // Disjoint hit regions: a press on the playhead cannot also become a
        // background seek, even when the drag leaves the handle's original area.
        for (side,left,right) in [(0,plot_left,handle_rect.left().max(plot_left)),(1,handle_rect.right().min(rect.right()),rect.right())] {
            if right<=left{continue;}
            let area=egui::Rect::from_min_max(egui::pos2(left,rect.top()),egui::pos2(right,rect.bottom()));
            let click=ui.interact(area,ui.make_persistent_id(("waveform_seek_area",side)),egui::Sense::click());
            if !handle.is_pointer_button_down_on() && !handle.dragged() && !handle.drag_stopped() && !handle.clicked() && click.clicked_by(egui::PointerButton::Primary) {
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
        for (index,peak) in peaks.iter().enumerate(){let x=plot_left+index as f32/peaks.len() as f32*plot_width;let amplitude=peak*full_height;
            painter.line_segment([egui::pos2(x,center-amplitude),egui::pos2(x,center+amplitude)],egui::Stroke::new(1.0_f32,egui::Color32::from_rgb(45,125,250)));}
        let channel_label=if wave.len()==1{"Mono"}else if channel==0{"L"}else{"R"};
        painter.text(egui::pos2(plot_left+8.0,top+3.0),egui::Align2::LEFT_TOP,channel_label,egui::FontId::proportional(12.0),ui.visuals().text_color());
    }
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
                painter.text(egui::pos2(x,ruler_top+8.0),if tick==0{egui::Align2::LEFT_TOP}else{egui::Align2::CENTER_TOP},ruler_time(seconds,step),egui::FontId::proportional(11.0),color);
            }
            for minor in 1..5 {let t=seconds+step*minor as f64/5.0;if t>=duration{break;}let x=plot_left+(t/duration) as f32*plot_width;
                painter.line_segment([egui::pos2(x,ruler_top),egui::pos2(x,ruler_top+3.0)],egui::Stroke::new(0.5_f32,color));}
        }
        if plot_width>=150.0 {painter.text(egui::pos2(rect.right(),ruler_top+8.0),egui::Align2::RIGHT_TOP,ruler_time(duration,step),egui::FontId::proportional(11.0),color);}
        let x=plot_left+(position/duration).clamp(0.0,1.0) as f32*plot_width;
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
