// seed-notifications.mjs - seed or clear ProtonSearch's local notifications table for QA runs.
// Usage: node seed-notifications.mjs --data-dir <dir> --count <N>
//   Needs <dir>/v2/index.db to exist (never creates it). Deletes all rows, then inserts N samples
//   with timestamp = now - i*3 min (ms epoch). --count 0 only clears. Prints {inserted, db}.
// SCHEMA is copied verbatim from crates/ps-sources/src/notifications.rs.
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const SCHEMA = `
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'info',
    read INTEGER NOT NULL DEFAULT 0,
    dismissed INTEGER NOT NULL DEFAULT 0,
    action_url TEXT
);
CREATE INDEX IF NOT EXISTS idx_notifications_timestamp
    ON notifications(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_unread
    ON notifications(read, dismissed);
`;

const SAMPLES = [
  ['Build finished', 'Release build completed with no errors in 42 seconds.'],
  ['Backup complete', 'Nightly index backup finished at 02:00.'],
  ['3 files indexed', 'Three new documents were added to the local index.'],
  ['Update available', 'Version 10.0.1 is ready to install. Restart when convenient.'],
  ['Clipboard history cleared', 'Entries older than 24 hours were removed.'],
  ['Standup in 10 minutes', 'Team sync starts at 10:00 on your calendar.'],
  ['Download complete', 'The installer archive was saved to Downloads.'],
  ['Index rebuild started', 'Results may be partial until the rebuild finishes.'],
  ['Disk space low on D:', 'Only 4.2 GB free. Old backups can be removed.'],
  ['New search source connected', 'The Notes folder is now searchable.'],
  ['Screenshot saved', 'The captured region was saved to Pictures.'],
  ['Sync paused', 'Sync is paused while battery saver is on.'],
];

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const dataDir = opt('--data-dir');
const count = Number(opt('--count'));
if (!dataDir || !Number.isInteger(count) || count < 0) {
  console.error('usage: node seed-notifications.mjs --data-dir <dir> --count <N>');
  process.exit(2);
}

const dbPath = join(dataDir, 'v2', 'index.db');
if (!existsSync(dbPath)) {
  console.error(`error: ${dbPath} not found; refusing to create a new database`);
  process.exit(1);
}

const db = new DatabaseSync(dbPath);
try {
  db.exec('PRAGMA busy_timeout = 5000;'); // WAL-safe: wait for the app's writer instead of failing
  db.exec(SCHEMA);
  db.exec('BEGIN IMMEDIATE;');
  try {
    db.exec('DELETE FROM notifications;');
    const insert = db.prepare(
      'INSERT INTO notifications (timestamp, title, body, source, kind, read, dismissed) VALUES (?, ?, ?, ?, ?, 0, 0)',
    );
    const now = Date.now();
    for (let i = 0; i < count; i++) {
      const [title, body] = SAMPLES[i % SAMPLES.length];
      insert.run(now - i * 3 * 60 * 1000, title, body, 'ProtonSearch', 'info');
    }
    db.exec('COMMIT;');
  } catch (err) {
    db.exec('ROLLBACK;');
    throw err;
  }
} finally {
  db.close();
}
console.log(JSON.stringify({ inserted: count, db: dbPath }));
