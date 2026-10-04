import { crsBySrid, DATUM_LABEL, CRS_REGISTRY, type CrsDef } from '../geo/crs';
import type { Vec2 } from './geometry';
import {
  crsPlaneMeasures,
  crsTransformIn,
  formatDd,
  formatDms,
  type DatumChoice,
  type PlaneMeasures,
  type PlaneRing,
  type System,
  type Transformed,
  type Unreached,
} from './geom/crsTransform';
import { crsSettings, datumChoices, ownSystem, secondSystem } from './projectCrs';

export type { PlaneRing };
import type { ProjectSettings } from './projectSettings';

/**
 * The project's second coordinate system (docs/adr/0167 §1–§2, §5): a point of the drawing in it, written as the user
 * reads it, with how sure the values are. The status bar and Koordinat oku read it; the desktop's
 * `kentos_interaction::second` writes the same. Either system may be one of the registry's or a definition of the
 * project's, and the project's datum choices are taken where they apply (docs/adr/0168, `projectCrs.ts`).
 *
 * A projected second system is written as the project's points are: east first, named as the project's type names its
 * axes, with its length decimals (`Y=412379.977, X=4512531.676`). A geographic one is latitude first, in the user's
 * notation (`display.geographic`) with fixed digits: `40°45′12.3456″K, 29°55′01.2345″D` or `40.7534293°K, 29.9170096°D`.
 */

/** How a geographic second system's latitude and longitude are written (`display.geographic`). */
export type GeographicNotation = 'dms' | 'dd';

/** What of the project's number formats the second system's values need (app/format.ts `Formatter` has it). */
export interface SecondFormat {
  readonly eastLabel: string;
  readonly northLabel: string;
  coord(v: number): string;
  length(m: number): string;
  area(m2: number): string;
}

const write = (notation: GeographicNotation, deg: number, latitude: boolean): string =>
  notation === 'dd' ? formatDd(deg, latitude, 7) : formatDms(deg, latitude, 4);

export class SecondCrs {
  /** Its name: “ED50 / TM30”, “Belediye sistemi”. */
  readonly name: string;
  /** As a sentence names it: “ED50 / TM30 (EPSG:2320)”, “Belediye sistemi (özel sistem)”. */
  readonly title: string;
  private readonly from: System;
  private readonly to: System;
  /** The project's datum choices (docs/adr/0168 §3). */
  private readonly choices: readonly DatumChoice[];

  private constructor(name: string, title: string, from: System, to: System, choices: readonly DatumChoice[]) {
    this.name = name;
    this.title = title;
    this.from = from;
    this.to = to;
    this.choices = choices;
  }

  /** The second system of a project that has one; null for none, a project without a system, one the transforms do not read. */
  static of(settings: ProjectSettings): SecondCrs | null {
    const s = crsSettings(settings);
    const [own, second] = [ownSystem(s), secondSystem(s)];
    return own?.system && second?.system ? new SecondCrs(second.name, second.title, own.system, second.system, datumChoices(s)) : null;
  }

  /** Its name without the slash: “ED50 TM30”, “WGS 84 UTM 35N”, “TUREF”. */
  get short(): string {
    return this.name.replace(' / ', ' ');
  }

  /** Whether its values are a latitude and a longitude. */
  get geographic(): boolean {
    return this.to.kind === 'geographic';
  }

  /** `p`, a point of the drawing, in the second system; or why it has no value there. */
  point(p: Vec2): Transformed | { readonly error: Unreached } {
    return crsTransformIn(this.from, this.to, p, this.choices);
  }

  /** The two values of `q` (a point in the second system) with their names: east and north as the project's type names them, or the latitude and the longitude. */
  values(q: Vec2, f: SecondFormat, notation: GeographicNotation): [[string, string], [string, string]] {
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
  reading(q: Vec2, f: SecondFormat, notation: GeographicNotation): string {
    const [a, b] = this.values(q, f, notation);
    return this.geographic ? `${a[1]}, ${b[1]}` : `${a[0]}=${a[1]}, ${b[0]}=${b[1]}`;
  }

  /**
   * A path (`closed` false) or an area's rings (the outer first), given in the project's system, measured in the second
   * system's plane (docs/adr/0167 §2): none in a geographic system or the Pseudo-Mercator, or where it does not reach.
   */
  measure(rings: readonly PlaneRing[], closed: boolean): PlaneMeasures {
    return crsPlaneMeasures(this.from, this.to, rings, closed, this.choices);
  }

  /** The line Mesafe ölç and Alan hesapla say after their own: the length, or the area and the perimeter, in the second system's plane, or why there are none. */
  measuresLine(rings: readonly PlaneRing[], closed: boolean, f: SecondFormat): string {
    const name = this.short;
    const m = this.measure(rings, closed);
    if ('why' in m) {
      if (m.why === 'geographic') return `${name} coğrafi bir sistem: uzunluk ve alan onun düzleminde verilmez.`;
      if (m.why === 'mercator') return `${name}: uzunluk ve alan verilmez, Pseudo-Mercator'un ölçeği her enlemde başkadır.`;
      if (m.why === 'noLink') return `${name}: datumlardan birinin WGS 84'e dönüşümü yok; değer yazılmadı.`;
      if (m.why === 'noGrid') return `${name}: datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.`;
      return `${name}: ölçülen yerin bir noktası bu sistemin ulaştığı yerin dışında; değer yazılmadı.`;
    }
    return closed ? `${name} düzleminde: Alan ${f.area(m.area)}   Çevre ${f.length(m.length)}` : `${name} düzleminde: Toplam uzunluk ${f.length(m.length)}`;
  }

  /** How sure the values are: “±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil”, “±1 m, EPSG:5261”, or “kesin, yalnız projeksiyon” within one datum. */
  accuracy(t: Transformed): string {
    return accuracyText(t);
  }
}

/** Why the cursor has no value in the second system, as the status bar's tip says it (the desktop's `cursor_unreached`). */
export function cursorUnreached(why: Unreached): string {
  switch (why) {
    case 'outside':
      return 'İmleç bu sistemin ulaştığı yerin dışında; değer yazılmadı.';
    case 'outsideGrid':
      return 'İmleç datum dönüşümünün ızgarasının dışında; değer yazılmadı.';
    case 'noLink':
      return "Datumlardan birinin WGS 84'e dönüşümü yok; değer yazılmadı.";
    case 'noGrid':
      return 'Datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.';
  }
}

/** Why a point has no value in the second system, as Koordinat oku says it after the system's name (the desktop's `point_unreached`). */
export function pointUnreached(why: Unreached): string {
  switch (why) {
    case 'outside':
      return 'nokta bu sistemin ulaştığı yerin dışında; değeri yazılmadı.';
    case 'outsideGrid':
      return 'nokta datum dönüşümünün ızgarasının dışında; değeri yazılmadı.';
    case 'noLink':
      return "datumlardan birinin WGS 84'e dönüşümü yok; değeri yazılmadı.";
    case 'noGrid':
      return 'datum dönüşümünün ızgarası bu cihazda yok; değeri yazılmadı.';
  }
}

/**
 * How sure a point moved between two systems is: “±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil” (an EPSG
 * operation of ED50 was used, docs/adr/0167 §5), “±1 m, EPSG:5261”, “doğruluğu bilinmiyor, Bölge 7” (a step's accuracy
 * is not written, docs/adr/0168 §2), or “kesin, yalnız projeksiyon” within one datum (the desktop's `accuracy_text`).
 */
export function accuracyText(t: Transformed): string {
  if (!t.via) return 'kesin, yalnız projeksiyon';
  const sure = t.accuracy === undefined ? 'doğruluğu bilinmiyor' : `±${t.accuracy} m`;
  return `${sure}, ${t.via}${t.unofficial ? '; resmî dönüşüm değil' : ''}`;
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
