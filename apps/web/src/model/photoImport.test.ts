import { describe, expect, it } from 'vitest';
import type { PhotoExif } from '../contracts/generated/PhotoExif';
import { crsBySrid } from '../geo/crs';
import { systemOf } from './geom/crsTransform';
import { photoEntities, placePhotos, type Photo } from './photoImport';

/**
 * Konumlu fotoğraflar' points (docs/adr/0225 §3) against fixtures/photos/v1/import.json (scripts/fixtures/photo_import_cases.py,
 * from the ADR's rules and PROJ): names, attributes, attachments and the photos left out exactly, the positions within
 * the fixture's tolerance. The desktop checks the same file (crates/native/interaction/tests/all/photos.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  tolerance: { metres: number };
  cases: {
    name: string;
    srid: number;
    photos: (Omit<Photo, 'exif'> & { exif: PhotoExif })[];
    expect: { placed: { index: number; name: string; p: [number, number]; attrs: Record<string, string>; attachment: unknown }[]; skipped: { line: number; message: string }[] };
  }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/photos/v1/import.json', import.meta.url), 'utf8')) as File;

describe("Konumlu fotoğraflar' points (docs/adr/0225 §3)", () => {
  it('are the shared cases', () => {
    expect(file.format).toBe('kentos.photo-import');
    for (const c of file.cases) {
      const crs = crsBySrid(c.srid)!;
      const plan = placePhotos(c.photos, systemOf(crs)!, crs.name);
      expect(plan.skipped, c.name).toEqual(c.expect.skipped);
      expect(plan.placed.length, c.name).toBe(c.expect.placed.length);
      plan.placed.forEach((got, i) => {
        const want = c.expect.placed[i]!;
        expect([got.index, got.name], c.name).toEqual([want.index, want.name]);
        expect(Math.abs(got.p.x - want.p[0]), `${c.name}: ${got.name} x`).toBeLessThanOrEqual(file.tolerance.metres);
        expect(Math.abs(got.p.y - want.p[1]), `${c.name}: ${got.name} y`).toBeLessThanOrEqual(file.tolerance.metres);
        expect(got.attrs, `${c.name}: ${got.name}`).toEqual(want.attrs);
        expect(got.attachment, `${c.name}: ${got.name}`).toEqual(want.attachment);
      });
      const made = photoEntities(plan.placed);
      expect(made.map((e) => [e.label, e.layerId, e.attachments])).toEqual(plan.placed.map((p) => [p.name, '', [p.attachment]]));
    }
  });
});
