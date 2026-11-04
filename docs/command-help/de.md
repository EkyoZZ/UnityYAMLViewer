## PNG-Stapeloptimierung

`pngopt [-all] <file-or-directory> [...]`

Verarbeitet nur Dateien direkt im angegebenen Ordner und jeden Pfad einmal. Entfernt iTXt direkt aus der Datei. Der Alphakanal entfällt nur bei statischen 32-Bit-RGBA-PNGs, deren Pixel alle den Alphawert 255 haben.

| Optionen | |
| --- | --- |
| `-all` | Unterordner einbeziehen und symbolische Links sowie Verzeichnisverbindungen überspringen |
| `-h, --help` | Hilfe anzeigen |
| `--` | Weitere Argumente als Pfade behandeln |

### PowerShell-Beispiele

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

Ergebnisstatus: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## 9-Slice-Komprimierung im Stapel

`slice9 <file-or-directory> [...] [options]`

Unterstützt PNG, TGA, BMP und statisches WebP. Kleinere Ergebnisse ersetzen die Originale ohne Sicherung. Unterstützte .meta-Dateien einzelner Sprites können aktualisiert werden. Prüfen Sie den Plan zuerst mit `--dry-run`.

| Optionen | |
| --- | --- |
| `-r, --recursive` | Unterordner einbeziehen |
| `--preview` | Bis zu 10 bearbeitbare Vorschauen öffnen, ohne automatisch zu speichern |
| `--dry-run` | Plan berechnen, ohne Dateien zu schreiben |
| `--center WxH` | Erhaltene Stretch-Mitte, Standard 4x4 |
| `--repeat-count XxY` | Erhaltene Wiederholungszyklen, Standard 1x1 |
| `--direction auto / both / horizontal / vertical` | Komprimierungsrichtung wählen |
| `--mode auto / stretch / repeat` | Komprimierungsverfahren wählen |
| `--no-preserve-edges` | Transparente Ränder dürfen beschnitten werden |
| `--no-align4` | Ausrichtung auf Vielfache von vier deaktivieren |
| `--report FILE` | JSON-Bericht in einer neuen Datei speichern |
| `-h, --help` | Hilfe anzeigen |
| `--` | Weitere Argumente als Pfade behandeln |

### PowerShell-Beispiele

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## Dateien öffnen

Setzen Sie jeden Pfad mit Leerzeichen oder Sonderzeichen in Anführungszeichen.

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## Ergebnisse und Rückgabecodes

Beide Befehle geben JSON mit Einzelergebnissen aus. Rückgabecodes: 0 ohne Fehler, 1 bei Dateifehlern und 2 bei Argument- oder Berichtsfehlern. In PowerShell wartet `Start-Process -Wait -PassThru` auf den Abschluss und liefert den Rückgabecode.
