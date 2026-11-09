import os, json, shutil, sqlite3, hashlib
src = os.path.abspath('.devdata-mig/file_index.db')
dst_dir = os.path.abspath('.devdata-mig2')
os.makedirs(dst_dir, exist_ok=True)
dst = os.path.join(dst_dir, 'file_index.db')
shutil.copy2(src, dst)
user = os.path.expandvars(r'%USERPROFILE%')
settings = {
  'schemaVersion': 1, 'userName': 'Pranshul', 'hotkey': 'Alt+Space', 'theme': 'light',
  'density': 'comfortable', 'motion': 'system', 'launchAtLogin': False,
  'indexing': {'roots': [
      {'path': os.path.join(user, 'Documents'), 'mode': 'content', 'enabled': True},
      {'path': os.path.join(user, 'Desktop'), 'mode': 'names_only', 'enabled': True},
      {'path': os.path.join(user, 'Downloads'), 'mode': 'names_only', 'enabled': True}],
    'excludedPaths': [], 'contentEnabled': True, 'ocrEnabled': False,
    'respectPowerState': True, 'paused': False},
  'privacy': {'activityHistory': True, 'clipboardHistory': True, 'browserHistory': True,
              'gitIndexing': True, 'telemetry': False},
  'agent': {'enabled': False}, 'semantic': {'enabled': False},
  'onboardingComplete': True, 'legacyMigrated': False, 'resultLimit': 30,
}
os.makedirs(os.path.join(dst_dir, 'v2'), exist_ok=True)
json.dump(settings, open(os.path.join(dst_dir, 'v2', 'settings.json'), 'w', encoding='utf-8'), indent=2)
print('prepared', dst_dir, 'legacy_MB', round(os.path.getsize(dst)/1e6,1))
