Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$out = Join-Path (Get-Location) 'docs\qa\g8-verify-round3'
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Behind text window (opacity check)'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(0,0)
$form.Size = New-Object System.Drawing.Size(1920, 700)
$form.BackColor = [System.Drawing.Color]::White
$label = New-Object System.Windows.Forms.Label
$label.Dock = 'Fill'
$label.Font = New-Object System.Drawing.Font('Segoe UI', 14)
$label.ForeColor = [System.Drawing.Color]::Black
$lines = (1..26 | ForEach-Object { "Opacity check line $_ - if you can read this text THROUGH the island, opacity is broken. ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789" })
$label.Text = ($lines -join [Environment]::NewLine)
$form.Controls.Add($label)
$form.Show()
$form.Activate()
[System.Windows.Forms.Application]::DoEvents()
Start-Sleep -Milliseconds 1200
$bmp = New-Object System.Drawing.Bitmap 1300, 380
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen(320, 0, 0, 0, (New-Object System.Drawing.Size 1300, 380))
$path = Join-Path $out 'release-opacity-over-text.png'
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Start-Sleep -Milliseconds 300
$form.Close()
'OPACITY=' + $path
