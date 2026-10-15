# Unity YAML Viewer

[中文](#中文) | [English](#english)

---

## 中文

**Unity YAML Viewer** 是一个面向 Unity 文本序列化资源的桌面工具。它将 `.prefab`、`.unity`、`.asset` 等 YAML 文件解析为更易理解的层级和 Inspector 视图，帮助你在不打开 Unity 编辑器时检查资源内容。

项目现已支持**属性编辑**：通过 Inspector 控件修改已识别的值，确认保存后写回原文件。类型校验、引用保护和精确片段写回帮助减少手动编辑 YAML 时的缩进、引用或格式损坏问题。

### 已支持

- 材质节点交互参考 Amplify Shader Editor：数值、颜色、纹理与逐行分量直接在模块内输入，每个 Property 模块可单独重置，不另设节点参数 Inspector；保留低对比网格、深浅主题紧凑节点、类型色条、细连线和橙色选中边框。输出实时画面位于画布中的可拖动预览模块，可从右下角拉伸大小，支持“定位输出”。2D 保留平面与棋盘格 / 纯色背景切换；3D 可选择球体、圆柱体、立方体真实网格，拖动旋转并重置姿态，复用当前材质参数与纹理。完整预览说明集中在菜单。
- 预览面板与 Inspector 随主窗口可用宽度按比例调整，手动拖动分隔线后保持新的比例；窄窗口对两侧保留可用空间。材质节点连线两端水平出线、控制柄随距离与画布缩放变化；Frag 输出端口固定在组框顶部附近，避免源码长度改变出线高度。
- Windows 应用图标包含 16 / 20 / 24 / 32 / 40 / 48 / 64 / 72 / 80 / 96 / 128 / 192 / 256 像素，各档直接使用 assets 中对应尺寸的 PNG，打包时不再缩放或做颜色转换。
- 材质 / Shader 节点预览开启时，可将单张本地 PNG、JPG、JPEG、TGA、BMP 或 WebP 图片拖到可见的 2D 纹理 Property 模块，临时替换纹理并保留 Tiling / Offset，不写回文件。其他落点、多文件拖入仍按普通文件打开；拖放源需提供本地文件路径（Unity 拖放取决于其是否提供该路径）。预览缓存尺寸不支持时跳过缓存继续直接渲染，零像素裁剪区域跳过当前帧。

- Material（`.mat`）和 Shader（`.shader`）使用 Node-based UI（节点式界面）预览，通过统一“预览”按钮展开 / 收起可调宽的独立节点预览面板，遵循自动打开预览设置；Inspector 保留属性 / YAML，Shader 源码单独查看。节点采用 Property 标题、类型色条、端口、贝塞尔连线和网格背景，可拖动标题调整位置；输入直接在节点内编辑，RGBA / XYZW 分量各占一行。Color 声明及可识别的分量 / 颜色运算上下文决定默认颜色视图，float4 / half4 精度不代替语义判断，可手动切换 RGBA / XYZW；可识别的 RGB / A / 单分量访问与赋值显示对应连接，未使用属性不虚构连线。节点输入预览中：材质加载实际输入、Shader 加载 Properties 默认值；可临时调整数值 / 颜色 / 向量、替换 2D 纹理和修改纹理缩放偏移，并重置输入。修改仅保存在当前预览内存中，不写回文件；画布可平移缩放，节点连线表示参数绑定，Vert、Frag 与各辅助函数以可拖动大框分组，函数体以只读源码语句卡片展示，尚不拆成算术算子节点；已识别的函数调用单独标注。Vertex Input 与 v2f / Varyings 接口分别分组，列出字段、类型和语义；TEXCOORD1/2/3 等插值通道不等同于模型 UV1/2/3，显示声明不代表渲染器已绑定通道。平面预览不包含场景光照、蒙版及自定义顶点流；整数 uniform、非四分量向量 uniform 和镜像纹理采样暂不支持。受限 Built-in 单 CG Pass 使用 GPU 平面 / 预置模型预览；URP / HDRP、多 Pass、复杂宏、无法解析的引用及非 2D 纹理不保证渲染，显示具体原因。`.mat` 需可解析的 Shader GUID 和项目资源索引；缺少材质参数时使用 Shader 默认值，重新载入预览清除临时输入，文件级重载才重读当前文件。

- 独立应用弹窗新开或重新唤醒时恢复主窗口居中位置，并请求显示、取消最小化和前置聚焦；再次点击设置 / 关于入口也会唤醒已有窗口。初始定位完成后仍可手动移动。设置中的预览开关说明限制在右列内换行，长语言文案不覆盖左侧标签。

- 支持本地 `.m3u8` / `.m3u` 音视频播放列表，默认保存 UTF-8 M3U8；从当前媒体右侧面板按目录全部文件或文件名前缀创建，默认不递归，按自然顺序排列，保存前可预览、排除和排序，名称可修改，同名覆盖需确认。优先保存相对列表位置的路径，跨盘保存绝对路径；M3U 兼容 UTF-8、UTF-16 BOM 及 Windows 当前 ANSI 编码。
- 每份列表只占一个页签，最近打开只记录列表；当前媒体信息随条目切换，列表播放器和进度独立于普通媒体页签。视频支持紧密编号网格或名称单列，音乐使用名称单列，面板内纵向滚动，滚动条始终靠右内边缘；侧栏仅占媒体画面区域，上方文件信息和底部时间轴保持全宽；标题顶部增加 8 点留白，管理菜单采用随主题着色的 SVG 扳手图标；左侧内容增加 2 点内边距，创建“保存并使用”按钮底部居中、128×36 点。悬浮箭头展开 / 收起、边界拖动调宽，各列表面板最大宽度统一为窗口宽度的三分之一，最小目标宽度 220 点（小窗口随上限收缩）；全屏浮层默认半屏高、垂直居中，上下边缘拖动保持中心并对称改变高度，按列表记住高度。全屏展开列表使用带细主题边框的半透明浮层覆盖画面；背板 45% 不透明度，控件保持清晰，左上 / 左下 30 点圆角、右侧直角贴窗口边缘，拖宽时右缘固定，不挤占视频可用区域或改变视频缩放，底部时间轴保持全宽；鼠标停止活动 1 秒后隐藏全屏箭头与列表，移动后恢复，拖动或输入时保持可见；收起时 SVG 箭头按钮贴媒体区域右边缘、图标居中，不预留列或缩小适应窗口的宽度。窗口和全屏的展开 / 收起箭头仅在媒体画布内鼠标移动时显示，停止 1 秒或移出区域后隐藏，不改变列表状态。前缀创建时可去掉名称前缀，显示名称写入通用 EXTINF 标题，不修改原文件名。
- 列表播放控制栏使用矢量“下一项”按钮替代停止，窗口 / 全屏一致；手动下一项在单曲循环中仍切换到下一条，随机模式使用随机队列，顺序末项暂停。
- 音频列表在当前文件名及大小后显示当前播放模式，收起列表后仍可见；支持顺序（末项暂停）、随机（每轮不重复，持续下一轮）和单曲循环。视频“跳过片头片尾”按每份列表独立设置非负整数秒（单位固定 `s`）；时间输入框为 60×24 点，输入框 / s / 按钮采用 2 点紧凑间距，框后“片头”按钮取当前播放时间，“片尾”按钮取总时长减当前时间，向下取整到秒，未知时长时禁用片尾按钮；复选框后的“重置”将当前列表两项时间同时恢复为 `0`，不改变开关；到总时长减片尾时切到下一集，最后一集暂停并在画面中央显示“已是最后一集”和确定按钮；无效时间或片头片尾覆盖整集时暂停并提示。
- 列表本地记录当前条目、时间和媒体身份，重新打开恢复；手动 / 自动切换从零或片头结束处播放。播放中约每十秒、暂停、切换、关闭和正常退出时保存；目录扫描和定时记录在后台处理。各列表模式、跳过时间和面板偏好独立记忆；清理缓存只删除列表进度，保留列表设置及用户列表文件。列表改名 / 移动不追踪，缺失条目提示并停下，媒体内容修改不使用旧进度。

- “格式支持”和设置中的“文件关联”可进入同一个默认程序弹窗，按分组查询 Windows 当前默认程序。每项“去设置”复制后缀并打开系统默认应用页面，需粘贴搜索并由用户确认；可复制本程序 EXE 路径，不自动注册或改写关联。复合后缀按最后一段处理，共用后缀可能包含不同内容。
- 音视频打开后在媒体信息就绪时自动播放，有记录时从上次位置继续；重复打开已有媒体页签会继续播放，普通页签切换保持暂停行为。关闭页签、切换页签或正常退出时记忆播放位置。记录按路径、大小及修改时间核对，最多 500 项、2 MiB；文件替换、位置无效或已播放完毕时从头开始，网络下载缓存不记录播放位置。
- 设置中的“文件关联”“软件缓存”依次位于“最近打开记录”上方；缓存项以 KB / MB / GB 显示占用并提供文件管理器查看及后台清理，不显示固定说明段落。保留设置、最近文件及页签恢复信息；清除播放记录、可重建项目索引及未使用的网络 / 更新下载。打开中的文件、待安装更新、更新失败备份及未知文件保留，目录链接不跟随；占用或权限错误会提示。下载 / 索引扫描时禁用清理，缓存使用后可重新生成；不清理外部 FFmpeg 或用户资源。

- 音频信息显示采样率、位深、声道和音频流码率；视频的音频信息也显示当前所选音轨码率，切换音轨同步更新（kb/s）；元数据缺失时显示未知，不使用容器总码率替代。波形与底部控制栏保持紧凑间距。
- 音轨菜单读取标题（缺失时读取 handler_name）；常见语言名称与代码按界面语言显示，包含英语、国语、粤语、日语等 18 类语言；未覆盖的标记保留原文，泛中文不推定方言，悬浮查看原始名称和语言标记，便于区分多音轨；不识别实际语音或自动纠正文件标记。

- 工具栏提供可收起的放大镜查找入口，窄窗口自动换行；收起时显示拖入文件提示及格式支持按钮，可打开后缀列表；最近打开支持单项删除及非空时全部清空；删除长名称后框体宽度随列表调整，矢量减号保持右侧对齐。浅色音频电平峰值线使用深灰色配色；悬浮文件页签名称显示完整路径；过长页签名称中间省略，保留末尾两个字符及扩展名。
- 文件菜单支持打开 HTTP / HTTPS 文件直链：后台下载完成后使用已有查看器打开，最大 1 GiB、总超时 5 分钟；网页解析、登录认证和边下载边播放暂不支持。新下载文件缓存在软件数据目录，不加入最近打开或恢复页签；可在设置中清理未使用的下载缓存。

- 展开的 Prefab 实例根节点可从 Inspector 标题旁打开源资源页签，场景覆盖不会复制到源资源。基础粒子支持预算内循环预热、缓存重播、Billboard 独立宽高及静态 ConeVolume。

- 二维场景与 Prefab 共用基础粒子实时预览，提供播放、暂停和固定种子重播；View Billboard 使用 Renderer 材质在 GPU 绘制，支持持续发射 / Burst、初始属性、大小 / 颜色 / Z 旋转生命周期变化及部分基础形状。

- 场景与独立 Spine 3.8 预览可按原始网格 / 图集和材质执行受限 GPU Shader；场景入口携带 Renderer 材质与初始皮肤，独立动画更新网格，直接打开 SkeletonDataAsset 使用图集默认材质。

- 场景内有效空 Mesh 正常跳过绘制，不误报资源缺失；与零顶点矛盾的缓冲数据仍提示错误。

- TilemapRenderer 首材质接入受限 GPU Shader 预览，保留各 Tile 的纹理、UV 与颜色；不支持的 Shader 回退普通贴图并提示。

- 场景 / Prefab 静态 2D 与 3D 预览：Sprite、Tilemap、常见 UI、模型及 Spine 静态合成；支持同文件 fileID 引用的未压缩文本三角 Mesh（线拓扑暂不支持）；支持嵌套 Prefab 展开和常见覆盖，按层索引实例引用并共享同源模板（模板 / 展开文本各 512 MiB，每层最多 50 万文档）；展开状态下场景点选嵌套 Prefab 会展开祖先、滚动并高亮对应节点；引用节点属性只读；展开限制详情可一键复制。
- GPU 场景预览首次渲染完成后，鼠标离开画面或程序失去焦点时冻结最后一帧，返回后恢复；冻结画面在窗口缩放时保持比例。
- GPU 场景预览首次渲染完成后，鼠标离开画面或程序失去焦点时冻结最后一帧，返回后恢复；冻结画面在窗口缩放时保持比例。
- Spine 3.8 独立骨骼预览：动画、皮肤、多轨播放、速度、时间轴、缩放和平移。
- FBX / OBJ 模型预览：贴图、颜色、实体 / 线框、网格统计、静态蒙皮与视角操作；单蒙皮 FBX 按名称匹配外部材质并保留多个槽，已识别的 PFCore/PF3D2_Default / PF3D2_Transparent 兼容主色、Alpha 与颜色调整公式。
- 图片通道查看、缩放和平移；常见脚本 / 文本查看、基础语法高亮及工程字体预览。
- 音频 / 视频播放：播放、暂停、停止、进度跳转、音量及空格键控制；音量弹窗仅显示竖直滑条，不显示百分比数字；视频支持音轨选择、暂停方向键逐帧及自动 / 软解 / 硬解选项。
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
- 设置、关于、更新、加载进度、编辑确认及颜色 / 渐变面板使用独立系统窗口，新开或重新打开时外框中心与主窗口外框中心对齐，初始尺寸稳定后可手动移出主窗口，内容超出窗口时可滚动；系统文件选择器及 Windows 设置页面由系统管理位置。设置支持主题、语言、预览、Hierarchy 展开层级、视频解码及撤销容量等选项；最后两项固定为“单个文件缓存撤销步骤”和“检查更新”。

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

- 节点画布目前用于源码驱动的展示和临时输入预览；新增 Property、编辑 / 保存连线、生成 Shader 及完整替代 Unity Shader Graph 尚未实现；圆角立方体、自定义预览模型导入与 HDRI / PBR 环境尚未实现。当前 3D 仅提供凸形预置网格的正交预览，原始三角形按视深排序，复杂顶点位移、自交 / 透明相交与完整场景深度仍有限制；不为无光照 Shader 额外伪造光照。组件用途判断只覆盖可识别的直接访问、赋值和部分颜色运算；函数间数据流、复杂宏和子图不保证识别，无法确定时保留通用参数连接或 XYZW。

- 播放列表目前只支持本地媒体及本地 file URI，不支持 HLS 或网络媒体条目；每份列表最多 10,000 项、8 MiB。相对路径依赖列表与媒体的相对位置；随机顺序不跨程序重启保存，崩溃 / 强制退出不保证最后几秒进度。

- 粒子为有限 CPU 模拟与 GPU 绘制：UI Canvas / 3D、Mesh / Stretch Renderer、Custom Vertex Streams、纹理动画、碰撞、子发射器、轨迹、复杂力场 / Shape、超预算预热和发射器动画未还原；不支持的启用模块会提示。最多 128 系统、每系统 4096 粒子、每帧 65536 可见粒子；曲线积分、随机分布和重力为预览近似，不保证与 Unity 一致，尚无逐粒子拾取。

- Spine 特殊混合 / 双色染色、缺少材质映射或不支持的 Shader 回退普通显示；仅略过明确的 ShadowCaster Pass，其他多颜色 Pass、条件变体、阴影 / 遮罩和昼夜脚本未还原。原始骨骼文件须能通过同目录资源索引定位配套 SkeletonDataAsset 才能取得材质上下文；UI Spine 与 3D 模式仍未接入本路径。

- 静态模型使用最近表面烘焙，完整多 Pass、剔除、场景深度状态及透明表面叠加尚未还原；其他自定义 Shader 保持近似显示。

- GameObject 属性视图显示名称、Active、Layer、Tag，保留 YAML 切换；编辑模式可修改本地节点 Active，名称 / Layer / Tag 和展开的 Prefab / FBX 节点暂只读。父节点失活不批量改写子节点 Active。

- 嵌套 FBX Model Prefab 可展开已验证的单网格节点模型，沿场景 fileID / stripped 引用应用父节点、变换、名称、排序和材质槽覆盖；单一未蒙皮网格可将原始三角形、UV0、颜色与法线传入受限的 GPU Shader 路径。多节点、缺少唯一源映射、蒙皮暂不支持，完整导入器与切线还原仍有限制。

- 纯 2D 场景提供有限 Built-in 单 Pass Sprite / 网格 Shader GPU 路径，运行时读取Assets、Packages 和 Library/PackageCache 内引用源码及相对 / 项目路径 Include（解析后的真实路径须位于这些资源目录；Packages 虚拟引用可匹配缓存包，同名多版本无法唯一确定时报错），绑定多纹理、标量、颜色 / 向量、世界矩阵与时间；Blend Off 或未声明时采用 One/Zero 不透明默认值，支持材质参数及 Shader 属性默认值；支持不依赖实例化条件分支的 multi_compile_instancing Shader 按独立绘制执行及 rsqrt，主纹理 TexelSize 按实际绑定纹理提供，片元 clip 保留负分量丢弃语义；TEXCOORD1/2 的标量及 2～4 分量顶点输入暂填零，未还原粒子自定义顶点流；兼容简单外层 Include Guard、2×2 / 3×3 矩阵类型及函数内 uniform 局部副本修改，Cull / ZWrite / ZTest 参数解析后仍限于当前支持组合（Cull Off/Back、ZWrite Off，完整深度未支持；Stencil 遮罩忽略并提示，其他兼容条件满足时继续执行 Shader）；场景纹理总预算为 2 GiB（CPU 缓存与 GPU 源纹理分别计量，并非进程总内存上限）。预览无雾；未使用的标量声明不要求赋值，未知已引用标量默认 1 并提示，无法读取 Unity 脚本运行时 Global 值。昼夜全局颜色默认白色（RGBA 均为 1）；支持含 UTF-8 BOM 的 Shader Include。预览左上角悬浮显示提交绘制帧率 FPS，滚轮以鼠标位置为中心缩放。不支持的 Shader 报告诊断并回退普通贴图或烘焙图。CG 到 GLSL 仅支持部分语法与 Unity 函数，不保证与 Unity 完全一致；混合 UI 场景、URP / LWRP、ShaderGraph、完整光照、完整粒子模拟及场景整体动画暂不支持此路径。
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

- Material-node interactions follow the requested Amplify Shader Editor reference: edit numbers, colors, textures and individual components inside each module, with a reset action on every Property node and no separate node-parameter Inspector. The graph retains a subtle grid, compact themed nodes, type accents, thin connections and orange selection outlines. The live output is inside a draggable graph module with a bottom-right resize grip and a Locate output action. 2D shows the plane with checkerboard or solid background; 3D offers real sphere, cylinder and cube meshes, drag rotation and rotation reset, reusing the current material values and textures. Detailed preview notes are available from a menu.
- Preview panels and the Inspector share available window width proportionally, preserving a new split after dragging the divider and reserving space for both panes in narrow windows. Material-node wires use horizontal endpoint tangents with handles based on distance and graph zoom. The Frag output port stays near the top of its group so source length does not change its connection height.
- Windows application icons include 16 / 20 / 24 / 32 / 40 / 48 / 64 / 72 / 80 / 96 / 128 / 192 / 256 px. Each size uses its matching PNG in assets directly, without resizing or color conversion during packaging.
- With material / shader node preview open, drop one local PNG, JPG, JPEG, TGA, BMP or WebP image onto a visible 2D texture Property module to replace its preview texture, retaining Tiling / Offset without writing files. Other targets and multiple-file drops open files normally. The source must supply a local file path; Unity drag support depends on it providing that path. Unsupported snapshot dimensions bypass the cache and keep direct rendering; zero-pixel clipped regions skip the current frame.

- Materials (`.mat`) and shaders (`.shader`) have a node-based UI preview and use the shared Preview button to expand or collapse a separate resizable node panel, following the automatic-preview setting. The Inspector keeps material properties/YAML, and shader source remains available separately. Property nodes have type accents, ports, curved connections and a graph grid; drag headers to reposition nodes. Edit inputs directly inside nodes, with each RGBA / XYZW component on its own row. Color metadata and identifiable component/color usage determine the default presentation, independently of float4/half4 precision; RGBA/XYZW presentation can be switched. Recognizable RGB/alpha/component accesses and assignments have corresponding connections; unused properties remain unconnected. Materials load their saved inputs; shaders load Properties defaults. Temporarily adjust numbers, colors and vectors, replace 2D textures, edit texture tiling/offset, or reset inputs without writing files. The canvas pans and zooms; connections represent input bindings, with separate draggable Vert, Frag and helper-function groups. Function bodies use read-only source statement cards rather than reconstructed arithmetic nodes; identifiable calls are labeled separately. Vertex Input and v2f / Varyings interfaces list their fields, types and semantics independently. TEXCOORD1/2/3 interpolators need not be mesh UV1/2/3, and showing a declaration does not imply renderer support. Scene lighting, masks and custom vertex streams are excluded; integer uniforms, vector uniforms other than four components, and mirrored texture sampling are unsupported. A limited Built-in single-CG-pass subset has GPU plane and preset-model previews. URP/HDRP, multiple passes, complex macros, unresolved references and non-2D textures may prevent rendering; the reason is shown. Materials require a resolvable shader GUID and project resource index. Missing material values use shader defaults; reinitializing the preview clears temporary overrides, while file-level reload rereads the current file.

- Native app dialogs center on the main window when opened or reactivated and request visibility, restoration and focus. Clicking Settings/About again reactivates the existing dialog. Manual movement remains available after placement settles. Long preview-toggle text wraps inside the right Settings column without covering the left label.

- Local `.m3u8` / `.m3u` audio/video playlists, saved as UTF-8 M3U8 by default. Create from all matching media in a folder or a filename prefix in the current media's side panel, without recursion and with natural sorting. Preview, exclude and reorder entries before saving; edit the name and confirm overwrites. Paths are relative to the playlist where possible, absolute across drives. M3U accepts UTF-8, UTF-16 BOM and the current Windows ANSI encoding.
- A playlist occupies one tab and is the only item added to recent history. Its current media information changes with selection; its player and progress are independent of ordinary media tabs. Video uses a compact number grid or a single name column; audio uses a name column, with vertical scrolling and a scrollbar fixed to the inner right edge. The windowed playlist occupies only the media canvas; metadata and transport timeline retain full width. The header has 8 points of top padding and a theme-tinted SVG wrench for its management menu. Content has 2 points of left padding, and the 128×36 Save and use button is centered at the bottom. A floating arrow toggles the resizable sidebar. All playlist panels are capped at one third of the window width, with a target minimum of 220 points (reduced for small windows). The fullscreen overlay defaults to half the screen height, stays vertically centered and resizes symmetrically from its top or bottom edge; height is remembered per playlist. In fullscreen, the playlist overlays the video with a 45%-opaque backplate and clear controls, a thin theme border, 30-point left corners and square right corners pinned to the window edge during resize, preserving the video area, scaling and full-width transport; fullscreen arrows and the playlist hide after one second of inactivity and return on activity, remaining visible during dragging or text input; when collapsed, a centered SVG arrow floats at the right edge of the media region without reserving a column or reducing fit-to-window width. In windowed and fullscreen modes, expand/collapse arrows appear only on mouse movement within the media canvas, hiding after one second or when the pointer leaves without changing playlist state. Prefix matching can strip that prefix from display titles, exported as standard EXTINF titles without renaming media files.
- Playlist transport replaces Stop with a vector Next button in windowed and fullscreen modes. Manual Next advances even in repeat-one mode, uses the shuffle queue in shuffle mode, and pauses at the end of a sequential list.
- Audio playlists show the current playback mode after the filename and size even when collapsed; they support sequential playback (pause at the end), shuffle (no repeats within each round, continuing into new rounds) and repeat one. Skip intro and outro stores times separately for each playlist as nonnegative integer seconds with the fixed unit `s`. Time inputs use 60×24 points with 2-point spacing between the input, s and button. The Intro button uses the current position; Outro uses total duration minus current position, rounded down to seconds. Outro is disabled when duration is unknown; Reset next to the checkbox sets both times to `0` for the current playlist without changing the checkbox; reaching duration minus outro advances to the next episode. The last episode pauses with a centered message and OK button. Invalid times or skips covering the entire episode pause with a warning.
- Local playlist state remembers the current entry, time and media identity for resume. Manual or automatic entry changes start at zero or after the intro. Background records save roughly every ten seconds during playback and on pause, switching, close and normal exit. Folder scans run in the background. Modes, skips and sidebar preferences are independent per playlist; cache cleanup removes playlist progress while keeping settings and user playlist documents. Playlist moves/renames are not tracked; missing entries pause with an error and changed media does not reuse stale progress.

- Supported formats and Settings → File associations open the same default-app dialog, with grouped Windows association queries. Go to settings copies an extension and opens Default apps; paste it into search and confirm the choice in Windows. The EXE path can also be copied. No automatic app registration or association changes are made. Compound suffixes use the final extension; shared extensions may contain different content.
- Audio/video plays automatically once metadata is ready, resuming the last saved position. Opening an existing media tab resumes playback; ordinary tab switching keeps it paused. Positions are saved when closing or switching tabs and on normal exit. Records are checked against the path, file size and modification time, with limits of 500 entries and 2 MiB. Replaced files, invalid positions and completed playback restart at zero; downloaded network-cache files have no playback records.
- File associations and App cache appear above Recent file history in Settings. The cache section shows usage in KB / MB / GB, opens its folders and cleans disposable files in the background, without permanent explanation paragraphs. Settings, recent files and tab restore data stay; playback records, rebuildable project indexes and unused network/update downloads are removed. Open files, pending updates, failed-update backups and unknown files stay; directory links are not followed and access errors are reported. Cleanup is disabled during downloads/index scans, and caches may regenerate during use. External FFmpeg installations and user assets are excluded.

- Audio information shows sample rate, bit depth, channels and audio-stream bitrate (kb/s); missing bitrate is shown as unknown rather than replaced with container bitrate. Video audio information also shows the selected track bitrate and updates on track changes. The waveform sits close to the transport bar.
- Audio track menus read metadata titles (falling back to handler_name), localize 18 common language categories including English, Mandarin, Cantonese and Japanese (unknown tags remain unchanged; generic Chinese tags do not imply a dialect), and show original names and language tags on hover; no speech identification or automatic metadata correction.

- A collapsible magnifying-glass search control moves to a second row in narrow windows; when collapsed, a drag-and-drop hint and Supported formats button open an extension list; recent files can be removed individually or cleared when nonempty; deleting a long name also updates the menu width, keeping vector minus buttons aligned at the right. Audio peak lines use dark gray in light mode; hovering over a file tab name shows its full path. Long tab names use middle ellipsis, retaining the last two stem characters and extension.
- The File menu opens direct HTTP / HTTPS file URLs after a background download (up to 1 GiB, 5-minute total timeout). Web-page parsing, login and playback during download are unsupported. New downloads use the app data directory and are excluded from recent files and restored tabs; unused downloads can be cleaned in Settings.

- Expanded Prefab roots can open source assets in separate tabs from the Inspector title; scene overrides are not copied. Basic particles support bounded looping prewarm, cached replay, independent billboard width / height and static ConeVolume emission.

- 2D scenes and Prefabs share basic real-time particle previews with play, pause and repeatable seeded replay. View billboards use Renderer materials on the GPU, with continuous emission / bursts, initial properties, size / color / Z rotation over lifetime and selected basic shapes.

- Scene and standalone Spine 3.8 previews can execute limited GPU shaders using original meshes, atlas pages and materials. Scene navigation carries Renderer materials and the initial skin; standalone animation updates geometry, while directly opening a SkeletonDataAsset uses atlas materials.

- Valid empty scene-local Meshes are skipped without missing-resource warnings; buffers inconsistent with zero vertices still report errors.

- TilemapRenderer uses its first material in the limited GPU Shader preview, preserving each Tile texture, UV and color; unsupported shaders fall back to plain textures with diagnostics.

- Static 2D / 3D scene and Prefab previews with Sprites, Tilemaps, common UI, models and Spine poses; support uncompressed text triangle Meshes referenced by local fileID (line topology is unsupported); nested Prefab expansion and common overrides, with per-level reference indexing and shared source templates (512 MiB each for templates and expanded text, up to 500,000 documents per level); picking a nested Prefab in the scene while expansion is enabled reveals, scrolls to and highlights its Hierarchy node; referenced nodes are read-only; expansion diagnostics can be copied with one click.
- GPU scene previews complete their first render, then freeze when the pointer leaves the canvas or the app loses focus; returning resumes updates. Frozen images retain their aspect ratio on resize.
- Standalone Spine 3.8 previews with animation, skins, multiple tracks, playback speed, timeline, zoom and pan.
- FBX / OBJ previews with textures, colors, solid / wireframe display, mesh statistics, static skinning and camera controls; single-skinned FBX previews match external materials by name and retain multiple slots, with main color, alpha and color adjustment formulas for recognized PFCore/PF3D2_Default / PF3D2_Transparent shaders.
- Image channels, zoom and pan; script / text viewing, basic syntax highlighting and project font previews.
- Audio / video playback with pause, stop, seeking, volume and Space controls; the volume popup shows only a vertical slider without percentage numbers; video audio-track selection, paused arrow-key frame stepping and automatic / software / hardware decoding options.
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
- Settings, About, update dialogs, loading progress, edit confirmations and color / gradient panels use separate native windows with scrolling when needed. New or reopened windows align their outer-frame center with the main window and remain movable once their initial size settles. System file pickers and Windows Settings retain system-managed positioning. Settings includes theme, language, preview, hierarchy expansion, video decoder and undo options. Undo history capacity and Check for Updates remain the final two items.

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

- The graph is a source-driven viewer with temporary input overrides. Creating Property nodes, editing/saving connections, generating shaders and replacing Unity Shader Graph are not implemented. Rounded-cube meshes, custom preview-model import and HDRI/PBR environments are pending. The 3D viewer uses orthographic convex presets with triangles sorted by view depth; complex vertex displacement, self-intersection, intersecting transparency and full scene depth remain limited. Unlit shaders receive no invented lighting. Component inference covers identifiable direct accesses, assignments and some color operations; interprocedural data flow, complex macros and subgraphs may be unresolved, retaining generic parameter connections or XYZW when uncertain.

- Playlists support local media paths and local file URIs only, with no HLS or network entries; up to 10,000 entries and 8 MiB per document. Relative paths depend on the folder relationship. Shuffle order does not persist across restarts, and crashes/forced exits may lose the most recent seconds of progress.

- Particles use limited CPU simulation and GPU drawing. UI Canvas / 3D, Mesh / Stretch renderers, custom vertex streams, texture animation, collisions, subemitters, trails, complex forces / shapes, prewarm beyond the budget and emitter animation remain unsupported, with diagnostics for enabled modules. Limits are 128 systems, 4,096 particles per system and 65,536 visible particles per frame. Curve integration, random distributions and gravity are preview approximations; per-particle picking is unavailable.

- Spine special blends, two-color tint, missing material mappings and unsupported shaders use ordinary fallback. Only explicit ShadowCaster passes are omitted; multiple color passes, conditional variants, shadows, masks and day/night scripts are not reproduced. Raw skeleton files need an indexed companion SkeletonDataAsset in the same directory for material context; UI Spine and 3D mode remain outside this GPU path.

- Static model baking retains the nearest surface; full multipass, culling, scene depth states and layered transparent surfaces remain unsupported. Other custom shaders use approximate display.

- GameObject properties show Name, Active, Layer and Tag with a YAML switch. Edit mode can change Active on locally serialized objects; Name/Layer/Tag and expanded Prefab/FBX objects remain read-only. Disabling a parent does not rewrite its children's Active flags.

- Nested FBX Model Prefabs can expand verified single-mesh-node models, using scene fileID/stripped references for parenting, transforms, names, sorting and material-slot overrides. A single unskinned mesh can pass raw triangles, UV0, colors and normals to the limited GPU shader path. Multiple nodes, missing unique source mappings, skinning are unsupported; full importer and tangent fidelity remain limited.

- Pure 2D scenes have a limited GPU path for single-pass Built-in Sprite/mesh shaders. Referenced source in Assets, Packages and Library/PackageCache and relative/project includes are read at runtime (resolved paths must remain within these resource directories; virtual Packages references can resolve cached packages, while ambiguous versions report errors), with multiple textures, scalar/color/vector properties, world matrices and time bound to the program. Blend Off or absent Blend uses the opaque One/Zero default. Shaders declaring multi_compile_instancing can use individual draws when their code does not require instancing branches; rsqrt and bound main-texture TexelSize are supported; fragment clip discards fragments with negative components. Simple outer include guards, 2×2/3×3 matrix types and local copies for function-level uniform writes are supported. Blend supports material properties and shader property defaults; parameterized Cull/ZWrite/ZTest remain limited to supported combinations (Cull Off/Back and ZWrite Off; full depth is unsupported; Stencil masks are ignored with diagnostics, while otherwise compatible shaders continue). Scene textures have a 2 GiB budget (CPU cache and GPU source textures are counted separately; this is not a process memory limit). The preview has no fog; unused scalar declarations need no binding, while unresolved referenced scalars default to 1 with diagnostics. Unity script runtime globals cannot be read from these files. Global day/night colors default to white (RGBA all 1); UTF-8 BOMs in shader includes are supported. A floating FPS overlay shows submitted preview frames, and wheel zoom is centered on the pointer. Unsupported shaders report diagnostics and fall back to ordinary textures or baked images. CG-to-GLSL syntax and Unity function compatibility are partial; full fidelity is not guaranteed. Mixed UI scenes, URP/LWRP, ShaderGraph, full lighting, full particle simulation and scene-wide animation are outside this path.
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

静态蒙皮 FBX 按资源轴向转换，并兼容可唯一确认的根骨骼均匀缩放覆盖；其他骨骼覆盖仍未还原。
Static skinned FBX previews convert declared axes and support unambiguous uniform skeleton-root scale overrides; other bone overrides remain unsupported.
