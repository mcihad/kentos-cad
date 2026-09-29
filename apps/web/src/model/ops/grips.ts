import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

/**
 * Grip points of an entity, in a stable order that `moveGrip` understands
 * (computed by the geometry core, docs/adr/0008).
 *   line: a, b · path: vertices, then one mid grip per segment, then
 *   the vertices of each hole (polygon); a multi-part area's part after
 *   part, each so (docs/adr/0143)
 *   circle: centre, 4 quadrants · arc: start, mid, end, centre
 *   point/text: insertion point · spline: fit points
 *   dimension: a, b, dimension line (arc, leader end), vertex (angular) · hatch: ring
 */
export const entityGrips = op<(e: Entity) => Vec2[]>('entityGrips');

/** Entity with grip `index` moved to `p` (same id), or null if the result would be degenerate. */
export const moveGrip = entityOp<<E extends Entity>(e: E, index: number, p: Vec2) => E | null>('moveGrip');

/**
 * Segment index of a path's mid grip (grip indices after the vertices), else null. A multi-part area's grip
 * counts within its own part (`gripPart` says which, docs/adr/0143).
 */
export const midGripSegment = op<(e: Entity, index: number) => number | null>('midGripSegment');

/**
 * Hole and vertex of a polygon's hole grip (after the outer vertices and mid grips), else null. A multi-part
 * area's grip counts within its own part (`gripPart` says which).
 */
export const holeGrip = op<(e: Entity, index: number) => { hole: number; vertex: number } | null>('holeGrip');

/**
 * The part of a multi-part area that grip `index` is on (0: the area's own first part, then `parts[part − 1]`) and
 * the grip's index within that part, as `entityGrips` lists it there (docs/adr/0143). Any other object is all part 0;
 * null past the last grip.
 */
export const gripPart = op<(e: Entity, index: number) => { part: number; index: number } | null>('gripPart');
