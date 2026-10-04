import { fixed } from '../core/displayNumber';
import { crsGroundMeasures, crsHasPointScale, type PlaneRing } from './geom/crsTransform';
import type { Grid } from './geom/surveyCalc';
import { crsSettings, ownSystem, type CrsSettings } from './projectCrs';
import type { ProjectSettings } from './projectSettings';

/**
 * Plane, ellipsoid and ground in Mesafe ölç and Alan hesapla (docs/adr/0171 §4a): after their own line, what was
 * measured on the project's ellipsoid and on the ground at its mean ellipsoidal height (the core's `crs::ground`); the
 * desktop's `kentos_interaction::ground` writes the same. Only a project that names the height asks for them: any other
 * says what it said before, its own line last (the status bar shows the last), and a project without a coordinate system
 * has no ellipsoid.
 */

/** What of the project's number formats the lines need (app/format.ts `Formatter` has it). */
export interface GroundFormat {
  length(m: number): string;
  area(m2: number): string;
}

/** Said in place of both when a point is beyond the project's system. */
export const UNREACHED = 'Elipsoit üstünde: ölçülen yerin bir noktası projenin sisteminin ulaştığı yerin dışında; değer yazılmadı.';

/** A height as the form writes it: the display rule's four decimals without trailing zeros or a bare point. */
export function trimmedHeight(v: number): string {
  let s = fixed(v, 4);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  return s === '' || s === '-0' ? '0' : s;
}

/**
 * The lines Mesafe ölç (`closed` false: the first ring, a path) and Alan hesapla (the rings, the first the outer) say
 * after their own: the ellipsoid's, with the plane's scale against it, and the ground's, with the height's factor; none
 * in a project that names no height or has no coordinate system.
 */
export function groundLines(settings: ProjectSettings, rings: readonly PlaneRing[], closed: boolean, f: GroundFormat): string[] {
  const height = settings.groundHeight;
  if (height === null) return [];
  const system = ownSystem(crsSettings(settings))?.system;
  if (!system) return [];
  const m = crsGroundMeasures(system, rings, closed, height);
  if ('why' in m) return [UNREACHED];
  const what = (area: number | undefined, length: number) =>
    area !== undefined && closed ? `Alan ${f.area(area)}   Çevre ${f.length(length)}` : `Toplam uzunluk ${f.length(length)}`;
  const scale = m.scale !== undefined ? `   Ölçek ${fixed(m.scale, 8)}` : '';
  const ellipsoid = `Elipsoit üstünde: ${what(m.ellipsoidArea, m.ellipsoidLength)}${scale}`;
  const factor = m.heightFactor !== undefined ? `   Yükseklik çarpanı ${fixed(m.heightFactor, 8)}` : '';
  return [ellipsoid, `Zeminde (h = ${trimmedHeight(height)} m): ${what(m.groundArea, m.groundLength ?? 0)}${factor}`];
}

/** Why Uzunlukları projeksiyona indir cannot be turned on (docs/adr/0171 §4); the desktop's `kentos_interaction::ground`'s. */
export const NEEDS_HEIGHT = 'Ortalama elipsoit yüksekliği yazılınca açılır.';
export const NEEDS_SYSTEM = 'Projenin koordinat sistemi yok: uzunluklar projeksiyona indirilemez.';
export const NEEDS_SCALE =
  'Projenin sisteminde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem): uzunluklar projeksiyona indirilemez.';

/** Why the survey windows cannot take lengths to the grid with these systems and this height, or null. */
export function whyNotGrid(crs: CrsSettings, height: number | null | undefined): string | null {
  if (height === null || height === undefined) return NEEDS_HEIGHT;
  const system = ownSystem(crs)?.system;
  if (!system) return NEEDS_SYSTEM;
  return crsHasPointScale(system) ? null : NEEDS_SCALE;
}

/**
 * The grid the survey windows take measured lengths to (docs/adr/0171 §4): the project asks for it (Uzunlukları
 * projeksiyona indir, with a height) and its system has one scale at a point; null otherwise.
 */
export function surveyGrid(settings: ProjectSettings): Grid | null {
  const height = settings.groundHeight;
  if (!settings.reducesToGrid || height === null || whyNotGrid(crsSettings(settings), height)) return null;
  const system = ownSystem(crsSettings(settings))?.system;
  return system ? { system, height } : null;
}

/** What Kutupsal alım and Poligon hesabı say when they take lengths to the grid. */
export const gridNote = (height: number): string =>
  `Ölçülen uzunluklar projeksiyona indirildi: ortalama elipsoit yüksekliği ${trimmedHeight(height)} m, çizginin ölçeği ve yükseklik çarpanıyla (Proje ayarları › Ölçme).`;

/** What Aplikasyon says when it gives the ground's distances. */
export const stakeNote = (height: number): string =>
  `Zemin uzunlukları da verildi: ortalama elipsoit yüksekliği ${trimmedHeight(height)} m, çizginin ölçeği ve yükseklik çarpanıyla (Proje ayarları › Ölçme). Arazide zemindekini ölçün.`;
