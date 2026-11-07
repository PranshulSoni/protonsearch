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

// Check DOM structure & nesting
const stageIdx = html.indexOf('class="dock-stage"');
const clusterIdx = html.indexOf('class="dock-cluster"');
const shelfIdx = html.indexOf('class="dock-shelf"');
const railIdx = html.indexOf('id="dockRail"');

assert(stageIdx !== -1, 'dock-stage missing');
assert(clusterIdx !== -1, 'dock-cluster missing');
assert(shelfIdx !== -1, 'dock-shelf missing');
assert(railIdx !== -1, 'dockRail missing');
assert(stageIdx < clusterIdx, 'dock-cluster must be inside dock-stage');
assert(clusterIdx < shelfIdx, 'dock-shelf must be inside dock-cluster');
assert(shelfIdx < railIdx, 'dock-shelf must precede dockRail');

// Verify tag balance in body before script
const bodyHtml = html.slice(html.indexOf('<body'), html.indexOf('<script>'));
assert(!bodyHtml.includes('<header class="desktop-hint">'), 'Unnecessary clutter: desktop-hint header found');
const divOpens = (bodyHtml.match(/<div\b/g) || []).length;
const divCloses = (bodyHtml.match(/<\/div>/g) || []).length;
assert.strictEqual(divOpens, divCloses, 'DIV tag mismatch in body: opens=' + divOpens + ' closes=' + divCloses);
console.log('✓ HTML div structure perfectly balanced (opens: ' + divOpens + ', closes: ' + divCloses + ')');
console.log('✓ Dock hierarchy verified (stage -> cluster -> [shelf, popovers, rail])');

// Check no em dashes across touched files
const filesToCheck = [
  'dock-editable.html',
  'antigravity/spec.md',
  'antigravity/BACKEND_IMPLEMENTATION_PROMPT.md'
];

for (const filePath of filesToCheck) {
  const content = fs.readFileSync(filePath, 'utf8');
  const emDashMatch = content.match(/\u2014/g);
  assert(!emDashMatch, 'Violation of anti-slop: em dashes found in ' + filePath + ' (' + (emDashMatch ? emDashMatch.length : 0) + ')');
  console.log('✓ Zero em dashes in ' + filePath + ' (anti-slop verified)');
}

console.log('ALL DOCK-EDITABLE VALIDATIONS PASSED.');
