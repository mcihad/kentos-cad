import { describe, expect, it } from 'vitest';
import layout from '../../../../../fixtures/dimension/v1/layout.json?raw';
import quick from '../../../../../fixtures/dimension/v1/quick.json?raw';
import type { Vec2 } from '../geometry';
import { dimensionLabel, dimensionMeasure, layoutDimension, quickDimensions, type DimensionGeom, type DimensionStyle } from './dimension';

/**
 * The dimensions of docs/adr/0147 through WASM against the shared cases (fixtures/dimension/v1/layout.json, written
 * from the rules alone by scripts/fixtures/dimension_cases.py); natively crates/shared/geometry-core/tests/all/dimensions.rs.
 */

interface Case {
  name: string;
  dimension: DimensionGeom;
  want: { lines: [Vec2, Vec2][]; ext: number[]; textAt: Vec2; rotation: number; value: number; unit: string; prefix: string; handle: Vec2 } | null;
}

const near = (a: number, e: number) => Math.abs(a - e) <= 1e-9 + 1e-15 * Math.abs(e);
const at = (p: Vec2, e: Vec2) => near(p.x, e.x) && near(p.y, e.y);

describe('the new dimensions (fixtures/dimension/v1)', () => {
  it('lays every shared case out as the independent reference does', () => {
    const file = JSON.parse(layout) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.dimension-cases', 1]);
    expect(file.cases).toHaveLength(25);
    for (const c of file.cases) {
      const got = layoutDimension(c.dimension);
      if (!c.want) {
        expect(got, c.name).toBeNull();
        continue;
      }
      expect(got, c.name).not.toBeNull();
      if (!got) continue;
      const w = c.want;
      expect(got.lines.length, c.name).toBe(w.lines.length);
      // Which lines are extension lines (docs/adr/0205 §6).
      expect(got.ext, c.name).toEqual(w.ext);
      expect(
        got.lines.every(([p, q], i) => at(p, w.lines[i][0]) && at(q, w.lines[i][1])),
        c.name,
      ).toBe(true);
      expect(at(got.textAt, w.textAt) && near(got.rotation, w.rotation) && near(got.value, w.value), c.name).toBe(true);
      expect([got.unit, got.prefix], c.name).toEqual([w.unit, w.prefix]);
      expect(at(got.handle, w.handle), c.name).toBe(true);
    }
  });

  it("measures by style as the core's layout does, for every style", () => {
    const geom: Record<DimensionStyle, DimensionGeom> = {
      aligned: { a: { x: 0, y: 0 }, b: { x: 10, y: 0 }, offset: 2, height: 1 },
      linear: { a: { x: 0, y: 0 }, b: { x: 10, y: 3 }, offset: 2, height: 1, style: 'linear', angle: 0 },
      angular: { a: { x: 10, y: 0 }, b: { x: 0, y: 10 }, c: { x: 0, y: 0 }, offset: 5, height: 1, style: 'angular' },
      radius: { a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: 1, height: 1, style: 'radius' },
      diameter: { a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: 1, height: 1, style: 'diameter' },
      ordinate: { a: { x: 0, y: 0 }, b: { x: 10, y: 0 }, offset: 0, height: 1, style: 'ordinate', angle: 90 },
      arcLength: { a: { x: 10, y: 0 }, b: { x: 0, y: 10 }, c: { x: 0, y: 0 }, offset: 2, height: 1, style: 'arcLength' },
      jogged: { a: { x: 0, y: 0 }, b: { x: 300, y: 0 }, c: { x: 280, y: 4 }, offset: 5, height: 1, style: 'jogged' },
      azimuth: { a: { x: 0, y: 0 }, b: { x: 10, y: 10 }, offset: 1, height: 1, style: 'azimuth' },
      slope: { a: { x: 0, y: 0 }, b: { x: 10, y: 0 }, offset: 1, height: 1, style: 'slope', za: 10, zb: 9 },
    };
    for (const [style, d] of Object.entries(geom) as [DimensionStyle, DimensionGeom][]) {
      const l = layoutDimension(d);
      expect(l && { unit: l.unit, prefix: l.prefix }, style).toEqual(dimensionMeasure(d.style, d.angle));
    }
    // An ordinate's Y.
    expect(dimensionMeasure('ordinate', 0)).toEqual({ unit: 'coordinate', prefix: 'Y=' });
    expect(dimensionMeasure('ordinate', undefined)).toEqual({ unit: 'coordinate', prefix: 'Y=' });
  });

  it('writes a coordinate as a length and a slope in percent', () => {
    const fmt = { length: (m: number) => m.toFixed(3), angle: (a: number) => `${((a * 200) / Math.PI).toFixed(4)} g`, percent: (v: number) => v.toFixed(2) };
    expect(dimensionLabel(undefined, { unit: 'coordinate', prefix: 'Y=', value: 452345.12345 }, fmt)).toBe('Y=452345.123');
    expect(dimensionLabel(undefined, { unit: 'percent', prefix: '%', value: 1.25 }, fmt)).toBe('%1.25');
    expect(dimensionLabel(undefined, { unit: 'angle', prefix: 't=', value: Math.PI / 2 }, fmt)).toBe('t=100.0000 g');
    expect(dimensionLabel('Kot farkı', { unit: 'percent', prefix: '%', value: 1.25 }, fmt)).toBe('Kot farkı');
  });
});

describe('Hızlı ölçü (fixtures/dimension/v1/quick.json)', () => {
  interface QuickCase {
    name: string;
    objects: { kind: string }[];
    at: Vec2;
    typed: number | null;
    height: number;
    want: { dimensions: (Pick<DimensionGeom, 'a' | 'b' | 'offset' | 'c'> & { style?: string })[]; skipped: number };
  }

  it("gives every shared case's dimensions as the independent reference does", () => {
    const file = JSON.parse(quick) as { format: string; version: number; cases: QuickCase[] };
    expect([file.format, file.version]).toEqual(['kentos.quick-dimension-cases', 1]);
    expect(file.cases).toHaveLength(31);
    for (const c of file.cases) {
      const got = quickDimensions(c.objects, c.at, c.typed, c.height);
      expect(got.skipped, c.name).toBe(c.want.skipped);
      expect(got.dimensions.length, c.name).toBe(c.want.dimensions.length);
      got.dimensions.forEach((g, i) => {
        const w = c.want.dimensions[i];
        expect(at(g.a, w.a) && at(g.b, w.b) && near(g.offset, w.offset) && g.height === c.height, `${c.name} #${i}`).toBe(true);
        expect([g.style ?? undefined, !!g.c && !!w.c && at(g.c, w.c), !g.c && !w.c], `${c.name} #${i}`).toEqual([w.style, !!w.c, !w.c]);
      });
    }
  });
});
