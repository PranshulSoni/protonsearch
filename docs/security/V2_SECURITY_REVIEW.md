# v2 Security Review — migration, engine, Tauri shell, ps-windows, ps-content

Reviewer: L1-D (rel-recon) / Date: 2026-11-09
Scope: v2 code added after Gate 0 (de0ef19 -> b4df328 + working tree at review time):
crates/ps-core/{engine,migration,db,settings,paths}.rs, apps/desktop/src-tauri/src/main.rs
+ capabilities/default.json + tauri.conf.json, crates/ps-windows/**, crates/ps-content/
{legacy,index,extract}.rs, and migration temp-file handling.
Method: static review + targeted greps + a real-DB READ-ONLY migration dry run via
tests/migration/harness (calls the actual ps_core::migration::migrate_legacy_db).
Limitations: no pen-test, no GUI run, no fuzzing. crates/ps-sources was mid-landing during
review (apps/browser/git/recent still stubs at the time); findings there are scoped to code
actually present (clipboard, actions). Re-review ps-sources when sources-content finishes.

## Verdict

No CRITICAL findings in the new v2 code. The shell command surface is small, structured and
CSP/capability-constrained; there is no shell string interpolation, no network plugin, and
URLs are scheme-validated. The material risks are contract/privacy/reliability issues:
migration can silently mark itself done after failure, its "source unchanged" guarantee only
covers the main .db file (not WAL/SHM), import counts/flags are misleading, destructive native
actions rely on UI-only confirmation, and clipboard capture defaults to ON with no source-app
exclusions once wired. Fix before G4/G5 acceptance.

## Findings (ranked)

### V2-01 HIGH (reliability, silent data loss) — failed migration is marked permanently done
Engine::migrate_legacy sets legacy_migrated = true and saves settings EVEN WHEN
migrate_legacy_db returned Err (warnings only): crates/ps-core/src/engine.rs:852-856. First-run
migration runs once in the background (engine.rs:869-873, 864-878); a transient failure (locked
DB, WAL access denied, disk full) leaves the user with an empty v2 index and no retry.
report.source_unchanged == false also only pushes a warning (:835-840).
Fix: only persist legacy_migrated on success; persist a failure state (or leave false) and
retry on next launch with a surfaced error; treat source_unchanged == false as a hard stop.

### V2-02 MEDIUM (contract correctness) — "source unchanged" only hashes the main DB file
migrate_legacy_db hashes the .db file only (migration.rs:61-67, 152, 227-233) and opens it with
mode=ro (:51-58) - not immutable=1. A WAL-mode database can create/modify -wal/-shm even for
read-only opens (SQLite needs the SHM as shared scratch). The real baseline has a 23.8 MB
un-checkpointed WAL. The contract therefore does NOT prove the legacy directory is untouched;
it proves the main DB file is byte-identical. The probe now records WAL/SHM hashes + new files
in the directory (see evidence).
Fix: use immutable=1 when the legacy DB is quiescent (or copy DB+WAL+SHM to a migration cache
and open the copy); hash/snapshot the whole legacy dir and surface WAL/SHM changes in the
report; document that the true invariant is "no logical writes to legacy data".

### V2-03 MEDIUM (correctness/consent) — import report counts source rows, not imported rows
copy() returns per-table inserted counts but the map is discarded (migration.rs:197-224,
"let _ = copied"); report fields are source COUNT(*) values. INSERT OR IGNORE can silently drop
conflicting rows without a warning. import_ai_keys = true has no code path (the flag only
controls adding ai_settings to skipped_tables), and import_clipboard = true merely pushes a
warning that ps-sources owns it (:215-220). A caller can therefore believe keys/clipboard were
imported when nothing happened.
Fix: report imported/skipped per table; fail or warn when source > 0 and imported = 0; remove
or implement the flags (today: always skip, and say so in the API docs/UI).

### V2-04 MEDIUM (authorisation boundary) — destructive native actions are UI-confirmed only
open_result accepts any ActionSpec from the WebView (main.rs:263-272) and
Engine::execute_action forwards destructive RunNative commands straight to
ps_sources::actions::run_native (engine.rs:644-664; ps-sources/src/actions.rs:79-100):
shutdown, restart, sleep, empty recycle bin. The comment "confirmed by the UI/engine" is only
true for the UI path; there is no engine-side confirmation token/state and no restriction to
actions that the engine itself produced. Any script executing the Tauri invoke for open_result
with a RunNative Shutdown payload (e.g. a future XSS, the CDP escape hatch below, or a plugin)
powers off the machine in one call.
Fix: require an engine-issued nonce/confirmation state for destructive commands; validate
OpenPath/LaunchApp against engine-emitted results (or restrict paths to configured roots and
known app targets); keep confirmation UX, but enforce server-side.

### V2-05 MEDIUM (dev-only hardening) — CDP remote debugging enabled by env var
PROTONSEARCH_CDP_PORT is checked in the Tauri shell and, if set to digits, adds
--remote-debugging-port=<port> to WebView2 (main.rs:159-167, 178, 190-195, 223-228). That
exposes the UI invoke surface (including V2-04) to any local process speaking CDP. It is
intended for the E2E harness and is loopback-only, but an env var is a low bar for a shipping
binary.
Fix: compile it only under cfg(debug_assertions) or a cdp cargo feature; never in release
builds; document in KNOWN_LIMITATIONS.

### V2-06 MEDIUM today -> HIGH when wired (privacy) — clipboard capture default ON, no exclusions/TTL
privacy.clipboard_history defaults to TRUE (crates/ps-ipc/src/lib.rs:502-515), and
ps-sources/src/clipboard.rs:36-50 captures every clipboard update through the ps-windows
listener, storing text (up to 1 MB) and images (PNG up to 32 MB, plaintext on disk) with no
source-app exclusion, no password-manager filter, no retention/pruning, no encryption. It also
"imports Windows clipboard history" by reading arbitrary files under
%LOCALAPPDATA%\Microsoft\Windows\Clipboard as UTF-8 (:229-270) - content and provenance are
questionable and should be explicit opt-in. No caller of start_listener was found in the engine
at review time, so the risk is currently latent; wiring it with the current default would
repeat legacy S-07.
Fix: default FALSE; opt-in UI with plain-language consequence; exclude password managers/source
apps; TTL/size caps; DPAPI-encrypt rows or document plaintext risk; gate the Windows-history
import separately and never read it automatically.

### V2-07 LOW-MEDIUM (robustness) — clipboard DIB decode has no pixel/size cap
decode_dib validates header/compression and computes required with checked arithmetic
(ps-windows/src/imp/clipboard.rs:132-160), then allocates width*height*4 (:162). It is bounded
by the actual clipboard payload (required <= size) up to ~1.33x, but a local process can put a
very large bitmap on the clipboard and force a large allocation/CPU on the listener thread.
Fix: cap pixels (e.g. 64 MP) and total bytes (e.g. 256 MB) before decode; skip with a log.

### V2-08 LOW — launch_app accepts arbitrary shell/target strings from IPC
ps-windows/src/imp/shell.rs:149-169 passes any shell:* app id/target, any existing file target,
or shell:AppsFolder\<app_id> to ShellExecuteW. This is intended app launching, but combined with
V2-04's open IPC a compromised WebView can open arbitrary executables/shell namespaces.
Mitigate with V2-04 plus allowing only targets present in the app index.

### V2-09 LOW (correctness) — migration read-only URI does not percent-encode
ps-core/src/migration.rs:51-58 builds file:<path>?mode=ro after only replacing \ with /; legacy
paths containing #, ?, or % break URI parsing. ps-content/src/legacy.rs:263-290 and
ps-sources/src/clipboard.rs:320-342 do this correctly (percent-encode # ? %, handle UNC). Reuse
that helper. Low because the path is fixed AppData, but it is a real divergence.

### V2-10 LOW (privacy transparency) — activity log records queries/actions by default
privacy.activity_history defaults true (ps-ipc:511-514); search queries are logged
(engine.rs:413-421) and successful actions with path/message (:667-674), bounded to 20k rows
(db.rs:109-114). Local-only and expected for an Activity feature, but it is unencrypted
user-profile data; keep the default, add a clear opt-out/clear action and document retention.

### V2-11 LOW (test hygiene) — predictable temp dirs in migration unit tests
ps-core/src/migration.rs:271-274, 293-294 uses std::env::temp_dir()/ps-migration-<pid> and
remove_dir_all before/after. Test-only and same-user, but predictable names + recursive delete
are a local race; prefer tempfile-style unique dirs or create with create_new.

### V2-12 INFO — dev-only tooling must not ship
ps-bench (workspace member) invokes certutil/child processes (crates/ps-bench/src/lib.rs:79,
childproc.rs) and writes bench artifacts; it is a dev CLI. ps-agent exists with
Command::new(&config.python) (ps-agent/src/client.rs:288) and temp venv dirs (lib.rs:75-105) but
is opt-in/stubbed. G7 packaging must exclude both (sidecar only with explicit consent).

## Controls verified (good; keep)

1. Tauri surface: capability default allows only core:default + global-shortcut:default
   (capabilities/default.json); no shell/http/fs plugins. CSP "default-src 'self';
   script-src 'self'" with no remote origins (tauri.conf.json app.security.csp).
2. No shell interpolation: all process launches use Command::new(program).arg(...) or
   ShellExecuteW(verb, file, params) with structured arguments; no cmd /c <user text> in v2
   (grep for Command::new / format! confirms only fixed programs and fixed args).
3. URL validation: open_url accepts only http(s), rejects control/whitespace chars and requires
   a host before ShellExecuteW (ps-windows/src/imp/shell.rs:125-146).
4. SQL: queries are parameterised; FTS input is tokenised and each term quoted, max 16 terms /
   128 chars (ps-content/src/index.rs:421-458); dynamic table names are constants.
5. Filesystem indexing: never follows symlinks/junctions (ps-index/src/scan.rs:220-221, 381
   follow_links(false)), pruning/exclusions and max_entries bounds present.
6. Untrusted documents: extraction has size caps (64 MB input, 512 KB default text), 10 s
   default timeout on a worker thread, catch_unwind, zip-bomb caps (8 MB/entry, 64 MB/archive,
   4096 entries) (ps-content/src/extract.rs:18-170).
7. Persistence: filename index save is temp+rename with checksum (ps-index/src/persist.rs:148-241);
   settings save temp+rename (ps-core/src/settings.rs:13-21); engine DB uses WAL + busy_timeout
   + short transactions (ps-core/src/db.rs:70-83).
8. Clipboard listener: single global listener, bounded retries opening the clipboard, explicit
   stop/join (ps-windows/src/imp/clipboard.rs:35-44, 242-274).
9. No XSS sinks found in the UI: grep for dangerouslySetInnerHTML, innerHTML, eval(,
   new Function, javascript: in apps/desktop/src returns only a test DOM reset
   (launcherSurface.test.tsx:54). React escapes content by default.

## Migration dry-run evidence (real DB, READ-ONLY)

REAL_DRY_RUN_RESULT_PLACEHOLDER

## Recommended actions before G4/G5

1. V2-01 migration retry/state (must-fix; data loss).
2. V2-04 destructive-action authorisation (must-fix before any public build).
3. V2-06 clipboard default + exclusions (must-fix before wiring capture).
4. V2-02/V2-03 migration contract hardening + imported counts (should-fix; G4).
5. V2-05 CDP debug-gate (should-fix; release build).
6. V2-07/V2-08/V2-09/V2-10/V2-11 hardening (nice-to-have; can follow RC).
