# Unity YAML Viewer

[中文](#中文) | [English](#english)

---

## 中文

**Unity YAML Viewer** 是一个面向 Unity 文本序列化资源的桌面工具。它将 `.prefab`、`.unity`、`.asset` 等 YAML 文件解析为更易理解的层级和 Inspector 视图，帮助你在不打开 Unity 编辑器时检查资源内容。

项目的目标不止于“阅读 YAML”：它希望逐步提供**可安全编辑的字段面板**，让用户直接修改可识别的值，同时通过结构化写回、校验和预览来避免手动编辑 YAML 时常见的缩进、引用或格式损坏问题。

### 当前状态

当前版本是**只读浏览器**：不会写回、保存或修改任何打开的资源文件。安全编辑面板属于后续开发方向，尚未实现。

### 已有功能

- 支持打开或拖放 `.prefab`、`.unity`、`.asset`、`.mat`、`.controller`、`.overrideController`、`.anim`、`.spriteatlas`、`.guiskin`、`.yaml` 和 `.yml`
- 按 `GameObject` / `Transform` 关系重建 Prefab 和场景层级，并列出组件
- 搜索对象名称、组件、字段值、`fileID` 或 GUID，并高亮匹配项
- 多标签页浏览，并记住上一次打开的文件夹
- 复制对象 `fileID` 及资源引用的 GUID
- 自动识别 UTF-8；遇到无效 UTF-8 字节时尝试按 GBK 读取
- 为 `AnimationClip`、`AnimatorController`、`AnimatorOverrideController`、`Material` 和 `SpriteAtlas` 提供结构化导航
- 按路径 / 组件 / 属性查看动画曲线，显示时长、采样率、关键帧，并定位回原始 YAML
- 将规则 YAML 列表显示为表格，并支持冻结表头
- 跟随系统、浅色与深色主题

### 计划中的安全编辑

未来的编辑功能会优先围绕“避免破坏 YAML”设计，而不是把原始文本编辑器直接搬进应用：

- 为可识别的标量、列表和常见 Unity 字段提供表单式编辑
- 保留未知字段、注释和对象引用，尽量缩小写入范围
- 在保存前显示变更预览，并检查 YAML 结构、`fileID` / GUID 引用和类型格式
- 仅在校验通过后写入；失败时保留原文件，并提供清晰的错误信息
- 提供显式保存与备份策略，避免无意覆盖资源

这些是项目方向，不代表当前版本已经具备上述能力。

### 使用方法

1. 启动应用。
2. 点击“打开”，或将 Unity YAML 文件拖进窗口。
3. 在左侧层级中选择对象或组件，在右侧只读 Inspector 查看对应 YAML。
4. 用顶部搜索框查找名称、字段值、`fileID` 或 GUID。

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
- 当前内容均为只读展示。修改资源请使用 Unity 编辑器或其他适合版本控制的文本编辑器。

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

The project is intended to grow beyond YAML viewing. Its longer-term goal is a **safe, value-editing Inspector**: users should be able to edit recognized values through controls while structured writes, validation, and previews help prevent the indentation, reference, and formatting damage that can result from hand-editing YAML.

### Current status

The current release is a **read-only viewer**. It never writes, saves, or modifies an opened asset. The safe editing Inspector is planned work and is not available yet.

### Available features

- Open or drag and drop `.prefab`, `.unity`, `.asset`, `.mat`, `.controller`, `.overrideController`, `.anim`, `.spriteatlas`, `.guiskin`, `.yaml`, and `.yml` files
- Rebuild Prefab and scene hierarchies from `GameObject` / `Transform` relationships and list components
- Search object names, components, field values, `fileID`s, or GUIDs, with highlighted matches
- Browse several files in tabs and remember the last opened directory
- Copy object `fileID`s and referenced GUIDs
- Detect UTF-8 automatically and fall back to GBK when invalid UTF-8 bytes are found
- Structured navigation for `AnimationClip`, `AnimatorController`, `AnimatorOverrideController`, `Material`, and `SpriteAtlas`
- Browse animation curves by path, component, and property; inspect duration, sample rate, and keyframes; jump back to the original YAML
- Display regular YAML lists as tables, with an optional frozen header
- System, light, and dark themes

### Planned safe editing

Editing will be designed around preserving asset integrity rather than exposing a raw text editor:

- Form controls for recognized scalar values, lists, and common Unity fields
- Preserve unknown fields, comments, and object references while limiting the write scope
- Show a change preview and validate YAML structure, `fileID` / GUID references, and value formats before saving
- Write only after validation succeeds; keep the original file intact and report actionable errors on failure
- Require an explicit save and provide a backup strategy to prevent accidental overwrites

These items describe the project direction, not functionality already present in this release.

### Usage

1. Start the application.
2. Click **Open**, or drag a Unity YAML file into the window.
3. Select an object or component in the hierarchy to view its YAML in the read-only Inspector.
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

- The viewer targets **text-serialized** Unity YAML assets only; it cannot read binary-serialized assets.
- Parsing targets common Unity YAML layouts and does not guarantee coverage of every Unity version, built-in type, or custom serialization format.
- All displayed content is currently read-only. Use the Unity Editor or a version-control-friendly text editor to modify assets.

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
