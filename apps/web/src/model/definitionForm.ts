import type { Convention } from '../contracts/generated/Convention';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { CrsPlane } from '../contracts/generated/CrsPlane';
import type { CrsSystem } from '../contracts/generated/CrsSystem';
import type { CustomDatum } from '../contracts/generated/CustomDatum';
import type { RegistryDatum } from '../contracts/generated/RegistryDatum';
import { CRS_REGISTRY, crsBySrid } from '../geo/crs';
import { systemOf } from './geom/crsTransform';
import { definitionSystem } from './projectCrs';

/**
 * The Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §6; the desktop's `kentos_project::definition_form`):
 * what is typed turned into the project's definition (`CrsDefinition`) or, field by field, what is wrong; a definition
 * the registry has already is said. The shared cases are fixtures/crs/v1/definition-form.json
 * (scripts/fixtures/crs_definition_form_cases.py, from the ADR's rules).
 */

export type Kind = 'tm' | 'geographic' | 'local';
/** A definition's datum: one of the registry's by name, or the project's own. */
export type DatumPick = RegistryDatum | 'custom';
export type PlaneKind = 'similarity' | 'affine';

/** The classic ellipsoids by name: semi-major axis (m), inverse flattening (the core's `crs::text` table). */
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

/** “EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.”: the registry's system a definition on one of its datums is. */
function sameAsRegistry(d: CrsDefinition): string | null {
  const s = d.system;
  if (s.kind === 'local' || s.datum === undefined) return null;
  const it = definitionSystem(d);
  if (!it) return null;
  const same = CRS_REGISTRY.find((c) => c.kind !== 'local' && JSON.stringify(systemOf(c)) === JSON.stringify(it));
  return same ? `EPSG:${same.srid} (${same.name}) ile aynı; kayıttakini seçin.` : null;
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
