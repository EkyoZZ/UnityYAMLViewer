# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

Inspect Unity assets, edit properties and preview materials without opening Unity.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

**Unity 相关**

- **资源查看**：层级与 Inspector 视图，支持 `.prefab`、`.unity`、`.asset` 及资源引用定位。手型开关仅禁止预览选中，层级树和组件仍可选中。
- **目录浏览**：多根目录树、独立忽略规则与自动恢复，复用预览页签并保留修改，不影响最近打开记录。
- **Markdown 与富文本**：Markdown 默认排版预览，可切换源码；支持标题、列表、表格、代码块及常见 HTML 标题／段落／链接／图片标签，支持这些块的 align="center"／text-align:center 居中及带文字 SVG 徽章、图片链接；文字链接为蓝色下划线，一二级标题有分割线，普通代码块使用独立底色与蓝色文字，PowerShell 代码按命令、参数、字符串与注释着色并支持复制；不支持完整 HTML／CSS。图片支持文档相对路径和 HTTP(S) 后台下载缓存（每张最多 16 MiB、每文档最多 32 个资源），可通过清理缓存删除；同网址缓存不自动更新。RTF 在 Windows 上通过 RichEdit 直接只读渲染，支持缩放和复制全文，不需要 LibreOffice 或 PDFium；复杂排版、嵌入对象与原文档可能不同。
- **表格坐标**：CSV／TSV／PSV 行号从 1 开始，行号列及右侧分割线固定，不随横向滚动移动。
- **组件下载安装**：名称旁显示绿色已安装／红色未安装状态，支持同排重新检测，未安装状态后显示绿色描边安装按钮。内置下载器按需获取 FFmpeg Shared、LibreOffice 和 PDFium，显示字节进度与校验／安装状态；不依赖 Scoop、WinGet 或 Git。FFmpeg 不随软件附带，下载后完整解压并保留原目录结构；自动检测优先使用 PATH 中的工具，手动指定路径优先于自动检测，安装器不自动修改 PATH；PATH 缺少工具且已有托管安装时，可点击按钮将 bin 加入用户 PATH，新启动的终端或程序可使用；LibreOffice 调用官方 MSI，可能需要系统权限。已有程序可自动检测或手动指定，安装错误详情可选择复制请求与已解析的下载地址。
- **文本阅读**：文本、代码与 Shader 预览可调整并记住字号、行间距与段落间距，保留高亮与复制。
- **属性编辑**：编辑已识别的属性，支持校验、保存和撤销。
- **材质与 Shader**：节点、2D / 3D 及部分 Shader 单步预览。
- **场景与动画**：场景、模型、动画、Spine 和部分粒子效果预览；Spine 支持动态播放与预乘 Alpha 切换。
- **多图浏览**：同目录缩略图、方向键切图与位置计数，切图保留旧画面并预加载相邻图片，可设置默认浏览模式与适应窗口。
- **图片信息与 PNG 清理**：显示源位深、像素尺寸、文件大小及DPI（两轴相同时合并显示，不同时分别显示；缺少已解析密度时可补充标注 Windows 读取值，可能为系统默认值）；PNG 仅在确认所有像素完全不透明时可原位去除 Alpha、清理大于 0 字节的 iTXt，支持会话内撤销／重做，不修改 `.meta`。
- **Unity 图片工具**：清晰缩放、比例快捷菜单、通道与背景预览、九宫格精简和 Sprite Border 同步；序列拆分与合成、去底色及 GIF／视频／PNG 序列导出。

**附加工具**

- **四方连续图**：提取最小循环、多图案错位排列、实时编辑与 PNG 导出。
- **音视频工具**：播放、剪切、裁剪、缩放与转码，支持多音轨、字幕、媒体详情及 GIF／视频／PNG 序列导出。
- **文本表格**：CSV / TSV / PSV 查看与单元格编辑，支持查找、类型与必填检查、固定行列编号及冻结首行。
- **电子表格**：XLS / XLSX / ODS 只读预览，支持多工作表切换、查找复制、公式／缓存结果与冻结表头。
- **文件预览**：PDF、Word／PowerPoint、OpenDocument、RTF、SVG、GIF、ICO、受限 PSD 及常见文本；办公文档可在 LibreOffice 编辑，转换预览的文档支持另存为 PDF。
- **文件关联**：启动时自动更新 Windows 关联名称与独立文件图标登记，由用户选择默认程序；内容缩略图仍由系统处理。
- **日常操作**：多文件页签、最近记录、文件定位、深浅主题、七种语言、在线更新与内置第三方组件下载管理。

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
- 不支持视频合并、自由多轨剪辑、PDF 或 Office 内置编辑；暂不支持加密文档、宏格式及苹果办公格式。
- 电子表格不重算公式或还原样式、合并与图表；冻结设置仅能读取 XLSX，无法读取时可冻结首行。Word／演示文档按转换后的 PDF 预览，字体与排版可能不同，不支持文字选择或搜索。
- Word／演示文档预览需要 LibreOffice 和 PDFium，电子表格内置读取；音视频需要相应第三方组件，可指定组件路径，无需修改 PATH。安装状态和下载进度取决于安装器，安装中需等待。连续图不自动修补接缝；九宫格原位替换会覆盖文件，建议先预览或使用 `--dry-run`。

### 🛠️ 源码构建

安装稳定版 [Rust](https://www.rust-lang.org/tools/install)，在项目目录运行：

```powershell
cargo run --release
```

Windows 产物：`target/release/unity_yaml_viewer.exe`。提交修改前可运行 `cargo check`。

## English

### ✨ Supported features

**Unity features**

- **Markdown and rich text**: Markdown defaults to rendered preview with a source toggle, headings, lists, tables, code blocks and common HTML heading/paragraph/link/image tags; Centered blocks using align="center" or text-align:center, SVG badge text and image links are supported. Text links are blue and underlined; level 1/2 headings have dividers, and plain code blocks use a distinct background and blue text; PowerShell blocks distinguish commands, parameters, strings and comments while supporting copying. Full HTML/CSS is unsupported. Images use document-relative paths or background HTTP(S) downloads cached locally (16 MiB each, 32 resources per document). Clear Cache removes downloads; cached URLs do not refresh automatically. On Windows, RTF renders directly through RichEdit with zoom and copy-all, without LibreOffice or PDFium. Complex layouts and embedded objects may differ from the original.
- **Table coordinates**: CSV/TSV/PSV row numbers start at 1; the row-number column and its right divider stay fixed during horizontal scrolling.
- **Component downloads**: Green installed/red missing badges appear beside component names with a Recheck button and a green outlined install button immediately after missing status. The built-in downloader fetches FFmpeg Shared, LibreOffice and PDFium on demand with byte progress and verification/installation status, without Scoop, WinGet or Git. FFmpeg is not bundled; installation extracts the entire package with its directory structure. Automatic detection checks PATH first, explicit manual paths override it, and installation does not modify PATH automatically. When PATH lacks tools and a managed installation exists, a button adds its bin directory to user PATH for newly started terminals and applications. LibreOffice uses its official MSI and may require permission. Existing tools can be detected or selected manually; installation errors include selectable request and resolved download URLs.
- **Text reading**: Text, code and Shader previews remember font size, line and paragraph spacing, with highlighting and copying.
- **Asset browsing**: Hierarchy and Inspector views for `.prefab`, `.unity` and `.asset` files, with reference navigation. The hand toggle disables preview selection only; Hierarchy nodes and components remain selectable.
- **Folder browsing**: Multiple roots, per-folder ignore rules and automatic restore, with a reusable preview tab that preserves edits without changing recent files.
- **Property editing**: Edit recognized properties with validation, saving and undo.
- **Materials and shaders**: Node views, 2D / 3D previews and step previews for supported shaders.
- **Scenes and animation**: Preview scenes, models, animation, Spine and supported particle effects; Spine includes animation playback and a premultiplied-alpha toggle.
- **Image browsing**: Folder thumbnails, arrow-key navigation and position counters, with smooth image switching, adjacent-image preloading and configurable browsing / fit-to-window defaults.
- **Image information and PNG cleanup**: Show source depth, dimensions, file size and DPI (one value when both axes match; separate values otherwise, with a marked Windows codec fallback that may be a system default). PNG supports in-place Alpha removal only when every pixel is confirmed fully opaque and iTXt cleanup whenever its size exceeds 0 bytes, with session undo/redo; `.meta` files remain untouched.
- **Unity image tools**: Sharp scaling, zoom presets, channel and background views, nine-slice compaction and Sprite Border updates; split and combine sequences, remove backgrounds and export GIF, video or PNG frames.

**Additional tools**

- **Seamless repeats**: Extract repeat tiles, arrange multiple motifs with offsets, edit live and export PNG.
- **Audio and video**: Playback, trimming, cropping, scaling and transcoding, with multiple audio tracks, subtitles, media details and GIF, video or PNG sequence export.
- **Text tables**: View and edit CSV / TSV / PSV cells with search, type / required-cell checks, fixed coordinates and first-row freezing.
- **Spreadsheets**: Read-only XLS / XLSX / ODS previews with worksheet tabs, search / copy, formulas / cached results and header freezing.
- **File previews**: PDF, Word / PowerPoint, OpenDocument, RTF, SVG, GIF, ICO, limited PSD and common text files; Office documents can open in LibreOffice, and converted previews can be saved as PDF.
- **File associations**: Startup automatically updates the Windows app name and separate file icon registration; users choose the default app. Content thumbnails remain managed by Windows.
- **Everyday tools**: File tabs, recent files, file locations, light / dark themes, seven languages, updates and built-in third-party component downloads.

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
- Video merging, free multitrack editing, built-in PDF / Office editing, encrypted documents, macro-enabled formats and Apple office formats are unsupported.
- Spreadsheets do not recalculate formulas or reproduce styles, merges or charts. Original freeze settings can only be read from XLSX, otherwise first-row freezing is available. Word / presentation previews use converted PDFs; fonts and layout may differ, and text selection / search is unavailable.
- Word / presentation previews require LibreOffice and PDFium; spreadsheets use a built-in reader. Media needs the relevant third-party components; custom paths work without changing PATH. Installation status and download progress depend on the installer; wait during installation. Seamless repeats do not repair seams automatically. Nine-slice replacement overwrites files; preview first or use `--dry-run`.

### 🛠️ Build from source

Install stable [Rust](https://www.rust-lang.org/tools/install), then run:

```powershell
cargo run --release
```

Windows output: `target/release/unity_yaml_viewer.exe`. Run `cargo check` before submitting changes.

## License

[MIT](LICENSE)
