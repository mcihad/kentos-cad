import type { CoordColumn } from '../contracts/generated/CoordColumn';
import type { CoordDelimiter } from '../contracts/generated/CoordDelimiter';
import type { CoordPoint } from '../contracts/generated/CoordPoint';
import type { Entity, PointEntity } from '../model/entities';
import type { LayerStyle } from '../model/layers';

/**
 * Coordinate lists (Netcad NCN, TXT, CSV) as the dialogs offer them: column
 * orders, delimiters and output formats, and the points of a drawing to
 * write. Reading and writing themselves are in Rust (crates/shared/formats).
 * Y is to the right (east, the model's x), X is up (north) (CLAUDE.md §5).
 */

export const COLUMN_LABEL: Record<CoordColumn, string> = {
  name: 'Ad',
  y: 'Y (sağa)',
  x: 'X (yukarı)',
  z: 'Z (kot)',
  code: 'Kod',
  skip: 'Alınmaz',
};

export const COLUMN_ROLES: readonly CoordColumn[] = ['name', 'y', 'x', 'z', 'code', 'skip'];

export const DELIMITER_LABEL: Record<CoordDelimiter, string> = {
  auto: 'Otomatik',
  space: 'Boşluk',
  tab: 'Sekme',
  semicolon: 'Noktalı virgül (;)',
  comma: 'Virgül (,)',
};

/** Column orders survey files use; Netcad's is first. */
export const COORD_ORDERS: readonly { id: string; label: string; columns: readonly CoordColumn[] }[] = [
  { id: 'nyxz', label: 'Ad Y X Z', columns: ['name', 'y', 'x', 'z'] },
  { id: 'nxyz', label: 'Ad X Y Z', columns: ['name', 'x', 'y', 'z'] },
  { id: 'yxz', label: 'Y X Z', columns: ['y', 'x', 'z'] },
  { id: 'xyz', label: 'X Y Z', columns: ['x', 'y', 'z'] },
];

/** An order laid over a file with `count` columns (the others are not read). */
export function orderColumns(order: readonly CoordColumn[], count: number): CoordColumn[] {
  return Array.from({ length: Math.max(count, order.length) }, (_, i) => order[i] ?? 'skip');
}

/** The order `columns` follow, if they are one of the known ones. */
export function orderOf(columns: readonly CoordColumn[]): string | null {
  const used = columns.filter((c) => c !== 'skip');
  return COORD_ORDERS.find((o) => o.columns.length === used.length && o.columns.every((c, i) => columns[i] === c))?.id ?? null;
}

export interface CoordFormat {
  id: 'ncn' | 'txt' | 'csv' | 'csvComma';
  label: string;
  delimiter: CoordDelimiter;
  extension: string;
  header: boolean;
}

/** Output formats of the coordinate list export. */
export const COORD_FORMATS: readonly CoordFormat[] = [
  { id: 'ncn', label: 'NCN (Netcad; boşlukla ayrılmış)', delimiter: 'space', extension: '.ncn', header: false },
  { id: 'txt', label: 'TXT (sekmeyle ayrılmış)', delimiter: 'tab', extension: '.txt', header: false },
  { id: 'csv', label: 'CSV (noktalı virgülle)', delimiter: 'semicolon', extension: '.csv', header: true },
  { id: 'csvComma', label: 'CSV (virgülle)', delimiter: 'comma', extension: '.csv', header: true },
];

/** Point objects as coordinate list rows: the name is the label, else the "Ad" attribute. */
export function coordPoints(entities: Iterable<Entity>): CoordPoint[] {
  const out: CoordPoint[] = [];
  for (const e of entities) {
    if (e.kind !== 'point') continue;
    const p: CoordPoint = { name: pointName(e), p: { x: e.p.x, y: e.p.y } };
    if (e.z !== undefined) p.z = e.z;
    const code = e.attrs.Kod;
    if (code) p.code = code;
    out.push(p);
  }
  return out;
}

export const pointName = (e: PointEntity): string => e.label ?? e.attrs.Ad ?? '';

/** The look of a layer made for imported points: a cross with the name beside it. */
export const POINT_LAYER_STYLE: Partial<LayerStyle> = {
  color: 'ink',
  point: { symbol: 'cross', size: 8 },
  label: { placement: 'beside', size: 10.5, minScale: 0.9 },
};
