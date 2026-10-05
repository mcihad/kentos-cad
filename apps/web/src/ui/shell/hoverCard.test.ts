import { describe, expect, it } from 'vitest';
import { Formatter } from '../../app/format';
import { Signal } from '../../core/signal';
import type { Entity } from '../../model/entities';
import { cardRows, deedAreaText } from './HoverCard';

/** The hover card's registered area: the deed's value as written, never parsed or rounded. */
describe('the hover card’s Tapu alanı', () => {
  it('shows the value as written, with m² after a plain decimal number, a comma or a point', () => {
    expect(deedAreaText({ 'Tapu alanı (m²)': '723,52' })).toBe('723,52 m²');
    expect(deedAreaText({ 'Tapu alanı (m²)': ' 723.525 ' })).toBe('723.525 m²');
    expect(deedAreaText({ 'Tapu alanı (m²)': '1200' })).toBe('1200 m²');
  });

  it('shows other text as it is, and nothing when the attribute is empty or absent', () => {
    expect(deedAreaText({ 'Tapu alanı (m²)': 'tapuda yok' })).toBe('tapuda yok');
    expect(deedAreaText({ 'Tapu alanı (m²)': '723abc' })).toBe('723abc');
    expect(deedAreaText({ 'Tapu alanı (m²)': '  ' })).toBeNull();
    expect(deedAreaText({})).toBeNull();
  });
});

/** The card's rows: the length in space beside the plan one, when every vertex has an elevation (docs/adr/0142). */
describe('the hover card’s 3B uzunluk and 3B çevre', () => {
  const format = new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) });
  const p = (x: number, y: number) => ({ x, y });
  const base = { id: 1, layerId: 'cizim', attrs: {} };
  const rows = (e: object) => cardRows({ ...base, ...e } as Entity, format);

  it('a line shows Uzunluk and, right after it, 3B uzunluk: 30 m in plan rising 40 m is 50 m', () => {
    expect(rows({ kind: 'line', a: p(0, 0), b: p(30, 0), za: 0, zb: 40 })).toEqual([
      ['Uzunluk', '30.000 m'],
      ['3B uzunluk', '50.000 m'],
    ]);
  });

  it('a polyline shows 3B uzunluk; an area shows Çevre and 3B çevre, its holes counted in both', () => {
    expect(rows({ kind: 'polyline', pts: [p(0, 0), p(30, 0), p(40, 0)], zs: [0, 40, 40] })).toEqual([
      ['Uzunluk', '40.000 m'],
      ['3B uzunluk', '60.000 m'],
    ]);
    const area = rows({ kind: 'polygon', pts: [p(0, 0), p(10, 0), p(10, 10), p(0, 10)], zs: [0, 10, 10, 0], holes: [{ pts: [p(2, 2), p(4, 2), p(4, 4), p(2, 4)], zs: [5, 5, 5, 5] }] });
    expect(area.map(([k]) => k)).toEqual(['Alan', 'Ada (delik)', 'Çevre', '3B çevre']);
    expect(area.find(([k]) => k === 'Çevre')?.[1]).toBe('48.000 m');
    expect(area.find(([k]) => k === '3B çevre')?.[1]).toBe(`${(28 + 20 * Math.SQRT2).toFixed(3)} m`);
  });

  it('none unless every vertex has an elevation: the card is as it was', () => {
    expect(rows({ kind: 'line', a: p(0, 0), b: p(30, 0) })).toEqual([['Uzunluk', '30.000 m']]);
    expect(rows({ kind: 'line', a: p(0, 0), b: p(30, 0), za: 4 })).toEqual([['Uzunluk', '30.000 m']]);
    expect(rows({ kind: 'polyline', pts: [p(0, 0), p(30, 0), p(40, 0)], zs: [0, null, 40] }).map(([k]) => k)).toEqual(['Uzunluk']);
  });

  it('a circle’s Çevre and a point’s Kot are as they were', () => {
    expect(rows({ kind: 'circle', c: p(0, 0), r: 1 }).map(([k]) => k)).toEqual(['Alan', 'Çevre', 'Yarıçap']);
    expect(rows({ kind: 'point', p: p(0, 0), z: 118.5 })).toEqual([['Kot', '118.500 m']]);
  });

  it('follows the formatter’s decimals', () => {
    const coarse = new Formatter({ lengthDecimals: new Signal(1), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) });
    expect(cardRows({ ...base, kind: 'line', a: p(0, 0), b: p(1, 0), za: 0, zb: 1 } as Entity, coarse)).toEqual([
      ['Uzunluk', '1.0 m'],
      ['3B uzunluk', '1.4 m'],
    ]);
  });
});

/** A multi-part area's rows (docs/adr/0143): its parts, and the islands of every part, as the desktop's card orders them. */
describe('the hover card’s Parça and Ada (delik) of a multi-part area', () => {
  const format = new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) });
  const p = (x: number, y: number) => ({ x, y });
  const square = (x: number, y: number, size: number) => ({ pts: [p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)] });
  const parcel = { id: 4, layerId: 'parsel', attrs: { Ada: '101' }, kind: 'polygon', ...square(0, 0, 10), holes: [square(2, 2, 2)] };
  const rows = (e: object) => cardRows({ ...parcel, ...e } as Entity, format);
  const names = (list: [string, string][]) => list.map(([k]) => k);

  it('a one-part area has no Parça row: Ada, Alan, Ada (delik), Çevre', () => {
    expect(names(rows({}))).toEqual(['Ada', 'Alan', 'Ada (delik)', 'Çevre']);
    expect(names(rows({ parts: [] }))).toEqual(['Ada', 'Alan', 'Ada (delik)', 'Çevre']);
  });

  it('a multi-part area shows Parça after the area and before Ada (delik), and counts every part’s holes', () => {
    const two = rows({ parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2), square(26, 6, 2)] }] });
    expect(names(two)).toEqual(['Ada', 'Alan', 'Parça', 'Ada (delik)', 'Çevre']);
    expect(two.find(([k]) => k === 'Parça')?.[1]).toBe('2');
    // The area's own hole and the second part's two.
    expect(two.find(([k]) => k === 'Ada (delik)')?.[1]).toBe('3');
    // Area and perimeter are the core's, part-aware: both parts, the islands taken out.
    expect(two.find(([k]) => k === 'Alan')?.[1]).toBe('188.00 m²');
    expect(two.find(([k]) => k === 'Çevre')?.[1]).toBe('104.000 m');
  });

  it('an area whose holes are all in a later part still shows Ada (delik); no holes anywhere shows none', () => {
    const later = rows({ holes: undefined, parts: [{ ...square(20, 0, 10), holes: [square(22, 2, 2)] }, square(40, 0, 10)] });
    expect(names(later)).toEqual(['Ada', 'Alan', 'Parça', 'Ada (delik)', 'Çevre']);
    expect(later.find(([k]) => k === 'Parça')?.[1]).toBe('3');
    expect(later.find(([k]) => k === 'Ada (delik)')?.[1]).toBe('1');
    expect(names(rows({ holes: undefined, parts: [square(20, 0, 10)] }))).toEqual(['Ada', 'Alan', 'Parça', 'Çevre']);
  });

  it('with a registered area the order is Tapu alanı, Hesaplanan alan, Parça', () => {
    const deed = rows({ attrs: { Ada: '101', 'Tapu alanı (m²)': '188' }, parts: [square(20, 0, 10)] });
    expect(names(deed)).toEqual(['Ada', 'Tapu alanı', 'Hesaplanan alan', 'Parça', 'Ada (delik)', 'Çevre']);
  });
});

/** The card of a multi-part polyline and of a multi-point object (docs/adr/0174 §6): as the desktop's. */
describe('the hover card’s Parça and Nokta', () => {
  const format = new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) });
  const p = (x: number, y: number) => ({ x, y });
  const base = { id: 1, layerId: 'cizim', attrs: {} };
  const rows = (e: object) => cardRows({ ...base, ...e } as Entity, format);

  it('a multi-part polyline says its parts before its length, every part’s', () => {
    expect(rows({ kind: 'polyline', pts: [p(0, 0), p(10, 0)], parts: [{ pts: [p(20, 0), p(25, 0)] }] })).toEqual([
      ['Parça', '2'],
      ['Uzunluk', '15.000 m'],
    ]);
  });

  it('a multi-point object says its points, and their elevation only when they share one', () => {
    const marks = (z?: number) => ({ kind: 'point', p: p(0, 0), z: 100, parts: [{ p: p(5, 0), ...(z !== undefined && { z }) }] });
    expect(rows(marks(100))).toEqual([
      ['Nokta', '2'],
      ['Kot', '100.000 m'],
    ]);
    expect(rows(marks())).toEqual([['Nokta', '2']]);
  });
});
