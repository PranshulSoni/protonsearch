import sqlite3, json, os
db = r'.devdata-mig/v2/index.db'
print('v2_db_MB', round(os.path.getsize(db)/1e6,1))
con = sqlite3.connect(db)
for t in ('files','documents','documents_fts','browser_items','quicklinks','snippets','activity','migration_state'):
    try: print(t, con.execute('select count(*) from '+t).fetchone()[0])
    except Exception as e: print(t, 'ERR', e)
print('legacyMigrated', json.load(open(r'.devdata-mig/v2/settings.json', encoding='utf-8')).get('legacyMigrated'))
