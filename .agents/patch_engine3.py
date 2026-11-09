import io
p = r'crates/ps-core/src/engine.rs'
s = io.open(p, encoding='utf-8').read()
s = s.replace('''fn content_result(hit: ps_content::ContentHit) -> SearchResult {''',
              '''fn content_result(hit: ps_content::ContentHit, settings: &Settings) -> SearchResult {''')
s = s.replace('''        title: hit.title,
        subtitle: hit.path.clone(),
        path: Some(hit.path.clone()),''',
              '''        title: hit.title,
        subtitle: display_path(&hit.path, settings),
        path: Some(hit.path.clone()),''')
s = s.replace('''                                result: content_result(hit),''',
              '''                                result: content_result(hit, &settings),''')
s = s.replace('''fn ocr_result(hit: crate::db::OcrHit) -> SearchResult {''',
              '''fn ocr_result(hit: crate::db::OcrHit, settings: &Settings) -> SearchResult {''')
s = s.replace('''        subtitle: hit.path.clone(),
        path: Some(hit.path.clone()),
        icon: IconRef::FileType { ext },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            line: hit.line,
            label: Some("Text in screenshot".into()),''',
              '''        subtitle: display_path(&hit.path, settings),
        path: Some(hit.path.clone()),
        icon: IconRef::FileType { ext },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            line: hit.line,
            label: Some("Text in screenshot".into()),''')
s = s.replace('''                            result: ocr_result(hit),''',
              '''                            result: ocr_result(hit, &settings),''')
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('content/ocr display path patched')
