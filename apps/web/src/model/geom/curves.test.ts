import { describe, expect, it } from 'vitest';
import { edgeLabels } from '../ops/edgeLabels';
import { dimensionLabel, dimensionOffsetAt, layoutDimension, linearAngleFor, sectorArms, signedOffset } from './dimension';
import { hatchLines } from './hatch';
import { tangentPoints } from './intersect';
import { catmullRom } from './spline';

const near = (a: { x: number; y: number }, b: { x: number; y: number }, eps = 1e-9) => Math.hypot(a.x - b.x, a.y - b.y) < eps;

describe('catmullRom', () => {
  const pts = [
    { x: 0, y: 0 },
    { x: 10, y: 5 },
    { x: 20, y: 0 },
    { x: 30, y: 8 },
  ];
  it('passes through every fit point', () => {
    const out = catmullRom(pts, false, 8);
    for (const p of pts) expect(out.some((q) => near(p, q))).toBe(true);
    expect(near(out[0], pts[0])).toBe(true);
    expect(near(out.at(-1)!, pts.at(-1)!)).toBe(true);
  });
  it('keeps collinear points on the line', () => {
    const out = catmullRom(
      [
        { x: 0, y: 0 },
        { x: 5, y: 5 },
        { x: 10, y: 10 },
      ],
      false,
      8,
    );
    for (const p of out) expect(Math.abs(p.x - p.y)).toBeLessThan(1e-9);
  });
  it('closes back to the first point', () => {
    const out = catmullRom(pts, true, 8);
    expect(out).toHaveLength(pts.length * 8 + 1);
    expect(near(out.at(-1)!, pts[0])).toBe(true);
  });
  it('tolerates repeated points', () => {
    const out = catmullRom([pts[0], pts[0], pts[1], pts[2]], false, 4);
    expect(out.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y))).toBe(true);
  });
});

describe('hatchLines', () => {
  const square = [
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 10, y: 10 },
    { x: 0, y: 10 },
  ];
  it('fills a square with horizontal lines at the given spacing', () => {
    const { segments } = hatchLines(square, 0, 1);
    expect(segments).toHaveLength(10);
    for (const [a, b] of segments) expect(Math.abs(Math.abs(b.x - a.x) - 10)).toBeLessThan(1e-9);
  });
  it('clips to a concave outline (two pieces across the notch)', () => {
    const u = [
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 10, y: 10 },
      { x: 7, y: 10 },
      { x: 7, y: 3 },
      { x: 3, y: 3 },
      { x: 3, y: 10 },
      { x: 0, y: 10 },
    ];
    const { segments } = hatchLines(u, 0, 1);
    const at5 = segments.filter(([a]) => Math.abs(a.y - 5) < 1e-9);
    expect(at5).toHaveLength(2);
  });
  it('anchors lines to world multiples of the spacing', () => {
    const shifted = square.map((p) => ({ x: p.x + 0.3, y: p.y + 0.3 }));
    const { segments } = hatchLines(shifted, 0, 1);
    for (const [a] of segments) expect(Math.abs(a.y - Math.round(a.y))).toBeLessThan(1e-9);
  });
  it('refuses absurd densities', () => {
    expect(hatchLines(square, 45, 1e-5).capped).toBe(true);
  });
});

describe('dimension', () => {
  it('puts the dimension line at the offset and measures the distance', () => {
    const l = layoutDimension({ a: { x: 0, y: 0 }, b: { x: 10, y: 0 }, offset: 3, height: 1 })!;
    expect(l.value).toBeCloseTo(10);
    expect(l.d1).toEqual({ x: 0, y: 3 });
    expect(l.rotation).toBeCloseTo(0);
    expect(l.textAt.y).toBeGreaterThan(3);
  });
  it('keeps text readable for right-to-left dimensions', () => {
    const l = layoutDimension({ a: { x: 10, y: 0 }, b: { x: 0, y: 0 }, offset: -3, height: 1 })!;
    expect(l.rotation).toBeCloseTo(0);
    // Offset −3 on a right-to-left line is above; text sits above the line too.
    expect(l.d1.y).toBeCloseTo(3);
    expect(l.textAt.y).toBeGreaterThan(3);
  });
  it('linear: horizontal (ΔY) and vertical (ΔX) distances', () => {
    const h = layoutDimension({ a: { x: 0, y: 0 }, b: { x: 10, y: 5 }, offset: -3, height: 1, style: 'linear', angle: 0 })!;
    expect(h.value).toBeCloseTo(10, 12);
    expect(h.d1).toEqual({ x: 0, y: -3 });
    expect(h.d2.x).toBeCloseTo(10, 12);
    expect(h.d2.y).toBeCloseTo(-3, 12);
    const v = layoutDimension({ a: { x: 0, y: 0 }, b: { x: 10, y: 5 }, offset: -12, height: 1, style: 'linear', angle: 90 })!;
    expect(v.value).toBeCloseTo(5, 12);
    expect(v.d1.x).toBeCloseTo(12, 12);
    expect(v.d2.x).toBeCloseTo(12, 12);
    expect(v.d2.y).toBeCloseTo(5, 12);
    // The grip puts the line back through the point it is dragged to.
    expect(dimensionOffsetAt({ a: { x: 0, y: 0 }, b: { x: 10, y: 5 }, offset: 0, height: 1, style: 'linear', angle: 90 }, { x: 12, y: 2 })).toBeCloseTo(-12, 12);
  });
  it('linear direction follows where the line is placed', () => {
    const a = { x: 0, y: 0 };
    const b = { x: 10, y: 5 };
    expect(linearAngleFor(a, b, { x: 5, y: 10 })).toBe(0);
    expect(linearAngleFor(a, b, { x: 15, y: 2 })).toBe(90);
  });
  it('angular: counter-clockwise from arm a to arm b, arc at the offset', () => {
    const l = layoutDimension({ a: { x: 10, y: 0 }, b: { x: 0, y: 10 }, c: { x: 0, y: 0 }, offset: 5, height: 1, style: 'angular' })!;
    expect(l.value).toBeCloseTo(Math.PI / 2, 12);
    expect(l.unit).toBe('angle');
    expect(l.d1.x).toBeCloseTo(5, 12);
    expect(l.d2.y).toBeCloseTo(5, 12);
    expect(l.pick[0].kind).toBe('arc');
    // Reflex angle when the arms are given the other way round.
    expect(layoutDimension({ a: { x: 0, y: 10 }, b: { x: 10, y: 0 }, c: { x: 0, y: 0 }, offset: 5, height: 1, style: 'angular' })!.value).toBeCloseTo(1.5 * Math.PI, 12);
  });
  it('angular between two lines takes the sector around the arc position', () => {
    const c = { x: 0, y: 0 };
    const [s, e] = sectorArms(c, { x: 1, y: 0 }, { x: 0, y: 1 }, { x: -1, y: 1 });
    expect(s).toEqual({ x: 0, y: 1 });
    expect(e.x).toBe(-1);
    expect(sectorArms(c, { x: 1, y: 0 }, { x: 0, y: 1 }, { x: 1, y: 1 })).toEqual([{ x: 1, y: 0 }, { x: 0, y: 1 }]);
  });
  it('radius and diameter with their prefixes', () => {
    const r = layoutDimension({ a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: 0, height: 1, style: 'radius' })!;
    expect(r.value).toBeCloseTo(5, 12);
    expect(dimensionLabel(undefined, r, { length: (m) => m.toFixed(2), angle: String })).toBe('R 5.00');
    const d = layoutDimension({ a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: 2, height: 1, style: 'diameter' })!;
    expect(d.value).toBeCloseTo(10, 12);
    expect(d.d1.x).toBeCloseTo(-3, 12);
    // The leader runs 2 m past the circle.
    expect(Math.hypot(d.d2.x, d.d2.y)).toBeCloseTo(7, 12);
    expect(dimensionLabel('Ø 10', d, { length: String, angle: String })).toBe('Ø 10');
  });
  it('computes a signed offset', () => {
    expect(signedOffset({ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 5, y: 2 })).toBeCloseTo(2);
    expect(signedOffset({ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 5, y: -2 })).toBeCloseTo(-2);
  });
});

describe('tangentPoints', () => {
  it('finds two tangents from an outside point', () => {
    const t = tangentPoints({ x: 10, y: 0 }, { x: 0, y: 0 }, 5);
    expect(t).toHaveLength(2);
    // Tangent point is perpendicular to the radius: (p − t)·(t − c) = 0
    for (const q of t) expect((10 - q.x) * q.x + (0 - q.y) * q.y).toBeCloseTo(0);
  });
  it('has none from inside', () => {
    expect(tangentPoints({ x: 1, y: 0 }, { x: 0, y: 0 }, 5)).toEqual([]);
  });
});

describe('edgeLabels', () => {
  const square = [
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 10, y: 20 },
    { x: 0, y: 20 },
  ];
  it('labels every edge of a ring with its length, outside the ring', () => {
    const labels = edgeLabels(square, true, 1);
    expect(labels.map((l) => l.length)).toEqual([10, 20, 10, 20]);
    expect(labels[0].p.y).toBeLessThan(0); // bottom edge label below
    expect(labels[1].p.x).toBeGreaterThan(10); // right edge label to the right
  });
  it('keeps every label readable', () => {
    for (const l of edgeLabels([...square].reverse(), true, 1)) expect(l.rotation > -90 && l.rotation <= 90).toBe(true);
  });
  it('skips short edges when asked', () => {
    expect(edgeLabels(square, true, 1, 15)).toHaveLength(2);
  });
});
