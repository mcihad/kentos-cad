import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { ArcGeom } from './arc';

/**
 * Constructions behind the drawing tools (rectangles, regular polygons,
 * AutoCAD's arc modes). Pure: points in, rings or arcs out; null when the
 * input cannot make a shape. Computed by the geometry core (docs/adr/0008).
 */

/** Rectangle on edge p1→p2, `width` to its left (negative: right). */
export const rectFromEdge = op<(p1: Vec2, p2: Vec2, width: number) => Vec2[] | null>('rectFromEdge');

/** Signed distance of p from the line p1→p2 (positive on the left). */
export const sideDistance = op<(p1: Vec2, p2: Vec2, p: Vec2) => number>('sideDistance');

/** Rectangle with opposite corners a and b whose sides run at `rotation` (radians, default 0). */
export const rectFromCorners = op<(a: Vec2, b: Vec2, rotation?: number) => Vec2[] | null>('rectFromCorners');

/**
 * Rectangle of given length (along the rotation) and width from corner a,
 * placed in the quadrant (of the rotated frame) that `towards` lies in.
 */
export const rectFromSize = op<(a: Vec2, length: number, width: number, rotation: number, towards: Vec2) => Vec2[] | null>('rectFromSize');

/**
 * Regular n-gon about `center`. Inscribed: `p` is a vertex (the circle
 * passes through the corners). Circumscribed: `p` is the middle of an edge
 * (the circle touches the edges).
 */
export const regularPolygon = op<(center: Vec2, sides: number, p: Vec2, mode: 'inscribed' | 'circumscribed') => Vec2[] | null>('regularPolygon');

/** Regular n-gon built on edge p1→p2, counter-clockwise (to the left of the edge). */
export const regularPolygonOnEdge = op<(p1: Vec2, p2: Vec2, sides: number) => Vec2[] | null>('regularPolygonOnEdge');

/** Start, centre, end direction: counter-clockwise from start to the ray through `end`. */
export const arcStartCenterEnd = op<(start: Vec2, center: Vec2, end: Vec2) => ArcGeom | null>('arcStartCenterEnd');

/** Start, centre, included angle (degrees; negative runs clockwise). */
export const arcStartCenterAngle = op<(start: Vec2, center: Vec2, degrees: number) => ArcGeom | null>('arcStartCenterAngle');

/** Start, centre, chord length (positive: minor arc CCW; negative: major arc). */
export const arcStartCenterChord = op<(start: Vec2, center: Vec2, chord: number) => ArcGeom | null>('arcStartCenterChord');

/** Start, end, included angle (degrees; positive counter-clockwise from start to end). */
export const arcStartEndAngle = op<(start: Vec2, end: Vec2, degrees: number) => ArcGeom | null>('arcStartEndAngle');

/** Start, end, tangent direction at the start. */
export const arcStartEndDirection = op<(start: Vec2, end: Vec2, dir: Vec2) => ArcGeom | null>('arcStartEndDirection');

/** Start, end, radius (positive: minor arc CCW from start to end; negative: major arc). */
export const arcStartEndRadius = op<(start: Vec2, end: Vec2, radius: number) => ArcGeom | null>('arcStartEndRadius');

/** Start, end and centre (the radius comes from the start; the end fixes the direction). */
export const arcStartEndCenter = op<(start: Vec2, end: Vec2, center: Vec2) => ArcGeom | null>('arcStartEndCenter');

/**
 * Scalloped outline of a closed ring: each side is divided into chords of
 * about `arc` metres and every chord bulges outwards.
 */
export const cloudOf = op<(ring: readonly Vec2[], arc: number) => { pts: Vec2[]; bulges: number[] } | null>('cloudOf');
