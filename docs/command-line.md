# 命令行参考 / Command-line reference

[返回首页 / Back to README](../README.md) · [使用指南 / User guide](user-guide.md)

## 命令行图片精简 / Command-line image compaction

主窗口顶部“格式支持”右侧的“指令查看”打开独立用法窗口，可复制英文指令说明；文件菜单不重复提供入口。也可使用 `--help` 或 `slice9 --help`。默认命令行无窗口执行，不转发给已打开的 GUI；--preview 则打开或转发到 GUI。

The **Commands** button beside **Format Support** opens a separate reference window with copyable English syntax and examples. It is not duplicated in the File menu. `--help` and `slice9 --help` show the same reference. CLI jobs run headlessly, independently of the GUI instance, except --preview which opens or forwards to the GUI.

```powershell
# 单张 / Single image
.\unity_yaml_viewer.exe slice9 "button.png"
# 多张 / Multiple images
.\unity_yaml_viewer.exe slice9 "a.png" "b.png"
# 当前目录层 / Directory, immediate files
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI"
# 子目录 / Recursive
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r
# 只计算，不修改 / Plan only
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r --dry-run
# 自定义中心、报告 / Custom center and report
.\unity_yaml_viewer.exe slice9 "button.png" --center 8x4 --report "result.json"
```

默认会**原位覆盖，不创建备份**：精确 RGBA 自动比较方向和 Stretch / Repeat，中心 4×4，循环倍数 1×1，保留透明边缘并对齐 4 倍数。仅输出面积减少时替换；无收益保持图片和 meta 不变。有可读受支持的单 Sprite `.meta` 时同步 Border / Full Rect 并保留 GUID；缺失 / 不可读时仅处理图片且不创建 meta，可读但不支持则该图片失败。补白按两侧分配、奇数多加右 / 下，不计入 Border。单张失败继续批次，路径去重，跳过符号链接和不支持文件。仅支持 PNG / TGA / BMP / 静态 WebP，动画报错。

Defaults **overwrite in place without backups**: exact RGBA automatic axis and Stretch / Repeat selection, retained center 4×4, cycle multiples 1×1, transparent-edge preservation and alignment to four. Only smaller output areas are written; no savings leaves image and meta unchanged. Readable supported single-Sprite metadata gets Border / Full Rect updates preserving GUID. Missing/unreadable metadata is not created; readable unsupported metadata fails that image. Balanced padding, with odd extras right/bottom, is excluded from Border. Processing continues after individual failures, deduplicates paths and skips symbolic links/unsupported files. Supported formats: PNG / TGA / BMP / static WebP; animations fail.

| 参数 / Option | 用途 / Behavior |
|---|---|
| `-r`, `--recursive` | 包含子目录 / Include subdirectories |
| `--dry-run` | 不写图片或 meta / No image or meta writes |
| `--center WxH` | 拉伸中心尺寸 / Stretch retained center |
| `--repeat-count XxY` | 重复保留倍数 / Repeat retained cycles |
| `--direction auto\|both\|horizontal\|vertical` | 指定方向 / Axis constraint |
| `--mode auto\|stretch\|repeat` | 指定处理方式 / Treatment constraint |
| `--no-preserve-edges` | 允许裁透明外圈 / Allow transparent-margin trimming |
| `--no-align4` | 取消 4 倍数对齐 / Disable alignment |
| `--report FILE` | 另写 JSON 报告，仅新文件 / Also write JSON; refuses existing files |
| `--` | 后续参数均按路径 / Remaining arguments are paths |

JSON 标准输出包含逐文件结果、原因、尺寸、节省面积、补白、Border、meta 写入状态与分类计数；退出码 0 无失败，1 文件失败，2 参数或报告错误。报告不会覆盖已有文件，写入失败可能发生在部分资源已处理之后，批次不提供整体回滚。Windows GUI EXE 在 PowerShell 中需要可靠等待 / 获取退出码时使用：

JSON stdout includes per-file results, reasons, sizes, saved area, padding, Border, metadata status and summary counts. Exit codes: 0 no failures, 1 file failures, 2 argument/report errors. Reports never overwrite existing files; report writes can fail after some assets were processed. Batches are not atomic. To reliably wait for the Windows GUI executable and retrieve its exit code in PowerShell:

```powershell
$p = Start-Process -FilePath ".\unity_yaml_viewer.exe" -ArgumentList 'slice9 "D:\Project\Assets\UI" -r --dry-run' -Wait -PassThru -NoNewWindow -RedirectStandardOutput "slice9-result.json" -RedirectStandardError "slice9-errors.txt"
$p.ExitCode
```
### 打开文件与路径引号 / Opening files and quoting paths

```powershell
# 单个 / Single
.\unity_yaml_viewer.exe a.png
# 批量 / Multiple
.\unity_yaml_viewer.exe a.png b.prefab c.mat
# 有空格：每个路径分别加引号 / Quote each path with spaces
.\unity_yaml_viewer.exe "D:\UI Images\a.png" "D:\My Assets\b.prefab"
```

路径不含空格或命令行特殊字符时，可不加引号；包含空格或 shell 特殊字符时应加引号。示例统一使用双引号方便复制，但转义规则由调用的 shell 决定；PowerShell 路径包含字面量 `$` 或反引号时，优先使用单引号，避免双引号内插值。

Quotes are optional for simple paths without spaces or shell-special characters. Quote each path containing such characters separately. Examples use double quotes for copying, but escaping is shell-specific; in PowerShell, prefer single quotes for literal paths containing `$` or backticks to avoid interpolation.

### 自动裁剪预览 / Automatic compaction preview

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
.\unity_yaml_viewer.exe slice9 "D:\Project\Assets\UI" -r --preview
```

`--preview` 打开 GUI 页签和独立九宫格窗口，并执行自动识别，**不自动修改图片或 meta**。用户可修改所有原有设置、重新识别、另存为、确认原位替换或直接关闭；原本对不支持格式 / meta / 尚未完成计算的限制仍保留。

每次请求按路径去重，最多取前 10 张（规范化路径排序）；当前进程的预览总数最多 10 个，包括手动打开的窗口。已打开的预览直接唤醒，不重置用户设置；超出的图片不排队、不自动修改，提示未打开数量。关闭窗口后可再次调用打开更多。已有主窗口时转发到该窗口，主界面用多个页签，预览之间相互独立，切换页签不停止其他预览。

预览支持 `-r`、`--center`、`--repeat-count`、透明边缘和对齐选项，但启动时统一自动选择方向与处理方式；不能与 `--dry-run`、`--report` 或显式 `--direction` / `--mode` 一起使用，打开后仍可在窗口手动切换方向与处理方式。十张大图可能占用较多内存。

`--preview` opens GUI tabs and native nine-slice windows with automatic detection, **without automatically changing images or metadata**. All editor controls remain available: adjust settings, detect again, save as, confirm replacement or close without changes. Existing format/metadata and calculation-state restrictions still apply.

Paths are deduplicated and sorted; each request selects at most the first 10. A process can have at most 10 preview windows, including manually opened ones. Existing previews are focused without resetting edits. Excess images are reported and neither queued nor modified; close windows and invoke again to open more. Requests forward to an existing main window. Files use tabs in the main window; their independent previews keep updating across tab switches.

Preview supports recursion, retained center/cycles, edge and alignment options, but initially chooses axes and treatment automatically. It cannot combine with `--dry-run`, `--report` or explicit `--direction` / `--mode`; those controls remain editable inside the window. Ten large images can consume substantial memory.

九宫格 4 倍数对齐：精简方向在中心循环区保留或补齐像素；未精简方向只在外侧补透明，Border 为 0。外补奇数像素多出部分放右 / 下，保留透明边缘选项继续生效。

Nine-slice alignment to 4 retains or repeats pixels in the compacted center. Inactive axes receive transparent outer padding with zero Border. Odd extra pixels go right / bottom; the preserve-transparent-edges option still applies.

Stretch 中心仅补齐所差的 1 / 2 / 3 像素；Repeat 仅补完整周期，若无法与固定边框共同对齐则外补透明。

Stretch adds only the missing 1 / 2 / 3 center pixels. Repeat adds whole cycles; if fixed borders prevent alignment, transparent outer padding is used.

Repeat 对齐后该方向尺寸必须小于原尺寸；否则保留该方向原像素与尺寸，不强制对齐到 4 的倍数。

After Repeat alignment, each compacted axis must be smaller than its original size. Otherwise its original pixels and size are retained, without forcing alignment to 4.

Stretch 为 4 倍数对齐多保留的 1～3 像素计入右 / 下 Border，可拉伸中心仍为设定值（默认 4 像素）。

Stretch assigns the extra 1–3 alignment pixels to the right / bottom Border; the stretchable center stays at its configured size (4 pixels by default).
