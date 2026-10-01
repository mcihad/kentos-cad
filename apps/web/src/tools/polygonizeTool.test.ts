import { beforeEach, describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { at, canvasLog, pt, toolHarness } from './toolHarness';
import { FIRST_ATTRIBUTE, labelValue, PolygonizeTool } from './polygonizeTool';

/**
 * Toplu alan (docs/adr/0151 §8): the line work and labels taken when it starts, the finding shown first, Adalar and
 * the attribute's name, the areas written in one step on the active layer. The cases are worked out by hand from the
 * rules of §2–§5; the core itself is checked against the independent reference in model/ops/polygonize.test.ts. The
 * desktop walks the same in crates/native/interaction/tests/polygonize.rs.
 */

beforeEach(() => {
  PolygonizeTool.islands = true;
  PolygonizeTool.attribute = FIRST_ATTRIBUTE;
});

const OPTIONS = (attribute = 'Ad', islands = 'açık') => `[Adalar (A): ${islands} / Öznitelik (Ö): ${attribute} / Uygula (Enter)]`;

/** A 40 × 30 block drawn as a frame and a cross: four 20 × 15 parcels. */
function block(h: ReturnType<typeof toolHarness>, zs?: (number | null)[]) {
  h.add({ kind: 'polyline', pts: [pt(0, 0), pt(40, 0), pt(40, 30), pt(0, 30), pt(0, 0)], ...(zs && { zs }) });
  h.add({ kind: 'line', a: pt(20, 0), b: pt(20, 30) });
  h.add({ kind: 'line', a: pt(0, 15), b: pt(40, 15) });
}

const text = (h: ReturnType<typeof toolHarness>, p: Vec2, value: string) => h.add({ kind: 'text', p, text: value, height: 1, rotation: 0 });

/** The new areas, by the parcel each lies in (its first vertex's quadrant of the block). */
function areas(h: ReturnType<typeof toolHarness>): PolylineEntity[] {
  return [...h.doc.all()].filter((e): e is PolylineEntity => e.kind === 'polygon');
}
const holding = (list: PolylineEntity[], p: Vec2) => list.find((a) => Math.min(...a.pts.map((q) => q.x)) <= p.x && Math.max(...a.pts.map((q) => q.x)) >= p.x && Math.min(...a.pts.map((q) => q.y)) <= p.y && Math.max(...a.pts.map((q) => q.y)) >= p.y);

describe('Toplu alan', () => {
  it('makes every parcel of the block an area with its number, in one step on the active layer', () => {
    const h = toolHarness();
    block(h);
    text(h, pt(8, 6), '101/1');
    text(h, pt(28, 6), '101/2');
    text(h, pt(8, 21), '101/3');
    text(h, pt(28, 21), '101/4');
    const tool = h.use(new PolygonizeTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual(['Toplu alan: 4 alan. Enter ile uygulayın (bütün çizim: 3 çizgi, 4 etiket).']);
    expect(tool.prompt.value).toBe(`Toplu alan: 4 alan ${OPTIONS()}`);
    tool.pointerMove(at(50, 10));
    const log = canvasLog();
    tool.draw(log.g, log.view);
    expect(log.texts).toEqual(['4 alan', 'Enter: uygula']);
    tool.confirm();
    const made = areas(h);
    expect(made).toHaveLength(4);
    expect(made.every((a) => a.layerId === 'cizim')).toBe(true);
    expect(holding(made, pt(10, 7))?.attrs).toEqual({ Ad: '101/1' });
    expect(holding(made, pt(30, 7))?.attrs).toEqual({ Ad: '101/2' });
    expect(holding(made, pt(10, 22))?.attrs).toEqual({ Ad: '101/3' });
    expect(holding(made, pt(30, 22))?.attrs).toEqual({ Ad: '101/4' });
    expect(h.said().at(-1)).toBe('Toplu alan: 4 alan oluşturuldu; “Ad” 4 alana yazıldı.');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Toplu alan');
    expect(areas(h)).toHaveLength(0);
  });

  it('says what to look at, takes Adalar and the attribute’s name, and leaves what is not one label empty', async () => {
    const h = toolHarness();
    block(h);
    h.add({ kind: 'line', a: pt(5, 5), b: pt(12, 9) });
    text(h, pt(2, 2), '101/1');
    text(h, pt(13, 11), 'fazla');
    text(h, pt(28, 6), '101/2');
    h.add({ kind: 'point', p: pt(20, 7), label: 'sınır' });
    text(h, pt(28, 21), '101/4');
    const tool = h.use(new PolygonizeTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual([
      'Toplu alan: 4 alan; 1 etiketsiz, 1 çok etiketli, 1 etiket sınırda, 2 uç boşta. Enter ile uygulayın (bütün çizim: 4 çizgi, 5 etiket).',
      "Toplu alan: 2 çizgi ucu boşta; kapanmayan bölge alan olmaz. Önce Topolojik temizlik'i deneyin.",
      'Toplu alan: bir bölgede 2 etiket: “101/1”, “fazla”.',
      'Toplu alan: “sınır” etiketi bir sınırın üstünde; hiçbir bölgeye verilmedi.',
    ]);
    const said = h.said().length;
    // Ö asks for the name in a text field by the cursor, the old one in it.
    tool.pointerMove(at(50, 10));
    expect(tool.input('Ö')).toBe(true);
    expect(tool.prompt.value).toBe('Toplu alan: öznitelik adını yazın (şimdi: Ad)');
    // The field opens once the key that asked has given the drawing its focus back.
    await Promise.resolve();
    const field = h.state.textInputs.at(-1)!;
    expect([field.at, field.initial, field.placeholder]).toEqual([pt(50, 10), 'Ad', 'Öznitelik adı']);
    field.commit('  ');
    expect(h.said().at(-1)).toBe('Öznitelik adı boş olamaz.');
    tool.input('Ö');
    await Promise.resolve();
    h.state.textInputs.at(-1)!.commit('Parsel');
    expect(tool.prompt.value).toBe(`Toplu alan: 4 alan; 1 etiketsiz, 1 çok etiketli, 1 etiket sınırda, 2 uç boşta ${OPTIONS('Parsel')}`);
    // O answers too, on a keyboard without Ö; Esc keeps the name.
    tool.input('o');
    await Promise.resolve();
    h.state.textInputs.at(-1)!.cancel();
    expect(tool.prompt.value).toMatch(/Öznitelik \(Ö\): Parsel/);
    tool.input('a');
    expect(tool.prompt.value).toMatch(/Adalar \(A\): kapalı/);
    // Nothing changed in what is found: nothing more is said.
    expect(h.said()).toHaveLength(said + 1);
    tool.confirm();
    const made = areas(h);
    expect(made).toHaveLength(4);
    expect(holding(made, pt(30, 7))?.attrs).toEqual({ Parsel: '101/2' });
    expect(holding(made, pt(30, 22))?.attrs).toEqual({ Parsel: '101/4' });
    expect(holding(made, pt(10, 7))?.attrs).toEqual({});
    expect(holding(made, pt(10, 22))?.attrs).toEqual({});
    expect(h.said().at(-1)).toBe('Toplu alan: 4 alan oluşturuldu; “Parsel” 2 alana yazıldı. Özniteliği boş kalan: 1 etiketsiz, 1 çok etiketli.');
    // Kept for the session.
    const again = h.use(new PolygonizeTool(h.ctx));
    again.activate();
    expect(again.prompt.value).toMatch(/Adalar \(A\): kapalı \/ Öznitelik \(Ö\): Parsel/);
  });

  it('does not write a region again that is an area already, and with a selection reads only it', () => {
    const h = toolHarness();
    block(h);
    h.add({ kind: 'polygon', pts: [pt(0, 0), pt(20, 0), pt(20, 15), pt(0, 15)] });
    const tool = h.use(new PolygonizeTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe(`Toplu alan: 3 alan; 3 etiketsiz, 1 zaten alan ${OPTIONS()}`);
    tool.confirm();
    expect(areas(h)).toHaveLength(4);
    h.doc.undo();
    // The frame and the divider only: two halves.
    const [frame, divider] = [...h.doc.all()];
    h.ctx.selection.set([frame.id, divider.id]);
    const picked = h.use(new PolygonizeTool(h.ctx));
    picked.activate();
    expect(h.said().at(-1)).toBe('Toplu alan: 2 alan; 2 etiketsiz. Enter ile uygulayın (seçili 2 çizgi, 0 etiket).');
  });

  it('with no line work says so and leaves', async () => {
    const h = toolHarness();
    text(h, pt(0, 0), '101');
    const tool = h.use(new PolygonizeTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Toplu alan: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin.');
    await Promise.resolve();
    expect(h.state.exited).toBe(1);
  });

  it('carries the corners’ elevations from the line work (docs/adr/0142)', () => {
    const h = toolHarness();
    block(h, [10, 11, 12, 13, 10]);
    const tool = h.use(new PolygonizeTool(h.ctx));
    tool.activate();
    tool.confirm();
    const first = holding(areas(h), pt(10, 7))!;
    const z = (p: Vec2) => first.zs?.[first.pts.findIndex((q) => q.x === p.x && q.y === p.y)];
    // On a frame vertex its elevation; on a frame edge the edge's; where the cross lines meet, none.
    expect(z(pt(0, 0))).toBe(10);
    expect(z(pt(20, 0))).toBeCloseTo(10.5, 12);
    expect(z(pt(0, 15))).toBeCloseTo(11.5, 12);
    expect(z(pt(20, 15))).toBeNull();
  });
});

describe('a label', () => {
  const h = toolHarness();
  it('is a text’s value or a point’s label, trimmed; nothing else is one', () => {
    expect(labelValue(h.add({ kind: 'text', p: pt(0, 0), text: ' 101/7 ', height: 1, rotation: 0 }))).toBe('101/7');
    expect(labelValue(h.add({ kind: 'point', p: pt(0, 0), label: 'P12' }))).toBe('P12');
    expect(labelValue(h.add({ kind: 'point', p: pt(0, 0) }))).toBeNull();
    expect(labelValue(h.add({ kind: 'line', a: pt(0, 0), b: pt(1, 0) }) as Entity)).toBeNull();
  });
});
