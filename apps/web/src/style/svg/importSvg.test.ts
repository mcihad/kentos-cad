import { describe, expect, it } from 'vitest';
import { colorUsage, docFromSvgTree, importSummary, mapColors, parseCss, readColor, readTransform, viewBoxTransform, type XmlNode } from './importSvg';
import { apply } from './pathData';
import { shapeBox, type SvgShape } from './svgModel';

const node = (tag: string, attrs: Record<string, string> = {}, children: XmlNode[] = [], text?: string): XmlNode => ({ tag, attrs, children, text });
const txt = (text: string): XmlNode => ({ tag: '#text', attrs: {}, children: [], text });
const close = (a: number, b: number, eps = 1e-6) => Math.abs(a - b) < eps;
const box = (s: SvgShape) => {
  const b = shapeBox(s);
  return [b.minX, b.minY, b.maxX, b.maxY].map((v) => Math.round(v * 1000) / 1000);
};

describe('colours and transforms', () => {
  it('reads every colour syntax with alpha', () => {
    expect(readColor('#abc')).toEqual({ hex: '#AABBCC', alpha: 1 });
    expect(readColor('#ff000080')?.alpha).toBeCloseTo(0.502, 2);
    expect(readColor('rebeccapurple')?.hex).toBe('#663399');
    expect(readColor('rgb(10% 20% 30% / 0.5)')).toEqual({ hex: '#1A334D', alpha: 0.5 });
    expect(readColor('hsl(120, 100%, 25%)')?.hex).toBe('#008000');
    expect(readColor('nonsense')).toBeUndefined();
  });

  it('applies every transform function, with compact numbers', () => {
    expect(apply(readTransform('rotate(90 10 10)'), 20, 10).map((v) => Math.round(v * 1e9) / 1e9)).toEqual([10, 20]);
    expect(apply(readTransform('translate(10-5)'), 0, 0)).toEqual([10, -5]);
    expect(apply(readTransform('skewX(45)'), 0, 10).map((v) => Math.round(v * 1e9) / 1e9)).toEqual([10, 10]);
    expect(apply(readTransform('matrix(2 0 0 2 1 1)'), 1, 1)).toEqual([3, 3]);
  });

  it('maps a viewBox onto a viewport with preserveAspectRatio', () => {
    // 10×10 into 40×20: meet scales by 2 and centres; none stretches; xMinYMin slice fills.
    expect(viewBoxTransform([0, 0, 10, 10], 40, 20)).toEqual([2, 0, 0, 2, 10, 0]);
    expect(viewBoxTransform([0, 0, 10, 10], 40, 20, 'none')).toEqual([4, 0, 0, 2, 0, 0]);
    expect(viewBoxTransform([5, 5, 10, 10], 40, 20, 'xMinYMin slice')).toEqual([4, 0, 0, 4, -20, -20]);
  });

  it('parses style sheets: lists, compounds, descendants, at-rules left out', () => {
    const rules = parseCss('/* c */ .a, #b { fill: red } @media print { .a { fill: blue } } g > rect.k.l { stroke: #00f !important } a:hover { fill: none }');
    expect(rules).toHaveLength(2);
    expect(rules[0].selectors).toHaveLength(2);
    expect(rules[1].decls[0]).toEqual({ prop: 'stroke', value: '#00f', important: true });
  });
});

describe('importing SVG files', () => {
  it('applies CSS classes and ids over attributes, style over both, with inheritance', () => {
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 100 100' }, [
        node('style', {}, [], '.red { fill: #FF0000 } #big { stroke: blue; stroke-width: 4 } g.box rect { stroke-opacity: .5 }'),
        node('g', { class: 'box', fill: 'green' }, [
          node('rect', { class: 'red', fill: 'yellow', x: '0', y: '0', width: '10', height: '10' }),
          node('rect', { id: 'big', x: '20', y: '0', width: '10', height: '10', style: 'fill: #00FF00' }),
        ]),
      ]),
      { symbolColor: null },
    );
    const [a, b] = doc.shapes;
    expect(a.fill).toBe('#FF0000');
    expect(b.fill).toBe('#00FF00');
    expect(b.stroke).toBe('#0000FF80');
    expect(b.strokeWidth).toBe(4);
  });

  it('expands <use> of groups and symbols (href and xlink:href) with their viewBox', () => {
    const { doc, report } = docFromSvgTree(
      node('svg', { viewBox: '0 0 100 100' }, [
        node('defs', {}, [
          node('circle', { id: 'dot', r: '5', fill: 'red' }),
          node('symbol', { id: 'sq', viewBox: '0 0 10 10' }, [node('rect', { width: '10', height: '10' })]),
        ]),
        node('use', { href: '#dot', x: '20', y: '30' }),
        node('use', { 'xlink:href': '#sq', x: '50', y: '50', width: '20', height: '20' }),
        node('use', { href: '#missing' }),
      ]),
    );
    expect(doc.shapes).toHaveLength(2);
    const [c, r] = doc.shapes;
    expect(c.kind === 'ellipse' && [c.cx, c.cy, c.rx]).toEqual([20, 30, 5]);
    expect(box(r)).toEqual([50, 50, 70, 70]);
    expect(report.uses).toBe(2);
    expect(report.broken).toBe(1);
  });

  it('reads units: a size in mm without a viewBox becomes mm units, with the symbol size', () => {
    const { doc } = docFromSvgTree(node('svg', { width: '20mm', height: '10mm' }, [node('rect', { x: '0', y: '0', width: '37.7952755906', height: '37.7952755906' })]));
    expect([doc.width, doc.height, doc.sizeMm]).toEqual([20, 10, 20]);
    expect(box(doc.shapes[0])).toEqual([0, 0, 10, 10]);
    const withBox = docFromSvgTree(node('svg', { width: '1in', height: '1in', viewBox: '0 0 24 24' }, [])).doc;
    expect([withBox.width, withBox.sizeMm]).toEqual([24, 25.4]);
  });

  it('reads nested svg viewports and every primitive', () => {
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 100 100' }, [
        node('svg', { x: '10', y: '10', width: '20', height: '20', viewBox: '0 0 2 2' }, [node('circle', { cx: '1', cy: '1', r: '1' })]),
        node('rect', { x: '40', y: '0', width: '20', height: '10', rx: '4', ry: '2' }),
        node('line', { x1: '0', y1: '50', x2: '10', y2: '50', stroke: 'red' }),
        node('polyline', { points: '0,60 10,60 10,70' }),
        node('polygon', { points: '20 60 30 60 30 70' }),
        node('ellipse', { cx: '80', cy: '80', rx: '10', ry: '5', transform: 'skewX(30)' }),
      ]),
    );
    const [circle, rect, line, pl, pg, sheared] = doc.shapes;
    expect(circle.kind === 'ellipse' && [circle.cx, circle.cy, circle.rx]).toEqual([20, 20, 10]);
    // Elliptic corners cannot stay a rectangle.
    expect(rect.kind).toBe('path');
    expect(box(rect)).toEqual([40, 0, 60, 10]);
    expect(line.kind === 'path' && [line.fill, line.stroke, line.cap, line.join]).toEqual(['none', '#FF0000', 'butt', 'miter']);
    // A polyline is filled (the SVG default) and open; a polygon closed.
    expect(pl.kind === 'path' && [pl.subs[0].closed, pl.fill, pl.fillRule]).toEqual([false, 'fill', 'nonzero']);
    expect(pg.kind === 'path' && pg.subs[0].closed).toBe(true);
    expect(sheared.kind).toBe('path');
  });

  it('keeps opacity, dash, caps, joins and fill rule', () => {
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 10 10' }, [
        node('g', { opacity: '0.5', transform: 'scale(2)' }, [
          node('path', { d: 'M0 0L5 0', fill: 'none', stroke: 'currentColor', 'stroke-opacity': '0.5', 'stroke-dasharray': '1 2', 'stroke-linecap': 'round', 'stroke-linejoin': 'bevel', 'stroke-width': '0.5' }),
          node('path', { d: 'M0 0L5 5L0 5Z', fill: '#123456', 'fill-opacity': '0.5', 'fill-rule': 'evenodd' }),
        ]),
      ]),
    );
    const [a, b] = doc.shapes;
    // The symbol colour carries no alpha: stroke opacity moves to the shape.
    expect([a.stroke, a.opacity, a.dash, a.cap, a.join, a.strokeWidth]).toEqual(['fill', 0.25, [2, 4], 'round', 'bevel', 1]);
    expect([b.fill, b.opacity, b.fillRule]).toEqual(['#12345680', 0.5, 'evenodd']);
  });

  it('flattens gradients and patterns to a colour, counts clips, masks and images', () => {
    const r = docFromSvgTree(
      node('svg', { viewBox: '0 0 10 10' }, [
        node('defs', {}, [
          node('linearGradient', { id: 'g' }, [node('stop', { 'stop-color': 'red' }), node('stop', { style: 'stop-color: #00FF00' }), node('stop', { 'stop-color': 'blue' })]),
          node('linearGradient', { id: 'g2', 'xlink:href': '#g' }),
          node('pattern', { id: 'p' }, [node('rect', { fill: '#ABCDEF', width: '1', height: '1' })]),
          node('clipPath', { id: 'c' }, [node('rect', { width: '5', height: '5' })]),
        ]),
        node('rect', { width: '5', height: '5', fill: 'url(#g)', 'clip-path': 'url(#c)' }),
        node('rect', { width: '5', height: '5', fill: 'url(#g2)' }),
        node('rect', { width: '5', height: '5', fill: 'url(#p)', mask: 'url(#m)' }),
        node('rect', { width: '5', height: '5', fill: 'url(#nothing) orange' }),
        node('image', { href: 'data:image/png;base64,AA' }),
      ]),
    );
    expect(r.doc.shapes.map((s) => s.fill)).toEqual(['#00FF00', '#00FF00', '#ABCDEF', '#FFA500']);
    expect([r.report.gradients, r.report.patterns, r.report.clips, r.report.masks, r.report.images]).toEqual([2, 1, 1, 1, 1]);
    const { done, lost } = importSummary(r.report);
    expect(done).toBe('4 şekil alındı');
    expect(lost).toContain('2 degrade düz renge çevrildi');
    expect(lost).toContain('1 kırpma yolu atlandı');
  });

  it('reads text with tspan lines, sizes in em and pt, anchors and weights', () => {
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 100 100' }, [
        node('text', { x: '10', y: '20', 'font-size': '12pt', 'font-family': 'Georgia, serif', 'text-anchor': 'middle' }, [
          txt('\n  Ada '),
          node('tspan', { 'font-weight': 'bold' }, [txt('12')]),
          node('tspan', { x: '10', dy: '1.2em', 'font-size': '0.5em' }, [txt('Parsel 4')]),
        ]),
      ]),
    );
    expect(doc.shapes).toHaveLength(2);
    const [a, b] = doc.shapes;
    expect(a.kind === 'text' && [a.text, a.x, a.y, a.size, a.font, a.anchor, a.weight]).toEqual(['Ada 12', 10, 20, 16, 'serif', 'middle', 400]);
    expect(b.kind === 'text' && [b.text, b.x, b.y, b.size]).toEqual(['Parsel 4', 10, 29.6, 8]);
  });

  it('maps black (or the dominant colour) to the symbol colour and a second colour to param(stroke)', () => {
    const tree = node('svg', { viewBox: '0 0 100 100' }, [
      node('rect', { width: '100', height: '100', fill: '#1D1D1B' }),
      node('rect', { width: '10', height: '10', fill: '#E0457B', stroke: '#1D1D1B' }),
      node('circle', { r: '4', fill: '#3366FF' }),
    ]);
    const auto = docFromSvgTree(tree);
    expect(auto.doc.shapes.map((s) => [s.fill, s.stroke])).toEqual([
      ['fill', 'none'],
      ['#E0457B', 'fill'],
      ['#3366FF', 'none'],
    ]);
    const raw = docFromSvgTree(tree, { symbolColor: null });
    expect(raw.colors.map((c) => c.color)).toEqual(['#1D1D1B', '#E0457B', '#3366FF']);
    const mapped = mapColors(raw.doc, 'dominant', '#E0457B');
    expect(mapped.shapes.map((s) => [s.fill, s.stroke])).toEqual([
      ['fill', 'none'],
      ['stroke', 'fill'],
      ['#3366FF', 'none'],
    ]);
    expect(colorUsage(mapped)).toHaveLength(1);
    // A file that paints with currentColor itself keeps its black.
    const own = docFromSvgTree(node('svg', { viewBox: '0 0 10 10' }, [node('rect', { width: '1', height: '1', fill: 'currentColor' }), node('rect', { width: '1', height: '1', fill: '#000' })]));
    expect(own.doc.shapes.map((s) => s.fill)).toEqual(['fill', '#000000']);
  });

  it('keeps the editor’s own ids, names, groups, hidden shapes, size and background', () => {
    const { doc, reference } = docFromSvgTree(
      node('svg', { viewBox: '0 0 50 50', 'data-size-mm': '8', 'data-background': '#FFFFEE' }, [
        node('defs', {}, [node('image', { 'data-kentos': 'reference', href: 'data:image/png;base64,AA', x: '1', y: '2', width: '30', height: '20', opacity: '0.4', 'data-locked': '0' })]),
        node('g', { 'data-group': 'gA' }, [node('rect', { id: 'r1', 'data-name': 'Çerçeve', width: '5', height: '5' }), node('rect', { id: 'r2', width: '5', height: '5', display: 'none' })]),
      ]),
      { editor: true, symbolColor: null },
    );
    expect(doc.shapes.map((s) => [s.id, s.name, s.group, s.hidden])).toEqual([
      ['r1', 'Çerçeve', 'gA', undefined],
      ['r2', undefined, 'gA', true],
    ]);
    expect([doc.sizeMm, doc.background]).toEqual([8, '#FFFFEE']);
    expect(reference && [reference.x, reference.y, reference.width, reference.opacity, reference.locked]).toEqual([1, 2, 30, 0.4, false]);
  });

  it('reads attribute selectors and font size names from the file only', () => {
    // The TypeScript looked names up in plain objects: `[constructor]` matched every element (every object
    // has one) and `font-size: constructor` read a function, so the text's size became NaN.
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 100 100' }, [
        node('style', {}, [], '[constructor] { fill: #FF0000 } [data-k] { fill: #0000FF }'),
        node('rect', { width: '10', height: '10', fill: '#00FF00' }),
        node('rect', { width: '10', height: '10', 'data-k': '' }),
        node('text', { 'font-size': 'constructor', x: '0', y: '20' }, [], 'A'),
      ]),
      { symbolColor: null },
    );
    expect(doc.shapes.map((s) => s.fill)).toEqual(['#00FF00', '#0000FF', '#000000']);
    expect(doc.shapes[2]).toMatchObject({ kind: 'text', size: 16 });
  });

  it('leaves out what is not drawn: display none, hidden visibility, defs, symbols, Inkscape layers make no group', () => {
    const { doc } = docFromSvgTree(
      node('svg', { viewBox: '0 0 10 10' }, [
        node('g', { 'inkscape:groupmode': 'layer' }, [node('rect', { width: '1', height: '1' }), node('rect', { width: '2', height: '2' })]),
        node('rect', { width: '1', height: '1', display: 'none' }),
        node('g', { visibility: 'hidden' }, [node('rect', { width: '1', height: '1' }), node('rect', { width: '1', height: '1', visibility: 'visible' })]),
        node('symbol', { id: 's' }, [node('rect', { width: '1', height: '1' })]),
      ]),
    );
    expect(doc.shapes).toHaveLength(3);
    expect(doc.shapes.every((s) => !s.group)).toBe(true);
    expect(close(doc.width, 10)).toBe(true);
  });
});
