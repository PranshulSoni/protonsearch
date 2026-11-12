# PROTONSEARCH V2 — MASTER EXECUTION PROMPT
## Project Phoenix: Build a best-in-class, local-first Windows search and command center

**READ THIS ENTIRE BRIEF BEFORE CHANGING A FILE.** You are not being asked to produce a plan, mockup, architectural essay, or collection of unfinished prototypes. You are being assigned to **inspect, design, implement, integrate, benchmark, test, package, and document a working ProtonSearch v2**, starting from its actual repository.

### Source of truth and supplied artifacts

- Existing product repository: **https://github.com/PranshulSoni/protonsearch**. This is the starting codebase; keep its project history and respect its existing user data.
- Competitive engineering/reference repository: **https://github.com/yjid/kensaku**. Study objectively, and also study Everything, Flow Launcher, PowerToys Command Palette, Raycast for Windows, and other relevant tools. Their implementations, architectures and measured behavior are reference material, not a mandate to copy.
- **Required UI source reference:** `protonsearch-v2-ui-reference.html`, provided beside this prompt. Open it in a browser, inspect it at multiple sizes, and use its *actual markup, styling, view hierarchy and interactions* as the starting visual specification. It contains three interactive views: **Launcher**, **Desktop app**, **Onboarding**. It is a design reference, not production-ready code and not a functional filesystem client.
- **Brand mark references:** `assets/protonsearch-mark.svg` and `assets/protonsearch-logo-concept.png`; the user's small original crop is `assets/original-logo-crop-reference.png`. The vector is an approximation, not an authority superior to the original image. Refine with appropriate SVG/visual tooling if needed, preserving the recognizable dark-navy/blue geometric **P** and ensuring legibility from 16px to large sizes. Do not make gradients/glow/glass the defining brand aesthetic.
- Product name: **ProtonSearch**. Do not accidentally rename it to “Proton Switch”; the user was referring to the full app versus the quick launcher. Preserve migrations from the earlier **OmniSearch** identity.
- If a reference file is not available in your working environment, explicitly report the missing artifact and reconstruct its requirements from this document. Never silently pretend you inspected it.

## I. Mission and non-negotiable principles

Build the most compelling *measurably fast* Windows-first launcher/search product feasible: fast file and folder lookup, indexed document/code/full-text search, OCR search over screenshots and scanned documents, application launching, clipboard and browser recall, window controls, useful native actions, local semantic retrieval when opted into, and an optional agent layer using an **existing Python agent framework**, not an invented agent orchestration engine.

The user wants outstanding performance, an exceptionally responsive and polished interface, a real onboarding flow, a beautiful full desktop app, and functionality that feels coherently integrated. The app should be capable of becoming more useful than Kensaku for Windows workflows while maintaining a remarkably light and reliable core. **“Fastest in the world” is an aspiration and a testing challenge, not a claim to be put in marketing without reproducible, fair comparative evidence.** Never fabricate throughput, latency, RAM, OCR accuracy, test coverage, release status or observed UX quality.

**Hard rules:**
1. **Do not rewrite from scratch before auditing the existing implementation.** Preserve working capabilities and compatibility. Perform an evidence-based architecture decision record (ADR) for every significant replacement.
2. **No artificial success:** code compiling is not equivalent to a usable application. Run and inspect the real Windows app, its launcher, onboarding, settings, and key interactions.
3. **Core search may never depend on WebView initialization, an LLM, network access, MCP, Python, an OCR model, or semantic embeddings.** Those are optional or separate processes/layers.
4. Prioritize **latency, correctness, keyboard behavior, safety, maintainability, power use and search relevance** above raw feature count.
5. **No UI slop**: no gradients as decoration, excessive purple, glossy glass, neon glows, giant tiles, generic three-card dashboards, meaningless badges, oversized whitespace, bounce animations, or gimmicky AI indicators. Design must be contextually restrained, differentiated, precise, and densely useful.
6. Never remove an existing feature merely because it is inconvenient to port; first inventory it, retain it, or propose and prove a safe migration/replacement.
7. User's source/index/clipboard/AI credentials are sensitive. Default to local-first, minimal permissions, visible opt-ins, and no telemetry or network egress without a disclosed user-approved purpose.
8. Do not claim that any helper sub-agent, external browser, test environment, operating system, or skill was used unless it actually was. If parallel sub-agents are unsupported, execute the same roles sequentially and disclose that limitation.

## II. Required hierarchy of sub-agents — an actual engineering organization

You are **L0 — PROGRAM DIRECTOR / PRINCIPAL ENGINEERING ORCHESTRATOR**. You own scope, architectural coherence, schedules/dependencies, integration, final acceptance, and truthful reporting. Do not merely delegate tasks and concatenate outputs. Establish an agent plan and a dependency graph; assign ownership, output contracts, test responsibilities, and integration points before coding.

### Level 1: four leads reporting directly to L0

**L1-A. Chief Architect & Integration Lead**
Owns cross-process contracts, the Rust crate boundaries, persistent schemas, compatibility, architecture decisions, integration branches, code reviews, and conflict resolution. *Every backend/frontend/agent interface requires this lead's approval.*

**L1-B. Product Design & Experience Lead**
Owns information architecture, the visual design system, fidelity to the supplied HTML, keyboard journeys, interaction motion, accessibility, launcher / full-app / onboarding consistency, and screenshot review. Directly accountable for getting real rendered states right, not merely producing Figma-like descriptions.

**L1-C. Performance & Search Quality Lead**
Owns benchmark methodology, baseline profiling, filename engine, query ranking, index/update efficiency, OCR throughput and quality, memory footprint, battery/power impact, regression budgets, and competitive comparisons. Must veto unsupported “fastest” claims.

**L1-D. Reliability, Security & Release Lead**
Owns threat modeling, permission design, tests, CI, installer/updater, crash diagnosis, migration validation, recovery paths, documentation, code signing strategy, reproducible builds, release gate and end-to-end test evidence.

### Level 2: specialist implementation teams under leads

Assign independent specialist agents (or equivalent role instances) with *specific scoped deliverables*. Suggested ownership hierarchy:

```text
L0  PROGRAM DIRECTOR / ORCHESTRATOR
│
├── L1-A  CHIEF ARCHITECT / INTEGRATION LEAD
│   ├── L2-A1  Rust Core Refactor & IPC Contracts Engineer
│   ├── L2-A2  Windows Native APIs & Filesystem Engineer
│   ├── L2-A3  Plugin/Extension and Backward-Compatibility Engineer
│   └── L2-A4  Python Agent Sidecar & MCP/ACP Integration Engineer
│
├── L1-B  PRODUCT DESIGN & EXPERIENCE LEAD
│   ├── L2-B1  Launcher UI / WebView2 Engineer
│   ├── L2-B2  Desktop App / Settings Engineer
│   ├── L2-B3  Onboarding / Personalization Engineer
│   └── L2-B4  Interaction, Accessibility & Visual QA Engineer
│
├── L1-C  PERFORMANCE & SEARCH QUALITY LEAD
│   ├── L2-C1  Fast Filename/Folder Index Engineer
│   ├── L2-C2  Content, Document & Code Retrieval Engineer
│   ├── L2-C3  OCR / Semantic Search Engineer
│   └── L2-C4  Benchmark, Profiling & Relevance Engineer
│
└── L1-D  RELIABILITY, SECURITY & RELEASE LEAD
    ├── L2-D1  Rust/Frontend Integration Test Engineer
    ├── L2-D2  Security, Privacy & Agent Permissions Engineer
    ├── L2-D3  Installer, Updater, Migration & CI Engineer
    └── L2-D4  Documentation, Packaging & Release QA Engineer
```

**Level 3:** create temporary, narrow workers for tasks such as profiling Rust hot functions, adding one file extractor, creating golden ranking fixtures, reproducing one Windows bug, testing one OCR model, or screenshot-auditing one onboarding state. Give each worker a clear issue, bounded write scope, acceptance test, and deliverable. Close the task when finished.

### Mandatory delegation protocol

- L0 first creates `docs/implementation/WORK_BREAKDOWN.md` with ownership, prerequisites, acceptance conditions, named integration contracts, estimated risks, and a running status ledger. Maintain it as work progresses.
- Each task requires **TASK BRIEF → INVESTIGATION EVIDENCE → IMPLEMENTATION → SELF-TEST → PEER REVIEW → INTEGRATION TEST → ACCEPT/REWORK**.
- Do not allow two agents to edit the same files concurrently. Use isolated worktrees or clean feature branches if supported; otherwise serialize writes with explicit code ownership. Keep modular, stable API boundaries.
- Agents should pass artifacts and precise interfaces, not vague descriptions. Examples: Rust trait/API, TypeScript interface, JSON event schema, SQL migration, benchmark dataset, screenshots, test results.
- For each feature, designate one *implementer*, one *reviewer from another team*, and one *integration owner*. UI preview rendering, query IPC and agent process management deserve additional cross-team review.
- Every agent must report **exact files changed**, evidence/tests run, results, known limitations, security implications, and rollback strategy. “Looks good,” “fully tested,” and “fast” without concrete evidence are invalid.
- Run independent performance, visual, and security reviews after major integration milestones. L0 resolves competing findings and reruns objective tests.
- Reuse installed skills where present: **Taste Skill** for distinct visual direction, **Emil Design Engineering** for interaction craft, **Impeccable** for visual critique and accessibility, and **Vercel Web Design Guidelines** for usability/performance. Inspect each actual skill before invoking it. Do not invent skills or blindly trust their recommendations.
- If external research is enabled, validate current docs and repository state. Track relevant citations/links in `docs/RESEARCH.md` and explain why each borrowed pattern fits this product.
- Work autonomously through milestones. Ask the user only when genuinely blocked by a destructive/irreversible decision, unavailable secrets/credentials, access privileges, or conflicting product requirements. Do not stop after creating a plan.

## III. Phase 0 — forensic baseline of the original application

Clone/inspect the actual ProtonSearch repository. Read README, license, Cargo manifests, source tree, existing feature flows, installer/update scripts, migrations, and issue history. Build and run current functionality on Windows, including the keyboard launcher, settings app, browser/clipboard/Git/AI integrations, existing database and upgrade path. Inventory all user-visible functionality in `docs/implementation/FEATURE_PARITY_MATRIX.md`, marking **existing/working, existing/broken, not present, planned**; never conflate README claims with verified behavior.

Audit the existing large Rust files (`main.rs`, `search.rs`, `indexer.rs`, etc.) for component boundaries, state coupling, synchronization, blocking I/O, rank merging, unsafe FFI, memory lifetime, Windows APIs, serialization, and testability. Large-file size alone is not proof of slowness; profile it. Inspect existing reported issues and check whether they still apply: folder categorization, Unicode/UTF-8 slicing, timestamp arithmetic, browser-history epochs, database foreign keys, cleanup of temporary files, subprocess command injection, and possible historical credential exposure. **Never print leaked credentials in your report; recommend revocation/rotation if confirmed.** Fix any critical security flaws before deploying wider agent functionality.

Study Kensaku's public architecture and benchmark harnesses, particularly its fast filename retrieval, compact metadata arena, Tantivy content index, query tiers, OCR, indexing, and MCP exposure. Avoid cloning Kensaku wholesale. Verify licenses and attribution for anything reused.

Produce: a current feature inventory, baseline latency/memory/indexing/OCR report, known bugs, security risk register, concise technical debt map, and an initial ADR on whether a Tauri/WebView2 migration is warranted. Preserve a working baseline branch/tag before beginning significant changes.

## IV. Core architecture — Rust first, UI second, Python optional

Adopt a **Windows-first modular Rust search engine** independent of GUI toolkit. A possible module breakdown (revise based on measured evidence):

```text
protonsearch/
  crates/
    ps-core/             # domain types, query coordinator, cancellation, ranking
    ps-windows/          # NTFS/USN, Win32, apps, windows, shell, icons
    ps-filename/         # compact metadata and fast name/path lookup
    ps-content/          # inverted index, text extractors, source positions
    ps-ocr/              # OCR extraction service and caching
    ps-semantic/         # optional embeddings, vector retrieval
    ps-sources/          # browser, clipboard, Git, notes, games, settings
    ps-actions/          # safe native commands, quicklinks, snippets
    ps-ipc/              # versioned contracts and event framing
    ps-agent-bridge/     # optional subprocess lifecycle and permissions
    ps-bench/            # reproducible micro + E2E benchmarks
  apps/
    desktop/             # Tauri 2 + React + TypeScript, WebView2 on Windows
    cli/                 # minimal query/index/diagnostic CLI
  agent-sidecar/         # optional, independently packaged Python runtime
  tests/                 # cross-subsystem golden corpora, fixtures, E2E
  docs/                  # architecture, ADRs, performance, security, release
```

This is a suggested target architecture, **not permission for an indiscriminate rewrite or to create empty crates**. Incrementally extract stable modules while keeping the existing application usable. Preserve internal APIs where practical; favor thin, explicit interfaces and deterministic tests.

### The nonblocking query path

1. User presses configurable global hotkey (default should preserve existing behavior if possible) → a preinitialized, hidden/resumable frameless launcher appears with input focused; no cold Python startup.
2. Input is processed immediately in UI state; submit a *versioned* request with a monotonic request ID to the Rust query coordinator. Do not debounce away responsiveness; use targeted scheduling for expensive tiers.
3. **Tier 0**: exact/prefix/app/recents/native command matches from memory, plus compact filename candidates. Return first results as quickly as safely possible.
4. **Tier 1**: fuzzy names and indexed content (full-text/BM25) with bounded CPU work and deterministic result ranking.
5. **Tier 2**: cached OCR text, optional local semantic results, deeper previews. Already-extracted OCR text is searched through a text index; OCR inference does not run per keystroke.
6. Merge results via a measured ranking strategy that distinguishes exact app/name match, prefix, path, content relevance, recency, usage frequency, and query intent. Show source labels where helpful. Prevent low-confidence semantic results from replacing clear exact hits.
7. Drop/cancel late results for outdated request IDs; never let a stale query reselect the wrong item. Stabilize selected result identity across incremental updates.
8. Index writes and model tasks never block keyboard events, list scrolling, preview rendering, or hotkey activation.

### Windows filesystem index: aggressively benchmarked

Evaluate and prototype an efficient initial NTFS enumeration strategy, including MFT/file-ID-oriented approaches **only where supported, legally accessible and permission-appropriate**. Investigate change tracking via NTFS USN journal (`FSCTL_QUERY_USN_JOURNAL`, `FSCTL_ENUM_USN_DATA`, `FSCTL_READ_USN_JOURNAL` as applicable), filesystem watchers, and low-overhead reconciliation. Implement graceful fallback scanning on FAT/exFAT/ReFS/network drives, unsupported Windows versions, unavailable journals, restricted permissions, OneDrive placeholders, and removable devices. Never require Administrator access merely to use basic search. Keep stable file identity for rename/move handling where possible; avoid re-extracting unchanged contents.

Investigate compact metadata storage (arena/SoA/path interning), candidate prefilters (character bloom/trigrams/prefixes), bounded top-k scoring, branch/cache locality, allocation reduction, and low-contention reads. Compare fuzzy libraries (e.g., nucleo-matcher) in real workloads. Avoid brute-force full scans at 1M+ entries when query selectivity allows faster candidates. Maintain correctness with Unicode, case folding, CJK, long paths, junctions/reparse points and inaccessible directories.

### Content search / document search

Profile existing SQLite FTS5 against **Tantivy**, on equal data and query suites, including index size, write amplification, update throughput, RSS, p50/p95/p99 query latency, snippets, phrase matches, filter pushdown, and recovery. Choose the best workload-specific option; hybrid approaches are permitted only if simpler options fail measured criteria. Implement reliable extraction for PDF, DOCX, PPTX, XLSX, Markdown, text, source code, JSON/YAML, EPUB and other prioritized formats, subject to actual extractor quality and platform dependencies. Record matching source offsets/line numbers when reliable; enable **Open at match** for supported editors and fallback when unavailable. Never invent line numbers or execute filenames via a shell string.

### OCR search

Compare Windows native OCR, PaddleOCR/PP-OCR variants, Tesseract, and relevant local accelerators with a fixed image/document corpus covering UI screenshots, small text, scanned PDFs, rotated/low-quality pages, numbers, symbols, code and multiple scripts. Measure **recognition accuracy separately from OCR inference speed**, plus model download/installed size, warm/cold inference, CPU/GPU/NPU use, batch throughput, and peak memory. Use OS-native acceleration only after validating hardware support and real speed/quality gains. OCR is an **asynchronous extraction pipeline** with bounded decoded pixels, timeouts, deduplication by content hash, safe caching, retries, incremental updates and user-configurable folder/indexing modes.

### Power, reliability and crash recovery

Use bounded worker queues, backpressure, priority-aware scheduling, adaptive concurrency and battery-aware policies. Don't saturate all logical cores after reboot or startup. Checkpoint durable indexing progress periodically so interruption does not reprocess the entire corpus. Use transactional schema migrations, corruption detection, repair/rebuild options, capped caches, observability, logging with privacy-safe redaction, and controlled recovery after interrupted extraction. Keep the search engine working if a plugin, webview renderer, OCR worker or Python sidecar crashes.

## V. UI: implement the supplied HTML direction faithfully, not generically

**Default frontend choice:** Tauri 2 + WebView2 on Windows + React + TypeScript, with Rust core behind versioned commands/events. Validate the cold/warm startup and process-tree RSS cost versus current native Rust UI before committing irrevocably. If WebView2 materially harms latency/memory targets, document evidence and propose a hybrid or native fallback; **do not use Electron by default**.

**Use `protonsearch-v2-ui-reference.html` as the literal visual foundation**, not as an excuse to produce a different dashboard. Open and test the reference, extract tokens/layout/hierarchy, and implement real reusable components with proper accessibility. The HTML's sample file paths/counts are mocks; replace with real data. Preserve these decisions:

### A. Quick Launcher: the primary hero experience
- **Frameless floating surface.** No conventional minimize/maximize/close buttons, no application title bar above the search bar. The P logo sits naturally at the start of the search input.
- Exactly the *right* structure: **search row → responsive results → understated footer**. Start compact; grow appropriately with results and optional preview. No unnecessary tabs, fake command buttons, decorative metric cards, or permanent AI status badge.
- **No “Ctrl+K” hint** or generic hotkey pill in the search field. User-configurable global summon shortcut is managed in settings; don't litter search with keyboard badge art.
- Keep **navigation shortcuts ONLY in the footer**, e.g. ↑/↓ Navigate, Enter Open, Esc Close/Clear according to verified behavior. Show relevant hints contextually, and make all actions keyboard accessible.
- Exact and high-confidence results must surface immediately. Use careful category grouping, precise text hierarchy, first-class app icons, realistic file icons, quiet separators, and an unobtrusive selected row.
- Selection change must feel instantly responsive; animation should not add input latency. Show previews only when useful (e.g., expansion or selected-item preview). Provide Ctrl/Space preview shortcuts if selected in settings, but don't confuse launch UI with help text.
- Support intelligent app/file/clipboard/native-action ranking, optional prefixes and filters, search terms, path scope, edit/rename/delete actions **with confirmation where destructive**, result context menus, multi-select where appropriate, and search/result keyboard reliability.
- The surface is a utility. Avoid heavy always-on document previews in the compact mode; provide an expanded Explore view for serious searching.

### B. Full Desktop App: not just the launcher stretched larger
- Build a polished, coherent companion app with navigation: **Home, Search & Indexes, Activity, Workflows, Preferences** (adjust taxonomy only with demonstrated usability benefit).
- Home should offer a **personalized time-of-day greeting** using an optionally entered first name and localization-aware time; never infer or extract identity without consent. A name is optional; provide tasteful neutral fallback.
- Show recent activity (respect opt-in/privacy), quick search entry, indexing coverage/health, useful source-specific management, and clear workflows. No empty analytics dashboard filler or huge useless hero cards.
- Search & Indexes: root-specific file name/content/OCR modes, exclusions, cloud placeholder handling, disk cost, status, errors, per-source health, pause/resume/rebuild and storage diagnostics.
- Preferences: global hotkey conflicts, fonts/density, light/dark/system themes, accessibility/reduced motion, privacy and history, agent connections, model downloads and cache, plugin controls, updater and diagnostics.
- Application must be beautiful and responsive at laptop sizes, scaled Windows DPI, high-contrast settings, dark theme, and keyboard-only usage.

### C. Real onboarding and upgrades
- True first-launch flow, **not a decorative greeting page**: introduction and privacy, optional name, choose indexed folders, choose names-only/content/OCR per root, understand resource tradeoffs, decide whether to configure agent/semantic models, configure or accept hotkey, then index-and-use.
- Filename search should become usable early while content/OCR indexing continues. Show truthful progress, errors and recovery; first-run tasks should be cancellable and resumable.
- **No account required. No forced cloud sign-in. No silent model download.** Explain exactly which data leaves the device when user opts into external providers.
- Existing ProtonSearch/OmniSearch users must be detected and migrated **without wiping settings, indexes, pinned clips, snippets, history or configured integrations**; offer rollback, export and repair paths. Do not force existing users through first-run setup.

### D. Visual craft and motion
- Brand mark: the supplied two-tone navy P. Visual language: **warm off-white, charcoal, dark navy, subdued blue as an accent, hairline separators, crisp variable-font typography, restrained corner radii**. Dark mode must be separately art-directed, not a naïve inversion.
- No purple gradient, glass blur, glowing controls, bouncing hover effects, fake green-dot AI badge, random sparkles, or generated dashboard clutter. Build a visual system with exact spacing, typographic scale, density rules, component states, and meaningful micro-interactions.
- For 60Hz screens, protect the ~16.7ms frame budget; for capable 120Hz displays, pursue ~8.3ms frame budget. Prefer `transform` and `opacity` for motion; virtualize large lists, defer expensive preview decoding, minimize WebView2↔Rust chatter, avoid React rerender storms and layout thrash.
- Selection transition: approximately **80–140 ms**, cancelable, with visual motion following user input without blocking it. No queued lag on rapid arrow presses. Smooth scrolling without scroll hijacking. Honor `prefers-reduced-motion`.
- Build and compare screenshots for: first summon, empty input, 0 matches, 1 match, many matches, selected result, preview, loading, index-error, offline, dark mode, onboarding each step, app home, settings, narrow width, 125/150/200% DPI, high contrast, keyboard focus.

## VI. Optional agentic workflow layer — adopt, do not reinvent

Use a **real existing Python agent runtime**, selected by a short licensed, measured spike. Candidate starting points include Nanobot, Pydantic AI (framework + integrations), or another actively maintained, appropriately licensed Python runtime. Evaluate Windows compatibility, embeddability, installation size, idle/warm memory, startup latency, tools/MCP support, model provider integration, controllable permissions and structured streaming. Record decision in an ADR. Do not create a custom planning/reasoning/tool-loop framework if a library/runtime already satisfies the needs.

- Launch the Python runtime **on demand** as an isolated, cancellable subprocess; keep it absent when user is merely searching. Use a small, versioned JSON-lines or equivalent local IPC protocol between Rust and Python; stream events, progress, tool requests, approvals and final results. Define schemas and timeout/cancellation behavior before frontend implementation.
- Existing Rust capabilities should be exposed to the agent through a **narrow, explicit permissioned API** rather than granting free OS access. Reuse known MCP tools where safe, but MCP is a *tool protocol*, not an agent runtime. Consider ACP for externally installed coding agents in a later phase.
- Support user-provided API keys and compatible local models/providers; credential storage must use OS secret storage where feasible. No hardcoded provider credentials, bundled secrets, or implicit billing.
- Make the agent UI **task-oriented**, not a permanent “Ask AI” advertisement: user invokes a workflow; sees plan/tool activity, previewable affected files, clear per-action approvals, cancellation, output and error handling. Read-only operations can have gentler policy; writes, deletion, shell execution, network, sensitive files and sending data externally require explicit appropriately scoped approval. No prompt injection from retrieved documents as authoritative instructions.
- Treat terminal execution/Windows UI automation as risky. Never pipe untrusted filenames into `cmd /c`; pass validated structured arguments, bounded privileges and explicit allowlists where appropriate. Keep file rollback/undo for feasible workflows.
- If agent runtime fails, core launcher remains entirely functional. Test model unavailable, API errors, slow network, permission denial, canceled task, compromised/untrusted tool output, and unexpected subprocess termination.

## VII. Product capabilities: preserve, improve, differentiate

**P0 — correctness and exceptional basics:** installed app launch/switch, file/folder exact/prefix/fuzzy, file content search, keyboard navigation, search ranking, correct icons, low-latency summon, clipboard search and paste, browser history/bookmarks, settings/system commands, indexing management, and secure migration. Fix existing issues before adding more surface area.

**P1 — user-visible advantages:** excellent document/image/code preview, Open at match, smart filters (type/size/date/path), user-specific relevance and recents, efficient OCR search, Git repositories/commits/TODOs, copy-path/reveal/open-with, native window and desktop actions, calculator/unit conversion, quicklinks/snippets, media actions, optional semantic retrieval, plugin architecture with scoped permissions, portable install where feasible.

**P2 — opt-in power features:** Python-based agentic workflows, voice/selected-screen context with explicit consent, integrations using MCP/ACP, advanced automation templates, user-defined action pipelines, more languages, additional parsers, alternative deployment platforms *only if Windows quality is already stable*.

Do not copy Kensaku's UI or call features “world first” without research. Avoid indiscriminate scope expansion. Where a feature is difficult, ship a real constrained implementation with documented limitation instead of a fake toggle or dead-end settings page.

## VIII. Performance science: define “fastest” correctly

Publish methodology and comparable benchmarks. Distinguish **warm vs cold, index creation vs incremental maintenance, filesystem search vs content search, OCR *indexing* vs indexed OCR *retrieval*, and renderer latency vs engine latency**. Compare best reasonable configurations under equal workload and platform.

Initial **stretch targets, NOT assumed achievable or marketing facts**:

| Measurement | Investigate target | Conditions |
|---|---:|---|
| Hotkey → visible + input-ready | p95 < 50 ms | prewarmed launcher, Windows laptop |
| 100k warm filename search | p95 < 5 ms | representative names, end-to-end Rust query |
| 1M warm filename search | p95 < 15 ms | realistic names/paths, indexed |
| First visible results from input | p95 < 30 ms | UI+IPC+Rust+paint, measured |
| Indexed document content retrieval | p95 < 30 ms | fixed mixed corpus |
| Indexed OCR-text retrieval | p95 < 30 ms | *text already OCR'd and indexed* |
| Responsive interaction | 60 FPS baseline; target 120 FPS when possible | scrolling, selection, previews |
| Inactive Python agent processes | 0 | no user-invoked agent |
| Idle CPU | near 0 when truly idle | no indexing or pending tasks |

**Benchmark suites:**
- 10k, 100k, 1M name/path datasets plus actual user directories; exact, prefix, fuzzy, typo, CJK, Unicode, hidden, duplicated names, long path, and common query shapes.
- Fresh index, warm rescan unchanged, one-file edit, mass rename, interrupted index/restart, removable drive, disconnected network root, OneDrive placeholder, Defender enabled, 100% disk usage stress.
- Mixed corpus: office docs, PDFs, EPUB, code, thousands of screenshots/images, scanned documents, compressed/large files, malformed files, multi-language content. Measure index size per file and realistic peak RSS.
- OCR: character/word recall and exact-match rates, pages/images/sec, memory, CPU/GPU/NPU, language variance, first-run download, cold/warm model load.
- UI: press-hotkey → first pixel → focus → query → first visible result, p50/p95/p99, WebView2 child-process total memory, FPS/frame time, CPU and battery impact; type quickly while indexing and agent sidecar are active.
- Competitor evaluation on **same machine, OS, datasets, cache state, configured indexing depth**. Everything, Kensaku and other appropriate comparators may have different search scopes; state scope mismatch openly.

Create stable benchmark harnesses and golden-query relevance fixtures. Profile first, prioritize the highest-impact bottlenecks, verify the improvement, and include baseline-vs-new tables. If a stretch target fails, document measured performance and reason rather than “optimizing” the numbers.

## IX. Testing, security, CI and release discipline

- Rust unit tests for ranking, index updates, migration and IPC. Frontend unit/interaction tests with realistic datasets. Property/fuzz tests for query parser, Unicode boundaries, path handling, file extraction and state reconciliation. Regression tests for current GitHub issues.
- Windows end-to-end tests covering summon/focus, fast typing, selection, result open/copy, tray behavior, app full-window navigation, on/offline modes, first-run/upgrade onboarding, source configuration, settings persistence, OCR/model optionality and agent approval/cancellation.
- Accessibility: keyboard-only, focus indicators, screen reader semantics, high DPI, reduced motion, contrast. No focus trapping and no Enter action on stale rows.
- Security: no hardcoded keys; scrub historical credential issues; typed arguments rather than shell-string interpolation; document parser limits; archive bombs, symlinks/reparse loops, plugin containment, IPC authentication when applicable, credential redaction, privacy controls and data-export/delete pathways.
- Releases: reproducible CI, lockfile integrity, package validation, checksums, tested installer/uninstaller, updater signature/checksum validation, migration fixture tests, release notes, repair path, license compliance, optional signing strategy.
- Run actual Windows builds. For external tool limitations, say exactly what is not validated; do not fake screenshots or test reports. The original repository is Windows-first: a Linux-only successful build is not proof of completion.
- Commit in coherent, reviewable milestones. Avoid unreviewed mega-commits or pushing every small edit. Do not force-push, erase history, publish builds, post on social media, buy services or expose credentials without explicit authorization.

## X. Required execution sequence and gates

**Gate 0 — Reconnaissance:** working original-app build (or documented blocker), feature-parity matrix, risks, benchmark baseline, repository/architecture audit, UI reference inspection.

**Gate 1 — Architecture/contracts:** ADRs, owner map, repo structure, versioned IPC and data schemas, dependency graph, migration plan, security model, test matrix, benchmark harness skeleton. Interface leads sign off.

**Gate 2 — Fast engine:** functional Rust filename/content pipeline, incremental indexing, ranking, query cancellation, stability under updates, measured baselines and demonstrable gains. No UI regressions from refactoring.

**Gate 3 — Integrated launcher:** WebView2/Tauri UI derived from provided HTML; frameless quick search with keyboard focus, predictable result groups, footer hints only, real data, animation performance verification, no white flash, no fake badge.

**Gate 4 — Desktop app and onboarding:** welcome and optional name, true first-run configuration, indexed location modes, real progress, correct settings/health, upgrade migration, dark mode, accessibility and responsive layout.

**Gate 5 — Rich search and workflows:** reliable previews/open-at-match, OCR and document search, clipboard/browser/Git actions, plugin contracts, optional semantic retrieval; tests and measurements for each.

**Gate 6 — Optional Python agent:** documented framework decision, opt-in sidecar packaging, safe tool contracts, approvals, stream/cancel, model integrations, no baseline memory cost.

**Gate 7 — Release candidate:** full parity/security/performance/UX checklist, real Windows screenshots/video or screen capture, signed-off regression suite, installer/update/migration, docs and release notes. Triage defects and iterate until gate criteria pass.

At each gate, L0 must show: completed deliverables, proof (commands/test reports/screenshots), performance changes and known deficiencies, merged integration status, remaining risks, and the next dependency-critical work. Tasks may run concurrently **only when they cannot corrupt shared files or assumptions**.

## XI. Required final handoff (do not declare victory without these)

1. A working integrated Windows executable/installer and explicit instructions to run it.
2. Source code with modular Rust core, functioning Tauri/WebView2 frontend, optional isolated Python agent, and versioned bridges.
3. **Visual comparison:** supplied reference HTML versus final launcher, desktop home, onboarding, light/dark, index states, keyboard selection, at realistic DPI/resolutions.
4. **Feature parity and acceptance matrix:** previous functionality retained or explicitly documented, new functionality implemented, regressions tracked.
5. **Benchmark report:** exact hardware, OS, corpus, cache state, methodology, p50/p95/p99, memory/CPU/battery where measured, OCR accuracy and throughput, competitor scope, reproducible commands. Do not claim world record without proof.
6. Tests, CI workflows, threat model, user-data migration and rollback, signed/unsigned install caveats, known limitations, third-party licenses.
7. User-facing README and first-launch guide, a genuine demo showing real use, and a concise changelog.
8. Clear, truthful **what is complete / partially complete / unsupported / not tested** report.

### Final directive to the orchestrator

Treat this as a genuine product engineering assignment. Establish the agent hierarchy, inspect real code, take measurements, implement in small reviewable increments, integrate continuously, and ship tested software. **Never substitute a collection of sub-agent reports for a coherent runnable ProtonSearch v2.** Delegate aggressively but keep ownership centralized. The highest-level quality bar is simple: the user should press the shortcut, find what they want almost immediately, understand exactly what actions are available, and trust the tool not to waste time, resources, or their data.

**Begin with Gate 0 now. Do not begin by rewriting the UI or blindly pasting the supplied HTML into production.**


---

# APPENDIX A — EXACT SELF-CONTAINED HTML DESIGN REFERENCE

The code below is the full HTML source of `protonsearch-v2-ui-reference.html`, reproduced to support agents that accept only one text prompt. Save it as `protonsearch-v2-ui-reference.html` and open it in a browser before implementation. Treat it as design reference, not a production implementation.

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <meta name="theme-color" content="#e9ece9" />
  <title>ProtonSearch v2 — UI Design Reference</title>
  <style>
    /* DESIGN REFERENCE ONLY. Not connected to the filesystem or a search backend.
       Brand direction: warm neutrals, graphite typography, restrained deep blue,
       hairline separators, no gradients/glow/glassmorphism, no generic dashboard cards. */
    :root { color-scheme: light; --desktop:#e8ebe8; --panel:#fbfcfa; --ink:#202b31; --muted:#828e8b;
      --quiet:#a0aaa6; --line:#e0e5df; --blue:#285975; --blue-deep:#1e4158;
      --selected:#e7eef2; --selected-border:#d6e2e9; --soft:#f2f5f1; --sidebar:#f3f6f2;
      font-family:Inter,'Segoe UI Variable','Segoe UI',system-ui,sans-serif; }
    * {box-sizing:border-box} body { margin:0;background:var(--desktop);color:var(--ink);font-size:13px;letter-spacing:-.012em }
    button,input {font:inherit} button {cursor:pointer} button:focus-visible,input:focus-visible {outline:2px solid #5482a3;outline-offset:2px}
    .stage { min-height:100vh;padding:26px 22px 75px;background:var(--desktop)}
    .outer {max-width:1120px;margin:0 auto}
    .controlbar {display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:27px;flex-wrap:wrap}
    .overline {font-size:10px;font-weight:760;letter-spacing:.16em;color:#687873}
    .segments {display:flex;background:#dce2dc;padding:4px;gap:4px;border:1px solid #ced6ce;border-radius:9px}
    .segments button {background:transparent;border:0;border-radius:5px;padding:9px 14px;color:#6a7973;font-size:12px;font-weight:680;transition:background .14s,color .14s}
    .segments button[aria-selected=true]{background:white;color:#24363c;box-shadow:0 1px 3px #12251b0f}
    .logo {width:29px;height:29px;display:block;flex:none} .brand {display:flex;align-items:center;gap:10px;font-weight:780;letter-spacing:-.035em}
    .demo-label {color:#889490;font-size:11px;margin:18px auto 0;max-width:830px;text-align:center}
    .launcher {max-width:820px;margin:28px auto 0;background:var(--panel);border:1px solid #cdd5ce;border-radius:14px;
      overflow:hidden;box-shadow:0 24px 56px #243c2b1d,0 2px 5px #152c180b}
    /* Launcher intentionally has NO title bar or minimize/maximize/close controls. */
    .searchline{height:72px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:14px;padding:14px 20px}
    .searchline input{flex:1;min-width:0;border:0;background:transparent;outline:0!important;font-size:21px;font-weight:430;letter-spacing:-.04em;color:#203039}
    .searchline input::placeholder{color:#b4bfba}.count{font-size:11px;color:#9ca8a3;white-space:nowrap}
    .results{height:auto;max-height:365px;min-height:140px;overflow:auto;padding:10px 11px 13px;scrollbar-width:thin;scrollbar-color:#d9dfda transparent}
    .group-label{font-size:10px;letter-spacing:.12em;color:#87948f;font-weight:760;padding:14px 13px 8px}
    .result{background:transparent;border:1px solid transparent;border-radius:8px;padding:8px 12px;min-height:58px;
      display:flex;align-items:center;gap:12px;width:100%;text-align:left;transition:background .11s,border-color .11s}
    .result:hover{background:#f0f3f0}.result.selected{background:var(--selected);border-color:var(--selected-border)}
    .fileicon{flex:none;display:grid;place-items:center;width:35px;height:37px;border-radius:7px;border:1px solid #dfe3de;
      font-size:11px;font-weight:800;letter-spacing:-.04em;background:#e8ece7;color:#57716d}
    .fileicon.word{background:#e5eef5;color:#285e8b}.fileicon.folder{background:#eee9dc;color:#a57b3c}
    .fileicon.code{background:#e9eee8;color:#386d58}.fileicon.action{background:#e7e9eb;color:#436276}
    .resultbody{flex:1;min-width:0}.filename{font-size:13px;font-weight:680;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
    .filepath{font-size:11px;color:#929e99;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;margin-top:4px}
    .resultmeta{font-size:10px;color:#95a19c;white-space:nowrap}
    .footer{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap;
      min-height:44px;border-top:1px solid var(--line);padding:9px 16px;color:#87938e;font-size:10px}
    .leftfoot,.rightfoot{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.ready{width:7px;height:7px;border-radius:50%;background:#759887}
    kbd{font:650 10px inherit;display:inline-block;color:#63756b;background:#f0f3ef;border:1px solid #dbe2da;border-radius:4px;padding:2px 5px}
    .footlink{border:0;background:transparent;color:#356d88;font-size:10px;font-weight:750;padding:3px}
    .empty{padding:70px 15px;text-align:center;color:#8d9c95}
    .desktop-app{width:min(100%,1000px);margin:auto;min-height:595px;background:var(--panel);
      border:1px solid #d4dbd4;border-radius:12px;overflow:hidden;box-shadow:0 18px 45px #213b2815;display:grid;grid-template-columns:190px minmax(0,1fr)}
    .sidebar{background:var(--sidebar);border-right:1px solid var(--line);display:flex;flex-direction:column;gap:27px;padding:21px 13px}
    .sidebar .brand{font-size:12px;padding:0 6px}.sidebar-label{font-size:10px;font-weight:750;letter-spacing:.12em;color:#9aa69e;padding:0 11px;margin-bottom:8px}
    .navbtn{display:flex;align-items:center;gap:10px;width:100%;background:transparent;border:0;border-radius:7px;
      padding:11px 10px;color:#697770;text-align:left;font-size:12px;font-weight:600}
    .navbtn:hover{background:#e9eeea}.navbtn[aria-current=page]{background:#e0eae8;color:#234759;font-weight:740}
    .navglyph{display:grid;place-items:center;flex:none;width:19px;font-size:16px;font-weight:500}
    .userfoot{margin-top:auto;border-top:1px solid var(--line);padding:16px 6px 0;display:flex;align-items:center;gap:9px;color:#64766d;font-size:11px}
    .avatar{width:30px;height:30px;border-radius:8px;display:grid;place-items:center;background:#dae7e8;color:#245168;font-weight:780}
    .workspace{padding:36px 40px;min-width:0}.eyebrow{font-size:10px;letter-spacing:.14em;font-weight:770;color:#82948b;margin-bottom:14px}
    h1{font-size:clamp(26px,4vw,41px);letter-spacing:-.065em;line-height:1.11;font-weight:630;margin:0 0 9px}
    .lead{font-size:13px;color:#8c9a92;line-height:1.7;margin:0 0 25px}
    .bigsearch{display:flex;align-items:center;gap:10px;width:100%;background:white;border:1px solid #dce4db;border-radius:8px;
      color:#8a9991;text-align:left;padding:17px 16px;box-shadow:0 1px 3px #00000005;font-size:13px}
    .bigsearch span{flex:1}.arrow{font-size:15px;color:#597d8e}
    .columns{display:grid;grid-template-columns:minmax(0,1.25fr) minmax(0,.9fr);gap:38px;margin-top:37px}
    h2{font-size:11px;letter-spacing:.1em;color:#7c8d85;font-weight:780;margin:0 0 14px;text-transform:uppercase}
    .recent{display:flex;align-items:center;gap:11px;padding:12px 0;border-bottom:1px solid #e8ece7}
    .recent:last-of-type{border-bottom:0}.recent-icon{width:32px;height:33px;background:#ecf1ec;border:1px solid #e2e8e0;border-radius:6px;display:grid;place-items:center;font-size:11px;font-weight:750;color:#667b77}
    .recent-title{font-weight:660;font-size:12px}.recent-sub{font-size:11px;color:#9aa8a0;margin-top:4px}
    .statsnum{font-size:34px;font-weight:670;letter-spacing:-.075em}.statsub{color:#95a39c;font-size:11px;margin-top:1px}
    .thinprogress{margin:20px 0 11px;height:4px;border-radius:3px;background:#e5ebe5;overflow:hidden}.thinprogress i{display:block;background:#407188;width:83%;height:100%}
    .metric{display:flex;justify-content:space-between;gap:10px;border-bottom:1px solid #e9ede8;padding:11px 0;color:#708278;font-size:12px}.metric:last-child{border-bottom:0}
    .metric strong{font-weight:700;color:#385766}.section{margin-top:24px}.section:first-child{margin-top:0}
    .info-panel{background:#fff;border:1px solid #e2e8df;padding:19px 21px;border-radius:8px;margin-top:18px}
    .muted{font-size:12px;line-height:1.65;color:#8b9990}
    .smallbtn,.primarybtn{border-radius:6px;padding:10px 14px;font-size:12px;font-weight:720}
    .smallbtn{background:white;border:1px solid #d8e2d9;color:#365a69}.smallbtn:hover{background:#f0f4ef}
    .primarybtn{background:#234a61;color:#fff;border:1px solid #234a61}.primarybtn:hover{background:#335e75}
    .onboarding{width:min(100%,830px);min-height:540px;margin:auto;background:var(--panel);border:1px solid #d4dbd4;border-radius:12px;overflow:hidden;
      box-shadow:0 18px 45px #213b2815;display:grid;grid-template-columns:240px minmax(0,1fr)}
    .setup-side{background:#e7eee9;border-right:1px solid #d6e0d8;display:flex;flex-direction:column;padding:32px 25px}
    .steps{margin-top:65px}.step-indicator{display:flex;gap:10px;align-items:center;color:#92a198;font-size:12px;font-weight:620;padding:14px 0}
    .step-num{display:grid;place-items:center;flex:none;border:1px solid #cbd7cd;border-radius:50%;width:25px;height:25px;font-size:11px}
    .step-indicator.current{color:#2a5366}.step-indicator.current .step-num{background:#295a70;color:white;border-color:#295a70}
    .setup-main{padding:44px 43px;display:flex;flex-direction:column}.setup-main h1{font-size:32px}.setup-main .lead{max-width:420px}
    .fieldlabel{display:block;font-size:11px;color:#6f8678;font-weight:740;margin:0 0 8px}
    .field{width:100%;padding:13px 14px;border:1px solid #d5dfd6;border-radius:7px;background:white;font-size:13px;color:var(--ink)}
    .choices{display:grid;gap:9px}.choice{display:flex;align-items:center;gap:10px;border:1px solid #e0e7df;background:#fff;border-radius:7px;padding:12px 13px;font-size:12px;font-weight:650;cursor:pointer}
    .choice input{accent-color:#285c73;width:16px;height:16px}.choice small{margin-left:auto;color:#96a39b;font-size:10px;font-weight:400}
    .setup-bottom{margin-top:auto;padding-top:30px;display:flex;align-items:center;justify-content:space-between;gap:14px}
    .warning{font-size:10px;color:#82948a;line-height:1.7;margin-top:12px}
    @media(max-width:700px){.stage{padding:13px 8px 45px}.controlbar{gap:10px}.overline{display:none}
      .launcher{margin-top:12px}.searchline{padding:11px 13px;height:61px}.searchline input{font-size:16px}.results{max-height:350px}
      .resultmeta{display:none}.desktop-app{grid-template-columns:56px minmax(0,1fr);min-height:510px}
      .sidebar{padding:17px 6px}.sidebar .brand span,.sidebar-label,.navtext,.userfoot span:last-child{display:none}
      .sidebar .brand,.navbtn,.userfoot{justify-content:center}.sidebar .brand{padding:0}.navbtn{padding:11px 0}.workspace{padding:31px 18px}
      .columns{grid-template-columns:1fr;gap:24px;margin-top:26px}.onboarding{display:block}.setup-side{display:none}.setup-main{min-height:520px;padding:32px 22px}
      .rightfoot{gap:6px}.logo{width:26px;height:26px}}
    @media(prefers-reduced-motion:reduce){*{transition:none!important;animation:none!important;scroll-behavior:auto!important}}
  </style>
</head>
<body>
  <main class="stage"><div class="outer">
    <div class="controlbar"><div class="overline">PROTONSEARCH / DESIGN REFERENCE 02</div>
      <div class="segments" role="tablist" aria-label="Prototype views">
        <button data-view="launcher" role="tab" aria-selected="true">Launcher</button>
        <button data-view="desktop" role="tab" aria-selected="false">Desktop app</button>
        <button data-view="onboarding" role="tab" aria-selected="false">Onboarding</button>
      </div>
    </div>
    <div id="mount" aria-live="off"></div>
    <p class="demo-label">Functional local UI reference: data and status are illustrative. No filesystem, Python agent, or OS action is connected.</p>
  </div></main>
  <script>
    // Plain JS ensures this reference opens without a build system. Production UI: React + TypeScript.
    const logo = `<svg class="logo" viewBox="0 0 64 64" fill="none" aria-label="ProtonSearch P mark"><path d="M21.5 5.5h19.8c12.7 0 19.2 8.4 16.2 19.3-2.7 10.3-10.8 16.3-22.9 16.3h-7L21.4 56c-1.7 5.1-6 6.9-10.9 5.2-3.2-1.1-4.9-4-3.9-7.7L16.5 10c.7-2.7 2.3-4.5 5-4.5Z" fill="#102448"/><path d="M21.5 5.5h19.8c10.4 0 16.4 5.2 16.7 14.1-9.1-.7-18.5 1.2-27.7 5.3L18.5 41 6.6 53.5 16.5 10c.7-2.7 2.3-4.5 5-4.5Z" fill="#1d5189"/><path d="M34 18h7.3c4.7 0 5.8 2.9 4.8 6.2-1 3.9-4.6 6.2-10.6 6.2h-4.2L34 18Z" fill="#fafbf9"/></svg>`;
    const data = [
      {kind:'word',symbol:'W',category:'DOCUMENTS',name:'Architecture notes.docx',path:'Documents / Projects / Nova',meta:'Yesterday'},
      {kind:'code',symbol:'MD',category:'DOCUMENTS',name:'search-architecture.md',path:'Projects / ProtonSearch / docs',meta:'Today'},
      {kind:'folder',symbol:'▰',category:'FOLDERS',name:'Architecture',path:'Projects / ProtonSearch',meta:'24 items'},
      {kind:'word',symbol:'PDF',category:'DOCUMENTS',name:'Systems and ranking.pdf',path:'Documents / Research',meta:'2 days ago'},
      {kind:'action',symbol:'VS',category:'APPLICATIONS',name:'Visual Studio Code',path:'Installed app',meta:'App'},
      {kind:'action',symbol:'↗',category:'ACTIONS',name:'Open display settings',path:'Windows / Settings',meta:'Action'},
      {kind:'code',symbol:'TXT',category:'CLIPBOARD',name:'Indexing benchmark notes',path:'Copied recently',meta:'Clipboard'},
    ];
    let view='launcher', section='Home', firstName='Alex', currentStep=0, search='architecture', selected=0;
    let locations = new Set(['Documents','Desktop','Downloads']), options={ocr:true,content:true,agent:false};
    const el=(s)=>document.querySelector(s);
    const clean=(s)=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    function chooseView(next){view=next;document.querySelectorAll('[data-view]').forEach(b=>b.setAttribute('aria-selected',String(b.dataset.view===view)));render()}
    document.querySelectorAll('[data-view]').forEach(b=>b.addEventListener('click',()=>chooseView(b.dataset.view)));
    function results(){const terms=search.trim().toLowerCase().split(/\s+/).filter(Boolean);return data.map((x,i)=>({...x,priority:i,score:terms.reduce((n,t)=>n+(x.name.toLowerCase().includes(t)?10:x.path.toLowerCase().includes(t)?3:-10),0)})).filter(x=>!terms.length||x.score>=0).sort((a,b)=>b.score-a.score||a.priority-b.priority)}
    function paintResults(){const rows=results(), target=el('#list');if(!target)return;selected=Math.max(0,Math.min(selected,rows.length-1));el('#match-count').textContent=rows.length+' matches';let group='';target.innerHTML=rows.length?rows.map((r,i)=>{const h=group!==r.category?`<div class="group-label">${clean(r.category)}</div>`:'';group=r.category;return `${h}<button class="result ${selected===i?'selected':''}" data-row="${i}" aria-selected="${selected===i}"><span class="fileicon ${r.kind}">${clean(r.symbol)}</span><span class="resultbody"><span class="filename" style="display:block">${clean(r.name)}</span><span class="filepath" style="display:block">${clean(r.path)}</span></span><span class="resultmeta">${clean(r.meta)}</span></button>`}).join(''):'<div class="empty">No matches in this sample. Try “architecture”, “Code”, or “display”.</div>';target.querySelectorAll('[data-row]').forEach(b=>b.onclick=()=>{selected=Number(b.dataset.row);search=rows[selected].name;el('#searchinput').value=search;selected=0;paintResults()})}
    function launcher(){el('#mount').innerHTML=`<div class="launcher"><div class="searchline">${logo}<input id="searchinput" autocomplete="off" spellcheck="false" aria-label="Search your computer" placeholder="Search files, apps, and actions…" value="${clean(search)}"/><span class="count" id="match-count"></span></div><div class="results" id="list"></div><div class="footer"><div class="leftfoot"><span class="ready"></span>Index ready <span>·</span> <button id="openapp" class="footlink">Open ProtonSearch</button></div><div class="rightfoot"><span><kbd>↑ ↓</kbd> Navigate</span><span><kbd>Enter</kbd> Open</span><span><kbd>Esc</kbd> Clear</span></div></div></div>`;
      el('#searchinput').oninput=e=>{search=e.target.value;selected=0;paintResults()};el('#searchinput').onkeydown=e=>{const n=results().length;if(['ArrowDown','ArrowUp'].includes(e.key)){e.preventDefault();selected=n?(selected+(e.key==='ArrowDown'?1:-1)+n)%n:0;paintResults();el('.result.selected')?.scrollIntoView({block:'nearest'})}else if(e.key==='Escape'){search='';selected=0;e.target.value='';paintResults()}else if(e.key==='Enter'){e.preventDefault();const r=results()[selected];if(r){el('#searchinput').value=r.name;search=r.name;selected=0;paintResults()}}};el('#openapp').onclick=()=>chooseView('desktop');paintResults()}
    const nav=[['Home','⌂'],['Search & indexes','⌕'],['Activity','◷'],['Workflows','↗'],['Preferences','⚙']];
    function desktop(){el('#mount').innerHTML=`<div class="desktop-app"><aside class="sidebar"><div class="brand">${logo}<span>ProtonSearch</span></div><div><div class="sidebar-label">WORKSPACE</div>${nav.map(([n,i])=>`<button class="navbtn" data-nav="${clean(n)}" ${n===section?'aria-current="page"':''}><span class="navglyph">${i}</span><span class="navtext">${clean(n)}</span></button>`).join('')}</div><div class="userfoot"><span class="avatar">${clean(firstName[0]?.toUpperCase()||'U')}</span><span>${clean(firstName)}</span></div></aside><section class="workspace" id="workspace"></section></div>`;document.querySelectorAll('[data-nav]').forEach(b=>b.onclick=()=>{section=b.dataset.nav;desktop()});const w=el('#workspace');
      if(section==='Home'){const h=new Date().getHours(),g=h<12?'Good morning':h<17?'Good afternoon':'Good evening';w.innerHTML=`<div class="eyebrow">YOUR WORKSPACE / HOME</div><h1>${g}, ${clean(firstName)}.</h1><p class="lead">Everything on your computer, within reach.</p><button class="bigsearch" id="openlauncher">⌕ <span>Find a file, open an app, or run an action…</span><span class="arrow">↗</span></button><div class="columns"><div><h2>Recently accessed</h2>${[['MD','Search architecture.md','Projects / ProtonSearch'],['PDF','Performance baseline.pdf','Documents / Research'],['▰','Product concepts','Projects / Design']].map(r=>`<div class="recent"><span class="recent-icon">${r[0]}</span><span style="flex:1;min-width:0"><div class="recent-title">${r[1]}</div><div class="recent-sub">${r[2]}</div></span><span style="color:#bac5be">↗</span></div>`).join('')}<button class="smallbtn" id="openindexes" style="margin-top:22px">Manage sources</button></div><div><h2>Your index</h2><div class="statsnum">124,318</div><div class="statsub">Locally indexed items (sample)</div><div class="thinprogress"><i></i></div><div class="muted" style="font-size:11px">Sample: background content enrichment</div><div style="margin-top:18px">${[['Documents','42,018'],['Applications','126'],['Images & OCR','8,440'],['Other files','73,734']].map(r=>`<div class="metric"><span>${r[0]}</span><strong>${r[1]}</strong></div>`).join('')}</div></div></div>`;el('#openlauncher').onclick=()=>chooseView('launcher');el('#openindexes').onclick=()=>{section='Search & indexes';desktop()};return}
      const pages={
        'Search & indexes':['Search & indexes','Choose what ProtonSearch can find, without blocking the fast filename index.',[['Indexed locations','Documents, Downloads, Desktop, Pictures'],['Index mode','File names now; content and OCR asynchronously'],['Index health','Ready (illustrative)']]],
        'Activity':['Your activity','A local history of recent searches, launches, and optional workflows.',[['Search','Architecture notes'],['Recent app','Visual Studio Code'],['History management','Pause and clear at any time']]],
        'Workflows':['Workflows','Native commands respond instantly; the Python agent sidecar is optional and starts only when needed.',[['Native commands','Window management, clipboard, quick actions'],['Agent runtime','Not running'],['Permissions','Explicit approvals for sensitive actions']]],
        'Preferences':['Make it yours','Customize behavior, appearance and privacy independently of search performance.',[['Launcher shortcut','Alt + Space (customizable)'],['Theme','System light / dark'],['Background indexing','Resource-aware'],['Privacy','Local-first and opt-in integrations']]],
      };const [title,sub,rows]=pages[section];w.innerHTML=`<div class="eyebrow">WORKSPACE / ${clean(section.toUpperCase())}</div><h1>${clean(title)}</h1><p class="lead">${clean(sub)}</p><div class="info-panel"><h2>Configuration</h2>${rows.map(r=>`<div class="metric"><span>${clean(r[0])}</span><strong>${clean(r[1])}</strong></div>`).join('')}</div>${section==='Preferences'?'<button class="smallbtn" id="setupagain" style="margin-top:20px">Preview onboarding</button>':''}`;if(el('#setupagain'))el('#setupagain').onclick=()=>{currentStep=0;chooseView('onboarding')};}
    const stepNames=['Welcome','Choose locations','Search depth','Ready'];
    function onboarding(){const titles=['Find anything, without the friction.','Choose where to search.','Decide how deep to index.','Ready when you are.'];const descriptions=['A fast, private Windows search experience built around your work. We only need a few preferences to get started.','Choose folders to index. ProtonSearch can begin with filenames and enrich results in the background.','Basic filename search never needs AI or an account. Choose optional content search and OCR; model downloads remain opt-in.','Your launcher can work while more advanced indexing runs. You can change everything later.'];el('#mount').innerHTML=`<div class="onboarding"><aside class="setup-side"><div class="brand">${logo}<span>ProtonSearch</span></div><div class="steps">${stepNames.map((n,i)=>`<div class="step-indicator ${i===currentStep?'current':''}"><div class="step-num">${i+1}</div><span>${n}</span></div>`).join('')}</div><div class="muted" style="margin-top:auto;font-size:11px">No account required.<br />Your files remain on your PC.</div></aside><section class="setup-main"><div class="eyebrow">GETTING STARTED / 0${currentStep+1} OF 04</div><h1>${titles[currentStep]}</h1><p class="lead">${descriptions[currentStep]}</p><div id="stepcontents"></div><div class="setup-bottom"><button class="smallbtn" id="backstep">${currentStep===0?'Skip for now':'Back'}</button><button class="primarybtn" id="nextstep">${currentStep===3?'Open ProtonSearch':'Continue →'}</button></div></section></div>`;const target=el('#stepcontents');
      if(currentStep===0)target.innerHTML=`<label for="personname" class="fieldlabel">WHAT SHOULD WE CALL YOU?</label><input id="personname" class="field" placeholder="Your first name" value="${clean(firstName==='Alex'?'':firstName)}"/><div class="warning">Only used for the welcome greeting in your local app.</div>`;
      if(currentStep===1)target.innerHTML=`<h2>Suggested locations</h2><div class="choices">${[['Documents','Notes and documents'],['Desktop','Files close at hand'],['Downloads','Recent downloads'],['Pictures','Screenshots and images']].map(([n,d])=>`<label class="choice"><input type="checkbox" value="${n}" ${locations.has(n)?'checked':''}/><span>${n}</span><small>${d}</small></label>`).join('')}</div>`;
      if(currentStep===2)target.innerHTML=`<h2>Indexing capabilities</h2><div class="choices"><label class="choice"><input type="checkbox" checked disabled/><span>Instant filenames</span><small>Always on</small></label><label class="choice"><input type="checkbox" data-option="content" ${options.content?'checked':''}/><span>Document and code content</span><small>Optional</small></label><label class="choice"><input type="checkbox" data-option="ocr" ${options.ocr?'checked':''}/><span>Screenshot OCR</span><small>Optional</small></label><label class="choice"><input type="checkbox" data-option="agent" ${options.agent?'checked':''}/><span>Configure AI later</span><small>No automatic downloads</small></label></div><p class="warning">Content and OCR extraction run in bounded background jobs, independently of the launcher.</p>`;
      if(currentStep===3)target.innerHTML=`<div class="info-panel" style="margin-top:0"><h2>Your setup</h2><div class="metric"><span>Included locations</span><strong>${locations.size}</strong></div><div class="metric"><span>Filename search</span><strong>Always available</strong></div><div class="metric"><span>Document content</span><strong>${options.content?'Enabled':'Off'}</strong></div><div class="metric"><span>OCR search</span><strong>${options.ocr?'Enabled':'Off'}</strong></div><div class="metric"><span>Agent framework</span><strong>${options.agent?'Configure later':'Disabled'}</strong></div></div>`;
      el('#backstep').onclick=()=>{if(currentStep===0)chooseView('desktop');else{currentStep--;onboarding()}};el('#nextstep').onclick=()=>{if(currentStep===0)firstName=el('#personname').value.trim()||'there';if(currentStep===1)locations=new Set([...document.querySelectorAll('#stepcontents input:checked')].map(x=>x.value));if(currentStep===2)document.querySelectorAll('[data-option]').forEach(x=>options[x.dataset.option]=x.checked);if(currentStep===3){section='Home';chooseView('desktop')}else{currentStep++;onboarding()}};
    }
    function render(){if(view==='launcher')launcher();else if(view==='desktop')desktop();else onboarding()}
    render();
  </script>
</body>
</html>

```
