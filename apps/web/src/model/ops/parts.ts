import type { Entity } from '../entities';
import { entityOp } from './entityOp';

/**
 * A multi-part area's parts (docs/adr/0143), computed by the geometry core: the parts as areas of their own, and
 * areas as one. An area with parts past its first has its own fields for the first part and `parts` for the rest;
 * an area without them is one part. Which part a grip is on is `gripPart` (./grips), the areas of each part
 * `areasOfEntity` (./areas).
 */

/**
 * Each part of an area as an area of its own, the object's other fields kept (its layer, colour, attributes,
 * label …), in the parts' order; an area of one part, or anything that is not an area, comes back as itself.
 */
export const splitParts = entityOp<<E extends Entity>(e: E) => E[]>('splitParts');

/**
 * The areas as one, their parts in order (an area that already has parts gives all of them), with the first
 * one's other fields; null for no area or when one of them is not an area. One area with one part is itself.
 */
export const joinParts = entityOp<<E extends Entity>(list: readonly E[]) => E | null>('joinParts');

/**
 * `e` with its part `k` (0: its own first part) replaced by `part`, a one-part area, the other parts kept in their
 * places (the grip menu's edit of one part); null when `e` has no part `k` or `part` is no area. The result has
 * `e`'s other fields.
 */
export function replacePart<E extends Entity>(e: E, k: number, part: E): E | null {
  const parts = splitParts(e);
  if (k < 0 || k >= parts.length) return null;
  parts[k] = part;
  return joinParts(parts);
}
