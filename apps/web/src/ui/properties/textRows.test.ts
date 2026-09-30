import { describe, expect, it } from 'vitest';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import type { TextEntity } from '../../model/entities';
import { textBox } from '../../model/entities';
import { pt, toolHarness } from '../../tools/toolHarness';
import { textRows } from './textRows';

/**
 * Öznitelikler's Hiza, Genişlik çarpanı and Zemin rows (docs/adr/0145 §6) as data: their common value or “Çeşitli”,
 * and what choosing or typing writes, over a document and a log, without a DOM. The desktop's are
 * apps/desktop/src/properties/tests.rs. Expected values are worked out by hand.
 */
type Harness = ReturnType<typeof toolHarness>;
const get = (h: Harness, t: TextEntity) => h.doc.get(t.id) as TextEntity;
const row = (rows: PropRow[], label: string) => rows.find((r) => r.label === label)!;
const items = (r: PropRow) => (r.editor as { items: () => MenuItem[] }).items();
const choose = (r: PropRow, label: string) => items(r).find((i) => i.label === label)!.run!();
const commit = (r: PropRow, text: string) => (r.editor as { commit: (v: string) => void }).commit(text);
/** A box's corners, rounded to a micrometre: where the text is drawn. */
const box = (h: Harness, t: TextEntity) => textBox({ ...t, font: h.ctx.doc.settings.drawingFont.value }).map((q) => [+q.x.toFixed(6), +q.y.toFixed(6)]);

function scene() {
  const h = toolHarness();
  const a = h.add({ kind: 'text', p: pt(10, 20), text: 'Ada 101', height: 2, rotation: 30 }) as TextEntity;
  const b = h.add({ kind: 'text', p: pt(10, 10), text: 'Ada 102', height: 2, rotation: 0, align: 'middleCenter', widthFactor: 0.8, mask: true }) as TextEntity;
  return { h, a, b };
}

describe("Öznitelikler's text rows", () => {
  it("one text's values; the Hiza menu is the twelve points, its own checked, with their icons", () => {
    const { h, b } = scene();
    const rows = textRows(h.ctx, [b], false);
    expect(rows.map((r) => [r.label, r.value])).toEqual([
      ['Hiza', 'Orta'],
      ['Genişlik çarpanı', '0.8'],
      ['Zemin', 'Açık'],
    ]);
    const menu = items(row(rows, 'Hiza'));
    expect(menu).toHaveLength(12);
    expect(menu.filter((i) => i.checked).map((i) => i.label)).toEqual(['Orta']);
    expect(menu[3]).toMatchObject({ label: 'Sol orta', icon: 'textAlignMiddleLeft' });
  });

  it('a selection that differs says Çeşitli', () => {
    const { h, a, b } = scene();
    const rows = textRows(h.ctx, [a, b], false);
    expect(rows.map((r) => r.value)).toEqual(['Çeşitli', 'Çeşitli', 'Çeşitli']);
    expect(items(row(rows, 'Hiza')).some((i) => i.checked)).toBe(false);
  });

  it('a new alignment keeps each text where it is, in one step “Değiştir”', () => {
    const { h, a, b } = scene();
    const before = [box(h, a), box(h, b)];
    choose(row(textRows(h.ctx, [a, b], false), 'Hiza'), 'Sağ üst');
    const [a2, b2] = [get(h, a), get(h, b)];
    expect([a2.align, b2.align]).toEqual(['topRight', 'topRight']);
    // The box is where it was, to a micrometre; the point moved to its top right corner.
    expect([box(h, a2), box(h, b2)]).toEqual(before);
    expect(a2.p).not.toEqual(a.p);
    expect(h.doc.undo()).toBe('Değiştir');
    expect([get(h, a).p, get(h, b).p, get(h, a).align, get(h, b).align]).toEqual([a.p, b.p, undefined, 'middleCenter']);
  });

  it('the left of the baseline takes the field away; the text still stays', () => {
    const { h, b } = scene();
    const before = box(h, b);
    choose(row(textRows(h.ctx, [b], false), 'Hiza'), 'Sol taban');
    expect('align' in get(h, b)).toBe(false);
    expect(box(h, get(h, b))).toEqual(before);
  });

  it('Genişlik çarpanı: written about each point; 1 is no field; out of range the command says why', () => {
    const { h, a, b } = scene();
    commit(row(textRows(h.ctx, [a, b], false), 'Genişlik çarpanı'), '1,5');
    expect([get(h, a).widthFactor, get(h, b).widthFactor, get(h, b).p]).toEqual([1.5, 1.5, b.p]);
    commit(row(textRows(h.ctx, [get(h, a)], false), 'Genişlik çarpanı'), '1');
    expect('widthFactor' in get(h, a)).toBe(false);
    commit(row(textRows(h.ctx, [get(h, b)], false), 'Genişlik çarpanı'), '0');
    expect(get(h, b).widthFactor).toBe(1.5);
    expect(h.said().at(-1)).toBe("Yazının genişlik çarpanı 0'dan büyük, en çok 100 olmalı; 0 verildi. Çarpanı bu aralıkta verin ya da alanı kaldırın (1).");
  });

  it('Zemin: on and off, one step each; nothing is written when nothing changes', () => {
    const { h, a, b } = scene();
    const revision = h.doc.revision;
    choose(row(textRows(h.ctx, [get(h, b)], false), 'Zemin'), 'Açık');
    expect(h.doc.revision).toBe(revision);
    choose(row(textRows(h.ctx, [a, b], false), 'Zemin'), 'Açık');
    expect([get(h, a).mask, get(h, b).mask]).toEqual([true, true]);
    choose(row(textRows(h.ctx, [get(h, a), get(h, b)], false), 'Zemin'), 'Kapalı');
    expect(['mask' in get(h, a), 'mask' in get(h, b)]).toEqual([false, false]);
    expect(h.doc.undo()).toBe('Değiştir');
    expect([get(h, a).mask, get(h, b).mask]).toEqual([true, true]);
  });

  it('on a locked layer the rows only show', () => {
    const { h } = scene();
    const locked = h.add({ kind: 'text', layerId: 'kilitli', p: pt(0, 0), text: 'Kilitli', height: 2, rotation: 0 }) as TextEntity;
    expect(textRows(h.ctx, [locked], true).every((r) => r.editor === undefined)).toBe(true);
  });
});
