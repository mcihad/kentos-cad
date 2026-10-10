import { describe, expect, it } from 'vitest';
import { DIALOG_TEXTS as T } from './dialogTexts';
import { pairValue, rasterPairsView, rasterValuesView, withoutRaster, withPair, withRasterValue } from './fieldPlan';

/** Rasterlere değer and Raster çiftleri's rows (docs/adr/0237 §9), as the desktop's `raster_rows` writes them. */
describe('raster rows', () => {
  it('lists the rasters, then the names the input does not hold', () => {
    const v = { Yol: 0.5, Eski: 2, Eğim: 'x' };
    const r = rasterValuesView(v, ['Eğim', 'Yol', 'Toprak']);
    expect(r.rows.map((x) => [x.name, x.text, x.listed])).toEqual([
      ['Eğim', 'x', true],
      ['Yol', '0.5', true],
      ['Toprak', '', true],
      ['Eski', '2', false],
    ]);
    expect([r.note, r.adds]).toEqual([null, false]);
    expect(rasterValuesView({}, []).note).toBe(T.rasters.none);
    const m = rasterValuesView({}, undefined);
    expect([m.note, m.adds]).toEqual([T.rasters.later, true]);
    expect(withRasterValue({ cell: 'number' }, v, 'Toprak', '1,25').Toprak).toBe(1.25);
    expect('Yol' in withRasterValue({ cell: 'number' }, v, 'Yol', ' ')).toBe(false);
    expect(Number.isNaN(withRasterValue({ cell: 'number' }, v, 'Yol', 'x').Yol as number)).toBe(true);
    expect('Eski' in withoutRaster(v, 'Eski')).toBe(false);
  });

  it('reads a pair both ways and keeps only the chosen comparison', () => {
    const v = [
      ['Yol', 'Eğim', 3],
      ['Eğim', 'Yol', 7],
      ['A', 'B', -2],
    ] as const;
    expect(pairValue(v, 'Eğim', 'Yol')).toBe(-3);
    const r = rasterPairsView(v, ['Eğim', 'Yol', 'Toprak']);
    expect(r.rows.map((x) => [x.a, x.b, x.text, x.listed])).toEqual([
      ['Eğim', 'Yol', 'Yol 3 kat', true],
      ['Eğim', 'Toprak', 'Eşit', true],
      ['Yol', 'Toprak', 'Eşit', true],
      ['A', 'B', 'B 2 kat', false],
    ]);
    const chosen = withPair(v, 'Eğim', 'Yol', 5);
    expect(chosen).toEqual([
      ['A', 'B', -2],
      ['Eğim', 'Yol', 5],
    ]);
    expect(withPair(chosen, 'Yol', 'Eğim', 1)).toEqual([['A', 'B', -2]]);
  });
});
