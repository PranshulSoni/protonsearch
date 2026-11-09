# ProtonSearch v2 — Preview P0 Reproduction & Pipeline Audit (Gate 0 / task-29)

- **Task:** task-29 (G0-A) — reproduce the user P0 "file/text previews do not show", map the full preview flow (engine → content → IPC → UI), rank root causes, size the Gate-2 repair.
- **Owner:** engine-core (L1-A Rust Core / Windows Platform lead). **Scope of this pass:** read-only audit + evidence (no product fixes).
- **Base revision:** working tree based on commit \`03633c7\` ("v10 G0/G1 scaffold: … preview settings+IPC contract …"), branch \`v2-phoenix\`. Debug app build measured: \`target/debug/ProtonSearch.exe\` (2026-11-10 02:09 local). UI bundle served from \`apps/desktop/dist\` (01:58).
- **Status:** reproduction complete at the payload level; fixture matrix and real-app screenshots captured (see §6). No fixes implemented.
- **Artifacts:** \`docs/qa/preview-audit/fixtures/\` (11 deterministic fixtures), \`.../payloads/\` (extraction matrix + engine search + app IPC payloads), \`.../screenshots/\`, \`.../scripts/\` (fixture generator, Rust audit harness, CDP capture driver).

## 1. Verdict (TL;DR)

The preview pipeline does not exist as a pipeline. Today the launcher can only render a **snippet that a content-FTS hit happened to carry in its own search response**. There is **no preview request/response in \`ps-ipc\`**, **no preview command in the Tauri shell**, and **no preview code in \`ps-core\`**. Consequently:

- Every **filename/file result** (the normal case: the user types a file name) carries \`meta.snippet = null\` → the preview pane renders the literal fallback **"Preview not available — press Enter to open."**
- **OCR results** also render the fallback: \`ocr_result()\` drops the OCR text even though \`db.ocr_search()\` returns it.
- **Images and scanned PDFs** have no preview path at all (no thumbnail field in IPC; OCR pipeline only accepts image extensions, never PDF pages).
- Only **content_match** rows preview today, and only when the query happens to match indexed document text (a full-text search), not when the user is searching for a file.

This matches the user's report: previews "do not show" because for file/text results there is nothing to show in the current contract.

## 2. Reproduction method

1. **Fixtures** (deterministic, committed): \`docs/qa/preview-audit/scripts/make_fixtures.py\` writes 11 files covering every required family:
   \`notes.txt\`, \`readme.md\`, \`sample.rs\`, \`report.pdf\` (real text layer), \`scanned.pdf\` (image-only), \`report.docx\`, \`sheet.xlsx\`, \`diagram.png\` (rendered text), \`blob.bin\`, \`unknown.xyz\`, \`large.txt\` (~1.2 MB).
2. **Extraction matrix** (Rust audit harness, uses the real \`ps-content\` extractor + \`ps-ocr\`):
   \`cargo run --manifest-path target/preview-audit-harness/Cargo.toml -- extract docs/qa/preview-audit/fixtures docs/qa/preview-audit/payloads/extraction-matrix.json\`
3. **Engine/IPC payload dump** (real \`Engine::open\` + background pipeline over the fixture root, then \`Engine::search\` responses serialized as \`ps_ipc::SearchResponse\`):
   \`cargo run --manifest-path target/preview-audit-harness/Cargo.toml -- build docs/qa/preview-audit/fixtures docs/qa/preview-audit/data docs/qa/preview-audit/payloads/engine-search-payloads.json\`
4. **Real app capture** (real Tauri/WebView2 app over CDP, fixture data dir, launcher preview opened with Ctrl+Space, IPC payloads dumped from inside the WebView, screenshots per scenario):
   \`node docs/qa/preview-audit/scripts/preview-capture.mjs\`
   App env: \`PROTONSEARCH_DATA_DIR=docs/qa/preview-audit/appdata\`, \`PROTONSEARCH_CDP_PORT=9333\`; engine ready with 11 filename entries / 8 content docs / 1 OCR doc before capture.

## 3. Format support matrix (measured)

| Format | Fixture | Extract (\`ps-content\`) | Search row kind | \`meta.snippet\` | Preview today | Required fallback (Gate 2) |
|---|---|---|---|---|---|---|
| txt | notes.txt | OK, 192 chars | file (filename query) | **null** | fallback note | text preview, first 20 lines + highlight |
| markdown | readme.md | OK, 203 chars | file / content_match | null / snippet | fallback for file row; snippet only on content query | rendered text preview |
| code | sample.rs | OK, 233 chars | file / content_match | null / snippet | same | mono text preview + line numbers |
| PDF (text) | report.pdf | OK, 83 chars | file / content_match | null / snippet | fallback for file row; snippet on content query | text preview + first page (optional render) |
| PDF (scanned) | scanned.pdf | OK, **0 chars** (image-only) | content_match with **snippet ""** + file | **""** | fallback note; label misleading ("In document") | "No text layer — OCR" path / page render |
| DOCX | report.docx | OK, 133 chars | file / content_match | null / snippet | fallback for file row | text preview |
| XLSX | sheet.xlsx | OK, 56 chars | file / content_match | null / snippet | fallback for file row | sheet text preview (cells) |
| image | diagram.png | no text extractor (\`Ok(None)\`); **OCR OK** ("IMAGE-OCR-PROBE-2026 …") | file; ocr_text for OCR query | **null** | fallback note; no thumbnail | image preview + OCR text |
| binary | blob.bin | unsupported (\`Ok(None)\`) | file | null | fallback note | metadata + explicit "no preview" |
| unsupported ext | unknown.xyz | unsupported | file | null | fallback note | metadata + explicit "no preview" |
| large text | large.txt | OK, truncated at 499,322 chars (512 KB cap) | content_match | snippet | content preview (truncated) | preview with "truncated" badge |

Evidence: \`payloads/extraction-matrix.json\`; engine payload samples in §5.

## 4. Pipeline map (where it breaks)

\`\`\`text
UI ResultPreview.tsx / previewClient.ts        (apps/desktop/src/launcher)
  body = meta.snippet ? snippet : kind-specific fallback
  file/document/ocr_text/content_match(empty) -> "Preview not available — press Enter to open."
        ^
        | SearchResponse.results[].meta (label/size/modified/itemCount/line/snippet/sourceLabel)
        |  <-- ps-ipc ResultMeta is the ONLY preview channel; no PreviewRequest/PreviewPayload
        |
Tauri shell apps/desktop/src-tauri/src/main.rs
  commands: search, open_result, get_status, get_settings, update_settings, get_activity,
            get_onboarding, complete_onboarding, index_command, set_launcher_geometry,
            hide_launcher, show_launcher, show_main, quit_app, app_info, set_hotkey,
            confirm_native_action, agent_* ...  (NO preview command)
        ^
        |
ps-core Engine::search()
  tier 1 filename -> file_result(): meta = { size, modified }          <-- no content, no snippet
  tier 1 content  -> content_result(): meta.snippet + meta.line        <-- only previewable body
  tier 2 OCR      -> ocr_result(): meta = { line, label }              <-- text dropped (bug)
        ^
        |
ps-content ContentIndex (FTS5 documents_fts: 8 fixture docs) + ps-content::extract_text
ps-ocr extract_image (Windows.Media.Ocr) -> db.ocr_documents (1 fixture doc)
ps-index filename index (11 fixture entries)
\`\`\`

Key code locations (working tree at audit time):
- \`crates/ps-ipc/src/lib.rs\` — \`ResultMeta { label, size, modified, item_count, line, snippet, source_label }\`; no preview DTOs.
- \`crates/ps-core/src/engine.rs\` — \`file_result()\` (~L1868) sets no snippet; \`content_result()\` (~L2313) sets snippet+line; \`ocr_result()\` (~L2345) sets only label/line; content tier gated by \`settings.indexing.content_enabled && content_scope_allows(scope)\` (~L471); OCR tier by \`settings.indexing.ocr_enabled\` (~L559).
- \`apps/desktop/src-tauri/src/main.rs\` — command surface (no preview).
- \`apps/desktop/src/launcher/ResultPreview.tsx\` — \`previewBody()\` fallback for \`file/document/content_match/ocr_text/…\`; default-off Ctrl+Space toggle per \`docs/design/SCREENS.md §2.6\`.
- \`apps/desktop/src/launcher/previewClient.ts\` (uncommitted, ui-launcher's Gate-2 seam) — provider interface \`PreviewPayload\` exists, but the default provider is metadata-only (\`buildMetadataPreview\`), i.e. the same fallback today; it is the intended wiring point for G2-A.

## 5. Payload dumps (abridged, redacted paths kept)

Filename query \`report\` (real \`Engine::search\`, limit 20):

\`\`\`json
{"kind":"file","title":"report.docx","path":"C:\\…\\fixtures\\report.docx","meta":{"size":36690,"modified":…}}
{"kind":"file","title":"report.pdf","path":"C:\\…\\fixtures\\report.pdf","meta":{"size":657,"modified":…}}
{"kind":"content_match","title":"report","path":"C:\\…\\fixtures\\report.docx",
 "meta":{"label":"In document","snippet":"Preview Audit DOCX\nThis Word document carries the token KOALA-DOCX-2026…","line":null}}
\`\`\`
→ file rows have no snippet (preview fallback); content rows do.

Content-token query \`WALRUS-PDF-2026\`:

\`\`\`json
{"kind":"content_match","title":"report","path":"C:\\…\\fixtures\\report.pdf",
 "meta":{"label":"line 3","line":3,"snippet":"WALRUS-PDF-2026 preview audit sample document text…"}}
\`\`\`

OCR query \`IMAGE-OCR-PROBE-2026\`:

\`\`\`json
{"kind":"ocr_text","title":"IMAGE-OCR-PROBE-2026 screenshot preview audit",
 "path":"C:\\…\\fixtures\\diagram.png",
 "meta":{"label":"Text in screenshot","line":null,"snippet":null}}
\`\`\`
→ OCR text exists (title is its first line) but is not delivered as preview text.

Scanned PDF query \`scanned\`:

\`\`\`json
{"kind":"content_match","title":"scanned","path":"C:\\…\\fixtures\\scanned.pdf",
 "meta":{"label":"In document","snippet":"","line":null}}
\`\`\`
→ empty snippet is falsy in the UI → fallback; label claims "In document" while there is no text.

\`ps-ipc\` \`ResultMeta\` (the only preview-ish fields in the whole contract):

\`\`\`rust
pub struct ResultMeta {
    pub label: Option<String>, pub size: Option<u64>, pub modified: Option<i64>,
    pub item_count: Option<u64>, pub line: Option<u32>, pub snippet: Option<String>,
    pub source_label: Option<String>,
}
\`\`\`

## 6. Real-app evidence (screenshots)

Captured with the CDP driver against the real app + fixture data dir (see \`screenshots/preview-capture-report.json\` for the machine-readable report incl. IPC payloads and timings):
- \`screenshots/00-launcher-empty.png\` — launcher empty state.
- \`screenshots/preview-file-*.png\` — preview pane after selecting a file result (txt/pdf/docx/xlsx/image/binary): shows metadata facts + **"Preview not available — press Enter to open."**
- \`screenshots/preview-content-*.png\` — content-token queries: preview shows the extracted snippet (the only working path today).
- \`screenshots/preview-ocr-image.png\` — OCR result: fallback despite available OCR text.
- \`screenshots/preview-scanned-pdf.png\` — scanned PDF: fallback + misleading "In document" label.

(Note: captures were taken on a shared multi-agent machine; the app instance is isolated to the audit data dir and CDP port 9333. One stray debug instance holding \`.devdata-live\` was stopped during capture; it was relaunchable.)

## 7. Root causes (ranked)

| # | Sev | Where | Root cause | Evidence |
|---|---|---|---|---|
| R1 | **P0** | ps-ipc + ps-core + Tauri | **No preview contract/pipeline at all.** \`ps-ipc\` has no PreviewRequest/PreviewPayload; Tauri has no preview command; ps-core has no preview API. Previews exist only as content-FTS snippets attached to content_match rows, so file/filename results always fall back. | §4/§5; command list; payload dumps |
| R2 | **P0** | ps-core \`ocr_result()\` | **OCR text is dropped from results**: \`db.ocr_search()\` returns \`OcrHit.text\` but the result sets no \`meta.snippet\`, so OCR rows always fall back even though text is available. | payload above |
| R3 | P1 | ps-core/ps-content/ps-ocr | **No image or scanned-PDF preview path**: images are filename-only (no thumbnail/IPC field); image-only PDFs extract to empty text, and the OCR pipeline only accepts image extensions, so PDF pages are never OCR'd. Empty content row is labelled "In document". | matrix + payloads |
| R4 | P1 | UI | **No renderer for text/code/document/image/PDF bodies**: \`ResultPreview\`/\`previewClient\` only render metadata + snippet; the fallback copy is the body for file/document/ocr kinds. Preview is default-off (Ctrl+Space) with no discoverability affordance, compounding the user perception. | ResultPreview.tsx, SCREENS §2.6 |
| R5 | P2 | ps-core \`root_modes()/mode_for()\` | **Forward-slash roots silently disable content/OCR indexing** (only the trailing separator is normalized; \`mode_for\` then returns 0). Reproduced: contentDocuments 0 with a \`docs/.../fixtures\` root vs 8 with the native path. Portable/imported settings can hit this. | harness run, §2 |
| R6 | P2 | ps-core scope rules | **FILES scope includes content_match rows** (\`content_scope_allows\` includes \`Files\`), inconsistent with the strict FILES/FOLDERS semantics and confusing for preview expectations. | payload: scope Files returned content_match rows |
| R7 | P2 | ps-content | **Extraction robustness**: the live v2 DB already has 12 \`content_errors\`; \`pdf-extract\` panics on several real PDFs (contained + recorded), and there is no preview fallback messaging for these files. | app warnings; DB counts |
| R8 | P3 (suspected) | UI | **Stale preview row** if Ctrl+Space is dispatched in the same frame as ArrowDown (synthetic CDP observation; needs real-user verification before it is treated as a bug). | capture attempt 1 |

## 8. Gate-2 repair plan (backend + IPC + UI)

Sized for Gate 2; no work started here.

**G2-A — Preview IPC contract v2 (0.5 d, owner L1-A + L0 for contract)**
\`\`\`rust
pub struct PreviewRequest { pub path: String, pub kind: SourceKind, pub query: Option<String>,
                            pub line: Option<u32>, pub max_bytes: Option<usize> }
pub enum PreviewStatus { Ready, Unsupported { reason: String }, Error { reason: String } }
pub struct PreviewPayload {
  pub status: PreviewStatus,
  pub text: Option<String>,            // bounded, e.g. first 20-40 lines / 64 KB
  pub total_lines: Option<u32>, pub truncated: bool,
  pub match_ranges: Vec<TextRange>,    // UTF-16 ranges for highlighting
  pub line: Option<u32>,
  pub image: Option<PreviewImage>,     // png path or bytes + width/height
  pub ocr: Option<PreviewOcr>,         // text + confidence
  pub facts: Vec<PreviewFact>,         // size/modified/kind/author/sheet…
}
\`\`\`
PROTOCOL_VERSION bump + TS mirror + roundtrip tests + redaction/limits rules.

**G2-B — Engine preview service (2–3 d, L1-A)**
\`Engine::preview(req) -> PreviewPayload\`: bounded, cancellable, never on the search thread; LRU cache keyed \`(path, mtime, size, query)\`; byte/time caps.
- text/md/code/json → \`ps_content::extract_text\` → first N lines + match ranges (reuse existing extractor, already working).
- pdf/docx/pptx/xlsx/epub → \`extract_text\` → bounded text.
- png/jpg/bmp/webp/tiff → image payload (cached PNG/thumbnail) + \`ps_ocr::extract_image\` text.
- image-only/scanned PDF → page rasterisation (WinRT PDF) + OCR behind a capability flag; explicit "no text layer" fallback when unavailable.
- binary/unsupported → \`Unsupported{reason}\` + metadata facts (honest fallback).

**G2-C — OCR result snippet fix (0.5 d, L1-A)** — \`ocr_result()\` includes truncated \`hit.text\` as \`meta.snippet\` (+ line when available); regression test.

**G2-D — UI preview rendering (2 d, L1-B/ui-launcher)** — \`ResultPreview\` consumes \`PreviewPayload\`: mono text/code with highlighted ranges and line numbers; image preview via asset URL; PDF page image; facts card; loading skeleton; error/unsupported copy; abort stale requests; auto-preview preference plus Ctrl+Space; UI tests + CDP capture regression.

**G2-E — Hardening (1 d)** — separator-insensitive root normalization (R5); FILES-scope/content decision (R6); extraction-error surfacing (R7); verify R8; perf budgets (text p95 < 50 ms, image/PDF page < 300 ms, cache hit < 5 ms), memory caps, security review (no shell, bounded decode, path validation).

**Acceptance for G2:** the full fixture matrix (§3) produces the expected preview body for every row, with payload dumps + screenshots in this folder; \`cargo test -p ps-core -p ps-content -p ps-ipc\` green; UI tests green; \`preview-capture.mjs\` regression run attached.

## 9. Limitations / unverified

- This is a Gate-0 audit on a shared multi-agent machine; the app binary used was the debug build current at audit time (02:09) while other agents were rebuilding the tree. Payload evidence is from the real engine; UI screenshots are from the real app but a rebuild may change chrome, not the preview logic.
- Real \`.devdata-live\` data was used only for DB-level facts (89,813 files / 16,848 docs / 373 OCR docs / 12 content_errors) and a one-off status check; captures were isolated to the fixture data dir to avoid mutating live dev data.
- R8 (stale preview row) is a synthetic-input observation, not confirmed by real-user interaction.
- No fixes were implemented in this pass (per task-29).
