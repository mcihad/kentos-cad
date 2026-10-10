// Layout pass (pnpm e2e:layout): opens every window, menu and panel section of the web at the shell's
// narrowest size (1100×650) and at 1440×900, in both themes, and checks each for what a user would see
// as broken (DESIGN.md §5.1, §7): a window or menu reaching past the screen, a window body or footer
// wider than the window (a horizontal scrollbar), a footer button pushed out, a button or menu row whose
// words are cut, the shell's bars overflowing, the information card over a drawing object leaving the
// drawing or letting its text out (the demo's MPYY names), a tooltip leaving the window. A picture of each
// goes to scripts/e2e/out/layout/ for a person to read what a script cannot judge: scrollbars over content,
// text cut inside fields, balance.
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
/** The text size (Uygulama ayarları → Görünüm → Yazı boyutu) in pixels, by the names of its five steps (docs/adr/0126). */
const SIZES = { small: 12, standard: 13, large: 14, xlarge: 15, xxlarge: 16 };
const scale = opt('scale')?.[0] ?? 'standard';
const DIR = join(OUT, 'layout');
mkdirSync(DIR, { recursive: true });

const gis = new URL('../../../../fixtures/formats/v1/gis/', import.meta.url);
const formats = new URL('../../../../fixtures/formats/v1/', import.meta.url);
const raw = (url) => readFileSync(url).toString('base64');

/**
 * Objects of the demo drawing to rest the pointer on, found in the page: the object and the world point to
 * rest on. The showcase symbol whose layer has the longest name (the point inside its sample area), and the
 * longest title text.
 */
const LONGEST_LAYER = `(k) => {
  let best = null;
  for (const e of k.doc.all()) {
    if (!e.symbol || e.kind !== 'polygon') continue;
    const n = (k.doc.layers.get(e.layerId)?.name ?? '').length;
    if (!best || n > best.n) best = { e, n };
  }
  if (!best) return null;
  const pts = best.e.pts;
  return { x: pts.reduce((s, q) => s + q.x, 0) / pts.length, y: pts.reduce((s, q) => s + q.y, 0) / pts.length };
}`;
const LONGEST_TEXT = `(k) => {
  let best = null;
  for (const e of k.doc.all()) if (e.kind === 'text' && (!best || e.text.length > best.text.length)) best = e;
  return best && { x: best.p.x + 2, y: best.p.y + best.height * 0.4 };
}`;

/** Each project type's ribbon tabs (docs/adr/0165 §6), as the web shows them: the file tab is the app menu, Seçim needs a selection. */
const TYPE_TABS = Object.fromEntries(
  Object.entries(JSON.parse(readFileSync(new URL('../../../../fixtures/shell/v1/ribbon.json', import.meta.url), 'utf8')).tabs).map(([type, tabs]) => [
    type,
    tabs.filter((t) => !t.contextual && t.id !== 'file').map((t) => t.id),
  ]),
);
/** The demo drawing made a project of `type` (null: not asked its type, shown as CBS). */
const typeSet = (type) => `window.kentos.doc.settings.workspace.set(${JSON.stringify(type)})`;

/** What each item opens, and how. `b` is the page; `ui` the helpers below. `must`: what has to be showing. */
const ITEMS = [
  // The shell itself: the ribbon (the only chrome, docs/adr/0155) and the bars at this width.
  { id: 'shell', open: async () => {} },
  { id: 'appmenu', open: (ui) => ui.click('.brand') },
  // Every tab of each project type's ribbon (fixtures/shell/v1/ribbon.json): the demo drawing's, a project not asked its
  // type, is CBS's; a CAD project's with the type set and put back after.
  ...TYPE_TABS.gis.map((t) => ({ id: `tab-${t}`, open: (ui) => ui.click(`.ribbon__tab[data-tab="${t}"]`), close: (ui) => ui.click('.ribbon__tab[data-tab="home"]') })),
  ...TYPE_TABS.cad.map((t) => ({
    id: `tab-cad-${t}`,
    open: async (ui) => (await ui.eval(typeSet('cad')), await sleep(300), await ui.click(`.ribbon__tab[data-tab="${t}"]`)),
    close: async (ui) => (await ui.click('.ribbon__tab[data-tab="home"]'), await ui.eval(typeSet(null)), await sleep(300)),
  })),
  { id: 'ribbon-layer', open: (ui) => ui.click('.ribbon__strip .dropdown--layer') },
  // The Özellikler panel's colour list: in the panel, or under the panel's button when the window folds it (1100 wide).
  {
    id: 'ribbon-properties',
    open: async (ui) => {
      if (!(await ui.visible('.ribbon__strip [aria-label="Renk"]'))) await ui.click('.ribbon__strip .rpanel__collapsed[aria-label="Özellikler"]');
      await ui.clickFirst(['.ribbon__strip [aria-label="Renk"]', '.ribbon-pop [aria-label="Renk"]']);
    },
  },
  { id: 'status-renderer', open: (ui) => ui.click('.status__renderer') },
  { id: 'status-mode', open: (ui) => ui.click('.status__mode') },
  // Topoloji's right-click menu: Noktalar da (docs/adr/0160 §1).
  { id: 'status-topology', open: (ui) => ui.rightClick('.status__toggle[data-command="draft.topology"]') },
  // Çakışma's right-click menu: the modes, and Katmanlar opened (docs/adr/0162 §1).
  { id: 'status-overlap', open: (ui) => ui.rightClick('.status__toggle[data-command="draft.overlap"]') },
  {
    id: 'status-overlap-layers',
    open: async (ui) => (await ui.rightClick('.status__toggle[data-command="draft.overlap"]'), await ui.clickText('.menu__item', 'Katmanlar')),
    close: (ui) => ui.escapeAll(2),
  },
  // Kenet's right-click menu: the kinds one by one, and Karelaj aralığı opened (docs/adr/0163 §6).
  { id: 'status-snap', open: (ui) => ui.rightClick('.status__toggle[data-command="draft.snap"]') },
  {
    id: 'status-snap-grid',
    open: async (ui) => (await ui.rightClick('.status__toggle[data-command="draft.snap"]'), await ui.clickText('.menu__item', 'Karelaj aralığı')),
    close: (ui) => ui.escapeAll(2),
  },
  // Süzgeç's right-click menu and Giriş › Seçim süzgeci ▾: the seventeen kinds by short name and icon, then every kind
  // or none (docs/adr/0187 §5, 8 Ekim); the drop-down under its folded panel's button in a narrow window.
  { id: 'status-selectfilter', open: (ui) => ui.rightClick('.status__toggle[data-command="edit.selectFilter"]') },
  {
    id: 'ribbon-selectfilter',
    open: async (ui) => {
      if (!(await ui.visible('.ribbon__strip .rbtn[data-menu="Seçim süzgeci"]'))) await ui.click('.ribbon__strip .rpanel__collapsed[aria-label="Seçim"]');
      await ui.clickFirst(['.ribbon__strip .rbtn[data-menu="Seçim süzgeci"]', '.ribbon-pop .rbtn[data-menu="Seçim süzgeci"]']);
    },
  },
  { id: 'status-account', open: (ui) => ui.click('.status__server') },
  { id: 'layer-row', open: (ui) => ui.rightClick('.panel--layers .tree__row[data-id="taslak"] .tree__name') },
  { id: 'layer-color', open: (ui) => ui.click('.panel--layers .tree__row[data-id="taslak"] .swatch--btn') },
  // A layer's own snapping (docs/adr/0163 §4): the magnets off and dashed, and Kenet ▸ open on a row.
  {
    id: 'layer-snap',
    open: async (ui) => (
      await ui.eval(`(() => { const L = window.kentos.doc.layers; const [a, b] = L.leaves().slice(1, 3); L.setSnap(a.id, { off: true }); L.setSnap(b.id, { kinds: ['endpoint', 'intersection'] }); })()`),
      await ui.rightClick('.panel--layers .tree__row[data-id="taslak"] .tree__name'),
      await ui.clickText('.menu__item', 'Kenet')
    ),
    close: async (ui) => (
      await ui.escapeAll(2),
      await ui.eval(`(() => { const L = window.kentos.doc.layers; for (const l of L.leaves()) L.setSnap(l.id, null); })()`)
    ),
  },
  { id: 'viewport-idle', open: (ui) => ui.viewportRight({}) },
  { id: 'viewport-snap', open: (ui) => ui.viewportRight({ shift: true }) },
  { id: 'viewport-command', open: (ui) => ui.viewportRight({ tool: 'tool.line', hold: true }), close: (ui) => ui.escapeAll(3) },
  // Kilit ▸ (docs/adr/0166 §6): Çoklu çizgi has its first point, so the locks can be had.
  {
    id: 'viewport-command-locks',
    open: async (ui) => (await ui.viewportRight({ tool: 'tool.polyline', hold: true }), await ui.clickText('.menu__item', 'Kilit')),
    close: (ui) => ui.escapeAll(4),
  },
  { id: 'ribbon-qat', open: (ui) => ui.click('.ribbon__qat-more'), close: (ui) => ui.escapeAll(2) },
  { id: 'ribbon-help', open: (ui) => ui.click('.ribbon__icon[aria-label="Yardım"]'), close: (ui) => ui.escapeAll(2) },
  { id: 'shortcuts', open: (ui) => ui.run('help.shortcuts') },
  { id: 'about', open: (ui) => ui.run('help.about') },
  ...['appearance', 'snap', 'newProjects', 'engine', 'file'].map((s) => ({ id: `app-settings-${s}`, open: (ui) => ui.run('tools.options', s) })),
  // Kenetleme's new kinds with their glyphs, and its lower half: Karelaj and Kenedin kapsamı (docs/adr/0163).
  {
    id: 'app-settings-snap-kinds',
    open: async (ui) => (
      await ui.run('tools.options', 'snap'),
      await ui.eval(`(() => { const row = [...document.querySelectorAll('.settings__content .srow')].find((r) => r.textContent.includes('En yakın')); if (row) row.scrollIntoView({ block: 'start' }); })()`),
      await sleep(150)
    ),
  },
  {
    id: 'app-settings-snap-lower',
    open: async (ui) => (await ui.run('tools.options', 'snap'), await ui.eval(`(() => { const c = document.querySelector('.settings__content'); if (c) c.scrollTop = c.scrollHeight; })()`), await sleep(150)),
  },
  { id: 'project-settings-general', open: (ui) => ui.run('file.settings') },
  { id: 'project-settings-crs', open: (ui) => ui.run('crs.set') },
  { id: 'project-settings-units', open: async (ui) => (await ui.run('file.settings'), await ui.clickText('.settings__navitem', 'Birimler')) },
  { id: 'new-project', open: (ui) => ui.run('file.new') },
  // The wizard's other steps (docs/adr/0165 §3): a CBS project's lists and zone strip, a CAD project's units, the summary.
  { id: 'new-project-coords', open: async (ui) => (await ui.run('file.new'), await ui.click('.dialog--wizard .wspick__card[data-mode="gis"]'), await ui.clickText('.dialog__foot .btn--primary', 'İleri')) },
  { id: 'new-project-units', open: async (ui) => (await ui.run('file.new'), await ui.click('.dialog--wizard .wspick__card[data-mode="cad"]'), await ui.clickText('.dialog__foot .btn--primary', 'İleri')) },
  { id: 'new-project-details', open: async (ui) => (await ui.run('file.new'), await ui.clickText('.dialog__foot .btn--primary', 'İleri'), await ui.clickText('.dialog__foot .btn--primary', 'İleri')) },
  { id: 'start', open: (ui) => ui.run('file.start') },
  { id: 'import-ncn', open: async (ui) => (await ui.pick([['liste.ncn', btoa('1001 487061.123 4420101.456 105.2\r\n1002 487071.5 4420111.25 106.75\r\n')]]), await ui.run('file.import.ncn')), ready: '.dialog--io tbody tr' },
  { id: 'import-dxf', open: async (ui) => (await ui.pick([['entities.dxf', raw(new URL('entities.dxf', formats))]]), await ui.run('file.import.dxf')), ready: '.dialog--io .dialog__foot .btn--primary' },
  { id: 'import-geojson', open: async (ui) => (await ui.pick([['features.geojson', raw(new URL('features.geojson', gis))]]), await ui.run('file.import.geojson')), ready: '.dialog--io tbody tr' },
  { id: 'import-shp', open: async (ui) => (await ui.pick(['karisik.shp', 'karisik.shx', 'karisik.dbf', 'karisik.prj', 'karisik.cpg'].map((n) => [n, raw(new URL(n, gis))])), await ui.run('file.import.shp')), ready: '.dialog--io tbody tr' },
  { id: 'export-dxf', open: (ui) => ui.run('file.export.dxf') },
  { id: 'export-geojson', open: (ui) => ui.run('file.export.geojson') },
  { id: 'export-ncn', open: (ui) => ui.run('file.export.ncn') },
  ...['calc.traverse', 'calc.polar', 'calc.stakeout', 'calc.forward', 'calc.resection', 'transform.fit', 'transform.edgematch', 'crs.transform'].map((c) => ({ id: c.replace('.', '-'), open: (ui) => ui.run(c) })),
  // Vektör oturtma's Kauçuk levha (docs/adr/0158 §5): four kinds and Sabit on every row; Helmert again on closing (what is typed stays for the session).
  {
    id: 'transform-fit-rubber',
    open: async (ui) => (await ui.run('transform.fit'), await ui.clickText('.dialog--fit .seg__opt', 'Kauçuk levha')),
    close: async (ui) => (await ui.clickText('.dialog--fit .seg__opt', 'Helmert'), await ui.escapeAll(2)),
  },
  // Koordinat dönüştür's list (docs/adr/0167 §4): the table and what it says; Tek nokta again on closing (what is typed stays for the session).
  {
    id: 'crs-transform-list',
    open: async (ui) => (await ui.run('crs.transform'), await ui.clickText('.dialog--calc .seg__opt', 'Liste')),
    close: async (ui) => (await ui.clickText('.dialog--calc .seg__opt', 'Tek nokta'), await ui.escapeAll(2)),
  },
  // Ağlar (docs/adr/0209 §10): the empty list, and a new network's whole form (Sil takes it back, so closing asks nothing);
  // an İşlemler network tool in a project without networks (its Ağlar… button).
  { id: 'networks', open: (ui) => ui.run('network.manage'), ready: '.dialog--networks' },
  {
    id: 'networks-new',
    open: async (ui) => (await ui.run('network.manage'), await ui.clickText('.dialog--networks .net-actions .btn', 'Yeni ağ')),
    close: async (ui) => (await ui.clickText('.dialog--networks .net-actions .btn', 'Sil'), await ui.escapeAll(2)),
    must: '.dialog--networks .net-line--edge',
  },
  { id: 'processing-network', open: (ui) => ui.run('processing.run.network.closestFacility'), ready: '.dialog--ptool' },
  // Zaman ve senaryolar (docs/adr/0210 §10): Zaman ayarları over the parcels, Senaryo oluştur with the tree's layers,
  // and the time slider's bar under the drawing (the parcels given years for it; both steps undone on closing).
  { id: 'time-layer', open: async (ui) => (await ui.eval(`window.kentos.doc.layers.setActive('parsel')`), await ui.run('time.layer')), ready: '.dialog--time-layer' },
  { id: 'scenario-create', open: (ui) => ui.run('scenario.create'), ready: '.dialog--scenario' },
  {
    id: 'time-bar',
    open: async (ui) => {
      await ui.eval(
        `(() => { const d = window.kentos.doc; d.updateMany(d.byLayer('parsel').map((e, i) => ({ id: e.id, attrs: { ...e.attrs, tarih: (2001 + (i % 20)) + '-01-01' } })), 'Tarih'); d.setLayerTime('parsel', { start: 'tarih', cumulative: true }, 'Zaman ayarları'); })()`,
      );
      await ui.run('time.slider');
    },
    close: async (ui) => (await ui.run('time.slider'), await ui.eval('(() => { window.kentos.doc.undo(); window.kentos.doc.undo(); })()')),
    must: '.timebar:not([hidden])',
  },
  // Katman süzgeci (docs/adr/0211 §4): the window over the parcels, with a condition that does not compile (its error
  // in place), and the layer tree with a filtered layer (its funnel and “geçen / bütün”; the step undone on closing).
  { id: 'layer-filter', open: async (ui) => (await ui.eval(`window.kentos.doc.layers.setActive('parsel')`), await ui.run('layer.filter')), ready: '.dialog--layer-filter' },
  // Etiketler (docs/adr/0212 §4): the parcels' labelling as a single label and in rules, each of the five tabs, a
  // class's long condition and an expression that does not compile.
  { id: 'labels', open: async (ui) => (await ui.eval(`window.kentos.doc.layers.setActive('parsel')`), await ui.run('layer.labels')), ready: '.dialog--labels' },
  ...[
    ['labels-rules-text', 'Metin'],
    ['labels-rules-place', 'Yerleşim'],
    ['labels-rules-look', 'Biçim'],
    ['labels-rules-fit', 'Sığdırma'],
    ['labels-rules-order', 'Öncelik'],
  ].map(([id, tab]) => ({
    id,
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.layers.setActive('parsel')`);
      await ui.run('layer.labels');
      await ui.clickText('.dialog--labels .seg button', 'Kurallı');
      await ui.eval(
        `(() => { const f = document.querySelector('.dialog--labels input[aria-label="Sınıfın koşulu"]'); f.value = "Nitelik = 'Arsa' ve $alan > 500 ve Malik <> '' ve Ada içinde ('101', '102', '103')"; f.dispatchEvent(new Event('change', { bubbles: true })); })()`,
      );
      await ui.clickText('.dialog--labels .lbl-tabs .tab', tab);
    },
    ready: '.dialog--labels',
  })),
  {
    id: 'labels-error',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.layers.setActive('parsel')`);
      await ui.run('layer.labels');
      await ui.clickText('.dialog--labels .seg button', 'İfade');
      await ui.eval(
        `(() => { const f = document.querySelector('.dialog--labels input[aria-label="İfade"]'); f.value = "Ada || '/' ||"; f.dispatchEvent(new Event('change', { bubbles: true })); })()`,
      );
    },
    ready: '.dialog--labels',
  },
  {
    id: 'layer-filter-error',
    open: async (ui) => {
      await ui.eval(`window.kentos.doc.layers.setActive('parsel')`);
      await ui.run('layer.filter');
      await ui.eval(
        `(() => { const f = document.querySelector('.lfilter__expr'); f.value = "Nitelik = 'Arsa' ve $alan >"; f.dispatchEvent(new Event('input', { bubbles: true })); })()`,
      );
    },
    must: '.lfilter__expr.is-invalid',
  },
  {
    id: 'layer-tree-filtered',
    open: (ui) => ui.eval(`window.kentos.doc.setLayerFilter('parsel', { expression: '$alan > 400' }, 'Katman süzgeci')`),
    close: (ui) => ui.eval('window.kentos.doc.undo()'),
    must: '.tree__filter:not([hidden])',
  },
  { id: 'style-manager', open: (ui) => ui.run('style.manager'), ready: '.smgr__grid, .dialog' },
  { id: 'symbol-designer', open: async (ui) => (await ui.run('style.manager'), await ui.clickText('.dialog button', 'Yeni sembol'), await ui.clickText('.menu__item', 'Alan sembolü')) },
  { id: 'layer-style', open: async (ui) => (await ui.eval(`window.kentos.doc.layers.setActive('ada')`), await ui.run('style.layerStyle')) },
  { id: 'legend', open: (ui) => ui.run('style.legend') },
  { id: 'svg-editor', open: (ui) => ui.run('style.svgEditor') },
  { id: 'processing-tool', open: (ui) => ui.run('map.edgeLengths') },
  // İfadeyle seç: the expression field with the builder's ε beside it.
  { id: 'processing-expression', open: (ui) => ui.run('processing.run.selection.byExpression'), ready: '.exprb-open' },
  // The expression builder (DESIGN.md §7.16) over İfadeyle seç: a call being written (its signature and help),
  // the completion list as a name is typed, and an error with a field's values listed.
  { id: 'expression-builder', open: (ui) => ui.builder("Nitelik = 'Arsa' ve yuvarla($alan, "), ready: '.dialog--exprb .xed__sigcode' },
  { id: 'expression-builder-complete', open: (ui) => ui.builder("Nitelik = 'Arsa' ve yu"), must: '.xed__list:not([hidden])' },
  {
    id: 'expression-builder-error',
    open: async (ui) => {
      await ui.builder('eğer(Nitelik = , 1, 2)');
      await ui.click('.xtree__row.xtree__item');
      await ui.clickText('.xhelp__vbtns .btn', 'Örnek değerler');
    },
    ready: '.xhelp__value',
  },
  // Akış (docs/adr/0101): the same expression as nodes; a node selected, with its inspector over the help.
  {
    id: 'expression-flow',
    open: async (ui) => (await ui.builder("durum eğer Nitelik = 'Arsa' ve $alan > 500 ise yuvarla($alan / 1000, 2) yoksa 0 son"), await ui.clickText('.exprb__tab', 'Akış')),
    ready: '.xfn[data-id="0"]',
  },
  {
    id: 'expression-flow-node',
    open: async (ui) => (await ui.builder("yuvarla(Nitelik || ' ' || ?, 2)"), await ui.clickText('.exprb__tab', 'Akış'), await ui.click('.xfn[data-id="0"] .xfn__title')),
    ready: '.xfi:not([hidden])',
  },
  { id: 'model-designer', open: (ui) => ui.run('processing.newModel') },
  { id: 'cloud-login', open: (ui) => ui.run('cloud.signIn') },
  { id: 'question-layer-remove', open: async (ui) => (await ui.rightClick('.panel--layers .tree__row[data-id="parsel"] .tree__name'), await ui.clickText('.menu__item', 'Sil')), ready: '.dialog--confirm' },
  // A project Editor in a cloud database project: Yeni katman is off, its tooltip says why (ui/layers/treeRights.ts).
  {
    id: 'tooltip-tree-locked',
    open: async (ui) => {
      await ui.eval(
        `window.kentos.cloud.project.set({ tenantId: 't', tenantName: 'Büro', tenantKind: 'organization', projectId: 'p', name: 'Ada 101', role: 'editor', state: 'active', storage: 'database', permissions: ['project.read', 'feature.write'], canWrite: true, canEditMeta: false })`,
      );
      await ui.hover('.panel--layers [data-command="layer.new"]');
    },
    ready: '.tooltip__note',
    close: async (ui) => (await ui.eval('window.kentos.cloud.project.set(null)'), await ui.escapeAll(3)),
  },
  // Son revizyonu aç over unsaved work (ui/cloud/FileConflict.ts offerNewest) needs a file project: the same question
  // with its texts, from the widget.
  {
    id: 'question-newest-unsaved',
    open: (ui) =>
      ui.eval(
        `import('/src/ui/widgets/confirm.ts').then((m) => void m.askUnsaved({ name: 'Ada 1244–1249 aplikasyon (kopya)', after: 'Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır (açık çizim revizyon 12). Saklamak için önce Kaydet ile kaydedin.', verb: 'aç', canSave: false }))`,
      ),
    ready: '.dialog--confirm',
  },
  // The information card over a drawing object (HoverCard, DESIGN.md §7.4.2): the demo's MPYY showcase names its
  // sections in full, so a symbol's layer name runs to a hundred characters. In the middle of the drawing, in its
  // bottom-right corner (the card goes left of and above the pointer), and over a long title near the right edge.
  ...[
    ['hover-card-long', LONGEST_LAYER, 0.5, 0.5],
    ['hover-card-corner', LONGEST_LAYER, 0.97, 0.95],
    ['hover-card-text', LONGEST_TEXT, 0.97, 0.5],
  ].map(([id, pick, fx, fy]) => ({ id, open: (ui) => ui.hoverObject(pick, fx, fy), must: '.hover-card:not([hidden])', close: (ui) => ui.leaveDrawing() })),
  // A tooltip at the window's right edge: the ribbon's last button, and a row of the processing tree in the right dock.
  {
    id: 'tooltip-ribbon-edge',
    open: (ui) => ui.hoverRightmost('.ribbon__strip button'),
    must: '.tooltip[data-open]',
    close: (ui) => ui.escapeAll(2),
  },
  // Last: it leaves the drawing unsaved.
  { id: 'question-unsaved', open: async (ui) => (await ui.eval(`window.kentos.doc.name.set('Soru')`), await ui.run('file.new'), await ui.clickText('.dialog__foot .btn--primary', 'İleri'), await ui.clickText('.dialog__foot .btn--primary', 'İleri'), await ui.clickText('.dialog__foot .btn--primary', 'Oluştur')), ready: '.dialog--confirm' },
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
  for (const bar of ['.ribbon__bar', '.ribbon__strip', '.status']) {
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
  // Cards beside the pointer or a control: the information card stays in the drawing area, a tooltip in the
  // window, and neither lets its text out of its box.
  const drawing = document.querySelector('.viewport')?.getBoundingClientRect();
  for (const card of document.querySelectorAll('.hover-card:not([hidden]), .tooltip[data-open]')) {
    const hover = card.classList.contains('hover-card');
    const r = rect(card);
    const box = hover && drawing ? drawing : { left: 0, top: 0, right: W, bottom: H };
    if (r.left < box.left - 0.5 || r.top < box.top - 0.5 || r.right > box.right + 0.5 || r.bottom > box.bottom + 0.5)
      out.push((hover ? 'bilgi kartı çizim alanından' : 'ipucu ekrandan') + ' taşıyor');
    for (const el of card.querySelectorAll('*')) {
      const e = rect(el);
      if (e.width && (e.right > r.right + 0.5 || e.left < r.left - 0.5)) {
        out.push('yazı kartın dışına taşıyor: ' + words(el));
        break;
      }
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
      if (scale !== 'standard') await b.eval(`window.kentos.prefs.textSize.set(${SIZES[scale]})`);
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
          if (item.must && !(await b.eval(`!!document.querySelector(${JSON.stringify(item.must)})`))) faults.push(`görünmedi: ${item.must}`);
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
    /** İfadeyle seç, its expression field's builder (ε), and `text` typed in place of the expression. */
    builder: async (text) => {
      await ui.run('processing.run.selection.byExpression');
      await b.waitFor(`document.querySelector('.exprb-open')`, 8000);
      await ui.click('.exprb-open');
      await b.waitFor(`document.querySelector('.dialog--exprb .xed__input')`, 8000);
      await sleep(300);
      // The builder opens in the view it was last in: the text first.
      await b.eval(`(() => { const t = document.querySelector('.dialog--exprb .exprb__tab'); if (t && t.getAttribute('aria-selected') !== 'true') t.click(); })()`);
      await sleep(100);
      await ui.click('.dialog--exprb .xed__input');
      await b.eval(`document.querySelector('.dialog--exprb .xed__input').select()`);
      await b.type(text);
      await sleep(200);
    },
    /** Files the next open or import is handed, as [name, base64] pairs. */
    pick: (files) =>
      b.eval(`(() => {
        const k = window.kentos;
        const made = ${JSON.stringify(files)}.map(([name, b64]) => ({ name, getFile: async () => new Blob([Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))]) }));
        window.__pickerOriginal ??= k.files.picker;
        k.files.picker = { ...window.__pickerOriginal, open: async () => made[0], openMany: async () => made };
      })()`),
    /** The pointer over an element, long enough for its tooltip. */
    hover: async (sel) => {
      const at = await centre(sel);
      if (!at) throw new Error(`yok: ${sel}`);
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0], y: at[1], button: 'none' });
      await sleep(900);
    },
    /**
     * The pointer resting on a drawing object: `pick` (a page function of window.kentos) gives the world
     * point, which the view puts at (fx, fy) of the drawing area at 4 px/m before the pointer goes there.
     */
    hoverObject: async (pick, fx, fy) => {
      const at = await b.eval(`(() => {
        const k = window.kentos;
        const p = (${pick})(k);
        if (!p) return null;
        k.tools.activate('select');
        const c = k.view.camera;
        c.scale = 4;
        c.center = { x: p.x - (${fx} - 0.5) * c.width / c.scale, y: p.y + (${fy} - 0.5) * c.height / c.scale };
        c.panBy(0, 0);
        const r = k.view.clientRect();
        return [Math.round(r.left + ${fx} * c.width), Math.round(r.top + ${fy} * c.height)];
      })()`);
      if (!at) throw new Error('çizimde uygun nesne yok');
      await sleep(250);
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0] - 4, y: at[1] - 3, button: 'none' });
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0], y: at[1], button: 'none' });
      await sleep(900);
    },
    /** The pointer over the rightmost of the visible elements `sel` matches, long enough for its tooltip. */
    hoverRightmost: async (sel) => {
      const at = await b.eval(`(() => {
        let best = null;
        for (const el of document.querySelectorAll(${JSON.stringify(sel)})) {
          const r = el.getBoundingClientRect();
          if (r.width && r.height && (!best || r.right > best.right)) best = r;
        }
        return best && [Math.round(best.left + best.width / 2), Math.round(best.top + best.height / 2)];
      })()`);
      if (!at) throw new Error(`yok: ${sel}`);
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at[0], y: at[1], button: 'none' });
      await sleep(900);
    },
    /** The pointer off the drawing (the card goes), and the drawing back to its extent. */
    leaveDrawing: async () => {
      await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2, button: 'none' });
      await b.eval(`window.kentos.commands.execute('view.zoomExtents')`);
      await ui.escapeAll(1);
    },
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
