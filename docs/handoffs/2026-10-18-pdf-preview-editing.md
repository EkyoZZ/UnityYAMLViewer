# PDF 预览与编辑接续交接（2026-10-18）

## 新会话先读

- 工作目录：`C:\Work\Git\UnityYAMLViewer`；Windows / PowerShell，文本保持 UTF-8。
- 当前版本 `1.2.3`，分支 `master`。本地 HEAD 为 `1179103`（完善音频编辑与文件堆叠页签），本地 Git 显示与 `origin/master` 同步；这不是本轮重新联网核验的远端状态。
- **下一步是正式接入 PDF 预览与有限编辑。目前只有第三方组件管理，没有 PDF 页面渲染，也没有任何 PDF 编辑功能。** 用户反复确认并理解这一状态。
- 本轮用户只要求交接文档，不要在交接轮继续开发、构建或发布。新会话在用户要求继续后开展实现。
- 必须读取根 `AGENTS.md`。未经新一轮明确编译授权，不运行生成可执行程序的命令（包括 cargo test 会编译测试 EXE）；可以 `cargo check --tests --locked --offline`。授权编译后，先检查目标 EXE / 测试 EXE 进程和独占访问；发现占用不能自动关闭，需用户关闭并再次确认。
- 用户明确不发布版本。不要自动升版、打标签或创建 Release。Git 推送曾被自动审批拒绝，之后用户选择自己推送；本轮没有新的提交 / 推送授权。

## 用户已确定的方向

1. PDFium 采用按需下载的动态 DLL，参考 FFmpeg 的管理方式，不把引擎静态打进主 EXE；当前选择 Windows x64、不带 V8 的构建。
2. 设置提供“第三方支持”入口，打开独立原生管理弹窗，FFmpeg / FFprobe、PDFium、LibreOffice 集中管理。
3. 缺少组件时提示直接跳转 / 唤醒该管理弹窗，不再跳到设置。
4. **第三方组件及下载暂存不能被软件缓存管理删除。** PDFium 专用目录必须在缓存清理范围外；FFmpeg / LibreOffice 通过系统 / WinGet 安装并管理。
5. LibreOffice 已安排管理入口，未来用于 Office 文档转换预览；目前没有 Office 或苹果办公文档预览 / 编辑。不要把这些计划写为已支持。
6. PDF 的预览和编辑要分层实现。讨论中已说明：页面管理、批注和有限页面对象修改可行；Word 式自动重排、扫描 PDF 的 OCR 文字编辑不是现成能力。具体首批编辑范围仍需落实，以下路线属于建议，不是已完成或已逐项确认的需求。

## 当前源码：第三方支持管理

核心文件 `src/third_party.rs`（未跟踪，尚未提交）：

- `Manager::open/show/is_open`：原生 `third_party_window`，从设置入口打开。
- `request_open/take_open_request`：统一缺失组件跳转入口；当前媒体错误按钮已复用，未来 PDF 缺失提示也应使用。
- FFmpeg 原管理器继续在 `src/media_tools.rs`；通过 WinGet `Gyan.FFmpeg` 安装，异步检测 FFmpeg / FFprobe 并显示路径。管理窗口关闭后，已安装通知会重试当前失败媒体。
- PDFium 从 `bblanchon/pdfium-binaries` 最新 GitHub Release 查找 `pdfium-win-x64.tgz`。限定仓库地址 / 附件名，要求 GitHub asset.digest SHA-256，核对大小，下载上限 64 MiB；有进度和取消。缺失摘要、限流、网络和解压错误均失败而非跳过验证。
- 解压使用系统 `tar`：读取清单、以 `tar -xOf` 输出 DLL 和许可文本，再写入指定文件名，不按归档路径直接落盘。最终 DLL 64 MiB、单许可文件 2 MiB 上限；保存许可文本。真实 Release 包结构与实际下载流程尚未端到端验证。
- `validate_library` 暂时只加载 DLL 并检查必要导出符号，随即卸载；**没有初始化 PDFium，没有打开 PDF 或渲染页面**。已有 `libloading = "0.8"`，尚未引入 `pdfium-render` 或 PDFium 完整 FFI 绑定。
- `Installed` / `component_root` / `read_installed` / `Installed::path` 目前是私有实现。下一步应提供受控的、验证后的 DLL 定位接口，不复制管理记录逻辑，更不能绕过校验盲目搜索或下载 DLL。
- `libreoffice_path.txt` 支持自选 `soffice.com` / `soffice.exe`；检测标准目录、PATH 与保存路径，以 `--headless --version` 验证，单次探测 15 秒上限。WinGet 安装包 `TheDocumentFoundation.LibreOffice`，系统级 MSI 可能需要管理员确认。没有执行文档转换。
- 未提供系统 FFmpeg / LibreOffice 的卸载。PDFium 有单独移除，升级目前可移除后下载最新构建。后续接入 PDFium 后，必须考虑打开的 PDF / DLL 生命周期：使用中应阻止或延迟移除，不能强制卸载仍被句柄依赖的库。

### 目录与清理边界

- 配置 / 缓存根：`%APPDATA%/UnityYAMLViewer`。
- 独立组件根：`%APPDATA%/UnityYAMLViewer-components`，与缓存根同级。
- PDFium：组件根下 `pdfium-<pid>-<timestamp>/pdfium.dll`，同目录有 `licenses/`；登记文件为组件根 `pdfium.json`，记录目录、版本与 DLL SHA-256。
- 当前 `src/app_cache.rs::roots/cleanup` 只管理原配置 / 缓存根及特定临时缓存，不扫描上述同级组件目录。后续修改缓存功能务必继续排除它。
- 正常失败 / 取消清理本轮暂存；异常进程退出可能留下未登记目录，缓存清理不会替用户删除这些组件残留。不要无校验地递归删除目录。

## PDF 接入建议与验收点

### 第一步：只读预览

- 先评估 `pdfium-render` Rust 封装与已下载 PDFium 导出 API 的兼容性，再决定绑定方案；默认离线检查，缺少依赖时如实报告，不假设依赖已缓存。
- 从第三方支持模块取得已验证 DLL；加载 / 初始化集中管理，确保库、文档、页和后台线程的生命周期正确，按照所选绑定的线程安全要求串行或专用线程调用。
- 新建独立 `pdf_preview` 模块，不挤进 SVG / 位图解码器。参考 `src/asset_viewer.rs` 分流与 `svg_preview.rs` 的后台加载 / 错误 / 缩放方式，但 PDF 需要文档句柄、页缓存与多页 UI。
- 接入文件过滤器、拖入、命令行、最近打开、会话恢复、页签分组、支持格式 / 文件关联、URL `application/pdf`，仅在真正支持后登记 `.pdf`。
- 建议第一批：当前页 / 总页数、上一页 / 下一页、直接跳页、适应窗口 / 宽度、缩放、拖动或滚动，随后增加缩略图、文字选择和搜索。具体 UI 可沿用现有主题与独立预览风格。
- 页面后台按当前显示尺寸 / DPI 渲染，限制单页像素 / 内存 / 总缓存，避免一次渲染所有页面；快速切页 / 缩放须丢弃过期任务结果，关闭页签正确释放资源。
- 错误路径包含缺组件直接开管理窗口、密码保护、损坏文件、超大页面、渲染失败。组件下载 / 检测完成后能重试原文件，不能通过更换文件路径绕过已有页签。

### 第二步：有限编辑

- 优先评估页旋转、删除 / 插入 / 重排、拆分 / 合并，随后批注 / 高亮；页旋转需区分只改视图与保存文档属性。
- 文字 / 图片对象修改单独评估，不能承诺原文自动重排。扫描图像需要 OCR 才能识别文字，当前没有 OCR 引擎。
- 编辑前明确保存策略（建议先提供另存为、不默认覆盖）、撤销 / 重做及当前设置中的撤销上限；失败不得标记为已保存。
- 未保存关闭提醒必须兼容普通页签 ×、堆叠菜单单项关闭、关闭全部和退出。关闭全部有队列与保持组状态，取消应停止剩余关闭；不要直接 `assets.forget` 导致丢失编辑。
- 文件有签名、权限或加密时应明确处理限制，不能宣称修改后数字签名仍有效。
- 原始文档和组件库是不同生命周期；禁止因清理页缓存而删组件 DLL。

## 相关工程入口

| 文件 | 用途 |
| --- | --- |
| `src/main.rs` | 设置入口、第三方窗口调度、文件打开分流、统一页签与关闭提醒、关于摘要 |
| `src/third_party.rs` | FFmpeg / PDFium / LibreOffice 管理与缺组件跳转 |
| `src/media_tools.rs` | FFmpeg 查找与 WinGet 管理 |
| `src/asset_viewer.rs` | 图像 / SVG / 媒体预览分发与资源回收 |
| `src/asset_viewer/svg_preview.rs` | SVG 后台只读渲染示例，背景切换而非通道 |
| `src/file_associations.rs` | 支持格式与文件关联共用清单 |
| `src/remote_file.rs` | URL 下载与 MIME 后缀识别 |
| `src/app_cache.rs` | 缓存扫描 / 清理边界，第三方组件必须排除 |
| `src/editing.rs` | 现有资源编辑状态和保存提醒供参考；PDF 不应直接套用 YAML 编辑器 |
| `src/translations.tsv` / `src/localized_format.rs` | 七语言与带参数文本，新增 lf! 模板需同步宏 |
| `docs/versions/1.2.3.md` | 当前能力、限制和验证状态；README 不写内部版本流程 |

## 已有 SVG / GIF 功能，接续时不要回退

- SVG：只读、缩放 / 平移 / 重载、棋盘格 / 灰色 / 自定义背景；无通道切换，九宫格入口隐藏，程序化位图编辑也拒绝 SVG。限制包括文字转轮廓、外部资源 / 内嵌位图 / 动画未支持，文件 16 MiB、渲染单边 4096。
- GIF：FFmpeg 播放，四种循环模式，时间轴按实际帧时长显示帧数和当前帧秒数；隐藏音量 / 音轨 UI，原音频信息行显示全局 / 局部调色板最大容量（不是实际用色数量）。
- 页签：同后缀超过 3 个堆叠，文件堆叠 SVG 图标及数量，支持整体拖拽；逐项减少至 3 个在原位展开，关闭全部期间不提前解散，取消停止后续关闭。
- 已修复三组旧音频关于摘要 TSV 被拆成单列的问题：合回七列，未改译文。新增翻译继续严格七列。

## 验证与本地产物：特别注意源码和 EXE 不一致

- 最近源码检查：`cargo check --tests --locked --offline` 通过，日志 `target/third-party-routing-check.log`；第三方管理及跳转代码尚未构建为 EXE，第三方管理测试仅编译检查，未运行断言。
- 最后一次实际 release EXE 是 SVG 背景设置构建，版本 1.2.3，路径 `target/release/unity_yaml_viewer.exe`，22,957,568 字节，SHA-256：`576E149087C6C5BAF94E22CD924BB9628C793C7202E164F0B4E84686B75EFDE4`（本交接轮重新核对）。它**不含第三方支持管理窗口**。
- 对应构建日志 `target/svg-background-release-build.log`，SVG 测试日志 `target/svg-background-tests.log`，当时 1 项渲染测试通过。较早 GIF 色表 / 循环 / 帧时长 3 项测试通过，日志 `target/gif-palette-tests.log`。
- 未实际下载 / 安装 PDFium 或 LibreOffice，未做真实下载 / 移除 / 管理窗口 GUI 测试；不要以导出符号检测代码或编译通过替代这些验证。
- 新功能必须同轮更新版本记录、README 中英文实际能力、当前关于摘要七语言。关于默认仅当前版 1.2.3 和上一版 1.2.2，其余展开历史；重新打开恢复折叠。设置最后两项依次为“单个文件缓存撤销步骤”“检查更新”，相邻且更新最后。

## 未提交改动快照

交接时 `git status`：

```text
 M Cargo.lock
 M Cargo.toml
 M HANDOFF.md
 M README.md
 M docs/versions/1.2.3.md
 M src/asset_viewer.rs
 M src/file_associations.rs
 M src/localized_format.rs
 M src/main.rs
 M src/material_graph.rs
 M src/media_preview.rs
 M src/media_tools.rs
 M src/remote_file.rs
 M src/translations.tsv
?? src/asset_viewer/svg_preview.rs
?? src/gif_playback.rs
?? src/third_party.rs
```

另有本交接文档。不要重置这些改动，尤其三个未跟踪的实现文件。实际继续前再次 `git status`，以当时结果为准。

## 可复制到新会话的提示

> 继续 UnityYAMLViewer 的 PDF 预览与有限编辑接入。请先阅读 AGENTS.md、HANDOFF.md 顶部，以及 docs/handoffs/2026-10-18-pdf-preview-editing.md。当前只有 PDFium / LibreOffice / FFmpeg 第三方管理源码，PDF 预览编辑尚未实现，最新 EXE 也不含管理窗口。先接通已验证 PDFium DLL 的加载生命周期与只读多页预览，再落实有限编辑范围；缺组件直达第三方管理，组件不得被缓存清理删除。保留工作区现有未提交改动，不发布版本；生成 EXE 前须另行取得编译确认并检查占用。
