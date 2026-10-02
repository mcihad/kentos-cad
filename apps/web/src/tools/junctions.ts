import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { Entity } from '../model/entities';
import type { Area } from '../model/geom/overlay';
import { adjoinJunctions } from '../model/ops/adjoin';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { topologyOn } from './neighbours';
import { ringBox } from './overlap';

/**
 * Corners shared with the neighbours (docs/adr/0162 §4): while Topoloji is on, a new area drawn by its outline is
 * joined with the visible lines, paths and areas it touches (points too with Noktalar da) corner by corner: its corners
 * on their edges go to them, theirs on its edges to it, within 1 µm. The shared core finds them (`adjoinJunctions`);
 * the neighbours are written through `cad.entities.edit` (`vertexAdd`, their elevations carried along the edge) in the
 * new area's undo step. A locked layer's neighbour takes none and is counted. The desktop's is
 * crates/native/interaction/src/junctions.rs; both play fixtures/interaction/v1/junctions.json.
 */

/** How near a corner lies on an edge (the core's `SAME`). */
const SAME = 1e-6;

/** The kinds that share corners; points only with Noktalar da. */
const SHARING = new Set<Entity['kind']>(['line', 'polyline', 'polygon', 'point']);

/** What joining a new area with its neighbours gives: the new area to write, the neighbours' writes and what to say. */
export interface Joining {
  /** The new area's parts, the neighbours' corners on their edges added. */
  areas: Area[];
  /** The neighbours given corners of the new area. */
  changes: EntityEdit[];
  /** How many corners the new area took. */
  taken: number;
  /** How many corners the neighbours were given, all together. */
  given: number;
  /** How many neighbours would have been given one but lie on a locked layer. */
  locked: number;
}

/**
 * The new area joined with its neighbours: the visible lines, paths and areas near it (points with Noktalar da), a
 * locked layer's counted. Null while Topoloji is off or nothing is shared.
 */
export function joinCorners(ctx: AppContext, areas: readonly Area[]): Joining | null {
  if (!topologyOn(ctx) || !areas.length) return null;
  const points = ctx.settings.topologyPoints.value;
  const boxes = areas.map((a) => ringBox(a.outer));
  const box = {
    minX: Math.min(...boxes.map((b) => b.minX)) - SAME,
    minY: Math.min(...boxes.map((b) => b.minY)) - SAME,
    maxX: Math.max(...boxes.map((b) => b.maxX)) + SAME,
    maxY: Math.max(...boxes.map((b) => b.maxY)) + SAME,
  };
  const found = ctx.view.entitiesIn(box).filter((e) => SHARING.has(e.kind) && (points || e.kind !== 'point'));
  if (!found.length) return null;
  const got = adjoinJunctions<Entity>(
    areas,
    found.map((e) => ({ shape: e, locked: ctx.doc.layers.isLocked(e.layerId) })),
    points,
  );
  if (!got.taken && !got.edited.length && !got.locked) return null;
  return {
    areas: got.areas,
    changes: got.edited.map(({ index, shape }) => ({ kind: 'update', uid: uidOf(ctx, found[index]), geometry: editGeometry(shape) })),
    taken: got.taken,
    given: got.given,
    locked: got.locked,
  };
}

/** Thrown inside the step when the neighbours' write is refused, so the new area goes too. */
const REFUSED = Symbol('refused');

/**
 * Writes what `write` writes (the new area) and the neighbours' corners as one undo step named `label` (the new area's
 * own); when either is refused (its message said), nothing stays. What `write` gave, or null.
 */
export function withJoined<T>(ctx: AppContext, joining: Joining | null, label: string, write: () => T | null): T | null {
  if (!joining?.changes.length) return write();
  try {
    return ctx.doc.transact(label, () => {
      const out = write();
      if (out === null) return null;
      if (!writeEdit(ctx, 'vertexAdd', joining.changes)) throw REFUSED;
      return out;
    });
  } catch (e) {
    if (e === REFUSED) return null;
    throw e;
  }
}

/** Says what joining did, once the new area is written. */
export function sayJoined(ctx: AppContext, joining: Joining | null): void {
  if (!joining) return;
  if (joining.changes.length) ctx.log.info(`Topolojik düzenleme: ${joining.changes.length} komşu nesneye yeni alanın ${joining.given} köşesi eklendi.`);
  if (joining.taken) ctx.log.info(`Topolojik düzenleme: yeni alana komşulardan ${joining.taken} köşe eklendi.`);
  if (joining.locked) ctx.log.warn(`Kilitli katmandaki ${joining.locked} komşu nesneye köşe eklenmedi.`);
}
