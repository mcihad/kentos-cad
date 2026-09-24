import { describe, expect, it } from 'vitest';
import { entityArea, entityLength, entityOutline, type ArcEntity, type CircleEntity, type EllipseEntity, type Entity, type LineEntity, type PolylineEntity } from '../entities';
import { closestParam, ellipsePoint } from '../geom/ellipse';
import type { Bounds } from '../geometry';
import { sweep } from '../geom/arc';
import { breakEntity } from './break';
import { entityEdges } from './edges';
import { explodeEntity } from './explode';
import { lengthenEntity, lengthOf, lengthToward, nearEnd } from './lengthen';
import { chamferLines, cornerOfPath } from './fillet';
import { entityGrips, moveGrip } from './grips';
import { joinEntities } from './join';
import { offsetEntity } from './offset';
import { divisionParams, divisionPoints, pathOf, pointAtS } from './path';
import { stretchEntity } from './stretch';
import { transformEntity } from './transform';
import { extendEntity, trimEntity } from './trim';
import { insertVertex, nearestSegment, removeVertex } from './vertex';
import { mirror } from '../geom/affine';

let nextId = 1;
const base = () => ({ id: nextId++, layerId: 'x', attrs: {} });
const v = (x: number, y: number) => ({ x, y });
const line = (ax: number, ay: number, bx: number, by: number): LineEntity => ({ ...base(), kind: 'line', a: v(ax, ay), b: v(bx, by) });
const poly = (pts: [number, number][], opts: { closed?: boolean; bulges?: number[] } = {}): PolylineEntity => ({
  ...base(),
  kind: opts.closed ? 'polygon' : 'polyline',
  pts: pts.map(([x, y]) => v(x, y)),
  ...(opts.bulges && { bulges: opts.bulges }),
});
const withId = <G extends object>(g: G) => ({ ...base(), ...g }) as unknown as Entity;
const rect = (minX: number, minY: number, maxX: number, maxY: number): Bounds => ({ minX, minY, maxX, maxY });

// A 10 × 10 square whose top edge bulges up into a half circle (bulge 1 = 180°).
const dome = () => poly([[0, 0], [10, 0], [10, 10], [0, 10]], { closed: true, bulges: [0, 0, 1, 0] });

describe('polyline arc segments', () => {
  it('measures length and area exactly', () => {
    const d = dome();
    expect(entityLength(d)).toBeCloseTo(30 + Math.PI * 5, 9);
    expect(entityArea(d)).toBeCloseTo(100 + (Math.PI * 25) / 2, 9);
  });
  it('produces signed arc edges that keep the travel direction', () => {
    const [, , top] = entityEdges(dome());
    expect(top.kind).toBe('arc');
    if (top.kind !== 'arc') return;
    expect(top.r).toBeCloseTo(5);
    expect(top.sweep).toBeCloseTo(Math.PI);
    // Travels from (10,10) over the top to (0,10).
    expect(top.c.x).toBeCloseTo(5);
    expect(top.c.y).toBeCloseTo(10);
  });
  it('tessellates the outline through the arc apex', () => {
    const ys = entityOutline(dome()).map((p) => p.y);
    expect(Math.max(...ys)).toBeCloseTo(15, 2);
  });
  it('flips bulges on reflection so the shape is mirrored, not inverted', () => {
    const m = transformEntity(dome(), mirror(v(0, 0), v(0, 1)));
    expect(m.bulges).toEqual([-0, -0, -1, -0]);
    expect(entityArea(m)).toBeCloseTo(100 + (Math.PI * 25) / 2, 9);
  });
});

describe('trim and extend with arc segments', () => {
  it('keeps the arc of a polyline piece exact', () => {
    // Open path: straight 0→10, then a half circle up to (10,10).
    const p = poly([[0, 0], [10, 0], [10, 10]], { bulges: [0, 1, 0] });
    const cutter = line(5, -1, 5, 1);
    const r = trimEntity(p, v(2, 0), entityEdges(cutter));
    expect('pieces' in r).toBe(true);
    if (!('pieces' in r)) return;
    const piece = withId(r.pieces[0]);
    expect(piece.kind).toBe('polyline');
    expect(entityLength(piece)).toBeCloseTo(5 + Math.PI * 5, 9);
  });
  it('extends a polyline that ends in an arc along its circle', () => {
    // Quarter circle from (10,0) to (0,10) about the origin (bulge tan(π/8)).
    const p = poly([[20, 0], [10, 0], [0, 10]], { bulges: [0, Math.tan(Math.PI / 8), 0] });
    const wall = line(-20, 0, 0, 0);
    const r = extendEntity(p, v(0, 10), entityEdges(wall));
    expect('geometry' in r).toBe(true);
    if (!('geometry' in r) || r.geometry.kind !== 'polyline') return;
    const end = r.geometry.pts.at(-1)!;
    expect(end.x).toBeCloseTo(-10, 9);
    expect(end.y).toBeCloseTo(0, 9);
    expect(r.geometry.bulges![1]).toBeCloseTo(Math.tan(Math.PI / 4), 9);
  });
});

describe('offset with arc segments', () => {
  it('offsets a bulged ring outwards: straight sides shift, the dome grows', () => {
    const r = offsetEntity(dome(), 1, v(5, -5));
    expect('geometry' in r).toBe(true);
    if (!('geometry' in r)) return;
    const g = withId(r.geometry);
    // Square grows to 12 × 11, the half circle to radius 6.
    expect(entityArea(g)).toBeCloseTo(12 * 11 + (Math.PI * 36) / 2, 6);
  });
  it('refuses to shrink an arc below zero radius', () => {
    const r = offsetEntity(dome(), 6, v(5, 5));
    expect('error' in r).toBe(true);
  });
});

describe('joinEntities', () => {
  it('joins lines and an arc into one polyline with a bulge', () => {
    const a = line(0, 0, 10, 0);
    const arc: ArcEntity = { ...base(), kind: 'arc', c: v(10, 5), r: 5, a0: -Math.PI / 2, a1: Math.PI / 2 };
    const b = line(0, 10, 10, 10);
    const { groups } = joinEntities([a, arc, b], 1e-6);
    expect(groups).toHaveLength(1);
    const g = withId(groups[0].geometry);
    expect(g.kind).toBe('polyline');
    expect(entityLength(g)).toBeCloseTo(20 + Math.PI * 5, 9);
    expect(groups[0].sources).toHaveLength(3);
  });
  it('closes a ring into a polygon and reverses pieces drawn backwards', () => {
    const { groups } = joinEntities([line(0, 0, 10, 0), line(10, 10, 10, 0), line(10, 10, 0, 10), line(0, 0, 0, 10)], 1e-6);
    expect(groups).toHaveLength(1);
    const g = withId(groups[0].geometry);
    expect(g.kind).toBe('polygon');
    expect(entityArea(g)).toBeCloseTo(100, 9);
  });
  it('bridges gaps within the tolerance only', () => {
    expect(joinEntities([line(0, 0, 10, 0), line(10.0005, 0, 20, 0)], 0.001).groups).toHaveLength(1);
    const far = joinEntities([line(0, 0, 10, 0), line(10.01, 0, 20, 0)], 0.001);
    expect(far.groups).toHaveLength(0);
    expect(far.skipped).toHaveLength(2);
  });
});

describe('explodeEntity', () => {
  it('splits a bulged polygon into lines and one counter-clockwise arc', () => {
    const r = explodeEntity(dome(), String);
    expect('pieces' in r).toBe(true);
    if (!('pieces' in r)) return;
    expect(r.pieces.map((p) => p.kind)).toEqual(['line', 'line', 'arc', 'line']);
    const arc = r.pieces[2];
    if (arc.kind === 'arc') expect(sweep(arc.a0, arc.a1)).toBeCloseTo(Math.PI);
  });
  it('turns a clockwise arc segment into a CCW arc with swapped ends', () => {
    const r = explodeEntity(poly([[0, 0], [10, 0]], { bulges: [-1, 0] }), String);
    if (!('pieces' in r)) throw new Error('expected pieces');
    const arc = r.pieces[0];
    expect(arc.kind).toBe('arc');
    if (arc.kind !== 'arc') return;
    // Clockwise from (0,0) to (10,0) bulges upward (left of travel is below).
    expect(Math.sin((arc.a0 + arc.a1) / 2 + (arc.a1 < arc.a0 ? Math.PI : 0))).toBeGreaterThan(0.99);
  });
  it('explodes a dimension into lines and its value text', () => {
    const d = withId({ kind: 'dimension', a: v(0, 0), b: v(10, 0), offset: 2, height: 1 });
    const r = explodeEntity(d, (l) => l.value.toFixed(2));
    if (!('pieces' in r)) throw new Error('expected pieces');
    const text = r.pieces.find((p) => p.kind === 'text');
    expect(text && text.kind === 'text' && text.text).toBe('10.00');
  });
  it('refuses simple entities', () => {
    expect('error' in explodeEntity(line(0, 0, 1, 1), String)).toBe(true);
  });
});

describe('breakEntity', () => {
  it('removes the part between two points of a line', () => {
    const r = breakEntity(line(0, 0, 10, 0), v(3, 0), v(6, 0));
    if (!('pieces' in r)) throw new Error('expected pieces');
    expect(r.pieces.map((p) => entityLength(withId(p)))).toEqual([3, 4]);
  });
  it('splits at a single point without removing anything', () => {
    const r = breakEntity(poly([[0, 0], [10, 0], [10, 10]]), v(10, 4), v(10, 4));
    if (!('pieces' in r)) throw new Error('expected pieces');
    expect(r.pieces).toHaveLength(2);
    expect(entityLength(withId(r.pieces[0]))! + entityLength(withId(r.pieces[1]))!).toBeCloseTo(20);
  });
  it('opens a polygon at one point into a polyline of full length', () => {
    const r = breakEntity(poly([[0, 0], [10, 0], [10, 10], [0, 10]], { closed: true }), v(5, 0), v(5, 0));
    if (!('pieces' in r)) throw new Error('expected pieces');
    const g = withId(r.pieces[0]);
    expect(g.kind).toBe('polyline');
    expect(entityLength(g)).toBeCloseTo(40);
  });
  it('removes the counter-clockwise part of a circle', () => {
    const c: CircleEntity = { ...base(), kind: 'circle', c: v(0, 0), r: 1 };
    const r = breakEntity(c, v(1, 0), v(0, 1));
    if (!('pieces' in r)) throw new Error('expected pieces');
    const arc = r.pieces[0];
    expect(arc.kind === 'arc' && sweep(arc.a0, arc.a1)).toBeCloseTo((3 * Math.PI) / 2);
  });
  it('removes the counter-clockwise part of a clockwise polygon too', () => {
    const cw = poly([[0, 0], [0, 10], [10, 10], [10, 0]], { closed: true });
    // CCW from the bottom edge middle (5,0) to the right edge middle (10,5) is the short way.
    const r = breakEntity(cw, v(5, 0), v(10, 5));
    if (!('pieces' in r)) throw new Error('expected pieces');
    expect(entityLength(withId(r.pieces[0]))).toBeCloseTo(30);
  });
});

describe('stretchEntity', () => {
  it('moves only the vertices inside the window', () => {
    const p = poly([[0, 0], [10, 0], [10, 10], [0, 10]], { closed: true });
    const g = stretchEntity(p, rect(8, -1, 12, 11), 5, 0);
    expect(g && g.kind === 'polygon' && g.pts.map((q) => q.x)).toEqual([0, 15, 15, 0]);
  });
  it('returns null when the window misses the entity', () => {
    expect(stretchEntity(line(0, 0, 1, 0), rect(5, 5, 6, 6), 1, 1)).toBeNull();
  });
});

describe('vertices', () => {
  it('adds a vertex on a straight segment and removes it again', () => {
    const p = poly([[0, 0], [10, 0], [10, 10]]);
    const r = insertVertex(p, 0, v(4, 0.3));
    if (!('geometry' in r) || r.geometry.kind !== 'polyline') throw new Error('expected polyline');
    expect(r.geometry.pts[1]).toEqual(v(4, 0));
    const back = removeVertex(withId(r.geometry), 1);
    if (!('geometry' in back) || back.geometry.kind !== 'polyline') throw new Error('expected polyline');
    expect(back.geometry.pts).toHaveLength(3);
  });
  it('splits an arc segment into two arcs on the same circle', () => {
    const r = insertVertex(dome(), 2, v(5, 16));
    if (!('geometry' in r)) throw new Error('expected geometry');
    const g = withId(r.geometry);
    expect(entityArea(g)).toBeCloseTo(entityArea(dome())!, 9);
    expect(g.kind === 'polygon' && g.pts[3].y).toBeCloseTo(15);
  });
  it('turns a line into a polyline when a vertex is added', () => {
    const r = insertVertex(line(0, 0, 10, 0), 0, v(5, 0));
    expect('geometry' in r && r.geometry.kind).toBe('polyline');
  });
  it('keeps a polygon at three vertices or more', () => {
    expect('error' in removeVertex(poly([[0, 0], [1, 0], [0, 1]], { closed: true }), 0)).toBe(true);
  });
  it('bends an arc segment through a dragged mid grip', () => {
    const d = dome();
    const midIndex = d.pts.length + 2;
    expect(entityGrips(d)[midIndex].y).toBeCloseTo(15);
    const moved = moveGrip(d, midIndex, v(5, 12));
    expect(moved && moved.kind === 'polygon' && moved.bulges![2]).toBeCloseTo(0.4, 9);
  });
});

describe('corners', () => {
  it('chamfers two lines at the given distances', () => {
    const r = chamferLines({ a: v(0, 0), b: v(10, 0) }, v(5, 0), { a: v(10, 0), b: v(10, 10) }, v(10, 5), 2, 3);
    if ('error' in r) throw new Error(r.error);
    expect(r.cut!.a).toEqual(v(8, 0));
    expect(r.cut!.b).toEqual(v(10, 3));
  });
  it('rounds a polygon corner with a tangent arc segment', () => {
    const sq = poly([[0, 0], [10, 0], [10, 10], [0, 10]], { closed: true });
    const r = cornerOfPath(sq.pts, undefined, true, 1, { radius: 2 });
    if ('error' in r) throw new Error(r.error);
    const g = withId({ kind: 'polygon', ...r });
    // Area loses the corner square minus the quarter disc.
    expect(entityArea(g)).toBeCloseTo(100 - (4 - Math.PI), 9);
    expect(r.pts).toHaveLength(5);
  });
  it('cuts a polyline corner for a chamfer', () => {
    const r = cornerOfPath([v(0, 0), v(10, 0), v(10, 10)], undefined, false, 1, { d1: 2, d2: 2 });
    if ('error' in r) throw new Error(r.error);
    expect(r.pts).toEqual([v(0, 0), v(8, 0), v(10, 2), v(10, 10)]);
  });
  it('refuses the open ends of a polyline', () => {
    expect('error' in cornerOfPath([v(0, 0), v(10, 0), v(10, 10)], undefined, false, 0, { radius: 1 })).toBe(true);
  });
});

describe('divisionParams', () => {
  it('divides an open path into equal parts', () => {
    const path = pathOf(line(0, 0, 9, 0))!;
    expect(divisionParams(path, { parts: 3 }).map((s) => pointAtS(path, s).x)).toEqual([3, 6]);
  });
  it('places n points around a closed path', () => {
    const c: CircleEntity = { ...base(), kind: 'circle', c: v(0, 0), r: 1 };
    expect(divisionParams(pathOf(c)!, { parts: 4 })).toHaveLength(4);
  });
  it('divides a full ellipse like a circle, an elliptical arc like an arc', () => {
    const full: Entity = { ...base(), kind: 'ellipse', c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 };
    expect(pathOf(full)!.closed).toBe(true);
    expect(divisionPoints(full, { parts: 4 }, false)).toHaveLength(4);
    expect(divisionPoints({ ...full, t1: Math.PI } as Entity, { parts: 4 }, false)).toHaveLength(3);
  });
  it('measures off a step without a point on the far end', () => {
    expect(divisionParams(pathOf(line(0, 0, 10, 0))!, { step: 2.5 })).toEqual([2.5, 5, 7.5]);
  });
  it('gives the points in one call: from either end, on the true curve of an ellipse', () => {
    const pl: PolylineEntity = { ...base(), kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 7)], bulges: [0.4, 0, 0] };
    const path = pathOf(pl)!;
    const one = (fromEnd: boolean) => divisionParams(path, { step: 3 }).map((s) => pointAtS(path, fromEnd ? path.length - s : s));
    expect(divisionPoints(pl, { step: 3 }, false)).toEqual(one(false));
    expect(divisionPoints(pl, { step: 3 }, true)).toEqual(one(true));
    const el: Entity = { ...base(), kind: 'ellipse', c: v(486512.34, 4420187.52), major: v(20, 5), ratio: 0.4, t0: 0.3, t1: 2.5 };
    const epath = pathOf(el)!;
    const want = divisionParams(epath, { parts: 7 }).map((s) => pointAtS(epath, s));
    const got = divisionPoints(el, { parts: 7 }, false);
    expect(got).toHaveLength(6);
    for (const [i, p] of got.entries()) {
      // On the curve (not on the chords it was measured along), next to the chord point.
      expect(Math.hypot(p.x - want[i].x, p.y - want[i].y)).toBeLessThan(1e-3);
      expect(p).toEqual(ellipsePoint(el as EllipseEntity, closestParam(el as EllipseEntity, want[i])));
    }
    expect(divisionPoints({ ...base(), kind: 'point', p: v(0, 0) }, { parts: 3 }, false)).toEqual([]);
  });
});

describe('uzat-kısalt', () => {
  const line = withId({ kind: 'line', a: v(0, 0), b: v(10, 0) });
  it('lengthens and shortens a line at either end', () => {
    const g = lengthenEntity(line, true, 15);
    expect('geometry' in g && g.geometry.kind === 'line' && g.geometry.b).toEqual(v(15, 0));
    const s = lengthenEntity(line, false, 4);
    expect('geometry' in s && s.geometry.kind === 'line' && s.geometry.a.x).toBeCloseTo(6, 12);
    expect('error' in lengthenEntity(line, true, 0)).toBe(true);
  });
  it('an arc grows on its circle, never past a full turn', () => {
    const arc = withId({ kind: 'arc', c: v(0, 0), r: 2, a0: 0, a1: Math.PI / 2 });
    const g = lengthenEntity(arc, true, Math.PI * 2);
    expect('geometry' in g && g.geometry.kind === 'arc' && g.geometry.a1).toBeCloseTo(Math.PI, 12);
    const s = lengthenEntity(arc, false, Math.PI / 2);
    expect('geometry' in s && s.geometry.kind === 'arc' && s.geometry.a0).toBeCloseTo(Math.PI / 4, 12);
    expect('error' in lengthenEntity(arc, true, 5 * Math.PI)).toBe(true);
  });
  it('a polyline shortens across vertices and lengthens its end segment', () => {
    const pl = withId({ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] });
    const s = lengthenEntity(pl, true, 5);
    expect('geometry' in s && s.geometry.kind === 'polyline' && s.geometry.pts).toEqual([v(0, 0), v(5, 0)]);
    const g = lengthenEntity(pl, true, 25);
    expect('geometry' in g && g.geometry.kind === 'polyline' && g.geometry.pts[2]).toEqual(v(10, 15));
    const f = lengthenEntity(pl, false, 22);
    expect('geometry' in f && f.geometry.kind === 'polyline' && f.geometry.pts[0].x).toBeCloseTo(-2, 12);
  });
  it('a polyline ending in an arc keeps the arc on its circle', () => {
    // Quarter circle from (10,0) to (20,10) about (10,10), counter-clockwise.
    const pl = withId({ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(20, 10)], bulges: [0, Math.tan(Math.PI / 8), 0] });
    const L = lengthOf(pl)!;
    const g = lengthenEntity(pl, true, L + 5 * Math.PI);
    if (!('geometry' in g) || g.geometry.kind !== 'polyline') throw new Error('expected polyline');
    // Now a half circle ending at (10,20).
    expect(g.geometry.pts[2].x).toBeCloseTo(10, 9);
    expect(g.geometry.pts[2].y).toBeCloseTo(20, 9);
    expect(lengthOf(withId(g.geometry))).toBeCloseTo(L + 5 * Math.PI, 9);
  });
  it('the dragged length follows the pointer beyond the end and inside the path', () => {
    expect(lengthToward(line, true, v(13, 2))).toBeCloseTo(13, 12);
    expect(lengthToward(line, true, v(7, -1))).toBeCloseTo(7, 12);
    expect(lengthToward(line, false, v(-3, 1))).toBeCloseTo(13, 12);
    expect(nearEnd(line, v(9, 0))).toBe(true);
    const arc = withId({ kind: 'arc', c: v(0, 0), r: 1, a0: 0, a1: Math.PI / 2 });
    expect(lengthToward(arc, true, v(-1, 0.001))).toBeCloseTo(Math.PI, 2);
  });
});

describe('nearestSegment', () => {
  it('gives a near-tie to the first edge, so the last bits of a distance do not decide', () => {
    // The third edge is 2.5·10⁻¹⁰ m nearer than the first: the same distance for drawing.
    const e = withId({ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 2), v(0, 2 - 5e-10)] });
    expect(nearestSegment(e, v(5, 1))).toBe(0);
    expect(nearestSegment(e, v(5, 1.5))).toBe(2);
  });
});
