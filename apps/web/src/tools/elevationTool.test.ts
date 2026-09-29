import { describe, expect, it } from 'vitest';
import type { Entity, LineEntity, PointEntity, PolylineEntity } from '../model/entities';
import { ElevationTool } from './elevationTool';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Kot ver (docs/adr/0142): the steps, the three ways (Sabit, Artır, Sıfırla), the messages, one undo step named
 * "Kot ver", what is left out, and the locked layer. Over a document and a log, without a view.
 */
const SQUARE = [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];

function scene() {
  const h = toolHarness();
  const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 10, zb: 20 });
  const path = h.add({ kind: 'polyline', pts: [pt(0, 5), pt(5, 5), pt(9, 5)], zs: [1, null, 3] });
  const bare = h.add({ kind: 'polyline', pts: [pt(0, 8), pt(5, 8), pt(9, 8)] });
  const area = h.add({ kind: 'polygon', pts: SQUARE, zs: [1, 2, 3, 4], holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4)], zs: [5, 6, 7] }] });
  const spot = h.add({ kind: 'point', p: pt(20, 20), z: 12.5 });
  const spotless = h.add({ kind: 'point', p: pt(21, 21) });
  return { h, line, path, bare, area, spot, spotless };
}

const select = (h: ReturnType<typeof toolHarness>, ...es: Entity[]) => h.ctx.selection.set(es.map((e) => e.id));
const levels = (h: ReturnType<typeof toolHarness>) => h.log.entries.value.map((e) => `${e.level}: ${e.text}`);
const get = <T extends Entity>(h: ReturnType<typeof toolHarness>, e: Entity) => h.doc.get(e.id) as T;

describe('Kot ver: the steps', () => {
  it('asks for the objects first, then for the elevation, with Artır and Sıfırla offered at both', () => {
    const { h, line } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]');
    // A click picks, Enter (or a right click) ends the picking.
    h.state.hit = line;
    tool.pointerDown(at(1, 0));
    tool.pointerUp(at(1, 0));
    expect(h.ctx.selection.has(line.id)).toBe(true);
    tool.confirm();
    expect(tool.prompt.value).toBe('Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]');
    // Nothing is written by picking.
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 10, zb: 20 });
  });

  it('a window picks too', () => {
    const { h, line, path } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    h.state.inWindow = [line.id, path.id];
    tool.pointerDown(at(-5, -5));
    tool.pointerMove(at(40, 40));
    tool.pointerUp(at(40, 40));
    expect([...h.ctx.selection.ids.value].sort()).toEqual([line.id, path.id].sort());
  });

  it('a selection made before the tool started is used at once: the elevation comes directly', () => {
    const { h, line } = scene();
    select(h, line);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]');
  });

  it('Artır is a toggle: the chip says açık when on, and the elevation asked for becomes a difference', () => {
    const { h, line } = scene();
    select(h, line);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('a')).toBe(true);
    expect(tool.prompt.value).toBe('Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]');
    expect(tool.input('A')).toBe(true);
    expect(tool.prompt.value).toBe('Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]');
  });

  it('Artır set at the first step is still on at the second', () => {
    const { h, line } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('A');
    expect(tool.prompt.value).toBe('Kot ver: kot verilecek nesneleri seçin [Artır (A): açık / Sıfırla (S)]');
    select(h, line);
    tool.confirm();
    expect(tool.prompt.value).toBe('Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]');
  });

  it('each start is a fresh run: Artır is not carried into the next one', () => {
    const { h, line } = scene();
    select(h, line);
    const first = h.use(new ElevationTool(h.ctx));
    first.activate();
    first.input('A');
    const second = h.use(new ElevationTool(h.ctx));
    second.activate();
    expect(second.prompt.value).toContain('kotu yazın (m) [Artır (A) / Sıfırla (S)]');
  });

  it('Esc steps back one step: from the elevation to the picking (the selection stays), from picking out of the tool', () => {
    const { h, line } = scene();
    select(h, line);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toBe('Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]');
    expect(h.ctx.selection.has(line.id)).toBe(true);
    expect(tool.cancel()).toBe(false);
  });

  it('Enter with nothing selected leaves the tool; a right click at the elevation step goes back a step', () => {
    const { h, line } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.confirm();
    expect(h.state.exited).toBe(1);
    select(h, line);
    tool.activate();
    tool.confirm();
    expect(h.state.exited).toBe(1);
    expect(tool.prompt.value).toContain('kot verilecek nesneleri seçin');
  });

  it('a number before any object is picked is not understood, and a point from the calculator means nothing', () => {
    const { h } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('100')).toBe(false);
    expect(tool.acceptPoint()).toBe(false);
    expect(h.doc.canUndo.value).toBe(true); // only the drawing's own “Ekle”
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('what is not a number is not understood; a comma reads as a coordinate', () => {
    const { h, line } = scene();
    select(h, line);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('yüz')).toBe(false);
    expect(tool.input('12,5')).toBe(false);
    expect(tool.input('@5,3')).toBe(false);
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 10, zb: 20 });
  });
});

describe('Kot ver: Sabit', () => {
  it('gives every vertex the number, a point its z, in one step named Kot ver, and goes back to picking', () => {
    const { h, line, path, bare, area, spot, spotless } = scene();
    select(h, line, path, bare, area, spot, spotless);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('100.5')).toBe(true);
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 100.5, zb: 100.5 });
    expect(get<PolylineEntity>(h, path).zs).toEqual([100.5, 100.5, 100.5]);
    expect(get<PolylineEntity>(h, bare).zs).toEqual([100.5, 100.5, 100.5]);
    const a = get<PolylineEntity>(h, area);
    expect(a.zs).toEqual([100.5, 100.5, 100.5, 100.5]);
    // The holes too.
    expect(a.holes?.[0].zs).toEqual([100.5, 100.5, 100.5]);
    expect(get<PointEntity>(h, spot).z).toBe(100.5);
    expect(get<PointEntity>(h, spotless).z).toBe(100.5);
    expect(levels(h).at(-1)).toBe('info: Kot verildi: 6 nesne.');
    // Back at the first step, nothing selected.
    expect(h.ctx.selection.size).toBe(0);
    expect(tool.prompt.value).toBe('Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]');
    expect(h.state.exited).toBe(0);
    // One step, named Kot ver; the next one under it is the drawing's own.
    expect(h.doc.undo()).toBe('Kot ver');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 10, zb: 20 });
    expect(get<PolylineEntity>(h, path).zs).toEqual([1, null, 3]);
    expect(get<PolylineEntity>(h, bare).zs).toBeUndefined();
    expect(get<PolylineEntity>(h, area).holes?.[0].zs).toEqual([5, 6, 7]);
    expect(get<PointEntity>(h, spot).z).toBe(12.5);
    expect(get<PointEntity>(h, spotless).z).toBeUndefined();
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('0 is an elevation, and a negative one is too (below sea level, an excavation)', () => {
    const { h, line, bare } = scene();
    select(h, line, bare);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('0');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 0, zb: 0 });
    expect(get<PolylineEntity>(h, bare).zs).toEqual([0, 0, 0]);
    select(h, line);
    tool.confirm();
    tool.input('-4.25');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: -4.25, zb: -4.25 });
  });

  it('can be done again on the next objects without leaving the tool', () => {
    const { h, line, bare } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    select(h, line);
    tool.confirm();
    tool.input('50');
    select(h, bare);
    tool.confirm();
    tool.input('60');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 50, zb: 50 });
    expect(get<PolylineEntity>(h, bare).zs).toEqual([60, 60, 60]);
    expect(h.state.exited).toBe(0);
    expect(h.doc.undo()).toBe('Kot ver');
    expect(h.doc.undo()).toBe('Kot ver');
  });

  it('keeps everything but the elevations: layer, colour, attributes, the geometry', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polygon', layerId: 'yol', color: '#E5484D', attrs: { Ada: '12' }, label: '7', pts: SQUARE });
    select(h, e);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('3');
    expect(get<PolylineEntity>(h, e)).toMatchObject({ layerId: 'yol', color: '#E5484D', attrs: { Ada: '12' }, label: '7', pts: SQUARE, zs: [3, 3, 3, 3] });
  });
});

describe('Kot ver: Artır', () => {
  it('adds the difference to each vertex that has an elevation; one without stays without', () => {
    const { h, line, path, bare, area, spot, spotless } = scene();
    select(h, line, path, bare, area, spot, spotless);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('A');
    expect(tool.input('2.5')).toBe(true);
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 12.5, zb: 22.5 });
    expect(get<PolylineEntity>(h, path).zs).toEqual([3.5, null, 5.5]);
    // No elevation to raise: left as it is.
    expect(get<PolylineEntity>(h, bare).zs).toBeUndefined();
    expect(get<PolylineEntity>(h, area).zs).toEqual([3.5, 4.5, 5.5, 6.5]);
    expect(get<PolylineEntity>(h, area).holes?.[0].zs).toEqual([7.5, 8.5, 9.5]);
    expect(get<PointEntity>(h, spot).z).toBe(15);
    expect(get<PointEntity>(h, spotless).z).toBeUndefined();
    // The objects raised are the ones counted.
    expect(levels(h).at(-1)).toBe('info: Kotlar +2.500 m değişti: 4 nesne.');
    expect(h.doc.undo()).toBe('Kot ver');
  });

  it('a negative difference lowers, and the message shows its minus sign (U+2212)', () => {
    const { h, line } = scene();
    select(h, line);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('A');
    tool.input('-1');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 9, zb: 19 });
    expect(levels(h).at(-1)).toBe('info: Kotlar −1.000 m değişti: 1 nesne.');
  });

  it('adds the sum as it is, not a rounded one', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 100.1, zb: 0.1 });
    select(h, e);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('A');
    tool.input('0.2');
    expect(get<LineEntity>(h, e)).toMatchObject({ za: 100.1 + 0.2, zb: 0.1 + 0.2 });
  });

  it('says so and writes nothing when no vertex of the selection has an elevation', () => {
    const { h, bare, spotless } = scene();
    select(h, bare, spotless);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('A');
    tool.input('2');
    expect(levels(h).at(-1)).toBe('warn: Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.');
    expect(get<PolylineEntity>(h, bare).zs).toBeUndefined();
    expect(h.doc.undo()).toBe('Ekle');
    // Still at the elevation step: the selection is kept.
    expect(h.ctx.selection.size).toBe(2);
  });
});

describe('Kot ver: Sıfırla', () => {
  it('writes at once: every vertex without an elevation (not 0), a point without z', () => {
    const { h, line, path, area, spot } = scene();
    select(h, line, path, area, spot);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('S')).toBe(true);
    const l = get<LineEntity>(h, line);
    expect('za' in l || 'zb' in l).toBe(false);
    expect('zs' in get<PolylineEntity>(h, path)).toBe(false);
    const a = get<PolylineEntity>(h, area);
    expect('zs' in a).toBe(false);
    expect(a.holes?.[0] && 'zs' in a.holes[0]).toBe(false);
    expect('z' in get<PointEntity>(h, spot)).toBe(false);
    expect(levels(h).at(-1)).toBe('info: Kot silindi: 4 nesne.');
    expect(h.ctx.selection.size).toBe(0);
    expect(h.doc.undo()).toBe('Kot ver');
    expect(get<LineEntity>(h, line)).toMatchObject({ za: 10, zb: 20 });
    expect(get<PolylineEntity>(h, path).zs).toEqual([1, null, 3]);
  });

  it('at the picking step it acts on the objects picked so far', () => {
    const { h, line } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    select(h, line);
    tool.input('S');
    expect('za' in get<LineEntity>(h, line)).toBe(false);
    expect(levels(h).at(-1)).toBe('info: Kot silindi: 1 nesne.');
    expect(tool.prompt.value).toBe('Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]');
  });

  it('at the picking step with nothing picked, asks for objects first and writes nothing', () => {
    const { h } = scene();
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('S')).toBe(true);
    expect(levels(h).at(-1)).toBe('warn: Önce kotu silinecek nesneleri seçin.');
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('started from the ribbon’s Sıfırla with a selection (the option typed right after the start), it resets that selection', () => {
    const { h, line, spot } = scene();
    select(h, line, spot);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(tool.input('S')).toBe(true);
    expect('za' in get<LineEntity>(h, line)).toBe(false);
    expect('z' in get<PointEntity>(h, spot)).toBe(false);
  });
});

describe('Kot ver: what is left out', () => {
  it('says how many objects take no elevation, on a line of its own after the result', () => {
    const { h, line } = scene();
    const circle = h.add({ kind: 'circle', c: pt(50, 50), r: 5 });
    const text = h.add({ kind: 'text', p: pt(1, 1), text: 'Park', height: 2, rotation: 0 });
    select(h, line, circle, text);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('100');
    expect(levels(h).slice(-2)).toEqual(['info: Kot verildi: 1 nesne.', 'warn: 2 nesne kot almaz (yalnız çizgi, çoklu çizgi, alan ve nokta).']);
    expect(h.doc.get(circle.id)).toEqual(circle);
  });

  it('nothing selected takes an elevation: says so, writes nothing, and stays at the picking', () => {
    const h = toolHarness();
    const circle = h.add({ kind: 'circle', c: pt(50, 50), r: 5 });
    select(h, circle);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    expect(levels(h).at(-1)).toBe('warn: Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.');
    expect(tool.prompt.value).toContain('kot verilecek nesneleri seçin');
    expect(tool.input('S')).toBe(true);
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('Sıfırla over picked objects that take none says so and stays at the picking', () => {
    const h = toolHarness();
    const circle = h.add({ kind: 'circle', c: pt(50, 50), r: 5 });
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    select(h, circle);
    tool.input('S');
    expect(levels(h).at(-1)).toBe('warn: Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.');
    expect(tool.prompt.value).toContain('kot verilecek nesneleri seçin');
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('the same words when the selection is confirmed with nothing that takes one', () => {
    const h = toolHarness();
    const circle = h.add({ kind: 'circle', c: pt(50, 50), r: 5 });
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    select(h, circle);
    tool.confirm();
    expect(levels(h).at(-1)).toBe('warn: Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.');
    expect(tool.prompt.value).toContain('kot verilecek nesneleri seçin');
  });
});

describe('Kot ver: the locked layer', () => {
  it('leaves objects on a locked layer out and says how many', () => {
    const h = toolHarness();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 2 });
    const open = h.add({ kind: 'line', a: pt(0, 5), b: pt(5, 5) });
    select(h, locked, open);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('10');
    expect(get<LineEntity>(h, locked)).toMatchObject({ za: 1, zb: 2 });
    expect(get<LineEntity>(h, open)).toMatchObject({ za: 10, zb: 10 });
    expect(levels(h).slice(-2)).toEqual(['info: Kot verildi: 1 nesne.', 'warn: 1 nesne kilitli katmanda olduğu için atlandı.']);
  });

  it('if only locked ones are chosen the command refuses with its own message, and nothing is written', () => {
    const h = toolHarness();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 2 });
    select(h, locked);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('10');
    expect(levels(h).at(-1)).toBe('warn: “Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.');
    expect(get<LineEntity>(h, locked)).toMatchObject({ za: 1, zb: 2 });
    expect(h.doc.undo()).toBe('Ekle');
    // Still at the elevation step: unlock the layer, and type it again.
    expect(tool.prompt.value).toContain('kotu yazın (m)');
    expect(h.ctx.selection.size).toBe(1);
  });

  it('Sıfırla and Artır are refused the same way', () => {
    const h = toolHarness();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 2 });
    select(h, locked);
    const tool = h.use(new ElevationTool(h.ctx));
    tool.activate();
    tool.input('S');
    expect(levels(h).at(-1)).toContain('katmanı kilitli');
    tool.input('A');
    tool.input('1');
    expect(levels(h).at(-1)).toContain('katmanı kilitli');
    expect(get<LineEntity>(h, locked)).toMatchObject({ za: 1, zb: 2 });
  });
});
