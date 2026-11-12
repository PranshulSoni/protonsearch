Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Native {
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@
$out = Join-Path (Get-Location) 'docs\qa\g8-verify-round3'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$islandHwnd = [IntPtr]9635370
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Behind text window (opacity check)'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(0,0)
$form.Size = New-Object System.Drawing.Size(1920, 620)
$form.BackColor = [System.Drawing.Color]::White
$label = New-Object System.Windows.Forms.Label
$label.Dock = 'Fill'
$label.Font = New-Object System.Drawing.Font('Segoe UI', 13)
$label.ForeColor = [System.Drawing.Color]::Black
$lines = (1..24 | ForEach-Object { "Line $_ - the island must be opaque over this text and must not bleed it through. ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789" })
$label.Text = ($lines -join [Environment]::NewLine)
$form.Controls.Add($label)
$form.Show()
$form.Activate()
[System.Windows.Forms.Application]::DoEvents()
Start-Sleep -Milliseconds 800
[Native]::SetForegroundWindow($islandHwnd) | Out-Null
Start-Sleep -Milliseconds 400
function Capture([string]$name) {
  $bmp = New-Object System.Drawing.Bitmap 900, 300
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen(510, 0, 0, 0, (New-Object System.Drawing.Size 900, 300))
  $path = Join-Path $out $name
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  return $path
}
$opacityPath = Capture 'release-opacity-over-text.png'
[Native]::SetForegroundWindow($islandHwnd) | Out-Null
Start-Sleep -Milliseconds 500
$frames = @()
function SendHotkey { [Native]::keybd_event(0x12,0,0,[UIntPtr]::Zero); [Native]::keybd_event(0x20,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [Native]::keybd_event(0x20,0,2,[UIntPtr]::Zero); [Native]::keybd_event(0x12,0,2,[UIntPtr]::Zero) }
SendHotkey
for ($i=0; $i -lt 12; $i++) { $frames += (Capture ('release-exit-' + $i.ToString('00') + '.png')); Start-Sleep -Milliseconds 35 }
Start-Sleep -Milliseconds 500
SendHotkey
for ($i=0; $i -lt 12; $i++) { $frames += (Capture ('release-enter-' + $i.ToString('00') + '.png')); Start-Sleep -Milliseconds 35 }
Start-Sleep -Milliseconds 300
$form.Close()
'OPACITY=' + $opacityPath
'FRAMES=' + ($frames -join ',')
