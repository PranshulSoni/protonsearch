"""Measure sidecar startup, run duration and idle cost on this machine.

Usage:
    agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_sidecar.py

Spawns `python -m protonsearch_agent` in deterministic test-model mode, sends
one AgentRunRequest, auto-approves every approval_request, then measures:
  startup_ms     spawn -> first stdout event (includes Python + pydantic-ai import)
  run_ms         run request written -> finished event
  idle_cpu_ms    child CPU time consumed during a 2.0 s idle window after the run
  idle_rss_mb    child working set after that idle window
Every number is printed with the command that produced it. No network, no keys.
"""

from __future__ import annotations

import json
import os
import queue
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

SIDECAR_DIR = Path(__file__).resolve().parent.parent
PYTHON = SIDECAR_DIR / ".venv" / "Scripts" / "python.exe"
IDLE_SECONDS = 2.0


def _win_process_counters(proc: subprocess.Popen) -> tuple[float, float]:
    """Return (cpu_ms, rss_mb) for a child process on Windows."""
    import ctypes
    from ctypes import wintypes

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    handle = ctypes.c_void_p(int(proc._handle))  # type: ignore[attr-defined]

    class FILETIME(ctypes.Structure):
        _fields_ = [("dwLowDateTime", wintypes.DWORD), ("dwHighDateTime", wintypes.DWORD)]

    class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    creation, exit_time, kernel, user = FILETIME(), FILETIME(), FILETIME(), FILETIME()
    kernel32.GetProcessTimes(handle, ctypes.byref(creation), ctypes.byref(exit_time), ctypes.byref(kernel), ctypes.byref(user))
    cpu_ms = (
        ((kernel.dwHighDateTime << 32) | kernel.dwLowDateTime)
        + ((user.dwHighDateTime << 32) | user.dwLowDateTime)
    ) / 10_000.0
    counters = PROCESS_MEMORY_COUNTERS()
    counters.cb = ctypes.sizeof(PROCESS_MEMORY_COUNTERS)
    if psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
        rss_mb = counters.WorkingSetSize / (1024 * 1024)
    else:
        rss_mb = float("nan")
    return cpu_ms, rss_mb


def _read_events(stream, output: queue.Queue) -> None:
    for line in stream:
        output.put(line)
    output.put(None)


def measure_once(index: int) -> dict:
    env = dict(os.environ)
    env.update(
        {
            "PROTONSEARCH_AGENT_TEST_MODEL": "1",
            "PYTHONIOENCODING": "utf-8",
            "PYTHONUNBUFFERED": "1",
        }
    )
    proc = subprocess.Popen(
        [str(PYTHON), "-m", "protonsearch_agent"],
        cwd=str(SIDECAR_DIR),
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        bufsize=1,
    )
    output: queue.Queue = queue.Queue()
    threading.Thread(target=_read_events, args=(proc.stdout, output), daemon=True).start()
    stderr_chunks: list[str] = []
    threading.Thread(target=lambda: stderr_chunks.append(proc.stderr.read()), daemon=True).start()

    spawn_ns = time.perf_counter()
    request = {"runId": f"measure-{index}", "prompt": "find the quarterly budget file"}
    # Write the run request immediately; the child reads it once its imports are
    # done, so spawn -> first event includes interpreter + pydantic-ai import.
    proc.stdin.write(json.dumps(request) + "\n")
    proc.stdin.flush()
    request_ns = time.perf_counter()
    first_event_ms = None
    run_ms = None
    events: list[str] = []
    ok = False
    try:
        while True:
            line = output.get(timeout=45)
            if line is None:
                break
            event = json.loads(line)
            now_ms = (time.perf_counter() - spawn_ns) * 1000
            if first_event_ms is None:
                first_event_ms = now_ms
            events.append(str(event.get("type")))
            if event.get("type") == "approval_request":
                response = {
                    "type": "approval_response",
                    "runId": request["runId"],
                    "approvalId": event["approval_id"],
                    "approved": True,
                }
                proc.stdin.write(json.dumps(response) + "\n")
                proc.stdin.flush()
            if event.get("type") == "finished":
                run_ms = (time.perf_counter() - request_ns) * 1000
                ok = bool(event.get("ok"))
                break
        time.sleep(IDLE_SECONDS)
        cpu_ms, rss_mb = _win_process_counters(proc)
        proc.stdin.close()
        exit_code = proc.wait(timeout=15)
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=10)

    return {
        "index": index,
        "startup_ms": None if first_event_ms is None else round(first_event_ms, 1),
        "run_ms": None if run_ms is None else round(run_ms, 1),
        "idle_window_s": IDLE_SECONDS,
        "idle_cpu_ms": round(cpu_ms, 1),
        "idle_rss_mb": round(rss_mb, 2),
        "events": events,
        "ok": ok,
        "exit_code": exit_code,
        "stderr_tail": ("".join(stderr_chunks))[-400:].strip(),
    }


def main() -> int:
    if not PYTHON.is_file():
        print(f"ERROR: venv python not found at {PYTHON}")
        return 2
    runs = [measure_once(i) for i in range(1, 4)]
    startup = [r["startup_ms"] for r in runs if r["startup_ms"] is not None]
    run_times = [r["run_ms"] for r in runs if r["run_ms"] is not None]
    summary = {
        "command": "agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_sidecar.py",
        "python": str(PYTHON),
        "runs": runs,
        "startup_ms_median": round(statistics.median(startup), 1) if startup else None,
        "run_ms_median": round(statistics.median(run_times), 1) if run_times else None,
        "idle_cpu_ms_max_of_runs": max((r["idle_cpu_ms"] for r in runs), default=None),
        "all_runs_ok": all(r["ok"] for r in runs),
    }
    print(json.dumps(summary, indent=2))
    return 0 if summary["all_runs_ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
