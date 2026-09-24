import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import type { Edge } from '../geom/intersect';
import { entityOp } from './entityOp';

/**
 * An entity seen as a path parameterised by arc length s ∈ [0, L]. Trim,
 * break, divide and measure all work on s values, so every kind that
 * provides edges (lines, bulged polylines, arcs, circles) behaves the same.
 * Computed by the geometry core (docs/adr/0008).
 */
export interface Path {
  edges: Edge[];
  /** Arc length at the start of each edge. */
  cum: number[];
  length: number;
  closed: boolean;
}

export const pathOf = op<(e: Entity) => Path | null>('pathOf');

/** Wraps s into [0, L) on closed paths, clamps it on open ones. */
export const normS = op<(path: Path, s: number) => number>('normS');

export const pointAtS = op<(path: Path, s: number) => Vec2>('pointAtS');

/** Unit travel direction at s. */
export const tangentAtS = op<(path: Path, s: number) => Vec2>('tangentAtS');

/** Arc length of the point on the path nearest to p. */
export const nearestS = op<(path: Path, p: Vec2) => number>('nearestS');

/** Sorted, de-duplicated s values where `boundaries` cross the path (ends excluded when open). */
export const cutsOn = op<(path: Path, boundaries: readonly Edge[]) => number[]>('cutsOn');

/**
 * Geometry of the sub-path from s0 to s1 (s1 may exceed L on closed
 * paths). Pieces of lines stay lines, of arcs and circles arcs, and of
 * polylines polylines with their arc segments cut exactly.
 */
export const subPath = entityOp<(path: Path, s0: number, s1: number, source: Entity) => EntityGeometry>('subPath');

/** Evenly spaced s values: n parts (points between them) or every `step` from the start. */
export const divisionParams = op<(path: Path, opts: { parts: number } | { step: number }) => number[]>('divisionParams');

/**
 * The Böl tool's points in one call: division parameters along the path,
 * counted from the end when `fromEnd`; an ellipse's points are placed on
 * the true curve (its path is fine chords). Returns [] without a path.
 */
export const divisionPoints = op<(e: Entity, opts: { parts: number } | { step: number }, fromEnd: boolean) => Vec2[]>('divisionPoints');
