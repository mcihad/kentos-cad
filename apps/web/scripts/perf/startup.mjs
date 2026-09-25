// Startup measurement (CLAUDE.md §20.4 step 1): serves the production build
// (`vite preview`, run `node scripts/perf/bundle.mjs` first) and loads the
// app in headless Chrome, cold (fresh profile) and warm (second load in the
// same profile), `--runs` times each. Reports per load:
//   - requests and bytes transferred (compressed, as on the wire),
//   - JS/CSS/WASM bytes by type,
//   - main-thread script and task time (Performance.getMetrics),
//   - time to `kentos:interactive` (marked after the first drawn frame).
// Nothing else heavy may run meanwhile (docs/adr/0005). Writes
// docs/perf/startup-<label>-<date>.{json,md}.
//
//   node scripts/perf/startup.mjs [--runs 3] [--label baseline] [--out docs/perf]
import { fileURLToPath } from 'node:url';
import { preview } from 'vite';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { execSync } from 'node:child_process';
import { cpus, totalmem } from 'node:os';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const runs = Number(opt('runs', '3'));
const label = opt('label', 'baseline');
const outDir = resolve(opt('out', fileURLToPath(new URL('../../../../docs/perf', import.meta.url))));

const server = await preview({ preview: { port: 0, strictPort: false, open: false }, logLevel: 'error' });
const url = server.resolvedUrls.local[0];

/** One page load with the given browser; `warm` reuses its cache. */
async function measure(b, warm) {
  const responses = new Map();
  b.on('Network.responseReceived', (p) => {
    const encoding = Object.entries(p.response.headers ?? {}).find(([k]) => k.toLowerCase() === 'content-encoding')?.[1] ?? '';
    responses.set(p.requestId, { url: p.response.url, status: p.response.status, type: p.type, mime: p.response.mimeType, encoding, bytes: 0 });
  });
  b.on('Network.loadingFinished', (p) => {
    const r = responses.get(p.requestId);
    if (r) r.bytes = p.encodedDataLength;
  });
  await b.send('Network.enable');
  await b.send('Performance.enable');
  if (!warm) await b.send('Network.clearBrowserCache');
  await b.send('Page.navigate', { url });
  const t0 = Date.now();
  let interactive = null;
  while (Date.now() - t0 < 30000) {
    interactive = await b.eval(`(() => { const m = performance.getEntriesByName('kentos:interactive')[0]; return m ? m.startTime : null; })()`).catch(() => null);
    if (interactive !== null) break;
    await sleep(100);
  }
  await sleep(500);
  const metrics = Object.fromEntries((await b.send('Performance.getMetrics')).metrics.map((m) => [m.name, m.value]));
  const list = [...responses.values()].filter((r) => r.url.startsWith(url));
  const bytesOf = (test) => list.filter(test).reduce((s, r) => s + r.bytes, 0);
  await b.send('Network.disable');
  return {
    warm,
    interactiveMs: interactive,
    requests: list.length,
    transferred: bytesOf(() => true),
    js: bytesOf((r) => r.mime.includes('javascript')),
    css: bytesOf((r) => r.mime.includes('css')),
    wasm: bytesOf((r) => r.mime.includes('wasm')),
    scriptMs: Math.round((metrics.ScriptDuration ?? 0) * 1000),
    taskMs: Math.round((metrics.TaskDuration ?? 0) * 1000),
    jsHeapMb: Math.round((metrics.JSHeapUsedSize ?? 0) / 1e6),
    list: list.map((r) => ({ path: new URL(r.url).pathname, status: r.status, bytes: r.bytes, encoding: r.encoding })),
  };
}

const loads = [];
try {
  for (let i = 0; i < runs; i++) {
    const b = await launch('about:blank', { width: 1600, height: 900 });
    try {
      loads.push(await measure(b, false));
      loads.push(await measure(b, true));
    } finally {
      b.close();
    }
    await sleep(500);
  }
} finally {
  await new Promise((ok) => server.httpServer.close(ok));
}

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? s[Math.floor(s.length / 2)] : null;
};
const summarize = (warm) => {
  const ls = loads.filter((l) => l.warm === warm);
  const keys = ['interactiveMs', 'requests', 'transferred', 'js', 'css', 'wasm', 'scriptMs', 'taskMs', 'jsHeapMb'];
  const out = Object.fromEntries(keys.map((k) => [k, median(ls.map((l) => l[k]).filter((v) => v !== null))]));
  const t = ls.map((l) => l.interactiveMs).filter((v) => v !== null);
  return { ...out, interactiveRange: t.length ? [Math.min(...t), Math.max(...t)] : null };
};
const cpu = cpus()[0]?.model ?? '?';
const report = {
  label,
  date: new Date().toISOString(),
  commit: execSync('git rev-parse --short HEAD').toString().trim(),
  machine: { cpu, threads: cpus().length, memoryGb: Math.round(totalmem() / 2 ** 30), chrome: execSync(`${process.env.CHROME_BIN ?? 'google-chrome'} --version`).toString().trim() },
  runs,
  cold: summarize(false),
  warm: summarize(true),
  loads,
};
const day = report.date.slice(0, 10);
mkdirSync(outDir, { recursive: true });
writeFileSync(`${outDir}/startup-${label}-${day}.json`, `${JSON.stringify(report, null, 2)}\n`);
const kb = (n) => (n === null ? '–' : `${(n / 1024).toFixed(1)} KB`);
const ms = (n) => (n === null ? '–' : `${Math.round(n)} ms`);
const range = (r) => (r ? ` (${Math.round(r[0])}–${Math.round(r[1])})` : '');
const encodings = [...new Set(loads.flatMap((l) => l.list.map((r) => r.encoding)).filter(Boolean))];
const first = loads.find((l) => !l.warm);
const md = [
  `# Başlangıç ölçümü: ${label} (${day}, ${report.commit})`,
  '',
  `${cpu}, ${report.machine.threads} iş parçacığı, ${report.machine.memoryGb} GB; ${report.machine.chrome}; başsız, WebGL2; \`vite preview\` (yerel). ${runs} ölçümün ortancası. Hedefler: docs/adr/0005.`,
  '',
  '| Ölçüt | Soğuk | Ilık |',
  '|---|---|---|',
  `| Etkileşime hazır (\`kentos:interactive\`; aralık) | ${ms(report.cold.interactiveMs)}${range(report.cold.interactiveRange)} | ${ms(report.warm.interactiveMs)}${range(report.warm.interactiveRange)} |`,
  `| İstek sayısı | ${report.cold.requests} | ${report.warm.requests} |`,
  `| Aktarılan toplam | ${kb(report.cold.transferred)} | ${kb(report.warm.transferred)} |`,
  `| JS aktarımı | ${kb(report.cold.js)} | ${kb(report.warm.js)} |`,
  `| CSS aktarımı | ${kb(report.cold.css)} | ${kb(report.warm.css)} |`,
  `| WASM aktarımı | ${kb(report.cold.wasm)} | ${kb(report.warm.wasm)} |`,
  `| Script süresi (ana iş parçacığı) | ${ms(report.cold.scriptMs)} | ${ms(report.warm.scriptMs)} |`,
  `| Görev süresi (toplam) | ${ms(report.cold.taskMs)} | ${ms(report.warm.taskMs)} |`,
  `| JS yığını | ${report.cold.jsHeapMb} MB | ${report.warm.jsHeapMb} MB |`,
  '',
  encodings.length
    ? `Aktarım, sunucunun gönderdiği sıkıştırılmış boyuttur (\`content-encoding: ${encodings.join(', ')}\`). Brotli karşılığı için build envanterine bakın.`
    : 'Aktarım ham baytır: sunucu sıkıştırma yapmadı. Sıkıştırılmış karşılık için build envanterindeki gzip/brotli sütunlarına bakın.',
  '',
  'İlk soğuk yüklemenin istekleri:',
  '',
  '| Yol | Durum | Aktarım | Kodlama |',
  '|---|---|---|---|',
  ...(first?.list ?? []).map((r) => `| \`${r.path}\` | ${r.status} | ${kb(r.bytes)} | ${r.encoding || '–'} |`),
  '',
].join('\n');
writeFileSync(`${outDir}/startup-${label}-${day}.md`, md);
console.log(md);
