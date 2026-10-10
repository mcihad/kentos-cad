import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { ProjectVariable } from '../contracts/generated/ProjectVariable';
import { crsBySrid } from '../geo/crs';
import type { Signal } from '../core/signal';
import type { ProjectSettings } from './projectSettings';
import type { VariableKind } from '../contracts/generated/VariableKind';

/**
 * Projenin değişkenleri (docs/adr/0214 §2.3): the rules a project's `@`
 * values keep (the contract's `kentos_contracts::variables`), as the settings
 * window and the document check them; and the built-in values every host
 * gives beside them.
 */

export type { ProjectVariable, VariableKind };

/** The most variables a project keeps, the longest name and text value (the contract's bounds). */
export const VARIABLES_MAX = 200;
export const VARIABLE_NAME_MAX = 64;
export const VARIABLE_TEXT_MAX = 4000;

/** The built-in variables with what they hold: a project's variable may not take one of these names. */
export const BUILTIN_VARIABLES: readonly (readonly [string, string])[] = [
  ['proje_adi', 'Projenin adı'],
  ['koordinat_sistemi', 'Koordinat sisteminin adı'],
  ['epsg', 'Koordinat sisteminin EPSG kodu'],
  ['olcek', 'Çizim ölçeğinin paydası'],
  ['tarih', 'Bugünün tarihi'],
  ['simdi', 'Şimdiki tarih ve saat'],
  ['katman_adi', 'Değerlendirilen nesnenin katmanının adı'],
  ['katman', 'Değerlendirilen nesnenin katmanının adı'],
  ['kullanici', 'Oturumdaki kullanıcının adı'],
];

const MARKS: Record<string, string> = { ı: 'I', i: 'I', İ: 'I', ğ: 'G', Ğ: 'G', ü: 'U', Ü: 'U', ş: 'S', Ş: 'S', ö: 'O', Ö: 'O', ç: 'C', Ç: 'C' };

/** A name as names are compared: Turkish letters and case aside (`Proje_Adı` is `proje_adi`). */
export const variableKey = (name: string): string => [...name].map((c) => MARKS[c] ?? c.toUpperCase()).join('');

const LETTER = /^\p{L}$/u;
const WORD = /^[\p{L}\p{N}_]$/u;

/** What is wrong with a name, when anything is. */
export function variableNameProblem(name: string): string | null {
  const chars = [...name];
  if (!chars.length) return 'değişkenin adı boş';
  if (chars.length > VARIABLE_NAME_MAX) return `“${name}” değişken adı ${VARIABLE_NAME_MAX} karakterden uzun`;
  if (!(LETTER.test(chars[0]) || chars[0] === '_') || !chars.every((c) => WORD.test(c)))
    return `“${name}” değişken adı olamaz: bir harf ya da _ ile başlar, harf, rakam ve _ içerir`;
  const key = variableKey(name);
  if (BUILTIN_VARIABLES.some(([b]) => variableKey(b) === key)) return `@${name} yerleşik bir değişkendir; başka ad seçin`;
  return null;
}

/** Whether a text is a calendar day `YYYY-AA-GG` (years 1–9999). */
export function isIsoDate(t: string): boolean {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(t);
  if (!m) return false;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  if (y < 1 || mo < 1 || mo > 12) return false;
  const leap = y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][mo - 1];
  return d >= 1 && d <= days;
}

/** What is wrong with a variable, when anything is (the contract's `ProjectVariable::problem`). */
export function variableProblem(v: ProjectVariable): string | null {
  const named = variableNameProblem(v.name);
  if (named) return named;
  const value = v.value ?? null;
  if (value === null) return null;
  switch (v.kind ?? 'text') {
    case 'number':
      return typeof value === 'number' && Number.isFinite(value) ? null : `@${v.name} sayı değişkeni; değeri sayı olmalı`;
    case 'bool':
      return typeof value === 'boolean' ? null : `@${v.name} doğru/yanlış değişkeni; değeri doğru ya da yanlış olmalı`;
    case 'date':
      return typeof value === 'string' && isIsoDate(value) ? null : `@${v.name} tarih değişkeni; değeri YYYY-AA-GG biçiminde bir tarih olmalı`;
    default:
      if (typeof value !== 'string') return `@${v.name} metin değişkeni; değeri metin olmalı`;
      return [...value].length > VARIABLE_TEXT_MAX ? `@${v.name} değeri ${VARIABLE_TEXT_MAX} karakterden uzun` : null;
  }
}

/** What is wrong with a project's variables, when anything is (the contract's `variables_problem`). */
export function variablesProblem(list: readonly ProjectVariable[]): string | null {
  if (list.length > VARIABLES_MAX) return `projede ${list.length} değişken var; en çok ${VARIABLES_MAX}`;
  const seen = new Set<string>();
  for (const v of list) {
    const p = variableProblem(v);
    if (p) return p;
    const key = variableKey(v.name);
    if (seen.has(key)) return `@${v.name} adında iki değişken var`;
    seen.add(key);
  }
  return null;
}

/** The variables as a project keeps them (the contract's `sanitized_variables`): of one name the first, none broken. */
export function sanitizedVariables(list: readonly ProjectVariable[]): ProjectVariable[] {
  const seen = new Set<string>();
  const out: ProjectVariable[] = [];
  for (const v of list) {
    if (out.length === VARIABLES_MAX) break;
    const key = variableKey(v.name);
    if (variableProblem(v) === null && !seen.has(key)) {
      seen.add(key);
      out.push(structuredClone(v));
    }
  }
  return out;
}

/** A `@` value as the expression facade takes it (model/expression/expression.ts `ExprVariable`). */
export interface ExpressionVariable {
  readonly name: string;
  readonly value: string | number | boolean | null;
  readonly description: string;
}

const KIND_NAME: Record<VariableKind, string> = { text: 'metin', number: 'sayı', bool: 'doğru/yanlış', date: 'tarih' };

const pad = (n: number, w = 2) => String(n).padStart(w, '0');

/** A moment written as the time core writes it: the date at midnight, else the date and the time to the second. */
function written(d: Date): string {
  const day = `${pad(d.getFullYear(), 4)}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const [h, m, s] = [d.getHours(), d.getMinutes(), d.getSeconds()];
  return h === 0 && m === 0 && s === 0 ? day : `${day}T${pad(h)}:${pad(m)}:${pad(s)}`;
}

/** What `expressionVariables` reads of a project. */
export interface VariableSource {
  readonly name: string;
  readonly variables: readonly ProjectVariable[];
  /** The coordinate system's name: the registry's or the project's definition's; null for a local project. */
  readonly system: string | null;
  /** The EPSG code; null for a local project or a definition of its own. */
  readonly epsg: number | null;
  readonly plotScale: number;
}

/**
 * The `@` values a project's expressions read (docs/adr/0214 §2.3): its own variables first, then the built-in
 * ones: the project's name, its coordinate system and EPSG code, its scale, the date and the moment (`now`, the
 * device's local time, the same through one evaluation), the signed-in user. `@katman_adi` is the object's own
 * layer: the language resolves it. The desktop's `kentos_project::variables` gives the same.
 */
export function expressionVariables(p: VariableSource, now: Date, user: string): ExpressionVariable[] {
  const own: ExpressionVariable[] = p.variables.map((v) => ({
    name: v.name,
    value: v.value ?? null,
    description: v.label || `Projenin değişkeni (${KIND_NAME[v.kind ?? 'text']})`,
  }));
  const day = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const builtins: [string, ExpressionVariable['value']][] = [
    ['proje_adi', p.name],
    ['koordinat_sistemi', p.system],
    ['epsg', p.epsg],
    ['olcek', p.plotScale],
    ['tarih', written(day)],
    ['simdi', written(now)],
    ['kullanici', user || null],
  ];
  const described = (n: string) => BUILTIN_VARIABLES.find(([b]) => b === n)?.[1] ?? '';
  return [...own, ...builtins.map(([name, value]) => ({ name, value, description: described(name) }))];
}

/** What `expressionVariables` reads of a project's settings as data (a settings window's draft too). */
export function settingsSource(
  name: string,
  s: { readonly srid: number; readonly customCrs?: CrsDefinition | null; readonly plotScale: number; readonly variables?: readonly ProjectVariable[] },
): VariableSource {
  const custom = s.customCrs ?? null;
  const crs = crsBySrid(s.srid);
  return {
    name,
    variables: s.variables ?? [],
    system: custom ? custom.name : !crs || crs.kind === 'local' ? null : crs.name,
    epsg: !custom && crs && crs.kind !== 'local' ? crs.srid : null,
    plotScale: s.plotScale,
  };
}

/** What `expressionVariables` reads of an open drawing: its name and its settings. */
export const documentSource = (name: string, s: ProjectSettings): VariableSource =>
  settingsSource(name, { srid: s.crs.value.srid, customCrs: s.customCrs.value, plotScale: s.plotScale.value, variables: s.variables.value });

/** An open drawing's `@` values (`expressionVariables`), `now` its moment, `user` who is signed in. */
export function documentVariables(doc: { readonly name: Signal<string>; readonly settings: ProjectSettings }, now: Date, user: string): ExpressionVariable[] {
  return expressionVariables(documentSource(doc.name.value, doc.settings), now, user);
}
