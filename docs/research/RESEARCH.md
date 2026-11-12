# Research — Competitive Architecture Notes (ProtonSearch v2 / Project Phoenix)

**Owner:** L1-D (rel-recon) / **Date:** 2026-11-09 / **Status:** source of truth for ADRs (cite URLs when reusing)
**Method:** read the projects' official README/architecture/security docs and licence files (web_fetch), plus vendor FAQ/pricing pages. No code was copied. Where a claim could not be verified from a primary source it is explicitly marked **UNVERIFIED**.
**Why this matters:** the v2 design must borrow proven mechanisms (fast filename index, tiered retrieval, safe updater, read-only agent access) and consciously reject the failure modes found in our own legacy app (embedded keys, silent updater, monolith).

---

## 1. Kensaku (closest architectural peer; Tauri 2 + Rust)

**Sources:** repo <https://github.com/yjid/kensaku>, architecture doc <https://github.com/yjid/kensaku/blob/master/docs/ARCHITECTURE.md>, security doc <https://github.com/yjid/kensaku/blob/master/SECURITY.md>, README <https://github.com/yjid/kensaku/blob/master/README.md>, licence <https://github.com/yjid/kensaku/blob/master/LICENSE>.

### 1.1 Shape
- Local index with **four retrieval tiers** and three clients: desktop app (Tauri 2 + React), CLI (\`kensaku-cli\`), MCP server.
- One Rust library crate + small CLI workspace. **No indexing logic in the webview** — the same architectural rule v2 should adopt (Tauri shell thin, engine in Rust crates).
- Windows runs the indexer **in-process in the GUI**; Unix can use a daemon sidecar (JSON-RPC over Unix socket / Windows named pipe, with token + \`protocol_version\` checks).
- Single writer, file locks (\`kensaku.lock\`, \`kensaku-gui.lock\`), second launch writes \`window.request\` and exits.

### 1.2 Storage & metadata arena (borrow this idea)
- Durable truth: **SQLite in WAL mode** (\`metadata.db\`) for roots/nodes/semantic units/usage/meta; content index in **Tantivy** (\`index/\`, schema v4); optional static embedding table in \`models/\`; thumbnails in \`thumbs/\`.
- Hot path uses a **compact in-memory arena**, not \`PathBuf\` per entry:
  - 64-byte \`Node\` per file/dir,
  - concatenated folded names + path index,
  - **64-bit character bloom prefilter** per name,
  - small delta buffer for watcher changes, merged at idle.
- Identity is \`(device, inode)\` / Windows \`file_id\` — **never the path** — so renames update one row and never re-extract content. Unavailable roots are preserved, not pruned.
- Reference footprint: ~**128 bytes/entry**, ~12.2 MB for 100k files; filename search **18 ms avg** (best 8.3 ms) on Windows default threads, **3.1 ms avg** with 4 threads (README benchmark table; same 100k corpus).

### 1.3 Retrieval tiers (design precedent for our G2/G5)
1. **Fuzzy filename** — \`nucleo-matcher\` over the arena, path/name weighting, bloom prefilter, top-k.
2. **BM25 content** — Tantivy schema: stored path, filename, content, folded content, raw content, modified/ext/size fast fields; custom CJK unigram+bigram tokenizer; filters pushed down as \`Must\` clauses.
3. **Semantic** — static embeddings (tokenize → table lookup → weighted mean → normalize), i8 table ~7.3 MiB; optional ONNX "smart" model feature; exact scan at desktop scale, **no ANN below hundreds of thousands of units**.
4. **Folded/substring fallback** — look-alike normalisation (0/o, 1/l/i, 5/s, 8/b, m/rn, w/vv, d/cl) + trigram; candidates verified against raw content; ranked after exact hits.
- Query pipeline: **input → lexer → AST → router → planner → tiers → RRF → rank → snippet → IPC**; "every inference is a removable chip"; grammar wins over natural-language intent.
- Query language includes \`type:\`, \`ext:\`, \`in:\`, \`path:\`, \`modified:\`, \`size:\`, \`lang:\`, \`name:\`, \`content:\`, \`re:\`, \`mode:exhaustive\`, phrases, \`near[n]\`, negation. Unknown keys stay literal; parse errors give a hint, not zero results.
- Content caps: up to 100 MB/file, 4 MB indexed text, 48 semantic chunks; OCR PP-OCRv6 default, Indra optional, Tesseract fallback; hidden/build folders skipped by default.

### 1.4 Security/update posture (directly relevant to our G7)
- Threat model calls out untrusted documents/images, path traversal/symlink escapes, command injection via paths/content, XSS from indexed text, agent data leak, secrets in index/logs.
- Parsers run on a bounded pool with per-file timeout/panic isolation; decode caps (50 MP images); **no shell**; agent surface has no shell/network tools; read-only by default.
- MCP: stdio by default; loopback HTTP requires bearer token in \`<config>/http.token\` (0600), checks peer; scoped roots resolve paths before comparison so \`..\`/symlink escapes are rejected; credential files/extensions refused; tool responses budgeted; **file content treated as data, never instructions**.
- Updates: **bounded and verified** — timeout, expected size and \`SHA256SUMS\` check before running the installer; truncated downloads rejected. This is the bar our v2 updater must meet (our legacy updater fails it; see security register S-02).
- Known limitation: builds unsigned (SHA256SUMS + SmartScreen guidance only).

### 1.5 Licence
- **MIT**, Copyright (c) 2026 Yash Jangid (LICENSE file). Reuse is permitted with copyright/permission notice; if we copy concepts (not code) we still write our own implementation and cite this research note.

---

## 2. Everything (voidtools) — the filename-index benchmark

**Sources:** FAQ <https://www.voidtools.com/faq/>, Indexes docs <https://www.voidtools.com/support/everything/indexes/>, licence <https://www.voidtools.com/License.txt>, forum explanation of USN indexing <https://www.voidtools.com/forum/viewtopic.php?t=12779>.

### 2.1 Indexing model
- **Filename/path only by default**; optional size/date/attributes. It "only indexes file and folder names" and builds its DB in seconds: ~250k files ≈ 5 s, ~1M files ≈ 1 min (FAQ).
- **NTFS**: updates via the NTFS **USN Journal**, which is maintained by the NTFS driver (not by Everything). Changes are not missed while Everything is closed; it catches up from the journal at startup. NTFS indexing requires the **Everything service or running as administrator** (FAQ). ReFS supported; FAT/network shares need folder indexing; offline media via File Lists.
- **Content search exists but is not indexed** — \`content:\` searches are slow (FAQ). This is the key limitation we must not copy: ProtonSearch's differentiator is indexed content + OCR.
- Resource figures (FAQ): 250k files ≈ 35 MB RAM / <14 MB disk; 1M files ≈ 100 MB RAM / 45 MB disk. DB location \`%LOCALAPPDATA%\Everything\Everything.db\`; DB lives in memory while running and is saved on exit.
- Fast-sort indexes (size/date/path/extension) are opt-in and cost memory; rich query syntax/wildcards; result list updates in real time.
- Lite build removes HTTP/FTP server, IPC, SDK, and accessibility.

### 2.2 Lessons for ProtonSearch v2
- The USN/MFT path is *the* way to make a per-user launcher feel instant at 1M files — but it needs a **service or elevation**, which conflicts with our "no system-wide install / per-user" constraint (work breakdown: \`InstallMode perUser\`). Recommendation: **do not depend on USN for v2 G2**; keep a fast user-space crawler + warm in-memory arena, and add an optional privileged/USN helper later only if measured need appears (ADR candidate).
- Borrow: name+path arena, bloom prefilter, optional metadata fields, fast sort, clear separation "indexed vs gathered-on-demand".
- Do not copy: name-only index; content search on demand (too slow); administrator/service requirement as baseline.
- Licence: Everything's \`License.txt\` grants MIT-style permission (Copyright (C) 2018 David Carpenter) plus **PCRE (BSD, University of Cambridge)** for the regex component. It is distributed as freeware; the repository/source is not public for the app. Only ideas are reusable; no code copying.

---

## 3. Flow Launcher — plugin-ecosystem precedent

**Sources:** repo <https://github.com/Flow-Launcher/Flow.Launcher>, README, plugin-dev doc <https://github.com/Flow-Launcher/docs/blob/master/plugin-dev.md>, licence <https://github.com/Flow-Launcher/Flow.Launcher/blob/master/LICENSE>, plugin manifest repo <https://github.com/Flow-Launcher/Flow.Launcher.PluginsManifest>.

### 3.1 Model
- Windows launcher, **C#/.NET 9 + WPF**; "community-made plugins"; file search can delegate to Everything or Windows Index (README).
- Plugin contract: each plugin answers a \`query\` call with a list of **Result** objects (Title, SubTitle, IcoPath, Score, action). Results carry a score that influences ordering; actions are callbacks/JSON-RPC methods.
- Integration modes: **.NET plugins run in-process** (fast, full API), **other languages run out-of-process over JSON-RPC over stdout** (Python, Node/TS, any executable). Every plugin ships a \`plugin.json\` manifest (ID, ActionKeyword, Name, Version, Language, IcoPath, ExecuteFileName) that Flow discovers by folder scan.
- Distribution: plugin manifest repo + reviewed Plugin Store; policy bans malicious code, piracy, deception, etc.
- Features comparable to ours: system commands, calculator, Windows/Control Panel settings search, preview panel, plugins for browser bookmarks, Everything/Windows index, translations (Crowdin), portable data.

### 3.2 Lessons for ProtonSearch v2
- Borrow: **manifest-driven discovery**, two integration tiers (fast in-process vs isolated out-of-process), score-based result contract, store review policy. Even if v2 ships no plugin store, the *boundary* (engine ↔ source modules ↔ optional external processes) is proven and matches our \`ps-sources\` trait plan.
- Do not copy: in-process .NET plugin model (we are Rust + Tauri; loading arbitrary .NET in-process is a security non-starter), WPF UI stack, or the plugin-store scope (out of v2 scope; avoid speculative extension API).
- Licence: **MIT** (Copyright (c) 2019 Flow-Launcher; Copyright (c) 2015 Wox). The dual copyright matters if we reuse plugin-format documentation text.

---

## 4. PowerToys Command Palette — extension isolation precedent

**Sources:** Microsoft Learn extensibility overview <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/extensibility-overview>, extension development <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/extension-development>, overview <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview>, repo/licence <https://github.com/microsoft/PowerToys>.

### 4.1 Model
- Command Palette is a PowerToys utility (activation default **Win + Alt + Space**, compact search-box mode, Dock, bookmarks).
- Extensions are **standalone .NET applications** that talk to Command Palette over a **WinRT API**, each running **out of process**; discovered via the **Windows Package Catalog** from an \`.appxmanifest\` declaring \`windows.appExtension\` name \`com.microsoft.commandpalette\`, with an **out-of-process COM server** (CLSID) as the activation/communication layer.
- Extension API exposes \`IExtension.GetProvider → ICommandProvider\`; providers supply top-level, fallback, and context-menu commands; commands navigate to **list/detail/form/markdown/grid pages**; extensions can add settings pages and publish to the built-in Gallery (Store/WinGet/self-host).
- This is the opposite trade-off from Flow: **isolation and OS-managed discovery over raw speed**.

### 4.2 Lessons for ProtonSearch v2
- Borrow: out-of-process isolation for third-party/optional code; manifest + unique IDs; page-type model for richer UI; explicit "fallback command" concept (useful for "search the web when no local result" without hijacking queries).
- Do not copy: .NET/COM/appx packaging stack (our installer is NSIS/per-user; v2 sidecar is Python, opt-in); gallery/store scope.
- Licence: **MIT** (Microsoft Corporation) per repo LICENSE; docs are CC-BY-4.0 (Microsoft Learn default) — cite, don't copy text.

---

## 5. Raycast for Windows — UX benchmark, closed-source

**Sources:** product page <https://www.raycast.com/windows>, changelog <https://www.raycast.com/changelog/windows>, developer docs <https://developers.raycast.com/>, extensions repo licence <https://github.com/raycast/extensions/blob/main/LICENSE>.

### 5.1 State (verified from the Windows page, retrieved 2026-11-09)
- Raycast is **shipping on Windows** (page shows v2.11.1.0, Windows 10+, WinGet install). Feature surface on Windows includes app launcher, file search, clipboard history, snippets, quicklinks, calculator, window management, AI, and a growing set of extensions (GitHub, Linear, Slack, Spotify, Notion, Kill Process, etc.). Page notes they are "hard at work bringing all the features from macOS to Windows".
- Extension platform: **React + Node.js + TypeScript** API; built-in UI components; local/private extensions; Store publishing; docs at developers.raycast.com. Extensions are declarative manifests + React components, not native plugins.
- **The Raycast app is proprietary/closed source** (no app source repository). The **extensions repository is MIT** (Copyright (c) 2021 Raycast), so extension *source* can be read for API/UX ideas, but app code/design assets cannot be reused.

### 5.2 Lessons for ProtonSearch v2
- Borrow UX patterns only at the level of interaction principles already in our reference HTML: action panel on selection, keyboard-first navigation, quicklinks/snippets/clipboard/calculator as first-class commands, native-feeling window, extension store *concept* if ever scoped.
- Do not copy: proprietary UI/branding/assets; React-in-webview extension runtime for v2 (our differentiator is native Win32/Tauri performance and local-first indexing); no dependency on cloud/AI account.
- **UNVERIFIED:** exact Windows extension-API parity and whether the Windows client supports third-party extension installation today; do not cite parity as a competitive fact without a primary source.

---

## 6. Cross-cutting synthesis: borrow / reject

| Topic | Kensaku | Everything | Flow Launcher | PowerToys CP | Raycast Win | ProtonSearch v2 decision |
|---|---|---|---|---|---|---|
| Filename index | in-memory arena + bloom + nucleo | MFT/USN + name DB | delegates to Everything/Windows Index | own index (not public detail) | own index | **Borrow arena + bloom; user-space crawler for G2; USN optional later (ADR)** |
| Content search | Tantivy BM25 + folded + semantic | not indexed (slow) | via plugins | n/a | own index | **FTS5 for v2 (already planned); Tantivy as measured ADR if FTS5 limits ranking/CJK** |
| OCR | PP-OCRv6/Indra/Tesseract, caps | none | plugins | Text Extractor (PowerOCR) separate | n/a | **Windows.Media.Ocr first (legacy path proven), bounded pipeline; no per-query OCR** |
| Query language | rich grammar + intent chips | rich syntax | plugin keywords | command pages | aliases/keywords | **Keep prefix scopes, calculator; ADR on grammar vs chips** |
| Extension model | CLI + MCP + read-only agent surface | n/a | in-process .NET + JSON-RPC plugins | out-of-process .NET/WinRT appx extensions | React/TS store | **v2: internal \`ps-sources\` traits + optional Python sidecar; no plugin store in scope** |
| Agent access | MCP read-only, token-auth loopback | n/a | n/a | n/a | AI built-in | **Adopt MCP-style read-only sidecar contract, loopback token, approvals** |
| Update integrity | size+checksum verified | n/a | n/a | Microsoft Store/WinGet | store updater | **Signed manifest + SHA-256 + Authenticode; hard gate G7** |
| Privilege model | per-user, no service | service/admin for NTFS | per-user | system components | per-user | **Per-user only (constraint); document USN trade-off** |
| Licence compatibility | MIT | MIT-style + PCRE BSD | MIT | MIT | app proprietary / extensions MIT | **All reusable except Raycast app; keep notices** |

### 6.1 Concrete borrow list for v2 (map to gates)
1. **In-memory filename arena with file identity** (G2, \`ps-index\`): 64-byte-ish entries, folded names, bloom prefilter, delta buffer; identity = \`(volume, file_id)\` so renames don't re-index (Kensaku ARCHITECTURE §3).
2. **Tiered engine with deterministic merge** (G2/G5, \`ps-core\`): exact/prefix → fuzzy filename → FTS content → OCR/semantic; no Tier-2 blocking Tier-0 (already in work breakdown; Kensaku RRF is a proven merge model).
3. **Query grammar with removable intent chips** (G3/G4): explicit syntax wins, unknown prefixes stay literal, parse errors are hints (Kensaku §5; avoids legacy's hardcoded prefix chain).
4. **Update verification** (G7): signed manifest + SHA-256 + Authenticode signer pin + bounded download (Kensaku SECURITY "Release integrity").
5. **Agent access contract** (G6): read-only default, stdio first, loopback with random token + peer checks, scoped roots, content-as-data, no shell for the agent by default (Kensaku SECURITY; PowerToys isolation rationale).
6. **Sensitive-path default deny** (G4/G5): hidden dirs, \`.ssh\`, \`.gnupg\`, \`.aws\`, credential extensions (.pem/.key/.pfx/.p12), password-manager folders, \`.env*\` (Kensaku SECURITY).
7. **Per-file extraction caps + timeout + panic isolation** (G5, \`ps-content\`/\`ps-ocr\`): Kensaku's caps (100 MB / 4 MB text / 48 chunks / 50 MP) are a sane starting point; our legacy already isolates PDF/DOCX in a child process.
8. **Manifest-style source registration** (G1/G2): Flow/PowerToys prove the manifest + typed provider contract; our \`SearchSource\` trait should be the internal analogue (no dynamic loading in v2).

### 6.2 Explicit reject list
1. Embedded/shared operator API keys (legacy S-01/S-03; nobody above ships operator keys in the client).
2. Silent, unsigned self-updates (legacy S-02). Kensaku sets the correct bar.
3. Name-only index (Everything) — content/OCR is our differentiator.
4. USN/MFT dependency requiring admin/service for baseline v2 (constraint conflict).
5. In-process third-party plugin loading (Flow) or appx/COM extension packaging (PowerToys) in v2 scope — too much surface for a per-user, single-binary app.
6. Raycast app code/design assets (proprietary); its public extension sources are MIT but the runtime is not ours.
7. Monolithic 13 KLOC main + 10 KLOC search files; blocking HTTP in UI-adjacent threads; fixed-name temp files.

---

## 7. Licence & attribution notes (for ADRs/release)

| Project | Code licence | What we may reuse | Attribution requirement |
|---|---|---|---|
| Kensaku | MIT (© 2026 Yash Jangid) | ideas; code if copied with notice | include MIT notice if code copied; cite repo in ADRs |
| voidtools Everything | MIT-style (© 2018 David Carpenter) + PCRE BSD (© 1997-2012 University of Cambridge) | ideas only (app source not public) | if PCRE used via Everything code — not applicable; cite vendor docs |
| Flow Launcher | MIT (© 2019 Flow-Launcher, © 2015 Wox) | ideas/manifest format concepts | MIT notice if code/docs copied |
| Microsoft PowerToys | MIT (© Microsoft Corporation); docs CC-BY-4.0 | ideas; code with MIT notice | MIT notice; docs attribution if text copied |
| Raycast | App **proprietary**; extensions repo MIT (© 2021 Raycast) | UX ideas; extension code under MIT if copied | do not reuse app assets/branding; MIT notice for extension code |
| Tantivy (if adopted) | MIT-style (© 2018 project authors, AUTHORS file) | dependency | standard MIT notice |
| nucleo-matcher (if adopted) | MIT (helix-editor/nucleo) — **UNVERIFIED here**; verify before adoption | dependency | verify licence at adoption time |

**Rule for v2:** every new third-party dependency gets its licence recorded in the release notes/SBOM at G7; no code from Raycast app; MIT notices bundled for any copied snippet (> trivial).

---

## 8. Open questions / unverified items
1. **Raycast Windows extension parity** — page markets extensibility; whether third-party extensions install/run on Windows today is **UNVERIFIED** (do not claim in ADRs).
2. **Kensaku benchmarks** on Windows are self-reported; we did not run them. Treat as directional, not as our acceptance numbers.
3. **Everything source** is not public; MFT/USN behaviour inferred from vendor docs/forum, not code.
4. **PowerToys Command Palette** requires Windows App SDK/.NET toolchain; its packaging model is not a candidate for our NSIS per-user installer.
5. **Flow Launcher plugin performance** and store review latency not measured.
6. **Tantivy vs FTS5** for our corpus must be decided by benchmark at G2 (we have 89,813 files / 14,609 content docs today; FTS5 latency measured at 0.03 ms for a single-term query on this DB — see Gate-0 baseline §3.4 — but that excludes ranking/merge).

---

## 9. Sources (retrieved 2026-11-09)

- Kensaku repo: <https://github.com/yjid/kensaku> · architecture: <https://github.com/yjid/kensaku/blob/master/docs/ARCHITECTURE.md> · security: <https://github.com/yjid/kensaku/blob/master/SECURITY.md> · licence: <https://github.com/yjid/kensaku/blob/master/LICENSE> · benchmark section: README "Performance"
- voidtools Everything: <https://www.voidtools.com/faq/> · indexes: <https://www.voidtools.com/support/everything/indexes/> · licence: <https://www.voidtools.com/License.txt> · USN forum answer: <https://www.voidtools.com/forum/viewtopic.php?t=12779>
- Flow Launcher: <https://github.com/Flow-Launcher/Flow.Launcher> · plugin dev: <https://github.com/Flow-Launcher/docs/blob/master/plugin-dev.md> · licence: <https://github.com/Flow-Launcher/Flow.Launcher/blob/master/LICENSE> · plugin store manifests: <https://github.com/Flow-Launcher/Flow.Launcher.PluginsManifest>
- PowerToys Command Palette: extensibility <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/extensibility-overview> · development <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/extension-development> · overview <https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview> · repo/licence <https://github.com/microsoft/PowerToys>
- Raycast: Windows <https://www.raycast.com/windows> · changelog <https://www.raycast.com/changelog/windows> · developer API <https://developers.raycast.com/> · extensions licence <https://github.com/raycast/extensions/blob/main/LICENSE>
- Tantivy licence (if adopted): <https://github.com/quickwit-oss/tantivy/blob/main/LICENSE>
