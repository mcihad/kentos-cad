import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/centerline/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import { centerline } from './constructions';

/**
 * Orta hat (docs/adr/0190) through the WASM core, on the independent reference's cases (fixtures/centerline/v1/cases.json,
 * scripts/fixtures/centerline_cases.py): the axis between two sides, edge by edge between matched sides, else sampled.
 * The core runs the same file natively (crates/shared/geometry-core/tests/all/centerline.rs).
 */
const f = JSON.parse(text);

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

describe('Orta hat (fixtures/centerline/v1)', () => {
  it('draws the axis between two sides', () => {
    for (const c of f.cases) {
      const got = centerline(c.a as Entity, c.b as Entity, c.step);
      if (c.expect.problem) {
        expect(got.problem, c.name).toBe(c.expect.problem);
        continue;
      }
      const axis = got.centerline!;
      expect(axis.method, c.name).toBe(c.expect.method);
      expect(axis.pts.length, `${c.name}: vertices`).toBe(c.expect.pts.length);
      axis.pts.forEach((p, i) => {
        near(p.x, c.expect.pts[i][0], 1e-7, `${c.name} x`);
        near(p.y, c.expect.pts[i][1], 1e-7, `${c.name} y`);
      });
      if (c.expect.bulges) {
        expect(axis.bulges?.length, `${c.name}: bulges`).toBe(c.expect.bulges.length);
        axis.bulges!.forEach((b, i) => near(b, c.expect.bulges[i], 1e-12, `${c.name} bulge`));
      } else expect(axis.bulges ?? null, `${c.name}: no bulges`).toBeNull();
    }
  });
});
