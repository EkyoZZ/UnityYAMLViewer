# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

Inspect Unity assets, edit properties and preview materials without opening Unity.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

**Unity 相关**

- **资源查看**：层级与 Inspector 视图，支持 `.prefab`、`.unity`、`.asset` 及资源引用定位。
- **目录浏览**：多根目录树、独立忽略规则与自动恢复，复用预览页签并保留修改，不影响最近打开记录。
- **属性编辑**：编辑已识别的属性，支持校验、保存和撤销。
- **材质与 Shader**：节点、2D / 3D 及部分 Shader 单步预览。
- **场景与动画**：场景、模型、动画、Spine 和部分粒子效果预览；Spine 支持动态播放与预乘 Alpha 切换。
- **多图浏览**：同目录缩略图、方向键切图与位置计数，可设置默认浏览模式和适应窗口显示。
- **Unity 图片工具**：清晰缩放、比例快捷菜单、通道与背景预览、九宫格精简和 Sprite Border 同步；序列拆分与合成、去底色及 GIF／视频／PNG 序列导出。

**附加工具**

- **四方连续图**：提取最小循环、多图案错位排列、实时编辑与 PNG 导出。
- **音视频工具**：播放、剪切、裁剪、缩放与转码，支持多音轨、字幕、媒体详情及 GIF／视频／PNG 序列导出。
- **文本表格**：CSV / TSV / PSV 查看、单元格编辑、类型检查与查找。
- **文件预览**：PDF、SVG、GIF、ICO、受限 PSD 合成图及常见文本文件查看。
- **日常操作**：多文件页签、最近记录、文件定位、深浅主题、七种语言、在线更新与第三方组件管理。

### 🚀 快速开始

1. 下载 Windows x64 程序，直接运行。
2. 拖入文件或目录，或从文件菜单打开；Unity 资源尽量保留在原工程目录中，便于解析引用。
3. 选择对象查看属性，启用编辑模式后修改并保存。

[使用指南](docs/user-guide.md#中文) · [详细功能与限制](docs/feature-reference.md#中文) · [命令行参考](docs/command-line.md)

### ⚠️ 待支持与当前限制

- 图片去水印入口未开放；序列工具的去底色仍可使用。

- 目录浏览不跟随符号链接或目录联接；多图浏览不递归子目录，不包含 GIF／SVG。
- ICO／PSD 只读且不提供图片编辑工具；ICO 不切换内部尺寸，PSD 仅支持 8 位 RGB／灰度、Raw／RLE，最大 8192 × 8192，不支持图层编辑或 ICC 色彩管理。
- 不支持 Unity 二进制序列化；未知属性和原始 YAML 保持只读。
- 场景、Spine、粒子和 Shader 兼容性有限，不能完整替代 Unity 渲染；不支持节点连线编辑或 Shader 生成。
- 不支持视频合并、自由多轨剪辑、PDF 编辑或 Office 文档预览 / 编辑。
- 音视频与 PDF 功能需要相应第三方组件，可在第三方支持中分别指定 FFmpeg、FFprobe 程序及 PDFium DLL 路径，无需修改 PATH；连续图不自动修补接缝。九宫格原位替换会覆盖文件，建议先预览或使用 `--dry-run`。

### 🛠️ 源码构建

安装稳定版 [Rust](https://www.rust-lang.org/tools/install)，在项目目录运行：

```powershell
cargo run --release
```

Windows 产物：`target/release/unity_yaml_viewer.exe`。提交修改前可运行 `cargo check`。

## English

### ✨ Supported features

**Unity features**

- **Asset browsing**: Hierarchy and Inspector views for `.prefab`, `.unity` and `.asset` files, with reference navigation.
- **Folder browsing**: Multiple roots, per-folder ignore rules and automatic restore, with a reusable preview tab that preserves edits without changing recent files.
- **Property editing**: Edit recognized properties with validation, saving and undo.
- **Materials and shaders**: Node views, 2D / 3D previews and step previews for supported shaders.
- **Scenes and animation**: Preview scenes, models, animation, Spine and supported particle effects; Spine includes animation playback and a premultiplied-alpha toggle.
- **Image browsing**: Folder thumbnails, arrow-key navigation and position counters, with configurable default browsing and fit-to-window modes.
- **Unity image tools**: Sharp scaling, zoom presets, channel and background views, nine-slice compaction and Sprite Border updates; split and combine sequences, remove backgrounds and export GIF, video or PNG frames.

**Additional tools**

- **Seamless repeats**: Extract repeat tiles, arrange multiple motifs with offsets, edit live and export PNG.
- **Audio and video**: Playback, trimming, cropping, scaling and transcoding, with multiple audio tracks, subtitles, media details and GIF, video or PNG sequence export.
- **Text tables**: View and edit CSV / TSV / PSV cells, check types and search.
- **File previews**: PDFs, SVGs, GIFs, ICOs, limited PSD composites and common text files.
- **Everyday tools**: File tabs, recent files, file locations, light / dark themes, seven languages, updates and third-party component management.

### 🚀 Quick start

1. Download and run the Windows x64 executable.
2. Drop a file or folder into the window, or open it from the File menu. Keep Unity assets inside their original project so references can be resolved.
3. Select an object to inspect its properties; enable Edit Mode to make and save changes.

[User guide](docs/user-guide.md#english) · [Detailed features and limitations](docs/feature-reference.md#english) · [CLI reference](docs/command-line.md)

### ⚠️ Pending support and limitations

- The image watermark-removal entry is unavailable; background removal in sequence tools remains available.

- Folder browsing skips symbolic links and junctions; image browsing is non-recursive and excludes GIF / SVG.
- ICO / PSD are read-only without image editing tools. ICO has no size selector; PSD supports only 8-bit RGB / grayscale, Raw / RLE, up to 8192 × 8192, without layer editing or ICC color management.
- Unity binary serialization is unsupported. Unknown properties and raw YAML remain read-only.
- Scene, Spine, particle and shader compatibility is partial and does not fully reproduce Unity rendering. Node connection editing and shader generation are unsupported.
- Video merging, free multitrack editing, PDF editing and Office document viewing / editing are unsupported.
- Media and PDF features need the relevant third-party components; FFmpeg, FFprobe and PDFium DLL paths can be set individually in third-party support without changing PATH. Seamless repeats do not repair seams automatically. Nine-slice replacement overwrites files; preview first or use `--dry-run`.

### 🛠️ Build from source

Install stable [Rust](https://www.rust-lang.org/tools/install), then run:

```powershell
cargo run --release
```

Windows output: `target/release/unity_yaml_viewer.exe`. Run `cargo check` before submitting changes.

## License

[MIT](LICENSE)
