import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { topologyChanges, topologyEdit, type TopologyChange } from '../model/ops/topologyEdit';
import { editGeometry, uidOf } from './editCommand';

/**
 * Topological editing (docs/adr/0160): the objects around an edit whose shared corners and edges go with it, while the
 * mode is on. What the edit changed and how the neighbours follow are the core's (`topologyChanges`,
 * `topologyEdit`); the neighbours are the visible objects within 1 µm of the places the edit changed, looked up in the
 * drawing's index (a locked layer's are counted, never changed). The desktop's is
 * `crates/native/interaction/src/neighbours.rs`; both play fixtures/interaction/v1/topology-edit.json.
 */

/** How near a neighbour's corner is shared (the core's `SAME`). */
const SAME = 1e-6;

/** The kinds that share corners; points only with Noktalar da. */
const SHARING = new Set<Entity['kind']>(['line', 'polyline', 'polygon', 'point']);

/** The neighbours an edit puts right: their writes, their shapes for the preview, how many are locked or left invalid. */
export interface Neighbours {
  changes: EntityEdit[];
  shapes: Entity[];
  locked: number;
  invalid: number;
}

/** Whether the mode is on. */
export const topologyOn = (ctx: AppContext): boolean => ctx.settings.topology.value;

/**
 * The visible objects sharing a corner with the places `at` might be (lines, paths, areas; points with Noktalar da),
 * the edited ones (`edited`, by slot) left out, in the order the index finds them.
 */
export function neighboursAt(ctx: AppContext, at: readonly Vec2[], edited: ReadonlySet<number>): Entity[] {
  const { doc, view, settings } = ctx;
  const points = settings.topologyPoints.value;
  const found: Entity[] = [];
  const seen = new Set(edited);
  for (const p of at)
    for (const id of view.pickRect({ minX: p.x - SAME, minY: p.y - SAME, maxX: p.x + SAME, maxY: p.y + SAME }, true)) {
      if (seen.has(id)) continue;
      seen.add(id);
      const e = doc.get(id);
      if (e && SHARING.has(e.kind) && (points || e.kind !== 'point')) found.push(e);
    }
  return found;
}

/** `found` put right by `changes` (the core's `topologyEdit`): their writes and shapes, the locked and invalid counts. */
export function putRight(ctx: AppContext, found: readonly Entity[], changes: readonly TopologyChange[]): Neighbours {
  const answer = topologyEdit<Entity>(
    found.map((e) => ({ shape: e, locked: ctx.doc.layers.isLocked(e.layerId) })),
    changes,
    ctx.settings.topologyPoints.value,
  );
  return {
    changes: answer.edited.map(({ index, shape }) => ({ kind: 'update', uid: uidOf(ctx, found[index]), geometry: editGeometry(shape) })),
    shapes: answer.edited.map(({ shape }) => shape),
    locked: answer.locked,
    invalid: answer.invalid,
  };
}

/**
 * The neighbours the edited objects (each before and after its edit) put right together; null while the mode is off
 * or nothing is shared. A point is a corner only with Noktalar da.
 */
export function neighboursOf(ctx: AppContext, edits: readonly (readonly [Entity, Entity])[]): Neighbours | null {
  if (!topologyOn(ctx)) return null;
  const points = ctx.settings.topologyPoints.value;
  const changes = edits.flatMap(([before, after]) => (before.kind === 'point' && !points ? [] : topologyChanges(before, after)));
  if (!changes.length) return null;
  const found = neighboursAt(ctx, anchors(changes), new Set(edits.map(([before]) => before.id)));
  return found.length ? putRight(ctx, found, changes) : null;
}

/** The neighbours `edited` becoming `after` puts right (a grip, the grip menu). */
export const neighbours = (ctx: AppContext, edited: Entity, after: Entity): Neighbours | null => neighboursOf(ctx, [[edited, after]]);

/** Says what the neighbours did, once the edit is written: how many changed with it, how many could not. */
export function sayNeighbours(ctx: AppContext, n: Neighbours | null): void {
  if (!n) return;
  if (n.changes.length) ctx.log.info(`Topolojik düzenleme: ${n.changes.length} komşu nesne de değişti.`);
  if (n.locked) ctx.log.warn(`Kilitli katmandaki ${n.locked} komşu nesne değişmedi; ortak sınır ayrıldı.`);
  if (n.invalid) ctx.log.warn(`${n.invalid} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).`);
}

/**
 * How many objects have a corner at `at`, the selected `own` among them, while the mode is on (the grip's tag,
 * docs/adr/0160 §5): the core finds the corners as a move that stays in place would. Null when only `own` has one,
 * for an object of a kind without corners, and for a point without Noktalar da.
 */
export function cornerCount(ctx: AppContext, own: Entity, at: Vec2): number | null {
  const corners = own.kind === 'line' || own.kind === 'polyline' || own.kind === 'polygon' || (own.kind === 'point' && ctx.settings.topologyPoints.value);
  if (!topologyOn(ctx) || !corners) return null;
  const found = neighboursAt(ctx, [at], new Set([own.id]));
  if (!found.length) return null;
  const n = putRight(ctx, found, [{ kind: 'move', at, to: at }]);
  const count = 1 + n.shapes.length + n.locked;
  return count > 1 ? count : null;
}

/** Where each change's neighbours have a corner: a move's and a removal's vertex, an edge's first end. */
export const anchors = (changes: readonly TopologyChange[]): Vec2[] => changes.map((c) => (c.kind === 'move' || c.kind === 'remove' ? c.at : c.a));
