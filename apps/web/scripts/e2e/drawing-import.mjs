// A real drawing imported the way a user does it (docs/adr/0138): Dosya → İçe aktar → NCZ or DXF,
// with the file named in KENTOS_NCZ or KENTOS_DXF (a user's drawing is theirs and stays out of the
// repository). The window is shot while the worker reads and once it has; then İçe aktar: the frames the
// page drew while the objects went in (their times), the panel half way, the drawing at the end and close
// to its smart objects' symbols. Last, one Geri al takes the whole import back, and Esc stops a second one
// half way with the drawing as it was. Not a gate: a measurement, and its screenshots in scripts/e2e/out/.
//
//   KENTOS_NCZ=/yol/plan.ncz CHROME_BIN="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
//     node scripts/e2e/drawing-import.mjs
//
// KENTOS_SRID gives the new project's system (the NCZ's own when it declares one, else 5256).
import { readFileSync, statSync } from 'node:fs';
import { basename } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep, WEBGPU_ARGS } from './cdp.mjs';

const path = process.env.KENTOS_NCZ ?? process.env.KENTOS_DXF;
if (!path) {
  console.error('KENTOS_NCZ ya da KENTOS_DXF ile içe aktarılacak dosyayı verin.');
  process.exit(2);
}
const source = process.env.KENTOS_NCZ ? 'ncz' : 'dxf';
const name = basename(path);
const tag = `import-${source}`;

// The file reaches the page as a user's pick would: its bytes, through the in-memory picker.
const serveFile = {
  name: 'kentos-import-file',
  configureServer(server) {
    server.middlewares.use('/__import-file', (_req, res) => {
      res.writeHead(200, { 'content-type': 'application/octet-stream' });
      res.end(readFileSync(path));
    });
  },
};
const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error', plugins: [serveFile] });
await server.listen();
// KENTOS_GPU=1: the machine's GPU (Metal) instead of the software one the checks use; a software GPU draws
// half a million objects in seconds a frame, which measures the software and not the import.
const GPU_ARGS = ['--use-angle=metal', '--ignore-gpu-blocklist', '--enable-gpu-rasterization', '--enable-unsafe-webgpu'];
const b = await launch(server.resolvedUrls.local[0], { width: 1600, height: 900, args: process.env.KENTOS_GPU ? GPU_ARGS : WEBGPU_ARGS });
const ms = (t) => `${Math.round(t)} ms`;

try {
  const ready = 'window.kentos && window.kentos.view.backendKind.value';
  await b.waitFor(ready, 30000);
  await sleep(1200); // first-load dependency optimisation can reload once
  await b.waitFor(ready, 30000);
  console.log(`dosya: ${name}, ${(statSync(path).size / 1e6).toFixed(1)} MB; çizim motoru: ${await b.eval('window.kentos.view.backendKind.value')}`);

  const srid = Number(process.env.KENTOS_SRID ?? 5256);
  await b.eval(`(async () => {
    for (const d of document.querySelectorAll('.dialog-backdrop')) d.remove();
    const { newProjectContent } = await import('/src/model/newProject.ts');
    await window.kentos.files.newProject(newProjectContent({ name: ${JSON.stringify(`İçe aktarma: ${name}`)}, srid: ${srid}, plotScale: 1000 }));
    const bytes = await (await fetch('/__import-file')).arrayBuffer();
    window.kentos.files.picker = { open: async () => ({ name: ${JSON.stringify(name)}, getFile: async () => new Blob([bytes]) }), save: async () => null };
    return bytes.byteLength;
  })()`);

  // The read: the window at once, its bar while the worker reads, the layers once it has.
  const t0 = Date.now();
  await b.eval(`window.kentos.commands.execute('file.import.${source}')`);
  await b.waitFor(`!!document.querySelector('.io-reading__bar > span')`, 20000);
  await b.waitFor(`parseFloat(document.querySelector('.io-reading__bar > span')?.style.width ?? '0') >= 20 || !document.querySelector('.io-reading')`, 60000);
  await b.shot(`${tag}-1-okunuyor`);
  await b.waitFor(`!!document.querySelector('.io-table tbody tr') || /okunamadı/.test(document.querySelector('.io-file__meta')?.textContent ?? '')`, 120000);
  console.log(`okuma: ${ms(Date.now() - t0)} (pencere açılıp dosya okunana dek)`);
  await sleep(300);
  await b.shot(`${tag}-2-pencere`);
  const summary = await b.eval(`document.querySelector('.io-summary')?.innerText.split('\\n')[0] ?? ''`);
  console.log(`özet: ${summary}`);
  if (await b.eval(`document.querySelector('.btn--primary').disabled`)) throw new Error(`İçe aktar kapalı: ${await b.eval(`document.querySelector('.dialog')?.innerText.slice(-600)`)}`);

  // The write: frames sampled from the page's own clock, each slice's own time, the panel shot half way.
  await b.eval(`(async () => {
    const { ProgressiveImport } = await import('/src/io/drawingImport.ts');
    const step = ProgressiveImport.prototype.step;
    window.__slices = [];
    ProgressiveImport.prototype.step = function (budget) {
      const t = performance.now();
      const r = step.call(this, budget);
      window.__slices.push([budget, performance.now() - t]);
      return r;
    };
  })()`);
  await b.eval(`(() => {
    window.__frames = [];
    let last = performance.now();
    const tick = () => { const now = performance.now(); window.__frames.push(now - last); last = now; if (!window.__stopFrames) requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
    window.__t = performance.now();
    document.querySelector('.btn--primary').click();
  })()`);
  await b.waitFor(`(() => { const p = document.querySelector('.importing__bar'); return !p || Number(p.getAttribute('aria-valuenow')) >= 40; })()`, 120000);
  await b.shot(`${tag}-3-yazilirken`);
  await b.waitFor(`!document.querySelector('.importing')`, 600000);
  const run = await b.eval(`(() => {
    window.__stopFrames = true;
    const f = window.__frames.slice(1).sort((a, b) => a - b);
    const q = (p) => Math.round(f[Math.floor(p * (f.length - 1))] ?? 0);
    const s = window.__slices.map(([, t]) => t);
    return { total: Math.round(performance.now() - window.__t), frames: f.length, p50: q(0.5), p90: q(0.9), p99: q(0.99), max: Math.round(f.at(-1) ?? 0), size: window.kentos.doc.size,
      slices: s.length, work: Math.round(s.reduce((a, t) => a + t, 0)), longest: Math.round(Math.max(...s)), budgets: [...new Set(window.__slices.map(([b]) => Math.round(b)))].slice(0, 12) };
  })()`);
  console.log(`yazma: ${ms(run.total)}, ${run.frames} kare; kare süresi ortanca ${run.p50} ms, %90 ${run.p90} ms, %99 ${run.p99} ms, en uzun ${run.max} ms; çizimde ${run.size} nesne`);
  console.log(`dilimler: ${run.slices}, içe aktarmanın kendi işi ${ms(run.work)}, en uzun dilim ${ms(run.longest)}; bütçeler ${run.budgets.join(', ')} ms`);
  await sleep(1500);
  await b.shot(`${tag}-4-bitti`);

  // Close to the smart objects: the first settlement symbol, 60 m round it.
  const near = await b.eval(`(() => {
    const k = window.kentos;
    const e = k.doc.all().find((x) => x.attrs?.['Akıllı nesne'] === 'Yerleşim') ?? k.doc.all().find((x) => x.kind === 'text');
    if (!e) return false;
    const b = k.doc.bounds([e.id]);
    const cx = (b.minX + b.maxX) / 2, cy = (b.minY + b.maxY) / 2;
    k.view.camera.fit({ minX: cx - 60, minY: cy - 34, maxX: cx + 60, maxY: cy + 34 });
    return true;
  })()`);
  if (near) {
    await sleep(1500);
    await b.shot(`${tag}-5-yakin`);
  }

  // One step takes it all back; Esc stops a second import half way and leaves the drawing as it was.
  const undone = await b.eval(`(() => { const k = window.kentos; const label = k.doc.undo(); return { label, size: k.doc.size }; })()`);
  console.log(`geri al: “${undone.label}”, çizimde ${undone.size} nesne`);
  await b.eval(`window.kentos.commands.execute('file.import.${source}')`);
  await b.waitFor(`!!document.querySelector('.io-table tbody tr')`, 120000);
  await b.eval(`document.querySelector('.btn--primary').click()`);
  await b.waitFor(`(() => { const p = document.querySelector('.importing__bar'); return !p || Number(p.getAttribute('aria-valuenow')) >= 30; })()`, 120000);
  await b.key('Escape');
  await b.waitFor(`!document.querySelector('.importing')`, 20000);
  const stopped = await b.eval(`({ size: window.kentos.doc.size, busy: window.kentos.doc.busy, undo: window.kentos.doc.undo() })`);
  console.log(`Esc: çizimde ${stopped.size} nesne, açık grup ${stopped.busy ? 'var' : 'yok'}, geri alınacak adım ${stopped.undo === null ? 'yok' : stopped.undo}`);
} finally {
  b.close();
  await server.close();
}
