import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

export type OffsetResult = { geometry: EntityGeometry } | { error: string };

/**
 * Parallel copy at `distance`, on the side of `through`. Circles and arcs
 * grow or shrink concentrically; paths are offset with mitred corners.
 * Computed by the geometry core (docs/adr/0008).
 */
export const offsetEntity = entityOp<(e: Entity, distance: number, through: Vec2) => OffsetResult>('offsetEntity');
