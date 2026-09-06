# Windows 向けパッケージングの共通処理。dot-source して使う。
#   . (Join-Path $PSScriptRoot 'packaging-common.ps1')
#
# コード署名に使う証明書は環境変数 CODESIGN_CERT で指定する。値の形で解釈が変わる。
#   - 実在するファイルのパス (.pfx/.p12) → signtool /f <path>
#                                          パスワードは CODESIGN_CERT_PASSWORD
#   - 40 桁の 16 進数                     → signtool /sha1 <thumbprint> (証明書ストア)
#   - それ以外の文字列                    → signtool /n <サブジェクト名>  (証明書ストア)
#
# 任意の環境変数:
#   CODESIGN_CERT_PASSWORD  .pfx のパスワード
#   CODESIGN_TIMESTAMP_URL  RFC3161 タイムスタンプ URL (既定: http://timestamp.digicert.com)
#   SIGNTOOL                signtool.exe のパス (自動検出できない場合に指定)

# 配布物のファイル名に使うアーキテクチャ名 (pdr-<version>-win-<arch>.<ext>)。
function Get-PackageArch {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
        'X64'   { 'x64' }
        'Arm64' { 'arm64' }
        'X86'   { 'x86' }
        default { 'x64' }
    }
}

# Windows SDK の signtool.exe を探す。
function Find-SignTool {
    if ($env:SIGNTOOL) {
        if (-not (Test-Path -LiteralPath $env:SIGNTOOL)) {
            throw "環境変数 SIGNTOOL のパスが存在しません: $env:SIGNTOOL"
        }
        return (Resolve-Path -LiteralPath $env:SIGNTOOL).Path
    }
    $cmd = Get-Command 'signtool.exe' -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $hostArch = Get-PackageArch
    foreach ($progRoot in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if (-not $progRoot) { continue }
        $binRoot = Join-Path $progRoot 'Windows Kits\10\bin'
        if (-not (Test-Path $binRoot)) { continue }
        $found = Get-ChildItem -Path $binRoot -Filter 'signtool.exe' -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.DirectoryName -like "*\$hostArch" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    throw @'
signtool.exe が見つかりません。Windows SDK (Windows App Certification Kit を含む)
をインストールするか、環境変数 SIGNTOOL に signtool.exe のパスを設定してください。
  winget install Microsoft.WindowsSDK.10.0.26100
'@
}

# CODESIGN_CERT から signtool の証明書指定引数を組み立てる。要素は必ず「フラグ, 値」の対。
function Get-CodeSignCertArgument {
    $cert = $env:CODESIGN_CERT
    if (-not $cert) {
        throw '環境変数 CODESIGN_CERT が設定されていません。証明書ファイル (.pfx) のパス、拇印 (SHA1 40 桁)、またはサブジェクト名を設定してください。'
    }
    if (Test-Path -LiteralPath $cert -PathType Leaf) {
        $a = @('/f', (Resolve-Path -LiteralPath $cert).Path)
        if ($env:CODESIGN_CERT_PASSWORD) { $a += @('/p', $env:CODESIGN_CERT_PASSWORD) }
        return $a
    }
    if ($cert -match '^[0-9A-Fa-f]{40}$') { return @('/sha1', $cert) }
    return @('/n', $cert)
}

function Get-CodeSignTimestampArgument {
    $url = if ($env:CODESIGN_TIMESTAMP_URL) { $env:CODESIGN_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
    return @('/tr', $url, '/td', 'sha256')
}

# 指定したファイルに署名する。
function Invoke-CodeSign {
    param([Parameter(Mandatory = $true)][string[]]$Path)

    $tool = Find-SignTool
    $a = @('sign', '/fd', 'sha256') + (Get-CodeSignCertArgument) + (Get-CodeSignTimestampArgument) + $Path
    & $tool @a
    if ($LASTEXITCODE -ne 0) {
        throw "署名に失敗しました (signtool exit $LASTEXITCODE): $($Path -join ', ')"
    }
    foreach ($p in $Path) { Write-Host "署名しました: $p" }
}

# Inno Setup の SignTool ディレクティブに渡すコマンド文字列を作る。
# Inno 側のプレースホルダを使うため、引用符は $q、対象ファイルは $f で表す
# (実際の " を含めないので、ISCC へ渡すときの引用符の扱いに悩まなくて済む)。
function Get-InnoSignToolCommand {
    $tool = Find-SignTool
    $parts = [System.Collections.Generic.List[string]]::new()
    $parts.Add('$q' + $tool + '$q')
    $parts.Add('sign')
    $parts.Add('/fd')
    $parts.Add('sha256')

    $certArgs = Get-CodeSignCertArgument
    for ($i = 0; $i -lt $certArgs.Count; $i += 2) {
        $parts.Add($certArgs[$i])
        $parts.Add('$q' + $certArgs[$i + 1] + '$q')
    }
    $ts = Get-CodeSignTimestampArgument
    $parts.Add($ts[0])
    $parts.Add('$q' + $ts[1] + '$q')
    $parts.Add($ts[2])
    $parts.Add($ts[3])

    $parts.Add('$f')
    return ($parts -join ' ')
}
