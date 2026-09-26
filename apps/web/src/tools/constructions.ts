import type { Entity, EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Affine } from '../model/geom/affine';
import type { ArcGeom } from '../model/geom/arc';
import type { EllipseGeom } from '../model/geom/ellipse';
import { op } from '../wasm/core';

/**
 * What the tools compute from their points and typed values, from the Rust
 * core (crates/shared/geometry-core/src/tools, docs/adr/0008 S5): the point
 * calculator's arithmetic, directions and angles, typed-radius polygons,
 * arc and ellipse helpers, polyline arc bulges, rotate/scale/polar array/align
 * transforms, fillet and chamfer corners, dimension arms. The tools only
 * pick, preview and record; camera and screen pixels stay in TypeScript.
 */

// ── Nokta hesabı ───────────────────────────────────────────────────────

/** İki nokta ortası. */
export const midpoint = op<(a: Vec2, b: Vec2) => Vec2>('midpoint');
/** Hat üzerinde nokta by ratio: `num/den` of the way from A to B. */
export const alongRatio = op<(a: Vec2, b: Vec2, num: number, den: number) => Vec2 | null>('alongRatio');
/** Açı-mesafe with the angle in the project's unit (`deg`, else grads), clockwise from S→R. */
export const calcPolar = op<(s: Vec2, r: Vec2, angle: number, unit: string, distance: number) => Vec2 | null>('calcPolar');
/** The candidate nearest to p (the first of equally near ones); null for none. */
export const nearestOf = op<(points: readonly Vec2[], p: Vec2) => Vec2 | null>('nearestOf');

// ── Çizim araçları ─────────────────────────────────────────────────────

/** Direction angle from a to b (radians, CCW from east). */
export const directionAngle = op<(a: Vec2, b: Vec2) => number>('directionAngle');
/** Regular polygon for a typed radius, bottom edge horizontal. */
export const regularPolygonRadius = op<(c: Vec2, sides: number, r: number, inscribed: boolean) => Vec2[] | null>('regularPolygonRadius');
/** End point and travel direction of a line, arc or polyline; null for other kinds. */
export const endTangent = op<(e: EntityGeometry) => { p: Vec2; dir: Vec2 } | null>('endTangent');
/** Unit direction of a typed angle in degrees. */
export const degDirection = op<(deg: number) => Vec2>('degDirection');
/** Circle on a diameter. */
export const circleOnDiameter = op<(a: Vec2, b: Vec2) => { c: Vec2; r: number }>('circleOnDiameter');
/** Ellipse parameter at the polar angle of p seen from the centre, relative to the major axis. */
export const ellipseParamToward = op<(g: EllipseGeom, p: Vec2) => number>('ellipseParamToward');
/** The other half-axis of an ellipse for a rotation angle (degrees). */
export const ellipseRotationHalf = op<(p0: Vec2, p1: Vec2, fromCenter: boolean, angle: number) => number>('ellipseRotationHalf');
/** Unit vector from a towards b; null when they (nearly) coincide. */
export const unitToward = op<(a: Vec2, b: Vec2) => Vec2 | null>('unitToward');
/** Direction of a construction line through p; null: not enough input yet. */
export const xlineDirection = op<(mode: string, pts: readonly Vec2[], p: Vec2, angle: number) => Vec2 | null>('xlineDirection');
/** The point on the circle (c, r) in the direction of p; null when p is the centre. */
export const radialPoint = op<(c: Vec2, r: number, p: Vec2) => Vec2 | null>('radialPoint');
/** Bulge of a polyline arc of radius r from `last` to p; null when the chord is longer than the diameter. */
export const radiusBulge = op<(last: Vec2, p: Vec2, r: number, tangent: Vec2 | null) => number | null>('radiusBulge');
/** Bulge of a polyline arc around a centre, counter-clockwise; null for no sweep. */
export const centreBulge = op<(c: Vec2, last: Vec2, end: Vec2) => number | null>('centreBulge');
/** p moved `distance` along `dir`. */
export const offsetAlong = op<(p: Vec2, dir: Vec2, distance: number) => Vec2>('offsetAlong');
/** Text angle (degrees) from two points, kept readable. */
export const textAngle = op<(from: Vec2, p: Vec2) => number>('textAngle');
/** Donut rings: the outer ring, and the hole when the inner diameter is not zero. */
export const donutRings = op<(c: Vec2, inner: number, outer: number) => { ring: Vec2[]; holes?: Vec2[][] }>('donutRings');

// ── Değiştirme araçları ────────────────────────────────────────────────

/** Rotation by the direction from `base` to p, less a reference angle (radians). */
export const rotationAngle = op<(base: Vec2, p: Vec2, ref: number) => number>('rotationAngle');
/** Scale factor: the distance from `base` to p over the reference length. */
export const scaleFactor = op<(base: Vec2, p: Vec2, refLength: number) => number>('scaleFactor');
/** Polar array: a transform per copy (the original left out). */
export const polarArrayTransforms = op<(c: Vec2, count: number, fill: number, rotate: boolean, ref: Vec2) => Affine[]>('polarArrayTransforms');
/** ALIGN: s1, d1[, s2, d2] → the transform; null when the points coincide. */
export const alignTransform = op<(pts: readonly Vec2[], scale: boolean) => Affine | null>('alignTransform');

/** A corner that can be rounded or cut: sides `u1`/`u2`, `reach` of the shorter one, angle `phi` between them. */
export interface CornerGeom {
  at: Vec2;
  u1: Vec2;
  u2: Vec2;
  reach: number;
  phi: number;
}

/** The corner at a path vertex between the segment from `prev` and the one to `next`; null beside an arc or on a straight run. */
export const vertexCorner = op<(prev: Vec2, at: Vec2, next: Vec2, bulgeIn: number, bulgeOut: number) => CornerGeom | null>('vertexCorner');
/** Corner of two lines picked at p1 and p2; each keeps the side its pick point is on. */
export const linesCornerAt = op<(a1: Vec2, b1: Vec2, p1: Vec2, a2: Vec2, b2: Vec2, p2: Vec2) => CornerGeom | null>('linesCornerAt');
/** How far the cursor has been pulled along the nearer side, rounded to a step that suits the zoom. */
export const pulledDistance = op<(c: CornerGeom, cursor: Vec2 | null, tol: number) => number>('pulledDistance');
/** Fillet radius for a pulled tangent length. */
export const filletRadiusFor = op<(t: number, phi: number) => number>('filletRadiusFor');
/** The fillet arc alone; null for no radius. */
export const filletArc = op<(c: CornerGeom, radius: number) => ArcGeom | null>('filletArc');
/** The chamfer cut alone; null unless both distances are positive. */
export const chamferLine = op<(c: CornerGeom, d1: number, d2: number) => { a: Vec2; b: Vec2 } | null>('chamferLine');

/** Where a corner under the cursor is: a path's vertex, or two lines meeting end to end (by their place in the candidates). */
export type CornerSite = { kind: 'vertex'; object: number; vertex: number } | { kind: 'lines'; first: number; pick1: Vec2; second: number; pick2: Vec2 };

/**
 * The corner nearest `p` within `tol` among the candidates (lines, polylines,
 * closed areas): a path vertex, or two lines whose ends lie within `same` of
 * each other; the first on a tie (docs/adr/0047). Each line of a pair keeps
 * the side of its far end.
 */
export const cornerNear = op<(candidates: readonly Entity[], p: Vec2, tol: number, same: number) => { site: CornerSite; corner: CornerGeom } | null>('cornerNear');

/** Angular dimension arms from a vertex and a point on each arm, in the sector `loc` is in. */
export const vertexArms = op<(c: Vec2, p1: Vec2, p2: Vec2, loc: Vec2) => { c: Vec2; a: Vec2; b: Vec2 }>('vertexArms');
/** Angular dimension arms between two picked edges; null for parallel edges. */
export const edgeArms = op<(e1: { a: Vec2; b: Vec2; at: Vec2 }, e2: { a: Vec2; b: Vec2; at: Vec2 }, loc: Vec2) => { c: Vec2; a: Vec2; b: Vec2 } | null>('edgeArms');
/** Radius or diameter dimension placed at `loc`: the point on the circle and the outward offset. */
export const radialDimension = op<(c: Vec2, r: number, loc: Vec2) => { b: Vec2; offset: number }>('radialDimension');
