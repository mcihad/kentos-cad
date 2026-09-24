import type { Vec2 } from '../../../model/geometry';
import { TAU } from '../../../model/geom/arc';
import type { DimensionGeom, DimensionStyle } from '../../../model/geom/dimension';
import { repeat, type CallSet, type Gen } from '../harness';

/** P3: path offsets, dimensions, hatch lines, edge-length labels (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const SQ = [v(0, 0), v(10, 0), v(10, 10), v(0, 10)];
const E = 486512.34;
const N = 4420187.52;
const PARCEL = [v(E, N), v(E + 23.417, N + 1.203), v(E + 25.881, N + 31.466), v(E + 2.004, N + 33.012)];
const STYLES: (DimensionStyle | undefined)[] = [undefined, 'aligned', 'linear', 'angular', 'radius', 'diameter'];

function path(g: Gen): Vec2[] {
  const n = g.int(0, 7);
  const pts = g.chance(0.5) ? g.pts(n, 30) : Array.from({ length: n }, () => g.gridPt(5, 2));
  if (n > 1 && g.chance(0.2)) pts.splice(g.int(1, n - 1), 0, { ...pts[g.int(0, n - 1)] });
  return pts;
}

function bulgesFor(g: Gen, n: number): number[] {
  return Array.from({ length: n }, () => (g.chance(0.5) ? 0 : g.chance(0.2) ? g.pick([1, -1, 0.5, -0.3]) : g.num(-1.5, 1.5)));
}

function dimension(g: Gen): DimensionGeom {
  const style = g.pick(STYLES);
  const d: DimensionGeom = { a: g.pt(), b: g.chance(0.1) ? v(0, 0) : g.pt(), offset: g.pick([0, -5, g.num(-40, 40)]), height: g.pick([0.5, 2.5, g.num(0.1, 5)]) };
  if (style) d.style = style;
  if (style === 'linear' && g.chance(0.8)) d.angle = g.pick([0, 90, g.num(-180, 360)]);
  if (style === 'angular' && g.chance(0.9)) d.c = g.pt();
  return d;
}

export const P3: CallSet = {
  file: 'calls-p3-offset-annotation.json',
  tolerance: {
    // A few ulps of a TM coordinate (4.4·10⁶ m: 1 ulp ≈ 9.3·10⁻¹⁰ m) — nanometres on a drawn hatch line.
    hatchLines: { abs: 2e-8, rel: 1e-14, why: 'Tarama çizgileri dünya koordinatında sin/cos ile döndürülür; V8 ile libm son bitte ayrışır, TM büyüklüğünde bu birkaç ulp eder.' },
  },
  named: [
    { name: 'kapalı kare dışa', fn: 'offsetPath', args: [SQ, 1, true] },
    { name: 'kapalı kare içe', fn: 'offsetPath', args: [SQ, -1, true] },
    { name: 'sivri köşede pah', fn: 'offsetPath', args: [[v(0, 0), v(10, 0), v(0, 0.5)], 1, false] },
    { name: 'doğrusal noktalar', fn: 'offsetPath', args: [[v(0, 0), v(5, 0), v(10, 0)], 2, false] },
    { name: 'tek nokta', fn: 'offsetPath', args: [[v(1, 1)], 2, false] },
    { name: 'sıfır uzunluklu yol', fn: 'offsetPath', args: [[v(1, 1), v(1, 1)], 2, false] },
    { name: 'TM parseli', fn: 'offsetPath', args: [PARCEL, 0.5, true] },
    { name: 'noktanın yanı', fn: 'sideOf', args: [SQ, true, v(5, -1)] },
    { name: 'yay yarıçapından büyük öteleme', fn: 'offsetBulgePath', args: [[v(0, 0), v(10, 0)], [1], 6, false] },
    { name: 'yaylı kare dışa', fn: 'offsetBulgePath', args: [SQ, [0, 0.5, 0, 0], -1, true] },
    { name: 'yaylı kare içe', fn: 'offsetBulgePath', args: [SQ, [0, 0.5, 0, 0], 1, true] },
    { name: 'teğet devam eden yay', fn: 'offsetBulgePath', args: [[v(0, 0), v(10, 0), v(20, 0)], [0, 1, 0], 1, false] },
    { name: 'boş yol', fn: 'offsetBulgePath', args: [[v(1, 1)], [], 1, false] },
    { name: 'hizalı ölçü', fn: 'layoutDimension', args: [{ a: v(0, 0), b: v(10, 0), offset: 3, height: 2.5 }] },
    { name: 'ters okunan ölçü', fn: 'layoutDimension', args: [{ a: v(10, 0), b: v(0, 0), offset: -3, height: 2.5, style: 'aligned' }] },
    { name: 'doğrusal ΔX', fn: 'layoutDimension', args: [{ a: v(0, 0), b: v(7, 12), offset: 4, height: 1, style: 'linear', angle: 90 }] },
    { name: 'açı ölçüsü', fn: 'layoutDimension', args: [{ a: v(10, 0), b: v(0, 10), offset: 6, height: 1, style: 'angular', c: v(0, 0) }] },
    { name: 'köşesiz açı', fn: 'layoutDimension', args: [{ a: v(10, 0), b: v(0, 10), offset: 6, height: 1, style: 'angular' }] },
    { name: 'çap', fn: 'layoutDimension', args: [{ a: v(E, N), b: v(E + 5, N + 5), offset: 4, height: 1, style: 'diameter' }] },
    { name: 'kısa yarıçap kılavuzu', fn: 'layoutDimension', args: [{ a: v(0, 0), b: v(5, 0), offset: 0.5, height: 1, style: 'radius' }] },
    { name: 'sıfır uzunluklu ölçü', fn: 'layoutDimension', args: [{ a: v(3, 3), b: v(3, 3), offset: 1, height: 1 }] },
    { name: 'noktalar arasında', fn: 'linearAngleFor', args: [v(0, 0), v(10, 10), v(5, 5)] },
    { name: 'yanda: düşey', fn: 'linearAngleFor', args: [v(0, 0), v(10, 10), v(15, 5)] },
    { name: 'dik kollar', fn: 'sectorArms', args: [v(0, 0), v(1, 0), v(0, 1), v(-1, -1)] },
    { name: 'aynı kol', fn: 'sectorArms', args: [v(0, 0), v(1, 0), v(1, 0), v(1, 1)] },
    { name: 'TM parseline 45° tarama', fn: 'hatchLines', args: [PARCEL, 45, 1, undefined] },
    { name: 'delikli kare', fn: 'hatchLines', args: [SQ, 0, 1, [[v(3, 3), v(7, 3), v(7, 7), v(3, 7)]]] },
    { name: 'sınırı aşan sıklık', fn: 'hatchLines', args: [SQ, 0, 1e-4, undefined] },
    { name: 'sıfır aralık', fn: 'hatchLines', args: [SQ, 0, 0, undefined] },
    { name: 'kareye ölçü yazıları', fn: 'edgeLabels', args: [SQ, true, 1, undefined, undefined, undefined] },
    { name: 'içeriden, yaylı', fn: 'edgeLabels', args: [SQ, true, 1, 0, [0, 0.5, 0, 0], 'inside'] },
    { name: 'kısa kenarlar atlanır', fn: 'edgeLabels', args: [PARCEL, true, 0.5, 24, undefined, undefined] },
  ],
  random: (g, n) => [
    ...repeat(g, 'offsetPath', n, () => {
      const closed = g.chance(0.5);
      return [closed && g.chance(0.6) ? g.ring(g.int(3, 8), 30, g.chance(0.5)) : path(g), g.pick([0, g.num(-5, 5)]), closed];
    }),
    ...repeat(g, 'sideOf', n, () => [path(g), g.chance(0.5), g.pt()]),
    ...repeat(g, 'offsetBulgePath', n, () => {
      const closed = g.chance(0.5);
      const p = closed && g.chance(0.6) ? g.ring(g.int(3, 7), 30, g.chance(0.5)) : path(g);
      return [p, bulgesFor(g, p.length), g.pick([0, g.num(-4, 4)]), closed];
    }),
    ...repeat(g, 'layoutDimension', n, () => [dimension(g)]),
    ...repeat(g, 'signedOffset', n, () => [g.pt(), g.chance(0.1) ? v(0, 0) : g.pt(), g.pt()]),
    ...repeat(g, 'dimensionOffsetAt', n, () => [dimension(g), g.pt()]),
    ...repeat(g, 'linearAngleFor', n, () => [g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'sectorArms', n, () => {
      const dir = () => {
        const t = g.num(0, TAU);
        return g.chance(0.2) ? g.pick([v(1, 0), v(0, 1), v(-1, 0)]) : v(Math.cos(t), Math.sin(t));
      };
      return [g.pt(), dir(), dir(), g.pt()];
    }),
    ...repeat(g, 'hatchLines', n, () => {
      const ring = g.chance(0.8) ? g.ring(g.int(3, 9), g.num(5, 60), g.chance(0.5)) : path(g);
      const c = ring[0] ?? v(0, 0);
      const hole = Array.from({ length: g.int(0, 5) }, () => v(c.x + g.num(-10, 10), c.y + g.num(-10, 10)));
      return [ring, g.pick([0, 45, 90, g.num(-180, 180)]), g.pick([0, -1, 1, g.num(0.2, 5)]), g.chance(0.5) ? [hole] : undefined];
    }),
    ...repeat(g, 'edgeLabels', n, () => {
      const closed = g.chance(0.6);
      const p = closed && g.chance(0.6) ? g.ring(g.int(3, 8), 30, g.chance(0.5)) : path(g);
      return [p, closed, g.num(0.2, 3), g.chance(0.5) ? undefined : g.num(0, 20), g.chance(0.5) ? bulgesFor(g, p.length) : undefined, g.pick([undefined, 'outside', 'inside'])];
    }),
  ],
};
