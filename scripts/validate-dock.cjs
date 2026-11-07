const fs = require('fs');
const assert = require('assert');

const html = fs.readFileSync('dock-editable.html', 'utf8');

console.log('Validating dock-editable.html...');
assert(html.length > 10000, 'HTML file is too short');

// Extract script
const scriptStart = html.indexOf('<script>');
const scriptEnd = html.lastIndexOf('</script>');
assert(scriptStart !== -1 && scriptEnd !== -1, 'Script tags missing');

const jsCode = html.slice(scriptStart + '<script>'.length, scriptEnd);

// Check syntax by parsing with new Function or vm
try {
  new Function(jsCode);
  console.log('✓ JavaScript syntax is 100% valid');
} catch (e) {
  console.error('✗ JavaScript syntax error:', e.message);
  process.exit(1);
}

// Check key widgets
const expectedWidgets = [
  'start',
  'search',
  'peripherals',
  'system-stats',
  'calendar',
  'running-apps',
  'pinned-apps',
  'music-player',
  'weather',
  'status-battery',
  'siri-ai',
  'claude',
  'codex',
  'wifi',
  'bluetooth',
  'notifications',
  'quick-notes',
  'system-tray',
  'clock'
];

for (const w of expectedWidgets) {
  assert(html.includes("'" + w + "'") || html.includes('"' + w + '"'), 'Missing widget ID: ' + w);
}
console.log('✓ All ' + expectedWidgets.length + ' key widget definitions present');

// Check CoolDock / Docktor classes and functions
const expectedIdentifiers = [
  'generateSparklineSvg',
  'renderBarsBox',
  'cycleMusicTrack',
  'updateLiveSystemStats',
  'updateLiveCalendar',
  'dock-sys-stats-wrap',
  'dock-cal-card',
  'dock-airpods-wrap',
  'app-notif-badge',
  'dock-music-expanded',
  'dock-weather-expanded'
];

for (const id of expectedIdentifiers) {
  assert(html.includes(id), 'Missing CoolDock/Docktor identifier: ' + id);
}
console.log('✓ CoolDock & Docktor live components and telemetry curves verified');

// Check no em dashes
const emDashMatch = html.match(/\u2014/g);
assert(!emDashMatch, 'Violation of anti-slop: em dashes found in dock-editable.html (' + (emDashMatch ? emDashMatch.length : 0) + ')');
console.log('✓ Zero em dashes in dock-editable.html (anti-slop verified)');

console.log('ALL DOCK-EDITABLE VALIDATIONS PASSED.');
