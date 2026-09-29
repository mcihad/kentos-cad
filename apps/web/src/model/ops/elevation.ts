import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Elevations carried to an edit's new vertices (docs/adr/0142): the core's
 * `ops::elevation`. A vertex on a source vertex takes its elevation; one of
 * the object the edit replaces, with as many vertices, the one in its place;
 * one on a source edge the edge's, linearly along it; an open result's end on
 * a source's straight extension its grade carried on; for Ötele the closest
 * point's. Any other vertex has none (null, not 0).
 */

/** A path of an object before an edit, with its elevations (the core's `Elevated`). */
export interface Elevated {
  pts: Vec2[];
  bulges?: number[];
  closed: boolean;
  zs: (number | null)[];
}

/** Elevations for the vertices `pts` of an edit's result (`closed`: a ring), from `sources` and `same`. */
export const carryElevations = op<(pts: Vec2[], closed: boolean, same: Elevated | null, sources: Elevated[], offset: boolean) => (number | null)[]>('carryElevations');
