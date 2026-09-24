import type { Vec2 } from '../model/geometry';
import { op } from '../wasm/core';

/**
 * Ear-clipping triangulation of a ring with holes, in the geometry core
 * (docs/adr/0008): holes are joined to the outer ring by bridges (Eberly,
 * "Triangulation by Ear Clipping") and the ring is clipped. Layers
 * triangulate their fills together (`render/fillQueue.ts`,
 * `triangulateMany`); this is the one-polygon form.
 */
const triangulateOp = op<(outer: readonly Vec2[], holes: readonly (readonly Vec2[])[], origin: Vec2) => number[]>('triangulate');

/** Appends the triangles to `out`, three vertices each as x, y relative to `origin`. */
export function triangulate(outer: readonly Vec2[], holes: readonly (readonly Vec2[])[], origin: Vec2, out: number[]): void {
  for (const v of triangulateOp(outer, holes, origin)) out.push(v);
}
