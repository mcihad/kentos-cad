import type { Vec2 } from '../../../model/geometry';
import { TAU } from '../../../model/geom/arc';
import {
  type EllipseGeom } from '../../../model/geom/ellipse';
import type { Edge } from '../../../model/geom/intersect';
import { repeat, type CallSet, type Gen } from '../harness';

/** P2: ellipses, splines, shape constructions, surveying, tangent circles (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const seg = (a: Vec2, b: Vec2): Edge => ({ kind: 'seg', a, b });
const circ = (c: Vec2, r: number): Edge => ({ kind: 'arc', c, r, a0: 0, sweep: TAU });
const E = 487000;
const N = 4420000;

function angle(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, Math.PI / 2, Math.PI, 1.5 * Math.PI, TAU, -Math.PI / 2, TAU - 1e-10]) : g.num(-TAU, 2 * TAU);
}

function ellipse(g: Gen): EllipseGeom {
  const full = g.chance(0.35);
  const t0 = angle(g);
  // The major axis is a vector from the centre (tens of metres), never a TM position.
  return { c: g.pt(), major: g.chance(0.2) ? v(g.pick([10, -10, 0]), g.pick([0, 5])) : v(g.num(-40, 40), g.num(-40, 40)), ratio: g.chance(0.2) ? g.pick([1, 0.5, 0.01]) : g.num(0.05, 1), t0, t1: full ? t0 : angle(g) };
}

function edge(g: Gen): Edge {
  if (g.chance(0.4)) return { kind: 'arc', c: g.chance(0.5) ? g.gridPt(5, 3) : g.pt(), r: g.num(0.5, 30), a0: angle(g), sweep: g.num(-TAU, TAU) };
  return g.chance(0.5) ? seg(g.gridPt(5, 3), g.gridPt(5, 3)) : seg(g.pt(), g.pt());
}

export const P2: CallSet = {
  file: 'calls-p2-curves.json',
  named: [
    { name: 'tam elips uzunluğu', fn: 'ellipseLength', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }] },
    { name: 'daire olan elips', fn: 'ellipseArea', args: [{ c: v(0, 0), major: v(0, 3), ratio: 1, t0: 1, t1: 1 }] },
    { name: 'merkezde en yakın', fn: 'closestParam', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, v(0, 0)] },
    { name: 'yay dışında en yakın uç', fn: 'closestParam', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: Math.PI / 2 }, v(-5, -5)] },
    { name: 'teğet doğru', fn: 'lineEllipse', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, v(-20, 5), v(20, 5)] },
    { name: 'içeriden teğet yok', fn: 'ellipseTangentPoints', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, v(1, 1)] },
    { name: 'küçük eksen verilince döner', fn: 'ellipseFromCenter', args: [v(0, 0), v(3, 0), 5] },
    { name: 'sıfır eksen', fn: 'ellipseFromAxis', args: [v(1, 1), v(1, 1), 3] },
    { name: 'iki noktalı eğri', fn: 'catmullRom', args: [[v(0, 0), v(5, 5)], false, undefined] },
    { name: 'çakışık noktalı eğri', fn: 'catmullRom', args: [[v(0, 0), v(0, 0), v(5, 5), v(10, 0)], false, undefined] },
    { name: 'kapalı eğri', fn: 'catmullRom', args: [[v(0, 0), v(10, 0), v(10, 10), v(0, 10)], true, 8] },
    { name: 'sıfır genişlik', fn: 'rectFromEdge', args: [v(0, 0), v(10, 0), 0] },
    { name: 'dejenere köşeler', fn: 'rectFromCorners', args: [v(0, 0), v(0, 5), undefined] },
    { name: 'dıştan çokgen', fn: 'regularPolygon', args: [v(0, 0), 6, v(10, 0), 'circumscribed'] },
    { name: 'yuvarlanan kenar sayısı', fn: 'regularPolygon', args: [v(E, N), 4.6, v(E + 10, N), 'inscribed'] },
    { name: 'iki kenarlı çokgen yok', fn: 'regularPolygonOnEdge', args: [v(0, 0), v(1, 0), 2.4] },
    { name: 'tam tur yay', fn: 'arcStartCenterEnd', args: [v(10, 0), v(0, 0), v(20, 0)] },
    { name: 'saat yönü açı', fn: 'arcStartCenterAngle', args: [v(10, 0), v(0, 0), -90] },
    { name: 'çaptan uzun kiriş', fn: 'arcStartCenterChord', args: [v(10, 0), v(0, 0), 25] },
    { name: 'büyük yay kirişi', fn: 'arcStartCenterChord', args: [v(10, 0), v(0, 0), -10] },
    { name: 'tam tur açı reddi', fn: 'arcStartEndAngle', args: [v(0, 0), v(10, 0), 360] },
    { name: 'arkaya bakan yön', fn: 'arcStartEndDirection', args: [v(0, 0), v(-10, 0), v(1, 0)] },
    { name: 'eksi yarıçap: büyük yay', fn: 'arcStartEndRadius', args: [v(0, 0), v(10, 0), -8] },
    { name: 'yarım kirişten küçük yarıçap', fn: 'arcStartEndRadius', args: [v(0, 0), v(10, 0), 4] },
    { name: 'saat yönü halkadan bulut', fn: 'cloudOf', args: [[v(0, 0), v(0, 10), v(10, 10), v(10, 0)], 2.5] },
    { name: 'sıfır adım bulut', fn: 'cloudOf', args: [[v(0, 0), v(10, 0), v(10, 10)], 0] },
    { name: 'yan nokta, TM', fn: 'sidePoint', args: [v(E, N), v(E + 30, N + 40), 12.5, -3.25] },
    { name: 'aynı nokta: yön yok', fn: 'sideOffsets', args: [v(1, 1), v(1, 1), v(5, 5)] },
    { name: 'kenar kesişimi: sağdaki önce', fn: 'distanceIntersection', args: [v(0, 0), v(10, 0), 6, 8] },
    { name: 'kenar kesişimi: değen', fn: 'distanceIntersection', args: [v(0, 0), v(10, 0), 4, 6] },
    { name: 'kenar kesişimi: sıfır uzaklık', fn: 'distanceIntersection', args: [v(0, 0), v(10, 0), 0, 6] },
    { name: 'paralel doğru kesişimi', fn: 'lineIntersection', args: [v(0, 0), v(10, 0), v(0, 1), v(10, 1)] },
    { name: 'arkaya hat üzerinde', fn: 'alongLine', args: [v(E, N), v(E + 3, N + 4), -7.5] },
    { name: 'açı-mesafe, 100 grad', fn: 'polarPoint', args: [v(E, N), v(E, N + 50), Math.PI / 2, 25] },
    { name: 'saat yönü açı: aynı yön', fn: 'clockwiseAngle', args: [v(0, 0), v(1, 0), v(2, 0)] },
    { name: 'köşeye teğet daire', fn: 'tangentTangentRadius', args: [seg(v(0, 0), v(10, 0)), v(3, 0), seg(v(0, 0), v(0, 10)), v(0, 3), 2] },
    { name: 'doğru ve daireye teğet', fn: 'tangentTangentRadius', args: [seg(v(-10, 0), v(10, 0)), v(3, 0), circ(v(0, 5), 2), v(1, 4), 2] },
    { name: 'sığmayan yarıçap', fn: 'tangentTangentRadius', args: [seg(v(0, 0), v(10, 0)), v(1, 0), seg(v(0, 1), v(10, 1)), v(1, 1), 5] },
    { name: 'üçgenin iç teğet dairesi', fn: 'tangentTangentTangent', args: [[seg(v(0, 0), v(4, 0)), seg(v(4, 0), v(0, 3)), seg(v(0, 3), v(0, 0))], [v(2, 0), v(2, 1.5), v(0, 1.5)]] },
    { name: 'üç daireye dıştan teğet', fn: 'tangentTangentTangent', args: [[circ(v(0, 0), 1), circ(v(4, 0), 1), circ(v(2, 2 * Math.sqrt(3)), 1)], [v(0.8, 0.5), v(3.2, 0.5), v(2, 2 * Math.sqrt(3) - 1)]] },
    { name: 'TM iki doğru ve daire', fn: 'tangentTangentTangent', args: [[seg(v(E - 20, N), v(E + 20, N)), seg(v(E, N - 5), v(E, N + 20)), circ(v(E + 10, N + 2), 2)], [v(E + 3, N), v(E, N + 3), v(E + 8.2, N + 2)]] },
    { name: 'paralel üç doğru', fn: 'tangentTangentTangent', args: [[seg(v(0, 0), v(10, 0)), seg(v(0, 2), v(10, 2)), seg(v(0, 4), v(10, 4))], [v(1, 0), v(2, 2), v(3, 4)]] },
  ],
  random: (g, n) => [
    ...repeat(g, 'minorAxis', n, () => [ellipse(g)]),
    ...repeat(g, 'majorLength', n, () => [ellipse(g)]),
    ...repeat(g, 'ellipseSweep', n, () => [ellipse(g)]),
    ...repeat(g, 'isFullEllipse', n, () => [ellipse(g)]),
    ...repeat(g, 'ellipsePoint', n, () => [ellipse(g), angle(g)]),
    ...repeat(g, 'ellipseDerivative', n, () => [ellipse(g), angle(g)]),
    ...repeat(g, 'paramOfPoint', n, () => [ellipse(g), g.pt()]),
    ...repeat(g, 'paramAtPolar', n, () => [ellipse(g), angle(g)]),
    ...repeat(g, 'onEllipse', n, () => [ellipse(g), angle(g), g.chance(0.3) ? g.num(0, 0.1) : undefined]),
    ...repeat(g, 'tessellateEllipse', n, () => [ellipse(g), g.chance(0.5) ? undefined : g.int(4, 300)]),
    ...repeat(g, 'ellipseLength', n, () => [ellipse(g)]),
    ...repeat(g, 'ellipseArea', n, () => [ellipse(g)]),
    ...repeat(g, 'closestParam', n, () => {
      const e = ellipse(g);
      // Not the centre of a circle: every parameter is nearest there, and the answer is noise.
      return [e, { x: e.c.x + g.num(-60, 60), y: e.c.y + g.num(-60, 60) }];
    }),
    ...repeat(g, 'lineEllipse', n, () => {
      const e = ellipse(g);
      const near = () => ({ x: e.c.x + g.num(-50, 50), y: e.c.y + g.num(-50, 50) });
      return [e, near(), near()];
    }),
    ...repeat(g, 'ellipseTangentPoints', n, () => {
      const e = ellipse(g);
      return [e, { x: e.c.x + g.num(-80, 80), y: e.c.y + g.num(-80, 80) }];
    }),
    ...repeat(g, 'quadrantParams', n, () => [ellipse(g)]),
    ...repeat(g, 'insideEllipse', n, () => {
      const e = ellipse(g);
      return [e, { x: e.c.x + g.num(-40, 40), y: e.c.y + g.num(-40, 40) }];
    }),
    ...repeat(g, 'ellipseFromAxis', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.pick([0, -3, g.num(-80, 80)])]),
    ...repeat(g, 'ellipseFromCenter', n, () => [g.pt(), g.pt(), g.pick([0, -3, g.num(-80, 80)])]),
    ...repeat(g, 'catmullRom', n, () => [g.chance(0.3) ? Array.from({ length: g.int(0, 6) }, () => g.gridPt(5, 2)) : g.pts(g.int(0, 8), 40), g.chance(0.5), g.chance(0.6) ? undefined : g.int(1, 20)]),
    ...repeat(g, 'rectFromEdge', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.pick([0, g.num(-50, 50)])]),
    ...repeat(g, 'sideDistance', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.pt()]),
    ...repeat(g, 'rectFromCorners', n, () => [g.pt(), g.pt(), g.chance(0.3) ? undefined : angle(g)]),
    ...repeat(g, 'rectFromSize', n, () => [g.pt(), g.num(-40, 40), g.num(-40, 40), angle(g), g.pt()]),
    ...repeat(g, 'regularPolygon', n, () => [g.pt(), g.pick([2, 3, 4.5, 6, g.num(0, 12)]), g.pt(), g.pick(['inscribed', 'circumscribed'])]),
    ...repeat(g, 'regularPolygonOnEdge', n, () => [g.pt(), g.pt(), g.pick([2, 3, 4.5, 6, g.num(0, 12)])]),
    ...repeat(g, 'arcStartCenterEnd', n, () => (g.chance(0.3) ? [g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'arcStartCenterAngle', n, () => [g.pt(), g.pt(), g.pick([0, 90, -90, 360, g.num(-720, 720)])]),
    ...repeat(g, 'arcStartCenterChord', n, () => [g.pt(), g.pt(), g.num(-150, 150)]),
    ...repeat(g, 'arcStartEndAngle', n, () => [g.pt(), g.pt(), g.pick([0, 180, -180, 360, g.num(-400, 400)])]),
    ...repeat(g, 'arcStartEndDirection', n, () => [g.pt(), g.pt(), g.vec(1)]),
    ...repeat(g, 'arcStartEndRadius', n, () => [g.pt(), g.pt(), g.num(-150, 150)]),
    ...repeat(g, 'arcStartEndCenter', n, () => [g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'cloudOf', n, () => [g.chance(0.8) ? g.ring(g.int(3, 7), 40, g.chance(0.5)) : g.pts(g.int(0, 3)), g.pick([0, -1, g.num(0.5, 30)])]),
    ...repeat(g, 'sidePoint', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.num(-50, 50), g.num(-50, 50)]),
    ...repeat(g, 'sideOffsets', n, () => [g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'distanceIntersection', n, () => [g.pt(), g.pt(), g.pick([0, -2, g.num(0, 150)]), g.num(0, 150)]),
    ...repeat(g, 'lineIntersection', n, () => (g.chance(0.3) ? [g.gridPt(), g.gridPt(), g.gridPt(), g.gridPt()] : [g.pt(), g.pt(), g.pt(), g.pt()])),
    ...repeat(g, 'alongLine', n, () => [g.pt(), g.pt(), g.num(-100, 100)]),
    ...repeat(g, 'polarPoint', n, () => [g.pt(), g.pt(), angle(g), g.num(-100, 100)]),
    ...repeat(g, 'clockwiseAngle', n, () => [g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'tangentTangentRadius', n, () => [edge(g), g.pt(), edge(g), g.pt(), g.pick([0, -1, g.num(0.1, 40)])]),
    ...repeat(g, 'tangentTangentTangent', n, () => [[edge(g), edge(g), edge(g)], [g.pt(), g.pt(), g.pt()]]),
  ],
};
