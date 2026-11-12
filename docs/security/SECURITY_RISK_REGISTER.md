# Security Risk Register — ProtonSearch v1.1.1 (Gate 0)

**Owner:** L1-D (rel-recon) / **Date:** 2026-11-09 / **Commit audited:** `de0ef19` (`v1.1.1-v2-baseline`)
**Scope:** legacy app (`protonsearch/`), repo root scripts/metadata, local user data. Read-only audit; no exploit was executed.
**Secret-handling rule:** no secret value is reproduced in this document. Key material found in history is reported only as a **masked prefix + length**, and must be treated as compromised.

---

## 1. Threat model (concrete, not generic)

### Assets
1. **Local index DB** `%APPDATA%\protonsearch\file_index.db` — filenames, full paths, extracted document text, browser history/bookmarks, clipboard text/images + OCR text, window titles, notes, AI/agent chat history.
2. **AI credentials** — `ai_settings.api_key`, `hermes_api_key`, `always_approve`, `%APPDATA%\protonsearch\ai_key.txt` / `opencode_key.txt`, Hermes gateway config, build-time `OMNISEARCH_KEYS`.
3. **User files on disk** — the app can open, index, OCR and upload (Lens) any file the user can read.
4. **Execution surface** — the app spawns PowerShell/cmd/taskkill/git/explorer/hermes/installers, some with user- or AI-derived input.
5. **Update channel** — `update.json` on GitHub + downloaded installer EXE + GitHub release assets.

### Adversaries / threat sources
| # | Adversary | Capability assumed |
|---|---|---|
| A1 | Other local process / other user on same machine | Read `%APPDATA%`, read process command lines, connect to loopback services |
| A2 | Malicious document / image / repo / clipboard content | Arbitrary bytes parsed by PDF/DOCX/OCR/git; text can contain prompt injection |
| A3 | Compromised or malicious update channel (repo write, GitHub account, TLS/CA compromise) | Replace `update.json`, release asset, or installer |
| A4 | Remote AI provider / fetched web page | Returns untrusted text that the AI layer feeds to a tool-enabled agent |
| A5 | User error / destructive feature | Focus-mode killer, Wi-Fi password display, uninstaller data deletion |
| A6 | Attacker with read access to git history / forks | Full historical repo including past committed keys |

### Trust boundaries
UI process ↔ SQLite DB ↔ worker threads ↔ child processes (git/powershell/hermes/installer) ↔ loopback HTTP (Hermes 127.0.0.1:8642) ↔ AI HTTP endpoints ↔ GitHub raw/update ↔ browser profile DBs ↔ OS clipboard.

### Explicitly out of scope
Kernel/OS compromise, attacker with full write access to the user's profile or DB (they already own the data), physical access, and DoS by indexing an enormous corpus.

---

## 2. Findings summary (ranked)

| ID | Severity | Finding | Key evidence |
|---|---|---|---|
| S-01 | **CRITICAL** | Full AI API key committed to git history (repo is public) | commits `41b56c8`, `1bfe506`, `11f0dfe`, `d55c3ad`; masked `sk-Hr…*** len=67` |
| S-02 | **CRITICAL** | Self-updater executes an unsigned, unverified download | `settings_ui.rs:677-900`; `update.json`; `scripts/install.ps1:19-47` |
| S-03 | **HIGH** | Local Hermes agent gateway reachable with a hardcoded default key and tools enabled | `ai.rs:519-522,566-573,580-592,609-620`; `main.rs:1138` |
| S-04 | **HIGH** | AI key stored in plaintext SQLite and passed to child process on argv | `settings_ui.rs:1145`; `main.rs:5973-5997`; `main.rs:4920-4928`; `ai.rs:143-180` |
| S-05 | **HIGH** | Arbitrary PowerShell execution from search input, no confirmation/allowlist | `main.rs:4479-4545` |
| S-06 | **HIGH** | Prompt injection + SSRF via "summarize URL" into tool-enabled agent | `ai.rs:1163-1172,1248-1266`; `main.rs:4857-4867`; `ai.rs:580-592` |
| S-07 | **HIGH** | Clipboard capture/import persists secrets and screenshots without opt-in | `main.rs:1291,1937-1999,11370-11450,11704-11826` |
| S-08 | **MEDIUM** | Focus-session enforcement kills arbitrary processes/windows every 3 s | `launcher.rs:335-380`; `search.rs:10547-10589` |
| S-09 | **MEDIUM** | "Show Wi-Fi Password" action reveals saved Wi-Fi creds (and is broken) | `launcher.rs:643-651`; `search.rs:8349-8358` |
| S-10 | **MEDIUM** | Predictable temp files: installer EXE and Lens screenshot HTML | `settings_ui.rs:777-778`; `circle_to_search.rs:410-424` |
| S-11 | **MEDIUM** | Browser DBs copied to `%TEMP%` with predictable names; live-copy errors ignored | `browser_indexer.rs:6-14,245-258,324-335` |
| S-12 | **MEDIUM** | Git binary resolved from PATH (hijack) and repo content enters index/AI context | `git_indexer.rs:229-273` |
| S-13 | **MEDIUM** | 215+ raw `unsafe` Win32/COM blocks; raw `State*` in `GWLP_USERDATA`; cross-thread `Box::into_raw` | `main.rs:1595,2129,4245,4321`; counts §5 |
| S-14 | **MEDIUM** | Uninstaller/installer deletes `%APPDATA%\protonsearch` (legacy index) without v2 migration guard | `installer.iss:32-33`; `uninstall.rs:88-227` |
| S-15 | **LOW** | Logs retain paths/window titles/errors; only 1 MB truncation, no redaction | `applog.rs`; `indexer.rs:361-381`; `main.rs:744-781` |
| S-16 | **LOW** | Circle-to-search sends screenshot region to Google Lens with no explicit egress notice | `circle_to_search.rs:410-448` |
| S-17 | **LOW** | Code-signing absent; winget manifests stale (OmniSearch 1.0.0) | `packaging/winget/...`; README install instructions |
| S-18 | **INFO/PASS** | SQL injection review: no user input concatenated into SQL; dynamic SQL fragments are compile-time constants | `search.rs:2576,2611,2625`; parameterised queries throughout |
| S-19 | **INFO/PASS** | File-open/launch uses OS APIs, not a shell; `CREATE_NO_WINDOW` used on all background shell-outs | e.g. `launcher.rs:686`, `ai.rs:553-620` |
| S-20 | **INFO** | Minor: unused `unsafe` block, misplaced `#![windows_subsystem]`, dead code | `inspect_db.rs:25`; `uninstall.rs:1` |

Severity = impact × reachability on a normal user install. S-01 requires history access but the repo is public; S-02/S-03 require only network/local assumptions that hold today.

---

## 3. Detailed findings

### S-01 — CRITICAL: AI API key material is in git history (assume leaked)
**Evidence (masked only):**
- `git log --all -S 'sk-HrvSzHIY'` → commits `41b56c8` ("Fix audit issues and restore web search"), `1bfe506` ("…OMNISEARCH_KEYS injection…"), `11f0dfe` ("Seed ai_settings table with default key…"), `d55c3ad` ("Hardcode default API key…"), `86f116e`.
- Historical files contain a **67-character** `sk-Hr…` literal: `omnisearch/src/search.rs` at `41b56c8`/`1bfe506`/`11f0dfe`, `omnisearch/src/ai.rs`/`opensearch-os/src/ai.rs` at `1bfe506`/`d55c3ad`. Script: `.agents/notes/hist_key_scan2.py`.
- Current working tree: scan of all text files for `sk-…` ≥20 chars returned **0 hits** (masked scanner `.agents/notes/secret_scan.py`). Current `ai.rs` still contains an 11-char sentinel prefix at `:169` and `:260` and fake test keys at `:1341-1405` (e.g. `sk-oc-…len=20` fixtures).
- README links the public GitHub repo (`github.com/PranshulSoni/protonsearch`), so history was likely published. `.git` is present locally; forks/caches/archives may retain the blob.
**Impact:** anyone can extract the key from history and spend the operator's AI quota; if the key has other privileges (provider account), wider abuse. A public-key leak cannot be un-leaked by history rewrite.
**Remediation (must happen before any further release):**
1. **Revoke/rotate the provider key(s) now** (all keys ever embedded/committed; OpenAI/OpenCode/DeepSeek/etc.).
2. Remove the "seeded shared key" model entirely (see S-03/S-04): v2 is BYO-key.
3. Scrub history with `git filter-repo` **after** rotation (belt-and-braces; rotate regardless).
4. Add secret scanning to CI (`gitleaks`/`trufflehog` or `git grep` gate) and a pre-commit hook.
5. Add a `SECURITY.md` disclosure policy and a rotation runbook.
**Do not:** print the value, paste it into tickets, or test it against the provider.

### S-02 — CRITICAL: Updater executes unverified code (supply-chain RCE as the user)
**Evidence:**
- Check fetches `https://raw.githubusercontent.com/PranshulSoni/protonsearch/main/update.json` (`settings_ui.rs:677-681`), reads `version` and **`url`** (`:686-693`), downloads whatever `url` says to `%TEMP%\protonsearch_update.exe` (`:760-833`), then launches it `/SILENT /SUPPRESSMSGBOXES /NORESTART` detached (`:892-895`).
- `update.json` contains only `version`, `url`, `notes` — **no hash, signature, size, or pinned signer** (verified by reading the file + scanner for `sha|signature|checksum` → none).
- `scripts/install.ps1` downloads the latest release asset and runs it with no hash/signature check (lines 19-47). `scripts/update-sign-install.ps1` signs the downloaded file and verifies after signing, but performs no verification against a trusted expected signer, and passes the PFX password on the `signtool` command line (`/p`).
- No code-signing/user-account controls are enforced anywhere in the update path.
**Impact:** compromise of the GitHub account/branch, a malicious maintainer PR merged to `main`, a DNS/TLS avenue, or a repo-write insider yields silent arbitrary code execution in every updating user's session (full user rights). This is the highest-impact architectural flaw after key leak.
**Remediation (v2 requirement):**
1. Sign the installer/binary with Authenticode; in the updater verify **signer thumbprint/subject pin** before launching (`WinVerifyTrust`/`signtool verify /pa`), not just the hash.
2. Ship a **signed manifest** (minisign/Ed25519 key pinned in the binary) with `version`, artifact URL, size, SHA-256; reject missing/mismatched fields; reject non-HTTPS and unexpected hosts.
3. Download to a per-user random temp path created with restrictive ACL, compare size+hash, then run.
4. Prefer an official release API with immutable assets; do not fetch executable metadata from a mutable branch.
5. Document the release signing key and rotation; add a build-provenance step (reproducible build/SBOM) for G7.

### S-03 — HIGH: Local Hermes agent gateway: static default key + enabled PC-control tools
**Evidence:** `ai.rs:519-522` hardcodes `endpoint http://127.0.0.1:8642/v1/chat/completions` and a default API key `"hermes"` (overridable by env/DB). `ai.rs:566-573` configures the gateway with `platforms.api_server.extra.key = "hermes"`; `:580-592` enables `terminal`, `computer_use`, `code_execution`, `skills` for the api_server platform; `:609-620` starts `hermes gateway run --replace --accept-hooks`, logging to `%APPDATA%\protonsearch\hermes_gateway.log`. `main.rs:1138` starts it at launch when configured.
**Impact:** any local process (or user script, or a compromised low-privilege app in the same session) that can reach loopback can call the agent with a known default key and exercise enabled PC-control tools — a local pivot beyond the user's normal expectations for a search app. Combined with S-06, remote content can steer that agent.
**Remediation:** generate a random per-install token (DPAPI-protected), pass via environment/stdin (never argv/config defaults), bind loopback with peer checks/host allowlist, disable `terminal`/`computer_use` by default, and require explicit opt-in for `--accept-hooks`. In v2 the sidecar must follow the work-breakdown's "approvals" contract.

### S-04 — HIGH: AI credentials at rest in plaintext and on child-process argv
**Evidence:** settings UI saves `api_key` as a plaintext row into `ai_settings` (`settings_ui.rs:1136-1163`, INSERT at `:1145`); the typed command `ai config key …` also writes it (`main.rs:5973-5997`); `ai.rs:139-180` reads it. The launcher passes the key to the Hermes CLI as an argument: `.args(["config","set","model.api_key",&api_key])` (`main.rs:4925-4928`) — visible to same-user process monitors and possibly persisted in Hermes config. Env fallbacks `OPENCODE_API_KEY`/`DEEPSEEK_API_KEY` (`ai.rs:183-204`) and plaintext files `opencode_key.txt`/`ai_key.txt` (`ai.rs:206-233`).
**Impact:** any same-session process (unprivileged) can read the key from the DB or process command line, then use the user's AI quota or pivot. Baseline DB currently has **no `api_key` row** (only `endpoint` len 43 and `model` len 22), so this is latent on this machine but active on any configured install.
**Remediation:** store secrets with DPAPI (`CryptProtectData`) or Windows Credential Manager; never pass secrets on argv (use env/stdin with minimal lifetime); scrub `%APPDATA%\` key files; migration must move legacy keys into the OS store with explicit consent or leave them untouched, never log values.

### S-05 — HIGH: Arbitrary PowerShell from the search box, no confirmation
**Evidence:** `local_agent_action` matches prefixes `run command `, `execute command `, `run powershell `, `powershell `, `powershell: `, `cmd: ` (`main.rs:4479-4507`) and `run_local_agent_action` executes:
`Command::new("powershell").args(["-NoProfile","-ExecutionPolicy","Bypass","-Command", &command]).output()` (`main.rs:4515-4525`).
**Impact:** a user who pastes text into the launcher (e.g. clipboard content, a snippet, a copied AI answer) and presses Enter can execute arbitrary code with the user's rights. It is also a social-engineering footgun ("paste this to fix Windows"). The AI agent path has an approval model; this fast path bypasses it.
**Remediation:** remove the prefix fast-path, or require an explicit confirmation dialog with the full command rendered; consider an allowlist and never auto-run text that originated from clipboard. Keep `-ExecutionPolicy` irrelevant to security (it is not a boundary).

### S-06 — HIGH: Prompt injection + SSRF through "summarize <URL>" into a tool-enabled agent
**Evidence:** `ai::run` treats `summarize http(s)://…` by fetching the URL with `ureq` and stripping HTML (`ai.rs:1248-1266`), fetched text becomes the user prompt (`:1262-1265`), and the response is generated by the configured model. `fetch_url_text` (`ai.rs:1163-1172`) accepts any http(s) URL, including `http://127.0.0.1:*` / intranet hosts (SSRF). The agent system prompt explicitly instructs the model it can operate the PC and run shell commands (`main.rs:4857-4867`), and Hermes runs/approval flows execute tool calls (`ai.rs:816-1070`, `main.rs:5240-5290`); `ALWAYS_APPROVE` can bypass approvals (`ai.rs:10,511-516,891`; `main.rs:5270-5285`, set at `main.rs:2354,3338`).
**Impact:** a fetched page can contain instructions ("ignore previous… run …") that steer the agent into actions while the user believes they asked for a summary; SSRF can hit local services/metadata endpoints. Severity depends on whether approvals are enabled; the default/always-approve setting makes it serious.
**Remediation:** treat fetched content strictly as data with a fixed system message; disable tools for summarize/fetch tasks; block private/link-local/loopback IPs and non-standard ports (SSRF allowlist); show URL egress consent; never set always-approve automatically; record agent tool calls in an audit log.

### S-07 — HIGH: Clipboard capture persists sensitive data by default, and imports Windows clipboard history
**Evidence:** `AddClipboardFormatListener` at startup (`main.rs:1291`); `WM_CLIPBOARDUPDATE` stores every non-empty text clip (up to 500, upsert by content) with source app name (`main.rs:1937-1999`) and images as BMP files under `%APPDATA%\protonsearch\clipboard_images` with OCR text (`main.rs:11370-11450`); on startup it **imports the entire Windows clipboard history** (text + bitmap, up to 50 MB per image) into the DB (`main.rs:11704-11826`), invoked unconditionally in a background thread (`main.rs:1353-1365`). The DB and image files are plain, unencrypted, user-readable; the search UI and OCR FTS can surface them.
**Impact:** passwords copied from password managers, 2FA codes, private messages, and screenshots become durable local data (and can enter AI context if the user asks AI about a clip). No opt-in prompt/consent, no source-app exclusion, no TTL for unpinned items beyond a 500-row cap.
**Remediation:** default-off clipboard capture, explicit opt-in with explanation; exclude password-manager/known-sensitive source apps; TTL + size caps; DPAPI encrypt sensitive rows or store clips in an encrypted vault; delete imports on user request; never send clipboard content to AI without an explicit action.

### S-08 — MEDIUM: Focus session force-kills processes/windows every 3 s
**Evidence:** `start_focus_session:<category>` spawns a loop that reads `focus_categories.blocked_apps` and, every 3 s, runs `taskkill /F /IM <item>` for `.exe` entries, otherwise `taskkill /F /FI "WINDOWTITLE eq *<item>*" /IM "*"` (`launcher.rs:335-380`). Catalog exposes this as "Start Focus Session" (`search.rs:10547-10589`).
**Impact:** a typo/wildcard entry can kill unrelated windows/processes; the wildcard filter can match arbitrary titles; runs until the static flag flips. Data loss (unsaved work) is plausible.
**Remediation:** restrict to an explicit allowlist of executables with signed-in-app confirmation; kill once with a preview of affected windows; block system processes / the launcher itself; log every kill; feature-gate in v2 with clear UX.

### S-09 — MEDIUM: Wi-Fi password action (broken, and dangerous if fixed naively)
**Evidence:** settings-catalog entry "Show Wi-Fi Password" with command `action:wifi_password` (`search.rs:8349-8358`); handler spawns cmd that runs `netsh wlan show profiles` and then asks to run `netsh wlan show profile name="PROFILE" key=clear` — the literal `PROFILE` placeholder means it does **not** show the selected profile (`launcher.rs:643-651`).
**Impact:** current behaviour is broken/user-confusing; any fix that runs `key=clear` makes saved Wi-Fi credentials visible on screen, shoulder-surfable, and searchable in history. This is a feature that should be cut or require explicit confirmation with hidden output.
**Remediation:** remove from catalog; if retained, use a secure dialog, no shell echo, require re-auth; document privacy impact.

### S-10 — MEDIUM: Predictable temp files
**Evidence:** installer downloaded to fixed `%TEMP%\protonsearch_update.exe` (`settings_ui.rs:777-778`); Lens screenshot base64 written to `%TEMP%\protonsearch_lens_<pid>.html` (`circle_to_search.rs:411-423`) and deleted after 60 s by a detached thread. `save_clipboard_image` writes `%APPDATA%\protonsearch\clipboard_images\image_<ts>.bmp` (`main.rs:11360-11450`).
**Impact:** local race/pre-creation attacks (write a malicious EXE at the fixed path ahead of download → user-approved installer runs attacker binary; temp HTML can be read by other local users, contains screenshot data); crash leaves sensitive files.
**Remediation:** `CREATE_NEW` + random name under a private per-user temp dir with restrictive ACL; verify hash before execute; delete on all paths (RAII); avoid writing screenshots to disk (post directly).

### S-11 — MEDIUM: Browser DB copies in %TEMP%; live-copy errors ignored
**Evidence:** Chromium `History` copied with `std::fs::copy` to `%TEMP%\browser_history_temp_<ms>.db` (`browser_indexer.rs:245-258`), Firefox `places.sqlite` to `%TEMP%\firefox_places_temp_<ms>.db` (`:324-335`); `TempFile` Drop removes them (`:6-14`). Copy of a live locked SQLite DB can be inconsistent; parse errors are printed/ignored (`:52-58, 65-84`).
**Impact:** full browsing history duplicated to world-readable temp during each 10-min cycle; if the process crashes/killed, copies remain. Inconsistent copies can silently drop data.
**Remediation:** open source DB read-only with immutable/backup API where possible; copy with `CREATE_NEW` random name + ACL; verify SQLite header; delete in a scope guard; document as sensitive.

### S-12 — MEDIUM: Git from PATH; repo content becomes index/AI input
**Evidence:** `get_git_executable` checks PATH first, then common install dirs, then bare `git` (`git_indexer.rs:229-273`); commands use `current_dir(repo)` with static args (`:280-322`) — no shell interpolation. Repo paths come from scanning user folders; commit messages/TODOs are inserted into the searchable index and can be surfaced to the AI.
**Impact:** PATH hijack (malicious `git.exe` in a user-writable PATH dir) runs code as the user during indexing; repo content can be malicious text (prompt injection when fed to AI, terminal escape sequences when printed). Severity limited by same-user execution context.
**Remediation:** resolve to a known signed Git installation or use libgit2/gix; validate repo paths against configured roots; sanitise repo text before display/AI; never feed raw file content to the agent without labelling as untrusted.

### S-13 — MEDIUM: Unsafe Win32/COM lifetime surface untested
**Evidence:** 215 `unsafe` token occurrences in `main.rs`, 30 in `launcher.rs`, 19 in `settings_ui.rs`, 16 in `search.rs`, 11 in `indexer.rs`, 10 in `circle_to_search.rs` (counts = token occurrences, upper bound). Raw `State` pointer in `GWLP_USERDATA` (`main.rs:1595`), `Box::into_raw` across threads (`main.rs:1389-1421`) and `Box::from_raw` in message handlers (`main.rs:2129`), `EnumWindows` callbacks dereferencing a borrowed `Vec` pointer (`search.rs:10167-10217`; `launcher.rs:240-315`), manual `CoTaskMemFree` (`launcher.rs:693-706`), GDI object lifetime managed by hand (`main.rs:3698, 7465-7520`). The code itself documents re-entrancy hazards around `SetWindowPos` (`main.rs:4245, 4321`).
**Impact:** potential UB/crash (UAF) under window destroy/re-entrancy or exotic message ordering; a crash in a search/launcher process can leave DB WAL/indexer state dirty (observed stale `is_indexing=1`). No sanitizer/CI coverage on Windows.
**Remediation:** v2 confines Win32 to `ps-windows` with safe wrappers; use `Arc<Mutex<UiState>>`/message-passing instead of raw pointers; add `#![deny(unsafe_op_in_unsafe_fn)]`, `unsafe` audit comments with safety invariants; drop/clippy gates; consider `cargo miri` for pure-Rust crates only.

### S-14 — MEDIUM: Legacy uninstaller deletes the user data directory (migration hazard)
**Evidence:** `installer.iss:32-33` `[UninstallDelete] Type: filesandordirs; Name: "{userappdata}\protonsearch"`; `uninstall.rs:170-179` deletes `%APPDATA%\protonsearch` and the legacy `omnisearch` dir after a MessageBox prompt (`:92-102`) in the standalone path.
**Impact:** installing/uninstalling v1 after v2 migration, or a v2 installer invoking the legacy uninstaller, could destroy the index/history/AI config. It is also a plain data-loss footgun today (user data is not backed up).
**Remediation:** v2 must never invoke the legacy uninstaller; migration is copy/read-only + explicit rollback export; v2 uninstaller must prompt separately for user data and default to **keep** it; remove legacy cleanup from any v2 install path.

### S-15 — LOW: Logs are path/title rich and unredacted
**Evidence:** `applog.rs:1-40` appends to exe dir then AppData, 1 MB truncation; `indexer.rs:361-381` logs paths/scan details; `git_indexer.rs:51-67`; `main.rs:744-781` panic backtraces. `settings_ui.rs:1019+ log_settings_ui`.
**Impact:** logs reveal filenames/folders and failure context; no user-visible redaction; may be shared in bug reports.
**Remediation:** redact/opt-in verbose logs; rotate with cap; document log contents; never log query text or clipboard content by default.

### S-16 — LOW: Google Lens egress without consent framing
**Evidence:** region screenshot → base64 → temp HTML auto-submits multipart POST to `https://lens.google.com/v3/upload` (`circle_to_search.rs:410-448`); no dialog saying the image leaves the machine. Code comment/README describe "circle to search" but not data egress.
**Impact:** screenshots (potentially sensitive) sent to a third party on user action; user may not realise the destination.
**Remediation:** explicit consent per use or a clear one-time disclosure; offer local OCR/visual search alternative; consider cutting in v2.

### S-17 — LOW: Release provenance / signing gaps
**Evidence:** no signing step in repo build; `install.ps1` runs an unsigned installer; README instructs `curl | powershell` (line 70) — a remote-code-execution install pattern; winget manifests are stale OmniSearch 1.0.0 (`packaging/winget/...`).
**Impact:** users are trained to bypass SmartScreen and pipe remote scripts into PowerShell; provenance cannot be verified.
**Remediation:** sign all release binaries; publish SHA256SUMS + signed checksums; replace `curl | powershell` guidance with a downloaded, verified installer; update winget manifests; document checks.

### S-18 — INFO / PASS: SQL injection review
Only 6 `format!`-built SQL strings exist in `search.rs`; the dynamic parts are compile-time constants: `const IMG_FILTER: &str = "f.extension IN ('png','jpg','jpeg','bmp','gif','webp')"` (`search.rs:2576`) used at `:2611` and `:2625`; remaining hits are message strings. All user-influenced values are bound parameters (e.g. `LIKE ?`, `MATCH ?`, `params!`). No SQL injection found in legacy code. Keep this invariant in v2 (query builder + parameters; forbid `format!` SQL via review/lint).

### S-19 — INFO / PASS: Shell injection review
No user-controlled filename is interpolated into a shell command string in the audited paths. File/folder launches use `ShellExecuteW` with quoted/handled parameters (`launcher.rs:91-215`), git uses `Command` args with `current_dir` (no shell), and all background spawns set `CREATE_NO_WINDOW` (e.g. `git_indexer.rs:283`, `ai.rs:555`, `indexer.rs:1060`). Exception: the intentional PowerShell fast path S-05 and static strings in `launcher.rs` (e.g. Wi-Fi command `:648` uses a literal, no interpolation).

### S-20 — INFO: Hygiene
- `src/uninstall.rs:1` `#![windows_subsystem = "windows"]` is ignored for non-root modules (warning).
- `src/inspect_db.rs:25` unnecessary nested `unsafe` (warning).
- Dead vector-search paths and unused enum variant (see baseline §2.3) — remove in v2, no security impact.
- No CI secret scanning or dependency audit today; v2 CI should add `cargo audit`/`cargo deny` once the workspace is stable.

---

## 4. Positive controls verified (do not regress)

1. SQL is parameterised; dynamic SQL uses compile-time constants (`search.rs:2576,2611,2625`).
2. Shell-outs use `CREATE_NO_WINDOW` consistently; file opening uses OS shell APIs, not `cmd /c start` with user input (`launcher.rs`, `git_indexer.rs:283`).
3. PDF/DOCX parsing is subprocess-isolated with a 30 s watchdog and drained stdout (`indexer.rs:1052-1104`) — limits blast radius of malformed documents.
4. Browser DB access copies to temp with RAII cleanup (`browser_indexer.rs:6-14`) — preserves live browser DBs and avoids locks.
5. SQLite uses WAL + `busy_timeout(5s)`; settings UI uses a 300 ms timeout for polling (`search.rs:725-727`; `settings_ui.rs:920-923`).
6. Single-instance enforcement with a session-local mutex (`main.rs:637-665`).
7. Hotkey validation rejects modifier-less/broken combos and checks availability (`hotkey.rs:92-113, 67-90`).
8. No telemetry/account in legacy code; network calls are AI endpoints, Lens (user action), GitHub update check, and favicon/image URLs.

---

## 5. Unsafe inventory (counts used for S-13)

Per-file occurrences of token `unsafe` (upper bound; includes comments): main.rs 215, launcher.rs 30, settings_ui.rs 19, search.rs 16, indexer.rs 11, circle_to_search.rs 10, hotkey.rs 2, inspect_db.rs 2, git_indexer.rs 1, uninstall.rs 1. `Command::new` calls: launcher.rs 41, main.rs 14, ai.rs 8, uninstall.rs 6, git_indexer.rs 2, indexer.rs 1, search.rs 1, settings_ui.rs 1. `thread::spawn`: main.rs 32, settings_ui.rs 8, indexer.rs 4, launcher.rs 4, circle_to_search.rs 2, browser_indexer.rs 1, git_indexer.rs 1.

---

## 6. Release / migration requirements derived from this register

**Must-fix before any v2 public build (G1/G7 gates):**
1. Rotate all provider keys and remove embedded-key machinery (S-01/S-03/S-04).
2. Replace the updater with signed-manifest + hash + Authenticode verification (S-02).
3. Remove/gate arbitrary PowerShell execution and disable always-approve auto-enable (S-05/S-06).
4. Secret storage via DPAPI/Credential Manager; no secrets on argv or in SQLite plaintext (S-04).
5. Clipboard capture opt-in **off by default**, source-app exclusions, encryption/TTL (S-07).
6. Migration must open the legacy DB **read-only**, never invoke the legacy uninstaller, and export a rollback copy (S-14).
7. CI gates: secret scan, `cargo deny`/`audit`, clippy `-D warnings`, fmt, tests (S-01, S-20).
8. Publish a threat model/security model doc for v2 and a disclosure process.

**Should-fix before RC:** S-08, S-09, S-10, S-11, S-12, S-13 (ps-windows confinement), S-15, S-16, S-17.

**Verified non-issues to preserve:** S-18, S-19, and the positive controls in §4.

---

## 7. Audit commands (reproducible, no secrets printed)

```powershell
python .agents/notes/secret_scan.py        # masked scan of working tree -> TOTAL_HITS=0
python .agents/notes/hist_key_scan2.py     # masked scan of historical commits
python .agents/notes/forensic_dump.py      # DB tables/counts; ai_settings keys/len only
git log --all --oneline -S 'OMNISEARCH_KEYS'
git log --all --oneline -S 'sk-HrvSzHIY'   # sentinel only; do not paste values
```

**Reporting rule for this repo:** when a suspected secret is found, record the file:line, a masked prefix, and length; never the value; rotate immediately and log the rotation in the release checklist.
