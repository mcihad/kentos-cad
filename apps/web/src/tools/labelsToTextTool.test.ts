import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { TextEntity } from '../model/entities';
import { PickIndex } from '../viewport/picking';
import { LabelsToTextTool, readScale } from './labelsToTextTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Etiketleri yazıya çevir (docs/adr/0175 §3): the labels taken when it starts, the finding shown first, the scale
 * and the three options, the texts written in one step on the standard text layer (opened in that step) or the
 * active one. The places and sizes are the core's, checked against the independent reference in
 * model/ops/labelText.test.ts; here the tool's flow. The desktop walks the same in
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
 * Two parcels with numbers, a third whose label falls on the first's, a parcel too small to label at 1:1000, a
 * named point and a street, each with its kind's default style (the drawing's).
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
    expect(h.said()).toEqual([`${LABEL}: bütün çizimde 6 etiket; 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak. Enter ile yazın.`]);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak ${OPTIONS()}`);
    tool.pointerMove(at(60, 10));
    const log = canvasLog();
    tool.draw(log.g, log.view);
    expect(log.texts.slice(-4)).toEqual(['4 yazı, 1:1000', '1 örtüşen atlanır', '1 küçük atlanır', 'Enter: yaz']);
    // Örtüşenler de: the third parcel's number too.
    expect(tool.input('R')).toBe(true);
    expect(h.said().at(-1)).toBe(`${LABEL}: 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak.`);
    // A typed scale: the parcels' labels grow to their 14 px cap.
    expect(tool.input('1:500')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:500 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak ${OPTIONS(500, 'açık')}`);
    tool.confirm();
    const made = texts(h);
    expect(made.map((t) => t.text)).toEqual(['101', '102', '104', 'P1', 'Cumhuriyet Cd.']);
    expect(made.every((t) => t.layerId === 'yazi')).toBe(true);
    expect(h.doc.layers.get('yazi')?.name).toBe('Yazılar');
    const k = 96 / 0.0254 / 500;
    expect(made[0]).toMatchObject({ p: pt(10, 7.5), align: 'middleCenter', rotation: 0 });
    expect(made[0].height).toBeCloseTo(14 / k, 12);
    expect(made[3].align).toBe('middleLeft');
    expect(made[3].p.x).toBeCloseTo(45 + 7 / k, 9);
    // The street runs west: its name turns half round to read.
    expect(made[4]).toMatchObject({ p: pt(25, 20), rotation: 0, align: 'middleCenter' });
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
    expect(h.said()).toEqual([`${LABEL}: seçimde 2 etiket; 1:1000 ölçekte 2 yazı olacak. Enter ile yazın.`]);
    expect(tool.input('Z')).toBe(true);
    expect(tool.input('K')).toBe(true);
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 2 yazı olacak ${OPTIONS(1000, 'kapalı', 'açık', 'Çizim')}`);
    tool.confirm();
    const made = texts(h);
    expect(made.map((t) => [t.text, t.layerId, t.mask])).toEqual([
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
    expect(tool.prompt.value).toBe(`${LABEL}: 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak ${OPTIONS(1000, 'kapalı', 'kapalı', 'Yazılar', 'açık')}`);
    tool.confirm();
    const made = texts(h);
    expect(made.map((t) => t.text)).toEqual(['101', '102', 'P1', 'Cumhuriyet Cd.']);
    // Each knows its object (the label it writes) and the scale.
    expect(made.map((t) => [h.doc.byUid(t.labelOf!)?.label, t.labelScale])).toEqual([
      ['101', 1000],
      ['102', 1000],
      ['P1', 1000],
      ['Cumhuriyet Cd.', 1000],
    ]);
    // The parcel moves: its text follows in the same step, and one undo takes both back.
    const parcel = h.doc.byUid(made[1].labelOf!)!;
    if (parcel.kind !== 'polygon') throw new Error('a parcel');
    h.doc.update(parcel.id, { pts: parcel.pts.map((p) => ({ x: p.x, y: p.y - 10 })) });
    const moved = h.doc.get(made[1].id) as TextEntity;
    expect(moved.p.x).toBeCloseTo(made[1].p.x, 9);
    expect(moved.p.y).toBeCloseTo(made[1].p.y - 10, 9);
    expect(moved.labelOf).toBe(made[1].labelOf);
    expect(h.doc.undo()).toBe('Değiştir');
    expect((h.doc.get(made[1].id) as TextEntity).p).toEqual(made[1].p);
    // Run again on the whole drawing: the linked objects' labels are texts now; 104's and the small 103's are left.
    h.ctx.selection.set([]);
    const again = h.use(new LabelsToTextTool(h.ctx));
    again.activate();
    expect(h.said().at(-1)).toBe(`${LABEL}: bütün çizimde 2 etiket; 1:1000 ölçekte 1 yazı olacak, 1 küçük etiket atlanacak. Enter ile yazın.`);
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
