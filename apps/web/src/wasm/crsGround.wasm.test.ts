import { describe, expect, it } from 'vitest';
import { crsGroundMeasures, crsLineFactors, crsPointScale, type GroundMeasures, type PlaneRing, type System } from '../model/geom/crsTransform';
import { op } from './core';

/**
 * Plane, ellipsoid and ground (docs/adr/0171) against fixtures/geodesy/v1/ground.json (written by
 * scripts/fixtures/ground_cases.py from PROJ, GeographicLib's C library and mpmath, not KentOS code), through the WASM
 * core the app calls: point and line scales and height factors within 1e-10, lengths within 1e-6 m, areas within
 * 1e-6 m² for every 100 m of their perimeter and four times their `areaNoise`, a measure's scale as its values allow.
 * The core runs the same file natively (crates/shared/geometry-core/tests/all/crs_ground.rs) and freezes its answers
 * in fixtures/geodesy/v1/ground-answers.json: the WASM gives the same bits (GeographicLib through libm, docs/adr/0171).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Point = [number, number];
interface File {
  format: string;
  scales: { name: string; system: System; point: Point; expect: number | null }[];
  lines: { name: string; system: System; a: Point; b: Point; height: number | null; expect: Record<string, number> | null }[];
  measures: {
    name: string;
    system: System;
    rings: { pts: Point[]; bulges?: number[] }[];
    closed: boolean;
    height: number | null;
    expect: Record<string, number> | null;
    areaNoise?: number;
  }[];
}

const xy = ([x, y]: Point) => ({ x, y });

/** `got` against `want` within `tolerance`, both missing alike. */
function off(name: string, got: number | null | undefined, want: number | null | undefined, tolerance: number): string[] {
  if (got == null || want == null) return (got == null) === (want == null) ? [] : [`${name}: ${got} ≠ ${want}`];
  return Math.abs(got - want) <= tolerance ? [] : [`${name}: ${got} ≠ ${want}`];
}

describe('crs::ground', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/ground.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.ground'));

  it('gives the projection scale at a point as PROJ does', () => {
    expect(file.scales.flatMap((c) => off(c.name, crsPointScale(c.system, xy(c.point)), c.expect, 1e-10))).toEqual([]);
    expect(file.scales.length).toBeGreaterThanOrEqual(15);
  });

  it("gives a line's factors: Simpson's scale, the geodesic and Euler's radius", () => {
    const wrong = file.lines.flatMap((c) => {
      const got = crsLineFactors(c.system, xy(c.a), xy(c.b), c.height);
      if (!c.expect) return 'why' in got ? [] : [`${c.name}: ${JSON.stringify(got)}`];
      if ('why' in got) return [`${c.name}: ${got.why}`];
      return [
        ...off(`${c.name}: geodesic`, got.ellipsoidLength, c.expect.ellipsoidLength, 1e-6),
        ...off(`${c.name}: scale`, got.scale, c.expect.scale, 1e-10),
        ...off(`${c.name}: height factor`, got.heightFactor, c.expect.heightFactor, 1e-10),
      ];
    });
    expect(wrong).toEqual([]);
  });

  it('measures paths and areas in the plane, on the ellipsoid and on the ground', () => {
    const wrong = file.measures.flatMap((c) => {
      const rings: PlaneRing[] = c.rings.map((r) => ({ pts: r.pts.map(xy), bulges: r.bulges ?? null }));
      const got: GroundMeasures = crsGroundMeasures(c.system, rings, c.closed, c.height);
      if (!c.expect) return 'why' in got && got.why === 'unreachable' ? [] : [`${c.name}: ${JSON.stringify(got)}`];
      if ('why' in got) return [`${c.name}: ${got.why}`];
      const want = c.expect;
      const perimeter = want.ellipsoidLength;
      const area = 1e-6 * Math.max(1, perimeter / 100) + 4 * (c.areaNoise ?? 0);
      // A path's scale is its lengths' ratio, an area's its areas' ratio's square root: their relative tolerance, or half of it.
      const scale = 1e-10 + (want.ellipsoidArea === undefined ? 1e-6 / Math.max(perimeter, 1e-6) : area / (2 * want.ellipsoidArea));
      return [
        ...off(`${c.name}: planeLength`, got.planeLength, want.planeLength, 1e-6),
        ...off(`${c.name}: planeArea`, got.planeArea, want.planeArea, area),
        ...off(`${c.name}: ellipsoidLength`, got.ellipsoidLength, want.ellipsoidLength, 1e-6),
        ...off(`${c.name}: ellipsoidArea`, got.ellipsoidArea, want.ellipsoidArea, area),
        ...off(`${c.name}: groundLength`, got.groundLength, want.groundLength, 1e-6),
        ...off(`${c.name}: groundArea`, got.groundArea, want.groundArea, area),
        ...off(`${c.name}: scale`, got.scale, want.scale, scale),
        ...off(`${c.name}: heightFactor`, got.heightFactor, want.heightFactor, 1e-10),
      ];
    });
    expect(wrong).toEqual([]);
    expect(file.measures.length).toBeGreaterThanOrEqual(20);
  });

  it("gives the native core's answers bit for bit", () => {
    const frozen = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/ground-answers.json', import.meta.url), 'utf8')) as {
      format: string;
      answers: { op: string; args: unknown[]; answer: unknown }[];
    };
    expect(frozen.format).toBe('kentos.ground-answers');
    const call = op as (name: string) => (...args: unknown[]) => unknown;
    expect(frozen.answers.map((a) => call(a.op)(...a.args))).toEqual(frozen.answers.map((a) => a.answer));
    expect(frozen.answers.length).toBe(file.scales.length + file.lines.length + file.measures.length);
  });
});
