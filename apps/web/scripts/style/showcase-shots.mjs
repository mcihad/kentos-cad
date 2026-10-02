// Pictures of the web's demo drawing at the views the desktop's styled drawing is compared at
// (docs/adr/0090; apps/desktop/src/style/screens.rs): the showcase's section layers at 1:1000 on screen,
// each section's top left a little inside the view, in the ribbon layout as on the desktop, dark and light,
// 1440×900 and 1100×650. For a person to set side by side with the desktop's; nothing is checked.
//
//   node apps/web/scripts/style/showcase-shots.mjs [--out DIR]
//
// Pictures: DIR (default .run/shots at the repository root) as web-stil-<view>-<w>x<h>[-acik].png.
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const out = args.includes('--out') ? args[args.indexOf('--out') + 1] : join(new URL('../../../..', import.meta.url).pathname, '.run/shots');
mkdirSync(out, { recursive: true });

/** Views: a name and the showcase layer whose top left is shown (the desktop's screens.rs `VIEWS`). */
const VIEWS = [
  ['temel', 'vitrin.temel-cizgi-tipleri'],
  ['uip-sinirlar', 'vitrin.mpyy-uygulama-imar-plani-sinirlar-planlama-sinirlari'],
  ['uip-konut', 'vitrin.mpyy-uygulama-imar-plani-konut-alanlari'],
  ['uip-yapi-duzeni', 'vitrin.mpyy-uygulama-imar-plani-yapi-duzeni-ve-yogunluklari'],
  ['uip-sosyal', 'vitrin.mpyy-uygulama-imar-plani-sosyal-altyapi-alanlari-egitim-tesisleri-alani'],
  ['uip-karayollari', 'vitrin.mpyy-uygulama-imar-plani-teknik-altyapi-ulasim-karayollari'],
];

/** Puts a layer's top left in view at 1:1000 on a 96 dpi screen (as screens.rs does). */
const SHOW = (layer) => `(() => {
  const k = window.kentos;
  let b = null;
  const grow = (x, y) => { if (!b) b = { minX: x, minY: y, maxX: x, maxY: y }; b.minX = Math.min(b.minX, x); b.minY = Math.min(b.minY, y); b.maxX = Math.max(b.maxX, x); b.maxY = Math.max(b.maxY, y); };
  for (const e of k.doc.byLayer(${JSON.stringify(layer)})) {
    if (e.kind === 'polygon' || e.kind === 'polyline') e.pts.forEach((p) => grow(p.x, p.y));
    else if (e.kind === 'point' || e.kind === 'text') grow(e.p.x, e.p.y);
  }
  if (!b) return false;
  const cam = k.view.camera;
  cam.scale = 1 / (0.00026458 * 1000);
  const w = cam.width / cam.scale, h = cam.height / cam.scale;
  cam.center = { x: b.minX - 4 + w / 2, y: b.maxY + 16 - h / 2 };
  cam.changed.update((v) => v + 1);
  k.view.requestRender();
  return true;
})()`;

// The web app's own folder, wherever the script is started from.
const root = new URL('../..', import.meta.url).pathname;
const server = await createServer({ root, configFile: join(root, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
try {
  for (const [w, h] of [
    [1440, 900],
    [1100, 650],
  ]) {
    for (const theme of ['dark', 'light']) {
      const b = await launch('about:blank', { width: w, height: h });
      try {
        await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
        const ready = 'window.kentos && window.kentos.view.backendKind.value';
        await b.waitFor(ready, 30000);
        await sleep(1200);
        await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
        await b.waitFor(`document.querySelector('.ribbon__strip .rpanel')`, 10000).catch(() => {});
        await b.eval('document.fonts.ready');
        await sleep(600);
        for (const [name, layer] of VIEWS) {
          const shown = await b.eval(SHOW(layer));
          if (!shown) {
            console.log(`✗ ${name}: katmanda nesne yok (${layer})`);
            continue;
          }
          // Images of the atlas arrive a frame or two later.
          await sleep(900);
          const file = `web-stil-${name}-${w}x${h}${theme === 'light' ? '-acik' : ''}`;
          await b.shot(file, undefined, out);
          console.log(join(out, `${file}.png`));
        }
      } finally {
        b.close();
      }
    }
  }
} finally {
  await server.close();
}
