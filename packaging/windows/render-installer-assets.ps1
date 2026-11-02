param([string]$OutputDirectory = $PSScriptRoot)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null

# Render the editable, geometric brand illustration at 3x for high-DPI setup.
$scale = 3
$bitmap = [System.Drawing.Bitmap]::new(224 * $scale, 520 * $scale)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$graphics.ScaleTransform($scale, $scale)
$graphics.Clear([System.Drawing.ColorTranslator]::FromHtml('#121314'))

function Draw-Line([string]$Color, [single[]]$Coordinates, [single]$Width = 0.7) {
    $pen = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml($Color), $Width)
    try {
        for ($i = 0; $i -lt $Coordinates.Length - 2; $i += 2) {
            $graphics.DrawLine($pen, $Coordinates[$i], $Coordinates[$i + 1], $Coordinates[$i + 2], $Coordinates[$i + 3])
        }
    } finally { $pen.Dispose() }
}

$redPen = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#C74C45'), 3)
$whitePen = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#F0F0EF'), 2.7)
$whitePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$whitePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$graphics.DrawEllipse($redPen, 37, 44, 52, 52)
$graphics.DrawLine($whitePen, 51, 70, 75, 70)
$graphics.DrawLine($whitePen, 63, 58, 63, 82)
$titleFont = [System.Drawing.Font]::new('Segoe UI Semibold', 23, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$smallFont = [System.Drawing.Font]::new('Segoe UI', 9, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$titleBrush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#EFEFEE'))
$mutedBrush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#A0A1A5'))
$graphics.DrawString('F O R G E', $titleFont, $titleBrush, 32, 110)
$graphics.DrawString('R O O M', $titleFont, $titleBrush, 32, 139)
$graphics.DrawString('E A R L Y   P R E V I E W', $smallFont, $mutedBrush, 37, 180)

# A crisp, opaque architectural room with a restrained ember doorway.
Draw-Line '#56585B' @(0, 310, 47, 279, 47, 405, 0, 436)
Draw-Line '#56585B' @(0, 310, 94, 342, 224, 283)
Draw-Line '#424448' @(47, 279, 94, 294, 224, 238)
Draw-Line '#3C3E42' @(94, 294, 94, 417, 224, 361)
Draw-Line '#4B4D51' @(117, 331, 194, 298, 194, 386, 117, 417, 117, 331)
Draw-Line '#333539' @(125, 336, 186, 310, 186, 382, 125, 407, 125, 336)
$doorBrush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#A4423D'))
$doorPoints = [System.Drawing.PointF[]]@([System.Drawing.PointF]::new(173, 325), [System.Drawing.PointF]::new(194, 317), [System.Drawing.PointF]::new(194, 386), [System.Drawing.PointF]::new(173, 394))
$graphics.FillPolygon($doorBrush, $doorPoints)
Draw-Line '#D16359' @(173, 325, 194, 317, 194, 386, 173, 394, 173, 325)
Draw-Line '#333539' @(0, 437, 94, 417, 224, 447)
foreach ($y in @(447, 460, 476, 493, 515)) { Draw-Line '#292B2E' @(0, $y, 94, 417, 224, ($y - 20)) 0.6 }
foreach ($x in @(0, 30, 65, 110, 165, 224)) { Draw-Line '#292B2E' @(94, 417, $x, 520) 0.6 }
Draw-Line '#343639' @(224, 0, 224, 520) 1

$bitmap.Save((Join-Path $OutputDirectory 'installer-art.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Save((Join-Path $OutputDirectory 'installer-art.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$graphics.Dispose()
$bitmap.Dispose()
foreach ($resource in @($redPen, $whitePen, $titleFont, $smallFont, $titleBrush, $mutedBrush, $doorBrush)) { $resource.Dispose() }

$iconBitmap = [System.Drawing.Bitmap]::new(256, 256)
$iconGraphics = [System.Drawing.Graphics]::FromImage($iconBitmap)
$iconGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$iconGraphics.Clear([System.Drawing.ColorTranslator]::FromHtml('#121314'))
$iconRed = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#C74C45'), 15)
$iconWhite = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#F0F0EF'), 12)
$iconWhite.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$iconWhite.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$iconGraphics.DrawEllipse($iconRed, 40, 40, 176, 176)
$iconGraphics.DrawLine($iconWhite, 85, 128, 171, 128)
$iconGraphics.DrawLine($iconWhite, 128, 85, 128, 171)
$icon = [System.Drawing.Icon]::FromHandle($iconBitmap.GetHicon())
$iconStream = [System.IO.File]::Create((Join-Path $OutputDirectory 'installer.ico'))
try { $icon.Save($iconStream) } finally { $iconStream.Dispose(); $icon.Dispose() }
$iconGraphics.Dispose()
$iconBitmap.Dispose()
$iconRed.Dispose()
$iconWhite.Dispose()
