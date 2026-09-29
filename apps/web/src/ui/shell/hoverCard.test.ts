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
