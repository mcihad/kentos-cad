// Visual comparison of the interface (pnpm e2e:visual): screenshots of the chrome (menu bar, ribbon,
// panels, status bar, dialogs, the start screen, the application menu) in both themes, every accent
// colour and the large type size, compared pixel by pixel with the reference images in
// scripts/e2e/visual/. The drawing itself is left out: it depends on the GPU (WebGL2 on SwiftShader here,
// a real card elsewhere), and the smoke test checks what it draws.
//
// No dependencies: the images are compared in the page itself (two canvases, getImageData). A region
// fails when more than MAX_SHARE of its pixels differ by more than PIXEL_TOLERANCE (sum of the channels);
// the differing pixels are painted red in scripts/e2e/out/visual-<name>-diff.png.
//
// The references are recorded in the cloud container (Linux, headless Chromium, the bundled faces);
// font rendering differs a little between systems, so on another machine record them once with
// `pnpm e2e:visual --update` and compare after that. A deliberate change of the look is recorded the
// same way; the new images are read in the diff before they are committed.
//
//   node scripts/e2e/visual.mjs [--update] [--only ribbon-dark,start]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const args = process.argv.slice(2);
const update = args.includes('--update');
const only = args.includes('--only') ? args[args.indexOf('--only') + 1].split(',') : null;
const REF = new URL('./visual/', import.meta.url).pathname;
const PIXEL_TOLERANCE = 48;
const MAX_SHARE = 0.004;

const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
mkdirSync(REF, { recursive: true });

const results = [];
let failed = 0;

/** A fresh page in a known state: a new profile, the demo drawing, WebGL2, 1440 × 860. */
async function page(prefs = {}, ui = {}) {
  const b = await launch('about:blank', { width: 1440, height: 860 });
  // Preferences go in before the app reads them.
  await b.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `try { localStorage.setItem('kentos.prefs.v1', ${JSON.stringify(JSON.stringify(prefs))}); localStorage.setItem('kentos.ui.v1', ${JSON.stringify(JSON.stringify(ui))}); } catch {}`,
  });
  await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  // The server check settles (no API here: "Sunucu: yok"), the faces load, the first frames are drawn.
  await b.waitFor(`window.kentos.server.state.value !== 'checking'`, 10000).catch(() => {});
  await b.eval('document.fonts.ready');
  await sleep(900);
  // Nothing hovered, focused or blinking; the status line's message cleared.
  await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1, y: 858, button: 'none' });
  await b.eval(`(() => { document.activeElement?.blur?.(); window.kentos.log.clear(); document.querySelector('.status__flash')?.replaceChildren(); })()`);
  await sleep(300);
  return b;
}

const rect = (b, sel) => b.eval(`(() => { const r = document.querySelector(${JSON.stringify(sel)})?.getBoundingClientRect(); return r ? { x: Math.floor(r.left), y: Math.floor(r.top), width: Math.ceil(r.width), height: Math.ceil(r.height) } : null; })()`);

/** Compares one region with its reference (or records it with --update). */
async function region(b, name, clip) {
  if (only && !only.some((o) => name.startsWith(o))) return;
  if (!clip) {
    results.push({ name, ok: false, note: 'bölge bulunamadı' });
    failed++;
    return;
  }
  const r = await b.send('Page.captureScreenshot', { format: 'png', clip: { ...clip, scale: 1 } });
  const refFile = join(REF, `${name}.png`);
  if (update || !existsSync(refFile)) {
    writeFileSync(refFile, Buffer.from(r.data, 'base64'));
    results.push({ name, ok: true, note: update ? 'kaydedildi' : 'kaynak yoktu, kaydedildi' });
    return;
  }
  const ref = readFileSync(refFile).toString('base64');
  const cmp = await b.eval(`(async () => {
    const load = (src) => new Promise((ok, no) => { const i = new Image(); i.onload = () => ok(i); i.onerror = no; i.src = src; });
    const [a, c] = await Promise.all([load('data:image/png;base64,${r.data}'), load('data:image/png;base64,${ref}')]);
    if (a.width !== c.width || a.height !== c.height) return { size: [a.width, a.height, c.width, c.height] };
    const cv = (img) => { const k = document.createElement('canvas'); k.width = img.width; k.height = img.height; const g = k.getContext('2d'); g.drawImage(img, 0, 0); return [k, g, g.getImageData(0, 0, img.width, img.height)]; };
    const [, , da] = cv(a);
    const [kc, gc, dc] = cv(c);
    let bad = 0;
    for (let i = 0; i < da.data.length; i += 4) {
      const d = Math.abs(da.data[i] - dc.data[i]) + Math.abs(da.data[i + 1] - dc.data[i + 1]) + Math.abs(da.data[i + 2] - dc.data[i + 2]);
      if (d > ${PIXEL_TOLERANCE}) { bad++; dc.data[i] = 255; dc.data[i + 1] = 0; dc.data[i + 2] = 0; }
    }
    gc.putImageData(dc, 0, 0);
    return { bad, total: da.data.length / 4, diff: bad ? kc.toDataURL('image/png').split(',')[1] : null };
  })()`);
  if (cmp.size) {
    results.push({ name, ok: false, note: `boyut farklı: ${cmp.size[0]}×${cmp.size[1]}, kaynak ${cmp.size[2]}×${cmp.size[3]}` });
    writeFileSync(join(OUT, `visual-${name}.png`), Buffer.from(r.data, 'base64'));
    failed++;
    return;
  }
  const share = cmp.bad / cmp.total;
  const ok = share <= MAX_SHARE;
  if (!ok) {
    failed++;
    writeFileSync(join(OUT, `visual-${name}.png`), Buffer.from(r.data, 'base64'));
    writeFileSync(join(OUT, `visual-${name}-diff.png`), Buffer.from(cmp.diff, 'base64'));
  }
  results.push({ name, ok, note: `${cmp.bad} / ${cmp.total} piksel farklı (%${(share * 100).toFixed(2)})` });
}

/** The chrome of a page: the top bar (menu and toolbar, or ribbon), the right dock, the status bar. */
async function chrome(b, prefix) {
  const top = (await rect(b, '.ribbon')) ?? (await rect(b, '.shell__chrome'));
  await region(b, `${prefix}-top`, top);
  await region(b, `${prefix}-dock`, await rect(b, '.shell__right, .dock'));
  // The status bar's right side: toggles, scale, mode, system, server, engine (the left side shows live messages).
  const status = await rect(b, '.status');
  const toggles = await rect(b, '.status__toggles');
  if (status && toggles) await region(b, `${prefix}-status`, { x: toggles.x, y: status.y, width: status.x + status.width - toggles.x, height: status.height });
}

try {
  // Classic shell and ribbon, dark and light.
  for (const theme of ['dark', 'light']) {
    for (const shell of ['classic', 'ribbon']) {
      const b = await page({ shell }, { theme });
      if (shell === 'ribbon') await b.waitFor(`!!document.querySelector('.ribbon .rpanel')`, 20000);
      await sleep(400);
      await chrome(b, `${shell}-${theme}`);
      b.close();
    }
  }
  // Accent colours: a running tool (filled accent button) and a checked toggle show them.
  for (const accent of ['navy', 'amber', 'teal', 'bordeaux']) {
    const b = await page({ shell: 'ribbon', accent }, { theme: 'dark' });
    await b.waitFor(`!!document.querySelector('.ribbon .rpanel')`, 20000);
    await b.eval(`window.kentos.commands.execute('tool.line')`);
    await sleep(400);
    await region(b, `accent-${accent}`, await rect(b, '.ribbon'));
    b.close();
  }
  // Large type.
  {
    const b = await page({ shell: 'ribbon', uiScale: 'large' }, { theme: 'dark' });
    await b.waitFor(`!!document.querySelector('.ribbon .rpanel')`, 20000);
    await sleep(400);
    await chrome(b, 'large');
    b.close();
  }
  // Windows: application settings, the start screen, the application menu (dark and light).
  for (const theme of ['dark', 'light']) {
    const b = await page({ shell: 'classic' }, { theme });
    await b.eval(`window.kentos.commands.execute('tools.options')`);
    await b.waitFor(`!!document.querySelector('.dialog')`, 10000);
    await sleep(500);
    await region(b, `settings-${theme}`, await rect(b, '.dialog'));
    await b.key('Escape');
    await b.eval(`window.kentos.commands.execute('file.start')`);
    await b.waitFor(`!!document.querySelector('.start')`, 10000);
    await b.eval('document.activeElement?.blur?.()');
    await sleep(500);
    await region(b, `start-${theme}`, await rect(b, '.start'));
    await b.key('Escape');
    await sleep(200);
    const brand = await rect(b, '.menubar .brand');
    await b.click(brand.x + brand.width / 2, brand.y + brand.height / 2);
    await b.waitFor(`!!document.querySelector('.appmenu')`, 10000);
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1, y: 858, button: 'none' });
    await b.eval('document.activeElement?.blur?.()');
    await sleep(500);
    await region(b, `appmenu-${theme}`, await rect(b, '.appmenu'));
    b.close();
  }
} finally {
  await server.close();
}

for (const r of results) console.log(`${r.ok ? '✓' : '✗'} ${r.name}: ${r.note}`);
console.log(`\n${results.length - failed}/${results.length} bölge kaynakla aynı${update ? ' (kaynaklar yeniden kaydedildi)' : ''}.`);
if (failed) {
  console.log(`Farklı olanlar ve kırmızı işaretli farkları: ${OUT}/visual-*.png`);
  process.exit(1);
}
