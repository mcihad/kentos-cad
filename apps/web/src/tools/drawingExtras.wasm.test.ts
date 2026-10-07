import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/drawing-extras/v1/cases.json?raw';
import { chosenTangent, commonTangents, fourthCorner, rangeRings, type Tangent } from '../model/drawingExtras';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';

/**
 * Çizim ekleri (docs/adr/0197) through the WASM core, on the independent reference's cases
 * (fixtures/drawing-extras/v1/cases.json, scripts/fixtures/drawing_extras_cases.py): two circles' or arcs' common
 * tangents and the one two clicks choose, a parallelogram's fourth corner, range rings and their rays. The core runs
 * the same file natively (crates/shared/geometry-core/tests/all/drawing_extras.rs).
 */
const f = JSON.parse(text);
const near = (got: number, want: number, what: string) => expect(Math.abs(got - want) <= 1e-9 * Math.max(1, Math.abs(want)), `${what}: ${got} ≠ ${want}`).toBe(true);
const nearPoint = (got: Vec2, want: Vec2, what: string) => {
  near(got.x, want.x, `${what}.x`);
  near(got.y, want.y, `${what}.y`);
};
const sameTangent = (got: Tangent, want: Tangent, what: string) => {
  expect(got.kind, what).toBe(want.kind);
  nearPoint(got.a, want.a, `${what} a`);
  nearPoint(got.b, want.b, `${what} b`);
};

describe('Çizim ekleri (fixtures/drawing-extras/v1/cases.json)', () => {
  it('gives two circles or arcs the reference’s common tangents, in order', () => {
    for (const c of f.tangents) {
      const got = commonTangents(c.first as Entity, c.second as Entity);
      expect(got.length, c.name).toBe(c.want.length);
      got.forEach((t, i) => sameTangent(t, c.want[i], `${c.name} #${i}`));
    }
  });

  it('chooses the tangent nearest the clicks', () => {
    for (const c of f.choices) {
      const got = chosenTangent(c.first as Entity, c.second as Entity, c.p1, c.p2);
      expect(got === null, c.name).toBe(c.want === null);
      if (got) sameTangent(got, c.want, c.name);
    }
  });

  it('finds the fourth corner and the rings and rays', () => {
    for (const c of f.fourth) nearPoint(fourthCorner(c.a, c.b, c.c), c.want, c.name);
    for (const c of f.rings) {
      const got = rangeRings(c.center, c.spacing, c.count, c.rays)!;
      expect(got.radii.length, c.name).toBe(c.want.radii.length);
      got.radii.forEach((r, i) => near(r, c.want.radii[i], `${c.name} radius ${i}`));
      expect(got.rays.length, c.name).toBe(c.want.rays.length);
      got.rays.forEach((p, i) => nearPoint(p, c.want.rays[i], `${c.name} ray ${i}`));
    }
    expect(rangeRings({ x: 0, y: 0 }, 0, 5, 0)).toBeNull();
    expect(rangeRings({ x: 0, y: 0 }, 10, 101, 0)).toBeNull();
    expect(rangeRings({ x: 0, y: 0 }, 10, 5, 361)).toBeNull();
  });
});
