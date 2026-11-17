# Optional legacy distribution only. Normal delivery uses installer/build.ps1.
param([switch]$GenerateLegacyArchives)
$ErrorActionPreference = 'Stop'
if (-not $GenerateLegacyArchives) {
    Write-Output '默认只交付 EXE 安装器，不生成 CCX／ZIP。确需旧格式时显式传入 -GenerateLegacyArchives。'
    return
}
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$pluginRoot = $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $pluginRoot 'manifest.json') -Raw -Encoding utf8 | ConvertFrom-Json
if ($manifest.host -is [array] -or $manifest.host.app -ne 'PS') { throw 'CCX requires a single Photoshop host object.' }
$packageFiles = @('manifest.json', 'index.html', 'main.js', 'tile-dark@1x.png','tile-light@1x.png','tile-dark.png', 'tile-dark@2x.png', 'tile-light.png', 'tile-light@2x.png', 'png.js', 'README.md')
$outputDir = Join-Path $pluginRoot 'dist'
[System.IO.Directory]::CreateDirectory($outputDir) | Out-Null
$packagePath = Join-Path $outputDir "isometric-tile-preview-$($manifest.version).ccx"
function Write-CheckedArchive {
param([string]$packagePath,[string[]]$packageFiles)
$stream = [System.IO.File]::Open($packagePath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
try {
    $archive = [System.IO.Compression.ZipArchive]::new($stream, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $packageFiles) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, (Join-Path $pluginRoot $name), $name, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    } finally { $archive.Dispose() }
} finally { $stream.Dispose() }

# Verify each unpacked entry against the source bytes without installing it.
$archive = [System.IO.Compression.ZipFile]::OpenRead($packagePath)
try {
    if ($archive.Entries.Count -ne $packageFiles.Count) { throw 'Unexpected package entry count.' }
    foreach ($name in $packageFiles) {
        $entry = $archive.GetEntry($name)
        if ($null -eq $entry) { throw "Missing entry: $name" }
        $inputStream = $entry.Open()
        $buffer = [System.IO.MemoryStream]::new()
        try {
            $inputStream.CopyTo($buffer)
            $actual = [Convert]::ToBase64String($buffer.ToArray())
            $expected = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes((Join-Path $pluginRoot $name)))
            if ($actual -cne $expected) { throw "Package bytes differ: $name" }
        } finally { $buffer.Dispose(); $inputStream.Dispose() }
    }
} finally { $archive.Dispose() }
$hash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$packagePath.sha256", "$hash  $([System.IO.Path]::GetFileName($packagePath))`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "Package: $packagePath"
Write-Output "Verified entries: $($packageFiles.Count)"
Write-Output "Bytes: $((Get-Item -LiteralPath $packagePath).Length)"
Write-Output "SHA-256: $hash"
}
Write-CheckedArchive -packagePath $packagePath -packageFiles $packageFiles
$manualZip = Join-Path $outputDir "isometric-tile-preview-$($manifest.version)-files.zip"
Write-CheckedArchive -packagePath $manualZip -packageFiles @('manifest.json','index.html','main.js','png.js','tile-dark@1x.png','tile-light@1x.png','tile-dark.png','tile-dark@2x.png','tile-light.png','tile-light@2x.png')
