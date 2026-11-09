//! Content extraction + FTS5 index tests with real fixtures created on disk.
//!
//! Fixtures are generated at test time (handcrafted PDF, DOCX zip, UTF-16 text,
//! CJK text, malformed PDF, synthetic legacy v1 database) so the suite is
//! self-contained and does not depend on the user's data.

use std::io::Write;
use std::path::{Path, PathBuf};

use ps_content::{
    extract_text, import_legacy, ContentIndex, DocKind, ExtractOptions, DEFAULT_MAX_BYTES,
};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// Build a minimal, structurally valid one-page PDF with known text.
fn minimal_pdf(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 24 Tf 72 720 Td ({text}) Tj ET");
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{body}\nendobj\n", index + 1));
    }
    let xref_offset = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
    pdf.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        objects.len() + 1
    ));
    pdf.into_bytes()
}

fn write_docx(path: &Path, paragraphs: &[&str]) {
    use zip::write::FileOptions;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(rels.as_bytes()).unwrap();
    let mut body = String::new();
    for paragraph in paragraphs {
        body.push_str(&format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">{paragraph}</w:t></w:r></w:p>"
        ));
    }
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{body}</w:body></w:document>"#
    );
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(document.as_bytes()).unwrap();
    zip.finish().unwrap();
}

fn write_xlsx(path: &Path, shared: &[&str], rows: &[&[&str]]) {
    use zip::write::FileOptions;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#).unwrap();
    let mut shared_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    for value in shared {
        shared_xml.push_str(&format!("<si><t>{value}</t></si>"));
    }
    shared_xml.push_str("</sst>");
    zip.start_file("xl/sharedStrings.xml", options).unwrap();
    zip.write_all(shared_xml.as_bytes()).unwrap();
    let mut sheet = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for row in rows {
        sheet.push_str("<row>");
        for cell in *row {
            if let Some(index) = cell.strip_prefix('$') {
                sheet.push_str(&format!("<c t=\"s\"><v>{index}</v></c>"));
            } else {
                sheet.push_str(&format!("<c><v>{cell}</v></c>"));
            }
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
    zip.write_all(sheet.as_bytes()).unwrap();
    zip.finish().unwrap();
}

/// Synthetic legacy v1 database with the real legacy schema subset.
fn make_legacy_db(path: &Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        r#"
CREATE TABLE files (
    path TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    extension TEXT NOT NULL,
    modified INTEGER NOT NULL,
    size INTEGER NOT NULL DEFAULT 0,
    is_dir INTEGER NOT NULL DEFAULT 0,
    last_seen_scan INTEGER NOT NULL DEFAULT 0
);
CREATE VIRTUAL TABLE files_fts USING fts5(path UNINDEXED, content);
CREATE TABLE quicklinks (name TEXT PRIMARY KEY, url TEXT NOT NULL, keyword TEXT NOT NULL);
CREATE TABLE snippets (name TEXT PRIMARY KEY, content TEXT NOT NULL, keyword TEXT);
"#,
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files(path,name,extension,modified,size,is_dir) VALUES(?1,?2,?3,?4,?5,0)",
        rusqlite::params![
            "C:\\Users\\legacy\\notes\\quarterly-notes.md",
            "quarterly-notes.md",
            "md",
            1_760_000_000i64,
            512i64
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files(path,name,extension,modified,size,is_dir) VALUES(?1,?2,?3,?4,?5,0)",
        rusqlite::params![
            "C:\\Users\\legacy\\docs\\architecture-notes.pdf",
            "architecture-notes.pdf",
            "pdf",
            1_760_000_100i64,
            2048i64
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files_fts(path, content) VALUES(?1, ?2)",
        rusqlite::params![
            "C:\\Users\\legacy\\notes\\quarterly-notes.md",
            "legacy quarterly revenue projection and hiring plan"
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO files_fts(path, content) VALUES(?1, ?2)",
        rusqlite::params![
            "C:\\Users\\legacy\\docs\\architecture-notes.pdf",
            "legacy architecture document about search ranking and index layout"
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO quicklinks(name,url,keyword) VALUES('Docs','https://example.com/docs','docs')",
        [],
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Extraction tests
// ---------------------------------------------------------------------------

#[test]
fn detects_supported_and_unsupported_kinds() {
    assert_eq!(ps_content::detect_kind(Path::new("a.PDF")), DocKind::Pdf);
    assert_eq!(ps_content::detect_kind(Path::new("notes.md")), DocKind::Markdown);
    assert_eq!(ps_content::detect_kind(Path::new("main.rs")), DocKind::Code);
    assert_eq!(ps_content::detect_kind(Path::new("doc.docx")), DocKind::Docx);
    assert_eq!(ps_content::detect_kind(Path::new("app.exe")), DocKind::Other);
    assert!(!ps_content::is_supported(Path::new("image.png")));
    assert!(ps_content::is_supported(Path::new("notes.txt")));
}

#[test]
fn extracts_minimal_pdf_text() {
    let dir = workdir("extract-pdf");
    let pdf = dir.join("report.pdf");
    std::fs::write(&pdf, minimal_pdf("Hello ProtonSearch PDF content search 12345")).unwrap();
    let document = extract_text(&pdf, DEFAULT_MAX_BYTES)
        .expect("pdf extraction must not fail")
        .expect("pdf is supported");
    assert_eq!(document.kind, DocKind::Pdf);
    assert!(!document.truncated);
    assert!(
        document.text.contains("ProtonSearch"),
        "unexpected pdf text: {:?}",
        document.text
    );
}

#[test]
fn extracts_docx_paragraphs() {
    let dir = workdir("extract-docx");
    let docx = dir.join("report.docx");
    write_docx(&docx, &["Quarterly protonsearch report", "Second paragraph budget"]);
    let document = extract_text(&docx, DEFAULT_MAX_BYTES)
        .expect("docx extraction must not fail")
        .expect("docx is supported");
    assert_eq!(document.kind, DocKind::Docx);
    assert!(document.text.contains("Quarterly"), "{:?}", document.text);
    assert!(document.text.contains("Second paragraph"), "{:?}", document.text);
}

#[test]
fn extracts_xlsx_best_effort() {
    let dir = workdir("extract-xlsx");
    let xlsx = dir.join("budget.xlsx");
    write_xlsx(&xlsx, &["Revenue", "Costs"], &[&["$0", "$1"], &["42", "7"]]);
    let document = extract_text(&xlsx, DEFAULT_MAX_BYTES)
        .expect("xlsx extraction must not fail")
        .expect("xlsx is supported");
    assert_eq!(document.kind, DocKind::Xlsx);
    assert!(document.text.contains("Revenue"), "{:?}", document.text);
    assert!(document.text.contains("42"), "{:?}", document.text);
}

#[test]
fn extracts_utf16_text_with_bom() {
    let dir = workdir("extract-utf16");
    let path = dir.join("utf16.txt");
    let text = "ProtonSearch UTF16 例子 content";
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes).unwrap();
    let document = extract_text(&path, DEFAULT_MAX_BYTES).unwrap().unwrap();
    assert_eq!(document.kind, DocKind::Text);
    assert!(document.text.contains("ProtonSearch"), "{:?}", document.text);
    assert!(document.text.contains("例子"), "{:?}", document.text);
}

#[test]
fn extracts_cjk_text() {
    let dir = workdir("extract-cjk");
    let path = write_file(
        &dir,
        "cjk.md",
        "搜索内容检索 中文文档 ProtonSearch".as_bytes(),
    );
    let document = extract_text(&path, DEFAULT_MAX_BYTES).unwrap().unwrap();
    assert_eq!(document.kind, DocKind::Markdown);
    assert!(document.text.contains("搜索"), "{:?}", document.text);
}

#[test]
fn malformed_pdf_errors_without_panicking() {
    let dir = workdir("extract-malformed");
    let path = write_file(&dir, "broken.pdf", b"%PDF-1.4\nthis is not a pdf");
    match extract_text(&path, DEFAULT_MAX_BYTES) {
        Ok(None) => {}
        Ok(Some(document)) => {
            // Some permissive parsers return partial text; it must be bounded.
            assert!(document.text.len() <= DEFAULT_MAX_BYTES);
        }
        Err(error) => {
            assert!(!error.to_string().is_empty());
        }
    }
}

#[test]
fn caps_extracted_text_and_flags_truncation() {
    let dir = workdir("extract-cap");
    let long = "protonsearch ".repeat(4000); // ~52 KB
    let path = write_file(&dir, "long.txt", long.as_bytes());
    let document = extract_text(&path, 4096).unwrap().unwrap();
    assert!(document.truncated);
    assert!(document.text.len() <= 4096, "len={}", document.text.len());
}

/// Malformed PDF whose content stream cannot be parsed. pdf-extract 0.10
/// panics internally on this class of input (InvalidContentStream), which used
/// to kill the extraction worker noisily; the pipeline must contain it.
fn pdf_with_invalid_content_stream() -> Vec<u8> {
    let content: Vec<u8> = b"BT ((( /F1 12 Tf 72 720 Td (broken) Tj ET \xFF\xFE\x00\x01 garbage ) Tj".to_vec();
    let objects: Vec<(String, Vec<u8>)> = vec![
        ("<< /Type /Catalog /Pages 2 0 R >>".to_string(), Vec::new()),
        ("<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(), Vec::new()),
        (
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
            Vec::new(),
        ),
        (
            format!("<< /Length {} >>\nstream\n", content.len()),
            content,
        ),
        ("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(), Vec::new()),
    ];
    let mut pdf: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, (dictionary, stream)) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{dictionary}", index + 1).as_bytes());
        if !stream.is_empty() {
            pdf.extend_from_slice(stream);
            pdf.extend_from_slice(b"\nendstream");
        }
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n", objects.len() + 1).as_bytes(),
    );
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[test]
fn panicking_pdf_is_contained_and_batch_continues() {
    let dir = workdir("pdf-panic-containment");
    let bad = dir.join("broken-content-stream.pdf");
    std::fs::write(&bad, pdf_with_invalid_content_stream()).unwrap();
    let good = dir.join("good-after-panic.txt");
    std::fs::write(&good, b"protonsearch survives the malformed pdf").unwrap();
    let db = dir.join("index.db");
    let index = ContentIndex::open(&db).unwrap();

    // A panicking parser must not abort the process nor the indexing batch.
    let summary = index.index_paths(&[bad.clone(), good.clone()], DEFAULT_MAX_BYTES);
    assert!(summary.indexed >= 1, "good file must be indexed: {summary:?}");
    let hits = index.search("protonsearch", 10).unwrap();
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert!(hits[0].path.ends_with("good-after-panic.txt"));
    if summary.errors > 0 {
        let stats = index.stats();
        assert!(stats.errors >= 1, "errors must be recorded: {stats:?}");
    }

    // Direct extraction of the malformed PDF must return (Err or Ok), never panic.
    match extract_text(&bad, DEFAULT_MAX_BYTES) {
        Ok(None) => {}
        Ok(Some(document)) => assert!(document.text.len() <= DEFAULT_MAX_BYTES),
        Err(error) => assert!(!error.to_string().is_empty()),
    }
}

// ---------------------------------------------------------------------------
// Index / search tests
// ---------------------------------------------------------------------------

fn index_two_documents(index: &ContentIndex, dir: &Path) -> (PathBuf, PathBuf) {
    let notes = write_file(
        dir,
        "notes.txt",
        b"line one\nline two\nprotonsearch needle is here\nline four\n",
    );
    let report = write_file(
        dir,
        "report.md",
        b"# Report\nThe ranking architecture uses BM25 scoring inside FTS5.\n",
    );
    assert!(index.index_file(&notes, DEFAULT_MAX_BYTES).unwrap());
    assert!(index.index_file(&report, DEFAULT_MAX_BYTES).unwrap());
    (notes, report)
}

#[test]
fn indexes_and_searches_content_with_line_numbers() {
    let dir = workdir("index-search");
    let db = dir.join("index.db");
    let index = ContentIndex::open(&db).unwrap();
    index_two_documents(&index, &dir);

    let hits = index.search("needle", 10).unwrap();
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert_eq!(hits[0].snippet_line, Some(3));
    assert!(hits[0].path.ends_with("notes.txt"));
    assert!(hits[0].title.contains("notes"));

    let architecture = index.search("bm25 ranking", 10).unwrap();
    assert_eq!(architecture.len(), 1, "hits={architecture:?}");
    assert!(architecture[0].path.ends_with("report.md"));

    assert!(index.search("nonexistent-term-xyz", 10).unwrap().is_empty());
    let stats = index.stats();
    assert_eq!(stats.documents, 2);
    assert!(stats.bytes_indexed > 0);
}

#[test]
fn skips_unchanged_files_and_reindexes_changed_ones() {
    let dir = workdir("index-incremental");
    let db = dir.join("index.db");
    let index = ContentIndex::open(&db).unwrap();
    let file = write_file(&dir, "mutable.txt", b"version one protonsearch");
    assert!(index.index_file(&file, DEFAULT_MAX_BYTES).unwrap());
    assert!(
        !index.index_file(&file, DEFAULT_MAX_BYTES).unwrap(),
        "unchanged file must be skipped"
    );
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, b"version two protonsearch ranking").unwrap();
    assert!(
        index.index_file(&file, DEFAULT_MAX_BYTES).unwrap(),
        "changed file must be reindexed"
    );
    let hits = index.search("ranking", 10).unwrap();
    assert_eq!(hits.len(), 1);
}

#[test]
fn removes_removed_files_from_search() {
    let dir = workdir("index-remove");
    let db = dir.join("index.db");
    let index = ContentIndex::open(&db).unwrap();
    let file = write_file(&dir, "gone.txt", b"vanishing protonsearch token");
    assert!(index.index_file(&file, DEFAULT_MAX_BYTES).unwrap());
    assert_eq!(index.search("vanishing", 10).unwrap().len(), 1);
    index.remove_file(&file).unwrap();
    assert!(index.search("vanishing", 10).unwrap().is_empty());
    assert_eq!(index.stats().documents, 0);
}

#[test]
fn fts_query_cannot_inject_syntax() {
    let dir = workdir("index-injection");
    let db = dir.join("index.db");
    let index = ContentIndex::open(&db).unwrap();
    index_two_documents(&index, &dir);
    for query in [
        "needle\" OR \"*",
        "needle'); DROP TABLE documents; --",
        "NEAR(needle report)",
        "title:needle",
        "\"unterminated",
        "((((",
    ] {
        // Must return a normal result (or empty) and never error or corrupt the DB.
        let _ = index.search(query, 10).expect("query must be sanitized");
    }
    assert_eq!(index.stats().documents, 2, "documents table must survive");
    assert_eq!(index.search("needle", 10).unwrap().len(), 1);
}

#[test]
fn sanitized_query_is_quoted_terms() {
    let expression = ps_content::sanitize_fts_query("alpha beta");
    assert_eq!(expression, "\"alpha\" \"beta\"");
    assert!(ps_content::sanitize_fts_query("(((  )))").is_empty());
    assert_eq!(ps_content::sanitize_fts_query("a\"b"), "\"a\" \"b\"");
}

// ---------------------------------------------------------------------------
// Legacy import tests
// ---------------------------------------------------------------------------

#[test]
fn imports_legacy_files_and_content_read_only() {
    let dir = workdir("legacy-import");
    let legacy = dir.join("legacy.db");
    make_legacy_db(&legacy);
    let legacy_bytes_before = std::fs::read(&legacy).unwrap();

    let v2 = dir.join("v2").join("index.db");
    let summary = import_legacy(&legacy, &v2).expect("legacy import");
    assert_eq!(summary.files_copied, 2, "summary={summary:?}");
    assert_eq!(summary.content_documents, 2, "summary={summary:?}");

    let index = ContentIndex::open(&v2).unwrap();
    let hits = index.search("quarterly revenue", 10).unwrap();
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert!(hits[0].path.contains("quarterly-notes.md"));
    let architecture = index.search("ranking layout", 10).unwrap();
    assert_eq!(architecture.len(), 1, "hits={architecture:?}");

    // The legacy database must be byte-identical after import.
    let legacy_bytes_after = std::fs::read(&legacy).unwrap();
    assert_eq!(legacy_bytes_before, legacy_bytes_after, "legacy db was mutated");

    // Second import is idempotent for content rows.
    let second = import_legacy(&legacy, &v2).unwrap();
    assert_eq!(second.content_documents, 0, "summary={second:?}");
    assert_eq!(second.files_copied, 2);
}

#[test]
fn legacy_import_rejects_missing_database() {
    let dir = workdir("legacy-missing");
    let error = import_legacy(&dir.join("nope.db"), &dir.join("v2.db"));
    assert!(error.is_err());
}

// ---------------------------------------------------------------------------
// Settings / options plumbing
// ---------------------------------------------------------------------------

#[test]
fn extract_options_are_respected() {
    let dir = workdir("extract-options");
    let path = write_file(&dir, "opts.txt", b"abcdefghijklmnopqrstuvwxyz");
    let options = ExtractOptions {
        max_bytes: 20,
        timeout: std::time::Duration::from_secs(5),
    };
    let document = ps_content::extract_text_with(&path, options).unwrap().unwrap();
    assert!(document.truncated);
    assert!(document.text.len() <= 20);
}