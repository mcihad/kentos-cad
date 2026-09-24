import type { Vec2 } from '../../../model/geometry';
import { mirror, rotation, scaling, translation, type Affine } from '../../../model/geom/affine';
import { TAU, type ArcGeom } from '../../../model/geom/arc';
import {
  type Edge } from '../../../model/geom/intersect';
import { repeat, type CallSet, type Gen } from '../harness';

/** P1: points and rings, affine transforms, arcs, edge intersections, bulges (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const SQ = [v(0, 0), v(10, 0), v(10, 10), v(0, 10)];
const TM = [v(486500.1, 4420100.2), v(486540.35, 4420101.9), v(486538.8, 4420141.15), v(486501.05, 4420139.6)];

/** Angles that cross 0/2π and hit exact quarters, plus random ones. */
function angle(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, Math.PI / 2, Math.PI, 1.5 * Math.PI, TAU, -Math.PI / 2, TAU - 1e-10, 1e-10, -1e-13]) : g.num(-2 * TAU, 2 * TAU);
}

function arcEdge(g: Gen): Extract<Edge, { kind: 'arc' }> {
  const c = g.chance(0.4) ? g.gridPt(5, 3) : g.pt();
  const sw = g.chance(0.2) ? g.pick([TAU, -TAU]) : g.num(-TAU, TAU);
  return { kind: 'arc', c, r: g.chance(0.3) ? g.pick([5, 10, 15]) : g.num(0.5, 60), a0: angle(g), sweep: sw };
}

function edge(g: Gen): Edge {
  if (g.chance(0.5)) return arcEdge(g);
  return g.chance(0.4) ? { kind: 'seg', a: g.gridPt(5, 3), b: g.gridPt(5, 3) } : { kind: 'seg', a: g.pt(), b: g.pt() };
}

function arcGeom(g: Gen): ArcGeom {
  return { c: g.pt(), r: g.num(0.1, 80), a0: angle(g), a1: angle(g) };
}

function affine(g: Gen): Affine {
  return g.pick([
    () => translation(g.num(-100, 100), g.num(-100, 100)),
    () => rotation(angle(g), g.pt()),
    () => scaling(g.num(-3, 3), g.pt()),
    () => mirror(g.pt(), g.pt()),
    () => [g.num(-2, 2), g.num(-2, 2), g.num(-2, 2), g.num(-2, 2), g.num(-1e6, 1e6), g.num(-1e6, 1e6)] as Affine,
  ])();
}

/** Bulges for an n-vertex path: some straight, some half circles, some small arcs. */
function bulges(g: Gen, n: number): number[] | undefined {
  if (g.chance(0.2)) return undefined;
  return Array.from({ length: n }, () => (g.chance(0.4) ? 0 : g.chance(0.2) ? g.pick([1, -1, 0.41421356237309503, 1e-13]) : g.num(-2, 2)));
}

function path(g: Gen): Vec2[] {
  const n = g.int(0, 7);
  const pts = g.chance(0.5) ? g.pts(n, 40) : Array.from({ length: n }, () => g.gridPt(5, 2));
  // Now and then a repeated vertex (zero-length segment).
  if (n > 1 && g.chance(0.3)) pts.splice(g.int(1, n - 1), 0, { ...pts[g.int(0, n - 1)] });
  return pts;
}

export const P1: CallSet = {
  file: 'calls-p1-primitives.json',
  named: [
    { name: 'boş halkanın ağırlık merkezi (NaN)', fn: 'centroid', args: [[]] },
    { name: 'doğrusal noktalar', fn: 'centroid', args: [[v(0, 0), v(1, 1), v(2, 2)]] },
    { name: 'TM parseli', fn: 'centroid', args: [TM] },
    { name: 'kapalı kare', fn: 'pathLength', args: [SQ, true] },
    { name: 'sıfır uzunluklu parça', fn: 'distToSegment', args: [v(3, 4), v(1, 1), v(1, 1)] },
    { name: 'kuzey 0 grad', fn: 'bearingGrad', args: [v(0, 0), v(0, 5)] },
    { name: 'batı 300 grad', fn: 'bearingGrad', args: [v(0, 0), v(-5, 0)] },
    { name: 'aynı nokta: açı 0', fn: 'angleDeg', args: [v(2, 2), v(2, 2)] },
    { name: 'sıfır uzunluklu ayna', fn: 'mirror', args: [v(1, 1), v(1, 1)] },
    { name: 'varsayılan merkez', fn: 'rotation', args: [Math.PI / 3, undefined] },
    { name: 'eksi sıfır', fn: 'normAngle', args: [-0] },
    { name: 'tam tur', fn: 'normAngle', args: [TAU] },
    { name: 'eşit açılar: tam tur', fn: 'sweep', args: [1, 1] },
    { name: 'doğrusal üç nokta', fn: 'circleThrough', args: [v(0, 0), v(1, 1), v(2, 2)] },
    { name: 'saat yönünde seçilen yay', fn: 'arcThrough', args: [v(10, 0), v(0, 10), v(-10, 0)] },
    { name: 'TM yayı', fn: 'arcThrough', args: [v(486500, 4420100), v(486510, 4420110), v(486520, 4420100)] },
    { name: 'paralel doğrular', fn: 'lineLine', args: [v(0, 0), v(10, 0), v(0, 5), v(10, 5)] },
    { name: 'çakışık doğrular', fn: 'lineLine', args: [v(0, 0), v(10, 0), v(2, 0), v(8, 0)] },
    { name: 'uçta değen parçalar', fn: 'segSeg', args: [v(0, 0), v(10, 0), v(10, 0), v(10, 10), undefined] },
    { name: 'teğet doğru', fn: 'lineCircleParams', args: [v(-10, 5), v(10, 5), v(0, 0), 5] },
    { name: 'yardımcı çizgi boyu (1000 km)', fn: 'lineCircleParams', args: [v(-1e6, 3), v(1e6, 3), v(0, 0), 5] },
    { name: 'eş merkezli çemberler', fn: 'circleCircle', args: [v(0, 0), 5, v(0, 0), 3] },
    { name: 'dıştan teğet çemberler', fn: 'circleCircle', args: [v(0, 0), 5, v(10, 0), 5] },
    { name: 'kare köşesinde kesişim', fn: 'intersectEdges', args: [{ kind: 'seg', a: v(0, 0), b: v(10, 0) }, { kind: 'arc', c: v(0, 0), r: 10, a0: 0, sweep: Math.PI / 2 }] },
    { name: 'saat yönü yayla yay', fn: 'intersectEdges', args: [{ kind: 'arc', c: v(0, 0), r: 10, a0: Math.PI, sweep: -Math.PI }, { kind: 'arc', c: v(10, 0), r: 10, a0: 0, sweep: TAU }] },
    { name: 'yay dışında en yakın uç', fn: 'closestOnEdge', args: [{ kind: 'arc', c: v(0, 0), r: 10, a0: 0, sweep: Math.PI / 2 }, v(-5, -5)] },
    { name: 'merkezde dik ayak yok', fn: 'perpendicularFoot', args: [{ kind: 'arc', c: v(0, 0), r: 10, a0: 0, sweep: TAU }, v(0, 0)] },
    { name: 'ışın çembere', fn: 'rayEdge', args: [v(-20, 0), v(1, 0), { kind: 'arc', c: v(0, 0), r: 5, a0: 0, sweep: TAU }, undefined] },
    { name: 'içeriden teğet yok', fn: 'tangentPoints', args: [v(1, 1), v(0, 0), 5] },
    { name: 'dizi dışı bulge', fn: 'bulgeAt', args: [[0.5], 3] },
    { name: 'bulge yok', fn: 'bulgeAt', args: [undefined, 0] },
    { name: 'yarım daire', fn: 'bulgeArc', args: [v(0, 0), v(10, 0), 1] },
    { name: 'sıfır kiriş', fn: 'bulgeArc', args: [v(3, 3), v(3, 3), 0.5] },
    { name: 'arkaya dönen teğet', fn: 'tangentBulge', args: [v(0, 0), v(1, 0), v(-5, 0)] },
    { name: 'sıfır uzunluklu teğet', fn: 'segmentTangent', args: [v(1, 1), v(1, 1), 0, true] },
    { name: 'kapalı yaylı kare', fn: 'bulgePathOutline', args: [SQ, [0, 0.5, 0, -0.3], true, undefined] },
    { name: 'açık yol, tek köşe', fn: 'bulgePathOutline', args: [[v(1, 2)], [1], false, undefined] },
    { name: 'kapalı yaylı alan', fn: 'bulgeRingArea', args: [SQ, [0, 1, 0, 0]] },
    { name: 'tek köşeli kapalı ters', fn: 'reverseBulgePath', args: [[v(1, 1)], [0.5], true] },
    { name: 'boş yol ters', fn: 'reverseBulgePath', args: [[], undefined, false] },
    { name: 'kapanışta tekrar eden köşe', fn: 'cleanBulgePath', args: [[v(0, 0), v(10, 0), v(10, 10), v(0, 0)], [0, 0.5, 0.2, 0.7], true, undefined] },
    { name: 'art arda aynı köşeler', fn: 'cleanBulgePath', args: [[v(0, 0), v(0, 0), v(5, 0), v(5, 0)], [0.3, 0.1, 0, 0.9], false, undefined] },
  ],
  random: (g, n) => [
    ...repeat(g, 'dist', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'signedArea', n, () => [g.ring(g.int(3, 12), 80, g.chance(0.5))]),
    ...repeat(g, 'pathLength', n, () => [path(g), g.chance(0.5)]),
    ...repeat(g, 'centroid', n, () => [g.chance(0.7) ? g.ring(g.int(3, 12), 60, g.chance(0.5)) : path(g)]),
    ...repeat(g, 'pointInPolygon', n, () => [g.chance(0.5) ? g.gridPt(10) : g.pt(), g.chance(0.5) ? g.ring(g.int(3, 9)) : Array.from({ length: g.int(3, 7) }, () => g.gridPt(10))]),
    ...repeat(g, 'distToSegment', n, () => [g.pt(), g.pt(), g.chance(0.2) ? g.gridPt() : g.pt()]),
    ...repeat(g, 'angleDeg', n, () => [g.pt(), g.chance(0.2) ? g.gridPt() : g.pt()]),
    ...repeat(g, 'bearingGrad', n, () => [g.gridPt(), g.chance(0.5) ? g.gridPt() : g.pt()]),
    ...repeat(g, 'translation', n, () => [g.num(-1e6, 1e6), g.num(-1e6, 1e6)]),
    ...repeat(g, 'rotation', n, () => [angle(g), g.chance(0.2) ? undefined : g.pt()]),
    ...repeat(g, 'scaling', n, () => [g.num(-5, 5), g.chance(0.2) ? undefined : g.pt()]),
    ...repeat(g, 'mirror', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt()]),
    ...repeat(g, 'compose', n, () => [affine(g), affine(g)]),
    ...repeat(g, 'apply', n, () => [affine(g), g.pt()]),
    ...repeat(g, 'applyLinear', n, () => [affine(g), g.vec(5)]),
    ...repeat(g, 'determinant', n, () => [affine(g)]),
    ...repeat(g, 'lengthScale', n, () => [affine(g)]),
    ...repeat(g, 'isReflection', n, () => [affine(g)]),
    ...repeat(g, 'normAngle', n, () => [angle(g)]),
    ...repeat(g, 'sweep', n, () => [angle(g), angle(g)]),
    ...repeat(g, 'onArc', n, () => [angle(g), angle(g), g.num(0, TAU), g.chance(0.3) ? g.num(0, 0.1) : undefined]),
    ...repeat(g, 'arcParam', n, () => [angle(g), angle(g), g.num(0.01, TAU)]),
    ...repeat(g, 'pointOnCircle', n, () => [g.pt(), g.num(0, 50), angle(g)]),
    ...repeat(g, 'circleThrough', n, () => (g.chance(0.3) ? [g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'arcThrough', n, () => (g.chance(0.3) ? [g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'tessellateArc', n, () => [arcGeom(g), g.chance(0.5) ? undefined : g.num(0.05, 1)]),
    ...repeat(g, 'arcStart', n, () => [arcGeom(g)]),
    ...repeat(g, 'arcEnd', n, () => [arcGeom(g)]),
    ...repeat(g, 'arcMid', n, () => [arcGeom(g)]),
    ...repeat(g, 'arcLength', n, () => [arcGeom(g)]),
    ...repeat(g, 'bulgeFromArc', n, () => [arcGeom(g)]),
    ...repeat(g, 'onEdgeArc', n, () => [arcEdge(g), angle(g)]),
    ...repeat(g, 'lineLine', n, () => (g.chance(0.4) ? [g.gridPt(), g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'segSeg', n, () => [...(g.chance(0.4) ? [g.gridPt(), g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt(), g.pt()]), g.chance(0.3) ? g.num(0, 0.1) : undefined]),
    ...repeat(g, 'lineCircleParams', n, () => [g.pt(), g.pt(), g.pt(), g.num(0, 150)]),
    ...repeat(g, 'circleCircle', n, () => (g.chance(0.4) ? [g.gridPt(5, 2), g.pick([5, 10]), g.gridPt(5, 2), g.pick([5, 10])] : [g.pt(), g.num(0, 100), g.pt(), g.num(0, 100)])),
    ...repeat(g, 'paramOn', n, () => [edge(g), g.pt()]),
    ...repeat(g, 'pointAt', n, () => [edge(g), g.num(-0.5, 1.5)]),
    ...repeat(g, 'intersectEdges', n, () => [edge(g), edge(g)]),
    ...repeat(g, 'rayEdge', n, () => [g.pt(), g.vec(5), edge(g), g.chance(0.5) ? undefined : g.num(-1, 1)]),
    ...repeat(g, 'closestOnEdge', n, () => [edge(g), g.pt()]),
    ...repeat(g, 'perpendicularFoot', n, () => [edge(g), g.pt()]),
    ...repeat(g, 'fullCircle', n, () => [g.pt(), g.num(0, 50)]),
    ...repeat(g, 'tangentPoints', n, () => [g.pt(), g.pt(), g.num(0, 80)]),
    ...repeat(g, 'bulgeAt', n, () => [bulges(g, 4), g.int(0, 6)]),
    ...repeat(g, 'isArcBulge', n, () => [g.pick([0, 1e-13, -1e-11, g.num(-2, 2)])]),
    ...repeat(g, 'hasBulges', n, () => [bulges(g, g.int(0, 5))]),
    ...repeat(g, 'bulgeArc', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.pick([0, 1, -1, 1e-13, g.num(-3, 3)])]),
    ...repeat(g, 'bulgeOfSweep', n, () => [g.num(-TAU, TAU)]),
    ...repeat(g, 'segmentMid', n, () => [g.pt(), g.pt(), g.num(-2, 2)]),
    ...repeat(g, 'bulgeThrough', n, () => (g.chance(0.3) ? [g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'tangentBulge', n, () => [g.pt(), g.vec(1), g.chance(0.1) ? v(0, 0) : g.pt()]),
    ...repeat(g, 'segmentTangent', n, () => [g.pt(), g.pt(), g.num(-2, 2), g.chance(0.5)]),
    ...repeat(g, 'bulgePathEdges', n, () => {
      const p = path(g);
      return [p, bulges(g, p.length), g.chance(0.5)];
    }),
    ...repeat(g, 'bulgePathOutline', n, () => {
      const p = path(g);
      return [p, bulges(g, p.length), g.chance(0.5), g.chance(0.5) ? undefined : g.num(0.05, 1)];
    }),
    ...repeat(g, 'bulgePathLength', n, () => {
      const p = path(g);
      return [p, bulges(g, p.length), g.chance(0.5)];
    }),
    ...repeat(g, 'bulgeRingArea', n, () => {
      const p = g.chance(0.7) ? g.ring(g.int(3, 8), 40, g.chance(0.5)) : path(g);
      return [p, bulges(g, p.length)];
    }),
    ...repeat(g, 'reverseBulgePath', n, () => {
      const p = path(g);
      return [p, bulges(g, p.length), g.chance(0.5)];
    }),
    ...repeat(g, 'cleanBulgePath', n, () => {
      const p = path(g);
      return [p, bulges(g, p.length), g.chance(0.5), g.chance(0.3) ? g.num(0, 3) : undefined];
    }),
  ],
};
