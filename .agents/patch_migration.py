import io
p = r'crates/ps-core/src/engine.rs'
s = io.open(p, encoding='utf-8').read()
start = s.index('    fn migrate_legacy(&self, legacy: &Path) {')
end = s.index('// ---------------------------------------------------------------------------\n// Background pipeline')
new = '''    fn migrate_legacy(&self, legacy: &Path) {
        let settings = self.settings_snapshot();
        if settings.legacy_migrated {
            return;
        }
        self.set_phase("migrating");
        let db_path = self.config.data_dir.join(DB_FILE);
        let options = crate::migration::MigrationOptions {
            import_clipboard: false,
            import_ai_keys: false,
            dry_run: false,
        };
        match crate::migration::migrate_legacy_db(legacy, &db_path, &options) {
            Ok(report) => {
                self.push_warning(format!(
                    "imported legacy index: {} files ({} folders), {} documents, {} browser items, {} quicklinks, {} snippets ({} ms)",
                    report.files,
                    report.dirs,
                    report.content_docs,
                    report.browser_items,
                    report.quicklinks,
                    report.snippets,
                    report.duration_ms
                ));
                for warning in report.warnings {
                    self.push_warning(warning);
                }
                if !report.source_unchanged {
                    self.push_warning(
                        "legacy database changed during migration - the old index must be restored from backup"
                            .to_string(),
                    );
                }
                if let Ok(entries) = load_migrated_files(&db_path) {
                    if !entries.is_empty() {
                        let mut index = self.index.write().unwrap();
                        index.replace_all(entries);
                    }
                }
                let documents = self
                    .with_content(|content| content.stats().documents)
                    .unwrap_or(0);
                self.content_documents.store(documents, Ordering::SeqCst);
            }
            Err(err) => self.push_warning(format!("legacy import failed: {err}")),
        }
        let mut next = self.settings_snapshot();
        next.legacy_migrated = true;
        let _ = self.write_settings(&next);
    }
}

'''
s = s[:start] + new + s[end:]
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('migrate_legacy patched')
