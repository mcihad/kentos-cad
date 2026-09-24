import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/** Circles and circular arcs, computed by the geometry core (docs/adr/0008). */

/** 2π (the double `Math.PI * 2` gives). */
export const TAU = 6.283185307179586;

/** Normalizes an angle to [0, 2π). */
export const normAngle = op<(a: number) => number>('normAngle');

/** CCW sweep from a0 to a1 in (0, 2π]. Equal angles mean a full turn. */
export const sweep = op<(a0: number, a1: number) => number>('sweep');

/** Whether angle θ lies on the CCW arc from a0 spanning `sw` (inclusive, with tolerance; 1e-9 by default). */
export const onArc = op<(theta: number, a0: number, sw: number, eps?: number) => boolean>('onArc');

/** Position of angle θ along the arc as 0..1 (for trimming parameters). */
export const arcParam = op<(theta: number, a0: number, sw: number) => number>('arcParam');

export const pointOnCircle = op<(c: Vec2, r: number, a: number) => Vec2>('pointOnCircle');

export const circleThrough = op<(p1: Vec2, p2: Vec2, p3: Vec2) => { c: Vec2; r: number } | null>('circleThrough');

export interface ArcGeom {
  c: Vec2;
  r: number;
  /** Start angle (radians, CCW from east). */
  a0: number;
  /** End angle; the arc runs CCW from a0 to a1. */
  a1: number;
}

/**
 * Arc from start through mid to end. Stored CCW, so a clockwise pick order
 * swaps start and end — the drawn shape is the same.
 */
export const arcThrough = op<(start: Vec2, mid: Vec2, end: Vec2) => ArcGeom | null>('arcThrough');

/** Points along the arc, steps of at most `maxStep` radians (a 72nd of a turn by default). */
export const tessellateArc = op<(a: ArcGeom, maxStep?: number) => Vec2[]>('tessellateArc');

export const arcStart = op<(a: ArcGeom) => Vec2>('arcStart');
export const arcEnd = op<(a: ArcGeom) => Vec2>('arcEnd');
export const arcMid = op<(a: ArcGeom) => Vec2>('arcMid');
export const arcLength = op<(a: ArcGeom) => number>('arcLength');

/** DXF bulge (tan(θ/4)) of a counter-clockwise arc. */
export const bulgeFromArc = op<(a: ArcGeom) => number>('bulgeFromArc');
