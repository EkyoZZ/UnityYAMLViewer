# 统一渲染系统

## 依据与目标

本分支从 1.1.7 开始。1.1.5 的材质近似实验已删除，1.1.6 记录的下一阶段方向为统一渲染系统，之后接入 Shader 与粒子；此前没有确认具体架构。本文是本分支的实施方案，不代表已完成全部能力。

目标是让场景、模型、Spine 和后续粒子共用明确的渲染输入、资源生命周期与混合规则，逐步替换各自的 CPU 合成代码。不得通过 Shader 名称或有限表达式猜测 Unity 材质效果。

## 当前接入状态

- 最新 MeshRenderer 路径支持受限的运行时源码 / Include 转换、多纹理与材质值绑定，昼夜全局颜色默认白色。海水及焦散引用源码均有转换测试；几何使用内置 Quad 或未压缩单 stream 原生文本 Mesh，保留网格 UV 和世界矩阵。
- 原生 Mesh 支持 Float32 / UNorm8、UV0、法线、切线、顶点颜色、16 / 32 位三角形索引及 fileID；多子网格静态合成可按材质槽显示，但 GPU 自定义 Shader 仅单子网格。压缩 / 二进制 / 外部分流、多 stream、UV1+ 和其他格式待实现。
- 材质额外纹理合计 64 MiB，Repeat / Clamp，昼夜颜色白色；全局 C# 状态不执行，RGB 法线解包仅适用原始 RGB 法线图，完整深度、导入器配置、URP 及任意 HLSL 仍不支持。12 项测试通过，海水实际 GL 编译、效果、绕序、动画和拖动待运行验证。
- Shader / Include 改动需重载文档；缓存尚无自动文件监视和完整变体键。下方简单 Sprite 限制为早期实现记录，以本节与当前版本记录为准。

- 后续针对“效果未还原”排查，将额外 GPU 原始贴图预算从 128 MiB 对齐至 256 MiB；原 Prefab 全部 Sprite PNG 引用约 129.48 MiB，仅用于静态资源估算，不是运行时有效贴图实测。新增 GPU / 普通贴图路径状态并明确选择 Glow；样例转换 3 项测试已运行通过，实际 GPU 效果仍待验证。下文 128 MiB 属于此前实现记录。

- 纯世界 2D 预览已接入 GPU 场景准备与 egui Glow 绘制回调；单 Pass Sprite Built-in Shader 的受限 CG 语法转换器保留源码表达式并生成 GLSL，GPU 实际编译和执行生成的程序。尚未启动应用，因此这里的“接入”表示代码实现及 Rust 编译检查，不表示实际 GPU / 画面验证通过。
- 首个目标的贴图、UV、原始颜色、材质标量与 UV ST 已绑定，混合状态由 Shader 读取；不支持的资源给出诊断并回退普通贴图。GPU 编译错误会保留日志并在后续帧回退 CPU。
- 直接绘制到场景区域当前 framebuffer，背景已经由 egui 绘制，保证目标颜色混合有底图；离屏目标尚未实现。GPU 程序、纹理、几何缓存不每帧重新编译 / 上传，旧资源按当前场景清理并在退出时销毁。
- 仍保留 CPU 合成用于初始准备和命中；混合 UI、复杂 Shader、URP / LWRP 与粒子未接入。约 16 ms 的重绘请求不是实测帧率保证。128 MiB 额外原始纹理预算、固定 Linear / Clamp 采样，以及运行时全局参数默认值均属于当前限制。
- 本轮 Rust 编译检查通过；转换测试仅编译未执行，生成 GLSL 的 GPU 编译、Unity 截图对比、上下文 / 退出释放、不同 DPI 和关于七语言布局待运行验证。

## 早期基础建设记录（以下未接入描述为当时状态）

- 新增 `render_system/gpu.rs`，以 eframe 已有 Glow 接口实现 GLSL 编译 / 链接与错误日志、共享程序和纹理缓存、原始 RGBA8 上传、Float / Vector / Matrix uniform 绑定及显式资源销毁。所有 GL 操作要求所属上下文在调用线程上当前有效，接入时须在渲染回调执行并管理 GL 状态；当前模块尚未被界面调用。
- 当前 GPU 模块仅为资源与程序层，不包含离屏目标、顶点 / 索引绘制或 `Backend::render` 实现；不能称为已完成 GPU 后端。纹理目前固定 Linear / Clamp，不代表还原 Unity TextureImporter 配置；缓存预算与自动淘汰尚未接入。
- `Program` 增加 Built-in / URP / LWRP / Unknown 来源分类。仅定义分类，尚未自动识别或提供任何管线兼容编译器。

- 用户已确认采用 GPU 实时架构，以引用的 Shader 实际执行为目标。新增 `render_system/scene.rs`：定义原始 RGBA 顶点颜色与纹理、材质参数、编译后程序、混合 / 写入状态、组件绘制命令、场景快照、相机和每帧时间 / 全局参数，以及后端接口。接口尚未接入预览，也没有后端实现，不能称为实时渲染已支持。
- 每帧以共享场景快照作为输入，仅更新相机、时间和参数；资源与程序使用独立 revision，供后端缓存失效使用。实际 GPU 缓存及任务调度尚未实现。

- `src/render_system.rs` 集中定义预乘 RGBA 的普通覆盖混合，以及场景 Alpha 命中阈值。
- 世界 2D 合成、Spine 三角形合成与模型线框覆盖接入同一混合函数。保留原有整数取整、Gamma 空间、绘制顺序和命中行为。
- 资源加载、几何光栅化、深度缓存、异步任务与 egui 贴图上传仍在原模块中；本阶段没有统一这些路径，也没有 GPU Shader 执行能力。
- 不新增设置项、不改变关于窗口摘要或 README 的已支持能力；当前仅内部重构。

## 后续实施顺序（待实现）

1. **统一绘制数据**：定义顶点、索引、纹理、材质、绘制命令和相机。Unity YAML / GUID 解析留在资源层，渲染层只接收已解析数据。保留组件 ID，明确绘制顺序与命中策略；不能将模型的局部深度缓存宣称为全场景深度。
2. **GPU 基础后端**：评估现有 eframe 的 Glow 接口与 WGPU 接口，确定一种后端后实现离屏颜色、深度目标和内置无光照纹理管线。先迁移普通 Sprite / Tilemap，再迁移模型和 Spine；保留 CPU 路径用于明确的兼容回退。当前未选择后端，不因默认 UI 后端推断目标架构。
3. **材质与 Shader 管线**：分开读取 ShaderLab 渲染状态、材质参数与实际着色代码；建立编译诊断、变体键和缓存失效。Unity HLSL / CG、Include、宏、Unity 内建变量和 ShaderGraph 不能直接作为 GLSL / WGSL 执行，须确定转换工具链及支持子集。无法执行的材质明确报告能力缺失，不恢复已删除的猜测规则。
4. **场景一致性**：接入全局逐像素深度、透明排序、独立混合状态、色彩空间和资源预算；UI Mask、TMP SDF、光照及 Unity 特定管线逐项声明支持范围。
5. **粒子**：在统一绘制接口稳定后，加入时间推进、随机种子、发射与生命周期，再逐步支持模块、Billboard / Mesh 与排序；Inspector 可读不等于模拟已实现。

## 验证门槛

- 基础重构需检查混合端点、半透明叠加、顶点乘色与命中阈值，并对已有相关测试做编译检查。
- GPU 阶段需实际运行核对不同 DPI、透明边缘、场景排序、模型深度、Spine 接缝、窗口重开及资源释放，编译成功不能替代画面验证。
- Shader 阶段需完整材质、Shader、依赖及 Unity 参考画面，尤其复核此前 `DstColor SrcAlpha` 暗区问题；缺少运行时全局参数时必须明示限制。
- 执行生成可执行程序的命令前仍须用户明确授权并检查程序占用。当前未构建、未运行应用，测试断言尚未执行。

## 首个目标样例

- 工程：`C:\Git\g001_client_new`，Unity 2022.3.62f3；项目 `m_ActiveColorSpace: 0`（Gamma）。本机可读取对应 Unity 安装的 Shader 依赖，尚未执行 UnityShaderCompiler 或编译 Shader。
- `Assets/Res/Materials/sprite_glow.mat` 的 Shader GUID 为 `3250f33607ea9e540b203a4541fe32e0`，已与 `pf_sprite_overlay.shader.meta` 核对。
- SpriteRenderer 提供 `_MainTex`、切片 UV 与非预乘顶点颜色；材质保存 `_AlphaScale=1`、`_Togglelit=0`、`_ToggleDayLit=0`。空材质贴图引用不能覆盖 SpriteRenderer 的贴图。
- 顶点入口 `vert`、片元入口 `frag`，引用 `UnityCG.cginc`；片元执行 `col.a *= lerp(1,abs(_ToggleDayLit - _LightIntensity),_Togglelit) * _AlphaScale` 与 `col = lerp(col,col*i.color,col.a)`，不是此前已删除实验中的反色公式。
- Shader 状态为 `Blend DstColor SrcAlpha`、`ColorMask RGB`、`Cull Off`、`ZWrite Off`；需要与当前场景目标颜色混合，不能先烘焙成普通透明精灵再叠加。
- `_LightIntensity` 由工程昼夜脚本 `Shader.SetGlobalFloat` 写入；本材质 `_Togglelit=0`，该值不影响此次 Alpha 计算。其他材质仍须提供运行时全局参数，不能仅通过读取 YAML 自动还原脚本状态。
- 尚未还原或运行对比该样例；用户提供的 Unity 截图是目标参考，不作为 Viewer 验证通过的证据。

## 管线兼容范围

- 第一目标为当前样例的 Built-in CG/HLSL，需兼容 `UnityCG.cginc`、顶点属性、坐标变换与材质绑定，再生成可交给 GPU 编译的程序。GLSL 编译成功不能证明 CG/HLSL 转换正确。
- 用户要求考虑 URP：将其作为独立兼容层，后续处理 UniversalPipeline 标签、包 Include、Pass / LightMode、灯光与全局资源。旧 LWRP 另行区分；Built-in 不是 LWRP 或 URP。
- 当前不支持 URP / LWRP / ShaderGraph 实际执行，也不支持执行项目 C# 脚本。尚未决定完整 HLSL 到 GLSL 工具链，不能将手写样例 GLSL 宣称为根据引用 Shader 自动还原。
