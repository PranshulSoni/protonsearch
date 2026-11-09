import sqlite3
con = sqlite3.connect('file:C:/Users/Pranshul/AppData/Roaming/protonsearch/file_index.db?mode=ro&immutable=1', uri=True)
rows = con.execute("select type,name,sql from sqlite_master where type in ('table','index') order by type,name").fetchall()
for t,n,s in rows:
    print(t, n, ':', (s or '').replace('\n',' ')[:300])
print('---COUNTS---')
for t,n,s in rows:
    if t=='table':
        try:
            print(n, con.execute('select count(*) from "'+n+'"').fetchone()[0])
        except Exception as e:
            print(n, 'ERR', e)
