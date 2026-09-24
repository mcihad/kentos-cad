import { polygonHoles, polygonRing } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { repeat, type CallSet, type Gen } from '../harness';
import { entity } from './p5-entities';

/** P8: fill triangulation, rings with holes (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x0: number, y0: number, s: number) => [v(x0, y0), v(x0 + s, y0), v(x0 + s, y0 + s), v(x0, y0 + s)];
const circle = (cx: number, cy: number, r: number, n = 48) => Array.from({ length: n }, (_, i) => v(cx + r * Math.cos((i / n) * 2 * Math.PI), cy + r * Math.sin((i / n) * 2 * Math.PI)));
const O = v(0, 0);
const E = 486512.34;
const N = 4420187.52;
const tm = (pts: Vec2[]) => pts.map((p) => v(E + p.x, N + p.y));

/** An orthogonal polygon on a grid (a staircase of cells): equal x and y everywhere, flat corners, vertices on rays. */
function staircase(g: Gen): Vec2[] {
  const c = g.gridPt(1, 20);
  const steps = g.int(1, 6);
  const out: Vec2[] = [v(c.x, c.y)];
  let x = c.x;
  let y = c.y;
  for (let i = 0; i < steps; i++) {
    x += g.int(1, 4);
    out.push(v(x, y));
    y += g.int(g.chance(0.2) ? 0 : 1, 4); // a zero step leaves a flat corner
    out.push(v(x, y));
  }
  out.push(v(c.x, y));
  return g.chance(0.3) ? out.reverse() : out;
}

/** Holes of a ring: small stars or squares inside (sometimes touching, overlapping or outside), sometimes too short. */
function holesOf(g: Gen, outer: Vec2[], grid: boolean): Vec2[][] {
  let cx = 0;
  let cy = 0;
  for (const p of outer) {
    cx += p.x / outer.length;
    cy += p.y / outer.length;
  }
  const out: Vec2[][] = [];
  for (let i = g.int(0, 4); i > 0; i--) {
    if (g.chance(0.1)) out.push([v(cx, cy), v(cx + 1, cy)]);
    else if (grid) {
      const s = g.int(1, 2);
      out.push(square(Math.round(cx) + g.int(-3, 3), Math.round(cy) + g.int(-3, 3), s));
    } else {
      const r = g.num(0.2, 3);
      const n = g.int(3, 10);
      const at = v(cx + g.num(-6, 6), cy + g.num(-6, 6));
      out.push(Array.from({ length: n }, (_, k) => {
        const a = ((g.chance(0.3) ? -1 : 1) * 2 * Math.PI * (k + g.num(0.1, 0.9))) / n;
        const d = r * g.num(0.4, 1);
        return v(at.x + d * Math.cos(a), at.y + d * Math.sin(a));
      }));
    }
  }
  return out;
}

/** A polygon to fill: a star ring, a staircase or an entity's own rings (bulges tessellated, as the renderer does). */
export function fillPolygon(g: Gen): [Vec2[], Vec2[][]] {
  const k = g.int(0, 9);
  if (k < 4) {
    const outer = g.ring(g.pick([3, 4, g.int(5, 40), g.int(40, 120)]), g.num(5, 30), g.chance(0.3));
    return [outer, holesOf(g, outer, false)];
  }
  if (k < 7) {
    const outer = staircase(g);
    return [outer, holesOf(g, outer, true)];
  }
  const e = entity(g, 'polygon');
  if (e.kind !== 'polygon') return [[], []];
  return [polygonRing(e), polygonHoles(e)];
}

export const P8: CallSet = {
  file: 'calls-p8-triangulate.json',
  named: [
    { name: 'düz halka, saat yönünde', fn: 'triangulate', args: [[...square(0, 0, 10)].reverse(), [], O] },
    { name: 'kare delikli kare', fn: 'triangulate', args: [square(0, 0, 10), [square(4, 4, 2)], O] },
    { name: 'iki delik ve yuvarlak bir delik', fn: 'triangulate', args: [square(0, 0, 10), [square(1, 1, 2), square(1, 6, 2), circle(7, 7, 1.5)], O] },
    { name: 'çentiğin yanında delik (köprü üçgeninde içbükey köşe)', fn: 'triangulate', args: [[v(0, 0), v(10, 0), v(10, 10), v(6, 10), v(6, 5.5), v(5, 5.5), v(5, 10), v(0, 10)], [square(2, 4, 1)], O] },
    { name: 'TM koordinatında delikli parsel', fn: 'triangulate', args: [tm([v(0, 0), v(31.25, 0.5), v(30.75, 22.125), v(-0.5, 20)]), [tm(square(10, 8, 4))], v(E, N)] },
    { name: 'TM koordinatı, orijin başlangıçta', fn: 'triangulate', args: [tm(square(0, 0, 10)), [], O] },
    { name: 'dışarıdaki delik', fn: 'triangulate', args: [square(0, 0, 10), [square(20, 20, 2)], O] },
    { name: 'iki köşeli delik yok sayılır', fn: 'triangulate', args: [square(0, 0, 10), [[v(4, 4), v(6, 6)]], O] },
    { name: 'boş halka', fn: 'triangulate', args: [[], [], O] },
    { name: 'iki köşe', fn: 'triangulate', args: [[v(0, 0), v(1, 1)], [], O] },
    { name: 'saat yönünde üçgen', fn: 'triangulate', args: [[v(0, 0), v(0, 5), v(5, 0)], [], O] },
    { name: 'doğrusal noktalar', fn: 'triangulate', args: [[v(0, 0), v(5, 0), v(10, 0), v(7, 0)], [], O] },
    { name: 'yinelenen köşeler', fn: 'triangulate', args: [[v(0, 0), v(10, 0), v(10, 0), v(10, 10), v(0, 10), v(0, 10)], [], O] },
    { name: 'köşesi dış halkaya değen delik', fn: 'triangulate', args: [square(0, 0, 10), [[v(10, 5), v(8, 4), v(8, 6)]], O] },
    { name: 'tarak', fn: 'triangulate', args: [[v(0, 0), v(10, 0), v(10, 6), v(9, 6), v(9, 1), v(8, 1), v(8, 6), v(7, 6), v(7, 1), v(6, 1), v(6, 6), v(0, 6)], [square(1, 1, 2)], O] },
    { name: 'sağ uçları aynı iki delik', fn: 'triangulate', args: [square(0, 0, 10), [square(6, 1, 2), square(6, 6, 2), square(2, 4, 1)], O] },
    { name: 'ışın dış halkanın köşesine çarpar', fn: 'triangulate', args: [[v(0, 0), v(10, 0), v(12, 5), v(10, 10), v(0, 10)], [[v(4, 5), v(3, 4), v(3, 6)]], O] },
    { name: 'delik dış halkayla aynı yönde', fn: 'triangulate', args: [square(0, 0, 10), [[...square(3, 3, 2)].reverse()], O] },
    { name: 'iç içe iki delik', fn: 'triangulate', args: [square(0, 0, 10), [square(2, 2, 6), square(4, 4, 2)], O] },
  ],
  random: (g, n) => [
    ...repeat(g, 'triangulate', n, () => {
      const [outer, holes] = fillPolygon(g);
      return [outer, holes, g.chance(0.5) ? O : g.pt(50)];
    }),
  ],
};
