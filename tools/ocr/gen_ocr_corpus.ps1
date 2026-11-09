# Ground-truth OCR corpus generator (GDI+ / System.Drawing, built into Windows).
# OWNER: perf-bench. Read-only helper: writes only under -OutDir (default .bench/ocr-corpus).
#
# Produces synthetic images with exactly known text so recognition accuracy can
# be measured separately from speed. These are rendered text, not real scanned
# documents or real screenshots; that limitation is recorded in the benchmark
# report. Deterministic: same inputs -> same scenario/text mapping. PNG bytes may
# differ across Windows builds (font rasterisation); the manifest is the contract.
param(
    [string]$OutDir = ".bench/ocr-corpus",
    [int]$Count = 0,
    [int]$Seed = 20261109
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$manifestPath = Join-Path $OutDir "manifest.jsonl"
if (Test-Path $manifestPath) { Remove-Item $manifestPath -Force }

# 9 scenarios covering common ProtonSearch screenshot/document shapes.
$scenarios = @(
    [pscustomobject]@{ Name = "ui_light";     Font = "Segoe UI";          Px = 14; Fg = "#1B1B1B"; Bg = "#FFFFFF"; W = 820; H = 140; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "ui_dark";      Font = "Segoe UI";          Px = 14; Fg = "#E8E8E8"; Bg = "#1E1E1E"; W = 820; H = 140; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "small_text";   Font = "Segoe UI";          Px = 11; Fg = "#222222"; Bg = "#FFFFFF"; W = 820; H = 140; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "heading";      Font = "Segoe UI";          Px = 30; Fg = "#0F1B2D"; Bg = "#FAF9F5"; W = 820; H = 160; Rotate = 0;   Bold = $true  },
    [pscustomobject]@{ Name = "doc_scan";     Font = "Georgia";           Px = 19; Fg = "#202020"; Bg = "#F4F0E6"; W = 820; H = 200; Rotate = 1.5; Bold = $false },
    [pscustomobject]@{ Name = "code";         Font = "Consolas";          Px = 13; Fg = "#D4D4D4"; Bg = "#1E1E1E"; W = 820; H = 160; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "numbers";      Font = "Segoe UI";          Px = 13; Fg = "#1F1F1F"; Bg = "#FFFFFF"; W = 820; H = 160; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "low_contrast"; Font = "Segoe UI";          Px = 15; Fg = "#8A8A8A"; Bg = "#EDEDED"; W = 820; H = 140; Rotate = 0;   Bold = $false },
    [pscustomobject]@{ Name = "cjk";          Font = "Microsoft YaHei";   Px = 16; Fg = "#1B1B1B"; Bg = "#FFFFFF"; W = 820; H = 140; Rotate = 0;   Bold = $false }
)

$texts = @(
    "Quarterly report final v2.pdf",
    "Search results for index.js in 3 projects",
    "Invoice INV-2026-0042 total 1,849.50 USD",
    "Meeting notes 2026-11-09 14:22 budget review",
    "DNS configuration failed error 0x80004005",
    "const results = index.search(query, 50);",
    "会議議事録 2026-11-09",
    "日本語のテキスト検索テスト",
    "Files modified after 2026-01-01 in C:\Users\Pranshul\Documents",
    "ProtonSearch v2 build 26300 performance"
)

function Resolve-Font([string]$name, [float]$px, [bool]$bold) {
    $style = [System.Drawing.FontStyle]::Regular
    if ($bold) { $style = [System.Drawing.FontStyle]::Bold }
    try {
        $font = New-Object System.Drawing.Font($name, $px, $style, [System.Drawing.GraphicsUnit]::Pixel)
        return $font
    } catch {
        return New-Object System.Drawing.Font("Arial", $px, $style, [System.Drawing.GraphicsUnit]::Pixel)
    }
}

$i = 0
foreach ($sc in $scenarios) {
    foreach ($text in $texts) {
        if ($Count -gt 0 -and $i -ge $Count) { break }
        $file = "img_{0:000}_{1}.png" -f $i, $sc.Name
        $full = Join-Path $OutDir $file
        $bmp = New-Object System.Drawing.Bitmap($sc.W, $sc.H)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
        $g.Clear([System.Drawing.ColorTranslator]::FromHtml($sc.Bg))
        if ($sc.Rotate -ne 0) {
            $g.TranslateTransform($sc.W / 2.0, $sc.H / 2.0)
            $g.RotateTransform([float]$sc.Rotate)
            $g.TranslateTransform(-$sc.W / 2.0, -$sc.H / 2.0)
        }
        $font = Resolve-Font $sc.Font ([float]$sc.Px) $sc.Bold
        $brush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml($sc.Fg))
        $rect = New-Object System.Drawing.RectangleF(24, 20, ($sc.W - 48), ($sc.H - 40))
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = [System.Drawing.StringAlignment]::Near
        $sf.LineAlignment = [System.Drawing.StringAlignment]::Near
        $sf.FormatFlags = [System.Drawing.StringFormatFlags]::NoClip
        $g.DrawString($text, $font, $brush, $rect, $sf)
        $g.Dispose()
        $bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()

        $obj = [pscustomobject]@{
            file     = $file
            text     = $text
            scenario = $sc.Name
            font     = $sc.Font
            px       = [int]$sc.Px
            width    = [int]$sc.W
            height   = [int]$sc.H
        }
        ($obj | ConvertTo-Json -Compress) | Add-Content -Path $manifestPath -Encoding UTF8
        $i++
    }
    if ($Count -gt 0 -and $i -ge $Count) { break }
}

Write-Output ("generated {0} images -> {1}" -f $i, $OutDir)
Write-Output ("manifest: {0}" -f $manifestPath)
