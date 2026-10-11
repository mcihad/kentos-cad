import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/stationing/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Route } from '../model/ops/lrs';
import { stationing, stationingMeasure, type Stationing } from './constructions';

/**
 * Km yaz (docs/adr/0189) through the WASM core, on the independent reference's cases (fixtures/stationing/v1/cases.json,
 * scripts/fixtures/stationing_cases.py): a route's stations and the ticks, km texts, cross-sections and points at them;
 * and by a measured route's measures (docs/adr/0224 §6, `measureCases`). The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/stationing.rs).
 */
const f = JSON.parse(text);

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

const point = (got: Vec2, want: number[], what: string) => {
  near(got.x, want[0], 1e-7, `${what} x`);
  near(got.y, want[1], 1e-7, `${what} y`);
};

/** The core's answer for a case against the reference's. */
function compare(name: string, got: { stationing?: Stationing | null; problem?: string | null }, want: any): void {
  if (want.problem) {
    expect(got.problem, name).toBe(want.problem);
    return;
  }
  const s = got.stationing!;
  expect(s.stations.length, `${name}: stations`).toBe(want.stations.length);
  s.stations.forEach((st, i) => {
    const w = want.stations[i];
    expect(st.text, name).toBe(w.text);
    near(st.s, w.s, 1e-7, name);
    point(st.point, w.point, name);
    point(st.tangent, w.tangent, name);
  });
  expect(s.texts.length, `${name}: texts`).toBe(want.texts.length);
  s.texts.forEach((t, i) => {
    const w = want.texts[i];
    point(t.p, w.p, name);
    near(t.rotation, w.rotation, 1e-9, name);
    expect([t.align ?? null, t.text], name).toEqual([w.align, w.text]);
  });
  for (const key of ['ticks', 'sections'] as const) {
    expect(s[key].length, `${name}: ${key}`).toBe(want[key].length);
    s[key].forEach((m, i) => {
      point(m.a, want[key][i].a, name);
      point(m.b, want[key][i].b, name);
      expect(m.km, name).toBe(want[key][i].km);
    });
  }
  expect(s.points.length, `${name}: points`).toBe(want.points.length);
  s.points.forEach((p, i) => {
    point(p.p, want.points[i].p, name);
    expect(p.km, name).toBe(want.points[i].km);
  });
}

describe('Km yaz (fixtures/stationing/v1)', () => {
  it('puts the stations along a route and writes at each', () => {
    for (const c of f.cases) compare(c.name, stationing(c.shape as Entity, c.rules, c.look), c.expect);
  });

  it('puts the stations of a measured route by its measures', () => {
    expect(f.measureCases.length).toBeGreaterThan(0);
    for (const c of f.measureCases) compare(c.name, stationingMeasure(c.route as Route, c.rules, c.look), c.expect);
  });
});
