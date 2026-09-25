import { svgOp } from './core';
import type { PathNode, Pt, SubPath } from './pathData';

/**
 * Cubic Bézier helpers for the SVG editor's path operations: segments of a
 * sub-path, points and tangents, de Casteljau splitting, flattening to a
 * tolerance, lengths and nearest points. A straight segment is a cubic
 * whose controls sit on its ends, so every segment has one form. Computed
 * by the SVG core (crates/shared/svg-core `bezier.rs`); which node a
 * segment runs between is bookkeeping and stays here.
 */

export type Cubic = readonly [Pt, Pt, Pt, Pt];

/** Segment i of a sub-path runs from node i to node i+1 (the last one of a closed sub-path back to node 0). */
export function segmentCount(sp: SubPath): number {
  const n = sp.nodes.length;
  if (n < 2) return sp.closed && n === 1 && (sp.nodes[0].in || sp.nodes[0].out) ? 1 : 0;
  return sp.closed ? n : n - 1;
}

export const isLineSeg = (a: PathNode, b: PathNode) => !a.out && !b.in;

/** The cubic of segment i (controls on the ends for a straight segment). */
export function segmentCubic(sp: SubPath, i: number): Cubic {
  const a = sp.nodes[i];
  const b = sp.nodes[(i + 1) % sp.nodes.length];
  return [[a.x, a.y], a.out ?? [a.x, a.y], b.in ?? [b.x, b.y], [b.x, b.y]];
}

export const segmentIsLine = (sp: SubPath, i: number) => isLineSeg(sp.nodes[i], sp.nodes[(i + 1) % sp.nodes.length]);

export const bez = svgOp<(c: Cubic, t: number) => Pt>('bez');

/** First derivative at t. */
export const bezDeriv = svgOp<(c: Cubic, t: number) => Pt>('bezDeriv');

/**
 * Unit tangent at t. At an end whose control coincides with it the first
 * derivative vanishes; the direction then comes from the next control.
 */
export const bezTangent = svgOp<(c: Cubic, t: number) => Pt>('bezTangent');

/** de Casteljau split at t: the part before and the part after. */
export const splitCubic = svgOp<(c: Cubic, t: number) => [Cubic, Cubic]>('splitCubic');

/** Chord ends along a cubic (both ends included) with their parameters. */
export const flattenCubic = svgOp<(c: Cubic, tol: number) => { pts: Pt[]; ts: number[] }>('flattenCubic');

/** A sub-path as a polyline within `tol`; a closed one does not repeat its start (the closing chord is implied). */
export const flattenSubPathTol = svgOp<(sp: SubPath, tol: number) => Pt[]>('flattenSubPathTol');

/** Arc length between t0 and t1 (Gauss–Legendre on quarters). */
export const cubicLength = svgOp<(c: Cubic, t0?: number, t1?: number) => number>('cubicLength');

/** Length of segment i of a sub-path. */
export const segmentLength = (sp: SubPath, i: number) => cubicLength(segmentCubic(sp, i));

/** Parameter on the curve nearest to p (sampling, then Newton steps). */
export const nearestOnCubic = svgOp<(c: Cubic, p: Pt) => { t: number; p: Pt; d: number }>('nearestOnCubic');

/** The parameter where the curve is `dist` away (straight line) from its start (`fromEnd`: from its end). */
export const paramAtDistance = svgOp<(c: Cubic, dist: number, fromEnd: boolean) => number | null>('paramAtDistance');

/** Signed area of a ring (shoelace, y down: positive is clockwise on screen). */
export const ringSignedArea = svgOp<(pts: readonly Pt[]) => number>('ringSignedArea');

/** Winding number of a closed polyline around p. */
export const windingOf = svgOp<(ring: readonly Pt[], p: Pt) => number>('windingOf');
