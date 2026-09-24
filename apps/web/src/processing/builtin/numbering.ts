import type { Vec2 } from '../../model/geometry';
import { op } from '../../wasm/core';
import type { CoreCorner, CornerWalk } from '../geometry';

/**
 * Numbering corner points: the number format and the names here; the order
 * a ring is walked in, one number per location across many shapes
 * (neighbouring parcels share their common corners) and the outward
 * direction at a corner come from the geometry core
 * (crates/shared/geometry-core/src/processing/numbering.rs, docs/adr/0008 S4).
 */

export interface NumberFormat {
  /** Text before the number ("P"). */
  prefix: string;
  /** Total length including the prefix ("P00001" → 6). */
  length: number;
  /** Fills the gap between prefix and digits; empty means no padding. */
  pad: string;
}

/** "P" + 17 at length 6 with "0" → "P00017". A number too long for the width is written in full. */
export function formatNumber(n: number, f: NumberFormat): string {
  const digits = String(n);
  const width = Math.max(0, f.length - f.prefix.length);
  return f.prefix + (f.pad ? digits.padStart(width, f.pad[0]) : digits);
}

/** The number in a name written in this format, or null ("P00017" → 17). */
export function parseNumber(name: string, f: NumberFormat): number | null {
  if (!name.startsWith(f.prefix)) return null;
  const rest = name.slice(f.prefix.length);
  const digits = f.pad && f.pad !== '0' ? rest.replace(new RegExp(`^[${f.pad.replace(/[\\\]^-]/g, '\\$&')}]+`), '') : rest;
  return /^\d+$/.test(digits) ? parseInt(digits, 10) : null;
}

export type StartCorner = CornerWalk['start'];
export type Direction = CornerWalk['dir'];

const coreRingOrder = op<(pts: readonly Vec2[], closed: boolean, dir: Direction, start: StartCorner, point: Vec2 | null) => number[]>('ringOrder');

/**
 * Indices of a ring in walking order: turned to the requested direction
 * and rotated to start at the chosen corner. An open path is walked from
 * whichever end is the better start (direction does not apply).
 */
export function ringOrder(pts: readonly Vec2[], closed: boolean, dir: Direction, start: StartCorner, point: Vec2 | null = null): number[] {
  return coreRingOrder(pts, closed, dir, start, point);
}

export interface NumberingInput {
  /** Closed rings (outer first, then holes) or one open path. */
  rings: { pts: readonly Vec2[]; closed: boolean }[];
}

export interface NumberingOptions {
  dir: Direction;
  start: StartCorner;
  point: Vec2 | null;
  format: NumberFormat;
  first: number;
  step: number;
  /** Corners closer than this (m) are one point with one number. */
  tolerance: number;
  /** Merge corners shared by several shapes (and with existing points). */
  shared: boolean;
  /** Points already numbered on the target layer: their names are kept and the counter continues after them. */
  existing: readonly { p: Vec2; name: string }[];
}

export interface NumberedCorner {
  p: Vec2;
  name: string;
  /** Unit vector pointing away from the shape at this corner (for text placement). */
  out: Vec2;
  /** False when the corner reused an existing point or an earlier shape's number. */
  created: boolean;
}

/** Points already numbered: a point without a name is not one, and a corner there gets a number of its own. */
export const numberedPoints = <T extends { name: string }>(existing: readonly T[]): T[] => existing.filter((e) => e.name);

/**
 * Names the core's corners (`RunGeometry.numberCorners`, in numbering
 * order): the counter starts at `first`, or after the highest number
 * written in this format among `named` (the points given to the core), and
 * a new number is made where it first appears.
 */
export function nameCorners(corners: readonly CoreCorner[], named: readonly { name: string }[], o: Pick<NumberingOptions, 'format' | 'first' | 'step'>): NumberedCorner[] {
  let next = o.first;
  for (const e of named) {
    const n = parseNumber(e.name, o.format);
    if (n !== null && n + o.step > next) next = n + o.step;
  }
  const made: string[] = [];
  return corners.map((c) => {
    if (c.ref < 0) return { p: c.p, name: named[-1 - c.ref].name, out: c.out, created: false };
    if (c.ref < made.length) return { p: c.p, name: made[c.ref], out: c.out, created: false };
    const name = formatNumber(next, o.format);
    next += o.step;
    made.push(name);
    return { p: c.p, name, out: c.out, created: true };
  });
}

const coreNumberCorners = op<(inputs: readonly NumberingInput[], walk: CornerWalk, existing: readonly Vec2[]) => CoreCorner[]>('numberCorners');

/**
 * Numbers the corners of every shape. Shapes are taken in order of their
 * start corner (north-west first for the compass starts, nearest first
 * for a point, as given for "first"); each ring is walked from its start
 * in the chosen direction, outer ring before holes.
 */
export function numberCorners(inputs: readonly NumberingInput[], o: NumberingOptions): NumberedCorner[] {
  const named = numberedPoints(o.existing);
  const walk: CornerWalk = { dir: o.dir, start: o.start, point: o.point, tolerance: o.tolerance, shared: o.shared };
  return nameCorners(coreNumberCorners(inputs, walk, o.shared ? named.map((e) => e.p) : []), named, o);
}
