# リリースビルドを配布用 zip にまとめる（ビルドはしない）。
#
# 事前に: cargo build --release
# 使い方:  pwsh -File scripts/package.ps1
#
# 出力: dist\pdr-<version>-win-x64.zip
#   pdr.exe / pdfium.dll / README.md / LICENSE / NOTICE と
#   pdfium のライセンス一式（pdfium/）を同梱する。

$ErrorActionPreference = 'Stop'
# このスクリプトは scripts/ 配下にある。リポジトリ直下はその親。
$root = Split-Path -Parent $PSScriptRoot
if (-not $root) { $root = Split-Path -Parent (Get-Location).Path }

# バージョンを Cargo.toml から取得
$verMatch = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1
if (-not $verMatch) { throw 'Cargo.toml から version を取得できませんでした' }
$version = $verMatch.Matches.Groups[1].Value

$exe = Join-Path $root 'target\release\pdr.exe'
$dll = Join-Path $root 'third_party\pdfium\pdfium.dll'
if (-not (Test-Path $exe)) {
    throw "pdr.exe がありません。先に 'cargo build --release' を実行してください: $exe"
}
if (-not (Test-Path $dll)) { throw "pdfium.dll がありません: $dll" }

$name = "pdr-$version-win-x64"
$stage = Join-Path $env:TEMP "pdr-pkg\$name"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

# 実行に必要なファイル
Copy-Item $exe $stage
Copy-Item $dll $stage

# ドキュメント・ライセンス
foreach ($f in 'README.md', 'LICENSE', 'NOTICE') {
    $p = Join-Path $root $f
    if (Test-Path $p) { Copy-Item $p $stage }
}

# pdfium のライセンス類（バイナリ再配布のため同梱）
$pdfiumDir = Join-Path $stage 'pdfium'
New-Item -ItemType Directory -Force -Path $pdfiumDir | Out-Null
Copy-Item (Join-Path $root 'third_party\pdfium\LICENSE') $pdfiumDir
Copy-Item (Join-Path $root 'third_party\pdfium\VERSION') $pdfiumDir
Copy-Item (Join-Path $root 'third_party\pdfium\licenses') $pdfiumDir -Recurse

# zip 出力（dist/ に <name>/ を含む形で作成）
$distDir = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $distDir | Out-Null
$zip = Join-Path $distDir "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip

$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "作成しました: $zip ($size MB)"
