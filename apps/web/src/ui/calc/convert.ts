import { fixed } from '../../core/displayNumber';
import type { Vec2 } from '../../model/geometry';
import { crsTransformIn, formatDd, formatDms, parseAngle, type DatumChoice, type System, type Unreached } from '../../model/geom/crsTransform';
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
 * length digits, or in the user's notation with fixed digits. The systems are the core's (the registry's or the
 * project's definitions), the project's datum choices taken where they apply (docs/adr/0168 §9 3c).
 */

/**
 * Why a point was not converted: a field unread, a latitude or longitude out of range, a point the target does not
 * reach; a datum of the project's with no way to WGS 84 between them, a grid choice not on this device, a point outside
 * its grid (docs/adr/0168 §2–§4).
 */
export type ConvertError = 'a' | 'b' | 'range' | 'unreachable' | 'noLink' | 'noGrid' | 'outsideGrid';

const BY_REASON: Record<Unreached, ConvertError> = { outside: 'unreachable', noLink: 'noLink', noGrid: 'noGrid', outsideGrid: 'outsideGrid' };

/** How the window reads and writes: the project's axes' names and length digits, the user's notation. */
export interface ConvertFormat {
  readonly east: string;
  readonly north: string;
  readonly decimals: number;
  readonly notation: GeographicNotation;
}

/** A converted point: its values with their names, how sure they are, and whether only the projection changed (one datum). */
export interface Converted {
  readonly point: Vec2;
  readonly values: readonly [readonly [string, string], readonly [string, string]];
  readonly accuracy: string;
  readonly exact: boolean;
}

const geographic = (s: System) => s.kind === 'geographic';

/** The two fields' names in a system: east and north as the project names them, or latitude and longitude. */
export function fieldNames(sys: System, f: ConvertFormat): [string, string] {
  return geographic(sys) ? ['Enlem', 'Boylam'] : [f.east, f.north];
}

/** A typed point in `sys`'s own order (x east or longitude, y north or latitude), or why there is none. */
export function readPoint(sys: System, a: string, b: string): Vec2 | ConvertError {
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
export function writePoint(sys: System, p: Vec2, f: ConvertFormat): Converted['values'] {
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

/** `a` and `b` typed in `from`, taken to `to` with the project's datum choices and written; or why not. */
export function convertPoint(from: System, to: System, choices: readonly DatumChoice[], a: string, b: string, f: ConvertFormat): Converted | ConvertError {
  const p = readPoint(from, a, b);
  if (typeof p === 'string') return p;
  const moved = crsTransformIn(from, to, p, choices);
  if ('error' in moved) return BY_REASON[moved.error];
  return { point: moved.point, values: writePoint(to, moved.point, f), accuracy: accuracyText(moved), exact: !moved.via };
}

/** What went wrong with a point, in a sentence that says how to put it right. */
export function errorText(e: ConvertError, from: System, f: ConvertFormat): string {
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
    case 'noLink':
      return "Datumlardan birinin WGS 84'e dönüşümü yok; değer yazılmadı.";
    case 'noGrid':
      return 'Datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.';
    case 'outsideGrid':
      return 'Nokta datum dönüşümünün ızgarasının dışında; değer yazılmadı.';
  }
}

/** A list's row: its name and the two values typed. */
export interface ConvertRow {
  name?: string;
  a?: string;
  b?: string;
}

/** A list converted: each filled row's result or its problem, numbered as the table numbers rows (1 first). */
export function convertRows(
  from: System,
  to: System,
  choices: readonly DatumChoice[],
  rows: readonly ConvertRow[],
  f: ConvertFormat,
): { name: string; result: Converted | ConvertError; row: number }[] {
  return rows.flatMap((r, i) => {
    const [name, a, b] = [r.name?.trim() ?? '', r.a ?? '', r.b ?? ''];
    if (!name && !a.trim() && !b.trim()) return [];
    return [{ name: name || String(i + 1), result: convertPoint(from, to, choices, a, b, f), row: i + 1 }];
  });
}

/** A CSV cell: quoted when it holds a comma, a quote or a line break. */
const csvCell = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);

/** The converted rows as CSV, a heading line first; rows that were not converted are left out. */
export function csvText(heading: readonly string[], rows: readonly (readonly string[])[]): string {
  return [heading, ...rows].map((r) => r.map(csvCell).join(',')).join('\n') + '\n';
}
