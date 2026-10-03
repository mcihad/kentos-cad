// Magnetic north on the web (docs/sheet/tasks-web.md E; design §8a), in the app on the demo drawing's ifraz
// sheet, its north arrow made magnetic and its date given (the sheet's “tarih”: 2026-10-03):
//
// - the inspector says the declination the paper writes with its source (“6°19' D · WMM2025 · 2026-10”, the
//   engine's northInfo) and where the date comes from;
// - “Sapmayı elle gir” starts the hand value from the model's (to the minute), and the paper writes “elle”;
// - the north diagram (“Kuzey çizelgesi”) writes GK, CK, MK and their angles, in the PDF too (pdftotext);
// - a sheet date the model does not cover: the preflight's warning; its fixes open the sheet's values and type
//   the declination by hand (the warning goes);
// - a system the core cannot invert (TUREF geographic): the preflight's error; its fixes open the project's
//   coordinate system and type the declination by hand (the error goes).
//
// A failed check fails the run.
//
//   node apps/web/scripts/e2e/sheet-north.mjs
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const WEB = fileURLToPath(new URL('../..', import.meta.url));
const DIR = join(OUT, 'shots', 'sheet-north');
mkdirSync(DIR, { recursive: true });
const failures = [];
const check = (name, ok, detail = '') => {
  console.log(`${ok ? '✓' : '✗'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures.push(name);
};

const server = await createServer({ root: WEB, configFile: join(WEB, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const b = await launch('about:blank', { width: 1440, height: 900 });
const S = `const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts');
  const id = window.__sheet; const book = () => s.book().book; const sheet = () => book().sheets.find((x) => x.id === id);
  const arrow = () => sheet()?.items.find((i) => i.kind.type === 'northArrow');
  const texts = () => s.plan(s.book(), id).prims.filter((p) => p.type === 'text' && p.item === arrow().id).map((p) => p.text);`;
const page = (body) => b.eval(`(async () => { ${S} ${body} })()`);
const until = async (expr, what, ms = 15000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (await page(`return !!(${expr});`).catch(() => false)) return;
    await sleep(100);
  }
  throw new Error(`beklenen olmadı: ${what}`);
};
const finding = (code) => `document.querySelector('.sheet-finding[data-code="${code}"]')`;
const press = (code, label) => page(`[...(${finding(code)}?.querySelectorAll('button') ?? [])].find((x) => x.textContent.trim() === ${JSON.stringify(label)})?.click(); return true;`);
const dialogTitled = (re) => `[...document.querySelectorAll('.dialog')].some((d) => ${re}.test(d.querySelector('.dialog__title')?.textContent ?? ''))`;
const closeDialogs = async () => {
  for (let i = 0; i < 3; i++) {
    await b.key('Escape');
    await sleep(200);
  }
};

const VALUES = { il: 'Sivas', ilce: 'Suşehri', mahalle: 'Kızılırmak', ada: '1245', pafta_no: 'P-12' };
try {
  await b.send('Page.navigate', { url: `${server.resolvedUrls.local[0]}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await sleep(1500);
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  await b.eval(`(async () => {
    const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts');
    k.doc.settings.workspace.set('cad');
    const e = await s.ensureEngine();
    const values = ${JSON.stringify(VALUES)};
    const id = await ta.sheetFromTemplate(s, e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi'), null, undefined, Object.entries(values).map(([name, value]) => ({ name, value })));
    window.__sheet = id;
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const arrow = sh.items.find((i) => i.kind.type === 'northArrow');
    s.apply([
      { op: 'setItemProps', id: arrow.id, patch: { kind: { north: 'magnetic' } } },
      { op: 'saveVariables', sheet: id, variables: [...sh.variables, { name: 'tarih', label: 'Tarih', kind: 'date', value: '2026-10-03' }] },
    ], 'Manyetik kuzey');
    s.openSheet(id);
    return true;
  })()`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) .sheet-stage__paper')`, 'pafta');
  await sleep(800);

  // ── The inspector: the declination, its source and its date ──────────────────────────────────────────
  await page(`s.state.select([arrow().id]); return true;`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) .sheet-north__value')`, 'kuzey okunun denetçisi');
  const shown = await page(`const v = document.querySelector('.sheet-north__value').textContent; const date = document.querySelector('.sheet-north__date')?.closest('.sheet-field')?.textContent ?? ''; return { v, date };`);
  check('inspector: the declination the paper writes, with its source (the engine’s northInfo)', shown.v === "6°19' D · WMM2025 · 2026-10", shown.v);
  check('inspector: the date it is for and where it comes from', /2026-10-03/.test(shown.date) && /paftanın “tarih” değişkeni/.test(shown.date), shown.date.replace(/\s+/g, ' ').slice(0, 90));
  const north = `document.querySelector('.sheet-ws:not([hidden]) [data-section="kind:northArrow"]')?.scrollIntoView({ block: 'start' })`;
  await b.eval(north);
  await sleep(300);
  await b.shot('denetci-model', undefined, DIR);

  // ── By hand: from the model's value, written “elle” ────────────────────────────────────────────────────
  await page(`document.querySelector('.sheet-ws:not([hidden]) button[role=switch][aria-label="Sapmayı elle gir"]').click(); return true;`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) [data-key="north.declination"]')`, 'elle sapma alanı');
  const hand = await page(`return { kind: arrow().kind, paper: texts(), value: document.querySelector('.sheet-north__value').textContent };`);
  check('“Sapmayı elle gir” starts from the model’s value and the paper writes “elle”', hand.kind.declinationHand === true && hand.kind.declination === 6317 && hand.paper.join(' ').includes("Manyetik sapma 6°19' D (elle)"), `${hand.kind.declination} mdeg; ${hand.value}`);
  await b.eval(north);
  await sleep(300);
  await b.shot('denetci-elle', undefined, DIR);
  await page(`s.undo(); return true;`);
  await until(`!arrow().kind.declinationHand`, 'geri alma');

  // ── The north diagram, on the paper and in the PDF ──────────────────────────────────────────────────────
  await page(`s.apply([{ op: 'setItemProps', id: arrow().id, patch: { kind: { style: 'diagram' } } }], 'Kuzey çizelgesi'); return true;`);
  const diagram = await page(`
    const pe = await import('/src/app/sheet/pdfExport.ts');
    const bytes = await pe.makePdf(k, s, { sheets: [id], dpi: 300, geo: true, layers: true });
    let bin = '';
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return { paper: texts(), pdf: btoa(bin) };`);
  const pdfFile = join(DIR, 'kuzey-cizelgesi.pdf');
  writeFileSync(pdfFile, Buffer.from(diagram.pdf, 'base64'));
  const pdfText = execFileSync('pdftotext', [pdfFile, '-'], { encoding: 'utf8' }).replace(/\s+/g, ' ');
  // The core writes minutes and seconds with ASCII marks, and breaks a line before its source where it must.
  const want = ["GK–CK yakınsama −0°06'", "CK–MK manyetik sapma 6°19' D (WMM2025, 2026-10)", "GK–MK açısı 6°25' D"];
  const paper = diagram.paper.join(' ').replace(/\s+/g, ' ');
  check('the north diagram writes GK, CK, MK and their angles', ['GK', 'CK', 'MK'].every((t) => diagram.paper.includes(t)) && want.every((t) => paper.includes(t)), diagram.paper.join(' | '));
  check('… and so does its PDF', want.every((t) => pdfText.includes(t)), want.filter((t) => !pdfText.includes(t)).join(', ') || 'üç satır da var');
  execFileSync('pdftoppm', ['-r', '150', '-png', '-x', '1700', '-y', '250', '-W', '700', '-H', '480', pdfFile, join(DIR, 'kuzey-cizelgesi')]);

  // ── A date the model does not cover: the warning and its fixes ───────────────────────────────────────────
  await page(`s.apply([{ op: 'saveVariables', sheet: id, variables: sheet().variables.map((v) => (v.name === 'tarih' ? { ...v, value: '2031-03-01' } : v)) }], 'Tarih 2031'); return true;`);
  await b.eval(`(() => { document.querySelector('.sheet-insp__tabs [data-tab="preflight"]').click(); return true; })()`);
  await until(finding('magnetic_out_of_model'), 'modelin dışı uyarısı');
  const warn = await page(`const f = ${finding('magnetic_out_of_model')}; return { cls: f.className, buttons: [...f.querySelectorAll('button')].map((x) => x.textContent.trim()) };`);
  check('out of the model: a warning with its two fixes', /sheet-finding--warning/.test(warn.cls) && warn.buttons.includes('Değişkenleri aç') && warn.buttons.includes('Sapmayı elle gir'), warn.buttons.join(', '));
  await b.shot('on-denetim-model-disi', undefined, DIR);
  await press('magnetic_out_of_model', 'Değişkenleri aç');
  await until(dialogTitled(/^Değişkenler/), 'değişkenler penceresi');
  check('… “Değişkenleri aç” opens the sheet’s values', true);
  await closeDialogs();
  await press('magnetic_out_of_model', 'Sapmayı elle gir');
  await until(`!${finding('magnetic_out_of_model')}`, 'uyarının gitmesi');
  const after = await page(`return arrow().kind.declinationHand === true;`);
  check('… “Sapmayı elle gir” types it by hand and the warning goes', after);

  // ── A system the core cannot invert: the error and its fixes ─────────────────────────────────────────────
  await page(`
    const { crsBySrid } = await import('/src/geo/crs.ts');
    window.__crs = k.doc.crs.value;
    s.apply([
      { op: 'setItemProps', id: arrow().id, patch: { kind: { declinationHand: false } } },
      { op: 'saveVariables', sheet: id, variables: sheet().variables.map((v) => (v.name === 'tarih' ? { ...v, value: '2026-10-03' } : v)) },
    ], 'Modelden');
    k.doc.crs.set(crsBySrid(5252));
    return true;`);
  await until(finding('magnetic_no_place'), 'yer yok hatası');
  const err = await page(`const f = ${finding('magnetic_no_place')}; return { cls: f.className, buttons: [...f.querySelectorAll('button')].map((x) => x.textContent.trim()) };`);
  check('no place: an error with its two fixes', /sheet-finding--error/.test(err.cls) && err.buttons.includes('Koordinat sistemi seç') && err.buttons.includes('Sapmayı elle gir'), err.buttons.join(', '));
  await b.shot('on-denetim-yer-yok', undefined, DIR);
  await press('magnetic_no_place', 'Koordinat sistemi seç');
  await until(`${dialogTitled(/Proje ayarları|Koordinat sistemi/)}`, 'projenin koordinat sistemi');
  check('… “Koordinat sistemi seç” opens the project’s coordinate system', true);
  await closeDialogs();
  await press('magnetic_no_place', 'Sapmayı elle gir');
  await until(`!${finding('magnetic_no_place')}`, 'hatanın gitmesi');
  check('… “Sapmayı elle gir” types it by hand and the error goes', true);
  await page(`k.doc.crs.set(window.__crs); return true;`);

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
