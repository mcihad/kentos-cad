import { describe, expect, it } from 'vitest';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import { crsTransformIn, type System } from '../model/geom/crsTransform';
import { forgetGrid } from '../model/geom/crsGrid';
import { datumChoices } from '../model/projectCrs';
import { accuracyText } from '../model/secondCrs';
import { GridLibrary, gridLine, memoryGridStore } from './gridLibrary';

/**
 * The device's NTv2 grid library (docs/adr/0168 §4): a grid a project's datum choice names that this device does not
 * have is said with its file's name and gives no value; added, it is listed with what its header says and the values
 * come; removed, the project is told again. The desktop's `grids::tests` do the same with the same grid
 * (fixtures/geodesy/v1/ntv2/tr.gsb, written by scripts/fixtures/ntv2_cases.py).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array } } }).process.getBuiltinModule('node:fs');
const TR = new Uint8Array(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/ntv2/tr.gsb', import.meta.url)));

const hex = async (bytes: Uint8Array) =>
  [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>))].map((b) => b.toString(16).padStart(2, '0')).join('');

const TM36: System = { kind: 'tm', datum: 'TUREF', centralMeridian: 36, scaleFactor: 1, falseEasting: 500000, falseNorthing: 0 };
const WGS84: System = { kind: 'geographic', datum: 'WGS84' };

describe('the grid library (docs/adr/0168 §4)', () => {
  it('says a missing grid, adds, lists and removes one', async () => {
    const id = await hex(TR);
    forgetGrid(id);
    const choice: DatumTransform = { from: 'TUREF', to: 'WGS84', name: 'TUREF → WGS 84: ızgara', grid: { id, file: 'tr.gsb', size: TR.length, accuracy: 0.05 } };
    // As the transforms take the project's choice (model/projectCrs.ts).
    const choices = datumChoices({ srid: 5256, datumTransforms: [choice] });
    const library = new GridLibrary(memoryGridStore());
    const said: string[] = [];
    const say = (t: string) => said.push(t);
    const at = { x: 486512.34, y: 4420187.52 };
    await library.follow([choice], say);
    expect(said).toHaveLength(1);
    expect(said[0]).toMatch(/^NTv2 ızgarası bu cihazda yok: tr\.gsb\. Proje ayarları › Koordinat sistemi › Izgaralar'dan ekleyin/);
    expect(crsTransformIn(TM36, WGS84, at, choices)).toEqual({ error: 'noGrid' });
    // Asked once: the same choices again say nothing.
    await library.follow([choice], say);
    expect(said).toHaveLength(1);

    const revision = library.revision.value;
    const entry = await library.add('tr.gsb', TR);
    if ('error' in entry) throw new Error(entry.error);
    expect(entry).toMatchObject({ id, file: 'tr.gsb', size: TR.length, subgrids: 1 });
    expect(library.revision.value).toBeGreaterThan(revision);
    expect(library.entries.value).toEqual([entry]);
    expect(gridLine(entry)).toMatch(new RegExp(` KB · ${id.slice(0, 12)}$`));
    const moved = crsTransformIn(TM36, WGS84, at, choices);
    if ('error' in moved) throw new Error(moved.error);
    expect(accuracyText(moved)).toBe('±0.05 m, TUREF → WGS 84: ızgara');

    await library.remove(id);
    expect(library.entries.value).toEqual([]);
    expect(crsTransformIn(TM36, WGS84, at, choices)).toEqual({ error: 'noGrid' });
    await library.follow([choice], say);
    expect(said).toHaveLength(2);

    const refused = await library.add('nokta.gsb', new TextEncoder().encode('nokta listesi'));
    expect(refused).toEqual({ error: '“nokta.gsb” NTv2 ızgarası olarak okunamadı: NTv2 dosyası değil.' });
  });
});
