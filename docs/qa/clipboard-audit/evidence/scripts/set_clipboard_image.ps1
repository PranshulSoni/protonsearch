Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bitmap = New-Object System.Drawing.Bitmap 320, 200
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.Clear([System.Drawing.Color]::White)
$font = New-Object System.Drawing.Font('Segoe UI', 22)
$graphics.DrawString('Clipboard audit .NET SetImage 2026', $font, [System.Drawing.Brushes]::DarkBlue, 8, 60)
$graphics.Dispose()
[System.Windows.Forms.Clipboard]::SetImage($bitmap)
$bitmap.Dispose()
Write-Output 'dotnet-setimage-ok'
