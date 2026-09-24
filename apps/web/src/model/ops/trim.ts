import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import type { Edge } from '../geom/intersect';
import { entityOp } from './entityOp';

/**
 * Quick trim and extend (AutoCAD-style: every other visible edge acts as
 * a boundary). The target is treated as a path parameterised by arc
 * length s ∈ [0, L] (see ./path); cuts are the s values where boundaries
 * cross it. Computed by the geometry core (docs/adr/0008); the tools ask
 * the geometry store, which gathers the boundaries itself.
 */

export type TrimResult = { pieces: EntityGeometry[] } | { error: string };

/** Removes the part of `target` between the two cuts that bracket `pick`. */
export const trimEntity = entityOp<(target: Entity, pick: Vec2, boundaries: readonly Edge[]) => TrimResult>('trimEntity');

export type ExtendResult = { geometry: EntityGeometry } | { error: string };

/** Extends the end of `target` nearest to `pick` to the first boundary it meets. */
export const extendEntity = entityOp<(target: Entity, pick: Vec2, boundaries: readonly Edge[]) => ExtendResult>('extendEntity');
