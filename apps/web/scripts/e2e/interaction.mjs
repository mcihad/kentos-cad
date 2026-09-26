// Plays the interaction traces (fixtures/interaction/v1, docs/adr/0018) in the
// real app: its own Vite server, headless Chrome, real mouse and keyboard
// events. After each step the expectations are read through the dev-only
// `window.kentos` handle. The desktop plays the same files natively once its
// drawing area and tool session exist; a trace is the behaviour both keep.
//
// Every trace runs once per variant (the §5 acceptance variants): the key
// events a US and a Turkish Q keyboard send, and a 2× (HiDPI) screen.
//
//   pnpm e2e:interaction [trace-id…] [--variant=us|tr-q|hidpi]   (CHROME_BIN overrides the browser binary)
import http from 'node:http';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep } from './cdp.mjs';

const DIR = new URL('../../../../fixtures/interaction/v1/', import.meta.url).pathname;
const args = process.argv.slice(2);
const only = args.filter((a) => !a.startsWith('--'));
const VARIANTS = [
  { id: 'us', layout: 'us', dpr: 1 },
  { id: 'tr-q', layout: 'tr-q', dpr: 1 },
  { id: 'hidpi', layout: 'us', dpr: 2 },
];
const wanted = args.filter((a) => a.startsWith('--variant=')).map((a) => a.slice('--variant='.length));
const variants = VARIANTS.filter((v) => !wanted.length || wanted.includes(v.id));
if (!variants.length) throw new Error(`no variant ${wanted.join(', ')}; there are ${VARIANTS.map((v) => v.id).join(', ')}`);
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
// each layout says which key events produce them.
const NAMED = {
  Enter: { key: 'Enter', code: 'Enter', vk: 13, text: '\r' },
  Esc: { key: 'Escape', code: 'Escape', vk: 27 },
  Tab: { key: 'Tab', code: 'Tab', vk: 9 },
  Backspace: { key: 'Backspace', code: 'Backspace', vk: 8 },
  Space: { key: ' ', code: 'Space', vk: 32, text: ' ' },
  Delete: { key: 'Delete', code: 'Delete', vk: 46 },
  F3: { key: 'F3', code: 'F3', vk: 114 },
  F8: { key: 'F8', code: 'F8', vk: 119 },
};
const LAYOUTS = {
  us: {
    '.': { code: 'Period', vk: 190 },
    ',': { code: 'Comma', vk: 188 },
    ';': { code: 'Semicolon', vk: 186 },
    '-': { code: 'Minus', vk: 189 },
    '+': { code: 'NumpadAdd', vk: 107 },
    '@': { code: 'Digit2', vk: 50, shift: true },
    '<': { code: 'IntlBackslash', vk: 226 },
    ' ': { code: 'Space', vk: 32 },
  },
  // Turkish Q: + is Shift+4, - sits right of *, @ is AltGr+Q. Windows
  // reports AltGr as Ctrl+Alt, so that is what the page receives.
  'tr-q': {
    '.': { code: 'Slash', vk: 191 },
    ',': { code: 'Backslash', vk: 220 },
    ';': { code: 'Backslash', vk: 220, shift: true },
    '-': { code: 'Equal', vk: 187 },
    '+': { code: 'Digit4', vk: 52, shift: true },
    '@': { code: 'KeyQ', vk: 81, altGr: true },
    '<': { code: 'IntlBackslash', vk: 226 },
    ' ': { code: 'Space', vk: 32 },
    ı: { code: 'KeyI', vk: 73 },
    i: { code: 'Quote', vk: 222 },
    ş: { code: 'Semicolon', vk: 186 },
    ğ: { code: 'BracketLeft', vk: 219 },
    ü: { code: 'BracketRight', vk: 221 },
    ö: { code: 'Comma', vk: 188 },
    ç: { code: 'Period', vk: 190 },
  },
};
let layout = LAYOUTS.us;

function keyFor(ch) {
  if (NAMED[ch]) return NAMED[ch];
  if (/^\d$/.test(ch)) return { key: ch, code: `Digit${ch}`, vk: 48 + Number(ch), text: ch };
  if (layout[ch]) return { key: ch, text: ch, ...layout[ch] };
  if (/^\p{L}$/u.test(ch)) {
    // An option letter is typed without Shift; the app compares upper case.
    const lower = ch.toLocaleLowerCase('tr-TR');
    if (layout[lower]) return { key: lower, text: lower, ...layout[lower] };
    const ascii = lower.normalize('NFD').replace(/\p{M}/gu, '').replace('ı', 'i').toUpperCase();
    return { key: lower, code: `Key${ascii}`, vk: ascii.charCodeAt(0), text: lower };
  }
  throw new Error(`no key for “${ch}”`);
}

async function press(chord) {
  const parts = chord === '+' ? ['+'] : chord.split('+');
  const name = parts.pop();
  let spec = keyFor(name);
  const altGr = Boolean(spec.altGr);
  const ctrl = parts.includes('Ctrl') || altGr;
  const alt = parts.includes('Alt') || altGr;
  // Shift+letter (Shift+H, Ctrl+Shift+V): the keyboard gives the capital, the layout's way.
  if (parts.includes('Shift') && /^\p{L}$/u.test(spec.key)) {
    const capital = spec.key.toLocaleUpperCase(layout === LAYOUTS['tr-q'] ? 'tr-TR' : 'en-US');
    spec = { ...spec, key: capital, text: capital, shift: true };
  }
  const modifiers = (alt ? 1 : 0) | (ctrl ? 2 : 0) | (spec.shift ? 8 : 0);
  // A chord types nothing; AltGr types its character.
  const text = (ctrl || alt) && !altGr ? undefined : spec.text;
  const base = { key: spec.key, code: spec.code, windowsVirtualKeyCode: spec.vk, modifiers };
  await b.send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...base, text });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
  await sleep(30);
}

// ── Mouse ───────────────────────────────────────────────────────────────
let origin = { x: 0, y: 0 };
/**
 * The page pixel of a trace point. A point off the drawing (under the ribbon
 * or a panel) would silently miss the canvas, so it stops the trace instead.
 */
async function toScreen([de, dn]) {
  const [x, y, inside] = await b.eval(`(() => {
    const k = window.kentos;
    const s = k.view.camera.worldToScreen({ x: ${origin.x + de}, y: ${origin.y + dn} });
    const r = k.view.clientRect();
    const x = Math.round(s.x + r.left);
    const y = Math.round(s.y + r.top);
    return [x, y, document.elementFromPoint(x, y)?.tagName === 'CANVAS'];
  })()`);
  if (!inside) throw new Error(`[${de}, ${dn}] çizim alanının dışında (${x}, ${y} px); izin noktalarını README'deki kutuda tutun.`);
  return [x, y];
}
const mouse = (type, x, y, extra = {}) => b.send('Input.dispatchMouseEvent', { type, x, y, button: 'none', ...extra });
/** A step's held keys as the mouse events carry them (CDP: Shift is 8). */
const held = (step) => (step.shift ? 8 : 0);

async function click(at, button = 'left', clickCount = 1, modifiers = 0) {
  const [x, y] = await toScreen(at);
  await mouse('mouseMoved', x, y, { modifiers });
  await mouse('mousePressed', x, y, { button, clickCount, modifiers });
  // A right press shorter than the hold that opens the command menu (RIGHT_HOLD_MS).
  if (button === 'right') await sleep(40);
  await mouse('mouseReleased', x, y, { button, clickCount, modifiers });
  await sleep(40);
}

/** The left button down at `from`, moved through the middle to `to`, released there (a selection box). */
async function drag([from, to], modifiers = 0) {
  const [x1, y1] = await toScreen(from);
  const [x2, y2] = await toScreen(to);
  const [xm, ym] = await toScreen([(from[0] + to[0]) / 2, (from[1] + to[1]) / 2]);
  await mouse('mouseMoved', x1, y1, { modifiers });
  await mouse('mousePressed', x1, y1, { button: 'left', buttons: 1, clickCount: 1, modifiers });
  await mouse('mouseMoved', xm, ym, { button: 'left', buttons: 1, modifiers });
  await mouse('mouseMoved', x2, y2, { button: 'left', buttons: 1, modifiers });
  await mouse('mouseReleased', x2, y2, { button: 'left', buttons: 0, clickCount: 1, modifiers });
  await sleep(40);
}

// ── Steps ───────────────────────────────────────────────────────────────
const FILE = 'iz.kcad';

async function setUp(t) {
  origin = { x: t.view.center[0], y: t.view.center[1] };
  const doc = readFileSync(join(DIR, t.document), 'utf8');
  await b.eval(`(async () => {
    const k = window.kentos;
    k.tools.activate('select');
    // Nothing typed in an earlier trace carries over.
    const line = document.querySelector('.cmdline__input');
    if (line) line.value = '';
    // Loading waits for the objects' persistent ids (the formats worker, ADR 0014); the view is set after it.
    if (!(await k.files.load(${JSON.stringify(doc)}, null))) throw new Error('${t.document} did not load');
    // Save and open write to memory; the drawing is asked about nowhere.
    const store = (window.__traceFiles = new Map());
    // Bytes, not text: a drawing is saved as the binary KCAD v2 (docs/adr/0025).
    const handle = (name) => ({
      name,
      async getFile() { return store.get(name) ?? new Blob([]); },
      async createWritable() {
        const parts = [];
        return { async write(d) { parts.push(typeof d === 'string' ? new TextEncoder().encode(d) : new Uint8Array(d)); }, async close() { store.set(name, new Blob(parts)); } };
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
  if (step.click) return click(step.click, 'left', 1, held(step));
  if (step.drag) return drag(step.drag, held(step));
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
    // A path's corners; a line's two ends; a point's place; an arc's start and end (counter-clockwise, as stored).
    const pts = (e) => (e.pts ? e.pts.map((p) => [p.x, p.y]) : e.kind === 'line' ? [[e.a.x, e.a.y], [e.b.x, e.b.y]] : e.kind === 'point' ? [[e.p.x, e.p.y]] : e.kind === 'arc' ? [e.a0, e.a1].map((a) => [e.c.x + e.r * Math.cos(a), e.c.y + e.r * Math.sin(a)]) : null);
    const shape = (e) => ({ kind: e.kind, pts: pts(e), bulges: e.bulges ?? [], center: e.c ? [e.c.x, e.c.y] : null, radius: e.r ?? null });
    return {
      tool: k.tools.activeId.value,
      points: k.tools.active.pointCount ?? 0,
      options: parsePrompt(k.tools.prompt.value).options.map((o) => o.key),
      dynamicInput: field && !field.hidden ? field.querySelector('input').value : null,
      commandLine: document.querySelector('.cmdline__input')?.value ?? null,
      entities: k.doc.size,
      newest: newest && shape(newest),
      // Every object by its id (the \`objects\` expectation, docs/adr/0037).
      objects: Object.fromEntries([...k.doc.all()].map((e) => [e.id, shape(e)])),
      canUndo: k.doc.canUndo.value,
      canRedo: k.doc.canRedo.value,
      dirty: k.doc.dirty.value,
      log: k.log.entries.value.at(-1)?.level ?? null,
      metresPerPixel: 1 / k.view.camera.scale,
      viewCenter: [k.view.camera.center.x, k.view.camera.center.y],
      selected: [...k.selection.ids.value],
      hover: k.selection.hover.value,
      snap: k.view.currentSnap?.kind ?? null,
      ids: [...k.doc.all()].map((e) => e.id),
    };
  })()`);

const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

/** Differences between an object's expected shape and what it is (`newest`, `objects`): `name` prefixes them. */
function compareShape(name, have, want, t) {
  const bad = [];
  const shape = have && { kind: have.kind, pts: have.pts?.map(([x, y]) => [x - origin.x, y - origin.y]) };
  if (!shape || shape.kind !== want.kind) return [`${name}: ${JSON.stringify(shape?.kind ?? null)}, beklenen ${want.kind}`];
  if (want.points) {
    const near = shape.pts?.length === want.points.length && shape.pts.every(([x, y], i) => Math.hypot(x - want.points[i][0], y - want.points[i][1]) <= t.clickTolerance);
    if (!near) bad.push(`${name}.points: ${JSON.stringify(shape.pts)}, beklenen ${JSON.stringify(want.points)} (±${t.clickTolerance} m)`);
  }
  // A circle's or an arc's centre and radius: from clicks, within the click tolerance.
  if (want.center) {
    const c = have.center && [have.center[0] - origin.x, have.center[1] - origin.y];
    if (!c || Math.hypot(c[0] - want.center[0], c[1] - want.center[1]) > t.clickTolerance)
      bad.push(`${name}.center: ${JSON.stringify(c)}, beklenen ${JSON.stringify(want.center)} (±${t.clickTolerance} m)`);
  }
  if (want.radius !== undefined && (have.radius === null || Math.abs(have.radius - want.radius) > t.clickTolerance))
    bad.push(`${name}.radius: ${have.radius}, beklenen ${want.radius} (±${t.clickTolerance} m)`);
  if (want.arcs !== undefined) {
    const arcs = have.bulges.filter((bulge) => bulge !== 0).length;
    if (arcs !== want.arcs) bad.push(`${name}.arcs: ${arcs}, beklenen ${want.arcs}`);
  }
  if (want.edges) {
    // Typed values are exact: consecutive vertex differences, not rounded.
    const edges = have.pts.slice(1).map(([x, y], i) => [x - have.pts[i][0], y - have.pts[i][1]]);
    if (!same(edges, want.edges)) bad.push(`${name}.edges: ${JSON.stringify(edges)}, beklenen ${JSON.stringify(want.edges)}`);
  }
  return bad;
}

/** Differences between what a step expects and what the app shows; empty when it matches. */
function compare(expect, got, t) {
  const bad = [];
  for (const [key, want] of Object.entries(expect)) {
    const have = got[key];
    if (key === 'metresPerPixel') {
      if (Math.abs(have - want) > want * 1e-9) bad.push(`${key}: ${have}, beklenen ${want}`);
    } else if (key === 'viewCenter') {
      // Where Kaydır and the zooms put the view (docs/adr/0056): from clicks, within the click tolerance.
      const c = [have[0] - origin.x, have[1] - origin.y];
      if (Math.hypot(c[0] - want[0], c[1] - want[1]) > t.clickTolerance)
        bad.push(`${key}: ${JSON.stringify(c)}, beklenen ${JSON.stringify(want)} (±${t.clickTolerance} m)`);
    } else if (key === 'newest') bad.push(...compareShape('newest', have, want, t));
    else if (key === 'objects') for (const w of want) bad.push(...compareShape(`objects[${w.id}]`, have[w.id] ?? null, w, t));
    else if (!same(have, want)) bad.push(`${key}: ${JSON.stringify(have)}, beklenen ${JSON.stringify(want)}`);
  }
  return bad;
}

let failed = 0;
try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 20000);
  await sleep(1200); // first-load dependency optimisation can reload once
  await b.waitFor(ready, 20000);
  for (const v of variants) {
    layout = LAYOUTS[v.layout];
    await b.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: v.dpr, mobile: false });
    await sleep(300);
    for (const t of traces) {
      await setUp(t);
      const problems = [];
      for (const [i, step] of t.steps.entries()) {
        const label = `  adım ${i + 1} ${JSON.stringify(Object.fromEntries(Object.entries(step).filter(([k]) => k !== 'expect' && k !== 'note')))}`;
        try {
          await act(step);
        } catch (e) {
          // A step that cannot be played ends its trace; the others still run.
          problems.push(`${label}: ${e instanceof Error ? e.message : e}`);
          break;
        }
        if (!step.expect) continue;
        const bad = compare(step.expect, await observe(), t);
        if (bad.length) problems.push(`${label}: ${bad.join('; ')}`);
      }
      // The end state of each trace, to look at (scripts/e2e/out, not committed).
      if (v.id === 'us') await b.shot(`interaction-${t.id}`);
      console.log(`${problems.length ? '✗' : '✓'} [${v.id}] ${t.id}: ${t.title}`);
      for (const p of problems) console.log(p);
      if (problems.length) failed++;
    }
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
console.log(failed ? `${failed} iz geçmedi.` : `${traces.length} iz × ${variants.length} varyant geçti.`);
process.exit(failed ? 1 : 0);
