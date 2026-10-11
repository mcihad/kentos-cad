import type { Entity, NewEntity } from '../../../model/entities';
import type { Elevated, RowResult, Zs } from '../../../model/ops/convert';
import { assignElevations, elevatedPaths, hasElevation } from '../../../product/elevation';
import type { FileValue } from '../../types';
import { outputStyle } from '../geometry/shared';
import { missing, sourceTable } from '../joinSource';

/**
 * What the Dönüştürme tools share (docs/adr/0218; the desktop's `builtin/convert/mod.rs`): their output layer, an
 * object's elevations as the core takes them, a result with its elevations as a new object, and the table tools'
 * source (a file's rows or a layer's objects' attributes), its columns and the report of the rows that gave nothing.
 */

/** The output layer: a new one of this name and colour by default (the geometry tools' look). */
export const outputLayer = (name: string, color: string) =>
  ({ name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: name }, newLayerStyle: outputStyle(color), description: 'Bu adda katman yoksa oluşturulur.' }) as const;

/** The scopes the tools offer, a layer first. */
export const CONVERT_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** An object's elevations path by path (`elevatedPaths`' order), null when it has none. */
export function zsOf(e: Entity): Zs | null {
  const paths = elevatedPaths(e);
  return hasElevation(paths) ? paths.map((p) => p.zs) : null;
}

/** A result as a new object on the output layer, with its attributes and elevations. */
export function placed(r: Elevated, layerId: string, attrs: Record<string, string>): NewEntity {
  const e = { ...r.shape, layerId, attrs } as NewEntity;
  if (r.zs) assignElevations(e, r.zs);
  return e;
}

// ── Tablolar ────────────────────────────────────────────────────────────

/** Where a table tool's rows come from. */
export const sourceParams = (what: string) =>
  [
    {
      name: 'sourceKind',
      label: 'Kaynak',
      type: 'enum',
      options: [
        { value: 'file', label: 'Dosya', hint: 'CSV, TXT ya da Excel dosyası' },
        { value: 'layer', label: 'Katman', hint: 'Nesnelerin öznitelikleri satırdır' },
      ],
      default: 'file',
    },
    {
      name: 'file',
      label: 'Dosya',
      type: 'file',
      accept: ['.csv', '.txt', '.xlsx'],
      visibleWhen: (v: { sourceKind?: unknown }) => v.sourceKind !== 'layer',
      description: `İlk satırı sütun adları olan CSV, TXT ya da Excel (.xlsx) dosyası; her satır bir ${what}.`,
    },
    {
      name: 'source',
      label: 'Kaynak nesneler',
      type: 'features',
      scopes: ['layer', 'all', 'visible', 'selection'],
      visibleWhen: (v: { sourceKind?: unknown }) => v.sourceKind === 'layer',
      description: `Öznitelikleri satır olan nesneler; her nesne bir ${what}.`,
    },
  ] as const;

/** A column parameter. */
export const column = (name: string, label: string, description: string, optional = false) =>
  ({ name, label, type: 'field', of: ['source', 'file'], ...(optional && { optional: true }), description }) as const;

/** The project's angle unit as the default of an angle's unit. */
export const unitParam = {
  name: 'unit',
  label: 'Açı birimi',
  type: 'enum',
  options: [
    { value: 'grad', label: 'Grad', hint: 'Tam dönüş 400' },
    { value: 'deg', label: 'Derece', hint: 'Tam dönüş 360' },
  ],
  default: (c: { angleUnit: string }) => (c.angleUnit === 'deg' ? 'deg' : 'grad'),
} as const;

/** A table tool's rows: the columns' cells row by row, each row's label in the report and its attributes. */
export interface Rows {
  readonly cells: string[][];
  readonly labels: string[];
  readonly attrs: Record<string, string>[];
}

/**
 * The rows of a file (its first line the columns' names; a row's label its line number, the names' line the first) or
 * of objects (their attributes; a row's label the object's number); a column the source does not have is refused.
 */
export function readRows(objects: readonly Entity[] | null, file: FileValue | null, columns: readonly string[]): Rows | { refused: string } {
  const { header, rows } = sourceTable(objects, file);
  const at: number[] = [];
  for (const c of columns) {
    const k = c ? header.indexOf(c) : -1;
    if (c && k < 0) return { refused: missing(c, header) };
    at.push(k);
  }
  const cells = rows.map((r) => at.map((k) => (k < 0 ? '' : (r[k] ?? ''))));
  const labels = objects ? objects.map((e) => `#${e.id}`) : rows.map((_, i) => String(i + 2));
  const attrs = rows.map((r) => {
    const a: Record<string, string> = {};
    header.forEach((h, k) => {
      const cell = r[k] ?? '';
      if (cell.trim()) a[h] = cell;
    });
    return a;
  });
  return { cells, labels, attrs };
}

/** Why a row gave nothing, in the report (§5): an empty cell, one that is no number, one not more than zero, ends in one place. */
export function reason(r: Exclude<RowResult, { shape: unknown }>, names: readonly string[], cells: readonly string[]): string {
  if (r.fault === 'same') return 'Başlangıç ve bitiş aynı yerde';
  const k = r.column ?? 0;
  const name = names[k];
  if (r.fault === 'empty') return `“${name}” boş`;
  if (r.fault === 'number') return `“${name}” sayı değil: ${(cells[k] ?? '').trim()}`;
  return `“${name}” sıfırdan büyük değil`;
}

/** The objects of the rows that gave one, the report of those that did not, the summary. */
export function fromRows(rows: Rows, results: readonly RowResult[], names: readonly string[], layerId: string, what: string) {
  const add: NewEntity[] = [];
  const report: string[][] = [];
  results.forEach((r, i) => {
    if ('shape' in r) add.push({ ...r.shape, layerId, attrs: rows.attrs[i] } as NewEntity);
    else report.push([rows.labels[i], reason(r, names, rows.cells[i])]);
  });
  return {
    add,
    report,
    summary: `${rows.cells.length} satırdan ${add.length} ${what} yazıldı.`,
  };
}
