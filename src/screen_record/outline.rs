//! Native green region frame and pointer selection; controls are app-rendered viewports.
pub(super) use super::toolbar::{Action, Toolbar};
use super::{Area, screen};
use eframe::egui;
use std::sync::mpsc;
#[derive(Clone, Copy, PartialEq)]
struct Target {
    area: Area,
    editable: bool,
    escape_enabled: bool,
}
enum Command {
    Show(Target),
    Toolbar([isize; 2]),
    Hide,
    Select(Area),
    Exit,
}
enum Event {
    Ready,
    Changed(Area),
    Selected(Area),
    Error(String),
    Action(Action),
}
pub(super) struct Outline {
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<Event>,
    last: Option<Target>,
    handles: [isize; 2],
    visible: bool,
    pub ready: bool,
    pub selecting: bool,
    pub error: Option<String>,
    pub actions: Vec<Action>,
}
impl Outline {
    pub fn new(ctx: &egui::Context) -> Self {
        let (commands, receiver) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            #[cfg(windows)]
            let result = native::run(receiver, &sender, &ctx);
            #[cfg(not(windows))]
            let result: Result<(), String> = {
                drop(receiver);
                Err(crate::localization::tr("录屏当前仅支持 Windows。").into())
            };
            if let Err(error) = result {
                let _ = sender.send(Event::Error(error));
                ctx.request_repaint();
            }
        });
        Self {
            commands,
            events,
            last: None,
            handles: [0; 2],
            visible: false,
            ready: false,
            selecting: false,
            error: None,
            actions: vec![],
        }
    }
    pub fn poll(&mut self) -> Option<Area> {
        let mut area = None;
        loop {
            match self.events.try_recv() {
                Ok(Event::Ready) => self.ready = true,
                Ok(Event::Changed(value)) => area = Some(value),
                Ok(Event::Selected(value)) => {
                    area = Some(value);
                    self.selecting = false;
                    self.last = None;
                }
                Ok(Event::Action(action)) => {
                    if matches!(action, Action::Close) {
                        self.selecting = false;
                    }
                    self.actions.push(action);
                }
                Ok(Event::Error(error)) => {
                    self.error = Some(error);
                    self.ready = false;
                    self.selecting = false;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ready = false;
                    break;
                }
            }
        }
        if let (Some(value), Some(last)) = (area, self.last.as_mut()) {
            last.area = value;
        }
        area
    }
    pub fn show(&mut self, area: Area, editable: bool, escape_enabled: bool) {
        if self.selecting {
            return;
        }
        let target = Target {
            area,
            editable,
            escape_enabled,
        };
        if self.last != Some(target) || !self.visible {
            let _ = self.commands.send(Command::Show(target));
            self.last = Some(target);
            self.visible = true;
        }
    }
    pub fn attach_toolbar(&mut self, handles: [isize; 2]) {
        if self.handles != handles {
            let _ = self.commands.send(Command::Toolbar(handles));
            self.handles = handles;
        }
    }
    pub fn hide(&mut self) {
        if self.visible || self.selecting {
            let _ = self.commands.send(Command::Hide);
            self.visible = false;
            self.selecting = false;
            self.last = None;
        }
    }
    pub fn select(&mut self, area: Area) {
        let _ = self.commands.send(Command::Select(area));
        self.selecting = true;
        self.visible = true;
        self.last = None;
    }
}
impl Drop for Outline {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Exit);
    }
}
#[cfg(windows)]
mod native {
    use super::*;
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
        time::Duration,
    };
    type Handle = *mut c_void;
    #[derive(Clone, Copy, Default)]
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    impl Rect {
        fn area(self) -> Area {
            Area {
                x: self.left,
                y: self.top,
                width: self.right - self.left,
                height: self.bottom - self.top,
            }
        }
    }
    #[derive(Clone, Copy, Default)]
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct MinMax {
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
    struct Class {
        style: u32,
        proc: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
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
    struct Version {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        service: [u16; 128],
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn RtlGetVersion(version: *mut Version) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Handle;
    }
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmGetWindowAttribute(
            hwnd: Handle,
            attribute: u32,
            value: *mut c_void,
            size: u32,
        ) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateSolidBrush(color: u32) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> Handle;
        fn CombineRgn(dest: Handle, first: Handle, second: Handle, mode: i32) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(class: *const c_void) -> u16;
        fn UnregisterClassW(name: *const u16, instance: Handle) -> i32;
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
        fn DestroyWindow(hwnd: Handle) -> i32;
        fn DefWindowProcW(hwnd: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
        #[cfg_attr(target_pointer_width = "32", link_name = "GetWindowLongW")]
        fn GetWindowLongPtrW(hwnd: Handle, index: i32) -> isize;
        #[cfg_attr(target_pointer_width = "32", link_name = "SetWindowLongW")]
        fn SetWindowLongPtrW(hwnd: Handle, index: i32, value: isize) -> isize;
        fn SetWindowPos(
            hwnd: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn SetWindowRgn(hwnd: Handle, region: Handle, redraw: i32) -> i32;
        fn SetLayeredWindowAttributes(hwnd: Handle, color: u32, alpha: u8, flags: u32) -> i32;
        fn SetWindowDisplayAffinity(hwnd: Handle, affinity: u32) -> i32;
        fn GetWindowDisplayAffinity(hwnd: Handle, affinity: *mut u32) -> i32;
        fn ShowWindow(hwnd: Handle, command: i32) -> i32;
        fn GetWindowRect(hwnd: Handle, rect: *mut Rect) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn GetAsyncKeyState(key: i32) -> i16;
        fn GetSystemMetrics(index: i32) -> i32;
        fn IsWindowVisible(hwnd: Handle) -> i32;
        fn IsWindow(hwnd: Handle) -> i32;
        fn IsIconic(hwnd: Handle) -> i32;
        fn EnumWindows(
            callback: Option<unsafe extern "system" fn(Handle, isize) -> i32>,
            data: isize,
        ) -> i32;
        fn GetClassNameW(hwnd: Handle, name: *mut u16, count: i32) -> i32;
        fn PeekMessageW(
            message: *mut Message,
            hwnd: Handle,
            first: u32,
            last: u32,
            remove: u32,
        ) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
        fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
        fn SetCursor(cursor: Handle) -> Handle;
        fn SetCapture(hwnd: Handle) -> Handle;
        fn GetCapture() -> Handle;
        fn ReleaseCapture() -> i32;
        fn ClientToScreen(hwnd: Handle, point: *mut Point) -> i32;
    }
    fn win_error() -> String {
        format!(
            "{}: {}",
            crate::localization::tr("无法创建可排除捕获的录屏线框。"),
            std::io::Error::last_os_error()
        )
    }
    fn bounded(area: Area) -> Area {
        fit_region(area, screen(true))
    }
    fn fit_region(area: Area, desktop: Area) -> Area {
        if desktop.width < 16 || desktop.height < 16 {
            return Area {
                x: desktop.x,
                y: desktop.y,
                width: desktop.width.max(0),
                height: desktop.height.max(0),
            };
        }
        let width = area.width.clamp(16, desktop.width.max(16));
        let height = area.height.clamp(16, desktop.height.max(16));
        Area {
            x: area.x.clamp(desktop.x, desktop.x + desktop.width - width),
            y: area.y.clamp(desktop.y, desktop.y + desktop.height - height),
            width,
            height,
        }
    }
    fn resized_area(rect: Rect, edge: usize, desktop: Area) -> Area {
        if desktop.width < 16 || desktop.height < 16 {
            return fit_region(rect.area(), desktop);
        }
        let (left, right) = if matches!(edge, 2 | 5 | 8) {
            let left = rect.left.clamp(desktop.x, desktop.x + desktop.width - 16);
            (left, rect.right.clamp(left + 16, desktop.x + desktop.width))
        } else {
            let right = rect.right.clamp(desktop.x + 16, desktop.x + desktop.width);
            (rect.left.clamp(desktop.x, right - 16), right)
        };
        let (top, bottom) = if matches!(edge, 6 | 7 | 8) {
            let top = rect.top.clamp(desktop.y, desktop.y + desktop.height - 16);
            (top, rect.bottom.clamp(top + 16, desktop.y + desktop.height))
        } else {
            let bottom = rect
                .bottom
                .clamp(desktop.y + 16, desktop.y + desktop.height);
            (rect.top.clamp(desktop.y, bottom - 16), bottom)
        };
        Area {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
    #[derive(Clone, Copy)]
    struct SelectionDrag {
        start: Point,
        snapped: Area,
        dragging: bool,
    }
    impl SelectionDrag {
        fn update(&mut self, point: Point, threshold: [i32; 2], desktop: Area) -> Area {
            self.dragging |= (i64::from(point.x) - i64::from(self.start.x)).abs()
                >= i64::from(threshold[0].max(1))
                || (i64::from(point.y) - i64::from(self.start.y)).abs()
                    >= i64::from(threshold[1].max(1));
            if !self.dragging {
                return self.snapped;
            }
            let start = Point {
                x: self.start.x.clamp(desktop.x, desktop.x + desktop.width),
                y: self.start.y.clamp(desktop.y, desktop.y + desktop.height),
            };
            let end = Point {
                x: point.x.clamp(desktop.x, desktop.x + desktop.width),
                y: point.y.clamp(desktop.y, desktop.y + desktop.height),
            };
            let width = (i64::from(end.x) - i64::from(start.x))
                .abs()
                .max(16)
                .min(i64::from(desktop.width)) as i32;
            let height = (i64::from(end.y) - i64::from(start.y))
                .abs()
                .max(16)
                .min(i64::from(desktop.height)) as i32;
            fit_region(
                Area {
                    x: if end.x >= start.x {
                        start.x
                    } else {
                        start.x - width
                    },
                    y: if end.y >= start.y {
                        start.y
                    } else {
                        start.y - height
                    },
                    width,
                    height,
                },
                desktop,
            )
        }
    }

    struct State {
        border: Handle,
        shield: Handle,
        toolbar: [Handle; 2],
        area: Area,
        fallback: Area,
        editable: bool,
        selecting: bool,
        selection_drag: Option<SelectionDrag>,
        move_drag: Option<MoveDrag>,
        programmatic: bool,
        rebuilding_region: bool,
        visible: bool,
        escape_enabled: bool,
        events: mpsc::Sender<Event>,
        ctx: egui::Context,
    }
    #[derive(Clone, Copy)]
    struct MoveDrag {
        start: Point,
        area: Area,
    }
    fn moved_area(drag: MoveDrag, point: Point, desktop: Area) -> Area {
        let area = fit_region(drag.area, desktop);
        let x = (i64::from(drag.area.x) + i64::from(point.x) - i64::from(drag.start.x)).clamp(
            i64::from(desktop.x),
            i64::from(desktop.x + desktop.width - area.width),
        ) as i32;
        let y = (i64::from(drag.area.y) + i64::from(point.y) - i64::from(drag.start.y)).clamp(
            i64::from(desktop.y),
            i64::from(desktop.y + desktop.height - area.height),
        ) as i32;
        Area { x, y, ..area }
    }
    fn border_hit(area: Area, point: Point, ctrl: bool) -> isize {
        let x = point.x - area.x;
        let y = point.y - area.y;
        let grip = 12.min(area.width.min(area.height) / 3).max(3);
        if x < grip && y < grip {
            13
        } else if x >= area.width - grip && y < grip {
            14
        } else if x < grip && y >= area.height - grip {
            16
        } else if x >= area.width - grip && y >= area.height - grip {
            17
        } else if (x - area.width / 2).abs() <= grip / 2 && y < grip {
            12
        } else if (x - area.width / 2).abs() <= grip / 2 && y >= area.height - grip {
            15
        } else if (y - area.height / 2).abs() <= grip / 2 && x < grip {
            10
        } else if (y - area.height / 2).abs() <= grip / 2 && x >= area.width - grip {
            11
        } else if y >= area.height - 4 {
            15
        } else if x < 4 {
            10
        } else if x >= area.width - 4 {
            11
        } else if ctrl {
            12
        } else {
            2
        }
    }
    unsafe fn raise_toolbar(state: &State) {
        unsafe {
            for hwnd in state.toolbar {
                if !hwnd.is_null() && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 {
                    SetWindowPos(hwnd, (-1isize) as Handle, 0, 0, 0, 0, 0x1 | 0x2 | 0x10);
                }
            }
        }
    }
    unsafe fn exit_tools(pointer: *mut State) {
        unsafe {
            if (*pointer).selecting {
                let area = (*pointer).fallback;
                set_area(pointer, area);
                emit(&*pointer, false);
            } else if let Some(drag) = (*pointer).move_drag {
                set_area(pointer, drag.area);
                emit(&*pointer, false);
            }
            (*pointer).selecting = false;
            (*pointer).visible = false;
            release_drag(pointer);
            ShowWindow((*pointer).shield, 0);
            ShowWindow((*pointer).border, 0);
            for hwnd in (*pointer).toolbar {
                if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                    ShowWindow(hwnd, 0);
                }
            }
            let _ = (*pointer).events.send(Event::Action(Action::Close));
            (*pointer).ctx.request_repaint();
        }
    }
    unsafe fn region(hwnd: Handle, width: i32, height: i32, editable: bool) {
        // A hollow window leaves the desktop clickable; larger corner grips ease resizing.
        let thickness = 3;
        let grip = if editable {
            12.min(width.min(height) / 3).max(3)
        } else {
            3
        };
        unsafe {
            let outer = CreateRectRgn(0, 0, width, height);
            let inner = CreateRectRgn(
                thickness,
                thickness,
                (width - thickness).max(thickness),
                (height - thickness).max(thickness),
            );
            CombineRgn(outer, outer, inner, 4);
            DeleteObject(inner);
            for (x, y) in [
                (0, 0),
                ((width - grip).max(0), 0),
                (0, (height - grip).max(0)),
                ((width - grip).max(0), (height - grip).max(0)),
            ] {
                let corner = CreateRectRgn(x, y, x + grip, y + grip);
                CombineRgn(outer, outer, corner, 2);
                DeleteObject(corner);
            }
            if editable {
                for (x, y) in [
                    (width / 2 - grip / 2, 0),
                    (width / 2 - grip / 2, height - grip),
                    (0, height / 2 - grip / 2),
                    (width - grip, height / 2 - grip / 2),
                ] {
                    let handle = CreateRectRgn(x, y, x + grip, y + grip);
                    CombineRgn(outer, outer, handle, 2);
                    DeleteObject(handle);
                }
            }
            if SetWindowRgn(hwnd, outer, 1) == 0 {
                DeleteObject(outer);
            }
        }
    }

    unsafe fn set_area(pointer: *mut State, area: Area) {
        unsafe {
            (*pointer).area = bounded(area);
            (*pointer).programmatic = true;
            let area = (*pointer).area;
            let border = (*pointer).border;
            SetWindowPos(
                border,
                (-1isize) as Handle,
                area.x,
                area.y,
                area.width,
                area.height,
                0x10,
            );
            update_region(pointer, area.width, area.height);
            update_interior(pointer);
            (*pointer).programmatic = false;
        }
    }
    unsafe fn style(pointer: *mut State, editable: bool) {
        unsafe {
            (*pointer).editable = editable;
            let border = (*pointer).border;
            let area = (*pointer).area;
            let ex = GetWindowLongPtrW(border, -20);
            SetWindowLongPtrW(border, -20, if editable { ex & !0x20 } else { ex | 0x20 });
            update_region(pointer, area.width, area.height);
            update_interior(pointer);
        }
    }
    unsafe fn update_interior(pointer: *mut State) {
        unsafe {
            if (*pointer).selecting {
                return;
            }
            if !(*pointer).editable || !(*pointer).visible {
                ShowWindow((*pointer).shield, 0);
                return;
            }
            let area = (*pointer).area;
            let previous = (*pointer).programmatic;
            (*pointer).programmatic = true;
            SetWindowPos(
                (*pointer).shield,
                (-1isize) as Handle,
                area.x,
                area.y,
                area.width,
                area.height,
                0x10 | 0x40,
            );
            SetWindowPos(
                (*pointer).border,
                (-1isize) as Handle,
                0,
                0,
                0,
                0,
                0x1 | 0x2 | 0x10,
            );
            (*pointer).programmatic = previous;
            raise_toolbar(&*pointer);
        }
    }
    unsafe fn update_region(pointer: *mut State, width: i32, height: i32) {
        unsafe {
            if (*pointer).rebuilding_region {
                return;
            }
            (*pointer).rebuilding_region = true;
            region((*pointer).border, width, height, (*pointer).editable);
            (*pointer).rebuilding_region = false;
        }
    }
    unsafe fn emit(state: &State, selected: bool) {
        let _ = state.events.send(if selected {
            Event::Selected(state.area)
        } else {
            Event::Changed(state.area)
        });
        state.ctx.request_repaint();
    }
    unsafe fn message_point(hwnd: Handle, lparam: isize) -> Point {
        let mut point = Point {
            x: (lparam as u16) as i16 as i32,
            y: ((lparam as usize >> 16) as u16) as i16 as i32,
        };
        unsafe {
            ClientToScreen(hwnd, &mut point);
        }
        point
    }
    unsafe fn release_drag(pointer: *mut State) {
        unsafe {
            (*pointer).selection_drag = None;
            (*pointer).move_drag = None;
            if GetCapture() == (*pointer).shield {
                ReleaseCapture();
            }
        }
    }
    unsafe fn update_drag(pointer: *mut State, point: Point) {
        unsafe {
            let Some(mut drag) = (*pointer).selection_drag else {
                return;
            };
            let area = drag.update(
                point,
                [GetSystemMetrics(68).max(4), GetSystemMetrics(69).max(4)],
                screen(true),
            );
            (*pointer).selection_drag = Some(drag);
            if area != (*pointer).area {
                set_area(pointer, area);
                emit(&*pointer, false);
            }
        }
    }
    unsafe fn finish_selection(pointer: *mut State, point: Point) {
        unsafe {
            update_drag(pointer, point);
            (*pointer).selecting = false;
            release_drag(pointer);
            ShowWindow((*pointer).shield, 0);
            style(pointer, true);
            raise_toolbar(&*pointer);
            emit(&*pointer, true);
        }
    }
    unsafe fn cancel_selection(pointer: *mut State) {
        unsafe {
            (*pointer).selecting = false;
            release_drag(pointer);
            ShowWindow((*pointer).shield, 0);
            let fallback = (*pointer).fallback;
            set_area(pointer, fallback);
            style(pointer, true);
            raise_toolbar(&*pointer);
            emit(&*pointer, true);
        }
    }
    unsafe fn update_move(pointer: *mut State, point: Point) {
        unsafe {
            let Some(drag) = (*pointer).move_drag else {
                return;
            };
            let area = moved_area(drag, point, screen(true));
            if area != (*pointer).area {
                set_area(pointer, area);
                emit(&*pointer, false);
            }
        }
    }
    unsafe fn finish_move(pointer: *mut State, point: Point) {
        unsafe {
            update_move(pointer, point);
            release_drag(pointer);
        }
    }
    unsafe fn cancel_move(pointer: *mut State) {
        unsafe {
            let Some(drag) = (*pointer).move_drag else {
                return;
            };
            release_drag(pointer);
            set_area(pointer, drag.area);
            emit(&*pointer, false);
        }
    }

    unsafe extern "system" fn proc(
        hwnd: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        unsafe {
            let pointer = GetWindowLongPtrW(hwnd, -21) as *mut State;
            if pointer.is_null() {
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            if message == 0x84 && hwnd == (*pointer).border {
                if !(*pointer).editable || (*pointer).selecting {
                    return -1;
                }
                let mut p = Point::default();
                GetCursorPos(&mut p);
                return border_hit((*pointer).area, p, GetAsyncKeyState(0x11) < 0);
            }
            if message == 0x21 {
                return 3;
            }
            if message == 0x20
                && hwnd == (*pointer).shield
                && ((*pointer).selecting || (*pointer).editable)
            {
                SetCursor(LoadCursorW(
                    null_mut(),
                    if (*pointer).selecting { 32515 } else { 32646 } as *const u16,
                ));
                return 1;
            }
            if hwnd == (*pointer).shield && !(*pointer).selecting && (*pointer).editable {
                if message == 0x201 {
                    let start = message_point(hwnd, lparam);
                    (*pointer).move_drag = Some(MoveDrag {
                        start,
                        area: (*pointer).area,
                    });
                    SetCapture(hwnd);
                    return 0;
                }
                if message == 0x200 && (*pointer).move_drag.is_some() {
                    let mut point = Point::default();
                    GetCursorPos(&mut point);
                    update_move(pointer, point);
                    return 0;
                }
                if message == 0x202 && (*pointer).move_drag.is_some() {
                    let mut point = Point::default();
                    GetCursorPos(&mut point);
                    finish_move(pointer, point);
                    return 0;
                }
                if (message == 0x204 || matches!(message, 0x1f | 0x215))
                    && (*pointer).move_drag.is_some()
                {
                    cancel_move(pointer);
                    return 0;
                }
            }
            if message == 0x83 && hwnd == (*pointer).border {
                return 0;
            }
            if message == 0x24 && hwnd == (*pointer).border && lparam != 0 {
                let limits = &mut *(lparam as *mut MinMax);
                let desktop = screen(true);
                limits.min_track = Point { x: 16, y: 16 };
                limits.max_track = Point {
                    x: desktop.width,
                    y: desktop.height,
                };
                return 0;
            }
            if message == 0x201 && hwnd == (*pointer).shield && (*pointer).selecting {
                let point = message_point(hwnd, lparam);
                let mut search = Search {
                    point,
                    skip: [
                        (*pointer).border,
                        (*pointer).shield,
                        (*pointer).toolbar[0],
                        (*pointer).toolbar[1],
                    ],
                    found: None,
                };
                EnumWindows(Some(find_window), (&mut search as *mut Search) as isize);
                let snapped = search.found.unwrap_or((*pointer).area);
                (*pointer).selection_drag = Some(SelectionDrag {
                    start: point,
                    snapped,
                    dragging: false,
                });
                set_area(pointer, snapped);
                SetCapture(hwnd);
                return 0;
            }
            if message == 0x200
                && hwnd == (*pointer).shield
                && (*pointer).selecting
                && (*pointer).selection_drag.is_some()
            {
                update_drag(pointer, message_point(hwnd, lparam));
                return 0;
            }
            if message == 0x202
                && hwnd == (*pointer).shield
                && (*pointer).selecting
                && (*pointer).selection_drag.is_some()
            {
                finish_selection(pointer, message_point(hwnd, lparam));
                return 0;
            }
            if message == 0x204 && hwnd == (*pointer).shield && (*pointer).selecting {
                cancel_selection(pointer);
                return 0;
            }
            if matches!(message, 0x1f | 0x215)
                && hwnd == (*pointer).shield
                && (*pointer).selecting
                && (*pointer).selection_drag.is_some()
            {
                cancel_selection(pointer);
                return 0;
            }
            if matches!(message, 0x214 | 0x216) && hwnd == (*pointer).border && lparam != 0 {
                let rect = &mut *(lparam as *mut Rect);
                let a = if message == 0x214 {
                    resized_area(*rect, wparam, screen(true))
                } else {
                    bounded(rect.area())
                };
                *rect = Rect {
                    left: a.x,
                    top: a.y,
                    right: a.x + a.width,
                    bottom: a.y + a.height,
                };
                return 1;
            }
            if message == 0x5 && hwnd == (*pointer).border {
                let mut rect = Rect::default();
                GetWindowRect(hwnd, &mut rect);
                let a = rect.area();
                update_region(pointer, a.width, a.height);
            }
            if message == 0x47 && hwnd == (*pointer).border && !(*pointer).programmatic {
                let mut rect = Rect::default();
                GetWindowRect(hwnd, &mut rect);
                (*pointer).area = rect.area();
                update_interior(pointer);
                emit(&*pointer, false);
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
    }
    struct Windows {
        handles: Vec<Handle>,
        class: Vec<u16>,
        instance: Handle,
        brush: Handle,
    }
    impl Drop for Windows {
        fn drop(&mut self) {
            unsafe {
                for hwnd in self.handles.drain(..) {
                    DestroyWindow(hwnd);
                }
                UnregisterClassW(self.class.as_ptr(), self.instance);
                DeleteObject(self.brush);
            }
        }
    }
    struct Search {
        point: Point,
        skip: [Handle; 4],
        found: Option<Area>,
    }
    unsafe extern "system" fn find_window(hwnd: Handle, data: isize) -> i32 {
        unsafe {
            let search = &mut *(data as *mut Search);
            if search.skip.contains(&hwnd) || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return 1;
            }
            let mut cloaked = 0u32;
            if DwmGetWindowAttribute(hwnd, 14, (&mut cloaked as *mut u32).cast(), 4) >= 0
                && cloaked != 0
            {
                return 1;
            }
            let mut class = [0u16; 64];
            let length = GetClassNameW(hwnd, class.as_mut_ptr(), 64);
            let class = String::from_utf16_lossy(&class[..length.max(0) as usize]);
            if matches!(
                class.as_str(),
                "Progman"
                    | "WorkerW"
                    | "Shell_TrayWnd"
                    | "Shell_SecondaryTrayWnd"
                    | "ComboLBox"
                    | "tooltips_class32"
            ) {
                return 1;
            }
            let mut rect = Rect::default();
            if DwmGetWindowAttribute(
                hwnd,
                9,
                (&mut rect as *mut Rect).cast(),
                std::mem::size_of::<Rect>() as u32,
            ) < 0
                && GetWindowRect(hwnd, &mut rect) == 0
            {
                return 1;
            }
            if rect.right - rect.left < 16 || rect.bottom - rect.top < 16 {
                return 1;
            }
            let p = search.point;
            if p.x >= rect.left && p.x < rect.right && p.y >= rect.top && p.y < rect.bottom {
                search.found = Some(bounded(rect.area()));
                return 0;
            }
            1
        }
    }

    pub(super) fn run(
        commands: mpsc::Receiver<Command>,
        events: &mpsc::Sender<Event>,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        unsafe {
            let mut version: Version = std::mem::zeroed();
            version.size = std::mem::size_of::<Version>() as u32;
            if RtlGetVersion(&mut version) < 0 || version.build < 19041 {
                return Err(
                    crate::localization::tr("录屏线框需要 Windows 10 2004 或更新版本。").into(),
                );
            }
            let instance = GetModuleHandleW(null());
            let brush = CreateSolidBrush(0x0050d020);
            let class: Vec<u16> = format!(
                "UnityYAMLViewer.RecordingFrame.{}.{}\0",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
            .encode_utf16()
            .collect();
            let definition = Class {
                style: 0,
                proc: Some(proc),
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: null_mut(),
                cursor: LoadCursorW(null_mut(), 32512 as *const u16),
                background: brush,
                menu: null(),
                name: class.as_ptr(),
            };
            if RegisterClassW((&definition as *const Class).cast()) == 0 {
                let error = win_error();
                DeleteObject(brush);
                return Err(error);
            }
            let mut windows = Windows {
                handles: vec![],
                class,
                instance,
                brush,
            };
            for index in 0..2 {
                let hwnd = CreateWindowExW(
                    0x80000 | 0x08000000 | 0x80 | 0x8 | if index == 0 { 0x20 } else { 0 },
                    windows.class.as_ptr(),
                    null(),
                    0x80000000 | if index == 0 { 0x40000 } else { 0 },
                    0,
                    0,
                    32,
                    32,
                    null_mut(),
                    null_mut(),
                    instance,
                    null_mut(),
                );
                if hwnd.is_null() {
                    return Err(win_error());
                }
                windows.handles.push(hwnd);
                if SetLayeredWindowAttributes(hwnd, 0, if index == 1 { 1 } else { 255 }, 2) == 0
                    || SetWindowDisplayAffinity(hwnd, 0x11) == 0
                {
                    return Err(win_error());
                }
                let mut affinity = 0;
                if GetWindowDisplayAffinity(hwnd, &mut affinity) == 0 || affinity != 0x11 {
                    return Err(win_error());
                }
            }
            let mut state = Box::new(State {
                border: windows.handles[0],
                shield: windows.handles[1],
                toolbar: [null_mut(); 2],
                area: screen(false),
                fallback: screen(false),
                editable: false,
                selecting: false,
                selection_drag: None,
                move_drag: None,
                programmatic: false,
                rebuilding_region: false,
                visible: false,
                escape_enabled: true,
                events: events.clone(),
                ctx: ctx.clone(),
            });
            for hwnd in &windows.handles {
                SetWindowLongPtrW(*hwnd, -21, (&raw mut *state) as isize);
            }
            let _ = events.send(Event::Ready);
            ctx.request_repaint();
            let mut escape_was_down = GetAsyncKeyState(0x1b) < 0;
            'running: loop {
                loop {
                    match commands.try_recv() {
                        Ok(Command::Exit) | Err(mpsc::TryRecvError::Disconnected) => break 'running,
                        Err(mpsc::TryRecvError::Empty) => break,
                        Ok(Command::Toolbar(handles)) => {
                            state.toolbar = handles.map(|handle| handle as Handle);
                            if state.selecting {
                                raise_toolbar(&state);
                            }
                        }
                        Ok(Command::Hide) => {
                            state.visible = false;
                            state.selecting = false;
                            release_drag(&raw mut *state);
                            for hwnd in &windows.handles {
                                ShowWindow(*hwnd, 0);
                            }
                        }
                        Ok(Command::Show(target)) => {
                            state.visible = true;
                            state.selecting = false;
                            state.escape_enabled = target.escape_enabled;
                            release_drag(&raw mut *state);
                            ShowWindow(state.shield, 0);
                            style(&raw mut *state, target.editable);
                            set_area(&raw mut *state, target.area);
                            ShowWindow(state.border, 4);
                        }
                        Ok(Command::Select(fallback)) => {
                            release_drag(&raw mut *state);
                            state.selecting = true;
                            state.visible = true;
                            state.escape_enabled = true;
                            state.fallback = fallback;
                            style(&raw mut *state, false);
                            let desktop = screen(true);
                            SetWindowPos(
                                state.shield,
                                (-1isize) as Handle,
                                desktop.x,
                                desktop.y,
                                desktop.width,
                                desktop.height,
                                0x10,
                            );
                            ShowWindow(state.shield, 4);
                            set_area(&raw mut *state, fallback);
                            ShowWindow(state.border, 4);
                            raise_toolbar(&state);
                        }
                    }
                }
                let escape = GetAsyncKeyState(0x1b) < 0;
                if state.visible && state.escape_enabled && escape && !escape_was_down {
                    exit_tools(&raw mut *state);
                }
                escape_was_down = escape;
                if state.selecting {
                    let mut point = Point::default();
                    GetCursorPos(&mut point);
                    if state.selection_drag.is_some() {
                        update_drag(&raw mut *state, point);
                        if GetAsyncKeyState(1) >= 0 {
                            finish_selection(&raw mut *state, point);
                        }
                    }
                    if state.selecting && state.selection_drag.is_none() {
                        let mut over_toolbar = false;
                        for hwnd in state.toolbar {
                            if !hwnd.is_null() && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0
                            {
                                let mut rect = Rect::default();
                                GetWindowRect(hwnd, &mut rect);
                                over_toolbar |= point.x >= rect.left
                                    && point.x < rect.right
                                    && point.y >= rect.top
                                    && point.y < rect.bottom;
                            }
                        }
                        if !over_toolbar {
                            let mut search = Search {
                                point,
                                skip: [
                                    state.border,
                                    state.shield,
                                    state.toolbar[0],
                                    state.toolbar[1],
                                ],
                                found: None,
                            };
                            EnumWindows(Some(find_window), (&mut search as *mut Search) as isize);
                            if let Some(area) = search.found {
                                if state.area != area {
                                    set_area(&raw mut *state, area);
                                    emit(&state, false);
                                }
                            }
                        }
                    }
                }
                if state.move_drag.is_some() {
                    let mut point = Point::default();
                    GetCursorPos(&mut point);
                    update_move(&raw mut *state, point);
                    if GetAsyncKeyState(1) >= 0 {
                        finish_move(&raw mut *state, point);
                    }
                }
                let mut message: Message = std::mem::zeroed();
                while PeekMessageW(&mut message, null_mut(), 0, 0, 1) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                std::thread::sleep(Duration::from_millis(16));
            }
            state.selecting = false;
            release_drag(&raw mut *state);
            for hwnd in &windows.handles {
                SetWindowLongPtrW(*hwnd, -21, 0);
            }
            drop(windows);
            drop(state);
            Ok(())
        }
    }
    #[cfg(test)]
    use super::super::toolbar::placement as toolbar_position;
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn region_interior_move_preserves_dimensions_and_clamps_negative_desktop() {
            let desktop = Area {
                x: -1920,
                y: -200,
                width: 3840,
                height: 1280,
            };
            let drag = MoveDrag {
                start: Point { x: -800, y: 200 },
                area: Area {
                    x: -1000,
                    y: 100,
                    width: 640,
                    height: 480,
                },
            };
            assert_eq!(
                moved_area(drag, Point { x: -600, y: 300 }, desktop),
                Area {
                    x: -800,
                    y: 200,
                    width: 640,
                    height: 480
                }
            );
            assert_eq!(
                moved_area(drag, Point { x: -5000, y: -5000 }, desktop),
                Area {
                    x: -1920,
                    y: -200,
                    width: 640,
                    height: 480
                }
            );
            assert_eq!(
                moved_area(drag, Point { x: 5000, y: 5000 }, desktop),
                Area {
                    x: 1280,
                    y: 600,
                    width: 640,
                    height: 480
                }
            );
        }
        #[test]
        fn middle_handles_resize_one_axis_and_corners_resize_both() {
            let area = Area {
                x: -200,
                y: 100,
                width: 200,
                height: 100,
            };
            for (x, y, expected) in [
                (100, 3, 12),
                (100, 97, 15),
                (3, 50, 10),
                (197, 50, 11),
                (3, 3, 13),
                (197, 3, 14),
                (3, 97, 16),
                (197, 97, 17),
            ] {
                assert_eq!(
                    border_hit(
                        area,
                        Point {
                            x: area.x + x,
                            y: area.y + y
                        },
                        false
                    ),
                    expected
                );
            }
        }
        #[test]
        fn resizing_preserves_opposite_edge_at_minimum_and_desktop_limits() {
            let desktop = Area {
                x: 0,
                y: 0,
                width: 1000,
                height: 800,
            };
            assert_eq!(
                resized_area(
                    Rect {
                        left: -300,
                        top: 100,
                        right: 300,
                        bottom: 400
                    },
                    1,
                    desktop
                ),
                Area {
                    x: 0,
                    y: 100,
                    width: 300,
                    height: 300
                }
            );
            assert_eq!(
                resized_area(
                    Rect {
                        left: 350,
                        top: 100,
                        right: 300,
                        bottom: 400
                    },
                    1,
                    desktop
                ),
                Area {
                    x: 284,
                    y: 100,
                    width: 16,
                    height: 300
                }
            );
            assert_eq!(
                resized_area(
                    Rect {
                        left: 100,
                        top: 100,
                        right: 90,
                        bottom: 400
                    },
                    2,
                    desktop
                ),
                Area {
                    x: 100,
                    y: 100,
                    width: 16,
                    height: 300
                }
            );
        }
        #[test]
        fn region_drag_preserves_click_snap_and_then_stays_in_drag_mode() {
            let desktop = Area {
                x: -1920,
                y: -200,
                width: 3840,
                height: 1280,
            };
            let snapped = Area {
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            };
            let mut drag = SelectionDrag {
                start: Point { x: 200, y: 200 },
                snapped,
                dragging: false,
            };
            assert_eq!(
                drag.update(Point { x: 202, y: 198 }, [4, 4], desktop),
                snapped
            );
            assert!(!drag.dragging);
            assert_eq!(
                drag.update(Point { x: 500, y: 400 }, [4, 4], desktop),
                Area {
                    x: 200,
                    y: 200,
                    width: 300,
                    height: 200
                }
            );
            assert!(drag.dragging);
            let near_start = drag.update(Point { x: 201, y: 201 }, [4, 4], desktop);
            assert_eq!((near_start.width, near_start.height), (16, 16));
            assert_ne!(near_start, snapped);
        }
        #[test]
        fn region_drag_supports_four_directions_negative_origins_and_screen_edges() {
            let desktop = Area {
                x: -1920,
                y: -200,
                width: 3840,
                height: 1280,
            };
            let snapped = desktop;
            for (x, y, expected_x, expected_y) in [
                (-200, 200, -500, 0),
                (-800, 200, -800, 0),
                (-200, -200, -500, -200),
                (-800, -200, -800, -200),
            ] {
                let mut drag = SelectionDrag {
                    start: Point { x: -500, y: 0 },
                    snapped,
                    dragging: false,
                };
                let area = drag.update(Point { x, y }, [4, 4], desktop);
                assert_eq!(
                    area,
                    Area {
                        x: expected_x,
                        y: expected_y,
                        width: 300,
                        height: 200
                    }
                );
            }
            let mut drag = SelectionDrag {
                start: Point { x: 1900, y: 1000 },
                snapped,
                dragging: false,
            };
            let area = drag.update(Point { x: 3000, y: 2000 }, [4, 4], desktop);
            assert_eq!(
                area,
                Area {
                    x: 1900,
                    y: 1000,
                    width: 20,
                    height: 80
                }
            );
            assert!(area.valid(desktop));
            let mut vertical = SelectionDrag {
                start: Point { x: 0, y: 0 },
                snapped,
                dragging: false,
            };
            assert_eq!(
                vertical.update(Point { x: 0, y: 100 }, [4, 4], desktop),
                Area {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 100
                }
            );
        }
        #[test]
        fn toolbar_prefers_free_space_and_avoids_monitor_gaps() {
            let work = Area {
                x: 0,
                y: 0,
                width: 1000,
                height: 800,
            };
            let below = toolbar_position(
                Area {
                    x: 100,
                    y: 100,
                    width: 600,
                    height: 300,
                },
                530,
                64,
                &[work],
            );
            assert_eq!(below, (170, 406));
            let above = toolbar_position(
                Area {
                    x: 100,
                    y: 550,
                    width: 600,
                    height: 220,
                },
                530,
                64,
                &[work],
            );
            assert_eq!(above, (170, 480));
            let side = toolbar_position(
                Area {
                    x: 0,
                    y: 0,
                    width: 400,
                    height: 800,
                },
                530,
                64,
                &[work],
            );
            assert_eq!(side, (406, 0));
            let negative = Area {
                x: -1200,
                y: 300,
                width: 1000,
                height: 800,
            };
            let placed = toolbar_position(
                Area {
                    x: -1100,
                    y: 400,
                    width: 600,
                    height: 300,
                },
                530,
                64,
                &[negative, work],
            );
            assert_eq!(placed, (-1030, 706));
            let full = toolbar_position(work, 530, 64, &[work]);
            assert!(full.0 >= 0 && full.1 >= 0 && full.0 + 530 <= 1000 && full.1 + 64 <= 800);
        }
        #[test]
        fn fitted_regions_remain_inside_negative_origin_desktop() {
            let desktop = Area {
                x: -1920,
                y: -200,
                width: 3840,
                height: 1280,
            };
            let small = fit_region(
                Area {
                    x: -2200,
                    y: -400,
                    width: 1,
                    height: 1,
                },
                desktop,
            );
            assert_eq!(
                small,
                Area {
                    x: -1920,
                    y: -200,
                    width: 16,
                    height: 16
                }
            );
            let edge = fit_region(
                Area {
                    x: 1900,
                    y: 1070,
                    width: 400,
                    height: 300,
                },
                desktop,
            );
            assert!(edge.valid(desktop));
            assert_eq!((edge.x, edge.y), (1520, 780));
            let large = fit_region(
                Area {
                    x: 0,
                    y: 0,
                    width: 8000,
                    height: 8000,
                },
                desktop,
            );
            assert_eq!(large, desktop);
        }
    }
}
