#!/usr/bin/env node
// Input -> first-visible-result latency harness via the Chrome DevTools Protocol.
//
// READ-ONLY design: it attaches to an ALREADY RUNNING WebView2/browser page and
// evaluates JavaScript inside it. It does not modify app source, does not install
// anything, opens no network connections except the local DevTools endpoint.
//
// What it measures precisely:
//   ps:keydown          keydown capture listener in the page (performance.now)
//   ps:results-committed first DOM mutation that changes the result count
//   ps:first-paint      two requestAnimationFrame ticks after that commit
//   input -> first visible result = ps:first-paint - ps:keydown
//
// What it does NOT measure: hotkey -> window visible (needs a Rust-side mark;
// see docs/performance/METHODOLOGY.md section 10), engine-only time (use the
// Rust SearchResponse.took_ms in app logs), or GPU/compositor timing.
//
// Targets:
//   * Tauri app: start it with
//       $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"
//     then: node tools/ui-latency/cdp-latency.mjs --endpoint http://127.0.0.1:9222
//   * Browser mock page (Vite dev server) for harness validation only:
//       npm run dev -- --port 5173   then the same command with the page URL.
//     Numbers from the mock bridge are labelled MOCK-BRIDGE and are NOT product
//     latency; do not publish them as such.
//
// Requires Node 22+ (global fetch + WebSocket). No npm dependencies.
//
// Output JSON records endpoint, page URL, mode label, per-take marks/latency and
// p50/p95/p99. Raw marks are kept so a reviewer can re-derive every number.

const DEFAULTS = {
  endpoint: 'http://127.0.0.1:9222',
  match: '',
  takes: 30,
  queries: 'index.js,report,project phoenix,café,会議議事録,zzqx-none',
  warmup: 3,
  timeoutMs: 5000,
  out: '',
};

function parseArgs(argv) {
  const args = { ...DEFAULTS };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => {
      const v = argv[++i];
      if (v === undefined) throw new Error('missing value for ' + a);
      return v;
    };
    switch (a) {
      case '--endpoint': args.endpoint = next(); break;
      case '--match': args.match = next(); break;
      case '--takes': args.takes = parseInt(next(), 10); break;
      case '--warmup': args.warmup = parseInt(next(), 10); break;
      case '--queries': args.queries = next(); break;
      case '--timeout-ms': args.timeoutMs = parseInt(next(), 10); break;
      case '--out': args.out = next(); break;
      case '--list': args.list = true; break;
      case '--help': case '-h': args.help = true; break;
      default: throw new Error('unknown argument ' + a);
    }
  }
  return args;
}

class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.seq = 0;
    this.pending = new Map();
    ws.addEventListener('message', (ev) => {
      let msg;
      try { msg = JSON.parse(ev.data); } catch { return; }
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(JSON.stringify(msg.error)));
        else resolve(msg.result);
      }
    });
  }
  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', resolve, { once: true });
      ws.addEventListener('error', () => reject(new Error('CDP websocket error: ' + url)), { once: true });
    });
    return new Cdp(ws);
  }
  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.seq;
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression, awaitPromise = false) {
    const result = await this.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise,
      userGesture: true,
    });
    if (result.exceptionDetails) {
      throw new Error('page exception: ' + JSON.stringify(result.exceptionDetails));
    }
    return result.result ? result.result.value : undefined;
  }
  close() { try { this.ws.close(); } catch { /* ignore */ } }
}

const INSTALL = String.rawString.raw`(() => {
  if (window.__PS_LATENCY__) return 'already-installed';
  const state = window.__PS_LATENCY__ = { marks: [], pending: null };
  window.addEventListener('keydown', () => {
    if (state.pending) state.pending.marks.push({ name: 'ps:keydown', t: performance.now() });
  }, true);
  window.__PS_LATENCY_BEGIN__ = (timeoutMs) => {
    const input = document.querySelector(
      'input[type="search"], input[role="combobox"], input[placeholder], input, textarea'
    );
    const container = document.querySelector('[data-results], [role="listbox"], main, body');
    if (!input || !container) return { ok: false, reason: 'input or results container not found' };
    input.focus();
    input.select && input.select();
    const count = () => container.querySelectorAll('[role="option"], [data-result], [data-result-row], li, article, button').length;
    const startCount = count();
    const p = { marks: [{ name: 'ps:measure-start', t: performance.now() }], done: false, result: null };
    state.pending = p;
    const finish = (result) => {
      if (p.done) return;
      p.done = true;
      obs.disconnect();
      clearTimeout(timer);
      state.pending = null;
      p.result = result;
    };
    const obs = new MutationObserver(() => {
      const n = count();
      if (n === startCount) return;
      p.marks.push({ name: 'ps:results-committed', t: performance.now(), count: n });
      requestAnimationFrame(() => requestAnimationFrame(() => {
        p.marks.push({ name: 'ps:first-paint', t: performance.now() });
        finish({ ok: true, marks: p.marks, startCount, count: n });
      }));
    });
    obs.observe(container, { childList: true, subtree: true, characterData: true, attributes: false });
    const timer = setTimeout(() => finish({ ok: false, marks: p.marks, reason: 'timeout waiting for result mutation' }), timeoutMs);
    return { ok: true, startCount };
  };
  window.__PS_LATENCY_RESULT__ = () => (state.pending ? state.pending.result : state.lastResult);
  window.__PS_LATENCY_WAIT__ = () => new Promise((resolve) => {
    const p = state.pending;
    if (!p) return resolve(null);
    const tick = () => { if (p.result) resolve(p.result); else setTimeout(tick, 4); };
    tick();
  });
  return 'installed';
})()`;

function percentile(sorted, pct) {
  if (!sorted.length) return 0;
  const rank = Math.ceil((pct / 100) * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(sorted.length - 1, rank))];
}

function summarize(values) {
  if (!values.length) return { count: 0, p50: 0, p95: 0, p99: 0, min: 0, max: 0, mean: 0 };
  const s = [...values].sort((a, b) => a - b);
  const mean = s.reduce((a, b) => a + b, 0) / s.length;
  return { count: s.length, p50: percentile(s, 50), p95: percentile(s, 95), p99: percentile(s, 99), min: s[0], max: s[s.length - 1], mean };
}

function modeLabel(url) {
  if (!url) return 'unknown';
  if (url.startsWith('http://localhost') || url.startsWith('http://127.0.0.1')) return 'MOCK-BRIDGE (browser page; not product latency)';
  if (url.includes('tauri')) return 'TAURI';
  return 'unknown:' + url;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log('see header of tools/ui-latency/cdp-latency.mjs');
    return;
  }
  const listRes = await fetch(args.endpoint.replace(/\/$/, '') + '/json/list');
  const pages = (await listRes.json()).filter((p) => p.type === 'page');
  if (args.list) {
    for (const p of pages) console.log(p.url + '\n  ' + p.webSocketDebuggerUrl);
    return;
  }
  const chosen = pages.find((p) => (args.match ? p.url.includes(args.match) : true));
  if (!chosen) throw new Error('no page target found at ' + args.endpoint + ' (use --list)');
  const cdp = await Cdp.connect(chosen.webSocketDebuggerUrl);
  try {
    await cdp.send('Runtime.enable');
    await cdp.send('Page.enable');
    const installed = await cdp.evaluate(INSTALL);
    console.log('page:', chosen.url);
    console.log('mode:', modeLabel(chosen.url));
    console.log('instrumentation:', installed);

    const queries = args.queries.split(',').map((q) => q.trim()).filter(Boolean);
    const takes = [];
    const total = args.warmup + args.takes;
    for (let k = 0; k < total; k++) {
      const query = queries[k % queries.length];
      const begin = await cdp.evaluate('window.__PS_LATENCY_BEGIN__(' + args.timeoutMs + ')');
      if (!begin || !begin.ok) throw new Error('could not begin take: ' + JSON.stringify(begin));
      // Type the query via real CDP keyboard events so keydown handlers fire.
      for (const ch of query) {
        await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', text: ch, key: ch });
        await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: ch });
      }
      const result = await cdp.evaluate('window.__PS_LATENCY_WAIT__()', true);
      if (!result || !result.ok) {
        takes.push({ take: k, query, ok: false, reason: result ? result.reason : 'no result', warmup: k < args.warmup });
        continue;
      }
      const marks = result.marks;
      const keydown = marks.find((m) => m.name === 'ps:keydown');
      const paint = marks.find((m) => m.name === 'ps:first-paint');
      const committed = marks.find((m) => m.name === 'ps:results-committed');
      takes.push({
        take: k,
        query,
        ok: !!keydown && !!paint,
        warmup: k < args.warmup,
        ms: keydown && paint ? paint.t - keydown.t : null,
        commitMs: keydown && committed ? committed.t - keydown.t : null,
        resultCount: result.count,
        marks,
      });
    }

    const measured = takes.filter((t) => t.ok && !t.warmup).map((t) => t.ms);
    const stats = summarize(measured);
    const report = {
      schema: 'ps-ui-latency/1',
      kind: 'input-to-first-visible-result (CDP)',
      capturedAtUtc: new Date().toISOString(),
      endpoint: args.endpoint,
      pageUrl: chosen.url,
      mode: modeLabel(chosen.url),
      queries,
      takesRequested: args.takes,
      warmup: args.warmup,
      stats,
      notes: [
        'measures keydown -> double-rAF after result DOM mutation inside the page',
        'engine-only time is NOT included/inferred; use Rust SearchResponse.took_ms separately',
        'hotkey -> window visible requires a Rust-side mark and is not measured here',
        'mock-bridge numbers are not product latency and must not be published as such',
      ],
      takes,
    };
    trackedJson = JSON.stringify(report, null, 2);
    if (args.out) {
      const fs = await import('node:fs');
      fs.writeFileSync(args.out, trackedJson);
      console.log('wrote', args.out);
    } else {
      console.log(JSON.stringify(report, null, 2));
    }
    console.log('input -> first visible result: p50=' + stats.p50.toFixed(2) + 'ms p95=' + stats.p95.toFixed(2) + 'ms p99=' + stats.p99.toFixed(2) + 'ms n=' + stats.count);
  } finally {
    cdp.close();
  }
}

main().catch((err) => {
  console.error('cdp-latency: ' + (err && err.stack ? err.stack : err));
  process.exit(1);
});
