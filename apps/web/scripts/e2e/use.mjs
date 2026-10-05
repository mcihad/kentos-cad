// A usage scenario played in the real app, pictured step by step:
//
//   pnpm -C apps/web e2e:use <trace-id | path.json> [--size 1440x900,1100x650] [--theme dark,light] [--variant us|tr-q|hidpi]
//
// A scenario is an interaction trace (fixtures/interaction/v1, its README): the steps a user takes,
// with `{"shot": "name"}` steps where a picture is wanted. It is played in the real app like
// `pnpm e2e:interaction` plays the tests (the same step engine, tracePlayer.mjs), and the trace's
// expectations are checked on the way and said, so using the app and testing it are one run. The
// desktop plays the same file (`kentos-cad kullan`, apps/desktop/src/usage.rs) and
// scripts/usage/compare.py puts the two platforms' pictures side by side.
//
// Each size and theme plays the scenario from its start in a new browser of that size, the theme
// chosen through the app's own settings. The pictures are
// `<repo>/.run/shots/kullanim/web-<id>-<nn>-<name>-<W>x<H>[-acik].png`, `nn` the picture's place in the
// scenario from 01; a scenario without shots gets one at its end, `son`. The names are the desktop's.
// In a window at least 800 px high the bottom panel's Komut geçmişi is open at its newest line in each
// picture, so it shows what the tools said; in a lower one it stays closed (it would leave the drawing
// area too small; the last message is in the status bar). It is put back as it was after each picture.
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { launch, sleep } from './cdp.mjs';
import { attach, loadTraces, play, replacePointer, setLayout, startApp, TRACE_DIR, VARIANTS, waitReady } from './tracePlayer.mjs';

const REPO = new URL('../../../../', import.meta.url).pathname;
const OUT = join(REPO, '.run/shots/kullanim');
/** A window this high or higher has room for the history beside the drawing. */
const HISTORY_FROM = 800;
const USAGE = 'kullanım: pnpm -C apps/web e2e:use <iz | iz.json> [--size 1440x900,1100x650] [--theme dark,light] [--variant us|tr-q|hidpi]';
const THEMES = { dark: '', koyu: '', light: '-acik', acik: '-acik' };

// ── Arguments ───────────────────────────────────────────────────────────
function parseArgs(argv) {
  const o = { trace: null, sizes: '1440x900,1100x650', themes: 'dark,light', variant: 'us' };
  const known = { '--size': 'sizes', '--theme': 'themes', '--variant': 'variant' };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--') continue;
    const [flag, inline] = arg.split(/=(.*)/s);
    if (known[flag]) {
      const value = inline ?? argv[++i];
      if (value === undefined) throw new Error(`${flag} bir değer ister (${USAGE})`);
      o[known[flag]] = value;
    } else if (arg.startsWith('--')) throw new Error(`${arg}: bilinmeyen seçenek (${USAGE})`);
    else if (o.trace === null) o.trace = arg;
    else throw new Error(`${arg}: tek bir iz verilir (${USAGE})`);
  }
  if (o.trace === null) throw new Error(USAGE);
  return o;
}

function parseSizes(text) {
  return text.split(',').map((one) => {
    const m = /^\s*(\d+)x(\d+)\s*$/.exec(one);
    if (!m || Number(m[1]) < 400 || Number(m[2]) < 300) throw new Error(`${one}: boyut GxY biçiminde olmalı (ör. 1440x900)`);
    return { w: Number(m[1]), h: Number(m[2]) };
  });
}

function parseThemes(text) {
  return text.split(',').map((t) => {
    const theme = t.trim();
    if (!(theme in THEMES)) throw new Error(`${theme}: tema dark ya da light olmalı`);
    return { name: THEMES[theme] === '' ? 'dark' : 'light', suffix: THEMES[theme] };
  });
}

/** A trace by id (a file of the traces' folder) or by the path of a .json file. */
function readTrace(name) {
  if (!name.endsWith('.json')) {
    const [trace] = loadTraces([name]);
    if (!trace) throw new Error(`${name}: böyle bir iz yok (${TRACE_DIR})`);
    return trace;
  }
  // `pnpm -C apps/web` runs in apps/web: a relative path is also read from where the command was typed, and from the repository.
  const file = [resolve(name), resolve(process.env.INIT_CWD ?? '.', name), resolve(REPO, name)].find(existsSync);
  if (!file) throw new Error(`${name}: dosya yok`);
  const trace = JSON.parse(readFileSync(file, 'utf8'));
  if (trace.format !== 'kentos.interaction-trace' || trace.version !== 1) throw new Error(`${name}: v1 etkileşim izi değil`);
  return trace;
}

// ── Pictures ────────────────────────────────────────────────────────────
/**
 * The bottom panel's Komut geçmişi open at its newest line, for a picture; another tab the trace opened (Koordinat
 * listesi, Noktalar) stays, as the desktop's player keeps it. What it was before is given back by `restore`.
 */
async function openHistory(b) {
  const before = await b.eval(`(() => {
    const { ui } = window.kentos;
    const before = { open: ui.bottomExpanded.value, tab: ui.bottomTab.value };
    if (!before.open) ui.bottomTab.set('history');
    ui.bottomExpanded.set(true);
    return before;
  })()`);
  // The drawing area gives way and lays out again, the mouse back on the trace's place of the drawing (the
  // crosshair, the preview and the hover card show it); then the list is scrolled to its newest line.
  await sleep(250);
  await replacePointer();
  await b.eval(`(() => { const c = document.querySelector('.bottom__content'); if (c) c.scrollTop = c.scrollHeight; })()`);
  await sleep(100);
  return async () => {
    await b.eval(`(() => { const { ui } = window.kentos; ui.bottomTab.set(${JSON.stringify(before.tab)}); ui.bottomExpanded.set(${before.open}); })()`);
    // The drawing area is its old size again, and the mouse on its place of it, before the next step reads the view.
    await sleep(250);
    await replacePointer();
  };
}

// ── One scenario, at one size and theme ─────────────────────────────────
/** Plays `trace` from its start at `size` in `theme`, writing its pictures; the problems, each naming the size and theme. */
async function playOnce(app, trace, variant, size, theme) {
  const label = `${size.w}x${size.h}${theme.suffix}`;
  const b = await launch(app.url, { width: size.w, height: size.h });
  attach(b);
  try {
    await waitReady(b);
    setLayout(variant.layout);
    await b.send('Emulation.setDeviceMetricsOverride', { width: size.w, height: size.h, deviceScaleFactor: variant.dpr, mobile: false });
    await sleep(300);
    // As a user would: through the app's own settings.
    await b.eval(`window.kentos.prefs.theme.set(${JSON.stringify(theme.name)})`);
    await b.waitFor(`document.documentElement.dataset.theme === ${JSON.stringify(theme.name)}`, 5000);
    await sleep(300);
    let count = 0;
    const take = async (name) => {
      count++;
      const restore = size.h >= HISTORY_FROM ? await openHistory(b) : null;
      const file = await b.shot(`web-${trace.id}-${String(count).padStart(2, '0')}-${name}-${label}`, undefined, OUT);
      await restore?.();
      console.log(file);
    };
    const problems = await play(trace, { onShot: take });
    if (!count) await take('son');
    const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
    return [...problems, ...errors.map((e) => `sayfada hata: ${e}`)].map((p) => `${label}: ${p.trimStart()}`);
  } finally {
    b.close();
  }
}

// ── Main ────────────────────────────────────────────────────────────────
const options = parseArgs(process.argv.slice(2));
const trace = readTrace(options.trace);
const sizes = parseSizes(options.sizes);
const themes = parseThemes(options.themes);
const variant = VARIANTS.find((v) => v.id === options.variant);
if (!variant) throw new Error(`${options.variant}: böyle bir varyant yok (${VARIANTS.map((v) => v.id).join(', ')})`);
mkdirSync(OUT, { recursive: true });

const app = await startApp();
const problems = [];
try {
  for (const size of sizes) for (const theme of themes) problems.push(...(await playOnce(app, trace, variant, size, theme)));
} finally {
  await app.close();
}
if (problems.length) {
  for (const p of problems) console.error(`Uyarı: ${p}`);
  console.error(`${trace.id}: ${problems.length} beklenti tutmadı.`);
} else console.log(`${trace.id}: beklentilerin hepsi tuttu.`);
process.exit(problems.length ? 1 : 0);
