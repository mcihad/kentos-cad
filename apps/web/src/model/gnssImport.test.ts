import { describe, expect, it } from 'vitest';
import type { GnssLine } from '../contracts/generated/GnssLine';
import type { GnssPoint } from '../contracts/generated/GnssPoint';
import { crsBySrid } from '../geo/crs';
import { systemOf, type System } from './geom/crsTransform';
import { gnssEntities, gnssLineEntities, placeGnss, placeGnssAll, type GnssOptions } from './gnssImport';

/**
 * The GNSS import's points and lines (docs/adr/0169 §6, 0222 §6) against fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py,
 * from the ADR's rules and PROJ, not KentOS code): names, elevations and attributes exactly, the positions within the
 * fixture's tolerance. The desktop checks the same file (crates/native/interaction/tests/all/gnss.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  tolerance: { metres: number };
  cases: {
    name: string;
    srid: number;
    options: GnssOptions;
    points: GnssPoint[];
    lines: GnssLine[];
    expect: { name: string; p: [number, number]; z?: number; attrs: Record<string, string> }[];
    expectLines: {
      placed: { line: number; name?: string; parts: { p: [number, number]; z?: number }[][]; attrs: Record<string, string> }[];
      skipped: { line: number; message: string }[];
    };
  }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/gnss/v1/import.json', import.meta.url), 'utf8')) as File;

describe("the GNSS import's points (docs/adr/0169 §6)", () => {
  it('are the shared cases', () => {
    expect(file.format).toBe('kentos.gnss-import');
    expect(file.cases.length).toBeGreaterThanOrEqual(3);
    for (const c of file.cases) {
      const crs = crsBySrid(c.srid)!;
      const to = systemOf(crs)!;
      const plan = placeGnssAll(c.points, c.lines, c.options, to, crs.name);
      expect(plan.skipped, c.name).toEqual([]);
      expect(plan.placed.length, c.name).toBe(c.expect.length);
      plan.placed.forEach((got, i) => {
        const want = c.expect[i]!;
        expect(got.name, c.name).toBe(want.name);
        expect(Math.abs(got.p.x - want.p[0]), `${c.name}: ${got.name} x`).toBeLessThanOrEqual(file.tolerance.metres);
        expect(Math.abs(got.p.y - want.p[1]), `${c.name}: ${got.name} y`).toBeLessThanOrEqual(file.tolerance.metres);
        expect(got.z, `${c.name}: ${got.name}`).toBe(want.z ?? null);
        expect(got.attrs, `${c.name}: ${got.name}`).toEqual(want.attrs);
      });
      // As the import writes them: points with their names as labels and their elevations, on the source layer the
      // window sends on.
      const made = gnssEntities(plan.placed);
      expect(made).toEqual(
        plan.placed.map((p) => ({ kind: 'point', id: 0, layerId: '', attrs: p.attrs, label: p.name, p: p.p, ...(p.z !== null ? { z: p.z } : {}) })),
      );
      // The routes and tracks (docs/adr/0222 §6).
      expect(plan.linesSkipped, c.name).toEqual(c.expectLines.skipped);
      expect(plan.lines.length, c.name).toBe(c.expectLines.placed.length);
      plan.lines.forEach((got, i) => {
        const want = c.expectLines.placed[i]!;
        expect([got.line, got.name, got.attrs], c.name).toEqual([want.line, want.name ?? null, want.attrs]);
        expect(got.parts.map((r) => r.length), c.name).toEqual(want.parts.map((r) => r.length));
        got.parts.forEach((run, k) =>
          run.forEach((v, j) => {
            const w = want.parts[k]![j]!;
            expect(Math.abs(v.p.x - w.p[0]) + Math.abs(v.p.y - w.p[1]), c.name).toBeLessThanOrEqual(2 * file.tolerance.metres);
            expect(v.z, c.name).toBe(w.z ?? null);
          }),
        );
      });
      const lines = gnssLineEntities(plan.lines);
      expect(lines.map((e) => e.kind)).toEqual(plan.lines.map(() => 'polyline'));
    }
  });

  it('says the points a system does not reach, with why', () => {
    // A project's datum without a way to WGS 84 stands alone (docs/adr/0168 §2): no GNSS point reaches its system.
    const alone: System = {
      kind: 'tm',
      datum: { name: 'Şantiye datumu', ellipsoid: { name: 'GRS 1980', semiMajor: 6378137, inverseFlattening: 298.257222101 } },
      centralMeridian: 30,
      scaleFactor: 1,
      falseEasting: 500000,
      falseNorthing: 0,
    };
    const plan = placeGnss(
      [
        { kind: 'wpt', name: 'N1', lat: 40.75, lon: 29.38, line: 4 },
        { kind: 'gga', lat: 40.76, lon: 29.39, line: 7 },
      ],
      { kinds: ['wpt', 'gga'], prefix: 'G', start: 1 },
      alone,
      'Şantiye',
    );
    const all = placeGnssAll(
      [
        { kind: 'wpt', name: 'N1', lat: 40.75, lon: 29.38, line: 4 },
        { kind: 'gga', lat: 40.76, lon: 29.39, line: 7 },
      ],
      [{ kind: 'nmea', parts: [[0, 1]], line: 4 }],
      { kinds: ['wpt', 'gga'], prefix: 'G', start: 1, lines: true },
      alone,
      'Şantiye',
    );
    expect(all.lines).toEqual([]);
    expect(all.linesSkipped).toEqual([
      { line: 4, message: "Satır 4: NMEA kaydı çizgisi projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi." },
    ]);
    expect(plan.placed).toEqual([]);
    expect(plan.skipped).toEqual([
      { line: 4, message: "Satır 4: N1 noktası projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi." },
      { line: 7, message: "Satır 7: Adsız noktası projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi." },
    ]);
  });
});
