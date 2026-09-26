import { op } from '../../wasm/core';
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

/**
 * The distance “Noktadan geç” offsets by: from `p` to the object's nearest
 * edge, so the copy passes through `p`; Infinity without edges. Computed by
 * the geometry core (docs/adr/0047).
 */
export const offsetThroughDistance = op<(e: Entity, p: Vec2) => number>('offsetThroughDistance');
