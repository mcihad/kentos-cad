import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import { elevationAt, gripElevation, hasVertexElevation, mapElevations, nearestVertex, spaceLength, summarizeElevations, takesElevation, uniformElevation, vertexElevations } from './elevationValues';

/**
 * The vertex elevations the interface reads and writes (docs/adr/0142): what a list of them comes to, the length in
 * space by hand, the elevation under a grip and at a snapped point, and the geometry Kot ver writes.
 */
const p = (x: number, y: number) => ({ x, y });
const base = { id: 1, layerId: 'cizim', attrs: {} };
const line = (a: [number, number], b: [number, number], za?: number, zb?: number): Entity => ({ ...base, kind: 'line', a: p(...a), b: p(...b), ...(za !== undefined ? { za } : {}), ...(zb !== undefined ? { zb } : {}) });
const poly = (kind: 'polyline' | 'polygon', pts: [number, number][], zs?: (number | null)[], more: object = {}): Entity => ({ ...base, kind, pts: pts.map(([x, y]) => p(x, y)), ...(zs ? { zs } : {}), ...more }) as Entity;
const SQUARE: [number, number][] = [[0, 0], [10, 0], [10, 10], [0, 10]];

describe('vertex elevations', () => {
  it('lists every vertex in one order: the ends, the vertices, the outer ring then each hole', () => {
    expect(vertexElevations(line([0, 0], [5, 0], 1, 2))).toEqual([1, 2]);
    expect(vertexElevations(line([0, 0], [5, 0], undefined, 2))).toEqual([null, 2]);
    expect(vertexElevations(poly('polyline', SQUARE.slice(0, 3), [1, null, 3]))).toEqual([1, null, 3]);
    const holed = poly('polygon', SQUARE, [1, 2, 3, 4], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [7, 8, 9] }, { pts: [p(6, 6), p(8, 6), p(8, 8)] }] });
    // The second hole has none: its three vertices are null, in their place.
    expect(vertexElevations(holed)).toEqual([1, 2, 3, 4, 7, 8, 9, null, null, null]);
    expect(vertexElevations({ ...base, kind: 'point', p: p(0, 0), z: 12.5 })).toEqual([12.5]);
    expect(vertexElevations({ ...base, kind: 'point', p: p(0, 0) })).toEqual([null]);
    expect(vertexElevations({ ...base, kind: 'circle', c: p(0, 0), r: 1 })).toEqual([]);
  });

  it('knows which kinds take elevations and whether any vertex has one', () => {
    expect(['point', 'line', 'polyline', 'polygon'].every((kind) => takesElevation({ kind } as Entity))).toBe(true);
    expect(['circle', 'arc', 'ellipse', 'spline', 'text', 'hatch', 'dimension', 'xline', 'ray'].some((kind) => takesElevation({ kind } as Entity))).toBe(false);
    expect(hasVertexElevation(line([0, 0], [5, 0]))).toBe(false);
    expect(hasVertexElevation(line([0, 0], [5, 0], undefined, 0))).toBe(true);
    // 0 is an elevation; null is none.
    expect(hasVertexElevation(poly('polyline', SQUARE.slice(0, 3), [null, null, null]))).toBe(false);
    expect(hasVertexElevation(poly('polyline', SQUARE.slice(0, 3), [null, 0, null]))).toBe(true);
    // A hole's alone counts.
    expect(hasVertexElevation(poly('polygon', SQUARE, undefined, { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [null, 1, null] }] }))).toBe(true);
  });
});

describe('the one elevation of every vertex of an object', () => {
  it('the number all its vertices have, null when none has one, mixed otherwise', () => {
    expect(uniformElevation(line([0, 0], [5, 0], 7, 7))).toBe(7);
    expect(uniformElevation(line([0, 0], [5, 0], 7, 8))).toBe('mixed');
    expect(uniformElevation(line([0, 0], [5, 0], 7))).toBe('mixed');
    expect(uniformElevation(line([0, 0], [5, 0]))).toBeNull();
    // 0 is an elevation.
    expect(uniformElevation(line([0, 0], [5, 0], 0, 0))).toBe(0);
    expect(uniformElevation(poly('polyline', SQUARE.slice(0, 3), [4, 4, 4]))).toBe(4);
    expect(uniformElevation(poly('polyline', SQUARE.slice(0, 3), [4, null, 4]))).toBe('mixed');
    expect(uniformElevation(poly('polyline', SQUARE.slice(0, 3), [4, 4, 5]))).toBe('mixed');
    expect(uniformElevation(poly('polyline', SQUARE.slice(0, 3)))).toBeNull();
    expect(uniformElevation({ ...base, kind: 'point', p: p(0, 0), z: 3 })).toBe(3);
    expect(uniformElevation({ ...base, kind: 'point', p: p(0, 0) })).toBeNull();
    expect(uniformElevation({ ...base, kind: 'circle', c: p(0, 0), r: 1 })).toBeNull();
  });

  it('an area’s holes are part of it: all the same, or mixed', () => {
    const ring = (zs?: (number | null)[]) => ({ pts: [p(2, 2), p(4, 2), p(4, 4)], ...(zs ? { zs } : {}) });
    expect(uniformElevation(poly('polygon', SQUARE, [4, 4, 4, 4], { holes: [ring([4, 4, 4])] }))).toBe(4);
    expect(uniformElevation(poly('polygon', SQUARE, [4, 4, 4, 4], { holes: [ring([4, 4, 5])] }))).toBe('mixed');
    expect(uniformElevation(poly('polygon', SQUARE, [4, 4, 4, 4], { holes: [ring()] }))).toBe('mixed');
    // A hole with an elevation and a ring without any (the ring's first vertex has none): mixed.
    expect(uniformElevation(poly('polygon', SQUARE, undefined, { holes: [ring([4, 4, 4])] }))).toBe('mixed');
  });
});

describe('what a list of elevations comes to', () => {
  it('none, one value, a range, and a range of those that have one', () => {
    expect(summarizeElevations([null, null])).toEqual({ kind: 'none' });
    expect(summarizeElevations([])).toEqual({ kind: 'none' });
    expect(summarizeElevations([100, 100, 100])).toEqual({ kind: 'value', z: 100 });
    expect(summarizeElevations([0, 0])).toEqual({ kind: 'value', z: 0 });
    expect(summarizeElevations([105.25, 98.5, 101])).toEqual({ kind: 'range', min: 98.5, max: 105.25 });
    expect(summarizeElevations([-2, 3])).toEqual({ kind: 'range', min: -2, max: 3 });
    expect(summarizeElevations([105.25, null, 98.5])).toEqual({ kind: 'partial', min: 98.5, max: 105.25 });
    // The ones that have an elevation agree: a partial value, not a range of one.
    expect(summarizeElevations([100, null, 100])).toEqual({ kind: 'partial', min: 100, max: 100 });
  });

  it('a path of hundreds of thousands of vertices (a spread into Math.min would overflow the call)', () => {
    const zs: (number | null)[] = Array.from({ length: 400_000 }, (_, i) => 1000 + (i % 250));
    expect(summarizeElevations(zs)).toEqual({ kind: 'range', min: 1000, max: 1249 });
    zs[7] = null;
    expect(summarizeElevations(zs)).toEqual({ kind: 'partial', min: 1000, max: 1249 });
  });
});

describe('the length in space', () => {
  it('30 m in plan rising 40 m is 50 m (a line)', () => {
    expect(spaceLength(line([0, 0], [30, 0], 0, 40))).toEqual({ label: '3B uzunluk', value: 50 });
    // Falling is the same length.
    expect(spaceLength(line([0, 0], [30, 0], 40, 0))?.value).toBe(50);
  });

  it('a polyline adds the rise of each edge: 50 m rising, then 10 m level', () => {
    const e = poly('polyline', [[0, 0], [30, 0], [40, 0]], [0, 40, 40]);
    expect(spaceLength(e)).toEqual({ label: '3B uzunluk', value: 60 });
  });

  it('an area counts its ring closed and every hole: 3B çevre', () => {
    // A 10 m square at 0, 10, 10, 0: two level edges of 10 m and two of 10√2 m; its 2 m hole is level, 8 m round.
    const ring = poly('polygon', SQUARE, [0, 10, 10, 0]);
    expect(spaceLength(ring)?.label).toBe('3B çevre');
    expect(spaceLength(ring)?.value).toBeCloseTo(20 + 20 * Math.SQRT2, 9);
    const holed = poly('polygon', SQUARE, [0, 10, 10, 0], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4), p(2, 4)], zs: [5, 5, 5, 5] }] });
    expect(spaceLength(holed)?.value).toBeCloseTo(28 + 20 * Math.SQRT2, 9);
  });

  it('an arc edge is measured along its curve: a half circle of 10 m chord rising 5π is 5π√2', () => {
    const e: Entity = { ...base, kind: 'polyline', pts: [p(0, 0), p(10, 0)], bulges: [1, 0], zs: [0, 5 * Math.PI] };
    expect(spaceLength(e)?.value).toBeCloseTo(5 * Math.PI * Math.SQRT2, 9);
  });

  it('none unless every vertex has an elevation, and none for what has no length', () => {
    expect(spaceLength(line([0, 0], [30, 0]))).toBeNull();
    expect(spaceLength(line([0, 0], [30, 0], 0))).toBeNull();
    expect(spaceLength(poly('polyline', [[0, 0], [30, 0], [40, 0]], [0, null, 40]))).toBeNull();
    // The outer ring has them all, a hole misses one.
    const holed = poly('polygon', SQUARE, [0, 1, 2, 3], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [5, null, 5] }] });
    expect(spaceLength(holed)).toBeNull();
    expect(spaceLength({ ...base, kind: 'circle', c: p(0, 0), r: 1 })).toBeNull();
    expect(spaceLength({ ...base, kind: 'point', p: p(0, 0), z: 3 })).toBeNull();
  });
});

describe('the elevation under a grip', () => {
  const holed = poly('polygon', SQUARE, [10, 11, null, 13], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [7, null, 9] }] });

  it('a line’s two ends', () => {
    const e = line([0, 0], [5, 0], 1, 2);
    expect([0, 1, 2].map((i) => gripElevation(e, i))).toEqual([1, 2, null]);
  });

  it('a path’s vertices, not the mid grips that follow them', () => {
    const e = poly('polyline', [[0, 0], [5, 0], [5, 5]], [1, 2, 3]);
    // Three vertices, then two mid grips.
    expect([0, 1, 2, 3, 4].map((i) => gripElevation(e, i))).toEqual([1, 2, 3, null, null]);
  });

  it('an area’s vertices, its mid grips (none) and its holes’ vertices', () => {
    // Four vertices, four mid grips, then the hole's three vertices.
    expect([0, 1, 2, 3].map((i) => gripElevation(holed, i))).toEqual([10, 11, null, 13]);
    expect([4, 5, 6, 7].map((i) => gripElevation(holed, i))).toEqual([null, null, null, null]);
    expect([8, 9, 10].map((i) => gripElevation(holed, i))).toEqual([7, null, 9]);
    expect(gripElevation(holed, 11)).toBeNull();
  });

  it('a point’s grip is the point; other kinds have none', () => {
    expect(gripElevation({ ...base, kind: 'point', p: p(0, 0), z: 4 }, 0)).toBe(4);
    expect(gripElevation({ ...base, kind: 'point', p: p(0, 0) }, 0)).toBeNull();
    expect(gripElevation({ ...base, kind: 'circle', c: p(0, 0), r: 1 }, 0)).toBeNull();
  });
});

describe('the vertex nearest a point', () => {
  it('a point, a line’s ends, a path’s vertices, an area’s ring and its holes: the place, the elevation and how far', () => {
    expect(nearestVertex({ ...base, kind: 'point', p: p(1, 2), z: 4 }, p(1, 3), 2)).toEqual({ at: p(1, 2), z: 4, d: 1 });
    expect(nearestVertex({ ...base, kind: 'point', p: p(1, 2) }, p(1, 2), 2)).toEqual({ at: p(1, 2), z: null, d: 0 });
    const l = line([0, 0], [5, 0], 1);
    expect(nearestVertex(l, p(0, 1), 3)).toEqual({ at: p(0, 0), z: 1, d: 1 });
    expect(nearestVertex(l, p(4, 0), 3)).toEqual({ at: p(5, 0), z: null, d: 1 });
    const path = poly('polyline', [[0, 0], [50, 0], [90, 40]], [1, null, 3]);
    expect(nearestVertex(path, p(89, 39), 3)?.z).toBe(3);
    expect(nearestVertex(path, p(50, 1), 3)).toMatchObject({ z: null, d: 1 });
    const holed = poly('polygon', SQUARE, [1, 2, 3, 4], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [7, 8, 9] }] });
    expect(nearestVertex(holed, p(4, 2.5), 1)).toMatchObject({ at: p(4, 2), z: 8 });
    expect(nearestVertex(holed, p(10, 9), 1.5)).toMatchObject({ at: p(10, 10), z: 3 });
  });

  it('the nearest of several within reach, and none beyond it (the reach is inclusive)', () => {
    const twin = poly('polyline', [[0, 0], [4, 0]], [null, 55]);
    expect(nearestVertex(twin, p(3, 0), 6)).toMatchObject({ z: 55, d: 1 });
    expect(nearestVertex(twin, p(1, 0), 6)).toMatchObject({ z: null, d: 1 });
    expect(nearestVertex(twin, p(0, 6), 6)).toMatchObject({ z: null, d: 6 });
    expect(nearestVertex(twin, p(0, 6.001), 6)).toBeNull();
    // A box corner is nearer than the reach in x and y, farther than it in distance.
    expect(nearestVertex(twin, p(5, 5), 5)).toBeNull();
  });

  it('nothing for a kind without vertices to grip', () => {
    expect(nearestVertex({ ...base, kind: 'circle', c: p(0, 0), r: 1 }, p(1, 0), 5)).toBeNull();
  });
});

describe('the elevation at a snapped point', () => {
  it('a vertex or a line’s end that has one; where there is no vertex, none', () => {
    const e = line([0, 0], [10, 0], 5, 15);
    expect(elevationAt(e, p(0, 0))).toBe(5);
    expect(elevationAt(e, p(10, 0))).toBe(15);
    // The middle is no vertex: the elevation there is not read off the edge.
    expect(elevationAt(e, p(5, 0))).toBeNull();
    expect(elevationAt(e, p(10.001, 0))).toBeNull();
    const path = poly('polyline', [[0, 0], [5, 0], [5, 5]], [1, null, 3]);
    expect(elevationAt(path, p(5, 5))).toBe(3);
    expect(elevationAt(path, p(5, 0))).toBeNull();
    const holed = poly('polygon', SQUARE, undefined, { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [7, 8, 9] }] });
    expect(elevationAt(holed, p(4, 2))).toBe(8);
    expect(elevationAt(holed, p(0, 0))).toBeNull();
    expect(elevationAt({ ...base, kind: 'point', p: p(3, 3), z: 12 }, p(3, 3))).toBe(12);
    expect(elevationAt({ ...base, kind: 'circle', c: p(0, 0), r: 1 }, p(1, 0))).toBeNull();
  });

  it('0 is an elevation', () => {
    expect(elevationAt(line([0, 0], [10, 0], 0, 1), p(0, 0))).toBe(0);
  });
});

describe('the geometry Kot ver writes', () => {
  it('a value for every vertex: zs for a line, a polyline and an area, the holes’ in each hole', () => {
    const holed = poly('polygon', SQUARE, [1, null, 3, 4], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)] }] });
    expect(mapElevations(holed, () => 100)).toEqual({
      kind: 'polygon',
      pts: SQUARE.map(([x, y]) => p(x, y)),
      zs: [100, 100, 100, 100],
      holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [100, 100, 100] }],
    });
    expect(mapElevations(line([0, 0], [5, 0]), () => -2.5)).toEqual({ kind: 'line', a: p(0, 0), b: p(5, 0), zs: [-2.5, -2.5] });
    expect(mapElevations(poly('polyline', SQUARE.slice(0, 3)), () => 0)).toMatchObject({ kind: 'polyline', zs: [0, 0, 0] });
  });

  it('a raise leaves a vertex without an elevation as it is, and does not round', () => {
    const e = poly('polyline', SQUARE.slice(0, 3), [1, null, 3]);
    expect(mapElevations(e, (z) => (z === null ? null : z + 2.5))).toMatchObject({ zs: [3.5, null, 5.5] });
    expect(mapElevations(line([0, 0], [5, 0], 100.1), (z) => (z === null ? null : z + 0.2))).toMatchObject({ zs: [100.1 + 0.2, null] });
  });

  it('a reset is all null: a vertex without one, not 0', () => {
    expect(mapElevations(line([0, 0], [5, 0], 1, 2), () => null)).toMatchObject({ zs: [null, null] });
    const holed = poly('polygon', SQUARE, [1, 2, 3, 4], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [5, 6, 7] }] });
    expect(mapElevations(holed, () => null)).toMatchObject({ zs: [null, null, null, null], holes: [{ zs: [null, null, null] }] });
  });

  it('the function is given each vertex’s place in `vertexElevations`’ order: a line end alone changes', () => {
    const e = line([0, 0], [5, 0], 1, 2);
    expect(mapElevations(e, (z, i) => (i === 1 ? 20 : z))).toMatchObject({ zs: [1, 20] });
    const holed = poly('polygon', SQUARE, [1, 2, 3, 4], { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)], zs: [5, 6, 7] }] });
    const seen: number[] = [];
    mapElevations(holed, (z, i) => (seen.push(i), z));
    expect(seen).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });

  it('a point takes z, or none when the value is null', () => {
    const pt: Entity = { ...base, kind: 'point', p: p(1, 2), z: 4 };
    expect(mapElevations(pt, () => 9)).toEqual({ kind: 'point', p: p(1, 2), z: 9 });
    expect(mapElevations(pt, (z) => (z === null ? null : z + 1))).toEqual({ kind: 'point', p: p(1, 2), z: 5 });
    expect(mapElevations(pt, () => null)).toEqual({ kind: 'point', p: p(1, 2) });
    // A point without one stays without under a raise.
    expect(mapElevations({ ...base, kind: 'point', p: p(1, 2) }, (z) => (z === null ? null : z + 1))).toEqual({ kind: 'point', p: p(1, 2) });
  });

  it('shares nothing with the object, and a kind without elevations gives none', () => {
    const e = poly('polygon', SQUARE, undefined, { holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4)] }] });
    const g = mapElevations(e, () => 1) as unknown as { pts: { x: number }[]; holes: { pts: { x: number }[] }[] };
    g.pts[0].x = 99;
    g.holes[0].pts[0].x = 99;
    expect((e as unknown as { pts: { x: number }[] }).pts[0].x).toBe(0);
    expect((e as unknown as { holes: { pts: { x: number }[] }[] }).holes[0].pts[0].x).toBe(2);
    expect(mapElevations({ ...base, kind: 'circle', c: p(0, 0), r: 1 }, () => 1)).toBeNull();
  });
});
