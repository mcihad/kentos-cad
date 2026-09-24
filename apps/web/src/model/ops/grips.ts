import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

/**
 * Grip points of an entity, in a stable order that `moveGrip` understands
 * (computed by the geometry core, docs/adr/0008).
 *   line: a, b · path: vertices, then one mid grip per segment, then
 *   the vertices of each hole (polygon)
 *   circle: centre, 4 quadrants · arc: start, mid, end, centre
 *   point/text: insertion point · spline: fit points
 *   dimension: a, b, dimension line (arc, leader end), vertex (angular) · hatch: ring
 */
export const entityGrips = op<(e: Entity) => Vec2[]>('entityGrips');

/** Entity with grip `index` moved to `p` (same id), or null if the result would be degenerate. */
export const moveGrip = entityOp<<E extends Entity>(e: E, index: number, p: Vec2) => E | null>('moveGrip');

/** Segment index of a path's mid grip (grip indices after the vertices), else null. */
export const midGripSegment = op<(e: Entity, index: number) => number | null>('midGripSegment');

/** Hole and vertex of a polygon's hole grip (after the outer vertices and mid grips), else null. */
export const holeGrip = op<(e: Entity, index: number) => { hole: number; vertex: number } | null>('holeGrip');
