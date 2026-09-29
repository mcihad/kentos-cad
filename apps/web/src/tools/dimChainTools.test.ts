import { describe, expect, it } from 'vitest';
import type { DimensionEntity } from '../model/entities';
import { DimensionTool } from './dimensionTool';
import { DimBaselineTool, DimContinueTool, forgetDimension } from './dimChainTools';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Zincir ölçü and Baz ölçü (docs/adr/0140): more dimensions along the last aligned or
 * linear one; each click one dimension, one undo step each.
 */
const dims = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is DimensionEntity => e.kind === 'dimension');

/** An aligned dimension drawn by the Ölçülendirme tool: the base the chain tools remember. */
function drawBase(h: ReturnType<typeof toolHarness>, a = pt(0, 0), b = pt(10, 0), through = pt(5, 3)) {
  const tool = h.use(new DimensionTool(h.ctx));
  tool.activate();
  tool.pointerDown(at(a.x, a.y));
  tool.pointerDown(at(b.x, b.y));
  tool.pointerDown(at(through.x, through.y));
  return dims(h).at(-1)!;
}

describe('Zincir ölçü', () => {
  it('continues from the dimension just drawn: each click adds one dimension from the point before, on the same line', () => {
    forgetDimension();
    const h = toolHarness();
    const base = drawBase(h);
    expect(base.offset).toBeCloseTo(3);
    const tool = h.use(new DimContinueTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Son çizilen ölçüden devam ediliyor.');
    expect(tool.prompt.value).toContain('sonraki ölçünün noktasını belirtin');
    expect(tool.prompt.value).toContain('Ölçü seç (S) / Bitir (Enter)');
    const r = recorder();
    tool.pointerMove(at(25, 0));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    expect(dims(h)).toHaveLength(1);
    tool.pointerDown(at(25, 0));
    tool.pointerDown(at(31, 0));
    const [, second, third] = dims(h);
    expect(second).toMatchObject({ a: pt(10, 0), b: pt(25, 0), style: 'linear', angle: 0, height: base.height });
    expect(second.offset).toBeCloseTo(3);
    expect(third).toMatchObject({ a: pt(25, 0), b: pt(31, 0) });
    expect(h.said().filter((s) => s.startsWith('Zincir ölçü eklendi'))).toHaveLength(2);
    // One step each, named after the tool.
    expect(h.doc.undo()).toBe('Zincir ölçü');
    expect(dims(h)).toHaveLength(2);
    // Right click ends and says how many.
    tool.confirm();
    expect(h.said().at(-1)).toBe('Zincir ölçü: 2 ölçü eklendi.');
    expect(h.state.exited).toBe(1);
  });

  it('asks to click a dimension when none was drawn this session, and takes only aligned or linear ones', () => {
    forgetDimension();
    const h = toolHarness();
    const base = h.add({ kind: 'dimension', a: pt(0, 0), b: pt(4, 0), offset: 2, height: 0.5 });
    const radius = h.add({ kind: 'dimension', a: pt(0, 0), b: pt(4, 0), offset: 0, height: 0.5, style: 'radius' });
    const tool = h.use(new DimContinueTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('devam edilecek hizalı ya da doğrusal ölçüye tıklayın');
    // A point is no dimension.
    tool.pointerDown(at(9, 9));
    expect(h.said().at(-1)).toMatch(/^Hizalı ya da doğrusal bir ölçüye tıklayın/);
    h.state.hit = radius;
    tool.pointerDown(at(1, 1));
    expect(h.said().at(-1)).toMatch(/^Hizalı ya da doğrusal bir ölçüye tıklayın/);
    h.state.hit = base;
    tool.pointerDown(at(1, 2));
    expect(tool.prompt.value).toContain('sonraki ölçünün noktasını belirtin');
    tool.pointerDown(at(9, 0));
    expect(dims(h).at(-1)).toMatchObject({ a: pt(4, 0), b: pt(9, 0) });
  });

  it('Esc leaves; “Ölçü seç” picks another base, Esc then returns to the one in use', () => {
    forgetDimension();
    const h = toolHarness();
    drawBase(h);
    const other = h.add({ kind: 'dimension', a: pt(0, 20), b: pt(6, 20), offset: 1, height: 0.5 });
    const tool = h.use(new DimContinueTool(h.ctx));
    tool.activate();
    expect(tool.input('S')).toBe(true);
    expect(tool.prompt.value).toContain('devam edilecek hizalı ya da doğrusal ölçüye tıklayın');
    expect(tool.prompt.value).toContain('Vazgeç (Esc)');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('sonraki ölçünün noktasını belirtin');
    tool.input('S');
    h.state.hit = other;
    tool.pointerDown(at(1, 20));
    tool.pointerDown(at(10, 20));
    expect(dims(h).at(-1)).toMatchObject({ a: pt(6, 20), b: pt(10, 20) });
    expect(tool.cancel()).toBe(false);
  });

  it('a point on the last one makes no dimension and says so; nothing is written on a locked layer', () => {
    forgetDimension();
    const h = toolHarness();
    drawBase(h);
    const tool = h.use(new DimContinueTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(10, 0));
    expect(h.said().at(-1)).toMatch(/^Bu noktadan ölçü oluşmuyor/);
    expect(dims(h)).toHaveLength(1);
    h.doc.layers.active.set('kilitli');
    tool.pointerDown(at(20, 0));
    expect(dims(h)).toHaveLength(1);
    expect(h.said().at(-1)).toMatch(/kilitli/);
  });
});

describe('Baz ölçü', () => {
  it('measures every point from the base’s first point, the lines three text heights apart, one level more each', () => {
    forgetDimension();
    const h = toolHarness();
    const base = drawBase(h);
    const tool = h.use(new DimBaselineTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(18, 0));
    tool.pointerDown(at(30, 0));
    const [, first, second] = dims(h);
    expect(first).toMatchObject({ a: pt(0, 0), b: pt(18, 0), style: 'linear', angle: 0 });
    expect(second).toMatchObject({ a: pt(0, 0), b: pt(30, 0) });
    const spacing = 3 * base.height;
    expect(first.offset).toBeCloseTo(base.offset + spacing);
    expect(second.offset).toBeCloseTo(base.offset + 2 * spacing);
    expect(h.said().filter((s) => s.startsWith('Baz ölçü eklendi'))).toHaveLength(2);
    expect(h.doc.undo()).toBe('Baz ölçü');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Baz ölçü: 2 ölçü eklendi.');
  });

  it('Ctrl+Z takes the last dimension back and gives its level back', () => {
    forgetDimension();
    const h = toolHarness();
    const base = drawBase(h);
    const tool = h.use(new DimBaselineTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(18, 0));
    expect(tool.undoStep()).toBe(true);
    expect(dims(h)).toHaveLength(1);
    tool.pointerDown(at(20, 0));
    expect(dims(h).at(-1)!.offset).toBeCloseTo(base.offset + 3 * base.height);
  });
});
