import type { CrsDef } from '../../geo/crs';
import { fixed } from '../../core/displayNumber';
import type { Vec2 } from '../../model/geometry';
import { crsTransform, formatDd, formatDms, parseAngle, systemOf } from '../../model/geom/crsTransform';
import { accuracyText, type GeographicNotation } from '../../model/secondCrs';
import { readNumber } from './read';

/**
 * Koordinat dönüştür's readings and writings (docs/adr/0167 §4), apart from its window (ConvertDialog.ts) so the shared
 * cases (fixtures/crs/v1/convert.json, scripts/fixtures/crs_convert_cases.py: PROJ) check them; the desktop's are
 * `apps/desktop/src/calc/convert.rs`.
 *
 * A projected system's point is two numbers, east first, named as the project's type names its axes, read as the Hesap
 * windows read numbers; a geographic one is the latitude, then the longitude, in decimal degrees, degrees and minutes,
 * or degrees minutes seconds (spaces or ° ′ ″ ' "; a closing hemisphere letter). Values are written with the project's
 * length digits, or in the user's notation with fixed digits.
 */

/** Why a point was not converted: a field unread, a latitude or longitude out of range, a point the target does not reach. */
export type ConvertError = 'a' | 'b' | 'range' | 'unreachable';

/** How the window reads and writes: the project's axes' names and length digits, the user's notation. */
export interface ConvertFormat {
  readonly east: string;
  readonly north: string;
  readonly decimals: number;
  readonly notation: GeographicNotation;
}

/** A converted point: its values with their names, how sure they are. */
export interface Converted {
  readonly point: Vec2;
  readonly values: readonly [readonly [string, string], readonly [string, string]];
  readonly accuracy: string;
}

const geographic = (c: CrsDef) => c.kind === 'geographic';

/** The two fields' names in a system: east and north as the project names them, or latitude and longitude. */
export function fieldNames(sys: CrsDef, f: ConvertFormat): [string, string] {
  return geographic(sys) ? ['Enlem', 'Boylam'] : [f.east, f.north];
}

/** A typed point in `sys`'s own order (x east or longitude, y north or latitude), or why there is none. */
export function readPoint(sys: CrsDef, a: string, b: string): Vec2 | ConvertError {
  if (geographic(sys)) {
    const lat = parseAngle(a);
    if (lat === null) return 'a';
    const lon = parseAngle(b);
    if (lon === null) return 'b';
    return Math.abs(lat) > 90 || Math.abs(lon) > 180 ? 'range' : { x: lon, y: lat };
  }
  const east = readNumber(a);
  if (east === null || !Number.isFinite(east)) return 'a';
  const north = readNumber(b);
  if (north === null || !Number.isFinite(north)) return 'b';
  return { x: east, y: north };
}

/** A point of `sys` (its own order) as the window writes it. */
export function writePoint(sys: CrsDef, p: Vec2, f: ConvertFormat): Converted['values'] {
  if (geographic(sys)) {
    const write = (deg: number, latitude: boolean) => (f.notation === 'dd' ? formatDd(deg, latitude, 7) : formatDms(deg, latitude, 4));
    return [
      ['Enlem', write(p.y, true)],
      ['Boylam', write(p.x, false)],
    ];
  }
  return [
    [f.east, fixed(p.x, f.decimals)],
    [f.north, fixed(p.y, f.decimals)],
  ];
}

/** `a` and `b` typed in `from`, taken to `to` and written; or why not. */
export function convertPoint(from: CrsDef, to: CrsDef, a: string, b: string, f: ConvertFormat): Converted | ConvertError {
  const p = readPoint(from, a, b);
  if (typeof p === 'string') return p;
  const [s, t] = [systemOf(from), systemOf(to)];
  const moved = s && t ? crsTransform(s, t, p) : null;
  if (!moved) return 'unreachable';
  return { point: moved.point, values: writePoint(to, moved.point, f), accuracy: accuracyText(moved, from.datum === 'ED50' || to.datum === 'ED50') };
}

/** What went wrong with a point, in a sentence that says how to put it right. */
export function errorText(e: ConvertError, from: CrsDef, f: ConvertFormat): string {
  const [a, b] = fieldNames(from, f);
  const how = geographic(from) ? '40 45 12.3456, 40°45′12.3456″K ya da 40.7534293 biçiminde yazın' : 'bir sayı yazın, ör. 414120.512';
  switch (e) {
    case 'a':
      return `${a} okunamadı: ${how}.`;
    case 'b':
      return `${b} okunamadı: ${how}.`;
    case 'range':
      return 'Enlem −90° ile 90°, boylam −180° ile 180° arasında olmalı.';
    case 'unreachable':
      return 'Nokta hedef sistemin ulaştığı yerin dışında; değer yazılmadı.';
  }
}

/** A list's row: its name and the two values typed. */
export interface ConvertRow {
  name?: string;
  a?: string;
  b?: string;
}

/** A list converted: each filled row's result or its problem, numbered as the table numbers rows (1 first). */
export function convertRows(from: CrsDef, to: CrsDef, rows: readonly ConvertRow[], f: ConvertFormat): { name: string; result: Converted | ConvertError; row: number }[] {
  return rows.flatMap((r, i) => {
    const [name, a, b] = [r.name?.trim() ?? '', r.a ?? '', r.b ?? ''];
    if (!name && !a.trim() && !b.trim()) return [];
    return [{ name: name || String(i + 1), result: convertPoint(from, to, a, b, f), row: i + 1 }];
  });
}

/** A CSV cell: quoted when it holds a comma, a quote or a line break. */
const csvCell = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);

/** The converted rows as CSV, a heading line first; rows that were not converted are left out. */
export function csvText(heading: readonly string[], rows: readonly (readonly string[])[]): string {
  return [heading, ...rows].map((r) => r.map(csvCell).join(',')).join('\n') + '\n';
}
