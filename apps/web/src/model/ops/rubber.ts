import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Kauçuk levha's solution (docs/adr/0158 §2): the core's `ops::rubber`, one for both platforms. The thin plate spline
 * of the links' displacements passes exactly through every link (a fixed point is a link whose target is its source)
 * and follows their affine trend far from them. The independent reference is scripts/fixtures/rubber_cases.py
 * (mpmath, 50 digits).
 */

/** A link: a point of the drawing and where it is to go. */
export interface RubberLink {
  from: Vec2;
  to: Vec2;
}

/** Why there is no sheet: fewer than 3 links, two from one point, sources on one line, no single solution, over 1000. */
export interface RubberFailure {
  error: 'too_few' | 'duplicate' | 'collinear' | 'singular' | 'too_many';
}

/** Where each probe goes and the map's derivative there ([a, b, c, d]: the images of a metre east and north). */
export interface RubberProbes {
  map: Vec2[];
  jacobian: [number, number, number, number][];
}

/** The sheet through `links`, asked where `probes` go; or why there is none. */
export const rubberSheet = op<(links: readonly RubberLink[], probes: readonly Vec2[]) => RubberProbes | RubberFailure>('rubberSheet');
