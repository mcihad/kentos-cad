import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { DimensionGeom } from '../geom/dimension';
import { entityOp } from './entityOp';

/**
 * Faz 2's constructions by the geometry core (docs/adr/0140, 0008): the
 * circle sector, points between two points, the meetings of two distances or
 * two bearings, an angle at a vertex, the next dimension of a chain or a
 * baseline. The tools pick and preview; what the points make is computed
 * there.
 */

/** The closed ring of a sector: the centre, the arc's start and end, the arc as the middle edge. */
export interface SectorRing {
  pts: Vec2[];
  bulges?: number[];
}

/**
 * The sector centred on `c` of radius `r`, swept counter-clockwise from `a0` to `a1` (radians
 * from east). Null for a radius not above zero and for a sweep of nothing or of a whole turn.
 */
export const sectorRing = entityOp<(c: Vec2, r: number, a0: number, a1: number) => SectorRing | null>('sector');

/** How Ara nokta places its points on the line from a to b. */
export type Between = { parts: number } | { distances: number[] } | { ratios: number[] };

/** The points between `a` and `b`: n − 1 for `parts` (2 to 10 000), or at the distances from a, or the fractions of the way. Null for two points that fall together, a bad count. */
export const pointsBetween = op<(a: Vec2, b: Vec2, how: Between) => Vec2[] | null>('pointsBetween');

/** Kesişim noktası, İki uzaklık: the points `da` from `a` and `db` from `b` (none, one where the circles touch, or two). */
export const distanceDistance = op<(a: Vec2, da: number, b: Vec2, db: number) => Vec2[]>('distanceDistance');

/**
 * Kesişim noktası, İki doğrultu: where the line from `a` along the survey bearing `ta` meets the
 * one from `b` along `tb` (radians from grid north, clockwise). Null for parallel bearings or a
 * meeting behind either point.
 */
export const bearingBearing = op<(a: Vec2, ta: number, b: Vec2, tb: number) => Vec2 | null>('bearingBearing');

/** The angle at a vertex, radians: counter-clockwise from the first arm to the second, the smaller of it and its explement, the larger. */
export interface AngleAt {
  sweep: number;
  inner: number;
  outer: number;
}

/** Açı ölç: null when an arm has no length. */
export const angleAt = op<(vertex: Vec2, p1: Vec2, p2: Vec2) => AngleAt | null>('angleAt');

/**
 * Zincir ölçü: the next dimension of a chain from `from` to `p`, along the base's direction with its
 * line on the base's line. Null for a base that is not an aligned or a linear dimension, or one of no length.
 */
export const continueDimension = op<(base: DimensionGeom, from: Vec2, p: Vec2) => DimensionGeom | null>('continueDimension');

/** Baz ölçü: from the base's first point to `p`, `level` steps of `spacing` metres further out than the base's line. Null as for `continueDimension`, or a spacing not above zero. */
export const baselineDimension = op<(base: DimensionGeom, p: Vec2, level: number, spacing: number) => DimensionGeom | null>('baselineDimension');
