import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Primitive edges every entity decomposes into. Circles are arcs with a
 * full sweep. Intersection, trimming, extending and snapping all work on
 * these, so a new entity kind only has to provide its edges. Computed by
 * the geometry core (docs/adr/0008).
 *
 * An arc edge runs from angle `a0` through the signed `sweep`: positive is
 * counter-clockwise, negative clockwise. The sign keeps the travel
 * direction of polyline arc segments, so parameters along a path stay
 * monotonic.
 */
export type Edge =
  | { kind: 'seg'; a: Vec2; b: Vec2 }
  | { kind: 'arc'; c: Vec2; r: number; a0: number; sweep: number };

type ArcEdge = Extract<Edge, { kind: 'arc' }>;

/** Whether angle θ lies on the arc edge (either direction). */
export const onEdgeArc = op<(e: ArcEdge, theta: number) => boolean>('onEdgeArc');

/** A hit on edge 1 at parameter t (0..1 along it) and on edge 2 at u. */
export interface Hit {
  p: Vec2;
  t: number;
  u: number;
}

/** Infinite-line intersection; t, u are parameters along a→b and c→d. */
export const lineLine = op<(a: Vec2, b: Vec2, c: Vec2, d: Vec2) => Hit | null>('lineLine');

/** Segment intersection (ends within `eps`, 1e-9 by default, count). */
export const segSeg = op<(a: Vec2, b: Vec2, c: Vec2, d: Vec2, eps?: number) => Hit | null>('segSeg');

/**
 * Parameters t along a→b (unbounded) where the line meets the circle.
 * Solved from the foot of the perpendicular from the centre rather than
 * the quadratic's discriminant: construction lines are 2·10⁷ m long, and
 * the discriminant would lose most of its digits to cancellation.
 */
export const lineCircleParams = op<(a: Vec2, b: Vec2, c: Vec2, r: number) => number[]>('lineCircleParams');

export const circleCircle = op<(c1: Vec2, r1: number, c2: Vec2, r2: number) => Vec2[]>('circleCircle');

/** Parameter (0..1) of a point known to lie on the edge. */
export const paramOn = op<(e: Edge, p: Vec2) => number>('paramOn');

export const pointAt = op<(e: Edge, t: number) => Vec2>('pointAt');

/** All intersections between two bounded edges. */
export const intersectEdges = op<(e1: Edge, e2: Edge) => Hit[]>('intersectEdges');

/**
 * Where a ray from `o` along `dir` first meets an edge beyond `minT`
 * (ray parameter in units of |dir|; 1e-9 by default). Used by Extend.
 */
export const rayEdge = op<(o: Vec2, dir: Vec2, e: Edge, minT?: number) => number[]>('rayEdge');

/** Closest point on an edge and its parameter. */
export const closestOnEdge = op<(e: Edge, p: Vec2) => { p: Vec2; t: number; d: number }>('closestOnEdge');

/** Foot of the perpendicular from p onto the edge's supporting line/circle, if it lies on the edge. */
export const perpendicularFoot = op<(e: Edge, p: Vec2) => Vec2 | null>('perpendicularFoot');

export const fullCircle = op<(c: Vec2, r: number) => Edge>('fullCircle');

/** Points where lines from `p` touch the circle (c, r); empty when p is inside. */
export const tangentPoints = op<(p: Vec2, c: Vec2, r: number) => Vec2[]>('tangentPoints');
