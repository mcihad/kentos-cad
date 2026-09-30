// Reference pictures of the web for porting (not a test): named scenes, each at 1440×900 and 1100×650 in the
// dark and the light theme, into scripts/e2e/out/shots/<group>/<scene>-<theme>-<width>.png, on the demo drawing.
// A scene opens what it shows through the app's own commands and the dev-only window.kentos handle, and leaves
// the app as it found it (closing windows, undoing runs).
//
//   node scripts/e2e/shots.mjs <group> [--only a,b] [--sizes 1440x900,1100x650] [--themes dark,light]
//
// Groups: processing (İşlemler: the dock, the menu, the tool and model dialogs and their states); layerstyle (Katman
// stili: each renderer, its classes, the symbol slot, errors, applied); stylemanager (Stil yöneticisi: the tree, a
// search, the kinds, system and own items, the menus, a delete question, applying to a selection, pick mode); legend
// (Lejant: its options, a layer left out, a categorized layer); symboldesigner (Sembol tasarımcısı: every layer type's
// form, the add menus, a child marker, ƒ on, the preview geometry, the unsaved question, inline and library symbols);
// svgedit (SVG düzenleyicisi: a new and a library drawing, shapes, the tabs, the menus, a polyline in progress, text,
// node editing, measuring, the XML source, document properties, export, the unsaved question);
// shell (the classic shell: the bars, and the toolbox docked, in two columns, folded, widened, its tip, a snapping drag);
// log (the bottom panel's lines with their times and levels, Uyarılar with its badge, a warning in the status bar, the
// empty history); layout (panels, sizes and toolbox as kept, sizes kept larger than the window shown within it, the
// ribbon's kept tab, quick access and split choices, folded); modeldesigner (Model tasarımcısı: a new model, the
// built-in model's copy, an input, a step and a source list, a wire dragged and its menu, a step with problems, a chain,
// a number input, the tools searched and one carried, the unsaved question); ribbon (the key tips on the tabs, on
// Giriş and narrowed, the quick access bar's menu, right clicks on a command, an added one, a fixed one and a tab, a
// tool's methods and a family under their split buttons, the folded ribbon open); tools (docs/adr/0140: the tabs of
// the new drawing and editing tools, their split buttons, each tool at work); blocks (docs/adr/0144: DXF içe aktar
// over a file with blocks read a second time, Blokları patlat clicked, the imported blocks in the Bloklar panel).
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const args = process.argv.slice(2);
const group = args.find((a) => !a.startsWith('--') && !args[args.indexOf(a) - 1]?.startsWith('--'));
const opt = (name) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1].split(',') : null);
const only = opt('only');
const sizes = (opt('sizes') ?? ['1440x900', '1100x650']).map((s) => s.split('x').map(Number));
const themes = opt('themes') ?? ['dark', 'light'];

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
    { id: 'menu-models', open: async (ui) => (await ui.clickSel('.menubar__item[data-menu="processing"]'), await ui.hoverText('.menu .menu__item', 'Modeller')) },
    { id: 'menu-category', open: async (ui) => (await ui.clickSel('.menubar__item[data-menu="processing"]'), await ui.hoverText('.menu .menu__item', 'Nokta işlemleri')) },
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

// The classic shell (ui/shell/shellPlan.ts, fixtures/shell/v1/shell.json): the bars at rest, and the toolbox docked, in
// two columns, with a folded group, widened to fit the height (1100×650), a tool's tip with its steps, and snapping to
// an edge in the middle of a drag.
const TOOLBOX_RESET = `(() => { const u = window.kentos.ui; u.toolboxDocked.set(false); u.toolboxColumns.set(3); u.toolboxFolded.set([]); u.toolboxX.set(12); u.toolboxY.set(12); u.toolboxVisible.set(true); })()`;
const centreOf = (sel) => `(() => { const r = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`;
SCENES.shell = [
  { id: 'classic', open: async (ui) => (await ui.eval(TOOLBOX_RESET), await ui.sleep(400)) },
  { id: 'toolbox-docked', open: async (ui) => (await ui.eval(`window.kentos.commands.execute('view.toolboxDock')`), await ui.sleep(400)) },
  { id: 'toolbox-two-columns', open: async (ui) => (await ui.eval(`window.kentos.ui.toolboxColumns.set(2)`), await ui.sleep(400)) },
  { id: 'toolbox-folded-group', open: async (ui) => (await ui.eval(`window.kentos.ui.toolboxFolded.set(['draw', 'annotate'])`), await ui.sleep(400)) },
  {
    id: 'toolbox-tip',
    open: async (ui) => {
      await ui.move(...(await ui.eval(centreOf('.toolbox__tool[data-tool="line"]'))));
      await ui.waitFor(`!!document.querySelector('.tooltip[data-open]')`);
      await ui.sleep(300);
    },
  },
  {
    id: 'toolbox-snap-drag',
    open: async (ui) => {
      // Held by the grip and dragged towards the drawing's right edge: within 14 px it sits on the margin.
      const [x, y] = await ui.eval(centreOf('.toolbox__grip'));
      const right = await ui.eval(`(() => { const r = window.kentos.view.clientRect(); const t = document.querySelector('.toolbox').getBoundingClientRect(); return Math.round(r.right - t.width / 2 - 12); })()`);
      await ui.move(x, y);
      await ui.pressAt(x, y);
      for (let i = 1; i <= 8; i++) await ui.moveHeld(x + ((right - x) * i) / 8, y + 60 * (i / 8));
      await ui.sleep(300);
    },
    close: async (ui) => (await ui.releaseAt(2, 2), await ui.eval(TOOLBOX_RESET)),
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(1), await ui.eval(TOOLBOX_RESET)), ...s }));

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

// The kept layout (app/layoutPlan.ts, fixtures/shell/v1/layout.json): panels, sizes and the toolbox as kept; a dock and
// a bottom panel kept larger than this window allows, shown within it; the ribbon with its kept tab, quick access
// commands and split choices, and folded. Each scene sets the live layout and puts it back after.
const LAYOUT_KEEP = `window.__shotLayout = Object.fromEntries(Object.entries(window.kentos.ui).map(([k, s]) => [k, s.value]))`;
const LAYOUT_BACK = `(() => { const ui = window.kentos.ui; for (const [k, v] of Object.entries(window.__shotLayout ?? {})) ui[k].set(v); })()`;
/** Layout fields set on the live layout (the kept ones first saved aside). */
const layoutSet = (fields) => `(() => { ${LAYOUT_KEEP}; const ui = window.kentos.ui; for (const [k, v] of Object.entries(${JSON.stringify(fields)})) ui[k].set(v); })()`;
/** The shell as Uygulama ayarları sets it; the ribbon loads on first use. */
async function shellTo(ui, kind) {
  await ui.eval(`window.kentos.prefs.shell.set(${JSON.stringify(kind)})`);
  await ui.waitFor(kind === 'ribbon' ? `!!document.querySelector('.ribbon__strip .rpanel')` : `!!document.querySelector('.menubar')`, 10000);
  await ui.sleep(500);
}
const RIBBON_KEPT = { ribbonTab: 'draw', ribbonQuickAccess: ['view.zoomExtents', 'tool.line'], ribbonSplits: { circle: 'tool.circle|3N', rectangle: 'tool.regularPolygon|' } };
SCENES.layout = [
  {
    id: 'kept',
    open: async (ui) => {
      await ui.eval(layoutSet({ dockWidth: 400, layersFraction: 0.35, bottomExpanded: true, bottomTab: 'coords', toolboxDocked: true, toolboxColumns: 2, toolboxFolded: ['annotate'] }));
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
      await ui.eval(layoutSet(RIBBON_KEPT));
      await shellTo(ui, 'ribbon');
    },
    close: async (ui) => (await shellTo(ui, 'classic'), await ui.eval(LAYOUT_BACK)),
  },
  {
    id: 'ribbon-collapsed',
    open: async (ui) => {
      await ui.eval(layoutSet({ ...RIBBON_KEPT, ribbonCollapsed: true }));
      await shellTo(ui, 'ribbon');
    },
    close: async (ui) => (await shellTo(ui, 'classic'), await ui.eval(LAYOUT_BACK)),
  },
].map((s) => ({ close: async (ui) => (await ui.escapeAll(1), await ui.eval(LAYOUT_BACK), await ui.sleep(300)), ...s }));

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
// the classic shell and the layout back after.
const RIBBON_BAR = { ribbonQuickAccess: ['view.zoomExtents', 'tool.line'], ribbonTab: 'home', ribbonCollapsed: false };
async function ribbonOn(ui, fields = {}) {
  await ui.eval(layoutSet({ ...RIBBON_BAR, ...fields }));
  await shellTo(ui, 'ribbon');
}
async function ribbonOff(ui) {
  await ui.escapeAll(3);
  await shellTo(ui, 'classic');
  await ui.eval(LAYOUT_BACK);
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
  { id: 'menu-elsewhere', open: async (ui) => (await ribbonOn(ui), await rightClick(ui, '.ribbon__tab[data-tab="draw"]')) },
  {
    id: 'split-methods',
    open: async (ui) => (await ribbonOn(ui), await ui.clickSel('.ribbon__strip [data-split="circle"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
  },
  {
    id: 'split-family',
    open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'draw' }), await ui.clickSel('.ribbon__strip [data-split="rectangle"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
  },
  {
    id: 'folded-open',
    open: async (ui) => (await ribbonOn(ui, { ribbonCollapsed: true }), await ui.clickSel('.ribbon__tab[data-tab="home"]'), await ui.sleep(500)),
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
  const ribbon = (ui) => ribbonOn(ui, { ribbonTab: 'modify' });
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
    await ribbonOn(ui, { ribbonTab: 'modify', ...fields });
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
    ...[
      ['split-trim-methods', 'modify', 'trim'],
      ['split-array', 'modify', 'array'],
      ['split-dimension', 'draw', 'dimension'],
      ['split-points-between', 'draw', 'pointsBetween'],
      ['split-intersect-point', 'draw', 'intersectPoint'],
    ].map(([id, tab, key]) => ({
      id,
      open: async (ui) => (
        await ribbonOn(ui, { ribbonTab: tab }),
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
    await bare(ui, OBJECTS, { ribbonTab: 'modify', ...LOGGED });
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
    await bare(ui, OBJECTS, { ribbonTab: 'modify', layersFraction: 0.15 });
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
    await bare(ui, OBJECTS, { ribbonTab: 'modify' });
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
        await bare(ui, OBJECTS, { ribbonTab: 'modify', ...LOGGED });
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
    ['ribbon-draw', 'draw'],
    ['ribbon-modify', 'modify'],
    ['ribbon-map', 'map'],
  ].map(([id, tab]) => ({ id, open: async (ui) => (await ribbonOn(ui, { ribbonTab: tab }), await ui.sleep(400)) })),
  {
    id: 'split-corner',
    open: async (ui) => (await ribbonOn(ui, { ribbonTab: 'modify' }), await ui.clickSel('.ribbon__strip [data-split="corner"] .rsplit__arrow'), await ui.waitFor(`!!document.querySelector('.menu')`), await ui.sleep(300)),
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
    // Clear of the floating toolbox on the left.
    const c = k.view.camera;
    c.scale = c.scale * 0.85;
    c.center = { x: c.center.x - 170 / c.scale, y: c.center.y };
    c.panBy(0, 0);
  })()`);
  await ui.sleep(600);
};
/** Closer in: the view centred on `x`, `y` at `times` the whole scene's scale. */
const closeIn = (x, y, times) => `(() => { const c = window.kentos.view.camera; c.center = { x: ${x}, y: ${y} }; c.scale = c.scale * ${times}; c.panBy(0, 0); window.kentos.view.requestRender(); })()`;
SCENES.texts = [
  { id: 'text-extras', open: openTextExtras },
  { id: 'text-extras-mask', open: async (ui) => (await openTextExtras(ui), await ui.eval(closeIn(487108, 4419985, 2.5 / 0.85)), await ui.sleep(400)) },
  { id: 'text-extras-turned', open: async (ui) => (await openTextExtras(ui), await ui.eval(closeIn(487118, 4420025, 5 / 0.85)), await ui.sleep(400)) },
  ...textToolScenes(),
];

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
    await ribbonOn(ui, { ribbonTab: 'draw', ...fields });
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
  const upside = async (ui) => {
    await ribbonOn(ui, { ribbonTab: 'modify', ...LOGGED });
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
    { id: 'readable-selected', open: upside, close },
    { id: 'readable-done', open: async (ui) => (await upside(ui), await startTool(ui, 'readable'), await ui.move(2, 2)), close },
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
    const b = await launch('about:blank', { width: w, height: hgt });
    const ui = helpers(b);
    try {
      await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
      const ready = 'window.kentos && window.kentos.view.backendKind.value';
      await b.waitFor(ready, 30000);
      await sleep(1200);
      await b.waitFor(ready, 20000);
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      await b.eval('document.fonts.ready');
      await sleep(300);
      for (const scene of SCENES[group]) {
        if (only && !only.includes(scene.id)) continue;
        const name = `${scene.id}-${theme}-${w}`;
        try {
          await scene.open(ui);
          await sleep(250);
          written.push(await b.shot(name, undefined, DIR));
        } catch (e) {
          failed.push(`${name}: ${String(e.message ?? e).slice(0, 200)}`);
        }
        await (scene.close ?? ((u) => u.escapeAll(3)))(ui);
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
