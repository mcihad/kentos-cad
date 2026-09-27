// Feature inventory of the web app (TODOS.md BASE-04): commands, tools,
// processing tools and models, work modes, settings, browser storage, the
// `.kcad` fields (v1 read, v2 written), windows and panels, the menu and ribbon layout in
// order, and the icon set. Each item has a status
// (implemented / partial / pending), per-platform status and the tests that
// mention it. The inventory is derived, never typed in:
//   - the running app's registries (collect.mjs, dev server + headless Chrome);
//   - a scan of the sources (sources.mjs);
//   - hand-written notes, statuses and acceptance scenarios from
//     docs/inventory/annotations.json; a note for an item that no longer
//     exists is an error, so notes cannot go stale unseen.
// Writes docs/inventory/web.json (machine-readable) and web.md (summary).
// `--check` writes nothing and fails when either file is out of date.
//
//   node scripts/inventory/inventory.mjs [--check]     (pnpm inventory, pnpm inventory:check)
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch } from '../e2e/cdp.mjs';
import { collectInPage } from './collect.mjs';
import { quoted, scanFileFields, scanScreens, scanStorage, testIndex, uiIndex } from './sources.mjs';
import { summaryMarkdown } from './summary.mjs';

const WEB = fileURLToPath(new URL('../..', import.meta.url));
const ROOT = join(WEB, '../..');
const OUT = join(ROOT, 'docs/inventory');
const check = process.argv.includes('--check');

const STATUS = ['implemented', 'partial', 'pending'];
/** Per platform: a status, `none` (not built there yet) or `n/a` (does not apply there). */
const PLATFORM = [...STATUS, 'none', 'n/a'];

// ── The running app ──────────────────────────────────────────────────
const server = await createServer({ root: WEB, server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
let live;
const browser = await launch(`${server.resolvedUrls.local[0]}?start=0`, { width: 1440, height: 860 });
try {
  await browser.waitFor('window.kentos && window.kentos.view.backendKind.value', 30000);
  live = await browser.eval(`(${collectInPage.toString()})()`);
  const errors = browser.consoleLog.filter((l) => /^(error|EXCEPTION)/.test(l));
  if (errors.length) throw new Error(`Uygulama açılırken hata verdi:\n${errors.join('\n')}`);
} finally {
  browser.close();
  await server.close();
}

// ── Items ────────────────────────────────────────────────────────────
const refs = testIndex(ROOT, WEB);
const uiRefs = uiIndex(ROOT, WEB);
const byId = (a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
const item = (fields, status, tests) => ({ ...fields, status, platforms: { web: status, desktop: 'none' }, ...(tests ? { tests } : {}) });

const sections = {
  commands: live.commands.map(({ pending, ...c }) => item({ ...c, uiSources: uiRefs(quoted(c.id)) }, pending ? 'pending' : 'implemented', refs(quoted(c.id)))),
  tools: live.tools.map(({ ready, ...t }) => item(t, ready ? 'implemented' : 'pending', refs([...quoted(`tool.${t.id}`), `activate('${t.id}')`, `activate("${t.id}")`]))),
  processing: live.processing.map((t) => item(t, 'implemented', refs([...quoted(t.id), ...quoted(t.command)]))),
  models: live.models.map((m) => item(m, 'implemented', refs([...quoted(m.id), ...quoted(m.command)]))),
  workspaces: live.workspaces.map(({ ready, ...w }) => item(w, ready ? 'implemented' : 'pending')),
  settings: live.settings.map((s) => item({ id: `${s.scope}.${s.key}`, ...s }, 'implemented')),
  storage: scanStorage(ROOT, join(WEB, 'src')).map((s) => item(s, 'implemented')),
  fileFields: scanFileFields(ROOT, join(WEB, 'src/contracts/generated'), join(ROOT, 'crates/shared/contracts/src')).map((f) => item(f, 'implemented')),
  screens: scanScreens(ROOT, join(WEB, 'src')).map((s) => item(s, 'implemented')),
};
for (const list of Object.values(sections)) list.sort(byId);

// ── Desktop column ───────────────────────────────────────────────────
// In this order, each over the one before: a whole section from the desktop's table
// (apps/desktop/equivalents.json `sections`); the commands the desktop shell runs
// (apps/desktop/ported.json, kept equal to its catalog::PORTED by a test; docs/adr/0017),
// and through them its tools, processing tools, models and work modes; the typed settings whose
// schema hosts the desktop; an item of the table (its desktop place or why it does not
// apply there); last a hand note (annotations.json, below). Anything else stays `none`.
const portedFile = join(ROOT, 'apps/desktop/ported.json');
const ported = new Set(existsSync(portedFile) ? JSON.parse(readFileSync(portedFile, 'utf8')).commands : []);
const unknownPorted = [...ported].filter((id) => !sections.commands.some((c) => c.id === id));
if (unknownPorted.length) {
  console.error(`apps/desktop/ported.json web'de olmayan komutlar içeriyor: ${unknownPorted.join(', ')}`);
  process.exit(1);
}
const equivalents = readEquivalents(join(ROOT, 'apps/desktop/equivalents.json'), sections, PLATFORM);
// A whole section's word goes on each item; its place and reason are said once, in the summary.
for (const [name, e] of Object.entries(equivalents.sections ?? {})) for (const i of sections[name]) i.platforms.desktop = e.desktop;
for (const c of sections.commands) if (ported.has(c.id)) c.platforms.desktop = 'implemented';
for (const t of sections.tools) if (ported.has(`tool.${t.id}`)) t.platforms.desktop = 'implemented';
for (const p of [...sections.processing, ...sections.models]) if (ported.has(p.command)) p.platforms.desktop = 'implemented';
for (const w of sections.workspaces) if (ported.has(`workspace.${w.id}`)) w.platforms.desktop = 'implemented';
// Typed settings the desktop uses too: the schema's hosts (docs/adr/0023).
const settingsSchema = JSON.parse(readFileSync(join(WEB, 'src/contracts/generated/settingsSchema.json'), 'utf8'));
const settingHosts = new Map(settingsSchema.settings.map((d) => [d.key, d.hosts]));
for (const s of sections.settings) if (s.setting && settingHosts.get(s.setting)?.includes('desktop')) s.platforms.desktop = 'implemented';
for (const [name, list] of Object.entries(sections)) for (const [id, e] of Object.entries(equivalents[name] ?? {})) desktopFrom(list.find((i) => i.id === id), e);

// ── Hand-written notes ───────────────────────────────────────────────
/** Section of an annotation key `section:id` → the inventory section. */
const KEYS = { command: 'commands', tool: 'tools', processing: 'processing', model: 'models', workspace: 'workspaces', setting: 'settings', storage: 'storage', fileField: 'fileFields', screen: 'screens' };
const annotations = JSON.parse(readFileSync(join(OUT, 'annotations.json'), 'utf8'));
const problems = [];
for (const [key, note] of Object.entries(annotations)) {
  if (key.startsWith('$')) continue;
  const at = key.indexOf(':');
  const target = sections[KEYS[key.slice(0, at)]]?.find((i) => i.id === key.slice(at + 1));
  if (!target) {
    problems.push(`${key}: envanterde böyle bir öğe yok (silindi ya da adı değişti; annotations.json'ı düzeltin)`);
    continue;
  }
  for (const field of Object.keys(note)) if (!['status', 'desktop', 'note', 'acceptance'].includes(field)) problems.push(`${key}: bilinmeyen alan “${field}” (status, desktop, note, acceptance)`);
  if (note.status && !STATUS.includes(note.status)) problems.push(`${key}: status ${STATUS.join(' | ')} olmalı`);
  if (note.desktop && !PLATFORM.includes(note.desktop)) problems.push(`${key}: desktop ${PLATFORM.join(' | ')} olmalı`);
  if (note.status) target.status = target.platforms.web = note.status;
  if (note.desktop) target.platforms.desktop = note.desktop;
  if (note.note) target.note = note.note;
  if (note.acceptance) target.acceptance = note.acceptance;
}
if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}

// ── Output ───────────────────────────────────────────────────────────
const count = (list) => Object.fromEntries([['total', list.length], ...STATUS.map((s) => [s, list.filter((i) => i.status === s).length])]);
const desktopCount = (list) => Object.fromEntries([['total', list.length], ...PLATFORM.map((p) => [p, list.filter((i) => i.platforms.desktop === p).length])]);
const commands = sections.commands;
const inventory = {
  format: 'kentos.inventory',
  version: 1,
  app: 'web',
  generated: 'pnpm inventory (apps/web/scripts/inventory): the running app and a scan of its sources; notes from docs/inventory/annotations.json. Do not edit by hand.',
  vocabulary: { status: STATUS, platform: PLATFORM },
  summary: {
    ...Object.fromEntries(Object.entries(sections).map(([name, list]) => [name, count(list)])),
    commandsWithoutPlace: commands.filter((c) => !c.menus.length && !c.ribbon.length && !c.quickAccess && !c.toolbox && !c.uiSources.length).map((c) => c.id),
    commandsWithoutTests: commands.filter((c) => !c.tests.length).length,
    commandsOnDesktop: commands.filter((c) => c.platforms.desktop === 'implemented').length,
    // Per section, how many items the desktop has, partly has, lacks, or has no use for.
    desktop: Object.fromEntries(Object.entries(sections).map(([name, list]) => [name, desktopCount(list)])),
    // Whole sections the desktop's table speaks for (apps/desktop/equivalents.json `sections`).
    desktopSections: equivalents.sections ?? {},
  },
  ...sections,
  // Menus and ribbon in the web's order (the desktop shell mirrors them, docs/adr/0017).
  layout: live.layout,
  // The web's icons, name → SVG markup of a 20×20 stroke icon (ui/icons.ts); the desktop draws them (docs/adr/0054).
  icons: live.icons,
};
const files = { 'web.json': `${JSON.stringify(inventory, null, 2)}\n`, 'web.md': summaryMarkdown(inventory) };

if (check) {
  const stale = Object.keys(files).filter((name) => !existsSync(join(OUT, name)) || readFileSync(join(OUT, name), 'utf8') !== files[name]);
  if (stale.length) {
    const old = existsSync(join(OUT, 'web.json')) ? JSON.parse(readFileSync(join(OUT, 'web.json'), 'utf8')) : {};
    const changed = Object.keys(sections).flatMap((name) => diffIds(name, old[name] ?? [], sections[name]));
    const shown = changed.length > 20 ? [...changed.slice(0, 20), '…'] : changed;
    console.error(`docs/inventory/${stale.join(', ')} güncel değil.${changed.length ? ` Değişen öğeler: ${shown.join(', ')}` : ''}`);
    console.error('Envanteri yenileyin: pnpm inventory; farkı okuyup commit edin.');
    process.exit(1);
  }
  console.log('Envanter güncel.');
} else {
  mkdirSync(OUT, { recursive: true });
  for (const [name, text] of Object.entries(files)) writeFileSync(join(OUT, name), text);
  const s = inventory.summary;
  console.log(Object.keys(sections).map((n) => `${n} ${s[n].total} (${STATUS.map((st) => `${st} ${s[n][st]}`).join(', ')})`).join('\n'));
  console.log('docs/inventory/web.json ve web.md yazıldı.');
}

/**
 * The desktop's table (apps/desktop/equivalents.json, kentos.desktop-equivalents v1): per
 * inventory section, items by their id → { desktop, where?, reason? }, and `sections`, one
 * such entry for a whole section. A missing file is an empty table. An id or section the
 * inventory does not have, an unknown word or field, or `n/a` without its reason stops the
 * script, so the table cannot go stale unseen.
 */
function readEquivalents(file, sections, platform) {
  if (!existsSync(file)) return {};
  const table = JSON.parse(readFileSync(file, 'utf8'));
  const problems = [];
  if (table.format !== 'kentos.desktop-equivalents' || table.version !== 1) problems.push('biçim kentos.desktop-equivalents, sürüm 1 olmalı');
  const entry = (where, e) => {
    if (!e || typeof e !== 'object') return problems.push(`${where}: bir nesne olmalı`);
    for (const f of Object.keys(e)) if (!['desktop', 'where', 'reason'].includes(f)) problems.push(`${where}: bilinmeyen alan “${f}” (desktop, where, reason)`);
    if (!platform.includes(e.desktop)) problems.push(`${where}: desktop ${platform.join(' | ')} olmalı`);
    if (e.desktop === 'n/a' && !e.reason) problems.push(`${where}: n/a nedeniyle (reason) yazılır`);
    for (const f of ['where', 'reason']) if (e[f] !== undefined && (typeof e[f] !== 'string' || !e[f].trim())) problems.push(`${where}: ${f} boş olmayan bir metin olmalı`);
  };
  for (const [key, value] of Object.entries(table)) {
    if (['format', 'version'].includes(key) || key.startsWith('$')) continue;
    if (key === 'sections') {
      for (const [name, e] of Object.entries(value)) {
        if (!sections[name]) problems.push(`sections.${name}: envanterde böyle bir bölüm yok`);
        entry(`sections.${name}`, e);
      }
      continue;
    }
    if (!sections[key]) {
      problems.push(`${key}: envanterde böyle bir bölüm yok (${Object.keys(sections).join(', ')}, sections)`);
      continue;
    }
    for (const [id, e] of Object.entries(value)) {
      if (!sections[key].some((i) => i.id === id)) problems.push(`${key}:${id}: envanterde böyle bir öğe yok (silindi ya da adı değişti; equivalents.json'ı düzeltin)`);
      entry(`${key}:${id}`, e);
    }
  }
  if (problems.length) {
    console.error(`apps/desktop/equivalents.json:\n${problems.join('\n')}`);
    process.exit(1);
  }
  return table;
}

/** An item's desktop column from a table entry: its word, where it is there, why. */
function desktopFrom(item, e) {
  if (!item) return;
  item.platforms.desktop = e.desktop;
  if (e.where) item.desktopWhere = e.where;
  else delete item.desktopWhere;
  if (e.reason) item.desktopNote = e.reason;
  else delete item.desktopNote;
}

/** Ids added, removed or changed in one section, for the --check report. */
function diffIds(section, before, after) {
  const was = new Map(before.map((i) => [i.id, JSON.stringify(i)]));
  const now = new Map(after.map((i) => [i.id, JSON.stringify(i)]));
  return [
    ...[...now.keys()].filter((id) => !was.has(id)).map((id) => `+${section}:${id}`),
    ...[...was.keys()].filter((id) => !now.has(id)).map((id) => `-${section}:${id}`),
    ...[...now.keys()].filter((id) => was.has(id) && was.get(id) !== now.get(id)).map((id) => `~${section}:${id}`),
  ];
}
