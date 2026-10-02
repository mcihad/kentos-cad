import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Vektör oturtma's warping of objects (docs/adr/0156 §4–§5): the core's `ops::warp`, one for both platforms. A
 * similarity moves every kind as the modify tools do; otherwise straight geometry maps exactly, curves become the exact
 * ellipse of their image (affine) or straight vertices within 0.1 mm (projective, and a path's arc segments), and
 * texts, notes, blocks, dimensions and hatch patterns keep their shape. The independent reference is
 * scripts/fixtures/warp_cases.py.
 */

/** A transform in `cad.entities.transform`'s centred form: `from` the source centre, `to` the target's. */
export type Warp =
  | { kind: 'similarity'; from: Vec2; to: Vec2; a: number; b: number }
  | { kind: 'affine'; from: Vec2; to: Vec2; m: [number, number, number, number] }
  | { kind: 'projective'; from: Vec2; to: Vec2; h: [number, number, number, number, number, number, number, number] };

/** A shape's paths' elevations: outer, holes, then each part's ring and holes; a line's two ends. */
export type PathElevations = (number | null)[][];

/**
 * Every shape warped, with its paths' elevations; how many curves became straight vertices and how many texts, notes,
 * blocks, dimensions and hatch patterns kept their shape. A shape is an object's geometry as the core reads it (its
 * other fields are not read). A curve can come back of another kind: a circle as an ellipse or a polyline.
 */
export interface Warped<S> {
  shapes: S[];
  zs: PathElevations[];
  curves: number;
  kept: number;
}

/** A point of a shape beyond a projective transform's horizon: nothing warped, the shape's index named. */
export interface WarpRefusal {
  error: 'beyond_horizon';
  at: number;
}

export const warpShapes = op<<S>(shapes: readonly S[], zs: readonly PathElevations[], warp: Warp) => Warped<S> | WarpRefusal>('warpShapes');
