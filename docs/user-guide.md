# Unity YAML Viewer 使用指南 / User Guide

[返回首页 / Back to README](../README.md) · [中文](#中文) | [English](#english)

[详细功能与限制 / Feature reference](feature-reference.md) · [命令行参考 / CLI reference](command-line.md)

---

## 中文

**Unity YAML Viewer** 是一个面向 Unity 文本序列化资源的桌面工具。它将 `.prefab`、`.unity`、`.asset` 等 YAML 文件解析为更易理解的层级和 Inspector 视图，帮助你在不打开 Unity 编辑器时检查资源内容。

项目现已支持**属性编辑**：通过 Inspector 控件修改已识别的值，确认保存后写回原文件。类型校验、引用保护和精确片段写回帮助减少手动编辑 YAML 时的缩进、引用或格式损坏问题。

### 已支持

- Vert 同样支持逐步选择标量、向量、输入 / 输出结构体字段及分量。选中值在执行到该步时截取，通过空闲 Varying 传到片元阶段插值显示；原 Vert 后续计算及最终顶点位置继续执行，保留当前平面 / 3D 模型形状。界面标注“Vert 顶点值（插值显示）”，不等同于逐像素重算，也不显示原始顶点数值表。整个输出结构体、多个 / 提前 return、无空闲插值通道或不可解析类型不支持。


- Frag 顺序语句显示为带编号与结果端口的步骤节点；点击可预览该步赋值后的标量 / 向量 / 分量，顶部可切换 A 与最终输出。实际执行原 Shader 该步之前的代码并保留采样、UV、函数调用和时间，不以 CPU 伪造图像。标量为灰度、二维为 RG、三维 / 四维为 RGB；A 灰度独立查看。诊断值使用不透明输出并暂时绕过 Blend，Render State 控件禁用；返回最终输出恢复临时状态覆盖。
- 单步目前限 Frag 直线赋值 / 变量返回（每函数最多 32 条）；未知类型、未初始化声明、分支 / 循环前缀及非标量 / 向量不保证可预览，显示原因。辅助函数快照限可唯一确定的直接阶段调用；尚不支持完整算术节点逆向拆解或节点连线编辑，节点快照不连续实时更新。负数 / 大于 1 的诊断值按当前显示范围裁剪，不自动归一化；标量与四分量结果可查看 A。


- 节点数字输入在回车或切换焦点后提交；鼠标拖动数字 / 滑块时只更新节点显示，释放左键后统一刷新渲染。Property 使用按文件与属性区分的稳定标识，后台加载提示和缩略图变化不打断输入。
- Frag → Render State → Output：独立可拖动渲染状态模块，临时设置 Cull Off / Back、Blend 源 / 目标因子并恢复 Shader 状态；Blend Off 等效于 One / Zero。Lighting、ZWrite、ZTest 展示源码状态，当前不新增固定管线光照或深度渲染，Cull Front 不支持；不写文件。


- 材质节点交互参考 Amplify Shader Editor：数值、颜色、纹理与逐行分量直接在模块内输入，每个 Property 模块可单独重置，不另设节点参数 Inspector；保留低对比网格、深浅主题紧凑节点、类型色条、细连线和橙色选中边框。输出实时画面位于画布中的可拖动预览模块，可从右下角拉伸大小，支持“定位输出”。2D 保留平面与棋盘格 / 纯色背景切换；3D 可选择球体、圆柱体、立方体真实网格，拖动旋转并重置姿态，复用当前材质参数与纹理。完整预览说明集中在菜单。
- 颜色 Property 的 RGB 拾色与逐行 A 输入分开处理；透明度为 0 时仍保留 RGB，查看节点不会改变颜色参数，适用于 Shader 分别使用 RGB / A 的情况。
- 预览面板与 Inspector 随主窗口可用宽度按比例调整，手动拖动分隔线后保持新的比例；窄窗口对两侧保留可用空间。材质节点连线两端水平出线、控制柄随距离与画布缩放变化；Frag 输出端口固定在组框顶部附近，避免源码长度改变出线高度。
- Windows 应用图标包含 16 / 20 / 24 / 32 / 40 / 48 / 64 / 72 / 80 / 96 / 128 / 192 / 256 像素，各档直接使用 assets 中对应尺寸的 PNG，打包时不再缩放或做颜色转换。
- 材质 / Shader 节点预览开启时，可将单张本地 PNG、JPG、JPEG、TGA、BMP 或 WebP 图片拖到可见的 2D 纹理 Property 模块，临时替换纹理并保留 Tiling / Offset，不写回文件。其他落点、多文件拖入仍按普通文件打开；拖放源需提供本地文件路径（Unity 拖放取决于其是否提供该路径）。预览缓存尺寸不支持时跳过缓存继续直接渲染，零像素裁剪区域跳过当前帧。

- Material（`.mat`）和 Shader（`.shader`）使用 Node-based UI（节点式界面）预览，通过统一“预览”按钮展开 / 收起可调宽的独立节点预览面板，遵循自动打开预览设置；Inspector 保留属性 / YAML，Shader 源码单独查看。节点采用 Property 标题、类型色条、端口、贝塞尔连线和网格背景，可拖动标题调整位置；输入直接在节点内编辑，RGBA / XYZW 分量各占一行。Color 声明及可识别的分量 / 颜色运算上下文决定默认颜色视图，float4 / half4 精度不代替语义判断，可手动切换 RGBA / XYZW；可识别的 RGB / A / 单分量访问与赋值显示对应连接，未使用属性不虚构连线。节点输入预览中：材质加载实际输入、Shader 加载 Properties 默认值；可临时调整数值 / 颜色 / 向量、替换 2D 纹理和修改纹理缩放偏移，并重置输入。修改仅保存在当前预览内存中，不写回文件；画布可平移缩放，节点连线表示参数绑定，Vert、Frag 与各辅助函数使用独立 In / 步骤 / Out，步骤可单独拖动，统一分组块按明确成员移动，常见表达式拆为运算节点，不保证完整还原所有 HLSL；已识别的函数调用单独标注。Vertex Input 与 v2f / Varyings 接口分别分组，列出字段、类型和语义；TEXCOORD1/2/3 等插值通道不等同于模型 UV1/2/3，显示声明不代表渲染器已绑定通道。平面预览不包含场景光照、蒙版及自定义顶点流；整数 uniform、非四分量向量 uniform 和镜像纹理采样暂不支持。受限 Built-in 单 CG Pass 使用 GPU 平面 / 预置模型预览；URP / HDRP、多 Pass、复杂宏、无法解析的引用及非 2D 纹理不保证渲染，显示具体原因。`.mat` 需可解析的 Shader GUID 和项目资源索引；缺少材质参数时使用 Shader 默认值，重新载入预览清除临时输入，文件级重载才重读当前文件。

- 独立应用弹窗新开或重新唤醒时恢复主窗口居中位置，并请求显示、取消最小化和前置聚焦；再次点击设置 / 关于入口也会唤醒已有窗口。初始定位完成后仍可手动移动。设置中的预览开关说明限制在右列内换行，长语言文案不覆盖左侧标签。

- 支持 `.m3u8` / `.m3u` 音视频播放列表，默认保存 UTF-8 M3U8；从当前媒体右侧面板按目录全部文件或文件名前缀创建，默认不递归，按自然顺序排列，保存前可预览、排除和排序，名称可修改，同名覆盖需确认。优先保存相对列表位置的路径，跨盘保存绝对路径；M3U 兼容 UTF-8、UTF-16 BOM 及 Windows 当前 ANSI 编码。
- 列表可混合本地媒体与 HTTP / HTTPS、RTP、RTSP 网络频道，读取 `#EXTINF` 参数区之后的显示名称和下一行链接（兼容时长后多一个逗号再写参数的列表，保留分组和台标等属性，不用 tvg-id/tvg-name 替代末尾显示名称）；首次打开含网络条目的列表默认显示名称。“打开 URL”可下载 `.m3u` 列表后打开，不再按纯文本改成 `.txt`。选择频道时复用现有 FFmpeg 播放器，仅连接所选条目；保存和另存为均保留频道名、链接、签名参数、频道属性与文件头参数。网络条目重开或切换从头连接，不恢复上次时间，不提供列表内下载或缓存。列表内允许未知时长的网络流，进度条不填充、不接受拖动，显示已播放时间，“跳过片头片尾”、重置及相关输入/取值按钮均禁用；已保存的跳过设置保留，切换到已知时长媒体后可继续使用。Prev（上一项）按列表顺序返回前一项，第一项时禁用；音频随机模式下点击 Prev 会清空当前随机队列，下一次 Next 重新开始随机选择。
- 每份列表只占一个页签，最近打开只记录列表；当前媒体信息随条目切换，列表播放器和进度独立于普通媒体页签。视频支持紧密编号网格或名称单列，音乐使用名称单列，面板内纵向滚动，滚动条始终靠右内边缘；侧栏仅占媒体画面区域，上方文件信息和底部时间轴保持全宽；标题顶部增加 8 点留白，管理菜单采用随主题着色的 SVG 扳手图标；左侧内容增加 2 点内边距，创建“保存并使用”按钮底部居中、128×36 点。悬浮箭头展开 / 收起、边界拖动调宽，各列表面板最大宽度统一为窗口宽度的三分之一，最小目标宽度 220 点（小窗口随上限收缩）；全屏浮层默认半屏高、垂直居中，上下边缘拖动保持中心并对称改变高度，按列表记住高度。全屏展开列表使用带细主题边框的半透明浮层覆盖画面；背板 45% 不透明度，控件保持清晰，左上 / 左下 30 点圆角、右侧直角贴窗口边缘，拖宽时右缘固定，不挤占视频可用区域或改变视频缩放，底部时间轴保持全宽；鼠标停止活动 1 秒后隐藏全屏箭头与列表，移动后恢复，拖动或输入时保持可见；收起时 SVG 箭头按钮贴媒体区域右边缘、图标居中，不预留列或缩小适应窗口的宽度。窗口和全屏的展开 / 收起箭头仅在媒体画布内鼠标移动时显示，停止 1 秒或移出区域后隐藏，不改变列表状态。前缀创建时可去掉名称前缀，显示名称写入通用 EXTINF 标题，不修改原文件名。
- 列表播放控制栏使用矢量“下一项”按钮替代停止，窗口 / 全屏一致；手动下一项在单曲循环中仍切换到下一条，随机模式使用随机队列，顺序末项暂停。
- 音频列表在当前文件名及大小后显示当前播放模式，收起列表后仍可见；支持顺序（末项暂停）、随机（每轮不重复，持续下一轮）和单曲循环。视频“跳过片头片尾”按每份列表独立设置非负整数秒（单位固定 `s`）；时间输入框为 60×24 点，输入框 / s / 按钮采用 2 点紧凑间距，框后“片头”按钮取当前播放时间，“片尾”按钮取总时长减当前时间，向下取整到秒，未知时长时禁用片尾按钮；复选框后的“重置”将当前列表两项时间同时恢复为 `0`，不改变开关；到总时长减片尾时切到下一集，最后一集暂停并在画面中央显示“已是最后一集”和确定按钮；无效时间或片头片尾覆盖整集时暂停并提示。
- 列表本地记录当前条目、时间和媒体身份，重新打开恢复；手动 / 自动切换从零或片头结束处播放。播放中约每十秒、暂停、切换、关闭和正常退出时保存；目录扫描和定时记录在后台处理。各列表模式、跳过时间和面板偏好独立记忆；清理缓存只删除列表进度，保留列表设置及用户列表文件。列表改名 / 移动不追踪，缺失条目提示并停下，媒体内容修改不使用旧进度。

- “格式支持”和设置中的“文件关联”可进入同一个默认程序弹窗，按分组查询 Windows 当前默认程序。每项“去设置”复制后缀并打开系统默认应用页面，需粘贴搜索并由用户确认；可复制本程序 EXE 路径；启动时在后台更新当前用户下的程序名称、路径、支持后缀与独立文件图标登记，进入系统设置前也会重试，不自动更改默认程序。复合后缀按最后一段处理，共用后缀可能包含不同内容。
- 音视频打开后在媒体信息就绪时自动播放，有记录时从上次位置继续；重复打开已有媒体页签会继续播放，普通页签切换保持暂停行为。关闭页签、切换页签或正常退出时记忆播放位置。记录按路径、大小及修改时间核对，最多 500 项、2 MiB；文件替换、位置无效或已播放完毕时从头开始，网络下载缓存不记录播放位置。
- 设置中的“文件关联”“软件缓存”依次位于“最近打开记录”上方；缓存项以 KB / MB / GB 显示占用并提供文件管理器查看及后台清理，不显示固定说明段落。保留设置、最近文件及页签恢复信息；清除播放记录、可重建项目索引及未使用的网络 / 更新下载。打开中的文件、待安装更新、更新失败备份及未知文件保留，目录链接不跟随；占用或权限错误会提示。下载 / 索引扫描时禁用清理，缓存使用后可重新生成；不清理外部 FFmpeg 或用户资源。

- 音频信息显示采样率、位深、声道和音频流码率；视频的音频信息也显示当前所选音轨码率，切换音轨同步更新（kb/s）；元数据缺失时显示未知，不使用容器总码率替代。波形与底部控制栏保持紧凑间距。
- 音轨菜单读取标题（缺失时读取 handler_name）；常见语言名称与代码按界面语言显示，包含英语、国语、粤语、日语等 18 类语言；未覆盖的标记保留原文，泛中文不推定方言，悬浮查看原始名称和语言标记，便于区分多音轨；不识别实际语音或自动纠正文件标记。

- 工具栏提供可收起的放大镜查找入口，窄窗口自动换行；收起时显示拖入文件提示及格式支持按钮，可打开后缀列表；最近打开支持单项删除及非空时全部清空；删除长名称后框体宽度随列表调整，矢量减号保持右侧对齐。浅色音频电平峰值线使用深灰色配色；悬浮文件页签名称显示完整路径；过长页签名称中间省略，保留末尾两个字符及扩展名。
- 文件菜单的“打开 URL”支持 HTTP / HTTPS 文件直链，后台下载后打开，最大 1 GiB、总超时 5 分钟。返回 HTML/XHTML 网页时先扫描源码中的 HLS 地址（最多读取 3 MB，去重后最多 300 条）；单条直接播放，多条在弹窗下方逐行显示，长地址截断、悬停显示完整地址、右键复制，右侧“打开”按钮播放所选地址并携带来源页 Referer。该扫描不执行网页脚本或监听浏览器请求，不递归外部脚本/iframe，候选地址未预先验证可播放。未发现 HLS，或内置加载无法识别/打开有效文件（含未知类型与 HTTP 请求失败）时，不按站点区分，自动转交 yt-dlp 分析一次，成功后播放首个视频格式及其配套首条音轨，无需先下载或再次点击播放；沿用视频提取中已选择的会话设置。无可用视频、缺少组件或站点访问失败时直接在提取窗口显示相应错误，可调整后重试；不会无限重试或再跳回文件分析。提取已有任务时保留原任务并提示稍后重试；关闭提取窗口取消此次自动播放。普通文件缓存在软件数据目录。最近打开记录原 URL，点击可重新下载打开。网络页签不自动恢复，可在设置中清理未使用的下载缓存。
- 视频直链和具有已知时长的 HLS 点播直接在播放页打开。HLS 支持 HTTP / HTTPS 的 `.m3u8` 及带 `url` 参数的 ZyPlayer 链接，保留签名参数。最近打开可按原链接重开。网络页签不自动恢复。
- 播放页按可用格式显示清晰度和音频选项；同语言的不同编码、码率以及未标注语言的音频均保留可选。播放器只在点击“缓存到本地”后启动后台缓存。缓存时用带线框的“缓存中…51%”文字代替进度条，后接取消按钮且控件组不拆行；处理阶段和取消中显示对应状态。缓存过程中可取消，显示“缓存完毕”后会在原页签切换到本地并隐藏缓存按钮。切换到未缓存的格式会重新显示按钮。分离的画面和音轨分别保存，切换音频时复用同一清晰度的本地画面，缺少的音轨直接在线播放。单路视频和音频无需选择。当前格式已有本地缓存时，播放页显示带下划线的“视频已缓存”链接；分离音频另有“音频已缓存”链接，两个链接放在播放工具栏缩放比例之后，同排显示、空间不足换行。悬停显示完整缓存路径，点击定位所在目录，页面不常驻显示地址。后台缓存完成任务左键定位实际视频缓存文件，右键可返回播放页。缓存被清理后任务显示失效路径；分离视频缓存可能仅含画面，并不等于已合并的成品。
- “打开 URL”和“视频提取”在键盘或右键粘贴时，会将字面的反斜线 u0026 等 Unicode 转义还原一层；保留 %26 等百分号编码、HTML 实体及无效/控制字符转义，不改动手动输入。“文件 → 视频提取”可粘贴网页链接或分享文字。分析后选择格式，点“播放”直接进入同一播放页，优先使用匹配的已下载成品或缓存。蓝色“下载视频”仍由你指定保存路径，按格式输出 MP4、WebM 或 MKV。命中缓存的轨道可直接用于合成，只下载缺失的轨道。视频提取需要 yt-dlp 和 FFmpeg，站点可用格式取决于提取器和访问条件。部分站点格式只能通过下载后播放。暂不支持直播或 DRM。
- 主工具栏设置齿轮左侧的后台任务图标可展开后台任务中心。蓝点表示任务运行中。收到新结果时橙点轻闪两次后常亮，打开面板即清除提醒。同一轮闪烁内的新结果共用一次提醒。面板显示视频缓存、视频提取下载和音视频编辑导出，可取消支持取消的任务，完成后可返回播放页或在资源管理器中定位输出。点击面板外或按 Esc 收起。记录仅在本次运行期间保留。
- Windows 可在“第三方支持 → 网页浏览器HLS嗅探助手”查看安装步骤：点击“安装油猴插件”打开 [Tampermonkey 官网](https://www.tampermonkey.net/) 安装浏览器插件，再点击“安装脚本”打开 [Greasy Fork 脚本页面](https://greasyfork.org/zh-CN/scripts/604041-unityyamlviewer-hls-%E5%97%85%E6%8E%A2%E5%8A%A9%E6%89%8B) 安装；也可使用本地 `scripts/Tampermonkey/UnityYAMLViewer-HLS/UnityYAMLViewer-HLS.user.js`。安装在浏览器完成，软件不检测扩展或脚本安装状态。先正常运行支持该功能的新版软件一次以注册当前用户的 `unityyamlviewer:` 协议，再在 Tampermonkey 中安装脚本。网页面板提供复制地址和“在 UnityYamlViewer 中打开”，浏览器可能要求允许外部应用。已运行时转交当前实例，未运行时启动；仅传媒体地址及来源 Referer，不传 Cookie、不下载/解密视频。登录/DRM/额外请求头、跨域帧/Worker/脚本隔离及长协议地址可能受限。移动程序后重新启动更新注册路径；安装和限制详见脚本同目录 README。
- 网站要求会话时，可开启“使用浏览器会话”并选择浏览器，再重新分析。Windows 登记的浏览器仅作参考。QQ 浏览器等无法直接读取的浏览器可选择只包含目标站点 Cookie 的 Netscape 文本文件。软件使用临时副本，保留原文件。
- YouTube 提取需要 Deno 2.3+ 或 Node.js 22+，可加入 PATH 或放在 yt-dlp 同目录。运行时不会自动安装。遇到“登录验证”时，先在浏览器确认视频可播放，再主动选择“使用浏览器会话”或目标网站的 Cookie 文件并重新分析；安装运行时不能替代登录验证，也不保证解除站点限制。
- RTP／RTSP 地址目前通过播放列表打开，RTP 必须有接收端口；标准 IPv6 地址须放在方括号内，例如 `rtp://[ff15::1234]:5140`、`rtsp://[2001:db8::1]/channel.smil`（仅为格式示例）。组播使用系统默认网络接口，需要线路、路由及防火墙允许对应频道数据；不自动选择网卡或改变网络配置。RTSP 优先尝试 TCP，允许 UDP/组播协商；RTP 使用 UDP 接收。媒体信息读取总超时约 20 秒，超时可重试或切换频道；实际接收和解码取决于 FFmpeg 与源站。需要额外 SDP 描述的动态 RTP 不在本次支持范围。源文件中已经写成省略号的地址不能还原；不要把这些占位地址当作完整频道链接。页签与列表标题还原下载缓存文件名中的一层 URL 编码，实际缓存路径、原始 URL 和签名参数不变；名称列表左对齐。
- 在视频画面右键菜单选择“比例 → 原始 / 16:9 / 4:3”。原始沿用源像素尺寸比例，16:9 / 4:3 只调整显示矩形，不重新解码或改写媒体。先确定显示比例，再叠加“适应窗口 / 原尺寸 / 自由缩放”；更改比例不改变已有缩放模式，自由缩放的倍数与平移保留。工具栏顺序为解码方式、三个缩放选项、比例状态、镜像状态、缩放百分比。窗口与全屏共用选择；同一列表切换频道或网络视频切换格式时保留比例，关闭并重开恢复原始，不写入编辑工程或导出结果。
- 网络播放底部在播放时间右侧显示媒体接收速率；电视／电台列表按“网速 → 源 → 全屏”排列，例如 `↓ 300 KB/s`、`↓ 1.0 MB/s`，窗口与全屏一致。数字取当前音视频接收进程最近 3 秒读取的编码媒体包大小之和，使用十进制单位（1 MB = 1000 KB），不是解码后的画面/PCM 数据量或标称码率，也不是整机网卡流量；不含协议、分片封装等开销。独立音视频进程可能重复读取同一源，统计会合计这些读取；缓冲、解码消费节奏及暂停都会影响数值，不能单凭低值断定线路故障。开始尚无统计时显示 `— KB/s`，停止读入后约 3 秒降为 `0 KB/s`，切换或重建播放会话重新计数；停止后隐藏。本地文件、已切到本地缓存的播放、媒体探测或下载任务不显示此播放速率。只在内存读取 FFmpeg 包长诊断，不保存诊断日志或媒体包内容。
- 含电视／电台属性的网络 M3U 自动使用频道视图：按 group-title 分组，按原文件顺序排列平铺的可折叠分组条并显示频道数，条宽跟随侧栏、文字居中，不使用树形缩进；默认全部折叠，关闭重开恢复折叠。展开后为固定宽度的台标卡片，上方台标、下方名称，每排数量按侧栏完整可用宽度计算，逐行对齐，不因文字占位而错排。同一个非空 tvg-id 合并为频道；缺少 ID 时按非空 tvg-name、再按末尾名称精确匹配，不做模糊匹配。同名但不同 ID 的频道不会混合；同 ID 跨组时在各所属组都可找到，完全相同的源地址在切换菜单中只列一次。名字仍使用参数后的名称。“更新时间 + 日期”记录单独显示为信息，保存时保留。右下角“源1”展开切换当前频道的源，悬停可查看地址；Prev/Next 切换频道，不在备用源间跳转。正在读取或播放失败时仍可换源。频道视图不显示“编号/名称”或片头片尾设置；编辑时使用原始条目列表逐条管理。
- 台标只在展开且可见时后台加载，支持常见位图格式，失败时保留频道名称；不自动读取 EPG。台标本地缓存位于软件数据目录，可通过设置中的缓存清理删除，之后按需重新获取。每个地址缓存缩略 PNG，7 天后再次加载时尝试更新，网络失败可继续使用旧副本；最多 1024 张、32 MiB，内存最多 256 张，每列表最多 4 个并行加载。单次下载最多 1 MiB、8 秒，超大图片或不支持格式使用占位符；最多尝试该频道的前三个不同台标地址。
- 管理菜单的“另存为”保存 M3U/M3U8 副本，重新计算本地媒体相对路径，并保留全部源、频道属性与文件头（包括 EPG 地址，但不下载节目单）。已存在目标须确认覆盖，等待期间被修改则中止。成功后显示路径，可点“打开列表”；副本不会自动覆盖原列表或清除原列表未保存的编辑。原有“保存列表”仍保存当前文件。
- 同一列表记住最近观看的频道和源，约每 10 秒及暂停、切换、关闭时保存；重开先按频道 ID 与源地址恢复，地址改变时按原源序号回退，已移除的频道或源回退到现有条目。直播重新连接当前内容，不恢复历史时间。URL 下载的列表通过原始地址散列识别，同一 URL 的缓存目录变化不影响新记录；查询参数不同视为不同列表，本地文件移动/改名不自动追踪。另存为副本复制当前选择，之后各文件独立保存；清理播放进度或异常退出可能失去最近的选择。
- 电视台模式下，上/下方向键调音量，左/右切换上一/下一频道，Ctrl+左/右循环切换当前频道的源（单源时无效）；这些切台/切源操作在读取中或报错时仍可用。频道到达首尾不循环，源才循环。一次按下切换一次，长按不连续启动频道；输入文字或菜单打开时不抢方向键。普通文件左右键仍用于快进/快退。切台不重置分组展开与滚动位置，频道列表在加载和出错期间保留。
- 第三方支持的 yt-dlp 项可以手动检查最新稳定版。由软件安装的副本可以在发现新版后直接更新。手动选择或从系统路径检测到的副本沿用原安装方式更新。

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
- 音频 / 视频播放：播放、暂停、停止、进度跳转、音量及空格键控制；音量弹窗仅显示竖直滑条，不显示百分比数字。上/下方向键调音量时居中显示“音量:70%”等百分比，长按期间保持，松键后 0.2 秒消失。视频支持音轨选择、暂停方向键逐帧及自动 / 软解 / 硬解选项。
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

Windows 构建时直接将 `assets/app_icon_<尺寸>.png` 的各档图标嵌入 EXE，不重新缩放。

### 待支持与当前限制

- 节点画布目前用于源码驱动的展示和临时输入预览；新增 Property、编辑 / 保存连线、生成 Shader 及完整替代 Unity Shader Graph 尚未实现；圆角立方体、自定义预览模型导入与 HDRI / PBR 环境尚未实现。当前 3D 仅提供凸形预置网格的正交预览，原始三角形按视深排序，复杂顶点位移、自交 / 透明相交与完整场景深度仍有限制；不为无光照 Shader 额外伪造光照。组件用途判断只覆盖可识别的直接访问、赋值和部分颜色运算；函数间数据流、复杂宏和子图不保证识别，无法确定时保留通用参数连接或 XYZW。

- 播放列表支持本地媒体、本地 file URI 及 HTTP / HTTPS、RTP、RTSP 网络媒体；每份列表最多 10,000 项、8 MiB。HLS 分片清单本身不能作为频道集合列表读取；FTP 等其他协议、嵌套列表、EPG 节目时刻表及列表请求头/Cookie 配置暂不支持。无效或不支持的网络条目仍保留在列表中并以错误颜色和悬停提示标明，选择时单独报错，不阻止其他条目加载；保存不会静默丢弃这些原始地址。网络频道是否可播放取决于源站和 FFmpeg，不保证持续可用，也不支持直播回看。相对本地路径依赖列表与媒体的相对位置；随机顺序不跨程序重启保存，崩溃 / 强制退出不保证最后几秒本地播放进度。

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

- Vert steps also inspect scalar/vector values, input/output struct fields and components. Values captured at the selected statement pass through a free varying and appear interpolated across the surface. The remaining original Vert code and final position still run, preserving the plane / 3D geometry. The output is labeled as interpolated vertex values, not per-pixel recomputation or a raw vertex-value table. Whole-struct results, multiple / early returns, unavailable varying channels and unresolved types are unsupported.


- Frag statements become numbered step nodes with result ports. Select a step to preview its assigned scalar, vector or component; switch to A or final output from the toolbar. The original shader prefix runs with its sampling, UV operations, calls and time. Scalars use grayscale, 2D uses RG, 3D/4D uses RGB and A is separate grayscale. Diagnostic values render opaque and bypass Blend with Render State controls disabled; final output restores temporary state overrides.
- Step inspection currently supports straight-line Frag assignments / variable returns, up to 32 statements per function. Unknown types, uninitialized declarations, branching / loop prefixes and non-scalar/vector values report limitations. Complete arithmetic-node reconstruction, connection editing remain unsupported. Values outside 0–1 clip to the display range without automatic normalization; A inspection requires scalar or four-component results.


- Node numeric text commits on Enter or focus loss. Dragging numbers / sliders updates their displayed values, then refreshes rendering on left-button release. File- and property-specific widget identities keep background status and thumbnail changes from interrupting editing.
- Frag → Render State → Output: a draggable module temporarily overrides Cull Off / Back and source / destination Blend factors, with a reset to Shader state. Blend Off equals One / Zero. Lighting, ZWrite and ZTest display source states; fixed-function lighting, full depth rendering and Cull Front are unsupported. No files are written.


- Material-node interactions follow the requested Amplify Shader Editor reference: edit numbers, colors, textures and individual components inside each module, with a reset action on every Property node and no separate node-parameter Inspector. The graph retains a subtle grid, compact themed nodes, type accents, thin connections and orange selection outlines. The live output is inside a draggable graph module with a bottom-right resize grip and a Locate output action. 2D shows the plane with checkerboard or solid background; 3D offers real sphere, cylinder and cube meshes, drag rotation and rotation reset, reusing the current material values and textures. Detailed preview notes are available from a menu.
- Color Properties edit RGB and the separate A row independently. Zero alpha retains RGB, and displaying a node does not alter its color parameters, including when a shader uses RGB and alpha separately.
- Preview panels and the Inspector share available window width proportionally, preserving a new split after dragging the divider and reserving space for both panes in narrow windows. Material-node wires use horizontal endpoint tangents with handles based on distance and graph zoom. The Frag output port stays near the top of its group so source length does not change its connection height.
- Windows application icons include 16 / 20 / 24 / 32 / 40 / 48 / 64 / 72 / 80 / 96 / 128 / 192 / 256 px. Each size uses its matching PNG in assets directly, without resizing or color conversion during packaging.
- With material / shader node preview open, drop one local PNG, JPG, JPEG, TGA, BMP or WebP image onto a visible 2D texture Property module to replace its preview texture, retaining Tiling / Offset without writing files. Other targets and multiple-file drops open files normally. The source must supply a local file path; Unity drag support depends on it providing that path. Unsupported snapshot dimensions bypass the cache and keep direct rendering; zero-pixel clipped regions skip the current frame.

- Materials (`.mat`) and shaders (`.shader`) have a node-based UI preview and use the shared Preview button to expand or collapse a separate resizable node panel, following the automatic-preview setting. The Inspector keeps material properties/YAML, and shader source remains available separately. Property nodes have type accents, ports, curved connections and a graph grid; drag headers to reposition nodes. Edit inputs directly inside nodes, with each RGBA / XYZW component on its own row. Color metadata and identifiable component/color usage determine the default presentation, independently of float4/half4 precision; RGBA/XYZW presentation can be switched. Recognizable RGB/alpha/component accesses and assignments have corresponding connections; unused properties remain unconnected. Materials load their saved inputs; shaders load Properties defaults. Temporarily adjust numbers, colors and vectors, replace 2D textures, edit texture tiling/offset, or reset inputs without writing files. The canvas pans and zooms; connections represent input bindings, with separate Vert, Frag and helper boundary interfaces and operations and resizable explicit-membership blocks. Frag statements use selectable step nodes for intermediate-value previews; Vert also supports step nodes; helpers also use boundary ports and internal operations. Common arithmetic expressions use operation nodes; complete HLSL reconstruction is unsupported, and identifiable calls are labeled separately. Vertex Input and v2f / Varyings interfaces list their fields, types and semantics independently. TEXCOORD1/2/3 interpolators need not be mesh UV1/2/3, and showing a declaration does not imply renderer support. Scene lighting, masks and custom vertex streams are excluded; integer uniforms, vector uniforms other than four components, and mirrored texture sampling are unsupported. A limited Built-in single-CG-pass subset has GPU plane and preset-model previews. URP/HDRP, multiple passes, complex macros, unresolved references and non-2D textures may prevent rendering; the reason is shown. Materials require a resolvable shader GUID and project resource index. Missing material values use shader defaults; reinitializing the preview clears temporary overrides, while file-level reload rereads the current file.

- Native app dialogs center on the main window when opened or reactivated and request visibility, restoration and focus. Clicking Settings/About again reactivates the existing dialog. Manual movement remains available after placement settles. Long preview-toggle text wraps inside the right Settings column without covering the left label.

- `.m3u8` / `.m3u` audio/video playlists, saved as UTF-8 M3U8 by default. Create from all matching media in a folder or a filename prefix in the current media's side panel, without recursion and with natural sorting. Preview, exclude and reorder entries before saving; edit the name and confirm overwrites. Paths are relative to the playlist where possible, absolute across drives. M3U accepts UTF-8, UTF-16 BOM and the current Windows ANSI encoding.
- Playlists can mix local media and HTTP / HTTPS, RTP and RTSP channels, using the display name after the `#EXTINF` parameter block and the URL on the following line. This also handles lists with an extra comma between duration and attributes; group, logo and other attributes are retained, while tvg-id/tvg-name do not replace the trailing display name. New online lists default to names. Open URL downloads `.m3u` lists without changing them to `.txt`. Selecting a channel reuses the FFmpeg player and connects only to that entry; Save and Save As preserve titles, URLs, signing parameters, channel attributes and header parameters. Network entries reconnect from the beginning without resuming a saved time, and have no playlist download/cache controls. Network streams of unknown duration are allowed within playlists: the progress bar has no fill or seeking, elapsed time is shown, and all intro/outro controls are disabled. Saved skip settings remain available for media with a known duration. Previous selects the preceding entry in list order and is disabled on the first item; in audio shuffle mode it clears the current shuffle queue, so Next starts a new random selection.
- A playlist occupies one tab and is the only item added to recent history. Its current media information changes with selection; its player and progress are independent of ordinary media tabs. Video uses a compact number grid or a single name column; audio uses a name column, with vertical scrolling and a scrollbar fixed to the inner right edge. The windowed playlist occupies only the media canvas; metadata and transport timeline retain full width. The header has 8 points of top padding and a theme-tinted SVG wrench for its management menu. Content has 2 points of left padding, and the 128×36 Save and use button is centered at the bottom. A floating arrow toggles the resizable sidebar. All playlist panels are capped at one third of the window width, with a target minimum of 220 points (reduced for small windows). The fullscreen overlay defaults to half the screen height, stays vertically centered and resizes symmetrically from its top or bottom edge; height is remembered per playlist. In fullscreen, the playlist overlays the video with a 45%-opaque backplate and clear controls, a thin theme border, 30-point left corners and square right corners pinned to the window edge during resize, preserving the video area, scaling and full-width transport; fullscreen arrows and the playlist hide after one second of inactivity and return on activity, remaining visible during dragging or text input; when collapsed, a centered SVG arrow floats at the right edge of the media region without reserving a column or reducing fit-to-window width. In windowed and fullscreen modes, expand/collapse arrows appear only on mouse movement within the media canvas, hiding after one second or when the pointer leaves without changing playlist state. Prefix matching can strip that prefix from display titles, exported as standard EXTINF titles without renaming media files.
- Playlist transport replaces Stop with a vector Next button in windowed and fullscreen modes. Manual Next advances even in repeat-one mode, uses the shuffle queue in shuffle mode, and pauses at the end of a sequential list.
- Audio playlists show the current playback mode after the filename and size even when collapsed; they support sequential playback (pause at the end), shuffle (no repeats within each round, continuing into new rounds) and repeat one. Skip intro and outro stores times separately for each playlist as nonnegative integer seconds with the fixed unit `s`. Time inputs use 60×24 points with 2-point spacing between the input, s and button. The Intro button uses the current position; Outro uses total duration minus current position, rounded down to seconds. Outro is disabled when duration is unknown; Reset next to the checkbox sets both times to `0` for the current playlist without changing the checkbox; reaching duration minus outro advances to the next episode. The last episode pauses with a centered message and OK button. Invalid times or skips covering the entire episode pause with a warning.
- Local playlist state remembers the current entry, time and media identity for resume. Manual or automatic entry changes start at zero or after the intro. Background records save roughly every ten seconds during playback and on pause, switching, close and normal exit. Folder scans run in the background. Modes, skips and sidebar preferences are independent per playlist; cache cleanup removes playlist progress while keeping settings and user playlist documents. Playlist moves/renames are not tracked; missing entries pause with an error and changed media does not reuse stale progress.

- Supported formats and Settings → File associations open the same default-app dialog, with grouped Windows association queries. Go to settings copies an extension and opens Default apps; paste it into search and confirm the choice in Windows. The EXE path can also be copied. On startup, the app updates its name, path, supported extensions and separate file icon registration for the current user in the background, and retries before opening Settings; default choices are not changed automatically. Compound suffixes use the final extension; shared extensions may contain different content.
- Audio/video plays automatically once metadata is ready, resuming the last saved position. Opening an existing media tab resumes playback; ordinary tab switching keeps it paused. Positions are saved when closing or switching tabs and on normal exit. Records are checked against the path, file size and modification time, with limits of 500 entries and 2 MiB. Replaced files, invalid positions and completed playback restart at zero; downloaded network-cache files have no playback records.
- File associations and App cache appear above Recent file history in Settings. The cache section shows usage in KB / MB / GB, opens its folders and cleans disposable files in the background, without permanent explanation paragraphs. Settings, recent files and tab restore data stay; playback records, rebuildable project indexes and unused network/update downloads are removed. Open files, pending updates, failed-update backups and unknown files stay; directory links are not followed and access errors are reported. Cleanup is disabled during downloads/index scans, and caches may regenerate during use. External FFmpeg installations and user assets are excluded.

- Audio information shows sample rate, bit depth, channels and audio-stream bitrate (kb/s); missing bitrate is shown as unknown rather than replaced with container bitrate. Video audio information also shows the selected track bitrate and updates on track changes. The waveform sits close to the transport bar.
- Audio track menus read metadata titles (falling back to handler_name), localize 18 common language categories including English, Mandarin, Cantonese and Japanese (unknown tags remain unchanged; generic Chinese tags do not imply a dialect), and show original names and language tags on hover; no speech identification or automatic metadata correction.

- A collapsible magnifying-glass search control moves to a second row in narrow windows; when collapsed, a drag-and-drop hint and Supported formats button open an extension list; recent files can be removed individually or cleared when nonempty; deleting a long name also updates the menu width, keeping vector minus buttons aligned at the right. Audio peak lines use dark gray in light mode; hovering over a file tab name shows its full path. Long tab names use middle ellipsis, retaining the last two stem characters and extension.
- Open URL in the File menu opens direct HTTP / HTTPS file URLs after a background download (up to 1 GiB, 5-minute total timeout). Regular downloads use the app data directory. Recent Files stores the original URL so it can be opened again. Network tabs are excluded from session restore. Clear unused downloads in Settings.
- Open URL also plays HLS on-demand streams with a known duration. Paste a direct HTTP / HTTPS `.m3u8` link or a ZyPlayer web link with a `url` parameter. The app preserves signing parameters. The tab uses the link’s `name`, an embedded title recognized by FFmpeg, or the source filename, in that order. Unencrypted TS streams prefetch segments in the background, including while paused. The light green line shows the end of the furthest cached segment. There may be gaps before that point. Playback uses the original URL until all segments are cached. It switches to local segments the next time you start, resume, or seek. Recent Files retains the stream URL and its signing parameters. Network tabs are excluded from session restore. Live streams, general web-page parsing, login, and HLS editing are unsupported.
- Select “Download MP4” in the HLS playback tab and choose the destination and filename. The app reuses playback segments and waits for missing segments before merging. It retries each missing segment up to 20 times and reports an incomplete download if one remains unavailable. The playback cache is cleared after the tab closes. A completion dialog shows where the file was saved. Downloads can be cancelled and existing files are preserved. The first video and audio tracks, when present, must be compatible with MP4.
- In File → Video extraction, paste a web link or share text. Analyze it, then choose a video format and a separate audio track when available. Download preserves MP4 or WebM for matching tracks and uses MKV for other combinations. Play prepares a cached file before opening it in a playback tab. Install yt-dlp and FFmpeg in Third-party Support or select their paths manually. The extractor handles a single video and takes the first item from multi-video links. Available formats depend on the site's extractor and access conditions. Live streams and DRM are unsupported.
- When a site requires a session, enable “Use browser session”, choose a browser, and analyze again. The Windows browser list shows registered browsers for reference. For a browser yt-dlp cannot read directly, such as QQ Browser, select a Netscape-format text file containing only the target site's cookies. The app uses a temporary copy and preserves the original.
- YouTube extraction requires Deno 2.3+ or Node.js 22+, available in PATH or beside yt-dlp. Runtimes are not installed automatically. When sign-in verification is required, first confirm the video plays in your browser, then explicitly select “Use browser session” or a cookie file for the target site and analyze again. Installing a runtime does not replace sign-in verification or guarantee access.
- Open RTP/RTSP addresses through playlists; RTP requires an explicit reception port. Put IPv6 literals in brackets, for example `rtp://[ff15::1234]:5140` or `rtsp://[2001:db8::1]/channel.smil` (format examples only). Multicast uses the default system interface and needs the network, routing and firewall to carry the stream; the app does not select an interface or alter network settings. RTSP prefers TCP while allowing UDP/multicast negotiation; RTP receives via UDP. Media probing has a total timeout of about 20 seconds; retry or choose another channel if it fails. Reception and decoding depend on FFmpeg and the source. Dynamic RTP requiring a separate SDP description is outside this change. URLs already abbreviated with ellipses in the source cannot be reconstructed. Tab and playlist titles decode one layer of URL encoding in cached filenames without changing cache paths, source URLs or signatures; name rows are left aligned.
- Right-click the video and choose Ratio → Original / 16:9 / 4:3. Original keeps the source pixel-dimension ratio; the overrides change only the display rectangle without re-decoding or modifying media. The display ratio is applied before Fit to window / Native size / Free zoom; changing it retains the current scaling mode, free-zoom factor and pan. Toolbar order is decoder, the three scaling options, ratio status, mirror status and zoom percentage. Windowed and fullscreen views share the selection, which survives channel or network-format changes in the current session. Closing and reopening resets it to Original; it is not written into editing projects or exports.
- Network playback shows media receive rate to the right of playback time; TV/radio lists use the order receive rate → source → fullscreen, for example `↓ 300 KB/s` or `↓ 1.0 MB/s`, in windowed and fullscreen views. It sums encoded input packet sizes read by the current audio/video receivers over the last 3 seconds, using decimal units (1 MB = 1000 KB). It is not decoded frame/PCM output, declared bitrate or total interface traffic, and excludes protocol and segment-container overhead. Separate receivers may read the same source twice, and those reads are summed. Buffering, decoder consumption and pause affect the number, so a low value alone does not prove a network fault. It starts at `— KB/s`, reaches `0 KB/s` about 3 seconds after reads stop, resets for each new playback session and hides when stopped. Local files, playback switched to local cache, probes and download tasks do not use this indicator. FFmpeg packet-length diagnostics are consumed in memory without saving diagnostic logs or packet payloads.
- Network M3U lists with TV/radio attributes automatically use channel view. Flat collapsible group-title bars follow file order and show channel counts, with centered text and full sidebar width, without tree indentation; all groups start collapsed and reset on closing and reopening. Expanded groups show fixed-width logo cards with names below, using the full sidebar width to calculate aligned rows as the width changes. Equal nonempty tvg-id values merge; without an ID, exact tvg-name and then trailing names are used, without fuzzy matching. Different IDs remain separate even if names match. Channels can belong to several groups; identical source URLs appear once in the source menu. Trailing names remain display names. Dated “更新时间” rows are shown as information and retained on save. The bottom-right Source menu changes sources within a channel and shows URLs on hover. Previous/Next change channels, not backup sources; source switching remains available during probing or errors. Channel view hides number/name and intro/outro controls; editing uses original entries.
- Logos load in the background only when visible in expanded groups. Common bitmap formats are supported; failures retain channel names and EPG is not fetched. Local cache is included in Settings cache cleanup and rebuilt on demand. Each URL stores a thumbnail PNG; after 7 days, a new load attempts refresh, retaining stale content if the network fails. Limits are 1024 files/32 MiB on disk, 256 images in memory and 4 simultaneous loads per list; each download is limited to 1 MiB and 8 seconds. Unsupported or oversized images use a placeholder; at most three distinct logo URLs are tried per channel.
- Save As writes an M3U/M3U8 copy, recalculates relative local paths and preserves all sources, attributes and the header, including EPG addresses without fetching schedules. Existing targets require overwrite confirmation; external changes while awaiting confirmation abort the save. The saved path has an Open list action. Saving a copy does not overwrite the original automatically or clear its unsaved edits. Save list still writes the current file.
- Each list remembers the last channel and source, saving about every ten seconds and on pause, switching and close. Reopening first matches channel ID and source URL, then falls back to the saved source number if URLs changed; missing channels/sources fall back to an existing entry. Live streams reconnect to current content without restoring an old time. Downloaded lists use a hash of the original URL, so new cache directories retain new selection records. Distinct query parameters create distinct list identities; local file moves/renames are not tracked. Save As copies the current selection, then the files save independently. Clearing playback progress or abnormal exits may lose recent selections.
- In TV mode, Up/Down controls volume, Left/Right selects the previous/next channel, and Ctrl+Left/Right cycles the current channel's sources (no action with one source). Channel/source switching also works during loading or errors. Channels do not wrap at the ends; sources do. Each press switches once without launching repeated channel requests when held. Text input and open menus retain their arrow keys. Ordinary files still use Left/Right for seeking. Channel changes preserve group expansion and scroll position, and the sidebar remains available while loading or after errors.
- The yt-dlp section in Third-party Support checks the latest stable release on request. You can apply updates to copies installed by the app. Update copies found on the system path or selected manually through their original installation method.

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
- Switching tabs preserves staged edits for each file. Save or discard before closing an editing tab or exiting the application. Unsaved edits and undo histories are not restored after restarting. Saving checks for external changes and replaces the asset using a temporary file in the same directory. Failed writes preserve the original.
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

Windows builds embed each matching `assets/app_icon_<size>.png` directly into the executable without resizing.

### Pending support and current limitations

- The graph is a source-driven viewer with temporary input overrides. Creating Property nodes, editing/saving connections, generating shaders and replacing Unity Shader Graph are not implemented. Rounded-cube meshes, custom preview-model import and HDRI/PBR environments are pending. The 3D viewer uses orthographic convex presets with triangles sorted by view depth; complex vertex displacement, self-intersection, intersecting transparency and full scene depth remain limited. Unlit shaders receive no invented lighting. Component inference covers identifiable direct accesses, assignments and some color operations; interprocedural data flow, complex macros and subgraphs may be unresolved, retaining generic parameter connections or XYZW when uncertain.

- Playlists support local media paths, local file URIs and HTTP / HTTPS, RTP and RTSP media, with up to 10,000 entries and 8 MiB per document. HLS segment manifests cannot be read as channel collections. FTP and other protocols, nested playlists, EPG programme schedules and playlist header/cookie configuration are unsupported. Invalid or unsupported network entries remain visible with an error color and tooltip, report errors individually when selected, and are preserved when saving; they do not block other entries from loading. Online channels depend on the source and FFmpeg, are not guaranteed to remain available, and have no live rewind. Relative local paths depend on the folder relationship. Shuffle order does not persist across restarts, and crashes/forced exits may lose the most recent seconds of local playback progress.

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

This project is licensed under the [MIT License](../LICENSE).

GPU preview accepts scalar and two- to four-component TEXCOORD1/2 vertex inputs with zero values as a temporary fallback. Particle custom vertex streams and their actual data are not yet reproduced.

Quoted Unicode escapes in object names are decoded for display and name searches without rewriting source YAML.

支持单蒙皮网格嵌套 FBX 的静态导入姿态（需唯一根 Transform / Renderer 源 ID），应用根变换、材质与排序；内部骨骼节点、单骨骼覆盖和动画暂未还原。

Nested FBX models with one skinned mesh can display their static imported pose when root Transform and Renderer source IDs are unambiguous. Root transforms, materials and sorting apply; internal bone nodes, per-bone overrides and animations are not reproduced.

静态蒙皮 FBX 按资源轴向转换，并兼容可唯一确认的根骨骼均匀缩放覆盖；其他骨骼覆盖仍未还原。
Static skinned FBX previews convert declared axes and support unambiguous uniform skeleton-root scale overrides; other bone overrides remain unsupported.

Vert / Frag 使用独立 In → 步骤 → Out 节点并按可识别读写依赖连线。颜色处理、纹理颜色采样及 R/G/B/A 来源的单通道运算提供节点内预览。展开且可见时，192×192 预览约每 16.7 毫秒使用当前 Shader 时间更新一次；收起或离屏暂停，返回后继续。复用已准备场景与贴图，每节点最多一个采集任务；多节点实际帧率受 GPU 性能影响。输入提交或手动刷新会重建预览。普通非颜色计算仍仅显示计算与连线。可独立拖动步骤；统一分组块支持拉伸及整体移动，只有连接到阶段 In 且明确加入的节点随框移动，几何覆盖不会自动加入。

Vert / Frag use separate In → step → Out nodes linked by identifiable read/write dependencies. Color operations, texture samples and operations derived from R/G/B/A channels have inline previews. Expanded, visible 192×192 previews refresh with current shader time about every 16.7 ms; collapsed or offscreen previews pause and resume when visible. Prepared scenes and textures are reused, with at most one capture in flight per node; actual rates depend on GPU load. Input commits or manual refresh rebuild the previews. Ordinary non-color calculations have no inline preview. Steps move independently. Resizable group blocks move only explicitly joined nodes connected to stage In; overlap never joins a node.


普通函数同样使用 In → 独立语句步骤 → Out；调用语句连接函数 In，函数 Out 返回调用步骤，函数范围框沿用显式成员规则。可唯一确定的直接 Vert / Frag 调用、数值参数、直线步骤支持使用实际实参生成颜色快照；多调用、嵌套上下文、重载、out / inout、复杂控制流暂只展示拆分，给出预览限制。预览仅改写内存源码；不声称完整语言级数据流还原。

Helpers also use In → independent statement steps → Out. Call steps link to helper In and receive helper Out; group blocks use explicit membership. Color snapshots use actual arguments for a unique direct Vert / Frag call with numeric parameters and straight-line steps. Multiple call sites, nested contexts, overloads, out / inout and complex control flow remain display-only with diagnostics. Instrumentation changes memory only and is not complete language-level dataflow reconstruction.

阶段及函数区域使用各自类型色的淡底色与边界包围 In / 步骤 / Out，随节点位置调整显示范围；该底色区域即唯一分组块，拖动标题栏移动、右下角拉伸，不再绘制独立范围框。函数调用线连接实际调用步骤，不连接虚拟大框锚点。

Stage and helper scopes have tinted backgrounds and borders around In / steps / Out, following their node positions. These tinted boundaries are the single group blocks: drag the title to move, resize from the lower-right corner; no duplicate region is drawn. Calls connect to their actual statement steps, not invisible scope anchors.


分组块统一交互：初始生成且从 In 连通的步骤作为初始成员；拖动块标题时 In / Out 与成员一起移动，非成员及其他函数的节点保持原位置。步骤可自由拖出块边界；同函数内，拖动松开后按连通性和节点中心是否在块内更新成员，也可通过节点菜单加入 / 移出。拉伸仅改变显示范围，不自动加入覆盖节点，取消重复的范围框开关。

Unified blocks: generated steps connected to In are initial members. Dragging the block title moves In / Out and members, leaving excluded steps and other functions unchanged. Steps can move outside the block. On release, same-function membership follows connectivity and center containment; node menus also join / leave. Resizing changes only the visual bounds and never automatically joins covered nodes. There is no duplicate region toggle.

Out 的返回类型与语句按节点内容宽度自动换行，超出高度裁剪在内容区内，不遮挡连线或邻近节点；悬停可查看完整文本。

Out return types and statements wrap to the content width and clip within the node when too tall. Hover to read the complete text.

节点默认按可识别读写依赖从左到右分层，同层独立步骤上下展开，Out 放在最后一层右侧；Vert / Frag、辅助函数以及 Render State / 最终输出间距按内容宽度计算。输入连线分开落点，分组底色降低强度。已有手动布局保留，点击“重置视图”应用新默认排列。画布滚轮及重置视图最小缩放均为 10%，最大 175%；不保证任意复杂图无交叉，不是完整 ASE 算子重建。

Default nodes are layered left to right by identifiable read/write dependencies, with independent steps stacked within a layer and Out after the last layer. Stage, helper and final-output spacing follows content width. Input endpoints are separated and scope tints reduced. Manual positions remain until Reset View. Wheel zoom and automatic fit allow a 10% minimum, with a 175% maximum. Complex graphs can still cross; this is not full ASE operator reconstruction.

分组连通性包含参与该输入计算链的上游局部常量。例如 avgColor 不直接读取 In，但被已连接的 lerp 使用，也属于初始成员；未使用的常量和无关节点不会仅因覆盖而加入。

Group connectivity also includes local constants feeding the input-connected calculation. For example, avgColor used by a connected lerp is an initial member even without reading In directly. Unused constants and unrelated nodes never join merely by overlap.


### 紧凑运算节点

- 常见直线赋值与返回表达式按括号 / 运算优先级拆解，支持加减乘除、取余、一元运算、分量访问、向量构造，以及 tex2D、lerp、pow、saturate、dot 等调用；复合赋值按原值与右侧表达式建立依赖。小字面量只读显示在输入行，独立常量赋值保留紧凑 Constant 节点；修改预览参数仍通过 Property。
- 基础运算节点默认宽 100 点，纹理采样 / 自定义调用 / 源码回退等保留 200 点，按输入 / 输出行数计算高度；A / B、A / B / T、Tex / UV 等端口分行显示。输出根据当前可识别类型显示 RGB / RGBA 或 XY / XYZ / XYZW 及分量；可直接连接的 RGB / A 等分量使用对应输出端口，不额外堆叠 Swizzle。颜色判断结合 Property、分量访问与可识别命名 / 运算上下文，声明的宽度及隐式广播 / 截断保留为 Convert，位置 / 法线 / UV 等向量不强制视为颜色，推断不完整时不伪造预览。
- 默认收起所有节点快照；可用颜色节点标题右侧带框眼睛按钮展开 / 收起 128 点预览，只有展开且可见时才排队准备并以约 60 帧/秒持续更新；收起或离屏暂停。R/G/B/A 单通道、RG 等颜色分量及其后续运算保留颜色来源并可预览，单通道显示灰度，双通道显示 RG；命名为 alpha / opacity 的数值也可预览。无颜色来源的普通标量、UV 等非颜色结果不显示快照。展开时为同列后续节点让位，收起不强制恢复手动位置；“重置视图”清除位置和展开状态。源码、所属函数 / 原语句与限制说明放在标题悬停详情。
- 颜色子表达式在原语句执行前使用实际 Shader 前缀求值，避免复合赋值后的变量被二次套用；Vert 保留后续原始顶点变换，通过插值显示诊断。自定义函数调用的完整赋值结果沿用语句执行后的探针，避免提前重复调用；内部子调用结果可能暂不预览。Helper 仍受唯一直接阶段调用等已有上下文限制。
- 组件分别写入后读取整个变量时显示显式 Read 状态读取及生产者连接，不虚构一个算术公式来拼接状态。控制流、三元、数组索引、强制类型转换、增减副作用、未知复杂语法及节点 / 深度预算超限保留为“源码语句”，悬停查看原文与原因。不同函数重载 / 多次调用的完整语义还原、节点创建、连线编辑与 Shader 生成尚未实现。

### Compact operation nodes

- Common straight-line assignments and return expressions are decomposed with parentheses and operator precedence preserved: arithmetic, remainder, unary operations, swizzles, vector constructors and calls such as tex2D, lerp, pow, saturate and dot. Compound assignments read the previous value. Small literals appear read-only inline; standalone constant assignments keep compact Constant nodes. Preview values are still edited in Property nodes.
- Basic operation nodes are 100 logical points wide; texture samples, custom calls and source fallbacks retain 200 points and size their height by port rows. Inputs have names such as A / B, A / B / T and Tex / UV. Outputs show RGB / RGBA or XY / XYZ / XYZW and components according to identifiable semantics. Declared widths and implicit broadcasting / truncation are preserved through Convert nodes. Direct RGB / A reads use their matching output ports, avoiding redundant Swizzle nodes. Property metadata and source context guide color inference; positions, normals and UVs are not automatically colors.
- Snapshots start collapsed. A framed eye button expands eligible color previews to 128 points; only expanded, visible previews queue work and refresh at about 60 fps; collapsed or offscreen previews pause. R/G/B/A channels, RG pairs and operations derived from them retain color provenance and support previews: single channels use grayscale and pairs use RG. Numeric alpha / opacity names also enable previews. Ordinary scalars without color provenance and non-color vectors such as UV have no snapshot. Expansion makes room below in the same column; collapse does not reset manual positions. Reset View clears layout and expansion. Hover a title for source, original statement and limitations.
- Color subexpressions execute against the original prefix before their statement, avoiding double application of compound assignments. Vertex probes preserve the remaining geometry transformation and use interpolation. Completed custom-function assignments use after-statement probes to avoid evaluating the call twice; internal subcalls may have no preview. Helper context restrictions still apply.
- Reads after separate component writes use explicit Read nodes with producer links rather than invented reconstruction arithmetic. Control flow, ternaries, indexing, casts, increment/decrement side effects, unknown syntax and budget limits retain source-statement fallbacks. Complete overload / multi-call reconstruction, node creation, connection editing and Shader generation remain unsupported.

默认布局每列按该列最宽节点加 60 点留白计算，避免基础运算收窄后仍留下旧列距；已有手动布局可用“重置视图”重新排列。预览图保持 128 点，不随窄版缩小。

Default columns use their widest node plus 60 points of clearance. Reset View reapplies spacing to existing manual layouts. Preview images remain 128 points.

基础运算收起时宽 100 点，展开颜色快照时至少加宽至 148 点，预览图仍为 128 点；收起恢复窄宽。输入文本在左侧独立裁剪区显示，避免与输出端口文字重叠，悬停查看完整值。

Basic operations are 100 points wide when collapsed and at least 148 points wide with an expanded 128-point color preview. Closing restores the narrow width. Input text clips within its own left column, with complete values on hover.

Vert / Frag / 普通函数统一使用边界接口：左侧按变量名和类型显示输入，直接连入内部运算；右侧显示返回值及可识别分量。单一简单 Return 合并到边界，不再重复显示 In / Out 卡片；多路径返回保留源码节点。右上角带框减号按钮收起，收起后带框方框按钮展开；两种图标直接以矢量线条绘制，按钮提供悬停与按下反馈；可预览的运算使用直接绘制的矢量眼睛图标，节点背景先注册交互，再注册眼睛按钮，避免点击被节点接走；，按钮带常驻边框及悬停 / 按下反馈，激活时高亮。折叠保留接口并隐藏内部运算，展开恢复布局、大小和快照展开状态。拖动标题仍只移动明确成员，拉伸边框时返回端口跟随右边界。重置视图恢复展开。

Vert, Frag and helpers use named, typed boundary inputs wired to internal operations and a return interface on the right. A single simple Return merges into the boundary, replacing separate In / Out cards; multiple return paths retain source nodes. The top-right framed minus button folds the function; a framed square expands it. Both icons use directly drawn vector lines, with hover and press feedback. Previewable operations use a directly drawn vector eye icon; the node background registers interaction before the eye button so the button receives clicks, with persistent borders, hover / press feedback and highlighting while open. Folding keeps its ports and hides operations; expanding restores layout, size and snapshot expansion state. The title moves explicit members, and resizing keeps return ports on the right edge. Reset View restores the expanded view.

### 画布操作与多选

- 顶部按钮栏下显示操作说明。左键点选，空白左拖框选；Shift 点选 / 框选切换命中项，点击空白清空选择。拖动节点标题或非控件主体移动选区，数值、按钮、快照等控件优先处理自身操作。
- 输出预览节点移出屏幕不影响节点拖动或空白框选。下拉菜单展开时滚轮由菜单处理，画布暂停缩放和平移。右键从可见画布内起拖平移，可从节点上方起拖；滚轮以光标为中心缩放 10%～175%。无需切换模式，无 Q/W 快捷键或工具图标。滚轮不继续传给节点数值控件。
- 节点与展开 / 折叠函数组可混选，组选框按可见范围相交判定；展开组拖标题、折叠组可拖主体。组只带动明确成员，组与成员同时选中只移动一次。重置视图清空选择；拖动由画布统一捕获，松开或失焦释放，再次按下重置残留状态。布局仅保存在当前预览，不写回 Shader / Material。

### Canvas interaction and multiple selection

- A hint below the top buttons explains the controls. Left-click selects; left-drag blank space draws a marquee. Shift-click / marquee toggles hits; clicking blank space clears selection. Drag a node title or non-control body to move the selection. Fields, buttons and snapshots retain their own interactions.
- Node dragging and blank-space selection remain active when the output preview is offscreen. Open popups retain wheel input while canvas zoom and pan pause. Right-drag pans starting within the visible canvas, including over nodes. Wheel zoom stays centered on the pointer, from 10% to 175%, and does not also change numeric fields. There are no modes, Q/W shortcuts or tool icons.
- Nodes and expanded / collapsed function groups support mixed selection. Marquees intersect visible bounds. Drag expanded group titles or collapsed card bodies. Groups move explicit members; selecting both a group and its member moves the member once. Reset View clears selection. Drag capture is centralized; release or focus loss clears it, and a new press resets stale state. Layout stays in preview memory without writing Shader / Material files.


### 音视频编辑列表

视频编辑暂停时，左／右方向键后退／前进一帧；时间轴缩放后，Ctrl+左／右方向键每次向对应方向平移五帧，不改变播放位置。

In the video editor, Left/Right steps backward/forward one frame while paused. On a zoomed timeline, Ctrl+Left/Right pans the view by five frames without changing the playback position.

编辑窗口左上角的列表图标用于展开或收起侧栏，旁边的后台任务中心与主窗口共享进度和结果。展开后可点“添加文件…”或拖入同类音视频文件，点击整张卡片切换编辑，按住左上角的双横线手柄拖动排序，右上角叉号用于移除条目。每条独立保留裁切范围、画面处理和导出设置。同一路径在一个列表中保留一项。

- “队列”模式按列表顺序分别导出。“另存为”选择输出目录，自动给同名成品增加编号。列表只有一条时仍使用原来的文件保存对话框。
- “合并”模式在模式 Tab 和文件列表之间提供独立的“合并导出编码设置”，可设置分辨率、帧率、封装、音视频编码及采样参数。每段直接按合并参数编码，画面按指定分辨率等比缩放并居中补黑边，最后按列表顺序拼接，音视频轨道在拼接时直接复制。单条的裁切和音频编辑仍会应用，单条的编码及画面大小在合并导出时由合并设置覆盖。所有单条设置始终可编辑，切回队列后仍生效。音轨按位置对应，缺少的音轨补静音。
- 文本字幕按片段时间顺延后合并，样式按文本字幕输出。图形字幕、GIF 和 PNG 序列可用队列导出，合并模式会提示更换设置。
- 导出确认后可关闭编辑窗口，在后台任务中心查看结果并定位文件。缓存片段在任务结束后清理。任务运行期间暂停软件缓存清理。


## 音视频工程

音频编辑工程使用 `.uyap`，视频编辑工程使用 `.uyvp`。点击蓝色“保存工程”首次选择位置；保存后按钮变为“保存”，后续点击或按 Ctrl+S 更新当前工程。右键按钮可选择“工程另存为…”。保存不会重载编辑窗口或暂停当前试听，标题改为工程文件名。绿色“导出”输出音视频文件，与保存工程分开。

通过文件打开、拖入主窗口或在资源管理器双击工程，先在主窗口页签显示工程摘要，检测到 FFmpeg/FFprobe 后点击中央绿色“进入编辑窗口”恢复编辑窗口；缺少组件时不显示编辑按钮，提供第三方支持入口。同类多个工程页签折叠分组，音频和视频分别成组。工程记录列表顺序、选中条目、队列/合并模式及独立合并设置、每条素材的范围、裁剪/缩放、编码与音轨字幕设置、播放位置。已提交的音频编辑结果随工程保存，恢复后可继续编辑；撤销/重做历史和正在编辑但未确认的参数弹窗不保存。

工程包内含带格式标识和版本号的 UTF-8 JSON，音频编辑数据及临时图片序列转换的视频作为附件保存。原始素材及外加音轨/字幕通常仅记录路径：优先查找工程目录下的相对位置，再查找原绝对路径。缺失素材会列出路径并阻止不完整打开；当前需恢复素材原路径后重试，不提供重新定位向导。保存工程的文件复制在后台进行，可在后台任务中心查看结果。

Windows 启动时为两类工程登记打开方式；无默认应用时关联本程序，保留已有默认程序和系统 UserChoice。系统要求确认默认应用时，可在设置的默认打开程序中选择本程序。

### Audio and video projects

Use `.uyap` for audio projects and `.uyvp` for video projects. Save Project first asks for a destination, then becomes Save; Ctrl+S updates the current project and its context menu offers Save Project As. Saving preserves the live editor and updates its title. Export creates media separately.

Open a project from the file picker, drop it onto the main window, or double-click it in Explorer to show its project tab. After FFmpeg/FFprobe detection, click the central green Open Editor button to resume editing. Missing components hide the edit entry and show a Third-party Support link. Multiple project tabs are grouped separately by audio/video type. Projects restore ordered items, selection, batch/merge mode and separate merge settings, per-item ranges, cropping, scaling, encoding, track/subtitle choices and playback positions. Committed audio edits are embedded; undo/redo history and unconfirmed parameter dialogs are not saved.

The archive contains versioned UTF-8 JSON and necessary edited audio or temporary sequence media. Original media and added tracks/subtitles remain path references, resolved relative to the project first and then by the original absolute path. Missing sources prevent an incomplete restore; restore their paths before reopening. Relinking is not currently provided. Saving runs in the background with results in Background Tasks.

Windows startup registers both project types and assigns this app when no default exists, preserving existing defaults and UserChoice. Confirm the default in Windows Settings when required.

音视频编辑起始/结束旁的图标是独立锁定开关，红色表示已锁定，对应输入、边界拖动及点击设置被禁用。解锁后恢复，重置范围只修改未锁定的一端；播放指针和播放跳转仍可使用。工程会保存锁定状态。

The icons beside Start/End independently lock those boundaries. Red means locked: the matching input, boundary dragging and click-to-set are disabled. Unlock to resume editing; Reset Range changes only unlocked boundaries. Playback and seeking remain available. Projects preserve both locks.

音视频编辑列表的复选框决定条目是否参与队列/合并导出，默认全部勾选，取消勾选不删除素材或设置。右键条目可移除，至少保留一项；工程保存勾选状态。

Audio/video list checkboxes control inclusion in queue and merge exports. All are checked initially; unchecking preserves media and settings. Right-click an item to remove it (at least one remains). Projects preserve these selections.


### 工程分享与 ZIP 打包

建议将工程放在素材的上级目录，保留内部目录结构。工程页签的“打包工程”自动收集原素材、音轨与字幕，生成普通 ZIP，包内工程使用相对路径，同一源文件只收集一次。取消勾选的素材也会收集，以便对方继续编辑。工程内已有的编辑音频仍被保留。先在编辑窗口保存修改，再打包；该操作使用磁盘已保存版本，不修改原工程与素材。缺少素材时列出路径并停止，不生成不完整包。对方解压到任意目录，保持内部结构，打开 .uyap/.uyvp 后从页签进入编辑即可。后台任务中心可定位生成的 ZIP；当前不提供缺失素材重新定位向导。

### Sharing projects as ZIP bundles

Keep the project above its media folders and preserve their structure. Package Project collects source media, audio tracks and subtitles into a standard ZIP, rewriting the bundled project's references to relative paths and collecting each source file once. Unchecked items are included for continued editing, and embedded edited audio is preserved. Save editor changes first: packaging uses the on-disk project without changing the original project or media. Missing media paths are reported and no incomplete archive is produced. Recipients can extract anywhere, preserve the structure, open .uyap/.uyvp and enter the editor from the tab. Locate the ZIP from Background Tasks. A missing-media relinking wizard is not currently provided.

编辑列表移除后可在当前编辑窗口按 Ctrl+Z 恢复条目、原顺序及编辑设置，最多保留 100 项移除记录；有移除记录时优先撤销移除，记录不随工程保存。

In an editor, Ctrl+Z restores removed list items with their former position and edit settings, retaining up to 100 removals. Removal undo takes priority while available and is not saved in project files.


### 视频画布与填充色

“画布大小”可缩小或扩展原视频的显示区域，W/H 最大 8192 像素，X/Y 是画布相对原视频的原点，可输入负数向左/上扩展。只扩展画布不会拉伸原画面；超出原视频的区域使用“填充色”。拖动框内移动画布，边缘调整单边、四角按比例调整。“重置范围”恢复原视频画布尺寸和零偏移，不改变下方独立的画面缩放。填充色支持色板和 #RRGGBB（如 #FFFFFF）或 #RGB（如 #FFF）输入。

### Video canvas and fill color

Canvas Size crops or extends the video area, up to 8192 pixels per side. X/Y describe its origin relative to the source; negative values extend left/up. Extending does not stretch the source image; new areas use Fill Color. Drag inside to move the canvas, edges to resize one dimension, and corners proportionally. Reset Range restores source dimensions and zero offset without changing separate picture scaling. Fill Color supports the picker and #RRGGBB or #RGB input.


## 实时语音字幕 / Live speech subtitles

视频播放页“实时字幕”可启用本地识别并切换引擎；全屏时从画面右键菜单进入。音频来自当前播放音轨，本地视频、网络视频和频道复用播放器音频，不额外请求媒体连接。需要已安装 FFmpeg 和 sherpa-onnx 运行库。

- SenseVoice：普通话、英语、日语、韩语、粤语全部列出，共用约 240 MB 模型，安装一次即可全部使用。可在“第三方支持”安装运行库并点击“安装 / 校验语言模型”，也可在首次识别时下载模型。识别语言由用户选择，短句停顿后显示；持续讲话约 6 秒分段。
- Zipformer：本轮提供约 198 MB 的 INT8 中英双语流式模型，连续更新当前识别文字。它同时识别中英，不强制只识别其中一种；此模型不包含日语、韩语或粤语。可在同一组件窗口按需安装，首次启用也会下载。
- 模型准备完成后从当前播放位置开始，不补识别下载期间已经播放的内容。暂停时停止提供新音频，已经提交的短段可能稍后出字；跳转、重建播放、换音轨及关闭时清除旧会话，换引擎或语言也会重新开始。识别明显跟不上时停止字幕并提示，音频播放不等待识别。
- 实时字幕可显示原文，开启M2M100实时翻译后可选仅译文或原文／译文双语；自动缓存与SRT导出目前仍只保存识别原文，字幕文件翻译独立使用。实时字幕覆盖层使用独立显示样式；若已有文字字幕重叠，可先关闭原有字幕轨。只处理开启之后实际播放的内容，不是整片转写或原有字幕的替代备份。

Open **Live subtitles** on the video playback page; in fullscreen, use the video context menu. Local videos, online videos and channels reuse decoded audio from the selected playback track without opening another media connection. FFmpeg and the sherpa-onnx runtime are required.

- **SenseVoice** lists Mandarin, English, Japanese, Korean and Cantonese. All five share one model of about 240 MB. Install the runtime and use **Install / verify language model** in Third-party Support, or download on first recognition. Select the source language; phrases appear after a pause, with continuous speech segmented at about six seconds.
- **Zipformer** currently offers an INT8 Chinese/English bilingual streaming model of about 198 MB. It updates the current utterance continuously and recognizes both languages rather than forcing one. This model excludes Japanese, Korean and Cantonese. Install it from the component window or download on first use.
- Recognition begins at the current playback position after model preparation; audio played during downloads is not recovered. Pausing stops new input, although pending phrases may arrive later. Seeking, restarting playback, changing tracks, closing, or changing engines/languages resets the session. If recognition falls behind, subtitles stop with a message while audio playback continues.
- Live output supports source text or M2M100 translated/bilingual display. Recognition caching and SRT export still contain source text only; subtitle-file translation remains separate. The live overlay has its own display style; disable an existing subtitle track if text overlaps. Only audio played after enabling recognition is processed.


### 实时字幕缓存与回放 / Live subtitle caching and replay

每次识别独立保存 UTF-8 JSONL 记录和 SRT：记录媒体指纹、音轨、源语言、识别引擎及模型版本、媒体时间码和接收时间。JSONL 保留临时与最终结果，SRT 只追加已确认句子。“定位实时字幕缓存”打开最近一次会话的记录，“导出实时字幕 SRT”另存该会话的已确认字幕，不覆盖已有字幕文件；切换引擎、语言、音轨或跳转后形成独立会话，不把不同时间线直接拼接。可将导出的 SRT 与整片生成字幕人工对比；自动比对、合并、校正暂未实现，也不自动改写原字幕。没有缓存原始音频。

同一文件身份（路径、大小、修改时间）或已确认结束的 HLS 媒体清单、相同音轨 / 语言 / 引擎 / 模型下，回到已确认字幕区间时优先按媒体时间显示缓存，并跳过完全落在这些区间的重复识别音频包。未覆盖区间继续识别，跨缓存区间时重置识别时间基准。仅缓存当前实际播放并确认的片段，不宣称整片已完成；沉默空隙或句子边界仍可能需要处理。已识别区间起播时可暂不启动识别模型。

HLS 点播复用要求正时长，且在最多 1 MiB、4 秒超时的媒体清单检查中确认 `#EXT-X-ENDLIST`，使用完整来源地址散列及清单内容指纹；来源或清单改变则不复用。直播、未结束 / 无法读取 / 需额外 Cookie 的清单、只有主清单而未取得结束标记的情况仅记录本次识别，不跨会话自动复用。该检查只在启用实时字幕后后台额外读取一次清单，普通播放不等待，不重复下载媒体音频。签名 URL 改变可能产生新缓存，当前不去除鉴权参数推测同一内容。

缓存位于应用缓存目录的 `live_subtitles`，每会话最多 64 MiB；回读最多最近 128 会话、累计 64 MiB、10,000 句。达到上限或写入失败会提示并停止实时识别，不阻塞音频。设置中的缓存清理可删除这些记录，正在写入的会话受保护；缓存不作为永久备份，需保留时请导出 SRT。

Each recognition session saves UTF-8 JSONL evidence and an SRT file with media identity, audio track, source language, engine/model version, media timestamps and receipt times. JSONL retains drafts and finalized results; SRT appends finalized cues only. **Locate live subtitle cache** opens the latest session; **Export live subtitles as SRT** saves that session’s finalized cues without overwriting existing files. Seeking or changing engine, language or track creates a separate session. Compare exports with full-file subtitles manually; automatic comparison, merging and correction are not implemented. Original audio is not cached.

For matching local file identity (path, size and modification time), or a verified finite HLS media playlist, and matching track/language/engine/model, replay displays finalized cached cues at their media timestamps and skips recognition packets wholly inside those intervals. Uncovered intervals are recognized normally; recognizer timing resets across skipped intervals. Only confirmed, played portions are cached. Silence and cue boundaries may still require processing. Starting inside a cached interval can defer loading the recognition model.

HLS reuse requires positive duration and an `#EXT-X-ENDLIST` media playlist verified with a 1 MiB limit and four-second timeout. The full source address is hashed and combined with a playlist-content fingerprint. Changed sources/playlists, live or unfinished streams, failed checks, additional Cookie requirements, or master playlists without a verified end marker do not reuse earlier sessions. This runs in the background only after enabling live subtitles, adding one manifest request without delaying ordinary playback or downloading media audio again. Changed signed URLs can create separate caches; authentication parameters are retained in the identity.

Caches reside under `live_subtitles` in the application cache. Each session is limited to 64 MiB; replay reads at most 128 recent sessions, 64 MiB total and 10,000 cues. Limits or write failures stop recognition with a message without blocking playback. Settings cache cleanup removes these files while protecting active writers. Export SRT for lasting retention.

### 字幕投影 / Subtitle shadow

字幕样式中的投影支持独立 X/Y 偏移，范围各为 -12 到 12；X 正值向右、Y 正值向下，负值向相反方向。描边仍用一个宽度值。旧版单个投影值会同时迁移到 X/Y，原字幕文件保持不变。此设置用于已有文字字幕的渲染，实时字幕覆盖层仍使用其独立样式。

Subtitle styles provide independent shadow X/Y offsets from -12 to 12: positive X moves right and positive Y moves down. Outline retains a single width. Legacy scalar shadows migrate to both axes without changing source subtitle files. This setting affects existing text-subtitle rendering; the live overlay retains its independent style.


实时字幕优先按短停顿断句，连续讲话按时间和长度分段；临时文字会等连续识别结果一致后更新。画面按左右边距自动换行，最多显示两排，极长文本显示最新尾部，缓存仍保留完整识别结果。生成的长字幕按标点或单词边界拆分，分段时间按文字长度估算，不等同逐字时间对齐；模型误识别出的重复词不自动删除。

Live subtitles segment at short pauses, with duration/length limits for continuous speech. Partial text waits for agreement between consecutive hypotheses. Text wraps inside screen margins to at most two rows; exceptionally long text shows its latest tail while the full result remains cached. Generated long cues split at punctuation or word boundaries using proportional estimated timing, not word-level alignment. Repetitions recognized by the model are not automatically deleted.


### 点播提前识别与翻译组件重试 / VOD lookahead and installation retries

SenseVoice 的“点播提前识别字幕”默认启用，适用于本地有正时长的视频和已经确认 ENDLIST 的 HLS 点播。独立 FFmpeg 进程从当前播放位置解码后续音轨，在后台为识别器提供最多约 20 秒领先音频；结果写缓存，画面只按媒体时间显示对应确认字幕。暂停可继续补足这段窗口，跳转、换音轨、引擎或关闭会取消旧解码与识别会话；首次加载、跳至未缓存位置或识别慢于播放时仍可能暂时缺字幕，不保证零延迟。HLS 此模式会增加独立音轨读取的网络与解码开销，可关闭该选项；关闭后和直播一样复用播放 PCM，按句延迟显示。Zipformer 保持原有流式处理。提前识别片段也属于缓存内容，可能尚未被观看，不代表整片识别完成。

M2M100 安装包括 Python 运行库、INT8 模型下载、校验及加载验证。下载到100%不代表安装完成；失败保留断点文件，最多重试三次，续传响应须验证，模型须通过固定大小及SHA-256校验。下载缓存位于组件目录 `m2m100-418m-ct2-int8-1/downloads`。网络失败仍需恢复连接，日志使用UTF-8。

SenseVoice enables **Recognize VOD subtitles ahead** by default for finite local videos and HLS VOD with a verified ENDLIST. A separate FFmpeg process decodes the selected audio from the current position, feeding recognition up to about 20 seconds ahead. Results enter the cache and finalized cues display only at their media times. Pausing can fill the bounded window; seeking, track/engine changes and closing cancel the old session. Startup, uncached seeks and recognition slower than playback may still cause missing or late cues. HLS lookahead adds independent audio/network decoding; disable it to reuse playback PCM with phrase latency, as for live streams. Zipformer retains streaming processing. Cached lookahead may include content not yet watched and does not imply full-file completion.

M2M100 installs the Python runtime, downloads the INT8 model, verifies checksums and checks loading. A 100% download is not installation completion. Downloads retry up to three times and retain validated resumable partials in `m2m100-418m-ct2-int8-1/downloads`. Network connectivity is required for installation; logs use UTF-8.


字幕左右边距限定文字可用宽度，实时字幕以实际可见画面为范围，长词也会在框内折行。过小窗口会限制边距以保留最小文本区域。播放器的字幕样式优先于 ASS 单句边距、绝对定位及禁止换行设置；原字幕文件不改动，内嵌在视频像素中的字幕不受样式设置影响。

Horizontal margins constrain subtitle text width. Live text uses the visible video area and wraps long words inside the box. Very small windows clamp margins to retain a minimal text area. Player styles take priority over ASS per-cue margins, absolute positioning and disabled wrapping. Source subtitle files stay unchanged; text burned into video pixels is unaffected.


### M2M100 翻译与资源 / M2M100 translation and resource use

字幕翻译仅使用M2M100 418M INT8（CTranslate2），界面开放中文、英语、日语、韩语间12个直接翻译方向，不含粤语。源语言从字幕信息预填，未知时需确认。在第三方支持安装，运行库位于M2M100专用目录；不再提供OPUS-MT安装和翻译入口。

M2M100 从 JustFrederik 的社区 Fairseq 转换包固定版本下载，模型及分词 / 配置 / 词表 / 说明合计 494,252,337 字节，约 494 MB（471 MiB）；安装保留同等大小下载缓存以支持重试，模型和下载缓存合计约 1 GB，另加共享 Python 运行库，建议预留 2 GiB 硬盘空间。无需下载约 1.94 GB 原始权重或安装 Torch / Transformers 来转换。各文件固定大小和 SHA-256 校验，通过 CTranslate2 CPU INT8 加载及分词器检查后才发布到安装目录。来源标明社区转换，原模型 MIT；真实翻译质量与加载兼容性仍需本机验证。

第三方支持显示运行内存预算 1–3 GiB，属于保守估算而非实测占用，输入长度、词表和线程都会影响结果。Windows 可在执行后显示最近安装校验或翻译进程的峰值工作集（不是整机内存、不是保证下次相同）。每次字幕翻译使用独立 Python 进程，父进程等待退出后保存结果；不会在主程序里常驻翻译模型，任务进程退出后模型占用由系统回收。模型文件和下载缓存留在硬盘供下次使用，重新翻译需要重新加载；持续任务中内存保持占用。

翻译菜单选择源／目标语言，可选双语。输出使用 `.m2m100-ja-to-zh.srt` 等后缀，双语加 `.bilingual`，原文在上、译文在下并保留时间码。已有旧译文不删除，仍识别其语言方向后缀。超过512分词token的单句会报错要求先拆分。此菜单用于字幕文件翻译；实时翻译在实时字幕行单独开启，尚无自动校正。

M2M100 418M INT8 (CTranslate2) is the sole subtitle translation model. The UI enables 12 direct directions among Chinese, English, Japanese and Korean, without Cantonese or pivoting. Confirm unknown source languages yourself. Install it through third-party support; the runtime lives in the M2M100 component directory. OPUS-MT installation and translation entries have been removed.

The pinned community Fairseq conversion by JustFrederik totals 494,252,337 bytes, about 494 MB (471 MiB), including tokenizer, configuration, vocabulary and model card. Retained download files plus installed model total about 1 GB, excluding shared Python runtime; reserve 2 GiB. No 1.94 GB original weights, Torch or Transformers conversion are required. Fixed sizes/SHA-256 checks and CPU INT8/tokenizer loading precede installation publication. The source is identified as a community conversion of the MIT-licensed model; real translation quality and loading compatibility still need local validation.

The component UI budgets 1–3 GiB RAM as an estimate, not a measurement. Windows can display the last installation-check or translation process peak working set, not total system memory or a promise for future tasks. Translation runs in a dedicated Python process, which exits before the parent saves output; the GUI does not retain the model. Process exit releases its model memory through the OS. Model files and retained downloads remain on disk; subsequent tasks reload them.

Choose source/target languages and optional bilingual output. Filenames use suffixes such as `.m2m100-ja-to-zh.srt`, with `.bilingual` for source-then-translation output and unchanged timing. Existing legacy translations remain readable. Cues exceeding 512 input tokens fail explicitly. This menu translates subtitle files; live translation has separate live-subtitle controls. Automatic correction is not included.


### 字幕位置及长句换行 / Subtitle placement and long lines

字幕样式中的“字幕位置”可选顶部或底部，下方边距名称随之切换；旧设置默认底部。投影 X/Y 显示为输入框外标签。位置与边距应用于文字字幕，实时层沿用位置和边距，其字体和背景仍独立。文字字幕边距采用 ASS 脚本坐标，随画面缩放；实时层边距采用界面逻辑像素，两种渲染的数值不代表相同物理像素距离。

无空格中文等长句不再只依赖 libass 自动换行：临时字幕根据脚本宽度、两侧边距、字号及字体测量插入明确换行；Windows 使用系统字体测量并保留字形回退 / 加粗余量，其他系统采用保守宽度估算。原有显式换行与格式标签保留，单句字体 / 字号 / 横向缩放服从播放器设置以免测量和实际宽度不一致。其他窗口同步样式也会重建临时字幕，避免残留旧边距；原字幕文件不改写。

**Subtitle position** selects top or bottom, with a matching edge-margin label. Existing settings default to bottom. Shadow X/Y labels are outside the numeric inputs. Text subtitles and the live layer share position and margins, while live fonts/background remain separate. Text-subtitle margins use scaled ASS script coordinates; live margins use UI logical pixels, so equal numbers do not guarantee equal physical distances.

Unspaced long text no longer depends solely on libass automatic wrapping. Temporary subtitle text receives explicit breaks based on script width, margins, font size and measured glyph widths. Windows uses system-font measurements with fallback/bold headroom; other platforms use conservative estimates. Existing explicit breaks and formatting tags remain, while per-cue font/size/scaling follow player settings so measured and rendered widths agree. Cross-window style synchronization rebuilds temporary subtitles rather than retaining old margins. Original subtitle files remain unchanged.


### 统一样式、原生 ASS 与字幕导出 / Unified style, native ASS and export

“字幕样式”顶部提供两种模式，默认“统一样式”：字体、位置、边距和显式换行服从播放器设置，适合普通文字字幕。选择“原生 ASS”后，跳过播放器的字体 / 定位 / 换行重写和强制样式；直接加载 UTF-8 ASS 文件时复制原内容作为临时渲染副本，保留样式表、定位、移动、淡入淡出、卡拉 OK、旋转 / 缩放等标签。SSA、内嵌文字字幕及其他格式仍经 FFmpeg 转成 ASS，效果受转换、libass 版本和本机字体影响；不包含安装字幕字体或保证全部特效。原生模式由字幕文件控制排版，播放器边距不强制生效；实时字幕覆盖层仍独立，不以 ASS 标签渲染。

已载入文字字幕后，从字幕菜单的“导出当前字幕”选择 SRT 或 ASS。SRT 主要保存文字和时间码，无法保留 ASS 动画和布局。ASS 在统一模式下固化当前播放器样式及换行，在原生模式下保留临时副本的原生内容；采用 `.export.srt` / `.export.ass` 作为默认名字，另存不覆盖已有文件，成功后可定位文件。自动生成和翻译的默认输出仍为 SRT，生成后载入即可另存 ASS；实时缓存导出仍为 SRT。导出是字幕文件，不将字幕烧录进视频，也不嵌入字体。

样式窗口已打开时，再从任意字幕入口点击“字幕样式”会唤醒同一窗口，保留未应用的草稿和字体搜索；关闭后重新打开才从当前已应用样式初始化。特效预设、逐字高亮编辑器与 ASS 标签编辑器尚未实现，当前支持播放与保留已有 ASS 标签。

The style window offers **Unified style** by default, using player fonts, placement, margins and explicit wrapping. **Native ASS** skips player font/position/wrapping rewrites and forced styles. UTF-8 ASS files are copied directly into a temporary rendering file, preserving style tables, movement, fades, karaoke, rotation/scaling and other tags. SSA, embedded and other text formats still pass through FFmpeg conversion; effects depend on conversion, libass and installed fonts. Font installation and complete effect compatibility are not guaranteed. Native layout is controlled by the subtitle file rather than enforced margins; the live overlay remains separate.

After loading text subtitles, use **Export current subtitles** in the subtitle menu to choose SRT or ASS. SRT primarily retains text/timing, not ASS animation/layout. Unified ASS export bakes current player styles and wrapping; native ASS export keeps the temporary file’s native content. Defaults use `.export.srt` / `.export.ass`, existing files are not overwritten, and successful output can be located. Generation and translation still default to SRT; load their output to export ASS. Live-cache export remains SRT. Export does not burn subtitles into video or embed fonts.

Invoking the style window again refocuses the existing window while preserving pending edits and font search. Reopening after closing initializes from the applied style. Effect presets, a karaoke editor and an ASS tag editor are not implemented; current support plays and preserves existing ASS tags.


统一样式的文字字幕边距明确采用“脚本单位”，不是百分比，也不是固定屏幕像素。脚本高度沿用字幕值（普通转换常为 288），宽度按当前视频比例校正；例如 16:9 视频采用 512×288 脚本，横向显示宽度为 744 像素时，20 单位约合 29 像素。播放器缩放时实际像素距离随之变化。原生 ASS 保留文件自身的脚本坐标；实时覆盖层仍以界面逻辑像素解释同一设置数值。

Explicit wrapping now matches the unified ASS canvas aspect to the video while retaining script height. Margins are script units, not percentages or fixed screen pixels: a 16:9 video with a 512×288 script displayed 744 pixels wide gives about 29 displayed pixels for a 20-unit horizontal margin. Native ASS retains its original canvas; the live overlay still uses UI logical pixels. Font measurement uses GDI cell height rather than EM height, with bold measurement and a small rasterization allowance to avoid overly narrow CJK lines.


样式窗口中的文字字幕修改会在停止输入约 180 ms 后预览（后台重新生成字幕，可能短暂刷新画面）；播放页显示“预览（尚未保存）”及当前预览模式。“应用”才保存为全局设置，“取消”或直接关闭恢复当前已保存样式。重复呼出保留草稿；导出当前字幕使用当时已生成的预览资产及对应样式。实时语音覆盖层仍只读取已保存的边距 / 位置，不预览该窗口的草稿。

Text-subtitle style changes preview after about 180 ms without further edits, rebuilding subtitles in the background and potentially briefly refreshing the picture. The playback header identifies the mode and unsaved preview. Apply saves globally; Cancel or closing restores the saved style. Reopening preserves the draft, and exporting snapshots the prepared preview asset and its style. The live speech overlay still reads saved placement/margins rather than the dialog draft.


### 长字幕分时显示 / Long subtitle paging

统一样式会将超过两行容量的单行纯文本字幕优先按标点拆段，依次显示。每段至少1秒；没有逐字时间戳时按文字长度估算切换点，未保证逐字同步。显式多行、双语或ASS特效保留原分段，源文件不变。

Unified style pages long plain single-line cues into at most two lines per page, preferring punctuation. Each page needs at least one second. Without word timestamps, transitions are estimated from text length. Explicit multiline, bilingual and ASS effects keep their original segmentation; source files remain unchanged.

生成字幕也会按停顿、标点、视觉长度和目标时长拆段，并保存到SRT；实时最终字幕复用同一分段。SenseVoice连续识别段最长6秒。段内时间为长度估算，已有字幕与缓存需重新生成才能获得新分段。

Subtitle generation also segments speech by pauses, punctuation, visual length and target duration, saving the result to SRT. Final live cues share this segmentation. Continuous SenseVoice recognition segments are capped at six seconds. Internal timing is estimated from text length; existing subtitles and caches require regeneration to receive the new segmentation.


### 视频导出 ASS / ASS video export

“添加字幕”可添加ASS／SSA。单项或队列导出选择MKV时保留ASS样式、定位与特效标签，并复制相关输入的字体附件；其他容器仍按其支持的文字字幕格式转换。字体未随输入提供时依赖播放器已安装字体。裁剪会重设字幕时间；跨片段起点的动画／卡拉OK可能从裁剪后的事件起点重新计时。多片段合并到MKV暂不接受ASS轨道，请使用单项／队列导出，或取消该字幕轨道并选择烧录。

视频设置中的“烧录 ASS 字幕”选择外部ASS，按源视频绝对时间对齐，先渲染字幕再裁剪／缩放画面，因此画面裁剪也会裁掉对应字幕像素。烧录不可关闭，内嵌字幕仍按轨道列表独立导出；不需要重复字幕时取消该轨道。普通视频、双遍编码及合并片段支持烧录；GIF和PNG序列暂不支持。烧录依赖FFmpeg的libass和系统字体，不自动收集外部字体。所选ASS作为工程资产保存并随工程打包，保存后修改外部ASS不会改变已保存工程中的副本。

Add ASS/SSA through “Add subtitles”. Individual or queued MKV exports retain ASS styles, positions and effect tags and copy attachments from the relevant inputs. Other containers convert to their supported text subtitle format. Fonts absent from attachments depend on the player’s installed fonts. Clipping rebases event times; animation or karaoke crossing the trim start may restart relative to the clipped event. Merging ASS tracks into MKV is not yet supported: use individual/queue export or deselect the track and burn it in.

“Burn ASS subtitles” selects an external ASS file aligned to source video times. Rendering occurs before crop/scale, so cropping also cuts subtitle pixels. Burned subtitles cannot be disabled; embedded tracks are exported independently. Deselect unwanted duplicate subtitle tracks. Regular video, two-pass encoding and merged pieces support burn-in; GIF and PNG sequences do not. Burn-in requires FFmpeg libass and installed fonts; external fonts are not collected automatically. The selected ASS is saved as a project asset and included in project packages, so later edits to the external file do not change the saved copy.


### 实时字幕翻译开关 / Live translation controls

实时字幕行依次显示名称、设置图标、启用开关；开启后追加“翻译:”目标语言下拉菜单、翻译开关、“显示源字幕”开关。均使用设置界面的开关样式。模型准备及错误提示放在整行其他按钮之后；不显示逐句“正在翻译”及转圈提示，窄窗口可自然换行。设置菜单保留识别引擎、识别语言、提前识别与缓存操作；Zipformer中英混合识别需明确选择翻译源语言，不自动猜测语言。M2M100支持中英日韩，不支持粤语；源目标相同时原样显示。

翻译仅提交已确认短句及当前点播缓存字幕，不翻译不断修订的临时结果。每个活跃会话使用独立常驻Python进程，模型只加载一次；队列最多一个等待请求和一个处理中请求，忙时重试最新短句，不保证保留全部中间译文。最多保留128项内存译文，不写磁盘。原文独立即时更新，译文保持底部，双语时原文上移。较早译文短暂保留时标记“稍早译文”，不会把原文回滚成旧句；不启用翻译时保留原显示方式。原文默认显示；关闭“显示源字幕”后等待期间可能暂时无字幕。原文保留最多两行的现有规则；译文按宽度自然换行，实际多行高度向上推原文，超出安全区仍裁切。背景使用50%不透明度。关闭翻译、关闭实时字幕、切换目标／源语言、切换识别会话或关闭播放器会结束旧翻译进程，释放模型内存；再次启用需重新加载。翻译失败会停止翻译并显示错误，原文识别继续；需先在第三方支持安装M2M100。

The row contains the live-subtitle label, settings icon and enable switch. Enabling adds the translation target selector, translation switch and source-text switch, using the settings switch style. Model preparation and error notices follow all other row controls; per-phrase translation progress text and spinners are hidden. Narrow windows may wrap. Settings retain recognition, lookahead and cache controls. Zipformer bilingual recognition requires an explicit translation source language. M2M100 supports Chinese, English, Japanese and Korean, without Cantonese; identical source and target languages display unchanged text.

Only finalized phrases or currently visible cached VOD cues are submitted. Each active session uses a dedicated persistent Python process with one model load, one active request and at most one queued request. Busy sessions retry the latest phrase; intermediate translations may be skipped. Up to 128 translations stay in memory only. Source text updates independently. Translation remains at the bottom with source above in bilingual mode. Retained earlier translations are marked and never replace the current source sentence. Disabling translation preserves the original source-only display. Source text is shown by default; translation-only mode may show nothing while waiting. Source text keeps its existing two-row limit. Translation wraps naturally and its actual height pushes the source upward, with no reserved blank rows or extra block gap; overflow is clipped to the safe area. Background opacity is 50%. Disabling translation or recognition, changing source/target/session or closing the player terminates the old process and releases its model memory. Failures stop translation while source recognition continues. Install M2M100 through third-party support first.


### 字幕 AI 计算设置 / Shared subtitle AI compute settings

第三方支持提供“优先使用GPU”（默认开）和“CPU回退线程”（默认2，范围1～4），统一用于新启动的字幕生成、实时识别、文件翻译和实时翻译；运行中的任务需停止再启动才读取新设置。设置持久化于组件目录ai-runtime.json，不做全核自动占满。Windows下Python字幕任务采用较低进程优先级及被动等待，给播放器留出资源；线程数不是CPU占用百分比硬限制。

GPU优先表示验证后使用，不表示所有独立显卡可用。M2M100使用NVIDIA CUDA INT8混合精度，先执行实际短句预热验证，缺动态库／驱动不兼容／推理失败时回退CPU；实时设置菜单显示实际后端及回退原因。AMD／Intel GPU尚未接入。SenseVoice／Zipformer还需要CUDA版sherpa-onnx及相应ONNX Runtime；当前常规安装包缺少CUDA provider，仍使用CPU并记录原因。共享偏好不会把CPU引擎自动变成GPU引擎。

“安装／补齐GPU运行库”在独立组件目录ai-gpu-runtime安装固定版本cuBLAS、CUDA Runtime及cuDNN，不安装显卡驱动或整个CUDA Toolkit，不替换现有识别引擎。Windows下载约1.3GB，建议至少4GiB空闲，安装后验证DLL可加载再标记就绪；这不代表所有模型GPU推理都验证通过。组件用于本程序，不修改系统PATH。GPU运行仍可能占用部分CPU及显存，不能保证所有机器视频播放不卡顿。

Third-party support provides Prefer GPU (default on) and CPU fallback threads (default 2, range 1–4), shared by newly started generation, live recognition and file/live translation tasks. Restart an active task to apply changes. Preferences are stored in ai-runtime.json in the components directory. Windows Python tasks use below-normal priority and passive waiting; the thread limit is not a hard CPU-percentage cap.

GPU preference requires actual engine support. M2M100 uses NVIDIA CUDA mixed INT8 precision after a real warmup translation; unavailable libraries or failed inference fall back to CPU. Live settings display the actual backend and fallback reason. AMD/Intel GPU backends are not integrated. SenseVoice/Zipformer also need CUDA-enabled sherpa-onnx/ONNX Runtime; the current standard package lacks that provider and continues on CPU. A preference cannot turn a CPU engine into a GPU build.

Install/repair GPU runtime installs pinned cuBLAS, CUDA Runtime and cuDNN packages into the isolated ai-gpu-runtime component directory, without installing drivers, the full CUDA Toolkit or a replacement recognition engine. Windows downloads total about 1.3 GB; reserve at least 4 GiB. DLL loading is checked before marking libraries ready, separately from model validation. System PATH is unchanged. GPU execution still uses some CPU and VRAM; smooth playback is not guaranteed on every machine.


GPU运行库补齐会先校验，库完整时不重装；损坏需修复时若文件被使用会提示先关闭字幕任务，不会强制结束进程。实际修复在暂存目录安装并校验后替换，避免pip直接覆盖已用DLL。实时翻译启用后显示实际GPU／CPU标记，悬停可查看后端和回退原因；GPU优先开关本身不代表GPU已经使用。

GPU runtime repair validates first and skips intact libraries. If repair is needed while files are in use, it asks you to stop subtitle tasks without forcibly ending processes. Repairs install and validate staged files before replacement. Live translation displays its actual GPU/CPU backend; hover for details and fallback reasons. GPU preference alone does not mean GPU execution is active.


“适应窗口”播放会按启动播放时窗口的物理像素缩小预览帧，减少高分辨率画面的转换和上传开销；原尺寸／自由缩放保持源分辨率，切换显示模式会重启解码。调整窗口大小后在下次启动播放时更新预览尺寸。源文件和导出分辨率不变；4K窗口或原尺寸模式仍可能有较高开销。

Fit-to-window playback bounds preview frames by the physical window size when playback starts, reducing conversion/upload work. Original-size and free-scale modes retain source resolution; switching modes restarts decoding. Window resizes apply at the next playback start. Source files and export resolution remain unchanged; 4K windows and original-size mode may still incur high overhead.


### 视频 GPU 显示 / GPU video display

普通播放复用程序已有Glow渲染系统，常见8位SDR的YUV420P、NV12等来源由FFmpeg输出统一YUV420平面，通过GPU三张单通道纹理及着色器转换显示，不再在CPU逐帧构造RGBA图片并交给egui上传。视频信息行出现“YUV · GPU”表示正在使用该显示路径，这与翻译模型的GPU／CPU标记不同。egui继续绘制控件、实时字幕及覆盖层；镜像、缩放、拖动和可见范围裁切保持可用。

输入颜色矩阵／范围优先按视频信息指定；缺失时高清按BT.709、标清按BT.601，yuvj按全范围，其余默认限幅。FFmpeg归一化成BT.709限幅，GPU按对应规则显示。HDR、BT.2020、10位等未支持YUV的内容保留RGBA解码，再由RGBA着色器显示；透明视频、GIF／图片序列、暂停逐帧定位和编辑预览也可走共享GPU显示。GPU不可用时才用egui图片纹理兼容路径。着色器失败会回退兼容解码。不能将此功能理解成HDR色调映射或所有像素格式直通。

当前仍通过FFmpeg管道读回YUV并上传GPU，还不是硬解码纹理共享或零拷贝。4K25fps下YUV420原始帧约311MB/s，RGBA约829MB/s；这是数据量估算，不是CPU降幅或速度保证。适应窗口的尺寸限制仍可进一步减少预览数据，源文件和导出分辨率不变。GPU视频资源复用已有上下文并在渲染系统退出时清理。

Normal playback reuses the existing Glow renderer. Common 8-bit SDR sources such as YUV420P and NV12 are normalized by FFmpeg to planar YUV420, uploaded as three single-channel textures and converted by a GPU shader, avoiding per-frame CPU RGBA image construction and egui image uploads. “YUV · GPU” in video information identifies this display path, separately from the translation backend indicator. Egui retains controls and subtitle overlays; mirroring, scaling, panning and clipping remain available.

Source matrix/range metadata is preferred. Missing metadata defaults to BT.709 for HD or BT.601 for SD; yuvj is full-range and other sources default to limited range. FFmpeg normalizes to limited-range BT.709 for the shader. HDR, BT.2020, 10-bit and other unsupported YUV content retain RGBA decoding and use RGBA shaders. Alpha video, GIF/image sequences, frame seeking and editor previews also use shared GPU display. Egui image textures remain as fallback when GPU rendering is unavailable. Shader failures fall back to compatibility decoding. This does not add HDR tone mapping or native handling of every pixel format.

Frames are still read back through FFmpeg’s pipe and uploaded: hardware-decoder texture sharing and zero-copy are not implemented. At 4K25, raw YUV420 is about 311 MB/s versus 829 MB/s for RGBA; these are payload estimates, not promised CPU or speed improvements. Fit-to-window sizing can further reduce preview traffic without changing source/export resolution. Video GPU resources share the existing context and shutdown lifecycle.


图片主预览默认“平滑显示”：放大使用双线性采样，缩小配合mipmap，避免普通图片一直使用最近邻导致轮廓阶梯。“像素显示”使用最近邻，适合放大检查像素块；选项按当前图片保留，原图、导出和像素取样数据不变。平滑纹理的mipmap额外占用约三分之一显存，不能保证与Photoshop的缩放算法完全一致。

Main image previews default to Smooth: bilinear magnification and mipmapped minification avoid nearest-neighbor stair-stepping. Pixelated mode retains nearest sampling for inspecting individual pixels. The mode is per image and does not alter originals, exports or sampled pixel values. Smooth mipmaps add roughly one third to texture memory and are not guaranteed to match Photoshop’s resampling exactly.

图片缩放百分比按屏幕物理像素计算，100%表示一个源像素对应一个屏幕物理像素，而非一个系统缩放后的逻辑点；适应窗口的百分比会随DPI变化。绘制起点对齐物理像素。

Image zoom percentages use physical screen pixels: 100% means one source pixel per physical pixel, rather than one scaled UI point. Fit percentages therefore depend on DPI; image origins are aligned to physical pixels.

YUV播放预缩放采用Lanczos与精确舍入，输出色度位置统一到中心；GPU输出保留稳定空间抗色带抖动，不是画面模糊或帧间随机噪声。它不能消除源视频本身的隔行、压缩条纹或所有8位色带。

YUV playback uses Lanczos pre-scaling with accurate rounding and centered chroma. GPU output adds stable spatial dithering to reduce quantization bands, rather than blur or frame-varying noise. It cannot remove interlacing, compression stripes or every 8-bit band already present in the source.


### 识别运行库 CPU／GPU 版本 / CPU and GPU recognition runtimes

第三方支持的SenseVoice／Zipformer区域分别显示CPU和GPU运行库安装状态及“安装／校验运行库”，允许只安装一个或同时保留。已有CPU安装默认继续使用CPU；“使用此版本”选择仅影响新启动的生成／实时识别任务，正在运行的会话不自动切换。如果只安装了一个可用版本，则使用该版本。CPU和GPU引擎独立目录，不互相覆盖；两者共用原有SenseVoice和Zipformer模型目录，不重复下载语言模型。

GPU版使用官方固定2.0.1+cuda12.cudnn9安装包，仍需兼容NVIDIA驱动及共享CUDA动态库。选择GPU运行库与“优先使用GPU”不同：GPU包也支持CPU，当GPU优先关闭或模型／provider不兼容时可以回退CPU。安装校验检查包来源目录、CUDA版本标识及接口导入，不等于实际INT8模型GPU推理成功。CPU版开销已满足需求时可继续使用，无需为了GPU更换。

Third-party support lists separate CPU and GPU runtime installation states for SenseVoice/Zipformer. Install either version or keep both, then choose Use this version. Existing CPU installations remain on CPU by default; changes affect newly started recognition/generation tasks, not running sessions. A sole installed runtime is used automatically. Runtime directories are isolated and share the existing model directories.

The GPU package is pinned to the official 2.0.1+cuda12.cudnn9 build and requires compatible NVIDIA drivers and shared CUDA libraries. Runtime selection is distinct from Prefer GPU: GPU builds can still use CPU when the preference is off or the model/provider is incompatible. Installation verifies imports, package directory and CUDA version identity, not actual INT8 GPU inference. Continue using CPU when its overhead is already acceptable.

实时翻译任务不依赖顶部工具栏或右键菜单可见性，全屏隐藏控制栏时仍继续提交确认短句、接收译文并显示。

Live translation task updates run independently of toolbar and menu visibility, continuing finalized-phrase submission and result reception when fullscreen controls are hidden.


视频编辑预览现在与普通播放共用YUV／RGBA渲染模块。透明视频保留直通RGBA像素，GPU逐采样点预乘Alpha后再插值，按预乘Alpha方式与棋盘格或所选底色合成；不会将棋盘格写入源视频或导出像素。为避免CPU先缩放直通RGBA造成透明边缘污染，GPU RGBA播放保持源尺寸，由着色器做显示缩放，因此其传输量不会像YUV那样降低。逐帧缓存／图片序列已解码的预乘RGBA也可直接由同一模块显示，缩略图仍可使用小尺寸egui图片。裁剪坐标始终按原视频尺寸计算，不按缩小后的预览纹理计算。已有FFmpeg解码器的透明格式支持范围不因换渲染方式自动扩展；HDR仍未新增专门色调映射。

Editor previews share the normal player’s YUV/RGBA renderer. Straight RGBA alpha video is premultiplied per texel on the GPU before interpolation and composited over the checkerboard or selected matte. The background is never baked into source/export pixels. GPU RGBA playback retains source dimensions to avoid filtering straight-alpha pixels before premultiplication; it therefore does not receive YUV’s transfer-volume savings. Cached premultiplied frames and image sequences use the same module; small UI thumbnails may still use egui images. Crop coordinates use source-video dimensions, not reduced preview textures. Decoder alpha-format support is unchanged and HDR tone mapping is not added.


实时SenseVoice默认“自动识别”，通常无需手动指定国语／日语等；模型返回的实际语言会传入翻译和JSON缓存，设置菜单显示识别结果语言。手动语言选项仍可用于纠正或限定识别范围。源语变化会重建翻译会话，可能再次等待模型加载。自动模式和手动语言的识别缓存分开保留；旧缓存缺少语言字段时仅对假名／韩文等明确特征作保守判断，仅汉字无法确认时不自动假定国语，请手动选语或重新识别。粤语仍不作为普通话直接翻译，Zipformer继续使用明确源语设置。该自动选项目前针对实时SenseVoice，不改变整片生成字幕菜单。

Live SenseVoice defaults to Auto-detect. Detected language is passed to translation and JSON caches and displayed in recognition settings; manual language selection remains available. A source-language change rebuilds the translation session and may require model reload. Auto and manual recognition caches remain separate. Legacy cues without language metadata use conservative kana/Hangul detection; ambiguous Han-only text is not silently assumed to be Mandarin. Select a language manually or regenerate those cues. Cantonese is not treated as Mandarin; Zipformer retains explicit source-language selection. This auto option currently applies to live SenseVoice, not the full-file subtitle generation menu.

播放结束时，进度条可停在总时长终点，同时画面保留最后一个有效帧；总时长边界不代表还存在额外一帧。
At playback end, the progress bar may show the full duration while retaining the last valid frame; the duration endpoint is not an additional decoded frame.


点播提前识别开启时，未来约20秒内已确认的字幕也可提前送M2M100翻译，结果先留在内存，到对应媒体时间再显示。当前句优先，任务有界；遇不同源语言边界暂不跨越预翻译。首次加载、首句未结束或识别速度不足仍可能等待，短片并未禁用提前处理，也不保证开头无延迟。未来译文不会提前进入“稍早译文”显示槽。

With VOD lookahead enabled, confirmed cues up to about 20 seconds ahead can be translated by M2M100 and held in memory until their media time. Current cues have priority and requests are bounded; prefetch stops at source-language boundaries. Startup, unfinished first phrases or slow recognition may still cause delays. Short videos are not excluded, and zero startup latency is not guaranteed. Future translations never enter the earlier-translation display slot before their playback time.

### ASS／SSA／SRT／WebVTT 字幕编辑与内嵌字幕提取

播放本地视频并选中外挂 ASS／SSA／SRT／WebVTT 后，点击字幕选择框旁或字幕菜单中的“字幕编辑”。编辑窗口独立播放，初始采用当前视频位置与音轨；重复打开同一字幕会聚焦已有窗口。窗口内可加载其他 ASS／SSA／SRT／WebVTT，但同一个文件不能同时在两个编辑窗口中修改。

编辑仅作用于内存草稿。字幕列表、视频预览、波形和时间轴联动；单击列表选中并显露对应时间轴，双击另将视频跳到该句。开启“跟随播放”可自动选中当前句。拖动字幕块整体平移，拖动两端调整起止；边界附近也可抓取，悬停时加粗并显示横向调整光标。多选后拖动可整体移动，重叠事件分行显示。滚轮缩放，底部全宽浏览进度条横向浏览而不跳视频，与时间轴保留间距；暂停时左右键逐帧，Ctrl+左右键平移五帧。“吸附视频帧”以实际视频帧时间索引为准，尚无索引时不强行吸附。波形按编辑器当前选择的音轨生成，切换音轨后重建。

左上角字幕文件信息行的“横屏布局／竖屏布局”Tab 可随时切换，初次按视频方向选择。竖屏布局将视频与播放控制置于左侧，右侧显示字幕属性、时间轴和列表，可拖动分界线调整视频宽度；横屏布局保留上方视频、右侧属性及下方时间轴和列表。切换保留草稿、选中、播放位置和时间轴缩放。

支持新增、复制、删除、批量提前／延后、查找替换、在指针处拆分和合并相邻字幕。拆分先将原文字复制到两段，按内容需要分别修改；合并要求样式、图层、角色名、边距、效果及注释类型相同。重叠仅提示，不自动消除；复杂动画和卡拉 OK 的相对时间不自动重写。起止时间可以手动输入，或从播放指针设置。可查看时长、字符数与 CPS。批量平移控件在宽度不足时整组换行。时间轴上方固定居中显示播放当前句（区间图标）、播放／暂停、上一帧、下一帧和音量；区间图标在播放按钮左侧，未选中有效字幕时禁用。音轨／语言和当前／总时间留在视频下方，按视频区域居中。

ASS 提供命名样式选择／复制、字体、字号、颜色、描边、阴影、对齐、边距，以及单句图层、角色、效果和注释编辑。文本框保留 ASS 标签，换行对应 `\N`；复杂定位、移动、绘图、动画及卡拉 OK 暂无可视化编辑器。SRT 可编辑文字和时间，ASS 专属项禁用，另存为 ASS 后可用。已有 ASS 的脚本信息、未知段落、样式字段和未修改的事件字段保留；不以播放器的统一样式或自动换行替换源稿。

“保存”或 Ctrl+S 覆盖当前文件；覆盖前保留一份同路径 `.bak`，并检查自加载以来源文件是否改变。发现外部修改时不覆盖，应重新加载或另存到新路径。保存采用同目录暂存后替换，失败保留内存草稿。另存为初次打开时默认 ASS 类型及 .ass 文件名；可切换保存类型。另存为成功后，新文件成为当前编辑目标；原文件保留。关闭窗口或切换来源时，未保存内容提供保存／放弃／取消。支持文档撤销／重做，历史最多 64 步并受约 32 MiB 源文本快照预算限制；聚焦文本框时 Ctrl+Z/Y 由文本框处理。

支持 UTF-8 和带 BOM 的 UTF-16 输入，保存为 UTF-8；其他编码需先转换。文件上限 32 MiB。ASS 事件的 Format 要求 Text 为最后一列，异常结构报错而不静默改写。SRT 位置扩展暂不接受。SRT 保留毫秒精度；转成 ASS 或修改 ASS 时间时按百分之一秒取整。SRT→ASS 使用视频尺寸建立默认脚本，保留常用粗体／斜体／下划线／字体 HTML 标记；复杂嵌套标记不保证完全等效。ASS→SRT 先提示样式／位置／动画等损失，跳过注释和纯绘图事件，并过滤混合事件里的绘图指令；不对图形做 OCR。

选中视频内嵌文字字幕后，在同一字幕菜单点击“提取字幕（ASS）”。默认保存为 ASS，原 ASS／SSA 轨采用复制而非先转 SRT，其他文字轨转换成 ASS；始终按选中的字幕轨序号提取。提取不覆盖已有目标文件，成功后挂载并打开编辑器。图像字幕不支持此入口；MKV 中的字体附件不随字幕提取，缺失字体仍需自行安装。原文件中已经缺失的字体／样式也无法由提取恢复。

“使用草稿烧录…”会把当前草稿固定成独立 ASS 快照，送入已有视频编辑窗口的烧录设置；在视频编辑窗口选择导出。后续字幕编辑、保存或关闭字幕窗口不会改变该快照，重新点击才更新设置。导出任务持有快照直至完成；视频工程保存会嵌入这份烧录字幕。此按钮不直接覆盖原视频。

### ASS/SSA/SRT/WebVTT subtitle editing and embedded extraction

After selecting a local ASS/SSA/SRT/WebVTT subtitle during video playback, choose **Subtitle editor** beside the selector or in its menu. The independent player starts at the current video position and audio track. Reopening the same subtitle focuses its existing window. Loading another file handles unsaved changes first and prevents two windows editing the same file.

Edits affect a memory draft. Single-click a list cue to select it and reveal it in the timeline; double-click also seeks the video. Playback following selects the current cue. Drag blocks to move cues and their edges to change timing; enlarged edge targets highlight and show a horizontal resize cursor on hover. Selected cues move together; overlaps occupy separate rows and are never removed automatically. Wheel zooms; the full-width navigation bar below the timeline pans without seeking, with reserved spacing above it. Left/Right steps while paused, and Ctrl+Left/Right pans five frames. Snapping uses actual frame timestamps when available. Waveforms use the selected audio track and rebuild when it changes.

The **Landscape layout / Portrait layout** tabs at the left of the subtitle file information row switch layouts at any time, initially following the video orientation. Portrait layout puts video and playback controls in a resizable left column, with properties, timeline and list on the right. Landscape layout keeps video above the timeline and list, with properties beside it. Switching preserves the draft, selection, playback position and timeline zoom.

Add, duplicate, delete, find/replace, shift selected cues, split at the playhead and merge adjacent cues. Splitting duplicates the text into both halves for manual redistribution. Merging requires matching styles and event properties. Relative animation/karaoke times are not rewritten. Edit exact times or set them from the playhead, and inspect duration, characters and CPS. A fixed group centered above the timeline contains Play current cue (range icon), Play/Pause, Previous frame, Next frame and Volume. Batch-shift controls wrap together when space is limited. The range icon sits to the left of Play and is disabled without a valid selected cue. Audio/language and current/total time remain centered below the video.

ASS supports named style selection/duplication, font, size, colors, outline, shadow, alignment and margins, plus cue layer, actor, effect and comment properties. The text editor retains raw ASS tags; newlines map to `\N`. Advanced positioning, drawing, animation and karaoke have no visual authoring tools yet. SRT keeps text/timing editing; save as ASS to enable ASS features. Original script metadata, unknown sections and untouched fields survive ASS editing; player styling and automatic wrapping are not written back.

**Save / Ctrl+S** replaces the current file after checking external changes and keeping one `.bak`; a sibling temporary file is written before replacement. Failed saves retain the draft. Successful **Save as** changes the current editing target and preserves the old file. Closing/loading offers Save, Discard or Cancel. Document undo/redo has at most 64 steps and about a 32 MiB source-snapshot budget; a focused text box handles its own Ctrl+Z/Y.

Input accepts UTF-8 or BOM-marked UTF-16; output is UTF-8, with a 32 MiB input limit. Other encodings require conversion first. ASS events require Text as the final Format column; SRT placement extensions are currently rejected. SRT retains milliseconds; ASS conversion/edited times round to centiseconds. SRT→ASS uses video dimensions and basic HTML font/emphasis tags; complex nesting may differ. ASS→SRT warns about lost features, excludes comments/drawing-only cues and removes drawing commands from mixed cues without OCR.

When an embedded text track is selected, **Extract subtitles (ASS)** appears in the same menu. Native ASS/SSA is copied directly; other text codecs convert to ASS. The selected subtitle ordinal is used, existing destination files are preserved, and successful extraction loads the file and opens its editor. Bitmap subtitles and extracting MKV font attachments are unsupported; install missing fonts separately.

**Burn in draft…** sends an immutable ASS snapshot into the existing video editor for export. Further edits, saves or closing the subtitle editor do not alter that snapshot; sending again updates it. Export tasks retain their snapshot and saved video projects embed it. This action does not directly replace the original video.

### 字幕格式与保存范围 / Subtitle formats and saving

| 格式 | 原格式编辑与保存 | 另存转换 |
|---|---|---|
| ASS | 保留脚本、样式及事件字段 | ASS、SRT、WebVTT |
| SSA | 保留 V4 样式、Marked、十进制颜色及旧对齐编号 | SSA、ASS、SRT、WebVTT |
| SRT | 保留文字和毫秒时间 | SRT、ASS、WebVTT |
| WebVTT | 保留文件头、STYLE／REGION／NOTE、cue 标识、时间设置及原始文本标签 | WebVTT、ASS、SRT |

加载时优先按内容识别以上四种格式，文件后缀用于无法明确识别时的辅助判断；在加载对话框中可选择所有文件。保存当前文件保持实际识别格式，不因后缀不匹配而暗中转换；另存的默认文件名使用实际格式后缀。其他格式转为旧版 SSA 暂不支持，导入 LRC／TTML／SAMI／MicroDVD 及图像字幕 OCR 也尚未实现。

WebVTT 的 cue 标识与时间行设置可在属性区编辑。修改文字或时间不删除其 CSS、区域或备注；原时间未改时保留短时间写法及时间行格式。拆分／复制出来的新 cue 不沿用旧标识，时间设置仍保留。WebVTT／SRT 中输入的字面 `\N` 按普通文字保存，真实换行仍为文本换行。

WebVTT 原格式保存与预览效果不同：当前 ASS 播放管线仅近似呈现文字和常用强调，不完整还原 CSS、REGION、定位、说话人和逐词时间效果。转为 ASS／SRT 时先提示这些信息的损失，不悄悄覆盖源文件；转成 WebVTT 时，SRT 的字体／删除线等非通用标签会移除并保留文字，相关情况提示损失。SSA 预览将旧对齐编号映射成相同方位，保留写回原编号；描边和阴影共用 BackColour，TertiaryColour 原值保留，AlphaLevel 及阴影透明度按 SSA／libass 兼容规则预览。

同目录字幕发现扩展到 ASS／SSA／SRT／WebVTT；精确同名只有一份时优先加载，存在多种精确同名格式而无法唯一匹配时由下拉菜单手动选择。播放菜单同时保留“提取字幕（ASS）”和“按原格式提取字幕”。后者对内嵌 ASS、SRT 和 WebVTT 使用编码复制；ASS／SSA 家族的流以 ASS 封装提取，但读取时识别实际 V4／V4+ 结构。MP4 mov_text 等当前没有已支持独立原格式的轨道禁用原格式提取，可使用默认 ASS。提取不恢复容器／上游已经丢弃的 WebVTT 全局样式或备注，也不提取字体附件。

| Format | Native editing/saving | Save-as conversion |
|---|---|---|
| ASS | Retains script, styles and event fields | ASS, SRT, WebVTT |
| SSA | Retains V4 styles, Marked, decimal colors and legacy alignment | SSA, ASS, SRT, WebVTT |
| SRT | Text and millisecond timing | SRT, ASS, WebVTT |
| WebVTT | Header, STYLE/REGION/NOTE, cue ids/settings and original text tags | WebVTT, ASS, SRT |

Loading identifies these formats from content before using the extension as fallback; the dialog also allows all files. Saving the current file retains its detected format despite a mismatched extension; Save as defaults to the detected format's extension. Converting other formats into legacy SSA, importing LRC/TTML/SAMI/MicroDVD and bitmap OCR remain unsupported.

WebVTT cue identifiers and timing settings have editable fields. Text/timing edits retain CSS, regions and notes; unchanged timing keeps original shorthand and spacing. Split/duplicated cues clear the copied identifier while keeping settings. Literal `\N` in WebVTT/SRT is saved as text rather than turned into a newline.

Native WebVTT preservation does not imply browser-equivalent rendering. The ASS preview provides text/basic emphasis without fully reproducing CSS, REGION, placement, voices or word timing. Conversions warn before losing source features and preserve the source file. SRT font/strikeout markup unsupported by WebVTT is removed while retaining text, with a loss warning. SSA preview maps old alignment codes, uses shared BackColour for outline/shadow, preserves TertiaryColour on disk and follows SSA/libass AlphaLevel/shadow compatibility.

Sidecar discovery now includes all four formats. A unique exact-name file takes priority; ambiguous exact-name formats require manual selection unless language matching is unique. The menu retains default ASS extraction and adds original-format extraction. ASS-family, SRT and WebVTT codecs copy without transcoding; ASS-family streams use ASS packaging and their actual V4/V4+ structure is detected when loaded. Unsupported standalone codecs such as mov_text disable original-format extraction but can convert to ASS. Extraction cannot restore global WebVTT metadata already discarded upstream or recover font attachments.

Format references: [WebVTT specification](https://www.w3.org/TR/webvtt1/) and [libass SSA compatibility handling](https://github.com/libass/libass/blob/master/libass/ass.c).

### 字幕编辑的选中与固定工具栏 / Subtitle selection and fixed controls

- 列表和时间轴共用选中状态：单击列表字幕会高亮对应时间轴字幕，并在需要时横向定位、滚动到对应重叠行；单击时间轴字幕会选中并滚动显示对应列表行。若当前查找条件隐藏了该字幕，会清除查找条件再显示。Ctrl 可添加／取消多选，两边同步；取消最后一项后不保留隐藏的当前选中。
- 单击只选中，不改变视频播放位置。双击列表字幕会跳到该句起点，同时显示对应时间轴范围；双击时间轴字幕同样定位视频。浏览定位保留时间轴缩放，并在视频首尾钳制范围。编辑窗口初始选中当前时间命中的字幕或邻近下一句。
- 开启跟随播放时，在播放进入另一条字幕时同步两边选中及列表浏览，时间轴可保留手动浏览范围；多选和文字输入期间不抢占手动选择。手动选中不会在同一句播放期间被逐帧覆盖。
- 音量使用与普通视频播放相同的按钮和竖向滑条弹窗，按钮可静音／恢复。当前时间／总时长紧接音轨按钮，与播放控件同排；窄窗口可截短显示，悬停查看全文。
- “选中字幕批量平移”的毫秒值、提前和延后按钮固定在列表工具栏的“跟随播放”后方，不在右侧滚动属性中。没有选中字幕时禁用；窄窗口可在工具栏内换行，字幕列表本体仍单独滚动。

- The list and timeline share selection. Clicking a list row highlights/reveals its cue horizontally and in overlapping lanes; clicking a timeline cue selects and scrolls to its list row. A search hiding that cue is cleared. Ctrl toggles multiple selection in both views; deselecting the last cue leaves no hidden active selection.
- Single-click selects without seeking video. Double-click seeks to the cue start and reveals its timeline range. Navigation preserves timeline zoom and clamps at video boundaries. A new editor selects the cue at the current position or the nearby next cue.
- Follow playback synchronizes selection and list navigation when entering another cue; the timeline can retain its manually browsed range. Multiple selection and text input keep priority; manual selection is not overwritten every frame during the same playing cue.
- Volume reuses normal playback's button and vertical popup, including mute/restore. Current/total time follows the audio-track menu in the same row, with truncation and a full hover label when narrow.
- Selected-cue shift amount and earlier/later buttons remain in the fixed list toolbar after Follow playback, outside the scrolling properties. They disable without selection and can wrap within the toolbar on narrow windows while the list itself scrolls separately.

### 编辑器独立统一预览样式 / Independent editor preview style

SRT／WebVTT 的顶部工具栏提供“统一预览样式”，位于“使用草稿烧录…”之前。点击打开独立设置窗口，可调整字体、字号、颜色、描边、X／Y 阴影、上下位置及边距；修改后短暂延迟预览，应用后保存，取消或关闭恢复已保存值。重置先恢复默认草稿，需点击应用才记录。重复点击按钮聚焦已打开的设置窗口，不丢弃当前草稿。

编辑器使用自己的 `subtitle_editor_preview_style.json`，与普通播放的 `subtitle_style.json` 分开；初始读取编辑器自己的配置或默认值，不复制普通播放偏好。应用后供其他没有未应用草稿的编辑器预览同步使用，重启后仍恢复。ASS／SSA 保留文档自身样式，不应用这套统一预览。编辑预览仅换行而不把原字幕拆成新的计时事件，字号和边距沿用字幕脚本单位。

这套设置只影响预览，不使字幕文档变为未保存状态，不改变原文、时间或文档样式；保存、另存转换及交给视频编辑器的草稿烧录快照仍按字幕文档生成，不自动烘入预览偏好。需要改变最终 ASS 样式时，应另存 ASS 后编辑文档样式。

交互按用户最终确认保持：单击列表字幕，选中并将对应字幕移入时间轴视野，保留缩放，不跳视频；双击在此基础上定位视频到该句起点。时间轴点击选中对应列表字幕。跟随播放可同步选中和列表浏览，不强制把手动浏览的时间轴拉回播放位置。

For SRT/WebVTT, **Unified preview style** appears in the top toolbar before **Burn in draft…**. It opens independent font, size, color, outline, signed X/Y shadow, position and margin settings. Changes preview after a short delay; Apply saves, Cancel/close restores saved values, and Reset changes only the draft until applied. Reopening focuses the existing window without discarding edits.

The editor stores `subtitle_editor_preview_style.json` independently of normal playback's `subtitle_style.json`. It loads its own saved/default values, never copies normal playback preferences, and retains applied settings across restarts. Other editor previews without pending local changes receive applied values. ASS/SSA keep native document styles. Preview wrapping does not split source cues into new timed events; sizes/margins use subtitle script units.

Preview settings do not mark the subtitle document dirty or alter its text, timing or styles. Save, conversion and burn-in draft handoff still derive from the document, without automatically baking preview preferences. To change final ASS styling, save as ASS and edit document styles.

Final interaction: single-clicking a list cue selects/reveals it in the timeline at the existing zoom without seeking video; double-clicking seeks video to its start. Timeline clicks select the matching list cue. Follow playback may update selection/list navigation without forcing the manually browsed timeline back to playback.

## 录屏 / Screen recording

- 从“文件 → 录屏”进入，默认选择指定区域。十字光标悬停吸附可见窗口，左键单击确认；按住左键拖动则创建矩形，松开确认。右键取消本次选择并恢复原区域；录制前按 Esc 会关闭区域与整个工具条，包括正在画框和已经确认的状态。拖动超过系统容差（至少 4 个物理像素）后暂停吸附，支持四方向与负坐标，最小 16×16，限制在虚拟桌面范围内。
- 确认后，鼠标进入区域内部显示四向移动箭头，按住左键可整体移动区域而不改变尺寸。四角手柄同时改变宽高，上下边中点只改高度，左右边中点只改宽度；上边中段仍可移动，Ctrl+拖动上边调整顶部。拖动、缩放和输入数值保持同步。来源窗口随后移动不会自动跟随区域。
- 工具条采用与主程序相同的界面控件和主题：区域／屏幕选择后是圆圈十字“重新选择区域”，再后是主窗口同款设置图标，然后 X、Y、`宽 × 高 px`、红圆开始／方块停止和 × 退出。设置图标打开独立录屏设置窗口，记录参数与快捷键。单屏显示全屏，多屏提供各屏及全部屏幕。工具条优先置于选区下方，不足时尝试上方和左右，避开屏幕外、多屏空隙及任务栏；没有外围空间时才放入屏幕内。
- 设置包含 1–60 fps、质量、MKV／MP4、鼠标指针、开始前最小化、两个快捷键，以及独立的录屏历史上限（20–50 条，默认 20），可用数值框或同款设置滑条调整。保存跨重启记住，取消放弃未保存修改。默认停止 Ctrl+Shift+F9，进入指定区域 Ctrl+Shift+F8，可通过 Ctrl／Alt／Shift 与功能键或字母数字修改；F12 保留，字母／数字需 Ctrl 或 Alt，两个操作不能相同。系统注册检查全局占用，其他程序的局部快捷键仍可能相同。
- 点击开始先检查 FFmpeg／FFprobe；缺失列出组件，确定后打开第三方支持，不自动下载安装。通过检查后写入系统配置的用户下载目录，以 `recording-<纳秒时间戳>.mkv/mp4` 命名，不弹保存对话框、不覆盖已有文件。开始前有 3 秒准备时间；当前中央大数字在用户环境中未能显示，暂不依赖数字判断状态。可以用停止按钮或配置的停止快捷键取消。
- 工具条显示停止快捷键、时间与文件体积。全屏／全部屏幕开始后隐藏，区域录制保留停止按钮；录制时区域、参数及快捷键锁定，线框与区域内部不拦截鼠标。结束后恢复工具条，下方显示本次视频地址及编辑／打开目录按钮；不提供“完成”，用 × 退出。录制中 × 会先正常收尾再退出；Esc 不用于中止正在进行的录制。
- 重新进入录屏或重新选区时，不在下方展示上次的保存结果。工具条“历史记录”打开独立录屏历史窗口，与主程序“最近打开”分开保存和计数，按成功录制顺序保留最新记录。选中存在的文件可进入视频编辑或打开目录；不存在的标红并禁用这两个操作。红色“移除记录”只移除条目，任何情况下都不删除视频文件；调整上限淘汰旧记录也不删除视频。
- 绿色线框、选区交互层、工具条和屏幕选择菜单均设置捕获排除，要求 Windows 10 2004+；设置失败禁止开始。实际输出仍需在目标捕获环境核对。可录制默认播放设备的系统声音，不采集麦克风，不支持暂停续录；受保护画面、独占全屏、安全桌面与远程／特殊合成环境不保证兼容。
- English: Open File → Screen recording to select a region. The crosshair snaps to visible windows; click to confirm or hold the left button to draw a rectangle and release. Right-click cancels the current selection; before recording, Esc closes both region and toolbar, whether drawing or already confirmed. Movement beyond the system tolerance (at least four physical pixels) switches to drawing. All directions and negative origins are supported, with a 16×16 minimum inside the virtual desktop.
- Inside a confirmed region the pointer becomes a four-way move cursor; left-drag to move without resizing. Corner handles change both dimensions, top/bottom middle handles change height, left/right middle handles change width. The top middle segment still moves the region; Ctrl+drag the top edge adjusts upward. Fields and geometry stay synchronized. The region does not follow subsequent movement of its original window.
- The toolbar uses the app’s controls/theme: screen selector, circled cross to reselect, the same settings icon as the main window, X/Y, width × height px, red start circle/stop square and × close. Settings opens separately. Choose Fullscreen for one monitor or individual/all screens for multiple monitors. Placement prefers below, then above or beside the region within monitor work areas, falling inside a screen only if no outside space exists.
- Settings retain 1–60 fps, quality, MKV/MP4, pointer capture, minimization, two shortcuts and a separate recording history limit (20–50, default 20), adjusted with a number field or the standard settings slider. Save persists across restarts; Cancel discards unsaved edits. Default shortcuts are Ctrl+Shift+F9 to stop and Ctrl+Shift+F8 to select a region. Configure Ctrl/Alt/Shift with function keys or letters/digits; F12 is reserved and letters/digits require Ctrl or Alt. Bindings must differ. Registration detects global conflicts but cannot detect another app’s local shortcuts.
- Start checks FFmpeg/FFprobe; confirming missing components opens Third-party Support without automatically downloading. Output uses the configured Downloads folder and nanosecond timestamp filenames, without a save dialog or overwriting. Recording has a three-second preparation period. The central digits have not appeared in the reported environment, so do not rely on them to judge recording state. Cancel with Stop or its shortcut.
- The toolbar shows the stop shortcut, elapsed time and file size. Fullscreen/all-screen recording hides it; region recording retains Stop. Recording locks geometry/settings/shortcuts and makes the border/interior click-through. Completion restores the toolbar and shows the current output path plus editor/folder actions below it. There is no Done button; × exits, gracefully finalizing first if recording. Esc does not stop an active recording.
- Reopening recording or selecting a new region hides the previous completion result. History opens a separate recording history window, with storage/count independent of Recent files. It keeps the newest successful recordings. Select an existing file to edit or open its folder. Missing files appear red and disable these actions. The red Remove entry button only removes metadata; it never deletes a video. Trimming to a new limit likewise keeps all video files.
- Border, selection interaction layer, toolbar and screen menu use capture exclusion, requiring Windows 10 version 2004+. Failure disables starting; recorded output still needs checking in the target environment. System audio from the default playback device is supported; microphone capture and pause/resume are not. Protected content, exclusive fullscreen, secure desktops and remote/unusual composition environments are not guaranteed compatible.

- 工具条录制按钮前的麦克风形状按钮控制系统声音，默认开启；点击关闭后显示红色禁止标记，保存并记住选择。它不访问麦克风；录制过程中不可切换。采集默认播放设备（多声道转立体声），没有播放内容时保留静音；受保护音频可能无法捕获，更换系统播放设备需停止后重新开始。初始化或中途失败会提示，不悄悄改为无声录制。
- 工具条输入框和标准下拉框保持较矮并与图标垂直居中；停止提示简写为“停止: 快捷键”，使用正常字号。保存结果紧接工具条，不加分割线。透明圆角及投影由程序绘制。
- The microphone-shaped button before Record controls system playback audio, enabled by default and remembered. A red prohibition mark means disabled. It never accesses the microphone and is locked during recording. The default output device is captured, downmixed to stereo, with silence during idle playback. Protected audio may be unavailable; stop and restart after changing output devices. Initialization/capture failures are reported rather than silently falling back to silent video.
- Compact fields and the standard dropdown are vertically centered beside icons. Stop/status text uses the normal font size; saved results follow directly without a divider. Rounded transparent corners and shadows are drawn by the app.

- 工具条按第一排控件实际宽度自动收紧，关闭图标直接绘制圆头矢量交叉线，适应当前显示缩放。长保存路径在原宽度内横向浏览。
- The toolbar fits the actual width of its first row. Close uses directly drawn vector strokes with rounded ends at the current display scale. Long saved paths scroll horizontally within that width.

## 错误日志 / Error log

- 共用日志为 `%APPDATA%\UnityYAMLViewer\error.log`，UTF-8。每条带本机时间、软件版本、进程及模块；达到约 2 MiB 时轮换为 `error.log.1`，只保留一份旧日志。可用于检查录屏阶段、后台任务失败、运行库通过标准日志接口报告的警告／错误及程序 panic。
- 录屏错误弹窗置顶并保留至手动关闭，可复制详情或点击“打开日志”；关闭详情后，工具条仍保留简短错误及“查看错误详情”。日志记录声音开关／设备格式、失败接口及错误码、FFmpeg 参数／退出状态／错误输出和目标文件状态，不记录音视频内容。写入日志失败时保留原始录屏错误并提示写入失败。
- Shared diagnostics are saved as UTF-8 at `%APPDATA%\UnityYAMLViewer\error.log`, with local timestamps, app version, process and module. At roughly 2 MiB, the log rotates to `error.log.1`, retaining one previous file. It covers recording stages, failed background tasks, warnings/errors reported through the runtime logging interface, and Rust panics.
- Recording errors remain in an always-on-top dialog until dismissed, with Copy error and Open log actions. The toolbar retains a short error and a details button. Diagnostics include audio mode/format, failed API/error code, FFmpeg arguments/exit status/stderr and output-file status, without audio/video content. Log-write failure preserves the original recording error and adds a write-failure message.

- 字幕编辑时间轴下方使用与视频编辑相同的圆角比例滑块：长度代表当前可见时间范围，拖动滑块平移视野，点击空白轨道将滑块移到该处；不改变播放位置。原蓝色填充和条内时间文字移除，波形与浏览条之间仍保留间距。
- The subtitle timeline uses the same rounded proportional pan thumb as video editing. Its length represents the visible time range; dragging pans the view and clicking the empty track centers the thumb there, without seeking playback. The blue fill and in-bar time text are removed, with spacing retained beneath the waveform.
