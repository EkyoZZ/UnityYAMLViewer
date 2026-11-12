use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, mpsc},
};

use crate::{
    auto_subtitle::{install_packages, poll, python, run, run_install_command, save_new},
    localization::tr,
    media_tools::command,
    subtitle_player::Source,
    third_party::install_progress::Shared,
};
use eframe::egui;

const WORKER: &str = include_str!("translation_download.py");
const M2M_WORKER: &str = include_str!("m2m_translation.py");
const LANGUAGES: [(&str, &str); 4] = [
    ("zh", "中文"),
    ("en", "英语"),
    ("ja", "日语"),
    ("ko", "韩语"),
];
const PACKAGES: &[&str] = &["ctranslate2==4.8.5", "sentencepiece==0.2.3"];
static INSTALL: Mutex<()> = Mutex::new(());

pub(crate) fn m2m_root() -> Result<PathBuf, String> {
    Ok(crate::third_party::component_root()?.join("m2m100-418m-ct2-int8-1"))
}
pub(crate) fn m2m_installed() -> bool {
    m2m_root().is_ok_and(|p| p.join("runtime-ready").is_file())
        && m2m_root().is_ok_and(|p| {
            [
                ("model.bin", 490667638),
                ("config.json", 158),
                ("shared_vocabulary.txt", 1159930),
                ("spm.128k.model", 2423393),
                ("README.md", 1218),
            ]
            .iter()
            .all(|(name, size)| {
                fs::metadata(p.join("model").join(name)).is_ok_and(|m| m.len() == *size)
            })
        })
}

fn ensure_runtime(progress: &Shared) -> Result<(), String> {
    let root = m2m_root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let marker = root.join("runtime-ready");
    if !marker.is_file() {
        progress.log(tr("正在安装翻译运行库；完成后还需下载并转换语言模型。"));
        install_packages(&root.join("runtime"), PACKAGES, progress)?;
        let output = python()?
            .args([
                "-I",
                "-c",
                "import sys; sys.path.insert(0, sys.argv[1]); import ctranslate2, sentencepiece",
            ])
            .arg(root.join("runtime"))
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().into());
        }
        fs::write(marker, "").map_err(|error| error.to_string())?;
    }
    Ok(())
}
fn install_m2m(progress: &Shared) -> Result<PathBuf, String> {
    let _guard = INSTALL.lock().unwrap();
    ensure_runtime(progress)?;
    let root = m2m_root()?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mut worker = python()?;
    worker
        .args(["-I", "-u", "-c", M2M_WORKER, WORKER, "install"])
        .arg(&root);
    run_install_command(worker, progress)?;
    Ok(root)
}

#[derive(Default)]
pub(crate) struct Installer {
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    error: Option<String>,
    progress: Shared,
    success: bool,
}

impl Installer {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = poll(&mut self.job) {
            self.success = result.is_ok();
            self.error = result.err();
        }
        crate::theme::section_heading(ui, "M2M100 INT8 · CTranslate2");
        ui.horizontal_wrapped(|ui| {
            crate::third_party::component_status_badge(ui, m2m_installed());
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new(tr("安装 / 校验语言模型")),
                )
                .clicked()
            {
                self.error = None;
                self.success = false;
                self.progress = Shared::default();
                let progress = self.progress.clone();
                run(&mut self.job, ui.ctx(), move || install_m2m(&progress));
            }
        });
        ui.small(tr(
            "中、英、日、韩共用模型，支持直接互译；不含粤语。",
        ));
        ui.small(tr(
            "下载约 494 MB；模型与保留下载包共约 1 GB，另需共用运行库。建议预留 2 GiB 硬盘空间。",
        ));
        ui.small(tr(
            "运行内存预算约 1–3 GiB（估算，未实测）；任务结束进程退出后释放，模型文件保留。",
        ));
        ui.small(tr(
            "采用社区预转换 INT8 模型，无需下载原始权重或在本机转换；安装后会校验文件及模型加载。",
        ));
        ui.hyperlink_to(
            tr("组件来源与许可"),
            "https://huggingface.co/JustFrederik/m2m_100_418m_ct2_int8",
        );
        ui.hyperlink_to(
            "M2M100 · MIT",
            "https://huggingface.co/facebook/m2m100_418M",
        );
        if let Some(peak) = m2m_root()
            .ok()
            .and_then(|p| fs::read(p.join("last-peak.json")).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v["peak_working_set_bytes"].as_u64())
        {
            ui.small(format!(
                "{}: {:.0} MiB",
                tr("上次模型进程峰值工作集（含安装校验或翻译）"),
                peak as f64 / 1048576.0
            ));
        }
        if self.success {
            ui.label(tr("翻译组件安装或校验完成。"));
        }
        if self.job.is_some() {
            self.progress.show_for(ui, "M2M100");
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(150));
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
    source_language: usize,
    target_language: usize,
    source_identity: Option<(PathBuf, usize)>,
}

impl Translator {
    pub(crate) fn poll(&mut self, video: &Path) -> Option<PathBuf> {
        match poll(&mut self.job)? {
            Ok(path) => LANGUAGES
                .iter()
                .any(|(a, _)| {
                    LANGUAGES.iter().any(|(b, _)| {
                        let direction = format!("{a}-to-{b}");
                        path == output_path(video, &direction, false)
                            || path == output_path(video, &direction, true)
                    })
                })
                .then_some(path),
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        video: &Path,
        ffmpeg: &Path,
        source: Option<Source>,
    ) {
        ui.add_enabled_ui(source.is_some() && self.job.is_none(), |ui| {
            ui.menu_button(tr("翻译字幕"), |ui| {
                let Some(source) = source else {
                    return;
                };
                let identity = (source.path.clone(), source.ordinal);
                if self.source_identity.as_ref() != Some(&identity) {
                    if let Some(index) = LANGUAGES.iter().position(|(code, _)| {
                        crate::auto_subtitle::language_code(source.language) == Some(*code)
                    }) {
                        self.source_language = index;
                    }
                    self.target_language = if self.source_language == 0 { 1 } else { 0 };
                    self.source_identity = Some(identity);
                }
                ui.small(&source.label);
                ui.label("M2M100 INT8");
                let count = LANGUAGES.len();
                // Keep an unsupported source visible so a Japanese cue is never silently translated as Chinese.
                ui.label(tr("源语言"));
                egui::ComboBox::from_id_salt("subtitle_translation_source")
                    .selected_text(tr(LANGUAGES[self.source_language].1))
                    .show_ui(ui, |ui| {
                        for (index, (_, label)) in LANGUAGES.iter().enumerate().take(count) {
                            ui.selectable_value(&mut self.source_language, index, tr(label));
                        }
                    });
                ui.label(tr("目标语言"));
                egui::ComboBox::from_id_salt("subtitle_translation_target")
                    .selected_text(tr(LANGUAGES[self.target_language].1))
                    .show_ui(ui, |ui| {
                        for (index, (_, label)) in LANGUAGES.iter().enumerate().take(count) {
                            ui.selectable_value(&mut self.target_language, index, tr(label));
                        }
                    });
                ui.checkbox(&mut self.bilingual, tr("同时生成双语字幕"));
                let supported = self.source_language < count
                    && self.target_language < count
                    && self.source_language != self.target_language;
                if !supported {
                    ui.small(tr("请选择不同且受当前模型支持的源语言和目标语言。"));
                }
                if source.language == Some("粤语") {
                    ui.small(tr("当前翻译模型未提供粤语，请勿将其直接视为普通话。"));
                }
                let direction = format!(
                    "{}-to-{}",
                    LANGUAGES[self.source_language].0, LANGUAGES[self.target_language].0
                );
                let ready = m2m_installed();
                if ui
                    .add_enabled(
                        supported,
                        egui::Button::new(tr(if ready {
                            "开始翻译"
                        } else {
                            "前往第三方支持"
                        })),
                    )
                    .clicked()
                {
                    self.error = None;
                    if ready {
                        let (video, ffmpeg, source) =
                            (video.to_owned(), ffmpeg.to_owned(), source.clone());
                        let bilingual = self.bilingual;
                        run(&mut self.job, ui.ctx(), move || {
                            translate(&video, &ffmpeg, &source, &direction, bilingual)
                        });
                    } else {
                        crate::third_party::request_open(ui.ctx());
                    }
                    ui.close_menu();
                }
            });
        });
        if self.job.is_some() {
            ui.spinner();
            ui.label(tr("正在翻译字幕…"));
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}

fn output_path(video: &Path, direction: &str, bilingual: bool) -> PathBuf {
    let suffix = if bilingual { ".bilingual" } else { "" };
    video.with_extension(format!("m2m100-{direction}{suffix}.srt"))
}

pub(crate) fn is_translation(path: &Path) -> bool {
    path.file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.strip_suffix(".bilingual").unwrap_or(name);
            name.rsplit_once('.').is_some_and(|(_, suffix)| {
                let suffix = suffix.strip_prefix("m2m100-").unwrap_or(suffix);
                suffix.split_once("-to-").is_some_and(|(a, b)| {
                    a != b
                        && LANGUAGES.iter().any(|(code, _)| *code == a)
                        && LANGUAGES.iter().any(|(code, _)| *code == b)
                })
            })
        })
}

fn translate(
    video: &Path,
    ffmpeg: &Path,
    source: &Source,
    direction: &str,
    bilingual: bool,
) -> Result<PathBuf, String> {
    let single = output_path(video, direction, false);
    let dual = output_path(video, direction, true);
    if single.exists() || bilingual && dual.exists() {
        return Err(tr("已有同名字幕文件，请先移动或重命名。").into());
    }
    let srt = if source
        .path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("srt"))
    {
        fs::read(&source.path).map_err(|error| error.to_string())?
    } else {
        let output = command(ffmpeg)
            .args(["-nostdin", "-v", "error", "-i"])
            .arg(&source.path)
            .args([
                "-map",
                &format!("0:s:{}", source.ordinal),
                "-c:s",
                "srt",
                "-f",
                "srt",
                "pipe:1",
            ])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().into());
        }
        output.stdout
    };
    let mut command = python()?;
    let (from, to) = direction.split_once("-to-").ok_or("Invalid translation language pair")?;
    if from==to || !LANGUAGES.iter().any(|(code,_)|*code==from) || !LANGUAGES.iter().any(|(code,_)|*code==to){return Err("Unsupported translation language pair".into());}
    command.args(["-I", "-c", M2M_WORKER, WORKER, "translate"])
        .arg(m2m_root()?).args([from,to,if bilingual {"1"}else{"0"}]);
    let mut worker = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdin = worker.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(&srt));
    let output = worker.wait_with_output().map_err(|error| error.to_string());
    let written = writer.join().map_err(|_| tr("任务意外结束"))?;
    let output = output?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().into());
    }
    written.map_err(|error| error.to_string())?;
    let [translated, combined]: [String; 2] =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let saved = save_new(single, &translated)?;
    if bilingual {
        save_new(dual, &combined)
    } else {
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m2m_outputs_and_legacy_translation_suffixes_are_detected() {
        let video=Path::new("movie.mp4");
        assert_eq!(output_path(video,"ja-to-zh",true),PathBuf::from("movie.m2m100-ja-to-zh.bilingual.srt"));
        assert_eq!(output_path(video,"zh-to-en",false),PathBuf::from("movie.m2m100-zh-to-en.srt"));
        assert!(is_translation(Path::new("movie.m2m100-ko-to-zh.srt")));
        assert!(is_translation(Path::new("movie.en-to-zh.bilingual.srt")));
        assert!(!is_translation(Path::new("movie.ja.srt")));
        assert!(!is_translation(Path::new("movie.m2m100-yue-to-zh.srt")));
    }
}
