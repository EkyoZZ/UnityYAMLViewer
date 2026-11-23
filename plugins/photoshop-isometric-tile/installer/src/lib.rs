//! Current-user deployment only. No registry, Photoshop installation or UXP database edits.
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const ID: &str = "unityyamlviewer.isometric-preview";
pub const FILES: [(&str, &[u8]); 10] = [
    ("manifest.json", include_bytes!("../../manifest.json")),
    ("index.html", include_bytes!("../../index.html")),
    ("main.js", include_bytes!("../../main.js")),
    ("png.js", include_bytes!("../../png.js")),
    ("tile-dark@1x.png", include_bytes!("../../tile-dark@1x.png")),
    ("tile-light@1x.png", include_bytes!("../../tile-light@1x.png")),
    ("tile-dark.png", include_bytes!("../../tile-dark.png")),
    ("tile-dark@2x.png", include_bytes!("../../tile-dark@2x.png")),
    ("tile-light.png", include_bytes!("../../tile-light.png")),
    ("tile-light@2x.png", include_bytes!("../../tile-light@2x.png")),
];
pub type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn version() -> String {
    serde_json::from_slice::<serde_json::Value>(FILES[0].1).unwrap()["version"]
        .as_str()
        .unwrap()
        .into()
}
pub fn target(appdata: &Path) -> PathBuf {
    appdata.join("Adobe/UXP/Plugins/External").join(ID)
}
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
fn plain(path: &Path, directory: bool) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(m) if is_link(&m) => Err(format!("拒绝操作链接或重解析点：{}", path.display())),
        Ok(m) if m.is_dir() != directory => Err(format!("路径类型不正确：{}", path.display())),
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}
// All mutations have a fixed destination beneath the supplied current-user APPDATA.
fn external(appdata: &Path, create: bool) -> Result<PathBuf> {
    if !appdata.is_absolute() || !plain(appdata, true)? {
        return Err("无法确定当前用户的 AppData 目录。".into());
    }
    let mut path = appdata.to_owned();
    for part in ["Adobe", "UXP", "Plugins", "External"] {
        path.push(part);
        if !plain(&path, true)? && create {
            fs::create_dir(&path).map_err(error)?;
        }
    }
    Ok(path)
}
fn checked_child(parent: &Path, path: &Path) -> Result<()> {
    if path.parent() != Some(parent) || !path.is_absolute() {
        return Err("目标路径超出插件安装目录。".into());
    }
    plain(parent, true)?;
    if path.exists() {
        plain(path, true)?;
    }
    Ok(())
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(parent: &Path) -> Result<Lock> {
    let path = parent.join(format!(".{ID}.lock"));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| {
            format!(
                "无法取得安装锁，请确认没有其他安装器正在操作。{}：{e}",
                path.display()
            )
        })?;
    Ok(Lock(path))
}
fn temporary(parent: &Path, label: &str) -> Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_nanos();
    let path = parent.join(format!(".{ID}.{label}.{}.{nonce}", std::process::id()));
    checked_child(parent, &path)?;
    fs::create_dir(&path).map_err(error)?;
    Ok(path)
}
fn files(directory: &Path) -> Result<Vec<PathBuf>> {
    plain(directory, true)?;
    let mut paths = Vec::new();
    let mut bytes = 0u64;
    for entry in fs::read_dir(directory).map_err(error)? {
        let path = entry.map_err(error)?.path();
        plain(&path, false)?;
        let meta = fs::symlink_metadata(&path).map_err(error)?;
        bytes = bytes.saturating_add(meta.len());
        paths.push(path);
        if paths.len() > 64 || bytes > 32 * 1024 * 1024 {
            return Err("已有插件目录超过 64 个文件或 32 MiB，请先自行备份整理。".into());
        }
    }
    Ok(paths)
}
fn digest(path: &Path) -> Result<Vec<u8>> {
    Ok(Sha256::digest(fs::read(path).map_err(error)?).to_vec())
}
// Flat directories only, never recursively delete or follow links.
fn remove_flat(parent: &Path, path: &Path) -> Result<()> {
    checked_child(parent, path)?;
    for file in files(path)? {
        fs::remove_file(file).map_err(error)?;
    }
    fs::remove_dir(path).map_err(error)
}
fn identity(directory: &Path) -> Result<String> {
    let path = directory.join("manifest.json");
    if !plain(&path, false)? {
        return Err("已有目录缺少 manifest.json，无法确认是本插件；未作修改。".into());
    }
    if fs::metadata(&path).map_err(error)?.len() > 1024 * 1024 {
        return Err("插件清单过大；未作修改。".into());
    }
    let m: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(error)?).map_err(error)?;
    if m["id"].as_str() != Some(ID) {
        return Err("目录属于其他插件；未作修改。".into());
    }
    Ok(m["version"].as_str().unwrap_or("未知版本").into())
}
pub fn installed(appdata: &Path) -> Result<Option<String>> {
    external(appdata, false)?;
    let path = target(appdata);
    if !plain(&path, true)? {
        return Ok(None);
    }
    identity(&path).map(Some)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallAction { Install, Update, Overwrite, NewerInstalled }
pub fn install_action(local: Option<&str>) -> Result<InstallAction> {
    let Some(local)=local else {return Ok(InstallAction::Install);};
    let local=semver::Version::parse(local).map_err(|e|format!("无法比较目录中的插件版本：{e}"))?;
    let bundled=semver::Version::parse(&version()).map_err(|e|format!("安装器内置版本无效：{e}"))?;
    Ok(match local.cmp(&bundled) {
        std::cmp::Ordering::Less=>InstallAction::Update,
        std::cmp::Ordering::Equal=>InstallAction::Overwrite,
        std::cmp::Ordering::Greater=>InstallAction::NewerInstalled,
    })
}
fn verify(directory: &Path) -> Result<()> {
    for (name, bytes) in FILES {
        let path = directory.join(name);
        plain(&path, false)?;
        if digest(&path)? != Sha256::digest(bytes).to_vec() {
            return Err(format!("安装文件校验失败：{name}"));
        }
    }
    Ok(())
}
pub fn install(appdata: &Path) -> Result<String> {
    install_with_check(appdata, require_photoshop_closed)
}
fn install_with_check(appdata: &Path, check: impl Fn() -> Result<()>) -> Result<String> {
    check()?;
    let parent = external(appdata, true)?;
    let _lock = lock(&parent)?;
    let dest = target(appdata);
    checked_child(&parent, &dest)?;
    let exists = plain(&dest, true)?;
    if exists {
        let local=identity(&dest)?;
        if install_action(Some(&local))? == InstallAction::NewerInstalled {
            return Err(format!("目录中的插件版本 {local} 比安装器内置版本 {} 更新，未覆盖。",version()));
        }
    }
    let stage = temporary(&parent, "stage")?;
    let prepared = (|| -> Result<()> {
        if exists {
            // Preserve unrelated regular files; refuse nested folders or links.
            for source in files(&dest)? {
                let out = stage.join(source.file_name().ok_or("无效文件名")?);
                fs::copy(&source, &out).map_err(error)?;
                if digest(&source)? != digest(&out)? {
                    return Err("备份内容校验失败，原安装未改动。".into());
                }
            }
        }
        for (name, bytes) in FILES {
            fs::write(stage.join(name), bytes).map_err(error)?;
        }
        verify(&stage)
    })();
    if let Err(e) = prepared {
        let _ = remove_flat(&parent, &stage);
        return Err(e);
    }
    // Check again immediately before replacing the live directory.
    if let Err(e) = check() {
        let _ = remove_flat(&parent, &stage);
        return Err(e);
    }
    let backup = if exists {
        let path = temporary(&parent, "backup")?;
        fs::remove_dir(&path).map_err(error)?;
        checked_child(&parent, &dest)?;
        checked_child(&parent, &path)?;
        if let Err(e) = fs::rename(&dest, &path) {
            let _ = remove_flat(&parent, &stage);
            return Err(error(e));
        }
        Some(path)
    } else {
        None
    };
    if let Err(e) = fs::rename(&stage, &dest) {
        let restore = backup.as_ref().map(|b| fs::rename(b, &dest));
        let _ = remove_flat(&parent, &stage);
        return Err(format!("安装未完成：{e}。恢复原目录：{restore:?}"));
    }
    if let Err(e) = verify(&dest) {
        // Keep the failed directory for inspection instead of discarding evidence.
        if let Err(move_error) = fs::rename(&dest, &stage) {
            return Err(format!(
                "{e}；无法移开失败目录：{move_error}；备份：{backup:?}"
            ));
        }
        let restore = backup.as_ref().map(|b| fs::rename(b, &dest));
        return Err(format!(
            "{e}；恢复原目录：{restore:?}；失败文件：{}",
            stage.display()
        ));
    }
    if let Some(backup) = backup {
        if let Err(e) = remove_flat(&parent, &backup) {
            return Ok(format!(
                "已安装并校验。旧备份保留在 {}：{e}。请重新启动 PS。",
                backup.display()
            ));
        }
    }
    Ok(format!("已部署并校验 {} 个插件文件；尚未确认 Photoshop 已加载。请重新启动 PS，在增效工具菜单中检查 Isometric Tile Preview。", FILES.len()))
}
pub fn uninstall(appdata: &Path) -> Result<String> {
    uninstall_with_check(appdata, require_photoshop_closed)
}
fn uninstall_with_check(appdata: &Path, check: impl Fn() -> Result<()>) -> Result<String> {
    check()?;
    let parent = external(appdata, false)?;
    let dest = target(appdata);
    if !plain(&dest, true)? {
        return Ok("插件尚未安装，无需卸载。".into());
    }
    let _lock = lock(&parent)?;
    checked_child(&parent, &dest)?;
    identity(&dest)?;
    for (name, _) in FILES {
        plain(&dest.join(name), false)?;
    }
    let stage = temporary(&parent, "uninstall")?;
    let mut moved: Vec<&str> = Vec::new();
    for (name, _) in FILES {
        let source = dest.join(name);
        if !source.exists() {
            continue;
        }
        if let Err(e) = fs::rename(&source, stage.join(name)) {
            let mut failures = Vec::new();
            for name in moved.into_iter().rev() {
                if let Err(e) = fs::rename(stage.join(name), dest.join(name)) {
                    failures.push(e.to_string());
                }
            }
            let _ = fs::remove_dir(&stage);
            return Err(format!(
                "卸载未完成：{e}；恢复异常：{failures:?}；暂存：{}",
                stage.display()
            ));
        }
        moved.push(name);
    }
    remove_flat(&parent, &stage)?;
    match fs::remove_dir(&dest) {
        Ok(()) => Ok("已卸载插件。没有修改 PS 程序文件、首选项或其他插件。".into()),
        Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {
            Ok("已移除四个插件文件；目录中的其他文件已保留。".into())
        }
        Err(e) => Ok(format!("插件文件已移除，目录保留：{e}")),
    }
}
pub fn require_photoshop_closed() -> Result<()> {
    if photoshop_running()? {
        Err("检测到 Photoshop 正在运行。请先关闭 PS，再点击重试；安装器不会自动关闭程序。".into())
    } else {
        Ok(())
    }
}
#[cfg(windows)]
pub fn photoshop_running() -> Result<bool> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
    };
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(error)?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = false;
        let result = (|| -> Result<()> {
            Process32FirstW(handle, &mut entry).map_err(error)?;
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..end])
                    .eq_ignore_ascii_case("Photoshop.exe")
                {
                    found = true;
                    break;
                }
                if let Err(e) = Process32NextW(handle, &mut entry) {
                    if e.code().0 as u32 != 0x80070012 {
                        return Err(error(e));
                    } // ERROR_NO_MORE_FILES
                    break;
                }
            }
            Ok(())
        })();
        let _ = CloseHandle(handle);
        result?;
        Ok(found)
    }
}
#[cfg(not(windows))]
pub fn photoshop_running() -> Result<bool> {
    Err("安装器目前只支持 Windows。".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_actions_allow_equal_version_overwrite() {
        assert_eq!(install_action(None).unwrap(),InstallAction::Install);
        let bundled=version();
        assert_eq!(install_action(Some(&bundled)).unwrap(),InstallAction::Overwrite);
        assert_eq!(install_action(Some(&format!("{bundled}-older"))).unwrap(),InstallAction::Update);
        let mut newer=semver::Version::parse(&bundled).unwrap(); newer.major+=1;
        assert_eq!(install_action(Some(&newer.to_string())).unwrap(),InstallAction::NewerInstalled);
        assert!(install_action(Some("未知版本")).is_err());
    }
    #[test]
    fn newer_deployment_is_not_replaced_by_an_older_installer() {
        let root=Fixture::new();
        install_with_check(&root.0,||Ok(())).unwrap();
        let path=target(&root.0).join("manifest.json");
        let mut manifest:serde_json::Value=serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut newer=semver::Version::parse(&version()).unwrap(); newer.major+=1;
        manifest["version"]=newer.to_string().into();
        let original=serde_json::to_vec(&manifest).unwrap();
        fs::write(&path,&original).unwrap();
        assert!(install_with_check(&root.0,||Ok(())).is_err());
        assert_eq!(fs::read(path).unwrap(),original);
    }
    #[test]
    fn embedded_manifest_matches_deployment_id() {
        let m: serde_json::Value = serde_json::from_slice(FILES[0].1).unwrap();
        assert_eq!(m["id"], ID);
        assert!(m["host"].is_object());
        assert_eq!(FILES.len(), 10);
    }
    #[test]
    fn destination_is_fixed() {
        let root = std::env::temp_dir();
        assert_eq!(
            target(&root),
            root.join("Adobe/UXP/Plugins/External").join(ID)
        );
        assert!(checked_child(&root, &root.join("../escape")).is_err());
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let parent = std::env::temp_dir();
            let path = temporary(&parent, "test").unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let dest = target(&self.0);
            if dest.exists() {
                let _ = remove_flat(dest.parent().unwrap(), &dest);
            }
            for suffix in [
                "Adobe/UXP/Plugins/External",
                "Adobe/UXP/Plugins",
                "Adobe/UXP",
                "Adobe",
                "",
            ] {
                let _ = fs::remove_dir(self.0.join(suffix));
            }
        }
    }
    #[test]
    fn install_update_uninstall_preserve_unrelated_files() {
        let root = Fixture::new();
        install_with_check(&root.0, || Ok(())).unwrap();
        verify(&target(&root.0)).unwrap();
        fs::write(target(&root.0).join("user-notes.txt"), b"keep").unwrap();
        fs::write(target(&root.0).join("main.js"), b"older version").unwrap();
        install_with_check(&root.0, || Ok(())).unwrap();
        verify(&target(&root.0)).unwrap();
        assert_eq!(
            fs::read(target(&root.0).join("user-notes.txt")).unwrap(),
            b"keep"
        );
        uninstall_with_check(&root.0, || Ok(())).unwrap();
        assert_eq!(
            fs::read(target(&root.0).join("user-notes.txt")).unwrap(),
            b"keep"
        );
        assert!(!target(&root.0).join("manifest.json").exists());
    }
    #[test]
    fn running_check_or_other_plugin_never_changes_payload() {
        let root = Fixture::new();
        assert!(install_with_check(&root.0, || Err("Photoshop running".into())).is_err());
        assert!(!target(&root.0).exists());
        install_with_check(&root.0, || Ok(())).unwrap();
        fs::write(
            target(&root.0).join("manifest.json"),
            br#"{"id":"other-plugin"}"#,
        )
        .unwrap();
        let old = fs::read(target(&root.0).join("manifest.json")).unwrap();
        assert!(install_with_check(&root.0, || Ok(())).is_err());
        assert!(uninstall_with_check(&root.0, || Ok(())).is_err());
        assert_eq!(
            fs::read(target(&root.0).join("manifest.json")).unwrap(),
            old
        );
    }
    #[test]
    fn photoshop_opening_during_preparation_preserves_old_install() {
        let root = Fixture::new();
        install_with_check(&root.0, || Ok(())).unwrap();
        fs::write(target(&root.0).join("main.js"), b"old data").unwrap();
        let checks = std::cell::Cell::new(0);
        let result = install_with_check(&root.0, || {
            checks.set(checks.get() + 1);
            if checks.get() > 1 {
                Err("now running".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read(target(&root.0).join("main.js")).unwrap(),
            b"old data"
        );
    }
}
