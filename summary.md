Audit complete — no code edits made

I audited the Rust/Slint `omnisearch` codebase with extra focus on the newly added clipboard image OCR paths. I also used two independent review passes to cross-check the findings.

### Validation run

- `taskkill //F //IM omnisearch.exe`
  - Result: process not found, so the app was not running.
- `cargo check --manifest-path omnisearch/Cargo.toml`
  - Result: passed, warnings only.
- `cargo test --manifest-path omnisearch/Cargo.toml`
  - Result: passed, warnings only.
- `git --no-pager diff --stat`
  - Result: no tracked code changes.
- Language-server diagnostics still report 5 analyzer errors in `src/main.rs`, but `cargo check` passes. These appear to be analyzer/type-resolution false positives around normal Rust casts, not compiler blockers.

Current untracked files exist in the repo root: `6000`, `6000,+`, `a`, `phase4_ai_commands.md`, `project_audit_report.md`, `return`. I did not modify or remove them.

---

# Bottom line

The clipboard OCR feature is **not production-ready yet**. The app compiles and tests pass, but the new clipboard image OCR has multiple real runtime/data-flow bugs that can make it look like OCR “doesn’t work.”

The strongest root causes are:

1. Fresh databases can create `clipboard_history` **without `ocr_text`**, breaking OCR search.
2. Imported Windows clipboard-history image OCR runs on threads **without COM/WinRT initialization**.
3. Live clipboard capture treats clipboard text as higher priority and skips image capture when a copied image also exposes text/HTML/URL formats.
4. The homepage “Read Clipboard Image Text” action only uses WinRT `StandardDataFormats::Bitmap`, while the working save path supports classic Win32 formats like `CF_DIB`, `CF_DIBV5`, and `CF_BITMAP`.
5. OCR text preview can panic on Unicode due to byte slicing.

---

# Strengths

- `ocr_image_file()` has a reasonable WinRT OCR pipeline and logs many file OCR failure points in `Project-Raycast/omnisearch/src/indexer.rs:1113`.
- Manual OCR actions are offloaded to background threads instead of blocking the UI thread:
  - Ctrl+O selected image OCR: `Project-Raycast/omnisearch/src/main.rs:2680`
  - `ocr_image_file:` action: `Project-Raycast/omnisearch/src/main.rs:5908`
  - `action:ocr_clipboard`: `Project-Raycast/omnisearch/src/main.rs:5942`
- Live clipboard image saving supports multiple classic Win32 image formats:
  - `CF_DIBV5` / `CF_DIB`: `Project-Raycast/omnisearch/src/main.rs:11239`
  - `CF_BITMAP`: `Project-Raycast/omnisearch/src/main.rs:11284`
- SQLite connections generally use WAL and `busy_timeout`, which matches the project rules.

---

# Critical issues

## 1. Fresh DB schema can miss `ocr_text`

File: `Project-Raycast/omnisearch/src/search.rs:596-619`

`SearchEngine::new()` tries to add `ocr_text` before creating `clipboard_history`:

```rust
ALTER TABLE clipboard_history ADD COLUMN ocr_text TEXT;
```

Then it creates the table without `ocr_text`:

```sql
CREATE TABLE IF NOT EXISTS clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT UNIQUE,
    timestamp INTEGER NOT NULL,
    source_app TEXT NOT NULL,
    is_image INTEGER DEFAULT 0,
    pinned INTEGER DEFAULT 0
);
```

On a fresh DB:

1. `ALTER TABLE ... ADD COLUMN ocr_text` silently fails because the table does not exist.
2. `CREATE TABLE` creates the table without `ocr_text`.
3. OCR update queries silently fail:
   - `Project-Raycast/omnisearch/src/main.rs:11211`
   - `Project-Raycast/omnisearch/src/main.rs:11533`
4. Clipboard search prepares queries using `COALESCE(ocr_text, '')`, which fails and returns no results:
   - `Project-Raycast/omnisearch/src/search.rs:2230-2238`

### Fix

Create the table with `ocr_text TEXT`, then run best-effort migrations after creation for older DBs.

Also add a regression test verifying a fresh DB has the `ocr_text` column.

---

## 2. Imported Windows clipboard-history image OCR lacks COM init

File: `Project-Raycast/omnisearch/src/main.rs:11529-11539`

The parent Windows clipboard history import thread initializes COM at `Project-Raycast/omnisearch/src/main.rs:1269-1275`, but each imported image spawns another thread:

```rust
std::thread::spawn(move || {
    if let Some(text) = indexer::ocr_image_file(...) {
        ...
    }
});
```

That child thread calls WinRT APIs through `ocr_image_file()` without calling `CoInitializeEx`.

COM initialization is per-thread, so the parent thread’s initialization does not apply.

### Impact

Imported image history OCR can fail or crash-like behave silently, even though live image OCR paths were already fixed to use background MTA threads.

### Fix

Initialize COM inside this spawned OCR thread, matching the live image OCR path in `save_clipboard_image()`.

---

## 3. Live image clips are skipped if clipboard also contains text

File: `Project-Raycast/omnisearch/src/main.rs:1853-1895`

`WM_CLIPBOARDUPDATE` does:

```rust
if let Some(text) = paste_from_clipboard(hwnd) {
    // store text
} else {
    save_clipboard_image(...)
}
```

Many apps place multiple formats on the clipboard for a copied image: bitmap + HTML + URL + text. In those cases, the code stores the text and never attempts image capture/OCR.

### Likely symptom

OCR may work for screenshots copied from Snipping Tool but fail for images copied from browsers, Discord, Slack, design tools, etc.

### Fix

Check image formats independently from text. If image data exists, save/OCR the image even if text is also present.

---

## 4. Homepage OCR action only supports WinRT bitmap format

File: `Project-Raycast/omnisearch/src/indexer.rs:1059-1108`

The homepage result “Read Clipboard Image Text” runs `ocr_clipboard_image()`, which only uses:

```rust
Clipboard::GetContent()
StandardDataFormats::Bitmap()
content.GetBitmapAsync()
```

But the live save path supports classic Win32 clipboard image formats through:

- `capture_clipboard_dib_bmp_bytes()`: `Project-Raycast/omnisearch/src/main.rs:11239`
- `capture_clipboard_image_data()`: `Project-Raycast/omnisearch/src/main.rs:11284`

### Impact

The homepage OCR action can return `None` for valid image clips that the save/indexing path would handle.

### Fix

Make `ocr_clipboard_image()` fall back to the Win32 capture path, then run OCR through `ocr_image_file()` or a shared in-memory decoder.

---

## 5. OCR preview can panic on Unicode text

File: `Project-Raycast/omnisearch/src/search.rs:2295-2297`

```rust
let preview = if ocr_text.len() > 80 { &ocr_text[..80] } else { &ocr_text };
```

`ocr_text.len()` is bytes, not characters. If byte 80 lands inside a multibyte Unicode character, Rust panics.

### Fix

Use the existing character-safe helper:

```rust
let preview = ellipsize_chars(&ocr_text, 80);
```

---

# Important issues

## 6. Manual Ctrl+O OCR does not persist OCR text

File: `Project-Raycast/omnisearch/src/main.rs:2680-2712`

Ctrl+O OCR on a selected clipboard image runs OCR and posts the result to `WM_OCR_RESULT`, which puts the text into the search box:

`Project-Raycast/omnisearch/src/main.rs:3754-3778`

But it does not update:

```sql
clipboard_history.ocr_text
```

### Impact

If background OCR failed, Ctrl+O can extract text once but does not repair future searchability.

---

## 7. Background OCR does not refresh visible search results

File: `Project-Raycast/omnisearch/src/main.rs:11198-11218`

After live clipboard image OCR updates SQLite, it does not post `WM_REFRESH_SEARCH`.

### Impact

If the user is viewing `clip:` while OCR completes, results stay stale until another search is triggered.

---

## 8. Clipboard OCR is only searchable under `clip:` / `clipboard:`

File: `Project-Raycast/omnisearch/src/search.rs:3984-3992`

Clipboard search is only routed for:

```rust
clipboard:
clip:
```

Global search does not include `clipboard_history.ocr_text`.

### Impact

“Make clipboard images text-searchable” currently means scoped clipboard search only, not full OmniSearch search.

This may be intended, but README wording suggests broader searchability.

---

## 9. OCR/image filters do not align with clipboard OCR results

Clipboard image results are emitted with source `CLIPBOARD`:

`Project-Raycast/omnisearch/src/search.rs:2301-2310`

But the OCR filter checks only:

```rust
FilterType::OCR => src == "OCR"
```

in `Project-Raycast/omnisearch/src/main.rs:10330-10347`.

### Impact

A clipboard image with OCR text can appear under `clip:`, but not under the OCR filter.

---

## 10. Error observability is weak for clipboard OCR

File: `Project-Raycast/omnisearch/src/indexer.rs:1059-1108`

`ocr_clipboard_image()` collapses all failures into `None`:

- no bitmap format
- WinRT bitmap failure
- stream open failure
- decoder failure
- image too large
- OCR engine creation failure
- OCR found no text

The UI then always shows:

```text
No readable image on clipboard (copy a picture first)
```

from `Project-Raycast/omnisearch/src/main.rs:3767-3769`.

### Impact

The user cannot tell whether there is no image, OCR failed, OCR language is unavailable, or the image format is unsupported.

---

## 11. Unbounded OCR thread spawning

Files:

- Live image OCR: `Project-Raycast/omnisearch/src/main.rs:11198-11218`
- Imported history OCR: `Project-Raycast/omnisearch/src/main.rs:11529-11539`

Each image spawns its own OCR thread. Importing a large clipboard history can create many simultaneous WinRT OCR tasks and SQLite writers.

### Fix

Use a small bounded OCR worker queue, probably 1–2 concurrent jobs.

---

## 12. Large clipboard bitmap allocation happens before size guard

File: `Project-Raycast/omnisearch/src/main.rs:11314-11333`

The `CF_BITMAP` path allocates:

```rust
vec![0u8; (bmp.bmWidth * bmp.bmHeight * 4) as usize]
```

before any OCR dimension guard runs.

### Impact

Very large clipboard images can cause memory pressure. Multiplication is also not checked.

---

# Minor / cleanup findings

- `CoUninitialize()` is called unconditionally in several spawned threads even when `CoInitializeEx()` result is ignored:
  - `Project-Raycast/omnisearch/src/main.rs:2693-2700`
  - `Project-Raycast/omnisearch/src/main.rs:5914-5923`
  - `Project-Raycast/omnisearch/src/main.rs:5947-5956`
  - `Project-Raycast/omnisearch/src/main.rs:11202-11217`

  Prefer:

  ```rust
  let com_initialized = CoInitializeEx(...).is_ok();
  ...
  if com_initialized {
      CoUninitialize();
  }
  ```

- `ocr_image_file:` is handled in `execute_selected()` but normal clipboard image results emit `copy_image:`. Ctrl+O handles OCR separately, so `ocr_image_file:` appears mostly unreachable.
- Tests do not cover the new OCR schema or search behavior. Missing high-value tests:
  - fresh DB includes `clipboard_history.ocr_text`
  - `clip:` search matches `ocr_text`
  - Unicode OCR preview does not panic
  - image + text clipboard update still saves the image
- README currently overstates clipboard image search readiness relative to the implementation.

---

# Recommended fix order

1. Fix `clipboard_history` schema creation/migration.
2. Add tests for fresh DB schema and `ocr_text` search.
3. Add COM initialization to imported-history OCR child threads.
4. Replace Unicode byte slicing with `ellipsize_chars()`.
5. Change `WM_CLIPBOARDUPDATE` to capture images independently from text.
6. Add Win32 fallback to `ocr_clipboard_image()`.
7. Persist Ctrl+O OCR results back into `clipboard_history.ocr_text`.
8. Add logging/structured error messages for clipboard OCR failures.
9. Add bounded OCR worker queue.
10. Decide whether clipboard OCR should be global search, `clip:` only, or included in OCR/image filters.

If you want, I can next implement the critical fixes in a focused patch, starting with schema + COM + Unicode crash + image/text clipboard detection.