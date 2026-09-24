import { turefZoneFor, type CrsDef } from './crs';

/**
 * The CRS registry as a versioned file shared with Rust
 * (fixtures/crs/v1/registry.json): every system, the default, and the TUREF
 * zone suggested for sample longitudes (the nearest central meridian; on a
 * zone boundary the western zone). Built by the recorder
 * (scripts/fixtures/record-crs.test.ts) and compared by crs.test.ts.
 */

/** Longitudes across Türkiye, on the 28.5° / 43.5° boundaries and outside the country. */
export const ZONE_SAMPLES = [20, 25.9, 27, 28.4999, 28.5, 28.5001, 32.85, 36, 38.7, 41.2, 43.5, 44.8, 50];

export function crsFixture(registry: readonly CrsDef[], defaultSrid: number) {
  return {
    format: 'kentos.crs-registry',
    version: 1,
    source: 'src/geo/crs.ts',
    defaultSrid,
    systems: registry,
    zoneSuggestions: ZONE_SAMPLES.map((lon) => ({ lon, srid: turefZoneFor(lon)?.srid ?? null })),
  };
}
