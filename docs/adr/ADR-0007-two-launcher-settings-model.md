# ADR-0007: Two-launcher settings model (Normal Launcher + Dynamic Island)

- Status: **Accepted** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L1-A (schema/engine), L1-B (UI), L1-E (review)
- Supersedes/extends: ADR-0006 (IPC contract) for the v10 launcher settings surface

## Context

The v10 product has one Rust core and two launcher presentations (centered Normal Launcher and the V9
Dynamic Island). Both must share one index, one search contract and one settings store, and users must
be able to switch the primary launcher without re-indexing. The G1 contract review found the typed
model existed but there was no validation, no decision on settings migration and an ambiguous
clipboard privacy model.

## Decision

1. The single source of truth is ps_ipc::Settings.launcher (LauncherSettings { primaryMode, island })
   plus Settings.clipboard. Default primary mode is Normal; Dynamic Island is an explicit opt-in.
2. IslandSettings contains exactly the eight V9 cards in card order, with enabled flags, a default
   card, dots, focus-loss behavior and placement. Validation rules are enforced server-side in the
   Tauri shell before persisting (validate_settings): exactly 8 cards, no duplicate ids, the default
   card must be present and enabled, offsets are within +/-2000, clipboard retention is 0-3650 days
   and max items 10-100000 (main.rs:605-635, update_settings 637-659).
3. Switching primary launcher or changing placement takes effect live, without restart or index
   rebuild; the global shortcut toggles the active primary launcher (main.rs:257-266, 655-679).
4. Clipboard has one master gate: history is captured only when privacy.clipboard_history AND
   !clipboard.pause; capture_text and capture_images are format toggles that apply only while the
   master gate is open (ps-ipc:809-853).
5. Settings remain additive-serde compatible (no DATA_SCHEMA_VERSION bump for v10, see
   docs/adr/IPC_ADDITIVE_VERSIONING_POLICY.md). The legacy-settings parse test
   (crates/ps-ipc/src/lib.rs:1273-1297) proves an old v1 settings.json without launcher/clipboard
   fields loads with defaults, including Normal primary mode and 8 default cards.
6. Future breaking changes to persisted settings require a schema-version bump plus a migration test
   and backup policy; additive optional fields do not.

## Consequences

- UI and shell share one validated model; invalid card orders/offsets cannot be persisted through the
  settings command.
- Older app builds that save settings after a newer build may drop unknown v10 fields (serde default
  behavior). Downgrades are not supported without a backup restore; documented in the policy.
- The engine applies patches after shell validation; engine-side validation is defense in depth and
  should be added for non-shell callers.
- Two launchers do not fork search behavior or data; parity is a G8 test obligation.

## Alternatives considered

- Separate settings files/stores per launcher: rejected (drift, migration and precedence complexity).
- Untyped JSON settings: rejected (no validation, no migration story).
- Version bump now: rejected by L0 decision; additive serde fields with a legacy-parse test are
  sufficient for v10 and avoid unnecessary migration churn.

## Verification

- crates/ps-ipc/src/lib.rs: legacy settings parse test (1273-1297), settings defaults (936-960+).
- apps/desktop/src-tauri/src/main.rs: validate_settings (605-635), update_settings (637-659),
  island hot-apply (655-679), shortcut mode toggle (257-266).
- G2/G6: settings UI tests, restart/migration test, no-invalid-persist tests.
