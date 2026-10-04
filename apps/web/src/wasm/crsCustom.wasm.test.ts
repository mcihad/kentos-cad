import { describe, expect, it } from 'vitest';
import { crsTransformIn, type DatumChoice, type System } from '../model/geom/crsTransform';

/**
 * The project's coordinate systems (docs/adr/0168 §1–§3) against PROJ (fixtures/geodesy/v1/custom.json, written by
 * scripts/fixtures/crs_custom_cases.py from pyproj pipelines and the ADR's rules, not KentOS code), through the WASM core
 * the app calls: a transverse Mercator grid of any origin, a datum of the project's in either rotation convention, local
 * systems, the project's datum choices. Grid points within 1e-6 m, latitudes and longitudes within 1e-11 degrees; the
 * accuracy, what the values rest on and whether an EPSG operation of ED50 was used, exactly; a point with no value says
 * why. The core runs the same file natively (crates/shared/geometry-core/tests/crs_custom.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Expect = { point: [number, number]; accuracy: number | null; via: string; unofficial: boolean } | { error: string };

interface File {
  format: string;
  transforms: { name: string; from: System; to: System; choices: DatumChoice[]; points: [number, number][]; expect: Expect[] }[];
}

describe('crsTransformIn', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/custom.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.crs-custom'));

  it("moves points through the project's systems as PROJ moves them", () => {
    const off = file.transforms.flatMap((c) =>
      c.points.flatMap(([x, y], i) => {
        const want = c.expect[i]!;
        const got = crsTransformIn(c.from, c.to, { x, y }, c.choices);
        if ('error' in want) return 'error' in got && got.error === want.error ? [] : [`${c.name}: ${JSON.stringify(got)} ≠ ${want.error}`];
        if ('error' in got) return [`${c.name}: none (${got.error})`];
        const tol = c.to.kind === 'geographic' ? 1e-11 : 1e-6;
        const [wx, wy] = want.point;
        return Math.abs(got.point.x - wx) <= tol &&
          Math.abs(got.point.y - wy) <= tol &&
          (got.accuracy ?? null) === want.accuracy &&
          got.via === want.via &&
          got.unofficial === want.unofficial
          ? []
          : [`${c.name}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`];
      }),
    );
    expect(off).toEqual([]);
    expect(file.transforms.length).toBeGreaterThanOrEqual(20);
  });
});
