import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { TextEntity } from '../model/entities';
import { PickIndex } from '../viewport/picking';
import { LabelsToTextTool, readScale } from './labelsToTextTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Etiketleri yazıya çevir (docs/adr/0175 §3, 0212 §4): the objects taken when it starts, the finding shown first, the
 * scale and the options, the texts written in one step on the standard text layer (opened in that step) or the active
 * one. The places and sizes are the label engine's, checked against the independent reference in
 * viewport/labelEngine.test.ts; here the tool's flow. The desktop walks the same in
 * crates/native/interaction/tests/all/labels_to_text.rs.
 */

const pickers: PickIndex[] = [];
beforeEach(() => {
  LabelsToTextTool.every = false;
  LabelsToTextTool.mask = false;
  LabelsToTextTool.active = false;
  LabelsToTextTool.linked = false;
});
afterEach(() => pickers.splice(0).forEach((p) => p.dispose()));

const LABEL = 'Etiketleri yazıya çevir';
const OPTIONS = (scale = 1000, every = 'kapalı', mask = 'kapalı', layer = 'Yazılar', linked = 'kapalı') =>
  `[Ölçek (Ö): 1:${scale} / Örtüşenler de (R): ${every} / Zemin (Z): ${mask} / Katman (K): ${layer} / Nesneye bağlı (B): ${linked} / Uygula (Enter)]`;

/**
 * Two parcels with numbers, a third over the first (the engine finds its number another place in it), a parcel too
 * small to label at 1:1000, a named point and a street, each with its kind's default style (the drawing's).
 */
function drawing() {
  const h = toolHarness();
  const picker = new PickIndex(h.doc);
  pickers.push(picker);
  (h.ctx.view as unknown as { geometry: PickIndex }).geometry = picker;
  const square = (x0: number, y0: number, x1: number, y1: number, label: string) => h.add({ kind: 'polygon', pts: [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)], label });
  square(0, 0, 20, 15, '101');
  square(20, 0, 40, 15, '102');
  square(2, 1, 18, 14, '104');
  square(8, 6, 11, 9, '103');
  h.add({ kind: 'point', p: pt(45, 5), label: 'P1' });
  h.add({ kind: 'polyline', layerId: 'yol', pts: [pt(50, 20), pt(0, 20)], label: 'Cumhuriyet Cd.' });
  return h;
}

const texts = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is TextEntity => e.kind === 'text');

describe('Etiketleri yazıya çevir', () => {
  it('writes the labels as a sheet would, in one step on the text layer it opens', () => {
    const h = drawing();
    const n0 = h.doc.size;
    const tool = h.use(new LabelsToTextTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual([`${LABEL}: bütün çizimde 6 nesne; 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak. Enter ile yazın.`]);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak ${OPTIONS()}`);
    tool.pointerMove(at(60, 10));
    const log = canvasLog();
    tool.draw(log.g, log.view);
    expect(log.texts.slice(-3)).toEqual(['5 yazı, 1:1000', '1 küçük atlanır', 'Enter: yaz']);
    // Örtüşenler de: every label has a place here; nothing more to write.
    expect(tool.input('R')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak ${OPTIONS(1000, 'açık')}`);
    // A typed scale: the parcels' labels grow to their 14 px cap.
    expect(tool.input('1:500')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:500 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak ${OPTIONS(500, 'açık')}`);
    tool.confirm();
    const made = texts(h);
    const by = (text: string) => made.find((t) => t.text === text)!;
    expect(made.map((t) => t.text).sort()).toEqual(['101', '102', '104', 'Cumhuriyet Cd.', 'P1']);
    expect(made.every((t) => t.layerId === 'yazi')).toBe(true);
    expect(h.doc.layers.get('yazi')?.name).toBe('Yazılar');
    const k = 96 / 0.0254 / 500;
    // A parcel's number at its pole, the middle of a rectangle.
    expect(by('101')).toMatchObject({ align: 'middleCenter', rotation: 0 });
    expect(by('101').p.x).toBeCloseTo(10, 6);
    expect(by('101').p.y).toBeCloseTo(7.5, 6);
    expect(by('101').height).toBeCloseTo(14 / k, 12);
    // The third parcel's number is not on the first's.
    expect(Math.hypot(by('104').p.x - 10, by('104').p.y - 7.5)).toBeGreaterThan(0.5);
    // The point's name at its first place round it: to its top right.
    expect(by('P1').align).toBe('middleCenter');
    expect(by('P1').p.x).toBeGreaterThan(45);
    expect(by('P1').p.y).toBeGreaterThan(5);
    // The street runs west: its name turns half round to read.
    expect(by('Cumhuriyet Cd.')).toMatchObject({ rotation: 0, align: 'middleCenter' });
    expect(by('Cumhuriyet Cd.').p.x).toBeCloseTo(25, 6);
    expect(by('Cumhuriyet Cd.').p.y).toBeCloseTo(20, 6);
    expect(made.some((t) => t.mask)).toBe(false);
    expect([...h.ctx.selection.ids.value]).toEqual(made.map((t) => t.id));
    expect(h.said().slice(-2)).toEqual(['“Yazılar” katmanı çizimde yoktu; etiketlerin yazıları için açıldı.', `${LABEL}: 5 etiket yazıya çevrildi, 1 küçük etiket atlandı.`]);
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe(LABEL);
    expect(h.doc.size).toBe(n0);
    expect(h.doc.layers.get('yazi')).toBeUndefined();
  });

  it('takes a selection’s labels only, writes masked texts on the active layer, and keeps its options', () => {
    const h = drawing();
    const ids = [...h.doc.all()].filter((e) => e.label === '102' || e.label === 'P1').map((e) => e.id);
    h.ctx.selection.set(ids);
    const tool = h.use(new LabelsToTextTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual([`${LABEL}: seçimde 2 nesne; 1:1000 ölçekte 2 yazı olacak. Enter ile yazın.`]);
    expect(tool.input('Z')).toBe(true);
    expect(tool.input('K')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 2 yazı olacak ${OPTIONS(1000, 'kapalı', 'açık', 'Çizim')}`);
    tool.confirm();
    const made = texts(h);
    expect(made.map((t) => [t.text, t.layerId, t.mask]).sort()).toEqual([
      ['102', 'cizim', true],
      ['P1', 'cizim', true],
    ]);
    expect(h.doc.undo()).toBe(LABEL);
    expect(LabelsToTextTool.mask && LabelsToTextTool.active).toBe(true);
  });

  it('writes texts linked to their objects with Nesneye bağlı: they follow them, and their labels leave the scope', () => {
    const h = drawing();
    const tool = h.use(new LabelsToTextTool(h.ctx));
    tool.activate();
    expect(tool.input('B')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak ${OPTIONS(1000, 'kapalı', 'kapalı', 'Yazılar', 'açık')}`);
    tool.confirm();
    const made = texts(h);
    expect(made.map((t) => t.text).sort()).toEqual(['101', '102', '104', 'Cumhuriyet Cd.', 'P1']);
    // Each knows its object (the label it writes) and the scale.
    expect(made.map((t) => [h.doc.byUid(t.labelOf!)?.label, t.labelScale]).sort()).toEqual([
      ['101', 1000],
      ['102', 1000],
      ['104', 1000],
      ['Cumhuriyet Cd.', 1000],
      ['P1', 1000],
    ]);
    // The parcel moves: its text follows in the same step (the engine's place for it alone), and one undo takes both back.
    const text = made.find((t) => t.text === '102')!;
    const parcel = h.doc.byUid(text.labelOf!)!;
    if (parcel.kind !== 'polygon') throw new Error('a parcel');
    h.doc.update(parcel.id, { pts: parcel.pts.map((p) => ({ x: p.x, y: p.y - 10 })) });
    const moved = h.doc.get(text.id) as TextEntity;
    expect(moved.p.x).toBeCloseTo(30, 6);
    expect(moved.p.y).toBeCloseTo(7.5 - 10, 6);
    expect(moved.labelOf).toBe(text.labelOf);
    expect(h.doc.undo()).toBe('Değiştir');
    expect((h.doc.get(text.id) as TextEntity).p).toEqual(text.p);
    // Run again on the whole drawing: the linked objects' labels are texts now; the small 103's is left.
    h.ctx.selection.set([]);
    const again = h.use(new LabelsToTextTool(h.ctx));
    again.activate();
    expect(h.said().at(-1)).toBe(`${LABEL}: bütün çizimde 1 nesne; 1:1000 ölçekte yazı olacak etiket yok, 1 küçük etiket atlanacak; başka bir ölçek yazın.`);
  });

  it('asks for the scale with Ö, says a wrong one, and leaves at once when there is no label', () => {
    const h = drawing();
    const tool = h.use(new LabelsToTextTool(h.ctx));
    tool.activate();
    expect(tool.input('Ö')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: ölçeği 1:N ya da N olarak yazın (Enter: 1:1000)`);
    expect(tool.input('1:0')).toBe(true);
    expect(h.said().at(-1)).toBe('Ölçeği 1:N ya da N olarak, 1 ya da daha büyük bir tam sayıyla yazın (1:500, 1000).');
    expect(tool.input('5000')).toBe(true);
    // At 1:5000 the point and the street are below their styles' scales, the parcels too small.
    expect(h.said().at(-1)).toBe(`${LABEL}: 1:5000 ölçekte yazı olacak etiket yok, 2 ölçek dışı, 4 küçük etiket atlanacak; başka bir ölçek yazın.`);
    expect(tool.input('abc')).toBe(false);

    const empty = toolHarness();
    const picker = new PickIndex(empty.doc);
    pickers.push(picker);
    (empty.ctx.view as unknown as { geometry: PickIndex }).geometry = picker;
    empty.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const none = empty.use(new LabelsToTextTool(empty.ctx));
    none.activate();
    expect(empty.said()).toEqual([`${LABEL}: çizimde yazıya çevrilecek etiket yok.`]);
  });

  it('names a CAD project’s text layer as its template does', () => {
    const h = drawing();
    h.doc.settings.workspace.set('cad');
    const tool = h.use(new LabelsToTextTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value.endsWith(OPTIONS(1000, 'kapalı', 'kapalı', 'Yazı'))).toBe(true);
    tool.confirm();
    expect(h.doc.layers.get('yazi')?.name).toBe('Yazı');
  });

  it('reads a scale as 1:N or N', () => {
    expect([readScale('1:500'), readScale(' 1 : 2500 '), readScale('1000'), readScale('1:0'), readScale('0'), readScale('1:5.5'), readScale('2:500'), readScale('')]).toEqual([500, 2500, 1000, null, null, null, null, null]);
  });
});
