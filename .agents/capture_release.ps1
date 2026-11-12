$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public delegate bool EnumWindowsProc(IntPtr h, IntPtr lp);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
'@
$fresh = Join-Path $PWD '.devdata-release'
New-Item -ItemType Directory -Force $fresh | Out-Null
$env:PROTONSEARCH_DATA_DIR = $fresh
$exe = Join-Path $PWD 'dist-installer\ProtonSearch.exe'
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 12
New-Item -ItemType Directory -Force docs\qa\visual\app-captures-release | Out-Null
$windows = New-Object System.Collections.ArrayList
$cb = { param($h,$lp) [uint32]$procId=0; [void][W.U]::GetWindowThreadProcessId($h,[ref]$procId); if ($procId -eq $p.Id -and [W.U]::IsWindowVisible($h)) { $r = New-Object W.U+RECT; [void][W.U]::GetWindowRect($h,[ref]$r); if (($r.Right-$r.Left) -gt 200 -and ($r.Bottom-$r.Top) -gt 200) { [void]$windows.Add([pscustomobject]@{H=$h;L=$r.Left;T=$r.Top;W=$r.Right-$r.Left;Ht=$r.Bottom-$r.Top}) } }; return $true }
[void][W.U]::EnumWindows($cb,[IntPtr]::Zero)
echo "window_count=$($windows.Count) alive=$(-not $p.HasExited)"
$i = 0
foreach ($win in $windows) {
  $i++
  [void][W.U]::SetForegroundWindow($win.H)
  Start-Sleep -Milliseconds 500
  $bmp = New-Object System.Drawing.Bitmap $win.W, $win.Ht
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($win.L, $win.T, 0, 0, (New-Object System.Drawing.Size($win.W,$win.Ht)))
  $g.Dispose()
  $file = Join-Path $PWD ("docs\qa\visual\app-captures-release\release-{0}-{1}x{2}.png" -f $i, $win.W, $win.Ht)
  $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  echo "captured $file"
}
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
