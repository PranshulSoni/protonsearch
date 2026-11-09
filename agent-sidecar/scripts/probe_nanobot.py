
import importlib, importlib.metadata, inspect, os, time, sys
sys.stdout.reconfigure(encoding="utf-8")

def timed(label, fn):
    t0 = time.perf_counter()
    try:
        v = fn()
        print(f"{label}_MS", round((time.perf_counter()-t0)*1000,1), "OK")
        return v
    except Exception as e:
        print(f"{label}_MS", round((time.perf_counter()-t0)*1000,1), "ERR", type(e).__name__, str(e)[:300])
        return None

nanobot = timed("IMPORT_TOPCORE", lambda: importlib.import_module("nanobot"))
print("VERSION", importlib.metadata.version("nanobot-ai"))
print("EXPORTS", [n for n in dir(nanobot) if not n.startswith("_")][:60])

md = importlib.metadata.metadata("nanobot-ai")
print("LICENSE", md.get("License"))
print("REQUIRES_PYTHON", md.get("Requires-Python"))
desc = md.get("Description") or ""
lines = [ln.strip() for ln in desc.splitlines() if ln.strip()]
print("README_LINES", len(lines))
print("README_START")
for ln in lines[:30]:
    print("  ", ln[:160].encode("ascii", "replace").decode())

for mod in ("nanobot.config", "nanobot.agent", "nanobot.sdk", "nanobot.nanobot", "nanobot.providers"):
    timed("IMPORT_" + mod.replace(".", "_").upper(), lambda m=mod: importlib.import_module(m))

agent_pkg = importlib.import_module("nanobot.agent")
print("AGENT_FILES", sorted(os.listdir(os.path.dirname(agent_pkg.__file__)))[:40])

eps = importlib.metadata.entry_points()
for ep in eps.select(group="console_scripts"):
    if "nanobot" in ep.name:
        print("ENTRY", ep.name, ep.value)

# public API classes in key modules
for modname in ("nanobot.nanobot", "nanobot.agent", "nanobot.sdk", "nanobot.config"):
    mod = importlib.import_module(modname)
    classes = [n for n in dir(mod) if not n.startswith("_") and inspect.isclass(getattr(mod, n))]
    print("CLASSES", modname, classes[:30])

# Does the sdk expose a test/no-key path? search for TestModel / fake model / mock
root = os.path.dirname(nanobot.__file__)
hits = []
for dirpath, dirnames, filenames in os.walk(root):
    dirnames[:] = [d for d in dirnames if d not in ("__pycache__",)]
    for fn in filenames:
        if fn.endswith(".py"):
            p = os.path.join(dirpath, fn)
            try:
                text = open(p, encoding="utf-8", errors="replace").read()
            except Exception:
                continue
            for pat in ("TestModel", "FakeModel", "MockModel", "class LLM", "class Model", "def chat"):
                if pat in text:
                    hits.append((os.path.relpath(p, root), pat))
print("TEST_MODEL_HITS", sorted(set(hits))[:25])
