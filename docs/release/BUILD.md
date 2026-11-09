# Build Guide — ProtonSearch v2 (alpha)

Status: **alpha scaffold (v2.0.0-alpha.1), not distributable.** This guide records the
exact toolchain and commands verified on the audit host; anything not yet run is marked
**UNVERIFIED**. Evidence base: docs/audit/GATE0_BASELINE.md (legacy) and the probes below.

## 1. Toolchain facts (audit host, 2026-11-09)

| Tool | Version / path | How verified |
|---|---|---|
| OS | Windows 11 Home Single Language 10.0.**26300**, x64 | Get-CimInstance Win32_OperatingSystem |
| CPU / RAM | AMD Ryzen AI 9 465, 10C/20T, 31.2 GB | Win32_Processor / Win32_ComputerSystem |
| rustc | **1.99.0** (b940084d7 2026-09-28) | rustc --version |
| cargo | **1.99.0** (5f94df478 2026-08-27) | cargo --version |
| Node | **v26.7.0** | node --version |
| npm | **11.19.0** | npm --version |
| Python (host) | 3.14.7 (msys64, no pip) | python --version |
| Python (test tooling) | agent-sidecar venv 3.14 + PyYAML 6.0.3 | .venv\Scripts\python.exe -c "import yaml" |
| git | C:\Program Files\Git\cmd\git.exe | where.exe git |
| Tauri CLI | @tauri-apps/cli 2.12.4 (npm devDependency) | apps/desktop/package.json |
| Tauri Rust | tauri 2.12.5, tauri-build 2.7.4 | cargo metadata (544 resolved packages) |
| Frontend | React 19.3.1, Vite 8.3.7, TypeScript 5.9.3, Vitest 5.1.0 | package-lock.json / npm |
| Bundle target | NSIS, installMode=currentUser | apps/desktop/src-tauri/tauri.conf.json |

No rust-toolchain.toml exists in the repo today (checked). CI pins stable; the versions
above are the reproducibility record for this workspace as of Gate 0.

## 2. Lockfiles (reproducibility)

| File | Size | Git state at audit | Rule |
|---|---:|---|---|
| Cargo.lock (workspace root) | 138,226 B | **committed** | CI uses the resolved graph |
| apps/desktop/package-lock.json | 59,942 B | **committed** | CI uses npm ci |
| tools/visual-qa/package-lock.json | 1,210 B | **committed** | visual-QA tooling (dev-only) |
| protonsearch/Cargo.lock (legacy) | 168,711 B | tracked in the legacy tree | legacy only, not part of v2 build |

.gitignore excludes target/, apps/desktop/node_modules/, apps/desktop/dist/ but does **not**
exclude lockfiles — committing them is a Lead action, not a config change. Do not build a
release artifact while either lockfile is dirty/untracked (the installer script warns).

## 3. Data directory (do not write to AppData in tests)

crates/ps-core/src/paths.rs:6-19 implements a PROTONSEARCH_DATA_DIR override:
- default: %APPDATA%\protonsearch\ (legacy) with v2 data under ...\protonsearch\v2\;
- override: PROTONSEARCH_DATA_DIR is used as the base directly; v2 data then lives under
  <override>\v2 (v2_data_dir() joins v2 to the returned base).

**Sandbox/test-environment limitation (not product behaviour):** child processes in this
sandbox cannot write %APPDATA%. All E2E/migration tests must set PROTONSEARCH_DATA_DIR to a
temp directory. The migration fixture test does this by construction.

## 4. Commands

All commands run from the repo root unless noted. The legacy crate (protonsearch/) is
excluded from the workspace and must not be built as part of v2.

```powershell
# --- Rust workspace ---
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --workspace --release --locked

# --- Frontend (apps/desktop) ---
cd apps/desktop
npm ci
npm test                          # vitest suites land with the UI work
npm run build                     # tsc --noEmit && vite build
npx tauri build --bundles nsis    # UNSIGNED NSIS bundle (manual only)

# --- Installer wrapper (dry-run by default) ---
powershell -NoProfile -File scripts/build-installer.ps1
powershell -NoProfile -File scripts/build-installer.ps1 -Execute   # only after Lead confirms green

# --- Migration fixture contract test ---
python tests/migration/make_legacy_fixture.py --out-dir <temp-dir>
agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v
```

## 5. CI

.github/workflows/ci.yml (YAML validated locally with PyYAML 6.0.3 → YAML_OK):
- rust (windows-latest): cargo fmt --all -- --check, cargo clippy --workspace --all-targets
  -- -D warnings, cargo test --workspace --no-fail-fast.
- frontend (ubuntu-latest): npm ci, npm test (vitest), npm run build (tsc --noEmit + vite build).
- migration (windows-latest): generates the synthetic legacy fixture, builds the dev-only probe
  and calls ps_core::migration::migrate_legacy_db; uploads evidence. No real user data in CI.
- tauri-build (manual workflow_dispatch only): builds the unsigned NSIS bundle and uploads it
  as a 7-day artifact. No secrets, permissions: contents: read, never publishes.

**Honest current state (Gate 0):** the workflow is the target gate, not yet green:
cargo fmt --all -- --check currently reports diffs on the in-progress scaffold, and the
frontend has transient TS errors in files owned by the UI agents. Re-run after G1.

## 6. Build artifacts (expected locations, UNVERIFIED until built)

- apps/desktop/src-tauri/target/release/ProtonSearch.exe
- apps/desktop/src-tauri/target/release/bundle/nsis/*.exe
- dist-installer/ (wrapper copies + SHA256SUMS.txt + BUILD_INFO.txt)

**Unsigned-build caveat:** artifacts are not Authenticode-signed. Signing + signed update
manifest is a G7 gate (docs/security/SECURITY_RISK_REGISTER.md S-02/S-17). Do not publish.

## 7. Reproducibility checklist

1. Commit both lockfiles (above) and record the git commit in release notes.
2. Build with --locked (cargo) and npm ci (frontend) — never re-resolve dependencies.
3. Record rustc/cargo/node/npm versions and git rev-parse HEAD in BUILD_INFO.txt (the
   installer script does this automatically).
4. Publish SHA-256 for every artifact; sign them before public distribution.
5. Rebuild on a clean machine and compare hashes; note non-determinism rather than hiding it.
