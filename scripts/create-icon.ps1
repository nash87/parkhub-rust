# Create a simple parking icon for the Windows application
Add-Type -AssemblyName System.Drawing

$size = 256
$bmp = New-Object System.Drawing.Bitmap($size, $size)

$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Background - Securanido brand blue
$blueBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 41, 128, 185))
$g.FillRectangle($blueBrush, 0, 0, $size, $size)

# White P letter for Parking
$font = New-Object System.Drawing.Font("Segoe UI", 150, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$sf = New-Object System.Drawing.StringFormat
$sf.Alignment = [System.Drawing.StringAlignment]::Center
$sf.LineAlignment = [System.Drawing.StringAlignment]::Center
$rectF = New-Object System.Drawing.RectangleF(0, -10, $size, $size)
$g.DrawString("P", $font, $whiteBrush, $rectF, $sf)

# Small car shape hint at bottom
$carBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(200, 255, 255, 255))
$g.FillRectangle($carBrush, 85, 210, 86, 25)
$g.FillEllipse($carBrush, 95, 225, 20, 20)
$g.FillEllipse($carBrush, 141, 225, 20, 20)

# Ensure assets directory exists
$assetsDir = "C:\dev\securanido-parking-desktop\assets"
if (-not (Test-Path $assetsDir)) {
    New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null
}

# Save as PNG
$bmp.Save("$assetsDir\app.png", [System.Drawing.Imaging.ImageFormat]::Png)

# Create ICO file
$icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
$stream = New-Object System.IO.FileStream("$assetsDir\app.ico", [System.IO.FileMode]::Create)
$icon.Save($stream)
$stream.Close()

# Clean up
$sf.Dispose()
$font.Dispose()
$whiteBrush.Dispose()
$carBrush.Dispose()
$blueBrush.Dispose()
$g.Dispose()
$bmp.Dispose()
$icon.Dispose()

Write-Host "Icon created successfully at $assetsDir\app.ico"
