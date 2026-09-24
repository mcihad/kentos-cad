import { describe, expect, it } from 'vitest';
import { SNAP_KINDS, SnapIndex, type SnapKind } from './snapping';
import type { SvgShape } from './svgModel';

const base = { fill: 'fill', stroke: 'none', strokeWidth: 1 } as const;
const page = { width: 100, height: 100 };
const all = new Set<SnapKind>(SNAP_KINDS.map((k) => k.kind));
const only = (...k: SnapKind[]) => new Set<SnapKind>(k);
const close = (p: readonly number[] | undefined, q: readonly number[], eps = 1e-6) => !!p && Math.abs(p[0] - q[0]) < eps && Math.abs(p[1] - q[1]) < eps;

const square: SvgShape = {
  ...base,
  id: 'sq',
  kind: 'path',
  subs: [
    {
      closed: true,
      nodes: [
        { x: 10, y: 10 },
        { x: 30, y: 10 },
        { x: 30, y: 30 },
        { x: 10, y: 30 },
      ],
    },
  ],
};
const disc: SvgShape = { ...base, id: 'c', kind: 'ellipse', cx: 60, cy: 60, rx: 10, ry: 10 };
const smoothPath: SvgShape = {
  ...base,
  id: 'p',
  kind: 'path',
  subs: [
    {
      closed: false,
      nodes: [
        { x: 50, y: 20, out: [55, 20] },
        { x: 60, y: 25, in: [57, 25], out: [63, 25] },
        { x: 70, y: 20, in: [65, 20] },
      ],
    },
  ],
};

describe('snap index', () => {
  it('snaps to cusp and smooth nodes, segment middles and the centroid', () => {
    const ix = new SnapIndex({ shapes: [square, smoothPath], page, kinds: all });
    expect(ix.query([10.5, 10.4], 2)?.kind).toBe('cusp');
    expect(ix.query([60.2, 25.1], 1)).toMatchObject({ kind: 'smooth', p: [60, 25] });
    expect(ix.query([20.3, 10.2], 1)).toMatchObject({ kind: 'mid', p: [20, 10] });
    const c = ix.query([20.2, 20.1], 1);
    expect(c?.kind).toBe('centre');
    expect(close(c?.p, [20, 20])).toBe(true);
  });

  it('snaps to bounding boxes and the canvas', () => {
    const ix = new SnapIndex({ shapes: [disc], page, kinds: only('bboxCorner', 'bboxMid', 'bboxCentre', 'page') });
    expect(ix.query([50.2, 50.1], 1)).toMatchObject({ kind: 'bboxCorner', p: [50, 50] });
    expect(ix.query([60.1, 49.8], 1)).toMatchObject({ kind: 'bboxMid', p: [60, 50] });
    expect(ix.query([60.1, 60.1], 1)?.kind).toBe('bboxCentre');
    expect(ix.query([99.6, 0.3], 1)).toMatchObject({ kind: 'page', label: 'Tuval köşesi' });
    expect(ix.query([50.1, 50.1], 1)?.kind).toBe('bboxCorner');
    // On a canvas edge away from its key points: the nearest point of the edge.
    expect(ix.query([30, 99.4], 1)).toMatchObject({ kind: 'page', p: [30, 100], label: 'Tuval kenarı' });
  });

  it('finds crossings of outlines, curves included', () => {
    const other: SvgShape = { ...square, id: 'sq2', subs: [{ closed: true, nodes: square.kind === 'path' ? square.subs[0].nodes.map((n) => ({ x: n.x + 10, y: n.y + 10 })) : [] }] };
    const ix = new SnapIndex({ shapes: [square, other], page, kinds: only('intersection') });
    expect(ix.query([30.4, 20.3], 1)).toMatchObject({ kind: 'intersection', p: [30, 20] });
    const bar: SvgShape = {
      ...base,
      id: 'bar',
      kind: 'path',
      subs: [
        {
          closed: false,
          nodes: [
            { x: 60, y: 30 },
            { x: 60, y: 90 },
          ],
        },
      ],
    };
    const hit = new SnapIndex({ shapes: [disc, bar], page, kinds: only('intersection') }).query([60.3, 50.4], 1);
    expect(hit?.kind).toBe('intersection');
    expect(Math.abs(hit!.p[1] - 50)).toBeLessThan(0.01);
  });

  it('gives the perpendicular foot and the tangent point from where the tool started', () => {
    const ix = new SnapIndex({ shapes: [square, disc], page, kinds: only('perpendicular', 'tangent') });
    // From (40, 20), perpendicular to the square's right side x = 30.
    expect(ix.query([30.3, 21], 2, [40, 20])).toMatchObject({ kind: 'perpendicular', p: [30, 20] });
    // Nothing without a start point.
    expect(ix.query([30.3, 21], 2, null)).toBeNull();
    // Tangent from (60, 30) to the circle (radius 10 round (60, 60)): touching points at y = 60 − 100/30.
    const t = ix.query([51, 56], 2, [60, 30]);
    expect(t?.kind).toBe('tangent');
    const [x, y] = t!.p;
    expect(Math.abs(Math.hypot(x - 60, y - 60) - 10)).toBeLessThan(0.02);
    // The touching radius is square to the line from the start.
    expect(Math.abs((x - 60) * (x - 60) + (y - 60) * (y - 30))).toBeLessThan(0.3);
  });

  it('snaps to guides, their crossings and their crossings with outlines', () => {
    const guides = [
      { id: 'g1', x: 0, y: 40, angle: 0 },
      { id: 'g2', x: 45, y: 0, angle: 90 },
    ];
    const ix = new SnapIndex({ shapes: [], guides, page, kinds: only('guide') });
    expect(ix.query([45.3, 40.2], 1)).toMatchObject({ kind: 'guide', label: 'Kılavuz kesişimi' });
    expect(ix.query([10, 40.6], 1)).toMatchObject({ kind: 'guide', p: [10, 40] });
    const ix2 = new SnapIndex({ shapes: [square], guides: [{ id: 'g', x: 0, y: 20, angle: 0 }], page, kinds: only('guide', 'intersection') });
    expect(ix2.query([10.3, 20.2], 1)).toMatchObject({ kind: 'intersection', p: [10, 20] });
  });

  it('prefers points to lines, and leaves out moving shapes and nodes', () => {
    // The guide passes nearer than the node; the node still wins (crossings off, they would be nearer yet).
    const ix = new SnapIndex({ shapes: [square], guides: [{ id: 'g', x: 0, y: 10.25, angle: 0 }], page, kinds: new Set([...all].filter((k) => k !== 'intersection')) });
    expect(ix.query([10.2, 10.2], 1)?.kind).toBe('cusp');
    expect(new SnapIndex({ shapes: [square], page, kinds: all, exclude: new Set(['sq']) }).query([10.2, 10.2], 1)?.kind).not.toBe('cusp');
    const skip = new SnapIndex({ shapes: [square], page, kinds: only('cusp', 'mid'), skipNode: (_id, _s, i) => i === 0 });
    expect(skip.query([10.2, 10.2], 1)).toBeNull();
    // The segment next to a moving node has no middle to snap to either.
    expect(skip.query([20, 10], 1)).toBeNull();
    expect(skip.query([30, 20], 1)?.kind).toBe('mid');
    expect(new SnapIndex({ shapes: [square], page, kinds: only() }).query([10, 10], 5)).toBeNull();
  });
});
