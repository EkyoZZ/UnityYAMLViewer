use crate::localization::tr;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf, sync::mpsc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Hotkey {
    pub modifiers: u32,
    pub key: u32,
}
impl Hotkey {
    pub fn valid(self) -> bool {
        self.modifiers & !7 == 0
            && self.key != 0x7b
            && (matches!(self.key, 0x70..=0x87)
                || (self.modifiers & 3 != 0 && matches!(self.key,0x30..=0x39|0x41..=0x5a)))
    }
    pub fn label(self) -> String {
        let mut parts = vec![];
        if self.modifiers & 2 != 0 {
            parts.push("Ctrl".to_owned());
        }
        if self.modifiers & 1 != 0 {
            parts.push("Alt".to_owned());
        }
        if self.modifiers & 4 != 0 {
            parts.push("Shift".to_owned());
        }
        parts.push(key_name(self.key));
        parts.join("+")
    }
}
fn key_name(key: u32) -> String {
    if (0x70..=0x87).contains(&key) {
        format!("F{}", key - 0x70 + 1)
    } else {
        char::from_u32(key).unwrap_or('?').to_string()
    }
}
pub(super) fn hotkey_editor(ui: &mut egui::Ui, id: &str, binding: &mut Hotkey) {
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            for (label, mask) in [("Ctrl", 2), ("Alt", 1), ("Shift", 4)] {
                let mut enabled = binding.modifiers & mask != 0;
                if ui.checkbox(&mut enabled, label).changed() {
                    if enabled {
                        binding.modifiers |= mask;
                    } else {
                        binding.modifiers &= !mask;
                    }
                }
            }
            egui::ComboBox::from_id_salt("key")
                .width(64.0)
                .selected_text(key_name(binding.key))
                .show_ui(ui, |ui| {
                    for key in (0x70..=0x87)
                        .filter(|key| *key != 0x7b)
                        .chain(0x41..=0x5a)
                        .chain(0x30..=0x39)
                    {
                        ui.selectable_value(&mut binding.key, key, key_name(key));
                    }
                });
        })
    });
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub fps: u32,
    pub quality: usize,
    pub mp4: bool,
    pub mouse: bool,
    pub system_audio: bool,
    pub minimize: bool,
    pub history_limit: usize,
    pub stop: Hotkey,
    pub select: Hotkey,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            fps: 30,
            quality: 1,
            mp4: false,
            mouse: true,
            system_audio: true,
            minimize: true,
            history_limit: super::history::DEFAULT_LIMIT,
            stop: Hotkey {
                modifiers: 6,
                key: 0x78,
            },
            select: Hotkey {
                modifiers: 6,
                key: 0x77,
            },
        }
    }
}
impl Preferences {
    fn path() -> Result<PathBuf, String> {
        crate::last_open_dir_config_path()
            .map(|p| p.with_file_name("screen_record.json"))
            .ok_or_else(|| tr("无法确定录屏设置目录。").to_owned())
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=60).contains(&self.fps)
            || self.quality > 2
            || !(super::history::MIN_LIMIT..=super::history::MAX_LIMIT)
                .contains(&self.history_limit)
        {
            return Err(tr("录屏参数无效。").into());
        }
        if !self.stop.valid() || !self.select.valid() {
            return Err(tr("请选择有效组合键；字母和数字需要 Ctrl 或 Alt，F12 不可使用。").into());
        }
        if self.stop == self.select {
            return Err(tr("停止录制与选择区域不能使用相同快捷键。").into());
        }
        Ok(())
    }
    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        let preferences = match std::fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        preferences.validate()?;
        Ok(preferences)
    }
    pub(super) fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary = path.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            file.write_all(&serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(|e| format!("{}: {e}", path.display()))
    }
}
enum Command {
    Apply(Preferences),
    Exit,
}
pub(super) enum Event {
    Stop,
    Select,
    Applied(Result<Preferences, String>, bool),
}
pub(super) struct Shortcuts {
    commands: mpsc::Sender<Command>,
    pub events: mpsc::Receiver<Event>,
}
impl Shortcuts {
    pub fn new(ctx: &egui::Context, initial: Preferences) -> Self {
        let (commands, receiver) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            #[cfg(windows)]
            native::run(receiver, sender, &ctx, initial);
            #[cfg(not(windows))]
            {
                drop(receiver);
                let _ = sender.send(Event::Applied(
                    Err(tr("录屏当前仅支持 Windows。").into()),
                    false,
                ));
                ctx.request_repaint();
            }
        });
        Self { commands, events }
    }
    pub fn apply(&self, preferences: Preferences) -> Result<(), String> {
        self.commands
            .send(Command::Apply(preferences))
            .map_err(|_| tr("录屏快捷键服务已退出，请重新启动程序。").to_owned())
    }
}
impl Drop for Shortcuts {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Exit);
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct Message {
        hwnd: *mut c_void,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        point: Point,
        private: u32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterHotKey(hwnd: *mut c_void, id: i32, modifiers: u32, key: u32) -> i32;
        fn UnregisterHotKey(hwnd: *mut c_void, id: i32) -> i32;
        fn PeekMessageW(
            message: *mut Message,
            hwnd: *mut c_void,
            first: u32,
            last: u32,
            remove: u32,
        ) -> i32;
    }
    fn unregister() {
        unsafe {
            for id in [1, 2] {
                UnregisterHotKey(std::ptr::null_mut(), id);
            }
        }
    }
    fn register(preferences: &Preferences) -> Result<(), String> {
        preferences.validate()?;
        for (id, key) in [(1, preferences.stop), (2, preferences.select)] {
            if unsafe { RegisterHotKey(std::ptr::null_mut(), id, key.modifiers | 0x4000, key.key) }
                == 0
            {
                let error = std::io::Error::last_os_error();
                unregister();
                return Err(format!(
                    "{}: {} ({error})",
                    tr("快捷键注册失败，可能已被系统或其他程序占用。"),
                    key.label()
                ));
            }
        }
        Ok(())
    }
    struct Registration;
    impl Drop for Registration {
        fn drop(&mut self) {
            unregister();
        }
    }
    pub(super) fn run(
        commands: mpsc::Receiver<Command>,
        events: mpsc::Sender<Event>,
        ctx: &egui::Context,
        initial: Preferences,
    ) {
        let _registration = Registration;
        let mut current = initial.clone();
        let result = register(&initial);
        let mut ready = result.is_ok();
        let _ = events.send(Event::Applied(result.map(|_| initial), ready));
        ctx.request_repaint();
        loop {
            match commands.recv_timeout(Duration::from_millis(16)) {
                Ok(Command::Exit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Command::Apply(candidate)) => {
                    let was_ready = ready;
                    unregister();
                    let result = register(&candidate).and_then(|_| candidate.save());
                    if result.is_ok() {
                        current = candidate.clone();
                        ready = true;
                    } else {
                        unregister();
                        ready = was_ready && register(&current).is_ok();
                    }
                    let _ = events.send(Event::Applied(result.map(|_| candidate), ready));
                    ctx.request_repaint();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            unsafe {
                let mut message: Message = std::mem::zeroed();
                while PeekMessageW(&mut message, std::ptr::null_mut(), 0x312, 0x312, 1) != 0 {
                    if ready {
                        let _ = events.send(if message.wparam == 1 {
                            Event::Stop
                        } else {
                            Event::Select
                        });
                        ctx.request_repaint();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_preferences_default_history_to_twenty_and_validate_new_limits() {
        let mut preferences: Preferences =
            serde_json::from_str(r#"{"fps":24,"mp4":true}"#).unwrap();
        assert_eq!(preferences.history_limit, 20);
        assert!(preferences.system_audio);
        assert!(preferences.validate().is_ok());
        preferences.history_limit = 50;
        preferences.system_audio = false;
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
        assert_eq!(restored.history_limit, 50);
        assert!(!restored.system_audio);
        preferences.history_limit = 19;
        assert!(preferences.validate().is_err());
        preferences.history_limit = 51;
        assert!(preferences.validate().is_err());
    }
    #[test]
    fn distinct_valid_hotkeys_roundtrip_with_recording_settings() {
        let preferences = Preferences::default();
        assert!(preferences.validate().is_ok());
        assert_eq!(preferences.stop.label(), "Ctrl+Shift+F9");
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
        assert_eq!(restored, preferences);
    }
    #[test]
    fn invalid_conflicting_and_reserved_keys_are_rejected() {
        let mut preferences = Preferences::default();
        preferences.select = preferences.stop;
        assert!(preferences.validate().is_err());
        preferences.select = Hotkey {
            modifiers: 6,
            key: 0x7b,
        };
        assert!(preferences.validate().is_err());
        preferences.select = Hotkey {
            modifiers: 0,
            key: 0x41,
        };
        assert!(preferences.validate().is_err());
    }
}
