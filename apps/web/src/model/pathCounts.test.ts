import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity, RingGeometry } from './entities';
import { hasArcs, hasIslands, pathCounts } from './pathCounts';

/**
 * What the panels and the export windows count in a path or an area (docs/adr/0143): a multi-part area's parts are
 * counted whole, its own ring as the first part and every part past it, holes and arcs included.
 */
const p = (x: number, y: number) => ({ x, y });
const square = (x: number, y: number, size: number): RingGeometry => ({ pts: [p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)] });
const base = { id: 1, layerId: 'parsel', attrs: {} };
const area = (fields: Partial<PolylineEntity>): PolylineEntity => ({ ...base, kind: 'polygon', ...square(0, 0, 10), ...fields });

describe('pathCounts', () => {
  it('a one-part area: its own corners, one part, its own holes', () => {
    expect(pathCounts(area({}))).toEqual({ corners: 4, parts: 1, holes: 0 });
    expect(pathCounts(area({ holes: [square(2, 2, 2), square(6, 6, 2)] }))).toEqual({ corners: 4, parts: 1, holes: 2 });
  });

  it('a multi-part area: every part’s corners and holes, the parts counting the area’s own', () => {
    const e = area({
      holes: [square(2, 2, 2)],
      parts: [
        { ...square(20, 0, 10), holes: [square(22, 2, 2), square(26, 6, 2)] },
        { pts: [p(40, 0), p(50, 0), p(45, 8)] },
      ],
    });
    // 4 + 4 + 3 outer vertices (the holes' are not counted), three parts, 1 + 2 + 0 holes.
    expect(pathCounts(e)).toEqual({ corners: 11, parts: 3, holes: 3 });
  });

  it('an empty part list is a one-part area', () => {
    expect(pathCounts(area({ parts: [] }))).toEqual({ corners: 4, parts: 1, holes: 0 });
  });

  it('a polyline: its vertices and one part', () => {
    expect(pathCounts({ pts: [p(0, 0), p(1, 0), p(2, 1)] })).toEqual({ corners: 3, parts: 1, holes: 0 });
  });
});

describe('hasIslands', () => {
  it('is true when the area’s own ring or any part has a hole', () => {
    expect(hasIslands(area({}))).toBe(false);
    expect(hasIslands(area({ holes: [square(2, 2, 2)] }))).toBe(true);
    expect(hasIslands(area({ parts: [square(20, 0, 10)] }))).toBe(false);
    // Only the second part has an island: the area still counts.
    expect(hasIslands(area({ parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2)] }] }))).toBe(true);
  });

  it('is false for anything that is not an area', () => {
    expect(hasIslands({ ...base, kind: 'polyline', ...square(0, 0, 10), holes: [square(2, 2, 2)] } as Entity)).toBe(false);
    expect(hasIslands({ ...base, kind: 'circle', c: p(0, 0), r: 1 } as Entity)).toBe(false);
  });
});

describe('hasArcs', () => {
  const arc = [0, 0.5, 0, 0];
  it('is true for an arc in the ring, in a hole, in a part or in a part’s hole', () => {
    expect(hasArcs(area({}))).toBe(false);
    expect(hasArcs(area({ bulges: arc }))).toBe(true);
    expect(hasArcs(area({ holes: [{ ...square(2, 2, 2), bulges: arc }] }))).toBe(true);
    expect(hasArcs(area({ parts: [{ ...square(20, 0, 10), bulges: arc }] }))).toBe(true);
    expect(hasArcs(area({ parts: [{ ...square(20, 0, 10), holes: [{ ...square(22, 2, 2), bulges: arc }] }] }))).toBe(true);
  });

  it('is false when every bulge is zero, and for a polyline of straight edges', () => {
    expect(hasArcs(area({ bulges: [0, 0, 0, 0], parts: [{ ...square(20, 0, 10), bulges: [0, 0, 0, 0] }] }))).toBe(false);
    expect(hasArcs({ ...base, kind: 'polyline', ...square(0, 0, 10) } as Entity)).toBe(false);
  });

  it('is true for a polyline with an arc and false for kinds that have no bulges', () => {
    expect(hasArcs({ ...base, kind: 'polyline', ...square(0, 0, 10), bulges: arc } as Entity)).toBe(true);
    expect(hasArcs({ ...base, kind: 'circle', c: p(0, 0), r: 1 } as Entity)).toBe(false);
  });
});
