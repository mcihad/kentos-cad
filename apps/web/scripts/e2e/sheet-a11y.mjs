// The sheets' accessibility, read from Chrome's own accessibility tree (CDP `Accessibility.getFullAXTree`; no
// package): on the demo drawing, the sheet mode (the Model | Pafta tabs, the workspace with each inspector tab, the
// contextual Pafta tab of the ribbon), the template gallery, the share window and the export window. For each:
//
// - every interactive element (a button, a field, a switch, a tab, a list row one can pick …) has a role and an
//   accessible name, and the name is not several texts run together (“Sistem8”);
// - nothing takes the keyboard's focus without a role (a focusable plain element);
// - nothing reacts to the pointer (a pointer cursor) without a role or a way in from the keyboard.
//
// Then Tab is walked through the export window and the gallery: where the focus goes, in order, whether it stays in
// the window, lands only on what is shown, and stops once in a composite (a tab list, a list box: arrows inside).
//
// Writes scripts/e2e/out/a11y/sheet-a11y.json and prints the numbers; with --strict a finding fails the run.
//
//   node apps/web/scripts/e2e/sheet-a11y.mjs [--strict]
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch, OUT, sleep } from './cdp.mjs';

const WEB = fileURLToPath(new URL('../..', import.meta.url));
const DIR = join(OUT, 'a11y');
mkdirSync(DIR, { recursive: true });
const STRICT = process.argv.includes('--strict');

/** Roles a user acts on: each needs a name. */
const INTERACTIVE = new Set(['application', 'button', 'checkbox', 'switch', 'radio', 'tab', 'menuitem', 'menuitemcheckbox', 'menuitemradio', 'textbox', 'searchbox', 'combobox', 'listbox', 'option', 'slider', 'spinbutton', 'link', 'treeitem', 'gridcell', 'row', 'PopUpButton', 'ToggleButton']);
/** A name of several texts run together, or with a sentence in it (see `audit`). */
const GLUED = /[a-zçğıöşü][0-9]|[a-zçğıöşü][A-ZÇĞİÖŞÜ][a-zçğıöşü]/;
/** A name longer than this carries what belongs in a description (a note, a reason). */
const LONG = 60;
/** Roles that say nothing of what an element does. */
const NO_ROLE = new Set(['generic', 'none', 'presentation', 'StaticText', 'InlineTextBox', 'paragraph', 'Section', 'LabelText', 'Unknown', '']);

const server = await createServer({ root: WEB, configFile: join(WEB, 'vite.config.mjs'), server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const b = await launch('about:blank', { width: 1440, height: 900 });
const S = `const k = window.kentos; const m = await import('/src/app/sheet/install.ts'); const s = m.sheetsOf(k); const ta = await import('/src/app/sheet/templateActions.ts');`;
const page = (body) => b.eval(`(async () => { ${S} ${body} })()`);
const until = async (expr, what, ms = 20000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (await b.eval(`(async () => !!(${expr}))()`).catch(() => false)) return;
    await sleep(100);
  }
  throw new Error(`beklenen olmadı: ${what}`);
};

/** The AX nodes under the elements `selector` finds (each a subtree), not ignored. */
async function axNodes(selector) {
  const { root } = await b.send('DOM.getDocument', { depth: -1, pierce: true });
  const { nodeIds } = await b.send('DOM.querySelectorAll', { nodeId: root.nodeId, selector });
  const roots = new Set();
  for (const nodeId of nodeIds) roots.add((await b.send('DOM.describeNode', { nodeId })).node.backendNodeId);
  const { nodes } = await b.send('Accessibility.getFullAXTree');
  const byId = new Map(nodes.map((n) => [n.nodeId, n]));
  const out = [];
  const seen = new Set();
  const walk = (n) => {
    if (!n || seen.has(n.nodeId)) return;
    seen.add(n.nodeId);
    if (!n.ignored) out.push(n);
    for (const c of n.childIds ?? []) walk(byId.get(c));
  };
  for (const n of nodes) if (roots.has(n.backendDOMNodeId)) walk(n);
  return out;
}

const prop = (n, name) => n.properties?.find((p) => p.name === name)?.value?.value;

/** Where an element is, for the report: its tag, classes and text. */
async function describe(backendNodeId) {
  try {
    const { object } = await b.send('DOM.resolveNode', { backendNodeId });
    const { result } = await b.send('Runtime.callFunctionOn', {
      objectId: object.objectId,
      returnByValue: true,
      functionDeclaration: `function () { const e = this.nodeType === 1 ? this : this.parentElement; const c = (e.getAttribute('class') || '').trim().split(/\\s+/).slice(0, 2).join('.'); return e.localName + (c ? '.' + c : '') + (e.textContent.trim() ? ' “' + e.textContent.trim().slice(0, 30) + '”' : ''); }`,
    });
    return result.value;
  } catch {
    return `#${backendNodeId}`;
  }
}

/** One scene's findings: interactive elements without a name, focusable ones without a role, pointer-only ones. */
async function audit(scene, selector) {
  const nodes = await axNodes(selector);
  const interactive = nodes.filter((n) => INTERACTIVE.has(n.role?.value));
  const unnamed = interactive.filter((n) => !(n.name?.value ?? '').trim());
  // A name run together from several texts (a label and its count: “Sistem8”), or carrying a sentence that belongs in
  // its description (“Kurumum Kurum şablonları sonraki aşamada gelecek: …”).
  const glued = interactive.filter((n) => GLUED.test(n.name?.value ?? '') || (n.name?.value ?? '').length > LONG);
  const roleless = nodes.filter((n) => prop(n, 'focusable') === true && NO_ROLE.has(n.role?.value ?? '') && n.role?.value !== 'RootWebArea');
  // Pointer-only: reacts to the pointer (cursor) but has no role of its own or above it, and no way in from the keyboard.
  const pointer = await b.eval(`(() => {
    const out = [];
    const ROLE = 'button, a[href], input, select, textarea, summary, label, [role], [tabindex], [contenteditable=""], [contenteditable="true"], canvas';
    for (const root of document.querySelectorAll(${JSON.stringify(selector)}))
      for (const e of root.querySelectorAll('*')) {
        if (!e.checkVisibility?.()) continue;
        if (getComputedStyle(e).cursor !== 'pointer') continue;
        if (e.closest(ROLE)) continue;
        if (e.parentElement && getComputedStyle(e.parentElement).cursor === 'pointer') continue;
        const c = (e.getAttribute('class') || '').trim().split(/\\s+/).slice(0, 2).join('.');
        out.push(e.localName + (c ? '.' + c : '') + (e.textContent.trim() ? ' “' + e.textContent.trim().slice(0, 30) + '”' : ''));
      }
    return out;
  })()`);
  const result = {
    scene,
    nodes: nodes.length,
    interactive: interactive.length,
    unnamed: await Promise.all(unnamed.map(async (n) => `${n.role?.value}: ${await describe(n.backendDOMNodeId)}`)),
    glued: glued.map((n) => `${n.role?.value}: “${n.name.value.slice(0, 50)}”`),
    names: interactive.map((n) => `${n.role?.value}: ${(n.name?.value ?? '').slice(0, 80)}`),
    roleless: await Promise.all(roleless.map(async (n) => `${n.role?.value || 'rolsüz'}: ${await describe(n.backendDOMNodeId)}`)),
    pointer,
    roles: [...new Set(interactive.map((n) => n.role.value))].sort(),
  };
  console.log(`${scene}: ${result.interactive} etkileşimli öğe; adsız ${result.unnamed.length}, birleşik ya da açıklamalı adlı ${result.glued.length}, odaklanıp rolsüz ${result.roleless.length}, yalnız işaretçiyle ${result.pointer.length}`);
  for (const [what, list] of [['adsız', result.unnamed], ['birleşik ya da açıklamalı ad', result.glued], ['odaklanıp rolsüz', result.roleless], ['yalnız işaretçiyle', result.pointer]])
    for (const x of [...new Set(list)].slice(0, 12)) console.log(`   ${what}: ${x}`);
  return result;
}

/** Tab through a window from its first element: the focus's path, whether it stays inside and is shown. */
async function tabWalk(scene, selector, max = 80) {
  await b.eval(`(() => { const d = document.querySelector(${JSON.stringify(selector)}); const f = d?.querySelector('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'); f?.focus(); return !!f; })()`);
  const where = () =>
    b.eval(`(() => {
      const e = document.activeElement;
      if (!e || e === document.body) return { key: 'body', text: '(sayfa)', inside: false, shown: false };
      const name = e.getAttribute('aria-label') || e.getAttribute('title') || (e.labels?.[0]?.textContent ?? '') || e.textContent.trim() || e.getAttribute('placeholder') || e.value || '';
      const r = e.getBoundingClientRect();
      const box = e.closest('[role=radiogroup], [role=tablist], [role=listbox], [role=tree], [role=menu], [role=grid]');
      return {
        group: box ? (box.getAttribute('role') + ' “' + (box.getAttribute('aria-label') || '') + '”') : null,
        key: e.localName + '|' + (e.getAttribute('class') || '') + '|' + name.slice(0, 40) + '|' + Math.round(r.left) + ',' + Math.round(r.top),
        text: (e.getAttribute('role') || e.localName) + ' “' + name.trim().slice(0, 40) + '”',
        inside: !!e.closest(${JSON.stringify(selector)}),
        shown: r.width > 0 && r.height > 0 && e.checkVisibility?.() !== false,
      };
    })()`);
  const first = await where();
  const path = [first];
  for (let i = 0; i < max; i++) {
    await b.key('Tab');
    await sleep(60);
    const w = await where();
    if (w.key === first.key) break;
    path.push(w);
  }
  const back = path.length < max + 1;
  const outside = path.filter((p) => !p.inside);
  const hidden = path.filter((p) => !p.shown);
  // A composite (tab list, list box, radio group) is one stop, its members reached with the arrows: more is a finding.
  const groups = new Map();
  for (const p of path) if (p.group) groups.set(p.group, (groups.get(p.group) ?? 0) + 1);
  const extra = [...groups.values()].reduce((a, n) => a + n - 1, 0);
  console.log(`${scene} (Tab): ${path.length} durak${back ? ', başa döndü' : ', başa dönmedi'}; pencere dışına ${outside.length}, görünmeyene ${hidden.length}, bileşik denetimlerde fazladan ${extra}`);
  console.log(`   ${path.map((p) => p.text).join(' → ')}`);
  return { scene, stops: path.length, cycles: back, outside: outside.map((p) => p.text), hidden: hidden.map((p) => p.text), extra, path: path.map((p) => p.text) };
}

const VALUES = { il: 'Sivas', ilce: 'Suşehri', mahalle: 'Kızılırmak', ada: '1245', pafta_no: 'P-12' };
const report = { scenes: [], walks: [] };
try {
  await b.send('Page.navigate', { url: `${server.resolvedUrls.local[0]}?renderer=webgl2&start=0` });
  await b.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  await sleep(1500);
  await b.send('DOM.enable');
  await b.send('Accessibility.enable');
  await b.eval(`window.kentos.commands.execute('view.theme.light')`);
  // The ifraz sheet and an aplikasyon sketch on the demo drawing, a template of the user's own on this device.
  await page(`
    k.doc.settings.workspace.set('cad');
    const e = await s.ensureEngine();
    const tpl = (id) => e.systemTemplates().find((x) => x.meta.id === id);
    const values = ${JSON.stringify(VALUES)};
    const id = await ta.sheetFromTemplate(s, tpl('sys:ifraz-paftasi'), null, undefined, Object.entries(values).map(([name, value]) => ({ name, value })));
    await ta.sheetFromTemplate(s, tpl('sys:aplikasyon-krokisi'), null);
    await ta.saveAsTemplate(k, s, id, { name: 'Belediye ifraz paftası', description: 'Erişilebilirlik denetimi', category: 'kadastro', tags: [], papers: [{ paper: 'a3', orientation: 'landscape' }], workspaces: ['cad'], projectTypes: [] }, null);
    s.openSheet(id);
    window.__sheet = id;
    return true;`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) .sheet-stage__paper')`, 'pafta');
  await sleep(1200);

  // ── The sheet mode ──────────────────────────────────────────────────────────────────────────────────
  report.scenes.push(await audit('Model | Pafta sekmeleri', '.sheet-tabs'));
  report.scenes.push(await audit('Pafta kipi: seçimsiz, Öğe sekmesi', '.sheet-ws:not([hidden])'));
  await page(`const sh = s.state.sheet; s.state.select(sh.items.filter((i) => i.name === 'Harita').map((i) => i.id)); return true;`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) [data-section="kind:map"]')`, 'haritanın denetçisi');
  await sleep(300);
  report.scenes.push(await audit('Pafta kipi: harita seçili', '.sheet-ws:not([hidden])'));
  // The north arrow as magnetic north (Part E): the declination, its date, “Sapmayı elle gir”.
  await page(`const sh = s.book().book.sheets.find((x) => x.id === window.__sheet); const a = sh.items.find((i) => i.kind.type === 'northArrow');
    s.apply([{ op: 'setItemProps', id: a.id, patch: { kind: { north: 'magnetic', style: 'diagram' } } }], 'Manyetik'); s.state.select([a.id]); return true;`);
  await until(`document.querySelector('.sheet-ws:not([hidden]) .sheet-north__value')`, 'kuzey okunun denetçisi');
  await sleep(300);
  report.scenes.push(await audit('Pafta kipi: manyetik kuzey oku seçili', '.sheet-ws:not([hidden])'));
  await page(`s.undo(); s.state.select(s.state.sheet.items.filter((i) => i.name === 'Harita').map((i) => i.id)); return true;`);
  await sleep(300);
  for (const tab of ['page', 'preflight']) {
    await b.eval(`(() => { document.querySelector('.sheet-insp__tabs [data-tab="${tab}"]')?.click(); return true; })()`);
    await sleep(500);
    report.scenes.push(await audit(`Pafta kipi: ${tab === 'page' ? 'Sayfa' : 'Ön denetim'} sekmesi`, '.sheet-ws:not([hidden])'));
  }
  await b.eval(`(() => { document.querySelector('.sheet-insp__tabs [data-tab="item"]')?.click(); return true; })()`);
  // The inspector's tabs: one stop, the arrows move (right from Öğe: Sayfa).
  await b.eval(`(() => { document.querySelector('.sheet-insp__tabs [data-tab="item"]')?.focus(); return true; })()`);
  await b.key('ArrowRight');
  await sleep(300);
  const inspectorArrows = await b.eval(`document.activeElement?.dataset?.tab === 'page' && document.activeElement.getAttribute('aria-selected') === 'true' && document.activeElement.tabIndex === 0`);
  await b.key('ArrowLeft');
  await sleep(200);
  report.scenes.push(await audit('Pafta şeridi (bağlamsal sekme)', '.ribbon'));
  report.scenes.push(await audit('Durum çubuğunun pafta hücreleri', '.sheet-status__cell'));

  // ── The export window ────────────────────────────────────────────────────────────────────────────────
  await page(`k.commands.execute('sheet.export.pdf'); return true;`);
  await until(`document.querySelector('.sheet-export .sheet-export__file')`, 'dışa aktarma penceresi');
  await sleep(400);
  report.scenes.push(await audit('Dışa aktarma penceresi (PDF)', '.sheet-export-dialog'));
  report.walks.push(await tabWalk('Dışa aktarma penceresi', '.sheet-export-dialog'));
  await b.key('Escape');
  await sleep(300);

  // ── The gallery and the share window ─────────────────────────────────────────────────────────────────
  await page(`k.commands.execute('sheet.fromTemplate'); return true;`);
  await until(`document.querySelector('.dialog--tgal .tcard')`, 'galeri');
  await sleep(1200);
  report.scenes.push(await audit('Şablon galerisi (Sistem)', '.dialog--tgal'));
  report.walks.push(await tabWalk('Şablon galerisi', '.dialog--tgal'));
  // Inside a composite the arrows move: the sources (down: Benim, and it shows), the cards (right: the next, chosen).
  const arrows = {};
  await b.eval(`(() => { document.querySelector('.dialog--tgal .tgal__src[aria-selected="true"]')?.focus(); return true; })()`);
  await b.key('ArrowDown');
  await sleep(300);
  arrows.sources = await b.eval(`document.activeElement?.dataset?.section === 'mine' && document.activeElement.getAttribute('aria-selected') === 'true' && /^Benim/.test(document.querySelector('.dialog--tgal .tgal__listhead')?.textContent ?? '')`);
  await b.key('ArrowUp');
  await sleep(300);
  await b.eval(`(() => { document.querySelector('.dialog--tgal .tcard[tabindex="0"]')?.focus(); return true; })()`);
  const firstCard = await b.eval(`document.activeElement?.dataset?.id ?? null`);
  await b.key('ArrowRight');
  await sleep(300);
  arrows.cards = await b.eval(`(() => { const e = document.activeElement; return !!e?.classList.contains('tcard') && e.dataset.id !== ${JSON.stringify('__FIRST__')} && e.getAttribute('aria-selected') === 'true' && e.tabIndex === 0; })()`.replace('__FIRST__', firstCard ?? ''));
  report.arrows = arrows;
  await b.eval(`(() => { document.querySelector('.dialog--tgal .tgal__src[data-section="mine"]')?.click(); return true; })()`);
  await sleep(800);
  await b.eval(`(() => { document.querySelector('.dialog--tgal .tcard')?.click(); return true; })()`);
  await sleep(500);
  report.scenes.push(await audit('Şablon galerisi (Benim, şablon seçili)', '.dialog--tgal'));
  await b.eval(`(() => { [...document.querySelectorAll('.dialog--tgal button')].find((x) => /^Paylaş/.test(x.textContent.trim()))?.click(); return true; })()`);
  await until(`[...document.querySelectorAll('.dialog')].some((d) => /Şablonu paylaş/.test(d.textContent))`, 'paylaşım penceresi');
  await sleep(400);
  report.scenes.push(await audit('Paylaşım penceresi', '.dialog--share'));
  report.arrows.inspector = inspectorArrows;
  console.log(`oklar: galeri kaynakları ${report.arrows.sources ? 'evet' : 'HAYIR'}, kartlar ${report.arrows.cards ? 'evet' : 'HAYIR'}, denetçi sekmeleri ${report.arrows.inspector ? 'evet' : 'HAYIR'}`);
  const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
  report.console = errors;
} finally {
  b.close();
  await server.close();
}
writeFileSync(join(DIR, 'sheet-a11y.json'), JSON.stringify(report, null, 2));
const sum = (k) => report.scenes.reduce((n, s) => n + (Array.isArray(s[k]) ? s[k].length : s[k]), 0);
const walkFindings = report.walks.reduce((n, w) => n + w.outside.length + w.hidden.length + w.extra + (w.cycles ? 0 : 1), 0) + Object.values(report.arrows ?? {}).filter((x) => !x).length;
const findings = sum('unnamed') + sum('glued') + sum('roleless') + sum('pointer') + walkFindings;
console.log(`\ntoplam: ${sum('interactive')} etkileşimli öğe; adsız ${sum('unnamed')}, birleşik ya da açıklamalı adlı ${sum('glued')}, odaklanıp rolsüz ${sum('roleless')}, yalnız işaretçiyle ${sum('pointer')}; Tab bulguları ${walkFindings}`);
console.log(`yazıldı: ${join(DIR, 'sheet-a11y.json')}`);
process.exit(STRICT && findings ? 1 : 0);
