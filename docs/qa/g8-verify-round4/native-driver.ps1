# native-driver.ps1 - native desktop driver for ProtonSearch G8 round-4 QA.
# Windows PowerShell 5.1. The process sets per-monitor DPI v2, so ALL coordinates are PHYSICAL pixels.
# One subcommand per call; each prints ONE compact JSON line. Failures print {"error":...} and exit 1.
#
#   capture -X 0 -Y 0 -W 200 -H 100 -Out <png>   (or -Full -Out <png> for the primary screen)
#   click   -X 500 -Y 300 [-Double]              left click; -Double = two quick clicks
#   keys    -Keys "ctrl+down ctrl+up esc"        chords split by spaces, 40 ms apart
#   type    -Text "hello"                        Unicode to the foreground window, 20 ms/char
#   hotkey                                       Alt+Space (the app's global hotkey)
#   rects   -ProcessId <pid>                     visible top-level windows of a PID
#   behind  -Seconds 20 -X 100 -Y 100 -W 600 -H 400
#                                                plain white text window; returns {hwnd} once
#                                                shown; closes itself after -Seconds

param(
    [string]$Cmd = "",
    [int]$X = 0,
    [int]$Y = 0,
    [int]$W = 0,
    [int]$H = 0,
    [string]$Out = "",
    [switch]$Full,
    [switch]$Double,
    [string]$Keys = "",
    [string]$Text,
    [int]$ProcessId = 0,
    [int]$Seconds = 20,
    [switch]$Foreground,
    [string]$StateFile = ""
)

$ErrorActionPreference = "Stop"

$signature = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class PsNative {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] public struct HARDWAREINPUT { public uint uMsg; public ushort wParamL; public ushort wParamH; }
    [StructLayout(LayoutKind.Explicit)] public struct InputUnion {
        [FieldOffset(0)] public MOUSEINPUT mi;
        [FieldOffset(0)] public KEYBDINPUT ki;
        [FieldOffset(0)] public HARDWAREINPUT hi;
    }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public InputUnion U; }
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
    [DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);

    public static bool SetDpiPerMonitorV2() { return SetProcessDpiAwarenessContext((IntPtr)(-4)); }

    // One KEYEVENTF_UNICODE key-down + key-up pair for a UTF-16 code unit.
    public static uint TypeUnit(ushort unit) {
        INPUT[] a = new INPUT[2];
        a[0].type = 1; a[0].U.ki.wScan = unit; a[0].U.ki.dwFlags = 0x0004;           // KEYEVENTF_UNICODE
        a[1].type = 1; a[1].U.ki.wScan = unit; a[1].U.ki.dwFlags = 0x0004 | 0x0002;  // + KEYEVENTF_KEYUP
        return SendInput(2, a, Marshal.SizeOf(typeof(INPUT)));
    }

    public static List<IntPtr> TopLevelWindows(uint pid) {
        var list = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h) && !IsIconic(h)) list.Add(h);
            return true;
        }, IntPtr.Zero);
        return list;
    }

    public static int[] Rect(IntPtr h) {
        RECT r;
        GetWindowRect(h, out r);
        return new int[] { r.Left, r.Top, r.Right - r.Left, r.Bottom - r.Top };
    }

    public static string Title(IntPtr h) {
        var sb = new StringBuilder(GetWindowTextLength(h) + 2);
        GetWindowText(h, sb, sb.Capacity);
        return sb.ToString();
    }
}
'@
if (-not ([System.Management.Automation.PSTypeName]'PsNative').Type) {
    Add-Type -TypeDefinition $signature
}
# Must happen before any coordinate or window call, or coordinates are DPI-virtualized.
if (-not [PsNative]::SetDpiPerMonitorV2()) {
    throw "SetProcessDpiAwarenessContext(per-monitor v2) failed; coordinates would not be physical"
}

$VkMap = @{
    'ctrl' = 0x11; 'control' = 0x11; 'alt' = 0x12; 'shift' = 0x10; 'win' = 0x5B; 'meta' = 0x5B
    'esc' = 0x1B; 'escape' = 0x1B; 'enter' = 0x0D; 'return' = 0x0D; 'tab' = 0x09; 'space' = 0x20
    'backspace' = 0x08; 'insert' = 0x2D; 'delete' = 0x2E; 'del' = 0x2E; 'home' = 0x24; 'end' = 0x23
    'pageup' = 0x21; 'pagedown' = 0x22; 'left' = 0x25; 'up' = 0x26; 'right' = 0x27; 'down' = 0x28
}
# Arrows and nav keys need KEYEVENTF_EXTENDEDKEY (1).
$ExtendedVk = @(0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2D, 0x2E)

function Out-Json([object]$Obj) {
    Write-Output (ConvertTo-Json -InputObject $Obj -Compress -Depth 5)
}

function Get-Vk([string]$Name) {
    $n = $Name.ToLowerInvariant()
    if ($VkMap.ContainsKey($n)) { return [int]$VkMap[$n] }
    if ($n -match '^f([1-9]|1[0-2])$') { return 0x6F + [int]$Matches[1] }
    if ($n -match '^[a-z0-9]$') { return [int][char]$n.ToUpperInvariant() }
    throw "unknown key '$Name'"
}

function Send-Chord([string]$Chord) {
    $parts = $Chord.ToLowerInvariant().Split('+')
    $vk = Get-Vk ($parts[-1])
    $mods = @()
    for ($i = 0; $i -lt $parts.Count - 1; $i++) { $mods += (Get-Vk $parts[$i]) }
    $ext = if ($ExtendedVk -contains $vk) { 1 } else { 0 }
    foreach ($m in $mods) { [PsNative]::keybd_event([byte]$m, 0, 0, [UIntPtr]::Zero) }
    [PsNative]::keybd_event([byte]$vk, 0, $ext, [UIntPtr]::Zero)
    [PsNative]::keybd_event([byte]$vk, 0, ($ext -bor 2), [UIntPtr]::Zero)
    for ($i = $mods.Count - 1; $i -ge 0; $i--) { [PsNative]::keybd_event([byte]$mods[$i], 0, 2, [UIntPtr]::Zero) }
}

try {
    switch ($Cmd) {
        'capture' {
            if ($Full) {
                $X = 0; $Y = 0
                $W = [PsNative]::GetSystemMetrics(0)
                $H = [PsNative]::GetSystemMetrics(1)
            }
            if ($W -le 0 -or $H -le 0) { throw "capture needs -W and -H > 0 (or -Full)" }
            if (-not $Out) { throw "capture needs -Out <png>" }
            $path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Out)
            $dir = Split-Path -Parent $path
            if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
            Add-Type -AssemblyName System.Drawing
            $bmp = New-Object System.Drawing.Bitmap -ArgumentList $W, $H
            try {
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                try { $g.CopyFromScreen($X, $Y, 0, 0, (New-Object System.Drawing.Size -ArgumentList $W, $H)) }
                finally { $g.Dispose() }
                $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
            }
            finally { $bmp.Dispose() }
            Out-Json ([pscustomobject]@{ path = $path; w = $W; h = $H })
        }
        'click' {
            if (-not [PsNative]::SetCursorPos($X, $Y)) { throw "SetCursorPos($X,$Y) failed" }
            Start-Sleep -Milliseconds 60   # let the move land before pressing
            $n = 1
            if ($Double) { $n = 2 }
            for ($i = 0; $i -lt $n; $i++) {
                if ($i -gt 0) { Start-Sleep -Milliseconds 60 }   # stay inside the OS double-click time
                [PsNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # MOUSEEVENTF_LEFTDOWN
                Start-Sleep -Milliseconds 30
                [PsNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # MOUSEEVENTF_LEFTUP
            }
            Out-Json ([pscustomobject]@{ x = $X; y = $Y })
        }
        'keys' {
            if (-not $Keys) { throw 'keys needs -Keys, e.g. -Keys "ctrl+down ctrl+up"' }
            $tokens = @($Keys.Trim() -split '\s+' | Where-Object { $_ })
            foreach ($t in $tokens) { foreach ($p in $t.Split('+')) { [void](Get-Vk $p) } }   # validate all before pressing
            for ($i = 0; $i -lt $tokens.Count; $i++) {
                if ($i -gt 0) { Start-Sleep -Milliseconds 40 }
                Send-Chord $tokens[$i]
            }
            Out-Json ([pscustomobject]@{ sent = $tokens })
        }
        'type' {
            if ($null -eq $Text) { throw 'type needs -Text' }
            $n = 0
            foreach ($ch in $Text.ToCharArray()) {
                $sent = [PsNative]::TypeUnit([uint16][char]$ch)
                if ($sent -ne 2) { throw "SendInput accepted $sent of 2 events (no foreground window, or UIPI blocked)" }
                $n++
                Start-Sleep -Milliseconds 20
            }
            Out-Json ([pscustomobject]@{ typed = $n })
        }
        'hotkey' {
            $sentAtUtc = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            Send-Chord 'alt+space'
            Out-Json ([pscustomobject]@{ sentAtUtc = $sentAtUtc })
        }
        'rects' {
            if ($ProcessId -le 0) { throw 'rects needs -ProcessId' }
            $fg = [PsNative]::GetForegroundWindow().ToInt64()
            $rows = [System.Collections.Generic.List[object]]::new()
            foreach ($hw in [PsNative]::TopLevelWindows([uint32]$ProcessId)) {
                $r = [PsNative]::Rect($hw)
                if ($r[2] -le 0 -or $r[3] -le 0) { continue }
                $row = [pscustomobject]@{
                    hwnd       = $hw.ToInt64()
                    title      = [PsNative]::Title($hw)
                    left       = $r[0]
                    top        = $r[1]
                    width      = $r[2]
                    height     = $r[3]
                    foreground = ($hw.ToInt64() -eq $fg)
                }
                $rows.Add($row)
            }
            Out-Json $rows
        }
        'behind' {
            if ($W -le 0 -or $H -le 0) { throw 'behind needs -W and -H > 0' }
            if ($Foreground) {
                # Child: plain white window, 40 lines of black 12 pt text, pumped until -Seconds elapse.
                Add-Type -AssemblyName System.Windows.Forms, System.Drawing
                $lines = 1..40 | ForEach-Object { 'BEHIND LINE {0:D2}  lorem ipsum dolor sit amet consectetur adipiscing' -f $_ }
                $form = New-Object System.Windows.Forms.Form
                $form.Text = 'native-driver behind'
                $form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
                $form.Bounds = New-Object System.Drawing.Rectangle -ArgumentList $X, $Y, $W, $H
                $form.BackColor = [System.Drawing.Color]::White
                $form.AutoScaleMode = [System.Windows.Forms.AutoScaleMode]::None
                $label = New-Object System.Windows.Forms.Label
                $label.Dock = [System.Windows.Forms.DockStyle]::Fill
                $label.Font = New-Object System.Drawing.Font -ArgumentList 'Consolas', 12
                $label.ForeColor = [System.Drawing.Color]::Black
                $label.Text = $lines -join [Environment]::NewLine
                $form.Controls.Add($label)
                $form.Show()
                # Publish hwnd for the parent: write a temp name, then rename, so the parent never reads a partial file.
                [IO.File]::WriteAllText($StateFile + '.tmp', [string]$form.Handle.ToInt64())
                [IO.File]::Move($StateFile + '.tmp', $StateFile)
                $end = (Get-Date).AddSeconds($Seconds)
                while ((Get-Date) -lt $end) {
                    [System.Windows.Forms.Application]::DoEvents()
                    Start-Sleep -Milliseconds 50
                }
                $form.Close()
                exit 0
            }
            # Parent: start the child with no console window, wait (max 20 s) for its hwnd handshake, return it.
            $state = Join-Path ([IO.Path]::GetTempPath()) ('native-driver-behind-' + [guid]::NewGuid().ToString('N') + '.hwnd')
            $childArgs = '-NoProfile -STA -ExecutionPolicy Bypass -File "{0}" -Cmd behind -Foreground -Seconds {1} -X {2} -Y {3} -W {4} -H {5} -StateFile "{6}"' -f $PSCommandPath, $Seconds, $X, $Y, $W, $H, $state
            # CreateNoWindow, not -WindowStyle Hidden: the hidden STARTUPINFO left the child's first form invisible in testing.
            $psi = New-Object System.Diagnostics.ProcessStartInfo -Property @{ FileName = 'powershell.exe'; Arguments = $childArgs; UseShellExecute = $false; CreateNoWindow = $true }
            [void][System.Diagnostics.Process]::Start($psi)
            $deadline = (Get-Date).AddSeconds(20)
            while (-not (Test-Path -LiteralPath $state) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 100 }
            if (-not (Test-Path -LiteralPath $state)) { throw 'behind window did not appear within 20 s' }
            $hwnd = [int64]((Get-Content -LiteralPath $state -Raw).Trim())
            Remove-Item -LiteralPath $state -ErrorAction SilentlyContinue
            Out-Json ([pscustomobject]@{ hwnd = $hwnd })
        }
        default { throw "unknown or missing -Cmd '$Cmd' (capture|click|keys|type|hotkey|rects|behind)" }
    }
} catch {
    Out-Json ([pscustomobject]@{ error = $_.Exception.Message })
    exit 1
}
exit 0
