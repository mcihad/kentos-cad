import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

export interface EdgeLabel {
  /** Text anchor (baseline centre). */
  p: Vec2;
  /** Degrees CCW from east, always readable (−90° < r ≤ 90°). */
  rotation: number;
  /** Edge length in metres. */
  length: number;
  /** Segment index: the edge from pts[index] to pts[index + 1]. */
  index: number;
}

/**
 * Placement of edge-length labels ("kenar ölçüleri") for a parcel or
 * polyline: centred on each edge, lifted to the outside of a closed ring
 * (to the left of an open path) by a gap proportional to text height;
 * `side: 'inside'` puts them inside the ring (right of an open path).
 * Arc segments are labelled with their arc length at the arc's midpoint.
 * Computed by the geometry core (docs/adr/0008); `minLength` defaults to 0.
 */
export const edgeLabels = op<(pts: readonly Vec2[], closed: boolean, height: number, minLength?: number, bulges?: readonly number[], side?: 'outside' | 'inside') => EdgeLabel[]>('edgeLabels');
