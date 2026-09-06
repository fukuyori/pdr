# Inno Setup で Windows 用インストーラーを作成する（ビルドは既定でしない）。
#
# 事前に: cargo build --release   （または -Build を付ける）
#         Inno Setup 6.3 以降     （winget install JRSoftware.InnoSetup）
#
# 使い方:
#   pwsh -File scripts/package-windows-inno.ps1
#   pwsh -File scripts/package-windows-inno.ps1 -Build
#   pwsh -File scripts/package-windows-inno.ps1 -Sign
#   pwsh -File scripts/package-windows-inno.ps1 -Iscc 'C:\path\to\ISCC.exe'
#
# 出力: dist\pdr-<version>-win-<arch>.exe
#
# -Sign を付けると pdr.exe・インストーラー・アンインストーラーの 3 つに署名する。
# アンインストーラーはインストーラーのコンパイル時に生成されるため、Inno の
# SignTool / SignedUninstaller ディレクティブ経由で署名する。証明書の指定方法は
# scripts/packaging-common.ps1 の冒頭を参照（環境変数 CODESIGN_CERT）。

[CmdletBinding()]
param(
    # 実行ファイル・インストーラー・アンインストーラーに電子署名する
    [switch]$Sign,
    # パッケージ前に cargo build --release を実行する
    [switch]$Build,
    # ISCC.exe のパス（自動検出できない場合に指定）
    [string]$Iscc
)

$ErrorActionPreference = 'Stop'

# このスクリプトは scripts/ 配下にある。リポジトリ直下はその親。
$root = Split-Path -Parent $PSScriptRoot
if (-not $root) { $root = Split-Path -Parent (Get-Location).Path }

. (Join-Path $PSScriptRoot 'packaging-common.ps1')

# Inno Setup のコンパイラ ISCC.exe を探す。
function Find-Iscc {
    param([string]$Path)

    if ($Path) {
        if (-not (Test-Path -LiteralPath $Path)) { throw "-Iscc のパスが存在しません: $Path" }
        return (Resolve-Path -LiteralPath $Path).Path
    }
    $cmd = Get-Command 'ISCC.exe' -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    foreach ($progRoot in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if (-not $progRoot) { continue }
        $p = Join-Path $progRoot 'Inno Setup 6\ISCC.exe'
        if (Test-Path $p) { return $p }
    }
    throw @'
Inno Setup のコンパイラ (ISCC.exe) が見つかりません。
インストールするか、-Iscc でパスを指定してください。
  winget install JRSoftware.InnoSetup
'@
}

# --- 事前チェック -------------------------------------------------------------

$iscc = Find-Iscc -Path $Iscc
$innoDir = Split-Path -Parent $iscc

if ($Sign) {
    # 証明書と signtool は先に解決しておく（長いコンパイルの後で失敗させない）。
    $signCommand = Get-InnoSignToolCommand
}

if ($Build) {
    Push-Location $root
    try {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build --release に失敗しました (exit $LASTEXITCODE)" }
    } finally { Pop-Location }
}

# バージョンを Cargo.toml から取得
$verMatch = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1
if (-not $verMatch) { throw 'Cargo.toml から version を取得できませんでした' }
$version = $verMatch.Matches.Groups[1].Value
$arch = Get-PackageArch

$exe = Join-Path $root 'target\release\pdr.exe'
$dll = Join-Path $root 'third_party\pdfium\pdfium.dll'
if (-not (Test-Path $exe)) {
    throw "pdr.exe がありません。先に 'cargo build --release' を実行してください: $exe"
}
if (-not (Test-Path $dll)) { throw "pdfium.dll がありません: $dll" }

# --- インストール対象を staging へ集める ---------------------------------------

$workDir = Join-Path $root 'target\package-windows-inno'
$stage = Join-Path $workDir 'stage'
if (Test-Path $workDir) { Remove-Item -Recurse -Force $workDir }
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

# 実行ファイルの署名は staging 上のコピーに対して行う（target/release は汚さない）。
if ($Sign) {
    Invoke-CodeSign -Path @((Join-Path $stage 'pdr.exe'))
}

# --- .iss を生成する -----------------------------------------------------------

# Japanese.isl は Inno Setup 6 に同梱されているが、環境によっては無いので確認する。
$hasJapanese = Test-Path (Join-Path $innoDir 'Languages\Japanese.isl')

$distDir = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $distDir | Out-Null
$baseName = "pdr-$version-win-$arch"

switch ($arch) {
    'arm64' { $archAllowed = 'arm64'; $arch64Mode = 'arm64' }
    'x86'   { $archAllowed = 'x86compatible'; $arch64Mode = '' }
    default { $archAllowed = 'x64compatible'; $arch64Mode = 'x64compatible' }
}

$iss = [System.Collections.Generic.List[string]]::new()

$iss.Add('; このファイルは scripts/package-windows-inno.ps1 が生成する。編集しても次回上書きされる。')
$iss.Add('')
$iss.Add('[Setup]')
$iss.Add('AppId={{B7A6E8C2-3F41-4D9B-9E77-1A2C5D6F8B03}')
$iss.Add('AppName=PDR')
$iss.Add("AppVersion=$version")
$iss.Add("AppVerName=PDR $version")
$iss.Add("VersionInfoVersion=$version")
$iss.Add('AppPublisher=fukuyori')
$iss.Add('AppPublisherURL=https://github.com/fukuyori/pdr')
$iss.Add('AppSupportURL=https://github.com/fukuyori/pdr/issues')
$iss.Add('AppUpdatesURL=https://github.com/fukuyori/pdr/releases')
$iss.Add('DefaultDirName={autopf}\PDR')
$iss.Add('DefaultGroupName=PDR')
$iss.Add('DisableProgramGroupPage=yes')
$iss.Add("LicenseFile=$(Join-Path $root 'LICENSE')")
$iss.Add("SetupIconFile=$(Join-Path $root 'assets\AppIcon.ico')")
$iss.Add("OutputDir=$distDir")
$iss.Add("OutputBaseFilename=$baseName")
$iss.Add('UninstallDisplayName=PDR')
$iss.Add('UninstallDisplayIcon={app}\pdr.exe')
$iss.Add('Compression=lzma2/max')
$iss.Add('SolidCompression=yes')
$iss.Add('WizardStyle=modern')
$iss.Add('PrivilegesRequired=admin')
$iss.Add('PrivilegesRequiredOverridesAllowed=dialog')
$iss.Add('ChangesAssociations=yes')
$iss.Add("ArchitecturesAllowed=$archAllowed")
if ($arch64Mode) { $iss.Add("ArchitecturesInstallIn64BitMode=$arch64Mode") }
if ($Sign) {
    # SignTool= でインストーラー本体、SignedUninstaller= でアンインストーラーに署名する。
    $iss.Add('SignTool=pdrsign')
    $iss.Add('SignedUninstaller=yes')
}
$iss.Add('')
$iss.Add('[Languages]')
$iss.Add('Name: "en"; MessagesFile: "compiler:Default.isl"')
if ($hasJapanese) { $iss.Add('Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"') }
$iss.Add('')
$iss.Add('[CustomMessages]')
$iss.Add('en.AssociatePdf=Associate PDF files (.pdf) with PDR')
if ($hasJapanese) { $iss.Add('ja.AssociatePdf=PDF ファイル (.pdf) を PDR に関連付ける') }
$iss.Add('')
$iss.Add('[Tasks]')
$iss.Add('Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked')
$iss.Add('Name: "associatepdf"; Description: "{cm:AssociatePdf}"; Flags: unchecked')
$iss.Add('')
$iss.Add('[Files]')
$iss.Add('Source: "' + $stage + '\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs')
$iss.Add('')
$iss.Add('[Icons]')
$iss.Add('Name: "{group}\PDR"; Filename: "{app}\pdr.exe"')
$iss.Add('Name: "{group}\{cm:UninstallProgram,PDR}"; Filename: "{uninstallexe}"')
$iss.Add('Name: "{autodesktop}\PDR"; Filename: "{app}\pdr.exe"; Tasks: desktopicon')
$iss.Add('')
$iss.Add('[Registry]')
$iss.Add('; 既定の PDF ビューアには設定しない（Windows 10 以降はユーザー操作が必要）。')
$iss.Add('; 「プログラムから開く」に PDR が並ぶところまでを登録する。')
$iss.Add('Root: HKA; Subkey: "Software\Classes\pdr.pdf"; ValueType: string; ValueData: "PDF Document"; Flags: uninsdeletekey; Tasks: associatepdf')
$iss.Add('Root: HKA; Subkey: "Software\Classes\pdr.pdf\DefaultIcon"; ValueType: string; ValueData: "{app}\pdr.exe,0"; Tasks: associatepdf')
$iss.Add('Root: HKA; Subkey: "Software\Classes\pdr.pdf\shell\open\command"; ValueType: string; ValueData: """{app}\pdr.exe"" ""%1"""; Tasks: associatepdf')
$iss.Add('Root: HKA; Subkey: "Software\Classes\Applications\pdr.exe\shell\open\command"; ValueType: string; ValueData: """{app}\pdr.exe"" ""%1"""; Flags: uninsdeletekey; Tasks: associatepdf')
$iss.Add('Root: HKA; Subkey: "Software\Classes\.pdf\OpenWithProgids"; ValueType: string; ValueName: "pdr.pdf"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associatepdf')
$iss.Add('')
$iss.Add('[Run]')
$iss.Add('Filename: "{app}\pdr.exe"; Description: "{cm:LaunchProgram,PDR}"; Flags: nowait postinstall skipifsilent')

# Inno Setup は BOM の無い .iss を ANSI として読むため、UTF-8 BOM 付きで書く。
$issPath = Join-Path $workDir 'pdr.iss'
[System.IO.File]::WriteAllText($issPath, ($iss -join "`r`n") + "`r`n", (New-Object System.Text.UTF8Encoding($true)))

# --- コンパイル ---------------------------------------------------------------

$installer = Join-Path $distDir "$baseName.exe"
if (Test-Path $installer) { Remove-Item -Force $installer }

$isccArgs = @()
if ($Sign) { $isccArgs += "/Spdrsign=$signCommand" }
$isccArgs += $issPath

& $iscc @isccArgs
if ($LASTEXITCODE -ne 0) { throw "ISCC.exe に失敗しました (exit $LASTEXITCODE)" }
if (-not (Test-Path $installer)) { throw "インストーラーが生成されませんでした: $installer" }

$size = [math]::Round((Get-Item $installer).Length / 1MB, 1)
Write-Host "作成しました: $installer ($size MB)"
if ($Sign) { Write-Host '署名対象: pdr.exe / インストーラー / アンインストーラー' }
