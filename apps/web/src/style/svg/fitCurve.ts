import type { Cubic } from './bezier';
import { svgOp } from './core';
import type { Pt } from './pathData';

/**
 * Fits cubic Béziers through a polyline within a tolerance (Schneider,
 * "An Algorithm for Automatically Fitting Digitized Curves", Graphics Gems
 * 1990): chord-length parameters, least-squares handles along fixed end
 * tangents, Newton re-parameterisation, and a split at the worst point
 * when one curve is not enough. Straight runs stay lines and sharp turns
 * stay corners. Computed by the SVG core (crates/shared/svg-core `fit.rs`),
 * which simplify, stroke to path, offsets and tracing use.
 */

/** One run of points as cubics (null: a line, when the run is straight within `tol`). */
export const fitRun = svgOp<(pts: readonly Pt[], tol: number, t1?: Pt, t2?: Pt) => (Cubic | null)[]>('fitRun');

/** The one cubic that best fits the points with these end tangents (node deletion keeping the shape). */
export const fitOne = svgOp<(pts: readonly Pt[], t1: Pt, t2: Pt) => Cubic | null>('fitOne');
