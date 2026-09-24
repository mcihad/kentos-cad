import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { triangulate } from './triangulate';

const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x0: number, y0: number, s: number) => [v(x0, y0), v(x0 + s, y0), v(x0 + s, y0 + s), v(x0, y0 + s)];
const circle = (cx: number, cy: number, r: number, n = 48) => Array.from({ length: n }, (_, i) => v(cx + r * Math.cos((i / n) * 2 * Math.PI), cy + r * Math.sin((i / n) * 2 * Math.PI)));

/** Sum of the triangle areas (all must be counter-clockwise). */
function covered(out: number[]): number {
  let a = 0;
  for (let i = 0; i < out.length; i += 6) {
    const t = ((out[i + 2] - out[i]) * (out[i + 5] - out[i + 1]) - (out[i + 3] - out[i + 1]) * (out[i + 4] - out[i])) / 2;
    expect(t).toBeGreaterThanOrEqual(0);
    a += t;
  }
  return a;
}
const polyArea = (pts: Vec2[]) => Math.abs(pts.reduce((s, p, i) => s + p.x * pts[(i + 1) % pts.length].y - pts[(i + 1) % pts.length].x * p.y, 0)) / 2;

describe('triangulate', () => {
  const o = v(0, 0);
  it('a plain ring, either orientation', () => {
    const out: number[] = [];
    triangulate([...square(0, 0, 10)].reverse(), [], o, out);
    expect(covered(out)).toBeCloseTo(100, 9);
  });
  it('a square with a square hole', () => {
    const out: number[] = [];
    triangulate(square(0, 0, 10), [square(4, 4, 2)], o, out);
    expect(covered(out)).toBeCloseTo(96, 9);
  });
  it('two holes and a round one', () => {
    const out: number[] = [];
    const round = circle(7, 7, 1.5);
    triangulate(square(0, 0, 10), [square(1, 1, 2), square(1, 6, 2), round], o, out);
    expect(covered(out)).toBeCloseTo(100 - 4 - 4 - polyArea(round), 6);
  });
  it('a hole beside a notch (reflex vertex in the bridge triangle)', () => {
    const outer = [v(0, 0), v(10, 0), v(10, 10), v(6, 10), v(6, 5.5), v(5, 5.5), v(5, 10), v(0, 10)];
    const out: number[] = [];
    triangulate(outer, [square(2, 4, 1)], o, out);
    expect(covered(out)).toBeCloseTo(polyArea(outer) - 1, 9);
  });
});
