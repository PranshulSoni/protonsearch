"""Read-only OS credential lookup (Windows Credential Manager).

Placeholder by design: the sidecar only *reads* a credential that the app (or
the user) provisioned; it never writes credentials, never bundles keys and
never logs them. Non-Windows hosts return None so the sidecar still runs.

Target names checked, in order:
  1. `protonsearch/agent-api-key`
  2. `protonsearch-agent-api-key`

The caller falls back to environment variables when nothing is stored. On this
host no credential was created during the spike, so a successful read is
UNVERIFIED; a missing credential returning None is covered by tests.
"""

from __future__ import annotations

import os
import sys

TARGETS = ("protonsearch/agent-api-key", "protonsearch-agent-api-key")


def read_secret(target: str | None = None) -> str | None:
    """Return the stored API key for `target`, or None when unavailable."""
    if sys.platform != "win32":
        return None
    try:
        import ctypes
        from ctypes import wintypes
    except Exception:  # pragma: no cover - ctypes is always present on CPython
        return None

    class CREDENTIALW(ctypes.Structure):
        _fields_ = [
            ("Flags", wintypes.DWORD),
            ("Type", wintypes.DWORD),
            ("TargetName", wintypes.LPWSTR),
            ("Comment", wintypes.LPWSTR),
            ("LastWritten", wintypes.FILETIME),
            ("CredentialBlobSize", wintypes.DWORD),
            ("CredentialBlob", ctypes.POINTER(ctypes.c_ubyte)),
            ("Persist", wintypes.DWORD),
            ("AttributeCount", wintypes.DWORD),
            ("Attributes", ctypes.c_void_p),
            ("TargetAlias", wintypes.LPWSTR),
            ("UserName", wintypes.LPWSTR),
        ]

    advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    advapi32.CredReadW.restype = wintypes.BOOL
    advapi32.CredReadW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        ctypes.POINTER(ctypes.POINTER(CREDENTIALW)),
    ]
    advapi32.CredFree.restype = None
    advapi32.CredFree.argtypes = [ctypes.c_void_p]

    for candidate in (target,) if target else TARGETS:
        pointer = ctypes.POINTER(CREDENTIALW)()
        ok = advapi32.CredReadW(candidate, 1, 0, ctypes.byref(pointer))  # CRED_TYPE_GENERIC
        if not ok or not pointer:
            continue
        try:
            credential = pointer.contents
            size = int(credential.CredentialBlobSize)
            if size <= 0 or not credential.CredentialBlob:
                return None
            blob = ctypes.string_at(credential.CredentialBlob, size)
            for encoding in ("utf-16-le", "utf-8"):
                try:
                    value = blob.decode(encoding).rstrip("\x00")
                except UnicodeDecodeError:
                    continue
                if value:
                    return value
            return None
        finally:
            advapi32.CredFree(pointer)

    return None


def api_key_from_env_or_store() -> str | None:
    """Environment first (dev/CI), OS store second (installed app)."""
    for name in ("PROTONSEARCH_AGENT_API_KEY",):
        value = os.environ.get(name)
        if value and value.strip():
            return value.strip()
    return read_secret()
