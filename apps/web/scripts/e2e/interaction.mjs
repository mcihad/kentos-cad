// Plays the interaction traces (fixtures/interaction/v1, docs/adr/0018) in the
// real app: its own Vite server, headless Chrome, real mouse and keyboard
// events. After each step the expectations are read through the dev-only
// `window.kentos` handle. The desktop plays the same files natively once its
// drawing area and tool session exist; a trace is the behaviour both keep.
//
//   pnpm e2e:interaction [trace-id…]      (CHROME_BIN overrides the browser binary)
import http from 'node:http';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep } from './cdp.mjs';

const DIR = new URL('../../../../fixtures/interaction/v1/', import.meta.url).pathname;
const only = process.argv.slice(2);
const traces = readdirSync(DIR)
  .filter((f) => f.endsWith('.json'))
  .sort()
  .map((f) => JSON.parse(readFileSync(join(DIR, f), 'utf8')))
  .filter((t) => !only.length || only.includes(t.id));
for (const t of traces) {
  if (t.format !== 'kentos.interaction-trace' || t.version !== 1) throw new Error(`${t.id}: not a v1 interaction trace`);
}
if (!traces.length) throw new Error(`no trace matches ${only.join(', ')}`);

// A stand-in for the KentOS API, so the dev server's /v1 forwarding does not log refused connections.
const api = http.createServer((req, res) => {
  if (req.url !== '/v1/health') return res.writeHead(404).end();
  res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify({ status: 'ok', service: 'kentos-api', version: '0.0.0-e2e', contracts: 1 }));
});
await new Promise((r) => api.listen(0, '127.0.0.1', r));
process.env.KENTOS_API_PORT = String(api.address().port);

// No file watching or hot reload: a file saved while the traces run must not reload the page under them.
const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const b = await launch(server.resolvedUrls.local[0]);

// ── Keyboard ────────────────────────────────────────────────────────────
// A trace names characters (what the keyboard produces), not key positions;
// these are the key events a US layout sends for them.
const NAMED = {
  Enter: { key: 'Enter', code: 'Enter', vk: 13, text: '\r' },
  Esc: { key: 'Escape', code: 'Escape', vk: 27 },
  Tab: { key: 'Tab', code: 'Tab', vk: 9 },
  Backspace: { key: 'Backspace', code: 'Backspace', vk: 8 },
  Space: { key: ' ', code: 'Space', vk: 32, text: ' ' },
};
const SYMBOLS = {
  '.': { code: 'Period', vk: 190 },
  ',': { code: 'Comma', vk: 188 },
  ';': { code: 'Semicolon', vk: 186 },
  '-': { code: 'Minus', vk: 189 },
  '+': { code: 'NumpadAdd', vk: 107 },
  '@': { code: 'Digit2', vk: 50, shift: true },
  '<': { code: 'IntlBackslash', vk: 226 },
  ' ': { code: 'Space', vk: 32 },
};

function keyFor(ch) {
  if (NAMED[ch]) return NAMED[ch];
  if (/^\d$/.test(ch)) return { key: ch, code: `Digit${ch}`, vk: 48 + Number(ch), text: ch };
  if (SYMBOLS[ch]) return { key: ch, text: ch, ...SYMBOLS[ch] };
  if (/^\p{L}$/u.test(ch)) {
    // An option letter is typed without Shift; the app compares upper case.
    const lower = ch.toLocaleLowerCase('tr-TR');
    const ascii = lower.normalize('NFD').replace(/\p{M}/gu, '').replace('ı', 'i').toUpperCase();
    return { key: lower, code: `Key${ascii}`, vk: ascii.charCodeAt(0), text: lower };
  }
  throw new Error(`no key for “${ch}”`);
}

async function press(chord) {
  const parts = chord === '+' ? ['+'] : chord.split('+');
  const name = parts.pop();
  const ctrl = parts.includes('Ctrl');
  const alt = parts.includes('Alt');
  const spec = keyFor(name);
  const modifiers = (alt ? 1 : 0) | (ctrl ? 2 : 0) | (spec.shift ? 8 : 0);
  const text = ctrl || alt ? undefined : spec.text;
  const base = { key: spec.key, code: spec.code, windowsVirtualKeyCode: spec.vk, modifiers };
  await b.send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...base, text });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
  await sleep(30);
}

// ── Mouse ───────────────────────────────────────────────────────────────
let origin = { x: 0, y: 0 };
const toScreen = ([de, dn]) =>
  b.eval(`(() => { const k = window.kentos; const s = k.view.camera.worldToScreen({ x: ${origin.x + de}, y: ${origin.y + dn} }); const r = k.view.clientRect(); return [Math.round(s.x + r.left), Math.round(s.y + r.top)]; })()`);
const mouse = (type, x, y, extra = {}) => b.send('Input.dispatchMouseEvent', { type, x, y, button: 'none', ...extra });

async function click(at, button = 'left', clickCount = 1) {
  const [x, y] = await toScreen(at);
  await mouse('mouseMoved', x, y);
  await mouse('mousePressed', x, y, { button, clickCount });
  // A right press shorter than the hold that opens the command menu (RIGHT_HOLD_MS).
  if (button === 'right') await sleep(40);
  await mouse('mouseReleased', x, y, { button, clickCount });
  await sleep(40);
}

// ── Steps ───────────────────────────────────────────────────────────────
const FILE = 'iz.kcad';

async function setUp(t) {
  origin = { x: t.view.center[0], y: t.view.center[1] };
  const doc = readFileSync(join(DIR, t.document), 'utf8');
  await b.eval(`(() => {
    const k = window.kentos;
    k.tools.activate('select');
    if (!k.files.load(${JSON.stringify(doc)}, null)) throw new Error('${t.document} did not load');
    // Save and open write to memory; the drawing is asked about nowhere.
    const store = (window.__traceFiles = new Map());
    const handle = (name) => ({
      name,
      async getFile() { return new Blob([store.get(name) ?? '']); },
      async createWritable() {
        const parts = [];
        return { async write(d) { parts.push(typeof d === 'string' ? d : new TextDecoder().decode(d)); }, async close() { store.set(name, parts.join('')); } };
      },
    });
    k.files.picker = { async save() { return handle('${FILE}'); }, async open() { return handle('${FILE}'); } };
    k.files.ask = async () => 'drop';
    for (const [key, v] of Object.entries(${JSON.stringify(t.draft ?? {})})) k.settings[key].set(v);
    for (const [key, v] of Object.entries(${JSON.stringify(t.prefs ?? {})})) k.prefs[key].set(v);
    const c = k.view.camera;
    c.center = { x: ${origin.x}, y: ${origin.y} };
    c.scale = ${1 / t.view.metresPerPixel};
    c.panBy(0, 0);
    k.view.focus();
  })()`);
  await sleep(120);
}

async function saveAndReopen() {
  await press('Ctrl+S');
  await b.waitFor(`!window.kentos.files.busy.value && window.__traceFiles.has('${FILE}')`, 8000);
  await press('Ctrl+O');
  await sleep(60);
  await b.waitFor('!window.kentos.files.busy.value', 8000);
}

async function act(step) {
  if (step.run) return void (await b.eval(`window.kentos.commands.execute(${JSON.stringify(step.run)})`));
  if (step.key) return press(step.key);
  if (step.text !== undefined) {
    for (const ch of step.text) await press(ch);
    return;
  }
  if (step.move) {
    const [x, y] = await toScreen(step.move);
    await mouse('mouseMoved', x, y);
    return sleep(30);
  }
  if (step.click) return click(step.click);
  if (step.doubleClick) {
    await click(step.doubleClick, 'left', 1);
    return click(step.doubleClick, 'left', 2);
  }
  if (step.rightClick) return click(step.rightClick, 'right');
  if (step.focus === 'commandLine') {
    const [x, y] = await b.eval(`(() => { const r = document.querySelector('.cmdline__input').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    await mouse('mouseMoved', x, y);
    await mouse('mousePressed', x, y, { button: 'left', clickCount: 1 });
    await mouse('mouseReleased', x, y, { button: 'left', clickCount: 1 });
    return sleep(40);
  }
  if (step.saveAndReopen) return saveAndReopen();
  if (!step.expect) throw new Error(`unknown step ${JSON.stringify(step)}`);
}

/** What a trace can observe, read the way the app itself reads it. */
const observe = () =>
  b.eval(`(async () => {
    const k = window.kentos;
    const { parsePrompt } = await import('/src/ui/promptOptions.ts');
    const field = document.querySelector('.cursor-input');
    let newest = null;
    for (const e of k.doc.all()) if (!newest || e.id > newest.id) newest = e;
    return {
      tool: k.tools.activeId.value,
      points: k.tools.active.pointCount ?? 0,
      options: parsePrompt(k.tools.prompt.value).options.map((o) => o.key),
      dynamicInput: field && !field.hidden ? field.querySelector('input').value : null,
      commandLine: document.querySelector('.cmdline__input')?.value ?? null,
      entities: k.doc.size,
      newest: newest && { kind: newest.kind, pts: newest.pts ? newest.pts.map((p) => [p.x, p.y]) : null },
      canUndo: k.doc.canUndo.value,
      canRedo: k.doc.canRedo.value,
      dirty: k.doc.dirty.value,
      log: k.log.entries.value.at(-1)?.level ?? null,
      metresPerPixel: 1 / k.view.camera.scale,
    };
  })()`);

const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

/** Differences between what a step expects and what the app shows; empty when it matches. */
function compare(expect, got, t) {
  const bad = [];
  for (const [key, want] of Object.entries(expect)) {
    const have = got[key];
    if (key === 'metresPerPixel') {
      if (Math.abs(have - want) > want * 1e-9) bad.push(`${key}: ${have}, beklenen ${want}`);
    } else if (key === 'newest') {
      const shape = have && { kind: have.kind, pts: have.pts?.map(([x, y]) => [x - origin.x, y - origin.y]) };
      if (!shape || shape.kind !== want.kind) bad.push(`newest: ${JSON.stringify(shape?.kind ?? null)}, beklenen ${want.kind}`);
      else if (want.points) {
        const near = shape.pts?.length === want.points.length && shape.pts.every(([x, y], i) => Math.hypot(x - want.points[i][0], y - want.points[i][1]) <= t.clickTolerance);
        if (!near) bad.push(`newest.points: ${JSON.stringify(shape.pts)}, beklenen ${JSON.stringify(want.points)} (±${t.clickTolerance} m)`);
      }
      if (shape && want.edges) {
        // Typed values are exact: consecutive vertex differences, not rounded.
        const edges = have.pts.slice(1).map(([x, y], i) => [x - have.pts[i][0], y - have.pts[i][1]]);
        if (!same(edges, want.edges)) bad.push(`newest.edges: ${JSON.stringify(edges)}, beklenen ${JSON.stringify(want.edges)}`);
      }
    } else if (!same(have, want)) bad.push(`${key}: ${JSON.stringify(have)}, beklenen ${JSON.stringify(want)}`);
  }
  return bad;
}

let failed = 0;
try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 20000);
  await sleep(1200); // first-load dependency optimisation can reload once
  await b.waitFor(ready, 20000);
  for (const t of traces) {
    await setUp(t);
    const problems = [];
    for (const [i, step] of t.steps.entries()) {
      await act(step);
      if (!step.expect) continue;
      const bad = compare(step.expect, await observe(), t);
      if (bad.length) problems.push(`  adım ${i + 1} ${JSON.stringify(Object.fromEntries(Object.entries(step).filter(([k]) => k !== 'expect' && k !== 'note')))}: ${bad.join('; ')}`);
    }
    console.log(`${problems.length ? '✗' : '✓'} ${t.id}: ${t.title}`);
    for (const p of problems) console.log(p);
    if (problems.length) failed++;
  }
  const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
  if (errors.length) {
    console.log(`Sayfada hata:\n  ${errors.join('\n  ')}`);
    failed++;
  }
} finally {
  b.close();
  await server.close();
  api.close();
}
console.log(failed ? `${failed} iz geçmedi.` : `${traces.length} iz geçti.`);
process.exit(failed ? 1 : 0);
