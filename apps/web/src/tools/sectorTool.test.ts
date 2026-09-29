import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { SectorTool } from './sectorTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Daire dilimi (docs/adr/0140): the centre, the start (a point, or a typed radius and a
 * start angle), the end direction; written as one closed area with its arc through
 * cad.polygon.create. Over a document and a log, without a view.
 */
const polygons = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is PolylineEntity => e.kind === 'polygon');
const grad = (g: number) => (g * Math.PI) / 200;

describe('Daire dilimi', () => {
  it('writes the slice from a centre, a start point and an end direction, with the arc as its middle edge', () => {
    const h = toolHarness();
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('dilimin merkezini belirtin');
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toContain('başlangıç noktasını gösterin ya da yarıçapı yazın');
    expect(tool.prompt.value).toContain('Geri (G)');
    tool.pointerDown(at(10, 0));
    // The angle unit of the project (grad) is in the prompt.
    expect(tool.prompt.value).toContain('bitiş doğrultusunu gösterin ya da bitiş açısını yazın (grad)');
    const r = recorder();
    tool.pointerMove(at(0, 10));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('fill');
    expect(polygons(h)).toHaveLength(0);
    tool.pointerDown(at(0, 10));
    const [s] = polygons(h);
    expect(s.pts).toHaveLength(3);
    expect(s.pts[0]).toEqual(pt(0, 0));
    expect(s.pts[1].x).toBeCloseTo(10);
    expect(s.pts[2].y).toBeCloseTo(10);
    expect(s.bulges?.[1]).toBeCloseTo(Math.tan(Math.PI / 8));
    expect(h.said().at(-1)).toBe('Daire dilimi eklendi: r = 10.000 m, açı 100.0000 g');
    expect(h.doc.undo()).toBe('Ekle');
    expect(polygons(h)).toHaveLength(0);
    // It asks for the next centre.
    expect(tool.prompt.value).toContain('dilimin merkezini belirtin');
  });

  it('takes a typed radius, then a typed start angle, then a typed end angle, in the project unit', () => {
    const h = toolHarness();
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.input('5,5');
    expect(tool.input('4')).toBe(true);
    expect(tool.prompt.value).toContain('başlangıç açısını yazın (grad, doğudan saat yönünün tersine)');
    expect(tool.prompt.value).toContain('yarıçap 4.000 m');
    expect(tool.input('0')).toBe(true);
    expect(tool.input('50')).toBe(true);
    const [s] = polygons(h);
    expect(s.pts[0]).toEqual(pt(5, 5));
    expect(s.pts[1].x).toBeCloseTo(9);
    expect(s.pts[2].x).toBeCloseTo(5 + 4 * Math.cos(grad(50)));
    expect(s.pts[2].y).toBeCloseTo(5 + 4 * Math.sin(grad(50)));
    expect(h.said().at(-1)).toContain('açı 50.0000 g');
  });

  it('degrees when the project counts in degrees', () => {
    const h = toolHarness();
    (h.ctx.format as unknown as { prefs: { angleUnit: { set(v: string): void } } }).prefs.angleUnit.set('deg');
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('2');
    expect(tool.input('0')).toBe(true);
    expect(tool.prompt.value).toContain('(derece)');
    tool.input('90');
    expect(polygons(h)[0].pts[2].y).toBeCloseTo(2);
    expect(h.said().at(-1)).toContain('açı 90.0000°');
  });

  it('refuses a radius of nothing and an end on the start, and asks again', () => {
    const h = toolHarness();
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Yarıçap sıfırdan büyük olmalı.');
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(20, 0));
    expect(h.said().at(-1)).toMatch(/^Bitiş doğrultusu başlangıçla aynı/);
    expect(polygons(h)).toHaveLength(0);
    // The end is still asked.
    expect(tool.prompt.value).toContain('bitiş doğrultusunu');
  });

  it('a point on the centre says so', () => {
    const h = toolHarness();
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(1, 1));
    tool.pointerDown(at(1, 1));
    expect(h.said().at(-1)).toMatch(/^Nokta merkezle çakışıyor/);
  });

  it('Esc steps back a stage at a time, then leaves; Enter starts over', () => {
    const h = toolHarness();
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('başlangıç noktasını');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('dilimin merkezini');
    expect(tool.cancel()).toBe(false);
    tool.pointerDown(at(0, 0));
    tool.confirm();
    expect(tool.prompt.value).toContain('dilimin merkezini');
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('writes nothing on a locked layer and says so', () => {
    const h = toolHarness();
    h.doc.layers.active.set('kilitli');
    const tool = h.use(new SectorTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('3');
    tool.input('0');
    tool.input('100');
    expect(polygons(h)).toHaveLength(0);
    expect(h.said().at(-1)).toMatch(/kilitli/);
  });
});
