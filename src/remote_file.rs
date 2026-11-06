//! Classify HTTP(S) links and open video streams or download ordinary files.
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::mpsc, time::Duration};

mod video_cache;
mod hls_scan;
pub(crate) use video_cache::Download;

const LIMIT: u64 = 1024 * 1024 * 1024;

/// Decode one layer of pasted Unicode escapes; preserve percent encoding.
pub(crate) fn decode_pasted_unicode(text: &str) -> String {
    let unit = |value: &str| value.strip_prefix("\\u").and_then(|value| value.get(..4))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .and_then(|value| u16::from_str_radix(value, 16).ok());
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        if rest.starts_with("\\\\") {
            output.push_str("\\\\");
            index += 2;
            continue;
        }
        if let Some(first) = unit(rest) {
            let length = if (0xd800..=0xdbff).contains(&first)
                && rest.get(6..).and_then(unit).is_some_and(|second| (0xdc00..=0xdfff).contains(&second)) { 12 } else { 6 };
            if let Some(encoded) = rest.get(..length) {
                if let Ok(decoded) = serde_json::from_str::<String>(&format!("\"{encoded}\"")) {
                    if !decoded.chars().any(char::is_control) {
                        output.push_str(&decoded);
                        index += length;
                        continue;
                    }
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}
fn cache_root() -> PathBuf { crate::app_cache::network_root() }
pub fn is_cached(path: &Path) -> bool { path.starts_with(cache_root()) || path.starts_with(std::env::temp_dir().join("UnityYAMLViewer-network")) }
fn origin_key(input:&str)->Option<String>{
    use sha2::{Digest,Sha256};
    let mut url=reqwest::Url::parse(input).ok()?;url.set_fragment(None);
    Some(format!("{:x}",Sha256::digest(url.as_str().as_bytes())))
}
pub(crate) fn playlist_origin(path:&Path)->Option<String>{
    if !is_cached(path){return None;}
    let mut bytes=Vec::new();std::fs::File::open(path.parent()?.join("playlist-origin.key")).ok()?.take(65).read_to_end(&mut bytes).ok()?;
    (bytes.len()==64 && bytes.iter().all(u8::is_ascii_hexdigit)).then(||String::from_utf8(bytes).ok()).flatten()
}
fn download_name(encoded: &str) -> String {
    let input=encoded.as_bytes();let mut bytes=Vec::with_capacity(input.len());let mut index=0;
    while index<input.len() {
        if input[index]==b'%' && index+2<input.len() {
            let digit=|byte:u8| (byte as char).to_digit(16).map(|value|value as u8);
            if let Some(value)=digit(input[index+1]).zip(digit(input[index+2])).map(|(a,b)|a*16+b) {
                bytes.push(value);index+=3;continue;
            }
        }
        bytes.push(input[index]);index+=1;
    }
    let decoded=String::from_utf8(bytes).unwrap_or_else(|_|encoded.to_owned());
    let name:String=decoded.chars().map(|ch|if ch.is_control() || "<>:\"/\\|?*".contains(ch){'_'}else{ch}).take(160).collect();
    let name=name.trim_end_matches([' ','.']);
    if name.is_empty(){"download".into()}else{name.to_owned()}
}
pub(crate) fn file_label(path: &Path) -> String {
    // Decode only cached display labels, once; keep cache paths and signed URLs unchanged.
    let name=path.file_name().unwrap_or(path.as_os_str()).to_string_lossy();
    if is_cached(path){download_name(&name)}else{name.into_owned()}
}
fn hls_url(input: &str) -> Option<reqwest::Url> {
    if !input.starts_with("https://") && !input.starts_with("http://") { return None; }
    let url = reqwest::Url::parse(input).ok()?;
    if url.host_str().is_some() && url.path().to_ascii_lowercase().ends_with(".m3u8") {
        Some(url)
    } else {
        None
    }
}

pub fn is_hls(path: &Path) -> bool { path.to_str().and_then(hls_url).is_some() }

fn hls_response(url: &str, mime: &str) -> bool {
    if reqwest::Url::parse(url).is_ok_and(|url| url.path().to_ascii_lowercase().ends_with(".m3u")) { return false; }
    hls_url(url).is_some() || matches!(mime,
        "application/vnd.apple.mpegurl" | "application/x-mpegurl" | "audio/mpegurl" | "audio/x-mpegurl")
}

pub fn direct_video(input: &str) -> Option<PathBuf> {
    let url = reqwest::Url::parse(input).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() { return None; }
    let extension = Path::new(url.path()).extension()?.to_str()?.to_ascii_lowercase();
    matches!(extension.as_str(), "mp4" | "m4v" | "mkv" | "webm" | "mov" | "avi" | "wmv" |
        "flv" | "mpeg" | "mpg" | "ts" | "mts" | "m2ts" | "3gp").then(|| PathBuf::from(url.as_str()))
}

pub fn stream_label(path: &Path) -> Option<&str> {
    let input = path.to_str()?;
    hls_url(input)?;
    input.split('?').next()?.rsplit('/').next()
}

pub fn clean_title(value: &str) -> Option<String> {
    let cleaned: String = value.chars().filter(|c| !c.is_control()).collect();
    let title = cleaned.trim();
    (!title.is_empty()).then(|| title.to_owned())
}

fn link_title(url: &reqwest::Url) -> Option<String> {
    url.query_pairs().filter(|(key, _)| key == "name")
        .find_map(|(_, value)| clean_title(&value))
}

pub fn stream_source(input: &str) -> Option<(PathBuf, Option<String>)> {
    let normalized = input.replace("https\\://", "https://").replace("http\\://", "http://");
    let (mut source, title) = if let Some(url) = hls_url(&normalized) {
        let title = link_title(&url);
        (url, title)
    } else {
        let wrapper = reqwest::Url::parse(&normalized).ok()?;
        if wrapper.origin().ascii_serialization() != "https://web.zyplayer.fun" { return None; }
        let raw = wrapper.query()?.split('&').find_map(|pair| pair.strip_prefix("url="));
        let source = if let Some(source) = raw.and_then(hls_url) {
            source
        } else {
            let (_, source) = wrapper.query_pairs().find(|(key, _)| key == "url")?;
            hls_url(&source)?
        };
        let title = link_title(&wrapper).or_else(|| link_title(&source));
        (source, title)
    };
    source.set_fragment(None);
    Some((PathBuf::from(source.as_str()), title))
}

pub enum Opened {
    File(PathBuf),
    Video { path: PathBuf, hls: bool },
    Webpage { page: String, hls: Vec<String> },
}

pub fn start(url: String, ctx: eframe::egui::Context) -> mpsc::Receiver<Result<Opened, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(download(&url));
        ctx.request_repaint();
    });
    receiver
}

fn download(input: &str) -> Result<Opened, String> {
    let tr = crate::localization::tr;
    let url = reqwest::Url::parse(input).map_err(|error| format!("{}: {error}", tr("URL 格式无效")))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(tr("仅支持 HTTP / HTTPS 文件直链").into());
    }
    let client = reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300)).build().map_err(|error| format!("{}: {error}", tr("网络加载失败")))?;
    let mut response = client.get(url).send().and_then(|response| response.error_for_status())
        .map_err(|error| format!("{}: {error}", tr("网络加载失败")))?;
    let mime = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok())
        .unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    let url = response.url().as_str();
    let hls = hls_response(url, &mime);
    if hls || mime.starts_with("video/") || direct_video(url).is_some() {
        return Ok(Opened::Video { path: PathBuf::from(url), hls });
    }
    if response.content_length().is_some_and(|size| size > LIMIT) { return Err(tr("文件超过 1 GiB，无法通过 URL 打开").into()); }
    if matches!(mime.as_str(), "text/html" | "application/xhtml+xml") {
        let page = url.to_owned();
        let mut bytes = Vec::new();
        response.take(3_000_001).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
        if bytes.len() > 3_000_000 { bytes.truncate(3_000_000); }
        let text = String::from_utf8_lossy(&bytes);
        let hls = hls_scan::scan(&text, &page);
        return Ok(Opened::Webpage { page, hls });
    }
    let candidate = response.url().path_segments().and_then(|mut parts| parts.next_back()).unwrap_or("download");
    let mut name:String=candidate.chars().map(|ch|if ch.is_control() || "<>:\"/\\|?*".contains(ch){'_'}else{ch}).take(160).collect();
    if name.is_empty(){name="download".into();}
    let mut path = PathBuf::from(name);
    if !supported(&path) {
        let extension = match mime.as_str() {
            "application/pdf" => "pdf", "image/gif" => "gif", "image/svg+xml" => "svg", "image/png" => "png", "image/jpeg" => "jpg", "image/webp" => "webp", "image/bmp" => "bmp",
            "audio/mpeg" => "mp3", "audio/wav" | "audio/x-wav" => "wav", "audio/flac" => "flac",
            "audio/ogg" => "ogg", "audio/mp4" => "m4a",
            "text/plain" => "txt",
            _ => return Err(tr("无法识别受支持的文件类型，请使用带文件扩展名的直链").into()),
        };
        path.set_extension(extension);
    }
    let _cache_guard = crate::app_cache::mutation_guard();
    let root = cache_root().join(format!("{}-{}", std::process::id(), std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", tr("无法保存网络文件")))?;
    path = root.join(path);
    let partial = root.join("download.part");
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::File::create(&partial).map_err(|error| error.to_string())?;
        let mut total = 0_u64;
        let mut buffer = [0_u8; 65536];
        loop {
            let count = response.read(&mut buffer).map_err(|error| format!("{}: {error}", tr("网络加载失败")))?;
            if count == 0 { break; }
            total += count as u64;
            if total > LIMIT { return Err(tr("文件超过 1 GiB，无法通过 URL 打开").into()); }
            file.write_all(&buffer[..count]).map_err(|error| format!("{}: {error}", tr("无法保存网络文件")))?;
        }
        if total == 0 { return Err(tr("服务器返回空文件").into()); }
        drop(file);
        std::fs::rename(&partial, &path).map_err(|error| error.to_string())?;
        Ok(())
    })();
    if let Err(error) = result { let _ = std::fs::remove_file(&partial); let _ = std::fs::remove_dir(&root); return Err(error); }
    if crate::playlist::is_playlist(&path){if let Some(key)=origin_key(input){let _=std::fs::write(root.join("playlist-origin.key"),key);}}
    Ok(Opened::File(path))
}
fn supported(path: &Path) -> bool {
    crate::playlist::is_playlist(path) || crate::media_preview::is_media(path) || crate::asset_viewer::is_image(path) || crate::asset_viewer::is_office_document(path) || crate::asset_viewer::is_text(path)
        || crate::model_preview::is_model(path)
        || matches!(path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "unity" | "prefab" | "asset" | "controller" | "anim" | "mat")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_playlist_origin_ignores_cache_paths_and_fragments_but_keeps_query_values(){
        assert_eq!(origin_key("https://EXAMPLE.org/list.m3u#one"),origin_key("https://example.org/list.m3u#two"));
        assert_ne!(origin_key("https://example.org/list.m3u?key=A"),origin_key("https://example.org/list.m3u?key=a"));
        assert_eq!(origin_key("https://example.org/list.m3u").unwrap().len(),64);
    }

    #[test]
    fn cached_names_decode_once_without_decoding_playback_urls(){
        assert_eq!(download_name("%E5%9B%BD%E5%86%85%E7%94%B5%E8%A7%86202509.m3u"),"国内电视202509.m3u");
        assert_eq!(download_name("a+b%2520c.m3u"),"a+b%20c.m3u");
        assert_eq!(download_name("%2e%2e%2Fchannels.m3u"),".._channels.m3u");
        assert_eq!(download_name("%2e%2e"),"download");
        assert_eq!(download_name("bad%FF.m3u"),"bad%FF.m3u");
        assert_eq!(file_label(Path::new("local%20name.m3u")),"local%20name.m3u");
    }

    #[test]
    fn recognizes_m3u_as_a_playlist_instead_of_plain_text(){
        assert!(supported(Path::new("%E5%9B%BD%E5%86%85%E7%94%B5%E8%A7%86.m3u")));
        assert!(supported(Path::new("channels.M3U")));
        assert!(supported(Path::new("channels.m3u8")));
        for mime in ["text/plain", "audio/x-mpegurl", "application/vnd.apple.mpegurl"] {
            assert!(!hls_response("https://example.org/channels.M3U?token=a%2Fb",mime));
        }
        assert!(hls_response("https://example.org/live.m3u8?token=a%2Fb","text/plain"));
        assert!(hls_response("https://example.org/live","application/vnd.apple.mpegurl"));
    }

    #[test]
    fn pasted_unicode_preserves_url_encoding_and_only_decodes_once() {
        assert_eq!(decode_pasted_unicode(r"https://example.org/?a=1\u0026b=%26%2B%3D"),
            "https://example.org/?a=1&b=%26%2B%3D");
        assert_eq!(decode_pasted_unicode(r"\u4e2d\u6587\uD83D\uDE00"), "中文😀");
        for unchanged in [r"\uD800", r"\uDC00", r"\u12", r"\uZZZZ", r"\u000a", r"\\u0026", "%26", "&amp;", "普通文字"] {
            assert_eq!(decode_pasted_unicode(unchanged), unchanged);
        }
        assert_eq!(decode_pasted_unicode(r"\u005cu0026"), r"\u0026");
    }

    #[test]
    fn recognizes_hls_and_preserves_signed_source() {
        let source = "https://example.org/path/index.M3U8?sign=a+b%2Fc";
        let (direct, title) = stream_source(source).unwrap();
        assert_eq!(title, None);
        assert_eq!(direct.to_str(), Some(source));
        assert_eq!(stream_label(&direct), Some("index.M3U8"));
        assert_eq!(stream_source(&format!("https://web.zyplayer.fun/?url={source}&name=吞噬星空 第241集")),
            Some((direct.clone(), Some("吞噬星空 第241集".into()))));

        let mut wrapper = reqwest::Url::parse("https://web.zyplayer.fun/").unwrap();
        wrapper.query_pairs_mut().append_pair("url", source).append_pair("name", "吞噬星空 第241集");
        assert_eq!(stream_source(wrapper.as_str()), Some((direct, Some("吞噬星空 第241集".into()))));
        let named = format!("{source}&name=Episode+241");
        assert_eq!(stream_source(&named), Some((PathBuf::from(named.as_str()), Some("Episode 241".into()))));
        assert!(stream_source("https://example.org/?url=https://example.org/index.m3u8").is_none());
        assert!(stream_source("https://web.zyplayer.fun/?url=file:///index.m3u8").is_none());
    }
}
