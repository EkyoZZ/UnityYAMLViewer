use std::{fs, io::Write, path::{Path, PathBuf}, process::Stdio, sync::{Mutex, mpsc}};

use eframe::egui;
use crate::{
    auto_subtitle::{install_packages, poll, python, run, run_install_command, save_new},
    localization::tr,
    media_tools::command,
    subtitle_player::Source,
    third_party::install_progress::Shared,
};

const WORKER: &str = include_str!("subtitle_translation.py");
const DIRECTIONS: [(&str, &str); 2] = [("zh-to-en", "中 → 英"), ("en-to-zh", "英 → 中")];
const PACKAGES: &[&str] = &["ctranslate2==4.8.5", "sentencepiece==0.2.3"];
static INSTALL: Mutex<()> = Mutex::new(());

fn root() -> Result<PathBuf, String> {
    Ok(crate::third_party::component_root()?.join("opus-mt-int8-1"))
}

fn installed(direction: &str) -> bool {
    root().is_ok_and(|root| {
        let model = root.join(direction);
        root.join("runtime-ready").is_file() &&
            ["model.bin", "config.json", "shared_vocabulary.json", "source.spm", "target.spm"]
                .iter().all(|name| model.join(name).is_file())
    })
}

fn install(direction: &str, progress: &Shared) -> Result<PathBuf, String> {
    let _guard = INSTALL.lock().unwrap();
    let root = root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let marker = root.join("runtime-ready");
    if !marker.is_file() {
        install_packages(&root.join("runtime"), PACKAGES, progress)?;
        let output = python()?.args(["-I", "-c",
            "import sys; sys.path.insert(0, sys.argv[1]); import ctranslate2, sentencepiece"])
            .arg(root.join("runtime")).output().map_err(|error| error.to_string())?;
        if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().into()); }
        fs::write(marker, "").map_err(|error| error.to_string())?;
    }
    let mut worker = python()?;
    worker.args(["-I", "-u", "-c", WORKER, "install"]).arg(&root).arg(direction);
    run_install_command(worker, progress)?;
    Ok(root)
}

#[derive(Default)]
pub(crate) struct Installer {
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    error: Option<String>,
    progress: Shared,
}

impl Installer {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = poll(&mut self.job) { self.error = result.err(); }
        crate::theme::section_heading(ui, "OPUS-MT INT8");
        ui.small(tr("每个方向首次下载约 289 MB，转换后模型约 86 MB，共用 Python 运行库。"));
        for (direction, label) in DIRECTIONS {
            let installed = installed(direction);
            ui.horizontal(|ui| {
                ui.label(tr(label));
                crate::third_party::component_status_badge(ui, installed);
                if ui.add_enabled(!installed && self.job.is_none(), egui::Button::new(tr("安装"))).clicked() {
                    self.error = None;
                    self.progress = Shared::default();
                    let progress = self.progress.clone();
                    run(&mut self.job, ui.ctx(), move || install(direction, &progress));
                }
            });
        }
        if self.job.is_some() {
            self.progress.show_for(ui, "OPUS-MT");
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        if let Some(error) = &self.error {
            self.progress.show_log(ui);
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}

#[derive(Default)]
pub(crate) struct Translator {
    bilingual: bool,
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    error: Option<String>,
}

impl Translator {
    pub(crate) fn poll(&mut self, video: &Path) -> Option<PathBuf> {
        match poll(&mut self.job)? {
            Ok(path) => DIRECTIONS.iter().any(|(direction, _)| {
                path == output_path(video, direction, false) || path == output_path(video, direction, true)
            }).then_some(path),
            Err(error) => { self.error = Some(error); None }
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, video: &Path, ffmpeg: &Path, source: Option<Source>) {
        ui.add_enabled_ui(source.is_some() && self.job.is_none(), |ui| {
            ui.menu_button(tr("翻译字幕"), |ui| {
                let Some(source) = source else {return;};
                ui.small(&source.label);
                ui.checkbox(&mut self.bilingual, tr("同时生成双语字幕"));
                for (direction, label) in DIRECTIONS {
                    if ui.button(tr(label)).clicked() {
                        self.error = None;
                        if installed(direction) {
                            let (video, ffmpeg, source) = (video.to_owned(), ffmpeg.to_owned(), source.clone());
                            let bilingual = self.bilingual;
                            run(&mut self.job, ui.ctx(), move || translate(&video, &ffmpeg, &source, direction, bilingual));
                        } else {
                            crate::third_party::request_open(ui.ctx());
                        }
                        ui.close_menu();
                    }
                }
            });
        });
        if self.job.is_some() { ui.spinner(); ui.label(tr("正在翻译字幕…")); }
        if let Some(error) = &self.error { ui.colored_label(ui.visuals().error_fg_color, error); }
    }
}

fn output_path(video: &Path, direction: &str, bilingual: bool) -> PathBuf {
    let suffix = if bilingual { ".bilingual" } else { "" };
    video.with_extension(format!("{direction}{suffix}.srt"))
}

pub(crate) fn is_translation(path: &Path) -> bool {
    path.file_stem().and_then(|name| name.to_str()).is_some_and(|name| {
        let name = name.strip_suffix(".bilingual").unwrap_or(name);
        name.rsplit_once('.').is_some_and(|(_, suffix)| DIRECTIONS.iter().any(|(direction, _)| *direction == suffix))
    })
}

fn translate(video: &Path, ffmpeg: &Path, source: &Source, direction: &str, bilingual: bool) -> Result<PathBuf, String> {
    let single = output_path(video, direction, false);
    let dual = output_path(video, direction, true);
    if single.exists() || bilingual && dual.exists() {
        return Err(tr("已有同名字幕文件，请先移动或重命名。").into());
    }
    let srt = if source.path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("srt")) {
        fs::read(&source.path).map_err(|error| error.to_string())?
    } else {
        let output = command(ffmpeg).args(["-nostdin", "-v", "error", "-i"]).arg(&source.path)
            .args(["-map", &format!("0:s:{}", source.ordinal), "-c:s", "srt", "-f", "srt", "pipe:1"])
            .output().map_err(|error| error.to_string())?;
        if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().into()); }
        output.stdout
    };
    let mut worker = python()?.args(["-I", "-c", WORKER, "translate"]).arg(root()?)
        .arg(direction).arg(if bilingual { "1" } else { "0" })
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|error| error.to_string())?;
    let mut stdin = worker.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(&srt));
    let output = worker.wait_with_output().map_err(|error| error.to_string());
    let written = writer.join().map_err(|_| tr("任务意外结束"))?;
    let output = output?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().into()); }
    written.map_err(|error| error.to_string())?;
    let [translated, combined]: [String; 2] = serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let saved = save_new(single, &translated)?;
    if bilingual { save_new(dual, &combined) } else { Ok(saved) }
}
