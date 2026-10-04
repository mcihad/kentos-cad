import { describe, expect, it } from 'vitest';
import { crsBySrid } from '../geo/crs';
import { crsTransform, formatDd, formatDms, parseAngle, systemOf, type System } from '../model/geom/crsTransform';

/**
 * The coordinate transforms against PROJ (fixtures/geodesy/v1/transform.json, written by
 * scripts/fixtures/crs_transform_cases.py from pyproj and the EPSG operations by name, not KentOS code; docs/adr/0167),
 * through the WASM core the app calls: grid points within 1e-6 m, latitudes and longitudes within 1e-11°, accuracy and
 * operations exactly; degrees, minutes and seconds written and read as the reference gives them. The core runs the same
 * file natively (crates/shared/geometry-core/tests/crs.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  transform: { name: string; from: System; to: System; fromSrid: number; toSrid: number; p: [number, number]; expect: { point: [number, number]; accuracy: number; via: string } }[];
  formatting: { deg: number; latitude: boolean; decimals: number; dms?: string; dd?: string }[];
  parse: { text: string; expect: number | null }[];
}

describe('crsTransform', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/transform.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.crs-transform'));

  it('moves points between the systems as PROJ moves them', () => {
    const off = file.transform.flatMap((c) => {
      const got = crsTransform(c.from, c.to, { x: c.p[0], y: c.p[1] });
      if (!got) return [`${c.name}: none`];
      const tol = c.to.kind === 'geographic' ? 1e-11 : 1e-6;
      const far = Math.max(Math.abs(got.point.x - c.expect.point[0]), Math.abs(got.point.y - c.expect.point[1]));
      const wrong = far > tol ? [`${c.name}: ${far}`] : [];
      if (got.accuracy !== c.expect.accuracy || got.via !== c.expect.via) wrong.push(`${c.name}: ${got.accuracy} ${got.via}`);
      return wrong;
    });
    expect(off).toEqual([]);
    expect(file.transform.length).toBeGreaterThanOrEqual(30);
  });

  it('builds the reference’s systems from the registry', () => {
    for (const c of file.transform) {
      expect(systemOf(crsBySrid(c.fromSrid)!), `${c.fromSrid}`).toEqual(c.from);
      expect(systemOf(crsBySrid(c.toSrid)!), `${c.toSrid}`).toEqual(c.to);
    }
    expect(systemOf(crsBySrid(0)!)).toBeNull();
  });

  it('writes and reads angles as the rules say', () => {
    for (const c of file.formatting) {
      if (c.dms) expect(formatDms(c.deg, c.latitude, c.decimals), `${c.deg}`).toBe(c.dms);
      if (c.dd) expect(formatDd(c.deg, c.latitude, c.decimals), `${c.deg}`).toBe(c.dd);
    }
    for (const c of file.parse) {
      const got = parseAngle(c.text);
      if (c.expect === null) expect(got, JSON.stringify(c.text)).toBeNull();
      else expect(Math.abs((got ?? NaN) - c.expect), JSON.stringify(c.text)).toBeLessThanOrEqual(1e-12);
    }
  });
});
