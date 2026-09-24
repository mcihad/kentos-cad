import { describe, expect, it } from 'vitest';
import { docFromSvgTree, readPaint, readTransform, type XmlNode } from './importSvg';
import { apply, arcToCubics, flattenSubPath, parsePathData, pathDataOf, subPathsBox } from './pathData';
import { boxToBox, regularPolygon, rotation, serializeDoc, shapeBox, transformShape, translate, type SvgShape } from './svgModel';

const close = (a: number, b: number, eps = 1e-6) => Math.abs(a - b) < eps;

describe('path data', () => {
  it('reads every command, absolute and relative, into nodes', () => {
    const [sp] = parsePathData('M10 10 h20 v20 H10 z');
    expect(sp.closed).toBe(true);
    expect(sp.nodes.map((n) => [n.x, n.y])).toEqual([
      [10, 10],
      [30, 10],
      [30, 30],
      [10, 30],
    ]);
    const [c] = parsePathData('M0 0C10 0 20 10 20 20s10 20 20 20');
    expect(c.nodes).toHaveLength(3);
    expect(c.nodes[1].in).toEqual([20, 10]);
    // The smooth curve's first handle mirrors the last one.
    expect(c.nodes[1].out).toEqual([20, 30]);
    const [q] = parsePathData('M0 0Q10 10 20 0T40 0');
    expect(q.nodes[1].x).toBe(20);
    expect(q.nodes[2].x).toBe(40);
    // Implicit lineto after moveto, compact numbers.
    const [l] = parsePathData('m1-1 2.5.5.5.5');
    expect(l.nodes.map((n) => [n.x, n.y])).toEqual([
      [1, -1],
      [3.5, -0.5],
      [4, 0],
    ]);
  });

  it('turns arcs into cubics that stay on the circle, with compact flags', () => {
    const cubics = arcToCubics(10, 0, 10, 10, 0, false, true, -10, 0);
    expect(cubics).toHaveLength(2);
    const last = cubics[1];
    expect(close(last[4], -10) && close(last[5], 0)).toBe(true);
    const [sp] = parsePathData('M10 0A10 10 0 1110 0.001');
    // Large arc = sweep puts the centre on the far side (SVG F.6.5): a near-full circle round (20, 0).
    const pts = flattenSubPath(sp, 12);
    expect(pts.every(([x, y]) => Math.abs(Math.hypot(x - 20, y) - 10) < 0.05)).toBe(true);
    expect(Math.max(...pts.map(([x]) => x))).toBeCloseTo(30, 1);
  });

  it('writes and reads back the same nodes; boxes cover the curves', () => {
    const d = 'M0 0L10 0C15 0 20 5 20 10Z';
    const subs = parsePathData(d);
    expect(parsePathData(pathDataOf(subs))).toEqual(subs);
    const box = subPathsBox(parsePathData('M0 0C0 10 10 10 10 0'));
    expect(close(box.maxY, 7.5, 1e-3)).toBe(true);
  });
});

describe('shapes', () => {
  const rect: SvgShape = { id: 'r', kind: 'rect', x: 0, y: 0, w: 10, h: 20, fill: 'fill', stroke: 'none', strokeWidth: 1 };

  it('keeps rectangles under moves, scales and uniform rotations; shears make paths', () => {
    const moved = transformShape(rect, translate(5, 5));
    expect(moved.kind === 'rect' && [moved.x, moved.y, moved.w, moved.h]).toEqual([5, 5, 10, 20]);
    const scaled = transformShape(rect, boxToBox({ minX: 0, minY: 0, maxX: 10, maxY: 20 }, { minX: 0, minY: 0, maxX: 20, maxY: 20 }));
    expect(scaled.kind === 'rect' && [scaled.w, scaled.h]).toEqual([20, 20]);
    const turned = transformShape(rect, rotation(90, 5, 10));
    expect(turned.kind).toBe('rect');
    expect(turned.kind === 'rect' && turned.rotate).toBe(90);
    const box = shapeBox(turned);
    expect(close(box.maxX - box.minX, 20, 1e-6) && close(box.maxY - box.minY, 10, 1e-6)).toBe(true);
    expect(transformShape(rect, [1, 0, 0.5, 1, 0, 0]).kind).toBe('path');
  });

  it('writes an SVG with the symbol colour parameters and groups', () => {
    const svg = serializeDoc({
      width: 40,
      height: 20,
      shapes: [
        { ...rect, group: 'a' },
        { id: 'e', kind: 'ellipse', cx: 30, cy: 10, rx: 5, ry: 5, fill: 'none', stroke: 'stroke', strokeWidth: 2, group: 'a' },
        { id: 't', kind: 'text', x: 20, y: 18, text: 'A<B', size: 6, weight: 700, font: 'sans', anchor: 'middle', fill: '#FF0000', stroke: 'none', strokeWidth: 0 },
      ],
    });
    expect(svg).toContain('viewBox="0 0 40 20"');
    expect(svg).toContain('fill="currentColor"');
    expect(svg).toContain('stroke="param(stroke) #000000"');
    expect(svg.match(/<g data-group="a">/g)).toHaveLength(1);
    expect(svg).toContain('>A&lt;B</text>');
  });

  it('makes regular polygons and stars with the first corner up', () => {
    const tri = regularPolygon(0, 0, 10, 3);
    expect(tri.nodes).toHaveLength(3);
    expect(close(tri.nodes[0].x, 0) && close(tri.nodes[0].y, -10)).toBe(true);
    expect(regularPolygon(0, 0, 10, 5, 4).nodes).toHaveLength(10);
  });
});

describe('importing SVG', () => {
  const node = (tag: string, attrs: Record<string, string> = {}, children: XmlNode[] = [], text?: string): XmlNode => ({ tag, attrs, children, text });

  it('reads colours and transforms', () => {
    expect(readPaint('#000')).toBe('fill');
    expect(readPaint('rgb(255, 0, 0)')).toBe('#FF0000');
    expect(readPaint('currentColor')).toBe('fill');
    expect(readPaint('param(stroke) #123456')).toBe('stroke');
    expect(readPaint('url(#g)')).toBeUndefined();
    const m = readTransform('translate(10 20) scale(2)');
    expect(apply(m, 1, 1)).toEqual([12, 22]);
  });

  it('walks groups with their transforms and paints, keeps kinds and lists what it skipped', () => {
    const { doc, skipped } = docFromSvgTree(
      node('svg', { viewBox: '10 10 50 50' }, [
        node('g', { transform: 'translate(5 0)', fill: 'red' }, [node('rect', { x: '10', y: '10', width: '10', height: '10' }), node('circle', { cx: '30', cy: '30', r: '5', fill: 'none', stroke: '#00F' })]),
        node('path', { d: 'M10 10L20 20', stroke: 'black', 'stroke-width': '2' }),
        node('text', { x: '20', y: '40', 'font-size': '8', 'font-weight': 'bold' }, [], 'KA'),
        node('image', { href: 'x.png' }),
      ]),
    );
    expect([doc.width, doc.height]).toEqual([50, 50]);
    const [r, c, p, t] = doc.shapes;
    // The viewBox origin moves to 0,0; the group's translation applies.
    expect(r.kind === 'rect' && [r.x, r.y, r.fill]).toEqual([5, 0, '#FF0000']);
    expect(c.kind === 'ellipse' && [c.cx, c.cy, c.fill, c.stroke]).toEqual([25, 20, 'none', '#0000FF']);
    expect(r.group).toBe(c.group);
    expect(p.kind === 'path' && [p.stroke, p.strokeWidth, p.group]).toEqual(['fill', 2, undefined]);
    expect(t.kind === 'text' && [t.text, t.weight, t.x, t.y]).toEqual(['KA', 700, 10, 30]);
    expect(skipped).toEqual(['image']);
  });
});
