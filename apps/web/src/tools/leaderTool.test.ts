import { describe, expect, it } from 'vitest';
import type { LeaderEntity } from '../model/entities';
import { TextTool } from './annotateTools';
import { LeaderTool } from './leaderTool';
import { at, toolHarness } from './toolHarness';

/**
 * Kılavuz (docs/adr/0146 §7): the tip and the vertices clicked, the note typed in the field that opens past the
 * landing on the note's side, one step (“Kılavuz”); the empty field's Enter writes the arrow alone. Ok's menu, Zemin,
 * Geri, Yazı's shared height, Esc one step back, the locked layer. The desktop's are
 * crates/native/interaction/tests/all/leaders.rs. Places worked out by hand: 2.5 mm at 1:1000 is 2.5 m.
 */
/** A run from the defaults: the options outlive a run (as Yazı's do), so each test sets them back first. */
function started() {
  const h = toolHarness();
  const tool = h.use(new LeaderTool(h.ctx));
  tool.activate();
  tool.chooseOption('O', 'dolu');
  if (LeaderTool.options().mask) tool.input('Z');
  TextTool.setHeight(2.5);
  return { h, tool };
}

const leaders = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is LeaderEntity => e.kind === 'leader');

describe('Kılavuz', () => {
  it('opens the note past the landing, on the side the last segment goes, and writes the leader in one step', () => {
    const { h, tool } = started();
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(6, 5));
    expect(tool.prompt.value).toBe('Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı / Geri (G)]');
    tool.confirm();
    // The landing 5 m east of (6, 5), the note 1.25 m on: its middle left at (12.25, 5).
    const field = h.state.textInputs.at(-1)!;
    expect([field.at, field.height, field.rotation, field.align]).toEqual([{ x: 12.25, y: 5 }, 2.5, 0, 'middleLeft']);
    expect(tool.prompt.value).toBe('Kılavuz: notu kolun ucuna yazın; Enter ekler, boş Enter notsuz ekler, Esc köşelere döner');
    field.commit('Mevcut bina');
    const [l] = leaders(h);
    expect([l.pts, l.text, l.height, l.rotation, l.arrow, l.mask]).toEqual([[{ x: 0, y: 0 }, { x: 6, y: 5 }], 'Mevcut bina', 2.5, 0, undefined, undefined]);
    expect(h.said().at(-1)).toBe('Kılavuz eklendi: “Mevcut bina”');
    expect(tool.prompt.value).toBe('Kılavuz: okun ucuna tıklayın [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı]');
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
    expect(tool.optionChoices('O')!.map((c) => [c.label, c.icon, c.checked])).toEqual([
      ['Dolu', 'leaderArrowFilled', true],
      ['Açık', 'leaderArrowOpen', false],
      ['Nokta', 'leaderArrowDot', false],
      ['Yok', 'leaderArrowNone', false],
    ]);
    expect(tool.chooseOption('O', 'nokta')).toBe(true);
    tool.input('O');
    expect(tool.prompt.value).toBe('Kılavuz: ok başını seçin ya da adını yazın: dolu, açık, nokta, yok [Ok (O): nokta]');
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
    expect(h.said().at(-1)).toBe('“üçgen” bir ok başı adı değil. Ok başını menüden seçin ya da adını yazın: dolu, açık, nokta, yok.');
    tool.input('dolu');
    expect(LeaderTool.options()).toEqual({ arrow: null, mask: true });
  });

  it('Yükseklik is Yazı’s: typed here, the text tool writes with it too', () => {
    const { h, tool } = started();
    tool.input('Y');
    expect(tool.prompt.value).toBe('Kılavuz: kâğıt üzerindeki not yüksekliğini mm olarak yazın (Yazı ile ortak)');
    expect(tool.input('4')).toBe(true);
    expect(TextTool.options().heightMm).toBe(4);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(4, 3));
    tool.confirm();
    expect(h.state.textInputs.at(-1)!.height).toBe(4);
  });

  it('Esc steps back: out of the field to the vertices, then the leader drawn, then out of the tool', () => {
    const { h, tool } = started();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(4, 3));
    tool.confirm();
    h.state.textInputs.at(-1)!.cancel();
    expect(tool.prompt.value).toBe('Kılavuz: sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır [Ok (O): dolu / Yükseklik (Y): 2.5 mm / Zemin (Z): kapalı / Geri (G)]');
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
