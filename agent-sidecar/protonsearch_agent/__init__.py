"""ProtonSearch optional agent sidecar (protocol v1).

Run with::

    python -m protonsearch_agent

See agent-sidecar/README.md and docs/agent/INTEGRATION.md. This package is
started on demand by the Rust client (crates/ps-agent); it is never imported
by, or on, the search path of the desktop engine.
"""

__version__ = "0.1.0"
PROTOCOL_VERSION = 1
