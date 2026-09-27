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
// form, the add menus, a child marker, ƒ on, the preview geometry, the unsaved question, inline and library symbols).
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
