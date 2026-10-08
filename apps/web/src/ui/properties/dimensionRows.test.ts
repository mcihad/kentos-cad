import { describe, expect, it } from 'vitest';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import type { DimensionEntity } from '../../model/entities';
import { pt, toolHarness } from '../../tools/toolHarness';
import { dimensionRows } from './dimensionRows';

/**
 * Öznitelikler's Ölçü rows (docs/adr/0147 §7) as data: Zemin; the lines' colours, weights and types and the value's
 * colour (docs/adr/0205 §6); an ordinate's Koordinat; a slope's two elevations; an arc length's radius and angle,
 * shown only. Their common value or “Çeşitli”, and what choosing or typing writes, in
 * one step “Değiştir”, without a DOM. The desktop's are apps/desktop/src/properties/tests.rs
 * (`a_dimensions_rows_write_…`). Expected values are worked out by hand.
 */
type Harness = ReturnType<typeof toolHarness>;
const get = (h: Harness, d: DimensionEntity) => h.doc.get(d.id) as DimensionEntity;
const row = (rows: PropRow[], label: string) => rows.find((r) => r.label === label)!;
const items = (r: PropRow) => (r.editor as { items: () => MenuItem[] }).items();
const choose = (r: PropRow, label: string) => items(r).find((i) => i.label === label)!.run!();
const commit = (r: PropRow, text: string) => (r.editor as { commit: (v: string) => void }).commit(text);
const dimension = (h: Harness, d: Partial<DimensionEntity>) =>
  h.add({ kind: 'dimension', a: pt(0, 0), b: pt(10, 0), offset: 3, height: 2.5, ...d } as DimensionEntity) as DimensionEntity;
/** Every dimension's rows of its lines (docs/adr/0205 §6), after Zemin, as a styleless one has them. */
const LINES: [string, string][] = [
  ['Çizgi rengi', 'Nesnenin rengi'],
  ['Çizgi kalınlığı', 'Kılcal'],
  ['Çizgi tipi', 'Sürekli'],
  ['Uzatma rengi', 'Nesnenin rengi'],
  ['Uzatma kalınlığı', 'Kılcal'],
  ['Uzatma tipi', 'Sürekli'],
  ['Değer rengi', 'Nesnenin rengi'],
];

describe("Öznitelikler's dimension rows", () => {
  it("an arc length's radius and angle, shown only", () => {
    const h = toolHarness();
    const arc = dimension(h, { style: 'arcLength', a: pt(10, 40), b: pt(0, 50), c: pt(0, 40) });
    const rows = dimensionRows(h.ctx, [arc], false);
    expect(rows.map((r) => [r.label, r.value])).toEqual([['Zemin', 'Kapalı'], ...LINES, ['Yarıçap', '10.000'], ['Açı', '100.0000']]);
    expect(row(rows, 'Yarıçap').editor).toBeUndefined();
    expect(row(rows, 'Açı').editor).toBeUndefined();
  });

  it('ordinates: Koordinat and Zemin say Çeşitli; a choice is written in one step, nothing for those that have it', () => {
    const h = toolHarness();
    const oy = dimension(h, { style: 'ordinate', angle: 0, a: pt(0, 60), b: pt(5, 70) });
    const ox = dimension(h, { style: 'ordinate', angle: 90, mask: true, a: pt(0, 80), b: pt(-10, 85) });
    const rows = dimensionRows(h.ctx, [oy, ox], false);
    expect(['Zemin', 'Koordinat'].map((l) => row(rows, l).value)).toEqual(['Çeşitli', 'Çeşitli']);
    choose(row(rows, 'Koordinat'), 'X');
    expect([get(h, oy).angle, get(h, ox).angle]).toEqual([90, 90]);
    expect(row(dimensionRows(h.ctx, [get(h, oy), get(h, ox)], false), 'Koordinat').value).toBe('X');
    const revision = h.doc.revision;
    choose(row(dimensionRows(h.ctx, [get(h, oy), get(h, ox)], false), 'Koordinat'), 'X');
    expect(h.doc.revision).toBe(revision);
    // Zemin on: the X's has it already, the Y's alone is written.
    choose(row(dimensionRows(h.ctx, [get(h, oy), get(h, ox)], false), 'Zemin'), 'Açık');
    expect([get(h, oy).mask, get(h, ox).mask]).toEqual([true, true]);
    expect(h.doc.undo()).toBe('Değiştir');
    expect(get(h, oy).mask).toBeUndefined();
  });

  it("slopes: their elevations, a typed one written to each; not a number, nothing; mixed kinds show Zemin only", () => {
    const h = toolHarness();
    const s1 = dimension(h, { style: 'slope', za: 100, zb: 99, a: pt(0, 100), b: pt(40, 100) });
    const s2 = dimension(h, { style: 'slope', za: 100, zb: 98, a: pt(0, 110), b: pt(40, 110) });
    const rows = dimensionRows(h.ctx, [s1, s2], false);
    expect(row(rows, 'Birinci kot').value).toBe('100.000');
    expect(row(rows, 'İkinci kot').value).toBe('Çeşitli');
    commit(row(rows, 'İkinci kot'), '97,5');
    expect([get(h, s1).zb, get(h, s2).zb]).toEqual([97.5, 97.5]);
    const revision = h.doc.revision;
    commit(row(dimensionRows(h.ctx, [get(h, s1)], false), 'Birinci kot'), 'kot');
    expect(h.doc.revision).toBe(revision);
    const oy = dimension(h, { style: 'ordinate', angle: 0, a: pt(0, 60), b: pt(5, 70) });
    expect(dimensionRows(h.ctx, [oy, get(h, s1)], false).map((r) => r.label)).toEqual(['Zemin', ...LINES.map(([l]) => l)]);
  });

  it("the lines' rows: common or Çeşitli; a colour, a weight and a type written in one step, none taking it away", () => {
    const h = toolHarness();
    const d1 = dimension(h, { dimLineColor: '#E5484D', extWeight: 0.35, extLineType: 'dashed' });
    const d2 = dimension(h, { a: pt(0, 20), b: pt(10, 20), dimLineColor: '#e5484d', textColor: '#123456' });
    const rows = dimensionRows(h.ctx, [d1, d2], false);
    // The same colour whatever its case; another program's by its value.
    expect(['Çizgi rengi', 'Uzatma kalınlığı', 'Uzatma tipi', 'Değer rengi'].map((l) => row(rows, l).value)).toEqual(['Kırmızı', 'Çeşitli', 'Çeşitli', 'Çeşitli']);
    expect(row(dimensionRows(h.ctx, [d2], false), 'Değer rengi').value).toBe('#123456');
    // The colours but ink: the object's is the first.
    expect(items(row(rows, 'Uzatma rengi')).map((i) => i.label)).toEqual(['Nesnenin rengi', undefined, 'Kırmızı', 'Sarı', 'Yeşil', 'Camgöbeği', 'Mavi', 'Eflatun', 'Gri']);
    choose(row(rows, 'Uzatma rengi'), 'Mavi');
    expect([get(h, d1).extColor, get(h, d2).extColor]).toEqual(['#4F8EF7', '#4F8EF7']);
    expect(h.doc.undo()).toBe('Değiştir');
    expect([get(h, d1).extColor, get(h, d2).extColor]).toEqual([undefined, undefined]);
    // A weight: the one that has it left out; Kılcal takes it away.
    choose(row(dimensionRows(h.ctx, [get(h, d1), get(h, d2)], false), 'Uzatma kalınlığı'), '0.35 mm');
    expect([get(h, d1).extWeight, get(h, d2).extWeight]).toEqual([0.35, 0.35]);
    choose(row(dimensionRows(h.ctx, [get(h, d1), get(h, d2)], false), 'Uzatma kalınlığı'), 'Kılcal');
    expect([get(h, d1).extWeight, get(h, d2).extWeight]).toEqual([undefined, undefined]);
    // A type: Sürekli takes it away.
    choose(row(dimensionRows(h.ctx, [get(h, d1), get(h, d2)], false), 'Çizgi tipi'), 'Noktalı kesik');
    expect([get(h, d1).dimLineType, get(h, d2).dimLineType]).toEqual(['dashdot', 'dashdot']);
    choose(row(dimensionRows(h.ctx, [get(h, d1), get(h, d2)], false), 'Uzatma tipi'), 'Sürekli');
    expect([get(h, d1).extLineType, get(h, d2).extLineType]).toEqual([undefined, undefined]);
    // The value's colour back to the object's.
    choose(row(dimensionRows(h.ctx, [get(h, d2)], false), 'Değer rengi'), 'Nesnenin rengi');
    expect(get(h, d2).textColor).toBeUndefined();
    const revision = h.doc.revision;
    choose(row(dimensionRows(h.ctx, [get(h, d1), get(h, d2)], false), 'Çizgi rengi'), 'Kırmızı');
    expect(h.doc.revision, 'both have it already').toBe(revision);
  });

  it('on a locked layer the rows only show', () => {
    const h = toolHarness();
    const s = dimension(h, { style: 'slope', za: 100, zb: 99 });
    expect(dimensionRows(h.ctx, [s], true).every((r) => r.editor === undefined)).toBe(true);
  });
});
