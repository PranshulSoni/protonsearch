# Storage Health

Generated with npm run widget:create -- storage-health.

Native binding: provider_snapshot('storage'), permission storage:read, read-only system-drive totals. No filesystem paths or arbitrary commands accepted. Shared scheduler polls every 30 seconds while visible, coalesces concurrent reads, retains stale snapshots and cleans subscriptions on unmount.

Before adapting to another source read docs/widgets/CREATION_PLAYBOOK.md, CONTRACT.md, QA_CHECKLIST.md and docs/design-reference.md. Update fixtures, provider ID, permissions and this README together. Preview values are isolated fixtures.
# Size variants

Compact uses 140px for free space. Standard uses 180px and adds drive/total space.
Wide uses 260px and adds the used percentage calculated from the same native snapshot.
No new permissions or refreshes are required. Existing custom widths remain saved.

