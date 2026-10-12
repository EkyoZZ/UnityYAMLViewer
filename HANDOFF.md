# 1.1.7 发布准备交接（2026-10-13）

本节优先于下方历史交接。当前源码与已构建 EXE 均为 1.1.7；用户准备自行发布，尚未上传、创建发布标签或正式对外发布。

## 当前成果

- 版本：Cargo.toml / Cargo.lock 1.1.7；README、中英版本摘要、关于窗口与七语言更新条目已同步。
- 视频全屏图标、F11 / Esc、纯黑背景、上下约 50% 不透明度浮层，鼠标静止 1 秒后隐藏控件与指针。
- 三种视频缩放模式、自由缩放 5 个逻辑像素点击 / 拖动区分、全屏物理像素边界对齐、音视频空格播放控制。
- 音频峰值线宽 2 个物理像素，上升立即跟随、下降每秒 5 dB；用户后续未要求再降低速度。
- FFmpeg 检测读取最新用户 / 系统 PATH 与 WinGet 包目录；检测成功后关闭设置自动重试当前失败媒体页。用户重新安装后确认没有问题。

## 文件与产物

- 完整当前记录：docs/versions/1.1.7.md；对外发布说明：docs/releases/1.1.7.md。
- 发布目录：target/publish/v1.1.7，包含版本命名 EXE、SHA256SUMS.txt、release-notes.md；target 被 Git 忽略，换电脑要额外复制。
- 程序原输出：target/release/unity_yaml_viewer.exe。大小及 SHA-256 见下方本次产物核对。
- 本轮提交标题：发布 v1.1.7：完善音视频交互与 FFmpeg 安装体验。完整提交包含此前工作区的相关改动；用 git log -1 核对提交号。不自动推送或创建标签。

## 验证与限制

- cargo check --offline --locked --tests、Release 构建、git diff --check 通过，七语言列数核对通过；测试源码仅编译，未运行测试断言。本次没有启动应用或进行全面回归。
- 用户确认最近一次 FFmpeg 安装与关闭设置后的自动刷新；更早第一次安装失败缺少日志，不能声称所有安装问题均已解决。
- FFmpeg 仍为外部组件；全屏、不同 DPI、输入焦点、自动隐藏与峰值动画的全面回归仍待发布前核对。非整数缩放仍有插值，100% 沿用逻辑尺寸。
- 统一 Shader 渲染系统、粒子、场景整体动画、字幕渲染、HDR 输出与精确声卡时钟同步尚未实现。
- Spine Runtime 保留 158 条已有编译警告；不沿用 1.1.4 的 98 项测试结果作为本版验证结论。

## 发布步骤

1. 使用 target/publish/v1.1.7 的 EXE 做最后运行核对；如修改源码需重新授权编译、检查占用、构建并更新副本 / 哈希。
2. 推送确认过的提交并创建 v1.1.7 标签；发布 GitHub Release，使用 release-notes.md 作为说明并上传版本命名 EXE 与 SHA256SUMS.txt。
3. 确认 GitHub 附件提供 SHA-256 digest，核对附件名、架构、大小；源码 ZIP 不是更新包。在线更新器仅替换 EXE。
4. 只有实际上传与对外发布完成后才修改发布状态；当前文档与构建不代表已经发布。

## 工程规则

- 文本保持 UTF-8；当前版本用户可见改动同步维护 docs/versions/1.1.7.md。
- 生成 EXE 前必须取得用户明确授权，并重新检查进程和目标文件独占访问；不得自动关闭程序或换目录绕过占用。
- 设置最后两项固定为单个文件缓存撤销步骤、检查更新，顺序及相邻关系保持。

## 本次产物核对

- 文件版本 / 产品版本：1.1.7
- 大小：18547712 字节
- SHA-256：2D0FE14F4E1931E4320D0A4AC1F31402B29596FC30AA9F35C66492ADB9F2AAED

---

# 1.1.4 开发交接（2026-10-11）

本节优先于后面的历史交接。源码版本已升级到 1.1.4；尚未正式发布，本次版本修改未生成 EXE。已有可执行文件不能视为已包含新版本号。

- 相对 1.1.3 的功能与正式发布日志草稿记录在 docs/versions/1.1.4.md，CHANGELOG.md 提供各版本索引；后续发布 1.1.4 最终版时以此为基础完善发行说明。
- 后续每次新增功能、修复或改变用户可见行为，同步维护当前版本功能记录；具体约定见 AGENTS.md。
- 本轮包括紧凑右上角主题与 SVG 图标、GameObject 层级与可折叠组件 Inspector、最近 10 个文件菜单、脚本/图片标签与基础高亮（Shader、.lua.txt）、图片通道/居中/拖动/缩放/半透明比例提示、工程字体回退、MeshRenderer 和 SpriteAtlas 属性显示。
- 新增模块和 assets/icons 必须一起提交或迁移。保留所有现有未提交修改。
- AtlasCache 样本已成功解码，但具体缓存定位尚未确认；用户明确暂缓 Atlas 打包图预览。不要列为已实现功能，也不要删除 Unity 缓存。
- 正式生成 1.1.4 EXE 前仍需本次明确编译授权及占用检查；发布附件与 SHA-256 应在最终构建后重新准备，不沿用历史 1.1.3 哈希。

---
# 换电脑交接：1.1.3 与更新弹窗后续修正

交接日期：2026-10-10。以下总览优先于后面的历史记录；历史中的未实现、旧版本和旧显示方案可能已被后续修改取代。

## 当前状态与下一步

- 当前源码版本 1.1.3。当前 HEAD：c291ec1，提交标题为「* 更新1.1.3 支持部分Prefab静态预览,暂不支持Spine和3D对象」。
- 1.1.3 已正式编译过，发布副本已准备好；用户随后提供了 1.1.2 检测到 v1.1.3 的更新弹窗截图，说明版本检测已走通。是否已完成下载、覆盖、重启，以及任务栏启动，尚未确认。
- 最新更新弹窗修正还在未提交工作区，未重新编译进现有 EXE。下一台电脑应先带齐这些文件，再检查源码，获得新的明确编译授权后重新构建并更新发布副本和校验文件。
- 未提交文件：HANDOFF.md、src/main.rs、src/translations.tsv、src/updater.rs；未跟踪的新文件：src/release_notes.rs。仅拉取 Git 仓库无法带走这些改动，尤其不能漏掉 release_notes.rs。此次交接没有自动提交、推送或发布。

## 换电脑要带走什么

1. 推荐复制整个工程工作区，保留 .git 和所有未提交文件。若仅同步 Git，需要自行提交或另外复制以上五个文件。
2. target 是构建输出，不应假设 Git 会同步。要保留现有发行附件，请额外复制 target/publish/v1.1.3；否则在新电脑按规则重建。
3. 运行验证用 Unity 项目也需另外同步，包含 prefab、贴图及 .meta、脚本。只带 prefab 无法解析外部 GUID 图片引用。
4. 旧版 1.1.2 EXE 和原任务栏快捷方式环境是在线升级测试的重要条件；要验证任务栏固定项，最好在同一台已有固定项的电脑验证。
5. 软件配置位于 APPDATA/UnityYAMLViewer，包括打开标签、语言、主题、最后目录和更新偏好。路径依赖本机位置，换电脑后可能需要重新打开文件。LOCALAPPDATA/UnityYAMLViewer/open_requests 是运行时命令转交目录，不必迁移。

## 必须遵守的工程规则

- 文本文件保持 UTF-8。
- 生成 EXE 前必须取得用户对本次编译的明确授权；此前授权不代表换电脑后可自行重编译。
- 编译前重新检查 unity_yaml_viewer 进程和目标 EXE 的独占 ReadWrite 访问。占用时告知用户关闭，等待再次明确授权并重新检查。不得杀进程或换输出目录绕过检查。
- cargo check --offline、cargo check --offline --tests 可用于检查；后者只编译检查测试代码，不代表测试实际执行通过。
- 任务结束时未运行程序、未自动更新安装、未改变任务栏固定项。

## 已完成并进入此前 1.1.3 的功能

- Prefab 静态 2D 预览：SpriteRenderer 的原始贴图、切片、pivot、PPU、颜色、翻转及层级位置/旋转/缩放。Polygon Sprite 可用原图矩形近似显示。不支持 Spine 与 3D 对象。
- UI：Image 普通、九宫格和横向/纵向填充，RawImage，Text/TMP 近似静态文字；RectTransform 锚点布局，CanvasGroup 透明度。字体与富文本不保证完全一致；动态布局、遮罩、平铺和径向填充尚未完整支持。
- 所有根节点三轴 Scale 全为零时仅在预览按 (1,1,1) 回退；子节点零缩放不改。屏幕模式根 Canvas 使用参考尺寸及有效缩放。文件和 Inspector 保存值保持原样。
- UI 默认和适应窗口以参考范围居中。最终视觉规则为左下角、右上角两个灰色十字，四条贯穿线围住参考范围，不绘制独立矩形框；不同 pivot 下按两角定位。
- 预览点击选择 SpriteRenderer / UI Graphic，并同时显示所属 Transform / RectTransform。层级双击将节点及可见子图形居中，保持缩放，不编辑节点位置。
- 层级眼睛/手开关控制预览显示和可选状态，父节点影响子节点；仅本机视图状态，不写入 YAML。
- 列表支持独立属性 / 表格切换；数值支持聚焦后的左右拖动，已有编辑和预览刷新优化。
- 命令行可传多个中文、带空格或相对路径。Windows 已有窗口时用独占锁和文件队列转交请求，切换标签并请求激活窗口；无参数再次调用激活已有窗口。

## 未提交的更新弹窗修正

- src/release_notes.rs：识别发行说明中的 HTML img src 和 Markdown 图片语法，后台 HTTPS 加载，等比例适配显示；失败提供原图链接。下载限制 8 MiB，尺寸限制 8192，解码分配限制 64 MiB。这是图片识别，不是完整 Markdown/HTML 渲染器。
- src/updater.rs：新增图片状态，切换发行版本时清空；page_url 使用固定项目 URL 和编码后的 tag 路径。
- src/main.rs：更新 / 取消按钮 120x36 居中；确认弹窗固定 320 宽、说明和未保存路径换行、确认 / 取消按钮 120x36 居中。
- 发行说明弹窗以及应用内检查/下载/安装启动失败消息提供「打开发布页面，手动更新」；已知版本打开指定 tag，否则打开 latest。后台安装脚本失败的系统 MessageBox 尚未增加可点击链接。
- src/translations.tsv：补充相关七语言文本。行必须保持七列。
- 用户截图来自旧版 1.1.2。新版改动无法改变已经运行的旧版更新弹窗；新版安装后才会影响未来更新提示。

## 构建输出与发布

现有构建尚未包含上面未提交的弹窗改动：

- 原输出：target/release/unity_yaml_viewer.exe。
- 发布附件：target/publish/v1.1.3/UnityYAMLViewer-v1.1.3-windows-x64.exe。
- 大小：13264896 字节。
- SHA-256：1aee00e31e58450e6fcb6b5d7d621c14aa7f26e561c3c1ba0b097c0fe5a34a40。
- 同目录有 SHA256SUMS.txt 和 release-notes.md。重新编译后必须重新复制发布附件并计算哈希；旧哈希不能沿用。
- 发布说明：支持 Prefab 预览，目前仅支持 2D，Spine 和 3D 对象暂不支持。
- 标签 v1.1.3，附件名称需符合上述命名或 unity_yaml_viewer.exe。更新源：https://api.github.com/repos/EkyoZZ/UnityYAMLViewer/releases/latest。
- 当前安装代码使用 current_exe() 原路径覆盖，保留原文件名，不按发行附件改名。任务栏快捷方式理论上保留目标路径；若原名包含版本号，名字仍会保留旧版本数字。实际固定项仍需运行验证。
- 升级会校验附件大小、GitHub digest SHA-256 和 PE 架构，下载后需用户确认，未保存改动会阻止安装；后台等待旧进程退出、备份、覆盖和重启。

## 验证状态与建议顺序

已通过多次 cargo check --offline，早期功能也通过 cargo check --offline --tests；没有实际执行测试程序。更新说明图片尚未通过在线加载或窗口视觉验证。GitHub 网络在本次工具环境不可用，不能据此断言用户电脑不可访问。

新电脑建议：

1. 检查上述未提交文件都存在，运行编译检查与 git diff --check。
2. 核对确认窗换行、按钮布局、图片加载失败入口；获得授权并完成占用检查后构建最新版。
3. 用 1.1.2 实测检测 v1.1.3、下载/哈希验证、取消、未保存阻止、确认、覆盖、重启、关于版本显示和任务栏启动。截图目前仅确认检测及弹窗出现。
4. 用包含新修正的程序检查后续发行说明，验证 HTML 图片、Markdown 图片、中文文本、宽图缩放、加载失败、两个弹窗的按钮居中。1.1.3 同版本不能触发自身更新，可使用后续实际发行版测试；不要误称旧版弹窗已经修复。
5. 若 1.1.3 已正式发布，是否替换附件或发布后续补丁版本由用户决定，不擅自更改线上附件或版本。

## 样例及关键文件

实际样例是 C:/Git/g003_client/Assets/Res/Prefabs/UI/Panel/ui_tips_panel.prefab，注意是 g003_client；之前误用 q003_client 导致无法读取。根 Canvas RenderMode=1，根 Scale=(0,0,0)、SizeDelta=(0,0)、Pivot=(0,0)，参考分辨率 864x1536，子 CanvasGroup alpha=1。两张图片位于 Res/Textures/UI/BoardV2/ui_boardv2_lucky_word.png 和 ui_boardv2_lucky_super_word.png。用户最新截图已显示图片文字及双十字参考范围，仍提示四项不支持/不可见，需要按详情判断，不应直接忽略。

关键源码：src/static_preview.rs、src/ui_preview.rs（渲染/布局），src/hierarchy_controls.rs（节点开关），src/command_line.rs（启动参数/转交），src/script_properties.rs 和 src/editing.rs（属性/编辑），src/updater.rs 与 src/release_notes.rs（更新），src/main.rs（窗口和交互）。

---

# 历史工作记录

## 2026-10-10 更新弹窗补充调整

确认更新弹窗固定 320 宽，说明及未保存路径换行，确认 / 取消按钮 120x36 居中。发行说明弹窗新增发布页超链接，更新消息弹窗（含下载和安装启动失败）也显示入口；已知版本链接到项目 releases/tag/<tag>，无版本时链接到 releases/latest。URL 固定项目域名并按路径段编码 tag。外部安装脚本自身失败的 Windows MessageBox 暂沿用旧提示。图片说明渲染和按钮修复仍未重新编译到发布 EXE，现有发布副本为之前的 1.1.3。cargo check --offline 已通过。

## 2026-10-10 更新弹窗图片和按钮修复（1.1.3 未重新构建）

新增 release_notes 模块解析 Release 说明里的 HTML img src 和 Markdown 图片语法，后台 HTTPS 加载并限制 8 MiB 下载、8192 尺寸及 64 MiB 解码，等比例适配说明宽度，失败显示提示和原图链接。更新 / 取消按钮统一 120x36 并居中，下载中布局一致。切换发行版本时清除图片缓存。cargo check --offline 通过，尚未重新生成发布附件。注意截图由旧 1.1.2 渲染，新版修复无法改变已发布旧程序的弹窗；需更新后才能用于后续版本说明。

## 2026-10-10 正式编译 1.1.3 完成

用户明确授权正式编译并准备自行发布。进程及目标 EXE 独占访问检查通过，cargo build --release --offline 成功（1.1.3）。原输出 target/release/unity_yaml_viewer.exe；发布副本 target/publish/v1.1.3/UnityYAMLViewer-v1.1.3-windows-x64.exe，附 SHA256SUMS.txt 和 release-notes.md。尚未上传或发布 GitHub Release，1.1.2 到 1.1.3 在线升级及任务栏快捷方式仍待实测。

## 2026-10-10 版本升级到 1.1.3

按用户要求更新 Cargo.toml 和 Cargo.lock 的应用版本为 1.1.3。关于窗口新增 1.1.3 更新内容：支持 Prefab 预览，目前仅支持 2D，Spine 和 3D 对象暂不支持；在线更新历史明确归到 1.1.2。README 中英当前版本与新增更新说明同步，应用说明补齐七语言。cargo check --offline 与 git diff --check 通过。未生成 1.1.3 EXE，现有 release EXE 仍为先前 1.1.2。

## 2026-10-10 最新 Release 构建（UI 双十字范围）

用户明确授权后，通过进程及目标 EXE 独占访问检查，cargo build --release --offline 成功。版本 1.1.2，输出 target/release/unity_yaml_viewer.exe。UI 参考范围不再绘制独立矩形，左下角和右上角各绘制灰色贯穿十字，四条辅助线围出参考范围，按范围两角定位而非固定原点，支持不同 Pivot。未自动启动或验证交互画面。

## 2026-10-10 UI 参考矩形改为右上角十字辅助线

按用户最新截图和补充要求，去掉 UI 参考范围闭合矩形，改为参考范围右上角的横竖贯穿预览辅助线，与原点十字使用同样灰色。参考范围仍用于默认居中与适应窗口，辅助线同步尺寸、缩放和平移。cargo check --offline 通过，未生成新版 EXE。

## 2026-10-10 最新 Release 构建（根 Canvas 空白修复）

用户明确授权后重新通过进程及 EXE 独占访问检查，cargo build --release --offline 成功，版本 1.1.2，输出 target/release/unity_yaml_viewer.exe。包含屏幕模式根 Canvas 参考布局、根节点全零 Scale 预览回退 (1,1,1)、独立 UI 参考框及不可见原因诊断。未自动启动程序，实际画面待用户验证。

## 2026-10-10 根节点零缩放预览回退

按用户要求，在公共 local(doc, root) 变换计算中，根节点 LocalScale 三轴全部为零时仅预览回退到 (1,1,1)。适用于普通 Sprite、RectTransform 和世界 Canvas 根节点；非零根缩放及子节点零缩放保持原值，不修改 YAML 或 Inspector 显示值。新增根/子零缩放区别的回归检查，cargo check --offline --tests 通过，未生成 EXE，未执行运行验证。

## 2026-10-10 ui_tips_panel 空白预览根因修复

实际文件位于 C:/Git/g003_client/Assets/Res/Prefabs/UI/Panel/ui_tips_panel.prefab（此前路径 q003_client 不存在）。根 Canvas RenderMode=1，根 RectTransform LocalScale=(0,0,0)、SizeDelta=(0,0)、Pivot=(0,0)，CanvasScaler 参考尺寸 864x1536，子 CanvasGroup alpha=1。零根缩放将全部图形压为零面积。新增 seed_screen_canvases 为启用且激活的屏幕模式根 Canvas 预设布局，参考尺寸作为大小、归一化根缩放为 1，保持根旋转/pivot、子节点原始变换，不改文件；世界空间 Canvas 保持原逻辑。参考框改为独立参考平面，不受活动和缩放影响。跳过图形新增布局、尺寸、零面积、透明度诊断。两张引用图片 GUID 已在 Res/Textures/UI/BoardV2 下匹配 lucky_word 和 lucky_super_word。新增屏幕模式零缩放回归检查，并验证世界模式不归一化（仅编译检查，不执行）。尚未生成新 EXE 或验证运行画面。

## 2026-10-10 最新 Release 编译完成

用户关闭占用程序并再次明确授权后，重新通过进程及目标 EXE 独占访问检查，cargo build --release --offline 成功。版本 1.1.2，输出 target/release/unity_yaml_viewer.exe；包含命令行文件打开及现有窗口转交、列表属性 / 表格切换、双击层级节点居中、UI 参考尺寸灰色边框及参考范围默认居中。未自动启动程序，交互效果仍待实际运行验证。

## 2026-10-10 UI 参考范围边框和默认居中

UI 预览以首个根 RectTransform 的 pivot 和矩阵放置参考尺寸矩形，使用与十字轴线一致的灰色线条。默认视图及适应窗口以参考矩形的边界中心和尺寸计算，超出参考范围的图形不影响默认中心或缩放；非 UI 保留原图形边界算法。边框同步参考尺寸、缩放和平移，双击节点仍通过平移居中目标。cargo check --offline 通过，未生成 EXE 或运行验证。

## 2026-10-10 双击层级节点居中预览

GameObject 和组件标签双击会打开静态预览并请求定位所属节点及可见子图形的联合边界中心。支持 SpriteRenderer、UI Image、RawImage 和 Text/TMP；保持当前缩放，只调整预览平移，不编辑 Transform 或 YAML。目标集合沿 Transform/RectTransform 子层级构建，有环时集合增长终止。纹理尚未加载时最多等待 10 秒，无可见图形不改变视图。cargo check --offline --tests 通过，未生成新版 EXE，未运行交互验证。

## 2026-10-10 Inspector 列表属性 / 表格切换

在 typed_field_inner 的可表格化数组及 List 列表中增加属性 / 表格切换，默认保留表格。各列表按 UI ID 独立记忆当前会话选择；属性模式按索引折叠展开各元素并逐字段显示，复用原 typed_field 和编辑 with_row 范围，因此类型控件、引用跳转、编辑及还原沿用现有逻辑。表格宽度、滚动和冻结表头状态保留。原始列表 YAML 仍可展开，packed 数据和无法表格化的复杂嵌套结构沿用原显示。新增七语言按钮翻译。cargo check --offline --tests 通过，未生成 EXE 或运行验证。

## 2026-10-10 命令行打开及 Windows 单窗口转交

新增 src/command_line.rs，读取所有文件路径参数（绝对化及规范化），启动恢复标签后打开传入文件。Windows 使用 LOCALAPPDATA/UnityYAMLViewer/open_requests 下独占 instance.lock 文件锁及原子 JSON 请求文件队列，现有窗口每 250ms 接收请求、打开标签并请求恢复和聚焦窗口；再次无参数调用仅激活窗口。锁随窗口释放，转交时原窗口退出可接管，5 秒未响应显示错误。非 Windows 仅支持启动路径参数。新增直接依赖 serde_json（已有缓存）。已完成 cargo check --offline；未生成新版 EXE，未运行验证。旧版窗口必须关闭后首次启动新版。

## 2026-10-10 最新 Release 构建完成（UI Prefab 预览）

用户明确授权编译后，已通过进程和目标 EXE 独占访问检查，并完成 cargo build --release --offline。版本 1.1.2，输出 target/release/unity_yaml_viewer.exe。包含 UI Image、九宫格、RawImage、Text/TMP 近似静态预览及 Graphic 点击选择；字体、动态布局和遮罩等限制见后续实现记录。未启动程序，尚未进行实际 UI Prefab 运行验证。

## 2026-10-10 UI Prefab 首批静态预览（未重新构建）

用户明确要求支持 UI Prefab，并追加 Text/文本框显示；接受字体效果暂不完全一致。新增 ui_preview.rs：识别 uGUI Image/RawImage/Text 与 TMP，计算 RectTransform 锚点、父尺寸、sizeDelta、anchoredPosition、pivot、TRS，按 m_Children 兄弟顺序建立 UI 绘制/点击顺序，Canvas 启用及 CanvasGroup alpha 继承。根尺寸缺失优先 CanvasScaler 参考分辨率，默认 1280×720；预览工具提供 UI 参考尺寸调整。

static_preview.rs 支持 Image Simple/Sliced（meta Border、PPU/Canvas reference PPU/multiplier、Fill Center、尺寸小于边框时等比压缩）、线性 Filled、RawImage UVRect，空 Sprite 白色矩形及 Unity 内置 Sprite 颜色矩形近似并提示。Text/TMP 使用系统字体绘制内容、字号、颜色、对齐、换行与旋转，基础富文本标签剥除；输入框显示其子 Text/TMP 内容。文字与图像按相同 UI 层级顺序绘制/命中，点击选择 Graphic，眼睛/手继承继续生效，Sliced 选中只绘制一圈矩形边框。

main Inspector 对 UI Graphic 联合显示所属 RectTransform，script_properties.rs 增加已知 uGUI 常见字段的编辑，不依赖 package 脚本成功解析；显示名识别 Image/RawImage/Text/TMP。暂存修改经现有 rendered 缓存实时更新。

限制：Tiled、径向 Filled、Mask/RectMask2D 裁剪、LayoutGroup/ContentSizeFitter 等动态布局、嵌套 Prefab、精确 TMP SDF/字体材质/富文本特效未实现；文字非均匀缩放及倾斜透视仅近似。没有 Unity 环境或用户实际 UI 样本，尚未运行界面验证。cargo check --offline --tests 及空白检查通过，锚点/九宫格/文字测试仅编译，未执行断言，未重新生成 EXE。版本仍 1.1.2；构建须明确授权并检查占用。

## 2026-10-10 预览点击目标改为渲染组件（未重新构建）

按用户最新要求，预览点击 Sprite 改为选择 SpriteRenderer 的 fileID，Inspector 显示 Sprite 属性并保留已有所属 Transform 联合显示，Hierarchy 定位对应组件；选中轮廓随渲染组件同步。预览命中结果携带渲染组件 ID，为后续 UI Image 等 Graphic 选择沿用同一约定。当前 UI 图形渲染尚未实现，不能声称已支持 UI 预览/点击。尚未重新生成 EXE。

## 2026-10-10 最新 Release 构建完成（预览点击选中）

用户明确授权可以进行编译；unity_yaml_viewer 进程检查和目标 EXE 独占读写检查通过。cargo build --release --offline 成功（15.12 秒），target/release/unity_yaml_viewer.exe 为 1.1.2，已包含预览点击 Sprite 选择所属 Transform/RectTransform、透明像素和禁选过滤、层级定位与选中轮廓。尚未启动此构建验证预览点击及重叠选择交互，测试断言未执行，未提交或发布。

## 2026-10-10 预览点击选择 Transform（未重新构建）

用户指出预览应能点击图层选中所属节点 Transform，实际位置修改仍仅通过编辑模式的属性窗口进行，无需预览拖动修改对象。已将画布 Sense 改为 click_and_drag，单击按绘制顺序逆序命中 Sprite 矩形，逆变换处理旋转/镜像并检查源图 alpha，跳过透明像素与手图标禁选对象；返回所属 Transform/RectTransform fileID，由 main 同步 Inspector 属性页及 Hierarchy 展开定位，清除搜索过滤。选中的 Transform 图形显示细蓝色矩形轮廓。拖动仍只平移预览，不改变对象位置。

纹理缓存保存 alpha 字节以命中测试，缓存预算按每像素 5 字节统计。新增旋转/镜像/退化矩形命中测试源码，cargo check --offline --tests 和差异空白检查通过，测试断言未执行，尚未运行预览点击/重叠/禁选交互或重新生成 EXE。

## 2026-10-10 最新 Release 构建完成（数值拖动及性能优化）

用户明确授权进行编译，unity_yaml_viewer 进程与目标 EXE 独占读写检查通过。cargo build --release --offline 成功（9.99 秒），target/release/unity_yaml_viewer.exe 为 1.1.2，包含数值框聚焦后水平拖动、Shift/Ctrl 速度调整、拖动合并撤销、暂存修改实时预览、减少层级 YAML 重复解析及文件检查，以及无边框预览入口和紧凑层级间距。尚未启动此构建验证数值拖动、输入、撤销和实际流畅度，测试断言未执行，未提交或发布。

## 2026-10-10 数值框拖动与编辑性能优化（未重新构建）

用户要求数值编辑更流畅，并在聚焦输入框后按住左键左右拖动改值，保留文本输入、不使用滑条。已在 editing.rs 的通用数值及向量分量 TextEdit 接入水平 scrub：整数默认每像素 1，浮点每像素 0.1；Shift 慢调、Ctrl 快调；沿用类型/范围验证，单次拖动 change_grouped 合并一次撤销，失焦/Enter 文本提交保持原有行为。Euler 拖动仍转回四元数；超出 f64 精确整数范围的 64 位整数保留键盘输入。向量输入 ID 不再随 session revision 变化。

卡顿静态排查发现 node_label 对每个组件/对象反复调用 document_text，经 controller_source 完整解析 YAML；已仅在 GameObject 名称时读取已知文档范围，避免每个节点解析全文。parse_unity_yaml 正则改 OnceLock 缓存。预览文件 metadata 检查降为每秒一次。预览此前读取原始 documents，已改读取 Session.rendered 并缓存解析结果，确认文本/拖动修改实时反映到图形且不修改源 offsets。

另有尚未构建的 UI 微调：预览按钮无底色无边框、绘制细线左右箭头，悬停浅背景；左侧图标保留宽度 30，去掉额外 4px 留白，GameObject 树缩进改 14px。

cargo check --offline --tests 与差异空白检查通过。新增暂存向量预览/原始范围不变/单次撤销测试仅编译，未执行断言；拖动行为、实际流畅度、数字键盘输入尚未运行验证，未重新生成 EXE。后续构建仍须明确授权并检查进程和 EXE 占用。

## 2026-10-10 最新 Release 构建完成（预览修正）

用户明确授权可以进行编译；unity_yaml_viewer 进程检查与目标 EXE 独占读写检查通过。cargo build --release --offline 成功（10.26 秒），target/release/unity_yaml_viewer.exe 为 1.1.2，包含图标左侧固定两列及缩小、预览展开/收起箭头、灰色坐标轴、预览问题详情、Polygon Sprite 矩形兼容和 Hierarchy 双轴滚动黑条修正。尚未启动此构建验证两棵树恢复、图标布局或黑条消失，测试断言未执行，未提交或发布。

## 2026-10-10 层级/预览之间间歇黑条排查（未重新构建）

用户截图显示层级与预览之间偶发黑色竖条。检查 egui 0.31 SidePanel/ScrollArea 源码发现：Hierarchy 使用仅纵向 ScrollArea，长名称/深层缩进使非滚动轴内容宽度增大，Frame/SidePanel 按内容实际宽度推进下一个面板，但 SidePanel 仍裁剪于配置宽度，因而两面板之间可能留下未绘制区域并露出默认黑色窗口底色。将 Hierarchy 改为双轴 ScrollArea 并指定独立 ID，使长名称通过横向滚动查看，视口宽度保持面板约束。实际旧 EXE 黑条尚未运行复现/验证修复，未重新生成 EXE。

## 2026-10-10 Polygon Sprite 矩形预览兼容（未重新构建）

用户补充缺失树使用多边形轮廓，应仍可按矩形预览。检查发现 region 仅支持 spriteMode 1/2，直接拒绝 Unity Polygon 模式 3。已将模式 3 与 Single 一样按完整源图矩形、spritePivot/Pixels Per Unit 绘制，忽略 outline/physicsShape 的网格裁剪，保留图像 alpha。不修改资源导入设置。Multiple/Tight 已按 rect 取图，不依赖多边形轮廓。

新增 Polygon 带 outline/physicsShape 及自定义 pivot、Multiple/Tight 带 outline 的测试源码，仅做编译检查；尚未取得用户树资源验证实际 spriteMode，因此这是已修复的明确兼容漏项，不能断言两棵树已实际验证恢复。未重新生成 EXE。

## 2026-10-10 预览布局与缺失资源诊断调整（未重新构建）

按用户截图将眼睛/手缩至 15 宽按钮，放在 Hierarchy 左侧固定两列，树箭头及名称在独立缩进区域，不再随节点缩进挤占名称；预览按钮加入 ◀（展开中）/▶（收起）箭头，坐标轴改为低对比灰色。增加可展开预览问题详情，显示对象/纹理路径/fileID 与 GUID 缺失、绘制模式不支持、解码失败或切片无效等原因。

用户截图两棵树未显示且报 4 项缺失/不支持；当前环境无法读取 C:/Git/q003_client 示例工程，因此尚未确认这两棵树的实际失败原因，不得声称已修复树渲染。需提供对应资源或由用户查看新版问题详情。源码编译检查通过，尚未重新生成 EXE，后续构建仍须明确授权并检查占用。

## 2026-10-10 最新 Release 构建完成

用户明确授权进行编译，构建前未发现 unity_yaml_viewer 进程，目标 EXE 当时不存在。cargo build --release --offline 成功（18.21 秒），生成 target/release/unity_yaml_viewer.exe，版本 1.1.2，包含静态 Prefab Sprite 组合预览、Sprite/Transform 联合 Inspector 及眼睛/手父子继承开关。尚未启动验证界面或执行测试断言，未提交或发布。

## 2026-10-10 层级可见性与选择开关（未生成 EXE）

GameObject 节点前新增绘制的眼睛/手图标，开启与斜线禁用状态、悬停说明支持七语言。眼睛控制静态预览是否渲染，手控制层级节点与其组件是否允许选中；父节点关闭会沿 Transform/RectTransform 层级对子节点生效，子节点本地偏好独立保留，父节点恢复后继续按子节点本地开关生效。禁选当前对象或祖先会清除当前选择，展开箭头与图标仍可操作。开关不修改 Unity 的激活状态、Renderer Enabled 或源 YAML。按文件在本次打开会话中隔离，关闭标签清除。

新增 `src/hierarchy_controls.rs`（状态、父级继承、绘制图标及测试源码），`main.rs` 接入自定义层级标题和选择限制，`static_preview.rs` 读取有效可见性过滤 Sprite。`cargo check --offline --tests` 通过，测试断言未运行，未生成 EXE、未验证界面。

用户询问 Spine：当前预览不支持其动态 MeshRenderer 网格。将来需解析 skeleton JSON/binary、atlas/纹理、皮肤和附件并接入兼容运行时/骨骼网格计算；不需要安装 Unity/Spine 编辑器本身。当前没有加入 Spine 渲染实现。

## 2026-10-10 静态 Prefab 预览首批实现（未生成 EXE）

用户确认以整个 Prefab 为原点，按层级 Transform/RectTransform 的坐标、缩放、旋转构建 Sprite 静态组合；选中 SpriteRenderer 时同时显示所属节点的 Transform 属性。

新增 `src/static_preview.rs`，XY 正交投影，根位置归零，保留根缩放和旋转；父子完整 TRS 矩阵累积，绘制普通 SpriteRenderer，包含 Sprite 裁剪、pivot、Pixels Per Unit、颜色、Flip X/Y、启用及对象激活状态、Sorting Layer 索引与 Order。通过现有 GUID 索引定位源纹理与 meta；现代 internalID 与旧 name/fileID 映射均可定位切片。图像后台解码，最多两项同时加载，限制尺寸、分配和缓存总量，纹理及 meta 时间戳变更时重新加载。

Prefab 含非空 Sprite 引用时在 Inspector 标题前出现预览按钮；展开/收起、可拖动分隔线、展开隐藏属性类型列，支持适应窗口、滚轮缩放及拖动平移。选中 SpriteRenderer 时，Transform/RectTransform 折叠组与 Sprite 属性共用滚动区；编辑分别发布对应文档范围，沿用撤销及保存，预览随已确认暂存修改刷新。

限制：尚未展开嵌套 Prefab/variant；仅 Simple 绘制模式，Sliced/Tiled 跳过并提示；不还原自定义 Shader、材质、SpriteMask、SortingGroup 或 Unity 透明排序。RectTransform 使用序列化 Local TRS，不计算 Canvas/anchor 布局；暂不渲染 UI Image/Text。源图支持 PNG/JPEG/TGA/BMP/WebP，无法解码或切片定位失败会计入缺失/不支持提示。

版本保持当前 1.1.2。`cargo check --offline --tests` 与差异空白检查通过，新增变换/循环父引用/切片边界测试仅编译，未执行断言。尚未生成 EXE、运行界面或验证真实工程样本，未提交或发布。后续生成 EXE 必须重新获得明确授权并先检查进程与目标文件占用。

# Unity YAML Viewer 交接：1.1.1 收尾 → 静态图形预览

交接日期：2026-10-10。工程：`C:\Git\UnityYAMLViewer`。

最新构建完成：用户明确授权“可以编译”，程序进程及目标 EXE 独占读写检查通过，`cargo build --release --offline` 成功（9.98 秒）。当前 1.1.2 的 `target/release/unity_yaml_viewer.exe` 已包含撤销数字框左侧对齐及滑轨占满剩余宽度、更新结果弹窗宽度 260 且保留原有 16 / 20 / 10 留白。尚未运行此次构建验证显示效果。下面未构建说明为历史记录。

提示尺寸修正：用户要求保留上下留白，更新结果弹窗恢复 16 / 20 / 10 的上 / 中 / 下间距，仅保留宽度 260 的调整。尚未重新生成 EXE。

最新未构建提示尺寸调整：用户反馈更新结果弹窗过大，将内容宽度从 360 缩至固定 260，上 / 中 / 下留白从 16 / 20 / 10 缩至 10 / 14 / 6，保留文字和 112×34 确定按钮居中，长提示仍换行。尚未重新生成 EXE。

最新未构建微调：撤销步数使用右列内从左至右布局，独立固定 40 宽数值框贴齐右列左边缘，隐藏滑块自带数值框并把剩余宽度全部分配给轨道；数值仍限制 50～1000，并保持设置持久化和编辑会话撤销上限更新。尚未重新生成 EXE。

最新构建完成：用户明确授权“可以编译”，程序进程与目标 EXE 独占读写检查通过，`cargo build --release --offline` 成功（9.29 秒）。1.1.2 的 `target/release/unity_yaml_viewer.exe` 已包含更新结果提示文字 / 确定按钮居中，以及设置右列统一宽度、复选框多语言换行和检查更新按钮独立一行。尚未运行此次构建验证显示效果。下面未构建说明为历史记录。

最新未构建排版调整：更新结果提示文字按整行居中，确定按钮在完整宽度行内显式计算居中矩形，保持 112×34；设置右列关于按钮、语言下拉、主题分段及检查更新按钮统一 208 宽，检查更新按钮单独一行，启动检查复选框使用 Wrap 模式支持多语言。尚未重新生成 EXE。

最新构建完成：用户明确授权“可以进行编译”，Get-Process 进程检查和目标 EXE 独占读写检查通过，`cargo build --release --offline` 成功（9.29 秒）。当前 `target/release/unity_yaml_viewer.exe` 为 1.1.2，包含统一滑块样式、设置多行内容整体居中、更新提示弹窗加宽和确定按钮放大居中。尚未启动此构建进行视觉及更新替换验证；下面未构建说明为历史记录。

最新未构建样式修改：统一普通滑块为 `theme::slider(ui, enabled, slider)`，设置撤销步数、MonoBehaviour Range 属性（含只读禁用状态）、缩放及颜色通道全部复用。轨道灰色提高对比度，启用态蓝色填充 / 描边，禁用态提供独立的清晰滑块颜色；局部作用域避免影响按钮和输入框。删除原有重复样式，保留数值范围和编辑权限。`cargo check --offline` 与差异空白检查通过，未重新生成 EXE，未运行进行视觉验证。

最新未构建界面修改：撤销与更新右侧多行内容通过 `settings_group` 缓存实际组高度，让网格以整组高度和左侧标签垂直居中对齐；高度变化时请求重新布局，兼容语言变化及更新检查状态。检查更新结果提示弹窗最小宽度 360，增加上下留白，“确定”按钮 112×34 并居中。`cargo check --offline` 与差异空白检查通过，尚未生成此次修改的 EXE 或运行验证。

最新构建：用户再次明确要求“进行编译”，Get-Process 进程检查及目标 EXE 独占读写检查均通过，`cargo build --release --offline` 成功（9.01 秒）。`target/release/unity_yaml_viewer.exe` 为 1.1.2，已包含下述设置两列布局及撤销说明间距调整。尚未启动此构建进行界面验证。下述“尚未重新生成 EXE”为此次构建前记录。

最新界面调整（在 1.1.2 构建之后，尚未重新生成 EXE）：设置窗口从 332 加宽至 348，右列从 192 加宽至 208；撤销滑块和说明合并为同一右列单元，间距 4；检查更新加入原有两列设置网格，左列标签对齐，右列纵向放启动自动检查复选框、检查按钮及检查状态。`cargo check --offline` 与差异空白检查通过。重新生成 EXE 仍须用户明确授权和占用检查。

## 最新源码：1.1.2 在线更新（尚未生成 EXE）

后续构建状态（2026-10-10）：用户明确要求“编译”，目标 EXE 独占读写检查通过，CIM 进程查询不可用后改用 Get-Process 补查确认没有 Viewer 进程；`cargo build --release --offline` 成功（38.22 秒）。最新 `target/release/unity_yaml_viewer.exe` 已包含 1.1.2 在线更新及此前调色板修改。尚未启动新版进行交互或替换重启验证。下面“未生成 EXE”描述是本次构建前的历史状态。

用户指定在线更新为 1.1.2 主要更新，Cargo 版本已升至 1.1.2。新增 `src/updater.rs`：GitHub 最新正式 Release 检查、后台下载、大小 / 必须提供的 SHA-256 digest / PE 架构校验，Windows PowerShell 等待当前进程退出后备份、替换及重启，失败尝试恢复并显示错误及备份路径。支持当前发布命名 `UnityYAMLViewer-v版本-windows-x64.exe`，以及通用 `unity_yaml_viewer.exe`；更新仅替换单个 EXE。

设置底部加入默认开启并持久化的启动检查及手动检查。后台发现新版本后设置前显示红色 New，点击显示发行说明；手动检查最新版提示并清除 New。下载进度、取消下载、保存提醒和二次确认已接入；任何编辑会话有未保存修改时禁用确认更新，可取消并保存后再确认。关于界面、七语言文案和 README 已补充 1.1.2 更新记录。

已实读本仓库 Releases API，当前最新 v1.1.1，附件名称与 digest 字段符合要求。最终 `cargo check --offline`、`git diff --check`、翻译七列及重复键检查通过。未生成 EXE、未运行新程序、未实际执行替换重启测试、未发布或提交；现有 1.1.1 EXE 不含更新器，首次升级至 1.1.2 须手动安装。后续生成 EXE 仍须明确授权并先检查程序占用。保留已有未提交编辑修改。

## 最新源码追加：动画颜色调色板（尚未构建）

用户反馈编辑模式的动画 RGBA 色块无法点击编辑。已接入既有 `color_property::show` 调色板，当前时刻 R/G/B/A 均有实际关键帧时可点击，编辑通过 `editing::set_animation_values` 原子批量修改各分量及对应曲线副本，支持撤销与保存。只读模式调色只用于预览；分量关键帧不齐时色块仍只预览，不隐式新增关键帧。`cargo check --offline` 通过，**现有 EXE 尚未包含此次调色板修改**。后续构建需要用户明确授权并检查占用。下面“无待构建修改”是此次追加之前的历史构建记录。

## 最新追加修改及构建状态

2026-10-10，用户再次明确授权，程序进程及 EXE 占用检查通过，`cargo build --release --offline` 成功。版本保持 1.1.1，最新 `target\release\unity_yaml_viewer.exe` 已包含以下追加修改，无待构建的已知源码修改：

- End 标签文字与 35、45 等普通刻度数字处于同一高度，40 / 41 等结束帧均一致；保留文字在色块内居中。
- 动画 RGB / RGBA 预览复用 `src/color_property.rs::paint_swatch`：上方 RGB 色带、底部 4px 黑底白色 Alpha 条，与材质颜色样式一致。只有 A 分量时继续只显示数值，不绘制颜色。

最新 EXE 尚未运行做交互回归，测试断言仍未执行。后续构建依然需要明确授权和占用检查。

## 当前结论

用户准备封存 1.1.1，在新窗口开始「静态图形预览」。用户最新明确要求：版本号保持 1.1.1，不升级到 1.2.0；这条要求覆盖之前的阶段命名及升级建议。静态预览尚未开始实现。

**1.1.1 最终 release 构建已完成。** 用户明确授权后，检查进程及 EXE 占用通过，执行 `cargo build --release --offline` 成功（2026-10-10）。`target\release\unity_yaml_viewer.exe` 已包含最后的独立分量选择 / 拖动修复。尚未运行最终 EXE 做交互回归，也未执行测试断言；没有 Git 提交、标签或发布。

## 必须遵守

- 先阅读工程 `AGENTS.md`，所有修改保持 UTF-8。
- 必须获得用户明确确认才可生成可执行程序。普通修改、封版本、交接请求不自动授权 `cargo build/run/install`、测试程序构建或打包。
- 构建前检查 `unity_yaml_viewer` 进程，以及目标 EXE 的独占文件访问。被占用时告知用户关闭，等待再次明确确认后重新检查。不得自动结束进程或改输出路径绕过占用。
- 可以执行 `cargo check` 或 `cargo check --tests`，但这些只检查编译，不执行测试断言。
- 当前工作区有大量未提交修改及未跟踪文件，包含之前所有工作。不得清理、回滚或覆盖；本轮没有提交、打标签或发布。
- 字体使用系统字体。用户已明确把自定义字体选择器推迟，不属于 1.1.1 收尾。

## 1.1.1 已实现

- Clash Verge 参考深浅主题；浅色 / 深色 / 系统三段切换，正确左右顺序及外侧圆角。
- 简中、繁中、英语、日语、韩语、德语、法语；即时切换，持久化偏好。
- 设置紧凑左右布局，行内垂直居中、标签靠左、控件靠右；撤销步数 50～1000，说明在滑条下。
- 系统字体回退；工具栏短按钮最小宽度；移除搜索框多余外框。
- 编辑模式 Inspector 头部蓝底，放弃红底、保存绿底；Controller State 圆角增加。
- 属性 Inspector 的纵向滚动条靠右；关于历史滚动条靠右。属性名保留层级缩进，类型和值列补偿缩进以对齐。
- 材质依据 Shader Properties 声明区分 Color 与 Vector4；未知类型保持只读。ShaderGraph / 内置 Shader 类型尚未完整解析。
- 关于默认显示最新历史，按钮展开旧版本；1.1.1 中英 README 与七语言历史包括 AnimationClip 时间轴编辑。

## AnimationClip 功能及行为

- 属性分量可编辑；属性编辑 / 曲线编辑双 Tab。
- 帧刻度，每 5 帧较高刻度与数字；长动画按显示密度减少绘制。
- 点击选帧，空白处左键拖动框选，拖动多选关键帧保持相对间距并按帧吸附；Ctrl / Shift 可辅助多选，但直接框选不需要键盘。
- 刻度右键新增关键帧，按现有曲线插值；分割加权曲线段以保持形状。右键删除选中关键帧，支持撤销及保存。
- 曲线切线手柄、切线与权重数值；自动钳制、自动、自由平滑、平坦、断开、左右 / 双侧自由、线性、常量、加权菜单。
- 分量颜色：X/R 红、Y/G 绿、Z/B 蓝、W/A 金色。
- 横向时间缩放、纵向曲线缩放、滚动；缩放滑条轨道灰蓝 / 蓝，5px 高；时间轴与曲线左右各留 12px。
- 时间轴显示整个动画结束点后额外 10 帧；拖动期间冻结横向范围，松手后按新时长继续预留 10 帧。
- 刻度橙色标签 `End 40`，用首字母大写 End，文本居中，色块下移。位置基于整个 AnimationClip，不是当前属性。
- 拖动更新 `m_StopTime`：所有时间字段结合本次移动结果求最大值；从 42 回到 40 时，没有其他更晚帧则 End 回到 40。不会只看当前属性。
- 删除的代码目前不主动重新缩短 `m_StopTime`，不要声称删除也已自动回缩。
- 最后修复：**独立存储的分量不再因同时间自动联选；实际共用运行时向量时间字段的分量仍联动**，避免运行时与编辑曲线不一致。该区别是当前实现限制，不能宣称所有 Transform 向量分量均可独立移动。
- 曾发生点击 m_Alpha 卡死：`data_mut` 回调内调用 `selection_id` 再次锁定 egui Context。已修复为锁外计算 ID。继续修改时避免嵌套 Context 数据锁或 Session 锁。

## 文件导航

- `src/main.rs`：ViewerApp、设置 / 关于、动画解析、帧尺 / End 标签、Inspector 路由。
- `src/timeline_edit.rs`：时间轴选中、框选、拖动、吸附、联动规则、冻结显示范围。最后的独立分量选择修复在此文件，已包含在最终构建中。
- `src/animation_curves.rs`：Unity 关键帧 YAML 块、曲线采样、加帧 / 删除、切线 / 权重、曲线图。
- `src/editing.rs`：编辑 Session、批量时间修改、结构修改、撤销、保存、总时长重算。
- `src/theme.rs`：主题、三段按钮、可见缩放滑条。
- `src/localization.rs`、`src/translations.tsv`：七语言文本。
- `scripts/generate_localized_formats.py` → `src/localized_format.rs`：校验 / 生成格式化翻译。
- `src/script_properties.rs`：脚本及内置属性、材质类型识别、列对齐；`ProjectIndex` 含资源 GUID、Shader 类型索引，可作为预览入口。
- `src/controller_graph.rs`、`src/color_property.rs`、`src/gradient_property.rs`：已有编辑及绘图组件。
- `Cargo.toml`、`Cargo.lock`：1.1.1，Rust 2024，eframe / egui 0.31，rfd、regex、encoding_rs。

## 验证状态与收尾

- 多次 `cargo check --offline` / `cargo check --offline --tests` 和 `git diff --check` 通过。七语言目录最近校验 240 条、33 个格式化模板。
- 新增 Rust 测试源码覆盖选择、不重入锁、移动限制、曲线插值 / 分割、同步修改和撤销；**测试只做过编译检查，未执行断言**。
- 用户运行 EXE 验证过部分界面，并反馈了卡死、滑轨、滚动条、边距、End 更新及分量联选问题。不要把这些等同于全面回归验证。
- 多次 release 编译成功，最新最后一次成功 EXE 在 `target\release\unity_yaml_viewer.exe`；最后的分量联选修复已包含在最终 release 构建中。
- 下一窗口可先验证同帧 X/Y 单独选择、框选移动及 End 更新，再开展静态图形预览。最终构建已完成，无需重复构建；后续生成 EXE 仍需明确授权和占用检查。不要擅自打 Git 标签或发布。
- 工具路径（若 PATH 无 cargo / python）：
  - Cargo：`C:\Users\ekyo\.cargo\bin\cargo.exe`
  - Python：`C:\Users\ekyo\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe`
- 构建曾出现路径 canonicalize 与增量缓存硬链接警告，未阻止成功检查 / release 构建。

## 下一阶段：静态图形预览（版本号保持 1.1.1）

用户只确定「静态图形预览」，尚未确定完整支持范围。以下是建议入口，不是已批准或已完成的需求清单：

1. 先明确首批预览对象：纹理原图、SpriteRenderer 的 Sprite、图集切片，或 Prefab/UI 静态组合。不要直接扩展成完整 Unity 渲染器。
2. 优先复用 ProjectIndex 的资源 GUID → 路径及 `fileID` 解析，定位原图和 `.meta`；评估 Sprite 裁剪矩形、pivot、packed / rotated atlas 的支持。
3. 增加只读预览区域，考虑透明棋盘背景、适应窗口、缩放 / 平移、资源缺失提示；遵循现有主题和七语言框架。
4. 目前 `image` 仅在 build-dependencies 用于图标生成；运行时解码如需要它，须明确增加到 dependencies，并检查离线依赖可用性。
5. 预览缓存按资源与文件变更管理，避免解码 / 扫描阻塞主线程；关注大图内存、纹理释放及 DPI。
6. 用户不希望记录琐碎小修改；后续更新历史按主要能力归纳。按用户最新要求，不升级版本号到 1.2.0；版本号保持 1.1.1。

建议新窗口首条消息：
「请先阅读 C:\Git\UnityYAMLViewer\HANDOFF.md 和 AGENTS.md。1.1.1 最终 release 已构建，接下来讨论并实现静态图形预览。版本号保持 1.1.1，不升级到 1.2.0；后续构建须明确授权并检查程序占用。」
