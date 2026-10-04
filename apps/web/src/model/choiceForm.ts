import type { Convention } from '../contracts/generated/Convention';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import type { RegistryDatum } from '../contracts/generated/RegistryDatum';

/**
 * Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6; the desktop's `kentos_project::choice_form`): for each of
 * the registry's three datum pairs, what EPSG's way is, and what is typed turned into the project's choice
 * (`DatumTransform`) or, field by field, what is wrong. The shared cases are fixtures/crs/v1/choice-form.json
 * (scripts/fixtures/crs_choice_form_cases.py, from the ADR's rules).
 */

/** A pair of the registry's datums. */
export type Pair = readonly [RegistryDatum, RegistryDatum];

/** The registry's datum pairs, in the order Datum dönüşümleri lists them. */
export const PAIRS: readonly Pair[] = [
  ['ED50', 'TUREF'],
  ['ED50', 'WGS84'],
  ['TUREF', 'WGS84'],
];

/** EPSG's way between a pair, as the transforms say it (docs/adr/0167 §3). */
export function epsgText(pair: Pair): string {
  if (pair[0] === 'ED50' && pair[1] === 'TUREF') return '±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil';
  if (pair[0] === 'ED50') return '±2 m, EPSG:1784; resmî dönüşüm değil';
  return '±1 m, EPSG:5261';
}

/** A datum's name as Datum dönüşümleri writes it. */
export const datumName = (d: RegistryDatum): string => (d === 'WGS84' ? 'WGS 84' : d);

/** How a pair goes: EPSG's way, the project's seven parameters, or a grid of the device's library. */
export type Method = 'epsg' | 'helmert' | 'grid';

/** The seven parameters' fields, in their order: the translations (m), the rotations (″), the scale difference (ppm). */
export const PARAMETERS = ['tx', 'ty', 'tz', 'rx', 'ry', 'rz', 'ds'] as const;
export type Parameter = (typeof PARAMETERS)[number];

/** What Datum dönüşümleri holds for a pair, as typed. */
export interface ChoiceForm {
  method: Method;
  /** From the pair's second datum to its first. */
  reversed: boolean;
  name: string;
  parameters: Record<Parameter, string>;
  convention: Convention;
  /** The grid's SHA-256. */
  grid: string;
  accuracy: string;
}

/** A grid the form may name: one of the device's library, or the one the project's choice names already. */
export interface GridRef {
  readonly id: string;
  readonly file: string;
  readonly size: number;
}

/** What is wrong, by field: `name`, `tx` … `ds`, `grid`, `accuracy`. */
export type Problems = Partial<Record<'name' | Parameter | 'grid' | 'accuracy', string>>;

export const TEXTS = {
  name: 'Adını yazın: değerlerin yanında dayanağı olarak görünür.',
  number: 'Sayı yazın, ör. -84.1.',
  accuracy: '0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.',
  grid: "Bir ızgara seçin; listede yoksa Izgaralar'dan ekleyin.",
} as const;

const empty = (): Record<Parameter, string> => ({ tx: '', ty: '', tz: '', rx: '', ry: '', rz: '', ds: '' });

/** EPSG's way: no choice. */
export const epsgForm = (): ChoiceForm => ({ method: 'epsg', reversed: false, name: '', parameters: empty(), convention: 'positionVector', grid: '', accuracy: '' });

/** A number as the Hesap windows read one: trimmed, its first comma a point; null for anything else. */
function number(text: string): number | null {
  const t = text.trim().replace(',', '.');
  return t && /^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$/i.test(t) ? Number(t) : null;
}

/** The choice a pair's form gives, null for EPSG's way; or what is wrong. */
export function buildChoice(pair: Pair, form: ChoiceForm, grids: readonly GridRef[]): { readonly choice: DatumTransform | null } | { readonly problems: Problems } {
  if (form.method === 'epsg') return { choice: null };
  const [from, to] = form.reversed ? [pair[1], pair[0]] : pair;
  const problems: Problems = {};
  const name = form.name.trim();
  if (!name) problems.name = TEXTS.name;
  let accuracy: number | undefined;
  if (form.accuracy.trim()) {
    const a = number(form.accuracy);
    if (a === null || a < 0) problems.accuracy = TEXTS.accuracy;
    else accuracy = a;
  }
  const choice: DatumTransform = { from, to, name };
  if (form.method === 'helmert') {
    const values = PARAMETERS.map((key, i) => {
      const text = form.parameters[key].trim();
      // An empty rotation or scale difference is 0: three parameters.
      if (!text && i >= 3) return 0;
      const v = number(text);
      if (v === null) problems[key] = TEXTS.number;
      return v ?? 0;
    });
    choice.helmert = {
      translation: [values[0]!, values[1]!, values[2]!],
      rotation: [values[3]!, values[4]!, values[5]!],
      scale: values[6]!,
      convention: form.convention,
      ...(accuracy !== undefined ? { accuracy } : {}),
    };
  } else {
    const g = grids.find((x) => x.id === form.grid);
    if (!g) problems.grid = TEXTS.grid;
    else choice.grid = { id: g.id, file: g.file, size: g.size, ...(accuracy !== undefined ? { accuracy } : {}) };
  }
  return Object.keys(problems).length ? { problems } : { choice };
}

/** The pair a choice is for, either way round. */
const samePair = (c: DatumTransform, pair: Pair) => (c.from === pair[0] && c.to === pair[1]) || (c.from === pair[1] && c.to === pair[0]);

/** The form of a pair's choice among the project's; EPSG's way without one. */
export function formOf(pair: Pair, choices: readonly DatumTransform[]): ChoiceForm {
  const c = choices.find((x) => samePair(x, pair));
  if (!c) return epsgForm();
  const form: ChoiceForm = { ...epsgForm(), reversed: c.from === pair[1], name: c.name };
  if (c.helmert) {
    const h = c.helmert;
    const v = [...h.translation, ...h.rotation, h.scale];
    form.method = 'helmert';
    form.parameters = Object.fromEntries(PARAMETERS.map((k, i) => [k, String(v[i])])) as Record<Parameter, string>;
    form.convention = h.convention;
    form.accuracy = h.accuracy === undefined ? '' : String(h.accuracy);
  } else if (c.grid) {
    form.method = 'grid';
    form.grid = c.grid.id;
    form.accuracy = c.grid.accuracy === undefined ? '' : String(c.grid.accuracy);
  }
  return form;
}
