# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**  
Inspect Unity assets, edit properties and preview materials without opening Unity.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### 已支持

**Unity 相关**

- **资源查看与编辑**：浏览 Prefab、Scene、Asset 的层级和引用，编辑已识别属性，支持校验、保存与撤销。
- **材质与场景预览**：查看材质及 Shader 节点，预览场景、模型、动画、Spine 和部分粒子效果。
- **目录与图片浏览**：多根目录、同目录缩略图和快捷切图，自动恢复目录并保留页签修改。
- **图片工具**：支持平滑或像素预览、通道与图片信息查看，支持九宫格、Sprite Border、序列拆分合成、去底色、多格式导出及 PNG 批量优化。

**附加工具**

- **文档阅读与编辑**：文本和代码高亮；Markdown／RTF 原生直读，支持常用 HTML 与富文本。Windows RTF 支持独立编辑、图片和表格。
- **文件与表格预览**：预览 PDF、办公文档及常见图片；编辑 CSV／TSV／PSV，或只读查看 XLS／XLSX／ODS，支持查找复制与固定行列编号。
- **音视频工具**：多文件编辑、转码与队列／合并导出，支持播放列表、多音轨与字幕、透明视频 GPU 播放及编辑预览、序列输出、画面比例和可续编／打包的工程。字幕支持生成与语言匹配、中英日韩离线／实时翻译及缓存回放、统一或原生 ASS 显示、ASS／SSA／SRT／WebVTT 编辑与字幕转换，以及内嵌字幕原格式提取、MKV 样式保留和画面烧录。
- **录屏**：Windows 支持全屏、多屏及窗口吸附／拖拽选区录制，提供悬浮工具条、独立录屏历史与可保存的快捷键，按预设质量自动保存 MKV／MP4 到下载目录；可选录制系统声音，结束后可进入视频编辑。
- **网络视频**：频道列表支持分组、台标和多源切换；网页 HLS 或视频提取支持格式选择、下载与缓存，Windows 提供浏览器助手。
- **四方连续图**：提取最小循环，多图案错位排列，实时预览并导出 PNG。
- **错误诊断**：共用日志记录录屏、后台任务失败和运行时错误，便于排查问题。
- **日常与组件管理**：多页签、最近记录、文件定位与关联、深浅主题、七语言和在线更新；按需安装组件，识别运行库可选 CPU／GPU 或同时保留，支持检测与手动路径。

### 快速开始

1. 下载 Windows x64 程序并运行，拖入文件或目录。
2. Unity 资源尽量保留在原工程目录中，便于解析引用。
3. 选择对象查看属性，需要修改时启用编辑模式。

[使用指南](docs/user-guide.md#中文) · [详细功能与限制](docs/feature-reference.md#中文) · [命令行参考](docs/command-line.md)

### 待支持与当前限制

- Unity 二进制序列化、未知属性编辑、节点连线编辑和 Shader 生成尚不支持；场景等预览不能完整替代 Unity 渲染。
- PDF／Office 内置编辑、自由多轨剪辑、加密文档、宏格式和苹果办公格式尚不支持。办公文档可在 LibreOffice 中编辑，表格预览不重算公式或还原完整样式。
- Word／演示文档预览需要 LibreOffice 和 PDFium；媒体处理、网页提取分别需要 FFmpeg、yt-dlp。组播频道需要线路提供对应信号；浏览器助手不传 Cookie，部分站点或受保护视频无法播放。
- 目录和图片浏览、ICO／PSD 兼容范围有限；图片去水印未开放，连续图不自动修补接缝。
- 原位覆盖会修改源文件，撤销不是永久备份；九宫格替换前请预览或使用 `--dry-run`。

## English

### Supported features

**Unity features**

- **Asset browsing and editing**: Inspect Prefab, Scene and Asset hierarchies and references; edit recognized properties with validation, saving and undo.
- **Materials and scenes**: View material and Shader nodes; preview scenes, models, animation, Spine and supported particle effects.
- **Folders and images**: Browse multiple roots and same-folder thumbnails, switch images quickly, and restore folders while retaining tab edits.
- **Image tools**: Choose smooth or pixelated previews and inspect channels and image information; use nine-slice, Sprite Border, sequence splitting/composition, background removal, format export and batch PNG optimization.

**Additional tools**

- **Documents**: Highlight text and code; read Markdown/RTF directly with common HTML and rich-text support. Edit RTF with images and tables on Windows.
- **Files and tables**: Preview PDFs, Office documents and common images; edit CSV/TSV/PSV or read XLS/XLSX/ODS, with search, copy and fixed row/column coordinates.
- **Audio and video**: Edit and transcode multiple files, export queues or merged media, and use playlists, multiple tracks, subtitles, GPU playback/editor previews including transparency, image sequences, display ratios and resumable or shareable projects. Subtitles support generation and language matching, offline/live translation among Chinese, English, Japanese and Korean, cached replay, unified or native ASS display, ASS/SSA/SRT/WebVTT editing, subtitle conversion, extraction in the embedded track's original format, style retention in MKV and burn-in.
- **Screen recording**: On Windows, record screens or a snapped/custom-drawn region to MKV/MP4 in Downloads using quality presets, floating controls, recording history and saved shortcuts, optionally capturing system audio and opening the video editor afterward.
- **Online video**: Channel lists support groups, logos and multiple sources; webpage HLS and video extraction support format selection, downloads and caching, with a browser assistant on Windows.
- **Seamless repeats**: Extract repeat tiles, arrange motifs with offsets, preview live and export PNG.
- **Diagnostics**: Shared logs record screen recording, failed background tasks and runtime errors to help troubleshoot problems.
- **Everyday tools and components**: Tabs, recent files, file locations and associations, light/dark themes, seven languages and updates; install components on demand, keep CPU/GPU recognition runtimes separately or together, and use detection or manual paths.

### Quick start

1. Download and run the Windows x64 app, then drop in files or folders.
2. Keep Unity assets inside their original project to resolve references.
3. Select an object to inspect properties; enable Edit Mode to make changes.

[User guide](docs/user-guide.md#english) · [Detailed features and limitations](docs/feature-reference.md#english) · [CLI reference](docs/command-line.md)

### Pending support and limitations

- Unity binary serialization, editing unknown properties, node connections and Shader generation are unsupported. Previews do not fully reproduce Unity rendering.
- Built-in PDF/Office editing, free multitrack editing, encrypted documents, macro formats and Apple office formats are unsupported. Office files can open in LibreOffice; spreadsheet previews do not recalculate formulas or reproduce full formatting.
- Word/presentation previews require LibreOffice and PDFium; media processing and webpage extraction require FFmpeg and yt-dlp respectively. Multicast channels require a network carrying those streams. The browser assistant does not transfer cookies; some sites or protected videos may not play.
- Folder/image browsing and ICO/PSD compatibility have limits. Image watermark removal is unavailable; repeat tools do not repair seams automatically.
- In-place replacement changes source files; undo is not a permanent backup. Preview nine-slice replacements first or use `--dry-run`.

## 源码构建 / Build from source

安装稳定版 [Rust](https://www.rust-lang.org/tools/install) 后运行 / Install stable Rust, then run:

```powershell
cargo run --release
```

Windows 产物 / Output: `target/release/unity_yaml_viewer.exe`。编译检查 / Compile check: `cargo check`。

## License

[MIT](LICENSE)
