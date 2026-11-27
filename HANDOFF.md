# 1.3.9 主程序待发布

- 用户已确认发布 1.3.9，将自行提交 Git 后通知。发布标签和显示标题均使用 `v1.3.9`。仓库说明在 [docs/releases/1.3.9.md](docs/releases/1.3.9.md)，供 GitHub 使用的功能更新和修复正文在 `target/releases/v1.3.9/release-notes.md`。
- Windows x64 候选附件为 `target/releases/v1.3.9/UnityYAMLViewer-v1.3.9-windows-x64.exe`，与画中画实测通过的 EXE 逐字节一致。大小 38,572,544 字节，SHA-256 为 `89C05055B50EBA20EAE00FAA7A7B5F0AA5E07A83FFBAA071ED38A424D417A305`。GitHub 附件只上传该 EXE，同目录的 `SHA256SUMS.txt` 用于本地核对。

# 1.3.9 本地测试构建

- 2026-11-26 用户授权后检查进程与目标 EXE 独占均通过，1.3.9 本地 Release 构建成功。文件与产品版本均为 1.3.9，大小 38206464 字节，SHA-256 31252c5c23b03afd59721c8e42625839401e3afb6a4a39f37a2ce4a3279d2897，已同步本地校验文件。包含纯摄像头画内倒计时、画中画录制切换保留画面及录制链路调整。实际设备帧率和首帧耗时待实测。

- 2026-11-26 编译前通过进程及目标 EXE 独占检查，1.3.9 本地 Release 构建成功。文件及产品版本均为 1.3.9，大小 38120960 字节，SHA-256 f0dd9a45fab8079cf4d0db3dfd34ac37d1717ddd01cfc018d0a344a05a71bbff，已同步本地校验文件。包含绘 Tile 窗口闪烁修复、录屏历史缩略图与待机画中画 GPU 叠放。界面和设备待实测。

- 2026-11-26 用户授权编译，进程和目标 EXE 独占检查通过。包含预览布局修复、设备设置分组和绘图快捷键的本地 Release 构建成功。文件及产品版本均为 1.3.9，大小 37908992 字节，SHA-256 0e8ac561c4628f517c42c76295cf50adff131b06e7c340b2110e579028fdc28f，本地校验文件已同步。预览画面、改键和 Wacom 待实测。

- 2026-11-26 用户关闭应用后，进程及目标 EXE 独占检查通过。摄像头预览画布测试版 Release 构建成功。文件和产品版本均为 1.3.9，大小 38022656 字节，SHA-256 b7fd2e48d7f1314b6752698597901b96c67f0c20437cdcef96e32ac16d1a7611，已同步本地校验文件。USB 摄像头、预览窗口及成片画面待实测。

- 2026-11-26 经用户关闭主程序并授权，目标 EXE 独占检查通过，录制控件测试版 Release 构建成功。文件及产品版本均为 1.3.9，大小 37895168 字节，SHA-256 `52cd7037f1fb3a27b96b9b88dcf8aaa554d112e9ef08b21201d9efa49e4ca524`，已同步本地校验文件。日志为 `target/v1.3.9-camera-controls-build.log`。录制方式切换、实时静音及画中画摄像头开关待实测。

- 2026-11-26 经用户授权编译摄像头、麦克风和画中画测试版，进程和目标 EXE 占用检查通过。本地 Release 构建成功，文件及产品版本均为 1.3.9，大小 37905920 字节，SHA-256 `7d5e9af2b24cc551053e56b446ef9c7de824bbea41d2988d87fef39f27ff3111`，已同步本地校验文件。构建日志为 `target/v1.3.9-camera-pip-build.log`。实体 USB 摄像头、麦克风、录制期间移动及捕获排除待实测。

# 1.3.8 正式发布

- GitHub Release 459136897 已于北京时间 2026-11-26 16:33:35 正式发布。标题和标签均为 v1.3.8，标签指向 `0bdf18a04f2d0b8b142aced27f8599f175862f8e`，已核对 Latest。仅上传已实测的 Windows x64 EXE，远端大小和 SHA-256 与本地候选一致。核验记录位于 `target/releases/v1.3.8/published-verification.json`。

# 1.3.8 本地测试构建

- 2026-11-26 经用户授权编译独立 FBX 单面剔除修复测试版，进程和目标 EXE 占用检查通过。本地 Release 构建成功，文件及产品版本均为 1.3.8，大小 37605376 字节，SHA-256 `250de383318cde1e2f3660beede16e1b808af53b1ed6f0bde46715ce4b96f9e9`，已同步本地校验文件。构建日志为 `target/v1.3.8-cull-back-build.log`。

- 2026-11-26 经用户授权编译材质槽对齐测试版，进程与目标 EXE 占用检查通过。本地 Release 构建成功，文件及产品版本均为 1.3.8，大小 37602304 字节，SHA-256 `d74345cc0c7fb50fdbc97a69b9f5c11dcef0079d3e2e550e99a148579fd8d7d7`，已同步本地校验文件。构建日志为 `target/v1.3.8-material-alignment-build.log`。

- 2026-11-26 经用户授权编译独立 FBX 左右对比测试版，进程和 EXE 占用检查通过，本地 Release 构建成功。文件及产品版本为 1.3.8，大小 37602304 字节，SHA-256 `f90d54ef2bbc0d15813735e1a2ab100227a632772067bf960b5742a8b8017b9e`，已同步本地校验文件。日志为 `target/v1.3.8-standalone-fbx-comparison-build.log`。

- 2026-11-26 经用户授权编译材质槽按钮测试版，进程和 EXE 占用检查通过，本地 Release 构建成功。文件及产品版本为 1.3.8，大小 37,586,944 字节，SHA-256 `3b8fedb1c5e078d5223c129c6e5b106f196efb76c8205bd2837077996ea027ce`，已同步本地校验文件。日志为 `target/v1.3.8-material-slot-build.log`。

- 2026-11-26 经用户授权构建独立 FBX 时间轴布局测试版。进程和目标 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。产物 `target/release/unity_yaml_viewer.exe` 的文件和产品版本均为 1.3.8，大小 37,724,160 字节，SHA-256 `89aa55a6effaf70f1df95637e8d83a8752267581c489f658120d2e037880dd94`，已同步本地校验文件。构建日志为 `target/v1.3.8-timeline-layout-build.log`。时间轴布局及滚动操作待实测。

- 2026-11-26 经用户授权构建独立 FBX 关键帧优化测试版。进程和目标 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。产物 `target/release/unity_yaml_viewer.exe` 的文件和产品版本均为 1.3.8，大小 37,763,072 字节，SHA-256 `7814cff981ea6da8d7f5c5e01ceff7cdf4480b9d181b0bcdc690bfd341a0a65c`，已同步本地校验文件。构建日志为 `target/v1.3.8-standalone-fbx-optimization-build.log`。独立动作减键预览及保存待实测。

- 2026-11-26 经用户授权构建独立 FBX 预览测试版。进程和目标 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。产物 `target/release/unity_yaml_viewer.exe` 的文件和产品版本均为 1.3.8，大小 37,494,272 字节，SHA-256 `be8169d350ddac9137d584555b6405c601d26aca1cf20b3f64b2d6e30d1dd216`，已同步本地校验文件。日志为 `target/v1.3.8-standalone-fbx-build.log`。包含同目录自动识别及手动重新扫描，姜饼人的实际贴图、拖放、时间轴和蒙皮待测试。

- 2026-11-26 经用户授权构建 FBX 保存测试版。进程及目标 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。产物 `target/release/unity_yaml_viewer.exe` 的文件和产品版本均为 1.3.8，大小 37,353,984 字节，SHA-256 `a3ded0120fcd5cdb21d734d1dd2314606ad4c98f6f56b067cca1f3c399b1ec4d`，已同步本地校验文件。日志为 `target/v1.3.8-fbx-save-build.log`。用户将测试 children@listen 的保存及 Unity 重新导入后的引用。

- 2026-11-26 经用户授权重编固定取景测试版。进程及 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。产物 `target/release/unity_yaml_viewer.exe` 的文件和产品版本均为 1.3.8，大小 37,283,840 字节，SHA-256 `ff6e459ea2326b22943c2de9d0ea1a6a989fcfa3210f02665791c7a907e76651`，已同步本地校验文件。日志为 `target/v1.3.8-fixed-framing-build.log`。原预览 2D 和 3D 的固定取景及影子表现待实测。

以下为上一轮构建记录。

- 2026-11-26 经用户授权构建。主程序进程检查和目标 EXE 独占访问检查通过，`cargo build --release --locked --offline` 成功。
- 产物为 `target/release/unity_yaml_viewer.exe`，文件及产品版本均为 1.3.8，大小为 37,328,384 字节。SHA-256 为 `3667161ef590395149f9f793ff7d540e5c037acd5f6b99716713906592c84af4`，已同步本地校验文件。构建日志位于 `target/v1.3.8-animation-comparison-build.log`。
- 包含关键帧优化的左右同步对比窗口及单独预览的草稿标签。实际动作和界面待验证。

# 1.3.7 正式发布

- 用户确认文字改色验证通过。发布候选 target/releases/v1.3.7/UnityYAMLViewer-v1.3.7-windows-x64.exe 与已验证 EXE 一致，36,965,376 字节，SHA-256 `ab7c79514b6807f2fa13d56e2527b87ee36bf7bffcb2cb2e068b978d324be629`，PE AMD64。发布说明位于 docs/releases/1.3.7.md。已发布 [v1.3.7](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.7)，标签指向 `db6afaa43fbfc2a4bc37c16af4357c7cb78b0573`，附件哈希与大小及 Latest 已核对。

# 最新PS安装器本地构建：0.1.0

## 2026-11-19 安装器版本比较本地构建完成（未更新公开附件）

- 用户明确授权编译插件安装器；事前检查ps-tile-installer进程未运行且固定目标EXE独占访问通过。cargo build --release --locked --offline 成功，耗时3.51秒，无构建警告。
- 产物 plugins/photoshop-isometric-tile/installer/target/release/ps-tile-installer.exe，文件／产品及插件版本均0.1.0，5262336字节，SHA-256 94313a5daa2e111727889a8c0f93002166cf86291c5c71a78d4a7f730d74d759，完成时间2026-11-19T20:25:47.6402895+08:00。十个内嵌运行文件与当前插件源码逐字节匹配，本地.sha256已更新。
- 包含安装／更新／同版本覆盖安装的比较和提示、阻止较新部署降级、卸载文件计数修正。此前cargo check --tests已通过，Rust用例仅编译未运行；本次未自动运行安装器、未安装／覆盖插件，实际按钮和安装行为待用户验证。
- 未重编主程序，未创建或更新Release、附件及在线版本清单；公开ps-v0.1.0仍为此前安装器。日志target/ps-version-action-release-build.log，事前检查及核验分别为target/ps-version-action-release-preflight.json、target/ps-version-action-release-verification.json。

以下为历史交接记录。

# 最新本地验证构建：1.3.7

## 2026-11-19 最新本地验证构建完成（未发布）

- 用户明确授权，构建前确认主程序未运行且固定EXE独占访问通过。cargo build --release --locked --offline 成功，耗时1分16秒，仅spine38既有警告与VideoSettings::configure未使用警告。
- 产物 target/release/unity_yaml_viewer.exe，文件／产品版本均1.3.7，34075648字节，SHA-256 94e38ffcec400f57bfa8646164ce7cf14ab4543a563302b7cbdfe3789ab9c763，完成时间 2026-11-19T18:51:38.5099027+08:00。251项构建输入前后未变化，本地.sha256同步。未自动启动、未发布。
- 包含统一BaseLayer、原生Tile窗口拖放坐标修正与导入结果、连通同色填充及仅“填充(G)”提示、加减SVG简化、主题Tab及缩放位置调整、Slot选中高亮、1×1快捷与Slot隐藏、设置／素材库／图层标题字号及素材库折叠。保留此前六边形、素材库、Q/W/H和可拖动工具栏等修改。
- 编译成功不代表实机验证通过；素材拖入、填充、标题折叠、Slot状态与工程读写仍待用户运行验证。证据 target/v1.3.7-tile-ui-release-preflight.json、target/v1.3.7-tile-ui-release-build.log、target/v1.3.7-tile-ui-release-verification.json、target/v1.3.7-tile-ui-build-inputs.json。

以下为历史交接记录。

# 最新本地验证构建：1.3.7（2026-11-19 18:13，未发布）

## 2026-11-19 18:13 本地验证构建完成（主程序未发布）

- 用户明确授权编译；事前确认 unity_yaml_viewer 未运行且固定 EXE 独占访问通过。cargo build --release --locked --offline 成功，耗时57.55秒；保留spine38既有警告、VideoSettings::configure及仅测试使用的replace方法未使用警告，无编译错误。
- 产物 target/release/unity_yaml_viewer.exe，文件／产品版本1.3.7，34132992字节，SHA-256 d1014367e80e92069f9dc0efad6b9611b8ae163d38b0d04cedfbaf8442f8bf56，完成时间2026-11-19T18:13:59.3492501+08:00。已核对251项构建输入在编译前后不变，本地.sha256同步；包含当前未提交源码修改，未自动启动。
- 包含矩形／菱形／两种六边形、工程素材库及固定BaseLayer、素材缩略图／外部拖入／右键移除、合并有边界Q／无边界W与回中心H、工具栏拖动位置记忆与边界约束、设置折叠、统一Slot数字编号，以及此前图层交互、PS下载详情等修改。工程格式5兼容读取1–4。
- 仅本地编译，不发布主程序、不更改PS Release或在线清单。真实界面、数位板、文件拖拽、工程保存重开、PS下载及工具栏位置恢复仍待用户验证；构建成功不等于实机全面通过。
- 日志及证据：target/v1.3.7-materials-release-preflight.json、target/v1.3.7-materials-release-build.log、target/v1.3.7-materials-release-verification.json、target/v1.3.7-materials-build-inputs.json。

以下为历史交接记录。

# 最新状态：PS 0.1.0 已发布，主程序 1.3.7 仅本地构建


## 2026-11-19 PS 插件 0.1.0 正式发布

- 用户通知 Git 已提交并授权发布。发布前本地与远程 master 均为 8eb8396；候选安装器与已保存源码输入哈希一致，复用既有授权构建，未重编安装器。
- [GitHub Release](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/ps-v0.1.0)，Release ID 451041187，显示标题 v0.1.0，标签 ps-v0.1.0，正式版、非草稿、非预发布；发布时间 2026-11-19T08:09:04Z。
- 唯一公开附件 PS-Tile-Installer-v0.1.0-windows-x64.exe，5246976 字节，SHA-256 38e75e85e0da9b74010d9e0099a9c945612922207239545e9d9587b645aad490；GitHub 返回大小及 digest 与本地一致。未上传重复校验附件。
- make_latest=false，发布前后主程序 Latest 均为 v1.3.6。附件核验后将 updates/ps-tile.json 改为 published=true 和实际发布地址，通过 GitHub 提交 593b1cfa0ce3519822e8bf23c2d36fe3dbcb9bbe 更新 master，本地已快进同步。
- 主程序实际使用的 raw.githubusercontent.com 清单地址实测 HTTP 200，返回 0.1.0、published=true 及正确发布地址。该读取是本轮发布核验，不是应用自动查询；应用仍仅按钮点击查询。
- 发布、附件与清单证据位于 target/releases/ps-v0.1.0/。插件已发布不代表所有 PS 笔触及宿主全面通过，已有软笔接缝和历史交互限制保留。

## 2026-11-19 主程序 1.3.7 本地验证构建（未发布）

- 用户明确授权编译；构建前确认 unity_yaml_viewer 未运行，固定目标 EXE 独占访问通过。cargo build --release --locked --offline 成功，耗时 2 分 01 秒，仅既有 spine38 与 VideoSettings::configure 警告。
- 产物 target/release/unity_yaml_viewer.exe，文件／产品版本均 1.3.7，33868288 字节，SHA-256 73cdd25ac80d953446c240cc1cce1bfd4fe1987cf24e6db067078e8cd014364d；北京时间 2026-11-19 16:09:23 完成。旁存本地 .sha256，未自动启动。
- 构建源码为 8eb8396，后续 593b1cf 仅更新在线清单，不影响构建输入。已核验在线公开清单能返回 PS 0.1.0；用户需打开像素 Tile → Photoshop 插件 → 检查 PS 插件更新验证实际界面及网络。
- 日志 target/v1.3.7-local-release-build.log，产物核验 target/v1.3.7-local-release-verification.json。主程序 1.3.7 未创建 Release 或上传附件；主程序 Latest 仍为 v1.3.6。

以下为历史交接，状态以本节为准。

# 1.3.7 版本与记录迁移（2026-11-19）

- 主程序 Cargo.toml／Cargo.lock 升至 1.3.7；PS 插件／安装器保持 0.1.0。发布后开发记录迁入 docs/versions/1.3.7.md，1.3.6 保留发布核验与附件清理历史，关于摘要恢复发布核验提交 14127d6 内容。
- 新版关于摘要七语言同步，默认 1.3.7／1.3.6 两版，更早历史重开收起。实际界面仍待运行验证。
- 本轮验证：cargo check --tests --locked --offline 通过（18.59 秒，target/version-1.3.7-check.log），仅保留既有 vendor 警告；未生成或运行测试 EXE。原始迁移内容完整性、索引默认两版／重开收起及全部七语言完整性静态检查通过，git diff --check 通过。实际界面仍待运行验证。
- 本轮不生成可执行程序、不更新 GitHub；下方旧构建和发布记录保留历史事实，不代表 1.3.7 已构建或发布。

# 录屏线框状态颜色（2026-11-16，未重建）

- 选区／倒计时／启动准备保持绿框，收到实际首帧录制通知后切红框，停止或异常结束恢复绿框。`ui.rs` 从 `Job.started` 与 `stopping` 派生状态，`outline.rs` 的目标状态包含颜色标志，原生线框刷新时绘制对应颜色；重新选区显式恢复绿色。
- 捕获排除、尺寸和鼠标穿透沿用现有行为；停止后不会恢复区域鼠标拦截。`cargo check --locked --offline` 通过，日志 `target/recording-frame-color-check.log`；中英文指南、版本记录与七语言关于摘要同步。
- 未生成 EXE、未运行应用、未发布。实际颜色切换、取消／失败、重复录制及捕获排除待运行验证；现有 EXE 仍为下述 16:54 构建，不含本节修改。

# 1.3.6 本地 Release 构建完成（2026-11-16 16:54:09，未更新 GitHub 附件）

- 用户明确授权编译；构建前进程与 EXE 独占访问检查通过。`cargo build --release --locked --offline` 成功，61.71 秒，仅有既有警告。产物 `target/release/unity_yaml_viewer.exe`，文件／产品版本 1.3.6，33,484,800 字节，SHA-256：`07170FAE030A2674F94A2FA141E5EC1DB65AF43BCEBAE0DA35DEBB8B3703CC4F`。
- 包含矢量倒计时居中、外部音轨试听／波形、轨道连续编号、独立字幕预览、“无音轨／无字幕”及工程预览选择保存，连同此前录屏和交互修复。沿用先前编译检查、FFmpeg 解码及几何预览验证；本轮没有运行新应用，实际窗口、高 DPI、切轨／静音／字幕与工程重开仍待运行验证。
- 记录：`target/recording-vector-release-preflight.json`、`target/recording-vector-release-build.log`、`target/recording-vector-release-build-status.json`、`target/recording-vector-release-verification.json`。没有更新 GitHub 附件、历史发布副本或版本号，没有提交／推送；下方未重建描述保留此前修改时点事实，当前本地产物已包含相关调整。

# 倒计时矢量数字居中（2026-11-16，未重建）

- `src/screen_record/countdown.rs` 将字体数字替换为加粗圆润的矢量 3、2、1，以包含笔画粗细的图形边界中心对齐圆心，统一可见高度；不改变计时和原生窗口逻辑。
- `cargo check --locked --offline` 通过（`target/recording-countdown-vector-check.log`）；源码曲线的 3 数字 × 6 缩放共 18 项几何居中检查通过，预览已检查。证据 `target/recording-countdown-vector-geometry.json`，图片 `target/recording-countdown-vector-preview.png`；实际应用窗口、高 DPI／混合缩放与录制操作待验证。
- 中英文指南、版本记录与七语言关于摘要同步。未生成 EXE、未运行应用、未发布；现有 EXE 仍为下述 16:00 构建，不包含本节及后续编辑轨道预览修改。

# 编辑轨道预览调整（2026-11-16，未重建）

- 外部音轨接入波形／试听，下拉列表与右侧“音轨1、音轨2……”统一编号。音轨下方增加字幕预览下拉菜单；两者始终提供“无音轨／无字幕”，关闭预览不改变导出勾选或 ASS 烧录。
- 编辑字幕默认关闭，不再继承播放页会话、不自动扫描外挂或按语言启用。内嵌／外部文字字幕显式选择后按绝对流编号提取；图形字幕预览不支持，原画面压制字幕不可隐藏。工程兼容保存／恢复预览来源与关闭状态，路径重定位沿用既有轨道引用。
- 新模块 `src/media_preview/edit_audio.rs` 保存来源身份及菜单构造；编辑入口、轨道列表、字幕会话与工程序列化相应更新。`cargo check --tests --locked --offline` 通过，新增 5 项 Rust 回归用例仅编译；FFmpeg 外部音轨、静音尾部、内嵌／外部字幕流提取验证通过，证据 `target/editor-preview-tracks-verification.json`。
- 中英文指南／README、七语言关于摘要和菜单文字同步。未编译 EXE、未运行新应用、未发布；实际下拉菜单、字幕消失、切轨／静音和工程重开仍需运行验证。现有 EXE 仍为下述 16:00 构建，不含本节更改。

# 1.3.6 本地 Release 构建完成（2026-11-16 16:00:48，未更新 GitHub 附件）

- 用户授权编译，构建前进程与目标 EXE 独占访问检查通过；`cargo build --release --locked --offline` 成功，约 61 秒，仅有既有警告。产物 `target/release/unity_yaml_viewer.exe`，文件／产品版本 1.3.6，33,397,248 字节，SHA-256：`D384B8AFFE934CAEA2609029B67C4E12EC7B9CF682DFA67CF429E03E514A960B`。
- 包含录屏进入编辑无声修复、视频音轨波形／切换、时间戳对齐与尾部静音、录屏结束后的绿框穿透、字幕框收窄及此前倒计时修正。此前编译检查及 FFmpeg 音频核验通过，实际界面与扬声器试听待验证；未自动启动应用。
- 日志／证据：`target/video-edit-audio-release-build.log`、`target/video-edit-audio-release-preflight.json`、`target/video-edit-audio-release-verification.json`。未替换 GitHub 附件或历史发布副本，没有提交／推送；下方“未重建”保留先前源码修改时点事实。

# 视频编辑音轨与录屏试听（2026-11-16，未重建）

- 修复录屏结果／历史直接进入编辑漏选音轨导致无声；与编辑列表添加文件共用默认音轨初始化。视频时间轴加入内嵌音轨波形，左侧多轨切换同步试听与波形并保留位置和范围；无音轨保留原布局。导出音轨勾选及外部添加音轨不受预览切换影响，外部音轨尚不参与试听。
- 波形／视频编辑试听按时间戳对齐，短音轨尾部试听补静音并限制当前范围；解码错误记录 error.log。cargo check --locked --offline 通过；FFmpeg 多音轨、短／长／延迟／跳到音频结束后静音及用户录屏非零 PCM 核验通过，target/video-edit-audio-verification.json。实际界面与扬声器试听待构建验证。
- 中英文指南、README 能力概览、七语言关于摘要及版本记录同步；每语言 11 条、默认最近两版、重开收起配置核对通过。cargo check --tests --locked --offline 及 git diff --check 通过（target/video-edit-audio-tests-check.log），未运行 Rust 测试。没有生成新 EXE、没有替换 GitHub 附件；现有本地 EXE 仍为倒计时构建，不含本次、绿框穿透及字幕框收窄。

# 录屏结束后绿框保持鼠标穿透（2026-11-16，未重建）

- src/screen_record/ui.rs 独立保存线框交互状态：录屏任务结束即关闭线框拖动／缩放，只有再次进入选区才恢复；复用原生穿透样式并隐藏内部拖动层。工具条坐标输入及结果操作继续可用，修改坐标不恢复鼠标拦截。
- 中英文使用指南及七语言关于摘要同步；每语言 11 条、默认两版及重开收起配置已核对。`cargo check --locked --offline` 与 `git diff --check` 通过，日志 `target/recording-outline-passthrough-check.log`。实际底层软件鼠标操作、重新选区／移动／缩放待新构建验证。未生成 EXE、未更新 GitHub 附件；当前本地 EXE 仍是下述倒计时构建，不包含本次及字幕选择框宽度调整。

# 播放页字幕选择框收窄（2026-11-16，未重建）

- 用户截图指向视频信息行的字幕选择框；src/subtitle_player.rs 将最大宽度 180 改为 140 个逻辑像素，保留长名称省略／悬停全文及展开菜单最大 280。七语言关于摘要同步，索引配置不变。
- 本轮不生成 EXE，实际布局待新构建验证。当前本地 EXE 仍为下述倒计时修复构建，不包含本次宽度调整；GitHub 附件未改变。
- `cargo check --locked --offline` 与 `git diff --check` 通过；检查日志为 `target/subtitle-selector-140-check.log`。

# 1.3.6 倒计时修复已本地重建（2026-11-16，未更新 GitHub 附件）

- 用户明确授权编译；构建及测试前均确认程序未运行、目标 EXE 独占访问通过。cargo build --release --locked --offline 成功（72.50 秒）；target/release/unity_yaml_viewer.exe，文件／产品版本 1.3.6，33,551,872 字节，SHA-256：A4099A10E3660CFC6C68CE459959008C3D829EE73FB063928B5A73A427D6F111。包含透明窗口初始化、显示就绪后开始三秒准备和错误保留修正，仅保留既有警告。
- 新增倒计时针对性测试 2 项通过，日志 target/recording-countdown-visibility-tests.log；构建日志 target/recording-countdown-visibility-release-build.log，核验 target/recording-countdown-visibility-release-verification.json。未自动启动 GUI，实际数字、混合 DPI／多屏／全屏与关于窗口仍待运行验证。
- 本轮没有更新 GitHub 发布附件、target/releases/v1.3.6 历史副本或版本号，也未提交／推送。下方“未重建／测试仅编译”保留此前源码修改时点事实。

# 1.3.6 发布后倒计时源码修正（2026-11-16，当时未重建）

- 按用户最新要求修复中央数字不显示：winit 的鼠标穿透添加 WS_EX_LAYERED，原流程未初始化分层透明属性；现在在显示前补上 Alpha 255／LWA_ALPHA，保留鼠标穿透与捕获排除。倒计时从数字窗口就绪后开始，窗口准备阶段可取消，5 秒仍未就绪则报错，并记录 countdown.waiting／countdown.shown。
- cargo check --tests --locked --offline 通过，日志 target/recording-countdown-visibility-check.log；新增两项计时／取消回归用例仅编译。隐藏 Win32 窗口探针验证初始化和穿透／捕获排除兼容性通过，证据 target/recording-countdown-layered-probe.json；不等于实际应用视觉验证。
- 七语言关于摘要与错误提示、中英文指南同步；关于默认两版和重开收起配置不变。未生成新 EXE、未运行 Rust 测试或新应用 GUI、未发布；现有 EXE 与 GitHub 附件仍是原版本，后续需获得编译授权并检查程序占用后重建。实际数字、全屏／混合 DPI／多屏负坐标及关于窗口交互待验证。

# 当前发布：1.3.6（已正式发布）

- 2026-11-16，按用户反馈移除 GitHub v1.3.6 多余的 SHA256SUMS.txt 附件；EXE 原附件、正文、标签和 Latest／正式版状态均未变。GitHub 自带 SHA-256，后续不再上传重复校验附件；本地校验文件及大小／哈希核验继续保留，AGENTS.md 已补充约定。核验 target/v1.3.6-checksum-removal-verification.json。本轮仅清理该远程附件并回填本地记录，未编译或修改发布正文；以下双附件上传记录为首次发布时的历史事实。

- 用户明确授权发布到 GitHub。北京时间 2026-11-12 22:16:43 正式发布 v1.3.6，Release ID 443041359，显示标题仅 v1.3.6，非草稿、非预发布，latest API 已确认为最新稳定版。标签指向 3ae9149ebc048883dbd5fb1e7ef9cfeed120f39e；该提交相对构建基准仅增加发布资料，313 项构建输入字节一致，无需重新编译。
- 已上传 UnityYAMLViewer-v1.3.6-windows-x64.exe（33,457,664 字节）及 SHA256SUMS.txt；GitHub 返回的两个附件大小与 SHA-256 均与本地一致。EXE SHA-256：CF0392D497503E8158C6A425C595E09EDDCBD6F57543EACF6A07D864F54C326D。
- 发布地址：[v1.3.6](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.6)。公开正文使用已准备的更新内容和已知限制，中央大数字倒计时未显示仍明确保留。
- 发布前本地／远程 master 一致且工作区干净；用户已提交发布准备资料。未重新编译，未改动 v1.3.5 或其他历史 Release。发布核验：target/v1.3.6-publish-audit.json、target/v1.3.6-upload-verification.json、target/v1.3.6-published.json、target/v1.3.6-published-verification.json。
- 本轮发布后的状态回填仅修改本地文档，未自动提交／推送；下方准备、迁移及旧版本记录保留当时时点事实。运行验证范围仍以用户已确认的操作及既有测试记录为准。

# 1.3.6 正式版发布准备完成（尚未正式发布）

- 2026-11-12，用户明确授权编译及准备正式版。测试／构建前均确认程序未运行且目标 EXE 独占访问通过；Windows x64 Release 离线构建成功（69.28 秒），文件／产品版本均为 1.3.6；33,457,664 字节，SHA-256：CF0392D497503E8158C6A425C595E09EDDCBD6F57543EACF6A07D864F54C326D。未自动启动程序。
- 主程序 target/release/unity_yaml_viewer.exe；候选 target/releases/v1.3.6/UnityYAMLViewer-v1.3.6-windows-x64.exe。已核对二者哈希一致、PE 为 AMD64，文件／产品版本为 1.3.6；同目录 SHA256SUMS.txt 和 release-notes.md 已准备。
- 录屏 PCM 2 项、共用错误日志 1 项针对性测试全部通过。日志 target/v1.3.6-pcm-tests.log、target/v1.3.6-error-log-tests.log；构建日志 target/v1.3.6-release-build.log，核验 target/v1.3.6-release-verification.json。仅保留 spine38 既有 158 项及 VideoSettings::configure 未使用警告。
- 构建源码提交 2e56d71875afb7d6beb15468487afa1422e7ab5f；313 项构建输入快照 target/v1.3.6-prepared-source-manifest.json 已保存且构建前后相同。后续提交发布资料后仍需按快照核对；快照不作为编译器来源证明。
- 发布说明 docs/releases/1.3.6.md，版本状态与索引已同步。公开正文仅包含更新内容和使用限制；Release 显示标题仅 v1.3.6，计划为正式稳定版。
- 用户已确认录屏修复与已体验操作通过；中央大数字倒计时未看到，按要求暂不调整并在发布说明披露。没有把测试／构建成功当作所有设备或新 GUI 全面验证。
- 尚未提交本轮文档／推送、创建标签或 GitHub Release，也未上传附件。当前正式发布记录仍为 v1.3.5；不得仅因候选构建成功改为“已正式发布”。以下交接保留各自时点事实。

# 1.3.6 版本迁移交接（当时尚未构建此版号）

- 2026-11-12，按用户要求将 Cargo.toml／Cargo.lock 升至 1.3.6，并将 v1.3.5 标签之后的开发记录迁入 docs/versions/1.3.6.md。1.3.5 版本记录以 7089dc2 的发布核验补记为基线，关于摘要恢复 v1.3.5 标签内容；正式发布资料、标签、附件和哈希不变。
- 1.3.6 包含字幕生成／实时识别／翻译／缓存／独立编辑、字幕样式与导出优化、GPU 播放和图片预览改进、Windows 录屏与共用 error.log。最终范围及暂缓事项见版本记录；Moonshine／OPUS-MT 仅保留历史开发事实，已不提供入口。
- 用户确认录屏错误修复、其他已体验操作验证通过；中央大数字倒计时没有看到，按要求暂不调整。保留新构建关于界面及未覆盖环境的待验证状态，不宣称全部功能全面回归。
- 最近本地 EXE 仍为升级前开发版 1.3.5：target/release/unity_yaml_viewer.exe，33,502,720 字节，SHA-256：F4739FB0AC34BD4D4A36B588CD60AD354D67E29CC4D91B56A1669CC3AB21A52F。对应 target/recording-pcm-countdown-timeline-release-verification.json；它与 v1.3.5 正式发布附件不同。本轮不生成新 EXE，不提交／推送、打标签或发布。
- 关于窗口默认两版为 1.3.6／1.3.5，更早历史默认收起；新摘要具备七语言。下方旧交接、版号与各时点验证结论保留，最新状态以本节和 1.3.6 版本记录为准。
- 本轮 cargo check --tests --locked --offline 通过（18.64 秒，target/version-1.3.6-check.log）；未运行测试程序。迁移及七语言静态核对通过，28 版索引与默认两版／重开收起逻辑保持，发布后记录完整保留，1.3.5 发布资料与其他历史摘要未改变，原 EXE 哈希不变；证据 target/version-1.3.6-migration/verification.json。git diff --check 通过；新版本关于窗口的实际显示与交互待运行验证。

# 当前发布：1.3.5（已正式发布）

- 正式发布：用户确认 Git 已提交并明确授权发布；本地与远程 master 均为 3e192b9559222301dbd94d7187d7008300264574，258 项准备构建输入逐字节未改变，附件哈希与最新已核验产物一致，未重新编译。北京时间 2026-11-06 12:35:09 发布 v1.3.5，Release ID 436481768，显示标题仅 v1.3.5，非草稿、非预发布，latest API 确认为最新稳定版，标签指向已核对。附件 UnityYAMLViewer-v1.3.5-windows-x64.exe，31095296 字节，SHA-256：B85D5494321C768F78F2BA064010D860DA05E57A2C5531D32EAD93E667202CD3，GitHub 返回大小及 digest 核验通过。证据 target/v1.3.5-published-verification.json、target/v1.3.5-published.json、target/v1.3.5-release-asset.json。下文未发布、需重建及旧产物记录保留当时时点事实；不扩大实际运行验证范围。

# 1.3.5 最新发布候选已重建（未正式发布）

- 2026-11-06，索引式关于历史改造完成本地构建：用户明确授权，编译前确认程序未运行、目标 EXE 独占访问通过。cargo build --release --locked --offline 成功（53.64 秒），文件／产品版本均为 1.3.5，31095296 字节，SHA-256：B85D5494321C768F78F2BA064010D860DA05E57A2C5531D32EAD93E667202CD3。包含 index.json 控制版本顺序、默认显示数量及重开时折叠状态，以及 27 版七语言历史文档；仅保留 spine38 158 项既有警告。日志 target/v1.3.5-history-index-release-build.log，核验 target/v1.3.5-history-index-release-verification.json；target/releases/v1.3.5 下候选 EXE 与 SHA256SUMS.txt 已同步，附件副本哈希一致。258 项输入快照 target/v1.3.5-prepared-source-manifest.json 已更新，仅供提交后比较，不是编译器来源证明。未自动运行、未提交或发布；关于窗口实际语言切换／展开／重开仍待 GUI 验证。以下未重建及旧产物描述保留当时时点事实。

# 1.3.5 候选需要重建

- 索引方案最终验证：index.json 按明确顺序列出 27 版，默认显示前 2 版且每次打开收起旧历史；当前首两版为 1.3.5／1.3.4。索引加载及校验的 cargo check --tests --locked --offline 通过（13.16 秒，target/about-history-files-check.log），按索引再次核对 1,323 条七语言文本与迁移前完全一致。未生成新的 EXE，实际 GUI 待验证。

- 文档驱动历史验证：cargo check --tests --locked --offline 通过（target/about-history-files-check.log），仅保留 spine38 158 项及 egui_commonmark 1 项既有警告；git diff --check 通过。迁移基线 target/about-history-migration-baseline.json 与 27 份文档逐项比较，1,323 条七语言文本完全一致。未执行测试或生成新的 EXE；原候选需重建，GUI 切换／折叠待验证。

- 2026-11-06，关于更新历史改为文档驱动：将 main.rs 中 27 个版本的摘要及通用翻译表中对应的 189 行翻译迁入 docs/about-history/<版本号>.md，每文件包含七语言小节；逐项比对 1,323 条语言文本与原内容一致。index.json 统一管理版本顺序、默认显示数量及重开时的展开状态；build.rs 按索引校验并加载，索引首版必须匹配 Cargo.toml。界面只保留统一显示逻辑，语言切换即时选择对应小节，默认显示当前及上一版，其余历史统一折叠，重开恢复折叠的逻辑保留。格式与维护规则写入目录 README 及 AGENTS.md。不解析任意 Markdown／HTML，也不依赖外部文件随 EXE 分发。此前 1.3.5 EXE 不含本次修改，原发布候选需重新编译后替换；七语言切换及关于折叠实际 GUI 仍待验证。

# 1.3.5 发布准备完成（尚未正式发布）

- 2026-11-06，用户明确授权编译及准备发布。构建前确认程序未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功（47.96 秒），文件／产品版本 1.3.5，31,088,640 字节，SHA-256：5B1E37F38AC4A09F5D3C152961592A3870FF55407D43A6C4EBC0051041A2C6F4。日志 target/v1.3.5-release-build.log，核验 target/v1.3.5-release-verification.json；附件副本 target/releases/v1.3.5/UnityYAMLViewer-v1.3.5-windows-x64.exe 与原产物哈希一致。发布说明 docs/releases/1.3.5.md 及索引已整理；未自动运行、未提交／推送／打标签或创建 GitHub Release。保留 spine38 158 项既有警告，实际界面待用户验证；此前未构建描述保留当时状态。

# 当前开发：1.3.5（未正式发布）

- 2026-11-06，版本升为 1.3.5，发布后记录迁入 docs/versions/1.3.5.md；同步索引、CHANGELOG 及关于七语言摘要顺序。历史产物信息保留。本轮不生成 EXE，最近本地产物仍为 1.3.4；关于界面待验证。

# 当前发布：1.3.4（已正式发布）

- 2026-11-06，小卡片本地构建完成：用户明确授权，测试及 Release 前均确认程序未运行、目标独占访问通过；卡片列数/对齐回归测试 1 项通过。Release 离线构建成功（45.04 秒），仅 spine38 既有警告。产物 target/release/unity_yaml_viewer.exe，版本 1.3.4，31,085,056 字节，SHA-256：9E9153A3EACF9C4FD864EF500526B0FA014D92F279BE715E581F718CBD5C1193。日志 target/iptv-small-cards-tests.log、target/iptv-small-cards-release-build.log，核验 target/iptv-small-cards-release-verification.json。已包含 76×70 卡片，实际默认三列待用户验证；未自动运行、未更新发布附件。下文未重建为此前时点记录。

- 2026-11-06，卡片缩小源码已完成，未重建：固定 76×70 点、间距 8 点，244 点内容宽可排 3 列，台标区域 64×38 点，保留默认侧栏宽度及动态列数。边界用例同步，cargo check --tests --locked --offline 通过（target/iptv-small-cards-check.log），GUI 待验证。当前 EXE 仍为此前 9606E15E… 产物，不含本次缩小。

- 2026-11-06，平铺分组/侧栏消失/电视快捷键修正版本地构建完成：用户明确授权，测试与 Release 编译前均确认程序未运行、目标 EXE 可独占访问。22 个不同回归用例全部通过（23 次执行，合组用例复核一次）；包含读取中及源报错后分组可见的无界面渲染检查、固定卡片网格边界与对齐、按钮焦点下方向键切台、Ctrl 切源循环/单源无效，以及原列表/参数/保存/台标/网速用例。cargo build --release --locked --offline 成功（45.15 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，31,085,056 字节，SHA-256：9606E15E11E63EBD5F83A3A16727F4CB88B81155AC23BF75239C423977E6F4AC。日志 target/iptv-flat-playlist-tests.log、target/iptv-flat-keyboard-tests.log、target/iptv-flat-rate-tests.log、target/iptv-flat-release-build.log；核验 target/iptv-flat-release-verification.json。已包含全宽 30 点平铺 Group 条、对齐卡片、加载/错误侧栏保留、56×28 居中源按钮和电视键盘操作。未自动启动应用；真实源加载失败/成功切换、缩放/全屏、间距和键盘体验仍待用户 GUI 验证。未更新 GitHub 附件或历史候选副本，下列未重建及旧产物描述仅代表当时时点。

- 2026-11-06，用户测试后布局/切台修正已改源码，未重建：Group 改为 30 点高、侧栏全宽、文本居中的平铺折叠条，取消树缩进；88×82 卡片按可用全宽算列，绝对网格位置绘制避免错行。修复加载/错误时剩余区域被消费导致侧栏消失，保留分组和滚动以便继续换台；源按钮 56×28 与全屏同高、居中、紧凑排列。电视台上下调音量、左右切频道、Ctrl+左右循环切源，单源无效，加载和错误阶段也可操作，文本/菜单不抢键。新增几何、加载/错误侧栏可见和循环边界用例仅编译未运行，cargo check --tests --locked --offline 通过，日志 target/iptv-flat-layout-check.log。七语言/双语文档同步；实际 GUI 待验证。本轮未生成新 EXE，当前产物仍为下条 A15FF5B1… 构建，包含用户反馈的旧布局问题，需重新构建后验证。

- 2026-11-06，电视／电台分组、台标缓存、多源、观看记录、另存为及网速显示已完成本地构建：用户明确授权，测试与 Release 编译前均检查程序未运行、目标 EXE 可独占访问。19 个不同回归用例全部通过（20 次执行，URL 来源键用例复核一次），覆盖原列表兼容、IPTV 参数名称、分组/多源/元数据与头往返、另存为相对路径、频道源恢复及旧记录、台标解码/缓存键/离屏请求名额、网速统计及显示比例。cargo build --release --locked --offline 成功（49.47 秒），仅保留 spine38 既有 158 项警告。最新产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，31,155,200 字节，SHA-256：A15FF5B1A434C3C157D0FB69FF999C2CAB16F377D43B3B1A8654F2176FB4AFC1。日志 target/iptv-release-playlist-tests.log、target/iptv-release-rate-tests.log、target/iptv-release-origin-tests.log、target/iptv-release-aspect-tests.log、target/iptv-groups-release-build.log；核验 target/iptv-groups-release-verification.json。未自动启动应用；真实 result.m3u、折叠/卡片/源菜单布局、台标落盘刷新及清理、观看记录跨重启、另存为对话框、窄窗口/全屏/七语言和关于窗口交互待用户验证。未更新 GitHub 附件或 target/releases 历史候选副本；以下未重建/用例仅编译均为此前时点记录。

- 2026-11-06，电视／电台列表新视图、源菜单、台标缓存、记忆及另存为源码已完成，未重建：用户 result.m3u 只读分析为 839 条、322 个不同 ID；按 group-title 折叠分组，默认及重开全部折叠，固定宽度台标卡片自动换行，同 ID 合源，网速/源菜单紧邻全屏左侧，频道视图隐藏编号名称和片头片尾。台标按需后台加载，内存/磁盘缓存有界，支持过期刷新与设置清理；记住最近频道和源，URL 列表按原地址散列跨下载缓存恢复新记录。另存为保留元数据、头和全部源，处理本地相对路径、覆盖确认/外部变更检查，保存副本并提供打开入口。细节、限制和验证见 docs/versions/1.3.4.md。cargo check --tests --locked --offline 通过，新增用例只编译未运行；真实分组/台标/源切换、持久记录、另存为和 GUI 待验证。新增模块 src/playlist/iptv.rs 与此前 src/media_preview/receive_rate.rs 均需随源码纳入版本管理；未自动提交、编译 EXE 或发布。当前可执行文件仍为此前 RTP/RTSP/比例构建，不含此后 IPTV 名称、网速及本轮功能。

- 2026-11-06，网络播放接收速率源码已完成、未重建：底部音量与时间之间显示编码媒体输入的 3 秒平均速率（KB/s、MB/s），按音视频接收进程合计；FFmpeg 包长诊断仅内存消费，日志行和统计桶有界，不统计解码输出/整机网卡流量，不含协议封装开销。本地媒体/缓存播放不显示，换源重置、断流归零；窗口/全屏共用。用户期望用于比较频道，不能单凭该值断定卡顿原因，本次未宣称修复画面/声音冻结。细节及边界见 docs/versions/1.3.4.md。cargo check --tests --locked --offline 通过，日志 target/network-receive-rate-check.log；新增用例仅编译未运行，真实速率和 UI 待验证。七语言与双语文档同步；当前 EXE 仍为此前 RTP/RTSP/比例构建，既不含本次速率，也不含上一轮 IPTV 参数名称修正，未重新构建或发布。

- 2026-11-06，IPTV 频道名称参数修正源码已完成，未重建：兼容 `#EXTINF:-1,tvg-id=... tvg-logo=...,频道名`，显示参数区后的名称，忽略属性，不使用 ID/别名代替末尾名称、不加载台标；分隔符跳过引号内逗号，保留标准格式与普通名称中的逗号、原媒体 URL。新增针对性用例仅编译未运行，cargo check --tests --locked --offline 通过，日志 target/iptv-channel-name-check.log。双语使用指南、版本记录、关于七语言同步，GUI 待验证。用户亦确认其他频道长时间观看没有中断或静止，仅记录反馈范围。当前 target/release/unity_yaml_viewer.exe 仍为此前 RTP/RTSP/比例构建，不含本次名称解析修正；未构建或发布。

- 2026-11-06，直播播放一段后静止问题已针对用户给定 cztv channel012 地址核验：跨分片周期及随机查询参数请求均 HTTP 200，返回相同序号 360252198 与 3×8 秒 TS，节目时间仍为 2025-12-30，响应时间为当前日期。该入口当时未更新，可解释重选也只重复约 24 秒；不把重连/循环旧片段当作修复。证据和边界见 docs/versions/1.3.4.md，target/live-stop-*-verification.json 及 manifest 样本。本次仅核验源站和补记录，没有修改播放器或重建；当前 EXE 仍为下条产物。

- 2026-11-06，RTP/RTSP/IPv6、名称及显示比例本地构建完成：用户明确授权，测试和 Release 构建前均检查程序未运行、目标 EXE 可独占访问。13 项针对性回归测试全部通过（10 项播放列表相关用例、协议参数与 IPv6、缓存名一次解码、比例几何与自由缩放/平移保留各 1 项）。cargo build --release --locked --offline 成功（1 分 10 秒），仅保留 spine38 既有 158 项警告。最新产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，30,945,792 字节，SHA-256：32D1C123A41D89772D95210A2FA2E9F56C83C81F3427D800A2D4FA2E2F6E5CD2。日志 target/rtp-rtsp-playlist-tests.log、target/rtp-rtsp-protocols-tests.log、target/rtp-rtsp-names-tests.log、target/rtp-rtsp-aspect-tests.log、target/rtp-rtsp-release-build.log；核验 target/rtp-rtsp-release-verification.json。最终工具栏为缩放 Tabs → 比例状态 → 镜像状态 → 缩放值，右键“比例 → 原始 / 16:9 / 4:3”。未自动启动程序，真实 RTP/RTSP/IPv6 信号、总超时、中文显示/左对齐、比例右键菜单和关于窗口交互待用户验证。未更新 GitHub 附件或 target/releases 历史候选副本。此前有关缓存的答复为现有行为说明：URL 加载的列表文件落盘，列表记录设置/条目和本地媒体进度，网络频道只有临时缓冲，不生成离线媒体缓存。以下未重建/仅编译未运行均为此前时点记录。

- 2026-11-06，RTP/RTSP/IPv6、无效条目隔离、缓存名称显示和画面比例源码验证已完成，尚未重建：列表接受 RTP/RTSP 标准 IPv4/IPv6 地址，按协议传递 FFmpeg 参数，媒体信息探测增加约 20 秒总超时；无效网络条目独立显示错误并保留保存，不再拖垮整份列表。页签/列表标题显示一次解码后的中文缓存名，名称行左对齐。右键“比例 → 原始 / 16:9 / 4:3”，显示比例与已有缩放/平移叠加，工具栏按解码、三个缩放选项、比例状态、镜像状态、百分比排列。双语文档、当前关于与提示七语言同步；细节、限制、用户截图反馈和真实地址试探结果见 docs/versions/1.3.4.md。RTP 18 秒未取得信息，RTSP 与 kankanlive 本机试探连接失败，未确认收到直播信号；本轮仅修改和编译检查，不生成应用 EXE。当前 target/release/unity_yaml_viewer.exe 仍为下条网络列表/Prev 构建，不含这些新修改。

- 2026-11-06，网络频道播放列表及 Prev 已完成本地构建：用户明确授权，测试和 Release 构建前均检查程序未运行、目标 EXE 可独占访问。9 项播放列表相关回归测试全部通过，覆盖网络频道名/混合路径/签名 URL 保存往返、非法协议和 HLS 分片清单拒绝、本地列表兼容、M3U MIME 分流、网络预览与 Prev 事件。cargo build --release --locked --offline 成功（46.05 秒），仅保留 spine38 的 158 项既有警告。最新产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，30,946,816 字节，SHA-256：34AF42A551CADEC5FE282AFEE7306A0D0C2722EED6BA2396FA3CE9B90188F187。日志 target/network-playlist-tests.log、target/network-playlist-release-build.log；核验 target/network-playlist-release-verification.json。已包含网络频道列表、Prev 及未知时长禁用进度拖动/片头片尾设置。未自动启动程序，真实频道连接及 GUI 待用户验证；未更新 GitHub 附件或 target/releases 历史候选副本。以下“尚未重建/仅编译未运行”为此前时点记录。

- 2026-11-06，网络频道播放列表及 Prev 源码已修改，尚未重建：M3U/M3U8 频道名称与 HTTP(S) 地址接入既有列表播放器，保留签名 URL，首次默认显示名称，网络条目不检查本地文件是否存在，切换/重开重新连接。`.m3u` URL 下载保留扩展名，避免 TXT 或单个 HLS 误识别；列表内允许未知时长网络流，禁用进度拖动与所有片头片尾控件、不执行旧跳过设置。Prev 按列表顺序返回、首项禁用；其他行为、限制及验证详见 docs/versions/1.3.4.md。README 双语、使用指南和关于七语言已同步。cargo check --tests --locked --offline 通过，日志 target/network-playlist-check.log；新增回归用例仅编译未运行，实际频道和 GUI 待验证。未运行构建/打包命令、未更新发布附件；当前 target/release/unity_yaml_viewer.exe 仍为下条所列 YouTube 修正构建，不含本轮播放列表修改。

- 2026-11-06，YouTube 错误提示及运行时查找修正完成本地构建：用户明确授权，测试与 Release 构建前均检查程序未运行、目标 EXE 可独占访问。1 项组合错误回归测试通过（两种撇号、登录验证与缺失运行时同时提示、普通 HTTP 错误不误报）。cargo build --release --locked --offline 成功（54.36 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，30,884,352 字节，SHA-256：EBD9F9D83F433FBDB94BEA0443DA40440A60473FAFA05D24D9207EE7FBFA1F97。日志 target/youtube-runtime-tests.log、target/youtube-runtime-release-build.log；核验 target/youtube-runtime-release-verification.json。未自动启动程序，实际 YouTube 提取、运行时发现和关于窗口待验证。用户此前截图确认旧 EXE 能获取并显示 4K 视频和独立音轨，不计作本次修正验证。未更新 GitHub Release、附件或 target/releases 历史候选副本；下列发布信息仍对应此前发布产物。

- 正式发布：用户提交 Git 后明确授权发布，本地与远程 master 均为 56cac36cc56f303f663657d100105ad4e84a75fe。225 项准备时构建输入未改变，提交内容无实质差异（156 项仅换行不同）。未重新编译；上传 30,860,288 字节附件并核验 GitHub SHA-256 为 A3CCBCC9D9309CCD40237E9DAFF03636E617F397687B9133887FA992DA4B7F45，于北京时间 2026-11-06 11:25:21 正式发布 v1.3.4（Release ID 436065305）。标题仅 v1.3.4，非草稿、非预发布，latest API 确认为最新稳定版；标签指向已核对。证据：target/v1.3.4-publish-audit.json、target/v1.3.4-release-asset.json、target/v1.3.4-published.json、target/v1.3.4-published-verification.json。以下未发布、待提交或待验证描述保留开发阶段史实；测试通过的范围仍以用户实际反馈为准。

- 发布地址：[v1.3.4](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.4)。发布后的状态记录仅更新本地文档，未自动提交或推送。Greasy Fork 在线脚本未在本轮更新。

以下保留发布前交接与开发历史，相关未发布状态仅代表当时时点。

# 1.3.4 发布准备交接

- 状态：文档与本地候选已准备，用户自行提交 Git 后另行通知发布。尚未创建 v1.3.4 标签、GitHub Release 或上传附件；本轮未提交或推送。
- 发布说明：[docs/releases/1.3.4.md](docs/releases/1.3.4.md)。公开正文采用“更新内容”“安装嗅探助手”“使用限制”，不复制发布准备与验证交接段。Release 显示标题仅 v1.3.4。
- 候选：target/releases/v1.3.4/UnityYAMLViewer-v1.3.4-windows-x64.exe，30,860,288 字节，SHA-256：A3CCBCC9D9309CCD40237E9DAFF03636E617F397687B9133887FA992DA4B7F45；已核对与用户测试通过的 target/release/unity_yaml_viewer.exe 完全一致。本轮只复制现有产物，没有重新编译。
- 构建日志：target/hls-install-links-release-build.log；核验：target/hls-install-links-release-verification.json；发布准备核验：target/v1.3.4-release-preparation.json。target 下文件被 Git 忽略，提交源码不会包含候选程序。
- 验证：用户确认当前安装入口、间距及此前嗅探功能通过；其他验证范围与历史事实见 docs/versions/1.3.4.md，未将反馈扩大为全站点、全浏览器、七语言或关于窗口折叠全面回归。README 已按能力概览精简，中英文保持对应。
- 后续收到用户发布通知时：先核对用户最终提交及远程提交、版本号、候选大小和哈希、代码是否又有变化，再按明确授权创建标签和发布。若构建输入变化，不沿用未经验证的新产物；重新构建仍需明确授权并检查占用。独立脚本与在线 Greasy Fork 不自动同步，不宣称线上脚本版本已更新。

- 发布准备自查：版本记录、索引和发布说明链接有效；README 中英文能力条目对应，未加入版本或发布流程分节；node --check 和 git diff --check 通过。发布候选哈希与用户测试产物一致。源码准备快照 target/v1.3.4-prepared-source-manifest.json 记录 225 项当前构建输入，供用户提交后比对；此快照不是编译器生成的构建来源证明。

以下保留此前开发、构建与验证的时间记录；相关未构建或待验证描述按当时状态理解。

# 当前开发：1.3.4（尚未正式发布）

- 2026-11-06，用户确认最新 1.3.4 本地产物测试没有问题，包括第三方支持的安装链接与间距，以及此前的嗅探功能。确认对象为 target/release/unity_yaml_viewer.exe，SHA-256：A3CCBCC9D9309CCD40237E9DAFF03636E617F397687B9133887FA992DA4B7F45。按用户反馈记录本次验证通过；未逐项提供测试步骤，不扩展为所有站点、浏览器、七语言、窄窗口或关于窗口折叠均已全面回归。以下相关待验证描述保留各次修改当时状态，以本次用户确认的范围为准；尚未正式发布，未更新 GitHub 附件。

- 2026-11-06，安装链接与间距修改完成本地构建：用户明确授权，构建前确认程序未运行、目标文件独占访问通过；cargo build --release --locked --offline 成功（45.70 秒）。产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.4，大小 30860288 字节，SHA-256：A3CCBCC9D9309CCD40237E9DAFF03636E617F397687B9133887FA992DA4B7F45。已包含安装油猴插件链接及与安装脚本之间的额外间距；仅保留 spine38 的 158 项既有警告。日志 target/hls-install-links-release-build.log，核验 target/hls-install-links-release-verification.json。未自动运行程序；实际间距、换行及链接打开待验证，未正式发布或更新 GitHub 附件。以下未重建描述保留此前时点事实。

- 2026-11-06，第三方支持的网页浏览器 HLS 嗅探助手增加“安装油猴插件”链接，指向 https://www.tampermonkey.net/，位于“安装脚本”之前；两链接按顺序同行排列，额外增加 12 点间距，空间不足允许换行。新增七语言翻译；关于窗口已有安装引导摘要涵盖此项，核对后沿用。实际界面间距、窄窗口换行及浏览器打开待运行验证；此前构建不包含本次修改。cargo check --locked --offline、七语言完整性核对及 git diff --check 通过，仅保留 spine38 既有警告；未重建可执行程序。

- 2026-11-06，用户明确授权编译；构建前确认程序未运行、目标文件独占访问通过。cargo build --release --locked --offline 成功（1 分 10 秒），文件/产品版本 1.3.4 / 1.3.4，大小 30802944 字节，SHA-256：26B695F535392828CF13078AD8F3CD9B60E9BC72E2310210C33E7F49D438A464。产物 target/release/unity_yaml_viewer.exe；构建日志 target/v1.3.4-release-build.log，核验 target/v1.3.4-release-verification.json。仅保留 spine38 的 158 项既有警告；未自动运行程序，实际界面、脚本安装链接及关于窗口折叠待验证，尚未正式发布，未更新 GitHub 附件。

以下为本次构建之前的修改与验证历史，未构建描述仅代表当时时点。

- 2026-11-06，源码版本升级为 1.3.4，Cargo.toml / Cargo.lock 同步。1.3.3 正式发布后的变更与验证记录迁移至 docs/versions/1.3.4.md，版本索引与关于窗口同步；原构建版本及哈希不改写。第三方支持新增油猴助手安装引导及用户指定的 Greasy Fork 入口。本轮未生成新版程序、未修改 GitHub Release 或附件。
- 2026-11-06，本轮验证：cargo check --locked --offline 通过，仅保留 spine38 的 158 项既有警告；git diff --check、记录迁移、正式版摘要保留、历史顺序/重开重置代码及七语言完整性静态核对通过。未构建应用；实际界面与链接点击待运行验证。

- 下列为归入 1.3.4 的历史本地修改与产物记录；产物当时仍标为 1.3.3，不代表 1.3.4 构建结果。

- 2026-11-06，独立油猴助手 1.0.6：展开标题栏仅 HLS 数量＋绿色软件按钮，标题按钮发送当前网页让软件分析，列表按钮仍发送媒体地址。折叠最小态和两态半透明保持，按钮与拖动/折叠隔离；Node 语法及模拟参数/点击检查通过，实际浏览器待验证。仅脚本/文档修改，无需编译应用。

- 2026-11-06，独立油猴助手 1.0.5：展开浮层按三个操作按钮的实际宽度加间距/内边距/滚动条计算，地址与提示不撑宽面板；展开/新增候选/窗口缩放刷新宽度，窄屏允许按钮换行。保留折叠最小态及两态 30% 不透明背景；node --check、git diff --check 通过，浏览器实际宽度待验证，无需编译应用。

- 2026-11-06，独立油猴助手 1.0.3：默认最小态仅“▶ HLS 数量”，宽度随文字、折叠底色为 30% 不透明且文字清晰；标题栏可拖动，拖动不误触展开，展开/窗口缩放时校正可视边界，位置仅当前页。node --check、git diff --check 和 Node 模拟拖动/点击/边界检查通过；真实浏览器外观/触控待验证，仅脚本与文档修改，无需编译应用。

- 2026-11-06，最新本地产物（油猴唤起协议）：用户明确授权且测试/编译前进程与目标独占访问检查通过，2 项协议解析/IPC 测试通过，Release 离线构建成功（32.19 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,777,856 字节，SHA-256：08B4313F7EB7FE4A42E087F6ED25743C57D0F7FF80A846B1872A020D3D588776。独立脚本 scripts/Tampermonkey/UnityYAMLViewer-HLS/UnityYAMLViewer-HLS.user.js 为 1.0.1，含复制链接/原在线工具/本地软件三按钮；原威软脚本未改。构建仅保留 spine38 的 158 项警告，Node 语法及 git diff --check 通过。日志 target/browser-protocol-regression-tests.log、target/browser-protocol-release-build.log。未自动运行程序/注册协议；用户需先正常启动新版一次，再验证浏览器首次确认、唤起/单实例和播放。未更新已发布附件或历史候选副本，下列尚未构建状态为此前记录。

- 2026-11-06，独立油猴助手 1.0.1 保留“在工具中打开”：按钮顺序为复制链接→原在线工具→UnityYamlViewer，第三项替代原直接下载 TS。仅改新目录，原威软脚本未修改；node --check、git diff --check 通过，真实浏览器待验证。软件协议支持仍待重新编译；此次脚本调整本身无需编译应用。

- 2026-11-06，独立油猴助手与 Windows 自定义协议尚未重建：新目录 scripts/Tampermonkey/UnityYAMLViewer-HLS/ 含可安装 .user.js 和说明，原威软目录未改。协议 unityyamlviewer://open?url=...&referer=...，启动注册当前用户关联，沿用单实例转交，HLS 直接播放并保留来源 Referer，不传 Cookie。已完成 Rust 编译检查、Node 语法检查，新增协议/IPC 测试尚未执行；未编译新版、未实际写注册表或启动浏览器。需新版正常启动一次后安装脚本验证。README、七语言关于摘要及版本/使用说明同步，真实浏览器与 GUI 待验证；当前可执行程序不含本功能。

- 2026-11-06，最新本地产物（网页 HLS 候选扫描/选择）：用户明确授权，测试及编译前进程/目标独占访问检查通过；4 项 remote_file 测试通过，Release 离线构建成功（30.56 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,655,488 字节，SHA-256：C417DBB7D72077A21D8A620C9812900F1A478C637E63EDB79C9333E20025B74F。包含静态网页 HLS 发现、单条直接播放、多条截断列表/悬浮完整地址/右键复制/右侧打开，Referer 随播放传递；不执行脚本或监听浏览器 fetch/XHR。构建仅保留 spine38 的 158 项警告，测试缓存提示不影响通过。日志 target/hls-discovery-regression-tests.log、target/hls-discovery-release-build.log，git diff --check 通过；实际 GUI 待用户验证。未更新 GitHub 附件或历史候选副本；下列尚未构建状态为此前时点记录。

- 2026-11-06，网页 HLS 候选扫描/选择尚未重建：参考 scripts/Tampermonkey 下用户脚本的静态源码扫描部分（未修改/执行脚本），新增 HTML 响应扫描，单条直接打开、多条在 URL 弹窗下方单行截断列表选择，悬停完整地址、右键复制、右侧打开；播放携带来源页 Referer。最多读取 3,000,000 字节、返回 300 条并去重，保留百分号编码，无结果继续 yt-dlp。未实现浏览器 fetch/XHR 运行时监听、脚本/iframe 递归或认证绕过；不等同油猴完整嗅探。cargo check --tests --locked --offline 通过，新增测试尚未运行；七语言及 README/使用指南/版本记录同步，实际 GUI 待验证。当前可执行程序不含本修改，未自动编译或发布。

- 2026-11-06，最新本地产物（缓存工具栏顺序＋间距）：用户明确授权、进程及目标独占访问检查通过，Release 离线构建成功（30.65 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,755,840 字节，SHA-256：47554111E391739E906F3BDBFCAFAAA2EA3F91F46826E1D53D70A209A8118A8F。包含缩放比例后同排显示视频已缓存→音频已缓存→帧索引状态，及无格式选项/缓存链接时消除空白行；仅保留 spine38 的 158 项警告。日志 target/cache-toolbar-order-release-build.log，git diff --check 通过，实际 GUI 待用户验证；未更新 GitHub 附件或历史候选副本。下列未重建状态为此前时点记录。

- 2026-11-06，缓存链接位置更正尚未重建：“视频已缓存／音频已缓存”已移入播放工具栏，在缩放比例/缓存操作之后、帧索引状态之前，删除独立缓存链接行。链接外观及定位功能保持；cargo check --tests --locked --offline、git diff --check 通过，实际 GUI 待验证，当前程序不含此修改及此前空白行间距修正。

- 2026-11-06，网络播放页间距修正尚未重建：仅在有多个格式选项时创建选择行，仅在当前格式有本地轨道缓存时创建缓存链接行，消除两处无内容布局空行。cargo check --tests --locked --offline、git diff --check 通过，实际不同格式/缓存状态间距待用户验证；现有可执行程序不含本次修正。

- 2026-11-06，最新本地产物（缓存对齐＋帧索引后移）：用户明确授权、进程及目标独占访问检查通过，Release 离线构建成功（32.17 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,756,864 字节，SHA-256：84ABDDC897737D753402AB252D834C80728AE0E81AA4FC949C5C0F8E02DB69AF。包含缓存状态/取消按钮统一 24 点高度和居中、网络工具栏帧索引状态移到缓存操作之后；仅保留 spine38 的 158 项警告。日志 target/cache-alignment-release-build.log，git diff --check 通过；实际 GUI 待用户验证。未更新 GitHub 附件或历史候选副本；下列尚未重建状态是此前时点记录。

- 2026-11-06，缓存状态对齐修正尚未重建：缓存文字框和取消按钮统一 24 点高度，与前方解码/缩放控件一致，按文字宽度分配单行居中布局，保留真实进度/处理/取消状态。cargo check --tests --locked --offline、git diff --check 通过。实际 GUI 待验证；当前程序不含本修正及此前帧索引状态后移。

- 2026-11-06，帧索引提示后移尚未重建：网络播放页把“正在索引视频帧…”放在工具栏末尾，缓存状态/取消之后，转圈和文字同组；普通本地/列表播放顺序保持。cargo check --tests --locked --offline、git diff --check 通过，实际顺序及窄窗口待验证；现有可执行程序不含本次修改。

- 2026-11-06，最新本地产物（缓存界面精简）：用户明确授权、进程及目标文件占用检查通过，Release 离线构建成功（32.48 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,774,784 字节，SHA-256：B51ADA10ED7588B7634016895D019B3B9AC53730CCAC6D2702F31AE1943FC5EE。包含缓存完成链接文字精简、悬停路径/点击定位及缓存中描边百分比文本＋取消；仅保留 spine38 的 158 项警告。日志 target/compact-cache-ui-release-build.log；git diff --check 通过，实际 GUI 待用户验证。未更新 GitHub 附件或历史候选副本；下列尚未重建描述是此前时点记录。

- 2026-11-06，缓存进度紧凑显示尚未重建：播放页移除进度条/转圈和重复缓存按钮，改为描边百分比文字及紧跟取消按钮，控件组内不换行；处理与取消阶段显示真实状态。七语言和文档同步，cargo check --tests --locked --offline、git diff --check 通过，实际 GUI 待验证。现有可执行程序不包含本次修改及前述缓存链接文字精简。

- 2026-11-06，缓存链接文字精简尚未重建：播放页改为带下划线的“视频已缓存／音频已缓存”，悬停显示完整路径，点击定位，两个链接同排且可换行；去掉常驻路径及截断触发的额外悬浮提示。七语言和文档同步，cargo check --tests --locked --offline 通过。实际 GUI 待验证；现有可执行程序不含本次精简。

- 2026-11-06，最新本地产物（缓存路径＋Unicode 粘贴）：用户明确授权且测试/编译前进程与目标独占访问检查通过，2 项 remote_file 测试通过，Release 离线构建成功（29.49 秒）。target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,717,440 字节，SHA-256：C168B21F9C91D4A4295D28DBB3BED0A274AE354DD86AD6EC580D62D7F50551E7。包含播放页视频/音频缓存路径、缓存完成任务点击定位及右键打开播放页、打开 URL/视频提取粘贴时单层 Unicode 解码并保留百分号编码。仅保留 spine38 的 158 项构建警告；测试构建有增量缓存整理/硬链接提示但退出成功。日志 target/pasted-unicode-regression-tests.log、target/cache-path-unicode-release-build.log；新增七语言和 git diff --check 通过。实际界面待用户验证，未更新 GitHub 附件或历史候选副本；下列未重建状态是此前时点记录。

- 2026-11-06，新增缓存路径定位修改尚未重建：播放页显示当前格式的视频/音频本地缓存路径，点击定位；已完成缓存任务保存实际视频缓存路径，左键定位，右键保留打开原播放页，已删除缓存沿用失效路径提示。cargo check --tests --locked --offline、git diff --check 和新增七语言文案核对通过。实际界面、HLS 完成切换及文件删除场景待验证；现有可执行程序仍为下列用户已测试通过的旧构建，不包含本修改。

- 2026-11-06，用户确认最新本地产物测试通过，覆盖本轮 Cookie 修正、完整音频选项、音量悬浮提示及 URL 通用提取兜底。确认对象 target/release/unity_yaml_viewer.exe，SHA-256：6F36B9AF153486BB9BEFBC03AEAC51B254CF9D2DC69EDDFD7B45459CA16857DD。不代表所有站点/语言或全部历史功能已全面回归，关于窗口折叠未单独报告验证；未自动提交、推送或更新 GitHub 发布附件。

- 2026-11-06，最新本地验证产物：用户再次明确授权，测试及 Release 编译前均确认程序未运行、目标可独占访问；13 项视频提取测试及 1 项网络视频缓存复用测试全部通过。Release 离线构建成功（32.15 秒），target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,703,616 字节，SHA-256：6F36B9AF153486BB9BEFBC03AEAC51B254CF9D2DC69EDDFD7B45459CA16857DD。包含 Cookie 引号/转义解码、完整音频格式选项、方向键音量居中提示（松键 0.2 秒消失）和打开 URL 无法加载文件时由 yt-dlp 兜底自动播放。构建仅保留 spine38 的 158 项警告；测试阶段增量缓存整理/硬链接提示不影响通过。日志 target/url-playback-regression-tests.log、target/network-video-regression-tests.log、target/url-playback-release-build.log。关键七语言文案和 git diff --check 核对通过；实际 TikTok/B 站播放、音轨切换、音量提示及关于窗口展示待用户验证。自动选择第一个格式指提取界面既有高质量优先排序的第一项。未更新 GitHub 附件或 target/releases 历史候选副本；下列“尚未构建”等为先前时点记录。

- 2026-11-06，Cookie 解码后续修正尚未构建：同一 TikTok 媒体地址在解码 tt_chain_token 序列化引号后 FFprobe 成功（时长 23.127007 秒），此前同参数未解码返回 403，yt-dlp 前 10 KiB 读取成功。已修改源码并新增引号/转义/注入过滤回归测试，cargo check --tests --locked --offline 通过。编译前目标 target/release/unity_yaml_viewer.exe 独占访问失败，未运行测试构建或 Release 重建；用户关闭占用并再次明确确认后须复查，当前可执行程序仍为下列旧构建。GUI 待验证。

- 2026-11-06，发布后 Cookie 传递修正本地构建：补传 yt-dlp cookies 到 FFmpeg，保留域名/路径作用域，探测、播放和手动缓存共用；cargo check --tests --locked --offline、2 项针对性测试通过。用户授权且构建前进程/目标文件占用复查通过，Release 构建成功（31.13 秒）。产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,707,200 字节，SHA-256：10D5F129D292E5346E7B9DA1ED37E1196AAD34A10F418CD7D22198FCBF2CB270。日志 target/playback-cookie-check.log、target/playback-cookie-tests.log、target/playback-cookie-release-build.log。仅保留 spine38 警告；TikTok DNS 解析失败，需用户通过“视频提取 → 重新分析 → 不下载直接播放”验证，不能宣称 403 已解决。七语言关于摘要已更新，GUI 待验证；未更新 GitHub 附件或历史候选副本。

- 2026-11-05，发布后警告清理本地构建：用户明确授权，检查程序未运行且目标可独占访问后，cargo build --release --locked --offline 成功，耗时 42.62 秒。产物 target/release/unity_yaml_viewer.exe，文件/产品版本 1.3.3，30,615,552 字节，SHA-256：436EC0FB87DF4EB105EE12958D4A96410B137AF687DFE90A91E4BCABCA913164。项目自身 3 项代码警告已消除，仅保留 spine38 的 158 项警告；日志 target/warning-cleanup-release-build.log。未运行 GUI 验证，未更新已发布附件及 target/releases 下历史候选副本。

# 当前正式发布：1.3.3（已正式发布）

- 正式发布：用户确认 1.3.3 新增内容验证通过并明确授权发布；远程 master 与本地提交 cd776fd41cc061e7c308d92f5d53feb727c8e351 一致。225 项候选输入中排除 1 个不参与构建的 Python 字节码缓存，其余与提交无实质差异，155 项仅换行格式不同。未重新编译。先创建草稿，上传 30,748,160 字节附件并核对 GitHub SHA-256 为 DE80D78C2A1EC3C8D7AC3CDBF73CEC5047D047A7E44316EBC2927C8CF67B4BDA，再于北京时间 2026-11-05 22:31:05 正式发布 v1.3.3（Release ID 435309872），标题仅版本号、非预发布、latest API 确认为最新稳定版。证据 target/v1.3.3-publish-audit.json、target/v1.3.3-release-asset.json、target/v1.3.3-published.json。以下待验证/未发布表述保留开发阶段史实，不代表当前发布状态。

- 发布后的状态文档仅更新在本地，未自动提交或推送；没有修改其他历史 Release。

以下为发布前准备及开发历史记录。

# 1.3.3 提交与发布交接

- 文档已整理：README 中英文能力概览、docs/user-guide.md 使用说明、docs/versions/1.3.3.md、版本索引、CHANGELOG、docs/releases/1.3.3.md 发布准备稿。1.3.2 已发布后变更归入 1.3.3，保留旧构建事实。
- 当前候选：target/releases/v1.3.3/UnityYAMLViewer-v1.3.3-windows-x64.exe，30,748,160 字节；SHA-256 DE80D78C2A1EC3C8D7AC3CDBF73CEC5047D047A7E44316EBC2927C8CF67B4BDA。已重新核对与 target/release/unity_yaml_viewer.exe 一致。
- 构建日志 target/v1.3.3-release-build.log，产物核验 target/v1.3.3-release-verification.json，当前构建输入哈希清单 target/v1.3.3-build-manifest.json。最后构建后仅整理文档，没有继续修改源码。
- 用户负责提交 Git 并通知提交结果；代理尚未提交/推送、创建标签、上传附件或发布。收到通知后先核对提交中构建输入与候选清单；有源码差异则需重新确认产物，不直接上传旧候选。
- 正式发布标题仅 v1.3.3。发布前仍需用户确认最终验证结果；现有文档不宣称 GUI 全面回归通过。发布准备稿中状态/构建交接段不作为面向用户的更新内容发布。
- 临时样本 11406_d45ec9b4-index.m3u8.txt 与 11406_d45ec9b4-mixed.m3u8.txt 不属于功能源码或发布附件；保留原文件，由用户自行决定是否提交。

以下为历史开发与候选记录，旧状态/哈希不代表当前产物。

## 最新候选更新

- 2026-11-05，用户授权且进程/候选文件占用检查通过后，release 构建成功（45.20 秒）。文件/产品版本 1.3.3，大小 30748160 字节，SHA-256：DE80D78C2A1EC3C8D7AC3CDBF73CEC5047D047A7E44316EBC2927C8CF67B4BDA。候选副本同步且哈希一致，包含音频单个/多个垂直居中、单条预览背景跟随填充色及合并独立填充补边。用户将继续验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权且程序/候选文件占用检查通过后，release 构建成功（45.86 秒）。文件/产品版本 1.3.3，大小 30749184 字节，SHA-256：1FB372A10F9096333E61C0E94E370B293C7E35018F8DD4D12DD59D0A2515319C。候选副本同步且哈希一致，包含按勾选数量的灰色加粗提示、至少选一项规则、范围重置撤销、大圆播放图标、画布扩展/填充色/Hex 输入及名称左对齐。画布扩展和重置 2 项测试、颜色解析 1 项测试通过，FFmpeg 四组尺寸/填充样本通过；实际 GUI 待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权且程序/候选文件占用检查通过后，release 构建成功（43.16 秒）。文件/产品版本 1.3.3，大小 30701568 字节，SHA-256：5179BA279D0115CE4681BD725820FD07EE5C303E5C46C0305978E76B67E1E08C。候选副本同步且哈希一致，包含队列模式切换不计修改、名称左对齐、序号排序手柄、Ctrl+Z 恢复移除、重置范围位置及 Next/翻转 Next 边界跳转。实际 GUI 待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权且程序/候选文件占用检查通过后，release 构建成功（43.88 秒）。文件/产品版本 1.3.3，大小 30624256 字节，SHA-256：C866E39E0335173E685F2B37758405639E3C2F30E148EFF05511BD84015B05E1。候选副本同步且哈希一致，包含中央绿色进入编辑窗口按钮、组件缺失隐藏入口及整体上移布局，顶部保留打包工程。实际 GUI 待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权并完成进程及文件占用检查后，release 构建成功（41.86 秒）。文件/产品版本 1.3.3，大小 30681088 字节，SHA-256：DCE09995345D4706B0F17E3EFCF37C9761F8EDF3E067ACF3D18D12C546C3B8BB。候选副本同步且哈希一致，包含 ZIP 打包与工程页签分享说明。打包解压换目录恢复、嵌入数据保留及缺失素材阻止输出的测试在普通 Windows 权限下通过；受限环境文件移动曾报拒绝访问。GUI 及真实工程分享待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权且目标与候选文件占用检查通过后，release 构建成功（45.26 秒）。文件/产品版本 1.3.3，大小 30629376 字节，SHA-256：3DA6E8A74B5CC3B28F9358B34CFD1DF6FBDB4F7981797D199EA606BD739EE6E1。候选副本同步且哈希一致，包含工程页签/同类分组/组件检测与居中说明、固定合并设置、列表滚动及 8 像素末尾拖放、确认框分区、勾线与按钮外扩调整；实际 GUI 待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户授权并检查目标及候选文件无占用后，release 构建成功（45.05 秒）。文件/产品版本 1.3.3，大小 30453248 字节，SHA-256：A4B8CAD1381A2C94A4291F0B197450AF7B87B7464ABDC52368093FC6A889F25E。发布候选副本已同步并核对一致，包含放大绿底白勾复选框、编辑窗口内部居中模态确认、红色确定/绿色取消及取消关闭时保持原位置尺寸。实际界面待验证，尚未发布。

## 最新候选更新

- 2026-11-05，用户确认后重新检查主程序及候选文件均无占用，release 构建成功（44.02 秒）。文件/产品版本 1.3.3，大小 30576128 字节，SHA-256：F3401291DAB4B437E49B095936E37EAAB8AC12CDAFE96447D435E678EC82BCFB。已更新 target/releases/v1.3.3/UnityYAMLViewer-v1.3.3-windows-x64.exe 并核对一致；包含绿色勾选导出、右键移除、默认箭头及未保存关闭确认。实际交互待验证，尚未发布。

# 当前版本：1.3.3（发布候选，尚未发布）

## 2026-11-05 发布准备

- 已从 1.3.2 发布后记录迁移到 docs/versions/1.3.3.md，保留旧构建真实版本；Cargo/索引/CHANGELOG/关于七语言同步，默认显示 1.3.3 与 1.3.2，旧版折叠。
- 2026-11-05，经用户授权并重新确认程序未运行、目标无占用后，cargo build --release --locked --offline 成功（48.98 秒）。文件及产品版本均为 1.3.3，大小 30380032 字节，SHA-256：4E2D8320A4DA330C3A67DC8DD828FC3DB08FC063D59EBED07086A3FF264D7996。候选副本 target/releases/v1.3.3/UnityYAMLViewer-v1.3.3-windows-x64.exe 哈希一致；日志 target/v1.3.3-release-build.log，核验 target/v1.3.3-release-verification.json。仍有既有依赖/未使用导入/浮点警告；尚未发布。
- 48 个不同针对性测试通过、1 项外部项目测试跳过；覆盖文件测试在受限环境拒绝访问，普通 Windows 权限复核通过。实际 GUI、真实网络及工程恢复仍待回归。未提交/推送、打标签、上传附件或发布。发布标题仅 v1.3.3。用户临时 m3u8 txt 样本未作为候选附件。

以下保留历史交接记录。

# 当前版本：1.2.9（已正式发布）

## GitHub 正式发布（2026-11-02）

- 用户明确授权发布；发布前工作区干净，提交 1d9e0b3c4ea21ebf598f70e1428708cf24079de8 与 GitHub master 一致。189 项实际构建输入与该提交一致；原清单 190 项中的一个 Python 字节码缓存不参与构建，单独排除并记录，未重新编译。
- 北京时间 2026-11-02 20:00:00 正式发布 [v1.2.9](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.9)，Release ID：431409448；标题仅 v1.2.9，非草稿／非预发布，发布后 latest API 确认为最新正式版。
- 附件 UnityYAMLViewer-v1.2.9-windows-x64.exe，26,760,704 字节；GitHub 附件 SHA256 与本地产物一致：558C831630253DE09F7BF9DCF1D7D6B4C19ADD26D9F686B1478A397F1D745CA1。
- 先建草稿，上传并核验附件状态、大小和哈希后正式发布。发布证据 target/v1.2.9-published.json，源码核对 target/v1.2.9-publish-audit.json。未修改其他历史 Release 或附件，未自动提交／推送发布后的文档更新。
- 正式发布状态取代此前准备阶段的未发布说明；保留实际验证范围，未宣称执行全部测试或补做 GUI 验证。


## 1.2.9 发布准备与本地构建（2026-11-02，未发布）

- 按用户要求精简 README 中英文能力条目与限制；不加入逐版本更新分节或内部发布流程。新增简要发布说明准备稿 docs/releases/1.2.9.md。
- 用户明确授权编译；构建前检查查看器进程和目标 EXE 独占访问，无占用。cargo build --release --locked --offline 成功（43.00 秒，8 条警告）；日志 target/v1.2.9-release-build.log。
- 产物 target/release/unity_yaml_viewer.exe，文件／产品版本均为 1.2.9，26760704 字节；SHA256：558C831630253DE09F7BF9DCF1D7D6B4C19ADD26D9F686B1478A397F1D745CA1。
- 发布候选副本 target/releases/v1.2.9/UnityYAMLViewer-v1.2.9-windows-x64.exe 与原产物哈希一致；核验 target/v1.2.9-release-verification.json，190 项构建输入记录 target/v1.2.9-build-manifest.json。
- 1517 条翻译七列完整且键唯一，关于默认 1.2.9／1.2.8、1.2.7 起折叠及重开折叠静态核对通过；git diff --check 通过。此前代码检查通过，自动测试未实际执行，本轮未运行 GUI，不声明全面验证。
- 实际新版 PDF 保存、表格边线／冻结交互及七语言关于窗口布局仍待验证。仅准备产物，未提交／推送源码，未创建标签、Release 或上传附件；正式发布标题须仅为 v1.2.9。


## 1.2.9 版本整理（2026-11-02）

- 从已发布 1.2.8 的正式发布记录之后划分边界，后续开发与验证记录迁入 docs/versions/1.2.9.md；1.2.8 保留发布能力与发布证据，未改历史发布说明、标签或 Release。
- Cargo.toml／Cargo.lock 升为 1.2.9，更新版本索引和 CHANGELOG；README 保持当前能力结构，不添加版本流程内容。
- 关于默认展示 1.2.9 与 1.2.8，1.2.8 摘要依据 v1.2.8 标签恢复；1.2.7 及以前移入原历史折叠区。保留已有七语言，补充当前摘要七语言，重新打开仍恢复折叠。
- 迁入的旧构建仍标记真实版本 1.2.8，不改写为 1.2.9。当前现有 EXE 仍是此前构建；尚未编译或发布 1.2.9，关于窗口新排序与七语言布局待运行验证。

以下为历史交接记录，旧版本构建与阶段描述保留原始事实。

# 当前补充：行列坐标默认固定＋可选沿用 XLSX 冻结设置（2026-10-30，仅源码）

## 固定行列编号、可选冻结表头与 XLSX 原冻结设置（2026-10-30，源码）

- 用户分别明确：顶部列字母和左侧行号应默认固定；额外“冻结表头”应优先使用原文件设置，无法读取才兜底首行。现将固定坐标作为表格视图常态，顶部列字母／类型不随纵向滚动消失，左侧行号不随横向滚动消失，仍随对应数据行／列保持对齐。
- 新增独立“冻结表头”，位于“首行为表头”之后，不依赖类型检查开关，不影响原有类型推断／必填／首行跳过规则。数据文本模式禁用此显示开关。没有可读原设置时默认关闭，开启后冻结当前显示数据的首行；关闭后仍保持坐标固定。
- calamine 没有冻结元数据接口，新增基于已缓存 quick-xml 0.42 的 XLSX 补充读取：根据 workbook.xml 的工作表名和关系 ID 定位该表，解析 sheetViews 中 frozen／frozenSplit 的 xSplit／ySplit；读取到有效设置后首次加载自动沿用，后续帧不覆盖用户手动取消。支持多冻结行／列，按当前有效数据坐标偏移换算并限制到当前显示范围。只读 ZIP 内部，不提取文件，不访问外部关系，元数据读取上限 4 MiB。
- 不将普通 split 模式的尺寸或 topLeftCell 的保存滚动位置误认为冻结范围；无有效设置、解析失败或不支持格式使用首行兜底。XLS／ODS／CSV／TSV／PSV 原冻结元数据未读取，不宣称完整还原所有格式。界面仍提供关闭入口；源冻结区域大于窗口时不能保证所有冻结内容同时可见。
- 单一双轴滚动视口保持，横纵滚动条仍按溢出分别显示。冻结区采用查看区相同底色避免露出后面的数据；滚动内容按固定区域裁剪，冻结和普通单元格共用绘制／编辑／复制／高亮逻辑，保持实际行列地址和数据偏移。列宽拖动、搜索／校验跳转与冻结交互仍需实际 GUI 验证。
- 用户样本只读 XML 核对确认：主表原有 ySplit=1.0、state=frozen（第一行冻结），另外三表没有冻结窗格。独立对照 JSON 已补充这些设置，样本测试代码会在明确执行时一并验证。未修改用户原文件。
- README 中英文、功能参考、关于当前摘要及新增提示七语言同步；设置末尾两项、关于默认两版／历史折叠与重开逻辑不改。
- 验证：`cargo check --tests --locked --offline`、七语言字段／唯一键／新增提示覆盖和 `git diff --check` 通过。新增冻结设置首次沿用／手动取消、真实 egui 双轴滚动坐标／冻结区对齐、XLSX 窗格计数／split 排除／路径校验测试代码，尚未执行。遵循用户要求未编译、未运行测试或 GUI，也未检查程序占用或请求编译确认。实际冻结、切换、多语言／DPI、编辑／复制／定位和关于历史交互待运行验证。

---
# 当前补充：滑条与工作表栏留 6 像素间隔，仅代码修改（2026-10-30）

## 表格滑条与工作表栏增加间距（2026-10-30，源码）

- 用户反馈水平滑条与底部工作表标签靠得过近／重叠。现在在表格内容区域与独立工作表栏之间预留 6 逻辑像素间距，只缩小表格区域，底部标签位置和无整条底色的样式保持，不恢复额外底部留白。
- 仅底栏实际存在时预留间距；单工作表不留空间，极小高度时按剩余空间截断间距，不产生负高度。标签自身横向溢出和两区域独立裁剪逻辑保留。
- 功能参考及当前关于摘要七语言同步，README 能力范围无变化。设置末尾两项及关于默认两版／历史折叠与重开状态不变。
- `cargo check --tests --locked --offline`、翻译键／七语言字段核对和 `git diff --check` 通过。显示间距调整未新增测试；实际浅深主题、不同 DPI／窗口大小及横纵溢出场景待 GUI 验证。按用户要求，本轮只修改和检查代码，没有检查程序占用、编译或请求编译确认。

---
# 当前构建：工作表栏去底色与表头选项前置（2026-10-30）

## 工作表底栏样式与表头选项顺序本地构建（2026-10-30）

- 用户本轮明确说明“可以编译”后才执行占用检查；系统进程确认无查看器运行，目标 EXE 可独占访问。未自动关闭程序。后续普通修改仍仅做代码检查，等待用户明确要求编译。
- `cargo build --release --locked --offline` 成功，41.62 秒，8 项应用警告；日志：`target/table-option-order-release-build.log`。此前 `cargo check --tests --locked --offline`、1503 条翻译键／七语言字段核对和 `git diff --check` 通过。
- 产物 `target/release/unity_yaml_viewer.exe`，版本 1.2.8，26,719,744 字节；SHA-256：`8BE47F276688BFB58926F13B303F35E239BEE9B6BE8B6CB8EFF294E9AA141DA9`。
- 包含此前未构建的底栏整条底色／分隔线移除、取消额外上下留白（保留活动标签高亮），以及“首行为表头 → 类型检查 → 必填”的选项顺序。此前表格直读、底部多表切换、按需滑条和必填功能保留。
- 本轮未重复运行自动测试、未启动 GUI 或用户文档编辑、未安装组件；底栏视觉位置、深浅主题／DPI、长标签横向溢出、选项布局与关于历史交互仍待运行验证。未提交、推送、创建标签或发布 Release。

---
# 当前补充：表头选项前置，仅代码修改与检查（2026-10-30）

- 用户明确要求等其主动说明后才编译；此偏好覆盖之后的普通修改，不主动检查占用或催促编译。

## 表格检查选项顺序调整（2026-10-30，源码）

- 将共用表格视图的选项顺序改为“首行为表头 → 类型检查 → 必填”；必填仍仅在类型检查开启时显示。适用于 CSV／TSV／PSV 和 XLS／XLSX／ODS，不改变校验规则、默认值或缓存失效／重查行为。
- 关于当前摘要七语言同步，README 能力范围无变化；设置窗口末尾排序及关于默认两版／历史折叠与重开逻辑不变。
- `cargo check --tests --locked --offline`、翻译字段／唯一键和 `git diff --check` 通过；本轮未新增或执行测试、未运行 GUI。遵循用户最新要求，只修改与检查代码，不主动检查程序占用、不编译或请求编译确认。实际布局及多语言／DPI 待运行验证。

---
# 当前补充：工作表栏样式已修正，程序占用待编译确认（2026-10-30）
- 构建前检查发现查看器仍在运行（PID 43412，`target/release/unity_yaml_viewer.exe`），已停止编译且未关闭程序。现有 EXE 不含本次去底色／收紧样式；需用户关闭后再次确认，再检查占用后构建。检查日志：`target/worksheet-footer-style-check.log`。


## 工作表底栏去除底色并收紧留白（2026-10-30，源码）

- 用户反馈底部整条色块突兀，希望去掉并更贴底部。现移除底栏整条背景填充和顶部额外分隔线，继承查看区背景；保留单个标签的边框和当前工作表选中高亮。
- 取消底栏额外的 4 像素顶部间距和合计 8 像素的纵向预留；常规底栏只保留标签所需高度，按钮底边贴近查看区内容底部，释放的高度回到表格区域。长标签溢出时仅额外保留底栏自身水平滚动条所需空间。窗口通用外边距未改变。
- 仍仅在多个工作表时显示独立底栏；单工作表不留空区域，底栏在表格水平滑条下方且不随表格滚动。标签切换、悬停全名、加载禁用逻辑保持。
- 当前关于摘要七语言和功能参考同步；README 能力范围无变化。设置末尾顺序、关于默认两版及历史折叠／重开逻辑未改。
- 验证：`cargo check --tests --locked --offline`、七语言字段／唯一键及 `git diff --check` 通过。此显示样式调整未新增测试，深浅主题、不同窗口尺寸／DPI、长标签溢出和关于历史交互仍待实际 GUI 验证；构建状态见后续记录。

---
# 当前构建：底部工作表栏、按需滑条与必填校验（2026-10-30）

## 底部工作表栏／按需滑条／必填检查本地构建（2026-10-30）

- 用户再次明确确认查看器已关闭并授权测试、编译。重新通过系统进程检查确认无查看器运行，目标 EXE 可独占访问；构建前再次核对 EXE 可独占访问，未自动关闭任何用户程序。
- `cargo check --tests --locked --offline` 已通过；`cargo test --locked --offline delimited_table::tests` 最终实际执行 12 项并全部通过，其他 380 项未执行。覆盖原有解析／类型／搜索／坐标回归、必填关闭兼容、空白和短行缺列、0／false／NULL、表头跳过及合并错误顺序。
- 核对必填标红时发现原先背景范围跟随文字，完全空白单元格可能无可见宽度；现改用完整单元格矩形绘制错误／匹配背景、选中边框与定位范围。增加 egui 无原生窗口绘制测试，实际验证唯一空白必填单元格的红色背景至少覆盖 100×20 逻辑像素，最终通过。此测试不替代最终桌面 UI 验证。
- 日志：`target/worksheet-footer-required-check.log`、`target/worksheet-footer-required-tests.log`。1501 条翻译键唯一性、七语言字段完整性／新增提示覆盖和 `git diff --check` 通过；未扩展设置项，关于默认当前／上一版本及历史折叠逻辑未改。
- `cargo build --release --locked --offline` 最终成功，37.37 秒，8 项应用警告；日志：`target/worksheet-footer-required-release-build.log`。中间构建后补充空值绘制修正并重新构建，以下信息对应最终产物。
- 产物：`target/release/unity_yaml_viewer.exe`，版本 1.2.8，26,715,648 字节；SHA-256：`912240F701DA0AF3AC5C8D2E22C89D4275651F35F9E0DCAB53882EB940CC8564`。
- 包含底部多工作表独立切换栏（仅多表时保留区域）、横纵滚动条分别按溢出显示，以及类型检查开启时可见且默认关闭的必填开关。保留此前电子表格直读、公式／缓存切换、外部 LibreOffice 编辑及安装器选择。
- 用户问询 Numbers：当前读取库及本软件仍不直读 .numbers；可后续评估 LibreOffice 导入后转临时 ODS 再读表格，但尚未实现或验证，不把该方案记为支持。未启动用户文件的 LibreOffice 编辑或组件安装。
- 未启动最终桌面 GUI，底栏位置／单表释放空间／长标签横向滚动、加载和错误切换、四种滚动溢出组合、实际必填交互、七语言／DPI、关于展开／收起与重开仍待运行验证。没有提交、推送、创建标签或更新 Release。

---
# 当前补充：底部多工作表栏＋必填校验已接入，待关闭程序后确认构建（2026-10-30）

## 底部多工作表切换栏与可选必填校验（2026-10-30，源码）

- 用户要求保持常见表格软件的布局：移除顶部工作表下拉，将工作表名称和当前序号／总数移至底部独立固定区域，位于表格自身水平滚动条下方。只有多于一个工作表时才预留底栏；单个或尚未读取到工作表列表时，表格使用全部剩余高度。
- 底栏与数据区域分别裁剪，不随数据滚动；使用当前主题底色和选中高亮。名称最长显示宽度 260 逻辑像素并提供完整名称提示，超过可用宽度时底栏独立横向滚动并按需显示自己的滚动条。读取后或切换时将选中标签滚入可见范围。加载中底栏保留但禁用标签，失败页仍可切换其他工作表。
- 用户确认类型检查默认允许空白后要求增加“必填”。现仅在类型检查开启时显示此开关，默认关闭；打开后空内容、仅含空白字符（含制表／换行）的内容及短行缺失列都作为校验错误。表头按现有“首行为表头”跳过，0／false／文字 NULL 不视为空值；非空值仍按推断类型检查。空值和类型错误只记一次，按行列顺序合并，开启必填时计数标签为“校验错误”，保持红色标记和上一项／下一项定位；关闭恢复原类型错误计数语义。
- 必填只报告问题，不阻断原有保存，不自动填空，不新增字段级必填规则。开关跟随当前表格会话；电子表格切换／重载沿用既有视图重置逻辑，不新增跨重启持久化设置。适用于 CSV／TSV／PSV 和 XLS／XLSX／ODS 共用视图；内置电子表格仍只读，不写回公式或数据。
- README 中英文、功能参考和关于当前摘要七语言同步；设置最后两项排序、关于默认两版与历史展开／重开恢复逻辑不变。此前滚动条逐轴溢出显示修正继续保留，将与本轮一起构建。
- 验证：`cargo check --tests --locked --offline`、七语言唯一键／字段／新增提示覆盖及 `git diff --check` 通过。补充必填关闭兼容、空白／短行缺列、0／false／NULL、表头选择及类型＋必填合并顺序的测试代码，尚未执行。底栏具体位置、单表释放高度、横向溢出、多语言／DPI、加载／错误切换、必填交互和关于展开／重开仍待运行验证。此前已发现程序占用（PID 40260）；本轮未生成新 EXE，需用户关闭后再次明确确认编译，再检查占用。

---
# 当前补充：表格滑条已修正，因程序占用等待编译确认（2026-10-30）
- 本轮构建前检查发现查看器仍在运行（PID 40260，`target/release/unity_yaml_viewer.exe`），已按 AGENTS.md 停止编译，没有关闭程序。需用户关闭后再次明确确认，再重新检查占用并构建；现有 EXE 尚不含本轮按需滚动条修正。检查日志：`target/table-scrollbars-check.log`。


## 表格滚动条按对应方向溢出显示（2026-10-30）

- 用户澄清此前“始终显示”是有条件的：内容超出宽度才显示水平滚动条，内容超出高度才显示垂直滚动条，两方向独立；空间足够时不显示对应滚动条。
- 共用 TSV／CSV／PSV 与 XLS／XLSX／ODS 表格数据视图改用 `ScrollBarVisibility::VisibleWhenNeeded`，保留实体滚动条和单一双轴视口；需要滚动时保持可见，不依赖悬停，不恢复浮动遮挡。调整窗口或列宽后由布局自动重新判断溢出；原始／数据文本视图沿用已有默认按需显示逻辑。
- README 中英文、功能参考及当前关于摘要七语言同步；设置末尾两项、关于默认两版／历史折叠和重开状态不变。
- 验证：`cargo check --tests --locked --offline`、七语言唯一键／字段完整性和 `git diff --check` 通过。本轮不为这一低影响显示策略增加测试；实际四种横纵溢出组合、缩放窗口／列宽后切换、搜索跳转、七语言布局与关于历史交互仍待运行验证，编译检查不能代替 GUI 验证。构建情况见后续记录。

---
# 当前构建：电子表格数据视图与“在LibreOffice编辑”（2026-10-30）

## 电子表格数据视图与外部编辑入口本地构建（2026-10-30）

- 用户明确确认查看器已关闭并授权编译及样本测试。重新通过系统进程检查确认查看器未运行，目标 EXE 可独占访问后执行；没有自动关闭用户程序。
- `cargo check --tests --locked --offline` 通过；新增表格逻辑测试实际执行 2 项通过，覆盖缓存值 0／缺失结果、公式独立范围、坐标偏移、TSV 内嵌分隔／换行／引号及范围拒绝；普通 TSV 的 9 项解析／类型／搜索／坐标回归测试全部通过。
- 显式执行用户样本对照测试 1 项通过：calamine→TSV→表格解析所得数据与 Python 标准库独立读取的 XLSX XML 逐项比对。4 个工作表共 4250 个有效单元格（3708＋393＋35＋114），35 个公式及缓存结果均一致；数值按浮点数值比较，字符串／布尔值／表达式逐字比较。测试只读取原文件，不产生写回。上述合计 12 项测试通过，其他 377 项未执行，不代表全应用回归。
- 日志：`target/spreadsheet-grid-check.log`、`target/spreadsheet-grid-tests.log`、`target/spreadsheet-sample-tests.log`、`target/spreadsheet-tsv-regression-tests.log`。独立对照数据 `target/animation-state-oracle.json` 被 Git 忽略，样本路径只通过测试环境变量传入，不硬编码入产品或测试源码。
- 1496 条翻译键的唯一性、七语言字段完整性及新增 UI 标签覆盖与 `git diff --check` 通过。
- `cargo build --release --locked --offline` 成功，57.34 秒，8 项应用警告；日志：`target/spreadsheet-grid-release-build.log`。产物：`target/release/unity_yaml_viewer.exe`，版本 1.2.8，26,663,936 字节，SHA-256：`A72F69AF9C070FE161A3335800E1BD89332B5B2553B6DA89CC34156B0578F7AA`。
- 已包含 XLS／XLSX／ODS 表格直读、工作表切换、搜索复制、公式／缓存结果切换，以及此前尚未编译的“在LibreOffice编辑”入口和既有 Scoop／WinGet 管理功能。
- 样本原文件构建后 SHA-256 再次核对仍为 `54100377A26135A00DA7203A8CFC6395FEB607912521AFD5DAF31102DEBB8CAA`，与读取前一致。未实际启动 LibreOffice 编辑用户文件、未启动最终 GUI，也未发起组件安装。
- 待运行验证：实际表格 UI／多工作表与公式切换、选择和右键复制、搜索跳转、日期／样式等限制、外部编辑保存后重载、不同 XLS／ODS、异常／加密／大范围、七语言／DPI 和关于历史展开收起／重开。没有提交、推送、标签或 Release 操作。

---
# 当前补充：电子表格改为原生数据表格，等待关闭程序后确认构建（2026-10-30）

## 发布后的新增：XLS／XLSX／ODS 原生只读数据表格（2026-10-30，源码）

- 用户反馈电子表格转 PDF 的打印分页、空白边距和复制限制不适合查数据，要求像 TSV 一样查看，并确认公式默认显示文件保存的缓存结果、可以查看表达式，不在软件内重算。
- 新增内置 calamine 0.37.0，只读 XLS／XLSX／ODS 不再经过 LibreOffice／PDFium 或打印分页，无需用户另外安装表格读取库。Word／RTF／演示文档继续既有分页预览。普通文件打开、目录树、引用与 URL 下载后的入口均路由到新数据视图；表格不再进入 PDF 全屏逻辑。
- 复用 TSV 的表格、生成的数据文本、类型检查、搜索、可调列宽及选择复制；只读入口明确绕过编辑会话和单元格绑定，不提供修改／保存二进制表格。增加右键“复制单元格内容”，复制完整单元格字符串；表格行号从实际工作表行号 1 开始，列号和检查定位保留原工作表坐标，普通 TSV 的既有零起点行为不改。
- 支持工作表选择、“显示公式”和重载。“在LibreOffice编辑”保留，可直接打开原文档；外部保存后手动重载。后台每次读取当前选定工作表，读取过程中禁用切换／重载以避免重复重任务；关闭预览可取消后续整理，第三方底层解析中途不承诺立即中断。
- 默认显示保存的值／公式缓存结果；切换可查看 `=...`。无缓存明确显示“公式结果未缓存”及表达式，不能把缺失结果伪装为 0，已有缓存的数值 0 正常保留。提示缓存可能过期，需 Excel／LibreOffice 重算并保存后再重载。公式读取失败仍保留可读数值并显示限制；不执行公式／宏，也不写回或固化原文件公式。
- 数据视图不还原字体／颜色／合并／图表／图片／批注／打印布局，日期和自定义数值格式可能显示底层数据。加密／损坏／不兼容格式报读取错误，不改为 PDF 兜底。只接入已有 XLS／XLSX／ODS，不扩展 XLSM／XLSB 或 Apple 格式。
- 文件限 128 MiB，ZIP 文件目录最多 10000 项、声明解压总量不超过 256 MiB；完成解析后的单表显示范围限 250000 格／100000 行／1024 列，两种 TSV 数据文本各限 16 MiB。第三方解析器在进程内工作，范围检查不等同于解析前内存上限或进程资源隔离。
- README 中英文、功能参考、当前关于摘要和新增提示七语言同步；设置末尾排序、关于当前／上一版本展示及历史默认折叠／重开逻辑未改动。
- 依赖解析遇到公司网络波动：未配置代理或替换软件源；从官方 crates.io 索引和 static.crates.io 经 IPv4 下载缺失项并按 Cargo.lock SHA-256 校验，依赖已完整缓存，可继续离线检查。没有安装 Scoop／Office 或下载用户表格。
- 验证：`cargo check --tests --locked --offline` 通过（既有警告），包括缓存值 0／缺失结果、公式独立范围、坐标偏移、TSV 引号／制表符／换行和范围上限测试代码；尚未实际执行这些新测试。准备了可显式运行的忽略测试，从用户指定路径读取实际工作簿并与独立 XML 对照数据比对，默认不含用户路径和内容、不执行用户文件写入。当前未编译新 EXE、未运行 GUI，仍需按此前程序占用要求由用户关闭并再次确认构建。

### 用户样本只读结构核对

- 用户提供 `C:/Users/ekyo/Downloads/animation_state.xlsx`（74696 字节）。使用 Python 标准库只读解包 XML，生成本地 `target/animation-state-oracle.json` 作为后续 Rust 读取对照；target 被 Git 忽略，不将用户数据加入源码。源文件 SHA-256：`54100377A26135A00DA7203A8CFC6395FEB607912521AFD5DAF31102DEBB8CAA`。
- 4 个工作表：dev_animation_state 为 464 行／8 列，3708 个有效单元格；对比为 208 行／2 列，393 个有效单元格；版本标签辅助表为 35 行／1 列，35 个公式且均有缓存结果；保留词辅助表为 114 行／1 列，114 个有效单元格。这是独立 XML 结构核对，不等同于 calamine 读取、表格 UI 或公式显示运行验证。
- 待验证：实际工作簿／多页切换、公式和缓存、无缓存或损坏／加密文件、日期与格式限制、大范围／性能、选择复制与搜索、TSV 原编辑回归、外部编辑与保存重载、七语言／DPI、关于展开收起与重开默认折叠。

---
# 当前补充：在LibreOffice编辑已接入，构建因程序占用等待确认（2026-10-30）

## 外部编辑入口构建前占用检查（2026-10-30）

- 沿用用户此前编译授权进行占用检查，发现 `unity_yaml_viewer.exe`（PID 44008）正在运行，路径为 `C:/Git/UnityYAMLViewer/target/release/unity_yaml_viewer.exe`。按 AGENTS.md 停止后续构建，未自动关闭程序、未换目录构建。必须由用户关闭后再次明确确认编译，再重新检查占用。
- 本轮外部编辑入口和七语言提示已通过 `cargo check --tests --locked --offline`、1476 条翻译键／七语言字段与新 UI 标签覆盖和 `git diff --check`。未执行新的自动测试、未启动用户文档编辑或重新生成 EXE；现有 EXE 仍是 Scoop 主题 Tab 构建，不含“在LibreOffice编辑”。

---
# 当前反馈：Scoop 添加 Extras 时 GitHub 连接失败（2026-10-30）

## Scoop 软件源网络失败运行反馈（2026-10-30）

- 用户运行本轮构建后返回日志：Scoop 已进入 Git 配置 Extras 软件源流程，连接 `github.com:443` 约 21 秒后失败（Could not connect to server），退出码 1，界面能显示原始 Git／PowerShell 诊断。此反馈不表示 LibreOffice 已下载安装成功。
- 本地只读检查确认 `C:/Users/ekyo/scoop/apps/scoop/current/bin/scoop.ps1` 存在，Extras 目录及其 LibreOffice 清单不存在；读取的 Scoop 配置未设置代理，用户 Git 全局配置未见代理条目。未改变网络或代理配置、未重新发起安装、未切换到非官方软件源。下一步需可达 GitHub 的网络／用户提供的可用代理，或使用已下载的官网安装包。

---
# 当前构建：WinGet／Scoop 主题 Tab 与同行状态（2026-10-30）

## WinGet／Scoop 主题 Tab 本地构建（2026-10-30）

- 沿用用户此前“实现后再编译”的授权完成本轮构建；用户随后再次明确表示可以编译。构建前系统进程检查确认查看器未运行，目标 EXE 可独占访问，没有自动关闭程序。
- `cargo check --tests --locked --offline` 通过；`cargo test --locked --offline third_party::` 实际执行 7 项相关测试全部通过，379 项未执行。既有进程识别测试扩展覆盖 Scoop PowerShell 脚本／组件任务识别；并不代表 Scoop 官方脚本、安装流程或 UI 已运行验证。日志：`target/scoop-installer-check.log`、`target/scoop-installer-tests.log`。
- 1467 条翻译键的唯一性、七语言字段完整性、新增安装器提示覆盖与 `git diff --check` 通过。
- `cargo build --release --locked --offline` 成功，38.24 秒，8 项应用警告；日志：`target/scoop-installer-release-build.log`。产物 `target/release/unity_yaml_viewer.exe`，版本 1.2.8，25,954,816 字节；SHA-256：`4E867A46366D39A9934AE41D6A965992A3BFA35919EB7E8B958CCF4A5DFEBC65`。
- 新产物包含顶部主题 Tab、Scoop 同行检测状态／安装／重新检测、安装器选择持久化、Scoop 按需首装及 FFmpeg／LibreOffice 安装路由与路径检测。PDFium 独立下载和既有 WinGet 下载取消逻辑保留。
- 未启动 GUI、未下载或安装 Scoop／软件包；本轮首次安装、Git／extras、权限／网络、跨实例、自定义路径、七语言／DPI 的同行布局和关于历史交互待运行验证。用户对上一版授权弹窗和取消日志的反馈已单独记录，不扩展到本轮 Scoop。没有提交、推送、标签或 Release 操作。

---
# 用户运行反馈：上一版安装调用与授权取消日志通过（2026-10-30）

- 用户确认上一版能发起安装并弹出系统授权窗口，点击取消后日志显示用户取消；不表示组件已安装或 Office 转换通过。本轮 Scoop 与同行 Tab UI 仍待运行验证。

---
# 当前补充：WinGet／Scoop 主题安装器 Tab（2026-10-30，源码）

- 用户指定第三方支持顶部使用与主题一致的 Tab，选中 Scoop 后在同行显示状态并提示安装。已接入，并保存选择；选择只检测，点击安装才下载官方脚本。
- FFmpeg／LibreOffice 安装跟随选择，补充 Scoop 原生程序路径检测；PDFium 独立下载不变。安装时禁止切换，Scoop／组件共用任务互斥和外部进程检查。Scoop 暂不提供取消，保留 WinGet 已确认下载的取消重试。
- `cargo check --tests --locked --offline`、新增提示七语言字段／唯一键与静态覆盖及 `git diff --check` 通过；未执行测试、未启动安装或 GUI、未重新生成 EXE。现有 EXE 仍是此前的重复安装保护构建，不含本轮 Scoop UI。实际状态与布局、首次安装／路径／Git／extras、不同权限／网络和关于历史交互待验证。详细行为与限制见 docs/versions/1.2.8.md 最新章节。

---
# 当前构建：LibreOffice 重复安装保护及下载取消重试（2026-10-30）

## LibreOffice 重复安装保护与取消重试本地构建（2026-10-30）

- 用户明确授权“实现后再编译”；构建前通过系统进程列表确认查看器未运行，目标 EXE 可独占访问。未自动关闭用户程序。
- `cargo check --tests --locked --offline` 通过；`cargo test --locked --offline third_party::` 实际执行 7 项测试全部通过，379 项未执行，不代表全应用回归。覆盖未知状态阻止取消、下载阶段取消条件、日志有界／ANSI 清理、跨线程命名互斥及释放、文件占用与解除、外部／自身任务识别、组件缓存隔离。日志：`target/libreoffice-retry-check.log`、`target/libreoffice-retry-tests.log`。
- 1443 条翻译键的唯一性、七语言字段完整性、新增字面提示覆盖及 `git diff --check` 通过。关于版本顺序、默认两版展示和历史展开逻辑未改动；实际关于七语言布局／折叠／重开仍待 GUI 验证。
- `cargo build --release --locked --offline` 成功，41.56 秒，8 项应用警告；日志：`target/libreoffice-retry-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`；版本 1.2.8；25,885,184 字节；SHA-256：`A6373DC7F55C80FFF3BB93AC146D120B5248E91F85E631D0CF828839DDC94884`。包含本轮重复安装保护与下载取消重试，以及此前 Office 只读预览和安装阶段显示。
- 未启动 GUI、未发起 WinGet／官网安装包安装，真实跨实例／权限、下载取消与文件释放、依赖安装阶段变化、不同 WinGet 输出及 Office 转换仍待运行验证。取消仅针对本程序拥有的 WinGet，不清理下载文件或结束子安装程序；若占用未释放则不会自动开始新任务。未提交、推送、创建标签或发布 Release。

---
# 当前补充：LibreOffice 重复安装保护与下载取消重试（2026-10-30）

- 用户明确要求实现后编译。已增加跨实例互斥、系统安装进程与暂存文件占用检查；15 分钟改为长等待提示并继续监视，不再释放成可重复安装的空闲状态。
- 已确认下载阶段提供二次确认的取消并重试，只结束本程序拥有的 WinGet 进程本身；安装中、未知状态、系统查询失败或外部任务均保留并阻止重复启动。退出且文件释放后才重试，不删除系统暂存文件。
- README 中英文、当前关于摘要及新增提示七语言同步。真实 WinGet、复杂安装时序与 UI 待验证；最终测试与本地产物信息见后续记录。未自动启动组件安装。

---
# 当前补充：Office 预览与安装阶段已完成本地构建（2026-10-30）

## Office 预览与安装状态本地构建（2026-10-30）

- 用户明确授权编译；CIM 查询不可用后，使用进程列表检查无 unity_yaml_viewer 运行，并确认目标 EXE 可独占读写，再开始构建。未关闭任何用户程序、未更换输出目录绕过占用检查。
- `cargo build --release --locked --offline` 成功，耗时 47.35 秒，8 项应用警告；日志：`target/office-preview-release-build.log`。此前 `cargo check --tests --locked --offline`、七语言字段／唯一键与新增提示覆盖、`git diff --check` 已通过。
- 产物：`target/release/unity_yaml_viewer.exe`，文件版本 1.2.8，25,848,320 字节，SHA-256：`AB4C86AB1D1A9E5567900CD8700206E88077A216129EAE4130F3CF83793957E7`。包含 Office 只读预览、LibreOffice 安装期间可用性检测、阶段／WinGet 最近状态／等待时间与超时提示。
- 未执行测试、未启动 GUI 或 LibreOffice／WinGet 安装；文档真实转换、安装阶段与数值进度、失败／超时及七语言界面、关于历史折叠等仍待运行验证。未提交、推送、创建标签或更新 Release。

---
# 当前补充：LibreOffice 安装状态修复（2026-10-30）

- 用户澄清官网安装包已下载、尚未安装，先验证当前流程；没有启动安装或重新下载。
- 已增加安装阶段、WinGet 最近输出、等待时间与无新状态提示；数值进度仅按 WinGet 实际输出显示，未接入 COM API，未实现伪造百分比或可视进度条。解析测试代码仅完成编译检查、尚未执行。
- 用户反馈长期停在“正在安装”。已将无限等待 WinGet 改为安装前及安装期间检测组件可用性，检测成功直接刷新版本／路径；等待超过 15 分钟退出界面等待并提示检查系统安装，不强制结束安装程序。
- 编译检查、七语言字段／唯一键检查和 diff 空白检查通过；未构建 EXE、未启动安装或运行 GUI。当前运行程序仍需后续取得用户明确构建授权并检查占用后更新。具体行为与待验证项见 docs/versions/1.2.8.md。

---
# 当前补充：Office 只读预览源码接入（2026-10-30）

- 用户明确先只读，编辑后续接入；当前版本保持 1.2.8，已有 Release 不变。
- 新增 `src/asset_viewer/office_preview.rs`，复用 LibreOffice 路径检测与 PDFium 查看器，支持 Word／Excel／PowerPoint、OpenDocument、RTF；转换临时副本，表格按打印布局分页。详细限制与待验证项见 `docs/versions/1.2.8.md` 的发布后新增记录。
- `cargo check --tests --locked --offline` 通过；未获生成可执行程序授权，未构建 EXE、未运行测试、未启动 GUI，也未安装组件。真实转换、取消清理、宏与链接设置、各格式排版及七语言界面待验证；后续构建必须先取得用户明确确认并检查程序占用。保留本轮之前所有工作区改动，未提交、未推送或发布。

---
# 当前交接：1.2.6 已正式发布（2026-10-21）

## GitHub 正式发布（2026-10-21）

- 用户确认 Git 已提交、GUI 验证已完成并明确授权发布。远端 master、发布目标提交及构建输入摘要核对一致：b6ec1b30923b285c6605986694af84194d636fc7。
- [v1.2.6](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.6) 于北京时间 2026-10-21 01:33:13 正式发布；标题仅 v1.2.6，非草稿 / 非预发布，latest API 核对为本版。Release ID：418679687。
- 附件 UnityYAMLViewer-v1.2.6-windows-x64.exe，24,288,768 字节；GitHub asset.digest 与本地 SHA-256 一致：F8B69F36AF2F72F4B81C5DD74ED4506DEE36F2BC0901D2082E712840BC7F297B。
- 草稿阶段完成上传和摘要核验后转为正式发布；标签 v1.2.6 指向上述提交。未重新编译，未改变旧版本标签或附件。发布证据：target/v1.2.6-published.json。
- 23 项相关自动测试通过；GUI 验证由用户确认完成。未执行全部自动测试，未额外宣称全格式透明视频兼容性已验证。
- 发布后状态仅更新本地文档，未自动提交或推送。

---
# 当前交接：1.2.6 已构建，等待用户提交后通知发布（2026-10-21）

## 本轮 1.2.6 构建与验证（2026-10-21）

- 用户明确授权升级与编译，编译前检查程序未运行、目标 EXE 可独占访问，未自动关闭程序。
- cargo check --tests --locked --offline 通过；23 项相关测试实际执行并通过：连续图 18、字幕逻辑 3、本地化 2；未执行其他 311 项，不代表完整应用回归。日志：target/v1.2.6-asset_viewer-seamless-tests.log、target/v1.2.6-subtitle_player-tests-tests.log、target/v1.2.6-localization-tests-tests.log。
- cargo build --release --locked --offline 成功（23.07 秒，8 项应用警告）；日志：target/v1.2.6-release-build.log。Cargo.toml / Cargo.lock / EXE 版本均为 1.2.6；1126 条七语言字段与唯一键、关于版本顺序静态核对及 git diff --check 通过。
- 产物：target/release/unity_yaml_viewer.exe；大小：24288768 字节；SHA-256：F8B69F36AF2F72F4B81C5DD74ED4506DEE36F2BC0901D2082E712840BC7F297B。
- README 以 1c96245 精简版为模板，维持 101 行；不含版本日志或发布流程状态，细节移入功能参考。
- 未运行最终 GUI；关于展开 / 收起和重开、字体、透明视频、布局及背景恢复仍待运行验证。源码由用户自行提交 Git，等待用户提交后通知再发布；未提交、未推送、未创建标签或 Release、未上传附件。

- 版本记录：docs/versions/1.2.6.md；发布说明草稿：docs/releases/1.2.6.md。未来 Release 显示标题只能为 v1.2.6；发布前核对用户提交与本次构建源码一致。

---
# 当前交接：1.2.5 媒体详细信息与摘要本地构建（2026-10-21）

### 媒体详细信息与摘要本地构建（2026-10-21）

- 用户明确授权编译；编译前确认程序未运行、目标 EXE 可独占访问。cargo build --release --locked --offline 成功（23.68 秒，8 项应用警告）；日志：target/media-details-release-build.log。
- 包含文件 / 视频 / 音频 / 字幕分组详情、逐轨音频参数与音乐标签；视频页基础摘要及两行直接详细信息入口；纯音频所选音轨摘要、详细入口与音轨切换逻辑同步。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24285184 字节；SHA-256：96C1194759813FEE5453B606AAFB4BB43AAF59BB713660CB328A9C13584F9AF6。
- git diff --check 通过；未启动 GUI、未重复执行测试；菜单布局、不同格式元数据、多音轨切换及七语言待运行验证。未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 字幕与播放界面本地构建（2026-10-21）

### 字幕与播放界面本地构建（2026-10-21）

- 用户明确授权编译；编译前确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（21.54 秒，8 项应用警告）；日志：target/media-ui-release-build.log。
- 包含连续图另存 120 × 40 / 字号 16 且紧随两行文字、最近打开限高滚动及底部固定清空、关于作者署名、字幕样式系统字体选择与底部居中按钮 / 默认宽度 320，以及透明 GIF / 视频棋盘底图。缓存统计已按用户决定恢复自动逻辑。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24216064 字节；SHA-256：15907B22629FAC6214B70D8E51B2B03442429CD110188FC34203658DB91D4630。
- git diff --check 通过；本轮未启动 GUI、未重复执行测试；系统字体实际应用、菜单滚动、透明 MOV / WebM 解码及界面布局待运行验证。未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 统一背景设置与连续图布局本地构建（2026-10-21）

### 统一背景设置与连续图布局本地构建（2026-10-21）

- 用户明确授权编译；编译前确认程序未运行、目标 EXE 可独占访问。cargo build --release --locked --offline 成功（23.20 秒，8 项应用警告）；日志：target/shared-background-release-build.log。
- 包含各类预览共用背景模式与自定义颜色、初始自定义色黑色及持久化；连续图背景色选项移至模式后方，另存按钮与两行标题整体对齐。显示标签统一为背景色。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24195584 字节；SHA-256：ACC0A4D78FB84B228B9D6558D865619C2F827A80F2DFB400D477F52098AA77B4。
- git diff --check 通过；未运行 GUI，跨窗口同步、重启恢复、记录写入失败及标题布局仍待运行验证。本轮未重复执行测试，未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 主题 Tab 与工具栏布局本地构建（2026-10-20）

### 主题 Tab 与工具栏布局本地构建（2026-10-20）

- 用户明确授权，编译前确认程序未运行、目标 EXE 可独占访问。cargo build --release --locked --offline 成功（21.68 秒，8 项应用警告）；日志：target/preview-toolbar-release-build.log。
- 包含图片通道及三处背景的主题 Tab、图片工具按钮移至文件名后、静态预览排序轴主题 Tab 与紧凑辅助线开关；保留此前预览像素和背景修复。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24238080 字节；SHA-256：39D9168B82467EC8D34817CDCD0913CCD078C3269F4BB8CE6EE3CE84C0A798CE。
- git diff --check 通过；未启动 GUI，布局、Tab 交互、辅助线开关、七语言及不同 DPI 待运行验证。本轮未重复执行已有测试，未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 预览清晰度与背景选项本地构建（2026-10-20）

### 预览清晰度与背景选项本地构建（2026-10-20）

- 用户明确授权编译，编译前检查程序未运行且目标 EXE 可独占访问。cargo build --release --locked --offline 成功（23.95 秒，8 项应用警告）；日志：target/preview-background-release-build.log。
- 包含连续图小图保留原始像素、超限等比最近邻缩小和三栏棋盘格对齐；普通图片移除固定倍率按钮，普通图片 / 九宫格 / 连续图新增背景选择；连续图使用绿色另存大按钮。
- cargo test --locked --offline preview_pixel_tests 实际执行 2 项并全部通过（小图 RGBA 保真、超限等比且不插值）；其余 332 项未执行。日志：target/preview-pixel-tests.log。git diff --check 通过。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24132608 字节；SHA-256：73A80E84EBCB96BFBEBBD00A9F0E6C5D0B55288F0D2A99BCBDF1BEE1590A777E。
- 未启动 GUI；三栏清晰度、背景切换、按钮布局、七语言与不同 DPI 待运行验证。未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 像素对齐棋盘格本地构建（2026-10-20）

### 像素对齐棋盘格本地构建（2026-10-20）

- 用户明确授权，编译前确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（21.51 秒，8 项应用警告）；日志：target/pixel-grid-release-build.log。
- 包含普通图片最近邻采样、800% 起棋盘格每格对齐一个源像素、低倍固定格大小及每帧拖动位移修正；保留此前九宫格标签与覆盖保存修复。
- 产物：target/release/unity_yaml_viewer.exe；文件版本：1.2.5；大小：24113152 字节；SHA-256：07787D3AEF306DEE60BB06D25F366D0623A35534C87F714DEFCE76C57BE40339。
- git diff --check 通过；本轮未启动 GUI，阈值切换、缩放和平移对齐及不同 DPI 待运行验证。未上传或更新 GitHub Release。

---
# 当前交接：1.2.5 发布后预览与另存修复本地构建（2026-10-20）

### 预览调整本地编译（2026-10-20）

- 用户明确授权，编译前确认程序未运行且目标 EXE 可独占访问。cargo build --release --locked --offline 成功（21.30 秒，8 项应用警告）；日志：target/preview-release-build.log。
- 包含普通图片最近邻采样、连续图与普通预览棋盘背景无边缘抗锯齿、九宫格上下标签移至左右，以及前述覆盖保存修复。
- 产物：target/release/unity_yaml_viewer.exe；版本：1.2.5；大小：24118784 字节；SHA-256：E99B4749515F1F9D3460476B7138CFEA9B7C0A10EF5B260893DF12303717465A。
- 未启动 GUI，黑白细线是否消除、标签布局、确认弹窗及七语言实际显示仍待运行验证；未上传 Release。

---
# 当前交接：1.2.4 已正式发布（2026-10-18）

北京时间 2026-10-18 23:49:41，用户确认提交并明确授权发布后，GitHub v1.2.4 正式发布。标题仅 v1.2.4，非草稿 / 非预发布，latest API 已核对为 v1.2.4。

发布页：https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.4 ，Release ID 416542539。远端 master 已实时核对，标签指向用户提交 981b6c39e846d4eecdc0818870a11ef412662033；与构建基线 bc0e243 之间仅文档变化。

沿用现有 Release EXE，未重新编译。附件 UnityYAMLViewer-v1.2.4-windows-x64.exe，23,726,080 字节，版本 1.2.4，SHA-256 A43EFC4E0372110CB3EE4148DC9EFDC99F0D4848EB0513BEC55DBBBDE0638887；GitHub asset.digest 与本地一致。证据 target/v1.2.4-published.json。

版本记录、索引、CHANGELOG、发布说明和本交接已更新真实发布状态；这些发布后文档改动尚未提交 / 推送，由用户自行同步。不重打标签或重传附件，原有 GUI / 测试执行与性能验证边界仍保留。下方待提交 / 未发布说明为历史阶段，以本节为准。

---
# 当前交接：1.2.4 文档与附件准备完成，等待用户提交（2026-10-18）

本节优先于下方历史阶段记录。用户确定本版功能范围，由用户自行提交 Git；用户表示提交后可以发布。**本轮只准备文档及现有 EXE 的同字节发布附件，不提交、不推送、不打标签、不发布。后续须等用户确认提交完成，并核实远端已包含目标提交。**

## 最终资料

- 整理前工作区干净，源码基线 bc0e243（增加 csv,tsv,psv 表格查看和编辑,格式验证,查找定位）。本轮仅修改文档并复制已有产物，未重新编译。
- 最终功能、限制和验证边界：[版本记录](docs/versions/1.2.4.md)；对外更新内容：[发布说明](docs/releases/1.2.4.md)。逐轮原始记录完整保留到 [开发归档](docs/versions/archive/1.2.4-development-history.md)。索引与 CHANGELOG 已同步。
- Cargo / EXE 版本 1.2.4。最新构建日志 target/table-json-kind-release-build.log；Release 构建成功，7 条既有应用警告。
- 原 EXE：target/release/unity_yaml_viewer.exe；发布附件：target/release/UnityYAMLViewer-v1.2.4-windows-x64.exe。
- 两份产物均为 23,726,080 字节，SHA-256 A43EFC4E0372110CB3EE4148DC9EFDC99F0D4848EB0513BEC55DBBBDE0638887。附件在 target 下，不需提交 Git。
- 用户多轮截图反馈已用于修正布局；代理未运行最终完整 GUI 回归，新增测试仅完成 cargo check --tests，未执行测试二进制。发布授权不能替代全部语言 / DPI、多轨字幕导出、大表性能等验证，边界保留在版本记录。
- README 保持中英文实际支持 / 限制，不添加发布流程或逐版本日志。关于窗口当前 1.2.4 / 上一版 1.2.3，其他历史默认折叠；七语言摘要保留，本轮未改源码或重打 EXE。

## 用户提交后的发布步骤

1. 用户确认已提交后，读取新的目标提交和工作区状态；检查本轮文档已包含，实时核实远端分支包含该提交，不仅依据本地 origin 缓存。若尚未推送，先让用户完成同步，不发布远端缺失的提交。
2. 复核 Cargo 版本、两份 EXE 的文件版本 / 字节数 / SHA-256、发布附件及 docs/releases/1.2.4.md。沿用现有 EXE，不自动重编译；若用户授权新编译仍需先检查占用。
3. 检查 v1.2.4 标签及 Release 是否已存在，避免覆盖。按照本会话用户提交后发布的授权，标签指向已核实的提交，使用上述更新说明和附件。Release API 的 name 只允许 v1.2.4，正式发布而非草稿 / 预发布。
4. 核对标签实际提交、Release 状态、Latest、附件名 / 大小及 GitHub asset.digest。真实发布成功后再更新版本记录、索引、CHANGELOG、发布说明与本交接的发布时间 / 链接，不把构建成功写成已发布。

---
# 当前交接：1.2.3 已正式发布（2026-10-18）

北京时间 2026-10-18 13:30:30，用户确认已推送后，GitHub v1.2.3 正式发布，非草稿 / 非预发布，已核对 latest API 为 v1.2.3。发布页 https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.3 ，Release ID 416256040，目标提交 eab07a7b741eeb188c8b08a2b2eefa3a51c6d2df。

沿用验收 EXE，未重新编译。附件 UnityYAMLViewer-v1.2.3-windows-x64.exe，23,282,176 字节，SHA-256 F634207A4470CCFF9343787BA25288364D29F18AF15C02F0F6C44164A437F358，GitHub asset.digest 和本地一致。发布证据 target/v1.2.3-published.json。

版本记录、索引、CHANGELOG、发布说明和本交接已补记真实发布状态；这些是发布后的本地文档修改，未自动提交 / 推送，用户可自行同步。不需要重打标签或重发附件。此前等待推送 / 未发布描述为历史状态，以本节为准。验证边界与功能限制仍保留。

---
# 当前交接：1.2.3 发布准备完成，等待用户推送确认（2026-10-18）

本节优先于下方所有历史阶段状态。用户确认最新视频 / GIF 整框播放交互已验证并授权正式发布；为避免推送权限问题，用户自行提交 / 推送本轮文档。**当前不能创建标签或 GitHub Release，必须等用户明确确认推送完成。** 本轮未提交、未推送、未打标签、未发布，未重新编译。

## 文档与产物

- 源码基线提交 b161773；整理前工作区干净，本轮只修改文档。Cargo.toml / Cargo.lock 与本地 EXE 都为 1.2.3。
- 最终功能 / 限制 / 验证：[版本记录](docs/versions/1.2.3.md)；对外更新内容：[发布说明](docs/releases/1.2.3.md)。版本索引与 CHANGELOG 已链接，状态为待用户推送 / 未发布。
- 原版逐轮记录完整保留到 docs/versions/archive/1.2.3-development-history.md，升级前记录仍在 archive/1.2.3-development.md；旧记录的待实现 / 旧哈希只代表当时阶段。
- 沿用最后一次已验收 Release：target/release/unity_yaml_viewer.exe，23,282,176 字节，文件 / 产品版本 1.2.3，SHA-256 F634207A4470CCFF9343787BA25288364D29F18AF15C02F0F6C44164A437F358。构建日志 target/media-canvas-click-release-build.log，19.78 秒，保留既有警告。
- 已复制并复核发布附件 target/release/UnityYAMLViewer-v1.2.3-windows-x64.exe，同字节数 / SHA-256；target 通常忽略，不要求用户把 EXE 提交 Git。不得误用历史体验构建。
- 最新用户确认覆盖整框点击；此前定位文件 / 复制地址及页签堆叠有明确验收。其他复杂场景的待验证边界保留在最终版本记录，发布授权不等于全面回归通过。

## 用户推送后的发布步骤

1. 用户提交并推送本轮文档后明确告知。再检查工作区、远端 master 与本地目标提交一致，并确认包含 b161773 及整理后的文档；不要凭当前本地 origin/master 缓存宣称远端已同步。
2. 复核 Cargo 版本、EXE 版本 / 字节数 / SHA-256、附件命名及 docs/releases/1.2.3.md。不重新编译，除非用户再次明确授权；授权编译也必须先检查程序进程及 EXE 独占访问。
3. 检查 v1.2.3 标签 / Release 是否已存在，避免覆盖。创建指向已确认推送提交的 v1.2.3，使用本版发布说明和上述验收附件，正式发布而非预发布 / 草稿。
4. 核对发布页、最新正式版、目标标签提交、附件名 / 大小 / GitHub asset.digest。成功后才将版本记录、索引、CHANGELOG、发布说明和本交接更新为真实发布时间 / 链接；如后续文档仍需推送而无权限，交给用户处理，不伪报同步。

本轮只整理文档和复制已有附件，README 仍按实际已支持 / 限制维护，不加入内部发布流程。源码中的关于窗口保持默认 1.2.3 / 1.2.2，历史默认折叠，七语言保留；本轮没有改变源码或已有产物。

---

## 视频 / GIF 整框点击 Release 构建（2026-10-18）

- 用户明确授权后，相关程序进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（19.78 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23282176 字节，SHA-256：F634207A4470CCFF9343787BA25288364D29F18AF15C02F0F6C44164A437F358。日志 target/media-canvas-click-release-build.log。
- 包含视频 / GIF 整个预览画布（含留白）左键单击播放 / 暂停，保持自由缩放拖动阈值和播放列表控件排除区域。
- 未启动 EXE，实际留白单击、拖动与控件交互待 GUI 验证；未运行测试断言，未提交或发布。
---

## 视频 / GIF 整框点击播放暂停（2026-10-18）

- 播放暂停的左键单击命中从实际画面扩大到整个可见预览画布，包括上下 / 左右留白，适应窗口、原尺寸、自由缩放和全屏共用。
- 自由缩放保持原有按下 / 移动 / 松开手势及拖动阈值，拖动画布不触发播放切换；松开在框外不触发。播放列表浮层 / 外部控件排除区域继续生效，进度条不在预览画布内。右键菜单和滚轮缩放保留。
- README 中英与当前关于摘要七语言同步。cargo check --tests --locked --offline、git diff --check 及翻译七列 / 唯一键检查通过，日志 target/media-canvas-click-check.log。未构建 EXE，实际留白单击、拖动、全屏和播放列表控件待 GUI 验证。

---

## 文件右键菜单用户验证（2026-10-18）

- 用户在最新文件右键菜单 Release 构建后明确反馈“已验证”，据此记录本轮右键菜单、打开文件所在目录并选中文件、复制文件地址功能用户验证通过。
- 对应 1.2.3 本地产物 SHA-256：AF8B9E49E2D72002E660F6E466A28FD457D708AFBCFF3D1ADE65D5605FBE9F80。该反馈不扩展为其他历史待验证功能、全部语言 / DPI 或所有路径场景均已验证；未发布。
---

## 文件右键菜单 Release 构建（2026-10-18）

- 用户明确授权后，相关程序进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（20.16 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23281152 字节，SHA-256：AF8B9E49E2D72002E660F6E466A28FD457D708AFBCFF3D1ADE65D5605FBE9F80。日志 target/file-location-menu-release-build.log。
- 包含页签、堆叠文件项和预览文件名的右键定位 / 复制地址；定位使用资源管理器选中目标文件。
- 未启动 EXE，实际右键菜单、Explorer 选中与剪贴板操作待 GUI 验证；未运行测试断言，未提交或发布。
---

## 文件定位与地址右键菜单（2026-10-18）

- 页签（堆叠标题对应当前代表文件）、堆叠菜单每个文件名、图片 / PDF / 媒体预览文件名新增右键菜单：打开文件所在目录、复制文件地址。Explorer 使用 /select, 选中目标文件，含空格 / 中文路径通过独立参数传递，不使用 shell 拼接。
- 缺失文件禁用定位，仍可复制路径；网络资源对应本地缓存文件，不作为原始 URL。定位启动失败在菜单中显示错误。README 中英、关于与菜单七语言同步。
- cargo check --tests --locked --offline、git diff --check、七列及唯一键检查通过，日志 target/file-location-menu-check.log。未构建 EXE，实际 Windows Explorer 选中及多层菜单交互待 GUI 验证。
- 另完成真实 GIF 帧时长只读核验：D:/Image/66b3de17ly1hzt5zili3qg20b40b4x6u.gif 共 877 帧，原始 GCE 延时与 FFprobe 一致：0.030 秒 584 帧、0.040 秒 292 帧、0.200 秒 1 帧。前八帧 3,4,3,3,4,3,3,4 百分之一秒，显示可变帧时长符合源文件。

---

## GIF 进度格式 Release 构建（2026-10-18）

- 用户明确授权后，相关程序进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（22.46 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23211008 字节，SHA-256：04EFD78F056953155BA48345A61807DFCF27660118D1FDCAF03602ECF2146F65。日志 target/gif-progress-release-build.log。
- 包含 GIF 进度和悬停提示格式 h:mm:ss.mmm(f00)/h:mm:ss.mmm(f00) · 0.030 s/f；f 前置，与视频一致，f / s/f 不翻译。原有按帧实际时长分布的进度条刻度保留。
- 未启动 EXE，实际进度文字、悬停、窄窗口与刻度显示仍待 GUI 验证；未运行测试断言，未提交或发布。
---

## GIF 进度时间与固定单位（2026-10-18）

- GIF 进度文字及悬停提示统一为 当前时间(当前帧f)/总时间(总帧数f) · 0.030 s/f。时间为 h:mm:ss.mmm，帧号从 1 开始、至少两位，不截断大帧号；f 在数字后，s/f 固定不翻译。视频 / 普通音频格式不变。
- 可变帧率 GIF 的 s/f 仍取当前帧实际时长；时间索引未就绪时保留时间和已知帧数，显示 — s/f，不用总时长猜测当前帧时长。
- README 中英及关于摘要七语言同步。cargo check --tests --locked --offline、git diff --check 及七列 / 唯一键检查通过，日志 target/gif-progress-check.log。未构建 EXE，实际窄窗口显示、播放 / 拖动与悬停提示待 GUI 验证，未发布。

---

## PDF 过渡菜单 Release 构建（2026-10-18）

- 用户明确授权后，相关进程检查及目标 EXE 独占读写通过；cargo build --release --locked --offline 成功（19.95 秒，保留既有警告）。本次恢复仅核对上次已完成产物，未重复构建。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23268864 字节，SHA-256：C7D471CA187BA40A3F3229B995E3731126904F1D4AC05B999FB436EB625B10E7。日志 target/pdf-transition-release-build.log。
- 包含单页隐藏自动翻页设置、翻页保留旧画面、超过 1 秒才显示加载提示，以及六项过渡菜单。中文首项仅显示“无”，不附加英文或默认标记；英文语言显示 None。
- 未启动 EXE，实际过渡动画、不同页面尺寸、加载提示与单页布局待 GUI 验证；未运行测试断言，未提交或发布。
---

## PDF 翻页过渡下拉菜单（2026-10-18）

- 按用户最新要求，循环后使用“过渡”下拉菜单，第一项 / 默认“无”（英文 None），其后为淡入淡出、左右滑动、推页、擦除、缩放淡入。不开启翻书效果。单页与页数未知时随自动翻页设置一起隐藏。
- 非无效果时，新页面就绪才捕获旧纹理并开始 0.25 秒平滑动画；保留原先等待时显示旧页及超过 1 秒才显示转圈。手动 / 自动 / 全屏共用效果；滑动、推页和擦除根据页码前后决定方向，循环末页回首页按向前页码方向处理。
- 动画最多额外保留一张旧页面纹理，结束或切换样式后释放；缩放刷新不触发翻页动画。不同尺寸页面在新页面矩形内过渡，旧纹理短暂适配到新矩形，不是纸张卷曲或透视翻书。关闭 / 重载按现有生命周期清理。
- README 中英、关于摘要与选项七语言同步。cargo check --tests --locked --offline、git diff --check 及七列 / 唯一键检查通过，日志 target/pdf-transition-check.log。未生成 EXE，实际五种效果、不同尺寸与 DPI、全屏和循环的视觉与性能待验证，未发布。

---

## PDF 翻页保持画面与延迟加载提示（2026-10-18）

- 翻页不再清空当前纹理 / 尺寸；独立记录请求页和已显示页，即使新旧页尺寸相同仍请求新页面。新页结果成功后统一更新纹理、尺寸、页码，并重置平移；加载期间保持上一页画面与页码，避免空白闪烁。自动 / 方向键 / 圆形按钮翻页共用此行为。
- 每次后台渲染记录单调时钟起点，等待达到 1 秒后才显示转圈；首次打开无画面时同样延迟，保留 100ms 重绘以便跨过阈值；有旧画面时转圈叠在画布左上，不替换旧页面。
- README 中英和关于摘要七语言同步。cargo check --tests --locked --offline、git diff --check、七列和唯一键静态检查通过，日志 target/pdf-page-transition-check.log。未生成 EXE，实际连续翻页、不同纸张尺寸、慢加载与全屏仍待 GUI 验证，未发布。

---

## 单页 PDF 隐藏自动翻页设置（2026-10-18）

- 仅页数大于 1 时显示文件名后的自动翻页开关、秒数 / 单位和循环开关。页数未知及单页隐藏整组设置，单页停止自动计时，避免无意义刷新。文件名及其他预览工具保留。
- README 中英及当前关于摘要七语言同步。cargo check --tests --locked --offline、git diff --check、七列与唯一键检查通过，日志 target/pdf-single-page-check.log。未构建新 EXE，实际单页 / 多页及加载过渡 GUI 待验证。

---

## PDF 循环与重载按钮修复 Release 构建（2026-10-18）

- 用户明确授权后，相关程序进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（19.86 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23318016 字节，SHA-256：21102A7C73516DB6137C5B01D2F523E988B4EE07CF2549335C8BDD1506CEA4FF。日志 target/pdf-loop-release-build.log。
- 包含秒数后的循环开关、开启后末页回到第一页（全屏同样生效），以及缩放 / 翻页期间重载按钮保持稳定、在途渲染完成后处理合并的重载请求。
- 未启动 EXE，实际开关布局、循环 / 末页行为和重载按钮视觉待 GUI 验证；未运行测试断言，未提交或发布。

---

## PDF 自动翻页循环开关（2026-10-18）

- 在自动翻页秒数 / 单位之后新增“循环”，复用现有主题开关样式，默认关闭，不持久化。自动翻页开启时，循环开启会在末页停留完整间隔后回到第一页继续；关闭循环则在末页停止。单页 PDF 循环保持当前页，不重复渲染。
- 切换循环重置计时；循环本身不启动自动翻页，全屏沿用当前设置。README 中英及关于摘要七语言同步。
- cargo check --tests --locked --offline、git diff --check 及七列 / 唯一键检查通过，日志 target/pdf-loop-check.log。未构建 EXE，实际开关布局 / 全屏循环与末页行为待 GUI 验证，未发布。

---

## PDF 重载按钮闪烁修复（2026-10-18）

- 原因是缩放 / 翻页的每次后台渲染都将“重载”切换为禁用，任务完成后恢复启用。按钮改为保持启用与稳定样式；渲染期间点击重载只登记一次请求，当前任务结束后再重载，避免叠加原生任务或旧结果覆盖新状态。
- 当前关于摘要七语言同步；README 能力范围不变。cargo check --tests --locked --offline、git diff --check 及翻译静态检查通过，日志 target/pdf-reload-button-check.log。未构建新 EXE，连续缩放 / 翻页及渲染中重载的实际 GUI 待验证。

---

## PDF 清晰度与自动翻页 Release 构建（2026-10-18）

- 用户明确授权后，相关程序进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（21.28 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23321600 字节，SHA-256：412A96CB4A19193B9CA0AD7B9A3730630743788B5DC3730B187FB1B29F240F4A。日志 target/pdf-auto-page-release-build.log。
- 包含 PDF 100% 的 96 DPI 基准换算、物理像素对齐、屏幕文字抗锯齿，以及文件名后现有样式的自动翻页开关、默认 5 秒及 1–3600 秒设置，全屏继续、末页停止。
- 未启动 EXE，实际清晰度与 Adobe 对照、定时翻页、末页停止和 GUI 布局待运行验证；未运行测试断言，未提交或发布。

---

## PDF 自动翻页（2026-10-18）

- 文件名同行后新增“自动翻页”，复用 theme::toggle_switch 开关，跟随当前主题；后接整数秒输入和单位，默认 5 秒，范围 1–3600 秒，默认关闭，不持久化。
- 开启后对当前显示的 PDF 定时向后翻页，全屏隐藏标题栏但继续计时；末页停留一个间隔后关闭自动翻页，不循环。渲染期间暂停计时，出现错误停止；修改开关 / 间隔或手动翻页重置累计时间。离开预览不后台翻页、不补跳漏过的页面。
- 自动翻页不改变文件，不执行 PDF 编辑。README 中英与关于摘要七语言同步。
- cargo check --tests --locked --offline、git diff --check 及七列 / 唯一键检查通过，日志 target/pdf-auto-page-check.log。未构建 EXE，实际定时 / 全屏 / 多页末页 / 鼠标输入与主题待 GUI 验证，未发布。

---

## PDF 100% 清晰度与 DPI 修正（2026-10-18）

- 修正 PDFium 返回的 1/72 英寸页面尺寸直接作为 UI 逻辑像素的问题：转换为 96 DPI 基准，再叠加当前窗口 pixels_per_point；100% 页面宽高约为此前的 4/3，适应窗口仍按可用尺寸计算，不承诺与 Adobe 自定义显示 DPI 设置完全一致。
- 页面绘制起点与显示尺寸对齐实际物理像素，减少半像素平移及纹理二次缩放模糊；启用 FPDF_ANNOT | FPDF_LCD_TEXT（屏幕文字抗锯齿）。来源依据 PDFium public/fpdfview.h 的页面单位与渲染标志定义。
- 单边 4096 上限保持；超过该上限仍会放大有限分辨率纹理，内嵌低分辨率扫描图无法恢复原本不存在的细节。文字与图片、不同 DPI 下的实际观感及 Adobe 同文档对照尚待运行验证，不能宣称已达到相同清晰度。
- README 中英及关于七语言同步。cargo check --tests --locked --offline、差异与翻译静态检查通过，日志 target/pdf-sharpness-check.log。未构建 EXE或执行 GUI，未发布。

---

## PDF 悬浮翻页与全屏 Release 构建（2026-10-18）

- 用户明确授权编译后，检查无相关程序进程，目标 EXE 独占读写通过；cargo build --release --locked --offline 成功（19.44 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23296512 字节，SHA-256：952C8597A9F761B2884B0B728861545BC0F3890528EC6C8CA5D3B67B010789B2。日志 target/pdf-fullscreen-release-build.log。
- 包含底部页码 / 缩放悬浮显示、等高圆底矢量翻页按钮、上 / 左上一页及下 / 右下一页，以及重载后的全屏按钮、F11 切换和 Esc 退出，黑底全屏保留底部控件。
- 未启动 EXE，实际浮层布局、DPI、翻页及 Windows 全屏交互待 GUI 验证；未运行测试断言，未提交或发布。

---

## PDF 黑底全屏查看（2026-10-18）

- “全屏”按钮放在“重载”之后，F11 同样可进入。全屏隐藏主工具栏、页签和 PDF 顶部工具栏 / 说明，以黑色背景显示页面，保留底部页码 / 缩放浮层、圆形翻页按钮、方向键、滚轮缩放及拖动。
- 再按 F11 或 Esc 退出；切换到其他文件也退出 PDF 全屏。保留当前页码、缩放及平移状态，适应窗口模式随全屏可用尺寸重新计算。PDF 全屏使用独立状态，不改变媒体全屏标志。
- README 中英文、关于摘要七语言同步。cargo check --tests --locked --offline、git diff --check 和七列 / 唯一键静态检查通过；日志 target/pdf-fullscreen-check.log。未生成 EXE，实际 Windows 全屏、快捷键、底部控件和恢复窗口待 GUI 验证，未发布。

---

## PDF 底部悬浮翻页与方向键（2026-10-18）

- 移除顶部翻页 / 页码输入控件，底部居中显示“当前页/总页数 缩放%”；左右各独立半透明黑色圆底按钮，间隔 12，直径与信息底板高度一致。复用图片缩放浮层的 13pt 字体、水平 12 / 垂直 6 内边距及底部 8 间距。
- 三角形用矢量多边形绘制，以几何重心对齐圆心；边界或加载期间按钮置灰。左 / 上键上一页，右 / 下键下一页，不越界；输入控件获得键盘焦点时不抢方向键。翻页重置平移，保留缩放模式。顶部适应窗口 / 宽度、固定倍率及重载保留。
- README 中英及当前关于摘要七语言同步。cargo check --tests --locked --offline 通过（target/pdf-floating-nav-check.log），git diff --check 及七列 / 唯一键静态检查通过。未构建 EXE或运行 GUI，按钮中心、浮层位置、DPI、方向键及多页切换实际交互待验证。

---

## 第三方检测同步与主题标题 Release 构建（2026-10-18）

- 用户明确授权后，相关进程检查及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（19.65 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23308288 字节，SHA-256：0963AA18B7529FB9A7EEB02CCE24D44D84CA75A32F4D3D455AD388C1776132F1。日志 target/third-party-refresh-release-build.log。
- 包含 PDFium / FFmpeg 安装后检测与重新检测结果同步、PDF 失败预览重试，以及深色亮蓝 / 浅色深蓝组件标题。关闭第三方窗口不再触发额外检测或刷新；已启动任务关闭后仍可完成结果同步。
- 未启动 EXE，实际安装 / 检测同步、PDF 渲染和深浅主题显示待 GUI 验证；未运行测试断言，未提交或发布。

---

## 组件检测结果同步时机调整（2026-10-18）

- 按用户最新要求取消第三方窗口关闭时的检测 / 主窗口刷新。FFmpeg 安装后执行检测，PDFium 下载登记后再次读取并校验实际安装；安装后检测及“重新检测”结果到达时同步主窗口一次。未安装 / 检测失败结果同样触发一次结果同步，不循环重试。
- FFmpeg 通知移至主线程接收结果时；PDFium 沿用 Manager 接收完成后的单次通知。已关闭窗口时后台任务仍可完成同步，关闭动作本身不触发同步。
- README 中英及关于摘要七语言同步。真实安装 / 重新检测及 GUI 待验证，未构建新 EXE或发布。本节取代上方历史关闭刷新行为。

---

## FFmpeg 检测生命周期迁移补齐（2026-10-18）

- FFmpeg 检测由第三方管理窗口打开 / 关闭触发，设置窗口只提供入口；后台任务在管理窗口关闭后仍由 Manager 轮询接收。安装中关闭不另启并行检测，沿用任务完成通知。
- 每次成功检测到 FFmpeg / FFprobe 都发送就绪通知，主窗口立即重试当前失败媒体，不再等待第三方窗口关闭。保留 PDFium 就绪 / 关闭重试逻辑。
- README 中英、当前关于摘要七语言同步。cargo check --tests --locked --offline 通过；实际 WinGet 安装、手动安装后重新打开 / 关闭窗口及主界面恢复仍待 GUI 验证，未重新构建 EXE或发布。

---

## PDF 组件就绪与窗口关闭刷新修复（2026-10-18）

- 修复 PDFium 安装完成后 PDF 页签仍停留于缺组件提示：安装或检测成功通知主窗口刷新并重试失败的 PDF 预览，不要求先关闭第三方支持窗口；关闭管理窗口也触发一次重试，以覆盖手动安装。
- 重试通知保留到已有后台渲染完成后再消费，防止旧任务稍后返回缺组件错误。成功预览不重新加载；无组件或文件仍有错误时只重试一次，不持续循环。FFmpeg 原有通知处理保留。
- README 中英文与当前关于摘要七语言同步。cargo check --tests --locked --offline 通过，日志 target/pdf-component-retry-check.log；实际安装 / 关闭窗口后的 GUI 刷新仍待验证。本次未重新构建 EXE，最新 EXE 不含本修复，未发布。

---

## PDFium 只读预览 Release 构建（2026-10-18）

- 用户明确授权编译后，检查无相关程序进程，目标 EXE 独占读写通过；cargo build --release --locked --offline 成功（20.38 秒，保留既有警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.3，23301120 字节，SHA-256：93A9EC56F241F80E2ABB7A711C1171A913C318CE0BA6AF40CF067D4BF6EDA254。日志 target/pdf-preview-release-build.log。包含第三方支持管理与 PDFium 只读多页预览。
- 未启动 EXE，真实 PDFium 下载 / DLL 加载、PDF 渲染及翻页缩放等 GUI 操作仍待验证；未运行测试断言，未提交或发布。

---

# 最新接续：PDFium 只读多页预览源码（2026-10-18）

已按用户要求接入只读 PDFium 预览，暂不做编辑。实现 src/asset_viewer/pdf_preview.rs，稳定 API 动态加载、后台串行渲染、当前页纹理、页码 / 翻页 / 缩放 / 拖动 / 重载、缺组件直达管理和手动重试，详情与限制见 docs/versions/1.2.3.md。真实 DLL 和 GUI 尚未运行验证；本轮未获生成 EXE 授权，未构建、提交或发布。最新 EXE 仍是下述 SVG 背景版，不含第三方管理和 PDF 预览。下方专项交接的“PDF 尚未实现”为此前状态，以本节为准。

---
# 最新接续入口：PDF 预览与编辑（2026-10-18）

请优先阅读 [PDF 专项交接](docs/handoffs/2026-10-18-pdf-preview-editing.md)。当前版本 1.2.3，第三方支持管理源码已接入，**PDF 预览 / 编辑尚未实现，最新 EXE 也不含第三方管理窗口**。下一会话从此处继续，保留现有未提交 / 未跟踪改动；不自动构建、提交或发布。组件目录在缓存清理范围之外。下方为历史交接记录，较旧标题不代表当前状态。

---

# 当前交接：1.2.1 已正式发布（2026-10-16）

用户确认此前 60 帧节点预览构建检查通过，明确授权升级 1.2.1、重新编译并发布 GitHub。更新内容为替换 AppIcon、优化 Material 和 Shader 预览操作体验。

Release 构建成功，文件 / 产品版本 1.2.1，21479936 字节，SHA-256：9D5A27D0F5381C8A02FECC5232323229EFF9485534B266EF84A7600E5474DBF7。附件 target/release/UnityYAMLViewer-v1.2.1-windows-x64.exe；日志 target/version-1.2.1-release-build.log。已核对版本顺序、默认两版、重新打开折叠逻辑与 704 条七语言目录；测试源码编译通过，断言未执行。升级后 EXE 未启动，实际关于界面待验证。北京时间 2026-10-16 17:54:57 已发布 GitHub v1.2.1 并确认为最新正式版，非草稿 / 非预发布；附件大小与 GitHub asset.digest 核对一致。发布页 https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.1，标签指向 d970776ff4f31d1476bbc8c16204eb352724e6fa。

# 最新本地构建：Vert / Frag 单步预览（2026-10-15）

## Vert / Frag 单步预览本地构建（2026-10-15）

- 用户明确授权编译，构建前进程及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（23.95 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21143552 字节；SHA-256：06B678024FEAE8F8472B610FF18C7CA310B9B02B07C01DF360DE891CFBD50920。
- 包含 Vert / Frag 步骤节点、中间数值 / A 预览、最终输出切换；Vert 保留原几何并以插值显示。此前输入提交规则、透明 RGB 修正、Render State 和 Windows 拖图修正均包含在内。
- 日志 target/stage-step-preview-release-build.log，保留 Spine Runtime 158 条既有警告。未启动 EXE，单步 GPU 画面 / 交互及兼容性仍待运行验证；新增测试断言未执行。仅替换本地产物，GitHub 正式附件 / 标签未变，未提交或发布本次修正。

# 最新未发布源码：Vert 步骤值预览（2026-10-15）

Vert / Frag 均可选步骤预览。Vert 截取该步数值后继续原顶点计算，经空闲 Varying 插值显示，保留原几何；整结构体 / 多返回 / 通道不足拒绝。尚未生成 EXE，实际图像待验证；此前 Frag 首版待支持 Vert 的说明为当时状态。

# 最新未发布源码：Frag 步骤值预览（2026-10-15）

Frag 源语句以可选择的步骤节点展示，按原 Shader 执行前缀预览中间标量 / 向量，可看 A 或返回最终输出；诊断绕过 Blend，不改用户文件。本轮仅直线 Frag 语句级探针，完整算术拆解 / Vert / helper 内部 / 同时缩略图未实现。未重新生成 EXE，实际图像待验证。

# 最新本地构建：输入提交、颜色与 Render State（2026-10-15）

## 输入提交、颜色与 Render State 本地构建（2026-10-15）

- 用户明确授权编译，构建前进程及目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（23.21 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21108224 字节；SHA-256：F0811596752373DEA76633AE20CC3086DA6513EB7683B5934E2D92F822A79410。
- 包含此前 Windows 拖放修正、颜色控件保留透明 RGB、稳定输入标识、键盘回车 / 失焦提交、鼠标左键 release 后刷新，以及 Frag → Render State → Output；Cull Off / Back 与 Blend 可临时调节，Lighting / 深度仍只读并标出限制。
- 日志 target/material-focus-state-release-build.log，保留 Spine Runtime 158 条既有警告。未启动 EXE，实际拖放、发黑修正、连续输入 / 拖动与状态模块仍待运行验证；新增测试断言未执行。仅替换本地产物，GitHub 正式附件 / 标签未变，未提交或发布修正。

# 最新未发布源码：输入提交与 Render State（2026-10-15）

节点输入保留稳定焦点，键盘回车 / 失焦生效，拖动左键 release 后刷新；新增 Frag → Render State → Output，支持临时 Cull / Blend，光照和深度只读标明限制。尚未生成新 EXE；颜色修正、输入与状态模块均不在下方最新本地构建中。

# 最新未发布源码：透明 RGB 被颜色控件清零的修正（2026-10-15）

材质节点的 RGB 拾色器改用临时副本，确认编辑才提交 RGB，A 独立，防止 OverlayColor=(0.5,0.5,0.5,0) 在显示时变黑。未改 Shader / 用户资源 / Blend，尚未重新生成 EXE；下方最新本地 EXE 仅含拖放修正，尚无此次颜色修正。

# 最新本地构建：Windows 纹理拖放修正（2026-10-15）

## Windows 纹理拖放修正本地构建（2026-10-15）

- 用户明确授权“可以编译”，构建前进程和目标 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（24.97 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21000704 字节；SHA-256：E1E3D1DDBB514D2E39763972D97BE3F01FFE0ABFD03CB5329F9390968788FBB4。
- 包含 Windows 主窗口客户区鼠标坐标检测及拖放期间持续刷新，解决此前依赖普通鼠标事件而可能不出现节点蓝框、误开图片页签的问题。编译成功，实际 Windows / Unity 拖放仍待用户运行验证，测试断言未执行。
- 日志 target/texture-native-drop-release-build.log，保留 Spine Runtime 158 条既有警告。本地产物已替换，GitHub v1.2.0 附件 / 标签保持不变，未提交、未推送或发布本次修正。此前“尚未生成 EXE”指当时状态。

# 最新未发布源码：Windows 纹理拖放落点修正（2026-10-15）

修复预览纹理拖图依赖普通鼠标坐标的问题，改用主 HWND 客户区实际坐标并持续刷新悬停。已发布 1.2.0 不含本次修正；尚未重新生成 EXE，实际拖放待验证。同工作区 README 精简与使用指南迁移仍未提交，须保留。

# 最新正式发布：1.2.0（2026-10-15）

## 正式发布（2026-10-15）

- 北京时间 2026-10-15 17:12:42，GitHub v1.2.0 已正式发布并确认为最新正式版，非草稿、非预发布。用户已确认测试通过并授权发布；沿用已验收构建，无重新编译。
- 标签 v1.2.0 指向 4fc87f004541d1ce9ac7a8bf81ac8268e84b4a97（基于 c831301 的发布文档提交），包含此前全部源码。附件 UnityYAMLViewer-v1.2.0-windows-x64.exe，Windows x64、版本 1.2.0、21012480 字节；本地 SHA-256 与 GitHub asset.digest 一致：037FA1057426695C5B5C5CD892DCE8DCBA1C4046E12BBF9564743CB35EC095EE。
- 发布页 https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.0，发布说明 docs/releases/1.2.0.md；最新正式版 API、远端标签提交、附件大小与摘要均已复核。在线更新附件命名与摘要字段符合现有更新器规则。
- 本条优先于下方历史未发布 / 待验证描述。用户确认测试通过，不扩展为所有资源、显卡、DPI 和语言组合全面验证；自动测试断言仍未执行，已记录的能力限制保留。

# 1.2.0 正式发布准备（2026-10-15）

用户已确认测试通过并授权正式发布。沿用已验收的最后一次 Release EXE（21012480 字节，SHA-256 037FA1057426695C5B5C5CD892DCE8DCBA1C4046E12BBF9564743CB35EC095EE），不重新编译。发布说明见 docs/releases/1.2.0.md；正式状态待上传 / 远端核对后更新。本节优先于下方待运行验证与未发布的历史描述，具体功能限制仍保留。

# 最新构建：比例布局、连线与图标 1.2.0（2026-10-15）

本节为最新产物，优先于下方未构建 / 旧构建描述。

## 比例布局、连线与图标 Release 构建（2026-10-15）

- 用户明确授权后，目标进程与 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（39.13 秒）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21012480 字节；SHA-256：037FA1057426695C5B5C5CD892DCE8DCBA1C4046E12BBF9564743CB35EC095EE。
- 包含预览 / Inspector 比例宽度、可手动调整分隔线、自适应贝塞尔控制柄、顶部 Frag 输出端口，以及直接打包用户提供的各尺寸 PNG；只读提取最终 EXE 图标资源确认全部 13 档（16 / 20 / 24 / 32 / 40 / 48 / 64 / 72 / 80 / 96 / 128 / 192 / 256）。此前纹理拖放与缓存容错仍包含在内。
- 日志 target/responsive-icon-release-build.log，保留 Spine Runtime 158 条既有警告。未启动 EXE，窗口缩放 / 连线视觉、Explorer DPI / 缓存和拖放等实际操作仍待运行验证；测试断言未执行。未提交、未推送、未发布。

# 最新源码：直接使用用户补充的图标 PNG（2026-10-15）

build.rs 直接打包对应尺寸的 13 档 PNG，移除自动缩放 / 颜色转换；本轮比例布局、连线和图标修改仍未重新生成 EXE。

# 最新源码：图标改为两级线性采样（2026-10-15）

新增四档图标改为线性颜色空间、预乘 alpha 的两级双线性缩小；原有九档保持不变。尚未重新生成 EXE，比例布局、连线与图标变更均未进入当前 Release。

# 最新源码：图标中间尺寸（2026-10-15）

build.rs 新增由高清源生成的 72 / 80 / 96 / 192 像素图标；本轮比例布局、连线和图标修改尚未重新生成 EXE，下方 Release 不含这些变更。

# 最新源码：窗口比例布局与连线（2026-10-15）

预览 / Inspector 已按可用宽度比例调整，保留分隔线拖动；节点贝塞尔控制柄按距离计算，Frag 输出端口固定在顶部附近。本次尚未重新生成 EXE，下方 Release 不含该修改。验证状态见 docs/versions/1.2.0.md。

# 最新构建：纹理拖放与缓存尺寸容错 1.2.0（2026-10-15）

本节为最新产物，优先于下方旧构建及未构建记录。

## 纹理拖放与缓存容错 Release 构建（2026-10-15）

- 用户明确授权后，目标进程与 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（39.57 秒）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，20980736 字节；SHA-256：56D6F95E3AA6195E43E8C3F4C7616B22B3EB0DEDBA632419F3193AE5D3F56097。
- 已包含单张图片拖入可见 2D 纹理 Property 临时替换，以及缓存尺寸不支持时继续直接渲染 / 零像素裁剪区域跳过该帧；未改为固定 512×512 离屏渲染。
- 日志 target/texture-drop-release-build.log，保留 Spine Runtime 158 条既有警告。未启动本次 EXE，Windows / Unity 拖放、缩放裁剪与渲染恢复仍待实际运行验证；测试仅编译、未执行断言。未提交、未推送、未发布。

# 最新源码：纹理拖放与缓存尺寸容错（2026-10-15）

已增加单图拖到 2D 纹理 Property 临时替换，以及缓存尺寸无效时跳过缓存 / 零像素帧的容错。尚未重新生成 EXE；下方 Release 不含本次修改。Windows / Unity 拖放及渲染恢复待运行验证，检查结果见 docs/versions/1.2.0.md。

# 最新构建：画布内 2D / 3D 输出模块 1.2.0（2026-10-15）

本节优先于下方未构建及旧 EXE 描述，当前产物已包含最新画布输出模块与预置 3D 模型。

## 画布输出模块与 3D 模型 Release 构建（2026-10-15）

- 用户明确授权编译后，目标进程与原 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（28.70 秒）。
- 产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.2.0，20973568 字节；SHA-256：C6C6F85BDBAB1CF0F51946A293E202D854D60FBBE71B263E03FE884F40E87CED。
- 已包含画布内输出模块、右下角拉伸、2D 棋盘格 / 纯色、3D 球体 / 圆柱体 / 立方体及模型旋转 / 重置，以及此前全部源码修改。
- 日志 target/material-output-node-3d-release-build.log，保留 Spine Runtime 158 条既有警告。未启动本次 EXE，实际模块操作 / 三种模型 / Shader 兼容及七语言待运行验证，测试断言未执行；完整深度、复杂位移 / 透明相交、HDRI / PBR 和自定义模型导入仍有限制或未实现。
- 未提交、未推送、未发布。此前“未重新生成 EXE”描述为修改当时状态，本节为最新产物。

---

# 最新源码：画布内 2D / 3D 输出模块（2026-10-15，未重新构建）

按用户截图，输出画面已放回画布模块，删除右侧 / 下方停靠区；模块标题拖动、右下角拉伸、顶部定位输出。2D 保持棋盘格 / 纯色；3D 可选择真实球体、圆柱体、立方体，拖动旋转并重置，复用当前材质参数与纹理。

当前是受限 Shader 的正交凸网格预览，原始三角形按视深排序，不是完整深度 / HDRI / PBR 渲染。复杂顶点变形、透明相交仍有限制，自定义模型导入与圆角模型未实现。测试编译检查、git diff --check 与 647 条七语言静态核对通过；实际界面与测试断言待运行。

本节未重新生成 EXE，下方模块内输入版 EXE 不含本次输出模块与 3D 变更；未提交、未发布。

---

# 最新构建：模块内输入与独立输出 1.2.0（2026-10-15）

本节优先于下方未重新构建 / 保留阶段分组 EXE 的历史描述；当前已包含最新模块交互和视觉源码。

## 模块内输入与独立输出 Release 构建（2026-10-15）

- 用户明确授权编译后，目标进程检查及原 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（25.54 秒）。
- 产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.2.0，20968448 字节；SHA-256：90350BA1E1848F24EB9F7089121183BEF9495FA8C6225223EE61D1C7BCC6DA7D。
- 包含当前全部源码：参考 Amplify Shader Editor 的模块内输入、标量端口对齐与每个 Property 单独重置、RGBA / XYZW 逐行、独立可调输出区、棋盘格 / 纯色背景，以及此前阶段 / 函数 / v2f 分组与预览入口修正。
- 日志 target/material-amplify-inline-release-build.log，保留 Spine Runtime 158 条既有警告；未启动此次 EXE，实际输入 / 重置 / 拖动 / 输出布局与七语言待运行验证，测试断言未执行。球体等形状仍未实现。
- 未提交、未推送、未发布；上述源码“未重新构建”及保留旧 EXE 描述为当时状态，本节为最新产物。

---

# 最新源码：模块交互参考 Amplify Shader Editor（2026-10-15，未重新构建）

用户最新指定交互方向为 Amplify Shader Editor：参数直接在模块内输入，不另设节点参数 Inspector。标量行与 Out 端口对齐，四分量逐行保留；每个 Property 新增单独重置按钮，标题拖动区域排除按钮。独立输出区仍保留，仅显示预览与背景选择。原文件属性 / YAML 功能保留，临时参数不写回文件。

测试编译检查、git diff --check 与 640 条七语言静态核对通过；实际界面与测试断言待运行验证。既有阶段分组 EXE 保持供查看版本，本次未重新编译或覆盖 Release 产物，未提交、未发布。

---

# 最新源码：Designer 风格第一轮（2026-10-15，未重新构建）

用户指定顺序已完成：先生成下方阶段分组 1.2.0 EXE，再修改视觉源码。最新 EXE 保留，SHA-256 仍应为 1DFE3CDFF4DDFA0290A154476EB1981F0747A3395757571491646EDA6F84AE18。

后续源码增加独立停靠的输出区（窄下宽右、分隔条调节、隐藏 / 恢复、棋盘格 / 纯色背景），实时画面不随画布缩放；图形输出节点改为紧凑终端。节点、分组、连线和网格收敛为 Designer 参考风格，详细说明移入菜单。球体等新预览网格和 HDRI / PBR 尚未实现；当前仍仅平面预览。普通编译检查、测试编译检查、git diff --check 与 639 条七语言静态核对通过，真实界面与测试断言待运行。

本节源码未生成新的 EXE、未提交、未推送、未发布。请先用下方编译产物查看阶段分组版本。

---

# 最新构建：阶段分组预览 1.2.0（2026-10-15）

用户先验看此版 EXE，后续 Designer 风格改造仅修改源码。本节优先于下方“未构建”描述。

## 阶段分组预览 Release 构建（2026-10-15）

- 用户明确要求先编译供查看再继续视觉改造；目标进程检查与原 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（25.54 秒）。
- 产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.2.0，20941312 字节；SHA-256：1DFE3CDFF4DDFA0290A154476EB1981F0747A3395757571491646EDA6F84AE18。
- 包含独立预览面板、Property 节点直接编辑与逐分量端口、Vert / Frag / 辅助函数与 Vertex Input / v2f 接口分组、播放列表标题 / 扳手及当前图标资源。此 EXE 为参考 Substance Designer 视觉改造之前的版本。
- 日志 target/material-stage-groups-release-build.log；保留 Spine Runtime 158 条既有警告。未启动此次 EXE，真实界面和测试断言待验证；未提交、未推送、未发布。后续源码视觉改造不覆盖本产物。

---

# 最新源码：Vert / Frag / 函数与 v2f 接口分组（2026-10-15，未重新构建）

已增加 Vert、Frag 与各辅助函数独立大框及源码语句卡片，直接 Property 引用连接至所属函数；明确标注函数调用关系。Vertex Input 与 Varyings / v2f 分开列出字段、类型及语义，不混同模型 UV 与插值 TEXCOORD 通道；声明展示不等于 GPU 支持。支持受预算和路径限制的 include 展开、共享入口识别、组标题拖动。当前仍仅展示及临时参数预览，不编辑连线或生成 Shader。

编译检查、测试编译检查、git diff --check 与 633 条七语言静态核对通过；真实界面及新增测试断言待运行。下方 1.2.0 EXE 不含本节与前述节点 / 面板修正，未重新生成 EXE、未提交、未发布。

---

# 最新源码：Property 节点直接编辑与分量逐行显示（2026-10-15，未重新构建）

已将临时参数编辑移进独立预览面板中的 Property 节点；RGBA / XYZW 各分量逐行显示，类型 / 上下文决定默认颜色视图，可手动切换。已知 RGB / A / 分量使用显示对应端口连接；Shader 内部运算仍折叠。节点标题可拖动，增加网格、类型色条和纹理缩略图。当前仅展示与临时输入，不创建 Property、不改连线、不生成或保存 Shader。

编译和测试编译检查通过，实际界面与测试断言尚未运行。下方已有 1.2.0 EXE 不含本节及独立预览面板修正，未重新生成 EXE；未提交、未发布。

---

# 最新源码：材质 / Shader 独立预览面板（2026-10-15，未重新构建）

用户运行截图要求节点画布不要放在 Inspector。当前源码已删除 Inspector 的节点模式和 Shader 正文内画布；统一“预览”开关在独立可调宽面板显示画布，材质 Inspector 保留属性 / YAML，Shader 保留源码，自动打开设置同步接入。cargo check、测试编译检查、git diff --check 与 623 条七语言静态核对通过；实际运行待验证，测试断言未执行。

下方 1.2.0 EXE 不包含此次入口修正；未重新生成 EXE、未提交、未发布。

---

# 最新构建：1.2.0 材质与 Shader 节点预览（2026-10-15）

本节优先于下方旧版产物信息；最新正式发布仍为 1.1.9，本地 1.2.0 尚未发布。

## 1.2.0 Release 构建（2026-10-15）

- 用户明确授权编译后，目标进程检查与原 EXE 独占读写检查通过；cargo build --release --locked --offline 成功（28.83 秒）。
- 产物：target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.2.0，20779008 字节；SHA-256：5B11971D64E23304F09B390B1375337A44E0557624B427E7C7262A1DC2487029。
- 包含材质 / Shader 节点输入与临时参数预览、播放列表标题间距及矢量扳手、当前工作区图标资源和此前源码。构建日志：target/material-graph-1.2.0-release-build.log；保留 Spine Runtime 158 条既有警告。
- 未启动本次 EXE，节点布局 / 输入变化 / GPU 输出与七语言仍待运行验证；测试断言未执行。已构建不代表已发布，未提交、未推送、未发布。此前未生成说明代表对应修改当时状态。

---

# 正式发布：1.1.9（2026-10-15）

本节优先于下方未发布描述。用户验收通过并自行提交推送 master 后，已发布 GitHub v1.1.9 为最新正式版（非草稿、非预发布）。标签指向 fdcc3f3a691c060ce20342c2f2fdabae1cc80ffc。附件 UnityYAMLViewer-v1.1.9-windows-x64.exe，20600832 字节；GitHub asset.digest 与本地 SHA-256 均为 4A2B07EDD772D5FC7CA0BE6801F22B1E6CDD29AE1E3A377980A2AC00D15ACEB2，未因发布重新构建。

发布页：https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.1.9，最新正式版接口、标签和附件已核对。用户验收不等于所有资源 / 显卡 / DPI / 七语言全面验证，测试断言未执行，手动宽高比调整仍暂缓。详情见 docs/releases/1.1.9.md 和 docs/versions/1.1.9.md。

本轮发布后仅维护本地发布状态文档，未自动提交 / 推送这些文档；交由用户后续提交。后续构建仍需明确授权与占用检查。

---

# 最新构建：弹窗唤醒与设置换行（2026-10-15）

本节优先于下方未构建说明。用户明确授权，进程及 EXE 独占检查通过，Release 构建成功（15.30 秒）。target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.1.9，20600832 字节；SHA-256：4A2B07EDD772D5FC7CA0BE6801F22B1E6CDD29AE1E3A377980A2AC00D15ACEB2。

包含弹窗重新唤醒居中及显示 / 前置聚焦请求、预览开关右列多行文案，以及此前全部修改。未启动新版，实际系统窗口层级 / 居中及七语言布局待验证，测试断言未执行；保留 Spine 158 条警告，未提交、未发布。后续构建仍需明确授权及占用检查。

---

# 最新源码：弹窗重新唤醒与设置换行（2026-10-15，未构建）

统一宿主新增明确唤醒 / 恢复聚焦时重新居中及显示、取消最小化、Focus，设置 / 关于入口唤醒已有窗口，不永久置顶；预览开关文字限制右列多行显示，保留设置末尾排序。README 中英和关于七语言摘要同步。最新 EXE 不含本节修改，后续构建需明确授权与占用检查；实际前置、居中和多语言布局待运行验证。

---

# 最新构建：音频信息、区域箭头与模式标题（2026-10-15）

本节优先于下方未构建说明。用户明确授权，进程及 EXE 独占检查通过，Release 构建成功（16.55 秒）。target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.1.9，20547072 字节；SHA-256：9799D62FC0F6EA092FF36E463392C1995670BF2F4C9DCDD318533D1F4F0F0549。

已包含音频底部留白减少、音频 / 视频当前音轨码率、窗口及全屏列表箭头由媒体区域鼠标移动唤醒并一秒静止 / 移出隐藏、音频标题当前模式，以及此前全部修改。未启动新版，实际布局和播放待运行验证，测试断言未执行；保留 158 条 Spine 警告，未提交、未发布。后续构建仍需明确授权及占用检查。

---

# 最新源码：音频间距与码率（2026-10-15，未构建）

音频波形移除额外 40 点底部空白，信息加入源音频流码率 kb/s（缺失为未知），普通音频及列表共用。README 中英、关于摘要和七语言标签同步；视频当前音轨信息也加源码率，切换音轨同步；窗口 / 全屏箭头仅由媒体画布内鼠标移动唤醒，一秒静止或移出隐藏，不改变列表状态。音频列表当前文件名和大小后显示当前播放模式，侧栏收起也可见。实际显示待运行验证。下方最新 EXE 不含此修订，后续构建需明确授权与占用检查。

---

# 最新构建：去掉非全屏列表线框（2026-10-15）

本节优先于下方未构建说明。用户明确授权，进程及 EXE 独占检查通过，Release 构建成功（16.24 秒）。target/release/unity_yaml_viewer.exe，版本 1.1.9，20517376 字节；SHA-256：20535BAEB41C31C603F3536D24C8E3B34197E898DD571927F7E4E99ED10DC619。

已包含线框撤回及此前全部源码修订。用户讨论的手动宽高比调整已明确暂缓，不属于已实现功能。未启动新产物，实际界面待验证，测试断言未执行；保留 Spine 158 条警告，未提交、未发布。后续构建仍需明确授权及重新检查占用。

---

# 最新源码：去掉非全屏列表线框（2026-10-15，未构建）

用户验证后撤回非全屏四边线框，源码已删除描边，保留布局及调宽，不恢复额外竖线。下方最新 EXE 仍包含线框；本轮未重新生成 EXE，后续构建需明确授权及占用检查。版本记录与 README 中英同步，实际显示待运行验证。

---

# 最新构建：普通列表四边框与紧凑秒数（2026-10-15）

本节优先于下方未构建说明。用户明确授权后，进程及 EXE 独占检查通过，Release 构建成功（15.49 秒）。target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.1.9，20518912 字节；SHA-256：8C5E4FA5CDAB2EB6A143A7A2EECD9CAFD1ED1BF69F732971255FD4CBAB45D067。

包含普通列表四边线框、60×24 点秒数输入框及输入 / s / 按钮 2 点紧凑间距，以及前面所有源码修订。未启动此次产物，实际布局和播放行为待验证，测试断言未执行；保留 158 条 Spine 警告，未提交、未发布。后续构建仍需明确授权和重新检查占用。

---

# 最新构建：箭头、秒数与全屏浮层（2026-10-15）

用户明确授权，进程及 EXE 独占检查通过，Release 构建成功（15.15 秒）。产物 target/release/unity_yaml_viewer.exe，版本 1.1.9，20517376 字节；SHA-256：336D8BDD29E554D8C62035CE817215EE56B4687F0F2CD3734393F1E7110EFF22。包含下方此前未构建的 SVG 靠边箭头、整数秒、最小输入尺寸、滚动条靠右、全屏透明 / 圆角 / 自动隐藏 / 拖宽贴右 / 去额外竖线。未启动验证，测试断言未执行，未发布。

随后用户澄清线框针对非全屏列表，本轮补普通列表四边细框；另按截图将秒数输入框缩为 60×24 点，将输入 / s / 按钮并入同一紧凑横向组（2 点间距），避免按钮挤出。以上追加不在上述 EXE 中，后续构建需新的明确授权及占用检查。

---

# 最新源码补充：列表箭头靠边与 SVG（2026-10-15，未重新构建）

收起箭头改为贴媒体区域最右边缘、垂直居中，不跟随实际图像边缘；文字箭头换为居中 18×18 SVG 左右折线，仍只悬浮、不占视频宽度。全屏半透明列表同时增加 1 点低对比主题细边框；两行时间输入框最新改为显式 120×24 点最小尺寸，避免仅期望宽度被压缩；输入格式最终改为非负整数秒，固定单位 s，快捷取秒、重置为 0，旧分:秒设置读取时迁移成秒。创建与播放滚动区关闭横向自动收缩，滚动条固定右内边缘。全屏背板最新为 45% 不透明度、左侧 30 点圆角右侧直角；箭头 / 列表共用 1 秒隐藏计时，活动恢复，编辑 / 拖动保持可见；关闭浮层自动约束，拖宽右侧固定贴边；全屏左侧额外调宽分隔线不再绘制，拖动热区与外框描边保留。版本记录和 README 中英同步；实际显示待运行验证。下方最新 EXE 不含此修订，后续构建须明确授权及占用检查。

---

# 最新构建：播放列表布局与全屏浮层（2026-10-15）

本节优先于下方尚未重编译说明。用户明确授权，进程及 EXE 独占检查通过，cargo build --release --locked --offline 成功（17.18 秒）。target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.1.9，20535808 字节；SHA-256：8ACB680D3A0A893EAE9D5891225E8204B1C2ACA2EA625D6BEF90150C1010DF98。

已包含所有截至本节的侧栏、保存按钮、箭头 / 边距、跳过片头片尾快捷设置 / 加宽 / 重置、全屏半透明居中浮层及对称调高、最大窗口宽度三分之一限制。未启动产物，实际界面和媒体待运行验证，测试断言未执行；保留 158 条 Spine 警告，未提交、未发布。后续构建仍需明确授权与占用检查。

---

# 最新源码补充：播放列表布局修订（2026-10-15，尚未重编译 EXE）

优先于下方构建范围描述。最新还将全屏列表改为整体半透明浮层（82% 不透明度），覆盖在完整视频画布上，不挤占视频宽度；窗口布局规则保留；全屏浮层默认半屏高、居中，上下边缘可对称调高并记忆，所有列表面板最大宽度统一为窗口宽度三分之一（最小目标 220 点，小窗口受上限约束）。用户运行截图后要求侧栏仅占视频画面，不挤占文件信息与底部时间轴；创建保存按钮底部居中并放大为 128×36；收起箭头仅覆盖视频，不占适应窗口宽度；侧栏内容左侧加 2 点内边距。后续还将“剧集模式”更名为“跳过片头片尾”，时间框后加片头 / 片尾按钮，分别捕获当前位置和总时长减当前位置，向下取整到秒；各列表独立保存；复选框后新增重置（同时将两项设为 0:00，不改开关）；两行时间输入框后续从 70 加宽到 100 点，普通 cargo check 与差异检查通过。源码已修改，README 中英、关于七语言和 1.1.9 版本记录同步。cargo check --locked --offline --tests、差异检查及七语言最新 596 条 / 40 模板通过。实际显示仍待运行验证，未生成包含修订的 EXE；后续构建须新的明确授权及占用检查。

---

# 最新构建：1.1.9 播放列表与 Next（2026-10-15）

本节优先于下方未生成 EXE / 旧产物说明。用户再次授权后，进程与 EXE 独占检查通过，cargo build --release --locked --offline 成功（16.70 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.1.9，20519936 字节；SHA-256：E32F759FD077F6A1E5EC8A65F486B3CE91B78A8F357CD53241A562F2560153D2。

包含本轮播放列表及矢量 Next，以及前轮未构建的最近记录宽度、设置缓存布局与音量数字移除。单曲循环手动 Next 按顺序切下一曲，仍保持单曲循环；末项暂停。此前发现占用的一次检查停止后没有构建，本次重新检查通过才编译。

未启动新版，测试断言未执行，创建 / 覆盖 / 续播 / 模式 / 片头片尾 / 最后一集 / 窗口及全屏布局与七语言待运行验证；保留 158 条 Spine 警告。未提交、未发布。构建日志 target/playlist-release-build.log，详细记录见 docs/versions/1.1.9.md。后续生成 EXE 仍须明确授权及重新检查占用。

---

# 当前交接补充：1.1.9 播放列表源码（2026-10-15）

本节优先于下方“下一步创建播放列表”旧状态。用户已确认讨论方案并授权实现，当前源码已接入播放列表，版本仍为 1.1.9；没有生成 EXE 的授权，本轮只做 cargo check 与测试编译检查。

- 起始 HEAD 为 e145b8c，工作区原本干净；下方 862bf1d / 尚未提交说明已经过时。本轮源码与文档修改尚未提交，不自动提交或发布。
- 新增 src/playlist.rs，标准本地 M3U8 / M3U、侧栏创建（全部同类 / 前缀）、候选名称及覆盖保护、单页签 / 列表最近记录、独立播放器和进度。视频编号网格或名称单列，音频名称单列；窗口 / 全屏悬浮箭头、动画、边界调宽与滚动。
- 音频顺序 / 随机 / 单曲循环；每份列表独立剧集模式及分:秒片头片尾，最后一集暂停并显示中央确定提示。进度与设置分开存，十秒及退出等事件后台保存；清理进度不删列表文档 / 设置。
- 后续修订：列表控制栏的停止替换为圆角三角 + 竖条 SVG Next，窗口 / 全屏共用；手动在单曲循环中也切下一项，随机沿用随机队列。测试编译、差异检查及目录 589 条 / 40 模板通过，未生成 EXE，实际运行待验证。
- 用户特别确认：音乐始终竖向名称列表；电影合集前缀（如宫崎骏_）可移除显示前缀，列表标题不加“播放列表”或第几项；列表内切换不添加单个媒体最近记录，也不复用普通同路径播放器。
- 实现与限制见 docs/versions/1.1.9.md 本轮播放列表节；仍需真实媒体、窗口 / 全屏、七语言与混合资源运行验证。新增测试仅编译，未执行断言。cargo check --locked --offline / --tests 与 git diff --check 通过；七语言目录 587 条 / 40 个模板校验通过，保留 Spine 158 条既有警告及增量缓存复制提示。
- 未运行程序、未生成本轮 EXE、未提交、未发布；现有 20197888 字节 Release 仍为旧产物，不含播放列表及前次三项最终修订。后续构建须明确授权并重新检查进程 / 文件占用。

---

# 当前交接：1.1.9 界面与媒体体验，下一步创建播放列表（2026-10-15）

本节是新对话的当前入口，优先于下方历史交接。工程路径为 `C:\Work\Git\UnityYAMLViewer`。先阅读 `AGENTS.md`、本节和 `docs/versions/1.1.9.md`，不要沿用历史正文中的旧路径、旧版本或旧需求状态。

## 当前代码与 Git 状态

- Cargo.toml 当前版本为 **1.1.9**；本次未发布。下一阶段没有自动升级版本号的授权，版本决策另行处理。
- 交接时分支为 `master`，HEAD 为 `862bf1d docs: record v1.1.8 official release`；1.1.9 修改仍在工作区 / 暂存区，用户准备自行提交，尚不能认为提交已经完成。新对话必须重新检查 `git status` 和 HEAD，不重置、覆盖或自动提交用户修改。
- 最新源码与暂存区不完全一致，包含后续布局修正；交接时 `.codex-check-output.log`、`.codex-test-check-output.log` 为 `AD`（曾暂存、工作区已删除）。用户提交前需同步最终修改和日志删除，避免将临时检查日志提交。交接文档本身也需纳入用户此次提交。

## 本轮最终实现与用户偏好

- 查找栏使用矢量放大镜 / 清除图标，支持收起与窄窗口换行；收起时显示拖入文件提示及“格式支持”。最近记录可单项删除 / 清空，删除图标为矢量减号，右侧同列对齐；最新源码同步菜单内容宽度，修复删除长名称后框体仍过宽的问题。
- “格式支持”列出实际支持范围；该窗口与设置“文件关联”共用默认程序弹窗。后台查询 Windows 真实默认程序；每项复制后缀并打开系统默认应用总页，用户粘贴搜索后确认。**不自动注册程序，不直接改写关联，不宣称已定位到特定后缀。** `.skel.bytes` / `.json.txt` 按最终 `.bytes` / `.txt` 处理，`.json` / `.ts` 的用途冲突已说明。
- HTTP / HTTPS 文件直链下载后打开，最大 1 GiB、总超时五分钟；网页解析、认证、流媒体及边下载边播放未实现。网络缓存文件不进入最近文件 / 恢复页签，也不保存播放位置。
- **音视频打开就播放，没有自动播放开关。** 用户明确取消了开关方案，不应重新添加。打开已有媒体页签会继续播放；普通页签切换暂停，不自动恢复。音轨标题 / handler_name 回退及常见语言的七语言显示保留，未知标记不推测实际语音。
- 关闭媒体页签、切换页签、正常退出时保存播放位置，下次打开恢复并播放；最多 500 条、序列化文件最多 2 MiB，淘汰最久未更新记录。按路径、大小及修改时间确认文件身份；文件移动 / 改名不跟踪，内容修改、已播完或无效位置从零开始。无定时持久化，崩溃 / 强制退出不保证最后进度。视频无帧索引时近似定位，索引完成后校正，不保证任意资源的精确帧级续播。
- 软件缓存显示占用及“查看 / 清理”；**不显示两段固定说明，容量标签为 B / KB / MB / GB，保留每级 1024 的换算。** 查看直接打开文件管理器，旧缓存另提供对应目录入口。清理播放记录、可重建索引和未使用下载，保留设置、最近文件、页签恢复、未知文件、正在使用的文件及待安装更新 / 更新失败备份；不跟随目录链接，不删除用户工程、媒体原文件或 FFmpeg。
- 设置末段顺序固定为：**文件关联 → 软件缓存 → 最近打开记录 → 单个文件缓存撤销步骤 → 检查更新**。以后新增设置必须插在固定最后两项之前，两项相邻且检查更新始终最后。
- 统一应用弹窗新开 / 重开时，以完整外框（含标题栏）中心对齐主窗口外框中心；初始高度稳定后可手动移动。嵌套窗口仍以主窗口为基准，考虑不同 DPI 和副屏负坐标。系统文件选择器 / 系统消息框 / Windows 设置页面仍由系统管理位置。
- **音量弹窗只保留竖直滑条，不显示百分比数字，也不预留数字区域；按钮提示只显示音量名称。** 保留拖动、方向键微调、静音 / 恢复及音量记忆。
- 其他本轮改动包括输入框垂直居中、页签完整路径提示、350 点页签中间截断、浅色音频深灰峰值拖尾线，以及全屏按钮框体。全屏主题保留原行为，用户明确不希望强制深色。

## 最新产物与验证边界

- 已核对本地 `target/release/unity_yaml_viewer.exe`：文件 / 产品版本 1.1.9，**20197888 字节**；SHA-256：`29FEDA0530E30120A6AD99B0E19A56BB9F954E1464CEA85DC3AF8C453CA8A669`。
- 该 EXE 是用户明确授权且进程 / 独占检查通过后生成的 Release，包含默认程序、自动续播、缓存管理和弹窗居中等功能。
- **该 EXE 尚不包含最后三项源码修订：** 最近记录删除后的框体宽度同步；文件关联 / 软件缓存移到最近记录上方、缓存固定说明移除与 KB / MB / GB 标签；音量数字移除。不要将最新源码等同于已构建产物。
- `cargo check --locked --offline` / `cargo check --locked --offline --tests`、七语言目录生成器及 `git diff --check` 已通过。目录最近核对 524 条文案、40 个格式模板；保留 Spine Runtime 158 条既有警告及增量缓存硬链接失败后的复制提示。
- 新增 Rust 测试仅编译，**未执行断言**。用户提供了程序运行截图，可确认看到部分界面；不等同于续播、清理、混合 DPI、所有资源或七语言全面验证。具体待验证事项见当前版本记录。
- 本轮未发布，未打标签。后续生成可执行程序须再次遵守用户的构建授权与占用检查约定；此前构建授权不视为下一阶段持续授权。没有明确授权时使用 `cargo check`，不要执行 `cargo build/run/install` 或会生成测试可执行程序的测试命令。

## 下一步：创建播放列表

用户明确希望新对话继续做“创建播放列表”；本轮**未实现**播放列表，不要写成已有功能。

当前版本记录已有以下待实现需求，作为后续讨论的依据；首批范围和 UI 尚需在新对话确认：

1. 按目录生成音频、视频全部及剧集播放列表；保存列表文件，将列表文件加入最近打开。
2. 列表内切换媒体不受已有同文件普通页签影响，不把列表中的单个媒体写入最近打开。
3. 音频顺序 / 随机 / 单曲循环。
4. 视频剧集支持片头 / 片尾时间设置，最后一集结束后暂停并提示。

建议先讨论并绘制创建入口、列表面板和保存流程，再落地首批创建 / 保存 / 打开 / 切换；这只是下一阶段的实现建议，不能视为已批准的 UI 或文件格式。需要明确：

- 列表文件格式 / 后缀、用户保存位置、相对路径策略、目录是否递归、自然排序和缺失文件处理。
- 剧集如何分组与排序，避免未经确认用文件名推断错误剧集；是否允许预览 / 手动调整条目。
- 列表“继续播放”保存的是当前条目与位置，还是沿用独立文件记录；自动切到下一项是否从零开始，以及片头 / 片尾与续播的优先级。
- 音频模式、剧集设置是首批同时做，还是分阶段。新对话不要把未完成部分记录成已完成。
- **用户保存的播放列表属于用户文档，不能当作可删除缓存。** 清理缓存可以清掉播放进度，但不应删掉列表文件；列表保存建议提供用户选定位置，不直接把列表塞进会被清理的目录。

## 代码入口及接入注意点

| 文件 | 当前职责 / 下一步注意点 |
| --- | --- |
| `src/main.rs` | `open_file_with_directory_tracking` 按路径复用已有页签，随后调用 `request_media_playback`；`activate_tab` 暂停全部媒体；`remember_recent_file` 记录当前路径。列表打开 / 列表内选项需要独立分发，避免走普通文件的路径去重和最近记录逻辑。 |
| `src/asset_viewer.rs` | `Assets.media` 是按文件路径存储的 `HashMap<PathBuf, Preview>`。现有同路径播放器复用不能直接满足“列表不受普通同文件页签影响”，需明确列表会话 / 播放器所有权。 |
| `src/media_preview.rs` | FFmpeg 解码、播放时钟、暂停 / 停止 / 跳转、音量、音轨、帧缓存。`Preview::new` 恢复位置并申请自动播放，`deactivate` 保存位置，`Drop` 再保存；完成、暂停、停止和解码错误需要区分，下一首只能响应一次真实播放完成事件。 |
| `src/media_history.rs` | 播放位置与文件身份记录、500 条 / 2 MiB 限制、原子替换及清空；不要把列表文档混入此可清理数据。 |
| `src/recent_files.rs` | 路径去重、10～50 条上限、持久化与删除。未来应该记录列表路径，而不是列表切换产生的单个媒体路径。 |
| `src/remote_file.rs`、`src/app_cache.rs` | 新下载 / 更新缓存、旧目录兼容、占用统计与安全清理；清理只限明确的可丢弃目录，并保护正在使用文件。磁盘缓存写入和清理通过 `mutation_guard` 串行协调。 |
| `src/native_window.rs` | 应用弹窗统一宿主与首次居中定位；新创建列表窗口应复用。 |
| `src/translations.tsv`、`scripts/generate_localized_formats.py` | 七语言文案和格式化模板生成；运行时还容许空行，生成器要求每行七个非空字段，新增文案需完整同步。 |

推荐回归重点：列表重新打开与最近记录、同媒体已开普通页签时的列表播放、下一项只推进一次、暂停 / 停止 / 错误不误推进、快速连续切换取消旧解码任务、最后一项暂停、排序 / 相对路径 / 中文空格路径、清理不删列表。测试执行或新 EXE 构建仍须先获得符合 AGENTS.md 的明确授权。

## 新对话建议首条消息

> 请先阅读 C:\Work\Git\UnityYAMLViewer\AGENTS.md、HANDOFF.md 顶部当前交接和 docs/versions/1.1.9.md，核对 Git 工作区及源码 / EXE 差异。接下来做“创建播放列表”：先结合已记录的待实现需求讨论首批范围与 UI 布局，再实现。不要恢复自动播放开关，不要删除用户保存的列表，不要擅自生成可执行程序或升级版本号。

---

以下为历史交接与构建记录，按记录时间理解；当前状态以顶部交接及当前版本记录为准。

## 最新产物补充（2026-10-15）

- 当前版本为 1.1.9；以下旧交接正文属于早期 1.1.1 历史，不代表当前版本或最新产物。
- 用户授权且进程 / EXE 独占检查通过后，cargo build --release --locked --offline 成功。最新产物：C:\Work\Git\UnityYAMLViewer\target\release\unity_yaml_viewer.exe；20197888 字节，文件 / 产品版本 1.1.9 / 1.1.9。
- SHA-256：29FEDA0530E30120A6AD99B0E19A56BB9F954E1464CEA85DC3AF8C453CA8A669。
- 包含矢量减号、格式支持、默认程序引导、打开自动续播、受限播放记录、缓存管理、主窗口相对居中弹窗；完整范围与限制见 docs/versions/1.1.9.md。
- 编译及测试编译检查通过，测试断言未执行；未启动应用，实际界面 / 系统跳转 / 续播 / 清理 / 居中待验证，未发布。

# 正式发布：1.1.8（2026-10-14）

- 2026-10-14 18:36（北京时间）：用户确认已合并 master 并授权正式发布。GitHub v1.1.8 已发布为最新正式版（非草稿、非预发布），源标签指向 master 的 3188e4f80f0e790e00685c21c420c1d33e1c5471；沿用已核对的最新 Release 构建，不因发布重新生成 EXE。附件 UnityYAMLViewer-v1.1.8-windows-x64.exe，Windows x64 PE、文件 / 产品版本 1.1.8，19743232 字节；本地 SHA-256 与 GitHub asset.digest 均为 A0691B6D5ABCF13E521CC4C2DC7B78614ADD9341747BB89AE6221089C89EE361。发布页 https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.1.8，最新正式版 API 与标签已复核。6 项粒子和 1 项 Prefab 源导航回归通过；此前粒子 Shader 回归及编译检查通过，部分真实资源有用户确认，未声称全部显卡 / 资源 / 七语言全面验证。发布说明见 docs/releases/1.1.8.md。

本节优先于下方未发布描述。当前分支 master，源代码已由用户提交并同步远程；本轮只维护发布文档。后续构建仍需明确授权及占用检查，复杂粒子、UI Canvas / 3D 等限制继续保留。

---

# 最新构建：1.1.8 灯塔粒子与 Prefab 打开（2026-10-14）

- 2026-10-14 用户关闭占用程序并再次明确授权后，进程 / EXE 独占检查通过。6 项粒子回归（含真实灯塔预热与继续播放 0.25 秒后的 Burst）及 1 项多实例 Prefab 源导航映射回归通过。首次灯塔断言错误地要求预热结束瞬间已有至少四个系统存活；真实两个 Burst 每周期 0.2 秒触发，已按该时间点修正验证，不改变实际发射计划。Release 构建前再次确认无占用，cargo build --release --offline --locked 成功（24.00 秒）。产物 target/release/unity_yaml_viewer.exe，版本 1.1.8，19743232 字节，SHA-256：A0691B6D5ABCF13E521CC4C2DC7B78614ADD9341747BB89AE6221089C89EE361。包含 Prefab 根节点打开源资源、有限预热与缓存重播、Billboard 独立宽高、ConeVolume 和粒子 straight RGBA 修复及此前累计修改。保留 Spine 158 条既有警告，未启动本次 EXE；实际灯塔效果、源页签打开 / 目标、排序 / 重播和七语言界面仍待运行验证，未发布。日志 target/lighthouse-particle-tests.log、target/prefab-open-tests.log、target/lighthouse-build.log。

本节优先于下方未构建和新断言未执行描述。下一步验证 Level_102 灯塔，鼠标移回画面继续播放；源 Prefab 打开不携带场景覆盖。后续构建仍需明确授权与占用检查。

---

# 最新源码：Prefab 打开与灯塔粒子（2026-10-14）

- 2026-10-14 Prefab 源资源入口与灯塔粒子阻塞修复：展开数据记录每个实例根节点的实际源 GUID，保留嵌套实例与 stripped 别名映射；Inspector 标题旁新增打开入口，独立打开源 Prefab / 模型资源，不写入或复制场景实例覆盖，源文件不可用时禁用。灯塔六个粒子系统均开启 prewarm，此前拒绝该配置导致没有粒子绘制。现支持循环系统预热一个周期（最多 60 秒、估算 2,000,000 particle-steps），后台初始化后缓存粒子 / 时间 / RNG / 发射累计状态，重播直接恢复缓存。Billboard 的 size3D 读取 X/Y 独立宽高，Z 厚度不使用；新增静态 ConeVolume 体积分布。粒子顶点保留 straight RGBA，由原 Shader 处理 Alpha / 混合，修正 One 混合中重复预乘。Velocity 等未支持模块仍提供提示，鼠标离开画面 / 离焦冻结规则保留。新增实例源导航映射、预热恢复和真实灯塔系统回归代码；cargo check --offline --locked --tests 通过，新增测试断言未执行，源资源导航、灯塔实际画面 / 排序 / 重播及七语言界面待验证。未生成本轮 EXE，现有基础粒子构建不含修复，未发布。

本节优先于下方记录。编译须明确授权并检查占用；下一步验证 Level_102 灯塔及独立打开源 Prefab。场景覆盖不复制到源资源。

---

# 最新构建：1.1.8 基础粒子实时预览（2026-10-14）

- 2026-10-14 用户明确授权后检查程序进程及目标 EXE 独占访问均通过，粒子模拟 / Shader / 属性回归共 7 项通过（包含真实 newitem Prefab 与材质 Shader）。受限环境首次真实 Include 路径检查拒绝访问，经正常权限重试全部通过；日志 target/particle-tests.log、target/particle-shader-tests.log。构建前重新检查主程序无占用，cargo build --release --offline --locked 成功（21.21 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本均为 1.1.8，19671040 字节，SHA-256：2BD63219E4918F8A644663755FFE6B2226FEEAC75429634D2910706962F7615E。包含场景 / Prefab 基础粒子播放、暂停 / 重播、Renderer GPU 材质及此前 Spine / Tilemap / 模型 / 空 Mesh 累计修改。保留 Spine 158 条既有警告。未启动本次 EXE；实际显卡编译、粒子画面、场景 / Prefab、暂停 / 重播 / 冻结、变换 / 排序与七语言界面待运行验证，复杂模块与 UI Canvas / 3D 限制仍保留，未发布。

本节优先于下方未构建 / 测试断言未执行描述。下一步在 board_item_newitem_effect.prefab 等资源及场景内检查粒子画面与控制，按实际诊断扩展尚未支持模块。后续构建仍须明确授权并检查占用；LineRenderer / TrailRenderer / Terrain 仍暂缓。

---

# 最新源码：场景与 Prefab 基础粒子播放（2026-10-14）

- 2026-10-14 场景 / Prefab 基础粒子实时预览：两者复用现有二维场景 GPU 路径，不新增独立粒子窗口。CPU 按固定 1/60 秒步进模拟，ParticleSystemRenderer 首材质执行既有限定 Built-in Shader，绑定真实主纹理、UV0、颜色、时间和每帧 Billboard 三角网格；材质 / 纹理缓存复用。提供播放 / 暂停 / 重播，共用预览时间用于 Shader _Time；重播恢复固定随机种子，遵循此前离焦 / 鼠标离开画面冻结规则。支持 duration / looping / 常量 startDelay、simulationSpeed 0..2、playOnAwake、持续 rateOverTime 和 Burst（数量 / 重复 / 概率）、初始寿命 / 速度 / 大小 / Z 旋转 / 颜色、MinMaxCurve 常量 / 曲线 / 双曲线 / 双常量及颜色 / 渐变；生命周期大小、颜色与 Z 旋转，曲线复用已有加权采样，旋转积分为 16 段数值近似。支持不启用 Shape，以及静态完整 Sphere / Cone 基底 / Box / Circle / Rectangle 的位置、旋转、缩放和半径厚度；只接入 View Billboard，静态父节点变换下采用既有层级矩阵，尚未播放发射器 Transform 动画，Local-only / Custom 缩放或空间、3D 初始大小 / 旋转等拒绝。默认重力取 -9.81，未加载项目 Physics 全局设置；速率曲线按步长中点积分，预览随机分布不保证与 Unity 相同。prewarm / ring buffer、部分 / 动态 Shape arc、Mesh / Stretch 等 Renderer、Custom Vertex Streams / TEXCOORD1/2 实值、UV 动画、Velocity / Force / Noise / Collision / SubEmitter / Trail 等模块未还原；启用未支持模块提供逐模块提示，仍允许其余基础行为，不能称为完整粒子还原。UI Canvas 与 3D 预览明确提示尚未接入；粒子逐粒子画面拾取未实现。未支持 / 缺失的材质 Shader 报告诊断，不能显示粒子真实效果。每场景最多 128 个系统、每系统最多 4096 粒子、每帧最多 65536 个可见粒子，生命周期预览最多 120 秒，duration 支持 0.001..3600 秒；超限或不支持行为明确提示，估计发射范围用于初始适应窗口，实际最大范围可能不同。新增 Burst 嵌套曲线不误拆、寿命 / 持续发射 / playOnAwake、确定性重播、原始几何 / Alpha 预乘和真实 newitem Prefab 初始 Burst 回归代码。cargo check --offline --locked --tests 通过（target/particle-check.log）；测试断言未执行，实际 GPU 粒子画面、材质、场景与单独 Prefab、暂停 / 重播 / 冻结、层级变换 / 排序和七语言界面待运行验证。源码仍为 1.1.8，未生成本轮 EXE、未发布。

- 本轮粒子 Shader 兼容补充：真实 newitem 材质为 PFW/UI Particles/Alpha Blended。二维世界粒子路径只移除未启用 UNITY_UI_CLIP_RECT 分支及其声明，保留原颜色 / 2 倍 Tint / AlphaScale / BSC 公式；启用裁剪关键字或嵌套裁剪条件明确拒绝，不声称支持 Canvas 遮罩。UnityUI.cginc 作为已知引擎引用保留至粒子专用预处理，非粒子路径仍拒绝未支持指令。当前无场景深度的世界预览把 unity_GUIZTestMode 设为 Always，ColorMask 按实际参数仅接受 RGB / RGBA。材质缓存区分粒子、Spine 和普通网格的编译模式。新增未启用裁剪 / 实际参数 / 启用关键字拒绝及真实粒子 Shader 回归代码；仅通过编译检查，未执行断言和实际显卡编译。

本节优先于下方构建记录。用户确认上一 Spine 构建已可用，随后要求粒子接入，并指定场景 / Prefab 共用实时预览和暂停 / 重播。现有 EXE 为下方 Spine GPU 构建，不含本轮粒子源码。下一步经明确授权并检查占用后构建，核对 board_item_newitem_effect.prefab 等真实资源及场景中嵌套粒子；复杂模块按实际反馈扩展，不把未支持事项写成完成。LineRenderer / TrailRenderer / Terrain 仍暂缓。

---

# 最新构建：1.1.8 场景与独立 Spine GPU Shader（2026-10-14）

- 2026-10-14 用户明确授权后，检查无 unity_yaml_viewer 进程且目标 EXE 独占访问通过，cargo build --release --offline --locked 成功（31.01 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本均为 1.1.8，19536896 字节，SHA-256：2FD4D52D93018779F268E88A6C4115E6DF4D27A01785228CFD3C49748F2D8A90。包含场景与独立 Spine 原始几何 / 图集 / 材质的受限 GPU Shader 接入、场景材质覆盖与初始皮肤传递、独立动画几何更新、有效空 Mesh 跳过，以及此前 Tilemap 材质、PF3D2 半透明模型与 FBX 轴向 / 缩放累计修复。编译检查、七语言与差异检查通过；保留 Spine 158 条既有警告，新增测试断言未执行。未启动本次 EXE，实际显卡 Shader 编译、Level_101 船屋夜景调色 / 透明边缘、多页材质、独立动画 / 皮肤切换、场景入口覆盖传递、控件层级、空 Mesh 提示和七语言界面仍待运行验证，未发布。

本节优先于下方未构建描述。下一步核对场景船屋调色与从场景进入独立 Spine 的材质、初始皮肤、动画、透明边缘及控件。特殊混合 / 双色染色、缺失映射或不支持 Shader 保留回退；LineRenderer / TrailRenderer / Terrain 仍暂缓。后续构建仍需明确授权并重新检查占用。

---

# 最新源码：场景与独立 Spine GPU Shader（2026-10-14）

- 2026-10-14 Spine GPU Shader 接入：场景 Spine 3.8 setup pose 保留运行时裁剪后的原始附件顶点 / UV / 索引、颜色及槽顺序，按 AtlasAsset 材质的 _MainTex 引用唯一匹配图集页与材质槽，并使用同 GameObject MeshRenderer 的材质覆盖。按原有场景变换 / 初始翻转 / 排序提交 GPU 绘制；保留 CPU 烘焙用于回退、边界与拾取，不把烘焙图再乘材质颜色冒充原始 Shader。独立 Spine 预览复用材质加载，动画每帧更新原始几何；从场景入口携带 Renderer 材质与初始皮肤，SkeletonDataAsset 直接打开使用 AtlasAsset 默认材质，原始骨骼文件仅在同目录索引能唯一找到配套 SkeletonDataAsset 时取得材质上下文，否则仍普通预览。材质 / 图集缓存复用，原始图集像素与 UV 保持原方向及源 Alpha 编码；One 混合的顶点 RGB 按 Alpha 预乘。船屋 Spine/Skeleton 的单颜色 Pass 按原公式编译，仅省略明确标记 LightMode=ShadowCaster 的 Pass；其他多颜色 Pass 仍拒绝，实际用到的 shader_feature 条件变体仍不支持，仅删除没有被代码引用的声明。特殊混合 / 双色染色、图集材质映射缺失、槽或 Shader 不支持时保留整对象普通回退及提示。场景仍静态姿态，独立动画继续使用已有 Spine Runtime；UI Spine / 3D 模式、阴影 / Stencil / 全局昼夜脚本未接入。GPU 几何上限每骨骼一百万顶点，图集 GPU 计入 2 GiB 源纹理预算；原 CPU 缓存和几何仍有额外占用。新增原始网格 / 图集像素 / 材质颜色绑定及颜色 Pass / 变体拒绝回归代码和真实船屋 Shader 检查代码；cargo check --offline --locked --tests 通过（target/spine-gpu-check.log），本轮测试断言未执行，实际 GL Shader 编译、船屋调色、透明边缘、多页材质、动画 / 皮肤切换、覆盖传递、控件层级、七语言界面仍待运行验证。未生成本轮 EXE，现有 Tilemap 修复构建不含 Spine GPU 和有效空 Mesh 修改，未发布。

本节优先于下方构建记录。有效空 Mesh 修改一并待构建。生成 EXE 须明确授权并检查占用；下一步核对 Level_101 船屋调色和从场景进入独立 Spine 的材质 / 皮肤及动画显示。LineRenderer / TrailRenderer / Terrain 仍按用户要求暂缓。

---

# 最新源码：有效空 Mesh（2026-10-14）

- 2026-10-14 有效空 Mesh 不再误报缺失 / 超预算：核对 Level_101.unity 的 MeshFilter 461402410 / 1148902295 / 1996789715，引用 Mesh 1939818100 / 1214895597 / 787006432，均为零顶点、零顶点数据、空顶点 / 索引缓冲、零索引数。空网格正常不绘制；非零缓冲或索引与零顶点矛盾仍报错，超一百万顶点单独报告预算错误。静态场景跳过确认的本地空 Mesh，模型烘焙也处理空几何，避免计算无效边界。独立 Mesh 保留 Inspector，不把 Hierarchy 的未关联分组等同资源缺失。新增空网格 / 矛盾数据 / 超预算回归代码；本轮仅编译检查，未执行测试断言，实际界面待验证，未生成本轮 EXE、未发布。

本节优先于下方构建记录，现有 EXE 尚不含本修复。后续构建须明确授权并检查占用。

---

# 最新构建：1.1.8 Tilemap 材质（2026-10-14）

- 2026-10-14 用户明确授权后，重新检查无 unity_yaml_viewer 进程且目标 EXE 独占访问通过，cargo build --release --offline --locked 成功（18.95 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本均为 1.1.8，19430400 字节，SHA-256：AC2E1B7CAD6BE11658CC642E1DB73D27329A3BBC94E164A7FD0418D53D3E036C。包含 TilemapRenderer 首材质 GPU Shader 接入，以及此前 PF3D2 透明模型、多材质槽、FBX 轴向 / 缩放等累计修复。cargo check --offline --locked --tests、七语言校验与 git diff --check 通过；新增测试断言未执行，保留 Spine 158 条既有警告。未启动本次 EXE，Level_101.unity 的 LandTilemap / sprite_night 调色、单元贴图 / 排序 / 点选及七语言界面仍待运行验证；未发布。

- 用户确认目前未使用 LineRenderer、TrailRenderer、Terrain，明确暂缓接入；相关代码未修改，不属于本次已完成能力。

本节优先于下方未构建描述。下一步验证 Level_101.unity 的 LandTilemap / sprite_night 效果。后续构建仍需明确授权并重新检查占用。

---

# 最新源码：Tilemap Renderer 材质（2026-10-14）

- 2026-10-14 Tilemap Renderer 材质接入：Level_101.unity 的 LandTilemap 使用 sprite_night（PFW/Sprite/SpriteDefault），此前 GPU 材质筛选遗漏 Tilemap，导致仅用普通贴图。现通过同 GameObject 的 TilemapRenderer 获取首材质，复用已有 Shader 编译、参数和纹理绑定路径；每个 Tile 单元继续提供自身 Sprite / 图集纹理、UV、Tile / Tilemap 颜色、变换、排序及 Tilemap 拾取 ID。按组件去重材质准备，材质缓存区分 Renderer 提供主纹理与 Mesh 材质主纹理绑定模式。Shader 不支持 / 编译失败仍提示并回退普通贴图；仅支持既有限定 Built-in Shader 范围，非 Unity 完整 Tilemap Chunk / Individual 渲染、遮罩或运行时脚本。真实材质参数 _Color=(0.50980395,0.6431373,0.7607843,1)，Shader 位于 Library/PackageCache，包含相对 CommonMath.cginc。新增双纹理单元共享材质、UV / 颜色 / 拾取 ID 回归代码，cargo check --offline --locked --tests 通过（target/tilemap-material-check.log）；测试断言未执行，真实显卡 sprite_night 调色、画面 / 排序 / 点选和七语言界面待运行验证。未生成本轮 EXE；现有透明模型修复构建不含 Tilemap 材质接入，未发布。

本节优先于下方构建记录。后续构建须明确授权并重新检查占用；下一步验证 Level_101.unity 的 LandTilemap / sprite_night 材质调色。

---

# 最新构建：1.1.8 静态模型透明度（2026-10-14）

- 2026-10-14 用户明确授权编译后，检查无 unity_yaml_viewer 进程且目标 EXE 独占访问通过，cargo build --release --offline --locked 成功（18.88 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本均为 1.1.8，19411456 字节，SHA-256：72097E277A26F55035D6992BD504A32852D3EB0D7779189FB8D7CE2C4857FD1B。包含本轮 PF3D2 主色 / Alpha / 颜色公式与静态 FBX 按名称匹配多材质槽修复，以及此前轴向 / 缩放等累计修改。编译检查、七语言翻译与 git diff --check 通过，保留 Spine 158 条既有警告；新增测试断言未执行。未启动新 EXE，海豚半透明、蜻蜓透明翅膀、材质槽效果及七语言界面仍待运行验证；完整多 Pass / 剔除 / 场景深度和透明表面叠加仍未还原，未发布。

本节优先于下方未构建描述。下一步在 Level_102.unity 核对海豚半透明与蜻蜓透明翅膀。后续构建仍需明确授权并检查占用。

---

# 最新源码：静态模型材质透明度（2026-10-14）

- 2026-10-14 静态模型半透明修复：用户确认上一构建的海鸥 / 海豚 / 蜻蜓尺寸与方向正确，但海豚和蜻蜓翅膀显示实色。定位到静态模型只读取 _Color，遗漏 PFCore/PF3D2_Default / PFCore/PF3D2_Transparent 的 _MainColor 与 _AlphaScale，且静态 FBX 代理只保留首个材质。现按 FBX 材质名称匹配 importer externalObjects 并保留全部槽；匹配不到保留空槽，不按顺序猜测。已识别的这两个 Shader 使用其纹理、主色、Tint、AlphaScale、亮度 / 饱和度 / 对比度、环境色和 Overlay 公式；未知 Shader 仍走原有近似材质路径。海豚材质主色 Alpha 约 0.102，翅膀 0.4，均与贴图 Alpha 相乘。仍是静态 CPU 最近表面烘焙，不代表完整 Shader 执行、多 Pass / Cull / ZWrite / ZTest、多个透明表面或跨对象场景深度的还原。新增材质名称 / 槽顺序与 Alpha 合成回归代码；cargo check --offline --locked --tests 通过（target/model-alpha-check.log），本轮未执行测试断言，实际半透明效果及七语言界面待运行验证。本轮尚未生成 EXE，现有 1.1.8 EXE 不含此修复，未发布。

本节优先于下方构建记录；后续生成 EXE 仍需明确授权并检查占用。下一步验证 Level_102.unity 的海豚半透与蜻蜓透明翅膀。

---

# 最新构建：1.1.8 渲染系统（2026-10-14）

- 2026-10-14 用户明确授权后，重新检查无 unity_yaml_viewer 进程且目标 EXE 独占访问通过，cargo build --release --offline --locked 成功（20.78 秒）。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本均为 1.1.8，19375616 字节，SHA-256：AD4621340B2A758FAE4238BD596BA351D746B5969FA5314FBD74E3930B244FE3。包含 FBX 轴向转换、静态蒙皮与蜻蜓根骨骼均匀缩放修复及此前累计修改。cargo check --offline --locked --tests 与 git diff --check 通过；保留 Spine 158 条既有警告。未启动新 EXE，实际场景朝向、尺寸、贴图、透明翅膀、深度 / 排序及七语言界面仍待运行验证，未发布。

本节优先于下方构建前交接。下一步请在 Level_102.unity 核对海鸥、海豚、蜻蜓朝向与蜻蜓尺寸，以及透明翅膀表现。后续重新构建仍需用户明确授权并重新检查占用。

---

# 新会话交接：1.1.8 静态 FBX 轴向与缩放（2026-10-14）

本节优先于下方历史交接。请先读工程 AGENTS.md；保留用户已有提交和未提交改动。

## 当前版本、分支与产物

- 工作目录：C:\Git\UnityYAMLViewer；分支 render_system；当前 HEAD：6bdcfc1（* 渲染系统）。不是所有渲染工作都未提交：用户已经提交此前工作。
- Cargo.toml / Cargo.lock 当前源码为 1.1.8，版本整理已在 HEAD 中。渲染系统记录迁至 docs/versions/1.1.8.md，1.1.7 保留此前媒体 / 界面记录；原已合并的旧 1.1.8 开发记录单独放入 docs/versions/archive/1.1.8-merged-into-1.1.7.md。
- 当前未提交文件：HANDOFF.md、README.md、docs/versions/1.1.8.md、src/main.rs、src/model_preview.rs、src/static_preview/prefab.rs、src/translations.tsv。开始工作时重新核对 git status。
- 现有 target/release/unity_yaml_viewer.exe 仍为 1.1.7，19378176 字节，SHA-256：972E85DAE0E9EA865C93855D8EB86E7EEAE3CA6CA0810688316F0492611D8F69。
- 现有 EXE 包含首次静态海鸥模型支持、Unicode 中文名称显示、Prefab 诊断复制、此前运行时 Shader 与预览修改；不含本轮 FBX 轴向 / 蜻蜓缩放修复，也不是 1.1.8 构建。尚未发布。

## 用户最新问题与已修改源码

用户在 C:\Git\g003_client\Assets\Scenes\Level_102.unity 中确认海鸥、海豚、蜻蜓已经出现，但角度不对，只有蜻蜓尺寸严重放大。截图已在本会话中给出。

1. src/model_preview.rs：新增 fbx_axes_point，依据 FBX right/up/front 把源坐标转换为 Y-up，再由 unity_point 转换手性并应用场景世界矩阵。真实三个 FBX 均为 right=PositiveX、up=PositiveZ、front=NegativeY，unit_meters=0.01；旧路径仅翻 X，没有处理 Z-up。
2. 静态导入路径新增 load_evaluated，开启 ufbx evaluate_skinning；读取 skinned_position，按 skinned_is_local 判断是否应用 geometry_to_world。普通模型与已有显式骨骼姿态路径维持原逻辑。
3. 蜻蜓：Assets/Res/Prefabs/Map/Npc/dragonfly_red.prefab 将 FBX 实例根缩放设为 20000，同时把源骨骼对象 3805688988141419503 三轴缩放覆盖为 1；实际 FBX RootBone 默认缩放约 100。此前静态代理未恢复骨骼覆盖，产生约 100 倍放大。
4. src/static_preview/prefab.rs 的 static_skin_template：只有一个完整非根均匀缩放覆盖、一个明显非单位且均匀缩放骨骼，并且全部蒙皮骨骼均为该骨骼后代时，才生成 preview_FbxBoneScaleRatio。本例为 0.01；烘焙围绕该骨骼世界枢轴应用比例。此方案依靠唯一性与结构确认，不是完整 Unity importer fileID 映射，不应扩展到存在歧义的多骨骼覆盖。
5. 本轮更新 README 中英能力限制、1.1.8 记录、关于摘要及七语言翻译。关于默认 1.1.8 / 1.1.7，其余历史折叠，重新打开恢复折叠。

## 验证状态

- cargo check --offline --locked 通过：target/axes-check.log。
- real_seagull_static_pose：真实海鸥模板、材质 / 贴图及非空静态烘焙测试通过，target/static-axes-test.log。
- inspect_static_fbx_axes：真实海鸥、蜻蜓、海豚轴声明和默认蒙皮数据检查通过，target/fbx-axes.log。默认骨骼姿态下 raw / evaluated 顶点范围基本相同，单独开启 skinning 不是蜻蜓尺寸修复原因；修复来自根骨骼覆盖比例。
- real_dragonfly_root_scale_override：验证实际生成比例 0.01，通过，target/dragonfly-scale-test.log。
- git diff --check 通过。既有 Spine 158 条警告，增量缓存 hardlink 警告非错误。
- 未运行实际新画面；模型朝向、尺寸、贴图、透明翅膀、深度 / 排序仍待用户检查。未生成本轮 EXE。

## 下一步

- 若用户要求编译：必须先检查 unity_yaml_viewer 进程及目标 EXE 独占打开，确认无占用后 cargo build --release --offline --locked；若占用，不能自动关闭或更换输出绕过，需用户关闭后再次明确确认。本次交接请求不含编译授权。
- 构建后更新 docs/versions/1.1.8.md 与 HANDOFF.md 的产物版本、大小、哈希，明确未运行验证 / 未发布。
- 让用户核对三类模型的朝向与蜻蜓尺寸；如仍有问题，继续追踪实际 FBX 与覆盖，不使用物种固定角度 / 倍率。
- 蜻蜓翅膀曾显示灰白，透明材质 / alpha 表现尚未核验，不能声称已修好。

## 既有能力与剩余限制

- 实时 GPU 渲染有限 Built-in 单 Pass CG/HLSL→GLSL，不是 Unity 完整编译器；URP / ShaderGraph、完整光照 / 深度、粒子模拟未接入。
- TEXCOORD1/2 标量、2～4 分量输入暂填零，用户明确接受，未来粒子 Custom Vertex Streams 接入真实数据。
- clip 片元裁剪已修复；Stencil 用户要求忽略遮罩但继续 Shader，并保留提示。复杂 multi_compile PIXELSNAP_ON / 条件实例化 Shader 仍有材质不可用，未完整解决。
- 单网格多材质 GPU 按槽绘制，多余材质叠加最后子网格；CPU / 3D 额外槽规则未全面还原。
- 嵌套 FBX 静态代理当前限单蒙皮网格且唯一 stripped 根 Transform / SkinnedMeshRenderer 源 ID。尚未完整展开骨骼层级 / 还原骨骼旋转与位置覆盖 / 动画；本次只新增满足严格条件的根骨骼均匀缩放。
- 原生三角文本 Mesh 支持本地 fileID；线拓扑仍不支持，模型 GUID 缺失与其他骨骼绑定问题仍可能存在。
- Prefab 模板 / 生成文本各 512 MiB、每层 50 万文档；真实 g001 new_home_decoration 从 11 恢复 14 个子节点，生成约 308 MiB / 90938 文档。诊断上限 256 条，预算诊断优先保留。
- 纹理总预算 CPU / GPU 各 2 GiB，并非进程总内存上限；纹理按路径共享，部分模型 / 烘焙缓存仍可能独立占用。
- GPU 首次渲染必须完成；后续鼠标离开画面 / 应用失去焦点复用最后帧，返回恢复。缓存颜色、DPI / 缩放 / 恢复实际运行验证仍有限。
- 项目索引优先磁盘缓存，后台每启动每项目扫描一次，手动可重扫；Assets / Packages / Library/PackageCache 索引。同项目不同目录用规范化路径 SHA-256 区分缓存。
- tests/fixtures 被测试 include_str! 引用，提交源码须带上；分发 EXE 不需带测试目录。

---
# 静态模型轴向 / 缩放修复（1.1.8 源码）

- 2026-10-14 静态模型轴向 / 缩放修复：按 FBX right/up/front 转换至 Y-up 后转换 Unity 手性，并计算静态蒙皮数据。真实海鸥 / 蜻蜓 / 海豚均为 Z-up、单位 0.01。蜻蜓 RootBone 缩放约 100，Prefab 覆盖为 1；仅在唯一完整均匀缩放覆盖、唯一缩放骨骼且全部蒙皮骨骼均为其后代时应用相对倍率（本例 0.01）与枢轴修正，不使用物种固定倍率。其他骨骼覆盖 / 动画仍未支持。真实轴向 / 蒙皮检查、海鸥烘焙与蜻蜓比例回归通过，cargo check 通过；实际场景朝向 / 尺寸 / 透明翼效果和七语言界面待验证，未生成 1.1.8 EXE。

---

# 1.1.8 渲染系统版本整理（2026-10-14）

源码版本已升级至 1.1.8；渲染记录迁入 docs/versions/1.1.8.md，1.1.7 保留非渲染历史，旧 1.1.8 合并历史单独归档。关于默认显示 1.1.8 / 1.1.7，其他默认折叠。未生成 1.1.8 EXE、未发布；现有 EXE 仍为此前 1.1.7 构建，哈希见下方历史。

---

# 最新静态模型测试构建（2026-10-14）

- 2026-10-14 用户再次明确确认编译，检查无主程序进程且 EXE 独占访问通过，Release 构建成功。版本 1.1.7，19378176 字节，SHA-256 972E85DAE0E9EA865C93855D8EB86E7EEAE3CA6CA0810688316F0492611D8F69。包含单蒙皮 FBX 静态导入姿态、中文 Unicode 名称显示、Prefab 诊断复制及此前累计修改。真实海鸥网格 / 贴图烘焙回归与 cargo check 通过，保留 Spine 158 条既有警告。未启动本次 EXE；实际场景模型朝向 / 大小 / 排序、剪贴板、中文名称及七语言界面待运行验证，骨骼覆盖和动画仍未支持，未发布。下方未生成描述属于构建前状态。

---

# 单蒙皮 FBX 静态导入姿态（2026-10-14）

- 2026-10-14 单蒙皮网格嵌套 FBX 静态导入姿态：通过唯一 stripped Transform / SkinnedMeshRenderer 源 ID 建立根节点与 Renderer，加载 FBX 默认几何姿态、导入单位缩放和外部材质映射，继续应用根变换 / 材质 / 排序覆盖；复用模型烘焙与 3D 合成路径。当前只支持一个蒙皮网格且源 ID 唯一的模型，未展开内部骨骼节点，未还原单骨骼覆盖及动画，保留明确提示，不代表完整 Unity 姿态还原。真实 g003 seagull_skin.fbx 模板 / 静态烘焙测试通过，cargo check 通过；实际场景大小 / 朝向 / 贴图 / 排序及七语言界面待运行验证，未生成本轮 EXE。

---

# 最新测试构建（2026-10-14）

- 2026-10-14 用户明确确认后检查无主程序进程且 EXE 独占访问通过，Release 构建成功。版本 1.1.7，19383808 字节，SHA-256 13B503D0EA2FC012B3D514EA89392CDBD58247A4B05B5FD612A316D09E3838D1。包含 TEXCOORD1/2 零值输入、片元 clip、单网格多材质、嵌套节点定位、512 MiB 展开预算及首次渲染后离焦冻结等累计修改。cargo check 与此前针对性回归通过，Spine 保留 158 条既有警告。未启动本次 EXE；实际显卡编译、画面、冻结恢复及七语言界面待用户运行验证，未发布。下方未生成描述为此前状态。

---

# 展开预算与离焦冻结（2026-10-14）

- 2026-10-14 提高 Prefab 源模板与生成文本预算至各 512 MiB，每层文档上限至 50 万，预算诊断优先保留。真实 Level_101_Scene 对照测试：旧 256 MiB 时 new_home_decoration 为 11 个子节点，新预算生成约 308 MiB / 90938 文档，恢复为 14 个子节点。GPU 首次加载与渲染完成后，鼠标离开画面或程序失去焦点时复用最后一帧，返回恢复；明确相机或内容更新允许刷新，冻结时 FPS 显示 —。窗口缩放保持缓存画面比例，缓存额外占用一张画面的 GPU 内存。6 项 Prefab 回归、真实场景预算对照、画面比例测试和 cargo check 通过；实际显卡颜色 / 方向、首次离焦渲染、DPI / 缩放、恢复交互及七语言界面待运行验证。未生成本轮 EXE。

最新 Release 仍为此前构建，本轮尚未生成 EXE。

---

# 展开预算与离焦冻结（2026-10-14）

- 2026-10-14 Prefab 展开预算与预览冻结：源模板及生成文本预算提高到 512 MiB，每层文档上限提高到 50 万；预算诊断优先保留，避免被其他提示截断。真实 g001 Level_101_Scene 对照测试：旧 256 MiB 时 new_home_decoration 有 11 个子节点，新预算生成约 308 MiB / 90938 文档，该组恢复 14 个子节点。GPU 预览首次加载与首次渲染完成后，鼠标离开画面或程序失去焦点时复用最后一帧，返回画面恢复更新；初次渲染和明确相机 / 内容更新允许刷新，冻结时 FPS 显示 —。缓存随场景替换释放，窗口缩放保持画面比例；额外缓存占用一张画面的 GPU 内存，不计作 2 GiB 程序总上限。6 项 Prefab 回归、真实场景预算对照、缓存画面比例测试及 cargo check 通过；实际显卡缓存颜色 / 方向、首次离焦渲染、DPI / 窗口缩放、恢复交互和关于七语言界面待运行验证。未生成本轮 EXE。

最新 Release 仍为此前缓存 / Packages 构建；本轮源码修改尚未生成 EXE。

---

# 片元 clip 兼容（2026-10-14）

- 2026-10-14 片元 clip 兼容：HLSL clip 调用重定向到独立 helper，标量 <0 或向量任意分量 <0 时执行 GLSL discard，零值不丢弃，不删除透明裁剪逻辑。共享 CG helper 同时进入两个阶段，顶点阶段提供未使用函数所需的空定义；先检查顶点入口可达调用图并合并重载调用，若顶点可达 clip 则明确拒绝，不静默取消顶点语义。16 项 Shader 回归与 cargo check 通过，含片元调用保留和顶点可达检查。日志中 TEXCOORD1 输入、复杂 instancing 条件宏、g001 外部模型 / 骨骼等仍待后续；Stencil ignored 属于继续 Shader 的提示，不是回退。实际显卡 Alpha 裁剪与关于七语言界面待验证，未生成新 EXE。

---

# 嵌套 Prefab 场景点选定位（2026-10-14）

- 2026-10-14 场景点选嵌套 Prefab 的 Hierarchy 定位：展开模式中把拾取的场景 PrefabInstance 代理 ID 映射到该实例独立的实际根 GameObject，stripped 引用也从展开数据定位，不再把实例文档 ID 当作树行 ID。选中展开节点后清除搜索过滤，自动展开祖先、滚动并高亮，Inspector 进入同一展开节点的只读属性。同源多个实例保持独立，不定位到另一副本；祖先集合每次定位按树数据统一计算，避免逐行反复查整个文档。节点映射 / 多实例隔离 / 祖先检查通过，cargo check 通过；真实 UI 的滚动、折叠动画、深层节点和关于七语言布局待验证，未生成新 EXE。

---

# GPU 网格多材质槽接入（2026-10-14）

- 2026-10-14 同一 Mesh 双材质修复：用户场景 MeshRenderer 213277026 对应 l102_river1.asset 只有一个三角子网格，旧 GPU 路径仅取首槽。现读取最多 64 个 MeshRenderer 材质槽并分别准备纹理 / 参数 / Shader，按槽序提交绘制：槽与子网格对应，多余槽复用最后子网格顶点 / 索引 Arc 并叠加绘制，维持 Renderer 排序。单槽失败保留其他槽、报告诊断；全槽不可用或几何失败保留既有整对象烘焙回退；材质槽不足时未匹配子网格不绘制并提示。GPU 解除单子网格限制，但 CPU 回退与 3D 预览的额外槽叠加仍未实现；多 Pass / 深度 / Stencil 等既有限制保留。一个子网格两材质回归检查通过，验证两次提交、独立颜色顺序及几何缓冲共享；实际水面第二层效果与关于七语言界面待验证，未生成新 EXE。材质多于子网格重复最后子网格规则参考 Unity MeshRenderer 手册：https://docs.unity3d.com/cn/2022.2/Manual/class-MeshRenderer.html 。

---

# 非实例化径向风与本地 Mesh 引用（2026-10-14）

- 2026-10-14 g003 最后两类提示排查：sprite_radial_wind 的 multi_compile_instancing 声明在当前逐 draw 预览中忽略，保留非实例化风动顶点公式，补充标量 rsqrt，实际绑定主纹理提供 _MainTex_TexelSize；依赖 instancing 条件 / 缓冲宏的其他 Shader 仍可能拒绝，非 GPU 批量实例化支持。场景 MeshFilter 295257948 实际引用本文件 Mesh 2123986730（Canvas_NoFill_Object，topology=3），并非缺失 GUID；2D 烘焙 / 3D 场景 / GPU 原始路径按本地 fileID 解析文本 Mesh，支持既有三角拓扑范围，线拓扑明确报告限制、不误报 GUID。15 项 Shader 回归和模型预览回归通过（含本地三角 Mesh 与线拓扑诊断），cargo check 通过；实际风动形变 / 坐标 / 混合与关于七语言界面待运行验证，线网格渲染未实现，未生成新 EXE。

---

# 索引磁盘缓存与 Package 支持版已构建（2026-10-14）

- 2026-10-14 用户明确确认后检查无主程序进程及 EXE 独占访问通过，Release 构建成功。版本 1.1.7，19365376 字节，SHA-256 `7308F53BCF205EEE69C1F7A1648E2BC25FDFFC2B682CB1FBE9E45C25ACB667B3`。包含项目索引磁盘缓存优先加载 / 后台刷新、Packages 和 PackageCache Shader / Include 引用、sincos、Blend Off 及已知标量分量截取兼容。此前 17 项脚本 / 缓存回归、14 项 Shader 回归及 cargo check 通过。未启动本次 EXE；真实重启缓存速度、后台更新 / 手动扫描、g003 显卡画面仍待运行验证，未发布。下方未生成描述为构建前状态。缓存文件以规范化项目根目录路径的 SHA-256 命名，不同目录副本分别保存。

产物：target/release/unity_yaml_viewer.exe。

---

# PFW.VisualFX 等包引用与水面兼容（2026-10-14）

- 2026-10-14 按后续 g003 / PFW.VisualFX 包结构接入 Shader 路径：编译入口允许 Assets、Packages、Library/PackageCache；相对 Include 和 Packages 虚拟路径可解析到同源 / 唯一缓存包，真实路径仍须位于上述项目资源目录，同名多版本无法确定时报错，不放开任意外部文件。组件脚本 GUID / 类 / 属性索引原有扫描已经包含 Packages 与 PackageCache，本次磁盘缓存也保留这些结果；不能执行 Unity 脚本生命周期或还原仅 DLL 的未知类型。增加标量 / 向量 sincos 输出参数兼容、Blend Off 使用 One/Zero，并针对水面 dis(float4)参与 uv.y 标量算术赋值按已知类型截取首分量。14 项 Shader 回归通过，包含包入口、相对 Include、PackageCache 虚拟路径及越界拒绝（因受限环境拒绝 Windows canonicalize，使用正常权限验证成功）；实际 g003 显卡效果、复杂包依赖 / instancing 仍待验证。尚未生成新 EXE，当前 Release 不含索引缓存及本轮包兼容。

---

# 项目索引磁盘缓存接入（2026-10-14）

- 2026-10-14 项目索引磁盘缓存：保存资源 GUID 路径、脚本 GUID、类 / 继承 / 字段 / Range、枚举及 Shader 属性索引到 APPDATA/UnityYAMLViewer/project_indexes，以项目路径哈希区分，校验缓存格式版本、应用版本和项目标识。首次访问项目由后台线程优先读取旧缓存并提供资源与脚本查询，继续后台扫描；刷新完成前保留旧目录与属性索引，完成后一次替换。会话内项目状态保留，同项目文件切换不自动重扫，每次启动每项目最多一次自动扫描；手动“重新扫描项目索引”可再次扫描，进行中禁用重复请求。缓存写入使用临时文件 / flush / sync / rename，损坏、错项目或不兼容缓存忽略并重扫；扫描有异常时不覆盖已有磁盘缓存，写入失败提供提示。旧缓存可能暂时含已删除或更改的引用；缓存上限 256 MiB，不是纹理预算。17 项脚本属性 / 缓存回归及 cargo check 通过，包含往返、已有文件替换、错项目 / 损坏拒绝、异常扫描保留旧缓存及刷新期间类型可查询。真实大项目重启读取速度、后台扫描替换、手动刷新和关于七语言实际界面待验证；尚未生成新 EXE。

缓存加载 / 保存均在工作线程，既有内存索引在手动刷新时继续可用。当前 Release 仍为此前海水 Shader 修复版，未包含本次索引缓存。

---

# 海水 Shader 兼容修复版已构建（2026-10-14）

- 2026-10-14 用户明确确认编译后，检查无主程序进程且 EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。版本 1.1.7，19161088 字节，SHA-256 `91F9CAB83AA9A7D808255B02577BB4D31FEB3B39163E5A4C50D2BBD3795D852D`。包含海水向量构造 / 截取、float2x2 / float3x3 类型、函数内 uniform 局部副本、缺省 Blend One/Zero、简单 Include Guard 等最新兼容修复。12 项 Shader 回归与 cargo check 此前通过。未启动本次 EXE；实际海水显卡编译和画面待用户验证；instancing 条件宏、模型 GUID / 骨骼绑定等仍未完整支持，未发布。下方未生成描述为构建前状态。

产物：target/release/unity_yaml_viewer.exe。

---

# 海水向量 / 局部 uniform / 默认状态兼容（2026-10-13）

- 后续海水 Shader 兼容：缺省 Blend 按 Unity 不透明默认 One/Zero 处理，显式但无法解析的 Blend 仍报错；支持简单外层同名 #ifndef/#define/#endif Include Guard 及数字常量 #define（非完整预处理器）；补充 float2x2 / float3x3 类型映射。向量初始化通过显式构造器截取 / 标量广播；函数内检测到 uniform 写入时先创建局部副本，保持 GPU uniform 只读；单分量赋值里的裸 _Time 按 .x 处理。12 项 Shader 回归通过，实际显卡编译、矩阵约定、重复 Include 与复杂条件宏仍需验证；instancing 指令及其宏仍未完整接入，模型 / 骨骼问题未修复，未生成新 EXE。

---

# 紧凑状态与水面兼容修复版已构建（2026-10-13）

- 用户明确要求编译后，检查无主程序进程、EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。版本 1.1.7，19111424 字节，SHA-256 `159CAD1B94EC05FE9FB4E39983F074D9921B11435F51C5FA790636242F34E630`。包含 Cull[_Cull] / ZWrite[_ZWrite] 紧凑状态解析及同一行状态处理、水面 _Time 标量截取 / Luminance(vec3) / lerp 类型兼容、无雾宏处理、未使用标量忽略和未知已引用标量默认 1。11 项 Shader 回归与 cargo check 此前通过。未启动本次 EXE；实际剩余回退及画面待用户验证，instancing 宏、模型 GUID / 骨骼绑定等仍未完整支持，未发布。下方未生成描述为构建前状态。

产物：target/release/unity_yaml_viewer.exe。

---

# 水面类型、紧凑状态与缺省标量兼容（2026-10-13）

- 修复紧凑 ShaderLab 参数语法 Cull[_Cull] / ZWrite[_ZWrite]，逐命令解析避免同一行其他状态被吞并；修复已知裸 _Time 向量赋给标量时按 HLSL 首分量 .x 截取（不擅自改为 .y），保留显式时间分量。增加 Luminance(vec3) 与标量 / 向量 lerp 重载；Unity 雾传递 / 应用宏在当前无雾预览中为空操作，不宣称还原雾。11 项 Shader 回归通过。用户指出 pf_river_2d 的 _Shininess 仅声明，确认无引用，不再因该缺值回退；按用户要求，对未知且已引用、没有材质 / Shader 默认值的标量绑定 1 并提示，已知显式值保留，不能从文件恢复 Unity 脚本运行时 Global。Instancing 条件宏、模型 GUID / 骨骼绑定等仍待支持。实际显卡水面效果、缺省参数影响及关于七语言布局待验证，未生成新 EXE。

---

# 材质 Blend 与 Stencil 忽略版已构建（2026-10-13）

- 用户明确确认后检查无主程序进程、EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。版本 1.1.7，19078144 字节，SHA-256 `2D7271573E2D2A02854BE98D5EF771492DF6C0E94A45E8E0312C270F26D2E2A5`。包含材质参数式 Blend / Cull / ZWrite / ZTest 解析、完整常用 Blend 因子映射，以及忽略 Stencil 遮罩并报告提示的策略。10 项 Shader 回归与 cargo check 此前通过。未启动本次 EXE；实际剩余 Shader 回退、混合效果和界面待用户运行验证，其他不支持 CG 指令 / 状态、模型 GUID / 骨骼绑定问题仍未解决，未发布。下方未生成描述为构建前状态。

产物：target/release/unity_yaml_viewer.exe。

---

# Stencil 预览忽略策略（2026-10-13）

- 按用户要求，预览忽略 ShaderLab Stencil 块，不再仅因 Stencil 而回退普通贴图；保留 Shader 公式和 Blend 执行，材质准备成功时报告遮罩未还原的诊断。其他不支持状态、CG 指令、材质缺失或 GPU 编译失败仍可能回退，尤其 pf_sprite_add_light 的 instancing / 关键字预处理兼容尚未完整实现，不宣称忽略 Stencil 后所有 Shader 均可运行。10 项 Shader 回归通过（含公式未被替换检查），实际显卡 / 遮罩差异与关于七语言界面待验证；未生成新 EXE。

---

# 材质驱动 Blend 状态接入（2026-10-13）

- 材质参数式 ShaderLab 状态接入：运行时 Shader 编译前读取属性标量默认值及材质保存标量（含 m_Floats / m_Ints 条目），以保存值优先解析 Blend / Cull / ZWrite / ZTest 的方括号参数。Blend 支持 Unity 0..10 因子映射并由 GPU 执行；缺失 / 非整数 / 越界状态值报告错误，不猜测。Cull 仍仅 Off/Back、ZWrite 仍仅 Off；ZTest Off/Always 可在当前无深度测试的 2D 路径执行，已有 LEqual 为排序预览兼容，完整深度与 Stencil 尚未支持。9 项 Shader 回归及 cargo check 通过。实际工程材质准备检查通过，但本执行环境部分 Include 路径解析拒绝访问，相关路径仍回退，不能确认完整效果。实际显卡材质混合与七语言界面待验证；模型 GUID 和骨骼绑定错误未修复，未生成新 EXE。

---

# Prefab 展开优化与 Shader 兼容版已构建（2026-10-13）

- 用户明确确认编译后，检查无主程序进程及 EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。最新版本 1.1.7，19082240 字节，SHA-256 `3668062BC658E1C8CCE17AE930D14BAEC31BBF0C6DAFAA05200D3179E90FC002`。包含嵌套 Prefab 引用索引 / 模板共享优化、已知向量乘标量缩窄转换、Assets 内相对 Include 路径兼容，以及此前 2 GiB 预算 / 夜色 / FPS / 鼠标缩放。4 项普通 Prefab 回归、3 项实际工程 Prefab 回归和 8 项 Shader 回归此前通过。未启动本次 EXE；真实地图展开耗时、显卡编译和画面待验证；模型 GUID / 骨骼绑定及其他不支持状态仍有回退，未发布。下方未生成记录为构建前状态。

产物：target/release/unity_yaml_viewer.exe。

---

# Shader 窄化及相对 Include 兼容（2026-10-13）

- 针对后续 vec4→vec3 / Unsafe include 日志：新增已知向量函数参数乘已知标量的缩窄初始化转换，生成 .xy / .xyz，未扩展为完整 HLSL 表达式类型推导；8 项 Shader 回归通过。相对 Include 允许父目录引用，但先 canonicalize 并验证实际目标仍在项目 Assets 中（含链接解析）；绝对路径及越界引用继续拒绝，缺失引用报告具体路径。其他 Shader 指令 / 渲染状态可能仍回退，模型 GUID / 骨骼绑定缺失尚未修复。关于七语言、README 中英同步，实际显卡编译与 Include 路径运行验证待完成，未生成新 EXE。

---

# 嵌套 Prefab 展开性能优化（2026-10-13）

- 嵌套 Prefab 展开性能优化：每层一次建立 m_PrefabInstance 引用索引，替代每个实例扫描全部文档；同源已解析模板使用 Arc 共享，避免每实例复制模板文档列表。实例输出仍独立重映射 fileID、应用覆盖和移除对象，保留取消 / 深度 / 资源预算限制。4 项普通回归与 3 项实际工程回归（海滩 FBX、失效覆盖、场景花坛 SortingOrder）通过，cargo check 通过。真实大场景展开耗时改善尚未测量，不宣称具体倍数；关于摘要七语言及 README 中英同步，实际界面未验证，未生成优化版 Release EXE。

---

# 2 GiB 纹理预算与预览交互修复版已构建（2026-10-13）

- 用户再次确认编译后检查无主程序进程、EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。最新版本 1.1.7，19054080 字节，SHA-256 `3DB16A3313B9042B31E19235901A5D65432F5CBB95C1C5A8AF43C2C6E2207ED4`。包含 2 GiB 场景纹理总预算、UTF-8 BOM 夜色绑定修复、悬浮 FPS 与鼠标位置缩放。未启动本次 EXE；大地图资源完整性、实际内存占用、夜色 / FPS / 缩放交互及关于窗口七语言仍待运行验证，未发布。下方未生成描述为此前构建前状态。

产物：target/release/unity_yaml_viewer.exe。

---

# 场景纹理总预算提高到 2 GiB（2026-10-13）

- 按用户要求将场景预览纹理总预算由 256 MiB 提高到 2 GiB：世界 2D CPU 加载 / 缓存、GPU 场景源纹理，以及静态模型 / Spine 合成总缓存和常规预览上传预算同步采用共享常量；超限提示同步。单贴图尺寸、解码器分配限制、单份 Spine 解码及额外 Shader 材质纹理预算保持原有独立限制。CPU / GPU 仍各自计量，2 GiB 不代表程序总内存上限，也不解决模型缺失或其他 Shader 编译错误。cargo check 通过；真实大地图加载、内存占用与关于七语言实际界面待验证。未生成本轮 EXE，前次占用阻塞后尚需用户再次明确确认构建。

---

# 夜色绑定 / 悬浮 FPS / 鼠标位置缩放（2026-10-13）

用户确认泡沫已经可渲染。黑色原因定位到昼夜 Include 的 UTF-8 BOM，使首项 _GlobalDayNightColor 未转为 uniform；转换前清除 BOM，昼夜全局颜色在材料读取后设 RGBA=1。海岸 uniform 检查与 8 项 Shader 回归通过，白色绑定 / 鼠标锚点 / FPS 采样检查通过。

画布左上角悬浮 FPS 每半秒更新预览提交帧率，不代表 GPU 完成时间；滚轮围绕鼠标缩放。README 中英、当前版本记录、关于七语言已同步。实际画面和交互待运行验证。

Release EXE 独占访问失败，本轮未生成新 EXE；需要关闭占用程序并由用户再次确认编译，随后重新检查。当前 Release 仍为上一轮向量截取修复版。

---
# 海岸 Shader vec4→vec2 隐式截取修复，测试版已构建（2026-10-13）

用户显卡日志对应 `float2 inuv = v.uv`。已按函数参数结构类型与字段维度，为确认的向量缩窄初始化添加 `.xy` / `.xyz`，不替换 Shader 效果公式。8 项 Shader 测试通过，七语言摘要同步。

用户再次确认后，重新检查无程序进程及 EXE 独占访问通过，Release 构建成功，保留 158 条 Spine 既有警告。最新 target/release/unity_yaml_viewer.exe 版本 1.1.7，19,046,912 字节，SHA-256 `E214492DB8BA2FB60892E93A1712FCB3E587E37CCF87EEF5DE0E1D29B2F6554A`，包含本次转换修复。未启动应用，实际用户显卡编译 / 海岸效果仍待验证，未发布。其他未支持材质状态尚未处理。

---

# FBX GPU 原始网格格式分流修复（2026-10-13）

最终测试构建已完成：按此前授权检查无程序进程及 EXE 独占访问通过后，Release 构建成功，保留 158 条 Spine 既有警告。最新 `target/release/unity_yaml_viewer.exe` 为 1.1.7，18,999,808 字节，SHA-256 `770E43EDA187DB2B47DFD5A2A80A9152840139799D0EEE870A0777EEF6D62A85`。包含 FBX GPU 分流、Shader 缺省纹理与 5 秒按钮延迟；未启动本次 EXE，实际效果待验证，未发布。下方未生成描述为构建前状态。

同轮追加预览按钮延迟：忙碌满 5 秒显示暂停 / 退出，完成后重置，已暂停时立即显示继续 / 退出。5 秒 / 重置 / 暂停检查通过；状态行固定高度保持。实际运行交互待验证。

修复八条海滩实例 UTF-8 错误：FBX 由 ufbx 二进制解码，原生 .asset 由文本 Mesh 解码。单一未蒙皮网格的原始三角形 / UV0 / 颜色 / 法线进入现有 Shader 绘制，实例世界矩阵沿用展开后的 Transform。Shader 默认纹理可绑定 white / black / gray / bump，海岸空 MainTex 用 Shader 缺省白色，不用烘焙图代替。

四份真实海滩 FBX 原始几何测试通过，Rust 检查通过；当前未生成本轮 EXE、未验证海岸实际效果。参数式状态、Stencil、多节点 / 蒙皮 / GPU 多子网格、完整切线等仍有限制。下方旧产物不包含此修复。

---

# GameObject Active / 状态布局与覆盖诊断（2026-10-13）

后续测试构建已完成：按用户此前授权检查无程序进程、目标独占访问通过，Release 构建成功，158 条 Spine 既有警告。`target/release/unity_yaml_viewer.exe` 版本 1.1.7，18,997,248 字节，SHA-256 `FA6B5261F9E7CB69BC94290C81419FBD3351CDAB274A54E72C048F932D1F0386`。7 项 Shader 测试、固定状态行测试、真实源资源失效覆盖检查通过；未启动本次 EXE，Active 点击 / 保存、窗口布局及Shader实际画面待验证。未发布；下方未生成状态为构建前描述。

用户确认海滩 FBX 层级已还原。1395338311 的三条剩余 m_IsActive 覆盖指向源文件已不存在的对象，诊断补充 fileID / 路径 / 失效原因，保留警告而不生成假节点。

GameObject 新增属性 / YAML 视图，本地 Active 在编辑模式复用撤销与保存链路；展开 Prefab/FBX 仍只读，不支持新建实例覆盖。固定 GPU 路径与合成状态两行，布局检查通过；ShaderLab 支持值大小写不敏感、float4 UV0 与多纹理 ST 已接入，但 FBX 原始网格 GPU Shader 尚未完成。实际 UI 编辑、状态稳定性与海岸效果待验证；当前未生成本轮 EXE。

---

# 单节点 FBX Model Prefab 层级接入（2026-10-13）

后续已按此前授权构建测试版：无主程序进程、EXE 独占访问通过，Release 构建成功，158 条 Spine 既有警告。最新产物 `target/release/unity_yaml_viewer.exe` 为 1.1.7，18,979,328 字节，SHA-256 `E7F36CFB0572AD76EB57AE5342804FC234047117378EDD91545DADBED508EEB7`。未启动本次应用，实际层级与网格显示待验证，未发布；下方未生成描述为构建前状态。

最新源码展开四份海滩 FBX：验证单网格节点后，用已有覆盖 / stripped 类型引用唯一确定源对象，生成 GO/Transform/MeshFilter/MeshRenderer，并按实例 remap 和应用变换、排序、名称、材质槽覆盖。根 Transform 直接接回父节点，不再插入空包装 Transform。

真实场景层级集成检查通过（含用户标出的 Transform、BeachMesh 父级与材质替换）；4 项 Prefab 回归通过。多节点 FBX、复杂导入器映射、FBX GPU 原始网格 / 海岸泡沫 Shader 尚未完成，实际结构与静态显示待运行验证。未生成本轮 Release EXE。此前未解析的 1395338311 overrides 未在本轮修复。

---

# pow 兼容函数遮蔽修复（2026-10-13）

6 项 Shader 回归测试实际运行通过；按此前用户授权检查无程序进程、EXE 独占访问通过后，Release 构建成功（Spine 158 条既有警告）。最新 `target/release/unity_yaml_viewer.exe` 版本 1.1.7，18,958,336 字节，SHA-256 `E67D31854770E08E85764B7EDA209198728D8A7ECCD36B86C8DE966764C16D0B`。未启动本次 EXE，用户显卡编译与海水画面待验证，未发布。

用户提供显卡日志明确定位：同名 pow 兼容函数遮蔽了 GLSL 内建重载，导致海水、焦散等 Shader 失败。已使用独立 unity_compat_pow 名称并提供标量 / 向量重载，源码运算语义不变；新增防遮蔽回归检查。实际显卡编译与画面待验证。默认 Sprite、wave、reflection 等其他未支持材质状态未在本次处理。

---

# GPU 回退隔离与诊断修复（2026-10-13）

后续已按用户此前授权完成测试构建：无主程序进程、目标 EXE 独占访问通过，`cargo build --release --offline --locked` 成功，Spine 保留 158 条既有警告。最新 `target/release/unity_yaml_viewer.exe` 为 1.1.7，18,936,320 字节，SHA-256 `A02B8F9FCA13C81AFC8F30D01240979F72394E9D2C2348D8721A6179357D136A`。未启动此 EXE，实际 GPU 编译日志和效果待验证，未发布；本节下方未重新生成的描述为构建前状态。

用户反馈新测试版显示 GPU 未启用、错误无法展开。已新增可展开 / 复制的诊断、将全局 GPU 错误移至画布上方；Shader 编译失败按程序隔离回退，避免单份失败使全部材质退回 CPU。修复首次多纹理上传覆盖当前单元绑定的问题。

实际工程材质准备检查通过，Rust 编译检查通过；尚未取得用户显卡的具体 GLSL 日志，不能确认截图根因或声称海水已恢复。未重新生成本轮 Release EXE，实际界面、GPU 日志、隔离回退及首帧纹理绑定待验证。下方产物信息为此前版本。

---

# 海水运行时 Shader / 原生 Mesh 测试构建（2026-10-13）

用户再次明确授权编译；重新检查无主程序进程、目标 Release EXE 独占访问通过。`cargo build --release --offline --locked` 成功，保留 158 条 Spine 既有警告。

- 产物：`target/release/unity_yaml_viewer.exe`；文件 / 产品版本 1.1.7；18,983,936 字节；SHA-256：`741BDCC08C533A4BDC2FBDB3ED752D2B2C54AFD8505928AA0F64247FC7E36A83`。
- 包含运行时海水 / 焦散源码转换、多纹理与材质参数、白色昼夜颜色默认值、原生文本 Mesh 及此前移动变形修复。
- 12 项渲染相关测试已通过；未启动本次 EXE，实际 GLSL GPU 编译、海水 / 焦散画面、动态流动、网格方向与移动交互待验证。未发布、未同步发布附件；下方待构建说明及旧产物信息仅代表历史状态。

---

# 海水运行时 Shader / 原生 Mesh 开发，待构建验证（2026-10-13）

最新源码已扩展 Include 展开、多纹理与材质默认值 / 保存值绑定、通用 varying 和常用 CG 到 GLSL 兼容；运行时源码编译范围仍受限，不是任意 Unity Shader 自动编译。昼夜全局颜色按用户确认默认为白色。

海水与焦散 MeshRenderer 已准备内置 Quad 原始几何 / UV / 世界矩阵；原生 `.asset` Mesh 支持未压缩单 stream Float32 / UNorm8、UV0、颜色 / 法线 / 切线、16 / 32 位三角形索引及 fileID。GPU 多子网格、压缩 / 二进制 Mesh、UV1+、URP 等仍未支持。详见 docs/versions/1.1.7.md 与 docs/render-system.md。

12 项渲染相关测试通过，Rust 编译检查通过；未生成本轮 Release EXE、未运行海水实际画面，也未验证上一轮拖动修复。此前 Release EXE 被占用后的等待确认仍未解除授权要求；须用户再次确认，重新检查占用后构建。下方产物哈希仅代表历史版本。

---

# GPU 移动变形修复，待解除占用后编译（2026-10-13）

用户确认上一测试构建的 Shader 效果正确，同时反馈移动时画面变形。已将 GPU 回调视口固定到预览面板，并把缓存图像的平移 / 缩放写入投影，避免窗口边界截断视口造成拉伸。相关 5 项测试实际运行通过；关于摘要补充七语言修复条目，实际交互与布局待验证。

Release EXE 独占访问失败，此次未启动 Release 构建。等待用户关闭程序 / 解除占用后再次确认编译，再重新检查。现有 EXE 不包含本次移动修复；下方哈希为历史产物。

---

# render_system 光斑反馈修复测试构建（2026-10-13）

用户再次明确授权编译，重新检查无 unity_yaml_viewer 进程、目标 EXE 独占访问通过；`cargo build --release --offline --locked` 成功，Spine Runtime 保留 158 条既有警告。

- 最新产物：`target/release/unity_yaml_viewer.exe`；文件 / 产品版本 1.1.7；18,820,608 字节；SHA-256：`25F929161AA7F4F1461E66610DBEB8374B7018EE1B626E3AE2783024EA230D89`。
- 包含 256 MiB GPU 原始纹理预算、显式 Glow 后端以及 GPU / 普通贴图路径状态提示。
- 未启动本次产物；样例画面、实际 GPU 编译及渲染性能仍待用户验证。未发布，未同步发布附件。下方待授权和旧构建信息仅代表历史状态。

---

# render_system 光斑反馈修复，待解除占用后重新编译（2026-10-13）

额外 GPU 原始贴图预算已由 128 MiB 对齐至 256 MiB，预览增加 GPU / 普通贴图路径状态，显式选择 Glow。样例转换 3 项测试实际运行通过，Rust 编译检查通过；未验证实际 GPU 效果。静态资源统计约 129.48 MiB，不能确认截图必然由预算超限导致。

重新构建前发现 Release EXE 被占用；检查脚本异常未及时终止，构建误启动后已中止。等待用户解除占用、再次明确确认后重新检查与构建，不自动关闭程序或更换输出路径。下方成功构建与哈希仅代表此前版本，不包含本次修复。

---

# render_system 分支测试构建（2026-10-13）

本节优先于下方历史交接。用户明确授权编译；构建前未发现 unity_yaml_viewer 进程，目标 Release EXE 当时不存在，无目标文件占用。`cargo build --release --offline --locked` 成功，Spine Runtime 保留 158 条既有警告。

- 产物：`target/release/unity_yaml_viewer.exe`；文件 / 产品版本 1.1.7；大小 18,818,560 字节；SHA-256：`511F99CEC626E9FE12E01D879B4BF37B187A930AC19B030A03C010F0A9D950EA`。
- 包含有限 Built-in Sprite Shader CG 转换、Glow GPU 预览与材质输入绑定；不是完整 Unity Shader / URP 兼容实现。
- 未启动本次 EXE，未执行测试断言或 GPU GLSL 编译，样例效果、运行性能、回退和退出释放待验证；未发布、未更新发布附件。构建日志在 `target/render-system-build.log`，target 内容不受 Git 跟踪。

---

# 合并修订 1.1.7 Release 构建（2026-10-13）

本节优先于下方历史交接。源码及 target/release/unity_yaml_viewer.exe 均为 1.1.7，包含原 1.1.8 合并功能。用户授权后检查无程序进程、目标独占访问通过，cargo build --release --offline --locked 成功；Spine Runtime 保留 158 条既有警告。未启动本次程序、未重新上传发布。

- 文件 / 产品版本：1.1.7；大小：18655232 字节；SHA-256：248D938DEDF76529BCA9737A7A5A258189998017FD0A9EA1301DED6AFC9C1FB2。
- 发布说明已更新本次构建信息。target/publish/v1.1.7 的旧附件未在本轮同步，应在发布前另行更新核对。下方旧版本状态与哈希仅代表历史。

---

# 合并至 1.1.7 重新发布准备（2026-10-13）

用户要求将 1.1.8 内容合入 1.1.7，功能代码保持不变。Cargo.toml / Cargo.lock 当前为 1.1.7；版本记录、发布说明与关于摘要已合并，关于默认显示 1.1.7 / 1.1.6。原 1.1.8 文档归档保留。本次未编译、未上传或重新发布；旧 EXE 与发布目录产物不能作为新版 1.1.7，需用户明确授权后检查占用并构建，重新核对哈希和附件。

---

# 测试通过并恢复 1.1.8（2026-10-13）

本节优先于下方历史交接。用户确认最新临时 1.1.6 测试版验证通过，Cargo.toml / Cargo.lock 已恢复为 1.1.8，保留全部开发改动。本次没有重新编译 1.1.8 EXE；此前测试产物及哈希仅代表其构建时状态，不作为正式 1.1.8 产物。未发布，不将本次用户确认扩展为全面回归通过。后续生成 EXE 仍须明确授权及占用检查。

---

# 最新临时 1.1.6 测试构建（2026-10-13）

本节优先于下方历史交接。源码及 target/release/unity_yaml_viewer.exe 继续临时保持 1.1.6，已包含更新窗口自动收缩高度及“FFmpeg 已安装”七语言文案，保留其他 1.1.8 开发改动。用户明确授权后检查进程及 EXE 独占访问通过，cargo build --release --offline --locked 成功，Spine Runtime 158 条既有警告。等待用户验证后要求恢复，不自动恢复 1.1.8。

- 文件 / 产品版本 1.1.6；大小 18655744 字节。
- SHA-256：AD6DBCF37601DC8FBF30CF0A298816744AEAF30FEEDE2680FDAA6B2601724A28。
- 未运行本次程序，布局及更新流程待验证；非历史 1.1.6 发布产物，下方旧哈希仅代表历史状态。

---

# 解码器状态文案（2026-10-13）

按用户要求仍保持临时 1.1.6，等待验证后再恢复。视频解码就绪标签改为“FFmpeg 已安装”，提供七语言翻译；编译检查通过，未重新生成 EXE。本次文案及此前更新窗口高度修复均尚未包含在测试产物中。

---

# 软件更新窗口高度修复（2026-10-13）

当前源码临时保持 1.1.6。更新窗口已启用内容高度自适应，去除按钮下方默认高度造成的空白；编译检查通过，未重新生成 EXE，运行验证待完成。

---

# 临时 1.1.6 更新检查测试（2026-10-13）

本节优先于下方历史交接。按用户要求，Cargo.toml / Cargo.lock 及 target/release/unity_yaml_viewer.exe 当前均临时为 1.1.6，保留实际 1.1.8 开发功能（系统代理、独立弹窗、历史与布局改进、设置内联显示自动 / 手动检查结果）。等待用户更新检查完成后再恢复 1.1.8，不自动恢复。此产物不是历史 1.1.6 发布版；关于历史保留真实开发版本摘要。

- 构建前进程及文件独占检查通过，cargo build --release --offline --locked 成功；Spine Runtime 158 条既有警告。
- 文件 / 产品版本 1.1.6；大小 18655744 字节；SHA-256：A36B40AD71D9A8EC3BD9467B59BFA262ECE9DD7ABCC1C0BF933241A5AB9CC3DB。
- 未运行本次测试；详情在 docs/versions/1.1.8.md 记录，下方源码版本及旧产物描述仅代表历史状态。

---

# 自动检查结果保留（2026-10-13）

当前 1.1.8 源码自动检查也保留结果：无新版本显示最新版本提示，失败显示错误及链接；未检查不显示结果。关闭自动检查不清除已完成结果，自动发现新版本不主动弹窗。编译检查通过，运行验证待完成；未重新生成 EXE。

---

# 检查更新结果内联显示（2026-10-13）

1.1.8 源码将手动检查结果显示在设置按钮下方；最新版本不显示链接，失败才显示手动更新链接。新版本仍打开与 New 共用的更新窗口；下载 / 安装错误沿用提示。编译检查通过，运行验证待完成，未重新生成 EXE，下方构建不包含此次改动。

---

# 1.1.8 默认两版历史构建交接（2026-10-13）

本节优先于下方历史交接。最新 target/release/unity_yaml_viewer.exe 默认只展示 1.1.8 / 1.1.7，其余版本统一折叠，保留七语言历史。用户明确授权后检查进程及文件占用通过，cargo build --release --offline --locked 成功；Spine Runtime 保留 158 条既有警告。未运行本次程序，实际折叠交互及多语言显示待验证，未发布。

- 文件 / 产品版本：1.1.8；大小：18654208 字节。
- SHA-256：4698FBC7C27C238DDE4C2D18013681886B940E92D8C819CCE0899BBC31714429。
- 关于维护规则已加入 AGENTS.md；下方旧产物信息仅代表历史状态。

---

# 关于历史默认折叠修复（2026-10-13）

源码 1.1.8 默认只展示 1.1.8 / 1.1.7，其他版本统一在“查看更多历史”中展开。保留完整摘要和翻译。编译检查通过，运行验证待完成；未重新生成 EXE，下方构建不包含本次修复。

---

# 1.1.8 历史与布局修正构建交接（2026-10-13）

本节优先于下方历史交接。target/release/unity_yaml_viewer.exe 已包含关于窗口版本摘要补齐与七语言翻译、设置标签列固定宽度、关于窗口内容高度自适应，以及此前独立弹窗和系统代理等改动。用户授权后已检查无主程序进程、目标 EXE 独占访问通过；cargo build --release --offline --locked 成功，Spine Runtime 保留 158 条既有警告。未运行本次程序，多语言布局与历史显示待验证，未发布。

- 文件 / 产品版本：1.1.8；大小：18654208 字节。
- SHA-256：DDE5806A10D417D0C26BF4E4343E07E7DE16DF2B2424E7D7D1FE4160B377C004。
- 下方旧产物哈希及未构建说明仅代表历史状态。

---

# 关于窗口历史补齐（2026-10-13）

当前 1.1.8 源码已补齐关于窗口 1.1.3～1.1.7 摘要，并修正 1.1.8 条目，新增摘要提供七语言翻译。编译检查、翻译列数与重复键检查通过；未重新生成 EXE，下方旧产物不包含本次历史补齐及此前边距 / 关于高度调整。实际显示待运行验证。

---

# 多语言边距与关于窗口高度调整（2026-10-13）

当前源码 1.1.8 固定设置标签列宽，统一多语言右侧留白；关于窗口按内容自动调整高度，宽度可调整。cargo check --offline --locked --tests 通过，运行验证待完成，未重新生成 EXE；下方最新构建不包含本次调整。

---

# 1.1.8 独立弹窗构建交接（2026-10-13）

本节优先于下方历史交接。应用弹窗统一使用独立原生窗口（设置、关于、更新相关、加载进度、编辑确认 / 提示、颜色 / 渐变），可移出主窗口并在内容溢出时滚动。放弃编辑确认保留主窗口输入保护；无关闭状态的窗口仍通过原操作按钮处理。源码及 target/release/unity_yaml_viewer.exe 均为 1.1.8。

- 用户授权处理后直接编译；构建前无主程序进程且目标独占检查通过。cargo check --offline --locked --tests、cargo build --release --offline --locked、git diff --check 通过，Spine Runtime 保留 158 条既有警告。
- 文件 / 产品版本 1.1.8；大小 18660352 字节；SHA-256：389FD57E36E878D3DA0D4211DE030733F08773D936545BDEB22E72C8DC1CB70E。
- 未启动程序，多窗口关闭重开、颜色渐变编辑、语言主题、解码刷新、不同 DPI 布局待运行验证，未发布。下方旧产物说明仅代表历史状态。

---

# 1.1.8 最新布局构建交接（2026-10-13）

本节优先于下方历史交接。target/release/unity_yaml_viewer.exe 已重新编译，包含清空记录按钮靠右及中间滑条按本地化按钮宽度自适应调整。用户明确授权、构建前无主程序进程且目标 EXE 独占检查通过，cargo build --release --offline --locked 成功。Spine Runtime 保留 158 条既有警告。未启动程序，多语言布局待运行验证，未发布。

- 文件 / 产品版本：1.1.8；大小：18546176 字节。
- SHA-256：88A1156A1B94F41E930BFEA997843AC6CD5BAF71307594D8C86186F10CF51733。
- 下方同版本旧哈希及未构建说明仅代表历史状态。

---

# 最近记录布局修改（2026-10-13）

当前源码 1.1.8 已将清空记录按钮右对齐，并按本地化按钮实际宽度分配剩余滑条宽度。cargo check --offline --locked --tests 通过；未重新生成 EXE，下面的 Release 产物不包含本次布局修改。多语言运行验证待完成。

---

# 1.1.8 Release 构建交接（2026-10-13）

本节优先于下方历史交接。源码及 target/release/unity_yaml_viewer.exe 均为 1.1.8，包含最近记录设置与系统代理修复。用户明确授权编译，进程及目标文件独占检查通过，cargo build --release --offline --locked 成功；Spine Runtime 保留 158 条既有警告。未启动本次产物、未全面回归、未发布。用户此前确认临时 1.1.6 通过系统代理升级到线上 1.1.7 成功；最近记录设置运行验证待完成。

- 文件 / 产品版本：1.1.8；大小：18546176 字节。
- SHA-256：8FEED97E2E0D49CBCC8410FDACE9CC77BC4FD646472A9402927CD9ECCDC94FE7。
- 完整记录：docs/versions/1.1.8.md。

---

# 在线更新测试交接（2026-10-13）

本节优先于下方历史交接。当前源码为 1.1.8；此前生成的 target/release/unity_yaml_viewer.exe 是临时标记为 1.1.6 的测试程序，包含系统代理修复和最近记录设置，用于由用户验证升级到线上 1.1.7。cargo build --release --offline --locked 成功，构建前无主程序进程且占用检查通过。用户已确认系统代理联网及 1.1.6 → 线上 1.1.7 更新测试通过，并要求恢复 1.1.8；已核对 Cargo.toml / Cargo.lock 均为 1.1.8，未重新构建 1.1.8 EXE。测试产物不作为历史 1.1.6 或正式 1.1.8 发布产物。升级成功后程序会被线上 1.1.7 替换，其中不包含本轮 1.1.8 开发改动。

- 原始测试产物核对（更新前）：文件 / 产品版本 1.1.6，大小 18546176 字节，SHA-256：18F8BCBE3AA65CC609A5B5A3EF361C5C97DD5D8385311B79A6345E7D00EC007D。

---

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

## 独立阶段节点后续源码（2026-10-15）

已实现 In / 步骤 / Out 依赖布局、仅颜色节点快照、纯计算紧凑节点及显式范围成员。普通与测试编译检查通过，测试断言 / GPU / 交互未运行。本轮未生成 EXE，最新本地 EXE 仍为此前 Vert / Frag 单步版本，SHA-256 06B678024FEAE8F8472B610FF18C7CA310B9B02B07C01DF360DE891CFBD50920；未更新 GitHub。细节见 docs/versions/1.2.0.md。

## 独立阶段节点本地构建（2026-10-15）

- 用户明确授权编译；构建前进程及目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21360640 字节；SHA-256：034B87C46CDEAAF43A7F7B77A80E39CA26B3EB13990DFA7200A8F3886AE5E180。
- 包含独立 Vert / Frag In → 步骤 → Out、颜色节点快照、纯计算无预览区与可选范围框。日志 target/stage-flow-release-build.log。
- 未启动程序；实际 GPU 画面、节点拖动及范围成员交互待运行验证，测试断言未执行。仅更新本地产物，未更新 GitHub 正式发布。

## 普通函数拆分后续源码（2026-10-15）

Helper 已使用独立 In / 步骤 / Out，调用 / 返回连线接具体步骤。数值参数的唯一直接阶段调用可生成颜色快照，复杂上下文仅展示并提示限制。普通与测试编译检查通过，断言及 GPU / 界面未运行，未生成本轮 EXE；本地 EXE 仍为上一轮独立阶段节点版本（SHA-256 034B87C46CDEAAF43A7F7B77A80E39CA26B3EB13990DFA7200A8F3886AE5E180），GitHub 未改。

阶段 / 函数区域底色修正：In / 步骤 / Out 按实际位置绘制淡底色和边界，调用线仍接具体步骤，不改变范围框成员。cargo check 通过，尚未生成新 EXE，实际视觉待验证。

## 函数拆分与区域底色本地构建（2026-10-15）

- 用户明确授权编译；构建前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（24.14 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21403136 字节；SHA-256：69BB02007DA2EFF06FCCC55B09E7944BBCB81E48811BA6F1D5AD2302C13A0EB6。
- 包含普通函数 In / 步骤 / Out、可确定调用上下文的颜色快照、具体步骤的调用 / 返回连线，以及阶段 / 函数区域淡底色和边界。日志 target/helper-scope-release-build.log，保留 Spine Runtime 158 条既有警告。
- 未启动程序；函数 GPU 快照、连线、深浅主题区域视觉与交互待运行验证，新增测试断言未执行。仅更新本地产物，GitHub 正式发布未改。

## 统一分组块源码（2026-10-15）

原阶段 / 函数底色区域成为唯一可移动 / 拉伸分组块，移除重复范围框与开关；In / Out 和明确成员随块移动，非成员保持画布位置，节点可拖出边界。普通与测试编译检查通过，断言 / 实际交互未运行。未生成本轮 EXE，最新本地 EXE 仍为上一轮函数拆分与底色版本，SHA-256 69BB02007DA2EFF06FCCC55B09E7944BBCB81E48811BA6F1D5AD2302C13A0EB6。

Out 文本已改为按内容宽度换行、节点内裁剪，悬停查看全文。cargo check 通过，实际视觉待验证；与统一分组块修改均未生成新 EXE。

## 统一分组块与 Out 换行本地构建（2026-10-15）

- 用户明确授权编译；构建前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（27.33 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21387264 字节；SHA-256：1515AFB08490F90725180C71FC0A0087827AE725EE5CA03C7AEC7B73090BCF09。
- 包含统一可拖动 / 拉伸分组块、移除重复范围框、成员随动 / 非成员保持位置，以及 Out 文本自动换行、内容区裁剪和全文悬停。日志 target/unified-block-output-wrap-release-build.log，保留 Spine Runtime 158 条既有警告。
- 未启动程序；实际分组拖动 / 拉伸、成员交互及长文本视觉待运行验证，新增测试断言未执行。仅更新本地产物，GitHub 正式发布未改。

## ASE 布局与分组常量修正源码（2026-10-15）

默认节点按依赖横向分层，分开输入落点、淡化底色，输出 / 阶段间距随内容宽度计算；缩放最低 10%。分组成员连通判断补入参与输入计算链的上游常量，avgColor 不再漏掉，无关常量仍排除。普通与测试编译检查通过，断言 / 实际运行未验证；未生成新 EXE，本地产物仍为统一分组块 / Out 换行版，SHA-256 1515AFB08490F90725180C71FC0A0087827AE725EE5CA03C7AEC7B73090BCF09。

## ASE 参考布局本地构建（2026-10-15）

- 用户明确授权编译；构建前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（31.15 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21457920 字节；SHA-256：B1C437D63F46018EBEB862C7F64ADBEF798AC653C97034E466A230900632A14C。
- 包含依赖横向分层布局、输入分点连线、淡化分组底色、10% 最小缩放，以及参与计算的局部常量随分组移动修正。日志 target/ase-layout-release-build.log，保留 Spine Runtime 158 条既有警告。
- 未启动程序；实际布局、缩放、常量随动和 GPU 画面待运行验证，新增测试断言未执行。仅更新本地产物，GitHub 正式发布未改。

## 2026-10-16：紧凑运算节点源码

已自行实现表达式解析及紧凑运算图（src/material_graph/operations.rs），宽 200 点，源码悬停，颜色快照默认收起、按需展开，直接 RGB / A 端口及声明宽度转换保留。复杂表达式回退源码语句；引用目录仅供阅读，未复制 ASE 实现。普通与测试编译检查通过，新测试断言、UI / GPU 与交互未运行。没有本轮 EXE；最新本地产物仍为 2026-10-15 ASE 参考布局版，SHA-256 B1C437D63F46018EBEB862C7F64ADBEF798AC653C97034E466A230900632A14C，GitHub 未更新。详见 docs/versions/1.2.0.md。

## 紧凑运算节点本地构建（2026-10-16）

- 用户明确授权编译；构建前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（36.39 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21565952 字节；SHA-256：C833532E0701D43270946C0E7FA5CA864F7A365962CF0C448F9E30A4E781C40A。
- 包含常见表达式运算节点、紧凑布局、源码悬停详情、RGB / A 等对应端口连接、声明宽度转换、按需展开颜色快照和复杂语句回退。日志 target/operation-nodes-release-build.log，保留 Spine Runtime 158 条既有警告。
- 未启动程序；实际紧凑排版、展开 / 收起、拖动、性能和 GPU 画面待运行验证，新测试断言未执行。仅更新本地产物，GitHub 正式发布及 ref 参考目录未改。

基础运算节点收窄源码：基础节点 140 点、其他节点 200 点，默认列距按列宽加 60 点计算，快照仍 128 点。cargo check 通过，未生成本轮 EXE；本地 EXE 仍为上一轮紧凑运算节点版（C833532E0701D43270946C0E7FA5CA864F7A365962CF0C448F9E30A4E781C40A），实际视觉待验证。

基础运算宽度按最终要求改为 100 点；展开预览至少 148 点、图像仍 128 点，输入文字独立裁剪。cargo check 通过，尚未生成新 EXE，实际视觉待验证。

## 100 点宽运算节点本地构建（2026-10-16）

- 用户明确授权编译；构建前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（38.25 秒）。
- 本地产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21624320 字节；SHA-256：B04F62BB30B2260FBCED71D1FF1181BDB8541703827EFF128C617B2F8B159923。
- 包含基础运算默认 100 点宽、随列宽调整默认间距、展开颜色预览时至少 148 点宽及输入文字独立裁剪，预览图仍为 128 点。日志 target/100-point-operation-nodes-release-build.log，保留 Spine Runtime 158 条既有警告。
- 未启动程序；实际窄节点文本、展开 / 收起及连线视觉待运行验证。仅更新本地产物，GitHub 正式发布未改。

函数 Out 定位修正源码：Out 标题可独立拖动、分组拉伸按相对位置跟随并约束框内，所有外部连线使用新端口。普通及测试编译检查通过，断言 / 实际交互未运行，未生成本轮 EXE；最新 EXE 仍为 100 点宽版，SHA-256 B04F62BB30B2260FBCED71D1FF1181BDB8541703827EFF128C617B2F8B159923。

## View / Move 与多选源码（2026-10-16）

新增 src/material_graph/tools.rs：画布左上角矢量手型 View（Q）/ 箭头十字 Move（W）浮动 Tab；View 左键平移、不选择节点，Move 点选 / Shift / 框选 / 多选标题拖动。两模式均 10%～175% 滚轮缩放。选择 / 位移集中应用，覆盖 Property、运算、接口、函数 In / Out、Render State、输出；标题点击仅选择，点击展开快照查看诊断。普通与测试编译检查通过，测试断言与实际 UI 未运行；无新 EXE，最新本地仍为 100 点宽版本 B04F62BB30B2260FBCED71D1FF1181BDB8541703827EFF128C617B2F8B159923。之前 Out 拖动 / 拉伸跟随源码也仍未打包。

## 函数边界与折叠源码（2026-10-16）

新增 interfaces.rs：具名边界输入与返回接口取代独立 In / Out 卡片，简单单路径 Return 合并；函数右上角折叠 / 展开保留布局与尺寸，折叠跳过内部快照。旧 Out 独立拖动约束已被边界方案替代。普通 / 测试编译检查通过，断言与 UI / GPU 未运行。无新 EXE，最新本地产物仍为 B04F62BB30B2260FBCED71D1FF1181BDB8541703827EFF128C617B2F8B159923。

## 函数边界与折叠本地构建（2026-10-16）

- 用户明确授权编译；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21607936 字节；SHA-256：3732A2ECBBD03B9C238EB696BDC08207BC4893BEC09ECA1BDA3EFCA4A5384716。
- 包含具名函数边界接口、单一简单 Return 合并、右上角折叠 / 展开，以及此前 View / Move 多选与紧凑运算节点修改。日志 target/function-boundary-release-build.log。
- 未启动程序，实际 UI / GPU 与交互待运行验证；仅更新本地产物，未发布 GitHub。

## 工具条显示修正（2026-10-16）

Q/W 工具条移除独立 Area，改为画布内末尾绘制并仅按画布可见范围裁剪。编译检查通过，实际缩放 / 面板伸缩及点击待验证，无新 EXE。

## Move 画布平移源码（2026-10-16）

Move 左拖空白平移，Shift 左拖空白保留框选；两模式右键可从节点上起拖平移。节点 / 分组 / 拉伸拖动限制为左键。编译检查通过，交互待验证，无新 EXE。

## 组多选源码（2026-10-16）

函数组加入 Tools 选择注册，组标题点选 / Shift，框选按组边界命中；混选统一位移并去重成员，非成员补偿保持原位。普通 / 测试编译检查通过，断言及实际交互未运行，无新 EXE。

## 画布工具与组多选本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21597696 字节；SHA-256：1F9359381E184E53ACEE7C4CBFB6680A2C9CB38CD28D4CFB389E8B3F36F4F4AE。
- 包含画布内固定工具条、Move 空白左拖平移、Shift 框选、跨节点右键平移，以及函数组与节点混合多选 / 成员移动去重。保留前轮函数边界接口与折叠。日志 target/group-multi-selection-release-build.log。
- 未启动程序，实际 UI / GPU 与交互待用户运行验证；仅更新本地 EXE，GitHub 发布未变。

## 紧急：画布平移死锁修复源码（2026-10-16）

pan_delta 在 Ui::input 内调用 dragged_by 重入 Context 锁，已将拖动判断与 clip_rect 提前读取。编译检查通过，未生成 EXE；最新 1F9359381E184E53ACEE7C4CBFB6680A2C9CB38CD28D4CFB389E8B3F36F4F4AE 仍含死锁，需用户授权且检查占用后重编译。不得自动关闭卡死进程。

## 画布平移死锁修复本地构建（2026-10-16）

- 用户明确授权；重新检查进程与目标 EXE 独占读写均通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21601280 字节；SHA-256：C5A8878B9F0C97382EB67B1DD12857FB2F720D40D283516FFB6F0223C597121A。
- 包含 pan_delta 避免 Context 输入锁重入的修复，保留工具条、画布平移及组多选功能；日志 target/canvas-pan-deadlock-release-build.log。此产物替代此前包含死锁的本地 EXE。
- 未启动程序，实际左 / 右键平移交互待运行验证；GitHub 发布未改。

## 折叠卡片拖动源码（2026-10-16）

折叠函数主体改为拖动面，保留右侧按钮 / 端口列；展开仅标题拖动。检查通过，实际交互待验证，无新 EXE。

## 折叠函数拖动修正本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21597696 字节；SHA-256：E6EEF8872325D6FCF0C0C4705020EBB76AA6465AF9C12A0515BFF9AC6C4430B5。
- 包含折叠函数主体拖动面修正，保留右侧按钮 / 端口列和组多选去重；包含此前画布平移死锁修复。日志 target/collapsed-function-drag-release-build.log。
- 未启动程序，实际折叠拖动与多选待运行验证；仅更新本地 EXE，GitHub 发布未改。

## 统一节点选择源码（2026-10-16）

移除旧 Preview.selected 橙框；Tools.node 改单一点击 / 拖动响应，非控件主体可移动，控件仍优先。编译检查通过，实际交互待验证，无新 EXE。

## 统一节点选择与拖动本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21566976 字节；SHA-256：82B180BA22B41407B90327336BA9B90BA2D4B5897E085D14CBC07D654A1172C8。
- 包含移除旧 Property 橙色单选框、统一蓝框与单一点击 / 拖动响应，节点非控件主体可拖动；保留折叠组拖动、多选及平移死锁修复。日志 target/unified-node-selection-release-build.log。
- 未启动程序，实际节点拖动、输入控件优先级及选择显示待运行验证；仅更新本地 EXE，GitHub 发布未改。

## 图标脚本（2026-10-16）

assets/generate_icons.py 已添加，Pillow 两次双线性缩放及预乘 Alpha，1024 优先 / 512 回退 / 缺失报错，13 档与 build.rs 同步。实际验证通过，测试输出在 target/icon-script-validation，原 assets PNG 未覆盖，无新 EXE。

## 统一鼠标交互源码（2026-10-16）

移除 Q/W 模式与工具条。左键选择 / 框选与节点拖动，右键画布平移，滚轮缩放，Shift 增减选择。按钮栏下新增七语言操作提示。普通 / 测试编译检查通过，实际交互待验证，无新 EXE。

## 统一鼠标交互本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21463552 字节；SHA-256：A9B6EF434792F75B0A8CF164C648FC902618A2783108F141406F75EE33CDE8F7。
- 移除 Q/W 模式与图标，包含左键选择 / 框选及节点拖动、右键画布平移、滚轮缩放和顶部七语言操作说明。保留组多选、折叠与统一蓝框；日志 target/mouse-graph-interaction-release-build.log。
- 未启动程序，实际拖动 / 框选 / 平移与提示布局待运行验证；仅更新本地 EXE，GitHub 发布未改。

## 连续拖动生命周期源码（2026-10-16）

Tools 增加 active_drag、画布作用域 ID、统一 finish 位移，按下 / 松开 / 失焦清理；成员更新兼容丢失释放事件。当前链路未发现嵌套输入锁，现场根因仍需运行核对。连续两次 egui 拖动序列测试仅编译；普通 / 测试检查通过，无新 EXE。

## 连续拖动状态修正本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21465088 字节；SHA-256：3C6A0799DF6995CAF25CB3FF09440B84722F9F11B61B96291D3BA5BC847F0124。
- 包含画布级拖动捕获、统一位移与按下 / 松开 / 失焦清理，保留统一鼠标操作和顶部说明。日志 target/repeated-graph-drag-release-build.log。
- 未启动程序，连续拖动问题是否解决待用户实际验证；新增测试仅编译，断言未执行。仅更新本地 EXE，GitHub 发布未改。

## 明确根因：输出离屏跳过交互提交（2026-10-16）

show 输出离屏时提前 return，原来跳过末尾 finish_tools，导致拖动位移及框选均失效；现离屏返回也先收尾。popup 开启时画布保留滚轮给菜单并暂停右拖。普通 / 测试检查通过，实际交互待验证，无新 EXE。

## 离屏交互与菜单滚轮修复本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21466112 字节；SHA-256：191293927279C7680F28E6157DA96D72A1042A32FF48606F260F0EB92D14C27C。
- 包含输出预览离屏分支补执行交互收尾，恢复该场景节点 / 组位移和空白框选；popup 开启时画布不消费滚轮并暂停平移。日志 target/graph-output-culling-interaction-release-build.log。
- 未启动程序，实际离屏拖动 / 框选和下拉菜单滚轮待用户验证；仅更新本地 EXE，GitHub 发布未改。

## 矢量按钮与 EXE 图标核对（2026-10-16）

函数 Minus / Square SVG，运算预览共享 Eye SVG；检查通过，无新 EXE。最新 191293... EXE 内两张 256 PNG 与当前 assets 256 PNG 像素相同，旧图标显示疑似 Windows 缓存，未修改源图 / 系统缓存。

## 矢量节点按钮本地构建（2026-10-16）

- 用户明确授权；进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21466112 字节；SHA-256：44C44BB2A92FF462BABD811523244E1538D93B1310C9363502FD24C012CE4B88。
- 包含函数框右上角 Minus / Square SVG 和复用 Hierarchy Eye 的运算预览按钮，保留离屏交互与下拉滚轮修复。日志 target/vector-graph-controls-release-build.log。
- 未启动程序，矢量按钮视觉与交互待运行验证；仅更新本地 EXE，GitHub 发布未改。

## RGBA 预览与带框按钮本地构建（2026-10-16）

- 用户明确授权；编译前进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（存在工程与依赖警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21487104 字节；SHA-256：9E12A4B747D4B701954AD9848C8B293DA688AE8799401B2E0EE8D2887C9D68A8。
- 包含直接绘制的折叠 / 展开图标与按钮边框、RGBA 单通道及相关运算预览恢复、带框眼睛预览按钮。日志 target/rgba-preview-framed-controls-release-build.log。
- 未启动程序；实际按钮显示 / 交互与颜色通道快照仍待运行验证，回归测试仅完成编译检查。仅更新本地 EXE，未发布。

## 眼睛按钮显示与点击修复本地构建（2026-10-16）

- 用户明确授权；编译前程序进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（存在工程及依赖警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21483520 字节；SHA-256：62AB8EE486B35FADD5314B705298EF172A34126B6CA19C1C030F0F0B4D382283。
- 包含直接绘制矢量眼睛图标、保留按钮边框与高亮、节点背景先于眼睛按钮注册交互，展开 / 收起请求重绘。日志 target/eye-button-interaction-release-build.log。
- 未启动程序；实际图标、点击展开 / 收起、拖动及 GPU 快照待运行验证，新增回归测试仅完成编译检查。仅更新本地 EXE，未发布。

## 节点预览持续更新本地构建（2026-10-16）

- 用户明确授权；编译前程序进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（存在工程及依赖警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21484032 字节；SHA-256：9BF1306002E1E1D67FD624BD876A279FE7F8CCBCC92249ACCB4C1AD6BE89930B。
- 包含展开且可见节点预览约每 100 毫秒按当前 Shader 时间更新、收起 / 离屏暂停、场景与贴图复用及每节点单一在途采集；保留此前眼睛按钮显示与点击修复。日志 target/live-node-preview-release-build.log。
- 未启动程序；实际动画、参数更新、离屏恢复与多节点性能待运行验证；新增回归测试仅完成编译检查。仅更新本地 EXE，未发布。

## 60 帧节点预览本地构建（2026-10-16）

- 用户明确授权；编译前程序进程与目标 EXE 独占读写检查通过，cargo build --release --locked --offline 成功（存在工程及依赖警告）。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.0，21484032 字节；SHA-256：BF57E1963B8FD9A611A3FCF7C8F6638D7B732D79467FC099B545942B84B04A1F。
- 包含节点预览采集间隔 16,666,667 纳秒、目标约 60 帧/秒，保留收起 / 离屏暂停、场景 / 贴图复用与每节点单一在途采集。日志 target/live-node-preview-60fps-release-build.log。
- 未启动程序；实际刷新率、动画与多节点性能仍待运行验证，不将构建成功视为稳定 60 帧验证。回归测试仅完成编译检查。仅更新本地 EXE，未发布。

## 图片九宫格严格 RGBA 判重与独立窗口（2026-10-16）

- 当前源码取消近似采样，逐像素比较完整 RGBA，仅精简连续相同整行 / 整列；透明像素隐藏 RGB 同样参与。自动识别最长连续重复区，差异方向退回原尺寸。使用共用 native_window 独立系统窗口。
- 保留同屏面积对照、单轴标记、同位置可读单 Sprite .meta 同步、无备份原位替换、软件记录目录临时文件清理和页签重载。
- cargo check --tests --locked --offline 及七语言静态核对通过；8 条九宫格回归测试已编译，严格判重版未执行。实际独立窗口、Unity 刷新与像素效果待验收。
- 当前 target/release/unity_yaml_viewer.exe 仍为早期采样版：22,100,992 字节，版本 1.2.1，SHA-256 43A573161BA907E1910D3910A7AB40FCE288EDC9C5E4F50AE6D9D485502CA3BA。其 Release 构建与 6 条测试通过；日志 target/nine-slice-release-build.log、target/nine-slice-tests.log。
- 用户已授权编译，但重新构建前发现目标 EXE 被占用。已依 AGENTS.md 停止，等待用户关闭占用并再次确认，然后重新检查占用，再执行严格判重版测试与 Release 构建。未发布。

## 图片精简最新源码交接（2026-10-16）

- 原位构建仍待用户关闭占用程序并再次确认；当前 EXE 仍为此前采样方案，不包含本节的新实现。禁止绕过原输出路径重新构建。
- 最新源码：简洁方向 tab、Stretch / Repeat；中心默认 4 px，透明边缘保留和 4 倍数对齐默认开启。Repeat 自动最小周期与横纵倍数，保留完整周期；不足一循环尾段进入固定边缘。支持透明裁边、必要时右 / 下补白，并显示真实尺寸、面积及实际倍数。
- 原图移到参数右侧且无标记，下方裁剪 / 结果对照。裁剪图半透明自动对比色遮罩（可退回红色），白框 / 虚线标 Repeat 保留循环，原 .meta Border 可选绿色叠加，原始 Border 与本次切分线不会混为一谈。
- 保存机制与约束沿用前述：无备份，软件记录目录暂存并清理，成功后刷新原页签，保留 .meta GUID，不修改 Scene / Prefab Image Type。Unity 分别需 Sliced / Tiled，实际网格须验收。
- cargo check --tests --locked --offline 通过；807 行七语言和关于折叠顺序静态核对通过；11 条相关回归测试仅编译、未执行，待关闭占用后明确确认再运行测试与 Release 构建。

## 最新九宫格编辑器构建完成（2026-10-16）

- 用户再次明确授权，测试及正式构建前分别重新检查程序进程 / EXE 占用通过，之前的等待状态已解除。
- cargo test --release --locked --offline nine_slice：11 / 11 通过；cargo build --release --locked --offline 成功，保留既有警告。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log。
- 当前产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.1，22221824 字节，SHA-256：652BC0704C129CFCCA363893528E166B9438CFD97AD566B60D9867200C23E83A；已包含前述全部最新图片编辑改动，取代采样版产物。
- 未启动主程序。独立窗口、三视图 / 遮罩 / 循环交互、七语言实际布局及 Unity 切换 / 导入效果仍待用户验收。未发布。
### 九宫格自动识别与布局稳定性调整（2026-10-16，源码）

- 方向、拉伸 / 重复使用主题切换同一套控件；自动识别同排比较六种组合，遵守透明边缘、中心尺寸、循环倍数与 4 倍数选项，按实际输出面积选择。纯单像素周期优先拉伸，避免无必要地使用 Tiled；不将近似 RGBA 判为相同。
- 分方向显示不能精简的具体原因、候选长度、实际保留量、受保护透明边缘及可用的 RGBA 差异示例；可展开六种方案比较。自动识别不再重置用户中心尺寸为 4。
- 修复数值拖动时旧预览被移除、输入宽度及说明高度变化引起的布局抖动：固定输入尺寸与顶部预览区域，参数区独立滚动，按下鼠标时保留参数区高度；异步更新期间继续显示上次结果，结果匹配当前参数前禁用导出与原位替换。
- 移除冗余的“还原至原尺寸预览”及对应预览纹理生成，右上方保留无标记原图。底部操作放大至至少 144 × 44，靠右排列；原位替换为绿色，另存为为蓝色，原位替换仍需确认。
- README 中英文、当前关于摘要及七语言文案同步维护；当前 / 上一版默认展示及旧版折叠逻辑未改。新增 3 条回归测试（九宫格累计 14 条），覆盖六组合选择、实际对齐面积排序、保留选项与失败原因。
- 验证：cargo check --tests --locked --offline 通过；本轮新增测试尚未执行，实际窗口拖动、七语言布局和 Unity 效果待运行验证。本段为源码状态，前一构建记录的 EXE 不包含本段新增调整。
- 最终静态核对：832 行翻译均为七语言、键唯一且非空；新增九宫格界面文案覆盖检查通过，git diff --check 通过。目标 Release EXE 独占打开失败（仍被占用）；占用检查脚本未将该异常设为终止错误，误启动了测试构建，发现后已立即中止，未继续 Release 构建，不计为测试通过。须关闭程序并再次明确确认后才能重新生成可执行程序；当前 EXE 尚不包含本轮源码调整。
### 边界输入排列调整（2026-10-16）

- 边界输入固定为两行两列：左列上 / 下，右列左 / 右，不再横排四项；保留方向禁用、数值范围与原有边界映射。
- px 作为独立单位放在输入框右侧，间距 2 px；输入框保持固定尺寸，避免数值位数变化影响排列。
- 同步 README 中英文、当前关于摘要及七语言翻译。实际窗口与七语言布局仍待运行验证；本轮仅进行 cargo check，不生成可执行程序。
### 精简收益高亮（2026-10-16）

- 结果宽高与节省面积 / 百分比使用加粗白字和底色突出显示；节省为绿色、面积增加为橙色、无变化为灰色，避免补白增加面积时误示节约。
- 同步 README 中英文、当前关于摘要与七语言文案。cargo check --tests --locked --offline 与 git diff --check 通过；实际浅 / 深色窗口显示待验证，未生成新 EXE。
### 最新交互调整本地编译验证（2026-10-16）

- 用户明确授权编译；测试编译前检查程序进程与既有 Release / 测试 EXE 独占读写，正式构建前再次检查进程与目标 EXE，均无占用。
- cargo test --release --locked --offline nine_slice：14 passed，0 failed，249 filtered out。覆盖六组合自动识别、实际对齐面积选择、限制诊断、逐像素 RGBA、循环重建、透明边缘 / 对齐、元信息保留与原位替换清理。
- cargo build --release --locked --offline 成功。target/release/unity_yaml_viewer.exe，版本 1.2.1，22349824 字节，SHA-256：10157094480B4B3C3E17FAC237C873C406AC416B8960CADECC1C4CEA9105D408。
- 此次 EXE 包含六方案自动识别、主题样式选项卡、拖动时保留上次预览与保存限制、移除还原预览、放大靠右的绿色替换 / 蓝色另存为按钮、两行边界输入与框外 2 px 间距单位，以及结果尺寸 / 面积变化底色高亮。
- 839 项七语言列数 / 非空 / 唯一键核对及 git diff --check 通过；保留既有编译警告。实际窗口交互、不同主题 / 语言布局和 Unity 导入仍待运行验证；未启动主程序，未发布。日志：target/nine-slice-tests.log、target/nine-slice-release-build.log。
### 底部信息与操作对齐（2026-10-17）

- 底部改为同一行的左右结构：左侧元信息与格式说明按可用宽度换行，右侧操作按钮相对信息区垂直居中，不再占据信息之后的独立一行。
- 操作区右侧预留 18 px，避免原生窗口滚动条覆盖按钮；保持按钮最小 144 × 44、颜色、保存条件与替换确认不变。
- 同步 README 中英文、当前关于摘要及七语言。cargo check --tests --locked --offline 通过，git diff --check 通过；实际窗口垂直居中、长路径换行与滚动条遮挡情况待运行验证。本轮未重新生成 EXE。
### 精简诊断默认收起（2026-10-17）

- 缩小状态旁新增“详情”按钮，实际保留中心、逐方向诊断与候选长度、RGBA 差异、六方案比较、裁剪 / 补白与循环详情默认隐藏，点击展开 / 收起；关闭后重新打开恢复收起。尺寸、面积收益与缩小状态仍直接展示。
- 同步 README 中英文、当前关于摘要及七语言翻译。cargo check --tests --locked --offline 与 git diff --check 通过；实际点击、重新打开与各语言布局待运行验证，未生成新 EXE。
### 精简范围说明澄清（2026-10-17）

- 详情由“候选区长度 / 实际保留”改为“可精简区 / 保留 / 移除”，例如 20 / 4 / 16 px；仅对通过精确重复判定的区域显示该组信息。未通过判定时只显示“检查范围”，避免把检查长度误称可删除区域。
- Repeat 统计仅包含完整周期，不完整尾段仍归固定边缘；透明外圈独立显示。处理算法、默认中心 4 px、用户可调尺寸及自动识别遵守用户设置的行为均不改变。
- 同步 README 中英文、当前关于摘要及七语言；cargo check --tests --locked --offline、git diff --check 通过，实际界面待验证，未生成新 EXE。
### 删除区斜纹标识（2026-10-17）

- 待删除区域保留半透明底色，并增加间隔 14 逻辑像素、宽 1.25 的稀疏斜线，沿用自动对比色 / 红色选项，斜纹 Alpha 230。
- 斜纹严格裁剪到各个不重叠删除矩形及界面裁剪范围；全图使用统一相位，交叉区域连续且不重复覆盖。仅影响裁剪预览，不修改源图、结果像素或保存算法。
- 同步 README 中英文、当前关于摘要及七语言图例；cargo check --tests --locked --offline 与 git diff --check 通过。实际斜纹观感与不同缩放下的清晰度待运行验证，未生成新 EXE。
### 精简窗口滚动条常显（2026-10-17）

- 九宫格独立窗口的外层滚动区使用实体滚动条：仅在内容溢出可滚动时显示，闲置不淡出，并占用独立宽度，提示下方仍有操作且避免覆盖内容。其他原生弹窗默认行为不变。
- 同步 README 中英文、当前关于摘要及七语言；cargo check --tests --locked --offline、git diff --check 通过。窗口缩放、长内容与底部按钮可见性待运行验证；未生成新 EXE。
### 顶部设置移出参数滚动区（2026-10-17）

- 左侧内部滚动区不再包含顶部方向 / 处理方式 / 自动识别及下一行复选框。边界、中心 / 循环倍数、统计与详情继续内部滚动；外层整窗滚动不变。
- 内部滚动区高度扣除固定设置高度，保持顶部参数 / 原图区域总高约 360。
- 同步 README 中英文、当前关于摘要与七语言；cargo check --tests --locked --offline、git diff --check 通过。实际滚动、换行与拖动待运行验证，未生成新 EXE。
### 右上区域改为精简信息（2026-10-17）

- 移除右上独立原图展示，把尺寸变化、面积收益、更新状态、缩小状态与默认收起的详情移至右上独立滚动区；左上仅保留设置与遮罩选色，下方仍同屏显示原图上的裁剪标记和精简结果。
- 两侧顶部区域保留约 360 高度，详情展开时在右侧内部滚动；顶部两行设置不随左侧内部参数滚动，避免详情变化影响参数位置。
- 同步 README 中英文、当前关于摘要及七语言。cargo check --tests --locked --offline 与 git diff --check 通过；实际两列布局、详情滚动和参数拖动待运行验证，未生成新 EXE。
### 无元信息时底部按钮居中（2026-10-17）

- 没有成功读取同位置 .meta 时不再保留空白元信息列，两个底部操作按钮整体在可用内容区居中；有可读 .meta 时保留左信息、右按钮及垂直居中布局。
- 不支持原位替换的格式说明在无元信息时独立置于按钮上方，按钮颜色、尺寸、启用条件与确认流程不变。
- 同步 README 中英文、当前关于摘要与七语言。cargo check --tests --locked --offline、git diff --check 通过；有 / 无 meta 的实际布局待运行验证，未生成新 EXE。
### 四区等宽与内容换行（2026-10-17）

- 设置 / 信息与裁剪 / 结果两行统一按相同可用宽度固定二等分；子内容不参与父区列宽分配，窗口缩放保持左右 1:1，两个分隔位置一致。
- 列内文字启用换行，选项组使用换行排列，缩小状态与详情按钮亦可换行；溢出绘制限制在本列范围内，不覆盖相邻区。
- 同步 README 中英文、当前关于摘要和七语言。cargo check --tests --locked --offline、git diff --check 通过；不同窗口宽度、七语言长文本与实际交互待运行验证，未生成新 EXE。
### 四区布局与遮罩调整本地构建（2026-10-17）

- 用户明确授权编译；测试前程序进程及既有 Release / 测试 EXE 独占检查通过，正式构建前再次检查进程与目标 EXE 无占用。
- cargo test --release --locked --offline nine_slice：14 passed，0 failed，249 filtered out；cargo build --release --locked --offline 成功，保留既有警告。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.1，22226432 字节，SHA-256：4A52E2BBD076DD79E85808D03B96EDF54F25D7318C781089B34E00A75E11B859。
- 本次 EXE 包含左右等宽四区与超宽换行、右上信息替代原图、详情默认收起、可精简 / 保留 / 移除文案、删除区斜纹、可滚动时常显实体滚动条、顶部两行设置移出参数滚动区，以及有 meta 时左右底部布局 / 无 meta 时按钮居中。
- 854 项七语言完整性与键唯一性、git diff --check 通过。未启动主程序；实际窗口缩放、文字换行、滚动与按钮对齐及 Unity 效果待运行验证。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；仅本地构建，未发布。
### 顶部按设置内容收紧及单位布局修复（2026-10-17）

- 移除左侧设置内部滚动区及 360 固定高度；左侧按实际设置内容排版，右上信息区上限使用当帧左侧高度，内容超出时独立滚动。信息展开不再推高整个顶部区域。
- Border 叠加开关移至左上设置末尾。边界输入由嵌套水平布局改为直接六列网格，标签和 px 不拆行，单位位于输入框右侧且保持 2 px 间距；其他说明文字继续在等宽列内换行。
- 同步 README 中英文、当前关于摘要与七语言。cargo check --tests --locked --offline、git diff --check 通过；实际单位布局、设置高度与信息滚动待运行验证，未生成新 EXE。
### 边界组标签居中（2026-10-17）

- 上 / 下、左 / 右四项继续作为同一两行网格；左侧“边界:”相对整个网格垂直居中，不再对齐第一行。单位间距、输入范围与方向禁用逻辑保持不变。
- 同步 README 中英文、当前关于摘要及七语言；cargo check --tests --locked --offline、git diff --check 通过。实际标签对齐待运行验证，未生成新 EXE。
### 顶部紧凑布局与边界对齐本地构建（2026-10-17）

- 用户明确授权编译；测试前检查程序进程与既有 Release / 测试 EXE 独占读写无占用，正式构建前再次检查进程与目标 EXE 无占用。
- cargo test --release --locked --offline nine_slice：14 passed，0 failed，249 filtered out；cargo build --release --locked --offline 成功，保留既有警告。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.1，22202880 字节，SHA-256：01CFE6F4951A41F1A631E87C9B4BE030641A9463DF7FA516E7850CC0E1174087。
- 此次 EXE 包含取消左上内部滚动条与固定高度、右上按设置内容等高并独立滚动、Border 叠加开关移入设置、px 不拆行及“边界:”相对两行输入垂直居中。
- 856 项七语言格式 / 非空 / 唯一键检查、git diff --check 通过；未启动主程序，实际单位对齐、紧凑高度及信息滚动效果待运行验证。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；仅本地构建，未发布。
### 预览伸展、边界间距与底部对齐（2026-10-17）

- 边界网格取消默认最小列宽，方向标签与输入框保持 2 px 列间距，不再为短标签留大块空白；“边界:”保持相对两行居中。
- 详情按钮改为“查看详情 / 收起详情”，移除无法显示的箭头字符。原图 / 结果像素尺寸移至各自标题旁。
- 裁剪与结果区域使用相同的剩余可视高度，随窗口增高；底部说明、meta 与操作区以实际测量高度预留，最小预览高度 180，空间不足时仍通过整窗滚动访问。左右列保持等宽。元信息列明确占满预留宽度，按钮靠右；无 meta 时仍居中。
- 同步 README 中英文、当前关于摘要与七语言。cargo check --tests --locked --offline、git diff --check 通过；窗口伸展 / 底部定位、不同语言长文本与确认区域布局仍待运行验证，未生成新 EXE。
### 预览伸展与底部布局本地构建（2026-10-17）

- 用户明确授权编译；测试前程序进程与既有 Release / 测试 EXE 独占读写检查通过，正式构建前再次确认进程与目标 EXE 无占用。
- cargo test --release --locked --offline nine_slice：14 passed，0 failed，249 filtered out；cargo build --release --locked --offline 成功，保留既有警告。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.1，22233600 字节，SHA-256：F11415498EE13E6134F656FF8C9576D044D6F57ACD6E0B8DEDACB072EAB29FE1。
- 包含边界标签紧贴输入框、查看 / 收起详情无特殊箭头、标题旁像素尺寸、预览随窗口增高及底部按实测高度预留、有 meta 时按钮靠右等调整。
- 859 项七语言格式 / 非空 / 唯一键与 git diff --check 通过；未启动主程序，实际窗口伸展、底部定位和多语言布局待运行验证。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；仅本地构建，未发布。
### 像素尺寸移至图片下方（2026-10-17）

- 裁剪与结果像素尺寸从标题旁移回各自图片下方，水平居中，使用半透明深色圆角底衬及白字；位置跟随实际图片矩形，不固定在整块预览区底部。
- 有下侧外置边界距离标签时，在其下方紧接尺寸标签；仅纵向模式没有外置标签时靠近图片下缘。预览适配预留更大的标注空间，避免遮盖图像或说明。
- 同步 README 中英文、当前关于摘要及七语言。cargo check --tests --locked --offline、git diff --check 通过；实际标签间距与不同缩放观感待运行验证，未生成新 EXE。
### 边界标签显式居中与组间距修正（2026-10-17）

- 根据截图确认普通水平布局仍让标签靠上，本轮改为明确预留 50 高度（两行各 24、行间距 2），在其中心绘制“边界:”，不依赖网格的布局对齐结果。
- 上 / 下的 px 与左 / 右方向文字之间增加独立间隔列，合计 12 px；输入框与 px 仍为 2 px。四项保持一个网格、单位不拆行。
- 同步 README 中英文、当前关于摘要及七语言。cargo check --tests --locked --offline、git diff --check 通过；实际对齐与不同 DPI 显示待运行验证，未生成新 EXE。
### 像素底衬与边界间距本地构建（2026-10-17）

- 用户明确授权编译；测试前程序进程及既有 Release / 测试 EXE 独占读写检查通过，正式构建前再次检查进程与目标 EXE 无占用。
- cargo test --release --locked --offline nine_slice：14 passed，0 failed，249 filtered out；cargo build --release --locked --offline 成功，保留既有警告。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.1，22233088 字节，SHA-256：E159855E43D124CE10ACB04DB52459374BC4D4485AB65D91A92F373533DB9EDC。
- 包含图片下方像素尺寸圆角底衬、尺寸紧接边界标记、边界标签按两行总高显式居中及 px 到下一方向组 12 px 间距。
- 861 项七语言格式 / 非空 / 唯一键及 git diff --check 通过；未启动主程序，实际标签对齐、尺寸底衬间距与不同 DPI 显示待运行验证。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；仅本地构建，未发布。
### 对齐补白分配与 Border 分离（2026-10-17）

- 4 倍数对齐需要补透明像素时，按轴将总量分为前侧 floor(n/2)、后侧 ceil(n/2)：1 加右 / 下，2 两侧各 1，3 左 / 上 1、右 / 下 2。原始内容整体平移，不重采样、不覆盖保留像素；优先多保留重复像素 / 完整周期的策略不变。
- 补白不再加到待写入 Border；元信息写入与结果边界标记使用原精简边界数值，因此原先为 0 的 B / T 等仍为 0。详情列出四侧补白，明确不计入 Border；源图裁剪线与删除区不受补白偏移影响。
- 更新重建映射与旧测试的内容偏移断言，新增 1 条回归测试覆盖 25 / 26 / 27 补至 28 的四侧透明像素、原像素定位、零 Border 保持和非零 Border 不增量。九宫格累计 15 条测试完成编译，本轮未执行。
- 同步 README 中英文、当前关于摘要和七语言；cargo check --tests --locked --offline、git diff --check 通过。实际透明补白、Unity 导入及 UI 留白待运行验证，未生成新 EXE。
### 普通焦点恢复不重新定位弹窗（2026-10-17）

- 共享原生弹窗不再因为失焦后重新获得焦点而开启定位流程，也不额外发出显示 / 恢复 / 聚焦命令；鼠标点击、窗口切换等普通激活保持用户原有位置。
- 首次打开、关闭后重开及程序通过 wake 明确唤醒仍按既有流程对齐主窗口。适用于共享原生弹窗，包括九宫格窗口。
- 同步 README 中英文、当前关于摘要及七语言。cargo check --tests --locked --offline、git diff --check 通过；鼠标激活、任务切换与按钮唤醒的实际窗口行为待运行验证，未生成新 EXE。
### 对齐补白与弹窗焦点修复本地构建（2026-10-17）

- 用户明确授权编译；测试前程序进程及既有 Release / 测试 EXE 独占读写检查通过，正式构建前再次检查进程与目标 EXE 无占用。
- cargo test --release --locked --offline nine_slice：15 passed，0 failed，249 filtered out；包含新增补白分配 / 原像素位置 / Border 不增量测试。cargo build --release --locked --offline 成功，保留既有警告。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.1，22219264 字节，SHA-256：411A1158413CF6BC82FAB43B372AC4347E9625881C59EF460E534C855D36AF12。
- 此次 EXE 包含透明补白两侧分配、奇数多补右 / 下、补白不计入 Unity Border，以及普通焦点恢复不再对齐主窗口的修复。
- 864 项七语言格式 / 非空 / 唯一键及 git diff --check 通过；未启动主程序，实际鼠标激活与按钮唤醒位置、Unity 导入及补白视觉效果待运行验证。日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；仅本地构建，未发布。
### slice9 命令行与顶部用法入口（2026-10-17，源码）

- 新增 slice9 无界面入口，在 GUI 单实例转发之前识别；原有直接传文件路径打开 / 转发已有窗口的方式不变。--help 与 slice9 --help 提供相同英文参数和示例，Windows 附着调用方控制台，保留重定向输出。
- 支持多个文件 / 目录、-r / --recursive、--dry-run、--center WxH、--repeat-count XxY、--direction、--mode、--no-preserve-edges、--no-align4、--report 与 -- 路径分隔。目录按层或递归枚举，跳过符号链接与不支持文件，规范化路径去重，逐张失败继续。
- 复用精确 RGBA 六方案、实际面积排序与原位写回；默认中心 4×4、透明边缘保留及 4 倍数对齐，只在输出面积减少时写入。缺失 / 不可读 meta 不创建；可读但不支持的 meta 返回该文件失败；支持 PNG / TGA / BMP / 静态 WebP，动画即使 dry-run 也报错。写前再次核验磁盘图片及已读 meta，GUID / 临时文件清理沿用现有流程。
- 标准输出为 JSON，包含逐文件尺寸、面积、方向 / 模式、四侧补白、L/B/R/T Border、meta 是否写入、诊断及分类汇总。退出码 0 无失败（含无收益 / 跳过），1 文件失败，2 参数 / 报告失败。报告仅创建新文件，开始修改资源前预先打开；最终报告写失败仍可能已有部分资源完成，批次不整体回滚。PowerShell 等待 GUI EXE 与取退出码的 Start-Process 示例见 README / 帮助。
- 主窗口“格式支持”右侧新增“指令查看”，预留按钮空间以免被拖入提示挤走；打开独立原生用法窗口，说明默认覆盖无备份，提供完整英文帮助及复制。文件菜单不添加重复入口。当前关于摘要及七语言窗口标题 / 按钮 / 提示、中英文 README 同步。
- 新增 2 条 CLI 回归测试覆盖默认参数与非法输入、目录递归 / 去重、dry-run 不写源图及指定横向的输出约束，九宫格累计 17 条测试仅完成编译检查。cargo check --tests --locked --offline 通过，869 项七语言列数 / 非空 / 唯一键、git diff --check 通过。关于默认两版 / 历史折叠逻辑未改。
- 本轮未获新增可执行编译授权，未执行新增测试、未生成新 EXE。命令行端到端、Windows 控制台等待 / 重定向 / 退出码、批量原位写回与实际窗口布局均待运行验证；现有 EXE 不包含 slice9。
### 打开示例与路径引号说明（2026-10-17）

- 共享指令说明明确列出单个打开、批量打开示例，补充简单路径可省略引号，含空格或 shell 特殊字符时各路径分别引用；指出不同 shell 转义规则不同，PowerShell 字面量含 $ / 反引号时优先单引号。
- 指令查看增加七语言引号提示，README 中英文和当前关于摘要同步。cargo check --tests --locked --offline、git diff --check 通过；未生成新 EXE，实际帮助显示待验证。
### slice9 首次本地构建与命令行验证（2026-10-17）

- 用户明确授权编译；测试及正式构建前分别检查程序进程与目标 / 测试 EXE 独占占用，无占用后执行。首次测试 16 通过、1 失败，查明受限环境 canonicalize 拒绝访问导致目录图片未收集，修复为规范化失败时保留绝对路径；再次 cargo test --release --locked --offline nine_slice：17 passed，0 failed，249 filtered out。
- cargo build --release --locked --offline 成功，保留既有警告；产物 target/release/unity_yaml_viewer.exe，版本 1.2.1，22331392 字节，SHA-256：60394B6FB2FB0D827947D10CFD4181CBA72E19BEB080A7BD88B5FAD013D2428F。包含 slice9、顶部指令查看、单个 / 批量打开示例及引号说明。
- 使用临时测试图片执行真实 EXE：含空格路径的 dry-run 保持源图哈希不变、目录递归 dry-run、JSON 标准输出重定向、slice9 --help 包含批量打开用法、非法参数退出码 2，均通过。原位写回端到端返回文件失败 / 退出码 1（拒绝访问 os error 5，当前受限环境文件权限），因此不记为写回验证通过；元信息 / 原位替换单元测试通过。未修改用户资源。
- 日志 target/nine-slice-tests.log、target/nine-slice-release-build.log；真实 CLI 测试输出 target/slice9-smoke-a1d46c894a194170b3a1382fcf7bc83c/。871 项七语言完整性 / 唯一键与 git diff --check 通过。
- 未启动 GUI；顶部入口、帮助窗口布局、原有 GUI 打开 / 单实例转发、非受限环境 CLI 原位写回及 Unity 导入仍待运行验证。仅本地构建，未发布。
### slice9 可编辑预览与十窗口上限（2026-10-17，源码）

- 新增 --preview，支持多个文件 / 目录与递归，路径去重后按规范化路径排序，每次最多取前 10 张；同进程同时最多 10 个九宫格窗口（含手动打开），重复请求唤醒原窗口且不重置用户设置，超出不排队并提示数量。关闭后可再次调用打开其他图片。
- 预览仅自动计算，不自动写图片 / meta；复用完整原有编辑器，用户可改参数、切换方向 / Stretch / Repeat、再识别、另存为、确认原位替换或关闭不改，原有格式 / meta / 计算状态限制不变。支持启动中心、循环倍数、透明边缘与对齐参数；不支持与 dry-run / report / 显式方向或处理方式同时启动，避免命令含义冲突。
- 命令行预览通过原有单实例通道传递带设置的请求；普通打开仍使用原逻辑，接收端兼容旧 JSON 路径数组。新建 GUI 与已有 GUI 均处理预览请求。调整图片资源缓存与每帧独立窗口刷新，切换主窗口页签不释放仍打开 / 正在保存的预览，防止多窗口消失或停止计算；保存后仍消费刷新消息重载。
- 命令帮助、指令查看共享文本、README 中英文和当前关于摘要及七语言同步。说明预览与无界面默认覆盖的区别、十张上限、超出处理、所有编辑功能保留及可能的内存占用。
- 新增 1 条预览回归测试覆盖不兼容参数、12 个资源重复输入只选择 10 个、通知、请求设置序列化往返以及源文件不变；累计 18 条九宫格测试完成编译检查，本轮未执行。cargo check --tests --locked --offline 通过；875 项七语言完整性 / 唯一键及 git diff --check 通过。
- 未生成新 EXE；实际多窗口、跨进程转发、十窗口上限、窗口关闭 / 再打开、保存 / 刷新与无修改关闭待运行验证，不能以编译检查替代。

# 1.2.2 发布准备（2026-10-17）


## 1.2.2 正式产物验证（2026-10-17）

- 用户授权发布含重新编译；测试编译前检查进程及 Release / 测试 EXE 独占读写，正式构建前再次检查，无占用后执行。
- cargo test --release --locked --offline nine_slice：18 passed，0 failed，249 filtered out；cargo build --release --locked --offline 成功，保留既有警告。
- EXE 文件版本与产品版本均为 1.2.2，Windows x64 单文件，22322688 字节，SHA-256：AAE927FC6DF60C20AD23BD41108685216E2AEBB92B5B988B3D587332C3F3A58D。
- 在非受限环境使用新建临时图片完成真实 EXE 测试：带空格路径 dry-run 不变、自动原位替换、meta 写回 Full Rect / 保留 GUID、帮助包含 preview / 批量打开示例、preview 与 dry-run 冲突返回退出码 2，均通过。仅处理 target 下测试资源，未修改用户图片。该验证补齐此前受限环境拒绝访问导致未完成的 CLI 写回验证。
- 879 项七语言列数 / 非空 / 唯一键、git diff --check、关于当前 / 上一版顺序与历史默认折叠 / 重开重置源码核对通过；设置末两项顺序未改。
- 日志 target/v1.2.2-tests.log、target/v1.2.2-build.log、target/v1.2.2-smoke-cd9d7111206649439fd4edf4246e3465/。GUI 多窗口 / 转发、关于历史实际点击和 Unity 运行仍待验证，未将测试或构建等同于完整验收。

# 1.2.2 已正式发布（2026-10-17）

- 用户要求的重新编译、测试、上传已完成；EXE 文件 / 产品版本均 1.2.2，18 条九宫格测试和真实 CLI 写回验证通过。
- GitHub Release：https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.2，latest=v1.2.2；附件 UnityYAMLViewer-v1.2.2-windows-x64.exe，22322688 字节，SHA-256 AAE927FC6DF60C20AD23BD41108685216E2AEBB92B5B988B3D587332C3F3A58D，GitHub asset digest 核验一致。
- 源码标签 v1.2.2 / commit 5069d9e92135188b505198cc350a4b864f2c3f7f 已推送 master；发布状态文档随后提交。GUI 多窗口、关于历史实际点击与 Unity 全面验收仍待验证，限制未隐去。

## 2026-10-17 音频剪切本地体验构建

- 用户明确授权编译以体验音频剪切；未升级版本、未发布。
- 构建前 Get-Process 未发现 UnityYAMLViewer 进程，target/release/unity_yaml_viewer.exe 独占读写通过；CIM 查询不可用，改用前述检查。
- cargo build --release --locked --offline 成功，21.81 秒，保留既有警告。日志 target/media-trim-build.log。
- 体验产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.2，22497280 字节，SHA-256 67E588737C0CDF8ED810416EB01A84CBC2776A27F4849A47848908553D3A405C。
- 音视频播放页顶部“剪切/格式转换”入口；音频绿色起止边界 / 双侧黑色遮罩、左右键标记、黄色游标定位，底部播放 / 剪切播放 / 停止 / 音量 / 时间轴；按格式提供 MP3 / M4A 码率或 WAV / FLAC 位深，选择路径与名称后导出。视频新布局按用户要求等待设计。
- cargo check、七语言列数 / 唯一键、真实 FFmpeg 参数验证通过；实际 GUI 体验与导出流程待用户验证。完整限制见 docs/versions/1.2.2.md 工作区新增部分。此本地 EXE 与旧 GitHub v1.2.2 发布产物不同，未改动旧发布附件。


## 2026-10-17 剪切原生独立窗口修复

- 独立原生窗口修复后的体验构建：沿用用户已明确授权的本地编译范围，构建前重新检查 Get-Process 无程序进程、Release EXE 独占读写通过；cargo build --release --locked --offline 成功（18.39 秒），保留既有警告。产物 target/release/unity_yaml_viewer.exe，版本 1.2.2，22596096 字节，SHA-256 098796B4C9C9FF764094932A2584884443574C19C244B474F634F47561618903；日志 target/media-trim-build.log 已更新。仅本地构建，原生窗口实际运行体验待验证，未发布。
- 播放页入口和剪切内容不变，宿主由 egui 浮层改用 native_window::Window；源文件路径作为窗口 ID，入口再次点击唤醒。视频只更换宿主，新布局继续等待用户设计。

## 2026-10-17 最新音频体验调整待重新构建

- 最新音频体验调整的构建未成功：cargo check 通过；Release EXE 独占检查提示文件被占用，但临时 PowerShell 检查未设置终止错误，错误后仍误启动 cargo build --release --locked --offline，最终因 os error 5 失败。未生成可交付的新版本。已停止构建重试，遵循 AGENTS.md 等待用户关闭占用程序并再次明确授权；下次使用终止式检查，任何占用检查失败均不得启动构建。最新 UI 修改仅源码 / check 完成，现有 EXE 不代表此次修改。

## 2026-10-17 最新音频体验修改编译完成

- 用户确认已关闭并再次授权后，重新检查 Get-Process 无相关进程、目标 EXE 独占读写通过；cargo build --release --locked --offline 成功（18.55 秒），保留既有警告。产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.2，22621184 字节，SHA-256 1148CA26587C7923310B5CB45C5C49AF2C93374D6EBE6E2260E67D49ECAA2FC0。日志 target/media-trim-build.log。本次包含简写导出、同尺寸实心绿色箭头及细线、原弹出音量、居中进度时间、范围工具条单行常显、底部固定控制与自适应波形、音频标题本地化及去除内容区文件名 / 大小。此前构建占用阻塞已解除；实际 GUI 操作与缩放仍待体验验证，未发布。

## 2026-10-17 独立剪切播放与统一图标体验构建

- 本轮用户明确授权后，重新检查无相关程序进程、目标 Release EXE 独占读写通过；cargo build --release --locked --offline 成功（18.62 秒），保留既有警告。最新产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.2，22528000 字节，SHA-256 27AA2A14F474AB3E2BCB2A48D9D517953702ECCE6FA20B2A4EE874A5277B685C。日志 target/media-trim-build.log。包含独立播放 / 音量 / 游标、全局多文件剪切窗及切换或关闭来源页签后保留、打开时停止来源播放、右上绿色大导出按钮、第三排范围操作、主播放页同尺寸的播放 / 暂停与停止图标、Next 同款实心剪切播放图标，以及底部去分隔线保留间距。实际声音、生命周期、多窗口并播和视觉布局仍待 GUI 验证，未发布。

## 2026-10-17 导出按钮居中与边界拖动显示修复构建

- 本轮用户明确授权后，检查无相关程序进程、目标 EXE 独占读写通过；cargo build --release --locked --offline 成功（17.99 秒），保留既有警告。最新产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.2，22532608 字节，SHA-256 F47B45DFB1689317CEC55D925BD16CBB205D4D1EE1A59FCFC3DFFE27AC50E41C。日志 target/media-trim-build.log。本次包含导出按钮高度 40 / 文字水平垂直居中，以及绿色边界悬停和拖动时持续显示时间刻度、网格、黄色播放竖线与箭头。实际布局和拖动显示待 GUI 验证，未发布。

## 2026-10-17 另存为按钮体验构建

- 用户明确授权后，检查无相关程序进程、目标 Release EXE 独占读写通过；cargo build --release --locked --offline 成功（18.65 秒），保留既有警告。最新产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.2，22534144 字节，SHA-256 A8E5738FB19A9139968D27D50545A7CEB1514951B8C3E6876DCE3406F447700A。日志 target/media-trim-build.log。本次包含右上绿色按钮改名“另存为”、本地化文字居中及宽度随译文适配；保存流程不变。实际界面显示待 GUI 验证，未发布。

## 2026-10-17 升级 1.2.3 与功能归属迁移

- Cargo.toml / Cargo.lock 当前版本为 1.2.3，未发布，尚未构建 1.2.3 EXE。
- 当前音频剪切 / 转换功能、独立播放与多窗口、相关布局和限制集中记录在 docs/versions/1.2.3.md。原 1.2.2 工作区新增记录和构建阻塞过程迁至 docs/versions/archive/1.2.3-development.md，保留当时体验 EXE 真实版本与哈希；docs/versions/1.2.2.md 只保留正式版原有功能与发布事实。
- 关于默认显示 1.2.3 / 1.2.2，1.2.1 移入默认收起历史，旧摘要及七语言保留；当前音频摘要七语言完整。版本索引和 CHANGELOG 已加入 1.2.3；README 不新增版本日志。
- cargo check --locked --offline 与 cargo check --tests --locked --offline 通过，915 项翻译结构 / 唯一键和关于窗口顺序及折叠重置源码检查通过，未运行 GUI 或测试 EXE。日志 target/v1.2.3-check.log、target/v1.2.3-check-tests.log。
- 现有 target/release/unity_yaml_viewer.exe 仍为升级前 1.2.2 体验构建（22534144 字节，SHA-256 A8E5738FB19A9139968D27D50545A7CEB1514951B8C3E6876DCE3406F447700A），不得标注为 1.2.3 产物。正式发布状态未改变。

## 2026-10-17 1.2.3 音频编辑能力实现

- 新增 src/audio_edit.rs：临时目录浮点 PCM 不可变版本、后台头尾 / 区间剪切、片段并集拼接、三条淡入淡出曲线与整体 dB、按窗口 Undo / Redo。历史沿用设置中的缓存撤销步数，编辑成功提交、失败恢复，拖动合并一步，新编辑清空重做链。
- 头尾 / 区间复用主题式分段 Tab；区间反转左右键设置目标，允许交叉，按并集保留两端，重叠不重复音频。剪切后刷新实际音频 / 波形 / 时长。淡入 / 淡出与音量(dB)各自原生设置窗口，设置时预览包络，确认后生成新缓存版本，取消恢复。黑色遮罩 30% 不透明。
- 保存当前已提交完整时间轴，未点击剪切仅为范围预览；另存为读取临时音频而非重新读取原文件，保留原文件、最终编码一次。缓存由当前状态 / 历史 / 工作线程 / 导出引用共同持有，关闭尽量清理，Windows 短暂句柄占用重试，崩溃仍可能残留。
- cargo check / cargo check --tests 仅编译检查通过；新增 5 项编辑状态、包络、曲线、区间并集与 FFmpeg 缓存历史测试源码，尚未运行测试 EXE。本轮未构建应用 EXE。
- 实际 FFmpeg 参数验证：4 秒音频删除中间 2 秒，缓存 2.000000 秒；+6.0206 dB 与 1 秒淡入 / 淡出抽样振幅 0、0.25、0.5、0.25，符合预期。曾发现 aeval 声道布局导致 concat 失败，已补 c=same 后重测通过。测试资源 target/audio-edit-smoke-e5604f5b38e7467cb164effdf1f8b272/。
- 待验证实际 GUI 编辑、应用 / 取消、Ctrl+Z / Ctrl+Y、缓存释放、多声道与最终另存为流程。完整限制见 docs/versions/1.2.3.md；七语言及 README 中英同步，关于默认当前 / 上一版和设置末两项顺序保持。

## 2026-10-17 1.2.3 音频编辑 EXE 与针对性测试完成

- 用户明确授权后，构建前检查无相关程序进程、目标 Release EXE 独占读写通过；cargo build --release --locked --offline 成功（21.36 秒），保留既有警告。产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22672384 字节，SHA-256 55CD375A4899184CAF0F35D2FA569DDDDE49DA0C7BACD23973A17EF48BC0E51A。日志 target/v1.2.3-audio-edit-build.log。
- 测试编译前再次检查程序进程及 Release / 测试 EXE 独占访问，通过后 cargo test --release --locked --offline audio_edit::tests -- --nocapture：5 passed，0 failed，269 filtered out，执行 0.72 秒；日志 target/v1.2.3-audio-edit-tests.log。另使用已生成测试 EXE 以 Start-Process -Wait -PassThru 执行两个独立会话测试，各退出码 0；日志 target/v1.2.3-lifecycle-1.log、target/v1.2.3-lifecycle-2.log。
- 仅本地体验构建与针对性测试，不标记正式发布；设置弹窗实际布局、鼠标操作、快捷键焦点、声音试听及另存为整条 GUI 流程待体验确认。

## 2026-10-17 1.2.3 选区 dB 与统一左右键构建

- 本轮按用户明确授权，构建前检查无相关程序进程、Release / 测试 EXE 独占访问通过；cargo build --release --locked --offline 成功（19.73 秒），再次占用检查后 cargo test --release --locked --offline audio_edit::tests -- --nocapture：6 passed，0 failed，269 filtered out，执行 0.90 秒。保留既有警告。
- 最新产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22769664 字节，SHA-256 23A47740274074ABE70D64E0DB1AA365D0F74BBF0EE4DF5AA9CA7412ADFC6B37；日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-audio-edit-tests.log。包含左右键统一与反向选区归一化、选区 dB、未选区间时作用于当前剪切阶段整段、弹窗作用范围提示。真实缓存测试验证区间降增益不改变区间外样本，并且 Undo / Redo 恢复对应临时版本、时长仍为剪切后的 2 秒、源文件不变。
- 本地构建与针对性测试完成；实际 GUI 选区操作、设置弹窗、试听和另存为完整流程仍待验证，未发布。

## 参数窗与时长显示构建（2026-10-17）

- 用户明确授权后检查进程及目标 EXE 独占访问，无占用，cargo build --release --locked --offline 成功（20.03 秒，4 项警告）。包含淡入淡出 / 音量参数窗布局、默认选区时长、居中按钮及模式 Tab 后的区间 / 剪切后时长。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22701056 字节，SHA-256 9CA479EC555281DA94AFD45D46F65BEB60EAB44BF145980F1C87A8B1082C4B4C。日志 target/v1.2.3-audio-edit-build.log。未运行 GUI 验证，未发布。

## 音效参数窗置顶构建（2026-10-17）

- 用户明确授权后检查目标进程和 EXE 独占访问，无占用，cargo build --release --locked --offline 成功。包含淡入、淡出、音量(dB)参数窗 AlwaysOnTop 层级。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22660096 字节，SHA-256 E12E918E14A1B492A82A559CCFB061011DA93B5AD95E0E99065DE4BF04BDA93F。构建日志 target/v1.2.3-audio-edit-build.log。实际 Windows 置顶与多窗口切换待 GUI 验证，未发布。

## 时间轴缩放与编辑命名构建（2026-10-17）

- 用户明确授权后检查目标进程和 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.95 秒，4 项警告）。包含主窗口音频 / 音频编辑的滚轮缩放、水平浏览滑条，以及音频编辑 / 视频编辑入口和标题。
- 重新检查占用后执行 cargo test --release --locked --offline waveform_zoom_preserves_anchor_and_limits_visible_range -- --nocapture：1 项通过，0 失败，275 项过滤。验证鼠标时间锚点保持、最大缩放边界与缩回全长；不能代替 GUI 交互验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22809088 字节，SHA-256 329626F75EB17D8E097A221ED6A28EFBCCBBDEE28D2E15000D748853E99CAA64。日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-wave-zoom-tests.log。实际窗口缩放、滑条、拖动、多语言及长音频性能待 GUI 验证，未发布。
## 曲线预设与波形填充构建（2026-10-17）

- 用户明确授权后检查进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（18.55 秒，4 项警告）。包含七种淡入淡出下拉预设、共享波形填充网格及物理像素对齐修复。
- 再次检查占用后编译并运行 all_fade_presets 测试，1 项通过；随后直接运行同一最新测试程序的 waveform_columns_cover_fractional_dpi_without_gaps，1 项通过。覆盖七条曲线端点 / 有界 / 单调性及四种 DPI 列边缘覆盖。新增曲线的实际 FFmpeg 执行、GUI 预设选择与用户音频缝隙仍待验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22784512 字节，SHA-256 7806B15F7DCBDF2C2E8A59CEB14311072E85067EA9CFAE860DE1BA4B96D706F8。日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-fade-presets-tests.log、target/v1.2.3-wave-gaps-tests.log。未发布。
## 自定义曲线与停止归零构建（2026-10-17）

- 用户明确授权后检查进程及目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（18.74 秒，5 项警告）。包含可编辑淡入淡出曲线、预设 / 自定义切换及音频编辑停止归零。
- 再次检查占用后 cargo test --release --locked --offline custom_curve_interpolates_and_survives_history -- --nocapture：1 项通过；直接运行最新测试程序的 stop_rewinds_and_clears_pending_trim_playback：1 项通过。分别覆盖自定义插值与 Undo / Redo 点集恢复，以及停止归零 / 清除片段续播状态。实际 FFmpeg 自定义曲线执行与 GUI 交互仍待验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22789632 字节，SHA-256 60B3835778D3C73C79D066C9F2D7FE9044F3B1B1D2807AB779419E3F262BC9AB。日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-custom-curve-tests.log、target/v1.2.3-stop-rewind-tests.log。未发布。
## 贝塞尔手柄构建与 FFmpeg 兼容修复

- 用户明确授权后检查进程及 EXE 独占访问，无占用并构建 Release。首次音频测试发现当前 FFmpeg 已移除 filter_complex_script 旧参数，改为优先使用 -/filter_complex 文件参数，不识别时回退旧参数；修正后重新检查占用并构建成功。
- cargo test --release --locked --offline audio_edit::tests -- --nocapture：9 项通过，0 失败，包含曲率手柄、历史恢复、预设及真实 FFmpeg 缓存剪切 / 效果。GUI 手柄交互与完整另存为流程仍待验证，未发布。
- 最终产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22744576 字节，SHA-256 FFEE892CE2DC6922EAAAF715A78DDB598906EA22EF143371BBC26C236F1CA8FE。日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-bezier-tests.log。

## 播放居中跟随与端点幅度构建（2026-10-18）

- 用户明确授权后检查程序进程及目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.61 秒，6 项警告）。包含淡入起点 / 淡出终点上下拖动、主窗口及音频编辑播放时缩放后 0.1 秒居中跟随；全长未放大时不跟随，两端限制在实际时间轴范围。
- 重新检查占用后编译运行 waveform_follow_centers_playhead_and_clamps_at_both_ends：1 项通过；直接运行最新测试程序 audio_edit::tests：9 项通过、0 失败。覆盖跟随中心 / 两端计算，以及曲线、历史与真实 FFmpeg 缓存处理。0.1 秒交互时机、实际端点拖动、播放 / 暂停及手动浏览仍待 GUI 验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22820864 字节，SHA-256 BB3F7906E6C47E8A3008F85A79BA1B7D194DC3AED4E4EDFE0ECCFCDE0A461B05。日志 target/v1.2.3-audio-edit-build.log、target/v1.2.3-wave-follow-tests.log、target/v1.2.3-bezier-tests.log。未发布。
## 页签分组与整组拖拽构建（2026-10-18）

- 用户明确授权后检查程序进程及目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.13 秒，6 项警告）。包含同后缀超过 3 个自动折叠、统一单成员 / 多成员页签组、整组拖拽、逐项关闭至 3 个原位展开，以及关闭全部期间保持成组与未保存提醒。
- 再次检查程序及测试 EXE 占用后执行 cargo test --release --locked --offline tab_reorder_tests -- --nocapture：3 项通过，0 失败。覆盖整组 / 单成员移动、首尾与跨行插入、成员原位归拢与单成员显示判断；实际 GUI 拖拽、菜单、批量关闭与保存 / 取消、多语言仍待验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22823936 字节，SHA-256 F886E9F4957A2836B6AB0B4F019BBDA319A289089E17B3AD6D984201896228DE。日志 target/tab-group-release-build.log、target/tab-group-tests.log。未发布。

## 页签菜单样式与矢量图标构建（2026-10-18）

- 用户明确授权后检查程序进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（18.59 秒，6 项警告）。包含页签 SVG 下拉 / 关闭图标，以及参照最近打开的自适应宽度、左对齐单行截断、路径提示和无边框分组菜单。
- 本轮仅样式调整，cargo check --tests --locked --offline 已通过，未重复运行排序测试。实际菜单、深浅主题、DPI 与拖拽前景图标仍待 GUI 验证，未发布。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22854144 字节，SHA-256 7A916C67781E233BF58CA2F8634C4F67BFD92F791DE3BE75DC218C25DACC7FC3。构建日志 target/tab-menu-style-release-build.log。

## 文件堆叠数量与页签高度修复构建（2026-10-18）

- 用户明确授权后检查程序进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.95 秒，6 项警告）。包含文件堆叠 SVG 图标、组内数量与多位数字宽度适配，以及选中 / 悬停图标绘制不再推进父布局的高度修复。
- 再次检查程序与测试 EXE 占用后执行 cargo test --release --locked --offline tab_reorder_tests -- --nocapture：4 项通过，0 失败。覆盖整组排序、跨行插入、原位归拢，以及三个选中状态和四个鼠标位置下父布局矩形不变（含两位数数量）。实际 GUI、DPI、文件关闭后的数量更新和拖拽动画仍待验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22856192 字节，SHA-256 B7A271C2BF180899E5F7111796385CD9387A46062C0B4DEADA2ABAF4FE4C6D20。日志 target/tab-stack-release-build.log、target/tab-stack-tests.log。未发布。

## 文件堆叠效果反馈与提交（2026-10-18）

- 用户实际查看最新构建后反馈文件堆叠效果满意，并授权 Git 提交和推送，明确不发布版本。该反馈确认整体显示效果，不等同于全部 GUI 边界场景、七语言或 DPI 验证完成。
- 本次提交包含此前音频编辑完善、文件堆叠数量、整组拖拽与页签栏高度修复；版本仍为 1.2.3 开发中 / 未发布，不创建发布标签或 Release。

## SVG 与 GIF 循环预览构建（2026-10-18）

- 用户明确授权后检查程序进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（21.16 秒，6 项警告）。包含 SVG 只读缩放预览与位图操作禁用，以及 FFmpeg GIF 播放、四种循环模式、帧数 / 每帧秒数和按实际时长分布的帧边界时间轴。
- 检查程序及测试 EXE 占用后编译运行 GIF 专用测试：2 项通过；再通过 Cargo 运行 SVG 渲染测试 1 项、SVG 文件类型识别 1 项、GIF 媒体扩展识别 1 项，合计 5 项通过，0 失败。覆盖循环扩展 / 截断、有限 / 无限 / 自定义计数、可变帧时长与末帧停留计算、viewBox、透明像素、无效 SVG 及尺寸上限。未用测试代替实际 GIF 解码播放 / 循环与 GUI 验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，23014400 字节，SHA-256 F50D3D32F3949AF2FFB51E105ED353E23920D8EF4C986FD6A1C678E9CF8A5927。构建日志 target/svg-gif-release-build.log，测试日志 target/svg-gif-tests.log、target/svg-render-tests.log、target/reference_types_and_syntaxes_include_common_scripts.log、target/media_extensions_are_case_insensitive_and_do_not_capture_yaml.log。实际界面与真实媒体播放仍待验证，未发布。

## GIF 音频界面与调色板布局构建（2026-10-18）

- 用户明确授权后检查程序进程及目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.39 秒，6 项警告）。包含 GIF 隐藏音量 / 音轨相关界面、在原音频信息行显示全局 / 局部调色板最大容量；普通视频保持原样。
- 再次检查程序及测试 EXE 占用后执行 cargo test --release --locked --offline gif_playback::tests -- --nocapture：3 项通过，0 失败，覆盖色表容量、循环扩展与策略、可变帧时长 / 末帧时长计算。实际界面位置、音频界面隐藏与真实 GIF 播放仍待 GUI 验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，23042560 字节，SHA-256 E3B63363273300041214271C86A4152F41C00FBFCB6D6FACA5183A88E1F0EE8F。日志 target/gif-palette-release-build.log、target/gif-palette-tests.log。未发布。

## SVG 背景设置构建（2026-10-18）

- 用户明确授权后检查程序进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功（19.69 秒，6 项警告）。包含移除 SVG 通道显示、隐藏九宫格入口，以及复用 Prefab 预览的棋盘格 / 灰色 / 自定义背景与重载保留背景设置。
- 再次检查程序及测试 EXE 占用后运行 cargo test --release --locked --offline asset_viewer::svg_preview::tests -- --nocapture：1 项通过，0 失败，覆盖 SVG viewBox、透明像素、无效输入及渲染尺寸上限。背景切换、自定义颜色与重载实际 GUI 仍待验证。
- 本地产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.3，22957568 字节，SHA-256 576E149087C6C5BAF94E22CD924BB9628C793C7202E164F0B4E84686B75EFDE4。日志 target/svg-background-release-build.log、target/svg-background-tests.log。未发布。

## Release 显示标题统一（2026-10-18）

- 按用户要求，仅修改 GitHub Release name 为对应 v版本号：v1.2.3～v1.1.8，以及 v1.1.4～v1.1.0。用户已修改的 v1.1.5～v1.1.7 保留。
- API 回读核对标题；标签名 / 目标提交、正文、附件及草稿 / 预发布状态保持不变，latest API 仍是 v1.2.3。未重新编译或上传附件。
- 标题规则已写入根 AGENTS.md，后续发布只用 v1.x.x 作为显示标题。该规则和发布后状态文档均需用户自行提交 / 推送。
## 1.2.4 视频编辑本地体验构建（2026-10-18）
- 编译前检查：未发现 unity_yaml_viewer 进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用产生 7 条警告；日志：target/video-editor-release-build.log。
- 产物：target/release/unity_yaml_viewer.exe，版本 1.2.4，23310848 字节，SHA-256：DFB00D657DD6A7EF6EEE4BD1DDB87BEAE6205EEB098611323EF4A9C7083121D1。
- 尚未运行应用验证新视频编辑界面与端到端导出；本地构建不代表正式发布。

## 视频裁剪与编辑时间轴体验构建（2026-10-18）

- 用户明确授权编译；编译前未发现程序进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用 7 条警告；日志 target/video-timeline-release-build.log。
- 本次 EXE 包含视频裁剪叠加、九种容器、自定义后缀、独立时间 / 帧刻度编辑轴、顶部范围操作及右上角另存为。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23319040 字节；SHA-256：60AEE22587CD657F209F2779E35A20DD25B37B058DCF6C856D0CC1F5BC66E065。
- 未运行应用验证实际 GUI 和端到端导出，未发布。前述未重新构建的记录为当时状态，本次构建已包含这些修改。

## 紧凑时间轴与标题对齐体验构建（2026-10-18）

- 用户明确授权；编译前未发现运行进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/video-layout-release-build.log。
- 已包含 88 逻辑像素紧凑编辑轴（上方帧刻度、中间选区、底部时间）及导出设置标题底部对齐。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.4，23318528 字节；SHA-256：59BB3246C47539AA51F7BEB007D55157E2A7C8C18E60F590FFD11DAAEEFB0D0A。
- 实际 GUI 与导出仍待运行验证，未正式发布。此前未重新构建说明为对应修改当时状态。

## 多轨、字幕与缩放体验构建（2026-10-18）

- 首次编译检查发现目标 EXE 被占用，未启动构建；用户再次明确确认后重新检查，未发现运行进程且目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/multitrack-subtitle-release-build.log。
- 本次包含新增视频 / 音频编码、GIF 颜色与抖动、宽高锁链缩放、默认保留与增删多音轨 / 多字幕、语言设置、播放器文字字幕样式与音轨语言跟随等当前修改。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23634432 字节；SHA-256：90537DBDF44937E46F7FF7BC0652D9B2BC31261235DD65495EF90F42CCD0221F。
- 尚未运行应用进行 GUI 与端到端验收；新增测试仅做编译检查。未正式发布，前文未重新构建说明为各阶段当时状态。

## 导出参数与画面大小布局体验构建（2026-10-18）

- 用户明确授权；重新检查未发现运行进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/video-export-settings-release-build.log。
- 本次包含时间轴绿色全高边界 / 黄色倒三角、右侧设置延伸到底部、视频音频采样率选择、目标码率 / 通用帧率 / 兼容编码两遍处理，以及画面大小上下宽高、左侧锁链、右侧现有 Tab 与加宽输入框。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23651328 字节；SHA-256：3E6CDEBEDCC50211540E73EF68ADBB8FD1A2809A732E290E268B7C0FA87D490F。
- 实际 GUI 与端到端导出待验收；新增测试仅编译未运行。未发布，前文未重新构建说明为对应阶段状态。

## 时间轴与关联按钮完善体验构建（2026-10-18）

- 用户明确授权；编译前未发现运行进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/video-timeline-polish-release-build.log。
- 包含 Tab 右侧避让与非浮动滚动条、PS 参考关联按钮、未锁定无底灰色图标、黄色游标连续拖动 / 松开寻址及独立刷新、每帧平均约 12 像素的放大限制、剔除区蓝灰色显示。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23659008 字节；SHA-256：3E18516F8B71518EBE824682C773E1CCC4C2C2C6877ACC9D394FD318FD4ACADF。
- 实际 GUI 平滑度与布局仍待体验验证，未正式发布；此前未重新构建说明为对应修改阶段状态。

## 侧栏宽度与关联图标样式体验构建（2026-10-18）

- 用户明确授权；编译前未发现运行进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/video-panel-link-release-build.log。
- 包含画面大小输入固定宽度、侧栏宽度反馈修复与按文件独立状态、宽高标签对齐，以及关联图标无底框 / 锁定主题蓝 / 未锁定灰色。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23691776 字节；SHA-256：D43DA798C86C3C93096BDFAD2693EE2B34ECADA1F82A4101BB98CDBCF651CF41。
- 实际 GUI 布局与交互待体验验证，未正式发布；前述未重新构建为对应阶段状态。

## 预览同步与导出参数排版体验构建（2026-10-18）

- 用户明确授权；编译前未发现运行进程，目标 EXE 独占打开检查通过。
- cargo build --release --offline --locked 成功，应用保留 7 条警告；日志 target/video-preview-layout-release-build.log。
- 包含关联括线端点对齐文字中心、画面大小预览同步、质量 / 目标码率 Tab 样式，以及视频封装格式 & 编码格式同行和七语言标题。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.2.4，23693824 字节；SHA-256：A4D538F3C354B89810296506D4B6C3B65C58F77A3556FCDCAB37F2DA9D9AAE45。
- 实际 GUI 与端到端导出待体验验收，未正式发布；前文未重新构建说明为对应阶段状态。

## 最新播放列表与时间轴 Release 构建

- 用户明确授权后，已检查目标程序未运行、EXE 可独占打开；cargo build --release --offline --locked 成功（7 条应用警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.4；大小 23694336 字节；SHA256：70B57F6243CC2020C7E7AC99E231B9F6261BDF6D257ECC22251301052A28C45F。
- 包含导出完成 / 失败弹窗、播放列表菜单按编辑状态切换（普通与全屏共用）、保存成功退出编辑，以及视频时间轴橙色游标与浅色移除区域配色。实际 GUI 与端到端导出仍待验证，未正式发布。
- 构建日志：target/playlist-timeline-release-build.log。

## CSV / TSV / PSV Release 构建验证

- 用户明确授权后检查程序未运行且目标 EXE 无占用，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4，产物 target/release/unity_yaml_viewer.exe，大小 23754240 字节，SHA256：203AD2F6513D2BD638B705AAFC93B789CDFE8DF12FE76C3B5147342116CB582B。构建日志 target/delimited-release-build.log。
- 包含 CSV / TSV / PSV 表格预览、原文切换、列宽调整及七语言文案；实际 GUI 与解析测试执行仍待验证。未正式发布。

## 表格编辑 Release 构建验证

- 用户明确授权后检查程序未运行且目标 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23650304 字节；SHA256：CBAA1FF92A2E53F585FE43065EA2F71D1F7279455B546F10B5B4D3024DCF8B72。日志 target/table-edit-release-build.log。
- 包含 CSV / TSV / PSV 表格固定边缘非浮动滑条、单元格编辑与修改高亮、Ctrl+Z 撤销、保存编辑及放弃编辑。GUI 交互与解析 / 撤销测试执行仍待验证，未正式发布。

## 表格类型检查 Release 构建验证

- 用户明确授权后确认程序未运行、目标 EXE 无占用，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23654912 字节；SHA256：497AB85F0D13475B332492810172A4BB08D8B8A95512BC11AAE4E741FE67EFCB。日志 target/table-validation-release-build.log。
- 包含三种分隔文本的列类型检查、错误标红和数量、上一个 / 下一个双向定位，以及先采样 3 行、不确定再扩展 5 行的策略。实际 GUI 与测试执行待验证，未正式发布。

## 表格查找 Release 构建验证

- 用户明确授权后确认程序未运行且目标 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23721984 字节；SHA256：B61AD1A5E300117EDF226C8B04B5C6737F46A2C6062055B36FF240EC3DACD6A1。日志 target/table-search-release-build.log。
- 包含表格滚动条常显、简化的上一个 / 下一个文案、单元格查找与淡绿色匹配标记。实际 GUI 与测试执行待验证，未正式发布。

## 分档查找 Release 构建验证

- 用户明确授权后确认程序未运行且目标 EXE 无占用，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23801856 字节；SHA256：197094E0B9EBBF1EF1017336FA3A70101900923A61712BD02BAEE5F3C2B22BD7。日志 target/table-search-tier-release-build.log。
- 包含 Ctrl+F 聚焦查找、小表实时 / 中表延迟 / 大表回车后台查找，以及淡绿匹配标记和前后定位。测试实际执行、GUI 交互和性能仍待验证，未正式发布。

## 表格查找导航布局修复 Release 构建

- 用户明确授权后确认程序未运行且 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23758336 字节；SHA256：80298AF76ABF56C861C2555001DA834C9A8CADA49086D61334AC7B750D2E7EA8。日志 target/table-toolbar-release-build.log。
- 包含匹配导航栏限制单行高度、恢复表格视口的布局修复。实际 GUI 仍待运行验证，未正式发布。

## 单元格编辑栏与翻译清理 Release 构建

- 用户明确授权后确认程序未运行且目标 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23707648 字节；SHA256：DD39601D9A83472FA232A6E18559FF17DB5D9974A2E488C41B4B42874AB281B7。日志 target/table-cell-bar-release-build.log。
- 包含紧凑双行表格工具栏、悬浮提示去重、字母列标 / 从零行号、同步单元格编辑栏及 173 条废弃翻译清理。实际 GUI 和测试执行仍待验证，未正式发布。

## 表格界面细节 Release 构建

- 用户明确授权后确认程序未运行且目标 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23718400 字节；SHA256：459BF7E596C03530D28E143C4DF57E2635B21FA6531DEDAB2B2E9F78BF301155。日志 target/table-polish-release-build.log。
- 包含高对比常显滑条、文件名同行操作按钮、工具栏分隔间距、地址对齐和弱化文字色，以及 1 px 选中描边。实际 GUI 交互仍待验证，未正式发布。

## JSON 对象与数组识别 Release 构建

- 用户明确授权后确认程序未运行且目标 EXE 可独占打开，cargo build --release --offline --locked 成功（7 条应用警告）。
- 版本 1.2.4；产物 target/release/unity_yaml_viewer.exe；大小 23726080 字节；SHA256：A43EFC4E0372110CB3EE4148DC9EFDC99F0D4848EB0513BEC55DBBBDE0638887。日志 target/table-json-kind-release-build.log。
- 包含 JSON 对象 / 数组类型识别与校验、编辑栏移到分隔线上方，以及顶部搜索展开隐藏指令查看。实际 GUI 与测试执行仍待验证，未正式发布。

## 每日更新检查与材质预览 Tab 构建（2026-10-19）

- 用户明确授权后重新检查程序未运行、目标 EXE 可独占访问；cargo build --release --offline --locked 成功，7 条应用警告。
- 包含启动自动检查更新每 24 小时最多一次、材质 Shader 预览 2D / 3D 主题 Tab 样式。
- 产物 target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 23710720 字节；SHA-256：10F1E471C5A42250A03E1CF14A9C1D81C8AD983DF1CAA3BA520ABBD6DA002D04。日志 target/latest-release-build.log。
- 实际 GUI 与跨重启节流尚未运行验证，本次为本地构建，未更新 GitHub Release。

## 材质节点控件缩放修复构建（2026-10-19）

- 用户明确授权后检查程序未运行、EXE 可独占访问；cargo build --release --offline --locked 成功（7 条应用警告）。
- 包含 2D / 3D Tab 及背景 / 模型 / Cull / Blend 下拉框与菜单的缩放修复。
- 产物 target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 23719424 字节；SHA-256：C602164FB2E96B54ADFB470C2F9314E7BA421E0BBBDC1D9D674A89D09DD84BE5。日志 target/material-zoom-release-build.log。
- 实际缩放与菜单 GUI 待体验验证，未发布。

## 九宫格方向对齐修复构建（2026-10-19）

- 用户明确授权后检查程序未运行、目标 EXE 可独占访问；cargo build --release --offline --locked 成功（7 条应用警告）。
- 包含九宫格精简方向中心区 4 倍数对齐、未精简方向透明外补与零 Border，以及七语言关于摘要。
- 产物 target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 23724032 字节；SHA-256：4FECFA2B5D0E4774D75A4D9CE3E24ED5B21E500F7F93900D29A18ECAF320E698。日志 target/nine-slice-align-release-build.log。
- 实际 GUI 与测试执行仍待验证；本次仅本地构建，未发布。

## 四方连续图正式配置本地构建（2026-10-20）

- 用户明确授权编译后，检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 34.11 秒，8 项应用警告。
- 包含独立四方连续图弹窗、精确最小循环提取、多图案排列、错位、间距、平铺预览和 PNG 另存；版本 1.2.4。
- 产物：target/release/unity_yaml_viewer.exe；大小 23906816 字节；SHA-256：ED1C77F789769CC640F6F613BDE07E801D13BD8B63F5C77CF245951832CA013D。构建日志：target/seamless-release-build.log。
- 本次仅本地构建，未发布、未启动程序；回归测试执行、真实素材、独立弹窗及七语言 GUI 验证仍待完成。

## 连续图后续源码修改（2026-10-20，未重新构建）

- 本地已加入拖入图案自动添加、列表移除 / 编号维护、自动后台实时预览、独立参数滚动、左侧编辑 / 右侧平铺，移除原图显示和生成按钮。
- 支持按排列位置独立选中、拖动和 X / Y 输入，以及“单图双排错位”预设；图案透明补齐 4 倍数，格距自动兼顾 4 和错位分母，导出按完整周期生成最小 4 对齐循环。
- cargo check --tests --locked --offline、七语言静态检查（1033 条）和 git diff --check 通过；新增与更新回归用例未执行，真实界面 / 拖放 / 导出待验证。
- 现有 target/release/unity_yaml_viewer.exe 为前文记录的 34.11 秒构建，不包含上述后续源码。用户本轮未再次要求编译，未生成新 EXE 或发布；下次构建仍须明确授权并检查进程与目标 EXE 独占访问。

## 连续图实时编辑本地构建（2026-10-20）

- 用户明确授权后，检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 32.51 秒，8 项应用既有警告。
- 包含拖入图案及列表移除、自动实时预览、左右编辑 / 平铺布局、单图双排错位、独立位置拖动和 4 倍数补齐；不显示原图，无手动生成按钮。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 23937536 字节；SHA-256：DC43BA68FCDB7EEF1C368E6B4D3656BF58B2074C937732AE76E3CB0DD696DD0A。日志：target/seamless-live-release-build.log。
- 本次仅本地构建，未发布、未启动程序；回归断言执行、真实 GUI / 拖放 / 拖动手感 / PNG 输出验证仍待完成。

## 连续图三栏与画布自动扩展本地构建（2026-10-20）

- 用户明确授权后检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 31.77 秒，8 项应用既有警告。
- 包含实时编辑 / 最小循环单元 / 连续平铺三栏，移动图案自动扩展四侧画布、4 倍数补齐、固定拖动换算比例，以及此前图案拖入、移除、双排错位和实时预览功能。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 23936512 字节；SHA-256：3FFDF670E5219F10C0DDA7C653CCB4C40BD1822FFACF35CB80BFF4474EE7270C。日志：target/seamless-expand-release-build.log。
- 本次仅本地构建，未发布、未启动程序；真实 GUI / 拖动手感 / 系统拖放 / PNG 输出及回归断言执行仍待验证。

## 固定编辑画布与弹窗边界修正本地构建（2026-10-20）

- 用户明确授权后，确认 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 30.86 秒，8 项应用既有警告。
- 包含固定虚拟编辑视口、缩放 / 平移、独立元素拖动、无循环副本编辑、间距同步，以及三栏 / 4 对齐 / 图案拖入等既有功能；同时包含共享弹窗先居中主窗口再修正屏幕工作区越界、过大窗口缩小。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 24113664 字节；SHA-256：B6234ECBFBC578981845F0205D70C40F078537D023E7D1C9C5D9F503166A01A0。日志：target/seamless-world-canvas-release-build.log。
- 本次仅本地构建，未发布、未启动程序；真实 GUI / 缩放平移 / 间距联动 / 多屏边界 / PNG 输出及回归断言执行仍待验证。

## 拖动修复与工具栏列表调整本地构建（2026-10-20）

- 用户明确授权后检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 33.69 秒，8 项应用既有警告。
- 包含图案连续拖动累计位移修复、真实按下点命中、边框开关与 PNG 另存工具栏归位、图案列表最长名称宽度及居左、移除底部重复像素尺寸，保留此前固定画布、三栏和弹窗边界修正。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 24151552 字节；SHA-256：BD6A36E2D919A48E16595342EDBFCDBFA76D0BA032F0F64950D7A4A80853844F。日志：target/seamless-drag-toolbar-release-build.log。
- 本次仅本地构建，未发布、未启动程序；连续拖动手感、工具栏 / 列表 GUI、七语言与 PNG 输出及回归断言执行仍待验证。

## 自定义循环宽高与主题 Tab 本地构建（2026-10-20）

- 用户明确授权后检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 31.04 秒，8 项应用既有警告。
- 包含自定义循环宽高 / 4 对齐裁剪及补背景、提取与制作 / 排列方式主题 Tab，以及图案列表固定左内边距文字绘制，保留此前拖动修复和工具栏调整。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 24105984 字节；SHA-256：53ADA538A237D0929B020A28B74814399D820592BBDD59D332DD6DE15BBF88D7。日志：target/seamless-custom-size-tabs-release-build.log。
- 本次仅本地构建，未发布、未启动程序；自定义宽高、Tab 深浅主题、图案文字左对齐等真实 GUI 与回归断言执行仍待验证。

## 无更小循环回退原图本地构建（2026-10-20）

- 用户明确授权后检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问；cargo build --release --locked --offline 成功，耗时 34.26 秒，8 项应用既有警告。
- 包含提取不到更小循环时直接显示原图并用于平铺、保留原始像素和尺寸的回退逻辑，保留此前自定义循环宽高、主题 Tab、列表居左、连续拖动和工具栏修改。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.4；大小 24135168 字节；SHA-256：CD3BC0CBDB28C597E869067158B9395A0E7BAD4E388DF776660DD7EC0780215F。日志：target/seamless-original-fallback-release-build.log。
- 本次仅本地构建，未发布、未启动程序；原图回退 / 平铺、提示及真实 GUI 与回归断言执行仍待验证。

## 升级 1.2.5

发布后改动已统一归入 docs/versions/1.2.5.md；升级前逐轮过程移至 docs/versions/archive/1.2.5-development-history.md。关于窗口当前版 4 条简短摘要、默认 1.2.5 / 1.2.4，1.2.3 及更早历史默认收起；原本地 1.2.4 EXE 已被本次 1.2.5 构建替换。

### 本轮 1.2.5 验证与本地产物（2026-10-20）

- 用户明确授权重新编译，编译前检查程序未运行、目标 EXE 可独占访问，未自动关闭程序。
- cargo check --tests --locked --offline 通过；45 项相关回归实际执行并通过：四方连续图 16、九宫格 22、弹窗定位 5、本地化 2。其余测试未执行，不代表完整应用回归。
- cargo build --release --locked --offline 成功（42.58 秒，8 项既有应用警告）。日志：target/v1.2.5-release-build.log；测试日志：target/v1.2.5-seamless-tests.log、target/v1.2.5-asset_viewer-nine_slice-tests.log、target/v1.2.5-native_window-tests.log、target/v1.2.5-localization-tests-tests.log。
- 产物：target/release/unity_yaml_viewer.exe；文件版本 1.2.5；大小 24090112 字节；SHA-256：8CE57ADF24DCE0A8C6F7D63C5E62FC6DADC2F1924A26919C30F4E032316ABD76。
- 本次未启动 GUI，未创建 GitHub Release、上传附件或改动标签；正式发布的 1.2.4 保持不变。完整运行验证仍按上方待验证范围执行。

## 1.2.5 已正式发布

### GitHub 正式发布（2026-10-20）

- [v1.2.5](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.5) 已于北京时间 2026-10-20 18:55:24 发布；显示标题仅为 v1.2.5，非草稿 / 非预发布，latest 接口核对为本版。Release ID：418294717。
- 标签 v1.2.5 指向 c13784c68dd5b2d830ba784f626e1b3cbc31f192，与本次构建源码一致，发布时远程 master 为该提交。
- 附件 UnityYAMLViewer-v1.2.5-windows-x64.exe，24,090,112 字节；GitHub asset.digest 与本地 SHA-256 一致：8CE57ADF24DCE0A8C6F7D63C5E62FC6DADC2F1924A26919C30F4E032316ABD76。
- 上传在草稿阶段完成状态、大小和摘要核验后正式发布；发布过程未重新编译、未改变旧版标签或附件。证据：target/v1.2.5-published.json。
- 45 项相关回归通过；完整 GUI、多屏 / DPI 和端到端输出仍未全部覆盖，发布成功不等同完整回归通过。发布后记录为本地文档更新，未自动提交或推送此状态记录。

## 序列工具本地构建（2026-10-21）

- 用户明确授权编译；构建前检查 unity_yaml_viewer 进程未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功，耗时 33.23 秒，8 项既有应用警告。日志：`target/sequence-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6，大小 24482816 字节，SHA-256：`8BD8D276D7AEFFE7C9EFBA927457A443DEA451E6598883DF541E7806A429B5A9`。包含序列拆分 / 制作、排序移除、共同最小尺寸裁剪、双向 Padding、4 对齐与无空格自动排列。
- 本轮未执行自动测试或启动 GUI；此前 `cargo check --tests --locked --offline` 通过，测试仅编译检查。实际导出、界面与七语言、关于历史折叠仍待运行验证。
- 仅完成本地构建，未提交 / 推送、未发布新版或更改现有 Release / 标签 / 附件。

## 序列左右布局与预览本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（45.27 秒，8 项既有应用警告）；日志 `target/sequence-preview-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24571392 字节；SHA-256：`F1AE57A3E1236002654C52A25B000075C346F3DDD3C90E176AB04903AA65FE99`。包含左右布局、主题 Tab、右上方导出、整图分格和点击单帧预览。
- 本轮未启动 GUI 或执行自动测试；此前编译检查通过，界面布局、不同 DPI、分格点选与导出结果对照、七语言及关于历史折叠仍待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列最新功能本地构建（2026-10-21）

- 用户明确授权编译；首次检查目标文件被占用（Windows 错误 32），未启动构建。用户再次确认后重新检查，程序未运行且目标 EXE 可独占访问，未自动关闭程序或绕过占用。
- `cargo build --release --locked --offline` 成功（36.49 秒，8 项既有应用警告）；日志 `target/sequence-latest-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6，大小 24582656 字节，SHA-256：`EFFBF03D5F29F74C7D23ED7889607458BE60D0C23B3507409148909A7B1DC1BA`。
- 包含左预览 / 右设置、智能识别行列、拆分命名和空白 / 非法字符红字检查、主题 Tab 等高绿色导出按钮、制作拖入图片、单格居中及可选图片边框、拆分可选居中透明补齐为 4 的倍数。补白整数分配：1 像素放右 / 下，2 像素两侧各 1，3 像素左 / 上 1、右 / 下 2，不产生插值。
- 本轮未执行自动测试或启动 GUI；相关测试此前仅通过编译检查。实际识别、拖入、命名 / 导出、预览边框、七语言和关于历史折叠仍待运行验证。
- 仅本地构建，未提交 / 推送，未更改现有 Release、标签或附件。

## 序列预览与布局更新本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（32.41 秒，8 项既有应用警告）；日志 `target/sequence-ui-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24742400 字节；SHA-256：`205B129E57E76052DA065D01A74BB7CF5895BD921E496D4599BBF65980C10D37`。
- 包含拆分参数顺序、单格与补齐尺寸摘要、智能识别改进、右侧等比单帧预览及前后切换、顶部共享背景设置和命名输入框自适应宽度。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查，用户原始 PNG 尚未取得，不能宣称实际识别为 5 × 3 已验证。界面、比例、识别、导出、七语言及关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列 meta 与制作排列更新本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（37.47 秒，8 项既有应用警告）；日志 `target/sequence-meta-layout-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24746496 字节；SHA-256：`43F1B18FACAFE2DAE08E03C262D55EFECDE88E7FEB1AAD8FA05F9886C27C5311`。
- 包含导出成功通知、拆分末端补边排除、Multiple meta 优先切片、当前图片默认参与制作且可移除、矩形优先 / 不留空格排列、列表与单帧双向选择、图片名显示和手动行列联动。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查。实际 meta、分格与导出结果、图片列表点选 / 滚动 / 移除、行列联动、七语言和关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列缩放、间隔和 tpsheet 本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（33.61 秒，8 项既有应用警告）；日志 `target/sequence-tpsheet-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24706560 字节；SHA-256：`01CCEEA504A9EBA442E52CB3F0E138BD065BACE577B07C12929FF57C9C3BC73C`。
- 包含拆分拖入自动切换制作、单图宽高与比例联动、裁剪后缩放、水平 / 垂直 Padding 与右下 4 对齐、自动排列同排 Tab 与下方候选、可选同步导出 Unity tpsheet。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查。Unity / TexturePacker Importer 实际导入未验证；界面、缩放 / 间隔、导出与覆盖、七语言和关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列居中、播放与预览操作本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（35.30 秒，8 项既有应用警告）；日志 `target/sequence-centered-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24698368 字节；SHA-256：`EE90BCDC25635A836EA24E8CCB02B9BD9016C379F7F13571AC948D0A59E240F7`。
- 包含同组图片自动加入与自然排序、拆分按钮同行、播放 / 帧率 / 循环开关、滚轮缩放与拖动平移、显式适应窗口按钮、制作图片在最终格子内居中及 tpsheet 坐标同步、单帧图片名 / 实际尺寸 / 缩放合并显示。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查。实际粒子序列效果、Unity tpsheet 导入、自动加入、播放与缩放交互、七语言及关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列播放工具栏对齐本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（38.37 秒，8 项既有应用警告）；日志 `target/sequence-toolbar-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24749568 字节；SHA-256：`6E89BA5105DBF77A3EF0043E1440A8A132E20E348C42C7A2C20BD3D0624BD68D`。
- 包含单帧播放工具栏垂直居中及按钮 / 帧率输入高度对齐。
- 本轮未启动 GUI 或执行自动测试；实际对齐、DPI、七语言及关于历史折叠待运行验证，Unity tpsheet 导入也尚未验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 原图尺寸初始化与保存按钮本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（33.64 秒，8 项既有应用警告）；日志 `target/sequence-original-save32-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24701440 字节；SHA-256：`8E788887527483F6308267E214B0A590808CA1E7AF12CE806A11710A4BBDC41E`。
- 包含制作首次按原图实际宽高 / 100% 初始化，以及四方连续图另存为按钮高度 32 逻辑像素。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查，实际原图初始化、按钮高度、DPI、七语言及关于历史折叠待运行验证；Unity tpsheet 导入仍未验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 序列多余留白修复本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（35.04 秒，8 项既有应用警告）；日志 `target/sequence-spacing-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24701952 字节；SHA-256：`C9B7602A795431504DEDAC9AA14BF8F389F8D09D4D95C076197251E3938BE25A`。
- 包含单帧播放栏上下多余留白修复，以及识别 / 导出状态提示移入右侧设置区、避免顶部整行空白。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查，实际两模式布局、识别前后状态、窗口缩放 / DPI、七语言及关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 通道 Alpha 与组件手动路径本地构建（2026-10-21）

- 用户明确授权编译；构建前确认程序未运行、目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（32.84 秒，8 项既有应用警告）；日志 `target/manual-component-paths-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24836096 字节；SHA-256：`6B95AE9EEA39E4F1D329A8EFBEEB2A721B8A94968819C139269DD682916C4720`。
- 包含 R / G / B 应用 Alpha 开关，以及独立 FFmpeg、FFprobe 和 PDFium DLL 手动路径、保存 / 恢复自动功能，无需修改 PATH。
- 本轮未启动 GUI 或执行自动测试；相关测试仅通过编译检查。实际通道预览、手动组件验证与调用、持久化 / 重试、七语言及关于历史折叠待运行验证。
- 仅本地构建，未提交 / 推送，未修改现有 Release、标签或附件。

## 四边补边、GIF／PNG 与序列去底本地构建（2026-10-21）

- 用户明确授权编译；构建前未发现程序进程，目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（35.28 秒，8 项既有警告）；日志 `target/sequence-gif-key-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 24872960 字节；SHA-256：`2998F7B2E845A5CF5847E983185935ECFC79C210B1DDF6B46FD8F77DB5A2E268`。
- 包含拆分四方向补边、GIF 帧率/颜色压缩/滤波/循环设置、GIF 转视频、GIF 与视频导出 PNG 序列、制作/拆分转入视频编辑、视频导出底色，以及制作/拆分共用吸色去底、容差遮罩、边缘柔化与去色边。
- FFmpeg 等价链路样例验证与编译检查已完成；本轮未启动 GUI 或执行 Rust 测试，实际取色/容差预览、窗口交互、导出、缓存生命周期、七语言及关于历史折叠仍待运行验证。
- 仅本地构建，未提交/推送，未修改现有 Release、标签或附件。
## 去水印、序列历史与缓存帧预览本地构建（2026-10-21）

- 用户明确授权编译；构建前未发现程序进程，目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（41.83 秒，8 项既有应用警告）；日志 `target/watermark-sequence-preview-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 25229824 字节；SHA-256：`D7260775B10A95D3C941B6835F5972323CFAAFA520512E97F44809BEA27FCCD6`。
- 包含吸取底色操作指引/取色光标与候选色值、独立蒙版去水印/去底色及结果缓存、制作和拆分序列操作撤销/重做、序列转 GIF/视频直接读取缓存 PNG 的首帧/逐帧定位/播放及透明棋盘背景；转换默认取序列预览 FPS。
- 本轮未启动 GUI 或执行 Rust 测试；新增测试仅通过此前编译检查。实际蒙版修补效果、历史手势归并、缓存保存、窗口交互、序列首帧和播放/定位/暂停、七语言及关于历史折叠仍待运行验证。
- 仅本地构建，未提交/推送，未修改现有 Release、标签或附件。
## 相似水印、空帧与预览边界更新本地构建（2026-10-21）

- 用户明确授权编译；构建前未发现程序进程，目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（43.85 秒，8 项既有应用警告）；日志 `target/latest-media-watermark-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 25327616 字节；SHA-256：`CA0B71ADBBA1CBF2852F734AA7B5AE9010BB648D98A29380408C862BC20CB187`。
- 包含单样本本地相似水印查找/一键修补、去水印背景 Tab 左上角布局、四方连续图预览顶部对齐、拆分跳过透明空帧、视频选区终点预览排除边界、GIF 默认 Bicubic＋Sierra2 4A，以及音视频/GIF 同名文件编码成功后替换。
- 此前编译检查通过；本轮未执行 Rust 测试或启动 GUI。实际匹配/修补效果、空帧播放导出、选区末帧停留、文件覆盖与失败保护、窗口布局、七语言及关于历史折叠待运行验证。
- 仅本地构建，未提交/推送，未修改现有 Release、标签或附件。
## 紧凑补边标签与隐藏去水印入口本地构建（2026-10-22）

- 用户明确授权编译；构建前未发现程序进程，目标 EXE 可独占访问。
- `cargo build --release --locked --offline` 成功（37.49 秒，8 项既有应用警告）；日志 `target/compact-padding-hidden-watermark-release-build.log`。
- 产物：`target/release/unity_yaml_viewer.exe`，版本仍为 1.2.6；大小 25338880 字节；SHA-256：`AAE68E92E45B789E7CD5E6532AA547A6181227DF7A43610B5130C1543343FB2B`。
- 包含拆分“忽略补边：上/下/左/右”紧凑分组输入，以及按用户要求隐藏图片去水印入口；去水印实现保留，序列去底色继续可用。
- 本轮未启动 GUI 或执行 Rust 测试；实际标签布局、入口隐藏、序列去底色、七语言和关于历史折叠待运行验证。
- 仅本地构建，未提交/推送，未修改现有 Release、标签或附件。

## 1.2.7 发布准备（2026-10-22）

- 用户明确授权升级为 1.2.7 并重新编译，确认本版增量新增与优化已经验证无问题；去水印入口继续隐藏。
- 已将 v1.2.6 发布之后的开发内容迁入 docs/versions/1.2.7.md 和 archive/1.2.7-development-history.md；恢复 1.2.6 已发布范围并保留发布证据，准备 docs/releases/1.2.7.md，更新版本索引及 CHANGELOG。
- Cargo.toml／Cargo.lock／EXE 均为 1.2.7；关于默认 1.2.7 与 1.2.6，其他版本折叠，保留旧摘要及七语言。1345 条七语言、版本顺序、重开重置与 diff 静态检查通过；本轮未启动 GUI 或重新执行自动测试。
- 编译前确认程序未运行且原目标 EXE 可独占访问；cargo build --release --locked --offline 成功，37.54 秒，8 项既有应用警告。日志 target/v1.2.7-release-build.log。
- 原始产物 target/release/unity_yaml_viewer.exe；发布候选 target/releases/v1.2.7/UnityYAMLViewer-v1.2.7-windows-x64.exe；25283584 字节；SHA-256：B5C1C2F5157BE23553BCAA1AFCB17FFF8F988D926CE6DB02A881415349A16D48。校验文件同目录 SHA256SUMS.txt。
- 构建来源：HEAD 95355a69352a66c9671515689ec6821f90bfccc8 加本轮未提交版本／关于改动；完整构建输入摘要 target/v1.2.7-build-manifest.json。用户提交后、发布前核对输入摘要，若源码改变则重新编译。
- 用户自行提交 Git，等待其提交后另行明确通知才发布 GitHub；本轮未提交、推送、打标签、创建 Release 或上传附件。发布标题仅 v1.2.7。


## 1.2.7 GitHub 正式发布（2026-10-22）

- 用户在提交后明确授权发布；目标提交 `17a51c6c9e249780361e649c8cc73b0dcbd27cff`，与远端 master 一致，构建输入及发布附件校验全部匹配。
- 北京时间 2026-10-22 14:58:04 正式发布 https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.7；Release ID 420396003，标题 v1.2.7，非草稿／非预发布，latest 已核验。
- 附件 UnityYAMLViewer-v1.2.7-windows-x64.exe，25,283,584 字节；GitHub digest 与本地 SHA-256 一致：B5C1C2F5157BE23553BCAA1AFCB17FFF8F988D926CE6DB02A881415349A16D48。先上传验证，再转正式发布，未重新构建、未修改旧版标签或附件。
- 发布证据 target/v1.2.7-published.json；发布后已更新本地版本记录、索引和发布状态，未自动提交或推送这些文档。

## 目录图片浏览本地构建（2026-10-22，未发布）

- 用户明确授权编译；检查程序未运行、目标 EXE 可独占访问后，cargo build --release --locked --offline 成功（36.44 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25388544 字节；SHA-256：7FE1B1F310CA93EA20CB607F4FFEA4B7FE002EF944511FD11E20A3B4B1E93B47。
- 构建日志：target/image-browser-release-build.log。实际界面仍待运行验证，未上传或更新 GitHub Release。

## 固定底部缩略图布局本地构建（2026-10-22，未发布）

- 用户再次明确授权编译；检查程序未运行、目标 EXE 可独占访问后，cargo build --release --locked --offline 成功（33.37 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25392128 字节；SHA-256：6F85699826A7DEA15D7DBEB3790D4752269E1B62A6358E0901142832E21980DF。
- 构建日志：target/image-browser-layout-release-build.log。固定底栏布局待实际界面验证，未更新 GitHub Release。

## 缩略图选中与路径提示本地构建（2026-10-22，未发布）

- 用户明确授权编译；检查程序未运行、目标 EXE 可独占访问后，cargo build --release --locked --offline 成功（35.42 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25355264 字节；SHA-256：CAE58B8C3AB7B8CA5853F763AE4645162374C7FE20EAB64F9CC007222D2CC64A。
- 日志：target/image-browser-selection-release-build.log。含缩略图下移、圆形导航、蓝色选中线框和路径悬停提示；实际界面待确认，未更新 GitHub Release。

## 图片浏览细调与缩放菜单本地构建（2026-10-22，未发布）

- 用户授权并确认程序未运行、目标 EXE 可独占访问后，cargo build --release --locked --offline 成功（35.19 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25338368 字节；SHA-256：17D990FB3930B16F3CC5636D9101407DA9B08728563660FFC3C6D22C25F57B7C。
- 日志：target/image-browser-refinements-release-build.log。含比例快捷菜单、多图位置、四方向键、图片边界修复与 2 像素选中框；实际运行待验证，未更新 GitHub Release。

## 设置与菜单对齐本地构建（2026-10-22，未发布）

- 用户授权并确认程序未运行、目标 EXE 可独占访问后，cargo build --release --locked --offline 成功（35.03 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25345536 字节；SHA-256：D504F4FF0E0C2C9AD3C42A6584BD36D6D7310EE38FCBDDE235802C44DFD292A5。
- 日志：target/image-preview-settings-release-build.log；包含浅色底栏、预览设置更名、默认多图模式开关与菜单宽度对齐，实际运行待验证，未更新 GitHub Release。

## 透明缩略图最终样式本地构建（2026-10-22，未发布）

- 用户明确授权；编译前确认程序未运行且目标 EXE 可独占访问，cargo build --release --locked --offline 成功（39.46 秒，8 项工程警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25442816 字节；SHA-256：657B03776EE1DFF3523C3926880DC4CC85B5D580776BF41462BBDC5E028C2C83。
- 日志：target/image-browser-checkerboard-release-build.log。包含透明棋盘缩略图、去遮罩和 3 像素蓝色框；实际界面待确认，未更新 GitHub Release。

## 目录树与独立忽略规则本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表检查未发现程序运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（50.15 秒，8 项应用警告）。
- 已包含目录拖入、左侧目录树、多根目录、刷新与右键移除、每个根目录独立忽略文件规则，以及移除最后一个根目录后隐藏侧栏。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25532928 字节；SHA-256：9CD6155243F10487FC834A78EAB2010FC950908522F82FE9BD555AB73692C87C。
- 日志：target/folder-browser-release-build.log。git diff --check 与 1362 条翻译七列完整性、键唯一性检查通过。实际界面与交互仍待运行验证；未发布或上传附件。本条构建状态取代上述本轮未生成可执行程序的阶段性描述。

## 目录浏览交互完善本地构建（2026-10-28，未发布）

- 用户再次明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（40.97 秒，8 项应用警告）。
- 已包含目录侧栏占用原页签左侧区域、文件名单行与按需加宽、忽略规则灰色示例和白色实际输入、菜单标题与说明消歧义、根目录等文字高度齿轮入口及保留右键菜单、目录树打开不写入或重排最近打开记录。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25508864 字节；SHA-256：49657016477D79EA4FD7BD262BAB31A44E562941E0647A4C5EB520299EB2A43B。
- 日志：target/folder-browser-final-release-build.log。git diff --check、1371 条七语言列数与键唯一性静态检查通过。实际界面与交互仍待运行验证，本条构建状态取代上述尚未生成 EXE 的阶段性描述；未发布或上传附件。

## 目录浏览性能与布局修正本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（41.59 秒，8 项应用警告）。
- 已包含目录标题与页签对齐、滚动区域贴底与纵向滑条靠近分割线、小号齿轮、目录列表及忽略规则跨重启保存、默认宽度与手动调整及超长名称截断、目录预览页签复用和同目录缩略图缓存保留、层级线可见区裁剪与物理像素对齐。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25598464 字节；SHA-256：63748B5FCD70200741D5D5A2A790A54AA2F872DBF8AA3D35239D5B99F8998D90。
- 日志：target/folder-browser-latest-release-build.log。git diff --check、1376 条七语言列数与键唯一性检查通过。此前回归测试只完成编译检查，未执行；本轮未启动 GUI，不宣称性能与视觉问题已运行验证。本条构建状态取代上述未生成可执行程序的阶段性描述；未发布或上传附件。

## 目录预览固定首位与滚动条修正本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（52.57 秒，8 项应用警告）。
- 已包含修改后的目录预览保留普通页签与同类合并、目录预览独立分组且固定首位、页签标题使用实际点击所属根目录名称，以及固定宽度滚动条与分割线命中区域分离。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25598464 字节；SHA-256：6E7980BF2E8DF9D8CA0D8041E9C65C8C43F5D30BF735A477EDECC1F6C40B2D88。
- 日志：target/folder-preview-pinned-release-build.log。git diff --check、1380 条七语言列数与键唯一性检查通过。回归测试此前仅编译检查，未执行；本轮未启动 GUI，实际修改保护、页签顺序与标题、滚动条边缘闪烁待运行验证。本条构建状态取代上述未生成可执行程序的阶段性描述；未发布或上传附件。

## PSD 只读预览本地构建与体积比较（2026-10-28，未发布）

- 用户明确授权编译；编译前确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（46.42 秒，8 项应用警告）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25574912 字节（24.39 MiB）；SHA-256：311C68B8688D11AA3336C892FD3531260194EABA71EB5ACF771AA81A234AF227。
- 上一个本地构建为 25,598,464 字节；本次净减少 23,552 字节（23 KiB，约 0.092%）。该数值是前后 EXE 的净体积差，包含 PSD 解码、入口、文案及构建布局影响，不等同于解码模块单独体积。
- 日志：target/psd-preview-release-build.log。git diff --check、1383 条七语言列数与键唯一性检查通过。真实 PSD 样本及界面仍待运行验证；测试此前仅编译检查未执行。本条取代此前尚未生成可执行程序的阶段性状态，未发布或上传附件。

## ICO 与 PSD 多图浏览本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（44.42 秒，8 项应用警告）。
- 已包含 ICO 只读预览、ICO／PSD 加入底部多图列表、统一主图与缩略图解码、缩略图错误提示，以及保留普通位图工具的格式限制。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25545728 字节（24.36 MiB）；SHA-256：E3E8CF203225BEA7773E368A5D5056ECEEBA1BE50EAE450EF8C1FF8CCF2EB8C3。
- 日志：target/ico-psd-browser-release-build.log。git diff --check、1384 条七语言列数与键唯一性检查通过。回归测试此前仅编译检查未执行；本轮未启动 GUI，实际 ICO 透明度、PSD／ICO 混合目录切图与性能待运行验证。本条取代尚未生成可执行程序的阶段性状态；未发布或上传附件。

## 根目录底色与 Spine 路径修正本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（35.01 秒，8 项应用警告）。
- 已包含根目录标题行深浅主题淡底色，以及 Spine 贴图路径分隔符统一，保留 Windows 扩展路径、网络 UNC 与 Unicode 路径支持。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25626112 字节（24.44 MiB）；SHA-256：81E380C95D411646D7C7C7913209561282B3C69114331FFA6709458329892D00。
- 日志：target/spine-folder-path-release-build.log。git diff --check、1386 条七语言列数与键唯一性检查通过。回归测试此前仅编译检查未执行；原网络目录 Spine 资源、动画及根目录底色实际显示仍待运行验证。本条取代上述尚未生成可执行程序的阶段性状态；未发布或上传附件。

## Spine 预乘 Alpha 与预览默认设置本地构建（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（42.26 秒，8 项应用警告）。
- 已包含 Spine 默认动态播放、辅助线紧凑排列、普通预览预乘 Alpha 开关（默认开启，位于辅助线后）、材质状态提示移到设置／重载下方并使用普通文本颜色，以及图片默认适应窗口开关（默认开启并持久化，关闭时新图 100%）。
- 产物：target/release/unity_yaml_viewer.exe；版本 1.2.7；大小：25665024 字节（24.48 MiB）；SHA-256：097BBAE69E3B83030E2AED1BA416A3C998BDD63E43030482C1D164BA5C792E02。
- 日志：target/spine-pma-settings-release-build.log。git diff --check、1396 条七语言列数与键唯一性检查通过。回归测试此前仅编译检查未执行；本轮未启动 GUI，PMA 边缘效果、材质回退、默认播放、设置持久化及布局仍待运行验证。本条取代上述未生成可执行程序的阶段性状态；未发布或上传附件。

## 1.2.8 版本升级与功能归属整理（2026-10-28）

- 用户确认本轮已有功能与优化均验证通过，并授权升级源码版本至 1.2.8。主要新增为目录浏览模式、多图浏览模式，ICO／PSD 为查看能力补充，Spine、比例菜单、边界与布局等归为优化。
- Cargo.toml 与 Cargo.lock 同步 1.2.8；本轮未生成 EXE，当前 target/release/unity_yaml_viewer.exe 仍为前一轮 1.2.7 本地产物（25,665,024 字节，SHA-256 097BBAE69E3B83030E2AED1BA416A3C998BDD63E43030482C1D164BA5C792E02）。未发布、未创建标签或上传附件。
- 已将 docs/versions/1.2.7.md 中正式 Release 之后的开发增量迁入 docs/versions/archive/1.2.8-development-history.md；新建 docs/versions/1.2.8.md 汇总最终能力、限制及用户验证状态，1.2.7 保留正式发布事实及归属更正。
- 新建 docs/releases/1.2.8.md 发布说明草稿；版本索引与 CHANGELOG 链接同步。README 保持当前能力说明，不新增版本日志或发布流程信息。
- 关于窗口默认展示 1.2.8 与 1.2.7；1.2.6 及更早移入原有“查看更多历史”，重新打开恢复折叠。1.2.8 六条摘要同步七语言，1.2.7 摘要依据已发布内容保留序列等能力，既有更早历史与翻译保留。
- 用户验证结论覆盖升级前功能；1.2.8 关于新摘要与版本标识的实际界面仍待后续构建核对，不以源码检查代替运行验证。
- 本轮 cargo check --tests --locked --offline 通过（14.42 秒，8 项应用警告），日志 target/v1.2.8-version-check.log；1402 条翻译与关于默认两版的 13 条摘要、历史折叠和重开重置静态核对通过，审计 target/v1.2.8-static-audit.json。现有 EXE 的 1.2.7 版本与哈希未变化；未编译 1.2.8、未发布。

## 1.2.8 本地 Release 构建与 README 精简（2026-10-28，未发布）

- 用户明确授权编译；编译前通过进程列表确认程序未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（45.94 秒，8 项应用警告）。
- 产物：target/release/unity_yaml_viewer.exe；文件／产品版本均为 1.2.8；大小：25616384 字节（24.43 MiB）；SHA-256：11B273899A35B636F57584B472F0CBB432E12A72BB2DE510B289CA3938C121B3。
- 日志：target/v1.2.8-release-build.log；构建输入摘要：target/v1.2.8-build-manifest.json，覆盖 165 个源码、依赖配置与资源文件，供提交后核对。此前 1.2.7 产物说明为历史记录，本次不沿用旧版本 EXE。
- README 对照既有按能力条目介绍的结构，将目录浏览、多图浏览各压缩为一句，合并 ICO／PSD 到文件预览；保留简短格式限制、使用及构建说明，中文英文同步，不加入版本日志或内部发布流程。
- git diff --check、1402 条翻译列数／键唯一性检查通过。用户已确认已有功能验证通过；本轮未启动 GUI 复核新版关于窗口，未宣称执行全部自动测试。
- 用户自行提交 Git；必须等提交后另行通知发布，再核对提交源码与构建清单。未创建标签、Release 或上传附件，未自动提交或推送。

## 1.2.8 最新本地产物（2026-10-28，未发布）

- 用户授权重新编译；编译前确认程序未运行且目标 EXE 可独占访问。cargo build --release --locked --offline 成功（48.26 秒，8 项应用警告），git diff --check 通过。
- 当前产物：target/release/unity_yaml_viewer.exe；文件／产品版本 1.2.8；大小 25616896 字节（24.43 MiB）；SHA-256：7EC561EA90D5B8818FE0AF1EAC49684963F8C72C286CAE694B85193CEE0EE88D。
- 日志 target/v1.2.8-settings-version-release-build.log；已更新 target/v1.2.8-build-manifest.json 的 165 项输入摘要及产物哈希，后续发布以此最新产物核对，取代此前哈希。
- 按用户要求，本次显示细节不纳入功能、优化、关于摘要或升级说明。未启动 GUI；等待用户自行提交 Git 后另行通知发布，未创建标签／Release、未上传附件、未自动提交或推送。

## GitHub 正式发布（2026-10-28）

- 用户确认 Git 已提交并明确授权发布。发布前工作区干净，HEAD 与远端 master 一致：08dab4f64802efb75bbad390f7e50c8718e7dd6d；165 项构建输入及文件集合与该提交工作区完全一致，未重新编译。
- 北京时间 2026-10-28 18:45:39 正式发布；Release ID：426542862，标题仅 v1.2.8，非草稿／非预发布，latest API 核对为本版。地址：https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.2.8。
- 附件 UnityYAMLViewer-v1.2.8-windows-x64.exe，25,616,896 字节；GitHub asset.digest 与本地 SHA-256 一致：7EC561EA90D5B8818FE0AF1EAC49684963F8C72C286CAE694B85193CEE0EE88D。
- 使用已核验的 target/releases/v1.2.8/UnityYAMLViewer-v1.2.8-windows-x64.exe；先创建草稿上传并验证附件，再正式发布为最新版本。发布证据 target/v1.2.8-published.json，构建摘要 target/v1.2.8-build-manifest.json。
- 未修改历史 Release／标签／附件；发布后仅更新本地版本索引、发布说明与交接状态，未自动提交或推送。这一正式发布状态取代前述构建阶段的未发布／等待提交说明。设置版本号显示小改动仍未纳入新增、优化或升级说明。

## 独立文件图标本地构建（2026-10-29，未发布）

- 用户明确授权编译。CIM 查询不可用，改用 Get-Process 完成进程核对，并确认目标 EXE 可独占访问后开始构建。
- `cargo build --release --locked --offline` 成功（44.97 秒，8 项应用警告）；日志 `target/file-icons-release-build.log`。
- 产物 `target/release/unity_yaml_viewer.exe`，版本 1.2.8，25,552,384 字节；SHA-256 `2BE95963F8D14C27F438693B58FD087F676B1BF68E1FFD366AC0593A794046BC`。
- 以数据资源方式读取 EXE，确认主图标资源 1 与文件图标资源 2 均包含 13 个尺寸；核验记录 `target/file-icons-release-verification.json`。未启动程序、未写入当前系统文件关联，资源管理器实际图标及关于窗口仍待运行验证；未发布或上传附件。

## Windows 打开方式名称本地构建（2026-10-29，未发布）

- 用户确认显示名称为 Unity YAML Viewer，并明确授权编译；Get-Process 与目标 EXE 独占访问检查通过。
- cargo build --release --locked --offline 成功（37.73 秒，8 项应用警告），日志 target/file-association-name-release-build.log。
- 产物 target/release/unity_yaml_viewer.exe，大小 25552384 字节，文件版本 1.2.8；SHA-256：4580558E7E07F700A5425F52402535C54DBC8E248F96D3A470F5CF8F67CDBA5A。
- 已读取成品版本资源，FileDescription 与 ProductName 均确认为 Unity YAML Viewer；核验记录 target/file-association-name-release-verification.json。
- 未启动程序或更改当前系统文件关联；Windows 属性和打开方式界面的名称／缓存刷新仍待运行验证。未发布或上传附件。

## 启动更新关联登记本地构建（2026-10-29，未发布）

- 用户明确授权编译，Get-Process 及目标 EXE 独占访问检查通过。
- cargo build --release --locked --offline 成功（38.79 秒，8 项应用警告），日志 target/startup-file-association-release-build.log。
- 产物 target/release/unity_yaml_viewer.exe，25666560 字节，版本 1.2.8；SHA-256：5F8DF76248322D86B56B40FB515A4989F294339AE5594B0C6FB11DB9A3C93DAB。
- 成品 FileDescription 与 ProductName 均为 Unity YAML Viewer；核验记录 target/startup-file-association-release-verification.json。包含启动后台更新关联名称、路径及独立文件图标登记，未修改图标缩小算法。
- 未启动 GUI 或执行此 EXE 的登记逻辑，实际启动登记、权限错误提示、默认程序保持、名称及图标刷新和七语言界面待用户运行验证；未发布或上传附件。

## 多图切换与相邻帧缓存本地构建（2026-10-30，未发布）

- 用户明确授权编译，Get-Process 与目标 EXE 独占访问检查通过。
- cargo build --release --locked --offline 成功（40.76 秒，8 项应用警告）；日志 target/image-prefetch-release-build.log。
- 产物 target/release/unity_yaml_viewer.exe，版本 1.2.8，25731072 字节；SHA-256：B7F313E4CB01415365BB6656D8327DA900C6CF33D7C212DFBA4289779B5D59F6。
- 包含切图保留旧画面、相邻帧预加载与约 64 MiB 额外缓存。成品信息见 target/image-prefetch-release-verification.json。
- 未启动 GUI，实际连续切图流畅度、解码耗时／帧率、缓存预算边界、失败恢复及关于窗口七语言布局与折叠交互仍待运行验证；未发布或上传附件。

## 多图浏览运行验证反馈（2026-10-30）

- 用户在本轮本地构建后反馈“验证通过”，确认多图切换保留旧画面及相邻帧预加载／缓存的实际浏览效果通过验证。
- 此反馈不代表已测得具体解码耗时或帧率，也不扩展为缓存预算边界、异常文件恢复、超大图片或关于窗口七语言布局与折叠交互的全面验证。
- 对应构建：target/release/unity_yaml_viewer.exe，版本 1.2.8，SHA-256：B7F313E4CB01415365BB6656D8327DA900C6CF33D7C212DFBA4289779B5D59F6。未发布或上传附件。

### 本轮编译验证（2026-10-30）

- 用户明确授权后，已检查查看器进程和目标文件独占访问，确认无占用。
- 离线锁定依赖的 release 编译成功，耗时 38.24 秒，保留 8 条编译警告；日志：target/freeze-header-release-build.log。
- 产物：target/release/unity_yaml_viewer.exe，26770944 字节；SHA256：64477B7541D1A29A92F4C93C54A6446695E6863CB06A17BF5640693506780107。
- 本轮未运行测试或实际界面，冻结行列、滚动及工作表栏间距仍待界面验证；未发布。

### 冻结区域遮挡与分割线修复（2026-10-30，源码）

- 根据实际界面反馈，移除底层随滚动重复绘制的列标题，整个表格按可见视口裁剪，避免冻结表头上沿露出滚动内容。
- 在冻结区域遮罩上重绘列分割线和行号边界，保持与数据区列边界对齐；沿用主题线条颜色。
- 同步七语言关于摘要。仅完成代码检查；尚未编译此修复或运行界面验证。

### 转换预览文档另存为 PDF（2026-11-02，源码）

- Word（DOC/DOCX）、PowerPoint（PPT/PPTX）、ODT/ODP/RTF 等走转换预览的文档新增“另存为 PDF”，放在“在LibreOffice编辑”旁；原生 PDF 不显示，XLS/XLSX/ODS 继续使用独立数据表格，不新增 PDF 转换。
- 转换并加载成功后按钮可用，复用内存中的完整 PDF 字节，不重新转换、不按当前页或缩放截图；保存的是当前预览版本，外部修改后应先重载。
- 保存对话框默认原文件名加 .pdf，保留系统覆盖确认，要求 PDF 扩展名；后台写入，显示完成路径或失败原因，取消不写入。不覆盖原文档，不改变原文件路径或切换标签。
- 同步 README 中英文与关于摘要七语言。代码检查通过；未生成可执行程序，尚未验证实际保存对话框、取消、覆盖确认、失败提示及导出内容；关于窗口默认两版展示逻辑未改，运行核对待完成。

### 另存 PDF 与冻结区域修复编译（2026-11-02）

- 用户明确授权后检查查看器进程和目标独占访问，无占用；release 离线锁定依赖编译成功，耗时 41.29 秒，8 条警告。
- 产物：target/release/unity_yaml_viewer.exe，26878976 字节；SHA256：9AF808F914BB9CAEBA81C611B05F58F4AE474F26A4A68BE88BBDC404902EAEA8。日志：target/office-save-pdf-release-build.log。
- 包含转换预览文档另存完整 PDF、冻结区域分割线与裁剪修复；未执行运行测试，实际保存与冻结显示效果待验证，未发布。

### 列坐标与冻结首行之间的横线（2026-11-02，源码）

- 用户确认需要两行之间的横线：在顶部列字母／类型行与冻结数据表头之间补充横向分割线，跨越行号与数据列区域，使用主题现有表格线颜色；仅存在冻结数据行时增加此内部横线。
- 核对当前关于摘要七语言，已有“冻结区域分割线”说明覆盖此修复，不新增重复摘要或改变版本折叠逻辑。
- cargo check --tests --locked --offline 检查通过；未重新编译或运行界面验证，实际浅色／深色主题及滚动效果待验证。

### 表格顶部边线衔接（2026-11-02，源码）

- 移除表格顶部独立分隔控件及其下方额外空隙，将顶部横线直接绘制在固定列坐标行上沿，与列竖线共用视口坐标和主题线条样式，避免两者之间断开。
- 保留工具栏与表格的必要间隔，以及列坐标行与冻结数据表头之间的横线。关于摘要七语言已有冻结区域分割线修复说明，覆盖本次调整。
- cargo check --tests --locked --offline 通过；未编译可执行程序，实际界面、主题及滚动显示待验证。

### 表格底部边线（2026-11-02，源码）

- 在表格可视区域底部补充横线，位于水平滚动条／工作表栏上方，与顶部边线及列分割线使用相同主题样式；边线随视口固定，不跟随数据滚动。
- 关于摘要七语言已有表格分割线修复说明，覆盖本次补充。cargo check --tests --locked --offline 通过；未编译可执行程序，实际界面和滚动条出现／隐藏时的显示仍待验证。

### 表格横线调整编译（2026-11-02）

- 用户明确授权后检查查看器进程及目标独占访问，无占用；release 离线锁定依赖编译成功，耗时 38.31 秒，8 条警告。
- 包含列坐标与冻结数据行之间的横线、顶部边线衔接及可视区域底部边线。产物：target/release/unity_yaml_viewer.exe，26815488 字节；SHA256：4C26499E2122F294372684307593BE97DC44DAA06914B77E6744DBF944575D59。日志：target/table-borders-release-build.log。
- 本轮未运行界面验证，实际主题、滚动和边线显示效果待验证；未发布。

- 版本整理验证：cargo check --tests --locked --offline 通过；1517 条非空翻译记录均为七列且键唯一，默认两版顺序、1.2.7 起折叠、重开折叠状态静态核对通过；1.2.8 摘要与发布标签一致，发布说明未修改。实际界面待验证，未编译 1.2.9。

## 发布后本地编译：内置组件安装器与预览修复（2026-11-03）

- 用户明确授权编译，构建前检查查看器进程与目标 EXE 独占访问，无占用；cargo build --release --locked --offline 成功，耗时 50.15 秒，8 条警告。
- 包含手型仅限制预览选中、文本字号及间距、Markdown 排版与图片缓存、固定行号分割线及从 1 编号，以及移除 Scoop／WinGet 后的内置组件下载器。FFmpeg 按需下载 Shared 版，不随本程序打包；LibreOffice 直接下载安装包。
- 产物：target/release/unity_yaml_viewer.exe，27,332,608 字节，文件／产品版本均为 1.2.9；SHA256：DA1AB7FE5D4CA34AA9669E448A12F91A6E4D796F2B3DA959807F949128693E48。
- 日志：target/v1.2.9-internal-installer-release-build.log。本轮未启动 GUI、未执行测试程序、未下载或安装第三方组件；相关端到端与界面效果仍待验证。
- 仅本地编译；未提交、推送、创建标签或发布，不替换 GitHub 已发布附件。此前源码阶段的“未生成可执行程序”是当时状态，本节为最新构建记录。

## 本地编译：Shared 完整解压与用户 PATH（2026-11-03）

- 用户明确授权编译；首次检查发现目标 EXE 被占用，未开始编译。用户再次确认后重新检查，目标独占访问通过且未发现查看器进程，才执行 cargo build --release --locked --offline。
- release 编译成功，耗时 48.79 秒，8 条警告。包含 FFmpeg GPL Shared 完整解压、PATH 优先自动检测、加入用户 PATH 按钮，以及移除两处冗余提示。
- 产物 target/release/unity_yaml_viewer.exe，27,441,664 字节，文件／产品版本均为 1.2.9；SHA256：51DBF135076D443D9ABD724F7E567C67FF07BFA39D12FC618E41C92502EF5F86。
- 构建日志 target/v1.2.9-user-path-release-build.log；未启动界面、未执行测试程序、未下载或安装组件、未修改实际 PATH。完整解压、PATH 按钮及七语言界面仍待运行验证。
- 仅本地编译，未发布、未提交／推送或替换 GitHub 附件。本节产物为当前本地产物，取代此前本地构建哈希记录的当前状态。

## 本地编译：第三方组件标题行布局（2026-11-03）

- 用户明确授权，构建前未发现查看器进程且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 40.24 秒，8 条警告。
- 包含组件名称后的绿色／红色安装状态框、同排重新检测、PDFium“移除”与 LibreOffice“选择程序…”；保留任务期间禁用、PDFium 使用中保护及原路径验证行为。
- 产物 target/release/unity_yaml_viewer.exe，27,373,568 字节，文件／产品版本均为 1.2.9；SHA256：9247ED715CABA0631DCD4A03B28DC9BEFC530FA18ED2A38ECD2BD6A488CE0C94。
- 构建日志 target/v1.2.9-component-header-release-build.log；本轮未启动 GUI、未执行测试程序、未下载安装组件，布局与七语言显示仍待运行验证。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 本地编译：Markdown 居中、SVG 徽章与安装入口（2026-11-03）

- 用户明确授权，构建前未发现查看器进程且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 45.80 秒；主程序未报告编译警告，第三方 Spine 依赖仍有既有警告。
- 包含 8 条主程序警告清理、Markdown 居中块与 SVG 徽章文字／图片链接修复，以及未安装组件后同排绿色描边安装入口。
- 产物 target/release/unity_yaml_viewer.exe，28,384,256 字节，文件／产品版本均为 1.2.9；SHA256：D699A7EA5CA7CE2B777BAED6733B1A742998413793A3D67F8D176D2F34C62AAC。
- 日志 target/v1.2.9-markdown-badges-release-build.log；本轮未启动 GUI、未执行测试程序或下载安装组件，实际排版、徽章与按钮布局仍待运行验证。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 本地编译：RTF 直读与 Markdown 样式（2026-11-03）

- 用户明确授权，构建前未发现查看器进程，目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 35.97 秒。主程序无编译警告，第三方 Spine 依赖仍有既有警告。
- 包含 Windows RichEdit RTF 直读、缩放与复制全文，以及 Markdown 蓝色下划线链接、多徽章绘制、标题分割线和代码块配色。
- 产物 target/release/unity_yaml_viewer.exe，28,423,680 字节，文件／产品版本均为 1.2.9；SHA256：D72DC69AA1DF97C910AB03D5FB0C4B629A6496D6047734AB0218534B5043AC9A。
- 日志 target/v1.2.9-rtf-direct-release-build.log；本轮未启动 GUI、未执行测试程序，RTF 分页与格式、Markdown 样式及七语言实际布局待运行验证。RTF 示例 target/samples/sample-1.rtf 可供人工检查。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 本地编译：行内链接、原复制图标与 PowerShell 着色（2026-11-03）

- 用户明确授权，构建前未发现查看器进程且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 40.32 秒。主程序无编译警告，第三方 Spine 依赖仍有既有警告。
- 包含 Markdown 列表／段落行内链接回调修复、代码块原复制图标与选区复制恢复、PowerShell 命令／参数等专用着色。标题规则本轮仅检查与说明，未改为真实粗体字体，不宣称已调整标题字重。
- 产物 target/release/unity_yaml_viewer.exe，28,475,904 字节，文件／产品版本均为 1.2.9；SHA256：41CE112AAA609E86FC924481AB53F6DC10FB05AC1A5242A9384746BD3101B43F。
- 日志 target/v1.2.9-markdown-inline-release-build.log；未启动 GUI、未执行测试程序。链接／复制／代码配色实际显示待验证；用户此前截图已确认 RTF 格式样例的常用字符格式、对齐与中文显示，不能扩展为复杂 RTF 全面验证。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 本地编译：下载匹配诊断与代码框内距（2026-11-03）

- 用户明确授权，构建前未发现查看器进程且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 40.30 秒。主程序无编译警告，第三方 Spine 依赖仍有既有警告。
- 包含 FFmpeg Windows x64 GPL Shared 主线快照的新旧包名匹配、失败详情中的请求／下载地址，以及 Markdown 单层代码边框与至少 4 像素内距、复制图标右侧预留。
- 产物 target/release/unity_yaml_viewer.exe，28,494,336 字节，文件／产品版本均为 1.2.9；SHA256：06C53F406395853A36F1DF40908CEDC46B42A9CBF39FB64021DFD1203D6FE377。
- 日志 target/v1.2.9-download-match-code-frame-release-build.log；本轮未启动 GUI、未下载或安装组件、未执行测试程序，实际下载与代码框效果仍待运行验证。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 本地编译：图片信息与可撤销 PNG 清理（2026-11-03）

- 用户明确授权，构建前未发现查看器进程且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 43.78 秒。主程序无编译警告，第三方 Spine 依赖仍有既有警告。
- 包含图片源信息、PPI／iTXt大小、完全不透明 PNG 才显示去 Alpha、超过10 KiB的iTXt清理、自动刷新及会话撤销／重做，以及代码框内距位置纠正。
- 产物 target/release/unity_yaml_viewer.exe，28,510,208 字节，文件／产品版本均为 1.2.9；SHA256：EFDED75965B656B9B28CD210D9C7CA9E08BBB6E1AE965ACE14F6FABE0BBF3150。
- 日志 target/v1.2.9-image-info-release-build.log；本轮未启动 GUI、未实际写入图片或执行测试程序，信息显示、按钮显隐、原位替换／自动刷新／撤销和代码框效果待运行验证。
- 仅本地编译，未发布、未提交／推送、未替换 GitHub 附件；本节为当前本地产物信息。

## 1.3.0 版本整理与发布准备构建（2026-11-03）

- 以 v1.2.9 正式发布章节末尾为边界，将其后开发与本地构建记录原样迁入本版本；1.2.9 正式发布记录、历史 Release 文档及附件信息未改写。历史1.2.9哈希仍对应当时真实产物。
- Cargo.toml／Cargo.lock 升为1.3.0；更新版本索引与CHANGELOG、新增docs/releases/1.3.0.md准备稿。README保持用户能力说明，不添加版本流程内容。
- 关于摘要重新归纳为6条并补齐七语言，默认显示1.3.0／1.2.9；1.2.9摘要与v1.2.9标签一致，1.2.8及更早移入原历史折叠区，保留旧翻译和重开折叠逻辑。静态核对通过，实际界面待验证。
- cargo check --tests --locked --offline、git diff --check、翻译七列与键唯一性、版本顺序及迁移边界核对通过；测试程序未生成或执行。本轮未启动GUI或操作图片／安装组件／用户PATH。
- 用户明确授权构建，构建前未发现查看器进程，目标EXE独占访问通过。cargo build --release --locked --offline成功，耗时42.10秒，主程序无编译警告；第三方Spine仍有既有警告。日志target/v1.3.0-release-build.log。
- 产物target/release/unity_yaml_viewer.exe，文件／产品版本均1.3.0，28,509,184字节；SHA256：BC6033DAABF8D2A8480DD9063A6921E196D02F6E96120C8C1000CD07E5B5603A。
- 发布候选副本target/releases/v1.3.0/UnityYAMLViewer-v1.3.0-windows-x64.exe与原产物哈希一致。核验target/v1.3.0-release-verification.json；203项主要构建输入清单target/v1.3.0-build-manifest.json，构建后哈希再次核对一致。
- 等待用户提交Git；本轮未stage、提交、推送、创建标签、Release或上传附件，尚未发布。用户告知提交后，应重新核对提交内容与构建输入，不能仅因版本升级／构建成功宣称已发布。正式Release标题仅为v1.3.0。

## 发布候选更新：DPI 简约显示与 Windows 补充读取（2026-11-03）

- 用户明确授权编译并纳入发布候选；构建前未发现查看器进程，默认目标EXE与发布候选副本独占访问均通过。cargo build --release --locked --offline成功，耗时40.31秒，主程序无编译警告，Spine依赖仍有既有警告。
- 纳入Windows WIC补充读取DPI、来源悬停说明和“dpi: 72”简约显示；两轴不同时保留两个数值，不将Windows默认值写回图片。
- 当前产物target/release/unity_yaml_viewer.exe及候选target/releases/v1.3.0/UnityYAMLViewer-v1.3.0-windows-x64.exe，文件／产品版本均1.3.0，28,507,136字节，SHA256均为DB46D382054BB80BB2EEA3AE5577D815DCD0ECFC991940EC96C4D185E09CBDA0。
- 更新target/v1.3.0-build-manifest.json（204项主要构建输入）及target/v1.3.0-release-verification.json，构建后输入哈希核对一致。日志target/v1.3.0-dpi-release-build.log。本候选替代此前本地候选，不改写此前构建史实。
- 未启动GUI或执行测试程序，实际DPI界面及其他端到端验证边界不变。未stage、提交／推送、创建标签、Release或上传附件；等待用户提交Git后告知再核对发布。

## GitHub 正式发布（2026-11-03）

- 用户提交Git后明确授权发布。发布前工作区干净，提交fdeb939f546e36844738bb5ed028ff048dc18bcb与GitHub master一致。204项主要构建输入与构建时工作文件哈希完全一致；其中51项与提交字节完全一致、153项仅Git CRLF/LF规范化差异，源码内容无差异。审计target/v1.3.0-publish-audit.json，未重新编译。
- 北京时间2026-11-03 22:20:43正式发布[v1.3.0](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.0)，Release ID 432793952，标题仅v1.3.0，非草稿／非预发布，latest API确认最新正式版。标签指向上述提交。
- 先建草稿，再上传核验附件，随后正式发布。附件UnityYAMLViewer-v1.3.0-windows-x64.exe，28,507,136字节，GitHub上传状态uploaded，SHA256与本地一致：DB46D382054BB80BB2EEA3AE5577D815DCD0ECFC991940EC96C4D185E09CBDA0。
- 发布证据target/v1.3.0-published.json；附件证据target/v1.3.0-uploaded-asset.json。更新发布说明、版本索引与CHANGELOG的发布状态，不修改其他Release或附件。
- 保留已确认的验证边界，未补做完整GUI回归或执行测试程序。发布后文档更新未自动stage、提交或推送，等待用户自行纳入后续提交。

## 1.3.1 发布准备（2026-11-04）

- 已准备 docs/releases/1.3.1.md。Windows x64 候选 target/releases/v1.3.1/UnityYAMLViewer-v1.3.1-windows-x64.exe 的 SHA-256 与已核验的构建产物一致。
- 等待用户提交并推送，发布前核对目标提交与构建内容。计划使用标签及标题 v1.3.1。

## v1.3.1 正式发布（2026-11-04）

- 北京时间 2026-11-04 11:08:20发布 [v1.3.1](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.1)，Release ID 433466401。标签指向远端 master 提交 120d97496a6ad1d0689319330af67eb311b77d18，已核验为最新正式版。
- Windows x64 附件 ID 708029993，28,517,376 字节，GitHub SHA-256 与本地一致。核验记录见 target/v1.3.1-published.json 和 target/v1.3.1-uploaded-asset.json。

## v1.3.2 Windows 本地构建（2026-11-04）

- Windows Release 构建通过（47.92 秒）。文件版本 1.3.2，产品版本 1.3.2，大小 29361664 字节，SHA-256 为 D7B75D44E9A8D9DE065F4E65D647119DAB311B34E4116FAE7C665B749D5D139A。 产物位于 `target/release/unity_yaml_viewer.exe`。编译前未发现进程，目标文件可独占访问。打包、界面运行验证及发布待完成。

## v1.3.2 HLS 修复后的 Windows 本地构建（2026-11-04）

- Windows Release 构建通过（43.27 秒）。文件版本 1.3.2，产品版本 1.3.2，大小 29360128 字节，SHA-256 为 FE57C763B2430AE4DD55C89076C5A5A244E2B857102B2A68D7BD26E3C90038F3。 产物位于 `target/release/unity_yaml_viewer.exe`。编译前未发现进程，目标文件可独占访问。应用内验证、打包和发布待完成。

## v1.3.2 HLS 后台缓存 Windows 本地构建（2026-11-04）

- Windows Release 构建通过（41.90 秒）。文件版本 1.3.2，产品版本 1.3.2，大小 29359104 字节，SHA-256 为 DDE45ACC1D6B3AE7534AA9F3AFEA00A0753A9781A96EB84A686F0A2BBA85E60F。 产物位于 `target/release/unity_yaml_viewer.exe`。编译前未发现进程，目标文件可独占访问。应用内验证、打包和发布待完成。

## v1.3.2 HLS 音画同步修正后的 Windows 本地构建（2026-11-04）

- Windows Release 构建通过（40.04 秒）。文件版本 1.3.2，产品版本 1.3.2，大小 29324800 字节，SHA-256 为 C33D69C0A466EDC29EF40FFEE2EE464220783EE7D08E86A1271123218F5F600A。 产物位于 `target/release/unity_yaml_viewer.exe`。编译前未发现进程，目标文件可独占访问。应用内验证、打包和发布待完成。

## v1.3.2 正式发布（2026-11-04）

- 北京时间 2026-11-04 20:48:45 发布 [v1.3.2](https://github.com/EkyoZZ/UnityYAMLViewer/releases/tag/v1.3.2)，Release ID 433912470。标签指向 fa5a6bf9a92247df9dc542ea16c4617b34ab709c，已核验为最新正式版。
- Windows x64 附件 UnityYAMLViewer-v1.3.2-windows-x64.exe，29324800 字节。上传状态及 GitHub 附件校验值已核对，sha256:c33d69c0a466edc29ef40ffee2ee464220783ee7d08e86a1271123218f5f600a。用户已确认发布前验证通过，抖音视频提取不支持。


## 2026-11-11 实时原文字幕源码交接

- SenseVoice 五语言共用模型列出与安装入口；Zipformer 首批中英双语 INT8 模型按需下载。视频页 / 画面右键切换实时识别引擎；复用播放 PCM，分句 / 流式覆盖显示，识别队列过载停止字幕而不阻塞播放。源码详见 src/live_subtitle.rs、src/live_subtitle.py、src/auto_subtitle.* 与 src/media_preview.rs。
- python -B tests/test_live_subtitle_worker.py：6 项通过；模型版本 / 路径、分包、尾段及端点使用模拟对象验证，不代表真实模型已加载。Rust 两项回归用例仅 cargo check --tests --locked --offline 编译检查通过（target/live-subtitle-check.log），未运行测试或构建新 EXE。Python 语法、七语言文案 / 当前关于摘要、默认两版历史索引及 git diff --check 通过。
- 当前实时输出仅原文，不保存、不实时翻译；Zipformer 本轮模型仅中英。实际下载 / CPU 速度 / 多语言识别质量、YouTube 和频道、暂停跳转换引擎、叠加样式、关于历史交互均待运行验证。未更改发布状态或附件；既有 EXE 不包含本轮修改。


## 2026-11-11 字幕缓存 / 回放与投影补充

- 实时识别现在自动写入应用缓存 live_subtitles 下的独立 JSONL / SRT，会话元数据与草稿 / 最终结果分明；提供定位和 SRT 导出。相同媒体身份、轨道、语言、引擎 / 模型版本可复用已确认片段并跳过覆盖 PCM 包。HLS 仅在正时长且有 ENDLIST 的媒体清单校验后跨会话复用；失败或未知直播不复用。未实现自动比较 / 合并 / 校正，不缓存音频、不覆盖原字幕。
- 字幕投影增加 -12..12 的 X/Y 独立偏移，兼容旧单值，原字幕不改写；实时覆盖样式仍独立。具体限制、容量及验证见 docs/versions/1.3.6.md 和双语指南。
- Python 8 项通过，Rust 新增用例仅编译检查通过：target/live-subtitle-cache-check.log；未生成新 EXE。真实模型、HLS 回放 / 重开、缓存清理与导出、投影四方向及关于窗口交互仍待运行验证。


## 2026-11-11 实时字幕 / 缓存与投影 Release 构建
- 2026-11-11，用户明确授权编译实时字幕 / 缓存与投影版：构建前确认 unity_yaml_viewer 未运行，目标 EXE 可独占访问。cargo build --release --locked --offline 成功（1 分 02 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.3.5，31,592,960 字节，SHA-256：D1A0CA3A3CE34C08C29FD6626AABE407C4EBAF40EC2DFE410AAC597D74BABBCC。构建日志 target/live-subtitle-release-build.log。未自动启动、未发布或更新附件；真实 SenseVoice / Zipformer、实时字幕缓存与 HLS 回放复用、投影 X/Y、七语言及关于窗口交互仍待运行验证。此前“未生成 EXE”描述为当时状态。


## 2026-11-11 点播 SenseVoice 提前识别与 OPUS-MT 安装补充

- 点播提前识别默认启用、SenseVoice 专用，独立 FFmpeg 音轨供给最大约 20 秒领先窗口，确认句仅按媒体时钟显示；暂停可补足窗口，跳转换轨 / 引擎或关闭取消旧任务。HLS 仅 ENDLIST 校验通过后启用，增加网络音轨读取开销，可在实时字幕菜单关闭。首次加载 / 未缓存跳转仍可能缺字幕，未实现预缓冲阻塞播放。
- OPUS-MT 安装区分运行库和模型阶段，-X utf8 修正日志编码；最多 3 次下载尝试，断点续传 / 200 重下、固定大小 SHA-256 校验、完整 ZIP 留存以复用转换重试。用户截图 10060 是连接超时，未宣称已复现源站或解决网络环境。
- Python 下载测试 6 项、字幕测试 10 项通过；Rust 媒体时钟窗口与协议用例只编译未运行。cargo check --tests --locked --offline 通过，target/subtitle-lookahead-check.log。未编译新 EXE；真实点播 / HLS / 下载转换及界面仍待验证。


## 2026-11-11 字幕边距与可用文字宽度

实时层改用可见画面减去左右边距及文字内边距排版，底边距沿用样式；长词可折行，窄窗口安全限制。临时 ASS 统一每句边距并覆盖 q2 / pos / move / org / an 等定位设置，以播放器全局边距为准；原文件不修改。新增两项回归用例只编译检查，target/subtitle-margins-check.log；首尾字符实际显示与截图复现、全屏 / 缩放待 GUI 验证，未构建新 EXE。


## 2026-11-11 字幕边距 / 提前识别 / 安装修正 Release

- 2026-11-11，用户再次明确授权后，编译前检查 unity_yaml_viewer 无运行进程，目标 EXE 可独占访问；cargo build --release --locked --offline 成功（53.26 秒），保留 spine38 既有 158 项警告。此构建包含字幕两排 / 断句、左右边距修正、SenseVoice 点播提前识别与 OPUS-MT 安装续传 / 重试和日志 UTF-8。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.3.5，31,698,944 字节，SHA-256：BAEE96F5B5EE61751DB4170E4FD0A02BEFD253170AC2723E8978F01277AFCA52。日志 target/subtitle-margins-release-build.log。未自动启动、未发布或更新附件；实际首字裁切 / 换行、提前识别同步、真实网络安装及关于窗口交互仍待运行验证。此前未生成 EXE 的表述为各次源码修改时点状态。


## 2026-11-11 M2M100 INT8 双模型字幕翻译

保留 OPUS-MT，新增 M2M100 INT8 社区固定版约 494 MB 下载、约 1 GB 持久模型加下载包（另加共用运行库，建议 2 GiB 磁盘）。第三方支持显示估算 1–3 GiB RAM 和可用时的上次模型进程峰值工作集；独立 Python 进程任务后退出，模型文件保留。中英日韩 12 个直接方向，源 / 目标可选，模型文件名隔离，双语 M2M100 原文在上译文在下。仅文件翻译，实时翻译暂未实现。M2M100 6 项 / OPUS 下载 6 项模拟测试通过，Rust 命名回归仅编译检查通过，target/m2m100-check.log。未下载大模型、未实测质量 / 内存 / 加载兼容性，未构建新 EXE。详见当前版本记录。


## 2026-11-11 字幕样式后续反馈修正

投影 X/Y 改输入框外标签，增加顶部 / 底部与动态边距，旧配置默认底部。长中文以 Windows 字体测量 + 脚本宽度和边距显式插入硬换行，替代仅 q0 自动换行；原文 / 格式标签保留，内联字号和缩放服从播放器设置。全局样式同步重新生成临时 ASS，避免旧边距残留。系统 FFmpeg 两行中文独立夹具验证底边距20/40及顶部显示，证据 target/subtitle-style-render-verification.json；不是应用运行或原视频验证。Rust 新用例仅编译检查通过，target/subtitle-style-layout-check.log；未生成新 EXE，实际字体 / 大字号 / GUI 待验证。


## 2026-11-11 原生 ASS / 导出 / 样式窗口聚焦

统一样式默认保留，新增原生 ASS 模式（UTF-8 外部 ASS 临时原文复制、无播放器重写 / force_style）；当前文字字幕菜单可导出 SRT / ASS，统一 ASS 固化样式，原生保留内容。生成 / 翻译默认 SRT 不变，实时缓存仍 SRT；特效制作预设与标签编辑器未实现。重复呼出同一个样式窗口通过 native_window::wake 前置，保留未应用草稿和搜索。Rust 新回归用例仅编译检查通过：target/ass-mode-export-check.log。实际操作系统聚焦、特效与字体 / 导出往返待实机验证；未构建新 EXE。


## 2026-11-11 ASS 模式 / 导出版 Release

- 2026-11-11，用户授权编译 ASS 模式 / 导出版：构建前确认 unity_yaml_viewer 无运行进程且目标 EXE 可独占访问；cargo build --release --locked --offline 成功（1 分 14 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.3.5，31,817,728 字节，SHA-256：77E42D196DCE9DBA27704BD3C343895A9414C1E3CC280F1461B84FB77BDDF3DB。日志 target/ass-mode-release-build.log。包含 M2M100 / OPUS-MT 双翻译模型、字幕样式顶底位置 / 显式换行、原生 ASS / 统一样式、SRT / ASS 导出和样式窗口重复呼出唤醒。本次未自动启动程序、未发布或更新附件；真实模型安装 / 内存与翻译、原视频边距 / 换行、ASS 特效 / 导出及窗口 OS 聚焦仍待实机验证。此前各次“未生成新 EXE”为当时源码修改状态。


## 2026-11-11 字幕换行过窄 / 边距单位

边距20明确为ASS脚本单位，非20%或20屏幕像素。统一模式脚本宽度匹配实际视频比例，默认384×288的4:3脚本在16:9视频修正为512×288；原生ASS不变。GDI字宽改正高度cell口径，使用粗体测量加3%余量替代负高度EM加15%。当前机器四中文字符测量由1340降为992（高分辨率单位），Rust新增比例回归仅编译，target/subtitle-width-units-check.log；用户视频实际宽度 / 完整性仍待验证，未编译EXE。


## 2026-11-11 字幕宽度 / 单位修正版 Release

- 2026-11-11，用户授权后编译字幕宽度 / 单位修正版：构建前确认程序无运行进程，目标 EXE 独占访问通过；cargo build --release --locked --offline 成功（1 分 00 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.3.5，31,740,928 字节，SHA-256：C288EF00BF1228716E453565E476080132C24D83DEF23B02085FA1E63FCE3DDE。日志 target/subtitle-width-release-build.log。包含脚本比例匹配视频、GDI 字宽口径修正及边距脚本单位说明，保留此前 ASS 模式 / 导出与双翻译模型。未自动启动、未发布或更新附件；实际原视频行宽 / 首尾字完整性与窗口交互仍待用户运行验证。


## 2026-11-11 实际 ASS 边距列索引根因与草稿预览

已获用户窗口授权实机确认旧构建“应用”可提交 / 位置可变；发现边距真正根因为 ASS Dialogue 列错一位：应写5/6/7，原写6/7/8，垂直边距写到Effect。已修正并新增精确列断言。用户实际字幕副本 FFmpeg 91.606秒验证底部 y413→278（垂直100），证据 target/subtitle-margin-columns-verification.json；未运行新构建。恢复GUI底部 / 水平10 / 垂直100和原暂停位置，播放器仍开着。

源码新增180ms文字字幕草稿预览，应用保存 / 取消关闭回滚、播放页模式和未保存标识，导出用预览快照；实时层不读取未保存草稿。Rust新回归仅编译检查，target/subtitle-margin-columns-check.log，未构建EXE。后续若授权编译必须重新检查程序占用，不得自动关闭当前程序。


## 2026-11-11 边距列修复 / 样式预览版 Release

- 2026-11-11，用户授权编译边距列修复 / 样式预览版：构建前确认播放器未运行且目标 EXE 可独占访问；cargo build --release --locked --offline 成功（59.16 秒），仅保留 spine38 既有 158 项警告。产物 target/release/unity_yaml_viewer.exe，文件 / 产品版本 1.3.5，31,808,000 字节，SHA-256：CC6C99A66E8C4495EE58C8935FDF877420432E3106A0DA9B97BC1414BFDC0C62。日志 target/subtitle-margin-columns-release-build.log。包含 ASS Dialogue 边距字段5/6/7修正及当前文字字幕草稿预览 / 应用保存 / 取消恢复。未自动启动、未发布或更新附件；当前新构建的实际边距及预览交互仍待验证，前述用户字幕副本 FFmpeg 验证与旧构建实机排查不等于此新构建验证。

## 2026-11-11 实时字幕翻译整合版 Release

- 2026-11-11，用户授权编译实时字幕翻译整合版：编译前确认unity_yaml_viewer无运行进程且目标EXE独占访问通过；cargo build --release --locked --offline成功（1分04秒）。产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31975936字节，SHA-256：5764646EC99C5250AE1B8024A76070600CEE4C9D506EABFDAE2199DBDA0183AB。日志target/live-translation-release-build.log。保留spine38既有158项警告，另有VideoSettings::configure未使用警告。包含字幕切换保留画面、播放及生成长句拆段、ASS内嵌／烧录、Zipformer四候选束搜索、移除OPUS-MT并使用M2M专用运行库、实时翻译开关／源字幕显示及行末状态。未自动启动或发布；新构建GUI、实际识别重复改善及持续翻译性能仍待验证。之前系统FFmpeg和模型进程验证不等于完整新应用运行验证。


## 2026-11-11 共享 GPU 运行库与低 CPU 偏好

本机已授权安装到 C:/Users/ekyo/AppData/Roaming/UnityYAMLViewer-components/ai-gpu-runtime，三包版本见当前版本记录，ready经4个DLL加载及RTX4060真实M2M GPU推理验证后写入。组件目录ai-runtime.json默认prefer_gpu=true、cpu_threads=2。源码新增第三方统一设置及安装／校验入口，模型任务启动时读取；未构建新EXE。旧运行EXE不会自动使用新配置／DLL搜索路径，需用户后续明确授权编译且先检查占用。当前sherpa-onnx识别包无CUDA provider，继续CPU，不得把补齐库宣称为识别GPU可用。基准／验证证据见target/shared-ai-gpu-verification.json与target/live-translation-cpu-benchmark.json。新增Small开关及隐藏逐句翻译提示也尚未构建。


- 2026-11-11，用户明确授权编译共享GPU／Small控件版：构建前确认unity_yaml_viewer未运行且目标EXE独占访问通过；cargo build --release --locked --offline成功（53.09秒）。产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31859712字节，SHA-256：0856F330E75985E7E9764E6FF9B657513EE25398F61BBE1D0C671F1D9858B302。日志target/shared-ai-runtime-release-build.log。包含第三方统一GPU优先／CPU线程设置、GPU动态库补齐入口、M2M100实际预热验证后GPU推理及CPU回退、低后台CPU优先级、Small开关与按文本收窄的语言框、隐藏逐句翻译状态。当前本机共享CUDA库已装并单独验证M2M GPU成功；sherpa识别组件仍缺CUDA provider，保持CPU。编译保留第三方既有警告及VideoSettings::configure未使用警告。未自动启动或发布；新构建界面、长时间实时负载和播放流畅度待运行验证。


- 2026-11-11，用户明确授权编译独立原文／译文与GPU修复版：构建前确认播放器未运行且目标EXE独占访问通过；cargo build --release --locked --offline成功（48.82秒）。产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31902208字节，SHA-256：E00D7D803B11E93E74D5A5B03258B04D1E8FF96B372BE23D16733AAB05B9156E。日志target/live-subtitle-lanes-release-build.log。包含原文／译文独立更新、译文底部与双语原文上移、GPU／CPU实际后端标记、GPU库先校验及暂存修复、播放输入解码4线程及滤镜2线程。保留第三方既有警告及VideoSettings::configure未使用警告。未自动启动或发布，实际持续CPU占用与新显示布局待验证；用户继续报告旧运行体验关闭程序CPU4%～7%、开启SenseVoice字幕与翻译约74%，仍未验证纯视频播放基线，不得声称高CPU问题已解决。


## 2026-11-11 YUV 视频接入共享渲染系统（源码）

已新增src/render_system/video.rs并复用render_system::Gpu的上下文／回调／销毁，常规SDR播放管道使用Arc<YUV420 Frame>并绕过ColorImage，兼容RGBA留给HDR／透明／编辑等场景。cargo check --tests通过，实际WGL离屏GLSL与FFmpeg平面验证通过，详见当前版本记录。未授权新一轮EXE构建，现有target/release仍是E00D7D803B11E93E74D5A5B03258B04D1E8FF96B372BE23D16733AAB05B9156E版，不包含尺寸优化和YUV渲染改造。后续编译须用户明确授权且先检查程序占用。完整应用CPU改善仍未验证，不能宣称实现零拷贝。


- 2026-11-11，用户授权编译期间进一步澄清图片100%显示问题，同轮纳入物理像素倍率修正后重新检查播放器无运行进程及目标EXE独占访问，通过后cargo build --release --locked --offline成功（47.93秒）。最终产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31852032字节，SHA-256：1CA3FAC41C4EC1900B8332320395DC4E44B7244757F2D1DFAD92D5D3C2156E31。日志target/yuv-image-physical-zoom-release-build.log；此前51.61秒首轮日志target/yuv-image-sampling-release-build.log对应未含物理100%修正的中间产物，已被最终构建替换。包含YUV播放共享Glow渲染、按窗口限制预览帧、图片平滑／像素模式及100%物理一对一修正。保留第三方既有警告及VideoSettings::configure未使用警告。未自动启动、未发布；用户原图100%实际对齐、七语言UI、视频CPU占用等仍待运行验证。Rust物理DPI几何用例仅编译检查（target/image-physical-zoom-check.log），不宣称已实机解决全部锯齿。


- 2026-11-11，用户明确授权编译YUV画质与紧贴字幕版：编译前确认播放器未运行且目标EXE独占访问通过；cargo build --release --locked --offline成功（47.97秒）。产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31847936字节，SHA-256：D9A8B187B4F1D2E853009D363A4954197A361386943968ACDC4C97C3317CDDAD。日志target/yuv-quality-subtitle-stack-release-build.log。包含YUV Lanczos缩小／色度位置／上传状态及稳定抗色带处理、实时双语按实际行高紧贴堆叠、译文多行推高原文及背景50%不透明度。保留第三方既有警告及VideoSettings::configure未使用警告。未自动启动或发布；用户频道清晰度／淡横线改善、持续CPU占用及实际字幕布局仍待验证，离屏GLSL验证不等于新应用端到端验证。


- 2026-11-11，用户明确授权编译自动语种／点播提前翻译／编辑GPU整合版：编译前确认播放器未运行且目标EXE独占访问通过；cargo build --release --locked --offline成功（48.48秒）。产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.5，31920128字节，SHA-256：954187ECC20C444ED5976E6F0F51C1ED127C7FBA9F1879F47C39FC0BAEDEADD4。日志target/auto-language-editor-gpu-release-build.log。包含CPU／GPU识别引擎独立安装与选择、全屏翻译独立更新、编辑及透明RGBA GPU预览、播放终点取帧边界修复、实时SenseVoice默认自动语种与lang元数据贯通、点播未来确认字幕有界提前翻译及防提前显示。保留第三方既有警告及VideoSettings::configure未使用警告。未自动启动或发布；新应用GPU识别包实际安装／推理、全屏、末帧、自动日译中、预翻译和编辑交互仍待运行验证，已有模型／FFmpeg／离屏GL测试不等于完整新应用验证。

## 2026-11-12 字幕编辑与提取源码交接（未重建 EXE）

- 新增 src/subtitle_document.rs、src/media_preview/subtitle_editor.rs：独立 ASS／SRT 编辑窗口、源文档内存草稿、视频／所选音轨波形／时间轴／列表／样式编辑、历史、备份与外部冲突检测、另存互转、不可变草稿烧录交接。subtitle_player 同一菜单新增默认 ASS 原轨提取，选中内嵌 ASS／SSA 使用 copy 保留特效，成功挂载并进入编辑。
- VideoSettings 以 serde skip 的 Render 引用持有烧录快照，克隆进入现有导出任务；现有视频工程附件逻辑负责嵌入。未替换既有媒体导出流程，不影响本轮之前已有的大量未提交源码变更。
- cargo check --tests --locked --offline 通过，日志 target/subtitle-editor-check.log；八项新增 Rust 模型用例仅编译。node tests/verify_subtitle_formats.mjs 实际通过，FFmpeg 双字幕轨／ASS 特效保留／绘图转换／字幕像素证据在 target/subtitle-editor-verification/。文案七语言和关于索引静态检查通过；git diff --check 通过。
- 完整应用界面／保存／时间轴／独立播放及烧录交接、项目恢复仍待新构建运行验证。图像字幕、字体附件提取、高级 ASS 可视化暂不支持。具体范围见 docs/versions/1.3.6.md 和 docs/user-guide.md。本轮没有生成可执行程序、运行 Rust 测试、提交或发布；当前 EXE 不含这些源码修改。

## 2026-11-12 SSA／WebVTT 扩展源码交接（未重建 EXE）

- 四种格式 ASS／SSA／SRT／WebVTT 原格式加载编辑保存，内容优先识别；SSA 旧样式／对齐／颜色与 WebVTT 元数据保留，转换提示损失。WebVTT 预览非浏览器完整排版；生成旧版 SSA、其他文字格式导入与图像 OCR 仍未实现。
- 播放菜单同时提供默认 ASS 和按原编码 copy 提取（ASS 家族／SRT／WebVTT）。来源 codec 用类型字段传递，mov_text 等禁用原格式按钮。外部四格式同目录发现遇到多份精确同名不任意选择。
- 新增 tests/verify_subtitle_ssa_webvtt.mjs，实际三轨 MKV 提取及 SSA 顶部对齐渲染验证通过，结果 target/subtitle-ssa-webvtt-verification/result.json。旧提取回归也通过。九项新增 Rust 用例仅编译；cargo check --tests --locked --offline 通过，日志 target/subtitle-ssa-webvtt-check.log。新增文案七语言及关于索引静态核对完成，git diff --check 通过。
- 实际编辑／保存／窗口提示与关于历史折叠仍待新构建运行验证；当前 EXE 不含本轮源码，未执行构建、Rust 测试程序、提交或发布。

- 2026-11-12，用户明确授权编译字幕编辑／SSA／WebVTT 整合版：编译前确认播放器未运行且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功（73.27 秒）。产物 target/release/unity_yaml_viewer.exe，文件／产品版本 1.3.5，32,747,008 字节，SHA-256：255319BD5CEA7140D0791D16BEC45832D47D65B99E77C1C581159224CCD29924。日志 target/subtitle-editor-ssa-webvtt-release-build.log，核验 target/subtitle-editor-ssa-webvtt-release-verification.json。包含本轮字幕工具栏精简、编辑器方向键、独立内存草稿字幕编辑、基础样式／时间轴／保存互转／烧录快照，以及 SSA／WebVTT 原格式与内嵌字幕提取扩展。保留 spine38 既有 158 项警告及 VideoSettings::configure 未使用警告。未自动启动、提交或发布；完整应用的窗口布局、字幕编辑／保存／转换、原格式提取、烧录交接与七语言界面仍待运行验证。此前“未生成 EXE”描述保留对应源码修改时点事实。


- 2026-11-12，用户明确授权编译字幕时间轴全宽进度条版：编译前确认播放器未运行且目标 EXE 独占访问通过；cargo build --release --locked --offline 成功（57.46 秒）。产物 target/release/unity_yaml_viewer.exe，文件／产品版本 1.3.5，32593408 字节，SHA-256：40748BCCD1B3E88DD05C2303B84367A53D64CFD0750CCD5149CD57F5661BA5A7。日志 target/subtitle-timeline-progress-release-build.log，核验 target/subtitle-timeline-progress-release-verification.json。包含字幕时间轴底部全宽进度条、点击／拖动浏览及条内可见时间范围显示。保留 spine38 既有 158 项警告及 VideoSettings::configure 未使用警告。未自动启动、提交或发布；新进度条实际布局及点击／拖动仍待运行验证。此前未重建描述保留对应修改时点事实。


- 2026-11-12，用户澄清字幕时间轴底部保留滑块、只需与时间轴同宽。已恢复无填充／无数值的全宽 egui 滑块，七语言关于摘要同步；cargo check --locked --offline 通过（target/subtitle-timeline-wide-slider-check.log），实际界面待验证。本次未重建 EXE，当前已有全宽进度条版产物不含此修正。


- 2026-11-12，字幕编辑器补齐播放栏同款音量弹窗／时间同行、列表双击定位时间轴、双向选中及自动滚动（Ctrl 多选同步）、批量平移移到固定列表工具栏。修正播放跟随只改列表 active 与 Ctrl 取消残留高亮。cargo check --tests --locked --offline 通过（target/subtitle-editor-selection-transport-check.log），会话回归用例扩展仅编译。七语言摘要与使用指南同步；实际 GUI 待验证，本轮未重建 EXE，已有程序不含本轮及前述同宽滑块修正。

- 2026-11-12，字幕编辑器新增顶部“统一预览样式”按钮（SRT／WebVTT），独立配置 subtitle_editor_preview_style.json，与普通播放分离。共用控件、不共用偏好；仅预览，不改文档／保存／转换／烧录结果。保持 cue 时间，不沿用普通播放的自动分段计时。用户最终确认单击列表显露时间轴、双击定位视频；播放跟随不强制移动手动时间轴。cargo check --tests --locked --offline 通过（target/subtitle-editor-preview-style-check.log），两项隔离／配置回归用例仅编译；七语言与文档同步。GUI／重启恢复待验证，未重建 EXE。

- 2026-11-12，字幕编辑新增横屏／竖屏布局 Tab，初次按视频方向选择；竖屏视频与播放控制独占左列、编辑区在右侧。扩大起止边界抓取区域并锁定按下起点，保留短字幕中部移动。按用户最终选择保留全宽浏览进度条，固定预留 8 间距＋18 条高，消除滚动区域最小高度造成的挤压。cargo check --tests --locked --offline 通过（target/subtitle-editor-layout-handles-check.log），新增两项用例仅编译；七语言摘要／使用指南同步。实际 GUI 待验证，未重建 EXE；现有 EXE 不含本轮及此前未重建的编辑器修改。

- 2026-11-12：用户明确授权重新编译。构建前通过进程 API 确认程序未运行，并通过目标 EXE 独占访问检查；cargo build --release --locked --offline 成功（1 分 02 秒）。产物 target/release/unity_yaml_viewer.exe，版本 1.3.5，32814592 字节，SHA-256：A67752828EB3F994C69ADC6B9C52CA150BEE6BF60ADD085D32E664915E93D6F7。包含本轮横屏／竖屏布局 Tab、字幕边界抓取及浏览条间距修正，也包含前述未重建的独立预览样式、播放栏和选中定位修改。日志 target/subtitle-editor-layout-release-build.log，核验 target/subtitle-editor-layout-release-verification.json。保留第三方既有警告和 VideoSettings::configure 未使用警告；未自动启动，实际 GUI 仍待验证，未发布。

- 2026-11-12，按用户最终澄清，顶部字幕选择框维持最大 180、展开菜单由最大 320 收窄至 280（逻辑像素），保留截断及名称全文提示。cargo check --locked --offline 通过（target/subtitle-menu-width-check.log），七语言摘要同步，实际 GUI 待验证。本轮未重建 EXE；现有 Release 仍不含此次菜单宽度修改。

- 2026-11-12，实时字幕整组控件与状态从视频信息行移至音频信息行，在自动生成／翻译字幕之后；后台任务调度及右键入口不变。cargo check --locked --offline 通过（target/live-subtitle-audio-row-check.log），七语言关于摘要同步。实际 GUI 待验证，未重建 EXE，现有产物不含此修改及前述菜单最大 280 修改。

- 2026-11-12，布局 Tab 移到字幕文件信息行最左侧；竖屏右侧属性改用独立裁切面板，修复溢出覆盖时间轴。视频列收紧默认边距，右侧属性保留左右 6／底部 4 留白，播放栏缩减内边距。cargo check --locked --offline 通过（target/subtitle-editor-portrait-overlap-check.log），七语言摘要及指南同步；GUI 待验证，未重建 EXE，现有程序不含本轮与此前菜单宽度／实时字幕音频行修改。

- 2026-11-12，用户明确以横屏当前字幕编辑区间距为准。横竖屏属性面板统一采用相同侧面板框架（左右 8、上下 2），替代竖屏单独的左右 6／底部 4；继续保留独立裁切及紧凑视频边距。cargo check --locked --offline 通过（target/subtitle-editor-matching-property-padding-check.log），七语言摘要同步。GUI 待验证，未重建 EXE。

- 2026-11-12，字幕编辑播放当前句（区间图标）／播放暂停／前后逐帧／音量封装成固定控件组，居中放在属性区底部、时间轴上方；当前句图标在播放左侧。视频下方只保留居中的音轨与时间，删除原属性滚动区播放当前句文字按钮。cargo check --tests --locked --offline 通过（target/subtitle-editor-centered-transport-check.log），七语言摘要与指南同步，GUI 待验证。未重建 EXE，当前产物不含本轮及之前源码修改。

- 2026-11-12：用户明确授权编译；构建前确认程序未运行并通过目标 EXE 独占访问检查。cargo build --release --locked --offline 成功（1 分 16 秒）。产物 target/release/unity_yaml_viewer.exe，版本 1.3.5，32815616 字节，SHA-256：7F08FE21D47454446FAB4A8DC5F1390E6E9218C184D70C1DE59640FDDCC1FCB1。包含当前字幕编辑播放控制居中／当前句区间图标、横竖屏属性间距及裁切、文件信息行布局 Tab，以及字幕展开菜单最大 280、实时字幕移至音频信息行等此前未重建修改。日志 target/subtitle-editor-centered-transport-release-build.log，核验 target/subtitle-editor-centered-transport-release-verification.json。保留既有警告；未自动启动，实际 GUI 仍待验证，未发布。

- 2026-11-12，保持播放栏原高并按全部可用高度垂直居中；批量平移四项按整组空间换行；另存为初始文件名统一 .ass，与默认 ASS 类型一致。cargo check --tests --locked --offline 通过（target/subtitle-editor-toolbar-saveas-check.log），七语言摘要同步。实际 GUI 待验证，未重建 EXE，已有 Release 不含这三项修改。

## 2026-11-12：录屏源代码已接入，待新 EXE 运行验证

- 新增 src/screen_record.rs 与“文件 → 录屏”，全屏／多屏／坐标区域直接编码 MKV／MP4，质量／帧率、倒计时、最小化、Ctrl+Shift+F9 停止及完成后选择进入视频编辑；没有系统声音／麦克风／暂停续录／鼠标框选，编码使用 CPU。七语言和当前版本文档同步。
- cargo check --tests --locked --offline 通过（target/screen-record-check.log）；已有 FFmpeg 的合成视频 MKV／MP4 编码、q 停止、FFprobe 元数据及重复目标哈希保护已实际验证，证据 target/screen-record-validation-20261112-170657/results.json。Rust 回归用例仅编译，未运行测试程序；真实桌面、长期负载、多屏高 DPI、最小化与快捷键、进入编辑及关于历史 GUI 待验证。
- 本轮未重新生成 EXE、未启动新应用、未发布；此前 target/release 下程序不含本轮录屏。生成可执行程序仍需用户明确确认，届时须重新检查进程及目标占用。保留此前已有工作区修改。

## 2026-11-12：录屏 Release 已编译，未运行／未发布

- 用户明确授权，cargo build --release --locked --offline 成功（1 分 06 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，32833536 字节，SHA-256：1A8341DF7BF9215AC52914FAE0D566029892FABEAC7F34F09C644F751664EFF3。包含录屏及现有工作区修改，日志 target/screen-record-release-build.log、核验 target/screen-record-release-verification.json。
- 编译前 EXE 独占访问通过；CIM 查询拒绝后脚本未中止即开始编译，随后 Get-Process 确认程序未运行，详见 preflight JSON 与当前版本记录。后续检查脚本应将查询错误视为终止错误，并在构建前完成备用进程检查。实际 GUI 仍待验证；未自动启动、未发布。

## 2026-11-12：录屏交互调整源码完成

- 取消保存对话框，自动写入系统用户下载目录，以纳秒时间戳命名。屏幕菜单按单屏／多屏调整；窗口悬停吸附、左键确认、右键／Esc 取消，绿色线框确认后可移动缩放；不使用按住左键拖框。
- 悬浮框含坐标、尺寸、红圆开始／红方块停止和停止快捷键；全屏开始后隐藏悬浮框，区域录制保留停止按钮。录制时线框锁定并穿透鼠标。三个原生辅助窗口设置并回读捕获排除，要求 Windows 10 2004+；失败禁用开始。
- cargo check --tests --locked --offline 通过（target/screen-record-region-check.log），新边界用例仅编译，七语言及关于默认两版／历史收起静态检查通过；文档同步。实际 GUI、混合 DPI／多屏、悬浮交互及视频中完全没有线框／悬浮内容仍待运行验证。此段记录时 EXE 尚未重建，后续构建结果另记；未发布。

## 2026-11-12：悬浮录屏工具条与快捷键设置源码完成，需重建验证

- 录屏入口直接进入窗口吸附区域，十字光标确认后编辑绿色线框。工具条包含模式切换、X/Y/W/H、设置、开始／停止及退出；优先选区下方，按显示器工作区自动避让。齿轮进入独立录屏设置，原大录屏面板不再使用。
- screen_record.json 保存参数和两个可修改快捷键（默认 Ctrl+Shift+F9 停止、Ctrl+Shift+F8 选区），使用系统全局注册检查占用，取消不保存，注册／写入失败尝试恢复。开始先检查 FFmpeg／FFprobe，缺失确认后进入第三方支持。全屏录制隐藏工具条，结束后恢复并在下方显示路径／编辑／目录；无成功弹窗，错误详情单独提示。
- cargo check --tests --locked --offline 通过（target/screen-record-toolbar-check.log）；新增设置／快捷键／位置回退用例仅编译，未运行测试程序。七语言及文档同步。尚未生成新 EXE，之前程序不包含本轮和上一轮未重建的交互；实际 GUI、快捷键冲突、持久化与视频捕获排除均待运行验证。未发布。
- 先前编译占用检查曾发现 target/release/unity_yaml_viewer.exe 正在运行；用户尚未按规则关闭并再次明确授权编译。后续生成 EXE 前必须重新核对授权、进程及目标独占访问，不自动关闭程序。

## 2026-11-12：悬浮录屏工具条 Release 已重建

- 用户再次明确授权；构建前进程检查及 EXE 独占访问均通过。cargo build --release --locked --offline 成功（1 分 04 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33288192 字节，SHA-256：0198B31EB40E736051B996F8B6E30ED5E54346A4D41BE5631C8647A522022676。包含悬浮录屏工具条及前述未重建的录屏交互，连同工作区此前修改。
- 日志 target/screen-record-toolbar-release-build.log，检查 target/screen-record-toolbar-release-preflight.json，核验 target/screen-record-toolbar-release-verification.json。未自动启动、未发布；实际 GUI、全局快捷键冲突／重启恢复、工具条避让、组件缺失跳转、录制成功内联结果与捕获排除仍待运行验证。

## 2026-11-12：左键拖拽创建录屏区域源码已补，EXE 待重建

- 指定区域按下不再立即确认：单击保留窗口吸附，超过系统容差后按左键拖拽创建矩形，松开留下可编辑线框；支持四方向、负坐标、桌面边界和最小 16×16。拖动时暂停吸附并实时更新尺寸，右键／Esc／丢失捕获恢复原区域，清理所有退出路径的鼠标捕获。
- cargo check --tests --locked --offline 通过（target/screen-record-drag-check.log）；点击容差和矩形几何回归仅编译，未生成或运行测试程序。七语言和当前版本文档同步。实际鼠标交互、混合 DPI／跨屏及取消／录制输出待运行验证；未重建 EXE、未启动、未发布。此前已编译的 0198B31E… 产物不含本轮拖拽创建。

## 2026-11-12：自绘录屏工具条、框内移动／八手柄、录屏历史源码完成

- 工具条改为主程序 egui 独立无边框视口，去掉系统控件及完成按钮。最终设置入口是主窗口同款 Gear SVG，在圆圈十字后、X/Y 和“宽 × 高 px”前；圆圈十字重新进入窗口吸附／拖拽画框。Esc 录制前退出区域与工具条，录制中仍走停止快捷键。
- 确认区域后可从内部四向光标拖动移动；四边中点单轴缩放、四角双轴缩放，限制边界并保持对边锚定。录制期间隐藏移动交互层、锁定并穿透线框。工具条／屏幕菜单新视口显示前检查进程归属与捕获排除，真实输出仍待验证。
- 独立录屏历史默认20、可设20–50；旧设置默认兼容。重新进入或重新选择时收起上次结果。缺失视频标红，存在视频可选中编辑或打开目录。按用户最终取消删除文件的要求，红色按钮仅移除记录，任何历史裁切／移除均不删除视频。
- cargo check --tests --locked --offline 通过（target/screen-record-region-handles-check.log），新增几何、历史及偏好兼容用例仅编译，未运行测试程序。七语言与双语文档同步，实际 GUI／多屏 DPI／操作手感、历史持久化、捕获排除仍待新版运行验证。未生成新 EXE；此前 0198B31E… 产物不含本轮及前一轮拖拽创建功能，未启动／发布。

## 2026-11-12：自绘工具条／录屏历史 Release 已重建

- 用户明确授权；构建前进程检查与 EXE 独占访问均通过。cargo build --release --locked --offline 成功（1 分 07 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33413632 字节，SHA-256：2508DC1A56C38578A6EB8F7FC6390113B5F842489415B560B1A6D49FA887B2EC。包含主程序自绘录屏工具条、最终设置图标位置、宽×高 px、区域内移动和八手柄、Esc 退出、独立历史记录及此前拖拽选区，连同工作区此前修改。
- 日志 target/screen-record-custom-toolbar-release-build.log，检查 target/screen-record-custom-toolbar-release-preflight.json，核验 target/screen-record-custom-toolbar-release-verification.json。未自动启动、未发布；实际 GUI／混合 DPI／操作手感、历史重启恢复及视频捕获排除仍待运行验证。

## 2026-11-12：紧凑工具条／透明投影／系统声音源码完成

- 550 逻辑点主体、较矮输入框与标准 ComboBox 居中、Body 提示字号、简写停止快捷键、无结果分割线。同一透明视口显示标准屏幕列表，透明清屏和外边距内自绘阴影避免原生阴影不稳定。
- 录制前增加系统声音按钮（麦克风图形，默认开启，关闭显示红色禁止，配置持久化）。录制锁定；不采集麦克风。WASAPI 默认播放设备回环，有限缓冲生成静音并流式交给 FFmpeg 编码 AAC 128 kbit/s 立体声。关闭不打开音频设备且没有音轨；错误提示并保留已有视频检查。Windows 0.54 直接依赖复用锁文件版本。
- cargo check --tests --locked --offline 通过：target/screen-record-audio-check.log。配置兼容用例仅编译，未运行测试程序。七语言和当前版本文档同步，关于默认两版／旧版收起配置保持不变。实际 GUI 透明圆角／投影、混合 DPI／菜单避让、主窗口显示、音频格式兼容、同步与长录制、静音恢复和设备中断、捕获排除待运行验证。
- 未重建 EXE、未启动／发布。上次 2508DC1A56C38578A6EB8F7FC6390113B5F842489415B560B1A6D49FA887B2EC 的 Release 不包含此轮；如用户授权构建，先重新检查目标程序进程与 EXE 独占访问，禁止自动关闭运行程序。

## 2026-11-12：紧凑录屏工具条与系统声音 Release 已重建

- 用户明确授权后，确认 unity_yaml_viewer 无运行进程、目标 EXE 可独占访问，执行 cargo build --release --locked --offline 成功（约 1 分 08 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33280512 字节；SHA-256：A7BF34CE5A37601344DEE8C5B238F53FF3C8C85452B0AF99D4C033766F66AB9E。
- 本次产物包含紧凑输入与标准屏幕下拉框、正常字号提示与无分割线结果区、透明圆角及自绘投影调整，以及默认开启并记忆的系统声音开关和 WASAPI 回环采集，连同工作区此前修改。
- 构建日志 target/screen-record-audio-release-build.log；占用检查 target/screen-record-audio-release-preflight.json；产物核验 target/screen-record-audio-release-verification.json。构建仅有既有 vendor 与 media_trim.configure 未使用警告。未自动启动、未发布；实际音频同步、静音恢复／设备兼容、透明圆角投影及捕获排除仍待运行验证。

## 2026-11-12：外圈黑底修正／历史上限滑条，EXE 待重建

- 用户实拍确认 A7BF34CE… 的工具条外圈黑底仍存在。根因是旧 glutin-winit::finalize_window 清除透明属性，winit 未启用 DWM 透明。vendor/glutin-winit 0.5.0 增加 Windows 保留透明请求的小补丁，通过 Cargo patch 接入；保留 MIT 许可和 LOCAL_PATCHES.md，不修改外部 Cargo 缓存。依据上游 https://github.com/emilk/egui/pull/8423 ，不整体升级 eframe。
- 录屏设置历史条数改为数值框加主设置同款 theme::slider，20–50、默认 20，保存后生效。关于七语言及指南同步。
- 待验证：真实透明圆角／阴影、屏幕列表展开、全屏隐藏后恢复、混合 DPI／跨屏、捕获排除及滑条保存取消。此轮不含新 EXE；上一次 A7BF34CE… 不含此修复。后续用户授权构建时须重新检查程序及目标 EXE 占用，不自动关闭程序。
- 追加工具条收紧与关闭图标调整：按首排实际控件宽度计算主体宽度，保留阴影透明外边距；关闭图标为物理像素对齐的圆头矢量 X。长结果路径在同一宽度内显示，不再撑宽。此轮同样待新版 GUI 验证。
- 验证：cargo check --locked --offline 通过（target/screen-record-transparency-check.log）；cargo tree 确认 eframe 使用本地 glutin-winit 0.5.0 补丁，原许可及其他原始源码核对一致。七语言摘要、默认两版／重开收起配置和 git diff --check 通过。仅有既有 vendor／缓存及 media_trim.configure 未使用警告；未运行 GUI、未生成新 EXE、未发布。

## 2026-11-12：透明工具条修正／紧凑宽度／矢量关闭按钮 Release 已重建

- 用户明确授权后，确认 unity_yaml_viewer 无运行进程、目标 EXE 可独占访问，cargo build --release --locked --offline 成功（约 59 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33383424 字节；SHA-256：9573870B6095AB9F9C46E25E75C0436F5B6DE502EEBD878198FF8130E94338DE。
- 本次产物包含 glutin-winit 0.5.0 的 Windows 透明标记修复、按第一排实际控件宽度收紧的工具条、圆头矢量关闭 X、录屏历史上限数值框与主设置同款滑条，连同此前系统声音及工作区修改。
- 日志 target/screen-record-transparency-release-build.log；占用检查 target/screen-record-transparency-release-preflight.json；产物核验 target/screen-record-transparency-release-verification.json。仅有既有 vendor 与 media_trim.configure 未使用警告。未自动启动、未发布；实际黑边消失／圆角投影、菜单及隐藏恢复、混合 DPI、关闭图标清晰度与宽度、历史上限滑条、声音同步和捕获排除仍待运行验证。

## 2026-11-12：共用 error.log 与录屏故障诊断源码

- 用户要求先记日志并指定 error.log；全软件入口为 error_log::write/error，初始化接标准日志 Warn/Error 与 panic 回溯。启动错误、后台任务失败／中断和录屏阶段已接入。日志 `%APPDATA%\UnityYAMLViewer\error.log`，约 2 MiB 轮换为 error.log.1；未声称全部历史 UI 报错已接入。
- 录屏日志包含会话 ID、参数、WASAPI 接口／格式／错误码、FFmpeg 参数／PID／stderr／退出状态及文件状态。错误窗置顶且手动关闭前保留，可复制／打开日志；工具条保留错误详情入口。
- 用户反馈的三秒后停止无文件尚未定位。现有 FFmpeg 合成流输出成功，WASAPI 初始化探针成功（48 kHz 双声道 float32，零数据包）；真实采集未验证。证据在 target/recording-start-diagnostic。按用户“先日志”要求，中央大数字倒计时暂未实现，保留后续待办。
- 未重建 EXE；上次 9573870B… 不包含日志机制。下一次用户授权编译后须先查占用，再通过新版复现并读取 error.log 定位。未启动／发布。
- 验证：cargo check --tests --locked --offline 通过（target/screen-record-error-log-check.log），含 UTF-8 追加／轮换／超大日志截断回归用例的编译检查，未生成或运行测试程序。七语言新文案／关于摘要、关于默认两版与折叠设置及 git diff --check 已核对。错误详情显示时暂时隐藏置顶工具条，关闭详情后恢复，防止遮住错误。日志实际写入与 UI 保持仍待新版运行核对。

## 2026-11-12：共用 error.log 诊断版本 Release 已重建

- 用户明确授权后，检查 unity_yaml_viewer 无运行进程且目标 EXE 可独占访问，cargo build --release --locked --offline 成功（约 59 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33509888 字节；SHA-256：08F579F92B19C5ED5DE95C78359FE1BC2265CB15B5BFB224959271F5EEFA2C80。
- 产物包含共用 error.log、标准运行库警告／错误和 panic、后台任务失败记录，以及录屏会话阶段／声音接口／FFmpeg stderr 与退出信息；录屏错误可持续查看、复制并打开日志，连同此前工作区修改。中央大数字倒计时仍待实现；自动结束且无文件的根因待用本次版本复现并读取日志定位，不宣称修复。
- 构建日志 target/screen-record-error-log-release-build.log；占用检查 target/screen-record-error-log-release-preflight.json；产物核验 target/screen-record-error-log-release-verification.json。仅有既有 vendor 和 media_trim.configure 未使用警告。未自动启动、未发布；日志实际写入／轮换、错误窗口保持和真实录屏仍待运行验证。

## 2026-11-12：录屏 PCM 暂时不可写修复／中央倒计时／字幕浏览条源码完成

- 用户实测先正常后重录失败；error.log 已明确定位 write PCM 的 10035，WASAPI 初始化与启动正常。改用 8 MiB 有界发送队列和短写偏移，WouldBlock 不作为致命错误、不重复或丢失字节；采集继续，5 秒无写入进展才失败，停止最多 2 秒排空。原失败轮存在 199812 字节部分视频。新版连续重录／同步仍待实测。
- 中央倒计时已补：独立透明捕获排除窗口显示 3→2→1，截止或取消关闭，工具条去掉固定三秒文字。多屏混合 DPI、实际数字位置及输出排除待验证。
- 字幕浏览条复用视频／音频同款比例滑块，去蓝色填充及时间文字；保留全宽、间距和只平移不定位播放的操作。
- cargo check --tests --locked --offline 通过（target/recording-pcm-countdown-timeline-check.log），含新增 PCM 队列回归和字幕间距用例的编译检查，未运行测试程序。七语言与文档同步；未重建 EXE、未启动／发布。上一构建 08F579F9… 不含本轮，用户授权构建后仍需先检查占用。

## 2026-11-12：PCM 10035 修复／中央倒计时／字幕浏览条 Release 已重建

- 用户明确授权后，确认 unity_yaml_viewer 无运行进程、目标 EXE 可独占访问，cargo build --release --locked --offline 成功（约 1 分 09 秒）。target/release/unity_yaml_viewer.exe，版本 1.3.5，33502720 字节；SHA-256：F4739FB0AC34BD4D4A36B588CD60AD354D67E29CC4D91B56A1669CC3AB21A52F。
- 产物包含录屏 PCM 非阻塞暂时不可写的有界队列重试、区域中央 3→2→1 大数字倒计时，以及字幕／视频编辑共用比例浏览滑块，连同此前 error.log 及工作区修改。未自动启动；连续重录、声音同步、倒计时显示和捕获排除、字幕滑块实际交互仍待运行验证。
- 构建日志 target/recording-pcm-countdown-timeline-release-build.log；占用检查 target/recording-pcm-countdown-timeline-release-preflight.json；产物核验 target/recording-pcm-countdown-timeline-release-verification.json。仅有既有 vendor 和 media_trim.configure 未使用警告，未发布。


## 像素 Tile / Photoshop UXP（2026-11-16）

新增 src/asset_viewer/pixel_tile.rs；文件菜单、图片工具栏及 .uytile 打开入口已接线。支持单层像素编辑、同图／多 Tile 循环、跨格绘制、带高度预览、工程保存与当前 Tile PNG 导出。独立 PS 原型位于 plugins/photoshop-isometric-tile。

未执行生成可执行程序的命令；Rust 测试只做 check，未运行。界面与 PS 插件均待实机验证，不代表已发布。保留本轮开始前已有录屏、媒体、字幕和文档等工作区修改；未提交。

### PS CCX 试装包（2026-11-16）

- 产物：plugins/photoshop-isometric-tile/dist/isometric-tile-preview-0.1.0.ccx（6147 字节）。
- SHA-256：ed397ddaf6c57512860e4bfc9ef98c63d408ea32ca6305357698e966d66a2d60；旁边保留 .sha256 文件。
- package.ps1 可重现封装并逐项检查包内 5 个文件；只封装插件资源，不生成 Rust 可执行程序。源码 JS 语法、PNG 编码测试通过。
- 未使用 UDT（当前环境未安装），按 ZIP 格式生成试装容器；Creative Cloud 安装、PS 加载与卸载待用户试装，不能作为已验证发布包。


### 安装器源码、PS 左右布局与 PSD Slot（2026-11-16）

- 独立工程：plugins/photoshop-isometric-tile/installer；Cargo.lock 独立维护，check --offline --locked --tests 已通过，仅检查，不运行测试、不生成目标 EXE。复用主项目图标；安装器 build.ps1 只有用户确认编译后方可执行，已做语法检查而未执行。
- 用户已手动加载 PS 面板；当前插件源码更新为 0.1.2，左侧设置宽 260px，右侧自适应预览，增加多文档 Slot 绑定和变更缓存。模拟宿主与 PNG 编码检查通过；新版 PS 效果仍待用户验证。
- 未改动用户已安装的插件目录，未关闭 PS。用户需要关闭 PS，再用当前四个源码文件覆盖手动安装目录后重新打开。

### PS 0.1.3 手动更新产物

- dist/isometric-tile-preview-0.1.3-files.zip：6396 字节，SHA-256 92b55e5d200f3d03af317b78f0847430e94b196ad94c7a0115cd22224c390476，只有四个运行文件，解压到用户已验证的插件目录。
- dist/isometric-tile-preview-0.1.3.ccx：8539 字节，SHA-256 7fa2ea8558dfb6dd19ebba47f5fc99063b3e246a0ee5ca5b4023e27d2c9e37c8。两包逐项核对源码字节一致，哈希文件在旁边保留。
- 当前版紧凑间距、顶部版本号来自清单、左右布局与多 PSD Slot。仍需 PS 实机复核，未对用户目录执行安装。安装器目标 EXE 未生成。

### PS 0.1.5 模式与边缘回绕边界

- 单文档网格／多文档 Slot／单 Tile 重复共存。新增顶层 Multiply 全锁定辅助层、单文档斜轴网格预览及逐格透明 PNG 导出；3×3 九张、256×128 拆四张128×64 已由模拟宿主与实际 PNG 解码校验。
- 源码还未覆盖用户已安装目录；PS 实机效果待验证。自动隐藏辅助层的读取使用模态历史回滚，需重点检查锁定、可见性、撤销、画笔光标与忙碌时行为。
- 用户进一步澄清重点是 PS 原生画笔的菱形边缘回绕；该能力未实现，界面和文档已明确，不应以副本更新冒充回绕。正在查 PS 内的现成方案（Artlandia SymmetryShop 等），不再以独立像素软件替代需求。

### PS 0.1.5 产物与 PS 原生回绕调查

- 四文件包 dist/isometric-tile-preview-0.1.5-files.zip：12125 字节，SHA-256 94c7f9c14a6012e95fb50dba0e0b9a198f14031729550e3e6b76485a7971b22a。CCX：15008 字节，SHA-256 ae3cf292cd1f55acaebaedb6139c0c3d38fe13c428c49ef5f9d3f50294300044。包内字节与源码核对通过；没有自动安装。
- PS 内的现成候选：Artlandia SymmetryShop 4。官方 v4 页面说明 Photoshop 画笔、橡皮等操作会触发 Linked Pattern 自动重建，控制路径可调整重复大小与方向；官方 requirements 范围为 CC 2014–2025，包含 PS 2024。https://artlandia.com/products/SymmetryShop/v4.html 及 https://artlandia.com/products/SymmetryShop/requirements.html 。尚未实际验证等距菱形跨边缘原生笔刷逐点回绕；不能把 Linked Pattern 自动重建视为已确认等价。


## 软件内像素 Tile 回绕与多屏预览（2026-11-16）

- 修改 src/asset_viewer/pixel_tile.rs：单／多格周期回绕、1–32方形刷头、整组编辑、独立副本、最多64张、工程v2兼容v1；窗口设置／绘制／预览三列，独立只读预览走 native_window 多视口。
- 独立预览不会调用 finish_stroke，避免副视口无鼠标按下状态提前提交主视口笔触；主视口修改或副窗口开关／缩放时请求重绘。绘制纹理按受影响 Tile 更新。
- cargo check --tests 已通过；Rust测试未运行以遵循不生成可执行程序约定，独立几何样例核对通过。真实窗口布局、多屏 DPI、拖动、关闭恢复和画笔回绕手感仍待授权构建后运行验证。保留原工作区媒体、录屏等修改，未执行安装、构建或发布。

## 用户授权后生成验证产物（2026-11-16）

- 最新主程序源码按用户参考图将Tile工具改为互斥图标（画笔／橡皮／滴管），Tool枚举替代erase/pick布尔，点击当前工具不取消，切换finish_stroke，Alt临时取色不改变所选。使用icons统一SVG绘制，新增brush/eraser/eyedropper三资源及画笔七语言tooltip；未编译主程序EXE，实际UI待验证。

- 最新双产物交付：2026-11-17用户分别授权主程序和PS安装器，均检查进程关闭和EXE未占用后Release成功。
  - `target/release/unity_yaml_viewer.exe`：1.3.6，33605120字节，SHA-256 `38754ab83aca05a8375c87445304b2ea24190d3b79c494015c00f959f115ca8c`；含主图平滑RGBA8回调及Tile关闭／工具布局／滚轮／HEX源码。
  - `plugins/photoshop-isometric-tile/installer/target/release/ps-tile-installer.exe`：内嵌插件0.1.0，5196288字节，SHA-256 `700094e1b9c743e6bb844ab2173d05d1ae6285c7383d5044ee0007cf3dba123e`；含边界复制、历史方向比较、自动wrap-status及统计。
- EXE版本资源核对通过，两个输出旁保留本地.sha256。PS安装器5项Rust及wrap／PNG／panel／icon检查全部通过。未自动运行或安装、未发布、未生成旧格式资源包；实际GPU和PS表现待用户复验，以下旧构建数据为历史。
- 主程序Release专项测试已实际执行：render_system::image_preview::tests 2项、pixel_tile::tests 9项全通过；不替代真实GPU上下文测试。构建日志target/alpha-smooth-release.log，测试日志target/alpha-smooth-tests.log和target/pixel-tile-tests.log；安装器日志仍在其target目录。

- 最新主程序平滑亮边源码：asset_viewer主图片平滑改用render_system/image_preview.rs回调，gamma预乘RGBA8经现有video::Renderer手动双线性／预乘混合，独立Gpu.image_preview缓存，不走egui sRGB纹理过滤再gamma转换；pixel模式不改。ImageTab缓存按原图Arc／通道／Alpha键重建，预算及淘汰同步，GPU shutdown释放图片缓存。
- cargo check --tests通过，新增亮边与部分Alpha检查仅编译未运行，实际主窗口效果待授权构建后验证；本轮未生成EXE，不能把该主程序修复计入已交付PS安装器。

- 最新用户最终规则：边界覆盖原位保留且复制到对边，真正越界才搬移清空。wrapPixels新增copied统计和RGBA直接复制，不积累Alpha；读取前一historyStateID比较变化侧（支持内向外），缺少历史按外缘向内并提示，透明源不传播擦除。原导出diamondMask范围不变。
- 已加入紧邻自动开关的wrap-status，显示等待paint／200ms延迟、手动自动、完成搬移/边界复制数、位移、跳过和失败原因；既有笔迹不开启即处理。wrap-check新增四边保留同步、下边2×2配对、内侧历史变化、半透明幂等和状态时序通过，panel检查通过。未生成新EXE、实机待验证，保持0.1.0；前述唯一归属试验已撤回，PSD路径问题不阻塞当前实现。

- 最新回落排查：用户明确仅左下／右下到上方差1像素，四边辅助覆盖都保留。已撤回未交付的insideWrapUnit互斥判断／repeatMask预览试验，实际仍diamondMask保边，不接受“重叠就必少一像素”的解释。新增2×2下向上整数位移和四边保留测试通过，但实机根因未确认；已异步询问回落前PSD完整路径，需实际源像素定位，不能写成已修复。
- 源码新增wrap-status紧邻开关，显示手动／自动区别、等待paint／200ms延迟／执行／完成含位移统计／状态变化跳过／失败。开关只启用监听不立即处理旧笔迹，手动按钮可处理旧内容。模拟状态时序通过，未重新生成安装器，版本0.1.0。

- 最新交付：2026-11-17用户授权编译PS安装器，进程关闭及独占打开检查通过，Release成功；同路径EXE5192192字节，SHA-256 `39c4772564fdc76b6f549a814d8b64dc4044738ee27d993d0afad7cb6bffc71e`。含普通层手动／自动回落、Web用途PNG无损压缩、辅助线边缘包含与滚轮焦点调整，0.1.0。5项Rust安装器测试及wrap-check／check／panel-check／icon-check全部通过。未自动安装、未发布、不生成CCX／ZIP；实际PS重点验证回落事件与撤销、源尖角转换导出、真实PNG大小及滚轮，主程序EXE未重建。

- 用户询问旧图标是否无用，核对manifest／安装器／生成脚本后删除四个无引用的tile-dark-23.png、tile-dark-46.png、tile-light-23.png、tile-light-46.png；保留基础名和@1x／@2x。icon-check通过。删除前四文件已由用户暂存为A，本轮仅删除工作区文件，未改暂存区，当前AD需用户下次暂存时纳入删除。

- 最新PS源码实现当前普通层回落：manual按钮与可选paint事件自动处理，双轴周期映射1×1／多格、源清空、同层透明叠加，保留边界和其他层；RGB8／普通可见未锁定／正常混合100%层，拒绝蒙版及效果等，读取完整图层bounds含已有画布外内容，最多16M像素。历史事务失败回滚／一步撤销，自动处理核对历史身份避免undo重做。wrap-check.cjs覆盖映射、负范围、alpha、幂等、限制、回滚与自动事件通过；未实机验证、未编译EXE，0.1.0。未加画布缓冲区，画布外未记录笔迹与对边橡皮仍不支持。
- PNG导出改为web=true无损滤波+LZ77 fixed-Huffman压缩，必要时回退stored；8位RGBA不降色、只IHDR/IDAT/IEND。独立unfilter+Node zlib随机及结构RGBA回读逐字节／CRC通过，渐变128×64 32900→387字节，真实纹理压缩结果不承诺。不是PS原生Save for Web命令；preview默认仍stored以免每次捕获新增压缩开销。新增png-test-utils.cjs仅测试使用，不部署。需与本轮边缘覆盖及滚轮焦点调整一起授权构建后实机验证。

- 用户最终明确边缘规则：应切菱形内部并包含辅助线覆盖范围。已以diamondMask统一转换／平面预览／cellPNG：内部并集gridRaster边界覆盖，保留源RGBA不烘焙线色、矩形外角透明。上一轮完整矩形方案被替代，现不是保留所有矩形像素；边界可在相邻切片重复。新增全部guide像素覆盖与四端转换／PNG检查通过，真实PS待验证，尚未重建EXE，版本0.1.0。旧转换丢失像素需源文档重新转换恢复。

- 最新PS源码修复转换丢尖角：移除readSlotPixels理想菱形判定，保留完整矩形RGBA（仍按设定尺寸／8位sRGB），每Slot独立摆放。新增全缓冲保留和四端尖角检查，模拟通过。旧转换doc不自动恢复，需源文档重新转换；cellPNG导出仍有独立硬遮罩，尚未宣称解决其边缘保真。未编译安装器，保持0.1.0；需与滚轮焦点调整一并交付验证。

- 最新PS滚轮兼容源码：用户确认预览wheel未接收、文档仍滚动；stage加tabindex=1、mouseenter／mousedown focus、捕获阶段非passive wheel、前景img pointer-events:none。模拟检查通过，真正PS事件分发待验证，未重建EXE。参考官方UXP正tabindex说明和开发者报告 https://forums.creativeclouddeveloper.com/t/bug-mouse-wheel-scrolling-not-working-in-uxp-panels-when-zoom-with-scroll-wheel-is-enabled/12437 （报告针对27.11+，不能直接认定PS2024同因）；没有修改全局偏好。
- 用户进一步询问单文档回落层范围，已说明建议仅当前普通像素层、排除辅助层及其他层；回落功能仍未实现，不能把该设计描述为当前能力。

- 2026-11-17最新交付：用户明确授权编译PS安装器，检查进程关闭和EXE未占用后Release成功。同路径EXE5178880字节，SHA-256 `44d2c616c8a4ace82220b70aa30000e875002b31629369704db46b0fd38ab83d`。插件保持0.1.0，包含最新像素中心格线、2／3／4预设、转换单文档、错位选择、1:1和滚轮。5项Rust及图标／PNG／模拟宿主与转换检查全通过，未自动安装、未发布；实机待复验，主程序未重建。
- 按EXE单一交付约定，本轮未调用旧格式package.ps1，未重新生成CCX／ZIP；以下资源包记录为已清理历史。

- 最新PS源码增加预览wheel缩放，仅preview区域监听，单位归一及单次幅度限制，0.1–10倍，与底部及1:1共享updateZoom。方向／边界／零增量和无额外采样模拟检查通过；实际PS待验证，尚未重建安装器，保持0.1.0。

- 最新PS源码在缩放条右侧新增zoom-reset（1:1），复用updateZoom同步数值／滑条、回到1倍；从10倍复位及不额外读取模拟检查通过。未编译安装器，版本0.1.0。

- 最新PS源码增加可选错位预览：多Slot下stagger-mode选择无／沿列／沿行／双向，列行各0–8整数、默认2，可组合1×2／2×1。按相邻循环组奇偶映射来源Slot，地面格点不变，标签／点击匹配来源；仅预览，不改绑定和转换原排列，不额外采样。纯映射与模拟UI验证四模式、负坐标双组周期、单行单列排列与1×2／2×1偏移通过，真实PS待验证、未生成EXE，版本0.1.0。前述待选方向现已落实为用户可选。

- 最新PS源码增加“转换为单文档”：多Slot平面排列转新doc，每格独立像素层含空格／重复引用、行列顺序，顶层锁定Multiply网格，自动切换单文档刷新；源可见画面8位sRGB按设定尺寸缩放／裁菱形，不保留原PSD内部层结构，源文档保持不合并。源去重读取、辅助层历史回滚、目标部分失败保留；3×2位置／顺序／空格／重复／切模式及三类失败模拟通过，实际PS未验证、未生成EXE，版本0.1.0。
- 用户追加“错一个2×2”预览，已异步询问沿列错2／沿行错2／行列各错2，等待回答，不应擅自把错位预览写入单文档转换几何。

- 最新PS源码将快捷排列改为2×2／3×3／4×4，8×8仅手填，仍支持64格及缩小后保留绑定。三个快捷值、手填8×8和3×5模拟检查通过；未重新编译安装器，后续与像素中心格线修正一并构建，0.1.0。

- 最新PS网格再修正：用户指出下移一像素；旧预览与PSD共用栅格仍有整数落点／像素中心偏差和底端clamp问题。gridRaster现按像素中心主轴求交、裁除越界、不压缩端点，新增独立对称与尖角检查通过。新版预览及新建辅助层受益，已有PSD层不自动重写；尚未编译安装器，0.1.0。

- 用户要求只通过 EXE 安装：已删除 dist 内22个历史 CCX／ZIP及对应.sha256，新增插件目录.gitignore忽略dist；package.ps1默认不生成，仅显式-GenerateLegacyArchives可生成备用旧格式，默认无产物检查通过。之后常规安装器编译不再附带调用该资源打包脚本；源码、当前EXE及本地构建／校验日志保留。以下资源包路径和哈希为已清理的历史记录。

- 最新交付：用户授权后检查安装器关闭、EXE未占用，Release成功；同路径EXE 5169664字节，SHA-256 `a4503a022ec62e7bbb244c7eaec4f9d732558b1ad22219d32aa69cf7c993e5c6`，内嵌0.1.0含底部0.1–10缩放条及Slot坐标开关前置。5项Rust及图标／PNG／模拟宿主检查全部通过，未自动安装、实机待验证，主程序未重建。
- 同步手动ZIP16304字节，SHA-256 `da7946d28dd3eb8414f825119152f9aeddd2355fbc93f26fd1e539105f685242`；CCX19675字节，SHA-256 `2f9d3faf16141aaf42beced79e767a0311083fb3075123303787109d7b6affcd`。以下旧哈希为历史产物。

- 最新PS布局源码：show-slots开关移至selected-slot下方、文档绑定／新建／导出之前，便于就近控制坐标标签。模拟检查通过；尚未与底部缩放条调整一起重建安装器，保持0.1.0。

- 最新PS源码：缩放移到预览区底部固定条，数值框加range滑条同步，0.1–10倍（旧EXE上限8），即时拖动复用捕获缓存。边界、双向同步、无效输入恢复、无额外采样模拟检查通过；尚未重建安装器，0.1.0。

- 最新交付（本轮“可以编译”按上下文编译 PS 安装器）：占用检查通过，Release成功，同路径EXE 5168640字节，SHA-256 `91928024ba85c3d85b009471894c76f3f310a5dfe4d4e9494969ca158ef5b9d9`。包含棋盘格固定图块、刷新恢复、网格对齐、Slot圆角和宽高前置，保持0.1.0。5项Rust测试及图标、PNG、模拟宿主检查通过，未自动安装，实际PS待复验；主程序尚未重新生成。
- 最新手动ZIP十文件16023字节，SHA-256 `1d93aae4287627ff49f875dc68ac55fa79afd79f97fd652427472b50c19c3afd`；CCX十一文件19329字节，SHA-256 `8b0433d16269a73135ee109b404f01e147ced46951bc07b11af35439adaedc7d`。以下旧哈希和未构建标记为历史记录。

- 用户确认“卡死”为插件状态异常，PS 未卡死。源码 refresh 统一捕获背景／状态校正异常，start 先建立轮询，mode 保持上次有效选择，reconcileControls 按配置变化修正显示；空文档记录 lastKey。模拟背景故障后恢复、无 change 配置更新、无效下拉保留模式及重新开文档通过；未确认实机确切触发点，未编译新安装器，0.1.0。后续交付需包含棋盘格、网格对齐、圆角、尺寸前置及此恢复逻辑。

- 最新 PS 棋盘格源码修复：新增 preview-stage／preview-background 底层，固定256px PNG图块平铺（棋盘格8逻辑像素），移除旧全视口缩略背景拉伸。复用节点、缩小回收、颜色模式隐藏，主预览节点不受影响。非整倍数视口和像素检查通过，真实PS待验证；现有EXE不含此修改以及格线对齐、Slot圆角和宽高前置调整，保持0.1.0。

- 最新 PS 布局源码将宽高输入及预计画布尺寸移到循环行列之前（带高度选项下方），避免新建操作上方找不到尺寸。模拟宿主检查通过；尚未重建安装器，后续与网格对齐、圆角修复一并打包，版本0.1.0。

- 最新 PS Slot 样式源码：固定2px浅色描边和13px圆角，蓝色选中／深色未选中，避免点击后默认边框消失；模拟检查覆盖选中／取消样式，实际PS待验证。尚未重建安装器，需与网格对齐修正一并打包，版本保持0.1.0。

- 用户最新实机反馈并截图确认 PS 折叠菱形图标已显示，图标该场景验证通过；先前“图标未验证／空白”为历史状态。其他主题／DPI及最新网格对齐修正仍待验证，版本不升级，保持0.1.0。

- 最新 PS 源码修正网格偏移：previewGridImage 不再按屏幕尺寸／宽高减一绘制，改用来源采样尺寸、与 PSD 共用 gridRaster，缩放时复用同一 PNG。新增2×2／3×5、0.5／1／2／4倍率逐像素比对，通过；实际 PS 待验证。现有安装器不含此修正，插件仍0.1.0，未重建。

- 最新主程序源码支持工具栏 HEX 颜色输入，#RRGGBB 保留 Alpha，#RRGGBBAA 可设置 Alpha，非法内容不修改颜色、失焦恢复；拾色器及 Alt 取色同步显示。cargo check --tests 通过；与布局／滚轮／关闭修复一起等待后续授权生成主程序。

- 最新主程序布局源码：按标注将倍率／网格放到各区标题旁，新增独立 preview_grid 状态；绘制工具集中到绘制区顶部 drawing_tools，可换行，左侧仅保留配置与文件／工程。cargo check --tests 通过；主程序未重建，后续需连同关闭弹窗与滚轮缩放一起验证。

- 最新主程序源码：绘制区及循环预览支持悬停滚轮缩放（独立预览同样支持），绘制区鼠标锚点、预览中心缩放，分别保持现有范围。cargo check --tests 通过；未生成主程序 EXE，需与关闭确认弹窗修复一起构建后验证。

- 最新主程序源码修复：用户反馈像素 Tile 点 X 无提示，截图实际左设置栏内有被裁切的行内确认。已改为主 Tile 视口中央 egui::Modal，继续／放弃关闭两按钮竖排，Esc／遮罩取消，关闭请求主动重绘。cargo check --tests 通过；未生成新主程序 EXE，实际窗口行为待验证。PS 安装器不受此项主程序修改影响。

- 最新安装器交付（用户明确授权）：先检查进程关闭和目标 EXE 未占用，Release 成功。路径仍为 `plugins/photoshop-isometric-tile/installer/target/release/ps-tile-installer.exe`，5167616 字节，SHA-256 `490e4566ede93dc2c2e9e30aa898d8005aab4f4c2f50fb7b86b3e81bb35c9780`。包含上述背景／辅助网格／Slot 缓存／图标兼容修改，版本保持 0.1.0。5 项 Rust 安装器测试及图标、PNG、模拟宿主检查通过；未自动安装，PS 实机仍待验证。
- 最新 0.1.0 手动 ZIP 十文件、15690 字节，SHA-256 `111e5a8a82eb99495615151509a3420c44efef251165edb5757858b06c04036d`；CCX 十一文件、18996 字节，SHA-256 `18f062769fcad33660f2b4677dd72e239a692f7e2915a453c95bab2809d9db90`。下文尚未编译或其他哈希均属历史构建。

- 最新用户实机反馈：折叠图标仍空白，不能标记修复完成。源码改 generic 图标用途，插件级同步备用 icons，补 tile-dark@1x.png／tile-light@1x.png（与基础图字节相同），安装／打包列表十文件，图标检查及 cargo check 通过。未确认根因，未生成包含此调整以及背景／预览网格／Slot 缓存修复的新安装器，版本保持 0.1.0。

- 后续 Slot 闪烁排查源码修复：slotUI 按排列形状复用按钮，隐藏模式不刷新文档下拉，重复绑定 change 不触发刷新；capture／refresh 缓存采用网格回滚后的 history ID，防止循环模态采样。新增模拟回滚改变 ID 后五轮无重复捕获、Slot 原节点选中／取消选中测试，通过。实际 PS 待验证，现有 EXE 不包含本轮修复及预览背景／网格，版本保持 0.1.0。

- 最新源码新增预览辅助网格开关 show-grid（默认关），单／多 Slot 显示地面菱形、单文档依实际行列分格，透明黄色 PNG 叠加并鼠标穿透，不影响 PSD／导出；与标签开关独立。模拟宿主测试通过，含 3×3 单文档及不额外 getPixels；尚未生成包含背景与此网格功能的安装器，插件继续 0.1.0。

- 后续源码新增预览背景选择，默认棋盘格，自定义颜色用 #RRGGBB 输入；仅当前会话，不影响源文档和 PNG。previewBackground 使用至多 512px 全视口 PNG，绕开 UXP background-repeat 不支持，背景切换不重建前景或调用 getPixels。模拟测试通过，实际 PS 待验证；当前已交付 EXE 不含此背景功能，尚未重新编译。

- 最新交付：用户授权后检查安装器已关闭且 EXE 未占用，Release 重新生成，插件版本仍为 0.1.0，包含 @2x 图标命名、预览节点复用和部署状态提示修正。安装器路径不变，5161984 字节，SHA-256 `4a53519cc317331e26663a1b6562037feff47e3a5335d3aa3cff8c944d1df34f`；5 项 Rust 测试及图标、PNG、模拟宿主检查均通过。
- 最新手动 ZIP（13834 字节）SHA-256 `225aedfb84d34f4e483b4a596447d7ffb01ac9cce523d29454ecde49edb1a4b7`；CCX（16956 字节）SHA-256 `dd939ae5cf3eae8cc599c84f20b84c19e4ff786a380ae676f45f1ba5457cd0da`。实际 PS 图标和预览闪烁待用户复验，未自动安装。以下“尚未编译”和旧哈希为先前状态。

- 用户确认 PS 可打开插件，当前待解决为图标空白与预览闪烁。源码改用 tile-dark.png／tile-dark@2x.png、浅色对应文件，manifest scale [1,2]；安装器嵌入列表、打包列表及 README 同步。新增 icon-check.cjs 校验实际路径、尺寸、透明轮廓，通过。
- main.js 新增 commitPreview 复用已挂载图片，两个预览模式不再清空整个 DOM；采样失败保留上一帧。模拟宿主、图标、PNG 检查全部通过，实际 PS 待复验。用户明确“稍等编译”，未生成新 EXE；当前 EXE 尚不包含安装提示、图标命名和节点复用修改。dist 0.1.0 图标修复 ZIP 是闪烁修复前的中间包，后续交付前需要重新打包。版本继续保持 0.1.0。

- 卸载重装排查：实际 `%APPDATA%/Adobe/UXP/Plugins/External/unityyamlviewer.isometric-preview` 八个文件哈希全部匹配当前源码，manifest 为 0.1.0，部署已成功；PS 加载结果未确认。已询问用户重新启动后的增效工具菜单情况。安装器当前进程仍运行（未关闭它），只改源码提示区分部署与宿主加载，未重新构建；现有 EXE 尚不含该文案修正。

- 最终测试版本按用户要求固定为 0.1.0（覆盖先前 0.1.5 约定）。为排查无法放大，清单明确 maximumSize 7680×4320，未锁定为固定大小；实际拖拽仍待 PS 验证。模拟检查通过，安装器重新生成且保留模态修复及菱形图标，SHA-256 `ca2f1b3b2680d14bd8ba0e58d130483a57c173c5e7f107e6d2d9d08f0dda903d`。0.1.0 手动包 SHA-256 `6af7fa0ec17f40f9af45fa087366c98c8405dccda6c0dd2fa5f12b54a720a39e`；未自动安装。

- 最新用户约定：测试未确认通过前不升级版本号。插件恢复并固定为 0.1.5，保留全部修复和图标；AGENTS.md 已记录。同路径安装器重新生成，SHA-256 `e13fb02357613003b66b566633d09f89cc0523ebc8c9b331a92d3d0b1ff6dc2e`。当前 0.1.5 手动 ZIP 为 13603 字节，SHA-256 `2e166a76e23100839f4efce27685cb83bc23999fea7c388186d080f6f2466437`。先前 0.1.6／0.1.7 及旧 0.1.5 构建哈希均是历史记录；单看面板版本无法区分本轮测试修订。

- PS 实机反馈 v0.1.5 普通 PNG 预览报模态作用域错误；已在 0.1.7 修复 withoutGrid 无辅助层直接读取分支，改为所有预览捕获均进入 executeAsModal。模拟宿主严格检查所有 getPixels 处于模态且不嵌套，全部通过；PNG 检查通过，真实 PS 效果待用户复验。
- 最新同路径安装器内嵌 0.1.7（含菱形图标），重建前脚本占用检查通过，Release 成功，SHA-256 `d49f48171dab410e770edd5987fcc3e1655590a988556b6dda18bcefb22acbcb`。0.1.7 四张图标加四个代码文件的手动 ZIP 为 13603 字节，SHA-256 `74a5c3abd4b997096a839eee07a5ff233033c8a76aade568a6f1dc9b3b24c4bc`；未自动安装。以下 0.1.6／0.1.5 哈希为前次构建。

- 后续图标更新：插件 0.1.6 新增透明空心等距菱形折叠面板图标，深浅主题各 23／46px；源图由 `plugins/photoshop-isometric-tile/generate-icons.cjs` 确定性生成。运行文件从四个增为八个，安装器和两种包均同步，PS 实际显示待用户验证。
- 同路径安装器已重新构建，内嵌插件为 0.1.6，最新 SHA-256 `f267d4b08996cb6cb0a97887fa3de518c0d1832f5a3aab0358fd5dc64ecaaced`。5 项安装器测试及模拟 PS 检查通过，未实际安装。0.1.6 手动 ZIP（13556 字节）SHA-256 `edd406b2512b9e6b1f3c9c1a6c2608c70ade924dda35d53eadf56b08c6d4fe70`；CCX（16571 字节）SHA-256 `87f65070700ee5d145f6adab628d5d112a3371b5af4b56e73dc2b74af16150bc`。以下安装器 0.1.5 数据为前次构建记录。

- 用户明确要求编译主程序和 PS 安装工具。构建前检查两程序未运行、主程序目标可独占打开、安装器目标不存在；两个 Release 构建成功，未自动启动、安装或发布。
- 主程序 1.3.6：`target/release/unity_yaml_viewer.exe`，33721344 字节，SHA-256 `6e4051397f627e3eeaba9d957d2e885dbf107b589d88c1aeae4209f8f91d1831`。
- 安装器 0.1.0，内嵌插件 0.1.5：`plugins/photoshop-isometric-tile/installer/target/release/ps-tile-installer.exe`，5140992 字节，SHA-256 `032a989460936325c975f3ad521369abf386e8e352f209d3be697d7b504a3601`。Windows EXE 头及版本资源核对通过。
- 主程序像素 Tile 9 项 Rust Release 测试、安装器 5 项 Rust Release 测试全部通过；PNG 编码和模拟 PS 宿主检查再次通过。安装器测试仅修改临时目录，未修改真实 Adobe 目录。
- 构建日志：`target/pixel-tile-release.log`、`plugins/photoshop-isometric-tile/installer/target/release-build.log`；测试日志：`target/pixel-tile-tests.log`、`plugins/photoshop-isometric-tile/installer/target/tests.log`。编译有既有依赖／无用方法警告，无错误。
- 用户待验证：主窗口三列、单／多格菱形画笔跨边、独立预览多屏 DPI 与重绘、PS 安装更新卸载和插件真实行为。安装／卸载前需关闭 Photoshop，安装器不会主动关闭它；PS 原生画笔菱形回绕仍未实现。

- 像素 Tile：互斥工具 Tab、B/I/E/U/G、Ctrl+Z/Y、[ / ] 笔宽 1–32、空格左拖及右拖平移、工程按钮置顶。cargo check --offline --locked --tests 通过，未生成新 EXE，交互待验证。

- 像素 Tile 复用独立拾色窗口，工具栏 HEX 替换为 Alpha 0–255 滑条；窗口内仍可输入 HEX。新增 editable 调用保留其他资源拾色的预览／编辑权限。未重新生成 EXE，UI 待验证。

- PS 安装器错误状态新增独立标志，PS 占用等失败提示加粗标红；重试清除错误样式。不改变进程检查，未生成新安装器。

- PS自动回落：paint事件200ms＋新画笔历史稳定500ms兜底，识别用户截图“画笔工具”；排除既有历史undo/redo、插件自身回读／回落和文档／图层切换。新增auto-wrap-check.cjs。未生成安装器，Wacom实测待验证。

### 2026-11-17 本轮双程序构建

- 用户明确授权后，检查两个固定目标进程均未运行，EXE 独占打开检查通过，再生成 release 产物；主程序1.3.6、PS插件／安装器0.1.0，不递增测试版本。
- 主程序：`target/release/unity_yaml_viewer.exe`，33622528字节，SHA-256 `e08396595cfb7c629d529b41073fa1d1d0e7977fd79ea9c27e46c8f22e5f96ca`。
- 安装器：`plugins/photoshop-isometric-tile/installer/target/release/ps-tile-installer.exe`，5199360字节，SHA-256 `a9fb56816c0aced781206c16b4e584ed0197ec74c31e40a2e75979a46f35c8a0`。对应.sha256已更新。
- 验证：像素Tile测试11项、安装器测试5项通过；Node自动回落历史、回落像素、面板转换与布局逻辑、PNG及图标检查通过。构建日志：`target/tile-ui-release.log`、`target/tile-installer-release.log`；测试日志：`target/tile-ui-tests.log`、`target/installer-tests.log`。
- 新EXE包含本轮工具快捷键、笔宽快捷键、平移、独立拾色器／Alpha、关闭弹窗及菜单排序；安装器包含历史兜底自动回落、预览设置集中、输入框对齐和安装错误标红。Wacom实际触发、PS文字与布局、主程序新增UI仍待用户实测；未发布。

- 像素Tile坐标格选择＋下方导入／清空，默认独立；去掉Tile素材下拉／添加空白／独立副本按钮。旧共享工程载入时分离，超预算拒绝且不改当前数据。适应画布已移到绘制区标题行。新测试仅编译检查，待授权构建／运行。

- 本轮新增像素Tile图层（基底默认锁定、独立绘制层、缩略图、≡拖序、右键删除／重命名、右侧锁与显隐），.uytile格式3、可见层合成及多格逐格PNG、2/3/4快捷预设；修复关闭确认重复居中原生窗口（retain_on_close_request＋CancelClose）。仅cargo check --tests，新增测试未运行；待授权编译和用户UI验证。

- PS背景色块新增原生调色板按钮，modal＋batchPlay showColorPicker，HEX／背景同步；取消不改、错误提示。color-picker-check.cjs及panel-check.cjs通过，PS实际兼容待验证，未打包。

- 用户确认PS历史兜底自动回落有效，但报告左下→右上部分像素缺色。已修复自动任务使用笔触前基线ID（连续笔触保留最早）、outside合成结束后重新同步周期边界副本。wrap-check新增软笔跨边RGBA一致性，auto-wrap-check新增基线断言；缺色修复待实机，未打包。

- PS自动回落增加文档ID绑定、显示绑定名／ID及手动改绑按钮；切其他文档暂停、关闭不改绑、取消勾选解除。事件／轮询／执行前均守卫，跨文档mock回归通过；尚未打包。

- PS自动回落重开恢复：按完整已保存文件路径匹配新ID（非文件名），网格校验、清队列与重建基线；未保存／多候选需手动绑定。修复执行前恢复导致空队列访问的边界。Node回归通过，尚未打包／PS实测。

- PS本次工作显式绑定后后台回落；收起仅停预览，销毁／取消勾选才停回落。独立wrapTimer、连续笔触不重置等待起点、错误9保留重试、最近执行信息，后台完成不展开面板。auto-wrap-check覆盖隐藏与重试等；未打包，Wacom真实持续绘制稳定性待验证。

### 2026-11-17 最新双程序构建（图层与后台回落）

- 用户明确授权，主程序与安装器固定目标均通过进程／独占占用检查。主程序仍为1.3.6，PS插件及安装器仍为0.1.0。
- `C:\Git\UnityYAMLViewer\target\release\unity_yaml_viewer.exe`：33888256字节；SHA-256 `0aded8ce0cda65e8b9b0e1a6390c177938075e88e2dbe6a162bc97c701690de8`。
- `C:\Git\UnityYAMLViewer\plugins\photoshop-isometric-tile\installer\target\release\ps-tile-installer.exe`：5207552字节；SHA-256 `ab362dab630916f4f681178e319d3a252a348b346d9b81ac2caa27c48f345356`。
- 对应.sha256已更新；主程序构建日志target/tile-layers-release.log，安装器target/tile-background-installer-release.log。
- 像素Tile测试14项通过（图层锁保护／Alpha合成／橡皮撤销、坐标格独立修改、3×3逐格导出及工程层结构等）；安装器测试5项通过。Node自动回落（隐藏生命周期、忙碌重试、绑定／重开及连续笔触）、边界像素、面板、调色板、PNG、图标检查通过。测试日志target/tile-layers-tests.log及target/tile-background-installer-tests.log。
- 本次产物包含本轮全部源代码调整；之前条目“未生成新EXE／安装器”为当时状态。主程序图层拖拽、右键／锁、光标／Alpha／关闭定位，PS Wacom持续绘制、收起后台／同文件重开／缺色修复及调色板实际行为仍需用户验证；未发布。

- 本轮像素Tile：竖向悬浮六工具＋L直线、方框笔宽快捷与滑条、图层下背景（棋盘格／自定义色块＋HEX）、.uytile可选背景字段。仅cargo check --tests，新增直线测试待运行，未生成EXE。

- 2026-11-18 像素Tile图层圆点替换眼睛／eye-off SVG与独立点击区，移除≡手柄，名称拖拽排序。新增显隐点击与composite验证测试（仅check编译），锁点击测试也待运行；未生成EXE。

- PS 单文档模式加入循环错位预览和子组＋整体联动回落设置：子组尺寸按列／行整除整体，Slot选择确定当前组，预览网格区分并高亮子组；子组同位边界联动到其他组。多文档错位预览保留。更新插件README、使用指南、1.3.6记录和七语言关于摘要。
- 当前仅修改源码与文档；未运行测试／静态检查，未打包插件或生成安装器，测试版本仍为0.1.0。需后续检查分组有效组合、PS像素边界和错位布局，尤其验证历史基线读取及自动回落实际行为。

### 2026-11-18 PS子组安装器已构建

- 用户明确授权后完成占用检查和固定目标release构建；安装器及插件0.1.0。产物：`C:\Git\UnityYAMLViewer\plugins\photoshop-isometric-tile\installer\target\release\ps-tile-installer.exe`，5,218,304字节，SHA-256 `eee038356fc66f1c21036272a4dd07b1bb98d1598f200ab8941e2c1bdb6d7f90`，相邻.sha256已更新。核对10个内嵌运行文件逐字节匹配当前源码。
- 编译前补齐模拟宿主新增控件／selected状态；新增subgroup-check.cjs及单文档错位／网格预算面板检查。七个Node脚本全部通过。双向错位预览使用缓存逐格映射，固定格点无空洞；子组网格缓存最长边512。无效配置时保留子组输入入口。
- 日志：`target/ps-subgroup-checks.log`、`target/ps-subgroup-installer-build.log`，核验：`target/ps-subgroup-installer-artifact.json`。未启动安装器或修改Adobe目录，PS2024／Wacom实际行为仍待实测；主程序未重建，先前像素Tile主程序源码改动仍不在旧EXE中。

### 2026-11-18 PS中心区域源码调整（未重编）

- 单文档网格支持更大画布，中心整数偏移统一用于预览、导出及当前层普通／子组回落；新建仍为最小尺寸，画布可中心扩展。参考内容放其他层，关闭自动回落后可编辑参考层，恢复时重新绑定；不引入同层参考内容识别或额外历史要求。
- workspace-check、wrap-check、panel-check、subgroup-check、auto-wrap-check通过。新增中心映射、奇数画布、逐格读取和未选中参考层不变检查；更新指南、插件README、版本记录及七语言关于摘要。PS实机行为待验证。
- 版本仍为0.1.0，本轮未重建安装器；上节5,218,304字节EXE及其SHA-256仍是现有产物，不包含本次中心区域修改。主程序也未编译。

### 2026-11-18 PS图层名单／扩展文档（未重编）

- 自动回落改为当前工作文档中累计记录图层ID。绑定／解绑按钮随活动层变化；暂停保留名单，解绑只删当前层，后台与重试均检查名单，写回前再次确认。跨文档主动绑定更换工作文档；重开同路径仅同ID层恢复，复制层不继承，重载插件清空名单。
- 文档设置可折叠，新建按钮简化，增加“扩展文档”。按用户确认原1,1保留为1,1；格子尺寸固定，新增行列扩展画布并保持原四周余量，通过两次原生画布补边保持旧格位置与新网格居中。重写辅助层，保留绘画层ID／像素／绑定，单步历史，失败回滚。不会自动复制Tile或重排参考图。
- 九个Node检查脚本通过（新增expand-check.cjs并扩充auto-wrap-check和wrap-check），功能／限制／验证写入1.3.6记录，七语言摘要和指南同步。仍需PS原生UI及Wacom实测。
- 本轮未生成EXE，现有安装器仍为上节5,218,304字节旧产物，不含中心区域、图层名单与扩展文档这些后续源码改动；插件测试版本保持0.1.0。

### 2026-11-18 最新PS安装器（图层绑定／扩展文档）

- 用户明确授权后通过进程与固定目标独占访问检查，release构建成功。`C:\Git\UnityYAMLViewer\plugins\photoshop-isometric-tile\installer\target\release\ps-tile-installer.exe`，5,226,496字节；SHA-256 `8b35402511bb519faf11ed9d0e6c79acc9c892779ce969f53a8df86e40dfbea3`，相邻.sha256已更新。
- 10个内嵌运行文件逐字节匹配当前源码；插件清单、EXE文件及产品版本均0.1.0。包含大画布中心区域、图层累计绑定／逐层解绑、可折叠文档设置及保留旧格位置扩展，前文“未重编”为当时状态。
- 日志`target/ps-layer-bind-expand-installer-build.log`，核验`target/ps-layer-bind-expand-installer-artifact.json`。九组Node检查已在源码修改轮通过，本轮只构建与核验。未运行安装器、未修改Adobe目录、未发布；主程序未重建，PS与Wacom实际操作仍待用户验证。

### 2026-11-18 PS折叠布局／加载文档入口（未重编）

- 修复侧栏折叠分组与按钮的布局、默认单文档格子控件初始化；提前注册宿主入口并延后执行UI工作，绑定读取异常不再阻断设置和预览。标题下新增原刷新按钮的位置，改名“加载当前打开文档”，仍只重新读取预览。
- 无文档时右侧提示打开或新建，扩展按钮禁用；关闭最后一个文档清除旧预览，暂停自动更新也生效；打开后恢复扩展入口。源代码保留扩展时的再次检查。
- panel-check、expand-check、auto-wrap-check、wrap-check通过。更新1.3.6记录、七语言关于摘要及使用指南；真实PS折叠／缩放布局尚未验证。未改动安装目录、未生成EXE、未发布。
- 当前安装器仍为上节5,226,496字节、SHA-256 `8b35402511bb519faf11ed9d0e6c79acc9c892779ce969f53a8df86e40dfbea3`，不包含本节源码修复；主程序未重建。后续编译需用户明确授权并重新检查目标占用，插件版本保持0.1.0。

### 2026-11-18 PS的1×1 Slot区简化（未重编）

- 1×1省略Slot选择／当前格／坐标开关，不显示预览标签；多格坐标默认关闭，可手动开启，临时缩成1×1保留多格偏好。仅隐藏独立选择分组，绑定、导出和回落入口保留。
- panel-check通过，使用指南、版本记录与七语言关于摘要同步，PS实机布局待验证。本次未生成EXE，现有安装器不包含这项和前述折叠布局修复，插件仍0.1.0。

### 2026-11-18 PS扩展文档锁定错误（未重编）

- 用户给出扩展失败的受保护图层错误。现在先临时解锁并选中插件辅助层，再扩展画布／更新网格，最后重新保护辅助层并恢复原选择；不解锁其他层。新增目标画布尺寸核对与失败步骤提示，操作状态移到文档设置分组下方。
- expand-check、panel-check、auto-wrap-check通过；增加真实尺寸2×2→3×3、锁定层／多选、局部锁、画布和写入失败的事务回滚检查。PS真实行为尚待用户验证，未编译、未安装或发布。
- 插件仍0.1.0，现有5,226,496字节EXE仍不包含本次及之前未重编改动；最新已构建安装器哈希仍为 `8b35402511bb519faf11ed9d0e6c79acc9c892779ce969f53a8df86e40dfbea3`。

- 后续文案调整：“自动更新”改为“自动更新平铺预览”，收笔时机说明放到title悬停提示；未改功能逻辑，未编译安装器，PS实机文字与悬停待验证。使用说明、1.3.6记录与七语言摘要已同步。

- 后续Slot布局调整：两个单文档PNG导出按钮紧接Slot坐标显示开关，位于错位和子组设置之前；1×1隐藏选择区时仍保留导出，其他模式不显示。panel-check与静态检查通过，PS实际排版待验证；版本仍0.1.0、安装器未重编，现有EXE不包含此调整。

### 2026-11-18 PS预览设置／绑定回落布局（未重编）

- 错位预览移到右侧顶部，与网格和背景设置一行显示；模式和偏移参数横排，窄窗横向滚动。只在开启错位时显示第二行说明；无文档或预览错误不影响控件切换。
- 左侧绑定／解绑按钮下依次为自动回落、子组＋整体回落及其参数；导出保留在Slot旁。刷新、偏移和回落功能逻辑不变。
- panel-check与静态差异检查通过，指南、1.3.6记录与七语言摘要同步。PS原生排版仍待验证，版本0.1.0不变；未编译或安装，现有5,226,496字节EXE不含本节及前述待编译修改。

- 随后按用户更正，回落区不再新增固定说明：去掉原重复说明／网格绘画提示，隐藏普通检测信息和次数，已有启用操作提示统一为“自动回落只在绑定的图层生效”；绑定对象、当前层和红色错误保留。panel-check、auto-wrap-check通过，文档同步，未重编。

### 2026-11-18 最新PS安装器（布局与扩展修复）

- 用户明确授权编译，进程与固定EXE独占访问检查通过，build.ps1离线release成功。`C:\Git\UnityYAMLViewer\plugins\photoshop-isometric-tile\installer\target\release\ps-tile-installer.exe`为5,231,104字节，SHA-256 `be575be5801f4783c5b7aff47868d2e01c4030570a2632a27f6e8348e8eca698`，本地.sha256已更新，版本资源和插件均0.1.0。
- 10个内嵌运行资源逐字节核对一致；已包含上次安装器后的全部PS源码调整，包括锁定辅助层扩展、折叠和缺失设置修复、加载入口、无文档提示、1×1简化、Slot导出、右侧错位设置与绑定／回落说明精简。上方未编译描述是各轮当时状态。
- 日志`target/ps-panel-layout-installer-build.log`，核验`target/ps-panel-layout-installer-artifact.json`。此前相关Node检查通过，本轮未重复测试或自动运行安装器；未修改Adobe目录、未发布。主程序EXE没有重编，PS真实UI与扩展仍待用户验证。

### 2026-11-18 PS启动与双面板源码（未重编）

- 实际PS2024日志确认entrypoints.setup因plugin.create缺失而抛错，已补齐并添加红色启动失败提示。保留0.1.0，未触碰Adobe安装目录。
- 设置与预览分为tilePreview／tilePreviewView两入口；旧入口变为窄设置面板，模式选择放入可折叠文档设置。加载会打开预览，预览可回到设置。显示面板通过Plugin Manager的自身ID调用，需要manifest IPC权限；关闭使用PS原生×，首次浮动需用户拖出标签。
- 所有控件保留直接引用；设置隐藏不停止独立预览，预览隐藏停止读取，面板关闭不主动清除绑定，插件仍运行时自动回落继续；整体插件退出／重载才清理。panel-check、auto-wrap-check、expand-check、wrap-check和icon-check通过，真实UXP多面板行为待用户验证。
- 本轮只修改源码／文档和执行Node检查，未编译或安装。现有安装器仍为5,231,104字节，SHA-256 be575be5801f4783c5b7aff47868d2e01c4030570a2632a27f6e8348e8eca698，不含启动修复和双面板。下次明确编译授权后，先检查固定EXE占用，再执行installer/build.ps1，核对10个嵌入运行文件；不递增版本。

### 2026-11-18 最新PS安装器（初始化修复与独立预览）

- 用户授权“可以编译”，安装器进程和固定EXE独占访问检查通过，离线release构建成功。产物：`C:\Git\UnityYAMLViewer\plugins\photoshop-isometric-tile\installer\target\release\ps-tile-installer.exe`，5,235,712字节，SHA-256 `86a95a2845178801a840bf17ffa099e85f3a98e6a0ddae847c5801a178e3b753`，插件／安装器／EXE版本均0.1.0。
- 10个运行资源内嵌内容与源码逐字节核对一致；已包含上节启动修复和双面板改动、模式收入文档设置，更新本地哈希文件。上方“未重编”表示各轮当时状态；当前EXE包含这些修改。
- 日志target/ps-dual-panel-installer-build.log，核验target/ps-dual-panel-installer-artifact.json。未启动安装器、未修改Adobe目录、未重编主程序、未发布。需关闭PS后安装／更新，再从插件菜单打开Tile设置，加载文档并验证独立预览；如PS把面板归组，可拖出预览标签独立浮动。实际PS行为待用户验证。

### 2026-11-18 PS设置面板固定尺寸（未重编）

- 设置入口tilePreview的最小／最大／建议尺寸统一280×700，设置区只纵向滚动；tilePreviewView仍自由缩放。用于处理用户要求移除右下角宿主缩放手柄，实际PS手柄显隐尚待验证，不能用静态尺寸声明代替实测。
- 静态核对通过，指南、1.3.6记录与七语言摘要已同步，版本保持0.1.0。未编译或安装；现有安装器5,235,712字节、SHA-256 86a95a2845178801a840bf17ffa099e85f3a98e6a0ddae847c5801a178e3b753不含本次调整。

### 2026-11-18 PS预览自动更新简化（未重编）

- 移除预览自动更新开关，显示时默认更新、未打开／关闭／收起时不读取预览，重开显示最新画面；自动回落仍独立运行，设置收起不影响显示中的预览。
- panel-check与auto-wrap-check通过，覆盖仅设置启动、预览关闭但设置保留、修改历史不读取、重开后自动更新和隐藏回落监测。中英文指南及七语言摘要同步；插件保持0.1.0，未编译或安装，实际PS显隐待验证。
- 当前5,235,712字节安装器（SHA-256 86a95a2845178801a840bf17ffa099e85f3a98e6a0ddae847c5801a178e3b753）尚未包含固定设置尺寸与本次自动更新调整；等待明确编译授权后再检查目标占用并构建。

### 2026-11-18 最新PS安装器（Slot编号与设置精简）

- 用户明确授权编译，进程与固定目标独占检查通过，离线release构建成功。安装器0.1.0，产物 5237248 字节，SHA-256 `ef4ceb4ccee4bc137e9cb132c2ab2afda6697e29cb9f467d96a60820074e4541`。10个内嵌运行文件与当前源码逐字节匹配，本地.sha256已更新。
- 包含固定设置面板尺寸、预览自动更新简化、TileSlot默认收起分组、标题右侧预览按钮、加载前网格校验与设置区错误提示、按钮及预览连续编号与两位数宽标记、移除重复回落提示。
- 面板及自动回落Node检查已通过；构建日志target/ps-slot-ui-installer-build.log，核验target/ps-slot-ui-installer-artifact.json。未运行安装器、未安装或发布、未重编主程序；PS实际布局与操作待用户验证。上述未重编是修改当时状态，本次产物已包含这些PS调整。

### 2026-11-18 最新PS安装器（统一错误与预览控件整理）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功。产物 5239808 字节，SHA-256 `86d6582e77944ec78cffe13742f350f8f41c50f03f92f0a30ea52dd0f7f74125`，插件版本0.1.0，10个嵌入运行文件与源码逐字节核对一致；本地.sha256已更新。
- 包含标题／版本行布局、文档设置标题、统一错误区与清除按钮、空占位清理、预览编号开关迁移、Slot分组提示与分割线、隐藏背景HEX输入；包含辅助层先选择再解锁和写入前复核、空白层透明历史基线与真实历史读取错误提示。
- 相关面板、扩展、自动回落、普通／子组回落、选色检查及透明基线针对性检查已在修改轮通过；PS实机效果仍待验证。未运行安装器、未安装、未发布，主程序未重编。日志target/ps-settings-errors-installer-build.log，核验target/ps-settings-errors-installer-artifact.json。之前“未重编”是当轮状态，本次安装器包含上述全部PS调整。

### 2026-11-18 最新PS安装器（文档临时记忆与缩放恢复）

- 用户授权后检查安装器进程及固定EXE独占访问，通过后离线release构建成功；产物 5242368 字节，SHA-256 `adcdfacca93adaf8a23490a3aa8e53085e9e56f651b7e0ca0fe64d6f1024c6d3`。插件0.1.0，10个运行文件内嵌内容与源码逐字节一致，本地.sha256已更新。
- 包含文档网格／Slot／子组／错位会话记忆、设置面板恢复可调整大小及状态轮询避免重复UI写入。面板、文档设置及自动回落回归已通过；PS抖动是否消除和实际文档切换仍待验证。
- 日志target/ps-document-settings-installer-build.log，核验target/ps-document-settings-installer-artifact.json。未运行安装器、未安装或发布、未重编主程序；先前未重编为当轮状态，本次已包含上述PS修改。

### 2026-11-18 最新PS安装器（隐藏回落像素统计）

- 用户授权后安装器进程及固定EXE独占检查通过，离线release成功；产物 5241856 字节，SHA-256 `7aed4814fbbe613eb4e796130121b7fec5a8a245226cf8fce1cd444bf77cdb05`，插件0.1.0，10个内嵌运行文件与源码逐字节核对一致，本地.sha256更新。包含隐藏回落完成像素统计及前轮全部PS改动。
- 普通回落和自动回落Node检查已通过；未运行安装器、未安装或发布、未重编主程序，PS实际显示待验证。日志target/ps-wrap-status-installer-build.log，核验target/ps-wrap-status-installer-artifact.json。

### 2026-11-18 最新PS安装器（空白层首笔历史读取）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过后离线release成功。产物 5242368 字节，SHA-256 `0a3d7e30407b3c6cc7804b00b7c9f321192c4f72c9415202298a1a3e1b59c617`，插件0.1.0，10个内嵌运行文件与当前源码逐字节核对一致，本地.sha256已更新。
- 已包含历史读取“No pixels in the requested area”按透明RGBA处理的修复，针对新建空白层→绑定→首笔绘制流程；相关历史像素／普通回落／子组检查已通过，PS实际回落仍待用户验证。
- 日志target/ps-empty-history-installer-build.log，核验target/ps-empty-history-installer-artifact.json。未运行安装器、未安装或发布，主程序未重编。上文未重编指修改当轮状态，本次安装器包含这项修复。

### 2026-11-18 最新PS安装器（路径网格试用）

- 用户授权后安装器进程及固定EXE独占检查通过，离线release成功；产物 5244416 字节，SHA-256 `f320e4cb9e5f9dd5ee65d67dad2a04c145335ff9219441be163daf57b175e22f`，插件0.1.0，10个运行文件内嵌内容与源码逐字节一致，本地.sha256已更新。
- 包含默认开启、可取消回退的路径网格新建入口，以及临时网格读取后的显式显隐恢复。路径文档不含辅助像素层，预览直接读合成，无临时显隐；旧文档方式保留，路径扩展暂不支持。
- 路径几何／不碰历史读取、面板、旧扩展及显隐恢复检查已在修改轮通过；路径轮廓持续显示、画笔和撤销重做仍待PS实测。未运行安装器、未安装或发布，主程序未重编。日志target/ps-path-grid-installer-build.log，核验target/ps-path-grid-installer-artifact.json。

### 2026-11-18 最新PS安装器（路径构造器修复）

- 用户明确授权编译，安装器进程及固定EXE独占访问检查通过，离线release成功；产物 5244416 字节，SHA-256 `fed4f68e55d85dcabede929dc59f56075fdf4732fcc8a74ca74d3102c3994f9a`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含从photoshop.app获取路径构造器的修复；路径与面板回归已通过，实际PS路径创建、绘制和撤销重做待实测。未运行安装器、未安装或发布、未重编主程序。日志target/ps-path-constructor-installer-build.log，核验target/ps-path-constructor-installer-artifact.json。

### 2026-11-18 最新PS安装器（恢复辅助图层网格）

- 用户确认保留原辅助图层及预览临时显隐方式：撤销／重做到隐藏状态时由用户手动显示网格，不增加强制显示监测。路径创建试用已撤回，已有路径文档仅保留读取兼容。
- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功；产物 5243392 字节，SHA-256 `3c3b854858e53d34be221b4eca3ccd83b0a25769d46d12ebe9c323732d4f01dd`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 路径撤回、面板与扩展回归已通过；未运行安装器、未安装或发布、未重编主程序。日志target/ps-layer-grid-restored-installer-build.log，核验target/ps-layer-grid-restored-installer-artifact.json。PS实际恢复待验证，不宣称撤销历史交互问题已消除。

### 2026-11-18 最新PS安装器（绑定联动、焦点与半透明叠加）

- 用户明确授权编译，安装器进程及固定EXE独占访问检查通过，离线release成功；产物 5244416 字节，SHA-256 `3eded5e95e800de0dbdd0231284b05e23822dfeb6850af4102f6a108dca9e317`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含移除用户可见自动回落全局开关、绑定图层自动启用、预览不主动抢焦点及控件释放焦点、子组新增半透明笔迹叠加及擦除处理。当前和历史回落读取限定当前层；用户报告的少量跨层染色尚未复现。
- 面板、自动回落、普通回落、子组及新增Alpha检查已通过。快捷键恢复、半透明真实笔迹及染色仍待PS实测，未宣称全面解决。未运行安装器、未安装或发布、未重编主程序。日志target/ps-alpha-focus-installer-build.log，核验target/ps-alpha-focus-installer-artifact.json。

### 2026-11-18 最新PS安装器（软笔合成与绑定配色）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功；产物 5245440 字节，SHA-256 `7de9c0632ff49bd8f501827cbf8ecef725a02c5a9d51e16c79eeb535a0c7b150`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含彻底移除内部auto-wrap输入控件、绑定淡绿／解绑淡红按钮，以及不透明底色上软笔增量估算。后者无法精确反推所有笔刷效果，实际接缝仍待PS测试。
- 面板、自动回落、普通回落、子组及Alpha针对性检查已在修改轮通过。未运行安装器、未安装或发布、未重编主程序。日志target/ps-soft-alpha-installer-build.log，核验target/ps-soft-alpha-installer-artifact.json。

### 2026-11-18 最新PS安装器（统一提示与圆角按钮）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功；产物 5246464 字节，SHA-256 `8bfdd868f1544f19b671ae994455d271ac0a4a512aae95f063f9bee29b7c6a59`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含绑定／解绑按钮显式圆角、普通操作提示统一至标题下方、清除所有提示信息按钮及普通提示5秒自动消失。按用户决定不添加关闭／暂停预览功能。面板回归已通过，实际圆角和计时待PS验证。
- 未运行安装器、未安装或发布、未重编主程序。日志target/ps-notice-rounded-installer-build.log，核验target/ps-notice-rounded-installer-artifact.json。

### 2026-11-18 最新PS安装器（周期位置多笔迹叠加）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功；产物 5246976 字节，SHA-256 `631842625bedf274a3da549fd0046609c426bc32b4a928e8e62faee1bb10b502`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含子组同一周期位置多个不同笔迹贡献合成，避免仅保留最后来源；用户确认在已有内容上叠画，实际缺口效果仍待PS实测。子组含周期冲突／Alpha及普通回落回归已通过，不宣称全面修复。
- 未运行安装器、未安装或发布、未重编主程序。日志target/ps-overpaint-installer-build.log，核验target/ps-overpaint-installer-artifact.json。

### 2026-11-18 最新PS安装器（隐藏自动检查统计）

- 用户明确授权编译，安装器进程及固定EXE独占检查通过，离线release成功；产物 5246976 字节，SHA-256 `38e75e85e0da9b74010d9e0099a9c945612922207239545e9d9587b645aad490`，插件0.1.0。10个运行文件内嵌内容与当前源码逐字节一致，本地.sha256已更新。
- 包含隐藏自动检查次数、像素数与历史ID；面板及自动回落回归已通过。用户实测软笔叠画接缝仍存在，决定保留当前方式，不做自动临时图层。遇到问题撤销笔迹，再新建透明图层绑定绘制，确认后合并；不得宣称透明合成已全面修复。
- 未运行安装器、未安装或发布、未重编主程序。日志target/ps-hidden-wrap-stats-installer-build.log，核验target/ps-hidden-wrap-stats-installer-artifact.json。

## 2026-11-19 PS 插件安装与更新引导

- 像素 Tile 设置顶部增加可折叠 Photoshop 插件入口。仅用户点击检查按钮时后台读取独立线上清单；打开窗口／展开区域不联网，不使用主程序 Latest。查询中禁用重复请求，同窗口至少间隔 60 秒，可重试；超时或错误保留手动下载页入口。
- 读取当前用户固定部署目录 manifest.json，并验证插件 ID 与版本；读取失败和未检测到分开提示。“已部署”不代表 PS 已加载。检查时刷新本地版本。按独立语义版本显示安装／更新／最新版／本地较新，支持主程序停留在 1.3.7 时获取未来插件版本。
- 清单校验格式版本、版本号、固定仓库发布地址与安装包名称，限制 64 KiB 和网络超时。只打开下载页，不自动下载安装。首版 0.1.0 尚未发布，清单 published=false，避免虚假可下载状态。需提交并推送清单至 master 后线上查询才可用，本轮未推送或发布。
- AGENTS.md 补充独立标签、版本记录、非 Latest Release、构建授权、附件核验和最后更新线上清单顺序；PS 0.1.0 独立记录已建立，版本保持不变。界面七语言同步。
- 实际界面、真实网络查询及 PS 安装后状态仍待运行验证；本轮不生成 EXE。

- 验证补记：cargo check --tests --locked --offline 通过（12.31 秒，target/ps-plugin-update-check.log）；新增清单地址／版本契约 Rust 用例仅编译未执行。七列翻译、七语言摘要条目数、默认两版与重开收起、未发布清单及唯一按钮请求入口静态核对通过。未运行测试 EXE、未运行应用或联网查询。

# PS 0.1.0 发布准备（2026-11-19，等待用户上传 Git）

- 用户要求只先发布 PS 插件，主程序 1.3.7 暂不发布；本轮仅准备，必须等待用户明确通知 Git 已上传后才发布插件。
- 复用 2026-11-18 已授权构建的安装器，未重新编译；版本／产品版本均 0.1.0，5246976 字节，SHA-256 38e75e85e0da9b74010d9e0099a9c945612922207239545e9d9587b645aad490。十个运行文件均在 EXE 中逐字节匹配当前源码。
- 候选 target/releases/ps-v0.1.0/PS-Tile-Installer-v0.1.0-windows-x64.exe；本地校验、核验 JSON 及回归日志同目录保存，不上传重复校验附件。
- 发布正文 plugins/photoshop-isometric-tile/docs/releases/0.1.0.md；计划标签 ps-v0.1.0，标题 v0.1.0，正式发布且 make_latest=false。不得创建主程序 v1.3.7 Release 或改变现有主程序 Latest。
- 发布时先核对用户推送提交及运行文件与候选一致，再上传核验 EXE；最后将 updates/ps-tile.json 改为 published=true 和准确发布地址并推送 master。当前清单仍 false，不提前声称可下载。用户本地通过主程序按钮手动检查，不自动联网。
- 尚未创建标签／Release、未上传附件、未推送。

- 准备核验完成：13 项插件 Node 回归检查全部通过，候选十个内嵌运行文件逐字节匹配；source-manifest.json 保存发布输入哈希供 Git 上传后复核。未执行安装或主程序发布。

## 2026-11-19 像素 Tile 默认移动工具

- 绘制区竖向工具栏在画笔上方新增四向箭头“移动（V）”，新编辑器默认选中。左键拖动平移绘制视图，不修改像素或新增撤销记录；B 切换画笔，原空格＋左键／中键／右键平移保留。
- 移动状态禁止可编辑循环预览落笔；独立只读预览保持只读。移动不受当前层显隐／锁定限制，悬停为抓手、拖动为抓取光标；输入文字时 V 不抢占，切换工具先结束已有笔触。
- 新增 SVG 图标，七语言移动文案及关于摘要、中英文指南同步；主程序仍 1.3.7。实际鼠标操作与工具栏外观待运行验证，本轮未生成 EXE、未发布。

- 移动工具验证：cargo check --tests --locked --offline 通过（target/pixel-tile-move-check.log）；既有工具快捷键用例补充默认移动及 V 选择，仅编译未运行测试 EXE。git diff --check、七语言摘要条目数及翻译列数检查通过。实际拖动及预览防误画待运行验证。

## 2026-11-19 PS 安装包下载、启动与定位

- 插件检查改为比较本程序管理的已下载安装包，不再读取 PS 部署目录。有效本地记录须通过版本、大小与 SHA-256 检查；记录不可读、文件丢失或损坏时按未下载处理。本地等于或高于远端时不显示下载按钮，保留打开安装器与定位目录。
- 用户点击下载后后台读取对应 Release 的附件元数据，限制同仓库固定版本／附件名、64 MiB 上限，下载进度可见；文件大小与 GitHub digest 校验通过后才写入本地版本记录。临时下载失败清理残留并允许重试，不将失败下载标记完成。
- 安装包保存在应用配置目录 ps_plugin_installers 下，完成后仅启动 EXE，不代替用户安装或更新、不关闭 PS。完整路径可见，可再次打开安装器或用 Explorer 定位文件；每次打开前核验文件。用户另行手动下载的文件不扫描纳管。打开失败保留下载记录，可再次打开或手动处理；文件失效则刷新为未下载。
- 打开界面无主动联网，检查仍须点击且有 60 秒间隔；只有明确点击下载才请求 Release API 与 EXE。手动下载链接指向对应版本页面，远端查询失败保留总下载页。
- AGENTS、中英文使用指南、能力概览及七语言关于摘要同步；主程序仍 1.3.7。未编译 EXE、未安装／启动安装器、未改线上发布；真实网络下载、EXE 启动与目录定位待运行验证。

- 下载入口验证：cargo check --tests --locked --offline 通过（target/ps-plugin-download-check.log），新增本地缺失／旧版／同版／新版／无效版本及非法缓存路径用例仅编译未执行；修正既有清单用例不再依赖线上清单未发布状态。git diff --check、七语言摘要条目数与七列翻译检查通过。未生成或运行测试 EXE，实际下载、启动、Explorer 定位仍待用户运行验证。

## 2026-11-19 透明底色填充与图层按钮交互

- 填充保留原闭合区域同 RGBA 四邻域行为；点击 Alpha=0 的透明区域时，忽略隐藏 RGB 差异，将菱形／矩形有限边界视为边界而非未闭合错误，填充连通空白。已有颜色不覆盖，闭合圈内外仍按连通性区分；仅当前可编辑层生效，支持整次撤销／重做，原大小限制及带高度模式限制保留。
- 图层眼睛／锁按钮改为仅分配布局空间后注册独立点击区域；移除包围整行的重复右键菜单交互，避免整行覆盖子按钮。名称右键重命名／删除及名称拖拽排序保留；显隐、锁定不要求先选中图层。
- 新增透明菱形底色、隐藏 RGB、已有半透明笔迹保留与撤销／重做用例，更新未闭合透明区原拒绝用例；既有眼睛及锁双向点击用例保留。实际点击、填充显示待运行验证。主程序仍 1.3.7，本轮未生成 EXE、未发布。

- 填充／图层检查：cargo check --tests --locked --offline 通过（4.87 秒，target/pixel-tile-fill-layer-check.log），仅既有依赖警告；Rust 用例仅编译未运行。git diff --check、七语言摘要条目数和七列翻译核对通过；未运行 GUI，不将按钮命中区调整视为已通过用户实测。

## 2026-11-19 数位板拖拽缩放

- 工具栏填充下方新增放大镜“缩放（Z）”。绘制区落笔／左键按下后，以初始落点为中心，水平向左缩小、向右放大，松开结束。按相对初始落点的总位移计算指数缩放，与帧率无关，竖向位移不改变缩放；0.25–64 倍限制，拖到极限再返回可恢复。
- 进入缩放模式不修改像素，不增加撤销记录；循环预览禁止落笔。保留空格／中键／右键临时平移和滚轮缩放，缩放拖拽期间不叠加滚轮。普通 Z 选工具，Ctrl+Z 保留撤销，文本输入不抢快捷键。
- 复用现有放大镜 SVG，七语言提示与关于摘要、中英文使用指南同步。实际数位板笔尖按下／拖动／抬起、鼠标缩放中心及光标待运行验证，主程序保持 1.3.7，本轮未生成 EXE。

- 拖拽缩放验证：cargo check --tests --locked --offline 通过（target/pixel-tile-pen-zoom-check.log）；新增方向、上下限与落点几何不变用例，并将 Z 纳入快捷键用例，仅编译未执行 Rust 测试。git diff --check、七语言摘要与翻译列数核对通过；实际数位板与界面仍待运行验证。

## 2026-11-19 循环预览独立平移

- 循环预览支持移动工具 V＋左键拖动，及空格＋左键／中键／右键临时平移；独立于绘制区视图偏移。平移不修改像素或新增撤销，平移开始结束已有笔触并清除预览笔迹连接，避免平移后跨位置连线。
- 显示原点同步用于平铺、网格、坐标命中、取色、带高度遮挡判断、笔刷指示与形状预览；可编辑预览继续支持原绘制操作。可见格搜索围绕平移后的视口重新计算，保留原数量上限，避免移出初始原点后整片空白。
- 独立预览仍只读，但可平移；停靠／分离共用预览偏移并请求重绘。偏移仅为会话视图状态，不写工程；原滚轮缩放保持。新增模拟拖动用例核对预览独立偏移及图像不变，待运行验证。版本 1.3.7，未生成 EXE。

- 循环预览平移验证：cargo check --tests --locked --offline 通过（target/pixel-tile-preview-pan-check.log），模拟拖动用例仅编译未执行；git diff --check 与七语言摘要完整性核对通过。实际平移后绘制、取色、形状及独立窗口仍待运行验证。

## 2026-11-19 像素 Tile 改进本地构建完成（未发布）

- 用户明确授权编译，事前确认主程序进程未运行、固定目标 EXE 独占访问通过；cargo build --release --locked --offline 成功，耗时 1 分 14 秒，仅既有 vendor 与 VideoSettings::configure 警告。
- 产物 target/release/unity_yaml_viewer.exe，文件／产品版本均 1.3.7，34011648 字节，SHA-256 c83c59a232615e8c427c5bb8b6866ab7564fd708c1a1f6bc3851710c76bb8fdd，完成时间 2026-11-19T16:54:30.0163000+08:00。旁存 .sha256，本轮未自动启动应用、未发布主程序或更新任何 Release。
- 包含此前默认移动 V、数位板缩放 Z、循环预览独立平移与绘制坐标同步、透明底色填充、图层眼睛／锁点击区域调整，以及 PS 安装包下载／校验／打开和定位。源输入快照 248 项在构建期间保存，构建后核对未变化；该快照不单独作为编译器来源证明。
- 沿用已通过的 cargo check --tests，不另行生成或运行测试程序；数位板操作、预览平移后绘制、图层按钮、PS 下载启动及目录定位均仍待用户实际运行验证。
- 证据 target/v1.3.7-tile-release-preflight.json、target/v1.3.7-tile-release-build.log、target/v1.3.7-tile-release-verification.json 和 target/v1.3.7-tile-build-inputs.json。

## 2026-11-19 新建图层矢量图标

- 图层标题旁新建按钮由字体加号改为带加号的折角纸张 SVG，复用通用矢量图标按钮，随主题着色并保留“添加图层”提示。新增、选中、撤销及图层数量／内存限制不变。
- 当前版本保持 1.3.7，本轮未重编 EXE；实际图标外观待运行验证。

- 新建图层图标验证：cargo check --locked --offline 通过（11.73 秒，target/pixel-tile-add-layer-icon-check.log），git diff --check 与七语言摘要完整性核对通过；未运行界面或生成 EXE。

## 2026-11-19 循环预览接入 Z 拖拽缩放

- 修正 Z 工具此前只在绘制区响应、循环预览不缩放的问题。循环预览内落笔／左键按下后以落点为中心，左拖缩小、右拖放大，松开结束；更新预览顶部数值，独立于绘制区缩放。停靠与独立只读预览均支持视图缩放，仍不将独立预览改为可绘制。
- 沿用循环预览 0.25–8 倍范围，按实际显示比例（包括最小 Tile 尺寸及带高度显示下限）计算落点偏移，避免低倍数或高 DPI 下缩放中心漂移。预览拖拽期间不叠加滚轮，平移优先，不修改像素或新增撤销。
- 主程序保持 1.3.7，实际数位板／鼠标缩放和顶部数值显示待运行验证；本轮未重编 EXE。

- 预览缩放检查：cargo check --tests --locked --offline 通过（target/pixel-tile-preview-zoom-check.log），七语言摘要完整性检查通过；实际界面待验证，未运行或生成测试 EXE。

## 2026-11-19 PS 下载折叠详情

- PS 插件区域增加默认折叠的“下载详情”，每次下载重新建立折叠状态，保留本次阶段日志及最终失败原因，可选中文本并复制详情。失败摘要仍在原位置显示，不再只能通过悬停查看具体原因。
- 日志记录准备、获取／校验附件信息、目录准备、连接安装包、写入、大小与 SHA-256 校验、保存版本记录及启动安装器阶段；记录固定请求地址、HTTP 状态、最终响应主机、限流剩余／重置与 Retry-After（响应存在时）、预期／实际大小和哈希、保存路径及底层网络错误链。下载前的元数据失败也有日志，界面显示当前准备阶段，不把尚未收到文件字节误判为无任务。
- 不增加自动联网或重试，不调整下载与安装授权方式；尚未定位用户本机失败根因，可能原因不能写为已确认。Release 显示标题与下载无关，代码按 ps-v<版本号> 标签查找，不读取 name。
- 七语言界面、关于摘要及中英文指南同步，主程序仍 1.3.7；真实失败原因展示与复制待运行验证，本轮未生成 EXE。

- 下载详情验证：cargo check --tests --locked --offline 通过（target/ps-plugin-download-details-check.log），git diff --check、七列翻译及七语言摘要完整性核对通过。未运行应用、未发起实际下载、未生成测试／主程序 EXE；本机失败根因待新构建运行后按详情确认。

## 2026-11-19 图层删除按钮与右键菜单

- 新建图层旁增加减号 SVG 按钮，删除当前选中图层；复用图标系统与“删除图层”七语言提示。锁定图层或仅剩一层时禁用，删除仍进入撤销历史并选中剩余有效图层。
- 图层名称取消通用 dnd_drag_source（其任意鼠标按钮都能触发拖拽），改为名称按钮自身处理点击／拖动；仅左键开始拖拽时设置排序载荷，且仅左键松开接受排序，右键保留名称上的删除／重命名菜单。不恢复覆盖整行的交互，眼睛／锁保持独立点击区。
- 主程序维持 1.3.7，本轮未生成 EXE。实际右键菜单、左键排序、删除按钮与撤销仍待运行验证。

- 图层菜单验证：cargo check --tests --locked --offline 通过（4.13 秒，target/pixel-tile-layer-menu-check.log）；git diff --check 与七语言摘要完整性核对通过。未运行测试 EXE／GUI，实际右键弹出和排序仍待用户实测。

## 2026-11-19 普通图层、最后一层删除与清空

- 不再自动创建 BaseLayer 及附带绘画层，新建只提供一个透明可编辑普通图层。导入当前图片建立默认锁定的普通图层；导入图片新增普通图层并默认锁定，解锁后可编辑／删除。旧工程图层名和内容保持，不将名为 BaseLayer 的历史层作为特殊层处理。
- 采用方案 1：删除最后一个已解锁图层后，同一操作内补一个透明、未锁定的普通图层，保持尺寸及格子排列；删除与自动补层整体参与撤销／重做。锁定层仍需先解锁。
- “清空当前格”改名“清空选中图层”，行为同步为清空选中层全部格子，不改变其他图层；为空、隐藏或锁定时禁用，保留明确提示及撤销。旧按钮只清当前层当前格，故空层或下方仍有图像时可能看不出变化。
- 多格导入扩展按用户要求暂缓：保留选格单张导入，每次新增默认锁定层；未支持批量导入自动排列及同层连续导入，不写成已实现。
- 新增普通层初始化、导入锁定／解锁、清空撤销及最后一层替换用例；既有合成与控件用例显式建立两层测试场景，不再依赖默认 BaseLayer。实际界面与操作待运行验证。保持 1.3.7，本轮未生成 EXE。

- 普通图层验证：cargo check --tests --locked --offline 通过（target/pixel-tile-ordinary-layers-check.log），新增与调整的 Rust 用例仅编译未执行；git diff --check、七列翻译和七语言摘要检查通过。未生成 EXE，实际清空、解锁删除及最后一层自动补层待运行验证。

## 2026-11-19 矩形／菱形／六边形模式

- 工程按钮下增加模式选择，替代原“菱形平铺”复选框；提供矩形、菱形、六边形，六边形下可选尖顶和平顶。带高度 Tile 保留菱形路径，与六边形互斥。切换模式保留原像素及图层，重置视图偏移，参与撤销；模式裁切只影响显示和导出，不删除隐藏角落的源像素。
- 六边形宽高为外接矩形，斜边由顶点自动计算，可独立调整宽高形成拉伸六边形。尖顶顶点在上下中点，四侧点位于高度1/4与3/4；平顶转置。尖顶高度须为4倍数，平顶宽度须为4倍数，另一边沿用偶数2–1024限制。切换不兼容尺寸明确提示而不修改已有画布；在六边形模式新建时对相应尺寸向上对齐4倍数。正六边形近似比例：尖顶宽/高=0.866，平顶宽/高=1.155，整数像素仅能近似。
- 两朝向共用蜂窝轴坐标与最近中心像素归属，绘制、橡皮、取色、直线／框画、周期回绕、透明底色填充、有限多格范围、平铺预览和 PNG 透明裁切使用同一归属。负坐标和边界不重复分配像素；尺寸满足条件时两轴周期位移为整数。预览保留已有移动、缩放和直接绘制。
- 工程格式升至4，保存六边形朝向并兼容读入格式1–3；旧工程无新增字段时按原矩形／菱形恢复。旧程序不支持格式4。软件版本仍为1.3.7；本轮未生成 EXE、未发布。
- 独立 JavaScript 几何模型核对两种朝向、三种尺寸、负坐标和两轴周期，共55296项周期检查通过；单格像素面积均为外接矩形3/4，像素局部坐标均在边界内。证据 target/pixel-tile-hex-geometry.json。新增 Rust 几何／多格填充导出撤销用例，仅编译未运行；实际 UI、笔触手感、多格排列和旧工程读写仍待运行验证。

- 六边形最终检查：cargo check --tests --locked --offline 通过（target/pixel-tile-hex-check.log）；新增 Rust 用例仅编译未执行。独立几何模型55296项检查、git diff --check、七列翻译及七语言摘要完整性通过；未生成 EXE，实际界面和工程往返仍待运行验证。

## 2026-11-19 工程素材库与固定 BaseLayer（替代此前取消 BaseLayer 的多格方案）

- 按用户新设计，多格模式保留专用 BaseLayer，固定最底部且不可删除、改名、解锁或排序，不直接绘画；名称右键菜单为“设置为空／导入素材”，只改变当前格基础素材，上方普通绘画层保持。删除最后一个普通绘画层会补一个透明普通层，BaseLayer 不计入可绘画层。单图的普通图层导入与解锁规则保留。旧工程已有锁定 BaseLayer 可识别为专用素材层，其他旧内容不删除。
- 素材库先随当前 .uytile 工程保存，非全局库。增加图片导入、缩略图网格、点击填入选中格、当前可见合成存为新素材；导出继续合成可见素材层与绘画层并按模式裁切。素材填入格子时复制内容，不随库条目移除而丢失。库支持最多64项、像素总量16 MiB，图片尺寸必须与单格一致，不自动缩放。
- 库区域支持外部本地图片拖拽，可多文件加入库而不自动分配格子，悬停高亮；仅库区域内接收文件并消费拖放事件，编辑弹窗绘制在主文件打开处理之前。尺寸不符、解码失败或超预算报告具体文件／原因，部分成功保留并显示数量。真实 Windows 拖入及主窗口路由仍待实测。
- 缩略图右键“移除素材库”，替代直接叉号；已填格子和绘画层保留，素材库增删及格子素材替换／清空进入工程撤销历史。BaseLayer 角色固定，不能通过右键删除或按钮删除。
- 工程格式升至5，内嵌库名称／像素并保存图层素材角色，兼容读取格式1–4；旧程序不能读取格式5。此方案取代上方“多格同层素材填入暂缓”及“取消 BaseLayer”的当前行为描述，原条目保留历史事实。批量自动排格、全局跨工程库仍未实现。
- 普通图层上绘制可跨格，素材库区域不改画笔状态；素材层可切换显隐。模式／导出沿用矩形、菱形和两种六边形。README、双语指南及七语言摘要同步；主程序仍1.3.7，本轮未生成 EXE。

- 素材库最终验证：cargo check --tests --locked --offline 通过（target/pixel-tile-material-library-check.log），新增素材赋值不改上层、清空恢复、BaseLayer不可删除及普通层补层用例仅编译未执行。七列翻译、七语言关于摘要与 git diff --check 通过。关于摘要同步归纳为六项，保留已知限制。保存前增加256 MiB序列化大小检查，超限不覆盖目标文件，避免生成无法重开的超大工程。真实 Windows 外部拖入、缩略图右键、保存重开仍待用户运行验证；未生成或运行 EXE。

## 2026-11-19 合并绘制与循环预览、双视图与定位

- 主界面由设置／绘制／循环预览三列改为设置＋统一绘制区，移除右侧重复绘制面板。顶部提供“有边界（Q）／无边界（W）”：前者显示设置的有限格子，后者在同一组中心格子周围重复，并可直接编辑周期对应像素。中心格子以黄色轮廓标出；普通多格统一编辑整组，不再显示“编辑整个循环单元”开关。
- 两种视图共享绘制区缩放、平移和工具，切换不重置视图或改变像素；无边界复用此前循环预览的遮挡／命中逻辑，中心原点改为设置循环单元的几何中心，覆盖矩形、菱形、两朝向六边形及带高度显示。无边界下编辑重复副本写回中心对应格，不依赖有限模式的边缘回绕开关；填充仍按有限循环单元执行。
- “无边界”后增加圆环十字矢量定位图标，快捷键 H；定位只重置主视图偏移，不改变缩放。Q／W／H 在文字输入时不触发，保留 Ctrl+Z 等组合键；切换视图或定位会结束已有笔触，避免跨位置连线。
- 保留可选独立只读预览窗口，拥有独立视图状态；主界面无边界绘制不会改动独立窗口的倍率和拖动状态。极小格子／低倍率保留原预览每方向约32格的绘制预算，不宣称真正无限绘制数量。双视图状态为会话视图，不写入工程。
- 新增 Q／W／H 只影响视图、回中保持倍率，以及各平面模式中心格几何对齐用例；仅编译检查，实际视图切换、带高度遮挡、外圈绘制和回中心仍待运行验证。版本保持1.3.7，本轮未生成 EXE。

- 合并视图最终验证：cargo check --tests --locked --offline 通过（target/pixel-tile-unified-canvas-check.log），Q／W／H视图状态及四种平面几何中心用例仅编译未执行；git diff --check、七语言摘要及七列翻译核对通过。未运行或生成EXE；真实视图切换、定位、数位板、重复副本绘制与独立窗口仍待用户实测。

## 2026-11-19 绘制区工具栏拖动与位置记忆

- 工具栏顶部增加双横线拖动区，仅左键拖动移动工具栏；工具按钮继续只切换工具，不将整条工具栏作为拖动区。工具栏拖动期间阻止画布落笔／平移穿透，兼容有边界与无边界视图。
- 位置保存为相对绘制区左上角的逻辑坐标，在用户结束拖动时写入应用配置目录 pixel_tile_toolbar.json；新开编辑器恢复最后保存位置，不改工程内容或撤销历史，文件写入失败显示提示。
- 每帧根据绘制区实际可见裁切范围和工具栏测量尺寸约束位置，窗口移动、缩放和 DPI／布局变化后自动回到可见范围；不是使用桌面绝对坐标。可用高度不足时工具按钮区域纵向滚动，顶部拖动区保留。
- 新增缩小窗口、不同绘制区原点、负值／NaN和极小区域的位置约束用例，仅编译检查；真实拖动、窗口缩放、高DPI与重开位置恢复仍待运行验证。版本保持1.3.7，本轮未生成EXE。

## 2026-11-19 设置折叠与统一 Slot 编号

- 画布尺寸／新建、模式与朝向、导入当前图片／另存PNG、带高度与地面尺寸、边缘回绕、独立预览、多格行列及快捷预设集中到“设置”，默认收起并可展开；保存／打开工程、Slot选择、素材库、图层和背景控件保留在外部。仅改变界面分组，不新增主程序全局设置项。
- 绘制区“网格”后增加默认关闭的“显示 Slot”，名称与 PS 插件一致；有／无边界及独立预览显示相同的逐行连续编号。左侧格子按钮改为1…N，3×3为1–9，当前格信息改为当前Slot编号；悬停按钮仍可查看行列，内部坐标／图像引用及工程数据保持不变。
- 画面编号使用半透明底，不写图像、PNG或工程像素；画面格子过小时省略标记避免重叠，显示开关是会话视图状态。多格行列改变后按当前行列重新编号，重复副本显示所属中心Slot号。
- 主程序保持1.3.7，七语言提示、关于摘要及中英文指南同步；实际折叠布局、Slot对应、高DPI和工具栏拖动／位置恢复待运行验证，本轮未生成EXE。

- 工具栏／设置／Slot最终验证：cargo check --tests --locked --offline 分别通过（target/pixel-tile-floating-toolbar-check.log、target/pixel-tile-settings-group-check.log、target/pixel-tile-slot-labels-check.log）；工具栏范围用例仅编译未执行。git diff --check、七列翻译及七语言摘要检查通过；未生成EXE，实际工具栏拖动、折叠和编号显示待用户运行验证。

## 2026-11-19 绘制区顶部 Tab 与缩放位置

- 有边界／无边界改用与主题切换共用的紧凑连体Tab组件，保留蓝色选中态、浅深主题适配和Q／W切换；原视图切换逻辑不变。
- 缩放数值移至“适应画布”按钮之后、撤销之前；定位、网格、显示Slot及其功能不变。
- 仅UI布局及组件复用，主程序仍1.3.7；实际显示待运行验证，本轮未生成EXE。

## 2026-11-19 画面 Slot 选中高亮

- 修正左侧Slot选择后画面编号无区别的问题：当前Slot编号徽标采用主题选中底色／文字色、加粗描边和略大字号，普通编号保持半透明黑底白字。深浅主题均读取主题配色。
- 有边界、无边界及独立预览使用相同逻辑；无边界中属于同一Slot的重复副本同步高亮，负行列用周期取模对应，编号及选择数据不变。仅显示叠加，不修改素材、像素、撤销及导出。
- 原显示Slot开关与低倍率省略规则保留，1.3.7不变；本轮未生成EXE，实际深浅主题及连续切换待运行验证。

- 顶栏／Slot高亮核验：cargo check --locked --offline 通过（target/pixel-tile-header-tabs-check.log、target/pixel-tile-slot-highlight-check.log）；git diff --check与七语言摘要完整性核对通过。未运行GUI或生成EXE，主题Tab、缩放顺序和Slot高亮待新构建实测。

## 2026-11-19 单格／多格统一保留 BaseLayer

- 按用户最新决定，替代此前“单格不保留BaseLayer”的分支：单Tile、新建空白、多Tile切换回单Tile都保留唯一固定BaseLayer；素材库与设置为空／导入素材在单格也可用，专用层仍不可删除、解锁、改名或排序。
- 单格图片导入也改为加入工程素材库并填入当前格基础层；“导入当前图片”重建画布时将源图放入BaseLayer，上方保留一个透明、可编辑普通绘画层，新建空白则BaseLayer与绘画层都透明。导入源图自动加入工程素材库，保留尺寸／容量限制。
- 旧工程有固定素材层时复用，不重复创建；无BaseLayer时按现有层数和像素预算补充透明素材层，不覆盖旧图层内容。极限预算不足时沿用明确错误提示，不静默删除或转换普通图层。相关初始化、模式往返、源图／空白重建用例仅编译未运行。
- 主程序维持1.3.7，本轮未生成EXE；单格库操作、模式切换、源图显示、撤销与旧工程迁移仍待运行验证。

## 2026-11-19 素材库原生拖放坐标、连通填充及简化加号

- 源码确认素材库此前复用主窗口拖放位置，原生Tile子窗口可能使用错误客户区坐标，普通指针事件在OS拖放时也可能中断。现按当前原生子视口的客户区桌面边界及实际pixels_per_point，把系统光标物理坐标转换到该窗口局部；根视口保持原HWND逻辑。负屏幕坐标、高DPI及右下半开边界纳入检查。Windows拖放不再回退到可能过期的普通指针位置。
- 库区域下方直接显示加入数量和逐文件失败原因；目标未命中时提示拖入素材库边框内。已有尺寸／预算／格式限制不放宽，成功接收仍消费本窗口的拖放事件。新建、切换工程和主动导入清除旧状态，实际Explorer多文件拖入仍待运行验证。
- G填充改为对当前层连通的相同RGBA区域替换，包括触及格子边界的已绘制颜色，不覆盖断开的同色部分；透明Alpha=0继续忽略隐藏RGB并填充连通空白。统一以有限格子范围截断，不再要求闭合。提示仅显示“填充(G)”，不附加行为说明。
- 新增图层SVG简化为纯加号，与减号使用相同笔宽、圆端点和长度；功能／提示不变。主程序保持1.3.7，本轮未生成EXE。

- 本轮最终验证：cargo check --tests --locked --offline 通过（target/pixel-tile-unified-base-layer-check.log、target/pixel-tile-drop-fill-check.log），新增单／多格BaseLayer、源图初始化、原生子窗口拖放坐标和边界连通同色填充用例仅编译未执行。git diff --check、七列翻译及七语言摘要核对通过；未生成EXE。真实拖入、填充和单格BaseLayer仍待新构建运行验证。

## 2026-11-19 单格精简与侧栏标题折叠

- 多Tile预设在2×2之前补充1×1，保留2×2／3×3／4×4。实际循环只有一格时隐藏左侧Slot按钮与当前Slot提示，绘制区保留网格，不显示“显示Slot”开关，也不绘制Slot徽标；单Tile同样省略。1×N／N×1等多格仍显示编号。隐藏不影响格子选择和素材归属。
- 返回多格时恢复原显示Slot偏好；1×1仍保留BaseLayer和素材库，切换不删除图像。
- 设置、工程素材库及图层统一使用Heading标题字号并加粗。设置保留默认收起；素材库使用可折叠标题、默认展开，收起时隐藏库内按钮、缩略图、拖入区、结果和说明，不改变已填素材及导出。收起时不能向库内拖入文件。
- 图层标题行最小高度与28逻辑点加减按钮一致，沿原水平居中布局排列。主程序保持1.3.7，本轮未生成EXE，实际标题、折叠与1×1往返仍待运行验证。

- 单格／标题核验：cargo check --locked --offline 通过（target/pixel-tile-single-slot-settings-check.log），git diff --check与七语言摘要完整性核对通过；实际标题对齐、素材库折叠和单格Slot隐藏待运行验证，未生成EXE。

## 2026-11-19 绘制区顶部垂直居中修正

- 顶栏不再用交互默认高度启动、被后续24点Tab和28点定位按钮逐步撑高；改为预先分配不小于主题Heading文字高度的24点行，整行使用垂直居中布局，保留自动换行。
- 定位按钮改为与主题Tab相同的24×24尺寸，图标保持14点；按钮、数值框等仅在该行统一交互高度，不修改全局样式或其他工具栏。缩放仍在适应画布后，Q／W／H功能不变。
- 主程序仍1.3.7，本轮未生成EXE；实际顶栏、多语言及高DPI对齐待运行验证。

- 顶栏对齐检查：cargo check --locked --offline 通过（target/pixel-tile-header-alignment-check.log），git diff --check及七语言摘要完整性核对通过；未生成EXE或运行界面，实际垂直对齐待验证。

## 2026-11-19 安装器版本比较与同版本覆盖（发布后源码调整，未重建）

- 安装器读取已部署版本并进行语义版本比较：未部署显示安装、旧版显示更新、同版本显示覆盖安装；同版本明确提示重新部署并校验，用户点击后直接执行覆盖，不额外确认。读取失败／版本无效禁用安装并提示原因，较新部署禁用旧安装器覆盖；底层安装执行前再次检查，防止状态刷新后发生误降级。
- 保留Photoshop进程保护、暂存／切换／回滚、额外文件保留及SHA-256校验，不因同版本绕过原保护。卸载确认文件数改为按当前内嵌清单显示。源码与插件仍0.1.0，未重编安装器、未修改公开附件或发布状态；已发布安装器尚无本次按钮比较提示。
- 新增未安装／同版覆盖／较旧更新／较新阻止／无效版本及阻止较新部署写入用例；既有同版重部署及保留额外文件用例继续适用。仅编译检查，实际按钮与安装行为待新版安装器验证。

## 2026-11-19 用户验证：主程序下载并启动PS安装器

- 用户明确反馈“已测试主程序能成功下载并运行”。主程序手动查询后下载安装包、完成现有校验并启动安装器的用户链路实测通过。此结论仅覆盖下载和运行安装器，不代表新版本比较界面已验证或PS插件全部功能通过。

- 安装器版本比较核验：cargo check --tests --locked --offline --manifest-path plugins/photoshop-isometric-tile/installer/Cargo.toml 通过（target/ps-installer-version-action-check.log），新增Rust用例仅编译未运行。git diff --check和七语言摘要完整性检查通过，未生成EXE。


## 2026-11-23 绘Tile 本地验证构建完成（未发布）

- 经用户明确授权，编译前主程序进程与固定 EXE 独占访问检查通过；cargo build --release --locked --offline 成功，耗时 1分12秒。
- 产物 target/release/unity_yaml_viewer.exe，文件版本 1.3.7，产品版本 1.3.7，34083840 字节，SHA-256 8cd10c623e92d2c40b31bec9ed771df99ac429799504aa71e716fb0250cc59c9；本地 .sha256 已同步。
- 包含硬边圆形铅笔、铅笔／柔边画笔 Tab、共用 B 与对应 E 橡皮、工具图标及绘Tile入口更名。压感、硬度／流量／间距调节尚未实现，带高度模式仍为单像素限制。
- 未自动启动、未发布；实际界面与绘制行为仍待用户验证。构建日志 target/tile-painter-release-build.log，产物核验 target/tile-painter-release-verification.json。


## 2026-11-23 绘Tile 与图片预览调整本地构建（未发布）

- 用户明确授权，构建前主程序进程与固定 EXE 独占访问检查通过；cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe；文件／产品版本 1.3.7／1.3.7，34187264 字节，SHA-256 30c0ba991db28145cbfab9ab4d51efb9c0a1907b72c88d2b309cef743adb8d9e。本地 .sha256 已同步。
- 包含图片预览共用缩放模式／采样／通道／应用Alpha、默认适应窗口仅重启初始化、绘Tile单行工具与单一尺寸预览、不透明度文案、半透明亮边显示修正、未锁图标灰色、清空图层按钮归位、素材导入入口合并、可用加减图标提亮加粗。
- 未自动启动或发布；实际跨图预览、重启初始化、绘制效果、界面布局与各主题按钮仍待用户运行验证。画笔压感、独立硬度／流量／间距仍未实现。
- 日志 target/tile-refinements-release-build.log，核验 target/tile-refinements-release-verification.json。


## 2026-11-23 画笔样式与取色修正本地构建（未发布）

- 用户授权，构建前主程序进程及固定 EXE 独占访问检查通过；cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe；文件／产品版本 1.3.7／1.3.7，34056192 字节，SHA-256 53517d960c80f89b316c7356bdd2329fda14fb898e41fbeadcdc9d2ac4fc2373。本地 .sha256 同步。
- 包含柔边／硬边圆及硬度、七种画笔混合模式（大小数值框后）、工程参数保存、无边界合成取色、有边界有限范围取色、吸管光标、绘制颜色／预览背景色独立窗口置顶及点击唤回。
- 未自动启动或发布。实际取色边界、吸管热点、各模式／硬度、工程读写及调色窗口层级待用户运行验证；流量、压感与其他 PS 模式未实现。
- 日志 target/tile-brush-picker-release-build.log，核验 target/tile-brush-picker-release-verification.json。


## 2026-11-23 选区／压感与素材库本地构建（未发布）

- 用户授权，构建前主程序进程及固定 EXE 独占访问检查通过；cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe；文件／产品版本 1.3.7／1.3.7，34345984 字节，SHA-256 ea735efd061643ec07a741280242a823b7877f5bb5642da93ac355c27758e6f9。本地 .sha256 同步。
- 包含样式箭头／硬度滑条修正、百分比缩放、画笔衰减与抗锯齿／单笔覆盖、Windows Ink 压力大小／不透明度、素材缩略图等比居中与5至1列大小滑条、BaseLayer分范围复制／普通图层复制及Ctrl+J、四模式选区及M记忆、选区增减／移动／Ctrl剪切移动与复制裁切、工具组再次点击展开、手柄右键横竖排列及菜单边缘换侧。选取工具紧随移动工具。
- 未自动启动或发布。界面、选区编辑／右键／复制／撤销、菜单边缘布局、软硬笔触与Wacom实机压力仍待运行验证；单元用例此前仅编译未运行。流量、WinTab专用驱动、压感曲线编辑、选区羽化／旋转缩放及系统剪贴板未实现。
- 日志 target/tile-selection-pressure-release-build.log，核验 target/tile-selection-pressure-release-verification.json。


## 2026-11-23 用户指定绘制参考：PhotoCraft

- 用户指定 https://github.com/storytold/photocraft 作为后续绘制参考。已在线核对README：项目自称纯Rust的Photoshop clean-room reimplementation（净室重实现），不据此称为Adobe源码逆向。仓库workspace标注MIT OR Apache-2.0；本轮只读参考，未复制其源码或添加依赖。
- 已读 crates/paint/src/lib.rs、render.rs、dynamics.rs：独立笔画缓冲、笔尖间距与动态输入、压力插值、流量逐落点累积但受笔画不透明度上限限制，再与笔前像素合成；可参考以改进当前最大覆盖近似。流量仍按用户约定最后接入，不能据阅读标记已实现。
- 已读 crates/tablet/src/lib.rs：Windows Ink补充笔状态，以及WinTab后备、倾斜／旋转／笔尾信息；当前项目Windows Ink压力大小已用户验证，不因参考而替换现有输入链路。
- 已读 crates/compose/src/lib.rs 与 crates/ops/src/lib.rs：显示RGB合成参考与CPU/GPU对照、写时复制像素块／事务快照历史；用于后续颜色／撤销行为对照，不保证其全部功能已独立验证。
- 来源：https://raw.githubusercontent.com/storytold/photocraft/main/crates/paint/src/render.rs 、https://raw.githubusercontent.com/storytold/photocraft/main/crates/paint/src/dynamics.rs 、https://raw.githubusercontent.com/storytold/photocraft/main/crates/tablet/src/lib.rs 、https://raw.githubusercontent.com/storytold/photocraft/main/crates/compose/src/lib.rs 、https://raw.githubusercontent.com/storytold/photocraft/main/crates/ops/src/lib.rs 。链接随main变化，正式采纳实现前应记录当时提交并按实际文件保留许可归属。


## 2026-11-23 PhotoCraft参考范围与Tile流量实现

- 用户明确只优化Tile绘制体验，完整编辑需求直接使用PhotoCraft，不重复造完整编辑器。本轮独立实现距离落点／流量缓冲与压力流量，未复制外部源码、未加依赖。以笔画前快照合成，区分流量与整笔不透明度；旧最大覆盖近似替换。
- 仅流量／间距／压感流量及当前Tile交互；不扩展完整笔刷、滤镜或PSD编辑体系。工程可选参数保持旧文件兼容，版本1.3.7，未重建EXE；检查证据 target/tile-brush-flow-check.log，Rust用例仅编译，实机待验证。


## 2026-11-23 工程预览／子组／流量预设本地构建（未发布）

- 用户明确授权，构建前主程序进程及固定 EXE 独占访问检查通过；cargo build --release --locked --offline 成功。
- 产物 target/release/unity_yaml_viewer.exe；文件／产品版本 1.3.7／1.3.7，34558464 字节，SHA-256 57fed713e90e9b6444db4f38ec2d2411af4120c809d5d110478a5b809c3d3a33。本地 .sha256 已同步。
- 包含工具组菜单输入锁重入修正、工程标签后台预览与布局形状信息、高度Tile只读限制、子组边缘抬笔联动、Delete保护BaseLayer与Ctrl+Shift+J剪切新层、压感抬笔状态提示、Tile画笔距离采样／亚像素／流量／间距／压力流量，以及8种预设圆图标与笔触列表双向联动。
- 未自动启动、未发布。实际菜单、预览刷新、子组与选区组合、图层快捷键、流量手感、预设联动及旧工程读写待用户验证；Rust用例此前仅编译未运行。Wacom压力大小既有用户验证不代表本轮所有流量压感行为通过。
- 日志 target/tile-flow-presets-release-build.log，核验 target/tile-flow-presets-release-verification.json。

## 2026-11-23 Tile图层组／蒙版链接适配（未构建）

当前1.3.7源码新增layer_groups.rs和layer_properties.rs，实现层级组与基础图层外观；多选Ctrl／Shift、图层间链接、独立的蒙版内容链接、V移动和Tile回绕。schema 7保存，旧1–6兼容读取。详细范围与限制见当前版本记录；未生成新的EXE。

参考：https://github.com/storytold/photocraft/blob/main/crates/doc/src/lib.rs （LayerMask.linked、Layer.link_group、Group）；https://github.com/storytold/photocraft/blob/main/crates/compose/src/composite.rs （composite_layer_plain的穿透／隔离及预乘覆盖插值）。为现有Tile模型独立适配，无源码复制。公开API目录请求曾网络超时，后通过raw源码核对模型。

检查：cargo check --tests --locked --offline通过；新增测试仅编译未运行，界面及设备交互待验证。

## 2026-11-23 图层组与蒙版链接本地构建（未发布）

- 用户明确授权编译；构建前未发现unity_yaml_viewer进程，固定target/release/unity_yaml_viewer.exe独占访问通过。
- cargo build --release --locked --offline成功，日志target/tile-groups-release-build.log；保留依赖及media_trim既有警告。
- EXE文件／产品版本1.3.7／1.3.7，34565632字节，SHA-256 110a7372641a16cb09c8a13eafab719aee15c79af8bfe7d030e26977966167f8；本地unity_yaml_viewer.exe.sha256已更新。
- 本次包含图层外观、组容器与穿透合成、多选链接、蒙版内容独立移动及此前选区轮廓／菜单调整。未自动启动、未发布；新增测试仅编译检查，实际交互、旧工程读写与导出效果仍待用户验证。

## 2026-11-23 图层面板图标化（源码，未重建）

按用户PS截图合并图层首行操作，新增图层使用方框加号、删除使用垃圾桶，锁定改图标；不透明度／填充保留直接输入和水平DragValue，不提供滑条下拉。组文字行移入文件夹图标菜单，清空操作保留图层右键菜单。cargo check --tests --locked --offline通过；未重建EXE，界面待实测，版本1.3.7。

## 2026-11-23 蒙版缩略图右键（源码，未构建）

右键蒙版缩略图提供启用、删除、应用、选区替换／加／减／交叉、反相及内容链接。灰度转硬选区以128为阈值；组及调整层禁用应用。cargo check --tests --locked --offline通过，新增用例仅编译，未生成EXE，界面待验证。

## 2026-11-23 锁定图标联动（源码）

绘制锁清除并禁用透明度锁；父组绘制锁也禁用成员透明度锁操作。cargo check --tests --locked --offline通过，未重建EXE，版本1.3.7，界面待验证。

## 2026-11-23 移动光标层级（源码）

移动三角光标改用独立Tooltip绘制层，避免画布及图层遮挡；未重建EXE，待实际界面验证。

## 2026-11-23 工具上下文选项栏（源码）

第二行按移动、选区、吸管、缩放、笔刷／擦子、形状及填充切换，隐藏无效参数。未扩展自动选层／对齐／变换。cargo check --tests --locked --offline通过，未重建EXE，界面待验证。

## 2026-11-23 放弃修改恢复工程预览（源码）

显式discard_requested事件使ProjectPreview重新异步加载保存文件，替换接收通道，防止空默认文档和过期渲染覆盖。新增回归用例仅编译，未运行；未生成EXE，版本1.3.7。

## 2026-11-23 工程预览静默限频（源码）

移除已有预览的后台更新转圈，保留首次加载状态。任务启动最小间隔1秒、变化稳定250ms、抬笔后取快照，无并发队列；纹理复用。cargo check --tests --locked --offline通过，未重建EXE，实际卡顿改善待验证。

## 2026-11-23 绘Tile背景Tab（源码）

背景仅棋盘格／自定义，颜色控件同排，无灰色；状态沿用本工程独立字段，不与全局背景共用。cargo check --tests --locked --offline通过，未重建EXE，界面待验证。

## 2026-11-23 擦子图标清空图层（源码）

图层操作栏复用Eraser图标调用clear_selected_layer，保留原权限与撤销；蒙版目标下禁用以区分图层清空。cargo check --tests --locked --offline通过，未生成EXE。

## 2026-11-23 图层面板与预览调整本地构建（未发布）

- 用户明确授权，构建前未发现unity_yaml_viewer运行，固定EXE独占访问通过。cargo build --release --locked --offline成功，日志target/tile-ui-refinements-release-build.log，保留依赖及media_trim既有警告。
- 产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.7／1.3.7，34902016字节，SHA-256 47f47bc19032465732db28b88e0c22d5485bbeb0f895ebec969c1df2b6a46933；本地.sha256已更新。
- 包含图层操作与锁定图标、蒙版右键、绘制锁与透明度锁联动、三角光标顶层、上下文工具栏、放弃修改恢复工程预览、静默限频刷新、独立背景Tab及擦子图标清空图层。
- 未自动启动、未发布；实际交互与流畅度待用户验证，新增回归用例目前仅通过编译检查。应用版本保持1.3.7。

## 2026-11-23 图标清晰度（源码）

移动／画笔／擦子／搜索／新增／删除采用原生像素对齐绘制，面板统一正方形16px，垃圾桶简化重画，透明度锁改实心棋盘格。未重建EXE，不同DPI实际清晰度待验证。

## 2026-11-23 直接添加蒙版（源码）

顶部蒙版按钮单击添加白色链接蒙版并切到蒙版编辑；已有时禁用，不再开菜单，管理统一缩略图右键。cargo check --tests --locked --offline通过，未重建EXE。

## 2026-11-23 清理孤立图层链接（源码）

删除单层／多选／组、解散组及工程加载时清理单成员link_id；删除原快照支持撤销恢复链接。新增回归用例仅编译，cargo check --tests --locked --offline通过，未重建EXE。

## 2026-11-23 锁图标像素对齐（源码）

Lock／Unlock统一原生矢量，图层行与锁定栏复用；灰色与选中状态保持，未重建EXE，实际清晰度待验证。

## 2026-11-23 锁定对象眼睛开关（源码）

移除自身／祖先锁定对显示切换的限制，保留撤销和其他编辑权限；未重建EXE，界面待验证。

## 2026-11-23 移动自动选择与Tab（源码）

增加默认关闭的自动选择，图层／组改相邻Tab；启用时按可见像素和蒙版命中，关闭时只移动当前对象。组取最外层，保留多选／链接移动。新增用例仅编译，cargo check --tests --locked --offline通过，未重建EXE。

## 2026-11-23 曲线／剪贴／图层目标／工具栏／打开保护（源码，未构建）

新增curve_editor.rs：RGB和通道三次贝塞尔锚点／手柄、增删点、平滑、LUT；旧五点保持读取与节点转换，新存schema 8，应用1.3.7。新增同父级Alt间隙剪贴与右键切换、连续剪贴调整合成，调整图标与内容／蒙版互斥高亮。浮动工具栏方向独立缓存、取消滚动默认宽度并收缩。打开工程有未保存内容时暂存路径并弹确认，取消不替换。

检查日志target/tile-curves-clipping-ui-check.log、target/tile-project-open-guard-check.log及target/tile-toolbar-orientation-check.log；cargo check --tests --locked --offline通过，用例仅编译未运行，未生成EXE。限制：组剪贴隔离、调整层不能作剪贴基底，无PS全功能色彩／直方图兼容。

## 2026-11-23 组拖放／行对齐／新层位置（源码）

组仅文件夹图标，开闭两态；眼睛最前固定、箭头统一列。组中部拖入、边缘排序与跨父级、根区域移出，多选根对象保留像素与索引映射，拒绝循环及受保护对象。组选中时新增层在孩子顶部并展开。新增用例仅编译；未重建EXE，实际拖放待验证。

## 2026-11-23 外侧软硬笔尖预览（源码）

外侧图标复用round_tip_coverage生成的缓存纹理，跟随硬度，尺寸仍在固定框内变化；铅笔预览不变。未重建EXE，实际显示待验证。

## 2026-11-23 组右键文字（源码）

复制／删除按组显示，组隐藏Ctrl+Shift+J剪切新层。未重建EXE，菜单待验证。

## 2026-11-23 曲线与图层组交互本地构建（未发布）

- 用户明确授权编译；构建前无unity_yaml_viewer进程，固定EXE独占访问检查通过。cargo build --release --locked --offline成功，日志target/tile-layers-curves-release-build.log。
- 产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.7／1.3.7，34876416字节，SHA-256 e9b7d08834355d7d456bd2cca728abd5b06e7c361fd01d4c29cd9cfd6f520099；本地.sha256已更新。
- 包含图标清晰度、蒙版直接添加、孤立链接清理、锁定眼睛开关、自动选择Tab、贝塞尔曲线和schema 8、Alt剪贴、调整图标与目标高亮、工具栏方向尺寸、打开未保存工程保护、组拖入拖出与新层顶部插入、软硬笔尖同步及组右键文字。
- 构建保留依赖警告，以及identity_curves旧曲线测试辅助未在release使用、media_trim configure未使用警告。未自动启动、未发布；新增用例此前仅编译未运行，实际界面、曲线手柄、连续剪贴与拖放待用户验证。

## 2026-11-23 矩形填充／清空删除确认／fx（源码）

矩形新增内部填充，正常Alpha与回绕去重、选区和锁定约束，4Mi工作框；填充时隐藏笔尖参数。清空／删除图层与组各UI入口统一确认，取消不改动、确认可撤销。启用样式行显示fx并预留宽度。cargo check --tests --locked --offline通过，用例仅编译，未生成EXE，界面待验证。

## 2026-11-23 图层行空隙收紧（源码）

普通层移除空箭头占位，组箭头10、文件夹区域28、层级步进6、间距2，眼睛保持最前固定。未重建EXE，布局待验证。

## 2026-11-23 多边形选区实时首段（源码）

首点即开放预览，起点圆圈与8物理像素吸附闭合，按用户确认不改套索。cargo check --tests --locked --offline通过，新增用例仅编译，未重建EXE。

## 2026-11-23 工具选项固定高度（源码）

固定预留工具选项行，子Ui控件垂直居中；画布分配不再依赖各工具控件高度。未重建EXE，不同字号与切换待验证。

## 2026-11-23 缩放比例下拉（源码）

缩放工具显示self.zoom，改为25–6400%常用预设下拉，与顶栏和鼠标拖拽同步；切预设按比例调整pan保留中心。未重建EXE，交互待验证。

## 2026-11-23 选区填色（源码）

选区栏共享颜色／不透明度和填充按钮；Alt+Enter全绘制工具生效，按选区正常Alpha合成，保留选区与单步撤销，尊重锁定。新增用例仅编译；未重建EXE。

## 2026-11-24 调整与fx参数框（源码）

调整层行末三角、样式行fx切换独立参数框，点击选择对象和内容目标，再点／切层收起；移除侧栏常驻参数以避免列表跳动。复用现有编辑与撤销，锁定只读。新增用例仅编译，未重建EXE，交互待验证。

## 2026-11-24 亮度／对比度滑条（源码）

按参考图标签+数值、下方滑条，亮度-150～150，对比度用户确认-50～100；旧超范围值不在打开界面时截断。cargo check --tests --locked --offline通过，未重建EXE。

## 2026-11-24 色阶／HSL控件（源码）

新增levels_editor.rs，输入RGB直方图及黑／Gamma／白多滑块、输出渐变双滑块；输入上下文按组／剪贴计算并缓存至下次展开。HSL增加彩色滑条。输出色阶进入schema 9，旧值默认0／255，应用1.3.7。新增用例仅编译，未生成EXE，实际滑块待验证。

## 2026-11-24 填充与调整控件本地构建（未发布）

- 用户明确授权；构建前无unity_yaml_viewer运行，固定EXE独占访问通过。cargo build --release --locked --offline成功，日志target/tile-adjustments-ui-release-build.log。
- 产物target/release/unity_yaml_viewer.exe，文件／产品版本1.3.7／1.3.7，34926080字节，SHA-256 7f7b3c70fdc61ddd948fec584c56f51a0fd5a0b24e04451e196f9dfa8756b4a2；本地.sha256已更新。
- 包含矩形填充、清空／删除确认、fx标记、图层间隔、多边形预览、统一工具栏高度、缩放预设同步、Alt+Enter选区填色、调整／fx展开框、亮度／对比度滑条、色阶直方图与输出色阶及HSL彩色滑条；工程新存schema 9，应用版本仍1.3.7。
- 仅本地构建，未自动启动、未发布；新增用例此前仅编译未运行，界面、快捷键和颜色结果待用户验证。保留依赖及未使用辅助函数警告。

## 2026-11-24 参数框收缩（源码）

去重复调整标题／正文标签，内容宽260，Window自动尺寸+内部自动收缩滚动；独立紧凑窗口缓存。未重建EXE，实际高度和位置待验证。

## 2026-11-24 统一参数框宽度（源码）

统一DETAIL_WIDTH=240，与曲线绘图区相同；曲线提示显式换行，不撑宽窗口，保留自动高度。未重建EXE，界面待验证。

## 2026-11-24 调整层右键剪切项（源码）

调整层同组一样隐藏Ctrl+Shift+J通过剪切的新层，仅普通像素层显示；执行端已有保护。未重建EXE。

## 2026-11-24 继承隐藏的眼睛灰显（源码）

眼睛高亮取visible&&ancestors_visible，未有效显示的Eye／EyeOff灰色；不改自身visible标记、不禁用开关。未重建EXE，主题与嵌套待验证。

## 2026-11-24 继承父组锁图标（源码）

父组锁行为保持，仅子对象显示灰色闭锁、不高亮，顶部继承锁不高亮；自身锁标记不变。未重建EXE，主题显示待验证。

## 2026-11-24 继承锁BaseLayer禁用样式（源码）

继承锁行内按钮直接复用BaseLayer相同helper与禁用状态，顶部继承锁也保留锁定但禁用。自有locked不变，眼睛独立。未重建EXE，颜色待验证。

## 2026-11-24 参数框与继承状态本地构建（未发布）

- 用户明确授权；构建前无unity_yaml_viewer进程，固定EXE独占访问通过。cargo build --release --locked --offline成功，日志target/tile-panels-lock-state-release-build.log。
- EXE文件／产品版本1.3.7／1.3.7，34912768字节，SHA-256 756ee6ec88e91a0ec17c34324a06e83d968569f72d681a0dc44468cbd9a889c9；target/release/unity_yaml_viewer.exe.sha256已更新。
- 包含参数框自动收缩、标题简化、统一240曲线宽度、调整层隐藏剪切菜单、父组隐藏时子对象眼睛灰显、继承锁复用BaseLayer禁用样式；子对象自身状态保留，眼睛独立。
- 未自动启动、未发布；实际参数框宽高、菜单及组状态仍待用户验证，应用版本保持1.3.7。

## 2026-11-24 停用蒙版斜线（源码）

mask_enabled=false时缩略图加红色斜线，启用即移除，不改数据或合成。未重建EXE。

- 2026-11-24：FX统一开关面板及样式复制／粘贴／清除菜单完成；检查通过见 target/tile-fx-context-check.log。包含此前停用蒙版红线的修改均尚未重建 EXE，待用户授权及界面验证。

## 2026-11-24 绘 Tile 八种图层样式

- 主程序 1.3.7 的 fx 现按效果分页编辑，补齐内阴影、内外发光、渐变叠加及斜面和浮雕，并扩展描边、投影、颜色叠加。参考 PhotoCraft 提交 `b84c777860324526e8da83690331a911e5c57931`，源码路径、Tile 适配范围与限制见当前版本记录。
- 填充为 0% 时保留样式，图层不透明度统一控制整体。工程格式 10 写入参数及工程全局光，兼容旧格式。静态检查见 `target/tile-fx-photocraft-check.log`。尚未构建新 EXE，界面及像素回归用例待运行。

## 2026-11-24 Tile 文字水印、自由变换和多项 fx

- 主程序源码保持 1.3.7。矩形文字层可通过 T 添加，行末箭头展开字符／段落属性，系统字体按需读取，样张后台缓存，手动刷新识别新安装字体。移动和选区增加 Ctrl+T。fx 使用独立置顶窗口，五类效果支持多项。工程格式、适用对象和限制见当前版本记录。
- 编译检查日志 `target/tile-text-fx-check.log`。未重建 EXE，交互、实际字体安装刷新和导出仍待实机核验。

## 2026-11-24 Image / Tile 和 PSD

- 普通图片与 PSD 共用“编辑图像”。Image 和 Tile 有独立工作状态，工程文件固定 Tile。Image 可原位保存或另存六种格式。支持的 8 位 RGB PSD 可保留像素层、组及蒙版。原生复杂结构保留源文件，仅允许将兼容合成图另存。
- 核验入口和兼容范围见当前版本记录。测试目标与主程序的编译检查日志为 `target/image-tile-psd-check.log` 和 `target/image-tile-psd-app-check.log`。未生成测试程序或新 EXE，Photoshop 文件往返与窗口实机交互待验证。

## 2026-11-24 单图拆分与图层拖放

- 主程序 1.3.7 的 Tile 设置使用“拆分 Tile / 多 Tile”。1×1 保持单图，应用拆分可生成互相独立的矩形或菱形格子。切换预览和 Slot 不标记修改。Image/Tile 标题随当前页切换，支持外部图片拖入普通图层。
- 逐像素核对 432 组几何组合。常规及测试目标的检查日志见 `target/tile-split-check.log` 与 `target/tile-split-check-tests.log`。新 EXE、Rust 测试程序和窗口实测均未执行。

## 2026-11-24 当前格与整图导出

- 当前格导入与 PNG 保存位于格子选择下方。多格额外提供逐格和合并 PNG 导出。合并图使用矩形、菱形或六边形的原始像素排列与透明度。检查日志见 `target/tile-merged-png-check-tests.log` 与 `target/tile-merged-png-check.log`。版本仍为 1.3.7，导出对话框和真实窗口操作待验证。
- 2026-11-24 经用户授权编译。主程序进程及固定 EXE 独占访问检查通过后，`cargo build --release --locked --offline` 成功，耗时 1 分 29 秒。产物 `target/release/unity_yaml_viewer.exe` 的文件版本和产品版本均为 1.3.7，大小 35,784,704 字节，SHA-256 为 `58e14bce1984bb52a0257faae89ced4ade8649e143019944290087551b0ec3e6`。相邻校验文件已更新，构建日志为 `target/tile-image-split-export-release-build.log`。窗口和 PNG 导出待实机验证。

## 2026-11-24 Image 尺寸调整和背景色输出

- 主程序源码 1.3.7 增加 Image 画布裁扩和整图重采样，可切换百分比／像素并选择四种插值。Alpha 开关控制普通图片的透明或背景色输出，分层和合成 PSD 始终保留透明度。
- 静态编译检查日志为 `target/image-size-alpha-test-check.log` 和 `target/image-size-alpha-check.log`。上次构建的 EXE 早于这项改动，窗口、实际输出和新回归用例仍待运行验证。
- 2026-11-24 经用户授权，通过主程序进程与固定 EXE 独占检查后，`cargo build --release --locked --offline` 成功，耗时 1 分 24 秒。产物 `target/release/unity_yaml_viewer.exe` 为 35,960,832 字节，文件版本与产品版本均为 1.3.7，SHA-256 为 `d23e31067fd98d9d9b6af009c94f55c833e9221cae7a902d0a8f6b709c819c4f`。已更新相邻校验文件并核对内嵌的尺寸和 PSD 透明度提示。日志为 `target/image-size-alpha-release-build.log`，界面与实际导出待验证。

## 2026-11-24 Image 相对尺寸和输入框对齐测试版

- 经用户授权并确认主程序和固定 EXE 均无占用后，Release 构建完成，耗时 1 分 25 秒。产物 `target/release/unity_yaml_viewer.exe` 为 35,893,248 字节，文件及产品版本均为 1.3.7，SHA-256 为 `ba723c434cc65abf0e7b69ede631909b038a26a22cc5cee944db21e195033a54`。已核对内嵌的相对输入提示和“两次线性”名称，并更新相邻校验文件。
- 尺寸模块 4 项回归测试全部通过。构建和测试日志分别为 `target/image-size-relative-release-build.log` 与 `target/image-size-relative-regression.log`。输入框等宽、垂直居中与鼠标交互待实机验证。
## 2026-11-24 PSD 原生样式与画布编辑测试版

- 经用户授权并确认主程序和固定 EXE 无占用后，`cargo build --release --locked --offline` 成功，耗时 1 分 11 秒。产物 `target/release/unity_yaml_viewer.exe` 为 35,988,480 字节，文件及产品版本为 1.3.7，SHA-256 为 `1fe7ac056304153d614f3ad2531b812fd1a361b86b45d17039777a50f689b824`。已更新相邻校验文件，构建日志见 `target/editor-psd-guides-release-build.log`。
- 本地测试版包含 PSD 原生 FX、画布文字、标尺及辅助线吸附、紫罗兰色对齐线和图层键盘微移。实际窗口交互和 Photoshop 双向兼容性待验证。
## 2026-11-24 矢量与文字编辑测试版

- 按用户授权并确认 EXE 无占用后完成 1.3.7 Release 构建，耗时 1 分 23 秒。产物 `target/release/unity_yaml_viewer.exe` 为 36,283,392 字节，SHA-256 为 `3e68906845c333e0c4fc0317ee8c02e056998763c8e5a2467f438139f19c5194`。已核对文件及产品版本并更新相邻校验文件。日志见 `target/editor-vector-text-release-build.log`。
- 本地测试版包含矢量图形 U、像素框画 K、PSD 矢量和图层元数据读写、文字区间样式及最近的面板调整。实际窗口及 Photoshop 往返待验证。
## 2026-11-24 PSD 图案资源和矢量工具栏测试版

- 经用户授权并确认目标程序无占用，1.3.7 Release 构建通过，耗时 1 分 12 秒。产物 `target/release/unity_yaml_viewer.exe` 为 36111872 字节，SHA-256 为 `137255736c021831aa453e31c7119c235bde850d19895203c85e4e4169bfa728`。已核对版本并更新相邻校验文件，构建日志见 `target/psd-pattern-toolbar-release-build.log`。
- 测试版包含 PSD 图案库分层读取修复、无滚动浮动工具栏及重绘的矢量图标。用户原 PSD 和窗口显示待实机验证。

- 2026-11-24 经用户授权并检查目标 EXE 无占用，PSD 兼容性调整后的 Release 构建成功。文件版本 1.3.7，大小 36572160 字节，SHA-256 131fe22954216169ab83230c91274a613caf36d62d11d8488d3b31367748fd80。日志 target/psd-compatibility-release-build.log。

- 2026-11-24 经用户授权并检查程序无占用，Txt2 同步的 Release 构建成功。文件版本 1.3.7，大小 36571136 字节，SHA-256 0ad08cbea332b0793c6936ff54411c489f1d762127653a6857afa62139e89046。构建日志 target/psd-txt2-release-build.log，SABI 回归日志 target/psd-txt2-fixture.log。仅修改像素层、修改文字及字体的两个副本均通过 Photoshop 2024 无提示打开和继续编辑验证。

- 2026-11-24 经授权并检查固定 EXE 无占用后构建 PSD 元数据往返版。版本 1.3.7，大小 36475392 字节，SHA-256 7a2bd85599f71b75afcd816813048e04b262a3dd089f40f4020a7b125afe5b7a。SABI 改字导出与 Photoshop 实测副本哈希一致，Photoshop 改字、重存、重开后四个块的内容与原件一致。日志 target/psd-metadata-release-build.log 和 target/psd-metadata-fixture.log。

- 2026-11-24 经用户授权并检查目标 EXE 无占用，保存状态和标题星号的 1.3.7 Release 构建成功。文件大小 36473344 字节，SHA-256 c07191984cde70b9c2f8c9329842ff99e9fdebdf8351f5598daa53d921b640c6。日志 target/image-save-state-release-build.log，界面待实测。

- 2026-11-24 经授权并检查 EXE 无占用后完成 Tile 工程保存相关 Release 构建。版本 1.3.7，大小 36456960 字节，SHA-256 7228db73a65d860451c44f4263569c43bd36103982a3b73ea123888245dd61e7。日志 target/tile-project-save-release-build.log，界面待实测。

- 2026-11-24 经授权并检查目标 EXE 无占用后构建菱形选区修复版。版本 1.3.7，大小 36456960 字节，SHA-256 87fef8fbcd5ab9a6bd4a7c3446e7379c030b85e231720bbef84971e9aee2e91a。日志 target/diamond-selection-release-build.log，松手交互待实测。

- 2026-11-24 检查固定 EXE 无占用后按授权构建 Tile 整组绕回版。版本 1.3.7，大小 36619264 字节，SHA-256 01381333b34b0f9f144d7aa4b2d8299811b317e007f178427936ec1a787895ea。日志 target/tile-geometry-wrap-release.log 和 target/tile-geometry-wrap-tests.log。2×2 回归通过，未覆盖源工程。

- 2026-11-24 经授权并检查目标 EXE 无占用后重新检查 Tile 绕回版 Release 构建。版本 1.3.7，大小 36619264 字节，SHA-256 d781de55455bc8691cf8aaebb6cb00b30a6bd9808787d15c72d827e5da5935cb。日志 target/tile-wrap-verification-build.log。


- Canvas 粒子和右下角父子播放控制，真实样例 ui_double_production_drop.prefab 自动验证通过。Release 构建完成，窗口视觉核验待完成。产物 target/release/unity_yaml_viewer.exe · 36734464 bytes · SHA-256 15297A297EB540777AF61ED6BBAA9B651111940E364AF424014A013FC2C93EBA。


- 多格 Tile 文字绕回渐变测试版已构建。产物 target/release/unity_yaml_viewer.exe · 36716032 bytes · SHA-256 C8409E6827DAC336B3EEEFFF7C76B92588082A55648A258D231728FD08CED603。用户已在 2×2 菱形工程中验证文字绕回渐变连续。


- Prefab 动画预览窗口测试版已编译。产物 target/release/unity_yaml_viewer.exe · 36653568 bytes · SHA-256 51D14728013E5E3A9D2088121E579690B8DCD7A9F79B4E32213CBDD0A3CAC7CA。跨屏窗口、播放和写回待界面验证。


- Animator／Animation Inspector 动画预览入口已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36728832 bytes · SHA-256 BF4A5CBAE90489D1131B34034A1E14B2CDBA77F5990E050EF50D25EB6B141173。入口与窗口交互待实测。


- 动画窗口布局和关键帧过滤锁已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36835840 bytes · SHA-256 7CCD8DD93A72ADFF89CCDA408FAE817569A1EC8AF8FE03DE5C8632B3375BB150。界面待实测。


- Controller 参数分栏、逐行 Clip 时间轴、准星过滤和图标播放控件已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36878336 bytes · SHA-256 0E8F54DE0B1C3E90000FFE1C54F5C9673A13D8DB191A6093FF3015543C3BE85E。界面待实测。


- AnimationClip 总时间轴和关键帧编辑合并已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36782592 bytes · SHA-256 1438BC57FC370B07E4ECD555E720E25F5739587EABEA9967EE50ACE7489C7473。界面待实测。


- AnimationClip 关键帧／曲线页签、固定标尺及精简轨道名已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36720128 bytes · SHA-256 D802BE2534D2D0A61D9CF60B8D0B6D68120E888BD24FF4F8158AFA7FBB38D3C5。界面待实测。


- AnimationClip 曲线右栏铺满、轨道色点、分量缩进、连体 Tab 和分割线已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36833792 bytes · SHA-256 D5E8AC64E2E23872AAA9C6CD882163BB8569CEA2F6CB4565114F1E97FE2A9AE7。界面待实测。


- AnimationClip 的 IsActive 粒子启停、Image Alpha 曲线及布尔复选框已纳入 1.3.7 测试版。产物 target/release/unity_yaml_viewer.exe · 36995072 bytes · SHA-256 E29BB4C4385930F860B6989813831BF109B349D996A8A5D8905267328A46D653。实际 Prefab 与界面待验证。

## 2026-11-26 1.3.9 画中画录屏测试构建

- 已在确认目标 EXE 无占用后完成 Release 构建。产物 target/release/unity_yaml_viewer.exe，文件版本 1.3.9，产品版本 1.3.9，38166016 字节，SHA-256 7CA76811600D2311A7B05AC645570D26F6684447B4012D8AC3083C3AA6EA691F。构建日志 target/pip-1.3.9-release-build.log。
- 用于 USB 摄像头及画中画录屏实测。本包基于上次通过检查的矢量功能，PSD 多子路径和异色分组写回仍待实现和 Photoshop 往返验证。

## 2026-11-26 1.3.9 镜像预览及复合形状 PSD 验证

- Release EXE 文件及产品版本均为 1.3.9，大小 38,437,376 字节，SHA-256 775E083C8CEBE95214F7076A0CD8934DCBC144426EA204E711EF77596195318E。产物 target/release/unity_yaml_viewer.exe，日志 target/mirror-psd-1.3.9-release-build.log。
- PSD 回归测试通过。Photoshop 2024 25.3.1 对同色、异色样例分别打开、移动可编辑形状层、另存、重开。同色保留一个形状层，异色保留一个组内的三段形状层，移动保留。另存前后 PSD 路径操作码 1、2、3、0 一致。样例与日志在 target/psd-1.3.9-qa。
- 摄像头和画中画镜像切换待用户实测。画中画录制掉帧未解决，镜像预览的修复与此独立。

## 2026-11-26 1.3.9 画中画时钟修复实测

- Release 已构建。产物 target/release/unity_yaml_viewer.exe，版本 1.3.9，38,402,560 字节，SHA-256 1FE33C94A0CFFBAA4E20CFAC052FCE2D5EB149C507E7FB51E2664A185185C97F，日志 target/pip-timing-1.3.9-build.log。
- USB 摄像头、系统声音和麦克风全开启，1080p 30 FPS。样本 recording-1795702824766322100.mp4，347 帧、视频 11.588 秒、平均 29.94 FPS、最长帧间隔 54.7 毫秒，首帧从 0 秒可正常解码。NVENC 实时编码，停止后约 2.6 秒完成。倒计时结束到首次编码报告仍约 4.6 秒。

## 2026-11-26 1.3.9 画中画预热及全屏播放测试构建

- Release 构建通过，产物 target/release/unity_yaml_viewer.exe，文件版本 1.3.9，产品版本 1.3.9，38401024 字节，SHA-256 B5EE04F3F221B8959ECD1FD7C26CD1070324743A4B442567B7A9B988143E838C。日志 target/pip-warmup-1.3.9-build.log。
- 画中画预热、倒计时结束到首帧时间、黄边及本地 1080p 视频从窗口切换全屏的清晰度由用户实测。本轮未启动 Computer Use 截图。

## 2026-11-26 1.3.9 仅屏幕录制预热测试构建

- Release 构建通过。文件及产品版本分别为 1.3.9 与 1.3.9，产物 target/release/unity_yaml_viewer.exe，38178816 字节，SHA-256 F4F375148E13945320741083DC9C0825754ACBF91AE7FD3843F0B67B1C5B1BF6。日志 target/screen-warmup-1.3.9-build.log。
- FFmpeg 合成源验证仅屏幕预热支路可在成片通道开启前收到帧，开启后输出从 0 秒开始。实际屏幕录制、悬浮窗箭头和音频收尾待用户实测。

## 2026-11-26 1.3.9 画中画音频同步测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38179328 字节，SHA-256 886AE0D5E76BF050920CAADD528A4D851C4E8ABADFB8626A75872E67D4DB2D60。日志 target/pip-audio-1.3.9-build.log。
- 双路音频合成源验证通过。实际 USB 摄像头 1080p 30 FPS、系统声音加麦克风的口型同步及人声清晰度待用户验证。

## 2026-11-26 1.3.9 录制诊断测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38226944 字节，SHA-256 4F50535B017EA7FB3695000C32F635DAFC864BA08D6EC153A60628BF96D3FE35。日志 target/pip-diagnostics-1.3.9-build.log。
- FFmpeg 模拟源验证正常混音成片、两条 WAV 和输入时间戳可同时输出。实际 USB 双音源拍手样本待用户录制。

## 2026-11-26 1.3.9 画中画音频预热测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38005760 字节，SHA-256 AC3934F5E22CE9FC29A45B155C076FAFD933FD38B57368C747B3BDF29D579A7C。日志 target/pip-audio-warmup-1.3.9-build.log。
- 合成双音源验证两路放行命令成功，输出音画均从零开始。真实 USB 双音源拍手样本待用户录制核对。

## 2026-11-26 1.3.9 画中画合成时钟测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38030848 字节，SHA-256 B17824D67DF82F84E6E01DDC314FB8056BFF7CF7696094F822CCCDBB7BC089DD。日志 target/pip-composite-clock-1.3.9-build.log。
- 合成双视频和双音源验证通过。实际 USB 1080p 30 FPS 诊断拍手样本待用户录制。

## 2026-11-26 1.3.9 画中画混音前预热测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38024704 字节，SHA-256 219CA40D178888CEE66DD4BD89DC2EAD426F4B457350E52F8E461E78DA178B93。日志 target/pip-premix-clock-1.3.9-build.log。
- 合成双音源验证三路放行命令成功。实际 USB 摄像头 1080p 30 FPS 两次拍手复测待用户录制。

## 2026-11-26 1.3.9 Photoshop 同层多矩形读取测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38350336 字节，SHA-256 D9DEDBF611FF665EDBBC5E6CB366446E7C4C50E3E3B4F9B0E9676EFF36AFE345。日志 target/psd-vowv-1.3.9-build.log。
- 已用样本核对 vowv 版本 2 和两条 vsms 闭合矩形路径。实际应用重开、编辑与 Photoshop 往返待用户实测。

## 2026-11-26 1.3.9 矢量缩略图色板修复测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38351360 字节，SHA-256 45CBD3DD79563578F12282BC5B03937945AFB5C84F93B2724E20874D63BD55EF。日志 target/vector-fill-1.3.9-build.log。
- 修复选色时在列表遍历中清空缩略图导致的越界，移除缩略图的填充悬浮提示。界面待实测。

## 2026-11-27 1.3.9 矢量锚点和填充测试构建

- Release 构建通过。文件版本 1.3.9，产品版本 1.3.9，产物 target/release/unity_yaml_viewer.exe，38395904 字节，SHA-256 D1CA96A59134D2F2D7219B19EBBD410BF69E6EC5F92E4587BE900600D5B5B0D9。日志 target/vector-anchor-fill-1.3.9-build.log。
- 新建矢量默认填充、已有形状独立填充与复合形状锚点选取待用户实测。

## 2026-11-27 1.3.9 路径选择、锚点框选与子路径分层测试构建

- 主程序 Release 构建完成，用时 1 分 42 秒。文件版本和产品版本均为 1.3.9。产物 `target/release/unity_yaml_viewer.exe` 为 38,371,840 字节，SHA-256 为 `9347F4B86159E1AB0D5CC4A5DBCE92C06FD81FFF969FBD46BAF3A0AD3A328D98`。相邻校验文件已更新，构建日志在 `target/path-selection-1.3.9-build.log`。
- 2026-11-27 用户确认路径选取、框选锚点及两个分层快捷键实测通过。Ctrl+J 复制选中形状并保留原形状，Ctrl+Shift+J 将选中形状剪切到新层并从原层移除。

## 2026-11-27 1.3.9 画中画录屏 CPU 优化测试构建

- 主程序 Release 构建完成，用时 1 分 42 秒。文件版本和产品版本均为 1.3.9。产物 `target/release/unity_yaml_viewer.exe` 为 38,407,680 字节，SHA-256 为 `1FB344473BFF13A957E3021CB32587487287F0A5766B4532CFBE126BE9C593FA`，已核对相邻校验文件。构建日志在 `target/pip-cpu-1.3.9-build.log`。
- 已在 EXE 中核对单像素预热滤镜和 FFmpeg CPU 诊断日志标记。G HUB 运行与退出两种状态下的画中画实际帧率及 CPU 用时待用户对照实测。

## 2026-11-27 1.3.9 画中画掉帧修复测试构建

- 用户关闭主程序后，进程和固定 EXE 独占检查通过。Release 构建用时 1 分 33 秒。产物 `target/release/unity_yaml_viewer.exe` 的文件版本和产品版本均为 1.3.9，大小为 38,572,544 字节，SHA-256 为 `89C05055B50EBA20EAE00FAA7A7B5F0AA5E07A83FFBAA071ED38A424D417A305`。相邻校验文件已更新，构建日志位于 `target/pip-capture-pacing-1.3.9-build.log`。
- 已在产物中核对独立采集滤镜标记。用户确认双音源画中画实录流畅、开销低、拍手音画同步，实时预览无可感延迟。此次 1080p NVENC 成片为 9.575 秒、261 帧，首个帧间隔为 142 毫秒，之后最长约 67 毫秒。详细测量见 `docs/versions/1.3.9.md`。
