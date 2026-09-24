// Screenshots of the symbol catalogue (docs/STYLE.md §7): opens the app on a
// running dev server, zooms to the catalogue cells whose symbol id starts
// with a prefix and saves a screenshot per page of cells.
//
//   node scripts/showcase/shot.mjs <id-prefix> [--port 5199] [--renderer webgl2|webgpu] [--theme light|dark] [--rows 3] [--out dir] [--scale 1]
//
// Rows of 8 cells are shot `--rows` at a time so symbols show near their
// paper size (1 mm = 1 m at 1:1000). `--scale` multiplies the zoom.
import { launch, sleep, WEBGPU_ARGS } from '../e2e/cdp.mjs';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
const opt = (name, def) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : def;
};
const prefix = args.find((a) => !a.startsWith('--') && !args[args.indexOf(a) - 1]?.startsWith('--')) ?? 'mpyy.';
const port = opt('port', '5199');
const renderer = opt('renderer', 'webgl2');
const rows = Number(opt('rows', '3'));
const scale = Number(opt('scale', '1'));
const outDir = resolve(opt('out', 'scripts/showcase/out'));
const theme = opt('theme', 'light');
mkdirSync(outDir, { recursive: true });

const b = await launch(`http://localhost:${port}/?renderer=${renderer}`, { width: 1600, height: 1000, args: renderer === 'webgpu' ? WEBGPU_ARGS : [] });
try {
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 20000);
  // Paper (light theme) by default, as the regulation prints; the toolbox out of the way.
  await b.eval(`(() => { window.kentos.commands.execute('view.theme.${theme}'); window.kentos.settings.grid.set(false); for (const e of document.querySelectorAll('.toolbox')) e.style.visibility = 'hidden'; return document.documentElement.dataset.theme; })()`).then((t) => console.log('theme', t));
  await sleep(600);
  const cells = await b.eval(`(() => {
    const k = window.kentos;
    const list = [...k.doc.all()].filter((e) => e.symbol && e.symbol.startsWith(${JSON.stringify(prefix)}));
    return list.map((e) => { const b = k.doc.bounds([e.id]); return { id: e.symbol, b }; });
  })()`);
  if (!cells.length) throw new Error(`no catalogue cells for "${prefix}"`);
  // Group cells into rows by their top edge, then shoot `rows` rows at a time.
  const byRow = new Map();
  for (const c of cells) {
    const key = Math.round(c.b.maxY / 10);
    if (!byRow.has(key)) byRow.set(key, []);
    byRow.get(key).push(c);
  }
  const rowList = [...byRow.entries()].sort((a, b) => b[0] - a[0]).map(([, v]) => v);
  let page = 0;
  for (let i = 0; i < rowList.length; i += rows) {
    const group = rowList.slice(i, i + rows).flat();
    const box = group.reduce((a, c) => ({ minX: Math.min(a.minX, c.b.minX), minY: Math.min(a.minY, c.b.minY), maxX: Math.max(a.maxX, c.b.maxX), maxY: Math.max(a.maxY, c.b.maxY) }), { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
    // Room for the labels under the cells.
    box.minY -= 9;
    await b.eval(`(() => { const k = window.kentos; k.view.camera.fit(${JSON.stringify(box)}, 24); if (${scale} !== 1) k.view.camera.zoomAt(${scale}, { x: k.view.camera.width / 2, y: k.view.camera.height / 2 }); k.view.requestRender(); })()`);
    await sleep(700);
    const name = `${prefix.replace(/[^a-z0-9.-]/gi, '_')}-${String(++page).padStart(2, '0')}`;
    const clip = await b.eval(`(() => { const r = document.querySelector('.viewport__gl').getBoundingClientRect(); return { x: r.left, y: r.top, width: r.width, height: r.height }; })()`);
    await b.shot(name, clip, outDir);
    console.log(`${outDir}/${name}.png  (${group.map((c) => c.id.split('.').pop()).join(', ')})`);
  }
  const errs = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  if (errs.length) console.log('console errors:\n' + errs.slice(0, 10).join('\n'));
} finally {
  b.close();
}
