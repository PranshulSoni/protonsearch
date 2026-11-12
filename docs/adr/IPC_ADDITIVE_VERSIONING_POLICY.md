# Policy: additive IPC/settings versioning (v10)

- Status: **Accepted** (L0 decision, 2026-11-10)
- Owner: L0; maintained by L1-A/L1-E. Amends ADR-0006 for the v10 additive changes.
- Applies to: ps-ipc DTOs, Tauri command surface, Tauri event payloads, persisted settings JSON,
  bridge/types.ts mirrors and the agent task store.

## Context

ADR-0006 states that adding a capability means a deliberate contract change with a protocol version
bump and mirrored TS types. v10 added launcher, island, clipboard, monitor and preview DTOs/fields,
new Tauri commands and settings fields. L0 decided NOT to bump PROTOCOL_VERSION or
DATA_SCHEMA_VERSION, because the additions are backward-compatible when old settings/DTOs are still
accepted. This policy documents exactly what "additive" means so the decision stays auditable and
does not become permission for silent breaking changes.

## Decision (rules)

1. **Allowed without a version bump:**
   - New optional fields on existing DTOs/settings with Rust serde(default) and optional/clamped TS
     fields; new optional settings groups (e.g. Settings.launcher, Settings.clipboard) that default
     when absent.
   - New Tauri commands/events; new enum variants only when every consumer is updated in the same
     release or tolerates unknown values (serde aliases/defaults); additive PreviewPayload status/
     truncated/error/imageMime fields.
   - New fields in ResultMeta for specific row kinds (e.g. clipboard image thumbnail/dimensions/mime)
     provided absent means "no data" rather than an error.
2. **Requires PROTOCOL_VERSION bump + mirrored TS update + migration/consumer notes:**
   - Removing/renaming a field, changing a field type/units/semantics, making an optional field
     required, changing command names/arguments/return semantics, changing event ordering guarantees.
3. **Requires DATA_SCHEMA_VERSION bump + migration test + backup/rollback:**
   - Breaking changes to persisted settings JSON or SQLite/agent-task store shape (renames, removals,
     reinterpretation, key path changes). Additive optional fields with safe defaults do not require a
     bump; Settings.schema_version and task-store schema_version remain 1 for v10.
4. **Forward/backward compatibility:** older settings files must parse with defaults and defaults must
   be safe (privacy-preserving). Downgrades are not supported; an older build may drop unknown new
   fields when saving, so a backup is required before downgrade. Unknown JSON fields are ignored on
   read; they are not preserved on write.
5. **Required tests (CI/PR blocking):**
   - Legacy-settings-parse test: an old v1 JSON without v10 fields parses to typed defaults. Status:
     PRESENT at crates/ps-ipc/src/lib.rs:1273-1297 (legacy_settings_without_v10_fields_parse_with_
     defaults) and must stay green.
   - Additive-field contract test: serialize/deserialize with and without new optional fields.
   - TS/Rust schema check: bridge/types.ts must be checked against ps-ipc (generated or asserted);
     until task-44 moves agent task DTOs, the documented mirror must be kept in sync.
   - Persisted settings round-trip + no-invalid-persist tests for validation rules (ADR-0007).
6. **Review:** additive changes are reviewed by L1-E (contract owner for release) and accepted by L0;
   any change that would break an old client, a persisted file or a released UI must be proposed as a
   breaking change with a version bump, not as an "additive" edit.

## Consequences

- v10 ships without unnecessary protocol/schema churn; additive fields remain cheap.
- The audit trail is this policy plus ADR-0006 and the tests above; reviewers must classify each
  contract change as additive or breaking.
- Risk: TS mirror drift. Mitigation: task-44 moves agent task DTOs into ps-ipc; bridge/types.ts
  remains a documented mirror and must be checked at every contract change.
- Risk: downgrade data loss for unknown fields. Mitigation: documented no-downgrade policy + backup.

## Verification

- Current constants: PROTOCOL_VERSION = 1, DATA_SCHEMA_VERSION = 1 (crates/ps-ipc/src/lib.rs:17,20).
- Legacy-settings-parse test: crates/ps-ipc/src/lib.rs:1273-1297.
- Additive examples: Settings.launcher/clipboard serde(default) (ps-ipc:936-960), PreviewPayload
  status/truncated/error/imageMime (ps-ipc:903-934), ResultMeta image fields (ps-ipc:252-281).
- Task-44: agent task DTOs moved to ps-ipc; bridge/types.ts updated as documented mirror.
