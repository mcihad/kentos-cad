import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';
import { allFaces, faceAt, insideArea, intersectAreas, netArea, ringArea, splitArea, subtractAreas, unionAreas, type Area, type Ring, type Source } from './region';

const v = (x: number, y: number): Vec2 => ({ x, y });
const rect = (x0: number, y0: number, x1: number, y1: number): Area => ({ outer: { pts: [v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)] }, holes: [] });
/** Circle as two half-circle bulges (the form a polygon entity stores). */
const disk = (cx: number, cy: number, r: number): Area => ({ outer: { pts: [v(cx + r, cy), v(cx - r, cy)], bulges: [1, 1] }, holes: [] });
const total = (list: Area[]) => list.reduce((s, a) => s + netArea(a), 0);
const lines = (...segs: [Vec2, Vec2][]): Source => ({ edges: segs.map(([a, b]): Edge => ({ kind: 'seg', a, b })), cut: true });
const hasPoint = (r: Ring, p: Vec2) => r.pts.some((q) => q.x === p.x && q.y === p.y);

describe('area booleans: straight edges', () => {
  it('union of two overlapping squares', () => {
    const r = unionAreas([rect(0, 0, 10, 10), rect(5, 5, 15, 15)]);
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(175, 9);
    expect(r[0].outer.pts).toHaveLength(8);
    expect(r[0].holes).toHaveLength(0);
    expect(ringArea(r[0].outer)).toBeGreaterThan(0);
  });
  it('union of neighbours sharing an edge removes the shared edge and keeps its corners', () => {
    const r = unionAreas([rect(0, 0, 10, 10), rect(10, 0, 20, 10)]);
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(200, 9);
    // (10,0) and (10,10) are surveyed corners of both parcels: they stay.
    expect(r[0].outer.pts).toHaveLength(6);
    expect(hasPoint(r[0].outer, v(10, 0)) && hasPoint(r[0].outer, v(10, 10))).toBe(true);
  });
  it('union with a T-junction keeps the neighbour corners on the shared line', () => {
    const r = unionAreas([rect(0, 0, 10, 10), rect(10, 2, 20, 8)]);
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(160, 9);
    for (const p of [v(10, 0), v(10, 2), v(10, 8), v(10, 10)]) expect(hasPoint(r[0].outer, p)).toBe(true);
  });
  it('intersection and difference of overlapping squares', () => {
    const i = intersectAreas([rect(0, 0, 10, 10), rect(5, 5, 15, 15)]);
    expect(i).toHaveLength(1);
    expect(netArea(i[0])).toBeCloseTo(25, 9);
    const d = subtractAreas([rect(0, 0, 10, 10)], [rect(5, 5, 15, 15)]);
    expect(d).toHaveLength(1);
    expect(netArea(d[0])).toBeCloseTo(75, 9);
    expect(d[0].outer.pts).toHaveLength(6);
  });
  it('difference with a cutter strictly inside leaves a hole', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [rect(4, 4, 6, 6)]);
    expect(d).toHaveLength(1);
    expect(d[0].holes).toHaveLength(1);
    expect(ringArea(d[0].holes[0])).toBeLessThan(0);
    expect(netArea(d[0])).toBeCloseTo(96, 9);
    expect(insideArea(d[0], v(5, 5))).toBe(false);
    expect(insideArea(d[0], v(1, 1))).toBe(true);
  });
  it('difference touching the boundary makes a notch, not a hole', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [rect(0, 0, 2, 2)]);
    expect(d).toHaveLength(1);
    expect(d[0].holes).toHaveLength(0);
    expect(netArea(d[0])).toBeCloseTo(96, 9);
    expect(d[0].outer.pts).toHaveLength(6);
  });
  it('a hole touching the outer ring at one point is still a hole', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [{ outer: { pts: [v(5, 0), v(7, 2), v(5, 4), v(3, 2)] }, holes: [] }]);
    expect(total(d)).toBeCloseTo(92, 9);
    for (const a of d) expect(ringArea(a.outer)).toBeGreaterThan(0);
  });
  it('disjoint areas stay separate; squares meeting at a corner give two rings', () => {
    expect(unionAreas([rect(0, 0, 1, 1), rect(5, 5, 6, 6)])).toHaveLength(2);
    const r = unionAreas([rect(0, 0, 1, 1), rect(1, 1, 2, 2)]);
    expect(r).toHaveLength(2);
    expect(total(r)).toBeCloseTo(2, 12);
  });
  it('subtracting everything leaves nothing; no overlap leaves the area unchanged', () => {
    expect(subtractAreas([rect(2, 2, 3, 3)], [rect(0, 0, 10, 10)])).toHaveLength(0);
    const d = subtractAreas([rect(0, 0, 1, 1)], [rect(5, 5, 6, 6)]);
    expect(total(d)).toBeCloseTo(1, 12);
    expect(intersectAreas([rect(0, 0, 1, 1), rect(5, 5, 6, 6)])).toHaveLength(0);
  });
  it('a patch over a hole fills it', () => {
    const holed: Area = { outer: rect(0, 0, 10, 10).outer, holes: [{ pts: [v(4, 4), v(4, 6), v(6, 6), v(6, 4)] }] };
    const r = unionAreas([holed, rect(3, 3, 7, 7)]);
    expect(r).toHaveLength(1);
    expect(r[0].holes).toHaveLength(0);
    expect(netArea(r[0])).toBeCloseTo(100, 9);
  });
  it('clockwise input rings are read the same way', () => {
    const cw: Area = { outer: { pts: [v(0, 0), v(0, 10), v(10, 10), v(10, 0)] }, holes: [] };
    expect(netArea(unionAreas([cw, rect(5, 5, 15, 15)])[0])).toBeCloseTo(175, 9);
  });
  it('three areas at once', () => {
    const r = unionAreas([rect(0, 0, 2, 1), rect(1, 0, 3, 1), rect(2, 0, 4, 1)]);
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(4, 12);
    expect(netArea(intersectAreas([rect(0, 0, 3, 1), rect(1, 0, 4, 1), rect(2, 0, 5, 1)])[0])).toBeCloseTo(1, 12);
  });
});

describe('area booleans: arcs stay arcs', () => {
  it('union of two circles', () => {
    const r = unionAreas([disk(0, 0, 1), disk(1, 0, 1)]);
    expect(r).toHaveLength(1);
    const lens = (2 * Math.PI) / 3 - Math.sqrt(3) / 2;
    expect(netArea(r[0])).toBeCloseTo(2 * Math.PI - lens, 10);
    expect(r[0].outer.bulges?.every((b) => b !== 0)).toBe(true);
  });
  it('intersection of two circles is the lens', () => {
    const r = intersectAreas([disk(0, 0, 1), disk(1, 0, 1)]);
    expect(netArea(r[0])).toBeCloseTo((2 * Math.PI) / 3 - Math.sqrt(3) / 2, 10);
    // The crossings plus each circle's own point inside the other (input vertices stay).
    expect(r[0].outer.pts).toHaveLength(4);
    expect(r[0].outer.bulges?.every((b) => b > 0)).toBe(true);
  });
  it('square minus a circle in the middle: a round hole', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [disk(5, 5, 2)]);
    expect(d[0].holes).toHaveLength(1);
    expect(netArea(d[0])).toBeCloseTo(100 - 4 * Math.PI, 10);
  });
  it('square minus a circle on its edge: a round notch', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [disk(5, 0, 2)]);
    expect(d).toHaveLength(1);
    expect(d[0].holes).toHaveLength(0);
    expect(netArea(d[0])).toBeCloseTo(100 - 2 * Math.PI, 10);
  });
  it('half disks sharing their chord join into the whole disk', () => {
    const upper: Area = { outer: { pts: [v(0, 0), v(2, 0)], bulges: [0, 1] }, holes: [] };
    const lower: Area = { outer: { pts: [v(2, 0), v(0, 0)], bulges: [0, 1] }, holes: [] };
    const r = unionAreas([upper, lower]);
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(Math.PI, 12);
  });
  it('a shared arc edge: the bump comes off along its chord', () => {
    const bumped: Area = { outer: { pts: [v(0, 0), v(2, 0), v(2, 2), v(0, 2)], bulges: [0.5, 0, 0, 0] }, holes: [] };
    const bump: Area = { outer: { pts: [v(2, 0), v(0, 0)], bulges: [0, 0.5] }, holes: [] };
    expect(netArea(unionAreas([bumped, bump])[0])).toBeCloseTo(netArea(bumped), 12);
    const d = subtractAreas([bumped], [bump]);
    expect(d).toHaveLength(1);
    expect(netArea(d[0])).toBeCloseTo(4, 12);
    expect(d[0].outer.bulges ?? []).toEqual(expect.not.arrayContaining([0.5]));
  });
});

describe('area booleans on survey coordinates', () => {
  const E = 487123.456;
  const N = 4420100.789;
  const parcel = (x0: number, y0: number, x1: number, y1: number) => rect(E + x0, N + y0, E + x1, N + y1);
  it('input corners come back bit for bit', () => {
    const a = parcel(0, 0, 20.123, 30.456);
    const b = parcel(20.123, 5, 41, 30.456);
    const r = unionAreas([a, b]);
    expect(r).toHaveLength(1);
    for (const p of [...a.outer.pts, ...b.outer.pts]) expect(hasPoint(r[0].outer, p)).toBe(true);
    expect(netArea(r[0])).toBeCloseTo(netArea(a) + netArea(b), 9);
  });
  it('a crossing point is exact to a hair', () => {
    const r = intersectAreas([parcel(0, 0, 10, 10), { outer: { pts: [v(E + 5, N - 2.5), v(E + 12.5, N + 5), v(E + 5, N + 12.5), v(E - 2.5, N + 5)] }, holes: [] }]);
    expect(netArea(r[0])).toBeCloseTo(100 - 4 * 3.125, 9);
    expect(r[0].outer.pts).toHaveLength(8);
  });
});

describe('splitting and faces', () => {
  it('a line across a square splits it in two', () => {
    const r = splitArea(rect(0, 0, 10, 10), lines([v(4, -1), v(4, 11)]));
    expect(r).toHaveLength(2);
    expect(r.map(netArea).sort((a, b) => a - b)).toEqual([expect.closeTo(40, 9), expect.closeTo(60, 9)]);
  });
  it('a line ending inside does not cut', () => {
    const r = splitArea(rect(0, 0, 10, 10), lines([v(4, -1), v(4, 5)]));
    expect(r).toHaveLength(1);
    expect(netArea(r[0])).toBeCloseTo(100, 9);
  });
  it('a zigzag and two crossing lines', () => {
    const z = splitArea(rect(0, 0, 10, 10), lines([v(-1, 2), v(5, 8)], [v(5, 8), v(11, 2)]));
    expect(z).toHaveLength(2);
    expect(total(z)).toBeCloseTo(100, 9);
    const x = splitArea(rect(0, 0, 10, 10), lines([v(5, -1), v(5, 11)], [v(-1, 5), v(11, 5)]));
    expect(x).toHaveLength(4);
    expect(x.every((a) => Math.abs(netArea(a) - 25) < 1e-9)).toBe(true);
  });
  it('splitting a holed area keeps the hole in the right piece', () => {
    const d = subtractAreas([rect(0, 0, 10, 10)], [rect(6, 4, 8, 6)]);
    const r = splitArea(d[0], lines([v(5, -1), v(5, 11)]));
    expect(r).toHaveLength(2);
    const right = r.find((a) => insideArea(a, v(9, 9)))!;
    expect(right.holes).toHaveLength(1);
    expect(netArea(right)).toBeCloseTo(46, 9);
  });
  it('face of overshooting lines, with an island as a hole', () => {
    const frame = lines([v(-1, 0), v(11, 0)], [v(10, -1), v(10, 11)], [v(11, 10), v(-1, 10)], [v(0, 11), v(0, -1)]);
    const island = lines([v(4, 4), v(6, 4)], [v(6, 4), v(6, 6)], [v(6, 6), v(4, 6)], [v(4, 6), v(4, 4)]);
    const f = faceAt([frame, island], v(2, 2))!;
    expect(f.holes).toHaveLength(1);
    expect(netArea(f)).toBeCloseTo(96, 9);
    expect(netArea(faceAt([frame, island], v(2, 2), false)!)).toBeCloseTo(100, 9);
    expect(netArea(faceAt([frame, island], v(5, 5))!)).toBeCloseTo(4, 9);
    expect(faceAt([frame], v(20, 20))).toBeNull();
  });
  it('a dangling line inside a face is ignored', () => {
    const frame = lines([v(0, 0), v(10, 0)], [v(10, 0), v(10, 10)], [v(10, 10), v(0, 10)], [v(0, 10), v(0, 0)], [v(0, 5), v(4, 5)]);
    const f = faceAt([frame], v(8, 8))!;
    expect(netArea(f)).toBeCloseTo(100, 9);
    expect(f.outer.pts).toHaveLength(5);
  });
  it('all faces of a grid of lines', () => {
    const grid = lines([v(0, 0), v(2, 0)], [v(0, 1), v(2, 1)], [v(0, 2), v(2, 2)], [v(0, 0), v(0, 2)], [v(1, 0), v(1, 2)], [v(2, 0), v(2, 2)]);
    const f = allFaces([grid]);
    expect(f).toHaveLength(4);
    expect(f.every((a) => Math.abs(netArea(a) - 1) < 1e-12)).toBe(true);
  });
  it('a circle and a line through it give two half-disk faces', () => {
    const circle: Source = { edges: [{ kind: 'arc', c: v(0, 0), r: 1, a0: 0, sweep: 2 * Math.PI }], cut: true };
    const f = allFaces([circle, lines([v(-2, 0), v(2, 0)])]);
    expect(f).toHaveLength(2);
    expect(f.every((a) => Math.abs(netArea(a) - Math.PI / 2) < 1e-12)).toBe(true);
  });
});

describe('overlay speed', () => {
  // The winding index of the classify step is tested in the core (geom/arrangement.rs).
  it('keeps booleans of many pieces fast (classify is no longer quadratic)', () => {
    // 30 × 30 overlapping squares: thousands of pieces; the old all-pairs step took tens of seconds.
    const squares: Area[] = [];
    for (let i = 0; i < 30; i++) for (let j = 0; j < 30; j++) squares.push(rect(i * 1.5, j * 1.5, i * 1.5 + 2, j * 1.5 + 2));
    const t0 = performance.now();
    const out = unionAreas(squares);
    const ms = performance.now() - t0;
    expect(out.length).toBe(1);
    expect(total(out)).toBeCloseTo(45.5 * 45.5, 6);
    expect(ms).toBeLessThan(15000);
  });
});
