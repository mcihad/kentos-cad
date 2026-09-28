import type { Entity } from '../entities';
import { entityOp } from './entityOp';

/**
 * Whole-object reshaping by the geometry core (docs/adr/0140, 0008): every
 * corner of a path rounded or cut, a path drawn the other way, a path with
 * the vertices within a tolerance dropped. Each answers the object itself,
 * changed: every field but its geometry comes back untouched.
 */

/** A rounding (`radius`) or a cut (`d1`, `d2`), the value every corner of the object takes. */
export type CornerValue = { radius: number } | { d1: number; d2: number };

export interface Reshaped {
  entity: Entity;
  /** Corners rounded or cut; vertices removed (Sadeleştir). */
  done: number;
  /** Corners the value did not fit, or that lie next to an arc edge. */
  skipped: number;
  /** Sadeleştir: the largest distance of a removed vertex from the new outline, metres; 0 otherwise. */
  deviation: number;
}

/**
 * Every corner of a polyline or polygon (holes too) rounded or cut with one
 * value. A corner the value does not fit, one sharing a short edge with the
 * next, and one next to an arc edge are left and counted in `skipped`. Null
 * for another kind of object, or a value not above zero.
 */
export const allCorners = entityOp<(e: Entity, op: CornerValue) => Reshaped | null>('allCorners');

/**
 * A line, polyline, spline or area drawn the other way (vertex order, arcs
 * and the rings of an area turned round; the shape is the same). Null for an
 * arc, ellipse, circle, point, text and the rest.
 */
export const reverseEntity = entityOp<(e: Entity) => Entity | null>('reverseEntity');

/**
 * Douglas–Peucker over the straight runs of a polyline or polygon: the
 * vertices within `tolerance` metres of the new outline go (arc edges stay
 * whole; a closed ring keeps three). `done` counts the vertices removed and
 * `deviation` tells how far the farthest of them lay. Null for another kind.
 */
export const simplifyEntity = entityOp<(e: Entity, tolerance: number) => Reshaped | null>('simplifyEntity');
