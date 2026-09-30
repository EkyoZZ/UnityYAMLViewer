#[cfg(target_os = "windows")]
fn main() {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use std::{env, fs::File, path::PathBuf};

    let source = PathBuf::from("assets/app_icon.png");
    println!("cargo:rerun-if-changed={}", source.display());
    let image = image::open(&source).expect("无法读取应用图标");
    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("unity-yaml-viewer.ico");
    let mut icon = IconDir::new(ResourceType::Icon);
    for size in [16, 32, 48, 256] {
        let resized = image
            .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        let entry =
            IconDirEntry::encode(&IconImage::from_rgba_data(size, size, resized.into_raw()))
                .expect("无法生成图标尺寸");
        icon.add_entry(entry);
    }
    icon.write(File::create(&output).expect("无法创建 ico 文件"))
        .expect("无法写入 ico 文件");
    winres::WindowsResource::new()
        .set_icon(output.to_str().unwrap())
        .compile()
        .expect("无法嵌入 Windows 图标");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
