# AppIcon.png から Windows 用 PNG とマルチサイズ ICO を生成する。

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$sourcePath = Join-Path $root 'assets\AppIcon.png'
$outputPngPath = Join-Path $root 'assets\AppIconWindows.png'
$outputIcoPath = Join-Path $root 'assets\AppIcon.ico'
$sizes = 16, 24, 32, 48, 64, 128, 256
# 元画像の約 10% の透明余白を約 2% に縮め、小さい表示でも絵柄を見やすくする。
$artworkScale = 1.20

Add-Type -AssemblyName System.Drawing

function New-ScaledIconBitmap {
    param(
        [System.Drawing.Bitmap]$Source,
        [int]$Size,
        [double]$Scale
    )

    $bitmap = New-Object System.Drawing.Bitmap(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)

    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.CompositingMode =
            [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
        $graphics.CompositingQuality =
            [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.InterpolationMode =
            [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode =
            [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.SmoothingMode =
            [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $drawSize = $Size * $Scale
        $offset = ($Size - $drawSize) / 2
        $graphics.DrawImage($Source, $offset, $offset, $drawSize, $drawSize)
    } finally {
        $graphics.Dispose()
    }

    return $bitmap
}

$source = [System.Drawing.Bitmap]::FromFile($sourcePath)
$frames = @()

try {
    $windowsPng = New-ScaledIconBitmap $source 1024 $artworkScale
    try {
        $windowsPng.Save($outputPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $windowsPng.Dispose()
    }

    foreach ($size in $sizes) {
        $bitmap = New-ScaledIconBitmap $source $size $artworkScale
        $stream = New-Object System.IO.MemoryStream

        try {
            $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)

            $frames += [PSCustomObject]@{
                Size = $size
                Data = [byte[]]$stream.ToArray()
            }
        } finally {
            $stream.Dispose()
            $bitmap.Dispose()
        }
    }
} finally {
    $source.Dispose()
}

$file = [System.IO.File]::Create($outputIcoPath)
$writer = New-Object System.IO.BinaryWriter($file)

try {
    $writer.Write([uint16]0) # reserved
    $writer.Write([uint16]1) # icon
    $writer.Write([uint16]$frames.Count)

    $offset = 6 + (16 * $frames.Count)
    foreach ($frame in $frames) {
        $dimension = if ($frame.Size -eq 256) { 0 } else { $frame.Size }
        $writer.Write([byte]$dimension)
        $writer.Write([byte]$dimension)
        $writer.Write([byte]0) # palette size
        $writer.Write([byte]0) # reserved
        $writer.Write([uint16]1) # color planes
        $writer.Write([uint16]32) # bits per pixel
        $writer.Write([uint32]$frame.Data.Length)
        $writer.Write([uint32]$offset)
        $offset += $frame.Data.Length
    }

    foreach ($frame in $frames) {
        $writer.Write([byte[]]$frame.Data)
    }
} finally {
    $writer.Dispose()
    $file.Dispose()
}

Write-Host "生成しました: $outputPngPath"
Write-Host "生成しました: $outputIcoPath"
