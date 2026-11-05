use super::*;
use std::collections::HashMap;

pub(super) fn start(ctx:&egui::Context,project:PathBuf,target:PathBuf)->mpsc::Receiver<Result<PathBuf,String>> {
    let (tx,rx)=mpsc::channel();
    use crate::background_tasks::{Task,Progress,Outcome,Target};
    let task=Task::start(ctx,"打包工程",target.file_name().unwrap_or_default().to_string_lossy().into_owned(),||Progress{stage:tr("正在打包工程…").into(),fraction:None,cancellable:false},None);
    std::thread::spawn(move|| {
        let result=pack(&project,&target).map(|()|target);
        task.finish(match &result {Ok(path)=>Outcome::Complete(Some(Target::File(path.clone()))),Err(error)=>Outcome::Failed(error.clone())});
        let _=tx.send(result);
    });rx
}

fn pack(project:&Path,target:&Path)->Result<(),String> {
    let mut archive=zip::ZipArchive::new(fs::File::open(project).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let mut document:Document={
        let entry=archive.by_name("project.json").map_err(|e|e.to_string())?;
        if entry.size()>8*1024*1024{return Err(tr("工程设置无效或格式版本不受支持。").into());}
        serde_json::from_reader(entry).map_err(|e|e.to_string())?
    };
    if document.format!="UnityYAMLViewer.MediaProject" || document.version!=1 || !matches!(document.kind.as_str(),"audio"|"video") {
        return Err(tr("工程设置无效或格式版本不受支持。").into());
    }
    let root=std::env::temp_dir().join(format!("unity-project-bundle-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
    fs::create_dir(&root).map_err(|e|e.to_string())?;let assets=Assets(root);
    let mut next=0;let mut external_files=HashMap::<PathBuf,String>::new();let mut embedded_files=HashMap::<String,PathBuf>::new();
    let mut missing=Vec::new();
    for item in &mut document.items {
        for reference in std::iter::once(&mut item.source).chain(item.tracks.iter_mut()).chain(item.clip.iter_mut()) {
            match reference.clone() {
                Reference::External{..}=> {
                    match resolve(reference,project,&mut archive,&assets,&mut next) {
                        Ok(path)=> {
                            let canonical=fs::canonicalize(&path).map_err(|e|e.to_string())?;
                            let target_absolute=target.parent().and_then(|p|fs::canonicalize(p).ok()).map(|p|p.join(target.file_name().unwrap_or_default()));
                            if target_absolute.as_ref()==Some(&canonical){return Err(tr("工程不能覆盖素材文件。").into());}
                            let count=external_files.len();
                            let name=external_files.entry(canonical).or_insert_with(||format!("media/{count:04}_{}",path.file_name().unwrap_or_default().to_string_lossy())).clone();
                            *reference=Reference::External{absolute:PathBuf::from(&name),relative:Some(PathBuf::from(name))};
                        }
                        Err(error)=>missing.push(error),
                    }
                }
                Reference::Embedded{entry}=> {
                    if !embedded_files.contains_key(&entry) {
                        let path=resolve(reference,project,&mut archive,&assets,&mut next)?;
                        embedded_files.insert(entry,path);
                    }
                }
            }
        }
        // Track paths are restored from their matching references on opening.
        for (track,reference) in item.state.tracks.iter_mut().zip(&item.tracks) {
            track.path=match reference {Reference::External{relative,..}=>relative.clone().unwrap_or_default(),Reference::Embedded{entry}=>PathBuf::from(entry)};
        }
    }
    if !missing.is_empty(){return Err(missing.join("\n"));}
    let project_name=project.file_name().ok_or_else(||tr("工程打包失败。").to_owned())?;
    let staged=assets.0.join(project_name);
    write_archive(&staged,&document,&embedded_files.into_iter().collect::<Vec<_>>())?;
    let temporary=target.with_file_name(format!(".unity-bundle-{}-{}.tmp",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
    let result=(|| {
        let output=fs::OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|e|e.to_string())?;
        let mut zip=zip::ZipWriter::new(output);
        let options=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
        let entries=std::iter::once((staged,project_name.to_string_lossy().into_owned())).chain(external_files.into_iter());
        for (path,name) in entries {
            zip.start_file(name,options).map_err(|e|e.to_string())?;
            std::io::copy(&mut fs::File::open(path).map_err(|e|e.to_string())?,&mut zip).map_err(|e|e.to_string())?;
        }
        zip.finish().map_err(|e|e.to_string())?.sync_all().map_err(|e|e.to_string())?;
        crate::playlist::replace(&temporary,target)
    })();
    if result.is_err(){let _=fs::remove_file(&temporary);}result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_bundle_relinks_shared_media_after_moving() {
        let root=std::env::temp_dir().join(format!("unity-bundle-test-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();let _cleanup=Assets(root.clone());
        let source=root.join("source.wav");fs::write(&source,b"source fixture").unwrap();
        let clip=root.join("edited.wav");fs::write(&clip,b"edited fixture").unwrap();
        let project=root.join("example.uyap");
        let item=Item{enabled:true,source:external(&source,&root),state:EditState::fixture(),tracks:vec![],clip:Some(Reference::Embedded{entry:"assets/edit.wav".into()}),position:0.0,selected_audio:None};
        let document=Document{format:"UnityYAMLViewer.MediaProject".into(),version:1,kind:"audio".into(),active:0,expanded:true,merge:false,merge_settings:MergeSettings::default(),items:vec![item]};
        write_archive(&project,&document,&[("assets/edit.wav".into(),clip)]).unwrap();
        let target=root.join("share.zip");pack(&project,&target).unwrap();
        let moved=root.join("recipient");fs::create_dir(&moved).unwrap();
        zip::ZipArchive::new(fs::File::open(target).unwrap()).unwrap().extract(&moved).unwrap();
        fs::remove_file(source).unwrap();
        let mut archive=zip::ZipArchive::new(fs::File::open(moved.join("example.uyap")).unwrap()).unwrap();
        let restored:Document=serde_json::from_reader(archive.by_name("project.json").unwrap()).unwrap();
        let scratch=Assets(root.join("scratch"));fs::create_dir(&scratch.0).unwrap();
        let mut next=0;
        let media=resolve(&restored.items[0].source,&moved.join("example.uyap"),&mut archive,&scratch,&mut next).unwrap();
        assert_eq!(fs::read(media).unwrap(),b"source fixture");
        let edit=resolve(restored.items[0].clip.as_ref().unwrap(),&moved.join("example.uyap"),&mut archive,&scratch,&mut next).unwrap();
        assert_eq!(fs::read(edit).unwrap(),b"edited fixture");
        assert!(pack(&project,&root.join("missing.zip")).is_err());
        assert!(!root.join("missing.zip").exists());
    }
}
