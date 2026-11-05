use std::{fs, io, path::PathBuf};

#[derive(Clone,serde::Serialize,serde::Deserialize)]
pub struct OpenRequest {
    pub paths:Vec<PathBuf>,
    pub preview:Option<crate::asset_viewer::nine_slice::processing::Request>,
    #[serde(default)] pub notice:String,
    #[serde(default)] pub browser:Option<BrowserRequest>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserRequest { pub url: String, pub referer: Option<String> }

pub fn parse_browser_request(input: &str) -> Result<BrowserRequest, String> {
    if input.len() > 32_000 || input.chars().any(char::is_control) { return Err("Invalid browser URL length or characters".into()); }
    let uri = reqwest::Url::parse(input).map_err(|_| "Invalid browser protocol URL")?;
    if uri.scheme() != "unityyamlviewer" || uri.host_str() != Some("open") || !matches!(uri.path(), "" | "/")
        || uri.fragment().is_some() || !uri.username().is_empty() || uri.password().is_some() || uri.port().is_some() {
        return Err("Expected unityyamlviewer://open?url=...".into());
    }
    let mut url = None;
    let mut referer = None;
    for (key, value) in uri.query_pairs() {
        let slot = match key.as_ref() { "url" => &mut url, "referer" => &mut referer, _ => return Err("Unknown browser URL parameter".into()) };
        if slot.is_some() { return Err("Duplicate browser URL parameter".into()); }
        let parsed = reqwest::Url::parse(&value).map_err(|_| "Invalid HTTP URL")?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none()
            || !parsed.username().is_empty() || parsed.password().is_some() || value.chars().any(char::is_control) {
            return Err("Only HTTP/HTTPS browser URLs without embedded credentials are supported".into());
        }
        *slot = Some(value.into_owned());
    }
    Ok(BrowserRequest { url: url.ok_or("Missing url parameter")?, referer })
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    #[test]
    fn browser_protocol_preserves_nested_signature_encoding_and_ipc_payload() {
        let url = "https://example.org/video.m3u8?a=%26%2B&b=a+b&name=中文";
        let mut protocol = reqwest::Url::parse("unityyamlviewer://open").unwrap();
        protocol.query_pairs_mut().append_pair("url", url).append_pair("referer", "https://example.org/watch/1");
        let browser = parse_browser_request(protocol.as_str()).unwrap();
        assert_eq!(browser.url, url);
        assert_eq!(browser.referer.as_deref(), Some("https://example.org/watch/1"));
        let request = OpenRequest { paths: Vec::new(), preview: None, notice: String::new(), browser: Some(browser) };
        let restored: OpenRequest = serde_json::from_slice(&serde_json_bytes(&request).unwrap()).unwrap();
        assert_eq!(restored.browser.unwrap().url, url);
        assert!(serde_json::from_str::<OpenRequest>(r#"{"paths":[],"preview":null,"notice":""}"#).unwrap().browser.is_none());
    }
    #[test]
    fn browser_protocol_rejects_local_files_commands_and_ambiguous_parameters() {
        for input in ["unityyamlviewer://open?url=file%3A%2F%2F%2FC%3A%2Fa.txt",
            "unityyamlviewer://open?url=javascript%3Aalert(1)", "unityyamlviewer://delete?url=https%3A%2F%2Fexample.org",
            "unityyamlviewer://open?url=https%3A%2F%2Fexample.org&url=https%3A%2F%2Fother.org",
            "unityyamlviewer://open?url=https%3A%2F%2Fexample.org&command=run",
            "unityyamlviewer://open?url=https%3A%2F%2Fuser%3Apass%40example.org",
            "unityyamlviewer://open?url=https%3A%2F%2Fexample.org&referer=file%3A%2F%2Fx",
            "unityyamlviewer://open"] {
            assert!(parse_browser_request(input).is_err(), "{input}");
        }
    }
}
pub struct Inbox {
    _lock: fs::File,
    directory: PathBuf,
    next_poll: std::time::Instant,
}

pub enum Startup {
    Primary(Option<Inbox>, OpenRequest),
    Forwarded,
}

pub fn start() -> io::Result<Startup> {
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.first().and_then(|arg| arg.to_str()).is_some_and(|arg| arg.to_ascii_lowercase().starts_with("unityyamlviewer:")) {
        if args.len() != 1 { return Err(io::Error::other("Browser protocol accepts one argument")); }
        let browser = parse_browser_request(args[0].to_str().ok_or_else(|| io::Error::other("Invalid browser URL"))?)
            .map_err(io::Error::other)?;
        return start_with_paths(OpenRequest { paths: Vec::new(), preview: None, notice: String::new(), browser: Some(browser) });
    }
    if args.first().is_some_and(|a|a=="slice9") {
        let request=crate::asset_viewer::nine_slice::cli::preview_request(args.into_iter().skip(1).collect()).map_err(io::Error::other)?;
        return start_with_paths(request)
    }
    let cwd=std::env::current_dir()?;
    let paths=args.into_iter().map(|a|{let p=PathBuf::from(a);let p=if p.is_absolute(){p}else{cwd.join(p)};fs::canonicalize(&p).unwrap_or(p)}).collect();
    let paths=OpenRequest {paths,preview:None,notice:String::new(),browser:None};
    start_with_paths(paths)
}

#[cfg(not(windows))]
fn start_with_paths(paths: OpenRequest) -> io::Result<Startup> {
    Ok(Startup::Primary(None, paths))
}

#[cfg(windows)]
fn start_with_paths(paths: OpenRequest) -> io::Result<Startup> {
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

fn serde_json_bytes(paths: &OpenRequest) -> io::Result<Vec<u8>> {
    // A JSON array preserves spaces, Unicode and multiple file arguments.
    serde_json::to_vec(paths).map_err(io::Error::other)
}

impl Inbox {
    pub fn receive(&mut self) -> Vec<OpenRequest> {
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
            let bytes=fs::read(&path).ok()?;
            let paths=serde_json::from_slice::<OpenRequest>(&bytes).or_else(|_|serde_json::from_slice::<Vec<PathBuf>>(&bytes).map(|paths|OpenRequest {paths,preview:None,notice:String::new(),browser:None})).ok()?;
            fs::remove_file(path).ok()?;
            Some(paths)
        }).collect()
    }
}


const PNGOPT_HELP: &str = r#"pngopt [-all] <file-or-directory> [...]
  -all          Include subdirectories (skip symbolic links and junctions)
  -h, --help    Show this help
  --            Treat remaining arguments as paths

Examples (PowerShell):
  .\unity_yaml_viewer.exe pngopt "a.png" "b.png"
  .\unity_yaml_viewer.exe pngopt "Assets\UI" "other.png"

By default, directories include only immediate PNG files. Duplicate paths are processed once.
Removes iTXt metadata. Alpha is removed only from static 32-bit RGBA PNGs
whose pixels all have alpha 255. Writes in place without backups.
Output: JSON on stdout. Exit 0: no failures; 1: file failures; 2: argument error.
"#;

pub fn help() -> String {
    format!("{}\n{PNGOPT_HELP}", crate::asset_viewer::nine_slice::cli::HELP)
}

#[cfg(windows)]
pub(crate) fn attach_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    unsafe { AttachConsole(u32::MAX); }
}

#[cfg(not(windows))]
pub(crate) fn attach_console() {}

pub fn dispatch_pngopt() -> Option<i32> {
    use crate::asset_viewer::image_info::{optimize_png, Optimization};
    use serde_json::json;
    use std::{collections::BTreeSet, path::Path};

    let mut args = std::env::args_os().skip(1);
    if args.next()? != "pngopt" {
        return None;
    }
    attach_console();
    let mut inputs = Vec::new();
    let mut literal = false;
    let mut recursive = false;
    for arg in args {
        if !literal {
            match arg.to_str() {
                Some("-h" | "--help") => {
                    println!("{PNGOPT_HELP}");
                    return Some(0);
                }
                Some("-all") => {
                    recursive = true;
                    continue;
                }
                Some("--") => {
                    literal = true;
                    continue;
                }
                Some(option) if option.starts_with('-') => {
                    eprintln!("Unknown option: {option}\nUse pngopt --help");
                    return Some(2);
                }
                _ => {}
            }
        }
        inputs.push(PathBuf::from(arg));
    }
    if inputs.is_empty() {
        eprintln!("Provide at least one PNG file or directory. Use pngopt --help.");
        return Some(2);
    }

    let is_png = |path: &Path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
    let mut files = BTreeSet::new();
    let mut records = Vec::new();
    let mut pending: Vec<_> = inputs.into_iter().map(|path| (path, true)).collect();
    while let Some((path, explicit)) = pending.pop() {
        let mut record_failure = |reason: String| {
            records.push(json!({"path": path.to_string_lossy(), "status": "failed", "reason": reason}));
        };
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                record_failure(error.to_string());
                continue;
            }
        };
        // Windows junctions and other reparse points can redirect traversal too.
        #[cfg(windows)]
        let is_link = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let is_link = metadata.file_type().is_symlink();
        if is_link {
            if explicit {
                record_failure("Symbolic links and junctions are not supported".to_owned());
            }
        } else if metadata.is_dir() && (explicit || recursive) {
            match fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => match entry.file_type() {
                                Ok(kind) if (recursive && kind.is_dir()) || (kind.is_file() && is_png(&entry.path())) => {
                                    pending.push((entry.path(), false));
                                }
                                Ok(_) => {}
                                Err(error) => record_failure(error.to_string()),
                            },
                            Err(error) => record_failure(error.to_string()),
                        }
                    }
                }
                Err(error) => record_failure(error.to_string()),
            }
        } else if metadata.is_file() && is_png(&path) {
            match fs::canonicalize(&path) {
                Ok(path) => { files.insert(path); }
                Err(error) => record_failure(error.to_string()),
            }
        } else if explicit {
            record_failure("Expected a regular PNG file or directory".to_owned());
        }
    }
    for path in files {
        let result = optimize_png(&path);
        let path = path.to_string_lossy();
        records.push(match result {
            Ok(result) => {
                let status = match result {
                    Optimization::Unchanged => "unchanged",
                    Optimization::Alpha => "alpha",
                    Optimization::Text => "itxt",
                    Optimization::Both => "both",
                };
                json!({"path": path, "status": status})
            }
            Err(reason) => json!({"path": path, "status": "failed", "reason": reason}),
        });
    }
    let count = |status| records.iter().filter(|record| record["status"] == status).count();
    let failed = count("failed");
    let output = json!({
        "summary": {"alpha": count("alpha"), "itxt": count("itxt"), "both": count("both"),
                    "unchanged": count("unchanged"), "failed": failed},
        "files": records,
    });
    println!("{output:#}");
    Some(if failed == 0 { 0 } else { 1 })
}
