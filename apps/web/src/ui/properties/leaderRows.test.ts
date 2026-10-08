import { describe, expect, it } from 'vitest';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import type { LeaderEntity } from '../../model/entities';
import { pt, toolHarness } from '../../tools/toolHarness';
import { leaderRows } from './leaderRows';

/**
 * Öznitelikler's Kılavuz rows (docs/adr/0146 §7) as data: their common value or “Çeşitli”, and what choosing or
 * typing writes, in one step “Değiştir”, over a document and a log, without a DOM. The desktop's are
 * apps/desktop/src/properties/tests.rs (`a_leaders_rows_write_…`). Expected values are worked out by hand.
 */
type Harness = ReturnType<typeof toolHarness>;
const get = (h: Harness, l: LeaderEntity) => h.doc.get(l.id) as LeaderEntity;
const row = (rows: PropRow[], label: string) => rows.find((r) => r.label === label)!;
const items = (r: PropRow) => (r.editor as { items: () => MenuItem[] }).items();
const choose = (r: PropRow, label: string) => items(r).find((i) => i.label === label)!.run!();
const commit = (r: PropRow, text: string) => (r.editor as { commit: (v: string) => void }).commit(text);

function scene() {
  const h = toolHarness();
  const a = h.add({ kind: 'leader', pts: [pt(0, 20), pt(4, 23)], text: 'Mevcut bina', height: 2, rotation: 0 }) as LeaderEntity;
  const b = h.add({ kind: 'leader', pts: [pt(20, 20), pt(24, 23)], text: 'Ø150 PVC', height: 2, rotation: 0, arrow: 'open', mask: true }) as LeaderEntity;
  return { h, a, b };
}

describe("Öznitelikler's leader rows", () => {
  it("one leader's values; the Ok menu is AutoCAD's arrowheads, its own checked, with their icons", () => {
    const { h, a } = scene();
    const rows = leaderRows(h.ctx, [a], false);
    expect(rows.map((r) => [r.label, r.value])).toEqual([
      ['Not', 'Mevcut bina'],
      ['Yükseklik', '2.000'],
      ['Dönüş', '0.00'],
      ['Ok', 'Dolu üçgen'],
      ['Ok boyu', '1.00'],
      ['Zemin', 'Kapalı'],
    ]);
    const menu = items(row(rows, 'Ok'));
    expect(menu.map((i) => [i.label, i.icon, i.checked])).toEqual([
      ['Dolu üçgen', 'leaderArrowFilled', true],
      ['Boş üçgen', 'leaderArrowClosed', false],
      ['Açık ok', 'leaderArrowOpen', false],
      ['İnce açık ok', 'leaderArrowOpen30', false],
      ['Dik açık ok', 'leaderArrowOpen90', false],
      ['Dolu nokta', 'leaderArrowDot', false],
      ['Küçük nokta', 'leaderArrowDotSmall', false],
      ['Boş nokta', 'leaderArrowDotBlank', false],
      ['Eğik çizgi', 'leaderArrowOblique', false],
      ['Mimari çentik', 'leaderArrowArchTick', false],
      ['Dolu kare', 'leaderArrowBoxFilled', false],
      ['Boş kare', 'leaderArrowBoxBlank', false],
      ['Dayanak üçgeni', 'leaderArrowDatum', false],
      ['Yok', 'leaderArrowNone', false],
    ]);
  });

  it('Ok boyu writes the arrowhead size in the note’s height, 0.1 to 10; 1 is no field (docs/adr/0205 §7)', () => {
    const { h, a, b } = scene();
    const revision = h.doc.revision;
    commit(row(leaderRows(h.ctx, [a, b], false), 'Ok boyu'), '12');
    expect(h.doc.revision).toBe(revision);
    commit(row(leaderRows(h.ctx, [a, b], false), 'Ok boyu'), '1,5');
    expect([get(h, a).arrowSize, get(h, b).arrowSize]).toEqual([1.5, 1.5]);
    expect(row(leaderRows(h.ctx, [get(h, a), get(h, b)], false), 'Ok boyu').value).toBe('1.50');
    commit(row(leaderRows(h.ctx, [get(h, a)], false), 'Ok boyu'), '1');
    expect('arrowSize' in get(h, a)).toBe(false);
    expect(row(leaderRows(h.ctx, [get(h, a), get(h, b)], false), 'Ok boyu').value).toBe('Çeşitli');
  });

  it('a selection that differs says Çeşitli; an arrowhead is written to every leader in one step', () => {
    const { h, a, b } = scene();
    const rows = leaderRows(h.ctx, [a, b], false);
    expect(['Not', 'Ok', 'Zemin'].map((l) => row(rows, l).value)).toEqual(['Çeşitli', 'Çeşitli', 'Çeşitli']);
    expect(row(rows, 'Yükseklik').value).toBe('2.000');
    choose(row(rows, 'Ok'), 'Dolu nokta');
    expect([get(h, a).arrow, get(h, b).arrow]).toEqual(['dot', 'dot']);
    expect(h.doc.undo()).toBe('Değiştir');
    expect([get(h, a).arrow, get(h, b).arrow]).toEqual([undefined, 'open']);
  });

  it('Zemin, Dönüş, Yükseklik and Not: nothing written for what has the value; a height not over 0 is not taken; an emptied note is the arrow alone', () => {
    const { h, a, b } = scene();
    choose(row(leaderRows(h.ctx, [a, b], false), 'Zemin'), 'Açık');
    expect([get(h, a).mask, get(h, b).mask]).toEqual([true, true]);
    commit(row(leaderRows(h.ctx, [get(h, a), get(h, b)], false), 'Dönüş'), '370');
    expect(get(h, b).rotation).toBe(10);
    const revision = h.doc.revision;
    commit(row(leaderRows(h.ctx, [get(h, a)], false), 'Yükseklik'), '0');
    expect(h.doc.revision).toBe(revision);
    commit(row(leaderRows(h.ctx, [get(h, a)], false), 'Yükseklik'), '3,5');
    expect(get(h, a).height).toBe(3.5);
    commit(row(leaderRows(h.ctx, [get(h, b)], false), 'Not'), '  ');
    expect('text' in get(h, b)).toBe(false);
    commit(row(leaderRows(h.ctx, [get(h, b)], false), 'Not'), ' Ø200 PVC ');
    expect(get(h, b).text).toBe('Ø200 PVC');
    expect(h.doc.undo()).toBe('Değiştir');
    expect('text' in get(h, b)).toBe(false);
  });

  it('on a locked layer the rows only show', () => {
    const { h, a } = scene();
    expect(leaderRows(h.ctx, [a], true).every((r) => r.editor === undefined)).toBe(true);
  });
});
