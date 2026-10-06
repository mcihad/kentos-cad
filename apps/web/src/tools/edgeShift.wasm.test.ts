import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/edge-shift/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { edgeShift, edgeShiftForArea, edgeShiftPick } from './constructions';

/**
 * Paralel kaydır (docs/adr/0191) through the WASM core, on the independent reference's cases
 * (fixtures/edge-shift/v1/cases.json, scripts/fixtures/edge_shift_cases.py): an edge moved parallel, its neighbours
 * following, and the distance for a target area. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/edge_shift.rs).
 */
const f = JSON.parse(text);

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

const ring = (got: readonly Vec2[], want: number[][], what: string) => {
  expect(got.length, `${what}: vertices`).toBe(want.length);
  got.forEach((p, i) => {
    near(p.x, want[i][0], 1e-9, `${what} x`);
    near(p.y, want[i][1], 1e-9, `${what} y`);
  });
};

describe('Paralel kaydır (fixtures/edge-shift/v1)', () => {
  it('moves an edge parallel, its neighbours following', () => {
    for (const c of f.shifts) {
      const got = edgeShift(c.shape as Entity, c.ring, c.edge, c.distance);
      if (c.expect.problem) {
        expect(got.problem, c.name).toBe(c.expect.problem);
        continue;
      }
      const e = got.entity as Entity & { pts: Vec2[]; holes?: { pts: Vec2[] }[]; bulges?: number[] };
      ring(e.pts, c.expect.pts, c.name);
      (c.expect.holes ?? []).forEach((h: number[][], i: number) => ring(e.holes![i].pts, h, c.name));
      expect(e.bulges ?? null, `${c.name}: bulges kept`).toEqual(c.expect.bulges ?? null);
      expect(e.id, `${c.name}: its other fields kept`).toBe(c.shape.id);
      if (c.expect.area !== undefined) near(got.area!, c.expect.area, 1e-9, c.name);
      else expect(got.area ?? null, `${c.name}: a polyline has no area`).toBeNull();
    }
  });

  it('finds the distance for a target area', () => {
    for (const c of f.targets) {
      const got = edgeShiftForArea(c.shape as Entity, c.ring, c.edge, c.target);
      if (c.expect.problem) expect(got.problem, c.name).toBe(c.expect.problem);
      else near(got.distance!, c.expect.distance, 1e-9, c.name);
    }
  });

  it('picks the edge under a click, its normal away from the area', () => {
    const square = { ...f.shifts[0].shape, holes: [{ pts: [{ x: 4, y: 4 }, { x: 6, y: 4 }, { x: 6, y: 6 }, { x: 4, y: 6 }] }] } as Entity;
    const east = edgeShiftPick(square, { x: 10.2, y: 3 }).picked!;
    expect([east.ring, east.edge, east.normal.x]).toEqual([0, 1, 1]);
    const hole = edgeShiftPick(square, { x: 5, y: 6.1 }).picked!;
    expect([hole.ring, hole.edge, hole.normal.y]).toEqual([1, 2, -1]);
  });
});
