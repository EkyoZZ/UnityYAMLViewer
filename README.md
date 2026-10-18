# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

A desktop viewer for Unity assets, with property editing and material previews.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

- **第三方支持**：设置提供独立管理窗口，集中显示 FFmpeg / FFprobe 的安装状态和检测路径，继续通过 WinGet 安装。PDFium 提供 Windows x64 不带 V8 组件的按需下载、SHA-256 校验、版本 / 路径显示、重新检测和移除；来源为 bblanchon/pdfium-binaries，解压依赖 Windows tar。组件保存在 `%APPDATA%/UnityYAMLViewer-components`，位于软件缓存扫描范围之外，不会被缓存清理删除；FFmpeg 由 WinGet 管理。LibreOffice 同样提供 WinGet 安装、版本 / 路径检测和手动选择程序，可能需要管理员确认，安装文件由系统管理。打开窗口仅检测，点击下载或安装才联网安装。FFmpeg 在第三方窗口打开时检测；安装后的检测和“重新检测”完成后同步主窗口并重试失败的媒体预览，关闭窗口不触发额外检测。

- **PDF 预览**：通过第三方支持下载并校验 PDFium 后，支持多页只读预览、多页 PDF 在文件名后提供自动翻页开关（单页及页数未加载时隐藏相关设置）与 1–3600 秒间隔（默认 5 秒，全屏继续；秒数后可切换循环，默认关闭，开启时末页回到第一页，否则末页停止）；循环后提供过渡下拉菜单，默认“无”，可选淡入淡出、左右滑动、推页、擦除、缩放淡入，动画 0.25 秒且全屏同样生效。底部页码 / 缩放浮层与圆形矢量上一页 / 下一页按钮、方向键翻页、黑底全屏查看（重载后的“全屏”按钮或 F11 进入，F11 / Esc 退出）、适应窗口 / 宽度、固定倍率、滚轮缩放、拖动与重载。缺少组件可直接打开管理窗口，安装后校验检测及“重新检测”完成后同步结果并自动重试失败预览，也可手动重试；关闭管理窗口不触发刷新。100% 采用 96 DPI 逻辑像素基准并叠加系统 DPI，页面按物理像素对齐，使用屏幕文字抗锯齿。文件最大 128 MiB，按当前页与 DPI 后台渲染，单边最大 4096 像素；翻页时保留上一页直到新页完成，等待超过 1 秒才显示加载转圈。白色页面背景，不执行脚本或表单交互，加密文档暂不支持。

- **SVG 与 GIF 预览**：SVG 支持只读预览、棋盘格 / 灰色 / 自定义背景（沿用 Prefab 预览控件）、适应窗口、50% / 100% / 200%、滚轮缩放、拖动浏览与重载；缩放时后台重新渲染，单边最大 4096 像素，文件最大 16 MiB。支持渲染器可识别的静态路径 / 图形；文字需转轮廓，不加载外部资源或内嵌位图，不执行脚本或动画。GIF 隐藏音量、音频信息和音轨菜单，显示全局 / 局部调色板最大容量（非实际使用颜色数）。GIF 通过外部 FFmpeg / FFprobe 使用媒体控件播放、暂停、停止与定位，复用视频 UI，时间轴按实际时长分布并显示“0:00:00.000(f00)/0:00:00.000(f00) · 0.030 s/f”（f / s/f 固定不翻译，时长索引未就绪时显示 — s/f），悬停查看对应帧，空间允许时显示帧边界；支持按文件设置、播放一次、无限循环或指定总播放次数。按文件设置识别 NETSCAPE / ANIMEXTS 循环扩展，0 表示无限，未声明时播放一次；正值沿用 FFmpeg 的总轮次含义。暂停保留轮次，停止归零并重置轮次。SVG 隐藏通道切换与九宫格入口，背景设置仅用于预览、不写回文件，重载保留背景设置。SVG / GIF 不支持九宫格或编辑，GIF 暂不提供通道切换；SVG 编辑后续扩展。
- **文件页签**：相同后缀名（忽略大小写）超过 3 个文件时折叠为下拉页签，显示组内最近激活的文件名；菜单沿用“最近打开”的自适应宽度与最大宽度限制，文件名左对齐、单行截断，悬停显示完整路径；高亮当前文件，使用无边框按钮与矢量文件堆叠 / 关闭图标，文件堆叠图标后显示组内数量，每项带 ×，底部提供“关闭全部”。资源 / 播放列表存在未保存修改时提醒，取消会停止剩余批量关闭；逐项关闭减至 3 个及以下时，在原组位置按组内顺序展开为单成员页签；单成员页签显示 ×，不显示下拉菜单。“关闭全部”期间不因数量降至 3 个而解散，直到关闭结束；取消后恢复自动展开。普通页签与折叠组均支持拖拽排序，折叠组整体移动并保留组内顺序；独立音视频编辑窗口沿用原有生命周期。
- **文件定位**：页签、堆叠菜单内的文件项及图片 / PDF / 媒体预览文件名可右键打开文件所在目录（资源管理器选中文件），或复制完整文件地址；缺失文件禁用定位。网络缓存文件定位实际缓存位置。
- **资源查看**：打开 Prefab、Scene、Material 等文本资源，显示 Hierarchy 与 Inspector，支持搜索、引用定位和多标签页。
- **弹窗定位**：首次打开或通过程序按钮唤醒时对齐主窗口；鼠标点击、切换窗口等普通焦点恢复保留用户摆放的位置。
- **图片编辑**：在独立系统窗口中逐像素比较完整 RGBA，参数在左上，右上显示尺寸、面积收益与可展开详情，下方同屏对照带删除标记的裁剪方案和结果，不另设原图展示；四区左右保持 1:1 等宽，窗口缩放不改变列宽比例，超宽文字换行；预览随窗口增高，像素尺寸居中显示在图片下方的圆角底衬中，紧接边界距离标记，说明与操作位于底部；裁剪图用半透明底色和稀疏斜纹标出移除区，默认从候选色自动选对比色，可切换半透明红色；重复模式白框标出保留循环，虚线标最小循环边界；“同时 / 横向 / 纵向”和“拉伸 / 重复”使用主题切换样式，与自动识别放在同一行；左上设置不设内部滚动条，顶部高度跟随设置内容，右上信息超高时独立滚动；原始 Border 叠加开关放在左上设置中；自动识别比较六种组合，按实际对齐后面积选择，逐方向解释不能精简的原因；实际保留尺寸、诊断、循环和方案比较默认收起，详情区分可精简区、保留与移除像素；点击状态旁“查看详情”展开，重新打开窗口恢复收起。拖动参数时保留上次预览，更新完成前禁用保存；没有可读 `.meta` 时，底部按钮整体居中；有元信息时，底部信息在左、放大的绿色原位替换和蓝色另存为按钮在右垂直居中排列，右侧预留滚动条空间；独立精简窗口内容超出可视区域时，滚动条常显且占用独立空间，不覆盖内容。重复模式识别连续区域的最小循环，可设置横向 / 纵向保留倍数。边界四项成组，按上 / 下、左 / 右分两行排列，“边界:”标签相对两行输入垂直居中，px 紧邻输入框右侧，与下一组方向文字留出 12 px 间距；中心宽高默认 4 px、允许手动调整；默认保留透明边缘、将宽高对齐到 4 的倍数；标注实际保留中心、循环倍数、节省面积和补白；结果尺寸与面积变化用底色突出，节省为绿色、增加为橙色、无变化为灰色。裁剪图可叠加已读取 `.meta` 的原始 Unity Border（绿色），与本次切分线（黄 / 蓝）分别标明来源。确认后原位替换、不创建备份，并重载页签；可另存 PNG。仅在成功读取同位置单 Sprite `.meta` 时显示同步操作，写入 Border / Full Rect 并保留 GUID。
- **属性编辑**：编辑已识别的属性、颜色、向量和列表，支持校验、撤销与保存。
- **材质预览**：Material / Shader 使用 **Node-based UI**，支持节点内临时调参、拖图换纹理、2D / 3D 输出与 Cull / Blend 调节；常见公式拆成100 点宽运算节点，颜色预览按需展开，可见时约 60 帧/秒更新，支持左键选择 / 框选（不受输出节点可见性影响）、统一选框的节点与组多选拖动、右键平移、带框折叠 / 展开和眼睛预览按钮（含 R/G/B/A 单通道及其运算）的可拖动函数分组及具名边界接口与 10%～175% 缩放。
- **场景与动画**：预览受支持的 2D / UI、Spine 和模型资源，查看 Animator 状态图、编辑动画曲线。
- **音视频播放**：视频 / GIF 的整个预览框（含留白）均可左键单击播放 / 暂停，自由缩放拖动不触发播放切换，控制条与播放列表控件保留独立操作。支持播放列表、波形、全屏和进度记忆。顶部“剪切/格式转换”打开独立原生窗口并停止来源页签；弹窗独立播放和调节试听音量，切换或关闭来源页签后仍保留，支持多个文件，同一文件再次打开会唤醒已有窗口。视频目前支持起止范围另存 MP4（H.264 / AAC），仅主视频和当前音轨、不含字幕。播放与编辑依赖外部 FFmpeg，可从“设置 → 第三方支持”安装 / 检测。
- **音频剪切与转换**：头尾 / 区间使用主题切换式 Tab，旁边显示区间时长，区间模式另显示删除选区后的时长。头尾模式保留中间；两种模式都左键设起点、右键设终点，区间模式点击“剪切”删除两条边界之间的部分并拼接两端，波形和时间轴同步缩短。区间边界允许交叉，实际范围按两者较小值至较大值计算，淡入淡出也使用该范围；绿色边界可拖动，删除区域为 30% 不透明黑色遮罩，黄色游标仍可定位。淡入 / 淡出弹窗默认采用当前选区可用时长，可修改，通过下拉列表提供线性、平滑、等功率、二次、三次、平方根、更平滑七种曲线；下方曲线图支持圆形锚点及方形贝塞尔手柄，拖动手柄调整曲率，点击添加、右键删除中间锚点，端点时间固定，淡入起点 / 淡出终点可上下拖动调整幅度，另一端固定 100%，最多 8 个锚点，修改后显示“自定义”，按分段三次贝塞尔曲线处理；手柄时间顺序限制在各段内，幅度限制 0–100%，重新选择预设会替换编辑结果。音量(dB)弹窗调整当前选区增益，未选范围时调整剪切后当前时间轴整段；淡入、淡出和音量(dB)参数窗保持置顶，按内容设置最小宽度，说明可换行，应用 / 取消居中。设置时预览波形，确认后刷新音频与实际波形。Ctrl+Z / Ctrl+Y 和撤销 / 重做按钮按窗口独立维护步骤，沿用设置中的撤销步数。中间版本使用临时目录的浮点 PCM，无重复有损编码；另存为输出当前已提交的完整时间轴，支持 MP3 / WAV / FLAC / M4A（AAC），按格式显示采样率、声道、位深或码率，不覆盖已有文件。入口与窗口标题使用“音频编辑”，视频对应“视频编辑”。主窗口音频与音频编辑波形均支持鼠标位置锚定的滚轮缩放，最小显示全长，最大约 10 毫秒主刻度；放大后在刻度下方拖动水平滑条浏览；主窗口与音频编辑窗播放时，停止缩放操作 0.1 秒后黄线居中跟随，开头 / 结尾受时间轴边界限制，暂停不跟随。播放控制固定底部；停止返回 0:00，暂停保留当前位置。波形使用 8 kHz 解码峰值、每声道最多 400 万点，长音频细节受峰值上限限制，不支持采样点编辑。处理期间等待后台任务完成；增益可能削波，中间缓存占用磁盘，关闭窗口尽量清理，崩溃可能残留。不含交叉淡化、批量转换或自由多轨编辑。
- **日常使用**：最近文件、标签恢复、深浅主题、七种语言和在线更新；GitHub 接口限流时提示等待重试，也可打开发布页手动更新。

### 🚀 快速开始

1. [下载 Windows x64 程序](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)，直接运行 EXE。
2. 将文件拖入窗口，或通过“文件”菜单打开；Unity 资源尽量保留在原项目目录中，以便解析引用。
3. 选择对象查看属性；需要修改时开启“编辑模式”。材质节点中的调整仅用于预览，不写回文件。

详细操作、支持格式和预览范围见[使用指南](docs/user-guide.md#中文)。

### ⚠️ 待支持与当前限制

- PDF 暂不支持编辑、文字选择、搜索、缩略图和密码输入；Office 与苹果办公文档预览 / 编辑未实现。

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

- **Third-party Support**: A separate manager under Settings shows FFmpeg / FFprobe status and resolved paths and retains WinGet installation. PDFium offers on-demand download of the Windows x64 build without V8, SHA-256 verification, version / path display, re-detection and removal. Downloads come from bblanchon/pdfium-binaries and extraction uses Windows tar. Components live in `%APPDATA%/UnityYAMLViewer-components`, outside disposable cache roots; FFmpeg remains managed by WinGet. LibreOffice also offers WinGet installation, version / path detection and manual program selection; installation may require administrator approval and remains system-managed. Opening the manager only checks local components; installation starts on an explicit button click. FFmpeg is checked when the manager opens; post-installation checks and Re-detect results sync to the main window and retry failed media previews. Closing the manager triggers no extra detection.

- **File tabs**: More than 3 files with the same extension (case-insensitive) collapse into a dropdown showing the last active file of that group. The menu follows Recent Files sizing with a maximum width, left-aligned single-line truncated names and full-path tooltips; it highlights that file and uses borderless buttons and vector file-stack / close icons with the member count after the stack, with an × per entry and Close All at the bottom. Unsaved resource / playlist edits prompt before closure; cancelling stops the remaining batch. Individual closure to 3 or fewer files expands the group in place, in member order; single-member tabs use × without a dropdown. Close All suppresses dissolution until completion; cancelling restores automatic expansion. Regular tabs and collapsed groups support drag reordering; groups move as a whole and retain member order. Independent media editor windows retain their existing lifecycle.
- **PDF preview**: Download and verify PDFium through Third-party Support for read-only multi-page viewing, an auto-page switch after the filename for multi-page PDFs (controls hidden for single-page files and while page count is unknown) with a 1–3600 second interval (default 5, active fullscreen; an adjacent Loop switch is off by default, returning to the first page when enabled and stopping at the last page otherwise), a Transition menu after Loop (default None; Crossfade, Slide, Push, Wipe, or Zoom and fade; 0.25 seconds, also fullscreen), a floating page / zoom badge with circular vector previous / next buttons, arrow-key navigation, black-background fullscreen viewing (Fullscreen after Reload or F11 to enter, F11 / Esc to exit), fit window / width, fixed scales, wheel zoom, pan and reload. Missing-component prompts open the manager directly; post-installation verification and Re-detect results sync to the main window and retry failed previews; manual Retry remains available. Closing the manager triggers no refresh. 100% uses a 96 DPI logical-pixel baseline with system DPI scaling, physical-pixel alignment and screen text antialiasing. Files are limited to 128 MiB; background rendering uses the current page and DPI, capped at 4096 pixels per side. The previous page stays visible until the next is ready; the spinner appears only after one second of loading. Pages have a white background; scripts, interactive forms and encrypted documents are not supported.

- **SVG and GIF preview**: Read-only SVG viewing supports checkerboard / gray / custom backgrounds using the Prefab preview controls, fit-to-window, 50% / 100% / 200%, wheel zoom, panning and reload. Background rerendering is capped at 4096 pixels per side; source files are limited to 16 MiB. Supported static paths / shapes follow the renderer; convert text to outlines. External resources, embedded bitmaps, scripts and animation are not supported. GIF hides volume, audio information and audio-track menus and displays the largest global/local palette capacity (not the actual used color count). GIF uses external FFmpeg / FFprobe and the media controls for play, pause, stop and seeking. It reuses video UI, with a duration-proportional timeline showing current / total time and frame numbers as 0:00:00.000(f00)/0:00:00.000(f00) · 0.030 s/f (fixed units across languages; — s/f until timing is indexed), hover details and frame boundaries where space permits. Loop modes are File Setting, Once, Forever and Custom Total Count. NETSCAPE / ANIMEXTS loop extensions are read: zero means forever, absence means once, and positive values follow FFmpeg total-iteration semantics. Pause preserves progress; Stop rewinds and resets the iteration count. SVG hides channel and nine-slice controls; backgrounds affect preview only and survive reload without modifying the file. SVG / GIF have no nine-slice or editing, and GIF has no channel switching; SVG editing is planned for later.
- **Asset browsing**: Open text assets such as Prefabs, Scenes and Materials with a Hierarchy, Inspector, search, reference navigation and tabs.
- **Dialog positioning**: Dialogs center on the main window when first opened or explicitly summoned by a button. Clicking or switching focus preserves their current position.
- **Image editing**: Native RGBA editor with settings at upper left, dimensions, savings and expandable details at upper right, and crop / result views below. The marked crop view replaces the separate original display. Both rows retain equal column widths as the window resizes; long text wraps within its column. Preview heights grow with the window, pixel dimensions sit centered on rounded backgrounds just below the image border labels, and notes and actions remain at the bottom. A translucent fill with sparse diagonal hatching marks removed pixels; it automatically picks a contrasting candidate color, with a red fallback option. Repeat outlines retained cycles and marks minimum-cycle boundaries. Both / Horizontal / Vertical and Stretch / Repeat use theme-style tabs alongside Auto Detect. Settings have no internal scrollbar; their content determines the top height, and information scrolls independently when taller. The original Border overlay toggle sits with the settings. Auto Detect compares all six combinations by actual aligned area and explains limitations for each axis. Retained dimensions, diagnostics, cycles and option comparisons are collapsed by default; Details distinguish compactable, retained and removed pixels. View Details beside the status expands them, and reopening resets the collapsed state. While dragging, the previous preview stays visible and saving waits for the current result. Without readable metadata, footer buttons are centered. With metadata, footer information sits on the left, with larger green Replace and blue Save As buttons vertically centered on the right and clearance for the scrollbar. When the native editor overflows, its scrollbar stays visible and reserves space instead of overlaying content. Repeat detects the smallest contiguous cycle and supports separate cycle multiples per axis. Border inputs form one group with its label vertically centered beside both rows, and use two rows (Top / Bottom in the left column, Left / Right in the right), with px directly beside each field and 12 px before the next direction group. Retained center width / height default to 4 px and remain editable. Transparent edge preservation and dimensions in multiples of 4 are enabled by default. Highlights output dimensions and area changes in green for savings, orange for growth and gray for no change. Shows actual retained dimensions, cycles, saved area and padding, and optionally overlays original Unity Border values in green on the crop view, separate from proposed split lines in yellow / blue. Confirm in-place replacement without backups and reload the tab, or save a new PNG. Metadata controls appear only after reading a companion single-Sprite `.meta`; Border / Full Rect updates preserve GUID.
- **Property editing**: Edit recognized properties, colors, vectors and lists, with validation, undo and saving.
- **Material previews**: A **node-based UI** for Materials / Shaders, with temporary inputs, texture drops, 2D / 3D output and Cull / Blend controls. Common formulas use 100-point-wide operation nodes with expandable color previews refreshed at up to about 60 fps while visible, left-click selection and marquee selection even with the output offscreen, unified selection and dragging of nodes and groups, right-drag panning, draggable function groups with framed fold / expand and eye preview buttons (including R/G/B/A channels and their operations) and named boundary ports and 10%–175% zoom.
- **Scenes and animation**: Preview supported 2D / UI, Spine and model assets, inspect Animator graphs and edit animation curves.
- **File locations**: Right-click tabs, grouped file entries or image / PDF / media preview filenames to reveal and select the file in File Explorer or copy its full path. Missing files cannot be revealed; remote cached files point to the local cache.
- **Media playback**: Video / GIF playback can be toggled by left-clicking anywhere in the preview canvas, including margins. Panning in free-zoom mode does not toggle playback; timeline and playlist controls keep their own interactions. Playlists, waveforms, fullscreen and remembered positions. Trim/Convert opens an independent native window and stops the source tab. Editors have independent playback and preview volume, survive source-tab switching or closure, support multiple files and reuse an existing window for the same file. Video currently supports range export to MP4 (H.264 / AAC), main video and current audio only, without subtitles. Playback and editing require external FFmpeg, installable / detectable under Settings → Third-party Support.
- **Audio editing and conversion**: Ends / Interval use theme-style tabs with selection duration; Interval also shows duration after deleting the selection. Ends retains the middle; Both modes use left-click for start and right-click for end. Cut removes the middle and joins both retained ends, shortening the waveform and timeline. Interval boundaries may cross; the selected span always runs from the smaller value to the larger, also for fades. Green markers are draggable, removed regions use a 30%-opaque black mask and the yellow playhead remains usable. Fade dialogs default to the available selected duration, allow changes and offer seven dropdown curves: linear, smooth, equal-power, quadratic, cubic, square root and smootherstep. The graph has round anchors and draggable square Bezier handles for curvature, with click-to-add and right-click-to-delete interior anchors, fixed endpoint times and up to 8 anchors; the fade-in start / fade-out end can move vertically to set amplitude, while the opposite endpoint stays at 100%. Manual edits switch to Custom with piecewise cubic Bezier interpolation; handles remain time-ordered within their segment and amplitude is limited to 0–100%; selecting a preset replaces the edits. Gain (dB) affects the selected span; without a range it affects the entire current edited timeline, excluding removed content. Fade and Gain (dB) dialogs stay on top and size to their contents, wrap explanatory text and center Apply / Cancel. Settings preview the waveform, and Apply refreshes the audio and actual waveform. Ctrl+Z / Ctrl+Y and Undo / Redo are per window and respect the configured undo-step limit. Intermediate revisions use temporary floating-point PCM without repeated lossy encoding. Save As exports the complete committed timeline as MP3 / WAV / FLAC / M4A (AAC), with format-specific rate, channel, depth or bitrate controls and no overwrite. Entry points and titles use Audio Editor / Video Editor. Main audio playback and the audio editor support mouse-anchored wheel zoom, from the complete timeline to roughly 10 ms major ticks, with a horizontal browsing thumb below the ruler when zoomed. During main-window and Audio Editor playback, 0.1 seconds after zoom input stops, the waveform follows the centered playhead, clamped at the timeline ends; paused playback does not follow. Playback controls stay at the bottom; Stop returns to 0:00, while Pause keeps the current position. Waveforms use 8 kHz decoded peaks capped at four million points per channel; long recordings have reduced detail, and sample-level editing is not supported. Processing runs in the background; gain may clip, caches consume disk space and cleanup is best-effort, with possible leftovers after a crash. Crossfades, batch conversion and unrestricted multitrack editing are not supported.
- **Everyday tools**: Recent files, restored tabs, light / dark themes, seven languages and online updates; when GitHub rate limits checks, wait before retrying or open the release page to update manually.

### 🚀 Quick start

1. [Download the Windows x64 executable](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest) and run it directly.
2. Drop a file into the window or open it from the File menu. Keep Unity assets inside their original project where possible so references can be resolved.
3. Select an object to inspect it; enable Edit Mode to change properties. Material-node adjustments affect the preview only and do not write files.

See the [user guide](docs/user-guide.md#english) for detailed controls, formats and preview support.

### ⚠️ Pending support and limitations

- PDF editing, text selection, search, thumbnails and password input are not supported. Microsoft / Apple office document preview and editing are not implemented.

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
