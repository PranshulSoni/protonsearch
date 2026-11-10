# Process-tree memory / CPU metrics for the app baseline. OWNER: perf-bench (L1-C).
# Modes:
#   memory: instant snapshot of the tree rooted at -ProcessId
#   cpu: samples CPU time deltas for -SampleSeconds
# Output: one JSON line.
param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [ValidateSet("memory", "cpu")][string]$Mode = "memory",
    [int]$SampleSeconds = 10
)

$ErrorActionPreference = "Stop"

function Get-Tree([int]$rootId) {
    $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, WorkingSetSize, PrivatePageCount, CommandLine
    $map = @{}
    foreach ($p in $all) { $map[[int]$p.ProcessId] = $p }
    $tree = New-Object System.Collections.Generic.List[object]
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($rootId)
    $seen = New-Object 'System.Collections.Generic.HashSet[int]'
    while ($queue.Count -gt 0) {
        $current = $queue.Dequeue()
        if (-not $seen.Add($current)) { continue }
        $proc = $map[$current]
        if ($null -eq $proc) { continue }
        $tree.Add($proc)
        foreach ($p in $all) {
            if ([int]$p.ParentProcessId -eq $current) { $queue.Enqueue([int]$p.ProcessId) }
        }
    }
    return $tree
}

function Cpu-Seconds($processIds) {
    $total = 0.0
    foreach ($id in $processIds) {
        try {
            $p = Get-Process -Id $id -ErrorAction Stop
            $total += $p.TotalProcessorTime.TotalSeconds
        } catch { }
    }
    return $total
}

$tree = Get-Tree $ProcessId
if ($Mode -eq "memory") {
    $ids = @($tree | ForEach-Object { [int]$_.ProcessId })
    $cpuNow = Cpu-Seconds $ids
    [pscustomobject]@{
        mode = "memory"
        rootPid = $ProcessId
        capturedAtUtc = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        processCount = $tree.Count
        totalWorkingSetBytes = [int64](($tree | Measure-Object -Property WorkingSetSize -Sum).Sum)
        totalPrivateBytes = [int64](($tree | Measure-Object -Property PrivatePageCount -Sum).Sum)
        processes = @($tree | ForEach-Object { [pscustomobject]@{ pid = [int]$_.ProcessId; name = $_.Name; wsBytes = [int64]$_.WorkingSetSize; privateBytes = [int64]$_.PrivatePageCount; commandLine = $(if ($_.CommandLine) { $_.CommandLine.Substring(0, [Math]::Min(400, $_.CommandLine.Length)) } else { "" }) } })
        cpuSecondsTotal = [math]::Round($cpuNow, 3)
    } | ConvertTo-Json -Compress -Depth 4
} else {
    $ids = @($tree | ForEach-Object { [int]$_.ProcessId })
    $cores = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    $startCpu = Cpu-Seconds $ids
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $SampleSeconds
    $sw.Stop()
    $endCpu = Cpu-Seconds $ids
    $delta = $endCpu - $startCpu
    $elapsed = $sw.Elapsed.TotalSeconds
    [pscustomobject]@{
        mode = "cpu"
        rootPid = $ProcessId
        sampleSeconds = [math]::Round($elapsed, 3)
        processCount = $ids.Count
        cpuSecondsDelta = [math]::Round($delta, 3)
        logicalCores = $cores
        cpuPctOfOneCore = if ($elapsed -gt 0) { [math]::Round(($delta / $elapsed) * 100, 2) } else { 0 }
        cpuPctOfMachine = if ($elapsed -gt 0 -and $cores -gt 0) { [math]::Round((($delta / $elapsed) / $cores) * 100, 2) } else { 0 }
    } | ConvertTo-Json -Compress -Depth 4
}
