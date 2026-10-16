# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

A desktop viewer for Unity assets, with property editing and material previews.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

- **资源查看**：打开 Prefab、Scene、Material 等文本资源，显示 Hierarchy 与 Inspector，支持搜索、引用定位和多标签页。
- **弹窗定位**：首次打开或通过程序按钮唤醒时对齐主窗口；鼠标点击、切换窗口等普通焦点恢复保留用户摆放的位置。
- **图片编辑**：在独立系统窗口中逐像素比较完整 RGBA，参数在左上，右上显示尺寸、面积收益与可展开详情，下方同屏对照带删除标记的裁剪方案和结果，不另设原图展示；四区左右保持 1:1 等宽，窗口缩放不改变列宽比例，超宽文字换行；预览随窗口增高，像素尺寸居中显示在图片下方的圆角底衬中，紧接边界距离标记，说明与操作位于底部；裁剪图用半透明底色和稀疏斜纹标出移除区，默认从候选色自动选对比色，可切换半透明红色；重复模式白框标出保留循环，虚线标最小循环边界；“同时 / 横向 / 纵向”和“拉伸 / 重复”使用主题切换样式，与自动识别放在同一行；左上设置不设内部滚动条，顶部高度跟随设置内容，右上信息超高时独立滚动；原始 Border 叠加开关放在左上设置中；自动识别比较六种组合，按实际对齐后面积选择，逐方向解释不能精简的原因；实际保留尺寸、诊断、循环和方案比较默认收起，详情区分可精简区、保留与移除像素；点击状态旁“查看详情”展开，重新打开窗口恢复收起。拖动参数时保留上次预览，更新完成前禁用保存；没有可读 `.meta` 时，底部按钮整体居中；有元信息时，底部信息在左、放大的绿色原位替换和蓝色另存为按钮在右垂直居中排列，右侧预留滚动条空间；独立精简窗口内容超出可视区域时，滚动条常显且占用独立空间，不覆盖内容。重复模式识别连续区域的最小循环，可设置横向 / 纵向保留倍数。边界四项成组，按上 / 下、左 / 右分两行排列，“边界:”标签相对两行输入垂直居中，px 紧邻输入框右侧，与下一组方向文字留出 12 px 间距；中心宽高默认 4 px、允许手动调整；默认保留透明边缘、将宽高对齐到 4 的倍数；标注实际保留中心、循环倍数、节省面积和补白；结果尺寸与面积变化用底色突出，节省为绿色、增加为橙色、无变化为灰色。裁剪图可叠加已读取 `.meta` 的原始 Unity Border（绿色），与本次切分线（黄 / 蓝）分别标明来源。确认后原位替换、不创建备份，并重载页签；可另存 PNG。仅在成功读取同位置单 Sprite `.meta` 时显示同步操作，写入 Border / Full Rect 并保留 GUID。
- **属性编辑**：编辑已识别的属性、颜色、向量和列表，支持校验、撤销与保存。
- **材质预览**：Material / Shader 使用 **Node-based UI**，支持节点内临时调参、拖图换纹理、2D / 3D 输出与 Cull / Blend 调节；常见公式拆成100 点宽运算节点，颜色预览按需展开，可见时约 60 帧/秒更新，支持左键选择 / 框选（不受输出节点可见性影响）、统一选框的节点与组多选拖动、右键平移、带框折叠 / 展开和眼睛预览按钮（含 R/G/B/A 单通道及其运算）的可拖动函数分组及具名边界接口与 10%～175% 缩放。
- **场景与动画**：预览受支持的 2D / UI、Spine 和模型资源，查看 Animator 状态图、编辑动画曲线。
- **音视频播放**：支持播放列表、波形、全屏和播放进度记忆；解码依赖外部 FFmpeg，可在设置中安装 / 检测。
- **日常使用**：最近文件、标签恢复、深浅主题、七种语言和在线更新；GitHub 接口限流时提示等待重试，也可打开发布页手动更新。

### 🚀 快速开始

1. [下载 Windows x64 程序](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)，直接运行 EXE。
2. 将文件拖入窗口，或通过“文件”菜单打开；Unity 资源尽量保留在原项目目录中，以便解析引用。
3. 选择对象查看属性；需要修改时开启“编辑模式”。材质节点中的调整仅用于预览，不写回文件。

详细操作、支持格式和预览范围见[使用指南](docs/user-guide.md#中文)。

### ⚠️ 待支持与当前限制

- 拉伸只删除完整 RGBA 一致的连续行列；重复只提取通过完整行列像素验证的连续周期，不做近似采样。取消透明边缘保留时，仅沿所选方向裁去 Alpha 为 0 的外圈（会丢弃其中隐藏 RGB、改变留白）；全透明图默认不裁。4 倍数对齐优先多保留重复像素 / 完整循环，不足时每轴补 1–3 个透明像素：1 加右 / 下，2 两侧各 1，3 左 / 上 1、右 / 下 2；补白不计入 Unity Border，可能增加尺寸；实际倍数可能高于输入值或受区域大小限制。原位支持 PNG / TGA / BMP / WebP；JPEG 和动画需另存静态 PNG。输出为 8 位 RGBA，不保留附加图像元数据。多 Sprite 或其他纹理类型不支持原位替换；未读到 `.meta` 时不显示修改提示或创建它。Unity 需重新导入，UI Image 需手动选择 Sliced / Tiled；不承诺具体网格数量。输入上限为单边 8192 像素、256 MiB 解码分配；节省面积不等于压缩文件或显存节省。
- Unity 二进制序列化资源暂不支持；未知类型、原始 YAML 和 fileID 引用保持只读。
- 场景、Spine 和 Shader 为部分兼容，不能完整替代 Unity 渲染；URP / HDRP、多 Pass、复杂 Shader 与完整光照仍有限制。
- 节点界面目前用于预览，不支持创建节点、编辑连线或生成 Shader；函数快照限可唯一定位到阶段调用处的数值参数函数，复杂调用只展示拆分；不支持的表达式保留为源码语句。

### 🛠️ 源码构建

安装稳定版 [Rust](https://www.rust-lang.org/tools/install)，在项目目录运行：

```powershell
cargo run --release
```

Windows 产物：`target/release/unity_yaml_viewer.exe`。技术栈：Rust · egui / eframe。
欢迎提交 Issue 或 PR；提交前运行 `cargo check`。

## English

### ✨ Supported features

- **Asset browsing**: Open text assets such as Prefabs, Scenes and Materials with a Hierarchy, Inspector, search, reference navigation and tabs.
- **Dialog positioning**: Dialogs center on the main window when first opened or explicitly summoned by a button. Clicking or switching focus preserves their current position.
- **Image editing**: Native RGBA editor with settings at upper left, dimensions, savings and expandable details at upper right, and crop / result views below. The marked crop view replaces the separate original display. Both rows retain equal column widths as the window resizes; long text wraps within its column. Preview heights grow with the window, pixel dimensions sit centered on rounded backgrounds just below the image border labels, and notes and actions remain at the bottom. A translucent fill with sparse diagonal hatching marks removed pixels; it automatically picks a contrasting candidate color, with a red fallback option. Repeat outlines retained cycles and marks minimum-cycle boundaries. Both / Horizontal / Vertical and Stretch / Repeat use theme-style tabs alongside Auto Detect. Settings have no internal scrollbar; their content determines the top height, and information scrolls independently when taller. The original Border overlay toggle sits with the settings. Auto Detect compares all six combinations by actual aligned area and explains limitations for each axis. Retained dimensions, diagnostics, cycles and option comparisons are collapsed by default; Details distinguish compactable, retained and removed pixels. View Details beside the status expands them, and reopening resets the collapsed state. While dragging, the previous preview stays visible and saving waits for the current result. Without readable metadata, footer buttons are centered. With metadata, footer information sits on the left, with larger green Replace and blue Save As buttons vertically centered on the right and clearance for the scrollbar. When the native editor overflows, its scrollbar stays visible and reserves space instead of overlaying content. Repeat detects the smallest contiguous cycle and supports separate cycle multiples per axis. Border inputs form one group with its label vertically centered beside both rows, and use two rows (Top / Bottom in the left column, Left / Right in the right), with px directly beside each field and 12 px before the next direction group. Retained center width / height default to 4 px and remain editable. Transparent edge preservation and dimensions in multiples of 4 are enabled by default. Highlights output dimensions and area changes in green for savings, orange for growth and gray for no change. Shows actual retained dimensions, cycles, saved area and padding, and optionally overlays original Unity Border values in green on the crop view, separate from proposed split lines in yellow / blue. Confirm in-place replacement without backups and reload the tab, or save a new PNG. Metadata controls appear only after reading a companion single-Sprite `.meta`; Border / Full Rect updates preserve GUID.
- **Property editing**: Edit recognized properties, colors, vectors and lists, with validation, undo and saving.
- **Material previews**: A **node-based UI** for Materials / Shaders, with temporary inputs, texture drops, 2D / 3D output and Cull / Blend controls. Common formulas use 100-point-wide operation nodes with expandable color previews refreshed at up to about 60 fps while visible, left-click selection and marquee selection even with the output offscreen, unified selection and dragging of nodes and groups, right-drag panning, draggable function groups with framed fold / expand and eye preview buttons (including R/G/B/A channels and their operations) and named boundary ports and 10%–175% zoom.
- **Scenes and animation**: Preview supported 2D / UI, Spine and model assets, inspect Animator graphs and edit animation curves.
- **Media playback**: Playlists, waveforms, fullscreen and remembered playback positions. Decoding requires external FFmpeg, installable / detectable in Settings.
- **Everyday tools**: Recent files, restored tabs, light / dark themes, seven languages and online updates; when GitHub rate limits checks, wait before retrying or open the release page to update manually.

### 🚀 Quick start

1. [Download the Windows x64 executable](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest) and run it directly.
2. Drop a file into the window or open it from the File menu. Keep Unity assets inside their original project where possible so references can be resolved.
3. Select an object to inspect it; enable Edit Mode to change properties. Material-node adjustments affect the preview only and do not write files.

See the [user guide](docs/user-guide.md#english) for detailed controls, formats and preview support.

### ⚠️ Pending support and limitations

- Stretch removes only consecutive identical full RGBA rows or columns; Repeat extracts exact contiguous periods without approximation. Disabling edge preservation trims Alpha-zero margins on selected axes, discarding hidden RGB and changing whitespace; fully transparent images are preserved by default. Alignment retains extra pixels or whole cycles first, then pads each axis by up to 3 transparent pixels if needed: 1 on right/bottom; 2 split evenly; 3 as 1 on left/top and 2 on right/bottom. Padding is excluded from Unity Border; dimensions may grow and actual cycle multiples can differ from input. In-place replacement supports PNG / TGA / BMP / WebP; JPEG and animation require exporting a static PNG. Output is 8-bit RGBA without auxiliary image metadata. Multiple-Sprite sheets and other texture types cannot be replaced in place. No metadata hints or creation without a readable companion. Unity must reimport; select Sliced / Tiled on UI Image manually. Actual mesh count is not guaranteed. Input decoding is limited to 8192 pixels per side and 256 MiB; saved area does not equal compressed file or GPU memory savings.
- Unity binary serialization is unsupported. Unknown types, raw YAML and fileID references remain read-only.
- Scene, Spine and Shader compatibility is partial and does not reproduce Unity rendering completely. URP / HDRP, multiple passes, complex shaders and full lighting remain limited.
- The node interface is for previews; creating nodes, editing connections and generating shaders are unsupported. Helper snapshots require a unique direct stage call with numeric parameters; complex calls are display-only. Unsupported expressions remain source statements.

### 🛠️ Build from source

Install stable [Rust](https://www.rust-lang.org/tools/install), then run from the project directory:

```powershell
cargo run --release
```

Windows output: `target/release/unity_yaml_viewer.exe`. Built with Rust and egui / eframe.
Issues and PRs are welcome; run `cargo check` before submitting.

## License

[MIT](LICENSE)

图标生成：安装 Pillow 后运行 `python assets/generate_icons.py`，优先使用 1024 源图，否则使用 512，两者缺失时报错；两次双线性缩放生成 16～256 PNG，覆盖小尺寸图标。

Icon generation: install Pillow and run `python assets/generate_icons.py`. It prefers the 1024 source, falls back to 512, and errors if neither exists. Two bilinear passes regenerate and replace the 16–256 PNG icons.

## 命令行图片精简 / Command-line image compaction

主窗口顶部“格式支持”右侧的“指令查看”打开独立用法窗口，可复制英文指令说明；文件菜单不重复提供入口。也可使用 `--help` 或 `slice9 --help`。默认命令行无窗口执行，不转发给已打开的 GUI；--preview 则打开或转发到 GUI。

The **Commands** button beside **Format Support** opens a separate reference window with copyable English syntax and examples. It is not duplicated in the File menu. `--help` and `slice9 --help` show the same reference. CLI jobs run headlessly, independently of the GUI instance, except --preview which opens or forwards to the GUI.

```powershell
# 单张 / Single image
.\unity_yaml_viewer.exe slice9 "button.png"
# 多张 / Multiple images
.\unity_yaml_viewer.exe slice9 "a.png" "b.png"
# 当前目录层 / Directory, immediate files
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI"
# 子目录 / Recursive
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r
# 只计算，不修改 / Plan only
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r --dry-run
# 自定义中心、报告 / Custom center and report
.\unity_yaml_viewer.exe slice9 "button.png" --center 8x4 --report "result.json"
```

默认会**原位覆盖，不创建备份**：精确 RGBA 自动比较方向和 Stretch / Repeat，中心 4×4，循环倍数 1×1，保留透明边缘并对齐 4 倍数。仅输出面积减少时替换；无收益保持图片和 meta 不变。有可读受支持的单 Sprite `.meta` 时同步 Border / Full Rect 并保留 GUID；缺失 / 不可读时仅处理图片且不创建 meta，可读但不支持则该图片失败。补白按两侧分配、奇数多加右 / 下，不计入 Border。单张失败继续批次，路径去重，跳过符号链接和不支持文件。仅支持 PNG / TGA / BMP / 静态 WebP，动画报错。

Defaults **overwrite in place without backups**: exact RGBA automatic axis and Stretch / Repeat selection, retained center 4×4, cycle multiples 1×1, transparent-edge preservation and alignment to four. Only smaller output areas are written; no savings leaves image and meta unchanged. Readable supported single-Sprite metadata gets Border / Full Rect updates preserving GUID. Missing/unreadable metadata is not created; readable unsupported metadata fails that image. Balanced padding, with odd extras right/bottom, is excluded from Border. Processing continues after individual failures, deduplicates paths and skips symbolic links/unsupported files. Supported formats: PNG / TGA / BMP / static WebP; animations fail.

| 参数 / Option | 用途 / Behavior |
|---|---|
| `-r`, `--recursive` | 包含子目录 / Include subdirectories |
| `--dry-run` | 不写图片或 meta / No image or meta writes |
| `--center WxH` | 拉伸中心尺寸 / Stretch retained center |
| `--repeat-count XxY` | 重复保留倍数 / Repeat retained cycles |
| `--direction auto\|both\|horizontal\|vertical` | 指定方向 / Axis constraint |
| `--mode auto\|stretch\|repeat` | 指定处理方式 / Treatment constraint |
| `--no-preserve-edges` | 允许裁透明外圈 / Allow transparent-margin trimming |
| `--no-align4` | 取消 4 倍数对齐 / Disable alignment |
| `--report FILE` | 另写 JSON 报告，仅新文件 / Also write JSON; refuses existing files |
| `--` | 后续参数均按路径 / Remaining arguments are paths |

JSON 标准输出包含逐文件结果、原因、尺寸、节省面积、补白、Border、meta 写入状态与分类计数；退出码 0 无失败，1 文件失败，2 参数或报告错误。报告不会覆盖已有文件，写入失败可能发生在部分资源已处理之后，批次不提供整体回滚。Windows GUI EXE 在 PowerShell 中需要可靠等待 / 获取退出码时使用：

JSON stdout includes per-file results, reasons, sizes, saved area, padding, Border, metadata status and summary counts. Exit codes: 0 no failures, 1 file failures, 2 argument/report errors. Reports never overwrite existing files; report writes can fail after some assets were processed. Batches are not atomic. To reliably wait for the Windows GUI executable and retrieve its exit code in PowerShell:

```powershell
$p = Start-Process -FilePath ".\unity_yaml_viewer.exe" -ArgumentList 'slice9 "D:\Project\Assets\UI" -r --dry-run' -Wait -PassThru -NoNewWindow -RedirectStandardOutput "slice9-result.json" -RedirectStandardError "slice9-errors.txt"
$p.ExitCode
```
### 打开文件与路径引号 / Opening files and quoting paths

```powershell
# 单个 / Single
.\unity_yaml_viewer.exe a.png
# 批量 / Multiple
.\unity_yaml_viewer.exe a.png b.prefab c.mat
# 有空格：每个路径分别加引号 / Quote each path with spaces
.\unity_yaml_viewer.exe "D:\UI Images\a.png" "D:\My Assets\b.prefab"
```

路径不含空格或命令行特殊字符时，可不加引号；包含空格或 shell 特殊字符时应加引号。示例统一使用双引号方便复制，但转义规则由调用的 shell 决定；PowerShell 路径包含字面量 `$` 或反引号时，优先使用单引号，避免双引号内插值。

Quotes are optional for simple paths without spaces or shell-special characters. Quote each path containing such characters separately. Examples use double quotes for copying, but escaping is shell-specific; in PowerShell, prefer single quotes for literal paths containing `$` or backticks to avoid interpolation.

### 自动裁剪预览 / Automatic compaction preview

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r --preview
```

`--preview` 打开 GUI 页签和独立九宫格窗口，并执行自动识别，**不自动修改图片或 meta**。用户可修改所有原有设置、重新识别、另存为、确认原位替换或直接关闭；原本对不支持格式 / meta / 尚未完成计算的限制仍保留。

每次请求按路径去重，最多取前 10 张（规范化路径排序）；当前进程的预览总数最多 10 个，包括手动打开的窗口。已打开的预览直接唤醒，不重置用户设置；超出的图片不排队、不自动修改，提示未打开数量。关闭窗口后可再次调用打开更多。已有主窗口时转发到该窗口，主界面用多个页签，预览之间相互独立，切换页签不停止其他预览。

预览支持 `-r`、`--center`、`--repeat-count`、透明边缘和对齐选项，但启动时统一自动选择方向与处理方式；不能与 `--dry-run`、`--report` 或显式 `--direction` / `--mode` 一起使用，打开后仍可在窗口手动切换方向与处理方式。十张大图可能占用较多内存。

`--preview` opens GUI tabs and native nine-slice windows with automatic detection, **without automatically changing images or metadata**. All editor controls remain available: adjust settings, detect again, save as, confirm replacement or close without changes. Existing format/metadata and calculation-state restrictions still apply.

Paths are deduplicated and sorted; each request selects at most the first 10. A process can have at most 10 preview windows, including manually opened ones. Existing previews are focused without resetting edits. Excess images are reported and neither queued nor modified; close windows and invoke again to open more. Requests forward to an existing main window. Files use tabs in the main window; their independent previews keep updating across tab switches.

Preview supports recursion, retained center/cycles, edge and alignment options, but initially chooses axes and treatment automatically. It cannot combine with `--dry-run`, `--report` or explicit `--direction` / `--mode`; those controls remain editable inside the window. Ten large images can consume substantial memory.
