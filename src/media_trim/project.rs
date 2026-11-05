use super::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct EditState {
    start: f64,
    end: f64,
    trim_playback: bool,
    #[serde(default)]
    boundary_locks: [bool;2],
    settings: AudioSettings,
    video_settings: VideoSettings,
    pub tracks: Vec<crate::export_tracks::Track>,
    pub audio_state: Option<crate::audio_edit::State>,
}

impl EditState {
    #[cfg(test)]
    pub(crate) fn fixture() -> Self { Self::from_merge(VideoSettings::default(), AudioSettings::default()) }
    pub(super) fn from_merge(video_settings: VideoSettings, settings: AudioSettings) -> Self {
        Self { start:0.0,end:0.0,trim_playback:false,boundary_locks:[false;2],video_settings,settings,tracks:vec![],audio_state:None }
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        let v = &self.video_settings;
        if !self.start.is_finite() || !self.end.is_finite() || self.start < 0.0 || self.end < 0.0
            || v.container >= VideoSettings::CONTAINERS.len() || v.codec >= VideoSettings::CODECS.len()
            || v.audio_codec >= VideoSettings::AUDIO.len() || v.gif_filter >= VideoSettings::GIF_FILTERS.len()
            || v.gif_dither >= VideoSettings::DITHER.len() || !v.compatible(v.codec)
            || !VideoSettings::FPS.iter().any(|(id,_)| *id == v.fps)
            || self.settings.channels == 0 || self.settings.channels > 8 || self.settings.rate == 0
        { return Err(tr("工程设置无效或格式版本不受支持。").into()); }
        v.crop.validate_project()?;
        if let Some(s) = &self.audio_state {
            if ![s.start,s.end,s.db,s.fade_in,s.fade_out].iter().all(|n|n.is_finite())
                || !matches!(s.mode.as_str(), "ends" | "interval") || s.start < 0.0 || s.end < 0.0
            { return Err(tr("工程设置无效或格式版本不受支持。").into()); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_settings_round_trip_and_reject_unknown_indexes() {
        let mut state=EditState::fixture();state.start=12.345;state.end=67.89;state.boundary_locks=[true,false];
        state.video_settings.crop.prepare([1920,1080]);
        let bytes=serde_json::to_vec(&state).unwrap();
        let restored:EditState=serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();assert_eq!(restored.start,12.345);assert_eq!(restored.end,67.89);
        assert_eq!(restored.boundary_locks,[true,false]);
        let mut legacy=serde_json::to_value(&restored).unwrap();legacy.as_object_mut().unwrap().remove("boundary_locks");
        assert_eq!(serde_json::from_value::<EditState>(legacy).unwrap().boundary_locks,[false,false]);
        let mut value=serde_json::to_value(restored).unwrap();value["video_settings"]["codec"]=serde_json::json!(999);
        assert!(serde_json::from_value::<EditState>(value).unwrap().validate().is_err());
    }
}

impl Trim {
    pub(crate) fn project_state(&self, video: bool) -> Result<EditState, String> {
        if self.pending.is_some() || self.fade_dialog.is_some() || self.gain_dialog
            || video && !self.video_tracks.ready()
            || !video && self.session.as_ref().is_none_or(|session| !session.ready())
        { return Err(tr("请等待编辑处理完成并关闭参数弹窗后保存工程。").into()); }
        Ok(EditState { start:self.start,end:self.end,trim_playback:self.trim_playback,boundary_locks:self.boundary_locks,
            settings:self.settings.clone(),video_settings:self.video_settings.clone(),
            tracks:self.video_tracks.entries.clone(),audio_state:self.session.as_ref().map(|s|s.state.clone()) })
    }

    pub(crate) fn restore_project(&mut self, state: EditState, clip: Option<(&Path,f64)>, ffmpeg: PathBuf, channels: usize) -> Result<(), String> {
        state.validate()?;
        self.start=state.start; self.end=state.end; self.trim_playback=state.trim_playback;self.boundary_locks=state.boundary_locks;
        self.settings=state.settings; self.video_settings=state.video_settings;
        self.video_tracks=crate::export_tracks::Tracks::from_project(state.tracks);
        if let Some(audio_state)=state.audio_state {
            let (path,duration)=clip.ok_or_else(||tr("工程中的音频编辑数据缺失。").to_owned())?;
            self.session=Some(crate::audio_edit::Session::from_project(audio_state,path,duration,ffmpeg,channels)?);
        }
        Ok(())
    }
}
