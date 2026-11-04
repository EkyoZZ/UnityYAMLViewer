//! Application-owned direct downloads. No package manager or repository bootstrap.
use std::{fs,io::{Read,Write},path::{Path,PathBuf},time::{Duration,Instant}};
use sha2::{Digest,Sha256};
use super::install_progress::Shared;
use crate::localization::tr;
const LIMIT:u64=1024*1024*1024;
const FFMPEG_URL: &str = "https://github.com/EkyoZZ/UnityYAMLViewer/releases/download/ffmpeg-20250731/ffmpeg-N-120424-g03b9180fe3-20250731-win64-gpl-shared.zip";
const FFMPEG_SHA256: &str = "471be94adaf65b1eeba4f9b5bef41a27e2c5c1c05eacb51f84fc860f1119782a";
const FFMPEG_SIZE: u64 = 70_227_964;
const OFFICE_ROOT:&str="https://download.documentfoundation.org/libreoffice/stable/";
fn error(e:impl std::fmt::Display)->String{e.to_string()}
fn cancelled(progress:&Shared)->Result<(),String>{if progress.cancel_requested(){Err(tr("下载已取消。").into())}else{Ok(())}}
fn text(client:&reqwest::blocking::Client,url:&str,progress:&Shared)->Result<String,String>{
    progress.log(format!("{}: {url}",tr("请求地址")));
    let response=client.get(url).send().and_then(|r|r.error_for_status()).map_err(error)?;
    let mut bytes=Vec::new();response.take(4*1024*1024+1).read_to_end(&mut bytes).map_err(error)?;
    if bytes.len()>4*1024*1024{return Err(tr("组件发行信息过大").into());}
    String::from_utf8(bytes).map_err(error)
}
fn hash_value(value:&str)->Result<String,String>{
    let hash=value.split_whitespace().next().unwrap_or("").strip_prefix("sha256:").unwrap_or_else(||value.split_whitespace().next().unwrap_or(""));
    if hash.len()!=64||!hash.bytes().all(|b|b.is_ascii_hexdigit()){return Err(tr("组件缺少 SHA-256 校验值").into());}Ok(hash.to_ascii_lowercase())
}
async fn wait<F:std::future::Future>(future:F,progress:&Shared)->Result<F::Output,String>{
    let mut future=Box::pin(future);
    loop{cancelled(progress)?;if let Ok(result)=tokio::time::timeout(Duration::from_millis(100),future.as_mut()).await{return Ok(result);}}
}
fn download(_client:&reqwest::blocking::Client,url:&str,path:&Path,expected:&str,known:u64,progress:&Shared)->Result<(),String>{
    cancelled(progress)?;progress.phase(2);progress.log(format!("{}: {url}",tr("下载地址")));
    let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(error)?;
    runtime.block_on(async{
        let client=reqwest::Client::builder().user_agent("UnityYAMLViewer/third-party").https_only(true)
            .connect_timeout(Duration::from_secs(20)).read_timeout(Duration::from_secs(30)).timeout(Duration::from_secs(1800)).build().map_err(error)?;
        let mut response=wait(client.get(url).send(),progress).await?.and_then(|r|r.error_for_status()).map_err(error)?;
        let total=response.content_length().unwrap_or(known);
        if total>LIMIT{return Err(tr("组件下载大小无效").into());}
        let mut file=fs::File::create(path).map_err(error)?;let mut hash=Sha256::new();let mut done=0;progress.bytes(0,total);
        while let Some(bytes)=wait(response.chunk(),progress).await?.map_err(error)?{
            done+=bytes.len() as u64;
            if done>LIMIT||(total>0&&done>total){return Err(tr("组件下载大小无效").into());}
            file.write_all(&bytes).map_err(error)?;hash.update(&bytes);progress.bytes(done,total);
        }
        file.sync_all().map_err(error)?;cancelled(progress)?;progress.phase(3);
        if done==0||(total>0&&done!=total)||format!("{:x}",hash.finalize())!=expected{return Err(tr("组件包校验失败，未执行安装。").into());}
        progress.log(tr("组件包 SHA-256 校验通过。"));Ok(())
    })
}

struct Workspace{path:PathBuf,keep:bool}
impl Workspace{
    fn new(name:&str)->Result<Self,String>{let root=super::component_root()?;fs::create_dir_all(&root).map_err(error)?;
        let path=root.join(format!("{name}-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        fs::create_dir(&path).map_err(error)?;Ok(Self{path,keep:false})}
}
impl Drop for Workspace{fn drop(&mut self){if !self.keep{
    if let (Ok(root),Ok(path))=(super::component_root().and_then(|p|fs::canonicalize(p).map_err(error)),fs::canonicalize(&self.path)){
        if path.parent()==Some(root.as_path()){let _=fs::remove_dir_all(path);}
    }
}}}
#[derive(serde::Serialize,serde::Deserialize)]
struct FfmpegRecord{folder:String,#[serde(default="default_bin")] bin:String}
fn default_bin()->String{"bin".into()}
fn relative_directory(value:&str)->bool{!value.is_empty()&&Path::new(value).components().all(|part|matches!(part,std::path::Component::Normal(_)))}
pub(crate) fn tool_directories(_name:&str)->Vec<PathBuf>{
    let Ok(root)=super::component_root()else{return Vec::new()};
    let record=fs::read(root.join("ffmpeg.json")).ok().and_then(|b|serde_json::from_slice::<FfmpegRecord>(&b).ok());
    record.filter(|r|r.folder.starts_with("ffmpeg-")&&r.folder.len()<100&&r.folder.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')&&relative_directory(&r.bin)).map(|r|vec![root.join(r.folder).join(r.bin)]).unwrap_or_default()
}
fn publish_ffmpeg(folder:&Path,bin:&Path)->Result<(),String>{
    for tool in ["ffmpeg","ffprobe"]{crate::component_paths::validate_tool(&folder.join(bin).join(format!("{tool}.exe")),tool)?;}
    let root=super::component_root()?;let record=FfmpegRecord{folder:folder.file_name().ok_or("Invalid component path")?.to_string_lossy().into_owned(),bin:bin.to_string_lossy().into_owned()};
    let temp=root.join("ffmpeg.json.tmp");fs::write(&temp,serde_json::to_vec(&record).map_err(error)?).map_err(error)?;
    fs::rename(temp,root.join("ffmpeg.json")).map_err(error)
}
fn ffmpeg(client: &reqwest::blocking::Client, progress: &Shared) -> Result<(), String> {
    progress.log(format!("{}: N-120424-g03b9180fe3-20250731", tr("发行版本")));
    let mut workspace=Workspace::new("ffmpeg")?;let package=workspace.path.join("package.zip");
    download(client, FFMPEG_URL, &package, FFMPEG_SHA256, FFMPEG_SIZE, progress)?;
    progress.phase(4);
    let mut archive=zip::ZipArchive::new(fs::File::open(&package).map_err(error)?).map_err(error)?;let mut total=0u64;let mut bin_directory=None;
    for index in 0..archive.len(){let mut entry=archive.by_index(index).map_err(error)?;
        let enclosed=entry.enclosed_name().ok_or_else(||tr("组件压缩包路径无效").to_owned())?;
        if entry.unix_mode().is_some_and(|mode|mode&0o170000==0o120000){return Err(tr("组件压缩包路径无效").into());}
        let destination=workspace.path.join(&enclosed);
        if entry.is_dir(){fs::create_dir_all(&destination).map_err(error)?;continue;}
        if enclosed.file_name().is_some_and(|name|name=="ffmpeg.exe"){
            let bin=enclosed.parent().filter(|p|relative_directory(&p.to_string_lossy())).ok_or("Invalid FFmpeg bin directory")?.to_owned();
            if bin_directory.replace(bin).is_some(){return Err("Multiple FFmpeg binaries in archive".into());}
        }
        total=total.checked_add(entry.size()).ok_or("Archive size overflow")?;
        if total>512*1024*1024||entry.size()>256*1024*1024{return Err(tr("组件解压大小超限").into());}
        fs::create_dir_all(destination.parent().unwrap()).map_err(error)?;
        let mut output=fs::OpenOptions::new().write(true).create_new(true).open(destination).map_err(error)?;
        let expected=entry.size();let copied=std::io::copy(&mut entry.by_ref().take(expected+1),&mut output).map_err(error)?;
        if copied!=expected{return Err(tr("组件解压大小超限").into());}
    }
    drop(archive);fs::remove_file(package).map_err(error)?;progress.phase(5);publish_ffmpeg(&workspace.path,&bin_directory.ok_or("Missing FFmpeg executable")?)?;workspace.keep=true;Ok(())
}
fn office(client:&reqwest::blocking::Client,progress:&Shared)->Result<(),String>{
    let listing=text(client,OFFICE_ROOT,progress)?;
    let re=regex::Regex::new(r#"href="(\d+\.\d+\.\d+)/""#).map_err(error)?;
    let version=re.captures_iter(&listing).filter_map(|c|semver::Version::parse(&c[1]).ok()).max().ok_or_else(||tr("未找到 LibreOffice 稳定版下载信息").to_owned())?;
    let url=format!("{OFFICE_ROOT}{version}/win/x86_64/LibreOffice_{version}_Win_x86-64.msi");
    let hash=hash_value(&text(client,&format!("{url}.sha256"),progress)?)?;cancelled(progress)?;
    let workspace=Workspace::new("libreoffice-install")?;let package=workspace.path.join("LibreOffice.msi");
    download(client,&url,&package,&hash,0,progress)?;
    super::install_task::ensure_no_other_installer(None)?;progress.phase(4);
    progress.log(tr("正在调用 Windows Installer，请留意系统权限提示。"));
    let mut child=crate::media_tools::command("msiexec.exe").arg("/i").arg(&package).args(["/passive","/norestart","/L*v"]).arg(workspace.path.join("install.log"))
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(error)?;
    let started=Instant::now();let code=loop{match child.try_wait(){Ok(Some(status))=>break status.code().unwrap_or(-1),Ok(None)=>{},Err(e)=>{progress.log(e.to_string());}}
        if started.elapsed()>Duration::from_secs(900){progress.log(tr("安装仍在进行，请检查系统提示；不会自动终止安装程序。"));}
        std::thread::sleep(Duration::from_millis(500));
    };
    if !matches!(code,0|1641|3010){let log=workspace.path.join("install.log");let location=log.display().to_string();let mut workspace=workspace;workspace.keep=true;
        return Err(format!("Windows Installer: {code}\n{}: {location}",tr("安装日志")));}
    if matches!(code,1641|3010){progress.reboot();}progress.phase(5);Ok(())
}
pub(crate) fn install(component:&str,progress:&Shared)->Result<(),String>{
    if !cfg!(all(windows,target_arch="x86_64")){return Err(tr("内置组件安装目前仅支持 Windows x64。").into());}
    let _guard=super::install_task::Guard::acquire()?;super::install_task::ensure_no_other_installer(None)?;
    progress.phase(1);let client=reqwest::blocking::Client::builder().user_agent("UnityYAMLViewer/third-party").https_only(true).connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(30)).build().map_err(error)?;
    match component{"ffmpeg"=>ffmpeg(&client,progress),"libreoffice"=>office(&client,progress),_=>Err(tr("不支持的第三方组件").into())}.map_err(|error|format!("{error}\n{}",progress.details()))
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn checksum_requires_sha256(){assert!(hash_value("bad").is_err());assert!(hash_value(&format!("{}  package.msi","a".repeat(64))).is_ok());}
}
