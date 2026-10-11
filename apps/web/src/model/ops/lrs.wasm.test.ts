import { describe, expect, it } from 'vitest';
import text from '../../../../../fixtures/lrs/v1/cases.json?raw';
import type { EntityGeometry } from '../entities';
import { lrsCalibrate, lrsCreate, lrsEvents, lrsLocate, lrsOverlay, lrsSummaries, type Route } from './lrs';

/**
 * Doğrusal referanslama (docs/adr/0224) through the WASM core, on the independent reference's cases
 * (fixtures/lrs/v1/cases.json, scripts/fixtures/lrs_cases.py): the core runs the same file natively
 * (crates/shared/geometry-core/tests/all/lrs.rs).
 */
const f = JSON.parse(text);
const TOL = 1e-7;

/** Numbers within the tolerance, everything else equal, all the way down (the expected object's keys). */
function same(got: unknown, want: unknown, what: string): void {
  if (typeof want === 'number') {
    expect(typeof got, what).toBe('number');
    expect(Math.abs((got as number) - want), `${what}: ${String(got)} ≠ ${want}`).toBeLessThanOrEqual(TOL * Math.max(Math.abs(want), 1));
  } else if (Array.isArray(want)) {
    expect(Array.isArray(got), what).toBe(true);
    expect((got as unknown[]).length, what).toBe(want.length);
    want.forEach((w, i) => same((got as unknown[])[i], w, `${what}[${i}]`));
  } else if (want !== null && typeof want === 'object') {
    for (const [k, w] of Object.entries(want)) same((got as Record<string, unknown>)?.[k] ?? null, w, `${what}.${k}`);
  } else expect(got ?? null, what).toBe(want);
}

describe('Doğrusal referanslama (fixtures/lrs/v1)', () => {
  it('finds a point’s place along a route', () => {
    for (const c of f.locate) {
      const [rows] = lrsLocate([c.route as Route], [{ kind: 'point', p: c.p } as EntityGeometry], 1e9, false);
      same(rows[0].m, c.expect.m, c.title);
      same(rows[0].distance, c.expect.distance, c.title);
    }
  });

  it('places a measure and a stretch of measures', () => {
    for (const c of f.pointAt) {
      const [out] = lrsEvents([c.route as Route], [{ route: 0, m: c.m, offset: c.offset }]);
      if (c.expect.fault) same(out.fault, c.expect, c.title);
      else same(out.p, c.expect.p, c.title);
    }
    for (const c of f.stretch) {
      const [out] = lrsEvents([c.route as Route], [{ route: 0, from: c.from, to: c.to, offset: c.offset }]);
      if (c.expect.fault) expect(out.fault?.fault, c.title).toBe(c.expect.fault);
      else same(out.parts, c.expect.parts, c.title);
    }
  });

  it('makes, calibrates and sums routes up', () => {
    for (const c of f.create) same(lrsCreate(c.pieces, c.rules), c.expect, c.title);
    for (const c of f.calibrate) {
      const [out] = lrsCalibrate([c.route as Route], [c.points], c.rules);
      if (c.expect.fault) expect(out.fault?.fault, c.title).toBe(c.expect.fault);
      else same(out.parts, c.expect.parts, c.title);
      same(out.used, c.expect.used, c.title);
    }
    for (const c of f.summary) same(lrsSummaries([c.route as Route])[0], c.expect, c.title);
  });

  it('overlays line events', () => {
    for (const c of f.overlay) same(lrsOverlay(c.a, c.b, c.union), c.expect, c.title);
  });

  it('locates lines and areas along a route', () => {
    for (const c of f.locateLine) {
      const [rows] = lrsLocate([c.route as Route], [{ kind: 'polyline', pts: c.pts } as EntityGeometry], c.radius, true);
      if (c.expect.none) expect(rows.length, c.title).toBe(0);
      else same(rows[0], c.expect, c.title);
    }
    for (const c of f.locateArea) {
      const [outer, ...holes] = c.rings;
      const area = { kind: 'polygon', pts: outer, holes: holes.map((pts: unknown) => ({ pts })) } as EntityGeometry;
      const [rows] = lrsLocate([c.route as Route], [area], 0, true);
      same(
        rows,
        c.expect.map((q: { from: number | null; to: number | null }) => ({ route: 0, from: q.from, to: q.to })),
        c.title,
      );
    }
  });
});
