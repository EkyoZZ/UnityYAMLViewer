## PNG 일괄 최적화

`pngopt [-all] <file-or-directory> [...]`

폴더 바로 아래 파일만 검사하고 중복 경로는 한 번 처리합니다. 원본에서 iTXt를 제거합니다. 모든 픽셀의 알파 값이 255인 정적 32비트 RGBA PNG에서만 알파 채널을 제거합니다.

| 옵션 | |
| --- | --- |
| `-all` | 하위 폴더도 처리하며 심볼릭 링크와 정션은 건너뜀 |
| `-h, --help` | 도움말 표시 |
| `--` | 나머지 인수를 경로로 처리 |

### PowerShell 예제

```powershell
.\unity_yaml_viewer.exe pngopt "a.png" "b.png" "Assets\UI"
```

```powershell
.\unity_yaml_viewer.exe pngopt -all "Assets\UI"
```

결과 상태: `alpha`, `itxt`, `both`, `unchanged`, `failed`.

## 9슬라이스 일괄 축소

`slice9 <file-or-directory> [...] [options]`

PNG, TGA, BMP 및 정적 WebP를 지원합니다. 크기가 줄면 백업 없이 원본을 교체합니다. 지원되는 단일 Sprite의 .meta도 갱신할 수 있습니다. 먼저 `--dry-run`으로 확인하세요.

| 옵션 | |
| --- | --- |
| `-r, --recursive` | 하위 폴더 포함 |
| `--preview` | 편집 가능한 미리보기 최대 10개 열기. 자동 저장 없음 |
| `--dry-run` | 계획만 계산하고 파일에 쓰지 않음 |
| `--center WxH` | 늘이기용 중앙 크기. 기본값 4x4 |
| `--repeat-count XxY` | 유지할 반복 횟수. 기본값 1x1 |
| `--direction auto / both / horizontal / vertical` | 축소 방향 지정 |
| `--mode auto / stretch / repeat` | 축소 방식 지정 |
| `--no-preserve-edges` | 투명 여백 자르기 허용 |
| `--no-align4` | 4의 배수 정렬 끄기 |
| `--report FILE` | 새 파일에 JSON 보고서 저장 |
| `-h, --help` | 도움말 표시 |
| `--` | 나머지 인수를 경로로 처리 |

### PowerShell 예제

```powershell
.\unity_yaml_viewer.exe slice9 "Assets\UI" -r --dry-run
```

```powershell
.\unity_yaml_viewer.exe slice9 "a.png" "b.png" --preview
```

## 파일 열기

공백이나 특수 문자가 포함된 경로는 각각 따옴표로 감싸세요.

```powershell
.\unity_yaml_viewer.exe "a.png" "Project Assets\b.prefab"
```

## 결과 및 종료 코드

두 명령 모두 파일별 JSON 결과를 출력합니다. 종료 코드 0은 실패 없음, 1은 파일 실패, 2는 인수 또는 보고서 오류입니다. PowerShell에서 `Start-Process -Wait -PassThru`로 완료를 기다리고 종료 코드를 읽을 수 있습니다.
