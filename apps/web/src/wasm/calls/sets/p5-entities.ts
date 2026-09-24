import {
  type Entity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { mirror, rotation, scaling, translation, type Affine } from '../../../model/geom/affine';
import { TAU } from '../../../model/geom/arc';
import { entityGrips } from '../../../model/ops/grips';
import { repeat, type CallSet, type Gen } from '../harness';

/** P5: the entity model's geometry helpers, edges, transforms, stretch and grips (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'xline', 'ray', 'spline', 'text', 'dimension', 'hatch'] as const;

function angle(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, Math.PI / 2, Math.PI, 1.5 * Math.PI, TAU]) : g.num(-TAU, TAU);
}

/** A random entity of any kind, with the fields the core must hand back untouched. */
export function entity(g: Gen, kind = g.pick(KINDS)): Entity {
  const attrs: Record<string, string> = g.chance(0.5) ? { Ada: String(g.int(1, 999)), Parsel: String(g.int(1, 99)) } : {};
  const base = { id: g.int(1, 9999), layerId: g.pick(['parsel', 'bina', 'taslak']), attrs, ...(g.chance(0.3) ? { label: 'P' + g.int(1, 99) } : {}), ...(g.chance(0.2) ? { color: '#aa3322' } : {}), ...(g.chance(0.2) ? { symbol: 'mpyy:konut' } : {}) };
  const ring = (n: number) => g.ring(n, g.num(3, 30), g.chance(0.3));
  const bulges = (n: number) => (g.chance(0.5) ? undefined : Array.from({ length: n }, () => (g.chance(0.5) ? 0 : g.num(-1, 1))));
  switch (kind) {
    case 'point':
      return { ...base, kind, p: g.pt(), ...(g.chance(0.3) ? { z: g.num(0, 1500) } : {}) };
    case 'line':
      return { ...base, kind, a: g.pt(), b: g.chance(0.1) ? v(0, 0) : g.pt() };
    case 'polyline': {
      const pts = g.pts(g.int(0, 7), 40);
      const b = bulges(pts.length);
      return { ...base, kind, pts, ...(b ? { bulges: b } : {}) };
    }
    case 'polygon': {
      const pts = ring(g.int(3, 8));
      const b = bulges(pts.length);
      const c = pts[0];
      const holes = g.chance(0.4) ? [{ pts: [v(c.x - 0.5, c.y - 0.3), v(c.x + 0.5, c.y - 0.3), v(c.x, c.y + 0.4)] }] : undefined;
      return { ...base, kind, pts, ...(b ? { bulges: b } : {}), ...(holes ? { holes } : {}) };
    }
    case 'circle':
      return { ...base, kind, c: g.pt(), r: g.pick([0, 5, g.num(0.1, 50)]) };
    case 'arc':
      return { ...base, kind, c: g.pt(), r: g.num(0.1, 50), a0: angle(g), a1: angle(g) };
    case 'ellipse': {
      const t0 = angle(g);
      return { ...base, kind, c: g.pt(), major: g.vec(30), ratio: g.num(0.05, 1), t0, t1: g.chance(0.4) ? t0 : angle(g) };
    }
    case 'xline':
    case 'ray': {
      const t = g.num(0, TAU);
      return { ...base, kind, p: g.pt(), dir: v(Math.cos(t), Math.sin(t)) };
    }
    case 'spline':
      return { ...base, kind, pts: g.pts(g.int(0, 6), 30), closed: g.chance(0.4) };
    case 'text':
      return { ...base, kind, p: g.pt(), text: g.pick(['', 'Ada 104', 'Çiçek sokağı', '😀 not']), height: g.num(0.2, 5), rotation: g.num(-360, 360) };
    case 'dimension': {
      const style = g.pick([undefined, 'aligned', 'linear', 'angular', 'radius', 'diameter'] as const);
      return { ...base, kind, a: g.pt(), b: g.pt(), offset: g.num(-20, 20), height: g.num(0.3, 3), ...(g.chance(0.2) ? { text: '12,5 m' } : {}), ...(style ? { style } : {}), ...(style === 'linear' ? { angle: g.pick([0, 90]) } : {}), ...(style === 'angular' || g.chance(0.1) ? { c: g.pt() } : {}) };
    }
    case 'hatch': {
      const r = ring(g.int(3, 7));
      const c = r[0];
      return { ...base, kind, ring: r, ...(g.chance(0.4) ? { holes: [[v(c.x - 0.5, c.y), v(c.x + 0.5, c.y), v(c.x, c.y + 0.5)]] } : {}), pattern: { type: g.pick(['solid', 'lines', 'cross'] as const), angle: g.num(0, 180), spacing: g.num(0.1, 3) } };
    }
  }
}

function affine(g: Gen): Affine {
  return g.pick([() => translation(g.num(-100, 100), g.num(-100, 100)), () => rotation(angle(g), g.pt()), () => scaling(g.num(0.2, 3), g.pt()), () => mirror(g.pt(), g.pt())])();
}

export const P5: CallSet = {
  file: 'calls-p5-entities.json',
  named: [
    { name: 'boş çoklu çizginin çapası', fn: 'entityAnchor', args: [{ id: 1, layerId: 'a', attrs: {}, kind: 'polyline', pts: [] }] },
    { name: 'aynalanan yazı okunur kalır', fn: 'transformEntity', args: [{ id: 2, layerId: 'a', attrs: { Ad: 'x' }, kind: 'text', p: v(5, 5), text: 'Ada 1', height: 2, rotation: 30 }, mirror(v(0, 0), v(0, 1))] },
    { name: 'aynalanan yay saat yönünün tersine kalır', fn: 'transformEntity', args: [{ id: 3, layerId: 'a', attrs: {}, kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }, mirror(v(0, 0), v(1, 1))] },
    { name: 'aynalanan açı ölçüsü kolları değişir', fn: 'transformEntity', args: [{ id: 4, layerId: 'a', attrs: {}, kind: 'dimension', a: v(10, 0), b: v(0, 10), offset: 5, height: 1, style: 'angular', c: v(0, 0) }, mirror(v(0, 0), v(1, 0))] },
    { name: 'TM parseli döndürme', fn: 'transformEntity', args: [{ id: 5, layerId: 'parsel', attrs: { Ada: '104', Parsel: '7' }, label: '7', kind: 'polygon', pts: [v(486512.34, 4420187.52), v(486535.757, 4420188.723), v(486538.221, 4420218.986), v(486514.344, 4420220.532)] }, rotation(0.3, v(486520, 4420200))] },
    { name: 'adalı alanın kenar ortası tutamacı', fn: 'moveGrip', args: [{ id: 6, layerId: 'a', attrs: {}, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], holes: [{ pts: [v(3, 3), v(7, 3), v(5, 7)] }] }, 5, v(12, 5)] },
    { name: 'delik köşesi tutamacı', fn: 'moveGrip', args: [{ id: 7, layerId: 'a', attrs: {}, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], holes: [{ pts: [v(3, 3), v(7, 3), v(5, 7)] }] }, 9, v(4, 4)] },
    { name: 'yay kenarını bükmek', fn: 'moveGrip', args: [{ id: 8, layerId: 'a', attrs: {}, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(20, 0)], bulges: [0.5, 0, 0] }, 3, v(5, 4)] },
    { name: 'düz kenara köşe eklemek', fn: 'moveGrip', args: [{ id: 9, layerId: 'a', attrs: {}, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(20, 0)], bulges: [0.5, 0] }, 4, v(15, 2)] },
    { name: 'tam elips ekseni', fn: 'moveGrip', args: [{ id: 10, layerId: 'a', attrs: {}, kind: 'ellipse', c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, 2, v(0, 8)] },
    { name: 'yayın bir ucunu esnetmek', fn: 'stretchEntity', args: [{ id: 11, layerId: 'a', attrs: {}, kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }, { minX: 8, minY: -1, maxX: 12, maxY: 1 }, 2, 0] },
    { name: 'pencere dışında', fn: 'stretchEntity', args: [{ id: 12, layerId: 'a', attrs: {}, kind: 'line', a: v(0, 0), b: v(10, 0) }, { minX: 20, minY: 20, maxX: 30, maxY: 30 }, 1, 1] },
    { name: 'emoji yazı kutusu', fn: 'textBox', args: [{ p: v(0, 0), text: '😀', height: 2, rotation: 0 }] },
  ],
  random: (g, n) => [
    ...repeat(g, 'tessellateCircle', n, () => [g.pt(), g.num(0, 30), g.chance(0.5) ? undefined : g.int(3, 128)]),
    ...repeat(g, 'entityVertices', n, () => [entity(g)]),
    ...repeat(g, 'entityOutline', n, () => [entity(g), g.chance(0.6) ? undefined : g.int(8, 96)]),
    ...repeat(g, 'polygonRing', n, () => [entity(g, 'polygon')]),
    ...repeat(g, 'polygonHoles', n, () => [entity(g, g.pick(['polygon', 'hatch', 'line'] as const))]),
    ...repeat(g, 'insidePolygon', n, () => {
      const e = entity(g, g.pick(['polygon', 'polyline'] as const)) as Extract<Entity, { kind: 'polygon' | 'polyline' }>;
      const c = e.pts[0] ?? v(0, 0);
      return [e, v(c.x + g.num(-20, 20), c.y + g.num(-20, 20))];
    }),
    ...repeat(g, 'textBox', n, () => [entity(g, 'text')]),
    ...repeat(g, 'isClosedOutline', n, () => [entity(g)]),
    ...repeat(g, 'entityBounds', n, () => [entity(g)]),
    ...repeat(g, 'entityAnchor', n, () => [entity(g)]),
    ...repeat(g, 'entityLength', n, () => [entity(g)]),
    ...repeat(g, 'entityArea', n, () => [entity(g)]),
    ...repeat(g, 'entityGeometry', n, () => [entity(g)]),
    ...repeat(g, 'entityEdges', n, () => [entity(g)]),
    ...repeat(g, 'edgeLength', n, () => [g.chance(0.5) ? { kind: 'seg', a: g.pt(), b: g.pt() } : { kind: 'arc', c: g.pt(), r: g.num(0, 30), a0: angle(g), sweep: g.num(-TAU, TAU) }]),
    ...repeat(g, 'transformEntity', n, () => [entity(g), affine(g)]),
    ...repeat(g, 'translateEntity', n, () => [entity(g), g.num(-100, 100), g.num(-100, 100)]),
    ...repeat(g, 'stretchEntity', n, () => {
      const e = entity(g);
      const c = g.pt();
      return [e, { minX: c.x - g.num(0, 60), minY: c.y - g.num(0, 60), maxX: c.x + g.num(0, 60), maxY: c.y + g.num(0, 60) }, g.num(-10, 10), g.num(-10, 10)];
    }),
    ...repeat(g, 'entityGrips', n, () => [entity(g)]),
    ...repeat(g, 'moveGrip', n, () => {
      const e = entity(g);
      return [e, g.int(0, Math.max(0, entityGrips(e).length + 1)), g.pt()];
    }),
    ...repeat(g, 'midGripSegment', n, () => [entity(g, g.pick(['polygon', 'polyline', 'line'] as const)), g.int(0, 20)]),
    ...repeat(g, 'holeGrip', n, () => [entity(g, 'polygon'), g.int(0, 25)]),
  ],
};
