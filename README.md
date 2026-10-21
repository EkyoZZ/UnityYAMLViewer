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
- **Unity 图片工具**：像素清晰缩放、统一背景设置、R/G/B 单通道可选应用 Alpha、九宫格无损精简和 Sprite Border 同步，支持命令行批处理；序列工具支持按行列拆分图片及多图拖入、同组自动加入与自然排序合并为 PNG，支持优先按 Multiple meta 实际切片拆分、独立忽略拆分图上／下／左／右补边、拆分帧居中透明补齐至 4 的倍数、可选跳过去底后完全透明空帧（播放与 PNG／GIF／视频一致，连续编号，缩短动画时长）、自定义拆分命名与非法字符检查、按透明间隔识别拆分行列、可缩放平移的分格与单帧播放预览（帧率／循环可调）、矩形优先／无空格自动排列、共同最小尺寸裁剪、单图区域居中及可选图片边框、水平／垂直间隔、可选 Unity tpsheet 同步导出（需 TexturePacker Importer）、整图宽高 4 对齐；制作与拆分均支持吸取纯色背景、容差与遮罩预览、吸色光标与候选色值（单击选色、Esc 取消，色块可打开调色板）、边缘柔化和去底色污染，可能移除同色主体；拆分帧和制作列表可按预览帧率转入视频编辑（缓存 PNG 首帧即显、逐帧定位与播放，透明区域使用棋盘背景），导出 GIF／视频／PNG 序列，普通视频编码使用可选颜色的底色，GIF／PNG 可保留透明（GIF 不保留半透明）。

- **序列操作历史**：制作／拆分支持图片拖入、自动加入、删除、排序及参数修改的撤销／重做（Ctrl+Z／Ctrl+Y）；不撤回已保存文件。

**附加工具**

- **四方连续图**：提取最小循环、多图案错位排列、实时编辑与 PNG 导出。
- **音视频工具**：播放、剪切、裁剪、转码、多音轨与字幕，支持媒体详情查看和字幕字体选择；视频预览按选区播放，终点显示保留片段最后一帧；音视频／GIF 导出在保存确认后支持覆盖同名文件，先编码成功再替换；GIF 导出沿用时长裁剪、画面裁剪与缩放，可选缩放滤波器、抖动（默认 Bicubic＋Sierra2 4A）、颜色压缩、1–100 FPS 和循环次数（0 为永远），不含音频与字幕；GIF 可沿用视频编辑转视频，GIF 与视频均可按选区、裁剪、缩放和帧率导出连续编号 PNG 到新文件夹（图片不记录播放时长）。
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
- **Unity image tools**: Pixel-sharp scaling, shared preview backgrounds, optional Alpha application for individual R/G/B channels, lossless nine-slice compaction and Sprite Border updates, with batch CLI support; sequence tools split images into grids and accept dropped or automatically matched numbered images and combine naturally sorted images into PNG sheets, with exact Multiple-meta slicing by default, independent top/bottom/left/right padding exclusion and transparent centering of split frames to dimensions divisible by 4, optional skipping of fully transparent frames after background removal (matching playback and PNG/GIF/video export, consecutive numbering and shorter duration), custom split naming and filename validation, grid detection from transparent gaps, zoomable, pannable grid and animated frame previews with adjustable frame rate and looping, compact-rectangle or exact-grid automatic layouts, common minimum cropping, centered image areas with optional image bounds, horizontal/vertical spacing, optional Unity tpsheet export (requires TexturePacker Importer) and dimensions divisible by 4; composition and splitting support flat-background picking, tolerance/mask previews, a picking cursor and candidate color values (click to pick, Esc to cancel, swatch opens the palette), edge softness and color decontamination (matching subject colors may be removed). Split frames and composition lists enter video editing at the preview frame rate (cached PNG first-frame display, frame seeking and playback, with checkerboard transparency) for GIF/video/PNG sequence export. Current video codecs use a chosen background color; GIF/PNG may retain transparency (GIF has no partial alpha).

- **Sequence history**: Undo/redo image drops, automatic additions, removals, ordering and settings in composition/splitting (Ctrl+Z/Ctrl+Y); previously saved files remain.

**Additional tools**

- **Seamless repeats**: Extract repeat tiles, arrange multiple motifs with offsets, edit live and export PNG.
- **Audio and video**: Playback, trimming, cropping, transcoding, multiple tracks and subtitles, with media details and subtitle font selection; video previews play the selected range and hold its last retained frame at the end; audio/video and GIF exports can replace existing files after save confirmation, encoding successfully before replacement; GIF export reuses time trimming, cropping and scaling, with scaling filters and dithering (default Bicubic + Sierra2 4A), optional color reduction, 1–100 FPS and loop count (0 = forever), without audio or subtitles; GIF can be converted to video in the same editor, and both GIF and video export numbered PNG frames to a new folder using the selected time range, crop, scale and frame rate (images contain no playback timing).
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
