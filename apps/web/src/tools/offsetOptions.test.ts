import { describe, expect, it } from 'vitest';
import type { CircleEntity, LineEntity, PolylineEntity } from '../model/entities';
import { OffsetTool } from './edgeTools';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Ötele's two options (docs/adr/0140): İki yana (I) makes the copy on both sides, Kaynağı
 * sil (S) deletes the source in the same step. One cad.entities.edit step, “Ötele”.
 */
const lines = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LineEntity => e.kind === 'line');

/** The options as a fresh session has them. */
function reset(tool: OffsetTool, ...on: string[]) {
  for (const key of ['N', 'I', 'S']) {
    const label = { N: 'Noktadan geç', I: 'İki yana', S: 'Kaynağı sil' }[key]!;
    if (tool.prompt.value.includes(`${label} (${key}): açık`) !== on.includes(key)) tool.input(key);
  }
}

function start(...on: string[]) {
  const h = toolHarness();
  const source = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0), color: '#E5484D', attrs: { Ad: 'eksen' } });
  const tool = h.use(new OffsetTool(h.ctx));
  tool.activate();
  reset(tool, ...on);
  tool.input('2');
  return { h, source, tool };
}

describe('Ötele: seçenekler', () => {
  it('lists the options with their state in the prompt', () => {
    const { tool } = start();
    // In the order N, I, S; a toggle shows its value only when it is on.
    expect(tool.prompt.value).toContain('[Noktadan geç (N) / İki yana (I) / Kaynağı sil (S)]');
    tool.input('I');
    expect(tool.prompt.value).toContain('İki yana (I): açık');
    // The dotted key a Turkish keyboard types is the same option.
    tool.input('İ');
    expect(tool.prompt.value).toContain('İki yana (I) /');
    tool.input('s');
    expect(tool.prompt.value).toContain('Kaynağı sil (S): açık');
  });

  it('İki yana: copies on both sides at the distance, in one step', () => {
    const { h, source, tool } = start('I');
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    expect(tool.prompt.value).toContain('kopyaların gideceği tarafa tıklayın (iki yana çıkar)');
    const r = recorder();
    tool.pointerMove(at(5, 3));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.pointerDown(at(5, 3));
    const ys = lines(h).map((l) => l.a.y).sort((a, b) => a - b);
    expect(ys).toEqual([-2, 0, 2]);
    // The copies take the source's layer and colour.
    expect(lines(h).filter((l) => l.color === '#E5484D')).toHaveLength(3);
    expect(h.said().at(-1)).toBe('2.000 m uzaklıkta iki yana 2 ötelenmiş kopya eklendi.');
    expect(h.doc.undo()).toBe('Ötele');
    expect(lines(h)).toHaveLength(1);
  });

  it('İki yana on a circle: one outside and one inside', () => {
    const h = toolHarness();
    const c = h.add({ kind: 'circle', c: pt(0, 0), r: 10 });
    const tool = h.use(new OffsetTool(h.ctx));
    tool.activate();
    reset(tool, 'I');
    tool.input('3');
    h.state.hit = c;
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(15, 0));
    const radii = [...h.doc.all()].filter((e): e is CircleEntity => e.kind === 'circle').map((e) => e.r).sort((a, b) => a - b);
    expect(radii).toEqual([7, 10, 13]);
  });

  it('İki yana on a bent polyline: both sides', () => {
    const h = toolHarness();
    const p = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(10, 0), pt(10, 10)] });
    const tool = h.use(new OffsetTool(h.ctx));
    tool.activate();
    reset(tool, 'I');
    tool.input('1');
    h.state.hit = p;
    tool.pointerDown(at(5, 0));
    tool.pointerDown(at(5, -4));
    const polys = [...h.doc.all()].filter((e): e is PolylineEntity => e.kind === 'polyline');
    expect(polys).toHaveLength(3);
    const corners = polys.map((q) => q.pts[1]).sort((a, b) => a.x - b.x);
    expect(corners[0].x).toBeCloseTo(9);
    expect(corners[1].x).toBeCloseTo(10);
    expect(corners[2].x).toBeCloseTo(11);
  });

  it('Kaynağı sil: the source goes in the same step; the undo brings it back with its data', () => {
    const { h, source, tool } = start('S');
    const uid = h.doc.uidOf(source.id);
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    const r = recorder();
    tool.pointerMove(at(5, 3));
    tool.draw(r.g, r.view);
    tool.pointerDown(at(5, 3));
    expect(lines(h)).toHaveLength(1);
    expect(lines(h)[0]).toMatchObject({ a: pt(0, 2), b: pt(10, 2), color: '#E5484D' });
    expect(h.doc.get(source.id)).toBeUndefined();
    expect(h.said().at(-1)).toBe('2.000 m ötelenmiş kopya eklendi; kaynak silindi.');
    expect(h.doc.undo()).toBe('Ötele');
    expect(lines(h)).toHaveLength(1);
    expect(h.doc.uidOf(lines(h)[0].id)).toBe(uid);
    expect(lines(h)[0].attrs).toEqual({ Ad: 'eksen' });
  });

  it('both together: two copies and no source', () => {
    const { h, source, tool } = start('I', 'S');
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    tool.pointerDown(at(5, 3));
    expect(lines(h).map((l) => l.a.y).sort((a, b) => a - b)).toEqual([-2, 2]);
    expect(h.said().at(-1)).toBe('2.000 m uzaklıkta iki yana 2 ötelenmiş kopya eklendi; kaynak silindi.');
    expect(h.doc.undo()).toBe('Ötele');
    expect(lines(h)).toHaveLength(1);
  });

  it('with Noktadan geç, İki yana mirrors the distance to the point', () => {
    const { h, source, tool } = start('I');
    // Typing a distance turns “Noktadan geç” off; it is switched on after.
    tool.input('N');
    expect(tool.prompt.value).toContain('Noktadan geç (N): açık');
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    tool.pointerDown(at(5, 3, { snap: null }));
    expect(lines(h).map((l) => l.a.y).sort((a, b) => a - b)).toEqual([-3, 0, 3]);
  });

  it('a side that cannot be made is said while the other is still written', () => {
    const h = toolHarness();
    const c = h.add({ kind: 'circle', c: pt(0, 0), r: 2 });
    const tool = h.use(new OffsetTool(h.ctx));
    tool.activate();
    reset(tool, 'I');
    tool.input('3');
    h.state.hit = c;
    tool.pointerDown(at(2, 0));
    tool.pointerDown(at(9, 0));
    // Outside r = 5 is made; inside would go below zero.
    expect([...h.doc.all()].filter((e) => e.kind === 'circle')).toHaveLength(2);
    expect(h.said().some((s) => s.startsWith('Öbür yandaki kopya oluşmadı'))).toBe(true);
  });

  it('İki yana on a click that lies on the object makes one copy and says so', () => {
    const { h, source, tool } = start('I');
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    tool.pointerDown(at(5, 0));
    expect(lines(h)).toHaveLength(2);
    expect(h.said().at(-1)).toMatch(/^Tıklanan nokta nesnenin üzerinde/);
  });

  it('shows the distance in a tag over an object instead of in the prompt', () => {
    const { h, source, tool } = start();
    expect(tool.prompt.value).not.toContain('mesafe');
    h.state.hit = source;
    tool.pointerMove(at(5, 0));
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fillText');
  });

  it('nothing on a locked layer is offset, deleted or copied', () => {
    const h = toolHarness();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(10, 0) });
    const tool = h.use(new OffsetTool(h.ctx));
    tool.activate();
    reset(tool, 'I', 'S');
    h.state.hit = locked;
    tool.pointerDown(at(5, 0));
    expect(h.said().at(-1)).toMatch(/^Düzenlenebilir bir çizgi/);
    expect(lines(h)).toHaveLength(1);
  });

  it('with the options off, one copy, the source stays, and the behaviour is the old one', () => {
    const { h, source, tool } = start();
    h.state.hit = source;
    tool.pointerDown(at(5, 0));
    tool.pointerDown(at(5, -3));
    expect(lines(h).map((l) => l.a.y).sort((a, b) => a - b)).toEqual([-2, 0]);
    expect(h.said().at(-1)).toBe('2.000 m ötelenmiş kopya eklendi.');
  });
});
