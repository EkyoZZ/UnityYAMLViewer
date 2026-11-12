use super::Area;
#[cfg(not(windows))]
use super::screen;

pub(super) struct Display {
    pub key: String,
    pub number: u32,
    pub primary: bool,
    pub area: Area,
    pub work: Area,
}
#[cfg(windows)]
pub(super) fn list() -> Vec<Display> {
    use std::ffi::c_void;
    #[derive(Default)]
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct Info {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumDisplayMonitors(
            dc: *mut c_void,
            clip: *const Rect,
            callback: Option<
                unsafe extern "system" fn(*mut c_void, *mut c_void, *mut Rect, isize) -> i32,
            >,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut c_void) -> i32;
    }
    unsafe extern "system" fn collect(
        monitor: *mut c_void,
        _dc: *mut c_void,
        _rect: *mut Rect,
        data: isize,
    ) -> i32 {
        unsafe {
            let mut info: Info = std::mem::zeroed();
            info.size = std::mem::size_of::<Info>() as u32;
            if GetMonitorInfoW(monitor, (&mut info as *mut Info).cast()) != 0 {
                let key = String::from_utf16_lossy(
                    &info.device[..info.device.iter().position(|c| *c == 0).unwrap_or(32)],
                );
                let number = key
                    .trim_start_matches("\\\\.\\DISPLAY")
                    .parse()
                    .unwrap_or(0);
                let area = Area {
                    x: info.monitor.left,
                    y: info.monitor.top,
                    width: info.monitor.right - info.monitor.left,
                    height: info.monitor.bottom - info.monitor.top,
                };
                if area.width > 0 && area.height > 0 {
                    (*(data as *mut Vec<Display>)).push(Display {
                        key,
                        number,
                        primary: info.flags & 1 != 0,
                        area,
                        work: Area {
                            x: info.work.left,
                            y: info.work.top,
                            width: info.work.right - info.work.left,
                            height: info.work.bottom - info.work.top,
                        },
                    });
                }
            }
            1
        }
    }
    let mut displays: Vec<Display> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(collect),
            (&mut displays as *mut Vec<Display>) as isize,
        );
    }
    displays.sort_by_key(|display| display.number);
    displays
}
#[cfg(not(windows))]
pub(super) fn list() -> Vec<Display> {
    vec![Display {
        key: "primary".into(),
        number: 1,
        primary: true,
        area: screen(false),
        work: screen(false),
    }]
}
