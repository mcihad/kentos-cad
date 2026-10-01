import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Elevated } from './elevation';

/**
 * Köşelere nokta (docs/adr/0152 §5): the core's `ops::vertex_points`. A named point at every vertex of the given
 * paths, objects in the drawing's order; a place two vertices share is one point, its elevation the first given; a
 * place an existing point holds is passed over and counted; names run from the first by Yazı's Artır. The independent
 * reference is scripts/fixtures/vertex_points_cases.py.
 */

export interface VertexPoint {
  p: Vec2;
  z?: number | null;
  name?: string | null;
}

export interface VertexPoints {
  points: VertexPoint[];
  /** The places passed over because a point is there already. */
  skipped: number;
  /** The name after the last point's: where the next run starts. */
  next?: string | null;
}

/** The points for `objects` (each its paths with their elevations), past `existing` points, named from `first`. */
export const vertexPoints = op<(objects: readonly (readonly Elevated[])[], existing: readonly Vec2[], first: string | null) => VertexPoints>('vertexPoints');
