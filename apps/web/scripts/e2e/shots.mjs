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
// the new drawing and editing tools, their split buttons, each tool at work).
import { mkdirSync } from 'node:fs';
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
].map((s) => ({ close: (ui) => ribbonOff(ui), ...s }));

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
