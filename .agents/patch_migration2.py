import io
p = r'crates/ps-core/src/migration.rs'
s = io.open(p, encoding='utf-8').read()

# 1) Report gains a sidecar-change flag (WAL/SHM may be touched by read-only opens).
s = s.replace('''    pub skipped_tables: Vec<&'static str>,
    pub source_unchanged: bool,''', '''    pub skipped_tables: Vec<&'static str>,
    /// True when the main database file is byte-identical afterwards.
    pub source_unchanged: bool,
    /// True when the related -wal/-shm sidecar files changed. SQLite can touch
    /// these even for read-only opens, so this is reported separately instead
    /// of being hidden inside "source unchanged".
    pub wal_or_shm_changed: bool,''')

# 2) Percent-encode the URI path (V2-09) and keep the file:/// form.
s = s.replace('''    let normalized = canonical.to_string_lossy().replace('\\\\', "/");
    let stripped = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    Ok(format!("file:///{stripped}?mode=ro"))''', '''    let normalized = canonical.to_string_lossy().replace('\\\\', "/");
    let stripped = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    let mut encoded = String::with_capacity(stripped.len() + 16);
    for ch in stripped.chars() {
        match ch {
            '%' => encoded.push_str("%25"),
            '#' => encoded.push_str("%23"),
            '?' => encoded.push_str("%3F"),
            ' ' => encoded.push_str("%20"),
            other => encoded.push(other),
        }
    }
    Ok(format!("file:///{encoded}?mode=ro"))''')

# 3) Fingerprint helper for db + sidecars.
s = s.replace('''fn sha256_file(path: &Path) -> anyhow::Result<String> {''', '''fn sidecar_fingerprint(db: &Path) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    for suffix in ["", "-wal", "-shm"] {
        let path = if suffix.is_empty() {
            db.to_path_buf()
        } else {
            let mut name = db.as_os_str().to_owned();
            name.push(suffix);
            std::path::PathBuf::from(name)
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                hasher.update(suffix.as_bytes());
                hasher.update(&(bytes.len() as u64).to_le_bytes());
                hasher.update(&bytes);
            }
            Err(_) => {
                hasher.update(suffix.as_bytes());
                hasher.update(b"absent");
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {''')

# 4) In the main migration function: hash sidecars before/after and report imported counts.
s = s.replace('''    let started = std::time::Instant::now();
    let before = sha256_file(legacy_db)?;
    let uri = read_only_uri(legacy_db)?;''', '''    let started = std::time::Instant::now();
    let before = sha256_file(legacy_db)?;
    let sidecars_before = sidecar_fingerprint(legacy_db)?;
    let uri = read_only_uri(legacy_db)?;''')

s = s.replace('''    if !opts.import_clipboard {
        report.skipped_tables.push("clipboard_history");
        report.skipped_tables.push("timeline_events");
    }
    if !opts.import_ai_keys {
        report.skipped_tables.push("ai_settings");
    }''', '''    report.skipped_tables.push("clipboard_history");
    report.skipped_tables.push("timeline_events");
    report.skipped_tables.push("ai_settings");
    if opts.import_clipboard {
        report.warnings.push(
            "clipboard import is never performed by the migration API; enable clipboard capture in settings instead".into(),
        );
    }
    if opts.import_ai_keys {
        report.warnings.push(
            "AI keys are never imported from legacy storage; configure the provider again explicitly".into(),
        );
    }''')

s = s.replace('''        if let Some(summary) = &import_summary {
            report.warnings.extend(summary.warnings.iter().cloned());
        }
        let _ = copied;''', '''        if let Some(summary) = &import_summary {
            report.files = summary.files_copied;
            report.content_docs = summary.content_documents;
            report.warnings.extend(summary.warnings.iter().cloned());
        }
        if let Some(count) = copied.get("browser_items") {
            report.browser_items = *count;
        }
        if let Some(count) = copied.get("quicklinks") {
            report.quicklinks = *count;
        }
        if let Some(count) = copied.get("snippets") {
            report.snippets = *count;
        }
        if report.quicklinks > 0 && copied.get("quicklinks").copied().unwrap_or(0) == 0 {
            report
                .warnings
                .push("legacy quicklinks exist but none could be imported".into());
        }''')

s = s.replace('''    let after = sha256_file(legacy_db)?;
    report.source_unchanged = before == after;''', '''    let after = sha256_file(legacy_db)?;
    let sidecars_after = sidecar_fingerprint(legacy_db)?;
    report.wal_or_shm_changed = sidecars_before != sidecars_after;
    report.source_unchanged = before == after && !report.wal_or_shm_changed;''')

# 5) Unit tests: assert new field and keep expectations valid.
s = s.replace('''        assert!(report.source_unchanged);
        assert_eq!(report.files, 2);''', '''        assert!(report.source_unchanged);
        assert!(!report.wal_or_shm_changed);
        assert_eq!(report.files, 2);''')
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('migration hardened')
