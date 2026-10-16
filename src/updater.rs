//! GitHub release checks and staged Windows updates. No installed files are touched until confirmation.
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::Duration};
use eframe::egui;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const RELEASE_API: &str = "https://api.github.com/repos/EkyoZZ/UnityYAMLViewer/releases/latest";
const MAX_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Clone, Deserialize)]
pub struct Asset { pub name: String, browser_download_url: String, size: u64, digest: Option<String> }
#[derive(Clone, Deserialize)]
pub struct Release { pub tag_name: String, pub body: Option<String>, assets: Vec<Asset> }
impl Release {
    pub fn page_url(&self) -> String {
        let mut url = reqwest::Url::parse("https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/").unwrap();
        url.path_segments_mut().unwrap().pop_if_empty().push(&self.tag_name);
        url.into()
    }
    pub fn asset(&self) -> Option<&Asset> {
        let architecture = if cfg!(target_arch = "aarch64") { "arm64" } else if cfg!(target_arch = "x86") { "x86" } else { "x64" };
        let named_executable = format!("unityyamlviewer-{}-windows-{architecture}.exe", self.tag_name.to_ascii_lowercase());
        // Match the release naming convention and never select another architecture.
        self.assets.iter().find(|a| a.name.eq_ignore_ascii_case("unity_yaml_viewer.exe"))
            .or_else(|| self.assets.iter().find(|a| a.name.eq_ignore_ascii_case(&named_executable)))
            .or_else(|| self.assets.iter().find(|a| {
                let name = a.name.to_ascii_lowercase();
                name.ends_with(".zip") && (name.contains("unityyamlviewer") || name.contains("unity_yaml_viewer"))
                    && name.contains("windows") && name.contains(architecture)
            }))
    }
}
enum Event { Checked(Result<Option<Release>, String>), Progress(u64, u64), Downloaded(Result<PathBuf, String>) }
#[derive(Default)]
pub struct Updater {
    pub notes: crate::release_notes::ReleaseNotes,
    receiver: Option<mpsc::Receiver<Event>>,
    pub release: Option<Release>,
    pub show: bool,
    pub checking: bool,
    pub downloading: bool,
    pub progress: (u64, u64),
    pub staged: Option<PathBuf>,
    pub message: Option<String>,
    pub check_message: Option<String>,
    pub check_failed: bool,
    pub confirm: bool,
    manual: bool,
    cancel: Arc<AtomicBool>,
}
fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder().user_agent("UnityYAMLViewer updater")
        .connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(300)).build().map_err(|e| e.to_string())
}
fn check() -> Result<Option<Release>, String> {
    // Respect server backoff even when the user clicks Check again.
    static RETRY_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
    let rate_limit_message = |seconds: u64| format!("GitHub 更新接口已限流，请在 {seconds} 秒后重试，或打开发布页面手动更新。");
    {
        let retry_at = RETRY_AT.lock().map_err(|e| e.to_string())?;
        if let Some(remaining) = retry_at.and_then(|at| at.checked_duration_since(std::time::Instant::now())) {
            return Err(rate_limit_message(remaining.as_secs().saturating_add(1)));
        }
    }
    let response = client()?.get(RELEASE_API).header("Accept", "application/vnd.github+json").send().map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND { return Err("尚未发布可用的更新版本。".into()); }
    if matches!(response.status(), reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::TOO_MANY_REQUESTS) {
        let status = response.status();
        let header_number = |name| response.headers().get(name).and_then(|value| value.to_str().ok()).and_then(|value| value.parse::<u64>().ok());
        let retry_after = header_number("retry-after");
        let exhausted = header_number("x-ratelimit-remaining") == Some(0);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let reset_wait = if exhausted { header_number("x-ratelimit-reset").map(|reset| reset.saturating_sub(now)) } else { None };
        let body = response.text().map_err(|e| e.to_string())?;
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS || exhausted || retry_after.is_some() || body.to_ascii_lowercase().contains("rate limit") {
            let seconds = retry_after.unwrap_or(0).max(reset_wait.unwrap_or(0)).max(60);
            *RETRY_AT.lock().map_err(|e| e.to_string())? = std::time::Instant::now().checked_add(Duration::from_secs(seconds));
            return Err(rate_limit_message(seconds));
        }
        return Err("GitHub 拒绝访问更新接口（HTTP 403），请检查网络或打开发布页面手动更新。".into());
    }
    let release: Release = response.error_for_status().map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())?;
    let latest = semver::Version::parse(release.tag_name.trim().trim_start_matches(['v', 'V'])).map_err(|e| e.to_string())?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    Ok((latest > current && latest.pre.is_empty()).then_some(release))
}
impl Updater {
    pub fn check(&mut self, ctx: &egui::Context, manual: bool) {
        if self.checking || self.downloading { return; }
        self.checking = true;
        self.manual = manual;
        self.check_message = None;
        self.check_failed = false;
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let ctx = ctx.clone();
        std::thread::spawn(move || { let _ = sender.send(Event::Checked(check())); ctx.request_repaint(); });
    }
    pub fn poll(&mut self) {
        let events: Vec<_> = self.receiver.as_ref().map(|r| r.try_iter().collect()).unwrap_or_default();
        for event in events {
            match event {
                Event::Checked(result) => {
                    self.checking = false;
                    match result {
                        Ok(release) => {
                            if self.release.as_ref().map(|r| &r.tag_name) != release.as_ref().map(|r| &r.tag_name) {
                                self.notes = Default::default();
                                self.staged = None;
                                self.confirm = false;
                            }
                            self.release = release;
                            if self.release.is_none() {
                                self.check_message = Some("目前已是最新版本。".into());
                                self.show = false;
                            } else if self.manual {
                                self.show = true;
                            }
                        }
                        Err(error) => {
                            self.check_failed = true;
                            self.check_message = Some(format!("检查更新失败：{error}"));
                        },
                    }
                }
                Event::Progress(done, total) => self.progress = (done, total),
                Event::Downloaded(result) => {
                    self.downloading = false;
                    match result { Ok(path) => { self.staged = Some(path); self.confirm = true; }, Err(error) => self.message = Some(format!("下载更新失败：{error}")) }
                }
            }
        }
    }
    pub fn download(&mut self, ctx: &egui::Context) {
        let Some(asset) = self.release.as_ref().and_then(Release::asset).cloned() else { return };
        if self.downloading { return; }
        self.downloading = true;
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        self.progress = (0, asset.size);
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = download(&asset, &sender, &ctx, &cancel);
            let _ = sender.send(Event::Downloaded(result));
            ctx.request_repaint();
        });
    }
    pub fn cancel_download(&self) { self.cancel.store(true, Ordering::Relaxed); }
}
fn download(asset: &Asset, sender: &mpsc::Sender<Event>, ctx: &egui::Context, cancel: &AtomicBool) -> Result<PathBuf, String> {
    let _cache_guard = crate::app_cache::mutation_guard();
    if asset.size == 0 || asset.size > MAX_SIZE { return Err("更新文件大小不符合要求。".into()); }
    let expected_digest = asset.digest.as_deref().filter(|d| d.starts_with("sha256:") && d.len() == 71)
        .ok_or("发行附件缺少 SHA-256 校验值，无法安全地自动更新。")?;
    if !asset.browser_download_url.starts_with("https://github.com/EkyoZZ/UnityYAMLViewer/releases/download/") {
        return Err("更新下载地址不属于本项目。".into());
    }
    let base = crate::app_cache::root().map_or_else(std::env::temp_dir, |path| path.join("updates"));
    let directory = base.join(format!("UnityYAMLViewer-update-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let result = (|| {
        let package = directory.join("package");
        let mut file = fs::File::create(&package).map_err(|e| e.to_string())?;
        let mut response = client()?.get(&asset.browser_download_url).send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut done = 0;
        let mut buffer = [0u8; 65536];
        loop {
            if cancel.load(Ordering::Relaxed) { return Err("下载已取消。".into()); }
            let count = response.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 { break; }
            done += count as u64;
            if done > asset.size { return Err("下载大小超过发行附件大小。".into()); }
            file.write_all(&buffer[..count]).map_err(|e| e.to_string())?;
            hash.update(&buffer[..count]);
            let _ = sender.send(Event::Progress(done, asset.size));
            ctx.request_repaint();
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if done != asset.size { return Err("下载不完整，请重试。".into()); }
        let actual = format!("sha256:{:x}", hash.finalize());
        if !expected_digest.eq_ignore_ascii_case(&actual) { return Err("SHA-256 校验失败，更新文件可能损坏。".into()); }
        let executable = directory.join("update.exe");
        if asset.name.to_ascii_lowercase().ends_with(".zip") {
            let mut archive = zip::ZipArchive::new(fs::File::open(&package).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let expected = format!("{}.exe", asset.name.trim_end_matches(".zip"));
            let indices: Vec<_> = (0..archive.len()).filter(|&i| archive.by_index(i).ok().is_some_and(|f| {
                f.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("unity_yaml_viewer.exe") || n.to_string_lossy().eq_ignore_ascii_case(&expected))).unwrap_or(false)
            })).collect();
            if indices.len() != 1 { return Err("压缩包中必须有且仅有一个 unity_yaml_viewer.exe。".into()); }
            let mut entry = archive.by_index(indices[0]).map_err(|e| e.to_string())?;
            if entry.size() > MAX_SIZE { return Err("解压后的程序过大。".into()); }
            let mut output = fs::File::create(&executable).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
        } else { fs::rename(&package, &executable).map_err(|e| e.to_string())?; }
        validate_executable(&executable)?;
        if cancel.load(Ordering::Relaxed) { return Err("下载已取消。".into()); }
        Ok(executable)
    })();
    if result.is_err() { let _ = fs::remove_dir_all(&directory); }
    result
}
fn validate_executable(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let invalid = || "下载文件不是兼容的 Windows 程序。".to_string();
    if bytes.len() < 64 || &bytes[..2] != b"MZ" { return Err(invalid()); }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    let header = bytes.get(offset..offset.saturating_add(24)).ok_or_else(invalid)?;
    if &header[..4] != b"PE\0\0" { return Err(invalid()); }
    let machine = u16::from_le_bytes([header[4], header[5]]);
    let expected = if cfg!(target_arch = "aarch64") { 0xaa64 } else if cfg!(target_arch = "x86") { 0x14c } else { 0x8664 };
    if machine != expected { return Err(invalid()); }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install(staged: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let target = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = staged.parent().ok_or("更新目录不存在")?;
    // Literal PowerShell strings protect paths containing spaces, quotes and shell metacharacters.
    let quote = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "''"));
    let script = directory.join("install.ps1");
    let body = format!(r#"$ErrorActionPreference = 'Stop'
$target = {target}
$staged = {staged}
$backup = {backup}
$directory = {directory}
try {{
    $process = Get-Process -Id {pid} -ErrorAction SilentlyContinue
    if ($process) {{ if (-not $process.WaitForExit(60000)) {{ throw '等待程序退出超时，未替换更新。' }} }}
    Copy-Item -LiteralPath $target -Destination $backup
    $installed = $false
    for ($attempt = 0; $attempt -lt 20; $attempt++) {{
        try {{ Copy-Item -LiteralPath $staged -Destination $target -Force; $installed = $true; break }}
        catch {{ Start-Sleep -Milliseconds 500 }}
    }}
    if (-not $installed) {{ throw '无法替换程序。请检查目录写入权限或程序占用。' }}
    try {{ Start-Process -FilePath $target -WorkingDirectory (Split-Path -LiteralPath $target) }}
    catch {{ Copy-Item -LiteralPath $backup -Destination $target -Force; throw }}
}} catch {{
    if (Test-Path -LiteralPath $backup) {{
        try {{ Copy-Item -LiteralPath $backup -Destination $target -Force }} catch {{}}
    }}
    $_ | Out-String | Set-Content -LiteralPath (Join-Path $directory 'error.txt') -Encoding UTF8
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show("更新失败，旧程序备份保存在：$backup`n错误信息：$_", 'Unity YAML Viewer') | Out-Null
    exit 1
}}
Remove-Item -LiteralPath $directory -Recurse -Force -ErrorAction SilentlyContinue
"#, target=quote(&target), staged=quote(staged), backup=quote(&directory.join("previous.exe")), directory=quote(directory), pid=std::process::id());
    // Windows PowerShell 5 reads UTF-8 scripts reliably with a BOM.
    fs::write(&script, format!("\u{feff}{body}")).map_err(|e| e.to_string())?;
    std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(&script).creation_flags(0x08000000).spawn().map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn install(_: &Path) -> Result<(), String> { Err("自动替换更新仅支持 Windows。".into()) }
