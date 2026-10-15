# Unity YAML Viewer

**不打开 Unity，也能查看资源、编辑属性和预览材质。**

A desktop viewer for Unity assets, with property editing and material previews.

[中文](#中文) · [English](#english) · [下载 Windows x64 / Download](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)

## 中文

### ✨ 已支持

- **资源查看**：打开 Prefab、Scene、Material 等文本资源，显示 Hierarchy 与 Inspector，支持搜索、引用定位和多标签页。
- **属性编辑**：编辑已识别的属性、颜色、向量和列表，支持校验、撤销与保存。
- **材质预览**：Material 和 Shader 使用 **Node-based UI** 预览，在节点内临时调参、拖图替换纹理，切换 2D / 3D 输出；Render State 可临时调整 Cull / Blend，Vert / Frag 与普通函数以可移动、可拉伸的淡色分组区分（包含参与计算的常量），拆为 In → 步骤 → Out，颜色节点支持快照，纯计算不预览，Out 文本自动换行，按依赖横向排列，画布支持 10%～175% 缩放。
- **场景与动画**：预览受支持的 2D / UI、Spine 和模型资源，查看 Animator 状态图、编辑动画曲线。
- **音视频播放**：支持播放列表、波形、全屏和播放进度记忆；解码依赖外部 FFmpeg，可在设置中安装 / 检测。
- **日常使用**：最近文件、标签恢复、深浅主题、七种语言和在线更新。

### 🚀 快速开始

1. [下载 Windows x64 程序](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest)，直接运行 EXE。
2. 将文件拖入窗口，或通过“文件”菜单打开；Unity 资源尽量保留在原项目目录中，以便解析引用。
3. 选择对象查看属性；需要修改时开启“编辑模式”。材质节点中的调整仅用于预览，不写回文件。

详细操作、支持格式和预览范围见[使用指南](docs/user-guide.md#中文)。

### ⚠️ 待支持与当前限制

- Unity 二进制序列化资源暂不支持；未知类型、原始 YAML 和 fileID 引用保持只读。
- 场景、Spine 和 Shader 为部分兼容，不能完整替代 Unity 渲染；URP / HDRP、多 Pass、复杂 Shader 与完整光照仍有限制。
- 节点界面目前用于预览，不支持创建节点、编辑连线或生成 Shader；函数快照限可唯一定位到阶段调用处的数值参数函数，复杂调用只展示拆分。

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
- **Property editing**: Edit recognized properties, colors, vectors and lists, with validation, undo and saving.
- **Material previews**: Preview Materials and Shaders in a **node-based UI**; adjust inputs, drop replacement textures and switch between 2D / 3D output, with temporary Cull / Blend controls in Render State and draggable, resizable tinted scopes including supporting constants with separate In → step → Out nodes for stages and helpers. Color nodes have snapshots; calculations have no preview. Out text wraps within its node; dependencies flow horizontally, with 10%–175% canvas zoom.
- **Scenes and animation**: Preview supported 2D / UI, Spine and model assets, inspect Animator graphs and edit animation curves.
- **Media playback**: Playlists, waveforms, fullscreen and remembered playback positions. Decoding requires external FFmpeg, installable / detectable in Settings.
- **Everyday tools**: Recent files, restored tabs, light / dark themes, seven languages and online updates.

### 🚀 Quick start

1. [Download the Windows x64 executable](https://github.com/EkyoZZ/UnityYAMLViewer/releases/latest) and run it directly.
2. Drop a file into the window or open it from the File menu. Keep Unity assets inside their original project where possible so references can be resolved.
3. Select an object to inspect it; enable Edit Mode to change properties. Material-node adjustments affect the preview only and do not write files.

See the [user guide](docs/user-guide.md#english) for detailed controls, formats and preview support.

### ⚠️ Pending support and limitations

- Unity binary serialization is unsupported. Unknown types, raw YAML and fileID references remain read-only.
- Scene, Spine and Shader compatibility is partial and does not reproduce Unity rendering completely. URP / HDRP, multiple passes, complex shaders and full lighting remain limited.
- The node interface is for previews; creating nodes, editing connections and generating shaders are unsupported. Helper snapshots require a unique direct stage call with numeric parameters; complex calls are display-only.

### 🛠️ Build from source

Install stable [Rust](https://www.rust-lang.org/tools/install), then run from the project directory:

```powershell
cargo run --release
```

Windows output: `target/release/unity_yaml_viewer.exe`. Built with Rust and egui / eframe.
Issues and PRs are welcome; run `cargo check` before submitting.

## License

[MIT](LICENSE)
