param([switch]$ConfirmExecutableBuild)
$ErrorActionPreference = 'Stop'
if (-not $ConfirmExecutableBuild) {
    throw '本脚本会生成 EXE。必须先获得用户明确编译确认，再传入 -ConfirmExecutableBuild。'
}
$executable = Join-Path $PSScriptRoot 'target/release/ps-tile-installer.exe'
$running = @(Get-Process -Name 'ps-tile-installer' -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    throw '安装器正在运行。请先关闭，再由用户明确确认重新编译；不得改输出路径绕过检查。'
}
if (Test-Path -LiteralPath $executable) {
    try {
        $handle = [System.IO.File]::Open($executable, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
        $handle.Dispose()
    } catch {
        throw "目标 EXE 正被占用。请先解除占用，再由用户明确确认重新编译：$executable"
    }
}
# Keep one fixed output directory, independent of the caller's Cargo environment.
& cargo build --release --locked --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --target-dir (Join-Path $PSScriptRoot 'target')
if ($LASTEXITCODE -ne 0) { throw '安装器编译失败。' }
Write-Output "Installer: $executable"
Get-FileHash -LiteralPath $executable -Algorithm SHA256
