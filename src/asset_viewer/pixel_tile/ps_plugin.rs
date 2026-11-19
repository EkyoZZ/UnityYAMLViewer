//! Independent, explicitly requested Photoshop plugin release discovery.
use crate::localization::tr;
use eframe::egui;
use serde::Deserialize;
use std::{io::Read, sync::mpsc, time::{Duration, Instant}};

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/EkyoZZ/UnityYAMLViewer/master/updates/ps-tile.json";
const DOWNLOADS: &str = "https://github.com/EkyoZZ/UnityYAMLViewer/releases";
const PLUGIN_ID: &str = "unityyamlviewer.isometric-preview";

#[derive(Debug, Deserialize)]
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
fn deployed_version() -> Result<Option<semver::Version>, String> {
    let Some(root) = std::env::var_os("APPDATA") else { return Err("APPDATA unavailable".into()); };
    let path = std::path::PathBuf::from(root).join("Adobe/UXP/Plugins/External")
        .join(PLUGIN_ID).join("manifest.json");
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 65536 { return Err("Local plugin manifest exceeds 64 KiB".into()); }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["id"].as_str() != Some(PLUGIN_ID) { return Err("Unexpected plugin ID".into()); }
    semver::Version::parse(value["version"].as_str().unwrap_or("")).map(Some).map_err(|e| e.to_string())
}

#[derive(Default)]
pub(super) struct State {
    receiver: Option<mpsc::Receiver<Result<Manifest, String>>>,
    result: Option<Result<Manifest, String>>,
    last_check: Option<Instant>,
    local: Option<Result<Option<semver::Version>, String>>,
}
impl State {
    pub(super) fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => { self.result = Some(result); self.receiver = None; }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.result = Some(Err("Plugin query worker disconnected".into()));
                    self.receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        egui::CollapsingHeader::new(tr("Photoshop 插件")).id_salt("ps_plugin_install").show(ui, |ui| {
            let local = self.local.get_or_insert_with(deployed_version);
            match local {
                Ok(Some(version)) => { ui.label(format!("{} {version}", tr("已部署版本："))); }
                Ok(None) => { ui.label(tr("未检测到已部署的 PS 插件")); }
                Err(error) => { ui.label(tr("无法读取已部署版本")).on_hover_text(error.as_str()); }
            }
            ui.small(tr("仅检测当前用户部署目录，不代表 Photoshop 已加载插件。"));
            let cooling = self.last_check.is_some_and(|t| t.elapsed() < Duration::from_secs(60));
            if ui.add_enabled(self.receiver.is_none() && !cooling,
                egui::Button::new(tr("检查 PS 插件更新"))).clicked() {
                self.local = Some(deployed_version());
                self.last_check = Some(Instant::now());
                self.result = None;
                let (tx, rx) = mpsc::channel();
                self.receiver = Some(rx);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || { let _ = tx.send(query()); ctx.request_repaint(); });
            }
            if self.receiver.is_some() { ui.spinner(); }
            if cooling {
                ui.small(tr("两次查询至少间隔 60 秒"));
                ui.ctx().request_repaint_after(Duration::from_secs(1));
            }
            match &self.result {
                Some(Ok(manifest)) if manifest.published => {
                    ui.label(format!("{} {}", tr("最新插件版本："), manifest.version));
                    ui.label(format!("Photoshop ≥ {}", manifest.minimum_photoshop_version));
                    let latest = semver::Version::parse(&manifest.version).expect("validated manifest");
                    let label = match self.local.as_ref() {
                        Some(Ok(Some(local))) if *local < latest => tr("更新 PS 插件"),
                        Some(Ok(Some(local))) if *local == latest => tr("已是最新版，打开下载页"),
                        Some(Ok(Some(_))) => tr("已部署版本较新，打开下载页"),
                        Some(Ok(None)) => tr("安装 PS 插件"),
                        _ => tr("打开插件下载页"),
                    };
                    ui.hyperlink_to(label, manifest.release_url.as_deref().unwrap());
                }
                Some(Ok(_)) => { ui.label(tr("PS 插件尚未正式发布")); }
                Some(Err(error)) => { ui.colored_label(ui.visuals().error_fg_color,
                    tr("插件更新查询失败，可稍后重试或打开下载页。")).on_hover_text(error); }
                None => {}
            }
            ui.hyperlink_to(tr("打开插件下载页"), DOWNLOADS);
            ui.small(tr("关闭 Photoshop，运行安装器并点击安装／更新，然后重新启动 Photoshop。"));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_contract_rejects_mismatched_and_external_links() {
        let mut manifest: Manifest = serde_json::from_str(include_str!("../../../updates/ps-tile.json")).unwrap();
        assert!(manifest.validate().is_ok());
        manifest.published = true;
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
