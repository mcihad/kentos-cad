import { describe, expect, it } from 'vitest';
import { crsPlaneMeasures, type PlaneRing, type System } from '../model/geom/crsTransform';

/**
 * Lengths and areas in the second system's plane (docs/adr/0167 §2) against PROJ (fixtures/geodesy/v1/measure.json,
 * written by scripts/fixtures/crs_measure_cases.py from pyproj and the ADR's rule, not KentOS code), through the WASM
 * core the app calls: lengths within 1e-6 m, areas within 1e-6 m² for every 100 m of their perimeter, none in a
 * geographic system or the Pseudo-Mercator. The core runs the same file natively (crates/shared/geometry-core/tests/all/crs_measure.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  measure: {
    name: string;
    from: System;
    to: System;
    rings: { pts: [number, number][]; bulges?: number[] }[];
    closed: boolean;
    expect: { length: number; area?: number } | null;
    why?: string;
  }[];
}

describe('crsPlaneMeasures', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/measure.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.crs-measure'));

  it('measures paths and areas in the second plane as PROJ takes them', () => {
    const off = file.measure.flatMap((c) => {
      const rings: PlaneRing[] = c.rings.map((r) => ({ pts: r.pts.map(([x, y]) => ({ x, y })), bulges: r.bulges ?? null }));
      const got = crsPlaneMeasures(c.from, c.to, rings, c.closed);
      if (!c.expect) return 'why' in got && got.why === c.why ? [] : [`${c.name}: ${JSON.stringify(got)} ≠ ${c.why}`];
      if ('why' in got) return [`${c.name}: none (${got.why})`];
      const area = c.expect.area ?? 0;
      return Math.abs(got.length - c.expect.length) <= 1e-6 && Math.abs(got.area - area) <= 1e-6 * Math.max(1, c.expect.length / 100)
        ? []
        : [`${c.name}: ${got.length}, ${got.area} ≠ ${c.expect.length}, ${area}`];
    });
    expect(off).toEqual([]);
    expect(file.measure.length).toBeGreaterThanOrEqual(20);
  });
});
