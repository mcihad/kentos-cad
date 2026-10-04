import { describe, expect, it } from 'vitest';
import { crsBySrid } from '../../geo/crs';
import { convertPoint, convertRows, csvText, errorText, type ConvertFormat } from './convert';

/**
 * Koordinat dönüştür's readings and writings (docs/adr/0167 §4) against the shared cases (fixtures/crs/v1/convert.json,
 * written by scripts/fixtures/crs_convert_cases.py from PROJ and the ADR's rules, not KentOS code): the texts exactly.
 * The desktop's window reads the same file (apps/desktop/src/calc/convert.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  cases: {
    name: string;
    fromSrid: number;
    toSrid: number;
    axes: 'gis' | 'cad';
    decimals: number;
    notation: 'dms' | 'dd';
    input: [string, string];
    expect?: { values: [string, string][]; accuracy: string };
    error?: string;
  }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/crs/v1/convert.json', import.meta.url), 'utf8')) as File;
const formatOf = (c: File['cases'][number]): ConvertFormat => ({ east: c.axes === 'cad' ? 'X' : 'Y', north: c.axes === 'cad' ? 'Y' : 'X', decimals: c.decimals, notation: c.notation });

describe('Koordinat dönüştür (docs/adr/0167 §4)', () => {
  it('reads and writes as the shared cases say', () => {
    expect(file.format).toBe('kentos.crs-convert');
    const off = file.cases.flatMap((c) => {
      const got = convertPoint(crsBySrid(c.fromSrid)!, crsBySrid(c.toSrid)!, c.input[0], c.input[1], formatOf(c));
      if (c.error) return got === c.error ? [] : [`${c.name}: ${JSON.stringify(got)} ≠ ${c.error}`];
      if (typeof got === 'string') return [`${c.name}: ${got}`];
      const same = JSON.stringify(got.values) === JSON.stringify(c.expect!.values) && got.accuracy === c.expect!.accuracy;
      return same ? [] : [`${c.name}: ${JSON.stringify(got.values)} ${got.accuracy} ≠ ${JSON.stringify(c.expect!.values)} ${c.expect!.accuracy}`];
    });
    expect(off).toEqual([]);
    expect(file.cases.length).toBeGreaterThanOrEqual(15);
  });

  it('says what is wrong and how to write it', () => {
    const f: ConvertFormat = { east: 'Y', north: 'X', decimals: 3, notation: 'dms' };
    expect(errorText('a', crsBySrid(5254)!, f)).toBe('Y okunamadı: bir sayı yazın, ör. 414120.512.');
    expect(errorText('b', crsBySrid(4326)!, f)).toBe('Boylam okunamadı: 40 45 12.3456, 40°45′12.3456″K ya da 40.7534293 biçiminde yazın.');
  });

  it('converts a list, numbering what it cannot read, and writes CSV', () => {
    const f: ConvertFormat = { east: 'Y', north: 'X', decimals: 3, notation: 'dms' };
    const rows = convertRows(crsBySrid(5254)!, crsBySrid(2320)!, [{ name: 'R1', a: '414120.512', b: '4540398.207' }, {}, { a: 'yok', b: '1' }], f);
    expect(rows.map((r) => [r.name, r.row, typeof r.result === 'string' ? r.result : r.result.values.map((v) => v[1])])).toEqual([
      ['R1', 1, ['414154.869', '4540584.350']],
      ['3', 3, 'a'],
    ]);
    expect(csvText(['Ad', 'Y', 'X'], [['R1', '414154.869', '4540584.350'], ['a,b', '1', '2']])).toBe('Ad,Y,X\nR1,414154.869,4540584.350\n"a,b",1,2\n');
  });
});
