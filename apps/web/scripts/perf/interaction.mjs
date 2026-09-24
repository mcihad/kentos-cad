// Interaction measurement (CLAUDE.md §6.1, docs/adr/0005): the baseline the
// Rust geometry store (docs/adr/0008, slice S1) is compared against. Starts
// its own Vite dev server and one headless Chrome (WebGL2), builds the ADR
// 0005 data sets inside the page from a fixed seed, loads each with
// `kentos.doc.replaceWith` (no history) and times:
//   - per pointer move (real Input.dispatchMouseEvent mouseMoved events along
//     a seeded path): picking with the select tool (hover), object snap with
//     the line tool after its first point, edge picking with the trim tool;
//     at a working zoom (screen 1:1000) and at the overview (zoom extents),
//   - per frame: the overlay and its labels while panning (middle button),
//     the trim tool's preview, and rebuilding the big layer after a style
//     change.
// Times are main-thread milliseconds from the dev-only probe in
// src/viewport/ViewportController.ts (ViewportProbe); the GPU's own work is
// not in them. Chrome draws on the machine's GPU (ANGLE on OpenGL): with the
// SwiftShader software GPU a full frame of these data sets takes seconds, the
// frames queue up and block the page long after (up to a minute per move),
// so the harness refuses it unless --allow-swiftshader. Every run reloads the
// page; the report gives each percentile's median over the runs and the p95
// spread. Nothing else heavy may run meanwhile (docs/adr/0005). Start-up is
// measured by startup.mjs, not here.
//
// Writes docs/perf/interaction-<label>.{json,md}. A label other than
// "baseline" is compared with docs/perf/interaction-baseline.json when it
// exists (or with --compare <file>).
//
//   node scripts/perf/interaction.mjs [--label latest] [--runs 3] [--out docs/perf]
//     [--datasets parsel-50k,hat-1m] [--scenarios line-close,rebuild] [--scale 1]
//     [--compare <file>] [--memory-limit 3500] [--allow-swiftshader]
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { execFileSync, execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { cpus, loadavg, totalmem } from 'node:os';
import { relative, resolve } from 'node:path';
import { launch, sleep } from '../e2e/cdp.mjs';
import { datasets } from './datasets.mjs';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const label = opt('label', 'latest');
const runs = Number(opt('runs', '3'));
const outDir = resolve(opt('out', fileURLToPath(new URL('../../../../docs/perf', import.meta.url))));
const scale = Number(opt('scale', '1'));
const memoryLimitMb = Number(opt('memory-limit', '3500'));
const allowSwiftShader = args.includes('--allow-swiftshader');
const only = opt('datasets', 'parsel-50k,hat-1m').split(',');
/** A subset of scenarios (e.g. `line-close,rebuild`) while investigating one; the baseline runs them all. */
const scenarioFilter = opt('scenarios', null)?.split(',') ?? null;
const wanted = (id) => !scenarioFilter || scenarioFilter.includes(id);
const DATASETS = datasets(scale);
const compareFile = opt('compare', label === 'baseline' ? null : resolve(outDir, 'interaction-baseline.json'));

/** Screen scale of the working zoom (the cadastral drawing scale; 96 dpi as in the status bar). */
const WORK_DENOMINATOR = 1000;
const PX_M = 0.00026458;
/** Pointer moves per sequence, pan steps, layer rebuilds per run (trim moves are per data set: its preview can take a second a frame). */
const MOVES = 240;
const WARM_MOVES = 24;
const PAN_STEPS = 80;
const REBUILDS = 6;
/** A sequence stops early after this long (the slowest data set on a slow machine); the report says how many events it got. */
const SEQUENCE_CAP_MS = 120_000;

// ── Pointer paths (seeded; screen px inside the free part of the canvas) ──

function mulberry32(seed) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A hand-like wander: steps of 6–24 px, heading drifting, bouncing off the edges of `r`. */
function pointerPath(n, seed, r) {
  const rand = mulberry32(seed);
  let x = (r.x0 + r.x1) / 2;
  let y = (r.y0 + r.y1) / 2;
  let a = rand() * Math.PI * 2;
  const out = [];
  for (let i = 0; i < n; i++) {
    a += (rand() - 0.5) * 0.9;
    const step = 6 + rand() * 18;
    x += Math.cos(a) * step;
    y += Math.sin(a) * step;
    if (x < r.x0 || x > r.x1) {
      a = Math.PI - a;
      x = Math.min(r.x1, Math.max(r.x0, x));
    }
    if (y < r.y0 || y > r.y1) {
      a = -a;
      y = Math.min(r.y1, Math.max(r.y0, y));
    }
    out.push([Math.round(x), Math.round(y)]);
  }
  return out;
}

// ── Statistics ──────────────────────────────────────────────────────────

/** Nearest-rank percentiles (a p95 is a value that was measured). */
function summarize(values) {
  const s = values.filter(Number.isFinite).sort((a, b) => a - b);
  if (!s.length) return null;
  const q = (p) => s[Math.min(s.length - 1, Math.max(0, Math.ceil(p * s.length) - 1))];
  return { n: s.length, p50: q(0.5), p95: q(0.95), p99: q(0.99), max: s.at(-1), mean: s.reduce((a, b) => a + b, 0) / s.length };
}

const median = (xs) => {
  const s = xs.filter(Number.isFinite).sort((a, b) => a - b);
  if (!s.length) return null;
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};

// ── Machine and browser helpers ─────────────────────────────────────────

/** Proportional set size (MB) of a process and its descendants (Chrome's GPU, renderer and utility processes). */
function treePssMb(rootPid) {
  const kids = new Map();
  for (const line of execFileSync('ps', ['-e', '-o', 'pid=,ppid='], { encoding: 'utf8' }).trim().split('\n')) {
    const [pid, ppid] = line.trim().split(/\s+/).map(Number);
    if (!kids.has(ppid)) kids.set(ppid, []);
    kids.get(ppid).push(pid);
  }
  let kb = 0;
  const stack = [rootPid];
  while (stack.length) {
    const pid = stack.pop();
    stack.push(...(kids.get(pid) ?? []));
    try {
      const m = /^Pss:\s+(\d+)/m.exec(readFileSync(`/proc/${pid}/smaps_rollup`, 'utf8'));
      if (m) kb += Number(m[1]);
    } catch {}
  }
  return kb / 1024;
}

const alive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

// ── Measurement ─────────────────────────────────────────────────────────

// The app talks to no API (a closed port: "Sunucu: yok"), so no server traffic runs meanwhile.
process.env.KENTOS_API_PORT = '9';
// Cross-origin isolation gives performance.now() 5 µs steps instead of 100 µs; no watching or hot reload.
const server = await createServer({
  server: { port: 0, strictPort: false, hmr: false, watch: null, headers: { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' } },
  logLevel: 'error',
});
await server.listen();
const url = server.resolvedUrls.local[0];
// The machine's GPU through ANGLE's OpenGL backend (cdp.mjs starts SwiftShader; a later flag wins). Frames keep
// Chrome's own 60 Hz pace: an unpaced compositor spins a thread and slowed the page ~10 % in trials.
const CHROME_ARGS = allowSwiftShader ? [] : ['--use-angle=gl', '--ignore-gpu-blocklist'];
const b = await launch('about:blank', { width: 1600, height: 900, args: CHROME_ARGS });
const chromePid = b.pid;
let peakMb = 0;
/** Peak since the current data set was loaded (reset per data set). */
let datasetPeakMb = 0;
let memoryError = null;
const memoryTimer = setInterval(() => {
  // A browser that died leaves CDP calls unanswered: stop instead of waiting forever.
  if (!alive(chromePid)) {
    console.error('Chrome beklenmedik biçimde kapandı.');
    clearInterval(memoryTimer);
    server.close().finally(() => process.exit(1));
    return;
  }
  const mb = treePssMb(chromePid);
  peakMb = Math.max(peakMb, mb);
  datasetPeakMb = Math.max(datasetPeakMb, mb);
  if (mb > memoryLimitMb && !memoryError) memoryError = `Chrome ${Math.round(mb)} MB kullanıyor (sınır ${memoryLimitMb} MB); veri seti bu makine için büyük, --scale ile küçültün.`;
}, 1000);

const settle = () => b.eval('new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)))');
let gcFailed = false;
const gc = () =>
  b.send('HeapProfiler.collectGarbage').catch((e) => {
    if (!gcFailed) console.warn(`  uyarı: çöp toplama zorlanamadı (${e.message})`);
    gcFailed = true;
  });
const startProbe = () => b.eval('void (window.kentos.view.probe = { moves: [], frames: [], overlay: { labels: 0, tool: 0 } })');
const stopProbe = () => b.eval('(() => { const p = window.kentos.view.probe; window.kentos.view.probe = undefined; return p; })()');
const checkMemory = () => {
  if (memoryError) throw new Error(memoryError);
};

async function openApp() {
  await b.eval('window.__perfOld = true').catch(() => {});
  await b.send('Page.navigate', { url });
  const ready = '!window.__perfOld && window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 60000);
  await sleep(800);
  await b.waitFor(ready, 20000);
  // Timer-driven helpers depend on how long events take (resting 350 ms acquires a tracking
  // point, 500 ms opens the hover card): off, so a faster core does not do different work.
  // The pointer stays clear of the toolbox (left) and of the command bar a running command shows at the top.
  return b.eval(`(() => {
    const k = window.kentos;
    k.settings.tracking.set(false);
    k.prefs.hoverInfo.set(false);
    const r = k.view.clientRect();
    const tb = document.querySelector('.toolbox')?.getBoundingClientRect();
    const left = tb && tb.right > r.left && tb.left < r.right && tb.bottom > r.top ? tb.right - r.left + 16 : 16;
    let top = 16;
    for (const tool of ['line', 'trim']) {
      k.tools.activate(tool);
      k.tools.active.acceptPoint?.(k.view.camera.center);
      const bar = document.querySelector('.cmdbar');
      if (bar && !bar.hidden) top = Math.max(top, bar.getBoundingClientRect().bottom - r.top + 16);
    }
    k.tools.activate('select');
    let res = Infinity;
    for (let i = 0, last = performance.now(); i < 1e6 && res > 0.001; i++) {
      const t = performance.now();
      if (t > last) { res = Math.min(res, t - last); last = t; }
    }
    const gl = document.querySelector('.viewport__gl')?.getContext('webgl2');
    const info = gl?.getExtension('WEBGL_debug_renderer_info');
    const glRenderer = gl ? String(gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER)) : null;
    return { canvas: { x: r.left, y: r.top, w: r.width, h: r.height }, free: { x0: left, y0: Math.round(top), x1: r.width - 16, y1: r.height - 16 }, backend: k.view.backendKind.value, glRenderer, isolated: crossOriginIsolated, dpr: devicePixelRatio, timerMs: res };
  })()`);
}

/** Every path point must land on the drawing canvas with the tool's command bar showing. */
async function checkPath(env, path, what) {
  const bad = await b.eval(`${JSON.stringify(path)}.filter(([x, y]) => document.elementFromPoint(${env.canvas.x} + x, ${env.canvas.y} + y) !== document.querySelector('.viewport__overlay')).length`);
  if (bad) throw new Error(`${what}: ${bad} imleç noktası çizim alanının dışında ya da bir panelin altında.`);
}

async function loadDataset(name) {
  const d = DATASETS[name];
  await gc();
  const info = await b.eval(`(() => {
    const k = window.kentos;
    const build = ${d.build.toString()};
    const t0 = performance.now();
    const data = build(${JSON.stringify(d.params)});
    const t1 = performance.now();
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const e of data.entities) for (const q of e.pts) { minX = Math.min(minX, q.x); minY = Math.min(minY, q.y); maxX = Math.max(maxX, q.x); maxY = Math.max(maxY, q.y); }
    const bounds = { minX, minY, maxX, maxY };
    k.view.probe = { moves: [], frames: [], overlay: { labels: 0, tool: 0 } };
    k.doc.replaceWith({
      name: ${JSON.stringify(`perf-${name}`)},
      settings: k.doc.settings.toJSON(),
      origin: { x: Math.round((minX + maxX) / 200) * 100, y: Math.round((minY + maxY) / 200) * 100 },
      homeView: bounds,
      layers: data.layers,
      activeLayer: 'taslak',
      entities: data.entities,
      styles: { items: [], categories: [] },
    });
    const t2 = performance.now();
    k.selection.clear();
    return { generateMs: t1 - t0, replaceMs: t2 - t1, entities: k.doc.size, bigLayer: data.bigLayer, count: data.count, bounds };
  })()`);
  // The first frame rebuilds every layer.
  await settle();
  await settle();
  const probe = await stopProbe();
  const first = probe.frames.find((f) => f.build > 0) ?? probe.frames[0];
  await b.send('Performance.enable');
  const metrics = Object.fromEntries((await b.send('Performance.getMetrics')).metrics.map((m) => [m.name, m.value]));
  await b.send('Performance.disable');
  return { ...info, firstBuildMs: first?.build ?? null, firstFrameMs: first ? first.build + first.render + first.overlay : null, jsHeapMb: Math.round((metrics.JSHeapUsedSize ?? 0) / 1e6), chromeMb: Math.round(treePssMb(chromePid)) };
}

/** Working zoom around the data set's centre, or zoom extents; returns the scale, what is visible and where the data lies on screen. */
async function setZoom(zoom, bounds) {
  const info = await b.eval(`(() => {
    const v = window.kentos.view;
    const c = v.camera;
    const d = ${JSON.stringify(bounds)};
    if (${JSON.stringify(zoom)} === 'overview') v.zoomExtents();
    else {
      const s = 1 / (${WORK_DENOMINATOR} * ${PX_M});
      const w = c.width / s, h = c.height / s;
      const x = (d.minX + d.maxX) / 2, y = (d.minY + d.maxY) / 2;
      c.fit({ minX: x - w / 2, minY: y - h / 2, maxX: x + w / 2, maxY: y + h / 2 }, 0);
    }
    const a = c.worldToScreen({ x: d.minX, y: d.maxY });
    const z = c.worldToScreen({ x: d.maxX, y: d.minY });
    return { pxPerM: c.scale, denominator: Math.round(1 / (c.scale * ${PX_M})), visible: v.entitiesIn(c.visibleBounds()).length, data: { x0: a.x, y0: a.y, x1: z.x, y1: z.y } };
  })()`);
  await settle();
  await settle();
  return info;
}

/** One real mouse move; CDP answers once the page has handled it and drawn its frame. */
const moveTo = (env, [x, y], extra = {}) => b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: env.canvas.x + x, y: env.canvas.y + y, button: 'none', buttons: 0, ...extra });

/** Each measured move must reach the viewport as its own event (not merged, not lost) and draw its own frame. */
function checkMoves(what, sent, probe) {
  if (probe.moves.length !== sent) throw new Error(`${what}: ${sent} hareket gönderildi, ${probe.moves.length} ölçüldü (ViewportProbe eksik ya da olaylar birleşti).`);
  if (probe.frames.length < sent) console.warn(`  uyarı: ${what}: ${sent} harekette ${probe.frames.length} kare çizildi.`);
}

/** Pointer moves with a tool; the line tool gets its first point at the path's start. */
async function moveScenario(env, tool, path, warm) {
  await b.eval(`(() => { const k = window.kentos; k.tools.activate('select'); k.selection.clear(); k.tools.activate(${JSON.stringify(tool)}); })()`);
  await moveTo(env, path[0]);
  if (tool === 'line') {
    const [x, y] = path[0];
    await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: env.canvas.x + x, y: env.canvas.y + y, button: 'left', buttons: 1, clickCount: 1 });
    await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: env.canvas.x + x, y: env.canvas.y + y, button: 'left', buttons: 0, clickCount: 1 });
  }
  await settle();
  await checkPath(env, [...path, ...warm], tool);
  // The warm-up ends elsewhere, so the first measured move really moves (a move to where the pointer is may not fire).
  for (const pt of warm) await moveTo(env, pt);
  await settle();
  await gc();
  await startProbe();
  const t0 = Date.now();
  let sent = 0;
  // For trim, whether each move left an edge under the cursor, i.e. whether its frame drew a preview.
  const hovered = [];
  for (const pt of path) {
    await moveTo(env, pt);
    if (tool === 'trim') hovered.push(await b.eval('!!window.kentos.tools.active.hover'));
    sent++;
    checkMemory();
    if (Date.now() - t0 > SEQUENCE_CAP_MS) break;
  }
  await settle();
  const wallMs = Date.now() - t0;
  const probe = await stopProbe();
  const state = await b.eval(`(() => { const k = window.kentos; const s = { tool: k.tools.activeId.value, points: k.tools.active.pts?.length ?? null, size: k.doc.size }; k.tools.activate('select'); return s; })()`);
  if (state.tool !== tool) throw new Error(`${tool} aracı ölçüm sırasında kapandı (${state.tool}).`);
  if (tool === 'line' && state.points !== 1) throw new Error(`Çizgi aracında ${state.points} nokta var, 1 bekleniyordu.`);
  checkMoves(tool, sent, probe);
  return { planned: path.length, sent, wallMs, size: state.size, moves: probe.moves, frames: probe.frames, hovered: tool === 'trim' ? hovered : null };
}

/** Middle-button drag: there and back in a zigzag, one frame per step. */
async function panScenario(env) {
  await b.eval(`window.kentos.tools.activate('select')`);
  const f = env.free;
  let x = Math.round((f.x0 + f.x1) / 2) - 200;
  let y = Math.round((f.y0 + f.y1) / 2) - 80;
  await moveTo(env, [x, y]);
  await settle();
  await gc();
  const at = () => ({ x: env.canvas.x + x, y: env.canvas.y + y });
  await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', ...at(), button: 'middle', buttons: 4, clickCount: 1 });
  await settle();
  await startProbe();
  const t0 = Date.now();
  for (let i = 0; i < PAN_STEPS; i++) {
    const sign = i < PAN_STEPS / 2 ? 1 : -1;
    x += 10 * sign;
    y += (i % 8 < 4 ? 4 : -2) * sign;
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', ...at(), button: 'middle', buttons: 4 });
    checkMemory();
  }
  await settle();
  const wallMs = Date.now() - t0;
  const probe = await stopProbe();
  await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', ...at(), button: 'middle', buttons: 0, clickCount: 1 });
  await settle();
  checkMoves('pan', PAN_STEPS, probe);
  return { pan: true, planned: PAN_STEPS, sent: PAN_STEPS, wallMs, moves: probe.moves, frames: probe.frames };
}

/** A style change on the big layer rebuilds it (and only it) in the next frame. */
async function rebuildScenario(layer) {
  const frames = [];
  const t0 = Date.now();
  for (let i = 0; i < REBUILDS; i++) {
    await gc();
    await startProbe();
    await b.eval(`window.kentos.doc.setLayerStyle(${JSON.stringify(layer)}, { color: ${JSON.stringify(i % 2 ? '#8C9AAA' : '#AA9A8C')} }, 'Ölçüm')`);
    await settle();
    const probe = await stopProbe();
    const f = probe.frames.find((q) => q.build > 0);
    if (!f) throw new Error(`${layer} katmanı yeniden kurulmadı.`);
    frames.push(f);
    checkMemory();
  }
  return { planned: REBUILDS, sent: REBUILDS, wallMs: Date.now() - t0, frames };
}

/** Per-metric summaries of one scenario: moves (whole handler, snap, tool) and frames. */
function scenarioMetrics(r) {
  const out = {};
  if (r.moves?.length) {
    out['move.total'] = summarize(r.moves.map((m) => m.total));
    out['move.snap'] = summarize(r.moves.map((m) => m.snap));
    out['move.tool'] = summarize(r.moves.map((m) => m.tool));
  }
  if (r.frames?.length) {
    out['frame.cpu'] = summarize(r.frames.map((f) => f.build + f.render + f.overlay));
    for (const k of ['build', 'render', 'overlay', 'labels', 'tool']) out[`frame.${k}`] = summarize(r.frames.map((f) => f[k]));
    // Wall time from one frame's start to the next while panning: the frame rate reached, the GPU's work included.
    // (Elsewhere the harness itself sets the pace: a read after each trim move, a pause between rebuilds.)
    if (r.pan && r.frames.length > 1 && Number.isFinite(r.frames[0].at)) out['frame.interval'] = summarize(r.frames.slice(1).map((f, i) => f.at - r.frames[i].at));
    // Frame i belongs to move i (one frame per move, checked); only frames with an edge under the cursor draw a preview.
    if (r.hovered && r.frames.length === r.hovered.length) out['frame.preview'] = summarize(r.frames.filter((_, i) => r.hovered[i]).map((f) => f.tool));
  }
  return out;
}

const results = []; // { run, dataset, env, load, zooms, scenarios: { id: { sent, wallMs, metrics } } }
const startedAt = Date.now();
const load0 = loadavg();
let env0 = null;
let failure = null;
try {
  await b.send('Network.enable');
  // Web fonts come from Google: blocked, so every run draws with the same local font and needs no network.
  await b.send('Network.setBlockedURLs', { urls: ['*fonts.googleapis.com*', '*fonts.gstatic.com*'] });
  for (let run = 1; run <= runs; run++)
    for (const name of only) {
      if (!DATASETS[name]) throw new Error(`Bilinmeyen veri seti: ${name}`);
      const env = await openApp();
      env0 ??= env;
      if (env.backend !== 'webgl2') throw new Error(`Çizim motoru ${env.backend}; ölçüm WebGL2 ister.`);
      if (/swiftshader/i.test(env.glRenderer ?? '') && !allowSwiftShader)
        throw new Error(`Chrome donanım GPU'su yerine yazılımla çiziyor (${env.glRenderer}). SwiftShader'da tek kare saniyeler sürer ve ölçüm dakikalarca takılır; yine de denemek için --allow-swiftshader.`);
      datasetPeakMb = 0;
      const load = await loadDataset(name);
      checkMemory();
      const r = { run, dataset: name, env, load, zooms: {}, scenarios: {} };
      const record = (id, s) => {
        if (s.size !== undefined && s.size !== load.entities) throw new Error(`${id}: çizimde ${s.size} nesne var, ${load.entities} bekleniyordu.`);
        const metrics = scenarioMetrics(s);
        r.scenarios[id] = { planned: s.planned, sent: s.sent, frames: s.frames.length, wallMs: s.wallMs, metrics };
        const p50 = (m) => (metrics[m] ? metrics[m].p50.toFixed(2) : '–');
        console.log(`  ${name} #${run} ${id}: ${s.sent} olay, ${(s.wallMs / 1000).toFixed(1)} s; p50 olay ${p50('move.total')} ms, kare ${p50('frame.cpu')} ms`);
      };
      for (const zoom of ['close', 'overview']) {
        const z = (r.zooms[zoom] = await setZoom(zoom, load.bounds));
        // The pointer wanders over the data set only: at the overview it does not fill the canvas.
        const f = env.free;
        const area = { x0: Math.max(f.x0, z.data.x0 + 8), y0: Math.max(f.y0, z.data.y0 + 8), x1: Math.min(f.x1, z.data.x1 - 8), y1: Math.min(f.y1, z.data.y1 - 8) };
        z.pointerArea = area;
        const path = pointerPath(MOVES, 7, area);
        const warm = pointerPath(WARM_MOVES, 11, area);
        if (wanted(`select-${zoom}`)) record(`select-${zoom}`, await moveScenario(env, 'select', path, warm));
        if (wanted(`line-${zoom}`)) record(`line-${zoom}`, await moveScenario(env, 'line', path, warm));
        if (zoom === 'close' && wanted('trim-close')) record('trim-close', await moveScenario(env, 'trim', path.slice(0, DATASETS[name].trimMoves), warm.slice(0, 8)));
        if (wanted(`pan-${zoom}`)) record(`pan-${zoom}`, await panScenario(env));
      }
      await setZoom('close', load.bounds);
      if (wanted('rebuild')) record('rebuild', await rebuildScenario(load.bigLayer));
      r.peakChromeMb = Math.round(datasetPeakMb);
      results.push(r);
    }
  const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  if (errors.length) throw new Error(`Sayfada hata: ${errors.join(' | ')}`);
} catch (e) {
  failure = e;
} finally {
  clearInterval(memoryTimer);
  b.close();
  for (let i = 0; i < 50 && alive(chromePid); i++) await sleep(100);
  if (alive(chromePid)) process.kill(chromePid, 'SIGKILL');
  await server.close();
}
if (failure) {
  console.error(failure);
  process.exit(1);
}

// ── Report ──────────────────────────────────────────────────────────────

/** Metric key: data set / scenario / metric. */
const metrics = {};
for (const r of results)
  for (const [id, s] of Object.entries(r.scenarios))
    for (const [m, v] of Object.entries(s.metrics)) {
      if (!v) continue;
      const key = `${r.dataset}/${id}/${m}`;
      (metrics[key] ??= { perRun: [] }).perRun.push(v);
    }
for (const m of Object.values(metrics)) {
  const p95s = m.perRun.map((v) => v.p95);
  Object.assign(m, {
    n: m.perRun.reduce((s, v) => s + v.n, 0),
    p50: median(m.perRun.map((v) => v.p50)),
    p95: median(p95s),
    p95Range: [Math.min(...p95s), Math.max(...p95s)],
    p99: median(m.perRun.map((v) => v.p99)),
    max: Math.max(...m.perRun.map((v) => v.max)),
  });
}

const commit = execSync('git rev-parse --short HEAD').toString().trim();
const dirtyTree = execSync('git status --porcelain --untracked-files=no').toString().trim().length > 0;
const cpu = cpus()[0]?.model ?? '?';
const summaries = Object.fromEntries(
  only.map((name) => {
    const rs = results.filter((r) => r.dataset === name);
    const l = rs.map((r) => r.load);
    return [
      name,
      {
        title: DATASETS[name].title,
        params: DATASETS[name].params,
        entities: l[0]?.entities,
        count: l[0]?.count,
        bigLayer: l[0]?.bigLayer,
        bounds: l[0]?.bounds,
        zooms: rs[0]?.zooms,
        generateMs: median(l.map((x) => x.generateMs)),
        replaceMs: median(l.map((x) => x.replaceMs)),
        firstBuildMs: median(l.map((x) => x.firstBuildMs)),
        jsHeapMb: median(l.map((x) => x.jsHeapMb)),
        chromeMbAfterLoad: median(l.map((x) => x.chromeMb)),
      },
    ];
  }),
);
const report = {
  label,
  date: new Date().toISOString(),
  commit,
  dirtyTree,
  machine: { cpu, threads: cpus().length, memoryGb: Math.round(totalmem() / 2 ** 30), loadAverageAtStart: load0.map((v) => Math.round(v * 100) / 100), chrome: execFileSync(process.env.CHROME_BIN ?? 'google-chrome', ['--version']).toString().trim(), node: process.version },
  setup: {
    runs,
    scale,
    moves: MOVES,
    trimMoves: Object.fromEntries(only.map((name) => [name, DATASETS[name].trimMoves])),
    panSteps: PAN_STEPS,
    rebuilds: REBUILDS,
    workDenominator: WORK_DENOMINATOR,
    window: { width: 1600, height: 900 },
    canvas: env0?.canvas,
    pointerArea: env0?.free,
    backend: env0?.backend,
    glRenderer: env0?.glRenderer,
    chromeArgs: CHROME_ARGS,
    dpr: env0?.dpr,
    crossOriginIsolated: env0?.isolated,
    timerResolutionMs: env0?.timerMs,
    peakChromeMb: Math.round(peakMb),
    durationS: Math.round((Date.now() - startedAt) / 1000),
  },
  datasets: summaries,
  metrics,
  runs: results.map((r) => ({ run: r.run, dataset: r.dataset, load: r.load, zooms: r.zooms, peakChromeMb: r.peakChromeMb, scenarios: Object.fromEntries(Object.entries(r.scenarios).map(([id, s]) => [id, { planned: s.planned, sent: s.sent, frames: s.frames, wallMs: s.wallMs }])) })),
};
mkdirSync(outDir, { recursive: true });
writeFileSync(`${outDir}/interaction-${label}.json`, `${JSON.stringify(report, null, 2)}\n`);

// Markdown (Turkish, like the other reports; the decimal separator is a point, CLAUDE.md §5).
const ms = (v) => (v === null || v === undefined ? '–' : v < 10 ? v.toFixed(2) : v < 100 ? v.toFixed(1) : String(Math.round(v)));
const cell = (m) => (m ? `${ms(m.p50)} | ${ms(m.p95)} (${ms(m.p95Range[0])}–${ms(m.p95Range[1])}) | ${ms(m.p99)} | ${ms(m.max)}` : '– | – | – | –');
const get = (ds, id, m) => metrics[`${ds}/${id}/${m}`];
const ZOOM = { close: `yakın (1:${WORK_DENOMINATOR})`, overview: 'genel (tümü)' };
const moveRows = [
  ['select-close', 'Seç (üzerine gelme)', 'close', [['move.tool', 'seçme'], ['move.total', 'olayın tamamı']]],
  ['select-overview', 'Seç (üzerine gelme)', 'overview', [['move.tool', 'seçme'], ['move.total', 'olayın tamamı']]],
  ['line-close', 'Çizgi, ilk noktadan sonra', 'close', [['move.snap', 'kenet'], ['move.total', 'olayın tamamı']]],
  ['line-overview', 'Çizgi, ilk noktadan sonra', 'overview', [['move.snap', 'kenet'], ['move.total', 'olayın tamamı']]],
  ['trim-close', 'Buda', 'close', [['move.tool', 'kenar seçme'], ['move.total', 'olayın tamamı']]],
];
const frameRows = [
  ['pan-close', `Kaydırma, ${ZOOM.close}`, [['frame.overlay', 'üst katman'], ['frame.labels', 'etiketler'], ['frame.render', 'GPU gönderimi'], ['frame.cpu', 'kare (CPU)'], ['frame.interval', 'kare aralığı (GPU dahil)']]],
  ['pan-overview', `Kaydırma, ${ZOOM.overview}`, [['frame.overlay', 'üst katman'], ['frame.labels', 'etiketler'], ['frame.render', 'GPU gönderimi'], ['frame.cpu', 'kare (CPU)'], ['frame.interval', 'kare aralığı (GPU dahil)']]],
  ['trim-close', `Buda önizlemesi, ${ZOOM.close}`, [['frame.preview', 'önizleme (imleç bir kenardayken)'], ['frame.cpu', 'kare (CPU), bütün kareler']]],
  ['select-close', `Seç, ${ZOOM.close} (vurgu değişince tam çizim)`, [['frame.cpu', 'kare (CPU)']]],
  ['line-close', `Çizgi, ${ZOOM.close}`, [['frame.cpu', 'kare (CPU)']]],
  ['rebuild', 'Büyük katmanı yeniden kurma (stil değişikliği)', [['frame.build', 'kurma ve yükleme']]],
];
const n = (v) => (v === undefined || v === null ? '–' : v.toLocaleString('en-US').replaceAll(',', ' '));
const spreads = Object.entries(metrics)
  .filter(([, m]) => m.p95 >= 0.5)
  .map(([k, m]) => ({ k, rel: (m.p95Range[1] - m.p95Range[0]) / m.p95 }))
  .sort((a, b) => b.rel - a.rel);
const pct = (v) => `%${Math.round(v * 100)}`;
const md = [];
md.push(`# Etkileşim ölçümü: ${label} (${report.date.slice(0, 10)}, ${commit}${dirtyTree ? ', kaydedilmemiş değişiklikle' : ''})`);
md.push('');
md.push(
  `${cpu}, ${report.machine.threads} iş parçacığı, ${report.machine.memoryGb} GB; ${report.machine.chrome}; başsız, ${env0?.backend === 'webgl2' ? 'WebGL2' : env0?.backend} (${env0?.glRenderer ?? '?'}); Vite geliştirme sunucusu (\`window.kentos\`). Pencere 1600×900, çizim alanı ${env0?.canvas.w}×${env0?.canvas.h} CSS px, dpr ${env0?.dpr}. ${runs} koşu; her koşu sayfayı yeniden açar ve veri setini yeniden kurar. Toplam ${Math.round(report.setup.durationS / 60)} dk, Chrome en çok ${n(report.setup.peakChromeMb)} MB (PSS).`,
);
md.push('');
md.push(
  `Süreler ana iş parçacığında ms'dir (\`ViewportProbe\`, src/viewport/ViewportController.ts; yalnız geliştirme derlemesinde). Her hücre ${runs} koşunun ortancasıdır; p95'in yanında koşular arasındaki en düşük–en yüksek p95 yazar. Zamanlayıcı adımı ${env0 ? Math.round(env0.timerMs * 1000) : '–'} µs (${env0?.isolated ? 'cross-origin isolated' : 'isolated değil'}). Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir; ADR **taslaktır ve kullanıcı onayı bekler**, bu rapor yalnızca kayıttır.`,
);
md.push('');
md.push('## Veri setleri');
md.push('');
md.push('| Ad | İçerik | Nesne | Ham kenar | Üretme | `replaceWith` | İlk kare (bütün katmanlar) | JS yığını | Chrome (yüklemeden sonra) |');
md.push('|---|---|---|---|---|---|---|---|---|');
for (const [name, d] of Object.entries(summaries))
  md.push(`| \`${name}\` | ${d.title} | ${n(d.entities)} | ${n(d.count?.edges)} | ${ms(d.generateMs)} ms | ${ms(d.replaceMs)} ms | ${ms(d.firstBuildMs)} ms | ${n(d.jsHeapMb)} MB | ${n(d.chromeMbAfterLoad)} MB |`);
md.push('');
for (const [name, d] of Object.entries(summaries)) {
  const c = d.count ?? {};
  const z = d.zooms ?? {};
  const parts = name === 'parsel-50k' ? [`${n(c.parcels)} parsel (${n(c.bulged)} yaylı cephe, ${n(c.holed)} delikli), ${n(c.buildings)} yapı`] : [`${n(c.lines)} çoklu çizgi, hepsi tek katmanda`];
  md.push(`- \`${name}\`: ${parts.join('')}; büyük katman \`${d.bigLayer}\`. Yakın görünüm 1:${n(z.close?.denominator)}, ${n(z.close?.visible)} nesne görünür; genel görünüm 1:${n(z.overview?.denominator)}, ${n(z.overview?.visible)} nesne. Tohum ${d.params.seed}.`);
}
md.push('');
md.push('## İmleç hareketi (olay başına)');
md.push('');
md.push(`Aynı tohumlu ${MOVES} hareketlik yol (buda: ilk ${only.map((name) => `${DATASETS[name].trimMoves} (\`${name}\`)`).join(', ')}); her hareket kendi karesini çizer. “Seçme” ve “kenar seçme” aracın hareket işleyicisi, “kenet” nesne kenetidir; “olayın tamamı” \`pointermove\` işleyicisinin bütünüdür (durum çubuğu koordinatları dahil).`);
md.push('');
md.push('| Veri seti | Araç | Görünüm | Ölçüt | p50 | p95 (aralık) | p99 | en çok |');
md.push('|---|---|---|---|---|---|---|---|');
for (const name of Object.keys(summaries)) for (const [id, tool, zoom, ms_] of moveRows) for (const [m, what] of ms_) md.push(`| \`${name}\` | ${tool} | ${ZOOM[zoom]} | ${what} | ${cell(get(name, id, m))} |`);
md.push('');
md.push('## Kareler');
md.push('');
md.push(`Kaydırma orta tuşla ${PAN_STEPS} adımdır (gidip gelir); “GPU gönderimi” çizim çağrılarının ana iş parçacığındaki süresidir, GPU'nun kendi çizimi dahil değildir. “Kare aralığı” bir karenin başından sonrakinin başına geçen duvar saatidir: GPU'nun işi ve 60 Hz ekran temposu dahil, ulaşılan kare hızını gösterir (16.7 ms = 60 fps; tempo yüzünden 16.7'nin katlarına yakın çıkar). Yeniden kurma koşu başına ${REBUILDS} stil değişikliğidir.`);
md.push('');
md.push('| Veri seti | Senaryo | Ölçüt | p50 | p95 (aralık) | p99 | en çok |');
md.push('|---|---|---|---|---|---|---|');
for (const name of Object.keys(summaries)) for (const [id, what, ms_] of frameRows) for (const [m, part] of ms_) md.push(`| \`${name}\` | ${what} | ${part} | ${cell(get(name, id, m))} |`);
md.push('');
md.push('## ADR 0005 taslağıyla karşılaştırma (yalnızca kayıt)');
md.push('');
md.push('| Hedef (taslak) | Ölçülen (p95, koşuların ortancası) | Not |');
md.push('|---|---|---|');
const p95 = (ds, id, m) => ms(get(ds, id, m)?.p95);
if (summaries['parsel-50k']) {
  const e = n(summaries['parsel-50k'].entities);
  md.push(`| İmleç hareketinde seçme ve kenet: 100 bin nesnede olay başına < 2 ms | seçme ${p95('parsel-50k', 'select-close', 'move.tool')} / ${p95('parsel-50k', 'select-overview', 'move.tool')} ms; kenet ${p95('parsel-50k', 'line-close', 'move.snap')} / ${p95('parsel-50k', 'line-overview', 'move.snap')} ms (yakın / genel) | \`parsel-50k\`, ${e} nesne |`);
}
if (summaries['hat-1m']) {
  md.push(`| Kaydırma: 1 milyon segmentte kare ≤ 16 ms | kare (CPU) ${p95('hat-1m', 'pan-close', 'frame.cpu')} / ${p95('hat-1m', 'pan-overview', 'frame.cpu')} ms; kare aralığı ${p95('hat-1m', 'pan-close', 'frame.interval')} / ${p95('hat-1m', 'pan-overview', 'frame.interval')} ms (yakın / genel) | \`hat-1m\`; kare (CPU) yalnız ana iş parçacığıdır, kare aralığı bu makinenin GPU'suyla (başlıktaki) duvar saatidir |`);
  md.push(`| Bir katmanı yeniden kurma: 100 bin segmentte < 50 ms | ${p95('hat-1m', 'rebuild', 'frame.build')} ms | \`hat-1m\` büyük katmanı 1 milyon segmenttir; hedef 100 bin segment içindir |`);
}
if (summaries['parsel-50k']) md.push(`| Bir katmanı yeniden kurma | ${p95('parsel-50k', 'rebuild', 'frame.build')} ms | \`parsel-50k\` \`parsel\` katmanı, ${n(summaries['parsel-50k'].count?.parcels)} alan |`);
md.push('');
md.push("Karşılaştırma bir kabul kararı değildir: hedefler onaylanmadı; ölçüm başsız Chrome'da, ana iş parçacığında alındı.");
md.push('');
const warnings = [];
for (const r of results)
  for (const [id, sc] of Object.entries(r.scenarios)) {
    if (sc.sent < sc.planned) warnings.push(`\`${r.dataset}\` #${r.run} \`${id}\`: ${sc.planned} olaydan ${sc.sent} tanesi ölçüldü (dizi ${SEQUENCE_CAP_MS / 1000} s sınırına takıldı).`);
    if (id !== 'rebuild' && sc.frames < sc.sent) warnings.push(`\`${r.dataset}\` #${r.run} \`${id}\`: ${sc.sent} olayda ${sc.frames} kare çizildi.`);
    if (id === 'trim-close' && !sc.metrics['frame.preview']) warnings.push(`\`${r.dataset}\` #${r.run} \`${id}\`: imleç hiçbir karede bir kenarda görünmedi (\`tools.active.hover\` okunamıyor olabilir); önizleme ölçütü yok.`);
  }
if (gcFailed) warnings.push('Çöp toplama zorlanamadı (HeapProfiler.collectGarbage).');
if (warnings.length) {
  md.push('## Uyarılar');
  md.push('');
  for (const w of warnings) md.push(`- ${w}`);
  md.push('');
}
report.warnings = warnings;
writeFileSync(`${outDir}/interaction-${label}.json`, `${JSON.stringify(report, null, 2)}\n`);
md.push('## Gürültü');
md.push('');
md.push(
  `- p95'i 0.5 ms'den büyük ${spreads.length} ölçütte koşular arası yayılım (en yüksek − en düşük p95, ortancaya oranla): ortanca ${spreads.length ? pct(median(spreads.map((s) => s.rel))) : '–'}, en çok ${spreads[0] ? `${pct(spreads[0].rel)} (\`${spreads[0].k}\`)` : '–'}.`,
);
md.push("- Tek tek olaylar çöp toplamayla sıçrayabilir: p99 ve “en çok” bu yüzden p95'ten gürültülüdür. Her dizi öncesinde çöp toplama zorlanır.");
md.push('- Karşılaştırmada iki ölçümün farkı p95 aralıklarının dışına çıkmıyorsa gürültü sayılmalıdır.');
md.push('');
md.push('## Yöntem');
md.push('');
md.push(`- Veri setleri sayfada, tohumlu üreteçle kurulur ve \`kentos.doc.replaceWith\` ile açılır (geçmişsiz). Üreteçler ve tohumlar \`scripts/perf/datasets.mjs\`'tedir; parametreler JSON raporundadır.`);
md.push('- İmleç olayları gerçek `Input.dispatchMouseEvent` hareketleridir. Chrome bir hareketi karesi çizilince yanıtlar; sonraki hareket ancak o zaman gider, böylece olaylar birleşmez ve her olay kendi karesini çizer (her dizide sayılır). Yol, çizim alanının araç kutusu ve komut şeridi dışında kalan kısmında, veri setinin ekrandaki kutusunun içindedir (genel görünümde veri seti tuvali doldurmaz).');
md.push(`- Chrome makinenin GPU'sunda çizer (${CHROME_ARGS.join(' ') || 'varsayılan bayraklar'}); kareler Chrome'un kendi 60 Hz hızındadır. SwiftShader (yazılım GPU) ile bu veri setlerinde tek kare saniyeler sürüyor, kareler birikip sayfayı sonradan dakikaya varan sürelerle durduruyordu; ana iş parçacığı süreleri iki durumda da aynıydı.`);
md.push('- Nesne izleme ve bilgi kartı kapalıdır: ikisi de beklemeye (350 ve 500 ms) bağlıdır ve hızlı bir çekirdeğin farklı iş yapmasına yol açardı. Kenet türleri, açıklıklar ve ızgara varsayılandır.');
md.push("- Google yazı tipleri engellenir (her koşuda aynı yerel yazı tipi, ağ yok). Sunucu bağlantısı yoktur (\"Sunucu: yok\").");
md.push('- Başlangıç süreleri bu betikte değil, `scripts/perf/startup.mjs` raporlarındadır.');
md.push('');

let compared = null;
if (compareFile && existsSync(compareFile) && resolve(compareFile) !== resolve(`${outDir}/interaction-${label}.json`)) {
  const base = JSON.parse(readFileSync(compareFile, 'utf8'));
  // Only like with like: the same generator parameters (a --scale run is not comparable) and the same machine, Chrome and GPU.
  const sameData = (name) => JSON.stringify(base.datasets?.[name]?.params) === JSON.stringify(summaries[name]?.params);
  const skipped = Object.keys(summaries).filter((name) => !sameData(name));
  const sameConditions = base.machine?.cpu === report.machine.cpu && base.machine?.chrome === report.machine.chrome && base.setup?.glRenderer === report.setup.glRenderer;
  compared = [];
  for (const [key, m] of Object.entries(metrics)) {
    const o = base.metrics?.[key];
    if (!o || !(o.p95 > 0) || skipped.includes(key.split('/')[0])) continue;
    const ratio = m.p95 / o.p95;
    // Outside the other run's whole p95 range by more than 10 % (and by 0.1 ms, the scale of timer and scheduling noise).
    const clear = Math.abs(m.p95 - o.p95) >= 0.1;
    const verdict = clear && m.p95 > o.p95Range[1] * 1.1 ? 'gerileme' : clear && m.p95 < o.p95Range[0] / 1.1 ? 'iyileşme' : 'fark yok';
    compared.push({ key, base: o.p95, now: m.p95, ratio, verdict });
  }
  const order = { gerileme: 0, iyileşme: 1, 'fark yok': 2 };
  compared.sort((a, b) => order[a.verdict] - order[b.verdict] || a.key.localeCompare(b.key));
  md.push(`## Karşılaştırma: ${base.label} (${base.date?.slice(0, 10)}, ${base.commit}) → ${label}`);
  md.push('');
  md.push("p95 koşuların ortancasıdır. “Gerileme”: yeni p95, eski ölçümün en yüksek koşusundan %10'dan ve 0.1 ms'den fazla yüksek; “iyileşme”: en düşük koşusundan aynı paylarla düşük. Ölçüm koşulları (makine, Chrome, GPU) aynı değilse karşılaştırma geçersizdir.");
  md.push('');
  md.push(`Önce: ${base.machine?.cpu}, ${base.machine?.chrome}, ${base.setup?.glRenderer}. Şimdi: ${report.machine.cpu}, ${report.machine.chrome}, ${report.setup.glRenderer}.`);
  if (!sameConditions) md.push('', '**Makine, Chrome ya da GPU farklı: bu karşılaştırma geçersizdir; önce aynı koşullarda yeni taban alın.**');
  if (skipped.length) md.push('', `**Veri seti parametreleri farklı, karşılaştırılmadı:** ${skipped.map((name) => `\`${name}\``).join(', ')} (ör. \`--scale\`).`);
  md.push('');
  md.push('| Ölçüt | önce p95 | şimdi p95 | oran | sonuç |');
  md.push('|---|---|---|---|---|');
  for (const c of compared) md.push(`| \`${c.key}\` | ${ms(c.base)} | ${ms(c.now)} | ${c.ratio.toFixed(2)} | ${c.verdict} |`);
  md.push('');
  const worse = compared.filter((c) => c.verdict === 'gerileme');
  md.push(!compared.length ? 'Karşılaştırılacak ölçüt yok.' : worse.length ? `**${worse.length} ölçütte gerileme var.**` : 'Gerileme yok.');
  md.push('');
  report.compared = { with: relative(fileURLToPath(new URL('../../../../', import.meta.url)), compareFile), sameConditions, skipped, results: compared };
  writeFileSync(`${outDir}/interaction-${label}.json`, `${JSON.stringify(report, null, 2)}\n`);
}
writeFileSync(`${outDir}/interaction-${label}.md`, md.join('\n'));
console.log(md.join('\n'));
