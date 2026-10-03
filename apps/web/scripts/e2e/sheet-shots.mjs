// Pictures of the sheet layouts (docs/sheet/tasks-web.md A10, B9; not a test): named scenes at 1440×900 and
// 1100×650, in the light and the dark theme, into scripts/e2e/out/shots/sheet/<scene>-<theme>-<width>.png, on the
// demo drawing. Uses cdp.mjs as shots.mjs does and leaves shots.mjs alone. Every sheet is the engine's: made from
// its system templates on the drawing through the app's own sheet service (app/sheet/install.ts `sheetsOf`), the
// values and the table's columns given as the Değişkenler window and the inspector give them, a template of the
// user's own saved as “Şablon olarak kaydet” saves it. Some scenes check what they show as they go (a failed check
// fails the run): the keys, Geri al from the quick access bar on the sheet's own history.
//
//   node apps/web/scripts/e2e/sheet-shots.mjs [--only a,b] [--sizes 1440x900,1100x650] [--themes light,dark]
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const args = process.argv.slice(2);
const opt = (name) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1].split(',') : null);
const only = opt('only');
const sizes = (opt('sizes') ?? ['1440x900', '1100x650']).map((s) => s.split('x').map(Number));
const themes = opt('themes') ?? ['light', 'dark'];

const DIR = join(OUT, 'shots', 'sheet');
mkdirSync(DIR, { recursive: true });

/** The app, its sheet service and the template actions, in the page. */
const S = `const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts');`;
const run = (body) => `(async () => { ${S} ${body} })()`;

/** The parcel table's columns for the demo drawing's parcels (as the table's section in the inspector sets them). */
const COLUMNS = [
  { heading: 'Parsel', value: 'Parsel', width: 13000, align: 'center' },
  { heading: 'Yüzölçümü (m²)', value: '$alan', width: 0, align: 'right', decimals: 2 },
  { heading: 'Tapu alanı (m²)', value: '[Tapu alanı (m²)]', width: 0, align: 'right', decimals: 2 },
  { heading: 'Nitelik', value: 'Nitelik', width: 0, align: 'left' },
];
/** The ifraz sheet's values (kontrol_eden and onaylayan are left for the preflight to ask). */
const VALUES = { il: 'Sivas', ilce: 'Suşehri', mahalle: 'Kızılırmak', ada: '1245', pafta_no: 'P-12' };

/**
 * The project's sheets, made once per page: the ifraz sheet from the system template (CAD), its values and its
 * table's columns; an aplikasyon sketch; a template of the user's own on this device. Then the asked sheet in front.
 */
const SETUP = (ws = 'cad', open = 'sys:ifraz-paftasi') =>
  run(`
  k.doc.settings.workspace.set(${JSON.stringify(ws)});
  const e = await s.ensureEngine();
  const tpl = (id) => e.systemTemplates().find((x) => x.meta.id === id);
  const has = (id) => s.book().book.sheets.find((x) => x.origin?.templateId === id);
  if (!has('sys:ifraz-paftasi')) {
    const id = await ta.sheetFromTemplate(s, tpl('sys:ifraz-paftasi'), null);
    const sh = s.book().book.sheets.find((x) => x.id === id);
    const values = ${JSON.stringify(VALUES)};
    const table = sh.items.find((i) => i.kind.type === 'table');
    s.apply([
      { op: 'saveVariables', sheet: id, variables: sh.variables.map((v) => ({ ...v, value: values[v.name] ?? v.value })) },
      { op: 'setItemProps', id: table.id, patch: { kind: { columns: ${JSON.stringify(COLUMNS)}, source: { sort: [{ expression: 'to_int(Parsel)', descending: false }] } } } },
    ], 'Değişkenler ve parsel tablosu');
  }
  if (!has('sys:aplikasyon-krokisi')) await ta.sheetFromTemplate(s, tpl('sys:aplikasyon-krokisi'), null);
  await s.device.refresh();
  if (!s.device.cards.value.length)
    await ta.saveAsTemplate(k, s, has('sys:ifraz-paftasi').id, { name: 'Belediye ifraz paftası', description: 'Belediyenin ifraz paftası: Kızılırmak mahallesi düzeni, parsel tablosu ve imzalar.', category: 'kadastro', tags: ['ifraz', 'belediye'], papers: [{ paper: 'a3', orientation: 'landscape' }], workspaces: ['cad'], projectTypes: ['subdivision'] }, null);
  s.openSheet(has(${JSON.stringify(open)}).id);
  s.state.clearSelection();
  s.state.tool.set({ kind: 'select' });
  return true;`);
const BACK = run(`s.state.openSheet(null); s.state.clearSelection(); s.state.tool.set({ kind: 'select' }); k.selection.clear(); k.doc.settings.workspace.set('gis'); return true;`);
/** Chooses items of the sheet in front by their names. */
const CHOOSE = (...names) => run(`const sh = s.state.sheet; s.state.select(sh.items.filter((i) => ${JSON.stringify(names)}.includes(i.name)).map((i) => i.id)); return s.state.selection.value.size;`);
/** The window position (CSS px) of an item's frame point (fx, fy of its width and height) on the paper. */
const AT = (name, fx = 0.5, fy = 0.5) =>
  run(`const st = m.workspaceOf(k).stage; const it = s.state.sheet.items.find((i) => i.name === ${JSON.stringify(name)}); const f = it.frame; const p = st.clientPoint({ x: f.left + f.width * ${fx}, y: f.top + f.height * ${fy} }); return [Math.round(p.x), Math.round(p.y)];`);
/** The book's digest: a scene that changes the book takes it back (Geri al) until this is so again. */
const DIGEST = run(`return s.engine().bookDigest(s.book());`);
const UNDO_TO = (digest) => run(`for (let i = 0; i < 20 && s.engine().bookDigest(s.book()) !== ${JSON.stringify(digest)}; i++) s.undo(); return s.engine().bookDigest(s.book()) === ${JSON.stringify(digest)};`);

/** Waits until the workspace is on the page and drawn (the map's picture and the typefaces too). */
const workspaceReady = async (ui) => {
  await ui.waitFor(`document.querySelector('.sheet-ws:not([hidden]) .sheet-stage__paper')`);
  await ui.sleep(900);
};
const opened = async (ui, ws, open) => {
  await ui.eval(SETUP(ws, open));
  await workspaceReady(ui);
  // Each scene starts from the whole page (a sheet keeps its zoom while the window is open).
  await ui.eval(`window.kentos.commands.execute('sheet.zoomPage')`);
  await ui.sleep(300);
};
const gallery = async (ui) => {
  await ui.eval(`window.kentos.commands.execute('sheet.fromTemplate')`);
  await ui.waitFor(`document.querySelector('.dialog--tgal .tcard canvas')`, 15000);
  await ui.sleep(800);
};

const SCENES = [
  // ── Part B ─────────────────────────────────────────────────────────
  // (1) The ifraz sheet from the system template on the demo drawing: the map is the drawing, the table its parcels.
  { id: 'b1-ifraz-sistem-sablonu', open: async (ui) => (await opened(ui), await ui.move(2, 2)) },
  // (2) Dragging the north arrow: the engine's smart guides, distances and equal-spacing marks while the button is down.
  {
    id: 'b2-surukle-kilavuzlar',
    open: async (ui) => {
      await opened(ui);
      ui.state.digest = await ui.eval(DIGEST);
      // Closer: the strip beside the map, its title block, arrow, scale and the parcel table's top.
      await ui.eval(CHOOSE('Başlık ve yer', 'Kuzey oku', 'Ölçek çubuğu', 'Ayıraç'));
      await ui.eval(`window.kentos.commands.execute('sheet.zoomSelection')`);
      await ui.sleep(1200);
      const p = await ui.eval(AT('Kuzey oku'));
      await ui.click(p);
      await ui.dragHold(p, [-38, 22]);
      await ui.sleep(300);
    },
    close: async (ui) => {
      await ui.release();
      await ui.eval(UNDO_TO(ui.state.digest));
      await ui.escapeAll(2);
      await ui.eval(BACK);
    },
  },
  // (3) Several items chosen: the inspector shares their values (“—” where they differ) and the constraint editor.
  {
    id: 'b3-coklu-secim-kisit',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(CHOOSE('Başlık ve yer', 'Parsel tablosu', 'İmzalar'));
      await ui.sleep(300);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // (4) The gallery: the system's templates drawn small from their own plans on this drawing, then the user's own.
  {
    id: 'b4-galeri-sistem',
    open: async (ui) => {
      await opened(ui);
      await gallery(ui);
      await ui.clickSel('.tcard[data-id="sys:ifraz-paftasi"]');
      await ui.move(2, 2);
      await ui.sleep(500);
    },
  },
  {
    id: 'b4-galeri-benim',
    open: async (ui) => {
      await opened(ui);
      await gallery(ui);
      await ui.clickSel('.tgal__src[data-section="mine"]');
      await ui.sleep(500);
      await ui.clickSel('.tcard');
      await ui.move(2, 2);
      await ui.sleep(500);
    },
  },
  // (5) The preflight's findings on the inspector's Ön denetim tab, each with its fixes.
  {
    id: 'b5-on-denetim',
    open: async (ui) => {
      await opened(ui);
      await ui.clickSel('.sheet-insp__tabs [data-tab="preflight"]');
      await ui.sleep(300);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.clickSel('.sheet-insp__tabs [data-tab="item"]'), await ui.eval(BACK)),
  },
  // (6) Dışa aktar: PNG at 300 dpi with the preflight at that dpi; with an error it waits for “Yine de aktar”.
  {
    id: 'b6-disa-aktar',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(`window.kentos.commands.execute('sheet.export.png')`);
      await ui.waitFor(`document.querySelector('.sheet-export')`);
      await ui.sleep(400);
      await ui.move(2, 2);
    },
  },
  // (D) PDF: the sheets going, GeoPDF and layers, how each map goes (vectors here), the file's name; Yazdır.
  {
    id: 'd-pdf-disa-aktar',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(`window.kentos.commands.execute('sheet.print')`);
      await ui.waitFor(`document.querySelector('.sheet-export .sheet-export__file')`);
      // The preflight's errors are the signature cells left for later: export anyway, then the PDF's options in view.
      await ui.eval(`(() => { const sw = [...document.querySelectorAll('.sheet-export .sheet-insp__row')].find((r) => r.textContent.includes('yine de aktar'))?.querySelector('[role=switch]'); sw?.click(); return !!sw; })()`);
      await ui.sleep(300);
      await ui.eval(`(() => { document.querySelector('.sheet-export [aria-label="Paftalar"]')?.scrollIntoView({ block: 'start' }); return true; })()`);
      await ui.sleep(400);
      await ui.move(2, 2);
    },
  },
  // (D) PDF, the fallback: both sheets chosen, and a layer with no vector form here (Yapı given a pattern fill)
  // sends its map as a picture at the fallback resolution; the window says which layer and why.
  {
    id: 'd-pdf-resim-yedegi',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(`(() => {
        const k = window.kentos;
        const find = (nodes) => { for (const n of nodes) { if (n.name === 'Yapı') return n; const c = find(n.children); if (c) return c; } return null; };
        const dots = { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'circle', size: 0.8, fill: '#3366cc', stroke: null }] };
        k.doc.setLayerStyle(find(k.doc.layers.tree).id, { renderer: { type: 'single', symbols: { fill: { type: 'fill', layers: [{ id: 'p', type: 'patternFill', marker: dots, spacingX: 3, spacingY: 3 }] } } } }, 'Desenli yapılar');
        return true;
      })()`);
      await ui.eval(`window.kentos.commands.execute('sheet.export.pdf')`);
      await ui.waitFor(`document.querySelector('.sheet-export .sheet-export__file')`);
      await ui.eval(`(() => { [...document.querySelectorAll('.sheet-export [aria-label="Paftalar"] [role=radio]')].find((x) => /^Seçtiklerim/.test(x.textContent))?.click(); return true; })()`);
      await ui.sleep(200);
      await ui.eval(`(() => { document.querySelectorAll('.sheet-export__sheets [role=switch][aria-checked="false"]').forEach((x) => x.click()); return true; })()`);
      await ui.sleep(200);
      await ui.eval(`(() => { document.querySelector('.sheet-export button[role=switch][aria-label="Hatalar varken yine de aktar"][aria-checked="false"]')?.click(); return true; })()`);
      await ui.sleep(300);
      await ui.eval(`(() => { document.querySelector('.sheet-export [aria-label="Paftalar"]')?.scrollIntoView({ block: 'start' }); return true; })()`);
      await ui.sleep(400);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(`(window.kentos.doc.undo(), true)`), await ui.eval(BACK), await ui.sleep(150)),
  },
  // (E) Magnetic north: the ifraz sheet's arrow as the north diagram (grid, true and magnetic north with their
  // angles), chosen: the inspector says the declination the paper writes and its source, and the date it is for.
  {
    id: 'e-kuzey-cizelgesi',
    open: async (ui) => {
      await opened(ui);
      ui.state.digest = await ui.eval(DIGEST);
      await ui.eval(
        run(`const sh = s.book().book.sheets.find((x) => x.id === s.state.open.value);
        const arrow = sh.items.find((i) => i.kind.type === 'northArrow');
        s.apply([{ op: 'setItemProps', id: arrow.id, patch: { kind: { north: 'magnetic', style: 'diagram' } } }], 'Kuzey çizelgesi');
        return true;`),
      );
      await ui.eval(CHOOSE('Kuzey oku', 'Başlık ve yer', 'Ölçek çubuğu'));
      await ui.eval(`window.kentos.commands.execute('sheet.zoomSelection')`);
      await ui.eval(CHOOSE('Kuzey oku'));
      await ui.sleep(1200);
      await ui.eval(`document.querySelector('.sheet-ws:not([hidden]) [data-section="kind:northArrow"]')?.scrollIntoView({ block: 'start' })`);
      await ui.sleep(300);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.eval(UNDO_TO(ui.state.digest)), await ui.escapeAll(2), await ui.eval(BACK)),
  },
  // (E) A sheet date the magnetic model does not cover: the preflight's warning, with its two fixes.
  {
    id: 'e-manyetik-on-denetim',
    open: async (ui) => {
      await opened(ui);
      ui.state.digest = await ui.eval(DIGEST);
      await ui.eval(
        run(`const sh = s.book().book.sheets.find((x) => x.id === s.state.open.value);
        const arrow = sh.items.find((i) => i.kind.type === 'northArrow');
        s.apply([
          { op: 'setItemProps', id: arrow.id, patch: { kind: { north: 'magnetic' } } },
          { op: 'saveVariables', sheet: sh.id, variables: [...sh.variables.filter((v) => v.name !== 'tarih'), { name: 'tarih', label: 'Tarih', kind: 'date', value: '2031-03-01' }] },
        ], 'Manyetik kuzey, 2031');
        return true;`),
      );
      await ui.clickSel('.sheet-insp__tabs [data-tab="preflight"]');
      await ui.sleep(400);
      await ui.eval(`document.querySelector('.sheet-finding[data-code="magnetic_out_of_model"]')?.scrollIntoView({ block: 'center' })`);
      await ui.sleep(300);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.clickSel('.sheet-insp__tabs [data-tab="item"]'), await ui.eval(UNDO_TO(ui.state.digest)), await ui.eval(BACK)),
  },
  // ── Part A's scenes, on the engine's sheets ────────────────────────
  { id: 'model-sekmeler', open: async (ui) => (await opened(ui), await ui.eval(BACK), await ui.sleep(300), await ui.move(2, 2)) },
  { id: 'model-arti-menu', open: async (ui) => (await opened(ui), await ui.eval(BACK), await ui.clickSel('.sheet-tabs__add'), await ui.sleep(300)) },
  {
    id: 'harita-secili',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(CHOOSE('Harita'));
      await ui.sleep(300);
      await ui.eval(`document.querySelector('[data-section="kind:map"]')?.scrollIntoView({ block: 'start' })`);
      await ui.hoverSel('.sheet-stage__paper', 0.35, 0.45);
    },
  },
  {
    // The constraint editor on the title block: its pins say what is kept; a pin's tooltip says how.
    id: 'kisit-ipucu',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(CHOOSE('Başlık ve yer'));
      await ui.sleep(300);
      await ui.eval(`document.querySelector('[data-section="anchors"]')?.scrollIntoView({ block: 'start' })`);
      await ui.sleep(200);
      await ui.hoverSel('.cedit [data-pin="h-end"]');
      await ui.sleep(500);
    },
  },
  {
    id: 'sayfa',
    open: async (ui) => {
      await opened(ui);
      await ui.clickSel('.sheet-insp__tabs [data-tab="page"]');
      await ui.sleep(300);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.clickSel('.sheet-insp__tabs [data-tab="item"]'), await ui.eval(BACK)),
  },
  {
    // Another mode's item (design §11a): in CAD the parcel table is CBS's attribute table, kept and edited, its note the engine's.
    id: 'baska-kip',
    open: async (ui) => {
      await opened(ui, 'cad');
      await ui.eval(CHOOSE('Parsel tablosu'));
      await ui.sleep(300);
      await ui.move(2, 2);
    },
  },
  { id: 'sekme-menusu', open: async (ui) => (await opened(ui), await ui.contextSel('.sheet-tab[aria-selected="true"]')) },
  {
    id: 'serit-ipucu',
    open: async (ui) => {
      await opened(ui);
      await ui.hoverSel('[data-command="sheet.add.map"]');
      await ui.sleep(700);
    },
  },
  {
    id: 'gercek-boyut',
    open: async (ui) => {
      // A parcel chosen on the drawing: the sketch's coordinate list gives its corners and its area.
      await ui.eval(`(() => { const k = window.kentos; const c = k.view.camera.center; const p = k.doc.byLayer('parsel').map((e) => ({ e, d: Math.hypot(e.pts[0].x - c.x, e.pts[0].y - c.y) })).sort((a, b) => a.d - b.d)[0]?.e; if (p) k.selection.set([p.id]); return !!p; })()`);
      await opened(ui, 'cad', 'sys:aplikasyon-krokisi');
      await ui.eval(`window.kentos.commands.execute('sheet.zoomReal')`);
      await ui.sleep(1200);
      await ui.hoverSel('.sheet-stage__paper', 0.5, 0.55);
    },
  },
  {
    // A screen of twice the pixels: the canvases follow the pixel ratio and every hairline stays one device pixel.
    id: 'hidpi',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(CHOOSE('Kuzey oku'));
      await ui.pixelRatio(2);
      await ui.sleep(1500);
      await ui.hoverSel('.sheet-stage__paper', 0.62, 0.3);
    },
    clip: async (ui) => ui.eval(`(() => { const r = document.querySelector('.sheet-stage').getBoundingClientRect(); return { x: r.left + r.width * 0.45, y: r.top, width: Math.min(560, r.width * 0.55), height: Math.min(360, r.height) }; })()`),
    close: async (ui) => (await ui.pixelRatio(1), await ui.escapeAll(3), await ui.eval(BACK)),
  },
  {
    // The sheet mode's keys, checked as they are pressed: H and V take the paper's tools, Ctrl+1 is real size and
    // Ctrl+0 the whole page, a drawing tool's letter (C) is held and said why, M takes the mode's map tool and Esc
    // puts Seç back; an arrow moves the chosen item by the engine's step, Geri al in the quick access bar takes it
    // back on the sheet's own history (W-10), Ctrl+Y makes it again, Ctrl+Z takes it back.
    id: 'tuslar',
    open: async (ui) => {
      await opened(ui);
      await ui.clickSel('.sheet-stage__paper', 0.02, 0.02);
      const tool = run(`return s.state.tool.value.kind + (s.state.tool.value.tool ? ':' + s.state.tool.value.tool : '');`);
      const zoom = `document.querySelector('.sheet-zoom__value').textContent`;
      const lastLog = `window.kentos.log.entries.value.at(-1)?.text ?? ''`;
      const left = run(`return s.state.sheet.items.find((i) => i.name === 'Kuzey oku').frame.left;`);
      await ui.key('h');
      await ui.expect(`(${tool}).then((t) => t === 'hand')`, 'H El aracını seçmedi');
      await ui.key('v');
      await ui.expect(`(${tool}).then((t) => t === 'select')`, 'V Seç aracını seçmedi');
      await ui.key('1', { ctrl: true });
      await ui.sleep(200);
      await ui.expect(`${zoom} === '%100'`, 'Ctrl+1 gerçek boyuta geçmedi');
      await ui.key('0', { ctrl: true });
      await ui.sleep(200);
      await ui.expect(`${zoom} !== '%100'`, 'Ctrl+0 sayfayı sığdırmadı');
      await ui.key('c');
      await ui.expect(`window.kentos.tools.activeId.value === 'select' && (${lastLog}).includes('çizim alanının kısayolu')`, 'C çizimin aracını başlattı ya da nedenini söylemedi');
      await ui.key('m');
      await ui.expect(`(${tool}).then((t) => t === 'add:map')`, 'M haritanın aracını almadı');
      await ui.key('Escape');
      await ui.expect(`(${tool}).then((t) => t === 'select')`, 'Esc Seç aracına dönmedi');
      const before = await ui.eval(left);
      await ui.eval(CHOOSE('Kuzey oku'));
      await ui.key('ArrowRight', { shift: true });
      await ui.expect(`(${left}).then((l) => l === ${before} + 10)`, 'Shift+→ öğeyi 10 mm taşımadı');
      await ui.clickSel('.ribbon__qat [data-command="edit.undo"]');
      await ui.expect(`(${left}).then((l) => l === ${before})`, 'Hızlı erişimdeki Geri al paftanın geçmişini geri almadı');
      await ui.clickSel('.sheet-stage__paper', 0.02, 0.02);
      await ui.eval(CHOOSE('Kuzey oku'));
      await ui.key('y', { ctrl: true });
      await ui.expect(`(${left}).then((l) => l === ${before} + 10)`, 'Ctrl+Y yinelemedi');
      await ui.key('z', { ctrl: true });
      await ui.expect(`(${left}).then((l) => l === ${before})`, 'Ctrl+Z geri almadı');
      await ui.expect(`(${lastLog}).includes('Geri alındı (pafta)')`, 'Geri alma ileti alanında söylenmedi');
      await ui.move(2, 2);
    },
  },
  {
    // In CAD, another mode's templates shown with “Bütün kiplerin şablonları”, badged; what one needs said before use.
    id: 'galeri-butun-kipler',
    open: async (ui) => {
      await opened(ui, 'cad');
      await gallery(ui);
      await ui.clickSel('.tgal__all .switch');
      await ui.sleep(600);
      await ui.eval(`document.querySelector('.tcard[data-id="sys:gis-tematik"]')?.scrollIntoView({ block: 'nearest' })`);
      await ui.clickSel('.tcard[data-id="sys:gis-tematik"]');
      await ui.move(2, 2);
      await ui.sleep(500);
    },
  },
  { id: 'galeri-kurumum', open: async (ui) => (await opened(ui), await gallery(ui), await ui.clickSel('.tgal__src[data-section="org"]'), await ui.sleep(400), await ui.move(2, 2)) },
  {
    id: 'sablon-paylas',
    open: async (ui) => {
      await opened(ui);
      await gallery(ui);
      await ui.clickSel('.tgal__src[data-section="mine"]');
      await ui.sleep(400);
      await ui.clickSel('.tcard');
      await ui.clickText('.tgal__actions .btn', 'Paylaş');
      await ui.sleep(400);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(BACK)),
  },
  // Kullan asks the template's questions before the sheet is made (design §12), filled with its own values.
  {
    id: 'sablon-sorulari',
    open: async (ui) => {
      await opened(ui);
      await gallery(ui);
      await ui.clickSel('.tcard[data-id="sys:ifraz-paftasi"]');
      await ui.sleep(300);
      await ui.clickText('.tgal .btn--primary', 'Kullan');
      await ui.waitFor(`document.querySelector('.sheet-ask-q')`);
      await ui.eval(`(() => { const f = [...document.querySelectorAll('.sheet-ask-q input')]; const put = (i, v) => { f[i].value = v; }; put(0, 'Sivas'); put(1, 'Suşehri'); put(2, 'Kızılırmak'); put(3, '1246'); return f.length; })()`);
      await ui.sleep(300);
      await ui.move(2, 2);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(BACK)),
  },
  // ── The sheet's windows ────────────────────────────────────────────
  {
    id: 'degiskenler',
    open: async (ui) => (await opened(ui), await ui.eval(`window.kentos.commands.execute('sheet.variables')`), await ui.waitFor(`document.querySelector('.sheet-vars')`), await ui.sleep(400), await ui.move(2, 2)),
  },
  {
    id: 'sayfa-ayarlari',
    open: async (ui) => (await opened(ui), await ui.eval(`window.kentos.commands.execute('sheet.pageSetup')`), await ui.waitFor(`document.querySelector('.sheet-dialog')`), await ui.sleep(400), await ui.move(2, 2)),
  },
  {
    // ƒ on the map's width: the engine's names for what may be bound, the expression checked as it is typed.
    id: 'fx-ifade',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(CHOOSE('Harita'));
      await ui.sleep(300);
      await ui.clickSel('.sheet-field__box:has([data-key="frame.width"]) .sheet-fx');
      await ui.waitFor(`document.querySelector('.sheet-expr')`);
      await ui.eval(`(() => { const t = document.querySelector('.sheet-expr__input'); t.value = "if(@kagit = 'A3', 273, 380)"; t.dispatchEvent(new Event('input')); return true; })()`);
      await ui.sleep(300);
      await ui.move(2, 2);
    },
  },
  {
    // A tool of the mode in hand (Metin): a frame dragged out on the paper with its size.
    id: 'arac-ekle',
    open: async (ui) => {
      await opened(ui);
      await ui.eval(`window.kentos.commands.execute('sheet.add.text')`);
      const p = await ui.eval(AT('Harita', 0.12, 0.08));
      await ui.dragHold(p, [150, 40]);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.key('Escape'), await ui.release(), await ui.escapeAll(2), await ui.eval(BACK)),
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(3), await ui.eval(BACK), await ui.sleep(150)), ...s }));

// The app's own root and config, from wherever the script is run (the repository root or apps/web).
const WEB = new URL('../..', import.meta.url).pathname;
const server = await createServer({ root: WEB, configFile: join(WEB, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
const written = [];
const failed = [];

for (const [w, hgt] of sizes) {
  for (const theme of themes) {
    const b = await launch('about:blank', { width: w, height: hgt });
    const ui = helpers(b);
    try {
      await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
      const ready = 'window.kentos && window.kentos.view.backendKind.value';
      await b.waitFor(ready, 30000);
      await sleep(1200);
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      await b.eval('document.fonts.ready');
      await sleep(300);
      for (const scene of SCENES) {
        if (only && !only.includes(scene.id)) continue;
        const name = `${scene.id}-${theme}-${w}`;
        try {
          await scene.open(ui);
          await sleep(250);
          written.push(await b.shot(name, scene.clip ? await scene.clip(ui) : undefined, DIR));
        } catch (e) {
          failed.push(`${name}: ${String(e.message ?? e).slice(0, 300)}`);
        }
        try {
          await scene.close(ui);
        } catch (e) {
          failed.push(`${name} (kapanış): ${String(e.message ?? e).slice(0, 200)}`);
        }
      }
      const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
      if (errors.length) failed.push(...errors.map((l) => `${theme}-${w} konsol: ${l.slice(0, 300)}`));
    } finally {
      b.close();
    }
  }
}
await server.close();
console.log(`${written.length} resim: ${DIR}`);
for (const f of failed) console.log(`✗ ${f}`);
process.exit(failed.length ? 1 : 0);

/** Actions the scenes use on page `b`. */
function helpers(b) {
  const at = (sel, fx = 0.5, fy = 0.5) =>
    b.eval(`(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width * ${fx}), Math.round(r.top + r.height * ${fy})]; })()`);
  const byText = (sel, text) =>
    b.eval(
      `(() => { const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)})); if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`,
    );
  let held = null;
  const ui = {
    state: {},
    eval: (expr) => b.eval(expr),
    sleep,
    waitFor: (expr, ms = 8000) => b.waitFor(expr, ms),
    move: (x, y) => b.move(x, y),
    click: async (p) => (await b.click(...p), await sleep(200)),
    key: (k, mods) => b.key(k, mods),
    /** A check on the page: throws (the scene fails) when the expression is not true. */
    expect: async (expr, what) => {
      await sleep(120);
      if (!(await b.eval(expr))) throw new Error(what);
    },
    /** The screen's pixel ratio (a HiDPI screen is 2). */
    pixelRatio: async (dpr) => {
      const size = await b.eval('[innerWidth, innerHeight]');
      await b.send('Emulation.setDeviceMetricsOverride', { width: size[0], height: size[1], deviceScaleFactor: dpr, mobile: false });
      await sleep(300);
    },
    /** Presses at `p` and moves by `by` in steps, the button kept down (released by `release`). */
    dragHold: async (p, by) => {
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p[0], y: p[1] });
      await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p[0], y: p[1], button: 'left', buttons: 1, clickCount: 1 });
      const steps = 10;
      for (let i = 1; i <= steps; i++) {
        await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p[0] + (by[0] * i) / steps, y: p[1] + (by[1] * i) / steps, button: 'left', buttons: 1 });
        await sleep(40);
      }
      held = [p[0] + by[0], p[1] + by[1]];
    },
    release: async () => {
      if (!held) return;
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: held[0], y: held[1], button: 'left', buttons: 0, clickCount: 1 });
      held = null;
      await sleep(200);
    },
    clickSel: async (sel, fx = 0.5, fy = 0.5) => {
      const p = await at(sel, fx, fy);
      if (!p) throw new Error(`yok: ${sel}`);
      await b.click(...p);
      await sleep(250);
    },
    clickText: async (sel, text) => {
      const p = await byText(sel, text);
      if (!p) throw new Error(`yok: ${sel} “${text}”`);
      await b.click(...p);
      await sleep(250);
    },
    hoverSel: async (sel, fx = 0.5, fy = 0.5) => {
      const p = await at(sel, fx, fy);
      if (!p) throw new Error(`yok: ${sel}`);
      await b.move(p[0] - 3, p[1] - 3);
      await b.move(...p);
      await sleep(600);
    },
    contextSel: async (sel) => {
      const p = await at(sel);
      if (!p) throw new Error(`yok: ${sel}`);
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p[0], y: p[1] });
      await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p[0], y: p[1], button: 'right', buttons: 2, clickCount: 1 });
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: p[0], y: p[1], button: 'right', buttons: 0, clickCount: 1 });
      await sleep(300);
    },
    escapeAll: async (times) => {
      for (let i = 0; i < times; i++) {
        await b.key('Escape');
        await sleep(120);
      }
      await b.move(2, 2);
    },
  };
  return ui;
}
