import { describe, expect, it } from 'vitest';
import type { LineEntity, PolylineEntity } from '../model/entities';
import { SplitTool } from './splitTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Parçala (docs/adr/0140): from the crossings of the selected objects, in
 * equal parts, or by length from an end. One edit through cad.entities.edit,
 * the first piece keeping the object's place and persistent id.
 */
const lines = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LineEntity => e.kind === 'line');
const len = (l: LineEntity) => Math.hypot(l.b.x - l.a.x, l.b.y - l.a.y);

describe('Parçala: Kesişimlerden', () => {
  it('cuts the selected objects where they cross, previews the pieces, and writes them as one step on Enter', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0), color: '#E5484D', attrs: { Ad: 'A' }, label: 'yol' });
    const b = h.add({ kind: 'line', a: pt(5, -5), b: pt(5, 5) });
    h.ctx.selection.set([a.id, b.id]);
    const uid = h.doc.uidOf(a.id);
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('2 nesne 4 parçaya bölünecek');
    expect(tool.prompt.value).toContain('Eşit parçalara (E)');
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    expect(lines(h)).toHaveLength(2);
    tool.confirm();
    expect(lines(h)).toHaveLength(4);
    // The first piece keeps the slot and the persistent id; every piece keeps the data.
    expect(h.doc.uidOf(a.id)).toBe(uid);
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(0, 0), b: pt(5, 0), color: '#E5484D', attrs: { Ad: 'A' }, label: 'yol' });
    expect(lines(h).filter((l) => l.color === '#E5484D')).toHaveLength(2);
    expect(h.said().at(-1)).toBe('2 nesne 4 parçaya bölündü.');
    // Everything that came of the cut is selected.
    expect(h.ctx.selection.size).toBe(4);
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Parçala');
    expect(lines(h)).toHaveLength(2);
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('picks first when nothing is selected, then goes on to the preview with Enter', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const b = h.add({ kind: 'line', a: pt(5, -5), b: pt(5, 5) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(tool.prompt.value).toContain('Eşit parçalara (E) / Uzunluktan (U)');
    h.ctx.selection.set([a.id, b.id]);
    tool.confirm();
    expect(tool.prompt.value).toContain('4 parçaya bölünecek');
    tool.confirm();
    expect(lines(h)).toHaveLength(4);
  });

  it('objects that do not cross say so and write nothing; Esc goes back to the picking', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const b = h.add({ kind: 'line', a: pt(0, 5), b: pt(10, 5) });
    h.ctx.selection.set([a.id, b.id]);
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('birbirini kesmiyor');
    tool.confirm();
    expect(h.said().at(-1)).toMatch(/^Seçilen nesneler birbirini kesmiyor; bölünecek yer yok/);
    expect(lines(h)).toHaveLength(2);
    expect(h.state.exited).toBe(0);
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    expect(tool.cancel()).toBe(false);
  });

  it('leaves the objects on a locked layer out and says so', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(10, 0) });
    const b = h.add({ kind: 'line', a: pt(5, -5), b: pt(5, 5) });
    h.ctx.selection.set([a.id, b.id]);
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.confirm();
    expect(h.said().at(-1)).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    // Only the unlocked object is a candidate, and it has nothing to cross.
    expect(lines(h)).toHaveLength(2);
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(0, 0), b: pt(10, 0) });
  });
});

describe('Parçala: Eşit parçalara', () => {
  it('cuts the clicked object into the typed number of equal parts and stays for the next object', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(12, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    expect(tool.input('E')).toBe(true);
    expect(tool.prompt.value).toContain('bölünecek nesneye tıklayın');
    h.state.hit = a;
    tool.pointerDown(at(3, 0));
    expect(tool.prompt.value).toContain('parça sayısını yazın');
    expect(tool.input('3')).toBe(true);
    const r = recorder();
    tool.pointerMove(at(3, 0));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    // Typed input previews; Enter writes.
    expect(lines(h)).toHaveLength(1);
    tool.confirm();
    expect(lines(h).map(len)).toEqual([4, 4, 4]);
    expect(h.said().at(-1)).toBe('Nesne 3 eşit parçaya bölündü.');
    expect(h.state.exited).toBe(0);
    expect(tool.prompt.value).toContain('bölünecek nesneye tıklayın');
    expect(h.doc.undo()).toBe('Parçala');
    expect(lines(h)).toHaveLength(1);
  });

  it('a count outside 2 to 10 000 is refused; Esc drops the object, then leaves', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(12, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('E');
    h.state.hit = a;
    tool.pointerDown(at(1, 0));
    expect(tool.input('1')).toBe(true);
    expect(h.said().at(-1)).toBe('Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.');
    expect(tool.input('2.5')).toBe(true);
    expect(tool.cancel()).toBe(true);
    expect(tool.cancel()).toBe(false);
    expect(lines(h)).toHaveLength(1);
  });

  it('an object on a locked layer cannot be clicked, a polygon neither', () => {
    const h = toolHarness();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(12, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('E');
    h.state.hit = locked;
    tool.pointerDown(at(1, 0));
    expect(h.said().at(-1)).toBe('Bölünecek düzenlenebilir bir çizgiye, açık çoklu çizgiye, yaya ya da daireye tıklayın.');
    h.state.hit = h.add({ kind: 'polygon', layerId: 'cizim', pts: [pt(0, 0), pt(4, 0), pt(4, 4)] });
    tool.pointerDown(at(1, 0));
    expect(h.said().at(-1)).toMatch(/^Bölünecek düzenlenebilir/);
  });

  it('cuts an open polyline, the middle piece keeping its corner', () => {
    const h = toolHarness();
    const p = h.add({ kind: 'polyline', layerId: 'cizim', pts: [pt(0, 0), pt(10, 0), pt(10, 10)] });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('E');
    h.state.hit = p;
    tool.pointerDown(at(1, 0));
    tool.input('3');
    tool.confirm();
    // Three pieces of 20 / 3 m: the middle one runs round the corner and stays a polyline.
    const polys = [...h.doc.all()].filter((e): e is PolylineEntity => e.kind === 'polyline');
    expect([...h.doc.all()]).toHaveLength(3);
    expect(polys).toHaveLength(1);
    expect(polys[0].pts).toHaveLength(3);
    expect(polys[0].pts[1]).toEqual(pt(10, 0));
  });
});

describe('Parçala: daire', () => {
  it('a circle comes apart into arcs in two or more parts, and the pieces are arcs', () => {
    const h = toolHarness();
    const c = h.add({ kind: 'circle', c: pt(0, 0), r: 5 });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('E');
    h.state.hit = c;
    tool.pointerDown(at(5, 0));
    tool.input('4');
    const r = recorder();
    tool.draw(r.g, r.view);
    tool.confirm();
    const all = [...h.doc.all()];
    expect(all).toHaveLength(4);
    expect(all.every((e) => e.kind === 'arc')).toBe(true);
    expect(h.said().at(-1)).toBe('Nesne 4 eşit parçaya bölündü.');
  });
});

describe('Parçala: Uzunluktan', () => {
  it('measures from the end nearer the click, the short piece at the other end', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('U');
    expect(tool.prompt.value).toContain('ölçme yakın uçtan başlar');
    h.state.hit = a;
    // Near the end b.
    tool.pointerDown(at(9, 0));
    expect(tool.input('3')).toBe(true);
    tool.confirm();
    // From b: 3, 3, 3 and 1 short at a.
    expect(lines(h).map((l) => Math.round(len(l) * 1000) / 1000)).toEqual([1, 3, 3, 3]);
    expect(h.doc.get(a.id)).toMatchObject({ a: pt(0, 0), b: pt(1, 0) });
    expect(h.said().at(-1)).toBe('Nesne 4 parçaya bölündü: 3.000 m uzunlukta, sondan ölçülü.');
    expect(h.doc.undo()).toBe('Parçala');
  });

  it('measures from the start when the click is nearer it', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('U');
    h.state.hit = a;
    tool.pointerDown(at(1, 0));
    tool.input('4');
    tool.confirm();
    expect(lines(h).map(len)).toEqual([4, 4, 2]);
  });

  it('a length not below the object’s says so and writes nothing', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('U');
    h.state.hit = a;
    tool.pointerDown(at(1, 0));
    tool.input('12');
    tool.confirm();
    expect(h.said().at(-1)).toBe('Uzunluk nesne boyundan küçük olmalı; nesne 10.000 m uzunluğunda, yazılan 12.000 m.');
    expect(lines(h)).toHaveLength(1);
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Parça uzunluğu sıfırdan büyük olmalı.');
  });

  it('a right click with nothing clicked leaves the tool; the options switch between the ways', () => {
    const h = toolHarness();
    const tool = h.use(new SplitTool(h.ctx));
    tool.activate();
    tool.input('U');
    expect(tool.prompt.value).toContain('Kesişimlerden (K) / Eşit parçalara (E)');
    tool.input('E');
    expect(tool.prompt.value).toMatch(/\d+ parça/);
    tool.input('K');
    expect(tool.prompt.value).toContain('nesnelere tıklayın');
    tool.input('E');
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });
});
