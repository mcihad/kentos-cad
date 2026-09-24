import { describe, expect, it } from 'vitest';
import { signedArea } from '../geometry';
import { arcEnd, arcStart, sweep } from './arc';
import { bulgeRingArea } from './bulge';
import {
  arcStartCenterAngle,
  cloudOf,
  arcStartCenterChord,
  arcStartCenterEnd,
  arcStartEndAngle,
  arcStartEndDirection,
  arcStartEndRadius,
  rectFromCorners,
  rectFromEdge,
  rectFromSize,
  regularPolygon,
  regularPolygonOnEdge,
  sideDistance,
} from './shapes';

const v = (x: number, y: number) => ({ x, y });
const close = (a: { x: number; y: number }, b: { x: number; y: number }) => {
  expect(a.x).toBeCloseTo(b.x, 9);
  expect(a.y).toBeCloseTo(b.y, 9);
};

describe('rectangles', () => {
  it('builds a rotated rectangle on an edge, width to the left', () => {
    const r = rectFromEdge(v(0, 0), v(3, 4), 2)!;
    expect(Math.abs(signedArea(r))).toBeCloseTo(10, 9);
    close(r[3], v(-1.6, 1.2));
    expect(sideDistance(v(0, 0), v(3, 4), v(-1.6, 1.2))).toBeCloseTo(2, 9);
  });
  it('flips to the right for a negative width', () => {
    expect(signedArea(rectFromEdge(v(0, 0), v(10, 0), -3)!)).toBeLessThan(0);
  });
  it('rotates a corner-to-corner rectangle', () => {
    const r = rectFromCorners(v(0, 0), v(0, 2), Math.PI / 4)!;
    expect(Math.abs(signedArea(r))).toBeCloseTo(2, 9);
  });
  it('places a sized rectangle in the quadrant of the pick', () => {
    const r = rectFromSize(v(10, 10), 4, 2, 0, v(5, 5))!;
    close(r[2], v(6, 8));
  });
});

describe('regular polygons', () => {
  it('inscribes a hexagon: the pick is a vertex', () => {
    const p = regularPolygon(v(0, 0), 6, v(10, 0), 'inscribed')!;
    expect(p).toHaveLength(6);
    close(p[0], v(10, 0));
    expect(Math.hypot(p[3].x, p[3].y)).toBeCloseTo(10, 9);
  });
  it('circumscribes a square: the pick is an edge middle', () => {
    const p = regularPolygon(v(0, 0), 4, v(5, 0), 'circumscribed')!;
    close(p[0], v(5, -5));
    close(p[1], v(5, 5));
  });
  it('builds a polygon counter-clockwise on an edge', () => {
    const p = regularPolygonOnEdge(v(0, 0), v(10, 0), 4)!;
    expect(p.map((q) => [Math.round(q.x), Math.round(q.y)])).toEqual([[0, 0], [10, 0], [10, 10], [0, 10]]);
    expect(signedArea(p)).toBeCloseTo(100, 9);
  });
});

describe('arc modes', () => {
  it('start, centre, end runs counter-clockwise', () => {
    const a = arcStartCenterEnd(v(10, 0), v(0, 0), v(0, 5))!;
    close(arcStart(a), v(10, 0));
    close(arcEnd(a), v(0, 10));
  });
  it('start, centre, negative angle runs clockwise (stored CCW)', () => {
    const a = arcStartCenterAngle(v(10, 0), v(0, 0), -90)!;
    close(arcStart(a), v(0, -10));
    close(arcEnd(a), v(10, 0));
  });
  it('start, centre, chord length', () => {
    const a = arcStartCenterChord(v(10, 0), v(0, 0), Math.SQRT2 * 10)!;
    expect(sweep(a.a0, a.a1)).toBeCloseTo(Math.PI / 2, 9);
    const major = arcStartCenterChord(v(10, 0), v(0, 0), -Math.SQRT2 * 10)!;
    expect(sweep(major.a0, major.a1)).toBeCloseTo((3 * Math.PI) / 2, 9);
    expect(arcStartCenterChord(v(10, 0), v(0, 0), 25)).toBeNull();
  });
  it('start, end, angle and start, end, radius agree', () => {
    const a = arcStartEndAngle(v(10, 0), v(0, 10), 90)!;
    close(a.c, v(0, 0));
    const b = arcStartEndRadius(v(10, 0), v(0, 10), 10)!;
    close(b.c, v(0, 0));
    expect(sweep(b.a0, b.a1)).toBeCloseTo(Math.PI / 2, 9);
  });
  it('start, end, tangent direction', () => {
    // Leaving (10,0) straight up and ending at (0,10): a quarter circle about the origin.
    const a = arcStartEndDirection(v(10, 0), v(0, 10), v(0, 1))!;
    close(a.c, v(0, 0));
  });
  it('rejects a radius too small for the chord', () => {
    expect(arcStartEndRadius(v(0, 0), v(10, 0), 4)).toBeNull();
  });
});

describe('revizyon bulutu', () => {
  it('divides the sides into chords of about the arc length, all bulging outwards', () => {
    // Clockwise input is turned counter-clockwise, so positive bulges point outside.
    const c = cloudOf([v(0, 0), v(0, 10), v(20, 10), v(20, 0)], 5)!;
    expect(c.pts).toHaveLength(2 + 4 + 2 + 4);
    expect(c.bulges.every((b) => b > 0)).toBe(true);
    expect(bulgeRingArea(c.pts, c.bulges)).toBeGreaterThan(200);
    expect(cloudOf([v(0, 0), v(1, 1)], 5)).toBeNull();
  });
});
