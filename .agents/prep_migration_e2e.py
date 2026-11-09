import sqlite3, os, hashlib, json, shutil, time
src = os.path.join(os.environ['APPDATA'], 'protonsearch', 'file_index.db')
dst_dir = os.path.abspath('.devdata-mig')
os.makedirs(dst_dir, exist_ok=True)
dst = os.path.join(dst_dir, 'file_index.db')
def sha(p):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()
before = sha(src)
print('legacy_size_MB', round(os.path.getsize(src)/1e6, 1), 'sha_before', before[:16])
if os.path.exists(dst):
    os.remove(dst)
# Consistent, WAL-aware read-only backup of the live legacy index.
s = sqlite3.connect('file:' + src.replace('\\','/') + '?mode=ro', uri=True)
d = sqlite3.connect(dst)
s.backup(d)
d.close(); s.close()
print('copy_size_MB', round(os.path.getsize(dst)/1e6, 1))
con = sqlite3.connect(dst)
print('copy_files', con.execute('select count(*) from files').fetchone()[0])
print('copy_fts', con.execute('select count(*) from files_fts').fetchone()[0])
con.close()
# Seed v2 settings: onboarding done, names + content enabled
user = os.path.expandvars(r'%USERPROFILE%')
settings = {
  'schemaVersion': 1, 'userName': 'Pranshul', 'hotkey': 'Alt+Space', 'theme': 'system',
  'density': 'comfortable', 'motion': 'system', 'launchAtLogin': False,
  'indexing': {'roots': [
      {'path': os.path.join(user, 'Documents'), 'mode': 'content', 'enabled': True},
      {'path': os.path.join(user, 'Desktop'), 'mode': 'names_only', 'enabled': True},
      {'path': os.path.join(user, 'Downloads'), 'mode': 'names_only', 'enabled': True}],
    'excludedPaths': [], 'contentEnabled': True, 'ocrEnabled': False,
    'respectPowerState': True, 'paused': False},
  'privacy': {'activityHistory': True, 'clipboardHistory': True, 'browserHistory': False,
              'gitIndexing': False, 'telemetry': False},
  'agent': {'enabled': False}, 'semantic': {'enabled': False},
  'onboardingComplete': True, 'legacyMigrated': False, 'resultLimit': 30,
}
os.makedirs(os.path.join(dst_dir, 'v2'), exist_ok=True)
with open(os.path.join(dst_dir, 'v2', 'settings.json'), 'w', encoding='utf-8') as f:
    json.dump(settings, f, indent=2)
print('seeded settings')
