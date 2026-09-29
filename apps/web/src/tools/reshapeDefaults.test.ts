import { describe, expect, it } from 'vitest';
import type { LineEntity, PolylineEntity } from '../model/entities';
import { ChamferAllTool, FilletAllTool, SimplifyTool } from './reshapeTools';
import { SplitTool } from './splitTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * The values of the reshaping tools before anything was typed (docs/adr/0140): they do not
 * ask with nothing to go by, they preview and write with the same values as the desktop.
 * Each test is the first of its tool in this file, so the value is the untouched default.
 */
const square = { kind: 'polygon' as const, pts: [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)] };

describe('defaults', () => {
  it('Tüm köşeleri yuvarla: 1 m, previewed at once and written by Enter', () => {
    const h = toolHarness();
    const e = h.add(square);
    h.ctx.selection.set([e.id]);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('yarıçap 1.000 m');
    expect(tool.prompt.value).toContain('Uygula (Enter)');
    const r = recorder();
    tool.pointerMove(at(5, 5));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.confirm();
    const done = h.doc.get(e.id) as PolylineEntity;
    expect(done.pts).toHaveLength(8);
    // A rounding of a metre on a right angle: the arc's tangent points a metre from the corner.
    expect(done.pts.some((p) => Math.abs(p.x - 1) < 1e-9 && p.y === 0)).toBe(true);
    expect(h.said().at(-1)).toBe('4 köşe yuvarlandı.');
  });

  it('Tüm köşelere pah: 1 m on both edges', () => {
    const h = toolHarness();
    const e = h.add(square);
    h.ctx.selection.set([e.id]);
    const tool = h.use(new ChamferAllTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('mesafe 1.000 m');
    tool.confirm();
    const done = h.doc.get(e.id) as PolylineEntity;
    expect(done.pts).toHaveLength(8);
    expect(done.pts.some((p) => Math.abs(p.x - 1) < 1e-9 && p.y === 0)).toBe(true);
    expect(h.said().at(-1)).toBe('4 köşeye pah kırıldı.');
  });

  it('Sadeleştir: 1 cm', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0.005), pt(10, 0), pt(10, 10)] });
    h.ctx.selection.set([e.id]);
    const tool = h.use(new SimplifyTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('tolerans 0.010 m');
    tool.confirm();
    expect((h.doc.get(e.id) as PolylineEntity).pts).toEqual([pt(0, 0), pt(10, 0), pt(10, 10)]);
    expect(h.said().at(-1)).toMatch(/^Sadeleştir: 1 köşe atıldı/);
  });

  it('Parçala: 4 equal parts, and pieces of 10 m', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(40, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('E');
    h.state.hit = a;
    tool.pointerDown(at(3, 0));
    expect(tool.prompt.value).toContain('4 parça');
    tool.confirm();
    const lines = () => [...h.doc.all()].filter((e): e is LineEntity => e.kind === 'line');
    expect(lines()).toHaveLength(4);
    h.doc.undo();
    tool.input('U');
    tool.pointerDown(at(3, 0));
    expect(tool.prompt.value).toContain('parça 10.000 m');
    tool.confirm();
    expect(lines().map((l) => l.b.x - l.a.x)).toEqual([10, 10, 10, 10]);
  });
});
