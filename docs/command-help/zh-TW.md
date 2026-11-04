## 批次最佳化 PNG

`pngopt [-all] <file-or-directory> [...]`

只掃描目前資料夾這一層，重複路徑只處理一次。就地移除 iTXt。僅在靜態 32 位元 RGBA PNG 的所有像素 Alpha 均為 255 時移除 Alpha。

| 參數 | |
| --- | --- |
| `-all` | 遞迴處理子資料夾，略過符號連結和目錄接合點 |
| `-h, --help` | 顯示說明 |
| `--` | 後續引數視為路徑 |

### PowerShell 範例

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

結果狀態: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## 批次九宮格精簡

`slice9 <file-or-directory> [...] [options]`

支援 PNG、TGA、BMP 和靜態 WebP。有效益時就地取代，無備份。支援的單一 Sprite 可同步更新 .meta。建議先以 `--dry-run` 檢查。

| 參數 | |
| --- | --- |
| `-r, --recursive` | 包含子資料夾 |
| `--preview` | 開啟最多 10 個可編輯預覽，不自動寫入 |
| `--dry-run` | 只計算，不寫入 |
| `--center WxH` | 拉伸保留中心，預設 4x4 |
| `--repeat-count XxY` | 重複保留倍數，預設 1x1 |
| `--direction auto / both / horizontal / vertical` | 指定精簡方向 |
| `--mode auto / stretch / repeat` | 指定精簡方式 |
| `--no-preserve-edges` | 允許裁去透明外緣 |
| `--no-align4` | 停用 4 倍數對齊 |
| `--report FILE` | 另存 JSON，目標檔案須不存在 |
| `-h, --help` | 顯示說明 |
| `--` | 後續引數視為路徑 |

### PowerShell 範例

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## 開啟檔案

含空格或特殊字元的路徑請逐一加上引號。

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## 結果與結束代碼

兩個指令均輸出逐檔 JSON。結束代碼 0 表示無失敗，1 表示檔案失敗，2 表示參數或報告錯誤。PowerShell 可用 `Start-Process -Wait -PassThru` 等候並取得結束代碼。
