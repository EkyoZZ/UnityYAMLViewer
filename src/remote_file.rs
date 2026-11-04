//! Direct HTTP(S) files are downloaded off-thread, then opened by existing viewers.
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::mpsc, time::Duration};

mod hls_download;
mod hls_cache;
pub(crate) use hls_cache::Cache as HlsCache;
pub(crate) use hls_download::Download;

const LIMIT: u64 = 1024 * 1024 * 1024;
fn cache_root() -> PathBuf { crate::app_cache::network_root() }
pub fn is_cached(path: &Path) -> bool { path.starts_with(cache_root()) || path.starts_with(std::env::temp_dir().join("UnityYAMLViewer-network")) }
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

pub fn start(url: String, ctx: eframe::egui::Context) -> mpsc::Receiver<Result<PathBuf, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(download(&url));
        ctx.request_repaint();
    });
    receiver
}

fn download(input: &str) -> Result<PathBuf, String> {
    let tr = crate::localization::tr;
    let url = reqwest::Url::parse(input).map_err(|error| format!("{}: {error}", tr("URL 格式无效")))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(tr("仅支持 HTTP / HTTPS 文件直链").into());
    }
    let client = reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300)).build().map_err(|error| format!("{}: {error}", tr("网络加载失败")))?;
    let mut response = client.get(url).send().and_then(|response| response.error_for_status())
        .map_err(|error| format!("{}: {error}", tr("网络加载失败")))?;
    if response.content_length().is_some_and(|size| size > LIMIT) { return Err(tr("文件超过 1 GiB，无法通过 URL 打开").into()); }
    let mime = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok())
        .unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if matches!(mime.as_str(), "text/html" | "application/xhtml+xml") {
        return Err(tr("此 URL 返回网页，请提供文件直链；暂不支持网页解析或登录认证").into());
    }
    let candidate = response.url().path_segments().and_then(|mut parts| parts.next_back()).unwrap_or("download");
    let mut name: String = candidate.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).take(160).collect();
    if name.is_empty() { name = "download".into(); }
    let mut path = PathBuf::from(&name);
    if !supported(&path) {
        let extension = match mime.as_str() {
            "application/pdf" => "pdf", "image/gif" => "gif", "image/svg+xml" => "svg", "image/png" => "png", "image/jpeg" => "jpg", "image/webp" => "webp", "image/bmp" => "bmp",
            "audio/mpeg" => "mp3", "audio/wav" | "audio/x-wav" => "wav", "audio/flac" => "flac",
            "audio/ogg" => "ogg", "audio/mp4" => "m4a", "video/mp4" => "mp4", "video/webm" => "webm",
            "video/x-matroska" => "mkv", "text/plain" => "txt",
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
    Ok(path)
}
fn supported(path: &Path) -> bool {
    crate::media_preview::is_media(path) || crate::asset_viewer::is_image(path) || crate::asset_viewer::is_office_document(path) || crate::asset_viewer::is_text(path)
        || crate::model_preview::is_model(path)
        || matches!(path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "unity" | "prefab" | "asset" | "controller" | "anim" | "mat")
}

#[cfg(test)]
mod tests {
    use super::*;

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
