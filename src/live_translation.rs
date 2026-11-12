//! One bounded, disposable M2M100 process per active live translation session.
use std::{collections::{HashMap,HashSet},io::{BufRead,BufReader,Write},process::{Child,Stdio},sync::{Arc,Mutex,mpsc}};
use eframe::egui;
pub(crate) struct Translation {
    source_language:Option<String>,
    visible_sources:HashSet<String>,
    pub backend:String,pub enabled:bool,pub target:usize,pub hide_source:bool,pub error:Option<String>,
    recent:Option<(String,String,std::time::Instant)>,session:Option<Session>,cache:HashMap<String,String>,pending:HashSet<String>,
}
impl Default for Translation {fn default()->Self{Self{visible_sources:HashSet::new(),source_language:None,backend:String::new(),enabled:false,target:1,hide_source:false,error:None,recent:None,session:None,cache:HashMap::new(),pending:HashSet::new()}}}
pub(crate) struct Display<'a> {pub source:Option<&'a str>,pub translated:Option<&'a str>,pub previous:bool}
struct Session { child:Arc<Mutex<Child>>,sender:mpsc::SyncSender<String>,results:mpsc::Receiver<Result<(String,String,String),String>> }
impl Drop for Session {fn drop(&mut self){let child=self.child.clone();std::thread::spawn(move||{let mut child=child.lock().unwrap();let _=child.kill();let _=child.wait();});}}
impl Translation {
    pub fn reset(&mut self){self.error=None;self.source_language=None;self.backend.clear();self.session=None;self.recent=None;self.cache.clear();self.pending.clear();self.visible_sources.clear();}
    pub fn pair<'a>(&'a self,source:&'a str)->Option<(&'a str,&'a str)>{
        self.cache.get(source).map(|text|(source,text.as_str())).or_else(||self.recent.as_ref().filter(|(_,_,time)|time.elapsed()<std::time::Duration::from_secs(6)).map(|(source,text,_)|(source.as_str(),text.as_str())))
    }
    pub fn display<'a>(&'a self,source:Option<&'a str>)->Display<'a>{
        let source=source.filter(|text|!text.is_empty());
        if !self.enabled{return Display{source,translated:None,previous:false};}
        let pair=self.pair(source.unwrap_or(""));
        Display{source:if self.hide_source{None}else{source},translated:pair.map(|(_,text)|text),previous:pair.is_some_and(|(original,_)|Some(original)!=source)}
    }
    pub fn mark_visible(&mut self,source:&str){
        if source.is_empty(){return;}
        if self.visible_sources.len()>=128{self.visible_sources.clear();}
        self.visible_sources.insert(source.to_owned());
        if let Some(text)=self.cache.get(source){self.recent=Some((source.to_owned(),text.clone(),std::time::Instant::now()));}
    }
    pub fn submit(&mut self,source:&str,language:&str,ctx:&egui::Context){
        if !self.enabled||source.is_empty(){return;}
        if self.source_language.as_deref()!=Some(language){self.reset();self.source_language=Some(language.to_owned());}
        if self.error.is_some(){return;}
        let target=["zh","en","ja","ko"][self.target.min(3)];
        if language==target {if self.cache.len()>=128{self.cache.clear();}self.cache.insert(source.to_owned(),source.to_owned());return;}
        if !["zh","en","ja","ko"].contains(&language){self.error=Some("Unsupported live translation source language".into());return;}
        if self.cache.contains_key(source)||self.pending.contains(source){return;}
        if self.session.is_none(){match Session::start(language,target,crate::ai_runtime::settings().cpu_threads,ctx){Ok(s)=>self.session=Some(s),Err(e)=>{self.error=Some(e);return;}}}
        if let Some(session)=&self.session {if session.sender.try_send(source.to_owned()).is_ok(){self.pending.insert(source.to_owned());}}
    }
    pub fn poll(&mut self){
        let mut error=None;
        if let Some(session)=&self.session {for result in session.results.try_iter(){match result{
            Ok((source,text,backend))=>{self.backend=backend;self.pending.remove(&source);if self.cache.len()>=128{self.cache.clear();}if self.visible_sources.contains(&source){self.recent=Some((source.clone(),text.clone(),std::time::Instant::now()));}self.cache.insert(source,text);},
            Err(e)=>{error=Some(e);break;}
        }}}
        if let Some(error)=error{self.reset();self.error=Some(error);self.enabled=false;}
    }
}
impl Session {
    fn start(source:&str,target:&str,cpu_threads:usize,ctx:&egui::Context)->Result<Self,String>{
        if !crate::subtitle_translation::m2m_installed(){return Err(crate::localization::tr("请先安装 M2M100 翻译模型。").into());}
        let mut command=crate::auto_subtitle::python()?;
        #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000|0x00004000);}
        command.env("OMP_WAIT_POLICY","PASSIVE").env("KMP_BLOCKTIME","0");
        let mut child=command.args(["-I","-u","-c",include_str!("m2m_translation.py"),include_str!("translation_download.py"),"live"])
            .arg(crate::subtitle_translation::m2m_root()?).args([source,target,&cpu_threads.clamp(1,4).to_string()]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
        let mut input=child.stdin.take().ok_or("Missing translation stdin")?;
        let output=child.stdout.take().ok_or("Missing translation stdout")?;
        let errors=child.stderr.take();let diagnostics=Arc::new(Mutex::new(String::new()));let log=diagnostics.clone();
        std::thread::spawn(move||{if let Some(errors)=errors {for line in BufReader::new(errors).lines().map_while(Result::ok){let mut log=log.lock().unwrap();if log.len()<8192{log.push_str(&line);log.push('\n');}}}});
        let child=Arc::new(Mutex::new(child));let (sender,requests)=mpsc::sync_channel::<String>(1);let (tx,results)=mpsc::channel();let ctx=ctx.clone();
        std::thread::spawn(move||{let mut output=BufReader::new(output);for source in requests{
            let result=(||{
                writeln!(input,"{}",serde_json::json!({"text":source})).map_err(|e|e.to_string())?;input.flush().map_err(|e|e.to_string())?;
                let mut line=String::new();if output.read_line(&mut line).map_err(|e|e.to_string())?==0{return Err(format!("M2M100 process exited: {}",diagnostics.lock().unwrap()));}
                let result:serde_json::Value=serde_json::from_str(&line).map_err(|e|e.to_string())?;
                if let Some(error)=result["error"].as_str(){return Err(error.to_owned());}
                if result["source"].as_str()!=Some(&source){return Err("M2M100 result source mismatch".into());}
                Ok((source,result["text"].as_str().ok_or("Missing translation")?.to_owned(),result["backend"].as_str().unwrap_or("").to_owned()))
            })();let failed=result.is_err();let _=tx.send(result);ctx.request_repaint();if failed{break;}
        }});
        Ok(Self{child,sender,results})
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prefetched_text_becomes_recent_only_after_its_media_time(){
        let mut state=Translation::default();state.enabled=true;state.cache.insert("future".into(),"future translation".into());
        assert!(state.display(None).translated.is_none());
        state.mark_visible("current");assert!(state.display(Some("current")).translated.is_none());
        state.mark_visible("future");assert_eq!(state.display(Some("future")).translated,Some("future translation"));
    }
    use super::*;
    #[test]
    fn source_updates_without_waiting_for_translation(){
        let mut state=Translation::default();state.enabled=true;
        state.recent=Some(("earlier".into(),"earlier translation".into(),std::time::Instant::now()));
        let display=state.display(Some("new source"));
        assert_eq!(display.source,Some("new source"));
        assert_eq!(display.translated,Some("earlier translation"));assert!(display.previous);
        state.cache.insert("new source".into(),"new translation".into());
        let display=state.display(Some("new source"));
        assert_eq!(display.source,Some("new source"));assert_eq!(display.translated,Some("new translation"));assert!(!display.previous);
    }
    #[test]
    fn disabled_translation_preserves_source_and_expired_results_do_not_linger(){
        let mut state=Translation::default();state.hide_source=true;
        let display=state.display(Some("source"));assert_eq!(display.source,Some("source"));assert!(display.translated.is_none());
        state.enabled=true;state.recent=Some(("old".into(),"old translation".into(),std::time::Instant::now()-std::time::Duration::from_secs(7)));
        let display=state.display(Some("new source"));assert!(display.source.is_none());assert!(display.translated.is_none());
    }
    #[test]
    fn reset_invalidates_results_and_pending_work(){
        let mut state=Translation::default();state.enabled=true;
        state.cache.insert("old".into(),"translation".into());state.pending.insert("pending".into());
        state.recent=Some(("old".into(),"translation".into(),std::time::Instant::now()));
        assert_eq!(state.pair("old"),Some(("old","translation")));
        state.reset();assert!(state.pair("old").is_none());assert!(state.pending.is_empty());assert!(state.session.is_none());
    }
}
