use std::{path::{Path, PathBuf}, sync::{Arc, mpsc}};
use eframe::egui;
use image::RgbaImage;
use crate::localization::tr;

#[derive(Clone,Copy,Debug,PartialEq)]
pub(super) struct ColorKey {pub(super) enabled:bool,pub(super) color:[u8;3],pub(super) tolerance:u8,pub(super) softness:u8,pub(super) unmix:bool}
impl Default for ColorKey {fn default()->Self{Self{enabled:false,color:[255;3],tolerance:12,softness:32,unmix:true}}}
impl ColorKey {
    pub(super) fn apply(self,image:&mut RgbaImage,mask:bool){
        if !self.enabled{return}
        for pixel in image.pixels_mut(){
            let distance=(0..3).map(|c|pixel[c].abs_diff(self.color[c])).max().unwrap_or(0) as f32;
            let t=((distance-self.tolerance as f32)/self.softness.max(1) as f32).clamp(0.,1.);
            let alpha=t*t*(3.-2.*t);
            if self.unmix&&alpha>0.&&alpha<1. {for c in 0..3 {pixel[c]=((pixel[c] as f32-self.color[c] as f32*(1.-alpha))/alpha).round().clamp(0.,255.) as u8;}}
            pixel[3]=(pixel[3] as f32*alpha).round() as u8;
            if mask {let a=pixel[3];*pixel=image::Rgba([a,a,a,255]);}
        }
    }
}

#[derive(Clone)]
struct Snapshot {
    skip_empty:bool,
    selected_cell:usize,
    compose:bool,
    color_key:ColorKey,
    key_mask:bool,
    columns:u32,
    rows:u32,
    width:u32,
    height:u32,
    auto_grid:bool,
    rectangle_first:bool,
    files_initialized:bool,
    split_align4:bool,
    use_meta:bool,
    trim_top:u32,
    trim_left:u32,
    trim_right:u32,
    trim_bottom:u32,
    crop_minimum:bool,
    crop_scale:f64,
    export_tpsheet:bool,
    padding_x:u32,
    padding_y:u32,
    show_image_bounds:bool,
    naming:Option<Naming>,
    files:Vec<PathBuf>,
    loop_playback:bool,
    playback_fps:u32,
    background:super::ImageBackground,
    background_color:Option<[u8;3]>,
}

impl PartialEq for Snapshot{fn eq(&self,other:&Self)->bool{self.skip_empty==other.skip_empty&&self.compose==other.compose&&self.color_key==other.color_key&&self.key_mask==other.key_mask&&self.columns==other.columns&&self.rows==other.rows&&self.width==other.width&&self.height==other.height&&self.auto_grid==other.auto_grid&&self.rectangle_first==other.rectangle_first&&self.files_initialized==other.files_initialized&&self.split_align4==other.split_align4&&self.use_meta==other.use_meta&&self.trim_top==other.trim_top&&self.trim_left==other.trim_left&&self.trim_right==other.trim_right&&self.trim_bottom==other.trim_bottom&&self.crop_minimum==other.crop_minimum&&self.crop_scale==other.crop_scale&&self.export_tpsheet==other.export_tpsheet&&self.padding_x==other.padding_x&&self.padding_y==other.padding_y&&self.show_image_bounds==other.show_image_bounds&&self.naming==other.naming&&self.files==other.files&&self.loop_playback==other.loop_playback&&self.playback_fps==other.playback_fps&&self.background==other.background&&self.background_color==other.background_color}}

pub struct Editor {
    skip_empty:bool,
    frame_indices:Vec<usize>,
    history:super::image_history::History<Snapshot>,
    gesture:Option<Snapshot>,
    pub open: bool,
    compose: bool,
    color_key:ColorKey,
    key_mask:bool,
    picking_key:bool,
    pick_pixels:Option<Arc<RgbaImage>>,
    animation_pending:Option<mpsc::Receiver<Result<Arc<crate::media_preview::SequenceSource>,String>>>,
    columns: u32,
    rows: u32,
    width: u32,
    height: u32,
    auto_grid: bool,
    rectangle_first:bool,
    files_initialized:bool,
    scroll_to_selected:bool,
    split_align4: bool,
    use_meta:bool,
    meta_source:usize,
    meta_rects:Vec<[u32;4]>,
    trim_top:u32,
    trim_left:u32,
    trim_right:u32,
    trim_bottom:u32,
    crop_minimum: bool,
    crop_scale:f64,
    export_tpsheet:bool,
    padding_x:u32,
    padding_y:u32,
    show_image_bounds: bool,
    image_bounds: Vec<[u32;6]>,
    naming: Option<Naming>,
    files: Vec<PathBuf>,
    pending: Option<mpsc::Receiver<Result<String,String>>>,
    detecting: Option<(usize,mpsc::Receiver<Option<(u32,u32,u32,u32)>>)>,
    message: String,
    preview_key: String,
    preview_pending: Option<(String,mpsc::Receiver<Result<(RgbaImage,(u32,u32),Vec<[u32;6]>,Option<Arc<RgbaImage>>,Vec<usize>),String>>)>,
    preview: Option<egui::TextureHandle>,
    preview_size: (u32,u32),
    preview_error: String,
    selected_cell: usize,
    grid_zoom:f32,
    grid_pan:egui::Vec2,
    playing:bool,
    loop_playback:bool,
    playback_fps:u32,
    playback_time:Option<f64>,
    background: super::ImageBackground,
    background_color: Option<[u8;3]>,
}
impl Default for Editor {
    fn default() -> Self { Self {skip_empty:false,frame_indices:Vec::new(),history:Default::default(),gesture:None,open:false,compose:false,color_key:Default::default(),key_mask:false,picking_key:false,pick_pixels:None,animation_pending:None,columns:4,rows:4,width:1,height:1,auto_grid:true,rectangle_first:true,files_initialized:false,scroll_to_selected:false,split_align4:false,use_meta:false,meta_source:0,meta_rects:Vec::new(),trim_top:0,trim_left:0,trim_right:0,trim_bottom:0,crop_minimum:false,crop_scale:1.0,export_tpsheet:false,padding_x:0,padding_y:0,show_image_bounds:true,image_bounds:Vec::new(),naming:None,files:Vec::new(),pending:None,detecting:None,message:String::new(),preview_key:String::new(),preview_pending:None,preview:None,preview_size:(0,0),preview_error:String::new(),selected_cell:0,grid_zoom:1.,grid_pan:egui::Vec2::ZERO,playing:false,loop_playback:true,playback_fps:10,playback_time:None,background:Default::default(),background_color:None} }
}
#[derive(Clone,PartialEq)]
struct Naming { prefix:String, separator:String, digits:usize, start:u32 }
impl Naming {
    fn filename(&self,index:u32)->String {
        format!("{}{}{:0width$}.png",self.prefix,self.separator,u64::from(self.start)+u64::from(index),width=self.digits)
    }
    fn validate(&self,count:u32)->Result<(),String> {
        let last=self.filename(count.saturating_sub(1));
        // Treat user text exclusively as a filename, never as a relative path.
        if last.chars().any(|c|c.is_control()||c.is_whitespace()||"<>:\"/\\|?*".contains(c)) || last.len()>240 {
            return Err(tr("文件名含空格、非法字符或过长，请修改前缀或分隔符。").into());
        }
        let base=last.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(base.as_str(),"CON"|"PRN"|"AUX"|"NUL") || (base.len()==4&&(base.starts_with("COM")||base.starts_with("LPT"))&&matches!(base.as_bytes()[3],b'1'..=b'9')) {
            return Err(tr("文件名含空格、非法字符或过长，请修改前缀或分隔符。").into());
        }
        Ok(())
    }
}
/// Exact factor pairs only; closest to square first, landscape preferred on ties.
fn grid_options(count:usize)->Vec<(u32,u32)> {
    let mut options=Vec::new();
    for columns in 1..=64u32 {
        if count>0 && count%columns as usize==0 {
            let rows=count/columns as usize;
            if rows<=64 {options.push((columns,rows as u32));}
        }
    }
    options.sort_by_key(|&(columns,rows)|(columns.abs_diff(rows),std::cmp::Reverse(columns)));
    options
}
fn natural_name_cmp(left:&Path,right:&Path)->std::cmp::Ordering {
    let a=left.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let b=right.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    static TOKENS:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
    let tokens=TOKENS.get_or_init(||regex::Regex::new(r"[0-9]+|[^0-9]+").unwrap());
    let mut a=tokens.find_iter(&a);let mut b=tokens.find_iter(&b);
    loop {match (a.next(),b.next()) {
        (Some(x),Some(y))=>{
            let (x,y)=(x.as_str(),y.as_str());
            let order=if x.as_bytes()[0].is_ascii_digit()&&y.as_bytes()[0].is_ascii_digit(){
                let (x,y)=(x.trim_start_matches('0'),y.trim_start_matches('0'));
                x.len().cmp(&y.len()).then_with(||x.cmp(y))
            }else{x.cmp(y)};
            if order!=std::cmp::Ordering::Equal{return order}
        },(None,None)=>return left.cmp(right),(None,_)=>return std::cmp::Ordering::Less,(_,None)=>return std::cmp::Ordering::Greater,
    }}
}
fn path_key(path:&Path)->String {
    std::fs::canonicalize(path).unwrap_or_else(|_|path.to_owned()).to_string_lossy().to_lowercase()
}
fn auto_add(files:&mut Vec<PathBuf>,source:&Path)->Result<usize,String> {
    let pattern=regex::Regex::new(r"^(.*?)([0-9]+)([^0-9]*)$").unwrap();
    let stem=source.file_stem().unwrap_or_default().to_string_lossy();
    let group=pattern.captures(&stem).ok_or_else(||tr("当前文件名没有数字序号，无法自动匹配同组图片。").to_owned())?;
    let prefix=group[1].to_lowercase();let suffix=group[3].to_lowercase();
    let extension=source.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let mut known:std::collections::HashSet<String>=files.iter().map(|p|path_key(p)).collect();
    let mut additions=Vec::new();
    for entry in std::fs::read_dir(source.parent().unwrap_or(Path::new("."))).map_err(|e|e.to_string())? {
        let path=entry.map_err(|e|e.to_string())?.path();
        if !path.is_file()||!super::is_bitmap(&path)||path.extension().unwrap_or_default().to_string_lossy().to_lowercase()!=extension {continue}
        let candidate=path.file_stem().unwrap_or_default().to_string_lossy();
        if let Some(parts)=pattern.captures(&candidate) {
            if parts[1].to_lowercase()==prefix&&parts[3].to_lowercase()==suffix&&known.insert(path_key(&path)){additions.push(path);}
        }
    }
    let count=additions.len();files.extend(additions);files.sort_by(|a,b|natural_name_cmp(a,b));Ok(count)
}
fn rectangle_options(count:usize)->Vec<(u32,u32)> {
    if count==0||count>4096{return Vec::new()}
    let mut options=Vec::new();
    for columns in 1..=64u32 {
        let rows=(count as u32).div_ceil(columns);
        if rows<=64 {options.push((columns,rows));options.push((rows,columns));}
    }
    options.sort_unstable();options.dedup();
    options.sort_by_key(|&(c,r)|(c.abs_diff(r),c*r-count as u32,std::cmp::Reverse(c)));
    options.truncate(8);options
}
/// Recommend uniform grids only when alpha valleys support every internal boundary.
fn detect_axis(projection:&[u64])->u32 {
    let peak=projection.iter().copied().max().unwrap_or(0);
    if peak==0 {return 1}
    let size=projection.len();
    let mut best=1;
    for count in 2..=64usize.min(size/3) {
        let peaks:Vec<u64>=(0..count).map(|i|projection[size*i/count..size*(i+1)/count].iter().copied().max().unwrap_or(0)).collect();
        // Fading animation rows can be much weaker than the first row.
        if peaks.iter().any(|&value|value<peak.div_ceil(100)) {continue}
        let radius=(size/count/8).max(1);
        let boundaries_clear=(1..count).all(|i| {
            let boundary=size*i/count;
            // Search around the grid line: glow and trimmed margins can move the valley.
            let start=boundary.saturating_sub(radius).max(1);
            let end=(boundary+radius).min(size-1);
            let valley=(start..=end).map(|p|projection[p-1].max(projection[p])).min().unwrap_or(peak);
            // Compare with adjacent bands, rather than a bright global first frame.
            valley.saturating_mul(5)<=peaks[i-1].min(peaks[i])
        });
        if boundaries_clear {best=count as u32;}
    }
    best
}
fn detect_grid(image:&RgbaImage)->Option<(u32,u32)> {
    let mut horizontal=vec![0u64;image.width() as usize];
    let mut vertical=vec![0u64;image.height() as usize];
    let mut transparent=false;let mut visible=false;
    for (x,y,pixel) in image.enumerate_pixels() {
        let alpha=pixel[3];
        transparent|=alpha<=8;
        if alpha>8 {
            visible=true;
            // Weight opaque content above low-alpha glow without discarding faint frames.
            let weight=u64::from(alpha)*u64::from(alpha);
            horizontal[x as usize]+=weight;vertical[y as usize]+=weight;
        }
    }
    if !transparent||!visible {return None}
    let columns=detect_axis(&horizontal);let rows=detect_axis(&vertical);
    if columns==1&&rows==1 {None}else{Some((columns,rows))}
}
fn detect_with_trim(image:&RgbaImage)->Option<(u32,u32,u32,u32)> {
    let (columns,rows)=detect_grid(image)?;
    let right=image.width()%columns;let bottom=image.height()%rows;
    // Only propose the remainder, never remove a whole frame or ordinary cell margins.
    let right=if (image.width()-right..image.width()).all(|x|(0..image.height()).all(|y|image.get_pixel(x,y)[3]==0)){right}else{0};
    let bottom=if (image.height()-bottom..image.height()).all(|y|(0..image.width()).all(|x|image.get_pixel(x,y)[3]==0)){bottom}else{0};
    Some((columns,rows,right,bottom))
}
fn read_meta(path:&Path,width:u32,height:u32)->Option<Vec<[u32;4]>> {
    let mut name=path.as_os_str().to_owned();name.push(".meta");
    let file=PathBuf::from(name);
    if std::fs::metadata(&file).ok()?.len()>8*1024*1024 {return None}
    parse_meta(&std::fs::read_to_string(file).ok()?,width,height)
}
fn parse_meta(text:&str,width:u32,height:u32)->Option<Vec<[u32;4]>> {
    if !text.lines().any(|line|line.trim()=="spriteMode: 2") {return None}
    let mut section=String::new();let mut in_sprites=false;
    for line in text.lines(){
        if line.trim()=="sprites:" {in_sprites=true;continue}
        if in_sprites {
            let indent=line.len()-line.trim_start().len();
            if !line.trim().is_empty()&&indent<=4&&!line.trim_start().starts_with('-'){break}
            section.push_str(line);section.push('\n');
        }
    }
    let pattern=regex::Regex::new(r"(?m)^      rect:\s*\n(?:        serializedVersion:.*\n)?        x: ([^\r\n]+)\r?\n        y: ([^\r\n]+)\r?\n        width: ([^\r\n]+)\r?\n        height: ([^\r\n]+)").ok()?;
    let mut rects=Vec::new();
    for capture in pattern.captures_iter(&section) {
        let mut values=[0u32;4];
        for i in 0..4 {
            let n=capture[i+1].trim().parse::<f64>().ok()?;
            if !n.is_finite()||n<0.||n>u32::MAX as f64||n.fract()!=0. {return None}
            values[i]=n as u32;
        }
        let [x,y,w,h]=values;
        if w==0||h==0||x.checked_add(w)?>width||y.checked_add(h)?>height||rects.len()>=4096 {return None}
        rects.push([x,height-y-h,w,h]);
    }
    let entries=section.lines().filter(|line|line.starts_with("    - ")).count();
    if rects.is_empty()||rects.len()!=entries {None}else{Some(rects)}
}
fn validate(width:u32,height:u32,columns:u32,rows:u32)->Result<(),String> {
    if columns==0 || rows==0 || columns>64 || rows>64 || width==0 || height==0 || u64::from(width)*u64::from(height)>16_777_216 {
        return Err(tr("序列尺寸无效或超过 16777216 像素，行列最多 64。").into());
    }
    Ok(())
}
// Align each cell uniformly so grid-based slicing still recovers the frames.
fn layout(w:u32,h:u32,columns:u32,rows:u32)->Result<(u32,u32,u32,u32),String> {
    let align=|size:u32|size.checked_add(3).map(|v|v/4*4);
    let invalid=||tr("序列尺寸无效或超过 16777216 像素，行列最多 64。").to_owned();
    let cw=align(w).ok_or_else(invalid)?;
    let ch=align(h).ok_or_else(invalid)?;
    let width=cw.checked_mul(columns).ok_or_else(invalid)?;
    let height=ch.checked_mul(rows).ok_or_else(invalid)?;
    validate(width,height,columns,rows)?;
    Ok((cw,ch,width,height))
}
fn decode(file:&Path)->Result<RgbaImage,String> {
    let mut reader=image::ImageReader::open(file).map_err(|e|format!("{}: {e}",file.display()))?;
    let mut limits=image::Limits::default();limits.max_alloc=Some(128*1024*1024);reader.limits(limits);
    reader.decode().map(|v|v.to_rgba8()).map_err(|e|format!("{}: {e}",file.display()))
}
fn crop_center(frame:&RgbaImage,w:u32,h:u32)->RgbaImage {
    image::imageops::crop_imm(frame,(frame.width()-w)/2,(frame.height()-h)/2,w,h).to_image()
}
fn bounds(size:u32,index:u32,count:u32)->(u32,u32) {
    let start=(u64::from(size)*u64::from(index)/u64::from(count)) as u32;
    let end=(u64::from(size)*u64::from(index+1)/u64::from(count)) as u32;
    (start,end-start)
}
fn compose_layout(w:u32,h:u32,px:u32,py:u32,columns:u32,rows:u32)->Result<(u32,u32,u32,u32),String> {
    let invalid=||tr("序列尺寸无效或超过 16777216 像素，行列最多 64。").to_owned();
    layout(w.checked_add(px).ok_or_else(invalid)?,h.checked_add(py).ok_or_else(invalid)?,columns,rows)
}
fn aligned_size(w:u32,h:u32)->Result<(u32,u32),String> {
    let (_,_,w,h)=layout(w,h,1,1)?;Ok((w,h))
}
fn pad_frame(frame:RgbaImage)->Result<RgbaImage,String> {
    let (w,h)=aligned_size(frame.width(),frame.height())?;
    if frame.dimensions()==(w,h){return Ok(frame)}
    let mut output=RgbaImage::new(w,h);
    image::imageops::replace(&mut output,&frame,i64::from((w-frame.width())/2),i64::from((h-frame.height())/2));
    Ok(output)
}
fn rect_has_pixels(image:&RgbaImage,[x,y,w,h]:[u32;4])->bool{
    (y..y+h).any(|py|(x..x+w).any(|px|image.get_pixel(px,py)[3]!=0))
}
fn filtered_rects(image:&RgbaImage,rects:&[[u32;4]],key:ColorKey,skip:bool)->Result<Vec<[u32;4]>,String>{
    if !skip{return Ok(rects.to_vec())}
    let retained:Vec<_>=rects.iter().copied().filter(|&[x,y,w,h]|{
        let mut frame=image::imageops::crop_imm(image,x,y,w,h).to_image();key.apply(&mut frame,false);frame.pixels().any(|p|p[3]!=0)
    }).collect();
    if retained.is_empty(){return Err(tr("全部帧为空，请调整去底色或关闭跳过空帧。").into())}Ok(retained)
}
fn save(image:&RgbaImage,path:&Path)->Result<(),String> {
    let mut bytes=std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes,image::ImageFormat::Png).map_err(|e|e.to_string())?;
    std::fs::write(path,bytes.into_inner()).map_err(|e|e.to_string())
}
fn compose_sheet(files:&[PathBuf],mut cell_width:u32,mut cell_height:u32,crop_minimum:bool,crop_scale:f64,px:u32,py:u32,columns:u32,rows:u32,key:ColorKey)->Result<(RgbaImage,Vec<[u32;6]>),String> {
    if files.is_empty()||files.len()>(columns*rows) as usize {return Err(tr("请添加图片，且数量不能超过格子数。").into());}
    // Decode one frame at a time, including the minimum-size scan, to bound memory.
    if crop_minimum {
        cell_width=u32::MAX;cell_height=u32::MAX;
        for file in files {
            let frame=decode(file)?;
            cell_width=cell_width.min(frame.width());cell_height=cell_height.min(frame.height());
        }
    }
    let crop_size=(cell_width,cell_height);
    if crop_minimum {
        cell_width=(cell_width as f64*crop_scale).round().clamp(1.,8192.) as u32;
        cell_height=(cell_height as f64*crop_scale).round().clamp(1.,8192.) as u32;
    }
    let (stride_x,stride_y,width,height)=compose_layout(cell_width,cell_height,px,py,columns,rows)?;
    let mut output=RgbaImage::new(width,height);
    let mut image_bounds=Vec::new();
    for (i,file) in files.iter().enumerate(){
        let mut frame=decode(file)?;key.apply(&mut frame,false);
        let original_size=frame.dimensions();
        if crop_minimum && (frame.width()<crop_size.0||frame.height()<crop_size.1) {return Err(format!("{}: {}",file.display(),tr("图片尺寸已改变，请重试。")));}
        let frame=if crop_minimum {
            let cropped=crop_center(&frame,crop_size.0,crop_size.1);
            if cropped.dimensions()==(cell_width,cell_height){cropped}else{image::imageops::resize(&cropped,cell_width,cell_height,image::imageops::FilterType::Lanczos3)}
        }else{
            image::DynamicImage::ImageRgba8(frame).resize(cell_width,cell_height,image::imageops::FilterType::Lanczos3).to_rgba8()
        };
        let x=i as u32%columns*stride_x+(stride_x-frame.width())/2;
        let y=i as u32/columns*stride_y+(stride_y-frame.height())/2;
        image_bounds.push([x,y,frame.width(),frame.height(),original_size.0,original_size.1]);
        image::imageops::replace(&mut output,&frame,i64::from(x),i64::from(y));
    }
    Ok((output,image_bounds))
}
impl Editor {
    fn snapshot(&self)->Snapshot{Snapshot{skip_empty:self.skip_empty,selected_cell:self.selected_cell,compose:self.compose.clone(),color_key:self.color_key.clone(),key_mask:self.key_mask.clone(),columns:self.columns.clone(),rows:self.rows.clone(),width:self.width.clone(),height:self.height.clone(),auto_grid:self.auto_grid.clone(),rectangle_first:self.rectangle_first.clone(),files_initialized:self.files_initialized.clone(),split_align4:self.split_align4.clone(),use_meta:self.use_meta.clone(),trim_top:self.trim_top.clone(),trim_left:self.trim_left.clone(),trim_right:self.trim_right.clone(),trim_bottom:self.trim_bottom.clone(),crop_minimum:self.crop_minimum.clone(),crop_scale:self.crop_scale.clone(),export_tpsheet:self.export_tpsheet.clone(),padding_x:self.padding_x.clone(),padding_y:self.padding_y.clone(),show_image_bounds:self.show_image_bounds.clone(),naming:self.naming.clone(),files:self.files.clone(),loop_playback:self.loop_playback.clone(),playback_fps:self.playback_fps.clone(),background:self.background.clone(),background_color:self.background_color.clone()}}
    fn restore(&mut self,state:Snapshot){self.skip_empty=state.skip_empty;self.selected_cell=state.selected_cell;self.compose=state.compose;self.color_key=state.color_key;self.key_mask=state.key_mask;self.columns=state.columns;self.rows=state.rows;self.width=state.width;self.height=state.height;self.auto_grid=state.auto_grid;self.rectangle_first=state.rectangle_first;self.files_initialized=state.files_initialized;self.split_align4=state.split_align4;self.use_meta=state.use_meta;self.trim_top=state.trim_top;self.trim_left=state.trim_left;self.trim_right=state.trim_right;self.trim_bottom=state.trim_bottom;self.crop_minimum=state.crop_minimum;self.crop_scale=state.crop_scale;self.export_tpsheet=state.export_tpsheet;self.padding_x=state.padding_x;self.padding_y=state.padding_y;self.show_image_bounds=state.show_image_bounds;self.naming=state.naming;self.files=state.files;self.loop_playback=state.loop_playback;self.playback_fps=state.playback_fps;self.background=state.background;self.background_color=state.background_color;self.picking_key=false;self.playing=false;self.playback_time=None;self.preview_key.clear();}
    pub fn active(&self)->bool {self.open||self.animation_pending.is_some()||self.pending.is_some()||self.preview_pending.is_some()||self.detecting.is_some()}
    fn arrangements(&self)->Vec<(u32,u32)> {if self.rectangle_first {rectangle_options(self.files.len())}else{grid_options(self.files.len())}}
    fn meta_active(&self)->bool {!self.compose&&self.use_meta&&!self.meta_rects.is_empty()}
    fn split_rects(&self,width:u32,height:u32)->Vec<[u32;4]> {
        if self.meta_active(){return self.meta_rects.clone()}
        (0..self.rows).flat_map(|row|(0..self.columns).map(move |column|{
            let (x,w)=bounds(width,column,self.columns);let (y,h)=bounds(height,row,self.rows);[x,y,w,h]
        })).collect()
    }
    fn preview_ui(&mut self,ui:&mut egui::Ui,original:&Arc<RgbaImage>) {
        let key=format!("{:?}",(self.compose,self.columns,self.rows,self.width,self.height,self.crop_minimum,(self.trim_top,self.trim_left,self.trim_right,self.trim_bottom),(self.meta_active(),self.color_key,self.key_mask,self.picking_key,self.skip_empty),(self.padding_x,self.padding_y,self.crop_scale.to_bits()),&self.files,Arc::as_ptr(original)));
        if let Some((pending_key,result))=self.preview_pending.as_ref().and_then(|(k,r)|match r.try_recv(){
            Ok(v)=>Some((k.clone(),v)),Err(mpsc::TryRecvError::Disconnected)=>Some((k.clone(),Err(tr("图片处理任务中断，请重试。").into()))),Err(_)=>None
        }) {
            self.preview_pending=None;
            if pending_key==key {
                self.preview_key=key.clone();self.preview=None;self.preview_error.clear();
                match result {
                    Ok((image,size,image_bounds,pick_pixels,frame_indices))=>{
                        self.frame_indices=frame_indices;
                        self.pick_pixels=pick_pixels;
                        self.image_bounds=image_bounds;
                        if self.preview_size!=size {self.grid_zoom=1.;self.grid_pan=egui::Vec2::ZERO;}
                        self.preview_size=size;
                        let pixels=image.pixels().map(|p|super::preview_color(p.0)).collect();
                        self.preview=Some(ui.ctx().load_texture("sequence_preview",egui::ColorImage{size:[image.width() as usize,image.height() as usize],pixels},egui::TextureOptions::NEAREST));
                    },Err(error)=>self.preview_error=error,
                }
            }
        }
        if self.preview_key!=key && self.preview_pending.is_none() {
            self.preview=None;self.preview_error.clear();
            let files=self.files.clone();let original=original.clone();
            let (compose,w,h,crop,columns,rows)=(self.compose,self.width,self.height,self.crop_minimum,self.columns,self.rows);
            let (px,py)=(self.padding_x,self.padding_y);let crop_scale=self.crop_scale;
            let use_meta=self.meta_active();let color_key=self.color_key;let key_mask=self.key_mask;let picking_key=self.picking_key;
            let (trim_left,trim_top,trim_right,trim_bottom)=if use_meta{(0,0,0,0)}else{(self.trim_left,self.trim_top,self.trim_right,self.trim_bottom)};
            let preview_rects=self.split_rects(original.width().saturating_sub(trim_left).saturating_sub(trim_right),original.height().saturating_sub(trim_top).saturating_sub(trim_bottom));let skip_empty=self.skip_empty;
            let (tx,rx)=mpsc::channel();self.preview_pending=Some((key.clone(),rx));let ctx=ui.ctx().clone();
            std::thread::spawn(move || {
                let result=(|| {
                    let (pixels,image_bounds)=if compose {let (pixels,bounds)=compose_sheet(&files,w,h,crop,crop_scale,px,py,columns,rows,color_key)?;(Arc::new(pixels),bounds)} else {
                        let width=original.width().saturating_sub(trim_left).saturating_sub(trim_right);let height=original.height().saturating_sub(trim_top).saturating_sub(trim_bottom);
                        if !use_meta&&(columns>width||rows>height){return Err(tr("行列数不能超过原图像素尺寸。").into());}
                        (if trim_left==0&&trim_top==0&&trim_right==0&&trim_bottom==0 {original}else{Arc::new(image::imageops::crop_imm(original.as_ref(),trim_left,trim_top,width,height).to_image())},Vec::new())
                    };
                    let pick_pixels=if picking_key{Some(if compose{Arc::new(compose_sheet(&files,w,h,crop,crop_scale,px,py,columns,rows,ColorKey::default())?.0)}else{pixels.clone()})}else{None};
                    let pixels=if !compose&&color_key.enabled {let mut pixels=pixels.as_ref().clone();color_key.apply(&mut pixels,false);Arc::new(pixels)}else{pixels};
                    let frame_indices=preview_rects.iter().enumerate().filter_map(|(i,&rect)|if compose||!skip_empty||rect_has_pixels(&pixels,rect){Some(i)}else{None}).collect();
                    let pixels=if color_key.enabled&&key_mask{let mut pixels=pixels.as_ref().clone();for p in pixels.pixels_mut(){let a=p[3];*p=image::Rgba([a,a,a,255]);}Arc::new(pixels)}else{pixels};
                    let size=pixels.dimensions();
                    let display=if size.0>2048||size.1>2048 {image::imageops::thumbnail(pixels.as_ref(),2048,2048)}else{pixels.as_ref().clone()};
                    Ok((display,size,image_bounds,pick_pixels,frame_indices))
                })();let _=tx.send(result);ctx.request_repaint();
            });
        }
        ui.horizontal(|ui| {
            ui.heading(tr("格子预览"));
            if ui.button(tr("适应窗口")).clicked(){self.grid_zoom=1.;self.grid_pan=egui::Vec2::ZERO;}
        });
        ui.small(tr("点击格子查看单帧；预览最长边 2048 像素，导出保留实际尺寸。"));
        ui.small(tr("滚轮缩放，拖动平移，点击适应窗口恢复视图。"));
        if self.preview_key!=key||self.preview_pending.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));return}
        if !self.preview_error.is_empty(){ui.label(&self.preview_error);return}
        let Some(texture)=&self.preview else{return};
        let (width,height)=self.preview_size;
        if self.meta_active(){ui.label(format!("{width} × {height} px · {}: {}",tr("按 meta 信息拆分"),self.meta_rects.len()));}else{ui.label(format!("{width} × {height} px · {} × {}",self.columns,self.rows));}
        let canvas_height=ui.available_height().max(60.);
        let (canvas,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),canvas_height),egui::Sense::click_and_drag());
        if response.dragged(){self.grid_pan+=response.drag_delta();}
        if response.hovered() {
            let scroll=ui.input_mut(|input|{let delta=input.smooth_scroll_delta.y;input.smooth_scroll_delta=egui::Vec2::ZERO;input.raw_scroll_delta=egui::Vec2::ZERO;delta});
            if scroll!=0. {
                let previous=self.grid_zoom;self.grid_zoom=(previous*(scroll*0.002).exp()).clamp(0.1,64.);
                if let Some(pointer)=response.hover_pos(){self.grid_pan=pointer-canvas.center()-(pointer-canvas.center()-self.grid_pan)*(self.grid_zoom/previous);}
            }
        }
        let scale=(canvas.width()/width as f32).min(canvas.height()/height as f32)*self.grid_zoom;
        let rect=egui::Rect::from_center_size(canvas.center()+self.grid_pan,egui::vec2(width as f32,height as f32)*scale);
        let was_picking=self.picking_key;
        let mut picked_hover=None;
        if self.picking_key {
            if let Some(pointer)=response.hover_pos().filter(|p|rect.contains(*p)){
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                let x=(((pointer.x-rect.left())/scale).floor() as u32).min(width-1);
                let y=(((pointer.y-rect.top())/scale).floor() as u32).min(height-1);
                if let Some(pixel)=self.pick_pixels.as_ref().and_then(|image|image.get_pixel_checked(x,y)){
                    picked_hover=Some((pointer,*pixel));
                    if response.clicked(){
                        if pixel[3]==0{self.message=tr("请在实际图片区域吸取底色。").into();}
                        else{self.color_key.color=[pixel[0],pixel[1],pixel[2]];self.color_key.enabled=true;self.picking_key=false;self.message=tr("底色已更新").into();}
                    }
                }
            }
        }
        // Checkerboard stays clipped to the sheet and shows alpha in both modes.
        let painter=ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));
        super::seamless::checker_background(&painter,rect.intersect(canvas),rect.min,egui::Vec2::splat(scale),&self.background,self.background_color);
        painter.image(texture.id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.,1.)),egui::Color32::WHITE);
        let rects=self.split_rects(width,height);
        let total=rects.len();
        self.selected_cell=self.selected_cell.min(total.saturating_sub(1));
        for (index,&[x,y,w,h]) in rects.iter().enumerate() {
            let cell=egui::Rect::from_min_size(rect.min+egui::vec2(x as f32,y as f32)*scale,egui::vec2(w as f32,h as f32)*scale);
            if !was_picking&&response.clicked()&&response.interact_pointer_pos().is_some_and(|position|cell.contains(position)){self.selected_cell=index;self.scroll_to_selected=true;self.playing=false;self.playback_time=None;}
            painter.rect_stroke(cell,0.,egui::Stroke::new(1.0_f32,if index==self.selected_cell {egui::Color32::YELLOW}else{egui::Color32::from_rgb(90,180,240)}),egui::StrokeKind::Inside);
        }
        if self.compose&&self.show_image_bounds {
            for &[x,y,w,h,_,_] in &self.image_bounds {
                let bounds=egui::Rect::from_min_size(rect.min+egui::vec2(x as f32,y as f32)*scale,egui::vec2(w as f32,h as f32)*scale);
                painter.rect_stroke(bounds,0.,egui::Stroke::new(1.0_f32,egui::Color32::from_rgb(255,145,60)),egui::StrokeKind::Inside);
            }
        }
        if let Some((pointer,pixel))=picked_hover {
            let overlay=ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip,ui.id().with("sequence_eyedropper")));
            let base=pointer+egui::vec2(12.,-24.);
            let tip=base+egui::vec2(0.,14.);let top=base+egui::vec2(12.,2.);
            overlay.line_segment([tip,top],egui::Stroke::new(7_f32,egui::Color32::BLACK));
            overlay.line_segment([tip,top],egui::Stroke::new(3_f32,egui::Color32::WHITE));
            overlay.line_segment([base+egui::vec2(5.,0.),base+egui::vec2(14.,9.)],egui::Stroke::new(3_f32,egui::Color32::WHITE));
            let swatch=egui::Rect::from_min_size(pointer+egui::vec2(24.,8.),egui::vec2(22.,22.));
            overlay.rect_filled(swatch,2.,egui::Color32::from_rgb(pixel[0],pixel[1],pixel[2]));
            overlay.rect_stroke(swatch,2.,egui::Stroke::new(1_f32,egui::Color32::WHITE),egui::StrokeKind::Outside);
            let text=if pixel[3]==0 {tr("请在实际图片区域吸取底色。").to_owned()}else{format!("#{:02X}{:02X}{:02X} · {}",pixel[0],pixel[1],pixel[2],tr("单击取色 · Esc 取消"))};
            let font=egui::FontId::proportional(14.);
            let galley=overlay.layout_no_wrap(text,font,egui::Color32::WHITE);
            let text_pos=swatch.right_top()+egui::vec2(7.,2.);
            overlay.rect_filled(egui::Rect::from_min_size(text_pos-egui::vec2(3.,2.),galley.size()+egui::vec2(6.,4.)),2.,egui::Color32::from_black_alpha(230));
            overlay.galley(text_pos,galley,egui::Color32::WHITE);
        }
    }
    fn frame_preview_ui(&mut self,ui:&mut egui::Ui,original:&Arc<RgbaImage>) {
        let key=format!("{:?}",(self.compose,self.columns,self.rows,self.width,self.height,self.crop_minimum,(self.trim_top,self.trim_left,self.trim_right,self.trim_bottom),(self.meta_active(),self.color_key,self.key_mask,self.picking_key,self.skip_empty),(self.padding_x,self.padding_y,self.crop_scale.to_bits()),&self.files,Arc::as_ptr(original)));
        if self.preview_key!=key||self.preview_pending.is_some(){self.playback_time=None;return}
        let Some(texture)=&self.preview else{return};
        let (width,height)=self.preview_size;
        let rects=self.split_rects(width,height);
        let map:Vec<usize>=if self.compose{(0..rects.len().min(self.files.len())).collect()}else if self.skip_empty{self.frame_indices.clone()}else{(0..rects.len()).collect()};
        let total=map.len();let count=total;
        if count==0{self.playing=false;ui.label(tr("全部帧为空，请调整去底色或关闭跳过空帧。"));return}
        let mut cursor=map.iter().position(|&i|i==self.selected_cell).unwrap_or(0);
        if !self.compose&&self.skip_empty{ui.label(format!("{}: {count} / {}",tr("有效帧"),rects.len()));}
        let now=ui.input(|input|input.time);
        ui.separator();
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y=26.;
            ui.horizontal_wrapped(|ui| {
            ui.label(tr("单帧预览"));
            if ui.add_enabled(cursor>0,egui::Button::new(tr("上一帧"))).clicked(){cursor-=1;self.scroll_to_selected=true;self.playing=false;self.playback_time=None;}
            if ui.add_enabled(cursor+1<total,egui::Button::new(tr("下一帧"))).clicked(){cursor+=1;self.scroll_to_selected=true;self.playing=false;self.playback_time=None;}
            ui.add_enabled_ui(count>1,|ui| {
                if crate::theme::compact_toggle_switch(ui,tr("播放"),&mut self.playing).changed(){
                    self.playback_time=Some(now);
                    if self.playing&&cursor+1>=count {cursor=0;self.scroll_to_selected=true;}
                }
            });
            if ui.add(egui::DragValue::new(&mut self.playback_fps).range(1..=120)).changed(){self.playback_time=Some(now);}
            ui.label(tr("帧/秒"));
            if crate::theme::compact_toggle_switch(ui,tr("循环"),&mut self.loop_playback).changed(){self.playback_time=Some(now);}
        });
        });
        if count<=1 {self.playing=false;}
        if self.playing {
            let started=self.playback_time.get_or_insert(now);
            let steps=((now-*started).max(0.)*self.playback_fps as f64).floor() as usize;
            if steps>0 {
                *started+=steps as f64/self.playback_fps as f64;
                if self.loop_playback {cursor=(cursor.min(count-1)+steps)%count;}
                else {
                    cursor=cursor.saturating_add(steps).min(count-1);
                    if cursor==count-1 {self.playing=false;}
                }
                self.scroll_to_selected=true;
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(1./self.playback_fps as f64));
        }else{self.playback_time=None;}
        let (width,height)=self.preview_size;
        self.selected_cell=map[cursor];
        let [x,y,w,h]=rects[self.selected_cell];
        ui.label(format!("{} / {} · {w} × {h} px",cursor+1,total));
        if self.compose {
            ui.horizontal_wrapped(|ui| {
                if let Some(file)=self.files.get(self.selected_cell){ui.label(file.file_name().unwrap_or_default().to_string_lossy()).on_hover_text(file.display().to_string());}
                if let Some(&[_,_,image_w,image_h,original_w,original_h])=self.image_bounds.get(self.selected_cell){
                    let scale=if self.crop_minimum {self.crop_scale}else{(self.width as f64/original_w as f64).min(self.height as f64/original_h as f64)};
                    let percent=format!("{:.1}",scale*100.);
                    ui.label(format!("· {image_w} × {image_h} px · {}: {}%",tr("缩放"),percent.trim_end_matches('0').trim_end_matches('.')));
                }
            });
        }
        let (output_w,output_h)=if !self.compose&&self.split_align4 {
            match aligned_size(w,h){Ok(size)=>{ui.label(format!("{}: {} × {} px",tr("补齐后尺寸"),size.0,size.1));size},Err(error)=>{ui.colored_label(ui.visuals().error_fg_color,error);return}}
        }else{(w,h)};
        let uv=egui::Rect::from_min_max(egui::pos2(x as f32/width as f32,y as f32/height as f32),egui::pos2((x+w) as f32/width as f32,(y+h) as f32/height as f32));
        let (canvas,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),240.),egui::Sense::hover());
        let scale=(canvas.width()/output_w.max(1) as f32).min(canvas.height()/output_h.max(1) as f32);
        let target=egui::Rect::from_center_size(canvas.center(),egui::vec2(output_w as f32,output_h as f32)*scale);
        super::seamless::checker_background(ui.painter(),target,target.min,egui::Vec2::splat(scale),&self.background,self.background_color);
        let frame=egui::Rect::from_min_size(target.min+egui::vec2(((output_w-w)/2) as f32,((output_h-h)/2) as f32)*scale,egui::vec2(w as f32,h as f32)*scale);
        // Painting the cropped UV directly avoids Image sizing using the full sheet aspect ratio.
        ui.painter().image(texture.id(),frame,uv,egui::Color32::WHITE);
    }

    pub fn show(&mut self,ctx:&egui::Context,path:&Path,original:&Arc<RgbaImage>) {
        if self.picking_key&&ctx.input(|input|input.key_pressed(egui::Key::Escape)){self.picking_key=false;}
        if !self.color_key.enabled{self.picking_key=false;}
        if let Some(result)=self.animation_pending.as_ref().and_then(|rx|match rx.try_recv(){Ok(v)=>Some(v),Err(mpsc::TryRecvError::Disconnected)=>Some(Err(tr("图片处理任务中断，请重试。").into())),Err(_)=>None}){
            self.animation_pending=None;
            match result {Ok(source)=>{crate::media_preview::open_sequence_editor(source,ctx);self.message.clear();},Err(error)=>self.message=error}
        }
        if self.animation_pending.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(100));}

        if let Some(result)=self.pending.as_ref().and_then(|r|match r.try_recv(){Ok(v)=>Some(v),Err(mpsc::TryRecvError::Disconnected)=>Some(Err(tr("图片处理任务中断，请重试。").into())),Err(_)=>None}) {
            self.pending=None;
            if let Ok(message)=&result {
                let title=tr("序列导出完成").to_owned();let description=message.clone();
                // Consume the result once, and keep the UI thread free while the native dialog is open.
                std::thread::spawn(move || {
                    rfd::MessageDialog::new().set_title(title).set_description(description)
                        .set_level(rfd::MessageLevel::Info).set_buttons(rfd::MessageButtons::Ok).show();
                });
            }
            self.message=result.unwrap_or_else(|e|e);
        }
        if let Some((source,result))=self.detecting.as_ref().and_then(|(source,rx)|match rx.try_recv(){
            Ok(result)=>Some((*source,result)),Err(mpsc::TryRecvError::Disconnected)=>Some((*source,None)),Err(_)=>None
        }) {
            self.detecting=None;
            if source==Arc::as_ptr(original) as usize {
                if let Some((columns,rows,right,bottom))=result {
                    self.columns=columns;self.rows=rows;self.trim_left=0;self.trim_top=0;self.trim_right=right;self.trim_bottom=bottom;self.selected_cell=0;
                    self.message=tr("已推荐行列，请检查预览，可继续手动调整。").into();
                } else {self.message=tr("未找到可靠的透明分隔，请手动设置行列。").into();}
            }
        }
        if !self.open {self.playing=false;self.playback_time=None;self.preview_pending=None;self.preview=None;self.preview_key.clear();self.meta_source=0;return}
        let source=Arc::as_ptr(original) as usize;
        if self.meta_source!=source {
            self.meta_source=source;self.meta_rects=read_meta(path,original.width(),original.height()).unwrap_or_default();
            self.use_meta=!self.meta_rects.is_empty();self.selected_cell=0;
        }
        if self.naming.is_none(){self.naming=Some(Naming{prefix:path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),separator:"_".into(),digits:4,start:1});}
        let mut open=self.open;let mut history_before=None;let mut history_held=false;let mut history_action=0;
        crate::native_window::Window::new(format!("{} · {}",tr("序列"),path.file_name().unwrap_or_default().to_string_lossy()))
            .id(egui::Id::new(("sequence",path))).open(&mut open).default_width(1050.).default_height(720.).min_width(640.).scrollable(false).show(ctx,|ui| {
            history_before=Some(self.snapshot());
            if self.pending.is_none()&&self.detecting.is_none()&&self.animation_pending.is_none(){history_action=super::image_history::shortcut(ui);}
            history_held=ui.input(|i|i.pointer.any_down())||ui.ctx().wants_keyboard_input();
            {
                // Native viewports own their drops; embedded windows require a pointer hit.
                let owns_drop=ui.ctx().viewport_id()!=egui::ViewportId::ROOT || ui.input(|input|
                    input.pointer.latest_pos().is_some_and(|position|ui.clip_rect().contains(position)));
                if owns_drop {
                    let dropped=ui.input_mut(|input|std::mem::take(&mut input.raw.dropped_files));
                    if !dropped.is_empty() {
                        if self.pending.is_some()||self.detecting.is_some()||self.animation_pending.is_some(){self.message=tr("处理中，请完成后再拖入图片。").into();}
                        else {
                            let mut skipped=false;
                            let mut accepted=Vec::new();
                            for file in dropped {
                                if let Some(path)=file.path.filter(|path|path.is_file()&&super::is_bitmap(path)) {accepted.push(path);}
                                else {skipped=true;}
                            }
                            if !accepted.is_empty(){self.compose=true;self.files.extend(accepted);}
                            self.message=if skipped {tr("已跳过不支持的项目，仅接受本地 PNG、JPG、JPEG、TGA、BMP、WebP 图片。").into()}else{String::new()};
                        }
                    }
                }
            }
            let mut export=false;let mut animate=false;
            ui.add_enabled_ui(self.pending.is_none()&&self.detecting.is_none()&&self.animation_pending.is_none(),|ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.history.undo.is_empty()||self.gesture.is_some(),egui::Button::new(tr("撤销"))).on_hover_text("Ctrl+Z").clicked(){history_action=-1;}
                    if ui.add_enabled(!self.history.redo.is_empty(),egui::Button::new(tr("重做"))).on_hover_text("Ctrl+Y / Ctrl+Shift+Z").clicked(){history_action=1;}
                    ui.small(format!("{} / {}",self.history.undo.len(),super::image_history::limit()));
                    let mut mode=if self.compose {"compose"}else{"split"}.to_owned();
                    crate::theme::compact_selector(ui,&mut mode,[("split","拆分序列"),("compose","制作序列")]);
                    self.compose=mode=="compose";
                    if self.compose&&!self.files_initialized {
                        self.files.insert(0,path.to_owned());self.files_initialized=true;self.selected_cell=0;
                        self.width=original.width();self.height=original.height();self.crop_scale=1.0;
                    }
                    self.background.controls(ui,&mut self.background_color);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                        let green=egui::Color32::from_rgb(30,150,85);
                        let button=crate::theme::toolbar_button(ui,tr(if self.compose {"导出合成 PNG"}else{"导出拆分 PNG"}),false)
                            .fill(green).stroke(egui::Stroke::new(1.0_f32,green));
                        export=ui.add(button).clicked();
                        animate=ui.button(tr("转 GIF / 视频")).clicked();
                        if self.compose {ui.checkbox(&mut self.export_tpsheet,tr("同时导出 tpsheet"));}
                    });
                });
                ui.separator();
                let available=ui.available_rect_before_wrap();
                let settings_width=(available.width()*0.43).clamp(270.,430.).min(available.width()*0.55);
                let right=egui::Rect::from_min_max(egui::pos2(available.right()-settings_width,available.top()),available.max);
                let left=egui::Rect::from_min_max(available.min,egui::pos2(right.left()-12.,available.bottom()));
                let mut parameters=ui.new_child(egui::UiBuilder::new().id_salt("sequence_parameters").max_rect(right));
                parameters.set_clip_rect(right.intersect(ui.clip_rect()));
                egui::ScrollArea::vertical().id_salt("sequence_parameters_scroll").auto_shrink([false,false]).show(&mut parameters,|ui| {
                    ui.separator();
                    ui.checkbox(&mut self.color_key.enabled,tr("去底色"));
                    ui.add_enabled_ui(self.color_key.enabled,|ui|{
                        ui.horizontal_wrapped(|ui|{ui.color_edit_button_srgb(&mut self.color_key.color).on_hover_text(tr("点击色块打开调色板，或点击吸取底色后到左侧格子预览取色。"));let label=if self.picking_key{"取消吸色"}else{"吸取底色"};ui.toggle_value(&mut self.picking_key,tr(label));});
                        if self.picking_key {ui.colored_label(egui::Color32::from_rgb(80,180,255),tr("吸色中：移到左侧格子预览，单击底色；Esc 或再次点击按钮取消。"));}
                        ui.horizontal_wrapped(|ui|{ui.label(tr("容差"));ui.add(egui::DragValue::new(&mut self.color_key.tolerance).range(0..=255));ui.label(tr("边缘柔化"));ui.add(egui::DragValue::new(&mut self.color_key.softness).range(0..=255));});
                        ui.checkbox(&mut self.color_key.unmix,tr("去除边缘底色混合"));
                        ui.checkbox(&mut self.key_mask,tr("容差遮罩预览"));
                        ui.small(tr("在格子预览点击吸色；黑色移除，白色保留，灰色为半透明。遮罩不导出。"));
                        ui.small(tr("去色边按纯底色反算边缘；同色主体也会被移除，复杂背景无法精确恢复。"));
                    });

                if !self.compose {
                    ui.horizontal(|ui| {
                    if !self.meta_rects.is_empty() {
                        if ui.selectable_label(self.use_meta,tr("按 meta 信息拆分")).clicked(){self.use_meta=true;self.selected_cell=0;}
                    }
                    if ui.button(tr("智能识别行列")).clicked(){
                        self.use_meta=false;
                        let original=original.clone();let source=Arc::as_ptr(&original) as usize;
                        let (tx,rx)=mpsc::channel();self.detecting=Some((source,rx));let ctx=ui.ctx().clone();
                        std::thread::spawn(move ||{let result=detect_with_trim(&original);let _=tx.send(result);ctx.request_repaint();});
                    }
                    });
                    if self.meta_active() {ui.label(format!("{} · {}",tr("按 meta 信息拆分"),self.meta_rects.len()));}
                    ui.small(tr("按透明间隔推荐均匀网格，不规则图集或相连图案请手动调整。"));
                    if !self.meta_active() {
                    ui.horizontal(|ui| {
                        ui.label(tr("列数"));ui.add(egui::DragValue::new(&mut self.columns).range(1..=64));
                        ui.label(tr("行数"));ui.add(egui::DragValue::new(&mut self.rows).range(1..=64));
                    });
                    ui.small(tr("从左到右、从上到下排列。拆分余数分配到相邻格，保留全部像素。"));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x=4.;
                        ui.label(format!("{}:",tr("忽略补边")));
                        ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=3.;ui.label(tr("上"));ui.add_sized([40.,ui.spacing().interact_size.y],egui::DragValue::new(&mut self.trim_top).range(0..=original.height().saturating_sub(self.trim_bottom).saturating_sub(1)));});
                        ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=3.;ui.label(tr("下"));ui.add_sized([40.,ui.spacing().interact_size.y],egui::DragValue::new(&mut self.trim_bottom).range(0..=original.height().saturating_sub(self.trim_top).saturating_sub(1)));});
                        ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=3.;ui.label(tr("左"));ui.add_sized([40.,ui.spacing().interact_size.y],egui::DragValue::new(&mut self.trim_left).range(0..=original.width().saturating_sub(self.trim_right).saturating_sub(1)));});
                        ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=3.;ui.label(tr("右"));ui.add_sized([40.,ui.spacing().interact_size.y],egui::DragValue::new(&mut self.trim_right).range(0..=original.width().saturating_sub(self.trim_left).saturating_sub(1)));});
                    });
                    ui.small(tr("单位为像素；智能识别仅建议忽略完全透明的末端余数，可手动修改。"));
                    let effective_width=original.width().saturating_sub(self.trim_left).saturating_sub(self.trim_right);
                    let effective_height=original.height().saturating_sub(self.trim_top).saturating_sub(self.trim_bottom);
                    if self.trim_left>0||self.trim_top>0||self.trim_right>0||self.trim_bottom>0 {ui.label(format!("{}: {effective_width} × {effective_height} px",tr("有效拆分尺寸")));}
                    let range=|size:u32,count:u32,align:bool| {
                        let mut min=size/count;let mut max=size.div_ceil(count);
                        if align {min=min.div_ceil(4)*4;max=max.div_ceil(4)*4;}
                        if min==max {min.to_string()}else{format!("{min}–{max}")}
                    };
                    ui.label(format!("{} × {} px → {}  |  {}: {} × {} px",original.width(),original.height(),self.columns*self.rows,tr("单格尺寸"),range(effective_width,self.columns,false),range(effective_height,self.rows,false)));
                    if self.split_align4 {ui.label(format!("{}: {} × {} px",tr("补齐后尺寸"),range(effective_width,self.columns,true),range(effective_height,self.rows,true)));}
                    }
                    if self.meta_active(){ui.small(tr("使用 meta 实际矩形，保留原有切片顺序，忽略网格与补边设置。"));}
                    ui.checkbox(&mut self.skip_empty,tr("跳过空帧"));
                    ui.small(tr("去底色后完全透明才算空帧；播放和导出跳过，PNG 连续编号，动画时长相应缩短。"));
                    ui.checkbox(&mut self.split_align4,tr("拆分输出尺寸补齐为 4 的倍数"));
                    ui.small(tr("原像素居中，四周补透明像素；奇数余量右侧或底部多 1 像素，不缩放、不裁剪。"));
                    ui.separator();
                    let naming_count=if self.skip_empty{self.frame_indices.len() as u32}else if self.use_meta&&!self.meta_rects.is_empty(){self.meta_rects.len() as u32}else{self.columns*self.rows};
                    let naming=self.naming.as_mut().unwrap();
                    ui.label(tr("导出命名规则"));
                    ui.horizontal(|ui|{ui.label(tr("文件名前缀"));ui.add(egui::TextEdit::singleline(&mut naming.prefix).desired_width(ui.available_width()));});
                    ui.horizontal(|ui|{ui.label(tr("分隔符（可为空）"));ui.add(egui::TextEdit::singleline(&mut naming.separator).desired_width(ui.available_width()));});
                    ui.horizontal(|ui|{
                        ui.label(tr("编号位数"));ui.add(egui::DragValue::new(&mut naming.digits).range(1..=10));
                        ui.label(tr("起始编号"));ui.add(egui::DragValue::new(&mut naming.start).range(0..=u32::MAX));
                    });
                    ui.small(tr("编号不足指定位数时补零，超出位数时保留完整编号。"));
                    ui.label(format!("{}: {} → {}",tr("文件名示例"),naming.filename(0),naming.filename(naming_count.saturating_sub(1))));
                    if let Err(error)=naming.validate(naming_count){ui.colored_label(egui::Color32::from_rgb(255,80,80),error);}

                }
                if self.compose {
                ui.horizontal(|ui| {
                ui.checkbox(&mut self.auto_grid,tr("自动排列"));
                ui.add_enabled_ui(self.auto_grid,|ui| {
                    let mut arrangement=if self.rectangle_first {"rectangle"}else{"exact"}.to_owned();
                    if crate::theme::compact_selector(ui,&mut arrangement,[("rectangle","矩形优先"),("exact","不留空格")]) {
                        self.rectangle_first=arrangement=="rectangle";
                        if let Some(&(columns,rows))=self.arrangements().first(){self.columns=columns;self.rows=rows;}
                    }
                });
                });
                    if self.auto_grid {
                        let options=self.arrangements();
                        if let Some(&(columns,rows))=options.first() {
                            if !options.contains(&(self.columns,self.rows)) {self.columns=columns;self.rows=rows;}
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr("排列组合（列 × 行）"));
                                for (columns,rows) in options {
                                    if ui.selectable_label((self.columns,self.rows)==(columns,rows),format!("{columns} × {rows}")).clicked(){self.columns=columns;self.rows=rows;}
                                }
                            });
                        } else if !self.files.is_empty() {ui.label(tr("当前数量无法在行列各不超过 64 时排满，请调整图片数量或关闭自动排列。"));}
                    }
                ui.add_enabled_ui(!self.auto_grid,|ui| {ui.horizontal(|ui| {
                    let count=self.files.len().min(4096) as u32;
                    let minimum=count.div_ceil(64).max(1);
                    ui.label(tr("列数"));
                    if ui.add(egui::DragValue::new(&mut self.columns).range(minimum..=64)).changed(){
                        self.rows=count.div_ceil(self.columns.max(1)).max(1);
                    }
                    ui.label(tr("行数"));
                    if ui.add(egui::DragValue::new(&mut self.rows).range(minimum..=64)).changed(){
                        self.columns=count.div_ceil(self.rows.max(1)).max(1);
                    }
                });
                });
                ui.small(tr("从左到右、从上到下排列。拆分余数分配到相邻格，保留全部像素。"));
                    ui.checkbox(&mut self.crop_minimum,tr("裁剪到共同最小尺寸"));
                    let base=if self.crop_minimum {
                        self.image_bounds.iter().map(|r|(r[4],r[5])).reduce(|a,b|(a.0.min(b.0),a.1.min(b.1)))
                    }else{self.image_bounds.first().map(|r|(r[4],r[5]))};
                    if self.crop_minimum {if let Some((w,h))=base {
                        self.width=(w as f64*self.crop_scale).round().clamp(1.,8192.) as u32;
                        self.height=(h as f64*self.crop_scale).round().clamp(1.,8192.) as u32;
                    }}
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr("单图宽度"));let width_changed=ui.add(egui::DragValue::new(&mut self.width).range(1..=8192)).changed();
                        ui.label(tr("单图高度"));let height_changed=ui.add(egui::DragValue::new(&mut self.height).range(1..=8192)).changed();
                        if let Some((w,h))=base {
                            let max_scale=(8192./w as f64).min(8192./h as f64);
                            let mut scale=if self.crop_minimum {self.crop_scale}else{(self.width as f64/w as f64).min(self.height as f64/h as f64)};
                            if width_changed {scale=self.width as f64/w as f64;}
                            if height_changed {scale=self.height as f64/h as f64;}
                            let mut percent=scale*100.;
                            ui.label(tr("比例"));let ratio_changed=ui.add(egui::DragValue::new(&mut percent).suffix("%").speed(0.5).range(0.01..=max_scale*100.)).changed();
                            if width_changed||height_changed||ratio_changed {
                                scale=(percent/100.).clamp(0.0001,max_scale);
                                self.width=(w as f64*scale).round().clamp(1.,8192.) as u32;
                                self.height=(h as f64*scale).round().clamp(1.,8192.) as u32;
                                self.crop_scale=scale;
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr("水平 Padding"));ui.add(egui::DragValue::new(&mut self.padding_x).range(0..=8192));
                        ui.label(tr("垂直 Padding"));ui.add(egui::DragValue::new(&mut self.padding_y).range(0..=8192));
                    });
                    ui.small(tr("图片居中于含 Padding 和 4 对齐补白的最终格子；奇数余量右侧或底部多 1 像素。"));
                    ui.checkbox(&mut self.show_image_bounds,tr("显示单图边框"));
                    ui.small(tr("共同最小尺寸先居中裁剪，再按比例缩放；宽高和比例联动，比例以裁剪尺寸为基准。"));
                    ui.small(tr("单图居中放入格子，橙框显示实际图片尺寸；分格线始终显示，单格自动补齐至 4 的倍数。"));
                    ui.small(tr("可拖入多张本地图片到此窗口，按拖入顺序追加到列表。"));
                    ui.horizontal(|ui| {
                        if ui.button(tr("添加图片")).clicked() {
                            if let Some(files)=rfd::FileDialog::new().add_filter(tr("图片"),&["png","jpg","jpeg","tga","bmp","webp"]).pick_files(){self.files.extend(files);}
                        }
                        if ui.button(tr("自动加入")).on_hover_text(tr("按当前打开图片的数字序号规则查找同目录同扩展名图片，去重后按自然升序排列。")).clicked() {
                            let selected=self.files.get(self.selected_cell).map(|p|path_key(p));
                            self.message=match auto_add(&mut self.files,path) {
                                Ok(count)=>format!("{}: {count}",tr("已自动加入")),Err(error)=>error,
                            };
                            if let Some(selected)=selected {if let Some(index)=self.files.iter().position(|p|path_key(p)==selected){self.selected_cell=index;self.scroll_to_selected=true;self.playing=false;self.playback_time=None;}}
                        }
                    });
                    let mut action=None;
                    egui::ScrollArea::vertical().id_salt("sequence_files").max_height(240.).show(ui,|ui| {
                        for (i,file) in self.files.iter().enumerate(){ui.horizontal(|ui| {
                            let response=ui.selectable_label(self.selected_cell==i,format!("{} · {}",i+1,file.file_name().unwrap_or_default().to_string_lossy())).on_hover_text(file.display().to_string());
                            if response.clicked(){self.selected_cell=i;self.playing=false;self.playback_time=None;}
                            if self.scroll_to_selected&&self.selected_cell==i {response.scroll_to_me(Some(egui::Align::Center));}
                            if ui.add_enabled(i>0,egui::Button::new("↑")).clicked(){action=Some((i,0));}
                            if ui.add_enabled(i+1<self.files.len(),egui::Button::new("↓")).clicked(){action=Some((i,1));}
                            if ui.button(tr("移除")).clicked(){action=Some((i,2));}
                        });}
                    });
                    self.scroll_to_selected=false;
                    if let Some((i,a))=action {
                        let target=if a==0 {i-1}else{(i+1).min(self.files.len()-1)};
                        if a<2 {
                            self.files.swap(i,target);
                            if self.selected_cell==i {self.selected_cell=target;}else if self.selected_cell==target {self.selected_cell=i;}
                        }else{
                            self.files.remove(i);
                            if self.selected_cell>i {self.selected_cell-=1;}
                            self.selected_cell=self.selected_cell.min(self.files.len().saturating_sub(1));
                        }
                    }
                    if self.auto_grid {
                        if let Some(&(columns,rows))=self.arrangements().first().filter(|_|!self.arrangements().contains(&(self.columns,self.rows))) {self.columns=columns;self.rows=rows;}
                    }
                    ui.label(format!("{} / {}",self.files.len(),self.columns*self.rows));
                }
                if self.pending.is_some()||self.detecting.is_some(){ui.spinner();ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));}
                if !self.message.is_empty(){ui.label(&self.message);}
                self.frame_preview_ui(ui,original);
                });
                let mut preview_ui=ui.new_child(egui::UiBuilder::new().id_salt("sequence_preview").max_rect(left));
                preview_ui.set_clip_rect(left.intersect(ui.clip_rect()));
                self.preview_ui(&mut preview_ui,original);
                ui.allocate_rect(available,egui::Sense::hover());
                if export||animate {
                    let (columns,rows)=(self.columns,self.rows);
                    let width=if self.compose && !self.crop_minimum {self.width}else{original.width().saturating_sub(if self.compose||self.meta_active() {0}else{self.trim_left.saturating_add(self.trim_right)})};
                    let height=if self.compose && !self.crop_minimum {self.height}else{original.height().saturating_sub(if self.compose||self.meta_active() {0}else{self.trim_top.saturating_add(self.trim_bottom)})};
                    let valid=(if self.meta_active(){Ok(())} else if self.compose && self.auto_grid && self.arrangements().is_empty() {Err(tr("当前数量无法在行列各不超过 64 时排满，请调整图片数量或关闭自动排列。").into())} else if self.compose { if self.crop_minimum {Ok(())} else {compose_layout(width,height,self.padding_x,self.padding_y,columns,rows).map(|_|())} } else {validate(width,height,columns,rows)}).and_then(|_| {
                        if self.compose && (self.files.is_empty()||self.files.len()> (columns*rows) as usize) {Err(tr("请添加图片，且数量不能超过格子数。").into())}
                        else if !self.compose && !self.meta_active() && (columns>width||rows>height) {Err(tr("行列数不能超过原图像素尺寸。").into())}else{Ok(())}
                    });
                    if let Err(error)=valid {self.message=error;return}
                    let mut split_rects=self.split_rects(width,height);
                    if !self.compose&&!self.meta_active(){for rect in &mut split_rects {rect[0]+=self.trim_left;rect[1]+=self.trim_top;}}
                    if animate {
                        let original=original.clone();let files=self.files.clone();let compose=self.compose;
                        let (w,h,crop,scale,px,py,columns,rows)=(self.width,self.height,self.crop_minimum,self.crop_scale,self.padding_x,self.padding_y,self.columns,self.rows);
                        let key=self.color_key;let skip_empty=self.skip_empty;let align=self.split_align4;let fps=self.playback_fps.max(1);
                        let export_dir=path.parent().unwrap_or(Path::new(".")).to_owned();
                        let (tx,rx)=mpsc::channel();self.animation_pending=Some(rx);self.message=tr("正在准备序列视频…").into();let ctx=ctx.clone();
                        std::thread::spawn(move||{
                            let result=(||{
                                let source=crate::media_preview::SequenceSource::new(export_dir)?;
                                if compose {
                                    let (sheet,_)=compose_sheet(&files,w,h,crop,scale,px,py,columns,rows,key)?;
                                    let (cw,ch)=(sheet.width()/columns,sheet.height()/rows);
                                    for i in 0..files.len(){let frame=image::imageops::crop_imm(&sheet,i as u32%columns*cw,i as u32/columns*ch,cw,ch).to_image();save(&frame,&source.frame_path(i))?;}
                                }else{
                                    let split_rects=filtered_rects(original.as_ref(),&split_rects,key,skip_empty)?;
                                    let max_w=split_rects.iter().map(|r|r[2]).max().unwrap_or(0);let max_h=split_rects.iter().map(|r|r[3]).max().unwrap_or(0);
                                    let (max_w,max_h)=if align{aligned_size(max_w,max_h)?}else{(max_w,max_h)};
                                    for (i,&[x,y,w,h]) in split_rects.iter().enumerate(){
                                        let mut frame=image::imageops::crop_imm(original.as_ref(),x,y,w,h).to_image();key.apply(&mut frame,false);
                                        let mut canvas=RgbaImage::new(max_w,max_h);image::imageops::replace(&mut canvas,&frame,i64::from((max_w-w)/2),i64::from((max_h-h)/2));save(&canvas,&source.frame_path(i))?;
                                    }
                                }
                                source.finish(fps)
                            })();let _=tx.send(result);ctx.request_repaint();
                        });
                        return;
                    }
                    if !self.compose {if let Err(error)=self.naming.as_ref().unwrap().validate(if self.skip_empty{self.frame_indices.len().max(1) as u32}else{split_rects.len() as u32}){self.message=error;return}}
                    let destination=if self.compose {rfd::FileDialog::new().set_file_name("sequence.png").add_filter("PNG",&["png"]).save_file()}
                        else{rfd::FileDialog::new().pick_folder()};
                    if let Some(destination)=destination {
                        let export_tpsheet=self.compose&&self.export_tpsheet;
                        let sheet_path=destination.with_extension("tpsheet");
                        if export_tpsheet&&sheet_path.exists()&&rfd::MessageDialog::new()
                            .set_title(tr("同时导出 tpsheet")).set_description(format!("{}
{}",tr("已有同名 tpsheet，是否覆盖？"),sheet_path.display()))
                            .set_buttons(rfd::MessageButtons::YesNo).show()!=rfd::MessageDialogResult::Yes {return}

                        let compose=self.compose;let files=self.files.clone();let original=original.clone();
                        let (cell_width,cell_height)=(self.width,self.height);
                        let (px,py)=(self.padding_x,self.padding_y);let crop_scale=self.crop_scale;
                        let crop_minimum=self.crop_minimum;let split_align4=self.split_align4;let color_key=self.color_key;let skip_empty=self.skip_empty;
                        let naming=self.naming.as_ref().unwrap().clone();
                        let stem=path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                        let (tx,rx)=mpsc::channel();self.pending=Some(rx);self.message.clear();let ctx=ctx.clone();
                        std::thread::spawn(move || {
                            let result=(||->Result<String,String>{
                                if compose {
                                    let (output,bounds)=compose_sheet(&files,cell_width,cell_height,crop_minimum,crop_scale,px,py,columns,rows,color_key)?;
                                    let sheet=if export_tpsheet {Some(super::sequence_tpsheet::encode(&destination,output.dimensions(),&files,&bounds)?)}else{None};
                                    save(&output,&destination)?;
                                    if let Some(sheet)=sheet {
                                        std::fs::write(&sheet_path,sheet).map_err(|e|format!("{}: {} — {e}",tr("PNG 已保存，但 tpsheet 保存失败"),sheet_path.display()))?;
                                        return Ok(format!("{}:
{}
{}",tr("已保存"),destination.display(),sheet_path.display()));
                                    }

                                    Ok(format!("{}: {}",tr("已保存"),destination.display()))
                                } else {
                                    let split_rects=filtered_rects(original.as_ref(),&split_rects,color_key,skip_empty)?;naming.validate(split_rects.len() as u32)?;
                                    // A new directory prevents collisions and never overwrites existing frames.
                                    let mut output=None;
                                    for suffix in 1..=10000 {
                                        let folder=destination.join(format!("{stem}_sequence_{suffix}"));
                                        match std::fs::create_dir(&folder){Ok(())=>{output=Some(folder);break},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(e)=>return Err(e.to_string())}
                                    }
                                    let output=output.ok_or_else(||tr("无法创建序列输出目录。").to_owned())?;
                                    for (index,&[x,y,w,h]) in split_rects.iter().enumerate() {
                                        let mut frame=image::imageops::crop_imm(original.as_ref(),x,y,w,h).to_image();color_key.apply(&mut frame,false);
                                        let frame=if split_align4 {pad_frame(frame)?}else{frame};
                                        save(&frame,&output.join(naming.filename(index as u32)))
                                            .map_err(|e|format!("{}: {e}",output.display()))?;
                                    }
                                    Ok(format!("{}: {}",tr("已保存"),output.display()))
                                }
                            })();let _=tx.send(result);ctx.request_repaint();
                        });
                    }
                }
            });

        });
        self.open=open;
        if let Some(before)=history_before{
            let after=self.snapshot();
            if history_action!=0{
                if let Some(start)=self.gesture.take(){self.history.record(start,&after);}
                let restored=if history_action<0{self.history.undo(&after)}else{self.history.redo(&after)};
                if let Some(state)=restored{self.restore(state);}
            }else{
                if before!=after||self.detecting.is_some(){self.gesture.get_or_insert(before);}
                if !history_held&&self.detecting.is_none(){if let Some(start)=self.gesture.take(){self.history.record(start,&after);}}
            }
            self.history.trim();
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_frame_filter_uses_final_alpha_and_preserves_order(){
        let mut sheet=RgbaImage::new(6,2);sheet.put_pixel(0,0,image::Rgba([200,0,0,0]));sheet.put_pixel(2,0,image::Rgba([10,20,30,1]));sheet.put_pixel(4,0,image::Rgba([255,255,255,255]));
        let rects=vec![[0,0,2,2],[2,0,2,2],[4,0,2,2]];
        assert!(!rect_has_pixels(&sheet,rects[0]));
        assert_eq!(filtered_rects(&sheet,&rects,ColorKey::default(),true).unwrap(),rects[1..]);
        assert_eq!(filtered_rects(&sheet,&rects,ColorKey::default(),false).unwrap(),rects);
        let key=ColorKey{enabled:true,color:[255;3],tolerance:0,softness:0,unmix:true};
        assert_eq!(filtered_rects(&sheet,&rects,key,true).unwrap(),vec![rects[1]]);
        assert!(filtered_rects(&sheet,&[rects[0]],key,true).is_err());
    }

    #[test]
    fn editor_history_restores_drop_order_mode_and_parameters(){
        let mut editor=Editor::default();let initial=editor.snapshot();editor.compose=true;editor.files=vec![PathBuf::from("first.png"),PathBuf::from("unrelated.png")];editor.files_initialized=true;editor.width=20;editor.trim_left=3;
        let added=editor.snapshot();editor.history.record(initial.clone(),&added);
        let previous=editor.history.undo(&added).unwrap();editor.restore(previous);assert!(editor.files.is_empty());assert!(!editor.compose);assert_eq!(editor.trim_left,0);
        let next=editor.history.redo(&editor.snapshot()).unwrap();editor.restore(next);assert_eq!(editor.files[1],PathBuf::from("unrelated.png"));assert!(editor.compose);assert_eq!(editor.width,20);assert_eq!(editor.trim_left,3);
    }

    #[test]
    fn color_key_preserves_existing_alpha_and_unmixes_soft_edges(){
        let key=ColorKey{enabled:true,color:[0;3],tolerance:0,softness:128,unmix:true};
        let mut pixels=RgbaImage::new(3,1);
        pixels.put_pixel(0,0,image::Rgba([0,0,0,255]));
        pixels.put_pixel(1,0,image::Rgba([64,0,0,128]));
        pixels.put_pixel(2,0,image::Rgba([255,0,0,255]));
        key.apply(&mut pixels,false);
        assert_eq!(pixels.get_pixel(0,0)[3],0);
        assert_eq!(pixels.get_pixel(1,0).0,[128,0,0,64]);
        assert_eq!(pixels.get_pixel(2,0).0,[255,0,0,255]);
        let mut mask=RgbaImage::from_pixel(1,1,image::Rgba([64,0,0,255]));key.apply(&mut mask,true);
        assert_eq!(mask.get_pixel(0,0).0,[128,128,128,255]);
    }
    #[test]
    fn zero_softness_is_a_hard_key_and_disabled_key_is_unchanged(){
        let mut pixel=RgbaImage::from_pixel(1,1,image::Rgba([255,255,254,128]));
        let original=pixel.clone();ColorKey::default().apply(&mut pixel,false);assert_eq!(pixel,original);
        ColorKey{enabled:true,color:[255;3],tolerance:1,softness:0,unmix:true}.apply(&mut pixel,false);
        assert_eq!(pixel.get_pixel(0,0)[3],0);
    }

    #[test]
    fn sequence_names_sort_by_numeric_value(){
        let mut names=vec![PathBuf::from("frame10.png"),PathBuf::from("frame2.png"),PathBuf::from("frame001.png")];
        names.sort_by(|a,b|natural_name_cmp(a,b));
        assert_eq!(names,vec![PathBuf::from("frame001.png"),PathBuf::from("frame2.png"),PathBuf::from("frame10.png")]);
    }
    #[test]
    fn composition_centers_image_and_tpsheet_uses_actual_position(){
        let directory=std::env::temp_dir().join(format!("sequence-centering-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&directory).unwrap();
        let file=directory.join("frame.png");
        let source=RgbaImage::from_pixel(98,98,image::Rgba([255,80,10,255]));
        save(&source,&file).unwrap();
        let (output,bounds)=compose_sheet(&[file.clone()],98,98,false,1.,0,0,1,1,ColorKey::default()).unwrap();
        assert_eq!(output.dimensions(),(100,100));
        assert_eq!(bounds[0],[1,1,98,98,98,98]);
        assert_eq!(output.get_pixel(0,0)[3],0);
        assert_eq!(output.get_pixel(1,1),source.get_pixel(0,0));
        assert_eq!(output.get_pixel(98,98),source.get_pixel(97,97));
        assert_eq!(output.get_pixel(99,99)[3],0);
        let sheet=super::super::sequence_tpsheet::encode(Path::new("sheet.png"),output.dimensions(),&[file.clone()],&bounds).unwrap();
        assert!(sheet.contains("frame;1;1;98;98;0.5;0.5"));
        save(&RgbaImage::from_pixel(97,97,image::Rgba([255,80,10,255])),&file).unwrap();
        let (odd,rects)=compose_sheet(&[file.clone()],97,97,true,1.,0,0,1,1,ColorKey::default()).unwrap();
        assert_eq!(odd.dimensions(),(100,100));assert_eq!(rects[0],[1,1,97,97,97,97]);
        assert_eq!(odd.get_pixel(97,97)[3],255);
        assert_eq!(odd.get_pixel(98,98)[3],0);assert_eq!(odd.get_pixel(99,99)[3],0);
        std::fs::remove_file(file).unwrap();std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn composition_keys_each_source_without_changing_originals(){
        let directory=std::env::temp_dir().join(format!("unity-compose-key-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&directory).unwrap();let file=directory.join("frame.png");
        let mut source=RgbaImage::from_pixel(4,4,image::Rgba([255,255,255,255]));source.put_pixel(2,2,image::Rgba([255,0,0,128]));save(&source,&file).unwrap();
        let key=ColorKey{enabled:true,color:[255;3],tolerance:0,softness:0,unmix:true};
        for crop in [false,true]{
            let (sheet,_)=compose_sheet(&[file.clone(),file.clone()],4,4,crop,1.,0,0,2,1,key).unwrap();
            assert_eq!(sheet.get_pixel(0,0)[3],0);assert_eq!(sheet.get_pixel(4,0)[3],0);
            assert_eq!(sheet.get_pixel(2,2).0,[255,0,0,128]);assert_eq!(sheet.get_pixel(6,2).0,[255,0,0,128]);
        }
        assert_eq!(decode(&file).unwrap(),source);
        std::fs::remove_file(file).unwrap();std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn composition_padding_is_included_before_cell_alignment(){
        assert_eq!(compose_layout(51,51,0,0,5,3).unwrap(),(52,52,260,156));
        assert_eq!(compose_layout(51,51,2,3,5,3).unwrap(),(56,56,280,168));
        assert!(compose_layout(u32::MAX,51,1,0,5,3).is_err());
    }
    #[test]
    fn rectangular_arrangement_keeps_prime_sequences_compact(){
        assert_eq!(rectangle_options(29)[0],(6,5));
        assert_eq!(grid_options(29)[0],(29,1));
        assert_eq!(rectangle_options(10)[0],(4,3));
        assert!(rectangle_options(0).is_empty());
        for count in 1..=4096 {for (c,r) in rectangle_options(count){assert!(c*r>=count as u32&&c<=64&&r<=64);}}
    }
    #[test]
    fn multiple_meta_uses_exact_rectangles_and_converts_origin(){
        let text="TextureImporter:\n  spriteMode: 2\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: apple_0\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 104\n        width: 51\n        height: 51\n    - serializedVersion: 2\n      name: apple_1\n      rect:\n        serializedVersion: 2\n        x: 51\n        y: 52\n        width: 51\n        height: 51\n    outline: []\n";
        assert_eq!(parse_meta(&text,256,156),Some(vec![[0,1,51,51],[51,53,51,51]]));
        assert!(parse_meta(&text.replace("width: 51","width: 999"),256,156).is_none());
        assert!(parse_meta(&text.replace("spriteMode: 2","spriteMode: 1"),256,156).is_none());
        assert!(parse_meta(&text.replace("height: 51","height: 0"),256,156).is_none());
    }
    #[test]
    fn detection_suggests_only_transparent_remainder(){
        let mut sheet=RgbaImage::new(256,157);
        for row in 0..3 {for column in 0..5 {
            for y in row*52+4..row*52+48 {for x in column*51+4..column*51+47 {sheet.put_pixel(x,y,image::Rgba([255,200,0,255]));}}
        }}
        assert_eq!(detect_with_trim(&sheet),Some((5,3,1,1)));
        sheet.put_pixel(255,20,image::Rgba([255,0,0,1]));
        assert_eq!(detect_with_trim(&sheet),Some((5,3,0,1)));
        for i in 0..5 {assert_eq!(bounds(255,i,5),(i*51,51));}
    }
    #[test]
    fn split_padding_centers_pixels_without_resampling(){
        let frame=RgbaImage::from_fn(5,7,|x,y|image::Rgba([x as u8,y as u8,123,255]));
        let padded=pad_frame(frame.clone()).unwrap();
        assert_eq!(padded.dimensions(),(8,8));
        for y in 0..8 {for x in 0..8 {
            if (1..6).contains(&x)&&y<7 {assert_eq!(padded.get_pixel(x,y),frame.get_pixel(x-1,y));}
            else {assert_eq!(padded.get_pixel(x,y),&image::Rgba([0,0,0,0]));}
        }}
        let aligned=RgbaImage::from_pixel(4,8,image::Rgba([1,2,3,4]));
        assert_eq!(pad_frame(aligned.clone()).unwrap(),aligned);
    }
    #[test]
    fn naming_supports_empty_separator_and_custom_start(){
        let mut naming=Naming{prefix:"apple".into(),separator:"_".into(),digits:4,start:1};
        assert_eq!(naming.filename(0),"apple_0001.png");
        naming.separator.clear();naming.start=0;naming.digits=2;
        assert_eq!(naming.filename(0),"apple00.png");
        assert_eq!(naming.filename(100),"apple100.png");
        assert!(naming.validate(16).is_ok());
        naming.prefix="../escape".into();assert!(naming.validate(16).is_err());
        naming.prefix="CON.".into();assert!(naming.validate(16).is_err());
        naming.prefix="frame".into();naming.start=u32::MAX;
        assert_eq!(naming.filename(1),"frame4294967296.png");
    }
    #[test]
    fn detection_handles_glow_offset_and_fading_rows(){
        let mut sheet=RgbaImage::new(100,60);
        for row in 0..3 {for column in 0..5 {
            if row==2&&column==4 {continue}
            let alpha=if row==2 {35}else{255};
            // A faint halo crosses the exact grid boundary.
            for y in row*20..(row*20+20).min(60) {for x in column*20+2..column*20+18 {
                sheet.put_pixel(x,y,image::Rgba([255,200,0,12]));
            }}
            // Offset transparent valley; the last row is short and faint.
            let top=if row==2 {row*20+14}else{row*20+3};
            for y in top..row*20+19 {for x in column*20+3..column*20+17 {
                sheet.put_pixel(x,y,image::Rgba([255,200,0,alpha]));
            }}
        }}
        assert_eq!(detect_grid(&sheet),Some((5,3)));
    }
    #[test]
    fn detection_finds_uniform_sheet_with_empty_last_frame(){
        let mut sheet=RgbaImage::new(100,60);
        for row in 0..3 {for column in 0..5 {
            if row==2&&column==4 {continue}
            for y in row*20+3..row*20+17 {for x in column*20+3..column*20+17 {sheet.put_pixel(x,y,image::Rgba([255,200,0,255]));}}
        }}
        assert_eq!(detect_grid(&sheet),Some((5,3)));
        assert_eq!(detect_grid(&RgbaImage::new(30,30)),None);
        assert_eq!(detect_grid(&RgbaImage::from_pixel(30,30,image::Rgba([255,0,0,255]))),None);
        let single=image::imageops::crop_imm(&sheet,0,0,20,20).to_image();
        assert_eq!(detect_grid(&single),None);
    }
    #[test]
    fn automatic_grids_are_exact_and_prefer_landscape(){
        assert_eq!(grid_options(10),vec![(5,2),(2,5),(10,1),(1,10)]);
        assert_eq!(grid_options(12)[0],(4,3));
        assert_eq!(grid_options(7),vec![(7,1),(1,7)]);
        assert!(grid_options(0).is_empty());assert!(grid_options(67).is_empty());
        assert_eq!(grid_options(4096),vec![(64,64)]);
        for n in 1..=4096 {for (c,r) in grid_options(n) {assert_eq!(c as usize*r as usize,n);assert!(c<=64&&r<=64);}}
    }
    #[test]
    fn uneven_grid_covers_each_pixel_once(){
        for size in 1..100 {for count in 1..=size.min(64) {
            let mut end=0;for i in 0..count {let (start,len)=bounds(size,i,count);assert_eq!(start,end);assert!(len>0);end=start+len;}assert_eq!(end,size);
        }}
    }
    #[test]
    fn cell_alignment_and_crop_preserve_pixels(){
        assert_eq!(layout(5,7,3,2).unwrap(),(8,8,24,16));

        assert!(layout(u32::MAX,7,3,2).is_err());
        let frame=RgbaImage::from_fn(9,7,|x,y|image::Rgba([x as u8,y as u8,123,255]));
        let cropped=crop_center(&frame,5,3);
        assert_eq!(cropped.dimensions(),(5,3));
        assert_eq!(cropped.get_pixel(0,0),frame.get_pixel(2,2));
        assert_eq!(cropped.get_pixel(4,2),frame.get_pixel(6,4));
    }
    #[test]
    fn rejects_oversized_output_and_invalid_grids(){
        assert!(validate(4096,4096,64,64).is_ok());
        for values in [(4097,4096,4,4),(1,1,0,1),(1,1,1,65),(0,1,1,1)] {assert!(validate(values.0,values.1,values.2,values.3).is_err());}
    }
}
