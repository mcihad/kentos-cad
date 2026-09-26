import type { FeatureChange } from '../../contracts/generated/FeatureChange';
import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';
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
