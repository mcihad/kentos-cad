import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

/**
 * Uzat-kısalt (AutoCAD LENGTHEN): a line, an arc or an open polyline gets
 * a new total length, changed at one end. Shortening cuts the path there
 * (arcs exactly); lengthening continues the end segment: a straight one
 * along its direction, an arc on its own circle. Computed by the geometry
 * core (docs/adr/0008).
 */

export type LengthenResult = { geometry: EntityGeometry } | { error: string };

/** Current length of what Uzat-kısalt accepts, or null. */
export const lengthOf = op<(e: Entity) => number | null>('lengthOf');

/** Whether a click at p is nearer the end (true) or the start of the entity. */
export const nearEnd = op<(e: Entity, p: Vec2) => boolean>('nearEnd');

export const lengthenEntity = entityOp<(e: Entity, atEnd: boolean, newLength: number) => LengthenResult>('lengthenEntity');

/**
 * The total length that puts the moving end at the point nearest to p:
 * beyond the end the end segment continues (line direction or circle),
 * inside the path the length up to p's projection.
 */
export const lengthToward = op<(e: Entity, atEnd: boolean, p: Vec2) => number | null>('lengthToward');
