const fs = require('fs');
const assert = require('assert');
const { JSDOM, VirtualConsole } = require('jsdom');

const html = fs.readFileSync('dock-editable.html', 'utf8');

const errors = [];
const vc = new VirtualConsole();
vc.on('jsdomError', e => errors.push(e.message));

const dom = new JSDOM(html, {
  url: 'http://localhost',
  runScripts: 'dangerously',
  pretendToBeVisual: true,
  virtualConsole: vc,
  beforeParse(w) {
    w.requestAnimationFrame = () => 0;
    w.matchMedia = () => ({ matches: false });
    w.HTMLElement.prototype.scrollIntoView = () => {};
    w.confirm = () => true;
  }
});

assert.deepEqual(errors, [], 'No JSDOM script errors allowed');
const w = dom.window;

console.log('Testing ProtonSearch Launcher on dock-editable.html...');

// 1. Check modal initial state
const modal = w.document.getElementById('searchModal');
assert(modal, '#searchModal must exist');
assert(!modal.classList.contains('is-open'), 'Modal must initially be closed');

// 2. Open search modal
w.toggleProtonSearch(true);
assert(modal.classList.contains('is-open'), 'Modal must open when toggleProtonSearch(true) called');

// 3. Search query and results
w.handleSearchInput('code');
const rows = w.document.querySelectorAll('.ps-result-row');
assert(rows.length > 0, 'Must render .ps-result-row elements on query');
console.log('✓ Found', rows.length, 'result rows for "code"');

// 4. Test match highlighting
const highlights = w.document.querySelectorAll('.ps-match-hl');
assert(highlights.length > 0, 'Query match text must be highlighted with .ps-match-hl');
console.log('✓ Match highlighting verified (.ps-match-hl count: ' + highlights.length + ')');

// 5. Test Math parser
w.handleSearchInput('sqrt(144) + 15% of 200');
const calcHero = w.document.getElementById('calculatorHeroContainer');
const calcVal = w.document.getElementById('calcHeroVal');
assert.equal(calcHero.style.display, 'block', 'Calculator hero should be displayed for math expressions');
assert.equal(calcVal.textContent, '= 42', 'Math expression sqrt(144) + 15% of 200 must evaluate to 42');
console.log('✓ In-process math calculation evaluated correctly: = 42');

// 6. Test scope switching
w.handleSearchInput('');
w.selectNavScope('apps');
const activePill = w.document.querySelector('.nav-scope-pill.active');
assert(activePill, 'Scope pill must have active class');
assert.equal(activePill.dataset.scope, 'apps', 'Active pill scope must be apps');
const appRows = w.document.querySelectorAll('.ps-result-row');
assert(appRows.length > 0, 'Apps scope must render app rows');
console.log('✓ Scope switching to "apps" verified (' + appRows.length + ' apps found)');

// 7. Test Quick Look drawer
const qlDrawer = w.document.getElementById('psQuickLookDrawer');
assert(qlDrawer, '#psQuickLookDrawer must exist');
w.openQuickLook();
assert.equal(qlDrawer.style.display, 'flex', 'Quick Look drawer must display on openQuickLook()');
w.closeQuickLook();
assert.equal(qlDrawer.style.display, 'none', 'Quick Look drawer must hide on closeQuickLook()');
console.log('✓ Spacebar Quick Look drawer functionality verified');

// 8. Close search modal
w.toggleProtonSearch(false);
assert(!modal.classList.contains('is-open'), 'Modal must close on toggleProtonSearch(false)');
console.log('✓ Modal close verified');

dom.window.close();
console.log('ALL PROTONSEARCH LAUNCHER TESTS PASSED.');
