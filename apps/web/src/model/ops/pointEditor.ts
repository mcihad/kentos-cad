import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Elevated } from './elevation';

/**
 * Nokta editörü's computations (docs/adr/0153 §6): the core's `ops::point_editor` and `text::natural`, one for both
 * platforms. The independent reference is scripts/fixtures/point_editor_cases.py.
 */

/** A point as the table reads it: name (label) and code as written, place (Y east, X north), elevation, layer name. */
export interface TableRow {
  name: string | null;
  east: number;
  north: number;
  z: number | null;
  code: string | null;
  layer: string;
  selected: boolean;
}

/** A column the table sorts by. */
export type SortColumn = 'name' | 'east' | 'north' | 'z' | 'code' | 'layer';

/** What the table shows: the search box's text, one layer (null: all), only the selected, the sort (null: drawing order). */
export interface TableQuery {
  search: string;
  layer: string | null;
  onlySelected: boolean;
  sort: SortColumn | null;
  descending: boolean;
}

/** The rows shown, by their index in `rows`, in the order shown: the filters, then the column's order (ties: the drawing's). */
export const pointTable = op<(rows: readonly TableRow[], query: TableQuery) => number[]>('pointTable');

/** `names` in the natural order (P2 before P10), as their indices; equal names keep their order. */
export const naturalOrder = op<(names: readonly string[]) => number[]>('naturalOrder');

export interface DupPoint {
  p: Vec2;
  z: number | null;
  name: string | null;
}

export interface DupGroup {
  /** Its members' indices, in the drawing's order. */
  members: number[];
  kept: number;
  /** Where the one kept stands after: its own place and elevation, or the members' mean. */
  p: Vec2;
  z?: number | null;
}

export interface Duplicates {
  groups: DupGroup[];
  /** The points that go, in the drawing's order. */
  removed: number[];
}

/** Çift noktaları ayıkla: groups by name or by place within `tolerance` metres; each keeps its first, last or mean. */
export const duplicatePoints = op<(points: readonly DupPoint[], by: 'name' | 'place', tolerance: number, keep: 'first' | 'last' | 'average') => Duplicates>('duplicatePoints');

/**
 * The paths of a line, polyline or area (`elevatedPaths`' order) with the vertices within 1 µm of `from` moved to `to`,
 * and with `setZ` given the elevation `z` (null: none); null when no vertex is there or nothing would change.
 */
export const followPoint = op<(paths: readonly Elevated[], from: Vec2, to: Vec2, setZ: boolean, z: number | null) => Elevated[] | null>('followPoint');
