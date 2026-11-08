import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const PINNED_ID = '2126c4990818e68d157e2207977702f5ab349d19';
const blobId = (bytes) => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');

const lf = (path) => readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
const html = lf('docs/design/dock-editable.2126c49.html');
assert.equal(blobId(Buffer.from(html)), PINNED_ID);

const style = html.slice(html.indexOf('<style'), html.indexOf('</style>'));
for (const re of [
  /--tile-h:\s*48px/,
  /--dock-gap:\s*6px/,
  /--dock-pad:\s*6px/,
  /--dock-radius:\s*18px/,
  /--mag-strength:\s*0\.22/,
  /--mag-radius:\s*86px/,
  /--lift:\s*-4px/,
]) assert.match(style, re);
assert.doesNotMatch(style, /@media/);

const dockOrder = html.match(/var DEFAULT_DOCK_ORDER = \[([\s\S]*?)\];/)[1];
assert.equal(dockOrder.match(/'[^']*'/g).length, 19);

const profiles = html.match(/var DEFAULT_PROFILES = \{([\s\S]*?)\n\s*\};/)[1];
const profileIds = (name) => {
  const list = profiles.match(new RegExp(`${name}: \\[([^\\]]*)\\]`))[1];
  return [...list.matchAll(/'([^']*)'/g)].map(([, id]) => id);
};
assert.deepEqual(profileIds('daily'), ['start', 'search', 'peripherals', 'system-stats', 'calendar', 'running-apps', 'music-player', 'weather', 'clock']);
assert.deepEqual(profileIds('work'), ['start', 'search', 'calendar', 'system-stats', 'running-apps', 'pinned-apps', 'weather', 'status-battery', 'wifi', 'bluetooth', 'notifications', 'quick-notes', 'clock']);
assert.deepEqual(profileIds('founder'), ['start', 'search', 'claude', 'codex', 'system-stats', 'calendar', 'running-apps', 'pinned-apps', 'music-player', 'wifi', 'system-tray', 'clock']);
assert.deepEqual(profileIds('focus'), ['start', 'search', 'calendar', 'system-stats', 'music-player', 'quick-notes', 'clock']);

const cycle = html.slice(html.indexOf('function cycleWidgetWidth'));
assert.match(cycle.slice(0, cycle.indexOf('function ', 1)), /48[\s\S]*96[\s\S]*140[\s\S]*168/);

const live = blobId(Buffer.from(lf('dock-editable.html')));
console.log(`dock-editable.html drift: ${live === PINNED_ID ? 'none' : `yes (${live})`}`);
console.log('html-pin: ok');
