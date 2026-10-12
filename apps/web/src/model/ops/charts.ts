import { CoreChartWork } from '../../wasm/core';

/**
 * Grafikler ve alan istatistikleri (docs/adr/0227, `kentos.charts/1`): the core's `ops::charts` through its WASM work
 * object, one rule for both platforms. A data set is rows and columns (../chartData.ts reads them from a layer or a
 * table); the core counts a field's statistics, works a chart out, lays it out for a frame as a drawing list and
 * answers clicks. The independent reference is scripts/fixtures/chart_cases.py.
 */

export type ColumnKind = 'number' | 'text' | 'date' | 'boolean';

/** A cell: empty, a value (its canonical text, what it shows when that differs) or one its kind refuses. */
export type ChartCell = null | { readonly v: string; readonly s?: string } | { readonly invalid: string };

/** A column of a data set: its cells, or a geometry measure's doubles (NaN: none). */
export interface DataColumn {
  readonly key: string;
  readonly label: string;
  readonly kind: ColumnKind;
  readonly labelled: boolean;
  readonly cells?: readonly ChartCell[];
  readonly measures?: Float64Array;
}

/** The geometry measures' keys (docs/adr/0227 §2). */
export const AREA_KEY = '$alan';
export const LENGTH_KEY = '$uzunluk';

/** A column's row in the İstatistik tab's summary table. */
export interface FieldSummary {
  readonly key: string;
  readonly label: string;
  readonly kind: string;
  readonly objects: number;
  readonly filled: number;
  readonly empty: number;
  readonly invalid: number;
  readonly distinct: number;
  readonly min?: string;
  readonly max?: string;
  readonly mean?: string;
  readonly sum?: string;
}

export interface FieldStat {
  readonly label: string;
  readonly value: string;
}

/** A row of a field's value list: a value, the refused ones or the empty ones. */
export interface ValueRow {
  readonly text: string;
  readonly count: number;
  readonly share: string;
  readonly mark: 'value' | 'invalid' | 'empty';
}

export interface FieldDetail {
  readonly summary: FieldSummary;
  readonly stats: readonly FieldStat[];
  readonly values: readonly ValueRow[];
  readonly more: number;
  readonly notes: readonly string[];
}

export type ChartKind = 'bar' | 'pie' | 'histogram' | 'box' | 'scatter' | 'line';
export type Aggregate = 'none' | 'count' | 'sum' | 'mean' | 'min' | 'max' | 'median';
export type BarSort = 'category' | 'desc' | 'asc';

/** What to chart: the columns by key and the choices (docs/adr/0227 §4). */
export interface ChartSpec {
  kind: ChartKind;
  category?: string;
  value?: string;
  aggregate?: Aggregate;
  sort?: BarSort;
  /** Histogram's intervals; none: Sturges. */
  bins?: number;
  guides?: boolean;
  group?: string;
  x?: string;
  y?: string[];
  series?: string;
  trend?: boolean;
}

/** A chart worked out: its words and its table (Tablo, Panoya kopyala). */
export interface ChartInfo {
  readonly kind: ChartKind;
  readonly xTitle: string;
  readonly yTitle: string;
  readonly notes: readonly string[];
  readonly elements: number;
  readonly table: { readonly columns: readonly string[]; readonly rows: readonly (readonly string[])[] };
}

/** A colour by its role (the platform resolves it from the theme). */
export type ChartRole = 'surface' | 'grid' | 'axis' | 'ink1' | 'ink2' | 'ink3' | 'other' | 'selection' | `series${number}`;

/** The drawing list's primitives (crates/shared/geometry-core/src/ops/charts/layout.rs). */
export type ChartPrim =
  | { t: 'rect'; x: number; y: number; w: number; h: number; rt: number; rb: number; fill: ChartRole; a: number; e: number }
  | { t: 'line'; x1: number; y1: number; x2: number; y2: number; stroke: ChartRole; w: number; a: number }
  | { t: 'poly'; pts: number[]; closed: boolean; stroke: ChartRole; w: number; a: number }
  | { t: 'circle'; x: number; y: number; r: number; fill?: ChartRole; stroke?: ChartRole; w: number; a: number; e: number }
  | { t: 'wedge'; cx: number; cy: number; r0: number; r1: number; a0: number; a1: number; fill: ChartRole; a: number; e: number }
  | { t: 'cross'; x: number; y: number; s: number; stroke: ChartRole; w: number }
  | { t: 'text'; x: number; y: number; text: string; size: number; weight: number; align: 'start' | 'middle' | 'end'; baseline: 'top' | 'middle' | 'bottom'; color: ChartRole }
  | { t: 'points'; start: number; count: number; r: number; ring: boolean; fill: ChartRole; a: number };

export interface ChartLayout {
  readonly plot: readonly number[];
  readonly prims: readonly ChartPrim[];
}

/** The room a chart is laid out in: CSS pixels, its text's size and the interface's typeface. */
export interface ChartFrame {
  width: number;
  height: number;
  font: number;
  family: string;
}

/** The colours of a saved picture (`#rrggbb`) and its typeface. */
export interface ChartPalette {
  background: string;
  surface: string;
  grid: string;
  axis: string;
  ink1: string;
  ink2: string;
  ink3: string;
  series: string[];
  other: string;
  selection: string;
  font: string;
}

/** Joins texts for the boundary: their UTF-16 lengths, −1 for none. */
function joined(texts: readonly (string | null)[]): { text: string; lens: Int32Array } {
  const lens = new Int32Array(texts.length);
  const parts: string[] = [];
  texts.forEach((t, i) => {
    lens[i] = t === null ? -1 : t.length;
    if (t !== null) parts.push(t);
  });
  return { text: parts.join(''), lens };
}

/** A data set in the core: built once, asked many times; `free` releases it. */
export class ChartData {
  private readonly work: CoreChartWork;
  readonly rows: number;
  readonly columns: readonly DataColumn[];

  /**
   * `geometry`: the objects' measure records from the geometry store (`ctx.view.geometry.measures`), which add the
   * columns Alan and Uzunluk after `columns` (their keys `$alan`, `$uzunluk`), in the unit `perMetre` of which make a
   * metre (docs/adr/0165 §2: m², or mm² in a project in millimetres).
   */
  constructor(rows: number, columns: readonly DataColumn[], geometry?: Float64Array, perMetre = 1) {
    this.rows = rows;
    this.work = new CoreChartWork(rows);
    for (const c of columns) {
      if (c.measures) {
        this.work.addMeasures(c.key, c.label, c.measures);
        continue;
      }
      const cells = c.cells ?? [];
      const values: (string | null)[] = [];
      const shown: (string | null)[] = [];
      const refused: number[] = [];
      cells.forEach((cell, i) => {
        if (cell === null) {
          values.push(null);
          shown.push(null);
        } else if ('invalid' in cell) {
          values.push(cell.invalid);
          shown.push(null);
          refused.push(i);
        } else {
          values.push(cell.v);
          shown.push(cell.s ?? null);
        }
      });
      const v = joined(values);
      const s = joined(shown);
      this.work.addColumn(c.key, c.label, c.kind, c.labelled, v.text, v.lens, s.text, s.lens, Uint32Array.from(refused));
    }
    const all: DataColumn[] = [...columns];
    if (geometry) {
      // Each where an object has one: a layer of lines has no area, one of points neither.
      const added = this.work.addGeometry(geometry, perMetre);
      const mark = perMetre === 1000 ? 'mm' : perMetre === 100 ? 'cm' : 'm';
      if (added.includes(AREA_KEY)) all.push({ key: AREA_KEY, label: `Alan (${mark}²)`, kind: 'number', labelled: false });
      if (added.includes(LENGTH_KEY)) all.push({ key: LENGTH_KEY, label: `Uzunluk (${mark})`, kind: 'number', labelled: false });
    }
    this.columns = all;
    this.work.seal();
  }

  summary(column: number, rows: Uint32Array): FieldSummary {
    return this.work.summary(column, rows) as FieldSummary;
  }

  detail(column: number, rows: Uint32Array): FieldDetail {
    return this.work.detail(column, rows) as FieldDetail;
  }

  /** The rows of the last statistics' value row `i`. */
  valueRows(i: number): Uint32Array {
    return this.work.valueRows(i);
  }

  /** A spec made whole for this data: the columns its kind needs, the first that fit when not given (`specs::complete`). */
  complete(spec: ChartSpec): ChartSpec {
    return this.work.complete(JSON.stringify(spec)) as ChartSpec;
  }

  /** İstatistik's Grafik for a column: a number's histogram, anything else's bars (`specs::for_column`). */
  forColumn(key: string): ChartSpec {
    return this.work.forColumn(key) as ChartSpec;
  }

  /** An İşlemler table's chart: its tool's suggestion when the table has its columns, else bars (`specs::for_table`). */
  forTable(tool: string): ChartSpec {
    return this.work.forTable(tool) as ChartSpec;
  }

  chart(spec: ChartSpec, rows: Uint32Array): ChartInfo {
    return this.work.chart(JSON.stringify(spec), rows) as ChartInfo;
  }

  /** The drawing's selection by rows: whether the chart's marks changed. */
  select(rows: Uint32Array): boolean {
    return this.work.select(rows);
  }

  /** The elements the selection marks, by their places (0 first). */
  marked(): Uint32Array {
    return this.work.marked();
  }

  /** Where element `e` is pressed on the last layout, in its pixels; none off the chart. */
  elementAt(e: number): { x: number; y: number } | null {
    const p = this.work.elementAt(e);
    return p.length === 2 ? { x: p[0], y: p[1] } : null;
  }

  layOut(frame: ChartFrame): ChartLayout | null {
    return this.work.layOut(JSON.stringify(frame)) as ChartLayout | null;
  }

  points(): Float32Array {
    return this.work.points();
  }

  /** The bulk points' elements, in the points' order. */
  pointElements(): Uint32Array {
    return this.work.pointElements();
  }

  hit(x: number, y: number): number {
    return this.work.hit(x, y);
  }

  tip(element: number): [string, string][] {
    return this.work.tip(element) as [string, string][];
  }

  elementRows(element: number): Uint32Array {
    return this.work.elementRows(element);
  }

  rowsWithin(x0: number, y0: number, x1: number, y1: number): Uint32Array {
    return this.work.rowsWithin(x0, y0, x1, y1);
  }

  /** The chart laid out unmarked in a frame of its own (a saved picture), its points inline. */
  plainLayout(frame: ChartFrame): ChartLayout & { readonly points: readonly number[] } {
    return this.work.layOutPlain(JSON.stringify(frame)) as ChartLayout & { readonly points: readonly number[] };
  }

  svg(frame: ChartFrame, palette: ChartPalette): string {
    return this.work.svg(JSON.stringify(frame), JSON.stringify(palette));
  }

  free(): void {
    this.work.free();
  }
}
