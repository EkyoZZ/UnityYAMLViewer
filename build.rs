#[cfg(target_os = "windows")]
fn main() {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use std::{env, fs::File, path::PathBuf};

    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("unity-yaml-viewer.ico");
    let mut icon = IconDir::new(ResourceType::Icon);
    for (size, source) in [
        (16, "assets/app_icon_16.png"),
        (20, "assets/app_icon_20.png"),
        (24, "assets/app_icon_24.png"),
        (32, "assets/app_icon_32.png"),
        (40, "assets/app_icon_40.png"),
        (48, "assets/app_icon_48.png"),
        (64, "assets/app_icon_64.png"),
        (72, "assets/app_icon_72.png"),
        (80, "assets/app_icon_80.png"),
        (96, "assets/app_icon_96.png"),
        (128, "assets/app_icon_128.png"),
        (192, "assets/app_icon_192.png"),
        (256, "assets/app_icon_256.png"),
    ] {
        println!("cargo:rerun-if-changed={source}");
        let image = image::open(source).unwrap_or_else(|_| panic!("无法读取应用图标：{source}"));
        assert_eq!(
            (image.width(), image.height()),
            (size, size),
            "应用图标尺寸必须为 {size}×{size}：{source}"
        );
        let entry = IconDirEntry::encode(&IconImage::from_rgba_data(
            size,
            size,
            image.to_rgba8().into_raw(),
        ))
        .expect("无法生成图标尺寸");
        icon.add_entry(entry);
    }
    icon.write(File::create(&output).expect("无法创建 ico 文件"))
        .expect("无法写入 ico 文件");
    // Resource 1 stays the application icon; resource 2 is only for associated files.
    let file_output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("associated-file.ico");
    let mut file_icon = IconDir::new(ResourceType::Icon);
    for size in [16, 20, 24, 32, 40, 48, 64, 72, 80, 96, 128, 192, 256] {
        let source = format!("assets/default_icons/file_icon_{size}.png");
        println!("cargo:rerun-if-changed={source}");
        let image = image::open(&source).expect("无法读取关联文件图标");
        assert_eq!((image.width(), image.height()), (size, size), "关联文件图标尺寸错误：{source}");
        file_icon.add_entry(IconDirEntry::encode(&IconImage::from_rgba_data(
            size, size, image.to_rgba8().into_raw(),
        )).expect("无法生成关联文件图标"));
    }
    file_icon.write(File::create(&file_output).expect("无法创建关联文件 ico"))
        .expect("无法写入关联文件 ico");
    winres::WindowsResource::new()
        .set("FileDescription", "Unity YAML Viewer")
        .set("ProductName", "Unity YAML Viewer")
        .set_icon(output.to_str().unwrap())
        .set_icon_with_id(file_output.to_str().unwrap(), "2")
        .compile()
        .expect("无法嵌入 Windows 图标");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
