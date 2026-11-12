# Send the app's global hotkey (default Alt+Space) and measure when its window
# becomes visible and foreground. One take per invocation; callers hide the
# launcher between takes through the app's own IPC (debug build) or a second
# SendOnly invocation (release black-box).
# OWNER: perf-bench (L1-C).
#
# Modes:
#   show  (default): send hotkey, poll windows, report visibleMs/focusedMs
#   probe: no send, poll windows (validates window detection)
#   send: send hotkey only, return immediately (used to toggle hide)
# Output: one JSON line.
param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [int]$TimeoutMs = 3000,
    [ValidateSet("show", "probe", "send")][string]$Mode = "show"
)

$ErrorActionPreference = "Stop"

$signature = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class PsHotkeyWin32 {
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    // Enumerate top-level windows for a PID. Returns "hwnd|visible|w|h|x|y|title".
    public static string[] Windows(int pid) {
        var result = new List<string>();
        EnumWindows((h, l) => {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != (uint)pid) return true;
            RECT r;
            GetWindowRect(h, out r);
            int len = GetWindowTextLength(h);
            var sb = new StringBuilder(len + 2);
            GetWindowText(h, sb, sb.Capacity);
            result.Add(((long)h) + "|" + (IsWindowVisible(h) ? "1" : "0") + "|" + (r.Right - r.Left) + "|" + (r.Bottom - r.Top) + "|" + r.Left + "|" + r.Top + "|" + sb.ToString());
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }

    // Visible top-level windows larger than minW x minH for the PID.
    public static string[] VisibleWindows(int pid, int minW, int minH) {
        var result = new List<string>();
        foreach (var line in Windows(pid)) {
            var parts = line.Split('|');
            if (parts.Length < 7) continue;
            if (parts[1] != "1") continue;
            int w = int.Parse(parts[2]);
            int h = int.Parse(parts[3]);
            if (w >= minW && h >= minH) result.Add(line);
        }
        return result.ToArray();
    }

    public static void SendAltSpace() {
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x20, 0, 0, UIntPtr.Zero);
        keybd_event(0x20, 0, 2, UIntPtr.Zero);
        keybd_event(0x12, 0, 2, UIntPtr.Zero);
    }
}
'@
if (-not ([System.Management.Automation.PSTypeName]'PsHotkeyWin32').Type) {
    Add-Type -TypeDefinition $signature
}

$sendAt = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
if ($Mode -eq "send") {
    [PsHotkeyWin32]::SendAltSpace()
    [pscustomobject]@{ pid = $ProcessId; mode = "send"; sentAtUtc = $sendAt } | ConvertTo-Json -Compress
    exit 0
}
# Windows that are already visible before the hotkey are NOT the launcher we just
# summoned (e.g. the main app window can stay visible in some builds); exclude
# their hwnds so a hotkey->visible reading is not a false positive.
$beforeSet = @{}
foreach ($line in [PsHotkeyWin32]::VisibleWindows($ProcessId, 80, 40)) {
    $beforeSet[($line.Split('|')[0])] = $true
}
if ($Mode -eq "show") {
    [PsHotkeyWin32]::SendAltSpace()
}

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$visibleMs = $null
$focusedMs = $null
$line = $null
$timedOut = $true
$nextEnum = 0L
while ($stopwatch.ElapsedMilliseconds -lt $TimeoutMs) {
    if ($stopwatch.ElapsedMilliseconds -ge $nextEnum) {
        $nextEnum = $stopwatch.ElapsedMilliseconds + 1
        $found = ""
        foreach ($candidate in [PsHotkeyWin32]::VisibleWindows($ProcessId, 80, 40)) {
            $hwnd = $candidate.Split('|')[0]
            if (-not $beforeSet.ContainsKey($hwnd)) { $found = $candidate; break }
        }
        if ($found -ne "") {
            $line = $found
            if ($null -eq $visibleMs) {
                $visibleMs = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
            }
            $parts = $found.Split('|')
            if ([PsHotkeyWin32]::GetForegroundWindow() -eq [IntPtr][long]$parts[0]) {
                $focusedMs = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
                $timedOut = $false
                break
            }
        }
    }
    [System.Threading.Thread]::Sleep(0) | Out-Null
}

$parts = if ($line) { $line.Split('|') } else { @() }
[pscustomobject]@{
    pid = $ProcessId
    mode = $Mode
    sendAtUtc = $sendAt
    visibleMs = $visibleMs
    focusedMs = $focusedMs
    hwnd = if ($parts.Length -ge 7) { [int64]$parts[0] } else { $null }
    title = if ($parts.Length -ge 7) { $parts[6] } else { $null }
    rect = if ($parts.Length -ge 7) { @{ width = [int]$parts[2]; height = [int]$parts[3]; left = [int]$parts[4]; top = [int]$parts[5] } } else { $null }
    timedOut = $timedOut
    visibleBefore = if ($beforeSet.Count -gt 0) { $true } else { $false }
    visibleBeforeCount = $beforeSet.Count
} | ConvertTo-Json -Compress
