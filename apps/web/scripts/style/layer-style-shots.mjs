// Pictures of the web's Katman stili window in the states the desktop's are taken in
// (docs/adr/0091; apps/desktop/src/style/screens.rs `layer_style_screens`): Basit on Ada sınırı,
// Kategorili on Parsel sınırı by Nitelik, Aralıklı on Yapı by the storeys, Kurallar on Parsel
// sınırı, Tek sembol on Kot noktaları; dark and light, 1440×900 and 1100×650. For a person to set
// side by side with the desktop's; nothing is checked.
//
//   node apps/web/scripts/style/layer-style-shots.mjs [--out DIR]
//
// Pictures: DIR (default .run/shots at the repository root) as web-lstil-<state>-<w>x<h>[-acik].png.
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, sleep } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const out = args.includes('--out') ? args[args.indexOf('--out') + 1] : join(new URL('../../../..', import.meta.url).pathname, '.run/shots');
mkdirSync(out, { recursive: true });

// Driving the window through its controls, as a person would (each change draws it again).
const D = `(() => {
  const q = (sel) => document.querySelector('.dialog--lstyle ' + sel);
  const all = (sel) => [...document.querySelectorAll('.dialog--lstyle ' + sel)];
  const button = (text) => all('button').find((b) => b.textContent.trim() === text);
  const kind = (text) => all('.seg__opt').find((b) => b.textContent.trim() === text).click();
  const type = (el, value) => { el.value = value; el.dispatchEvent(new Event('change')); };
  return { q, all, button, kind, type };
})()`;

/** A state: its name, the layer, and the script that puts the window in it. */
const STATES = [
  ['basit', 'ada', ''],
  [
    'kategorili',
    'parsel',
    `d.kind('Kategorili');
     d.type(d.q('.lsty__expr'), 'Nitelik');
     d.button('Değerlerden sınıfla').click();
     d.q('input[aria-label="Diğer değerler çizilsin"]').click();
     d.button('Uygula').click();`,
  ],
  [
    'aralikli',
    'yapi',
    `d.kind('Aralıklı');
     d.type(d.q('.lsty__expr'), '[Kat adedi]');
     d.type(d.q('input[aria-label="Sınıf sayısı"]'), '4');
     d.type(d.q('select[aria-label="Renk rampası"]'), 'maviler');
     d.button('Sınıfla').click();
     d.button('Uygula').click();`,
  ],
  [
    'kurallar',
    'parsel',
    `d.kind('Kurallar');
     d.type(d.all('input[aria-label="Kural adı"]')[0], 'Arsalar');
     d.type(d.all('input[aria-label="Koşul"]')[0], "Nitelik = 'Arsa'");
     d.all('button[aria-label="Alt kural ekle"]')[0].click();
     d.type(d.all('input[aria-label="Kural adı"]')[1], 'Büyük arsalar');
     d.type(d.all('input[aria-label="Koşul"]')[1], '$alan > 400');
     d.type(d.all('input[aria-label="En uzak ölçek"]')[1], '5.000');
     d.button('Değilse kuralı ekle').click();
     d.button('Kural ekle').click();
     d.type(d.all('input[aria-label="Kural adı"]')[3], 'Hatalı koşul');
     d.type(d.all('input[aria-label="Koşul"]')[3], '$alan >');
     d.button('Uygula').click();`,
  ],
  ['tek', 'kot', `d.kind('Tek sembol'); d.button('Uygula').click();`],
];

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
        await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
        await sleep(1200);
        await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
        await b.eval('document.fonts.ready');
        await sleep(400);
        for (const [name, layer, script] of STATES) {
          await b.eval(`window.kentos.doc.layers.setActive(${JSON.stringify(layer)}); window.kentos.commands.execute('style.layerStyle')`);
          await b.waitFor(`document.querySelector('.dialog--lstyle')`, 10000);
          await sleep(300);
          await b.eval(`(() => { const d = ${D}; ${script} })()`);
          // The pictures of the slots are drawn a moment later.
          await sleep(700);
          const file = `web-lstil-${name}-${w}x${h}${theme === 'light' ? '-acik' : ''}`;
          await b.shot(file, undefined, out);
          console.log(join(out, `${file}.png`));
          await b.eval(`[...document.querySelectorAll('.dialog--lstyle button')].find((x) => x.textContent.trim() === 'Vazgeç').click()`);
          await sleep(200);
        }
      } finally {
        b.close();
      }
    }
  }
} finally {
  await server.close();
}
