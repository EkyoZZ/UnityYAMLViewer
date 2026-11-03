//! Markdown layout with a limited HTML compatibility layer and asynchronous images.
use std::{collections::HashMap, io::Read, path::{Path, PathBuf}, sync::mpsc, time::Duration};
use eframe::egui;
use pulldown_cmark::{Event, Parser, Tag};
use sha2::{Digest, Sha256};

const LIMIT:u64=16*1024*1024;
type Download=Result<(Vec<u8>,bool),String>;
enum Image { Pending(mpsc::Receiver<Download>), Ready(String), Failed(String) }
#[derive(Default)]
pub(super) struct Preview {
    pub source:bool,
    cache:egui_commonmark::CommonMarkCache,
    images:HashMap<String,Image>,
    document:PathBuf,
}

fn attribute(tag:&str,name:&str)->Option<String>{
    let pattern=format!(r#"(?i)\b{}\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#,name);
    let re=regex::Regex::new(&pattern).ok()?;
    let captures=re.captures(tag)?;
    Some(unescape(captures.get(1).or_else(||captures.get(2)).or_else(||captures.get(3))?.as_str()))
}
fn unescape(value:&str)->String {value.replace("&#10;","\n").replace("&#13;","\r").replace("&quot;","\"").replace("&#39;","'").replace("&lt;","<").replace("&gt;",">").replace("&amp;","&")}

// Only transform parser-identified HTML, leaving fenced/inline code untouched.
fn html(value:&str,anchors:&mut Vec<String>)->String {
    let comments=regex::Regex::new(r"(?s)<!--.*?-->").unwrap();
    let value=comments.replace_all(value,"");
    let tags=regex::Regex::new(r"(?is)<[^>]+>").unwrap();
    tags.replace_all(&value,|c:&regex::Captures|{
        let tag=&c[0];let lower=tag.to_ascii_lowercase();
        if lower.starts_with("<img ") {
            attribute(tag,"src").map(|src|format!("![{}](<{}>)",attribute(tag,"alt").unwrap_or_default().replace(['[',']'],""),src.replace(['\n','\r','<','>'],""))).unwrap_or_default()
        } else if lower.starts_with("<a ") {
            if let Some(href)=attribute(tag,"href"){anchors.push(href.replace(['\n','\r','<','>'],""));"[".into()}else{String::new()}
        } else if lower.starts_with("</a") {anchors.pop().map(|href|format!("](<{href}>)")).unwrap_or_default()}
        else if lower.starts_with("<h") && lower.as_bytes().get(2).is_some_and(|b|b.is_ascii_digit()) {
            format!("\n\n{} ","#".repeat((lower.as_bytes()[2]-b'0').clamp(1,6) as usize))
        } else if lower.starts_with("</h") || lower.starts_with("<p") || lower.starts_with("</p") || lower.starts_with("<div") || lower.starts_with("</div") || lower.starts_with("<center") || lower.starts_with("</center") {"\n\n".into()}
        else if lower.starts_with("<br") {"  \n".into()}
        else if matches!(lower.as_str(),"<b>"|"</b>"|"<strong>"|"</strong>") {"**".into()}
        else if matches!(lower.as_str(),"<i>"|"</i>"|"<em>"|"</em>") {"*".into()}
        else {tag.to_owned()}
    }).lines().map(str::trim_start).collect::<Vec<_>>().join("\n")
}

fn normalized(text:&str)->String {
    let mut result=String::new();let mut end=0;let mut anchors=Vec::new();
    for (event,range) in Parser::new(text).into_offset_iter(){
        if matches!(event,Event::Html(_)|Event::InlineHtml(_)){
            result.push_str(&text[end..range.start]);result.push_str(&html(&text[range.clone()],&mut anchors));end=range.end;
        }
    }
    result.push_str(&text[end..]);
    result
}

// Scan only HTML events so alignment-like text in code stays literal.
fn aligned_sections(text:&str)->Vec<(bool,String)>{
    let tags=regex::Regex::new(r"(?is)<!--.*?-->|</?([a-z][a-z0-9]*)\b[^>]*>").unwrap();
    let mut stack:Vec<(String,bool)>=Vec::new();let mut sections=Vec::new();let mut start=0;let mut centered=false;
    for (event,range) in Parser::new(text).into_offset_iter(){
        if !matches!(event,Event::Html(_)|Event::InlineHtml(_)){continue;}
        for captures in tags.captures_iter(&text[range.clone()]){
            let Some(name)=captures.get(1)else{continue};let name=name.as_str().to_ascii_lowercase();
            if !matches!(name.as_str(),"p"|"div"|"center"|"h1"|"h2"|"h3"|"h4"|"h5"|"h6"){continue;}
            let token=captures.get(0).unwrap();let tag=token.as_str();let before=centered;
            if tag.starts_with("</"){
                if let Some(index)=stack.iter().rposition(|(open,_)|open==&name){stack.truncate(index);}
            }else{
                let align=attribute(tag,"align").is_some_and(|a|a.trim().eq_ignore_ascii_case("center"));
                let style=attribute(tag,"style").is_some_and(|style|style.split(';').any(|rule|rule.split_once(':').is_some_and(|(key,value)|key.trim().eq_ignore_ascii_case("text-align")&&value.trim().eq_ignore_ascii_case("center"))));
                stack.push((name,align||style||tag.to_ascii_lowercase().starts_with("<center")));
            }
            centered=stack.iter().any(|(_,center)|*center);
            if before!=centered{
                let boundary=range.start+if centered{token.start()}else{token.end()};
                if boundary>start{sections.push((before,text[start..boundary].to_owned()));}start=boundary;
            }
        }
    }
    if start<text.len(){sections.push((centered,text[start..].to_owned()));}sections
}
fn escape_attribute(value:&str)->String{value.replace('&',"&amp;").replace('"',"&quot;").replace('<',"&lt;").replace('>',"&gt;").replace('\n',"&#10;").replace('\r',"&#13;")}
fn linked_images(text:&str)->String{
    let mut result=text.to_owned();let mut replacements=Vec::new();
    for (event,range) in Parser::new(text).into_offset_iter(){
        if let Event::Start(Tag::Link{dest_url,..})=event{
            let image=Parser::new(&text[range.clone()]).find_map(|event|if let Event::Start(Tag::Image{dest_url,..})=event{Some(dest_url.into_string())}else{None});
            let value=if let Some(source)=image{format!("<uyv-image src=\"{}\" href=\"{}\" />",escape_attribute(&source),escape_attribute(&dest_url))}
            else{let label=Parser::new(&text[range.clone()]).filter_map(|event|match event{Event::Text(s)|Event::Code(s)=>Some(s.into_string()),Event::SoftBreak|Event::HardBreak=>Some(" ".into()),_=>None}).collect::<String>();format!("<uyv-link label=\"{}\" href=\"{}\" />",escape_attribute(&label),escape_attribute(&dest_url))};
            replacements.push((range,value));
        }else if let Event::Start(Tag::CodeBlock(kind))=event{
            let language=match kind{pulldown_cmark::CodeBlockKind::Fenced(info)=>info.split_whitespace().next().unwrap_or("").to_owned(),_=>String::new()};
            let code=Parser::new(&text[range.clone()]).filter_map(|event|if let Event::Text(s)=event{Some(s.into_string())}else{None}).collect::<String>();
            replacements.push((range,format!("<uyv-code language=\"{}\" text=\"{}\" />\n\n",escape_attribute(&language),escape_attribute(&code))));
        }
    }
    for (range,value) in replacements.into_iter().rev(){result.replace_range(range,&value);}result
}
fn heading_rules(text:&str)->String{
    let events:Vec<_>=Parser::new(text).into_offset_iter().collect();let mut result=text.to_owned();
    for (index,(event,range)) in events.iter().enumerate().rev(){
        if matches!(event,Event::End(pulldown_cmark::TagEnd::Heading(pulldown_cmark::HeadingLevel::H1|pulldown_cmark::HeadingLevel::H2)))&&!events.get(index+1).is_some_and(|(event,_)|matches!(event,Event::Rule)){
            result.insert_str(range.end,"\n\n---\n\n");
        }
    }result
}
fn powershell_layout(ui:&egui::Ui,text:&str)->egui::text::LayoutJob{
    static TOKENS:std::sync::LazyLock<regex::Regex>=std::sync::LazyLock::new(||regex::Regex::new(
        r#"(?m)<\#(?s:.*?)\#>|\#[^\r\n]*|"(?:`[\s\S]|[^"`])*"|'(?:''|[^'])*'|\$\{[^}]*\}|\$(?:[A-Za-z_][A-Za-z_0-9]*:)?[A-Za-z_][A-Za-z_0-9]*|-[A-Za-z][A-Za-z0-9_-]*|\b[A-Za-z]+-[A-Za-z][A-Za-z0-9-]*\b|[|;&]"#
    ).unwrap());
    let dark=ui.visuals().dark_mode;
    let command=if dark{egui::Color32::from_rgb(100,205,255)}else{egui::Color32::from_rgb(0,85,160)};
    let parameter=if dark{egui::Color32::from_rgb(255,160,130)}else{egui::Color32::from_rgb(160,55,25)};
    let string=if dark{egui::Color32::from_rgb(175,220,255)}else{egui::Color32::from_rgb(20,90,130)};
    let comment=if dark{egui::Color32::from_rgb(135,155,140)}else{egui::Color32::from_rgb(60,115,65)};
    let variable=if dark{egui::Color32::from_rgb(225,185,120)}else{egui::Color32::from_rgb(125,75,10)};
    let font=egui::TextStyle::Monospace.resolve(ui.style());let mut job=egui::text::LayoutJob::default();let mut end=0;
    for token in TOKENS.find_iter(text){
        job.append(&text[end..token.start()],0.0,egui::TextFormat::simple(font.clone(),ui.visuals().text_color()));
        let value=token.as_str();let color=if value.starts_with('#')||value.starts_with("<#"){comment}else if value.starts_with(['\"','\'']){string}else if value.starts_with('$'){variable}else if value.starts_with('-')||matches!(value,"|"|";"|"&"){parameter}else{command};
        job.append(value,0.0,egui::TextFormat::simple(font.clone(),color));end=token.end();
    }
    job.append(&text[end..],0.0,egui::TextFormat::simple(font,ui.visuals().text_color()));job
}
fn inline_image(ui:&mut egui::Ui,value:&str){
    let tags=regex::Regex::new(r"(?s)<uyv-(image|link|code)\b[^>]*>").unwrap();
    for (index,captures) in tags.captures_iter(value).enumerate(){
        let tag=captures.get(0).unwrap().as_str();
        if index>0{ui.add_space(4.0);}
        match &captures[1]{
            "image"=>if let Some(source)=attribute(tag,"src"){
                let response=ui.add(egui::Image::new(source).fit_to_original_size(1.0).max_width(ui.max_rect().width()).sense(egui::Sense::click()));
                if let Some(href)=attribute(tag,"href"){
                    if response.clicked(){ui.ctx().open_url(egui::OpenUrl::new_tab(href.clone()));}
                    response.on_hover_text(href).on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            },
            "link"=>if let (Some(label),Some(href))=(attribute(tag,"label"),attribute(tag,"href")){
                ui.hyperlink_to(egui::RichText::new(label).color(crate::theme::BLUE).underline(),href);
            },
            "code"=>if let Some(code)=attribute(tag,"text"){
                let language=attribute(tag,"language").unwrap_or_default();
                let color=if ui.visuals().dark_mode{egui::Color32::from_rgb(120,205,255)}else{egui::Color32::from_rgb(0,85,150)};
                ui.push_id(("markdown_code",index,format!("{:x}",Sha256::digest(code.as_bytes()))),|ui|{
                    let mut layouter=|ui:&egui::Ui,string:&str,wrap_width:f32|{
                        let mut job=if matches!(language.to_ascii_lowercase().as_str(),"powershell"|"ps1"|"pwsh"|"psm1"|"psd1"){powershell_layout(ui,string)}else if language.is_empty(){egui::text::LayoutJob::simple(string.to_owned(),egui::TextStyle::Monospace.resolve(ui.style()),color,wrap_width)}else{
                            let theme=egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(),ui.style());
                            let mut job=egui_extras::syntax_highlighting::highlight(ui.ctx(),ui.style(),&theme,string,&language);
                            if job.sections.first().is_some_and(|first|job.sections.iter().all(|section|section.format.color==first.format.color)){
                                for section in &mut job.sections{section.format.color=color;}
                            }job
                        };
                        job.wrap.max_width=wrap_width;ui.fonts(|fonts|fonts.layout_job(job))
                    };
                    egui_commonmark::code_block(ui,ui.available_width(),&code,&mut layouter);
                });
            },_=>{}
        }
    }
}
fn ready_image(bytes:Vec<u8>)->Download{
    let svg=String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).contains("<svg");
    if svg{super::svg_preview::svg_with_text(&bytes).map(|bytes|(bytes,true))}else{Ok((bytes,false))}
}

fn download(source:&str)->Download {
    let root=crate::app_cache::network_root().join("markdown_images");
    let key=format!("{:x}",Sha256::digest(source.as_bytes()));
    let file=root.join(key);
    let cached=std::fs::metadata(&file).ok().filter(|m|m.len()<=LIMIT).and_then(|_|std::fs::read(&file).ok());
    if let Some(bytes)=cached {return ready_image(bytes);}
    let bytes=if source.starts_with("https://")||source.starts_with("http://") {
        let client=reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(10)).timeout(Duration::from_secs(30)).build().map_err(|e|e.to_string())?;
        let response=client.get(source).send().and_then(|r|r.error_for_status()).map_err(|e|e.to_string())?;
        if response.content_length().is_some_and(|n|n>LIMIT){return Err("Image exceeds 16 MiB".into());}
        let mut bytes=Vec::new();response.take(LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len() as u64>LIMIT{return Err("Image exceeds 16 MiB".into());}
        let _guard=crate::app_cache::mutation_guard();
        if std::fs::create_dir_all(&root).is_ok(){let partial=file.with_extension(format!("{}-{}.part",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));if std::fs::write(&partial,&bytes).is_ok(){let _=std::fs::rename(&partial,&file);}}
        bytes
    } else {
        let path=reqwest::Url::parse(source).ok().and_then(|url|url.to_file_path().ok()).ok_or("Unsupported image URL")?;
        let file=std::fs::File::open(path).map_err(|e|e.to_string())?;
        let mut bytes=Vec::new();file.take(LIMIT+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len() as u64>LIMIT{return Err("Image exceeds 16 MiB".into());}bytes
    };
    ready_image(bytes)
}

impl Preview {
    pub fn invalidate(&mut self){self.document=PathBuf::new();}
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path,text:&str,reading:&super::TextReading){
        if self.document!=path {for image in self.images.values(){if let Image::Ready(uri)=image{ui.ctx().forget_image(uri);}} self.images.clear();self.cache=Default::default();self.document=path.to_owned();}
        ui.scope(|ui|{
            let base=egui::TextStyle::Body.resolve(ui.style()).size;
            for font in ui.style_mut().text_styles.values_mut(){font.size*=reading.size/base;}
            ui.style_mut().spacing.item_spacing.y=reading.paragraph+reading.size*(reading.line-1.0);
            for (index,(centered,section)) in aligned_sections(text).into_iter().enumerate(){
                let has_heading=Parser::new(&normalized(&section)).any(|event|matches!(event,Event::Start(Tag::Heading{level:pulldown_cmark::HeadingLevel::H1|pulldown_cmark::HeadingLevel::H2,..})));
                let rendered=linked_images(&self.resolve_images(ui,path,&if centered{normalized(&section)}else{heading_rules(&normalized(&section))}));
                if rendered.trim().is_empty(){continue;}
                ui.push_id(("markdown_section",index),|ui|{
                    let width=ui.available_width().max(1.0);
                    if centered{
                        let origin=ui.next_widget_position();
                        let mut measure=ui.new_child(egui::UiBuilder::new().id_salt("measure").max_rect(egui::Rect::from_min_size(origin,egui::vec2(width,100000.0))).layout(egui::Layout::top_down(egui::Align::LEFT)).sizing_pass().invisible());
                        egui_commonmark::CommonMarkViewer::new().render_html_fn(Some(&inline_image)).max_image_width(Some(width as usize)).show(&mut measure,&mut self.cache,&rendered);
                        let size=measure.min_rect().size();let content_width=size.x.min(width).max(1.0);
                        let rect=egui::Rect::from_min_size(origin+egui::vec2((width-content_width)*0.5,0.0),egui::vec2(content_width,size.y));
                        let mut child=ui.new_child(egui::UiBuilder::new().id_salt("centered").max_rect(rect).layout(egui::Layout::top_down(egui::Align::LEFT)));
                        egui_commonmark::CommonMarkViewer::new().render_html_fn(Some(&inline_image)).max_image_width(Some(content_width as usize)).show(&mut child,&mut self.cache,&rendered);
                        ui.advance_cursor_after_rect(egui::Rect::from_min_max(origin,egui::pos2(origin.x+width,child.min_rect().bottom())));
                        if has_heading{ui.separator();}
                    }else{egui_commonmark::CommonMarkViewer::new().render_html_fn(Some(&inline_image)).max_image_width(Some(width as usize)).show(ui,&mut self.cache,&rendered);}
                });
            }
        });
    }
    fn resolve_images(&mut self,ui:&mut egui::Ui,path:&Path,text:&str)->String{
        let mut rendered=text.to_owned();
        let replacements:Vec<_>=Parser::new_ext(&rendered,pulldown_cmark::Options::all()).into_offset_iter().filter_map(|(event,range)|{
            if let Event::Start(Tag::Image{dest_url,..})=event {Some((range,dest_url.into_string()))}else{None}
        }).collect();
        for (range,destination) in replacements.into_iter().rev(){
            let source=if Path::new(&destination).is_absolute(){reqwest::Url::from_file_path(&destination).ok().map(|url|url.to_string())}else if let Ok(url)=reqwest::Url::parse(&destination){
                if matches!(url.scheme(),"http"|"https"|"file"){Some(url.to_string())}else{None}
            }else{reqwest::Url::from_file_path(path).ok().and_then(|base|base.join(&destination).ok()).map(|u|u.to_string())};
            let Some(source)=source else {rendered.replace_range(range,"[unsupported image URL]");continue};
            if !self.images.contains_key(&source) && self.images.len()<32 && self.images.values().filter(|image|matches!(image,Image::Pending(_))).count()<4 {
                let (tx,rx)=mpsc::channel();self.images.insert(source.clone(),Image::Pending(rx));
                let ctx=ui.ctx().clone();let source=source.clone();std::thread::spawn(move||{let _=tx.send(download(&source));ctx.request_repaint();});
            }
            if let Some(Image::Pending(rx))=self.images.get(&source) {
                if let Ok(result)=rx.try_recv(){let state=match result {
                    Ok((bytes,svg))=>{let uri=format!("bytes://markdown/{:x}.{}",Sha256::digest(source.as_bytes()),if svg{"svg"}else{"png"});ui.ctx().include_bytes(uri.clone(),bytes);Image::Ready(uri)},
                    Err(error)=>Image::Failed(error),
                };self.images.insert(source.clone(),state);}
            }
            let replacement=match self.images.get(&source){
                Some(Image::Ready(uri))=>format!("![image](<{uri}>)"),
                Some(Image::Failed(error))=>format!("[{}: {}]",crate::localization::tr("图片加载失败"),error.replace(['[',']','\n']," ")),
                None if self.images.len()>=32=>format!("[{}]",crate::localization::tr("图片数量超过预览限制")),
                _=>format!("[{}]",crate::localization::tr("正在加载图片")),
            };rendered.replace_range(range,&replacement);
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn html_images_and_comments(){
        let source="<h1 align=\"center\">Scoop</h1>\n\n<!-- <img src=\"hidden.png\"> -->\n<img src='badge.svg' alt='badge'>";
        let result=normalized(source);assert!(result.contains("# Scoop"));assert!(result.contains("![badge](<badge.svg>)"));assert!(!result.contains("hidden.png"));
    }
    #[test] fn html_links_keep_labels_and_images(){
        let result=normalized(r#"<p align="center"><a href="https://example.com"><img src="https://example.com/badge.svg" alt="badge"></a></p>"#);
        assert!(result.contains("[![badge](<https://example.com/badge.svg>)](<https://example.com>)"));
    }
    #[test] fn centered_blocks_preserve_normal_content_and_code(){
        let source="before\n\n<h1 align=\"center\">Title</h1>\n\n<p style=\"text-align:center\">links</p>\n\nafter\n\n```html\n<p align=\"center\">code</p>\n```";
        let sections=aligned_sections(source);assert_eq!(sections.iter().filter(|(center,_)|*center).count(),2);
        assert_eq!(sections.iter().map(|(_,s)|s.as_str()).collect::<String>(),source);
        assert!(!sections.last().unwrap().0);
    }
    #[test] fn linked_image_keeps_target(){let result=linked_images("[![badge](bytes://badge.svg)](https://example.com)");assert!(result.contains("href=\"https://example.com\""));assert!(result.contains("src=\"bytes://badge.svg\""));}
    #[test] fn group_keeps_all_images_and_links(){
        let source="[![a](bytes://a.svg)](https://example.com/a) [![b](bytes://b.svg)](https://example.com/b) [docs](https://example.com/docs)";
        let rendered=linked_images(source);assert_eq!(rendered.matches("<uyv-image ").count(),2);assert_eq!(rendered.matches("<uyv-link ").count(),1);
    }
    #[test] fn code_attributes_round_trip_and_rules_do_not_duplicate(){
        let code="echo \"hello\" & <tag>\nnext\n";
        let tag=format!("<uyv-code text=\"{}\" />",escape_attribute(code));assert_eq!(attribute(&tag,"text").unwrap(),code);
        let result=heading_rules("# Title\n\n---\n\n## Second\n\nbody");assert_eq!(result.matches("---").count(),2);
    }
    #[test] fn inline_links_in_lists_use_the_html_callback(){
        let ctx=egui::Context::default();let mut cache=egui_commonmark::CommonMarkCache::default();
        let source=linked_images("- Eliminates [User Account Control](https://example.com) prompts.");
        let calls=std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let recorded=calls.clone();
        let html=move |ui:&mut egui::Ui,text:&str|{if text.contains("<uyv-link "){recorded.fetch_add(1,std::sync::atomic::Ordering::Relaxed);}inline_image(ui,text);};
        let _=ctx.run(Default::default(),|ctx|{egui::CentralPanel::default().show(ctx,|ui|{egui_commonmark::CommonMarkViewer::new().render_html_fn(Some(&html)).show(ui,&mut cache,&source);});});
        assert!(calls.load(std::sync::atomic::Ordering::Relaxed)>0,"inline list links must reach the HTML renderer");
    }
    #[test] fn code_is_not_html(){let source="```html\n<img src='code.png'>\n```\n\n`<img src='inline.png'>`";assert_eq!(normalized(source),source);}
}
