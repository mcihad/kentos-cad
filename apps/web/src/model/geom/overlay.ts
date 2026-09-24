import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';

/**
 * Planar overlay of straight and circular edges — the engine behind area
 * booleans, splitting and "click inside to make an area" — in the geometry
 * core (docs/adr/0008, `geom/arrangement.rs`, `geom/overlay.rs`). Edges
 * are cut where they meet, pieces lying on top of each other merge, a rule
 * keeps the pieces that bound the result from the winding numbers on
 * either side, and the kept pieces chain into rings with the result on
 * their left.
 *
 * Arcs stay arcs throughout, and an input vertex keeps its exact input
 * coordinates, so a boolean never moves a surveyed corner.
 */

/** Closed boundary as vertices with DXF bulges (same form as a polygon entity). */
export interface Ring {
  pts: Vec2[];
  bulges?: number[];
}

/** One area: a counter-clockwise outer ring and clockwise holes. */
export interface Area {
  outer: Ring;
  holes: Ring[];
}

/** Overlay input: an area's oriented boundary (interior on the left) or cut lines. */
export interface Source {
  edges: Edge[];
  /**
   * Exact input vertices. Edge ends near them take these coordinates, so
   * the output repeats input corners bit for bit (arc ends recomputed from
   * a centre would drift in the last digits).
   */
  points?: Vec2[];
  /** Cut lines take no part in inside tests; a cut inside the result splits it. */
  cut?: boolean;
}

/**
 * Which pieces the result keeps, from whether a point is inside each
 * source: inside any, all, an odd number of them, the first and no other,
 * the first (a cut splits it), or every bounded face.
 */
export type OverlayRule = 'any' | 'all' | 'odd' | 'firstNotOthers' | 'first' | 'always';

/** Runs the overlay and returns the areas where `rule` holds. */
export const overlay = op<(sources: readonly Source[], rule: OverlayRule) => Area[]>('overlay');

/** A closed walk of the line work: a bounded face (area > 0) or the outline of a connected group (< 0). */
export interface FaceRing {
  ring: Ring;
  area: number;
  /** A point just off the ring on its left (inside a face, outside a group). */
  probe: Vec2;
  /** Bounding box (quick reject before a winding test). */
  box: { minX: number; minY: number; maxX: number; maxY: number };
}

/** Every closed walk formed by the line work. */
export const faceRings = op<(sources: readonly Source[]) => FaceRing[]>('faceRings');

/** Winding number of closed edges around p (0 outside). */
export const winding = op<(edges: readonly Edge[], p: Vec2) => number>('winding');
