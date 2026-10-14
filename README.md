# Unity YAML Viewer

[中文](#中文) | [English](#english)

---

## 中文

**Unity YAML Viewer** 是一个面向 Unity 文本序列化资源的桌面工具。它将 `.prefab`、`.unity`、`.asset` 等 YAML 文件解析为更易理解的层级和 Inspector 视图，帮助你在不打开 Unity 编辑器时检查资源内容。

项目现已支持**属性编辑**：通过 Inspector 控件修改已识别的值，确认保存后写回原文件。类型校验、引用保护和精确片段写回帮助减少手动编辑 YAML 时的缩进、引用或格式损坏问题。

### 已支持

- 场景 / Prefab 静态 2D 与 3D 预览：Sprite、Tilemap、常见 UI、模型及 Spine 静态合成；支持同文件 fileID 引用的未压缩文本三角 Mesh（线拓扑暂不支持）；支持嵌套 Prefab 展开和常见覆盖，按层索引实例引用并共享同源模板（模板 / 展开文本各 512 MiB，每层最多 50 万文档）；展开状态下场景点选嵌套 Prefab 会展开祖先、滚动并高亮对应节点；引用节点属性只读；展开限制详情可一键复制。
- GPU 场景预览首次渲染完成后，鼠标离开画面或程序失去焦点时冻结最后一帧，返回后恢复；冻结画面在窗口缩放时保持比例。
- GPU 场景预览首次渲染完成后，鼠标离开画面或程序失去焦点时冻结最后一帧，返回后恢复；冻结画面在窗口缩放时保持比例。
- Spine 3.8 独立骨骼预览：动画、皮肤、多轨播放、速度、时间轴、缩放和平移。
- FBX / OBJ 模型预览：贴图、颜色、实体 / 线框、网格统计、静态蒙皮与视角操作。
- 图片通道查看、缩放和平移；常见脚本 / 文本查看、基础语法高亮及工程字体预览。
- 音频 / 视频播放：播放、暂停、停止、进度跳转、音量及空格键控制；视频支持音轨选择、暂停方向键逐帧及自动 / 软解 / 硬解选项。
- 视频适应窗口、原尺寸、自由缩放与全屏播放：F11 切换、Esc 退出，全屏悬浮控件在鼠标静止 1 秒后隐藏。自由缩放支持鼠标锚点缩放、拖动及轻点播放 / 暂停。
- 音频波形、dBFS 电平条及白色峰值拖尾，峰值线以每秒 5 dB 回落；电平为源音频近似峰值。
- 设置安装 / 检测外部 FFmpeg，安装就绪后关闭设置自动刷新当前失败媒体页。
- 页签拖拽排序、启动恢复及最近打开记录（10～50 条，默认 30 条，可在设置中清空）；后台文件加载与资源索引、可配置的预览默认展开及 Hierarchy 展开层级。
- 简中、繁中、英语、日语、韩语、德语、法语与系统 / 浅色 / 深色主题。
- GitHub Releases 在线更新（自动读取 Windows 系统手动代理及代理环境变量；不支持 PAC / 自动代理发现）：自动 / 手动检查（结果显示在设置底部，检查失败提供手动更新链接；有新版本时手动检查与 New 共用更新窗口）、发行说明、下载进度、大小 / SHA-256 / 架构校验，未保存修改保护及备份替换重启。

- Prefab 静态 Sprite 组合预览：按层级 Transform 叠加位置、缩放和旋转，支持源图切片、颜色和翻转；可拖动调整预览宽度、缩放及平移。选中 SpriteRenderer 时同时查看和编辑所属节点的 Transform/RectTransform。
- UI Prefab 预览支持 Image 普通/九宫格及横向/纵向填充、RawImage、Text/TextMeshPro 静态文字；按 RectTransform 锚点、pivot、尺寸及层级变换计算布局，点击选中对应 Graphic，同时查看 RectTransform。UI 根尺寸缺失时可调整参考尺寸。
- UI 预览使用 XY 正交视图；字体优先尝试工程引用，无法加载时回退默认字体，内置 Sprite 使用颜色矩形近似。

- 支持打开或拖放 `.prefab`、`.unity`、`.asset`、`.mat`、`.controller`、`.overrideController`、`.anim`、`.spriteatlas`、`.guiskin`、`.yaml` 和 `.yml`
- 按 `GameObject` / `Transform` 关系重建 Prefab 和场景层级，并列出组件；对象名称中的 Unicode 转义显示为实际文字
- 搜索对象名称、组件、字段值、`fileID` 或 GUID，并高亮匹配项
- 多标签页浏览，并记住上一次打开的文件夹
- 自动保存未关闭的文件标签与当前标签，下次启动恢复；手动关闭的标签和已删除的文件不会恢复
- 复制对象 `fileID` 及资源引用的 GUID
- 自动识别 UTF-8；遇到无效 UTF-8 字节时尝试按 GBK 读取
- 为 `AnimationClip`、`AnimatorController`、`AnimatorOverrideController`、`Material` 和 `SpriteAtlas` 提供结构化导航
- 按路径 / 组件 / 属性查看动画曲线，显示时长、采样率、关键帧，并定位回原始 YAML
- 动画时间轴支持缩放、多选拖动、刻度右键按曲线插值加帧及右键删除；属性 / 曲线双页支持分量配色、切线手柄、切线模式和权重编辑，支持撤销与保存
- Animator Controller 提供只读状态关系图：Layer 切换、默认状态、Entry / Any State / Exit、过渡连线、子状态机、画布拖动缩放和参数默认值；点击状态查看 Motion 引用，点击连线查看过渡条件，支持切回 YAML
- 将规则 YAML 列表显示为表格，并支持冻结表头
- Transform / RectTransform 支持属性与 YAML 切换，可在编辑模式中修改已识别属性
- MonoBehaviour 根据 `m_Script.guid` 在 Unity 工程中定位脚本，按字段声明显示属性（整数、浮点数、布尔值、字符串、颜色、向量和简单枚举），支持编辑已识别字段，并折叠组件公共信息
- 跟随系统、浅色与深色主题

MonoBehaviour 属性视图会在后台扫描 `Assets`、`Packages` 和 `Library/PackageCache` 中的 `.cs.meta`，并将资源 GUID、脚本类型和 Shader 属性索引保存到 `%APPDATA%/UnityYAMLViewer/project_indexes`。下次打开同一项目优先在后台载入上次缓存并立即提供查询，再后台扫描更新；每个项目每次启动自动扫描一次，切换同项目文件复用索引，修改资源后可点击“重新扫描项目索引”（扫描中禁用）。缓存损坏或版本不兼容时重新建立；扫描有异常时不覆盖已有磁盘缓存，更新完成前可能暂时看到旧引用。支持可唯一定位的父类和嵌套类型，以及 `SerializeField`、`FormerlySerializedAs` 等常见声明。该解析器不是 C# 编译器：条件编译区域、泛型或 partial 类、歧义类型及无法读取的脚本不保证识别，未知字段保留原值；已识别的列表和数组通过表格查看与编辑，不支持的复杂结构可展开查看原文。文件脱离 Unity 工程或脚本来自 DLL 时，可能无法确定字段类型。未知类型及原始复杂结构保持只读。

### 属性编辑与设置

- 支持编辑已识别的标量、字符串、布尔值、枚举、向量、颜色、Transform / RectTransform、材质字段、Gradient 关键点和列表 / 数组元素；具体字段须能唯一定位到原文。
- 点击“编辑模式”暂存修改，修改的属性、表格行和层级节点高亮；点击“还原”恢复该属性或整行最近保存的值。
- “放弃编辑”恢复最近保存的值，“保存编辑”写入 UTF-8 文件；两者均退出编辑模式。
- Ctrl+Z 按文件分别撤销。默认保留 200 步，设置范围 50～1000；超过上限淘汰最早记录。保存后退出编辑模式，但保留撤销记录；Ctrl+Z 自动进入编辑模式并恢复之前的值，再次保存才写入磁盘。关闭文件或点击“放弃编辑”清空记录；放弃恢复最近保存的内容。
- 确认输入（Enter 或失去焦点）时验证整数范围、有限浮点数、向量分量和已识别枚举；无效值警告并回退。未知类型、原始 YAML、fileID 引用保持只读。
- 切换标签页保留各文件的暂存修改；处于编辑模式时，关闭标签或退出应用前须先保存或放弃。未保存修改和撤销记录不会跨启动恢复。
- 未确认输入或警告会阻止保存；保存前检查外部改动，通过同目录临时文件替换原文件，写入失败保留原文件。
- 设置、关于、更新、加载进度、编辑确认及颜色 / 渐变面板使用独立系统窗口，可移出主窗口，内容超出窗口时可滚动；设置支持主题、语言、预览、Hierarchy 展开层级、视频解码及撤销容量等选项；最后两项固定为“单个文件缓存撤销步骤”和“检查更新”。

### 使用方法

1. 启动应用。
2. 点击“打开”，或将 Unity YAML 文件拖进窗口。
3. 在左侧层级中选择对象或组件，在右侧 Inspector 查看属性或原始 YAML。
4. 用顶部搜索框查找名称、字段值、`fileID` 或 GUID。

### 命令行及 AI 工具打开文件

通过可执行程序传入一个或多个文件路径即可打开；包含空格的路径需要加引号。支持相对路径和中文路径，最后一个文件成为当前标签。

```powershell
& 'C:\Git\UnityYAMLViewer\target\release\unity_yaml_viewer.exe' 'C:\项目\Assets\示例.prefab'
```

Windows 下已有窗口时，请求会交给现有窗口，打开文件并请求将窗口置于前台；已打开的文件直接切换到对应标签。不带路径调用则激活现有窗口。AI 工具可直接调用同一命令。首次使用这项功能前需要编译新版，并关闭旧版窗口。

### 构建与运行

本项目使用 Rust 2024 edition。先安装稳定版 [Rust](https://www.rust-lang.org/tools/install)，然后运行：

```powershell
cargo run --release
```

Windows 构建产物：

```text
target\release\unity_yaml_viewer.exe
```

构建时会自动将 `assets/app_icon.png` 转换并嵌入为 Windows 应用图标。

### 待支持与当前限制

- GameObject 属性视图显示名称、Active、Layer、Tag，保留 YAML 切换；编辑模式可修改本地节点 Active，名称 / Layer / Tag 和展开的 Prefab / FBX 节点暂只读。父节点失活不批量改写子节点 Active。

- 嵌套 FBX Model Prefab 可展开已验证的单网格节点模型，沿场景 fileID / stripped 引用应用父节点、变换、名称、排序和材质槽覆盖；单一未蒙皮网格可将原始三角形、UV0、颜色与法线传入受限的 GPU Shader 路径。多节点、缺少唯一源映射、蒙皮暂不支持，完整导入器与切线还原仍有限制。

- 纯 2D 场景提供有限 Built-in 单 Pass Sprite / 网格 Shader GPU 路径，运行时读取Assets、Packages 和 Library/PackageCache 内引用源码及相对 / 项目路径 Include（解析后的真实路径须位于这些资源目录；Packages 虚拟引用可匹配缓存包，同名多版本无法唯一确定时报错），绑定多纹理、标量、颜色 / 向量、世界矩阵与时间；Blend Off 或未声明时采用 One/Zero 不透明默认值，支持材质参数及 Shader 属性默认值；支持不依赖实例化条件分支的 multi_compile_instancing Shader 按独立绘制执行及 rsqrt，主纹理 TexelSize 按实际绑定纹理提供，片元 clip 保留负分量丢弃语义；TEXCOORD1/2 的标量及 2～4 分量顶点输入暂填零，未还原粒子自定义顶点流；兼容简单外层 Include Guard、2×2 / 3×3 矩阵类型及函数内 uniform 局部副本修改，Cull / ZWrite / ZTest 参数解析后仍限于当前支持组合（Cull Off/Back、ZWrite Off，完整深度未支持；Stencil 遮罩忽略并提示，其他兼容条件满足时继续执行 Shader）；场景纹理总预算为 2 GiB（CPU 缓存与 GPU 源纹理分别计量，并非进程总内存上限）。预览无雾；未使用的标量声明不要求赋值，未知已引用标量默认 1 并提示，无法读取 Unity 脚本运行时 Global 值。昼夜全局颜色默认白色（RGBA 均为 1）；支持含 UTF-8 BOM 的 Shader Include。预览左上角悬浮显示提交绘制帧率 FPS，滚轮以鼠标位置为中心缩放。不支持的 Shader 报告诊断并回退普通贴图或烘焙图。CG 到 GLSL 仅支持部分语法与 Unity 函数，不保证与 Unity 完全一致；混合 UI 场景、URP / LWRP、ShaderGraph、完整光照、粒子模拟及场景整体动画暂不支持此路径。
- Spine 4.1 / 4.2、完整 SkeletonGraphic 缩放与特殊混合、Unity Mask / RectMask2D 裁剪。
- 完整 UI 动态布局、平铺 / 径向填充、复杂嵌套 Prefab 数组覆盖及 AtlasCache 打包图预览。
- 场景内 Unity 文本 Mesh.asset 支持未压缩、单 stream 的 Float32 / UNorm8 通道、UV0、法线、切线、颜色、16 / 32 位三角形索引及 fileID 选择；静态合成按子网格使用材质，GPU 自定义 Shader 按材质槽匹配子网格，多余材质按顺序叠加最后一个子网格（最多 64 槽）；不可用材质槽跳过并提示，材质少于子网格时未匹配部分不绘制；CPU 回退与 3D 预览尚未补齐多余材质叠加。二进制 / 压缩 / 外部分流 Mesh、多 stream、UV1 及更高 UV、FBX 子网格 fileID 精确选择、Blend Shape 与 3D 点选联动仍有限制或未支持。
- 字幕渲染、HDR 输出与精确声卡时钟同步；音视频格式和硬解能力依赖外部 FFmpeg 与设备。
- 非整数倍率缩放仍有插值，高 DPI 下 100% 沿用逻辑尺寸；大文件完整读入内存，尚未采用磁盘分页。

### 支持范围与限制

- 工具仅面向 Unity 的**文本序列化** YAML 资源，无法读取二进制序列化资源。
- 解析逻辑针对常见 Unity YAML 结构；不保证覆盖全部 Unity 版本、内置类型或自定义序列化格式。
- 默认只读；通过“编辑模式”修改已识别属性。带 fileID 的引用保持锁定，点击会提示风险。

### 技术栈

- [Rust](https://www.rust-lang.org/)
- [eframe / egui](https://github.com/emilk/egui)
- [rfd](https://github.com/PolyMeilex/rfd)
- [regex](https://github.com/rust-lang/regex)
- [encoding_rs](https://github.com/hsivonen/encoding_rs)

### 贡献

欢迎提交 Issue 或 Pull Request。提交前请确认：

```powershell
cargo check
```

---

## English

**Unity YAML Viewer** is a desktop tool for Unity text-serialized assets. It parses YAML files such as `.prefab`, `.unity`, and `.asset` into a more approachable hierarchy and Inspector view, making it easier to inspect assets without opening the Unity Editor.

The project now supports **property editing**: change recognized values through Inspector controls and save them back to the original file. Type validation, reference protection, and targeted text updates help reduce indentation, reference, and formatting errors from manual YAML editing.

### Supported features

- Static 2D / 3D scene and Prefab previews with Sprites, Tilemaps, common UI, models and Spine poses; support uncompressed text triangle Meshes referenced by local fileID (line topology is unsupported); nested Prefab expansion and common overrides, with per-level reference indexing and shared source templates (512 MiB each for templates and expanded text, up to 500,000 documents per level); picking a nested Prefab in the scene while expansion is enabled reveals, scrolls to and highlights its Hierarchy node; referenced nodes are read-only; expansion diagnostics can be copied with one click.
- GPU scene previews complete their first render, then freeze when the pointer leaves the canvas or the app loses focus; returning resumes updates. Frozen images retain their aspect ratio on resize.
- Standalone Spine 3.8 previews with animation, skins, multiple tracks, playback speed, timeline, zoom and pan.
- FBX / OBJ previews with textures, colors, solid / wireframe display, mesh statistics, static skinning and camera controls.
- Image channels, zoom and pan; script / text viewing, basic syntax highlighting and project font previews.
- Audio / video playback with pause, stop, seeking, volume and Space controls; video audio-track selection, paused arrow-key frame stepping and automatic / software / hardware decoding options.
- Fit, native-size and free-scale video modes, plus fullscreen via F11 and exit via F11 / Esc. Floating fullscreen controls hide after 1 second without mouse movement. Free scale supports pointer-centered zoom, dragging and click playback controls.
- Audio waveforms, approximate source dBFS levels and white peak trails falling at 5 dB per second.
- External FFmpeg installation / detection in Settings; failed media automatically retries after closing Settings once the decoder is ready.
- Tab reordering and session restoration, configurable recent file history (10–50 entries, default 30, with a clear button in Settings), background loading / resource indexing and configurable preview / hierarchy expansion.
- Simplified Chinese, Traditional Chinese, English, Japanese, Korean, German and French; system, light and dark themes.
- GitHub Releases updates (automatically follows Windows manual system proxies and proxy environment variables; PAC / automatic proxy discovery is unsupported) with automatic / manual checks (results appear in Settings; failed checks provide a manual update link, and new releases use the same dialog as the New badge), release notes, download progress, size / SHA-256 / architecture verification, unsaved-edit protection and backup / replacement / restart.

- Static Prefab Sprite preview with hierarchical transforms, source texture slicing, tint and flips; resize the preview pane, zoom and pan. Selecting a SpriteRenderer also displays its owning Transform/RectTransform for inspection and editing.
- UI Prefab previews support Simple/Sliced and linear Filled Images, RawImage and static Text/TextMeshPro, with RectTransform anchors, pivots, size and hierarchical transforms. Clicking selects the Graphic with its RectTransform shown alongside it. Reference dimensions can be adjusted when the root size is missing.
- UI previews use an XY orthographic view, project fonts where available with fallback fonts, and colored rectangles for built-in UI Sprites.

- Open or drag and drop `.prefab`, `.unity`, `.asset`, `.mat`, `.controller`, `.overrideController`, `.anim`, `.spriteatlas`, `.guiskin`, `.yaml`, and `.yml` files
- Rebuild Prefab and scene hierarchies from `GameObject` / `Transform` relationships and list components
- Search object names, components, field values, `fileID`s, or GUIDs, with highlighted matches
- Browse several files in tabs and remember the last opened directory
- Restore open tabs and the active tab on startup; closed tabs and deleted files are omitted
- Copy object `fileID`s and referenced GUIDs
- Detect UTF-8 automatically and fall back to GBK when invalid UTF-8 bytes are found
- Structured navigation for `AnimationClip`, `AnimatorController`, `AnimatorOverrideController`, `Material`, and `SpriteAtlas`
- Browse animation curves by path, component, and property; inspect duration, sample rate, and keyframes; jump back to the original YAML
- Read-only Animator Controller graphs with layers, default states, Entry / Any State / Exit, transitions, nested state machines, pan/zoom, parameter types and defaults, Motion references, and transition conditions; switch back to YAML at any time
- Display regular YAML lists as tables, with an optional frozen header
- Switch between properties and YAML for Transform / RectTransform; edit recognized properties in Edit Mode
- Resolve MonoBehaviour scripts through `m_Script.guid` and display declared field types with editable controls for recognized numbers, booleans, strings, colors, vectors, and simple enums; collapse common component metadata
- System, light, and dark themes

The MonoBehaviour inspector indexes `.cs.meta` files in `Assets`, `Packages`, and `Library/PackageCache` in the background. Resource GUIDs, script types and shader properties are persisted under `%APPDATA%/UnityYAMLViewer/project_indexes`. On the next launch, the saved cache is loaded in the background and served first while a fresh scan runs. Each project is scanned automatically once per launch; opening another file in that project reuses the index. Use “Rescan project index” after resource changes (disabled during scanning). Corrupt or incompatible caches are rebuilt; scans with issues preserve the existing disk cache. References may temporarily be stale until refresh completes. Unambiguous inheritance, nested types, and common attributes such as `SerializeField` and `FormerlySerializedAs` are supported. This source parser is not a C# compiler: conditional sections, generic or partial classes, ambiguous types, and unavailable scripts may remain unresolved. Unknown values are preserved. Recognized lists and arrays can be viewed and edited in tables; unsupported complex structures can be expanded as raw text. Assets outside a Unity project and scripts from DLLs may have unknown field types. Unknown types and unsupported raw structures remain read-only.

### Property editing and settings

- Editable fields include recognized scalars, strings, booleans, enums, vectors, colors, Transform / RectTransform properties, material fields, Gradient keys, and list / array elements. Each field must map unambiguously to its source text. Modified rows are highlighted; Restore reverts one property or table row independently.
- Discard Edits returns to the latest saved values. Save Edits writes UTF-8 and exits Edit Mode.
- Ctrl+Z keeps separate histories per file. Default: 200 steps; configurable range: 50–1000. The oldest entries expire beyond the limit. Saving exits Edit Mode and retains undo history. Ctrl+Z re-enters Edit Mode and restores a previous value; save again to write it to disk. Closing the file or choosing Discard Edits clears its history. Discard restores the latest saved contents.
- Committed input validates numeric widths, finite floats, vector components and recognized enums. Invalid input warns and reverts. Unknown types, raw YAML and fileID references remain locked.
- Switching tabs preserves staged edits for each file. Save or discard before closing an editing tab or exiting the application. Unsaved edits and undo histories are not restored after restarting. and replaces the asset using a temporary file in the same directory. Failed writes preserve the original.
- Settings, About, update dialogs, loading progress, edit confirmations and color / gradient panels use separate native windows with scrolling when needed. Settings includes theme, language, preview, hierarchy expansion, video decoder and undo options. Undo history capacity and Check for Updates remain the final two items.

### Usage

1. Start the application.
2. Click **Open**, or drag a Unity YAML file into the window.
3. Select an object or component in the hierarchy to view its properties or raw YAML in the Inspector.
4. Use the search field to find names, field values, `fileID`s, or GUIDs.

### Build and run

This project uses Rust 2024 edition. Install stable [Rust](https://www.rust-lang.org/tools/install), then run:

```powershell
cargo run --release
```

The Windows executable is created at:

```text
target\release\unity_yaml_viewer.exe
```

During Windows builds, `assets/app_icon.png` is converted and embedded as the application icon.

### Pending support and current limitations

- GameObject properties show Name, Active, Layer and Tag with a YAML switch. Edit mode can change Active on locally serialized objects; Name/Layer/Tag and expanded Prefab/FBX objects remain read-only. Disabling a parent does not rewrite its children's Active flags.

- Nested FBX Model Prefabs can expand verified single-mesh-node models, using scene fileID/stripped references for parenting, transforms, names, sorting and material-slot overrides. A single unskinned mesh can pass raw triangles, UV0, colors and normals to the limited GPU shader path. Multiple nodes, missing unique source mappings, skinning are unsupported; full importer and tangent fidelity remain limited.

- Pure 2D scenes have a limited GPU path for single-pass Built-in Sprite/mesh shaders. Referenced source in Assets, Packages and Library/PackageCache and relative/project includes are read at runtime (resolved paths must remain within these resource directories; virtual Packages references can resolve cached packages, while ambiguous versions report errors), with multiple textures, scalar/color/vector properties, world matrices and time bound to the program. Blend Off or absent Blend uses the opaque One/Zero default. Shaders declaring multi_compile_instancing can use individual draws when their code does not require instancing branches; rsqrt and bound main-texture TexelSize are supported; fragment clip discards fragments with negative components. Simple outer include guards, 2×2/3×3 matrix types and local copies for function-level uniform writes are supported. Blend supports material properties and shader property defaults; parameterized Cull/ZWrite/ZTest remain limited to supported combinations (Cull Off/Back and ZWrite Off; full depth is unsupported; Stencil masks are ignored with diagnostics, while otherwise compatible shaders continue). Scene textures have a 2 GiB budget (CPU cache and GPU source textures are counted separately; this is not a process memory limit). The preview has no fog; unused scalar declarations need no binding, while unresolved referenced scalars default to 1 with diagnostics. Unity script runtime globals cannot be read from these files. Global day/night colors default to white (RGBA all 1); UTF-8 BOMs in shader includes are supported. A floating FPS overlay shows submitted preview frames, and wheel zoom is centered on the pointer. Unsupported shaders report diagnostics and fall back to ordinary textures or baked images. CG-to-GLSL syntax and Unity function compatibility are partial; full fidelity is not guaranteed. Mixed UI scenes, URP/LWRP, ShaderGraph, full lighting, particle simulation and scene-wide animation are outside this path.
- Spine 4.1 / 4.2, complete SkeletonGraphic scaling / special blending and Unity Mask / RectMask2D clipping.
- Complete driven UI layouts, tiled / radial fills, complex nested Prefab array overrides and AtlasCache packed-image previews.
- Scene-referenced text Mesh assets support uncompressed single-stream Float32/UNorm8 channels, UV0, normals, tangents, colors, 16/32-bit triangle indices and fileID selection. Static composition maps submeshes to materials; custom GPU shaders match material slots to submeshes and draw extra slots on the last submesh in order (up to 64 slots). Unavailable slots are skipped with diagnostics; unmatched submeshes are omitted when there are fewer material slots. Extra-material layering in CPU fallback and 3D preview is not yet implemented. Binary/compressed/external-stream Mesh data, multiple streams, UV1 and higher, precise FBX submesh fileID selection, Blend Shapes and linked 3D picking remain limited or unsupported.
- Subtitle rendering, HDR output and precise audio-device clock synchronization. Media formats and hardware decoding depend on external FFmpeg and the device.
- Non-integer scaling still interpolates; 100% uses logical sizing at high DPI. Large files remain fully loaded into memory without disk paging.

### Scope and limitations

- Quoted YAML strings are decoded for property and table display, including Unicode escapes such as `\u7070` → `灰`. Raw YAML remains unchanged.
- Edits keep decoded display values separate from serialized text, save as UTF-8, preserve quoting and Unicode escape style where possible, and escape quotes, backslashes, and control characters. Unchanged text, line endings, and existing UTF-8 BOMs are preserved.

- The viewer targets **text-serialized** Unity YAML assets only; it cannot read binary-serialized assets.
- Parsing targets common Unity YAML layouts and does not guarantee coverage of every Unity version, built-in type, or custom serialization format.
- Recognized properties are editable in Edit Mode. fileID references, raw YAML, unknown types, and fields without an unambiguous source location remain read-only.

### Technology

- [Rust](https://www.rust-lang.org/)
- [eframe / egui](https://github.com/emilk/egui)
- [rfd](https://github.com/PolyMeilex/rfd)
- [regex](https://github.com/rust-lang/regex)
- [encoding_rs](https://github.com/hsivonen/encoding_rs)

### Contributing

Issues and pull requests are welcome. Before submitting, please run:

```powershell
cargo check
```

## License

This project is licensed under the [MIT License](LICENSE).

GPU preview accepts scalar and two- to four-component TEXCOORD1/2 vertex inputs with zero values as a temporary fallback. Particle custom vertex streams and their actual data are not yet reproduced.

Quoted Unicode escapes in object names are decoded for display and name searches without rewriting source YAML.

支持单蒙皮网格嵌套 FBX 的静态导入姿态（需唯一根 Transform / Renderer 源 ID），应用根变换、材质与排序；内部骨骼节点、单骨骼覆盖和动画暂未还原。

Nested FBX models with one skinned mesh can display their static imported pose when root Transform and Renderer source IDs are unambiguous. Root transforms, materials and sorting apply; internal bone nodes, per-bone overrides and animations are not reproduced.
