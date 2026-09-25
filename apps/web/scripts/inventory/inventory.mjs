// Feature inventory of the web app (TODOS.md BASE-04): commands, tools,
// processing tools and models, work modes, settings, browser storage, the
// `.kcad` v1 fields, windows and panels. Each item has a status
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
  },
  ...sections,
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
