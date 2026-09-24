import { coreAngleDeg, coreBearingGrad, coreDist, coreDistToSegment, ringCentroid, ringPathLength, ringPointInPolygon, ringSignedArea } from '../wasm/core';

/**
 * Points, boxes and the small measures on them. The measures are computed
 * by the geometry core (docs/adr/0008, S3b) through numbers-only entry
 * points: the tools call them on every pointer move. Growing a box is
 * bookkeeping, not a calculation, and stays here.
 */

/** World coordinates in metres. x = easting (Y, sağa), y = northing (X, yukarı). */
export interface Vec2 {
  x: number;
  y: number;
}

export interface Bounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

export const vec = (x: number, y: number): Vec2 => ({ x, y });
export const dist = (a: Vec2, b: Vec2): number => coreDist(a.x, a.y, b.x, b.y);

export function emptyBounds(): Bounds {
  return { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
}

export function extendBounds(b: Bounds, p: Vec2, pad = 0): Bounds {
  b.minX = Math.min(b.minX, p.x - pad);
  b.minY = Math.min(b.minY, p.y - pad);
  b.maxX = Math.max(b.maxX, p.x + pad);
  b.maxY = Math.max(b.maxY, p.y + pad);
  return b;
}

export const isEmptyBounds = (b: Bounds) => !(b.maxX >= b.minX && b.maxY >= b.minY);

/**
 * Signed shoelace area; positive for counter-clockwise rings, taken
 * relative to the first vertex (raw TM products would cancel away the
 * fourth decimal of a parcel area).
 */
export const signedArea = (pts: readonly Vec2[]): number => ringSignedArea(pts);

export const pathLength = (pts: readonly Vec2[], closed = false): number => ringPathLength(pts, closed);

export const centroid = (pts: readonly Vec2[]): Vec2 => ringCentroid(pts);

export const pointInPolygon = (p: Vec2, pts: readonly Vec2[]): boolean => ringPointInPolygon(p.x, p.y, pts);

export const distToSegment = (p: Vec2, a: Vec2, b: Vec2): number => coreDistToSegment(p.x, p.y, a.x, a.y, b.x, b.y);

/** Angle in degrees, counter-clockwise from east (CAD convention). */
export const angleDeg = (a: Vec2, b: Vec2): number => coreAngleDeg(a.x, a.y, b.x, b.y);

/**
 * Surveying bearing (semt) in grads, clockwise from grid north — the unit
 * Turkish surveyors read off a total station.
 */
export const bearingGrad = (a: Vec2, b: Vec2): number => coreBearingGrad(a.x, a.y, b.x, b.y);
