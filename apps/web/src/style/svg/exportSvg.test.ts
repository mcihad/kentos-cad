import { describe, expect, it } from 'vitest';
import { crc32, exportBox, pngSize, sourceText, svgText, withPngDpi } from './exportSvg';
import { docFromSvgTree, type XmlNode } from './importSvg';
import type { SvgDoc } from './svgModel';

/** Just enough XML reading for the writer's own output (no comments, CDATA or entities beyond the five). */
function parseXml(text: string): XmlNode {
  const unesc = (v: string) => v.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&amp;/g, '&');
  const stack: XmlNode[] = [{ tag: '#root', attrs: {}, children: [] }];
  for (const m of text.matchAll(/<(\/?)([\w:-]+)([^>]*?)(\/?)>|([^<]+)/g)) {
    const top = stack[stack.length - 1];
    if (m[5] !== undefined) {
      if (m[5].trim()) top.children.push({ tag: '#text', attrs: {}, children: [], text: unesc(m[5]) });
      continue;
    }
    if (m[1]) {
      stack.pop();
      continue;
    }
    const attrs: Record<string, string> = {};
    for (const a of m[3].matchAll(/([\w:-]+)="([^"]*)"/g)) attrs[a[1]] = unesc(a[2]);
    const node: XmlNode = { tag: m[2], attrs, children: [] };
    top.children.push(node);
    if (!m[4]) stack.push(node);
  }
  return stack[0].children[0];
}

const doc: SvgDoc = {
  width: 40,
  height: 20,
  sizeMm: 8,
  background: '#FFFFEE',
  shapes: [
    { id: 'a', kind: 'rect', x: 0, y: 0, w: 10, h: 10, fill: 'fill', stroke: 'none', strokeWidth: 1, group: 'g1', name: 'Kutu' },
    { id: 'b', kind: 'ellipse', cx: 30, cy: 10, rx: 5, ry: 5, fill: '#FF000080', stroke: 'stroke', strokeWidth: 2, group: 'g1' },
    { id: 'c', kind: 'text', x: 20, y: 18, text: 'A<B', size: 6, weight: 700, font: 'sans', anchor: 'middle', fill: '#123456', stroke: 'none', strokeWidth: 0 },
    { id: 'd', kind: 'path', subs: [{ closed: false, nodes: [{ x: 0, y: 20 }, { x: 40, y: 20 }] }], fill: 'none', stroke: 'fill', strokeWidth: 1, hidden: true },
  ],
};

describe('exporting SVG', () => {
  it('writes a symbol SVG with the colour parameters, size and background, without hidden shapes', () => {
    const svg = svgText(doc);
    expect(svg).toContain('viewBox="0 0 40 20" width="40" height="20" data-size-mm="8" data-background="#FFFFEE"');
    expect(svg).toContain('fill="currentColor"');
    expect(svg).toContain('stroke="param(stroke) #000000"');
    expect(svg).not.toContain('<path');
    // Read back: the same shapes and colours.
    const back = docFromSvgTree(parseXml(svg), { symbolColor: null }).doc;
    expect(back.shapes.map((s) => [s.kind, s.fill, s.stroke])).toEqual([
      ['rect', 'fill', 'none'],
      ['ellipse', '#FF000080', 'stroke'],
      ['text', '#123456', 'none'],
    ]);
    expect([back.sizeMm, back.background]).toEqual([8, '#FFFFEE']);
  });

  it('writes a plain SVG: preview colours, alpha as opacity, size in mm', () => {
    const svg = svgText(doc, { colors: { ink: '#000000', second: '#2B83BA' }, pretty: true });
    expect(svg).toContain('width="8mm" height="4mm"');
    expect(svg).not.toMatch(/currentColor|param\(/);
    expect(svg).toContain('fill="#FF0000" fill-opacity="0.502" stroke="#2B83BA"');
    expect(svg).not.toContain('data-group');
    expect(svg.split('\n').length).toBeGreaterThan(5);
  });

  it('crops to the selection with its strokes', () => {
    const only = new Set(['b']);
    expect(exportBox(doc, only)).toEqual({ x: 24, y: 4, w: 12, h: 12 });
    const svg = svgText(doc, { only });
    expect(svg).toContain('viewBox="24 4 12 12"');
    expect(svg).not.toContain('<rect');
  });

  it('keeps a tracing reference out of the drawing, in <defs>', () => {
    const svg = svgText(doc, { reference: { href: 'data:image/png;base64,AAAA', x: 1, y: 2, width: 30, height: 15, opacity: 0.4, locked: true, name: 'Pafta' } });
    expect(svg).toMatch(/<defs><image data-kentos="reference"[^>]*href="data:image\/png;base64,AAAA"[^>]*\/><\/defs>/);
    const r = docFromSvgTree(parseXml(svg)).reference;
    expect(r && [r.x, r.y, r.width, r.height, r.opacity, r.locked, r.name]).toEqual([1, 2, 30, 15, 0.4, true, 'Pafta']);
  });

  it('writes the source view with ids, names and hidden shapes, and each element’s range', () => {
    const { text, spans } = sourceText(doc);
    const [a0, a1] = spans.get('a')!;
    expect(text.slice(a0, a1)).toMatch(/^<rect id="a" data-name="Kutu" /);
    const [d0, d1] = spans.get('d')!;
    expect(text.slice(d0, d1)).toMatch(/^<path id="d" .*display="none"\/>$/);
    // Read back in editor mode: the same drawing.
    const back = docFromSvgTree(parseXml(text), { editor: true, symbolColor: null }).doc;
    expect(back.shapes.map((s) => [s.id, s.name, s.group, s.hidden])).toEqual([
      ['a', 'Kutu', 'g1', undefined],
      ['b', undefined, 'g1', undefined],
      ['c', undefined, undefined, undefined],
      ['d', undefined, undefined, true],
    ]);
    // Paths come back with their round ends written out; from then on the text is stable.
    const again = sourceText(back).text;
    expect(sourceText(docFromSvgTree(parseXml(again), { editor: true, symbolColor: null }).doc).text).toBe(again);
  });
});

describe('exporting PNG', () => {
  it('sizes by pixels or by DPI and the drawing’s mm width', () => {
    expect(pngSize(40, 20, { px: 512 })).toEqual({ width: 512, height: 256 });
    expect(pngSize(40, 20, { dpi: 300, widthMm: 25.4 })).toEqual({ width: 300, height: 150 });
    // Without a size in mm a unit is a CSS pixel.
    expect(pngSize(96, 48, { dpi: 192 })).toEqual({ width: 192, height: 96 });
  });

  it('records the DPI in a pHYs chunk with a valid CRC', () => {
    expect(crc32(new TextEncoder().encode('IEND'))).toBe(0xae426082);
    const sig = [137, 80, 78, 71, 13, 10, 26, 10];
    const ihdr = [0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89];
    const iend = [0, 0, 0, 0, 73, 69, 78, 68, 0xae, 0x42, 0x60, 0x82];
    const png = withPngDpi(new Uint8Array([...sig, ...ihdr, ...iend]), 300);
    const v = new DataView(png.buffer);
    expect(String.fromCharCode(...png.subarray(37, 41))).toBe('pHYs');
    expect(v.getUint32(41)).toBe(11811);
    expect(v.getUint32(50)).toBe(crc32(png.subarray(37, 50)));
    expect(String.fromCharCode(...png.subarray(58, 62))).toBe('IEND');
    // Writing again replaces the chunk.
    expect(withPngDpi(png, 96).length).toBe(png.length);
  });
});
