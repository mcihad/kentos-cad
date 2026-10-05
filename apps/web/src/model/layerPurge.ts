import type { LayerNode } from './layers';

/**
 * Kullanılmayanları temizle's rule (docs/adr/0177 §5; the desktop's `kentos_interaction::layer_purge`, shared cases
 * fixtures/layers/v1/purge.json from scripts/fixtures/layer_purge_cases.py). What may go: the layers (not the active
 * one, not locked with their groups' locks), the groups (not locked), the block definitions, the project library's
 * symbols and assets. Something stays when anything that stays uses it: a layer an object of the drawing or of a staying
 * definition is on; a group a staying node is under; a definition an insert of the drawing or of a staying definition
 * places; a symbol an object draws with, a staying layer's style names (`ref`) or a template of any library names
 * (`symbol`); an asset a staying symbol of any library or a staying layer's style draws with (`asset`). What is found is
 * the largest set nothing staying uses; removing the checked ones takes the same rule over them alone.
 */

/** An object as the rule reads it: its layer, the library symbol it draws with, the definition it places. */
export interface PurgeObject {
  readonly layer: string;
  readonly symbol?: string;
  readonly block?: string;
}

/** A block definition and its objects. */
export interface PurgeBlock {
  readonly id: string;
  readonly name: string;
  readonly objects: readonly PurgeObject[];
}

/** A library item of any source; a symbol's and a template's JSON are read for what they name. */
export interface PurgeItem {
  readonly id: string;
  readonly kind: string;
  readonly source: string;
  readonly name: string;
  readonly symbol?: unknown;
  readonly template?: unknown;
}

/** What the rule reads of a drawing and its library. */
export interface PurgeSource {
  readonly tree: readonly LayerNode[];
  readonly active: string;
  readonly objects: readonly PurgeObject[];
  readonly blocks: readonly PurgeBlock[];
  readonly library: readonly PurgeItem[];
}

export const PURGE_KINDS = ['layers', 'groups', 'blocks', 'symbols', 'assets'] as const;
export type PurgeKind = (typeof PURGE_KINDS)[number];

/** What the window lists: each kind in its order; a locked layer to be unlocked first. */
export interface PurgeFound {
  readonly layers: readonly { readonly id: string; readonly path: string; readonly locked: boolean }[];
  readonly groups: readonly { readonly id: string; readonly path: string }[];
  readonly blocks: readonly { readonly id: string; readonly name: string }[];
  readonly symbols: readonly { readonly id: string; readonly name: string }[];
  readonly assets: readonly { readonly id: string; readonly name: string }[];
}

export type PurgeIds = Readonly<Record<PurgeKind, readonly string[]>>;

const key = (kind: PurgeKind, id: string) => `${kind}\u0000${id}`;

/** Every string under `name` anywhere in a JSON value. */
function stringsAt(value: unknown, name: string, out: Set<string>): Set<string> {
  if (Array.isArray(value)) for (const v of value) stringsAt(v, name, out);
  else if (value && typeof value === 'object')
    for (const [k, v] of Object.entries(value)) {
      if (k === name && typeof v === 'string') out.add(v);
      else stringsAt(v, name, out);
    }
  return out;
}

function* walk(nodes: readonly LayerNode[], parent: LayerNode | null = null): Generator<[LayerNode, LayerNode | null]> {
  for (const n of nodes) {
    yield [n, parent];
    yield* walk(n.children, n);
  }
}

/** Each node's path (“Kadastro / Bina”) and whether it is locked, its own lock or a group's above it. */
function treeFacts(tree: readonly LayerNode[]): { paths: Map<string, string>; locked: Set<string> } {
  const paths = new Map<string, string>();
  const locked = new Set<string>();
  const go = (nodes: readonly LayerNode[], above: string[], lockedAbove: boolean) => {
    for (const n of nodes) {
      const path = [...above, n.name];
      paths.set(n.id, path.join(' / '));
      const here = lockedAbove || n.locked;
      if (here) locked.add(n.id);
      go(n.children, path, here);
    }
  };
  go(tree, [], false);
  return { paths, locked };
}

function removable(src: PurgeSource, locked: Set<string>): Set<string> {
  const out = new Set<string>();
  for (const [n] of walk(src.tree)) {
    if (locked.has(n.id)) continue;
    if (n.type === 'layer' && n.id !== src.active) out.add(key('layers', n.id));
    if (n.type === 'group') out.add(key('groups', n.id));
  }
  for (const b of src.blocks) out.add(key('blocks', b.id));
  for (const it of src.library)
    if (it.source === 'project' && (it.kind === 'symbol' || it.kind === 'asset')) out.add(key(it.kind === 'symbol' ? 'symbols' : 'assets', it.id));
  return out;
}

function usedByStaying(src: PurgeSource, going: Set<string>): Set<string> {
  const used = new Set<string>();
  const objects = (list: readonly PurgeObject[]) => {
    for (const o of list) {
      used.add(key('layers', o.layer));
      if (o.symbol) used.add(key('symbols', o.symbol));
      if (o.block) used.add(key('blocks', o.block));
    }
  };
  objects(src.objects);
  for (const b of src.blocks) if (!going.has(key('blocks', b.id))) objects(b.objects);
  for (const [n, parent] of walk(src.tree)) {
    if (going.has(key(n.type === 'layer' ? 'layers' : 'groups', n.id))) continue;
    if (parent) used.add(key('groups', parent.id));
    if (n.type === 'layer') {
      for (const ref of stringsAt(n.style.renderer, 'ref', new Set())) used.add(key('symbols', ref));
      for (const asset of stringsAt(n.style.renderer, 'asset', new Set())) used.add(key('assets', asset));
    }
  }
  for (const it of src.library) {
    if (it.kind === 'template') for (const ref of stringsAt(it.template, 'symbol', new Set())) used.add(key('symbols', ref));
    if (it.kind === 'symbol' && !going.has(key('symbols', it.id))) for (const asset of stringsAt(it.symbol, 'asset', new Set())) used.add(key('assets', asset));
  }
  return used;
}

/** The largest part of `going` nothing staying uses: what something staying uses is given back, round after round. */
function settle(src: PurgeSource, going: Set<string>): Set<string> {
  for (;;) {
    const used = usedByStaying(src, going);
    const kept = [...going].filter((k) => used.has(k));
    if (!kept.length) return going;
    for (const k of kept) going.delete(k);
  }
}

/** What Kullanılmayanları temizle lists for the drawing. */
export function purgeFound(src: PurgeSource): PurgeFound {
  const { paths, locked } = treeFacts(src.tree);
  const going = settle(src, removable(src, locked));
  const onObjects = new Set(src.objects.map((o) => o.layer));
  const onBlocks = new Set(src.blocks.filter((b) => !going.has(key('blocks', b.id))).flatMap((b) => b.objects.map((o) => o.layer)));
  const layers: { id: string; path: string; locked: boolean }[] = [];
  const groups: { id: string; path: string }[] = [];
  for (const [n] of walk(src.tree)) {
    const path = paths.get(n.id)!;
    if (n.type === 'group') {
      if (going.has(key('groups', n.id))) groups.push({ id: n.id, path });
    } else if (going.has(key('layers', n.id))) layers.push({ id: n.id, path, locked: false });
    else if (locked.has(n.id) && n.id !== src.active && !onObjects.has(n.id) && !onBlocks.has(n.id)) layers.push({ id: n.id, path, locked: true });
  }
  const project = src.library.filter((it) => it.source === 'project');
  return {
    layers,
    groups,
    blocks: src.blocks.filter((b) => going.has(key('blocks', b.id))).map((b) => ({ id: b.id, name: b.name })),
    symbols: project.filter((it) => it.kind === 'symbol' && going.has(key('symbols', it.id))).map((it) => ({ id: it.id, name: it.name })),
    assets: project.filter((it) => it.kind === 'asset' && going.has(key('assets', it.id))).map((it) => ({ id: it.id, name: it.name })),
  };
}

/**
 * What removing the checked ones takes, in the order it goes (definitions in rounds, one placed in another after it;
 * groups deepest first, each empty when it goes), and how many checked ones stay because something staying uses them.
 */
export function purgeRemoved(src: PurgeSource, checked: Partial<PurgeIds>): { removed: PurgeIds; kept: number } {
  const { locked } = treeFacts(src.tree);
  const can = removable(src, locked);
  const asked = new Set(PURGE_KINDS.flatMap((k) => (checked[k] ?? []).map((id) => key(k, id))));
  const going = settle(src, new Set([...asked].filter((k) => can.has(k))));
  const has = (kind: PurgeKind, id: string) => going.has(key(kind, id));
  const groups: string[] = [];
  const deepest = (nodes: readonly LayerNode[]) => {
    for (const n of nodes) {
      deepest(n.children);
      if (n.type === 'group' && has('groups', n.id)) groups.push(n.id);
    }
  };
  deepest(src.tree);
  const blocks: string[] = [];
  let left = src.blocks.filter((b) => has('blocks', b.id));
  while (left.length) {
    const inside = new Set(left.flatMap((b) => b.objects.flatMap((o) => (o.block ? [o.block] : []))));
    const now = left.filter((b) => !inside.has(b.id)).map((b) => b.id);
    if (!now.length) break;
    blocks.push(...now);
    left = left.filter((b) => !now.includes(b.id));
  }
  const project = src.library.filter((it) => it.source === 'project');
  return {
    removed: {
      layers: [...walk(src.tree)].flatMap(([n]) => (n.type === 'layer' && has('layers', n.id) ? [n.id] : [])),
      groups,
      blocks,
      symbols: project.flatMap((it) => (it.kind === 'symbol' && has('symbols', it.id) ? [it.id] : [])),
      assets: project.flatMap((it) => (it.kind === 'asset' && has('assets', it.id) ? [it.id] : [])),
    },
    kept: asked.size - going.size,
  };
}
