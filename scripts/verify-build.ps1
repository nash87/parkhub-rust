# Verify build script
$exePath = "C:\dev\securanido-parking-desktop\target\release\securanido-parking.exe"

Write-Host "=== Build Verification ===" -ForegroundColor Cyan

# Check executable exists
if (Test-Path $exePath) {
    $file = Get-Item $exePath
    Write-Host "Executable: $($file.FullName)" -ForegroundColor Green
    Write-Host "Size: $([math]::Round($file.Length / 1MB, 2)) MB"
    Write-Host "Modified: $($file.LastWriteTime)"

    # Check version info
    $version = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($exePath)
    Write-Host ""
    Write-Host "Product: $($version.ProductName)"
    Write-Host "Version: $($version.ProductVersion)"
    Write-Host "Description: $($version.FileDescription)"

    # Check icon
    Add-Type -AssemblyName System.Drawing
    try {
        $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($exePath)
        if ($icon) {
            Write-Host ""
            Write-Host "Icon: Embedded ($($icon.Width)x$($icon.Height))" -ForegroundColor Green
            $icon.Dispose()
        }
    } catch {
        Write-Host "Icon: Could not extract" -ForegroundColor Yellow
    }
} else {
    Write-Host "Executable not found!" -ForegroundColor Red
    exit 1
}

# Check assets
Write-Host ""
Write-Host "=== Assets ===" -ForegroundColor Cyan
$assetsPath = "C:\dev\securanido-parking-desktop\assets"
if (Test-Path $assetsPath) {
    Get-ChildItem $assetsPath | ForEach-Object {
        Write-Host "  $($_.Name) - $([math]::Round($_.Length / 1KB, 1)) KB"
    }
}

# Check config
Write-Host ""
Write-Host "=== Configuration ===" -ForegroundColor Cyan
$configPath = "C:\dev\securanido-parking-desktop\config"
if (Test-Path $configPath) {
    Get-ChildItem $configPath | ForEach-Object {
        Write-Host "  $($_.Name)"
    }
}

Write-Host ""
Write-Host "=== Verification Complete ===" -ForegroundColor Green
