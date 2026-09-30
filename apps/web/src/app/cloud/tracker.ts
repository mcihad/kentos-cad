import type { BlockChange } from '../../contracts/generated/BlockChange';
import type { FeatureChange } from '../../contracts/generated/FeatureChange';
import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';
import { checkNesting, type BlockDefinition } from '../../model/blocks';
import type { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';

/**
 * What the server has of each object, and what differs locally. An object's
 * persistent id (`uid`, docs/adr/0014) is its id on the server too: one
 * identity from the drawing to the server and back, no second mapping
 * (ADR 0014 slice 3, docs/adr/0026). For every object the server has, the
 * tracker keeps the version it last saw and the object's text at that
 * version, so a change is found by comparison, not by replaying edits: an
 * undo back to the saved state sends nothing. An object the server does not
 * have is not tracked; one brought back by undo is created again under the
 * same id.
 */

export interface Tracked {
  /** The server's version of the object. */
  version: string;
  /** The object's text at that version (without its slot and persistent id). */
  json: string;
}

/** What one object needs so the server matches the drawing; `id` is its persistent id, the server's id. */
export type Planned =
  | { op: 'create'; id: string; entity: Entity; json: string }
  | { op: 'update'; id: string; entity: Entity; json: string; expected: string }
  | { op: 'delete'; id: string; expected: string };

/**
 * An object's text for comparison: everything but its ids. The persistent id
 * names it on the server (`FeatureChange.id`): it is the key, not content.
 */
export function entityJson(e: Entity): string {
  const { id: _id, uid: _uid, ...rest } = e;
  return JSON.stringify(rest);
}

export function changeOf(p: Planned): FeatureChange {
  if (p.op === 'delete') return { op: 'delete', id: p.id };
  return { op: p.op, id: p.id, entity: p.entity };
}

/**
 * An object as the server and the device draft take it: the contract's
 * shape (the contract's `Entity` carries no persistent id before ADR 0014
 * slice 4); the id goes beside it, as the change's or the draft entry's key.
 */
export function wire(e: Entity): Entity {
  const { uid: _uid, ...rest } = e;
  return rest;
}

export class Tracker {
  private readonly known = new Map<string, Tracked>();

  /** What the server has of the object with this persistent id, if anything. */
  get(id: string): Tracked | undefined {
    return this.known.get(id);
  }

  /** The server has the object at `version`, as `json`; null: the server does not have it. */
  set(id: string, t: Tracked | null): void {
    if (t) this.known.set(id, t);
    else this.known.delete(id);
  }

  /** What the object with persistent id `id` needs so the server matches the drawing (null: nothing). */
  plan(doc: CadDocument, id: string): Planned | null {
    const cur = doc.byUid(id);
    const t = this.known.get(id);
    if (cur) {
      const json = entityJson(cur);
      if (!t) return { op: 'create', id, entity: wire(cur), json };
      if (t.json !== json) return { op: 'update', id, entity: wire(cur), json, expected: t.version };
      return null;
    }
    return t ? { op: 'delete', id, expected: t.version } : null;
  }

  /** The server accepted `p` at `version` (a delete: it no longer has the object). */
  acknowledge(p: Planned, version: string | undefined): void {
    this.set(p.id, p.op === 'delete' || version === undefined ? null : { version, json: p.json });
  }

  /** Whether the drawing still holds exactly what `p` sent (then nothing is left to send for it). */
  settled(doc: CadDocument, p: Planned): boolean {
    const cur = doc.byUid(p.id);
    return p.op === 'delete' ? !cur : !!cur && entityJson(cur) === p.json;
  }
}

/**
 * The key a block definition goes under in `expectedVersions`, a commit's
 * `versions` and `deleted`, and a conflict (docs/adr/0144 §5; the
 * contracts' `block_key`): objects go under their ids, the metadata under
 * `@project`.
 */
export const blockKey = (id: string): string => `block:${id}`;

/** The definition a conflict's or a commit's key names; null for an object's or the metadata's. */
export const blockOfKey = (key: string): string | null => (key.startsWith('block:') ? key.slice(6) : null);

/** A definition's text for comparison: the whole of it (its id is part of what the server keeps). */
export const blockJson = (b: BlockDefinition): string => JSON.stringify(b);

/** What one definition needs so the server matches the drawing. */
export type PlannedBlock =
  | { op: 'create'; id: string; block: BlockDefinition; json: string }
  | { op: 'update'; id: string; block: BlockDefinition; json: string; expected: string }
  | { op: 'delete'; id: string; expected: string };

/** A planned change as a command carries it: a copy, so the command sent again after a lost answer is the same. */
export function blockChangeOf(p: PlannedBlock): BlockChange {
  if (p.op === 'delete') return { op: 'delete', id: p.id };
  return { op: p.op, block: structuredClone(p.block) as unknown as Extract<BlockChange, { op: 'create' }>['block'] };
}

/**
 * What the server has of each block definition (docs/adr/0144 §5), as the
 * `Tracker` keeps it of objects: the version it last saw and the
 * definition's text then. The changes are found by comparing the drawing's
 * list with it, so an undo back to the saved state sends nothing.
 */
export class BlockTracker {
  private readonly known = new Map<string, Tracked>();

  get(id: string): Tracked | undefined {
    return this.known.get(id);
  }

  /** The server has the definition at `version`, as `json`; null: the server does not have it. */
  set(id: string, t: Tracked | null): void {
    if (t) this.known.set(id, t);
    else this.known.delete(id);
  }

  /** Every definition the server has, by id. */
  ids(): IterableIterator<string> {
    return this.known.keys();
  }

  /** Whether the drawing's definition with this id (or its absence) differs from what the server has. */
  differs(doc: CadDocument, id: string): boolean {
    const b = doc.block(id);
    return (b ? blockJson(b) : null) !== (this.known.get(id)?.json ?? null);
  }

  /** What the server needs so its definitions match the drawing's: made and changed ones in the drawing's order, then removed ones. */
  plan(doc: CadDocument): PlannedBlock[] {
    const out: PlannedBlock[] = [];
    const here = new Set<string>();
    for (const block of doc.blocks.value) {
      here.add(block.id);
      const json = blockJson(block);
      const t = this.known.get(block.id);
      if (!t) out.push({ op: 'create', id: block.id, block, json });
      else if (t.json !== json) out.push({ op: 'update', id: block.id, block, json, expected: t.version });
    }
    for (const [id, t] of this.known) if (!here.has(id)) out.push({ op: 'delete', id, expected: t.version });
    return out;
  }

  /** The server accepted `p` at `version` (a delete: it no longer has the definition). */
  acknowledge(p: PlannedBlock, version: string | undefined): void {
    this.set(p.id, p.op === 'delete' || version === undefined ? null : { version, json: p.json });
  }

  /**
   * `plan`'s changes in the order the server takes them a part at a time
   * (docs/adr/0144 §5): made and changed definitions inner first, each after
   * the ones it places, so every insert inside names one the server has or
   * gets in the same command; removed ones outer first, so none is removed
   * while a definition still there places it. Within a nesting depth, the
   * drawing's order.
   */
  ordered(doc: CadDocument, planned: readonly PlannedBlock[]): { upserts: PlannedBlock[]; deletes: PlannedBlock[] } {
    const upserts = planned.filter((p) => p.op !== 'delete');
    const deletes = planned.filter((p) => p.op === 'delete');
    // The drawing's definitions and the removed ones as the server last had them.
    const gone = deletes.flatMap((p) => {
      try {
        return [JSON.parse(this.known.get(p.id)?.json ?? '') as BlockDefinition];
      } catch {
        return [];
      }
    });
    const list = [...doc.blocks.value, ...gone];
    const index = new Map(list.map((b, i) => [b.id, i]));
    const nesting = checkNesting(list, index);
    if ('fault' in nesting) return { upserts, deletes };
    const depth = (p: PlannedBlock) => nesting.depth[index.get(p.id) ?? 0] ?? 0;
    return { upserts: [...upserts].sort((a, b) => depth(a) - depth(b)), deletes: [...deletes].sort((a, b) => depth(b) - depth(a)) };
  }
}

/** The project's shared metadata as comparable text (the active layer is each user's own). */
export interface MetaParts {
  name: string;
  settings: string;
  layers: string;
  styles: string;
}

export function metaParts(doc: CadDocument): MetaParts {
  return {
    name: doc.name.value,
    settings: JSON.stringify(doc.settings.toJSON()),
    layers: JSON.stringify(doc.layers.tree),
    styles: JSON.stringify(doc.styles.value),
  };
}

type TreeNode = { id: string; children: readonly unknown[] };

/** Every node id of a layer tree. */
function nodeIds(nodes: readonly TreeNode[], out = new Set<string>()): Set<string> {
  for (const n of nodes) {
    out.add(n.id);
    nodeIds(n.children as TreeNode[], out);
  }
  return out;
}

/** The layer an object's text (`Tracked.json`) puts it on; undefined when the text says nothing readable. */
function layerOf(json: string): string | undefined {
  try {
    return (JSON.parse(json) as { layerId?: unknown }).layerId as string | undefined;
  } catch {
    return undefined;
  }
}

/**
 * When `tree` removes nodes the sent tree had (`base`, as `metaParts` wrote
 * it): the objects of `dirty` whose change the server must have before or
 * with that tree. Its guard refuses a tree while an object it holds on a
 * removed layer is neither deleted nor moved off by the same command. They
 * are every delete (a delete never needs the new tree) and every change of
 * an object the server holds on a removed layer (or where, it cannot be
 * read). Null when the tree removes nothing.
 */
export function leavingObjects(doc: CadDocument, tracker: Tracker, dirty: Iterable<string>, base: string, tree: readonly TreeNode[]): Set<string> | null {
  const kept = nodeIds(tree);
  const dropped = [...nodeIds(JSON.parse(base) as TreeNode[])].filter((id) => !kept.has(id));
  if (!dropped.length) return null;
  const removed = new Set(dropped);
  const out = new Set<string>();
  for (const id of dirty) {
    const t = tracker.get(id);
    // Not on the server: a create, which may need the new tree.
    if (!t) continue;
    if (!doc.byUid(id)) out.add(id);
    else {
      const layer = layerOf(t.json);
      if (layer === undefined || removed.has(layer)) out.add(id);
    }
  }
  return out;
}

/** The metadata that differs from `base`, or null. A new layer tree carries the active layer, which must exist in it. */
export function metaPatch(doc: CadDocument, base: MetaParts): ProjectPatch | null {
  const cur = metaParts(doc);
  const patch: ProjectPatch = {};
  if (cur.name !== base.name) patch.name = doc.name.value;
  if (cur.settings !== base.settings) patch.settings = doc.settings.toJSON();
  if (cur.layers !== base.layers) {
    patch.layers = structuredClone([...doc.layers.tree]);
    patch.activeLayer = doc.layers.active.value;
  }
  if (cur.styles !== base.styles) patch.styles = { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) };
  return Object.keys(patch).length ? patch : null;
}
