import { describe, expect, it } from 'vitest';
import type { ArcEntity, Entity, LineEntity, PolylineEntity } from '../entities';
import { mirror, rotation } from '../geom/affine';
import { sweep } from '../geom/arc';
import { entityEdges } from './edges';
import { filletLines } from './fillet';
import { entityGrips, moveGrip } from './grips';
import { offsetEntity } from './offset';
import { transformEntity } from './transform';
import { extendEntity, trimEntity } from './trim';

const base = { id: 1, layerId: 'x', attrs: {} };
const line = (ax: number, ay: number, bx: number, by: number): LineEntity => ({ ...base, kind: 'line', a: { x: ax, y: ay }, b: { x: bx, y: by } });
const edgesOf = (...es: Entity[]) => es.flatMap(entityEdges);

describe('transformEntity', () => {
  it('rotates an arc and keeps its sweep', () => {
    const arc: ArcEntity = { ...base, kind: 'arc', c: { x: 0, y: 0 }, r: 2, a0: 0, a1: Math.PI / 2 };
    const r = transformEntity(arc, rotation(Math.PI / 2));
    expect(r.a0).toBeCloseTo(Math.PI / 2);
    expect(r.a1).toBeCloseTo(Math.PI);
  });
  it('mirrors an arc and keeps it counter-clockwise', () => {
    const arc: ArcEntity = { ...base, kind: 'arc', c: { x: 0, y: 0 }, r: 2, a0: 0, a1: Math.PI / 2 };
    const m = transformEntity(arc, mirror({ x: 0, y: 0 }, { x: 0, y: 1 }));
    // Quarter in +x+y mirrors into −x+y: from 90° to 180°.
    expect(m.a0).toBeCloseTo(Math.PI / 2);
    expect(m.a1).toBeCloseTo(Math.PI);
    expect(sweep(m.a0, m.a1)).toBeCloseTo(Math.PI / 2);
  });
  it('keeps mirrored text readable', () => {
    const t = transformEntity({ ...base, kind: 'text', p: { x: 1, y: 0 }, text: 'A', height: 1, rotation: 0 }, mirror({ x: 0, y: 0 }, { x: 0, y: 1 }));
    expect(t.p.x).toBeCloseTo(-1);
    expect(t.rotation).toBeCloseTo(0);
  });
});

describe('trimEntity', () => {
  it('removes the middle of a line between two cutters', () => {
    const target = line(0, 0, 10, 0);
    const r = trimEntity(target, { x: 5, y: 0.1 }, edgesOf(line(3, -1, 3, 1), line(7, -1, 7, 1)));
    expect('pieces' in r && r.pieces).toHaveLength(2);
    if (!('pieces' in r)) return;
    const [p1, p2] = r.pieces as LineEntity[];
    expect(p1.b.x).toBeCloseTo(3);
    expect(p2.a.x).toBeCloseTo(7);
  });
  it('removes an end overhang', () => {
    const r = trimEntity(line(0, 0, 10, 0), { x: 9, y: 0 }, edgesOf(line(7, -1, 7, 1)));
    expect('pieces' in r && r.pieces).toHaveLength(1);
    if ('pieces' in r) expect((r.pieces[0] as LineEntity).b.x).toBeCloseTo(7);
  });
  it('reports when nothing cuts the picked part', () => {
    expect('error' in trimEntity(line(0, 0, 10, 0), { x: 5, y: 0 }, [])).toBe(true);
  });
  it('opens a closed square into a polyline', () => {
    const sq: PolylineEntity = {
      ...base,
      kind: 'polygon',
      pts: [
        { x: 0, y: 0 },
        { x: 10, y: 0 },
        { x: 10, y: 10 },
        { x: 0, y: 10 },
      ],
    };
    // A vertical cutter through the bottom and top edges at x = 4; pick the bottom-right part.
    const r = trimEntity(sq, { x: 8, y: 0 }, edgesOf(line(4, -5, 4, 15)));
    expect('pieces' in r).toBe(true);
    if (!('pieces' in r)) return;
    const piece = r.pieces[0] as PolylineEntity;
    expect(piece.kind).toBe('polyline');
    // Kept side runs from the top cut around the left edge to the bottom cut.
    expect(piece.pts[0].x).toBeCloseTo(4);
    expect(piece.pts[0].y).toBeCloseTo(10);
    expect(piece.pts.at(-1)!.x).toBeCloseTo(4);
    expect(piece.pts.at(-1)!.y).toBeCloseTo(0);
  });
  it('turns a trimmed circle into an arc', () => {
    const circle: Entity = { ...base, kind: 'circle', c: { x: 0, y: 0 }, r: 5 };
    const r = trimEntity(circle, { x: 5, y: 0 }, edgesOf(line(3, -10, 3, 10)));
    expect('pieces' in r).toBe(true);
    if ('pieces' in r) expect(r.pieces[0].kind).toBe('arc');
  });
});

describe('extendEntity', () => {
  it('extends the nearer end of a line to the boundary', () => {
    const r = extendEntity(line(0, 0, 5, 0), { x: 4.5, y: 0 }, edgesOf(line(9, -1, 9, 1), line(20, -1, 20, 1)));
    expect('geometry' in r).toBe(true);
    if ('geometry' in r) expect((r.geometry as LineEntity).b.x).toBeCloseTo(9);
  });
  it('reports when no boundary lies ahead', () => {
    expect('error' in extendEntity(line(0, 0, 5, 0), { x: 5, y: 0 }, edgesOf(line(-9, -1, -9, 1)))).toBe(true);
  });
  it('refuses a polyline without an end segment instead of throwing', () => {
    const one: PolylineEntity = { ...base, kind: 'polyline', pts: [{ x: 0, y: 0 }] };
    expect(extendEntity(one, { x: 0, y: 0 }, edgesOf(line(-5, -5, 5, -5)))).toEqual({ error: 'Uzatmak için en az iki köşe gerekir.' });
  });
});

describe('offsetEntity', () => {
  it('offsets a line towards the picked side', () => {
    const r = offsetEntity(line(0, 0, 10, 0), 2, { x: 5, y: -3 });
    expect('geometry' in r).toBe(true);
    if ('geometry' in r) expect((r.geometry as LineEntity).a.y).toBeCloseTo(-2);
  });
  it('shrinks a circle when picked inside', () => {
    const r = offsetEntity({ ...base, kind: 'circle', c: { x: 0, y: 0 }, r: 5 }, 2, { x: 1, y: 0 });
    expect('geometry' in r && r.geometry.kind === 'circle' && r.geometry.r).toBeCloseTo(3);
  });
  it('refuses a zero-length line instead of returning one without points', () => {
    expect(offsetEntity(line(3, 4, 3, 4), 2, { x: 5, y: 5 })).toEqual({ error: 'Öteleme sonucu geçerli bir şekil oluşmadı.' });
  });
});

describe('filletLines', () => {
  const l1 = { a: { x: 0, y: 0 }, b: { x: 10, y: 0 } };
  const l2 = { a: { x: 12, y: 2 }, b: { x: 12, y: 10 } };
  it('makes a sharp corner with radius 0', () => {
    const r = filletLines(l1, { x: 2, y: 0 }, l2, { x: 12, y: 8 }, 0);
    expect('line1' in r).toBe(true);
    if (!('line1' in r)) return;
    expect(r.line1.b).toEqual({ x: 12, y: 0 });
    expect(r.line2.b.y).toBeCloseTo(0);
    expect(r.arc).toBeNull();
  });
  it('places tangent points r away from the corner for a right angle', () => {
    const r = filletLines(l1, { x: 2, y: 0 }, l2, { x: 12, y: 8 }, 2);
    if (!('line1' in r)) throw new Error(r.error);
    expect(r.line1.b.x).toBeCloseTo(10);
    expect(r.line2.b.y).toBeCloseTo(2);
    expect(r.arc!.c.x).toBeCloseTo(10);
    expect(r.arc!.c.y).toBeCloseTo(2);
    expect(sweep(r.arc!.a0, r.arc!.a1)).toBeCloseTo(Math.PI / 2);
  });
  it('rejects a radius larger than the lines allow', () => {
    expect('error' in filletLines(l1, { x: 2, y: 0 }, l2, { x: 12, y: 8 }, 50)).toBe(true);
  });
});

describe('grips', () => {
  it('moves a line endpoint', () => {
    const m = moveGrip(line(0, 0, 10, 0), 1, { x: 10, y: 5 })!;
    expect(m.b).toEqual({ x: 10, y: 5 });
    expect(m.a).toEqual({ x: 0, y: 0 });
  });
  it('resizes a circle from a quadrant grip', () => {
    const c: Entity = { ...base, kind: 'circle', c: { x: 0, y: 0 }, r: 5 };
    expect(entityGrips(c)).toHaveLength(5);
    const m = moveGrip(c, 1, { x: 8, y: 0 });
    expect(m && m.kind === 'circle' && m.r).toBeCloseTo(8);
  });
  it('reshapes an arc through its mid grip and keeps the ends', () => {
    const arc: ArcEntity = { ...base, kind: 'arc', c: { x: 0, y: 0 }, r: 5, a0: 0, a1: Math.PI };
    const m = moveGrip(arc, 1, { x: 0, y: 2 })!;
    const [s, , e] = entityGrips(m);
    expect(s.x).toBeCloseTo(5);
    expect(e.x).toBeCloseTo(-5);
    expect(m.r).toBeGreaterThan(5);
  });
  it('rejects a degenerate arc', () => {
    const arc: ArcEntity = { ...base, kind: 'arc', c: { x: 0, y: 0 }, r: 5, a0: 0, a1: Math.PI };
    expect(moveGrip(arc, 1, { x: 0, y: 0 })).toBeNull();
  });
});

describe('transformEntity (annotation kinds)', () => {
  it('mirrors a dimension and keeps its line on the mirrored side', () => {
    const d: Entity = { ...base, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 10, y: 0 }, offset: 3, height: 1 };
    const m = transformEntity(d, mirror({ x: 0, y: 0 }, { x: 1, y: 0 })); // across the x axis
    if (m.kind !== 'dimension') throw new Error();
    // The dimension line was at y = +3; mirrored it must be at y = −3.
    const ny = m.offset * ((m.b.x - m.a.x) / Math.abs(m.b.x - m.a.x));
    expect(ny).toBeCloseTo(-3);
  });
  it('rotates a hatch pattern with its boundary', () => {
    const h: Entity = {
      ...base,
      kind: 'hatch',
      ring: [
        { x: 0, y: 0 },
        { x: 1, y: 0 },
        { x: 1, y: 1 },
      ],
      pattern: { type: 'lines', angle: 45, spacing: 2 },
    };
    const r = transformEntity(h, rotation(Math.PI / 2));
    expect(r.kind === 'hatch' && r.pattern.angle).toBeCloseTo(135);
  });
});
