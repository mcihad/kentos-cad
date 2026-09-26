// KCAD v2 save and open in the browser (TODOS.md FILE-15, FILE-20, FILE-24;
// docs/adr/0025, 0030): how long the app's own Kaydet and Aç take for a large
// drawing, how long the page's main thread is blocked meanwhile, how much
// memory Chrome takes on top, and how big the file is. Starts its own Vite dev
// server and one headless Chrome (the machine's GPU, as interaction.mjs), and
// for every size and run:
//
//   1. opens the app, builds a deterministic drawing in the page (parcels of
//      20 vertices with three attributes and a label: the drawing of
//      crates/shared/kcad/tests/measure.rs) and puts it on screen;
//   2. Farklı kaydet into an in-memory file (`files.saveAs()`, the picker
//      answered by the page): time until the save resolves, long tasks
//      (PerformanceObserver, > 50 ms) and Chrome's peak memory above the
//      level before the save (PSS of the whole process tree, sampled every
//      50 ms);
//   3. stops the formats worker (its memory goes back), then Aç from the same
//      in-memory file (`files.open()`): time until the drawing is replaced,
//      until its first frame is drawn, long tasks and peak memory.
//
// The app's API is the same before and after the typed worker boundary, so
// the same script measures both. Nothing is written but the report:
// docs/perf/kcad-web-<label>-<date>.{json,md}. Nothing else heavy may run
// meanwhile (docs/adr/0005); the memory limit stops a run that grows too big
// for the machine.
//
//   pnpm perf:kcad [--label latest] [--sizes 1,25000,50000,100000,200000]
//     [--runs 3] [--out docs/perf] [--memory-limit 4500] [--allow-swiftshader]
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { cpus, loadavg, release, totalmem } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const label = opt('label', 'latest');
const runs = Number(opt('runs', '3'));
const sizes = opt('sizes', '1,25000,50000,100000,200000').split(',').map(Number);
const outDir = resolve(opt('out', fileURLToPath(new URL('../../../../docs/perf', import.meta.url))));
const memoryLimitMb = Number(opt('memory-limit', '4500'));
const allowSwiftShader = args.includes('--allow-swiftshader');
const date = new Date().toISOString().slice(0, 10);

// ── The drawing (runs inside the page: self-contained) ──────────────────

/** `n` parcels of 20 vertices, attributes Ada/Parsel/Nitelik and a label, on one layer (measure.rs). */
function parcels(n) {
  const entities = [];
  for (let i = 0; i < n; i++) {
    const x0 = 486000 + (i % 300) * 31.7;
    const y0 = 4420000 + Math.floor(i / 300) * 27.3;
    const pts = [];
    for (let k = 0; k < 20; k++) {
      const a = (k / 20) * Math.PI * 2;
      pts.push({ x: x0 + 12.5 + 11 * Math.cos(a), y: y0 + 12.5 + 9 * Math.sin(a) });
    }
    const ada = String(100 + Math.floor(i / 50));
    const parsel = String((i % 50) + 1);
    entities.push({ id: i + 1, kind: 'polygon', layerId: 'parsel', attrs: { Ada: ada, Parsel: parsel, Nitelik: 'Arsa' }, label: `${ada}/${parsel}`, pts });
  }
  const rows = Math.max(1, Math.ceil(n / 300));
  return {
    entities,
    layers: [{ id: 'parsel', name: 'Parsel', style: { color: '#E06C75', lineWeight: 0.35 } }],
    bounds: { minX: 486000, minY: 4420000, maxX: 486000 + 300 * 31.7, maxY: 4420000 + rows * 27.3 },
  };
}

// ── Machine and browser ─────────────────────────────────────────────────

/** Proportional set size (MB) of a process and its descendants. */
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

const osName = (() => {
  try {
    return /^PRETTY_NAME="?([^"\n]+)/m.exec(readFileSync('/etc/os-release', 'utf8'))?.[1] ?? 'Linux';
  } catch {
    return 'Linux';
  }
})();
const git = (...a) => execFileSync('git', a, { encoding: 'utf8' }).trim();
const commit = git('rev-parse', '--short', 'HEAD');
const dirtyTree = git('status', '--porcelain', '--untracked-files=no').length > 0;

// ── Measurement ─────────────────────────────────────────────────────────

process.env.KENTOS_API_PORT = '9';
const server = await createServer({
  server: { port: 0, strictPort: false, hmr: false, watch: null, headers: { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' } },
  logLevel: 'error',
});
await server.listen();
const url = `${server.resolvedUrls.local[0]}?start=0`;
const CHROME_ARGS = allowSwiftShader ? [] : ['--use-angle=gl', '--ignore-gpu-blocklist'];
const b = await launch('about:blank', { width: 1600, height: 900, args: CHROME_ARGS });
const chromePid = b.pid;

/** Samples Chrome's memory while `task` runs: the peak above `base`, and a memory limit that stops the run. */
async function sampled(task) {
  let peak = 0;
  let over = null;
  const timer = setInterval(() => {
    const mb = treePssMb(chromePid);
    peak = Math.max(peak, mb);
    if (mb > memoryLimitMb && !over) over = mb;
  }, 50);
  try {
    const value = await task();
    return { value, peakMb: peak, over };
  } finally {
    clearInterval(timer);
    peak = Math.max(peak, treePssMb(chromePid));
  }
}

const gc = () => b.send('HeapProfiler.collectGarbage').catch(() => {});
const settle = () => b.eval('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(r, 0))))');

async function openApp() {
  await b.eval('window.__perfOld = true').catch(() => {});
  await b.send('Page.navigate', { url });
  const ready = '!window.__perfOld && window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 60000);
  await sleep(800);
  await b.waitFor(ready, 20000);
  return b.eval(`(() => {
    const gl = document.querySelector('.viewport__gl')?.getContext('webgl2');
    const info = gl?.getExtension('WEBGL_debug_renderer_info');
    return { backend: window.kentos.view.backendKind.value, glRenderer: gl ? String(gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER)) : null, isolated: crossOriginIsolated };
  })()`);
}

/** Builds the drawing in the page, puts it on screen and installs the in-memory file and the long-task log. */
async function prepare(n) {
  return b.eval(`(() => {
    const k = window.kentos;
    const parcels = ${parcels.toString()};
    const t0 = performance.now();
    const d = parcels(${n});
    k.doc.replaceWith({
      name: 'olcum-${n}',
      settings: { ...k.doc.settings.toJSON(), srid: 5256 },
      origin: { x: 486000, y: 4420000 },
      homeView: d.bounds,
      layers: d.layers,
      activeLayer: 'parsel',
      entities: d.entities,
      styles: { items: [], categories: [] },
    });
    k.selection.clear();
    const disk = (window.__kcadDisk = { blob: null });
    const handle = {
      name: 'olcum-${n}.kcad',
      getFile: async () => disk.blob ?? new Blob([]),
      createWritable: async () => {
        const parts = [];
        return { write: async (data) => { parts.push(data); }, close: async () => { disk.blob = new Blob(parts, { type: 'application/octet-stream' }); } };
      },
    };
    k.files.picker = { save: async () => handle, open: async () => handle };
    k.files.handle = null;
    // Where the time goes: the codec's calls and the worker's progress, stamped as the page hears them.
    const marks = (window.__marks = []);
    const mark = (what) => marks.push([what, performance.now()]);
    if (!k.files.__origKcad) k.files.__origKcad = k.files.kcad;
    k.files.kcad = async () => {
      const c = await k.files.__origKcad();
      return {
        ...c,
        encode: (d, p) => (mark('encode'), c.encode(d, (x) => (mark('encode:' + x.stage), p?.(x))).then((r) => (mark('encoded'), r))),
        decode: (b, p) => (mark('decode'), c.decode(b, (x) => (mark('decode:' + x.stage), p?.(x))).then((r) => (mark('decoded'), r))),
      };
    };
    // The open's window, where the app has one: the page's own stages (its checks, the drawing put on screen).
    if (k.files.opening && !k.files.__origOpening) k.files.__origOpening = k.files.opening;
    if (k.files.__origOpening)
      k.files.opening = async (name, cancel) => {
        const v = await k.files.__origOpening(name, cancel);
        return { ...v, step: (text, f) => (mark(text.startsWith('Çizim ekrana') ? 'view' : text.startsWith('Nesneler denetleniyor') ? 'checks' : 'step'), v.step(text, f)), close: () => (mark('closed'), v.close()) };
      };
    const tasks = (window.__longTasks = []);
    new PerformanceObserver((list) => { for (const e of list.getEntries()) tasks.push({ start: e.startTime, duration: e.duration }); }).observe({ type: 'longtask' });
    return { entities: k.doc.size, buildMs: performance.now() - t0 };
  })()`);
}

/** Runs one file operation in the page: its time, its outcome and the long tasks meanwhile. */
const operation = (call, check) =>
  b.eval(`(async () => {
    const k = window.kentos;
    const t0 = performance.now();
    const ok = await ${call};
    const t1 = performance.now();
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));
    const t2 = performance.now();
    await new Promise((r) => setTimeout(r, 100));
    const tasks = window.__longTasks.filter((t) => t.start >= t0 - 1 && t.start <= t2);
    // The first time each mark was heard, after the start.
    const stages = {};
    for (const [what, at] of window.__marks) if (at >= t0 && !(what in stages)) stages[what] = at - t0;
    return { ok, ms: t1 - t0, drawnMs: t2 - t0, longest: Math.max(0, ...tasks.map((t) => t.duration)), blocked: tasks.reduce((s, t) => s + t.duration - 50, 0), tasks: tasks.length, stages, ${check} };
  })()`);

const results = [];
let env0 = null;
const load0 = loadavg();
const startedAt = Date.now();
for (const n of sizes) {
  for (let run = 1; run <= runs; run++) {
    const env = await openApp();
    env0 ??= env;
    // A small save and open first: the formats worker and its module are loaded and compiled once.
    await prepare(1);
    await operation('k.files.saveAs()', 'x: 0');
    await operation('k.files.open()', 'x: 0');
    const prepared = await prepare(n);
    await settle();
    await gc();
    await sleep(300);
    const before = treePssMb(chromePid);
    const save = await sampled(() => operation('k.files.saveAs()', "bytes: window.__kcadDisk.blob?.size ?? 0, dirty: k.doc.dirty.value"));
    if (save.over) throw new Error(`Chrome ${Math.round(save.over)} MB kullandı (sınır ${memoryLimitMb} MB); ${n} parselde kayıt durduruldu.`);
    // The worker's memory goes back before the open (it would stop by itself after half a minute).
    await b.eval(`import('/src/io/client.ts').then((m) => m.formats().cancel())`).catch(() => {});
    await gc();
    await sleep(300);
    const beforeOpen = treePssMb(chromePid);
    const open = await sampled(() => operation('k.files.open()', 'size: k.doc.size, name: k.doc.name.value, dirty: k.doc.dirty.value'));
    if (open.over) throw new Error(`Chrome ${Math.round(open.over)} MB kullandı (sınır ${memoryLimitMb} MB); ${n} parselde açma durduruldu.`);
    const failed = !save.value.ok || save.value.dirty || !open.value.ok || open.value.size !== n;
    const log = failed ? await b.eval('window.kentos.log.entries.value.slice(-4).map((e) => e.text)') : [];
    results.push({
      parcels: n,
      run,
      buildMs: prepared.buildMs,
      bytes: save.value.bytes,
      save: { ms: save.value.ms, longestTaskMs: save.value.longest, blockedMs: save.value.blocked, longTasks: save.value.tasks, peakAboveMb: save.peakMb - before, chromeBeforeMb: before, stages: save.value.stages, ok: save.value.ok && !save.value.dirty },
      open: { ms: open.value.ms, drawnMs: open.value.drawnMs, longestTaskMs: open.value.longest, blockedMs: open.value.blocked, longTasks: open.value.tasks, peakAboveMb: open.peakMb - beforeOpen, chromeBeforeMb: beforeOpen, stages: open.value.stages, ok: open.value.ok && open.value.size === n },
      log,
    });
    const last = results.at(-1);
    if (process.env.KCAD_STAGES) console.log(JSON.stringify({ save: last.save.stages, open: last.open.stages }));
    console.log(
      `${n} parsel, koşu ${run}: ${(last.bytes / 1e6).toFixed(1)} MB; kayıt ${Math.round(last.save.ms)} ms (en uzun görev ${Math.round(last.save.longestTaskMs)} ms, +${Math.round(last.save.peakAboveMb)} MB); açma ${Math.round(last.open.ms)} ms, çizildi ${Math.round(last.open.drawnMs)} ms (en uzun görev ${Math.round(last.open.longestTaskMs)} ms, +${Math.round(last.open.peakAboveMb)} MB)${failed ? ` BAŞARISIZ: ${log.join(' | ')}` : ''}`,
    );
  }
}
b.close();
await server.close();

// ── Report ──────────────────────────────────────────────────────────────

const median = (xs) => {
  const s = xs.filter(Number.isFinite).sort((a, c) => a - c);
  if (!s.length) return null;
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};
const range = (xs) => [Math.min(...xs), Math.max(...xs)];
const summary = sizes.map((n) => {
  const rs = results.filter((r) => r.parcels === n);
  const pick = (f) => ({ median: median(rs.map(f)), range: range(rs.map(f)) });
  return {
    parcels: n,
    bytes: rs[0]?.bytes,
    ok: rs.every((r) => r.save.ok && r.open.ok),
    saveMs: pick((r) => r.save.ms),
    saveLongestTaskMs: pick((r) => r.save.longestTaskMs),
    savePeakAboveMb: pick((r) => r.save.peakAboveMb),
    openMs: pick((r) => r.open.ms),
    openDrawnMs: pick((r) => r.open.drawnMs),
    openLongestTaskMs: pick((r) => r.open.longestTaskMs),
    openPeakAboveMb: pick((r) => r.open.peakAboveMb),
    chromeWithDrawingMb: pick((r) => r.save.chromeBeforeMb),
  };
});
const chrome = execFileSync(process.env.CHROME_BIN ?? 'google-chrome', ['--version']).toString().trim();
const rustc = (() => {
  try {
    return execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim();
  } catch {
    return '?';
  }
})();
const report = {
  label,
  date: new Date().toISOString(),
  commit,
  dirtyTree,
  machine: { cpu: cpus()[0]?.model ?? '?', threads: cpus().length, memoryGb: Math.round(totalmem() / 2 ** 30), os: osName, kernel: release(), loadAverageAtStart: load0.map((v) => Math.round(v * 100) / 100), chrome, node: process.version, rustc },
  setup: { runs, sizes, memoryLimitMb, chromeArgs: CHROME_ARGS, backend: env0?.backend, glRenderer: env0?.glRenderer, crossOriginIsolated: env0?.isolated, server: 'vite dev (window.kentos), WASM --profile wasm', durationS: Math.round((Date.now() - startedAt) / 1000) },
  summary,
  runs: results,
};
mkdirSync(outDir, { recursive: true });
const base = `${outDir}/kcad-web-${label}-${date}`;
writeFileSync(`${base}.json`, `${JSON.stringify(report, null, 2)}\n`);

const n0 = (v) => (v === undefined || v === null ? '–' : Math.round(v).toLocaleString('en-US').replaceAll(',', ' '));
const mb = (bytes) => (bytes / 1e6).toFixed(1);
const cell = (m) => (m.median === null ? '–' : `${n0(m.median)} (${n0(m.range[0])}–${n0(m.range[1])})`);
const md = [];
md.push(`# KCAD v2 web'de kaydet ve aç: ${label} (${date}, ${commit}${dirtyTree ? ', kaydedilmemiş değişiklikle' : ''})`);
md.push('');
md.push(
  `${report.machine.cpu}, ${report.machine.threads} iş parçacığı, ${report.machine.memoryGb} GB; ${osName} (${release()}); ${chrome}, başsız, ${env0?.backend === 'webgl2' ? 'WebGL2' : env0?.backend} (${env0?.glRenderer ?? '?'}); Vite geliştirme sunucusu, biçim modülü \`--profile wasm\` (${rustc}); Node ${process.version}. ${runs} koşu, her hücre ortanca (aralık). Betik \`apps/web/scripts/perf/kcad.mjs\`; toplam ${Math.round(report.setup.durationS / 60)} dk. Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir (taslak); bu rapor yalnız kayıttır.`,
);
md.push('');
md.push(
  'Çizim: parsel başına 20 köşeli bir alan, üç öznitelik (Ada, Parsel, Nitelik) ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Kaydet” uygulamanın kendi Farklı kaydet\'idir (`files.saveAs()`, bellekteki bir dosyaya): çizimin alınması, biçim işçisinde kodlama ve doğrulama, dosyaya yazma. “Aç” uygulamanın Aç\'ıdır (`files.open()`): dosyanın okunması, işçide çözme, sayfada denetim ve belgenin değişmesi; “çizildi” ilk karenin çizildiği andır. “En uzun görev” sayfanın ana iş parçacığını kesintisiz tutan en uzun görevdir (Long Tasks API): kullanıcının donma olarak gördüğü süre. Bellek, işlem sürerken Chrome\'un bütün süreçlerinin PSS toplamındaki en yüksek artıştır (50 ms örnekleme); biçim işçisi kayıttan sonra durdurulur, açma kendi işçisini yeniden başlatır.',
);
md.push('');
md.push('| Parsel | Dosya | Kaydet | Kayıtta en uzun görev | Kayıtta bellek artışı | Aç | Aç: çizildi | Açışta en uzun görev | Açışta bellek artışı | Çizimle Chrome |');
md.push('|---|---|---|---|---|---|---|---|---|---|');
for (const s of summary)
  md.push(
    `| ${n0(s.parcels)} | ${s.bytes ? `${mb(s.bytes)} MB` : '–'} | ${cell(s.saveMs)} ms | ${cell(s.saveLongestTaskMs)} ms | ${cell(s.savePeakAboveMb)} MB | ${cell(s.openMs)} ms | ${cell(s.openDrawnMs)} ms | ${cell(s.openLongestTaskMs)} ms | ${cell(s.openPeakAboveMb)} MB | ${cell(s.chromeWithDrawingMb)} MB |`,
  );
md.push('');
if (summary.some((s) => !s.ok)) md.push('**Başarısız koşular var**: ham veride (`.json`, `log`) nedenleri yazar.');
writeFileSync(`${base}.md`, `${md.join('\n')}\n`);
console.log(`Yazıldı: ${base}.{json,md}`);
