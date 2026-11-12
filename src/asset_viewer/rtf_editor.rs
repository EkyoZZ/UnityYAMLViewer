//! Standalone Windows RichEdit window. All HWNDs stay on their message-loop thread.
use super::native::{Handle, SendMessageW, read_rtf, stream_in, stream_out};
use crate::localization::tr;
use eframe::egui;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    Win32WindowHandle, WindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    io::Write,
    num::NonZeroIsize,
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};

const ACTIVATE: u32 = 0x8001;
const UPDATE_THEME: u32 = 0x8002;
const SAVE: usize = 1;
const SAVE_AS: usize = 2;
const UNDO: usize = 3;
const REDO: usize = 4;
const INSERT_IMAGE: usize = 5;
const BOLD: usize = 6;
const ITALIC: usize = 7;
const UNDERLINE: usize = 8;
const STRIKE: usize = 9;
const PAPER: usize = 10;
const FONT_SIZE: usize = 11;
const INSERT_TABLE: usize = 12;
const NEW_PAGE: usize = 13;
const DELETE_PAGE: usize = 14;
const PREV_PAGE: usize = 15;
const NEXT_PAGE: usize = 16;
const TEXT_COLOR: usize = 17;
const TEXT_BACKGROUND: usize = 18;
const CLEAR_TEXT_BACKGROUND: usize = 19;
const BULLETS: usize = 20;
const ALIGN_LEFT: usize = 21;
const ALIGN_CENTER: usize = 22;
const ALIGN_RIGHT: usize = 23;
const CUT: usize = 24;
const COPY: usize = 25;
const PASTE: usize = 26;
const SELECT_ALL: usize = 27;
const CFM_SIZE: u32 = 0x80000000;
const CFM_COLOR: u32 = 0x40000000;
const CFM_BACKCOLOR: u32 = 0x04000000;
const TOOLBAR_HEIGHT: i32 = 163;

#[path = "rtf_editor_pages.rs"]
mod pages;
#[path = "rtf_editor_table.rs"]
mod table;

#[path = "rtf_editor_style.rs"]
mod style;

pub(super) struct Session {
    hwnd: Arc<AtomicUsize>,
    thread: thread::JoinHandle<()>,
    saved: mpsc::Receiver<PathBuf>,
    dark: Cell<Option<bool>>,
}

impl Session {
    pub(super) fn open(path: PathBuf, ctx: egui::Context) -> Self {
        let hwnd = Arc::new(AtomicUsize::new(0));
        let window = hwnd.clone();
        let (saved_tx, saved) = mpsc::channel();
        let thread = thread::spawn(move || {
            if let Err(error) = run(path, &window, saved_tx, &ctx) {
                alert(null_mut(), tr("编辑 RTF"), &error, 0x10);
            }
            ctx.request_repaint();
        });
        Self {
            hwnd,
            thread,
            saved,
            dark: Cell::new(None),
        }
    }

    pub(super) fn is_open(&self) -> bool {
        !self.thread.is_finished()
    }

    pub(super) fn update_theme(&self, dark: bool) {
        let window = self.hwnd.load(Ordering::Acquire) as Handle;
        if !window.is_null()
            && self.dark.get() != Some(dark)
            && unsafe { PostMessageW(window, UPDATE_THEME, 0, 0) } != 0
        {
            self.dark.set(Some(dark));
        }
    }

    pub(super) fn activate(&self) {
        let window = self.hwnd.load(Ordering::Acquire) as Handle;
        if !window.is_null() {
            unsafe {
                PostMessageW(window, ACTIVATE, 0, 0);
            }
        }
    }

    pub(super) fn request_close(&self) {
        let window = self.hwnd.load(Ordering::Acquire) as Handle;
        if !window.is_null() {
            unsafe {
                PostMessageW(window, ACTIVATE, 0, 0);
                PostMessageW(window, 0x0010, 0, 0);
            }
        }
    }

    pub(super) fn take_saved(&self, path: &Path) -> bool {
        let mut saved = self.saved.try_iter();
        let Some(first) = saved.next() else {
            return false;
        };
        let path = path.canonicalize();
        std::iter::once(first)
            .chain(saved)
            .fold(false, |reload, target| {
                reload || path.as_ref().is_ok_and(|path| *path == target)
            })
    }
}

#[repr(C)]
struct WindowClass {
    style: u32,
    proc: unsafe extern "system" fn(Handle, u32, usize, isize) -> isize,
    class_extra: i32,
    window_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu: *const u16,
    name: *const u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track: Point,
    max_track: Point,
}

#[repr(C)]
struct Message {
    hwnd: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    point: Point,
    private: u32,
}

#[repr(C)]
struct Notify {
    hwnd: Handle,
    id: usize,
    code: i32,
}

#[repr(C)]
#[derive(Default)]
struct CharFormat {
    size: u32,
    mask: u32,
    effects: u32,
    height: i32,
    offset: i32,
    color: u32,
    charset: u8,
    pitch: u8,
    face: [u16; 32],
    weight: u16,
    spacing: i16,
    background: u32,
    locale: u32,
    reserved: u32,
    style: i16,
    kerning: u16,
    underline_type: u8,
    animation: u8,
    revision_author: u8,
    underline_color: u8,
}

#[repr(C)]
struct ParagraphFormat {
    size: u32,
    mask: u32,
    numbering: u16,
    effects: u16,
    start_indent: i32,
    right_indent: i32,
    offset: i32,
    alignment: u16,
    tab_count: i16,
    tabs: [i32; 32],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const std::ffi::c_void) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: Handle,
    ) -> Handle;
    fn DefWindowProcW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> isize;
    #[cfg(target_pointer_width = "32")]
    #[link_name = "GetWindowLongW"]
    fn GetWindowLongPtrW(hwnd: Handle, index: i32) -> isize;
    #[cfg(target_pointer_width = "64")]
    fn GetWindowLongPtrW(hwnd: Handle, index: i32) -> isize;
    #[cfg(target_pointer_width = "32")]
    #[link_name = "SetWindowLongW"]
    fn SetWindowLongPtrW(hwnd: Handle, index: i32, value: isize) -> isize;
    #[cfg(target_pointer_width = "64")]
    fn SetWindowLongPtrW(hwnd: Handle, index: i32, value: isize) -> isize;
    fn DestroyWindow(hwnd: Handle) -> i32;
    fn MoveWindow(hwnd: Handle, x: i32, y: i32, width: i32, height: i32, repaint: i32) -> i32;
    fn ShowWindow(hwnd: Handle, command: i32) -> i32;
    fn SetForegroundWindow(hwnd: Handle) -> i32;
    fn SetFocus(hwnd: Handle) -> Handle;
    fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    fn PostMessageW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> i32;
    fn PostQuitMessage(exit_code: i32);
    fn GetMessageW(msg: *mut Message, hwnd: Handle, first: u32, last: u32) -> i32;
    fn TranslateMessage(msg: *const Message) -> i32;
    fn DispatchMessageW(msg: *const Message) -> isize;
    fn GetKeyState(key: i32) -> i16;
    fn MessageBoxW(hwnd: Handle, body: *const u16, title: *const u16, flags: u32) -> i32;
    fn GetWindowTextW(window: Handle, text: *mut u16, count: i32) -> i32;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    fn GetFocus() -> Handle;
    fn SetTimer(window: Handle, id: usize, elapsed: u32, callback: Handle) -> usize;
    fn KillTimer(window: Handle, id: usize) -> i32;
    fn CreatePopupMenu() -> Handle;
    fn AppendMenuW(menu: Handle, flags: u32, id: usize, label: *const u16) -> i32;
    fn TrackPopupMenu(
        menu: Handle,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        owner: Handle,
        rect: Handle,
    ) -> u32;
    fn DestroyMenu(menu: Handle) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
}

type SubclassProc = unsafe extern "system" fn(Handle, u32, usize, isize, usize, usize) -> isize;
#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(window: Handle, callback: SubclassProc, id: usize, data: usize) -> i32;
    fn DefSubclassProc(window: Handle, msg: u32, wparam: usize, lparam: isize) -> isize;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetStockObject(index: i32) -> Handle;
}

struct Owner(Handle);

impl HasWindowHandle for Owner {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = NonZeroIsize::new(self.0 as isize).ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(hwnd));
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for Owner {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

struct Editor<'a> {
    hwnd: Handle,
    rich: Handle,
    format_buttons: [Handle; 4],
    bullet_button: Handle,
    bullets_active: Cell<bool>,
    alignment_buttons: [Handle; 3],
    alignment: Cell<u16>,
    font_size: Handle,
    table_inputs: [Handle; 3],
    page_status: Handle,
    page_starts: RefCell<Option<Vec<i32>>>,
    path: PathBuf,
    original: RefCell<Vec<u8>>,
    saved: mpsc::Sender<PathBuf>,
    ctx: &'a egui::Context,
    published_handle: &'a AtomicUsize,
    busy: Cell<bool>,
    active_formats: Cell<u32>,
    paper: Cell<u32>,
}

impl Editor<'_> {
    fn command(&self, command: usize) {
        if !(SAVE..=SELECT_ALL).contains(&command) || self.busy.replace(true) {
            return;
        }
        let result = match command {
            SAVE => self.save(&self.path),
            SAVE_AS => self.save_as(),
            INSERT_IMAGE => self.insert_image(),
            FONT_SIZE => self.apply_font_size(),
            INSERT_TABLE => table::insert(self.rich, self.table_inputs),
            NEW_PAGE => pages::insert_break(self),
            DELETE_PAGE => pages::delete(self),
            PREV_PAGE | NEXT_PAGE => pages::navigate(self, command == NEXT_PAGE),
            TEXT_COLOR | TEXT_BACKGROUND | CLEAR_TEXT_BACKGROUND => self.change_color(command),
            BULLETS => {
                let mut format = self.paragraph_format();
                format.numbering = u16::from(format.mask & 0x20 == 0 || format.numbering != 1);
                format.mask = 0x20;
                unsafe {
                    SendMessageW(self.rich, 0x0447, 0, &format as *const _ as isize);
                }
                Ok(())
            }
            ALIGN_LEFT..=ALIGN_RIGHT => {
                let mut format = self.paragraph_format();
                format.mask = 8;
                format.alignment = [1, 3, 2][command - ALIGN_LEFT];
                unsafe {
                    SendMessageW(self.rich, 0x0447, 0, &format as *const _ as isize);
                }
                Ok(())
            }
            CUT..=SELECT_ALL => {
                let (message, end) = if command == SELECT_ALL {
                    (0x00b1, -1)
                } else {
                    (0x0300 + (command - CUT) as u32, 0)
                };
                unsafe {
                    SendMessageW(self.rich, message, 0, end);
                }
                Ok(())
            }
            PAPER => {
                if let Some(color) = style::choose_paper(self.hwnd, self.paper.get()) {
                    self.paper.set(color);
                    style::refresh(self);
                }
                Ok(())
            }
            UNDO | REDO => {
                const EM_UNDO: u32 = 0x00c7;
                const EM_REDO: u32 = 0x0454;
                let message = if command == UNDO { EM_UNDO } else { EM_REDO };
                unsafe {
                    SendMessageW(self.rich, message, 0, 0);
                }
                Ok(())
            }
            BOLD..=STRIKE => {
                let effect = 1 << (command - BOLD);
                let current = self.selection_format();
                let format = CharFormat {
                    size: std::mem::size_of::<CharFormat>() as u32,
                    mask: effect,
                    effects: if current.mask & current.effects & effect != 0 {
                        0
                    } else {
                        effect
                    },
                    ..Default::default()
                };
                unsafe {
                    SendMessageW(self.rich, 0x444, 1, &format as *const _ as isize);
                }
                Ok(())
            }
            _ => unreachable!(),
        };
        if let Err(error) = result {
            let title = tr(match command {
                INSERT_IMAGE => "插入图片",
                FONT_SIZE => "字号",
                INSERT_TABLE => "插入表格",
                NEW_PAGE => "插入分页符",
                DELETE_PAGE | PREV_PAGE | NEXT_PAGE => "删除当前页",
                _ => "RTF 保存失败",
            });
            alert(self.hwnd, title, &error, 0x10);
        }
        self.update_format_buttons();
        pages::update_status(self);
        pages::schedule(self);
        unsafe {
            SetFocus(self.rich);
        }
        self.busy.set(false);
    }

    fn selection_format(&self) -> CharFormat {
        let mut format = CharFormat {
            size: std::mem::size_of::<CharFormat>() as u32,
            ..Default::default()
        };
        unsafe {
            SendMessageW(self.rich, 0x43a, 1, &mut format as *mut _ as isize);
        }
        format
    }

    fn update_format_buttons(&self) {
        let paragraph = self.paragraph_format();
        let alignment = if paragraph.mask & 8 != 0 {
            paragraph.alignment
        } else {
            0
        };
        if self.alignment.replace(alignment) != alignment {
            for button in self.alignment_buttons {
                style::invalidate_button(button);
            }
        }
        let bullets = paragraph.mask & 0x20 != 0 && paragraph.numbering == 1;
        if self.bullets_active.replace(bullets) != bullets {
            style::invalidate_button(self.bullet_button);
        }
        let format = self.selection_format();
        let current = format.mask & format.effects & 15;
        let changed = self.active_formats.replace(current) ^ current;
        for (index, button) in self.format_buttons.iter().enumerate() {
            if changed & (1 << index) != 0 {
                style::invalidate_button(*button);
            }
        }
        if unsafe { GetFocus() } != self.font_size {
            let size = if format.mask & CFM_SIZE != 0 {
                (format.height as f64 / 20.0).to_string()
            } else {
                String::new()
            };
            if control_text(self.font_size) != size {
                unsafe {
                    SetWindowTextW(self.font_size, wide(&size).as_ptr());
                }
            }
        }
    }

    fn paragraph_format(&self) -> ParagraphFormat {
        let mut format: ParagraphFormat = unsafe { std::mem::zeroed() };
        format.size = std::mem::size_of::<ParagraphFormat>() as u32;
        unsafe {
            SendMessageW(self.rich, 0x043d, 0, &mut format as *mut _ as isize);
        }
        format
    }

    fn apply_font_size(&self) -> Result<(), String> {
        let points = control_text(self.font_size)
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|value| (1.0..=200.0).contains(value))
            .ok_or_else(|| tr("字号须在 1–200 pt 之间。").to_owned())?;
        let format = CharFormat {
            size: std::mem::size_of::<CharFormat>() as u32,
            mask: CFM_SIZE,
            height: (points * 20.0).round() as i32,
            ..Default::default()
        };
        unsafe {
            SendMessageW(self.rich, 0x0444, 1, &format as *const _ as isize);
        }
        Ok(())
    }

    fn change_color(&self, command: usize) -> Result<(), String> {
        let current = self.selection_format();
        let mut format = CharFormat {
            size: std::mem::size_of::<CharFormat>() as u32,
            ..Default::default()
        };
        match command {
            CLEAR_TEXT_BACKGROUND => {
                format.mask = CFM_BACKCOLOR;
                format.effects = CFM_BACKCOLOR;
            }
            TEXT_BACKGROUND => {
                let initial =
                    if current.mask & CFM_BACKCOLOR != 0 && current.effects & CFM_BACKCOLOR == 0 {
                        current.background
                    } else {
                        0x00ffff
                    };
                let Some(color) = style::choose_paper(self.hwnd, initial) else {
                    return Ok(());
                };
                format.mask = CFM_BACKCOLOR;
                format.background = color;
            }
            _ => {
                let Some(color) = style::choose_paper(self.hwnd, current.color) else {
                    return Ok(());
                };
                format.mask = CFM_COLOR;
                format.color = color;
            }
        }
        unsafe {
            SendMessageW(self.rich, 0x0444, 1, &format as *const _ as isize);
        }
        Ok(())
    }

    fn context_menu(&self, position: isize) {
        if self.busy.replace(true) {
            return;
        }
        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            self.busy.set(false);
            return;
        }
        let mut point = Point {
            x: position as i16 as i32,
            y: (position >> 16) as i16 as i32,
        };
        if position == -1 {
            unsafe {
                GetCursorPos(&mut point);
            }
        }
        let format = self.selection_format();
        for (command, label) in [
            (CUT, "剪切"),
            (COPY, "复制"),
            (PASTE, "粘贴"),
            (SELECT_ALL, "全选"),
            (0, ""),
            (BOLD, "粗体"),
            (ITALIC, "斜体"),
            (UNDERLINE, "下划线"),
            (STRIKE, "删除线"),
            (TEXT_COLOR, "文字颜色"),
            (TEXT_BACKGROUND, "文字底色"),
            (CLEAR_TEXT_BACKGROUND, "清除文字底色"),
            (0, ""),
            (BULLETS, "项目符号"),
            (ALIGN_LEFT, "左对齐"),
            (ALIGN_CENTER, "居中对齐"),
            (ALIGN_RIGHT, "右对齐"),
        ] {
            if command == 0 {
                unsafe {
                    AppendMenuW(menu, 0x0800, 0, std::ptr::null());
                }
                continue;
            }
            let checked = (BOLD..=STRIKE).contains(&command)
                && format.mask & format.effects & (1 << (command - BOLD)) != 0;
            unsafe {
                AppendMenuW(
                    menu,
                    if checked { 8 } else { 0 },
                    command,
                    wide(tr(label)).as_ptr(),
                );
            }
        }
        let command =
            unsafe { TrackPopupMenu(menu, 0x0102, point.x, point.y, 0, self.hwnd, null_mut()) };
        unsafe {
            DestroyMenu(menu);
        }
        self.busy.set(false);
        if command != 0 {
            self.command(command as usize);
        }
    }

    fn save_as(&self) -> Result<(), String> {
        let dialog = rfd::FileDialog::new()
            .set_parent(&Owner(self.hwnd))
            .set_directory(self.path.parent().unwrap_or(Path::new(".")))
            .set_file_name(self.path.file_name().unwrap_or_default().to_string_lossy())
            .add_filter("RTF", &["rtf"]);
        if let Some(path) = dialog.save_file() {
            self.save(&path)?;
        }
        Ok(())
    }

    fn save(&self, destination: &Path) -> Result<(), String> {
        let canonical = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.to_owned());
        let original = canonical == self.path;
        if original && read_rtf(&canonical)? != *self.original.borrow() {
            return Err(tr("原 RTF 文件已被其他程序修改，请另存到其他路径。").into());
        }
        let bytes = stream_out(self.rich)?;
        save_rtf(&canonical, &bytes)?;
        if original {
            self.original.replace(bytes);
            unsafe {
                SendMessageW(self.rich, 0x00b9, 0, 0);
            }
        }
        let _ = self
            .saved
            .send(canonical.canonicalize().unwrap_or(canonical));
        self.ctx.request_repaint();
        Ok(())
    }

    fn insert_image(&self) -> Result<(), String> {
        let Some(path) = rfd::FileDialog::new()
            .set_parent(&Owner(self.hwnd))
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif"])
            .pick_file()
        else {
            return Ok(());
        };
        let mut reader = image::ImageReader::open(path)
            .map_err(|error| error.to_string())?
            .with_guessed_format()
            .map_err(|error| error.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().map_err(|error| error.to_string())?;
        let mut png = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let (width, height) = (image.width(), image.height());
        // At the editor's 96-DPI reference size, one source pixel is 15 twips.
        let (goal_width, goal_height) = (width * 15, height * 15);
        use std::fmt::Write as _;
        let mut fragment = format!(
            "{{\\rtf1{{\\pict\\pngblip\\picw{width}\\pich{height}\\picwgoal{goal_width}\\pichgoal{goal_height}\n"
        );
        if fragment.len() + png.get_ref().len() * 2 + 2 > super::native::MAX_RTF_BYTES {
            return Err(tr("RTF 内容超过预览限制。").into());
        }
        for byte in png.into_inner() {
            write!(fragment, "{byte:02x}").expect("writing to a String cannot fail");
        }
        fragment.push_str("}}");
        stream_in(self.rich, fragment.as_bytes(), true)
    }
}

fn save_rtf(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_file_name(format!(".rtf-{}-{nonce}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let result = file
        .write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string());
    drop(file);
    let result = result.and_then(|_| crate::playlist::replace(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn control_text(window: Handle) -> String {
    let mut text = [0u16; 64];
    let count = unsafe { GetWindowTextW(window, text.as_mut_ptr(), text.len() as i32) };
    String::from_utf16_lossy(&text[..count.max(0) as usize])
}

unsafe extern "system" fn rich_proc(
    hwnd: Handle,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _id: usize,
    parent: usize,
) -> isize {
    if msg == 0x007b {
        // WM_CONTEXTMENU
        return unsafe { SendMessageW(parent as Handle, msg, wparam, lparam) };
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn alert(window: Handle, title: &str, message: &str, flags: u32) -> i32 {
    unsafe { MessageBoxW(window, wide(message).as_ptr(), wide(title).as_ptr(), flags) }
}

unsafe extern "system" fn window_proc(
    hwnd: Handle,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if msg == 0x0024 {
        unsafe {
            (*(lparam as *mut MinMaxInfo)).min_track = Point { x: 690, y: 320 };
        }
        return 0;
    }
    let editor = unsafe { (GetWindowLongPtrW(hwnd, -21) as *const Editor<'_>).as_ref() };
    if let Some(editor) = editor {
        match msg {
            0x007b if !editor.busy.get() => {
                editor.context_menu(lparam);
                return 0;
            }
            0x0113 if wparam == 1 && !editor.busy.get() => {
                pages::refresh(editor);
                return 0;
            }
            0x0133 | 0x0138 => {
                return style::color_control(editor, wparam as Handle, msg == 0x0133) as isize;
            }
            0x0014 => {
                style::erase(editor, wparam as Handle);
                return 1;
            }
            0x002b if lparam != 0 => {
                if style::draw_button(editor, lparam) {
                    return 1;
                }
            }
            UPDATE_THEME => {
                style::refresh(editor);
                return 0;
            }
            0x0005 => {
                let width = (lparam & 0xffff) as i32;
                let height = ((lparam >> 16) & 0xffff) as i32;
                unsafe {
                    MoveWindow(
                        editor.rich,
                        8,
                        TOOLBAR_HEIGHT,
                        (width - 16).max(0),
                        (height - TOOLBAR_HEIGHT - 8).max(0),
                        1,
                    );
                }
                return 0;
            }
            0x0111 if lparam != 0 && wparam >> 16 == 0 => {
                editor.command(wparam & 0xffff);
                return 0;
            }
            0x0111 if lparam as Handle == editor.rich && wparam >> 16 == 0x0300 => {
                pages::schedule(editor);
                return 0;
            }
            0x004e if lparam != 0 => {
                let notice = unsafe { &*(lparam as *const Notify) };
                if notice.hwnd == editor.rich && notice.code == 0x0702 {
                    editor.update_format_buttons();
                    pages::update_status(editor);
                }
            }
            0x0010 => {
                if editor.busy.replace(true) {
                    return 0;
                }
                let modified = unsafe { SendMessageW(editor.rich, 0x00b8, 0, 0) } != 0;
                let discard = !modified
                    || style::confirm(
                        editor,
                        tr("未保存的修改"),
                        tr("放弃未保存的修改？"),
                        tr("放弃修改"),
                    );
                editor.busy.set(false);
                if discard {
                    unsafe {
                        DestroyWindow(hwnd);
                    }
                }
                return 0;
            }
            0x0002 => {
                editor.published_handle.store(0, Ordering::Release);
                unsafe {
                    PostQuitMessage(0);
                }
                return 0;
            }
            ACTIVATE => {
                unsafe {
                    ShowWindow(hwnd, 9);
                    SetForegroundWindow(hwnd);
                    SetFocus(editor.rich);
                }
                return 0;
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn run(
    path: PathBuf,
    handle: &AtomicUsize,
    saved: mpsc::Sender<PathBuf>,
    ctx: &egui::Context,
) -> Result<(), String> {
    struct Window(Handle);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }

    let original = read_rtf(&path)?;
    let _library =
        unsafe { libloading::Library::new("Msftedit.dll") }.map_err(|error| error.to_string())?;
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class_name = wide("UnityYamlViewer.RtfEditor");
    static CLASS: OnceLock<Result<(), String>> = OnceLock::new();
    CLASS
        .get_or_init(|| {
            let class = WindowClass {
                style: 0,
                proc: window_proc,
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: null_mut(),
                cursor: unsafe { LoadCursorW(null_mut(), 32512 as *const u16) },
                background: 16 as Handle,
                menu: std::ptr::null(),
                name: class_name.as_ptr(),
            };
            if unsafe { RegisterClassW((&class as *const WindowClass).cast()) } == 0 {
                Err(std::io::Error::last_os_error().to_string())
            } else {
                Ok(())
            }
        })
        .as_ref()
        .map_err(Clone::clone)?;

    let title = format!(
        "{} - {}",
        tr("编辑 RTF"),
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide(&title).as_ptr(),
            0x02cf0000,
            i32::MIN,
            i32::MIN,
            820,
            680,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut window = Window(hwnd);
    let rich = unsafe {
        CreateWindowExW(
            0,
            wide("RICHEDIT50W").as_ptr(),
            std::ptr::null(),
            0x50201144,
            8,
            TOOLBAR_HEIGHT,
            788,
            560,
            hwnd,
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if rich.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    unsafe {
        SendMessageW(rich, 0x435, 0, 8 * 1024 * 1024);
    }
    stream_in(rich, &original, false)?;
    unsafe {
        SendMessageW(rich, 0x00b9, 0, 0);
        SendMessageW(rich, 0x00cd, 0, 0);
        SendMessageW(rich, 0x445, 0, 0x80001);
    }

    let mut format_buttons = [null_mut(); 4];
    let mut bullet_button = null_mut();
    let mut alignment_buttons = [null_mut(); 3];
    let buttons = [
        (SAVE, tr("保存"), 8, 8, 105),
        (SAVE_AS, tr("另存为"), 119, 8, 150),
        (UNDO, tr("撤销"), 275, 8, 100),
        (REDO, tr("重做"), 381, 8, 100),
        (PREV_PAGE, "‹", 487, 8, 41),
        (NEXT_PAGE, "›", 624, 8, 41),
        (BOLD, tr("粗体"), 8, 39, 36),
        (ITALIC, tr("斜体"), 50, 39, 36),
        (UNDERLINE, tr("下划线"), 92, 39, 36),
        (STRIKE, tr("删除线"), 134, 39, 36),
        (BULLETS, tr("项目符号"), 176, 39, 36),
        (INSERT_IMAGE, tr("插入图片"), 218, 39, 36),
        (PAPER, tr("编辑区底色"), 260, 39, 172),
        (ALIGN_LEFT, tr("左对齐"), 442, 39, 36),
        (ALIGN_CENTER, tr("居中对齐"), 484, 39, 36),
        (ALIGN_RIGHT, tr("右对齐"), 526, 39, 36),
        (FONT_SIZE, tr("应用"), 225, 70, 105),
        (NEW_PAGE, tr("插入分页符"), 336, 70, 170),
        (DELETE_PAGE, tr("删除当前页"), 512, 70, 153),
        (INSERT_TABLE, tr("插入表格"), 515, 101, 150),
        (TEXT_COLOR, tr("文字颜色"), 8, 132, 170),
        (TEXT_BACKGROUND, tr("文字底色"), 184, 132, 170),
        (CLEAR_TEXT_BACKGROUND, tr("清除文字底色"), 360, 132, 195),
    ];
    let font = unsafe { GetStockObject(17) };
    for (id, label, x, y, width) in buttons {
        let button = unsafe {
            CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide(label).as_ptr(),
                0x5001000b,
                x,
                y,
                width,
                25,
                hwnd,
                id as Handle,
                instance,
                null_mut(),
            )
        };
        if button.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            SendMessageW(button, 0x0030, font as usize, 1);
        }
        if (BOLD..=STRIKE).contains(&id) {
            format_buttons[id - BOLD] = button;
        }
        if id == BULLETS {
            bullet_button = button;
        }
        if (ALIGN_LEFT..=ALIGN_RIGHT).contains(&id) {
            alignment_buttons[id - ALIGN_LEFT] = button;
        }
    }

    let make_control = |class: &str, title: &str, style, x, y, width| {
        let control = unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(title).as_ptr(),
                style,
                x,
                y,
                width,
                25,
                hwnd,
                null_mut(),
                instance,
                null_mut(),
            )
        };
        if control.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            SendMessageW(control, 0x0030, font as usize, 1);
        }
        Ok(control)
    };
    let label = |title: &str, x, y, width| make_control("STATIC", title, 0x50000200, x, y, width);
    let page_status = make_control("STATIC", "– / –", 0x50000201, 530, 8, 92)?;
    let field = |title: &str, x, y, width| -> Result<Handle, String> {
        let control = make_control("EDIT", title, 0x50810081, x, y, width)?;
        unsafe {
            SendMessageW(control, 0x00c5, 8, 0);
        }
        Ok(control)
    };
    label(&format!("{} (pt)", tr("字号")), 8, 70, 140)?;
    let font_size = field("", 151, 70, 68)?;
    label(tr("行数"), 8, 101, 89)?;
    label(tr("列数"), 150, 101, 89)?;
    label(&format!("{} (pt)", tr("列宽")), 292, 101, 147)?;
    let table_inputs = [
        field("2", 100, 101, 44)?,
        field("2", 242, 101, 44)?,
        field("100", 442, 101, 67)?,
    ];

    let editor = Editor {
        hwnd,
        rich,
        format_buttons,
        bullet_button,
        bullets_active: Cell::new(false),
        alignment_buttons,
        alignment: Cell::new(0),
        font_size,
        table_inputs,
        page_status,
        page_starts: RefCell::new(None),
        path,
        original: RefCell::new(original),
        saved,
        ctx,
        published_handle: handle,
        busy: Cell::new(false),
        active_formats: Cell::new(0),
        paper: Cell::new(0xffffff),
    };
    unsafe {
        SetWindowLongPtrW(hwnd, -21, &editor as *const _ as isize);
        SetWindowSubclass(rich, rich_proc, 1, hwnd as usize);
        handle.store(hwnd as usize, Ordering::Release);
        style::refresh(&editor);
        ShowWindow(hwnd, 5);
        SetForegroundWindow(hwnd);
        SetFocus(rich);
        editor.update_format_buttons();
        pages::schedule(&editor);
        let mut message: Message = std::mem::zeroed();
        loop {
            let status = GetMessageW(&mut message, null_mut(), 0, 0);
            if status <= 0 {
                break;
            }
            if message.message == 0x0100 && message.wparam == 0x0d && message.hwnd == font_size {
                editor.command(FONT_SIZE);
                continue;
            }
            if message.message == 0x0100 && message.wparam == b'S' as usize && GetKeyState(0x11) < 0
            {
                editor.command(if GetKeyState(0x10) < 0 { SAVE_AS } else { SAVE });
                continue;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if handle.swap(0, Ordering::AcqRel) == 0 {
            window.0 = null_mut();
        }
        drop(window);
    }
    Ok(())
}
