import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityGeometry, type Entity, type EntityGeometry } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { nearestS, pathOf, pointAtS } from '../model/ops/path';
import { fenceCrossings } from '../model/ops/trim';
import { withGeometry } from '../model/ops/transform';
import type { Geometry } from '../wasm/pack';
import { editGeometry, uidOf } from './editCommand';

/**
 * The Çit (fence) method of Buda and Uzat (docs/adr/0140): a fence of clicked points;
 * every object it crosses is trimmed where it crosses (Buda) or extended at the end
 * nearest the crossing (Uzat), as a click on the crossing would, all as one edit. The
 * crossings are the shared core's (`fenceCrossings`, in the fence's order); each pick
 * then goes through the same trim or extend computation a click uses
 * (`view.trim`, `view.extend`), and an object an earlier pick changed is worked on in
 * its new geometry (its pieces, for a trim).
 */

export type FenceAction = 'trim' | 'extend';

export interface FencePlan {
  /** Buda: each object crossed with the pieces of it that remain (none: all of it goes). */
  trims: { entity: Entity; pieces: Entity[] }[];
  /** Uzat: each object with its new geometry. */
  extends: { entity: Entity; geometry: EntityGeometry }[];
  /** Where the fence crosses what changes, for marking. */
  crossings: Vec2[];
  /** Objects the fence crossed that could not be trimmed or extended (no boundary, holes, another kind). */
  failed: number;
  /** Objects the fence crossed on locked layers, left alone. */
  locked: number;
}

/** How close a crossing lies to a piece of an object for the piece to be the one crossed, metres. */
const ON = 1e-6;

const boundsOf = (pts: readonly Vec2[]) => ({
  minX: Math.min(...pts.map((p) => p.x)),
  minY: Math.min(...pts.map((p) => p.y)),
  maxX: Math.max(...pts.map((p) => p.x)),
  maxY: Math.max(...pts.map((p) => p.y)),
});

/** Whether `p` lies on the object (within `ON`). */
function lies(e: Entity, p: Vec2): boolean {
  const path = pathOf(e);
  return !!path && dist(pointAtS(path, nearestS(path, p)), p) <= ON;
}

/**
 * What the fence does. `chosen` are the boundary objects picked with “Sınır seç” (null:
 * every visible edge). `limit` stops after that many objects, for the live preview of a
 * big drawing.
 */
export function planFence(ctx: AppContext, action: FenceAction, fence: readonly Vec2[], chosen: ReadonlySet<number> | null, limit = Infinity): FencePlan {
  const plan: FencePlan = { trims: [], extends: [], crossings: [], failed: 0, locked: 0 };
  if (fence.length < 2) return plan;
  const { doc, view } = ctx;
  let seen = 0;
  for (const e of view.entitiesIn(boundsOf(fence))) {
    const crossings = fenceCrossings(e, fence);
    if (!crossings.length) continue;
    if (doc.layers.isLocked(e.layerId)) {
      plan.locked++;
      continue;
    }
    if (seen >= limit) break;
    if (action === 'trim' ? planTrim(ctx, plan, e, crossings, chosen) : planExtend(ctx, plan, e, crossings, chosen)) seen++;
  }
  return plan;
}

function planTrim(ctx: AppContext, plan: FencePlan, e: Entity, crossings: readonly Vec2[], chosen: ReadonlySet<number> | null): boolean {
  // Cutting open an area with holes would lose them; it is refused as the click refuses it.
  if (e.kind === 'polygon' && e.holes?.length) {
    plan.failed++;
    return false;
  }
  let pieces: Entity[] = [e];
  const hits: Vec2[] = [];
  let changed = false;
  for (const q of crossings) {
    // The piece the crossing lies on; a crossing in a part an earlier pick took away is nothing to do.
    const i = pieces.findIndex((p) => lies(p, q));
    if (i < 0) continue;
    const r = ctx.view.trim(pieces[i], q, chosen);
    if ('error' in r) continue;
    pieces = [...pieces.slice(0, i), ...r.pieces.map((g) => withGeometry(pieces[i], g as unknown as Geometry)), ...pieces.slice(i + 1)];
    hits.push(q);
    changed = true;
  }
  if (!changed) {
    plan.failed++;
    return false;
  }
  plan.trims.push({ entity: e, pieces });
  plan.crossings.push(...hits);
  return true;
}

function planExtend(ctx: AppContext, plan: FencePlan, e: Entity, crossings: readonly Vec2[], chosen: ReadonlySet<number> | null): boolean {
  let now = e;
  const hits: Vec2[] = [];
  // Each end is extended at most once, however many times the fence crosses near it.
  const extended = new Set<'start' | 'end'>();
  for (const q of crossings) {
    const path = pathOf(now);
    const end = path && nearestS(path, q) > path.length / 2 ? 'end' : 'start';
    if (extended.has(end)) continue;
    const r = ctx.view.extend(now, q, chosen);
    if ('error' in r) continue;
    now = withGeometry(now, r.geometry as unknown as Geometry);
    extended.add(end);
    hits.push(q);
  }
  if (now === e) {
    plan.failed++;
    return false;
  }
  plan.extends.push({ entity: e, geometry: entityGeometry(now) });
  plan.crossings.push(...hits);
  return true;
}

/** The plan as the edits `cad.entities.edit` takes, one step: the first piece keeps the place and the persistent id, the others are new. */
export function fenceEdits(ctx: AppContext, plan: FencePlan): EntityEdit[] {
  const edits: EntityEdit[] = [];
  for (const { entity, pieces } of plan.trims) {
    const uid = uidOf(ctx, entity);
    const keep = pieces.length === 1 && entity.kind !== 'polygon';
    const [first, ...others] = pieces.map((p) => editGeometry(entityGeometry(p)));
    edits.push(first ? { kind: 'replace', uid, geometry: first, ...(keep && { keepData: true }) } : { kind: 'remove', uid });
    for (const g of others) edits.push({ kind: 'add', from: uid, geometry: g, ...(keep && { keepData: true }) });
  }
  for (const { entity, geometry } of plan.extends) edits.push({ kind: 'update', uid: uidOf(ctx, entity), geometry: editGeometry(geometry) });
  return edits;
}
