// The PNG the web's Lejant saves, for a person to set beside the desktop's (docs/adr/0093;
// apps/desktop/src/style/screens.rs `legend_screens`, .run/shots/lejant-resim*.png): the demo
// drawing's own layers (the showcase's are left out, as on the desktop), with and without the
// layer headings, drawn in a browser at a device pixel ratio of 1, where the symbols used to be
// stretched. Nothing is checked.
//
//   node apps/web/scripts/style/legend-png.mjs [--out DIR]
//
// Pictures: DIR (default .run/shots at the repository root) as web-lejant-resim[-basliksiz].png.
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const out = args.includes('--out') ? args[args.indexOf('--out') + 1] : join(new URL('../../../..', import.meta.url).pathname, '.run/shots');
mkdirSync(out, { recursive: true });

/** The picture as a data address: the legend of the visible layers but the showcase's. */
const PICTURE = (headings) => `(async () => {
  const k = window.kentos;
  const L = k.doc.layers;
  const { legendLayers, legendLayout, legendOf } = await import('/src/style/legend.ts');
  const { legendPicture } = await import('/src/ui/style/LegendDialog.ts');
  const layers = legendLayers(L.leaves().map((n) => ({ id: n.id, name: n.name, style: n.style, visible: L.isVisible(n.id) })), true)
    .filter((l) => !l.id.startsWith('vitrin'));
  const groups = legendOf(layers, { entities: (id) => k.doc.byLayer(id), symbol: (r) => k.styles.library.symbol(r), itemName: (id) => k.styles.library.get(id)?.name });
  const layout = legendLayout(groups, ${headings}, k.doc.name.value);
  return legendPicture(k, groups, layout).toDataURL('image/png');
})()`;

// The web app's own folder, wherever the script is started from.
const root = new URL('../..', import.meta.url).pathname;
const server = await createServer({ root, configFile: join(root, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
try {
  const b = await launch('about:blank', { width: 1440, height: 900 });
  try {
    await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
    await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
    await sleep(1200);
    await b.eval('document.fonts.ready');
    for (const [headings, name] of [
      [true, 'web-lejant-resim'],
      [false, 'web-lejant-resim-basliksiz'],
    ]) {
      const data = await b.eval(PICTURE(headings));
      const file = join(out, `${name}.png`);
      writeFileSync(file, Buffer.from(String(data).split(',')[1], 'base64'));
      console.log(file);
    }
  } finally {
    b.close();
  }
} finally {
  await server.close();
}
