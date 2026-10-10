use std::{fs, io, path::PathBuf};

pub struct Inbox {
    _lock: fs::File,
    directory: PathBuf,
    next_poll: std::time::Instant,
}

pub enum Startup {
    Primary(Option<Inbox>, Vec<PathBuf>),
    Forwarded,
}

pub fn start() -> io::Result<Startup> {
    let cwd = std::env::current_dir()?;
    let paths = std::env::args_os().skip(1).map(|argument| {
        let path = PathBuf::from(argument);
        let absolute = if path.is_absolute() { path } else { cwd.join(path) };
        fs::canonicalize(&absolute).unwrap_or(absolute)
    }).collect::<Vec<_>>();
    start_with_paths(paths)
}

#[cfg(not(windows))]
fn start_with_paths(paths: Vec<PathBuf>) -> io::Result<Startup> {
    Ok(Startup::Primary(None, paths))
}

#[cfg(windows)]
fn start_with_paths(paths: Vec<PathBuf>) -> io::Result<Startup> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    let directory = PathBuf::from(std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| io::Error::other("LOCALAPPDATA is unavailable"))?)
        .join("UnityYAMLViewer").join("open_requests");
    fs::create_dir_all(&directory)?;
    let acquire = || fs::OpenOptions::new().read(true).write(true).create(true)
        .truncate(false).share_mode(0).open(directory.join("instance.lock"));
    match acquire() {
        Ok(lock) => return Ok(Startup::Primary(Some(Inbox { _lock: lock, directory, next_poll: Instant::now() }), paths)),
        Err(error) if error.raw_os_error() == Some(32) => {},
        Err(error) => return Err(error),
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temporary = directory.join(format!("{}-{stamp}.tmp", std::process::id()));
    let request = temporary.with_extension("json");
    let bytes = serde_json_bytes(&paths)?;
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, &request)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !request.exists() { return Ok(Startup::Forwarded); }
        // If the original window closed during forwarding, become the new owner.
        if let Ok(lock) = acquire() {
            let _ = fs::remove_file(&request);
            return Ok(Startup::Primary(Some(Inbox { _lock: lock, directory, next_poll: Instant::now() }), paths));
        }
        if Instant::now() >= deadline {
            let _ = fs::remove_file(&request);
            return Err(io::Error::other("现有窗口未响应打开请求，请关闭对话框后重试。"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn serde_json_bytes(paths: &[PathBuf]) -> io::Result<Vec<u8>> {
    // A JSON array preserves spaces, Unicode and multiple file arguments.
    serde_json::to_vec(paths).map_err(io::Error::other)
}

impl Inbox {
    pub fn receive(&mut self) -> Vec<Vec<PathBuf>> {
        let now = std::time::Instant::now();
        if now < self.next_poll { return Vec::new(); }
        self.next_poll = now + std::time::Duration::from_millis(250);
        let Ok(entries) = fs::read_dir(&self.directory) else { return Vec::new() };
        let mut requests = entries.filter_map(Result::ok).map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
            .collect::<Vec<_>>();
        requests.sort();
        requests.into_iter().filter_map(|path| {
            if fs::metadata(&path).ok()?.len() > 1024 * 1024 { return None; }
            let paths = serde_json::from_slice::<Vec<PathBuf>>(&fs::read(&path).ok()?).ok()?;
            fs::remove_file(path).ok()?;
            Some(paths)
        }).collect()
    }
}
