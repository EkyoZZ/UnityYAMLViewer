# Unity YAML Viewer 交接：1.1.1 收尾 → 静态图形预览

交接日期：2026-10-10。工程：`C:\Git\UnityYAMLViewer`。

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
