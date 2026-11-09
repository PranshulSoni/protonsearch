# Regenerate EVERY ProtonSearch logo/icon from the single brand source.
# Source of truth: assets/brand/protonsearch-mark.svg
# Outputs:
#   apps/desktop/src-tauri/icons/*      -> window, tray, exe, taskbar, installer icon
#   apps/desktop/public/favicon*        -> WebView/browser surfaces
#   apps/desktop/src-tauri/icons/nsis-* -> installer header + sidebar artwork
param(
    [switch]$SkipTauriIcon
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    Write-Host "==> Rendering brand artwork from assets/brand/protonsearch-mark.svg"
    & node tools\visual-qa\brand-render.mjs
    if ($LASTEXITCODE -ne 0) { throw "brand render failed" }

    if (-not $SkipTauriIcon) {
        Write-Host "==> Generating all platform icon sizes (tauri icon)"
        Push-Location apps\desktop
        try { & npx tauri icon src-tauri\icons\source-icon.png | Select-Object -Last 4 } finally { Pop-Location }
    }

    Write-Host "==> Converting NSIS artwork to 24-bit BMP"
    Add-Type -AssemblyName System.Drawing
    function Write-Bmp([string]$png, [string]$bmpPath) {
        $src = [System.Drawing.Image]::FromFile((Resolve-Path $png))
        try {
            $bmp = New-Object System.Drawing.Bitmap($src.Width, $src.Height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.Clear([System.Drawing.Color]::White)
            $g.DrawImage($src, 0, 0, $src.Width, $src.Height)
            $g.Dispose()
            $bmp.Save($bmpPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
            $bmp.Dispose()
        } finally { $src.Dispose() }
    }
    Write-Bmp 'apps\desktop\src-tauri\icons\nsis-header.png'  'apps\desktop\src-tauri\icons\nsis-header.bmp'
    Write-Bmp 'apps\desktop\src-tauri\icons\nsis-sidebar.png' 'apps\desktop\src-tauri\icons\nsis-sidebar.bmp'

    Write-Host "==> Installing favicon.ico for the WebView"
    Copy-Item 'apps\desktop\src-tauri\icons\icon.ico' 'apps\desktop\public\favicon.ico' -Force

    Write-Host "==> Publishing runtime brand assets"
    $runtime = 'assets\brand\runtime'
    New-Item -ItemType Directory -Force $runtime | Out-Null
    Copy-Item 'apps\desktop\public\favicon-16x16.png'  "$runtime\protonsearch-16.png"  -Force
    Copy-Item 'apps\desktop\public\favicon-32x32.png'  "$runtime\protonsearch-32.png"  -Force
    Copy-Item 'apps\desktop\src-tauri\icons\128x128.png' "$runtime\protonsearch-128.png" -Force
    Copy-Item 'apps\desktop\src-tauri\icons\source-icon.png' "$runtime\protonsearch-1024.png" -Force
    Copy-Item 'apps\desktop\src-tauri\icons\icon.ico' "$runtime\protonsearch.ico" -Force

    Write-Host "==> Brand assets ready"
    Get-ChildItem apps\desktop\src-tauri\icons -File -Include icon.ico,32x32.png,128x128.png,nsis-*.bmp -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object Name,Length | Format-Table -AutoSize | Out-String | Write-Host
    Get-ChildItem apps\desktop\public -File | Select-Object Name,Length | Format-Table -AutoSize | Out-String | Write-Host
}
finally { Pop-Location }