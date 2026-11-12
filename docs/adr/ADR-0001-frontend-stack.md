# ADR-0001: Frontend stack — Tauri 2 + React + TypeScript on WebView2

- Status: **Accepted** (Gate 1, 2026-11-09)
- Owners: L1-B Product Design & Experience (proposal), L1-A Chief Architect (approval), L0 (decision)
- Supersedes: legacy native Win32 + Slint hybrid UI

## Context

The legacy ProtonSearch v1.1.1 UI is a hand-built Win32 launcher (main.rs is 542 KB, search.rs 428 KB, plus Slint for the settings window). It works, but:
- It is one giant message-loop-driven file; every visual change risks search-path regressions, and the settings UI uses a different toolkit than the launcher.
- Matching the supplied v2 reference (a dense, typographic, keyboard-first design with three views) in raw Win32/GDI would be slow to build, hard to make responsive at arbitrary DPI, and nearly impossible for a small team to art-direct consistently in light and dark themes.
- The supplied master prompt requires this exact visual language and explicitly recommends Tauri 2 + React + TypeScript + WebView2.

Environment evidence (measured on this host):
- WebView2 runtime present: 157.0.4341.42.
- Rust 1.99 MSVC toolchain present; Node 26.7.0; npm registry reachable; crates.io reachable.
- The engine remains pure Rust behind a versioned command surface, so UI failure cannot break filename search.

## Decision

Use **Tauri 2** (Rust host process + WebView2 renderer) with **React 19 + TypeScript + Vite 8** for all three experiences (Quick Launcher window, Desktop App window, and the Onboarding route inside the desktop window).

Constraints that make this safe:
1. The search engine is an in-process Rust library. The UI talks to it through one versioned, typed bridge (crates/ps-ipc mirrored in apps/desktop/src/bridge/types.ts). No network, no Python, no OCR model is needed for filename search.
2. The launcher window is created hidden at startup and pre-warmed; the global hotkey only shows/focuses it. There is no window creation or WebView cold start on the hotkey path.
3. Search is a single command per keystroke with monotonic request IDs; the engine does all ranking in Rust and returns at most the requested top-k. The WebView never scans the filesystem.

## Alternatives considered

- **Keep native Win32/Slint**: rejected for delivery risk and design fidelity; retains value as the legacy baseline and as a fallback if Tauri cold-start/memory measurements fail the Gate 3 thresholds.
- **WinUI 3 / C#**: rejected — adds a second runtime/toolchain and does not reuse the Rust engine as cleanly; larger installer, more moving parts.
- **Electron**: rejected outright by the brief and by memory/startup cost.
- **Slint for everything**: rejected — good for one window, but the required responsive, themeable, text-dense views with virtualized lists are materially easier in React; also the legacy Slint settings code has existing structural warnings.

## Consequences

- Two processes by design: app host + WebView2 renderer(s); memory must be measured (target: app + renderer under ~150 MB idle, reported honestly in the benchmark report). This is a real cost versus a pure native UI and is recorded as a known limitation if targets are missed.
- Need a CSP and capability allowlist: no remote content, no shell plugin, structured actions only.
- The installer grows (WebView2 evergreen is already on Windows 11; bootstrapper included for older systems).
- A hidden pre-warmed launcher window must not poll; it receives events and only runs a query when shown/typed. Idle CPU must remain ~0.
- If Gate 3 measures hotkey-to-first-visible-pixel above the 50 ms p95 budget in a way that cannot be fixed with pre-warm/pre-render, this ADR will be revisited (recorded fallback: native launcher shell + WebView for the desktop app only).
