import { describe, expect, it } from 'vitest';
import { CadDocument } from '../document';
import { LayerStore } from '../layers';
import { entityArea, entityLength, entityVertices, type Entity, type EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import { mirror } from '../geom/affine';
import { hatchLines } from '../geom/hatch';
import { netArea, subtractAreas, unionAreas } from '../geom/region';
import { areaOfEntity, lineSource, polygonOfArea, polylinesOfPolygon } from './areas';
import { entityEdges } from './edges';
import { explodeEntity } from './explode';
import { entityGrips, holeGrip, moveGrip } from './grips';
import { stretchEntity } from './stretch';
import { transformEntity } from './transform';

const v = (x: number, y: number): Vec2 => ({ x, y });
const base = { id: 1, layerId: 'l', attrs: {} };
const square = (x0: number, y0: number, s: number) => [v(x0, y0), v(x0 + s, y0), v(x0 + s, y0 + s), v(x0, y0 + s)];
/** 10×10 parcel with a 2×2 building hole. */
const holed = (): Entity => ({ ...base, kind: 'polygon', pts: square(0, 0, 10), holes: [{ pts: square(4, 4, 2).reverse() }] });

describe('entities as areas', () => {
  it('circle, closed polyline and polygon convert exactly', () => {
    expect(netArea(areaOfEntity({ kind: 'circle', c: v(3, 4), r: 2 })!)).toBeCloseTo(4 * Math.PI, 12);
    const closed: EntityGeometry = { kind: 'polyline', pts: [...square(0, 0, 3), v(0, 0)] };
    expect(netArea(areaOfEntity(closed)!)).toBeCloseTo(9, 12);
    expect(areaOfEntity({ kind: 'polyline', pts: square(0, 0, 3) })).toBeNull();
    expect(areaOfEntity({ kind: 'line', a: v(0, 0), b: v(1, 1) })).toBeNull();
  });
  it('a full ellipse stays within a millimetre in area terms', () => {
    const a = areaOfEntity({ kind: 'ellipse', c: v(0, 0), major: v(100, 0), ratio: 0.5, t0: 0, t1: 0 })!;
    expect(Math.abs(netArea(a) - Math.PI * 100 * 50)).toBeLessThan(0.3);
  });
  it('an area back to a polygon, and a polygon to closed polylines', () => {
    const [d] = subtractAreas([areaOfEntity({ kind: 'polygon', pts: square(0, 0, 10) })!], [areaOfEntity({ kind: 'circle', c: v(5, 5), r: 1 })!]);
    const g = polygonOfArea(d);
    expect(g.kind === 'polygon' && g.holes?.length).toBe(1);
    expect(entityArea({ ...base, ...g } as Entity)).toBeCloseTo(100 - Math.PI, 10);
    const lines = polylinesOfPolygon(g as Extract<EntityGeometry, { kind: 'polygon' }>);
    expect(lines).toHaveLength(2);
    for (const l of lines) {
      expect(l.kind).toBe('polyline');
      if (l.kind === 'polyline') expect(l.pts[0]).toEqual(l.pts[l.pts.length - 1]);
    }
    // Round trip: the closed polylines are areas again.
    expect(netArea(areaOfEntity(lines[0])!)).toBeCloseTo(100, 12);
    expect(Math.abs(netArea(areaOfEntity(lines[1])!))).toBeCloseTo(Math.PI, 10);
  });
  it('line work as a source keeps the exact end points', () => {
    const s = lineSource([{ ...base, kind: 'line', a: v(0, 0), b: v(1, 0) }]);
    expect(s.cut).toBe(true);
    expect(s.points).toEqual([v(0, 0), v(1, 0)]);
  });
});

describe('polygon with holes', () => {
  it('area, perimeter, vertices and edges include the hole', () => {
    const e = holed();
    expect(entityArea(e)).toBeCloseTo(96, 12);
    expect(entityLength(e)).toBeCloseTo(48, 12);
    expect(entityVertices(e)).toHaveLength(8);
    expect(entityEdges(e)).toHaveLength(8);
  });
  it('mirroring moves the hole and keeps the net area', () => {
    const e = transformEntity(holed(), mirror(v(20, 0), v(20, 1)));
    expect(e.kind === 'polygon' && e.holes![0].pts[0].x).toBe(36);
    expect(entityArea(e)).toBeCloseTo(96, 12);
  });
  it('hole vertices are grips after the outer vertices and mid grips', () => {
    const e = holed();
    const grips = entityGrips(e);
    expect(grips).toHaveLength(4 + 4 + 4);
    expect(holeGrip(e, 8)).toEqual({ hole: 0, vertex: 0 });
    expect(holeGrip(e, 7)).toBeNull();
    const moved = moveGrip(e, 8, v(3, 3))!;
    expect(moved.kind === 'polygon' && moved.holes![0].pts[0]).toEqual(v(3, 3));
  });
  it('stretch moves hole corners inside the window; explode gives the hole edges too', () => {
    const g = stretchEntity(holed(), { minX: 3, minY: 3, maxX: 7, maxY: 7 }, 1, 0)!;
    expect(g.kind === 'polygon' && g.holes![0].pts.every((p) => p.x >= 5)).toBe(true);
    const r = explodeEntity(holed(), String);
    expect('pieces' in r && r.pieces).toHaveLength(8);
  });
  it('the document drops holes when an edit turns a polygon into a polyline', () => {
    const doc = new CadDocument({ name: 't.kcad', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: v(0, 0) });
    const e = doc.add({ kind: 'polygon', pts: square(0, 0, 10), holes: [{ pts: square(4, 4, 2) }], layerId: 'a', attrs: {} });
    doc.update(e.id, { kind: 'polyline' } as Partial<Entity>);
    expect('holes' in doc.get(e.id)!).toBe(false);
    doc.undo();
    expect((doc.get(e.id) as { holes?: unknown[] }).holes).toHaveLength(1);
  });
  it('union of two holed parcels keeps both holes', () => {
    const a = areaOfEntity(holed())!;
    const b = areaOfEntity({ kind: 'polygon', pts: square(10, 0, 10), holes: [{ pts: square(14, 4, 2) }] })!;
    const [u] = unionAreas([a, b]);
    expect(u.holes).toHaveLength(2);
    expect(netArea(u)).toBeCloseTo(192, 10);
  });
});

describe('hatch with islands', () => {
  const ring = square(0.5, 0.5, 10);
  const hole = square(4.5, 4.5, 2);
  it('hatch lines skip the hole', () => {
    // Ten lines of 10 m; the two through the hole lose 2 m each.
    const { segments } = hatchLines(ring, 0, 1, [hole]);
    const total = segments.reduce((s, [a, b]) => s + Math.hypot(b.x - a.x, b.y - a.y), 0);
    expect(total).toBeCloseTo(96, 9);
  });
  it('area and transform follow the hole', () => {
    const h: Entity = { ...base, kind: 'hatch', ring, holes: [hole], pattern: { type: 'lines', angle: 45, spacing: 1 } };
    expect(entityArea(h)).toBeCloseTo(96, 12);
    const m = transformEntity(h, mirror(v(20, 0), v(20, 1)));
    expect(m.kind === 'hatch' && m.holes![0][0].x).toBeCloseTo(35.5, 12);
    expect(entityEdges(h)).toHaveLength(8);
  });
});
