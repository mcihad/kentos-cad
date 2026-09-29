import { describe, expect, it } from 'vitest';
import type { LineEntity } from '../entities';
import { angleAt, baselineDimension, bearingBearing, continueDimension, distanceDistance, pointsBetween, sectorRing } from './construct';
import { fenceCrossings } from './trim';

/** The wrappers reach the core's Faz 2 and 3 functions (docs/adr/0140); the core's own tests hold the numbers. */
const v = (x: number, y: number) => ({ x, y });

describe('constructions of docs/adr/0140', () => {
  it('a quarter sector is the centre, the arc ends and one arc edge', () => {
    const s = sectorRing(v(0, 0), 10, 0, Math.PI / 2)!;
    expect(s.pts).toHaveLength(3);
    expect(s.pts[1].x).toBeCloseTo(10);
    expect(s.pts[2].y).toBeCloseTo(10);
    expect(s.bulges?.[1]).toBeCloseTo(Math.tan(Math.PI / 8));
    expect(sectorRing(v(0, 0), 0, 0, 1)).toBeNull();
  });

  it('points between by parts, distances and ratios', () => {
    expect(pointsBetween(v(0, 0), v(12, 0), { parts: 3 })).toEqual([v(4, 0), v(8, 0)]);
    expect(pointsBetween(v(0, 0), v(10, 0), { distances: [3] })).toEqual([v(3, 0)]);
    expect(pointsBetween(v(0, 0), v(10, 0), { ratios: [0.25] })).toEqual([v(2.5, 0)]);
    expect(pointsBetween(v(0, 0), v(0, 0), { parts: 2 })).toBeNull();
  });

  it('two distances meet in two points, two bearings in one ahead of both', () => {
    expect(distanceDistance(v(0, 0), 5, v(8, 0), 5)).toHaveLength(2);
    expect(distanceDistance(v(0, 0), 1, v(8, 0), 1)).toEqual([]);
    const p = bearingBearing(v(0, 0), Math.PI / 4, v(10, 0), (7 * Math.PI) / 4)!;
    expect(p.x).toBeCloseTo(5);
    expect(p.y).toBeCloseTo(5);
    expect(bearingBearing(v(0, 0), (5 * Math.PI) / 4, v(10, 0), (3 * Math.PI) / 4)).toBeNull();
  });

  it('an angle at a vertex has its sweep, inner and outer', () => {
    const a = angleAt(v(0, 0), v(1, 0), v(0, 1))!;
    expect(a.sweep).toBeCloseTo(Math.PI / 2);
    expect(a.inner).toBeCloseTo(Math.PI / 2);
    expect(a.outer).toBeCloseTo((3 * Math.PI) / 2);
    expect(angleAt(v(0, 0), v(0, 0), v(0, 1))).toBeNull();
  });

  it('a chain and a baseline continue a dimension along its direction', () => {
    const base = { a: v(0, 0), b: v(10, 0), offset: 3, height: 0.5 };
    const c = continueDimension(base, v(10, 0), v(25, 0))!;
    expect(c).toMatchObject({ a: v(10, 0), b: v(25, 0), style: 'linear', angle: 0 });
    expect(c.offset).toBeCloseTo(3);
    const b = baselineDimension(base, v(25, 0), 2, 1.5)!;
    expect(b).toMatchObject({ a: v(0, 0), b: v(25, 0), style: 'linear' });
    expect(b.offset).toBeCloseTo(6);
    expect(baselineDimension(base, v(25, 0), 1, 0)).toBeNull();
  });

  it('a fence crosses an object where it crosses it, in the fence order', () => {
    const line: LineEntity = { id: 1, layerId: 'x', attrs: {}, kind: 'line', a: v(0, 0), b: v(10, 0) };
    const hits = fenceCrossings(line, [v(2, -1), v(2, 1), v(7, 1), v(7, -1)]);
    expect(hits).toEqual([v(2, 0), v(7, 0)]);
    expect(fenceCrossings(line, [v(0, 5), v(10, 5)])).toEqual([]);
  });
});
