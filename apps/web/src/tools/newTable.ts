import type { AppContext } from '../app/context';
import type { TextFace } from '../model/annotationStyles';
import type { Entity, TableEntity, TableGrid, TableSource } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { tableSchedule, tableSizes, type ListedObject, type ScheduleKind, type TableCells, type TableGeometry, type TableUnits } from '../model/ops/table';
import { geometryOf } from '../product/entitiesEdit';
import { elevatedPaths } from '../product/elevation';

/**
 * A new table from a drawing's objects (docs/adr/0184 §3): the schedules' objects as the core reads them, their
 * schedule in the project's units, a new table's sizes. Tablo ekle (app/tables.ts, ui/table/) and Köşelere koordinat
 * yaz's Çizelge (tools/coordinateLabelTool.ts, docs/adr/0185 §1) make their tables with it; the desktop's is
 * `kentos_interaction::table`.
 */

/** A blank table's columns, text heights wide (Boş tablo: room to type in). */
export const BLANK_COLUMN = 8;

/** The project's settings a schedule writes its numbers by. */
export function tableUnits(ctx: AppContext): TableUnits {
  const s = ctx.doc.settings;
  return { axes: ctx.format.axes, unit: ctx.format.unit, areaUnit: s.areaUnit.value, lengthDecimals: s.lengthDecimals.value, areaDecimals: s.areaDecimals.value };
}

/** An object as a schedule reads it: its shape, paths with their elevations, label and attributes. */
export const listedOf = (e: Entity): ListedObject => ({ shape: geometryOf(e as never), paths: elevatedPaths(e), label: e.label ?? null, attrs: { ...e.attrs } });

/** The objects with these ids, in the drawing's order. */
export function inOrder(ctx: AppContext, ids: Iterable<number>): Entity[] {
  const wanted = new Set(ids);
  return [...ctx.doc.all()].filter((e) => wanted.has(e.id));
}

/** The schedule of `kind` the objects write, in the order given. */
export const scheduleOf = (ctx: AppContext, kind: ScheduleKind, objects: readonly Entity[]): TableCells => tableSchedule(kind, objects.map(listedOf), tableUnits(ctx));

/** The source a schedule keeps: the objects' persistent ids, in order. */
export const sourceOf = (kind: ScheduleKind, objects: readonly Entity[]): TableSource => ({ kind, objects: objects.flatMap((e) => e.uid ?? []) });

/** What a new table looks like besides its cells: its heading row, text height, face, lines and frame width. */
export interface TableLook {
  readonly header: boolean;
  readonly height: number;
  readonly face: TextFace;
  readonly grid?: TableGrid;
  readonly frame?: number;
}

/** A table geometry as the core reads it. */
const shapeOf = (t: Omit<TableEntity, 'id' | 'uid' | 'layerId' | 'attrs'>): TableGeometry => t as unknown as TableGeometry;

/**
 * A new table of `cells` with its top left corner at `p`, as `look` says, from `source`: every row and column as the
 * core's sizes fit its words in the project's typeface; a blank table's columns `BLANK_COLUMN` heights wide.
 */
export function newTable(ctx: AppContext, cells: TableCells, look: TableLook, p: Vec2, source?: TableSource): Omit<TableEntity, 'id' | 'uid' | 'layerId' | 'attrs'> {
  const n = cells.cells.length;
  const m = cells.cells[0]?.length ?? 0;
  const h = look.height;
  const table: Omit<TableEntity, 'id' | 'uid' | 'layerId' | 'attrs'> = {
    kind: 'table',
    p: { x: p.x, y: p.y },
    rotation: 0,
    height: h,
    rows: new Array<number>(n).fill(h),
    columns: new Array<number>(m).fill(h),
    cells: cells.cells.map((r) => [...r]),
    ...(cells.aligns.some((a) => a !== 'left') && { aligns: cells.aligns as TableEntity['aligns'] }),
    ...(look.header && { header: true }),
    ...(look.grid && { grid: look.grid }),
    ...(look.frame !== undefined && { frame: look.frame }),
    ...look.face,
    ...(source && { source }),
  };
  const sizes = tableSizes(shapeOf(table), ctx.doc.settings.drawingFont.value);
  if (sizes) {
    table.rows = sizes.rows;
    table.columns = sizes.columns;
  }
  if (cells.cells.every((r) => r.every((w) => !w))) table.columns = new Array<number>(m).fill(BLANK_COLUMN * h);
  return table;
}
