import { op } from '../../wasm/core';

/**
 * The attribute table's rows (docs/adr/0199 §4): the core's `ops::feature_table`, one for both platforms. A row
 * arrives as its cells (the text each shows and its sort key when its value keeps its field's rules), whether it is
 * selected, in the view and kept by the expression filter; the answer is the rows shown, by their index, in the order
 * shown. The independent reference is scripts/fixtures/feature_table_cases.py; `../featureTable.ts` makes the rows.
 */

/** How a column's keys are ordered: exact numbers, the calendar, false before true, the natural order. */
export type ColumnOrder = 'text' | 'number' | 'date' | 'boolean';

/** A column: how its keys are ordered and whether the search looks in it. */
export interface TableColumn {
  order: ColumnOrder;
  searched: boolean;
}

/** A cell: the text it shows, and its sort key when its value is there and keeps its field's rules. */
export interface TableCell {
  shown: string;
  key?: string | null;
}

/** An object of the layer as the table reads it. */
export interface TableRow {
  cells: TableCell[];
  selected: boolean;
  inView: boolean;
  passes: boolean;
}

/** Göster: every row, the selected ones, the ones in the view. */
export type TableShow = 'all' | 'selected' | 'inView';

/** What the table shows: the search box's text, Göster, the column it is sorted by (null: the drawing's order) and which way. */
export interface TableQuery {
  search: string;
  show: TableShow;
  sort: number | null;
  descending: boolean;
}

/** The rows shown, by their index in `rows`, in the order shown. */
export const featureTable = op<(columns: readonly TableColumn[], rows: readonly TableRow[], query: TableQuery) => number[]>('featureTable');
