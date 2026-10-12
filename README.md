# Unity YAML Viewer

[中文](#中文) | [English](#english)

---

## 中文

**Unity YAML Viewer** 是一个面向 Unity 文本序列化资源的桌面工具。它将 `.prefab`、`.unity`、`.asset` 等 YAML 文件解析为更易理解的层级和 Inspector 视图，帮助你在不打开 Unity 编辑器时检查资源内容。

项目现已支持**属性编辑**：通过 Inspector 控件修改已识别的值，确认保存后写回原文件。类型校验、引用保护和精确片段写回帮助减少手动编辑 YAML 时的缩进、引用或格式损坏问题。

### 当前状态

当前版本 **1.1.7** 默认只读，支持显式进入编辑模式，在属性控件和列表表格中修改已识别类型的值；原始 YAML 保持只读。只有点击“保存编辑”才会写入文件。

### 1.1.7 更新

新增视频全屏与悬浮控制、空格播放 / 暂停、音频白色峰值拖尾，并改善 FFmpeg 安装后识别及媒体页自动刷新。统一 Shader 渲染系统和粒子渲染仍未实现。详见 [1.1.7 功能记录](docs/versions/1.1.7.md) 与 [发布说明](docs/releases/1.1.7.md)；准备发布中，尚未上传。

### 1.1.5 更新（开发中，待测试）

Spine 3.8 独立骨骼渲染与多轨动画预览、场景 / Prefab 静态 Spine 合成，以及左下轨道列表和底部中央缩放 / 播放悬浮控件。完整限制与验证状态见 [1.1.5 功能记录](docs/versions/1.1.5.md)。已生成 1.1.5 Release 开发构建，实际运行效果待测试，尚未正式发布。

### 1.1.4 更新（开发中）

新增组件折叠 Inspector、脚本语法高亮、图片通道与缩放查看、工程字体预览、最近文件菜单及 SVG 图标。完整更新日志草稿见 [CHANGELOG.md](CHANGELOG.md)，正式版尚未发布。

### 1.1.3 更新

- 支持 Prefab 预览，目前仅支持 2D，Spine 和 3D 对象暂不支持。

### 1.1.2 更新

- 新增 GitHub Releases 在线更新；设置底部提供“启动时自动检查更新”（默认开启）与“检查更新”按钮。
- 检测到新版本时，设置按钮前显示红色 New；点击查看版本与发行说明。确认已是最新版后清除 New，手动检查会提示。
- 后台下载并显示进度，必须通过附件大小、SHA-256 和 Windows 程序架构校验；失败不会替换现有程序。
- 下载完成后提醒保存；未保存修改会阻止更新。二次确认后退出、备份旧程序、替换并重启，替换失败尝试恢复备份。

更新源为本仓库最新正式 Release，标签采用 `v1.1.2` 等语义版本，附件采用 `UnityYAMLViewer-v1.1.2-windows-x64.exe`（对应架构也支持 x86 / arm64）或 `unity_yaml_viewer.exe`，并须由 GitHub 提供 SHA-256 digest。支持包含程序的同名 Windows ZIP；GitHub 自动生成的源码包不用于更新。当前更新器只替换单个可执行文件，不安装其他依赖或资源。

### 1.1.1 更新

- 更新深浅主题配色，优化界面与设置体验。
- 新增简中、繁中、英语、日语、韩语、德语、法语，支持即时切换并记住选择。
- 优化编辑模式视觉提示和更新历史展示。
- 新增 AnimationClip 时间轴编辑功能。

### 已有功能

- Prefab 静态 Sprite 组合预览：按层级 Transform 叠加位置、缩放和旋转，支持源图切片、颜色和翻转；可拖动调整预览宽度、缩放及平移。选中 SpriteRenderer 时同时查看和编辑所属节点的 Transform/RectTransform。
- UI Prefab 预览支持 Image 普通/九宫格及横向/纵向填充、RawImage、Text/TextMeshPro 静态文字；按 RectTransform 锚点、pivot、尺寸及层级变换计算布局，点击选中对应 Graphic，同时查看 RectTransform。UI 根尺寸缺失时可调整参考尺寸。
- 首批预览为 XY 正交视图；UI 文字使用系统字体近似，内置 Sprite 使用颜色矩形近似。嵌套 Prefab、平铺/径向填充、动态布局、自定义材质及遮罩尚未完整支持。

- 支持打开或拖放 `.prefab`、`.unity`、`.asset`、`.mat`、`.controller`、`.overrideController`、`.anim`、`.spriteatlas`、`.guiskin`、`.yaml` 和 `.yml`
- 按 `GameObject` / `Transform` 关系重建 Prefab 和场景层级，并列出组件
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

MonoBehaviour 属性视图会在后台扫描 `Assets`、`Packages` 和 `Library/PackageCache` 中的 `.cs.meta`，并缓存 GUID 索引；修改脚本后可点击“重新扫描脚本”。支持可唯一定位的父类和嵌套类型，以及 `SerializeField`、`FormerlySerializedAs` 等常见声明。该解析器不是 C# 编译器：条件编译区域、泛型或 partial 类、歧义类型及无法读取的脚本不保证识别，未知字段保留原值；已识别的列表和数组通过表格查看与编辑，不支持的复杂结构可展开查看原文。文件脱离 Unity 工程或脚本来自 DLL 时，可能无法确定字段类型。未知类型及原始复杂结构保持只读。

### 属性编辑与设置

- 支持编辑已识别的标量、字符串、布尔值、枚举、向量、颜色、Transform / RectTransform、材质字段、Gradient 关键点和列表 / 数组元素；具体字段须能唯一定位到原文。
- 点击“编辑模式”暂存修改，修改的属性、表格行和层级节点高亮；点击“还原”恢复该属性或整行最近保存的值。
- “放弃编辑”恢复最近保存的值，“保存编辑”写入 UTF-8 文件；两者均退出编辑模式。
- Ctrl+Z 按文件分别撤销。默认保留 200 步，设置范围 50～1000；超过上限淘汰最早记录。保存后退出编辑模式，但保留撤销记录；Ctrl+Z 自动进入编辑模式并恢复之前的值，再次保存才写入磁盘。关闭文件或点击“放弃编辑”清空记录；放弃恢复最近保存的内容。
- 确认输入（Enter 或失去焦点）时验证整数范围、有限浮点数、向量分量和已识别枚举；无效值警告并回退。未知类型、原始 YAML、fileID 引用保持只读。
- 切换标签页保留各文件的暂存修改；处于编辑模式时，关闭标签或退出应用前须先保存或放弃。未保存修改和撤销记录不会跨启动恢复。
- 未确认输入或警告会阻止保存；保存前检查外部改动，通过同目录临时文件替换原文件，写入失败保留原文件。
- “设置”弹窗依次包含“关于”按钮、主题下拉菜单和单个文件缓存撤销步骤；主题与步数在下次启动时恢复。

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

### Current status

Version **1.1.7** starts in read-only mode and provides explicit property and table editing for recognized types. Raw YAML stays read-only; only Save Edits writes to disk.

### 1.1.7 update

Adds fullscreen video with floating controls, Space playback controls, audio peak trails, and improved FFmpeg detection and media retry after installation. The unified Shader rendering system and particle rendering remain unimplemented. See the [1.1.7 feature record](docs/versions/1.1.7.md) and [release notes](docs/releases/1.1.7.md). Release preparation is in progress; no upload has been made.

### 1.1.5 release notes (in development, awaiting testing)

Spine 3.8 standalone rendering and multi-track animation preview, static Spine composition in scenes / prefabs, and floating track and zoom / playback controls. See the [1.1.5 feature record](docs/versions/1.1.5.md) for limitations and validation status. A 1.1.5 Release development build is available; runtime validation and official release remain pending.

### 1.1.4 release notes (in development)

Adds collapsible component inspectors, script syntax highlighting, image channels and zoom, project font previews, recent files, and SVG icons. See [CHANGELOG.md](CHANGELOG.md) for the draft release notes. The final release has not been published.

### 1.1.3 release notes

- Added Prefab preview. Currently supports 2D only; Spine and 3D objects are not supported yet.

### 1.1.2 release notes

- Added GitHub Releases updates, enabled startup checks, a manual check button and a red New badge.
- View release notes and download progress. Downloads require size, SHA-256 and executable architecture validation.
- Save pending edits before confirming replacement and restart. The updater backs up the old executable and attempts rollback on failure.
- Publish a stable semantic version tag and a matching `UnityYAMLViewer-v1.1.2-windows-x64.exe` asset with GitHub's SHA-256 digest. The updater replaces only the executable; source archives are not update packages.

### 1.1.1 release notes

- Refreshed light and dark themes, with improved interface and settings.
- Added Simplified Chinese, Traditional Chinese, English, Japanese, Korean, German and French, with instant switching and saved preferences.
- Improved visual cues in edit mode and release history display.
- Added AnimationClip timeline editing.

### Available features

- Static Prefab Sprite preview with hierarchical transforms, source texture slicing, tint and flips; resize the preview pane, zoom and pan. Selecting a SpriteRenderer also displays its owning Transform/RectTransform for inspection and editing.
- UI Prefab previews support Simple/Sliced and linear Filled Images, RawImage and static Text/TextMeshPro, with RectTransform anchors, pivots, size and hierarchical transforms. Clicking selects the Graphic with its RectTransform shown alongside it. Reference dimensions can be adjusted when the root size is missing.
- The preview uses an XY orthographic view, system font approximations and colored rectangles for built-in UI Sprites. Nested Prefabs, tiling/radial fills, driven layouts, custom materials and masks are not fully supported.

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

The MonoBehaviour inspector indexes `.cs.meta` files in `Assets`, `Packages`, and `Library/PackageCache` in the background. Use the rescan button after changing scripts. Unambiguous inheritance, nested types, and common attributes such as `SerializeField` and `FormerlySerializedAs` are supported. This source parser is not a C# compiler: conditional sections, generic or partial classes, ambiguous types, and unavailable scripts may remain unresolved. Unknown values are preserved. Recognized lists and arrays can be viewed and edited in tables; unsupported complex structures can be expanded as raw text. Assets outside a Unity project and scripts from DLLs may have unknown field types. Unknown types and unsupported raw structures remain read-only.

### Property editing and settings

- Editable fields include recognized scalars, strings, booleans, enums, vectors, colors, Transform / RectTransform properties, material fields, Gradient keys, and list / array elements. Each field must map unambiguously to its source text. Modified rows are highlighted; Restore reverts one property or table row independently.
- Discard Edits returns to the latest saved values. Save Edits writes UTF-8 and exits Edit Mode.
- Ctrl+Z keeps separate histories per file. Default: 200 steps; configurable range: 50–1000. The oldest entries expire beyond the limit. Saving exits Edit Mode and retains undo history. Ctrl+Z re-enters Edit Mode and restores a previous value; save again to write it to disk. Closing the file or choosing Discard Edits clears its history. Discard restores the latest saved contents.
- Committed input validates numeric widths, finite floats, vector components and recognized enums. Invalid input warns and reverts. Unknown types, raw YAML and fileID references remain locked.
- Switching tabs preserves staged edits for each file. Save or discard before closing an editing tab or exiting the application. Unsaved edits and undo histories are not restored after restarting. and replaces the asset using a temporary file in the same directory. Failed writes preserve the original.
- Settings contains About, a theme selector and undo history capacity. Theme and undo settings persist across restarts.

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
