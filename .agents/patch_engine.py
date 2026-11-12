import io, re, sys
p = r'crates/ps-core/src/engine.rs'
s = io.open(p, encoding='utf-8').read()

def rep(old, new, count=1):
    global s
    n = s.count(old)
    if n != count:
        print('WARN count mismatch', n, 'expected', count, 'for', old[:60].replace(chr(10),'|'))
    s = s.replace(old, new, count)

# 1) ps-index re-exports are at crate root
rep('ps_index::scan::ScanOptions', 'ps_index::ScanOptions')
rep('ps_index::scan::scan(options)', 'ps_index::scan(options)')

# 2) took_ms expects f32
s = s.replace('as_secs_f64() * 1000.0', 'as_secs_f64() as f32 * 1000.0')

# 3) content access through a helper (Option is inside the mutex)
rep('''            if let Some(content) = self.content_lock() {
                match content.search(&query_text, 12) {''',
'''            if let Some(result) = self.with_content(|content| content.search(&query_text, 12)) {
                match result {''')

rep('''            IndexCommand::RebuildContent => {
                if let Some(content) = self.content_lock() {
                    content.clear().ok();
                }''',
'''            IndexCommand::RebuildContent => {
                let _ = self.with_content(|content| content.clear());''')

rep('''        if let Some(content) = self.content_lock() {
            match content.import_legacy(legacy) {
                Ok(summary) => {''',
'''        if let Some(result) = self.with_content(|content| content.import_legacy(legacy)) {
            match result {
                Ok(summary) => {''')

rep('''                    self.content_documents
                        .store(content.stats().documents, Ordering::SeqCst);''',
'''                    let documents = self
                        .with_content(|content| content.stats().documents)
                        .unwrap_or(0);
                    self.content_documents.store(documents, Ordering::SeqCst);''')

# 4) index_content: no long-lived guard
rep('''    let Some(content) = engine.content_lock() else {
        return;
    };
    let mut written = 0usize;
    for path in candidates {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        match content.index_file(&path, ps_content::DEFAULT_MAX_BYTES) {
            Ok(true) => written += 1,
            Ok(false) => {}
            Err(err) => engine.push_warning(format!("content extraction failed for {}: {err}", path.display())),
        }
    }
    engine
        .content_documents
        .store(content.stats().documents, Ordering::SeqCst);''',
'''    if engine.with_content(|_| ()).is_none() {
        return;
    }
    let mut written = 0usize;
    for path in candidates {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        match engine.with_content(|content| content.index_file(&path, ps_content::DEFAULT_MAX_BYTES)) {
            Some(Ok(true)) => written += 1,
            Some(Ok(false)) => {}
            Some(Err(err)) => engine
                .push_warning(format!("content extraction failed for {}: {err}", path.display())),
            None => break,
        }
    }
    let documents = engine
        .with_content(|content| content.stats().documents)
        .unwrap_or(0);
    engine.content_documents.store(documents, Ordering::SeqCst);''')

# 5) helper replaces content_lock
rep('''    fn content_lock(&self) -> Option<std::sync::MutexGuard<'_, Option<ps_content::ContentIndex>>> {
        self.content.lock().ok()
    }''',
'''    /// Run a closure with the content index, if it is available. The lock is
    /// held only for the duration of the call (search/extraction are bounded).
    fn with_content<R>(&self, f: impl FnOnce(&ps_content::ContentIndex) -> R) -> Option<R> {
        let guard = self.content.lock().ok()?;
        guard.as_ref().map(f)
    }''')

# 6) first_line type
rep('''    let first_line = hit.text.lines().next().unwrap_or("").chars().take(120).collect();''',
'''    let first_line: String = hit.text.lines().next().unwrap_or("").chars().take(120).collect();''')

io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('patched')
