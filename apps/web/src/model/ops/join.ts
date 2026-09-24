import type { Entity, EntityGeometry } from '../entities';
import { entityOp } from './entityOp';

export interface JoinGroup {
  geometry: EntityGeometry;
  /** Source ids in chain order; the first decides layer and attributes. */
  sources: number[];
}

/**
 * Joins lines, arcs and open polylines whose ends meet (within `tol`) into
 * polylines with arc segments; a chain that returns to its start becomes a
 * closed polygon. Entities that connect to nothing are left out. Computed
 * by the geometry core (docs/adr/0008).
 */
export const joinEntities = entityOp<(list: readonly Entity[], tol: number) => { groups: JoinGroup[]; skipped: number[] }>('joinEntities');
