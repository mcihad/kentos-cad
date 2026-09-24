import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../geometry';
import {
  closestParam,
  ellipseArea,
  ellipseFromAxis,
  ellipseFromCenter,
  ellipseLength,
  ellipsePoint,
  ellipseTangentPoints,
  lineEllipse,
  onEllipse,
  paramAtPolar,
  paramOfPoint,
  quadrantParams,
  tessellateEllipse,
  type EllipseGeom,
} from './ellipse';

const v = (x: number, y: number): Vec2 => ({ x, y });
// 10 × 5 half-axes, major along +x.
const E: EllipseGeom = { c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 };

describe('ellipse', () => {
  it('evaluates points and parameters both ways', () => {
    expect(ellipsePoint(E, Math.PI / 2).y).toBeCloseTo(5, 12);
    expect(paramOfPoint(E, v(0, 5))).toBeCloseTo(Math.PI / 2, 12);
  });
  it('measures a circle exactly and an ellipse to Ramanujan precision', () => {
    expect(ellipseLength({ ...E, ratio: 1 })).toBeCloseTo(2 * Math.PI * 10, 9);
    const a = 10;
    const b = 5;
    const h = ((a - b) / (a + b)) ** 2;
    const ramanujan = Math.PI * (a + b) * (1 + (3 * h) / (10 + Math.sqrt(4 - 3 * h)));
    expect(ellipseLength(E)).toBeCloseTo(ramanujan, 6);
    expect(ellipseArea(E)).toBeCloseTo(Math.PI * 50, 12);
  });
  it('crosses a line exactly', () => {
    const hits = lineEllipse(E, v(-20, 0), v(20, 0));
    expect(hits.map((h) => ellipsePoint(E, h.t).x).sort((a, b) => a - b)).toEqual([-10, 10].map((x) => expect.closeTo(x, 12)));
    const diag = lineEllipse(E, v(0, 0), v(1, 1));
    for (const h of diag) {
      const p = ellipsePoint(E, h.t);
      expect(p.x).toBeCloseTo(p.y, 12);
      expect((p.x / 10) ** 2 + (p.y / 5) ** 2).toBeCloseTo(1, 12);
    }
  });
  it('finds the closest point with Newton steps', () => {
    const t = closestParam(E, v(3, 20));
    const p = ellipsePoint(E, t);
    // The residual from the true foot is orthogonal to the tangent.
    const d = { x: -10 * Math.sin(t), y: 5 * Math.cos(t) };
    expect((p.x - 3) * d.x + (p.y - 20) * d.y).toBeCloseTo(0, 9);
    expect(p.y).toBeGreaterThan(0);
  });
  it('never lands farther than the nearest point of the arc', () => {
    // Nearest point at an arc end: Newton from the end sample used to run off
    // to a stationary point farther away (or a maximum) and return it.
    const cases: [EllipseGeom, Vec2][] = [
      [{ c: v(8.52602543309331, 23.65580447949469), major: v(-6.9311634404584765, 4.354190295562148), ratio: 0.14283255736809225, t0: -0.2704085748711096, t1: -2.782506557835424 }, v(24.66441109776497, 44.74484540056437)],
      [{ c: v(23.198388097807765, -76.11601729877293), major: v(-1.5274271182715893, 24.21968382317573), ratio: 0.7157873110962101, t0: 3.5801842068941685, t1: 0 }, v(18.54403612203896, -70.73280825745314)],
      [{ c: v(10.812895325943828, -38.99370636790991), major: v(17.851013294421136, -24.863052782602608), ratio: 0.5212373115471565, t0: -1.6851746938558456, t1: 6.283185307179586 }, v(37.81510050408542, -25.53558237850666)],
      [{ c: v(486007.4576945044, 4420087.546784012), major: v(9.635122502222657, 9.76099387742579), ratio: 0.2948974524042569, t0: Math.PI, t1: -5.485320866888809 }, v(485992.7406639233, 4420095.807268135)],
      [{ c: v(485920.06134055555, 4419940.6982073095), major: v(-23.43082148116082, -10.015725544653833), ratio: 0.3182428688975051, t0: -3.610676042487226, t1: 0 }, v(485897.3410749389, 4419933.798946957)],
    ];
    for (const [e, p] of cases) {
      const d = (t: number) => {
        const q = ellipsePoint(e, t);
        return Math.hypot(q.x - p.x, q.y - p.y);
      };
      const sw = ((e.t1 - e.t0) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) || 2 * Math.PI;
      let dense = Infinity;
      for (let i = 0; i <= 100000; i++) dense = Math.min(dense, d(e.t0 + (sw * i) / 100000));
      const t = closestParam(e, p);
      expect(onEllipse(e, t)).toBe(true);
      expect(d(t)).toBeLessThanOrEqual(dense + 1e-9);
    }
    // Half a million dense samples through WASM take ~4 s alone, more on a busy machine.
  }, 30_000);
  it('finds the foot to full precision at TM coordinates', () => {
    const e: EllipseGeom = { c: v(486000.125, 4420000.375), major: v(18, 7), ratio: 0.4, t0: 0, t1: 0 };
    const p = v(486012.5, 4420021.25);
    const t = closestParam(e, p);
    // Measured from the centre, so the check itself loses nothing to the large coordinates.
    const m = { x: -e.major.y * e.ratio, y: e.major.x * e.ratio };
    const [c, s] = [Math.cos(t), Math.sin(t)];
    const r = { x: e.c.x - p.x + e.major.x * c + m.x * s, y: e.c.y - p.y + e.major.y * c + m.y * s };
    const dt = { x: -e.major.x * s + m.x * c, y: -e.major.y * s + m.y * c };
    expect(Math.abs(r.x * dt.x + r.y * dt.y) / (Math.hypot(r.x, r.y) * Math.hypot(dt.x, dt.y))).toBeLessThan(1e-13);
  });
  it('limits an arc to its range', () => {
    const arc = { ...E, t0: 0, t1: Math.PI / 2 };
    expect(quadrantParams(arc)).toEqual([0, Math.PI / 2]);
    expect(lineEllipse(arc, v(-20, 0), v(20, 0))).toHaveLength(1);
    expect(tessellateEllipse(arc).at(-1)!.y).toBeCloseTo(5, 12);
  });
  it('gives exact tangent points from outside', () => {
    const pts = ellipseTangentPoints(E, v(20, 0));
    expect(pts).toHaveLength(2);
    for (const p of pts) {
      // Tangency: (p − P)·normal = 0 with normal (x/a², y/b²).
      expect((20 - p.x) * (p.x / 100) + (0 - p.y) * (p.y / 25)).toBeCloseTo(0, 9);
    }
  });
  it('maps a polar angle to its parameter', () => {
    const t = paramAtPolar(E, Math.PI / 4);
    const p = ellipsePoint(E, t);
    expect(p.x).toBeCloseTo(p.y, 12);
  });
  it('builds from an axis and makes the longer axis the major one', () => {
    const e = ellipseFromAxis(v(-4, 0), v(4, 0), 10)!;
    expect(Math.hypot(e.major.x, e.major.y)).toBeCloseTo(10, 12);
    expect(e.ratio).toBeCloseTo(0.4, 12);
    expect(ellipseFromCenter(v(0, 0), v(0, 0), 3)).toBeNull();
  });
});
