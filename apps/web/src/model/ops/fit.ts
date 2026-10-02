import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Vektör oturtma's solution (docs/adr/0156 §2–§3): the core's `ops::fit`, one for both platforms. The independent
 * reference is scripts/fixtures/fit_cases.py (exact fractions).
 */

/** A control point: where it is in the drawing, where it is to go, whether it takes part. */
export interface FitPair {
  source: Vec2;
  target: Vec2;
  used: boolean;
}

export type FitKind = 'helmert' | 'affine' | 'projective';

/** The fewest used pairs each kind needs. */
export const FIT_NEED: Record<FitKind, number> = { helmert: 2, affine: 3, projective: 4 };

/**
 * A solution: the centres (`from` of the used sources, `to` of their targets), the numbers between the centred frames
 * (helmert a, b; affine a, b, c, d; projective a1, a2, a3, b1, b2, b3, c1, c2), every pair's residual (transformed
 * source less target: x, y, length), m0 (null without redundancy) and the kind's derived values (radians).
 */
export interface Fit {
  from: Vec2;
  to: Vec2;
  params: number[];
  residuals: [number, number, number][];
  m0: number | null;
  scale?: number;
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  shear?: number;
}

/** Why there is none: too few used pairs (and how many the kind needs), sources in one place, on one line, or no single solution. */
export interface FitFailure {
  error: 'too_few' | 'coincident' | 'collinear' | 'singular';
  need?: number;
}

/** The transform of `kind` that best carries the used pairs' sources onto their targets, or why there is none. */
export const fitTransform = op<(pairs: readonly FitPair[], kind: FitKind) => Fit | FitFailure>('fitTransform');
