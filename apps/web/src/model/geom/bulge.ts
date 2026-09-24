import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';

/**
 * Polyline arc segments follow the DXF "bulge" convention: the segment from
 * pts[i] to pts[i+1] carries bulge = tan(θ/4), θ being its included angle,
 * positive when the arc runs counter-clockwise; 0 is a straight segment.
 * Vertices stay the only coordinates (grips, snaps and coordinate lists
 * need no special case) and the data maps one-to-one onto DXF LWPOLYLINE.
 * Computed by the geometry core (docs/adr/0008).
 */

/** The bulge of segment i (0 when the path has none): reading the data, no calculation. */
export const bulgeAt = (bulges: readonly number[] | undefined, i: number): number => bulges?.[i] ?? 0;

export const isArcBulge = op<(b: number) => boolean>('isArcBulge');

export const hasBulges = op<(bulges: readonly number[] | undefined) => boolean>('hasBulges');

/** Circle, start angle and signed sweep of an arc segment; null when straight or degenerate. */
export const bulgeArc = op<(a: Vec2, b: Vec2, bulge: number) => { c: Vec2; r: number; a0: number; sweep: number } | null>('bulgeArc');

export const bulgeOfSweep = op<(sweep: number) => number>('bulgeOfSweep');

/** Middle of a segment: the chord midpoint, or the arc's midpoint when bulged. */
export const segmentMid = op<(a: Vec2, b: Vec2, bulge: number) => Vec2>('segmentMid');

/** Bulge of the arc from `a` through `m` to `b`; 0 when the points are collinear. */
export const bulgeThrough = op<(a: Vec2, m: Vec2, b: Vec2) => number>('bulgeThrough');

/**
 * Bulge of the arc leaving `a` along direction `dir` and ending at `b`
 * (polyline arc mode continues tangentially). Null when `b` lies straight
 * behind `a`, which would need a full circle.
 */
export const tangentBulge = op<(a: Vec2, dir: Vec2, b: Vec2) => number | null>('tangentBulge');

/** Unit travel direction at the end (`atEnd`) or start of a segment. */
export const segmentTangent = op<(a: Vec2, b: Vec2, bulge: number, atEnd: boolean) => Vec2>('segmentTangent');

/** Primitive edges of a bulged path (arc edges keep the travel direction). */
export const bulgePathEdges = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean) => Edge[]>('bulgePathEdges');

/**
 * Point list with arc segments tessellated (step ≤ `maxStep` radians, a
 * 72nd of a turn by default). A closed path returns a ring without
 * repeating its first point.
 */
export const bulgePathOutline = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean, maxStep?: number) => Vec2[]>('bulgePathOutline');

/** Length along the path, arcs measured exactly. */
export const bulgePathLength = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean) => number>('bulgePathLength');

/** Signed area of a closed bulged ring: shoelace plus each arc's circular segment. */
export const bulgeRingArea = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined) => number>('bulgeRingArea');

/** Path reversed: vertex order flips and every arc turns the other way. */
export const reverseBulgePath = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean) => { pts: Vec2[]; bulges: number[] }>('reverseBulgePath');

/**
 * Drops zero-length segments (coincident consecutive vertices, within
 * `tol`, 1e-9 m by default), keeping the bulge of the segment that
 * survives. Returns bulges only when one is an arc.
 */
export const cleanBulgePath = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean, tol?: number) => { pts: Vec2[]; bulges?: number[] }>('cleanBulgePath');
