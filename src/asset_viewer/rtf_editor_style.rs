//! Paint the native toolbar with the application's egui palette.
use super::{
    ALIGN_CENTER, ALIGN_LEFT, ALIGN_RIGHT, BOLD, BULLETS, Editor, Handle, INSERT_IMAGE, ITALIC,
    PAPER, SAVE, STRIKE, SendMessageW, UNDERLINE, wide,
};
use super::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetFocus, GetMessageW,
    GetModuleHandleW, GetWindowLongPtrW, LoadCursorW, Message, PostQuitMessage, RegisterClassW,
    SetFocus, SetWindowLongPtrW, ShowWindow, TranslateMessage, WindowClass, alert, tr,
};
use eframe::egui::Color32;
use std::ptr::{null, null_mut};
use std::{cell::Cell, sync::OnceLock};

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct DrawItem {
    kind: u32,
    id: u32,
    item: u32,
    action: u32,
    state: u32,
    window: Handle,
    dc: Handle,
    rect: Rect,
    data: usize,
}

#[repr(C)]
struct ChooseColor {
    size: u32,
    owner: Handle,
    instance: Handle,
    color: u32,
    custom_colors: *mut u32,
    flags: u32,
    data: isize,
    hook: usize,
    template: *const u16,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
    fn DrawTextW(dc: Handle, text: *mut u16, count: i32, rect: *mut Rect, format: u32) -> i32;
    fn GetWindowTextW(window: Handle, text: *mut u16, count: i32) -> i32;
    fn InvalidateRect(window: Handle, rect: *const Rect, erase: i32) -> i32;
    fn RedrawWindow(window: Handle, rect: *const Rect, region: Handle, flags: u32) -> i32;
    fn GetWindowRect(window: Handle, rect: *mut Rect) -> i32;
    fn EnableWindow(window: Handle, enabled: i32) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetStockObject(index: i32) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn SetDCBrushColor(dc: Handle, color: u32) -> u32;
    fn SetDCPenColor(dc: Handle, color: u32) -> u32;
    fn RoundRect(
        dc: Handle,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> i32;
    fn SetBkMode(dc: Handle, mode: i32) -> i32;
    fn SetBkColor(dc: Handle, color: u32) -> u32;
    fn SetTextColor(dc: Handle, color: u32) -> u32;
    fn SaveDC(dc: Handle) -> i32;
    fn RestoreDC(dc: Handle, saved: i32) -> i32;
    fn MoveToEx(dc: Handle, x: i32, y: i32, old: Handle) -> i32;
    fn LineTo(dc: Handle, x: i32, y: i32) -> i32;
    fn Rectangle(dc: Handle, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        output: u32,
        clip: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(window: Handle, attribute: u32, value: *const i32, size: u32) -> i32;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn ChooseColorW(settings: *mut ChooseColor) -> i32;
}

pub(super) fn choose_paper(window: Handle, initial: u32) -> Option<u32> {
    let mut custom_colors = [0xffffff; 16];
    let mut settings = ChooseColor {
        size: std::mem::size_of::<ChooseColor>() as u32,
        owner: window,
        instance: null_mut(),
        color: initial,
        custom_colors: custom_colors.as_mut_ptr(),
        flags: 0x0003, // Initialize the picker and show the custom color controls.
        data: 0,
        hook: 0,
        template: null(),
    };
    (unsafe { ChooseColorW(&mut settings) } != 0).then_some(settings.color)
}

pub(super) fn refresh(editor: &Editor<'_>) {
    let dark = i32::from(editor.ctx.style().visuals.dark_mode);
    unsafe {
        const EM_SETBKGNDCOLOR: u32 = 0x0443;
        const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
        DwmSetWindowAttribute(editor.hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark, 4);
        SendMessageW(
            editor.rich,
            EM_SETBKGNDCOLOR,
            0,
            editor.paper.get() as isize,
        );
        RedrawWindow(editor.hwnd, null(), null_mut(), 0x0085); // Invalidate and erase all children.
    }
}

fn colorref(color: Color32) -> u32 {
    u32::from_le_bytes([color.r(), color.g(), color.b(), 0])
}

fn fill(dc: Handle, rect: &Rect, color: Color32) {
    unsafe {
        let previous = SetDCBrushColor(dc, colorref(color));
        FillRect(dc, rect, GetStockObject(18)); // DC_BRUSH
        SetDCBrushColor(dc, previous);
    }
}

pub(super) fn erase(editor: &Editor<'_>, dc: Handle) {
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetClientRect(editor.hwnd, &mut rect) } != 0 {
        fill(dc, &rect, editor.ctx.style().visuals.panel_fill);
    }
}

pub(super) fn invalidate_button(button: Handle) {
    unsafe {
        InvalidateRect(button, null(), 0);
    }
}

pub(super) fn color_control(editor: &Editor<'_>, dc: Handle, edit: bool) -> Handle {
    let style = editor.ctx.style();
    let visuals = &style.visuals;
    unsafe {
        SetTextColor(dc, colorref(visuals.text_color()));
        SetBkMode(dc, if edit { 2 } else { 1 });
        let background = if edit {
            visuals.extreme_bg_color
        } else {
            visuals.panel_fill
        };
        SetDCBrushColor(dc, colorref(background));
        SetBkColor(dc, colorref(background));
        GetStockObject(18)
    }
}

pub(super) fn draw_button(editor: &Editor<'_>, data: isize) -> bool {
    let item = unsafe { &*(data as *const DrawItem) };
    if item.kind != 4 {
        return false;
    } // ODT_BUTTON

    let style = editor.ctx.style();
    let visuals = &style.visuals;
    let id = item.id as usize;
    let selected = match id {
        BOLD..=STRIKE => editor.active_formats.get() & (1 << (id - BOLD)) != 0,
        BULLETS => editor.bullets_active.get(),
        ALIGN_LEFT..=ALIGN_RIGHT => editor.alignment.get() == [1, 3, 2][id - ALIGN_LEFT],
        _ => false,
    };
    paint_button(
        item,
        visuals,
        selected,
        id == SAVE,
        Some((id, editor.paper.get())),
    );
    true
}

fn paint_button(
    item: &DrawItem,
    visuals: &eframe::egui::Visuals,
    selected: bool,
    primary: bool,
    icon: Option<(usize, u32)>,
) {
    let pressed = item.state & 0x0001 != 0;
    let focused = item.state & 0x0010 != 0 && item.state & 0x0200 == 0;
    let hovered = item.state & 0x0040 != 0;
    let widget = if selected || pressed {
        &visuals.widgets.active
    } else if focused || hovered {
        &visuals.widgets.hovered
    } else {
        &visuals.widgets.inactive
    };
    let (background, border, text) = if primary {
        let blue = if pressed {
            crate::theme::BLUE.gamma_multiply(0.8)
        } else {
            crate::theme::BLUE
        };
        (blue, blue, Color32::WHITE)
    } else {
        (
            widget.weak_bg_fill,
            widget.bg_stroke.color,
            visuals.text_color(),
        )
    };

    let mut label = [0u16; 128];
    let mut rect = Rect { ..item.rect };
    unsafe {
        let saved = SaveDC(item.dc);
        if saved == 0 {
            return;
        }
        fill(item.dc, &rect, visuals.panel_fill);
        SelectObject(item.dc, GetStockObject(18)); // DC_BRUSH
        SelectObject(item.dc, GetStockObject(19)); // DC_PEN
        SetDCBrushColor(item.dc, colorref(background));
        SetDCPenColor(item.dc, colorref(border));
        RoundRect(item.dc, rect.left, rect.top, rect.right, rect.bottom, 8, 8);
        if let Some((PAPER, paper)) = icon {
            let [r, g, b, _] = paper.to_le_bytes();
            let top = rect.top + (rect.bottom - rect.top - 12) / 2;
            let swatch = Rect {
                left: rect.left + 8,
                top,
                right: rect.left + 20,
                bottom: top + 12,
            };
            fill(item.dc, &swatch, Color32::from_rgb(r, g, b));
            rect.left += 16;
        }
        SelectObject(item.dc, GetStockObject(17)); // DEFAULT_GUI_FONT
        SetBkMode(item.dc, 1); // TRANSPARENT
        SetTextColor(item.dc, colorref(text));
        if !icon.is_some_and(|(id, _)| draw_icon(item.dc, &mut rect, id, text)) {
            let count = GetWindowTextW(item.window, label.as_mut_ptr(), label.len() as i32);
            DrawTextW(item.dc, label.as_mut_ptr(), count, &mut rect, 0x0825);
        }
        RestoreDC(item.dc, saved);
    }
}

fn draw_icon(dc: Handle, rect: &mut Rect, id: usize, color: Color32) -> bool {
    if (BOLD..=STRIKE).contains(&id) {
        let mut glyph = wide(["B", "I", "U", "ab"][id - BOLD]);
        unsafe {
            let font = CreateFontW(
                -15,
                0,
                0,
                0,
                if id == BOLD { 700 } else { 400 },
                u32::from(id == ITALIC),
                u32::from(id == UNDERLINE),
                u32::from(id == STRIKE),
                1,
                0,
                0,
                5,
                0,
                wide("Segoe UI").as_ptr(),
            );
            let previous = if font.is_null() {
                null_mut()
            } else {
                SelectObject(dc, font)
            };
            DrawTextW(
                dc,
                glyph.as_mut_ptr(),
                (glyph.len() - 1) as i32,
                rect,
                0x0825,
            );
            if !previous.is_null() {
                SelectObject(dc, previous);
            }
            if !font.is_null() {
                DeleteObject(font);
            }
        }
        return true;
    }
    let x = (rect.left + rect.right) / 2 - 8;
    let y = (rect.top + rect.bottom) / 2 - 6;
    let line = |x1, y1, x2, y2| unsafe {
        MoveToEx(dc, x + x1, y + y1, null_mut());
        LineTo(dc, x + x2, y + y2);
    };
    unsafe {
        SetDCPenColor(dc, colorref(color));
    }
    match id {
        INSERT_IMAGE => {
            unsafe {
                SelectObject(dc, GetStockObject(5)); // NULL_BRUSH
                Rectangle(dc, x, y, x + 17, y + 13);
            }
            line(1, 10, 6, 5);
            line(6, 5, 12, 11);
            line(8, 7, 11, 4);
            line(11, 4, 16, 9);
        }
        BULLETS => {
            for row in [1, 5, 9] {
                line(0, row, 2, row);
                line(5, row, 16, row);
            }
        }
        ALIGN_LEFT..=ALIGN_RIGHT => {
            for (row, length) in [(0, 16), (4, 10), (8, 16), (12, 10)] {
                let left = match id {
                    ALIGN_CENTER => (16 - length) / 2,
                    ALIGN_RIGHT => 16 - length,
                    _ => 0,
                };
                line(left, row, left + length, row);
            }
        }
        _ => return false,
    }
    true
}

struct Confirmation<'a> {
    visuals: &'a eframe::egui::Visuals,
    result: Cell<Option<bool>>,
}

unsafe extern "system" fn confirm_proc(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let dialog = unsafe { (GetWindowLongPtrW(window, -21) as *const Confirmation<'_>).as_ref() };
    if let Some(dialog) = dialog {
        match message {
            0x0010 => {
                dialog.result.set(Some(false));
                return 0;
            }
            0x0111 if wparam == 1 || wparam == 2 => {
                dialog.result.set(Some(wparam == 1));
                return 0;
            }
            0x0014 => {
                let mut rect = Rect {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                if unsafe { GetClientRect(window, &mut rect) } != 0 {
                    fill(wparam as Handle, &rect, dialog.visuals.window_fill);
                }
                return 1;
            }
            0x0138 => unsafe {
                let dc = wparam as Handle;
                SetTextColor(dc, colorref(dialog.visuals.text_color()));
                SetBkMode(dc, 1);
                SetDCBrushColor(dc, colorref(dialog.visuals.window_fill));
                return GetStockObject(18) as isize;
            },
            0x002b if lparam != 0 => {
                let item = unsafe { &*(lparam as *const DrawItem) };
                paint_button(item, dialog.visuals, false, item.id == 1, None);
                return 1;
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

pub(super) fn confirm(editor: &Editor<'_>, title: &str, message: &str, action: &str) -> bool {
    themed_confirmation(editor, title, message, action)
        .unwrap_or_else(|| alert(editor.hwnd, title, message, 0x134) == 6)
}

fn themed_confirmation(
    editor: &Editor<'_>,
    title: &str,
    message: &str,
    action: &str,
) -> Option<bool> {
    let name = wide("UnityYamlViewer.RtfConfirmation");
    let instance = unsafe { GetModuleHandleW(null()) };
    static REGISTERED: OnceLock<bool> = OnceLock::new();
    if !REGISTERED.get_or_init(|| {
        let class = WindowClass {
            style: 0,
            proc: confirm_proc,
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: null_mut(),
            cursor: unsafe { LoadCursorW(null_mut(), 32512 as *const u16) },
            background: null_mut(),
            menu: null(),
            name: name.as_ptr(),
        };
        unsafe { RegisterClassW(&class) != 0 }
    }) {
        return None;
    }

    let mut owner = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        GetWindowRect(editor.hwnd, &mut owner);
    }
    let width = 430;
    let height = 160;
    let x = owner.left + (owner.right - owner.left - width) / 2;
    let y = owner.top + (owner.bottom - owner.top - height) / 2;
    let window = unsafe {
        CreateWindowExW(
            1,
            name.as_ptr(),
            wide(title).as_ptr(),
            0x80c80000,
            x,
            y,
            width,
            height,
            editor.hwnd,
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if window.is_null() {
        return None;
    }
    let child = |class: &str, label: &str, style, id, x, y, width, height| unsafe {
        let child = CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            style,
            x,
            y,
            width,
            height,
            window,
            id as Handle,
            instance,
            null_mut(),
        );
        SendMessageW(child, 0x0030, GetStockObject(17) as usize, 1);
        child
    };
    let body = child("STATIC", message, 0x50000000, 0, 16, 16, 380, 50);
    let accept = child("BUTTON", action, 0x5001000b, 1, 128, 76, 162, 29);
    let cancel = child("BUTTON", tr("取消"), 0x5001000b, 2, 300, 76, 104, 29);
    if body.is_null() || accept.is_null() || cancel.is_null() {
        unsafe {
            DestroyWindow(window);
        }
        return None;
    }

    let style = editor.ctx.style();
    let dialog = Confirmation {
        visuals: &style.visuals,
        result: Cell::new(None),
    };
    unsafe {
        SetWindowLongPtrW(window, -21, &dialog as *const _ as isize);
        let dark = i32::from(style.visuals.dark_mode);
        DwmSetWindowAttribute(window, 20, &dark, 4);
        EnableWindow(editor.hwnd, 0);
        ShowWindow(window, 5);
        SetFocus(cancel);
        let mut event: Message = std::mem::zeroed();
        while dialog.result.get().is_none() {
            let status = GetMessageW(&mut event, null_mut(), 0, 0);
            if status <= 0 {
                if status == 0 {
                    PostQuitMessage(event.wparam as i32);
                }
                break;
            }
            if event.message == 0x0100 {
                match event.wparam {
                    0x1b => {
                        dialog.result.set(Some(false));
                        continue;
                    }
                    0x0d => {
                        dialog.result.set(Some(GetFocus() == accept));
                        continue;
                    }
                    0x09 => {
                        SetFocus(if GetFocus() == accept { cancel } else { accept });
                        continue;
                    }
                    _ => {}
                }
            }
            TranslateMessage(&event);
            DispatchMessageW(&event);
        }
        EnableWindow(editor.hwnd, 1);
        DestroyWindow(window);
    }
    Some(dialog.result.get().unwrap_or(false))
}
