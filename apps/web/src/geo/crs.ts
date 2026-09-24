/**
 * Coordinate reference systems known to KentOS, keyed by EPSG SRID.
 *
 * This is the single source of truth for CRS metadata. Transformations
 * (datum shifts, reprojection) will live in `geo/transform.ts` and consume
 * these definitions; nothing else should hard-code projection parameters.
 */

export type Datum = 'TUREF' | 'ED50' | 'WGS84';

export interface CrsDef {
  srid: number;
  /** EPSG name, e.g. "TUREF / TM36". */
  name: string;
  kind: 'projected' | 'geographic';
  datum: Datum;
  ellipsoid: 'GRS80' | 'International 1924' | 'WGS84';
  projection?: 'Transverse Mercator' | 'UTM' | 'Pseudo-Mercator';
  /** Degrees east of Greenwich. */
  centralMeridian?: number;
  scaleFactor?: number;
  falseEasting?: number;
  falseNorthing?: number;
  unit: 'metre' | 'degree';
  /** Human description of the zone's longitude band. */
  area: string;
}

export const DATUM_LABEL: Record<Datum, string> = {
  TUREF: 'TUREF (ITRF96)',
  ED50: 'ED50',
  WGS84: 'WGS 84',
};

const tm = (srid: number, datum: 'TUREF' | 'ED50', cm: number): CrsDef => ({
  srid,
  name: `${datum} / TM${cm}`,
  kind: 'projected',
  datum,
  ellipsoid: datum === 'TUREF' ? 'GRS80' : 'International 1924',
  projection: 'Transverse Mercator',
  centralMeridian: cm,
  scaleFactor: 1,
  falseEasting: 500_000,
  falseNorthing: 0,
  unit: 'metre',
  area: `${cm - 1.5}°–${cm + 1.5}° D (3° dilim)`,
});

const utm = (srid: number, datum: 'ED50' | 'WGS84', zone: number): CrsDef => {
  const cm = zone * 6 - 183;
  return {
    srid,
    name: `${datum === 'WGS84' ? 'WGS 84' : 'ED50'} / UTM ${zone}N`,
    kind: 'projected',
    datum,
    ellipsoid: datum === 'WGS84' ? 'WGS84' : 'International 1924',
    projection: 'UTM',
    centralMeridian: cm,
    scaleFactor: 0.9996,
    falseEasting: 500_000,
    falseNorthing: 0,
    unit: 'metre',
    area: `${cm - 3}°–${cm + 3}° D (6° dilim)`,
  };
};

const TM_MERIDIANS = [27, 30, 33, 36, 39, 42, 45];

export const CRS_REGISTRY: readonly CrsDef[] = [
  ...TM_MERIDIANS.map((cm, i) => tm(5253 + i, 'TUREF', cm)),
  { srid: 5252, name: 'TUREF', kind: 'geographic', datum: 'TUREF', ellipsoid: 'GRS80', unit: 'degree', area: 'Türkiye, coğrafi (enlem/boylam)' },
  ...TM_MERIDIANS.map((cm, i) => tm(2319 + i, 'ED50', cm)),
  ...[35, 36, 37, 38].map((z) => utm(23000 + z, 'ED50', z)),
  ...[35, 36, 37, 38].map((z) => utm(32600 + z, 'WGS84', z)),
  { srid: 4326, name: 'WGS 84', kind: 'geographic', datum: 'WGS84', ellipsoid: 'WGS84', unit: 'degree', area: 'Dünya, coğrafi (enlem/boylam)' },
  {
    srid: 3857,
    name: 'WGS 84 / Pseudo-Mercator',
    kind: 'projected',
    datum: 'WGS84',
    ellipsoid: 'WGS84',
    projection: 'Pseudo-Mercator',
    unit: 'metre',
    area: 'Web haritaları (altlık karolar)',
  },
];

const BY_SRID = new Map(CRS_REGISTRY.map((c) => [c.srid, c]));

export const DEFAULT_SRID = 5256;

export function crsBySrid(srid: number): CrsDef | undefined {
  return BY_SRID.get(srid);
}

/** Centre of Türkiye (35° D, 39° K): where a new, empty project is anchored. */
const WORK_LON = 35;
const WORK_LAT = 39;
/** Northing of 39° K on the central meridian (GRS80 meridian arc, rounded; TM k = 1 and UTM k = 0.9996 both land within 3 km). */
const WORK_NORTHING = 4_320_000;

/**
 * Where a new, empty project in `crs` is anchored (its local origin, the
 * GPU's float32 reference) before any object exists: the middle of the
 * zone's band at Türkiye's centre latitude. Every point of the country is
 * then within ~340 km of it. Rounded values; this is an anchor, not a
 * transformation (none is done here).
 */
export function workAreaCentre(crs: CrsDef): { x: number; y: number } {
  if (crs.kind === 'geographic') return { x: WORK_LON, y: WORK_LAT };
  if (crs.projection === 'Pseudo-Mercator') {
    const r = 6_378_137;
    const x = r * ((WORK_LON * Math.PI) / 180);
    const y = r * Math.log(Math.tan(Math.PI / 4 + (WORK_LAT * Math.PI) / 360));
    return { x: Math.round(x / 1000) * 1000, y: Math.round(y / 1000) * 1000 };
  }
  return { x: crs.falseEasting ?? 500_000, y: (crs.falseNorthing ?? 0) + WORK_NORTHING };
}

/** Suggests the TUREF TM zone for a longitude (degrees east). */
export function turefZoneFor(lon: number): CrsDef | undefined {
  const cm = TM_MERIDIANS.reduce((best, m) => (Math.abs(m - lon) < Math.abs(best - lon) ? m : best));
  return crsBySrid(5253 + TM_MERIDIANS.indexOf(cm));
}

export function searchCrs(query: string): CrsDef[] {
  const q = query.trim().toLocaleLowerCase('tr-TR').replace(/^epsg:?/, '');
  if (!q) return [...CRS_REGISTRY];
  return CRS_REGISTRY.filter((c) => String(c.srid).startsWith(q) || c.name.toLocaleLowerCase('tr-TR').includes(q) || c.area.toLocaleLowerCase('tr-TR').includes(q));
}
