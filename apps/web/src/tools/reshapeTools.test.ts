import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { ChamferAllTool, FilletAllTool, removedVertices, ReverseTool, SimplifyTool } from './reshapeTools';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Tüm köşeleri yuvarla, Tüm köşelere pah, Sadeleştir and Yönü çevir
 * (docs/adr/0140): the value typed and previewed, one edit through
 * cad.entities.edit, locked layers left alone. Over a document and a log,
 * without a view.
 */
const square = (size = 10, layerId = 'cizim') => ({ kind: 'polygon' as const, layerId, pts: [pt(0, 0), pt(size, 0), pt(size, size), pt(0, size)] });

function selected(h: ReturnType<typeof toolHarness>, ...es: Entity[]) {
  h.ctx.selection.set(es.map((e) => e.id));
}

describe('Tüm köşeleri yuvarla', () => {
  it('asks for the radius with the objects selected, previews it, and rounds every corner in one step on Enter', () => {
    const h = toolHarness();
    const e = h.add({ ...square(), color: '#E5484D', attrs: { Ada: '12' } });
    selected(h, e);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    // A selection skips the picking step.
    expect(tool.prompt.value).toContain('yarıçapı yazın');
    expect(tool.input('2')).toBe(true);
    expect(tool.prompt.value).toContain('Uygula (Enter)');
    expect(tool.prompt.value).toContain('yarıçap 2.000 m');
    // Nothing written before it is confirmed.
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(4);
    const r = recorder();
    tool.pointerMove(at(5, 5));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.confirm();
    const done = h.doc.get(e.id) as PolylineEntity;
    expect(done.pts).toHaveLength(8);
    expect(done.bulges?.filter((b) => b !== 0)).toHaveLength(4);
    // Everything but the geometry stays.
    expect(done).toMatchObject({ layerId: 'cizim', color: '#E5484D', attrs: { Ada: '12' } });
    expect(h.said().at(-1)).toBe('4 köşe yuvarlandı.');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Köşe yuvarla');
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(4);
    // One step only: the next one under it is the drawing's own “Ekle”.
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('picks first when nothing is selected: the click toggles, right click goes on to the value', () => {
    const h = toolHarness();
    const e = h.add(square());
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    h.state.hit = e;
    tool.pointerDown(at(1, 1));
    tool.pointerUp(at(1, 1));
    expect(h.ctx.selection.has(e.id)).toBe(true);
    tool.confirm();
    expect(tool.prompt.value).toContain('yarıçapı yazın');
    tool.input('1');
    tool.confirm();
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(8);
  });

  it('a click in the drawing applies, and the value is remembered by the next start', () => {
    const h = toolHarness();
    const a = h.add(square());
    selected(h, a);
    const first = h.use(new FilletAllTool(h.ctx));
    first.activate();
    first.input('1.5');
    first.pointerDown(at(30, 30));
    expect((h.doc.get(a.id) as PolylineEntity).pts).toHaveLength(8);
    const b = h.add(square(20));
    selected(h, b);
    const second = h.use(new FilletAllTool(h.ctx));
    second.activate();
    expect(second.prompt.value).toContain('yarıçap 1.500 m');
    // Enter with nothing typed uses it.
    second.confirm();
    expect((h.doc.get(b.id) as PolylineEntity).pts).toHaveLength(8);
  });

  it('says how many corners did not fit and changes nothing when none does', () => {
    const h = toolHarness();
    const e = h.add(square());
    selected(h, e);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    tool.input('20');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Hiçbir köşe yuvarlanamadı: 4 köşe sığmadığı ya da yaya komşu olduğu için atlandı. Daha küçük bir yarıçap yazın.');
    expect(h.doc.canUndo.value).toBe(true); // only the drawing's own “Ekle”
    expect(h.doc.undo()).toBe('Ekle');
    expect(h.state.exited).toBe(0);
  });

  it('counts the corners left when some fit and some do not', () => {
    const h = toolHarness();
    // A 10 × 1 strip: a 2 m radius does not fit the short edges.
    const e = h.add({ kind: 'polygon', layerId: 'cizim', pts: [pt(0, 0), pt(10, 0), pt(10, 1), pt(0, 1)] });
    const big = h.add(square(20));
    selected(h, e, big);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    tool.input('2');
    tool.confirm();
    expect(h.said().at(-1)).toBe('4 köşe yuvarlandı; 4 köşe sığmadığı ya da yaya komşu olduğu için atlandı.');
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(4);
  });

  it('leaves objects on a locked layer and kinds without corners out, saying so', () => {
    const h = toolHarness();
    const locked = h.add(square(10, 'kilitli'));
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 5) });
    const ok = h.add(square());
    selected(h, locked, line, ok);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual(expect.arrayContaining(['1 nesne çoklu çizgi ya da alan olmadığı için atlandı.', '1 nesne kilitli katmanda olduğu için atlandı.']));
    tool.input('1');
    tool.confirm();
    expect((h.doc.get(ok.id) as PolylineEntity).pts).toHaveLength(8);
    expect((h.doc.get(locked.id) as PolylineEntity).pts).toHaveLength(4);
  });

  it('a selection with nothing to round goes back to picking and writes nothing', () => {
    const h = toolHarness();
    const locked = h.add(square(10, 'kilitli'));
    selected(h, locked);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('Esc steps back from the value to the picking, then leaves', () => {
    const h = toolHarness();
    const e = h.add(square());
    selected(h, e);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    tool.input('1');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(tool.cancel()).toBe(false);
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(4);
  });

  it('refuses a radius not above zero and a text it does not understand', () => {
    const h = toolHarness();
    selected(h, h.add(square()));
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Yarıçap sıfırdan büyük olmalı.');
    expect(tool.input('abc')).toBe(false);
  });

  it('rounds the holes of an area too', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polygon', layerId: 'cizim', pts: [pt(0, 0), pt(20, 0), pt(20, 20), pt(0, 20)], holes: [{ pts: [pt(5, 5), pt(15, 5), pt(15, 15), pt(5, 15)] }] });
    selected(h, e);
    const tool = h.use(new FilletAllTool(h.ctx));
    tool.activate();
    tool.input('1');
    tool.confirm();
    const done = h.doc.get(e.id) as PolylineEntity;
    expect(done.pts).toHaveLength(8);
    expect(done.holes?.[0].pts).toHaveLength(8);
    expect(h.said().at(-1)).toBe('8 köşe yuvarlandı.');
  });
});

describe('Tüm köşelere pah', () => {
  it('cuts every corner with d, or d1 and d2, as one step “Pah”', () => {
    const h = toolHarness();
    const e = h.add(square());
    selected(h, e);
    const tool = h.use(new ChamferAllTool(h.ctx));
    tool.activate();
    expect(tool.input('1,2')).toBe(true);
    expect(tool.prompt.value).toContain('mesafe 1.000 ile 2.000 m');
    tool.confirm();
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(8);
    expect(h.said().at(-1)).toBe('4 köşeye pah kırıldı.');
    expect(h.doc.undo()).toBe('Pah');
  });

  it('says so when the distance fits no corner', () => {
    const h = toolHarness();
    const e = h.add(square());
    selected(h, e);
    const tool = h.use(new ChamferAllTool(h.ctx));
    tool.activate();
    tool.input('30');
    tool.confirm();
    expect(h.said().at(-1)).toMatch(/^Hiçbir köşeye pah kırılamadı: 4 köşe/);
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(4);
  });

  it('refuses zero distances', () => {
    const h = toolHarness();
    selected(h, h.add(square()));
    const tool = h.use(new ChamferAllTool(h.ctx));
    tool.activate();
    expect(tool.input('0,1')).toBe(true);
    expect(h.said().at(-1)).toBe('Pah mesafeleri sıfırdan büyük olmalı.');
  });
});

describe('Sadeleştir', () => {
  const wiggly = () => ({ kind: 'polyline' as const, layerId: 'cizim', pts: [pt(0, 0), pt(5, 0.004), pt(10, 0), pt(15, 0.006), pt(20, 0), pt(20, 10)] });

  it('marks the vertices that go, and drops them within the tolerance with the largest deviation told', () => {
    const h = toolHarness();
    const e = h.add(wiggly());
    selected(h, e);
    const tool = h.use(new SimplifyTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('toleransı yazın (m)');
    tool.input('0.01');
    const r = recorder();
    tool.pointerMove(at(3, 3));
    tool.draw(r.g, r.view);
    expect(r.calls.filter((c) => c === 'arc').length).toBeGreaterThanOrEqual(3);
    tool.confirm();
    expect((h.doc.get(e.id) as PolylineEntity).pts).toEqual([pt(0, 0), pt(20, 0), pt(20, 10)]);
    expect(h.said().at(-1)).toBe('Sadeleştir: 3 köşe atıldı; en büyük sapma 0.006 m.');
    expect(h.doc.undo()).toBe('Sadeleştir');
  });

  it('a tolerance that removes nothing says so and writes nothing', () => {
    const h = toolHarness();
    selected(h, h.add(wiggly()));
    const tool = h.use(new SimplifyTool(h.ctx));
    tool.activate();
    tool.input('0.0001');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.');
    expect(h.doc.undo()).toBe('Ekle');
    expect(h.doc.canUndo.value).toBe(false);
  });

  it('is refused on a locked layer', () => {
    const h = toolHarness();
    const e = h.add({ ...wiggly(), layerId: 'kilitli' });
    selected(h, e);
    const tool = h.use(new SimplifyTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(h.said()).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    expect((h.doc.get(e.id) as PolylineEntity).pts).toHaveLength(6);
  });

  it('the vertices a simplification dropped are the ones the result lacks', () => {
    const h = toolHarness();
    const before = h.add(wiggly());
    const after = { ...before, pts: [pt(0, 0), pt(20, 0), pt(20, 10)] } as Entity;
    expect(removedVertices(before, after)).toEqual([pt(5, 0.004), pt(10, 0), pt(15, 0.006)]);
  });
});

describe('Yönü çevir', () => {
  it('turns the selected objects round at once, as one step, and skips those without a direction', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0) });
    const poly = h.add({ kind: 'polyline', layerId: 'cizim', pts: [pt(0, 0), pt(5, 0), pt(5, 5)] });
    const arc = h.add({ kind: 'arc', c: pt(0, 0), r: 2, a0: 0, a1: 1 });
    selected(h, line, poly, arc);
    const tool = h.use(new ReverseTool(h.ctx));
    tool.activate();
    expect(h.doc.get(line.id)).toMatchObject({ a: pt(5, 0), b: pt(0, 0) });
    expect((h.doc.get(poly.id) as PolylineEntity).pts).toEqual([pt(5, 5), pt(5, 0), pt(0, 0)]);
    expect(h.said()).toContain('1 nesnenin yönü olmadığı için atlandı (yay, elips, daire, nokta, yazı…).');
    expect(h.said().at(-1)).toBe('2 nesnenin yönü çevrildi.');
    expect(h.doc.undo()).toBe('Yönü çevir');
    expect(h.doc.get(line.id)).toMatchObject({ a: pt(0, 0), b: pt(5, 0) });
  });

  it('shows the new direction with arrows while picking, and turns the picked objects on Enter', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(50, 0) });
    const tool = h.use(new ReverseTool(h.ctx));
    tool.activate();
    h.ctx.selection.set([line.id]);
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fill');
    expect(h.doc.get(line.id)).toMatchObject({ a: pt(0, 0) });
    tool.confirm();
    expect(h.doc.get(line.id)).toMatchObject({ a: pt(50, 0), b: pt(0, 0) });
  });

  it('says plainly when nothing selected has a direction, and writes nothing', () => {
    const h = toolHarness();
    const c = h.add({ kind: 'circle', c: pt(0, 0), r: 3 });
    selected(h, c);
    h.use(new ReverseTool(h.ctx)).activate();
    expect(h.said().at(-1)).toMatch(/^Seçili nesnelerin yönü çevrilemez/);
    expect(h.doc.undo()).toBe('Ekle');
    expect(h.doc.canUndo.value).toBe(false);
  });

  it('leaves an object on a locked layer as it is', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(5, 0) });
    selected(h, line);
    h.use(new ReverseTool(h.ctx)).activate();
    expect(h.said()).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    expect(h.doc.get(line.id)).toMatchObject({ a: pt(0, 0), b: pt(5, 0) });
  });
});
