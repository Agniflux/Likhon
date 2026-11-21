# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Crops a banner out of the design's splash artwork for the installer's Welcome page, as plain .bmp files
# (100%, 150% and 200% DPI), with rounded corners painted in the page colour (design system `bg`, #FBF9F4).
#   powershell -File Desktop\Installer\art\make-splash.ps1
# Source: Reference\design-v2\assets\splash.png (the same picture the app's splash uses; it is bundled in the installer only as these .bmp files).
Add-Type -AssemblyName System.Drawing
$here = $PSScriptRoot
$src = [System.Drawing.Image]::FromFile((Resolve-Path (Join-Path $here '..\..\..\Reference\design-v2\assets\splash.png')).Path)
$bg = [System.Drawing.Color]::FromArgb(0xFB, 0xF9, 0xF4)
# Crop: the child, the birds and the sun (source is 3202 x 2136).
$crop = New-Object System.Drawing.Rectangle 200, 930, 2800, 724
foreach ($scale in 1, 1.5, 2) {
    $w = [int](464 * $scale); $h = [int](120 * $scale); $r = [int](12 * $scale)
    $bmp = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'
    $g.Clear($bg)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $path.AddArc(0, 0, $d, $d, 180, 90); $path.AddArc($w - $d - 1, 0, $d, $d, 270, 90)
    $path.AddArc($w - $d - 1, $h - $d - 1, $d, $d, 0, 90); $path.AddArc(0, $h - $d - 1, $d, $d, 90, 90)
    $path.CloseFigure()
    $g.SetClip($path)
    $g.DrawImage($src, (New-Object System.Drawing.Rectangle 0, 0, $w, $h), $crop, [System.Drawing.GraphicsUnit]::Pixel)
    $g.Dispose()
    $out = Join-Path $here ("splash-{0}.bmp" -f [int](464 * $scale))
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Bmp); $bmp.Dispose()
    Write-Host "Wrote $out"
}
$src.Dispose()
