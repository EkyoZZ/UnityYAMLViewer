//! Shared, bounded UTF-8 diagnostic log. No media content is written here.
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, Once},
};

static LOCK: Mutex<()> = Mutex::new(());
static INIT: Once = Once::new();
static LOGGER: Logger = Logger;
const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn path() -> Option<PathBuf> {
    crate::last_open_dir_config_path().map(|path| path.with_file_name("error.log"))
}

pub(crate) fn write(level: &str, module: &str, message: &str) -> io::Result<PathBuf> {
    let _guard = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let path = path().ok_or_else(|| io::Error::other("Application log directory unavailable"))?;
    let entry = format!(
        "[{}] [{level}] [v{}] [pid:{}] [{module}] {}\n",
        timestamp(),
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        message
    );
    append(&path, &entry)?;
    Ok(path)
}

fn append(path: &Path, entry: &str) -> io::Result<()> {
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| io::Error::other("Invalid log path"))?,
    )?;
    if path
        .metadata()
        .is_ok_and(|meta| meta.len() + entry.len() as u64 > MAX_BYTES)
    {
        let previous = path.with_file_name("error.log.1");
        match std::fs::remove_file(&previous) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        std::fs::rename(path, previous)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    // Preserve the diagnostic tail if one error contains an unusually large dump.
    let mut start = entry.len().saturating_sub(MAX_BYTES as usize - 64);
    while !entry.is_char_boundary(start) {
        start += 1;
    }
    if start > 0 {
        file.write_all(b"[earlier content of this entry truncated]\n")?;
    }
    file.write_all(entry[start..].as_bytes())?;
    file.flush()?;
    Ok(())
}

pub(crate) fn error(module: &str, message: impl std::fmt::Display) {
    let _ = write("ERROR", module, &message.to_string());
}

pub(crate) fn init() {
    INIT.call_once(|| {
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            error(
                "panic",
                format!("{info}\n{}", std::backtrace::Backtrace::force_capture()),
            );
            previous(info);
        }));
    });
}
struct Logger;
impl log::Log for Logger {
    fn enabled(&self, meta: &log::Metadata<'_>) -> bool {
        meta.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let _ = write(
                record.level().as_str(),
                record.target(),
                &record.args().to_string(),
            );
        }
    }
    fn flush(&self) {}
}

#[cfg(windows)]
fn timestamp() -> String {
    #[repr(C)]
    #[derive(Default)]
    struct LocalTime {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut LocalTime);
    }
    let mut time = LocalTime::default();
    unsafe {
        GetLocalTime(&mut time);
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        time.year, time.month, time.day, time.hour, time.minute, time.second, time.millis
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appends_utf8_and_retains_previous_log_when_rotating() {
        let directory = std::env::temp_dir().join(format!(
            "uyv-error-log-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("error.log");
        append(&path, "first failure\n").unwrap();
        append(&path, "录屏失败\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "first failure\n录屏失败\n"
        );
        let oversized = "采集错误".repeat(MAX_BYTES as usize);
        append(&path, &oversized).unwrap();
        assert_eq!(
            std::fs::read_to_string(directory.join("error.log.1")).unwrap(),
            "first failure\n录屏失败\n"
        );
        let current = std::fs::read_to_string(&path).unwrap();
        assert!(current.len() <= MAX_BYTES as usize);
        assert!(current.ends_with("采集错误"));
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
#[cfg(not(windows))]
fn timestamp() -> String {
    format!(
        "unix_ms={}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}
