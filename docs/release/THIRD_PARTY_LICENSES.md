# Third-Party Licenses (alpha snapshot)

Status: **informational, generated from local evidence; not a final legal notice bundle.**
No licence below is invented: each is taken from cargo metadata, installed npm package.json
files, or the package's LICENSE file. Full notice generation is a G7 task.

## 1. Rust workspace (cargo metadata, 544 resolved packages)

Command: cargo metadata --format-version 1 (exit 0) -> parsed packages[].license.

- **544 packages resolved; 0 have a missing license field.**
- Workspace crates are **MIT** (Cargo.toml [workspace.package] license = "MIT"):
  ps-ipc, ps-core, ps-index, ps-content, ps-sources, ps-windows, ps-ocr, ps-agent, ps-bench,
  protonsearch-v2 (Tauri shell), all 2.0.0-alpha.1.

License-expression counts (top):

| Expression | Packages |
|---|---:|
| MIT OR Apache-2.0 | 245 |
| MIT | 132 |
| Apache-2.0 OR MIT | 49 |
| MIT/Apache-2.0 | 19 |
| Unicode-3.0 | 18 |
| Zlib OR Apache-2.0 OR MIT | 17 |
| Unlicense OR MIT | 10 |
| Apache-2.0/MIT | 6 |
| MPL-2.0 | 5 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 5 |
| BSD-3-Clause | 4 |
| MIT OR Apache-2.0 OR Zlib | 4 |
| Apache-2.0 | 3 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 3 |
| others (Zlib, BSD variants, ISC, 0BSD, CC0/Unlicense, etc.) | remainder |

Notes:
- **MPL-2.0 packages** (5): cssparser 0.37.0, cssparser-macros 0.7.1, dtoa-short 0.3.5,
  option-ext 0.2.0, selectors 0.38.0 (transitive GUI/webview stack). MPL-2.0 is file-level
  copyleft: linking/distributing binaries is allowed if those files are not modified and
  licence/notices are kept. Confirm at G7 what the installer actually ships.
- **LGPL-2.1-or-later** appears only as one option in r-efi's triple expression
  (MIT OR Apache-2.0 OR LGPL-2.1-or-later) — we rely on the MIT/Apache option.
- **Unicode-3.0** (18 icu_*/zerovec packages) is a data licence; notices required.
- Direct workspace dependency licences: tauri 2.12.5 Apache-2.0 OR MIT; tauri-build 2.7.4,
  tauri-plugin-global-shortcut 2.4.1, tauri-plugin-single-instance 2.5.5 Apache-2.0 OR MIT;
  rusqlite 0.31.0 MIT; serde/serde_json MIT OR Apache-2.0; windows 0.58.0 MIT OR Apache-2.0;
  anyhow MIT OR Apache-2.0; pdf-extract 0.10.0 MIT; quick-xml 0.42.0 MIT; sha2 0.10.9 MIT OR
  Apache-2.0; walkdir 2.5.0 Unlicense/MIT; winreg 0.56.0 MIT; image 0.24.9 MIT OR Apache-2.0;
  encoding_rs 0.8.44 (Apache-2.0 OR MIT) AND BSD-3-Clause; zip 0.6.6 MIT.

## 2. Frontend (apps/desktop, 75 installed packages)

Command: scan of node_modules/*/package.json and node_modules/@*/*/package.json license
fields (npm 11.19.0).

| Licence | Packages |
|---|---:|
| MIT | 58 |
| Apache-2.0 | 4 |
| Apache-2.0 OR MIT | 3 |
| BSD-2-Clause | 2 |
| MPL-2.0 | 2 (lightningcss + win32 binary — build-time CSS tooling) |
| ISC | 2 |
| BSD-3-Clause | 2 |
| BlueOak-1.0.0 | 1 (lru-cache) |
| OFL-1.1 | 1 (@fontsource-variable/inter) |

Runtime dependencies bundled into the app: react 19.3.1 MIT, react-dom 19.3.1 MIT,
@tauri-apps/api 2.12.4 Apache-2.0 OR MIT, and the Inter font (OFL-1.1). Dev-only tooling
(vite, vitest, typescript, Tauri CLI, transitive lightningcss) is not shipped as app code but
is used to build the bundle; confirm what the Tauri bundler embeds at G7.

## 3. Fonts — Inter (OFL-1.1)

- Package: @fontsource-variable/inter 5.3.0 (npm), license field OFL-1.1.
- Licence text: apps/desktop/node_modules/@fontsource-variable/inter/LICENSE
  ("Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter) ... SIL OPEN FONT
  LICENSE Version 1.1"). Distributed .woff2 files are emitted into apps/desktop/dist/assets/.
- Obligations: include the OFL licence and copyright notice; do not sell the fonts alone; do
  not use Reserved Font Names in modified versions.

## 4. Python sidecar — pydantic-ai (not yet a committed component)

- Installed evidence: agent-sidecar/.venv/lib/site-packages/pydantic_ai-2.66.0.dist-info/METADATA
  -> License-Expression: MIT; pydantic_ai_slim-2.66.0 -> MIT, classifier
  "License :: OSI Approved :: MIT License".
- agent-sidecar is excluded from the Rust workspace and currently has **no committed
  pyproject.toml/requirements** (only local venvs/logs/scripts). Licence obligations must be
  re-verified when the sidecar ships (G6). Do not distribute pydantic-ai code before then.

## 5. Repo licence

- LICENSE (repo root): **MIT** (legacy ProtonSearch project licence; retained).
- v2 workspace declares MIT (Cargo.toml [workspace.package] license = "MIT").

## 6. Regeneration commands (no secrets involved)

```powershell
cargo metadata --format-version 1 > metadata.json
# parse packages[].license with any JSON tool

cd apps/desktop
node -e "read node_modules/*/package.json license fields"
```

Final G7 deliverable: a generated THIRD_PARTY_NOTICES bundle with full licence texts for every
shipped crate/npm/font, reviewed against the actual installer contents.
