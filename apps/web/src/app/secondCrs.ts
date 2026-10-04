import { crsBySrid, DATUM_LABEL, LOCAL_SRID, CRS_REGISTRY, type CrsDef } from '../geo/crs';
import type { Vec2 } from '../model/geometry';
import { crsTransform, formatDd, formatDms, systemOf, type System, type Transformed } from '../model/geom/crsTransform';
import type { ProjectSettings } from '../model/projectSettings';
import type { Formatter } from './format';

/**
 * The project's second coordinate system (docs/adr/0167 §1–§2, §5): a point of the drawing in it, written as the user
 * reads it, with how sure the values are. The status bar and Koordinat oku read it; the desktop's
 * `kentos_interaction::second` writes the same.
 *
 * A projected second system is written as the project's points are: east first, named as the project's type names its
 * axes, with its length decimals (`Y=412379.977, X=4512531.676`). A geographic one is latitude first, in the user's
 * notation (`display.geographic`) with fixed digits: `40°45′12.3456″K, 29°55′01.2345″D` or `40.7534293°K, 29.9170096°D`.
 */

/** How a geographic second system's latitude and longitude are written (`display.geographic`). */
export type GeographicNotation = 'dms' | 'dd';

const write = (notation: GeographicNotation, deg: number, latitude: boolean): string =>
  notation === 'dd' ? formatDd(deg, latitude, 7) : formatDms(deg, latitude, 4);

export class SecondCrs {
  /** Its registry entry. */
  readonly system: CrsDef;
  private readonly from: System;
  private readonly to: System;
  /** ED50 on either side: the values are not an official transformation's (§5). */
  private readonly unofficial: boolean;

  private constructor(system: CrsDef, from: System, to: System, unofficial: boolean) {
    this.system = system;
    this.from = from;
    this.to = to;
    this.unofficial = unofficial;
  }

  /** The second system of a project that has one the registry knows; null for none, a local project, one the transforms do not read. */
  static of(settings: ProjectSettings): SecondCrs | null {
    const system = settings.second;
    const project = settings.crs.value;
    if (!system || project.srid === LOCAL_SRID) return null;
    const from = systemOf(project);
    const to = systemOf(system);
    return from && to ? new SecondCrs(system, from, to, project.datum === 'ED50' || system.datum === 'ED50') : null;
  }

  /** Its name without the slash: “ED50 TM30”, “WGS 84 UTM 35N”, “TUREF”. */
  get short(): string {
    return this.system.name.replace(' / ', ' ');
  }

  /** Whether its values are a latitude and a longitude. */
  get geographic(): boolean {
    return this.to.kind === 'geographic';
  }

  /** `p`, a point of the drawing, in the second system; null where the projection does not reach. */
  point(p: Vec2): Transformed | null {
    return crsTransform(this.from, this.to, p);
  }

  /** The two values of `q` (a point in the second system) with their names: east and north as the project's type names them, or the latitude and the longitude. */
  values(q: Vec2, f: Formatter, notation: GeographicNotation): [[string, string], [string, string]] {
    return this.geographic
      ? [
          ['Enlem', write(notation, q.y, true)],
          ['Boylam', write(notation, q.x, false)],
        ]
      : [
          [f.eastLabel, f.coord(q.x)],
          [f.northLabel, f.coord(q.y)],
        ];
  }

  /** The values on one line, as Koordinat oku says them: `Y=…, X=…`, or `40°45′12.3456″K, 29°55′01.2345″D`. */
  reading(q: Vec2, f: Formatter, notation: GeographicNotation): string {
    const [a, b] = this.values(q, f, notation);
    return this.geographic ? `${a[1]}, ${b[1]}` : `${a[0]}=${a[1]}, ${b[0]}=${b[1]}`;
  }

  /** How sure the values are: “±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil”, “±1 m, EPSG:5261”, or “kesin, yalnız projeksiyon” within one datum. */
  accuracy(t: Transformed): string {
    if (!t.via) return 'kesin, yalnız projeksiyon';
    return `±${t.accuracy} m, ${t.via}${this.unofficial ? '; resmî dönüşüm değil' : ''}`;
  }
}

/** The systems a project in `project` may take as its second, grouped by datum as the registry lists them: every one but the local and its own. */
export function secondChoices(project: CrsDef): { datum: string; systems: CrsDef[] }[] {
  const groups: { datum: string; systems: CrsDef[] }[] = [];
  for (const c of CRS_REGISTRY) {
    if (c.kind === 'local' || c.srid === project.srid) continue;
    const label = DATUM_LABEL[c.datum];
    const g = groups.find((x) => x.datum === label) ?? groups[groups.push({ datum: label, systems: [] }) - 1]!;
    g.systems.push(c);
  }
  return groups;
}

/** A second system's title in a sentence: “ED50 / TM30 (EPSG:2320)”; an unknown one by its code. */
export const secondTitle = (srid: number): string => {
  const c = crsBySrid(srid);
  return c ? `${c.name} (EPSG:${c.srid})` : `EPSG:${srid}`;
};
