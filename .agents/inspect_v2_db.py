import sqlite3, json, os
db = r'.devdata/v2/index.db'
print('exists', os.path.exists(db), 'size_MB', round(os.path.getsize(db)/1e6,1) if os.path.exists(db) else 0)
if os.path.exists(db):
    con = sqlite3.connect(db)
    for t in ('files','documents','documents_fts','browser_items','quicklinks','snippets','activity','ocr_documents'):
        try:
            print(t, con.execute('select count(*) from '+t).fetchone()[0])
        except Exception as e:
            print(t, 'ERR', e)
    print('frag sample', con.execute('select path from files limit 3').fetchall())
settings = json.load(open(r'.devdata/v2/settings.json', encoding='utf-8'))
print('legacyMigrated', settings.get('legacyMigrated'), 'contentEnabled', settings['indexing']['contentEnabled'], 'onboarding', settings.get('onboardingComplete'))
