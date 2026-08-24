# Regenerates the app icon: assets/homeacc.png (512px, used for the window and
# taskbar) and assets/homeacc.ico (multi-size, embedded into the .exe by
# build.rs so Explorer shows it too).
#
#   pwsh -File app/assets/make_icon.ps1
#
# A white euro on the same blue the UI uses for its accent. The glyph is set in
# Inter SemiBold — the app's own bold face — so the icon and the interface share
# a typographic voice, and it is positioned by its real ink bounds rather than
# the em box, which is what keeps it optically centred in the tile.

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$S = 512

$fonts = New-Object System.Drawing.Text.PrivateFontCollection
$fonts.AddFontFile((Join-Path $dir 'fonts\Inter-SemiBold.ttf'))
$family = $fonts.Families[0]

function New-RoundedPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

# Euro outline centred on (cx, cy) by the glyph's drawn extent.
function New-EuroPath([float]$size, [float]$cx, [float]$cy) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $p.AddString(
        [string][char]0x20AC, $family, 0, $size,
        (New-Object System.Drawing.PointF 0, 0),
        [System.Drawing.StringFormat]::GenericTypographic)
    $b = $p.GetBounds()
    $m = New-Object System.Drawing.Drawing2D.Matrix
    $m.Translate($cx - ($b.X + $b.Width / 2), $cy - ($b.Y + $b.Height / 2))
    $p.Transform($m)
    return $p
}

$bmp = New-Object System.Drawing.Bitmap $S, $S, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.Clear([System.Drawing.Color]::Transparent)

# --- tile: rounded square with a diagonal blue gradient -------------------
$tile = New-RoundedPath 0 0 $S $S 114
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    (New-Object System.Drawing.Point 0, 0),
    (New-Object System.Drawing.Point $S, $S),
    [System.Drawing.Color]::FromArgb(255, 91, 155, 255),
    [System.Drawing.Color]::FromArgb(255, 23, 64, 184))
$g.FillPath($grad, $tile)

# soft highlight across the top, clipped to the tile
$g.SetClip($tile)
$hi = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    (New-Object System.Drawing.Point 0, 0),
    (New-Object System.Drawing.Point 0, ([int]($S * 0.55))),
    [System.Drawing.Color]::FromArgb(46, 255, 255, 255),
    [System.Drawing.Color]::FromArgb(0, 255, 255, 255))
$g.FillRectangle($hi, 0, 0, $S, [int]($S * 0.55))
$g.ResetClip()

# --- the euro -------------------------------------------------------------
# Nudged 2px below the geometric centre: the tile's corner radius makes a
# perfectly centred mark sit visually high.
$euro = New-EuroPath 330 256 258
$g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)), $euro)

$g.Dispose()
$png = Join-Path $dir 'homeacc.png'
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
"wrote $png ($S x $S)"

# --- multi-size .ico ------------------------------------------------------
# Vista and later accept PNG-encoded entries, which keeps the file small.
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$blobs = @()
foreach ($sz in $sizes) {
    $small = New-Object System.Drawing.Bitmap $sz, $sz, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $sg = [System.Drawing.Graphics]::FromImage($small)
    $sg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $sg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $sg.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $sg.DrawImage($bmp, (New-Object System.Drawing.Rectangle 0, 0, $sz, $sz))
    $sg.Dispose()
    $ms = New-Object System.IO.MemoryStream
    $small.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $blobs += , @{ Size = $sz; Bytes = $ms.ToArray() }
    $ms.Dispose(); $small.Dispose()
}
$bmp.Dispose()

$ico = Join-Path $dir 'homeacc.ico'
$fs = [System.IO.File]::Create($ico)
$bw = New-Object System.IO.BinaryWriter $fs
$bw.Write([uint16]0)                 # reserved
$bw.Write([uint16]1)                 # type: icon
$bw.Write([uint16]$blobs.Count)
$offset = 6 + 16 * $blobs.Count
foreach ($b in $blobs) {
    $dim = if ($b.Size -ge 256) { 0 } else { $b.Size }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim)
    $bw.Write([byte]0); $bw.Write([byte]0)   # palette, reserved
    $bw.Write([uint16]1); $bw.Write([uint16]32)  # planes, bpp
    $bw.Write([uint32]$b.Bytes.Length)
    $bw.Write([uint32]$offset)
    $offset += $b.Bytes.Length
}
foreach ($b in $blobs) { $bw.Write($b.Bytes) }
$bw.Flush(); $bw.Dispose(); $fs.Dispose()
"wrote $ico ($($blobs.Count) sizes, $([math]::Round((Get-Item $ico).Length/1KB,1)) KB)"
