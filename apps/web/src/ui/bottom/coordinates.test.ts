import { describe, expect, it } from 'vitest';
import type { Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { vertexListing } from './coordinates';

const v = (x: number, y: number): Vec2 => ({ x, y });
const base = { id: 1, layerId: 'l', attrs: {} };
const square = (x0: number, y0: number, s: number) => [v(x0, y0), v(x0 + s, y0), v(x0 + s, y0 + s), v(x0, y0 + s)];

describe('the coordinate list of one object', () => {
  it("keeps each edge within its ring and gives a holed parcel's own area", () => {
    // A 10×10 parcel with a 2×2 building hole.
    const parcel: Entity = { ...base, kind: 'polygon', pts: square(0, 0, 10), holes: [{ pts: square(4, 4, 2).reverse() }] };
    const l = vertexListing(parcel);
    expect(l.pts).toHaveLength(8);
    // The outer ring closes on its first vertex; the hole on its own first vertex.
    expect(l.next(2)).toBe(3);
    expect(l.next(3)).toBe(0);
    expect(l.next(7)).toBe(4);
    // 100 − 4, not the area of the eight vertices strung together.
    expect(l.area).toBeCloseTo(96, 12);
    expect(l.length).toBeCloseTo(40 + 8, 12);
  });

  it('follows the arc of a bulged edge and leaves an open path open', () => {
    // A half disc: a diameter of 2 and a bulge of 1 (a half turn) back.
    const halfDisc: Entity = { ...base, kind: 'polygon', pts: [v(0, 0), v(2, 0)], bulges: [0, 1] };
    const l = vertexListing(halfDisc);
    expect(l.area).toBeCloseTo(Math.PI / 2, 9);
    const path: Entity = { ...base, kind: 'polyline', pts: [v(0, 0), v(3, 0), v(3, 4)] };
    const p = vertexListing(path);
    expect(p.next(1)).toBe(2);
    expect(p.next(2)).toBeNull();
    expect(p.area).toBeNull();
    expect(p.length).toBeCloseTo(7, 12);
  });
});
