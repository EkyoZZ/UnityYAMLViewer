use super::*;
use crate::background_tasks::{Outcome, Progress, Target, Task};
use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct MergeSettings {
    video: VideoSettings,
    audio: AudioSettings,
    size: [u32; 2],
}

impl Default for MergeSettings {
    fn default() -> Self {
        let mut video = VideoSettings::default();
        video.fps = "30".into();
        Self {
            video,
            audio: AudioSettings::default(),
            size: [1920, 1080],
        }
    }
}

impl MergeSettings {
    pub(crate) fn validate_project(&self) -> Result<(), String> {
        super::EditState::from_merge(self.video.clone(), self.audio.clone()).validate()?;
        if self.size.iter().any(|n| *n < 2 || *n > 8192 || *n % 2 != 0) {
            return Err(tr("工程设置无效或格式版本不受支持。").into());
        }
        Ok(())
    }
    pub(crate) fn set_initial_size(&mut self, size: Option<[usize; 2]>) {
        if let Some([width, height]) = size {
            self.size = [
                (width.clamp(2, 8192) as u32).div_ceil(2) * 2,
                (height.clamp(2, 8192) as u32).div_ceil(2) * 2,
            ];
        }
    }

    pub(crate) fn controls(&mut self, ui: &mut egui::Ui, video: bool) {
        if video {
            ui.label(tr("合并分辨率"));
            egui::Grid::new("merge_resolution")
                .num_columns(2)
                .show(ui, |ui| {
                    for (axis, label) in ["宽度", "高度"].into_iter().enumerate() {
                        ui.label(tr(label));
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.size[axis])
                                    .range(2..=8192)
                                    .suffix(" px"),
                            )
                            .changed()
                        {
                            self.size[axis] = self.size[axis].div_ceil(2) * 2;
                        }
                        ui.end_row();
                    }
                });
            ui.small(tr("按此尺寸等比缩放、居中补黑边。"));
            self.video.controls(ui);
        } else {
            self.audio.controls_rows(ui);
        }
    }

    fn extension(&self, video: bool) -> &'static str {
        if video {
            self.video.extension()
        } else {
            self.audio.format.extension()
        }
    }
}

pub(crate) struct ExportPlan {
    source: PathBuf,
    original: PathBuf,
    ffmpeg: PathBuf,
    start: f64,
    end: f64,
    duration: f64,
    video: bool,
    audio: Option<u64>,
    settings: AudioSettings,
    video_settings: VideoSettings,
    tracks: Vec<crate::export_tracks::Track>,
    filter: Option<String>,
    _clip: Option<Arc<crate::audio_edit::Clip>>,
    _sequence: Option<Arc<crate::media_preview::SequenceSource>>,
}

impl ExportPlan {
    fn extension(&self) -> &'static str {
        if self.video {
            self.video_settings.extension()
        } else {
            self.settings.format.extension()
        }
    }

    fn encode(&self, target: &Path) -> Result<PathBuf, String> {
        export(
            &self.ffmpeg,
            &self.source,
            target,
            self.start,
            self.end,
            self.duration,
            self.video,
            self.audio,
            &self.settings,
            &self.video_settings,
            &self.tracks,
            self.filter.as_deref(),
        )
    }
}

impl Trim {
    pub(crate) fn prepare_queue(
        &mut self,
        path: &Path,
        audio: Option<u64>,
        ctx: &egui::Context,
        video: bool,
    ) {
        if video {
            self.video_tracks.initialize(path, audio, ctx);
        }
    }

    pub(crate) fn export_plan(
        &self,
        path: &Path,
        ffmpeg: &Path,
        duration: f64,
        video: bool,
        audio: Option<u64>,
        merge: bool,
    ) -> Result<ExportPlan, String> {
        if self.pending.is_some()
            || self
                .session
                .as_ref()
                .is_some_and(|session| !session.ready())
        {
            return Err(tr("请等待当前编辑任务完成。").into());
        }
        if video
            && (if merge {
                !self.video_tracks.ready()
            } else {
                self.video_settings.container != 10
                    && !self.video_tracks.valid(self.video_settings.container)
            })
        {
            return Err(tr("请等待轨道信息读取完成，并检查每条的导出设置。").into());
        }
        let clip = self.clip();
        let (source, start, end, duration, audio) = match &clip {
            Some(clip) => (
                clip.path.clone(),
                0.0,
                clip.duration,
                clip.duration,
                Some(0),
            ),
            None => (path.to_owned(), self.start, self.end, duration, audio),
        };
        if !valid_range(start, end, duration) {
            return Err(tr("终点必须大于起点且不超过媒体时长").into());
        }
        Ok(ExportPlan {
            source,
            original: path.to_owned(),
            ffmpeg: ffmpeg.to_owned(),
            start,
            end,
            duration,
            video,
            audio,
            settings: self.settings.clone(),
            video_settings: self.video_settings.clone(),
            tracks: self.video_tracks.entries.clone(),
            filter: self.session.as_ref().map(|session| session.filter_string()),
            _clip: clip,
            _sequence: self.sequence_source.clone(),
        })
    }
}

pub(crate) fn export_queue(
    ctx: &egui::Context,
    plans: Vec<ExportPlan>,
    merge: Option<MergeSettings>,
) -> Result<(), String> {
    let Some(first) = plans.first() else {
        return Ok(());
    };
    if let Some(settings) = &merge {
        if first.video && settings.video.container >= 9 {
            return Err(tr("合并模式请选择普通视频格式，GIF 和 PNG 序列可用队列导出。").into());
        }
        if first.video
            && (u64::from(settings.size[0]) * u64::from(settings.size[1]) > 32 * 1024 * 1024)
        {
            return Err(tr("合并画面尺寸超过预览预算。").into());
        }
        if plans.iter().any(|plan| {
            plan.tracks.iter().any(|track| {
                track.enabled
                    && track.subtitle
                    && (!matches!(
                        track.codec.as_str(),
                        "subrip" | "ass" | "ssa" | "webvtt" | "mov_text" | "text"
                    ) || track.subtitle_encoder(settings.video.container).is_none())
            })
        }) {
            return Err(tr("合并封装不支持所选字幕，请更换合并设置或使用队列导出。").into());
        }
    }
    let directory = first.original.parent().map(Path::to_owned);
    let mut dialog = rfd::FileDialog::new();
    if let Some(directory) = directory {
        dialog = dialog.set_directory(directory);
    }
    let destination = if let Some(settings) = &merge {
        let extension = settings.extension(first.video);
        let Some(path) = dialog
            .add_filter(extension, &[extension])
            .set_file_name(format!("merged.{extension}"))
            .save_file()
        else {
            return Ok(());
        };
        let path = path.with_extension(extension);
        if plans.iter().any(|plan| same_path(&plan.original, &path)) {
            return Err(tr("合并输出不能覆盖列表中的源文件。").into());
        }
        path
    } else {
        let Some(path) = dialog.pick_folder() else {
            return Ok(());
        };
        path
    };
    let state = Arc::new(Mutex::new((tr("准备导出…").to_owned(), 0.0_f32)));
    let progress = state.clone();
    let task = Task::start(
        ctx,
        if merge.is_some() {
            "合并导出"
        } else {
            "队列导出"
        },
        destination.display().to_string(),
        move || {
            let state = progress.lock().unwrap();
            Progress {
                stage: state.0.clone(),
                fraction: Some(state.1),
                cancellable: false,
            }
        },
        None,
    );
    std::thread::spawn(move || {
        let result = if let Some(settings) = merge {
            merge_plans(&plans, &destination, &state, &settings).map(|_| destination.clone())
        } else {
            export_individual(&plans, &destination, &state).map(|_| destination.clone())
        };
        task.finish(match result {
            Ok(path) => Outcome::Complete(Some(Target::File(path))),
            Err(error) => Outcome::Failed(error),
        });
    });
    Ok(())
}

fn same_path(a: &Path, b: &Path) -> bool {
    let normalize = |path: &Path| {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_owned())
            .to_string_lossy()
            .to_lowercase()
    };
    normalize(a) == normalize(b)
}

fn export_individual(
    plans: &[ExportPlan],
    directory: &Path,
    state: &Mutex<(String, f32)>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        let stem = plan
            .original
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let frame_directory = plan.video && plan.video_settings.container == 10;
        let mut attempt = 0;
        let target = loop {
            let suffix = if attempt == 0 {
                String::new()
            } else {
                format!("-{attempt}")
            };
            let name = if frame_directory {
                format!("{stem}_frames{suffix}")
            } else {
                format!("{stem}_trim{suffix}.{}", plan.extension())
            };
            let target = directory.join(name);
            if frame_directory {
                if !target.exists() {
                    break target;
                }
            } else {
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                {
                    Ok(file) => {
                        drop(file);
                        break target;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
            attempt += 1;
        };
        *state.lock().unwrap() = (
            format!(
                "{}/{} · {}",
                index + 1,
                plans.len(),
                target.file_name().unwrap_or_default().to_string_lossy()
            ),
            index as f32 / plans.len() as f32,
        );
        match plan.encode(&target) {
            Ok(_) => {}
            Err(error) => {
                if !frame_directory {
                    let _ = fs::remove_file(&target);
                }
                errors.push(format!("{} · {error}", target.display()));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{}\n{}\n{}",
            tr("部分导出失败"),
            directory.display(),
            errors.join("\n")
        ))
    }
}

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = crate::app_cache::network_root().join(format!(
            "merge-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::app_cache::create_task_cache(&path).map_err(|error| error.to_string())?;
        Ok(Self(path))
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
        crate::app_cache::forget_task_cache(&self.0);
    }
}

fn execute(command: &mut std::process::Command) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

struct Metadata {
    duration: f64,
    size: Option<[u64; 2]>,
    fps: String,
}

fn metadata(probe: &Path, path: &Path) -> Result<Metadata, String> {
    let output = command(probe)
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let data: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let streams = data["streams"]
        .as_array()
        .ok_or_else(|| tr("无法读取媒体流").to_owned())?;
    let video = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video");
    let duration = data["format"]["duration"]
        .as_str()
        .and_then(|duration| duration.parse::<f64>().ok())
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .ok_or_else(|| tr("无法读取媒体时长").to_owned())?;
    Ok(Metadata {
        duration,
        size: video
            .and_then(|stream| Some([stream["width"].as_u64()?, stream["height"].as_u64()?])),
        fps: video
            .and_then(|stream| stream["avg_frame_rate"].as_str())
            .filter(|fps| *fps != "0/0")
            .unwrap_or("30")
            .to_owned(),
    })
}

fn merge_plans(
    plans: &[ExportPlan],
    target: &Path,
    state: &Mutex<(String, f32)>,
    settings: &MergeSettings,
) -> Result<(), String> {
    let first = &plans[0];
    let ffmpeg = &first.ffmpeg;
    let probe = crate::media_tools::find_tool("ffprobe")
        .ok_or_else(|| tr("未安装 ffprobe，请前往第三方支持安装。").to_owned())?;
    let workspace = Workspace::new()?;
    let root = &workspace.0;
    let video = first.video;
    let source_metadata = metadata(&probe, &first.source)?;
    let fps = if settings.video.fps == "source" {
        source_metadata.fps
    } else {
        settings.video.fps.clone()
    };
    let audio_count = plans
        .iter()
        .map(|plan| {
            if video {
                plan.tracks
                    .iter()
                    .filter(|track| track.enabled && !track.subtitle)
                    .count()
            } else {
                usize::from(plan.audio.is_some())
            }
        })
        .max()
        .unwrap_or_default();
    if video && settings.video.container == 5 && audio_count > 1 {
        return Err(tr("此容器最多导出一条音轨。").into());
    }
    let rate = if video {
        settings.video.audio_rate
    } else {
        settings.audio.rate
    };
    let channels = if video { 2 } else { settings.audio.channels };
    let mut pieces = Vec::new();
    let mut durations = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        *state.lock().unwrap() = (
            format!("{} {}/{}", tr("正在导出片段"), index + 1, plans.len()),
            index as f32 / (plans.len() + 1) as f32,
        );
        let path = root.join(format!("piece-{index}.{}", settings.extension(video)));
        encode_merge_piece(plan, settings, &path, &fps, audio_count, rate, channels)?;
        durations.push(metadata(&probe, &path)?.duration);
        pieces.push(path);
    }

    let subtitle_count = plans
        .iter()
        .map(|plan| {
            plan.tracks
                .iter()
                .filter(|track| track.enabled && track.subtitle)
                .count()
        })
        .max()
        .unwrap_or_default();
    let mut subtitle_tracks = Vec::new();
    for slot in 0..subtitle_count {
        let mut text = String::new();
        let mut offset = 0.0;
        let mut number = 0;
        let mut languages = Vec::new();
        for (plan, duration) in plans.iter().zip(&durations) {
            if let Some(track) = plan
                .tracks
                .iter()
                .filter(|track| track.enabled && track.subtitle)
                .nth(slot)
            {
                let output = command(ffmpeg)
                    .args(["-nostdin", "-v", "error", "-i"])
                    .arg(&track.path)
                    .args([
                        "-map",
                        &format!("0:{}", track.index),
                        "-c:s",
                        "srt",
                        "-f",
                        "srt",
                        "pipe:1",
                    ])
                    .output()
                    .map_err(|error| error.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).into());
                }
                let clipped = crate::export_tracks::clip_subtitle(
                    &String::from_utf8_lossy(&output.stdout),
                    plan.start,
                    plan.end - plan.start,
                );
                append_subtitles(&mut text, &clipped, offset, &mut number)?;
                languages.push(track.language.as_str());
            }
            offset += duration;
        }
        if !text.is_empty() {
            let path = root.join(format!("subtitle-{slot}.srt"));
            fs::write(&path, text).map_err(|error| error.to_string())?;
            subtitle_tracks.push((path, merged_language(languages.into_iter())));
        }
    }
    *state.lock().unwrap() = (tr("正在合并媒体…").into(), 0.95);
    let list = root.join("concat.txt");
    let contents = pieces
        .iter()
        .map(|path| format!("file '{}'\n", path.file_name().unwrap().to_string_lossy()))
        .collect::<String>();
    fs::write(&list, contents).map_err(|error| error.to_string())?;
    let joined = root.join(format!("merged.{}", settings.extension(video)));
    let mut cmd = command(ffmpeg);
    cmd.args([
        "-nostdin", "-v", "error", "-n", "-f", "concat", "-safe", "1", "-i",
    ])
    .arg(&list);
    for (subtitle, _) in &subtitle_tracks {
        cmd.arg("-i").arg(subtitle);
    }
    cmd.args(["-map", "0", "-c", "copy"]);
    for (slot, (_, language)) in subtitle_tracks.iter().enumerate() {
        let codec = match settings.video.container {
            0 | 3 | 7 => "mov_text",
            1 => "webvtt",
            _ => "srt",
        };
        cmd.args([
            "-map",
            &format!("{}:s:0", slot + 1),
            &format!("-c:s:{slot}"),
            codec,
            &format!("-metadata:s:s:{slot}"),
            &format!("language={language}"),
        ]);
    }
    if video {
        for slot in 0..audio_count {
            let language = merged_language(plans.iter().filter_map(|plan| {
                plan.tracks
                    .iter()
                    .filter(|track| track.enabled && !track.subtitle)
                    .nth(slot)
                    .map(|track| track.language.as_str())
            }));
            cmd.args([
                &format!("-metadata:s:a:{slot}"),
                &format!("language={language}"),
            ]);
        }
    }
    if video && matches!(settings.video.container, 0 | 3 | 7) {
        cmd.args(["-movflags", "+faststart"]);
    }
    cmd.arg(&joined);
    execute(&mut cmd)?;
    let actual = metadata(&probe, &joined)?;
    if video && actual.size != Some([u64::from(settings.size[0]), u64::from(settings.size[1])]) {
        return Err(tr("合并输出尺寸与设置不符。").into());
    }
    // Commit only a completed sibling file so replacing an existing destination remains atomic.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = target.with_file_name(format!(
        ".unity-merge-{}-{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        settings.extension(video)
    ));
    let result = (|| {
        use std::io::Write;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let mut input = fs::File::open(&joined).map_err(|error| error.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
        output
            .flush()
            .and_then(|()| output.sync_all())
            .map_err(|error| error.to_string())?;
        drop(output);
        commit_export(&temporary, target)
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn encode_merge_piece(
    plan: &ExportPlan,
    settings: &MergeSettings,
    target: &Path,
    fps: &str,
    audio_count: usize,
    rate: u32,
    channels: u32,
) -> Result<(), String> {
    let video = plan.video;
    let duration = plan.end - plan.start;
    let selected: Vec<_> = plan
        .tracks
        .iter()
        .filter(|track| track.enabled && !track.subtitle)
        .collect();
    let count = if video {
        selected.len()
    } else {
        usize::from(plan.audio.is_some())
    };
    let mut inputs = vec![plan.source.clone()];
    if video {
        for track in &selected {
            if !inputs.contains(&track.path) {
                inputs.push(track.path.clone());
            }
        }
    }
    let mut cmd = command(&plan.ffmpeg);
    cmd.args(["-nostdin", "-v", "error", "-n"]);
    for path in &inputs {
        cmd.args(["-ss", &plan.start.to_string(), "-t", &duration.to_string()]);
        if gif_source(path) {
            cmd.args(["-ignore_loop", "1"]);
        }
        cmd.arg("-i").arg(path);
    }
    if count < audio_count {
        cmd.args([
            "-f",
            "lavfi",
            "-i",
            &format!(
                "anullsrc=r={rate}:cl={}",
                if channels == 1 { "mono" } else { "stereo" }
            ),
        ]);
    }
    let input_args: Vec<_> = cmd.get_args().map(|arg| arg.to_os_string()).collect();
    let mut encoding = settings.video.clone();
    if video {
        cmd.args(["-map", "0:V:0"]);
        encoding.fps = fps.to_owned();
        encoding.crop = plan.video_settings.crop.without_scaling();
        let [width, height] = settings.size;
        let [r,g,b]=encoding.matte;
        let filter = format!(
            "{},scale={width}:{height}:force_original_aspect_ratio=decrease:force_divisible_by=2:reset_sar=1:flags=lanczos,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=0x{r:02X}{g:02X}{b:02X},setsar=1",
            encoding.spatial_filter("lanczos")
        );
        encoding.configure_with_filter(&mut cmd, Some(&filter));
        if encoding.use_two_pass() {
            let prefix = target.with_extension("pass");
            let logs = PassLogs(prefix.clone());
            let mut pass = command(&plan.ffmpeg);
            pass.args(&input_args).args(["-map", "0:V:0"]);
            encoding.configure_with_filter(&mut pass, Some(&filter));
            pass.args([
                "-an",
                "-sn",
                "-t",
                &duration.to_string(),
                "-pass",
                "1",
                "-passlogfile",
            ])
            .arg(&prefix)
            .args(["-f", "null", "-"]);
            execute(&mut pass)?;
            cmd.args(["-pass", "2", "-passlogfile"]).arg(&prefix);
            // Keep pass logs until the second pass finishes.
            return finish_merge_piece(
                cmd,
                plan,
                settings,
                target,
                selected,
                &inputs,
                count,
                audio_count,
                rate,
                channels,
                Some(logs),
            );
        }
    } else {
        cmd.arg("-vn");
    }
    finish_merge_piece(
        cmd,
        plan,
        settings,
        target,
        selected,
        &inputs,
        count,
        audio_count,
        rate,
        channels,
        None,
    )
}

fn finish_merge_piece(
    mut cmd: std::process::Command,
    plan: &ExportPlan,
    settings: &MergeSettings,
    target: &Path,
    selected: Vec<&crate::export_tracks::Track>,
    inputs: &[PathBuf],
    count: usize,
    audio_count: usize,
    rate: u32,
    channels: u32,
    _logs: Option<PassLogs>,
) -> Result<(), String> {
    for index in 0..audio_count {
        let stream = if index >= count {
            format!("{}:a:0", inputs.len())
        } else if plan.video {
            let track = selected[index];
            let input = inputs.iter().position(|path| *path == track.path).unwrap();
            format!("{input}:{}", track.index)
        } else {
            format!("0:{}", plan.audio.unwrap())
        };
        cmd.args(["-map", &stream]);
        if plan.video {
            let language = if index < count {
                selected[index].language.as_str()
            } else {
                "und"
            };
            cmd.args([
                &format!("-metadata:s:a:{index}"),
                &format!("language={language}"),
            ]);
        }
    }
    if audio_count > 0 {
        if plan.video {
            cmd.args(["-c:a", settings.video.audio_encoder()]);
            if !matches!(settings.video.audio_codec, 2 | 5) {
                cmd.args(["-b:a", &format!("{}k", settings.video.audio_bitrate)]);
            }
            cmd.args([
                "-ar",
                &rate.to_string(),
                "-ac",
                &channels.to_string(),
                "-af",
                "apad",
            ]);
        } else {
            settings.audio.configure(&mut cmd);
            let filter = plan
                .filter
                .as_deref()
                .filter(|filter| !filter.is_empty())
                .map_or("apad".to_owned(), |filter| format!("{filter},apad"));
            cmd.args(["-af", &filter]);
        }
    } else {
        cmd.arg("-an");
    }
    cmd.args([
        "-sn",
        "-dn",
        "-map_metadata",
        "-1",
        "-t",
        &(plan.end - plan.start).to_string(),
    ])
    .arg(target);
    execute(&mut cmd)
}

fn merged_language<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let languages: std::collections::BTreeSet<_> = values
        .filter(|value| !value.is_empty() && *value != "und")
        .collect();
    if languages.len() > 1 {
        "mul".into()
    } else {
        languages.first().copied().unwrap_or("und").to_owned()
    }
}

fn append_subtitles(
    output: &mut String,
    input: &str,
    offset: f64,
    number: &mut usize,
) -> Result<(), String> {
    let input = input.replace("\r\n", "\n");
    for block in input.split("\n\n").filter(|block| !block.trim().is_empty()) {
        let mut lines = block.lines();
        let _index = lines.next();
        let Some(timing) = lines.next() else {
            continue;
        };
        let (start, end) = timing
            .split_once(" --> ")
            .ok_or_else(|| tr("字幕时间格式无效").to_owned())?;
        let parse = |value: &str| {
            super::parse_video_time(&value.replace(',', "."))
                .ok_or_else(|| tr("字幕时间格式无效").to_owned())
        };
        let start = parse(start)? + offset;
        let end = parse(end.split_whitespace().next().unwrap_or(end))? + offset;
        *number += 1;
        output.push_str(&format!(
            "{}\n{} --> {}\n",
            number,
            crate::media_preview::time(start).replace('.', ","),
            crate::media_preview::time(end).replace('.', ",")
        ));
        for line in lines {
            output.push_str(line);
            output.push('\n');
        }
        output.push('\n');
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitles_shift_after_previous_segments() {
        let mut output = String::new();
        let mut number = 0;
        append_subtitles(
            &mut output,
            "1\n00:00:00,200 --> 00:00:00,900\nFirst\n\n",
            0.0,
            &mut number,
        )
        .unwrap();
        append_subtitles(
            &mut output,
            "1\n00:00:00,100 --> 00:00:01,000\nSecond\n\n",
            2.5,
            &mut number,
        )
        .unwrap();
        assert!(output.contains("2\n00:00:02,600 --> 00:00:03,500\nSecond"));
        assert_eq!(number, 2);
    }
}
