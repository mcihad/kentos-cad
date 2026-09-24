import type { Entity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import type { Edge } from '../../../model/geom/intersect';
import { entityEdges } from '../../../model/ops/edges';
import { lengthOf } from '../../../model/ops/lengthen';
import { pathOf, type Path } from '../../../model/ops/path';
import { repeat, type CallSet, type Gen } from '../harness';
import { entity } from './p5-entities';

/** P6: paths by arc length, trim, extend, break, lengthen, ellipse and construction-line cuts (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const base = { id: 1, layerId: 'a', attrs: {} };

/** Boundary edges near the target: other entities' edges and a few crossing segments. */
function boundaries(g: Gen, near: Vec2): Edge[] {
  const out: Edge[] = [];
  for (let i = g.int(0, 3); i > 0; i--) out.push(...entityEdges(entity(g)));
  for (let i = g.int(0, 3); i > 0; i--) out.push({ kind: 'seg', a: v(near.x + g.num(-60, 60), near.y + g.num(-60, 60)), b: v(near.x + g.num(-60, 60), near.y + g.num(-60, 60)) });
  return out;
}

/** An entity that has a path (line, polyline with 2+ points, polygon, arc, circle, spline, ellipse…). */
function pathEntity(g: Gen): { e: Entity; path: Path } {
  for (;;) {
    const e = entity(g, g.pick(['line', 'polyline', 'polygon', 'circle', 'arc', 'spline', 'ellipse', 'dimension', 'hatch'] as const));
    const path = pathOf(e);
    if (path && path.length > 1e-6) return { e, path };
  }
}

const anchorOf = (e: Entity): Vec2 => ('pts' in e && e.pts.length ? e.pts[0] : 'c' in e && e.c ? e.c : 'a' in e ? e.a : 'p' in e ? e.p : 'ring' in e ? e.ring[0] : v(0, 0));

export const P6: CallSet = {
  file: 'calls-p6-path-editing.json',
  named: [
    { name: 'eğri budanamaz', fn: 'trimEntity', args: [{ ...base, kind: 'spline', pts: [v(0, 0), v(5, 5), v(10, 0)], closed: false }, v(5, 5), []] },
    { name: 'kesişimsiz budama', fn: 'trimEntity', args: [{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, v(5, 0), []] },
    { name: 'çizgiyi ortadan budamak', fn: 'trimEntity', args: [{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, v(5, 0), [{ kind: 'seg', a: v(3, -1), b: v(3, 1) }, { kind: 'seg', a: v(7, -1), b: v(7, 1) }]] },
    { name: 'daireden yay', fn: 'trimEntity', args: [{ ...base, kind: 'circle', c: v(0, 0), r: 10 }, v(10, 0), [{ kind: 'seg', a: v(-20, 5), b: v(20, 5) }, { kind: 'seg', a: v(-20, -5), b: v(20, -5) }]] },
    { name: 'yaylı çoklu çizgiyi uzatmak', fn: 'extendEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0)], bulges: [0.5, 0] }, v(10, 0), [{ kind: 'seg', a: v(-30, -30), b: v(30, 30) }]] },
    { name: 'yay uzatmak', fn: 'extendEntity', args: [{ ...base, kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }, v(0, 10), [{ kind: 'seg', a: v(-20, 0), b: v(-5, 0) }]] },
    { name: 'tek noktada kırmak', fn: 'breakEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }, v(10, 5), v(10, 5)] },
    { name: 'daire tek noktadan kırılmaz', fn: 'breakEntity', args: [{ ...base, kind: 'circle', c: v(0, 0), r: 10 }, v(10, 0), v(10, 0)] },
    { name: 'saat yönü alanı kırmak', fn: 'breakEntity', args: [{ ...base, kind: 'polygon', pts: [v(0, 0), v(0, 10), v(10, 10), v(10, 0)] }, v(0, 5), v(5, 10)] },
    { name: 'yayı kendini kapatacak kadar uzatmak', fn: 'lengthenEntity', args: [{ ...base, kind: 'arc', c: v(0, 0), r: 1, a0: 0, a1: Math.PI }, true, 7] },
    { name: 'kısa çoklu çizgi çoklu çizgi kalır', fn: 'lengthenEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }, true, 5] },
    { name: 'aynı uzunluk', fn: 'lengthenEntity', args: [{ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) }, false, 10] },
    { name: 'tek köşeli çoklu çizgi uzamaz', fn: 'extendEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0)] }, v(0, 0), [{ kind: 'seg', a: v(-5, -5), b: v(5, -5) }]] },
    { name: 'tek köşeli çoklu çizgi uzatılıp kısaltılamaz', fn: 'lengthenEntity', args: [{ ...base, kind: 'polyline', pts: [v(0, 0)] }, true, 5] },
    { name: 'eşit parçalar', fn: 'divisionParams', args: [pathOf({ ...base, kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)] })!, { parts: 4 }] },
    { name: 'aralıkla', fn: 'divisionParams', args: [pathOf({ ...base, kind: 'line', a: v(0, 0), b: v(10, 0) })!, { step: 2.5 }] },
    { name: 'yardımcı çizgiyi budamak', fn: 'trimConstruction', args: [{ ...base, kind: 'xline', p: v(0, 0), dir: v(1, 0) }, v(1, 0), [{ kind: 'seg', a: v(-3, -1), b: v(-3, 1) }, { kind: 'seg', a: v(3, -1), b: v(3, 1) }]] },
    { name: 'ışını başlangıcından kırmak', fn: 'breakConstruction', args: [{ ...base, kind: 'ray', p: v(0, 0), dir: v(0, 1) }, v(0, 0), v(0, 0)] },
    { name: 'elipsi içe ötelemek', fn: 'offsetEllipse', args: [{ ...base, kind: 'ellipse', c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, 1, v(0, 0)] },
  ],
  random: (g, n) => [
    ...repeat(g, 'pathOf', n, () => [entity(g)]),
    ...repeat(g, 'normS', n, () => {
      const { path } = pathEntity(g);
      return [path, g.num(-2, 2) * path.length];
    }),
    ...repeat(g, 'pointAtS', n, () => {
      const { path } = pathEntity(g);
      return [path, g.num(-0.5, 1.5) * path.length];
    }),
    ...repeat(g, 'tangentAtS', n, () => {
      const { path } = pathEntity(g);
      return [path, g.num(-0.5, 1.5) * path.length];
    }),
    ...repeat(g, 'nearestS', n, () => {
      const { e, path } = pathEntity(g);
      const a = anchorOf(e);
      return [path, v(a.x + g.num(-40, 40), a.y + g.num(-40, 40))];
    }),
    ...repeat(g, 'cutsOn', n, () => {
      const { e, path } = pathEntity(g);
      return [path, boundaries(g, anchorOf(e))];
    }),
    ...repeat(g, 'subPath', n, () => {
      const { e, path } = pathEntity(g);
      const s0 = g.num(0, path.length);
      return [path, s0, s0 + g.num(0, path.closed ? path.length : path.length - s0), e];
    }),
    ...repeat(g, 'divisionParams', n, () => [pathEntity(g).path, g.chance(0.5) ? { parts: g.pick([0, 1, 2, g.num(0, 12)]) } : { step: g.pick([0, -1, g.num(0.1, 20)]) }]),
    ...repeat(g, 'ellipseCrossings', n, () => {
      const e = entity(g, 'ellipse');
      return [e, boundaries(g, anchorOf(e))];
    }),
    ...repeat(g, 'trimEllipse', n, () => {
      const e = entity(g, 'ellipse');
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-30, 30), a.y + g.num(-30, 30)), boundaries(g, a)];
    }),
    ...repeat(g, 'breakEllipse', n, () => {
      const e = entity(g, 'ellipse');
      const a = anchorOf(e);
      const p1 = v(a.x + g.num(-30, 30), a.y + g.num(-30, 30));
      return [e, p1, g.chance(0.3) ? p1 : v(a.x + g.num(-30, 30), a.y + g.num(-30, 30))];
    }),
    ...repeat(g, 'extendEllipse', n, () => {
      const e = entity(g, 'ellipse');
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-30, 30), a.y + g.num(-30, 30)), boundaries(g, a)];
    }),
    ...repeat(g, 'offsetEllipse', n, () => {
      const e = entity(g, 'ellipse');
      const a = anchorOf(e);
      return [e, g.num(0, 10), v(a.x + g.num(-40, 40), a.y + g.num(-40, 40))];
    }),
    ...repeat(g, 'trimConstruction', n, () => {
      const e = entity(g, g.pick(['xline', 'ray'] as const));
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-50, 50), a.y + g.num(-50, 50)), boundaries(g, a)];
    }),
    ...repeat(g, 'breakConstruction', n, () => {
      const e = entity(g, g.pick(['xline', 'ray'] as const));
      const a = anchorOf(e);
      const p1 = v(a.x + g.num(-50, 50), a.y + g.num(-50, 50));
      return [e, p1, g.chance(0.3) ? p1 : v(a.x + g.num(-50, 50), a.y + g.num(-50, 50))];
    }),
    ...repeat(g, 'offsetConstruction', n, () => {
      const e = entity(g, g.pick(['xline', 'ray'] as const));
      return [e, g.num(-10, 10), g.pt()];
    }),
    ...repeat(g, 'trimEntity', n, () => {
      const e = entity(g);
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-30, 30), a.y + g.num(-30, 30)), boundaries(g, a)];
    }),
    ...repeat(g, 'extendEntity', n, () => {
      // Two or more vertices here; the named case covers a shorter path.
      let e = entity(g, g.pick(['line', 'polyline', 'arc', 'ellipse', 'circle', 'text'] as const));
      while (e.kind === 'polyline' && e.pts.length < 2) e = entity(g, 'polyline');
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-30, 30), a.y + g.num(-30, 30)), boundaries(g, a)];
    }),
    ...repeat(g, 'breakEntity', n, () => {
      const e = entity(g);
      const a = anchorOf(e);
      const p1 = v(a.x + g.num(-30, 30), a.y + g.num(-30, 30));
      return [e, p1, g.chance(0.3) ? p1 : v(a.x + g.num(-30, 30), a.y + g.num(-30, 30))];
    }),
    ...repeat(g, 'lengthOf', n, () => [entity(g)]),
    ...repeat(g, 'nearEnd', n, () => {
      const e = entity(g);
      const a = anchorOf(e);
      return [e, v(a.x + g.num(-30, 30), a.y + g.num(-30, 30))];
    }),
    ...repeat(g, 'lengthenEntity', n, () => {
      const e = entity(g, g.pick(['line', 'polyline', 'arc', 'circle'] as const));
      const l = lengthOf(e) ?? 10;
      return [e, g.chance(0.5), g.pick([0, -1, l, l * g.num(0.1, 3), g.num(0, 100)])];
    }),
    ...repeat(g, 'lengthToward', n, () => {
      const e = entity(g, g.pick(['line', 'polyline', 'arc'] as const));
      const a = anchorOf(e);
      return [e, g.chance(0.5), v(a.x + g.num(-50, 50), a.y + g.num(-50, 50))];
    }),
  ],
};
