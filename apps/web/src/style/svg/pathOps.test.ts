import { describe, expect, it } from 'vitest';
import { flattenSubPathTol, ringSignedArea, segmentCount, windingOf } from './bezier';
import { booleanOp, cutPath, type RegionInput } from './pathBool';
import { applyResult, booleanShapes, breakApart, closeSubPath, combineShapes, offsetShapes, openSubPath, simplifySubPath, strokeToPath } from './pathOps';
import { offsetRegion, strokeOutline } from './pathStroke';
import type { SubPath } from './pathData';
import { toPath, type SvgShape } from './svgModel';

const base = { fill: 'fill', stroke: 'none', strokeWidth: 1 } as const;
const rect = (id: string, x: number, y: number, w: number, h: number): SvgShape => ({ ...base, id, kind: 'rect', x, y, w, h });
const circle = (id: string, cx: number, cy: number, r: number): SvgShape => ({ ...base, id, kind: 'ellipse', cx, cy, rx: r, ry: r });
const subsOf = (s: SvgShape) => (toPath(s) as Extract<SvgShape, { kind: 'path' }>).subs;
const region = (s: SvgShape): RegionInput => ({ subs: subsOf(s), fillRule: 'nonzero' });

/** Filled area of sub-paths under the even-odd rule (flattened finely). */
function areaOf(subs: readonly SubPath[]): number {
  const rings = subs.map((sp) => flattenSubPathTol(sp, 1e-4));
  // Sum of signed ring areas, each counted with the sign its nesting gives.
  let total = 0;
  rings.forEach((r, i) => {
    const depth = rings.filter((o, j) => j !== i && windingOf(o, r[0]) !== 0).length;
    total += (depth % 2 ? -1 : 1) * Math.abs(ringSignedArea(r));
  });
  return total;
}

/** Whether p is painted (even-odd). */
const inside = (subs: readonly SubPath[], x: number, y: number) => subs.reduce((w, sp) => w + windingOf(flattenSubPathTol(sp, 1e-4), [x, y]), 0) % 2 !== 0;

const nodes = (subs: readonly SubPath[]) => subs.reduce((k, sp) => k + sp.nodes.length, 0);

describe('booleans on straight shapes', () => {
  it('unions, intersects, subtracts and excludes overlapping squares', () => {
    const a = region(rect('a', 0, 0, 10, 10));
    const b = region(rect('b', 5, 5, 10, 10));
    const [u] = booleanOp('union', [a, b]);
    expect(areaOf(u)).toBeCloseTo(175, 6);
    expect(nodes(u)).toBe(8);
    const [i] = booleanOp('intersection', [a, b]);
    expect(areaOf(i)).toBeCloseTo(25, 6);
    const [d] = booleanOp('difference', [a, b]);
    expect(areaOf(d)).toBeCloseTo(75, 6);
    expect(inside(d, 2, 2) && !inside(d, 7, 7)).toBe(true);
    const [x] = booleanOp('exclusion', [a, b]);
    expect(areaOf(x)).toBeCloseTo(150, 6);
    expect(inside(x, 7, 7)).toBe(false);
  });

  it('keeps shared corners of touching squares and merges them into one outline', () => {
    const [u] = booleanOp('union', [region(rect('a', 0, 0, 10, 10)), region(rect('b', 10, 0, 10, 10))]);
    expect(u).toHaveLength(1);
    expect(areaOf(u)).toBeCloseTo(200, 6);
    // The shared edge's corners stay (they are corners of both squares).
    expect(u[0].nodes.some((n) => n.x === 10 && n.y === 0)).toBe(true);
  });

  it('gives a hole when a square is taken out of a bigger one', () => {
    const [d] = booleanOp('difference', [region(rect('a', 0, 0, 20, 20)), region(rect('b', 5, 5, 10, 10))]);
    expect(d).toHaveLength(2);
    expect(areaOf(d)).toBeCloseTo(300, 6);
    expect(inside(d, 10, 10)).toBe(false);
  });

  it('returns nothing for disjoint intersections', () => {
    const [i] = booleanOp('intersection', [region(rect('a', 0, 0, 5, 5)), region(rect('b', 10, 10, 5, 5))]);
    expect(i).toHaveLength(0);
  });

  it('divides the bottom shape along a line and along a closed shape', () => {
    const line: RegionInput = {
      subs: [
        {
          closed: false,
          nodes: [
            { x: 5, y: -5 },
            { x: 5, y: 15 },
          ],
        },
      ],
      fillRule: 'nonzero',
    };
    const pieces = booleanOp('division', [region(rect('a', 0, 0, 10, 10)), line]);
    expect(pieces).toHaveLength(2);
    expect(pieces.map(areaOf).sort()).toEqual([50, 50].map((v) => expect.closeTo(v, 6)));
    const byDisc = booleanOp('division', [region(rect('a', 0, 0, 10, 10)), region(circle('c', 10, 10, 5))]);
    expect(byDisc).toHaveLength(2);
    expect(byDisc.map(areaOf).reduce((s, v) => s + v, 0)).toBeCloseTo(100, 3);
  });
});

describe('booleans keep curves', () => {
  it('unions two discs into curves with the crossings as new nodes', () => {
    const [u] = booleanOp('union', [region(circle('a', 0, 0, 10)), region(circle('b', 10, 0, 10))]);
    expect(u).toHaveLength(1);
    // Each disc is 4 cubics; the union keeps 3 whole ones and 2 cut ones each: few nodes, not a polyline.
    expect(u[0].nodes.length).toBeLessThanOrEqual(10);
    expect(u[0].nodes.filter((n) => n.in || n.out).length).toBe(u[0].nodes.length);
    // Two unit-circle lenses overlap by r²(2π/3 − √3/2) each side: area = 2πr² − overlap.
    const r = 10;
    const lens = 2 * r * r * Math.acos(0.5) - (r / 2) * Math.sqrt(4 * r * r - r * r);
    // The cubic circle is 0.03 % off a true one; compare against the cubic discs.
    const disc = areaOf(subsOf(circle('a', 0, 0, 10)));
    expect(areaOf(u)).toBeCloseTo(2 * disc - lens, 0);
    // The crossings sit on both circles.
    const cross = u[0].nodes.filter((n) => Math.abs(n.x - 5) < 1e-3);
    expect(cross).toHaveLength(2);
    for (const n of cross) expect(Math.abs(Math.hypot(n.x, n.y) - 10)).toBeLessThan(0.02);
  });

  it('cuts a disc out of a square, the hole a curve', () => {
    const [d] = booleanOp('difference', [region(rect('a', -20, -20, 40, 40)), region(circle('c', 0, 0, 10))]);
    expect(d).toHaveLength(2);
    const hole = d.find((sp) => sp.nodes.length === 4 && sp.nodes.every((n) => n.in && n.out));
    expect(hole).toBeTruthy();
    expect(inside(d, 0, 0)).toBe(false);
    expect(inside(d, 15, 15)).toBe(true);
  });

  it('respects even-odd sub-paths (a ring shape) and self-crossing stars', () => {
    const ring: RegionInput = { subs: [...subsOf(circle('o', 0, 0, 10)), ...subsOf(circle('i', 0, 0, 5))], fillRule: 'evenodd' };
    const [u] = booleanOp('union', [ring, region(rect('r', 20, 20, 1, 1))]);
    expect(inside(u, 0, 0)).toBe(false);
    expect(inside(u, 7, 0)).toBe(true);
    // A pentagram drawn in one stroke: its centre is empty under even-odd, filled under nonzero.
    const star: SubPath = { closed: true, nodes: [0, 2, 4, 1, 3].map((k) => ({ x: 10 * Math.cos(-Math.PI / 2 + (k * 2 * Math.PI) / 5), y: 10 * Math.sin(-Math.PI / 2 + (k * 2 * Math.PI) / 5) })) };
    const far = region(rect('f', 50, 50, 1, 1));
    const [eo] = booleanOp('union', [{ subs: [star], fillRule: 'evenodd' }, far]);
    const [nz] = booleanOp('union', [{ subs: [star], fillRule: 'nonzero' }, far]);
    expect(inside(eo, 0, 0)).toBe(false);
    expect(inside(nz, 0, 0)).toBe(true);
    expect(inside(eo, 0, -7)).toBe(true);
  });
});

describe('cut path', () => {
  it('cuts a square outline where a line crosses it into open pieces', () => {
    const cut = cutPath(subsOf(rect('a', 0, 0, 10, 10)), [
      [
        {
          closed: false,
          nodes: [
            { x: 5, y: -5 },
            { x: 5, y: 15 },
          ],
        },
      ],
    ]);
    expect(cut).toHaveLength(2);
    expect(cut.every((sp) => !sp.closed)).toBe(true);
    const len = (sp: SubPath) => sp.nodes.slice(1).reduce((s, n, i) => s + Math.hypot(n.x - sp.nodes[i].x, n.y - sp.nodes[i].y), 0);
    expect(cut.map(len).reduce((a, b) => a + b, 0)).toBeCloseTo(40, 9);
  });

  it('splits curves exactly at the crossing', () => {
    const cut = cutPath(subsOf(circle('c', 0, 0, 10)), [
      [
        {
          closed: false,
          nodes: [
            { x: 0, y: -20 },
            { x: 0, y: 20 },
          ],
        },
      ],
    ]);
    expect(cut).toHaveLength(2);
    for (const sp of cut)
      for (const n of [sp.nodes[0], sp.nodes[sp.nodes.length - 1]]) {
        expect(Math.abs(n.x)).toBeLessThan(1e-9);
        expect(Math.abs(Math.abs(n.y) - 10)).toBeLessThan(1e-9);
      }
  });

  it('leaves a sub-path nothing crosses as it is', () => {
    const sq = subsOf(rect('a', 0, 0, 10, 10));
    expect(cutPath(sq, [subsOf(rect('b', 20, 20, 1, 1))])).toEqual(sq);
  });
});

describe('stroke to path and offsets', () => {
  it('outlines an open line with butt, square and round caps', () => {
    const line: SubPath[] = [
      {
        closed: false,
        nodes: [
          { x: 0, y: 0 },
          { x: 10, y: 0 },
        ],
      },
    ];
    expect(areaOf(strokeOutline(line, { width: 2, cap: 'butt', join: 'miter' }))).toBeCloseTo(20, 6);
    expect(areaOf(strokeOutline(line, { width: 2, cap: 'square', join: 'miter' }))).toBeCloseTo(24, 6);
    // Round caps are cubic arcs: 0.03 % off a true circle.
    expect(areaOf(strokeOutline(line, { width: 2, cap: 'round', join: 'miter' }))).toBeCloseTo(20 + Math.PI, 2);
  });

  it('mitres, bevels and rounds a right-angle corner', () => {
    const L: SubPath[] = [
      {
        closed: false,
        nodes: [
          { x: 0, y: 0 },
          { x: 10, y: 0 },
          { x: 10, y: 10 },
        ],
      },
    ];
    const miter = areaOf(strokeOutline(L, { width: 2, cap: 'butt', join: 'miter' }));
    const bevel = areaOf(strokeOutline(L, { width: 2, cap: 'butt', join: 'bevel' }));
    const round = areaOf(strokeOutline(L, { width: 2, cap: 'butt', join: 'round' }));
    // Two 10×2 bands overlapping 1×1 at the inner corner, plus the outer join.
    expect(miter).toBeCloseTo(40 - 1 + 1, 6);
    expect(bevel).toBeCloseTo(40 - 1 + 0.5, 6);
    expect(round).toBeCloseTo(40 - 1 + Math.PI / 4, 3);
  });

  it('turns a closed square outline into a ring with a hole', () => {
    const out = strokeOutline(subsOf(rect('a', 0, 0, 10, 10)), { width: 2, cap: 'butt', join: 'miter' });
    expect(out).toHaveLength(2);
    expect(areaOf(out)).toBeCloseTo(144 - 64, 6);
  });

  it('dashes the stroke first', () => {
    const line: SubPath[] = [
      {
        closed: false,
        nodes: [
          { x: 0, y: 0 },
          { x: 10, y: 0 },
        ],
      },
    ];
    const out = strokeOutline(line, { width: 1, cap: 'butt', join: 'miter', dash: [2, 3] });
    // Dashes at 0–2 and 5–7.
    expect(out).toHaveLength(2);
    expect(areaOf(out)).toBeCloseTo(4, 6);
  });

  it('outlines a curve with few fitted nodes', () => {
    const out = strokeOutline(subsOf(circle('c', 0, 0, 10)), { width: 2, cap: 'butt', join: 'round' });
    expect(out).toHaveLength(2);
    expect(nodes(out)).toBeLessThanOrEqual(14);
    expect(areaOf(out)).toBeCloseTo(Math.PI * (121 - 81), 0);
  });

  it('grows a square with round corners and shrinks it with sharp ones', () => {
    const sq = region(rect('a', 0, 0, 10, 10));
    const grown = offsetRegion(sq, 1, 'round');
    expect(areaOf(grown)).toBeCloseTo(100 + 40 + Math.PI, 2);
    const mitred = offsetRegion(sq, 1, 'miter');
    expect(areaOf(mitred)).toBeCloseTo(144, 4);
    const shrunk = offsetRegion(sq, -2, 'round');
    expect(areaOf(shrunk)).toBeCloseTo(36, 4);
    expect(offsetRegion(sq, -6, 'round')).toHaveLength(0);
  });

  it('offsets a disc to a disc', () => {
    const out = offsetRegion(region(circle('c', 0, 0, 10)), 2, 'round');
    expect(out).toHaveLength(1);
    for (const n of out[0].nodes) expect(Math.hypot(n.x, n.y)).toBeCloseTo(12, 1);
  });
});

describe('shape operations', () => {
  it('unions shapes into one path with the bottom style, in the bottom place', () => {
    const a = { ...rect('a', 0, 0, 10, 10), fill: '#FF0000' };
    const b = rect('b', 5, 5, 10, 10);
    const t: SvgShape = { ...base, id: 't', kind: 'text', x: 0, y: 0, text: 'A', size: 5, weight: 400, font: 'sans', anchor: 'start' };
    const r = booleanShapes('union', [a, b, t]);
    if ('error' in r) throw new Error(r.error);
    expect(r.add).toHaveLength(1);
    expect(r.add[0].fill).toBe('#FF0000');
    expect(r.remove).toEqual(['a', 'b']);
    expect(r.note).toContain('Yazı');
    const list = applyResult([rect('z', 0, 0, 1, 1), a, rect('m', 0, 0, 1, 1), b, t], r);
    expect(list.map((s) => s.id)).toEqual(['z', r.add[0].id, 'm', 't']);
    expect('error' in booleanShapes('difference', [a])).toBe(true);
  });

  it('combines and breaks apart, keeping holes with their outline when asked', () => {
    const c = combineShapes([rect('a', 0, 0, 10, 10), rect('b', 2, 2, 6, 6), rect('c', 20, 0, 5, 5)]);
    if ('error' in c) throw new Error(c.error);
    const path = c.add[0];
    expect(path.kind === 'path' && path.subs).toHaveLength(3);
    const all = breakApart(path, false);
    const kept = breakApart(path, true);
    if ('error' in all || 'error' in kept) throw new Error('break');
    expect(all.add).toHaveLength(3);
    expect(kept.add).toHaveLength(2);
    expect(kept.add.map((s) => (s.kind === 'path' ? s.subs.length : 0)).sort()).toEqual([1, 2]);
  });

  it('converts a stroked filled shape into fill plus outline, grouped', () => {
    const r = strokeToPath([{ ...rect('a', 0, 0, 10, 10), stroke: 'stroke', strokeWidth: 2 }]);
    if ('error' in r) throw new Error(r.error);
    expect(r.add).toHaveLength(2);
    expect(r.add[0].stroke).toBe('none');
    expect(r.add[1].fill).toBe('stroke');
    expect(r.add[0].group).toBeTruthy();
    expect(r.add[0].group).toBe(r.add[1].group);
    expect('error' in strokeToPath([rect('b', 0, 0, 1, 1)])).toBe(true);
  });

  it('insets shapes and reports the ones that vanish', () => {
    const r = offsetShapes([rect('a', 0, 0, 10, 10), rect('b', 20, 0, 2, 2)], -2);
    if ('error' in r) throw new Error(r.error);
    expect(r.add).toHaveLength(1);
    expect(r.add[0].id).toBe('a');
    expect(r.note).toContain('1 şekil');
  });
});

describe('simplify, close and open', () => {
  it('fits a densely sampled circle with a handful of nodes', () => {
    const pts = Array.from({ length: 200 }, (_, i) => ({ x: 10 * Math.cos((i / 200) * 2 * Math.PI), y: 10 * Math.sin((i / 200) * 2 * Math.PI) }));
    const out = simplifySubPath({ closed: true, nodes: pts }, 0.05);
    expect(out.nodes.length).toBeLessThanOrEqual(8);
    for (const p of flattenSubPathTol(out, 0.01)) expect(Math.abs(Math.hypot(p[0], p[1]) - 10)).toBeLessThan(0.1);
  });

  it('keeps corners and straight edges', () => {
    const sq = subsOf(rect('a', 0, 0, 10, 10))[0];
    const out = simplifySubPath(sq, 0.1);
    expect(out.nodes.map((n) => [n.x, n.y])).toEqual(sq.nodes.map((n) => [n.x, n.y]));
    expect(out.nodes.every((n) => !n.in && !n.out)).toBe(true);
  });

  it('closes merging an end on the start, opens keeping the closing segment', () => {
    const open: SubPath = {
      closed: false,
      nodes: [
        { x: 0, y: 0 },
        { x: 10, y: 0 },
        { x: 10, y: 10 },
        { x: 0, y: 0, in: [1, 1] },
      ],
    };
    const c = closeSubPath(open);
    expect(c.closed).toBe(true);
    expect(c.nodes).toHaveLength(3);
    expect(c.nodes[0].in).toEqual([1, 1]);
    const o = openSubPath(c);
    expect(o.closed).toBe(false);
    expect(o.nodes).toHaveLength(4);
    expect(segmentCount(o)).toBe(3);
    expect(o.nodes[3].in).toEqual([1, 1]);
  });
});
