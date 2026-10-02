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

/** The neighbours `edited` becoming `after` puts right; null while the mode is off or nothing is shared. */
export function neighbours(ctx: AppContext, edited: Entity, after: Entity): Neighbours | null {
  const { settings, doc, view } = ctx;
  if (!settings.topology.value) return null;
  const points = settings.topologyPoints.value;
  // A point is a corner only with Noktalar da.
  if (edited.kind === 'point' && !points) return null;
  const changes = topologyChanges(edited, after);
  if (!changes.length) return null;
  const found: Entity[] = [];
  const seen = new Set([edited.id]);
  for (const at of anchors(changes))
    for (const id of view.pickRect({ minX: at.x - SAME, minY: at.y - SAME, maxX: at.x + SAME, maxY: at.y + SAME }, true)) {
      if (seen.has(id)) continue;
      seen.add(id);
      const e = doc.get(id);
      if (e && SHARING.has(e.kind) && (points || e.kind !== 'point')) found.push(e);
    }
  if (!found.length) return null;
  const answer = topologyEdit<Entity>(
    found.map((e) => ({ shape: e, locked: doc.layers.isLocked(e.layerId) })),
    changes,
    points,
  );
  return {
    changes: answer.edited.map(({ index, shape }) => ({ kind: 'update', uid: uidOf(ctx, found[index]), geometry: editGeometry(shape) })),
    shapes: answer.edited.map(({ shape }) => shape),
    locked: answer.locked,
    invalid: answer.invalid,
  };
}

/** Says what the neighbours did, once the edit is written: how many changed with it, how many could not. */
export function sayNeighbours(ctx: AppContext, n: Neighbours | null): void {
  if (!n) return;
  if (n.changes.length) ctx.log.info(`Topolojik düzenleme: ${n.changes.length} komşu nesne de değişti.`);
  if (n.locked) ctx.log.warn(`Kilitli katmandaki ${n.locked} komşu nesne değişmedi; ortak sınır ayrıldı.`);
  if (n.invalid) ctx.log.warn(`${n.invalid} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).`);
}

/** Where each change's neighbours have a corner: a move's and a removal's vertex, an edge's first end. */
const anchors = (changes: readonly TopologyChange[]): Vec2[] => changes.map((c) => (c.kind === 'move' || c.kind === 'remove' ? c.at : c.a));
