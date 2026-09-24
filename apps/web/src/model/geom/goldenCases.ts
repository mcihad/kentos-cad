import { pointInPolygon, signedArea, type Vec2 } from '../geometry';
import { bulgeArc, bulgePathLength, bulgeRingArea } from './bulge';

/**
 * The golden geometry cases shared with the Rust core
 * (fixtures/geometry/v1/cases.json, docs/adr/0002-contracts-fixtures.md):
 * how the TypeScript reference computes each case, and how results are
 * compared. Used by golden.test.ts and by the recorder
 * (scripts/fixtures/record-geometry.test.ts).
 */

type Ring = { pts: Vec2[]; bulges?: number[] };
export type Case =
  | { name: string; op: 'bulgeArc'; input: { a: Vec2; b: Vec2; bulge: number }; expected?: unknown }
  | { name: string; op: 'bulgePathLength'; input: { pts: Vec2[]; bulges?: number[]; closed: boolean }; expected?: unknown }
  | { name: string; op: 'bulgeRingArea'; input: Ring; expected?: unknown }
  | { name: string; op: 'signedArea'; input: { pts: Vec2[] }; expected?: unknown }
  | { name: string; op: 'pointInPolygon'; input: { p: Vec2; pts: Vec2[] }; expected?: unknown }
  | { name: string; op: 'polygonArea'; input: { outer: Ring; holes: Ring[] }; expected?: unknown }
  | { name: string; op: 'polygonPerimeter'; input: { outer: Ring; holes: Ring[] }; expected?: unknown };

export interface GoldenFile {
  format: 'kentos.geometry-fixtures';
  version: 1;
  /** |actual − expected| ≤ abs + rel·max(|actual|, |expected|). */
  tolerance: { abs: number; rel: number };
  /** The coordinates' kind of system and unit: the tolerance holds for these (CLAUDE.md §14). */
  crs: { kind: 'projected'; unit: 'metre'; note: string };
  cases: Case[];
}

/** What the TypeScript core computes for a case (the reference). */
export function run(c: Case): unknown {
  switch (c.op) {
    case 'bulgeArc':
      return bulgeArc(c.input.a, c.input.b, c.input.bulge);
    case 'bulgePathLength':
      return bulgePathLength(c.input.pts, c.input.bulges, c.input.closed);
    case 'bulgeRingArea':
      return bulgeRingArea(c.input.pts, c.input.bulges);
    case 'signedArea':
      return signedArea(c.input.pts);
    case 'pointInPolygon':
      return pointInPolygon(c.input.p, c.input.pts);
    case 'polygonArea':
      // entityArea of a polygon: |outer| minus every |hole|.
      return Math.abs(bulgeRingArea(c.input.outer.pts, c.input.outer.bulges)) - c.input.holes.reduce((s, h) => s + Math.abs(bulgeRingArea(h.pts, h.bulges)), 0);
    case 'polygonPerimeter':
      return bulgePathLength(c.input.outer.pts, c.input.outer.bulges, true) + c.input.holes.reduce((s, h) => s + bulgePathLength(h.pts, h.bulges, true), 0);
  }
}

/** Deep comparison: numbers within the tolerance, everything else exactly. */
export function close(actual: unknown, expected: unknown, tol: GoldenFile['tolerance'], path: string): string | null {
  if (typeof expected === 'number' && typeof actual === 'number') {
    const d = Math.abs(actual - expected);
    return d <= tol.abs + tol.rel * Math.max(Math.abs(actual), Math.abs(expected)) ? null : `${path}: ${actual} ≠ ${expected} (fark ${d})`;
  }
  if (expected && typeof expected === 'object' && actual && typeof actual === 'object') {
    for (const k of Object.keys(expected)) {
      const r = close((actual as Record<string, unknown>)[k], (expected as Record<string, unknown>)[k], tol, `${path}.${k}`);
      if (r) return r;
    }
    return null;
  }
  return actual === expected ? null : `${path}: ${JSON.stringify(actual)} ≠ ${JSON.stringify(expected)}`;
}
