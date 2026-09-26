import { describe, expect, it } from 'vitest';
import { shapefileSet } from './shapefile';

/** The files a user chose together, sorted into one Shapefile layer (docs/adr/0046). */
const file = (name: string, byte = 1) => ({ name, bytes: new Uint8Array([byte]) });

describe('a Shapefile layer among the chosen files', () => {
  it('takes the .shp and the files of its name, whatever their case; says what it did not use', () => {
    const set = shapefileSet([file('Parsel.dbf', 2), file('PARSEL.SHP', 1), file('parsel.shx', 3), file('parsel.prj', 4), file('yol.dbf', 5), file('parsel.qmd', 6)]);
    if ('error' in set) throw new Error(set.error);
    expect(set.name).toBe('PARSEL');
    expect(set.parts).toEqual(['.shp', '.dbf', '.shx', '.prj']);
    expect([set.files.shp[0], set.files.dbf?.[0], set.files.shx?.[0], set.files.prj?.[0], set.files.cpg]).toEqual([1, 2, 3, 4, undefined]);
    expect(set.unused).toEqual(['yol.dbf', 'parsel.qmd']);
  });

  it('asks for a .shp when there is none, and for one layer at a time when there are several', () => {
    const none = shapefileSet([file('a.dbf'), file('a.prj')]);
    expect('error' in none && none.error).toMatch(/\.shp yok/);
    const two = shapefileSet([file('a.shp'), file('b.shp'), file('a.dbf')]);
    expect('error' in two && two.error).toMatch(/2 \.shp var \(a\.shp, b\.shp\).*bir katman/);
  });

  it('keeps the first of a repeated part and does not use the second', () => {
    const set = shapefileSet([file('a.shp'), file('a.dbf', 7), file('A.DBF', 8)]);
    if ('error' in set) throw new Error(set.error);
    expect([set.files.dbf?.[0], set.unused]).toEqual([7, ['A.DBF']]);
  });
});
