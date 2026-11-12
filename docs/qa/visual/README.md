# Visual QA evidence

Screenshots in this folder are produced by tools/visual-qa/shots.mjs from the real
React UI rendered in Chromium against the browser mock bridge (clearly stated in
the report). Real Tauri/WebView2 window captures with live engine data are stored
in docs/qa/visual/app-captures/ and distinguished by filename.

Rules:
- Screenshots must be regenerated after UI changes; never commit stale evidence.
- Each screenshot set is listed in docs/qa/visual/VISUAL_AUDIT.md with viewport,
  DPI scale, theme and state, plus the reviewer's findings.
