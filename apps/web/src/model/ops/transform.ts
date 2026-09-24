import type { Entity } from '../entities';
import type { Affine } from '../geom/affine';
import { entityOp } from './entityOp';

/**
 * Applies a similarity transform (move, rotate, uniform scale, mirror) to
 * any entity. Returns a copy with the same id; callers decide whether to
 * update the original or add the copy. Computed by the geometry core
 * (docs/adr/0008).
 */
export const transformEntity = entityOp<<E extends Entity>(e: E, m: Affine) => E>('transformEntity');

export const translateEntity = entityOp<<E extends Entity>(e: E, dx: number, dy: number) => E>('translateEntity');

/**
 * `transformEntity` of every entity by every affine, affine after affine,
 * in one call: moving, copying or arraying a selection of thousands of
 * objects crosses the boundary once, not once per object.
 */
export const transformEntities = entityOp<<E extends Entity>(list: readonly E[], ms: readonly Affine[]) => E[]>('transformEntities');
