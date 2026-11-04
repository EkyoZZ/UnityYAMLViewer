## 批量优化 PNG

`pngopt [-all] <file-or-directory> [...]`

只扫描目录当前层，重复路径只处理一次。原位移除 iTXt。仅当静态 32 位 RGBA PNG 的所有像素 Alpha 均为 255 时移除 Alpha。

| 参数 | |
| --- | --- |
| `-all` | 递归处理子目录，跳过符号链接和目录联接 |
| `-h, --help` | 显示帮助 |
| `--` | 后续参数都按路径处理 |

### PowerShell 示例

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

结果状态: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## 批量九宫格精简

`slice9 <file-or-directory> [...] [options]`

支持 PNG、TGA、BMP 和静态 WebP。有收益时原位替换，无备份。支持的单 Sprite 可同步更新 .meta。建议先用 `--dry-run` 检查。

| 参数 | |
| --- | --- |
| `-r, --recursive` | 包含子目录 |
| `--preview` | 打开最多 10 个可编辑预览，不自动写入 |
| `--dry-run` | 只计算，不写文件 |
| `--center WxH` | 拉伸保留中心，默认 4x4 |
| `--repeat-count XxY` | 重复保留倍数，默认 1x1 |
| `--direction auto / both / horizontal / vertical` | 指定精简方向 |
| `--mode auto / stretch / repeat` | 指定精简方式 |
| `--no-preserve-edges` | 允许裁掉透明外边缘 |
| `--no-align4` | 关闭 4 倍数对齐 |
| `--report FILE` | 另存 JSON，要求目标文件不存在 |
| `-h, --help` | 显示帮助 |
| `--` | 后续参数都按路径处理 |

### PowerShell 示例

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## 打开文件

路径含空格或特殊字符时，逐个加引号。

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## 结果与退出码

两条指令均输出逐文件 JSON。退出码 0 表示无失败，1 表示有文件失败，2 表示参数或报告错误。PowerShell 中可用 `Start-Process -Wait -PassThru` 等待并读取退出码。
