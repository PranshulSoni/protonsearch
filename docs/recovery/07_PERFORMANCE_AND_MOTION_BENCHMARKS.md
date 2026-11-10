# Performance evidence

Existing release island-switch probe was blocked (see baseline report). No valid before/after frame pacing, input latency, startup percentile or memory comparison is available yet. Existing tools/benchmark harnesses should be reused against embedded Release builds on the same host/profile. Do not infer smoothness from transitions or process Responding flags.

V9 reference measured with the existing harness: ../performance/results/recovery-v9-reference.json. Repository reference SHA256 matches supplied Downloads HTML. Headless Chromium / virtual 60Hz, 24 transitions, 957 frames: frame p50/p95 16.7ms, p99 16.8ms; zero reported dropped frames or long tasks. These are reference-browser measurements, NOT product performance or physical-display acceptance. No native comparison is claimed.
