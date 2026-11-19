# Isometric Tile Preview

Photoshop UXP 等距 Tile 预览面板。绘画保留在 PS 文档中，面板显示菱形循环、帮助建立网格文档并导出切片。需要 Photoshop 24.4 或更高版本。

## 安装与打开

默认通过独立 EXE 安装器安装：关闭 Photoshop，运行安装器并点击“安装／更新”，完成后重新启动 PS。再次运行安装器可卸载，无需先卸载再更新。安装器内嵌并部署以下运行资源，源码调试时也可手动复制：

```text
%APPDATA%\Adobe\UXP\Plugins\External\unityyamlviewer.isometric-preview\
    manifest.json
    index.html
    main.js
    png.js
    tile-dark@1x.png
    tile-light@1x.png
    tile-dark.png
    tile-dark@2x.png
    tile-light.png
    tile-light@2x.png
```

从“增效工具 → Isometric Tile Preview”选择“Tile 设置”或“Tile 预览”。安装器源码和构建说明位于 `installer/`；运行资源仍保留在本目录，由安装器内嵌部署。手动部署的卸载方式是关闭 PS 后移除此插件目录。

源码调试可在 Adobe UXP Developer Tool 中添加 `manifest.json` 并 Load；需要按 Adobe 指引开启开发者模式。常规交付只生成 EXE；`package.ps1` 默认不生成产物，仅显式传入 `-GenerateLegacyArchives` 时封装旧格式 CCX／手动 ZIP，本地 `dist/` 不纳入 Git。

## 使用

设置和预览为两个独立面板，可各自停靠、收起或拖到其他屏幕。设置中的“打开预览”会显示预览面板；“加载当前打开文档”会打开预览并重新读取画面。关闭请使用面板自身的 ×。

Settings and preview are separate panels that can be docked, collapsed or moved to another screen independently. Open Preview shows the preview; Load Current Open Document opens it and reloads the image. Close a panel using its own × button.

预览区支持悬停滚轮缩放，底部数值与滑动条同步，范围 0.1–10 倍，并可用1:1按钮恢复；设置面板独立滚动。

预览背景可选择透明棋盘格或自定义颜色（输入 `#RRGGBB`）。背景只影响面板显示，不写入文档或导出的 PNG；设置在本次面板会话中保留。

“显示辅助网格”可在预览上叠加黄色菱形格线，与 Slot 坐标标签独立控制；支持单 Tile、单文档网格和多文档 Slot，带高度模式标示地面格子，不改变源文档或导出的 PNG。

设置与预览面板均可调整大小，设置内容较多时纵向滚动。首次停靠位置与已有工作区由 Photoshop 管理，可拖动预览标签将其独立浮动。插件不提供脚本关闭或最大化。

Settings and preview panels are resizable, with vertical scrolling for longer settings content. Photoshop manages docking and saved workspaces and may override requested panel dimensions.

- **布局与缩放**：紧凑设置面板与可放大的预览面板分离，收起设置不停止独立预览；关闭预览暂停读取，重开保留本次会话的参数。
- **模式**：单 Tile 重复、单文档菱形网格、多文档 Slot 可以切换；前者预览当前文档，网格模式在一个文档中绘制整个循环单元。
- **子组＋整体回落**：在单文档模式开启后，设置子组列数／行数（须分别整除整体行列），再选中子组内任一 Slot。笔画边界会同步到其他同位子组，形成整体循环；预览网格以颜色区分各组并高亮当前组。支持如整体2×4分为两个2×2子组，也可设为1×1等更小子组。
- **多文档 Slot**：行、列各 1–8，最多 64 格，支持 2×2、3×3、4×4 快捷设置，其他排列手动填写。左侧格子和右侧可点击坐标相互对应；选格后从下拉菜单直接选择已打开的 PNG／PSD 文档。同文档可多格引用，空格留空，缩小排列保留隐藏格绑定。
- **转换为单文档**：将平面 Slot 排列生成一个新文档，每格为独立像素层，顶层附锁定的正片叠底网格，完成后切换到单文档预览。源文档保留，是否合并由用户决定。
- **当前层回落**：单文档可累计绑定多个绘画层，切换到已绑定层继续自动回落，解绑仅移除当前层。越界内容搬回，边界原位保留并同步到对边；支持子组联动，也可手动执行。
- **错位预览**：单文档网格和多文档 Slot 均可沿列、沿行或双向隔组错位，分别设置偏移格数；坐标对应来源格，仅影响预览排列。
- **单文档网格与切片**：新建带锁定辅助网格的透明文档，支持在大画布中心绘制及保留旧格位置扩展行列。预览、回落与切片使用同一区域，PNG可逐格导出。
- **地面与高度**：多文档使用共用地面宽高；平面文档缩放到地面尺寸，带高度文档保留像素尺寸并以底部中心对齐。带高度模式需让各 PSD 的地面尺寸一致，不同图片高度可以不同。
- **刷新**：预览显示时自动更新，关闭或收起后暂停，无需设置开关。重开会显示最新画面；“加载当前打开文档”可主动重新读取。目标为笔画完成后刷新，不保证绘制过程中持续更新。

Preview updates automatically while visible, pauses when closed or collapsed, and shows the latest image when reopened. No update toggle is required. Load Current Open Document forces a reload. Updates target completed strokes, rather than continuous live painting.

## 当前限制

- 转换使用源文档可见合成画面，按设定的单格尺寸读取为 8 位 sRGB；转换与PNG切片均保留菱形内部及辅助线覆盖的边界像素，辅助线颜色不写入结果。不保留源 PSD 内部图层、效果或高位深；空 Slot 保留空图层，转换后不联动源文档，暂不支持带高度排列。

- 预览为最长边至多 512 的 8 位 sRGB 图像，超过 16 个不同文档时降低采样尺寸，64 个不同文档为至多 256px；这是缩略像素预算，不包含 PNG/base64 和宿主内存。原始文档不缩小。
- 单文档 PNG 切片读取原始分辨率，输出 8 位 sRGB，每格边长须为 2–1024 的偶数；新建文档每边最多 8192、总像素最多 16777216。网格外区域不属于循环单元，会被遮罩。辅助层名称含网格信息，请勿重命名或合并；修改网格设定后需新建文档或恢复匹配设置。
- 为排除顶层辅助网格，读取时在 PS 模态范围临时隐藏网格并回滚可见性与锁定状态，不写入绘画内容；宿主忙碌、取消或读取失败时可能延后刷新，实际光标和闪烁效果受 PS 影响。
- 回落不接管原生画笔；限RGB 8位、可见未锁定、正常混合及不透明度／填充100%的单个普通像素层，不支持蒙版、效果、智能对象、组和跨文档处理。越界部分搬移后清空，同层重叠透明叠加；边界则复制RGBA、不重复叠加透明度，可读前一历史时按变化侧同步，否则按外缘向内复制。子组联动需要可读到笔触前历史；对边橡皮同步和画布外未记录笔迹恢复不支持。
- 勾选自动回落仅启用检测：paint事件后等待至少200ms，或检测新画笔历史并稳定至少200ms后处理；后者兼容未发出paint事件的输入设备。撤销、重做及切换文档／图层会跳过。既有笔迹可点手动按钮处理。回落可单独撤销一步，原笔触为前一步。
- 预览滚轮能否生效受 Photoshop 的焦点和滚轮偏好影响；若事件被宿主截走，可使用底部数值、滑动条和1:1按钮。
- Slot 绑定在本次面板会话中保留，关闭／重新加载插件后需重新设置；PSD 关闭后会提示重新绑定，不自动打开文件。支持已打开的 PS 文档，不要求预先保存为 PSD。
- 带高度排列按从后到前绘制，只支持底部中心锚点；不处理复杂穿插、真实深度和任意锚点。预览绘制数量限制为前后各 32 排、左右各 16 格，极端尺寸可能无法填满视区。
- 手动目录部署与 Creative Cloud 安装管理不同，不向 UXP 安装数据库注册；跨 PS 版本的支持情况受宿主影响。

开发检查可运行 `node check.cjs`、`node panel-check.cjs` 与 `node wrap-check.cjs`；模拟宿主不能替代 PS 实机验证。接口依据：[Adobe Imaging API](https://developer.adobe.com/photoshop/uxp/2022/ps-reference/media/imaging)。

面板折叠图标为透明底空心等距菱形，随 Photoshop 深浅主题切换，提供 100%／200% 像素资源；由 manifest.json 中 entrypoints[0].icons 配置。

自定义预览背景可点击色块打开 Photoshop 调色板；确认后更新预览背景，取消保留原色。

自动回落随图层绑定启用，无需全局开关。依次选中绘画层并点击“绑定当前图层”，可在当前工作文档中累计绑定；选中已绑定层时按钮显示“解绑当前图层”。未绑定层不自动处理，参考层可保持未绑定并锁定。解绑当前层即停止该层自动回落，其他已绑定层不受影响；手动回落仍作用于当前层。

收起设置或预览面板不清除绑定；插件仍在运行时自动回落继续检测，退出／重载插件后需重新绑定。切换到其他文档暂停；在另一文档点击绑定会切换工作文档。同一已保存文件重开时仅恢复相同图层ID，不按名称猜测；ID改变的层需重新绑定。详细扩展及绑定说明见工程使用指南。

Collapsing either panel preserves bindings and automatic wrapping while the plugin remains running. Closing or reloading the plugin clears the session bindings. Photoshop controls panel placement; drag the preview tab out to float it independently. UXP supports opening panels programmatically, but closing uses the host's panel controls.

Automatic wrapping keeps a list of bound drawing layers in the working document. Switching between bound layers continues wrapping; unbinding removes only the selected layer. Pausing retains the list. Grid expansion preserves existing slot positions and canvas margins without scaling artwork.

收笔后通过绘画事件及历史检测排队，连续笔触保留最早等待时间，防止一直延后。历史路径通常在收笔后的300–600ms内尝试执行；PS忙碌的模态拒绝会保留任务重试，其他错误标红。普通等待、历史检测和检查次数不常驻显示，不保证绘制过程中实时回绕。

单文档网格设置在插件本次运行期间按文档临时记忆，切回可恢复；退出／重载插件后清空，不写入 PSD。多文档 Slot 组合保持共用设置。

Single-document grid settings are remembered per document in plugin memory until exit or reload, without writing to PSD. Multi-document Slot compositions keep shared settings.

新建网格文档使用辅助图层；既有路径网格文档仍可读取，暂不支持扩展。

New grid documents use a helper layer. Existing path-grid documents remain readable but cannot be expanded.
