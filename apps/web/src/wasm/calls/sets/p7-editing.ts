import type { Entity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { normAngle } from '../../../model/geom/arc';
import { bulgeArc } from '../../../model/geom/bulge';
import type { Area } from '../../../model/geom/region';
import { areaOfEntity } from '../../../model/ops/areas';
import { entityEdges } from '../../../model/ops/edges';
import type { Seg } from '../../../model/ops/fillet';
import { nearestSegment } from '../../../model/ops/vertex';
import { repeat, type CallSet, type Gen } from '../harness';
import { entity } from './p5-entities';

/** P7: offset, fillet and chamfer, join, vertices, explode, entities ↔ areas (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const base = { id: 1, layerId: 'a', attrs: {} };

const anchorOf = (e: Entity): Vec2 => ('pts' in e && e.pts.length ? e.pts[0] : 'c' in e && e.c ? e.c : 'a' in e ? e.a : 'p' in e ? e.p : 'ring' in e ? e.ring[0] : v(0, 0));
const near = (g: Gen, p: Vec2, r: number) => v(p.x + g.num(-r, r), p.y + g.num(-r, r));

/** A path entity with two or more vertices (the TypeScript reads past a shorter one). */
function pathLike(g: Gen, kinds: readonly ('line' | 'polyline' | 'polygon')[] = ['line', 'polyline', 'polygon']): Entity {
  for (;;) {
    const e = entity(g, g.pick(kinds));
    if (e.kind !== 'polyline' || e.pts.length >= 2) return e;
  }
}

/** Two lines around a corner: their far ends, the picks on either side. */
function corner(g: Gen): [Seg, Vec2, Seg, Vec2] {
  const x = g.pt();
  const t1 = g.num(0, 2 * Math.PI);
  const t2 = g.chance(0.1) ? t1 + g.pick([0, Math.PI]) : g.num(0, 2 * Math.PI);
  const leg = (t: number): Seg => {
    const d = v(Math.cos(t), Math.sin(t));
    const s0 = g.num(-10, 5);
    const s1 = s0 + g.num(0.5, 30);
    return { a: v(x.x + d.x * s0, x.y + d.y * s0), b: v(x.x + d.x * s1, x.y + d.y * s1) };
  };
  const l1 = leg(t1);
  const l2 = leg(t2);
  const pick = (l: Seg) => {
    const k = g.num(-0.2, 1.2);
    return v(l.a.x + (l.b.x - l.a.x) * k, l.a.y + (l.b.y - l.a.y) * k);
  };
  return [l1, pick(l1), l2, pick(l2)];
}

/** Lines, arcs and polylines chained end to end (sometimes closing, sometimes reversed, ends within a jitter), shuffled, with strays. */
function chain(g: Gen): Entity[] {
  const out: Entity[] = [];
  const start = g.pt();
  const n = g.int(1, 6);
  const closing = g.chance(0.3);
  let p = start;
  for (let i = 0; i < n; i++) {
    const q = closing && i === n - 1 ? start : near(g, p, 20);
    const jitter = (w: Vec2) => (g.chance(0.3) ? near(g, w, 1e-4) : w);
    const [a, b] = g.chance(0.3) ? [jitter(q), p] : [p, jitter(q)];
    // Ids repeat now and then: the join keys on them.
    const own = { id: g.chance(0.1) ? 1 : g.int(2, 9999), layerId: 'a', attrs: {} };
    const kind = g.pick(['line', 'arc', 'polyline'] as const);
    const arc = kind === 'arc' ? bulgeArc(a, b, g.pick([0.5, -0.5, g.num(-1.5, 1.5)])) : null;
    if (arc) {
      const end = arc.a0 + arc.sweep;
      const [a0, a1] = arc.sweep > 0 ? [arc.a0, end] : [end, arc.a0];
      out.push({ ...own, kind: 'arc', c: arc.c, r: arc.r, a0: normAngle(a0), a1: normAngle(a1) });
    } else if (kind === 'polyline') {
      const m = near(g, v((a.x + b.x) / 2, (a.y + b.y) / 2), 5);
      out.push({ ...own, kind: 'polyline', pts: [a, m, b], ...(g.chance(0.5) ? { bulges: [g.num(-1, 1), 0, 0] } : {}) });
    } else out.push({ ...own, kind: 'line', a, b });
    p = q;
  }
  for (let i = g.int(0, 2); i > 0; i--) out.push(pathLike(g, ['line', 'polyline']));
  if (g.chance(0.3)) out.push(entity(g, g.pick(['circle', 'text', 'polygon'] as const)));
  for (let i = out.length - 1; i > 0; i--) {
    const j = g.int(0, i);
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

export const P7: CallSet = {
  file: 'calls-p7-editing.json',
  named: [
    { name: 'sıfır mesafe', fn: 'offsetEntity', args: [{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, 0, v(5, 5)] },
    { name: 'sıfır uzunluklu çizgi', fn: 'offsetEntity', args: [{ ...base, kind: 'line', a: v(3, 4), b: v(3, 4) }, 2, v(5, 5)] },
    { name: 'daireyi yok edecek kadar içe', fn: 'offsetEntity', args: [{ ...base, kind: 'circle', c: v(0, 0), r: 5 }, 5, v(1, 0)] },
    { name: 'yaylı alan', fn: 'offsetEntity', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], bulges: [0.3, 0, 0, 0] }, 1, v(5, 5)] },
    { name: 'TM çoklu çizgi', fn: 'offsetEntity', args: [{ ...base, kind: 'polyline', pts: [v(486000.12, 4420000.34), v(486010.5, 4420003.25), v(486020.75, 4419990.5)] }, 2.5, v(486010, 4420010)] },
    { name: 'paralel çizgiler', fn: 'filletLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(5, 0), { a: v(0, 5), b: v(10, 5) }, v(5, 5), 2] },
    { name: 'keskin köşe', fn: 'filletLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(2, 0), { a: v(12, 2), b: v(12, 10) }, v(12, 8), 0] },
    { name: 'dik köşeye yay', fn: 'filletLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(2, 0), { a: v(12, 2), b: v(12, 10) }, v(12, 8), 2] },
    { name: 'çok büyük yarıçap', fn: 'filletLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(2, 0), { a: v(12, 2), b: v(12, 10) }, v(12, 8), 50] },
    { name: 'sıfır pah', fn: 'chamferLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(2, 0), { a: v(12, 2), b: v(12, 10) }, v(12, 8), 0, 0] },
    { name: 'pah', fn: 'chamferLines', args: [{ a: v(0, 0), b: v(10, 0) }, v(2, 0), { a: v(12, 2), b: v(12, 10) }, v(12, 8), 2, 3] },
    { name: 'uç köşe değildir', fn: 'cornerOfPath', args: [[v(0, 0), v(10, 0), v(10, 10)], undefined, false, 0, { radius: 2 }] },
    { name: 'yaya komşu köşe', fn: 'cornerOfPath', args: [[v(0, 0), v(10, 0), v(10, 10)], [0.5, 0, 0], false, 1, { radius: 2 }] },
    { name: 'kapalı alanda ilk köşe', fn: 'cornerOfPath', args: [[v(0, 0), v(10, 0), v(10, 10), v(0, 10)], undefined, true, 0, { radius: 2 }] },
    { name: 'pahlı köşe', fn: 'cornerOfPath', args: [[v(0, 0), v(10, 0), v(10, 10)], undefined, false, 1, { d1: 2, d2: 3 }] },
    { name: 'doğrusal köşe', fn: 'cornerOfPath', args: [[v(0, 0), v(5, 0), v(10, 0)], undefined, false, 1, { radius: 1 }] },
    { name: 'iki çizgi', fn: 'joinEntities', args: [[{ ...base, id: 1, kind: 'line', a: v(0, 0), b: v(10, 0) }, { ...base, id: 2, kind: 'line', a: v(10, 0), b: v(10, 10) }], 1e-6] },
    { name: 'üçgen kapanır', fn: 'joinEntities', args: [[{ ...base, id: 1, kind: 'line', a: v(0, 0), b: v(10, 0) }, { ...base, id: 2, kind: 'line', a: v(0, 10), b: v(10, 0) }, { ...base, id: 3, kind: 'line', a: v(0, 10), b: v(0, 0) }], 1e-6] },
    { name: 'yay ve çizgi', fn: 'joinEntities', args: [[{ ...base, id: 1, kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }, { ...base, id: 2, kind: 'line', a: v(0, 10), b: v(-10, 10) }], 1e-6] },
    { name: 'bağlanmayanlar', fn: 'joinEntities', args: [[{ ...base, id: 1, kind: 'line', a: v(0, 0), b: v(10, 0) }, { ...base, id: 2, kind: 'circle', c: v(0, 0), r: 3 }, { ...base, id: 3, kind: 'line', a: v(20, 0), b: v(30, 0) }], 1e-6] },
    { name: 'çizginin ortasına', fn: 'insertVertex', args: [{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, 0, v(4, 1)] },
    { name: 'yayı bölmek', fn: 'insertVertex', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0)], bulges: [1, 0] }, 0, v(5, -5)] },
    { name: 'köşeye çok yakın', fn: 'insertVertex', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }, 0, v(0, 0)] },
    { name: 'deliğin kenarına', fn: 'insertVertex', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], holes: [{ pts: [v(4, 4), v(6, 4), v(5, 6)] }] }, 4, v(5, 3.9)] },
    { name: 'üç köşeli alan', fn: 'removeVertex', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10)] }, 1] },
    { name: 'ilk köşe', fn: 'removeVertex', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0.5, 0.5, 0] }, 0] },
    { name: 'son köşe', fn: 'removeVertex', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0.5, 0.5, 0] }, 2] },
    { name: 'alanın ilk köşesi', fn: 'removeVertex', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], bulges: [0, 0, 0, 0.4] }, 0] },
    { name: 'delikli alan', fn: 'explodeEntity', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], bulges: [0, -0.5, 0, 0], holes: [{ pts: [v(4, 4), v(6, 4), v(5, 6)] }] }, ''] },
    { name: 'açı ölçüsü', fn: 'explodeEntity', args: [{ ...base, kind: 'dimension', a: v(10, 0), b: v(0, 10), c: v(0, 0), offset: 6, height: 1, style: 'angular' }, '90°'] },
    { name: 'kendi yazısı olan ölçü', fn: 'explodeEntity', args: [{ ...base, kind: 'dimension', a: v(0, 0), b: v(10, 0), offset: 2, height: 0.5, text: 'Çıkma 1' }, '10.00'] },
    { name: 'çapraz tarama', fn: 'explodeEntity', args: [{ ...base, kind: 'hatch', ring: [v(0, 0), v(4, 0), v(4, 4), v(0, 4)], pattern: { type: 'cross', angle: 45, spacing: 1 } }, ''] },
    { name: 'dolu tarama', fn: 'explodeEntity', args: [{ ...base, kind: 'hatch', ring: [v(0, 0), v(4, 0), v(4, 4)], pattern: { type: 'solid', angle: 0, spacing: 1 } }, ''] },
    { name: 'daire patlamaz', fn: 'explodeEntity', args: [{ ...base, kind: 'circle', c: v(0, 0), r: 2 }, ''] },
    { name: 'kapalı eğri', fn: 'explodeEntity', args: [{ ...base, kind: 'spline', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], closed: true }, ''] },
    { name: 'daire alan olur', fn: 'areaOfEntity', args: [{ ...base, kind: 'circle', c: v(486000, 4420000), r: 12.5 }] },
    { name: 'kapanan çoklu çizgi', fn: 'areaOfEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 0)], bulges: [0, 0.2, 0.3, 0] }] },
    { name: 'açık çoklu çizgi', fn: 'areaOfEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }] },
    { name: 'tam elips', fn: 'areaOfEntity', args: [{ ...base, kind: 'ellipse', c: v(0, 0), major: v(40, 0), ratio: 0.5, t0: 0, t1: 0 }] },
    { name: 'kapalı eğri alanı', fn: 'areaOfEntity', args: [{ ...base, kind: 'spline', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], closed: true }] },
    { name: 'delikli alan geri', fn: 'polygonOfArea', args: [{ outer: { pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0.2, 0, 0] }, holes: [{ pts: [v(5, 2), v(6, 2), v(6, 3)] }] }] },
    { name: 'deliksiz alan geri', fn: 'polygonOfArea', args: [{ outer: { pts: [v(0, 0), v(10, 0), v(10, 10)] }, holes: [] }] },
    { name: 'yaylı delikli alan halkaları', fn: 'polylinesOfPolygon', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)], bulges: [0.5, 0, 0, 0], holes: [{ pts: [v(4, 4), v(6, 4), v(5, 6)] }] }] },
    { name: 'çizgiler, delikli alan ve yay', fn: 'lineSource', args: [[{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, { ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10)], holes: [{ pts: [v(6, 2), v(8, 2), v(8, 4)] }] }, { ...base, kind: 'arc', c: v(0, 0), r: 5, a0: 0, a1: 1 }, { ...base, kind: 'text', p: v(1, 1), text: 'x', height: 1, rotation: 0 }]] },
  ],
  random: (g, n) => [
    ...repeat(g, 'offsetEntity', n, () => {
      const e = entity(g);
      return [e, g.pick([0, -1, g.num(0, 20)]), near(g, anchorOf(e), 40)];
    }),
    ...repeat(g, 'filletLines', n, () => [...corner(g), g.pick([0, -1, g.num(0, 15)])]),
    ...repeat(g, 'chamferLines', n, () => [...corner(g), g.pick([0, g.num(0, 15)]), g.pick([0, g.num(0, 15)])]),
    ...repeat(g, 'cornerOfPath', n, () => {
      const closed = g.chance(0.5);
      const e = entity(g, closed ? 'polygon' : 'polyline');
      const pts = 'pts' in e ? e.pts : [];
      const bulges = 'bulges' in e ? e.bulges : undefined;
      // Any vertex of a ring; an open path may also be asked for a missing one.
      const index = closed ? g.int(0, pts.length - 1) : g.int(0, pts.length);
      const op = g.chance(0.5) ? { radius: g.pick([0, -1, g.num(0, 15)]) } : { d1: g.pick([0, g.num(0, 15)]), d2: g.pick([0, g.num(0, 15)]) };
      return [pts, bulges, closed, index, op];
    }),
    ...repeat(g, 'joinEntities', n, () => [chain(g), g.pick([1e-6, 1e-3, 0.5])]),
    ...repeat(g, 'nearestSegment', n, () => {
      const e = entity(g);
      return [e, near(g, anchorOf(e), 40)];
    }),
    ...repeat(g, 'insertVertex', n, () => {
      // A polyline needs an edge to insert on: the TypeScript reads a missing one.
      const e = g.chance(0.85) ? pathLike(g) : entity(g, g.pick(['circle', 'arc', 'ellipse', 'point', 'text'] as const));
      const p = near(g, anchorOf(e), 30);
      const edges = entityEdges(e).length;
      // The segment the tools pick, or another edge of the entity.
      const seg = g.chance(0.6) || edges === 0 ? nearestSegment(e, p) : g.int(0, edges - 1);
      return [e, seg, p];
    }),
    ...repeat(g, 'removeVertex', n, () => {
      const e = g.chance(0.85) ? entity(g, g.pick(['polyline', 'polygon'] as const)) : entity(g);
      const count = 'pts' in e ? e.pts.length : 3;
      return [e, g.int(0, count)];
    }),
    ...repeat(g, 'explodeEntity', n, () => [entity(g), g.pick(['', '12.50 m', 'R 4.20', '90°'])]),
    ...repeat(g, 'areaOfEntity', n, () => [entity(g)]),
    ...repeat(g, 'polygonOfArea', n, () => {
      let a: Area | null = null;
      while (!a) a = areaOfEntity(entity(g, g.pick(['polygon', 'circle', 'ellipse', 'spline', 'polyline'] as const)));
      return [a];
    }),
    ...repeat(g, 'polylinesOfPolygon', n, () => {
      let e = entity(g, g.pick(['polygon', 'polyline'] as const));
      // An empty ring would come back as a point list holding `undefined`.
      while ('pts' in e && e.pts.length === 0) e = entity(g, 'polyline');
      return [e];
    }),
    ...repeat(g, 'lineSource', n, () => [Array.from({ length: g.int(0, 5) }, () => entity(g))]),
  ],
};
