# Existing-file integrity report

The repository was already dirty before this migration. The pre-work status and SHA-256 manifest were captured in `/tmp/protonsearch-existing-status.txt` and `/tmp/protonsearch-existing-baseline.sha256` during the task. Existing modified and untracked files were not reset, deleted, reformatted, or moved.

All migration implementation files are additive under `protonsearch-linux/`, `docs/linux/`, `scripts/linux/`, and `packaging/arch/`. The only pre-existing path intentionally referenced by the new files is `LICENSE`, which was not changed.

The in-repository verifier `scripts/linux/verify-existing-files.sh` makes the byte comparison reproducible for any saved baseline. The generated `target/` cache is excluded from the checked baseline because Cargo legitimately rewrites it during validation; all source, asset, documentation, configuration, and user-created files remain checked:

```text
scripts/linux/verify-existing-files.sh /tmp/protonsearch-existing-baseline.sha256
```

Any baseline hash mismatch is a failure. The final Git status is also checked to confirm that no existing source, manifest, lockfile, asset, documentation, script, or dotfile was changed by this migration.

The Windows crate remains separate and its existing `Cargo.toml`, `Cargo.lock`, `build.rs`, Rust sources, Slint UI, installer, and packaging metadata were not edited.
