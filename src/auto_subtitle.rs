use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}, mpsc},
};

use eframe::egui;
use crate::{localization::tr, media_tools::command};
use crate::third_party::install_progress::Shared;

const WORKER: &str = include_str!("auto_subtitle.py");
static COMPONENT_ACCESS: Mutex<()> = Mutex::new(());

const PACKAGE: &str = "sherpa-onnx==2.0.1";

pub(crate) fn component() -> Result<PathBuf, String> {
    Ok(crate::third_party::component_root()?.join("sensevoice-sherpa-onnx-2.0.1"))
}

pub(crate) fn installed() -> bool {
    component().is_ok_and(|path| path.join("ready").is_file() && path.join("sherpa_onnx").is_dir())
}

pub(crate) fn sidecar(path: &Path, code: &str) -> PathBuf {
    path.with_extension(format!("{code}.srt"))
}

pub(crate) fn python() -> Result<Command, String> {
    let executable = if cfg!(windows) { "python.exe" } else { "python3" };
    let python = std::env::var_os("PATH").iter().flat_map(std::env::split_paths)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(executable))
        .find(|path| path.is_file())
        .ok_or_else(|| tr("请先安装 Python 并将其加入 PATH。").to_owned())?;
    Ok(command(python))
}

pub(crate) fn run(job: &mut Option<mpsc::Receiver<Result<PathBuf, String>>>, ctx: &egui::Context,
       work: impl FnOnce() -> Result<PathBuf, String> + Send + 'static) {
    let (sender, receiver) = mpsc::channel();
    *job = Some(receiver);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = sender.send(work());
        ctx.request_repaint();
    });
}

pub(crate) fn poll(job: &mut Option<mpsc::Receiver<Result<PathBuf, String>>>) -> Option<Result<PathBuf, String>> {
    let result = match job.as_ref()?.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return None,
        Err(mpsc::TryRecvError::Disconnected) => Err(tr("任务意外结束").into()),
    };
    *job = None;
    Some(result)
}

fn pip_download_progress(line: &str) -> Option<(u64, u64)> {
    let (done, total) = line.strip_prefix("Progress ")?.split_once(" of ")?;
    Some((done.parse().ok()?, total.parse().ok()?))
}

fn install(progress: &Shared) -> Result<PathBuf, String> {
    let _guard = COMPONENT_ACCESS.lock().unwrap();
    let path = component()?;
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    let marker = path.join("ready");
    if marker.exists() {
        fs::remove_file(&marker).map_err(|error| error.to_string())?;
    }
    install_packages(&path, &[PACKAGE], progress)?;
    progress.phase(5);
    verify(&path)?;
    fs::write(marker, PACKAGE).map_err(|error| error.to_string())?;
    Ok(path)
}

pub(crate) fn install_packages(path: &Path, packages: &[&str], progress: &Shared) -> Result<(), String> {
    let help = python()?.args(["-I", "-m", "pip", "install", "--help"])
        .output().map_err(|error| error.to_string())?;
    if !help.status.success() {
        return Err(String::from_utf8_lossy(&help.stderr).trim().into());
    }
    let raw = String::from_utf8_lossy(&help.stdout).split(|ch: char| !ch.is_ascii_alphabetic())
        .any(|word| word == "raw");
    let mut command = python()?;
    command.args(["-I", "-u", "-m", "pip", "install", "--upgrade",
        "--disable-pip-version-check", "--no-input", "--no-color", "--only-binary=:all:",
        "--progress-bar", if raw { "raw" } else { "off" }, "--target"])
        .arg(path).args(packages);
    run_install_command(command, progress)
}

pub(crate) fn run_install_command(mut command: Command, progress: &Shared) -> Result<(), String> {
    progress.phase(4);
    let (reader, writer) = io::pipe().map_err(|error| error.to_string())?;
    let mut child = command.stdin(Stdio::null())
        .stdout(writer.try_clone().map_err(|error| error.to_string())?).stderr(writer)
        .spawn().map_err(|error| error.to_string())?;
    drop(command);
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let read_result = loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break Ok(()),
            Ok(_) => {
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();
                if let Some((done, total)) = pip_download_progress(line) {
                    progress.bytes(done, total);
                    progress.phase(2);
                } else if !line.is_empty() {
                    progress.bytes(0, 0);
                    progress.phase(if line.starts_with("Downloading ") { 2 } else { 4 });
                    progress.log(line);
                }
            }
            Err(error) => break Err(error),
        }
    };
    drop(reader);
    let status = child.wait().map_err(|error| error.to_string())?;
    read_result.map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(status.to_string());
    }
    Ok(())
}

fn verify(path: &Path) -> Result<(), String> {
    let symbols = "OfflineRecognizer VadModelConfig VoiceActivityDetector";
    let output = python()?.args(["-I", "-c",
        "import importlib, sys; sys.path.insert(0, sys.argv[1]); module = importlib.import_module(sys.argv[2]); [getattr(module, name) for name in sys.argv[3].split()]"])
        .arg(path).arg("sherpa_onnx").arg(symbols).output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().into());
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct Installer {
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    error: Option<String>,
    progress: Shared,
    checking: bool,
    success: Option<&'static str>,
    model_job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    model_progress: Shared,
    model_error: Option<String>,
}

impl Installer {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(result) = poll(&mut self.job) {
            self.success = result.is_ok().then_some(if self.checking { "SenseVoice 检测通过" } else { "SenseVoice 安装完成" });
            self.error = result.err();
        }
        ui.horizontal_wrapped(|ui| {
            crate::theme::section_heading(ui, "SenseVoice");
            crate::third_party::component_status_badge(ui, installed());
            if ui.add_enabled(self.job.is_none() && self.model_job.is_none(), egui::Button::new(tr("安装 SenseVoice"))).clicked() {
                self.error = None;
                self.success = None;
                self.checking = false;
                self.progress = Shared::default();
                let progress = self.progress.clone();
                run(&mut self.job, ui.ctx(), move || install(&progress));
            }
            if ui.add_enabled(self.job.is_none() && self.model_job.is_none() && installed(), egui::Button::new(tr("重新检测"))).clicked() {
                self.error = None;
                self.success = None;
                self.checking = true;
                self.progress = Shared::default();
                run(&mut self.job, ui.ctx(), move || {
                    let _guard = COMPONENT_ACCESS.lock().unwrap();
                    let path = component()?;
                    verify(&path)?;
                    Ok(path)
                });
            }
        });
        if self.job.is_some() {
            self.progress.show_for(ui, "pip");
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        } else if self.error.is_some() {
            self.progress.show_log(ui);
        }
        if let Some(message) = self.success {
            ui.label(tr(message));
        }
        ui.small(tr("安装运行库需要 PATH 中的 Python 和 pip。"));
        if let Some(result)=poll(&mut self.model_job) {self.model_error=result.err();}
        for (engine, label) in [("sensevoice", "SenseVoice"), ("zipformer", "Zipformer")] {
            ui.horizontal_wrapped(|ui| {
                ui.label(label);
                crate::third_party::component_status_badge(ui, crate::live_subtitle::model_installed(engine));
                if ui.add_enabled(installed() && self.job.is_none() && self.model_job.is_none(),egui::Button::new(tr("安装 / 校验语言模型"))).clicked() {
                    self.model_error=None;self.model_progress=Shared::default();
                    let progress=self.model_progress.clone();
                    run(&mut self.model_job,ui.ctx(),move || {
                        let _guard=COMPONENT_ACCESS.lock().unwrap();
                        run_install_command(crate::live_subtitle::worker(engine,"zh","prepare")?,&progress)?;
                        Ok(component()?)
                    });
                }
            });
            if engine=="sensevoice" {
                for (_,label) in LANGUAGES {ui.label(format!("  • {}",tr(label)));}
                ui.small(tr("五种语言共用一个约 240 MB 模型，安装一次即可全部使用。"));
            } else {
                ui.small(tr("中英双语流式模型，约 198 MB；不包含日语、韩语或粤语。"));
                ui.hyperlink_to(tr("组件来源与许可"),"https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20");
            }
        }
        if self.model_job.is_some() {self.model_progress.show_for(ui,"ASR");ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));}
        if let Some(error)=&self.model_error {ui.colored_label(ui.visuals().error_fg_color,error);self.model_progress.show_log(ui);}
        ui.hyperlink_to(tr("组件来源与许可"), "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/blob/main/LICENSE");
        ui.hyperlink_to(tr("下载 Python"), "https://www.python.org/downloads/windows/");
        ui.small(tr("使用传统安装器时，勾选 Add Python to PATH 并保留 pip。安装后重启本程序。"));
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
}

#[derive(Default)]
pub(crate) struct Generator {
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    result: Option<Result<PathBuf, String>>,
    track: Option<u64>,
    stage: Arc<Mutex<GenerationStage>>,
    reveal_error: Option<String>,
}

#[derive(Clone, Copy, Default)]
enum GenerationStage {
    #[default]
    Waiting,
    Model,
    Transcribing,
    Saving,
}

impl GenerationStage {
    fn label(self) -> &'static str {
        match self {
            Self::Waiting => "正在等待字幕任务…",
            Self::Model => "正在准备语言模型（首次使用时下载）…",
            Self::Transcribing => "正在转写音轨…",
            Self::Saving => "正在保存字幕…",
        }
    }
}

pub(crate) const LANGUAGES: [(&str, &str); 5] = [("zh", "国语"), ("en", "英语"), ("ja", "日语"), ("ko", "韩语"), ("yue", "粤语")];

pub(crate) fn language_code(language: Option<&str>) -> Option<&'static str> {
    match language {
        Some("国语" | "中文") => Some("zh"),
        Some("英语") => Some("en"),
        Some("日语") => Some("ja"),
        Some("韩语") => Some("ko"),
        Some("粤语") => Some("yue"),
        _ => None,
    }
}

impl Generator {
    pub(crate) fn poll(&mut self, path: &Path) -> Option<(PathBuf, u64)> {
        let result = poll(&mut self.job)?;
        let ready = result.as_ref().ok().filter(|output| {
            LANGUAGES.iter().any(|(code, _)| **output == sidecar(path, code))
        }).cloned().zip(self.track.take());
        self.result = Some(result);
        self.reveal_error = None;
        ready
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, path: &Path, ffmpeg: &Path, track: u64) -> Option<PathBuf> {
        let mut ready = None;
        ui.add_enabled_ui(self.job.is_none(), |ui| {
            let existing = path.with_extension("srt");
            if existing.is_file() && ui.button(tr("加载同名字幕")).clicked() {
                self.result = None;
                ready = Some(existing);
            }
            ui.menu_button(tr("自动生成字幕"), |ui| {
                let installed = installed();
                for (code, label) in LANGUAGES {
                    let subtitle = sidecar(path, code);
                    let exists = subtitle.is_file();
                    let label = if exists {
                        format!("{} · {}", tr(label), tr("加载同名字幕"))
                    } else if installed {
                        tr(label).to_owned()
                    } else {
                        format!("{} · {}", tr(label), tr("前往第三方支持"))
                    };
                    if ui.button(label).clicked() {
                        if exists {
                            self.result = None;
                            ready = Some(subtitle);
                        } else if installed {
                            let (path, ffmpeg) = (path.to_owned(), ffmpeg.to_owned());
                            self.result = None;
                            self.track = Some(track);
                            self.stage = Arc::default();
                            let stage = self.stage.clone();
                            run(&mut self.job, ui.ctx(), move || generate(&path, &ffmpeg, track, code, &stage));
                        } else {
                            crate::third_party::request_open(ui.ctx());
                        }
                        ui.close_menu();
                    }
                }
            }).response.on_hover_text(tr("使用当前音轨，首次使用该语言时下载模型。"));
        });
        if self.job.is_some() {
            ui.spinner();
            ui.label(tr(self.stage.lock().unwrap().label()));
        }
        match &self.result {
            Some(Ok(path)) => {
                if ui.link(tr("字幕已保存")).on_hover_text(path.display().to_string()).clicked() {
                    if let Err(error) = crate::asset_viewer::reveal_file(path) {
                        self.reveal_error = Some(error.to_string());
                    } else {
                        self.reveal_error = None;
                    }
                }
                if let Some(error) = &self.reveal_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            }
            Some(Err(error)) => { ui.colored_label(ui.visuals().error_fg_color, error); }
            None => {}
        }
        ready
    }
}

fn generate(path: &Path, ffmpeg: &Path, track: u64, language: &str, stage: &Mutex<GenerationStage>) -> Result<PathBuf, String> {
    let destination = sidecar(path, language);
    if destination.exists() {
        return Err(tr("已有同名字幕文件，请先移动或重命名。").into());
    }
    let _guard = COMPONENT_ACCESS.lock().unwrap();
    if !installed() {
        return Err(tr("组件记录无效").into());
    }
    *stage.lock().unwrap() = GenerationStage::Model;
    let component = component()?;
    let mut worker = python()?;
    let mut decoder = command(ffmpeg)
        .args(["-nostdin", "-v", "error", "-xerror", "-i"]).arg(path)
        .args(["-map", &format!("0:{track}"), "-ac", "1", "-af",
            "aresample=16000:async=1:first_pts=0", "-f", "s16le", "pipe:1"])
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|error| error.to_string())?;
    let output = (|| {
        let mut child = worker.args(["-I", "-c", WORKER]).arg(component).arg(language)
            .stdin(decoder.stdout.take().unwrap()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().map_err(|error| error.to_string())?;
        let stderr = child.stderr.take().unwrap();
        std::thread::scope(|scope| {
            let diagnostics = scope.spawn(|| {
                let mut diagnostics = String::new();
                let mut reader = BufReader::new(stderr);
                let mut line = Vec::new();
                while reader.read_until(b'\n', &mut line).is_ok_and(|len| len != 0) {
                    let text = String::from_utf8_lossy(&line);
                    if text.trim() == "UNITY_SUBTITLE_STAGE transcribe" {
                        *stage.lock().unwrap() = GenerationStage::Transcribing;
                    } else {
                        diagnostics.push_str(&text);
                    }
                    line.clear();
                }
                diagnostics
            });
            let output = child.wait_with_output().map_err(|error| error.to_string())?;
            Ok::<_, String>((output, diagnostics.join().map_err(|_| tr("字幕生成失败"))?))
        })
    })();
    if !output.as_ref().is_ok_and(|(output, _)| output.status.success()) {
        let _ = decoder.kill();
    }
    let decoded = decoder.wait().map_err(|error| error.to_string())?;
    let (output, diagnostics) = output?;
    if !output.status.success() {
        return Err(if diagnostics.trim().is_empty() {
            format!("{} ({})", tr("字幕生成失败"), output.status)
        } else {
            diagnostics.trim().into()
        });
    }
    if !decoded.success() {
        return Err(tr("音轨解码失败").into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload = stdout.lines().rev().find_map(|line| line.strip_prefix("UNITY_SUBTITLE "))
        .ok_or_else(|| tr("字幕生成失败").to_owned())?;
    let subtitles: String = serde_json::from_str(payload).map_err(|error| error.to_string())?;
    if subtitles.is_empty() {
        return Err(tr("未识别到可生成字幕的语音。").into());
    }
    *stage.lock().unwrap() = GenerationStage::Saving;
    save_new(destination, &subtitles)
}

pub(crate) fn save_new(destination: PathBuf, subtitles: &str) -> Result<PathBuf, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let staging = destination.with_file_name(format!(".unity-subtitle-{}-{}.tmp",
        std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&staging)
        .map_err(|error| error.to_string())?;
    let written = file.write_all(subtitles.as_bytes()).and_then(|_| file.sync_all());
    drop(file);
    let result = written.map_err(|error| error.to_string())
        .and_then(|_| crate::playlist::move_new(&staging, &destination));
    let _ = fs::remove_file(staging);
    result.map(|_| destination)
}
