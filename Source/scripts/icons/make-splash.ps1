# Builds the splash artwork the exe embeds (v2 §6): Reference/design-v2/assets/splash.png scaled to 960 x 640 (the
# 480 x 320 splash at 200%), saved as JPEG quality 90 (about 65 KB; PNG would be about 170 KB) to
# assets/splash/splash.jpg. Build-time only; run again when the design's splash picture changes, then build the exe.
#
#   powershell -ExecutionPolicy Bypass -File scripts/icons/make-splash.ps1
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$src = Join-Path $root '..\Reference\design-v2\assets\splash.png'
$outDir = Join-Path $root 'assets\splash'
New-Item -ItemType Directory -Force $outDir | Out-Null
$out = Join-Path $outDir 'splash.jpg'

$img = [System.Drawing.Image]::FromFile((Resolve-Path $src))
try {
    $w = 960; $h = 640
    # Cover the frame: scale to fill, crop the overflow evenly (the art is 3:2, the same shape, so nothing is lost).
    $k = [Math]::Max($w / $img.Width, $h / $img.Height)
    $sw = $img.Width * $k; $sh = $img.Height * $k
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.DrawImage($img, [single](($w - $sw) / 2), [single](($h - $sh) / 2), [single]$sw, [single]$sh)
    $g.Dispose()
    $codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $params = New-Object System.Drawing.Imaging.EncoderParameters 1
    $params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality, [long]90)
    $bmp.Save($out, $codec, $params)
    $bmp.Dispose()
} finally { $img.Dispose() }
"splash.jpg: {0:N0} bytes" -f (Get-Item $out).Length
