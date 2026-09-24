import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Ellipse and elliptical arc in the DXF ELLIPSE form: centre, major axis
 * (vector from the centre to one end), ratio minor/major (0 < ratio ≤ 1)
 * and parameters t0 → t1, counter-clockwise; equal parameters mean the
 * whole ellipse. P(t) = c + major·cos t + minor·sin t, where minor is the
 * major axis turned +90° and scaled by ratio.
 *
 * An affine map takes the ellipse to the unit circle, so line crossings
 * and tangents are exact; the closest point is found with Newton steps.
 * Computed by the geometry core (docs/adr/0008).
 */
export interface EllipseGeom {
  c: Vec2;
  major: Vec2;
  ratio: number;
  t0: number;
  t1: number;
}

export const minorAxis = op<(e: EllipseGeom) => Vec2>('minorAxis');
export const majorLength = op<(e: EllipseGeom) => number>('majorLength');
export const ellipseSweep = op<(e: EllipseGeom) => number>('ellipseSweep');
export const isFullEllipse = op<(e: EllipseGeom) => boolean>('isFullEllipse');

export const ellipsePoint = op<(e: EllipseGeom, t: number) => Vec2>('ellipsePoint');

/** Derivative dP/dt (not normalised). */
export const ellipseDerivative = op<(e: EllipseGeom, t: number) => Vec2>('ellipseDerivative');

/** Parameter of a point on (or projected radially onto) the ellipse. */
export const paramOfPoint = op<(e: EllipseGeom, p: Vec2) => number>('paramOfPoint');

/** Parameter of the ellipse point seen from the centre at `angle` (radians) from the major axis. */
export const paramAtPolar = op<(e: EllipseGeom, angle: number) => number>('paramAtPolar');

/** Whether parameter t lies on the (arc) range (within `eps`, 1e-9 by default). */
export const onEllipse = op<(e: EllipseGeom, t: number, eps?: number) => boolean>('onEllipse');

/** Point list along the curve (128 a turn by default); a whole ellipse returns a ring (first point not repeated). */
export const tessellateEllipse = op<(e: EllipseGeom, perTurn?: number) => Vec2[]>('tessellateEllipse');

/** Arc length (composite Simpson, far below drawing precision). */
export const ellipseLength = op<(e: EllipseGeom) => number>('ellipseLength');

/** Area of a whole ellipse (π·a·b). */
export const ellipseArea = op<(e: EllipseGeom) => number>('ellipseArea');

/**
 * Parameter of the point on the curve nearest to p (within the arc range).
 * The nearest of 65 samples tells which way the distance falls; the foot
 * between it and the next sample that way is found by Newton steps on
 * f(t) = (P(t) − p)·P′(t), bisecting whenever a step would leave that
 * bracket, so the search always converges. When the distance only grows
 * past an arc end, the end is the answer.
 */
export const closestParam = op<(e: EllipseGeom, p: Vec2) => number>('closestParam');

/** Crossings of the infinite line a→b with the curve: line parameter and ellipse parameter. */
export const lineEllipse = op<(e: EllipseGeom, a: Vec2, b: Vec2) => { u: number; t: number }[]>('lineEllipse');

/** Points where lines from p touch the curve (empty when p is inside). */
export const ellipseTangentPoints = op<(e: EllipseGeom, p: Vec2) => Vec2[]>('ellipseTangentPoints');

/** Parameters of the four axis ends (quadrant snaps) that lie on the curve. */
export const quadrantParams = op<(e: EllipseGeom) => number[]>('quadrantParams');

/** Whether p is inside a whole ellipse. */
export const insideEllipse = op<(e: EllipseGeom, p: Vec2) => boolean>('insideEllipse');

/**
 * Ellipse through an axis (two ends) and the distance from the centre to
 * the other axis — AutoCAD's default construction. The longer axis is the
 * major one.
 */
export const ellipseFromAxis = op<(p1: Vec2, p2: Vec2, otherHalf: number) => EllipseGeom | null>('ellipseFromAxis');

/** Ellipse from its centre, one axis end and the other half-axis length. */
export const ellipseFromCenter = op<(c: Vec2, axisEnd: Vec2, otherHalf: number) => EllipseGeom | null>('ellipseFromCenter');
