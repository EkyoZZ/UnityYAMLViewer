//! Direct HTTP(S) files are downloaded off-thread, then opened by existing viewers.
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::mpsc, time::Duration};

const LIMIT: u64 = 1024 * 1024 * 1024;
fn cache_root() -> PathBuf { crate::app_cache::network_root() }
pub fn is_cached(path: &Path) -> bool { path.starts_with(cache_root()) || path.starts_with(std::env::temp_dir().join("UnityYAMLViewer-network")) }
pub fn start(url: String, ctx: eframe::egui::Context) -> mpsc::Receiver<Result<PathBuf, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || { let _ = sender.send(download(&url)); ctx.request_repaint(); });
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
            "image/png" => "png", "image/jpeg" => "jpg", "image/webp" => "webp", "image/bmp" => "bmp",
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
    crate::media_preview::is_media(path) || crate::asset_viewer::is_image(path) || crate::asset_viewer::is_text(path)
        || crate::model_preview::is_model(path)
        || matches!(path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "unity" | "prefab" | "asset" | "controller" | "anim" | "mat")
}
