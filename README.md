# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

Inspect Unity assets, edit properties and preview materials without opening Unity.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

**Unity 相关**

- **资源查看**：层级与 Inspector 视图，支持 `.prefab`、`.unity`、`.asset` 及资源引用定位。
- **属性编辑**：编辑已识别的属性，支持校验、保存和撤销。
- **材质与 Shader**：节点、2D / 3D 及部分 Shader 单步预览。
- **场景与动画**：场景、模型、动画、Spine 和部分粒子效果预览。
- **缩放快捷菜单**：点击图片、SVG、PDF、模型、Spine 及静态预览的悬浮比例，向上展开常用比例（10%～300%），菜单与信息条使用相同半透明背景。
- **目录图片浏览**：单图查看／多图浏览 Tab 切换，设置中可选择默认模式（默认开启多图浏览），底部固定缩略图栏（与预览分区、透明缩略图棋盘背景、当前项线框、悬停显示完整路径、主图信息显示当前序号／总数）、前后按钮和四个方向键（左／上切上一张，右／下切下一张）；仅扫描当前目录的 PNG／JPEG／TGA／BMP／WebP，不递归子目录，不包含 GIF／SVG。
- **Unity 图片工具**：清晰缩放与通道预览、统一背景、九宫格精简和 Sprite Border 同步；支持序列拆分与合成、去底色、跳过空帧及 GIF／视频／PNG 序列导出。

**附加工具**

- **四方连续图**：提取最小循环、多图案错位排列、实时编辑与 PNG 导出。
- **音视频工具**：播放、剪切、裁剪、缩放与转码，支持多音轨、字幕、媒体详情及 GIF／视频／PNG 序列导出。
- **文本表格**：CSV / TSV / PSV 查看、单元格编辑、类型检查与查找。
- **文件预览**：PDF 只读预览，以及 SVG、GIF 和常见文本文件查看。
- **日常操作**：多文件页签、最近记录、文件定位、深浅主题、七种语言、在线更新与第三方组件管理。

### 🚀 快速开始

1. 下载 Windows x64 程序，直接运行。
2. 拖入文件或从文件菜单打开；Unity 资源尽量保留在原工程目录中，便于解析引用。
3. 选择对象查看属性，启用编辑模式后修改并保存。

[使用指南](docs/user-guide.md#中文) · [详细功能与限制](docs/feature-reference.md#中文) · [命令行参考](docs/command-line.md)

### ⚠️ 待支持与当前限制

- 图片去水印入口未开放；序列工具的去底色仍可使用。

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
- **Property editing**: Edit recognized properties with validation, saving and undo.
- **Materials and shaders**: Node views, 2D / 3D previews and step previews for supported shaders.
- **Scenes and animation**: Preview scenes, models, animation, Spine and supported particle effects.
- **Zoom presets**: Click floating zoom indicators in image, SVG, PDF, model, Spine and static previews to open an upward preset menu (10%–300%) with the same translucent background as the indicator.
- **Folder image browsing**: Single-image / browse tabs with a saved default mode (folder browsing enabled by default), fixed bottom thumbnails separate from the preview, checkerboards behind transparent thumbnails, an outline on the current image, full paths on hover, current position / total in the preview badge, previous / next buttons and arrow keys (Left / Up for previous, Right / Down for next); scans PNG / JPEG / TGA / BMP / WebP in the current folder only, without subfolders or GIF / SVG.
- **Unity image tools**: Sharp scaling, channel previews, shared backgrounds, nine-slice compaction and Sprite Border updates; split and combine image sequences, remove flat backgrounds, skip empty frames and export GIF, video or PNG sequences.

**Additional tools**

- **Seamless repeats**: Extract repeat tiles, arrange multiple motifs with offsets, edit live and export PNG.
- **Audio and video**: Playback, trimming, cropping, scaling and transcoding, with multiple audio tracks, subtitles, media details and GIF, video or PNG sequence export.
- **Text tables**: View and edit CSV / TSV / PSV cells, check types and search.
- **File previews**: Read-only PDFs, SVGs, GIFs and common text files.
- **Everyday tools**: File tabs, recent files, file locations, light / dark themes, seven languages, updates and third-party component management.

### 🚀 Quick start

1. Download and run the Windows x64 executable.
2. Drop a file into the window or open it from the File menu. Keep Unity assets inside their original project so references can be resolved.
3. Select an object to inspect its properties; enable Edit Mode to make and save changes.

[User guide](docs/user-guide.md#english) · [Detailed features and limitations](docs/feature-reference.md#english) · [CLI reference](docs/command-line.md)

### ⚠️ Pending support and limitations

- The image watermark-removal entry is unavailable; background removal in sequence tools remains available.

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
