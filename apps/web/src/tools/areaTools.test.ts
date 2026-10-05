import { afterEach, describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { entityArea } from '../model/entities';
import { AreaIntersectTool, AreaSubtractTool, AreaUnionTool, PartsJoinTool, PartsSplitTool } from './areaTools';
import { pt, toolHarness } from './toolHarness';

/**
 * Alan işlemleri (docs/adr/0065) and the multi-part area (docs/adr/0143): Alan birleştir, kesiştir and çıkar take a
 * multi-part area whole; with Tek nesne (T) their result is one multi-part area; Parçaları birleştir and Parçalara
 * ayır. The same drawing and words as the desktop's (crates/native/interaction/tests/all/area.rs): parcels 1 and 2 of
 * 10 × 10 m overlapping 4 × 6 m, and parcel 8 of 10 × 10 m with a 4 × 4 m hole, apart from them.
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene() {
  const h = toolHarness();
  const p1 = h.add({ kind: 'polygon', pts: square(0, 0, 10), attrs: { Parsel: '1' }, label: '1', zs: [1, 2, 3, 4] }) as PolylineEntity;
  const p2 = h.add({ kind: 'polygon', pts: square(6, 4, 10), attrs: { Parsel: '2' }, label: '2' }) as PolylineEntity;
  const p8 = h.add({ kind: 'polygon', pts: square(50, 0, 10), holes: [{ pts: square(52, 3, 4) }], attrs: { Parsel: '8' }, label: '8', zs: [10, 20, 30, 40] }) as PolylineEntity;
  const select = (...es: Entity[]) => h.ctx.selection.set(es.map((e) => e.id));
  /** The tool started on the selection it finds (it acts at once), or with none (it asks). */
  const run = <T extends { activate(): void }>(tool: T, ...es: Entity[]): T => {
    select(...es);
    h.use(tool);
    tool.activate();
    return tool;
  };
  const now = (e: Entity) => h.doc.get(e.id) as PolylineEntity;
  const selected = () => [...h.ctx.selection.ids.value].map((id) => h.doc.get(id) as PolylineEntity);
  /** The area of each part of an area, the first its own. */
  const sizes = (e: PolylineEntity) =>
    [e, ...(e.parts ?? [])].map((part) => entityArea({ id: 0, layerId: 'cizim', attrs: {}, kind: 'polygon', pts: part.pts, ...(part.bulges && { bulges: part.bulges }), ...(part.holes && { holes: part.holes }) }));
  return { h, p1, p2, p8, select, run, now, selected, sizes };
}

/** Tek nesne is one flag for the three tools, kept between runs: put off, whatever a tool left it. */
function putOff() {
  const tool = new AreaUnionTool(toolHarness().ctx);
  tool.activate();
  if (tool.prompt.value.includes('(T): evet')) tool.input('T');
}
afterEach(putOff);

const near = (have: number | null, want: number) => expect(have).toBeCloseTo(want, 6);

describe('Alan birleştir', () => {
  it('asks for Tek nesne (T) beside nothing else, and toggles it', () => {
    const { h } = scene();
    const tool = h.use(new AreaUnionTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Alan birleştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [Tek nesne (T): hayır]');
    expect(tool.input('t')).toBe(true);
    expect(tool.prompt.value).toContain('[Tek nesne (T): evet]');
    expect(tool.input(' T ')).toBe(true);
    expect(tool.prompt.value).toContain('[Tek nesne (T): hayır]');
    expect(tool.input('X')).toBe(false);
  });

  it('leaves areas that do not touch apart, or with Tek nesne makes one area of two parts, the largest first', () => {
    const { h, p1, p8, run, selected, sizes } = scene();
    const count = h.doc.size;
    run(new AreaUnionTool(h.ctx), p1, p8);
    expect(selected()).toHaveLength(2);
    expect(h.said().at(-1)).toBe('2 alan birleştirildi: 2 ayrı alan (birbirine değmeyenler ayrı kalır), toplam 184.00 m².');
    expect(h.doc.undo()).toBe('Alan birleştir');
    expect(h.doc.size).toBe(count);

    // With Tek nesne: one area of two parts, parcel 1's 100 m² then parcel 8's 84 m², with the first one's data.
    const tool = h.use(new AreaUnionTool(h.ctx));
    h.ctx.selection.clear();
    tool.activate();
    expect(tool.input('t')).toBe(true);
    h.ctx.selection.set([p1.id, p8.id]);
    tool.confirm();
    const made = selected();
    expect(made).toHaveLength(1);
    const [a, b] = sizes(made[0]);
    near(a, 100);
    near(b, 84);
    expect(made[0]).toMatchObject({ label: '1', attrs: { Parsel: '1' }, layerId: 'cizim' });
    expect(h.said().at(-1)).toBe('2 alan birleştirildi: 2 parçalı tek alan, toplam 184.00 m².');
    expect(h.doc.undo()).toBe('Alan birleştir');
  });

  it('a result of one area is a plain area whatever Tek nesne says', () => {
    const { h, p1, p2, selected } = scene();
    const tool = h.use(new AreaUnionTool(h.ctx));
    h.ctx.selection.clear();
    tool.activate();
    tool.input('t');
    h.ctx.selection.set([p1.id, p2.id]);
    tool.confirm();
    expect(selected()).toHaveLength(1);
    expect(selected()[0].parts).toBeUndefined();
    expect(h.said().at(-1)).toBe('2 alan birleştirildi: tek alan, toplam 176.00 m².');
  });
});

describe('a multi-part area is taken whole', () => {
  /** Parcels 1 and 8 as one area of two parts (Tek nesne on), the option put off again. */
  function joined() {
    const s = scene();
    const tool = s.h.use(new AreaUnionTool(s.h.ctx));
    s.h.ctx.selection.clear();
    tool.activate();
    tool.input('T');
    s.h.ctx.selection.set([s.p1.id, s.p8.id]);
    tool.confirm();
    putOff();
    return { ...s, whole: s.selected()[0] };
  }

  it('Alan kesiştir meets what any part meets: parcel 2 overlaps only parcel 1', () => {
    const { h, p2, whole, run, selected } = joined();
    run(new AreaIntersectTool(h.ctx), whole, p2);
    near(entityArea(selected()[0]), 24);
    expect(h.said().at(-1)).toBe('Ortak alan: 24.00 m².');
    expect(h.doc.undo()).toBe('Alan kesiştir');
  });

  it('Alan çıkar cuts parcel 2 from every part: one area of two parts with Tek nesne, 84 m² then 76 m²', () => {
    const { h, p2, whole, sizes, selected } = joined();
    const tool = h.use(new AreaSubtractTool(h.ctx));
    h.ctx.selection.set([whole.id]);
    tool.activate();
    // The cut area is the first selection; its options come with the second.
    expect(tool.prompt.value).toBe('Alan çıkar: çıkarılacak alanları seçin, bitince sağ tıklayın (0 seçili) [Çıkarılanları sil (S): hayır / Tek nesne (T): hayır]');
    expect(tool.input('t')).toBe(true);
    h.ctx.selection.set([p2.id]);
    tool.confirm();
    const made = selected();
    expect(made).toHaveLength(1);
    const [a, b] = sizes(made[0]);
    near(a, 84);
    near(b, 76);
    expect(h.said().at(-1)).toBe('1 alandan çıkarıldı; kalan 160.00 m².');
    expect(h.doc.undo()).toBe('Alan çıkar');
  });

  it('Alan çıkar leaves an area alone that the cut does not touch, and says so', () => {
    const { h, p2, p8, selected } = scene();
    // Parcel 2 does not meet parcel 8.
    const tool = h.use(new AreaSubtractTool(h.ctx));
    h.ctx.selection.set([p8.id]);
    tool.activate();
    h.ctx.selection.set([p2.id]);
    tool.confirm();
    expect(h.said().at(-1)).toBe('Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.');
    expect(selected()).toHaveLength(0);
  });
});

describe('the prompts of Alan kesiştir and çıkar', () => {
  it('Alan kesiştir offers Kaynakları sil (S) and Tek nesne (T), each toggled by its key', () => {
    const { h } = scene();
    const tool = h.use(new AreaIntersectTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Alan kesiştir: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili) [Kaynakları sil (S): hayır / Tek nesne (T): hayır]');
    expect(tool.input('s')).toBe(true);
    expect(tool.prompt.value).toContain('Kaynakları sil (S): evet / Tek nesne (T): hayır');
    expect(tool.input('T')).toBe(true);
    expect(tool.prompt.value).toContain('Kaynakları sil (S): evet / Tek nesne (T): evet');
    tool.input('s');
    tool.input('t');
    expect(tool.prompt.value).toContain('Kaynakları sil (S): hayır / Tek nesne (T): hayır');
  });

  it('Tek nesne is one flag: what one tool sets, the others show', () => {
    const { h } = scene();
    const union = h.use(new AreaUnionTool(h.ctx));
    union.activate();
    union.input('T');
    const intersect = new AreaIntersectTool(h.ctx);
    intersect.activate();
    expect(intersect.prompt.value).toContain('Tek nesne (T): evet');
    const subtract = new AreaSubtractTool(h.ctx);
    h.ctx.selection.set([[...h.doc.all()][0].id]);
    subtract.activate();
    expect(subtract.prompt.value).toContain('Tek nesne (T): evet');
  });

  it('Alan çıkar has no option while it asks for the areas to cut from', () => {
    const { h } = scene();
    const tool = h.use(new AreaSubtractTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Alan çıkar: kesilecek alanları seçin, bitince sağ tıklayın (0 seçili)');
    expect(tool.input('t')).toBe(false);
    expect(tool.input('s')).toBe(false);
  });
});

describe('Parçaları birleştir', () => {
  it('makes the areas one in the first one’s place, every part kept as it was with its elevations, the largest first', () => {
    const { h, p1, p8, run, now, sizes } = scene();
    const count = h.doc.size;
    run(new PartsJoinTool(h.ctx), p1, p8);
    expect([...h.ctx.selection.ids.value]).toEqual([p1.id]);
    expect(h.doc.get(p8.id)).toBeUndefined();
    expect(h.doc.size).toBe(count - 1);
    const joined = now(p1);
    const [a, b] = sizes(joined);
    near(a, 100);
    near(b, 84);
    // Elevations as they were: parcel 1's on its own ring, parcel 8's on its part.
    expect(joined.zs).toEqual([1, 2, 3, 4]);
    expect(joined.parts).toEqual([{ pts: square(50, 0, 10), holes: [{ pts: square(52, 3, 4) }], zs: [10, 20, 30, 40] }]);
    expect(joined).toMatchObject({ label: '1', attrs: { Parsel: '1' } });
    expect(h.said().at(-1)).toBe('2 alan tek alanda birleşti: 2 parça, toplam 184.00 m².');
    expect(h.doc.undo()).toBe('Parçaları birleştir');
    expect(now(p1).parts).toBeUndefined();
    expect(h.doc.get(p8.id)).toBeDefined();
  });

  it('sizes the parts by their area: the larger second area comes first, its data staying with the first picked', () => {
    const { h, p1, p8, run, now } = scene();
    // Parcel 8 first: it lends its place, id and data, and its 84 m² follow parcel 1's 100 m².
    run(new PartsJoinTool(h.ctx), p8, p1);
    const joined = now(p8);
    expect(joined.pts).toEqual(square(0, 0, 10));
    expect(joined.parts?.[0].pts).toEqual(square(50, 0, 10));
    expect(joined).toMatchObject({ label: '8', attrs: { Parsel: '8' } });
    expect(h.doc.get(p1.id)).toBeUndefined();
  });

  it('merges the areas that overlap into one part', () => {
    const { h, p1, p2, run, now } = scene();
    run(new PartsJoinTool(h.ctx), p1, p2);
    const joined = now(p1);
    expect(joined.parts).toBeUndefined();
    near(entityArea(joined), 176);
    expect(h.said().at(-1)).toBe('2 alan tek alanda birleşti: örtüşenler birleşti, 1 parça, toplam 176.00 m².');
    expect(h.doc.undo()).toBe('Parçaları birleştir');
  });

  it('takes a multi-part area with its parts, and a circle becomes the area in its place', () => {
    const { h, p1, p8, run, now } = scene();
    run(new PartsJoinTool(h.ctx), p1, p8);
    const circle = h.add({ kind: 'circle', c: pt(100, 100), r: 3, attrs: { Not: 'daire' } });
    run(new PartsJoinTool(h.ctx), circle, now(p1));
    const joined = h.doc.get(circle.id) as PolylineEntity;
    expect(joined.kind).toBe('polygon');
    expect(joined.attrs).toEqual({ Not: 'daire' });
    // Three parts: parcel 1, parcel 8 and the circle's two half-circle edges.
    expect(joined.parts).toHaveLength(2);
    expect(h.doc.get(p1.id)).toBeUndefined();
    near(entityArea(joined), 100 + 84 + Math.PI * 9);
  });

  it('asks for two areas, as the other tools do, and leaves locked layers out', () => {
    const { h, p1, p2, run } = scene();
    run(new PartsJoinTool(h.ctx), p2);
    expect(h.said().at(-1)).toBe('Parçaları birleştirmek için en az iki alan seçin (kapalı alan, daire, elips ya da kapalı eğri).');
    const locked = h.add({ kind: 'polygon', layerId: 'kilitli', pts: square(80, 0, 5), attrs: {} });
    run(new PartsJoinTool(h.ctx), p1, locked);
    expect(h.said()).toContain('1 nesne kilitli katmanda olduğu için atlandı.');
    expect(h.doc.get(locked.id)).toBeDefined();
  });
});

describe('Parçalara ayır', () => {
  it('keeps the first part in place and gives the others as new areas with the data, elevations as they were', () => {
    const { h, p1, p8, run, now, selected } = scene();
    run(new PartsJoinTool(h.ctx), p1, p8);
    const count = h.doc.size;
    run(new PartsSplitTool(h.ctx), now(p1));
    const made = selected();
    expect(made.map((e) => e.id)).toEqual([p1.id, expect.any(Number)]);
    expect(made).toHaveLength(2);
    expect(h.doc.size).toBe(count + 1);
    expect(made[0].parts).toBeUndefined();
    expect(made[0].pts).toEqual(square(0, 0, 10));
    expect(made[0].zs).toEqual([1, 2, 3, 4]);
    expect(made[1]).toMatchObject({ kind: 'polygon', pts: square(50, 0, 10), holes: [{ pts: square(52, 3, 4) }], zs: [10, 20, 30, 40], label: '1', attrs: { Parsel: '1' }, layerId: 'cizim' });
    expect(h.said().at(-1)).toBe('1 alan parçalarına ayrıldı (2 alan).');
    expect(h.doc.undo()).toBe('Parçalara ayır');
    expect(h.doc.size).toBe(count);
    expect(now(p1).parts).toHaveLength(1);
  });

  it('splits several areas at once, each followed by the objects made from it', () => {
    const { h, p1, p2, p8, run, now, selected } = scene();
    run(new PartsJoinTool(h.ctx), p1, p8);
    const a = now(p1);
    const other = h.add({ kind: 'polygon', pts: square(200, 0, 10), parts: [{ pts: square(220, 0, 5) }, { pts: square(230, 0, 5) }], attrs: {} }) as PolylineEntity;
    run(new PartsSplitTool(h.ctx), a, other, p2);
    const made = selected();
    expect(made.map((e) => e.id)).toEqual([a.id, expect.any(Number), other.id, expect.any(Number), expect.any(Number)]);
    expect(h.said().at(-1)).toBe('2 alan parçalarına ayrıldı (5 alan).');
  });

  it('warns when nothing selected has parts', () => {
    const { h, p2, run } = scene();
    const revision = h.doc.revision;
    run(new PartsSplitTool(h.ctx), p2);
    expect(h.said().at(-1)).toBe('Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne.');
    expect(h.doc.revision).toBe(revision);
  });
});

describe('Parçaları birleştir and Parçalara ayır on lines and points (docs/adr/0174)', () => {
  it('makes a line and a polyline one multi-part polyline in the line’s place, and back', () => {
    const { h, run, now, selected } = scene();
    const line = h.add({ kind: 'line', a: { x: 0, y: 100 }, b: { x: 10, y: 100 }, za: 100, zb: 101.5, attrs: { Ad: 'Şerit' }, label: 'Ş1' });
    const path = h.add({ kind: 'polyline', pts: [{ x: 20, y: 100 }, { x: 30, y: 100 }, { x: 30, y: 110 }], zs: [102, null, 104], attrs: {} }) as PolylineEntity;
    const count = h.doc.size;
    run(new PartsJoinTool(h.ctx), line, path);
    const joined = now(line);
    expect(joined).toMatchObject({ kind: 'polyline', pts: [{ x: 0, y: 100 }, { x: 10, y: 100 }], zs: [100, 101.5], attrs: { Ad: 'Şerit' }, label: 'Ş1' });
    expect(joined.parts).toEqual([{ pts: path.pts, zs: [102, null, 104] }]);
    expect(h.doc.get(path.id)).toBeUndefined();
    expect(h.doc.size).toBe(count - 1);
    expect(h.said().at(-1)).toBe('2 çizgi tek çoklu çizgide birleşti: 2 parça, toplam 30.000 m.');
    run(new PartsSplitTool(h.ctx), joined);
    const made = selected();
    expect(made).toHaveLength(2);
    expect(made[0]).toMatchObject({ id: line.id, kind: 'polyline', pts: [{ x: 0, y: 100 }, { x: 10, y: 100 }] });
    expect(made[0].parts).toBeUndefined();
    expect(made[1]).toMatchObject({ kind: 'polyline', pts: path.pts, zs: [102, null, 104], attrs: { Ad: 'Şerit' }, label: 'Ş1' });
    expect(h.said().at(-1)).toBe('1 nesne parçalarına ayrıldı (2 nesne).');
    expect(h.doc.undo()).toBe('Parçalara ayır');
    expect(h.doc.undo()).toBe('Parçaları birleştir');
    expect(h.doc.get(line.id)?.kind).toBe('line');
  });

  it('makes points one multi-point object, each with its elevation, and back', () => {
    const { h, run, selected } = scene();
    const a = h.add({ kind: 'point', p: { x: 0, y: 200 }, z: 50, attrs: { Kod: 'K' }, label: 'N1' });
    const b = h.add({ kind: 'point', p: { x: 5, y: 200 }, attrs: {} });
    const c = h.add({ kind: 'point', p: { x: 9, y: 200 }, z: 0, attrs: {} });
    run(new PartsJoinTool(h.ctx), a, b, c);
    const joined = h.doc.get(a.id);
    expect(joined).toMatchObject({ kind: 'point', p: { x: 0, y: 200 }, z: 50, parts: [{ p: { x: 5, y: 200 } }, { p: { x: 9, y: 200 }, z: 0 }] });
    expect(h.said().at(-1)).toBe('3 nokta tek nesnede birleşti: 3 nokta.');
    run(new PartsSplitTool(h.ctx), joined!);
    expect(selected().map((e) => (e as unknown as { z?: number }).z)).toEqual([50, undefined, 0]);
    expect(h.said().at(-1)).toBe('1 nesne parçalarına ayrıldı (3 nesne).');
  });

  it('refuses kinds mixed, and asks for two of a kind', () => {
    const { h, p1, run } = scene();
    const line = h.add({ kind: 'line', a: { x: 0, y: 100 }, b: { x: 10, y: 100 }, attrs: {} });
    const point = h.add({ kind: 'point', p: { x: 0, y: 200 }, attrs: {} });
    const revision = h.doc.revision;
    run(new PartsJoinTool(h.ctx), p1, line);
    expect(h.said().at(-1)).toBe('Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.');
    run(new PartsJoinTool(h.ctx), line, point);
    expect(h.said().at(-1)).toBe('Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.');
    run(new PartsJoinTool(h.ctx), line);
    expect(h.said().at(-1)).toBe('Parçaları birleştirmek için en az iki çizgi ya da çoklu çizgi seçin.');
    run(new PartsJoinTool(h.ctx), point);
    expect(h.said().at(-1)).toBe('Parçaları birleştirmek için en az iki nokta seçin.');
    expect(h.doc.revision).toBe(revision);
  });
});

describe('the tools in the catalog', () => {
  it('names the tools and the steps they write as the desktop does', async () => {
    const { TOOL_CATALOG } = await import('./catalog');
    const byId = (id: string) => TOOL_CATALOG.find((t) => t.id === id)!;
    expect(byId('partsJoin')).toMatchObject({ label: 'Parçaları birleştir', icon: 'partsJoin', group: 'area', section: 'boolean', aliases: ['PARCABIRLESTIR', 'PARCALARIBIRLESTIR'], productCommand: 'cad.entities.edit' });
    expect(byId('partsSplit')).toMatchObject({ label: 'Parçalara ayır', icon: 'partsSplit', group: 'area', section: 'boolean', aliases: ['PARCALARAAYIR', 'PARCAAYIR'], productCommand: 'cad.entities.edit' });
    expect(byId('partsJoin').shortcut).toBeUndefined();
    expect(byId('partsSplit').shortcut).toBeUndefined();
    // Right after Alan çıkar.
    const ids = TOOL_CATALOG.map((t) => t.id);
    expect(ids.slice(ids.indexOf('areaSubtract'), ids.indexOf('areaSubtract') + 3)).toEqual(['areaSubtract', 'partsJoin', 'partsSplit']);
  });
});
