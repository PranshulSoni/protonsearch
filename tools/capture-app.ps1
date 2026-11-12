# Capture real ProtonSearch windows to PNG for visual evidence.
# Usage: pwsh -File tools/capture-app.ps1 -ProcessName ProtonSearch -OutDir docs/qa/visual/app-captures
param(
  [string]$ProcessName = "ProtonSearch",
  [string]$OutDir = "docs/qa/visual/app-captures",
  [int]$WaitMs = 0
)
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace Win32 -Name Gdi -MemberDefinition @'
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder lpString, int nMaxCount);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdcBlt, uint nFlags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
'@
# Per-monitor-v2 DPI awareness so window rects are physical pixels.
try { [void][Win32.Gdi]::SetProcessDpiAwarenessContext([IntPtr]::new(-4)) } catch {}

if ($WaitMs -gt 0) { Start-Sleep -Milliseconds $WaitMs }
$procs = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
if (-not $procs) { Write-Error "process '$ProcessName' is not running"; exit 1 }
$pidSet = @($procs | ForEach-Object { $_.Id })
New-Item -ItemType Directory -Force $OutDir | Out-Null

$windows = New-Object System.Collections.ArrayList
$callback = {
  param([IntPtr]$hwnd, [IntPtr]$lparam)
  [uint32]$procId = 0
  [void][Win32.Gdi]::GetWindowThreadProcessId($hwnd, [ref]$procId)
  if ($pidSet -contains $procId -and [Win32.Gdi]::IsWindowVisible($hwnd)) {
    $len = [Win32.Gdi]::GetWindowTextLength($hwnd)
    $sb = New-Object System.Text.StringBuilder ($len + 1)
    [void][Win32.Gdi]::GetWindowText($hwnd, $sb, $sb.Capacity)
    $rect = New-Object Win32.Gdi+RECT
    [void][Win32.Gdi]::GetWindowRect($hwnd, [ref]$rect)
    $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
    if ($w -gt 80 -and $h -gt 80) {
      [void]$windows.Add([pscustomobject]@{ Hwnd = $hwnd; Title = $sb.ToString(); Left = $rect.Left; Top = $rect.Top; Width = $w; Height = $h })
    }
  }
  return $true
}
[void][Win32.Gdi]::EnumWindows($callback, [IntPtr]::Zero)

$index = 0
foreach ($win in $windows) {
  $index++
  $label = if ($win.Title) { ($win.Title -replace '[^a-zA-Z0-9_-]', '-') } else { "window" }
  $file = Join-Path $OutDir ("{0:D2}-{1}-{2}x{3}.png" -f $index, $label, $win.Width, $win.Height)
  $bmp = New-Object System.Drawing.Bitmap $win.Width, $win.Height
  $gfx = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $gfx.GetHdc()
  # 2 = PW_RENDERFULLCONTENT (WebView2-friendly); fall back to screen copy.
  $ok = [Win32.Gdi]::PrintWindow($win.Hwnd, $hdc, 2)
  $gfx.ReleaseHdc($hdc)
  $gfx.Dispose()
  if (-not $ok) {
    [void][Win32.Gdi]::SetForegroundWindow($win.Hwnd)
    Start-Sleep -Milliseconds 300
    $bmp = New-Object System.Drawing.Bitmap $win.Width, $win.Height
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.CopyFromScreen($win.Left, $win.Top, 0, 0, (New-Object System.Drawing.Size($win.Width, $win.Height)))
    $gfx.Dispose()
  }
  $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  Write-Output ("captured {0} ({1}x{2}) -> {3}" -f $win.Title, $win.Width, $win.Height, $file)
}
if ($windows.Count -eq 0) { Write-Error "no visible windows for process '$ProcessName'"; exit 1 }
