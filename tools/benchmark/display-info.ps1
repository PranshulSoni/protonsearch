# Display/DPI facts for benchmark conditions. OWNER: perf-bench.
Add-Type -AssemblyName System.Windows.Forms | Out-Null
$screens = [System.Windows.Forms.Screen]::AllScreens
$primary = [System.Windows.Forms.Screen]::PrimaryScreen
$gpu = Get-CimInstance Win32_VideoController | Select-Object -First 1 Name, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate
$dpi = 96
try {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DpiProbe { [DllImport("user32.dll")] public static extern uint GetDpiForSystem(); }
'@
    $dpi = [DpiProbe]::GetDpiForSystem()
} catch { }
[pscustomobject]@{
    screenCount = $screens.Count
    primaryBounds = @{ width = $primary.Bounds.Width; height = $primary.Bounds.Height }
    allScreens = @($screens | ForEach-Object { @{ bounds = $_.Bounds.ToString(); workingArea = $_.WorkingArea.ToString(); primary = $_.Primary } })
    gpu = $gpu
    systemDpi = $dpi
    scalePercent = [math]::Round(($dpi / 96.0) * 100)
} | ConvertTo-Json -Compress -Depth 5
