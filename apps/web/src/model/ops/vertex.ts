import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

export type VertexResult = { geometry: EntityGeometry } | { error: string };

/**
 * Index of the segment of a line/polyline/polygon nearest to p. Edges within
 * 1e-9 m of each other tie and the first wins: at a shared vertex both edges
 * are nearest, and rounding in an arc's end must not pick between them.
 * Computed by the geometry core (docs/adr/0008).
 */
export const nearestSegment = op<(e: Entity, p: Vec2) => number>('nearestSegment');

/**
 * Adds a vertex on segment `seg` at the point nearest to p. A line becomes
 * a two-segment polyline; an arc segment is split into two arcs that
 * together keep the original curve.
 */
export const insertVertex = entityOp<(e: Entity, seg: number, p: Vec2) => VertexResult>('insertVertex');

/** Removes vertex `index`; its two segments merge into one straight segment. */
export const removeVertex = entityOp<(e: Entity, index: number) => VertexResult>('removeVertex');

/**
 * Whether p is nearer to one of a closed area's holes than to its outer
 * ring; false for anything else. Computed by the geometry core (docs/adr/0047).
 */
export const nearHole = op<(e: Entity, p: Vec2) => boolean>('nearHole');
