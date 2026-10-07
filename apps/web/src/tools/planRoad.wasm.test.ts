import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/plan-road/v1/cases.json?raw';
import type { Area, Ring } from '../model/geom/overlay';
import { medianRing, roadParts, roundInnerCorners } from '../model/planRoad';

/**
 * Plan yolu çizimi (docs/adr/0198) through the WASM core, on the independent reference's cases
 * (fixtures/plan-road/v1/cases.json, scripts/fixtures/plan_road_cases.py): a road's areas from its axis, the inner
 * corners of an area rounded, two lines closed into a median. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/plan_road.rs). A ring is compared from whichever of its vertices matches the
 * reference's first.
 */
const f = JSON.parse(text);
const close = (a: number, b: number) => Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(b));

function sameRing(got: Ring, want: Ring, what: string): void {
  expect(got.pts.length, what).toBe(want.pts.length);
  const n = got.pts.length;
  const bulge = (r: Ring, i: number) => r.bulges?.[i] ?? 0;
  const from = (k: number) =>
    want.pts.every((w, i) => {
      const g = got.pts[(i + k) % n];
      return close(g.x, w.x) && close(g.y, w.y) && Math.abs(bulge(got, (i + k) % n) - bulge(want, i)) <= 1e-9;
    });
  expect(
    Array.from({ length: n }, (_, k) => k).some(from),
    `${what}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`,
  ).toBe(true);
}

function sameArea(got: Area, want: Area, what: string): void {
  sameRing(got.outer, want.outer, `${what} outer`);
  expect(got.holes.length, `${what} holes`).toBe(want.holes.length);
  got.holes.forEach((h, i) => sameRing(h, want.holes[i], `${what} hole ${i}`));
}

describe('Plan yolu çizimi (fixtures/plan-road/v1/cases.json)', () => {
  it('gives a road the reference’s areas', () => {
    for (const c of f.roads) {
      const got = roadParts(c.axis, c.width, c.kerb, c.median)!;
      sameArea(got.road, c.want.road, `${c.name} road`);
      for (const part of ['carriageway', 'median'] as const) {
        expect(got[part] == null, `${c.name} ${part}`).toBe(c.want[part] === undefined);
        if (c.want[part]) sameArea(got[part]!, c.want[part], `${c.name} ${part}`);
      }
    }
  });

  it('rounds the inner corners as the reference does', () => {
    for (const c of f.corners) {
      const got = roundInnerCorners(c.area, c.radius);
      sameArea(got.area, c.want.area, c.name);
      expect([got.done, got.skipped], c.name).toEqual([c.want.done, c.want.skipped]);
    }
  });

  it('closes two lines into the reference’s median', () => {
    for (const c of f.medians) sameRing(medianRing(c.first, c.second, c.round)!, c.want, c.name);
    expect(roadParts([{ x: 0, y: 0 }, { x: 50, y: 0 }], 10, 5, 0)).toBeNull();
  });
});
