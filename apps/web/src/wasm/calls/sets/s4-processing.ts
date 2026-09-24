import type { Entity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import type { NumberingInput } from '../../../processing/builtin/numbering';
import type { CornerWalk } from '../../../processing/geometry';
import { repeat, type CallSet, type Gen } from '../harness';
import { entity } from './p5-entities';

/**
 * S4: what the processing tools compute (docs/adr/0008): the order a ring's
 * corners are numbered in, one number per location across shapes (shared
 * corners, existing points), where the text beside a corner goes, and
 * edge-length labels with the shared-edge test. Grids of parcels make
 * shared corners and edges; TM coordinates, holes, arcs and empty rings are
 * among the named cases.
 */

const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number, s: number) => [v(x, y), v(x + s, y), v(x + s, y + s), v(x, y + s)];
const E = 486512.34;
const N = 4420187.52;
const tm = (pts: Vec2[]) => pts.map((p) => v(E + p.x, N + p.y));
const closed = (pts: Vec2[]) => ({ pts, closed: true });
const shape = (...rings: { pts: Vec2[]; closed: boolean }[]): NumberingInput => ({ rings });

const DIRS: CornerWalk['dir'][] = ['cw', 'ccw'];
const STARTS: CornerWalk['start'][] = ['northwest', 'north', 'first', 'point'];
const walk = (w: Partial<CornerWalk> = {}): CornerWalk => ({ dir: 'cw', start: 'northwest', point: null, tolerance: 0.001, shared: true, ...w });

/** A parcel grid in the call's frame: squares of 10 or 20 on a 10 m grid (shared corners and edges), some counter-clockwise, some with a hole. */
export function parcelRings(g: Gen, n: number): Vec2[][] {
  return Array.from({ length: n }, () => {
    const c = g.gridPt(10, 3);
    const s = g.pick([10, 20]);
    const ring = square(c.x, c.y, s);
    return g.chance(0.3) ? ring.reverse() : ring;
  });
}

/** Shapes to number: grid parcels (some with a hole), star rings, open paths, now and then an empty or tiny ring. */
function shapes(g: Gen): NumberingInput[] {
  const out: NumberingInput[] = [];
  for (let i = g.int(1, 6); i > 0; i--) {
    const k = g.int(0, 9);
    if (k < 5) {
      const [ring] = parcelRings(g, 1);
      const c = ring[0];
      out.push(g.chance(0.2) ? shape(closed(ring), closed(square(c.x + 2, c.y + 2, 3))) : shape(closed(ring)));
    } else if (k < 7) out.push(shape(closed(g.ring(g.int(3, 9), g.num(3, 30), g.chance(0.4)))));
    else if (k < 9) out.push(shape({ pts: Array.from({ length: g.int(1, 5) }, () => g.gridPt(10, 3)), closed: false }));
    else out.push(shape({ pts: g.pick([[], [g.gridPt(10, 3)], [g.gridPt(10, 3), g.gridPt(10, 3)]]), closed: g.chance(0.5) }));
  }
  return out;
}

/** Existing points: corners of the shapes (exactly, or moved a little) and points anywhere. */
function existingFor(g: Gen, inputs: NumberingInput[]): Vec2[] {
  const corners = inputs.flatMap((s) => s.rings.flatMap((r) => r.pts));
  return Array.from({ length: g.int(0, 4) }, () => {
    if (!corners.length || g.chance(0.2)) return g.gridPt(10, 3);
    const p = g.pick(corners);
    const j = g.pick([0, 0, 0.0004, 0.002, 0.3]);
    return v(p.x + g.num(-j, j), p.y + g.num(-j, j));
  });
}

function walkFor(g: Gen): CornerWalk {
  return { dir: g.pick(DIRS), start: g.pick(STARTS), point: g.chance(0.8) ? g.gridPt(10, 4) : null, tolerance: g.pick([0, 0.001, 0.001, 0.5, 5]), shared: g.chance(0.8) };
}

/** Objects whose edges are labelled: grid parcels (sharing edges), lines along grid edges, paths with arcs, and other kinds. */
export function labelledObjects(g: Gen, n: number): Entity[] {
  let id = 0;
  return Array.from({ length: n }, (): Entity => {
    id += 1;
    const k = g.int(0, 9);
    if (k < 4) {
      const [pts] = parcelRings(g, 1);
      const bulged = g.chance(0.15);
      return { id, layerId: 'a', attrs: {}, kind: 'polygon', pts, ...(bulged ? { bulges: [0, g.num(-1, 1), 0, 0] } : {}) };
    }
    if (k < 6) {
      const a = g.gridPt(10, 3);
      const b = g.chance(0.5) ? v(a.x + g.pick([-10, 10, 20]), a.y) : v(a.x, a.y + g.pick([-10, 10]));
      return { id, layerId: 'a', attrs: {}, kind: 'line', a, b: g.chance(0.1) ? v(b.x + 0.0004, b.y) : b };
    }
    return { ...entity(g, g.pick(['polyline', 'polygon', 'polyline', 'circle', 'point', 'text'] as const)), id };
  });
}

export const S4: CallSet = {
  file: 'calls-s4-processing.json',
  named: [
    { name: 'kare, kuzeybatıdan saat yönünde', fn: 'ringOrder', args: [square(0, 0, 10), true, 'cw', 'northwest', null] },
    { name: 'kare, kuzeybatıdan saat yönünün tersine', fn: 'ringOrder', args: [square(0, 0, 10), true, 'ccw', 'northwest', null] },
    { name: 'saat yönündeki kare aynı yönde', fn: 'ringOrder', args: [[...square(0, 0, 10)].reverse(), true, 'cw', 'northwest', null] },
    { name: 'ilk köşeden', fn: 'ringOrder', args: [square(0, 0, 10), true, 'cw', 'first', null] },
    { name: 'noktaya en yakından', fn: 'ringOrder', args: [square(0, 0, 10), true, 'cw', 'point', v(11, -1)] },
    { name: 'noktasız en yakın: ilk köşe', fn: 'ringOrder', args: [square(0, 0, 10), true, 'ccw', 'point', null] },
    { name: 'açık yol iyi ucundan', fn: 'ringOrder', args: [[v(0, 0), v(10, 0), v(10, 10)], false, 'cw', 'north', null] },
    { name: 'açık yol ilk köşeden', fn: 'ringOrder', args: [[v(0, 10), v(10, 0), v(10, 10)], false, 'cw', 'first', null] },
    { name: 'boş halka saat yönünün tersine', fn: 'ringOrder', args: [[], true, 'ccw', 'northwest', null] },
    { name: 'tek köşe', fn: 'ringOrder', args: [[v(3, 4)], true, 'ccw', 'north', null] },
    { name: 'iki köşe', fn: 'ringOrder', args: [[v(0, 0), v(5, 5)], true, 'ccw', 'northwest', null] },
    { name: 'TM parseli, kuzeybatı', fn: 'ringOrder', args: [tm([v(0, 0), v(23.417, 1.203), v(25.881, 31.466), v(2.004, 33.012)]), true, 'cw', 'northwest', null] },
    { name: 'eşit kuzeyde batıdaki', fn: 'ringOrder', args: [[v(0, 0), v(10, 10), v(0, 10), v(-10, 0)], true, 'cw', 'north', null] },
    { name: 'komşu iki parsel ortak köşeli', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10))), shape(closed(square(10, 0, 10)))], walk(), []] },
    { name: 'ortak köşesiz', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10))), shape(closed(square(10, 0, 10)))], walk({ shared: false }), []] },
    { name: 'var olan nokta numarasını korur', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10)))], walk(), [v(0, 10), v(50, 50)]] },
    { name: 'tolerans içindeki var olan nokta', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10)))], walk({ tolerance: 0.01 }), [v(10.005, 10.004)]] },
    { name: 'toleransın dışındaki', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10)))], walk({ tolerance: 0.001 }), [v(10.005, 10.004)]] },
    { name: 'delikli parsel: önce dış halka', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10)), closed(square(3, 3, 2)))], walk({ dir: 'ccw' }), []] },
    { name: 'açık yol ve parsel ortak uçlu', fn: 'numberCorners', args: [[shape({ pts: [v(0, 20), v(0, 10), v(10, 10)], closed: false }), shape(closed(square(0, 0, 10)))], walk(), []] },
    { name: 'boş halka ve saat yönünün tersine', fn: 'numberCorners', args: [[shape(closed([])), shape(closed(square(0, 0, 10)))], walk({ dir: 'ccw' }), []] },
    { name: 'noktaya en yakın parsel önce', fn: 'numberCorners', args: [[shape(closed(square(0, 0, 10))), shape(closed(square(40, 0, 10)))], walk({ start: 'point', point: v(45, -3) }), []] },
    { name: 'ilk çizilen sırasıyla', fn: 'numberCorners', args: [[shape(closed(square(40, 40, 10))), shape(closed(square(0, 0, 10)))], walk({ start: 'first' }), []] },
    { name: 'TM ızgarasında dokuz parsel', fn: 'numberCorners', args: [[0, 1, 2].flatMap((i) => [0, 1, 2].map((j) => shape(closed(tm(square(10 * i, 10 * j, 10)))))), walk(), [v(E + 10, N + 10)]] },
    { name: 'başlangıçtan çok uzakta toleranssız', fn: 'numberCorners', args: [[shape(closed(square(1e10, 0, 10))), shape(closed(square(1e10 + 10, 0, 10)))], walk({ tolerance: 0 }), []] },
    { name: 'eksi koordinatta ızgara sınırı', fn: 'numberCorners', args: [[shape(closed(square(-10, -10, 10))), shape(closed(square(0, -10, 10)))], walk({ tolerance: 0.5 }), [v(-0.0001, 0.0001)]] },
    { name: 'dışarısı güneybatı', fn: 'cornerTextAt', args: [{ p: v(0, 0), out: v(-Math.SQRT1_2, -Math.SQRT1_2) }, 6, 2] },
    { name: 'dışarısı kuzeydoğu', fn: 'cornerTextAt', args: [{ p: v(E, N), out: v(Math.SQRT1_2, Math.SQRT1_2) }, 6, 2] },
    { name: 'yukarı, boş ad', fn: 'cornerTextAt', args: [{ p: v(1, 1), out: v(0, 1) }, 0, 0.5] },
    { name: 'batı', fn: 'cornerTextAt', args: [{ p: v(1, 1), out: v(-1, 0) }, 7, 3] },
    {
      name: 'komşu parsellerin ortak kenarı bir kez',
      fn: 'edgeLengthLabels',
      args: [[{ id: 1, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10) }, { id: 2, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(10, 0, 10) }], 2, 0, 'outside', true],
    },
    {
      name: 'ters yönde ve milimetre kaymış çizgi',
      fn: 'edgeLengthLabels',
      args: [[{ id: 1, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10) }, { id: 7, layerId: 'a', attrs: {}, kind: 'line', a: v(10.0004, 10), b: v(10, 0) }], 2, 0, 'outside', true],
    },
    {
      name: 'ortak kenar ayrı ayrı',
      fn: 'edgeLengthLabels',
      args: [[{ id: 1, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10) }, { id: 2, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(10, 0, 10) }], 2, 0, 'inside', false],
    },
    {
      name: 'yaylı parsel ve en kısa kenar',
      fn: 'edgeLengthLabels',
      args: [[{ id: 3, layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0, 10), bulges: [0, 0.5, 0, 0] }, { id: 4, layerId: 'a', attrs: {}, kind: 'polyline', pts: [v(0, 0), v(3, 0), v(3, 30)] }], 1.5, 5, 'outside', true],
    },
    {
      name: 'TM parselleri',
      fn: 'edgeLengthLabels',
      args: [[{ id: 1, layerId: 'a', attrs: {}, kind: 'polygon', pts: tm(square(0, 0, 10)) }, { id: 2, layerId: 'a', attrs: {}, kind: 'polygon', pts: tm([...square(10, 0, 10)].reverse()) }], 2, 0, 'outside', true],
    },
    {
      name: 'başka türler ve sıfır uzunluklu çizgi',
      fn: 'edgeLengthLabels',
      args: [[{ id: 5, layerId: 'a', attrs: {}, kind: 'circle', c: v(0, 0), r: 5 }, { id: 6, layerId: 'a', attrs: {}, kind: 'line', a: v(1, 1), b: v(1, 1) }, { id: 8, layerId: 'a', attrs: {}, kind: 'point', p: v(2, 2) }], 2, 0, 'outside', true],
    },
    {
      name: 'sıfır çevresinde milimetre ızgarası',
      fn: 'edgeLengthLabels',
      args: [[{ id: 1, layerId: 'a', attrs: {}, kind: 'line', a: v(-0.0004, 0), b: v(0, 10) }, { id: 2, layerId: 'a', attrs: {}, kind: 'line', a: v(0, 10), b: v(0.0004, 0) }], 2, 0, 'outside', true],
    },
  ],
  random: (g, n) => [
    ...repeat(g, 'ringOrder', n, () => {
      const pts = g.chance(0.6) ? g.ring(g.int(0, 12), g.num(3, 40), g.chance(0.5)) : Array.from({ length: g.int(0, 6) }, () => g.gridPt(10, 3));
      return [pts, g.chance(0.8), g.pick(DIRS), g.pick(STARTS), g.chance(0.8) ? g.pt(60) : null];
    }),
    ...repeat(g, 'numberCorners', n, () => {
      const inputs = shapes(g);
      return [inputs, walkFor(g), existingFor(g, inputs)];
    }),
    ...repeat(g, 'cornerTextAt', n, () => {
      const t = g.num(0, 2 * Math.PI);
      return [{ p: g.pt(), out: g.chance(0.1) ? v(0, 1) : v(Math.cos(t), Math.sin(t)) }, g.int(0, 12), g.pick([0.5, 2, g.num(0.1, 10)])];
    }),
    ...repeat(g, 'edgeLengthLabels', n, () => [labelledObjects(g, g.int(1, 8)), g.pick([1, 2, g.num(0.1, 5)]), g.pick([0, 0, 5, 15]), g.pick(['outside', 'inside']), g.chance(0.8)]),
  ],
};
