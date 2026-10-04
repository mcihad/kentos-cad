import type { GnssPoint } from '../contracts/generated/GnssPoint';
import type { LineError } from '../contracts/generated/LineError';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { fixed } from '../core/displayNumber';
import { crsTransformIn, formatDd, type DatumChoice, type System, type Unreached } from './geom/crsTransform';
import { accuracyText } from './secondCrs';

/**
 * The GNSS import's points (docs/adr/0169 §6; the desktop's `kentos_interaction::gnss`): a GNSS file's positions of the
 * chosen kinds moved from WGS 84 into the project's system the way Koordinat dönüştür moves them (docs/adr/0167, the
 * project's datum choices taken), named, with their fix and heights as attributes and their ellipsoidal height, when the
 * file gives one, as their elevation: a height whose datum the file does not say is never taken for it. The shared cases
 * are fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py, with PROJ).
 */

/** What the import takes: the kinds of points, and how the unnamed are named (the prefix, then a number from `start`). */
export interface GnssOptions {
  readonly kinds: readonly string[];
  readonly prefix: string;
  readonly start: number;
}

/** A point as the import writes it, and its line in the file. */
export interface PlacedPoint {
  readonly line: number;
  readonly name: string;
  readonly p: { readonly x: number; readonly y: number };
  readonly z: number | null;
  readonly attrs: Readonly<Record<string, string>>;
}

/** The points to write, and those the project's system does not reach, with why. */
export interface GnssPlan {
  readonly placed: PlacedPoint[];
  readonly skipped: LineError[];
}

/** What is said of a point the project's system does not reach; `{line}`, `{name}` and `{why}` are filled in. */
export const SKIPPED = 'Satır {line}: {name} noktası projenin sistemine çevrilemedi ({why}); eklenmedi.';

/** A point's source as its Kaynak attribute names it. */
export function gnssSource(kind: string): string {
  switch (kind) {
    case 'wpt':
      return 'GPX yol noktası';
    case 'rtept':
      return 'GPX rota noktası';
    case 'trkpt':
      return 'GPX iz noktası';
    case 'gga':
      return 'NMEA GGA';
    default:
      return 'GNSS';
  }
}

/** Why a point is not reached, in a few words. */
const WHY: Record<Unreached, string> = {
  outside: 'sistemin ulaştığı yerin dışında',
  noLink: "datumlardan birinin WGS 84'e dönüşümü yok",
  noGrid: 'datum dönüşümünün ızgarası bu cihazda yok',
  outsideGrid: 'datum dönüşümünün ızgarasının dışında',
};

const WGS84: System = { kind: 'geographic', datum: 'WGS84' };

/** The points of `options.kinds` in `to` (named `system`), in the file's order. */
export function placeGnss(points: readonly GnssPoint[], options: GnssOptions, to: System, system: string, choices: readonly DatumChoice[] = []): GnssPlan {
  const plan: GnssPlan = { placed: [], skipped: [] };
  let number = options.start;
  for (const g of points) {
    if (!options.kinds.includes(g.kind)) continue;
    const moved = crsTransformIn(WGS84, to, { x: g.lon, y: g.lat }, choices);
    if ('error' in moved) {
      const message = SKIPPED.replace('{line}', String(g.line))
        .replace('{why}', WHY[moved.error])
        .replace('{name}', g.name ?? 'Adsız');
      plan.skipped.push({ line: g.line, message });
      continue;
    }
    let name = g.name;
    if (!name) {
      name = `${options.prefix}${number}`;
      number += 1;
    }
    const attrs: Record<string, string> = { Ad: name, Tür: 'GNSS noktası', Kaynak: gnssSource(g.kind) };
    const put = (key: string, value: string | undefined): void => {
      if (value !== undefined) attrs[key] = value;
    };
    put('Çözüm', g.fix);
    put('Uydu', g.satellites === undefined ? undefined : String(g.satellites));
    put('HDOP', g.hdop === undefined ? undefined : fixed(g.hdop, 1));
    put('Zaman', g.time);
    put('Enlem', formatDd(g.lat, true, 9));
    put('Boylam', formatDd(g.lon, false, 9));
    put('Elipsoit yüksekliği (m)', g.ellipsoidal === undefined ? undefined : fixed(g.ellipsoidal, 3));
    put('Yükseklik (dosyada, m)', g.height === undefined ? undefined : fixed(g.height, 3));
    put('Geoit ayrımı (m)', g.geoid === undefined ? undefined : fixed(g.geoid, 3));
    put('Dönüşüm', `WGS 84 → ${system}: ${accuracyText(moved)}`);
    plan.placed.push({ line: g.line, name, p: moved.point, z: g.ellipsoidal ?? null, attrs });
  }
  return plan;
}

/**
 * The placed points as the import writes them (`io/apply.ts`): on the source layer `''`, which the window sends to its
 * target layer, their names as labels.
 */
export function gnssEntities(placed: readonly PlacedPoint[]): ContractEntity[] {
  return placed.map((pt) => ({
    kind: 'point',
    id: 0,
    layerId: '',
    attrs: { ...pt.attrs },
    label: pt.name,
    p: { x: pt.p.x, y: pt.p.y },
    ...(pt.z !== null ? { z: pt.z } : {}),
  }));
}
