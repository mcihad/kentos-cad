import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Topological editing (docs/adr/0160): the core's `ops::topology_edit`, one for both platforms. The neighbours of an
 * edited vertex or edge are put right with it so that shared corners and edges stay shared; nothing is computed, the
 * vertices take the changes' points and the edges their bulges. The independent reference is
 * scripts/fixtures/topology_edit_cases.py.
 */

/** What an edit did at shared places: the moves happen together, the other changes one after the other. */
export type TopologyChange =
  | { kind: 'move'; at: Vec2; to: Vec2 }
  | { kind: 'insert'; a: Vec2; b: Vec2; p: Vec2 }
  | { kind: 'bulge'; a: Vec2; b: Vec2; from: number; to: number }
  | { kind: 'remove'; at: Vec2; prev: Vec2; next: Vec2 };

/** An object near the edited one: its shape and whether its layer is locked. */
export interface TopologyNeighbour<S> {
  shape: S;
  locked?: boolean;
}

/** The neighbours put right (their indices and shapes), how many would have changed but are locked or left invalid. */
export interface TopologyAnswer<S> {
  edited: { index: number; shape: S }[];
  locked: number;
  invalid: number;
}

export const topologyEdit = op<<S>(neighbours: readonly TopologyNeighbour<S>[], changes: readonly TopologyChange[], points: boolean) => TopologyAnswer<S>>('topologyEdit');

/** The changes an edit made, from the edited object before and after it (moves first). */
export const topologyChanges = op<<S>(before: S, after: S) => TopologyChange[]>('topologyChanges');
