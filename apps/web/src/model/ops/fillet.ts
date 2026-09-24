import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { ArcGeom } from '../geom/arc';

/** Corners of two lines and of a path's vertex, computed by the geometry core (docs/adr/0008). */

export interface Seg {
  a: Vec2;
  b: Vec2;
}

export type FilletResult = { line1: Seg; line2: Seg; arc: ArcGeom | null } | { error: string };

/**
 * Joins two lines with a tangent arc of radius r (r = 0 makes a sharp
 * corner). The picked side of each line is kept.
 */
export const filletLines = op<(l1: Seg, pick1: Vec2, l2: Seg, pick2: Vec2, r: number) => FilletResult>('filletLines');

export type ChamferResult = { line1: Seg; line2: Seg; cut: Seg | null } | { error: string };

/**
 * Chamfer ("Pah"): cuts the corner of two lines at distance d1 along the
 * first and d2 along the second, measured from their intersection. Zero
 * distances make a sharp corner. The picked side of each line is kept.
 */
export const chamferLines = op<(l1: Seg, pick1: Vec2, l2: Seg, pick2: Vec2, d1: number, d2: number) => ChamferResult>('chamferLines');

export type CornerResult = { pts: Vec2[]; bulges?: number[] } | { error: string };

/**
 * Rounds (`radius`) or chamfers (`d1`, `d2`) the corner at vertex `index`
 * of a polyline or polygon. Both neighbouring segments must be straight;
 * the corner vertex is replaced by the two tangent (or cut) points, joined
 * by an arc segment or a straight cut.
 */
export const cornerOfPath = op<(pts: readonly Vec2[], bulges: readonly number[] | undefined, closed: boolean, index: number, op: { radius: number } | { d1: number; d2: number }) => CornerResult>('cornerOfPath');
