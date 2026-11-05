use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Input {
    pub path: PathBuf,
    pub headers: String,
    pub cookies: String,
    pub hls: bool,
    pub local: bool,
    pub stream: Option<u64>,
}

impl Input {
    pub fn new(path: PathBuf) -> Self {
        Self {
            hls: crate::remote_file::is_hls(&path),
            path,
            headers: String::new(),
            cookies: String::new(),
            stream: None,
            local: false,
        }
    }

    fn cached(path: PathBuf) -> Self {
        Self {
            local: true,
            ..Self::new(path)
        }
    }

    pub fn options(&self, command: &mut Command) {
        if self.local {
            return;
        }
        command.args([
            "-protocol_whitelist",
            "http,https,tcp,tls,crypto",
            "-rw_timeout",
            "15000000",
        ]);
        if self.hls {
            command.args(["-f", "hls"]);
        }
        if !self.headers.is_empty() {
            command.arg("-headers").arg(&self.headers);
        }
        if !self.cookies.is_empty() {
            command.arg("-cookies").arg(&self.cookies);
        }
    }
}

#[derive(Clone)]
pub(crate) struct Format {
    pub id: String,
    pub label: String,
    pub input: Option<Input>,
    pub language: String,
    pub separate_audio: bool,
}

pub(crate) struct Source {
    pub url: String,
    pub title: String,
    pub video: Vec<Format>,
    pub audio: Vec<Format>,
    pub selected_video: usize,
    pub selected_audio: usize,
}

pub(crate) fn url_title(path: &Path) -> String {
    path.to_str()
        .and_then(|value| reqwest::Url::parse(value).ok())
        .and_then(|url| {
            url.path_segments()?
                .filter(|part| !part.is_empty())
                .next_back()
                .map(str::to_owned)
        })
        .unwrap_or_else(|| crate::localization::tr("视频").to_owned())
}

pub(crate) fn language_key(language: &str) -> String {
    let base = language
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match base.as_str() {
        "und" | "unknown" | "—" => String::new(),
        "eng" => "en".into(),
        "zho" | "chi" | "cmn" => "zh".into(),
        "jpn" => "ja".into(),
        "kor" => "ko".into(),
        "deu" | "ger" => "de".into(),
        "fra" | "fre" => "fr".into(),
        "spa" => "es".into(),
        "por" => "pt".into(),
        "rus" => "ru".into(),
        _ => base,
    }
}

impl Source {
    pub fn direct(url: String, path: PathBuf, title: String) -> Self {
        Self {
            url,
            title,
            video: vec![Format {
                id: String::new(),
                label: String::new(),
                input: Some(Input::new(path)),
                language: String::new(),
                separate_audio: false,
            }],
            audio: Vec::new(),
            selected_video: 0,
            selected_audio: 0,
        }
    }

    pub fn is_direct_default(&self) -> bool {
        let video = &self.video[self.selected_video];
        video.id.is_empty() && video.input.is_some()
    }

    pub fn selection(&self) -> Selection {
        let video = &self.video[self.selected_video];
        let audio = video
            .separate_audio
            .then(|| self.audio.get(self.selected_audio))
            .flatten();
        Selection {
            url: self.url.clone(),
            title: self.title.clone(),
            video: video.clone(),
            audio: audio.cloned(),
            direct_catalog: video
                .input
                .as_ref()
                .filter(|input| input.stream.is_some())
                .map(|input| DirectCatalog {
                    hls: input.hls,
                    video: self
                        .video
                        .iter()
                        .filter_map(StreamChoice::from_format)
                        .collect(),
                    audio: self
                        .audio
                        .iter()
                        .filter_map(StreamChoice::from_format)
                        .collect(),
                }),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Selection {
    url: String,
    title: String,
    video: Format,
    audio: Option<Format>,
    direct_catalog: Option<DirectCatalog>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct StreamChoice {
    id: String,
    label: String,
    language: String,
    index: u64,
    separate_audio: bool,
}

impl StreamChoice {
    fn from_format(format: &Format) -> Option<Self> {
        Some(Self {
            id: format.id.clone(),
            label: format.label.clone(),
            language: format.language.clone(),
            index: format.input.as_ref()?.stream?,
            separate_audio: format.separate_audio,
        })
    }

    fn into_format(self, base: &Input) -> Format {
        let mut input = base.clone();
        input.stream = Some(self.index);
        Format {
            id: self.id,
            label: self.label,
            input: Some(input),
            language: self.language,
            separate_audio: self.separate_audio,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct DirectCatalog {
    hls: bool,
    video: Vec<StreamChoice>,
    audio: Vec<StreamChoice>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Record {
    title: String,
    video: String,
    video_label: String,
    audio: Option<String>,
    audio_label: Option<String>,
    audio_language: Option<String>,
    separate_audio: bool,
    downloaded: Option<PathBuf>,
    direct_catalog: Option<DirectCatalog>,
}

fn digest(value: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn source_dir(url: &str) -> PathBuf {
    crate::app_cache::network_root()
        .join("video")
        .join(digest(url))
}

impl Selection {
    pub fn inputs(&self) -> Option<(Input, Option<Input>)> {
        let video = self
            .track_cache(Track::Video)
            .map(Input::cached)
            .or_else(|| self.video.input.clone())?;
        let audio = match &self.audio {
            Some(audio) => Some(
                self.track_cache(Track::Audio)
                    .map(Input::cached)
                    .or_else(|| audio.input.clone())?,
            ),
            None => None,
        };
        Some((video, audio))
    }

    pub fn track_cache(&self, track: Track) -> Option<PathBuf> {
        let extensions: &[_] = match track {
            Track::Video => &["mp4", "mkv"],
            Track::Audio => &["m4a", "mka"],
        };
        extensions
            .iter()
            .filter_map(|ext| self.track_file(track, ext))
            .find(|path| path.is_file())
    }

    pub fn track_file(&self, track: Track, extension: &str) -> Option<PathBuf> {
        let (kind, id) = match track {
            Track::Video => (
                if self.video.separate_audio {
                    "picture"
                } else {
                    "muxed"
                },
                &self.video.id,
            ),
            Track::Audio => ("audio", &self.audio.as_ref()?.id),
        };
        Some(
            source_dir(&self.url)
                .join("tracks")
                .join(format!("{kind}-{}.{extension}", digest(id))),
        )
    }

    pub fn split(&self) -> bool {
        self.video.separate_audio
    }
    pub fn fully_cached(&self) -> bool {
        self.track_cache(Track::Video).is_some()
            && (self.audio.is_none() || self.track_cache(Track::Audio).is_some())
    }

    pub fn key(&self) -> String {
        digest(
            serde_json::to_vec(&(&self.video.id, self.audio.as_ref().map(|audio| &audio.id)))
                .unwrap(),
        )
    }

    fn dir(&self) -> PathBuf {
        source_dir(&self.url).join(self.key())
    }

    pub fn downloaded(&self) -> Option<PathBuf> {
        self.download_record().filter(|path| path.is_file())
    }

    pub fn download_record(&self) -> Option<PathBuf> {
        read_record(&self.dir()).and_then(|record| record.downloaded)
    }

    pub fn remember(&self, downloaded: Option<PathBuf>) -> Result<(), String> {
        let dir = self.dir();
        fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let record = Record {
            title: self.title.clone(),
            video: self.video.id.clone(),
            video_label: self.video.label.clone(),
            audio: self.audio.as_ref().map(|audio| audio.id.clone()),
            audio_label: self.audio.as_ref().map(|audio| audio.label.clone()),
            audio_language: self.audio.as_ref().map(|audio| audio.language.clone()),
            separate_audio: self.video.separate_audio,
            direct_catalog: self.direct_catalog.clone(),
            downloaded: downloaded
                .or_else(|| read_record(&dir).and_then(|record| record.downloaded)),
        };
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temporary = dir.join(format!(
            "record-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let bytes = serde_json::to_vec(&record).map_err(|error| error.to_string())?;
        let result = fs::write(&temporary, bytes)
            .map_err(|error| error.to_string())
            .and_then(|()| crate::playlist::replace(&temporary, &dir.join("record.json")));
        let _ = fs::remove_file(temporary);
        result
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Track {
    Video,
    Audio,
}

fn read_record(dir: &Path) -> Option<Record> {
    serde_json::from_slice(&fs::read(dir.join("record.json")).ok()?).ok()
}

pub(crate) fn cached_source(url: &str) -> Option<Source> {
    let mut records: Vec<_> = fs::read_dir(source_dir(url))
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let record = read_record(&entry.path())?;
            let modified = fs::metadata(entry.path().join("record.json"))
                .ok()?
                .modified()
                .ok()?;
            Some((modified, record))
        })
        .collect();
    records.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    let mut source = Source {
        url: url.to_owned(),
        title: String::new(),
        video: Vec::new(),
        audio: Vec::new(),
        selected_video: 0,
        selected_audio: 0,
    };
    for (_, record) in records {
        let video = Format {
            id: record.video,
            label: record.video_label,
            input: None,
            language: String::new(),
            separate_audio: record.separate_audio,
        };
        let audio = record.audio.map(|id| Format {
            id,
            label: record.audio_label.unwrap_or_default(),
            input: None,
            language: record.audio_language.unwrap_or_default(),
            separate_audio: false,
        });
        let selection = Selection {
            url: url.to_owned(),
            title: record.title,
            video,
            audio,
            direct_catalog: None,
        };
        if selection.downloaded().is_none() && !selection.fully_cached() {
            continue;
        }
        if source.video.is_empty() {
            source.title = selection.title.clone();
            if let Some(catalog) = record
                .direct_catalog
                .filter(|catalog| !catalog.video.is_empty())
            {
                let mut base = Input::new(PathBuf::from(url));
                base.hls = catalog.hls;
                source.selected_video = catalog
                    .video
                    .iter()
                    .position(|format| format.id == selection.video.id)
                    .unwrap_or_default();
                source.selected_audio = selection
                    .audio
                    .as_ref()
                    .and_then(|audio| {
                        catalog
                            .audio
                            .iter()
                            .position(|format| format.id == audio.id)
                    })
                    .unwrap_or_default();
                source.video = catalog
                    .video
                    .into_iter()
                    .map(|format| format.into_format(&base))
                    .collect();
                source.audio = catalog
                    .audio
                    .into_iter()
                    .map(|format| format.into_format(&base))
                    .collect();
                return Some(source);
            }
        }
        if !source
            .video
            .iter()
            .any(|format| format.id == selection.video.id)
        {
            source.video.push(selection.video);
        }
        if let Some(audio) = selection.audio {
            if !source.audio.iter().any(|format| format.id == audio.id) {
                source.audio.push(audio);
            }
        }
    }
    (!source.video.is_empty()).then_some(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_picture_across_languages_and_audio_across_resolutions() {
        let format = |id: &str, language: &str, separate_audio| Format {
            id: id.to_owned(),
            label: String::new(),
            input: None,
            language: language_key(language),
            separate_audio,
        };
        let mut source = Source {
            url: "https://example.invalid/video".into(),
            title: "Test".into(),
            video: vec![format("720p", "", true), format("1080p", "", true)],
            audio: vec![
                format("en-128", "en-US", false),
                format("en-256", "eng", false),
                format("ja", "ja", false),
            ],
            selected_video: 0,
            selected_audio: 0,
        };
        let first = source.selection();
        source.selected_audio = 2;
        let other_language = source.selection();
        assert_ne!(first.key(), other_language.key());
        assert_eq!(
            first.track_file(Track::Video, "mp4"),
            other_language.track_file(Track::Video, "mp4")
        );
        assert_ne!(
            first.track_file(Track::Audio, "m4a"),
            other_language.track_file(Track::Audio, "m4a")
        );
        source.selected_video = 1;
        assert_eq!(
            other_language.track_file(Track::Audio, "m4a"),
            source.selection().track_file(Track::Audio, "m4a")
        );
    }
}
