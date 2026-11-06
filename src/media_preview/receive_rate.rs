//! Encoded input throughput, never decoded frame/PCM output or system-wide traffic.
use std::{collections::VecDeque,io::{BufRead,BufReader,Read},sync::Mutex,time::{Duration,Instant}};

const WINDOW:Duration=Duration::from_secs(3);
const BUCKET:Duration=Duration::from_millis(250);

struct Samples {started:Instant,seen:bool,buckets:VecDeque<(Instant,u64)>}
pub(super) struct Meter(Mutex<Samples>);
impl Default for Meter {
    fn default()->Self {Self(Mutex::new(Samples{started:Instant::now(),seen:false,buckets:VecDeque::new()}))}
}
impl Meter {
    fn record(&self,bytes:u64,now:Instant){
        let mut samples=self.0.lock().unwrap();samples.seen=true;
        while samples.buckets.front().is_some_and(|(time,_)|now.saturating_duration_since(*time)>=WINDOW){samples.buckets.pop_front();}
        if let Some((_,total))=samples.buckets.back_mut().filter(|(time,_)|now.saturating_duration_since(*time)<BUCKET){
            *total=total.saturating_add(bytes);
        }else{samples.buckets.push_back((now,bytes));}
    }
    fn rate_at(&self,now:Instant)->Option<f64>{
        let samples=self.0.lock().unwrap();
        if !samples.seen{return None;}
        let bytes=samples.buckets.iter().filter(|(time,_)|now.saturating_duration_since(*time)<WINDOW)
            .fold(0u64,|sum,(_,bytes)|sum.saturating_add(*bytes));
        let seconds=now.saturating_duration_since(samples.started).min(WINDOW).as_secs_f64().max(0.25);
        Some(bytes as f64/seconds)
    }
    pub(super) fn label(&self)->String {format_rate(self.rate_at(Instant::now()))}
    pub(super) fn read_packets(&self,reader:impl Read){
        let mut reader=BufReader::new(reader);let mut line=Vec::with_capacity(256);let mut oversized=false;let mut parser=PacketSize::default();
        // Never keep a full diagnostic log or let a malformed line grow without bound.
        loop {
            let buffer=match reader.fill_buf(){
                Ok(buffer)=>buffer,
                Err(error) if error.kind()==std::io::ErrorKind::Interrupted=>continue,
                Err(_)=>break,
            };
            if buffer.is_empty(){break;}
            let end=buffer.iter().position(|byte|*byte==b'\n');
            let count=end.map_or(buffer.len(),|index|index+1);
            if !oversized && line.len()+count<=4096{line.extend_from_slice(&buffer[..count]);}else{oversized=true;}
            reader.consume(count);
            if end.is_some(){
                if oversized{parser=PacketSize::default();}
                else if let Some(bytes)=parser.line(&String::from_utf8_lossy(&line)){self.record(bytes,Instant::now());}
                line.clear();oversized=false;
            }
        }
    }
}

#[derive(Default)]
struct PacketSize {pending:bool}
impl PacketSize {
    fn line(&mut self,line:&str)->Option<u64>{
        let text=line.trim();
        if text.strip_prefix("stream #").and_then(|text|text.strip_suffix(':'))
            .is_some_and(|index|!index.is_empty() && index.bytes().all(|byte|byte.is_ascii_digit())){
            self.pending=true;return None;
        }
        if !self.pending{return None;}
        if let Some(size)=text.strip_prefix("size="){
            self.pending=false;
            return (!size.is_empty() && size.bytes().all(|byte|byte.is_ascii_digit())).then(||size.parse().ok()).flatten();
        }
        if !["keyframe=","duration=","dts="].iter().any(|prefix|text.starts_with(prefix)){self.pending=false;}
        None
    }
}

fn format_rate(rate:Option<f64>)->String{
    let Some(rate)=rate else{return "— KB/s".into();};
    if rate>=1_000_000.0{format!("{:.1} MB/s",rate/1_000_000.0)}else{format!("{:.0} KB/s",rate/1_000.0)}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_packets_only_and_bounded_diagnostics(){
        let meter=Meter::default();
        let log="size=999999\nframe=20 size=20000\nstream #0:\n  keyframe=1\n  duration=0.040\n  dts=0 pts=0\n  size=1200\nstream #1:\n  size=300\n[AVIOContext] Statistics: 99999 bytes written\n";
        meter.read_packets(log.as_bytes());
        assert_eq!(meter.0.lock().unwrap().buckets.iter().map(|(_,bytes)|bytes).sum::<u64>(),1500);
        let oversized=format!("stream #0:\n{}\nsize=50000\nstream #0:\n  size=100\n","x".repeat(10000));
        meter.read_packets(oversized.as_bytes());
        assert_eq!(meter.0.lock().unwrap().buckets.iter().map(|(_,bytes)|bytes).sum::<u64>(),1600);
    }
    #[test]
    fn rolling_rate_expires_and_formats_without_using_media_bitrate(){
        let meter=Meter::default();let started=meter.0.lock().unwrap().started;
        assert_eq!(meter.rate_at(started),None);
        meter.record(900_000,started);assert_eq!(meter.rate_at(started+Duration::from_secs(3)),Some(0.0));
        meter.record(900_000,started+Duration::from_secs(3));
        assert_eq!(meter.rate_at(started+Duration::from_secs(4)),Some(300_000.0));
        assert_eq!(format_rate(Some(300_000.0)),"300 KB/s");assert_eq!(format_rate(Some(1_000_000.0)),"1.0 MB/s");
        assert_eq!(format_rate(Some(0.0)),"0 KB/s");
    }
}
