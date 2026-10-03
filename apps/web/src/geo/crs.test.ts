import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/crs/v1/registry.json?raw';
import { CRS_REGISTRY, crsBySrid, crsCode, crsTitle, DEFAULT_SRID, isLocal, LOCAL_SRID, searchCrs, turefZoneFor, workAreaCentre } from './crs';
import { crsFixture } from './crsFixture';

const file = JSON.parse(text) as ReturnType<typeof crsFixture>;

describe('CRS registry', () => {
  it('is the file shared with Rust (re-record after a deliberate change)', () => {
    expect(file).toEqual(JSON.parse(JSON.stringify(crsFixture(CRS_REGISTRY, DEFAULT_SRID))));
  });

  it('has one entry per SRID and knows the default', () => {
    expect(new Set(CRS_REGISTRY.map((c) => c.srid)).size).toBe(CRS_REGISTRY.length);
    expect(crsBySrid(DEFAULT_SRID)?.name).toBe('TUREF / TM36');
    expect(crsBySrid(1234)).toBeUndefined();
  });

  it('suggests the nearest TUREF zone, the western one on a boundary', () => {
    expect(turefZoneFor(32.85)?.srid).toBe(5255);
    expect(turefZoneFor(28.5)?.srid).toBe(5253);
    expect(turefZoneFor(28.5001)?.srid).toBe(5254);
    expect(turefZoneFor(50)?.srid).toBe(5259);
  });

  it('anchors a new project in the middle of the zone at Türkiye’s centre latitude', () => {
    expect(workAreaCentre(crsBySrid(5256)!)).toEqual({ x: 500_000, y: 4_320_000 });
    expect(workAreaCentre(crsBySrid(32636)!)).toEqual({ x: 500_000, y: 4_320_000 });
    expect(workAreaCentre(crsBySrid(4326)!)).toEqual({ x: 35, y: 39 });
    expect(workAreaCentre(crsBySrid(3857)!)).toEqual({ x: 3_896_000, y: 4_722_000 });
    // Every TM zone of the country (36°–42° K) lies within ~340 km of the anchor.
    for (const north of [3_985_000, 4_650_000]) expect(Math.abs(north - workAreaCentre(crsBySrid(5254)!).y)).toBeLessThan(340_000);
  });

  it('knows the local system, SRID 0: no coordinate system, first in the list, anchored at 0,0 (docs/adr/0165 §2)', () => {
    const local = crsBySrid(LOCAL_SRID)!;
    expect(CRS_REGISTRY[0]).toBe(local);
    expect([local.kind, local.datum, local.unit, local.ellipsoid, local.projection]).toEqual(['local', 'LOCAL', 'metre', undefined, undefined]);
    expect(isLocal(local) && !isLocal(crsBySrid(DEFAULT_SRID)!)).toBe(true);
    expect(workAreaCentre(local)).toEqual({ x: 0, y: 0 });
    // Named as no coordinate system, never as an EPSG code.
    expect([crsTitle(local), crsCode(local)]).toEqual(['Yerel (koordinat sistemi yok)', 'SRID 0']);
    expect([crsTitle(crsBySrid(5256)!), crsCode(crsBySrid(5256)!)]).toEqual(['TUREF / TM36 (EPSG:5256)', 'EPSG:5256']);
    expect(searchCrs('yerel').map((c) => c.srid)).toEqual([LOCAL_SRID]);
  });

  it('is searched by SRID, name or area, with Turkish case folding', () => {
    expect(searchCrs('EPSG:5256').map((c) => c.srid)).toEqual([5256]);
    expect(searchCrs('ed50 / tm').every((c) => c.datum === 'ED50')).toBe(true);
    expect(searchCrs('DÜNYA').map((c) => c.srid)).toEqual([4326]);
    expect(searchCrs('  ')).toHaveLength(CRS_REGISTRY.length);
  });
});
