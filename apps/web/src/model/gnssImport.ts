import type { GnssLine } from '../contracts/generated/GnssLine';
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

/**
 * What the import takes: the kinds of points, how the unnamed are named (the prefix, then a number from `start`), and
 * whether the routes and tracks come in as lines (docs/adr/0222 §6).
 */
export interface GnssOptions {
  readonly kinds: readonly string[];
  readonly prefix: string;
  readonly start: number;
  readonly lines?: boolean;
}

/** A route's, track's or log's line as the import writes it: its parts' vertices with their points' ellipsoidal heights. */
export interface PlacedLine {
  readonly line: number;
  readonly name: string | null;
  readonly parts: { readonly p: { readonly x: number; readonly y: number }; readonly z: number | null }[][];
  readonly attrs: Readonly<Record<string, string>>;
}

/** A point as the import writes it, and its line in the file. */
export interface PlacedPoint {
  readonly line: number;
  readonly name: string;
  readonly p: { readonly x: number; readonly y: number };
  readonly z: number | null;
  readonly attrs: Readonly<Record<string, string>>;
}

/** The points and lines to write, and those the project's system does not reach (or a line without a part of two points), with why. */
export interface GnssPlan {
  readonly placed: PlacedPoint[];
  readonly skipped: LineError[];
  readonly lines: PlacedLine[];
  readonly linesSkipped: LineError[];
}

/** What is said of a point the project's system does not reach; `{line}`, `{name}` and `{why}` are filled in. */
export const SKIPPED = 'Satır {line}: {name} noktası projenin sistemine çevrilemedi ({why}); eklenmedi.';

/** What is said of a line without a part of two points, and of one the project's system does not reach. */
export const THIN = 'Satır {line}: {name} çizgisinin iki noktalı parçası yok; eklenmedi.';
export const LINE_SKIPPED = 'Satır {line}: {name} çizgisi projenin sistemine çevrilemedi ({why}); eklenmedi.';

/** A line's Tür, Kaynak and the name of one without its own. */
const LINE_KIND: Record<string, readonly [string, string, string]> = {
  rte: ['GNSS rotası', 'GPX rotası', 'Adsız rota'],
  nmea: ['GNSS izi', 'NMEA kaydı', 'NMEA kaydı'],
  trk: ['GNSS izi', 'GPX izi', 'Adsız iz'],
};

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
  const plan: GnssPlan = { placed: [], skipped: [], lines: [], linesSkipped: [] };
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

/** The file's lines in `to` (named `system`), in the file's order: each part of two points or more its points moved as the points are. */
export function placeGnssLines(
  points: readonly GnssPoint[],
  lines: readonly GnssLine[],
  to: System,
  system: string,
  choices: readonly DatumChoice[] = [],
): { lines: PlacedLine[]; skipped: LineError[] } {
  const out: PlacedLine[] = [];
  const skipped: LineError[] = [];
  let way: string | null = null;
  for (const l of lines) {
    const [kind, source, unnamed] = LINE_KIND[l.kind] ?? LINE_KIND.trk!;
    const name = l.name || null;
    const say = (text: string, why: string): LineError => ({
      line: l.line,
      message: text.replace('{line}', String(l.line)).replace('{why}', why).replace('{name}', name ?? unnamed),
    });
    const parts = l.parts.filter((p) => p.length >= 2);
    if (!parts.length) {
      skipped.push(say(THIN, ''));
      continue;
    }
    const runs: PlacedLine['parts'] = [];
    let missed: Unreached | null = null;
    for (const part of parts) {
      const run: PlacedLine['parts'][number] = [];
      for (const i of part) {
        const g = points[i];
        if (!g) continue;
        const moved = crsTransformIn(WGS84, to, { x: g.lon, y: g.lat }, choices);
        if ('error' in moved) {
          missed = moved.error;
          break;
        }
        way ??= `WGS 84 → ${system}: ${accuracyText(moved)}`;
        run.push({ p: moved.point, z: g.ellipsoidal ?? null });
      }
      if (missed) break;
      runs.push(run);
    }
    if (missed) {
      skipped.push(say(LINE_SKIPPED, WHY[missed]));
      continue;
    }
    const attrs: Record<string, string> = {};
    if (name) attrs.Ad = name;
    attrs.Tür = kind;
    attrs.Kaynak = source;
    attrs.Nokta = String(runs.reduce((n, r) => n + r.length, 0));
    attrs.Dönüşüm = way ?? '';
    out.push({ line: l.line, name, parts: runs, attrs });
  }
  return { lines: out, skipped };
}

/** The points of `options.kinds` in `to` (named `system`), in the file's order; and, asked to, the file's lines. */
export function placeGnssAll(
  points: readonly GnssPoint[],
  lines: readonly GnssLine[],
  options: GnssOptions,
  to: System,
  system: string,
  choices: readonly DatumChoice[] = [],
): GnssPlan {
  const plan = placeGnss(points, options, to, system, choices);
  if (!options.lines) return plan;
  const placed = placeGnssLines(points, lines, to, system, choices);
  return { ...plan, lines: placed.lines, linesSkipped: placed.skipped };
}

/** The placed lines as the import writes them: polylines on the source layer `''`, a part after the first a part of the polyline. */
export function gnssLineEntities(lines: readonly PlacedLine[]): ContractEntity[] {
  const run = (r: PlacedLine['parts'][number]) => {
    const zs = r.map((v) => v.z);
    return { pts: r.map((v) => ({ x: v.p.x, y: v.p.y })), ...(zs.some((z) => z !== null) ? { zs } : {}) };
  };
  return lines.flatMap((l): ContractEntity[] => {
    const [first, ...rest] = l.parts;
    if (!first) return [];
    return [
      {
        kind: 'polyline',
        id: 0,
        layerId: '',
        attrs: { ...l.attrs },
        ...(l.name ? { label: l.name } : {}),
        ...run(first),
        ...(rest.length ? { parts: rest.map(run) } : {}),
      },
    ];
  });
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
