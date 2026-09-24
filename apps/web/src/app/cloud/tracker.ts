import type { FeatureChange } from '../../contracts/generated/FeatureChange';
import type { ProjectPatch } from '../../contracts/generated/ProjectPatch';
import type { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';

/**
 * What the server has of each object, and what differs locally. The
 * browser numbers objects itself; the server knows them by UUID and row
 * version. For every local object the tracker keeps its server id, the
 * version it last saw and the object's text at that version, so a change is
 * found by comparison, not by replaying edits: an undo back to the saved
 * state sends nothing. A deleted object keeps its server id, so an undo that
 * brings it back creates it under the same id.
 */

export interface Tracked {
  featureId: string;
  /** The server's version, or null when the server does not have it (new, or deleted). */
  version: string | null;
  /** The object's text at that version (without the local id). */
  json: string | null;
}

export type Planned =
  | { op: 'create'; localId: number; featureId: string; entity: Entity; json: string }
  | { op: 'update'; localId: number; featureId: string; entity: Entity; json: string; expected: string }
  | { op: 'delete'; localId: number; featureId: string; expected: string };

/** An object's text for comparison: everything but the local id. */
export function entityJson(e: Entity): string {
  const { id: _id, ...rest } = e;
  return JSON.stringify(rest);
}

export function changeOf(p: Planned): FeatureChange {
  if (p.op === 'delete') return { op: 'delete', id: p.featureId };
  return { op: p.op, id: p.featureId, entity: p.entity };
}

export class Tracker {
  private readonly byLocal = new Map<number, Tracked>();
  private readonly byFeature = new Map<string, number>();
  private readonly newId: () => string;

  constructor(newId: () => string) {
    this.newId = newId;
  }

  get(localId: number): Tracked | undefined {
    return this.byLocal.get(localId);
  }

  localOf(featureId: string): number | undefined {
    return this.byFeature.get(featureId);
  }

  set(localId: number, t: Tracked): void {
    const old = this.byLocal.get(localId);
    if (old && old.featureId !== t.featureId) this.byFeature.delete(old.featureId);
    this.byLocal.set(localId, t);
    this.byFeature.set(t.featureId, localId);
  }

  /** What `localId` needs so the server matches the drawing (null: nothing). */
  plan(doc: CadDocument, localId: number): Planned | null {
    const cur = doc.get(localId);
    let t = this.byLocal.get(localId);
    if (cur) {
      const json = entityJson(cur);
      if (!t) {
        t = { featureId: this.newId(), version: null, json: null };
        this.set(localId, t);
      }
      if (t.version === null) return { op: 'create', localId, featureId: t.featureId, entity: cur, json };
      if (t.json !== json) return { op: 'update', localId, featureId: t.featureId, entity: cur, json, expected: t.version };
      return null;
    }
    if (t && t.version !== null) return { op: 'delete', localId, featureId: t.featureId, expected: t.version };
    return null;
  }

  /** The server accepted `p` at `version`. */
  acknowledge(p: Planned, version: string | undefined): void {
    if (p.op === 'delete') this.set(p.localId, { featureId: p.featureId, version: null, json: null });
    else this.set(p.localId, { featureId: p.featureId, version: version ?? null, json: p.json });
  }

  /** Whether the drawing still holds exactly what `p` sent (then nothing is left to send for it). */
  settled(doc: CadDocument, p: Planned): boolean {
    const cur = doc.get(p.localId);
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
