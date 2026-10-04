import type { Convention } from '../contracts/generated/Convention';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { CrsPlane } from '../contracts/generated/CrsPlane';
import type { CrsSystem } from '../contracts/generated/CrsSystem';
import type { CustomDatum } from '../contracts/generated/CustomDatum';
import type { RegistryDatum } from '../contracts/generated/RegistryDatum';
import { CRS_REGISTRY, crsBySrid } from '../geo/crs';
import { crsReadText, crsWriteProj, crsWriteWkt } from './geom/crsText';
import { fitTransform, type FitPair } from './ops/fit';
import { systemOf } from './geom/crsTransform';
import { definitionFrom, definitionSystem } from './projectCrs';

/**
 * The Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §5–§6; the desktop's
 * `kentos_project::definition_form`): what is typed turned into the project's definition (`CrsDefinition`) or, field by
 * field, what is wrong; a definition the registry has already is said; a WKT or PROJ text read into one, or why not; a
 * definition written as WKT and PROJ; a local system's plane from points known in both systems. The shared cases are
 * fixtures/crs/v1/definition-form.json, definition-text.json and definition-fit.json
 * (scripts/fixtures/crs_definition_form_cases.py, crs_definition_text_cases.py and crs_definition_fit_cases.py, from the
 * ADR's rules).
 */

export type Kind = 'tm' | 'geographic' | 'local';
/** A definition's datum: one of the registry's by name, or the project's own. */
export type DatumPick = RegistryDatum | 'custom';
export type PlaneKind = 'similarity' | 'affine';

/** The classic ellipsoids by name (EPSG's): semi-major axis (m), inverse flattening. */
export const ELLIPSOIDS: readonly (readonly [string, number, number])[] = [
  ['GRS 1980', 6378137, 298.257222101],
  ['WGS 84', 6378137, 298.257223563],
  ['International 1924', 6378388, 297],
  ['Bessel 1841', 6377397.155, 299.1528128],
  ['Krasovski 1940', 6378245, 298.3],
  ['Clarke 1880 (RGS)', 6378249.145, 293.465],
];

export const PARAMETERS = ['tx', 'ty', 'tz', 'rx', 'ry', 'rz', 'ds'] as const;
export type Parameter = (typeof PARAMETERS)[number];
export const AFFINE = ['a', 'b', 'c', 'd', 'e', 'f'] as const;
export type Coefficient = (typeof AFFINE)[number];

/** What the window holds, as typed. */
export interface DefinitionForm {
  name: string;
  kind: Kind;
  latitudeOfOrigin: string;
  centralMeridian: string;
  scaleFactor: string;
  falseEasting: string;
  falseNorthing: string;
  datum: DatumPick;
  datumName: string;
  /** One of `ELLIPSOIDS` by its name, or 'custom': typed. */
  ellipsoid: string;
  semiMajor: string;
  inverseFlattening: string;
  /** The project's datum is bound to WGS 84 by seven parameters. */
  linked: boolean;
  parameters: Record<Parameter, string>;
  convention: Convention;
  accuracy: string;
  /** The local system's base: a projected system of the registry by its code. */
  base: string;
  plane: PlaneKind;
  east: string;
  north: string;
  rotation: string;
  scale: string;
  a: string;
  b: string;
  c: string;
  d: string;
  e: string;
  f: string;
}

export type Problems = Record<string, string>;

export const TEXTS = {
  name: 'Adını yazın: sistem bu adla görünür.',
  number: 'Sayı yazın.',
  meridian: '−180 ile 180 arasında bir derece yazın.',
  latitude: '−90 ile 90 arasında bir derece yazın.',
  positive: "0'dan büyük bir sayı yazın.",
  datumName: 'Datumun adını yazın.',
  semiMajor: "Büyük yarı ekseni metre olarak, 0'dan büyük yazın.",
  inverseFlattening: "Ters basıklığı 1'den büyük yazın.",
  accuracy: '0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.',
  base: 'Kayıttaki projeksiyonlu bir sistem seçin.',
  folds: 'Bu katsayılar düzlemi katlıyor (a·e − b·d = 0).',
} as const;

const emptyParameters = (): Record<Parameter, string> => ({ tx: '', ty: '', tz: '', rx: '', ry: '', rz: '', ds: '' });

/** A new window: an empty transverse Mercator on TUREF. */
export const emptyForm = (): DefinitionForm => ({
  name: '',
  kind: 'tm',
  latitudeOfOrigin: '',
  centralMeridian: '',
  scaleFactor: '',
  falseEasting: '',
  falseNorthing: '',
  datum: 'TUREF',
  datumName: '',
  ellipsoid: 'GRS 1980',
  semiMajor: '',
  inverseFlattening: '',
  linked: true,
  parameters: emptyParameters(),
  convention: 'positionVector',
  accuracy: '',
  base: '',
  plane: 'similarity',
  east: '',
  north: '',
  rotation: '',
  scale: '',
  a: '',
  b: '',
  c: '',
  d: '',
  e: '',
  f: '',
});

/** A number as the Hesap windows read one: trimmed, its first comma a point; null for anything else. */
export function number(text: string): number | null {
  const t = text.trim().replace(',', '.');
  return t && /^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$/i.test(t) ? Number(t) : null;
}

/** Reads the form's numbers, keeping the problems by field. */
class Reader {
  readonly problems: Problems = {};

  /** The number typed in `key`: `fallback` when empty (undefined: required), `problem` when `ok` fails. 0 when wrong. */
  number(key: string, text: string, fallback?: number, ok?: (v: number) => boolean, problem?: keyof typeof TEXTS): number {
    const t = text.trim();
    if (!t && fallback !== undefined) return fallback;
    const v = number(t);
    if (v === null) {
      this.problems[key] = TEXTS.number;
      return 0;
    }
    if (ok && !ok(v)) {
      this.problems[key] = TEXTS[problem!];
      return 0;
    }
    return v;
  }
}

function customDatum(f: DefinitionForm, r: Reader): CustomDatum {
  const name = f.datumName.trim();
  if (!name) r.problems.datumName = TEXTS.datumName;
  const preset = ELLIPSOIDS.find(([n]) => n === f.ellipsoid);
  const ellipsoid = preset
    ? { name: preset[0], semiMajor: preset[1], inverseFlattening: preset[2] }
    : {
        name: `a=${f.semiMajor.trim()}, 1/f=${f.inverseFlattening.trim()}`,
        semiMajor: r.number('semiMajor', f.semiMajor, undefined, (v) => v > 0, 'semiMajor'),
        inverseFlattening: r.number('inverseFlattening', f.inverseFlattening, undefined, (v) => v > 1, 'inverseFlattening'),
      };
  const datum: CustomDatum = { name, ellipsoid };
  if (f.linked) {
    // An empty rotation or scale difference is 0: three parameters.
    const v = PARAMETERS.map((k, i) => r.number(k, f.parameters[k], i >= 3 ? 0 : undefined));
    let accuracy: number | undefined;
    if (f.accuracy.trim()) {
      const a = number(f.accuracy);
      if (a === null || a < 0) r.problems.accuracy = TEXTS.accuracy;
      else accuracy = a;
    }
    datum.toWgs84 = {
      translation: [v[0]!, v[1]!, v[2]!],
      rotation: [v[3]!, v[4]!, v[5]!],
      scale: v[6]!,
      convention: f.convention,
      ...(accuracy !== undefined ? { accuracy } : {}),
    };
  }
  return datum;
}

const datumFields = (f: DefinitionForm, r: Reader): { datum: RegistryDatum } | { customDatum: CustomDatum } =>
  f.datum === 'custom' ? { customDatum: customDatum(f, r) } : { datum: f.datum };

/** The definition the window gives, with what to say of it (the registry's system it is); or what is wrong. */
export function buildDefinition(f: DefinitionForm): { readonly definition: CrsDefinition; readonly note?: string } | { readonly problems: Problems } {
  const r = new Reader();
  const name = f.name.trim();
  if (!name) r.problems.name = TEXTS.name;
  let system: CrsSystem;
  if (f.kind === 'tm') {
    const datum = datumFields(f, r);
    const lat0 = r.number('latitudeOfOrigin', f.latitudeOfOrigin, 0, (v) => v >= -90 && v <= 90, 'latitude');
    system = {
      kind: 'tm',
      ...datum,
      ...(lat0 !== 0 ? { latitudeOfOrigin: lat0 } : {}),
      centralMeridian: r.number('centralMeridian', f.centralMeridian, undefined, (v) => v >= -180 && v <= 180, 'meridian'),
      scaleFactor: r.number('scaleFactor', f.scaleFactor, 1, (v) => v > 0, 'positive'),
      falseEasting: r.number('falseEasting', f.falseEasting),
      falseNorthing: r.number('falseNorthing', f.falseNorthing),
    };
  } else if (f.kind === 'geographic') {
    system = { kind: 'geographic', ...datumFields(f, r) };
  } else {
    const entry = /^\d+$/.test(f.base.trim()) ? crsBySrid(Number(f.base.trim())) : undefined;
    const base = entry && entry.kind === 'projected' ? entry : undefined;
    if (!base) r.problems.base = TEXTS.base;
    let plane: CrsPlane;
    if (f.plane === 'similarity') {
      plane = {
        kind: 'similarity',
        east: r.number('east', f.east),
        north: r.number('north', f.north),
        rotation: r.number('rotation', f.rotation, 0),
        scale: r.number('scale', f.scale, 1, (v) => v > 0, 'positive'),
      };
    } else {
      const v = AFFINE.map((k) => r.number(k, f[k]));
      const read = (['a', 'b', 'd', 'e'] as const).every((k) => !(k in r.problems));
      if (read && v[0]! * v[4]! - v[1]! * v[3]! === 0) r.problems.a = TEXTS.folds;
      plane = { kind: 'affine', a: v[0]!, b: v[1]!, c: v[2]!, d: v[3]!, e: v[4]!, f: v[5]! };
    }
    system = { kind: 'local', base: { srid: base?.srid ?? 0 }, plane };
  }
  if (Object.keys(r.problems).length) return { problems: r.problems };
  const definition: CrsDefinition = { name, system };
  const note = sameAsRegistry(definition);
  return note ? { definition, note } : { definition };
}

/** The registry's system a definition on one of its datums is, every value the same (the window's Kayıttakini seç). */
export function sameSrid(d: CrsDefinition): number | null {
  const s = d.system;
  if (s.kind === 'local' || s.datum === undefined) return null;
  const it = definitionSystem(d);
  if (!it) return null;
  const same = CRS_REGISTRY.find((c) => c.kind !== 'local' && JSON.stringify(systemOf(c)) === JSON.stringify(it));
  return same ? same.srid : null;
}

/** “EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.” */
function sameAsRegistry(d: CrsDefinition): string | null {
  const srid = sameSrid(d);
  const c = srid === null ? undefined : crsBySrid(srid);
  return c ? `EPSG:${c.srid} (${c.name}) ile aynı; kayıttakini seçin.` : null;
}

/** What reading a text says when it gives no definition, and of a grid the registry has on the text's own datum. */
export const READ_TEXTS = {
  syntax: 'Metin okunamadı: WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj= ile başlayan bir PROJ dizesi yapıştırın.',
  unsupported: '“{detail}” okunmuyor: yalnız Transverse Mercator (UTM dahil), coğrafi sistem ve afinle türetilmiş yerel sistem tanımlanabilir.',
  unit: 'Birim “{detail}”: yalnız metre ve derece okunur.',
  meridian: 'Başlangıç meridyeni “{detail}”: yalnız Greenwich okunur.',
  grid: 'Izgarası EPSG:{srid} ({name}) ile aynı; datumu metnin kendi datumu.',
} as const;

/** A WKT or PROJ text read (docs/adr/0168 §5): the definition it is, the registry's system it is, what its datum shares with the registry. */
export interface Imported {
  readonly definition: CrsDefinition;
  readonly same: number | null;
  readonly note: string | null;
}

/** A text pasted or a `.prj` file read as a definition, or why not. */
export function readDefinition(text: string): Imported | { readonly problem: string } {
  const r = crsReadText(text);
  if ('error' in r) return { problem: READ_TEXTS[r.error.kind].replaceAll('{detail}', () => r.error.detail) };
  const definition = definitionFrom(r.name, r.system);
  if (!definition) return { problem: READ_TEXTS.unsupported.replaceAll('{detail}', 'Pseudo-Mercator') };
  const shared = r.registry && !r.registry.exact ? crsBySrid(r.registry.srid) : undefined;
  return {
    definition,
    same: sameSrid(definition),
    note: shared ? READ_TEXTS.grid.replaceAll('{srid}', String(shared.srid)).replaceAll('{name}', () => shared.name) : null,
  };
}

/** What Ortak noktalardan hesapla says: rows left out, and why there is no plane. */
export const FIT_TEXTS = {
  skipped: 'Satır {rows} hesaba katılmadı: dört değer de sayı olmalı (bu sistemde ve tabanda sağa ve yukarı).',
  tooFew: '{kind} için en az {need} kullanılan ortak nokta gerekir; şimdi {n}.',
  coincident: 'Bu sistemdeki noktaların hepsi aynı yerde; düzlem bulunamaz.',
  collinear: 'Bu sistemdeki noktalar bir doğru üstünde; afin bulunamaz. Doğrunun dışında bir nokta ekleyin.',
} as const;

/** A row of Ortak noktalardan hesapla as typed: this system's east and north, the base's east and north, Kullan ("0" leaves it out). */
export type FitRow = readonly [string, string, string, string, string];

/**
 * The plane the common points give (this system → its base), the rows its pairs came from, each pair's residual
 * (transformed point less the base's: east, north, length; metres) and m0 (null without redundancy).
 */
export interface PlaneFit {
  readonly plane: CrsPlane;
  readonly rows: readonly number[];
  readonly residuals: readonly (readonly [number, number, number])[];
  readonly m0: number | null;
}

/** A row's four values when all read as numbers; null for an empty row or one being typed. */
function pairValues(r: FitRow): [number, number, number, number] | null {
  const v = r.slice(0, 4).map(number);
  return v.every((x) => x !== null) ? (v as [number, number, number, number]) : null;
}

/** The rows with something typed that are not pairs: left out of the solution, as Vektör oturtma leaves a row being typed, and named. */
export const unreadRows = (rows: readonly FitRow[]): number[] => rows.flatMap((r, i) => (r.slice(0, 4).some((v) => v.trim()) && !pairValues(r) ? [i] : []));

/** “Satır 3, 5 hesaba katılmadı: …”; null when every row is read. */
export const skippedText = (rows: readonly number[]): string | null => (rows.length ? FIT_TEXTS.skipped.replaceAll('{rows}', rows.map((r) => r + 1).join(', ')) : null);

/**
 * Ortak noktalardan hesapla (docs/adr/0168 §1, §6): the least-squares similarity or affine through the used pairs, as
 * Vektör oturtma solves them (the core's `ops::fit`, its pairs' centred frames), written as the window's plane; or what
 * stops it. Rows that are not pairs are left out. The desktop's `definition_form::fit_plane`.
 */
export function fitPlane(rows: readonly FitRow[], kind: PlaneKind): PlaneFit | { readonly problem: string } {
  const pairs: FitPair[] = [];
  const at: number[] = [];
  rows.forEach((r, i) => {
    const v = pairValues(r);
    if (!v) return;
    pairs.push({ source: { x: v[0], y: v[1] }, target: { x: v[2], y: v[3] }, used: r[4].trim() !== '0' });
    at.push(i);
  });
  const got = fitTransform(pairs, kind === 'similarity' ? 'helmert' : 'affine');
  if ('error' in got) {
    if (got.error === 'too_few')
      return {
        problem: FIT_TEXTS.tooFew
          .replaceAll('{kind}', kind === 'similarity' ? 'Benzerlik' : 'Afin')
          .replaceAll('{need}', String(got.need))
          .replaceAll('{n}', String(pairs.filter((p) => p.used).length)),
      };
    return { problem: got.error === 'coincident' ? FIT_TEXTS.coincident : FIT_TEXTS.collinear };
  }
  // The centred solution un-centred: base = to + M·(p − from).
  const { from: o, to: t, params } = got;
  let plane: CrsPlane;
  if (params.length === 2) {
    const [a, b] = params as [number, number];
    plane = { kind: 'similarity', east: t.x - (a * o.x - b * o.y), north: t.y - (b * o.x + a * o.y), rotation: Math.atan2(b, a) * (180 / Math.PI), scale: Math.hypot(a, b) };
  } else {
    const [a, b, c, d] = params as [number, number, number, number];
    plane = { kind: 'affine', a, b: c, c: t.x - a * o.x - c * o.y, d: b, e: d, f: t.y - b * o.x - d * o.y };
  }
  return { plane, rows: at, residuals: got.residuals, m0: got.m0 };
}

/** A plane's values as the form's fields write them. */
export function planeTexts(f: DefinitionForm, plane: CrsPlane): void {
  if (plane.kind === 'similarity') {
    f.plane = 'similarity';
    [f.east, f.north, f.rotation, f.scale] = [plane.east, plane.north, plane.rotation, plane.scale].map(String) as [string, string, string, string];
  } else {
    f.plane = 'affine';
    for (const k of AFFINE) f[k] = String(plane[k]);
  }
}

/** A definition as WKT: WKT 1, a local system WKT 2 over its base, named as the registry or the base's definition names it (§5). */
export function definitionWkt(d: CrsDefinition): string | null {
  const s = definitionSystem(d);
  if (!s) return null;
  let base: string | null = null;
  if (d.system.kind === 'local') {
    const b = d.system.base;
    base = b.definition ? b.definition.name : b.srid !== undefined ? (crsBySrid(b.srid)?.name ?? null) : null;
    if (base === null) return null;
  }
  return crsWriteWkt(d.name, s, base);
}

/** A definition as a PROJ string; null for a local system (PROJ cannot write one derived from another). */
export function definitionProj(d: CrsDefinition): string | null {
  const s = definitionSystem(d);
  return s ? crsWriteProj(s) : null;
}

/** The form of a definition. */
export function formOf(d: CrsDefinition): DefinitionForm {
  const f: DefinitionForm = { ...emptyForm(), name: d.name };
  const takeDatum = (datum: RegistryDatum | undefined, custom: CustomDatum | undefined) => {
    if (datum !== undefined) f.datum = datum;
    else if (custom) {
      f.datum = 'custom';
      f.datumName = custom.name;
      const e = custom.ellipsoid;
      // A classic ellipsoid by its name and values; any other is typed.
      f.ellipsoid = ELLIPSOIDS.find(([n, a, rf]) => n === e.name && a === e.semiMajor && rf === e.inverseFlattening)?.[0] ?? 'custom';
      f.semiMajor = String(e.semiMajor);
      f.inverseFlattening = String(e.inverseFlattening);
      f.linked = !!custom.toWgs84;
      const h = custom.toWgs84;
      if (h) {
        const v = [...h.translation, ...h.rotation, h.scale];
        f.parameters = Object.fromEntries(PARAMETERS.map((k, i) => [k, String(v[i])])) as Record<Parameter, string>;
        f.convention = h.convention;
        f.accuracy = h.accuracy === undefined ? '' : String(h.accuracy);
      }
    }
  };
  const s = d.system;
  if (s.kind === 'tm') {
    f.kind = 'tm';
    takeDatum(s.datum, s.customDatum);
    f.latitudeOfOrigin = s.latitudeOfOrigin === undefined ? '' : String(s.latitudeOfOrigin);
    f.centralMeridian = String(s.centralMeridian);
    f.scaleFactor = String(s.scaleFactor);
    f.falseEasting = String(s.falseEasting);
    f.falseNorthing = String(s.falseNorthing);
  } else if (s.kind === 'geographic') {
    f.kind = 'geographic';
    takeDatum(s.datum, s.customDatum);
  } else {
    f.kind = 'local';
    f.base = s.base.srid === undefined ? '' : String(s.base.srid);
    const p = s.plane;
    if (p.kind === 'similarity') {
      f.plane = 'similarity';
      [f.east, f.north, f.rotation, f.scale] = [p.east, p.north, p.rotation, p.scale].map(String) as [string, string, string, string];
    } else {
      f.plane = 'affine';
      for (const k of AFFINE) f[k] = String(p[k]);
    }
  }
  return f;
}
