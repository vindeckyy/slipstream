<#
.SYNOPSIS
  Generate the slipstream host installer branding assets (wizard BMPs + setup .ico).

.DESCRIPTION
  Renders the slipstream brand mark - the blue 3-swoosh S from assets/slipstream-mark.png
  (the SS.png-derived square mark, the same one the tray icons and the web console use) - into
  the assets Inno Setup consumes:

    wizard-image-*.bmp   welcome/finish page side panel (164x314 base, 100..200% DPI variants);
                         dark panel + the mark + the lowercase wordmark. The panel
                         is self-contained dark, so it reads correctly in BOTH the light and dark
                         (WizardStyle=dynamic) wizard appearances.
    wizard-small-*.bmp   header tile on the inner pages (55x55 base, 100..200% DPI variants);
                         the square brand tile (mark on black), matching the MSIX client tile.
    slipstream.ico        multi-size icon (16..256, PNG-compressed entries - Vista+ format, we
                         require Windows 10) for SetupIconFile + the Apps & Features entry.

  Outputs are COMMITTED next to this script (like include/slipstream_core.h, generated-but-checked-in);
  re-run only when the brand changes. Everything is drawn 4x supersampled and downscaled
  (System.Drawing regions/clips do not antialias), so edges stay clean at every size.

.EXAMPLE
  pwsh -File packaging/windows/branding/gen-branding.ps1
#>
[CmdletBinding()]
param([string]$OutDir = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

# --- brand constants (colors from the SS.png-derived mark; tile background from the MSIX assets) --
$colTile = [System.Drawing.Color]::FromArgb(255, 0x05, 0x0A, 0x0F)   # brand tile background (black)
$colPanelTop = [System.Drawing.Color]::FromArgb(255, 0x0B, 0x12, 0x18)   # wizard panel gradient
$colPanelBot = [System.Drawing.Color]::FromArgb(255, 0x02, 0x05, 0x08)
$colText = [System.Drawing.Color]::FromArgb(255, 0xC9, 0xE8, 0xFB)   # wordmark on the panel

# The brand mark (the blue 3-swoosh S) comes from the committed square mark PNG —
# assets/slipstream-mark.png (SS.png-derived). Drawn onto the tile, not analytic circles.
$markPath = Join-Path $PSScriptRoot '../../assets/slipstream-mark.png'
$markSource = [System.Drawing.Image]::FromFile((Resolve-Path $markPath))

# Draw the mark onto $g centered at ($cx,$cy) with bounding-box size $size (device pixels).
function Draw-Mark([System.Drawing.Graphics]$g, [double]$cx, [double]$cy, [double]$size) {
    $s = $size / $markSource.Width
    $x = [float]($cx - $size / 2.0)
    $y = [float]($cy - $size / 2.0)
    $w = [float]$size
    $h = [float]$size
    $g.DrawImage($markSource, $x, $y, $w, $h)
}

# New 32bpp canvas + antialiased Graphics.
function New-Canvas([int]$w, [int]$h) {
    $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
    @($bmp, $g)
}

# Downscale $src to $w x $h (high-quality bicubic) - the supersample resolve.
function Resize-Bitmap([System.Drawing.Bitmap]$src, [int]$w, [int]$h) {
    $dst = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($dst)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.DrawImage($src, (New-Object System.Drawing.Rectangle(0, 0, $w, $h)),
        0, 0, $src.Width, $src.Height, [System.Drawing.GraphicsUnit]::Pixel)
    $g.Dispose()
    $dst
}

# Save as 24bpp BMP (opaque - what Inno's wizard image loader expects by default).
function Save-Bmp24([System.Drawing.Bitmap]$bmp, [string]$path) {
    $b24 = $bmp.Clone((New-Object System.Drawing.Rectangle(0, 0, $bmp.Width, $bmp.Height)),
        [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $b24.Save($path, [System.Drawing.Imaging.ImageFormat]::Bmp)
    $b24.Dispose()
    Write-Host "  wrote $path ($($bmp.Width)x$($bmp.Height))"
}

$SS = 4   # supersample factor

# --- wizard side panel (welcome/finish page): gradient + mark + wordmark ----------------------
# Base size 164x314 (Inno's classic canvas); DPI variants via the wizard-image-*.bmp wildcard.
foreach ($pct in 100, 125, 150, 175, 200) {
    $w = [int][Math]::Round(164 * $pct / 100.0)
    $h = [int][Math]::Round(314 * $pct / 100.0)
    $bmp, $g = New-Canvas ($w * $SS) ($h * $SS)
    $rect = New-Object System.Drawing.Rectangle(0, 0, ($w * $SS), ($h * $SS))
    $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $colPanelTop, $colPanelBot, 90.0)
    $g.FillRectangle($grad, $rect); $grad.Dispose()
    # Mark: 58% of the panel width, centered horizontally, optical center at ~40% height.
    Draw-Mark $g ($w * $SS / 2.0) ($h * $SS * 0.40) ($w * $SS * 0.58)
    # Wordmark: lowercase brand name under the mark.
    $font = New-Object System.Drawing.Font('Segoe UI Semibold', [float](13.0 * $SS * $pct / 100.0), [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
    $tb = New-Object System.Drawing.SolidBrush($colText)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $g.DrawString('slipstream', $font, $tb,
        (New-Object System.Drawing.PointF([float]($w * $SS / 2.0), [float]($h * $SS * 0.60))), $fmt)
    $fmt.Dispose(); $tb.Dispose(); $font.Dispose(); $g.Dispose()
    $out = Resize-Bitmap $bmp $w $h
    $bmp.Dispose()
    Save-Bmp24 $out (Join-Path $OutDir ("wizard-image-{0}.bmp" -f $pct))
    $out.Dispose()
}

# --- wizard header tile (inner pages): the square brand tile --------------------------------
# Base size 55x55; DPI variants via the wizard-small-*.bmp wildcard. Opaque square (BMP has no
# alpha here): the same full-bleed dark tile as the client's MSIX logo assets.
foreach ($pct in 100, 125, 150, 175, 200) {
    $sz = [int][Math]::Round(55 * $pct / 100.0)
    $bmp, $g = New-Canvas ($sz * $SS) ($sz * $SS)
    $b = New-Object System.Drawing.SolidBrush($colTile)
    $g.FillRectangle($b, 0, 0, ($sz * $SS), ($sz * $SS)); $b.Dispose()
    Draw-Mark $g ($sz * $SS / 2.0) ($sz * $SS / 2.0) ($sz * $SS * 0.74)
    $g.Dispose()
    $out = Resize-Bitmap $bmp $sz $sz
    $bmp.Dispose()
    Save-Bmp24 $out (Join-Path $OutDir ("wizard-small-{0}.bmp" -f $pct))
    $out.Dispose()
}

# --- slipstream.ico: rounded brand tile at 16..256 --------------------------------------------
# Small sizes are classic 32bpp DIB entries (Inno's SetupIconFile resource updater and older shell
# consumers reject an all-PNG icon); only 128/256 use PNG compression (the standard Vista+ layout).
function New-IconTile([int]$sz) {
    $bmp, $g = New-Canvas ($sz * $SS) ($sz * $SS)
    # Rounded-rect tile (22% corner radius - the Windows 11 app-icon look).
    $S = $sz * $SS; $rad = [int]($S * 0.22)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc(0, 0, 2 * $rad, 2 * $rad, 180, 90)
    $path.AddArc($S - 2 * $rad, 0, 2 * $rad, 2 * $rad, 270, 90)
    $path.AddArc($S - 2 * $rad, $S - 2 * $rad, 2 * $rad, 2 * $rad, 0, 90)
    $path.AddArc(0, $S - 2 * $rad, 2 * $rad, 2 * $rad, 90, 90)
    $path.CloseFigure()
    $b = New-Object System.Drawing.SolidBrush($colTile)
    $g.FillPath($b, $path); $b.Dispose(); $path.Dispose()
    Draw-Mark $g ($S / 2.0) ($S / 2.0) ($S * 0.74)
    $g.Dispose()
    $out = Resize-Bitmap $bmp $sz $sz
    $bmp.Dispose()
    $out
}

# PNG-compressed entry payload (used for the 128/256 entries). The leading comma keeps the byte[]
# a single pipeline object (PowerShell would otherwise unroll it into individual bytes).
function ConvertTo-IconPng([System.Drawing.Bitmap]$tile) {
    $ms = New-Object System.IO.MemoryStream
    $tile.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    return , $ms.ToArray()
}

# Classic ICO DIB entry payload: BITMAPINFOHEADER (height doubled) + bottom-up 32bpp BGRA XOR data
# + an all-zero 1bpp AND mask (32bpp icons carry transparency in the alpha channel).
function ConvertTo-IconDib([System.Drawing.Bitmap]$tile) {
    $s = $tile.Width
    $rect = New-Object System.Drawing.Rectangle(0, 0, $s, $s)
    $data = $tile.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $px = New-Object byte[] ($data.Stride * $s)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $px, 0, $px.Length)
    $tile.UnlockBits($data)
    $maskStride = [int][Math]::Ceiling($s / 32.0) * 4     # 1bpp rows padded to 32 bits
    $ms = New-Object System.IO.MemoryStream
    $w = New-Object System.IO.BinaryWriter($ms)
    $w.Write([uint32]40); $w.Write([int]$s); $w.Write([int]($s * 2))   # biSize, biWidth, biHeight (XOR+AND)
    $w.Write([uint16]1); $w.Write([uint16]32); $w.Write([uint32]0)     # biPlanes, biBitCount, BI_RGB
    $w.Write([uint32]($s * $s * 4 + $maskStride * $s))                 # biSizeImage
    $w.Write([int]0); $w.Write([int]0); $w.Write([uint32]0); $w.Write([uint32]0)
    for ($y = $s - 1; $y -ge 0; $y--) { $w.Write($px, $y * $data.Stride, $s * 4) }   # XOR, bottom-up
    $w.Write((New-Object byte[] ($maskStride * $s)))                   # AND mask: all opaque
    $w.Flush()
    $bytes = $ms.ToArray()
    $w.Dispose(); $ms.Dispose()
    return , $bytes   # leading comma: emit the byte[] as ONE object, not unrolled bytes
}

$icoSizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$pngs = @(foreach ($s in $icoSizes) {
    $tile = New-IconTile $s
    if ($s -ge 128) { ConvertTo-IconPng $tile } else { ConvertTo-IconDib $tile }
    $tile.Dispose()
})
$ico = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ico)
# ICONDIR
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$icoSizes.Count)
# ICONDIRENTRYs (width/height byte 0 means 256)
$offset = 6 + 16 * $icoSizes.Count
for ($i = 0; $i -lt $icoSizes.Count; $i++) {
    $s = $icoSizes[$i]
    $dim = if ($s -ge 256) { 0 } else { $s }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim)          # width, height
    $bw.Write([byte]0); $bw.Write([byte]0)                # colors, reserved
    $bw.Write([uint16]1); $bw.Write([uint16]32)           # planes, bitcount
    $bw.Write([uint32]$pngs[$i].Length); $bw.Write([uint32]$offset)
    $offset += $pngs[$i].Length
}
foreach ($p in $pngs) { $bw.Write([byte[]]$p) }
$bw.Flush()
$icoPath = Join-Path $OutDir 'slipstream.ico'
[IO.File]::WriteAllBytes($icoPath, $ico.ToArray())
$bw.Dispose(); $ico.Dispose()
# Self-check: a malformed container here would only surface later as ISCC "Icon file is invalid".
$probe = New-Object System.Drawing.Icon($icoPath)
$probe.Dispose()
Write-Host "  wrote $icoPath ($($icoSizes -join ',')) - verified loadable"
$markSource.Dispose()
Write-Host "==> branding assets generated in $OutDir"
