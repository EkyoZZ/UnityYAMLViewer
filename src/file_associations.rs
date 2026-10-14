//! Read-only Windows association status and a guided handoff to system settings.
use std::{collections::{BTreeMap, BTreeSet}, path::PathBuf, sync::mpsc};
use eframe::egui;
use crate::{localization::tr, native_window};

// Shared with the supported-formats window, so the two entry points cannot drift.
pub const FORMAT_GROUPS: &[(&str, &str)] = &[
    ("Unity YAML", ".prefab .unity .asset .mat .controller .overridecontroller .anim .spriteatlas .guiskin .yaml .yml .physicsmaterial2d .physicmaterial"),
    ("图片", ".png .jpg .jpeg .tga .bmp .webp"),
    ("模型", ".fbx .obj"),
    ("Spine 3.8", ".skel .skel.bytes .json .json.txt"),
    ("音频", ".mp3 .wav .ogg .oga .flac .aac .m4a .wma .opus .aiff .aif"),
    ("视频", ".mp4 .m4v .mkv .webm .mov .avi .wmv .flv .mpeg .mpg .ts .mts .m2ts .3gp"),
    ("文本 / 代码", ".cs .txt .md .json .xml .shader .hlsl .cginc .compute .glsl .uxml .uss .asmdef .asmref .meta .csv .tsv .log .ini .toml .js .ts .lua .py .pyw .c .cpp .h .hpp .rs .java .html .css .sh .ps1 .bat .cmd"),
    ("播放列表", ".m3u8 .m3u"),
];

fn extensions(group: usize) -> Vec<String> {
    FORMAT_GROUPS[group].1.split_whitespace()
        // Windows associates the final extension, not .skel.bytes or .json.txt.
        .filter_map(|suffix| suffix.rsplit_once('.').map(|(_, ext)| format!(".{ext}")))
        .collect::<BTreeSet<_>>().into_iter().collect()
}

#[derive(Clone)]
enum Association {
    App { name: String, executable: Option<PathBuf>, is_current: bool },
    Unset,
    Error(String),
    #[cfg(not(windows))]
    Unsupported,
}
type Snapshot = BTreeMap<String, Association>;

#[derive(Default)]
pub struct Dialog {
    open: bool,
    group: usize,
    associations: Snapshot,
    pending: Option<mpsc::Receiver<Snapshot>>,
    focused: Option<bool>,
    selected_extension: Option<String>,
    settings_error: Option<String>,
    path_copied: bool,
}

impl Dialog {
    pub fn open(&mut self, ctx: &egui::Context) {
        if !self.open {
            self.open = true;
            self.focused = None;
            self.selected_extension = None;
            self.settings_error = None;
            self.path_copied = false;
        }
        self.refresh(ctx);
    }

    fn refresh(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() { return; }
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let all: BTreeSet<_> = (0..FORMAT_GROUPS.len()).flat_map(extensions).collect();
            let current_exe = std::env::current_exe().ok();
            let snapshot = all.into_iter().map(|ext| {
                let association = query(&ext, current_exe.as_deref());
                (ext, association)
            }).collect();
            let _ = send.send(snapshot);
            ctx.request_repaint();
        });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open { return; }
        if let Some(pending) = &self.pending {
            match pending.try_recv() {
                Ok(snapshot) => { self.associations = snapshot; self.pending = None; }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.settings_error = Some("默认程序查询中断，请重试刷新。".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let mut open = self.open;
        let mut close = false;
        native_window::Window::new(tr("默认打开程序"))
            .id(egui::Id::new("file_associations")).open(&mut open)
            .default_width(640.0).min_width(380.0).default_height(560.0)
            .show(ctx, |ui| {
                // Use this native dialog's focus, not the parent window's focus.
                let focused = ui.ctx().input(|input| input.focused);
                if focused && self.focused == Some(false) { self.refresh(ui.ctx()); }
                self.focused = Some(focused);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("格式分组"));
                    egui::ComboBox::from_id_salt("association_group")
                        .selected_text(tr(FORMAT_GROUPS[self.group].0)).show_ui(ui, |ui| {
                            for (index, (label, _)) in FORMAT_GROUPS.iter().enumerate() {
                                ui.selectable_value(&mut self.group, index, tr(label));
                            }
                        });
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new(tr("刷新状态"))).clicked() {
                        self.refresh(ui.ctx());
                    }
                    if self.pending.is_some() { ui.spinner(); ui.label(tr("正在查询默认程序…")); }
                });
                ui.add_space(8.0);
                ui.label(tr("点击去设置会复制后缀并打开 Windows 默认应用页面，请粘贴后缀搜索并选择本程序。此处不会直接修改文件关联。"));
                if !cfg!(windows) { ui.label(tr("文件关联设置仅支持 Windows。")); }
                if matches!(self.group, 3 | 5 | 6) {
                    ui.small(tr("关联按最后一个后缀生效：.skel.bytes 使用 .bytes，.json.txt 使用 .txt；.json 和 .ts 等后缀可能包含不同类型的内容。"));
                }
                ui.separator();
                let available = ui.available_width();
                let action_width = (ui.painter().layout_no_wrap(tr("去设置…").into(),
                    egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color()).size().x + 20.0).min(available * 0.45);
                let ext_width = 146.0_f32.min((available * 0.32).max(70.0));
                let app_width = (available - ext_width - action_width - 20.0).max(40.0);
                egui::Grid::new("association_rows").num_columns(3).spacing([8.0, 8.0]).show(ui, |ui| {
                    ui.add_sized([ext_width, 0.0], egui::Label::new(egui::RichText::new(tr("后缀")).strong()).wrap());
                    ui.add_sized([app_width, 0.0], egui::Label::new(egui::RichText::new(tr("当前默认程序")).strong()).wrap());
                    ui.label(""); ui.end_row();
                    for ext in extensions(self.group) {
                        ui.add_sized([ext_width, 0.0], egui::Label::new(&ext).wrap());
                        let association = self.associations.get(&ext);
                        let (name, tooltip) = match association {
                            Some(Association::App { name, executable, is_current }) => (
                                if *is_current { tr("已是本程序").to_owned() } else { name.clone() },
                                executable.as_ref().map_or_else(|| name.clone(), |path| path.display().to_string())),
                            Some(Association::Unset) => (tr("未设置").into(), String::new()),
                            Some(Association::Error(error)) => (tr("查询失败").into(), error.clone()),
                            #[cfg(not(windows))]
                            Some(Association::Unsupported) => (tr("不支持").into(), String::new()),
                            None => (tr("正在查询默认程序…").into(), String::new()),
                        };
                        ui.add_sized([app_width, 0.0], egui::Label::new(name).wrap()).on_hover_text(tooltip);
                        // Keep this available even for our own association, so users can change it away.
                        let button = ui.add_enabled_ui(cfg!(windows), |ui|
                            ui.add_sized([action_width, 0.0], egui::Button::new(tr("去设置…")).wrap())).inner;
                        if button.clicked() {
                            ui.ctx().copy_text(ext.clone());
                            self.selected_extension = Some(ext);
                            self.path_copied = false;
                            self.settings_error = open_settings().err();
                        }
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.label(tr("如果系统列表中没有本程序，请浏览选择下方 EXE；未列出的后缀可通过实际文件的打开方式选择本程序。"));
                if let Ok(path) = std::env::current_exe() {
                    ui.add(egui::Label::new(path.display().to_string()).wrap());
                    if ui.button(tr("复制程序路径")).clicked() {
                        ui.ctx().copy_text(path.display().to_string());
                        self.path_copied = true;
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(cfg!(windows), egui::Button::new(tr("打开 Windows 默认应用设置"))).clicked() {
                        self.settings_error = open_settings().err();
                    }
                    if ui.button(tr("关闭")).clicked() { close = true; }
                });
                if let Some(ext) = &self.selected_extension {
                    ui.label(crate::lf!("已复制后缀 {0}，请在系统默认应用页面搜索；返回此窗口会刷新状态。", ext));
                }
                if self.path_copied { ui.label(tr("已复制程序路径；使用此按钮会替换剪贴板中的后缀。")); }
                if let Some(error) = &self.settings_error {
                    ui.colored_label(ui.visuals().error_fg_color, crate::lf!("无法打开系统设置或读取关联：{0}", tr(error)));
                }
            });
        self.open = open && !close;
    }
}

#[cfg(windows)]
fn query(extension: &str, current_exe: Option<&std::path::Path>) -> Association {
    use std::os::windows::ffi::OsStringExt;
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut std::ffi::c_void, model: u32) -> i32;
        fn CoUninitialize();
    }
    struct ComGuard;
    impl Drop for ComGuard { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
    // Shell association queries use COM on the background worker thread.
    let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), 2) };
    if hr < 0 { return Association::Error(format!("CoInitializeEx: 0x{:08X}", hr as u32)); }
    let _com = ComGuard;
    let extension: Vec<u16> = extension.encode_utf16().chain(Some(0)).collect();
    fn value(extension: &[u16], kind: u32) -> Result<String, u32> {
        #[link(name = "shlwapi")]
        unsafe extern "system" {
            fn AssocQueryStringW(flags: u32, kind: u32, assoc: *const u16, extra: *const u16,
                out: *mut u16, length: *mut u32) -> i32;
        }
        // NOFIXUPS makes this a read-only query; IGNOREUNKNOWN avoids OpenWith.exe fallback.
        const FLAGS: u32 = 0x100 | 0x400;
        let mut length = 0;
        let hr = unsafe { AssocQueryStringW(FLAGS, kind, extension.as_ptr(), std::ptr::null(), std::ptr::null_mut(), &mut length) };
        if hr < 0 { return Err(hr as u32); }
        if length == 0 || length > 32768 { return Err(0x80004005); }
        // Association may change between sizing and reading; retry one larger buffer.
        for _ in 0..2 {
            let mut buffer = vec![0u16; length as usize];
            let hr = unsafe { AssocQueryStringW(FLAGS, kind, extension.as_ptr(), std::ptr::null(), buffer.as_mut_ptr(), &mut length) };
            if hr == 0 {
                let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
                let text = std::ffi::OsString::from_wide(&buffer[..end]).to_string_lossy().into_owned();
                return if text.is_empty() { Err(0x80070483) } else { Ok(text) };
            }
            if hr as u32 != 0x80004003 || length as usize <= buffer.len() || length > 32768 { return Err(hr as u32); }
        }
        Err(0x80004005)
    }
    // ProgID exists for packaged apps too; lack of an executable does not mean unset.
    let prog_id = match value(&extension, 20) {
        Ok(id) => id,
        Err(0x80070483 | 0x80070002 | 0x80070003) => return Association::Unset,
        Err(code) => return Association::Error(format!("HRESULT 0x{code:08X}")),
    };
    let executable = value(&extension, 2).ok().map(PathBuf::from);
    let name = value(&extension, 4).unwrap_or_else(|_| executable.as_ref()
        .and_then(|path| path.file_name()).map_or(prog_id, |name| name.to_string_lossy().into_owned()));
    let is_current = executable.as_deref().zip(current_exe).is_some_and(|(associated, current)| {
        // Canonical paths compare different slash styles and extended-length path prefixes.
        std::fs::canonicalize(associated).ok().zip(std::fs::canonicalize(current).ok())
            .is_some_and(|(a, b)| a.as_os_str().to_string_lossy().eq_ignore_ascii_case(&b.as_os_str().to_string_lossy()))
    });
    Association::App { name, executable, is_current }
}

#[cfg(not(windows))]
fn query(_: &str, _: Option<&std::path::Path>) -> Association { Association::Unsupported }

#[cfg(windows)]
fn open_settings() -> Result<(), String> {
    use std::ffi::c_void;
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(parent: *mut c_void, verb: *const u16, file: *const u16,
            parameters: *const u16, directory: *const u16, show: i32) -> *mut c_void;
    }
    let uri: Vec<u16> = "ms-settings:defaultapps".encode_utf16().chain(Some(0)).collect();
    let result = unsafe { ShellExecuteW(std::ptr::null_mut(), std::ptr::null(), uri.as_ptr(),
        std::ptr::null(), std::ptr::null(), 1) } as isize;
    if result > 32 { Ok(()) } else { Err(format!("ShellExecuteW: {result}")) }
}

#[cfg(not(windows))]
fn open_settings() -> Result<(), String> { Err(tr("文件关联设置仅支持 Windows。").into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compound_suffixes_use_windows_final_extension() {
        let spine = extensions(3);
        assert!(spine.contains(&".bytes".into()));
        assert!(spine.contains(&".txt".into()));
        assert!(!spine.contains(&".skel.bytes".into()));
        assert!(!spine.contains(&".json.txt".into()));
    }
    #[test]
    fn shared_extensions_are_unique_and_in_both_groups() {
        for group in 0..FORMAT_GROUPS.len() {
            let values = extensions(group);
            assert_eq!(values.len(), values.iter().collect::<BTreeSet<_>>().len());
            assert!(values.iter().all(|ext| ext.starts_with('.') && !ext[1..].contains('.')));
        }
        assert!(extensions(5).contains(&".ts".into()));
        assert!(extensions(6).contains(&".ts".into()));
        assert!(extensions(3).contains(&".json".into()));
        assert!(extensions(6).contains(&".json".into()));
    }
}
