import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { AreaMeasureTool } from './measureTools';
import { PathTool } from './pathTool';
import { RectangleTool } from './shapeTools';
import { at, pt, toolHarness } from './toolHarness';

/**
 * The overlap control (docs/adr/0162 §1–§2) in the tools that draw a new area by its outline: Serbest writes it as
 * drawn; Kendi katmanında önle cuts what overlaps the visible areas of its own layer, Seçili katmanlarda önle those of
 * the chosen layers; covered, nothing is written; cut apart, one multi-part area. A neighbour square (0, 0)–(10, 10);
 * expected values worked out by hand. The desktop's are crates/native/interaction/tests/all/overlap.rs, the shared trace
 * fixtures/interaction/v1/overlap.json.
 */
function scene() {
  const h = toolHarness();
  h.add({ kind: 'polygon', pts: [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)] });
  return h;
}

const newest = (h: ReturnType<typeof scene>) => [...h.doc.all()].at(-1) as PolylineEntity;
const count = (h: ReturnType<typeof scene>) => [...h.doc.all()].length;

function polygon(h: ReturnType<typeof scene>, ...corners: [number, number][]): void {
  const tool = h.use(new PathTool(h.ctx, { id: 'polygon', label: 'Kapalı alan', closed: true }));
  tool.activate();
  for (const [x, y] of corners) tool.pointerDown(at(x, y));
  tool.confirm();
  tool.deactivate();
}

describe('Çakışma denetimi', () => {
  it('Serbest writes the new area as drawn', () => {
    const h = scene();
    polygon(h, [5, 0], [20, 0], [20, 10], [5, 10]);
    expect(newest(h).pts).toEqual([pt(5, 0), pt(20, 0), pt(20, 10), pt(5, 10)]);
    expect(h.said().at(-1)).toBe('Kapalı alan eklendi: 150.00 m²');
  });

  it('Kendi katmanında önle cuts it back to the neighbour, the shared side the neighbour’s', () => {
    const h = scene();
    h.ctx.settings.overlap.set('layer');
    polygon(h, [5, 0], [20, 0], [20, 10], [5, 10]);
    const got = newest(h);
    expect(count(h)).toBe(2);
    expect(got.pts).toEqual([pt(10, 0), pt(20, 0), pt(20, 10), pt(10, 10)]);
    expect(got.parts ?? []).toEqual([]);
    expect(h.said().slice(-2)).toEqual(['Çakışma önlendi: 1 komşu alanla örtüşen kısım çıkarıldı.', 'Kapalı alan eklendi: 100.00 m²']);
    // One undo step takes the new area away.
    h.doc.undo();
    expect(count(h)).toBe(1);
  });

  it('takes a neighbour of another layer only when it is chosen', () => {
    const h = scene();
    const road = h.add({ kind: 'polygon', layerId: 'yol', pts: [pt(15, -5), pt(25, -5), pt(25, 15), pt(15, 15)] });
    h.ctx.settings.overlap.set('layer');
    polygon(h, [12, 0], [30, 0], [30, 10], [12, 10]);
    expect(newest(h).pts).toEqual([pt(12, 0), pt(30, 0), pt(30, 10), pt(12, 10)]);
    h.ctx.settings.overlap.set('layers');
    h.ctx.settings.overlapLayers.set(new Set([road.layerId]));
    polygon(h, [12, 0], [30, 0], [30, 10], [12, 10]);
    const got = newest(h);
    expect(got.pts).toEqual([pt(12, 0), pt(15, 0), pt(15, 10), pt(12, 10)]);
    expect(got.parts).toEqual([{ pts: [pt(25, 0), pt(30, 0), pt(30, 10), pt(25, 10)] }]);
  });

  it('writes nothing when the neighbours cover the new area', () => {
    const h = scene();
    h.ctx.settings.overlap.set('layer');
    polygon(h, [2, 2], [8, 2], [8, 8], [2, 8]);
    expect(count(h)).toBe(1);
    expect(h.said().at(-1)).toBe('Yeni alan komşu alanların içinde kalıyor; alan eklenmedi.');
  });

  it('says that Seçili katmanlarda önle has no layer chosen, and writes the area as drawn', () => {
    const h = scene();
    h.ctx.settings.overlap.set('layers');
    polygon(h, [5, 0], [20, 0], [20, 10], [5, 10]);
    expect(newest(h).pts).toEqual([pt(5, 0), pt(20, 0), pt(20, 10), pt(5, 10)]);
    expect(h.said()).toContain('Seçili katmanlarda önle kipinde seçili katman yok: alan olduğu gibi yazıldı. Katmanları Çakışma hücresinin menüsünden seçin.');
  });

  it('leaves a hidden layer’s areas out', () => {
    const h = scene();
    h.ctx.settings.overlap.set('layer');
    h.doc.layers.setVisible('cizim', false);
    polygon(h, [5, 0], [20, 0], [20, 10], [5, 10]);
    expect(newest(h).pts).toEqual([pt(5, 0), pt(20, 0), pt(20, 10), pt(5, 10)]);
  });

  it('cuts a rectangle as it cuts a drawn area', () => {
    const h = scene();
    h.ctx.settings.overlap.set('layer');
    const tool = h.use(new RectangleTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-5, 5));
    tool.pointerDown(at(5, 15));
    const got = newest(h);
    expect(got.kind).toBe('polygon');
    expect(got.pts).toEqual([pt(-5, 5), pt(0, 5), pt(0, 10), pt(5, 10), pt(5, 15), pt(-5, 15)]);
    expect(h.said()).toContain('Çakışma önlendi: 1 komşu alanla örtüşen kısım çıkarıldı.');
  });

  it('cuts a parcel against the parcel layer', () => {
    const h = scene();
    h.doc.layers.add({ id: 'parsel', name: 'Parsel' });
    h.add({ kind: 'polygon', layerId: 'parsel', pts: [pt(20, 0), pt(30, 0), pt(30, 10), pt(20, 10)], attrs: { Parsel: '1' } });
    h.ctx.settings.overlap.set('layer');
    const tool = h.use(new PathTool(h.ctx, { id: 'parcel', label: 'Parsel', closed: true, parcelLayer: 'parsel' }));
    tool.activate();
    for (const [x, y] of [[5, 0], [25, 0], [25, 10], [5, 10]] as const) tool.pointerDown(at(x, y));
    tool.confirm();
    tool.deactivate();
    const got = newest(h);
    // The area on Çizim is no parcel neighbour: only the parcel layer's is cut away.
    expect(got.layerId).toBe('parsel');
    expect(got.pts).toEqual([pt(5, 0), pt(20, 0), pt(20, 10), pt(5, 10)]);
    expect(got.attrs.Parsel).toBe('2');
    expect(h.said().at(-1)).toMatch(/^Parsel 2 oluşturuldu; geometrik alanı 150\.00 m²/);
  });

  it('cuts what Alan olarak çiz writes, in its one step', () => {
    const h = scene();
    const tool = h.use(new AreaMeasureTool(h.ctx));
    tool.activate();
    for (const [x, y] of [[5, 0], [20, 0], [20, 10], [5, 10]] as const) tool.pointerDown(at(x, y));
    tool.confirm();
    h.ctx.settings.overlap.set('layer');
    expect(tool.input('A')).toBe(true);
    tool.deactivate();
    expect(newest(h).pts).toEqual([pt(10, 0), pt(20, 0), pt(20, 10), pt(10, 10)]);
    expect(h.said().at(-1)).toBe('Alan olarak çizildi: 100.00 m².');
    h.doc.undo();
    expect(count(h)).toBe(1);
  });
});
