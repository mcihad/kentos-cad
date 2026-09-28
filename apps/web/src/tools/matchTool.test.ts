import { describe, expect, it } from 'vitest';
import { MatchPropertiesTool } from './matchTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Özellik kopyala (docs/adr/0140): the source clicked, then targets one click
 * or one window at a time; layer, colour, line weight and symbol pass,
 * attributes and label do not; each pick is one step through
 * cad.entities.set; a locked layer is refused.
 */
function scene() {
  const h = toolHarness();
  const source = h.add({ kind: 'line', layerId: 'yol', a: pt(0, 0), b: pt(10, 0), color: '#E5484D', lineWeight: 0.5, symbol: 'sembol-1', attrs: { Ad: 'kaynak' }, label: 'K' });
  const t1 = h.add({ kind: 'line', a: pt(0, 5), b: pt(10, 5), attrs: { Ad: 'hedef' }, label: 'H' });
  const t2 = h.add({ kind: 'circle', c: pt(20, 20), r: 3, color: '#00ff00', lineWeight: 0.13 });
  const tool = h.use(new MatchPropertiesTool(h.ctx));
  tool.activate();
  return { h, source, t1, t2, tool };
}

describe('Özellik kopyala', () => {
  it('asks for the source, then gives the clicked object its layer, colour, line weight and symbol, and not its attributes or label', () => {
    const { h, source, t1, tool } = scene();
    expect(tool.prompt.value).toBe('Özellik kopyala: özellikleri alınacak kaynak nesneye tıklayın');
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    expect(tool.prompt.value).toContain('özellikleri alacak nesnelere tıklayın');
    expect(h.said().at(-1)).toBe('Kaynak: “Yol” katmanı, renk #E5484D, 0.50 mm, sembol. Alacak nesnelere tıklayın; sağ tıklayınca biter.');
    h.state.hit = t1;
    const r = recorder();
    tool.pointerMove(at(1, 5));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.pointerDown(at(1, 5));
    tool.pointerUp(at(1, 5));
    expect(h.doc.get(t1.id)).toMatchObject({ layerId: 'yol', color: '#E5484D', lineWeight: 0.5, symbol: 'sembol-1', attrs: { Ad: 'hedef' }, label: 'H' });
    expect(h.said().at(-1)).toBe('1 nesne kaynağın özelliklerini aldı.');
    expect(h.doc.undo()).toBe('Özellik kopyala');
    expect(h.doc.get(t1.id)).toMatchObject({ layerId: 'cizim', attrs: { Ad: 'hedef' } });
    expect(h.doc.get(t1.id)?.color).toBeUndefined();
  });

  it('what the source has none of clears the target’s own (its layer’s again); every click is its own step', () => {
    const h = toolHarness();
    const plain = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const a = h.add({ kind: 'line', a: pt(0, 5), b: pt(10, 5), color: '#00ff00', lineWeight: 0.7, symbol: 'x' });
    const b = h.add({ kind: 'point', p: pt(3, 3), color: '#0000ff' });
    const tool = h.use(new MatchPropertiesTool(h.ctx));
    tool.activate();
    h.state.hit = plain;
    tool.pointerDown(at(1, 0));
    for (const e of [a, b]) {
      h.state.hit = e;
      tool.pointerDown(at(1, 5));
      tool.pointerUp(at(1, 5));
    }
    for (const e of [a, b]) {
      const now = h.doc.get(e.id)!;
      expect(now.color).toBeUndefined();
      expect(now.lineWeight).toBeUndefined();
      expect(now.symbol).toBeUndefined();
    }
    expect(h.doc.undo()).toBe('Özellik kopyala');
    expect(h.doc.get(a.id)?.color).toBeUndefined();
    expect(h.doc.get(b.id)?.color).toBe('#0000ff');
    expect(h.doc.undo()).toBe('Özellik kopyala');
    expect(h.doc.get(a.id)?.color).toBe('#00ff00');
  });

  it('a window gives every object in it the look in one step, skipping the source and locked layers', () => {
    const { h, source, t1, t2, tool } = scene();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 9), b: pt(9, 9) });
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    h.state.inWindow = [source.id, t1.id, t2.id, locked.id];
    tool.pointerDown(at(-5, -5));
    tool.pointerMove(at(40, 40));
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('strokeRect');
    tool.pointerUp(at(40, 40));
    expect(h.doc.get(t1.id)).toMatchObject({ layerId: 'yol', color: '#E5484D' });
    expect(h.doc.get(t2.id)).toMatchObject({ layerId: 'yol', color: '#E5484D', lineWeight: 0.5 });
    expect(h.doc.get(locked.id)).toMatchObject({ layerId: 'kilitli' });
    expect(h.said()).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    expect(h.said().at(-1)).toBe('2 nesne kaynağın özelliklerini aldı.');
    expect(h.doc.undo()).toBe('Özellik kopyala');
    expect(h.doc.get(t2.id)).toMatchObject({ layerId: 'cizim', color: '#00ff00' });
  });

  it('refuses a click on a locked layer with the command’s words and changes nothing', () => {
    const { h, source, tool } = scene();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 9), b: pt(9, 9) });
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    h.state.hit = locked;
    tool.pointerDown(at(1, 9));
    tool.pointerUp(at(1, 9));
    expect(h.said().at(-1)).toMatch(/“Kilitli” katmanı kilitli/);
    expect(h.doc.get(locked.id)).toMatchObject({ layerId: 'kilitli' });
  });

  it('a source on a locked layer cannot give its layer away', () => {
    const h = toolHarness();
    const src = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(10, 0) });
    const t = h.add({ kind: 'line', a: pt(0, 5), b: pt(10, 5) });
    const tool = h.use(new MatchPropertiesTool(h.ctx));
    tool.activate();
    h.state.hit = src;
    tool.pointerDown(at(1, 0));
    h.state.hit = t;
    tool.pointerDown(at(1, 5));
    tool.pointerUp(at(1, 5));
    expect(h.said().at(-1)).toMatch(/nesneler ona taşınamaz/);
    expect(h.doc.get(t.id)).toMatchObject({ layerId: 'cizim' });
  });

  it('the source itself is no target; objects already alike say nothing changed', () => {
    const { h, source, t1, tool } = scene();
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    tool.pointerDown(at(1, 0));
    tool.pointerUp(at(1, 0));
    expect(h.said().at(-1)).toBe('Kaynağın kendisi hedef olamaz; özellik alacak başka bir nesne seçin.');
    h.state.hit = t1;
    tool.pointerDown(at(1, 5));
    tool.pointerUp(at(1, 5));
    tool.pointerDown(at(1, 5));
    tool.pointerUp(at(1, 5));
    expect(h.said().at(-1)).toBe('Hedefler zaten kaynakla aynı özelliklere sahip; bir şey değişmedi.');
    // The second click wrote nothing: one step only.
    expect(h.doc.undo()).toBe('Özellik kopyala');
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('Esc steps back to the source, then leaves; right click or Enter finishes', () => {
    const { h, source, tool } = scene();
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('kaynak nesneye tıklayın');
    expect(tool.cancel()).toBe(false);
    tool.pointerDown(at(1, 0));
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('a click on nothing says what to click; a deleted source asks for a new one', () => {
    const { h, source, t1, tool } = scene();
    h.state.hit = null;
    tool.pointerDown(at(50, 50));
    expect(h.said().at(-1)).toBe('Özellikleri alınacak bir nesneye tıklayın.');
    h.state.hit = source;
    tool.pointerDown(at(1, 0));
    h.state.hit = null;
    tool.pointerDown(at(50, 50));
    tool.pointerUp(at(50, 50));
    expect(h.said().at(-1)).toBe('Özellik alacak bir nesneye tıklayın ya da pencereyle seçin.');
    h.doc.remove([source.id]);
    h.state.hit = t1;
    tool.pointerDown(at(1, 5));
    tool.pointerUp(at(1, 5));
    expect(h.said().at(-1)).toBe('Kaynak nesne çizimde artık yok; yeni bir kaynak seçin.');
    expect(tool.prompt.value).toContain('kaynak nesneye tıklayın');
  });
});
