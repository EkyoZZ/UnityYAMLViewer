fn main() {
    println!("cargo:rerun-if-changed=../../../assets");
    #[cfg(windows)]
    {
        use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
        let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("installer.ico");
        let mut icon = IconDir::new(ResourceType::Icon);
        for size in [16, 24, 32, 48, 64, 128, 256] {
            let path = root.join(format!("../../../assets/app_icon_{size}.png"));
            let image = image::open(&path)
                .expect("Cannot read installer icon")
                .to_rgba8();
            icon.add_entry(
                IconDirEntry::encode(&IconImage::from_rgba_data(size, size, image.into_raw()))
                    .expect("Cannot encode icon"),
            );
        }
        icon.write(std::fs::File::create(&output).unwrap()).unwrap();
        winres::WindowsResource::new()
            .set("FileDescription","PS Tile Plugin Installer")
            .set("ProductName","PS Tile Plugin Installer")
            .set_icon(output.to_str().unwrap())
            .set_manifest(r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo></assembly>"#)
            .compile().expect("Cannot embed installer resources");
    }
}
