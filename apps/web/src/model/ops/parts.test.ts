import { describe, expect, it } from 'vitest';
import { entityArea, entityBounds, entityLength, type PolylineEntity } from '../entities';
import type { Vec2 } from '../geometry';
import { translation } from '../geom/affine';
import { netArea } from '../geom/region';
import { areaOfEntity, areasOfEntity } from './areas';
import { breakEntity } from './break';
import { entityGrips, gripPart, holeGrip, midGripSegment, moveGrip } from './grips';
import { joinParts, replacePart, splitParts } from './parts';
import { geometryIsFinite, transformEntity, transformObjects, withGeometry } from './transform';
import { insertVertex, nearHole, removeVertex } from './vertex';

/**
 * A multi-part area through the core's ops and the helpers around them (docs/adr/0143): the area's own fields are its
 * first part, `parts` the others. Grips, edges and vertices run part after part; the elevations (`zs`) stay with
 * the object (the core has none), each part's with its part.
 */
const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number, side: number) => [v(x, y), v(x + side, y), v(x + side, y + side), v(x, y + side)];

/** A 10 m square with a 2 m hole and a 10 m square 20 m east with a 2 m hole, elevations on every ring. */
const two = (): PolylineEntity => ({
  id: 1,
  layerId: 'x',
  attrs: { Ada: '5' },
  kind: 'polygon',
  pts: square(0, 0, 10),
  zs: [1, 2, 3, 4],
  holes: [{ pts: square(2, 2, 2), zs: [10, 11, 12, 13] }],
  parts: [{ pts: square(20, 0, 10), zs: [5, 6, 7, 8], holes: [{ pts: square(24, 4, 2), zs: [20, 21, 22, 23] }] }],
});
const one = (): PolylineEntity => ({ id: 2, layerId: 'x', attrs: {}, kind: 'polygon', pts: square(0, 0, 10) });
const parts = (e: PolylineEntity) => e.parts;

describe('measures and grips of a multi-part area', () => {
  it('measures as the sum of its parts: 96 + 96 m², 48 + 48 m of edge', () => {
    expect(entityArea(two())).toBeCloseTo(192, 9);
    expect(entityLength(two())).toBeCloseTo(96, 9);
    expect(entityBounds(two())).toEqual({ minX: 0, minY: 0, maxX: 30, maxY: 10 });
  });

  it('has grips part after part, and says which part a grip is on and where', () => {
    const e = two();
    // Each part: 4 vertices, 4 mid grips, the hole's 4 vertices.
    expect(entityGrips(e)).toHaveLength(24);
    expect(gripPart(e, 0)).toEqual({ part: 0, index: 0 });
    expect(gripPart(e, 11)).toEqual({ part: 0, index: 11 });
    expect(gripPart(e, 12)).toEqual({ part: 1, index: 0 });
    expect(gripPart(e, 23)).toEqual({ part: 1, index: 11 });
    expect(gripPart(e, 24)).toBeNull();
    expect(entityGrips(e)[12]).toEqual(v(20, 0));
    // Mid grips and hole grips count in their own part.
    expect(midGripSegment(e, 4)).toBe(0);
    expect(midGripSegment(e, 16)).toBe(0);
    expect(midGripSegment(e, 18)).toBe(2);
    expect(midGripSegment(e, 20)).toBeNull();
    expect(holeGrip(e, 8)).toEqual({ hole: 0, vertex: 0 });
    expect(holeGrip(e, 20)).toEqual({ hole: 0, vertex: 0 });
    expect(holeGrip(e, 12)).toBeNull();
    // Any other object is all part 0.
    expect(gripPart(one(), 3)).toEqual({ part: 0, index: 3 });
  });

  it('moves the grip in its own part and leaves the others as they are', () => {
    const e = two();
    const moved = moveGrip(e, 12, v(19, -1))!;
    expect(parts(moved)![0].pts[0]).toEqual(v(19, -1));
    expect(parts(moved)![0].holes![0].pts).toEqual(square(24, 4, 2));
    expect(moved.pts).toEqual(e.pts);
    expect(moved.holes).toEqual([{ pts: square(2, 2, 2) }]);
  });

  it('near a hole means near a hole of any part', () => {
    expect(nearHole(two(), v(25, 4.2))).toBe(true);
    expect(nearHole(two(), v(20, 5))).toBe(false);
  });
});

describe('the parts as areas, split and joined', () => {
  it('gives one area for each part; areaOfEntity gives none for a multi-part area', () => {
    expect(areaOfEntity(two())).toBeNull();
    expect(areasOfEntity(two()).map(netArea)).toEqual([96, 96]);
    expect(areasOfEntity(one()).map(netArea)).toEqual([100]);
    expect(areasOfEntity({ kind: 'polyline', pts: square(0, 0, 3) })).toEqual([]);
  });

  it('splits into areas of their own with the object’s other fields, and joins back', () => {
    const list = splitParts(two());
    expect(list).toHaveLength(2);
    for (const p of list) {
      expect(p).toMatchObject({ kind: 'polygon', layerId: 'x', attrs: { Ada: '5' } });
      expect(parts(p)).toBeUndefined();
    }
    expect(list[1].pts).toEqual(square(20, 0, 10));
    expect(list[1].holes).toEqual([{ pts: square(24, 4, 2) }]);
    const back = joinParts(list)!;
    expect(back.pts).toEqual(two().pts);
    expect(parts(back)).toEqual([{ pts: square(20, 0, 10), holes: [{ pts: square(24, 4, 2) }] }]);
    // A one-part area is its own only part.
    expect(splitParts(one())).toHaveLength(1);
    expect(joinParts([one()])).toMatchObject({ pts: square(0, 0, 10) });
    expect(joinParts([])).toBeNull();
  });

  it('joins areas that have parts already, all of their parts in order', () => {
    const joined = joinParts([two(), { ...one(), pts: square(50, 0, 5) }])!;
    expect(parts(joined)!.map((p) => p.pts[0])).toEqual([v(20, 0), v(50, 0)]);
  });

  it('puts an edited part back among the others', () => {
    const list = splitParts(two());
    const edited = replacePart(two(), 1, { ...list[1], pts: square(20, 0, 12), holes: undefined })!;
    expect(edited.pts).toEqual(two().pts);
    expect(parts(edited)![0].pts).toEqual(square(20, 0, 12));
    expect(replacePart(two(), 2, list[1])).toBeNull();
  });
});

describe('vertices and edges of a multi-part area', () => {
  it('counts edges and vertices part after part: a vertex is added on, or removed from, its own part', () => {
    // Edges: the ring's 4 and the hole's 4, then the second part's 4 (8 to 11) and its hole's.
    const added = insertVertex(two(), 8, v(25, 0));
    if (!('geometry' in added)) throw new Error(added.error);
    const area = added.geometry as PolylineEntity;
    expect(parts(area)![0].pts).toEqual([v(20, 0), v(25, 0), v(30, 0), v(30, 10), v(20, 10)]);
    // The whole area comes back: the first part and its hole, the second part's hole.
    expect(area.pts).toEqual(square(0, 0, 10));
    expect(area.holes).toEqual([{ pts: square(2, 2, 2) }]);
    expect(parts(area)![0].holes).toEqual([{ pts: square(24, 4, 2) }]);
    // Vertices: 0 to 3 are the first part's, 4 to 7 the second's.
    const removed = removeVertex(two(), 5);
    if (!('geometry' in removed)) throw new Error(removed.error);
    const rest = removed.geometry as PolylineEntity;
    expect(parts(rest)![0].pts).toEqual([v(20, 0), v(30, 10), v(20, 10)]);
    expect(rest.pts).toEqual(square(0, 0, 10));
  });

  it('refuses to break an area of several parts, and says why', () => {
    const r = breakEntity(two(), v(20, 0), v(30, 10));
    expect(r).toEqual({ error: 'Bu işlem çok parçalı alanda çalışmaz; önce Parçalara ayır ile alanı parçalarına ayırın.' });
  });
});

describe('an area given a geometry back (withGeometry) and moved', () => {
  it('keeps each part’s elevations, and its holes’, where the vertex counts are the same', () => {
    const g = { kind: 'polygon', pts: square(1, 1, 10), holes: [{ pts: square(3, 3, 2) }], parts: [{ pts: square(21, 1, 10), holes: [{ pts: square(25, 5, 2) }] }] };
    const out = withGeometry(two(), g);
    expect(out.zs).toEqual([1, 2, 3, 4]);
    expect(out.holes).toEqual([{ pts: square(3, 3, 2), zs: [10, 11, 12, 13] }]);
    expect(parts(out)).toEqual([{ pts: square(21, 1, 10), zs: [5, 6, 7, 8], holes: [{ pts: square(25, 5, 2), zs: [20, 21, 22, 23] }] }]);
  });

  it('leaves out those of a part or a hole whose vertex count changed', () => {
    const g = { kind: 'polygon', pts: square(0, 0, 10), holes: [{ pts: square(2, 2, 2) }], parts: [{ pts: [...square(20, 0, 10), v(25, -3)], holes: [{ pts: square(24, 4, 2).slice(0, 3) }] }] };
    const out = withGeometry(two(), g);
    expect(parts(out)![0].zs).toBeUndefined();
    expect(parts(out)![0].holes![0].zs).toBeUndefined();
    expect(out.holes![0].zs).toEqual([10, 11, 12, 13]);
  });

  it('clears the parts when the geometry has none, as it clears the holes', () => {
    const out = withGeometry(two(), { kind: 'polygon', pts: square(0, 0, 10) }) as unknown as Record<string, unknown>;
    expect(out.parts).toBeUndefined();
    expect('parts' in out).toBe(true);
    expect(out.holes).toBeUndefined();
    expect('holes' in out).toBe(true);
    // An area that had no parts gets the same undefined (the JSON call's answer has it too).
    expect('parts' in withGeometry(one(), { kind: 'polygon', pts: square(0, 0, 3) })).toBe(true);
  });

  it('is every part moved by a transform, the elevations with them, and a one-part area comes back without parts', () => {
    const [moved] = transformObjects([two()], { kind: 'move', dx: 100, dy: 50 });
    expect(moved.pts).toEqual(square(100, 50, 10));
    expect(parts(moved)).toEqual([{ pts: square(120, 50, 10), zs: [5, 6, 7, 8], holes: [{ pts: square(124, 54, 2), zs: [20, 21, 22, 23] }] }]);
    expect((moved as { zs?: unknown }).zs).toEqual([1, 2, 3, 4]);
    const [still] = transformObjects([one()], { kind: 'move', dx: 1, dy: 1 });
    expect(parts(still)).toBeUndefined();
  });

  it('is the JSON call’s answer, field for field, without the elevations the core has none of', () => {
    const json = transformEntity(two(), translation(7, -3));
    const [packed] = transformObjects([two()], { kind: 'move', dx: 7, dy: -3 });
    expect(parts(packed)!.map((p) => p.pts)).toEqual(parts(json)!.map((p) => p.pts));
    expect(packed.pts).toEqual(json.pts);
    // The JSON call answers an area with one part with `parts` left undefined too.
    expect('parts' in transformEntity(one(), translation(1, 1))).toBe(true);
  });

  it('asks that every number of every part be finite', () => {
    expect(geometryIsFinite(two())).toBe(true);
    const bad = two();
    parts(bad)![0].pts[2] = v(30, NaN);
    expect(geometryIsFinite(bad)).toBe(false);
    const badHole = two();
    parts(badHole)![0].holes![0].pts[0] = v(Infinity, 4);
    expect(geometryIsFinite(badHole)).toBe(false);
  });
});
