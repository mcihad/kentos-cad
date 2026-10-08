import { describe, expect, it } from 'vitest';
import type { LeaderEntity } from '../model/entities';
import { TextTool } from './annotateTools';
import { annotationHeightMm, setAnnotationHeightMm } from './annotationHeights';
import { LeaderTool } from './leaderTool';
import { at, toolHarness } from './toolHarness';

/**
 * Kılavuz (docs/adr/0146 §7): the tip and the vertices clicked, the note typed in the field that opens past the
 * landing on the note's side, one step (“Kılavuz”); the empty field's Enter writes the arrow alone. Ok's menu of
 * AutoCAD's arrowheads and Ok boyu (docs/adr/0205 §7), Zemin, Geri, its own height (the project's Kılavuz height until
 * one is typed in this drawing, §2), Esc one step back, the locked layer. The desktop's are
 * crates/native/interaction/tests/all/leaders.rs. Places worked out by hand: 2.5 mm at 1:1000 is 2.5 m.
 */
/** A run from the defaults: the options outlive a run (as Yazı's do), so each test sets them back first. */
function started() {
  const h = toolHarness();
  const tool = h.use(new LeaderTool(h.ctx));
  tool.activate();
  tool.chooseOption('O', 'dolu');
  if (LeaderTool.options().mask) tool.input('Z');
  if (LeaderTool.options().arrowSize !== null) tool.input('B'), tool.input('1');
  return { h, tool };
}

const leaders = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LeaderEntity => e.kind === 'leader');

describe('Kılavuz', () => {
  it('opens the note past the landing, on the side the last segment goes, and writes the leader in one step', () => {
    const { h, tool } = started();
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Ok boyu (B): 1 / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(6, 5));
    expect(tool.prompt.value).toBe('Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır [Ok (O): dolu / Ok boyu (B): 1 / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı / Geri (G)]');
    tool.confirm();
    // The landing 5 m east of (6, 5), the note 1.25 m on: its middle left at (12.25, 5).
    const field = h.state.textInputs.at(-1)!;
    expect([field.at, field.height, field.rotation, field.align]).toEqual([{ x: 12.25, y: 5 }, 2.5, 0, 'middleLeft']);
    expect(tool.prompt.value).toBe('Kılavuz: notu kolun ucuna yazın; Enter ekler, boş Enter notsuz ekler, Esc köşelere döner');
    field.commit('Mevcut bina');
    const [l] = leaders(h);
    expect([l.pts, l.text, l.height, l.rotation, l.arrow, l.mask]).toEqual([[{ x: 0, y: 0 }, { x: 6, y: 5 }], 'Mevcut bina', 2.5, 0, undefined, undefined]);
    expect(h.said().at(-1)).toBe('Kılavuz eklendi: “Mevcut bina”');
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Ok boyu (B): 1 / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]');
    expect(h.doc.undo()).toBe('Kılavuz');
  });

  it('a note to the left opens on its middle right; the empty field writes the arrow alone, unmasked', () => {
    const { h, tool } = started();
    tool.input('Z');
    tool.pointerDown(at(30, 0));
    tool.pointerDown(at(26, 4));
    tool.pointerDown(at(20, 6));
    tool.confirm();
    // The landing 5 m west of (20, 6), the note 1.25 m on: its middle right at (13.75, 6).
    const field = h.state.textInputs.at(-1)!;
    expect([field.at, field.align]).toEqual([{ x: 13.75, y: 6 }, 'middleRight']);
    field.empty!();
    const [l] = leaders(h);
    expect([l.pts.length, l.text, l.mask]).toEqual([3, undefined, undefined]);
    expect(h.said().at(-1)).toBe('Kılavuz eklendi (notsuz).');
  });

  it('Ok chooses the arrowhead from its menu or by its name; Zemin masks the note; Geri takes the last vertex back', () => {
    const { h, tool } = started();
    expect(tool.optionChoices('O')!.map((c) => [c.label, c.typed, c.icon, c.checked])).toEqual([
      ['Dolu üçgen', 'dolu', 'leaderArrowFilled', true],
      ['Boş üçgen', 'boş', 'leaderArrowClosed', false],
      ['Açık ok', 'açık', 'leaderArrowOpen', false],
      ['İnce açık ok', 'ince', 'leaderArrowOpen30', false],
      ['Dik açık ok', 'dik', 'leaderArrowOpen90', false],
      ['Dolu nokta', 'nokta', 'leaderArrowDot', false],
      ['Küçük nokta', 'küçük nokta', 'leaderArrowDotSmall', false],
      ['Boş nokta', 'boş nokta', 'leaderArrowDotBlank', false],
      ['Eğik çizgi', 'eğik', 'leaderArrowOblique', false],
      ['Mimari çentik', 'çentik', 'leaderArrowArchTick', false],
      ['Dolu kare', 'kare', 'leaderArrowBoxFilled', false],
      ['Boş kare', 'boş kare', 'leaderArrowBoxBlank', false],
      ['Dayanak üçgeni', 'dayanak', 'leaderArrowDatum', false],
      ['Yok', 'yok', 'leaderArrowNone', false],
    ]);
    expect(tool.chooseOption('O', 'nokta')).toBe(true);
    tool.input('O');
    expect(tool.prompt.value).toBe('Kılavuz: ok başını menüden seçin ya da adını yazın [Ok (O): nokta]');
    // Two words are one name there; the label typed is read too.
    expect(tool.takesWords()).toBe(true);
    tool.input('Boş kare');
    expect(LeaderTool.options().arrow).toBe('boxBlank');
    tool.input('O');
    tool.input('dayanak üçgeni');
    expect(LeaderTool.options().arrow).toBe('datumFilled');
    tool.input('O');
    tool.input('Acik');
    tool.input('Z');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(3, 3));
    tool.pointerDown(at(9, 3));
    tool.input('G');
    tool.pointerDown(at(8, 6));
    tool.confirm();
    h.state.textInputs.at(-1)!.commit('Ø150 PVC');
    const [l] = leaders(h);
    expect([l.pts, l.arrow, l.mask]).toEqual([[{ x: 0, y: 0 }, { x: 3, y: 3 }, { x: 8, y: 6 }], 'open', true]);
    // A name that names none is said, and the tool waits on.
    tool.input('O');
    tool.input('üçgen');
    expect(h.said().at(-1)).toBe(
      '“üçgen” bir ok başı adı değil. Ok başını menüden seçin ya da adını yazın: dolu, boş, açık, ince, dik, nokta, küçük nokta, boş nokta, eğik, çentik, kare, boş kare, dayanak, yok.',
    );
    tool.input('dolu');
    expect(LeaderTool.options()).toEqual({ arrow: null, arrowSize: null, mask: true });
  });

  it('Ok boyu sets the arrowhead’s length in the note’s height, 0.1 to 10; the leader keeps it (docs/adr/0205 §7)', () => {
    const { h, tool } = started();
    tool.input('B');
    expect(tool.prompt.value).toBe('Kılavuz: ok boyunu notun yüksekliğinin katı olarak yazın, 0.1 ile 10 arası [Ok boyu (B): 1]');
    tool.input('20');
    expect(h.said().at(-1)).toBe('Ok boyu notun yüksekliğinin 0.1 ile 10 katı olur; 20 yazıldı.');
    tool.input('1.5');
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Ok boyu (B): 1.5 / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(4, 3));
    tool.confirm();
    h.state.textInputs.at(-1)!.commit('Bina');
    const [l] = leaders(h);
    expect([l.arrow, l.arrowSize]).toEqual([undefined, 1.5]);
    // 1 is the field's absence.
    tool.input('B');
    tool.input('1');
    expect(LeaderTool.options().arrowSize).toBe(null);
  });

  it('Yükseklik is its own: the project’s Kılavuz height until one is typed in this drawing; another drawing forgets it (docs/adr/0205 §2)', () => {
    const { h, tool } = started();
    h.doc.settings.assign({ annotation: { leader: 3.5 } });
    tool.input('Z');
    tool.input('Z');
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Ok boyu (B): 1 / Yükseklik (Y): 3.5 mm / Zemin (Z): kapalı]');
    tool.input('Y');
    expect(tool.prompt.value).toBe('Kılavuz: kâğıt üzerindeki not yüksekliğini mm olarak yazın');
    expect(tool.input('4')).toBe(true);
    // Yazı's is not Kılavuz's.
    expect([annotationHeightMm(h.ctx, 'leader'), TextTool.options(h.ctx).heightMm]).toEqual([4, 2.5]);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(4, 3));
    tool.confirm();
    expect(h.state.textInputs.at(-1)!.height).toBe(4);
    h.state.textInputs.at(-1)!.cancel();
    tool.cancel();
    // A drawing opened: what was typed in the last one is forgotten, its own project's height is the one.
    h.doc.replaceWith({
      name: 'Öbür',
      settings: { ...h.doc.settings.toJSON(), annotation: { leader: 3 } },
      origin: { x: 0, y: 0 },
      homeView: null,
      layers: [{ id: 'cizim', name: 'Çizim' }],
      activeLayer: 'cizim',
      entities: [],
      styles: { items: [], categories: [] },
    });
    expect(annotationHeightMm(h.ctx, 'leader')).toBe(3);
    setAnnotationHeightMm(h.ctx, 'leader', null);
  });

  it('Esc steps back: out of the field to the vertices, then the leader drawn, then out of the tool', () => {
    const { h, tool } = started();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(4, 3));
    tool.confirm();
    h.state.textInputs.at(-1)!.cancel();
    expect(tool.prompt.value).toBe('Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır [Ok (O): dolu / Ok boyu (B): 1 / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı / Geri (G)]');
    expect(tool.cancel()).toBe(true);
    expect(tool.pointCount).toBe(0);
    expect(tool.cancel()).toBe(false);
    expect(leaders(h)).toEqual([]);
  });

  it('with the tip alone Enter says what is missing; a locked layer is said at the tip', () => {
    const { h, tool } = started();
    tool.pointerDown(at(0, 0));
    tool.confirm();
    expect(h.said().at(-1)).toBe('Kılavuzun en az 2 köşesi olur: okun ucundan sonra bir köşeye daha tıklayın.');
    expect(h.state.textInputs).toEqual([]);
    tool.cancel();
    h.doc.layers.setActive('kilitli');
    tool.pointerDown(at(0, 0));
    expect(tool.pointCount).toBe(0);
    expect(h.said().at(-1)).toBe('“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.');
  });
});
