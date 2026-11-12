# Phase 2 service status

The Phase 2 acceptance gate remains open.

## To-do

Both existing To-do views use the same SQLite commands through `useTodos`.

| Command | Input | Result |
| --- | --- | --- |
| `todos_list` | None | Task array, pinned first, then position and stable ID |
| `todos_save` | `item` | Success or validation/storage error |
| `todos_delete` | `id` | Idempotent deletion |
| `todos_import` | `items`, `source` | Transactional, once-per-source migration |

Tasks include identity, title, notes, completion state and timestamps, due date,
timezone, priority, pin/order and an optional association. Existing browser data
is retained after migration. Failed imports roll back. Repeated imports preserve
edits and deletions. New installations start empty. Both views show storage errors.
Desktop mutations emit `todos:changed` to refresh other windows.

## Pi

The active desktop discovery and launch paths use Pi. Legacy task/configuration
data remains on disk. Startup work executes on a background worker. Concurrent
launches are rejected before a task is created. Each run receives its own event
callback, preventing later runs from updating the first run's task record.
Launch failures become failed tasks. Cancellation checks the requested run ID.
Unsupported pause/resume explanatory controls are omitted.

The current adapter remains read-only and uses Pi's JSON process mode. Pi installation review/confirmation/cancellation and model connection setup are exposed in Workflows. Full RPC sessions, per-tool approval enforcement and task rename/archive operations remain pending.
Pi's official RPC interface is documented at https://pi.dev/docs/latest/rpc.

## Checks performed

- Desktop TypeScript check passed.
- Frontend suite passed, 274 tests.
- Pi library suite passed, 75 tests.
- Pi process integration suite passed, 16 tests using the existing offline helper.
- SQLite task restart/update/delete/import regression passed.
- Installed Pi reported version 1.5.0.
- Release application built successfully.

Real model execution remains unverified. The local Pi authentication store had zero
configured providers during this check. Native-control hardware verification,
cold-start traces and simultaneous Pi/indexing/search resource measurements also
remain pending. UI automation reached the island but task-card interaction failed
with window-usability errors, so this run provides no live To-do acceptance claim.
