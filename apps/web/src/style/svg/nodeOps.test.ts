import { describe, expect, it } from 'vitest';
import { bez, flattenSubPathTol, segmentCount, segmentCubic } from './bezier';
import { alignNodes, breakAtNodes, cornerAt, cornerNodes, deleteNodes, deleteSegments, distributeNodes, filletRadius, insertMidNodes, joinEnds, moveNodes, nodeTypeOf, segmentsTo, setNodeType } from './nodeOps';
import type { SubPath } from './pathData';
import { toPath } from './svgModel';

const square = (): SubPath[] => [
  {
    closed: true,
    nodes: [
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 10, y: 10 },
      { x: 0, y: 10 },
    ],
  },
];
const polyline = (...pts: [number, number][]): SubPath => ({ closed: false, nodes: pts.map(([x, y]) => ({ x, y })) });
const disc = (): SubPath[] => (toPath({ id: 'c', kind: 'ellipse', cx: 0, cy: 0, rx: 10, ry: 10, fill: 'fill', stroke: 'none', strokeWidth: 1 }) as { subs: SubPath[] }).subs;
const xy = (sp: SubPath) => sp.nodes.map((n) => [n.x, n.y]);

describe('node types', () => {
  it('reads cusp, smooth and symmetric from the handles', () => {
    const sp: SubPath = {
      closed: false,
      nodes: [
        { x: 0, y: 0, out: [1, 0] },
        { x: 5, y: 0, in: [4, 0], out: [6, 0] },
        { x: 10, y: 0, in: [8, 0], out: [11, 0] },
        { x: 15, y: 5, in: [14, 4], out: [16, 0] },
      ],
    };
    expect([0, 1, 2, 3].map((i) => nodeTypeOf(sp, i))).toEqual(['cusp', 'symmetric', 'smooth', 'cusp']);
  });

  it('makes a corner smooth, symmetric or auto, and keeps the stored type', () => {
    const [sq] = square();
    const smooth = setNodeType([sq], [{ sub: 0, index: 1 }], 'smooth')[0];
    const n = smooth.nodes[1];
    expect(n.type).toBe('smooth');
    expect(n.in && n.out).toBeTruthy();
    // In line through the node.
    const a = [n.x - n.in![0], n.y - n.in![1]];
    const b = [n.out![0] - n.x, n.out![1] - n.y];
    expect(Math.abs(a[0] * b[1] - a[1] * b[0])).toBeLessThan(1e-9);
    const sym = setNodeType([smooth], [{ sub: 0, index: 1 }], 'symmetric')[0].nodes[1];
    expect(Math.hypot(sym.x - sym.in![0], sym.y - sym.in![1])).toBeCloseTo(Math.hypot(sym.out![0] - sym.x, sym.out![1] - sym.y), 9);
    // Auto: handles follow when a neighbour moves.
    const auto = setNodeType([sq], [{ sub: 0, index: 1 }], 'auto');
    const moved = moveNodes(auto, [{ sub: 0, index: 2 }], 0, 10)[0].nodes[1];
    expect(moved.out![1]).toBeGreaterThan(auto[0].nodes[1].out![1]);
  });
});

describe('adding and deleting nodes', () => {
  it('adds a node in the middle of each chosen segment, on the curve', () => {
    const [c] = disc();
    const r = insertMidNodes([c], [
      { sub: 0, index: 0 },
      { sub: 0, index: 1 },
    ]);
    expect(r.subs[0].nodes).toHaveLength(5);
    const m = r.subs[0].nodes[1];
    expect(Math.hypot(m.x, m.y)).toBeCloseTo(10, 2);
    expect(r.refs).toHaveLength(3);
    // The closing segment of a square.
    const sq = insertMidNodes(square(), [
      { sub: 0, index: 3 },
      { sub: 0, index: 0 },
    ]);
    expect(xy(sq.subs[0])).toEqual([
      [0, 0],
      [10, 0],
      [10, 10],
      [0, 10],
      [0, 5],
    ]);
  });

  it('deletes a node keeping the curve close to its old shape', () => {
    const [c] = disc();
    const out = deleteNodes([c], [{ sub: 0, index: 1 }], true)[0];
    expect(out.nodes).toHaveLength(3);
    // The fitted span still runs near the old circle.
    for (let t = 0.1; t < 1; t += 0.2) {
      const p = bez(segmentCubic(out, 0), t);
      expect(Math.abs(Math.hypot(p[0], p[1]) - 10)).toBeLessThan(1.2);
    }
    const plain = deleteNodes([c], [{ sub: 0, index: 1 }], false)[0];
    expect(plain.nodes).toHaveLength(3);
  });

  it('drops a sub-path left with too few nodes, and open ends lose their handles', () => {
    expect(deleteNodes([polyline([0, 0], [5, 0])], [{ sub: 0, index: 0 }])).toHaveLength(0);
    const open: SubPath = {
      closed: false,
      nodes: [
        { x: 0, y: 0, out: [1, 1] },
        { x: 5, y: 0, in: [4, 1], out: [6, 1] },
        { x: 10, y: 0, in: [9, 1] },
      ],
    };
    const r = deleteNodes([open], [{ sub: 0, index: 0 }])[0];
    expect(r.nodes).toHaveLength(2);
    expect(r.nodes[0].in).toBeUndefined();
  });
});

describe('joining, breaking and deleting segments', () => {
  it('joins the ends of two open paths into one, merging or with a segment', () => {
    const subs = [polyline([0, 0], [10, 0]), polyline([12, 0], [20, 5])];
    const merged = joinEnds(subs, [
      { sub: 0, index: 1 },
      { sub: 1, index: 0 },
    ], true);
    if ('error' in merged) throw new Error(merged.error);
    expect(merged.subs).toHaveLength(1);
    expect(xy(merged.subs[0])).toEqual([
      [0, 0],
      [11, 0],
      [20, 5],
    ]);
    const seg = joinEnds(subs, [
      { sub: 0, index: 0 },
      { sub: 1, index: 1 },
    ], false);
    if ('error' in seg) throw new Error(seg.error);
    // Start of the first to end of the second: both turn round so the chosen ends meet, then a straight segment.
    expect(seg.subs[0].nodes).toHaveLength(4);
    expect(segmentCount(seg.subs[0])).toBe(3);
  });

  it('closes a path when its own two ends are joined', () => {
    const r = joinEnds([polyline([0, 0], [10, 0], [10, 10], [0, 0.5])], [
      { sub: 0, index: 0 },
      { sub: 0, index: 3 },
    ], true);
    if ('error' in r) throw new Error(r.error);
    expect(r.subs[0].closed).toBe(true);
    expect(r.subs[0].nodes).toHaveLength(3);
    expect('error' in joinEnds(square(), [{ sub: 0, index: 1 }], true)).toBe(true);
  });

  it('breaks at nodes: a closed path opens, an open one splits', () => {
    const [open] = breakAtNodes(square(), [{ sub: 0, index: 2 }]);
    expect(open.closed).toBe(false);
    expect(xy(open)).toEqual([
      [10, 10],
      [0, 10],
      [0, 0],
      [10, 0],
      [10, 10],
    ]);
    const parts = breakAtNodes([polyline([0, 0], [5, 0], [10, 0])], [{ sub: 0, index: 1 }]);
    expect(parts.map(xy)).toEqual([
      [
        [0, 0],
        [5, 0],
      ],
      [
        [5, 0],
        [10, 0],
      ],
    ]);
  });

  it('deletes a segment: a closed path opens there', () => {
    const r = deleteSegments(square(), [
      { sub: 0, index: 1 },
      { sub: 0, index: 2 },
    ]);
    if ('error' in r) throw new Error(r.error);
    expect(r).toHaveLength(1);
    expect(r[0].closed).toBe(false);
    expect(xy(r[0])).toEqual([
      [10, 10],
      [0, 10],
      [0, 0],
      [10, 0],
    ]);
    const split = deleteSegments([polyline([0, 0], [5, 0], [10, 0], [15, 0])], [
      { sub: 0, index: 1 },
      { sub: 0, index: 2 },
    ]);
    if ('error' in split) throw new Error(split.error);
    expect(split).toHaveLength(2);
  });

  it('turns segments into lines and into curves', () => {
    const [c] = disc();
    const lines = segmentsTo([c], [
      { sub: 0, index: 0 },
      { sub: 0, index: 1 },
    ], 'line')[0];
    expect(lines.nodes[0].out).toBeUndefined();
    expect(lines.nodes[1].in).toBeUndefined();
    const curves = segmentsTo(square(), [
      { sub: 0, index: 0 },
      { sub: 0, index: 1 },
    ], 'curve')[0];
    expect(curves.nodes[0].out).toEqual([10 / 3, 0]);
  });
});

describe('fillet and chamfer', () => {
  it('rounds a square corner with a tangent arc of the given radius', () => {
    const r = cornerNodes(square(), [{ sub: 0, index: 1 }], 'fillet', 3);
    if ('error' in r) throw new Error(r.error);
    const sp = r.subs[0];
    expect(sp.nodes).toHaveLength(5);
    const P = sp.nodes[1];
    const Q = sp.nodes[2];
    expect(P.x).toBeCloseTo(7, 12);
    expect(P.y).toBeCloseTo(0, 12);
    expect(Q.x).toBeCloseTo(10, 12);
    expect(Q.y).toBeCloseTo(3, 12);
    // The arc's middle is r from the fillet centre (7, 3).
    const mid = bez(segmentCubic(sp, 1), 0.5);
    expect(Math.hypot(mid[0] - 7, mid[1] - 3)).toBeCloseTo(3, 3);
  });

  it('chamfers by a distance along each side', () => {
    const r = cornerNodes(square(), [{ sub: 0, index: 0 }], 'chamfer', 2);
    if ('error' in r) throw new Error(r.error);
    const pts = xy(r.subs[0]);
    expect(pts).toContainEqual([2, 0]);
    expect(pts).toContainEqual([0, 2]);
    expect(pts).not.toContainEqual([0, 0]);
    expect(r.subs[0].nodes.every((n) => !n.in && !n.out)).toBe(true);
  });

  it('merges into the neighbour when the cut reaches it, and refuses a radius too big', () => {
    const r = cornerNodes(square(), [{ sub: 0, index: 1 }], 'chamfer', 10);
    if ('error' in r) throw new Error(r.error);
    expect(r.subs[0].nodes).toHaveLength(3);
    expect('error' in cornerNodes(square(), [{ sub: 0, index: 1 }], 'fillet', 11)).toBe(true);
    expect('error' in cornerNodes([polyline([0, 0], [10, 0], [20, 0])], [{ sub: 0, index: 1 }], 'fillet', 1)).toBe(true);
    expect('error' in cornerNodes([polyline([0, 0], [10, 0])], [{ sub: 0, index: 0 }], 'fillet', 1)).toBe(true);
  });

  it('rounds several corners at once and gives the radius for a drag distance', () => {
    const r = cornerNodes(
      square(),
      [0, 1, 2, 3].map((index) => ({ sub: 0, index })),
      'fillet',
      2,
    );
    if ('error' in r) throw new Error(r.error);
    expect(r.subs[0].nodes).toHaveLength(8);
    const c = cornerAt(square()[0], 1)!;
    expect(c.angle).toBeCloseTo(Math.PI / 2, 9);
    expect(filletRadius(c, 3)).toBeCloseTo(3, 9);
    // A 60° corner: the arc touches farther out than its radius.
    const tri = cornerAt(polyline([0, 0], [10, 0], [5, 8.660254]), 1)!;
    expect(filletRadius(tri, 3)).toBeCloseTo(3 * Math.tan(Math.PI / 6), 5);
  });

  it('keeps curved sides curved, cutting them at the distance', () => {
    const sp: SubPath = {
      closed: false,
      nodes: [
        { x: 0, y: 0, out: [3, 3] },
        { x: 10, y: 0 },
        { x: 10, y: 10 },
      ],
    };
    const r = cornerNodes([sp], [{ sub: 0, index: 1 }], 'fillet', 2);
    if ('error' in r) throw new Error(r.error);
    const out = r.subs[0];
    expect(out.nodes[0].out).toBeTruthy();
    // The outline stays continuous and inside the old corner.
    const pts = flattenSubPathTol(out, 0.01);
    expect(Math.max(...pts.map((p) => p[0]))).toBeLessThanOrEqual(10 + 1e-9);
  });
});

describe('aligning and distributing nodes', () => {
  it('lines nodes up and spaces them evenly', () => {
    const sp = polyline([0, 0], [3, 2], [9, 5], [10, 1]);
    const refs = [0, 1, 2, 3].map((index) => ({ sub: 0, index }));
    expect(alignNodes([sp], refs, 'y', 'max')[0].nodes.map((n) => n.y)).toEqual([5, 5, 5, 5]);
    expect(alignNodes([sp], refs, 'x', 'mid')[0].nodes.map((n) => n.x)).toEqual([5, 5, 5, 5]);
    expect(distributeNodes([sp], refs, 'x')[0].nodes.map((n) => Math.round(n.x * 1000) / 1000)).toEqual([0, 3.333, 6.667, 10]);
  });
});
