import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Biçim değiştir, Sürdür and the hole operations (docs/adr/0173): the core's `ops::reshape_by`, `ops::continuation` and `ops::holes`, one for both
 * platforms. Each gives the object (`done`: its geometry; the caller keeps its other fields) or why not; the tools say
 * the reason in their own words. The independent reference is scripts/fixtures/reshape_cases.py.
 */

/** Why Biçim değiştir is refused. */
export type ReshapeRefusal = { why: 'notShape' } | { why: 'tooShort' } | { why: 'noCrossing' } | { why: 'manyParts' } | { why: 'touchesHole' } | { why: 'bothWays' } | { why: 'apart' } | { why: 'multiPart' };

/** Why a hole is not added, removed or filled. */
export type HoleRefusal = { why: 'notArea' } | { why: 'degenerate' } | { why: 'outside' } | { why: 'splits' } | { why: 'notInHole' };

/** A ring in vertex and bulge form. */
export interface Ring {
  pts: Vec2[];
  bulges?: number[];
}

export type Answer<T, R> = { done: T } | { refusal: R };

/** The object reshaped by the sketch (its points in order): an area cut or grown, a path's stretch redrawn. */
export const reshapeBy = op<(e: Entity, sketch: readonly Vec2[]) => Answer<Entity, ReshapeRefusal>>('reshapeBy');

/** The area with a hole `ring` added to the part that holds it (merged with the holes it meets). */
export const holeAdd = op<(e: Entity, ring: Ring) => Answer<Entity, HoleRefusal>>('holeAdd');

/** The hole that holds `p`: its part (0: the area's own fields) and its place among the part's holes; null when none. */
export const holeAt = op<(e: Entity, p: Vec2) => { part: number; hole: number } | null>('holeAt');

/** The ring of the hole that holds `p` (Deliği doldur's new area). */
export const holeRing = op<(e: Entity, p: Vec2) => Answer<Ring, HoleRefusal>>('holeRing');

/** The area without the hole that holds `p` (Deliği sil). */
export const holeRemove = op<(e: Entity, p: Vec2) => Answer<Entity, HoleRefusal>>('holeRemove');

/** A line's or an open polyline's two ends and the directions out of them (Sürdür); null for any other object. */
export const pathEnds = op<(e: Entity) => { first: Vec2; last: Vec2; outFirst?: Vec2; outLast?: Vec2 } | null>('pathEnds');

/**
 * The line or open polyline continued from its first end (`fromFirst`) or its last by the drawn path (its first point
 * the end itself), one bulge per drawn segment (Sürdür); null when it is no such object or nothing is drawn beyond the
 * end.
 */
export const continuePath = op<(e: Entity, fromFirst: boolean, drawn: readonly Vec2[], bulges: readonly number[]) => Entity | null>('continuePath');
