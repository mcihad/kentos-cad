import { describe, expect, it } from 'vitest';
import { apply, compose, isReflection, lengthScale, mirror, rotation, scaling, translation } from './affine';
import { arcThrough, circleThrough, onArc, sweep, TAU } from './arc';
import { circleCircle, closestOnEdge, intersectEdges, lineLine, rayEdge, segSeg } from './intersect';
import { offsetPath, sideOf } from './offset';
import { signedArea } from '../geometry';

const close = (a: { x: number; y: number }, b: { x: number; y: number }, eps = 1e-9) => {
  expect(a.x).toBeCloseTo(b.x, 9);
  expect(a.y).toBeCloseTo(b.y, 9);
  void eps;
};

describe('affine', () => {
  it('rotates 90° around a point', () => {
    close(apply(rotation(Math.PI / 2, { x: 1, y: 1 }), { x: 2, y: 1 }), { x: 1, y: 2 });
  });
  it('mirrors across a vertical line', () => {
    const m = mirror({ x: 5, y: 0 }, { x: 5, y: 10 });
    close(apply(m, { x: 7, y: 3 }), { x: 3, y: 3 });
    expect(isReflection(m)).toBe(true);
  });
  it('mirrors across a diagonal', () => {
    close(apply(mirror({ x: 0, y: 0 }, { x: 1, y: 1 }), { x: 3, y: 1 }), { x: 1, y: 3 });
  });
  it('composes in order (first m1 then m2)', () => {
    const m = compose(translation(10, 0), scaling(2));
    close(apply(m, { x: 1, y: 1 }), { x: 12, y: 2 });
    expect(lengthScale(m)).toBeCloseTo(2);
  });
  it('keeps large TM coordinates exact enough (float64)', () => {
    const o = { x: 486_780, y: 4_420_080 };
    const p = apply(rotation(Math.PI, o), { x: o.x + 12.345, y: o.y });
    expect(p.x).toBeCloseTo(o.x - 12.345, 6);
  });
});

describe('arc', () => {
  it('finds the circle through three points', () => {
    const c = circleThrough({ x: 1, y: 0 }, { x: 0, y: 1 }, { x: -1, y: 0 })!;
    close(c.c, { x: 0, y: 0 });
    expect(c.r).toBeCloseTo(1);
  });
  it('returns null for collinear points', () => {
    expect(circleThrough({ x: 0, y: 0 }, { x: 1, y: 1 }, { x: 2, y: 2 })).toBeNull();
  });
  it('stores arcs counter-clockwise regardless of pick order', () => {
    const ccw = arcThrough({ x: 1, y: 0 }, { x: 0, y: 1 }, { x: -1, y: 0 })!;
    const cw = arcThrough({ x: -1, y: 0 }, { x: 0, y: 1 }, { x: 1, y: 0 })!;
    expect(ccw.a0).toBeCloseTo(0);
    expect(ccw.a1).toBeCloseTo(Math.PI);
    expect(cw.a0).toBeCloseTo(0);
    expect(cw.a1).toBeCloseTo(Math.PI);
    expect(sweep(ccw.a0, ccw.a1)).toBeCloseTo(Math.PI);
  });
  it('tests angle membership across 0', () => {
    expect(onArc(0.1, TAU - 0.2, 0.5)).toBe(true);
    expect(onArc(1, TAU - 0.2, 0.5)).toBe(false);
  });
});

describe('intersect', () => {
  it('crosses two segments', () => {
    const h = segSeg({ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 0, y: 10 }, { x: 10, y: 0 })!;
    close(h.p, { x: 5, y: 5 });
    expect(h.t).toBeCloseTo(0.5);
  });
  it('misses segments that only cross as lines', () => {
    expect(segSeg({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 5, y: -1 }, { x: 5, y: 1 })).toBeNull();
    expect(lineLine({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 5, y: -1 }, { x: 5, y: 1 })!.t).toBeCloseTo(5);
  });
  it('treats parallel lines as no hit', () => {
    expect(lineLine({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: 1 }, { x: 1, y: 1 })).toBeNull();
  });
  it('intersects a segment with an arc only on the arc', () => {
    const upper = { kind: 'arc' as const, c: { x: 0, y: 0 }, r: 5, a0: 0, sweep: Math.PI };
    const hits = intersectEdges({ kind: 'seg', a: { x: -10, y: 3 }, b: { x: 10, y: 3 } }, upper);
    expect(hits).toHaveLength(2);
    const below = intersectEdges({ kind: 'seg', a: { x: -10, y: -3 }, b: { x: 10, y: -3 } }, upper);
    expect(below).toHaveLength(0);
  });
  it('intersects two circles', () => {
    const pts = circleCircle({ x: 0, y: 0 }, 5, { x: 8, y: 0 }, 5);
    expect(pts).toHaveLength(2);
    expect(pts[0].x).toBeCloseTo(4);
  });
  it('shoots rays at edges', () => {
    expect(rayEdge({ x: 0, y: 0 }, { x: 1, y: 0 }, { kind: 'seg', a: { x: 7, y: -1 }, b: { x: 7, y: 1 } })).toEqual([7]);
    expect(rayEdge({ x: 0, y: 0 }, { x: -1, y: 0 }, { kind: 'seg', a: { x: 7, y: -1 }, b: { x: 7, y: 1 } })).toEqual([]);
  });
  it('finds the closest point on an arc or its end', () => {
    const e = { kind: 'arc' as const, c: { x: 0, y: 0 }, r: 5, a0: 0, sweep: Math.PI / 2 };
    close(closestOnEdge(e, { x: 10, y: 10 }).p, { x: 5 * Math.SQRT1_2, y: 5 * Math.SQRT1_2 });
    close(closestOnEdge(e, { x: -3, y: -4 }).p, { x: 5, y: 0 });
  });
});

describe('offset', () => {
  const square = [
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 10, y: 10 },
    { x: 0, y: 10 },
  ];
  it('offsets a CCW square outward to the right (negative distance)', () => {
    const out = offsetPath(square, -1, true);
    expect(out).toHaveLength(4);
    close(out[0], { x: -1, y: -1 });
    close(out[2], { x: 11, y: 11 });
  });
  it('detects the side of a point', () => {
    expect(sideOf(square, true, { x: 5, y: 5 })).toBe(1);
    expect(sideOf(square, true, { x: 5, y: -5 })).toBe(-1);
  });
  it('bevels very sharp corners instead of spiking', () => {
    const spike = [
      { x: 0, y: 0 },
      { x: 100, y: 1 },
      { x: 0, y: 2 },
    ];
    const out = offsetPath(spike, 1, false);
    expect(out.length).toBe(4);
  });
});

describe('shoelace on survey coordinates', () => {
  it('keeps the parcel area exact at TM magnitudes', () => {
    // 20.123 × 30.456 m at E 487 123, N 4 420 100: raw products would lose ~10⁻⁴ m².
    const E = 487123.456;
    const N = 4420100.789;
    const ring = [
      { x: E, y: N },
      { x: E + 20.123, y: N },
      { x: E + 20.123, y: N + 30.456 },
      { x: E, y: N + 30.456 },
    ];
    // Compared with the sides as stored (E + 20.123 is not exactly 20.123 away from E).
    const w = ring[1].x - ring[0].x;
    const h = ring[2].y - ring[1].y;
    expect(signedArea(ring)).toBeCloseTo(w * h, 9);
    // Raw shoelace on the absolute values for comparison: visibly off.
    let raw = 0;
    for (let i = 0, j = 3; i < 4; j = i++) raw += ring[j].x * ring[i].y - ring[i].x * ring[j].y;
    expect(Math.abs(raw / 2 - w * h)).toBeGreaterThan(1e-9);
  });
});
