//! GIF container metadata and preview-only loop policy; pixels remain FFmpeg's responsibility.
use std::{io::{self, Read}, path::Path};
use eframe::egui;
use crate::localization::tr;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata { pub loops: Option<u16>, pub frames: usize, pub palette_colors: usize }
fn byte(reader: &mut impl Read) -> io::Result<u8> { let mut value=[0];reader.read_exact(&mut value)?;Ok(value[0]) }
fn skip(reader: &mut impl Read, mut count: usize) -> io::Result<()> {
    let mut buffer=[0;256];
    while count>0 {let chunk=count.min(buffer.len());reader.read_exact(&mut buffer[..chunk])?;count-=chunk;}Ok(())
}
fn blocks(reader: &mut impl Read) -> io::Result<()> {loop {let count=byte(reader)? as usize;if count==0{return Ok(())}skip(reader,count)?;}}
fn parse(reader: &mut impl Read) -> io::Result<Metadata> {
    let mut header=[0;13];reader.read_exact(&mut header)?;
    if &header[..6]!=b"GIF87a" && &header[..6]!=b"GIF89a" {return Err(io::Error::new(io::ErrorKind::InvalidData,"Invalid GIF signature"));}
    let global_colors=if header[10]&128!=0 {1usize<<((header[10]&7)+1)}else{0};
    skip(reader,3*global_colors)?;
    let mut result=Metadata{loops:None,frames:0,palette_colors:global_colors};
    loop {match byte(reader)? {
        0x3b=>return Ok(result),
        0x2c=>{let mut descriptor=[0;9];reader.read_exact(&mut descriptor)?;if descriptor[8]&128!=0 {let colors=1usize<<((descriptor[8]&7)+1);result.palette_colors=result.palette_colors.max(colors);skip(reader,3*colors)?;}byte(reader)?;blocks(reader)?;result.frames+=1;},
        0x21=>{let label=byte(reader)?;if label==0xff {
            let count=byte(reader)? as usize;let mut app=[0;255];reader.read_exact(&mut app[..count])?;
            let looping=count==11 && (&app[..11]==b"NETSCAPE2.0" || &app[..11]==b"ANIMEXTS1.0");
            if count>0 {loop {let count=byte(reader)? as usize;if count==0{break;}let mut data=[0;255];reader.read_exact(&mut data[..count])?;if looping&&count==3&&data[0]==1 {result.loops=Some(u16::from_le_bytes([data[1],data[2]]));}}}
        }else{blocks(reader)?;}},
        _=>return Err(io::Error::new(io::ErrorKind::InvalidData,"Invalid GIF block")),
    }}
}
pub fn read(path: &Path) -> Result<Metadata,String> {
    let file=std::fs::File::open(path).map_err(|error|error.to_string())?;
    // Bounded scan without retaining compressed frames.
    parse(&mut io::BufReader::new(file).take(256*1024*1024)).map_err(|error|error.to_string())
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode { #[default] File, Once, Forever, Count }
#[derive(Clone)]
pub struct State {pub mode:Mode,pub count:u32,pub completed:u32}
impl Default for State {fn default()->Self{Self{mode:Mode::File,count:1,completed:0}}}
impl State {
    pub fn finish(&mut self, metadata:Metadata)->bool {
        self.completed=self.completed.saturating_add(1);
        let total=match self.mode {Mode::Forever=>None,Mode::Once=>Some(1),Mode::Count=>Some(self.count.max(1)),Mode::File=>match metadata.loops{Some(0)=>None,Some(count)=>Some(u32::from(count).max(1)),None=>Some(1)}};
        total.is_none_or(|total|self.completed<total)
    }
    pub fn controls(&mut self,ui:&mut egui::Ui,metadata:Metadata) {
        let previous=(self.mode,self.count);
        ui.label(tr("GIF 循环"));
        let label=|mode|tr(match mode{Mode::File=>"按文件设置",Mode::Once=>"播放一次",Mode::Forever=>"无限循环",Mode::Count=>"指定播放次数"});
        egui::ComboBox::from_id_salt("gif_loop_mode").selected_text(label(self.mode)).show_ui(ui,|ui|{
            for mode in [Mode::File,Mode::Once,Mode::Forever,Mode::Count] {ui.selectable_value(&mut self.mode,mode,label(mode));}
        });
        if self.mode==Mode::Count {ui.add(egui::DragValue::new(&mut self.count).range(1..=65535));}
        if self.mode==Mode::File {ui.small(match metadata.loops{Some(0)=>tr("无限循环").to_owned(),Some(count)=>crate::lf!("播放 {0} 次",count.max(1)),None=>tr("播放一次").to_owned()});}
        if previous!=(self.mode,self.count){self.completed=0;}
    }
}
pub fn frame_at(times:&[f64],position:f64)->usize {times.partition_point(|time|*time<=position).saturating_sub(1).min(times.len().saturating_sub(1))}
pub fn frame_duration(times:&[f64],index:usize,duration:f64)->f64 {let start=times.get(index).copied().unwrap_or(0.0);(times.get(index+1).copied().unwrap_or(duration)-start).max(0.0)}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_loop_extensions_and_ignores_compressed_data() {
        let mut source=b"GIF89a\x01\x00\x01\x00\x00\x00\x00".to_vec();
        source.extend_from_slice(b"\x21\xff\x0bNETSCAPE2.0\x03\x01\x02\x00\x00");
        source.extend_from_slice(b"\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02\x44\x01\x00\x3b");
        assert_eq!(parse(&mut source.as_slice()).unwrap(),Metadata{loops:Some(2),frames:1,palette_colors:0});
        assert!(parse(&mut &source[..source.len()-1]).is_err());
        let mut state=State::default();assert!(state.finish(Metadata{loops:Some(2),frames:1,palette_colors:0}));assert!(!state.finish(Metadata{loops:Some(2),frames:1,palette_colors:0}));
        state.completed=0;assert!(!state.finish(Metadata{loops:None,frames:1,palette_colors:0}));
        state.completed=0;for _ in 0..10{assert!(state.finish(Metadata{loops:Some(0),frames:1,palette_colors:0}));}
        state.mode=Mode::Once;assert!(!state.finish(Metadata{loops:Some(0),frames:1,palette_colors:0}));
        state.mode=Mode::Count;state.count=3;state.completed=0;assert!(state.finish(Metadata{loops:None,frames:1,palette_colors:0}));assert!(state.finish(Metadata{loops:None,frames:1,palette_colors:0}));assert!(!state.finish(Metadata{loops:None,frames:1,palette_colors:0}));
    }
    #[test]
    fn reads_global_and_local_palette_capacity() {
        let mut source=b"GIF89a\x01\x00\x01\x00\x80\x00\x00".to_vec();
        source.extend_from_slice(&[0;6]);
        source.extend_from_slice(b"\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x81");
        source.extend_from_slice(&[0;12]);
        source.extend_from_slice(b"\x02\x02\x44\x01\x00\x3b");
        assert_eq!(parse(&mut source.as_slice()).unwrap().palette_colors,4);
    }
    #[test]
    fn variable_frame_durations_and_last_frame_hold() {
        let times=[0.0,0.04,0.24];assert_eq!(frame_at(&times,0.1),1);assert_eq!(frame_at(&times,0.5),2);
        assert!((frame_duration(&times,1,0.54)-0.2).abs()<1e-8);assert!((frame_duration(&times,2,0.54)-0.3).abs()<1e-8);
    }
}
