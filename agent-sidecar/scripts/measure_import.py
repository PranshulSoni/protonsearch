
"""Measure import cost for a Python package: in-process import ns and version."""
import importlib, sys, time

module = sys.argv[1] if len(sys.argv) > 1 else "pydantic_ai"
t0 = time.perf_counter()
mod = importlib.import_module(module)
ns = (time.perf_counter() - t0) * 1000
try:
    version = importlib.metadata.version(module.replace("_", "-"))
except Exception:
    version = getattr(mod, "__version__", "?")
print(f"module={module} version={version} import_ms={ns:.1f}")
