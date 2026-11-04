## PNG の一括最適化

`pngopt [-all] <file-or-directory> [...]`

フォルダー直下のみを対象とし、重複したパスは一度だけ処理します。iTXt を削除します。アルファの削除は、全画素の値が 255 の静止 32 ビット RGBA PNG に限ります。

| オプション | |
| --- | --- |
| `-all` | サブフォルダーも処理し、シンボリックリンクとジャンクションは除外 |
| `-h, --help` | ヘルプを表示 |
| `--` | 以降の引数をパスとして扱う |

### PowerShell の例

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

結果の状態: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## 9 スライスの一括縮小

`slice9 <file-or-directory> [...] [options]`

PNG、TGA、BMP、静止 WebP に対応します。縮小できた場合はバックアップを作成せず上書きします。対応する単一 Sprite の .meta も更新できます。まず `--dry-run` で確認できます。

| オプション | |
| --- | --- |
| `-r, --recursive` | サブフォルダーを含める |
| `--preview` | 編集可能なプレビューを最大 10 件開く。自動書き込みなし |
| `--dry-run` | 計画のみ。ファイルに書き込まない |
| `--center WxH` | 伸縮用に残す中央領域。既定値は 4x4 |
| `--repeat-count XxY` | 残す繰り返し回数。既定値は 1x1 |
| `--direction auto / both / horizontal / vertical` | 縮小方向を指定 |
| `--mode auto / stretch / repeat` | 縮小方式を指定 |
| `--no-preserve-edges` | 透明な余白の切り取りを許可 |
| `--no-align4` | 4 の倍数への整列を無効にする |
| `--report FILE` | 新規ファイルに JSON レポートを保存 |
| `-h, --help` | ヘルプを表示 |
| `--` | 以降の引数をパスとして扱う |

### PowerShell の例

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## ファイルを開く

空白や特殊文字を含むパスは個別に引用符で囲みます。

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## 結果と終了コード

どちらのコマンドもファイル別の結果を JSON で出力します。終了コードは成功が 0、ファイルエラーが 1、引数またはレポートのエラーが 2 です。PowerShell では `Start-Process -Wait -PassThru` で待機し、終了コードを取得できます。
