# リリースビルドして exe を dist/ に zip で固める。
# 使い方: pwsh scripts/package.ps1 [-SkipBuild]
# 出力: dist/rkpack-v<version>-<host triple>.zip (中身は rkpack.exe のみ)
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not $SkipBuild) {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed ($LASTEXITCODE)" }
    }

    $meta = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    $version = ($meta.packages | Where-Object name -eq 'rkpack').version
    $host_line = (rustc -vV) | Where-Object { $_ -match '^host:' }
    $target = ($host_line -replace '^host:\s*', '').Trim()

    $exe = Join-Path $root 'target\release\rkpack.exe'
    if (-not (Test-Path $exe)) { throw "not found: $exe" }

    $dist = Join-Path $root 'dist'
    New-Item -ItemType Directory -Force $dist | Out-Null
    $zip = Join-Path $dist "rkpack-v$version-$target.zip"
    if (Test-Path $zip) { Remove-Item $zip }

    Compress-Archive -Path $exe -DestinationPath $zip -CompressionLevel Optimal
    Write-Host "created: $zip"
    Write-Output $zip
}
finally {
    Pop-Location
}
