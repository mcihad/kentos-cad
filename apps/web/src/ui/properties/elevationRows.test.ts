import { describe, expect, it } from 'vitest';
import type { Entity, LineEntity, PointEntity, PolylineEntity } from '../../model/entities';
import { pt, toolHarness } from '../../tools/toolHarness';
import { commonElevationRow, elevationRow, lineEndRow, parseElevation, pathElevationRow, spaceRow } from './elevationRows';

/**
 * The elevation rows of Öznitelikler (docs/adr/0142) as data: what they say of the vertices, what typing into them
 * writes, and the 3D length beside the plan one. Over a document and a log, without a DOM.
 */
const SQUARE = [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];
type Harness = ReturnType<typeof toolHarness>;
const get = <T extends Entity>(h: Harness, e: Entity) => h.doc.get(e.id) as T;
const commit = (row: { editor?: { type: string; commit?: (v: string) => void } }, text: string) => (row.editor as { commit: (v: string) => void }).commit(text);
const levels = (h: Harness) => h.log.entries.value.map((e) => `${e.level}: ${e.text}`);

describe('a Kot row says what the vertices hold', () => {
  const h = toolHarness();
  const rowOf = (zs: (number | null)[]) => elevationRow(h.ctx, 'Kot', zs);

  it('kot yok when none has one', () => {
    expect(rowOf([null, null])).toMatchObject({ label: 'Kot', value: 'kot yok' });
    expect(rowOf([null, null]).unit).toBeUndefined();
    expect(rowOf([null, null]).note).toBeUndefined();
  });

  it('the value, in metres, when every vertex has the same one', () => {
    expect(rowOf([100, 100, 100])).toMatchObject({ value: '100.000', unit: 'm' });
    expect(rowOf([-4.25])).toMatchObject({ value: '-4.250', unit: 'm' });
    expect(rowOf([0, 0])).toMatchObject({ value: '0.000', unit: 'm' });
  });

  it('the range, lowest to highest with an en dash and no spaces, when they differ', () => {
    expect(rowOf([105.25, 98.5, 101])).toMatchObject({ value: '98.500–105.250 m' });
    expect(rowOf([105.25, 98.5, 101]).unit).toBeUndefined();
    expect(rowOf([105.25, 98.5, 101]).note).toBeUndefined();
  });

  it('a four-digit range (Sivas, Suşehri: 1100 to 1300 m) is its numbers, the dash and the unit: 1098.500–1105.250 m', () => {
    expect(rowOf([1098.5, 1105.25, 1101])).toMatchObject({ value: '1098.500–1105.250 m' });
    expect(rowOf([1098.5, null, 1305.25])).toMatchObject({ value: '1098.500–1305.250 m', note: '(bazı köşeler kotsuz)' });
  });

  it('the range of those that have one, with the remark that some vertices have none', () => {
    expect(rowOf([105.25, null, 98.5])).toMatchObject({ value: '98.500–105.250 m', note: '(bazı köşeler kotsuz)' });
    // Those that have one agree: the value, with the remark.
    expect(rowOf([100, null, 100])).toMatchObject({ value: '100.000', unit: 'm', note: '(bazı köşeler kotsuz)' });
  });

  it('numbers go through the project’s formatter: its decimals', () => {
    const f = toolHarness();
    (f.ctx.format as unknown as { prefs: { lengthDecimals: { set(n: number): void } } }).prefs.lengthDecimals.set(1);
    expect(elevationRow(f.ctx, 'Kot', [98.54, 105.26])).toMatchObject({ value: '98.5–105.3 m' });
  });

  it('is not editable without a writer (a locked layer)', () => {
    expect(rowOf([1, 2]).editor).toBeUndefined();
  });
});

describe('what is typed into a Kot row', () => {
  it('a number, with a decimal point or comma and a trailing m allowed; nothing clears; anything else is not taken', () => {
    expect(parseElevation('105.25')).toBe(105.25);
    expect(parseElevation(' -4,5 ')).toBe(-4.5);
    expect(parseElevation('12 m')).toBe(12);
    expect(parseElevation('12m')).toBe(12);
    expect(parseElevation('0')).toBe(0);
    expect(parseElevation('')).toBeNull();
    expect(parseElevation('   ')).toBeNull();
    expect(parseElevation('kot yok')).toBeUndefined();
    expect(parseElevation('12abc')).toBeUndefined();
    expect(parseElevation('1e3')).toBeUndefined();
    expect(parseElevation('1,2,3')).toBeUndefined();
  });
});

describe('a line’s two ends', () => {
  it('Kot (başlangıç) and Kot (bitiş): each end’s value, or kot yok', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 10 });
    const line = get<LineEntity>(h, e);
    expect(lineEndRow(h.ctx, line, 0, false)).toMatchObject({ label: 'Kot (başlangıç)', value: '10.000', unit: 'm' });
    expect(lineEndRow(h.ctx, line, 1, false)).toMatchObject({ label: 'Kot (bitiş)', value: 'kot yok' });
  });

  it('a number sets that end and leaves the other, as one step named Kot ver', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 10, zb: 20 });
    commit(lineEndRow(h.ctx, get(h, e), 1, false), '25.5');
    expect(get<LineEntity>(h, e)).toMatchObject({ za: 10, zb: 25.5 });
    expect(h.doc.undo()).toBe('Kot ver');
    expect(get<LineEntity>(h, e)).toMatchObject({ za: 10, zb: 20 });
    commit(lineEndRow(h.ctx, get(h, e), 0, false), '-3');
    expect(get<LineEntity>(h, e)).toMatchObject({ za: -3, zb: 20 });
  });

  it('gives an end its first elevation while the other has none', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0) });
    commit(lineEndRow(h.ctx, get(h, e), 0, false), '7');
    const now = get<LineEntity>(h, e);
    expect(now.za).toBe(7);
    expect('zb' in now).toBe(false);
  });

  it('an empty value clears that end: none, not 0', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 10, zb: 20 });
    commit(lineEndRow(h.ctx, get(h, e), 0, false), '');
    const now = get<LineEntity>(h, e);
    expect('za' in now).toBe(false);
    expect(now.zb).toBe(20);
  });

  it('what is not a number changes nothing and writes nothing', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 10, zb: 20 });
    commit(lineEndRow(h.ctx, get(h, e), 0, false), 'yüksek');
    expect(get<LineEntity>(h, e)).toMatchObject({ za: 10, zb: 20 });
    expect(h.doc.undo()).toBe('Ekle');
  });

  it('is read-only on a locked layer', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', layerId: 'kilitli', a: pt(0, 0), b: pt(30, 0), za: 10 });
    expect(lineEndRow(h.ctx, get(h, e), 0, true).editor).toBeUndefined();
    expect(lineEndRow(h.ctx, get(h, e), 0, true).value).toBe('10.000');
  });
});

describe('a polyline’s or an area’s Kot', () => {
  it('shows the range with a remark when some vertices have none, and the value or kot yok otherwise', () => {
    const h = toolHarness();
    const partial = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0), pt(9, 0), pt(9, 4)], zs: [98.5, null, 105.25, 101] });
    expect(pathElevationRow(h.ctx, get(h, partial), false)).toMatchObject({ label: 'Kot', value: '98.500–105.250 m', note: '(bazı köşeler kotsuz)' });
    const level = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0)], zs: [40, 40] });
    expect(pathElevationRow(h.ctx, get(h, level), false)).toMatchObject({ value: '40.000', unit: 'm' });
    const bare = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0)] });
    expect(pathElevationRow(h.ctx, get(h, bare), false)).toMatchObject({ value: 'kot yok' });
  });

  it('a number sets every vertex, the holes’ too, as one step named Kot ver', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polygon', pts: SQUARE, zs: [1, null, 3, 4], holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4)], zs: [5, 6, 7] }] });
    commit(pathElevationRow(h.ctx, get(h, e), false), '100');
    const now = get<PolylineEntity>(h, e);
    expect(now.zs).toEqual([100, 100, 100, 100]);
    expect(now.holes?.[0].zs).toEqual([100, 100, 100]);
    expect(h.doc.undo()).toBe('Kot ver');
    expect(get<PolylineEntity>(h, e).zs).toEqual([1, null, 3, 4]);
    expect(get<PolylineEntity>(h, e).holes?.[0].zs).toEqual([5, 6, 7]);
  });

  it('the range includes the holes', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polygon', pts: SQUARE, zs: [10, 10, 10, 10], holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4)], zs: [12, 12, 12] }] });
    expect(pathElevationRow(h.ctx, get(h, e), false)).toMatchObject({ value: '10.000–12.000 m' });
  });

  it('an empty value clears every vertex', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0)], zs: [1, 2] });
    commit(pathElevationRow(h.ctx, get(h, e), false), '');
    expect('zs' in get<PolylineEntity>(h, e)).toBe(false);
    expect(h.doc.undo()).toBe('Kot ver');
  });

  it('is read-only on a locked layer, and the command refuses a write that reaches it all the same', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', layerId: 'kilitli', pts: [pt(0, 0), pt(5, 0)], zs: [1, 2] });
    expect(pathElevationRow(h.ctx, get(h, e), true).editor).toBeUndefined();
    // The layer locked while the panel showed the row: the command refuses the write whole, and says so.
    commit(pathElevationRow(h.ctx, get(h, e), false), '5');
    expect(get<PolylineEntity>(h, e).zs).toEqual([1, 2]);
    expect(levels(h).at(-1)).toContain('katmanı kilitli');
    expect(h.doc.undo()).toBe('Ekle');
  });
});

describe('the length in space, beside the plan one', () => {
  it('30 m in plan rising 40 m: 3B uzunluk of a line is 50 m', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 0, zb: 40 });
    expect(spaceRow(h.ctx, get(h, e))).toEqual([{ label: '3B uzunluk', value: '50.000', numeric: true, unit: 'm' }]);
  });

  it('3B uzunluk of a polyline and 3B çevre of an area, ring and holes', () => {
    const h = toolHarness();
    const path = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(30, 0), pt(40, 0)], zs: [0, 40, 40] });
    expect(spaceRow(h.ctx, get(h, path))).toMatchObject([{ label: '3B uzunluk', value: '60.000' }]);
    const area = h.add({ kind: 'polygon', pts: SQUARE, zs: [0, 10, 10, 0], holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4), pt(2, 4)], zs: [5, 5, 5, 5] }] });
    // 20 + 20√2 for the ring and 8 for the hole.
    expect(spaceRow(h.ctx, get(h, area))).toMatchObject([{ label: '3B çevre', value: (28 + 20 * Math.SQRT2).toFixed(3) }]);
  });

  it('none unless every vertex has an elevation, and none for a kind without a length in space', () => {
    const h = toolHarness();
    expect(spaceRow(h.ctx, get(h, h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 0), za: 1 })))).toEqual([]);
    expect(spaceRow(h.ctx, get(h, h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0), pt(9, 0)], zs: [1, null, 3] })))).toEqual([]);
    expect(spaceRow(h.ctx, get(h, h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0)] })))).toEqual([]);
    expect(spaceRow(h.ctx, get(h, h.add({ kind: 'circle', c: pt(0, 0), r: 3 })))).toEqual([]);
    expect(spaceRow(h.ctx, get(h, h.add({ kind: 'point', p: pt(0, 0), z: 3 })))).toEqual([]);
  });

  it('follows the formatter’s decimals', () => {
    const h = toolHarness();
    (h.ctx.format as unknown as { prefs: { lengthDecimals: { set(n: number): void } } }).prefs.lengthDecimals.set(2);
    const e = h.add({ kind: 'line', a: pt(0, 0), b: pt(1, 0), za: 0, zb: 1 });
    expect(spaceRow(h.ctx, get(h, e))[0].value).toBe('1.41');
  });
});

describe('the common Kot of several objects', () => {
  it('the value when every object that takes one has the same everywhere, kot yok when none has any', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 50, zb: 50 });
    const b = h.add({ kind: 'polyline', pts: [pt(0, 1), pt(5, 1)], zs: [50, 50] });
    const spot = h.add({ kind: 'point', p: pt(3, 3), z: 50 });
    expect(commonElevationRow(h.ctx, [get(h, a), get(h, b), get(h, spot)], false)).toMatchObject([{ label: 'Kot', value: '50.000', unit: 'm' }]);
    const bare = h.add({ kind: 'polyline', pts: [pt(0, 2), pt(5, 2)] });
    const spotless = h.add({ kind: 'point', p: pt(4, 4) });
    expect(commonElevationRow(h.ctx, [get(h, bare), get(h, spotless)], false)).toMatchObject([{ label: 'Kot', value: 'kot yok' }]);
  });

  it('Çeşitli when the objects differ, or some vertex has none', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 50, zb: 50 });
    const b = h.add({ kind: 'line', a: pt(0, 1), b: pt(5, 1), za: 50, zb: 60 });
    const c = h.add({ kind: 'line', a: pt(0, 2), b: pt(5, 2), za: 50 });
    const differ = commonElevationRow(h.ctx, [get(h, a), get(h, b)], false)[0];
    expect(differ).toMatchObject({ label: 'Kot', value: 'Çeşitli' });
    expect(differ.unit).toBeUndefined();
    expect(commonElevationRow(h.ctx, [get(h, a), get(h, c)], false)[0].value).toBe('Çeşitli');
  });

  it('typing sets every vertex of every object that takes one, leaving the others; one step named Kot ver', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 2 });
    const b = h.add({ kind: 'polygon', pts: SQUARE, holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4)] }] });
    const spot = h.add({ kind: 'point', p: pt(3, 3) });
    const circle = h.add({ kind: 'circle', c: pt(9, 9), r: 1 });
    const row = commonElevationRow(h.ctx, [a, b, spot, circle].map((e) => get(h, e)), false)[0];
    commit(row, '75');
    expect(get<LineEntity>(h, a)).toMatchObject({ za: 75, zb: 75 });
    expect(get<PolylineEntity>(h, b).zs).toEqual([75, 75, 75, 75]);
    expect(get<PolylineEntity>(h, b).holes?.[0].zs).toEqual([75, 75, 75]);
    expect(get<PointEntity>(h, spot).z).toBe(75);
    expect(h.doc.get(circle.id)).toEqual(circle);
    expect(h.doc.undo()).toBe('Kot ver');
    expect(get<LineEntity>(h, a)).toMatchObject({ za: 1, zb: 2 });
    expect('z' in get<PointEntity>(h, spot)).toBe(false);
  });

  it('an empty value clears them all', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 2 });
    const spot = h.add({ kind: 'point', p: pt(3, 3), z: 4 });
    commit(commonElevationRow(h.ctx, [get(h, a), get(h, spot)], false)[0], '');
    expect('za' in get<LineEntity>(h, a)).toBe(false);
    expect('z' in get<PointEntity>(h, spot)).toBe(false);
  });

  it('has no row when nothing selected takes an elevation, and is read-only with a locked object among them', () => {
    const h = toolHarness();
    const circle = h.add({ kind: 'circle', c: pt(0, 0), r: 1 });
    expect(commonElevationRow(h.ctx, [get(h, circle)], false)).toEqual([]);
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(5, 0), za: 1, zb: 1 });
    expect(commonElevationRow(h.ctx, [get(h, a), get(h, circle)], true)[0].editor).toBeUndefined();
    expect(commonElevationRow(h.ctx, [get(h, a), get(h, circle)], false)[0].editor).toBeDefined();
    expect(levels(h)).toEqual([]);
  });
});
