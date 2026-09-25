// The part of the feature inventory (TODOS.md BASE-04) read from the source
// tree: windows and panels, browser storage, the `.kcad` v1 fields as the
// Rust contracts define them (through their generated TypeScript), and which
// tests mention an id. Plain text scans; the patterns are the conventions the
// code already follows (`open…` window functions, `persistedSignals`, ts-rs
// output), so something written another way is missed, not misread.
import { readdirSync, readFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';

/** `.ts` files under `dir` (tests left out unless asked), as absolute paths. */
function tsFiles(dir, { tests = false } = {}) {
  return readdirSync(dir, { recursive: true })
    .map(String)
    .filter((f) => f.endsWith('.ts') && tests === /\.test\.ts$/.test(f))
    .sort()
    .map((f) => join(dir, f));
}

/** Windows (`export function openX`) and panels (components the shell docks) under `src/ui`. */
export function scanScreens(root, src) {
  const out = [];
  for (const file of tsFiles(join(src, 'ui'))) {
    const text = readFileSync(file, 'utf8');
    const source = relative(root, file);
    for (const m of text.matchAll(/^export (?:async )?function (open[A-Z]\w*)\s*\(/gm)) out.push({ id: `${source}#${m[1]}`, name: m[1], role: 'window', source });
    for (const m of text.matchAll(/^export class (\w+) extends (?:Component|Panel)\b/gm)) if (m[1] !== 'Panel') out.push({ id: `${source}#${m[1]}`, name: m[1], role: 'panel', source });
  }
  return out;
}

/** localStorage keys (`persistedSignals('key', …)`) and IndexedDB stores (`const DB`/`STORE` beside `indexedDB.open`). */
export function scanStorage(root, src) {
  const out = new Map();
  for (const file of tsFiles(src)) {
    const text = readFileSync(file, 'utf8');
    const source = relative(root, file);
    for (const m of text.matchAll(/persistedSignals(?:<[^>]*>)?\('([^']+)'/g)) out.set(m[1], { id: m[1], kind: 'localStorage', source });
    if (text.includes('indexedDB.open(')) {
      const db = text.match(/^const DB = '([^']+)'/m)?.[1];
      const store = text.match(/^const STORE = '([^']+)'/m)?.[1];
      if (db && store) out.set(`${db}/${store}`, { id: `${db}/${store}`, kind: 'indexedDB', source });
    }
  }
  return [...out.values()].sort((a, b) => a.id.localeCompare(b.id));
}

/** Splits `text` at top-level commas (outside {}, <>, (), [] and strings). */
function splitTop(text) {
  const parts = [];
  let depth = 0;
  let quote = '';
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if ('{<(['.includes(c)) depth++;
    else if ('}>)]'.includes(c)) depth--;
    else if (c === ',' && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map((p) => p.trim()).filter(Boolean);
}

/** One generated contract: an object's fields, a tagged union's variants, or a union of literals. */
function parseContract(text) {
  const body = text
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\/\/.*$/gm, '')
    .replace(/^import .*$/gm, '')
    .match(/export type \w+ = ([\s\S]*);\s*$/)?.[1]
    .replace(/\s+/g, ' ')
    .trim();
  if (!body) return null;
  if (body.startsWith('{') && body.endsWith('}') && !body.includes('} &')) {
    const fields = splitTop(body.slice(1, -1)).map((f) => {
      const m = f.match(/^"?([\w$]+)"?(\??): (.*)$/);
      return m && { field: m[1], optional: m[2] === '?', type: m[3] };
    });
    return { shape: 'object', fields: fields.filter(Boolean), text: body };
  }
  const variants = [...body.matchAll(/\{ "(\w+)": "([^"]+)" \} & (\w+)/g)].map((m) => ({ tag: m[1], value: m[2], type: m[3] }));
  if (variants.length) return { shape: 'tagged', variants, text: body };
  const literals = body.split('|').map((s) => s.trim());
  if (literals.every((s) => /^"[^"]*"$/.test(s))) return { shape: 'enum', values: literals.map((s) => s.slice(1, -1)), text: body };
  return { shape: 'alias', text: body };
}

/**
 * The `.kcad` v1 fields: every contract reachable from `DocumentSnapshotV1`
 * in the ts-rs output, with the Rust file that defines it.
 */
export function scanFileFields(root, generatedDir, rustDir) {
  const contracts = new Map();
  for (const f of readdirSync(generatedDir).filter((f) => f.endsWith('.ts'))) {
    const c = parseContract(readFileSync(join(generatedDir, f), 'utf8'));
    if (c) contracts.set(basename(f, '.ts'), c);
  }
  const rust = new Map();
  for (const f of readdirSync(rustDir).filter((f) => f.endsWith('.rs'))) {
    const text = readFileSync(join(rustDir, f), 'utf8');
    for (const m of text.matchAll(/pub (?:struct|enum|type) (\w+)/g)) if (!rust.has(m[1])) rust.set(m[1], relative(root, join(rustDir, f)));
  }
  const seen = [];
  const queue = ['DocumentSnapshotV1'];
  while (queue.length) {
    const name = queue.shift();
    if (seen.includes(name) || !contracts.has(name)) continue;
    seen.push(name);
    for (const ref of contracts.get(name).text.match(/\b[A-Z]\w*\b/g) ?? []) if (contracts.has(ref) && !seen.includes(ref)) queue.push(ref);
  }
  const out = [];
  for (const name of seen) {
    const c = contracts.get(name);
    const base = { contract: name, rust: rust.get(name) };
    if (c.shape === 'object') for (const f of c.fields) out.push({ id: `${name}.${f.field}`, ...base, field: f.field, type: f.type, optional: f.optional });
    else if (c.shape === 'tagged') out.push({ id: name, ...base, variants: c.variants.map((v) => ({ [v.tag]: v.value, type: v.type })) });
    else if (c.shape === 'enum') out.push({ id: name, ...base, values: c.values });
    else out.push({ id: name, ...base, type: c.text });
  }
  return out;
}

/** Returns `refs(needles)`: the files (repository paths) whose text contains any needle. */
function textIndex(root, paths) {
  const files = paths.map((f) => ({ path: relative(root, f), text: readFileSync(f, 'utf8') }));
  return (needles) => files.filter((f) => needles.some((n) => f.text.includes(n))).map((f) => f.path);
}

/** Test files and e2e scripts, for looking up which of them mention an id. */
export function testIndex(root, web) {
  const e2e = join(web, 'scripts', 'e2e');
  return textIndex(root, [
    ...tsFiles(join(web, 'src'), { tests: true }),
    ...tsFiles(join(web, 'scripts'), { tests: true }),
    ...readdirSync(e2e)
      .filter((f) => f.endsWith('.mjs') && f !== 'cdp.mjs')
      .sort()
      .map((f) => join(e2e, f)),
  ]);
}

/**
 * Interface sources (`src/ui`, tests left out): a command id quoted there is
 * a button, menu item or link of a panel, the status bar or a window.
 */
export const uiIndex = (root, web) => textIndex(root, tsFiles(join(web, 'src', 'ui')));

/** The ways source code quotes an id: 'x', "x" and `x`. */
export const quoted = (id) => [`'${id}'`, `"${id}"`, `\`${id}\``];
