//! Independent, explicitly requested Photoshop plugin release discovery.
use crate::localization::tr;
use eframe::egui;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf};
use std::{io::Read, sync::mpsc, time::{Duration, Instant}};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/EkyoZZ/UnityYAMLViewer/master/updates/ps-tile.json";
const DOWNLOADS: &str = "https://github.com/EkyoZZ/UnityYAMLViewer/releases";
const MAX_INSTALLER: u64 = 64 * 1024 * 1024;
static DOWNLOAD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    published: bool,
    version: String,
    release_url: Option<String>,
    asset_name: String,
    minimum_photoshop_version: String,
}
impl Manifest {
    fn validate(&self) -> Result<semver::Version, String> {
        let version = semver::Version::parse(&self.version).map_err(|e| e.to_string())?;
        if self.schema_version != 1 || !version.pre.is_empty()
            || self.asset_name != format!("PS-Tile-Installer-v{}-windows-x64.exe", self.version)
            || semver::Version::parse(&self.minimum_photoshop_version).is_err()
            || (self.published && self.release_url.as_deref() != Some(
                format!("https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/ps-v{}", self.version).as_str()))
        {
            return Err("Invalid plugin update manifest".into());
        }
        Ok(version)
    }
}
fn query() -> Result<Manifest, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("UnityYAMLViewer-PS-Plugin")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build().map_err(|e| e.to_string())?;
    let response = client.get(MANIFEST_URL).send().map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    response.take(65537).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 65536 { return Err("Plugin update manifest exceeds 64 KiB".into()); }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate()?;
    Ok(manifest)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Downloaded { version: String, size: u64, sha256: String }
fn cache_root() -> Result<PathBuf, String> {
    crate::app_cache::root().map(|p| p.join("ps_plugin_installers")).ok_or_else(|| "Cannot locate plugin download directory".into())
}
impl Downloaded {
    fn path(&self) -> Result<PathBuf, String> {
        let v = semver::Version::parse(&self.version).map_err(|e| e.to_string())?;
        if v.to_string() != self.version { return Err("Invalid cached version".into()); }
        Ok(cache_root()?.join(format!("PS-Tile-Installer-v{}-windows-x64.exe", self.version)))
    }
    fn verify(&self) -> Result<PathBuf, String> {
        if self.size == 0 || self.size > MAX_INSTALLER { return Err("Invalid installer size".into()); }
        let path = self.path()?;
        let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() != self.size { return Err("Installer size mismatch".into()); }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop { let n = file.read(&mut buffer).map_err(|e| e.to_string())?; if n == 0 { break; } hash.update(&buffer[..n]); }
        if format!("{:x}", hash.finalize()) != self.sha256 { return Err("Installer SHA-256 mismatch".into()); }
        Ok(path)
    }
}
fn downloaded() -> Option<Downloaded> {
    let mut bytes = Vec::new();
    fs::File::open(cache_root().ok()?.join("downloaded.json")).ok()?
        .take(65537).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 65536 { return None; }
    let local: Downloaded = serde_json::from_slice(&bytes).ok()?;
    local.verify().ok()?;
    Some(local)
}
fn needs_download(local: Option<&Downloaded>, latest: &semver::Version) -> bool {
    local.and_then(|l| semver::Version::parse(&l.version).ok())
        .is_none_or(|version| version < *latest)
}
#[derive(Deserialize)]
struct Asset { name: String, browser_download_url: String, size: u64, digest: Option<String> }
#[derive(Deserialize)]
struct Release { tag_name: String, draft: bool, assets: Vec<Asset> }
enum Event {
    Progress(u64, u64),
    Detail(&'static str, String),
    Done(Result<Downloaded, String>),
}
fn trace(sender: &mpsc::Sender<Event>, ctx: &egui::Context, stage: &'static str, text: impl Into<String>) {
    let _ = sender.send(Event::Detail(stage, text.into()));
    ctx.request_repaint();
}
fn request(client: &reqwest::blocking::Client, url: &str, sender: &mpsc::Sender<Event>, ctx: &egui::Context,
    stage: &'static str) -> Result<reqwest::blocking::Response, String> {
    trace(sender, ctx, stage, format!("GET {url}"));
    let response = client.get(url).send().map_err(|e| format!("{e:?}"))?;
    let mut info = format!("HTTP {} · {}", response.status(), response.url().host_str().unwrap_or(""));
    for name in ["x-ratelimit-remaining", "x-ratelimit-reset", "retry-after", "content-length"] {
        if let Some(value) = response.headers().get(name).and_then(|h| h.to_str().ok()) {
            info.push_str(&format!("\n{name}: {value}"));
        }
    }
    trace(sender, ctx, stage, info);
    response.error_for_status().map_err(|e| format!("{e:?}"))
}
fn download(manifest: &Manifest, sender: &mpsc::Sender<Event>, ctx: &egui::Context) -> Result<Downloaded, String> {
    let mut stage = "准备下载";
    trace(sender, ctx, stage, format!("{} · {}", manifest.version, manifest.asset_name));
    let result = download_inner(manifest, sender, ctx, &mut stage);
    if let Err(error) = &result { trace(sender, ctx, stage, error.clone()); }
    result.map_err(|error| format!("{}: {error}", tr(stage)))
}
fn download_inner(manifest: &Manifest, sender: &mpsc::Sender<Event>, ctx: &egui::Context, stage: &mut &'static str) -> Result<Downloaded, String> {
    let _lock = DOWNLOAD_LOCK.try_lock().map_err(|_| "Another plugin download is in progress".to_owned())?;
    manifest.validate()?;
    if !manifest.published { return Err("Plugin is not published".into()); }
    let client = reqwest::blocking::Client::builder().user_agent("UnityYAMLViewer-PS-Plugin")
        .connect_timeout(Duration::from_secs(10)).timeout(Duration::from_secs(180)).build().map_err(|e| e.to_string())?;
    // Fetch the exact release only after the user requests a download, never while checking versions.
    *stage = "获取附件信息";
    let api_url = format!("https://api.github.com/repos/EkyoZZ/UnityYAMLViewer/releases/tags/ps-v{}", manifest.version);
    let response = request(&client, &api_url, sender, ctx, *stage)?;
    let mut json = Vec::new(); response.take(1024 * 1024 + 1).read_to_end(&mut json).map_err(|e| e.to_string())?;
    if json.len() > 1024 * 1024 { return Err("Release metadata too large".into()); }
    *stage = "校验附件信息";
    trace(sender, ctx, *stage, format!("JSON: {} bytes", json.len()));
    let release: Release = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
    if release.draft || release.tag_name != format!("ps-v{}", manifest.version) { return Err("Release identity mismatch".into()); }
    let matches: Vec<_> = release.assets.iter().filter(|a| a.name == manifest.asset_name).collect();
    if matches.len() != 1 { return Err("Installer attachment missing or duplicated".into()); }
    let asset = matches[0];
    let expected_url = format!("https://github.com/EkyoZZ/UnityYAMLViewer/releases/download/ps-v{}/{}", manifest.version, manifest.asset_name);
    let digest = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| d.len() == 64 && d.bytes().all(|c| c.is_ascii_hexdigit())).ok_or("Missing attachment SHA-256")?;
    if asset.size == 0 || asset.size > MAX_INSTALLER || asset.browser_download_url != expected_url { return Err("Invalid installer attachment".into()); }
    trace(sender, ctx, *stage, format!("{}\n{} bytes\nSHA-256: {}", asset.name, asset.size, digest));
    *stage = "准备保存目录";
    let root = cache_root()?;
    trace(sender, ctx, *stage, root.display().to_string());
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let partial = root.join("installer.part");
    let result = (|| {
        *stage = "连接安装包下载地址";
        let mut response = request(&client, &expected_url, sender, ctx, *stage)?;
        *stage = "写入安装包";
        trace(sender, ctx, *stage, partial.display().to_string());
        let _ = sender.send(Event::Progress(0, asset.size));
        let mut file = fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new(); let mut done = 0; let mut buffer = [0u8; 65536];
        loop {
            let n = response.read(&mut buffer).map_err(|e| format!("{e:?}"))?; if n == 0 { break; }
            done += n as u64; if done > asset.size { return Err("Installer exceeds expected size".into()); }
            file.write_all(&buffer[..n]).map_err(|e| e.to_string())?; hash.update(&buffer[..n]);
            let _ = sender.send(Event::Progress(done, asset.size)); ctx.request_repaint();
        }
        file.sync_all().map_err(|e| e.to_string())?; drop(file);
        *stage = "校验安装包";
        let actual = format!("{:x}", hash.finalize());
        trace(sender, ctx, *stage, format!("Expected: {} bytes · SHA-256: {}\nActual: {} bytes · SHA-256: {}", asset.size, digest, done, actual));
        if done != asset.size || !actual.eq_ignore_ascii_case(digest) { return Err("Installer size or SHA-256 mismatch".into()); }
        let local = Downloaded { version: manifest.version.clone(), size: done, sha256: actual };
        *stage = "保存安装包与版本记录";
        let path = local.path()?;
        trace(sender, ctx, *stage, path.display().to_string());
        if path.exists() { fs::remove_file(&path).map_err(|e| e.to_string())?; }
        fs::rename(&partial, path).map_err(|e| e.to_string())?;
        let record = root.join("downloaded.json");
        let pending = root.join("downloaded.json.part");
        fs::write(&pending, serde_json::to_vec(&local).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        if record.exists() { fs::remove_file(&record).map_err(|e| e.to_string())?; }
        fs::rename(pending, record).map_err(|e| e.to_string())?;
        trace(sender, ctx, "下载完成", local.version.clone());
        Ok(local)
    })();
    if result.is_err() { let _ = fs::remove_file(partial); }
    result
}
fn open_installer(local: &Downloaded) -> Result<(), String> {
    let path = local.verify()?;
    std::process::Command::new(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}
fn show_directory(local: &Downloaded) -> Result<(), String> {
    let path = local.verify()?;
    std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display()))
        .spawn().map(|_| ()).map_err(|e| e.to_string())
}
#[derive(Default)]
pub(super) struct State {
    receiver: Option<mpsc::Receiver<Result<Manifest, String>>>,
    result: Option<Result<Manifest, String>>,
    last_check: Option<Instant>,
    local: Option<Downloaded>,
    loaded: bool,
    download_rx: Option<mpsc::Receiver<Event>>,
    progress: (u64, u64),
    message: Option<&'static str>,
    error: Option<String>,
    details: Vec<(&'static str, String)>,
    detail_generation: u64,
    stage: Option<&'static str>,
}
impl State {
    pub(super) fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => { self.result = Some(result); self.receiver = None; }
                Err(mpsc::TryRecvError::Disconnected) => { self.result = Some(Err("Plugin query worker disconnected".into())); self.receiver = None; }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let mut completed = None;
        if let Some(rx) = &self.download_rx {
            loop { match rx.try_recv() {
                Ok(Event::Progress(n, total)) => self.progress = (n, total),
                Ok(Event::Detail(stage, text)) => { self.stage = Some(stage); self.details.push((stage, text)); },
                Ok(Event::Done(result)) => { completed = Some(result); break; }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => { completed = Some(Err("Download worker disconnected".into())); break; }
            } }
        }
        if let Some(result) = completed {
            self.download_rx = None;
            match result {
                Ok(local) => {
                    self.details.push(("打开安装器", local.path().map(|p| p.display().to_string()).unwrap_or_default()));
                    self.stage = Some("打开安装器");
                    self.local = Some(local);
                    self.message = Some("安装包已下载，已请求打开安装器。");
                    if let Err(e) = open_installer(self.local.as_ref().unwrap()) { self.error = Some(e); self.message = Some("安装包已下载，请手动打开安装器。"); }
                }
                Err(e) => self.error = Some(e),
            }
        }
        egui::CollapsingHeader::new(tr("Photoshop 插件")).id_salt("ps_plugin_install").show(ui, |ui| {
            if !self.loaded { self.local = downloaded(); self.loaded = true; }
            if let Some(local) = &self.local { ui.label(format!("{} {}", tr("已下载安装包版本："), local.version)); }
            else { ui.label(tr("未检测到有效的本地安装包")); }
            let cooling = self.last_check.is_some_and(|t| t.elapsed() < Duration::from_secs(60));
            let busy = self.receiver.is_some() || self.download_rx.is_some();
            if ui.add_enabled(!busy && !cooling, egui::Button::new(tr("检查 PS 插件更新"))).clicked() {
                self.local = downloaded(); self.last_check = Some(Instant::now()); self.result = None; self.message = None; self.error = None;
                let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ui.ctx().clone();
                std::thread::spawn(move || { let _ = tx.send(query()); ctx.request_repaint(); });
            }
            if self.receiver.is_some() { ui.spinner(); }
            if cooling { ui.small(tr("两次查询至少间隔 60 秒")); ui.ctx().request_repaint_after(Duration::from_secs(1)); }
            if let Some(Ok(manifest)) = &self.result {
                if manifest.published {
                    ui.label(format!("{} {}", tr("最新插件版本："), manifest.version));
                    ui.label(format!("Photoshop ≥ {}", manifest.minimum_photoshop_version));
                    let latest = semver::Version::parse(&manifest.version).expect("validated manifest");
                    if !needs_download(self.local.as_ref(), &latest) {
                        ui.label(tr("本地安装包已是最新版，无需下载"));
                    } else if ui.add_enabled(!busy, egui::Button::new(tr("下载并打开安装器"))).clicked() {
                        let manifest = manifest.clone(); let (tx, rx) = mpsc::channel(); self.download_rx = Some(rx);
                        self.progress = (0, 0); self.error = None; self.message = None;
                        self.details.clear(); self.stage = Some("准备下载"); self.detail_generation = self.detail_generation.wrapping_add(1);
                        let ctx = ui.ctx().clone();
                        std::thread::spawn(move || { let result = download(&manifest, &tx, &ctx); let _ = tx.send(Event::Done(result)); ctx.request_repaint(); });
                    }
                } else { ui.label(tr("PS 插件尚未正式发布")); }
            } else if let Some(Err(error)) = &self.result {
                ui.colored_label(ui.visuals().error_fg_color, tr("插件更新查询失败，可稍后重试或打开下载页。")).on_hover_text(error);
            }
            if self.download_rx.is_some() {
                if let Some(stage) = self.stage { ui.label(tr(stage)); }
                let (n, total) = self.progress;
                ui.add(egui::ProgressBar::new(if total == 0 { 0. } else { n as f32 / total as f32 }).show_percentage());
            }
            if let Some(local) = self.local.clone() {
                if let Ok(path) = local.path() { ui.label(tr("安装包位置：")); ui.add(egui::Label::new(path.display().to_string()).wrap()); }
                if ui.add_enabled(!busy, egui::Button::new(tr("打开安装器"))).clicked() {
                    if let Err(e) = open_installer(&local) { self.error = Some(e); self.local = downloaded(); }
                }
                if ui.add_enabled(!busy, egui::Button::new(tr("打开所在目录"))).clicked() {
                    if let Err(e) = show_directory(&local) { self.error = Some(e); self.local = downloaded(); }
                }
            }
            if let Some(message) = self.message { ui.small(tr(message)); }
            if let Some(error) = &self.error { ui.colored_label(ui.visuals().error_fg_color, tr("插件安装包操作失败，请重试或手动下载。")).on_hover_text(error); }
            if !self.details.is_empty() || self.error.is_some() {
                egui::CollapsingHeader::new(tr("下载详情"))
                    .id_salt(("ps_download_details", self.detail_generation)).default_open(false).show(ui, |ui| {
                        let mut text = self.details.iter().map(|(stage, text)| format!("[{}]\n{}", tr(stage), text))
                            .collect::<Vec<_>>().join("\n\n");
                        if let Some(error) = &self.error { text.push_str(&format!("\n\n[{}]\n{}", tr("操作失败"), error)); }
                        if ui.button(tr("复制详情")).clicked() { ui.ctx().copy_text(text.clone()); }
                        egui::ScrollArea::vertical().id_salt("ps_download_log_scroll").max_height(240.).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut text).font(egui::TextStyle::Monospace)
                                .desired_width(ui.available_width()).interactive(true));
                        });
                    });
            }
            let page = self.result.as_ref().and_then(|r| r.as_ref().ok()).and_then(|m| m.release_url.as_deref()).unwrap_or(DOWNLOADS);
            ui.hyperlink_to(tr("手动下载"), page);
            ui.small(tr("仅打开安装器，由你决定安装或更新；操作前请关闭 Photoshop。"));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_offer_uses_only_valid_downloaded_versions() {
        let latest = semver::Version::parse("0.1.1").unwrap();
        let mut local = Downloaded { version: "0.1.0".into(), size: 1, sha256: String::new() };
        assert!(needs_download(None, &latest));
        assert!(needs_download(Some(&local), &latest));
        local.version = "0.1.1".into();
        assert!(!needs_download(Some(&local), &latest));
        local.version = "0.1.2".into();
        assert!(!needs_download(Some(&local), &latest));
        local.version = "invalid".into();
        assert!(needs_download(Some(&local), &latest));
        local.version = "../../outside".into();
        assert!(local.path().is_err());
    }
    #[test]
    fn release_contract_rejects_mismatched_and_external_links() {
        let mut manifest: Manifest = serde_json::from_str(include_str!("../../../updates/ps-tile.json")).unwrap();
        assert!(manifest.validate().is_ok());
        manifest.published = true;
        manifest.release_url = None;
        assert!(manifest.validate().is_err());
        manifest.release_url = Some("https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/ps-v0.1.0".into());
        assert!(manifest.validate().is_ok());
        manifest.version = "0.1.1".into();
        assert!(manifest.validate().is_err());
        manifest.version = "0.1.0".into();
        manifest.release_url = Some("https://example.com/installer.exe".into());
        assert!(manifest.validate().is_err());
    }
}
