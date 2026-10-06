import type { Entity, EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Affine } from '../model/geom/affine';
import type { ArcGeom } from '../model/geom/arc';
import type { EllipseGeom } from '../model/geom/ellipse';
import type { Edge } from '../model/geom/intersect';
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

// Nokta hesaplayıcı ekleri (docs/adr/0188; crates/shared/geometry-core/src/tools/point_calc.rs).

/** A path's point `s` along it from its start (its end: `fromEnd`) and `offset` square to it, the right positive; none outside 0…length. */
export const pathStation = op<(e: Entity, fromEnd: boolean, s: number, offset: number) => { point?: Vec2 | null; length: number }>('pointCalcStation');
/** Where `p` stands against a path: the distance of its nearest point from the start and how far it is from it (right positive). */
export const pathReading = op<(e: Entity, fromEnd: boolean, p: Vec2) => { s: number; offset: number; length: number } | null>('pointCalcReading');
/** A km typed: `k+mmm.mmm` or metres; null when it is neither. */
export const kmValue = op<(text: string) => number | null>('kmValue');
/** Metres written as km (`k+mmm.ddd`) by the display rule. */
export const kmText = op<(value: number, decimals: number) => string>('kmText');
/** Mesafe ve eğim: a slope distance at a percent slope as its horizontal and its rise. */
export const slopeHorizontal = op<(s: number, percent: number) => { horizontal: number; rise: number }>('slopeHorizontal');
/** Açıortay: the point `d` from K along the bisector of A–K–B; null when K is on an arm's point. */
export const bisectorPoint = op<(k: Vec2, a: Vec2, b: Vec2, d: number) => Vec2 | null>('bisectorPoint');
/** The bisector's point nearest `p`, never behind K. */
export const bisectorNearest = op<(k: Vec2, a: Vec2, b: Vec2, p: Vec2) => Vec2 | null>('bisectorNearest');

// Km yaz (docs/adr/0189; crates/shared/geometry-core/src/ops/stationing.rs).

/** Where a route's stations are: the km interval and the first km (metres), walked from the end, its ends too, the ends' decimals. */
export interface StationRules {
  interval: number;
  start: number;
  reverse: boolean;
  ends: boolean;
  decimals: number;
}

/** What is written at each station, world metres: the km text's side (none: no text), its height, the tick's half length, the cross-section's half width, the point's offset (right positive). */
export interface StationLook {
  text: 'left' | 'right' | null;
  height: number;
  tick: number;
  section: number;
  point: number | null;
}

export interface Stationing {
  stations: { s: number; km: number; text: string; point: Vec2; tangent: Vec2 }[];
  texts: { p: Vec2; rotation: number; align?: string | null; text: string }[];
  ticks: { a: Vec2; b: Vec2; km: string }[];
  sections: { a: Vec2; b: Vec2; km: string }[];
  points: { p: Vec2; km: string }[];
}

/** A route's stations and what is written at them, or why nothing is. */
export const stationing = op<(e: Entity, rules: StationRules, look: StationLook) => { stationing?: Stationing | null; problem?: string | null }>('stationing');

// Orta hat (docs/adr/0190; crates/shared/geometry-core/src/ops/centerline.rs).

/** The axis between two sides: edge by edge between matched sides (with their arcs' bulges), else sampled. */
export interface Centerline {
  readonly method: 'matched' | 'sampled';
  readonly pts: Vec2[];
  readonly bulges?: number[] | null;
}

/** The centreline of two sides sampled every `step` along the longer when they do not match; why none, else. */
export const centerline = op<(a: Entity, b: Entity, step: number) => { centerline?: Centerline | null; problem?: string | null }>('centerline');

// Paralel kaydır (docs/adr/0191; crates/shared/geometry-core/src/ops/edge_shift.rs).

/** An edge picked: its ring (0 the path or the outer ring, 1… the holes), its index, its ends and its normal (outward, or right). */
export interface EdgeShiftPicked {
  readonly ring: number;
  readonly edge: number;
  readonly a: Vec2;
  readonly b: Vec2;
  readonly normal: Vec2;
}

/** The edge of the object nearest `p`, or why it cannot be shifted. */
export const edgeShiftPick = op<(e: Entity, p: Vec2) => { picked?: EdgeShiftPicked | null; problem?: string | null }>('edgeShiftPick');
/** The object with its edge moved `d` along its normal (its other fields kept) and an area's size, or why not. */
export const edgeShift = op<(e: Entity, ring: number, edge: number, d: number) => { entity?: Entity | null; area?: number | null; problem?: string | null }>('edgeShift');
/** The distance for an area to reach `target` m², or why none. */
export const edgeShiftForArea = op<(e: Entity, ring: number, edge: number, target: number) => { distance?: number | null; problem?: string | null }>('edgeShiftForArea');
/** A picture's frame from its lower left corner `p` and a second point `q` (docs/adr/0192 §5), or why none. */
export const imagePlaced = op<(p: Vec2, q: Vec2, aspect: number) => { placed?: { width: number; height: number; rotation: number } | null; problem?: string | null }>('imagePlaced');
/** A boundary drawn in the world as a picture's clip, in its own fractions cut to it (docs/adr/0192 §5), or why none. */
export const imageClip = op<(e: Entity, world: readonly Vec2[]) => { clip?: Vec2[] | null; problem?: string | null }>('imageClip');
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
/**
 * The corner at outer vertex `index` of a polyline or an area, an area's vertices counted part after part (its own
 * ring first, then each other part's, holes never; docs/adr/0143). Null past the last vertex, at an open path's end,
 * beside an arc and on a straight run.
 */
export const pathCornerAt = op<(e: Entity, index: number) => CornerGeom | null>('pathCornerAt');
/**
 * The vertex two picked edges of one path share: each pick's nearest edge, both on one outer ring and neighbours;
 * its index counts as `pathCornerAt`'s. Null for edges that are not neighbours on one ring (a hole's edges, edges of
 * two parts).
 */
export const sharedCorner = op<(e: Entity, a: Vec2, b: Vec2) => number | null>('sharedCorner');
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
 * the side of its far end. A vertex site's `vertex` counts an area's outer
 * vertices part after part (docs/adr/0143), as `pathCornerAt` does.
 */
export const cornerNear = op<(candidates: readonly Entity[], p: Vec2, tol: number, same: number) => { site: CornerSite; corner: CornerGeom } | null>('cornerNear');

/** Angular dimension arms from a vertex and a point on each arm, in the sector `loc` is in. */
export const vertexArms = op<(c: Vec2, p1: Vec2, p2: Vec2, loc: Vec2) => { c: Vec2; a: Vec2; b: Vec2 }>('vertexArms');
/** Angular dimension arms between two picked edges; null for parallel edges. */
export const edgeArms = op<(e1: { a: Vec2; b: Vec2; at: Vec2 }, e2: { a: Vec2; b: Vec2; at: Vec2 }, loc: Vec2) => { c: Vec2; a: Vec2; b: Vec2 } | null>('edgeArms');
/** Radius or diameter dimension placed at `loc`: the point on the circle and the outward offset. */
export const radialDimension = op<(c: Vec2, r: number, loc: Vec2) => { b: Vec2; offset: number }>('radialDimension');
/**
 * Yay uzunluğu's arc from a picked arc edge (docs/adr/0147 §7): its ends counter-clockwise and its centre; with
 * Kısmi the part between two points put on its circle (a point off the arc to its nearer end). Null for a full
 * turn, two points at one place or a point at the centre.
 */
export const arcLengthEnds = op<(edge: Extract<Edge, { kind: 'arc' }>, between: [Vec2, Vec2] | null) => { a: Vec2; b: Vec2; c: Vec2 } | null>('arcLengthEnds');
