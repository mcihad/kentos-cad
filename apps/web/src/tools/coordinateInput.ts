import type { PolarAngles } from '../app/format';
import type { Vec2 } from '../model/geometry';
import { op } from '../wasm/core';

const NUM = String.raw`[-+]?\d+(?:\.\d+)?`;
const ABS = new RegExp(String.raw`^(${NUM})\s*[,; ]\s*(${NUM})$`);
const REL = new RegExp(String.raw`^@(${NUM})\s*[,; ]\s*(${NUM})$`);
const POLAR = new RegExp(String.raw`^@?(${NUM})\s*<\s*(${NUM})$`);
const DIST = new RegExp(String.raw`^(${NUM})$`);

// The points themselves come from the Rust core (tools/point_input.rs, docs/adr/0008 S5).
const relativePoint = op<(last: Vec2, dx: number, dy: number) => Vec2>('relativePoint');
const polarPointIn = op<(last: Vec2, distance: number, angle: number, fromNorth: boolean, grads: boolean) => Vec2>('polarPointIn');
const towardPoint = op<(last: Vec2, cursor: Vec2, distance: number) => Vec2 | null>('towardPoint');

/**
 * Parses command-line point input (decimal point, "," or ";" as separator):
 *   486512.34,4420118.9   absolute: east, then north (CBS's Y,X, CAD's X,Y)
 *   @12.5,-3              relative to the last point: east, then north
 *   @25<45                distance<angle: a CAD project's angle from east, counter-clockwise; a CBS
 *                         project's semt from north, clockwise; in the project's angle unit
 *   18.4                  distance along the cursor direction
 * `metres` turns a typed length or coordinate into metres: a local
 * project's drawing unit (docs/adr/0165 §2); `angles` is how a polar
 * angle runs and its unit (§4): degrees from east, counter-clockwise, by default.
 */
export function parsePointInput(
  text: string,
  last: Vec2 | null,
  cursor: Vec2 | null,
  along?: (distance: number) => Vec2 | null,
  metres: (typed: number) => number = (v) => v,
  angles: PolarAngles = { fromNorth: false, grads: false },
): Vec2 | null {
  const t = text.trim();
  let m = t.match(REL);
  if (m) return last ? relativePoint(last, metres(+m[1]), metres(+m[2])) : null;
  m = t.match(POLAR);
  if (m) return last ? polarPointIn(last, metres(+m[1]), +m[2], angles.fromNorth, angles.grads) : null;
  m = t.match(ABS);
  if (m) return { x: metres(+m[1]), y: metres(+m[2]) };
  m = t.match(DIST);
  // A bare number follows an active tracking line first, then the cursor direction.
  const tracked = m && along ? along(metres(+m[1])) : null;
  if (tracked) return tracked;
  if (m && last && cursor) return towardPoint(last, cursor, metres(+m[1]));
  return null;
}

export function parseNumber(text: string): number | null {
  const m = text.trim().replace(',', '.').match(DIST);
  return m ? +m[1] : null;
}

/**
 * A typed length (a distance, a radius, a tolerance, an elevation) in the
 * project's unit, in metres, what the geometry keeps: a local project's
 * millimetres become metres (docs/adr/0165 §2). Counts, angles, factors and
 * paper millimetres are read with `parseNumber`.
 */
export function parseLength(format: { toMetres(typed: number): number }, text: string): number | null {
  const n = parseNumber(text);
  return n === null ? null : format.toMetres(n);
}

export const looksLikeCoordinate = (text: string) => /^[@\d.+#-]/.test(text.trim());

/**
 * `#ad`: a point's name typed for its place (docs/adr/0152 §4), the name after the `#` without the spaces round it;
 * null for any other text or an empty name. The application finds the point.
 */
export function pointName(text: string): string | null {
  const t = text.trim();
  if (!t.startsWith('#')) return null;
  return t.slice(1).trim() || null;
}
