import type { Vec2 } from '../model/geometry';
import { op } from '../wasm/core';

const NUM = String.raw`[-+]?\d+(?:\.\d+)?`;
const ABS = new RegExp(String.raw`^(${NUM})\s*[,; ]\s*(${NUM})$`);
const REL = new RegExp(String.raw`^@(${NUM})\s*[,; ]\s*(${NUM})$`);
const POLAR = new RegExp(String.raw`^@?(${NUM})\s*<\s*(${NUM})$`);
const DIST = new RegExp(String.raw`^(${NUM})$`);

// The points themselves come from the Rust core (tools/point_input.rs, docs/adr/0008 S5).
const relativePoint = op<(last: Vec2, dx: number, dy: number) => Vec2>('relativePoint');
const polarOffset = op<(last: Vec2, distance: number, angle: number) => Vec2>('polarOffset');
const towardPoint = op<(last: Vec2, cursor: Vec2, distance: number) => Vec2 | null>('towardPoint');

/**
 * Parses command-line point input (decimal point, "," or ";" as separator):
 *   486512.34,4420118.9   absolute Y,X
 *   @12.5,-3              relative to the last point
 *   @25<45                distance<angle (degrees, CCW from east)
 *   18.4                  distance along the cursor direction
 */
export function parsePointInput(text: string, last: Vec2 | null, cursor: Vec2 | null, along?: (distance: number) => Vec2 | null): Vec2 | null {
  const t = text.trim();
  let m = t.match(REL);
  if (m) return last ? relativePoint(last, +m[1], +m[2]) : null;
  m = t.match(POLAR);
  if (m) return last ? polarOffset(last, +m[1], +m[2]) : null;
  m = t.match(ABS);
  if (m) return { x: +m[1], y: +m[2] };
  m = t.match(DIST);
  // A bare number follows an active tracking line first, then the cursor direction.
  const tracked = m && along ? along(+m[1]) : null;
  if (tracked) return tracked;
  if (m && last && cursor) return towardPoint(last, cursor, +m[1]);
  return null;
}

export function parseNumber(text: string): number | null {
  const m = text.trim().replace(',', '.').match(DIST);
  return m ? +m[1] : null;
}

export const looksLikeCoordinate = (text: string) => /^[@\d.+-]/.test(text.trim());
