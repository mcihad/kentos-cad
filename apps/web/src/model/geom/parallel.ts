import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Area } from './overlay';

/**
 * Netcad "Paralel çizgi": hatches beside an axis at a left and a right
 * distance (road edges beside the centre line, both faces of a wall).
 * Corners are mitred like Ötele; the corridor between the two sides can
 * also be read as one area. Computed by the geometry core (docs/adr/0008).
 */

/** Axis without repeated points (a double click must not make a zero-length leg). */
export const cleanAxis = op<(axis: readonly Vec2[], closed: boolean) => Vec2[]>('cleanAxis');

/**
 * The two sides: `left` metres left of the travel direction, `right`
 * metres right of it. A side at distance 0 is the axis itself (null).
 */
export const parallelSides = op<(axis: readonly Vec2[], left: number, right: number, closed: boolean) => { left: Vec2[] | null; right: Vec2[] | null }>('parallelSides');

/**
 * The corridor between the sides as an area. An open axis gives one ring
 * (left side out, right side back); a closed axis gives a ring with a hole.
 * Null when the corridor has no width or the axis is too short.
 */
export const corridorArea = op<(axis: readonly Vec2[], left: number, right: number, closed: boolean) => Area | null>('corridorArea');
