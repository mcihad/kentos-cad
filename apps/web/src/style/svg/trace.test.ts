import { describe, expect, it } from 'vitest';
import { flattenSubPath, type SubPath } from './pathData';
import { inkMask, nestRings, ringArea, simplifyRing, traceBitmap, traceContours, type Bitmap } from './trace';

/** A white RGBA bitmap with black where `ink(x, y)` holds. */
function bitmap(w: number, h: number, ink: (x: number, y: number) => boolean): Bitmap {
  const data = new Uint8ClampedArray(w * h * 4).fill(255);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      if (ink(x, y)) {
        const i = (y * w + x) * 4;
        data[i] = data[i + 1] = data[i + 2] = 0;
      }
  return { width: w, height: h, data };
}

const area = (sp: SubPath) => Math.abs(ringArea(flattenSubPath(sp, 12)));
const bounds = (sp: SubPath) => {
  const pts = sp.nodes.map((n) => [n.x, n.y]);
  return [Math.min(...pts.map((p) => p[0])), Math.min(...pts.map((p) => p[1])), Math.max(...pts.map((p) => p[0])), Math.max(...pts.map((p) => p[1]))];
};

describe('trace bitmap', () => {
  it('reads brightness with alpha over white, and inverts', () => {
    const img: Bitmap = { width: 3, height: 1, data: [0, 0, 0, 255, 0, 0, 0, 0, 200, 200, 200, 255] };
    expect([...inkMask(img, 128)]).toEqual([1, 0, 0]);
    expect([...inkMask(img, 128, true)]).toEqual([0, 1, 1]);
  });

  it('outlines a single pixel as one ring, outer rings positive', () => {
    const rings = traceContours([1], 1, 1);
    expect(rings).toHaveLength(1);
    expect(ringArea(rings[0])).toBe(0.5);
  });

  it('traces a square with sharp corners on the pixel edges', () => {
    const r = traceBitmap(bitmap(20, 20, (x, y) => x >= 5 && x < 15 && y >= 5 && y < 15), { smooth: 0 });
    expect(r.shapes).toHaveLength(1);
    const sq = r.shapes[0].outer;
    expect(sq.nodes).toHaveLength(4);
    expect(bounds(sq)).toEqual([5, 5, 15, 15]);
    expect(area(sq)).toBeCloseTo(100, 6);
    expect(sq.nodes.every((n) => !n.in && !n.out)).toBe(true);
  });

  it('traces a ring with its hole, nested into one shape', () => {
    const img = bitmap(30, 30, (x, y) => x >= 5 && x < 25 && y >= 5 && y < 25 && !(x >= 11 && x < 19 && y >= 11 && y < 19));
    const rings = traceContours(inkMask(img, 128), 30, 30);
    expect(rings.map((r) => Math.sign(ringArea(r))).sort()).toEqual([-1, 1]);
    const r = traceBitmap(img, { smooth: 0 });
    expect(r.shapes).toHaveLength(1);
    expect(r.holes).toBe(1);
    expect(area(r.shapes[0].outer)).toBeCloseTo(400, 6);
    expect(area(r.shapes[0].holes[0])).toBeCloseTo(64, 6);
    // An island inside the hole is its own shape.
    const island = traceBitmap(bitmap(30, 30, (x, y) => (x >= 5 && x < 25 && y >= 5 && y < 25 && !(x >= 9 && x < 21 && y >= 9 && y < 21)) || (x >= 13 && x < 17 && y >= 13 && y < 17)), { smooth: 0, speckle: 0 });
    expect(island.shapes.map((s) => s.holes.length)).toEqual([1, 0]);
  });

  it('simplifies a diagonal band’s staircase to a few straight edges', () => {
    const img = bitmap(40, 40, (x, y) => Math.abs(x - y) <= 3 && x > 2 && x < 37);
    const r = traceBitmap(img, { smooth: 0, tolerance: 1 });
    expect(r.shapes).toHaveLength(1);
    const band = r.shapes[0].outer;
    expect(band.nodes.length).toBeLessThanOrEqual(8);
    // The long edges run at 45°.
    const edges = band.nodes.map((n, i) => {
      const m = band.nodes[(i + 1) % band.nodes.length];
      return { len: Math.hypot(m.x - n.x, m.y - n.y), ang: Math.abs((Math.atan2(m.y - n.y, m.x - n.x) * 180) / Math.PI) };
    });
    const long = edges.filter((e) => e.len > 20);
    expect(long).toHaveLength(2);
    for (const e of long) expect(Math.abs(e.ang - 45) < 3 || Math.abs(e.ang - 135) < 3).toBe(true);
  });

  it('drops specks and pinholes below the area but keeps the drawing', () => {
    let seed = 7;
    const rand = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
    const noise = new Set<number>();
    // Noise away from the square's edge (a speck touching it would be part of it).
    const nearEdge = (x: number, y: number) => x >= 18 && x < 46 && y >= 18 && y < 46 && !(x >= 22 && x < 42 && y >= 22 && y < 42);
    for (let i = 0; i < 60; i++) {
      const p = Math.floor(rand() * 64 * 64);
      if (!nearEdge(p % 64, Math.floor(p / 64))) noise.add(p);
    }
    const img = bitmap(64, 64, (x, y) => {
      const inSquare = x >= 20 && x < 44 && y >= 20 && y < 44;
      return noise.has(y * 64 + x) ? !inSquare : inSquare;
    });
    const raw = traceBitmap(img, { speckle: 0 });
    expect(raw.shapes.length).toBeGreaterThan(20);
    const clean = traceBitmap(img, { speckle: 6, smooth: 0 });
    expect(clean.shapes).toHaveLength(1);
    expect(clean.holes).toBe(0);
    expect(clean.removed).toBeGreaterThan(20);
    expect(bounds(clean.shapes[0].outer)).toEqual([20, 20, 44, 44]);
  });

  it('gives a disc smooth nodes with handles, and keeps its area', () => {
    const r = traceBitmap(bitmap(60, 60, (x, y) => Math.hypot(x + 0.5 - 30, y + 0.5 - 30) < 20), { smooth: 1, tolerance: 0.8 });
    const disc = r.shapes[0].outer;
    expect(disc.nodes.every((n) => n.in && n.out)).toBe(true);
    expect(disc.nodes.length).toBeLessThan(40);
    expect(Math.abs(area(disc) - Math.PI * 400) / (Math.PI * 400)).toBeLessThan(0.02);
  });

  it('simplifies a closed ring from an extreme point and nests with a minimum area', () => {
    const square: [number, number][] = [];
    for (let i = 0; i < 10; i++) square.push([i, 0]);
    for (let i = 0; i < 10; i++) square.push([10, i]);
    for (let i = 10; i > 0; i--) square.push([i, 10]);
    for (let i = 10; i > 0; i--) square.push([0, i]);
    expect(simplifyRing(square, 0.1)).toHaveLength(4);
    const { shapes, removed } = nestRings([square, [[0, 0], [1, 0], [1, 1]]], 2);
    expect([shapes.length, removed]).toEqual([1, 1]);
  });
});
