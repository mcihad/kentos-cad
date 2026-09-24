import { describe, expect, it } from 'vitest';
import { alongLine, clockwiseAngle, distanceIntersection, lineIntersection, polarPoint, sideOffsets, sidePoint } from './survey';

const v = (x: number, y: number) => ({ x, y });
const close = (a: { x: number; y: number } | null, b: { x: number; y: number }) => {
  expect(a).not.toBeNull();
  expect(a!.x).toBeCloseTo(b.x, 9);
  expect(a!.y).toBeCloseTo(b.y, 9);
};

describe('surveying constructions', () => {
  it('yan nokta: abscissa along the line, ordinate positive to the right', () => {
    // Line heading north: right is east.
    close(sidePoint(v(0, 0), v(0, 100), 30, 5), v(5, 30));
    close(sidePoint(v(0, 0), v(0, 100), 30, -5), v(-5, 30));
    expect(sideOffsets(v(0, 0), v(0, 100), v(5, 30))).toEqual({ absis: 30, ordinat: 5 });
  });
  it('kenar kesişimi: both solutions, the right-hand one first', () => {
    // d1 = d2 = 5 on a 10 m base: the circles touch at the middle.
    const [r, l] = distanceIntersection(v(0, 0), v(10, 0), 5, 5);
    close(r, v(5, 0));
    expect(l).toBeUndefined();
    const two = distanceIntersection(v(0, 0), v(10, 0), 5 * Math.SQRT2, 5 * Math.SQRT2);
    close(two[0], v(5, -5));
    close(two[1], v(5, 5));
    expect(distanceIntersection(v(0, 0), v(10, 0), 1, 1)).toEqual([]);
  });
  it('doğru kesişimi of two unbounded lines', () => {
    close(lineIntersection(v(0, 0), v(1, 1), v(10, 0), v(9, 1)), v(5, 5));
    expect(lineIntersection(v(0, 0), v(1, 0), v(0, 1), v(1, 1))).toBeNull();
  });
  it('hat üzerinde nokta', () => {
    close(alongLine(v(0, 0), v(30, 40), 25), v(15, 20));
  });
  it('açı-mesafe: clockwise from the reference direction', () => {
    // Reference north, 90° clockwise → east.
    close(polarPoint(v(0, 0), v(0, 10), Math.PI / 2, 7), v(7, 0));
    expect(clockwiseAngle(v(0, 0), v(0, 10), v(7, 0))).toBeCloseTo(Math.PI / 2, 12);
  });
});
