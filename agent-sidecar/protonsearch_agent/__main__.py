"""Entry point: `python -m protonsearch_agent`.

Protocol v1 only: JSON Lines on stdin/stdout, diagnostics on stderr. The host
(packages/ps-agent) starts this process on demand and kills it when idle.
"""

from __future__ import annotations

import json
import sys


def _bootstrap_error(exc: BaseException) -> int:
    message = f"{type(exc).__name__}: {exc}"[:500]
    try:
        sys.stdout.write(
            json.dumps(
                {"type": "error", "code": "runtime_import_failed", "message": message},
                separators=(",", ":"),
            )
            + "\n"
        )
        sys.stdout.flush()
    except OSError:
        pass
    sys.stderr.write(f"protonsearch_agent: failed to start: {message}\n")
    sys.stderr.flush()
    return 1


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    try:
        from .runtime import Sidecar, describe_runtime
    except Exception as exc:  # pragma: no cover - exercised when deps are missing
        return _bootstrap_error(exc)

    if "--describe" in argv:
        sys.stdout.write(json.dumps(describe_runtime(), separators=(",", ":")) + "\n")
        sys.stdout.flush()
        return 0
    return Sidecar().serve()


if __name__ == "__main__":
    raise SystemExit(main())
