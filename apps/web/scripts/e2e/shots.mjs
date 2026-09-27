// Reference pictures of the web for porting (not a test): named scenes, each at 1440×900 and 1100×650 in the
// dark and the light theme, into scripts/e2e/out/shots/<group>/<scene>-<theme>-<width>.png, on the demo drawing.
// A scene opens what it shows through the app's own commands and the dev-only window.kentos handle, and leaves
// the app as it found it (closing windows, undoing runs).
//
//   node scripts/e2e/shots.mjs <group> [--only a,b] [--sizes 1440x900,1100x650] [--themes dark,light]
//
// Groups: processing (İşlemler: the dock, the menu, the tool and model dialogs and their states).
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
};

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
  const centre = (expr) => b.eval(`(() => { const el = ${expr}; if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
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
