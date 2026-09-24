import { describe, expect, it } from 'vitest';
import text from '../../../../../fixtures/geometry/v1/reference.json?raw';
import type { Vec2 } from '../geometry';
import { bulgePathLength, bulgeRingArea } from './bulge';

/**
 * Accuracy against an independent reference (CLAUDE.md §23.4): exact areas
 * from the coordinates' decimal text (Python fractions) and closed formulas
 * with a 60-digit π, made without KentOS code
 * (scripts/fixtures/geometry_reference.py). Agreeing with the Rust port is
 * not enough; both must stay within each case's bound of these values.
 */

type Ring = { pts: [string, string][]; bulges?: string[] };
interface ReferenceCase {
  name: string;
  op: 'polygonArea' | 'polygonPerimeter';
  input: { outer: Ring; holes: Ring[] };
  expected: string;
  bound: string;
}

const file = JSON.parse(text) as { format: string; version: number; cases: ReferenceCase[] };

/** Decimal text read as a file reader would: to the nearest float64. */
const ring = (r: Ring): { pts: Vec2[]; bulges?: number[] } => ({ pts: r.pts.map(([x, y]) => ({ x: Number(x), y: Number(y) })), bulges: r.bulges?.map(Number) });

function compute(c: ReferenceCase): number {
  const outer = ring(c.input.outer);
  const holes = c.input.holes.map(ring);
  if (c.op === 'polygonArea') return Math.abs(bulgeRingArea(outer.pts, outer.bulges)) - holes.reduce((s, h) => s + Math.abs(bulgeRingArea(h.pts, h.bulges)), 0);
  return bulgePathLength(outer.pts, outer.bulges, true) + holes.reduce((s, h) => s + bulgePathLength(h.pts, h.bulges, true), 0);
}

describe('geometry against an independent exact reference', () => {
  it('reads a versioned reference file', () => {
    expect([file.format, file.version]).toEqual(['kentos.geometry-reference', 1]);
  });
  for (const c of file.cases)
    it(`${c.op}: ${c.name}`, () => {
      // The expected value keeps more digits than a float64; the difference is taken in float64
      // only after it is known to be small, so the bound is checked honestly.
      const err = Math.abs(compute(c) - Number(c.expected));
      expect(err).toBeLessThanOrEqual(Number(c.bound));
    });
});
