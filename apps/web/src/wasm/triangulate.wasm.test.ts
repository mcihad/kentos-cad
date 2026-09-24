import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { callNamed, triangulateMany } from './core';
import { callsOf, Gen } from './calls/harness';
import { fillPolygon, P8 } from './calls/sets/p8-triangulate';

/**
 * The typed batch entry point (`triangulateMany`, a layer's fills in one
 * call) against the named operation (`triangulate`), which the call
 * fixtures freeze (docs/adr/0008).
 * Triangles come back as indices into the packed points; the coordinates
 * they stand for must be exactly the named operation's.
 */

type Polygon = [Vec2[], Vec2[][]];

function pack(polys: readonly Polygon[]): { xy: Float64Array; ringSizes: Uint32Array; polyRings: Uint32Array } {
  const rings = polys.flatMap(([outer, holes]) => [outer, ...holes]);
  const xy = new Float64Array(rings.reduce((n, r) => n + 2 * r.length, 0));
  let k = 0;
  for (const r of rings)
    for (const p of r) {
      xy[k++] = p.x;
      xy[k++] = p.y;
    }
  return { xy, ringSizes: Uint32Array.from(rings, (r) => r.length), polyRings: Uint32Array.from(polys, ([, holes]) => 1 + holes.length) };
}

describe('triangulateMany', () => {
  it('gives the named operation’s triangles, polygon after polygon', () => {
    const named = callsOf(P8, 0).map((c) => [c.args[0], c.args[1]] as Polygon);
    const g = new Gen(8);
    const random = Array.from({ length: 400 }, (): Polygon => {
      g.frame();
      return fillPolygon(g);
    });
    const polys = [...named, ...random];
    const { xy, ringSizes, polyRings } = pack(polys);
    const idx = triangulateMany(xy, ringSizes, polyRings);
    expect(idx.length % 3).toBe(0);
    let at = 0;
    for (const [outer, holes] of polys) {
      const want = callNamed('triangulate', [outer, holes, { x: 0, y: 0 }]) as number[];
      const got: number[] = [];
      for (const i of idx.subarray(at, at + want.length / 2)) got.push(xy[2 * i], xy[2 * i + 1]);
      expect(got).toEqual(want);
      at += want.length / 2;
    }
    expect(at).toBe(idx.length);
  });

  it('covers a simple ring exactly: the triangles’ areas add up to the ring’s', () => {
    const g = new Gen(88);
    const rings = Array.from({ length: 300 }, () => {
      g.frame();
      return g.ring(g.int(3, 60), g.num(1, 40), g.chance(0.5));
    });
    const { xy, ringSizes, polyRings } = pack(rings.map((r): Polygon => [r, []]));
    const idx = triangulateMany(xy, ringSizes, polyRings);
    let at = 0;
    for (const r of rings) {
      // Shoelace about the first vertex, as the core measures areas (TM coordinates stay exact).
      const o = r[0];
      let area = 0;
      for (let i = 0, j = r.length - 1; i < r.length; j = i++) area += (r[j].x - o.x) * (r[i].y - o.y) - (r[i].x - o.x) * (r[j].y - o.y);
      area = Math.abs(area) / 2;
      const n = 3 * (r.length - 2); // a simple ring of n vertices has n − 2 triangles
      let covered = 0;
      for (let t = at; t < at + n; t += 3) {
        const [a, b, c] = [idx[t], idx[t + 1], idx[t + 2]].map((i) => ({ x: xy[2 * i] - o.x, y: xy[2 * i + 1] - o.y }));
        const tri = ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) / 2;
        expect(tri).toBeGreaterThanOrEqual(0);
        covered += tri;
      }
      expect(Math.abs(covered - area)).toBeLessThanOrEqual(1e-9 * Math.max(1, area));
      at += n;
    }
    expect(at).toBe(idx.length);
  });

  it('reads sizes beyond the data as the data’s end', () => {
    const { xy } = pack([[[{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }, { x: 0, y: 1 }], []]]);
    expect(Array.from(triangulateMany(xy, Uint32Array.of(9, 9), Uint32Array.of(2, 3))).length).toBe(6);
    expect(triangulateMany(new Float64Array(0), Uint32Array.of(3), Uint32Array.of(1)).length).toBe(0);
  });
});
