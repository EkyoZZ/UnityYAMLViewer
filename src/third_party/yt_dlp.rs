use std::{path::PathBuf, sync::mpsc, time::Duration};
use eframe::egui;
use crate::{component_paths, localization::tr};
use super::{component_header_actions, install_progress::Shared, installer};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action { Detect, Check, Install }

struct Installed { path: PathBuf, version: String, managed: bool }

#[derive(Default)]
pub(super) struct Settings {
    checked: bool,
    installed: Option<Installed>,
    latest: Option<String>,
    pending: Option<(Action, mpsc::Receiver<Result<(Option<Installed>, Option<String>), String>>)>,
    progress: Shared,
    picker: component_paths::Picker,
    error: Option<String>,
}

impl Settings {
    fn start(&mut self, ctx: &egui::Context, action: Action) {
        if self.pending.is_some() { return; }
        self.error = None;
        self.latest = None;
        self.progress = Shared::default();
        let progress = self.progress.clone();
        let ctx = ctx.clone();
        let (sender, receiver) = mpsc::channel();
        self.pending = Some((action, receiver));
        std::thread::spawn(move || {
            let result = (|| {
                if action == Action::Install { installer::install("yt-dlp", &progress)?; }
                let installed = crate::media_tools::find_tool("yt-dlp").map(|path| {
                    let version = component_paths::yt_dlp_version(&path)?;
                    let managed = component_paths::selected("yt-dlp").is_none()
                        && installer::tool_directories("yt-dlp").iter().any(|directory| path.parent() == Some(directory.as_path()));
                    Ok::<_, String>(Installed { path, version, managed })
                }).transpose()?;
                let latest = if action == Action::Check { Some(installer::latest_yt_dlp(&progress)?) } else { None };
                Ok((installed, latest))
            })();
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    pub(super) fn poll(&mut self) {
        let Some((_, pending)) = &self.pending else { return; };
        let result = match pending.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err(tr("组件任务中断").into()),
        };
        self.pending = None;
        self.checked = true;
        match result {
            Ok((installed, latest)) => { self.installed = installed; self.latest = latest; }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        if !self.checked && self.pending.is_none() { self.start(ui.ctx(), Action::Detect); }
        let busy = self.pending.is_some();
        let update = self.installed.as_ref().zip(self.latest.as_deref()).is_some_and(|(installed, latest)| {
            component_paths::yt_dlp_version_key(latest) > component_paths::yt_dlp_version_key(&installed.version)
        });
        let managed = self.installed.as_ref().is_some_and(|installed| installed.managed);
        let supported = cfg!(all(windows, target_arch = "x86_64"));
        let install = if self.checked && self.installed.is_none() && component_paths::selected("yt-dlp").is_none() {
            Some(("安装 yt-dlp", supported))
        } else if update && managed {
            Some(("更新 yt-dlp", supported))
        } else { None };
        let mut check = false;
        let (detect, install) = component_header_actions(ui, "yt-dlp", self.checked.then_some(self.installed.is_some()),
            busy, install, |ui| {
                check = ui.add_enabled(!busy && self.installed.is_some(), egui::Button::new(tr("检查更新"))).clicked();
            });
        if let Some(installed) = &self.installed {
            ui.label(format!("yt-dlp {}", installed.version));
            ui.add(egui::Label::new(installed.path.display().to_string()).wrap().selectable(true));
        }
        if let Some(latest) = self.latest.as_ref().filter(|_| self.installed.is_some()) {
            if update {
                ui.label(format!("{} {latest}", tr("可用新版本")));
                if !managed { ui.small(tr("手动安装的 yt-dlp 请通过原安装方式更新。")); }
            } else {
                ui.label(tr("目前已是最新版本。"));
            }
        }
        if let Some((action, _)) = &self.pending {
            if *action == Action::Install {
                self.progress.show_for(ui, "yt-dlp");
                if self.progress.can_cancel() && ui.button(tr("取消下载")).clicked() { self.progress.request_cancel(); }
            } else {
                ui.horizontal(|ui| { ui.spinner(); ui.label(tr(if *action == Action::Check { "正在检查更新…" } else { "正在检测…" })); });
            }
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        let mut changed = false;
        ui.add_enabled_ui(!busy, |ui| {
            changed = self.picker.show(ui, "yt-dlp", tr("yt-dlp 程序路径"));
        });
        ui.hyperlink_to(tr("手动安装"), "https://github.com/yt-dlp/yt-dlp/releases");
        if let Some(error) = &self.error { ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().warn_fg_color)).wrap().selectable(true)); }
        if install { self.start(ui.ctx(), Action::Install); }
        else if check { self.start(ui.ctx(), Action::Check); }
        else if detect || changed { self.start(ui.ctx(), Action::Detect); }
    }
}
