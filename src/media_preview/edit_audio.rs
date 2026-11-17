//! Preview identities include the source file; stream indices are only local to a file.
use super::{AudioTrack, Path, PathBuf};
use crate::{export_tracks::Track, localization::tr};

#[derive(Clone, Debug, Eq)]
pub(crate) struct Source {
    pub path: PathBuf,
    pub index: u64,
    occurrence: usize,
}
impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.occurrence == other.occurrence && same_path(&self.path, &other.path)
    }
}
impl Source {
    pub fn new(path: &Path, index: u64) -> Self {
        Self { path: path.to_owned(), index, occurrence: 0 }
    }
    pub fn from_entry(tracks: &[Track], index: usize) -> Option<Self> {
        Self::entry(tracks, index, false)
    }
    pub fn subtitle_entry(tracks: &[Track], index: usize) -> Option<Self> {
        Self::entry(tracks, index, true)
    }
    fn entry(tracks: &[Track], index: usize, subtitle: bool) -> Option<Self> {
        let track = tracks.get(index).filter(|track| track.subtitle == subtitle)?;
        let occurrence = tracks[..index].iter().filter(|other|
            other.subtitle == subtitle && other.index == track.index && same_path(&other.path, &track.path)).count();
        Some(Self { path: track.path.clone(), index: track.index, occurrence })
    }
    pub fn entry_index(&self, tracks: &[Track]) -> Option<usize> {
        (0..tracks.len()).find(|index| Self::from_entry(tracks, *index).as_ref() == Some(self))
    }
    pub fn subtitle_index(&self, tracks: &[Track]) -> Option<usize> {
        (0..tracks.len()).find(|index| Self::subtitle_entry(tracks, *index).as_ref() == Some(self))
    }
}
pub(crate) fn same_path(first: &Path, second: &Path) -> bool {
    if cfg!(windows) {
        let first = first.to_string_lossy().replace('/', "\\");
        let second = second.to_string_lossy().replace('/', "\\");
        first.strip_prefix("\\\\?\\").unwrap_or(&first)
            .eq_ignore_ascii_case(second.strip_prefix("\\\\?\\").unwrap_or(&second))
    } else { first == second }
}

pub(crate) struct Choice {
    pub source: Source,
    pub ordinal: usize,
    pub label: String,
    pub details: String,
    pub available: bool,
}
pub(crate) fn choices(path: &Path, native: &[AudioTrack], tracks: &[Track], loading_source: bool) -> Vec<Choice> {
    if tracks.is_empty() && loading_source {
        return native.iter().map(|track| Choice {
            source: Source::new(path, track.index), ordinal: track.ordinal,
            label: track.display_label(), details: track.label.clone(), available: true,
        }).collect();
    }
    tracks.iter().enumerate().filter(|(_, track)| !track.subtitle).enumerate().map(|(ordinal, (index, track))| {
        let ordinal = ordinal + 1;
        let name = track.path.file_name().unwrap_or_default().to_string_lossy();
        let language = super::audio_language_name(&track.title, &track.language)
            .map(tr).unwrap_or(&track.language);
        let label = format!("{} · {name} · {language} · #{} · {}", crate::export_tracks::numbered_label(false,ordinal), track.index, track.codec);
        Choice {
            source: Source::from_entry(tracks, index).unwrap(), ordinal,
            details: format!("{label}\n{}", track.path.display()), label, available: true,
        }
    }).collect()
}
pub(crate) fn reconcile(selected: Option<Source>, native: Option<Source>, choices: &[Choice]) -> Option<Source> {
    selected.filter(|source| choices.iter().any(|choice| &choice.source == source))
        .or_else(|| native.filter(|source| choices.iter().any(|choice| &choice.source == source)))
        .or_else(|| choices.first().map(|choice| choice.source.clone()))
}
pub(crate) fn subtitle_choices(tracks: &[Track]) -> Vec<Choice> {
    tracks.iter().enumerate().filter(|(_, track)| track.subtitle).enumerate().map(|(ordinal, (index, track))| {
        let ordinal=ordinal+1;
        let label=format!("{} · {} · {} · #{} · {}",crate::export_tracks::numbered_label(true,ordinal),
            track.path.file_name().unwrap_or_default().to_string_lossy(),track.language,track.index,track.codec);
        Choice {source:Source::subtitle_entry(tracks,index).unwrap(),ordinal,details:format!("{label}\n{}",track.path.display()),label,
            available:matches!(track.codec.as_str(),"subrip"|"srt"|"ass"|"ssa"|"webvtt"|"mov_text"|"text")}
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(path: &str, index: u64, subtitle: bool) -> Track {
        Track { path: path.into(), index, subtitle, codec: "aac".into(), language: "und".into(),
            title: String::new(), enabled: true, default: false, forced: false }
    }
    #[test]
    fn added_audio_uses_file_identity_and_contiguous_audio_numbers() {
        let mut tracks = vec![track("movie.mp4", 0, false), track("movie.mp4", 2, true), track("music.mp3", 0, false)];
        tracks[2].enabled = false; // Export selection is independent of preview.
        let choices = choices(Path::new("movie.mp4"), &[], &tracks, false);
        assert_eq!(choices.iter().map(|choice| choice.ordinal).collect::<Vec<_>>(), vec![1, 2]);
        assert_ne!(choices[0].source, choices[1].source);
        assert_eq!(choices[1].source.entry_index(&tracks), Some(2));
        assert_eq!(choices[1].source.path, Path::new("music.mp3"));
    }
    #[test]
    fn removal_reconciles_to_native_or_remaining_audio_and_can_clear_selection() {
        let tracks = vec![track("movie.mp4", 1, false), track("music.mp3", 0, false)];
        let native = Source::new(Path::new("movie.mp4"), 1);
        let external = Source::new(Path::new("music.mp3"), 0);
        let all = choices(Path::new("movie.mp4"), &[], &tracks, false);
        assert_eq!(reconcile(Some(external.clone()), Some(native.clone()), &all), Some(external.clone()));
        let remaining = choices(Path::new("movie.mp4"), &[], &tracks[..1], false);
        assert_eq!(reconcile(Some(external), Some(native.clone()), &remaining), Some(native.clone()));
        assert_eq!(reconcile(Some(native), None, &[]), None);
    }
    #[test]
    fn duplicate_streams_stay_distinct_and_project_selection_survives_path_relocation() {
        let mut tracks = vec![track("music.mp3", 0, false), track("music.mp3", 0, false)];
        let first = Source::from_entry(&tracks, 0).unwrap();
        let second = Source::from_entry(&tracks, 1).unwrap();
        assert_ne!(first, second);
        let selected = second.entry_index(&tracks).unwrap();
        for track in &mut tracks { track.path = PathBuf::from("unpacked/music.mp3"); }
        let restored = Source::from_entry(&tracks, selected).unwrap();
        assert_eq!(restored.path, Path::new("unpacked/music.mp3"));
        assert_eq!(restored.entry_index(&tracks), Some(1));
    }
}
