## Batch PNG optimization

`pngopt [-all] <file-or-directory> [...]`

Scans only direct folder entries and processes duplicate paths once. Removes iTXt in place. Alpha is removed only from static 32-bit RGBA PNGs whose pixels all have alpha 255.

| Options | |
| --- | --- |
| `-all` | Include subdirectories, skipping symbolic links and junctions |
| `-h, --help` | Show help |
| `--` | Treat remaining arguments as paths |

### PowerShell examples

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

Result status: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## Batch nine-slice compaction

`slice9 <file-or-directory> [...] [options]`

Supports PNG, TGA, BMP and static WebP. Smaller results replace originals without backups. Supported single-Sprite .meta files can be updated. Use `--dry-run` to inspect plans first.

| Options | |
| --- | --- |
| `-r, --recursive` | Include subdirectories |
| `--preview` | Open up to 10 editable previews without automatic writes |
| `--dry-run` | Plan without writing files |
| `--center WxH` | Retained Stretch center, default 4x4 |
| `--repeat-count XxY` | Retained Repeat cycles, default 1x1 |
| `--direction auto / both / horizontal / vertical` | Compaction direction |
| `--mode auto / stretch / repeat` | Compaction treatment |
| `--no-preserve-edges` | Allow trimming transparent margins |
| `--no-align4` | Disable alignment to multiples of four |
| `--report FILE` | Save an additional JSON report to a new file |
| `-h, --help` | Show help |
| `--` | Treat remaining arguments as paths |

### PowerShell examples

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## Opening files

Quote each path containing spaces or special characters.

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## Results and exit codes

Both commands print per-file JSON. Exit codes are 0 for no failures, 1 for file failures and 2 for argument or report errors. In PowerShell, use `Start-Process -Wait -PassThru` to wait and read the exit code.
