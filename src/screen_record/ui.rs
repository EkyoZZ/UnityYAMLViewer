use super::*;
use preferences::{Preferences, Shortcuts};

fn pip_area(display: &str, monitors: &[displays::Display]) -> Area {
    if display == "all" { return screen(true); }
    monitors.iter().find(|monitor| monitor.key == display).map_or_else(|| screen(false), |monitor| monitor.area)
}
struct Preparation {
    receiver: mpsc::Receiver<(Option<PathBuf>, Vec<&'static str>)>,
    area: Area,
}

pub struct Recorder {
    open: bool,
    mode: usize,
    monitor: String,
    area: Area,
    pending_select: bool,
    outline: Option<outline::Outline>,
    outline_editable: bool,
    toolbar: toolbar::View,
    countdown: countdown::Countdown,
    toolbar_actions: Vec<outline::Action>,
    history: history::History,
    shortcuts: Option<Shortcuts>,
    shortcuts_ready: bool,
    preferences: Preferences,
    draft: Preferences,
    settings_open: bool,
    applying: bool,
    settings_error: Option<String>,
    devices: camera::Devices,
    camera_scan: Option<mpsc::Receiver<Result<camera::Devices, String>>>,
    camera_preview: Option<camera::Preview>,
    pip_preview: pip::View,
    pip_preview_settings: Option<camera::Settings>,
    system_muted: bool,
    microphone_muted: bool,
    job: Option<Job>,
    result: Option<PathBuf>,
    result_open: bool,
    error: Option<String>,
    logged_error: Option<String>,
    log_open_error: Option<String>,
    restore: bool,
    preparing: Option<Preparation>,
    missing_tools: Option<Vec<&'static str>>,
    exit_requested: bool,
}
impl Default for Recorder {
    fn default() -> Self {
        let loaded = Preferences::load();
        let settings_error = loaded.as_ref().err().cloned();
        let preferences = loaded.unwrap_or_default();
        Self {
            open: false,
            mode: 2,
            monitor: String::new(),
            area: screen(false),
            pending_select: false,
            outline: None,
            outline_editable: false,
            toolbar: toolbar::View::default(),
            countdown: countdown::Countdown::default(),
            toolbar_actions: vec![],
            history: history::History::load(preferences.history_limit),
            shortcuts: None,
            shortcuts_ready: false,
            draft: preferences.clone(),
            preferences,
            settings_open: false,
            applying: false,
            settings_error,
            devices: Default::default(),
            camera_scan: None,
            camera_preview: None,
            pip_preview: Default::default(),
            pip_preview_settings: None,
            system_muted: false,
            microphone_muted: false,
            job: None,
            result: None,
            result_open: false,
            error: None,
            logged_error: None,
            log_open_error: None,
            restore: false,
            preparing: None,
            missing_tools: None,
            exit_requested: false,
        }
    }
}
impl Recorder {
    pub fn open(&mut self) {
        if self.busy() {
            return;
        }
        self.open = true;
        self.result = None;
        self.exit_requested = false;
        self.mode = 2;
        self.pending_select = self.preferences.camera.source == camera::Source::Screen;
    }
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.preparing.is_some()
    }
    fn stop(&mut self) {
        self.preparing = None;
        if let Some(job) = &mut self.job {
            if !job.stopping {
                let _ = job.stop.send(());
                job.stopping = true;
            }
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        if self.shortcuts.is_none() {
            self.shortcuts = Some(Shortcuts::new(ctx, self.preferences.clone()));
        }
        let mut shortcut_events = vec![];
        let mut disconnected = false;
        if let Some(shortcuts) = &self.shortcuts {
            loop {
                match shortcuts.events.try_recv() {
                    Ok(event) => shortcut_events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        for event in shortcut_events {
            match event {
                preferences::Event::Stop => {
                    if !self.settings_open {
                        self.stop();
                    }
                }
                preferences::Event::Select => {
                    if !self.busy() && !self.settings_open && !self.history.open {
                        self.open();
                    }
                }
                preferences::Event::Applied(result, ready) => {
                    self.shortcuts_ready = ready;
                    match result {
                        Ok(preferences) => {
                            self.history.set_limit(preferences.history_limit);
                            if self.preferences.camera.source != preferences.camera.source {
                                self.camera_preview = None;
                                self.pip_preview.open = false;
                                self.pip_preview.release();
                                self.pip_preview_settings = None;
                                self.pending_select = self.open && preferences.camera.source == camera::Source::Screen && self.mode == 2;
                            }
                            self.preferences = preferences;
                            self.draft = self.preferences.clone();
                            if self.applying {
                                self.settings_error = None;
                                self.settings_open = false;
                            }
                        }
                        Err(error) => self.settings_error = Some(error),
                    }
                    self.applying = false;
                }
            }
        }
        if disconnected {
            self.shortcuts_ready = false;
            self.applying = false;
            self.settings_error = Some(tr("录屏快捷键服务已退出，请重新启动程序。").into());
            self.stop();
        }
        if self.open && self.outline.is_none() {
            self.outline = Some(outline::Outline::new(ctx));
        }
        let mut actions = std::mem::take(&mut self.toolbar_actions);
        let busy = self.busy();
        if let Some(outline) = &mut self.outline {
            if let Some(area) = outline.poll() {
                if self.mode == 2 && !busy {
                    self.area = area;
                }
            }
            actions.extend(std::mem::take(&mut outline.actions));
        }
        let monitors = displays::list();
        if !monitors.iter().any(|display| display.key == self.monitor) {
            self.monitor = monitors
                .iter()
                .find(|display| display.primary)
                .or_else(|| monitors.first())
                .map(|display| display.key.clone())
                .unwrap_or_default();
        }
        if monitors.len() < 2 && self.mode == 1 {
            self.mode = 0;
        }
        let mut finished = None;
        if let Some(job) = &mut self.job {
            loop {
                match job.events.try_recv() {
                    Ok(Event::Countdown(ready)) => job.countdown = Some(countdown::Request::new(ready)),
                    Ok(Event::Starting(ready)) => {
                        job.countdown = None;
                        job.capture_started = true;
                        if job.pip.is_some() {
                            self.pip_preview.handoff();
                        }
                        if let Some(frames) = job.camera_frames.take()
                            && let Some(preview) = &mut self.camera_preview
                        {
                            preview.start_live(frames);
                        }
                        let _ = ready.send(());
                    }
                    Ok(Event::CountdownComplete) => job.countdown = None,
                    Ok(Event::Recording) => job.started = Some(Instant::now()),
                    Ok(Event::Stopping) => job.stopping = true,
                    Ok(Event::Finished(result)) => {
                        finished = Some(result);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = Some(Err(tr("录屏任务意外中断").into()));
                        break;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if let Some(result) = finished {
            self.job = None;
            if self.preferences.camera.source == camera::Source::Camera {
                self.camera_preview = None;
            }
            // Keep the recorded area click-through until the user selects a region again.
            self.outline_editable = false;
            self.open = !self.exit_requested;
            if self.restore {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                self.restore = false;
            }
            match result {
                Ok(path) => {
                    self.history.record(path.clone());
                    self.result = Some(path);
                    self.error = None;
                    self.result_open = false;
                }
                Err(error) => {
                    // A window initialization error may have requested cancellation.
                    // Keep that original cause rather than replacing it with "canceled".
                    if self.error.is_none() {
                        self.error = Some(error);
                    }
                    self.result_open = true;
                }
            }
        }
        let prepared = self
            .preparing
            .as_ref()
            .and_then(|job| match job.receiver.try_recv() {
                Ok(result) => Some(Ok((job.area, result))),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(tr("检查录屏组件失败。").to_owned()))
                }
            });
        if let Some(result) = prepared {
            self.preparing = None;
            match result {
                Ok((area, (Some(ffmpeg), missing))) if missing.is_empty() => {
                    self.begin_record(ctx, area, ffmpeg)
                }
                Ok((_, (_, missing))) => self.missing_tools = Some(missing),
                Err(error) => {
                    self.error = Some(error);
                    self.result_open = true;
                }
            }
        }
        let mut edit = None;
        if self.busy() && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.stop();
        }
        for action in actions {
            match action {
                outline::Action::ErrorDetails => {
                    self.result_open = true;
                    crate::native_window::wake(ctx, egui::Id::new("screen_record_result"));
                }
                outline::Action::ToggleSystemAudio => {
                    self.toggle_audio(0);
                }
                outline::Action::ToggleMicrophone => {
                    self.toggle_audio(1);
                }
                outline::Action::Toggle => {
                    if self.busy() {
                        self.stop();
                    } else {
                        self.start(ctx);
                    }
                }
                outline::Action::Settings => {
                    if !self.busy() {
                        if let Some(outline) = &mut self.outline {
                            outline.hide();
                        }
                        self.pending_select = false;
                        self.draft = self.preferences.clone();
                        self.settings_open = true;
                        crate::native_window::wake(ctx, egui::Id::new("screen_record_settings"));
                    }
                }
                outline::Action::Close => {
                    self.exit_requested = true;
                    self.preparing = None;
                    if self.job.is_some() {
                        self.stop();
                    } else {
                        self.open = false;
                        self.pending_select = false;
                    }
                }
                outline::Action::Edit => {
                    if !self.busy() {
                        edit = self.result.clone();
                    }
                }
                outline::Action::OpenFolder => {
                    if let Some(path) = &self.result {
                        let _ = command("explorer.exe").arg("/select,").arg(path).spawn();
                    }
                }
                outline::Action::History => {
                    if let Some(outline) = &mut self.outline {
                        if outline.selecting {
                            outline.hide();
                        }
                    }
                    self.pending_select = false;
                    self.history.open = true;
                    crate::native_window::wake(ctx, egui::Id::new("screen_record_history"));
                }
                outline::Action::SetArea(area) => {
                    if !self.busy() && area.valid(screen(true)) {
                        self.mode = 2;
                        self.area = area;
                        self.pending_select = false;
                    }
                }
                outline::Action::Choose(index) => {
                    if !self.busy() {
                        let camera_index = monitors.len() + 1 + usize::from(monitors.len() > 1);
                        if index >= camera_index {
                            let source = if index == camera_index { camera::Source::Camera } else { camera::Source::Combined };
                            self.switch_preview_source(ctx, source);
                            self.pending_select = false;
                            continue;
                        }
                        self.select_recording_source(ctx, camera::Source::Screen);
                        self.camera_preview = None;
                        self.pip_preview.open = false;
                        self.pip_preview.release();
                        self.history.open = false;
                        self.result = None;
                        if index != 0 {
                            if let Some(outline) = &mut self.outline {
                                outline.hide();
                            }
                        }
                        if index == 0 {
                            self.mode = 2;
                            self.pending_select = true;
                        } else if let Some(display) = monitors.get(index - 1) {
                            self.mode = 0;
                            self.monitor = display.key.clone();
                            self.pending_select = false;
                        } else if index == monitors.len() + 1 && monitors.len() > 1 {
                            self.mode = 1;
                            self.pending_select = false;
                        }
                    }
                }
            }
        }
        if self.preferences.camera.source == camera::Source::Screen
            && self.pending_select && !self.settings_open && !self.history.open {
            if let Some(outline) = &mut self.outline {
                if outline.ready {
                    outline.select(self.area);
                    self.outline_editable = true;
                    self.pending_select = false;
                }
            }
        }
        if let Some(error) = self
            .outline
            .as_ref()
            .and_then(|outline| outline.error.as_ref())
        {
            if self.error.as_ref() != Some(error) {
                self.error = Some(error.clone());
                self.result_open = true;
            }
        }
        self.show_settings(ctx);
        let previous_mirror = self.preferences.camera.mirrored;
        let switch_preview = !self.busy() && !self.applying;
        let mut preview_source = None;
        let mut preview_settings = false;
        let mut audio_toggles = [false; 2];
        let audio = [
            self.preferences.system_audio.then_some(!self.system_muted),
            self.preferences.microphone.then_some(!self.microphone_muted),
        ];
        let mut toggle_camera_recording = false;
        let status = self.recording_status();
        let stop_label = self.preferences.stop.label();
        let result = self.result.clone();
        let mut footer = pip::Footer {
            stop_label: &stop_label,
            status: &status,
            result: result.as_deref(),
            history_requested: false,
        };
        let camera_mode = self.preferences.camera.source == camera::Source::Camera;
        let camera_recording = camera_mode && self.job.as_ref().is_some_and(|job| job.started.is_some());
        let camera_control = if self.job.as_ref().is_some_and(|job| job.stopping) {
            pip::RecordControl::Stopping
        } else if self.busy() && camera_mode {
            pip::RecordControl::Stop
        } else if switch_preview && !self.settings_open && self.shortcuts_ready && camera_mode {
            pip::RecordControl::Start
        } else {
            pip::RecordControl::Unavailable
        };
        if !self.open && !self.settings_open {
            self.camera_preview = None;
        } else if self.open && !self.settings_open && !self.busy() && camera_mode && self.camera_preview.is_none() {
            self.open_camera_preview(ctx);
        }
        if self.camera_preview.is_some() {
            let settings = if self.settings_open { &mut self.draft.camera } else { &mut self.preferences.camera };
            if !settings.device.is_empty() {
                if switch_preview && self.camera_preview.as_ref().is_some_and(|preview| !preview.matches(settings)) {
                    self.camera_preview = None;
                    match camera::Preview::open(ctx, settings) {
                        Ok(preview) => self.camera_preview = Some(preview),
                        Err(error) => self.settings_error = Some(error),
                    }
                }
                let countdown = self.job.as_mut()
                    .filter(|job| camera_mode && !job.stopping)
                    .and_then(|job| job.countdown.as_mut());
                if self.camera_preview.as_mut().is_some_and(|preview| !preview.show(ctx, settings,
                    switch_preview, &mut preview_source, camera_control, camera_recording, &mut toggle_camera_recording,
                    &mut preview_settings, audio, &mut audio_toggles, &mut footer, countdown)) {
                    self.camera_preview = None;
                    if camera_mode && !self.settings_open {
                        self.open = false;
                    }
                }
            } else {
                self.camera_preview = None;
            }
        }
        let pip_mode = self.preferences.camera.source == camera::Source::Combined;
        if self.open && pip_mode && !self.settings_open && !self.busy() && !self.pip_preview.open {
            if self.preferences.camera.device.is_empty() {
                self.select_recording_source(ctx, camera::Source::Combined);
            } else {
                self.pip_preview.open = true;
                crate::native_window::wake(ctx, pip::View::id());
            }
        }
        let pip_was_open = self.pip_preview.open;
        let pip_control = if self.job.as_ref().is_some_and(|job| job.stopping) {
            pip::RecordControl::Stopping
        } else if self.busy() {
            pip::RecordControl::Stop
        } else if !self.settings_open && self.shortcuts_ready
            && self.preferences.camera.source == camera::Source::Combined {
            pip::RecordControl::Start
        } else {
            pip::RecordControl::Unavailable
        };
        let mut toggle_pip_recording = false;
        let previous_display = self.preferences.camera.display.clone();
        if let Some(job) = &mut self.job {
            if let Some(live) = &mut job.pip {
                let picture = job.capture_started.then_some((&mut live.picture, live.position.as_ref()));
                toggle_pip_recording = self.pip_preview.show(ctx, job.area, &monitors, &mut live.settings,
                    picture, job.capture_started, pip_control, false, &mut preview_source,
                    &mut preview_settings, audio, &mut audio_toggles, &mut footer, job.countdown.as_mut());
            }
        } else if self.open || self.settings_open {
            let settings = if self.settings_open { &mut self.draft.camera } else { &mut self.preferences.camera };
            if settings.source == camera::Source::Combined && !settings.device.is_empty() {
                let area = pip_area(&settings.display, &monitors);
                toggle_pip_recording = self.pip_preview.show(ctx, area, &monitors, settings, None, false, pip_control, switch_preview, &mut preview_source,
                    &mut preview_settings, audio, &mut audio_toggles, &mut footer, None);
                if self.pip_preview.open { self.pip_preview_settings = Some(settings.clone()); }
            } else if let Some(preview) = &mut self.pip_preview_settings {
                let area = pip_area(&preview.display, &monitors);
                self.pip_preview.show(ctx, area, &monitors, preview, None, false, pip::RecordControl::Unavailable, switch_preview, &mut preview_source,
                    &mut preview_settings, audio, &mut audio_toggles, &mut footer, None);
            }
        } else {
            self.pip_preview.open = false;
            self.pip_preview.release();
        }
        if self.preferences.camera.display != previous_display || self.preferences.camera.mirrored != previous_mirror {
            if let Err(error) = self.preferences.save() {
                self.preferences.camera.display = previous_display;
                self.preferences.camera.mirrored = previous_mirror;
                self.pip_preview_settings = Some(self.preferences.camera.clone());
                self.error = Some(error);
            }
        }
        for (index, toggle) in audio_toggles.into_iter().enumerate() {
            if toggle { self.toggle_audio(index); }
        }
        if preview_settings && !self.settings_open && !self.busy() {
            self.draft = self.preferences.clone();
            self.settings_open = true;
            crate::native_window::wake(ctx, egui::Id::new("screen_record_settings"));
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        } else if let Some(source) = preview_source {
            self.switch_preview_source(ctx, source);
        } else if toggle_pip_recording || toggle_camera_recording {
            if self.busy() { self.stop(); } else { self.start(ctx); }
        }
        if !self.pip_preview.open {
            self.pip_preview_settings = None;
            if pip_was_open && pip_mode && self.preferences.camera.source == camera::Source::Combined
                && !self.settings_open && !self.busy()
            {
                self.open = false;
            }
        }
        if footer.history_requested {
            self.history.open = true;
            crate::native_window::wake(ctx, egui::Id::new("screen_record_history"));
        }
        if let Some(path) = self.history.show(ctx) {
            edit = Some(path);
        }
        self.show_missing_tools(ctx);
        if self.error != self.logged_error {
            if let Some(error) = &self.error {
                crate::error_log::error("screen_record/ui", error);
                self.result_open = true;
                self.log_open_error = None;
                crate::native_window::wake(ctx, egui::Id::new("screen_record_result"));
            }
            self.logged_error = self.error.clone();
        }
        if let Some(request) = self.show_result(ctx) {
            edit = Some(request);
        }
        self.sync_outline(ctx, &monitors);
        edit
    }
    fn show_missing_tools(&mut self, ctx: &egui::Context) {
        let Some(missing) = &self.missing_tools else {
            return;
        };
        let mut open = true;
        let mut install = false;
        let mut cancel = false;
        crate::native_window::Window::new(tr("缺少录屏组件"))
            .id(egui::Id::new("screen_record_missing_tools"))
            .open(&mut open)
            .default_width(420.0)
            .default_height(190.0)
            .always_on_top()
            .show(ctx, |ui| {
                ui.label(tr("需要先安装以下组件，点击确定进入第三方支持："));
                for tool in missing {
                    ui.label(format!("• {tool}"));
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("取消")).clicked() {
                        cancel = true;
                    }
                    if ui.button(tr("确定")).clicked() {
                        install = true;
                    }
                });
            });
        if install {
            crate::third_party::request_open(ctx);
        }
        if !open || install || cancel {
            self.missing_tools = None;
        }
    }
    fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        if let Some(result) = self.camera_scan.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.camera_scan = None;
            match result {
                Ok(devices) => {
                    self.devices = devices;
                    self.settings_error = None;
                }
                Err(error) => self.settings_error = Some(error),
            }
        }
        crate::native_window::Window::new(tr("录屏设置")).id(egui::Id::new("screen_record_settings")).open(&mut open).default_width(480.0).default_height(570.0).resizable(true).always_on_top().show(ctx,|ui|{
            ui.small(tr("自动保存到用户下载目录，使用时间戳命名。"));
            ui.separator();
            ui.add_enabled_ui(!self.applying && !self.busy(),|ui|{
                self.device_settings(ui);
                ui.horizontal(|ui|{
                    ui.label(tr("帧率"));ui.add(egui::DragValue::new(&mut self.draft.fps).range(1..=60));
                    egui::ComboBox::from_id_salt("record_quality").selected_text(tr(["高质量","均衡","小体积"][self.draft.quality.min(2)])).show_ui(ui,|ui|{
                        for (index,label) in ["高质量","均衡","小体积"].iter().enumerate(){ui.selectable_value(&mut self.draft.quality,index,tr(label));}
                    });
                    egui::ComboBox::from_id_salt("record_container").selected_text(if self.draft.mp4{"MP4"}else{"MKV"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.draft.mp4,false,"MKV");ui.selectable_value(&mut self.draft.mp4,true,"MP4");});
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.draft.system_audio, tr("录制系统声音"));
                    ui.checkbox(&mut self.draft.microphone, tr("录制麦克风"));
                });
                ui.checkbox(&mut self.draft.capture_diagnostics, tr("录制诊断"))
                    .on_hover_text(tr("在成片旁保存混音前音频和采集时间戳。"));
                if self.draft.camera.source != camera::Source::Camera {
                    ui.checkbox(&mut self.draft.mouse, tr("录制鼠标指针"));
                }
                ui.checkbox(&mut self.draft.minimize,tr("开始前最小化主窗口"));
                ui.horizontal(|ui| {
                    ui.label(tr("录屏历史记录上限"));
                    ui.add_sized([40.0, ui.spacing().interact_size.y],
                        egui::DragValue::new(&mut self.draft.history_limit)
                            .range(history::MIN_LIMIT..=history::MAX_LIMIT).speed(1.0));
                    ui.spacing_mut().slider_width = 100.0;
                    crate::theme::slider(ui, true,
                        egui::Slider::new(&mut self.draft.history_limit, history::MIN_LIMIT..=history::MAX_LIMIT)
                            .integer().show_value(false));
                });
                ui.separator();
                ui.label(tr("停止录制快捷键"));preferences::hotkey_editor(ui,"record_stop_key",&mut self.draft.stop);
                ui.small(tr("快捷键在程序运行期间全局生效。保存时检查系统占用，其他软件的局部快捷键仍可能相同。"));
                if self.draft.camera.source != camera::Source::Camera {
                    ui.label(tr("进入指定区域快捷键"));preferences::hotkey_editor(ui,"record_select_key",&mut self.draft.select);
                    ui.small(tr("单击吸附或拖拽创建选区，圆圈十字重新选择；框内可拖动移动，边中点单轴缩放，录制前 Esc 退出。"));
                }
            });
            if let Some(error)=&self.settings_error{ui.colored_label(ui.visuals().error_fg_color,error);}
            if self.applying{ui.horizontal(|ui|{ui.spinner();ui.label(tr("正在保存设置…"));});}
            ui.separator();ui.horizontal(|ui|{
                if ui.add_enabled(!self.applying && !self.busy(),egui::Button::new(tr("保存"))).clicked(){save=true;}
                if ui.add_enabled(!self.applying,egui::Button::new(tr("取消"))).clicked(){cancel=true;}
                if ui.button(tr("第三方支持")).clicked(){crate::third_party::request_open(ctx);}
            });
        });
        if !open || cancel {
            self.settings_open = false;
            self.draft = self.preferences.clone();
        }
        if save {
            match self.draft.validate() {
                Ok(()) => {
                    self.applying = true;
                    if let Some(shortcuts) = &self.shortcuts {
                        if let Err(error) = shortcuts.apply(self.draft.clone()) {
                            self.applying = false;
                            self.shortcuts_ready = false;
                            self.settings_error = Some(error);
                        }
                    }
                }
                Err(error) => self.settings_error = Some(error),
            }
        }
    }

    fn device_settings(&mut self, ui: &mut egui::Ui) {
        let select_device = |ui: &mut egui::Ui, id, label, devices: &[(String, String)], device: &mut String| {
            let selected = devices.iter().find(|(id, _)| id == device)
                .map_or_else(|| if device.is_empty() { tr(label) } else { device.as_str() }, |(_, name)| name);
            egui::ComboBox::from_id_salt(id).width(200.0).truncate().selected_text(selected).show_ui(ui, |ui| {
                for (id, name) in devices {
                    ui.selectable_value(device, id.clone(), name);
                }
            });
        };
        ui.horizontal_wrapped(|ui| {
            select_device(ui, "record_camera_device", "选择摄像头", &self.devices.video, &mut self.draft.camera.device);
            self.scan_devices_button(ui, "扫描摄像头");
        });
        ui.horizontal_wrapped(|ui| {
            select_device(ui, "record_microphone", "选择麦克风", &self.devices.audio, &mut self.draft.microphone_device);
            self.scan_devices_button(ui, "扫描麦克风");
        });
        ui.separator();
    }

    fn select_recording_source(&mut self, ctx: &egui::Context, source: camera::Source) {
        let mut candidate = self.preferences.clone();
        candidate.camera.source = source;
        if source != camera::Source::Screen && candidate.camera.device.is_empty() {
            self.draft = candidate;
            self.settings_open = true;
            crate::native_window::wake(ctx, egui::Id::new("screen_record_settings"));
        } else if self.preferences.camera.source != source {
            match candidate.save() {
                Ok(()) => { self.preferences = candidate; self.draft = self.preferences.clone(); }
                Err(error) => self.error = Some(error),
            }
        }
    }

    fn toggle_audio(&mut self, index: usize) {
        if self.applying || self.settings_open || self.job.as_ref().is_some_and(|job| job.stopping) {
            return;
        }
        if index == 0 { self.system_muted = !self.system_muted; }
        else { self.microphone_muted = !self.microphone_muted; }
        if let Some(job) = &self.job {
            *job.mute.lock().unwrap() = [self.system_muted, self.microphone_muted];
        }
    }

    fn switch_preview_source(&mut self, ctx: &egui::Context, source: camera::Source) {
        use camera::Source;
        if self.busy() { return; }
        if self.settings_open {
            self.draft.camera.source = source;
        } else {
            self.select_recording_source(ctx, source);
            if !self.settings_open && self.preferences.camera.source != source { return; }
        }
        self.camera_preview = None;
        self.pip_preview.open = false;
        self.pip_preview.release();
        self.pip_preview_settings = None;
        let settings = if self.settings_open { &self.draft.camera } else { &self.preferences.camera };
        match source {
            Source::Screen => {
                self.mode = 2;
                self.pending_select = true;
            }
            Source::Camera if !settings.device.is_empty() => self.open_camera_preview(ctx),
            Source::Combined if !settings.device.is_empty() => {
                self.pip_preview.open = true;
                self.pip_preview_settings = Some(settings.clone());
                crate::native_window::wake(ctx, pip::View::id());
            }
            _ => {}
        }
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn open_camera_preview(&mut self, ctx: &egui::Context) {
        let settings = if self.settings_open { &self.draft.camera } else { &self.preferences.camera };
        if settings.device.is_empty() {
            self.select_recording_source(ctx, camera::Source::Camera);
            return;
        }
        match camera::Preview::open(ctx, settings) {
            Ok(preview) => {
                self.camera_preview = Some(preview);
                crate::native_window::wake(ctx, egui::Id::new("record_camera_preview"));
            }
            Err(error) => {
                self.open = false;
                self.error = Some(error);
            }
        }
    }

    fn scan_devices_button(&mut self, ui: &mut egui::Ui, label: &str) {
        if ui.add_enabled(self.camera_scan.is_none(), egui::Button::new(tr(label))).clicked() {
            let (send, receive) = mpsc::channel();
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let _ = send.send(camera::devices());
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            });
            self.camera_scan = Some(receive);
        }
        if self.camera_scan.is_some() { ui.spinner(); }
    }
    fn show_result(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        if !self.result_open {
            return None;
        }
        let mut open = true;
        let mut done = false;
        crate::native_window::Window::new(tr("录屏"))
            .id(egui::Id::new("screen_record_result"))
            .open(&mut open)
            .default_width(500.0)
            .default_height(200.0)
            .always_on_top()
            .show(ctx, |ui| {
                if let Some(error) = &self.error {
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(error).color(ui.visuals().error_fg_color),
                                )
                                .wrap(),
                            );
                        });
                    if ui.button(tr("复制错误")).clicked() {
                        ui.ctx().copy_text(error.clone());
                    }
                }
                if let Some(path) = crate::error_log::path() {
                    ui.add(egui::Label::new(format!("error.log: {}", path.display())).wrap());
                    if ui
                        .add_enabled(path.is_file(), egui::Button::new(tr("打开日志")))
                        .clicked()
                    {
                        if let Err(error) = crate::media_tools::command("notepad.exe")
                            .arg(&path)
                            .spawn()
                        {
                            crate::error_log::error("screen_record/open_log", &error);
                            self.log_open_error = Some(error.to_string());
                        }
                    }
                }
                if let Some(error) = &self.log_open_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if ui.button(tr("关闭")).clicked() {
                    done = true;
                }
            });
        if !open || done {
            self.result_open = false;
        }
        None
    }
    fn selected_area(&self, monitors: &[displays::Display]) -> Area {
        match self.preferences.camera.source {
            camera::Source::Camera => return screen(false),
            camera::Source::Combined => return pip_area(&self.preferences.camera.display, monitors),
            camera::Source::Screen => {}
        }
        match self.mode {
            0 => monitors
                .iter()
                .find(|display| display.key == self.monitor)
                .map_or(screen(false), |display| display.area),
            1 => screen(true),
            _ => self.area,
        }
    }
    fn recording_status(&self) -> String {
        self.job.as_ref().map_or_else(
                || {
                    if self.preparing.is_some() {
                        tr("正在检查录屏组件…").to_owned()
                    } else if self.shortcuts_ready {
                        String::new()
                    } else {
                        tr("快捷键未就绪，请打开录屏设置。").to_owned()
                    }
                },
                |job| {
                    if job.stopping {
                        return tr("正在结束录屏并保存…").to_owned();
                    }
                    let Some(started) = job.started else {
                        return if job.countdown.is_some() {
                            String::new()
                        } else {
                            tr("正在启动录屏…").to_owned()
                        };
                    };
                    let seconds = started.elapsed().as_secs();
                    let mib = job
                        .path
                        .metadata()
                        .map_or(0.0, |metadata| metadata.len() as f64 / 1048576.0);
                    format!(
                        "{:02}:{:02}:{:02}  {:.1} MiB",
                        seconds / 3600,
                        seconds / 60 % 60,
                        seconds % 60,
                        mib
                    )
                },
            )
    }
    fn sync_outline(&mut self, ctx: &egui::Context, monitors: &[displays::Display]) {
        let busy = self.busy();
        let recording = self.job.as_ref().is_some_and(|job| job.started.is_some() && !job.stopping);
        let area = self
            .job
            .as_ref()
            .map_or_else(|| self.selected_area(monitors), |job| job.area);
        let mut items = vec![tr("指定区域").to_owned()];
        for display in monitors {
            items.push(if monitors.len() == 1 {
                tr("全屏").to_owned()
            } else {
                tr("屏幕{0}").replace("{0}", &display.number.to_string())
            });
        }
        if monitors.len() > 1 {
            items.push(tr("全部屏幕").to_owned());
        }
        let camera_index = items.len();
        items.push(tr("摄像头").to_owned());
        items.push(tr("摄像头画中画").to_owned());
        let selected = match self.preferences.camera.source {
            camera::Source::Camera => camera_index,
            camera::Source::Combined => camera_index + 1,
            camera::Source::Screen => match self.mode {
            0 => monitors
                .iter()
                .position(|display| display.key == self.monitor)
                .map_or(0, |index| index + 1),
            1 => monitors.len() + 1,
            _ => 0,
            },
        };
        let toolbar = outline::Toolbar {
            error: self
                .error
                .as_ref()
                .map(|error| error.lines().next().unwrap_or(error).to_owned()),
            system_audio: self.preferences.system_audio.then_some(!self.system_muted),
            microphone: self.preferences.microphone.then_some(!self.microphone_muted),
            show_screen_region: self.preferences.camera.source != camera::Source::Camera,
            items,
            selected,
            stop_label: self.preferences.stop.label(),
            can_start: self.shortcuts_ready
                && !self.applying
                && !self.settings_open
                && self.toolbar.ready,
            status: self.recording_status(),
            result: self.result.clone(),
        };
        let active = (self.open || busy) && area.valid(screen(true));
        let countdown = self
            .job
            .as_mut()
            .filter(|job| !job.stopping && self.preferences.camera.source == camera::Source::Screen)
            .and_then(|job| job.countdown.as_mut());
        if let Err(error) = self.countdown.show(ctx, area, countdown) {
            self.error = Some(format!("{}\n{error}", tr("无法显示录屏倒计时，请重试。")));
            self.result_open = true;
            self.stop();
        }
        let selecting = self
            .outline
            .as_ref()
            .is_some_and(|outline| outline.selecting);
        let editable = self.preferences.camera.source == camera::Source::Screen
            && self.mode == 2 && !busy && !self.settings_open && !self.history.open;
        if active && self.preferences.camera.source == camera::Source::Screen {
            self.toolbar_actions = self.toolbar.show(
                ctx,
                area,
                &toolbar,
                editable,
                selecting,
                busy,
                (!busy || self.mode == 2 || self.preferences.camera.source != camera::Source::Screen)
                    && !self.settings_open && !self.result_open,
            );
            if let Some(error) = &self.toolbar.error {
                if self.error.as_ref() != Some(error) {
                    self.error = Some(error.clone());
                    self.result_open = true;
                }
            }
        } else {
            self.toolbar.close(ctx);
            self.toolbar_actions.clear();
        }
        if let Some(outline) = &mut self.outline {
            outline.attach_toolbar(self.toolbar.handles);
            if active && self.preferences.camera.source == camera::Source::Screen {
                outline.show(
                    area,
                    editable && self.outline_editable,
                    !busy
                        && !self.settings_open
                        && !self.history.open
                        && self.missing_tools.is_none()
                        && !self.result_open,
                    recording,
                );
            } else {
                outline.hide();
            }
        }
    }
    fn start(&mut self, ctx: &egui::Context) {
        let pip_controls = self.pip_preview.open && self.preferences.camera.source == camera::Source::Combined;
        let camera_controls = self.preferences.camera.source == camera::Source::Camera;
        if self.busy() || self.settings_open || !self.shortcuts_ready || (camera_controls && self.camera_preview.is_none()) || (!pip_controls && !camera_controls && !self.toolbar.ready) {
            return;
        }
        if !pip_controls && self.preferences.camera.source != camera::Source::Camera && !self
            .outline
            .as_ref()
            .is_some_and(|outline| outline.ready && !outline.selecting)
        {
            return;
        }
        self.history.open = false;
        let area = self.selected_area(&displays::list());
        if self.preferences.camera.source != camera::Source::Camera && !area.valid(screen(true)) {
            self.error = Some(tr("录制区域超出桌面范围，请重新设置。").into());
            self.result_open = true;
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let ffmpeg = find_tool("ffmpeg");
            let ffprobe = find_tool("ffprobe");
            let mut missing = vec![];
            if ffmpeg.is_none() {
                missing.push("FFmpeg");
            }
            if ffprobe.is_none() {
                missing.push("FFprobe");
            }
            let _ = sender.send((ffmpeg, missing));
            ctx.request_repaint();
        });
        self.preparing = Some(Preparation { receiver, area });
        self.missing_tools = None;
    }
    fn begin_record(&mut self, ctx: &egui::Context, area: Area, ffmpeg: PathBuf) {
        if self.pip_preview.open && self.preferences.camera.source == camera::Source::Combined
            && let Err(error) = self.pip_preview.verify_exclusion() {
            self.error = Some(error);
            self.result_open = true;
            return;
        }
        let path = match recording_path(if self.preferences.mp4 { "mp4" } else { "mkv" }) {
            Ok(path) => path,
            Err(error) => {
                self.error = Some(error);
                self.result_open = true;
                return;
            }
        };
        let options = Options {
            camera: self.preferences.camera.clone(),
            area,
            fps: self.preferences.fps,
            crf: [18, 23, 28][self.preferences.quality],
            mp4: self.preferences.mp4,
            mouse: self.preferences.mouse,
            system_audio: self.preferences.system_audio,
            capture_diagnostics: self.preferences.capture_diagnostics,
            microphone: self.preferences.microphone.then(|| self.preferences.microphone_device.clone()),
            muted: [self.system_muted, self.microphone_muted],
            path: path.clone(),
        };
        let (output, pip, camera_frames) = if self.pip_preview.open && options.camera.source == camera::Source::Combined {
            match pip::Output::bind(ctx, area, &options.camera) {
                Ok((output, picture, position)) => (Some(output), Some(pip::LiveView { picture, position, settings: options.camera.clone() }), None),
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.result_open = true;
                    return;
                }
            }
        } else if options.camera.source == camera::Source::Camera {
            if self.camera_preview.is_none() { return; }
            match pip::Output::bind_camera(ctx) {
                Ok((output, frames)) => (Some(output), None, Some(frames)),
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.result_open = true;
                    return;
                }
            }
        } else { (None, None, None) };
        let (stop, receiver) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        let context = ctx.clone();
        let mute = std::sync::Arc::new(std::sync::Mutex::new(options.muted));
        let worker_mute = mute.clone();
        std::thread::spawn(move || {
            let result = record(options, ffmpeg, receiver, &sender, &context, output, worker_mute);
            let _ = sender.send(Event::Finished(result));
            context.request_repaint();
        });
        self.job = Some(Job {
            area,
            stop,
            events,
            path,
            started: None,
            countdown: None,
            stopping: false,
            pip,
            camera_frames,
            capture_started: false,
            mute,
        });
        self.result = None;
        self.result_open = false;
        self.error = None;
        self.restore = self.preferences.minimize;
        if self.restore {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }
}
