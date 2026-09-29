import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { entityGrips } from '../model/ops/grips';
import { carryInto, elevatedPaths, withoutElevations } from './elevation';
import { elevationAt, gripElevation, hasVertexElevation, mapElevations, nearestVertex, spaceLength, uniformElevation, vertexElevations } from './elevationValues';

/**
 * The vertex elevations of a multi-part area (docs/adr/0143): every part's ring and its holes, part after part
 * after the area's own ring and holes, in the order the desktop's `paths` goes by. The Kot row, 3B çevre and the
 * tag of a grip all count every part.
 */
const p = (x: number, y: number) => ({ x, y });
const square = (x: number, y: number, side: number) => [p(x, y), p(x + side, y), p(x + side, y + side), p(x, y + side)];
const base = { id: 1, layerId: 'cizim', attrs: {} };

/** A 10 m square with a 2 m hole, and one 20 m east with a 2 m hole, every ring with elevations. */
const two = (extra: Partial<PolylineEntity> = {}): PolylineEntity => ({
  ...base,
  kind: 'polygon',
  pts: square(0, 0, 10),
  zs: [1, 2, 3, 4],
  holes: [{ pts: square(2, 2, 2), zs: [10, 11, 12, 13] }],
  parts: [{ pts: square(20, 0, 10), zs: [5, 6, 7, 8], holes: [{ pts: square(24, 4, 2), zs: [20, 21, 22, 23] }] }],
  ...extra,
});

describe('elevations of a multi-part area', () => {
  it('lists every vertex: the area’s ring and holes, then each part’s ring and holes', () => {
    expect(vertexElevations(two())).toEqual([1, 2, 3, 4, 10, 11, 12, 13, 5, 6, 7, 8, 20, 21, 22, 23]);
    // A ring without elevations has its vertices null, in their place.
    expect(vertexElevations(two({ parts: [{ pts: square(20, 0, 10), holes: [{ pts: square(24, 4, 2), zs: [1, 2, 3, 4] }] }] }))).toEqual([1, 2, 3, 4, 10, 11, 12, 13, null, null, null, null, 1, 2, 3, 4]);
    expect(elevatedPaths(two()).map((path) => [path.pts.length, path.closed, path.zs[0]])).toEqual([
      [4, true, 1],
      [4, true, 10],
      [4, true, 5],
      [4, true, 20],
    ]);
  });

  it('has an elevation when any part, or any hole of a part, has one', () => {
    const bare = (parts: PolylineEntity['parts']) => ({ ...base, kind: 'polygon' as const, pts: square(0, 0, 10), parts });
    expect(hasVertexElevation(bare([{ pts: square(20, 0, 10) }]))).toBe(false);
    expect(hasVertexElevation(bare([{ pts: square(20, 0, 10), zs: [null, null, null, null] }]))).toBe(false);
    expect(hasVertexElevation(bare([{ pts: square(20, 0, 10), zs: [null, 0, null, null] }]))).toBe(true);
    expect(hasVertexElevation(bare([{ pts: square(20, 0, 10) }, { pts: square(40, 0, 10), holes: [{ pts: square(42, 2, 2), zs: [null, null, 3, null] }] }]))).toBe(true);
  });

  it('finds the nearest vertex among every part’s and its holes’', () => {
    expect(nearestVertex(two(), p(30.2, 0), 1)).toMatchObject({ at: p(30, 0), z: 6 });
    expect(nearestVertex(two(), p(26, 6.3), 1)).toMatchObject({ at: p(26, 6), z: 22 });
    expect(nearestVertex(two(), p(2, 2), 1)).toMatchObject({ at: p(2, 2), z: 10 });
    expect(nearestVertex(two(), p(15, 5), 1)).toBeNull();
  });

  it('is uniform only when every vertex of every part is', () => {
    const level = (z: number) => two({ zs: [z, z, z, z], holes: [{ pts: square(2, 2, 2), zs: [z, z, z, z] }], parts: [{ pts: square(20, 0, 10), zs: [z, z, z, z], holes: [{ pts: square(24, 4, 2), zs: [z, z, z, z] }] }] });
    expect(uniformElevation(level(7))).toBe(7);
    const tilted = level(7);
    tilted.parts![0].holes![0].zs = [7, 7, 7, 8];
    expect(uniformElevation(tilted)).toBe('mixed');
    const lower = level(7);
    lower.parts![0].zs = [6, 6, 6, 6];
    expect(uniformElevation(lower)).toBe('mixed');
    const without = level(7);
    delete without.parts![0].zs;
    expect(uniformElevation(without)).toBe('mixed');
    expect(uniformElevation(two({ zs: undefined, holes: undefined, parts: [{ pts: square(20, 0, 10) }] }))).toBeNull();
  });

  it('has a length in space that adds every part’s rings: 40 + 8 flat and 72 with a rise of 24 in the second part', () => {
    const flat = (z: number) => [z, z, z, z];
    const e = two({ zs: flat(0), holes: [{ pts: square(2, 2, 2), zs: flat(0) }], parts: [{ pts: square(20, 0, 10), zs: [0, 0, 0, 24] }] });
    expect(spaceLength(e)).toEqual({ label: '3B çevre', value: 40 + 8 + (10 + 10 + Math.hypot(10, 24) * 2) });
    // Not while a vertex of the last part has none.
    const partial = two({ zs: flat(0), holes: [{ pts: square(2, 2, 2), zs: flat(0) }], parts: [{ pts: square(20, 0, 10), zs: [0, 0, 0, null] }] });
    expect(spaceLength(partial)).toBeNull();
  });

  it('says the elevation of a grip on any part: a vertex, a hole’s vertex, nothing for a mid grip', () => {
    const e = two();
    const grips = entityGrips(e);
    // Grips: the first part's 4 vertices, 4 mids, 4 hole vertices; then the second part's the same.
    expect(gripElevation(e, 3)).toBe(4);
    expect(gripElevation(e, 4)).toBeNull();
    expect(gripElevation(e, 8)).toBe(10);
    expect(gripElevation(e, 12)).toBe(5);
    expect(gripElevation(e, 15)).toBe(8);
    expect(gripElevation(e, 16)).toBeNull();
    expect(gripElevation(e, 20)).toBe(20);
    expect(gripElevation(e, 23)).toBe(23);
    expect(gripElevation(e, 24)).toBeNull();
    expect(grips[12]).toEqual(p(20, 0));
    // The tag follows the grip that is moved: the elevation is where the grip stands.
    expect(gripElevation(e, 13)).toBe(6);
    expect(grips[13]).toEqual(p(30, 0));
  });

  it('reads the elevation of a snapped vertex of any part', () => {
    expect(elevationAt(two(), p(30, 10))).toBe(7);
    expect(elevationAt(two(), p(24, 6))).toBe(23);
    expect(elevationAt(two(), p(25, 5))).toBeNull();
  });
});

describe('the geometry Kot ver writes for a multi-part area', () => {
  it('gives every vertex of every part what `f` gives, in the order of the list', () => {
    const seen: [number | null, number][] = [];
    const g = mapElevations(two(), (z, i) => (seen.push([z, i]), (z ?? 0) + 100 + i)) as unknown as PolylineEntity;
    expect(seen.map(([, i]) => i)).toEqual(Array.from({ length: 16 }, (_, i) => i));
    expect(seen.map(([z]) => z)).toEqual(vertexElevations(two()));
    expect(g.zs).toEqual([101, 103, 105, 107]);
    expect(g.holes![0].zs).toEqual([114, 116, 118, 120]);
    expect(g.parts![0].zs).toEqual([113, 115, 117, 119]);
    expect(g.parts![0].holes![0].zs).toEqual([132, 134, 136, 138]);
    // The rings are the object's own.
    expect(g.parts![0].pts).toEqual(square(20, 0, 10));
  });

  it('gives a part without elevations null for each of its vertices', () => {
    const e = two({ parts: [{ pts: square(20, 0, 10) }] });
    const seen: (number | null)[] = [];
    const g = mapElevations(e, (z) => (seen.push(z), 9)) as unknown as PolylineEntity;
    expect(seen.slice(8)).toEqual([null, null, null, null]);
    expect(g.parts![0].zs).toEqual([9, 9, 9, 9]);
  });

  it('a one-part area is written as it was: no parts', () => {
    const g = mapElevations(two({ parts: undefined }), () => 1) as unknown as PolylineEntity;
    expect('parts' in g).toBe(false);
  });
});

describe('elevations carried to what an edit writes', () => {
  it('follow each part by place: the rings after the outer one in the order of the list', () => {
    const before = two();
    // The second part's ring moved a vertex and its hole; the vertex takes the one in its place (a moving edit).
    const init = { ...before, parts: [{ pts: [p(19, -1), ...square(20, 0, 10).slice(1)], holes: [{ pts: square(24, 4, 2) }] }] } as unknown as Parameters<typeof carryInto>[0];
    const same = elevatedPaths(before);
    expect(carryInto(init, elevatedPaths(before), same, false)).toBe(true);
    const out = init as unknown as PolylineEntity;
    expect(out.zs).toEqual([1, 2, 3, 4]);
    expect(out.holes![0].zs).toEqual([10, 11, 12, 13]);
    expect(out.parts![0].zs).toEqual([5, 6, 7, 8]);
    expect(out.parts![0].holes![0].zs).toEqual([20, 21, 22, 23]);
    // A part lays its fields out as the typed columns do: ring, arcs, elevations, holes.
    expect(Object.keys(out.parts![0])).toEqual(['pts', 'zs', 'holes']);
  });

  it('take away the elevations a geometry carries, the parts’ and the holes’ too', () => {
    const g = withoutElevations(two());
    expect(g.zs).toBeUndefined();
    expect('zs' in g).toBe(false);
    expect(g.holes).toEqual([{ pts: square(2, 2, 2) }]);
    expect(g.parts).toEqual([{ pts: square(20, 0, 10), holes: [{ pts: square(24, 4, 2) }] }]);
    // A hatch’s holes are lists of points: nothing to take out, and they stay lists.
    const hatch = { kind: 'hatch', ring: square(0, 0, 4), holes: [square(1, 1, 1)], pattern: { type: 'lines', angle: 0, spacing: 1 } };
    expect(withoutElevations(hatch)).toEqual(hatch);
    expect(Array.isArray(withoutElevations(hatch).holes[0])).toBe(true);
    // The object given is left as it was.
    const e: Entity = two();
    withoutElevations(e);
    expect((e as PolylineEntity).parts![0].zs).toEqual([5, 6, 7, 8]);
  });
});
