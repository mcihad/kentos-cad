import type { Vec2 } from '../../../model/geometry';
import type { Edge } from '../../../model/geom/intersect';
import type { Area, OverlayRule, Source } from '../../../model/geom/overlay';
import { areaSource, ringEdges } from '../../../model/geom/region';
import { repeat, type CallSet, type Gen } from '../harness';

/** P4: the planar overlay engine, area algebra, faces of line work, parallel lines (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const rect = (x0: number, y0: number, x1: number, y1: number): Area => ({ outer: { pts: [v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)] }, holes: [] });
const disk = (cx: number, cy: number, r: number): Area => ({ outer: { pts: [v(cx + r, cy), v(cx - r, cy)], bulges: [1, 1] }, holes: [] });
const lines = (...segs: [Vec2, Vec2][]): Source => ({ edges: segs.map(([a, b]): Edge => ({ kind: 'seg', a, b })), cut: true });
const E = 486512.34;
const N = 4420187.52;

/** The overlay's rules, by the names the core takes. */
const RULES: OverlayRule[] = ['any', 'all', 'odd', 'firstNotOthers', 'first', 'always'];

/** A rectangle, a disk or a star polygon, often on a grid so edges coincide. */
function area(g: Gen): Area {
  const k = g.int(0, 9);
  if (k < 4) {
    const a = g.gridPt(5, 4);
    const b = g.gridPt(5, 4);
    if (a.x === b.x || a.y === b.y) return rect(a.x, a.y, a.x + 5, a.y + 5);
    return rect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.max(a.x, b.x), Math.max(a.y, b.y));
  }
  if (k < 6) {
    const c = g.chance(0.5) ? g.gridPt(5, 3) : g.pt(15);
    return disk(c.x, c.y, g.pick([5, 10, g.num(1, 12)]));
  }
  const ring = g.ring(g.int(3, 8), g.num(5, 20), g.chance(0.3));
  const holed = g.chance(0.3);
  const c = ring[0];
  return {
    outer: g.chance(0.3) ? { pts: ring, bulges: ring.map(() => (g.chance(0.7) ? 0 : g.num(-0.4, 0.4))) } : { pts: ring },
    holes: holed ? [{ pts: [v(c.x, c.y), v(c.x, c.y), v(c.x, c.y)].map((p, i) => v(p.x + [-0.5, 0.5, 0][i], p.y + [-0.3, -0.3, 0.4][i])) }] : [],
  };
}

function cutLines(g: Gen): Source {
  const n = g.int(1, 4);
  return { edges: Array.from({ length: n }, (): Edge => (g.chance(0.8) ? { kind: 'seg', a: g.gridPt(5, 5), b: g.gridPt(5, 5) } : { kind: 'arc', c: g.gridPt(5, 3), r: g.pick([5, 7.5]), a0: 0, sweep: 2 * Math.PI })) };
}

export const P4: CallSet = {
  file: 'calls-p4-overlay.json',
  named: [
    { name: 'örtüşen kareler', fn: 'unionAreas', args: [[rect(0, 0, 10, 10), rect(5, 5, 15, 15)]] },
    { name: 'ortak kenarlı komşular', fn: 'unionAreas', args: [[rect(0, 0, 10, 10), rect(10, 0, 20, 10)]] },
    { name: 'T bağlantısı', fn: 'unionAreas', args: [[rect(0, 0, 10, 10), rect(10, 2, 20, 8)]] },
    { name: 'kesişim', fn: 'intersectAreas', args: [[rect(0, 0, 10, 10), rect(5, 5, 15, 15)]] },
    { name: 'fark', fn: 'subtractAreas', args: [[rect(0, 0, 10, 10)], [rect(5, 5, 15, 15)]] },
    { name: 'içte kalan kesici: delik', fn: 'subtractAreas', args: [[rect(0, 0, 10, 10)], [rect(3, 3, 7, 7)]] },
    { name: 'kenara değen kesici: çentik', fn: 'subtractAreas', args: [[rect(0, 0, 10, 10)], [rect(3, 0, 7, 4)]] },
    { name: 'dış halkaya noktada değen delik', fn: 'subtractAreas', args: [[rect(0, 0, 10, 10)], [{ outer: { pts: [v(0, 5), v(5, 3), v(5, 7)] }, holes: [] }]] },
    { name: 'köşede değen kareler', fn: 'unionAreas', args: [[rect(0, 0, 10, 10), rect(10, 10, 20, 20)]] },
    { name: 'her şeyi çıkarmak', fn: 'subtractAreas', args: [[rect(0, 0, 10, 10)], [rect(-1, -1, 11, 11)]] },
    { name: 'deliği kapatan yama', fn: 'unionAreas', args: [[{ outer: rect(0, 0, 10, 10).outer, holes: [{ pts: [v(3, 3), v(3, 7), v(7, 7), v(7, 3)] }] }, rect(2, 2, 8, 8)]] },
    { name: 'saat yönü girdi', fn: 'unionAreas', args: [[{ outer: { pts: [v(0, 0), v(0, 10), v(10, 10), v(10, 0)] }, holes: [] }, rect(5, 5, 15, 15)]] },
    { name: 'üç alan', fn: 'unionAreas', args: [[rect(0, 0, 10, 10), rect(5, 0, 15, 10), rect(0, 5, 15, 20)]] },
    { name: 'iki dairenin birleşimi', fn: 'unionAreas', args: [[disk(0, 0, 10), disk(10, 0, 10)]] },
    { name: 'mercek', fn: 'intersectAreas', args: [[disk(0, 0, 10), disk(10, 0, 10)]] },
    { name: 'kareden daire deliği', fn: 'subtractAreas', args: [[rect(-10, -10, 10, 10)], [disk(0, 0, 5)]] },
    { name: 'kenarda daire çentiği', fn: 'subtractAreas', args: [[rect(0, 0, 20, 20)], [disk(10, 0, 5)]] },
    { name: 'kirişi ortak yarım daireler', fn: 'unionAreas', args: [[{ outer: { pts: [v(10, 0), v(-10, 0)], bulges: [1, 0] }, holes: [] }, { outer: { pts: [v(-10, 0), v(10, 0)], bulges: [1, 0] }, holes: [] }]] },
    { name: 'TM köşeleri bit bit', fn: 'unionAreas', args: [[rect(E, N, E + 23.417, N + 31.466), rect(E + 23.417, N, E + 40.001, N + 31.466)]] },
    { name: 'TM kesişim noktası', fn: 'intersectAreas', args: [[{ outer: { pts: [v(E, N), v(E + 30.123, N + 0.5), v(E + 29.9, N + 30.77), v(E + 0.2, N + 30.1)] }, holes: [] }, rect(E + 10.5, N - 5, E + 50, N + 12.25)]] },
    { name: 'kareyi bölen çizgi', fn: 'splitArea', args: [rect(0, 0, 10, 10), lines([v(-1, 5), v(11, 5)])] },
    { name: 'içeride biten çizgi bölmez', fn: 'splitArea', args: [rect(0, 0, 10, 10), lines([v(-1, 5), v(5, 5)])] },
    { name: 'zikzak ve kesişen çizgiler', fn: 'splitArea', args: [rect(0, 0, 10, 10), lines([v(-1, 2), v(5, 8)], [v(5, 8), v(11, 2)], [v(5, -1), v(5, 11)])] },
    { name: 'delikli alanı bölmek', fn: 'splitArea', args: [{ outer: rect(0, 0, 10, 10).outer, holes: [{ pts: [v(2, 2), v(2, 4), v(4, 4), v(4, 2)] }] }, lines([v(5, -1), v(5, 11)])] },
    { name: 'taşan çizgilerin yüzü, ada delik', fn: 'faceAt', args: [[lines([v(-1, 0), v(11, 0)], [v(10, -1), v(10, 11)], [v(11, 10), v(-1, 10)], [v(0, 11), v(0, -1)]), lines([v(4, 4), v(6, 4)], [v(6, 4), v(6, 6)], [v(6, 6), v(4, 6)], [v(4, 6), v(4, 4)])], v(2, 2), undefined] },
    { name: 'adasız yüz', fn: 'faceAt', args: [[lines([v(0, 0), v(10, 0)], [v(10, 0), v(10, 10)], [v(10, 10), v(0, 10)], [v(0, 10), v(0, 0)]), lines([v(4, 4), v(6, 4)], [v(6, 4), v(6, 6)], [v(6, 6), v(4, 6)], [v(4, 6), v(4, 4)])], v(2, 2), false] },
    { name: 'sarkan çizgi yok sayılır', fn: 'faceAt', args: [[lines([v(0, 0), v(10, 0)], [v(10, 0), v(10, 10)], [v(10, 10), v(0, 10)], [v(0, 10), v(0, 0)], [v(5, 5), v(7, 6)])], v(2, 2), undefined] },
    { name: 'dışarıda yüz yok', fn: 'faceAt', args: [[lines([v(0, 0), v(10, 0)], [v(10, 0), v(10, 10)], [v(10, 10), v(0, 10)], [v(0, 10), v(0, 0)])], v(20, 2), undefined] },
    { name: 'çizgi ızgarasının yüzleri', fn: 'allFaces', args: [[lines(...[0, 5, 10].map((x): [Vec2, Vec2] => [v(x, -1), v(x, 11)]), ...[0, 5, 10].map((y): [Vec2, Vec2] => [v(-1, y), v(11, y)]))]] },
    { name: 'daireyi bölen çizgi', fn: 'allFaces', args: [[{ edges: [{ kind: 'arc', c: v(0, 0), r: 1, a0: 0, sweep: 2 * Math.PI }], cut: true }, lines([v(-2, 0), v(2, 0)])]] },
    { name: 'kural adıyla bindirme: tek sayı', fn: 'overlay', args: [[areaSource([rect(0, 0, 10, 10)]), areaSource([rect(5, 5, 15, 15)])], 'odd'] },
    { name: 'kural adıyla bindirme: bölme', fn: 'overlay', args: [[areaSource([disk(0, 0, 10)]), { edges: [{ kind: 'seg', a: v(-12, 1), b: v(12, 1) }], cut: true }], 'first'] },
    { name: 'dairenin sarım sayısı', fn: 'winding', args: [ringEdges(disk(0, 0, 10).outer), v(1, 1)] },
    { name: 'kapalı eksenin koridoru', fn: 'corridorArea', args: [[v(0, 0), v(20, 0), v(20, 20), v(0, 20)], 1, 2, true] },
    { name: 'tek yanlı paralel', fn: 'parallelSides', args: [[v(0, 0), v(10, 0), v(10, 10)], 0, 2, false] },
    { name: 'kısa eksen', fn: 'parallelSides', args: [[v(0, 0), v(0, 0)], 1, 1, false] },
    { name: 'tekrar eden ve kapanan eksen', fn: 'cleanAxis', args: [[v(0, 0), v(0, 0), v(5, 0), v(5, 5), v(0, 0)], true] },
  ],
  random: (g, n) => [
    ...repeat(g, 'ringArea', n, () => [area(g).outer]),
    ...repeat(g, 'ringEdges', n, () => [area(g).outer]),
    ...repeat(g, 'orientRing', n, () => [area(g).outer, g.chance(0.5)]),
    ...repeat(g, 'netArea', n, () => [area(g)]),
    ...repeat(g, 'insideArea', n, () => [area(g), g.chance(0.5) ? g.gridPt(2.5, 8) : g.pt(25)]),
    ...repeat(g, 'areaSource', n, () => [Array.from({ length: g.int(1, 3) }, () => area(g))]),
    ...repeat(g, 'unionAreas', n, () => [Array.from({ length: g.int(0, 4) }, () => area(g))]),
    ...repeat(g, 'intersectAreas', n, () => [Array.from({ length: g.int(1, 3) }, () => area(g))]),
    ...repeat(g, 'subtractAreas', n, () => [Array.from({ length: g.int(1, 2) }, () => area(g)), Array.from({ length: g.int(0, 3) }, () => area(g))]),
    ...repeat(g, 'splitArea', n, () => [area(g), cutLines(g)]),
    ...repeat(g, 'faceAt', n, () => [[cutLines(g), cutLines(g), { ...areaSource([area(g)]), cut: true }], g.gridPt(2.5, 8), g.chance(0.5) ? undefined : g.chance(0.5)]),
    ...repeat(g, 'allFaces', n, () => [[cutLines(g), cutLines(g), { ...areaSource([area(g)]), cut: true }]]),
    ...repeat(g, 'overlay', n, () => [Array.from({ length: g.int(1, 3) }, () => (g.chance(0.8) ? areaSource([area(g)]) : { ...cutLines(g), cut: true })), g.pick(RULES)]),
    ...repeat(g, 'faceRings', n, () => [[cutLines(g), { ...areaSource([area(g)]), cut: true }]]),
    ...repeat(g, 'winding', n, () => [ringEdges(area(g).outer), g.chance(0.5) ? g.gridPt(2.5, 8) : g.pt(25)]),
    ...repeat(g, 'cleanAxis', n, () => [Array.from({ length: g.int(0, 6) }, () => g.gridPt(5, 2)), g.chance(0.5)]),
    ...repeat(g, 'parallelSides', n, () => [g.pts(g.int(0, 6), 30), g.pick([0, g.num(0, 5)]), g.pick([0, g.num(0, 5)]), g.chance(0.5)]),
    ...repeat(g, 'corridorArea', n, () => {
      const closed = g.chance(0.5);
      return [closed ? g.ring(g.int(3, 7), 30) : g.pts(g.int(0, 6), 30), g.pick([0, g.num(0, 5)]), g.pick([0, g.num(0, 5)]), closed];
    }),
  ],
};
