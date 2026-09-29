import { describe, expect, it } from 'vitest';
import type { Entity, LineEntity } from '../model/entities';
import { PathArrayTool } from './arrayPathTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Yol boyunca dizi (docs/adr/0140): the objects, the path, the count (or a spacing) and
 * Hizala; one write through cad.entities.array with the layout `path`.
 */
const lines = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LineEntity => e.kind === 'line');

/** The values the tool remembers for the session, back at their defaults. */
const defaults = () => Object.assign((PathArrayTool as unknown as { last: object }).last, { count: 5, spacing: 10, bySpacing: false, align: true });

function setup(pathKind: 'line' | 'circle' = 'line') {
  defaults();
  const h = toolHarness();
  const post = h.add({ kind: 'line', a: pt(0, 0), b: pt(0, 1), color: '#E5484D', attrs: { Tür: 'direk' } });
  const path: Entity = h.add(pathKind === 'line' ? { kind: 'line', a: pt(0, 0), b: pt(40, 0) } : { kind: 'circle', c: pt(0, 0), r: 10 });
  h.ctx.selection.set([post.id]);
  const tool = h.use(new PathArrayTool(h.ctx));
  return { h, post, path, tool };
}

describe('Yol boyunca dizi', () => {
  it('asks for the path after the objects, previews the count, and writes the copies along a line, aligned', () => {
    const { h, path, tool, post } = setup();
    tool.activate();
    // A selection skips the picking of objects.
    expect(tool.prompt.value).toContain('1 nesne için yolu seçin');
    // The click picks the path, on any layer.
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toContain('adedi yazın');
    expect(tool.prompt.value).toContain('5 adet');
    expect(tool.prompt.value).toContain('Aralık (A) / Hizala (H): açık / Uygula (Enter)');
    const r = recorder();
    tool.pointerMove(at(20, 5));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fillText');
    expect(tool.input('6')).toBe(true);
    expect(tool.prompt.value).toContain('6 adet');
    expect(lines(h)).toHaveLength(2);
    tool.confirm();
    // Six places: the original and five copies, from the start of the path to its end.
    const copies = lines(h).filter((l) => l.id !== post.id && l.id !== path.id);
    expect(copies).toHaveLength(5);
    expect(copies.map((c) => c.a.x).sort((a, b) => a - b).map((x) => Math.round(x))).toEqual([8, 16, 24, 32, 40]);
    // Copies keep the data.
    expect(copies.every((c) => c.color === '#E5484D' && c.attrs.Tür === 'direk')).toBe(true);
    expect(h.said().at(-1)).toBe('Yol boyunca dizi: 6 adet, 5 yeni nesne.');
    expect(h.doc.undo()).toBe('Yol boyunca dizi');
    expect(lines(h)).toHaveLength(2);
    expect(h.state.exited).toBe(1);
  });

  it('Hizala turns the copies with the path; switched off they only move', () => {
    const { h, tool, path, post } = setup('circle');
    tool.activate();
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    tool.input('4');
    tool.input('H');
    expect(tool.prompt.value).toContain('Hizala (H): kapalı');
    tool.confirm();
    const copy = lines(h).find((l) => l.id !== post.id)!;
    // Moved, not turned: the copy is still vertical.
    expect(copy.b.x - copy.a.x).toBeCloseTo(0);
    expect(copy.b.y - copy.a.y).toBeCloseTo(1);
    h.doc.undo();
    // Turned back on (a circle's copy is turned with it).
    const again = new PathArrayTool(h.ctx);
    h.use(again);
    again.activate();
    h.state.hit = path;
    again.pointerDown(at(10, 0));
    again.input('H');
    expect(again.prompt.value).toContain('Hizala (H): açık');
    again.confirm();
    const turned = lines(h).filter((l) => l.id !== post.id);
    expect(turned).toHaveLength(3);
    expect(turned.some((l) => Math.abs(l.b.x - l.a.x) > 0.5)).toBe(true);
  });

  it('Aralık: as many copies as fit at the typed spacing; N goes back to the count', () => {
    const { h, tool, path, post } = setup();
    tool.activate();
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    expect(tool.input('A')).toBe(true);
    expect(tool.prompt.value).toContain('aralığı yazın');
    expect(tool.prompt.value).toContain('Adet (N)');
    expect(tool.input('12')).toBe(true);
    // 40 m at 12 m: places at 0, 12, 24, 36.
    expect(tool.prompt.value).toContain('aralık 12.000 m; 4 adet');
    tool.confirm();
    const copies = lines(h).filter((l) => l.id !== post.id && l.id !== path.id);
    expect(copies.map((c) => Math.round(c.a.x)).sort((a, b) => a - b)).toEqual([12, 24, 36]);
    expect(h.said().at(-1)).toBe('Yol boyunca dizi: 4 adet, 12.000 m aralıkla, 3 yeni nesne.');
    h.doc.undo();
    // The spacing is kept for the next time; N is the way back to the count.
    const next = h.use(new PathArrayTool(h.ctx));
    next.activate();
    h.state.hit = path;
    next.pointerDown(at(10, 0));
    expect(next.prompt.value).toContain('aralığı yazın');
    expect(next.input('N')).toBe(true);
    expect(next.prompt.value).toContain('adedi yazın');
  });

  it('refuses a bad count, a bad spacing, and a spacing longer than the path', () => {
    const { h, tool, path } = setup();
    tool.activate();
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    expect(tool.input('1')).toBe(true);
    expect(h.said().at(-1)).toBe('Adet 2 ile 10 000 arasında bir tam sayı olmalı.');
    expect(tool.input('2.5')).toBe(true);
    tool.input('A');
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Aralık sıfırdan büyük olmalı.');
    tool.input('60');
    tool.confirm();
    expect(h.said().at(-1)).toMatch(/^Aralık yolun boyundan \(40.000 m\) uzun/);
    expect(lines(h)).toHaveLength(2);
  });

  it('a path that is one of the copied objects is refused', () => {
    const { h, tool, post } = setup();
    tool.activate();
    h.state.hit = post;
    tool.pointerDown(at(0, 0));
    expect(h.said().at(-1)).toMatch(/^Yol, çoğaltılacak nesnelerden biri olamaz/);
    expect(tool.prompt.value).toContain('yolu seçin');
  });

  it('a click on something that is no path says so; the path must have a length', () => {
    const { h, tool } = setup();
    tool.activate();
    const text = h.add({ kind: 'text', p: pt(5, 5), text: 'x', height: 1, rotation: 0 });
    h.state.hit = text;
    tool.pointerDown(at(5, 5));
    expect(h.said().at(-1)).toBe('Yol olarak bir çizgiye, yaya, daireye ya da çoklu çizgiye tıklayın.');
    const dot = h.add({ kind: 'line', a: pt(3, 3), b: pt(3, 3) });
    h.state.hit = dot;
    tool.pointerDown(at(3, 3));
    expect(h.said().at(-1)).toBe('Bu nesnenin uzunluğu yok; başka bir yol seçin.');
  });

  it('a circle path with the count fills the circle', () => {
    const { h, tool, path } = setup('circle');
    tool.activate();
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    tool.input('8');
    tool.confirm();
    // The original and seven copies.
    expect(lines(h)).toHaveLength(8);
  });

  it('objects on a locked layer are not copied, and it says so', () => {
    const h = toolHarness();
    const post = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(0, 1) });
    const path = h.add({ kind: 'line', a: pt(0, 0), b: pt(40, 0) });
    h.ctx.selection.set([post.id]);
    const tool = h.use(new PathArrayTool(h.ctx));
    tool.activate();
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    tool.confirm();
    expect(lines(h)).toHaveLength(2);
    expect(h.said().at(-1)).toMatch(/kilitli/);
    expect(h.state.exited).toBe(0);
  });

  it('Esc steps back: the path, then the objects, then it leaves; picking first works too', () => {
    defaults();
    const h = toolHarness();
    const path = h.add({ kind: 'line', a: pt(0, 0), b: pt(40, 0) });
    const post = h.add({ kind: 'line', a: pt(0, 0), b: pt(0, 1) });
    const tool = h.use(new PathArrayTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    h.ctx.selection.set([post.id]);
    tool.confirm();
    expect(tool.prompt.value).toContain('yolu seçin');
    h.state.hit = path;
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toContain('adedi yazın');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('yolu seçin');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(tool.cancel()).toBe(false);
  });
});
