import { op } from '../../wasm/core';
import type { CellRange, TableEntity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Tablo (docs/adr/0184): the core's `geom::table`, `ops::table` and `ops::table_edit`, one rule for both platforms.
 * Where a table's lines, frame and words go, how wide its columns must be, the schedules (koordinat, alan and öznitelik
 * çizelgesi), a file's rows as cells, Tabloyu düzenle's edits and Tabloyu güncelle's rule. The independent reference is
 * scripts/fixtures/table_cases.py (fixtures/table/v1/cases.json).
 */

/** A table as the core reads it: its shape fields (`geometryOf`'s), `kind: 'table'`. */
export type TableGeometry = Pick<TableEntity, 'kind' | 'p' | 'rotation' | 'height' | 'rows' | 'columns' | 'cells'> &
  Partial<Pick<TableEntity, 'merges' | 'aligns' | 'header' | 'grid' | 'frame' | 'font' | 'bold' | 'italic' | 'oblique' | 'source'>>;

/** A cell with words: where its baseline starts, how wide its words are, bold when a heading row's (or the face's). */
export interface CellPlace {
  readonly row: number;
  readonly col: number;
  readonly at: Vec2;
  readonly width: number;
  readonly bold: boolean;
}

/** Where a table's parts go: its corners (top left first), its lines, its frame's four strips, its cells with words. */
export interface TableLayout {
  readonly outline: readonly Vec2[];
  readonly lines: readonly (readonly [Vec2, Vec2])[];
  readonly frame: readonly (readonly Vec2[])[];
  readonly cells: readonly CellPlace[];
}

/** A table's cells as a schedule or a file gives them: their columns' alignment, how many columns were fitted, why not. */
export interface TableCells {
  readonly cells: string[][];
  readonly aligns: string[];
  readonly fitted: number;
  readonly problem?: string | null;
}

/** What a schedule reads of an object: its shape, its paths with their elevations, its label and attributes. */
export interface ListedObject {
  readonly shape: Record<string, unknown>;
  readonly paths?: readonly { pts: readonly Vec2[]; closed: boolean; zs?: readonly (number | null)[] }[];
  readonly label: string | null;
  readonly attrs: Readonly<Record<string, string>>;
}

/** The project's settings a schedule writes its numbers by. */
export interface TableUnits {
  readonly axes: 'cad' | 'gis';
  readonly unit: 'm' | 'cm' | 'mm';
  readonly areaUnit: 'm2' | 'donum' | 'ha';
  readonly lengthDecimals: number;
  readonly areaDecimals: number;
}

export type ScheduleKind = 'coordinates' | 'areas' | 'attributes';

/** One of Tabloyu düzenle's edits. */
export type TableEdit =
  | { kind: 'setCell'; row: number; col: number; words: string }
  | { kind: 'insertRows'; at: number; count: number }
  | { kind: 'deleteRows'; from: number; count: number }
  | { kind: 'insertColumns'; at: number; count: number }
  | { kind: 'deleteColumns'; from: number; count: number }
  | ({ kind: 'merge' } & CellRange)
  | { kind: 'unmerge'; row: number; col: number };

/** An edit's table, or why it was refused (the other absent). */
export interface TableEdited {
  readonly table?: TableGeometry;
  readonly problem?: string;
}

/** Where a table's parts go, its words measured in `font` (the drawing font's id); null for another shape. */
export const tableLayout = op<(table: TableGeometry, font: string) => TableLayout | null>('tableLayout');

/** How high each row and how wide each column must be for its words (Sığdır). */
export const tableSizes = op<(table: TableGeometry, font: string) => { rows: number[]; columns: number[] } | null>('tableSizes');

/** A schedule's cells from the objects it lists, in the order given. */
export const tableSchedule = op<(kind: ScheduleKind, objects: readonly ListedObject[], units: TableUnits) => TableCells>('tableSchedule');

/** A file's rows as cells: the empty rows and columns after the last with words dropped, the rest padded. */
export const tableFromRows = op<(rows: readonly (readonly string[])[], header: boolean) => TableCells>('tableFromRows');

/** A cell's words on one line: line breaks and tabs as spaces; `changed` when there were any. */
export const tableOneLine = op<(words: string) => { words: string; changed: boolean }>('tableOneLine');

/** A table with one of Tabloyu düzenle's edits made, or why not. */
export const tableEdit = op<(table: TableGeometry, edit: TableEdit) => TableEdited>('tableEdit');

/** A table with its source's new cells and their columns' alignment (Tabloyu güncelle); null for another shape. */
export const tableRefresh = op<(table: TableGeometry, cells: readonly (readonly string[])[], aligns: readonly string[], font: string) => TableGeometry | null>('tableRefresh');
