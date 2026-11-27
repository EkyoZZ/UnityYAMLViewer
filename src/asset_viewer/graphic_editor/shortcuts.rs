use std::{
    collections::BTreeMap,
    fs,
    sync::{OnceLock, RwLock},
};

use crate::localization::tr;
use eframe::egui::{self, Event, Key, Modifiers};
use serde::{Deserialize, Serialize};

macro_rules! commands {
    ($($name:ident: $label:literal, $key:ident, $ctrl:literal, $alt:literal, $shift:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum Command { $($name),* }
        impl Command {
            const ALL: &'static [Self] = &[$(Self::$name),*];
            fn id(self) -> &'static str {
                match self { $(Self::$name => stringify!($name)),* }
            }
            fn label(self) -> &'static str {
                match self { $(Self::$name => tr($label)),* }
            }
            fn default_binding(self) -> Binding {
                match self { $(Self::$name => Binding {
                    key: Key::$key.name().into(), ctrl: $ctrl, alt: $alt, shift: $shift
                }),* }
            }
        }
    };
}

commands! {
    Save: "保存", S, true, false, false;
    SaveAs: "另存为", S, true, false, true;
    Undo: "撤销", Z, true, false, false;
    Redo: "重做", Y, true, false, false;
    ZoomIn: "放大画布", Plus, true, false, false;
    ZoomOut: "缩小画布", Minus, true, false, false;
    Fit: "适应画布", Num0, true, false, false;
    Bounded: "有边界", Q, false, false, false;
    Unbounded: "无边界", W, false, false, false;
    Center: "回到中心循环单元", H, false, false, false;
    Rulers: "标尺", R, true, false, false;
    Guides: "辅助线", Semicolon, true, false, false;
    Pan: "平移画布", Space, false, false, false;
    TransformControls: "显示变换控件", H, true, false, false;
    Transform: "自由变换", T, true, false, false;
    Confirm: "确定", Enter, false, false, false;
    Cancel: "取消", Escape, false, false, false;
    CommitText: "确认文字编辑", Enter, true, false, false;
    Merge: "合并图层", E, true, false, false;
    MergeCopy: "合并到新图层", E, true, true, false;
    MergeAll: "合并底图以上图层", E, true, false, true;
    Duplicate: "复制图层", J, true, false, false;
    Cut: "通过剪切的图层", J, true, false, true;
    Delete: "删除图层", Delete, false, false, false;
    FillForeground: "填充前景色", Delete, false, true, false;
    FillBackground: "填充背景色", Delete, true, false, false;
    SelectAll: "全选", A, true, false, false;
    Deselect: "取消选区", D, true, false, false;
    BrushSmaller: "减小笔尖尺寸", OpenBracket, false, false, false;
    BrushLarger: "增大笔尖尺寸", CloseBracket, false, false, false;
    Move: "移动", V, false, false, false;
    Selection: "选区", M, false, false, false;
    Brush: "画笔", B, false, false, false;
    Eyedropper: "吸管", I, false, false, false;
    Eraser: "橡皮", E, false, false, false;
    Rectangle: "矩形", K, false, false, false;
    Line: "直线", L, false, false, false;
    Fill: "填充", G, false, false, false;
    Zoom: "缩放", Z, false, false, false;
    Text: "文字", T, false, false, false;
    Shape: "形状", U, false, false, false;
    Path: "路径", A, false, false, false;
    Pen: "钢笔", P, false, false, false;
    Left: "图层左移", ArrowLeft, false, false, false;
    Right: "图层右移", ArrowRight, false, false, false;
    Up: "图层上移", ArrowUp, false, false, false;
    Down: "图层下移", ArrowDown, false, false, false;
    LeftFast: "图层快速左移", ArrowLeft, false, false, true;
    RightFast: "图层快速右移", ArrowRight, false, false, true;
    UpFast: "图层快速上移", ArrowUp, false, false, true;
    DownFast: "图层快速下移", ArrowDown, false, false, true;
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Binding {
    key: String,
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl Binding {
    fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key: key.name().into(),
            ctrl: modifiers.ctrl || modifiers.command,
            alt: modifiers.alt,
            shift: modifiers.shift,
        }
    }

    fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        Key::from_name(&self.key) == Some(key)
            && self.ctrl == (modifiers.ctrl || modifiers.command)
            && self.alt == modifiers.alt
            && self.shift == modifiers.shift
    }

    fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(
            Key::from_name(&self.key)
                .map(Key::symbol_or_name)
                .unwrap_or(self.key.as_str()),
        );
        parts.join("+")
    }
}

type Overrides = BTreeMap<String, Binding>;

fn path() -> Option<std::path::PathBuf> {
    crate::last_open_dir_config_path().map(|path| path.with_file_name("drawing_shortcuts.json"))
}

fn current() -> &'static RwLock<Overrides> {
    static OVERRIDES: OnceLock<RwLock<Overrides>> = OnceLock::new();
    OVERRIDES.get_or_init(|| {
        let mut overrides: Overrides = path()
            .and_then(|p| fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        overrides.retain(|id, binding| {
            Command::ALL.iter().any(|cmd| cmd.id() == id) && Key::from_name(&binding.key).is_some()
        });
        RwLock::new(overrides)
    })
}

fn save(draft: &Overrides) -> Result<(), String> {
    let path = path().ok_or_else(|| tr("无法保存快捷键设置。").to_owned())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let data = serde_json::to_vec_pretty(draft).map_err(|error| error.to_string())?;
    fs::write(&temp, data).map_err(|error| error.to_string())?;
    fs::rename(&temp, path).map_err(|error| error.to_string())?;
    *current().write().unwrap() = draft.clone();
    Ok(())
}

impl Command {
    fn binding(self, overrides: &Overrides) -> Binding {
        overrides
            .get(self.id())
            .cloned()
            .unwrap_or_else(|| self.default_binding())
    }

    fn matches(self, overrides: &Overrides, key: Key, modifiers: Modifiers) -> bool {
        if self.binding(overrides).matches(key, modifiers) {
            return true;
        }
        self == Self::ZoomIn
            && !overrides.contains_key(self.id())
            && modifiers.ctrl
            && !modifiers.alt
            && matches!(key, Key::Equals | Key::Plus)
    }

    pub(super) fn label_key(self) -> String {
        self.binding(&current().read().unwrap()).label()
    }

    pub(super) fn matches_event(self, key: Key, modifiers: Modifiers) -> bool {
        self.matches(&current().read().unwrap(), key, modifiers)
    }

    pub(super) fn take(self, ctx: &egui::Context, allow_repeat: bool) -> bool {
        ctx.input_mut(|input| {
            let Some(index) = input
                .events
                .iter()
                .position(|event| self.is_pressed(event, allow_repeat))
            else {
                return false;
            };
            input.events.remove(index);
            true
        })
    }

    fn is_pressed(self, event: &Event, allow_repeat: bool) -> bool {
        matches!(event, Event::Key { key, pressed: true, repeat, modifiers, .. }
            if (allow_repeat || !repeat) && self.matches_event(*key, *modifiers))
    }

    pub(super) fn pressed(self, ctx: &egui::Context) -> bool {
        ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| self.is_pressed(event, true))
        })
    }

    pub(super) fn down(self, input: &egui::InputState) -> bool {
        let binding = self.binding(&current().read().unwrap());
        Key::from_name(&binding.key)
            .is_some_and(|key| binding.matches(key, input.modifiers) && input.key_down(key))
    }
}

#[derive(Clone)]
struct Dialog {
    draft: Overrides,
    capturing: Option<Command>,
    last_pass: Option<u64>,
    error: Option<String>,
}

fn id() -> egui::Id {
    egui::Id::new("drawing_shortcut_settings")
}

pub(super) fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<Dialog>(id()).is_some())
}

pub(super) fn open(ctx: &egui::Context) {
    if !is_open(ctx) {
        let draft = current().read().unwrap().clone();
        ctx.data_mut(|data| {
            data.insert_temp(
                id(),
                Dialog {
                    draft,
                    capturing: None,
                    last_pass: None,
                    error: None,
                },
            )
        });
    }
    crate::native_window::wake(ctx, id());
}

pub(super) fn show(ctx: &egui::Context) {
    let Some(mut dialog) = ctx.data(|data| data.get_temp::<Dialog>(id())) else {
        return;
    };
    let pass = ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT);
    if dialog.last_pass == Some(pass) {
        return;
    }
    dialog.last_pass = Some(pass);
    let mut open = true;
    let mut close = false;
    crate::native_window::Window::new(tr("快捷键设置"))
        .id(id())
        .open(&mut open)
        .always_on_top()
        .scrollable(false)
        .default_width(480.0)
        .min_width(340.0)
        .min_height(280.0)
        .default_height(580.0)
        .show(ctx, |ui| {
            if let Some(command) = dialog.capturing {
                let pressed = ui.input_mut(|input| {
                    let index = input.events.iter().position(|event| {
                        matches!(
                            event,
                            Event::Key {
                                pressed: true,
                                repeat: false,
                                ..
                            }
                        )
                    })?;
                    match input.events.remove(index) {
                        Event::Key { key, modifiers, .. } => Some((key, modifiers)),
                        _ => None,
                    }
                });
                if let Some((key, modifiers)) = pressed {
                    let binding = Binding::new(key, modifiers);
                    let conflict = Command::ALL.iter().copied().find(|other| {
                        *other != command && other.matches(&dialog.draft, key, modifiers)
                    });
                    if let Some(other) = conflict {
                        dialog.error = Some(
                            tr("{0} 已被「{1}」占用，再次点击按钮撤销。")
                                .replace("{0}", &binding.label())
                                .replace("{1}", other.label()),
                        );
                    } else {
                        dialog.draft.insert(command.id().into(), binding);
                        dialog.error = None;
                        dialog.capturing = None;
                    }
                }
            }
            egui::TopBottomPanel::bottom("shortcut_settings_actions")
                .exact_height(42.0)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let gap = 8.0;
                        ui.spacing_mut().item_spacing.x = gap;
                        let available = ui.available_width();
                        let width = ((available - 2.0 * gap) / 3.0).min(120.0);
                        ui.add_space(((available - (3.0 * width + 2.0 * gap)) * 0.5).max(0.0));
                        let mut button = |label| {
                            ui.add_sized([width, 34.0], egui::Button::new(tr(label)))
                                .clicked()
                        };
                        if button("恢复默认设置") {
                            dialog.draft.clear();
                            dialog.error = None;
                            dialog.capturing = None;
                        }
                        if button("取消") {
                            close = true;
                        }
                        if button("确定") {
                            match save(&dialog.draft) {
                                Ok(()) => close = true,
                                Err(error) => {
                                    dialog.error =
                                        Some(format!("{} {error}", tr("无法保存快捷键设置。")));
                                }
                            }
                        }
                    });
                });
            if let Some(error) = &dialog.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("drawing_shortcut_rows")
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for &command in Command::ALL {
                                ui.label(command.label());
                                let capturing = dialog.capturing == Some(command);
                                let label = if capturing {
                                    tr("按下新快捷键").into()
                                } else {
                                    command.binding(&dialog.draft).label()
                                };
                                if ui
                                    .add_sized(
                                        [150.0, 22.0],
                                        egui::Button::new(label).selected(capturing),
                                    )
                                    .clicked()
                                {
                                    dialog.capturing = if capturing { None } else { Some(command) };
                                    dialog.error = None;
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    ctx.data_mut(|data| {
        if close || !open {
            data.remove::<Dialog>(id());
        } else {
            data.insert_temp(id(), dialog);
        }
    });
    if close {
        ctx.send_viewport_cmd_to(egui::ViewportId(id()), egui::ViewportCommand::Close);
    }
}
