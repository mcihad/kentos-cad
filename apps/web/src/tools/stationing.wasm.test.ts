import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/stationing/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { stationing } from './constructions';

/**
 * Km yaz (docs/adr/0189) through the WASM core, on the independent reference's cases (fixtures/stationing/v1/cases.json,
 * scripts/fixtures/stationing_cases.py): a route's stations and the ticks, km texts, cross-sections and points at them.
 * The core runs the same file natively (crates/shared/geometry-core/tests/all/stationing.rs).
 */
const f = JSON.parse(text);

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

const point = (got: Vec2, want: number[], what: string) => {
  near(got.x, want[0], 1e-7, `${what} x`);
  near(got.y, want[1], 1e-7, `${what} y`);
};

describe('Km yaz (fixtures/stationing/v1)', () => {
  it('puts the stations along a route and writes at each', () => {
    for (const c of f.cases) {
      const got = stationing(c.shape as Entity, c.rules, c.look);
      if (c.expect.problem) {
        expect(got.problem, c.name).toBe(c.expect.problem);
        continue;
      }
      const s = got.stationing!;
      expect(s.stations.length, `${c.name}: stations`).toBe(c.expect.stations.length);
      s.stations.forEach((st, i) => {
        const w = c.expect.stations[i];
        expect(st.text, c.name).toBe(w.text);
        near(st.s, w.s, 1e-7, c.name);
        point(st.point, w.point, c.name);
        point(st.tangent, w.tangent, c.name);
      });
      s.texts.forEach((t, i) => {
        const w = c.expect.texts[i];
        point(t.p, w.p, c.name);
        near(t.rotation, w.rotation, 1e-9, c.name);
        expect([t.align ?? null, t.text], c.name).toEqual([w.align, w.text]);
      });
      for (const key of ['ticks', 'sections'] as const) {
        expect(s[key].length, `${c.name}: ${key}`).toBe(c.expect[key].length);
        s[key].forEach((m, i) => {
          point(m.a, c.expect[key][i].a, c.name);
          point(m.b, c.expect[key][i].b, c.name);
          expect(m.km, c.name).toBe(c.expect[key][i].km);
        });
      }
      expect(s.points.length, `${c.name}: points`).toBe(c.expect.points.length);
      s.points.forEach((p, i) => {
        point(p.p, c.expect.points[i].p, c.name);
        expect(p.km, c.name).toBe(c.expect.points[i].km);
      });
    }
  });
});
