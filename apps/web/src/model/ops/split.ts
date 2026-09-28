import type { Entity } from '../entities';
import { entityOp } from './entityOp';

/**
 * Parçala and Çizimi temizle by the geometry core (docs/adr/0140, 0008).
 * Objects come back with every field but their geometry as they were, so a
 * piece keeps its layer, colour, attributes and label.
 */

/** One object that came apart: its index in the list given, and its pieces in order along it. */
export interface SplitPieces {
  index: number;
  pieces: Entity[];
}

/**
 * Each object of the list cut where the others cross it (an object does not
 * cut itself; arcs stay arcs). Only the objects that came apart are named.
 */
export const splitAtCrossings = entityOp<(list: readonly Entity[]) => SplitPieces[]>('splitAtCrossings');

/** A line, open polyline, arc or circle in `parts` equal pieces (2 to 10 000); null when it cannot be. */
export const splitEqual = entityOp<(e: Entity, parts: number) => Entity[] | null>('splitEqual');

/**
 * Pieces `length` metres long from the start, or from the end (`fromEnd`;
 * an open object only), the last one shorter; null when it cannot be (a
 * length not above zero, one that leaves the object whole, over 10 000 pieces).
 */
export const splitByLength = entityOp<(e: Entity, length: number, fromEnd: boolean) => Entity[] | null>('splitByLength');

/** What Çizimi temizle found in a list of objects. */
export interface CleanupFindings {
  /** Ids of the objects that repeat an earlier one exactly (same layer, kind and geometry to 1e-9 m): they go. */
  repeats: number[];
  /** Ids of the objects with no length or no area: they go. */
  empty: number[];
  /** Objects that repeated a vertex in a row, as they will be. */
  cleaned: Entity[];
  /** The vertices dropped from `cleaned`. */
  vertices: number;
}

export const cleanupFindings = entityOp<(list: readonly Entity[]) => CleanupFindings>('cleanupFindings');
