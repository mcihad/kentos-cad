// Layout pass (pnpm e2e:layout): opens every window, menu and panel section of the web at the shell's
// narrowest size (1100×650) and at 1440×900, in both themes, and checks each for what a user would see
// as broken (DESIGN.md §5.1, §7): a window or menu reaching past the screen, a window body or footer
// wider than the window (a horizontal scrollbar), a footer button pushed out, a button or menu row whose
// words are cut, the shell's bars overflowing. A picture of each goes to scripts/e2e/out/layout/ for a
// person to read what a script cannot judge: scrollbars over content, text cut inside fields, balance.
// Exits 1 when a check fails.
//
//   node scripts/e2e/layout.mjs [--only id,id] [--sizes 1100x650,1440x900] [--themes dark,light] [--scale large|xxlarge]
//
// The cloud windows other than the sign-in need the API; pnpm e2e:cloud drives them.
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const args = process.argv.slice(2);
const opt = (name) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1].split(',') : null);
const only = opt('only');
const sizes = (opt('sizes') ?? ['1100x650', '1440x900']).map((s) => s.split('x').map(Number));
const themes = opt('themes') ?? ['dark', 'light'];
/** The type scale (Uygulama ayarları → Görünüm → Yazı boyutu), as the settings window applies it. */
const SCALES = { small: 0.93, standard: 1, large: 1.08, xlarge: 1.16, xxlarge: 1.25 };
const scale = opt('scale')?.[0] ?? 'standard';
const DIR = join(OUT, 'layout');
mkdirSync(DIR, { recursive: true });

const gis = new URL('../../../../fixtures/formats/v1/gis/', import.meta.url);
const formats = new URL('../../../../fixtures/formats/v1/', import.meta.url);
const raw = (url) => readFileSync(url).toString('base64');

/** What each item opens, and how. `b` is the page; `ui` the helpers below. */
const ITEMS = [
  // The shell itself: the bars at this width.
  { id: 'shell', open: async () => {} },
  { id: 'appmenu', open: (ui) => ui.click('.brand') },
  ...['file', 'edit', 'view', 'draw', 'modify', 'map', 'crs', 'calc', 'analysis', 'processing', 'tools', 'help'].map((m) => ({ id: `menu-${m}`, open: (ui) => ui.click(`.menubar__item[data-menu="${m}"]`) })),
  { id: 'toolbar-layer', open: (ui) => ui.click('.toolbar .dropdown--layer') },
  { id: 'toolbar-properties', open: (ui) => ui.clickFirst(['.toolbar [aria-label^="Geçerli özellikler:"]:not([hidden])', '.toolbar [aria-label="Renk"]']) },
  { id: 'toolbar-more', open: (ui) => ui.clickFirst(['.toolbar__more:not([hidden])']), when: (ui) => ui.visible('.toolbar__more') },
  { id: 'status-renderer', open: (ui) => ui.click('.status__renderer') },
  { id: 'status-mode', open: (ui) => ui.click('.status__mode') },
  { id: 'status-account', open: (ui) => ui.click('.status__server') },
  { id: 'layer-row', open: (ui) => ui.rightClick('.panel--layers .tree__row[data-id="taslak"] .tree__name') },
  { id: 'layer-color', open: (ui) => ui.click('.panel--layers .tree__row[data-id="taslak"] .swatch--btn') },
  { id: 'viewport-idle', open: (ui) => ui.viewportRight({}) },
  { id: 'viewport-snap', open: (ui) => ui.viewportRight({ shift: true }) },
  { id: 'viewport-command', open: (ui) => ui.viewportRight({ tool: 'tool.line', hold: true }), close: (ui) => ui.escapeAll(3) },
  { id: 'shortcuts', open: (ui) => ui.run('help.shortcuts') },
  { id: 'about', open: (ui) => ui.run('help.about') },
  ...['appearance', 'snap', 'newProjects', 'engine', 'file'].map((s) => ({ id: `app-settings-${s}`, open: (ui) => ui.run('tools.options', s) })),
  { id: 'project-settings-general', open: (ui) => ui.run('file.settings') },
  { id: 'project-settings-crs', open: (ui) => ui.run('crs.set') },
  { id: 'project-settings-units', open: async (ui) => (await ui.run('file.settings'), await ui.clickText('.settings__navitem', 'Birimler')) },
  { id: 'new-project', open: (ui) => ui.run('file.new') },
  { id: 'start', open: (ui) => ui.run('file.start') },
  { id: 'import-ncn', open: async (ui) => (await ui.pick([['liste.ncn', btoa('1001 487061.123 4420101.456 105.2\r\n1002 487071.5 4420111.25 106.75\r\n')]]), await ui.run('file.import.ncn')), ready: '.dialog--io tbody tr' },
  { id: 'import-dxf', open: async (ui) => (await ui.pick([['entities.dxf', raw(new URL('entities.dxf', formats))]]), await ui.run('file.import.dxf')), ready: '.dialog--io .dialog__foot .btn--primary' },
  { id: 'import-geojson', open: async (ui) => (await ui.pick([['features.geojson', raw(new URL('features.geojson', gis))]]), await ui.run('file.import.geojson')), ready: '.dialog--io tbody tr' },
  { id: 'import-shp', open: async (ui) => (await ui.pick(['karisik.shp', 'karisik.shx', 'karisik.dbf', 'karisik.prj', 'karisik.cpg'].map((n) => [n, raw(new URL(n, gis))])), await ui.run('file.import.shp')), ready: '.dialog--io tbody tr' },
  { id: 'export-dxf', open: (ui) => ui.run('file.export.dxf') },
  { id: 'export-geojson', open: (ui) => ui.run('file.export.geojson') },
  { id: 'export-ncn', open: (ui) => ui.run('file.export.ncn') },
  ...['calc.traverse', 'calc.polar', 'calc.stakeout', 'calc.forward', 'calc.resection'].map((c) => ({ id: c.replace('.', '-'), open: (ui) => ui.run(c) })),
  { id: 'style-manager', open: (ui) => ui.run('style.manager'), ready: '.smgr__grid, .dialog' },
  { id: 'symbol-designer', open: async (ui) => (await ui.run('style.manager'), await ui.clickText('.dialog button', 'Yeni sembol'), await ui.clickText('.menu__item', 'Alan sembolü')) },
  { id: 'layer-style', open: async (ui) => (await ui.eval(`window.kentos.doc.layers.setActive('ada')`), await ui.run('style.layerStyle')) },
  { id: 'legend', open: (ui) => ui.run('style.legend') },
  { id: 'svg-editor', open: (ui) => ui.run('style.svgEditor') },
  { id: 'processing-tool', open: (ui) => ui.run('map.edgeLengths') },
  { id: 'model-designer', open: (ui) => ui.run('processing.newModel') },
  { id: 'cloud-login', open: (ui) => ui.run('cloud.signIn') },
  // Last: it leaves the drawing unsaved.
  { id: 'question-unsaved', open: async (ui) => (await ui.eval(`window.kentos.doc.name.set('Soru')`), await ui.run('file.new'), await ui.clickText('.dialog__foot .btn--primary', 'Oluştur')), ready: '.dialog--confirm' },
];

/** Faults a person would see, read from the page: a list of short Turkish sentences. */
const FAULTS = `(() => {
  const W = innerWidth, H = innerHeight;
  const out = [];
  const seen = (el) => el && el.getClientRects().length > 0 && getComputedStyle(el).visibility !== 'hidden';
  const rect = (el) => el.getBoundingClientRect();
  const off = (r) => r.left < -0.5 || r.top < -0.5 || r.right > W + 0.5 || r.bottom > H + 0.5;
  const cut = (el) => el.scrollWidth > el.clientWidth + 1;
  const words = (el) => el.textContent.trim().replace(/\\s+/g, ' ').slice(0, 40);
  for (const bar of ['.menubar__menus', '.toolbar', '.status', '.ribbon__strip']) {
    const el = document.querySelector(bar);
    if (seen(el) && cut(el)) out.push('çubuk taşıyor: ' + bar);
  }
  for (const card of document.querySelectorAll('.dialog, .appmenu')) {
    if (!seen(card)) continue;
    if (off(rect(card))) out.push('pencere ekrandan taşıyor');
    const body = card.querySelector('.dialog__body');
    if (body && cut(body)) out.push('pencere gövdesi yana kayıyor (' + body.scrollWidth + ' > ' + body.clientWidth + ')');
    const foot = card.querySelector('.dialog__foot');
    if (foot) {
      if (cut(foot)) out.push('alt çubuk taşıyor');
      const fr = rect(foot);
      for (const b of foot.querySelectorAll('button')) if (seen(b) && (rect(b).left < fr.left - 0.5 || rect(b).right > fr.right + 0.5)) out.push('düğme dışarıda: ' + words(b));
    }
    for (const b of card.querySelectorAll('button')) if (seen(b) && words(b) && !b.closest('.dropdown') && cut(b)) out.push('düğme yazısı sığmıyor: ' + words(b));
    // Text shortened with an ellipsis must say its whole self on hover (a title on it or its row).
    for (const el of card.querySelectorAll('*')) {
      if (el.children.length || !seen(el) || !cut(el) || getComputedStyle(el).textOverflow !== 'ellipsis') continue;
      if (!el.closest('[title]') && !el.closest('.dropdown')) out.push('yazı kesik: ' + words(el));
    }
  }
  for (const m of document.querySelectorAll('.menu')) {
    if (!seen(m)) continue;
    if (off(rect(m))) out.push('menü ekrandan taşıyor');
    if (cut(m)) out.push('menü yana kayıyor');
    // A row the menu's width cuts must keep its whole text on hover (PopupMenu sets the title).
    for (const l of m.querySelectorAll('.menu__label, .menu__title')) if (cut(l) && !l.title) out.push('menü satırı kesik: ' + words(l));
  }
  return out;
})()`;

const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
let failed = 0;
let shown = 0;

for (const [w, hgt] of sizes) {
  for (const theme of themes) {
    const b = await launch('about:blank', { width: w, height: hgt });
    const ui = helpers(b, w, hgt);
    try {
      await b.send('Page.navigate', { url: `${url}?renderer=webgl2&start=0` });
      const ready = 'window.kentos && window.kentos.view.backendKind.value';
      await b.waitFor(ready, 30000);
      await sleep(1200);
      await b.waitFor(ready, 20000);
      await b.eval(`window.kentos.commands.execute('view.theme.${theme}')`);
      if (scale !== 'standard') await b.eval(`(() => { document.documentElement.style.setProperty('--ui-scale', '${SCALES[scale]}'); window.kentos.prefs.uiScale.set('${scale}'); })()`);
      await b.eval('document.fonts.ready');
      await sleep(300);
      for (const item of ITEMS) {
        if (only && !only.includes(item.id)) continue;
        if (item.when && !(await item.when(ui))) continue;
        const name = `${item.id}-${w}x${hgt}-${theme}${scale === 'standard' ? '' : `-${scale}`}`;
        let faults;
        try {
          await item.open(ui);
          if (item.ready) await b.waitFor(`document.querySelector(${JSON.stringify(item.ready)})`, 8000).catch(() => {});
          await sleep(450);
          faults = await b.eval(FAULTS);
          await b.shot(name, undefined, DIR);
        } catch (e) {
          faults = [`açılamadı: ${String(e.message ?? e).slice(0, 160)}`];
        }
        shown++;
        if (faults.length) failed++;
        console.log(`${faults.length ? '✗' : '✓'} ${name}${faults.length ? `: ${faults.join('; ')}` : ''}`);
        await (item.close ?? ((u) => u.escapeAll(3)))(ui);
      }
      const errors = b.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l) && !l.includes('/v1/'));
      if (errors.length) {
        failed++;
        console.log(`✗ konsol hataları (${w}x${hgt} ${theme}): ${errors.join(' | ').slice(0, 400)}`);
      }
    } finally {
      b.close();
    }
  }
}
await server.close();
console.log(`\n${shown} görünüm denetlendi; ${failed ? `${failed} sorunlu` : 'sorun yok'}. Resimler: ${DIR}`);
process.exit(failed ? 1 : 0);

/** Actions the items use, on page `b` of size w × h. */
function helpers(b, w, h) {
  const centre = (sel) => b.eval(`(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
  const ui = {
    eval: (expr) => b.eval(expr),
    visible: (sel) => b.eval(`(() => { const el = document.querySelector(${JSON.stringify(sel)}); return !!el && el.getClientRects().length > 0; })()`),
    run: async (id, arg) => {
      await b.eval(`window.kentos.commands.execute(${JSON.stringify(id)}${arg === undefined ? '' : `, ${JSON.stringify(arg)}`})`);
      await sleep(350);
    },
    click: async (sel) => {
      const at = await centre(sel);
      if (!at) throw new Error(`yok: ${sel}`);
      await b.click(...at);
      await sleep(250);
    },
    clickFirst: async (sels) => {
      for (const sel of sels) {
        const at = await centre(sel);
        if (at && at[0] > 0) return ui.click(sel);
      }
      throw new Error(`yok: ${sels.join(' | ')}`);
    },
    clickText: async (sel, text) => {
      await b.waitFor(`[...document.querySelectorAll(${JSON.stringify(sel)})].some((e) => e.textContent.includes(${JSON.stringify(text)}))`, 6000).catch(() => {});
      const at = await b.eval(`(() => { const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => e.textContent.includes(${JSON.stringify(text)})); if (!el) return null; const r = el.getBoundingClientRect(); return [Math.round(r.left + r.width / 2), Math.round(r.top + r.height / 2)]; })()`);
      if (!at) throw new Error(`yok: ${sel} “${text}”`);
      await b.click(...at);
      await sleep(350);
    },
    rightClick: async (sel) => {
      const at = await centre(sel);
      if (!at) throw new Error(`yok: ${sel}`);
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0], y: at[1], button: 'none' });
      await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: at[0], y: at[1], button: 'right', clickCount: 1 });
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at[0], y: at[1], button: 'right', clickCount: 1 });
      await sleep(250);
    },
    /** The right button over an empty spot of the drawing: a quick click, Shift, or held during a command. */
    viewportRight: async ({ shift = false, tool = null, hold = false }) => {
      const at = await b.eval(`(() => { const r = window.kentos.view.clientRect(); return [Math.round(r.left + r.width * 0.72), Math.round(r.top + r.height * 0.3)]; })()`);
      if (tool) {
        await b.eval(`window.kentos.commands.execute(${JSON.stringify(tool)})`);
        await b.click(...at);
      }
      const modifiers = shift ? 8 : 0;
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0] + 20, y: at[1] + 10, button: 'none' });
      await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: at[0] + 20, y: at[1] + 10, button: 'right', clickCount: 1, modifiers });
      if (hold) {
        await sleep(450);
        return;
      }
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at[0] + 20, y: at[1] + 10, button: 'right', clickCount: 1, modifiers });
      await sleep(250);
    },
    /** Files the next open or import is handed, as [name, base64] pairs. */
    pick: (files) =>
      b.eval(`(() => {
        const k = window.kentos;
        const made = ${JSON.stringify(files)}.map(([name, b64]) => ({ name, getFile: async () => new Blob([Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))]) }));
        window.__pickerOriginal ??= k.files.picker;
        k.files.picker = { ...window.__pickerOriginal, open: async () => made[0], openMany: async () => made };
      })()`),
    escapeAll: async (times) => {
      // A held right button is let go first; then Esc closes what is open (a question asks, Vazgeç answers).
      await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 2, y: 2, button: 'right', clickCount: 1 }).catch(() => {});
      for (let i = 0; i < times; i++) {
        await b.key('Escape');
        await sleep(120);
      }
      await b.eval(`(() => { const k = window.kentos; if (window.__pickerOriginal) k.files.picker = window.__pickerOriginal; k.tools.activate('select'); k.selection.clear(); })()`);
    },
  };
  void w;
  void h;
  return ui;
}
