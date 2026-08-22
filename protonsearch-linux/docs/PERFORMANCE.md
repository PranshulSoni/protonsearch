# ProtonSearch Linux performance model

ProtonSearch Linux uses bounded, on-demand providers. It does not maintain a
permanent filesystem index or a polling watcher, so the resident service stays
idle while the launcher is hidden.

At search time, ProtonSearch reads local hardware facts once and samples the
current load/power state. It selects one of three budgets:

- Low resource: up to 4 CPU threads or 6 GiB RAM, rotational storage, battery
  power, or a busy system. Traversal and provider limits are reduced.
- Balanced: the default for ordinary SSD systems.
- High performance: selected by capable hardware or explicitly in settings.

The mode is available in Settings → Indexing & Database. Adaptive mode is
recommended for installations across different Linux machines. The detector
reads `/proc`, `/sys`, and local filesystem metadata only; it does not send
hardware information anywhere.

The release benchmark can be run with:

```sh
./scripts/benchmark-linux.sh
```

It reports the binary hash, machine snapshot, CLI timings, and resident service
memory/task counters. It does not pretend to measure GTK paint latency or
cross-desktop shortcut behavior; those still require a desktop-session smoke
test on the target distribution.
