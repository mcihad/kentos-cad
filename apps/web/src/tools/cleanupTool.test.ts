import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { CleanupTool, repeatedIn } from './cleanupTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Çizimi temizle (docs/adr/0140): the finding shown first, Enter cleans in
 * one step; the selection or the whole drawing; locked layers left alone.
 */
function messy() {
  const h = toolHarness();
  const keep = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
  const repeat = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
  const empty = h.add({ kind: 'line', a: pt(4, 4), b: pt(4, 4) });
  const poly = h.add({ kind: 'polyline', layerId: 'cizim', pts: [pt(0, 5), pt(5, 5), pt(5, 5), pt(9, 5), pt(9, 5), pt(9, 5)] });
  return { h, keep, repeat, empty, poly };
}

describe('Çizimi temizle', () => {
  it('scans the whole drawing when nothing is selected, tells what it found, and cleans on Enter as one step', () => {
    const { h, keep, repeat, empty, poly } = messy();
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Çizimi temizle: 1 yinelenen, 1 boş nesne, 3 tekrarlanan köşe. Enter ile temizleyin.');
    expect(tool.prompt.value).toContain('bütün çizim: 1 yinelenen, 1 boş nesne, 3 tekrarlanan köşe');
    expect(tool.prompt.value).toContain('Temizle (Enter)');
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    // Nothing yet.
    expect(h.doc.size).toBe(4);
    tool.confirm();
    expect(h.doc.get(keep.id)).toBeDefined();
    expect(h.doc.get(repeat.id)).toBeUndefined();
    expect(h.doc.get(empty.id)).toBeUndefined();
    expect((h.doc.get(poly.id) as PolylineEntity).pts).toEqual([pt(0, 5), pt(5, 5), pt(9, 5)]);
    expect(h.said().at(-1)).toBe('Çizimi temizle: 1 yinelenen nesne silindi, 1 boş nesne silindi, 3 tekrarlanan köşe atıldı.');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Çizimi temizle');
    expect(h.doc.size).toBe(4);
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('scans only the selection when there is one', () => {
    const { h, keep, repeat, poly } = messy();
    h.ctx.selection.set([keep.id, poly.id]);
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('2 seçili nesne: 3 tekrarlanan köşe');
    tool.confirm();
    // The repeat is outside the scope: kept.
    expect(h.doc.get(repeat.id)).toBeDefined();
    expect((h.doc.get(poly.id) as PolylineEntity).pts).toHaveLength(3);
  });

  it('a click changes the scope and the finding follows', () => {
    const { h, keep, repeat } = messy();
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    h.state.hit = keep;
    tool.pointerDown(at(1, 0));
    tool.pointerUp(at(1, 0));
    expect(tool.prompt.value).toContain('1 seçili nesne: temizlenecek bir şey yok');
    h.state.hit = repeat;
    tool.pointerDown(at(1, 0));
    tool.pointerUp(at(1, 0));
    expect(tool.prompt.value).toContain('2 seçili nesne: 1 yinelenen');
  });

  it('a clean drawing says so plainly and changes nothing', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toMatch(/^Çizimi temizle: temizlenecek bir şey yok/);
    tool.confirm();
    expect(h.said().at(-1)).toBe('Çizimi temizle: temizlenecek bir şey yok; hiçbir şey değişmedi.');
    expect(h.doc.undo()).toBe('Ekle');
    expect(h.doc.canUndo.value).toBe(false);
  });

  it('leaves what is on a locked layer alone', () => {
    const h = toolHarness();
    h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(10, 0) });
    const twin = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('temizlenecek bir şey yok');
    expect(h.said().at(-1)).toContain('2 nesne kilitli katmanda, atlandı');
    tool.confirm();
    expect(h.doc.get(twin.id)).toBeDefined();
  });

  it('the same geometry on another layer is not a repeat', () => {
    const h = toolHarness();
    h.add({ kind: 'line', layerId: 'cizim', a: pt(0, 0), b: pt(10, 0) });
    h.add({ kind: 'line', layerId: 'yol', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new CleanupTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('temizlenecek bir şey yok');
  });
});

describe('the vertices Çizimi temizle marks', () => {
  it('are those repeated in a row, in every part of an area and its holes (docs/adr/0143)', () => {
    const h = toolHarness();
    const area = h.add({
      kind: 'polygon',
      pts: [pt(0, 0), pt(10, 0), pt(10, 0), pt(10, 10)],
      holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4), pt(4, 4)] }],
      parts: [{ pts: [pt(20, 0), pt(30, 0), pt(30, 10), pt(30, 10), pt(20, 10)], holes: [{ pts: [pt(22, 2), pt(22, 2), pt(24, 4)] }] }],
    });
    expect(repeatedIn(area)).toEqual([pt(10, 0), pt(4, 4), pt(30, 10), pt(22, 2)]);
  });
});
