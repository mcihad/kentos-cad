// The sheets' PDF from the web, checked with the tools the design names (docs/sheet/design.md §9a “Kabul
// ölçütleri”). On the demo drawing, the ifraz system template's sheet is made into a PDF in the app
// (app/sheet/pdfExport.ts: the core's writer, the maps from the web's drawing pipeline as vectors, the desktop's
// font files), then:
//
// - pdfinfo: the page is A3 landscape, 1190.55 × 841.89 pt;
// - pdffonts: every font embedded and subset;
// - pdftotext: the Turkish title, the place's values and the parcel table's values;
// - gdalinfo: the system (TUREF / TM36, EPSG:5256), the NEATLINE on the map frame's ground within 1 mm, the page's
//   corners within 1 cm, and every drawing layer of the map in the PDF's layer list (-mdd LAYERS);
// - pdftoppm -r 150: the page as a picture, into scripts/e2e/out/shots/sheet-pdf/, to look at.
//
// And in the same page:
// - the faces' files are fetched by the first export and not before, only those the core asks for;
// - the core's golden PDF (fixtures/sheet/v1/pdf: the inputs Rust's own test writes) comes out of the browser
//   byte for byte, with the font files as the app fetches them: the web's writer and fonts are the desktop's;
// - the export window: Ctrl+P on the sheet opens it as Yazdır (its one primary button), which puts the PDF in the print frame,
//   Yeni sekmede aç in the tab opened at the click, Kaydet through the file picker: the same bytes each time;
//   Hepsi writes both sheets, a page each, under the drawing's name;
// - the fallback: with a pattern-filled layer (no vector form here) the window names the layer and the map goes as
//   a picture at the fallback dpi, still georeferenced;
// - in a system that is not transverse Mercator (TUREF geographic) GeoPDF is off and the window says why;
// - a legend: its rows' symbols go as the paint's pictures, its labels as the layers' names;
// - a turned map: the core's NEATLINE is the frame the labels are kept by (a label is written when its anchor is
//   inside the frame, frameLabels.ts);
// - Yazdır ve pafta (`file.print`) with a sheet in front opens the sheet's Yazdır (W-17);
// - outlines: a washer marker's hole stays empty and a filled circle keeps its fill, in the map's picture (what the
//   screen, the gallery's pictures, PNG and SVG draw) and in the PDF's vectors;
// - an SVG picture: the export window says first that it goes as a picture (the core's finding); the PNG the browser
//   draws of it at its frame's pixels (pdfSvgSizes) is embedded in its place, and the finding is said in the log;
// - the fallback map picture has no paper under it: a soft mask in the PDF, as the desktop's.
//
// With --masaustu <pdf> (the desktop's PDF of the same sheet on the same drawing) the two are compared: the page,
// faces, NEATLINE and layers the same; every text the core draws outside the map at the same place; the maps'
// differences listed (the sheets' values may differ: they are the inputs, not the writer).
//
// A failed check fails the run.
//
//   node apps/web/scripts/e2e/sheet-pdf.mjs [--masaustu .run/shots/sheet-pdf/masaustu-ifraz.pdf]
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const WEB = fileURLToPath(new URL('../..', import.meta.url));
const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const DIR = join(OUT, 'shots', 'sheet-pdf');
mkdirSync(DIR, { recursive: true });
const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};
const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8' });
// --masaustu <pdf>: the desktop's PDF of the same sheet, compared below (its sheet's values may differ).
const desktopAt = process.argv.indexOf('--masaustu');
const DESKTOP = desktopAt > 0 ? process.argv[desktopAt + 1] : null;
/** A PDF's words with their boxes, millimetres on the page (pdftotext -bbox). */
const wordsOf = (file) =>
  [...run('pdftotext', ['-bbox', file, '-']).matchAll(/<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">([^<]*)<\/word>/g)].map((m) => ({
    text: m[5].replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"'),
    box: [1, 2, 3, 4].map((i) => (Number(m[i]) * 25.4) / 72),
  }));
/** Words of `a` not in `b` (as many times as they are more). */
const minus = (a, b) => {
  const left = new Map();
  for (const w of b) left.set(w.text, (left.get(w.text) ?? 0) + 1);
  return a.filter((w) => {
    const n = left.get(w.text) ?? 0;
    if (n) left.set(w.text, n - 1);
    return !n;
  }).map((w) => w.text);
};
/** The largest box difference (mm) over the words both have once. */
const apart = (a, b) => {
  const once = (ws) => new Map([...Map.groupBy(ws, (w) => w.text)].filter(([, v]) => v.length === 1).map(([t, v]) => [t, v[0]]));
  const ma = once(a);
  const mb = once(b);
  const common = [...ma.keys()].filter((t) => mb.has(t));
  const d = common.map((t) => [t, Math.max(...ma.get(t).box.map((v, i) => Math.abs(v - mb.get(t).box[i])))]).sort((x, y) => y[1] - x[1]);
  return { common: common.length, worst: d[0] ?? ['', 0], median: d.length ? d[d.length >> 1][1] : 0 };
};
const sha = (buf) => createHash('sha256').update(buf).digest('hex');

const server = await createServer({ root: WEB, configFile: join(WEB, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const b = await launch('about:blank', { width: 1440, height: 900 });
const S = `const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts'); const pe = await import('/src/app/sheet/pdfExport.ts');
  const b64 = (bytes) => { let bin = ''; for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000)); return btoa(bin); };`;
// Every request the page makes, from the start (the resource timing buffer of a dev page fills before the export).
const requests = [];
b.on('Network.requestWillBeSent', (p) => requests.push(p.request.url));
await b.send('Network.enable');
const fontsIn = (urls) => [...new Set(urls.filter((u) => /\.ttf\b/.test(u)).map((u) => decodeURIComponent(u).replace(/[?#].*$/, '').split('/').pop()))].sort();
const page = (body) => b.eval(`(async () => { ${S} ${body} })()`);
const until = async (expr, what, ms = 30000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (await b.eval(`(async () => !!(${expr}))()`).catch(() => false)) return;
    await sleep(100);
  }
  throw new Error(`beklenen olmadı: ${what}`);
};
const VALUES = { il: 'Sivas', ilce: 'Suşehri', mahalle: 'Kızılırmak', ada: '1245', pafta_no: 'P-12', kontrol_eden: 'Ali Kaya', onaylayan: 'Veli Demir' };
const COLUMNS = [
  { heading: 'Parsel', value: 'Parsel', width: 13000, align: 'center' },
  { heading: 'Yüzölçümü (m²)', value: '$alan', width: 0, align: 'right', decimals: 2 },
  { heading: 'Tapu alanı (m²)', value: '[Tapu alanı (m²)]', width: 0, align: 'right', decimals: 2 },
  { heading: 'Nitelik', value: 'Nitelik', width: 0, align: 'left' },
];
try {
  await b.send('Page.navigate', { url: `${server.resolvedUrls.local[0]}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await sleep(1500);

  // ── 1. The ifraz sheet (and the aplikasyon sketch, for Hepsi), its PDF written in the page ──────────────
  const sheet = await page(`
    k.doc.settings.workspace.set('cad');
    const e = await s.ensureEngine();
    const tpl = (id) => e.systemTemplates().find((x) => x.meta.id === id);
    const values = ${JSON.stringify(VALUES)};
    const id = await ta.sheetFromTemplate(s, tpl('sys:ifraz-paftasi'), null, undefined, Object.entries(values).map(([name, value]) => ({ name, value })));
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const table = sh.items.find((i) => i.kind.type === 'table');
    s.apply([{ op: 'setItemProps', id: table.id, patch: { kind: { columns: ${JSON.stringify(COLUMNS)}, source: { sort: [{ expression: 'to_int(Parsel)', descending: false }] } } } }], 'Parsel tablosu');
    await ta.sheetFromTemplate(s, tpl('sys:aplikasyon-krokisi'), null);
    s.openSheet(id);
    await new Promise((r) => setTimeout(r, 1500));
    return { id };
  `);
  const mark = requests.length;
  const made = await page(`
    const id = ${JSON.stringify(sheet.id)};
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const list = s.plan(s.book(), id);
    const map = list.prims.find((p) => p.type === 'map');
    const texts = list.prims.filter((p) => p.type === 'text').map((p) => p.text);
    const choice = { sheets: [id], dpi: 300, geo: true, layers: true };
    let bytes;
    try {
      bytes = await pe.makePdf(k, s, choice);
    } catch (x) {
      return { error: (x.code ? x.code + ': ' : '') + x.message };
    }
    const mc = await import('/src/app/sheet/mapContent.ts');
    const layers = (mc.vectorMap(k, map)?.layers ?? []).map((l) => l.name);
    return { id, pdf: b64(bytes), map: { clip: map.clip, view: map.view }, texts, page: sh.page, layers, name: pe.pdfName(k, s, choice) };
  `);
  if (made.error) throw new Error(`PDF yazılamadı: ${made.error}`);
  const pdf = Buffer.from(made.pdf, 'base64');
  const file = join(DIR, 'ifraz-paftasi.pdf');
  writeFileSync(file, pdf);
  console.log(`PDF: ${file} (${Math.round(pdf.length / 1024)} kB, “${made.name}.pdf”)`);

  // ── 2. The tools ──────────────────────────────────────────────────────────────────────────────────────
  const info = run('pdfinfo', [file]);
  const size = /Page size:\s+([\d.]+) x ([\d.]+) pts/.exec(info);
  check('pdfinfo: A3 landscape, 1190.55 × 841.89 pt', !!size && Math.abs(Number(size[1]) - 1190.55) < 0.01 && Math.abs(Number(size[2]) - 841.89) < 0.01, size ? `${size[1]} × ${size[2]}` : info.slice(0, 200));
  check('pdfinfo: produced by KentOS, titled by the sheet', /Producer:\s+KentOS/.test(info) && /Title:\s+İfraz paftası/.test(info), [/Title:.*/.exec(info)?.[0], /Producer:.*/.exec(info)?.[0]].join('; '));

  // name, type, encoding, emb, sub, uni, object: a subset's name has six capitals and a plus before it.
  const fonts = run('pdffonts', [file]).split('\n').slice(2).filter(Boolean);
  check('pdffonts: every font embedded and subset, with a Unicode map', fonts.length > 0 && fonts.every((l) => /^[A-Z]{6}\+/.test(l) && /\byes\s+yes\s+yes\b/.test(l)), fonts.map((l) => l.split(/\s+/)[0]).join(', '));

  const text = run('pdftotext', ['-layout', file, '-']);
  const want = ['İFRAZ / TEVHİT PAFTASI', 'Sivas', 'Suşehri', 'Kızılırmak', '1245', 'P-12', 'Ali Kaya', 'Veli Demir', 'PARSEL TABLOSU', 'Yüzölçümü (m²)', 'Kargir ev ve arsası'];
  const missing = want.filter((w) => !text.includes(w));
  check('pdftotext: the Turkish title, headings and the place’s values', !missing.length, missing.length ? `eksik: ${missing.join(', ')}` : want.join(', '));
  const table = made.texts.filter((t) => /^\d+[.,]\d{2}$/.test(t));
  const tableMissing = table.filter((t) => !text.includes(t));
  check('pdftotext: every value of the parcel table as the sheet writes it', table.length > 0 && !tableMissing.length, tableMissing.length ? `eksik: ${tableMissing.join(', ')}` : `${table.length} değer, ilki ${table[0]}`);
  writeFileSync(join(DIR, 'ifraz-paftasi.txt'), text);

  const gi = JSON.parse(run('gdalinfo', ['-json', file]));
  const wkt = gi.coordinateSystem?.wkt ?? '';
  check('gdalinfo: the system is TUREF / TM36 (EPSG:5256)', /^PROJCRS\["TUREF \/ TM36"/.test(wkt) && /ID\["EPSG",5256\]\]\s*$/.test(wkt), (/PROJCRS\["([^"]+)"/.exec(wkt) ?? ['', wkt.slice(0, 80)])[1]);
  // The map's ground: its view's centre and scale over the frame's content box (µm on the paper; the ifraz map is not turned).
  const c = made.map.clip;
  const v = made.map.view;
  const mPerUm = v.scale / 1_000_000;
  const ground = (xUm, yUm) => [v.center.x + (xUm - (c.left + c.width / 2)) * mPerUm, v.center.y - (yUm - (c.top + c.height / 2)) * mPerUm];
  const box = [ground(c.left, c.top), ground(c.left + c.width, c.top), ground(c.left + c.width, c.top + c.height), ground(c.left, c.top + c.height)];
  // NEATLINE: the map's frame on the ground as GDAL reads the PDF's viewport.
  const neat = (/POLYGON \(\((.+)\)\)/.exec(gi.metadata?.['']?.NEATLINE ?? '')?.[1] ?? '').split(',').filter(Boolean).map((p) => p.trim().split(/\s+/).map(Number));
  const far = (pts, wanted) => Math.max(...wanted.map((w) => Math.min(...pts.map((p) => Math.hypot(p[0] - w[0], p[1] - w[1])))));
  const neatOff = neat.length ? far(neat, box) : Infinity;
  check('gdalinfo: NEATLINE is the map frame on its ground, within 1 mm', neatOff < 0.001, `en büyük fark ${(neatOff * 1000).toFixed(3)} mm; ${neat.length ? neat.slice(0, 4).map((p) => p.map((x) => x.toFixed(3)).join(' ')).join(' | ') : 'NEATLINE yok'}`);
  // The page's corners: GDAL's georeference carried to the paper's edges.
  const paper = made.page.size;
  const corners = { upperLeft: ground(0, 0), upperRight: ground(paper.width, 0), lowerRight: ground(paper.width, paper.height), lowerLeft: ground(0, paper.height) };
  const cc = gi.cornerCoordinates ?? {};
  const pageOff = Math.max(...Object.entries(corners).map(([key, w]) => (cc[key] ? Math.hypot(cc[key][0] - w[0], cc[key][1] - w[1]) : Infinity)));
  check('gdalinfo: the page’s corners on the ground, within 1 cm', pageOff < 0.01, `en büyük fark ${(pageOff * 100).toFixed(2)} cm; sol üst ${cc.upperLeft?.join(', ')}`);
  const layerList = run('gdalinfo', ['-mdd', 'LAYERS', file]);
  const named = [...layerList.matchAll(/LAYER_\d+_NAME=(.+)/g)].map((x) => x[1].trim());
  const absent = made.layers.filter((n) => !named.includes(n.replace(/ /g, '_')));
  check('gdalinfo -mdd LAYERS: every drawing layer of the map is a PDF layer', made.layers.length > 0 && !absent.length, absent.length ? `eksik: ${absent.join(', ')}` : `${named.length} katman`);

  run('pdftoppm', ['-r', '150', '-png', file, join(DIR, 'ifraz-paftasi')]);
  console.log(`sayfa resmi: ${join(DIR, 'ifraz-paftasi-1.png')}`);

  // ── 3. The faces' files: fetched by the export, not before, and only those asked ───────────────────────
  const before = fontsIn(requests.slice(0, mark));
  const fetched = fontsIn(requests.slice(mark));
  check('fonts: no .ttf before the first export; after it, one file per face the PDF embeds', !before.length && fetched.length > 0 && fetched.length === fonts.length, `önce ${before.length ? before.join(', ') : 'yok'}; sonra ${fetched.join(', ')}`);

  // ── 4. The core's golden PDF, from the browser ────────────────────────────────────────────────────────
  const FIX = join(ROOT, 'fixtures/sheet/v1/pdf');
  const golden = JSON.parse(readFileSync(join(FIX, 'ifraz.json'), 'utf8'));
  const gold = await page(`
    const { bookText } = await import('/src/product/sheet/engine.ts');
    const { toBase64 } = await import('/src/product/sheet/store.ts');
    const pf = await import('/src/app/sheet/pdfFonts.ts');
    const e = await s.ensureEngine();
    const inputs = JSON.parse(${JSON.stringify(readFileSync(join(FIX, 'ifraz-inputs.json'), 'utf8'))});
    const book = bookText(JSON.parse(${JSON.stringify(readFileSync(join(FIX, 'ifraz-book.json'), 'utf8'))}));
    const got = await pf.fontFiles(inputs.fonts.map((f) => pe.faceFile(f)));
    for (const f of inputs.fonts) f.data = toBase64(got.get(pe.faceFile(f)));
    const bytes = e.toPdf(book, inputs, {});
    const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map((x) => x.toString(16).padStart(2, '0')).join('');
    return { size: bytes.length, digest };
  `);
  check('golden: the core’s fixture PDF comes out of the browser byte for byte (Rust’s bytes, the desktop’s fonts)', gold.size === golden.size && gold.digest === golden.sha256, `${gold.size} bayt, ${gold.digest.slice(0, 16)}…; altın ${golden.size}, ${golden.sha256.slice(0, 16)}…`);

  // ── 5. The export window: Yazdır, Yeni sekmede aç, Kaydet, Hepsi ──────────────────────────────────────
  await page(`
    window.__pdf = { saved: null, name: null, opened: null };
    window.__keep = { picker: k.files.picker, open: window.open };
    k.files.picker = { ...k.files.picker, save: async (name) => ({ name, createWritable: async () => { const parts = []; return { write: async (d) => parts.push(d), close: async () => { window.__pdf.saved = new Uint8Array(await new Blob(parts).arrayBuffer()); window.__pdf.name = name; } }; } }) };
    window.open = (url, target) => { const rec = (window.__pdf.opened = { url, target, href: null, closed: false }); return { location: { set href(x) { rec.href = x; }, get href() { return rec.href; } }, close() { rec.closed = true; } }; };
    return true;`);
  const DLG = `document.querySelector('.sheet-export')?.closest('.dialog')`;
  const button = (label) => `[...(${DLG}?.querySelectorAll('button') ?? [])].find((x) => x.textContent.trim() === ${JSON.stringify(label)})`;
  const ANYWAY = `document.querySelector('button[role=switch][aria-label="Hatalar varken yine de aktar"][aria-checked="false"]')?.click(); return true;`;
  const openWindow = async (command) => {
    if (command === 'Ctrl+P') {
      // The keyboard's way: the sheet is in front, the focus on its stage.
      await page(`document.querySelector('.sheet-ws:not([hidden]) .sheet-stage')?.focus(); return true;`);
      await b.key('p', { ctrl: true });
    } else await page(`k.commands.execute(${JSON.stringify(command)}); return true;`);
    await until(`document.querySelector('.sheet-export .sheet-export__file')`, 'dışa aktarma penceresi');
    await sleep(300);
    // The ifraz sheet's signature cell for @kullanici is left for later: export anyway.
    await page(ANYWAY);
    await sleep(200);
  };
  const bytesAt = (expr) => page(`const r = await fetch(${expr}); return b64(new Uint8Array(await r.arrayBuffer()));`);
  const direct = sha(pdf);

  await openWindow('Ctrl+P');
  const look = await page(`const d = ${DLG}; return { title: d.querySelector('.dialog__title')?.textContent ?? '', primary: [...d.querySelectorAll('.btn--primary')].map((x) => x.textContent.trim()), geo: d.querySelector('button[role=switch][aria-label="GeoPDF"]')?.getAttribute('aria-checked'), ways: [...d.querySelectorAll('.note')].map((x) => x.textContent).join(' ') };`);
  check('window: Ctrl+P on the sheet opens it as Yazdır, Yazdır its one primary button, GeoPDF on, the map as vectors', /^Yazdır:/.test(look.title) && /^Yazdır:/.test(look.title) && look.primary.length === 1 && look.primary[0] === 'Yazdır' && look.geo === 'true' && /vektör olarak gider/.test(look.ways), `${look.title}; birincil ${look.primary.join(', ')}; GeoPDF ${look.geo}`);
  await page(`${button('Yazdır')}.click(); return true;`);
  await until(`document.querySelector('iframe[src^="blob:"]')`, 'yazdırma çerçevesi');
  const printed = Buffer.from(await bytesAt(`document.querySelector('iframe[src^="blob:"]').src`), 'base64');
  await until(`!document.querySelector('.sheet-export')`, 'pencerenin kapanması');
  const fallback = await page(`return window.__pdf.opened;`);
  check('Yazdır: the print frame holds the same PDF, the window closes, no fallback tab', sha(printed) === direct && !fallback, `${printed.length} bayt${fallback ? '; yedek sekme açıldı' : ''}`);

  await openWindow('sheet.export.pdf');
  await page(`${button('Yeni sekmede aç')}.click(); return true;`);
  await until(`window.__pdf.opened?.href`, 'sekmenin adresi');
  const tab = await page(`return window.__pdf.opened;`);
  const opened = Buffer.from(await bytesAt(`window.__pdf.opened.href`), 'base64');
  check('Yeni sekmede aç: the tab opened at the click gets the same PDF', tab.url === '' && tab.target === '_blank' && /^blob:/.test(tab.href) && sha(opened) === direct, `${String(tab.href).slice(0, 40)}…`);

  await openWindow('sheet.export.pdf');
  await page(`${button('Kaydet')}.click(); return true;`);
  await until(`window.__pdf.saved`, 'kaydedilen dosya');
  const saved = await page(`return { name: window.__pdf.name, pdf: b64(window.__pdf.saved) };`);
  check('Kaydet: the picker gets the same PDF under the sheet’s export name', saved.name === `${made.name}.pdf` && sha(Buffer.from(saved.pdf, 'base64')) === direct, saved.name);

  await page(`window.__pdf.saved = null; return true;`);
  await openWindow('sheet.export.pdf');
  await page(`[...${DLG}.querySelectorAll('[aria-label="Paftalar"] [role=radio]')].find((x) => /^Hepsi/.test(x.textContent)).click(); return true;`);
  await sleep(400);
  await page(ANYWAY);
  await sleep(200);
  await page(`${button('Kaydet')}.click(); return true;`);
  await until(`window.__pdf.saved`, 'iki paftanın dosyası');
  const both = await page(`return { name: window.__pdf.name, pdf: b64(window.__pdf.saved), doc: k.doc.name.value };`);
  const bothFile = join(DIR, 'iki-pafta.pdf');
  writeFileSync(bothFile, Buffer.from(both.pdf, 'base64'));
  const pages = run('pdfinfo', ['-f', '1', '-l', '2', bothFile]);
  const sizes = [...pages.matchAll(/Page\s+\d+ size:\s+([\d.]+) x ([\d.]+) pts/g)].map((x) => `${x[1]} × ${x[2]}`);
  check('Hepsi: both sheets, a page each at its own paper, under the drawing’s name', /Pages:\s+2\b/.test(pages) && sizes[0] === '1190.55 × 841.89' && /^595\.2\d* × 841\.89$/.test(sizes[1] ?? '') && both.name === `${both.doc.replace(/\.[a-z0-9]+$/i, '')} paftaları.pdf`, `${both.name}; ${sizes.join(', ')}`);

  // ── 6. The fallback: a layer with no vector form here (a pattern fill) sends its map as a picture ───────
  await page(`
    const find = (nodes) => { for (const n of nodes) { if (n.name === 'Yapı') return n; const c = find(n.children); if (c) return c; } return null; };
    const yapi = find(k.doc.layers.tree);
    const dots = { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'circle', size: 0.8, fill: '#3366cc', stroke: null }] };
    k.doc.setLayerStyle(yapi.id, { renderer: { type: 'single', symbols: { fill: { type: 'fill', layers: [{ id: 'p', type: 'patternFill', marker: dots, spacingX: 3, spacingY: 3 }] } } } }, 'Desenli yapılar');
    window.__pdf.saved = null;
    return true;`);
  await openWindow('sheet.export.pdf');
  const way = await page(`return [...${DLG}.querySelectorAll('.note')].map((x) => x.textContent).join(' ');`);
  await page(`${button('Kaydet')}.click(); return true;`);
  await until(`window.__pdf.saved`, 'resimli haritanın dosyası');
  const pictured = await page(`const r = { pdf: b64(window.__pdf.saved) }; k.doc.undo(); return r;`);
  const picFile = join(DIR, 'ifraz-resimli-harita.pdf');
  writeFileSync(picFile, Buffer.from(pictured.pdf, 'base64'));
  const images = run('pdfimages', ['-list', picFile]).split('\n').slice(2).filter(Boolean).map((l) => l.trim().split(/\s+/));
  // page num type width height …: the map's content box at 300 dpi.
  const wantW = Math.round((c.width / 25_400) * 300);
  const wantH = Math.round((c.height / 25_400) * 300);
  const mapImage = images.find((x) => x[2] === 'image' && Math.abs(Number(x[3]) - wantW) <= 2 && Math.abs(Number(x[4]) - wantH) <= 2);
  check('fallback: the window names the layer and why; the map goes as a 300 dpi picture', /“Harita” 300 dpi resim olarak gider/.test(way) && /Yapı \(desen dolgusu\)/.test(way) && !!mapImage, `${way.trim().slice(0, 110)}; resim ${mapImage ? `${mapImage[3]} × ${mapImage[4]}` : `yok (${images.map((x) => `${x[3]}×${x[4]}`).join(', ')})`}`);
  // No paper under the map's picture: its alpha goes into the PDF as a soft mask (the desktop's picture is the same).
  const mapMask = mapImage && images.find((x) => x[2] === 'smask' && x[3] === mapImage[3] && x[4] === mapImage[4]);
  check('fallback: the pictured map has no paper under it (a soft mask, as the desktop’s)', !!mapMask, mapMask ? `smask ${mapMask[3]} × ${mapMask[4]}` : `smask yok (${images.map((x) => `${x[2]} ${x[3]}×${x[4]}`).join(', ')})`);
  const picGeo = JSON.parse(run('gdalinfo', ['-json', picFile]));
  check('fallback: the pictured map keeps its georeference', !!picGeo.metadata?.['']?.NEATLINE && /TUREF \/ TM36/.test(picGeo.coordinateSystem?.wkt ?? ''), picGeo.metadata?.['']?.NEATLINE?.slice(0, 60) ?? 'NEATLINE yok');
  run('pdftoppm', ['-r', '150', '-png', picFile, join(DIR, 'ifraz-resimli-harita')]);

  // ── 7. A system with no transverse Mercator (TUREF geographic): GeoPDF off, and the window says why ─────
  await page(`const { crsBySrid } = await import('/src/geo/crs.ts'); window.__crs = k.doc.crs.value; k.doc.crs.set(crsBySrid(5252)); return true;`);
  await page(`k.commands.execute('sheet.export.pdf'); return true;`);
  await until(`document.querySelector('.sheet-export .sheet-export__file')`, 'dışa aktarma penceresi');
  const geoOff = await page(`const sw = ${DLG}.querySelector('button[role=switch][aria-label="GeoPDF"]'); return { checked: sw.getAttribute('aria-checked'), disabled: sw.disabled, hint: sw.closest('.sheet-insp__row').nextElementSibling.textContent };`);
  check('GeoPDF: off and unavailable in a geographic system, with the reason', geoOff.checked === 'false' && geoOff.disabled && /enine Merkatör değil/.test(geoOff.hint), geoOff.hint);
  await page(`${button('Vazgeç')}.click(); k.doc.crs.set(window.__crs); return true;`);

  await page(`k.files.picker = window.__keep.picker; window.open = window.__keep.open; return true;`);
  // ── 9. A legend: its rows' symbols are the paint's PNGs, its labels the layers' names ─────────────────
  const legend = await page(`
    const e = await s.ensureEngine();
    const id = ${JSON.stringify(made.id)};
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const map = sh.items.find((i) => i.kind.type === 'map');
    const item = e.newItem(s.workspace(), s.capabilities(), { tool: 'legend', id: 'pdf-lejant', name: 'Lejant', frame: { left: 40_000, top: 190_000, width: 70_000, height: 75_000 }, link: map.id });
    s.apply([{ op: 'addItems', to: { kind: 'sheet', id }, items: [item] }], 'Lejant');
    let bytes;
    try {
      bytes = await pe.makePdf(k, s, { sheets: [id], dpi: 300, geo: true, layers: true });
    } finally {
      s.undo();
    }
    return { pdf: b64(bytes) };
  `);
  const legendFile = join(DIR, 'ifraz-lejantli.pdf');
  writeFileSync(legendFile, Buffer.from(legend.pdf, 'base64'));
  const legendText = run('pdftotext', ['-layout', legendFile, '-']);
  const symbols = run('pdfimages', ['-list', legendFile]).split('\n').slice(2).filter(Boolean).map((l) => l.trim().split(/\s+/)).filter((x) => x[2] === 'image');
  const rowNames = ['Parsel sınırı', 'Ada sınırı', 'Yapı', 'Eşyükselti'];
  check('legend: its rows’ symbols are pictures in the PDF, its labels the layers’ names', symbols.length >= rowNames.length && rowNames.every((n) => legendText.includes(n) && !text.includes(n)), `${symbols.length} resim (${symbols.slice(0, 3).map((x) => `${x[3]}×${x[4]}`).join(', ')}…)`);
  run('pdftoppm', ['-r', '150', '-png', legendFile, join(DIR, 'ifraz-lejantli')]);

  // ── 12. Outlines: a washer marker's hole stays empty, a filled circle keeps its fill, in the map's picture (the
  // screen, the gallery's pictures, PNG and SVG draw it) and in the PDF's vectors ────────────────────────────────
  for (const [hole, what] of [[0.5, 'washer'], [0, 'disc']]) {
    const probe = await page(`
      const find = (nodes) => { for (const n of nodes) { if (n.name === 'Kot noktaları') return n; const c = find(n.children); if (c) return c; } return null; };
      const layer = find(k.doc.layers.tree);
      k.doc.setLayerStyle(layer.id, { renderer: { type: 'single', symbols: { marker: { type: 'marker', layers: [{ id: 'w', type: 'shape', shape: 'circle', size: 4, unit: 'mm', fill: '#e01010', stroke: null, hole: ${hole} }] } } } }, 'Pul');
      await new Promise((r) => setTimeout(r, 400));
      const id = ${JSON.stringify(made.id)};
      const map = s.plan(s.book(), id).prims.find((p) => p.type === 'map');
      const v = map.view;
      const upm = 1e6 / v.scale; // paper µm per ground metre
      const west = v.center.x - map.clip.width / 2 / upm;
      const north = v.center.y + map.clip.height / 2 / upm;
      // A point of the layer well inside the frame.
      const pt = k.doc.byLayer(layer.id).filter((e) => e.kind === 'point').map((e) => e.p).find((p) => p.x > west + 20 && p.x < west + map.clip.width / upm - 20 && p.y < north - 20 && p.y > north - map.clip.height / upm + 20);
      // The map's picture at 150 dpi, as the screen and the exports draw it.
      const ppu = 150 / 25400;
      const pic = await s.paint.maps.exportPicture(map, ppu);
      const g = pic.getContext('2d');
      const x = (pt.x - west) * upm * ppu;
      const y = (north - pt.y) * upm * ppu;
      const at = (dx) => Array.from(g.getImageData(Math.round(x + dx), Math.round(y), 1, 1).data);
      const mm = 150 / 25.4;
      const pe2 = await import('/src/app/sheet/pdfExport.ts');
      const bytes = await pe2.makePdf(k, s, { sheets: [id], dpi: 150, geo: false, layers: false });
      k.doc.undo();
      return { centre: at(0), ring: at(-1.5 * mm), page: [map.clip.left / 1000 + (pt.x - west) * upm / 1000, map.clip.top / 1000 + (north - pt.y) * upm / 1000], pdf: b64(bytes) };
    `);
    const pf = join(DIR, `pul-${what}.pdf`);
    writeFileSync(pf, Buffer.from(probe.pdf, 'base64'));
    execFileSync('pdftoppm', ['-r', '150', '-gray', '-singlefile', pf, join(DIR, `pul-${what}`)]);
    const pgm = readFileSync(join(DIR, `pul-${what}.pgm`));
    const head = pgm.toString('latin1', 0, 40).match(/^P5\s+(\d+)\s+(\d+)\s+(\d+)\s/);
    const [w] = [Number(head[1])];
    const gray = (xMm, yMm) => pgm[head[0].length + Math.round((yMm * 150) / 25.4) * w + Math.round((xMm * 150) / 25.4)];
    const red = (c) => c[0] > 150 && c[1] < 100 && c[2] < 100;
    const white = (c) => c[0] > 200 && c[1] > 200 && c[2] > 200;
    const pdfCentre = gray(probe.page[0], probe.page[1]);
    const pdfRing = gray(probe.page[0] - 1.5, probe.page[1]);
    if (what === 'washer') check('outlines: a washer marker’s hole stays empty in the map’s picture and in the PDF', white(probe.centre) && red(probe.ring) && pdfCentre > 200 && pdfRing < 130, `resim ${probe.centre.slice(0, 3)} / ${probe.ring.slice(0, 3)}; PDF ${pdfCentre} / ${pdfRing}`);
    else check('outlines: a filled circle keeps its fill in the map’s picture and in the PDF', red(probe.centre) && red(probe.ring) && pdfCentre < 130 && pdfRing < 130, `resim ${probe.centre.slice(0, 3)} / ${probe.ring.slice(0, 3)}; PDF ${pdfCentre} / ${pdfRing}`);
  }

  // ── 13. An SVG picture: the PDF embeds the PNG the browser drew of it at the pixels the core asks (its frame at
  // the export's dpi; the core draws no SVG), in the picture's place, and the log says so ───────────────────────
  await page(`
    const e = await s.ensureEngine();
    const id = ${JSON.stringify(made.id)};
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100"><rect width="200" height="100" fill="#e01010"/><circle cx="100" cy="50" r="40" fill="#1040e0"/></svg>';
    const meta = await s.addPicture(new File([svg], 'e2e-isaret.svg', { type: 'image/svg+xml' }));
    const item = e.newItem(s.workspace(), s.capabilities(), { tool: 'picture', id: 'pdf-svg', name: 'SVG işaret', frame: { left: 40_000, top: 190_000, width: 60_000, height: 30_000 } });
    const has = s.book().book.assets.some((a) => a.sha256 === meta.sha256);
    s.apply([...(has ? [] : [{ op: 'addAssets', assets: [meta] }]), { op: 'addItems', to: { kind: 'sheet', id }, items: [{ ...item, kind: { ...item.kind, asset: meta.sha256 } }] }], 'SVG resim');
    return true;
  `);
  // The export window says first what the PDF does with it: the core's finding, with its pixels at the window's dpi.
  await openWindow('sheet.export.pdf');
  await until(`${DLG}.querySelector('[data-code="svg_as_picture"]')`, 'SVG resmin notu');
  const svgNote = await page(`return ${DLG}.querySelector('[data-code="svg_as_picture"]').closest('.note').textContent;`);
  await page(`${button('Vazgeç')}.click(); return true;`);
  const svgRun = await page(`
    const id = ${JSON.stringify(made.id)};
    const before = k.log.entries.value.length;
    let bytes;
    try {
      bytes = await pe.makePdf(k, s, { sheets: [id], dpi: 300, geo: false, layers: false });
    } finally {
      s.undo();
    }
    return { pdf: b64(bytes), said: k.log.entries.value.slice(before).map((x) => x.level + ': ' + x.text) };
  `);
  const svgFile = join(DIR, 'ifraz-svg-resim.pdf');
  writeFileSync(svgFile, Buffer.from(svgRun.pdf, 'base64'));
  const svgImages = run('pdfimages', ['-list', svgFile]).split('\n').slice(2).filter(Boolean).map((l) => l.trim().split(/\s+/));
  const svgImage = svgImages.find((x) => x[2] === 'image' && x[3] === '709' && x[4] === '355');
  execFileSync('pdftoppm', ['-r', '150', '-singlefile', svgFile, join(DIR, 'ifraz-svg-resim')]);
  const ppm = readFileSync(join(DIR, 'ifraz-svg-resim.ppm'));
  const ppmHead = ppm.toString('latin1', 0, 40).match(/^P6\s+(\d+)\s+(\d+)\s+(\d+)\s/);
  const rgb = (xMm, yMm) => {
    const at = ppmHead[0].length + (Math.round((yMm * 150) / 25.4) * Number(ppmHead[1]) + Math.round((xMm * 150) / 25.4)) * 3;
    return [ppm[at], ppm[at + 1], ppm[at + 2]];
  };
  // The frame is 40–100 mm across, 190–220 mm down: the circle in its middle, the rectangle at its left end.
  const svgCentre = rgb(70, 205);
  const svgLeft = rgb(43, 205);
  const svgSaid = svgRun.said.find((t) => t.startsWith('info: ') && /SVG resim PDF'e resim olarak gömülür \(709 × 355 piksel\)/.test(t));
  check('SVG picture: the export window says first that it goes as a picture, and of how many pixels', /SVG resim PDF'e resim olarak gömülür \(709 × 355 piksel\)/.test(svgNote), svgNote.trim());
  check(
    'SVG picture: embedded as the browser’s PNG at its frame’s 300 dpi pixels (never the empty box), drawn in its place, said in the log',
    !!svgImage && svgCentre[2] > 150 && svgCentre[0] < 100 && svgLeft[0] > 150 && svgLeft[2] < 100 && !!svgSaid,
    `${svgImage ? `${svgImage[3]} × ${svgImage[4]}` : `resim yok (${svgImages.map((x) => `${x[3]}×${x[4]}`).join(', ')})`}; orta ${svgCentre}, sol ${svgLeft}; ${svgSaid ?? svgRun.said.join(' | ')}`,
  );

  // ── 10. A turned map: the core's NEATLINE is the frame the labels are kept by (frameLabels.ts `insideFrame`) ──
  const turned = await page(`
    const fl = await import('/src/app/sheet/frameLabels.ts');
    const id = ${JSON.stringify(made.id)};
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const map = sh.items.find((i) => i.kind.type === 'map');
    s.apply([{ op: 'setItemProps', id: map.id, patch: { kind: { view: { ...map.kind.view, rotation: 30_000 } } } }], 'Haritayı döndür');
    let bytes;
    let prim;
    try {
      prim = s.plan(s.book(), id).prims.find((p) => p.type === 'map');
      bytes = await pe.makePdf(k, s, { sheets: [id], dpi: 300, geo: true, layers: true });
    } finally {
      s.undo();
    }
    window.__inside = fl.insideFrame(prim);
    return { pdf: b64(bytes), center: prim.view.center };
  `);
  const turnedFile = join(DIR, 'ifraz-donuk-harita.pdf');
  writeFileSync(turnedFile, Buffer.from(turned.pdf, 'base64'));
  const tGeo = JSON.parse(run('gdalinfo', ['-json', turnedFile]));
  const tNeat = (/POLYGON \(\((.+)\)\)/.exec(tGeo.metadata?.['']?.NEATLINE ?? '')?.[1] ?? '').split(',').filter(Boolean).map((p) => p.trim().split(/\s+/).map(Number)).slice(0, 4);
  // Each corner 1 cm towards the centre is inside the frame, 1 cm away from it outside.
  const nudged = tNeat.flatMap(([x, y]) => {
    const d = Math.hypot(x - turned.center.x, y - turned.center.y);
    const ux = (x - turned.center.x) / d;
    const uy = (y - turned.center.y) / d;
    return [[x - ux * 0.01, y - uy * 0.01, true], [x + ux * 0.01, y + uy * 0.01, false]];
  });
  const agree = await page(`return ${JSON.stringify(nudged)}.map(([x, y, want]) => window.__inside(x, y) === want);`);
  run('pdftoppm', ['-r', '150', '-png', turnedFile, join(DIR, 'ifraz-donuk-harita')]);
  check('turned map: the core’s NEATLINE corners are the frame the labels are kept by (30°)', tNeat.length === 4 && agree.every(Boolean), tNeat.map((p) => p.map((x) => x.toFixed(2)).join(' ')).join(' | '));

  // ── 11. Yazdır ve pafta (`file.print`, the app menu's, Ctrl+P's global command) prints the sheet in front ───
  const filePrint = await page(`
    const cmd = k.commands.get('file.print');
    const inSheet = cmd.pending;
    k.commands.execute('file.print');
    return { inSheet };
  `);
  await until(`document.querySelector('.sheet-export .sheet-export__file')`, 'Yazdır ve pafta’nın penceresi');
  const fileTitle = await page(`return ${DLG}.querySelector('.dialog__title')?.textContent ?? '';`);
  await page(`${button('Vazgeç')}.click(); return true;`);
  const onModel = await page(`
    s.openSheet(null);
    await new Promise((r) => setTimeout(r, 300));
    const pending = k.commands.get('file.print').pending;
    s.openSheet(${JSON.stringify(made.id)});
    await new Promise((r) => setTimeout(r, 300));
    return { pending, back: k.commands.get('file.print').pending };
  `);
  check('file.print: with a sheet in front it opens the sheet’s Yazdır and is not “Geliştirme aşamasında”; with Model in front it is', filePrint.inSheet === false && /^Yazdır:/.test(fileTitle) && onModel.pending === true && onModel.back === false, `${fileTitle}; Model'de bekleyen: ${onModel.pending}`);

  // ── 8. The desktop's PDF of the same sheet (with --masaustu): the core's parts the same, the maps' differences said ─
  if (DESKTOP) {
    const dInfo = run('pdfinfo', [DESKTOP]);
    const dGeo = JSON.parse(run('gdalinfo', ['-json', DESKTOP]));
    const dLayers = [...run('gdalinfo', ['-mdd', 'LAYERS', DESKTOP]).matchAll(/LAYER_\d+_NAME=(.+)/g)].map((x) => x[1].trim());
    const dFaces = run('pdffonts', [DESKTOP]).split('\n').slice(2).filter(Boolean).map((l) => l.split(/\s+/)[0].replace(/^[A-Z]{6}\+/, '')).sort();
    const faces = fonts.map((l) => l.split(/\s+/)[0].replace(/^[A-Z]{6}\+/, '')).sort();
    check('desktop: the same page, faces, NEATLINE and PDF layers', /Page size:\s+1190\.55 x 841\.89/.test(dInfo) && faces.join() === dFaces.join() && dGeo.metadata?.['']?.NEATLINE === gi.metadata?.['']?.NEATLINE && dLayers.join() === named.join(), `${dFaces.join(', ')}; ${dLayers.length} katman`);
    // Inside the map frame's content box: the map; outside it, what the core draws (and the sheet's values).
    const inMap = (w) => {
      const x = (w.box[0] + w.box[2]) / 2;
      const y = (w.box[1] + w.box[3]) / 2;
      return x >= c.left / 1000 && x <= (c.left + c.width) / 1000 && y >= c.top / 1000 && y <= (c.top + c.height) / 1000;
    };
    const ours = wordsOf(file);
    const theirs = wordsOf(DESKTOP);
    const outside = apart(ours.filter((w) => !inMap(w)), theirs.filter((w) => !inMap(w)));
    check('desktop: every text the core draws outside the map is where ours is', outside.common > 50 && outside.worst[1] < 0.01, `${outside.common} sözcük, en büyük fark ${outside.worst[1].toFixed(3)} mm; yalnız bizde: ${minus(ours.filter((w) => !inMap(w)), theirs.filter((w) => !inMap(w))).join(' ')}; yalnız masaüstünde: ${minus(theirs.filter((w) => !inMap(w)), ours.filter((w) => !inMap(w))).join(' ')}`);
    const inside = apart(ours.filter(inMap), theirs.filter(inMap));
    console.log(`  harita: ${ours.filter(inMap).length} / ${theirs.filter(inMap).length} sözcük; ${inside.common} ortak, en büyük fark ${inside.worst[1].toFixed(2)} mm (“${inside.worst[0]}”), ortanca ${inside.median.toFixed(2)} mm`);
    console.log(`  yalnız bizim haritada: ${minus(ours.filter(inMap), theirs.filter(inMap)).join(' ') || '—'}; yalnız masaüstününkinde: ${minus(theirs.filter(inMap), ours.filter(inMap)).join(' ') || '—'}`);
    run('pdftoppm', ['-r', '150', '-png', DESKTOP, join(DIR, 'masaustu')]);
  }

  const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
  check('the page’s console: no error', !errors.length, errors.slice(0, 3).join(' | ').slice(0, 300));
} catch (e) {
  failures.push(String(e?.message ?? e));
  console.error(e);
} finally {
  b.close();
  await server.close();
}
console.log(failures.length ? `\n${failures.length} denetim düştü.` : '\nTüm denetimler geçti.');
process.exit(failures.length ? 1 : 0);
