import { describe, expect, it } from 'vitest';
import { entityArea, entityBounds, entityLength, type ConstructionEntity, type EllipseEntity, type LineEntity } from '../entities';
import { mirror } from '../geom/affine';
import { ellipsePoint, ellipseSweep, isFullEllipse } from '../geom/ellipse';
import { breakEntity } from './break';
import { entityEdges } from './edges';
import { entityGrips, moveGrip } from './grips';
import { offsetEntity } from './offset';
import { transformEntity } from './transform';
import { extendEntity, trimEntity } from './trim';

const base = { id: 1, layerId: 'x', attrs: {} };
const v = (x: number, y: number) => ({ x, y });
const ell = (t0 = 0, t1 = 0): EllipseEntity => ({ ...base, kind: 'ellipse', c: v(0, 0), major: v(10, 0), ratio: 0.5, t0, t1 });
const line = (ax: number, ay: number, bx: number, by: number): LineEntity => ({ ...base, id: 9, kind: 'line', a: v(ax, ay), b: v(bx, by) });
const xline = (kind: 'xline' | 'ray' = 'xline'): ConstructionEntity => ({ ...base, kind, p: v(0, 0), dir: v(1, 0) });

describe('ellipse entity', () => {
  it('has exact area, a closed outline and bounds', () => {
    expect(entityArea(ell())).toBeCloseTo(Math.PI * 50, 12);
    const b = entityBounds(ell());
    expect(b.maxX).toBeCloseTo(10, 3);
    expect(b.maxY).toBeCloseTo(5, 3);
  });
  it('mirrors an elliptical arc and keeps it counter-clockwise', () => {
    const arc = ell(0, Math.PI / 2); // (10,0) → (0,5)
    const m = transformEntity(arc, mirror(v(0, 0), v(0, 1)));
    const s = ellipsePoint(m, m.t0);
    const f = ellipsePoint(m, m.t1);
    expect([s.x, s.y].map((x) => +x.toFixed(9))).toEqual([0, 5]);
    expect([f.x, f.y].map((x) => +x.toFixed(9))).toEqual([-10, 0]);
  });
  it('trims a whole ellipse between two crossings of a line exactly', () => {
    const cutter = line(-20, 0, 20, 0);
    const r = trimEntity(ell(), v(0, -5), entityEdges(cutter));
    if (!('pieces' in r)) throw new Error(r.error);
    const piece = r.pieces[0] as EllipseEntity;
    expect(piece.kind).toBe('ellipse');
    expect(ellipseSweep(piece)).toBeCloseTo(Math.PI, 12);
    expect(ellipsePoint(piece, piece.t0 + Math.PI / 2).y).toBeCloseTo(5, 12);
  });
  it('breaks an elliptical arc at a point into two arcs', () => {
    const r = breakEntity(ell(0, Math.PI), v(0, 5), v(0, 5));
    if (!('pieces' in r)) throw new Error(r.error);
    expect(r.pieces).toHaveLength(2);
    expect(r.pieces.every((p) => p.kind === 'ellipse' && !isFullEllipse(p))).toBe(true);
  });
  it('extends an elliptical arc to a line', () => {
    const r = extendEntity(ell(0, Math.PI / 4), v(6, 4), entityEdges(line(0, -1, 0, 20)));
    if (!('geometry' in r)) throw new Error(r.error);
    const g = r.geometry as EllipseEntity;
    const end = ellipsePoint(g, g.t1);
    expect(end.x).toBeCloseTo(0, 12);
    expect(end.y).toBeCloseTo(5, 12);
  });
  it('offsets outward along the true normals', () => {
    const r = offsetEntity(ell(), 1, v(20, 0));
    if (!('geometry' in r)) throw new Error(r.error);
    expect(r.geometry.kind).toBe('polygon');
    const len = entityLength({ ...base, ...r.geometry } as never)!;
    expect(len).toBeCloseTo(entityLength(ell())! + 2 * Math.PI, 2);
  });
  it('refuses an inward offset past the curvature limit', () => {
    expect('error' in offsetEntity(ell(), 3, v(0, 0))).toBe(true);
  });
  it('resizes axes from their grips', () => {
    const g = entityGrips(ell());
    expect(g).toHaveLength(5);
    const m = moveGrip(ell(), 2, v(0, 8))!;
    expect(Math.hypot(m.major.x, m.major.y) * m.ratio).toBeCloseTo(8, 12);
  });
});

describe('construction lines', () => {
  it('trim one side of an xline into a ray, both sides into a line', () => {
    const cutter = line(5, -1, 5, 1);
    const r = trimEntity(xline(), v(20, 0), entityEdges(cutter));
    if (!('pieces' in r)) throw new Error(r.error);
    expect(r.pieces).toEqual([{ kind: 'ray', p: v(5, 0), dir: v(-1, 0) }]);
    const ray = xline('ray');
    const r2 = trimEntity(ray, v(20, 0), entityEdges(cutter));
    if (!('pieces' in r2)) throw new Error(r2.error);
    expect(r2.pieces).toEqual([{ kind: 'line', a: v(0, 0), b: v(5, 0) }]);
  });
  it('breaks an xline at a point into two rays', () => {
    const r = breakEntity(xline(), v(3, 0), v(3, 0));
    if (!('pieces' in r)) throw new Error(r.error);
    expect(r.pieces.map((p) => p.kind)).toEqual(['ray', 'ray']);
  });
  it('offsets parallel on the picked side', () => {
    const r = offsetEntity(xline(), 2, v(0, -5));
    if (!('geometry' in r)) throw new Error(r.error);
    expect(r.geometry).toEqual({ kind: 'xline', p: v(0, -2), dir: v(1, 0) });
  });
  it('crosses finite geometry exactly in float64', () => {
    const [edge] = entityEdges({ ...xline(), p: v(487123.456, 4420000.789), dir: v(Math.SQRT1_2, Math.SQRT1_2) });
    expect(edge.kind).toBe('seg');
    const b = entityBounds(xline());
    expect(b.maxX - b.minX).toBe(0);
  });
});
