import { entityBounds, type Entity } from './entities';

/**
 * Çizimler arası alışveriş (docs/adr/0193; the desktop's `kentos_domain::exchange`): the rules of its three works over
 * drawings in the contract's form (`DocumentSnapshotV2`'s JSON), both platforms held to the shared cases
 * fixtures/exchange/v1/cases.json written by scripts/fixtures/exchange_cases.py.
 *
 * - `selectionDrawing`: the selected objects as a drawing of their own, with the layers, groups, blocks and library
 *   items they and their blocks use, the settings with the kept layers' states; links to objects left out drop.
 * - `takeFrom`: layers by path, text and dimension styles, library items by id (a symbol with its images), blocks by
 *   name (with those they place and the symbols their objects draw with), layer states by name, the project's units and
 *   scale, from another drawing; the same names skipped or replaced.
 * - `fileBlock`: another drawing's objects but pictures and tables as one block, its base the lower left of their
 *   extent; their layers by path (the missing made), their blocks and styles brought in under names this drawing has not.
 * - `layerTake`: another drawing's layer with its objects (Kaynaklar's Katman olarak ekle, docs/adr/0199 §7): the layer
 *   by path (a met one kept), the layers their blocks' objects are on, the blocks by name (a met name is ours), the
 *   styles by name (the missing added), the library items the objects and the made layers draw with; links, ties and a
 *   table's source dropped (the objects take new ids).
 *
 * The rules read and write the drawings as the contract's JSON, as the reference does: an object's style, link and tie
 * are its fields there.
 */

/** A value of the contract's JSON, read field by field as the reference reads it. */
export type Json = any;

export type Same = 'skip' | 'replace';

/** What Başka çizimden al takes: layers by their paths (“Kadastro / Bina”), blocks, styles and states by name, library items by id. */
export interface Picks {
  layers?: readonly string[];
  blocks?: readonly string[];
  textStyles?: readonly string[];
  dimensionStyles?: readonly string[];
  library?: readonly string[];
  layerStates?: readonly string[];
  settings?: boolean;
}

/** The project settings Başka çizimden al takes (never the coordinate systems); the annotation heights too (docs/adr/0205 §1). */
export const TAKEN_SETTINGS = ['lengthDecimals', 'areaDecimals', 'areaUnit', 'angleUnit', 'plotScale', 'drawingFont', 'drawingUnit', 'survey', 'annotation'] as const;

const FOLDED: Record<string, string> = { ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u', â: 'a', á: 'a', à: 'a', ä: 'a', î: 'i', í: 'i', ì: 'i', ï: 'i', û: 'u', ú: 'u', ù: 'u', ô: 'o', ó: 'o', ò: 'o', é: 'e', è: 'e', ê: 'e', ë: 'e' };

/** A name as names are compared: Turkish letters folded, case aside, trimmed (layers, styles and states). */
export function exchangeFold(name: string): string {
  const lower = [...name.trim()]
    .map((c) => (c === 'İ' ? 'i' : c === 'I' ? 'ı' : c))
    .join('')
    .toLowerCase();
  return [...lower].map((c) => FOLDED[c] ?? c).join('');
}

/** A block's name as blocks compare them (docs/adr/0144). */
export function blockKey(name: string): string {
  return [...name].map((c) => (c === 'I' ? 'ı' : c === 'İ' ? 'i' : c.toLowerCase())).join('');
}

/** Names for `incoming` that none of `taken` nor each other have (blocks' rule): `Ad (2)` … */
export function uniqueNames(taken: readonly string[], incoming: readonly string[]): string[] {
  const keys = new Set(taken.map(blockKey));
  return incoming.map((name) => {
    let chosen = name;
    for (let k = 2; keys.has(blockKey(chosen)); k++) chosen = `${name} (${k})`;
    keys.add(blockKey(chosen));
    return chosen;
  });
}

const clone = <T>(v: T): T => structuredClone(v);
const list = (v: Json, key: string): Json[] => (Array.isArray(v?.[key]) ? v[key] : []);
const listMut = (v: Json, key: string): Json[] => (Array.isArray(v[key]) ? v[key] : (v[key] = []));

function walk(nodes: readonly Json[], above: readonly string[] = [], out: [Json, string[]][] = []): [Json, string[]][] {
  for (const n of nodes) {
    const here = [...above, n.name as string];
    out.push([n, here]);
    walk(list(n, 'children'), here, out);
  }
  return out;
}

const allIds = (tree: readonly Json[]): Set<string> => new Set(walk(tree).map(([n]) => n.id as string));

function freeId(id: string, taken: Set<string>): string {
  let out = id;
  for (let k = 2; taken.has(out); k++) out = `${id}-${k}`;
  taken.add(out);
  return out;
}

function stringsAt(value: Json, key: string, out: Set<string>): Set<string> {
  if (Array.isArray(value)) for (const v of value) stringsAt(v, key, out);
  else if (value && typeof value === 'object')
    for (const [k, v] of Object.entries(value)) {
      if (k === key && typeof v === 'string') out.add(v);
      else stringsAt(v, key, out);
    }
  return out;
}

/** The definitions these objects place, and those they place in turn. */
function placedBlocks(entities: readonly Json[], byId: ReadonlyMap<string, Json>): Set<string> {
  const out = new Set<string>();
  const todo = entities.filter((e) => e.kind === 'insert').map((e) => e.block as string);
  while (todo.length) {
    const b = todo.pop()!;
    const def = byId.get(b);
    if (!def || out.has(b)) continue;
    out.add(b);
    for (const e of list(def, 'entities')) if (e.kind === 'insert') todo.push(e.block);
  }
  return out;
}

const blocksById = (doc: Json) => new Map<string, Json>(list(doc, 'blocks').map((b) => [b.id, b]));

// ── 1. The selection's drawing ────────────────────────────────────────

/** The selected objects (`uids`) as a drawing of their own named `name` (docs/adr/0193 §1). */
export function selectionDrawing(doc: Json, uids: readonly string[], name: string): Json {
  const keep = new Set(uids);
  const allUids: string[] = list(doc, 'uids');
  const keptUids = allUids.filter((u) => keep.has(u));
  const entities = list(doc, 'entities').filter((_, i) => keep.has(allUids[i]));
  const byId = blocksById(doc);
  const blocks = placedBlocks(entities, byId);
  const keptBlocks = list(doc, 'blocks').filter((b) => blocks.has(b.id));
  const usedLayers = new Set<string>([...entities, ...keptBlocks.flatMap((b) => list(b, 'entities'))].map((e) => e.layerId));
  const prune = (nodes: readonly Json[]): Json[] => {
    const out: Json[] = [];
    for (const n of nodes) {
      const kids = prune(list(n, 'children'));
      if ((n.type === 'layer' && usedLayers.has(n.id)) || (n.type !== 'layer' && kids.length)) out.push({ ...clone(n), children: kids });
    }
    return out;
  };
  const tree = prune(list(doc, 'layers'));
  const keptNodes = allIds(tree);
  const leaves = walk(tree)
    .filter(([n]) => n.type === 'layer')
    .map(([n]) => n.id as string);
  const active = leaves.includes(doc.activeLayer) ? doc.activeLayer : (leaves[0] ?? '');
  const out = entities.map((src, i) => {
    const e = { ...clone(src), id: i + 1 };
    if (e.kind === 'text' && e.labelOf !== undefined && !keep.has(e.labelOf)) {
      delete e.labelOf;
      delete e.labelScale;
    }
    if (e.kind === 'hatch' && e.assoc) {
      const followed = [e.assoc.outer, ...list(e.assoc, 'islands'), ...list(e.assoc, 'cutouts')];
      if (!followed.every((u) => keep.has(u))) delete e.assoc;
    }
    if (e.kind === 'table' && e.source && Array.isArray(e.source.objects)) {
      const objects = e.source.objects.filter((u: string) => keep.has(u));
      if (objects.length) e.source = { ...e.source, objects };
      else delete e.source;
    }
    return e;
  });
  // The library: what the kept objects, blocks' objects and layers use.
  const items = list(doc.styles, 'items');
  const symbols = new Set<string>();
  for (const e of [...out, ...keptBlocks.flatMap((b) => list(b, 'entities'))]) if (typeof e.symbol === 'string') symbols.add(e.symbol);
  const layerStyles = walk(tree).map(([n]) => n.style);
  for (const st of layerStyles) stringsAt(st, 'ref', symbols);
  const assets = new Set<string>();
  for (const e of out) if ((e.kind === 'image' || e.kind === 'raster') && typeof e.asset === 'string') assets.add(e.asset);
  for (const it of items) if (it.kind === 'symbol' && symbols.has(it.id)) stringsAt(it, 'asset', assets);
  for (const st of layerStyles) stringsAt(st, 'asset', assets);
  const keptItems = items.filter((it) => (it.kind === 'symbol' && symbols.has(it.id)) || (it.kind === 'asset' && assets.has(it.id))).map(clone);
  const settings = clone(doc.settings);
  const states = list(settings, 'layerStates')
    .map((st) => ({ ...st, nodes: list(st, 'nodes').filter((n) => keptNodes.has(n.node)) }))
    .filter((st) => st.nodes.length);
  if (states.length) settings.layerStates = states;
  else delete settings.layerStates;
  const result: Json = {
    format: doc.format,
    version: 2,
    name,
    settings,
    origin: clone(doc.origin),
    layers: tree,
    activeLayer: active,
    entities: out,
    uids: keptUids,
    styles: { items: keptItems, categories: clone(doc.styles.categories) },
  };
  if (keptBlocks.length) result.blocks = keptBlocks.map(clone);
  return result;
}

// ── 2. Taking from another drawing ────────────────────────────────────

function findPath(tree: readonly Json[], names: readonly string[]): Json | null {
  let nodes = tree;
  let found: Json | null = null;
  for (const name of names) {
    const hit = nodes.find((n) => exchangeFold(n.name) === exchangeFold(name));
    if (!hit) return null;
    found = hit;
    nodes = list(hit, 'children');
  }
  return found;
}

const pathOf = (tree: readonly Json[]) => new Map<string, string[]>(walk(tree).map(([n, here]) => [n.id, here]));

/** Our node at the path their node has: a layer only, unless `any` (a state names groups too). */
function ourNodeFor(ours: Json, theirPaths: ReadonlyMap<string, string[]>, their: string, any: boolean): string | null {
  const here = theirPaths.get(their);
  if (!here) return null;
  const hit = findPath(list(ours, 'layers'), here);
  return hit && (any || hit.type === 'layer') ? hit.id : null;
}

/** The layers of `theirs` at `paths` into `ours` (§2): missing paths made with their groups, a met layer's style replaced. */
function takeLayers(ours: Json, theirs: Json, paths: ReadonlySet<string>, same: Same, taken: Set<string>): void {
  const all = walk(list(theirs, 'layers'));
  const byPath = new Map(all.map(([n, here]) => [here.join('\u0000'), n]));
  for (const [n, here] of all) {
    if (n.type !== 'layer' || !paths.has(here.join(' / '))) continue;
    let nodes = listMut(ours, 'layers');
    for (let depth = 0; depth < here.length; depth++) {
      const names = here.slice(0, depth + 1);
      const source = byPath.get(names.join('\u0000'));
      const last = depth === here.length - 1;
      const at = nodes.findIndex((m) => exchangeFold(m.name) === exchangeFold(names[depth]));
      if (at < 0) {
        const made = { ...clone(source), id: freeId(source.id, taken), children: [] };
        // A scenario's links name the other drawing's layers, a filter's list its objects: they do not come; the time
        // setting and the filter's condition do (docs/adr/0210 §9, 0211 §2).
        delete made.scenario;
        delete made.replaces;
        if (made.filter) {
          if (made.filter.expression != null) made.filter = { expression: made.filter.expression };
          else delete made.filter;
        }
        nodes.push(made);
        nodes = made.children;
        continue;
      }
      if (nodes[at].type !== source.type) break;
      if (last && same === 'replace') nodes[at].style = clone(source.style);
      nodes = listMut(nodes[at], 'children');
    }
  }
}

function takeStyles(ours: Json, theirs: Json, key: string, names: ReadonlySet<string>, same: Same): void {
  const mine = listMut(ours.settings, key);
  for (const s of list(theirs.settings, key)) {
    if (!names.has(s.name)) continue;
    const at = mine.findIndex((m) => exchangeFold(m.name) === exchangeFold(s.name));
    if (at >= 0) {
      if (same === 'replace') mine[at] = { ...clone(s), id: mine[at].id };
      continue;
    }
    mine.push(mine.some((m) => m.id === s.id) ? { ...clone(s), id: crypto.randomUUID() } : clone(s));
  }
  if (!mine.length) delete ours.settings[key];
}

function takeItems(ours: Json, theirs: Json, ids: readonly string[], same: Same): void {
  const theirsItems = list(theirs.styles, 'items');
  const byId = new Map<string, Json>(theirsItems.map((it) => [it.id, it]));
  const wanted: string[] = [];
  for (const i of ids) {
    const it = byId.get(i);
    if (!it || wanted.includes(i)) continue;
    wanted.push(i);
    if (it.kind === 'symbol') for (const a of [...stringsAt(it, 'asset', new Set())].sort()) if (byId.has(a) && !wanted.includes(a)) wanted.push(a);
  }
  const order = theirsItems.map((it) => it.id as string);
  wanted.sort((a, b) => order.indexOf(a) - order.indexOf(b));
  const mine = listMut(ours.styles, 'items');
  for (const i of wanted) {
    const at = mine.findIndex((m) => m.id === i);
    if (at >= 0) {
      if (same === 'replace') mine[at] = clone(byId.get(i));
    } else mine.push(clone(byId.get(i)));
  }
}

/** Their text and dimension style ids → ours by name (null: we have no such style). */
function styleIds(ours: Json, theirs: Json): Map<string, string | null> {
  const out = new Map<string, string | null>();
  for (const key of ['textStyles', 'dimensionStyles'])
    for (const s of list(theirs.settings, key)) {
      const hit = list(ours.settings, key).find((m) => exchangeFold(m.name) === exchangeFold(s.name));
      out.set(s.id, hit ? hit.id : null);
    }
  return out;
}

/** One of their objects as one of ours: its layer, its block, its styles; no link, no tie (a block's object has no id). */
function mapObject(e: Json, layerOf: (id: string) => string, blockOf: ReadonlyMap<string, string>, styles: ReadonlyMap<string, string | null>): Json {
  const out = { ...clone(e), layerId: layerOf(e.layerId) };
  if (out.kind === 'insert' && blockOf.has(out.block)) out.block = blockOf.get(out.block);
  for (const field of ['textStyle', 'dimStyle'])
    if (typeof out[field] === 'string') {
      const to = styles.get(out[field]) ?? null;
      if (to === null) delete out[field];
      else out[field] = to;
    }
  delete out.labelOf;
  delete out.labelScale;
  delete out.assoc;
  return out;
}

function takeBlocks(ours: Json, theirs: Json, names: ReadonlySet<string>, same: Same, theirPaths: ReadonlyMap<string, string[]>): void {
  const byId = blocksById(theirs);
  const picked = list(theirs, 'blocks')
    .filter((b) => names.has(b.name))
    .map((b) => ({ kind: 'insert', block: b.id }));
  const wanted = placedBlocks(picked, byId);
  const theirsBlocks = list(theirs, 'blocks').filter((b) => wanted.has(b.id));
  const blockOf = new Map<string, string>();
  for (const b of theirsBlocks) {
    const hit = list(ours, 'blocks').find((m) => blockKey(m.name) === blockKey(b.name));
    blockOf.set(b.id, hit ? hit.id : b.id);
  }
  // The symbols their objects draw with, when we have none such.
  const have = new Set(list(ours.styles, 'items').map((it) => it.id as string));
  const symbols = theirsBlocks.flatMap((b) => list(b, 'entities')).flatMap((e) => (typeof e.symbol === 'string' && !have.has(e.symbol) ? [e.symbol as string] : []));
  takeItems(ours, theirs, symbols, 'skip');
  const styles = styleIds(ours, theirs);
  const snapshot = clone(ours);
  const layerOf = (i: string) => ourNodeFor(snapshot, theirPaths, i, false) ?? '';
  const mine = listMut(ours, 'blocks');
  for (const b of theirsBlocks) {
    const defined = { ...clone(b), id: blockOf.get(b.id), entities: list(b, 'entities').map((e) => mapObject(e, layerOf, blockOf, styles)) };
    const at = mine.findIndex((m) => blockKey(m.name) === blockKey(b.name));
    if (at >= 0) {
      if (same === 'replace') mine[at] = { ...defined, name: mine[at].name };
    } else mine.push(defined);
  }
  if (!mine.length) delete ours.blocks;
}

function takeStates(ours: Json, theirs: Json, names: ReadonlySet<string>, same: Same, theirPaths: ReadonlyMap<string, string[]>): void {
  const snapshot = clone(ours);
  const taken = new Set(list(ours.settings, 'layerStates').map((s) => s.id as string));
  const mine = listMut(ours.settings, 'layerStates');
  for (const s of list(theirs.settings, 'layerStates')) {
    if (!names.has(s.name)) continue;
    const nodes = list(s, 'nodes').flatMap((n) => {
      const to = ourNodeFor(snapshot, theirPaths, n.node, true);
      return to === null ? [] : [{ ...clone(n), node: to }];
    });
    const at = mine.findIndex((m) => exchangeFold(m.name) === exchangeFold(s.name));
    if (at >= 0) {
      if (same === 'replace') mine[at] = { ...mine[at], nodes };
    } else mine.push({ ...clone(s), id: freeId(s.id, taken), nodes });
  }
  if (!mine.length) delete ours.settings.layerStates;
}

/** What `picks` names of `theirs` taken into `ours` (docs/adr/0193 §2): a new drawing, `ours` as it was. */
export function takeFrom(oursIn: Json, theirs: Json, picks: Picks, same: Same): Json {
  const ours = clone(oursIn);
  if (picks.settings)
    for (const key of TAKEN_SETTINGS) {
      if (key in theirs.settings) ours.settings[key] = clone(theirs.settings[key]);
      else delete ours.settings[key];
    }
  const taken = allIds(list(ours, 'layers'));
  takeLayers(ours, theirs, new Set(picks.layers ?? []), same, taken);
  const theirPaths = pathOf(list(theirs, 'layers'));
  takeStyles(ours, theirs, 'textStyles', new Set(picks.textStyles ?? []), same);
  takeStyles(ours, theirs, 'dimensionStyles', new Set(picks.dimensionStyles ?? []), same);
  takeItems(ours, theirs, picks.library ?? [], same);
  takeBlocks(ours, theirs, new Set(picks.blocks ?? []), same, theirPaths);
  takeStates(ours, theirs, new Set(picks.layerStates ?? []), same, theirPaths);
  return ours;
}

// ── 3. A drawing as a block ───────────────────────────────────────────

/** The lower left of these objects' extent, each by its own box as the core measures it (an insert by its point); null for none. */
export function extentCorner(entities: readonly Json[]): [number, number] | null {
  let corner: [number, number] | null = null;
  for (const e of entities) {
    let b;
    try {
      b = entityBounds(e as Entity);
    } catch {
      continue;
    }
    if (!(Number.isFinite(b.minX) && Number.isFinite(b.minY))) continue;
    corner = corner ? [Math.min(corner[0], b.minX), Math.min(corner[1], b.minY)] : [b.minX, b.minY];
  }
  return corner;
}

/** `theirs` as one block of `ours` named after `file` (docs/adr/0193 §3): the new drawing with the block last (its id `$new`), and the pictures and tables left out; null when nothing is left. */
export function fileBlock(oursIn: Json, theirs: Json, file: string): { drawing: Json; images: number; tables: number } | null {
  const ours = clone(oursIn);
  const all = list(theirs, 'entities');
  // A block holds no picture, raster (docs/adr/0204 §9) or table: they are left out and counted with the pictures.
  const kept = all.filter((e) => e.kind !== 'image' && e.kind !== 'raster' && e.kind !== 'table');
  const corner = extentCorner(kept);
  if (!corner) return null;
  const theirPaths = pathOf(list(theirs, 'layers'));
  const byId = blocksById(theirs);
  const nested = placedBlocks(kept, byId);
  const nestedBlocks = list(theirs, 'blocks').filter((b) => nested.has(b.id));
  const pieces = [...kept, ...nestedBlocks.flatMap((b) => list(b, 'entities'))];
  const wanted = new Set<string>();
  for (const e of pieces) {
    const here = theirPaths.get(e.layerId);
    if (here) wanted.add(here.join(' / '));
  }
  takeLayers(ours, theirs, wanted, 'skip', allIds(list(ours, 'layers')));
  // Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
  const styles = new Map<string, string | null>();
  for (const [key, field] of [
    ['textStyles', 'textStyle'],
    ['dimensionStyles', 'dimStyle'],
  ] as const) {
    const used = new Set(pieces.flatMap((e) => (typeof e[field] === 'string' ? [e[field] as string] : [])));
    const mine = listMut(ours.settings, key);
    for (const s of list(theirs.settings, key)) {
      if (!used.has(s.id)) continue;
      const hit = mine.find((m) => exchangeFold(m.name) === exchangeFold(s.name));
      if (hit) styles.set(s.id, hit.id);
      else {
        const added = mine.some((m) => m.id === s.id) ? { ...clone(s), id: crypto.randomUUID() } : clone(s);
        styles.set(s.id, added.id);
        mine.push(added);
      }
    }
    if (!mine.length) delete ours.settings[key];
  }
  // The library items the objects draw with.
  const have = new Set(list(ours.styles, 'items').map((it) => it.id as string));
  takeItems(
    ours,
    theirs,
    pieces.flatMap((e) => (typeof e.symbol === 'string' && !have.has(e.symbol) ? [e.symbol as string] : [])),
    'skip',
  );
  // The blocks: the nested ones first (their order), then the drawing's own.
  const names = uniqueNames(
    list(ours, 'blocks').map((b) => b.name as string),
    [...nestedBlocks.map((b) => b.name as string), file],
  );
  const blockOf = new Map(nestedBlocks.map((b) => [b.id as string, b.id as string]));
  const snapshot = clone(ours);
  const layerOf = (i: string) => ourNodeFor(snapshot, theirPaths, i, false) ?? '';
  const mine = listMut(ours, 'blocks');
  nestedBlocks.forEach((b, k) => mine.push({ ...clone(b), name: names[k], entities: list(b, 'entities').map((e) => mapObject(e, layerOf, blockOf, styles)) }));
  mine.push({
    id: '$new',
    name: names[names.length - 1],
    base: { x: corner[0], y: corner[1] },
    entities: kept.map((e, i) => ({ ...mapObject(e, layerOf, blockOf, styles), id: i + 1 })),
  });
  return { drawing: ours, images: all.filter((e) => e.kind === 'image' || e.kind === 'raster').length, tables: all.filter((e) => e.kind === 'table').length };
}

// ── 4. A layer with its objects ───────────────────────────────────────

/**
 * Their layer at `path` with its objects into `ours` (docs/adr/0199 §7): the new drawing (its objects as they were) and
 * the objects to add, numbered from 1; null when `path` is not a layer of theirs or our node at it is not a layer.
 */
export function layerTake(oursIn: Json, theirs: Json, path: string): { drawing: Json; objects: Json[] } | null {
  const ours = clone(oursIn);
  const target = walk(list(theirs, 'layers')).find(([n, here]) => n.type === 'layer' && here.join(' / ') === path)?.[0];
  if (!target) return null;
  const objects = list(theirs, 'entities').filter((e) => e.layerId === target.id);
  const nested = placedBlocks(objects, blocksById(theirs));
  const nestedBlocks = list(theirs, 'blocks').filter((b) => nested.has(b.id));
  const pieces = [...objects, ...nestedBlocks.flatMap((b) => list(b, 'entities'))];
  const theirPaths = pathOf(list(theirs, 'layers'));
  const wanted = new Set([path]);
  for (const e of pieces) {
    const here = theirPaths.get(e.layerId);
    if (here) wanted.add(here.join(' / '));
  }
  const before = allIds(list(ours, 'layers'));
  takeLayers(ours, theirs, wanted, 'skip', new Set(before));
  if (ourNodeFor(ours, theirPaths, target.id, false) === null) return null;
  const madeStyles = walk(list(ours, 'layers'))
    .filter(([n]) => !before.has(n.id))
    .map(([n]) => n.style);
  // Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
  const styles = new Map<string, string | null>();
  for (const [key, field] of [
    ['textStyles', 'textStyle'],
    ['dimensionStyles', 'dimStyle'],
  ] as const) {
    const used = new Set(pieces.flatMap((e) => (typeof e[field] === 'string' ? [e[field] as string] : [])));
    const mine = listMut(ours.settings, key);
    for (const s of list(theirs.settings, key)) {
      if (!used.has(s.id)) continue;
      const hit = mine.find((m) => exchangeFold(m.name) === exchangeFold(s.name));
      if (hit) styles.set(s.id, hit.id);
      else {
        const added = mine.some((m) => m.id === s.id) ? { ...clone(s), id: crypto.randomUUID() } : clone(s);
        styles.set(s.id, added.id);
        mine.push(added);
      }
    }
    if (!mine.length) delete ours.settings[key];
  }
  // The library items the objects, their pictures and the made layers draw with, when we have none such.
  const have = new Set(list(ours.styles, 'items').map((it) => it.id as string));
  const items: string[] = [];
  for (const e of objects) {
    if (typeof e.symbol === 'string') items.push(e.symbol);
    if ((e.kind === 'image' || e.kind === 'raster') && typeof e.asset === 'string') items.push(e.asset);
  }
  for (const st of madeStyles) items.push(...[...stringsAt(st, 'ref', new Set())].sort(), ...[...stringsAt(st, 'asset', new Set())].sort());
  takeItems(
    ours,
    theirs,
    items.filter((i) => !have.has(i)),
    'skip',
  );
  // The blocks by name: a met one is ours, the others come with what they place.
  takeBlocks(ours, theirs, new Set(nestedBlocks.map((b) => b.name as string)), 'skip', theirPaths);
  const blockOf = new Map<string, string>(
    nestedBlocks.map((b) => [b.id as string, list(ours, 'blocks').find((m) => blockKey(m.name) === blockKey(b.name))?.id ?? b.id]),
  );
  const layerOf = (i: string) => ourNodeFor(ours, theirPaths, i, false) ?? '';
  const out = objects.map((e, i) => {
    const m = mapObject(e, layerOf, blockOf, styles);
    delete m.source;
    return { ...m, id: i + 1 };
  });
  return { drawing: ours, objects: out };
}
