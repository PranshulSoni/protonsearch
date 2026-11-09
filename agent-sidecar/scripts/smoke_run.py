"""Manual smoke test: pipe one run through the sidecar and auto-approve."""
import json
import os
import queue
import subprocess
import threading
import time
from pathlib import Path

SIDECAR_DIR = Path(__file__).resolve().parent.parent
PYTHON = SIDECAR_DIR / ".venv" / "Scripts" / "python.exe"
env = dict(os.environ)
env.update({"PROTONSEARCH_AGENT_TEST_MODEL": "1", "PYTHONIOENCODING": "utf-8", "PYTHONUNBUFFERED": "1"})
t0 = time.time()
p = subprocess.Popen(
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
q: queue.Queue = queue.Queue()
threading.Thread(target=lambda: [q.put(("out", line)) for line in p.stdout] or q.put(("out", None)), daemon=True).start()
threading.Thread(target=lambda: [q.put(("err", line)) for line in p.stderr], daemon=True).start()

p.stdin.write(json.dumps({"runId": "r1", "prompt": "find budget", "contextPaths": ["C:/x/budget.txt"]}) + "\n")
p.stdin.flush()
print("request written at", round(time.time() - t0, 2), flush=True)
count = 0
while time.time() - t0 < 60:
    try:
        kind, line = q.get(timeout=20)
    except queue.Empty:
        print("NO EVENT for 20s; alive=", p.poll() is None, flush=True)
        break
    if kind == "err":
        print("STDERR", line.rstrip()[:600], flush=True)
        continue
    if line is None:
        print("EOF; code=", p.wait(timeout=5), flush=True)
        break
    print("EVENT", line.strip()[:400], "at", round(time.time() - t0, 2), flush=True)
    count += 1
    event = json.loads(line)
    if event["type"] == "approval_request":
        resp = {"type": "approval_response", "runId": "r1", "approvalId": event["approval_id"], "approved": True}
        p.stdin.write(json.dumps(resp) + "\n")
        p.stdin.flush()
    if event["type"] == "finished":
        break
try:
    p.stdin.close()
except Exception:
    pass
print("EXIT", p.wait(timeout=15), "elapsed", round(time.time() - t0, 2), flush=True)
