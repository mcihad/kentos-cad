import { fromXY, offsetPathXY, op, toXY } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/** Path offsets, computed by the geometry core (docs/adr/0008). */

/**
 * Offsets a polyline by `d` (positive = left of the travel direction).
 * Corners are mitred; very sharp corners get a bevel so the result never
 * shoots far away from the source. Flat coordinates cross the boundary:
 * the style engine calls this per object while a layer is built.
 */
export function offsetPath(pts: readonly Vec2[], d: number, closed: boolean): Vec2[] {
  return fromXY(offsetPathXY(toXY(pts), d, closed));
}

/** Which side of a path a point lies on: +1 left, −1 right (nearest segment decides). */
export const sideOf = op<(pts: readonly Vec2[], closed: boolean, p: Vec2) => 1 | -1>('sideOf');

/**
 * Offsets a path with arc segments (DXF bulges) by `d` (positive = left of
 * travel). Lines shift along their normal, arcs grow or shrink about their
 * centre; neighbours meet where their supporting line/circle cross (nearest
 * to the source corner) or get a straight bevel when they do not.
 */
export const offsetBulgePath = op<(pts: readonly Vec2[], bulges: readonly number[], d: number, closed: boolean) => { pts: Vec2[]; bulges: number[] } | { error: string }>('offsetBulgePath');
