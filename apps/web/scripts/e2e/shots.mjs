// Reference pictures of the web for porting (not a test): named scenes, each at 1440×900 and 1100×650 in the
// dark and the light theme, into scripts/e2e/out/shots/<group>/<scene>-<theme>-<width>.png, on the demo drawing.
// A scene opens what it shows through the app's own commands and the dev-only window.kentos handle, and leaves
// the app as it found it (closing windows, undoing runs).
//
//   node scripts/e2e/shots.mjs <group> [--only a,b] [--sizes 1440x900,1100x650] [--themes dark,light] [--renderer webgpu]
//
// --renderer webgpu draws with WebGPU (Chrome's SwiftShader adapter); the pictures' names end in -webgpu.
//
// Groups: processing (İşlemler: the dock, the menu, the tool and model dialogs and their states); layerstyle (Katman
// stili: each renderer, its classes, the symbol slot, errors, applied); stylemanager (Stil yöneticisi: the tree, a
// search, the kinds, system and own items, the menus, a delete question, applying to a selection, pick mode); legend
// (Lejant: its options, a layer left out, a categorized layer); symboldesigner (Sembol tasarımcısı: every layer type's
// form, the add menus, a child marker, ƒ on, the preview geometry, the unsaved question, inline and library symbols);
// svgedit (SVG düzenleyicisi: a new and a library drawing, shapes, the tabs, the menus, a polyline in progress, text,
// node editing, measuring, the XML source, document properties, export, the unsaved question);
// log (the bottom panel's lines with their times and levels, Uyarılar with its badge, a warning in the status bar, the
// empty history); layout (panels and sizes as kept, sizes kept larger than the window shown within it, the
// ribbon's kept tab, quick access and split choices, folded); modeldesigner (Model tasarımcısı: a new model, the
// built-in model's copy, an input, a step and a source list, a wire dragged and its menu, a step with problems, a chain,
// a number input, the tools searched and one carried, the unsaved question); ribbon (the key tips on the tabs, on
// Giriş and narrowed, the quick access bar's menu, right clicks on a command, an added one, a fixed one and a tab, a
// tool's methods and a family under their split buttons, the folded ribbon open); types (docs/adr/0165 §6: every tab
// of a CAD and of a CBS project's ribbon); tools (docs/adr/0140: the tabs of
// the new drawing and editing tools, their split buttons, each tool at work); blocks (docs/adr/0144: DXF içe aktar
// over a file with blocks read a second time, Blokları patlat clicked, the imported blocks in the Bloklar panel);
// secondcrs (docs/adr/0167: the second system's values in the status bar, the coordinate system's menu, Koordinat oku,
// Proje ayarları' field); convert (Koordinat dönüştür: a point to the second system and to WGS 84, the systems, a list);
// customcrs (docs/adr/0168: a second system the project defines, its menu; a project whose system is its own
// definition, its second system by the project's datum choice, Koordinat oku); grids (Proje ayarları' Izgaralar: a
// grid added, one a datum choice names that the device does not have, Kaldır's question); datums (Proje ayarları' Datum
// dönüşümleri: seven parameters with a translation still to type, a grid of the library, EPSG's way); definitions
// (Özel koordinat sistemi: new, a TM on the project's datum, what is wrong, a local system by an affine; the definition
// chosen in Proje ayarları, a second definition in its list; a WKT read, a text not read, a trial point; common points);
// survey (Proje ayarları' Ölçme: empty, k and the tolerances typed, what does not hold); fieldbook (Karne editörü: a GSI
// book with a tolerance exceeded, a text book's columns, Kutupsal alım filled from a station, Poligon hesabı from both);
// gnss (GNSS içe aktar: a GPX and an NMEA file, the points imported, a project without a coordinate system);
// fieldsend (Cihaza gönder: Leica GSI-16, Trimble JobXML, Leica GSI-8 over TM coordinates); ground (docs/adr/0171 §4:
// Ölçme's reduction to the grid switched on, Kutupsal alım, Aplikasyon and Poligon hesabı reducing at 850 m);
// vertextable (docs/adr/0172: Köşe tablosu, a row selected, a value typed, a radius refused, a road, two objects);
// sources (docs/adr/0199 §7: Kaynaklar with a folder added and a folder inside it open, a file's menu); fields
// (docs/adr/0199 §5: Öznitelikler's fields by their kinds).
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, OUT, sleep, WEBGPU_ARGS } from './cdp.mjs';

const args = process.argv.slice(2);
const group = args.find((a) => !a.startsWith('--') && !args[args.indexOf(a) - 1]?.startsWith('--'));
const opt = (name) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1].split(',') : null);
const only = opt('only');
const sizes = (opt('sizes') ?? ['1440x900', '1100x650']).map((s) => s.split('x').map(Number));
const themes = opt('themes') ?? ['dark', 'light'];
const renderer = opt('renderer')?.[0] ?? 'webgl2';

/** A point of the drawing area, as fractions of its box, on the screen. */
const inView = (ui, fx, fy) => ui.eval(`(() => { const r = window.kentos.view.clientRect(); return [Math.round(r.left + r.width * ${fx}), Math.round(r.top + r.height * ${fy})]; })()`);

/**
 * The numbering dialog on two selected parcels, its input's Sahneden seç pressed; the screen points of two
 * other parcels to click, where the drawing is not under a floating panel.
 */
async function pickParcels(ui) {
  await ui.eval(SELECT_PARCELS);
  await ui.eval(openTool('points.numberVertices'));
  await ui.sleep(400);
  await ui.clickSel('[data-param="input"] .pfield__pick');
  await ui.sleep(300);
  const at = await ui.eval(`(() => {
    const k = window.kentos;
    const r = k.view.clientRect();
    const out = [];
    for (const e of k.doc.all()) {
      if (e.kind !== 'polygon' || !k.doc.layers.isVisible(e.layerId)) continue;
      const c = e.pts.reduce((a, p) => ({ x: a.x + p.x / e.pts.length, y: a.y + p.y / e.pts.length }), { x: 0, y: 0 });
      const s = k.view.camera.worldToScreen(c);
      const x = Math.round(s.x + r.left);
      const y = Math.round(s.y + r.top);
      if (s.x < 120 || s.y < 120 || s.x > r.width - 160 || s.y > r.height - 120) continue;
      // The parcel itself answers around the click (no spot height or label point on it), on the drawing's own canvas.
      const around = [[0, 0], [-4, 0], [4, 0], [0, -4], [0, 4]].every(([dx, dy]) => k.view.pick({ x: x - r.left + dx, y: y - r.top + dy })?.id === e.id);
      if (!around || document.elementFromPoint(x, y)?.tagName !== 'CANVAS') continue;
      out.push([x, y]);
      if (out.length === 2) break;
    }
    return out;
  })()`);
  if (at.length < 2) throw new Error('ekranda iki parsel yok');
  return at;
}

/** Two parcels of the demo drawing selected: the tools' default input (Seçili) then has something to read. */
const SELECT_PARCELS = `(() => {
  const k = window.kentos;
  const parcels = [...k.doc.all()].filter((e) => e.kind === 'polygon' && !e.symbol && k.doc.layers.isVisible(e.layerId)).slice(0, 2);
  k.selection.set(parcels.map((e) => e.id));
  return parcels.length;
})()`;
const openTool = (id, values) => `import('/src/ui/processing/ToolDialog.ts').then((m) => m.openToolDialog(window.kentos, ${JSON.stringify(id)}, ${values ? JSON.stringify(values) : 'undefined'}))`;
const dock = (tab) => `(() => { const k = window.kentos; k.ui.rightPanel?.set?.(true); k.ui.dockTab.set('processing'); k.ui.processingTab.set(${JSON.stringify(tab)}); })()`;

/** Three runs for the history: numbering (ok), the same tool on a place this app cannot run it (error), the parcel sheet model (ok). */
const RUNS = `(async () => {
  const k = window.kentos;
  const { defaultValues } = await import('/src/processing/parameters.ts');
  const { runModel } = await import('/src/processing/modelRunner.ts');
  const p = k.processing;
  const tool = p.registry.get('points.numberVertices');
  const values = defaultValues(tool, p.runner.defaults());
  await p.runner.run(tool, values);
  await p.runner.run(tool, values, { target: 'server' });
  const model = p.model('builtin.parcelSheet');
  await runModel(model, { parcels: { scope: 'selection' }, prefix: 'K' }, p.runner, (id) => p.registry.get(id));
  return p.runner.history.value.map((r) => r.status);
})()`;

const SCENES = {
  processing: [
    { id: 'dock-tools', open: async (ui) => (await ui.eval(dock('tools')), await ui.sleep(400)) },
    {
      id: 'dock-search',
      open: async (ui) => {
        await ui.eval(dock('tools'));
        await ui.sleep(300);
        await ui.clickSel('.panel input.field--search[aria-label="İşlem ara"]');
        await ui.type('kose');
        await ui.sleep(300);
      },
      close: async (ui) => (await ui.eval(`(() => { const s = document.querySelector('input[aria-label="İşlem ara"]'); s.value = ''; s.dispatchEvent(new Event('input', { bubbles: true })); })()`), await ui.escapeAll(1)),
    },
    {
      id: 'dock-history',
      open: async (ui) => {
        await ui.eval(SELECT_PARCELS);
        await ui.eval(RUNS);
        await ui.eval(dock('history'));
        await ui.sleep(400);
      },
      close: async (ui) => (await ui.eval(`(() => { const k = window.kentos; while (k.doc.canUndo.value) k.doc.undo(); k.ui.processingTab.set('tools'); })()`), await ui.escapeAll(1)),
    },
    { id: 'dialog-numbering', open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('points.numberVertices')), await ui.sleep(500)) },
    { id: 'dialog-edge-lengths', open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('annotation.edgeLengths')), await ui.sleep(500)) },
    { id: 'dialog-calculate', open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('attributes.calculate')), await ui.sleep(500)) },
    { id: 'dialog-select-by-expression', open: async (ui) => (await ui.eval(openTool('selection.byExpression')), await ui.sleep(500)) },
    { id: 'dialog-model-parcel-sheet', open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('model:builtin.parcelSheet')), await ui.sleep(500)) },
    {
      id: 'dialog-numbering-advanced',
      open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('points.numberVertices')), await ui.sleep(400), await ui.clickSel('.pgroup__toggle'), await ui.sleep(300)),
    },
    { id: 'dialog-numbering-start-point', open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('points.numberVertices', { start: 'point' })), await ui.sleep(500)) },
    {
      id: 'dialog-calculate-variables',
      open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('attributes.calculate')), await ui.sleep(400), await ui.clickText('.pchip--menu', 'Değişkenler'), await ui.sleep(300)),
    },
    {
      id: 'dialog-calculate-functions',
      open: async (ui) => (await ui.eval(SELECT_PARCELS), await ui.eval(openTool('attributes.calculate')), await ui.sleep(400), await ui.clickText('.pchip--menu', 'İşlevler'), await ui.sleep(300)),
    },
    {
      id: 'dialog-after-ok',
      open: async (ui) => {
        await ui.eval(SELECT_PARCELS);
        await ui.eval(openTool('points.numberVertices'));
        await ui.sleep(400);
        await ui.clickSel('.ptool__run');
        await ui.waitFor(`[...document.querySelectorAll('.dialog button')].some((b) => b.textContent.includes('Sonuçları seç'))`);
        await ui.sleep(300);
      },
      close: async (ui) => (await ui.escapeAll(2), await ui.eval(`(() => { const k = window.kentos; while (k.doc.canUndo.value) k.doc.undo(); })()`)),
    },
    {
      id: 'dialog-invalid',
      open: async (ui) => {
        await ui.eval(`window.kentos.selection.clear()`);
        await ui.eval(openTool('points.numberVertices'));
        await ui.sleep(400);
        await ui.clickSel('.ptool__run');
        await ui.sleep(400);
      },
    },
    // Sahneden seç (docs/adr/0088): the start vertex picked beside its choice, input objects picked on the drawing.
    {
      id: 'dialog-numbering-picked-start',
      open: async (ui) => {
        await ui.eval(SELECT_PARCELS);
        await ui.eval(openTool('points.numberVertices'));
        await ui.sleep(400);
        await ui.clickSel('[data-param="start"] .pfield__pick');
        await ui.clickAt(...(await inView(ui, 0.4, 0.45)));
        await ui.waitFor(`!!document.querySelector('.dialog--ptool [data-param="startPoint"] .pfield__coord.num')`);
        await ui.sleep(400);
      },
    },
    {
      id: 'pick-objects-running',
      open: async (ui) => {
        const at = await pickParcels(ui);
        await ui.clickAt(...at[0]);
        await ui.move(...at[1]);
        await ui.sleep(400);
      },
    },
    {
      id: 'pick-objects-box',
      open: async (ui) => {
        const at = await pickParcels(ui);
        await ui.clickAt(...at[0]);
        // A crossing box, right to left, held while the picture is taken.
        const [x, y] = at[1];
        await ui.move(x + 80, y - 50);
        await ui.pressAt(x + 80, y - 50);
        for (let i = 1; i <= 6; i++) await ui.moveHeld(x + 80 - (i * 160) / 6, y - 50 + (i * 100) / 6);
        await ui.sleep(300);
      },
      close: async (ui) => (await ui.releaseAt(2, 2), await ui.escapeAll(3)),
    },
    {
      id: 'pick-objects-back',
      open: async (ui) => {
        const at = await pickParcels(ui);
        await ui.clickAt(...at[0]);
        await ui.clickAt(...at[1]);
        await ui.key('Enter');
        await ui.waitFor(`!!document.querySelector('.dialog--ptool')`);
        await ui.sleep(500);
      },
    },
    {
      id: 'dialog-field-error',
      open: async (ui) => {
        await ui.eval(SELECT_PARCELS);
        await ui.eval(openTool('points.numberVertices'));
        await ui.sleep(400);
        await ui.eval(`(() => { const row = [...document.querySelectorAll('.prow')].find((r) => r.querySelector('.prow__label')?.textContent.includes('Toplam uzunluk')); const input = row.querySelector('input'); input.focus(); input.select(); })()`);
        await ui.type('99');
        await ui.sleep(400);
      },
    },
  ],
  // Katman stili (ui/style/LayerStyleDialog.ts): each kind of renderer, its classes made from the data, the symbol
  // slot's menu and what it opens, errors, applied; on parcels (areas), building footprints, roads and points.
  layerstyle: [
    { id: 'simple', open: (ui) => openStyled(ui, 'parsel', 'Basit') },
    { id: 'single', open: (ui) => openStyled(ui, 'parsel', 'Tek sembol') },
    {
      id: 'single-slot-menu',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Tek sembol');
        await ui.clickSel('.dialog--lstyle .slot');
        await ui.waitFor(`!!document.querySelector('.menu .menu__item')`);
        await ui.sleep(300);
      },
    },
    {
      id: 'single-pick-from-library',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Tek sembol');
        await ui.clickSel('.dialog--lstyle .slot');
        await ui.clickText('.menu .menu__item', 'Kitaplıktan seç');
        await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
        await ui.sleep(800);
      },
      close: (ui) => ui.escapeAll(3),
    },
    {
      id: 'single-edit-symbol',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Tek sembol');
        await ui.clickSel('.dialog--lstyle .slot');
        await ui.clickText('.menu .menu__item', 'Düzenle');
        await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
        await ui.sleep(800);
      },
      close: (ui) => ui.escapeAll(3),
    },
    { id: 'categorized-empty', open: (ui) => openStyled(ui, 'parsel', 'Kategorili') },
    {
      id: 'categorized',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kategorili');
        await ui.eval(setStyleExpr('Nitelik'));
        await ui.clickText('.dialog--lstyle .btn', 'Değerlerden sınıfla');
        await ui.sleep(500);
      },
    },
    {
      id: 'categorized-other-drawn',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kategorili');
        await ui.eval(setStyleExpr('Nitelik'));
        await ui.clickText('.dialog--lstyle .btn', 'Değerlerden sınıfla');
        await ui.eval(`document.querySelector('.dialog--lstyle input[aria-label="Diğer değerler çizilsin"]').click()`);
        await ui.sleep(500);
      },
    },
    {
      id: 'categorized-expression-error',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kategorili');
        await ui.eval(setStyleExpr('Nitelik ='));
        await ui.sleep(500);
      },
    },
    {
      id: 'categorized-lines',
      open: async (ui) => {
        await openStyled(ui, 'yol-ekseni', 'Kategorili');
        await ui.eval(setStyleExpr('Ad'));
        await ui.clickText('.dialog--lstyle .btn', 'Değerlerden sınıfla');
        await ui.sleep(500);
      },
    },
    { id: 'graduated-empty', open: (ui) => openStyled(ui, 'parsel', 'Aralıklı') },
    {
      id: 'graduated',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Aralıklı');
        await ui.clickText('.dialog--lstyle .btn', 'Sınıfla');
        await ui.sleep(500);
      },
    },
    {
      id: 'graduated-count-buildings',
      open: async (ui) => {
        await openStyled(ui, 'yapi', 'Aralıklı');
        await ui.eval(setStyleExpr('[Kat adedi]'));
        await ui.eval(`(() => { const d = document.querySelector('.dialog--lstyle'); const m = d.querySelector('select[aria-label="Yöntem"]'); m.value = 'count'; m.dispatchEvent(new Event('change')); const n = d.querySelector('input[aria-label="Sınıf sayısı"]'); n.value = '4'; n.dispatchEvent(new Event('change')); })()`);
        await ui.clickText('.dialog--lstyle .btn', 'Sınıfla');
        await ui.sleep(500);
      },
    },
    { id: 'rules-start', open: (ui) => openStyled(ui, 'parsel', 'Kurallar') },
    {
      id: 'rules',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kurallar');
        await ui.clickText('.dialog--lstyle .btn', 'Kural ekle');
        await ui.eval(setRuleFilter(1, "Nitelik = 'Arsa'"));
        await ui.eval(`document.querySelectorAll('.dialog--lstyle .rule')[1].querySelector('.ibtn[aria-label="Alt kural ekle"]').click()`);
        await ui.sleep(300);
        await ui.eval(setRuleFilter(2, '$alan > 500'));
        await ui.clickText('.dialog--lstyle .btn', 'Değilse kuralı ekle');
        await ui.sleep(500);
      },
    },
    {
      id: 'rules-filter-error',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kurallar');
        await ui.clickText('.dialog--lstyle .btn', 'Kural ekle');
        await ui.eval(setRuleFilter(1, 'Nitelik ='));
        await ui.sleep(500);
      },
    },
    { id: 'points-single', open: (ui) => openStyled(ui, 'poligon', 'Tek sembol') },
    {
      id: 'categorized-applied',
      open: async (ui) => {
        await openStyled(ui, 'parsel', 'Kategorili');
        await ui.eval(setStyleExpr('Nitelik'));
        await ui.clickText('.dialog--lstyle .btn', 'Değerlerden sınıfla');
        await ui.clickText('.dialog--lstyle .btn', 'Uygula');
        await ui.sleep(900);
      },
      close: async (ui) => (await ui.escapeAll(2), await ui.eval(`(() => { const d = window.kentos.doc; while (d.canUndo.value) d.undo(); })()`)),
    },
  ],
};

SCENES.stylemanager = [
  { id: 'open', open: (ui) => openManager(ui) },
  {
    id: 'mpyy-expanded',
    open: async (ui) => {
      await openManager(ui);
      await ui.eval(treeCaret('MPYY'));
      await ui.sleep(300);
      await ui.clickText('.dialog--styles .tree__row', 'Piktogramlar');
      await ui.sleep(700);
    },
  },
  {
    id: 'search',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickSel('.dialog--styles .smgr__search');
      await ui.type('konut');
      await ui.sleep(700);
    },
  },
  {
    id: 'kind-marker',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'İşaret');
      await ui.sleep(700);
    },
  },
  {
    // Object templates in Kitaplığım (docs/adr/0176): the desktop's `with_templates`.
    id: 'templates',
    open: async (ui) => {
      await ui.eval(ADD_TEMPLATES);
      await openManager(ui);
      await ui.clickText('.dialog--styles .tree__row', 'Kitaplığım');
      await ui.sleep(300);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'Şablon');
      await ui.sleep(500);
      await ui.clickText('.dialog--styles .scard', 'Parsel sınırı');
      await ui.sleep(500);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(REMOVE_MINE)),
  },
  {
    // Şablon düzenleyici on Parsel sınırı (docs/adr/0176 §4): the desktop's `template_editor` screens.
    id: 'template-editor',
    open: async (ui) => {
      await ui.eval(ADD_TEMPLATES);
      await openManager(ui);
      await ui.clickText('.dialog--styles .tree__row', 'Kitaplığım');
      await ui.sleep(300);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'Şablon');
      await ui.sleep(500);
      await ui.clickText('.dialog--styles .scard', 'Parsel sınırı');
      await ui.sleep(500);
      await ui.clickText('.dialog--styles .smgr__actions .btn', 'Düzenle');
      await ui.sleep(900);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(REMOVE_MINE)),
  },
  {
    // A group template's members in Şablon düzenleyici (docs/adr/0176 §5): the desktop's `template_editor` screens' “grup”.
    id: 'template-editor-group',
    open: async (ui) => {
      await ui.eval(ADD_TEMPLATES);
      await ui.eval(ADD_GROUP_TEMPLATE);
      await openManager(ui);
      await ui.clickText('.dialog--styles .tree__row', 'Kitaplığım');
      await ui.sleep(300);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'Şablon');
      await ui.sleep(500);
      await ui.clickText('.dialog--styles .scard', 'Parsel ve köşeleri');
      await ui.sleep(500);
      await ui.clickText('.dialog--styles .smgr__actions .btn', 'Düzenle');
      await ui.sleep(900);
      await ui.eval(`(() => { const b = [...document.querySelectorAll('.dialog__body')].at(-1); b.scrollTop = b.scrollHeight; })()`);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(REMOVE_MINE)),
  },
  {
    id: 'kind-drawings',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'Çizim');
      await ui.sleep(500);
      await ui.clickSel('.dialog--styles .scard');
      await ui.sleep(500);
    },
  },
  {
    id: 'system-symbol-selected',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickSel('.dialog--styles .scard');
      await ui.sleep(500);
    },
  },
  {
    id: 'copy-menu',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickSel('.dialog--styles .scard');
      await ui.clickText('.dialog--styles .smgr__actions .btn', 'Kopyala');
      await ui.sleep(300);
    },
  },
  {
    id: 'user-symbol-selected',
    open: async (ui) => {
      await openManager(ui);
      await copyFirstToMine(ui);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(REMOVE_MINE)),
  },
  {
    id: 'delete-question',
    open: async (ui) => {
      await openManager(ui);
      await copyFirstToMine(ui);
      await ui.clickText('.dialog--styles .smgr__actions .btn', 'Sil');
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(REMOVE_MINE)),
  },
  {
    id: 'user-category-menu',
    open: async (ui) => {
      await openManager(ui);
      await copyFirstToMine(ui);
      const at = await ui.eval(`(() => { const r = [...document.querySelectorAll('.dialog--styles .tree__row')].find((e) => e.textContent.trim().startsWith('Kitaplığım')); const b = r.getBoundingClientRect(); return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)]; })()`);
      await ui.contextClick(...at);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(REMOVE_MINE)),
  },
  {
    id: 'new-menu',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickText('.dialog--styles .btn', 'Yeni sembol');
      await ui.sleep(300);
    },
  },
  {
    id: 'import-menu',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickText('.dialog--styles .btn', 'İçe aktar');
      await ui.sleep(300);
    },
  },
  {
    id: 'export-menu',
    open: async (ui) => {
      await openManager(ui);
      await ui.clickText('.dialog--styles .btn', 'Dışa aktar');
      await ui.sleep(300);
    },
  },
  {
    id: 'apply-to-selection',
    open: async (ui) => {
      await ui.eval(SELECT_PARCELS);
      await openManager(ui);
      await ui.clickText('.dialog--styles .smgr__kinds .seg__opt', 'Alan');
      await ui.sleep(400);
      await ui.clickSel('.dialog--styles .scard');
      await ui.clickText('.dialog--styles .smgr__actions .btn', 'Seçili nesnelere uygula');
      await ui.sleep(600);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(`(() => { const d = window.kentos.doc; while (d.canUndo.value) d.undo(); })()`)),
  },
  {
    id: 'pick-marker',
    open: async (ui) => {
      await ui.eval(`import('/src/ui/style/StyleManager.ts').then((m) => m.openStyleManager(window.kentos, { pick: { kind: 'marker', title: 'Nokta sembolü seçin', onPick: () => {} } }))`);
      await ui.waitFor(`!!document.querySelector('.dialog--styles .scard')`);
      await ui.sleep(700);
      await ui.clickSel('.dialog--styles .scard');
      await ui.sleep(400);
    },
  },
];

SCENES.legend = [
  { id: 'open', open: (ui) => openLegend(ui) },
  { id: 'all-layers', open: async (ui) => (await openLegend(ui), await ui.clickText('.dialog--legend label', 'Yalnızca görünen katmanlar'), await ui.sleep(500)) },
  { id: 'no-headings', open: async (ui) => (await openLegend(ui), await ui.clickText('.dialog--legend label', 'Katman adlarını başlık yaz'), await ui.sleep(500)) },
  {
    id: 'layer-left-out',
    open: async (ui) => (await openLegend(ui), await ui.eval(`document.querySelector('.dialog--legend .leg__head input')?.click()`), await ui.sleep(500)),
  },
  {
    id: 'categorized-layer',
    open: async (ui) => {
      await ui.eval(`(() => {
        const k = window.kentos;
        const fill = (c) => ({ type: 'fill', layers: [{ id: '0', type: 'simpleFill', color: c }, { id: '1', type: 'simpleLine', color: 'ink', width: 0.2 }] });
        k.doc.setLayerStyle('parsel', { renderer: { type: 'categorized', expr: 'Nitelik', categories: [
          { value: 'Arsa', label: 'Arsa', symbols: { fill: fill('#EDC948') } },
          { value: 'Kargir ev ve arsası', label: 'Kargir ev ve arsası', symbols: { fill: fill('#E15759') } },
        ] } });
      })()`);
      await openLegend(ui);
    },
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(`(() => { const d = window.kentos.doc; while (d.canUndo.value) d.undo(); })()`)),
  },
];

SCENES.symboldesigner = [
  ...[
    ['fill', 'Dolgu', 'layer-simple-fill'],
    ['fill', 'Tarama', 'layer-hatch-fill'],
    ['fill', 'Desen', 'layer-pattern-fill'],
    ['fill', 'Görüntü dolgusu', 'layer-image-fill'],
    ['fill', 'İç noktada işaret', 'layer-centroid-marker'],
    ['line', 'Çizgi', 'layer-simple-line'],
    ['line', 'Çizgi boyunca işaret', 'layer-marker-line'],
    ['marker', 'Şekil', 'layer-shape'],
    ['marker', 'Yazı', 'layer-text'],
    ['marker', 'SVG çizimi', 'layer-svg'],
    ['marker', 'Görüntü', 'layer-raster'],
  ].map(([kind, label, id]) => ({
    id,
    open: async (ui) => {
      await openDesigner(ui, kind);
      // The new symbol starts with its kind's first layer; any other is added and shown.
      if (!(await ui.eval(`document.querySelector('.dialog--sdesign .sdes__row[aria-selected="true"]')?.textContent.includes(${JSON.stringify(label)})`))) {
        await ui.clickText('.dialog--sdesign .btn', 'Katman ekle');
        await ui.clickText('.menu .menu__item', label);
        await ui.sleep(500);
      }
    },
    close: closeDesigner,
  })),
  {
    id: 'add-layer-menu',
    open: async (ui) => (await openDesigner(ui, 'fill'), await ui.clickText('.dialog--sdesign .btn', 'Katman ekle'), await ui.sleep(300)),
    close: closeDesigner,
  },
  {
    id: 'add-layer-menu-into-marker',
    open: async (ui) => {
      await openDesigner(ui, 'line');
      await ui.clickText('.dialog--sdesign .btn', 'Katman ekle');
      await ui.clickText('.menu .menu__item', 'Çizgi boyunca işaret');
      await ui.sleep(400);
      await ui.clickText('.dialog--sdesign .btn', 'Katman ekle');
      await ui.sleep(300);
    },
    close: closeDesigner,
  },
  {
    id: 'child-marker-layer',
    open: async (ui) => {
      await openDesigner(ui, 'line');
      await ui.clickText('.dialog--sdesign .btn', 'Katman ekle');
      await ui.clickText('.menu .menu__item', 'Çizgi boyunca işaret');
      await ui.sleep(400);
      await ui.clickSel('.dialog--sdesign .sdes__row--child');
      await ui.sleep(500);
    },
    close: closeDesigner,
  },
  {
    id: 'data-defined-on',
    open: async (ui) => (await openDesigner(ui, 'fill'), await ui.clickSel('.dialog--sdesign .sdf__fx'), await ui.sleep(500)),
    close: closeDesigner,
  },
  {
    id: 'preview-area-with-hole',
    open: async (ui) => (await openDesigner(ui, 'fill'), await ui.clickText('.dialog--sdesign .seg__opt', 'Adalı alan'), await ui.sleep(500)),
    close: closeDesigner,
  },
  {
    id: 'unsaved-close-question',
    open: async (ui) => {
      await openDesigner(ui, 'fill');
      await ui.eval(`(() => { const i = document.querySelector('.dialog--sdesign input[aria-label="Sembol adı"]'); i.value = 'Bahçe alanı'; i.dispatchEvent(new Event('input')); })()`);
      await ui.sleep(200);
      await ui.escapeAll(1);
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
      await ui.sleep(300);
    },
    close: closeDesigner,
  },
  {
    id: 'inline-from-layer-style',
    open: async (ui) => {
      await ui.eval(
        `import('/src/ui/style/SymbolDesigner.ts').then((m) => m.openSymbolDesigner(window.kentos, { inline: { symbol: { type: 'fill', layers: [{ id: '0', type: 'hatchFill', color: '#4E79A7', spacing: 2, angle: 45, width: 0.2 }, { id: '1', type: 'simpleLine', color: 'ink', width: 0.35 }] }, title: 'Tek sembol (alan)', onDone: () => {} } }))`,
      );
      await ui.waitFor(`!!document.querySelector('.dialog--sdesign .sdes__row')`);
      await ui.sleep(700);
    },
    close: closeDesigner,
  },
  {
    id: 'library-symbol',
    open: async (ui) => {
      await ui.eval(`(() => { const lib = window.kentos.styles.library; const c = lib.copy('temel.alan.capraz', 'user'); window.__shotCopy = c.id; })()`);
      await ui.eval(`import('/src/ui/style/SymbolDesigner.ts').then((m) => m.openSymbolDesigner(window.kentos, { id: window.__shotCopy }))`);
      await ui.waitFor(`!!document.querySelector('.dialog--sdesign .sdes__row')`);
      await ui.sleep(700);
    },
    close: async (ui) => (await closeDesigner(ui), await ui.eval(REMOVE_MINE)),
  },
];

/** The middle of an element on the screen. */
const centreOf = (sel) => `(() => { const r = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`;

// The log (ui/bottom/logPlan.ts, fixtures/shell/v1/log.json): a short session's lines in Komut geçmişi with their
// times and levels (a tool's name, what was typed, the points it took, a value not understood, a mistyped command, a
// file written) and the Uyarılar badge; the warnings alone in Uyarılar; a warning in the status bar with the panel
// closed; the empty history. Every scene puts the log, the panel and the drawing back as they were.
/** The command line emptied (a mistyped command stays in it, selected). */
const CMDLINE_EMPTY = `(() => { const i = document.querySelector('.cmdline__input'); i.value = ''; i.dispatchEvent(new Event('input', { bubbles: true })); })()`;
const LOG_KEEP = `(() => { const k = window.kentos; window.__shotLog = k.log.entries.value; window.__shotHeight = k.ui.bottomHeight.value; })()`;
const LOG_RESTORE = `(() => {
  const k = window.kentos;
  k.tools.activate('select');
  while (k.doc.canUndo.value) k.doc.undo();
  k.log.entries.set(window.__shotLog ?? []);
  k.ui.bottomHeight.set(window.__shotHeight ?? 190);
  k.ui.bottomTab.set('history');
  k.ui.bottomExpanded.set(false);
  ${CMDLINE_EMPTY};
})()`;
/** The panel open on a tab, tall enough for the whole session. */
const LOG_OPEN = (tab) => `(() => { const u = window.kentos.ui; u.bottomHeight.set(Math.min(300, Math.round(innerHeight * 0.4))); u.bottomTab.set('${tab}'); u.bottomExpanded.set(true); })()`;
/** A line typed into the command line and entered. */
async function typeLine(ui, text) {
  await ui.clickSel('.cmdline__input');
  await ui.eval(CMDLINE_EMPTY);
  await ui.type(text);
  await ui.key('Enter');
  await ui.sleep(300);
}
/** The log kept aside and emptied, then a short session written into it, a second or more between some lines. */
async function logSession(ui) {
  await ui.eval(LOG_KEEP);
  await ui.eval(`window.kentos.log.clear()`);
  await ui.eval(`window.kentos.commands.execute('tool.line')`);
  await ui.sleep(300);
  await typeLine(ui, '412100,4512100');
  await ui.sleep(1000);
  await typeLine(ui, '@40,0');
  await typeLine(ui, '@0,30');
  await typeLine(ui, '@40<');
  await ui.key('Escape');
  await ui.sleep(1000);
  await typeLine(ui, 'PARSLE');
  await ui.eval(`window.kentos.log.success('“ada-101.ncn” yazıldı: 12 nokta.')`);
  await ui.eval(`window.kentos.view.focus()`);
}
SCENES.log = [
  {
    id: 'history',
    open: async (ui) => {
      await logSession(ui);
      await ui.eval(LOG_OPEN('history'));
      await ui.sleep(400);
    },
  },
  {
    id: 'warnings',
    open: async (ui) => {
      await logSession(ui);
      await ui.eval(LOG_OPEN('messages'));
      await ui.sleep(400);
    },
  },
  {
    id: 'status-warning',
    open: async (ui) => {
      await ui.eval(LOG_KEEP);
      await ui.eval(`window.kentos.ui.bottomExpanded.set(false)`);
      await ui.eval(`window.kentos.commands.execute('tool.line')`);
      await ui.sleep(300);
      await typeLine(ui, '@40<');
      await ui.sleep(300);
    },
  },
  {
    id: 'history-empty',
    open: async (ui) => {
      await ui.eval(LOG_KEEP);
      await ui.eval(`window.kentos.log.clear()`);
      await ui.eval(LOG_OPEN('history'));
      // The status bar's message of an earlier line fades by itself (5 s, 9 s for a warning).
      await ui.waitFor(`!document.querySelector('.status__flash[data-show]')`, 12000);
      await ui.sleep(400);
    },
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(1), await ui.eval(LOG_RESTORE), await ui.sleep(200)), ...s }));

// The kept layout (app/layoutPlan.ts, fixtures/shell/v1/layout.json): panels and sizes as kept; a dock and a bottom
// panel kept larger than this window allows, shown within it; the ribbon with its kept tab, quick access commands and
// split choices, and folded. Each scene sets the live layout and puts it back after.
const LAYOUT_KEEP = `window.__shotLayout = Object.fromEntries(Object.entries(window.kentos.ui).map(([k, s]) => [k, s.value]))`;
const LAYOUT_BACK = `(() => { const ui = window.kentos.ui; for (const [k, v] of Object.entries(window.__shotLayout ?? {})) ui[k].set(v); })()`;
/** Layout fields set on the live layout (the kept ones first saved aside). */
const layoutSet = (fields) => `(() => { ${LAYOUT_KEEP}; const ui = window.kentos.ui; for (const [k, v] of Object.entries(${JSON.stringify(fields)})) ui[k].set(v); })()`;
/** The ribbon laid out again after its kept fields changed. */
async function ribbonSettled(ui) {
  await ui.waitFor(`!!document.querySelector('.ribbon__strip .rpanel') || !!document.querySelector('.ribbon[data-collapsed]')`, 10000);
  await ui.sleep(500);
}
const RIBBON_KEPT = { ribbonTab: 'home', ribbonQuickAccess: ['view.zoomExtents', 'tool.line'], ribbonSplits: { circle: 'tool.circle|3N', rectangle: 'tool.regularPolygon|' } };
SCENES.layout = [
  {
    id: 'kept',
    open: async (ui) => {
      await ui.eval(layoutSet({ dockWidth: 400, layersFraction: 0.35, bottomExpanded: true, bottomTab: 'coords' }));
      await ui.sleep(500);
    },
  },
  {
    id: 'window-limits',
    open: async (ui) => {
      await ui.eval(layoutSet({ dockWidth: 560, bottomExpanded: true, bottomHeight: 600 }));
      await ui.sleep(500);
    },
  },
  {
    id: 'ribbon-kept',
    open: async (ui) => {
      // A CAD project's Giriş, whose Çizim shows both kept choices (Daire's 3 nokta, Düzgün çokgen).
      await ui.eval(typeSet('cad'));
      await ui.eval(layoutSet(RIBBON_KEPT));
      await ribbonSettled(ui);
    },
  },
  {
    id: 'ribbon-collapsed',
    open: async (ui) => {
      await ui.eval(typeSet('cad'));
      await ui.eval(layoutSet({ ...RIBBON_KEPT, ribbonCollapsed: true }));
      await ribbonSettled(ui);
    },
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(1), await ui.eval(LAYOUT_BACK), await ui.eval(TYPE_BACK), await ui.sleep(300)), ...s }));

// The model designer (ui/processing/model, docs/specs/model-designer.md): a new model; the built-in model's copy with
// an input, a step and a source list; a wire in the middle of its drag and the menu it opens; a step with problems; a
// chain built by clicking tools; a number input; the tools searched and one carried out of the palette; the unsaved
// question. Every scene closes the designer without saving.
/** The designer on a model (a built-in one opens as its copy) or on a new one. */
async function openDesigner2(ui, id) {
  await ui.eval(`window.kentos.commands.execute('processing.newModel'${id ? `, ${JSON.stringify(id)}` : ''})`);
  await ui.waitFor(`!!document.querySelector('.dialog--designer .mcanvas')`, 15000);
  await ui.sleep(700);
}
/** The designer closed without saving: Esc, and the unsaved question's “Kaydetmeden kapat” when it asks. */
async function closeModelDesigner(ui) {
  for (let i = 0; i < 6 && (await ui.eval(`!!document.querySelector('.dialog')`)); i++) {
    const asked = await ui.eval(`!![...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden/.test(b.textContent))`);
    if (asked) await ui.eval(`[...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden/.test(b.textContent)).click()`);
    else await ui.escapeAll(1);
    await ui.sleep(250);
  }
}
/** A palette entry clicked: an input kind, or a tool by its name. */
const paletteInput = (ui, label) => ui.clickText('.dialog--designer .mpalette__input', label);
const paletteTool = (ui, label) => ui.clickText('.dialog--designer .mpalette__tool', label);
/** A box of the diagram by its reference (an input's name, a step's id). */
const box = (kind, ref) => `.dialog--designer .mnode--${kind}[data-ref="${ref}"]`;
/** A wire from an input's port held over a step, released there when `drop`. */
async function wire(ui, fromRef, toRef, drop) {
  const [x, y] = await ui.eval(centreOf(`${box('input', fromRef)} .mnode__port`));
  const [tx, ty] = await ui.eval(centreOf(box('step', toRef)));
  await ui.move(x, y);
  await ui.pressAt(x, y);
  for (let i = 1; i <= 10; i++) await ui.moveHeld(x + ((tx - x) * i) / 10, y + ((ty - y) * i) / 10);
  await ui.sleep(250);
  if (drop) {
    await ui.releaseAt(tx, ty);
    await ui.waitFor(`!!document.querySelector('.menu')`);
    await ui.sleep(300);
  }
}
/** A model saved with two problems: a start point asked for and not given, and a step reading an input that is gone. */
const PROBLEM_MODEL = {
  id: 'm-shot-problems',
  label: 'Sorunlu model',
  category: 'points',
  description: 'Resim için: iki sorunlu adım.',
  inputs: [{ type: 'features', name: 'parseller', label: 'Parseller', kinds: ['polygon'], default: { scope: 'selection' } }],
  steps: [
    { id: 'numara', tool: 'points.numberVertices', values: { input: { kind: 'input', name: 'parseller' }, start: { kind: 'value', value: 'point' } }, position: { x: 330, y: 40 } },
    { id: 'kenar', tool: 'annotation.edgeLengths', values: { input: { kind: 'input', name: 'yok' } }, position: { x: 330, y: 160 } },
  ],
  outputs: [],
  inputPositions: { parseller: { x: 40, y: 40 } },
};
SCENES.modeldesigner = [
  { id: 'new', open: (ui) => openDesigner2(ui) },
  { id: 'builtin-copy', open: (ui) => openDesigner2(ui, 'builtin.parcelSheet') },
  { id: 'input-selected', open: async (ui) => (await openDesigner2(ui, 'builtin.parcelSheet'), await ui.clickSel(box('input', 'parcels')), await ui.sleep(300)) },
  { id: 'step-selected', open: async (ui) => (await openDesigner2(ui, 'builtin.parcelSheet'), await ui.clickSel(box('step', 'area')), await ui.sleep(300)) },
  {
    id: 'source-menu',
    open: async (ui) => {
      await openDesigner2(ui, 'builtin.parcelSheet');
      await ui.clickSel(box('step', 'corners'));
      await ui.sleep(300);
      await ui.clickSel('.dialog--designer .mins__param .mins__source');
      await ui.waitFor(`!!document.querySelector('.menu')`);
      await ui.sleep(300);
    },
  },
  {
    id: 'wire-drag',
    open: async (ui) => (await openDesigner2(ui, 'builtin.parcelSheet'), await wire(ui, 'prefix', 'area', false)),
    close: async (ui) => (await ui.releaseAt(2, 2), await closeModelDesigner(ui)),
  },
  { id: 'wire-menu', open: async (ui) => (await openDesigner2(ui, 'builtin.parcelSheet'), await wire(ui, 'prefix', 'area', true)) },
  {
    id: 'problems',
    open: async (ui) => (await ui.eval(`window.kentos.processing.saveModel(${JSON.stringify(PROBLEM_MODEL)})`), await openDesigner2(ui, PROBLEM_MODEL.id)),
    close: async (ui) => (await closeModelDesigner(ui), await ui.eval(`window.kentos.processing.removeModel('${PROBLEM_MODEL.id}')`)),
  },
  {
    id: 'problem-step',
    open: async (ui) => {
      await ui.eval(`window.kentos.processing.saveModel(${JSON.stringify(PROBLEM_MODEL)})`);
      await openDesigner2(ui, PROBLEM_MODEL.id);
      await ui.clickSel(box('step', 'numara'));
      await ui.sleep(300);
    },
    close: async (ui) => (await closeModelDesigner(ui), await ui.eval(`window.kentos.processing.removeModel('${PROBLEM_MODEL.id}')`)),
  },
  {
    id: 'new-step',
    open: async (ui) => {
      await openDesigner2(ui);
      await paletteTool(ui, 'Öznitelik hesapla');
      await ui.sleep(400);
    },
  },
  {
    id: 'chain',
    open: async (ui) => {
      await openDesigner2(ui);
      await paletteInput(ui, 'Nesneler');
      await paletteTool(ui, 'Köşe noktalarını numarala');
      await paletteTool(ui, 'Kenar uzunluklarını yaz');
      await ui.clickSel('.dialog--designer .dialog__foot .btn[title]');
      await ui.sleep(500);
    },
  },
  { id: 'number-input', open: async (ui) => (await openDesigner2(ui), await paletteInput(ui, 'Sayı'), await ui.sleep(300)) },
  {
    id: 'palette-search',
    open: async (ui) => {
      await openDesigner2(ui);
      await ui.clickSel('.dialog--designer .mpalette input[type="search"]');
      await ui.type('kenar');
      await ui.sleep(300);
    },
  },
  {
    id: 'palette-carry',
    open: async (ui) => {
      await openDesigner2(ui);
      const [x, y] = await ui.eval(centreOf('.dialog--designer .mpalette__tool'));
      const [tx, ty] = await ui.eval(centreOf('.dialog--designer .mcanvas'));
      await ui.move(x, y);
      await ui.pressAt(x, y);
      for (let i = 1; i <= 10; i++) await ui.moveHeld(x + ((tx - x) * i) / 10, y + ((ty - y) * i) / 10);
      await ui.sleep(250);
    },
    close: async (ui) => (await ui.releaseAt(2, 2), await closeModelDesigner(ui)),
  },
  {
    id: 'unsaved',
    open: async (ui) => {
      await openDesigner2(ui);
      await paletteInput(ui, 'Metin');
      await ui.clickText('.dialog--designer .dialog__foot .btn', 'Kapat');
      await ui.waitFor(`!![...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden/.test(b.textContent))`);
      await ui.sleep(300);
    },
  },
].map((s) => ({ close: (ui) => closeModelDesigner(ui), ...s }));

// The ribbon (ui/ribbon/ribbonPlan.ts, keytips.ts; docs/specs/ribbon.md): its key tips on the tabs and the quick
// access bar, then on Giriş's controls, then narrowed by a typed letter; the bar's menu; a right click on a command
// off the bar, on one added to it, on a fixed one and on a tab; a tool's methods and a family under their split
// buttons; the folded ribbon open over the drawing. Each scene starts from a bar with two added commands and puts
// the layout back after.
const RIBBON_BAR = { ribbonQuickAccess: ['view.zoomExtents', 'tool.line'], ribbonTab: 'home', ribbonCollapsed: false };
/**
 * The open drawing made a project of `type` for a scene: each type has its own ribbon (docs/adr/0165 §6), a CAD
 * project's Değiştir and Açıklama, a CBS project's Harita and Düzenle. The demo drawing is not asked its type (shown
 * as CBS); TYPE_BACK puts its type back after the scene.
 */
const typeSet = (type) => `(() => { const w = window.kentos.doc.settings.workspace; if (!('__shotType' in window)) window.__shotType = w.value; w.set(${JSON.stringify(type)}); })()`;
const TYPE_BACK = `(() => { if ('__shotType' in window) { window.kentos.doc.settings.workspace.set(window.__shotType); delete window.__shotType; } })()`;
async function ribbonOn(ui, { type, ...fields } = {}) {
  if (type) await ui.eval(typeSet(type));
  await ui.eval(layoutSet({ ...RIBBON_BAR, ...fields }));
  await ribbonSettled(ui);
  // The live ribbon reads its kept tab once, when it is built: a scene's own tab is clicked open, as a user opens it.
  if (fields.ribbonTab && !fields.ribbonCollapsed) {
    await ui.clickSel(`.ribbon__tab[data-tab="${fields.ribbonTab}"]`);
    await ribbonSettled(ui);
  }
}
async function ribbonOff(ui) {
  await ui.escapeAll(3);
  await ui.eval(LAYOUT_BACK);
  await ui.eval(TYPE_BACK);
  await ui.sleep(200);
}
/** The key tips shown (F6), then the letters typed one by one. */
async function keyTips(ui, letters = '') {
  await ui.eval(`window.kentos.commands.execute('view.keyTips')`);
  await ui.waitFor(`!!document.querySelector('.keytips__tip')`);
  for (const ch of letters) {
    await ui.key(ch);
    await ui.sleep(250);
  }
  await ui.sleep(400);
}
/** A right click on an element of the ribbon, and the menu it opens. */
async function rightClick(ui, sel) {
  await ui.contextClick(...(await ui.eval(centreOf(sel))));
  await ui.waitFor(`!!document.querySelector('.menu')`);
  await ui.sleep(300);
}
SCENES.ribbon = [
  { id: 'keytips-tabs', open: async (ui) => (await ribbonOn(ui), await keyTips(ui)) },
  { id: 'keytips-controls', open: async (ui) => (await ribbonOn(ui), await keyTips(ui, 'gi')) },
  { id: 'keytips-typed', open: async (ui) => (await ribbonOn(ui), await keyTips(ui, 'gik')) },
  { id: 'bar-menu', open: async (ui) => (await ribbonOn(ui), await ui.clickSel('.ribbon__qat-more'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)) },
  { id: 'menu-add', open: async (ui) => (await ribbonOn(ui), await rightClick(ui, '.ribbon__strip [data-command="tool.polyline"]')) },
  { id: 'menu-remove', open: async (ui) => (await ribbonOn(ui), await rightClick(ui, '.ribbon__qat [data-command="tool.line"]')) },
  { id: 'menu-fixed', open: async (ui) => (await ribbonOn(ui), await rightClick(ui, '.ribbon__qat [data-command="file.save"]')) },
  { id: 'menu-elsewhere', open: async (ui) => (await ribbonOn(ui), await rightClick(ui, '.ribbon__tab[data-tab="map"]')) },
  {
    id: 'split-methods',
    open: async (ui) => (await ribbonOn(ui), await ui.clickSel('.ribbon__strip [data-split="circle"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
  },
  {
    id: 'split-family',
    open: async (ui) => (await ribbonOn(ui, { type: 'cad' }), await ui.clickSel('.ribbon__strip [data-split="rectangle"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
  },
  {
    id: 'folded-open',
    open: async (ui) => (await ribbonOn(ui, { ribbonCollapsed: true }), await ui.clickSel('.ribbon__tab[data-tab="home"]'), await ui.sleep(500)),
  },
  // Katmanlar's ▾: the layer actions by an object (docs/adr/0177 §7); the desktop's icon tour “katman-araclari”.
  {
    id: 'layers-more',
    open: async (ui) => {
      await ribbonOn(ui);
      // A narrow window folds the panel: its ▾ is under the controls of its pop-up.
      const folded = await ui.eval(`document.querySelector('.rpanel[data-panel="Katmanlar"]')?.dataset.level === '3'`);
      if (folded) {
        await ui.clickSel('.rpanel[data-panel="Katmanlar"] .rpanel__collapsed');
        await ui.sleep(300);
      }
      await ui.clickSel(folded ? '.ribbon-pop .rpanel__more' : '.rpanel[data-panel="Katmanlar"] .rpanel__more');
      await ui.waitFor(`!!document.querySelector('.menu')`);
      await ui.sleep(300);
    },
  },
].map((s) => ({ close: (ui) => ribbonOff(ui), ...s }));

// The new tools at work (docs/adr/0140): each one is started from the ribbon's command, its value typed in the command
// line as a user does, and a picture taken with the preview showing and another after Enter. Objects the scene needs
// are added on bare ground next to the demo drawing (sized by the view's width) and taken back after; the demo
// drawing's own parcels serve where a parcel will do.
const UNDO_ALL = `(() => { const k = window.kentos; k.tools.activate('select'); while (k.doc.canUndo.value) k.doc.undo(); k.selection.clear(); k.view.zoomExtents(); })()`;
/** The page point of a world point. */
const PAGE_AT = (x, y) => `(() => { const k = window.kentos; const r = k.view.clientRect(); const s = k.view.camera.worldToScreen({ x: ${x}, y: ${y} }); return [Math.round(s.x + r.left), Math.round(s.y + r.top)]; })()`;
/** `body` run with the view's middle `c` and a sixth of its width `w`; `add` puts an object on the active layer. */
const SCRATCH = (body) => `(() => {
  const k = window.kentos;
  const b = k.view.camera.visibleBounds();
  const c = { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 };
  const w = (b.maxX - b.minX) / 6;
  const add = (e) => k.doc.add({ layerId: k.doc.layers.active.value, attrs: {}, ...e });
  ${body}
})()`;
/** The view over bare ground next to the drawing (about 120 m wide), for the scenes that draw their own objects. */
const CLEAR_VIEW = `(() => {
  const k = window.kentos;
  const e = k.view.extent();
  const r = k.view.clientRect();
  const w = 120;
  const h = (w * r.height) / r.width;
  k.selection.clear();
  k.view.camera.fit({ minX: e.maxX + 400, minY: e.minY, maxX: e.maxX + 400 + w, maxY: e.minY + h }, 0);
  k.view.requestRender();
})()`;
const runTool = (id) => `window.kentos.commands.execute('tool.${id}')`;
/** The mouse over the drawing at a world point, as a user reaches for the next click. */
async function hoverAt(ui, x, y) {
  await ui.move(...(await ui.eval(PAGE_AT(x, y))));
  await ui.sleep(350);
}
/** Six neighbouring parcels of the demo drawing (its “Parsel sınırı” layer) selected and filled in the view. */
const SIX_PARCELS = `(() => {
  const k = window.kentos;
  const all = [...k.doc.all()].filter((e) => e.kind === 'polygon' && k.doc.layers.get(e.layerId)?.name === 'Parsel sınırı');
  const mid = (e) => ({ x: e.pts.reduce((a, p) => a + p.x, 0) / e.pts.length, y: e.pts.reduce((a, p) => a + p.y, 0) / e.pts.length });
  const o = mid(all[0]);
  all.sort((a, b) => Math.hypot(mid(a).x - o.x, mid(a).y - o.y) - Math.hypot(mid(b).x - o.x, mid(b).y - o.y));
  k.selection.set(all.slice(0, 6).map((e) => e.id));
})()`;
async function parcelsInView(ui) {
  await ui.eval(SIX_PARCELS);
  await ui.eval(`window.kentos.view.zoomToSelection()`);
  await ui.sleep(500);
}
async function startTool(ui, id) {
  await ui.eval(runTool(id));
  await ui.sleep(300);
}
/** A line typed into the command line (the value a tool asks for). */
async function typeValue(ui, text) {
  await typeLine(ui, text);
  await ui.sleep(300);
}
/** Enter in the emptied command line: what a user presses to confirm. */
async function pressEnter(ui) {
  await ui.clickSel('.cmdline__input');
  await ui.eval(CMDLINE_EMPTY);
  await ui.key('Enter');
  await ui.sleep(500);
}
/** The middle of the view, in world coordinates, for the mouse. */
const VIEW_MID = `(() => { const k = window.kentos; const b = k.view.camera.visibleBounds(); return [(b.minX + b.maxX) / 2, (b.minY + b.maxY) / 2]; })()`;
async function hoverMid(ui) {
  await hoverAt(ui, ...(await ui.eval(VIEW_MID)));
}

function toolScenes() {
  const ribbon = (ui) => ribbonOn(ui, { ribbonTab: 'edit' });
  const lastOf = (kind) => `[...window.kentos.doc.all()].filter((e) => e.kind === '${kind}').slice(-1).map((e) => e.id)`;
  /** A wiggly open polyline of many vertices along a gentle curve, for Sadeleştir and Yönü çevir. */
  const WIGGLE = SCRATCH(`
    const pts = [];
    for (let i = 0; i <= 48; i++) {
      const t = i / 48;
      pts.push({ x: c.x - 2.2 * w + 4.4 * w * t, y: c.y + 0.7 * w * Math.sin(t * Math.PI * 1.5) + 0.02 * w * Math.sin(i * 7.3) });
    }
    const e = add({ kind: 'polyline', pts, color: '#E5484D' });
    k.selection.set([e.id]);`);
  const WIGGLE_TOL = SCRATCH(`return (0.05 * w).toFixed(2);`);
  /** Three lines crossing each other, selected. */
  const CROSSING = SCRATCH(`
    const a = add({ kind: 'line', a: { x: c.x - 2 * w, y: c.y }, b: { x: c.x + 2 * w, y: c.y } });
    const d = add({ kind: 'line', a: { x: c.x - 1.4 * w, y: c.y - 1.2 * w }, b: { x: c.x + 1.6 * w, y: c.y + 1.3 * w } });
    const v = add({ kind: 'line', a: { x: c.x + 0.5 * w, y: c.y - 1.4 * w }, b: { x: c.x + 0.5 * w, y: c.y + 1.4 * w } });
    k.selection.set([a.id, d.id, v.id]);`);
  /** A long line, alone; the point on it a click picks it at. */
  const ONE_LINE = SCRATCH(`add({ kind: 'line', a: { x: c.x - 2.2 * w, y: c.y }, b: { x: c.x + 2.2 * w, y: c.y + 0.5 * w } });`);
  const ON_LINE = SCRATCH(`return [c.x, c.y + 0.25 * w];`);
  /** A bent polyline; the point near its last end, and a piece length that leaves a short last piece. */
  const BENT = SCRATCH(`add({ kind: 'polyline', pts: [{ x: c.x - 2.2 * w, y: c.y - 0.6 * w }, { x: c.x + 0.6 * w, y: c.y - 0.6 * w }, { x: c.x + 2 * w, y: c.y + 0.9 * w }] });`);
  const NEAR_BENT_END = SCRATCH(`return [c.x + 2 * w - 0.06 * w, c.y + 0.9 * w - 0.06 * w * 1.5 / 1.4];`);
  const BENT_LENGTH = SCRATCH(`return (((0.6 + 2.2) * w + Math.hypot(1.4 * w, 1.5 * w)) / 5.5).toFixed(2);`);
  /** Drawn twice, empty, and with repeated vertices: what Çizimi temizle finds. */
  const MESSY = SCRATCH(`
    add({ kind: 'line', a: { x: c.x - 2 * w, y: c.y - w }, b: { x: c.x, y: c.y - w } });
    add({ kind: 'line', a: { x: c.x - 2 * w, y: c.y - w }, b: { x: c.x, y: c.y - w } });
    add({ kind: 'polygon', pts: [{ x: c.x + w, y: c.y - w }, { x: c.x + 2 * w, y: c.y - w }, { x: c.x + 2 * w, y: c.y - w }, { x: c.x + 2 * w, y: c.y }, { x: c.x + w, y: c.y }, { x: c.x + w, y: c.y }] });
    add({ kind: 'line', a: { x: c.x - w, y: c.y + w }, b: { x: c.x - w, y: c.y + w } });
    add({ kind: 'line', a: { x: c.x, y: c.y + 0.3 * w }, b: { x: c.x + 0.8 * w, y: c.y + 0.3 * w } });
    add({ kind: 'line', a: { x: c.x, y: c.y + 0.3 * w }, b: { x: c.x + 0.8 * w, y: c.y + 0.3 * w } });
    k.selection.set([...k.doc.all()].slice(-6).map((e) => e.id));`);
  /** A source with its own colour and weight, and plain objects to give them to; the points a click picks them at. */
  const MATCH = SCRATCH(`
    const sq = (x0, y0, x1, y1, o) => add({ kind: 'polygon', pts: [{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }], ...o });
    sq(c.x - 2 * w, c.y + 0.2 * w, c.x - 0.8 * w, c.y + 1.2 * w, { color: '#E5484D', lineWeight: 0.7 });
    sq(c.x - 0.4 * w, c.y + 0.2 * w, c.x + 0.8 * w, c.y + 1.2 * w, { color: '#3B82F6', lineWeight: 0.13 });
    add({ kind: 'circle', c: { x: c.x + 1.6 * w, y: c.y + 0.7 * w }, r: 0.5 * w, color: '#3B82F6', lineWeight: 0.13 });
    k.selection.clear();`);
  const MATCH_SOURCE = SCRATCH(`return [c.x - 1.4 * w, c.y + 0.2 * w];`);
  const MATCH_TARGET = SCRATCH(`return [c.x + 0.2 * w, c.y + 0.2 * w];`);
  const MATCH_TARGET_2 = SCRATCH(`return [c.x + 2.1 * w, c.y + 0.7 * w];`);
  const BELOW = SCRATCH(`return [c.x, c.y - 1.5 * w];`);
  const bare = async (ui, scratch, fields = {}) => {
    await ribbonOn(ui, { ribbonTab: 'edit', ...fields });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(scratch);
  };
  const clickWorld = async (ui, expr) => ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(expr))))));
  const splitOn = async (ui, scratch, option, point, value) => {
    await bare(ui, scratch);
    await startTool(ui, 'split');
    await ui.eval(`window.kentos.tools.active.input('${option}')`);
    await clickWorld(ui, point);
    await ui.sleep(300);
    await typeValue(ui, typeof value === 'function' ? await value() : value);
  };
  const matchStart = async (ui) => {
    await bare(ui, MATCH);
    await startTool(ui, 'matchProperties');
    await clickWorld(ui, MATCH_SOURCE);
    await ui.sleep(300);
  };
  return [
    // Tüm köşeleri yuvarla and Tüm köşelere pah: two parcels, the value typed.
    { id: 'filletall-preview', open: async (ui) => (await ribbon(ui), await parcelsInView(ui), await startTool(ui, 'filletAll'), await typeValue(ui, '4'), await hoverMid(ui)) },
    { id: 'filletall-result', open: async (ui) => (await ribbon(ui), await parcelsInView(ui), await startTool(ui, 'filletAll'), await typeValue(ui, '4'), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'chamferall-preview', open: async (ui) => (await ribbon(ui), await parcelsInView(ui), await startTool(ui, 'chamferAll'), await typeValue(ui, '4,6'), await hoverMid(ui)) },
    { id: 'chamferall-result', open: async (ui) => (await ribbon(ui), await parcelsInView(ui), await startTool(ui, 'chamferAll'), await typeValue(ui, '4,6'), await pressEnter(ui), await ui.move(2, 2)) },
    // Parçala: from the crossings, in equal parts, by length from an end.
    { id: 'split-crossings-preview', open: async (ui) => (await bare(ui, CROSSING), await startTool(ui, 'split'), await hoverMid(ui)) },
    { id: 'split-crossings-result', open: async (ui) => (await bare(ui, CROSSING), await startTool(ui, 'split'), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'split-equal-preview', open: async (ui) => (await splitOn(ui, ONE_LINE, 'E', ON_LINE, '5'), await hoverMid(ui)) },
    { id: 'split-equal-result', open: async (ui) => (await splitOn(ui, ONE_LINE, 'E', ON_LINE, '5'), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'split-length-preview', open: async (ui) => (await splitOn(ui, BENT, 'U', NEAR_BENT_END, () => ui.eval(BENT_LENGTH)), await hoverMid(ui)) },
    { id: 'split-length-result', open: async (ui) => (await splitOn(ui, BENT, 'U', NEAR_BENT_END, () => ui.eval(BENT_LENGTH)), await pressEnter(ui), await ui.move(2, 2)) },
    // Yönü çevir: the arrows show the direction the object will have; picked and confirmed, or selected first.
    {
      id: 'reverse-preview',
      open: async (ui) => {
        await bare(ui, WIGGLE);
        await ui.eval(`window.kentos.selection.clear()`);
        await startTool(ui, 'reverse');
        await ui.eval(`window.kentos.selection.set(${lastOf('polyline')})`);
        await hoverMid(ui);
      },
    },
    {
      id: 'reverse-result',
      open: async (ui) => {
        await bare(ui, WIGGLE);
        // Selected first: the tool turns it at once and says so; the direction itself shows only in the arrows of the picture before.
        await startTool(ui, 'reverse');
        await ui.sleep(400);
        await ui.move(2, 2);
      },
    },
    // Sadeleştir: the tolerance typed; the vertices that go are struck.
    { id: 'simplify-preview', open: async (ui) => (await bare(ui, WIGGLE), await startTool(ui, 'simplify'), await typeValue(ui, await ui.eval(WIGGLE_TOL)), await hoverMid(ui)) },
    { id: 'simplify-result', open: async (ui) => (await bare(ui, WIGGLE), await startTool(ui, 'simplify'), await typeValue(ui, await ui.eval(WIGGLE_TOL)), await pressEnter(ui), await ui.move(2, 2)) },
    // Çizimi temizle: the finding first, then the cleaned drawing.
    { id: 'cleanup-preview', open: async (ui) => (await bare(ui, MESSY), await startTool(ui, 'cleanup'), await hoverMid(ui)) },
    { id: 'cleanup-result', open: async (ui) => (await bare(ui, MESSY), await startTool(ui, 'cleanup'), await pressEnter(ui), await ui.move(2, 2)) },
    // Özellik kopyala: the source clicked and a target under the mouse; then two targets taken one after the other.
    { id: 'match-preview', open: async (ui) => (await matchStart(ui), await hoverAt(ui, ...(await ui.eval(MATCH_TARGET)))) },
    {
      id: 'match-result',
      open: async (ui) => {
        await matchStart(ui);
        for (const t of [MATCH_TARGET, MATCH_TARGET_2]) {
          await clickWorld(ui, t);
          await ui.sleep(300);
        }
        await hoverAt(ui, ...(await ui.eval(BELOW)));
      },
    },
    ...faz2Scenes(bare, clickWorld),
    ...faz3Scenes(bare, clickWorld),
    ...adr0141Scenes(bare, clickWorld),
    ...adr0142Scenes(bare, clickWorld),
  ];
}

/** A world point as fractions of the view's sixth-width unit around its middle (what `SCRATCH` sets up). */
const AT = (fx, fy) => SCRATCH(`return [c.x + ${fx} * w, c.y + ${fy} * w];`);
/** Clicks at each of the points, one after the other. */
async function clickAll(ui, clickWorld, ...pts) {
  for (const [fx, fy] of pts) {
    await clickWorld(ui, AT(fx, fy));
    await ui.sleep(250);
  }
}
const hoverFrac = async (ui, fx, fy) => hoverAt(ui, ...(await ui.eval(AT(fx, fy))));
/** The bottom panel's history open, tall enough for the lines a tool says (set with the ribbon's layout, put back by ribbonOff). */
const LOGGED = { bottomExpanded: true, bottomTab: 'history', bottomHeight: 190 };

// Faz 2 (docs/adr/0140): Daire dilimi, Ara nokta, Kesişim noktası, Açı ölç, Koordinat oku, Zincir ölçü, Baz ölçü.
function faz2Scenes(bare, clickWorld) {
  const none = SCRATCH('return 0;');
  const inputOption = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  /** The tool started on bare ground, with its option typed; `logged`: the log open under the drawing. */
  const on = async (ui, id, ...options) => {
    await bare(ui, none);
    await startTool(ui, id);
    for (const o of options) await inputOption(ui, o);
  };
  const onLogged = async (ui, id) => {
    await bare(ui, none, LOGGED);
    await ui.eval(`window.kentos.log.clear()`);
    await startTool(ui, id);
  };
  // Daire dilimi: a centre, the start point that sets the radius and the start angle, the end direction under the mouse.
  const sector = async (ui) => {
    await on(ui, 'sector');
    await clickAll(ui, clickWorld, [-0.8, -1.1], [0.8, -1.1]);
  };
  // A line drawn from two clicks, the mouse at the second: Ara nokta at its kept 4 parts.
  const between = async (ui, ...options) => {
    await on(ui, 'pointsBetween', ...options);
    await clickAll(ui, clickWorld, [-2.2, -0.6], [2.2, 0.8]);
  };
  const twoCircles = async (ui, logged = false) => {
    await (logged ? onLogged : on)(ui, 'intersectPoint');
    await clickWorld(ui, AT(-1.2, -0.3));
    await typeValue(ui, '30');
    await clickWorld(ui, AT(1.2, -0.3));
    await typeValue(ui, '30');
  };
  const bearings = async (ui) => {
    await on(ui, 'intersectPoint', 'D');
    await clickWorld(ui, AT(-1.6, -1));
    await typeValue(ui, '50');
    await clickWorld(ui, AT(1.6, -1));
  };
  const threeLinePoints = async (ui) => {
    await on(ui, 'intersectPoint', 'L');
    await clickAll(ui, clickWorld, [-2, -1], [-0.5, -0.4], [0.5, -1.2]);
  };
  const angleStart = async (ui) => {
    await on(ui, 'measureAngle');
    await clickAll(ui, clickWorld, [-0.5, -0.8], [1.8, -0.8]);
  };
  /** An aligned dimension drawn with Ölçülendirme (the base the chain tools remember), then a chain tool started. */
  const dimensionsFrom = async (ui, id) => {
    await on(ui, 'dimension');
    await clickAll(ui, clickWorld, [-2.4, -1.1], [-0.9, -1.1], [-1.6, -0.5]);
    await startTool(ui, id);
  };
  return [
    { id: 'sector-preview', open: async (ui) => (await sector(ui), await hoverFrac(ui, 0, 0.64)) },
    { id: 'sector-result', open: async (ui) => (await sector(ui), await clickWorld(ui, AT(0, 0.64)), await ui.move(2, 2)) },
    { id: 'between-preview', open: async (ui) => (await between(ui), await hoverFrac(ui, 2.5, 1.1)) },
    { id: 'between-result', open: async (ui) => (await between(ui), await typeValue(ui, '5'), await hoverFrac(ui, 0, -1.4)) },
    {
      id: 'between-distances-result',
      open: async (ui) => (await between(ui, 'U'), await typeValue(ui, '15, 45, 80'), await hoverFrac(ui, 0, -1.4)),
    },
    {
      id: 'between-ratios-preview',
      open: async (ui) => {
        await between(ui, 'O');
        await typeValue(ui, '0.1, 0.5, 0.9');
        // The next two points, the kept ratios shown on them.
        await clickAll(ui, clickWorld, [-2, -1.4], [2, -1.2]);
        await hoverFrac(ui, 2.4, -0.9);
      },
    },
    { id: 'meeting-distances-preview', open: async (ui) => (await twoCircles(ui), await hoverFrac(ui, 0.1, 0.8)) },
    { id: 'meeting-distances-result', open: async (ui) => (await twoCircles(ui, true), await clickWorld(ui, AT(0.1, 0.8)), await ui.move(2, 2)) },
    { id: 'meeting-bearings-preview', open: async (ui) => (await bearings(ui), await hoverFrac(ui, 0.89, -0.29)) },
    { id: 'meeting-bearings-result', open: async (ui) => (await bearings(ui), await typeValue(ui, '350'), await ui.move(2, 2)) },
    { id: 'meeting-lines-preview', open: async (ui) => (await threeLinePoints(ui), await hoverFrac(ui, 1.7, 0)) },
    { id: 'meeting-lines-result', open: async (ui) => (await threeLinePoints(ui), await clickWorld(ui, AT(1.7, 0)), await ui.move(2, 2)) },
    { id: 'angle-preview', open: async (ui) => (await angleStart(ui), await hoverFrac(ui, 0.8, 0.9)) },
    {
      id: 'angle-result',
      open: async (ui) => {
        await onLogged(ui, 'measureAngle');
        await clickAll(ui, clickWorld, [-0.5, -0.8], [1.8, -0.8], [0.8, 0.7]);
        await ui.move(2, 2);
      },
    },
    {
      id: 'coord-read',
      open: async (ui) => {
        await bare(ui, SCRATCH(`add({ kind: 'point', p: { x: c.x + 0.8 * w, y: c.y + 0.4 * w }, z: 118.5 }); add({ kind: 'line', a: { x: c.x - 2 * w, y: c.y - 0.6 * w }, b: { x: c.x + 2 * w, y: c.y - 0.2 * w } });`), LOGGED);
        await ui.eval(`window.kentos.log.clear()`);
        await ui.eval(`window.kentos.commands.execute('crs.query')`);
        await ui.sleep(300);
        await clickWorld(ui, AT(-0.6, 0.6));
        await ui.sleep(300);
        // The spot with its height, the mouse on it so the snap takes it.
        await clickWorld(ui, AT(0.8, 0.4));
        await ui.sleep(300);
      },
    },
    { id: 'dimcontinue-preview', open: async (ui) => (await dimensionsFrom(ui, 'dimContinue'), await clickWorld(ui, AT(0.2, -1.3)), await hoverFrac(ui, 1.5, -1.0)) },
    {
      id: 'dimcontinue-result',
      open: async (ui) => (await dimensionsFrom(ui, 'dimContinue'), await clickAll(ui, clickWorld, [0.2, -1.3], [1.5, -1.0], [2.6, -1.2]), await ui.move(2, 2)),
    },
    { id: 'dimbaseline-preview', open: async (ui) => (await dimensionsFrom(ui, 'dimBaseline'), await clickWorld(ui, AT(0.2, -1.3)), await hoverFrac(ui, 1.5, -1.0)) },
    {
      id: 'dimbaseline-result',
      open: async (ui) => (await dimensionsFrom(ui, 'dimBaseline'), await clickAll(ui, clickWorld, [0.2, -1.3], [1.5, -1.0], [2.6, -1.2]), await ui.move(2, 2)),
    },
  ];
}

// Faz 3: the Çit method of Buda and Uzat, Ötele's İki yana and Kaynağı sil, Yol boyunca dizi.
function faz3Scenes(bare, clickWorld) {
  const inputOption = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  /** Two vertical boundaries and four lines across them (all the way, for Buda). */
  const ACROSS = SCRATCH(`
    for (const x of [-0.6, 0.6]) add({ kind: 'line', a: { x: c.x + x * w, y: c.y - 1.5 * w }, b: { x: c.x + x * w, y: c.y + 1.5 * w } });
    for (const y of [-0.9, -0.3, 0.3, 0.9]) add({ kind: 'line', a: { x: c.x - 2.4 * w, y: c.y + y * w }, b: { x: c.x + 2.4 * w, y: c.y + y * w }, color: '#E5484D' });`);
  /** A boundary at the east and four lines that stop short of it, at different lengths. */
  const SHORT = SCRATCH(`
    add({ kind: 'line', a: { x: c.x + 1.9 * w, y: c.y - 1.5 * w }, b: { x: c.x + 1.9 * w, y: c.y + 1.5 * w } });
    [-0.9, -0.3, 0.3, 0.9].forEach((y, i) => add({ kind: 'line', a: { x: c.x - 2.4 * w, y: c.y + y * w }, b: { x: c.x + (0.9 + 0.25 * i) * w, y: c.y + y * w }, color: '#E5484D' }));`);
  const fence = async (ui, id, scratch) => {
    await bare(ui, scratch);
    await startTool(ui, id);
    await inputOption(ui, 'C');
    await ui.sleep(250);
  };
  const fenceOn = async (ui, id, scratch, x) => {
    await fence(ui, id, scratch);
    await clickAll(ui, clickWorld, [x + 0.1, -1.35], [x - 0.15, -0.4], [x + 0.1, 0.55], [x, 1.35]);
  };
  const BENT_LINE = SCRATCH(`add({ kind: 'polyline', pts: [{ x: c.x - 2 * w, y: c.y - 0.9 * w }, { x: c.x, y: c.y - 0.9 * w }, { x: c.x + 0.8 * w, y: c.y + 0.6 * w }, { x: c.x + 2 * w, y: c.y + 0.6 * w }], color: '#E5484D' });`);
  /** Ötele's options set to exactly these (they are kept for the session, so the last scene's are still on). */
  const offsetOptions = async (ui, want) => {
    for (const [key, label] of [['N', 'Noktadan geç'], ['I', 'İki yana'], ['S', 'Kaynağı sil']]) {
      const on = await ui.eval(`window.kentos.tools.active.prompt.value.includes('${label} (${key}): açık')`);
      if (on !== want.includes(key)) await inputOption(ui, key);
    }
  };
  const offsetOn = async (ui, ...options) => {
    await bare(ui, BENT_LINE);
    await startTool(ui, 'offset');
    await offsetOptions(ui, options);
    await typeValue(ui, '6');
    await clickWorld(ui, AT(-1, -0.9));
    await ui.sleep(250);
  };
  /** A square to copy at the start of an arc it will follow. */
  const PATH_ARRAY = SCRATCH(`
    add({ kind: 'arc', c: { x: c.x, y: c.y - 2 * w }, r: 2.9 * w, a0: 0.55, a1: 2.6, color: '#3B82F6' });
    const at = { x: c.x + 2.9 * w * Math.cos(0.55), y: c.y - 2 * w + 2.9 * w * Math.sin(0.55) };
    const s = 0.18 * w;
    const sq = add({ kind: 'polygon', pts: [{ x: at.x - s, y: at.y - s * 1.6 }, { x: at.x + s, y: at.y - s * 1.6 }, { x: at.x + s, y: at.y + s * 1.6 }, { x: at.x - s, y: at.y + s * 1.6 }], color: '#E5484D' });
    k.selection.set([sq.id]);
    return [c.x + 2.9 * Math.cos(1.6) * w, c.y - 2 * w + 2.9 * w * Math.sin(1.6)];`);
  const pathArray = async (ui) => {
    await bare(ui, PATH_ARRAY);
    await startTool(ui, 'arrayPath');
    // The top of the arc.
    await clickWorld(ui, SCRATCH(`return [c.x + 2.9 * Math.cos(1.6) * w, c.y - 2 * w + 2.9 * w * Math.sin(1.6)];`));
    await ui.sleep(250);
    await typeValue(ui, '7');
  };
  return [
    { id: 'trim-fence-preview', open: async (ui) => (await fenceOn(ui, 'trim', ACROSS, 0), await hoverFrac(ui, 0.05, 1.4)) },
    { id: 'trim-fence-result', open: async (ui) => (await fenceOn(ui, 'trim', ACROSS, 0), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'trim-fence-empty', open: async (ui) => (await fence(ui, 'trim', ACROSS), await hoverFrac(ui, 0, 0)) },
    { id: 'extend-fence-preview', open: async (ui) => (await fenceOn(ui, 'extend', SHORT, 0.5), await hoverFrac(ui, 0.55, 1.4)) },
    { id: 'extend-fence-result', open: async (ui) => (await fenceOn(ui, 'extend', SHORT, 0.5), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'offset-both-preview', open: async (ui) => (await offsetOn(ui, 'I'), await hoverFrac(ui, -1, -0.4)) },
    { id: 'offset-both-result', open: async (ui) => (await offsetOn(ui, 'I'), await clickWorld(ui, AT(-1, -0.4)), await ui.move(2, 2)) },
    { id: 'offset-erase-preview', open: async (ui) => (await offsetOn(ui, 'S'), await hoverFrac(ui, -1, -0.4)) },
    { id: 'offset-erase-result', open: async (ui) => (await offsetOn(ui, 'I', 'S'), await clickWorld(ui, AT(-1, -0.4)), await ui.move(2, 2)) },
    { id: 'arraypath-preview', open: async (ui) => (await pathArray(ui), await hoverFrac(ui, 0, -0.2)) },
    { id: 'arraypath-result', open: async (ui) => (await pathArray(ui), await pressEnter(ui), await ui.move(2, 2)) },
    // Dizi and Ölçülendirme are a CAD project's (a CBS project leaves them out of its ribbon).
    ...[
      ['split-trim-methods', 'edit', 'trim'],
      ['split-array', 'modify', 'array', 'cad'],
      ['split-dimension', 'annotate', 'dimension', 'cad'],
      ['split-points-between', 'edit', 'pointsBetween'],
      ['split-intersect-point', 'edit', 'intersectPoint'],
    ].map(([id, tab, key, type]) => ({
      id,
      open: async (ui) => (
        await ribbonOn(ui, { ribbonTab: tab, type }),
        await ui.clickSel(`.ribbon__strip [data-split="${key}"] .rsplit__arrow`),
        await ui.waitFor(`!!document.querySelector('.menu')`),
        await ui.sleep(300)
      ),
    })),
  ];
}

// docs/adr/0141: Görünüm › Yakınlaştır with the view history and Kapsam denetimi, the layer tree's Katmana
// yakınlaştır, the query tools (Mesafe ölç's fixed first point, Alan hesapla's İçine tıkla and Alan olarak çiz, Dik
// ayak ölç) and the selection tools. Scenes are added below as the phases are built.
function adr0141Scenes(bare, clickWorld) {
  /** The view history's two buttons both on: zoomed in twice, then one step back. */
  const HISTORY = `(() => { const k = window.kentos; k.view.navigation.history.clear(); k.view.zoomBy(1.5); k.view.zoomBy(1.5); k.view.viewBack(); })()`;
  /** A square of 60 m at the coordinate origin, where a drawing brought in with the wrong system lands. */
  const STRAY = `(() => {
    const k = window.kentos;
    return k.doc.add({ kind: 'polygon', layerId: k.doc.layers.active.value, attrs: {}, color: '#E5484D', pts: [{ x: 0, y: 0 }, { x: 60, y: 0 }, { x: 60, y: 60 }, { x: 0, y: 60 }] }).id;
  })()`;
  /** The first layer row of the tree with objects on it (a layer, not a group). */
  const LAYER_ROW = `(() => {
    const rows = [...document.querySelectorAll('.panel--layers .tree__row:not([data-group])')];
    const r = rows.find((e) => Number(e.querySelector('.tree__count')?.textContent.replace(/\D/g, '')) > 0);
    r.scrollIntoView({ block: 'nearest' });
    const b = r.getBoundingClientRect();
    return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)];
  })()`;
  const GROUP_ROW = `(() => {
    const r = document.querySelector('.panel--layers .tree__row[data-group]');
    const b = r.getBoundingClientRect();
    return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)];
  })()`;
  /** The demo sheet in view, as the drawing opens (the scenes end with Tümünü göster, which shows the symbol catalogue below it too). */
  const HOME = `(() => { const k = window.kentos; k.view.camera.fit(k.doc.homeView); k.log.clear(); })()`;
  const menuOn = async (ui, at) => {
    await ui.contextClick(...(await ui.eval(at)));
    await ui.waitFor(`!!document.querySelector('.menu')`);
    await ui.sleep(300);
  };
  return [
    { id: 'ribbon-view', open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'view' }), await ui.eval(HOME), await ui.eval(HISTORY), await ui.sleep(400)) },
    {
      id: 'extentcheck-result',
      open: async (ui) => {
        await ribbonOn(ui, { ribbonTab: 'view', ...LOGGED });
        const id = await ui.eval(STRAY);
        await ui.eval(`window.kentos.log.clear()`);
        await ui.eval(`window.kentos.commands.execute('view.extentCheck')`);
        await ui.sleep(300);
        // The selected stray alone in the view, as “Seçime yakınlaştır” would show it.
        await ui.eval(`(() => { const k = window.kentos; if (!k.selection.has(${id})) throw new Error('seçilmedi'); k.view.zoomToSelection(); })()`);
        await ui.sleep(500);
      },
    },
    {
      id: 'extentcheck-none',
      // The panel opens under the drawing first (the view is resized), then the sheet is put in view.
      open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'view', ...LOGGED }), await ui.sleep(400), await ui.eval(HOME), await ui.eval(`window.kentos.log.clear()`), await ui.eval(`window.kentos.commands.execute('view.extentCheck')`), await ui.sleep(400)),
    },
    { id: 'layers-zoom-menu', open: async (ui) => (await ribbonOn(ui), await ui.eval(HOME), await menuOn(ui, LAYER_ROW)) },
    { id: 'layers-zoom-group-menu', open: async (ui) => (await ribbonOn(ui), await ui.eval(HOME), await menuOn(ui, GROUP_ROW)) },
    ...queryScenes(bare, clickWorld),
    ...selectionScenes(bare, clickWorld),
  ];
}

// The scenes of docs/adr/0141 draw in `u`: a sixth of the view's width, or a 3.4th of its height where the log takes the
// height (1100×650), so what they draw fits both. `FIT` is `SCRATCH` with `u`, `AU` a point in fractions of it.
const FIT = (body) => SCRATCH(`const u = Math.min(w, (b.maxY - b.minY) / 3.4); ${body}`);
const AU = (fx, fy) => FIT(`return [c.x + ${fx} * u, c.y + ${fy} * u];`);
async function clickAllU(ui, clickWorld, ...pts) {
  for (const [fx, fy] of pts) {
    await clickWorld(ui, AU(fx, fy));
    await ui.sleep(250);
  }
}
const hoverU = async (ui, fx, fy) => hoverAt(ui, ...(await ui.eval(AU(fx, fy))));

// Faz 3: the selection tools, each on bare ground with the history open under the drawing.
function selectionScenes(bare, clickWorld) {
  const none = FIT('return 0;');
  const inputOption = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  const started = async (ui, id, scratch = none, fields = {}) => {
    await bare(ui, scratch, { ribbonTab: 'home', ...LOGGED, ...fields });
    await ui.eval(`window.kentos.log.clear()`);
    await startTool(ui, id);
  };
  /** Kesişen set to exactly this (it is kept for the session). */
  const crossing = async (ui, want) => {
    const on = await ui.eval(`window.kentos.tools.active.prompt.value.includes('Kesişen (K): açık')`);
    if (on !== want) await inputOption(ui, 'K');
  };
  // Six upright lines, two squares, a circle and a spot, spread across the view; the fence runs through most of them.
  const SCATTER = FIT(`
    for (const x of [-2.3, -1.4, -0.5, 0.4, 1.3, 2.2]) add({ kind: 'line', a: { x: c.x + x * u, y: c.y - 1.1 * u }, b: { x: c.x + x * u, y: c.y + 0.9 * u }, color: '#3B82F6' });
    const sq = (x0, y0, x1, y1) => add({ kind: 'polygon', pts: [{ x: c.x + x0 * u, y: c.y + y0 * u }, { x: c.x + x1 * u, y: c.y + y0 * u }, { x: c.x + x1 * u, y: c.y + y1 * u }, { x: c.x + x0 * u, y: c.y + y1 * u }], color: '#E5484D' });
    sq(-1.1, -0.5, -0.7, -0.1);
    sq(0.7, 0.1, 1.1, 0.5);
    add({ kind: 'circle', c: { x: c.x + 0.05 * u, y: c.y - 0.5 * u }, r: 0.3 * u, color: '#E5484D' });
    add({ kind: 'point', p: { x: c.x + 1.75 * u, y: c.y - 0.55 * u } });
    add({ kind: 'point', p: { x: c.x - 1.9 * u, y: c.y + 0.6 * u } });`);
  const FENCE = [[-2.6, -0.3], [-1.0, -0.3], [0.6, -0.55], [1.8, -0.55], [2.5, 0.3]];
  const fence = async (ui) => {
    await started(ui, 'selectFence', SCATTER);
    await clickAllU(ui, clickWorld, ...FENCE);
  };
  // Daireyle seç: what lies inside the circle, what it only touches, and the frame around all of it.
  const CLUSTER = FIT(`
    const sq = (x0, y0, x1, y1, o = {}) => add({ kind: 'polygon', pts: [{ x: c.x + x0 * u, y: c.y + y0 * u }, { x: c.x + x1 * u, y: c.y + y0 * u }, { x: c.x + x1 * u, y: c.y + y1 * u }, { x: c.x + x0 * u, y: c.y + y1 * u }], ...o });
    sq(-2.5, -1.4, 2.5, 1.4, { color: '#8B8B8B' });
    sq(-0.6, -0.5, -0.1, 0.0, { color: '#E5484D' });
    sq(0.2, 0.1, 0.7, 0.6, { color: '#E5484D' });
    sq(0.9, -0.6, 1.4, -0.1, { color: '#E5484D' });
    add({ kind: 'line', a: { x: c.x - 1.8 * u, y: c.y + 0.5 * u }, b: { x: c.x + 0.4 * u, y: c.y + 1.0 * u }, color: '#3B82F6' });
    add({ kind: 'line', a: { x: c.x - 0.3 * u, y: c.y - 0.95 * u }, b: { x: c.x + 2 * u, y: c.y - 0.75 * u }, color: '#3B82F6' });
    add({ kind: 'point', p: { x: c.x - 0.8 * u, y: c.y + 0.4 * u } });
    add({ kind: 'point', p: { x: c.x + 2.2 * u, y: c.y + 1.0 * u } });`);
  const circle = async (ui, on) => {
    await started(ui, 'selectCircle', CLUSTER);
    await crossing(ui, on);
    await clickWorld(ui, AU(0.2, 0.05));
    await ui.sleep(250);
  };
  // İçeren alanı seç: a parcel in a block in a district; the same place clicked again and again.
  const NESTED = FIT(`
    const sq = (cx, cy, half, o = {}) => add({ kind: 'polygon', pts: [{ x: c.x + (cx - half) * u, y: c.y + (cy - half * 0.7) * u }, { x: c.x + (cx + half) * u, y: c.y + (cy - half * 0.7) * u }, { x: c.x + (cx + half) * u, y: c.y + (cy + half * 0.7) * u }, { x: c.x + (cx - half) * u, y: c.y + (cy + half * 0.7) * u }], ...o });
    sq(0.5, 0, 0.35, { color: '#E5484D' });
    sq(0.3, 0, 1.2, { color: '#3B82F6' });
    sq(0, 0, 2.5, { color: '#8B8B8B' });`);
  const containing = async (ui, clicks) => {
    await started(ui, 'selectContaining', NESTED);
    for (let i = 0; i < clicks; i++) {
      await clickWorld(ui, AU(0.55, 0.05));
      await ui.sleep(250);
    }
  };
  return [
    { id: 'selectfence-preview', open: async (ui) => (await fence(ui), await hoverU(ui, 2.6, 0.85)) },
    { id: 'selectfence-result', open: async (ui) => (await fence(ui), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'selectcircle-preview', open: async (ui) => (await circle(ui, true), await hoverU(ui, 1.35, 0.05)) },
    { id: 'selectcircle-result', open: async (ui) => (await circle(ui, true), await clickWorld(ui, AU(1.35, 0.05)), await ui.move(2, 2)) },
    { id: 'selectcircle-window-result', open: async (ui) => (await circle(ui, false), await clickWorld(ui, AU(1.35, 0.05)), await ui.move(2, 2)) },
    { id: 'selectcontaining-first', open: async (ui) => (await containing(ui, 1), await ui.move(2, 2)) },
    { id: 'selectcontaining-second', open: async (ui) => (await containing(ui, 2), await hoverU(ui, 0.55, 0.05)) },
    {
      id: 'select-split-list',
      open: async (ui) => {
        await ribbonOn(ui, { ribbonTab: 'home' });
        await ui.eval(`(() => { const k = window.kentos; k.view.camera.fit(k.doc.homeView); k.log.clear(); })()`);
        await ui.clickSel('.ribbon__strip [data-split="select"] .rsplit__arrow');
        await ui.waitFor(`!!document.querySelector('.menu')`);
        await ui.sleep(300);
      },
    },
  ];
}

// Faz 2: the query tools, each on bare ground with the history open under the drawing.
function queryScenes(bare, clickWorld) {
  const none = FIT('return 0;');
  const inputOption = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  /** The tool's toggles (kept for the session) set to exactly these: `chip` is the prompt's text for it. */
  const toggle = async (ui, key, chip, want) => {
    const on = await ui.eval(`window.kentos.tools.active.prompt.value.includes('${chip}: açık')`);
    if (on !== want) await inputOption(ui, key);
  };
  const started = async (ui, id, scratch = none) => {
    await bare(ui, scratch, { ribbonTab: 'map', ...LOGGED });
    await ui.eval(`window.kentos.log.clear()`);
    await startTool(ui, id);
  };
  // Mesafe ölç with a fixed first point: the first, then three rays, the mouse on the way to a fourth.
  const rays = async (ui) => {
    await started(ui, 'measure');
    await toggle(ui, 'S', 'Sabit ilk nokta (S)', true);
    await clickAllU(ui, clickWorld, [-1.6, -0.5], [0.4, -1.3], [1.8, -0.3], [0.9, 1.2]);
  };
  // Alan hesapla: a block of four lines with a square island in it, and a second, smaller region beside it.
  const REGIONS = FIT(`
    const box = (x0, y0, x1, y1) => [[x0, y0, x1, y0], [x1, y0, x1, y1], [x1, y1, x0, y1], [x0, y1, x0, y0]].forEach(([ax, ay, bx, by]) => add({ kind: 'line', a: { x: c.x + ax * u, y: c.y + ay * u }, b: { x: c.x + bx * u, y: c.y + by * u } }));
    box(-2.4, -1.2, 0.8, 1.3);
    box(1.3, -0.6, 2.4, 0.6);
    add({ kind: 'polygon', pts: [{ x: c.x - 1.4 * u, y: c.y - 0.3 * u }, { x: c.x - 0.4 * u, y: c.y - 0.3 * u }, { x: c.x - 0.4 * u, y: c.y + 0.5 * u }, { x: c.x - 1.4 * u, y: c.y + 0.5 * u }], color: '#3B82F6' });`);
  const inside = async (ui, id = 'area') => {
    await started(ui, id, REGIONS);
    await toggle(ui, 'I', 'İçine tıkla (I)', true);
  };
  return [
    { id: 'measure-fixed-preview', open: async (ui) => (await rays(ui), await hoverU(ui, -1.9, 1.0)) },
    { id: 'measure-fixed-result', open: async (ui) => (await rays(ui), await pressEnter(ui), await ui.move(2, 2)) },
    { id: 'measure-fixed-off', open: async (ui) => (await started(ui, 'measure'), await toggle(ui, 'S', 'Sabit ilk nokta (S)', false), await hoverU(ui, 0, 0)) },
    { id: 'area-inside-preview', open: async (ui) => (await inside(ui), await hoverU(ui, -1.9, -0.9)) },
    { id: 'area-inside-result', open: async (ui) => (await inside(ui), await clickWorld(ui, AU(-1.9, -0.9)), await hoverU(ui, 1.85, 0)) },
    { id: 'area-inside-none', open: async (ui) => (await inside(ui), await clickWorld(ui, AU(0.9, -1.0)), await ui.move(2, 2)) },
    {
      id: 'area-draw-result',
      open: async (ui) => {
        await inside(ui);
        await clickWorld(ui, AU(-1.9, -0.9));
        await inputOption(ui, 'A');
        await ui.sleep(300);
        await hoverU(ui, 1.85, 0);
      },
    },
    {
      id: 'station-base',
      open: async (ui) => (await started(ui, 'stationOffset'), await clickAllU(ui, clickWorld, [-2.0, -0.6], [2.0, 0.5]), await hoverU(ui, 0.2, 0.4)),
    },
    {
      id: 'station-point',
      open: async (ui) => {
        await started(ui, 'stationOffset');
        await clickAllU(ui, clickWorld, [-2.0, -0.6], [2.0, 0.5], [-0.9, 0.6]);
        await hoverU(ui, 0.9, -0.9);
      },
    },
  ];
}

// docs/adr/0142: vertex elevations. Kot ver (its second step with the prompt and chips, and what it says after each way),
// the Kot rows of Öznitelikler, the tag of a grip that has an elevation, Koordinat oku on a vertex and the hover card's
// 3D length. Five objects on bare ground carry the scenes; their slots are kept in `window.__elev`.
function adr0142Scenes(bare, clickWorld) {
  const none = FIT('return 0;');
  /** The log emptied and the status bar's flash of the last message put out: nothing of an earlier scene shows. */
  const QUIET = `(() => { window.kentos.log.clear(); document.querySelector('.status__flash')?.removeAttribute('data-show'); })()`;
  // A line with both ends, a polyline with a range and two vertices without an elevation (one at the end), an area
  // with a hole and every vertex elevated, a spot with its height and a circle (takes none). The heights are four-digit,
  // as the Anatolian plateau's are (Sivas, Suşehri: 1100 to 1300 m): the widest text the rows and tags must hold.
  const OBJECTS = FIT(`
    const P = (fx, fy) => ({ x: c.x + fx * u, y: c.y + fy * u });
    const line = add({ kind: 'line', a: P(-2.3, 1.05), b: P(-0.9, 1.3), za: 1100.25, zb: 1104.75, color: '#3B82F6' });
    const path = add({ kind: 'polyline', pts: [P(-2.3, -0.9), P(-1.4, -0.3), P(-0.5, -0.75), P(0.4, -0.15), P(0.9, -0.5)], zs: [1098.5, null, 1105.25, 1101, null], color: '#E5484D' });
    const area = add({
      kind: 'polygon',
      pts: [P(0.4, 0.4), P(2.0, 0.4), P(2.0, 1.4), P(0.4, 1.4)],
      zs: [1096.4, 1097.1, 1098.9, 1097.8],
      holes: [{ pts: [P(0.9, 0.7), P(1.5, 0.7), P(1.5, 1.1), P(0.9, 1.1)], zs: [1097.5, 1097.5, 1097.9, 1097.9] }],
      color: '#3B82F6',
    });
    const spot = add({ kind: 'point', p: P(1.7, -0.6), z: 1118.5 });
    const circle = add({ kind: 'circle', c: P(-1.6, 0.25), r: 0.3 * u, color: '#8B8B8B' });
    window.__elev = { line: line.id, path: path.id, area: area.id, spot: spot.id, circle: circle.id };`);
  const selectNames = (...names) => `(() => { const k = window.kentos; const o = window.__elev; k.selection.set([${names.map((n) => `o.${n}`).join(', ')}]); })()`;
  /** The tool started on the objects (some selected), the log open under the drawing. */
  const started = async (ui, names, options = []) => {
    await bare(ui, OBJECTS, { ribbonTab: 'edit', ...LOGGED });
    await ui.eval(QUIET);
    await ui.eval(selectNames(...names));
    await startTool(ui, 'setElevation');
    for (const o of options) await ui.eval(`window.kentos.tools.active.input('${o}')`);
    await ui.sleep(200);
  };
  const SAMPLE = ['line', 'path', 'area', 'spot', 'circle'];
  /** A section of Öznitelikler folded or opened, whichever it is not now. */
  const section = async (ui, title, open) => {
    const head = `[...document.querySelectorAll('.panel--props .props__section')].find((b) => b.textContent.includes(${JSON.stringify(title)}))`;
    // No such section (nothing selected): nothing to fold or open.
    if (await ui.eval(`(() => { const b = ${head}; return !!b && (b.getAttribute('aria-expanded') === 'true') !== ${open}; })()`)) await ui.clickText('.panel--props .props__section', title);
  };
  /** Öznitelikler over the dock (the layer tree at its least) with Genel folded, so Geometri shows whole at 1100×650. */
  const props = async (ui, names) => {
    await bare(ui, OBJECTS, { ribbonTab: 'edit', layersFraction: 0.15 });
    await ui.eval(QUIET);
    await ui.eval(selectNames(...names));
    await ui.move(2, 2);
    await ui.sleep(400);
    await section(ui, 'Genel', false);
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  const propsClose = async (ui) => (await section(ui, 'Genel', true), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  /** The objects on bare ground with some selected, the select tool active. */
  const selected = async (ui, names) => {
    await bare(ui, OBJECTS, { ribbonTab: 'edit' });
    await ui.eval(QUIET);
    await ui.eval(selectNames(...names));
    await ui.sleep(300);
  };
  return [
    { id: 'elevation-step', open: async (ui) => (await started(ui, SAMPLE), await ui.move(2, 2)) },
    { id: 'elevation-raise-step', open: async (ui) => (await started(ui, SAMPLE, ['A']), await ui.move(2, 2)) },
    { id: 'elevation-picking', open: async (ui) => (await started(ui, []), await ui.move(2, 2)) },
    { id: 'elevation-fixed-result', open: async (ui) => (await started(ui, SAMPLE), await typeValue(ui, '100'), await ui.move(2, 2)) },
    { id: 'elevation-raise-result', open: async (ui) => (await started(ui, SAMPLE, ['A']), await typeValue(ui, '2.5'), await ui.move(2, 2)) },
    { id: 'elevation-reset-result', open: async (ui) => (await started(ui, SAMPLE, ['S']), await ui.move(2, 2)) },
    { id: 'props-line', open: async (ui) => props(ui, ['line']), close: propsClose },
    { id: 'props-polyline', open: async (ui) => props(ui, ['path']), close: propsClose },
    { id: 'props-polygon', open: async (ui) => props(ui, ['area']), close: propsClose },
    { id: 'props-several', open: async (ui) => props(ui, ['line', 'path', 'area', 'circle']), close: propsClose },
    // A grip of the polyline: its third vertex has an elevation, the second none. The tag beside the pointer resting on
    // it; then the grip clicked and carried, its tag with the distance and the elevation the vertex keeps.
    { id: 'grip-tag', open: async (ui) => (await selected(ui, ['path']), await hoverU(ui, -0.5, -0.75)) },
    { id: 'grip-tag-none', open: async (ui) => (await selected(ui, ['path']), await hoverU(ui, -1.4, -0.3)) },
    // GeoJSON içe aktar over the fixture with elevations (fixtures/formats/v1/gis/kotlu.geojson, EPSG:5256 like the
    // project): the file line's facts say how many objects came with them (Kotlu nesne).
    {
      id: 'import-geojson-elevations',
      open: async (ui) => {
        const bytes = readFileSync(new URL('../../../../fixtures/formats/v1/gis/kotlu.geojson', import.meta.url)).toString('base64');
        await bare(ui, none);
        await ui.eval(`import('/src/ui/io/GisImportDialog.ts').then((m) => m.openGeoJsonImport(window.kentos, { name: 'kotlu.geojson', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'GeoJSON', accept: { 'application/geo+json': ['.geojson', '.json'] } }))`);
        await ui.waitFor(`!!document.querySelector('.dialog--io .io-table')`, 15000);
        await ui.sleep(500);
      },
      close: async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL), await ribbonOff(ui)),
    },
    // The hover card (select tool, the pointer at rest on an object): 3B uzunluk of the line and 3B çevre of the area.
    { id: 'hover-card-line', open: async (ui) => (await selected(ui, []), await hoverU(ui, -1.6, 1.175), await ui.sleep(700)) },
    { id: 'hover-card-area', open: async (ui) => (await selected(ui, []), await hoverU(ui, 0.65, 0.9), await ui.sleep(700)) },
    // Koordinat oku on vertices: one without an elevation, then two that have one (the last one's tag has the Z).
    {
      id: 'coord-read-vertex',
      open: async (ui) => {
        await bare(ui, OBJECTS, { ribbonTab: 'edit', ...LOGGED });
        await ui.eval(QUIET);
        await ui.eval(`window.kentos.commands.execute('crs.query')`);
        await ui.sleep(300);
        for (const [fx, fy] of [[-1.4, -0.3], [-2.3, 1.05], [-0.5, -0.75]]) {
          await clickWorld(ui, AU(fx, fy));
          await ui.sleep(300);
        }
        await ui.move(2, 2);
      },
    },
    {
      id: 'grip-tag-moving',
      open: async (ui) => {
        await selected(ui, ['path']);
        await clickWorld(ui, AU(-0.5, -0.75));
        await ui.sleep(250);
        await hoverU(ui, -0.1, -1.15);
      },
    },
  ];
}

// The drawing and editing tools of docs/adr/0140: the ribbon tabs that hold them and their split buttons; each
// tool at work is added here as it is built. Same layout helpers as the ribbon group.
SCENES.tools = [
  ...[
    ['ribbon-modify', 'modify', 'cad'],
    ['ribbon-edit', 'edit'],
    ['ribbon-map', 'map'],
  ].map(([id, tab, type]) => ({ id, open: async (ui) => (await ribbonOn(ui, { ribbonTab: tab, type }), await ui.sleep(400)) })),
  {
    id: 'split-corner',
    open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'modify', type: 'cad' }), await ui.clickSel('.ribbon__strip [data-split="corner"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
  },
  ...toolScenes(),
].map((s) => ({ close: async (ui) => (await ui.eval(UNDO_ALL), await ribbonOff(ui)), ...s }));

SCENES.svgedit = [
  { id: 'new', open: (ui) => openSvg(ui) },
  { id: 'library-drawing', open: (ui) => openSvg(ui, 'mpyy.svg.akaryakit') },
  { id: 'rect-drawn', open: async (ui) => (await openSvg(ui), await drawShape(ui, 'r', [0.15, 0.15], [0.5, 0.42])) },
  { id: 'shapes-all-selected', open: async (ui) => (await openSvg(ui), await drawThree(ui), await ui.key('v'), await ui.key('a', { ctrl: true }), await ui.sleep(400)) },
  ...[
    ['Hizala', 'tab-align'],
    ['Dönüştür', 'tab-transform'],
    ['Dizi', 'tab-array'],
  ].map(([tab, id]) => ({
    id,
    open: async (ui) => {
      await openSvg(ui);
      await drawThree(ui);
      await ui.key('v');
      await ui.key('a', { ctrl: true });
      await ui.clickText('.dialog--svge .svgp__tab', tab);
      await ui.sleep(500);
    },
  })),
  ...[
    ['Dosya', 'menu-file'],
    ['Yol', 'menu-path'],
    ['Nesne', 'menu-object'],
    ['Seç', 'menu-select'],
  ].map(([menu, id]) => ({
    id,
    open: async (ui) => {
      await openSvg(ui);
      await drawThree(ui);
      await ui.key('v');
      await ui.key('a', { ctrl: true });
      await ui.clickText('.dialog--svge .svge__pbar .btn', menu);
      await ui.sleep(400);
    },
  })),
  { id: 'menu-snap-kinds', open: async (ui) => (await openSvg(ui), await ui.clickSel('.dialog--svge button[aria-label="Kenet türleri"]'), await ui.sleep(400)) },
  {
    id: 'polyline-in-progress',
    open: async (ui) => {
      await openSvg(ui);
      await ui.key('l');
      for (const at of [[0.15, 0.7], [0.4, 0.3], [0.65, 0.7]]) await ui.clickAt(...(await paperAt(ui, ...at)));
      await ui.move(...(await paperAt(ui, 0.85, 0.35)));
      await ui.sleep(400);
    },
  },
  {
    id: 'text-object',
    open: async (ui) => {
      await openSvg(ui);
      await ui.key('t');
      await ui.clickAt(...(await paperAt(ui, 0.3, 0.5)));
      await ui.sleep(300);
      // The tool's hint: click, then write the text on the right.
      await ui.clickSel('.dialog--svge input[aria-label="Metin"]');
      await ui.key('a', { ctrl: true });
      await ui.type('KentOS');
      await ui.sleep(400);
    },
  },
  {
    id: 'node-editing',
    open: async (ui) => {
      await openSvg(ui);
      await drawShape(ui, 'e', [0.25, 0.25], [0.75, 0.7]);
      await ui.key('c', { ctrl: true, shift: true });
      await ui.sleep(200);
      await ui.key('a');
      await ui.sleep(300);
      await ui.clickAt(...(await paperAt(ui, 0.75, 0.475)));
      await ui.sleep(400);
    },
  },
  {
    id: 'measure',
    open: async (ui) => {
      await openSvg(ui);
      await drawShape(ui, 'r', [0.15, 0.15], [0.5, 0.42]);
      await ui.key('m');
      await ui.drag(...(await paperAt(ui, 0.15, 0.15)), ...(await paperAt(ui, 0.5, 0.42)));
      await ui.sleep(400);
    },
  },
  { id: 'xml-source', open: async (ui) => (await openSvg(ui), await drawThree(ui), await ui.clickText('.dialog--svge .svge__pbar .btn', 'Kaynak'), await ui.sleep(500)) },
  {
    id: 'document-properties',
    open: async (ui) => {
      await openSvg(ui);
      await ui.clickText('.dialog--svge .svge__pbar .btn', 'Dosya');
      await ui.clickText('.menu .menu__item', 'Belge özellikleri');
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
      await ui.sleep(400);
    },
    close: closeSvg,
  },
  {
    id: 'export-dialog',
    open: async (ui) => {
      await openSvg(ui);
      await drawThree(ui);
      await ui.clickText('.dialog--svge .svge__pbar .btn', 'Dışa aktar');
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
      await ui.sleep(500);
    },
    close: closeSvg,
  },
  {
    id: 'unsaved-close-question',
    open: async (ui) => {
      await openSvg(ui);
      await drawShape(ui, 'r', [0.15, 0.15], [0.5, 0.42]);
      // Esc steps back (the draft, the selection) until the editor asks about the unsaved drawing.
      for (let i = 0; i < 4 && (await ui.eval(`document.querySelectorAll('.dialog').length < 2`)); i++) {
        await ui.key('Escape');
        await ui.sleep(250);
      }
      await ui.waitFor(`document.querySelectorAll('.dialog').length >= 2`);
      await ui.sleep(400);
    },
    close: closeSvg,
  },
].map((s) => ({ close: closeSvg, ...s }));

/** The SVG editor: a new drawing, or a library drawing by its id. */
async function openSvg(ui, id) {
  await ui.eval(`import('/src/ui/svgedit/SvgEditor.ts').then((m) => m.openSvgEditor(window.kentos, ${id ? `{ id: ${JSON.stringify(id)} }` : '{}'}))`);
  await ui.waitFor(`!!document.querySelector('.dialog--svge .svge__paper')`);
  await ui.sleep(600);
}
/** A point of the paper, as fractions of its box, on the screen. */
const paperAt = (ui, fx, fy) =>
  ui.eval(`(() => { const r = document.querySelector('.dialog--svge .svge__paper').getBoundingClientRect(); return [Math.round(r.left + r.width * ${fx}), Math.round(r.top + r.height * ${fy})]; })()`);
/** A shape drawn with its tool's key from one paper point to another. */
async function drawShape(ui, key, from, to) {
  await ui.key(key);
  await ui.drag(...(await paperAt(ui, ...from)), ...(await paperAt(ui, ...to)));
  await ui.sleep(300);
}
/** A rectangle, an ellipse and a polygon. */
async function drawThree(ui) {
  await drawShape(ui, 'r', [0.1, 0.12], [0.45, 0.4]);
  await drawShape(ui, 'e', [0.55, 0.12], [0.9, 0.4]);
  await drawShape(ui, 'p', [0.5, 0.72], [0.62, 0.88]);
}
/** The editor closed without saving: Esc until it asks, then its "don't save". */
async function closeSvg(ui) {
  for (let i = 0; i < 6 && (await ui.eval(`!!document.querySelector('.dialog')`)); i++) {
    const asked = await ui.eval(`[...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden|Uygulamadan/.test(b.textContent))`);
    if (asked) await ui.eval(`[...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden|Uygulamadan/.test(b.textContent)).click()`);
    else await ui.escapeAll(1);
    await ui.sleep(250);
  }
}

async function openLegend(ui) {
  await ui.eval(`window.kentos.commands.execute('style.legend')`);
  await ui.waitFor(`!!document.querySelector('.dialog--legend')`);
  await ui.sleep(700);
}
async function openDesigner(ui, kind) {
  await ui.eval(`import('/src/ui/style/SymbolDesigner.ts').then((m) => m.openSymbolDesigner(window.kentos, { newKind: ${JSON.stringify(kind)} }))`);
  await ui.waitFor(`!!document.querySelector('.dialog--sdesign .sdes__row')`);
  await ui.sleep(600);
}
/** The designer closed without saving: Esc, and the unsaved question's "don't save" when it asks. */
async function closeDesigner(ui) {
  for (let i = 0; i < 4 && (await ui.eval(`!!document.querySelector('.dialog')`)); i++) {
    const asked = await ui.eval(`[...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden|Uygulamadan/.test(b.textContent))`);
    if (asked) await ui.eval(`[...document.querySelectorAll('.dialog .btn')].find((b) => /Kaydetmeden|Uygulamadan/.test(b.textContent)).click()`);
    else await ui.escapeAll(1);
    await ui.sleep(250);
  }
}

async function openManager(ui) {
  await ui.eval(`window.kentos.commands.execute('style.manager')`);
  await ui.waitFor(`!!document.querySelector('.dialog--styles .scard')`);
  await ui.sleep(700);
}
/** Clicks the expand caret of a tree row by its leading text. */
const treeCaret = (text) => `[...document.querySelectorAll('.dialog--styles .tree__row')].find((e) => e.textContent.trim().startsWith(${JSON.stringify(text)}))?.querySelector('.tree__caret')?.click()`;
/** The first system symbol copied to Kitaplığım and shown (its fields become editable). */
async function copyFirstToMine(ui) {
  await ui.clickSel('.dialog--styles .scard');
  await ui.clickText('.dialog--styles .smgr__actions .btn', 'Kopyala');
  await ui.clickText('.menu .menu__item', 'Kitaplığıma');
  await ui.sleep(700);
}
/** Four object templates put in Kitaplığım (docs/adr/0176). */
const ADD_TEMPLATES = `(() => {
  const lib = window.kentos.styles.library;
  const templates = [
    { kind: 'template', id: 'u-sablon-parsel', name: 'Parsel sınırı', path: ['Kadastro'], description: 'Kadastro parselinin sınırı',
      template: { tool: 'polygon', layer: { path: ['Kadastro'], name: 'Parsel', color: '#E5484D', lineWeight: 0.35 }, symbol: 'temel.alan.kenar-ici', attrs: { Tür: 'Parsel' }, label: 'P' } },
    { kind: 'template', id: 'u-sablon-nokta', name: 'Poligon noktası', path: ['Kadastro'],
      template: { tool: 'point', layer: { path: [], name: 'Nokta' }, color: '#3E63DD', point: { name: 'P1', code: 'SN' } } },
    { kind: 'template', id: 'u-sablon-yol', name: 'Yol kenarı', path: ['Kadastro'], template: { tool: 'polyline', layer: { path: ['Ulaşım'], name: 'Yol' }, color: '#F5A524', lineWeight: 0.5 } },
    { kind: 'template', id: 'u-sablon-yazi', name: 'Ada numarası', path: ['Kadastro'], template: { tool: 'text', layer: { path: [], name: 'Yazılar' }, text: { height: 2.5, align: 'middleCenter', mask: true } } },
  ];
  for (const p of templates) if (!lib.get(p.id)) lib.add('user', p);
})()`;
/** A group template over ADD_TEMPLATES' (docs/adr/0176 §5): a parcel with its corner points, its number and an inner line. */
const ADD_GROUP_TEMPLATE = `(() => {
  const lib = window.kentos.styles.library;
  const items = [
    { kind: 'template', id: 'u-sablon-numara', name: 'Parsel numarası', path: ['Kadastro'],
      template: { tool: 'text', layer: { path: [], name: 'Yazılar' }, label: '101', text: { height: 2.5 } } },
    { kind: 'template', id: 'u-sablon-grup', name: 'Parsel ve köşeleri', path: ['Kadastro'],
      template: { tool: 'polygon', layer: { path: ['Kadastro'], name: 'Parsel' }, members: [
        { template: 'u-sablon-nokta', rule: 'vertices' },
        { template: 'u-sablon-numara', rule: 'centroid' },
        { template: 'u-sablon-parsel', rule: 'offset', distance: 0.5, side: 'inside' },
      ] } },
  ];
  for (const p of items) if (!lib.get(p.id)) lib.add('user', p);
})()`;
/** Kitaplığım emptied again after a scene that copied into it. */
const REMOVE_MINE = `(() => { const lib = window.kentos.styles.library; for (const i of lib.items('user')) lib.remove(i.id); })()`;

/** Katman stili on a layer of the demo drawing, on one kind of renderer. */
async function openStyled(ui, layerId, kind) {
  await ui.eval(`import('/src/ui/style/LayerStyleDialog.ts').then((m) => m.openLayerStyle(window.kentos, ${JSON.stringify(layerId)}))`);
  await ui.waitFor(`!!document.querySelector('.dialog--lstyle .lsty__top')`);
  await ui.clickText('.dialog--lstyle .seg__opt', kind);
  await ui.sleep(400);
}
/** The value expression, typed and left (its change event). */
const setStyleExpr = (text) => `(() => { const i = document.querySelector('.dialog--lstyle .lsty__expr'); i.value = ${JSON.stringify(text)}; i.dispatchEvent(new Event('change')); })()`;
// DXF içe aktar over the fixture with blocks (fixtures/formats/v1/blocks.dxf, docs/adr/0144 §5), the way a user
// works it: the window read, İçe aktar and Blokları patlat clicked with the mouse.
const DXF_BLOCKS = readFileSync(new URL('../../../../fixtures/formats/v1/blocks.dxf', import.meta.url)).toString('base64');
const OPEN_DXF_BLOCKS = `import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'blocks.dxf', bytes: Uint8Array.from(atob('${DXF_BLOCKS}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`;
const DXF_READ = `!!document.querySelector('.dialog--io .io-table') && !document.querySelector('.dialog--io .io-reading')`;
const dxfBlocksOpen = async (ui) => (await ui.eval(OPEN_DXF_BLOCKS), await ui.waitFor(DXF_READ, 15000));
const dxfBlocksIn = async (ui) => (await dxfBlocksOpen(ui), await ui.clickText('.dialog--io .btn--primary', 'İçe aktar'), await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000));
const DXF_ATTRIBUTES = readFileSync(new URL('../../../../fixtures/formats/v1/attributes.dxf', import.meta.url)).toString('base64');
const OPEN_DXF_ATTRIBUTES = `import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'attributes.dxf', bytes: Uint8Array.from(atob('${DXF_ATTRIBUTES}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`;
const dxfAttributesOpen = async (ui) => (await ui.eval(OPEN_DXF_ATTRIBUTES), await ui.waitFor(DXF_READ, 15000));
const dxfAttributesIn = async (ui) => (await dxfAttributesOpen(ui), await ui.clickText('.dialog--io .btn--primary', 'İçe aktar'), await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000));
/** The attribute fixture's inserts, framed, nothing chosen, the Bloklar panel open. */
const FRAME_ATTRIBUTES = `(() => { const k = window.kentos; k.selection.clear(); k.view.camera.fit({ minX: 487089, minY: 4420197.5, maxX: 487114.5, maxY: 4420203 }, 48); k.view.requestRender(); k.commands.execute('block.panel'); })()`;
/** A save picker that keeps what is written in the page (window.__disk.bytes); the user's picker kept aside. */
const CAPTURE_SAVE = `(() => { const disk = (window.__disk = { bytes: null, picker: window.kentos.files.picker }); window.kentos.files.picker = { save: async (name) => ({ name, createWritable: async () => { const parts = []; return { write: async (d) => parts.push(d), close: async () => { disk.bytes = new Uint8Array(await new Blob(parts).arrayBuffer()); } }; } }), open: async () => null }; })()`;
SCENES.blocks = [
  // Read a second time: the names are the drawing's now, and the window says which new names they take.
  { id: 'import-dxf-blocks', open: async (ui) => (await dxfBlocksIn(ui), await dxfBlocksOpen(ui), await ui.sleep(400)) },
  {
    id: 'import-dxf-blocks-explode',
    open: async (ui) => {
      await dxfBlocksOpen(ui);
      await ui.clickText('.dialog--io .io-check', 'Blokları patlat');
      await ui.waitFor(`${DXF_READ} && document.querySelector('.dialog--io .io-summary')?.textContent.includes('Blokları patlat seçili')`, 15000);
      await ui.sleep(400);
    },
  },
  // In, then DXF dışa aktar over the whole drawing: the summary says how the inserts and their blocks are written.
  {
    id: 'export-dxf-blocks',
    open: async (ui) => {
      await dxfBlocksIn(ui);
      await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
      await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
      await ui.clickText('.dialog--io .seg__opt', 'Tümü');
      await ui.sleep(400);
    },
  },
  // In, then GeoJSON dışa aktar over the whole drawing: an insert is its block's objects placed (GeometryCollection).
  {
    id: 'export-geojson-blocks',
    open: async (ui) => {
      await dxfBlocksIn(ui);
      await ui.eval(`window.kentos.commands.execute('file.export.geojson')`);
      await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
      await ui.clickText('.dialog--io .seg__opt', 'Tümü');
      await ui.sleep(400);
    },
  },
  // A block with attribute definitions (docs/adr/0144 §7, fixtures/formats/v1/attributes.dxf): the window says what
  // became of each ATTDEF and ATTRIB; in, the inserts show their values (and the default), EK's value a text of its own.
  { id: 'import-dxf-attributes', open: async (ui) => (await dxfAttributesOpen(ui), await ui.sleep(400)) },
  {
    id: 'import-dxf-attributes-done',
    open: async (ui) => {
      await ui.eval(`(() => { const u = window.kentos.ui; u.bottomHeight.set(190); u.bottomTab.set('history'); u.bottomExpanded.set(true); })()`);
      await dxfAttributesOpen(ui);
      await ui.clickText('.dialog--io .btn--primary', 'İçe aktar');
      await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000);
      await ui.eval(FRAME_ATTRIBUTES);
      await ui.sleep(600);
    },
  },
  // In, then DXF dışa aktar over the whole drawing: the summary says the attributes go as ATTDEF and ATTRIB.
  {
    id: 'export-dxf-attributes',
    open: async (ui) => {
      await dxfAttributesIn(ui);
      await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
      await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
      await ui.clickText('.dialog--io .seg__opt', 'Tümü');
      await ui.sleep(400);
    },
  },
  // Written with Dışa aktar, the import undone and the written file taken in again: the inserts show the values
  // they showed (the definitions' ATTDEFs, the inserts' ATTRIBs), the log says what went out and what came in.
  {
    id: 'import-dxf-attributes-back',
    open: async (ui) => {
      await ui.eval(`(() => { const u = window.kentos.ui; u.bottomHeight.set(190); u.bottomTab.set('history'); u.bottomExpanded.set(true); })()`);
      await dxfAttributesIn(ui);
      await ui.eval(CAPTURE_SAVE);
      await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
      await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
      await ui.clickText('.dialog--io .seg__opt', 'Tümü');
      await ui.clickText('.dialog--io .btn--primary', 'Dışa aktar');
      await ui.waitFor(`!document.querySelector('.dialog--io') && !!window.__disk.bytes`, 15000);
      await ui.eval(`(() => { window.kentos.files.picker = window.__disk.picker; })()`);
      await ui.eval(UNDO_ALL);
      await ui.eval(`import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'oznitelikler.dxf', bytes: window.__disk.bytes }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`);
      await ui.waitFor(DXF_READ, 15000);
      await ui.clickText('.dialog--io .btn--primary', 'İçe aktar');
      await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000);
      await ui.eval(FRAME_ATTRIBUTES);
      await ui.sleep(600);
    },
  },
  // In: the drawing shows the inserts, the Bloklar panel the four definitions and the message log what went in.
  {
    id: 'import-dxf-blocks-done',
    open: async (ui) => {
      await ui.eval(`(() => { const u = window.kentos.ui; u.bottomHeight.set(190); u.bottomTab.set('history'); u.bottomExpanded.set(true); })()`);
      await dxfBlocksIn(ui);
      await ui.eval(`window.kentos.commands.execute('block.panel')`);
      await ui.sleep(600);
    },
  },
].map((s) => ({
  close: async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL), await ui.eval(`(() => { const u = window.kentos.ui; u.dockTab.set('layers'); u.bottomExpanded.set(false); })()`)),
  ...s,
}));

// Nokta editörü (docs/adr/0153) on fixtures/interaction/v1/point-editor.kcad, the scene the desktop's
// `points::tests::screens` draws: the Noktalar tab with three points selected in the drawing; sorted by Ad; a search;
// the layer list open.
const POINT_EDITOR = readFileSync(new URL('../../../../fixtures/interaction/v1/point-editor.kcad', import.meta.url), 'utf8');
const openPointEditor = async (ui) => {
  await ui.eval(`(async () => {
    const k = window.kentos;
    k.files.ask = async () => 'drop';
    if (!(await k.files.load(${JSON.stringify(POINT_EDITOR)}, null))) throw new Error('point-editor.kcad did not load');
    (await import('/src/ui/bottom/PointTable.ts')).resetPointTable();
    k.ui.bottomHeight.set(300);
    k.commands.execute('point.editor');
    k.view.zoomExtents();
    // A little room round the drawing.
    const c = k.view.camera;
    c.scale = c.scale * 0.8;
    c.panBy(0, 0);
    k.selection.set([4, 5, 6]);
  })()`);
  await ui.sleep(700);
};
/** The centre of row `i`'s cell `j` (in the order shown). */
const rowCell = (ui, i, j) =>
  ui.eval(`(() => { const td = document.querySelectorAll('.ptable tbody tr[data-at="${i}"] td')[${j}]; const r = td.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
/** Çift noktaları ayıkla's window over every row: nothing selected, İşlemler ▾ → Çift noktaları ayıkla…. */
const openDedupe = async (ui) => {
  await openPointEditor(ui);
  await ui.eval(`window.kentos.selection.clear()`);
  await ui.sleep(200);
  await ui.clickText('.ptable__btn', 'İşlemler');
  await ui.clickText('.menu__title', 'Çift noktaları ayıkla…');
  await ui.waitFor(`!!document.querySelector('.dialog--point-batch')`, 3000);
  const said = await ui.eval(`document.querySelector('.dialog--point-batch .io-summary').textContent`);
  if (said !== '2 grupta 4 nokta; 2 nokta silinecek.') throw new Error(`Çift noktaları ayıkla: ${said}`);
};
/**
 * The files the next pick and save see: an open gives `text` as `name`; a save keeps what is written in
 * `window.__io.written` (the smoke test's picker). The scenes' close puts the app's own picker back.
 */
const fakeFiles = (ui, name, text) =>
  ui.eval(`(() => {
    const k = window.kentos;
    const io = (window.__io ??= { original: k.files.picker });
    io.written = null;
    k.files.picker = {
      open: async () => ({ name: ${JSON.stringify(name)}, getFile: async () => new Blob([${JSON.stringify(text)}]) }),
      save: async (n) => ({ name: n, getFile: async () => new Blob([]), createWritable: async () => { const parts = []; return { write: async (d) => { parts.push(d); }, close: async () => { io.written = { name: n, text: parts.map((p) => (typeof p === 'string' ? p : new TextDecoder().decode(p))).join('') }; } }; } }),
    };
  })()`);
/** A list of three points: 101 and 105 again (measured anew), 201 new; for İçe aktar. */
const IMPORTED = '101 487000.02 4419999.99 100.3\r\n105 487024.01 4420018 101.8\r\n201 487050 4420010 103\r\n';
/** İçe aktar from İşlemler ▾ with IMPORTED: the import window, İçe aktar, then the window Çift noktaları ayıkla opens. */
const importAgain = async (ui) => {
  await openPointEditor(ui);
  await ui.eval(`window.kentos.selection.clear()`);
  await fakeFiles(ui, 'olcum.ncn', IMPORTED);
  await ui.clickText('.ptable__btn', 'İşlemler');
  await ui.clickText('.menu__title', 'İçe aktar…');
  await ui.waitFor(`!!document.querySelector('.dialog--io .io-table tbody tr')`, 10000);
  await ui.clickText('.dialog--io .btn--primary', 'İçe aktar');
  await ui.waitFor(`!!document.querySelector('.dialog--point-batch')`, 10000);
  const seen = await ui.eval(`({ header: document.querySelector('.dialog--point-batch .io-file__name').textContent, by: document.querySelector('.dialog--point-batch .seg__opt[aria-checked="true"]').textContent, said: document.querySelector('.dialog--point-batch .io-summary').textContent })`);
  const want = { header: 'Aynı adlı 4 nokta', by: 'Aynı ad', said: '2 grupta 4 nokta; 2 nokta silinecek.' };
  if (JSON.stringify(seen) !== JSON.stringify(want)) throw new Error(`İçe aktar: ${JSON.stringify(seen)}`);
};
/** Rows `a` to `b` selected: a click on the first, Shift and a click on the last. */
const pickRows = async (ui, a, b) => {
  await ui.clickAt(...(await rowCell(ui, a, 1)));
  await ui.clickAt(...(await rowCell(ui, b, 1)), { modifiers: 8 });
  await ui.sleep(200);
};
/** Two templates drawn with: the field shows the last, the menu lists both first. */
const DRAW_TEMPLATES = `(() => {
  const k = window.kentos;
  k.commands.execute('template.draw', 'u-sablon-yol');
  k.tools.activate('select');
  k.commands.execute('template.draw', 'u-sablon-parsel');
})()`;
/** The ribbon's Şablonlar field opened: itself, or its folded panel's button first in a narrow window. */
async function openTemplateField(ui) {
  const shown = await ui.eval(`(() => { const d = document.querySelector('.dropdown--template'); return !!d && d.offsetParent !== null; })()`);
  if (!shown) {
    await ui.clickSel('.rpanel[data-panel="Şablonlar"] .rpanel__collapsed');
    await ui.sleep(300);
  }
  await ui.clickSel('.dropdown--template');
  await ui.sleep(300);
}
const closeTemplates = async (ui) => (await ui.escapeAll(3), await ui.eval(UNDO_ALL), await ui.eval(REMOVE_MINE), await ui.eval(`window.kentos.ui.dockTab.set('layers')`));
/** Şablonlar on the ribbon's Giriş (docs/adr/0176 §4c): the desktop's `templates_panel::tests::ribbon_screens`. */
SCENES.templates = [
  { id: 'ribbon-empty', open: async (ui) => (await ui.eval(ADD_TEMPLATES), await ui.sleep(300)), close: closeTemplates },
  { id: 'ribbon-drawing', open: async (ui) => (await ui.eval(ADD_TEMPLATES), await ui.eval(DRAW_TEMPLATES), await ui.sleep(400)), close: closeTemplates },
  {
    id: 'ribbon-menu',
    open: async (ui) => (await ui.eval(ADD_TEMPLATES), await ui.eval(DRAW_TEMPLATES), await ui.sleep(300), await openTemplateField(ui)),
    close: closeTemplates,
  },
  {
    // The Şablonlar panel's row menu with the drawing's objects selected: Seçili nesnelere uygula (docs/adr/0176 §6);
    // the desktop's `templates_panel::tests::panel_screens`.
    id: 'panel-menu',
    open: async (ui) => {
      await ui.eval(ADD_TEMPLATES);
      await ui.eval(`(() => { const k = window.kentos; k.commands.execute('edit.selectAll'); k.commands.execute('template.panel'); })()`);
      await ui.sleep(400);
      const at = await ui.eval(`(() => { const r = [...document.querySelectorAll('.panel--templates .tree__row')].find((e) => e.querySelector('.tree__name')?.textContent === 'Parsel sınırı'); const b = r.getBoundingClientRect(); return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)]; })()`);
      await ui.contextClick(...at);
      await ui.sleep(300);
    },
    close: async (ui) => (await closeTemplates(ui), await ui.eval(`window.kentos.commands.execute('edit.deselect')`)),
  },
];

// Kaynaklar (docs/adr/0199 §7): a folder added through the browser's folder access (here the page's own private
// folder, written first: the browser's window cannot be answered headless) with its files and a folder inside, a file's
// menu; KentOS without a server. The desktop's `sources::tests::screens`; KentOS's projects with a real server are
// cloud-shots.mjs's “sources-*”.
const SOURCE_FOLDER = `(async () => {
  const root = await navigator.storage.getDirectory();
  const dir = await root.getDirectoryHandle('Kaynak verisi', { create: true });
  const put = async (folder, name, text) => {
    const w = await (await folder.getFileHandle(name, { create: true })).createWritable();
    await w.write(text);
    await w.close();
  };
  for (const n of ['parsel.shp', 'parsel.shx', 'parsel.dbf', 'parsel.prj', 'yollar.geojson', 'imar.dxf', 'pafta.ncz', 'rota.gpx', 'alim.nmea', 'noktalar.ncn', 'rapor.pdf', '.gizli.dxf']) await put(dir, n, 'x');
  await put(await dir.getDirectoryHandle('Pafta 2', { create: true }), 'kot.csv', 'x');
  window.showDirectoryPicker = async () => dir;
  window.kentos.commands.execute('data.sources');
})()`;
const sourceRow = (name) =>
  `(() => { const r = [...document.querySelectorAll('.panel--sources .tree__row')].find((e) => e.querySelector('.tree__name')?.textContent === ${JSON.stringify(name)}); if (!r) return null; const b = r.getBoundingClientRect(); return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)]; })()`;
async function addSourceFolder(ui) {
  await ui.eval(SOURCE_FOLDER);
  await ui.sleep(300);
  await ui.eval(`document.querySelector('.panel--sources .src__add-folder').click()`);
  await ui.waitFor(`[...document.querySelectorAll('.panel--sources .tree__name')].some((e) => e.textContent === 'yollar.geojson')`);
  // The folder inside opened (an earlier scene may have left it open).
  const shown = `[...document.querySelectorAll('.panel--sources .tree__name')].some((e) => e.textContent === 'kot.csv')`;
  if (!(await ui.eval(shown))) await ui.clickAt(...(await ui.eval(sourceRow('Pafta 2'))));
  await ui.waitFor(shown);
  await ui.sleep(300);
}
SCENES.sources = [
  { id: 'folders', open: addSourceFolder, close: async (ui) => ui.eval(`window.kentos.ui.dockTab.set('layers')`) },
  {
    id: 'file-menu',
    open: async (ui) => {
      await addSourceFolder(ui);
      await ui.contextClick(...(await ui.eval(sourceRow('imar.dxf'))));
      await ui.waitFor(`!!document.querySelector('.menu')`);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(1), await ui.eval(`window.kentos.ui.dockTab.set('layers')`)),
  },
];

// Öznitelikler's fields (docs/adr/0199 §5): the sample's parcel layer given fields, its first parcel selected, Genel and
// Geometri folded; the fields by their kinds (a value list as a list, numbers and text in boxes). The desktop's
// `features::tests::screens`.
const PARCEL_FIELDS = JSON.stringify([
  { name: 'Ada', kind: 'integer', required: true },
  { name: 'Parsel', kind: 'integer', min: '1' },
  { name: 'Nitelik', kind: 'text', values: [{ code: 'Arsa', label: 'Arsa' }, { code: 'Kargir ev ve arsası', label: 'Kargir ev ve arsası' }] },
  { name: 'Tapu alanı (m²)', alias: 'Tapu alanı', kind: 'decimal', scale: 2 },
  { name: 'Mahalle', kind: 'text' },
  { name: 'Pafta', kind: 'text' },
]);
SCENES.fields = [
  {
    id: 'properties',
    open: async (ui) => {
      await ui.eval(`(() => { const k = window.kentos; k.doc.setLayerFields('parsel', ${PARCEL_FIELDS}); k.selection.set([k.doc.byLayer('parsel')[0].id]); })()`);
      await ui.sleep(300);
      await ui.clickText('.props__section', 'Genel');
      await ui.clickText('.props__section', 'Geometri');
      await ui.sleep(300);
    },
    close: async (ui) => {
      await ui.clickText('.props__section', 'Genel');
      await ui.clickText('.props__section', 'Geometri');
      await ui.eval(`(() => { const k = window.kentos; k.selection.clear(); while (k.doc.canUndo.value) k.doc.undo(); })()`);
    },
  },
];

// Katmanlar's row menu (docs/adr/0177 §3: Kopyasını oluştur, Başka katmanlarla birleştir…); the desktop's icon tour “katman”.
SCENES.layers = [
  {
    id: 'row-menu',
    open: async (ui) => {
      const at = await ui.eval(
        `(() => { const r = [...document.querySelectorAll('.tree__row')].find((e) => e.querySelector('.tree__name')?.textContent === 'Parsel sınırı'); const b = r.getBoundingClientRect(); return [Math.round(b.left + b.width / 2), Math.round(b.top + b.height / 2)]; })()`,
      );
      await ui.contextClick(...at);
      await ui.waitFor(`!!document.querySelector('.menu')`);
      await ui.sleep(300);
    },
    close: async (ui) => ui.escapeAll(2),
  },
];

SCENES.pointeditor = [
  { id: 'noktalar', open: openPointEditor },
  {
    id: 'noktalar-ad-sirali',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.clickText('.ptable__sort', 'Ad');
      await ui.sleep(300);
    },
  },
  {
    id: 'noktalar-ara',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.clickText('.ptable__sort', 'Ad');
      await ui.clickSel('.ptable__search');
      await ui.type('p1');
      await ui.move(2, 2);
      await ui.sleep(400);
    },
  },
  {
    id: 'noktalar-katman',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.clickSel('.ptable__layer');
      await ui.sleep(300);
    },
  },
  // A double click on the second row's Y opens it with its whole value; Enter writes it and opens the row below.
  {
    id: 'noktalar-duzenle',
    open: async (ui) => {
      await openPointEditor(ui);
      const at = await ui.eval(`(() => { const td = document.querySelectorAll('.ptable tbody tr[data-at="1"] td')[2]; const r = td.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      await ui.clickAt(...at);
      await ui.clickAt(...at, { clickCount: 2 });
      await ui.waitFor(`document.activeElement?.classList.contains('ptable__edit')`, 3000);
      const opened = await ui.eval(`document.activeElement.value`);
      if (opened !== '487024') throw new Error(`Y açıldı: ${opened}`);
      await ui.type('487024.5');
      await ui.key('Enter');
      await ui.sleep(300);
      const written = await ui.eval(`window.kentos.doc.get(5).p.x`);
      if (written !== 487024.5) throw new Error(`Y yazılmadı: ${written}`);
      const next = await ui.eval(`document.activeElement?.closest('tr')?.dataset.at`);
      if (next !== '2') throw new Error(`Enter alttaki satıra geçmedi: ${next}`);
      await ui.sleep(300);
    },
  },
  // Satır ekle: two rows typed and written, the third open with the name one more.
  {
    id: 'noktalar-satir-ekle',
    open: async (ui) => {
      await openPointEditor(ui);
      const before = await ui.eval(`window.kentos.doc.size`);
      await ui.clickText('.ptable__btn', 'Satır ekle');
      await ui.waitFor(`document.activeElement?.classList.contains('ptable__edit')`, 3000);
      for (const [text, key] of [['201', 'Tab'], ['487030.25', 'Tab'], ['4420030.5', 'Enter'], ['487031.75', 'Tab'], ['4420031', 'Enter']]) {
        await ui.type(text);
        await ui.key(key);
        await ui.sleep(150);
      }
      const after = await ui.eval(`[window.kentos.doc.size, [...window.kentos.doc.all()].slice(-2).map((e) => e.label)]`);
      if (after[0] !== before + 2 || after[1][0] !== '201' || after[1][1] !== '202') throw new Error(`Satır ekle: ${JSON.stringify(after)}`);
      const draft = await ui.eval(`document.querySelector('.ptable__draft td:nth-child(2)')?.textContent`);
      if (draft !== '203') throw new Error(`Sonraki taslağın adı: ${draft}`);
      await ui.sleep(300);
    },
  },
  // Sil: the first two rows selected (a click, then Shift and a click) go in one step; undone and done again.
  {
    id: 'noktalar-sil',
    open: async (ui) => {
      await openPointEditor(ui);
      const cell = (i) => ui.eval(`(() => { const td = document.querySelectorAll('.ptable tbody tr[data-at="${i}"] td')[1]; const r = td.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      const before = await ui.eval(`window.kentos.doc.size`);
      await ui.clickAt(...(await cell(0)));
      await ui.clickAt(...(await cell(1)), { modifiers: 8 });
      await ui.clickText('.ptable__btn', 'Sil');
      await ui.sleep(300);
      const after = await ui.eval(`[window.kentos.doc.size, document.querySelector('.ptable__count').textContent, document.querySelector('.ptable tbody tr[data-at="0"] td:nth-child(2)').textContent]`);
      if (after[0] !== before - 2 || after[1] !== '34 / 34 nokta' || after[2] !== '103') throw new Error(`Sil: ${JSON.stringify(after)}`);
      const undone = await ui.eval(`[window.kentos.doc.undo(), window.kentos.doc.size]`);
      if (undone[1] !== before) throw new Error(`Sil tek adımda geri alınmadı: ${JSON.stringify(undone)}`);
      await ui.eval(`window.kentos.doc.redo()`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // İşlemler ▾ over the three points selected in the drawing.
  {
    id: 'noktalar-islemler',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.clickText('.ptable__btn', 'İşlemler');
      await ui.waitFor(`document.querySelector('.menu__header')?.textContent === '3 seçili nokta'`, 3000);
      await ui.sleep(300);
    },
  },
  // Yeniden adlandır over the first six rows (a click, Shift and a click), P. typed: what would change.
  {
    id: 'noktalar-yeniden-adlandir',
    open: async (ui) => {
      await openPointEditor(ui);
      await pickRows(ui, 0, 5);
      await ui.clickText('.ptable__btn', 'İşlemler');
      await ui.clickText('.menu__title', 'Yeniden adlandır…');
      await ui.waitFor(`document.activeElement?.getAttribute('aria-label') === 'Önek'`, 3000);
      await ui.type('P.');
      await ui.sleep(200);
      const said = await ui.eval(`document.querySelector('.dialog--point-batch .io-summary').textContent`);
      if (said !== '6 noktanın adı değişecek; ilki “101” → “P.101”.') throw new Error(`Yeniden adlandır: ${said}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Sıralı numara ver over the first six rows from 201, written with Enter: one step, the table's new names.
  {
    id: 'noktalar-sirali-numara',
    open: async (ui) => {
      await openPointEditor(ui);
      await pickRows(ui, 0, 5);
      await ui.clickText('.ptable__btn', 'İşlemler');
      await ui.clickText('.menu__title', 'Sıralı numara ver…');
      await ui.waitFor(`document.activeElement?.getAttribute('aria-label') === 'Başlangıç adı'`, 3000);
      const start = await ui.eval(`document.activeElement.value`);
      if (start !== '101') throw new Error(`Başlangıç adı: ${start}`);
      await ui.type('201');
      await ui.key('Enter');
      await ui.sleep(300);
      const names = await ui.eval(`[...document.querySelectorAll('.ptable tbody tr[data-at] td:nth-child(2)')].slice(0, 7).map((td) => td.textContent)`);
      if (JSON.stringify(names) !== JSON.stringify(['201', '202', '203', '204', '205', '206', '101/1'])) throw new Error(`Sıralı numara: ${JSON.stringify(names)}`);
      const step = await ui.eval(`(() => { const d = window.kentos.doc; const s = d.undo(); d.redo(); return s; })()`);
      if (step !== 'Sıralı numara ver') throw new Error(`Adım: ${step}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // A right click on a row not selected: its menu at the pointer, named after it; the selection stays.
  {
    id: 'noktalar-sag-tik',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.contextClick(...(await rowCell(ui, 4, 1)));
      await ui.waitFor(`document.querySelector('.menu__header')?.textContent === 'Nokta 105'`, 3000);
      const selected = await ui.eval(`window.kentos.selection.size`);
      if (selected !== 3) throw new Error(`Sağ tık seçimi değiştirdi: ${selected}`);
      await ui.sleep(300);
    },
  },
  // Katmana taşı from that menu: Kot chosen and written; the rows show their new layer.
  {
    id: 'noktalar-katmana-tasi',
    open: async (ui) => {
      await openPointEditor(ui);
      await pickRows(ui, 0, 1);
      await ui.contextClick(...(await rowCell(ui, 1, 1)));
      await ui.clickText('.menu__title', 'Katmana taşı…');
      await ui.clickSel('.dialog--point-batch .dropdown');
      await ui.clickText('.menu__label', 'Kot');
      await ui.clickText('.dialog--point-batch .btn--primary', 'Uygula');
      await ui.sleep(300);
      const layers = await ui.eval(`[...document.querySelectorAll('.ptable tbody tr[data-at] td:nth-child(7)')].slice(0, 3).map((td) => td.textContent)`);
      if (JSON.stringify(layers) !== JSON.stringify(['Kot', 'Kot', 'Nokta'])) throw new Error(`Katmana taşı: ${JSON.stringify(layers)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Çift noktaları ayıkla over every row (nothing selected): Aynı yer, 1 mm; Ortalaması chosen, the count follows.
  {
    id: 'noktalar-cift-ayikla',
    open: async (ui) => {
      await openDedupe(ui);
      await ui.clickText('.dialog--point-batch .seg__opt', 'Ortalaması');
      const said = await ui.eval(`document.querySelector('.dialog--point-batch .io-summary').textContent`);
      if (said !== '2 grupta 4 nokta; 2 nokta silinecek, 1 nokta ortalamaya taşınacak.') throw new Error(`Çift noktaları ayıkla: ${said}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Çiftleri göster: the table shows the two groups, Sıra their numbers, the chip over them.
  {
    id: 'noktalar-ciftler',
    open: async (ui) => {
      await openDedupe(ui);
      await ui.clickText('.dialog--point-batch .btn', 'Çiftleri göster');
      await ui.sleep(300);
      const rows = await ui.eval(`[...document.querySelectorAll('.ptable tbody tr[data-at]')].map((tr) => tr.children[0].textContent + ' ' + tr.children[1].textContent)`);
      if (JSON.stringify(rows) !== JSON.stringify(['1 103', '1 103', '2 105', '2 S9'])) throw new Error(`Çiftleri göster: ${JSON.stringify(rows)}`);
      const chip = await ui.eval(`document.querySelector('.ptable__chip').textContent`);
      if (chip !== 'Çiftler: 2 grup') throw new Error(`Çip: ${chip}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Ayıkla over the groups shown, Ortalaması: one step; the extras go, the table shows its query again.
  {
    id: 'noktalar-ayiklandi',
    open: async (ui) => {
      await openDedupe(ui);
      await ui.clickText('.dialog--point-batch .btn', 'Çiftleri göster');
      await ui.clickText('.ptable__btn', 'İşlemler');
      await ui.clickText('.menu__title', 'Çift noktaları ayıkla…');
      await ui.waitFor(`!!document.querySelector('.dialog--point-batch')`, 3000);
      await ui.clickText('.dialog--point-batch .seg__opt', 'Ortalaması');
      await ui.clickText('.dialog--point-batch .btn--primary', 'Ayıkla');
      await ui.sleep(300);
      const after = await ui.eval(`[window.kentos.doc.size, document.querySelector('.ptable__count').textContent, document.querySelector('.ptable__chip').hidden]`);
      if (JSON.stringify(after) !== JSON.stringify([37, '34 / 34 nokta', true])) throw new Error(`Ayıkla: ${JSON.stringify(after)}`);
      const step = await ui.eval(`(() => { const d = window.kentos.doc; const s = d.undo(); d.redo(); return s; })()`);
      if (step !== 'Çift noktaları ayıkla') throw new Error(`Adım: ${step}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Dışa aktar with nothing selected, the table sorted by Ad: the coordinate list window offers the table's rows first.
  {
    id: 'noktalar-disa-aktar',
    open: async (ui) => {
      await openPointEditor(ui);
      await ui.eval(`window.kentos.selection.clear()`);
      await ui.clickText('.ptable__sort', 'Ad');
      await ui.clickText('.ptable__btn', 'İşlemler');
      await ui.clickText('.menu__title', 'Dışa aktar…');
      await ui.waitFor(`!!document.querySelector('.dialog--io .seg')`, 10000);
      const seen = await ui.eval(`({ scope: document.querySelector('.dialog--io .seg__opt[aria-checked="true"]').textContent, options: [...document.querySelectorAll('.dialog--io .seg')[0].querySelectorAll('.seg__opt')].map((o) => o.textContent), said: document.querySelector('.dialog--io .io-summary').textContent })`);
      if (seen.scope !== 'Tablodaki (36)' || seen.options.length !== 3 || !seen.said.startsWith('36 nokta yazılacak')) throw new Error(`Dışa aktar: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Dışa aktar over two selected rows, written: the file holds them in the table's order (sorted by Ad, descending).
  {
    id: 'noktalar-disa-aktarildi',
    open: async (ui) => {
      await openPointEditor(ui);
      await fakeFiles(ui, 'kullanilmaz.ncn', '');
      // Nothing selected first, so the table stays at its top: clickText's scrollIntoView on the sticky header of a
      // scrolled table would scroll the whole page.
      await ui.eval(`window.kentos.selection.clear()`);
      await ui.sleep(200);
      await ui.clickText('.ptable__sort', 'Ad');
      await ui.clickText('.ptable__sort', 'Ad');
      await pickRows(ui, 0, 1);
      const shown = await ui.eval(`[...document.querySelectorAll('.ptable tbody tr[data-at] td:nth-child(2)')].slice(0, 2).map((td) => td.textContent)`);
      await ui.contextClick(...(await rowCell(ui, 0, 1)));
      await ui.clickText('.menu__title', 'Dışa aktar…');
      await ui.waitFor(`!!document.querySelector('.dialog--io .seg')`, 10000);
      const scope = await ui.eval(`document.querySelector('.dialog--io .seg__opt[aria-checked="true"]').textContent`);
      if (scope !== 'Seçili satırlar (2)') throw new Error(`Kapsam: ${scope}`);
      await ui.clickText('.dialog--io .btn--primary', 'Dışa aktar');
      await ui.waitFor(`!!window.__io.written`, 10000);
      const names = await ui.eval(`window.__io.written.text.trim().split(/\\r?\\n/).map((l) => l.split(' ')[0])`);
      if (JSON.stringify(names) !== JSON.stringify(shown)) throw new Error(`Yazılan sıra: ${JSON.stringify(names)}, tablo: ${JSON.stringify(shown)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // İçe aktar from İşlemler ▾ with 101 and 105 measured anew and 201: in, then Çift noktaları ayıkla by Aynı ad.
  { id: 'noktalar-ice-aktar', open: importAgain },
  // Sonuncusu keeps the file's: the drawing's 101 and 105 go in one step, after the import's.
  {
    id: 'noktalar-ice-aktarildi',
    open: async (ui) => {
      await importAgain(ui);
      await ui.clickText('.dialog--point-batch .seg__opt', 'Sonuncusu');
      await ui.clickText('.dialog--point-batch .btn--primary', 'Ayıkla');
      await ui.sleep(300);
      const after = await ui.eval(`(() => {
        const k = window.kentos;
        const named = (n) => [...k.doc.all()].filter((e) => e.kind === 'point' && e.label === n).map((e) => [e.p.x, e.p.y, e.z ?? null]);
        return { count: document.querySelector('.ptable__count').textContent, p101: named('101'), p105: named('105'), p201: named('201') };
      })()`);
      const want = { count: '37 / 37 nokta', p101: [[487000.02, 4419999.99, 100.3]], p105: [[487024.01, 4420018, 101.8]], p201: [[487050, 4420010, 103]] };
      if (JSON.stringify(after) !== JSON.stringify(want)) throw new Error(`Ayıkla: ${JSON.stringify(after)}`);
      const steps = await ui.eval(`(() => { const d = window.kentos.doc; const a = d.undo(); const b = d.undo(); d.redo(); d.redo(); return [a, b]; })()`);
      if (JSON.stringify(steps) !== JSON.stringify(['Çift noktaları ayıkla', 'Koordinat listesi: olcum.ncn'])) throw new Error(`Adımlar: ${JSON.stringify(steps)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
].map((s) => ({
  close: async (ui) =>
    (await ui.escapeAll(2),
    await ui.eval(`(() => { const k = window.kentos; if (window.__io?.original) k.files.picker = window.__io.original; k.selection.clear(); k.ui.bottomExpanded.set(false); })()`)),
  ...s,
}));

// Text extras (docs/adr/0145) on fixtures/interaction/v1/text-extras.kcad, the scene the desktop's
// `labels::text_extras_screens` draws: the twelve alignments at their marked points, a turned centred text, width
// factors 0.6, 1 and 1.5, a masked text next to one without over a hatch and a line; the whole, then closer in.
const TEXT_EXTRAS = readFileSync(new URL('../../../../fixtures/interaction/v1/text-extras.kcad', import.meta.url), 'utf8');
const openTextExtras = async (ui) => {
  await ui.eval(`(async () => {
    const k = window.kentos;
    k.files.ask = async () => 'drop';
    if (!(await k.files.load(${JSON.stringify(TEXT_EXTRAS)}, null))) throw new Error('text-extras.kcad did not load');
    k.selection.clear();
    k.view.zoomExtents();
    // A little room round the drawing.
    const c = k.view.camera;
    c.scale = c.scale * 0.85;
    c.panBy(0, 0);
  })()`);
  await ui.sleep(600);
};
// Leaders (docs/adr/0146) on fixtures/interaction/v1/leaders.kcad, the scene the desktop's `labels::leader_screens`
// draws: the four arrowheads, one without a note, one masked, one turned and one in a block; the second selected so
// that Öznitelikler shows it.
const LEADERS = readFileSync(new URL('../../../../fixtures/interaction/v1/leaders.kcad', import.meta.url), 'utf8');
// Köşe tablosu (docs/adr/0172) on fixtures/interaction/v1/vertex-table.kcad, the scenes the desktop's
// `vertices::tests::screens` draws: the parcel's table with a row selected and its vertex ringed; a Y typed in place
// (Enter writes it and goes down); a radius shorter than half the chord refused with the cell open; the road's arcs;
// two objects selected (the first shown, not written).
const VERTEX_TABLE = readFileSync(new URL('../../../../fixtures/interaction/v1/vertex-table.kcad', import.meta.url), 'utf8');
const openVertexTable = async (ui, ids = [1]) => {
  await ui.eval(`(async () => {
    const k = window.kentos;
    k.files.ask = async () => 'drop';
    if (!(await k.files.load(${JSON.stringify(VERTEX_TABLE)}, null))) throw new Error('vertex-table.kcad did not load');
    k.ui.bottomHeight.set(300);
    k.selection.set(${JSON.stringify(ids)});
    k.commands.execute('view.coords');
    k.view.zoomExtents();
    // A little room round the drawing.
    const c = k.view.camera;
    c.scale = c.scale * 0.8;
    c.panBy(0, 0);
  })()`);
  await ui.sleep(700);
};
/** The centre of the table's row `i`, cell `j`. */
const vertexCell = (ui, i, j) =>
  ui.eval(`(() => { const td = document.querySelectorAll('.vtable tbody tr[data-at="${i}"] td')[${j}]; const r = td.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
/** A double click on row `i`'s cell `j` opens its editor with `whole`. */
const editVertexCell = async (ui, i, j, whole) => {
  const at = await vertexCell(ui, i, j);
  await ui.clickAt(...at);
  await ui.clickAt(...at, { clickCount: 2 });
  await ui.waitFor(`document.activeElement?.classList.contains('ptable__edit')`, 3000);
  const opened = await ui.eval(`document.activeElement.value`);
  if (whole !== undefined && opened !== whole) throw new Error(`hücre açıldı: ${opened}`);
};
SCENES.vertextable = [
  {
    id: 'kose-tablosu',
    open: async (ui) => {
      await openVertexTable(ui);
      await ui.clickAt(...(await vertexCell(ui, 2, 2)));
      await ui.sleep(300);
      const ringed = await ui.eval(`window.kentos.selection.vertices.value.length`);
      if (ringed !== 1) throw new Error(`vurgulanan köşe: ${ringed}`);
    },
  },
  {
    id: 'kose-tablosu-duzenle',
    open: async (ui) => {
      await openVertexTable(ui);
      // Köşe, Halka, Y: the second row's Y.
      await editVertexCell(ui, 1, 2, '486760');
      await ui.type('486761.5');
      await ui.key('Enter');
      await ui.sleep(300);
      const written = await ui.eval(`window.kentos.doc.get(1).pts[1].x`);
      if (written !== 486761.5) throw new Error(`Y yazılmadı: ${written}`);
      const next = await ui.eval(`document.activeElement?.closest('tr')?.dataset.at`);
      if (next !== '2') throw new Error(`Enter alttaki satıra geçmedi: ${next}`);
      await ui.sleep(200);
    },
  },
  {
    id: 'kose-tablosu-yaricap',
    open: async (ui) => {
      await openVertexTable(ui);
      // Köşe, Halka, Y, X, Z, Yarıçap: the first edge, 60.2 m long.
      await editVertexCell(ui, 0, 5, '');
      await ui.type('10');
      await ui.key('Enter');
      await ui.sleep(300);
      const open = await ui.eval(`document.activeElement?.classList.contains('ptable__edit')`);
      if (!open) throw new Error('reddedilen yarıçapın hücresi kapandı');
    },
    close: (ui) => ui.key('Escape'),
  },
  {
    // Satır ekle under the first row: Y, Tab, X, Enter writes the vertex; the next draft opens under it.
    id: 'kose-tablosu-satir-ekle',
    open: async (ui) => {
      await openVertexTable(ui);
      await ui.clickAt(...(await vertexCell(ui, 0, 2)));
      await ui.clickText('.vtable .ptable__btn', 'Satır ekle');
      await ui.waitFor(`document.activeElement?.classList.contains('ptable__edit')`, 3000);
      for (const [text, key] of [['486730', 'Tab'], ['4420190', 'Enter']]) {
        await ui.type(text);
        await ui.key(key);
        await ui.sleep(150);
      }
      const n = await ui.eval(`window.kentos.doc.get(1).pts.length`);
      if (n !== 6) throw new Error(`Satır ekle: ${n} köşe`);
      const draft = await ui.eval(`document.querySelector('.vtable .ptable__draft')?.dataset.at`);
      if (draft !== '2') throw new Error(`sonraki taslak: ${draft}`);
      await ui.sleep(200);
    },
    close: (ui) => ui.key('Escape'),
  },
  { id: 'kose-tablosu-yol', open: (ui) => openVertexTable(ui, [2]) },
  { id: 'kose-tablosu-coklu', open: (ui) => openVertexTable(ui, [1, 2]) },
];
// Vektör oturtma (docs/adr/0156 §7) on fixtures/interaction/v1/vector-fit.kcad, the scenes the desktop's
// `calc::fit::tests::screens` draws: a parcel surveyed in a local system and the same points measured in TUREF (P5 with a
// 15 cm blunder); Adla eşle fills the pairs, P5 shows as the worst residual and is left out, Uygula fits the local layer.
const VECTOR_FIT = readFileSync(new URL('../../../../fixtures/interaction/v1/vector-fit.kcad', import.meta.url), 'utf8');
const openFitScene = async (ui) => {
  await ui.eval(`(async () => {
    const k = window.kentos;
    k.files.ask = async () => 'drop';
    if (!(await k.files.load(${JSON.stringify(VECTOR_FIT)}, null))) throw new Error('vector-fit.kcad did not load');
    k.view.zoomExtents();
    const c = k.view.camera;
    c.scale = c.scale * 0.85;
    c.panBy(0, 0);
    k.selection.clear();
  })()`);
  await ui.sleep(300);
  await ui.eval(`window.kentos.commands.execute('transform.fit')`);
  await ui.waitFor(`!!document.querySelector('.dialog--fit')`, 8000);
  await ui.sleep(300);
};
/** Adla eşle with the scene's two layers, Helmert, the objects of the local layer. */
const matchFit = async (ui) => {
  await openFitScene(ui);
  // What is typed stays for the session: a scene before may have left Parametrelerle on.
  await ui.clickText('.dialog--fit .seg__opt', 'Kontrol noktaları');
  await ui.clickText('.dialog--fit .seg__opt', 'Helmert');
  await ui.eval(`(() => {
    const set = (key, value) => { const s = document.querySelector('.dialog--fit select[data-key="' + key + '"]'); s.value = value; s.dispatchEvent(new Event('change')); };
    set('source', 'yerel');
    set('target', 'tm');
  })()`);
  await ui.clickText('.dialog--fit .btn', 'Eşle');
  await ui.sleep(200);
  await ui.clickText('.dialog--fit .seg__opt', 'Katman');
  await ui.eval(`(() => { const s = document.querySelector('.dialog--fit select[data-key="layer"]'); s.value = 'yerel'; s.dispatchEvent(new Event('change')); })()`);
  await ui.sleep(200);
  const seen = await ui.eval(`({ rows: [...document.querySelectorAll('.dialog--fit .calc-grid tbody tr')].map((tr) => tr.children[2].querySelector('input')?.value), worst: document.querySelector('.dialog--fit tr[data-mark="worst"] input[data-key="name"]')?.value ?? null, said: document.querySelector('.dialog--fit .io-summary').textContent })`);
  if (JSON.stringify(seen.rows) !== JSON.stringify(['P1', 'P2', 'P3', 'P4', 'P5', 'P6']) || seen.worst !== 'P5' || !seen.said.startsWith('m0 = ±')) throw new Error(`Adla eşle: ${JSON.stringify(seen)}`);
};
/** Parametrelerle over the matched scene: the base and the numbers typed into their fields. */
const openParams = async (ui, base, numbers) => {
  await matchFit(ui);
  await ui.clickText('.dialog--fit .seg__opt', 'Parametrelerle');
  await ui.sleep(200);
  await ui.eval(`(() => {
    const set = (key, value) => { const i = document.querySelector('.dialog--fit input[data-key="' + key + '"]'); i.value = value; i.dispatchEvent(new Event('input')); };
    set('base', ${JSON.stringify(base)});
    // Every number, the scene's or its empty value: what an earlier scene typed stays for the session.
    for (const [k, v] of Object.entries(${JSON.stringify({ scaleY: '1', scaleX: '1', rotation: '0', shiftY: '0', shiftX: '0', ...numbers })})) set(k, v);
  })()`);
  await ui.sleep(200);
};
/**
 * The two steps of a fit (docs/adr/0158): Oturt by Helmert with P5 left out, then the window again over the moved
 * local layer: Adla eşle, Kauçuk levha, P5 left out again and P3 a fixed point (Sabit). P3's place before the sheet.
 */
const sheetAfterHelmert = async (ui) => {
  await matchFit(ui);
  await ui.eval(`document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[4].querySelector('input[type="checkbox"]').click()`);
  await ui.sleep(200);
  await ui.clickText('.dialog--fit .btn--primary', 'Uygula');
  await ui.sleep(300);
  await ui.eval(`window.kentos.commands.execute('transform.fit')`);
  await ui.waitFor(`!!document.querySelector('.dialog--fit')`, 8000);
  await ui.sleep(200);
  await ui.clickText('.dialog--fit .seg__opt', 'Kauçuk levha');
  await ui.clickText('.dialog--fit .btn', 'Eşle');
  await ui.sleep(200);
  await ui.eval(`document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[4].querySelector('input[type="checkbox"]').click()`);
  await ui.sleep(100);
  await ui.eval(`document.querySelector('.dialog--fit button[aria-label="3. satırı sabit yap"]').click()`);
  await ui.sleep(200);
  const seen = await ui.eval(`({ said: document.querySelector('.dialog--fit .io-summary').textContent, p3: [...window.kentos.doc.all()].find((e) => e.kind === 'point' && e.layerId === 'yerel' && e.label === 'P3')?.p, target: [...document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[2].querySelectorAll('input.calc-grid__cell')].map((i) => i.value) })`);
  if (!seen.said.startsWith('5 bağ (1 sabit nokta); levha her bağdan tam geçer.') || seen.target[1] !== seen.target[3] || seen.target[2] !== seen.target[4]) throw new Error(`Kauçuk levha, Sabit: ${JSON.stringify(seen)}`);
  return seen;
};
SCENES.vectorfit = [
  { id: 'oturt', open: openFitScene },
  { id: 'oturt-adla-eslendi', open: async (ui) => (await matchFit(ui), await ui.move(2, 2), await ui.sleep(300)) },
  // The window's foot: the objects it applies to (a layer) and Kopya, the body scrolled to its end.
  {
    id: 'oturt-uygulama',
    open: async (ui) => {
      await matchFit(ui);
      await ui.eval(`(() => { const b = document.querySelector('.dialog--fit .dialog__body'); b.scrollTop = b.scrollHeight; })()`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // P5 left out: m0 falls to millimetres, its row fades, its residual still shown.
  {
    id: 'oturt-p5-cikti',
    open: async (ui) => {
      await matchFit(ui);
      const before = await ui.eval(`document.querySelector('.dialog--fit .io-summary').textContent`);
      await ui.eval(`document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[4].querySelector('input[type="checkbox"]').click()`);
      await ui.sleep(200);
      const after = await ui.eval(`({ said: document.querySelector('.dialog--fit .io-summary').textContent, off: document.querySelectorAll('.dialog--fit tr[data-mark="off"]').length, v5: document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[4].querySelector('td[data-key="v"]').textContent })`);
      const m0 = (t) => Number(/m0 = ±([\d.,]+) mm/.exec(t)?.[1].replace(',', '.'));
      if (!(m0(after.said) < 5 && m0(before) > 20 && after.off === 1 && Number(after.v5.replace(',', '.')) > 100)) throw new Error(`P5 çıktı: ${before} | ${JSON.stringify(after)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Afin: its own parameters.
  {
    id: 'oturt-afin',
    open: async (ui) => {
      await matchFit(ui);
      await ui.clickText('.dialog--fit .seg__opt', 'Afin');
      await ui.sleep(200);
      const said = await ui.eval(`document.querySelector('.dialog--fit .io-summary').textContent`);
      if (!said.includes('X ölçeği') || !said.includes('kayma')) throw new Error(`Afin: ${said}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Uygula with P5 left out: the local layer lands on the TUREF points; one step, Oturt; the curves and the text said.
  {
    id: 'oturt-uygulandi',
    open: async (ui) => {
      await matchFit(ui);
      await ui.eval(`document.querySelectorAll('.dialog--fit .calc-grid tbody tr')[4].querySelector('input[type="checkbox"]').click()`);
      await ui.sleep(200);
      await ui.clickText('.dialog--fit .btn--primary', 'Uygula');
      await ui.sleep(300);
      const after = await ui.eval(`(() => {
        const k = window.kentos;
        const named = (layer, n) => [...k.doc.all()].find((e) => e.kind === 'point' && e.layerId === layer && e.label === n)?.p;
        const gap = (n) => { const a = named('yerel', n), b = named('tm', n); return Math.hypot(a.x - b.x, a.y - b.y); };
        const step = (() => { const s = k.doc.undo(); k.doc.redo(); return s; })();
        const circle = [...k.doc.all()].find((e) => e.layerId === 'yerel' && e.kind === 'circle');
        return { p1: gap('P1'), p5: gap('P5'), step, circle: !!circle, open: !!document.querySelector('.dialog--fit') };
      })()`);
      if (!(after.p1 < 0.01 && after.p5 > 0.1 && after.step === 'Oturt' && after.circle && !after.open)) throw new Error(`Uygula: ${JSON.stringify(after)}`);
      await ui.eval(`(() => { const k = window.kentos; k.view.zoomExtents(); const c = k.view.camera; c.scale = c.scale * 0.85; c.panBy(0, 0); })()`);
      await ui.move(2, 2);
      await ui.sleep(400);
    },
  },
  // Parametrelerle: about P1, the scales of the solution with P5 left out, its turn, no shift.
  {
    id: 'oturt-parametre',
    open: async (ui) => {
      await openParams(ui, 'P1', { scaleY: '1.0002', scaleX: '0.9997', rotation: '0.5' });
      const said = await ui.eval(`document.querySelector('.dialog--fit .io-summary').textContent`);
      if (!said.includes('Y ölçeği 1.0002, X ölçeği 0.9997') || !said.includes('Ölçekler farklı')) throw new Error(`Parametrelerle: ${said}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // What is wrong, in the fields' order; Uygula and the report off.
  {
    id: 'oturt-parametre-uyari',
    open: async (ui) => {
      await openParams(ui, 'P9', { scaleY: '0', shiftX: '12,5x' });
      const seen = await ui.eval(`({ warns: document.querySelectorAll('.dialog--fit .io-summary [data-kind="warn"], .dialog--fit .io-summary .summary__line--warn').length, said: document.querySelector('.dialog--fit .io-summary').textContent, apply: document.querySelector('.dialog--fit .btn--primary').disabled })`);
      if (!seen.said.includes('Y ölçeği sıfır olamaz') || !seen.said.includes('Öteleme ΔX sayı olmalı') || !seen.apply) throw new Error(`Parametrelerle uyarı: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Uygula: about P1, east 1.5 times; one step, Oturt; the circle an ellipse; the view on the local layer.
  {
    id: 'oturt-parametre-uygulandi',
    open: async (ui) => {
      await openParams(ui, 'P1', { scaleY: '1.5' });
      await ui.clickText('.dialog--fit .btn--primary', 'Uygula');
      await ui.sleep(300);
      const after = await ui.eval(`(() => {
        const k = window.kentos;
        const p2 = [...k.doc.all()].find((e) => e.kind === 'point' && e.layerId === 'yerel' && e.label === 'P2')?.p;
        const step = (() => { const s = k.doc.undo(); k.doc.redo(); return s; })();
        const ellipse = [...k.doc.all()].some((e) => e.layerId === 'yerel' && e.kind === 'ellipse');
        k.selection.set([...k.doc.all()].filter((e) => e.layerId === 'yerel').map((e) => e.id));
        k.commands.execute('view.zoomSelection');
        k.selection.clear();
        return { p2, step, ellipse, open: !!document.querySelector('.dialog--fit') };
      })()`);
      if (!(Math.abs(after.p2.x - 1093.75) < 1e-9 && Math.abs(after.p2.y - 2003.25) < 1e-9 && after.step === 'Oturt' && after.ellipse && !after.open)) throw new Error(`Parametrelerle Uygula: ${JSON.stringify(after)}`);
      await ui.move(2, 2);
      await ui.sleep(400);
    },
  },
  // Kauçuk levha (docs/adr/0158 §5) over the matched pairs: the links met exactly, Helmert's residuals the local corrections, P5's the largest; Sabit on every row.
  {
    id: 'oturt-levha',
    open: async (ui) => {
      await matchFit(ui);
      await ui.clickText('.dialog--fit .seg__opt', 'Kauçuk levha');
      await ui.sleep(200);
      const seen = await ui.eval(`({ said: document.querySelector('.dialog--fit .io-summary').textContent, fixes: document.querySelectorAll('.dialog--fit button[aria-label$="satırı sabit yap"]').length, apply: document.querySelector('.dialog--fit .btn--primary').disabled })`);
      if (!seen.said.startsWith('6 bağ (0 sabit nokta); levha her bağdan tam geçer.') || !seen.said.includes(': P5;') || seen.fixes !== 6 || seen.apply) throw new Error(`Kauçuk levha: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // After Oturt (Helmert, P5 left out): the window again, Adla eşle, Kauçuk levha; P5 left out again, P3 held where Helmert put it.
  {
    id: 'oturt-levha-sabit',
    open: async (ui) => {
      await sheetAfterHelmert(ui);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Uygula: the links met exactly, P3 stays; one step, Kauçuk levha; the view on the parcel.
  {
    id: 'oturt-levha-uygulandi',
    open: async (ui) => {
      const before = await sheetAfterHelmert(ui);
      await ui.clickText('.dialog--fit .btn--primary', 'Uygula');
      await ui.sleep(300);
      const after = await ui.eval(`(() => {
        const k = window.kentos;
        const named = (layer, n) => [...k.doc.all()].find((e) => e.kind === 'point' && e.layerId === layer && e.label === n)?.p;
        const gap = (n) => { const a = named('yerel', n), b = named('tm', n); return Math.hypot(a.x - b.x, a.y - b.y); };
        const step = (() => { const s = k.doc.undo(); k.doc.redo(); return s; })();
        const p3 = named('yerel', 'P3');
        k.selection.set([...k.doc.all()].filter((e) => e.layerId === 'yerel').map((e) => e.id));
        k.commands.execute('view.zoomSelection');
        k.selection.clear();
        return { gaps: ['P1', 'P2', 'P4', 'P6'].map(gap), p3, step, open: !!document.querySelector('.dialog--fit') };
      })()`);
      const held = Math.hypot(after.p3.x - before.p3.x, after.p3.y - before.p3.y);
      if (!(after.gaps.every((g) => g < 1e-6) && held < 1e-6 && after.step === 'Kauçuk levha' && !after.open)) throw new Error(`Kauçuk levha Uygula: ${JSON.stringify({ ...after, held })}`);
      await ui.move(2, 2);
      await ui.sleep(400);
    },
  },
  // Two links left: the least is three; Uygula off.
  {
    id: 'oturt-levha-uyari',
    open: async (ui) => {
      await matchFit(ui);
      await ui.clickText('.dialog--fit .seg__opt', 'Kauçuk levha');
      await ui.eval(`(() => { const rows = document.querySelectorAll('.dialog--fit .calc-grid tbody tr'); for (const r of [1, 2, 3, 4]) rows[r].querySelector('input[type="checkbox"]').click(); })()`);
      await ui.sleep(200);
      const seen = await ui.eval(`({ said: document.querySelector('.dialog--fit .io-summary').textContent, apply: document.querySelector('.dialog--fit .btn--primary').disabled })`);
      if (!seen.said.startsWith('Kauçuk levha için en az 3 kullanılan bağ gerekir; şimdi 2.') || !seen.apply) throw new Error(`Kauçuk levha uyarı: ${JSON.stringify(seen)}`);
      // The summary and Uygula in view at the small size too.
      await ui.eval(`(() => { const b = document.querySelector('.dialog--fit .dialog__body'); b.scrollTop = b.scrollHeight; })()`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.selection.clear()`)), ...s }));

// Kenar eşleme (docs/adr/0159 §9) on fixtures/interaction/v1/edgematch.kcad, the scenes the desktop's
// `calc::edgematch::tests::screens` draws: two sheets digitised apart, their roads, a building line and a fence reaching
// the shared edge a few centimetres short, over or beside their continuation; a street crossing the edge at right angles,
// two roads meeting at the edge, a parcel. What is typed stays for the session: each scene sets every choice itself.
const EDGEMATCH = readFileSync(new URL('../../../../fixtures/interaction/v1/edgematch.kcad', import.meta.url), 'utf8');
const EDGE_X = 487200;
const EDGE_Y = 4420000;
const openEdgeScene = async (ui) => {
  await ui.eval(`(async () => {
    const k = window.kentos;
    k.files.ask = async () => 'drop';
    if (!(await k.files.load(${JSON.stringify(EDGEMATCH)}, null))) throw new Error('edgematch.kcad did not load');
    k.view.zoomExtents();
    const c = k.view.camera;
    c.scale = c.scale * 0.85;
    c.panBy(0, 0);
    k.selection.clear();
  })()`);
  await ui.sleep(300);
  await ui.eval(`window.kentos.commands.execute('transform.edgematch')`);
  await ui.waitFor(`!!document.querySelector('.dialog--edgematch')`, 8000);
  await ui.sleep(200);
  // The scene's choices: Pafta 1 and Pafta 2, no border, the first values.
  await ui.clickText('.dialog--edgematch .seg__opt', 'Katman');
  await ui.eval(`(() => {
    const dlg = document.querySelector('.dialog--edgematch');
    const set = (key, value) => { const s = dlg.querySelector('select[data-key="' + key + '"]'); s.value = value; s.dispatchEvent(new Event('change')); };
    set('source', 'pafta1');
    set('adjacent', 'pafta2');
    set('key', '');
    const clear = dlg.querySelector('[aria-label="Sınırı kaldır"]');
    if (clear && !clear.disabled) clear.click();
    for (const [key, value] of [['distance', '0.5'], ['angle', '30']]) { const i = dlg.querySelector('input[data-key="' + key + '"]'); i.value = value; i.dispatchEvent(new Event('input')); }
  })()`);
  await ui.clickText('.dialog--edgematch .seg__opt', 'Komşunun ucunda');
  await ui.clickText('.dialog--edgematch .seg__opt', 'Ucu taşı');
  await ui.sleep(200);
};
/** Sınır shown on the drawing: the sheets' edge, past the frames' corners so that only it lies under the pointer; Enter keeps it. */
const pickEdge = async (ui) => {
  await ui.clickSel('.dialog--edgematch [aria-label="Sınırı çizimden seç"]');
  await ui.sleep(200);
  await ui.clickAt(...(await ui.eval(PAGE_AT(EDGE_X, EDGE_Y + 152.5))));
  await ui.sleep(100);
  await ui.key('Enter');
  await ui.waitFor(`!!document.querySelector('.dialog--edgematch')`, 8000);
  await ui.sleep(200);
};
const edgeSaid = (ui) => ui.eval(`document.querySelector('.dialog--edgematch .io-summary').textContent`);
SCENES.edgematch = [
  // The links found: four continuations, the fence that turns north unmatched, the junction and the parcel said.
  {
    id: 'kenar',
    open: async (ui) => {
      await openEdgeScene(ui);
      const said = await edgeSaid(ui);
      const rows = await ui.eval(`document.querySelectorAll('.dialog--edgematch .calc-grid tbody tr').length`);
      if (rows !== 4 || !said.startsWith('4 bağ bulundu; 4 bağ kullanılacak.') || !said.includes('1 uç eşsiz kaldı') || !said.includes('2 kavşak ucu eşlenmedi') || !said.includes('1 nesne katılmadı'))
        throw new Error(`Kenar eşleme: ${rows} | ${said}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Sınır picked on the drawing; the ends meet on it, the shift fading along the lines.
  {
    id: 'kenar-sinir',
    open: async (ui) => {
      await openEdgeScene(ui);
      await pickEdge(ui);
      await ui.clickText('.dialog--edgematch .seg__opt', 'Sınırda');
      await ui.clickText('.dialog--edgematch .seg__opt', 'Köşeleri ayarla');
      await ui.sleep(200);
      const seen = await ui.eval(`({ border: document.querySelector('.dialog--edgematch .calc-border__name').textContent, said: document.querySelector('.dialog--edgematch .io-summary').textContent, apply: document.querySelector('.dialog--edgematch .btn--primary').disabled })`);
      if (!seen.border.startsWith('Pafta sınırı: Pafta kenarı') || !seen.said.startsWith('4 bağ bulundu') || !seen.said.includes('sınıra yakın') || seen.apply) throw new Error(`Kenar eşleme, sınır: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
  // Göster: the window steps aside, the link's two lines selected, the view on the link; the prompt says how to come back.
  {
    id: 'kenar-goster',
    open: async (ui) => {
      await openEdgeScene(ui);
      await ui.clickSel('.dialog--edgematch .calc-grid tbody tr:nth-child(2) [aria-label="2. bağı çizimde göster"]');
      await ui.sleep(400);
      const seen = await ui.eval(`({ open: !!document.querySelector('.dialog--edgematch'), selected: window.kentos.selection.size, tool: window.kentos.tools.active?.id ?? null })`);
      if (seen.open || seen.selected !== 2 || seen.tool !== 'look') throw new Error(`Kenar eşleme, göster: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
    close: async (ui) => {
      await ui.key('Escape');
      await ui.waitFor(`!!document.querySelector('.dialog--edgematch')`, 8000);
      const back = await ui.eval(`window.kentos.selection.size`);
      if (back !== 0) throw new Error(`Kenar eşleme, göster: the selection came back as ${back}`);
      await ui.escapeAll(2);
    },
  },
  // Uygula on the border: every used link meets on the edge in one step (Kenar eşle); the lines put right selected.
  {
    id: 'kenar-uygulandi',
    open: async (ui) => {
      await openEdgeScene(ui);
      await pickEdge(ui);
      await ui.clickText('.dialog--edgematch .seg__opt', 'Sınırda');
      await ui.clickText('.dialog--edgematch .seg__opt', 'Köşeleri ayarla');
      await ui.clickText('.dialog--edgematch .btn--primary', 'Uygula');
      await ui.sleep(300);
      const after = await ui.eval(`(() => {
        const k = window.kentos;
        const ends = (layer) => [...k.doc.all()].filter((e) => e.layerId === layer && (e.kind === 'line' || e.kind === 'polyline')).map((e) => (e.kind === 'line' ? [e.a, e.b] : [e.pts[0], e.pts[e.pts.length - 1]]));
        const onEdge = ends('pafta1').flat().filter((p) => Math.abs(p.x - ${EDGE_X}) < 1e-9).length;
        const step = (() => { const s = k.doc.undo(); k.doc.redo(); return s; })();
        const box = { minX: ${EDGE_X} - 6, minY: ${EDGE_Y} + 55, maxX: ${EDGE_X} + 6, maxY: ${EDGE_Y} + 67 };
        k.view.zoomToBox(box, 24);
        return { onEdge, step, selected: k.selection.size, open: !!document.querySelector('.dialog--edgematch') };
      })()`);
      if (!(after.onEdge === 4 && after.step === 'Kenar eşle' && after.selected === 8 && !after.open)) throw new Error(`Kenar eşleme, Uygula: ${JSON.stringify(after)}`);
      await ui.move(2, 2);
      await ui.sleep(400);
    },
  },
  // A search distance of nothing: said, Uygula off.
  {
    id: 'kenar-uyari',
    open: async (ui) => {
      await openEdgeScene(ui);
      await ui.eval(`(() => { const i = document.querySelector('.dialog--edgematch input[data-key="distance"]'); i.value = '0'; i.dispatchEvent(new Event('input')); })()`);
      await ui.sleep(200);
      const seen = await ui.eval(`({ said: document.querySelector('.dialog--edgematch .io-summary').textContent, apply: document.querySelector('.dialog--edgematch .btn--primary').disabled })`);
      if (!seen.said.startsWith('Arama uzaklığı sıfırdan büyük') || !seen.apply) throw new Error(`Kenar eşleme, uyarı: ${JSON.stringify(seen)}`);
      await ui.move(2, 2);
      await ui.sleep(300);
    },
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.selection.clear()`)), ...s }));

SCENES.leaders = [
  {
    id: 'leaders',
    open: async (ui) => {
      await ui.eval(`(async () => {
        const k = window.kentos;
        k.files.ask = async () => 'drop';
        if (!(await k.files.load(${JSON.stringify(LEADERS)}, null))) throw new Error('leaders.kcad did not load');
        k.view.zoomExtents();
        // A little room round the drawing, the block on the right still in view.
        const c = k.view.camera;
        c.scale = c.scale * 0.8;
        c.panBy(0, 0);
        k.selection.set([2]);
      })()`);
      await ui.move(2, 2);
      await ui.sleep(600);
    },
  },
  ...leaderToolScenes(),
  ...dxfLeaderScenes(),
];

// The new dimensions (docs/adr/0147) on fixtures/interaction/v1/dimensions.kcad, the scene the desktop's
// `labels::dimension_screens` draws: a corner's Y and X, a road edge's arc length and jogged radius, an edge's azimuth
// and slope, a value masked over a hatch and a slope in a block; the arc length selected so that Öznitelikler shows it.
const DIMENSIONS = readFileSync(new URL('../../../../fixtures/interaction/v1/dimensions.kcad', import.meta.url), 'utf8');
SCENES.dimensions = [
  {
    id: 'dimension-kinds',
    open: async (ui) => {
      await ui.eval(`(async () => {
        const k = window.kentos;
        k.files.ask = async () => 'drop';
        if (!(await k.files.load(${JSON.stringify(DIMENSIONS)}, null))) throw new Error('dimensions.kcad did not load');
        k.view.zoomExtents();
        // A little room round the drawing.
        const c = k.view.camera;
        c.scale = c.scale * 0.85;
        c.panBy(0, 0);
        k.selection.set([9]);
      })()`);
      await ui.move(2, 2);
      await ui.sleep(600);
    },
  },
  ...dimensionToolScenes(),
];

// The new dimensions in DXF (docs/adr/0147 §8), as the desktop's `exchange::dimension_tests::screens` (aktar-*-22…25).
SCENES.dimensionsdxf = dxfDimensionScenes();

// The display rule (docs/adr/0149), the desktop's `olcu-yarim-yaricap`: a circle typed with a radius of 50.0005 m (a
// half at the three decimals shown, 50.000499999… in binary), eight radius dimensions round it and a diameter, each
// placed as the tool places it (the centre plus the radius along a direction, at TM coordinates, so each carries its
// own 1e-10 m of noise), the circle selected: all read R 50.001 and Ø 100.001, as Öznitelikler's radius.
SCENES.rounding = [
  {
    id: 'half-radius',
    open: async (ui) => {
      const E = 487000;
      const N = 4420000;
      const R = 50.0005;
      const c = { x: E + 20, y: N + 10 };
      const on = (t) => ({ x: c.x + Math.cos(t) * R, y: c.y + Math.sin(t) * R });
      const layer = (id, name, color, lineWeight) => ({ id, name, type: 'layer', visible: true, locked: false, expanded: true, style: { color, lineType: 'continuous', lineWeight }, children: [] });
      const entities = [{ kind: 'circle', id: 1, layerId: 'parsel', attrs: {}, c, r: R }];
      for (let k = 0; k < 8; k++) entities.push({ kind: 'dimension', id: entities.length + 1, layerId: 'parsel', attrs: {}, a: c, b: on((k * Math.PI) / 4 + 0.3), offset: 12, height: 5, style: 'radius' });
      entities.push({ kind: 'dimension', id: entities.length + 1, layerId: 'parsel', attrs: {}, a: c, b: on(-0.6), offset: 0, height: 5, style: 'diameter' });
      const doc = JSON.stringify({
        format: 'kentos.document',
        version: 1,
        name: 'Yarım yarıçap',
        settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'cad', drawingFont: 'barlow' },
        origin: { x: E, y: N },
        layers: [layer('cizim', 'Çizim', 'fg', 0.25), layer('parsel', 'Parsel', '#3E63DD', 0.35)],
        activeLayer: 'cizim',
        entities,
        styles: { items: [], categories: [] },
      });
      await ui.eval(`(async () => {
        const k = window.kentos;
        k.files.ask = async () => 'drop';
        if (!(await k.files.load(${JSON.stringify(doc)}, null))) throw new Error('the drawing did not load');
        k.view.zoomExtents();
        // A little room round the drawing.
        const cam = k.view.camera;
        cam.scale = cam.scale * 0.85;
        cam.panBy(0, 0);
        k.selection.set([1]);
      })()`);
      await ui.move(2, 2);
      await ui.sleep(600);
    },
  },
];

// Topolojik temizlik (docs/adr/0148 §9) on fixtures/interaction/v1/topology.kcad, the desktop's `topology_scenes`:
// Değiştir › Nesne ▾ with the tool beside Çizimi temizle; the finding at 0.05 m with Köşeler on over the whole block;
// then close up, at 2 500 px a metre, the node three ends go to, the end cut where it runs past and the end extended to
// the locked road.
const TOPOLOGY = readFileSync(new URL('../../../../fixtures/interaction/v1/topology.kcad', import.meta.url), 'utf8');
SCENES.topology = topologyScenes();
function topologyScenes() {
  const E = 487000;
  const N = 4420000;
  /** The drawing open and the tool at 0.05 m with Köşeler on, as the trace leaves it before Enter; the view at `look` (east, north, px a metre) if given. */
  const found = async (ui, look) => {
    await ribbonOn(ui, { ribbonTab: 'edit', ...LOGGED });
    // Close up, the lines as hairlines: at 2 500 px a metre a 0.25 mm weight at 1:1000 would be 625 px wide.
    const view = look
      ? `c.center = { x: ${E + look[0]}, y: ${N + look[1]} }; c.scale = ${look[2]}; if (k.prefs.lineWeights.value) k.commands.execute('view.lineWeights');`
      : 'c.scale = c.scale * 0.85; c.center = { x: c.center.x - 110 / c.scale, y: c.center.y };';
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(TOPOLOGY)}, null))) throw new Error('topology.kcad did not load');
      k.view.zoomExtents();
      const c = k.view.camera;
      ${view}
      c.panBy(0, 0);
    })()`);
    await ui.sleep(400);
    await startTool(ui, 'topology');
    await typeValue(ui, '0.05');
    await ui.eval(`window.kentos.tools.active.input('K')`);
  };
  const at = async (ui, x, y) => (await ui.move(...(await ui.eval(PAGE_AT(E + x, N + y)))), await ui.sleep(350));
  /** The tool's memory as it was (0.01 m, Köşeler off), the drawing back. */
  const restore = async (ui) => {
    await ui.eval(`(() => { const k = window.kentos; if (!k.prefs.lineWeights.value) k.commands.execute('view.lineWeights'); const t = k.tools.active; if (t?.id !== 'topology') return; t.input('K'); t.input('0.01'); })()`);
    await ui.escapeAll(2);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  const CLOSE = 2500;
  return [
    { id: 'topology-list', open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'edit' }), await ui.clickSel('.rpanel__more[data-commands~="tool.topology"]'), await ui.sleep(400)), close: async (ui) => (await ui.escapeAll(2), await ribbonOff(ui)) },
    { id: 'topology-preview', open: async (ui) => (await found(ui), await at(ui, -4, -14)), close: restore },
    { id: 'topology-node', open: async (ui) => (await found(ui, [0.01, 0, CLOSE]), await at(ui, -0.11, -0.05)), close: restore },
    { id: 'topology-trim', open: async (ui) => (await found(ui, [-10, 15, CLOSE]), await at(ui, -10.12, 14.95)), close: restore },
    { id: 'topology-extend', open: async (ui) => (await found(ui, [10, -9.99, CLOSE]), await at(ui, 9.88, -10.04)), close: restore },
  ];
}

// Ölçülendirme's new methods at work (docs/adr/0147 §7), the desktop's `dimension_scenes`: Koordinat with the parcel's
// corner taken and the cursor off to the right; Yay uzunluğu with the road's edge taken and the dimension arc on the
// cursor; Kısmi with its first point on the arc and the part to the cursor lit; Kırıklı yarıçap with its jog on the
// cursor; Semt on the parcel's west edge; Eğim between two levelled points, and its question for a bare corner's
// elevation; Açı from the road's edge (Yaydan) and from a manhole (Daireden, Zemin on); Doğrusal along a typed
// direction; Öznitelikler's rows of a slope and of two ordinates; Hızlı ölçü over two parcels and a road edge, as the
// cursor places them and as written; Ölçülendirme ▾'s methods.
function dimensionToolScenes() {
  const layer = (id, name, color, lineWeight) => ({ id, name, type: 'layer', visible: true, locked: false, expanded: true, style: { color, lineType: 'continuous', lineWeight }, children: [] });
  const E = 487000;
  const N = 4420000;
  const P = (x, y) => ({ x: E + x, y: N + y });
  const C = [20, -60];
  const onEdge = (deg) => [C[0] + 50 * Math.cos((deg * Math.PI) / 180), C[1] + 50 * Math.sin((deg * Math.PI) / 180)];
  const road = (id, r) => ({ kind: 'arc', id, layerId: 'yol', attrs: {}, c: P(...C), r, a0: (50 * Math.PI) / 180, a1: (130 * Math.PI) / 180 });
  const ground = (extra = []) => JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Ölçülendirme',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'cad', drawingFont: 'barlow' },
    origin: { x: E, y: N },
    layers: [layer('cizim', 'Çizim', 'fg', 0.25), layer('yol', 'Yol', '#E5484D', 0.5), layer('parsel', 'Parsel', '#3E63DD', 0.35)],
    activeLayer: 'cizim',
    entities: [
      { kind: 'polygon', id: 1, layerId: 'parsel', attrs: {}, pts: [P(0, 0), P(40, 0), P(44, 32), P(2, 30)] },
      road(2, 50),
      road(3, 42),
      // Two levelled points east of the parcel.
      { kind: 'point', id: 4, layerId: 'parsel', attrs: {}, p: P(50, 0), z: 102.4 },
      { kind: 'point', id: 5, layerId: 'parsel', attrs: {}, p: P(54, 32), z: 101.15 },
      ...extra,
    ],
    styles: { items: [], categories: [] },
  });
  const GROUND = ground();
  // A manhole east of the road, 5 m across its centre (Açı's Daireden).
  const MANHOLE = [62, -22];
  const WITH_MANHOLE = ground([{ kind: 'circle', id: 6, layerId: 'yol', attrs: {}, c: P(...MANHOLE), r: 5 }]);
  // A slope between the levelled points (4 m east), the north-west corner's Y (up), the south-west corner's X (left, with Zemin).
  const dim = (id, style, a, b, more) => ({ kind: 'dimension', id, layerId: 'parsel', attrs: {}, a: P(...a), b: P(...b), offset: 0, height: 2.5, style, ...more });
  // Hızlı ölçü's ground (as fixtures/interaction/v1/quick-dimension.kcad, the road 6 m further south so that its
  // dimensions stand clear of the parcels'): two parcels sharing their 30 m edge, a road edge 10 m straight then a
  // clockwise quarter arc.
  const QUICK = JSON.parse(GROUND);
  QUICK.entities = [
    { kind: 'polygon', id: 1, layerId: 'parsel', attrs: {}, pts: [P(0, 0), P(20, 0), P(20, 30), P(0, 30)] },
    { kind: 'polygon', id: 2, layerId: 'parsel', attrs: {}, pts: [P(20, 0), P(40, 0), P(40, 30), P(20, 30)] },
    { kind: 'polyline', id: 3, layerId: 'yol', attrs: {}, pts: [P(0, -16), P(10, -16), P(20, -26)], bulges: [0, -Math.tan(Math.PI / 8)] },
  ];
  /** Hızlı ölçü over the three, all selected, the cursor 4 m over the parcels' north edge (the desktop's `quick`). */
  const quick = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', ...LOGGED });
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(JSON.stringify(QUICK))}, null))) throw new Error('the ground did not load');
      k.view.zoomExtents();
      const c = k.view.camera;
      c.scale = c.scale * 0.85;
      c.center = { x: c.center.x - 110 / c.scale, y: c.center.y };
      c.panBy(0, 0);
      k.selection.set([1, 2, 3]);
    })()`);
    await ui.sleep(300);
    await startTool(ui, 'quickDimension');
    await hoverAt(ui, 10, 34);
  };
  const MEASURED = ground([
    dim(6, 'slope', [50, 0], [54, 32], { offset: -4, za: 102.4, zb: 101.15 }),
    dim(7, 'ordinate', [2, 30], [2, 40], { angle: 0 }),
    dim(8, 'ordinate', [0, 0], [-12, 0], { angle: 90, mask: true }),
  ]);
  const pageAt = (ui, x, y) => ui.eval(PAGE_AT(E + x, N + y));
  const clickAt = async (ui, x, y) => (await ui.clickAt(...(await pageAt(ui, x, y))), await ui.sleep(200));
  /** The ground opened, Ölçülendirme started with a method's option, as Ölçülendirme ▾ starts it. */
  const start = async (ui, option, drawing = GROUND) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', ...LOGGED });
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(drawing)}, null))) throw new Error('the ground did not load');
      k.view.zoomExtents();
      // A little room round the drawing.
      const c = k.view.camera;
      c.scale = c.scale * 0.85;
      c.panBy(0, 0);
    })()`);
    await ui.sleep(400);
    await startTool(ui, 'dimension');
    await ui.eval(`window.kentos.tools.active.input(${JSON.stringify(option)})`);
  };
  const hoverAt = async (ui, x, y) => (await ui.eval(`window.kentos.log.clear()`), await ui.move(...(await pageAt(ui, x, y))), await ui.sleep(350));
  /**
   * The tool's memory as it was (Hizalı; Kısmi off when `partial`; `before` runs while the dimension is still being
   * placed, `after` once it starts over), then the drawing back.
   */
  const restore = (partial, before = '', after = '') => async (ui) => {
    await ui.eval(`(() => { const t = window.kentos.tools.active; if (t?.id !== 'dimension') return; ${before} t.reset(); ${after} ${partial ? "t.input('K');" : ''} t.input('H'); })()`);
    await ui.escapeAll(2);
    await ui.eval(UNDO_ALL);
  };
  /** Öznitelikler's Genel section folded (or opened again). */
  const general = async (ui, open) => {
    const head = `[...document.querySelectorAll('.panel--props .props__section')].find((b) => b.textContent.includes('Genel'))`;
    if (await ui.eval(`(() => { const b = ${head}; return !!b && (b.getAttribute('aria-expanded') === 'true') !== ${open}; })()`)) await ui.clickText('.panel--props .props__section', 'Genel');
  };
  /** Öznitelikler over `ids` of the measured ground, the layer tree at its least and Genel folded (the desktop's `props`). */
  const propsOf = async (ui, ids) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', ...LOGGED, layersFraction: 0.15 });
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(MEASURED)}, null))) throw new Error('the ground did not load');
      k.view.zoomExtents();
      const c = k.view.camera;
      c.scale = c.scale * 0.85;
      c.center = { x: c.center.x - 110 / c.scale, y: c.center.y };
      c.panBy(0, 0);
      k.selection.set(${JSON.stringify(ids)});
    })()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await general(ui, false);
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  const propsClose = async (ui) => (await general(ui, true), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  return [
    { id: 'dimension-ordinate', open: async (ui) => (await start(ui, 'O'), await clickAt(ui, 44, 32), await hoverAt(ui, 66, 35)), close: restore(false) },
    { id: 'dimension-arc-length', open: async (ui) => (await start(ui, 'U'), await clickAt(ui, ...onEdge(80)), await hoverAt(ui, 24, -4.8)), close: restore(false) },
    {
      id: 'dimension-partial',
      open: async (ui) => {
        await start(ui, 'U');
        await ui.eval(`window.kentos.tools.active.input('K')`);
        await clickAt(ui, ...onEdge(80));
        await clickAt(ui, ...onEdge(90));
        await hoverAt(ui, ...onEdge(115));
      },
      close: restore(true),
    },
    {
      id: 'dimension-jogged',
      open: async (ui) => {
        await start(ui, 'I');
        await clickAt(ui, ...onEdge(60));
        await clickAt(ui, 32.402, -32.519);
        await clickAt(ui, ...onEdge(60));
        await hoverAt(ui, 38, -24);
      },
      close: restore(false),
    },
    { id: 'dimension-azimuth', open: async (ui) => (await start(ui, 'T'), await clickAt(ui, 0, 0), await clickAt(ui, 2, 30), await hoverAt(ui, -3, 15)), close: restore(false) },
    { id: 'dimension-slope', open: async (ui) => (await start(ui, 'E'), await clickAt(ui, 50, 0), await clickAt(ui, 54, 32), await hoverAt(ui, 57, 16)), close: restore(false) },
    { id: 'dimension-slope-ask', open: async (ui) => (await start(ui, 'E'), await clickAt(ui, 0, 0), await hoverAt(ui, 20, 4)), close: restore(false) },
    { id: 'dimension-angle-arc', open: async (ui) => (await start(ui, 'A'), await clickAt(ui, ...onEdge(90)), await hoverAt(ui, 20, -6)), close: restore(false) },
    {
      id: 'dimension-angle-circle',
      open: async (ui) => {
        await start(ui, 'A', WITH_MANHOLE);
        // Snapping off, as the desktop's scene has nothing to snap to here: the second point and the cursor exactly where given.
        await ui.eval(`(() => { const k = window.kentos; window.__snapWas = k.settings.snap.value; k.settings.snap.set(false); k.tools.active.input('Z'); })()`);
        await clickAt(ui, MANHOLE[0] + 5, MANHOLE[1]);
        // A click lands on a whole pixel, so the first point sits a little off east on a 5 m circle: the second point
        // is typed a right angle round from where it landed, 12 m out, so the angle reads 100 g as on the desktop.
        await ui.eval(`(() => {
          const t = window.kentos.tools.active;
          const { c, p1 } = t.angleCircle;
          const a = Math.atan2(p1.y - c.y, p1.x - c.x) + Math.PI / 2;
          t.input((c.x + 12 * Math.cos(a)).toFixed(6) + ',' + (c.y + 12 * Math.sin(a)).toFixed(6));
        })()`);
        await hoverAt(ui, MANHOLE[0] + 6, MANHOLE[1] + 6);
      },
      close: async (ui) => (await restore(false, '', "t.input('Z');")(ui), await ui.eval(`window.kentos.settings.snap.set(window.__snapWas ?? true)`)),
    },
    {
      id: 'dimension-linear-angle',
      open: async (ui) => {
        await start(ui, 'D');
        await clickAt(ui, 0, 0);
        await clickAt(ui, 44, 32);
        await ui.eval(`(() => { const t = window.kentos.tools.active; t.input('A'); t.input('60'); })()`);
        await hoverAt(ui, 40, -8);
      },
      close: restore(false, "t.input('O');"),
    },
    { id: 'dimension-quick', open: quick, close: restore(false) },
    {
      id: 'dimension-quick-result',
      open: async (ui) => {
        await quick(ui);
        await clickAt(ui, 10, 34);
        // Drawn back so that the dimensions over the parcels show whole (the desktop's zoom of 0.85 about (20, 4)).
        await ui.eval(`(() => {
          const k = window.kentos;
          k.selection.set([]);
          const c = k.view.camera;
          const at = { x: ${E} + 20, y: ${N} + 4 };
          c.center = { x: at.x + (c.center.x - at.x) / 0.85, y: at.y + (c.center.y - at.y) / 0.85 };
          c.scale = c.scale * 0.85;
          c.panBy(0, 0);
        })()`);
        await hoverAt(ui, 60, -30);
      },
      close: restore(false),
    },
    { id: 'dimension-properties', open: async (ui) => propsOf(ui, [6]), close: propsClose },
    { id: 'dimension-properties-many', open: async (ui) => propsOf(ui, [7, 8]), close: propsClose },
    {
      id: 'dimension-methods',
      open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad' }), await ui.clickSel('.ribbon__strip [data-split="dimension"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
    },
    // Açı chosen from Ölçülendirme ▾: the large button's face shows its icon (the desktop's `olcu-dugme-aci`).
    {
      id: 'dimension-chosen',
      open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad', ribbonSplits: { dimension: 'tool.dimension|A' } }), await ui.move(2, 2), await ui.sleep(300)),
      close: async (ui) => ribbonOff(ui),
    },
  ];
}

// DXF içe aktar over the leaders' fixture (fixtures/formats/v1/leaders.dxf, docs/adr/0146 §8): the window says what
// became of the LEADERs' MTEXTs, the hookline, the spline path, the MULTILEADERs' other lines and block content; in,
// every leader with its note (a note's other lines under it); out again, the window says how leaders are written.
// The desktop's are `exchange::tests::leader_screens` (aktar-*-19…21).
function dxfLeaderScenes() {
  const bytes = readFileSync(new URL('../../../../fixtures/formats/v1/leaders.dxf', import.meta.url)).toString('base64');
  const open = `import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'leaders.dxf', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`;
  const read = async (ui) => (await ui.eval(open), await ui.waitFor(DXF_READ, 15000));
  const into = async (ui) => (await read(ui), await ui.clickText('.dialog--io .btn--primary', 'İçe aktar'), await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000));
  const close = async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL));
  return [
    { id: 'import-dxf-leaders', open: async (ui) => (await read(ui), await ui.sleep(400)), close },
    {
      id: 'import-dxf-leaders-in',
      open: async (ui) => {
        await into(ui);
        // A little room round the drawing.
        await ui.eval(`(() => {
          const k = window.kentos;
          k.selection.clear();
          const c = k.view.camera;
          c.fit({ minX: -6, minY: -24, maxX: 170, maxY: 14 }, 24);
          c.scale = c.scale * 0.72;
          c.panBy(0, 0);
          k.view.requestRender();
        })()`);
        await ui.move(2, 2);
        await ui.sleep(600);
      },
      close,
    },
    {
      id: 'export-dxf-leaders',
      open: async (ui) => {
        await into(ui);
        await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
        await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
        await ui.clickText('.dialog--io .seg__opt', 'Tümü');
        await ui.sleep(400);
      },
      close,
    },
  ];
}

// DXF içe aktar over another program's new dimensions (fixtures/formats/v1/dimension-kinds.dxf, docs/adr/0147 §8):
// the window says what came in by its block and why; in, the ordinates, the arc length (masked) and the jogged radius
// as KentOS's own. Then KentOS's own file (fixtures/formats/v1/dxf-write/dimensions.dxf): every kind back; out again,
// the window says how Semt and Eğim are written.
function dxfDimensionScenes() {
  const file = (name) => readFileSync(new URL(`../../../../fixtures/formats/v1/${name}`, import.meta.url)).toString('base64');
  const open = (name, bytes) =>
    `import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: '${name}', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`;
  const foreign = open('dimension-kinds.dxf', file('dimension-kinds.dxf'));
  const own = open('dimensions.dxf', file('dxf-write/dimensions.dxf'));
  const read = async (ui, what) => (await ui.eval(what), await ui.waitFor(DXF_READ, 15000));
  const into = async (ui, what) => (await read(ui, what), await ui.clickText('.dialog--io .btn--primary', 'İçe aktar'), await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000));
  // The desktop's frame.
  const frame = async (ui) => {
    await ui.eval(`(() => {
      const k = window.kentos;
      k.selection.clear();
      const c = k.view.camera;
      c.fit({ minX: 452262, minY: 4412278, maxX: 452482, maxY: 4412350 }, 24);
      c.scale = c.scale * 0.8;
      c.panBy(0, 0);
      k.view.requestRender();
    })()`);
    await ui.move(2, 2);
    await ui.sleep(600);
  };
  const close = async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL));
  return [
    { id: 'import-dxf-dimensions', open: async (ui) => (await read(ui, foreign), await ui.sleep(400)), close },
    { id: 'import-dxf-dimensions-in', open: async (ui) => (await into(ui, foreign), await frame(ui)), close },
    { id: 'import-dxf-dimensions-own', open: async (ui) => (await into(ui, own), await frame(ui)), close },
    {
      id: 'export-dxf-dimensions',
      open: async (ui) => {
        await into(ui, own);
        await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
        await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
        await ui.clickText('.dialog--io .seg__opt', 'Tümü');
        // The summary in view in a short window too (the window's body scrolls).
        await ui.eval(`document.querySelector('.dialog--io .io-summary').scrollIntoView({ block: 'end' })`);
        await ui.sleep(400);
      },
      close,
    },
  ];
}

/**
 * Kılavuz at work (docs/adr/0146 §7) over the desktop's ground (`apps/desktop/src/leader_scenes.rs`): a parcel, a
 * building in it and a road below, in metres from the drawing's origin. The tip on the building's corner, a vertex,
 * the pointer where the landing starts; the note's field with the note typed; Ok's menu from the command line's chip;
 * the leaders written (a filled arrow with its note, an open one from the road going left with a masked note, a dot);
 * the first's note edited in place after a double click.
 */
function leaderToolScenes() {
  const layer = (id, name, color, lineWeight) => ({ id, name, type: 'layer', visible: true, locked: false, expanded: true, style: { color, lineType: 'continuous', lineWeight }, children: [] });
  const E = 487000;
  const N = 4420000;
  const P = (x, y) => ({ x: E + x, y: N + y });
  const box = (id, layerId, x0, y0, x1, y1) => ({ kind: 'polygon', id, layerId, attrs: {}, pts: [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)] });
  const GROUND = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Kılavuz',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'cad', drawingFont: 'barlow' },
    origin: { x: E, y: N },
    layers: [layer('cizim', 'Çizim', 'fg', 0.25), layer('yol', 'Yol', '#E5484D', 0.5), layer('parsel', 'Parsel', '#3E63DD', 0.35)],
    activeLayer: 'cizim',
    entities: [box(1, 'parsel', 0, 0, 40, 30), box(2, 'cizim', 8, 6, 22, 18), { kind: 'line', id: 3, layerId: 'yol', attrs: {}, a: P(-6, -4), b: P(60, -4) }],
    styles: { items: [], categories: [] },
  });
  const pageAt = (ui, x, y) => ui.eval(PAGE_AT(E + x, N + y));
  const clickAt = async (ui, x, y) => (await ui.clickAt(...(await pageAt(ui, x, y))), await ui.sleep(200));
  const enter = (ui) => ui.eval(`window.kentos.tools.active.confirm()`);
  const field = '.inline-text:not([hidden]) .inline-text__input';
  /** The note typed into its field, as a user types it. */
  const typeNote = async (ui, text) => (await ui.waitFor(`!!document.querySelector('${field}')`), await ui.clickSel(field), await ui.type(text), await ui.sleep(200));
  const keep = async (ui) => (await ui.key('Enter'), await ui.sleep(250));
  const tool = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', ...LOGGED });
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(GROUND)}, null))) throw new Error('the ground did not load');
      k.view.zoomExtents();
      // A little room round the drawing.
      const c = k.view.camera;
      c.scale = c.scale * 0.85;
      c.panBy(0, 0);
      k.log.clear();
    })()`);
    await ui.sleep(400);
    await startTool(ui, 'leader');
    await clickAt(ui, 22, 18);
    await clickAt(ui, 28, 24);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(...(await pageAt(ui, 36, 25)));
    await ui.sleep(350);
  };
  const note = async (ui) => {
    await tool(ui);
    await clickAt(ui, 36, 25);
    await enter(ui);
    await typeNote(ui, 'Mevcut bina');
  };
  const written = async (ui) => {
    await note(ui);
    await keep(ui);
    await ui.eval(`(() => { const t = window.kentos.tools.active; t.chooseOption('O', 'açık'); t.input('Z'); })()`);
    for (const [x, y] of [[50, -4], [46, 4], [38, 6]]) await clickAt(ui, x, y);
    await enter(ui);
    await typeNote(ui, 'Ø150 PVC');
    await keep(ui);
    await ui.eval(`(() => { const t = window.kentos.tools.active; t.chooseOption('O', 'nokta'); t.input('Z'); })()`);
    for (const [x, y] of [[8, 18], [3, 23]]) await clickAt(ui, x, y);
    await enter(ui);
    await ui.waitFor(`!!document.querySelector('${field}')`);
    await keep(ui);
    await ui.move(...(await pageAt(ui, 52, 12)));
    await ui.sleep(350);
  };
  /** Kılavuz left, a double click on the first leader's note: its field over it, the note's words changed (the building is to be pulled down). */
  const edit = async (ui) => {
    await written(ui);
    await ui.escapeAll(2);
    const [x, y] = await pageAt(ui, 46, 25);
    await ui.clickAt(x, y);
    await ui.clickAt(x, y);
    await ui.waitFor(`!!document.querySelector('${field}')`);
    // The field chooses its text once it is shown; typing then replaces it.
    await ui.sleep(300);
    await ui.eval(`document.querySelector('${field}').select()`);
    await ui.type('Yıkılacak bina');
    await ui.sleep(300);
  };
  const close = async (ui) => (await ui.escapeAll(3), await ribbonOff(ui));
  return [
    { id: 'leader-tool', open: tool, close },
    { id: 'leader-note', open: note, close },
    {
      id: 'leader-arrows',
      // Ok's chip, or in a narrow window Diğer's menu with Ok's open over it.
      open: async (ui) => {
        await tool(ui);
        const chip = '.cmdline__chip[aria-haspopup="menu"]:not(.cmdline__more):not([hidden])';
        if (await ui.eval(`!!document.querySelector('${chip}')`)) return (await ui.clickSel(chip), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300));
        await ui.clickSel('.cmdline__more');
        await ui.waitFor(`!!document.querySelector('.menu')`);
        await ui.hoverText('.menu .menu__item', 'Ok');
        await ui.sleep(300);
      },
      close,
    },
    { id: 'leader-written', open: written, close },
    { id: 'leader-edit', open: edit, close },
  ];
}

/** The project's type (docs/adr/0165): the question of a project opened without one, the status bar's menu. */
SCENES.projecttype = [
  {
    id: 'type-question',
    open: async (ui) => {
      await ui.eval(`(() => { const k = window.kentos; k.doc.settings.workspace.set(null); k.files.askTypeIfNeeded(); })()`);
      await ui.waitFor(`!!document.querySelector('.dialog--projtype')`);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(1), await ui.eval(`window.kentos.doc.settings.workspace.set('gis')`)),
  },
  { id: 'type-menu', open: async (ui) => (await ui.clickSel('.status__mode'), await ui.sleep(250)) },
];

/**
 * Dosya → Yeni proje as a wizard (docs/adr/0165 §3): the type's cards with their pictures; a CBS project's province,
 * its suggested system and the zone strip; a CAD project local in millimetres, and with real coordinates; the last
 * step's scale and summary. The desktop's are `project::wizard::tests::screens`.
 */
const WIZARD = `!!document.querySelector('.dialog--wizard .wiz__page')`;
const openWizard = async (ui) => (await ui.eval(`window.kentos.commands.execute('file.new')`), await ui.waitFor(WIZARD), await ui.sleep(300));
const wizardNext = async (ui) => (await ui.clickText('.dialog--wizard .dialog__foot .btn', 'İleri'), await ui.sleep(250));
const wizardType = async (ui, mode) => (await ui.clickSel(`.dialog--wizard .wspick__card[data-mode="${mode}"]`), await ui.sleep(150));
SCENES.newproject = [
  { id: 'wizard-type', open: async (ui) => (await openWizard(ui), await wizardType(ui, 'gis')), close: async (ui) => await ui.escapeAll(1) },
  {
    id: 'wizard-gis-izmir',
    open: async (ui) => {
      await openWizard(ui);
      await wizardType(ui, 'gis');
      await wizardNext(ui);
      await ui.clickSel('.dialog--wizard .wiz__search');
      await ui.type('izm');
      await ui.key('Enter');
      await ui.sleep(300);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
  {
    id: 'wizard-cad-local',
    open: async (ui) => {
      await openWizard(ui);
      await wizardType(ui, 'cad');
      await wizardNext(ui);
      await ui.clickSel('.dialog--wizard .wiz__choice[data-id="mm"]');
      await ui.sleep(250);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
  {
    id: 'wizard-cad-real',
    open: async (ui) => {
      await openWizard(ui);
      await wizardType(ui, 'cad');
      await wizardNext(ui);
      await ui.clickSel('.dialog--wizard .wiz__choice[data-id="real"]');
      await ui.sleep(200);
      await ui.clickSel('.dialog--wizard .wiz__search');
      await ui.type('61');
      await ui.key('Enter');
      await ui.sleep(300);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
  {
    id: 'wizard-details',
    open: async (ui) => {
      await openWizard(ui);
      await wizardType(ui, 'cad');
      await wizardNext(ui);
      await ui.clickSel('.dialog--wizard .wiz__choice[data-id="mm"]');
      await wizardNext(ui);
      await ui.sleep(250);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
];

/**
 * A local project in millimetres (docs/adr/0165 §2), new and without a coordinate system: Proje ayarları' Çizim
 * birimi, then a plate 120 × 80 mm with a hole at the origin, selected, read in millimetres. The desktop's are
 * `project::tests::screens` (proje-*-6, -7).
 */
const LOCAL_MM = `Promise.all([import('/src/model/newProject.ts'), import('/src/app/fileIO.ts')]).then(([np, io]) => {
  const c = np.newProjectContent({ name: 'Mil plakası', srid: 0, plotScale: 1, workspace: 'cad' });
  io.replaceDrawing(window.kentos, { ...c, settings: { ...c.settings, drawingUnit: 'mm' } });
})`;
SCENES.drawingunit = [
  {
    id: 'unit-settings',
    open: async (ui) => {
      await ui.eval(`${LOCAL_MM}.then(() => import('/src/ui/settings/ProjectSettingsDialog.ts')).then((m) => m.openProjectSettings(window.kentos, 'units'))`);
      await ui.waitFor(`!!document.querySelector('.settings__content')`);
      await ui.sleep(300);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
  {
    id: 'unit-drawing',
    open: async (ui) => {
      await ui.eval(`${LOCAL_MM}.then(() => {
        const k = window.kentos;
        const layerId = k.doc.layers.active.value;
        const plate = k.doc.add({ kind: 'polygon', layerId, attrs: {}, pts: [{ x: 0, y: 0 }, { x: 0.12, y: 0 }, { x: 0.12, y: 0.08 }, { x: 0, y: 0.08 }] });
        k.doc.add({ kind: 'circle', layerId, attrs: {}, c: { x: 0.06, y: 0.04 }, r: 0.025 });
        k.selection.set([plate.id]);
        k.view.camera.fit({ minX: -0.02, minY: -0.02, maxX: 0.14, maxY: 0.1 }, 24);
        k.view.requestRender();
      })`);
      await ui.waitFor(`window.kentos.doc.name.value === 'Mil plakası' && window.kentos.doc.size === 2`);
      await ui.sleep(600);
    },
  },
  // DXF içe aktar of a file in inches (fixtures/formats/v1/units.dxf): the window says its unit and that its values
  // come in the project's. The desktop's are `exchange::tests::unit_screens` (aktar-*-22).
  {
    id: 'unit-dxf-import',
    open: async (ui) => {
      const bytes = readFileSync(new URL('../../../../fixtures/formats/v1/units.dxf', import.meta.url)).toString('base64');
      const file = `{ name: 'units.dxf', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }`;
      await ui.eval(`${LOCAL_MM}.then(() => import('/src/ui/io/DrawingImportDialog.ts')).then((m) => m.openDxfImport(window.kentos, ${file}, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`);
      await ui.waitFor(DXF_READ, 15000);
      await ui.sleep(400);
    },
    close: async (ui) => await ui.escapeAll(1),
  },
];

/**
 * The project types' scenes (docs/adr/0165 §4–§5): a local CAD project's plate on slate with its coordinate axes, the
 * status bar's scale selector open over it, and its typed scale field. The desktop's are
 * `project::wizard::tests::axes_screens` and `screen_scale::tests::screens`.
 */
const CAD_PLATE = `${LOCAL_MM}.then(() => {
  const k = window.kentos;
  const layerId = k.doc.layers.active.value;
  const plate = k.doc.add({ kind: 'polygon', layerId, attrs: {}, pts: [{ x: 0.02, y: 0.02 }, { x: 0.14, y: 0.02 }, { x: 0.14, y: 0.1 }, { x: 0.05, y: 0.1 }] });
  k.selection.set([plate.id]);
  k.view.camera.fit({ minX: -0.02, minY: -0.02, maxX: 0.2, maxY: 0.14 }, 24);
  k.view.requestRender();
})`;
SCENES.scene = [
  { id: 'scene-cad', open: async (ui) => (await ui.eval(CAD_PLATE), await ui.sleep(600)) },
  {
    id: 'scale-menu',
    open: async (ui) => (await ui.eval(CAD_PLATE), await ui.sleep(400), await ui.clickSel('.status__zoom'), await ui.sleep(300)),
    close: async (ui) => await ui.escapeAll(1),
  },
  {
    id: 'scale-field',
    open: async (ui) => (await ui.eval(CAD_PLATE), await ui.sleep(400), await ui.clickSel('.status__zoom'), await ui.sleep(200), await ui.clickText('.menu__item', 'Ölçek yaz…'), await ui.sleep(300)),
    close: async (ui) => await ui.escapeAll(1),
  },
];

/**
 * The second coordinate system (docs/adr/0167 §1–§2): its values beside the cursor's in the status bar (ED50 TM36; WGS 84
 * in degrees, minutes and seconds), the coordinate system's right-click menu with İkinci sistem open, Koordinat oku's
 * reading and Proje ayarları' field. The desktop's are `second_crs::tests::screens` (ikinci-sistem-*).
 */
// The second system set, and the status bar's message put out: the values take their room back (StatusBar.ts).
const SECOND = (srid) =>
  `(() => { window.kentos.doc.settings.assign({ secondSrid: ${srid} }); document.querySelector('.status__flash')?.removeAttribute('data-show'); })()`;
const NO_SECOND = `(() => { const k = window.kentos; k.doc.settings.assign({ secondSrid: null }); k.prefs.geographic.set('dms'); })()`;
const overDrawing = async (ui) => {
  const [w, h] = await ui.eval(`[innerWidth, innerHeight]`);
  await ui.move(Math.round(w * 0.42), Math.round(h * 0.48));
  await ui.sleep(300);
};
const secondClose = async (ui) => (await ui.escapeAll(2), await ui.eval(NO_SECOND));
SCENES.secondcrs = [
  { id: 'second-status', open: async (ui) => (await ui.eval(SECOND(2322)), await overDrawing(ui)), close: secondClose },
  { id: 'second-wgs84', open: async (ui) => (await ui.eval(SECOND(4326)), await overDrawing(ui)), close: secondClose },
  {
    id: 'second-menu',
    open: async (ui) => {
      await ui.eval(SECOND(2322));
      const at = await ui.eval(`(() => { const r = document.querySelector('.ribbon__crs').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      await ui.contextClick(...at);
      await ui.hoverText('.menu__item', 'İkinci sistem');
    },
    close: secondClose,
  },
  {
    id: 'second-read',
    open: async (ui) => {
      await ui.eval(SECOND(2322));
      await ui.eval(`window.kentos.commands.execute('crs.query')`);
      const [w, h] = await ui.eval(`[innerWidth, innerHeight]`);
      await ui.clickAt(Math.round(w * 0.42), Math.round(h * 0.48));
      await ui.sleep(300);
      await ui.eval(`document.querySelector('.status__flash')?.removeAttribute('data-show')`);
      await ui.sleep(200);
    },
    close: secondClose,
  },
  {
    id: 'second-settings',
    open: async (ui) => {
      await ui.eval(`${SECOND(2322)}; import('/src/ui/settings/ProjectSettingsDialog.ts').then((m) => m.openProjectSettings(window.kentos, 'crs'))`);
      await ui.waitFor(`!!document.querySelector('.settings__content')`);
      await ui.eval(`[...document.querySelectorAll('.sgroup__title')].find((e) => e.textContent === 'İkinci koordinat sistemi')?.scrollIntoView({ block: 'center' })`);
      await ui.sleep(300);
    },
    close: secondClose,
  },
];

/**
 * The project's own definitions (docs/adr/0168) over the demo drawing (TUREF TM36): a second system the project defines
 * (a municipality's local system) in the status bar and in İkinci sistem; a project whose system is its own definition,
 * its second system ED50 TM36 by the project's datum choice, and Koordinat oku. The desktop's are
 * `second_crs::tests::custom_screens` (ozel-sistem-*), with the same definitions.
 */
const CUSTOM_SECOND = { name: 'Belediye sistemi', system: { kind: 'local', base: { srid: 5256 }, plane: { kind: 'similarity', east: 486000, north: 4419800, rotation: -15, scale: 1 } } };
const CUSTOM_OWN = { name: 'Şantiye', system: { kind: 'local', base: { srid: 5256 }, plane: { kind: 'similarity', east: 120, north: -80, rotation: 0, scale: 1 } } };
// Seven parameters for ED50–TUREF, made up for the pictures.
const CUSTOM_CHOICE = {
  from: 'ED50',
  to: 'TUREF',
  name: 'ED50 → TUREF: örnek parametreler',
  helmert: { translation: [-84.1, -101.8, -129.7], rotation: [0, 0, 0.468], scale: 1.05, convention: 'positionVector', accuracy: 0.3 },
};
const settle = `document.querySelector('.status__flash')?.removeAttribute('data-show')`;
const customSecond = `(() => { window.kentos.doc.settings.assign({ secondCustomCrs: ${JSON.stringify(CUSTOM_SECOND)} }); ${settle}; })()`;
const customOwn = `(() => { window.kentos.doc.settings.assign({ srid: 0, customCrs: ${JSON.stringify(CUSTOM_OWN)}, secondSrid: 2322, datumTransforms: [${JSON.stringify(CUSTOM_CHOICE)}] }); ${settle}; })()`;
const customClose = async (ui) => (
  await ui.escapeAll(2), await ui.eval(`window.kentos.doc.settings.assign({ srid: 5256, customCrs: null, secondSrid: null, secondCustomCrs: null, datumTransforms: [] })`)
);
/**
 * Proje ayarları' Izgaralar (docs/adr/0168 §4, §6): the test grid of fixtures/geodesy/v1/ntv2 added to the device's
 * library, the project's second system WGS 84 by it, and a choice naming a grid the device does not have; Kaldır's
 * question. The desktop's are `grids::tests::screens` (izgara-*).
 */
const TR_GSB = readFileSync(new URL('../../../../fixtures/geodesy/v1/ntv2/tr.gsb', import.meta.url));
const TR_ID = createHash('sha256').update(TR_GSB).digest('hex');
const gridChoice = (from, to, name, id, file) => ({ from, to, name, grid: { id, file, size: 1024 } });
const gridsOpen = async (ui) => {
  const choices = [gridChoice('TUREF', 'WGS84', 'TUREF → WGS 84: ızgara', TR_ID, 'tr.gsb'), gridChoice('ED50', 'TUREF', 'ED50 → TUREF: bölge ızgarası', 'ab'.repeat(32), 'ed50-turef-bolge.gsb')];
  await ui.eval(`(async () => {
    const k = window.kentos;
    const bytes = Uint8Array.from(atob('${TR_GSB.toString('base64')}'), (c) => c.charCodeAt(0));
    await k.grids.add('tr.gsb', bytes);
    k.doc.settings.assign({ secondSrid: 4326, datumTransforms: ${JSON.stringify(choices)} });
    ${settle};
  })()`);
  await ui.sleep(300);
  await ui.eval(`import('/src/ui/settings/ProjectSettingsDialog.ts').then((m) => m.openProjectSettings(window.kentos, 'crs'))`);
  await ui.waitFor(`!!document.querySelector('.settings__content')`);
  await ui.sleep(300);
  await ui.eval(`[...document.querySelectorAll('.sgroup__title')].find((e) => e.textContent === 'Izgaralar')?.scrollIntoView({ block: 'center' })`);
  await ui.sleep(300);
};
const gridsClose = async (ui) => (
  await ui.escapeAll(3), await ui.eval(`(async () => { const k = window.kentos; await k.grids.remove('${TR_ID}'); k.doc.settings.assign({ secondSrid: null, datumTransforms: [] }); })()`)
);
SCENES.grids = [
  { id: 'grids-list', open: gridsOpen, close: gridsClose },
  {
    id: 'grids-remove',
    open: async (ui) => (await gridsOpen(ui), await ui.clickText('.grid-library .btn', 'Kaldır'), await ui.sleep(300)),
    close: gridsClose,
  },
];

/**
 * Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6): ED50–TUREF by seven parameters with ΔZ still to type
 * (Kaydet waits), ED50–WGS 84 by the test grid of the device's library, TUREF–WGS 84 EPSG's way. The desktop's are
 * `project::choices::tests::screens` (datum-donusumleri-*).
 */
const byLabel = (label, text) =>
  `(() => { const el = document.querySelector(${JSON.stringify(`[aria-label="${label}"]`)}); el.value = ${JSON.stringify(text)}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`;
const clickIn = (group, option) =>
  `[...document.querySelectorAll(${JSON.stringify(`[aria-label="${group}"] .seg__opt`)})].find((b) => b.textContent === ${JSON.stringify(option)})?.click()`;
SCENES.datums = [
  {
    id: 'datum-choices',
    open: async (ui) => {
      await ui.eval(`(async () => { await window.kentos.grids.add('tr.gsb', Uint8Array.from(atob('${TR_GSB.toString('base64')}'), (c) => c.charCodeAt(0))); })()`);
      await ui.eval(`import('/src/ui/settings/ProjectSettingsDialog.ts').then((m) => m.openProjectSettings(window.kentos, 'crs'))`);
      await ui.waitFor(`!!document.querySelector('.settings__content')`);
      await ui.eval(clickIn('ED50 ↔ TUREF yöntemi', '7 parametre'));
      await ui.sleep(200);
      await ui.eval(byLabel('ED50 ↔ TUREF Ad', 'ED50 → TUREF: Bölge 7'));
      for (const [caption, v] of [['ΔX (m)', '-158.785'], ['ΔY (m)', '-109.965'], ['rX (″)', '1.4275'], ['rY (″)', '-3.0873'], ['rZ (″)', '0.5505'], ['Ölçek farkı (ppm)', '-5.1814'], ['Doğruluk (m)', '0.3']])
        await ui.eval(byLabel(`ED50 ↔ TUREF ${caption}`, v));
      await ui.eval(clickIn('ED50 ↔ TUREF dönüklüklerin kuralı', 'Koordinat çerçevesi (9607)'));
      await ui.sleep(200);
      await ui.eval(clickIn('ED50 ↔ WGS 84 yöntemi', 'NTv2 ızgarası'));
      await ui.sleep(200);
      await ui.clickSel('[aria-label="ED50 ↔ WGS 84 ızgarası"]');
      await ui.clickText('.menu__item', 'tr.gsb');
      await ui.sleep(200);
      await ui.eval(byLabel('ED50 ↔ WGS 84 Doğruluk (m)', '0.5'));
      await ui.eval(`[...document.querySelectorAll('.sgroup__title')].find((e) => e.textContent === 'Datum dönüşümleri')?.scrollIntoView({ block: 'start' })`);
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.escapeAll(3), await ui.eval(`window.kentos.grids.remove('${TR_ID}')`)),
  },
];

/**
 * Proje ayarları' Ölçme (docs/adr/0169 §3): empty, k, the ground height (docs/adr/0171 §2) and the tolerances typed, and
 * what does not hold said under its field (Kaydet waits). The desktop's are `project::survey::tests::screens` (olcme-ayar-*).
 */
const surveyOpen = async (ui, fields) => {
  await ui.eval(`import('/src/ui/settings/ProjectSettingsDialog.ts').then((m) => m.openProjectSettings(window.kentos, 'survey'))`);
  await ui.waitFor(`!!document.querySelector('.survey-field')`);
  for (const [label, v] of fields) await ui.clickSel(`[aria-label="${label}"]`), await ui.type(v);
  await ui.sleep(250);
};
const SURVEY_FILLED = [
  ['Refraksiyon katsayısı (k)', '0.14'],
  ['Ortalama elipsoit yüksekliği', '850'],
  ['İki durum yatay açı farkı', '20'],
  ['İndeks hatası', '10'],
  ['İki durum uzunluk farkı', '5'],
  ['Kenarın iki yönden farkı', '10'],
  ['Açı kapanması', '60'],
  ['Koordinat kapanması', '30'],
];
const SURVEY_WRONG = [
  ['Refraksiyon katsayısı (k)', '1.5'],
  ['Ortalama elipsoit yüksekliği', '9500'],
  ['İki durum yatay açı farkı', '0'],
  ['İndeks hatası', 'on'],
  ['İki durum uzunluk farkı', '5'],
  ['Koordinat kapanması', '-2'],
];
SCENES.survey = [
  { id: 'survey-empty', open: (ui) => surveyOpen(ui, []), close: (ui) => ui.escapeAll(3) },
  { id: 'survey-filled', open: (ui) => surveyOpen(ui, SURVEY_FILLED), close: (ui) => ui.escapeAll(3) },
  { id: 'survey-problems', open: (ui) => surveyOpen(ui, SURVEY_WRONG), close: (ui) => ui.escapeAll(3) },
  // The ground height beyond its bounds, the Zemin group in view (docs/adr/0171 §2).
  { id: 'survey-ground', open: (ui) => surveyOpen(ui, [['Ortalama elipsoit yüksekliği', '9500']]), close: (ui) => ui.escapeAll(3) },
];

/**
 * Karne editörü (docs/adr/0169 §6): fixtures/field/v1/sample.gsi with the project checking the faces' slope distances
 * against 5 mm (one target above it), and a text book waiting for its columns; the traverse's leg against a two-way
 * tolerance of 3 mm (above it), and Poligon hesabı's misclosures against 60 cc and 30 mm. The desktop's are
 * `calc::fieldbook::tests::screens` (karne-*).
 */
const SAMPLE_GSI = readFileSync(new URL('../../../../fixtures/field/v1/sample.gsi', import.meta.url));
const SAMPLE_SDR = readFileSync(new URL('../../../../fixtures/field/v1/sample.sdr', import.meta.url));
const SAMPLE_GT7 = readFileSync(new URL('../../../../fixtures/field/v1/sample.gt7', import.meta.url));
const SAMPLE_NIKON = readFileSync(new URL('../../../../fixtures/field/v1/sample-nikon.raw', import.meta.url));
const SAMPLE_JXL = readFileSync(new URL('../../../../fixtures/field/v1/sample.jxl', import.meta.url));
const TRAVERSE_SURVEY = `{ faceSlope: 0.005, twoWay: 0.003, traverseAngle: ${(60 * Math.PI) / 2_000_000}, traverseCoord: 0.03 }`;
const bodyToEnd = (ui, dialog) => ui.eval(`(() => { const b = document.querySelector('${dialog} .dialog__body'); b.scrollTop = b.scrollHeight; })()`);
const chooseFore = async (ui) => {
  await ui.eval(`(() => { const s = document.querySelector('.dialog--fieldbook select[aria-label="Bitişte bakılan"]'); s.value = '4'; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
  await ui.sleep(200);
};
const fieldBookOpen = async (ui, name, bytes, survey = '{ faceSlope: 0.005 }') => {
  await ui.eval(`window.kentos.doc.settings.assign({ survey: ${survey} })`);
  await ui.eval(`import('/src/ui/calc/FieldBookDialog.ts').then((m) => m.openFieldBook(window.kentos, new File([Uint8Array.from(atob('${Buffer.from(bytes).toString('base64')}'), (c) => c.charCodeAt(0))], '${name}')))`);
  await ui.waitFor(`!!document.querySelector('.dialog--fieldbook .fieldbook-file .io-file')`);
  await ui.sleep(400);
};
const fieldBookClose = async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.doc.settings.assign({ survey: null })`));
SCENES.fieldbook = [
  { id: 'fieldbook-gsi', open: (ui) => fieldBookOpen(ui, 'sample.gsi', SAMPLE_GSI), close: fieldBookClose },
  // The same book from a Sokkia SDR33, a Topcon GTS-7, a Nikon RAW and a Trimble JobXML file (docs/adr/0169 §1, step 5).
  { id: 'fieldbook-sdr', open: (ui) => fieldBookOpen(ui, 'sample.sdr', SAMPLE_SDR), close: fieldBookClose },
  { id: 'fieldbook-gts7', open: (ui) => fieldBookOpen(ui, 'sample.gt7', SAMPLE_GT7), close: fieldBookClose },
  { id: 'fieldbook-nikon', open: (ui) => fieldBookOpen(ui, 'sample-nikon.raw', SAMPLE_NIKON), close: fieldBookClose },
  { id: 'fieldbook-jobxml', open: (ui) => fieldBookOpen(ui, 'sample.jxl', SAMPLE_JXL), close: fieldBookClose },
  { id: 'fieldbook-csv', open: (ui) => fieldBookOpen(ui, 'arazi-karnesi.csv', Buffer.from('İstasyon;Alet;Nokta;Hz;V;SD;Prizma;Kod\nS1;1,55;A;10;99;100;1,6;POL\n')), close: fieldBookClose },
  {
    // Kutupsal alım'a aktar: Kutupsal alım filled from ST1, the station by the file's coordinates.
    id: 'fieldbook-polar',
    open: async (ui) => (await fieldBookOpen(ui, 'sample.gsi', SAMPLE_GSI), await ui.clickText('.dialog--fieldbook .btn', "Kutupsal alım'a aktar"), await ui.sleep(500)),
    close: fieldBookClose,
  },
  {
    // The traverse ST1 → ST2 ending on P9, its leg's two-way difference above 3 mm.
    id: 'fieldbook-legs',
    open: async (ui) => (await fieldBookOpen(ui, 'sample.gsi', SAMPLE_GSI, TRAVERSE_SURVEY), await chooseFore(ui), await bodyToEnd(ui, '.dialog--fieldbook'), await ui.sleep(200)),
    close: fieldBookClose,
  },
  {
    // Poligon hesabı'na aktar: ST1 → ST2, ending oriented on P9; the misclosures against the project's tolerances.
    id: 'fieldbook-traverse',
    open: async (ui) => {
      await fieldBookOpen(ui, 'sample.gsi', SAMPLE_GSI, TRAVERSE_SURVEY);
      await chooseFore(ui);
      await ui.clickText('.dialog--fieldbook .btn', "Poligon hesabı'na aktar");
      await ui.sleep(500);
    },
    close: fieldBookClose,
  },
  {
    // Poligon hesabı's misclosures against the project's tolerances of 6 cc and 10 mm: a connected traverse worked out
    // from its points' coordinates with 9 cc and 4 mm of error in it (the desktop's hesap-poligon-tolerans).
    id: 'traverse-tolerance',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.settings.assign({ survey: { traverseAngle: ${(6 * Math.PI) / 2_000_000}, traverseCoord: 0.01 } })`);
      const fill = {
        kind: 'connected',
        endOriented: true,
        start: '486513.341,4420189.522',
        back: '486535.757,4420188.723',
        end: '486538.221,4420218.986',
        fore: '486514.344,4420220.532',
        first: { name: '', angle: '330.3870', distance: '13.304' },
        rows: [
          { name: 'Y1', angle: '224.5472', distance: '10.905' },
          { name: 'Y2', angle: '188.9591', distance: '14.809' },
        ],
        last: { name: '', angle: '57.9557', distance: '' },
      };
      await ui.eval(`import('/src/ui/calc/TraverseDialog.ts').then((m) => m.openTraverseWith(window.kentos, ${JSON.stringify(fill)}))`);
      await ui.waitFor(`!!document.querySelector('.dialog--calc .io-summary')`);
      await bodyToEnd(ui, '.dialog--calc');
      await ui.sleep(400);
    },
    close: fieldBookClose,
  },
];

/**
 * Özel koordinat sistemi (docs/adr/0168 §6) with the mouse and the keyboard: Proje ayarları' Özel sistem… opens the
 * window: new; a TM on the project's own Bessel datum bound by seven parameters; what is wrong said under its field; a
 * local system on TUREF TM30 by an affine; then Proje ayarları with the definition chosen (its card, Düzenle, its row)
 * and the second system's list with a second definition and Özel sistem…. The desktop's are
 * `project::custom_crs::tests::screens` (ozel-crs-*).
 */
const DEF = '.dialog--custom-crs';
// fixtures/crs/v1/definition-fit.json's “benzerlik: beş nokta, gürültülü”, as a spreadsheet copies it.
const COMMON_POINTS = "1000.000\t2000.000\t412984.400\t4523008.270\n1250.500\t2010.250\t413234.809\t4523020.495\n1240.000\t2300.750\t413222.031\t4523310.900\n990.125\t2280.500\t412972.316\t4523288.690\n1120.000\t2150.000\t413103.218\t4523159.214";
const TM36_WKT =
  'PROJCS["TUREF_TM36",GEOGCS["GCS_TUREF",DATUM["D_Turkish_National_Reference_Frame",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",36.0],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]]';
const defOpen = async (ui) => {
  await ui.eval(`import('/src/ui/settings/ProjectSettingsDialog.ts').then((m) => m.openProjectSettings(window.kentos, 'crs'))`);
  await ui.waitFor(`!!document.querySelector('.crs-list__action')`);
  await ui.clickSel('.crs-list__action');
  await ui.waitFor(`!!document.querySelector('${DEF}')`);
  await ui.sleep(200);
};
const defType = async (ui, label, text) => (await ui.clickSel(`${DEF} [aria-label="${label}"]`), await ui.type(text));
const defChoose = async (ui, group, option) => (await ui.clickText(`${DEF} [aria-label="${group}"] .seg__opt`, option), await ui.sleep(150));
const defPick = async (ui, dropdown, item) => (await ui.clickSel(`${DEF} [aria-label="${dropdown}"]`), await ui.clickText('.menu__item', item), await ui.sleep(150));
const defTm = async (ui) => {
  await defType(ui, 'Ad', 'Bessel TM27');
  for (const [label, v] of [['Orta meridyen (°)', '27'], ['Ölçek', '1'], ['Sağa öteleme (m)', '500000'], ['Yukarı öteleme (m)', '0']]) await defType(ui, label, v);
  await defChoose(ui, 'Datum', 'Projenin datumu');
  await defType(ui, 'Datumun adı', 'Bessel datumu');
  await defPick(ui, 'Elipsoid', 'Bessel 1841');
  for (const [label, v] of [['ΔX (m)', '674.374'], ['ΔY (m)', '15.056'], ['ΔZ (m)', '405.346'], ['Doğruluk (m)', '1']]) await defType(ui, label, v);
};
const defLocal = async (ui) => {
  await defType(ui, 'Ad', 'Belediye yerel');
  await defChoose(ui, 'Tür', 'Yerel (taban sisteme bağlı)');
  await defPick(ui, 'Taban sistem', 'TUREF / TM30');
  await defChoose(ui, 'Düzlem dönüşümü', 'Afin');
  for (const [label, v] of [['a', '1.0000215'], ['b', '-0.0003871'], ['c', '412000'], ['d', '0.0003871'], ['e', '1.0000215'], ['f', '4521000']]) await defType(ui, label, v);
};
const defDone = async (ui) => (await ui.clickText(`${DEF} .btn`, 'Tamam'), await ui.sleep(300));
const defClose = async (ui) => (
  await ui.escapeAll(3), await ui.eval(`window.kentos.doc.settings.assign({ srid: 5256, customCrs: null, secondSrid: null, secondCustomCrs: null })`)
);
SCENES.definitions = [
  { id: 'definition-new', open: defOpen, close: defClose },
  { id: 'definition-tm', open: async (ui) => (await defOpen(ui), await defTm(ui)), close: defClose },
  {
    id: 'definition-problems',
    open: async (ui) => {
      await defOpen(ui);
      for (const [label, v] of [['Ad', 'Şantiye'], ['Orta meridyen (°)', '300'], ['Ölçek', '0']]) await defType(ui, label, v);
    },
    close: defClose,
  },
  { id: 'definition-local', open: async (ui) => (await defOpen(ui), await defLocal(ui)), close: defClose },
  { id: 'definition-chosen', open: async (ui) => (await defOpen(ui), await defTm(ui), await defDone(ui)), close: defClose },
  {
    id: 'definition-text',
    open: async (ui) => {
      await defOpen(ui);
      await defType(ui, 'WKT ya da PROJ metni', TM36_WKT);
      await ui.clickText(`${DEF} .btn`, 'Al');
      await ui.sleep(250);
      // What was read and the registry's system it is, with Kayıttakini seç, in view.
      await ui.eval(`document.querySelector('${DEF} .custom-crs__notes')?.scrollIntoView({ block: 'end' })`);
      await ui.sleep(200);
    },
    close: defClose,
  },
  {
    id: 'definition-text-error',
    open: async (ui) => {
      await defOpen(ui);
      await defType(ui, 'WKT ya da PROJ metni', '+proj=lcc +lat_1=36 +lat_2=42 +lon_0=33 +units=m');
      await ui.clickText(`${DEF} .btn`, 'Al');
      await ui.sleep(250);
    },
    close: defClose,
  },
  {
    id: 'definition-trial',
    open: async (ui) => {
      await defOpen(ui);
      for (const [label, v] of [['Ad', 'Kaydırılmış TM36'], ['Orta meridyen (°)', '36'], ['Sağa öteleme (m)', '400000'], ['Yukarı öteleme (m)', '0']]) await defType(ui, label, v);
      await defType(ui, 'Deneme noktası Y', '412345.678');
      await defType(ui, 'Deneme noktası X', '4421234.567');
      await ui.sleep(250);
      await ui.eval(`document.querySelector('${DEF} .custom-crs__trial')?.scrollIntoView({ block: 'end' })`);
      await ui.sleep(200);
    },
    close: defClose,
  },
  {
    id: 'definition-common',
    open: async (ui) => {
      await defOpen(ui);
      await defType(ui, 'Ad', 'Belediye yerel');
      await defChoose(ui, 'Tür', 'Yerel (taban sisteme bağlı)');
      await defPick(ui, 'Taban sistem', 'TUREF / TM30');
      // Five points pasted from a spreadsheet into the first cell, the third left out, the plane written.
      await ui.eval(`(() => {
        const cell = document.querySelector('${DEF} .calc-grid input[data-row="0"][data-key="le"]');
        const data = new DataTransfer();
        data.setData('text/plain', ${JSON.stringify(COMMON_POINTS)});
        cell.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
      })()`);
      await ui.sleep(200);
      await ui.clickSel(`${DEF} .calc-grid tr[data-row="2"] input[type="checkbox"]`);
      await ui.clickText(`${DEF} .btn`, 'Düzleme yaz');
      await ui.sleep(250);
      await ui.eval(`document.querySelector('${DEF} .custom-crs__fit')?.scrollIntoView({ block: 'end' })`);
      await ui.sleep(200);
    },
    close: defClose,
  },
  {
    id: 'definition-second',
    open: async (ui) => {
      await defOpen(ui);
      await defTm(ui);
      await defDone(ui);
      await ui.clickSel('[aria-label="İkinci koordinat sistemi"]');
      await ui.clickText('.menu__item', 'Özel sistem…');
      await ui.waitFor(`!!document.querySelector('${DEF}')`);
      await defLocal(ui);
      await defDone(ui);
      await ui.clickSel('[aria-label="İkinci koordinat sistemi"]');
      await ui.sleep(250);
    },
    close: defClose,
  },
];

SCENES.customcrs = [
  { id: 'custom-second', open: async (ui) => (await ui.eval(customSecond), await overDrawing(ui)), close: customClose },
  {
    id: 'custom-second-menu',
    open: async (ui) => {
      await ui.eval(customSecond);
      const at = await ui.eval(`(() => { const r = document.querySelector('.ribbon__crs').getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      await ui.contextClick(...at);
      await ui.hoverText('.menu__item', 'İkinci sistem');
    },
    close: customClose,
  },
  { id: 'custom-own', open: async (ui) => (await ui.eval(customOwn), await overDrawing(ui)), close: customClose },
  {
    id: 'custom-own-read',
    open: async (ui) => {
      await ui.eval(customOwn);
      await ui.eval(`window.kentos.commands.execute('crs.query')`);
      const [w, h] = await ui.eval(`[innerWidth, innerHeight]`);
      await ui.clickAt(Math.round(w * 0.42), Math.round(h * 0.48));
      await ui.sleep(300);
      await ui.eval(settle);
      await ui.sleep(200);
    },
    close: customClose,
  },
];

/**
 * Koordinat dönüştür (docs/adr/0167 §4) over the demo drawing (TUREF TM36), its second system ED50 TM36: a point typed,
 * converted to ED50 TM36 and to WGS 84 in degrees, minutes and seconds; the target's list of systems; a list with a
 * row it cannot read; a project whose system is its own definition (“Şantiye”), its second ED50 TM36 by the project's
 * datum choice, and the source's list with the project's definitions first (docs/adr/0168 §9 3c). The desktop's are
 * `calc::convert::tests::screens` (donustur-*).
 */
const OPEN_CONVERT = `window.kentos.commands.execute('crs.transform')`;
const CONVERT_OPEN = `!!document.querySelector('.dialog--calc .calc-convert-systems')`;
const typeInto = (sel, i, text) =>
  `(() => { const el = document.querySelectorAll(${JSON.stringify(sel)})[${i}]; el.value = ${JSON.stringify(text)}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`;
// The window keeps what was typed while the page is open: each scene says its target and mode.
const chooseTarget = async (ui, code) => {
  await ui.clickSel('.dialog--calc .calc-convert-systems > :last-child .dropdown');
  await ui.clickText('.menu__item', code);
  await ui.sleep(250);
};
const convertPoint = async (ui) => {
  await ui.eval(SECOND(2322));
  await ui.eval(OPEN_CONVERT);
  await ui.waitFor(CONVERT_OPEN);
  await chooseTarget(ui, 'EPSG:2322');
  await ui.clickText('.dialog--calc .seg__opt', 'Tek nokta');
  await ui.eval(typeInto('.dialog--calc .calc-convert-point input', 0, '486512.34'));
  await ui.eval(typeInto('.dialog--calc .calc-convert-point input', 1, '4420187.52'));
  await ui.sleep(250);
};
// The window keeps its systems while the page is open: the scene chooses the project's definition as the source.
const convertCustom = async (ui) => {
  await ui.eval(customOwn);
  await ui.eval(OPEN_CONVERT);
  await ui.waitFor(CONVERT_OPEN);
  await ui.clickSel('.dialog--calc .calc-convert-systems > :first-child .dropdown');
  await ui.clickText('.menu__item', 'Şantiye');
  await ui.sleep(250);
  await chooseTarget(ui, 'EPSG:2322');
  await ui.clickText('.dialog--calc .seg__opt', 'Tek nokta');
  await ui.eval(typeInto('.dialog--calc .calc-convert-point input', 0, '486512.34'));
  await ui.eval(typeInto('.dialog--calc .calc-convert-point input', 1, '4420187.52'));
  await ui.sleep(250);
};
const convertCustomClose = customClose;
SCENES.convert = [
  { id: 'convert-point', open: convertPoint, close: secondClose },
  { id: 'convert-custom', open: convertCustom, close: convertCustomClose },
  {
    id: 'convert-custom-systems',
    open: async (ui) => (await convertCustom(ui), await ui.clickSel('.dialog--calc .calc-convert-systems > :first-child .dropdown'), await ui.sleep(300)),
    close: convertCustomClose,
  },
  { id: 'convert-wgs84', open: async (ui) => (await convertPoint(ui), await chooseTarget(ui, 'EPSG:4326')), close: secondClose },
  {
    id: 'convert-systems',
    open: async (ui) => (await convertPoint(ui), await ui.clickSel('.dialog--calc .calc-convert-systems > :last-child .dropdown'), await ui.sleep(300)),
    close: secondClose,
  },
  {
    id: 'convert-list',
    open: async (ui) => {
      await convertPoint(ui);
      await ui.clickText('.dialog--calc .seg__opt', 'Liste');
      await ui.waitFor(`!!document.querySelector('.dialog--calc .calc-grid input, .dialog--calc table input')`);
      const cells = '.dialog--calc .calc-section input';
      const rows = [['P1', '486512.34', '4420187.52'], ['P2', '486530.25', '4420150.80'], ['P3', '48650x', '4420120.00']];
      for (const [r, row] of rows.entries()) for (const [c, v] of row.entries()) await ui.eval(typeInto(cells, r * 3 + c, v));
      await ui.sleep(300);
    },
    close: secondClose,
  },
];

// Each project type's own ribbon (docs/adr/0165 §6), every tab, over the demo drawing: a CAD project's AutoCAD drafting
// tabs and a CBS project's map work's (the desktop's are ribbon_tests::screens, serit-cad-* and serit-cbs-*).
const TYPE_TABS = {
  cad: ['file', 'home', 'insert', 'annotate', 'modify', 'view', 'manage', 'output'],
  gis: ['file', 'home', 'map', 'data', 'edit', 'analysis', 'survey', 'view', 'output'],
};
SCENES.types = Object.entries(TYPE_TABS).flatMap(([type, tabs]) =>
  tabs.map((tab) => ({
    id: `ribbon-${type === 'gis' ? 'cbs' : type}-${tab}`,
    open: async (ui) => (await ribbonOn(ui, { type, ribbonTab: tab }), await ui.move(2, 2), await ui.sleep(300)),
    close: (ui) => ribbonOff(ui),
  })),
);

/**
 * GNSS içe aktar (docs/adr/0169 §6): fixtures/gnss/v1/sample.gpx and sample.nmea (the sample drawing's parcel corners
 * measured by a receiver, scripts/fixtures/gnss_samples.py) into the TUREF / TM36 sample; the GPX's points imported
 * onto the drawing; the same window in a project without a coordinate system. The desktop's are
 * `exchange::gnss_tests::screens` (gnss-*).
 */
const gnssOpen = async (ui, name) => {
  const bytes = readFileSync(new URL(`../../../../fixtures/gnss/v1/${name}`, import.meta.url)).toString('base64');
  await ui.eval(
    `import('/src/ui/io/GnssImportDialog.ts').then((m) => m.openGnssImport(window.kentos, { name: '${name}', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'GNSS', accept: {} }))`,
  );
  await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary') && !document.querySelector('.dialog--io .io-summary')?.textContent?.includes('okunuyor')`);
  await ui.sleep(400);
};
const gnssUndo = `(() => { const k = window.kentos; while (k.doc.canUndo.value) k.doc.undo(); })()`;
SCENES.gnss = [
  { id: 'gnss-gpx', open: (ui) => gnssOpen(ui, 'sample.gpx'), close: (ui) => ui.escapeAll(2) },
  {
    id: 'gnss-imported',
    open: async (ui) => (await gnssOpen(ui, 'sample.gpx'), await ui.clickText('.dialog--io .btn', 'İçe aktar'), await ui.sleep(600)),
    close: async (ui) => (await ui.escapeAll(1), await ui.eval(gnssUndo)),
  },
  { id: 'gnss-nmea', open: (ui) => gnssOpen(ui, 'sample.nmea'), close: (ui) => ui.escapeAll(2) },
  {
    id: 'gnss-nosystem',
    open: async (ui) => (await ui.eval(`window.kentos.doc.settings.assign({ srid: 0 })`), await gnssOpen(ui, 'sample.gpx')),
    close: async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.doc.settings.assign({ srid: 5256 })`)),
  },
];

/**
 * Cihaza gönder (docs/adr/0169 §4): the sample's points from Nokta editörü's rows as Leica GSI-16 and as Trimble JobXML,
 * and the selected points as Leica GSI-8, whose eight digits do not hold TM coordinates. The desktop's are
 * `exchange::field_send_tests::screens` (cihaza-*).
 */
const fieldSendOpen = async (ui, format) => {
  await ui.eval(`(() => {
    const k = window.kentos;
    const ids = [...k.doc.all()].filter((e) => e.kind === 'point').slice(0, 40).map((e) => e.id);
    k.selection.set(ids);
    k.commands.execute('field.send');
  })()`);
  await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary .io-summary__line')`);
  await ui.eval(`(() => { const s = document.querySelector('.dialog--io select[aria-label="Biçim"]'); s.value = '${format}'; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
  await ui.sleep(500);
};
const fieldSendClose = async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.selection.set([])`));
SCENES.fieldsend = [
  { id: 'fieldsend-gsi16', open: (ui) => fieldSendOpen(ui, 'gsi16'), close: fieldSendClose },
  { id: 'fieldsend-jobxml', open: (ui) => fieldSendOpen(ui, 'jobxml'), close: fieldSendClose },
  { id: 'fieldsend-gsi8', open: (ui) => fieldSendOpen(ui, 'gsi8'), close: async (ui) => (await fieldSendClose(ui), await ui.eval(`(() => { const s = document.querySelector('.dialog--io select[aria-label="Biçim"]'); if (s) { s.value = 'gsi16'; s.dispatchEvent(new Event('change', { bubbles: true })); } })()`)) },
];

/**
 * Uzunlukları projeksiyona indir (docs/adr/0171 §4): Proje ayarları' Ölçme with the mean ellipsoidal height typed and the
 * switch on, then Kutupsal alım, Aplikasyon and Poligon hesabı with the project reducing to TM36 at 850 m: the ground and
 * the grid lengths side by side with their factors. The desktop's are `project::survey::tests::screens` (olcme-ayar-indir)
 * and `calc::tests::screens` (hesap-*-zemin).
 */
const GROUND_SURVEY = '{ groundHeight: 850, reduceToGrid: true }';
const groundClose = async (ui) => (await ui.escapeAll(2), await ui.eval(`window.kentos.doc.settings.assign({ survey: null })`));
const setInput = (sel, text) =>
  `(() => { const el = document.querySelector(${JSON.stringify(sel)}); el.value = ${JSON.stringify(text)}; el.dispatchEvent(new Event('input', { bubbles: true })); })()`;
SCENES.ground = [
  {
    id: 'ground-settings',
    open: async (ui) => {
      await surveyOpen(ui, [['Ortalama elipsoit yüksekliği', '850']]);
      await ui.clickSel('[aria-label="Uzunlukları projeksiyona indir"]');
      await ui.sleep(250);
    },
    close: (ui) => ui.escapeAll(3),
  },
  {
    id: 'ground-polar',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.settings.assign({ survey: ${GROUND_SURVEY} })`);
      const fill = {
        station: '486513.341,4420189.522',
        back: '486535.757,4420188.723',
        backReading: '0',
        stationZ: '812,40',
        instrumentHeight: '1.55',
        rows: [
          { name: 'K1', reading: '327.7849', distance: '17.727', zenith: '98.4410', target: '1.70' },
          { name: 'K2', reading: '372,1872', distance: '18.225' },
          { name: 'K3', reading: '288.3598', distance: '25.661', zenith: '101.2215', target: '1.70' },
        ],
      };
      await ui.eval(`import('/src/ui/calc/PolarDialog.ts').then((m) => m.openPolarWith(window.kentos, ${JSON.stringify(fill)}))`);
      await ui.waitFor(`!!document.querySelector('.dialog--calc .io-summary')`);
      await bodyToEnd(ui, '.dialog--calc');
      await ui.sleep(400);
    },
    close: groundClose,
  },
  {
    id: 'ground-stakeout',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.settings.assign({ survey: ${GROUND_SURVEY} })`);
      await ui.eval(`import('/src/ui/calc/StakeoutDialog.ts').then((m) => m.openStakeout(window.kentos))`);
      await ui.waitFor(`!!document.querySelector('.dialog--calc .calc-grid')`);
      await ui.eval(setInput('.dialog--calc input[data-key="station"]', '486513.341,4420189.522'));
      await ui.eval(setInput('.dialog--calc input[data-key="back"]', '486535.757,4420188.723'));
      await ui.eval(setInput('.dialog--calc .calc-grid input[data-row="0"][data-key="point"]', '486538.221,4420218.986'));
      await ui.eval(setInput('.dialog--calc .calc-grid input[data-row="1"][data-key="point"]', '486514.344 4420220.532'));
      await ui.sleep(300);
      await bodyToEnd(ui, '.dialog--calc');
      await ui.sleep(300);
    },
    close: async (ui) => {
      await ui.eval(setInput('.dialog--calc .calc-grid input[data-row="0"][data-key="point"]', ''));
      await ui.eval(setInput('.dialog--calc .calc-grid input[data-row="1"][data-key="point"]', ''));
      await groundClose(ui);
    },
  },
  {
    id: 'ground-traverse',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.settings.assign({ survey: ${GROUND_SURVEY} })`);
      const fill = {
        kind: 'connected',
        endOriented: true,
        start: '486513.341,4420189.522',
        back: '486535.757,4420188.723',
        end: '486538.221,4420218.986',
        fore: '486514.344,4420220.532',
        first: { name: '', angle: '330.3870', distance: '13.304' },
        rows: [
          { name: 'Y1', angle: '224.5472', distance: '10.905' },
          { name: 'Y2', angle: '188.9591', distance: '14.809' },
        ],
        last: { name: '', angle: '57.9557', distance: '' },
      };
      await ui.eval(`import('/src/ui/calc/TraverseDialog.ts').then((m) => m.openTraverseWith(window.kentos, ${JSON.stringify(fill)}))`);
      await ui.waitFor(`!!document.querySelector('.dialog--calc .io-summary')`);
      await bodyToEnd(ui, '.dialog--calc');
      await ui.sleep(400);
    },
    close: groundClose,
  },
];

/** Closer in: the view centred on `x`, `y` at `times` the whole scene's scale. */
const closeIn = (x, y, times) => `(() => { const c = window.kentos.view.camera; c.center = { x: ${x}, y: ${y} }; c.scale = c.scale * ${times}; c.panBy(0, 0); window.kentos.view.requestRender(); })()`;
SCENES.texts = [
  { id: 'text-extras', open: openTextExtras },
  { id: 'text-extras-mask', open: async (ui) => (await openTextExtras(ui), await ui.eval(closeIn(487108, 4419985, 2.5 / 0.85)), await ui.sleep(400)) },
  { id: 'text-extras-turned', open: async (ui) => (await openTextExtras(ui), await ui.eval(closeIn(487118, 4420025, 5 / 0.85)), await ui.sleep(400)) },
  ...textToolScenes(),
  ...dxfTextScenes(),
];

// DXF içe aktar over the texts' fixture (fixtures/formats/v1/texts.dxf, docs/adr/0145 §7), the way a user works it:
// the window says what became of the aligned and fitted texts; in, every text selected so that its grip shows where it
// stands; out again, the window says the masks are KentOS's. The desktop's are `exchange::tests::screens` (aktar-*-15…18).
function dxfTextScenes() {
  const bytes = readFileSync(new URL('../../../../fixtures/formats/v1/texts.dxf', import.meta.url)).toString('base64');
  const open = `import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'texts.dxf', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`;
  const read = async (ui) => (await ui.eval(open), await ui.waitFor(DXF_READ, 15000));
  const into = async (ui) => (await read(ui), await ui.clickText('.dialog--io .btn--primary', 'İçe aktar'), await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000));
  const close = async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL));
  return [
    { id: 'import-dxf-texts', open: async (ui) => (await read(ui), await ui.sleep(400)), close },
    // The twelve alignments and the MTEXTs, then the aligned, fitted, widened and masked ones with the blocks.
    ...[
      ['import-dxf-texts-aligns', '{ minX: -6, minY: -4, maxX: 68, maxY: 76 }'],
      ['import-dxf-texts-others', '{ minX: 92, minY: -4, maxX: 232, maxY: 26 }'],
    ].map(([id, frame]) => ({
      id,
      open: async (ui) => {
        await into(ui);
        await ui.eval(`(() => { const k = window.kentos; k.selection.set([...k.doc.all()].filter((e) => e.kind === 'text' && k.doc.layers.get(e.layerId)?.name === 'YAZI').map((e) => e.id)); k.view.camera.fit(${frame}, 24); k.view.requestRender(); })()`);
        await ui.move(2, 2);
        await ui.sleep(600);
      },
      close,
    })),
    {
      id: 'export-dxf-texts',
      open: async (ui) => {
        await into(ui);
        await ui.eval(`window.kentos.commands.execute('file.export.dxf')`);
        await ui.waitFor(`!!document.querySelector('.dialog--io .io-summary')`, 15000);
        await ui.clickText('.dialog--io .seg__opt', 'Tümü');
        await ui.sleep(400);
      },
      close,
    },
  ];
}

// Yazı's options and Öznitelikler's text rows (docs/adr/0145 §6, step 4a): the tool with Hiza sağ üst, Genişlik 0.8,
// Zemin and Artır on, its box about the pointer; the Hiza menu from the command line's chip; one text's rows and the
// Hiza drop-down; two texts that differ (Çeşitli). The texts stand on bare ground over a line and a square.
function textToolScenes() {
  const TEXTS = FIT(`
    const P = (fx, fy) => ({ x: c.x + fx * u, y: c.y + fy * u });
    const h = 0.16 * u;
    add({ kind: 'polygon', pts: [P(-2.2, -0.8), P(-0.4, -0.8), P(-0.4, 0.8), P(-2.2, 0.8)] });
    add({ kind: 'line', a: P(-2.5, 0), b: P(2.5, 0) });
    const first = add({ kind: 'text', p: P(-1.3, 0), text: 'Ada 101', height: h, rotation: 0, align: 'middleCenter', widthFactor: 0.8, mask: true });
    const second = add({ kind: 'text', p: P(0.9, 0.5), text: 'Yol 12', height: h, rotation: 20 });
    window.__texts = { a: first.id, b: second.id };`);
  const choose = (...names) => `(() => { const k = window.kentos; const o = window.__texts; k.selection.set([${names.map((n) => `o.${n}`).join(', ')}]); })()`;
  const ground = async (ui, fields) => {
    await ribbonOn(ui, { ribbonTab: 'edit', ...fields });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(TEXTS);
    await ui.eval(`window.kentos.log.clear()`);
  };
  /** The tool with its options set as a user sets them, the pointer where the next text goes. */
  const tool = async (ui) => {
    await ground(ui, LOGGED);
    await startTool(ui, 'text');
    await ui.eval(`(() => { const t = window.kentos.tools.active; t.chooseOption('H', 'sağ üst'); for (const o of ['G', '0.8', 'Z', 'R']) t.input(o); })()`);
    await hoverU(ui, 1.2, -0.55);
  };
  /** A section of Öznitelikler folded or opened, whichever it is not now. */
  const fold = async (ui, title, open) => {
    const head = `[...document.querySelectorAll('.panel--props .props__section')].find((b) => b.textContent.includes(${JSON.stringify(title)}))`;
    if (await ui.eval(`(() => { const b = ${head}; return !!b && (b.getAttribute('aria-expanded') === 'true') !== ${open}; })()`)) await ui.clickText('.panel--props .props__section', title);
  };
  /** Öznitelikler over the dock (the layer tree at its least) with Genel folded, the texts selected. */
  const props = async (ui, names) => {
    await ground(ui, { layersFraction: 0.15 });
    await ui.eval(choose(...names));
    await ui.move(2, 2);
    await ui.sleep(400);
    await fold(ui, 'Genel', false);
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  const menuOpen = async (ui, sel) => (await ui.clickSel(sel), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300));
  const close = async (ui) => (await fold(ui, 'Genel', true), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  /** Okunur yap (4b): two texts upside down (centred, and on the baseline's centre) and one that reads, all selected. */
  const UPSIDE = FIT(`
    const P = (fx, fy) => ({ x: c.x + fx * u, y: c.y + fy * u });
    const h = 0.16 * u;
    add({ kind: 'line', a: P(-2.5, 0), b: P(2.5, 0) });
    const ids = [
      add({ kind: 'text', p: P(-1.2, 0.3), text: 'Ada 101', height: h, rotation: 180, align: 'middleCenter' }),
      add({ kind: 'text', p: P(1.0, -0.2), text: 'Yol 12', height: h, rotation: 200, align: 'baselineCenter' }),
      add({ kind: 'text', p: P(0, -0.8), text: 'Park', height: h, rotation: 30 }),
    ].map((e) => e.id);
    k.selection.set(ids);`);
  /** Bul ve değiştir (4c) over the Okunur yap drawing and three more texts, one on the locked layer: Ada * → Parsel *. */
  const TEXTS_KCAD = readFileSync(new URL('../../../../fixtures/interaction/v1/texts.kcad', import.meta.url), 'utf8');
  const findWindow = async (ui) => {
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(TEXTS_KCAD)}, null))) throw new Error('texts.kcad did not load');
      const add = (layerId, text, x, y) => k.doc.add({ kind: 'text', layerId, attrs: {}, p: { x: 487000 + x, y: 4420000 + y }, text, height: 2, rotation: 0 });
      add('cizim', 'Ada 102', -24, -2);
      add('cizim', 'Ada 103', -24, -7);
      add('kilitli', 'Ada 9', 12, -12);
      k.selection.clear();
      k.view.zoomExtents();
    })()`);
    await ui.sleep(500);
    await ui.eval(`window.kentos.commands.execute('text.findReplace')`);
    await ui.waitFor(`!!document.querySelector('.dialog--find')`);
    await ui.clickSel('.dialog--find input[aria-label="Bul"]');
    await ui.type('Ada *');
    await ui.clickSel('.dialog--find input[aria-label="Değiştir"]');
    await ui.type('Parsel *');
    await ui.clickText('.dialog--find label.io-check', 'Joker');
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  /** Metin dosyası yerleştir (4d): a file of parcel names, Yazı at 3 mm and orta sol; the lines' boxes about the pointer. */
  const PARCELS = ['Ada 101 Parsel 1', 'Ada 101 Parsel 2', 'Ada 101 Parsel 3', '', 'Ada 102 Parsel 1', 'Ada 102 Parsel 2'].join('\n');
  const textFile = async (ui) => {
    await ground(ui, LOGGED);
    await ui.eval(`(() => {
      const k = window.kentos;
      const bytes = new TextEncoder().encode(${JSON.stringify(PARCELS)});
      k.files.picker = { ...k.files.picker, open: async () => ({ name: 'parseller.txt', async getFile() { return new Blob([bytes]); } }) };
    })()`);
    await startTool(ui, 'text');
    await ui.eval(`(() => { const t = window.kentos.tools.active; t.input('Y'); t.input('3'); t.chooseOption('H', 'sol orta'); })()`);
    await startTool(ui, 'placeTextFile');
    await ui.sleep(300);
    await hoverAt(ui, ...(await ui.eval(FILE_AT)));
  };
  // The first line 3 × 1.5 × 3 m over Ada 101's line, so that the empty fourth falls on Ada 101 whatever `u` is
  // (the desktop's `text_scenes` puts it there too).
  const FILE_AT = FIT(`return [c.x - 1.6 * u, c.y + 13.5];`);
  const upside = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'edit', ...LOGGED });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(UPSIDE);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  return [
    { id: 'text-options', open: tool, close },
    {
      id: 'text-options-align-menu',
      // Hiza's chip, or in a narrow window Diğer's menu with Hiza's open over it.
      open: async (ui) => {
        await tool(ui);
        const chip = '.cmdline__chip[aria-haspopup="menu"]:not(.cmdline__more):not([hidden])';
        if (await ui.eval(`!!document.querySelector('${chip}')`)) return menuOpen(ui, chip);
        await menuOpen(ui, '.cmdline__more');
        await ui.hoverText('.menu .menu__item', 'Hiza');
      },
      close,
    },
    { id: 'text-props', open: async (ui) => props(ui, ['a']), close },
    { id: 'text-props-align-menu', open: async (ui) => (await props(ui, ['a']), await menuOpen(ui, '.panel--props [data-prop-key="geometry:Hiza"]')), close },
    { id: 'text-props-several', open: async (ui) => props(ui, ['a', 'b']), close },
    { id: 'find-replace', open: findWindow, close: async (ui) => (await ui.escapeAll(2), await ui.eval(UNDO_ALL)) },
    { id: 'text-file-preview', open: textFile, close },
    { id: 'text-file-done', open: async (ui) => (await textFile(ui), await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(FILE_AT)))))), await ui.move(2, 2)), close },
    { id: 'readable-selected', open: upside, close },
    { id: 'readable-done', open: async (ui) => (await upside(ui), await startTool(ui, 'readable'), await ui.move(2, 2)), close },
  ];
}

// Çok satırlı yazı (docs/adr/0182): the texts as the drawing shows them (a boxed one with every format over its mask,
// a turned centred one, a bottom-right one with tight lines); the tool's box between its corners; the editor with a
// paragraph typed and formatted, the drawing showing it as it will be, and its Renk menu; Öznitelikler's rows; an
// AutoCAD MTEXT read in (fixtures/formats/v1/mtext.dxf). The desktop's are `paragraph_editor::tests::screens`.
SCENES.paragraph = paragraphScenes();
SCENES.styles = stylesScenes();
// Tablo (docs/adr/0184): the ribbon's Tablo panel in a CAD project, Tablo ekle with each source (two parcels and three
// named points selected first), its placement hanging from the pointer, the table drawn (with a frame), Tabloyu
// düzenle with a range chosen, Öznitelikler's rows. The desktop's are `tables::tests::screens`.
SCENES.tables = tableScenes();

// Koordinat yaz (docs/adr/0185) in a CAD project at 1:500: two parcels and parcel 7's numbered corner points; Koordinat
// yaz's label at the cursor over a corner after one written, Köşelere koordinat yaz's labels at every corner before
// Enter, its numbers and the coordinate schedule hanging from the cursor, the drawing with both written. The desktop's
// are `tools_screens`' koordinat-* (apps/desktop/src/coordinate_scenes.rs).
SCENES.coordinates = coordinateScenes();

// Tarama ekleri (docs/adr/0186) in a CAD project at 1:500: three parcels, a building and a pool as islands and a
// parcel's number; Tarama over the first with ANSI31 and Yazılar on (the region and the pattern before the click),
// Desen's menu from its chip, the library's patterns and the gradients side by side, Çoklu tara over the three parcels
// before Enter, a tied hatch following its building moved, Öznitelikler's rows of a hatch and its Desen menu. The
// desktop's are `tools_screens`' tarama-* (apps/desktop/src/hatch_scenes.rs).
SCENES.hatches = hatchScenes();

// Seçim ekleri (docs/adr/0187) in a CAD project at 1:500: two parcels sharing an edge, a red parcel above them, a road, a
// point on their shared corner, a line and a circle. A click on the shared edge with Sıradakini seç's chip and its list;
// Çokgenle seç on Kesişenler before the last corner and its answer; Benzerini seç from the first parcel; Seçim süzgeci
// without Kapalı alan after a window over the drawing, and the Süzgeç cell's menu. The desktop's are `tools_screens`' secim-*
// (apps/desktop/src/selection_scenes.rs).
SCENES.selecting = selectingScenes();

// Nokta hesaplayıcı ekleri (docs/adr/0188) in a CBS project at 1:500, the shared trace's ground: a route with a straight
// and a quarter-turn arc, a line, the point named 101 and an angle's corner and arms. Çizgi runs and the calculator over
// it: Obje üzerinde nokta with the cursor beside the arc, Km ve sapma from 1+000, Mesafe ve eğim, Açıortay, and the
// command line's chip with its eleven constructions. The desktop's are `tools_screens`' hesap-*
// (apps/desktop/src/point_calc_scenes.rs).
SCENES.pointcalc = pointCalcScenes();

// Km yaz (docs/adr/0189) in a CAD project at 1:500, the shared trace's ground: a road axis with a straight and a
// quarter-turn arc, a line and a point. Km yaz on the axis, its stations faint; every 10 m with cross-sections and
// points; and what it wrote. The desktop's are `tools_screens`' km-yaz-* (apps/desktop/src/stationing_scenes.rs).
SCENES.stationing = stationingScenes();

// Orta hat (docs/adr/0190) in a CAD project at 1:500, the shared trace's ground: a road's two sides with a quarter-turn
// bend, a stream's banks closing in and a bank in two lines. The road's sides picked, their axis dashed edge by edge; the
// stream's axis sampled every 5 m; and what was written. The desktop's are `tools_screens`' orta-hat-*
// (apps/desktop/src/centerline_scenes.rs).
SCENES.centerline = centerlineScenes();

// Paralel kaydır (docs/adr/0191) in a CAD project at 1:500, the shared trace's ground: a square parcel, a trapeze and a
// road's polyline. The trapeze's top edge following the cursor, its distance and the area's new size beside it; Alan's
// question on the square; and what was written. The desktop's are `tools_screens`' paralel-kaydir-*
// (apps/desktop/src/edge_shift_scenes.rs).
SCENES.edgeshift = edgeShiftScenes();

// Resim nesnesi (docs/adr/0192) in a CAD project at 1:500 on the shared traces' drawing (fixtures/interaction/v1/
// images.kcad): an embedded site picture and a parcel. Resim ekle with its frame following the cursor (the picker gives
// the traces' logo.png); the logo placed 16 m wide and selected, Öznitelikler's rows; Resmi kırp with its rectangle
// following the cursor; the site clipped to its middle, a third see-through, selected. The desktop's are
// `tools_screens`' resim-* (apps/desktop/src/image_scenes.rs).
const IMAGES = readFileSync(new URL('../../../../fixtures/interaction/v1/images.kcad', import.meta.url), 'utf8');
const LOGO = [...readFileSync(new URL('../../../../fixtures/interaction/v1/logo.png', import.meta.url))];
SCENES.images = imageScenes();

// Yazı ve ölçü stilleri (docs/adr/0183) in a CAD project at 1:500: a parcel drawn with the project's styles (its number
// bold in Arimo, the road's name in Barlow italic and slanted, a note in Courier Prime, its sides measured in Mimari
// (arrows, cm), Kadastro (ticks, the value centred, “L=”), Noktalı (dots) and Açık (open arrows), one dimension in
// Standart); the Yazı stilleri and Ölçü stilleri windows over it; Yazı with a style chosen and its Stil menu;
// Öznitelikler's Yazı stili row. The desktop's are `tools_screens`' stil-* (apps/desktop/src/style_scenes.rs).
function stylesScenes() {
  const ID = (n) => `0192f1a0-0000-7000-8000-0000000000${n}`;
  const TEXT_STYLES = [
    { id: ID('01'), name: 'Ada no', font: 'arimo', bold: true, height: 3.5, widthFactor: 0.9 },
    { id: ID('02'), name: 'Yol adı', font: 'barlow', italic: true, oblique: 12, height: 3 },
    { id: ID('03'), name: 'Not', font: 'courier-prime', height: 2 },
  ];
  const DIMENSION_STYLES = [
    { id: ID('d1'), name: 'Mimari', height: 2.5, arrow: 'closed', arrowSize: 3, extBeyond: 1.5, decimals: 0, unit: 'cm', suffix: ' cm', font: 'arimo' },
    { id: ID('d2'), name: 'Kadastro', height: 3, arrowSize: 2, textPlace: 'centre', decimals: 2, prefix: 'L=' },
    { id: ID('d3'), name: 'Noktalı', height: 2.5, arrow: 'dot', arrowSize: 2 },
    { id: ID('d4'), name: 'Açık', height: 2.5, arrow: 'open', font: 'overpass' },
  ];
  // Each object's look as the style gives it at 1:500 (its sizes times its value's height).
  const ADA = { textStyle: ID('01'), font: 'arimo', bold: true, widthFactor: 0.9, height: 1.75 };
  const YOL = { textStyle: ID('02'), font: 'barlow', italic: true, oblique: 12, height: 1.5 };
  const NOT = { textStyle: ID('03'), font: 'courier-prime', height: 1 };
  const LOOKS = [
    { dimStyle: ID('d1'), arrow: 'closed', arrowSize: 1.2, extBeyond: 0.6, decimals: 0, unit: 'cm', suffix: ' cm', font: 'arimo', height: 1.25 },
    { dimStyle: ID('d2'), arrowSize: 2 / 3, textPlace: 'centre', decimals: 2, prefix: 'L=', height: 1.5 },
    { dimStyle: ID('d3'), arrow: 'dot', arrowSize: 0.8, height: 1.25 },
    { dimStyle: ID('d4'), arrow: 'open', font: 'overpass', height: 1.25 },
  ];
  // The parcel's corners in metres from the view's middle, and the view about them (as the desktop's scene fits it).
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500, textStyles: ${JSON.stringify(TEXT_STYLES)}, dimensionStyles: ${JSON.stringify(DIMENSION_STYLES)} });
    const o = { x: c.x - 20, y: c.y - 8 };
    const P = (x, y) => ({ x: o.x + x, y: o.y + y });
    const C = [P(0, 0), P(40, 0), P(40, 26), P(0, 26)];
    add({ kind: 'polygon', pts: C });
    add({ kind: 'line', a: P(-6, -14), b: P(52, -14) });
    const ada = add({ kind: 'text', p: P(12, 12), text: 'Ada 101 Parsel 7', rotation: 0, ...${JSON.stringify(ADA)} });
    add({ kind: 'text', p: P(4, -12.4), text: 'Atatürk Caddesi', rotation: 0, ...${JSON.stringify(YOL)} });
    add({ kind: 'text', p: P(2, 22), text: "Not: ölçüler cm'dir", rotation: 0, ...${JSON.stringify(NOT)} });
    add({ kind: 'text', p: P(2, 3), text: 'Standart yazı', height: 1, rotation: 0 });
    const looks = ${JSON.stringify(LOOKS)};
    // Round the parcel anticlockwise: a side's left is inside, its dimension goes out (a negative offset).
    const sides = [[C[0], C[1], -5], [C[1], C[2], -6], [C[2], C[3], -5], [C[3], C[0], -6]];
    sides.forEach(([a, b, offset], i) => add({ kind: 'dimension', a, b, offset, ...looks[i] }));
    add({ kind: 'dimension', a: P(0, -14), b: P(40, -14), offset: -4, height: 1.25 });
    window.__styled = { ada: ada.id };
    k.view.camera.fit({ minX: o.x - 10, minY: o.y - 21, maxX: o.x + 50, maxY: o.y + 34 }, 24);
    k.view.requestRender();`);
  const ground = async (ui, fields = {}) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad', ...fields });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(500);
  };
  const close = async (ui) => (
    await ui.escapeAll(3),
    await ui.eval(UNDO_ALL),
    await ui.eval(`window.kentos.doc.settings.assign({ plotScale: 1000, textStyles: [], dimensionStyles: [] })`),
    await ribbonOff(ui)
  );
  /** A styles window over the drawing, `name`'s style chosen. */
  const window_ = async (ui, command, name) => {
    await ground(ui);
    await ui.eval(`window.kentos.commands.execute('${command}')`);
    await ui.waitFor(`!!document.querySelector('.dialog--annotation')`);
    await ui.clickText('.dialog--annotation .states-row__name', name);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  /** Yazı with Yol adı chosen, the pointer where the next text goes. */
  const tool = async (ui) => {
    await ground(ui, LOGGED);
    await startTool(ui, 'text');
    await ui.eval(`window.kentos.tools.active.chooseOption('S', 'Yol adı')`);
    await hoverAt(ui, ...(await ui.eval(SCRATCH('return [c.x - 14, c.y + 22];'))));
  };
  return [
    { id: 'styles-drawn', open: ground, close },
    { id: 'styles-text-window', open: (ui) => window_(ui, 'style.textStyles', 'Ada no'), close },
    { id: 'styles-dimension-window', open: (ui) => window_(ui, 'style.dimensionStyles', 'Mimari'), close },
    { id: 'styles-text-tool', open: tool, close },
    {
      id: 'styles-menu',
      open: async (ui) => {
        await tool(ui);
        const chip = '.cmdline__chip[aria-haspopup="menu"]:not(.cmdline__more):not([hidden])';
        await ui.clickText(chip, 'Stil');
        await ui.waitFor(`!!document.querySelector('.menu')`);
        await ui.sleep(300);
      },
      close,
    },
    {
      id: 'styles-props',
      open: async (ui) => (await ground(ui, { layersFraction: 0.15 }), await ui.eval(`window.kentos.selection.set([window.__styled.ada])`), await ui.move(2, 2), await ui.sleep(500)),
      close,
    },
  ];
}

function tableScenes() {
  const DRAWN = FIT(`
    const P = (fx, fy) => ({ x: c.x + fx * u, y: c.y + fy * u });
    const pa = add({ kind: 'polygon', pts: [P(-2.7, -1.3), P(-1.1, -1.5), P(-0.8, 0.1), P(-2.5, 0.3)], label: '101/5', zs: [812.4, 812.9, null, 813.35], attrs: { Ada: '101', Parsel: '5', Nitelik: 'Arsa' } });
    const pb = add({ kind: 'polygon', pts: [P(-1.1, -1.5), P(0.6, -1.4), P(0.7, 0.0), P(-0.8, 0.1)], label: '101/6', attrs: { Ada: '101', Parsel: '6', Nitelik: 'Bahçe' } });
    const pts = [P(1.3, -1.1), P(2.2, -0.7), P(1.7, 0.2)].map((p, i) => add({ kind: 'point', p, z: 811.5 + i * 0.25, label: String(201 + i) }));
    window.__tables = { a: pa.id, b: pb.id, pts: pts.map((e) => e.id) };`);
  const ground = async (ui, fields = {}) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad', ...fields });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  const close = async (ui) => (await ui.escapeAll(3), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  /** Tablo ekle with the parcels and points selected, the source card `source` chosen. */
  const insert = async (ui, source, after = async () => {}) => {
    await ground(ui);
    await ui.eval(`(() => { const t = window.__tables; window.kentos.selection.set([t.a, t.b, ...t.pts]); })()`);
    await ui.eval(`window.kentos.commands.execute('table.insert')`);
    await ui.waitFor(`!!document.querySelector('.dialog--table')`);
    await ui.clickSel(`.dialog--table [data-source="${source}"]`);
    await after();
    await ui.sleep(400);
  };
  /** The areas' schedule placed above the parcels, a frame round it. */
  const drawn = async (ui) => {
    await insert(ui, 'areas', async () => {
      await ui.clickSel('.dialog--table [data-grid="all"]');
      const on = await ui.eval(`document.querySelector('.dialog--table [data-key="frame"]').checked`);
      if (!on) await ui.clickSel('.dialog--table [data-key="frame"]');
    });
    await ui.clickText('.dialog--table .btn', 'Yerleştir');
    await ui.sleep(300);
    await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AU(-2.6, 1.45)))))));
    await ui.sleep(300);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  return [
    {
      id: 'table-ribbon',
      open: (ui) => ground(ui),
      close,
    },
    { id: 'table-insert-areas', open: (ui) => insert(ui, 'areas'), close },
    { id: 'table-insert-coords', open: (ui) => insert(ui, 'coordinates', async () => (await ui.clickSel('.dialog--table [data-grid="rows"]'))), close },
    { id: 'table-insert-attributes', open: (ui) => insert(ui, 'attributes'), close },
    { id: 'table-insert-blank', open: (ui) => insert(ui, 'blank'), close },
    { id: 'table-insert-file', open: (ui) => insert(ui, 'file'), close },
    {
      id: 'table-place',
      open: async (ui) => {
        await insert(ui, 'coordinates', async () => (await ui.clickSel('.dialog--table [data-grid="all"]')));
        await ui.clickText('.dialog--table .btn', 'Yerleştir');
        await hoverU(ui, 0.9, 1.5);
      },
      close,
    },
    { id: 'table-drawn', open: drawn, close },
    {
      id: 'table-drawn-selected',
      open: async (ui) => (await drawn(ui), await ui.eval(`(() => { const k = window.kentos; const t = [...k.doc.all()].find((e) => e.kind === 'table'); k.selection.set([t.id]); })()`), await ui.move(2, 2), await ui.sleep(400)),
      close,
    },
    {
      id: 'table-editor',
      open: async (ui) => {
        await drawn(ui);
        await ui.eval(`(() => { const k = window.kentos; const t = [...k.doc.all()].find((e) => e.kind === 'table'); k.selection.set([t.id]); k.commands.execute('table.edit'); })()`);
        await ui.waitFor(`!!document.querySelector('.dialog--table-editor')`);
        await ui.sleep(300);
        await ui.clickSel('.dialog--table-editor [data-row="1"][data-col="1"]');
        await ui.key('ArrowRight', { shift: true });
        await ui.key('ArrowDown', { shift: true });
        await ui.sleep(300);
      },
      close,
    },
    {
      id: 'table-props',
      open: async (ui) => (await drawn(ui), await ui.eval(`(() => { const k = window.kentos; const t = [...k.doc.all()].find((e) => e.kind === 'table'); k.selection.set([t.id]); })()`), await ui.move(2, 2), await ui.sleep(500)),
      close,
    },
  ];
}

function hatchScenes() {
  const A = [[0, 0], [30, 0], [30, 22], [0, 22]];
  const B = [[30, 0], [56, 0], [56, 22], [30, 22]];
  const C = [[56, 0], [80, 0], [80, 22], [56, 22]];
  const BUILDING = [[5, 5], [15, 5], [15, 13], [5, 13]];
  // The parcels in metres from a point left of the view's middle, the view about them (as the desktop's scene fits it).
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x - 40, y: c.y - 11 };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    const parcels = [${JSON.stringify(A)}, ${JSON.stringify(B)}, ${JSON.stringify(C)}].map((r) => add({ kind: 'polygon', pts: r.map(P) }));
    const building = add({ kind: 'polygon', pts: ${JSON.stringify(BUILDING)}.map(P) });
    add({ kind: 'text', p: P([20, 16]), text: '101', height: 2, rotation: 0 });
    add({ kind: 'circle', c: P([43, 11]), r: 3 });
    window.__hatch = { o, parcels: parcels.map((e) => e.id), building: building.id };
    k.view.camera.fit({ minX: o.x - 6, minY: o.y - 8, maxX: o.x + 86, maxY: o.y + 30 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__hatch.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const ground = async (ui, more = {}) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad', ...more });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  const close = async (ui) => (await ui.escapeAll(3), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  const input = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  const menuOpen = async (ui, sel) => (await ui.clickSel(sel), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300));
  /** Tarama with Yazılar on, the pointer in the first parcel: the region (the building an island, the number's box open). */
  const tool = async (ui) => {
    await ground(ui);
    await startTool(ui, 'hatch');
    await input(ui, 'Y');
    await hover(ui, 24, 4);
  };
  /** Yazılar back off (the session keeps it). */
  const toolClose = async (ui) => {
    await ui.escapeAll(3);
    await startTool(ui, 'hatch');
    if ((await ui.eval(`window.kentos.tools.active.prompt.value`)).includes('Yazılar (Y): boş bırakılır')) await input(ui, 'Y');
    await close(ui);
  };
  /** Every kind of pattern side by side: 14 × 10 m squares, each hatched and named beneath, four a row. */
  const SHOWN = [['ANSI31', 'ANSI31'], ['ANSI33', 'ANSI33'], ['ANSI36', 'ANSI36'], ['ANSI37', 'ANSI37'], ['ISO04W100', 'ISO04W100'], ['NET3', 'NET3'], ['BRICK', 'BRICK'], ['DOTS', 'DOTS'], ['CROSS', 'CROSS'], ['Degrade doğrusal', 'doğrusal'], ['Degrade silindir', 'silindir'], ['Degrade küre', 'küre']];
  const patterns = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(`(async () => {
      const k = window.kentos;
      const { hatchChoiceNamed, hatchToolPattern } = await import('/src/model/ops/hatchPatterns.ts');
      k.doc.settings.assign({ plotScale: 500 });
      const b = k.view.camera.visibleBounds();
      const o = { x: (b.minX + b.maxX) / 2 - 36, y: (b.minY + b.maxY) / 2 + 12 };
      const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
      const add = (e) => k.doc.add({ layerId: k.doc.layers.active.value, attrs: {}, ...e });
      ${JSON.stringify(SHOWN)}.forEach(([name, label], i) => {
        const [x, y] = [(i % 4) * 18, -Math.floor(i / 4) * 15];
        const ring = [[x, y], [x + 14, y], [x + 14, y + 10], [x, y + 10]].map(P);
        add({ kind: 'polygon', pts: ring });
        const pattern = hatchToolPattern({ choice: hatchChoiceNamed(name), scale: 1, angle: 0, color2: '#FFFFFF', inverted: false, plotScale: 500 });
        add({ kind: 'hatch', ring, pattern, ...(name.startsWith('Degrade') && { color: '#3E63DD' }) });
        add({ kind: 'text', p: P([x, y - 2.6]), text: label, height: 1.6, rotation: 0 });
      });
      k.view.camera.fit({ minX: o.x - 4, minY: o.y - 36, maxX: o.x + 76, maxY: o.y + 12 }, 24);
      k.view.requestRender();
      window.__hatch = { o };
    })()`);
    await ui.eval(`window.kentos.log.clear()`);
    await hover(ui, -3, 11);
  };
  /** A tied hatch in the first parcel (Yazılar as `texts`), the tool left. */
  const hatched = async (ui, texts) => {
    await ground(ui, texts ? { layersFraction: 0.15 } : {});
    await startTool(ui, 'hatch');
    if (texts) await input(ui, 'Y');
    await click(ui, 24, 4);
    if (texts) await input(ui, 'Y');
    await ui.escapeAll(2);
  };
  const fold = async (ui, title, open) => {
    const head = `[...document.querySelectorAll('.panel--props .props__section')].find((b) => b.textContent.includes(${JSON.stringify('Genel')}))`;
    if (await ui.eval(`(() => { const b = ${head}; return !!b && (b.getAttribute('aria-expanded') === 'true') !== ${open}; })()`)) await ui.clickText('.panel--props .props__section', title);
  };
  /** The tied hatch selected: Öznitelikler over the dock (the layer tree at its least) with Genel folded. */
  const props = async (ui) => {
    await hatched(ui, true);
    await ui.eval(`(() => { const k = window.kentos; const ids = [...k.doc.all()].filter((e) => e.kind === 'hatch').map((e) => e.id); k.selection.set(ids); })()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await fold(ui, 'Genel', false);
    await ui.move(2, 2);
    await ui.sleep(300);
  };
  const propsClose = async (ui) => (await fold(ui, 'Genel', true), await close(ui));
  return [
    { id: 'hatch-tool', open: tool, close: toolClose },
    {
      id: 'hatch-tool-pattern-menu',
      // Desen's chip, or in a narrow window Diğer's menu with Desen's open over it.
      open: async (ui) => {
        await tool(ui);
        const chip = '.cmdline__chip[aria-haspopup="menu"]:not(.cmdline__more):not([hidden])';
        if (await ui.eval(`!!document.querySelector('${chip}')`)) return menuOpen(ui, chip);
        await menuOpen(ui, '.cmdline__more');
        await ui.hoverText('.menu .menu__item', 'Desen');
      },
      close: toolClose,
    },
    { id: 'hatch-patterns', open: patterns, close },
    {
      id: 'hatch-selected',
      open: async (ui) => {
        await ground(ui);
        await ui.eval(`window.kentos.selection.set(window.__hatch.parcels)`);
        await startTool(ui, 'hatchSelected');
        await hover(ui, 40, 27);
      },
      close,
    },
    {
      id: 'hatch-follows',
      open: async (ui) => {
        await hatched(ui, false);
        await ui.eval(`window.kentos.selection.set([window.__hatch.building])`);
        await startTool(ui, 'move');
        await click(ui, ...BUILDING[0]);
        await typeValue(ui, '@8,0');
        await ui.eval(`window.kentos.selection.clear()`);
        await hover(ui, 40, 27);
      },
      close,
    },
    { id: 'hatch-props', open: props, close: propsClose },
    { id: 'hatch-props-pattern-menu', open: async (ui) => (await props(ui), await menuOpen(ui, '.panel--props [data-prop-key="geometry:Desen"]')), close: propsClose },
  ];
}

function stationingScenes() {
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x, y: c.y + 6 };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    add({ kind: 'polyline', pts: [[-40, -10], [-20, -10], [0, -10]].map(P), bulges: [0, Math.tan(Math.PI / 8)], color: '#E5484D' });
    add({ kind: 'line', a: P([-40, 10]), b: P([-20, 10]) });
    add({ kind: 'point', p: P([20, 10]) });
    window.__km = { o };
    k.view.camera.fit({ minX: o.x - 46, minY: o.y - 30, maxX: o.x + 26, maxY: o.y + 18 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__km.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  const shown = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await startTool(ui, 'stationLabels');
    await click(ui, -30, -10);
    await hover(ui, 12, 12);
  };
  const sections = async (ui) => {
    await shown(ui);
    for (const t of ['A', '10', 'E', '8', 'N', '4']) await ui.eval(`window.kentos.tools.active.input(${JSON.stringify(t)})`);
    await hover(ui, 12, 12);
  };
  /** The session's options as a new session has them. */
  const close = async (ui) => {
    await ui.eval(`(async () => {
      const { stationOptions } = await import('/src/tools/stationLabelTool.ts');
      Object.assign(stationOptions, { interval: 20, start: 0, text: 'left', heightMm: 2, tick: true, section: 0, point: null, ends: true });
    })()`);
    await ui.escapeAll(3);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  return [
    { id: 'stationing', open: shown, close },
    { id: 'stationing-sections', open: sections, close },
    {
      id: 'stationing-written',
      open: async (ui) => (await sections(ui), await ui.eval(`window.kentos.commands.execute('tool.confirm')`), await ui.sleep(300), await hover(ui, 12, 12)),
      close,
    },
  ];
}

function edgeShiftScenes() {
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x, y: c.y };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    add({ kind: 'polygon', pts: [[-30, -10], [-10, -10], [-10, 10], [-30, 10]].map(P), color: '#3E63DD' });
    add({ kind: 'polygon', pts: [[0, -10], [20, -10], [15, 0], [5, 0]].map(P), color: '#3E63DD' });
    add({ kind: 'polyline', pts: [[0, 10], [10, 10], [10, 20], [20, 20]].map(P), color: '#8C9AAA' });
    window.__ek = { o };
    k.view.camera.fit({ minX: o.x - 36, minY: o.y - 16, maxX: o.x + 26, maxY: o.y + 26 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__ek.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  const input = async (ui, list) => {
    for (const t of list) await ui.eval(`window.kentos.tools.active.input(${JSON.stringify(t)})`);
  };
  const opened = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'home', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await startTool(ui, 'edgeShift');
  };
  const shown = async (ui) => {
    await opened(ui);
    await click(ui, 10, 0);
    await hover(ui, 10, 3);
  };
  const area = async (ui) => {
    await opened(ui);
    await click(ui, -20, -10);
    await input(ui, ['A']);
    await hover(ui, -20, -12);
  };
  const written = async (ui) => {
    await shown(ui);
    await input(ui, ['A', '180']);
    await click(ui, -20, -10);
    await input(ui, ['2']);
    await click(ui, 10, 15);
    await click(ui, 13, 15);
    await hover(ui, 24, -6);
  };
  const close = async (ui) => {
    await ui.escapeAll(3);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  return [
    { id: 'edgeshift', open: shown, close },
    { id: 'edgeshift-area', open: area, close },
    { id: 'edgeshift-written', open: written, close },
  ];
}

function imageScenes() {
  const o = { x: 487000, y: 4420000 };
  const hover = async (ui, x, y) => hoverAt(ui, o.x + x, o.y + y);
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(o.x + x, o.y + y)))), await ui.sleep(300));
  const input = async (ui, list) => {
    for (const t of list) await ui.eval(`window.kentos.tools.active.input(${JSON.stringify(t)})`);
  };
  /** The traces' drawing open (a CAD project), the view round the site and the parcel; the picker gives the logo. */
  const opened = async (ui, tab, props = false) => {
    await ui.eval(`(async () => {
      const k = window.kentos;
      k.files.ask = async () => 'drop';
      if (!(await k.files.load(${JSON.stringify(IMAGES)}, null))) throw new Error('images.kcad did not load');
      k.selection.clear();
      k.files.picker.open = async () => ({ name: 'logo.png', async getFile() { return new Blob([new Uint8Array(${JSON.stringify(LOGO)})]); } });
    })()`);
    await ribbonOn(ui, { ribbonTab: tab, ...(props ? { layersFraction: 0.15 } : {}) });
    await ui.eval(`(() => { const k = window.kentos; k.view.camera.fit({ minX: ${o.x - 26}, minY: ${o.y - 20}, maxX: ${o.x + 46}, maxY: ${o.y + 26} }, 24); k.view.requestRender(); k.log.clear(); })()`);
    await ui.move(2, 2);
    await ui.sleep(900);
  };
  const fold = async (ui, title, open) => {
    const head = `[...document.querySelectorAll('.panel--props .props__section')].find((b) => b.textContent.includes(${JSON.stringify('Genel')}))`;
    if (await ui.eval(`(() => { const b = ${head}; return !!b && (b.getAttribute('aria-expanded') === 'true') !== ${open}; })()`)) await ui.clickText('.panel--props .props__section', title);
  };
  const inserting = async (ui, props = false) => {
    await opened(ui, 'insert', props);
    await startTool(ui, 'imageInsert');
    await ui.waitFor(`window.kentos.tools.active.prompt.value.includes('logo.png')`);
  };
  const insert = async (ui) => {
    await inserting(ui);
    await click(ui, 24, 8);
    await hover(ui, 41, 18);
  };
  const inserted = async (ui) => {
    await inserting(ui, true);
    await click(ui, 24, 8);
    await input(ui, ['16']);
    await ui.eval(`(() => { const k = window.kentos; const ids = [...k.doc.all()].filter((e) => e.kind === 'image').map((e) => e.id); k.selection.set([ids[ids.length - 1]]); })()`);
    await ui.sleep(400);
    await fold(ui, 'Genel', false);
    await hover(ui, 44, -14);
  };
  const clip = async (ui) => {
    await opened(ui, 'insert');
    await startTool(ui, 'imageClip');
    await click(ui, 0, -15);
    await click(ui, -12, -6);
    await hover(ui, 10, 10);
  };
  const clipped = async (ui) => {
    await opened(ui, 'insert', true);
    await startTool(ui, 'imageClip');
    await click(ui, 0, -15);
    await click(ui, -12, -6);
    await click(ui, 10, 10);
    await ui.escapeAll(2);
    await ui.eval(`(async () => {
      const k = window.kentos;
      const { setGeometry } = await import('/src/ui/properties/write.ts');
      const site = [...k.doc.all()].find((e) => e.kind === 'image');
      setGeometry(k, site, { opacity: 0.7 });
      k.selection.set([site.id]);
    })()`);
    await ui.sleep(400);
    await fold(ui, 'Genel', false);
    await hover(ui, 44, -14);
  };
  const close = async (ui) => {
    await fold(ui, 'Genel', true);
    await ui.escapeAll(3);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  return [
    { id: 'image-insert', open: insert, close },
    { id: 'image-inserted', open: inserted, close },
    { id: 'image-clip', open: clip, close },
    { id: 'image-clipped', open: clipped, close },
  ];
}

function centerlineScenes() {
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x, y: c.y };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    const bend = Math.tan(Math.PI / 8);
    add({ kind: 'polyline', pts: [[-40, -23], [-20, -23], [-13, -16]].map(P), bulges: [0, bend], color: '#8C9AAA' });
    add({ kind: 'polyline', pts: [[-40, -29], [-20, -29], [-7, -16]].map(P), bulges: [0, bend], color: '#8C9AAA' });
    add({ kind: 'line', a: P([-40, 4]), b: P([0, 4]), color: '#3E8ED0' });
    add({ kind: 'polyline', pts: [[-40, 12], [-20, 10], [0, 8]].map(P), color: '#3E8ED0' });
    add({ kind: 'line', a: P([-40, 24]), b: P([-20, 24]), color: '#3E8ED0' });
    add({ kind: 'line', a: P([-20, 24]), b: P([0, 24]), color: '#3E8ED0' });
    add({ kind: 'line', a: P([-40, 30]), b: P([0, 30]), color: '#3E8ED0' });
    window.__oh = { o };
    k.view.camera.fit({ minX: o.x - 48, minY: o.y - 34, maxX: o.x + 12, maxY: o.y + 34 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__oh.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  const input = async (ui, list) => {
    for (const t of list) await ui.eval(`window.kentos.tools.active.input(${JSON.stringify(t)})`);
  };
  const confirm = async (ui) => (await ui.eval(`window.kentos.commands.execute('tool.confirm')`), await ui.sleep(300));
  const shown = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'home', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await startTool(ui, 'centerline');
    await click(ui, -30, -23);
    await click(ui, -30, -29);
    await hover(ui, 6, -6);
  };
  const stream = async (ui) => {
    await shown(ui);
    await click(ui, -30, 4);
    await click(ui, -30, 11);
    await input(ui, ['B', '5']);
    await hover(ui, 6, -6);
  };
  const written = async (ui) => {
    await shown(ui);
    await confirm(ui);
    await click(ui, -30, 4);
    await click(ui, -30, 11);
    await input(ui, ['B', '5']);
    await confirm(ui);
    await click(ui, -30, 24);
    await click(ui, -30, 30);
    await confirm(ui);
    await hover(ui, 6, -6);
  };
  /** The session's options as a new session has them. */
  const close = async (ui) => {
    await ui.eval(`(async () => {
      const { centerlineOptions } = await import('/src/tools/centerlineTool.ts');
      Object.assign(centerlineOptions, { step: 1, chain: true });
    })()`);
    await ui.escapeAll(3);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  return [
    { id: 'centerline', open: shown, close },
    { id: 'centerline-stream', open: stream, close },
    { id: 'centerline-written', open: written, close },
  ];
}

function pointCalcScenes() {
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x, y: c.y + 4 };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    add({ kind: 'polyline', pts: [[-40, -10], [-20, -10], [0, -10]].map(P), bulges: [0, Math.tan(Math.PI / 8)], color: '#E5484D' });
    add({ kind: 'line', a: P([-40, 10]), b: P([-20, 10]) });
    add({ kind: 'point', p: P([20, 10]), label: '101' });
    for (const q of [[40, 10], [10, -30], [35, -30], [17, -6]]) add({ kind: 'point', p: P(q) });
    window.__calc = { o };
    k.view.camera.fit({ minX: o.x - 46, minY: o.y - 34, maxX: o.x + 46, maxY: o.y + 26 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__calc.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  /** The drawing in a CBS project, Çizgi started at its first point. */
  const ground = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'home', type: 'gis' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
    await startTool(ui, 'line');
    await click(ui, 44, 22);
  };
  const calc = (ui, kind) =>
    ui.eval(`(async () => { const { startPointCalc } = await import('/src/tools/pointCalc.ts'); startPointCalc(window.kentos, ${JSON.stringify(kind)}); })()`);
  const close = async (ui) => {
    await ui.escapeAll(4);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  return [
    { id: 'pointcalc-object', open: async (ui) => (await ground(ui), await calc(ui, 'object'), await click(ui, -38, -10), await hover(ui, -6, -18)), close },
    {
      id: 'pointcalc-km',
      open: async (ui) => {
        await ground(ui);
        await calc(ui, 'km');
        await click(ui, -30, -10);
        await ui.eval(`window.kentos.tools.active.input('B'); window.kentos.tools.active.input('1+000')`);
        await hover(ui, -14, -18);
      },
      close,
    },
    { id: 'pointcalc-slope', open: async (ui) => (await ground(ui), await calc(ui, 'slope'), await click(ui, 20, 10), await click(ui, 40, 10), await hover(ui, 29, 4)), close },
    {
      id: 'pointcalc-bisector',
      open: async (ui) => (await ground(ui), await calc(ui, 'bisector'), await click(ui, 10, -30), await click(ui, 35, -30), await click(ui, 17, -6), await hover(ui, 21, -14)),
      close,
    },
    {
      id: 'pointcalc-menu',
      open: async (ui) => (await ground(ui), await ui.clickSel('.cmdline__chip.cmdbar__calc'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
      close,
    },
  ];
}

function selectingScenes() {
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x, y: c.y - 2 };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    add({ kind: 'polygon', pts: [[-20, -10], [0, -10], [0, 10], [-20, 10]].map(P) });
    add({ kind: 'polygon', pts: [[0, -10], [20, -10], [20, 10], [0, 10]].map(P) });
    add({ kind: 'line', a: P([-25, -14]), b: P([25, -14]), color: '#E5484D' });
    add({ kind: 'point', p: P([0, 10]) });
    add({ kind: 'polygon', pts: [[-20, 12], [0, 12], [0, 18], [-20, 18]].map(P), color: '#E5484D' });
    add({ kind: 'line', a: P([5, 0]), b: P([15, 0]) });
    add({ kind: 'circle', c: P([-10, 0]), r: 3 });
    window.__sel = { o };
    k.view.camera.fit({ minX: o.x - 30, minY: o.y - 18, maxX: o.x + 30, maxY: o.y + 22 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__sel.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const ground = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'home', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  /** The session's filter and Çokgenle seç's mode as a new session has them. */
  const close = async (ui) => {
    await ui.escapeAll(3);
    await ui.eval(`(async () => {
      const k = window.kentos;
      const { FILTER_KINDS } = await import('/src/tools/selectable.ts');
      k.settings.selectFilter.set(false);
      k.settings.selectKinds.set(new Set(FILTER_KINDS));
      k.commands.execute('tool.selectPolygon');
      k.tools.active.input('i');
    })()`);
    await ui.escapeAll(3);
    await ui.eval(UNDO_ALL);
    await ribbonOff(ui);
  };
  const CORNERS = [[-24, -12], [4, -12], [4, 2], [-6, 2], [-6, 12], [-24, 12]];
  const chip = async (ui) => (await ground(ui), await click(ui, 0, 0), await hover(ui, 8, -5));
  /** Çokgenle seç on Kesişenler, five corners given, the pointer at the sixth. */
  const polygon = async (ui) => {
    await ground(ui);
    await startTool(ui, 'selectPolygon');
    await ui.eval(`window.kentos.tools.active.input('K')`);
    for (const [x, y] of CORNERS.slice(0, 5)) await click(ui, x, y);
    await hover(ui, ...CORNERS[5]);
  };
  /** Seçim süzgeci without Kapalı alan, then a window over the drawing: the rest selected, what it left out said. */
  const filtered = async (ui) => {
    await ground(ui);
    await ui.eval(`window.kentos.commands.execute('edit.selectFilter.polygon')`);
    const [a, b] = [await ui.eval(PAGE_AT(...(await ui.eval(AT(-28, -18))))), await ui.eval(PAGE_AT(...(await ui.eval(AT(28, 21)))))];
    await ui.drag(a[0], b[1], b[0], a[1]);
    await ui.sleep(300);
    await hover(ui, 26, 18);
  };
  return [
    { id: 'selection-chip', open: chip, close },
    {
      id: 'selection-chip-list',
      open: async (ui) => (await chip(ui), await ui.clickSel('.sel-chip'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
      close,
    },
    { id: 'selection-polygon', open: polygon, close },
    {
      id: 'selection-polygon-done',
      open: async (ui) => (await polygon(ui), await click(ui, ...CORNERS[5]), await ui.eval(`window.kentos.commands.execute('tool.confirm')`), await ui.sleep(300), await hover(ui, 26, 18)),
      close,
    },
    {
      id: 'selection-similar',
      open: async (ui) => (await ground(ui), await click(ui, -10, 6), await startTool(ui, 'selectSimilar'), await hover(ui, 26, 18)),
      close,
    },
    { id: 'selection-filter', open: filtered, close },
    { id: 'selection-filter-menu', open: async (ui) => (await filtered(ui), await rightClick(ui, '.status__toggle[data-command="edit.selectFilter"]')), close },
  ];
}

function coordinateScenes() {
  const PARCEL = [[0, 0], [42.5, -3.25], [47, 24], [18, 31.5], [-2, 22]];
  const NEIGHBOUR = [[42.5, -3.25], [71, -6], [74.5, 19], [47, 24]];
  // The parcels in metres from a point left of the view's middle, the view about them (as the desktop's scene fits it).
  const DRAWN = SCRATCH(`
    k.doc.settings.assign({ plotScale: 500 });
    const o = { x: c.x - 30, y: c.y - 12 };
    const P = ([x, y]) => ({ x: o.x + x, y: o.y + y });
    const pa = add({ kind: 'polygon', pts: ${JSON.stringify(PARCEL)}.map(P), label: '7', attrs: { Ada: '1043', Parsel: '7' } });
    const pb = add({ kind: 'polygon', pts: ${JSON.stringify(NEIGHBOUR)}.map(P), label: '8', attrs: { Ada: '1043', Parsel: '8' } });
    const pts = ${JSON.stringify(PARCEL)}.map((q, i) => add({ kind: 'point', p: P(q), z: 812.4 + i * 0.35, label: String(101 + i) }));
    window.__coords = { o, ids: [pa.id, pb.id, ...pts.map((e) => e.id)] };
    k.view.camera.fit({ minX: o.x - 14, minY: o.y - 14, maxX: o.x + 82, maxY: o.y + 46 }, 24);
    k.view.requestRender();`);
  const AT = (x, y) => `(() => { const o = window.__coords.o; return [o.x + ${x}, o.y + ${y}]; })()`;
  const ground = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad' });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  const close = async (ui) => (await ui.escapeAll(3), await ui.eval(UNDO_ALL), await ribbonOff(ui));
  const hover = async (ui, x, y) => hoverAt(ui, ...(await ui.eval(AT(x, y))));
  const click = async (ui, x, y) => (await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AT(x, y))))))), await ui.sleep(300));
  const selectGround = (ui) => ui.eval(`window.kentos.selection.set(window.__coords.ids)`);
  const input = (ui, key) => ui.eval(`window.kentos.tools.active.input('${key}')`);
  /** Şablon typed into its field. */
  const template = async (ui, text) => {
    await input(ui, 'Ş');
    await ui.sleep(300);
    await ui.type(text);
    await ui.key('Enter');
    await ui.sleep(300);
  };
  /** Köşelere koordinat yaz with the template {ad}, no leader and Çizelge on: Enter writes the numbers, the schedule hangs. */
  const numbered = async (ui) => {
    await selectGround(ui);
    await startTool(ui, 'coordinateVertices');
    await template(ui, '{ad}');
    await input(ui, 'K');
    await input(ui, 'Ç');
    await pressEnter(ui);
  };
  /** The options back as they began (the session keeps them). */
  const reset = async (ui) => {
    await ui.escapeAll(3);
    await selectGround(ui);
    await startTool(ui, 'coordinateVertices');
    await input(ui, 'Ş');
    await ui.sleep(300);
    await ui.key('Backspace');
    await ui.key('Enter');
    await ui.sleep(200);
    const prompt = await ui.eval(`window.kentos.tools.active.prompt.value`);
    if (prompt.includes('Kollu (K): kapalı')) await input(ui, 'K');
    if (prompt.includes('Çizelge (Ç): açık')) await input(ui, 'Ç');
    await ui.escapeAll(3);
  };
  return [
    {
      id: 'coordinate-label',
      open: async (ui) => {
        await ground(ui);
        await startTool(ui, 'coordinateLabel');
        await click(ui, ...PARCEL[2]);
        await hover(ui, ...PARCEL[3]);
      },
      close,
    },
    {
      id: 'coordinate-vertices',
      open: async (ui) => {
        await ground(ui);
        await selectGround(ui);
        await startTool(ui, 'coordinateVertices');
        await hover(ui, 62, 36);
      },
      close,
    },
    {
      id: 'coordinate-schedule',
      open: async (ui) => {
        await ground(ui);
        await numbered(ui);
        await hover(ui, 56, 44);
      },
      close: async (ui) => (await reset(ui), await close(ui)),
    },
    {
      id: 'coordinate-drawn',
      open: async (ui) => {
        await ground(ui);
        await selectGround(ui);
        await startTool(ui, 'coordinateVertices');
        await pressEnter(ui);
        await numbered(ui);
        await click(ui, 56, 44);
        await ui.eval(`window.kentos.selection.clear()`);
        await hover(ui, -10, 40);
      },
      close: async (ui) => (await reset(ui), await close(ui)),
    },
  ];
}

function paragraphScenes() {
  const DRAWN = FIT(`
    const P = (fx, fy) => ({ x: c.x + fx * u, y: c.y + fy * u });
    const h = 0.16 * u;
    add({ kind: 'polygon', pts: [P(-2.6, -1.3), P(-0.2, -1.3), P(-0.2, 0.2), P(-2.6, 0.2)] });
    add({ kind: 'line', a: P(-2.8, -0.4), b: P(2.8, -0.4) });
    const text = 'Parsel 101 — imar planına göre konut alanı\\nAlan: 450 m2 (tapuda)\\nH2O hattı altı çizili, kırmızı not';
    const runs = [
      { start: 0, end: 10, bold: true },
      { start: 11, end: 42, italic: true },
      { start: 54, end: 55, script: 'super' },
      { start: 66, end: 67, script: 'sub' },
      { start: 69, end: 86, underline: true },
      { start: 88, end: 95, color: '#E5484D' },
    ];
    const ta = add({ kind: 'text', p: P(-2.5, 1.2), text, height: h, rotation: 0, align: 'topLeft', boxWidth: 2.4 * u, lineSpacing: 1.2, runs, mask: true });
    const tb = add({ kind: 'text', p: P(1.4, 0.7), text: 'Ortalı ve dönük\\nçok satırlı not', height: h, rotation: 15, align: 'middleCenter', runs: [{ start: 0, end: 6, bold: true, color: '#4F8EF7' }] });
    const td = add({ kind: 'text', p: P(2.6, -1.4), text: 'Alt\\nsağa\\ndayalı', height: h * 0.8, rotation: 0, align: 'bottomRight', lineSpacing: 0.8 });
    window.__paragraphs = { a: ta.id, b: tb.id, d: td.id };`);
  const ground = async (ui, fields = {}) => {
    await ribbonOn(ui, { ribbonTab: 'annotate', type: 'cad', ...fields });
    await ui.eval(CLEAR_VIEW);
    await ui.sleep(300);
    await ui.eval(DRAWN);
    await ui.eval(`window.kentos.log.clear()`);
    await ui.move(2, 2);
    await ui.sleep(400);
  };
  const close = async (ui) => (await ui.escapeAll(3), await ui.eval(UNDO_ALL));
  /** The tool's first corner set, the pointer at the opposite one. */
  const box = async (ui) => {
    await ground(ui);
    await startTool(ui, 'mtext');
    await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AU(0.2, 1.4)))))));
    await hoverU(ui, 2.6, 0.9);
  };
  /** The editor over the box, a paragraph typed and formatted (as the traces do it). */
  const editor = async (ui) => {
    await box(ui);
    await ui.clickAt(...(await ui.eval(PAGE_AT(...(await ui.eval(AU(2.6, 0.9)))))));
    await ui.sleep(300);
    await ui.eval(`(() => {
      const ed = document.querySelector('.paragraph-editor');
      const area = ed.querySelector('textarea');
      area.value = 'Yeni not\\nKot farkı 2.5 m, eğim %3\\nAda 7 parsel 12';
      area.dispatchEvent(new Event('input'));
      const pick = (s, e, sel) => { area.setSelectionRange(s, e); ed.querySelector(sel).click(); };
      pick(0, 8, '[data-format="bold"]');
      pick(9, 18, '[data-format="italic"]');
      pick(34, 49, '[data-format="underline"]');
      area.setSelectionRange(31, 33);
    })()`);
    await ui.sleep(400);
  };
  return [
    { id: 'paragraph-drawn', open: ground, close },
    { id: 'paragraph-drawn-selected', open: async (ui) => (await ground(ui), await ui.eval(`(() => { const k = window.kentos; const o = window.__paragraphs; k.selection.set([o.a, o.b]); })()`), await ui.move(2, 2), await ui.sleep(400)), close },
    { id: 'paragraph-tool-box', open: box, close },
    { id: 'paragraph-editor', open: editor, close },
    { id: 'paragraph-editor-color', open: async (ui) => (await editor(ui), await ui.clickSel('.paragraph-editor [data-menu="color"]'), await ui.sleep(300)), close },
    { id: 'paragraph-editor-symbol', open: async (ui) => (await editor(ui), await ui.clickSel('.paragraph-editor [data-menu="symbol"]'), await ui.sleep(300)), close },
    {
      id: 'paragraph-props',
      open: async (ui) => (await ground(ui, { layersFraction: 0.15 }), await ui.eval(`(() => { const k = window.kentos; k.selection.set([window.__paragraphs.a]); })()`), await ui.move(2, 2), await ui.sleep(500)),
      close,
    },
    {
      id: 'paragraph-dxf',
      open: async (ui) => {
        const bytes = readFileSync(new URL('../../../../fixtures/formats/v1/mtext.dxf', import.meta.url)).toString('base64');
        await ui.eval(`import('/src/ui/io/DrawingImportDialog.ts').then((m) => m.openDxfImport(window.kentos, { name: 'mtext.dxf', bytes: Uint8Array.from(atob('${bytes}'), (c) => c.charCodeAt(0)) }, { description: 'DXF', accept: { 'application/dxf': ['.dxf'] } }))`);
        await ui.waitFor(DXF_READ, 15000);
        await ui.clickText('.dialog--io .btn--primary', 'İçe aktar');
        await ui.waitFor(`!document.querySelector('.dialog--io')`, 8000);
        await ui.eval(`(() => { const k = window.kentos; k.view.camera.fit({ minX: -4, minY: -30, maxX: 100, maxY: 8 }, 24); k.view.requestRender(); })()`);
        await ui.move(2, 2);
        await ui.sleep(600);
      },
      close,
    },
  ];
}

/** The n-th rule's condition (0 is the first), typed and left. */
const setRuleFilter = (n, text) =>
  `(() => { const i = document.querySelectorAll('.dialog--lstyle .rule input[aria-label="Koşul"]')[${n}]; i.value = ${JSON.stringify(text)}; i.dispatchEvent(new Event('change')); })()`;

if (!group || !SCENES[group]) {
  console.error(`Grup verin: ${Object.keys(SCENES).join(', ')}`);
  process.exit(2);
}
const DIR = join(OUT, 'shots', group);
mkdirSync(DIR, { recursive: true });

const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
const written = [];
const failed = [];

for (const [w, hgt] of sizes) {
  for (const theme of themes) {
    const b = await launch('about:blank', { width: w, height: hgt, ...(renderer === 'webgpu' ? { args: WEBGPU_ARGS } : {}) });
    const ui = helpers(b);
    try {
      await b.send('Page.navigate', { url: `${url}?renderer=${renderer}&start=0` });
      const ready = 'window.kentos && window.kentos.view.backendKind.value';
      await b.waitFor(ready, 30000);
      await sleep(1200);
      await b.waitFor(ready, 20000);
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      await b.eval('document.fonts.ready');
      await sleep(300);
      for (const scene of SCENES[group]) {
        if (only && !only.includes(scene.id)) continue;
        const name = `${scene.id}-${theme}-${w}${renderer === 'webgl2' ? '' : `-${renderer}`}`;
        try {
          await scene.open(ui);
          await sleep(250);
          written.push(await b.shot(name, undefined, DIR));
        } catch (e) {
          failed.push(`${name}: ${String(e.message ?? e).slice(0, 200)}`);
        }
        await (scene.close ?? ((u) => u.escapeAll(3)))(ui);
        await b.eval(TYPE_BACK);
        await b.eval(`(() => { const k = window.kentos; k.selection.clear(); k.tools.activate('select'); })()`);
      }
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
  // Scrolled into view first, as a user would: a button below a pane's fold is clicked where it shows.
  const centre = (expr) =>
    b.eval(`(() => { const el = ${expr}; if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  const bySel = (sel) => `document.querySelector(${JSON.stringify(sel)})`;
  const byText = (sel, text) => `[...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)}))`;
  const ui = {
    eval: (expr) => b.eval(expr),
    sleep,
    type: (text) => b.type(text),
    waitFor: (expr, ms = 8000) => b.waitFor(expr, ms),
    clickSel: async (sel) => {
      const at = await centre(bySel(sel));
      if (!at) throw new Error(`yok: ${sel}`);
      await b.click(...at);
      await sleep(250);
    },
    clickText: async (sel, text) => {
      const at = await centre(byText(sel, text));
      if (!at) throw new Error(`yok: ${sel} “${text}”`);
      await b.click(...at);
      await sleep(250);
    },
    hoverText: async (sel, text) => {
      await b.waitFor(byText(sel, text), 6000).catch(() => {});
      const at = await centre(byText(sel, text));
      if (!at) throw new Error(`yok: ${sel} “${text}”`);
      await b.move(...at);
      await sleep(500);
    },
    key: (k, mods) => b.key(k, mods),
    drag: (...at) => b.drag(...at),
    clickAt: (x, y, opts) => b.click(x, y, opts),
    move: (x, y) => b.move(x, y),
    /** The left button held down, moved and let go: a drag the picture is taken in the middle of. */
    pressAt: (x, y) => b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 }),
    moveHeld: (x, y) => b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'left', buttons: 1 }),
    releaseAt: (x, y) => b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 }),
    /** A right click (a context menu). */
    contextClick: async (x, y) => {
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
      await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'right', buttons: 2, clickCount: 1 });
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'right', buttons: 0, clickCount: 1 });
      await sleep(250);
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
