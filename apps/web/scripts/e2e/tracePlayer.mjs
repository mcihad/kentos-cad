// The step engine of the interaction traces (fixtures/interaction/v1, docs/adr/0018): the
// real app in its own Vite server, headless Chrome, real mouse and keyboard events, and the
// expectations read through the dev-only `window.kentos` handle. Two scripts play traces with
// it: interaction.mjs (the tests) and use.mjs (usage scenarios, pictured step by step).
// The desktop plays the same files natively (apps/desktop/src/traces/); a trace is the
// behaviour both keep.
//
// One browser at a time is attached (`attach`); the layout (US or Turkish Q keyboard) and the
// trace's origin are this module's state. interaction.mjs drives several browsers, each from a
// worker thread with its own instance of this module.
import http from 'node:http';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { sleep } from './cdp.mjs';

export const TRACE_DIR = new URL('../../../../fixtures/interaction/v1/', import.meta.url).pathname;

/** The §5 acceptance variants: the key events a US and a Turkish Q keyboard send, and a 2× (HiDPI) screen. */
export const VARIANTS = [
  { id: 'us', layout: 'us', dpr: 1 },
  { id: 'tr-q', layout: 'tr-q', dpr: 1 },
  { id: 'hidpi', layout: 'us', dpr: 2 },
];

/** The traces of the folder, all or the ones by these ids; every one a v1 trace. */
export function loadTraces(only = []) {
  const traces = readdirSync(TRACE_DIR)
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((f) => JSON.parse(readFileSync(join(TRACE_DIR, f), 'utf8')))
    .filter((t) => !only.length || only.includes(t.id));
  for (const t of traces) {
    if (t.format !== 'kentos.interaction-trace' || t.version !== 1) throw new Error(`${t.id}: not a v1 interaction trace`);
  }
  return traces;
}

/**
 * The app to play in: a stand-in for the KentOS API (so the dev server's /v1 forwarding does not log refused
 * connections) and a Vite server with no file watching or hot reload (a file saved while the traces run must
 * not reload the page under them). `url` is the app's; `close` stops both.
 */
export async function startApp() {
  const api = http.createServer((req, res) => {
    if (req.url !== '/v1/health') return res.writeHead(404).end();
    res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify({ status: 'ok', service: 'kentos-api', version: '0.0.0-e2e', contracts: 1 }));
  });
  await new Promise((r) => api.listen(0, '127.0.0.1', r));
  process.env.KENTOS_API_PORT = String(api.address().port);
  const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
  await server.listen();
  return {
    url: server.resolvedUrls.local[0],
    async close() {
      await server.close();
      api.close();
    },
  };
}

/** Waits until the app in the attached browser has started (the first load's dependency optimisation can reload once). */
export async function waitReady(browser) {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await browser.waitFor(ready, 20000);
  await sleep(1200);
  await browser.waitFor(ready, 20000);
}

/** The browser the steps are sent to. */
let b = null;
export function attach(browser) {
  b = browser;
}

/** The snap preferences as the settings schema starts them (docs/adr/0163), set before every trace. */
const SNAP_DEFAULTS = {
  snapEndpoint: true,
  snapMidpoint: true,
  snapCenter: true,
  snapNode: true,
  snapIntersection: true,
  snapPerpendicular: true,
  snapTangent: true,
  snapNearest: false,
  snapCentroid: false,
  snapExtension: false,
  snapParallel: false,
  snapGrid: false,
  snapGridEast: 1,
  snapGridNorth: 1,
  snapSelf: true,
  snapScaleMin: 0,
  snapScaleMax: 0,
};

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
    // A point's name starts with # (docs/adr/0152 §4).
    '#': { code: 'Digit3', vk: 51, shift: true },
    '<': { code: 'IntlBackslash', vk: 226 },
    '/': { code: 'Slash', vk: 191 },
    ' ': { code: 'Space', vk: 32 },
  },
  // Turkish Q: + is Shift+4, - sits right of *, / is Shift+7, @ is AltGr+Q, # AltGr+3. Windows
  // reports AltGr as Ctrl+Alt, so that is what the page receives.
  'tr-q': {
    '.': { code: 'Slash', vk: 191 },
    ',': { code: 'Backslash', vk: 220 },
    ';': { code: 'Backslash', vk: 220, shift: true },
    '-': { code: 'Equal', vk: 187 },
    '+': { code: 'Digit4', vk: 52, shift: true },
    '@': { code: 'KeyQ', vk: 81, altGr: true },
    '#': { code: 'Digit3', vk: 51, altGr: true },
    '<': { code: 'IntlBackslash', vk: 226 },
    '/': { code: 'Digit7', vk: 55, shift: true },
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
/** Types as this variant's keyboard does (`VARIANTS[].layout`). */
export function setLayout(id) {
  layout = LAYOUTS[id];
}

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
  // Shift held with a named key (Shift+F3) as well as the layout's own Shift (Turkish Q's + is Shift+4).
  const modifiers = (alt ? 1 : 0) | (ctrl ? 2 : 0) | (spec.shift || parts.includes('Shift') ? 8 : 0);
  // A chord types nothing; AltGr types its character.
  const text = (ctrl || alt) && !altGr ? undefined : spec.text;
  const base = { key: spec.key, code: spec.code, windowsVirtualKeyCode: spec.vk, modifiers };
  // A text field that has the focus takes an AltGr character in the browser on Windows; Chrome on Linux, sent
  // Ctrl+Alt with text, does not insert it. The page sees the key as it does on Windows, and the text is inserted
  // the way the browser would (the command line, after a value was accepted, keeps the focus). With the focus
  // elsewhere the page's own fallback types it (ui/bottom/CommandLine.ts), so the text goes with the key.
  const insert =
    altGr && (await b.eval(`(() => { const a = document.activeElement; return a instanceof HTMLInputElement || a instanceof HTMLTextAreaElement; })()`));
  if (insert) {
    await b.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base });
    await b.send('Input.insertText', { text: spec.text });
  } else await b.send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...base, text });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
  await sleep(30);
}

// ── Mouse ───────────────────────────────────────────────────────────────
let origin = { x: 0, y: 0 };
/** Where the trace last put the mouse, as a trace point; null before the first and while it is not on the drawing. */
let pointer = null;

/**
 * The mouse to where the trace last put it, by the drawing as it lies now. A picture that has the drawing area
 * change size (use.mjs opens and closes the history) would otherwise leave the mouse at its old pixel, which is
 * another place of the drawing: the crosshair, the preview and the hover card would show where the trace is not.
 */
export async function replacePointer() {
  if (!pointer) return;
  try {
    const [x, y] = await toScreen(pointer);
    await mouse('mouseMoved', x, y);
    await sleep(60);
  } catch {
    // The place is not on the drawing area as it is now: the mouse stays.
  }
}
/**
 * The page point of a trace point, not rounded to a whole pixel: the view's
 * centre may fall between pixels (a drawing area of odd size), and a rounded
 * pointer would land half a pixel off the point the trace names (0.0625 m at
 * 0.125 m/px), where the desktop's usage player (`kentos-cad kullan`) lands on
 * it; the desktop's tests round to device pixels, their expectations allowing
 * `clickTolerance`. Chrome carries the fraction to the pointer events the app reads. A
 * point off the drawing (under the ribbon or a panel) would silently miss
 * the canvas, so it stops the trace instead.
 */
async function toScreen([de, dn]) {
  const [x, y, inside] = await b.eval(`(() => {
    const k = window.kentos;
    const s = k.view.camera.worldToScreen({ x: ${origin.x + de}, y: ${origin.y + dn} });
    const r = k.view.clientRect();
    const x = s.x + r.left;
    const y = s.y + r.top;
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
  pointer = null;
  const doc = readFileSync(join(TRACE_DIR, t.document), 'utf8');
  // A file of the traces' folder the open dialog answers with (Metin dosyası yerleştir's, docs/adr/0145 §6).
  const opened = t.openFile ? [...readFileSync(join(TRACE_DIR, t.openFile))] : null;
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
    const opened = ${JSON.stringify(opened)};
    if (opened) k.files.picker.open = async () => ({ name: ${JSON.stringify(t.openFile ?? '')}, async getFile() { return new Blob([new Uint8Array(opened)]); } });
    k.files.ask = async () => 'drop';
    // Topological editing and the overlap control are off unless the trace turns them on, as the desktop's player
    // reads a missing key (docs/adr/0160, 0162): an earlier trace's never carries over. The chosen layers are a set.
    k.settings.topology.set(false);
    k.settings.topologyPoints.set(false);
    k.settings.overlap.set('allow');
    k.settings.overlapLast.set('layer');
    k.settings.overlapLayers.set(new Set());
    // The snap kinds and the snap's other preferences are the settings' first values unless the trace's prefs say
    // otherwise (docs/adr/0163): a kind an earlier trace turned on with its command never carries over.
    for (const [key, v] of Object.entries(${JSON.stringify(SNAP_DEFAULTS)})) k.prefs[key].set(v);
    for (const [key, v] of Object.entries(${JSON.stringify(t.draft ?? {})})) k.settings[key].set(key === 'overlapLayers' ? new Set(v) : v);
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

/** The title of the window on top, or null: the \`dialog\` a step sees. */
const TOP_TITLE = `([...document.querySelectorAll('.dialog-backdrop .dialog__title')].at(-1)?.textContent ?? null)`;

/**
 * A control of the window on top by its words (a field's or a box's label, a button's text), brought into view:
 * where to click it, whether it is checked and whether it is off; null when the window has none.
 */
const control = (selector, words) =>
  b.eval(`(() => {
    const d = [...document.querySelectorAll('.dialog-backdrop .dialog')].at(-1);
    const words = (el) => (el.getAttribute('aria-label') ?? el.closest('label')?.textContent ?? el.textContent ?? '').trim();
    const el = d && [...d.querySelectorAll(${JSON.stringify(selector)})].find((el) => words(el) === ${JSON.stringify(words)});
    if (!el) return null;
    el.scrollIntoView({ block: 'nearest' });
    const r = el.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2, checked: el.checked ?? null };
  })()`);

/** Ctrl+A in the field that has the focus: all its text chosen (the editing command goes with the key, as CDP needs it). */
async function chooseAll() {
  const base = { key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: 2 };
  await b.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base, commands: ['selectAll'] });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
}

/** A left click at a page point (a window's control). */
async function clickAt({ x, y }) {
  await mouse('mouseMoved', x, y);
  await mouse('mousePressed', x, y, { button: 'left', clickCount: 1 });
  await mouse('mouseReleased', x, y, { button: 'left', clickCount: 1 });
  await sleep(40);
}

/**
 * A \`dialog\` step: waits for the window titled so (a window is loaded when first opened), then, as a user does
 * with the mouse and the keyboard, types over its fields (\`fill\`, by label: clicked, all chosen, the text typed),
 * sets its check boxes (\`check\`, by their words) and presses its button (\`press\`, by its words). A button that
 * is off takes the click and does nothing, as it would. The desktop answers the same controls (answers.rs).
 */
async function answer(step) {
  try {
    await b.waitFor(`${TOP_TITLE} === ${JSON.stringify(step.dialog)}`, 8000);
  } catch {
    const open = await b.eval(TOP_TITLE);
    throw new Error(`“${step.dialog}” penceresi açık değil (açık: ${open === null ? 'yok' : `“${open}”`})`);
  }
  const missing = (what) => new Error(`“${step.dialog}” penceresinde ${what} yok`);
  for (const [label, text] of Object.entries(step.fill ?? {})) {
    const at = await control('input, textarea', label);
    if (!at) throw missing(`“${label}” alanı`);
    await clickAt(at);
    await chooseAll();
    if (text) await b.send('Input.insertText', { text });
    else await press('Delete');
    await sleep(40);
  }
  for (const [words, on] of Object.entries(step.check ?? {})) {
    const at = await control('input[type=checkbox]', words);
    if (!at) throw missing(`“${words}” kutusu`);
    if (at.checked !== on) await clickAt(at);
  }
  if (step.press !== undefined) {
    const at = await control('button', step.press);
    if (!at) throw missing(`“${step.press}” düğmesi`);
    await clickAt(at);
  }
  await sleep(60);
}

async function act(step) {
  // A picture is asked for (`shot`): no action, no expectation.
  if (step.shot !== undefined) return;
  if (step.dialog !== undefined) return answer(step);
  if (step.run) return void (await b.eval(`window.kentos.commands.execute(${JSON.stringify(step.run)})`));
  if (step.key) return press(step.key);
  if (step.text !== undefined) {
    for (const ch of step.text) {
      // A capital typed into a text field other than the command line (Yazı's field over the drawing) is Shift and
      // the letter, as a keyboard types it; on the drawing an option letter goes without Shift (Shift+H is Kaydır).
      const capital =
        /^\p{Lu}$/u.test(ch) &&
        (await b.eval(`(() => { const a = document.activeElement; return a instanceof HTMLInputElement && !a.classList.contains('cmdline__input'); })()`));
      await press(capital ? `Shift+${ch.toLocaleLowerCase('tr-TR')}` : ch);
    }
    return;
  }
  if (step.move) {
    const [x, y] = await toScreen(step.move);
    await mouse('mouseMoved', x, y);
    pointer = step.move;
    return sleep(30);
  }
  if (step.rest) {
    // Resting on a snap past the tracking dwell (350 ms, TRACK_DWELL_MS) acquires or releases it.
    const [x, y] = await toScreen(step.rest);
    await mouse('mouseMoved', x, y);
    pointer = step.rest;
    return sleep(500);
  }
  if (step.click) {
    await click(step.click, 'left', 1, held(step));
    pointer = step.click;
    return;
  }
  if (step.drag) {
    await drag(step.drag, held(step));
    pointer = step.drag[1];
    return;
  }
  if (step.doubleClick) {
    await click(step.doubleClick, 'left', 1);
    await click(step.doubleClick, 'left', 2);
    pointer = step.doubleClick;
    return;
  }
  if (step.rightClick) {
    await click(step.rightClick, 'right');
    pointer = step.rightClick;
    return;
  }
  if (step.focus === 'commandLine') {
    pointer = null;
    const [x, y] = await b.eval(`(() => { const r = document.querySelector('.cmdline__input').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
    await mouse('mouseMoved', x, y);
    await mouse('mousePressed', x, y, { button: 'left', clickCount: 1 });
    await mouse('mouseReleased', x, y, { button: 'left', clickCount: 1 });
    return sleep(40);
  }
  if (step.saveAndReopen) return saveAndReopen();
  if (!step.expect) throw new Error(`unknown step ${JSON.stringify(step)}`);
}

/** What a trace can observe, read the way the app itself reads it; `mark`: the newest message before the step. */
const observe = (mark) =>
  b.eval(`(async () => {
    const k = window.kentos;
    const { parsePrompt } = await import('/src/ui/promptOptions.ts');
    const field = document.querySelector('.cursor-input');
    let newest = null;
    for (const e of k.doc.all()) if (!newest || e.id > newest.id) newest = e;
    // A path's corners; a line's two ends; a point's place; an arc's start and end (counter-clockwise, as stored).
    // An ellipse's axis ends (major, minor, counter-clockwise) or an elliptical arc's start and end; a
    // construction line's point and one metre along it (docs/adr/0057). A spline's fit points are its pts.
    // A dimension's measured points, and an angle's vertex (docs/adr/0061). An insert's point, and where the
    // definition's points one metre east and north of its base go: turn, scale and mirror (docs/adr/0144).
    const placed = (e) => {
      const [c, s, k] = [Math.cos(e.rotation), Math.sin(e.rotation), e.mirror ? -1 : 1];
      return [[e.p.x, e.p.y], [e.p.x + e.scale * c, e.p.y + e.scale * s], [e.p.x - e.scale * k * s, e.p.y + e.scale * k * c]];
    };
    const along = (e, t) => [e.c.x + e.major.x * Math.cos(t) - e.major.y * e.ratio * Math.sin(t), e.c.y + e.major.y * Math.cos(t) + e.major.x * e.ratio * Math.sin(t)];
    const ellipse = (e) => (e.t0 === e.t1 ? [0, 1, 2, 3].map((i) => along(e, (i * Math.PI) / 2)) : [along(e, e.t0), along(e, e.t1)]);
    const pts = (e) => (e.pts ? e.pts.map((p) => [p.x, p.y]) : e.kind === 'line' ? [[e.a.x, e.a.y], [e.b.x, e.b.y]] : e.kind === 'point' ? [[e.p.x, e.p.y]] : e.kind === 'arc' ? [e.a0, e.a1].map((a) => [e.c.x + e.r * Math.cos(a), e.c.y + e.r * Math.sin(a)]) : e.kind === 'ellipse' ? ellipse(e) : e.kind === 'xline' || e.kind === 'ray' ? [[e.p.x, e.p.y], [e.p.x + e.dir.x, e.p.y + e.dir.y]] : e.kind === 'dimension' ? [e.a, e.b, ...(e.c ? [e.c] : [])].map((p) => [p.x, p.y]) : e.kind === 'insert' ? placed(e) : e.kind === 'text' ? [[e.p.x, e.p.y]] : null);
    const shape = (e) => ({
      kind: e.kind,
      pts: pts(e),
      bulges: e.bulges ?? [],
      center: e.c ? [e.c.x, e.c.y] : null,
      radius: e.r ?? null,
      text: e.kind === 'text' ? e.text : e.kind === 'leader' ? (e.text ?? null) : null,
      // A text's alignment (null: the left of the baseline), width factor and mask (docs/adr/0145).
      align: e.kind === 'text' ? (e.align ?? null) : null,
      widthFactor: e.kind === 'text' ? (e.widthFactor ?? 1) : null,
      mask: e.kind === 'text' || e.kind === 'leader' || e.kind === 'dimension' ? e.mask === true : null,
      rotation: e.kind === 'text' || e.kind === 'leader' ? e.rotation : null,
      // A leader's arrowhead (null: the filled arrow; docs/adr/0146).
      arrow: e.kind === 'leader' ? (e.arrow ?? null) : null,
      // A dimension's direction in degrees: a linear one's measured, an ordinate's axis (docs/adr/0147).
      angle: e.kind === 'dimension' ? (e.angle ?? null) : null,
      // The text beside it (a survey point's name), its attributes, a point's elevation (docs/adr/0152).
      label: e.label ?? null,
      attrs: e.attrs ?? {},
      z: e.kind === 'point' ? (e.z ?? null) : null,
      // The outer path's vertex elevations, a line's two ends' (docs/adr/0142, 0160).
      zs: e.kind === 'line' ? [e.za ?? null, e.zb ?? null] : e.kind === 'polygon' || e.kind === 'polyline' ? (e.zs ?? e.pts.map(() => null)) : [],
    });
    return {
      tool: k.tools.activeId.value,
      points: k.tools.active.pointCount ?? 0,
      options: parsePrompt(k.tools.prompt.value).options.map((o) => o.key),
      prompt: k.tools.prompt.value,
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
      // The texts of the messages the step wrote (the \`logged\` expectation).
      logged: k.log.entries.value.filter((e) => e.id > ${mark}).map((e) => e.text),
      metresPerPixel: 1 / k.view.camera.scale,
      viewCenter: [k.view.camera.center.x, k.view.camera.center.y],
      selected: [...k.selection.ids.value],
      hover: k.selection.hover.value,
      snap: k.view.currentSnap?.kind ?? null,
      // Object tracking: the acquired points, and the alignment the cursor is locked to.
      trackPoints: k.view.trackPoints.map((p) => [p.x, p.y]),
      track: (() => {
        const t = k.view.currentTrack;
        return t && { point: [t.point.x, t.point.y], lines: t.lines.map((l) => ({ origin: [l.origin.x, l.origin.y], angle: l.angle })) };
      })(),
      ids: [...k.doc.all()].map((e) => e.id),
      dialog: ${TOP_TITLE},
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
  // A text's content is exact (docs/adr/0144 §7: an exploded attribute's value).
  if (want.text !== undefined && have.text !== want.text) bad.push(`${name}.text: ${JSON.stringify(have.text)}, beklenen ${JSON.stringify(want.text)}`);
  // A text's alignment, width factor, mask and turn (docs/adr/0145), exact.
  // A survey point's name and elevation (docs/adr/0152), exact.
  for (const key of ['align', 'widthFactor', 'mask', 'rotation', 'arrow', 'label', 'z'])
    if (want[key] !== undefined && have[key] !== want[key]) bad.push(`${name}.${key}: ${JSON.stringify(have[key])}, beklenen ${JSON.stringify(want[key])}`);
  // Its attributes, all of them, exact.
  const sorted = (o) => JSON.stringify(Object.entries(o).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  if (want.attrs !== undefined && sorted(have.attrs) !== sorted(want.attrs)) bad.push(`${name}.attrs: ${JSON.stringify(have.attrs)}, beklenen ${JSON.stringify(want.attrs)}`);
  // A dimension's direction (docs/adr/0147), within 1e-9: a typed angle in grads comes back through radians.
  if (want.angle !== undefined && (have.angle === null || Math.abs(have.angle - want.angle) > 1e-9))
    bad.push(`${name}.angle: ${JSON.stringify(have.angle)}, beklenen ${want.angle}`);
  // A shared arc and an elevation along an edge (docs/adr/0160) come from the core's arithmetic: within 1e-9.
  if (want.bulges !== undefined) {
    const near = want.bulges.every((b, i) => Math.abs((have.bulges[i] ?? 0) - b) <= 1e-9) && have.bulges.slice(want.bulges.length).every((b) => b === 0);
    if (!near) bad.push(`${name}.bulges: ${JSON.stringify(have.bulges)}, beklenen ${JSON.stringify(want.bulges)} (±1e-9)`);
  }
  if (want.zs !== undefined) {
    const near = have.zs.length === want.zs.length && have.zs.every((z, i) => (z === null || want.zs[i] === null ? z === want.zs[i] : Math.abs(z - want.zs[i]) <= 1e-9));
    if (!near) bad.push(`${name}.zs: ${JSON.stringify(have.zs)}, beklenen ${JSON.stringify(want.zs)} (±1e-9)`);
  }
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

/** Differences between the tracking lock and the expected one: the point and the lines' origins within the click tolerance, angles exact. */
function compareTrack(have, want, t) {
  const rel = (p) => [p[0] - origin.x, p[1] - origin.y];
  const seen = have && { point: rel(have.point), lines: have.lines.map((l) => ({ origin: rel(l.origin), angle: l.angle })) };
  if (!want || !seen) return !want === !seen ? [] : [`track: ${JSON.stringify(seen)}, beklenen ${JSON.stringify(want)}`];
  const near = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]) <= t.clickTolerance;
  const ok =
    near(seen.point, want.point) &&
    seen.lines.length === want.lines.length &&
    seen.lines.every((l, i) => near(l.origin, want.lines[i].origin) && l.angle === want.lines[i].angle);
  return ok ? [] : [`track: ${JSON.stringify(seen)}, beklenen ${JSON.stringify(want)} (±${t.clickTolerance} m)`];
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
    else if (key === 'trackPoints') {
      const pts = have.map(([x, y]) => [x - origin.x, y - origin.y]);
      const near = pts.length === want.length && pts.every(([x, y], i) => Math.hypot(x - want[i][0], y - want[i][1]) <= t.clickTolerance);
      if (!near) bad.push(`trackPoints: ${JSON.stringify(pts)}, beklenen ${JSON.stringify(want)} (±${t.clickTolerance} m)`);
    } else if (key === 'track') bad.push(...compareTrack(have, want, t));
    else if (key === 'logged') {
      // Each text, whole, in this order among the step's messages; others may come between.
      let n = 0;
      for (const text of have) if (n < want.length && text === want[n]) n++;
      if (n < want.length) bad.push(`logged: ${JSON.stringify(have)}, beklenen sırasıyla ${JSON.stringify(want)}`);
    }
    else if (!same(have, want)) bad.push(`${key}: ${JSON.stringify(have)}, beklenen ${JSON.stringify(want)}`);
  }
  return bad;
}

/**
 * Plays a trace from its start: its set-up, then its steps. A `shot` step is no action and no expectation:
 * `onShot(name, step)` is called for it when given (the usage scenarios take a picture there), else it is
 * passed over. Returns the problems, each naming its step; a step that cannot be played ends the trace.
 */
export async function play(t, { onShot } = {}) {
  await setUp(t);
  const problems = [];
  for (const [i, step] of t.steps.entries()) {
    if (step.shot !== undefined) {
      await onShot?.(step.shot, step);
      continue;
    }
    const label = `  adım ${i + 1} ${JSON.stringify(Object.fromEntries(Object.entries(step).filter(([k]) => k !== 'expect' && k !== 'note')))}`;
    const mark = await b.eval('window.kentos.log.entries.value.at(-1)?.id ?? 0');
    try {
      await act(step);
    } catch (e) {
      problems.push(`${label}: ${e instanceof Error ? e.message : e}`);
      break;
    }
    if (!step.expect) continue;
    // A window opens once its module has loaded (CLAUDE.md §20) and closes at once: the step waits for it.
    if ('dialog' in step.expect) await b.waitFor(`${TOP_TITLE} === ${JSON.stringify(step.expect.dialog)}`, 8000).catch(() => {});
    const bad = compare(step.expect, await observe(mark), t);
    if (bad.length) problems.push(`${label}: ${bad.join('; ')}`);
  }
  return problems;
}
