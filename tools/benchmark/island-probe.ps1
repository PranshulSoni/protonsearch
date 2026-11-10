# Release-native Dynamic Island card-switch probe (black box; no CDP, debug-only).
# Starts UI-thread latency (WM_NULL round trip) and window-rect sampling BEFORE the
# summon Alt+Space, starts pixel sampling once the island is visible, then sends
# Ctrl+Down card switches at -GapMs spacing. One shared Stopwatch time base (ms since
# probe start). Writes -OutFile as JSON; prints one compact summary line on stdout.
# On failure writes {"error": "..."} to -OutFile and exits 1.
param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [int]$Switches = 16,
    [int]$GapMs = 900,
    [Parameter(Mandatory = $true)][string]$OutFile
)

$ErrorActionPreference = "Stop"

$signature = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class IslandProbeWin32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam, uint fuFlags, uint uTimeout, out IntPtr lpdwResult);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    public static readonly Stopwatch Clock = new Stopwatch();
    public static readonly List<double> Lat = new List<double>();
    public static readonly List<double[]> Stalls = new List<double[]>();
    public static readonly List<double[]> Rects = new List<double[]>();
    public static readonly List<double> PixelChanges = new List<double>();
    public static long PixelSamples;
    public static int ErrorCount;
    public static int ScreenW, ScreenH;
    public static int[] InitialRect = new int[] { 0, 0, 0, 0 };
    public static double HotkeyAtMs = -1, SummonVisibleMs = -1, SummonFocusedMs = -1;
    public static double SampleStartMs, SampleEndMs, PixelStartMs;
    static volatile bool running;
    static Thread uiThread, rectThread, pixThread;
    static int pxX, pxY, pxW, pxH;

    public static void Begin() { Clock.Start(); }
    public static double NowMs() { return Clock.Elapsed.TotalMilliseconds; }

    public static double Pct(double[] s, double p) {
        int idx = (int)Math.Ceiling(p / 100.0 * s.Length) - 1;
        if (idx < 0) idx = 0;
        if (idx >= s.Length) idx = s.Length - 1;
        return s[idx];
    }

    // Any top-level window of the PID with this exact title (hidden or visible).
    public static IntPtr FindTitled(int pid, string title) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr l) {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != (uint)pid) return true;
            var sb = new StringBuilder(GetWindowTextLength(h) + 2);
            GetWindowText(h, sb, sb.Capacity);
            if (sb.ToString() == title) { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static void SendAltSpace() {
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x20, 0, 0, UIntPtr.Zero);
        keybd_event(0x20, 0, 2, UIntPtr.Zero);
        keybd_event(0x12, 0, 2, UIntPtr.Zero);
    }

    public static void SendCtrlDown() {
        keybd_event(0x11, 0, 0, UIntPtr.Zero);
        Thread.Sleep(30);
        keybd_event(0x28, 0, 1, UIntPtr.Zero);
        Thread.Sleep(30);
        keybd_event(0x28, 0, 1 | 2, UIntPtr.Zero);
        Thread.Sleep(30);
        keybd_event(0x11, 0, 2, UIntPtr.Zero);
    }

    // Starts UI-latency and rect samplers (called before the summon hotkey).
    public static void StartUiAndRect(IntPtr h) {
        ScreenW = GetSystemMetrics(0);
        ScreenH = GetSystemMetrics(1);
        running = true;
        SampleStartMs = NowMs();
        uiThread = new Thread(new ThreadStart(delegate() { UiLoop(h); })) { IsBackground = true };
        rectThread = new Thread(new ThreadStart(delegate() { RectLoop(h); })) { IsBackground = true };
        uiThread.Start();
        rectThread.Start();
    }

    public static bool IsVisible(IntPtr h) { return IsWindowVisible(h); }

    // null when the island is visible and foreground (safe to send keys), else the reason.
    public static string KeyGuard(IntPtr h) {
        if (!IsWindowVisible(h)) return "island not visible";
        if (GetForegroundWindow() != h) return "island not foreground";
        return null;
    }

    // Sends Alt+Space if the island is hidden, then polls until visible AND foreground.
    // Starts the pixel sampler the moment the window first shows (needs its rect).
    public static bool Summon(IntPtr h, int timeoutMs) {
        if (IsWindowVisible(h)) { HotkeyAtMs = -1; }
        else { HotkeyAtMs = NowMs(); SendAltSpace(); }
        double t0 = NowMs();
        while (NowMs() - t0 < timeoutMs) {
            if (IsWindowVisible(h)) {
                if (SummonVisibleMs < 0) { SummonVisibleMs = NowMs(); StartPixelSampling(h); }
                if (GetForegroundWindow() == h) { SummonFocusedMs = NowMs(); return true; }
            }
            Thread.Sleep(1);
        }
        return false;
    }

    static void StartPixelSampling(IntPtr h) {
        RECT r;
        GetWindowRect(h, out r);
        int w = r.Right - r.Left;
        InitialRect = new int[] { r.Left, r.Top, w, r.Bottom - r.Top };
        int x0 = Math.Max(0, r.Left - 120);
        int x1 = Math.Min(ScreenW, r.Left + w + 120);
        pxX = x0;
        pxY = 0;
        pxW = Math.Max(1, x1 - x0);
        pxH = Math.Max(1, Math.Min(ScreenH, 900));
        PixelStartMs = NowMs();
        pixThread = new Thread(new ThreadStart(PixelLoop)) { IsBackground = true };
        pixThread.Start();
    }

    public static void StopSampling() {
        running = false;
        SampleEndMs = NowMs();
        Thread[] ts = new Thread[] { uiThread, rectThread, pixThread };
        foreach (Thread t in ts) { if (t != null) t.Join(5000); }
    }

    static void UiLoop(IntPtr h) {
        try {
            while (running) {
                double t0 = NowMs();
                IntPtr res;
                SendMessageTimeout(h, 0, IntPtr.Zero, IntPtr.Zero, 0, 3000, out res);
                double lat = NowMs() - t0;
                Lat.Add(lat);
                if (lat >= 20.0) Stalls.Add(new double[] { t0, lat });
                Thread.Sleep(4);
            }
        } catch (Exception) { Interlocked.Increment(ref ErrorCount); }
    }

    static void RectLoop(IntPtr h) {
        try {
            bool first = true;
            int pl = 0, pt = 0, pw = 0, ph = 0;
            while (running) {
                RECT r;
                if (GetWindowRect(h, out r)) {
                    int w = r.Right - r.Left, hh = r.Bottom - r.Top;
                    if (first || r.Left != pl || r.Top != pt || w != pw || hh != ph) {
                        Rects.Add(new double[] { NowMs(), r.Left, r.Top, w, hh });
                        pl = r.Left; pt = r.Top; pw = w; ph = hh; first = false;
                    }
                }
                Thread.Sleep(1);
            }
        } catch (Exception) { Interlocked.Increment(ref ErrorCount); }
    }

    static void PixelLoop() {
        try {
            using (Bitmap bmp = new Bitmap(pxW, pxH, PixelFormat.Format32bppArgb))
            using (Graphics g = Graphics.FromImage(bmp)) {
                bool first = true;
                ulong prev = 0;
                while (running) {
                    double t0 = NowMs();
                    try { g.CopyFromScreen(pxX, pxY, 0, 0, new Size(pxW, pxH), CopyPixelOperation.SourceCopy); }
                    catch (Exception) { Thread.Sleep(5); continue; }
                    BitmapData bd = bmp.LockBits(new Rectangle(0, 0, pxW, pxH), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
                    ulong hash;
                    try { hash = Fnv(bd.Scan0, bd.Stride, pxW, pxH); }
                    finally { bmp.UnlockBits(bd); }
                    PixelSamples++;
                    if (first || hash != prev) { PixelChanges.Add(t0); prev = hash; first = false; }
                }
            }
        } catch (Exception) { Interlocked.Increment(ref ErrorCount); }
    }

    // FNV-1a over 32-bit pixels, every 6th pixel in x and y.
    static ulong Fnv(IntPtr scan0, int stride, int w, int h) {
        ulong hash = 14695981039346656037UL;
        unchecked {
            for (int y = 0; y < h; y += 6) {
                for (int x = 0; x < w; x += 6) {
                    int v = Marshal.ReadInt32(scan0, y * stride + x * 4);
                    hash ^= (ulong)(uint)v;
                    hash *= 1099511628211UL;
                }
            }
        }
        return hash;
    }
}
'@
if (-not ([System.Management.Automation.PSTypeName]'IslandProbeWin32').Type) {
    Add-Type -TypeDefinition $signature -ReferencedAssemblies System.Drawing
}

function Write-Result([string]$json) {
    [System.IO.File]::WriteAllText($OutFile, $json, (New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false))
}

function Fail-Probe([string]$msg) {
    $json = ConvertTo-Json -InputObject ([pscustomobject]@{ error = $msg }) -Compress
    Write-Result $json
    Write-Output $json
    exit 1
}

function Get-Pct([double[]]$sorted, [double]$p) {
    if ($null -eq $sorted -or $sorted.Length -eq 0) { return $null }
    return [math]::Round([IslandProbeWin32]::Pct($sorted, $p), 3)
}

[void][IslandProbeWin32]::SetProcessDpiAwarenessContext([IntPtr](-4))
[IslandProbeWin32]::Begin()
$startEpochMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

try {
    $hwnd = [IslandProbeWin32]::FindTitled($ProcessId, "ProtonSearch Island")
    if ($hwnd.ToInt64() -eq 0) { Fail-Probe "island window 'ProtonSearch Island' not found for pid $ProcessId" }
    [IslandProbeWin32]::StartUiAndRect($hwnd)
    if (-not [IslandProbeWin32]::Summon($hwnd, 5000)) { Fail-Probe "island window not visible and focused within 5000 ms of Alt+Space" }
    Start-Sleep -Milliseconds 800

    $keys = New-Object System.Collections.Generic.List[object]
    $skipRun = 0
    for ($i = 1; $i -le $Switches; $i++) {
        $t = [math]::Round([IslandProbeWin32]::NowMs(), 3)
        $reason = [IslandProbeWin32]::KeyGuard($hwnd)
        if ($null -eq $reason) {
            $keys.Add([pscustomobject]@{ i = $i; t = $t; sent = $true })
            $skipRun = 0
            [IslandProbeWin32]::SendCtrlDown()
        } else {
            # never send keys into another window: record the skip instead
            $keys.Add([pscustomobject]@{ i = $i; t = $t; sent = $false; reason = $reason })
            $skipRun++
            if ($skipRun -ge 3) { break }
        }
        Start-Sleep -Milliseconds $GapMs
    }
    Start-Sleep -Milliseconds 800
    [IslandProbeWin32]::StopSampling()
    if ([IslandProbeWin32]::IsVisible($hwnd)) { [IslandProbeWin32]::SendAltSpace() }
    Start-Sleep -Milliseconds 600

    $lat = [IslandProbeWin32]::Lat.ToArray()
    [Array]::Sort($lat)
    $durS = ([IslandProbeWin32]::SampleEndMs - [IslandProbeWin32]::SampleStartMs) / 1000.0
    $pxDurS = ([IslandProbeWin32]::SampleEndMs - [IslandProbeWin32]::PixelStartMs) / 1000.0
    $uiRate = if ($durS -gt 0) { [math]::Round($lat.Length / $durS, 2) } else { 0 }
    $pxRate = if ($pxDurS -gt 0) { [math]::Round([IslandProbeWin32]::PixelSamples / $pxDurS, 2) } else { 0 }

    $stallRows = New-Object System.Collections.Generic.List[object]
    foreach ($s in [IslandProbeWin32]::Stalls) {
        $stallRows.Add([pscustomobject]@{ t = [math]::Round($s[0], 3); lat = [math]::Round($s[1], 3) })
    }
    $rectRows = New-Object System.Collections.Generic.List[object]
    foreach ($r in [IslandProbeWin32]::Rects) {
        $rectRows.Add([pscustomobject]@{ t = [math]::Round($r[0], 3); l = [int]$r[1]; top = [int]$r[2]; w = [int]$r[3]; h = [int]$r[4] })
    }
    $pcs = [IslandProbeWin32]::PixelChanges.ToArray()
    $ir = [IslandProbeWin32]::InitialRect
    $hk = [IslandProbeWin32]::HotkeyAtMs

    $out = [pscustomobject]@{
        startEpochMs = $startEpochMs
        hwnd = $hwnd.ToInt64()
        hotkeyAt = $(if ($hk -ge 0) { [math]::Round($hk, 3) } else { $null })
        summonVisibleMs = $(if ($hk -ge 0) { [math]::Round([IslandProbeWin32]::SummonVisibleMs - $hk, 3) } else { $null })
        summonFocusedMs = $(if ($hk -ge 0) { [math]::Round([IslandProbeWin32]::SummonFocusedMs - $hk, 3) } else { $null })
        initialRect = [pscustomobject]@{ left = $ir[0]; top = $ir[1]; width = $ir[2]; height = $ir[3] }
        keys = $keys.ToArray()
        uiLatency = [pscustomobject]@{
            samples = $lat.Length
            count = $stallRows.Count
            p50 = (Get-Pct $lat 50)
            p95 = (Get-Pct $lat 95)
            p99 = (Get-Pct $lat 99)
            max = $(if ($lat.Length -gt 0) { [math]::Round($lat[$lat.Length - 1], 3) } else { $null })
            rateHz = $uiRate
        }
        stalls = $stallRows.ToArray()
        rects = $rectRows.ToArray()
        pixelChanges = $pcs
        pixelSamples = [IslandProbeWin32]::PixelSamples
        pixelRateHz = $pxRate
        screenW = [IslandProbeWin32]::ScreenW
        screenH = [IslandProbeWin32]::ScreenH
        threadErrors = [IslandProbeWin32]::ErrorCount
        samplingMs = [math]::Round(([IslandProbeWin32]::SampleEndMs - [IslandProbeWin32]::SampleStartMs), 1)
    }
    Write-Result (ConvertTo-Json -InputObject $out -Depth 6)

    $summary = [pscustomobject]@{
        ok = $true; outFile = $OutFile; switches = $Switches; keys = $keys.Count
        stalls = $stallRows.Count; rects = $rectRows.Count; pixelChanges = $pcs.Length
        pixelSamples = [IslandProbeWin32]::PixelSamples; uiP95 = $out.uiLatency.p95
        summonVisibleMs = $out.summonVisibleMs; summonFocusedMs = $out.summonFocusedMs
    }
    Write-Output (ConvertTo-Json -InputObject $summary -Compress)
    exit 0
} catch {
    Fail-Probe ("probe exception: " + $_.Exception.Message)
}
