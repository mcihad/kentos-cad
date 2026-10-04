import { describe, expect, it } from 'vitest';
import { loadGrid } from '../model/geom/crsGrid';
import { crsTransformIn, type DatumChoice, type System } from '../model/geom/crsTransform';

/**
 * NTv2 grid shifts (docs/adr/0168 §3–§4) against PROJ (fixtures/geodesy/v1/ntv2.json, written by
 * scripts/fixtures/ntv2_cases.py: grids written without KentOS code, PROJ's `hgridshift` on them), through the WASM core
 * the app calls: what each grid says, the refused files' reasons, points shifted between ED50 and TUREF latitude and
 * longitude by the project's grid choice (within 1e-11 degrees), the transforms between transverse Mercator grids
 * (within 1e-6 m). The core runs the same file natively (crates/shared/geometry-core/tests/all/crs_ntv2.rs).
 */
const fs = (
  globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc?: 'utf8'): string & Uint8Array } } }
).process.getBuiltinModule('node:fs');

const dir = new URL('../../../../fixtures/geodesy/v1/', import.meta.url);
const bytes = (file: string) => new Uint8Array(fs.readFileSync(new URL(`ntv2/${file}`, dir)));

type Expect = { point: [number, number]; accuracy: number | null; via: string; unofficial: boolean } | { error: string };

interface File {
  format: string;
  grids: { file: string; id: string; from: string; to: string; fromAxes: [number, number]; toAxes: [number, number]; subgrids: number; extent: number[] }[];
  broken: { file: string; error: string }[];
  shifts: { name: string; file: string; reverse: boolean; points: [number, number][]; expect: ([number, number] | null)[] }[];
  transforms: { name: string; from: System; to: System; choices: DatumChoice[]; points: [number, number][]; expect: Expect[] }[];
}

describe('NTv2 grids', () => {
  const file = JSON.parse(fs.readFileSync(new URL('ntv2.json', dir), 'utf8')) as File;
  const ids = new Map(file.grids.map((g) => [g.file, g.id]));

  it('reads the grids and says what they are', () => {
    expect(file.format).toBe('kentos.ntv2');
    for (const g of file.grids) {
      const info = loadGrid(g.id, bytes(g.file));
      if ('error' in info) throw new Error(`${g.file}: ${info.error}`);
      expect([info.from, info.to, info.fromAxes, info.toAxes, info.subgrids], g.file).toEqual([g.from, g.to, g.fromAxes, g.toAxes, g.subgrids]);
      info.extent.forEach((v, i) => expect(Math.abs(v - g.extent[i]!), g.file).toBeLessThan(1e-9));
    }
  });

  it('refuses broken files and says why', () => {
    for (const b of file.broken) expect(loadGrid(`bozuk-${b.file}`, bytes(b.file)), b.file).toEqual({ error: b.error });
  });

  it('shifts latitudes and longitudes as PROJ shifts them', () => {
    const ed50: System = { kind: 'geographic', datum: 'ED50' };
    const turef: System = { kind: 'geographic', datum: 'TUREF' };
    for (const c of file.shifts) {
      const choice: DatumChoice = { from: 'ED50', to: 'TUREF', name: 'ızgara', grid: { id: ids.get(c.file)! } };
      c.points.forEach(([x, y], i) => {
        const want = c.expect[i];
        const got = c.reverse ? crsTransformIn(turef, ed50, { x, y }, [choice]) : crsTransformIn(ed50, turef, { x, y }, [choice]);
        if (!want) return expect(got, `${c.name} ${x} ${y}`).toEqual({ error: 'outsideGrid' });
        if ('error' in got) throw new Error(`${c.name} ${x} ${y}: ${got.error}`);
        expect(Math.abs(got.point.x - want[0]), `${c.name} ${x} ${y}`).toBeLessThanOrEqual(1e-11);
        expect(Math.abs(got.point.y - want[1]), `${c.name} ${x} ${y}`).toBeLessThanOrEqual(1e-11);
      });
    }
  });

  it("moves points by the project's grid choice as PROJ does", () => {
    for (const c of file.transforms) {
      c.points.forEach(([x, y], i) => {
        const want = c.expect[i]!;
        const got = crsTransformIn(c.from, c.to, { x, y }, c.choices);
        if ('error' in want) return expect(got, c.name).toEqual({ error: want.error });
        if ('error' in got) throw new Error(`${c.name}: ${got.error}`);
        expect(Math.abs(got.point.x - want.point[0]), c.name).toBeLessThanOrEqual(1e-6);
        expect(Math.abs(got.point.y - want.point[1]), c.name).toBeLessThanOrEqual(1e-6);
        expect([got.accuracy ?? null, got.via, got.unofficial], c.name).toEqual([want.accuracy, want.via, want.unofficial]);
      });
    }
  });
});
