// Heavy modules' opening time (docs/adr/0005 "Ağır modülün ilk açılışı", CLAUDE.md §20.4 step 1):
// serves the production build (`vite preview`, run `pnpm build` first) and, for each module in a
// fresh headless Chrome (cold profile), types its alias on the command line and presses Enter,
// timed in the page from that keydown to the module's window in the document and to the frame
// after it was painted (two animation frames). The window is closed and opened again in the same
// page for the second opening. The module's requests (JS, CSS, WASM) are listed as transferred.
// Nothing else heavy may run meanwhile (docs/adr/0005). Writes docs/perf/modules-<label>-<date>.{json,md}.
//
//   node scripts/perf/modules.mjs [--runs 3] [--label baseline] [--out docs/perf]
import { fileURLToPath } from 'node:url';
import { preview } from 'vite';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { execSync } from 'node:child_process';
import { cpus, totalmem } from 'node:os';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const runs = Number(opt('runs', '3'));
const label = opt('label', 'baseline');
const outDir = resolve(opt('out', fileURLToPath(new URL('../../../../docs/perf', import.meta.url))));

/** The modules of ADR 0005's target: the command-line alias that opens each, and its window. */
const MODULES = [
  { name: 'Stil yöneticisi', alias: 'STIL', window: '.dialog--styles' },
  { name: 'Model tasarımcısı', alias: 'MODEL', window: '.dialog--designer' },
  { name: 'SVG düzenleyicisi', alias: 'SVG', window: '.dialog--svge' },
];

const server = await preview({ preview: { port: 0, strictPort: false, open: false }, logLevel: 'error' });
const url = server.resolvedUrls.local[0];

/** Arms the page: the Enter keydown on the command line starts the clock, the window's arrival stops it. */
const arm = (selector) => `(() => {
  window.__open = null;
  const input = document.querySelector('.cmdline__input');
  let t0 = null;
  input.addEventListener('keydown', (e) => { if (e.key === 'Enter' && t0 === null) t0 = performance.now(); }, { capture: true, once: true });
  const seen = new MutationObserver(() => {
    if (!document.querySelector(${JSON.stringify(selector)})) return;
    seen.disconnect();
    const inserted = performance.now();
    requestAnimationFrame(() => requestAnimationFrame(() => { window.__open = { inserted: inserted - t0, painted: performance.now() - t0 }; }));
  });
  seen.observe(document.body, { childList: true, subtree: true });
  input.focus();
})()`;

async function open(b, m) {
  await b.eval(arm(m.window));
  await b.type(m.alias);
  await b.key('Enter');
  await b.waitFor('window.__open !== null', 20000);
  return b.eval('window.__open');
}

/** Esc until the window is gone; an unsaved-changes question is left without saving. */
async function close(b, m) {
  for (let i = 0; i < 6 && (await b.eval(`!!document.querySelector(${JSON.stringify(m.window)})`)); i++) {
    const leave = await b.eval(`(() => { const x = [...document.querySelectorAll('.dialog--confirm .btn')].find((e) => e.textContent.includes('Kaydetmeden')); if (!x) return null; const r = x.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; })()`);
    if (leave) await b.click(...leave);
    else await b.key('Escape');
    await sleep(200);
  }
}

/** One module in a fresh browser: first opening (with what it fetched), then a second one. */
async function measure(m) {
  const b = await launch(url + '?renderer=webgl2', { width: 1600, height: 1000 });
  const fetched = new Map();
  b.on('Network.responseReceived', (p) => {
    const encoding = Object.entries(p.response.headers ?? {}).find(([k]) => k.toLowerCase() === 'content-encoding')?.[1] ?? '';
    fetched.set(p.requestId, { url: p.response.url, mime: p.response.mimeType, encoding, bytes: 0 });
  });
  b.on('Network.loadingFinished', (p) => {
    const r = fetched.get(p.requestId);
    if (r) r.bytes = p.encodedDataLength;
  });
  try {
    await b.send('Network.enable');
    await b.waitFor(`performance.getEntriesByName('kentos:interactive').length > 0`, 30000);
    await sleep(1500);
    const before = new Set(fetched.keys());
    const first = await open(b, m);
    await sleep(500);
    const loaded = [...fetched.entries()].filter(([id, r]) => !before.has(id) && r.url.startsWith(url)).map(([, r]) => r);
    await close(b, m);
    await sleep(500);
    const second = await open(b, m);
    await close(b, m);
    const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l)).length;
    return { first, second, errors, fetched: loaded.map((r) => ({ path: new URL(r.url).pathname, mime: r.mime, encoding: r.encoding, bytes: r.bytes })) };
  } finally {
    b.close();
    await sleep(500);
  }
}

const loads = [];
try {
  for (let i = 0; i < runs; i++) for (const m of MODULES) loads.push({ module: m.name, ...(await measure(m)) });
} finally {
  await new Promise((ok) => server.httpServer.close(ok));
}

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? s[Math.floor(s.length / 2)] : null;
};
const summary = MODULES.map((m) => {
  const ls = loads.filter((l) => l.module === m.name);
  const pick = (k, f) => ls.map((l) => l[k][f]);
  const range = (xs) => [Math.min(...xs), Math.max(...xs)];
  return {
    module: m.name,
    firstInserted: median(pick('first', 'inserted')),
    firstPainted: median(pick('first', 'painted')),
    firstPaintedRange: range(pick('first', 'painted')),
    secondPainted: median(pick('second', 'painted')),
    secondPaintedRange: range(pick('second', 'painted')),
    fetchedBytes: median(ls.map((l) => l.fetched.reduce((s, r) => s + r.bytes, 0))),
    errors: ls.reduce((s, l) => s + l.errors, 0),
  };
});
const version = (() => {
  try {
    return execSync(`${process.env.CHROME_BIN ?? 'google-chrome'} --version`).toString().trim();
  } catch {
    return '?';
  }
})();
const report = {
  label,
  date: new Date().toISOString(),
  commit: execSync('git rev-parse --short HEAD').toString().trim(),
  // The reports themselves do not count: a re-run on the same commit is still a clean measurement.
  dirtyTree: execSync("git status --porcelain -- ':(top)' ':(exclude,top)docs/perf'").toString().trim().length > 0,
  machine: { cpu: cpus()[0]?.model ?? '?', threads: cpus().length, memoryGb: Math.round(totalmem() / 2 ** 30), chrome: version },
  runs,
  summary,
  loads,
};
const day = report.date.slice(0, 10);
mkdirSync(outDir, { recursive: true });
writeFileSync(`${outDir}/modules-${label}-${day}.json`, `${JSON.stringify(report, null, 2)}\n`);
const ms = (n) => `${Math.round(n)} ms`;
const kb = (n) => `${(n / 1024).toFixed(1)} KB`;
const first = (name) => loads.find((l) => l.module === name);
const encodings = [...new Set(loads.flatMap((l) => l.fetched.map((r) => r.encoding)).filter(Boolean))];
const md = [
  `# Ağır modüllerin açılışı: ${label} (${day}, ${report.commit}${report.dirtyTree ? ', çalışma ağacı temiz değil' : ''})`,
  '',
  `${report.machine.cpu}, ${report.machine.threads} iş parçacığı, ${report.machine.memoryGb} GB; ${version}; başsız, WebGL2; \`vite preview\` (yerel). Her modül kendi boş profiliyle, ${runs} ölçümün ortancası. Süre komut satırında Enter'dan modülün penceresinin belgeye girmesine ve boyandığı kareye (iki animasyon karesi) kadardır. Hedefler: docs/adr/0005 (ilk açılış ≤ 400 ms, ikinci ≤ 150 ms).`,
  '',
  '| Modül | İlk açılış: pencere | İlk açılış: boyandı (aralık) | İkinci açılış: boyandı (aralık) | İlk açılışta indirilen |',
  '|---|---|---|---|---|',
  ...summary.map((s) => `| ${s.module} | ${ms(s.firstInserted)} | ${ms(s.firstPainted)} (${Math.round(s.firstPaintedRange[0])}–${Math.round(s.firstPaintedRange[1])}) | ${ms(s.secondPainted)} (${Math.round(s.secondPaintedRange[0])}–${Math.round(s.secondPaintedRange[1])}) | ${kb(s.fetchedBytes)} |`),
  '',
  encodings.length
    ? `Aktarım, sunucunun gönderdiği boyuttur: ${encodings.join(', ')} ile sıkıştırılanlar sıkıştırılmış, öbürleri (WASM dahil) ham.`
    : 'Aktarım ham baytır: sunucu sıkıştırma yapmadı.',
  '',
  summary.some((s) => s.errors) ? `Konsol hataları: ${summary.map((s) => `${s.module} ${s.errors}`).join(', ')}.` : 'Konsolda hata yok.',
  '',
  'İlk ölçümde ilk açılışın istekleri:',
  '',
  ...MODULES.flatMap((m) => [`- ${m.name}: ${(first(m.name)?.fetched ?? []).map((r) => `\`${r.path.split('/').pop()}\` ${kb(r.bytes)}${r.encoding ? ` (${r.encoding})` : ''}`).join(', ') || '–'}`]),
  '',
].join('\n');
writeFileSync(`${outDir}/modules-${label}-${day}.md`, md);
console.log(md);
