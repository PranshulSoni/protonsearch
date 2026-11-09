# Security — ProtonSearch v2 (consolidated release view)

This is the release-facing summary. The full legacy threat model is in
**docs/security/SECURITY_RISK_REGISTER.md** (Gate 0, commit de0ef19); the v2 code review is in
**docs/security/V2_SECURITY_REVIEW.md** (no criticals; must-fixes listed). Nothing here replaces
those documents.

## 1. Legacy exposure that must be handled before any v2 release

| # | Severity | Issue | Action |
|---|---|---|---|
| S-01 | **CRITICAL** | A full AI API key was committed to git history (masked sk-Hr...***, 67 chars; commits 41b56c8, 1bfe506, 11f0dfe, d55c3ad). Repo is public. | **Rotate/revoke the key now.** History rewrite is secondary. Add secret scanning to CI. |
| S-02 | **CRITICAL** | Legacy updater fetches mutable update.json, downloads an EXE to %TEMP%, runs it with no signature/hash check (settings_ui.rs:677-900). | v2 must ship a signed manifest (Ed25519/minisign pinned key) + SHA-256 + Authenticode signer pin before executing any update. G7 gate. |
| S-03 | HIGH | Legacy Hermes gateway uses a static default key ("hermes") on loopback with terminal/computer_use tools enabled (ai.rs:519-622). | v2 sidecar: random per-install token, loopback peer checks, tools off by default, explicit approvals. |
| S-04 | HIGH | Legacy stores AI keys plaintext in SQLite and passes them on child argv (settings_ui.rs:1145, main.rs:4925-4928). | v2: DPAPI/Credential Manager; never argv; no plaintext keys in DB/logs. |
| S-05 | HIGH | Legacy "run powershell ..." search prefix executes arbitrary PowerShell without confirmation (main.rs:4479-4545). | v2 must not ship this path; if kept, explicit confirmation + allowlist. |
| S-06 | HIGH | Fetched web content can prompt-inject a tool-enabled agent; URL fetch is SSRF-capable (ai.rs:1163-1172,1248-1266). | Treat fetched content as data; disable tools for summarize; block private/loopback IPs; audit agent actions. |
| S-07 | HIGH | Legacy clipboard capture + Windows-history import persists secrets/screenshots without opt-in (main.rs:1937-1999,11704-11826). | v2 clipboard capture opt-in/off by default, source-app exclusions, TTL; never send clipboard to AI implicitly. |

Other findings (S-08...S-20) and positive controls are in the register.

## 1b. v2 code review status (2026-11-09)

docs/security/V2_SECURITY_REVIEW.md reviewed migration, engine, Tauri command surface,
ps-windows and ps-content. No criticals were found. Post-review fixes verified: migration retry
state (V2-01), DB+WAL+SHM fingerprinting (V2-02), imported-count reporting (V2-03), server-side
15 s confirmation handshake for destructive native actions (V2-04), CDP disabled in release
builds (V2-05), clipboard opt-in by default (V2-06), migration URI encoding (V2-09). Residual
risks: clipboard DIB pixel cap (V2-07), app-target validation depth (V2-08), activity-log
retention UX (V2-10). See the review document for file:line evidence.

## 2. v2 security requirements (release gates)

1. **No secrets in the repo or build artifacts.** CI secret scan must pass. No embedded or
   shared operator keys in clients — BYO key.
2. **Credential storage:** Windows DPAPI or Credential Manager; redact from logs, crash
   dumps, command lines and support bundles.
3. **Update integrity:** signed manifest + artifact SHA-256 + Authenticode signer pin; bounded
   download; no execution without verification (G7).
4. **Agent surface:** read-only by default; loopback token auth + peer check; tool calls require
   approval; file/web content is never treated as instructions (G6).
5. **Data minimisation:** clipboard capture opt-in/off; exclude hidden/credential paths by
   default; migration never imports AI keys silently (G4/G5).
6. **Migration safety:** legacy DB opened read-only; no legacy uninstaller invocation; rollback
   snapshot before import (G4).
7. **Installer:** per-user, no elevation requirement; uninstall keeps user data by default and
   asks separately for deletion (G7).
8. **Dependency hygiene:** cargo audit/cargo deny + lockfile review; licence notices generated
   (see THIRD_PARTY_LICENSES.md) at G7.

## 3. Disclosure policy

Report vulnerabilities **privately** via GitHub Security Advisories on
github.com/PranshulSoni/protonsearch ("Security -> Advisories -> Report a vulnerability").
Do not open a public issue for a vulnerability. Do not publish exploit details before a fix
and key rotation are available. (No private email address is invented here; the maintainer
must add one explicitly if desired.)

## 4. Secret-handling rules for this repository

- Never print, commit, paste into issues, or test real keys.
- Report leaked material as masked prefix + length + file:line, and rotate immediately.
- Any key ever committed is considered compromised forever (forks/caches).
- Do not add service credentials to CI; workflows use permissions: contents: read only.

## 5. Audit/gate commitments (G7)

- cargo audit (or cargo deny check advisories) clean or documented exceptions.
- Secret scan clean on the release commit.
- Installer signature verified (signtool verify /pa) and recorded.
- SHA256SUMS published and signed; update manifest signature verified.
- No unsafe expansion without review in ps-windows (Win32 confinement).
