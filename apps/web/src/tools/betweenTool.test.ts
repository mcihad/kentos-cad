import { describe, expect, it } from 'vitest';
import type { PointEntity } from '../model/entities';
import { PointsBetweenTool } from './betweenTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Ara nokta (docs/adr/0140): two points, then points between them by parts (E), by
 * distances (U) or by ratios (O); one edit through cad.entities.create.
 */
const points = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is PointEntity => e.kind === 'point');

describe('Ara nokta', () => {
  it('previews the points after the second click and writes them, in equal parts, when the count is typed', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('ilk noktayı belirtin');
    expect(tool.prompt.value).toContain('4 parça');
    expect(tool.prompt.value).toContain('Uzaklıkla (U) / Oranla (O)');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(12, 0));
    expect(tool.prompt.value).toContain('parça sayısını yazın (Enter: 4 parça)');
    const r = recorder();
    tool.pointerMove(at(12, 0));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fill');
    expect(points(h)).toHaveLength(0);
    expect(tool.input('3')).toBe(true);
    expect(points(h).map((p) => p.p)).toEqual([pt(4, 0), pt(8, 0)]);
    expect(h.said().at(-1)).toBe('2 nokta kondu.');
    expect(h.doc.undo()).toBe('Ara nokta');
    expect(points(h)).toHaveLength(0);
    // The next two points are asked; the count is kept.
    expect(tool.prompt.value).toContain('ilk noktayı belirtin');
    expect(tool.prompt.value).toContain('3 parça');
  });

  it('Enter writes with the kept count', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    tool.input('5');
    expect(points(h)).toHaveLength(4);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    tool.confirm();
    // Four points again: the count of 5 parts was kept.
    expect(points(h)).toHaveLength(8);
  });

  it('Uzaklıkla: comma separated distances from the first point', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    expect(tool.input('U')).toBe(true);
    expect(tool.prompt.value).toContain('Eşit aralık (E) / Oranla (O)');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(0, 20));
    expect(tool.prompt.value).toContain('uzaklıkları virgülle yazın');
    // No distance yet: Enter says what to type.
    tool.confirm();
    expect(h.said().at(-1)).toBe('Uzaklıkları yazın, ör. 5, 12.5.');
    expect(tool.input('5, 12.5')).toBe(true);
    expect(points(h).map((p) => p.p)).toEqual([pt(0, 5), pt(0, 12.5)]);
    expect(h.said().at(-1)).toBe('2 nokta kondu.');
    expect(tool.prompt.value).toContain('5.000, 12.500 m');
  });

  it('a distance beyond the second point is refused with the length said', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.input('U');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.input('4, 25')).toBe(true);
    expect(h.said().at(-1)).toBe('25.000 m uzaklığı iki nokta arasının (10.000 m) dışında kalıyor; 0 ile 10.000 m arasında yazın.');
    expect(points(h)).toHaveLength(0);
  });

  it('Oranla: fractions from 0 to 1', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.input('O');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    expect(tool.input('0.25 0.5')).toBe(true);
    expect(points(h).map((p) => p.p)).toEqual([pt(2, 0), pt(4, 0)]);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    expect(tool.input('1.5')).toBe(true);
    expect(h.said().at(-1)).toBe('1.5 oranı 0 ile 1 arasında değil; iki noktanın arasında kalan oranlar yazın.');
    expect(points(h)).toHaveLength(2);
  });

  it('E goes back to equal parts; a count outside 2 to 10 000 is refused', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.input('O');
    tool.input('E');
    expect(tool.prompt.value).toContain('Uzaklıkla (U) / Oranla (O)');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    expect(tool.input('1')).toBe(true);
    expect(h.said().at(-1)).toBe('Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.');
    expect(tool.input('2.5')).toBe(true);
    expect(points(h)).toHaveLength(0);
  });

  it('two points on each other say so; Esc steps back a point, then leaves', () => {
    const h = toolHarness();
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(3, 3));
    tool.pointerDown(at(3, 3));
    expect(h.said().at(-1)).toBe('İkinci nokta ilkiyle çakışıyor; başka bir yer gösterin.');
    tool.pointerDown(at(9, 3));
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('ikinci noktayı belirtin');
    expect(tool.cancel()).toBe(true);
    expect(tool.cancel()).toBe(false);
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('writes nothing on a locked layer and keeps the two points', () => {
    const h = toolHarness();
    h.doc.layers.active.set('kilitli');
    const tool = h.use(new PointsBetweenTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(8, 0));
    tool.input('4');
    expect(points(h)).toHaveLength(0);
    expect(h.said().at(-1)).toMatch(/kilitli/);
  });
});
